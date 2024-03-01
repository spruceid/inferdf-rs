use std::{borrow::Cow, cmp::Ordering};

use rdf_types::{interpretation::ReverseLiteralInterpretation, LexicalLiteralTypeRef, Vocabulary};
use replace_with::replace_with_or_abort_and_return;
use xsd_types::{ParseXsd, XSD_BOOLEAN, XSD_STRING};

use super::{regex, Error, Regex, Value};

/// Comparable value.
pub enum Comparable<'a, R> {
	Any(&'a R),
	Boolean(xsd_types::Boolean),
	Decimal(Cow<'a, xsd_types::Decimal>),
	String(&'a str),
	Regex(Cow<'a, Regex>),
}

impl<'a, R> Comparable<'a, R> {
	pub fn from_value<V, I>(
		vocabulary: &'a V,
		interpretation: &'a I,
		value: &'a Value<R>,
	) -> Result<Self, Error>
	where
		R: Clone,
		V: Vocabulary,
		I: ReverseLiteralInterpretation<Resource = R, Literal = V::Literal>,
	{
		match value {
			Value::Resource(r) => Self::from_resource(vocabulary, interpretation, r),
			Value::Boolean(b) => Ok(Self::Boolean(*b)),
			Value::Decimal(d) => Ok(Self::Decimal(Cow::Borrowed(d))),
			Value::String(s) => Ok(Self::String(s)),
			Value::Regex(e) => Ok(Self::Regex(Cow::Borrowed(e))),
		}
	}

	pub fn from_resource<V, I>(
		vocabulary: &'a V,
		interpretation: &'a I,
		resource: &'a R,
	) -> Result<Self, Error>
	where
		V: Vocabulary,
		I: ReverseLiteralInterpretation<Resource = R, Literal = V::Literal>,
	{
		let mut result = Self::Any(resource);

		for l in interpretation.literals_of(resource) {
			if let Some(l) = vocabulary.literal(l) {
				if let LexicalLiteralTypeRef::Any(iri) =
					l.type_.as_lexical_type_ref_with(vocabulary)
				{
					if iri == XSD_BOOLEAN {
						result.refine(Comparable::Boolean(xsd_types::Boolean::parse_xsd(
							&l.value,
						)?))?
					}

					if xsd_types::DecimalDatatype::from_iri(iri).is_some() {
						result.refine(Comparable::Decimal(Cow::Owned(
							xsd_types::Decimal::parse_xsd(&l.value)?,
						)))?
					}

					if iri == XSD_STRING {
						result.refine(Comparable::String(&l.value))?;
					}

					if iri == regex::TYPE_IRI {
						result.refine(Comparable::Regex(Cow::Owned(Regex::new(&l.value)?)))?
					}
				}
			}
		}

		Ok(result)
	}

	pub fn refine(&mut self, other: Self) -> Result<(), Error> {
		replace_with_or_abort_and_return(self, |this| match (this, other) {
			(Self::Any(_), b) => (Ok(()), b),
			(Self::Boolean(a), Self::Boolean(b)) if a == b => (Ok(()), Self::Boolean(b)),
			(Self::Decimal(a), Self::Decimal(b)) if a == b => (Ok(()), Self::Decimal(b)),
			(Self::String(a), Self::String(b)) if a == b => (Ok(()), Self::String(b)),
			(Self::Regex(a), Self::Regex(b)) if a == b => (Ok(()), Self::Regex(b)),
			(this, _) => (Err(Error::AmbiguousLiteral), this),
		})
	}
}

impl<'a, R: PartialEq> PartialEq for Comparable<'a, R> {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Self::Any(a), Self::Any(b)) => a == b,
			(Self::Boolean(a), Self::Boolean(b)) => a == b,
			(Self::Decimal(a), Self::Decimal(b)) => a == b,
			(Self::String(a), Self::String(b)) => a == b,
			_ => false,
		}
	}
}

impl<'a, R: PartialEq> PartialOrd for Comparable<'a, R> {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		match (self, other) {
			(Self::Any(a), Self::Any(b)) if a == b => Some(Ordering::Equal),
			(Self::Boolean(a), Self::Boolean(b)) if a == b => Some(Ordering::Equal),
			(Self::Decimal(a), Self::Decimal(b)) => a.partial_cmp(b),
			(Self::String(a), Self::String(b)) => a.partial_cmp(b),
			_ => None,
		}
	}
}
