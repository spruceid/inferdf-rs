use rdf_types::Vocabulary;

use crate::{uninterpreted, Id, IteratorWith};

pub mod composite;
pub mod local;

pub struct Contradiction(pub Id, pub Id);

pub trait Resource<'a, V: Vocabulary>: Clone {
	type Error;
	type Iris: 'a + IteratorWith<V, Item = Result<V::Iri, Self::Error>>;
	type Literals: 'a + IteratorWith<V, Item = Result<V::Literal, Self::Error>>;
	type Ids: 'a + Iterator<Item = Id>;

	fn as_iri(&self) -> Self::Iris;

	// fn as_blank(&self) -> Self::Blanks;

	fn as_literal(&self) -> Self::Literals;

	fn different_from(&self) -> Self::Ids;

	fn terms(&self) -> ResourceTerms<'a, V, Self> {
		ResourceTerms {
			as_iri: self.as_iri(),
			// as_blank: self.as_blank(),
			as_literal: self.as_literal(),
		}
	}
}

pub struct ResourceTerms<'a, V: Vocabulary, R: Resource<'a, V>> {
	as_iri: R::Iris,
	// as_blank: R::Blanks,
	as_literal: R::Literals,
}

impl<'a, V: Vocabulary, R: Resource<'a, V>> IteratorWith<V> for ResourceTerms<'a, V, R> {
	type Item = Result<uninterpreted::Term<V>, R::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		match self.as_iri.next_with(vocabulary) {
			Some(Ok(iri)) => Some(Ok(uninterpreted::Term::<V>::Id(rdf_types::Id::Iri(iri)))),
			Some(Err(e)) => Some(Err(e)),
			None => match self.as_literal.next_with(vocabulary) {
				Some(Ok(literal)) => Some(Ok(uninterpreted::Term::<V>::Literal(literal))),
				Some(Err(e)) => Some(Err(e)),
				None => None,
			},
		}
	}
}

/// Iterator over the (uninterpreted) terms representing the given resource.
pub enum OptionalResourceTerms<'a, V: Vocabulary, R: Resource<'a, V>> {
	None,
	Some(ResourceTerms<'a, V, R>),
}

impl<'a, V: Vocabulary, R: Resource<'a, V>> IteratorWith<V> for OptionalResourceTerms<'a, V, R> {
	type Item = Result<uninterpreted::Term<V>, R::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::Some(i) => i.next_with(vocabulary),
		}
	}
}

/// Interpretation.
pub trait Interpretation<'a, V: Vocabulary>: Clone {
	type Error;
	type Resource: Resource<'a, V, Error = Self::Error>;

	fn get(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error>;

	fn iri_interpretation(
		&self,
		vocabulary: &mut V,
		iri: V::Iri,
	) -> Result<Option<Id>, Self::Error>;

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		literal: V::Literal,
	) -> Result<Option<Id>, Self::Error>;

	fn term_interpretation(
		&self,
		vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Option<Id>, Self::Error> {
		match term {
			uninterpreted::Term::<V>::Id(rdf_types::Id::Iri(iri)) => {
				self.iri_interpretation(vocabulary, iri)
			}
			uninterpreted::Term::<V>::Id(rdf_types::Id::Blank(_)) => Ok(None),
			uninterpreted::Term::<V>::Literal(l) => self.literal_interpretation(vocabulary, l),
		}
	}

	fn terms_of(
		&self,
		id: Id,
	) -> Result<OptionalResourceTerms<'a, V, Self::Resource>, Self::Error> {
		match self.get(id)? {
			Some(r) => Ok(OptionalResourceTerms::Some(r.terms())),
			None => Ok(OptionalResourceTerms::None),
		}
	}
}

pub trait InterpretationMut<'a, V: Vocabulary> {
	type Error;

	fn insert_term(
		&mut self,
		vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Id, Self::Error>;
}

pub trait Interpret<V: Vocabulary> {
	type Interpreted;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error>;
}

impl<V: Vocabulary> Interpret<V> for uninterpreted::Term<V> {
	type Interpreted = Id;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		interpretation.insert_term(vocabulary, self)
	}
}

impl<V: Vocabulary, T: Interpret<V>> Interpret<V> for Vec<T> {
	type Interpreted = Vec<T::Interpreted>;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		let mut result = Vec::with_capacity(self.len());

		for t in self {
			result.push(t.interpret(vocabulary, interpretation)?)
		}

		Ok(result)
	}
}
