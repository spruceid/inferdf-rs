use crate::{Signed, Quad, Triple, Pattern};

pub mod inference;

use inference::rule::Statement;

pub trait Context {
	type PatternMatching<'a>: 'a + Iterator<Item = Quad> where Self: 'a;

	fn pattern_matching(&self, pattern: Signed<Pattern>) -> Self::PatternMatching<'_>;
}

pub trait Semantics {
	fn deduce(
		&self,
		context: &impl Context,
		triple: Signed<Triple>,
		f: impl FnMut(Signed<Statement>)
	);
}