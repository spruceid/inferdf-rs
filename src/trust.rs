use rdf_types::vocabulary::{EmbedIntoVocabulary, Vocabulary};
use serde::{Deserialize, Serialize};

use crate::pattern::{ApplyPartialSubstitution, ApplySubstitution, PatternSubstitution};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Trust {
	Trusted,
	Untrusted,
}

impl Trust {
	pub fn is_trusted(&self) -> bool {
		matches!(self, Self::Trusted)
	}

	pub fn is_untrusted(&self) -> bool {
		matches!(self, Self::Untrusted)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MaybeTrusted<T> {
	Trusted(T),
	Untrusted(T),
}

impl<T> MaybeTrusted<T> {
	pub fn new(value: T, trust: Trust) -> Self {
		match trust {
			Trust::Trusted => Self::Trusted(value),
			Trust::Untrusted => Self::Untrusted(value),
		}
	}

	pub fn value(&self) -> &T {
		match self {
			Self::Trusted(t) => t,
			Self::Untrusted(t) => t,
		}
	}

	pub fn as_parts(&self) -> (&T, Trust) {
		match self {
			Self::Trusted(t) => (t, Trust::Trusted),
			Self::Untrusted(t) => (t, Trust::Untrusted),
		}
	}

	pub fn into_parts(self) -> (T, Trust) {
		match self {
			Self::Trusted(t) => (t, Trust::Trusted),
			Self::Untrusted(t) => (t, Trust::Untrusted),
		}
	}

	pub fn map<U>(self, f: impl FnOnce(T) -> U) -> MaybeTrusted<U> {
		match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(f(t)),
			Self::Untrusted(t) => MaybeTrusted::Untrusted(f(t)),
		}
	}
}

impl<V: Vocabulary, T: EmbedIntoVocabulary<V>> EmbedIntoVocabulary<V> for MaybeTrusted<T> {
	type Embedded = MaybeTrusted<T::Embedded>;

	fn embed_into_vocabulary(self, vocabulary: &mut V) -> Self::Embedded {
		match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(t.embed_into_vocabulary(vocabulary)),
			Self::Untrusted(t) => MaybeTrusted::Untrusted(t.embed_into_vocabulary(vocabulary)),
		}
	}
}

// impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for MaybeTrusted<T> {
// 	type Output = MaybeTrusted<T::Output>;

// 	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
// 		match self {
// 			Self::Trusted(t) => MaybeTrusted::Trusted(t.map_literal(f)),
// 			Self::Untrusted(t) => MaybeTrusted::Untrusted(t.map_literal(f)),
// 		}
// 	}
// }

// impl<V: Vocabulary, T: Interpret<V>> Interpret<V> for MaybeTrusted<T> {
// 	type Interpreted = MaybeTrusted<T::Interpreted>;

// 	fn interpret<'a, I: InterpretationMut<'a, V>>(
// 		self,
// 		vocabulary: &mut V,
// 		interpretation: &mut I,
// 	) -> Result<Self::Interpreted, I::Error> {
// 		Ok(match self {
// 			Self::Trusted(t) => MaybeTrusted::Trusted(t.interpret(vocabulary, interpretation)?),
// 			Self::Untrusted(t) => MaybeTrusted::Untrusted(t.interpret(vocabulary, interpretation)?),
// 		})
// 	}
// }

impl<T, U: ApplySubstitution<T>> ApplySubstitution<T> for MaybeTrusted<U> {
	type Output = MaybeTrusted<U::Output>;

	fn apply_substitution(&self, substitution: &PatternSubstitution<T>) -> Option<Self::Output> {
		match self {
			Self::Trusted(t) => Some(MaybeTrusted::Trusted(t.apply_substitution(substitution)?)),
			Self::Untrusted(t) => {
				Some(MaybeTrusted::Untrusted(t.apply_substitution(substitution)?))
			}
		}
	}
}

impl<T, U: ApplyPartialSubstitution<T>> ApplyPartialSubstitution<T> for MaybeTrusted<U> {
	fn apply_partial_substitution(&self, substitution: &PatternSubstitution<T>) -> Self {
		match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(t.apply_partial_substitution(substitution)),
			Self::Untrusted(t) => {
				MaybeTrusted::Untrusted(t.apply_partial_substitution(substitution))
			}
		}
	}
}
