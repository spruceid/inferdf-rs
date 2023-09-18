use std::io;

use inferdf_core::{
	class::{group, GroupId},
	Class, Id, IteratorWith,
};
use paged::{no_context_mut, UnboundRef};
use rdf_types::Vocabulary;

use crate::header::{self, GetDescriptionBinder};

use super::{Error, Module};

pub struct Classification<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
}

impl<'a, V: Vocabulary, R> Classification<'a, V, R> {
	pub(crate) fn new(module: &'a Module<V, R>) -> Self {
		Self { module }
	}
}

pub type DescriptionRef<'a> = paged::Ref<'a, header::GroupById, UnboundRef<group::Description>>;

impl<'a, V: Vocabulary, R: io::Seek + io::Read> inferdf_core::Classification<'a, V>
	for Classification<'a, V, R>
{
	type Error = Error;

	type Classes = Classes<'a, R>;
	type Groups = Groups<'a, R>;

	type DescriptionRef = DescriptionRef<'a>;

	fn classes(&self) -> Self::Classes {
		Classes {
			inner: self
				.module
				.reader
				.iter(
					self.module.header.classification.representatives,
					&self.module.cache.classification_representatives,
					self.module.header.heap
				)
		}
	}

	fn groups(&self) -> Self::Groups {
		Groups {
			inner: self
				.module
				.reader
				.iter(
					self.module.header.classification.groups_by_id,
					&self.module.cache.classification_groups_by_id,
					self.module.header.heap
				)
		}
	}

	fn group(&self, id: GroupId) -> Result<Option<Self::DescriptionRef>, Self::Error> {
		Ok(self.module.reader.binary_search_by_key(
			self.module.header.classification.groups_by_id,
			&self.module.cache.classification_groups_by_id,
			no_context_mut(),
			self.module.header.heap,
			|g, _| {
				g.id.cmp(&id)
			}
		)?.map(|g| g.map(header::GetDescriptionBinder)))
	}

	/// Find a group with the given layer and description, if any.
	fn find_group_id(
		&self,
		description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error> {
		let layer = description.layer();
		Ok(self
			.module
			.reader
			.binary_search_by_key(
				self.module.header.classification.groups_by_desc,
				&self.module.cache.classification_groups_by_desc,
				no_context_mut(),
				self.module.header.heap,
				|g, _| {
					g.layer
						.cmp(&layer)
						.then_with(|| g.description.cmp(description))
				},
			)?
			.map(|g| GroupId::new(g.layer, g.index)))
	}

	/// Returns the representative of the given class, if any.
	fn class_representative(&self, term: Class) -> Result<Option<Id>, Self::Error> {
		Ok(self
			.module
			.reader
			.binary_search_by_key(
				self.module.header.classification.representatives,
				&self.module.cache.classification_representatives,
				no_context_mut(),
				self.module.header.heap,
				|r, _| r.class.cmp(&term),
			)?
			.map(|r| r.resource))
	}

	fn resource_class(&self, id: Id) -> Result<Option<Class>, Self::Error> {
		Ok(self
			.module
			.reader
			.binary_search_by_key(
				self.module.header.interpretation.resources,
				&self.module.cache.interpretation_resources,
				no_context_mut(),
				self.module.header.heap,
				|r, _| r.id.cmp(&id),
			)?
			.and_then(|r| r.class))
	}
}

pub struct Classes<'a, R> {
	inner: paged::Iter<'a, 'a, R, header::Representative>
}

impl<'a, R: io::Seek + io::Read> Iterator for Classes<'a, R> {
	type Item = Result<(Class, Id), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(|r| r.map(|r| (r.class, r.resource)))
	}
}

impl<'a, V, R: io::Seek + io::Read> IteratorWith<V> for Classes<'a, R> {
	type Item = Result<(Class, Id), Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct Groups<'a, R> {
	inner: paged::Iter<'a, 'a, R, header::GroupById>
}

impl<'a, R: io::Seek + io::Read> Iterator for Groups<'a, R> {
	type Item = Result<(GroupId, DescriptionRef<'a>), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(|r| r.map(|r| (r.id, r.map(GetDescriptionBinder))))
	}
}

impl<'a, V, R: io::Seek + io::Read> IteratorWith<V> for Groups<'a, R> {
	type Item = Result<(GroupId, DescriptionRef<'a>), Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}