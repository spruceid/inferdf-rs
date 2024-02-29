use rdf_types::{vocabulary::EmbedIntoVocabulary, Triple, Vocabulary};
use serde::{Deserialize, Serialize};

use crate::{
	pattern::{ApplyPartialSubstitution, ApplySubstitution, PatternSubstitution, ResourceOrVar},
	MaybeTrusted, QuadStatement, Signed,
};

use super::Variable;

/// Rule conclusion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Conclusion<T> {
	/// Variables introduced in the conclusion.
	pub variables: Vec<Variable>,

	/// Concluded statements.
	pub statements: Vec<MaybeTrusted<Signed<TripleStatementPattern<T>>>>,
}

impl<T> Conclusion<T> {
	pub fn new(
		variables: Vec<Variable>,
		statements: Vec<MaybeTrusted<Signed<TripleStatementPattern<T>>>>,
	) -> Self {
		Self {
			variables,
			statements,
		}
	}

	pub fn visit_variables(&self, mut f: impl FnMut(usize)) {
		for p in &self.statements {
			match &p.value().1 {
				TripleStatementPattern::Eq(s, o) => {
					if let ResourceOrVar::Var(x) = s {
						f(*x)
					}

					if let ResourceOrVar::Var(x) = o {
						f(*x)
					}
				}
				TripleStatementPattern::Triple(rdf_types::Triple(s, p, o)) => {
					if let ResourceOrVar::Var(x) = s {
						f(*x)
					}

					if let ResourceOrVar::Var(x) = p {
						f(*x)
					}

					if let ResourceOrVar::Var(x) = o {
						f(*x)
					}
				}
			}
		}
	}
}

impl<V: Vocabulary, T: EmbedIntoVocabulary<V>> EmbedIntoVocabulary<V> for Conclusion<T> {
	type Embedded = Conclusion<T::Embedded>;

	fn embed_into_vocabulary(self, vocabulary: &mut V) -> Self::Embedded {
		Conclusion {
			variables: self.variables,
			statements: self.statements.embed_into_vocabulary(vocabulary),
		}
	}
}

// impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Conclusion<T> {
// 	type Output = Conclusion<T::Output>;

// 	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
// 		Conclusion {
// 			variables: self.variables,
// 			statements: self.statements.map_literal(f),
// 		}
// 	}
// }

pub type TripleStatementPattern<T> = TripleStatement<ResourceOrVar<T>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TripleStatement<T> {
	Triple(Triple<T, T, T>),
	Eq(T, T),
}

impl<T> TripleStatement<T> {
	pub fn with_graph(self, g: Option<T>) -> QuadStatement<T> {
		match self {
			Self::Triple(t) => QuadStatement::Quad(t.into_quad(g)),
			Self::Eq(a, b) => QuadStatement::Eq(a, b, g),
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
		}
	}
}

// impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for TripleStatement<T> {
// 	type Output = TripleStatement<T::Output>;

// 	fn map_literal(self, mut f: impl FnMut(L) -> M) -> Self::Output {
// 		match self {
// 			Self::Triple(pattern) => TripleStatement::Triple(pattern.map_literal(f)),
// 			Self::Eq(a, b) => TripleStatement::Eq(a.map_literal(&mut f), b.map_literal(f)),
// 		}
// 	}
// }

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
		}
	}
}
