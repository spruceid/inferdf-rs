pub mod local;

pub use local::Classification as Local;

use crate::{Class, Id};

use super::{group, GroupId};

pub trait Classification<'a> {
	type Error;

	/// Find a group with the given layer and description, if any.
	fn find_group_id(
		&self,
		layer: u32,
		description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error>;

	/// Returns the representative of the given class, if any.
	fn class_representative(&self, term: Class) -> Result<Option<Id>, Self::Error>;

	fn resource_class(&self, id: Id) -> Result<Option<Class>, Self::Error>;
}