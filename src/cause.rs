#[cfg(feature = "paged")]
use paged::Paged;

use crate::Rule;

/// Cause of a deduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "paged", derive(Paged))]
pub enum Cause {
	/// The deducted fact is stated.
	Stated(u32),

	/// The deducted fact is entailed.
	Entailed(u32),
}

impl Cause {
	pub fn into_entailed(self) -> Option<u32> {
		match self {
			Self::Stated(_) => None,
			Self::Entailed(i) => Some(i),
		}
	}
}

/// Triple entailment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entailment<'r, T> {
	/// Rule reference.
	pub rule: &'r Rule<T>,

	/// Rule variables substitution.
	pub substitution: Vec<Option<T>>,
}

impl<'r, T> Entailment<'r, T> {
	pub fn new(rule: &'r Rule<T>, substitution: Vec<Option<T>>) -> Self {
		Self { rule, substitution }
	}
}
