use std::io;

use educe::Educe;
use inferdf_core::{Id, IteratorWith};
use iref::Iri;
use langtag::LanguageTag;
use paged::{no_context_mut, ContextualIterator};
use rdf_types::{literal, IriVocabularyMut, Literal, LiteralVocabulary, Vocabulary, VocabularyMut};

use crate::header;

use super::{Error, Module};

#[derive(Educe)]
#[educe(Clone)]
pub struct Interpretation<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
}

impl<'a, V: Vocabulary, R> Interpretation<'a, V, R> {
	pub(crate) fn new(module: &'a Module<V, R>) -> Self {
		Self { module }
	}
}

impl<'a, V: VocabularyMut, R: io::Seek + io::Read> inferdf_core::Interpretation<'a, V>
	for Interpretation<'a, V, R>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Iri: Clone,
	V::Literal: Clone,
	V::Value: AsRef<str> + From<String>,
{
	type Error = Error;

	type Resource = Resource<'a, V, R>;

	type Resources = Resources<'a, V, R>;

	type Iris = Iris<'a, V, R>;

	type Literals = Literals<'a, V, R>;

	fn resources(&self) -> Result<Self::Resources, Self::Error> {
		Ok(Resources {
			module: self.module,
			r: self.module.reader.iter(
				self.module.header.interpretation.resources,
				&self.module.cache.interpretation_resources,
				self.module.header.heap,
			),
		})
	}

	fn get(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		Ok(self
			.module
			.reader
			.binary_search_by_key(
				self.module.header.interpretation.resources,
				&self.module.cache.interpretation_resources,
				no_context_mut(),
				self.module.header.heap,
				|entry, _| entry.id.cmp(&id),
			)?
			.map(|r| Resource {
				module: self.module,
				r,
			}))
	}

	fn iris(&self) -> Result<Self::Iris, Self::Error> {
		Ok(Iris {
			r: self.module.reader.iter(
				self.module.header.interpretation.iris,
				&self.module.cache.iris,
				self.module.header.heap,
			),
		})
	}

	fn iri_interpretation(
		&self,
		vocabulary: &mut V,
		iri: V::Iri,
	) -> Result<Option<Id>, Self::Error> {
		let entry = self.module.reader.binary_search_by_key(
			self.module.header.interpretation.iris,
			&self.module.cache.iris,
			vocabulary,
			self.module.header.heap,
			|entry, vocabulary| {
				let a = vocabulary.iri(&entry.iri.0).unwrap();
				let b = vocabulary.iri(&iri).unwrap();
				a.cmp(b)
			},
		)?;

		Ok(entry.map(|e| e.interpretation))
	}

	fn literals(&self) -> Result<Self::Literals, Self::Error> {
		Ok(Literals {
			r: self.module.reader.iter(
				self.module.header.interpretation.literals,
				&self.module.cache.literals,
				self.module.header.heap,
			),
		})
	}

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		literal: V::Literal,
	) -> Result<Option<Id>, Self::Error> {
		let entry = self.module.reader.binary_search_by_key(
			self.module.header.interpretation.literals,
			&self.module.cache.literals,
			vocabulary,
			self.module.header.heap,
			|entry, vocabulary| {
				let a = extract_literal(vocabulary, vocabulary.literal(&entry.literal.0).unwrap());
				let b = extract_literal(vocabulary, vocabulary.literal(&literal).unwrap());
				a.cmp(&b)
			},
		)?;

		Ok(entry.map(|e| e.interpretation))
	}
}

fn extract_literal<'a, V: Vocabulary>(
	vocabulary: &'a V,
	lit: &'a Literal<literal::Type<V::Iri, V::LanguageTag>, V::Value>,
) -> Literal<literal::Type<&'a Iri, LanguageTag<'a>>, &'a str>
where
	V::Value: AsRef<str>,
{
	let ty = match lit.type_() {
		literal::Type::Any(i) => literal::Type::Any(vocabulary.iri(i).unwrap()),
		literal::Type::LangString(t) => {
			literal::Type::LangString(vocabulary.language_tag(t).unwrap())
		}
	};

	Literal::new(lit.value().as_ref(), ty)
}

#[derive(Educe)]
#[educe(Clone)]
pub struct Resource<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	r: paged::EntryRef<'a, header::InterpretedResource>,
}

