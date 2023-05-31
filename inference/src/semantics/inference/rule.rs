use rdf_types::{InsertIntoVocabulary, MapLiteral, Vocabulary};
use serde::{Deserialize, Serialize};

use inferdf_core::{
	interpretation::{Interpret, InterpretationMut},
	pattern::{IdOrVar, Instantiate, PatternSubstitution},
	uninterpreted, Id, Pattern, Signed, Triple,
};

use crate::builder::QuadStatement;

/// Inference rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Rule<T = Id> {
	pub hypothesis: Hypothesis<T>,
	pub conclusion: Conclusion<T>,
}

impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for Rule<T> {
	type Inserted = Rule<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		Rule {
			hypothesis: self.hypothesis.insert_into_vocabulary(vocabulary),
			conclusion: self.conclusion.insert_into_vocabulary(vocabulary),
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Rule<T> {
	type Output = Rule<T::Output>;

	fn map_literal(self, mut f: impl FnMut(L) -> M) -> Self::Output {
		Rule {
			hypothesis: self.hypothesis.map_literal(&mut f),
			conclusion: self.conclusion.map_literal(f),
		}
	}
}

impl<V: Vocabulary> Interpret<V> for Rule<uninterpreted::Term<V>> {
	type Interpreted = Rule;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		interpretation: &mut I,
	) -> Self::Interpreted {
		Rule {
			hypothesis: self.hypothesis.interpret(interpretation),
			conclusion: self.conclusion.interpret(interpretation),
		}
	}
}

/// Rule hypohtesis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hypothesis<T = Id> {
	pub patterns: Vec<Signed<Pattern<T>>>,
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
		interpretation: &mut I,
	) -> Self::Interpreted {
		Hypothesis {
			patterns: self.patterns.interpret(interpretation),
		}
	}
}

/// Rule conclusion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Conclusion<T = Id> {
	pub statements: Vec<Signed<StatementPattern<T>>>,
}

impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for Conclusion<T> {
	type Inserted = Conclusion<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		Conclusion {
			statements: self.statements.insert_into_vocabulary(vocabulary),
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Conclusion<T> {
	type Output = Conclusion<T::Output>;

	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
		Conclusion {
			statements: self.statements.map_literal(f),
		}
	}
}

impl<V: Vocabulary> Interpret<V> for Conclusion<uninterpreted::Term<V>> {
	type Interpreted = Conclusion;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		interpretation: &mut I,
	) -> Self::Interpreted {
		Conclusion {
			statements: self.statements.interpret(interpretation),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StatementPattern<T = Id> {
	Triple(Pattern<T>),
	Eq(IdOrVar<T>, IdOrVar<T>),
}

impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for StatementPattern<T> {
	type Inserted = StatementPattern<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		match self {
			Self::Triple(pattern) => {
				StatementPattern::Triple(pattern.insert_into_vocabulary(vocabulary))
			}
			Self::Eq(a, b) => StatementPattern::Eq(
				a.insert_into_vocabulary(vocabulary),
				b.insert_into_vocabulary(vocabulary),
			),
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for StatementPattern<T> {
	type Output = StatementPattern<T::Output>;

	fn map_literal(self, mut f: impl FnMut(L) -> M) -> Self::Output {
		match self {
			Self::Triple(pattern) => StatementPattern::Triple(pattern.map_literal(f)),
			Self::Eq(a, b) => StatementPattern::Eq(a.map_literal(&mut f), b.map_literal(f)),
		}
	}
}

impl<V: Vocabulary> Interpret<V> for StatementPattern<uninterpreted::Term<V>> {
	type Interpreted = StatementPattern;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		interpretation: &mut I,
	) -> Self::Interpreted {
		match self {
			Self::Triple(pattern) => StatementPattern::Triple(pattern.interpret(interpretation)),
			Self::Eq(a, b) => {
				StatementPattern::Eq(a.interpret(interpretation), b.interpret(interpretation))
			}
		}
	}
}

impl Instantiate for StatementPattern {
	type Output = TripleStatement;

	fn instantiate(
		&self,
		substitution: &mut PatternSubstitution,
		mut new_id: impl FnMut() -> Id,
	) -> Self::Output {
		match self {
			Self::Triple(pattern) => {
				TripleStatement::Triple(pattern.instantiate(substitution, new_id))
			}
			Self::Eq(a, b) => TripleStatement::Eq(
				a.instantiate(substitution, &mut new_id),
				b.instantiate(substitution, &mut new_id),
			),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TripleStatement {
	Triple(Triple),
	Eq(Id, Id),
}

impl TripleStatement {
	pub fn with_graph(self, g: Option<Id>) -> QuadStatement {
		match self {
			Self::Triple(t) => QuadStatement::Quad(t.into_quad(g)),
			Self::Eq(a, b) => QuadStatement::Eq(a, b, g),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path {
	pub rule: usize,
	pub pattern: usize,
}

impl Path {
	pub fn new(rule: usize, pattern: usize) -> Self {
		Self { rule, pattern }
	}
}
