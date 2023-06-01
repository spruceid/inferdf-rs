use std::io::Read;

use inferdf_core::{Cause, Id, Sign, Signed};
use iref::IriBuf;
use langtag::LanguageTagBuf;
use locspan::Meta;
use rdf_types::{literal, Literal};

use crate::{page, Header, Tag, Version, HEADER_TAG, VERSION};

use super::{IriPath, LiteralPath};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	IO(#[from] std::io::Error),

	#[error("invalid tag")]
	InvalidTag,

	#[error("unsupported version {0}")]
	UnsupportedVersion(u32),

	#[error("invalid IRI: {0}")]
	InvalidIri(iref::Error),

	#[error("invalid language tag: {0}")]
	InvalidLanguageTag(langtag::Error),

	#[error("invalid literal type")]
	InvalidLiteralType,

	#[error("invalid UTF-8 string")]
	InvalidUtf8,

	#[error("invalid triple sign")]
	InvalidSign,

	#[error("invalid triple cause")]
	InvalidCause,
}

pub trait DecodeSized<V>: Sized {
	fn decode_sized(vocabulary: &mut V, input: &mut impl Read, len: u32) -> Result<Self, Error>;
}

pub trait Decode: Sized {
	fn decode(input: &mut impl Read) -> Result<Self, Error>;
}

pub trait DecodeWith<V>: Sized {
	fn decode_with(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error>;
}

impl Decode for Header {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		Ok(Self {
			tag: Tag::decode(input)?,
			version: Version::decode(input)?,
			page_size: u32::decode(input)?,
			resource_count: u32::decode(input)?,
			resource_page_count: u32::decode(input)?,
			iri_count: u32::decode(input)?,
			iri_page_count: u32::decode(input)?,
			literal_count: u32::decode(input)?,
			literal_page_count: u32::decode(input)?,
			graph_count: u32::decode(input)?,
			graph_page_count: u32::decode(input)?,
			default_graph: page::graphs::Description::decode(input)?,
		})
	}
}

impl Decode for Tag {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let mut buf = [0u8; 4];
		input.read_exact(&mut buf)?;
		if buf == HEADER_TAG {
			Ok(Self)
		} else {
			Err(Error::InvalidTag)
		}
	}
}

impl Decode for Version {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let v = u32::decode(input)?;
		if v == VERSION {
			Ok(Self)
		} else {
			Err(Error::UnsupportedVersion(v))
		}
	}
}

impl Decode for u8 {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let mut buf = [0u8; 1];
		input.read_exact(&mut buf)?;
		Ok(buf[0])
	}
}

impl Decode for u32 {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let mut buf = [0u8; 4];
		input.read_exact(&mut buf)?;
		Ok(u32::from_be_bytes(buf))
	}
}

pub fn decode_bytes(input: &mut impl Read) -> Result<Vec<u8>, Error> {
	let len = u32::decode(input)? as usize;
	let mut buffer = Vec::with_capacity(len);

	unsafe {
		// SAFETY: `buffer` has been created with a capacity of `len`.
		input.read_exact(std::slice::from_raw_parts_mut(buffer.as_mut_ptr(), len))?;
		buffer.set_len(len);
	}

	Ok(buffer)
}

impl Decode for Id {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		u32::decode(input).map(Into::into)
	}
}

impl<T: Decode> Decode for Vec<T> {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let len = u32::decode(input)?;
		let mut entries = Vec::with_capacity(len as usize);

		for _i in 0..len {
			entries.push(T::decode(input)?)
		}

		Ok(entries)
	}
}

impl<V, T: DecodeWith<V>> DecodeWith<V> for Vec<T> {
	fn decode_with(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let len = u32::decode(input)?;
		let mut entries = Vec::with_capacity(len as usize);

		for _i in 0..len {
			entries.push(T::decode_with(vocabulary, input)?)
		}

		Ok(entries)
	}
}

impl<T: Decode, M: Decode> Decode for Meta<T, M> {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let t = T::decode(input)?;
		let m = M::decode(input)?;
		Ok(Meta(t, m))
	}
}

impl<T: Decode> Decode for Signed<T> {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let sign = Sign::decode(input)?;
		let t = T::decode(input)?;
		Ok(Signed(sign, t))
	}
}

impl Decode for Sign {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let d = u8::decode(input)?;
		match d {
			0 => Ok(Self::Positive),
			1 => Ok(Self::Negative),
			_ => Err(Error::InvalidSign),
		}
	}
}

impl Decode for Cause {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let d = u8::decode(input)?;
		let i = u32::decode(input)?;
		match d {
			0 => Ok(Self::Stated(i)),
			1 => Ok(Self::Entailed(i)),
			_ => Err(Error::InvalidCause),
		}
	}
}

impl Decode for IriPath {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		Ok(Self {
			page: u32::decode(input)?,
			index: u32::decode(input)?
		})
	}
}

impl Decode for LiteralPath {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		Ok(Self {
			page: u32::decode(input)?,
			index: u32::decode(input)?
		})
	}
}

impl Decode for String {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let bytes = decode_bytes(input)?;
		String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)
	}
}

impl Decode for IriBuf {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let bytes = decode_bytes(input)?;
		IriBuf::from_vec(bytes).map_err(|(e, _)| Error::InvalidIri(e))
	}
}

impl Decode for LanguageTagBuf {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let bytes = decode_bytes(input)?;
		LanguageTagBuf::new(bytes).map_err(|(e, _)| Error::InvalidLanguageTag(e))
	}
}

impl<I: Decode, L: Decode> Decode for literal::Type<I, L> {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let mut d = 0;
		input.read_exact(std::slice::from_mut(&mut d))?;
		match d {
			0 => Ok(Self::Any(I::decode(input)?)),
			1 => Ok(Self::LangString(L::decode(input)?)),
			_ => Err(Error::InvalidLiteralType),
		}
	}
}

impl<T: Decode, S: Decode> Decode for Literal<T, S> {
	fn decode(input: &mut impl Read) -> Result<Self, Error> {
		let type_ = T::decode(input)?;
		let value = S::decode(input)?;
		Ok(Self::new(value, type_))
	}
}
