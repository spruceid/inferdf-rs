use core::fmt;
use iref::Iri;
use rdf_types::LexicalLiteralTypeRef;
use static_iref::iri;

pub use regex::Error;

use super::{LiteralValue, ToLiteralValue};

/// Regex datatype IRI.
pub const TYPE_IRI: &Iri = iri!("https://schema.spruceid.com/#Regex");

#[derive(Debug, Clone)]
pub struct Regex(regex::Regex);

impl Regex {
	pub fn new(pattern: &str) -> Result<Self, regex::Error> {
		regex::Regex::new(pattern).map(Self)
	}

	pub fn as_str(&self) -> &str {
		self.0.as_str()
	}

	pub fn is_match(&self, haystack: &str) -> bool {
		self.0.is_match(haystack)
	}
}

impl fmt::Display for Regex {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.as_str().fmt(f)
	}
}

impl PartialEq for Regex {
	fn eq(&self, other: &Self) -> bool {
		self.0.as_str() == other.0.as_str()
	}
}

impl Eq for Regex {}

impl LiteralValue for Regex {
	const TYPE: &'static Iri = TYPE_IRI;

	fn parse_literal(
		value: &str,
		type_: LexicalLiteralTypeRef,
	) -> Result<Option<Self>, super::Error> {
		match type_ {
			LexicalLiteralTypeRef::Any(iri) if iri == TYPE_IRI => Ok(Some(
				Self::new(value).map_err(|_| super::Error::InvalidLiteral)?,
			)),
			_ => Ok(None),
		}
	}
}

impl ToLiteralValue for Regex {
	fn preferred_type(&self) -> &Iri {
		Self::TYPE
	}
}
