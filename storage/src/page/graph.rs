use inferdf_core::Id;

use crate::{
	reader::{self, Decode, DecodeSized},
	writer::Encode,
};

pub struct GraphPage(Vec<Entry>);

pub struct Entry {
	id: Id,
	triple_count: u32,
	triple_page_count: u32,
	first_page: u32,
}

impl<V> Encode<V> for GraphPage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V> DecodeSized<V> for GraphPage {
	fn decode_sized(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
		len: u32,
	) -> Result<Self, reader::Error> {
		let mut graphs = Vec::new();

		for _i in 0..len {
			graphs.push(Entry::decode(vocabulary, input)?)
		}

		Ok(Self(graphs))
	}
}

impl<V> Encode<V> for Entry {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.id.encode(vocabulary, output)?;
		self.triple_count.encode(vocabulary, output)?;
		self.triple_page_count.encode(vocabulary, output)?;
		self.first_page.encode(vocabulary, output)
	}
}

impl<V> Decode<V> for Entry {
	fn decode(vocabulary: &mut V, input: &mut impl std::io::Read) -> Result<Self, reader::Error> {
		Ok(Self {
			id: Id::decode(vocabulary, input)?,
			triple_count: u32::decode(vocabulary, input)?,
			triple_page_count: u32::decode(vocabulary, input)?,
			first_page: u32::decode(vocabulary, input)?,
		})
	}
}
