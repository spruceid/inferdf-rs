use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use xsd_types::Decimal;

use super::{Regex, Value};

/// Literal value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Literal {
	/// Decimal value.
	Decimal(#[serde(with = "decimal")] Decimal),

	/// Text string.
	String(String),

	/// Regular expression.
	Regex(Regex),
}

impl Literal {
	/// Evaluates the literal expression.
	pub fn eval<R: Clone>(&self) -> Value<R> {
		match self {
			Self::Decimal(d) => Value::Decimal(Cow::Borrowed(d)),
			Self::String(s) => Value::String(Cow::Borrowed(s)),
			Self::Regex(e) => Value::Regex(Cow::Borrowed(e)),
		}
	}
}

impl<'a> From<&'a str> for Literal {
	fn from(value: &'a str) -> Self {
		Self::String(value.to_owned())
	}
}

macro_rules! literal_from_int {
	($($ty:ident),*) => {
		$(
			impl From<$ty> for Literal {
				fn from(value: $ty) -> Self {
					Self::Decimal(value.into())
				}
			}
		)*
	};
}

literal_from_int!(u8, u16, u32, u64, i8, i16, i32, i64);

mod decimal {
	use serde::{de, Deserializer, Serialize, Serializer};
	use xsd_types::{
		Decimal, DecimalDatatype, IntDatatype, IntegerDatatype, LongDatatype,
		NonNegativeIntegerDatatype, ShortDatatype, UnsignedIntDatatype, UnsignedLongDatatype,
		UnsignedShortDatatype,
	};

	pub fn serialize<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match value.decimal_type() {
			DecimalDatatype::Decimal => match value.as_f64() {
				Some(f) => f.serialize(serializer),
				None => Err(serde::ser::Error::custom(
					"decimal number out of float range",
				)),
			},
			DecimalDatatype::Integer(IntegerDatatype::Long(i)) => match i {
				LongDatatype::Long => {
					let v: i64 = value.try_into().unwrap();
					v.serialize(serializer)
				}
				LongDatatype::Int(IntDatatype::Int) => {
					let v: i32 = value.try_into().unwrap();
					v.serialize(serializer)
				}
				LongDatatype::Int(IntDatatype::Short(ShortDatatype::Short)) => {
					let v: i16 = value.try_into().unwrap();
					v.serialize(serializer)
				}
				LongDatatype::Int(IntDatatype::Short(ShortDatatype::Byte)) => {
					let v: i8 = value.try_into().unwrap();
					v.serialize(serializer)
				}
			},
			DecimalDatatype::Integer(IntegerDatatype::NonNegativeInteger(
				NonNegativeIntegerDatatype::UnsignedLong(u),
			)) => match u {
				UnsignedLongDatatype::UnsignedLong => {
					let v: u64 = value.try_into().unwrap();
					v.serialize(serializer)
				}
				UnsignedLongDatatype::UnsignedInt(UnsignedIntDatatype::UnsignedInt) => {
					let v: i32 = value.try_into().unwrap();
					v.serialize(serializer)
				}
				UnsignedLongDatatype::UnsignedInt(UnsignedIntDatatype::UnsignedShort(
					UnsignedShortDatatype::UnsignedShort,
				)) => {
					let v: i16 = value.try_into().unwrap();
					v.serialize(serializer)
				}
				UnsignedLongDatatype::UnsignedInt(UnsignedIntDatatype::UnsignedShort(
					UnsignedShortDatatype::UnsignedByte,
				)) => {
					let v: i8 = value.try_into().unwrap();
					v.serialize(serializer)
				}
			},
			_ => Err(serde::ser::Error::custom("integer number out of bounds")),
		}
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct Visitor;

		impl<'de> de::Visitor<'de> for Visitor {
			type Value = Decimal;

			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				write!(formatter, "a number")
			}

			fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}

			fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}

			fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}

			fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}

			fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}

			fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}

			fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}

			fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				Ok(v.into())
			}
		}

		deserializer.deserialize_any(Visitor)
	}
}
