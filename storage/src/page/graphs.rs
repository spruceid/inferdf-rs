use std::cmp::Ordering;

use inferdf_core::Id;

use crate::{
	module::{self, Decode, DecodeSized},
	writer::Encode,
};

pub struct GraphsPage(Vec<Entry>);

impl GraphsPage {
	pub fn get(&self, i: usize) -> Option<&Entry> {
		self.0.get(i)
	}

	pub fn find(&self, id: Id) -> Result<usize, Ordering> {
		if self.0.is_empty() {
			Err(Ordering::Equal)
		} else if self.0[0].id > id {
			Err(Ordering::Greater)
		} else if self.0[self.0.len() - 1].id < id {
			Err(Ordering::Less)
		} else {
			match self.0.binary_search_by_key(&id, |e| e.id) {
				Ok(i) => Ok(i),
				Err(_) => Err(Ordering::Equal),
			}
		}
	}

	pub fn iter(&self) -> Iter {
		self.0.iter().copied()
	}
}

pub type Iter<'a> = std::iter::Copied<std::slice::Iter<'a, Entry>>;

#[derive(Debug, Clone, Copy)]
pub struct Description {
	pub triple_count: u32,
	pub triple_page_count: u32,
	pub resource_count: u32,
	pub resource_page_count: u32,
	pub first_page: u32,
}

impl Description {
	pub const LEN: usize = 4 * 5;
}

#[derive(Debug, Clone, Copy)]
pub struct Entry {
	pub id: Id,
	pub description: Description,
}

impl<V> Encode<V> for GraphsPage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V> DecodeSized<V> for GraphsPage {
	fn decode_sized(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
		len: u32,
	) -> Result<Self, module::decode::Error> {
		let mut graphs = Vec::new();

		for _i in 0..len {
			graphs.push(Entry::decode(input)?)
		}

		Ok(Self(graphs))
	}
}

impl<V> Encode<V> for Description {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.triple_count.encode(vocabulary, output)?;
		self.triple_page_count.encode(vocabulary, output)?;
		self.resource_count.encode(vocabulary, output)?;
		self.resource_page_count.encode(vocabulary, output)?;
		self.first_page.encode(vocabulary, output)
	}
}

impl Decode for Description {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, module::decode::Error> {
		Ok(Self {
			triple_count: u32::decode(input)?,
			triple_page_count: u32::decode(input)?,
			resource_count: u32::decode(input)?,
			resource_page_count: u32::decode(input)?,
			first_page: u32::decode(input)?,
		})
	}
}

impl<V> Encode<V> for Entry {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.id.encode(vocabulary, output)?;
		self.description.encode(vocabulary, output)
	}
}

impl Decode for Entry {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, module::decode::Error> {
		Ok(Self {
			id: Id::decode(input)?,
			description: Description::decode(input)?,
		})
	}
}
