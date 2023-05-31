use crate::{
	reader::{self, Decode},
	writer::Encode,
};

/// A Resource terms page.
///
/// Resources pages list resources and their known terms.
pub struct ResourceTermsPage(Vec<Entry>);

pub struct Entry {
	pub known_iris: Vec<u32>,
	pub known_literals: Vec<u32>
}

impl<V> Encode<V> for ResourceTermsPage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for ResourceTermsPage {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self(Vec::decode(vocabulary, input)?))
	}
}

impl<V> Encode<V> for Entry {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.known_iris.encode(vocabulary, output)?;
		self.known_literals.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for Entry {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self {
			known_iris: Vec::decode(vocabulary, input)?,
			known_literals: Vec::decode(vocabulary, input)?
		})
	}
}
