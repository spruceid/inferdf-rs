use crate::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cause {
	Stated(u32),
	Entailed(u32),
}

/// Triple entailment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entailment {
	/// Rule identifier.
	pub rule: Id,

	/// Rule variables substitution.
	pub substitution: Vec<Id>,
}

impl Entailment {
	pub fn new(rule: Id, substitution: Vec<Id>) -> Self {
		Self { rule, substitution }
	}
}
