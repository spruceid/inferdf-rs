use std::{
	collections::HashMap,
	hash::Hash,
	io::{self, BufWriter, Seek, SeekFrom, Write},
};

use inferdf_core::{dataset, Classification, Dataset, Interpretation, IteratorWith, Module};
use iref::Iri;
use langtag::LanguageTag;
use locspan::Meta;
use paged::{utils::CeilingDiv, Encode, EncodeSized, Heap};
use rdf_types::{literal, Literal, LiteralVocabulary, Vocabulary, VocabularyMut};

use crate::{
	header::{self, EncodedIri, EncodedLiteral, IriEntry, LiteralEntry},
	Header,
};

pub const DEFAULT_PAGE_SIZE: u32 = 4096;

#[derive(Clone, Copy)]
pub struct Options {
	pub page_size: u32,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			page_size: DEFAULT_PAGE_SIZE,
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum Error<M> {
	#[error(transparent)]
	Module(M),

	#[error(transparent)]
	IO(#[from] io::Error),

	#[error("invalid module")]
	Invalid,
}

pub fn build<V: VocabularyMut, M: Module<V>, W: Write + Seek>(
	vocabulary: &mut V,
	module: &M,
	output: &mut BufWriter<W>,
	options: Options,
) -> Result<(), Error<M::Error>>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Iri: Clone + Eq + Hash,
	V::Literal: Clone + Eq + Hash,
	V::Value: AsRef<str>,
{
	let first_page_offset =
		Header::<V>::ENCODED_SIZE.ceiling_div(options.page_size) * options.page_size;

	output.seek(SeekFrom::Start(first_page_offset as u64))?;
	let mut encoder = paged::Encoder::new(output, options.page_size);
	let mut heap = paged::Heap::new();

	// IRIs.
	let mut iri_entries = Vec::new();
	let mut module_iris = module.interpretation().iris().map_err(Error::Module)?;
	while let Some(iri_interpretation) = module_iris.next_with(vocabulary) {
		let (iri, id) = iri_interpretation.map_err(Error::Module)?;
		iri_entries.push(IriEntry {
			iri: EncodedIri(iri),
			interpretation: id,
		});
	}
	iri_entries.sort_by(|a, b| {
		vocabulary
			.iri(&a.iri.0)
			.unwrap()
			.cmp(vocabulary.iri(&b.iri.0).unwrap())
	});
	let mut iri_map: HashMap<V::Iri, u32> = HashMap::new();
	for (i, entry) in iri_entries.iter().enumerate() {
		let iri: &V::Iri = entry.iri();
		iri_map.insert(iri.clone(), i as u32);
	}
	let iris = encoder.section_from_iter_with(&mut heap, vocabulary, iri_entries.iter())?;

	// Literals.
	let mut literal_entries = Vec::new();
	let mut module_literals = module.interpretation().literals().map_err(Error::Module)?;
	while let Some(literal_interpretation) = module_literals.next_with(vocabulary) {
		let (literal, id) = literal_interpretation.map_err(Error::Module)?;
		literal_entries.push(LiteralEntry {
			literal: EncodedLiteral(literal),
			interpretation: id,
		});
	}
	literal_entries.sort_by(|a, b| {
		lexical_literal(vocabulary, &a.literal.0).cmp(&lexical_literal(vocabulary, &b.literal.0))
	});
	let mut literal_map = HashMap::new();
	for (i, entry) in literal_entries.iter().enumerate() {
		let literal: &V::Literal = entry.literal();
		literal_map.insert(literal.clone(), i as u32);
	}
	let literals = encoder.section_from_iter_with(&mut heap, vocabulary, literal_entries.iter())?;

	// Interpretation.
	let mut resource_entries = Vec::new();
	let mut module_resources = module.interpretation().resources().map_err(Error::Module)?;
	while let Some(r) = module_resources.next_with(vocabulary) {
		use inferdf_core::interpretation::Resource;
		let (id, r) = r.map_err(Error::Module)?;

		let mut iris = Vec::new();
		let mut r_iris = r.as_iri();
		while let Some(i) = r_iris.next_with(vocabulary) {
			let iri = i.map_err(Error::Module)?;
			iris.push(*iri_map.get(&iri).unwrap())
		}

		let mut literals = Vec::new();
		let mut r_literals = r.as_literal();
		while let Some(i) = r_literals.next_with(vocabulary) {
			let literal = i.map_err(Error::Module)?;
			literals.push(*literal_map.get(&literal).unwrap())
		}

		let mut ne = Vec::new();
		let mut r_ne = r.different_from();
		while let Some(i) = r_ne.next_with(vocabulary) {
			let id = i.map_err(Error::Module)?;
			ne.push(id)
		}

		let class = module
			.classification()
			.resource_class(id)
			.map_err(Error::Module)?;

		resource_entries.push(header::InterpretedResource::new(
			id, iris, literals, ne, class,
		))
	}
	let resources = encoder.section_from_iter(&mut heap, resource_entries.iter())?;

	// Graphs.
	// let mut graph_entries = Vec::new();
	let mut default_graph = None;
	let mut named_graphs = Vec::new();
	let mut graphs = module.dataset().graphs();
	while let Some(g) = graphs.next_with(vocabulary) {
		let (id, graph) = g.map_err(Error::Module)?;
		let description = build_graph(vocabulary, graph, &mut encoder, &mut heap)?;

		match id {
			Some(id) => named_graphs.push(header::Graph { id, description }),
			None => default_graph = Some(description),
		}
	}

	let mut named_graph_encoder = encoder.begin_section(&mut heap);
	for entry in named_graphs {
		named_graph_encoder.push(vocabulary, &entry)?;
	}
	let named_graphs = named_graph_encoder.end();

	// Classification.
	let mut groups_by_id = Vec::new();
	let mut groups_by_desc = Vec::new();

	let mut groups = module.classification().groups();
	while let Some(group) = groups.next_with(vocabulary) {
		let (group_id, desc) = group.map_err(Error::Module)?;
		groups_by_id.push(header::GroupById::new(group_id, desc.clone()));
		groups_by_desc.push(header::GroupByDesc::new(desc.clone(), group_id))
	}

	groups_by_id.sort_by_key(|g| g.id);
	groups_by_desc.sort_by(|a, b| a.description.cmp(&b.description));

	let groups_by_desc = encoder.section_from_iter(&mut heap, groups_by_desc.iter())?;
	let groups_by_id = encoder.section_from_iter(&mut heap, groups_by_id.iter())?;

	let mut representatives = Vec::new();
	let mut classes = module.classification().classes();
	while let Some(class) = classes.next_with(vocabulary) {
		let (class, id) = class.map_err(Error::Module)?;
		representatives.push(header::Representative::new(class, id))
	}
	representatives.sort_by_key(|e| e.class);
	let representatives = encoder.section_from_iter(&mut heap, representatives.iter())?;

	// Heap.
	let heap = encoder.add_heap(heap)?;

	// Header.
	let header = Header {
		tag: header::Tag,
		version: header::Version,
		page_size: options.page_size,
		interpretation: header::Interpretation {
			iris,
			literals,
			resources,
		},
		dataset: header::Dataset {
			default_graph: default_graph.unwrap(),
			named_graphs,
		},
		classification: header::Classification {
			groups_by_desc,
			groups_by_id,
			representatives,
		},
		heap,
	};

	let output = encoder.end();
	output.seek(SeekFrom::Start(0))?;
	header.encode(vocabulary, output)?;

	Ok(())
}

fn lexical_literal<'a, V: Vocabulary>(
	vocabulary: &'a V,
	literal: &'a V::Literal,
) -> LexicalLiteral<'a>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Value: AsRef<str>,
{
	let l = vocabulary.literal(literal).unwrap();
	let type_ = match l.type_() {
		literal::Type::Any(i) => literal::Type::Any(vocabulary.iri(i).unwrap()),
		literal::Type::LangString(t) => {
			literal::Type::LangString(vocabulary.language_tag(t).unwrap())
		}
	};

	let value = l.value().as_ref();
	Literal::new(value, type_)
}

type LexicalLiteral<'a> = Literal<literal::Type<&'a Iri, LanguageTag<'a>>, &'a str>;

fn build_graph<'a, V: VocabularyMut, G: dataset::Graph<'a, V>, W: Write + Seek>(
	vocabulary: &mut V,
	graph: G,
	encoder: &mut paged::Encoder<W>,
	heap: &mut Heap,
) -> Result<header::GraphDescription, Error<G::Error>>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Iri: Eq + Hash,
	V::Literal: Eq + Hash,
	V::Value: AsRef<str>,
	// V: LexicalLiteral,
{
	// let mut fact_entries = Vec::new();
	let mut fact_map = HashMap::new();
	let mut index_map = HashMap::new();
	let mut fact_count = 0u32;

	let mut facts_encoder = encoder.begin_section(heap);
	let mut module_facts = graph.triples();
	while let Some(fact) = module_facts.next_with(vocabulary) {
		let (i, Meta(triple, cause)) = fact.map_err(Error::Module)?;
		let entry = header::GraphFact {
			triple: triple.cast(),
			cause,
		};

		let mut result = Ok(());
		let j = *fact_map.entry(entry).or_insert_with_key(|entry| {
			let j = fact_count;
			fact_count += 1;
			result = facts_encoder.push(vocabulary, entry);
			j
		});

		result?;
		index_map.insert(i, j);
	}
	let facts = facts_encoder.end();

	let mut resource_entries = Vec::new();
	let mut module_resources = graph.resources();
	while let Some(resource) = module_resources.next_with(vocabulary) {
		use inferdf_core::dataset::graph::Resource;
		let (id, resource) = resource.map_err(Error::Module)?;

		let mut as_subject = Vec::new();
		for i in resource.as_subject() {
			match index_map.get(&i) {
				Some(j) => as_subject.push(*j),
				None => return Err(Error::Invalid),
			}
		}

		let mut as_predicate = Vec::new();
		for i in resource.as_predicate() {
			match index_map.get(&i) {
				Some(j) => as_predicate.push(*j),
				None => return Err(Error::Invalid),
			}
		}

		let mut as_object = Vec::new();
		for i in resource.as_object() {
			match index_map.get(&i) {
				Some(j) => as_object.push(*j),
				None => return Err(Error::Invalid),
			}
		}

		resource_entries.push(header::GraphResource {
			id,
			as_subject,
			as_object,
			as_predicate,
		})
	}

	let resources = encoder.section_from_iter(heap, resource_entries.iter())?;

	Ok(header::GraphDescription { facts, resources })
}
