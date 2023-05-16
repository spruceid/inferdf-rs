use crate::{Id, Signed, Pattern};

/// Inference rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rule {
	pub hypothesis: Hypothesis,
	pub conclusion: Conclusion
}

/// Rule hypohtesis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hypothesis {
	pub variables_count: usize,
	pub patterns: Vec<Signed<Pattern>>
}

/// Rule conclusion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Conclusion {
	pub variables_count: usize,
	pub statements: Vec<Signed<Statement>>
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Statement {
	Triple(Pattern),
	Eq(Id, Id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path {
	pub rule: usize,
	pub pattern: usize,
}