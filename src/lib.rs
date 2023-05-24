pub mod builder;
pub mod cause;
pub mod dataset;
pub mod interpretation;
pub mod pattern;
pub mod semantics;
mod uninterpreted;
mod utils;

pub use builder::Builder;
pub use cause::Cause;
pub use pattern::Pattern;
pub use utils::*;

pub type Triple = rdf_types::Triple<Id, Id, Id>;
pub type Quad = rdf_types::Quad<Id, Id, Id, Id>;

pub trait TripleExt {
	fn into_pattern(self) -> Pattern;
}

impl TripleExt for Triple {
	fn into_pattern(self) -> Pattern {
		rdf_types::Triple(self.0.into(), self.1.into(), self.2.into())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);
