use std::io::{self, Read, Write};

use crate::{decode, encode::StaticEncodedLen, page, Decode, Encode, HEADER_TAG, VERSION};

pub struct Header {
	pub tag: Tag,
	pub version: Version,
	pub page_size: u32,
	pub iri_count: u32,
	pub iri_page_count: u32,
	pub literal_count: u32,
	pub literal_page_count: u32,
	pub resource_count: u32,
	pub resource_page_count: u32,
	pub named_graph_count: u32,
	pub named_graph_page_count: u32,
	pub default_graph: page::graphs::Description,
}

impl StaticEncodedLen for Header {
	const ENCODED_LEN: u32 =
		Tag::ENCODED_LEN + Version::ENCODED_LEN + 4 * 9 + page::graphs::Description::ENCODED_LEN;
}

/// Header tag, used to recognize the file format.
pub struct Tag;

impl StaticEncodedLen for Tag {
	const ENCODED_LEN: u32 = 4;
}

/// Version number.
pub struct Version;

impl StaticEncodedLen for Version {
	const ENCODED_LEN: u32 = 4;
}

impl Encode for Header {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		self.tag.encode(output)?;
		self.version.encode(output)?;
		self.page_size.encode(output)?;
		self.iri_count.encode(output)?;
		self.iri_page_count.encode(output)?;
		self.literal_count.encode(output)?;
		self.literal_page_count.encode(output)?;
		self.resource_count.encode(output)?;
		self.resource_page_count.encode(output)?;
		self.named_graph_count.encode(output)?;
		self.named_graph_page_count.encode(output)?;
		self.default_graph.encode(output)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Decode for Header {
	fn decode(input: &mut impl Read) -> Result<Self, decode::Error> {
		Ok(Self {
			tag: Tag::decode(input)?,
			version: Version::decode(input)?,
			page_size: u32::decode(input)?,
			iri_count: u32::decode(input)?,
			iri_page_count: u32::decode(input)?,
			literal_count: u32::decode(input)?,
			literal_page_count: u32::decode(input)?,
			resource_count: u32::decode(input)?,
			resource_page_count: u32::decode(input)?,
			named_graph_count: u32::decode(input)?,
			named_graph_page_count: u32::decode(input)?,
			default_graph: page::graphs::Description::decode(input)?,
		})
	}
}

impl Encode for Tag {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		output.write_all(&HEADER_TAG)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Decode for Tag {
	fn decode(input: &mut impl Read) -> Result<Self, decode::Error> {
		let mut buf = [0u8; 4];
		input.read_exact(&mut buf)?;
		if buf == HEADER_TAG {
			Ok(Self)
		} else {
			Err(decode::Error::InvalidTag)
		}
	}
}

impl Encode for Version {
	fn encode(&self, output: &mut impl Write) -> Result<u32, io::Error> {
		VERSION.encode(output)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Decode for Version {
	fn decode(input: &mut impl Read) -> Result<Self, decode::Error> {
		let v = u32::decode(input)?;
		if v == VERSION {
			Ok(Self)
		} else {
			Err(decode::Error::UnsupportedVersion(v))
		}
	}
}
