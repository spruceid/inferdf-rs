mod id;
pub mod cause;
pub mod class;
pub mod dataset;
pub mod interpretation;
pub mod pattern;
pub mod uninterpreted;
pub mod module;
mod utils;

pub use id::*;
pub use cause::*;
pub use class::{Class, Classification};
pub use dataset::Dataset;
pub use interpretation::Interpretation;
use locspan::Meta;
pub use pattern::Pattern;
pub use module::Module;
pub use utils::*;

pub type Triple = rdf_types::Triple<Id, Id, Id>;
pub type Quad = rdf_types::Quad<Id, Id, Id, Id>;

pub type GraphFact = Meta<Signed<Triple>, Cause>;
pub type Fact = Meta<Signed<Quad>, Cause>;

pub trait TripleExt {
	fn into_pattern(self) -> Pattern;
}

impl TripleExt for Triple {
	fn into_pattern(self) -> Pattern {
		rdf_types::Triple(self.0.into(), self.1.into(), self.2.into())
	}
}