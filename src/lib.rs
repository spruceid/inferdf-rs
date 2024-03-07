//! InfeRDF is an RDF deduction library providing tools to infer implicit
//! statements and/or validate facts derived from an RDF dataset.
//!
//! # Usage
//!
//! InfeRDF works by defining deduction rules that can match against any given
//! RDF dataset. Each deduction [`Rule`] is an implication of the form
//! `hypotheses => conclusions` where each hypothesis is a non-linear
//! [RDF triple pattern][triple-pattern], and each conclusion a
//! [`TripleStatement`].
//! You can use the [`rule!`][rule-macro] macro to build rules more easily.
//!
//! [triple-pattern]: crate::Pattern
//! [rule-macro]: crate::rule!
//!
//! ## Example
//!
//! ```
//! // Citizenship implies humanship.
//! let rule = inferdf::rule! {
//!   for ?person, ?country {
//!     ?person <"https://example.org/#citizenOf"> ?country .
//!   } => {
//!     ?person <"http://www.w3.org/1999/02/22-rdf-syntax-ns#type"> <"https://example.org/#Human"> .
//!   }
//! };
//! ```
//!
//! Rules can be merged together into deduction [`System`]s.
//! Once a rule or system is defined, you can then use it to deduce new triples
//! from an input dataset (in the example above, deduce humanship from
//! citizenship) or validate the input dataset (check that citizenship implies
//! citizenship).
//!
//! ## Deduction
//!
//! Use the [`Rule::deduce`] or [`System::deduce`] methods to infer new triples
//! from a given dataset.
//!
//! ```
//! use rdf_types::{dataset::BTreeGraph, grdf_triples};
//! # let rule = inferdf::rule! {
//! #   for ?person, ?country {
//! #     ?person <"https://example.org/#citizenOf"> ?country .
//! #   } => {
//! #     ?person <"http://www.w3.org/1999/02/22-rdf-syntax-ns#type"> <"https://example.org/#Human"> .
//! #   }
//! # };
//!
//! // Build an RDF dataset (a single graph here).
//! let mut input: BTreeGraph = grdf_triples! [
//!   _:"FrançoisDupont" <"https://example.org/#citizenOf"> _:"France" .
//! ].into_iter().collect();
//!
//! // Deduce statements from the input dataset.
//! let deductions = rule.deduce(&input);
//!
//! // Evaluates the deductions. This may add new terms into the graph, so
//! // we need to provide a blank id generator to generate those terms.
//! let evaluated_deductions = deductions.eval(rdf_types::generator::Blank::new()).expect("evaluation failed");
//!
//! for deduction in evaluated_deductions {
//!   use inferdf::{Signed, Sign, TripleStatement};
//!   for statement in deduction.statements {
//!     if let Signed(Sign::Positive, TripleStatement::Triple(triple)) = statement {
//!       input.insert(triple); // insert the deduced triple into the graph.
//!     }
//!   }
//! }
//!
//! let mut expected: BTreeGraph = grdf_triples! [
//!   _:"FrançoisDupont" <"https://example.org/#citizenOf"> _:"France" .
//!   _:"FrançoisDupont" <"http://www.w3.org/1999/02/22-rdf-syntax-ns#type"> <"https://example.org/#Human"> .
//! ].into_iter().collect();
//!
//! assert_eq!(input, expected)
//! ```
//!
//! ## Validation
//!
//! Use the [`Rule::validate`]/[`System::validate`] to validate a given
//! dataset against a (set of) deduction rule(s). This will return a
//! [`Validation`] status value, either `Ok` or `Invalid`. The later also
//! provides a [`Reason`] why the validation failed.
//!
//! ```
//! use rdf_types::{dataset::BTreeGraph, grdf_triples};
//! # let rule = inferdf::rule! {
//! #   for ?person, ?country {
//! #     ?person <"https://example.org/#citizenOf"> ?country .
//! #   } => {
//! #     ?person <"http://www.w3.org/1999/02/22-rdf-syntax-ns#type"> <"https://example.org/#Human"> .
//! #   }
//! # };
//!
//! // Build an RDF dataset (a single graph here).
//! let input: BTreeGraph = grdf_triples! [
//!   _:"FrançoisDupont" <"https://example.org/#citizenOf"> _:"France" .
//!   _:"FrançoisDupont" <"http://www.w3.org/1999/02/22-rdf-syntax-ns#type"> <"https://example.org/#Human"> .
//! ].into_iter().collect();
//!
//! assert!(rule.validate(&input).unwrap().is_valid())
//! ```
use rdf_types::{Term, Triple};
use std::hash::Hash;

#[doc(hidden)]
pub use rdf_types;

#[doc(hidden)]
pub use static_iref;

mod sign;
pub use sign::*;

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

mod r#macros;
pub mod utils;

/// Signed triple.
pub type Fact<T> = Signed<Triple<T, T, T>>;

/// Signed triple reference.
pub type FactRef<'a, T> = Signed<Triple<&'a T, &'a T, &'a T>>;

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
