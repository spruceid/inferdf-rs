use normal_form::Normalize;
use paged::{utils::Inline, Paged};

use crate::Signed;

use super::Reference;

pub mod normalization;

pub use normalization::NonNormalizedDescription;

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

pub enum MembersSubstitution {
	NonReflexive,
	Reflexive(Vec<usize>),
}

impl MembersSubstitution {
	pub fn get(&self, i: u32) -> Option<u32> {
		match self {
			Self::NonReflexive => {
				if i == 0 {
					Some(0)
				} else {
					None
				}
			}
			Self::Reflexive(s) => s.get(i as usize).map(|j| *j as u32),
		}
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
	pub fn new(members: Vec<Member>) -> (Self, MembersSubstitution) {
		if members.len() == 1 {
			(Self { members }, MembersSubstitution::NonReflexive)
		} else {
			let desc = normalization::NonNormalizedDescription::new(members);
			let (normal_form, substitution) = desc.normalize();
			(normal_form, MembersSubstitution::Reflexive(substitution))
		}
	}

	pub fn non_reflexive(member: Member) -> Self {
		Self::from_normalized_members(vec![member])
	}

	pub fn from_normalized_members(members: Vec<Member>) -> Self {
		Self { members }
	}

	pub fn layer(&self) -> u32 {
		let mut layer = 0;

		for m in &self.members {
			layer = std::cmp::max(m.layer(), layer)
		}

		layer
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
			properties: Inline(properties),
		}
	}

	pub fn len(&self) -> usize {
		self.properties.len()
	}

	pub fn is_empty(&self) -> bool {
		self.properties.is_empty()
	}

	pub fn layer(&self) -> u32 {
		let mut layer = 0;

		for Signed(_, (a, b)) in &self.properties.0 {
			layer = std::cmp::max(std::cmp::max(a.layer(), b.layer()), layer)
		}

		layer
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
