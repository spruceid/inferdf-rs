use educe::Educe;

use crate::{rule::TripleStatementPattern, Entailment, MaybeTrusted, Signed};

#[derive(Educe)]
#[educe(Default)]
pub struct Deduction<T>(Vec<SubDeduction<T>>);

impl<T> Deduction<T> {
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn merge_with(&mut self, other: Self) {
		self.0.extend(other.0)
	}
}

impl<T> From<SubDeduction<T>> for Deduction<T> {
	fn from(value: SubDeduction<T>) -> Self {
		Self(vec![value])
	}
}

/// Deduced statements with a common cause.
pub struct SubDeduction<T> {
	/// Rule and variable substitution triggering this deduction.
	pub entailment: Entailment<T>,

	/// Deduced statements.
	pub statements: Vec<MaybeTrusted<Signed<TripleStatementPattern<T>>>>,
}

impl<T> SubDeduction<T> {
	pub fn new(entailment: Entailment<T>) -> Self {
		Self {
			entailment,
			statements: Vec::new(),
		}
	}

	pub fn insert(&mut self, statement: MaybeTrusted<Signed<TripleStatementPattern<T>>>) {
		self.statements.push(statement)
	}

	pub fn merge_with(&mut self, other: Deduction<T>) {
		for s in other.0 {
			self.statements.extend(s.statements)
		}
	}
}
