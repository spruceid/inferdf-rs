use rdf_types::vocabulary::{EmbedIntoVocabulary, Vocabulary};
use serde::{Deserialize, Serialize};

use crate::pattern::{ApplyPartialSubstitution, ApplySubstitution, PatternSubstitution};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Mode {
	Asserted,
	Required,
}

impl Mode {
	pub fn is_asserted(&self) -> bool {
		matches!(self, Self::Asserted)
	}

	pub fn is_required(&self) -> bool {
		matches!(self, Self::Required)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Modal<T> {
	Asserted(T),
	Required(T),
}

impl<T> Modal<T> {
	pub fn new(value: T, mode: Mode) -> Self {
		match mode {
			Mode::Asserted => Self::Asserted(value),
			Mode::Required => Self::Required(value),
		}
	}

	pub fn value(&self) -> &T {
		match self {
			Self::Asserted(t) => t,
			Self::Required(t) => t,
		}
	}

	pub fn as_parts(&self) -> (&T, Mode) {
		match self {
			Self::Asserted(t) => (t, Mode::Asserted),
			Self::Required(t) => (t, Mode::Required),
		}
	}

	pub fn into_parts(self) -> (T, Mode) {
		match self {
			Self::Asserted(t) => (t, Mode::Asserted),
			Self::Required(t) => (t, Mode::Required),
		}
	}

	pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Modal<U> {
		match self {
			Self::Asserted(t) => Modal::Asserted(f(t)),
			Self::Required(t) => Modal::Required(f(t)),
		}
	}
}

impl<V: Vocabulary, T: EmbedIntoVocabulary<V>> EmbedIntoVocabulary<V> for Modal<T> {
	type Embedded = Modal<T::Embedded>;

	fn embed_into_vocabulary(self, vocabulary: &mut V) -> Self::Embedded {
		match self {
			Self::Asserted(t) => Modal::Asserted(t.embed_into_vocabulary(vocabulary)),
			Self::Required(t) => Modal::Required(t.embed_into_vocabulary(vocabulary)),
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

impl<T, U: ApplySubstitution<T>> ApplySubstitution<T> for Modal<U> {
	type Output = Modal<U::Output>;

	fn apply_substitution(&self, substitution: &PatternSubstitution<T>) -> Option<Self::Output> {
		match self {
			Self::Asserted(t) => Some(Modal::Asserted(t.apply_substitution(substitution)?)),
			Self::Required(t) => Some(Modal::Required(t.apply_substitution(substitution)?)),
		}
	}
}

impl<T, U: ApplyPartialSubstitution<T>> ApplyPartialSubstitution<T> for Modal<U> {
	fn apply_partial_substitution(&self, substitution: &PatternSubstitution<T>) -> Self {
		match self {
			Self::Asserted(t) => Modal::Asserted(t.apply_partial_substitution(substitution)),
			Self::Required(t) => Modal::Required(t.apply_partial_substitution(substitution)),
		}
	}
}
