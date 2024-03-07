//! Triple statement expressions.
use core::fmt;
use std::borrow::Cow;

use iref::IriBuf;
use rdf_types::{
	interpretation::ReverseTermInterpretation, vocabulary::EmbedIntoVocabulary, Interpretation,
	Term, Triple, Vocabulary,
};
use serde::{Deserialize, Serialize};
use xsd_types::ParseXsdError;

mod literal;
pub use literal::*;

pub mod value;
pub use value::{Regex, Value};

use value::Comparable;

use crate::{
	pattern::{ApplyPartialSubstitution, ApplySubstitution, PatternSubstitution, ResourceOrVar},
	Signed,
};

/// Expression that evaluate into a resource.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Expression<T, F = BuiltInFunction> {
	Resource(T),
	Literal(Literal),
	Call(F, Vec<Self>),
}

pub trait Eval<'e, V, I> {
	type Output;

	/// Evaluates.
	fn eval(&'e self, vocabulary: &V, interpretation: &I) -> Result<Self::Output, Error>;

	fn eval_and_instantiate(
		&'e self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<<Self::Output as Instantiate<V, I>>::Instantiated, Error>
	where
		Self::Output: Instantiate<V, I>,
	{
		let value = self.eval(vocabulary, interpretation)?;
		Ok(value.instantiate(vocabulary, interpretation))
	}
}

pub trait Instantiate<V, I> {
	type Instantiated;

	fn instantiate(self, vocabulary: &mut V, interpretation: &mut I) -> Self::Instantiated;
}

impl<'e, T: 'e, F, V, I> Eval<'e, V, I> for Expression<T, F>
where
	T: Clone,
	F: Function<V, I>,
	I: Interpretation<Resource = T>,
{
	type Output = Value<'e, T>;

	/// Evaluates the expression.
	fn eval(&'e self, vocabulary: &V, interpretation: &I) -> Result<Value<T>, Error> {
		match self {
			Self::Resource(r) => Ok(Value::Resource(Cow::Borrowed(r))),
			Self::Literal(l) => Ok(l.eval()),
			Self::Call(f, args) => {
				let mut args_values = Vec::with_capacity(args.len());

				for a in args {
					args_values.push(a.eval(vocabulary, interpretation)?)
				}

				f.call(vocabulary, interpretation, &args_values)
			}
		}
	}
}

impl<'e, V, I, T: Eval<'e, V, I>> Eval<'e, V, I> for Triple<T> {
	type Output = Triple<T::Output>;

	fn eval(&'e self, vocabulary: &V, interpretation: &I) -> Result<Self::Output, Error> {
		Ok(Triple(
			self.0.eval(vocabulary, interpretation)?,
			self.1.eval(vocabulary, interpretation)?,
			self.2.eval(vocabulary, interpretation)?,
		))
	}
}

impl<V, I, T: Instantiate<V, I>> Instantiate<V, I> for Triple<T> {
	type Instantiated = Triple<T::Instantiated>;

	fn instantiate(self, vocabulary: &mut V, interpretation: &mut I) -> Self::Instantiated {
		Triple(
			self.0.instantiate(vocabulary, interpretation),
			self.1.instantiate(vocabulary, interpretation),
			self.2.instantiate(vocabulary, interpretation),
		)
	}
}

impl<'e, V, I, T: Eval<'e, V, I>> Eval<'e, V, I> for Signed<T> {
	type Output = Signed<T::Output>;

	fn eval(&'e self, vocabulary: &V, interpretation: &I) -> Result<Self::Output, Error> {
		Ok(Signed(self.0, self.1.eval(vocabulary, interpretation)?))
	}
}

impl<V, I, T: Instantiate<V, I>> Instantiate<V, I> for Signed<T> {
	type Instantiated = Signed<T::Instantiated>;

	fn instantiate(self, vocabulary: &mut V, interpretation: &mut I) -> Self::Instantiated {
		Signed(self.0, self.1.instantiate(vocabulary, interpretation))
	}
}

impl<V: Vocabulary, T: EmbedIntoVocabulary<V>, F> EmbedIntoVocabulary<V> for Expression<T, F> {
	type Embedded = Expression<T::Embedded, F>;

	fn embed_into_vocabulary(self, vocabulary: &mut V) -> Self::Embedded {
		match self {
			Self::Resource(t) => Expression::Resource(t.embed_into_vocabulary(vocabulary)),
			Self::Literal(l) => Expression::Literal(l),
			Self::Call(f, args) => Expression::Call(
				f,
				args.into_iter()
					.map(|a| a.embed_into_vocabulary(vocabulary))
					.collect(),
			),
		}
	}
}

impl<R, T: ApplyPartialSubstitution<R>, F: Clone> ApplyPartialSubstitution<R> for Expression<T, F> {
	fn apply_partial_substitution(&self, substitution: &PatternSubstitution<R>) -> Self {
		match self {
			Self::Resource(r) => Expression::Resource(r.apply_partial_substitution(substitution)),
			Self::Literal(l) => Expression::Literal(l.clone()),
			Self::Call(f, args) => Expression::Call(
				f.clone(),
				args.iter()
					.map(|a| a.apply_partial_substitution(substitution))
					.collect(),
			),
		}
	}
}

impl<R, T: ApplySubstitution<R>, F: Clone> ApplySubstitution<R> for Expression<T, F> {
	type Output = Expression<T::Output, F>;

	fn apply_substitution(&self, substitution: &PatternSubstitution<R>) -> Option<Self::Output> {
		match self {
			Self::Resource(r) => Some(Expression::Resource(r.apply_substitution(substitution)?)),
			Self::Literal(l) => Some(Expression::Literal(l.clone())),
			Self::Call(f, args) => Some(Expression::Call(
				f.clone(),
				args.iter()
					.map(|a| a.apply_substitution(substitution))
					.collect::<Option<Vec<_>>>()?,
			)),
		}
	}
}

