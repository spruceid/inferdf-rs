use rdf_types::{InsertIntoVocabulary, MapLiteral, Vocabulary};
use serde::{Deserialize, Serialize};

use inferdf_core::{
	interpretation::{Interpret, InterpretationMut},
	pattern::{IdOrVar, Instantiate, PatternSubstitution},
	uninterpreted, Id, Pattern, Signed, Triple,
};

use crate::{builder::QuadStatement, semantics::MaybeTrusted};

use super::Variable;

/// Rule conclusion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Conclusion<T = Id> {
	pub variables: Vec<Variable>,
	pub statements: Vec<MaybeTrusted<Signed<StatementPattern<T>>>>,
}

impl<T> Conclusion<T> {
	pub fn new(
		variables: Vec<Variable>,
		statements: Vec<MaybeTrusted<Signed<StatementPattern<T>>>>,
	) -> Self {
		Self {
			variables,
			statements,
		}
	}

	pub fn visit_variables(&self, mut f: impl FnMut(usize)) {
		for p in &self.statements {
			match &p.value().1 {
				StatementPattern::Eq(s, o) => {
					if let IdOrVar::Var(x) = s {
						f(*x)
					}

					if let IdOrVar::Var(x) = o {
						f(*x)
					}
				}
				StatementPattern::Triple(rdf_types::Triple(s, p, o)) => {
					if let IdOrVar::Var(x) = s {
						f(*x)
					}

					if let IdOrVar::Var(x) = p {
						f(*x)
					}

					if let IdOrVar::Var(x) = o {
						f(*x)
					}
				}
			}
		}
	}
}

impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for Conclusion<T> {
	type Inserted = Conclusion<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		Conclusion {
			variables: self.variables,
			statements: self.statements.insert_into_vocabulary(vocabulary),
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Conclusion<T> {
	type Output = Conclusion<T::Output>;

	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
		Conclusion {
			variables: self.variables,
			statements: self.statements.map_literal(f),
		}
	}
}

impl<V: Vocabulary> Interpret<V> for Conclusion<uninterpreted::Term<V>> {
	type Interpreted = Conclusion;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		Ok(Conclusion {
			variables: self.variables,
			statements: self.statements.interpret(vocabulary, interpretation)?,
		})
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
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		match self {
			Self::Triple(pattern) => Ok(StatementPattern::Triple(
				pattern.interpret(vocabulary, interpretation)?,
			)),
			Self::Eq(a, b) => Ok(StatementPattern::Eq(
				a.interpret(vocabulary, interpretation)?,
				b.interpret(vocabulary, interpretation)?,
			)),
		}
	}
}

impl Instantiate for StatementPattern {
	type Output = TripleStatement;

	fn instantiate(&self, substitution: &PatternSubstitution) -> Option<Self::Output> {
		match self {
			Self::Triple(pattern) => {
				Some(TripleStatement::Triple(pattern.instantiate(substitution)?))
			}
			Self::Eq(a, b) => Some(TripleStatement::Eq(
				a.instantiate(substitution)?,
				b.instantiate(substitution)?,
			)),
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
