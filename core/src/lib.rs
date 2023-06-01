pub mod cause;
pub mod dataset;
pub mod interpretation;
pub mod pattern;
pub mod uninterpreted;
mod utils;

pub use cause::*;
pub use dataset::Dataset;
pub use interpretation::Interpretation;
pub use pattern::Pattern;
use rdf_types::Vocabulary;
pub use utils::*;

pub type Triple = rdf_types::Triple<Id, Id, Id>;
pub type Quad = rdf_types::Quad<Id, Id, Id, Id>;

pub trait Module<V: Vocabulary> {
	type Error;
	type Dataset<'a>: Dataset<'a, Error = Self::Error>
	where
		Self: 'a;
	type Interpretation<'a>: Interpretation<'a, V, Error = Self::Error>
	where
		Self: 'a;

	fn dataset(&self) -> Self::Dataset<'_>;

	fn interpretation(&self) -> Self::Interpretation<'_>;
}

pub trait TripleExt {
	fn into_pattern(self) -> Pattern;
}

impl TripleExt for Triple {
	fn into_pattern(self) -> Pattern {
		rdf_types::Triple(self.0.into(), self.1.into(), self.2.into())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u32);

impl Id {
	pub fn index(&self) -> usize {
		self.0 as usize
	}
}

impl From<u32> for Id {
	fn from(value: u32) -> Self {
		Self(value)
	}
}

impl From<Id> for u32 {
	fn from(value: Id) -> Self {
		value.0
	}
}
