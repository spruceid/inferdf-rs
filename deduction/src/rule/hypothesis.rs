use educe::Educe;
use rdf_types::{InsertIntoVocabulary, MapLiteral, Vocabulary};
use serde::{Deserialize, Serialize};

use inferdf::{
	interpretation::{Interpret, InterpretationMut},
	pattern::IdOrVar,
	uninterpreted, Id, Pattern, Signed,
};

/// Rule hypohtesis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Educe)]
#[educe(Default)]
#[serde(transparent)]
pub struct Hypothesis<T = Id> {
	pub patterns: Vec<Signed<Pattern<T>>>,
}

impl<T> Hypothesis<T> {
	pub fn new(patterns: Vec<Signed<Pattern<T>>>) -> Self {
		Self { patterns }
	}

	pub fn is_empty(&self) -> bool {
		self.patterns.is_empty()
	}

	pub fn visit_variables(&self, mut f: impl FnMut(usize)) {
		for Signed(_, p) in &self.patterns {
			if let IdOrVar::Var(x) = &p.0 {
				f(*x)
			}

			if let IdOrVar::Var(x) = &p.1 {
				f(*x)
			}

			if let IdOrVar::Var(x) = &p.2 {
				f(*x)
			}
		}
	}
}

impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for Hypothesis<T> {
	type Inserted = Hypothesis<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		Hypothesis {
			patterns: self.patterns.insert_into_vocabulary(vocabulary),
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Hypothesis<T> {
	type Output = Hypothesis<T::Output>;

	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
		Hypothesis {
			patterns: self.patterns.map_literal(f),
		}
	}
}

impl<V: Vocabulary> Interpret<V> for Hypothesis<uninterpreted::Term<V>> {
	type Interpreted = Hypothesis;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		Ok(Hypothesis {
			patterns: self.patterns.interpret(vocabulary, interpretation)?,
		})
	}
}
