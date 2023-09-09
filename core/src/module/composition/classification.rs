use std::marker::PhantomData;

use derivative::Derivative;
use group::GroupId;
use rdf_types::Vocabulary;

use crate::{Module, class::group, Class, Id};

use super::Composition;

#[derive(Derivative)]
#[derivative(Clone(bound=""))]
pub struct Classification<'a, V, M> {
	module: &'a Composition<V, M>,
	v: PhantomData<V>
}

impl<'a, V: Vocabulary, M: Module<V>> crate::Classification<'a> for Classification<'a, V, M> {
	type Error = M::Error;

	/// Find a group with the given layer and description, if any.
	fn find_group_id(
		&self,
		layer: u32,
		description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error> {
		todo!()
	}

	/// Returns the representative of the given class, if any.
	fn class_representative(&self, term: Class) -> Result<Option<Id>, Self::Error> {
		todo!()
	}

	fn resource_class(&self, id: Id) -> Result<Option<Class>, Self::Error> {
		todo!()
	}
}