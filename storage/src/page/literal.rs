use inferdf_core::Id;
use rdf_types::Literal;

use crate::{
	reader::{self, Decode},
	writer::Encode,
};

pub struct LiteralPage<L = Literal>(Vec<Entry<L>>);

pub struct Entry<L> {
	literal: L,
	interpretations: Vec<Id>,
}

impl<V, L: Encode<V>> Encode<V> for LiteralPage<L> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V, L: Decode<V>> Decode<V> for LiteralPage<L> {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self(Vec::decode(vocabulary, input)?))
	}
}

impl<V, L: Encode<V>> Encode<V> for Entry<L> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.literal.encode(vocabulary, output)?;
		self.interpretations.encode(vocabulary, output)
	}
}

impl<V, L: Decode<V>> Decode<V> for Entry<L> {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self {
			literal: L::decode(vocabulary, input)?,
			interpretations: Vec::decode(vocabulary, input)?,
		})
	}
}
