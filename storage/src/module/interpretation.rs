use std::io::{Read, Seek};

use derivative::Derivative;
use inferdf_core::{Id, IteratorWith};
use rdf_types::Vocabulary;

use crate::{binary_search_page, page, DecodeWith};

use super::{cache, Error, Module};

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""))]
pub struct Interpretation<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
}

impl<'a, V: Vocabulary, R> Interpretation<'a, V, R> {
	pub fn new(module: &'a Module<V, R>) -> Self {
		Self { module }
	}
}

impl<'a, V: Vocabulary, R: Read + Seek> inferdf_core::Interpretation<'a, V>
	for Interpretation<'a, V, R>
where
	V::Type: Ord,
	V::Value: Ord,
	V::Iri: Copy + DecodeWith<V>,
	V::Literal: Copy + DecodeWith<V>,
{
	type Error = Error;
	type Resource = Resource<'a, V, R>;

	fn get(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		binary_search_page(
			self.module.sections.resources,
			self.module.sections.resources + self.module.header.resource_page_count,
			|i| {
				let page = self.module.get_resources_page(i)?;
				Ok(page.find(id).map(|r| Resource {
					module: self.module,
					entry: cache::Ref::map(page, |page| page.get(r).unwrap()),
				}))
			},
		)
	}

	fn iri_interpretation(
		&self,
		vocabulary: &mut V,
		iri: V::Iri,
	) -> Result<Option<Id>, Self::Error> {
		binary_search_page(
			self.module.sections.iris,
			self.module.sections.iris + self.module.header.iri_page_count,
			|i| {
				let page = self.module.get_iris_page(vocabulary, i)?;
				let iri = vocabulary.iri(&iri).unwrap();
				Ok(page
					.find(vocabulary, iri)
					.map(|k| page.get(k).unwrap().interpretation))
			},
		)
	}

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		literal: V::Literal,
	) -> Result<Option<Id>, Self::Error> {
		binary_search_page(
			self.module.sections.literals,
			self.module.sections.literals + self.module.header.literal_page_count,
			|i| {
				let page = self.module.get_literals_page(vocabulary, i)?;
				let literal = vocabulary.literal(&literal).unwrap();
				Ok(page
					.find(vocabulary, literal)
					.map(|k| page.get(k).unwrap().interpretation))
			},
		)
	}
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Resource<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	entry: cache::Ref<'a, page::resource_terms::Entry>,
}

impl<'a, V: Vocabulary, R: Read + Seek> inferdf_core::interpretation::Resource<'a, V>
	for Resource<'a, V, R>
where
	V::Iri: Copy + DecodeWith<V>,
	V::Literal: Copy + DecodeWith<V>,
{
	type Error = Error;
	type Iris = Iris<'a, V, R>;
	type Literals = Literals<'a, V, R>;
	type Ids = cache::IntoIterEscape<'a, page::resource_terms::DifferentFrom<'a>>;

	fn as_iri(&self) -> Self::Iris {
		Iris {
			module: self.module,
			iter: cache::Ref::aliasing_map(self.entry.clone(), |e| e.iter_known_iris()),
		}
	}

	fn as_literal(&self) -> Self::Literals {
		Literals {
			module: self.module,
			iter: cache::Ref::aliasing_map(self.entry.clone(), |e| e.iter_known_literals()),
		}
	}

	fn different_from(&self) -> Self::Ids {
		cache::Aliasing::into_iter_escape(cache::Ref::aliasing_map(self.entry.clone(), |e| {
			e.iter_different_from()
		}))
	}
}

pub struct Iris<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	iter: cache::Aliasing<'a, page::resource_terms::IriPaths<'a>>,
}

impl<'a, V: Vocabulary, R: Read + Seek> IteratorWith<V> for Iris<'a, V, R>
where
	V::Iri: Copy + DecodeWith<V>,
	V::Literal: Copy + DecodeWith<V>,
{
	type Item = Result<V::Iri, Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.iter
			.next()
			.map(|i| Ok(self.module.get_iri(vocabulary, i)?.unwrap()))
	}
}

pub struct Literals<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	iter: cache::Aliasing<'a, page::resource_terms::LiteralPaths<'a>>,
}

impl<'a, V: Vocabulary, R: Read + Seek> IteratorWith<V> for Literals<'a, V, R>
where
	V::Iri: Copy + DecodeWith<V>,
	V::Literal: Copy + DecodeWith<V>,
{
	type Item = Result<V::Literal, Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.iter
			.next()
			.map(|i| Ok(self.module.get_literal(vocabulary, i)?.unwrap()))
	}
}
