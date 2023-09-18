use std::{cell::RefCell, marker::PhantomData};

use hashbrown::HashMap;
use memo_map::MemoMap;
use rdf_types::Vocabulary;

use crate::{
	dataset::{self, Contradiction, TripleId},
	pattern, Dataset, Fact, FailibleIteratorWith, Id, Module, Sign, Signed, Triple, Class, class::{self, GroupId},
};

mod into_global;
mod into_local;
mod try_into_global;

pub use into_global::IntoGlobal;
pub use into_local::IntoLocal;
pub use try_into_global::TryIntoGlobal;

pub struct SubModule<V, M> {
	module: M,
	interface: Interface,
	vocabulary: PhantomData<V>,
}

impl<V, M> SubModule<V, M> {
	pub fn new(module: M) -> Self {
		Self {
			module,
			interface: Interface::new(),
			vocabulary: PhantomData,
		}
	}

	pub fn module(&self) -> &M {
		&self.module
	}

	pub fn interface(&self) -> &Interface {
		&self.interface
	}
}

#[derive(Debug, thiserror::Error)]
pub enum SubModuleError<E> {
	#[error(transparent)]
	Module(E),

	#[error(transparent)]
	Contradiction(#[from] Contradiction),
}

impl<V: Vocabulary, M: Module<V>> SubModule<V, M> {
	pub fn filter_triple(
		&self,
		vocabulary: &mut V,
		global_triple: Triple,
		sign: Sign,
	) -> Result<bool, SubModuleError<M::Error>> {
		match global_triple.into_local(&self.interface) {
			Some(local_triple) => {
				match self
					.module
					.dataset()
					.find_triple(vocabulary, local_triple)
					.map_err(SubModuleError::Module)?
				{
					Some((_, signed)) => {
						if signed.sign() == sign {
							Ok(false)
						} else {
							Err(SubModuleError::Contradiction(Contradiction(local_triple)))
						}
					}
					None => Ok(true),
				}
			}
			None => Ok(true),
		}
	}

	pub fn pattern_matching<G>(
		&self,
		global_pattern: Signed<pattern::Canonical>,
		generator: G,
	) -> PatternMatching<V, M, G> {
		let local_pattern = global_pattern.into_local(&self.interface);

		PatternMatching {
			interface: &self.interface,
			inner: local_pattern
				.map(|local_pattern| self.module.dataset().pattern_matching(local_pattern)),
			generator,
		}
	}
}

#[derive(Default)]
pub struct Interface {
	/// Maps sub-module-local identifiers to composition-global identifiers.
	local_to_global: RefCell<HashMap<Id, Id>>,

	/// Maps composition-global identifiers to sub-module-local identifiers.
	global_to_local: RefCell<HashMap<Id, Id>>,
}

impl Interface {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get_or_insert_global(&self, local_id: Id, f: impl FnOnce() -> Id) -> Id {
		let map = self.local_to_global.borrow();
		match map.get(&local_id) {
			Some(global_id) => *global_id,
			None => {
				std::mem::drop(map);
				let global_id = f();
				self.set_global_id(local_id, global_id);
				global_id
			}
		}
	}

	pub fn get_or_try_insert_global<E>(
		&self,
		local_id: Id,
		f: impl FnOnce(Id) -> Result<Id, E>,
	) -> Result<Id, E> {
		let map = self.local_to_global.borrow();
		match map.get(&local_id) {
			Some(global_id) => Ok(*global_id),
			None => {
				std::mem::drop(map);
				let global_id = f(local_id)?;
				self.set_global_id(local_id, global_id);
				Ok(global_id)
			}
		}
	}

	pub fn global_id(&self, local_id: Id) -> Option<Id> {
		let map = self.local_to_global.borrow();
		map.get(&local_id).copied()
	}

	pub fn local_id(&self, global_id: Id) -> Option<Id> {
		let map = self.global_to_local.borrow();
		map.get(&global_id).copied()
	}

