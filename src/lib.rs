pub mod cause;
pub mod interpretation;
pub mod dataset;
// pub mod rule;

pub use cause::Cause;

pub type Triple = rdf_types::Triple<Id, Id, Id>;
pub type Pattern = rdf_types::Triple<Option<Id>, Option<Id>, Option<Id>>;
pub type Quad = rdf_types::Quad<Id, Id, Id, Id>;

pub trait TripleExt {
	fn into_pattern(self)-> Pattern;
}

impl TripleExt for Triple {
	fn into_pattern(self)-> Pattern {
		rdf_types::Triple(Some(self.0), Some(self.1), Some(self.2))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);