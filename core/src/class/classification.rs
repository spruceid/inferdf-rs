use std::ops::Deref;

pub mod local;

pub use local::LocalClassification as Local;

use crate::{Class, Id, IteratorWith};

use super::{group, GroupId};

pub trait Classification<'a, V> {
	type Error;

	type Groups: IteratorWith<V, Item = Result<(GroupId, Self::DescriptionRef), Self::Error>>;

	type Classes: IteratorWith<V, Item = Result<(Class, Id), Self::Error>>;

	type DescriptionRef: Deref<Target = group::Description>;

	fn groups(&self) -> Self::Groups;

	fn group(&self, id: GroupId) -> Result<Option<Self::DescriptionRef>, Self::Error>;

	/// Find a group with the given layer and description, if any.
	fn find_group_id(
		&self,
		description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error>;

	fn classes(&self) -> Self::Classes;

	/// Returns the representative of the given class, if any.
	fn class_representative(&self, term: Class) -> Result<Option<Id>, Self::Error>;

	fn resource_class(&self, id: Id) -> Result<Option<Class>, Self::Error>;
}

impl<'a, V> Classification<'a, V> for () {
	type Error = std::convert::Infallible;

	type Groups = std::iter::Empty<Result<(GroupId, Self::DescriptionRef), Self::Error>>;

	type Classes = std::iter::Empty<Result<(Class, Id), Self::Error>>;

	type DescriptionRef = &'a group::Description;

	fn groups(&self) -> Self::Groups {
		std::iter::empty()
	}

	fn group(&self, _id: GroupId) -> Result<Option<Self::DescriptionRef>, Self::Error> {
		Ok(None)
	}

	fn find_group_id(
		&self,
		_description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error> {
		Ok(None)
	}

	fn classes(&self) -> Self::Classes {
		std::iter::empty()
	}

	fn class_representative(&self, _term: Class) -> Result<Option<Id>, Self::Error> {
		Ok(None)
	}

	fn resource_class(&self, _id: Id) -> Result<Option<Class>, Self::Error> {
		Ok(None)
	}
}
