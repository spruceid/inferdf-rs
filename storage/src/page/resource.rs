use crate::{
	reader::{self, Decode},
	writer::Encode,
};

/// A Resource page.
///
/// Resources pages list resources and their occurences in the dataset.
pub struct ResourcePage(Vec<Entry>);

pub struct Entry {
	pub known_iris: Vec<u32>,
	pub known_literals: Vec<u32>,
	pub as_subject: Vec<u32>,
	pub as_predicate: Vec<u32>,
	pub as_object: Vec<u32>,
}

impl<V> Encode<V> for ResourcePage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for ResourcePage {
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
		self.known_literals.encode(vocabulary, output)?;
		self.as_subject.encode(vocabulary, output)?;
		self.as_predicate.encode(vocabulary, output)?;
		self.as_object.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for Entry {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self {
			known_iris: Vec::decode(vocabulary, input)?,
			known_literals: Vec::decode(vocabulary, input)?,
			as_subject: Vec::decode(vocabulary, input)?,
			as_predicate: Vec::decode(vocabulary, input)?,
			as_object: Vec::decode(vocabulary, input)?,
		})
	}
}
