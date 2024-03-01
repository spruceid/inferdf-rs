use rdf_types::{Quad, Triple};

mod sign;
pub use sign::*;

mod trust;
pub use trust::*;

pub mod pattern;
pub use pattern::Pattern;

pub mod rule;
pub use rule::Rule;

pub mod system;
pub use system::System;

mod cause;
pub use cause::*;

mod dataset;
pub use dataset::{FallibleSignedPatternMatchingDataset, SignedPatternMatchingDataset};

pub mod expression;

pub mod utils;

pub type Fact<T> = Signed<Triple<T, T, T>>;

pub type FactRef<'a, T> = Signed<Triple<&'a T, &'a T, &'a T>>;

/// RDF quad statement.
pub enum QuadStatement<T> {
	/// States that the given quad is asserted.
	Quad(Quad<T, T, T, T>),

	/// States that the given two resources are equals.
	Eq(T, T, Option<T>),
}
