use crate::{
	dataset::{self, Dataset},
	pattern, Id, Quad, Signed, Triple,
};

pub mod inference;

use inference::rule::TripleStatement;

pub trait Context {
	type PatternMatching<'a>: 'a + Iterator<Item = Quad>
	where
		Self: 'a;

	fn pattern_matching(&self, pattern: Signed<pattern::Canonical>) -> Self::PatternMatching<'_>;
}

pub trait Semantics {
	fn deduce(
		&self,
		context: &impl Context,
		triple: Signed<Triple>,
		new_id: impl FnMut() -> Id,
		f: impl FnMut(Signed<TripleStatement>),
	);
}

impl<M> Context for Dataset<M> {
	type PatternMatching<'a> = dataset::MatchingQuads<'a, M> where M: 'a;

	fn pattern_matching(&self, pattern: Signed<pattern::Canonical>) -> Self::PatternMatching<'_> {
		self.signed_matching(pattern).into_quads()
	}
}
