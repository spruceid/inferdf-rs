use crate::{pattern, Id, Quad, Signed, Triple};

pub mod inference;

use inference::rule::TripleStatement;

pub trait Context {
	type PatternMatching<'a>: 'a + Iterator<Item = Quad>
	where
		Self: 'a;

	fn pattern_matching(&self, pattern: Signed<pattern::Canonical>) -> Self::PatternMatching<'_>;

	fn new_resource(&mut self) -> Id;
}

pub trait Semantics {
	fn deduce(
		&self,
		context: &mut impl Context,
		triple: Signed<Triple>,
		f: impl FnMut(Signed<TripleStatement>),
	);
}
