use std::hash::Hash;

use hashbrown::HashMap;
use inferdf_core::{
	class::{self, Class},
	Id, Signed,
};
use normal_form::Normalize;

use crate::utils::scc::Components;

use super::{LayerClass, LayerGroups, Node, ResourceNode};

pub(crate) fn compute_component(
	components: &Components<Node>,
	classes: &HashMap<Id, Class>,
	new_groups: &mut LayerGroups,
	new_classes: &mut HashMap<Id, LayerClass>,
	c: usize,
) {
	let component = components.get(c).unwrap();
	let mut members: Vec<class::group::Member> = Vec::new();
	let mut members_id: HashMap<Id, usize> = HashMap::new();
	let mut neighbors = Vec::new();

	for node in component {
		if let Node::Resource(ResourceNode::Anonymous(id)) = node {
			let i = members.len();
			members.push(class::group::Member::default());
			neighbors.push(Vec::new());
			members_id.insert(*id, i);
		}
	}

	for node in component {
		if let Node::Triple(Signed(sign, (id, a, b))) = node {
			let i = *members_id.get(id).unwrap();
			let a = build_class_reference(classes, &members_id, *a);
			let b = build_class_reference(classes, &members_id, *b);

			if let class::Reference::Group(a) = a {
				neighbors[i].push(a)
			}

			if let class::Reference::Group(b) = b {
				neighbors[i].push(b)
			}

			members[i].add(Signed(*sign, (a, b)));
		}
	}

	for d in components.successors(c).unwrap() {
		for node in components.get(d).unwrap() {
			if let Node::Triple(Signed(sign, (id, a, b))) = node {
				if let Some(&i) = members_id.get(id) {
					let a = build_class_reference(classes, &members_id, *a);
					let b = build_class_reference(classes, &members_id, *b);

					if let class::Reference::Group(a) = a {
						neighbors[i].push(a)
					}

					if let class::Reference::Group(b) = b {
						neighbors[i].push(b)
					}

					members[i].add(Signed(*sign, (a, b)));
				}
			}
		}
	}

	for n in &mut neighbors {
		n.sort_unstable();
		n.dedup();
	}

	let group = Group::new(members, neighbors);
	let (canonical_group, morphism) = group.normalize();
	let group_id = new_groups.add(canonical_group);

	for (id, i) in members_id {
		new_classes.insert(id, LayerClass::new(group_id, morphism[i] as u32));
	}
}

fn build_class_reference(
	classes: &HashMap<Id, Class>,
	group_members: &HashMap<Id, usize>,
	node: ResourceNode,
) -> class::Reference {
	match node {
		ResourceNode::Named(id) => class::Reference::Singleton(id),
		ResourceNode::Anonymous(id) => match group_members.get(&id) {
			Some(&i) => class::Reference::Group(i as u32),
			None => class::Reference::Class(*classes.get(&id).unwrap()),
		},
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Group {
	members: Vec<class::group::Member>,
	len: u32,
	neighbors: Vec<Vec<u32>>,
}

impl Group {
	pub fn new(members: Vec<class::group::Member>, neighbors: Vec<Vec<u32>>) -> Self {
		let len = members.len() as u32;

		Self {
			members,
			len,
			neighbors,
		}
	}
}

impl Normalize for Group {
	type Elements = u32;

	type Color = Color;

	type Morphed = class::group::Description;

	type Cache = Cache;

	fn elements(&self) -> &Self::Elements {
		&self.len
	}

	fn initial_coloring(&self) -> Vec<Color> {
		self.members
			.iter()
			.map(|m| {
				let mut picker = ColorPicker::with_capacity(m.len());
				for &binding in m {
					picker.insert(binding)
				}
				picker.pick()
			})
			.collect()
	}

	fn initialize_cache(&self) -> Self::Cache {
		let mut map = Vec::new();
		map.resize(self.len as usize, 0usize);

		Cache {
			stack: Vec::new(),
			map,
		}
	}

	fn refine_coloring(
		&self,
		cache: &mut Self::Cache,
		coloring: &mut normal_form::ReversibleColoring<Self::Elements>,
	) {
		coloring.make_equitable_with(&mut cache.stack, &mut cache.map, |i| {
			&self.neighbors[*i as usize]
		})
	}

	fn apply_morphism<F>(&self, morphism: F) -> Self::Morphed
	where
		F: Fn(&u32) -> usize,
	{
		let mut members = self.members.clone();

		fn apply_morphism_on_reference(
			reference: &mut class::Reference,
			f: impl Fn(&u32) -> usize,
		) {
			if let class::Reference::Group(x) = reference {
				*x = f(x) as u32
			}
		}

		for m in &mut members {
			for Signed(_, (a, b)) in m {
				apply_morphism_on_reference(a, &morphism);
				apply_morphism_on_reference(b, &morphism)
			}
		}

		members.sort_unstable();
		class::group::Description::new(members)
	}
}

struct Cache {
	stack: Vec<usize>,
	map: Vec<usize>,
}

/// Group member color.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Color(Vec<Pigment>);

/// Color builder.
#[derive(Default)]
struct ColorPicker(Vec<Pigment>);

impl ColorPicker {
	pub fn with_capacity(capacity: usize) -> Self {
		Self(Vec::with_capacity(capacity))
	}

	pub fn pick(self) -> Color {
		let mut pigments = self.0;
		pigments.sort_unstable();
		Color(pigments)
	}

	pub fn insert(&mut self, pigment: impl Into<Pigment>) {
		self.0.push(pigment.into())
	}
}

/// Color pigment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Pigment(Signed<(AnonymousReference, AnonymousReference)>);

impl From<Signed<(class::Reference, class::Reference)>> for Pigment {
	fn from(Signed(sign, (a, b)): Signed<(class::Reference, class::Reference)>) -> Self {
		Self(Signed(sign, (a.into(), b.into())))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AnonymousReference {
	Singleton(Id),
	Class(Class),
	Group,
}

impl From<class::Reference> for AnonymousReference {
	fn from(value: class::Reference) -> Self {
		match value {
			class::Reference::Singleton(id) => Self::Singleton(id),
			class::Reference::Class(id) => Self::Class(id),
			class::Reference::Group(_) => Self::Group,
		}
	}
}
