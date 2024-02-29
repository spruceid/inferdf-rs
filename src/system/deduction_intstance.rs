use educe::Educe;

use crate::{Entailment, MaybeTrusted, Signed};

use super::TripleStatement;

#[derive(Educe)]
#[educe(Default)]
pub struct DeductionInstance<T>(Vec<SubDeductionInstance<T>>);

impl<T> DeductionInstance<T> {
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn merge_with(&mut self, other: Self) {
		self.0.extend(other.0)
	}

	// pub fn collect(
	// 	self,
	// 	mut entailment_index: impl FnMut(Entailment<T>) -> u32,
	// 	mut new_triple: impl FnMut(Meta<MaybeTrusted<Signed<TripleStatement<T>>>, Cause>),
	// ) {
	// 	for s in self.0 {
	// 		let e = entailment_index(s.entailment);
	// 		for statement in s.statements {
	// 			new_triple(Meta(statement, Cause::Entailed(e)))
	// 		}
	// 	}
	// }
}

impl<T> From<SubDeductionInstance<T>> for DeductionInstance<T> {
	fn from(value: SubDeductionInstance<T>) -> Self {
		Self(vec![value])
	}
}

/// Deduced statements with a common cause.
pub struct SubDeductionInstance<T> {
	/// Rule and variable substitution triggering this deduction.
	pub entailment: Entailment<T>,

	/// Deduced statements.
	pub statements: Vec<MaybeTrusted<Signed<TripleStatement<T>>>>,
}

impl<T> SubDeductionInstance<T> {
	pub fn new(entailment: Entailment<T>) -> Self {
		Self {
			entailment,
			statements: Vec::new(),
		}
	}

	pub fn insert(&mut self, statement: MaybeTrusted<Signed<TripleStatement<T>>>) {
		self.statements.push(statement)
	}

	pub fn merge_with(&mut self, other: DeductionInstance<T>) {
		for s in other.0 {
			self.statements.extend(s.statements)
		}
	}
}
