use core::fmt;
use std::borrow::Cow;

use iref::IriBuf;
use rdf_types::{interpretation::ReverseTermInterpretation, Interpretation, Term, Vocabulary};
use xsd_types::ParseXsdError;

pub mod value;
pub use value::Value;

use value::{Comparable, Regex};

/// Expression that evaluate into a resource.
pub enum Expression<T, F = BuiltInFunction> {
	Resource(T),
	Literal(Literal),
	Call(F, Vec<Self>),
}

impl<T, F> Expression<T, F> {
	/// Evaluates the expression.
	pub fn eval<V, I>(&self, vocabulary: &V, interpretation: &I) -> Result<Value<T>, Error>
	where
		T: Clone,
		F: Function<V, I>,
		I: Interpretation<Resource = T>,
	{
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

/// Literal value.
pub enum Literal {
	/// Text string.
	String(String),

	/// Regular expression.
	Regex(Regex),
}

impl Literal {
	/// Evaluates the literal expression.
	pub fn eval<R: Clone>(&self) -> Value<R> {
		match self {
			Self::String(s) => Value::String(Cow::Borrowed(s)),
			Self::Regex(e) => Value::Regex(Cow::Borrowed(e)),
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
					let haystack = haystack.require_string(vocabulary, interpretation)?;
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
	Literal(IriBuf),
}

impl fmt::Display for Expected {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
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
