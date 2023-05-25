use std::io::Read;

use inferdf_core::Id;
use iref::IriBuf;
use langtag::LanguageTagBuf;
use rdf_types::{literal, Literal};

use crate::{Header, Tag, Version, HEADER_TAG, VERSION};

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
}

pub trait DecodeSized<V>: Sized {
	fn decode_sized(vocabulary: &mut V, input: &mut impl Read, len: u32) -> Result<Self, Error>;
}

pub trait Decode<V>: Sized {
	fn decode(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error>;
}

impl<V> Decode<V> for Header {
	fn decode(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		Ok(Self {
			tag: Tag::decode(vocabulary, input)?,
			version: Version::decode(vocabulary, input)?,
			page_size: u32::decode(vocabulary, input)?,
			resource_count: u32::decode(vocabulary, input)?,
			resource_page_count: u32::decode(vocabulary, input)?,
			iri_count: u32::decode(vocabulary, input)?,
			iri_page_count: u32::decode(vocabulary, input)?,
			literal_count: u32::decode(vocabulary, input)?,
			literal_page_count: u32::decode(vocabulary, input)?,
			graph_count: u32::decode(vocabulary, input)?,
			graph_page_count: u32::decode(vocabulary, input)?,
			default_graph_triple_count: u32::decode(vocabulary, input)?,
			default_graph_triple_page_count: u32::decode(vocabulary, input)?,
		})
	}
}

impl<V> Decode<V> for Tag {
	fn decode(_vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let mut buf = [0u8; 4];
		input.read_exact(&mut buf)?;
		if buf == HEADER_TAG {
			Ok(Self)
		} else {
			Err(Error::InvalidTag)
		}
	}
}

impl<V> Decode<V> for Version {
	fn decode(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let v = u32::decode(vocabulary, input)?;
		if v == VERSION {
			Ok(Self)
		} else {
			Err(Error::UnsupportedVersion(v))
		}
	}
}

impl<V> Decode<V> for u32 {
	fn decode(_vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let mut buf = [0u8; 4];
		input.read_exact(&mut buf)?;
		Ok(u32::from_be_bytes(buf))
	}
}

pub fn decode_bytes(input: &mut impl Read) -> Result<Vec<u8>, Error> {
	let len = u32::decode(&mut (), input)? as usize;
	let mut buffer = Vec::with_capacity(len);

	unsafe {
		// SAFETY: `buffer` has been created with a capacity of `len`.
		input.read_exact(std::slice::from_raw_parts_mut(buffer.as_mut_ptr(), len))?;
		buffer.set_len(len);
	}

	Ok(buffer)
}

impl<V> Decode<V> for Id {
	fn decode(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		u32::decode(vocabulary, input).map(Into::into)
	}
}

impl<V, T: Decode<V>> Decode<V> for Vec<T> {
	fn decode(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let len = u32::decode(vocabulary, input)?;
		let mut entries = Vec::with_capacity(len as usize);

		for _i in 0..len {
			entries.push(T::decode(vocabulary, input)?)
		}

		Ok(entries)
	}
}

impl<V> Decode<V> for String {
	fn decode(_vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let bytes = decode_bytes(input)?;
		String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)
	}
}

impl<'a, V> Decode<V> for IriBuf {
	fn decode(_vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let bytes = decode_bytes(input)?;
		IriBuf::from_vec(bytes).map_err(|(e, _)| Error::InvalidIri(e))
	}
}

impl<'a, V> Decode<V> for LanguageTagBuf {
	fn decode(_vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let bytes = decode_bytes(input)?;
		LanguageTagBuf::new(bytes).map_err(|(e, _)| Error::InvalidLanguageTag(e))
	}
}

impl<V, I: Decode<V>, L: Decode<V>> Decode<V> for literal::Type<I, L> {
	fn decode(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let mut d = 0;
		input.read_exact(std::slice::from_mut(&mut d))?;
		match d {
			0 => Ok(Self::Any(I::decode(vocabulary, input)?)),
			1 => Ok(Self::LangString(L::decode(vocabulary, input)?)),
			_ => Err(Error::InvalidLiteralType),
		}
	}
}

impl<V, T: Decode<V>, S: Decode<V>> Decode<V> for Literal<T, S> {
	fn decode(vocabulary: &mut V, input: &mut impl Read) -> Result<Self, Error> {
		let type_ = T::decode(vocabulary, input)?;
		let value = S::decode(vocabulary, input)?;
		Ok(Self::new(value, type_))
	}
}
