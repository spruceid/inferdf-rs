use rdf_types::{vocabulary::EmbedIntoVocabulary, Quad, Triple, Vocabulary};
use serde::{Deserialize, Serialize};

use crate::{
	expression::{Eval, Instantiate},
	pattern::{ApplyPartialSubstitution, ApplySubstitution, PatternSubstitution},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TripleStatement<T> {
	Triple(Triple<T, T, T>),
	Eq(T, T),
	True(T),
}

impl<T> TripleStatement<T> {
	pub fn with_graph(self, g: Option<T>) -> QuadStatement<T> {
		match self {
			Self::Triple(t) => QuadStatement::Quad(t.into_quad(g)),
			Self::Eq(a, b) => QuadStatement::Eq(a, b, g),
			Self::True(r) => QuadStatement::True(r),
		}
	}
}

impl<T, U: ApplySubstitution<T>> ApplySubstitution<T> for TripleStatement<U> {
	type Output = TripleStatement<U::Output>;

	fn apply_substitution(&self, substitution: &PatternSubstitution<T>) -> Option<Self::Output> {
		match self {
			Self::Triple(pattern) => Some(TripleStatement::Triple(
				pattern.apply_substitution(substitution)?,
			)),
			Self::Eq(a, b) => Some(TripleStatement::Eq(
				a.apply_substitution(substitution)?,
				b.apply_substitution(substitution)?,
			)),
			Self::True(r) => Some(TripleStatement::True(r.apply_substitution(substitution)?)),
		}
	}
}

impl<T, U: ApplyPartialSubstitution<T>> ApplyPartialSubstitution<T> for TripleStatement<U> {
	fn apply_partial_substitution(&self, substitution: &PatternSubstitution<T>) -> Self {
		match self {
			Self::Triple(pattern) => Self::Triple(pattern.apply_partial_substitution(substitution)),
			Self::Eq(a, b) => Self::Eq(
				a.apply_partial_substitution(substitution),
				b.apply_partial_substitution(substitution),
			),
			Self::True(r) => Self::True(r.apply_partial_substitution(substitution)),
		}
	}
}

impl<'e, V, I, T: Eval<'e, V, I>> Eval<'e, V, I> for TripleStatement<T> {
	type Output = TripleStatement<T::Output>;

	fn eval(
		&'e self,
		vocabulary: &V,
		interpretation: &I,
	) -> Result<Self::Output, crate::expression::Error> {
		match self {
			Self::Triple(pattern) => Ok(TripleStatement::Triple(
				pattern.eval(vocabulary, interpretation)?,
			)),
			Self::Eq(a, b) => Ok(TripleStatement::Eq(
				a.eval(vocabulary, interpretation)?,
				b.eval(vocabulary, interpretation)?,
			)),
			Self::True(r) => Ok(TripleStatement::True(r.eval(vocabulary, interpretation)?)),
		}
	}
}

impl<V, I, T: Instantiate<V, I>> Instantiate<V, I> for TripleStatement<T> {
	type Instantiated = TripleStatement<T::Instantiated>;

	fn instantiate(self, vocabulary: &mut V, interpretation: &mut I) -> Self::Instantiated {
		match self {
			Self::Triple(pattern) => {
				TripleStatement::Triple(pattern.instantiate(vocabulary, interpretation))
			}
			Self::Eq(a, b) => TripleStatement::Eq(
				a.instantiate(vocabulary, interpretation),
				b.instantiate(vocabulary, interpretation),
			),
			Self::True(r) => TripleStatement::True(r.instantiate(vocabulary, interpretation)),
		}
	}
}

impl<V: Vocabulary, T: EmbedIntoVocabulary<V>> EmbedIntoVocabulary<V> for TripleStatement<T> {
	type Embedded = TripleStatement<T::Embedded>;

	fn embed_into_vocabulary(self, vocabulary: &mut V) -> Self::Embedded {
		match self {
			Self::Triple(pattern) => {
				TripleStatement::Triple(pattern.embed_into_vocabulary(vocabulary))
			}
			Self::Eq(a, b) => TripleStatement::Eq(
				a.embed_into_vocabulary(vocabulary),
				b.embed_into_vocabulary(vocabulary),
			),
			Self::True(r) => TripleStatement::True(r.embed_into_vocabulary(vocabulary)),
		}
	}
}

/// RDF quad statement.
pub enum QuadStatement<T> {
	/// States that the given quad is asserted.
	Quad(Quad<T, T, T, T>),

	/// States that the given two resources are equals.
	Eq(T, T, Option<T>),

	/// States that the given value is the XSD boolean value `true`.
	True(T),
}
