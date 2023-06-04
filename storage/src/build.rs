use std::{
	collections::HashMap,
	hash::Hash,
	io::{self, BufWriter, Seek, Write},
};

use inferdf_core::DivCeil;
use iref::Iri;
use langtag::LanguageTag;
use rdf_types::{
	IndexVocabulary, IriVocabulary, LanguageTagVocabulary, LiteralVocabulary, Vocabulary,
};

use crate::{
	encode::PageError, first_page_offset, graphs_per_page, page, triples_per_page, Encode, Header,
	Tag, Version,
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

pub trait LexicalLiteral: LiteralVocabulary {
	fn lexical_value<'a>(&'a self, value: &'a Self::Value) -> Option<&'a [u8]>;

	fn lexical_type<'a>(
		&'a self,
		ty: &'a Self::Type,
	) -> Option<rdf_types::literal::Type<Iri<'a>, LanguageTag<'a>>>;

	fn lexical_literal<'a>(
		&'a self,
		literal: &'a Self::Literal,
	) -> Option<rdf_types::Literal<rdf_types::literal::Type<Iri<'a>, LanguageTag<'a>>, &'a [u8]>> {
		let l = self.literal(literal)?;
		Some(rdf_types::Literal::new(
			self.lexical_value(l.value())?,
			self.lexical_type(l.type_())?,
		))
	}
}

impl LexicalLiteral for IndexVocabulary {
	fn lexical_value<'a>(&'a self, value: &'a Self::Value) -> Option<&'a [u8]> {
		Some(value.as_bytes())
	}

	fn lexical_type<'a>(
		&'a self,
		ty: &'a Self::Type,
	) -> Option<rdf_types::literal::Type<Iri<'a>, LanguageTag<'a>>> {
		match ty {
			rdf_types::literal::Type::Any(iri) => {
				Some(rdf_types::literal::Type::Any(self.iri(iri)?))
			}
			rdf_types::literal::Type::LangString(tag) => Some(
				rdf_types::literal::Type::LangString(self.language_tag(tag)?),
			),
		}
	}
}

