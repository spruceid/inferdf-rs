use group::GroupId;
use hashbrown::HashMap;

use crate::{
	class::{group, Class},
	Id, IteratorWith,
};

pub struct LocalClassification {
	layers: Vec<Layer>,
	resource_classes: HashMap<Id, Class>,
	class_representatives: HashMap<Class, Id>,
}

impl LocalClassification {
	pub fn new(layers: Vec<Layer>, resource_classes: HashMap<Id, Class>) -> Self {
		let class_representatives = resource_classes.iter().map(|(id, c)| (*c, *id)).collect();

		Self {
			layers,
			resource_classes,
			class_representatives,
		}
	}
}

pub struct Layer {
	pub groups: Vec<group::Description>,
}

impl Layer {
	pub fn new(groups: Vec<group::Description>) -> Self {
		Self { groups }
	}
}

impl<'a, V> crate::Classification<'a, V> for &'a LocalClassification {
	type Error = std::convert::Infallible;

	type Groups = Groups<'a>;

	type Classes = Classes<'a>;

	type DescriptionRef = &'a group::Description;

	fn groups(&self) -> Self::Groups {
		Groups {
			layers: self.layers.iter().enumerate(),
			current: None,
		}
	}

	fn group(&self, id: GroupId) -> Result<Option<Self::DescriptionRef>, Self::Error> {
		match self.layers.get(id.layer as usize) {
			Some(layer) => Ok(layer.groups.get(id.index as usize)),
			None => Ok(None),
		}
	}

	/// Find a group with the given layer and description, if any.
	fn find_group_id(
		&self,
		description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error> {
		let l = description.layer();
		match self.layers.get(l as usize) {
			Some(layer) => Ok(layer
				.groups
				.iter()
				.position(|g| g == description)
				.map(|i| GroupId::new(l, i as u32))),
			None => Ok(None),
		}
	}

	fn classes(&self) -> Self::Classes {
		Classes(self.class_representatives.iter())
	}

	/// Returns the representative of the given class, if any.
	fn class_representative(&self, term: Class) -> Result<Option<Id>, Self::Error> {
		Ok(self.class_representatives.get(&term).copied())
	}

	fn resource_class(&self, id: Id) -> Result<Option<Class>, Self::Error> {
		Ok(self.resource_classes.get(&id).copied())
	}
}

pub struct Groups<'a> {
	layers: std::iter::Enumerate<std::slice::Iter<'a, Layer>>,
	current: Option<(
		u32,
		std::iter::Enumerate<std::slice::Iter<'a, group::Description>>,
	)>,
}

impl<'a> Iterator for Groups<'a> {
	type Item = (GroupId, &'a group::Description);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some((layer, groups)) => match groups.next() {
					Some((i, desc)) => break Some((GroupId::new(*layer, i as u32), desc)),
					None => self.current = None,
				},
				None => match self.layers.next() {
					Some((i, layer)) => {
						self.current = Some((i as u32, layer.groups.iter().enumerate()))
					}
					None => break None,
				},
			}
		}
	}
}

impl<'a, V> IteratorWith<V> for Groups<'a> {
	type Item = Result<(GroupId, &'a group::Description), std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next().map(Ok)
	}
}

pub struct Classes<'a>(hashbrown::hash_map::Iter<'a, Class, Id>);

impl<'a> Iterator for Classes<'a> {
	type Item = (Class, Id);

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(id, c)| (*id, *c))
	}
}

impl<'a, V> IteratorWith<V> for Classes<'a> {
	type Item = Result<(Class, Id), std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next().map(Ok)
	}
}