	pub fn set_global_id(&self, local_id: Id, global_id: Id) {
		let mut map = self.local_to_global.borrow_mut();
		map.insert(local_id, global_id);
		let mut map = self.global_to_local.borrow_mut();
		map.insert(global_id, local_id);
	}
}

pub trait ResourceGenerator {
	fn new_resource(&mut self) -> Id;
}

pub struct PatternMatching<'a, V: 'a + Vocabulary, M: 'a + Module<V>, G> {
	interface: &'a Interface,
	inner: Option<dataset::Matching<'a, V, M::Dataset<'a>>>,
	generator: G,
}

impl<'a, V: 'a + Vocabulary, M: 'a + Module<V>, G: ResourceGenerator> FailibleIteratorWith<V>
	for PatternMatching<'a, V, M, G>
{
	type Item = (TripleId, Fact);
	type Error = M::Error;

	fn try_next_with(&mut self, vocabulary: &mut V) -> Result<Option<Self::Item>, Self::Error> {
		match &mut self.inner {
			Some(i) => i.try_next_with(vocabulary).map(|r| {
				r.map(|(id, fact)| {
					(
						id,
						fact.into_global(self.interface, || self.generator.new_resource()),
					)
				})
			}),
			None => Ok(None),
		}
	}
}

pub trait GlobalClassification {
	fn group(&self, id: GroupId) -> Option<&class::group::Description>;

	fn insert_group(&self, desc: &class::group::Description) -> GroupId;
}

#[derive(Default)]
pub struct ClassificationInterface {
	global_to_local_description: MemoMap<class::group::Description, Option<(class::group::Description, class::group::MembersSubstitution)>>,

	local_to_global_description: MemoMap<class::group::Description, (class::group::Description, class::group::MembersSubstitution)>
}

