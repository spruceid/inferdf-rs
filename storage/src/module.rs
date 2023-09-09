use std::io;

use educe::Educe;
use paged::Decode;
use rdf_types::{literal, LiteralVocabulary, Vocabulary, VocabularyMut};

use crate::{header, Header};
pub use paged::reader::Error;

pub mod classification;
pub mod dataset;
pub mod interpretation;

pub use classification::Classification;
pub use dataset::Dataset;
pub use interpretation::Interpretation;

#[derive(Educe)]
#[educe(Default)]
pub struct Cache<V: Vocabulary> {
	iris: paged::Cache<header::IriEntry<V>>,
	literals: paged::Cache<header::LiteralEntry<V>>,
	graph_resources: paged::Cache<header::GraphResource>,
	graph_facts: paged::Cache<header::GraphFact>,
	named_graphs: paged::Cache<header::Graph>,
	interpretation_resources: paged::Cache<header::InterpretedResource>,
	classification_groups: paged::Cache<header::Group>,
	classification_representatives: paged::Cache<header::Representative>,
}

pub struct Module<V: Vocabulary, R> {
	reader: paged::Reader<R>,
	header: Header<V>,
	cache: Cache<V>,
}

impl<V: Vocabulary, R: io::Seek + io::Read> Module<V, R> {
	pub fn new(mut input: R) -> io::Result<Self> {
		let header = Header::<V>::decode(&mut input, &mut ())?;
		let first_page_offset = header.first_page_offset();
		input.seek(io::SeekFrom::Start(first_page_offset as u64))?;
		let reader = paged::Reader::new(input, header.page_size, first_page_offset);

		Ok(Self {
			reader,
			header,
			cache: Cache::default(),
		})
	}
}

impl<V: VocabularyMut, R: io::Seek + io::Read> inferdf_core::Module<V> for Module<V, R>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Iri: Clone,
	V::Literal: Clone,
	V::Value: AsRef<str> + From<String>,
{
	type Error = Error;

	type Dataset<'a> = Dataset<'a, V, R> where Self: 'a, V: 'a;

	type Interpretation<'a> = Interpretation<'a, V, R> where Self: 'a, V: 'a;

	type Classification<'a> = Classification<'a, V, R> where Self: 'a, V: 'a;

	fn dataset<'a>(&'a self) -> Self::Dataset<'a>
	where
		V: 'a,
	{
		Dataset::new(self)
	}

	fn interpretation<'a>(&'a self) -> Self::Interpretation<'a>
	where
		V: 'a,
	{
		Interpretation::new(self)
	}

	fn classification<'a>(&'a self) -> Self::Classification<'a>
	where
		V: 'a,
	{
		Classification::new(self)
	}
}
