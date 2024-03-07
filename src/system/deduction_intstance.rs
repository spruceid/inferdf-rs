use educe::Educe;
use rdf_types::Term;

use crate::{Entailment, Signed, TripleStatement};

#[derive(Educe)]
#[educe(Default)]
pub struct DeductionsInstance<'r, T = Term>(pub(crate) Vec<DeductionInstance<'r, T>>);

impl<'r, T> DeductionsInstance<'r, T> {
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn merge_with(&mut self, other: Self) {
		self.0.extend(other.0)
	}
}

impl<'r, T> IntoIterator for DeductionsInstance<'r, T> {
	type IntoIter = std::vec::IntoIter<DeductionInstance<'r, T>>;
	type Item = DeductionInstance<'r, T>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl<'r, T> From<DeductionInstance<'r, T>> for DeductionsInstance<'r, T> {
	fn from(value: DeductionInstance<'r, T>) -> Self {
		Self(vec![value])
	}
}

/// Deduced statements with a common cause.
pub struct DeductionInstance<'r, T> {
	/// Rule and variable substitution triggering this deduction.
	pub entailment: Entailment<'r, T>,

	/// Deduced statements.
	pub statements: Vec<Signed<TripleStatement<T>>>,
}

impl<'r, T> DeductionInstance<'r, T> {
	pub fn new(entailment: Entailment<'r, T>) -> Self {
		Self {
			entailment,
			statements: Vec::new(),
		}
	}

	pub fn insert(&mut self, statement: Signed<TripleStatement<T>>) {
		self.statements.push(statement)
	}

	pub fn merge_with(&mut self, other: DeductionsInstance<'r, T>) {
		for s in other.0 {
			self.statements.extend(s.statements)
		}
	}
}
