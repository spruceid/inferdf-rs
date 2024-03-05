use core::fmt;
use iref::Iri;
use rdf_types::LexicalLiteralTypeRef;
use serde::{Deserialize, Serialize};
use static_iref::iri;
use std::hash::Hash;

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

impl PartialOrd for Regex {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for Regex {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.as_str().cmp(other.as_str())
	}
}

impl Hash for Regex {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.as_str().hash(state)
	}
}

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

impl Serialize for Regex {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.as_str().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Regex {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let pattern = String::deserialize(deserializer)?;
		Self::new(&pattern).map_err(serde::de::Error::custom)
	}
}
