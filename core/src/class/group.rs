use paged::{Paged, utils::Inline};

use crate::Signed;

use super::Reference;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Paged)]
pub struct GroupId {
	pub layer: u32,
	pub index: u32,
}

impl GroupId {
	pub fn new(layer: u32, index: u32) -> Self {
		Self { layer, index }
	}
}

/// Resource group.
///
/// A group is composed of mutually recursive resources.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Paged)]
#[paged(heap)]
pub struct Description {
	/// Group members.
	pub members: Vec<Member>,
}

impl Description {
	pub fn new(members: Vec<Member>) -> Self {
		Self { members }
	}
}

/// Group members.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Paged)]
#[paged(unsized)]
pub struct Member {
	/// Resource properties.
	pub properties: Inline<Vec<Signed<(Reference, Reference)>>>,
}

impl Member {
	pub fn new(properties: Vec<Signed<(Reference, Reference)>>) -> Self {
		Self {
			properties: Inline(properties)
		}
	}

	pub fn len(&self) -> usize {
		self.properties.len()
	}

	pub fn is_empty(&self) -> bool {
		self.properties.is_empty()
	}

	pub fn add(&mut self, binding: Signed<(Reference, Reference)>) {
		self.properties.push(binding)
	}

	pub fn iter(&self) -> std::slice::Iter<Signed<(Reference, Reference)>> {
		self.properties.iter()
	}

	pub fn iter_mut(&mut self) -> std::slice::IterMut<Signed<(Reference, Reference)>> {
		self.properties.iter_mut()
	}
}

impl<'a> IntoIterator for &'a Member {
	type IntoIter = std::slice::Iter<'a, Signed<(Reference, Reference)>>;
	type Item = &'a Signed<(Reference, Reference)>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'a> IntoIterator for &'a mut Member {
	type IntoIter = std::slice::IterMut<'a, Signed<(Reference, Reference)>>;
	type Item = &'a mut Signed<(Reference, Reference)>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}
