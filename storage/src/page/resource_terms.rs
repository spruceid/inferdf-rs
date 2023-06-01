use std::cmp::Ordering;

use inferdf_core::Id;

use crate::{
	module::{self, Decode, IriPath, LiteralPath},
	writer::Encode,
};

/// A Resource terms page.
///
/// Resources pages list resources and their known terms.
pub struct ResourcesTermsPage(Vec<Entry>);

impl ResourcesTermsPage {
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
	pub known_iris: Vec<IriPath>,
	pub known_literals: Vec<LiteralPath>,
	pub different_from: Vec<Id>,
}

impl Entry {
	pub fn iter_known_iris(&self) -> IriPaths {
		self.known_iris.iter().copied()
	}

	pub fn iter_known_literals(&self) -> LiteralPaths {
		self.known_literals.iter().copied()
	}

	pub fn iter_different_from(&self) -> DifferentFrom {
		self.different_from.iter().copied()
	}
}

pub type IriPaths<'a> = std::iter::Copied<std::slice::Iter<'a, IriPath>>;

pub type LiteralPaths<'a> = std::iter::Copied<std::slice::Iter<'a, LiteralPath>>;

pub type DifferentFrom<'a> = std::iter::Copied<std::slice::Iter<'a, Id>>;

impl<V> Encode<V> for ResourcesTermsPage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl Decode for ResourcesTermsPage {
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
		self.known_iris.encode(vocabulary, output)?;
		self.known_literals.encode(vocabulary, output)?;
		self.different_from.encode(vocabulary, output)
	}
}

impl Decode for Entry {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, module::decode::Error> {
		Ok(Self {
			id: Id::decode(input)?,
			known_iris: Vec::decode(input)?,
			known_literals: Vec::decode(input)?,
			different_from: Vec::decode(input)?,
		})
	}
}
