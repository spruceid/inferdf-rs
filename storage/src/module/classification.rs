use std::io;

use inferdf_core::{
	class::{group, GroupId},
	Class, Id,
};
use paged::no_context_mut;
use rdf_types::Vocabulary;

use super::{Error, Module};

pub struct Classification<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
}

impl<'a, V: Vocabulary, R> Classification<'a, V, R> {
	pub(crate) fn new(
		module: &'a Module<V, R>,
	) -> Self {
		Self {
			module
		}
	}
}

impl<'a, V: Vocabulary, R: io::Seek + io::Read> inferdf_core::Classification<'a>
	for Classification<'a, V, R>
{
	type Error = Error;

	/// Find a group with the given layer and description, if any.
	fn find_group_id(
		&self,
		layer: u32,
		description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error> {
		Ok(self
			.module
			.reader
			.binary_search_by_key(
				self.module.header.classification.groups,
				&self.module.cache.classification_groups,
				no_context_mut(),
				self.module.header.heap,
				|g, _| {
					g.layer
						.cmp(&layer)
						.then_with(|| g.description.cmp(&description))
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
			.map(|r| r.class))
	}
}
