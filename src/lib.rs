use rdf_types::{Term, Triple};
use std::hash::Hash;

pub use rdf_types;
pub use static_iref;

mod sign;
pub use sign::*;

mod mode;
pub use mode::*;

mod statement;
pub use statement::*;

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
pub use expression::Expression;

pub mod utils;

pub type Fact<T> = Signed<Triple<T, T, T>>;

pub type FactRef<'a, T> = Signed<Triple<&'a T, &'a T, &'a T>>;

#[derive(Debug, thiserror::Error)]
pub enum Error<I, D> {
	#[error("interpretation error: {0}")]
	Interpretation(I),

	#[error("dataset error: {0}")]
	Dataset(D),
}

pub enum ValidationError<D> {
	Dataset(D),
	Expression(expression::Error),
}

impl ValidationError<std::convert::Infallible> {
	pub fn into_expression_error(self) -> expression::Error {
		match self {
			Self::Dataset(_) => unreachable!(),
			Self::Expression(e) => e,
		}
	}
}

impl From<ValidationError<std::convert::Infallible>> for expression::Error {
	fn from(value: ValidationError<std::convert::Infallible>) -> Self {
		value.into_expression_error()
	}
}

/// Validation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Validation<R = Term> {
	/// Dataset is valid.
	Ok,

	/// Dataset is invalid for the given reason.
	Invalid(Reason<R>),
}

impl<R> Validation<R> {
	pub fn is_valid(&self) -> bool {
		matches!(self, Self::Ok)
	}

	pub fn is_invalid(&self) -> bool {
		matches!(self, Self::Invalid(_))
	}
}

/// Reason why validation could fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reason<R = Term> {
	/// The given triple is required by a deduction rule, but not found in the
	/// dataset.
	MissingTriple(Signed<Triple<R>>),

	/// The given two resources are expected to be equals, but are not.
	NotEq(R, R),

	/// The given two resources are expected to be different, but are equals.
	NotNe(R, R),

	/// The given resource is expected to be the XSD boolean value `true`, but
	/// is not.
	NotTrue(R),

	/// The given resource is expected to be the XSD boolean value `false`, but
	/// is not.
	NotFalse(R),
}
