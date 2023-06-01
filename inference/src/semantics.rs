use inferdf_core::{pattern, Cause, Entailment, Id, Quad, Signed, Triple};

pub mod inference;

use inference::rule::TripleStatement;
use locspan::Meta;

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
		entailment_index: impl FnMut(Entailment) -> u32,
		new_triple: impl FnMut(Meta<Signed<TripleStatement>, Cause>),
	);
}
