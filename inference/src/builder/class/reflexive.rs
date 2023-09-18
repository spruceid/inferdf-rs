use hashbrown::HashMap;
use inferdf_core::{
	class::{self, Class},
	Id, Signed,
};
use normal_form::Normalize;

use crate::utils::scc::Components;

use super::{LayerClass, LayerGroupsBuilder, Node, ResourceNode};

pub(crate) fn compute_component(
	components: &Components<Node>,
	classes: &HashMap<Id, Class>,
	new_groups: &mut LayerGroupsBuilder,
	new_classes: &mut HashMap<Id, LayerClass>,
	c: usize,
) {
	let component = components.get(c).unwrap();
	let mut members: Vec<class::group::Member> = Vec::new();
	let mut members_id: HashMap<Id, usize> = HashMap::new();

	for node in component {
		if let Node::Resource(ResourceNode::Anonymous(id)) = node {
			let i = members.len();
			members.push(class::group::Member::default());
			members_id.insert(*id, i);
		}
	}

	for node in component {
		if let Node::Triple(Signed(sign, (id, a, b))) = node {
			let i = *members_id.get(id).unwrap();
			let a = build_class_reference(classes, &members_id, *a);
			let b = build_class_reference(classes, &members_id, *b);
			members[i].add(Signed(*sign, (a, b)));
		}
	}

	for d in components.successors(c).unwrap() {
		for node in components.get(d).unwrap() {
			if let Node::Triple(Signed(sign, (id, a, b))) = node {
				if let Some(&i) = members_id.get(id) {
					let a = build_class_reference(classes, &members_id, *a);
					let b = build_class_reference(classes, &members_id, *b);
					members[i].add(Signed(*sign, (a, b)));
				}
			}
		}
	}

	let desc = class::group::NonNormalizedDescription::new(members);
	let (canonical_group, morphism) = desc.normalize();
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
