use crate::{
	reader::{self, Decode},
	writer::Encode,
};

/// A Resource triples page.
///
/// Resources pages list resources of a graph and the triples they occur in.
pub struct ResourceTriplesPage(Vec<Entry>);

pub struct Entry {
	pub as_subject: Vec<u32>,
	pub as_predicate: Vec<u32>,
	pub as_object: Vec<u32>,
}

impl<V> Encode<V> for ResourceTriplesPage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for ResourceTriplesPage {
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
		self.as_subject.encode(vocabulary, output)?;
		self.as_predicate.encode(vocabulary, output)?;
		self.as_object.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for Entry {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self {
			as_subject: Vec::decode(vocabulary, input)?,
			as_predicate: Vec::decode(vocabulary, input)?,
			as_object: Vec::decode(vocabulary, input)?,
		})
	}
}