impl<T, F> Expression<ResourceOrVar<T>, F> {
	pub fn visit_variables(&self, mut f: impl FnMut(usize)) {
		self.visit_variables_ref_mut(&mut f)
	}

	fn visit_variables_ref_mut(&self, f: &mut impl FnMut(usize)) {
		match self {
			Self::Resource(ResourceOrVar::Var(x)) => f(*x),
			Self::Resource(ResourceOrVar::Resource(_)) => (),
			Self::Literal(_) => (),
			Self::Call(_, args) => {
				for a in args {
					a.visit_variables_ref_mut(&mut *f)
				}
			}
		}
	}
}

/// Function.
pub trait Function<V, I: Interpretation> {
	/// Calls the function with the given arguments.
	fn call(
		&self,
		vocabulary: &V,
		interpretation: &I,
		args: &[Value<I::Resource>],
	) -> Result<Value<I::Resource>, Error>
	where
		I::Resource: Clone;
}

/// Built-in functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BuiltInFunction {
	/// Boolean or.
	Or,

	/// Boolean and.
	And,

	/// Comparison.
	Compare(ComparisonOperator),

	/// Regular expression matching.
	Matches,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid number of arguments (expected {required}, found {found})")]
	InvalidArgumentCount { required: usize, found: usize },

	#[error("invalid literal value")]
	InvalidLiteral,

	#[error("ambiguous literal value")]
	AmbiguousLiteral,

	#[error("expected {0}, found {1}")]
	Unexpected(Expected, UnexpectedTerm),
}

impl<L, V> From<ParseXsdError<L, V>> for Error {
	fn from(_value: ParseXsdError<L, V>) -> Self {
		Self::InvalidLiteral
	}
}

impl From<regex::Error> for Error {
	fn from(_value: regex::Error) -> Self {
		Self::InvalidLiteral
	}
}

impl<V, I> Function<V, I> for BuiltInFunction
where
	V: Vocabulary,
	V::Iri: PartialEq,
	I: ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
	I::Resource: PartialEq,
{
	fn call(
		&self,
		vocabulary: &V,
		interpretation: &I,
		args: &[Value<I::Resource>],
	) -> Result<Value<I::Resource>, Error>
	where
		I::Resource: Clone,
	{
		match self {
			Self::Or => {
				for a in args {
					if a.require_boolean(vocabulary, interpretation)?.0 {
						return Ok(Value::Boolean(xsd_types::Boolean(true)));
					}
				}

				Ok(Value::Boolean(xsd_types::Boolean(false)))
			}
			Self::And => {
				for a in args {
					if !a.require_boolean(vocabulary, interpretation)?.0 {
						return Ok(Value::Boolean(xsd_types::Boolean(false)));
					}
				}

				Ok(Value::Boolean(xsd_types::Boolean(true)))
			}
			Self::Compare(op) => {
				let mut prev = None;

				for a in args {
					let a = Comparable::from_value(vocabulary, interpretation, a)?;
					if let Some(p) = &prev {
						if !op.eval(p, &a) {
							return Ok(Value::Boolean(xsd_types::Boolean(false)));
						}
					}

					prev = Some(a)
				}

				Ok(Value::Boolean(xsd_types::Boolean(true)))
			}
			Self::Matches => match args {
				[regex, haystack] => {
					let regex = regex.require_regex(vocabulary, interpretation)?;
					let haystack = haystack.require_any_literal(vocabulary, interpretation)?;
					Ok(Value::Boolean(xsd_types::Boolean(regex.is_match(haystack))))
				}
				_ => Err(Error::InvalidArgumentCount {
					required: 2,
					found: args.len(),
				}),
			},
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ComparisonOperator {
	/// Equality.
	Eq,

	/// Inequality.
	Ne,

	/// Less than.
	Lt,

	/// Less or equal.
	Leq,

	/// Greater than.
	Gt,

	/// Greater or equal.
	Geq,
}

impl ComparisonOperator {
	fn eval<R: PartialEq>(&self, a: &Comparable<R>, b: &Comparable<R>) -> bool {
		eprintln!("eval op: {:?} {self:?} {:?}", a.as_opaque(), b.as_opaque());
		match self {
			Self::Eq => a == b,
			Self::Ne => a != b,
			Self::Lt => a < b,
			Self::Leq => a <= b,
			Self::Gt => a > b,
			Self::Geq => a >= b,
		}
	}
}

#[derive(Debug)]
pub enum Expected {
	AnyLiteral,
	Literal(IriBuf),
}

impl fmt::Display for Expected {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::AnyLiteral => write!(f, "literal"),
			Self::Literal(type_) => write!(f, "literal of type <{type_}>"),
		}
	}
}

#[derive(Debug)]
pub enum UnexpectedTerm {
	Anonymous,
	Term(Term),
}

impl fmt::Display for UnexpectedTerm {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Anonymous => write!(f, "anonymous resource"),
			Self::Term(t) => write!(f, "term {t}"),
		}
	}
}

fn as_unexpected<V, I>(vocabulary: &V, interpretation: &I, resource: &I::Resource) -> UnexpectedTerm
where
	V: Vocabulary,
	I: ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
{
	if let Some(i) = interpretation.iris_of(resource).next() {
		return UnexpectedTerm::Term(Term::iri(vocabulary.iri(i).unwrap().to_owned()));
	}

	if let Some(b) = interpretation.blank_ids_of(resource).next() {
		return UnexpectedTerm::Term(Term::blank(vocabulary.blank_id(b).unwrap().to_owned()));
	}

	UnexpectedTerm::Anonymous
}