impl<'a, V: VocabularyMut, R: io::Seek + io::Read> inferdf_core::interpretation::Resource<'a, V>
	for Resource<'a, V, R>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Iri: Clone,
	V::Literal: Clone,
	V::Value: From<String>,
{
	type Error = Error;

	type Iris = ResourceIris<'a, V, R>;

	type Literals = ResourceLiterals<'a, V, R>;

	type DifferentFrom = DifferentFrom<'a>;

	fn as_iri(&self) -> Self::Iris {
		ResourceIris {
			module: self.module,
			r: self.r.clone().map(header::InterpretationResourceIrisBinder),
		}
	}

	fn as_literal(&self) -> Self::Literals {
		ResourceLiterals {
			module: self.module,
			r: self
				.r
				.clone()
				.map(header::InterpretationResourceLiteralsBinder),
		}
	}

	fn different_from(&self) -> Self::DifferentFrom {
		DifferentFrom {
			r: self.r.clone().map(header::InterpretationResourceNeBinder),
		}
	}
}

pub struct ResourceIris<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	r: paged::Ref<'a, header::InterpretedResource, paged::UnboundSliceIter<u32>>,
}

impl<'a, V: Vocabulary + IriVocabularyMut, R: io::Seek + io::Read> IteratorWith<V>
	for ResourceIris<'a, V, R>
where
	V::Iri: Clone,
{
	type Item = Result<V::Iri, Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.r.next().map(|r| {
			let iri_entry = self
				.module
				.reader
				.get(
					self.module.header.interpretation.iris,
					&self.module.cache.iris,
					vocabulary,
					self.module.header.heap,
					*r,
				)?
				.unwrap();

			Ok(iri_entry.iri.0.clone())
		})
	}
}

pub struct ResourceLiterals<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	r: paged::Ref<'a, header::InterpretedResource, paged::UnboundSliceIter<u32>>,
}

impl<'a, V: VocabularyMut, R: io::Seek + io::Read> IteratorWith<V> for ResourceLiterals<'a, V, R>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Literal: Clone,
	V::Value: From<String>,
{
	type Item = Result<V::Literal, Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.r.next().map(|r| {
			let literal_entry = self
				.module
				.reader
				.get(
					self.module.header.interpretation.literals,
					&self.module.cache.literals,
					vocabulary,
					self.module.header.heap,
					*r,
				)?
				.unwrap();

			Ok(literal_entry.literal.0.clone())
		})
	}
}

pub struct DifferentFrom<'a> {
	r: paged::Ref<'a, header::InterpretedResource, paged::UnboundSliceIter<Id>>,
}

impl<'a> Iterator for DifferentFrom<'a> {
	type Item = Result<Id, Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.r.next().map(|id| Ok(*id))
	}
}

impl<'a, V> IteratorWith<V> for DifferentFrom<'a> {
	type Item = Result<Id, Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct Resources<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	r: paged::Iter<'a, 'a, R, header::InterpretedResource>,
}

impl<'a, V: Vocabulary, R: io::Seek + io::Read> Iterator for Resources<'a, V, R> {
	type Item = Result<(Id, Resource<'a, V, R>), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.r.next().map(|r| {
			r.map(|r| {
				(
					r.id,
					Resource {
						module: self.module,
						r,
					},
				)
			})
		})
	}
}

impl<'a, V: Vocabulary, R: io::Seek + io::Read> IteratorWith<V> for Resources<'a, V, R> {
	type Item = Result<(Id, Resource<'a, V, R>), Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct Iris<'a, V: Vocabulary, R> {
	r: paged::Iter<'a, 'a, R, header::IriEntry<V>>,
}

impl<'a, V: Vocabulary + IriVocabularyMut, R: io::Seek + io::Read> IteratorWith<V>
	for Iris<'a, V, R>
where
	V::Iri: Clone,
{
	type Item = Result<(V::Iri, Id), Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.r
			.next_with(vocabulary)
			.map(|r| r.map(|entry| (entry.iri.0.clone(), entry.interpretation)))
	}
}

pub struct Literals<'a, V: Vocabulary, R> {
	r: paged::Iter<'a, 'a, R, header::LiteralEntry<V>>,
}

impl<'a, V: VocabularyMut, R: io::Seek + io::Read> IteratorWith<V> for Literals<'a, V, R>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Literal: Clone,
	V::Value: From<String>,
{
	type Item = Result<(V::Literal, Id), Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.r
			.next_with(vocabulary)
			.map(|r| r.map(|entry| (entry.literal.0.clone(), entry.interpretation)))
	}
}
