use std::cmp::Ordering;

use inferdf_core::Id;

use crate::{
	module::{self, Decode},
	writer::Encode,
};

/// A Resource triples page.
///
/// Resources pages list resources of a graph and the triples they occur in.
pub struct ResourcesTriplesPage(Vec<Entry>);

impl ResourcesTriplesPage {
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
}

pub struct Entry {
	pub id: Id,
	pub as_subject: Vec<u32>,
	pub as_predicate: Vec<u32>,
	pub as_object: Vec<u32>,
}

impl<V> Encode<V> for ResourcesTriplesPage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl Decode for ResourcesTriplesPage {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, module::decode::Error> {
		Ok(Self(Vec::decode(input)?))
	}
}

impl<V> Encode<V> for Entry {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.id.encode(vocabulary, output)?;
		self.as_subject.encode(vocabulary, output)?;
		self.as_predicate.encode(vocabulary, output)?;
		self.as_object.encode(vocabulary, output)
	}
}

impl Decode for Entry {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, module::decode::Error> {
		Ok(Self {
			id: Id::decode(input)?,
			as_subject: Vec::decode(input)?,
			as_predicate: Vec::decode(input)?,
			as_object: Vec::decode(input)?,
		})
	}
}
