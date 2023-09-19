use appendlist::AppendList;
use derivative::Derivative;
use group::GroupId;
use memo_map::MemoMap;
use rdf_types::Vocabulary;

use crate::{
	class::group, module::sub_module::GlobalClassification, Class, Id, IteratorWith, Module,
};

use super::{Composition, CompositionSubModule};

#[derive(Default)]
pub(crate) struct CompositionGlobalClassification {
	layers: AppendList<Layer>,
	map: MemoMap<group::Description, GroupId>,
	resource_classes: MemoMap<Id, Class>,
	class_representatives: MemoMap<Class, Id>,
}

impl CompositionGlobalClassification {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn resource_class(&self, global_id: Id) -> Option<Class> {
		self.resource_classes.get(&global_id).copied()
	}

	pub fn class_representative(&self, class: Class) -> Option<Id> {
		self.class_representatives.get(&class).copied()
	}

	pub fn set_class_representative(&self, class: Class, id: Id) -> bool {
		self.resource_classes.insert(id, class) && self.class_representatives.insert(class, id)
	}

	pub fn get_or_insert_class_representative(
		&self,
		class: Class,
		new_resource: impl FnOnce() -> Id,
	) -> Id {
		*self.class_representatives.get_or_insert(&class, || {
			let id = new_resource();
			self.resource_classes.insert(id, class);
			id
		})
	}

	fn get_or_insert_layer(&self, i: u32) -> &Layer {
		let i = i as usize;
		while i < self.layers.len() {
			self.layers.push(Layer::default())
		}

		self.layers.get(i).unwrap()
	}
}

impl GlobalClassification for CompositionGlobalClassification {
	fn group(&self, id: GroupId) -> Option<&group::Description> {
		let layer = self.layers.get(id.layer as usize)?;
		layer.get(id.index)
	}

	fn insert_group(&self, desc: &group::Description) -> GroupId {
		*self.map.get_or_insert(desc, || {
			let layer = desc.layer();
			let index = self.get_or_insert_layer(layer).insert(desc.clone());
			GroupId::new(layer, index)
		})
	}
}

#[derive(Default)]
struct Layer {
	groups: AppendList<group::Description>,
}

impl Layer {
	fn get(&self, i: u32) -> Option<&group::Description> {
		self.groups.get(i as usize)
	}

	fn insert(&self, desc: group::Description) -> u32 {
		let i = self.groups.len() as u32;
		self.groups.push(desc);
		i
	}
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Classification<'a, V, M> {
	composition: &'a Composition<V, M>,
}

impl<'a, V, M> Classification<'a, V, M> {
	pub(crate) fn new(composition: &'a Composition<V, M>) -> Self {
		Self { composition }
	}
}

impl<'a, V: Vocabulary, M: Module<V>> crate::Classification<'a, V> for Classification<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Error = M::Error;

	type Groups = Groups<'a, V, M>;

	type Classes = Classes<'a, V, M>;

	type DescriptionRef = &'a group::Description;

	fn groups(&self) -> Self::Groups {
		Groups {
			composition: self.composition,
			sub_modules: self.composition.modules.iter(),
			current: None,
		}
	}

	fn group(&self, global_id: GroupId) -> Result<Option<Self::DescriptionRef>, Self::Error> {
		Ok(self.composition.global_classification.group(global_id))
	}

	/// Find a group with the given layer and description, if any.
	fn find_group_id(
		&self,
		global_description: &group::Description,
	) -> Result<Option<GroupId>, Self::Error> {
		for m in &self.composition.modules {
			let local_desc = m.classification_interface().local_group_description(
				&self.composition.global_classification,
				m.as_sub_module(),
				global_description,
			)?;

			if local_desc.is_some() {
				return Ok(Some(
					self.composition
						.global_classification
						.insert_group(global_description),
				));
			}
		}

		Ok(None)
	}

	fn classes(&self) -> Self::Classes {
		Classes {
			composition: self.composition,
			sub_modules: self.composition.modules.iter(),
			current: None,
		}
	}

	/// Returns the representative of the given class, if any.
	fn class_representative(&self, global_class: Class) -> Result<Option<Id>, Self::Error> {
		Ok(self
			.composition
			.global_classification
			.class_representative(global_class))
	}

	fn resource_class(&self, global_id: Id) -> Result<Option<Class>, Self::Error> {
		Ok(self
			.composition
			.global_classification
			.resource_class(global_id))
	}
}

pub struct Groups<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	sub_modules: std::slice::Iter<'a, CompositionSubModule<V, M>>,
	current: Option<SubModuleGroups<'a, V, M>>,
}

struct SubModuleGroups<'a, V: Vocabulary, M: Module<V>> {
	sub_module: &'a CompositionSubModule<V, M>,
	groups: <M::Classification<'a> as crate::Classification<'a, V>>::Groups,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Groups<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Item = Result<(GroupId, &'a group::Description), M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		use crate::Classification;
		loop {
			match &mut self.current {
				Some(current) => match current.groups.next_with(vocabulary) {
					Some(Ok((local_id, _local_desc))) => {
						match current.sub_module.classification_interface().global_group(
							&self.composition.global_classification,
							current.sub_module.as_sub_module(),
							local_id,
							|id| {
								self.composition
									.import_resource(vocabulary, current.sub_module, id)
							},
						) {
							Ok((global_id, _)) => {
								let global_desc = self
									.composition
									.global_classification
									.group(global_id)
									.unwrap();
								break Some(Ok((global_id, global_desc)));
							}
							Err(e) => break Some(Err(e)),
						}
					}
					Some(Err(e)) => break Some(Err(e)),
					None => self.current = None,
				},
				None => match self.sub_modules.next() {
					Some(sub_module) => {
						self.current = Some(SubModuleGroups {
							sub_module,
							groups: sub_module.module().classification().groups(),
						})
					}
					None => break None,
				},
			}
		}
	}
}

pub struct Classes<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	sub_modules: std::slice::Iter<'a, CompositionSubModule<V, M>>,
	current: Option<SubModuleClasses<'a, V, M>>,
}

struct SubModuleClasses<'a, V: Vocabulary, M: Module<V>> {
	sub_module: &'a CompositionSubModule<V, M>,
	classes: <M::Classification<'a> as crate::Classification<'a, V>>::Classes,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Classes<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Item = Result<(Class, Id), M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		use crate::Classification;
		loop {
			match &mut self.current {
				Some(current) => match current.classes.next_with(vocabulary) {
					Some(Ok((local_class, local_id))) => {
						match current.sub_module.classification_interface().global_class(
							&self.composition.global_classification,
							current.sub_module.as_sub_module(),
							local_class,
							|id| {
								self.composition
									.import_resource(vocabulary, current.sub_module, id)
							},
						) {
							Ok(global_class) => {
								match self.composition.import_resource(
									vocabulary,
									current.sub_module,
									local_id,
								) {
									Ok(global_id) => break Some(Ok((global_class, global_id))),
									Err(e) => break Some(Err(e)),
								}
							}
							Err(e) => break Some(Err(e)),
						}
					}
					Some(Err(e)) => break Some(Err(e)),
					None => self.current = None,
				},
				None => match self.sub_modules.next() {
					Some(sub_module) => {
						self.current = Some(SubModuleClasses {
							sub_module,
							classes: sub_module.module().classification().classes(),
						})
					}
					None => break None,
				},
			}
		}
	}
}
