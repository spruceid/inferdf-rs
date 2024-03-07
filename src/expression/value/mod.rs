use std::borrow::Cow;

use rdf_types::{
	interpretation::{LiteralInterpretationMut, ReverseTermInterpretation},
	Interpretation, LiteralType, Term, Vocabulary, VocabularyMut,
};
use xsd_types::{XSD_BOOLEAN, XSD_DECIMAL, XSD_STRING};

use super::{as_unexpected, Error, Expected, Instantiate, UnexpectedTerm};

pub mod regex;
pub use regex::Regex;

mod literal;
pub use literal::*;

mod comparable;
pub use comparable::*;

/// Value.
pub enum Value<'e, R: Clone> {
	/// Any resource.
	Resource(Cow<'e, R>),

	/// Boolean value.
	Boolean(xsd_types::Boolean),

	/// Decimal value.
	Decimal(Cow<'e, xsd_types::Decimal>),

	/// Text string.
	String(Cow<'e, str>),

	/// Regular expression.
	Regex(Cow<'e, Regex>),
}

impl<'e, R: Clone> Value<'e, R> {
	pub fn require_boolean<V, I>(
		&self,
		vocabulary: &V,
		interpretation: &I,
	) -> Result<xsd_types::Boolean, Error>
	where
		V: Vocabulary,
		V::Iri: PartialEq,
		I: ReverseTermInterpretation<
			Resource = R,
			Iri = V::Iri,
			BlankId = V::BlankId,
			Literal = V::Literal,
		>,
	{
		match self {
			Self::Resource(resource) => {
				xsd_types::Boolean::from_resource(vocabulary, interpretation, resource)
			}
			Self::Boolean(b) => Ok(*b),
			Self::Decimal(value) => Err(Error::Unexpected(
				Expected::Literal(XSD_BOOLEAN.to_owned()),
				UnexpectedTerm::Term(Term::Literal(rdf_types::Literal::new(
					value.to_string(),
					LiteralType::Any(XSD_DECIMAL.to_owned()),
				))),
			)),
			Self::String(value) => Err(Error::Unexpected(
				Expected::Literal(XSD_BOOLEAN.to_owned()),
				UnexpectedTerm::Term(Term::Literal(rdf_types::Literal::new(
					value.as_ref().to_owned(),
					LiteralType::Any(XSD_STRING.to_owned()),
				))),
			)),
			Self::Regex(value) => Err(Error::Unexpected(
				Expected::Literal(XSD_BOOLEAN.to_owned()),
				UnexpectedTerm::Term(Term::Literal(rdf_types::Literal::new(
					value.as_str().to_owned(),
					LiteralType::Any(regex::TYPE_IRI.to_owned()),
				))),
			)),
		}
	}

	pub fn require_any_literal<'a, V, I>(
		&'a self,
		vocabulary: &'a V,
		interpretation: &'a I,
	) -> Result<&str, Error>
	where
		V: Vocabulary,
		V::Iri: PartialEq,
		I: ReverseTermInterpretation<
			Resource = R,
			Iri = V::Iri,
			BlankId = V::BlankId,
			Literal = V::Literal,
		>,
	{
		match self {
			Self::Resource(resource) => {
				let mut value: Option<&'a str> = None;

				for l in interpretation.literals_of(resource) {
					if let Some(literal) = vocabulary.literal(l) {
						if let Some(other) = value.replace(&literal.value) {
							if other != *value.as_ref().unwrap() {
								return Err(Error::AmbiguousLiteral);
							}
						}
					}
				}

				match value {
					Some(value) => Ok(value),
					None => Err(Error::Unexpected(
						Expected::AnyLiteral,
						as_unexpected(vocabulary, interpretation, resource),
					)),
				}
			}
			Self::Boolean(xsd_types::Boolean(true)) => Ok("true"),
			Self::Boolean(xsd_types::Boolean(false)) => Ok("false"),
			Self::Decimal(value) => Ok(value.lexical_representation().as_str()),
			Self::String(s) => Ok(s),
			Self::Regex(value) => Ok(value.as_str()),
		}
	}

	pub fn require_regex<'a, V, I>(
		&'a self,
		vocabulary: &'a V,
		interpretation: &'a I,
	) -> Result<Cow<Regex>, Error>
	where
		V: Vocabulary,
		V::Iri: PartialEq,
		I: ReverseTermInterpretation<
			Resource = R,
			Iri = V::Iri,
			BlankId = V::BlankId,
			Literal = V::Literal,
		>,
	{
		match self {
			Self::Resource(resource) => {
				Regex::from_resource(vocabulary, interpretation, resource).map(Cow::Owned)
			}
			Self::Boolean(value) => Err(Error::Unexpected(
				Expected::Literal(regex::TYPE_IRI.to_owned()),
				UnexpectedTerm::Term(Term::Literal(rdf_types::Literal::new(
					value.to_string(),
					LiteralType::Any(XSD_BOOLEAN.to_owned()),
				))),
			)),
			Self::Decimal(value) => Err(Error::Unexpected(
				Expected::Literal(regex::TYPE_IRI.to_owned()),
				UnexpectedTerm::Term(Term::Literal(rdf_types::Literal::new(
					value.to_string(),
					LiteralType::Any(XSD_DECIMAL.to_owned()),
				))),
			)),
			Self::String(value) => Err(Error::Unexpected(
				Expected::Literal(regex::TYPE_IRI.to_owned()),
				UnexpectedTerm::Term(Term::Literal(rdf_types::Literal::new(
					value.as_ref().to_owned(),
					LiteralType::Any(XSD_STRING.to_owned()),
				))),
			)),
			Self::Regex(e) => Ok(Cow::Borrowed(e)),
		}
	}

	pub fn into_resource<V, I>(self, vocabulary: &mut V, interpretation: &mut I) -> R
	where
		R: Clone,
		V: VocabularyMut,
		I: Interpretation<Resource = R> + LiteralInterpretationMut<V::Literal>,
	{
		match self {
			Self::Resource(r) => r.into_owned(),
			Self::Boolean(b) => b.to_resource(vocabulary, interpretation),
			Self::Decimal(d) => d.to_resource(vocabulary, interpretation),
			Self::String(s) => s.to_resource(vocabulary, interpretation),
			Self::Regex(e) => e.to_resource(vocabulary, interpretation),
		}
	}
}

impl<'e, V, I, R> Instantiate<V, I> for Value<'e, R>
where
	R: Clone,
	V: VocabularyMut,
	I: Interpretation<Resource = R> + LiteralInterpretationMut<V::Literal>,
{
	type Instantiated = R;

	fn instantiate(self, vocabulary: &mut V, interpretation: &mut I) -> Self::Instantiated {
		self.into_resource(vocabulary, interpretation)
	}
}