impl ClassificationInterface {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn local_group<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		global_id: GroupId
	) -> Result<Option<(GroupId, &class::group::MembersSubstitution)>, M::Error> {
		use crate::Classification;
		let global_desc = global_classification.group(global_id).unwrap();
		match self.local_group_description(global_classification, sub_module, global_desc)? {
			Some((local_desc, substitution)) => {
				Ok(sub_module.module().classification().find_group_id(local_desc)?.map(|local_id| (local_id, substitution)))
			}
			None => Ok(None)
		}
	}

	pub fn local_group_description<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		global_desc: &class::group::Description
	) -> Result<Option<(&class::group::Description, &class::group::MembersSubstitution)>, M::Error> {
		self.global_to_local_description.get_or_try_insert(
			&global_desc,
			|| {
				let mut local_members = Vec::with_capacity(global_desc.members.len());
				for member in &global_desc.members {
					match self.local_group_member(global_classification, sub_module, member)? {
						Some(m) => local_members.push(m),
						None => return Ok(None)
					}
				}

				Ok(Some(class::group::Description::new(local_members)))
			}
		).map(|r| r.as_ref().map(|(d, s)| (d, s)))
	}

	fn local_group_member<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		member: &class::group::Member,
	) -> Result<Option<class::group::Member>, M::Error> {
		let mut local_properties = Vec::with_capacity(member.properties.len());
		for Signed(sign, (a, b)) in &member.properties.0 {
			if let Some(local_a) = self.local_reference(global_classification, sub_module, a)? {
				if let Some(local_b) = self.local_reference(global_classification, sub_module, b)? {
					local_properties.push(Signed(*sign, (local_a, local_b)))
				}
			}
		}

		local_properties.sort_unstable();
		Ok(Some(class::group::Member::new(local_properties)))
	}

	fn local_reference<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		r: &class::Reference
	) -> Result<Option<class::Reference>, M::Error> {
		match r {
			class::Reference::Class(c) => {
				match self.local_class(global_classification, sub_module, *c)? {
					Some(local_c) => Ok(Some(class::Reference::Class(local_c))),
					None => Ok(None)
				}
			}
			class::Reference::Group(index) => {
				Ok(Some(class::Reference::Group(*index)))
			}
			class::Reference::Singleton(global_id) => {
				match global_id.into_local(sub_module.interface()) {
					Some(local_id) => Ok(Some(class::Reference::Singleton(local_id))),
					None => Ok(None)
				}
			}
		}
	}

	pub fn local_class<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		c: Class
	) -> Result<Option<Class>, M::Error> {
		match self.local_group(global_classification, sub_module, c.group)? {
			Some((local_group, substitution)) => {
				let local_member = substitution.get(c.member).unwrap();
				Ok(Some(Class::new(local_group, local_member)))
			}
			None => Ok(None)
		}
	}

	pub fn global_group<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		local_id: GroupId,
		mut import_local_resource: impl FnMut(Id) -> Result<Id, M::Error>
	) -> Result<(GroupId, &class::group::MembersSubstitution), M::Error> {
		self.global_group_with(global_classification, sub_module, local_id, &mut import_local_resource)
	}

	fn global_group_with<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		local_id: GroupId,
		import_local_resource: &mut impl FnMut(Id) -> Result<Id, M::Error>
	) -> Result<(GroupId, &class::group::MembersSubstitution), M::Error> {
		use crate::Classification;
		let classification = sub_module.module().classification();
		let local_desc = classification.group(local_id)?.unwrap();
		let (global_desc, sub) = self.global_group_description(global_classification, sub_module, &local_desc, import_local_resource)?;
		Ok((global_classification.insert_group(global_desc), sub))
	}

	fn global_group_description<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		local_desc: &class::group::Description,
		import_local_resource: &mut impl FnMut(Id) -> Result<Id, M::Error>
	) -> Result<(&class::group::Description, &class::group::MembersSubstitution), M::Error> {
		self.local_to_global_description.get_or_try_insert(
			&local_desc,
			|| {
				let mut global_members = Vec::with_capacity(local_desc.members.len());
				for member in &local_desc.members {
					let global_member = self.global_group_member(global_classification, sub_module, member, import_local_resource)?;
					global_members.push(global_member)
				}

				Ok(class::group::Description::new(global_members))
			}
		).map(|(d, s)| (d, s))
	}

	fn global_group_member<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		local_member: &class::group::Member,
		import_local_resource: &mut impl FnMut(Id) -> Result<Id, M::Error>
	) -> Result<class::group::Member, M::Error> {
		let mut global_properties = Vec::with_capacity(local_member.properties.len());
		for Signed(sign, (a, b)) in &local_member.properties.0 {
			let global_a = self.global_reference(global_classification, sub_module, a, &mut *import_local_resource)?;
			let global_b = self.global_reference(global_classification, sub_module, b, &mut *import_local_resource)?;
			global_properties.push(Signed(*sign, (global_a, global_b)))
		}

		global_properties.sort_unstable();
		Ok(class::group::Member::new(global_properties))
	}

	fn global_reference<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		r: &class::Reference,
		import_local_resource: &mut impl FnMut(Id) -> Result<Id, M::Error>
	) -> Result<class::Reference, M::Error> {
		match r {
			class::Reference::Class(local_class) => {
				let global_c = self.global_class(global_classification, sub_module, *local_class, import_local_resource)?;
				Ok(class::Reference::Class(global_c))
			}
			class::Reference::Group(index) => {
				Ok(class::Reference::Group(*index))
			}
			class::Reference::Singleton(local_id) => {
				let global_id = local_id.try_into_global(sub_module.interface(), import_local_resource)?;
				Ok(class::Reference::Singleton(global_id))
			}
		}
	}

	pub fn global_class<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		local_class: Class,
		mut import_local_resource: impl FnMut(Id) -> Result<Id, M::Error>
	) -> Result<Class, M::Error> {
		self.global_class_with(global_classification, sub_module, local_class, &mut import_local_resource)
	}

	fn global_class_with<V: Vocabulary, M: Module<V>>(
		&self,
		global_classification: &impl GlobalClassification,
		sub_module: &SubModule<V, M>,
		local_class: Class,
		import_local_resource: &mut impl FnMut(Id) -> Result<Id, M::Error>
	) -> Result<Class, M::Error> {
		let (global_group, substitution) = self.global_group(global_classification, sub_module, local_class.group, import_local_resource)?;
		let global_member = substitution.get(local_class.member).unwrap();
		Ok(Class::new(global_group, global_member))
	}
}