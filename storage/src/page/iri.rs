use inferdf_core::Id;
use iref::IriBuf;

use crate::{
	reader::{self, Decode},
	writer::Encode,
};

pub struct IriPage<I = IriBuf>(Vec<Entry<I>>);

pub struct Entry<I> {
	iri: I,
	interpretations: Vec<Id>,
}

impl<V, I: Encode<V>> Encode<V> for IriPage<I> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V, I: Decode<V>> Decode<V> for IriPage<I> {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self(Vec::decode(vocabulary, input)?))
	}
}

impl<V, I: Encode<V>> Encode<V> for Entry<I> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.iri.encode(vocabulary, output)?;
		self.interpretations.encode(vocabulary, output)
	}
}

impl<V, I: Decode<V>> Decode<V> for Entry<I> {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self {
			iri: I::decode(vocabulary, input)?,
			interpretations: Vec::decode(vocabulary, input)?,
		})
	}
}
