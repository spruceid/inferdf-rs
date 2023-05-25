use inferdf_core::Triple;

use crate::{
	reader::{self, Decode, DecodeSized},
	writer::Encode,
};

pub struct TriplePage(Vec<Triple>);

impl<V> Encode<V> for TriplePage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V> DecodeSized<V> for TriplePage {
	fn decode_sized(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
		len: u32,
	) -> Result<Self, reader::Error> {
		let mut triples = Vec::with_capacity(len as usize);

		for _i in 0..len {
			triples.push(Triple::decode(vocabulary, input)?)
		}

		Ok(Self(triples))
	}
}

impl<V> Encode<V> for Triple {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)?;
		self.1.encode(vocabulary, output)?;
		self.2.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for Triple {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		let s = u32::decode(vocabulary, input)?;
		let p = u32::decode(vocabulary, input)?;
		let o = u32::decode(vocabulary, input)?;
		Ok(Self::new(s.into(), p.into(), o.into()))
	}
}
