use inferdf::{interpretation::{Interpret, InterpretationMut}, uninterpreted, Id};
use rdf_types::{Vocabulary, InsertIntoVocabulary, MapLiteral};
use serde::{Deserialize, Serialize};
use super::{Conclusion, Template};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Action<T = Id> {
	Conclusion(Conclusion<T>),
	Instantiate(Box<Template<T>>)
}

impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for Action<T> {
	type Inserted = Action<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		match self {
			Self::Conclusion(c) => Action::Conclusion(c.insert_into_vocabulary(vocabulary)),
			Self::Instantiate(t) => Action::Instantiate(Box::new(t.insert_into_vocabulary(vocabulary)))
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Action<T> {
	type Output = Action<T::Output>;

	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
		match self {
			Self::Conclusion(c) => Action::Conclusion(c.map_literal(f)),
			Self::Instantiate(t) => Action::Instantiate(Box::new(t.map_literal(f)))
		}
	}
}

impl<V: Vocabulary> Interpret<V> for Action<uninterpreted::Term<V>> {
	type Interpreted = Action;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		match self {
			Self::Conclusion(c) => Ok(Action::Conclusion(c.interpret(vocabulary, interpretation)?)),
			Self::Instantiate(t) => Ok(Action::Instantiate(Box::new(t.interpret(vocabulary, interpretation)?)))
		}
	}
}