pub fn build<V: Vocabulary, W: Write + Seek>(
	vocabulary: &V,
	interpretation: &inferdf_core::interpretation::LocalInterpretation<V>,
	dataset: &inferdf_core::dataset::LocalDataset,
	output: &mut BufWriter<W>,
	options: Options,
) -> Result<(), PageError>
where
	V::Iri: Eq + Hash,
	V::Literal: Eq + Hash,
	V: LexicalLiteral,
{
	let graphs_per_page = graphs_per_page(options.page_size);

	let mut header = Header {
		tag: Tag,
		version: Version,
		page_size: options.page_size,
		iri_count: interpretation.terms_by_iri().len() as u32,
		iri_page_count: 0,
		literal_count: interpretation.terms_by_literal().len() as u32,
		literal_page_count: 0,
		resource_count: interpretation.len(),
		resource_page_count: 0,
		named_graph_count: dataset.named_graph_count() as u32,
		named_graph_page_count: DivCeil::div_ceil(
			dataset.named_graph_count() as u32,
			graphs_per_page,
		),
		default_graph: page::graphs::Description::default(),
	};

	let first_page_offset = first_page_offset(options.page_size);
	output.seek(std::io::SeekFrom::Start(first_page_offset))?;

	let mut iris: Vec<_> = interpretation
		.terms_by_iri()
		.iter()
		.map(|(iri, id)| (iri, vocabulary.iri(iri).unwrap(), *id))
		.collect();
	let mut iri_paths = HashMap::new();
	iris.sort_unstable_by(|a, b| a.1.cmp(&b.1));
	for page in page::iris::Pages::new(
		options.page_size,
		iris.into_iter()
			.map(|(t, iri, id)| (t, page::iris::Entry::new(iri, id))),
		|t, path| {
			iri_paths.insert(t, path);
		},
	) {
		page.encode_page(options.page_size, output)?;
		header.iri_page_count += 1;
	}

	let mut literals: Vec<_> = interpretation
		.terms_by_literal()
		.iter()
		.map(|(literal, id)| (literal, vocabulary.lexical_literal(literal).unwrap(), *id))
		.collect();
	let mut literal_paths = HashMap::new();
	literals.sort_unstable_by(|a, b| a.1.cmp(&b.1));
	for page in page::literals::Pages::new(
		options.page_size,
		literals
			.into_iter()
			.map(|(t, literal, id)| (t, page::literals::Entry::new(literal, id))),
		|t, path| {
			literal_paths.insert(t, path);
		},
	) {
		page.encode_page(options.page_size, output)?;
		header.literal_page_count += 1;
	}

	let resources = interpretation.iter().map(|(id, r)| {
		page::resource_terms::Entry::new(
			id,
			r.as_iri
				.iter()
				.map(|iri| *iri_paths.get(iri).unwrap())
				.collect(),
			r.as_literal
				.iter()
				.map(|iri| *literal_paths.get(iri).unwrap())
				.collect(),
			r.different_from.iter().copied().collect(),
		)
	});
	for page in page::resource_terms::Pages::new(options.page_size, resources) {
		page.encode_page(options.page_size, output)?;
		header.resource_page_count += 1;
	}

	let named_graphs_first_page =
		header.iri_page_count + header.literal_page_count + header.resource_page_count;
	let mut page_count = named_graphs_first_page + header.named_graph_page_count;

	// skip named graph descriptions pages for now.
	output.seek(io::SeekFrom::Current(
		(header.named_graph_page_count * header.page_size) as i64,
	))?;

	// write default graph.
	header.default_graph = build_graph(dataset.default_graph(), output, options, page_count)?;
	page_count += header.default_graph.page_count();

	// write named graphs.
	let mut named_graph_entries = Vec::new();
	let mut named_graphs: Vec<_> = dataset.named_graphs().collect();
	named_graphs.sort_unstable_by_key(|(id, _)| *id);
	for (id, graph) in named_graphs {
		let description = build_graph(graph, output, options, page_count)?;
		named_graph_entries.push(page::graphs::Entry { id, description });
		page_count += description.page_count();
	}

	// get back to write the named graphs descriptions.
	output.seek(io::SeekFrom::Start(
		first_page_offset + (header.page_size * named_graphs_first_page) as u64,
	))?;
	for page in page::graphs::Pages::new(options.page_size, named_graph_entries.into_iter()) {
		page.encode_page(options.page_size, output)?
	}

	// get to the begining to write the header.
	output.seek(io::SeekFrom::Start(0))?;
	header.encode(output)?;
	Ok(())
}

fn build_graph<W: Write + Seek>(
	graph: &inferdf_core::dataset::local::Graph,
	output: &mut BufWriter<W>,
	options: Options,
	page_index: u32,
) -> Result<page::graphs::Description, PageError> {
	let mut desc = page::graphs::Description {
		triple_count: graph.len() as u32,
		triple_page_count: DivCeil::div_ceil(
			graph.len() as u32,
			triples_per_page(options.page_size),
		),
		resource_count: graph.resource_count() as u32,
		resource_page_count: 0,
		first_page: page_index,
	};

	let mut triples_map = HashMap::new();
	let triples = graph.iter().enumerate().map(|(j, (i, fact))| {
		triples_map.insert(i as u32, j as u32);
		*fact
	});
	for page in page::triples::Pages::new(options.page_size, triples) {
		page.encode_page(options.page_size, output)?;
	}

	let mut resources: Vec<_> = graph.iter_resources().collect();
	resources.sort_unstable_by_key(|(id, _)| *id);
	let resources = resources.into_iter().map(|(id, r)| {
		page::resource_triples::Entry::new(
			id,
			r.iter_as_subject().map(|i| triples_map[&i]).collect(),
			r.iter_as_predicate().map(|i| triples_map[&i]).collect(),
			r.iter_as_object().map(|i| triples_map[&i]).collect(),
		)
	});
	for page in page::resource_triples::Pages::new(options.page_size, resources) {
		page.encode_page(options.page_size, output)?;
		desc.resource_page_count += 1;
	}

	Ok(desc)
}
