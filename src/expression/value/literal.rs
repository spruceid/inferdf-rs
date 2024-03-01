use iref::Iri;
use rdf_types::{
	interpretation::{LiteralInterpretationMut, ReverseTermInterpretation},
	LexicalLiteralTypeRef, LiteralType, Vocabulary, VocabularyMut,
};
use xsd_types::{ParseXsd, XSD_BOOLEAN, XSD_DECIMAL, XSD_STRING};

use crate::expression::{as_unexpected, Expected};

use super::Error;

pub trait ToLiteralValue: ToString {
	fn preferred_type(&self) -> &Iri;

	fn to_resource<V, I>(&self, vocabulary: &mut V, interpretation: &mut I) -> I::Resource
	where
		V: VocabularyMut,
		I: LiteralInterpretationMut<V::Literal>,
	{
		let type_ = LiteralType::Any(vocabulary.insert(self.preferred_type()));
		let literal = rdf_types::Literal::new(self.to_string(), type_);
		let l = vocabulary.insert_owned_literal(literal);
		interpretation.interpret_literal(l)
	}
}

pub trait LiteralValue: Sized + PartialEq {
	const TYPE: &'static Iri;

	fn parse_literal(value: &str, type_: LexicalLiteralTypeRef) -> Result<Option<Self>, Error>;

	fn from_resource<'a, V, I>(
		vocabulary: &'a V,
		interpretation: &'a I,
		resource: &'a I::Resource,
	) -> Result<Self, Error>
	where
		V: Vocabulary,
		V::Iri: PartialEq,
		I: ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
	{
		let mut value: Option<Self> = None;

		for l in interpretation.literals_of(resource) {
			if let Some(literal) = vocabulary.literal(l) {
				let type_ = literal.type_.as_lexical_type_ref_with(vocabulary);
				if let Some(v) = Self::parse_literal(&literal.value, type_)? {
					if let Some(other) = value.replace(v) {
						if other != *value.as_ref().unwrap() {
							return Err(Error::AmbiguousLiteral);
						}
					}
				}
			}
		}

		match value {
			Some(value) => Ok(value),
			None => Err(Error::Unexpected(
				Expected::Literal(Self::TYPE.to_owned()),
				as_unexpected(vocabulary, interpretation, resource),
			)),
		}
	}
}

impl LiteralValue for xsd_types::Boolean {
	const TYPE: &'static Iri = XSD_BOOLEAN;

	fn parse_literal(value: &str, type_: LexicalLiteralTypeRef) -> Result<Option<Self>, Error> {
		match type_ {
			LexicalLiteralTypeRef::Any(iri) if iri == XSD_BOOLEAN => {
				Ok(Some(xsd_types::Boolean::parse_xsd(value)?))
			}
			_ => Ok(None),
		}
	}
}

impl ToLiteralValue for xsd_types::Boolean {
	fn preferred_type(&self) -> &Iri {
		Self::TYPE
	}
}

impl LiteralValue for xsd_types::Decimal {
	const TYPE: &'static Iri = XSD_DECIMAL;

	fn parse_literal(value: &str, type_: LexicalLiteralTypeRef) -> Result<Option<Self>, Error> {
		match type_ {
			LexicalLiteralTypeRef::Any(iri) => {
				if xsd_types::DecimalDatatype::from_iri(iri).is_some() {
					Ok(Some(xsd_types::Decimal::parse_xsd(value)?))
				} else {
					Ok(None)
				}
			}
			_ => Ok(None),
		}
	}
}

impl ToLiteralValue for xsd_types::Decimal {
	fn preferred_type(&self) -> &Iri {
		Self::TYPE
	}
}

impl LiteralValue for String {
	const TYPE: &'static Iri = XSD_STRING;

	fn parse_literal(value: &str, type_: LexicalLiteralTypeRef) -> Result<Option<Self>, Error> {
		match type_ {
			LexicalLiteralTypeRef::Any(iri) if iri == XSD_STRING => Ok(Some(value.to_owned())),
			_ => Ok(None),
		}
	}
}

impl ToLiteralValue for str {
	fn preferred_type(&self) -> &Iri {
		XSD_STRING
	}
}
