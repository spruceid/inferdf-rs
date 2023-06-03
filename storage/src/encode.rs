use std::io::{self, Seek, Write};

use inferdf_core::{Cause, Id, Sign, Signed, Triple};
use iref::{Iri, IriBuf};
use langtag::LanguageTag;
use locspan::Meta;
use rdf_types::{literal, Literal};

use crate::{
	module::{IriPath, LiteralPath},
	Header, Tag, Version, HEADER_TAG, VERSION,
};

pub trait StaticEncodedLen {
	const ENCODED_LEN: u32;
}

impl StaticEncodedLen for u32 {
	const ENCODED_LEN: u32 = 4;
}

impl<T: StaticEncodedLen, M: StaticEncodedLen> StaticEncodedLen for Meta<T, M> {
	const ENCODED_LEN: u32 = T::ENCODED_LEN + M::ENCODED_LEN;
}

impl StaticEncodedLen for Sign {
	const ENCODED_LEN: u32 = 1;
}

impl<T: StaticEncodedLen> StaticEncodedLen for Signed<T> {
	const ENCODED_LEN: u32 = Sign::ENCODED_LEN + T::ENCODED_LEN;
}

impl StaticEncodedLen for Triple {
	const ENCODED_LEN: u32 = 3 * Id::ENCODED_LEN;
}

pub trait EncodedLen {
	fn encoded_len(&self) -> u32;
}

impl<T: StaticEncodedLen> EncodedLen for T {
	fn encoded_len(&self) -> u32 {
		T::ENCODED_LEN
	}
}

impl<T: StaticEncodedLen> EncodedLen for Vec<T> {
	fn encoded_len(&self) -> u32 {
		4 + self.len() as u32 * T::ENCODED_LEN
	}
}

impl<'a> EncodedLen for &'a [u8] {
	fn encoded_len(&self) -> u32 {
		4 + self.len() as u32
	}
}

impl<'a> EncodedLen for &'a str {
	fn encoded_len(&self) -> u32 {
		self.as_bytes().encoded_len()
	}
}

impl<'a> EncodedLen for Iri<'a> {
	fn encoded_len(&self) -> u32 {
		self.as_bytes().encoded_len()
	}
}

impl StaticEncodedLen for Id {
	const ENCODED_LEN: u32 = 4;
}

pub trait Encode {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error>;

	fn encode_page(
		&self,
		page_len: u32,
		output: &mut (impl Write + Seek),
	) -> Result<(), io::Error> {
		let len = self.encode(output)?;
		output.seek(io::SeekFrom::Current((page_len - len) as i64))?;
		Ok(())
	}
}

impl Encode for Header {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		self.tag.encode(output)?;
		self.version.encode(output)?;
		self.page_size.encode(output)?;
		self.resource_count.encode(output)?;
		self.resource_page_count.encode(output)?;
		self.iri_count.encode(output)?;
		self.iri_page_count.encode(output)?;
		self.literal_count.encode(output)?;
		self.literal_page_count.encode(output)?;
		self.named_graph_count.encode(output)?;
		self.named_graph_page_count.encode(output)?;
		self.default_graph.encode(output)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Encode for Tag {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		output.write_all(&HEADER_TAG)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Encode for Version {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		VERSION.encode(output)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Encode for u8 {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		output.write_all(std::slice::from_ref(self))?;
		Ok(1)
	}
}

impl Encode for u32 {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		let bytes = self.to_be_bytes();
		output.write_all(&bytes)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Encode for Id {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		let i: u32 = (*self).into();
		i.encode(output)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Encode for IriPath {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		Ok(self.page.encode(output)? + self.index.encode(output)?)
	}
}

impl StaticEncodedLen for IriPath {
	const ENCODED_LEN: u32 = u32::ENCODED_LEN + u32::ENCODED_LEN;
}

impl Encode for LiteralPath {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		Ok(self.page.encode(output)? + self.index.encode(output)?)
	}
}

impl StaticEncodedLen for LiteralPath {
	const ENCODED_LEN: u32 = u32::ENCODED_LEN + u32::ENCODED_LEN;
}

impl<'a> Encode for &'a [u8] {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		(self.len() as u32).encode(output)?;
		output.write_all(self)?;
		Ok(u32::ENCODED_LEN + self.len() as u32)
	}
}

impl<T: Encode> Encode for Vec<T> {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		let mut len = (self.len() as u32).encode(output)?;

		for entry in self {
			len += entry.encode(output)?
		}

		Ok(len)
	}
}

impl<T: Encode, M: Encode> Encode for Meta<T, M> {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		Ok(self.0.encode(output)? + self.1.encode(output)?)
	}
}

impl<T: Encode> Encode for Signed<T> {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		Ok(self.0.encode(output)? + self.1.encode(output)?)
	}
}

impl Encode for Sign {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		match self {
			Self::Positive => 0u8.encode(output),
			Self::Negative => 1u8.encode(output),
		}
	}
}

impl Encode for Cause {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		match self {
			Self::Stated(i) => Ok(0u8.encode(output)? + i.encode(output)?),
			Self::Entailed(i) => Ok(1u8.encode(output)? + i.encode(output)?),
		}
	}
}

impl<'a> Encode for &'a str {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		self.as_bytes().encode(output)
	}
}

impl Encode for String {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		self.as_str().encode(output)
	}
}

impl<'a> Encode for Iri<'a> {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		self.as_bytes().encode(output)
	}
}

impl Encode for IriBuf {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		self.as_iri().encode(output)
	}
}

impl<T: Encode, S: Encode> Encode for Literal<T, S> {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		Ok(self.type_().encode(output)? + self.value().encode(output)?)
	}
}

impl<T: EncodedLen, S: EncodedLen> EncodedLen for Literal<T, S> {
	fn encoded_len(&self) -> u32 {
		self.type_().encoded_len() + self.value().encoded_len()
	}
}

impl<I: Encode, L: Encode> Encode for literal::Type<I, L> {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		match self {
			Self::Any(ty) => Ok(0u8.encode(output)? + ty.encode(output)?),
			Self::LangString(tag) => Ok(1u8.encode(output)? + tag.encode(output)?),
		}
	}
}

impl<I: EncodedLen, L: EncodedLen> EncodedLen for literal::Type<I, L> {
	fn encoded_len(&self) -> u32 {
		match self {
			Self::Any(ty) => 0u32.encoded_len() + ty.encoded_len(),
			Self::LangString(tag) => 1u32.encoded_len() + tag.encoded_len(),
		}
	}
}

impl<'a> Encode for LanguageTag<'a> {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		self.as_bytes().encode(output)
	}
}

impl<'a> EncodedLen for LanguageTag<'a> {
	fn encoded_len(&self) -> u32 {
		self.as_bytes().encoded_len()
	}
}
