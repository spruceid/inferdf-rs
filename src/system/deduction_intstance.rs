use educe::Educe;

use crate::{Entailment, Signed, TripleStatement};

#[derive(Educe)]
#[educe(Default)]
pub struct DeductionInstance<'r, T>(pub(crate) Vec<SubDeductionInstance<'r, T>>);

impl<'r, T> DeductionInstance<'r, T> {
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

impl<'r, T> IntoIterator for DeductionInstance<'r, T> {
	type IntoIter = std::vec::IntoIter<SubDeductionInstance<'r, T>>;
	type Item = SubDeductionInstance<'r, T>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}

impl<'r, T> From<SubDeductionInstance<'r, T>> for DeductionInstance<'r, T> {
	fn from(value: SubDeductionInstance<'r, T>) -> Self {
		Self(vec![value])
	}
}

/// Deduced statements with a common cause.
pub struct SubDeductionInstance<'r, T> {
	/// Rule and variable substitution triggering this deduction.
	pub entailment: Entailment<'r, T>,

	/// Deduced statements.
	pub statements: Vec<Signed<TripleStatement<T>>>,
}

impl<'r, T> SubDeductionInstance<'r, T> {
	pub fn new(entailment: Entailment<'r, T>) -> Self {
		Self {
			entailment,
			statements: Vec::new(),
		}
	}

	pub fn insert(&mut self, statement: Signed<TripleStatement<T>>) {
		self.statements.push(statement)
	}

	pub fn merge_with(&mut self, other: DeductionInstance<'r, T>) {
		for s in other.0 {
			self.statements.extend(s.statements)
		}
	}
}
