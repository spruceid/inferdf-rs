use rdf_types::Vocabulary;

use crate::{uninterpreted, Id};

pub mod local;
pub mod composite;

pub struct Contradiction(pub Id, pub Id);

pub trait Resource<'a, V: Vocabulary>: Clone {
	type Iris: 'a + Iterator<Item = V::Iri>;
	type Blanks: 'a + Iterator<Item = V::BlankId>;
	type Literals: 'a + Iterator<Item = V::Literal>;
	type Ids: 'a + Iterator<Item = Id>;

	fn as_iri(&self) -> Self::Iris;

	fn as_blank(&self) -> Self::Blanks;

	fn as_literal(&self) -> Self::Literals;

	fn different_from(&self) -> Self::Ids;

	fn terms(&self) -> ResourceTerms<'a, V, Self> {
		ResourceTerms { as_iri: self.as_iri(), as_blank: self.as_blank(), as_literal: self.as_literal() }
	}
}

pub struct ResourceTerms<'a, V: Vocabulary, R: Resource<'a, V>> {
	as_iri: R::Iris,
	as_blank: R::Blanks,
	as_literal: R::Literals
}

impl<'a, V: Vocabulary, R: Resource<'a, V>> Iterator for ResourceTerms<'a, V, R> {
	type Item = uninterpreted::Term<V>;

	fn next(&mut self) -> Option<Self::Item> {
		self.as_iri
			.next()
			.map(|iri| uninterpreted::Term::<V>::Id(rdf_types::Id::Iri(iri)))
			.or_else(|| {
				self.as_blank
					.next()
					.map(|blank| uninterpreted::Term::<V>::Id(rdf_types::Id::Blank(blank)))
			})
			.or_else(|| {
				self.as_literal
					.next()
					.map(|literal| uninterpreted::Term::<V>::Literal(literal))
			})
	}
}

/// Iterator over the (uninterpreted) terms representing the given resource.
pub enum OptionalResourceTerms<'a, V: Vocabulary, R: Resource<'a, V>> {
	None,
	Some(ResourceTerms<'a, V, R>)
}

impl<'a, V: Vocabulary, R: Resource<'a, V>> Iterator for OptionalResourceTerms<'a, V, R> {
	type Item = uninterpreted::Term<V>;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::Some(i) => i.next()
		}
	}
}

/// Interpretation.
pub trait Interpretation<'a, V: Vocabulary>: Clone {
	type Resource: Resource<'a, V>;

	fn get(&self, id: Id) -> Option<Self::Resource>;

	fn term_interpretation(&self, term: uninterpreted::Term<V>) -> Option<Id>;

	fn terms_of(&self, id: Id) -> OptionalResourceTerms<'a, V, Self::Resource> {
		match self.get(id) {
			Some(r) => OptionalResourceTerms::Some(r.terms()),
			None => OptionalResourceTerms::None
		}
	}
}

pub trait InterpretationMut<'a, V: Vocabulary> {
	fn insert_term(&mut self, term: uninterpreted::Term<V>) -> Id;
}

pub trait Interpret<V: Vocabulary> {
	type Interpreted;

	fn interpret<'a, I: InterpretationMut<'a, V>>(self, interpretation: &mut I) -> Self::Interpreted;
}

impl<V: Vocabulary> Interpret<V> for uninterpreted::Term<V> {
	type Interpreted = Id;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		interpretation: &mut I,
	) -> Self::Interpreted {
		interpretation.insert_term(self)
	}
}

impl<V: Vocabulary, T: Interpret<V>> Interpret<V> for Vec<T> {
	type Interpreted = Vec<T::Interpreted>;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		interpretation: &mut I,
	) -> Self::Interpreted {
		self.into_iter()
			.map(|t| t.interpret(interpretation))
			.collect()
	}
}