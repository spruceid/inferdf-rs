use hashbrown::HashMap;
use inferdf_core::{
	class::{self, Class},
	Id, Signed,
};

use crate::{builder::class::ResourceNode, utils::scc::Components};

use super::{LayerClass, LayerGroupsBuilder, Node};

pub(crate) fn compute_component(
	components: &Components<Node>,
	classes: &HashMap<Id, Class>,
	new_groups: &mut LayerGroupsBuilder,
	new_classes: &mut HashMap<Id, LayerClass>,
	c: usize,
) {
	let component = components.get(c).unwrap();
	debug_assert_eq!(component.len(), 1);
	if let Node::Resource(ResourceNode::Anonymous(id)) = component[0] {
		let mut bindings: Vec<_> = components
			.successors(c)
			.unwrap()
			.flat_map(|d| {
				components
					.get(d)
					.unwrap()
					.iter()
					.filter_map(|node| match node {
						Node::Triple(Signed(sign, (other_id, a, b))) if *other_id == id => {
							let a = build_class_reference(classes, *a);
							let b = build_class_reference(classes, *b);
							Some(Signed(*sign, (a, b)))
						}
						_ => None,
					})
			})
			.collect();
		bindings.sort_unstable();

		let member = class::group::Member::new(bindings);
		let group = class::group::Description::non_reflexive(member);
		let group_id = new_groups.add(group);
		new_classes.insert(id, LayerClass::new(group_id, 0));
	}
}

fn build_class_reference(classes: &HashMap<Id, Class>, node: ResourceNode) -> class::Reference {
	match node {
		ResourceNode::Named(id) => class::Reference::Singleton(id),
		ResourceNode::Anonymous(id) => class::Reference::Class(*classes.get(&id).unwrap()),
	}
}
