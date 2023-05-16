use crate::{
	builder::QuadStatement,
	pattern::{IdOrVar, Instantiate, PatternSubstitution},
	Id, Pattern, Signed, Triple,
};

/// Inference rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rule {
	pub hypothesis: Hypothesis,
	pub conclusion: Conclusion,
}

/// Rule hypohtesis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hypothesis {
	pub variables_count: usize,
	pub patterns: Vec<Signed<Pattern>>,
}

/// Rule conclusion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Conclusion {
	pub variables_count: usize,
	pub statements: Vec<Signed<StatementPattern>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StatementPattern {
	Triple(Pattern),
	Eq(IdOrVar, IdOrVar),
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
