#[cfg(feature = "paged")]
use paged::Paged;

use crate::Rule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "paged", derive(Paged))]
pub enum Cause {
	Stated(u32),
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
