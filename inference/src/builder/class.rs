//! Classes definition.
//!
//! Classes are only for anonymous nodes (without IRI or literal value).
use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use indexmap::IndexSet;
use inferdf_core::{
	class::{self, Class},
	Id, Module, Signed,
};
use locspan::Meta;
use rdf_types::Vocabulary;

use crate::utils::SccGraph;

use super::Builder;

mod non_reflexive;
mod reflexive;

pub struct Classification {
	pub groups: Vec<class::group::Description>,
	pub classes: HashMap<Id, Class>,
}

#[derive(Default)]
struct Groups {
	set: IndexSet<class::group::Description>,
}

impl Groups {
	pub fn insert(&mut self, description: class::group::Description) -> class::GroupId {
		class::GroupId::new(self.set.insert_full(description).0 as u32)
	}

	pub fn into_vec(self) -> Vec<class::group::Description> {
		self.set.into_iter().collect()
	}
}

impl<V: Vocabulary, D: Module<V>, S> Builder<V, D, S> {
	/// Classify anonymous nodes.
	///
	/// # Classification algorithm
	///
	/// Overview of the algorithm:
	/// - The local dataset is scaned to find all the anonymous nodes.
	/// - A dependency graph is computed where each node is an anonymous node.
	/// - Graph nodes are merged into strongly connected components.
	/// - SCCs are treated by dependency depth, starting by the leaves:
	///   - If the component is reflexive (it depends on itself),
	///     a canonicalization algorithm is used to compute the class.
	///   - Otherwise the class is directly computed.
	///   - The depth's component classes are sorted and given a unique index
	///     (following the classes ascending order).
	pub fn classify_anonymous_nodes(&mut self) -> Result<Classification, D::Error> {
		let mut graph = Graph {
			nodes: HashMap::new(),
		};

		// Find anonymous nodes and compute the dependency graph.
		for (id, r) in self.local_interpretation().iter() {
			if r.is_anonymous() {
				let mut successors = HashSet::new();

				for (_, _, Meta(Signed(sign, quad), _)) in self
					.dataset()
					.matching(rdf_types::Triple(Some(id), None, None).into())
				{
					let p = ResourceNode::new(self, quad.1);
					let o = ResourceNode::new(self, quad.2);
					let bnode = Node::Triple(Signed(*sign, (id, p, o)));
					graph.nodes.insert(
						bnode,
						[Node::Resource(p), Node::Resource(o)].into_iter().collect(),
					);
					successors.insert(bnode);
				}

				graph
					.nodes
					.insert(Node::Resource(ResourceNode::Anonymous(id)), successors);
			}
		}

		let components = graph.strongly_connected_components();

		let mut groups = Groups::default();
		let mut classes = HashMap::new();

		// Organize SCCs by depth.
		let depths = components.depths();
		let max_depth = *depths.iter().max().unwrap();
		let mut by_depth: Vec<Vec<usize>> = Vec::new();
		by_depth.resize_with(max_depth, Vec::new);

		for (c, d) in depths.into_iter().enumerate() {
			by_depth[d].push(c);
		}

		// Compute the class of each component, proceding by depth, starting by
		// the leaves.
		for layer_components in by_depth.into_iter().rev() {
			let mut layer_groups = LayerGroups::default();
			let mut layer_classes = HashMap::new();

			// Compute each component's class description.
			for c in layer_components {
				if components.is_reflexive(c) {
					reflexive::compute_component(
						&components,
						&classes,
						&mut layer_groups,
						&mut layer_classes,
						c,
					)
				} else {
					non_reflexive::compute_component(
						&components,
						&classes,
						&mut layer_groups,
						&mut layer_classes,
						c,
					)
				}
			}

			layer_groups.insert_all(&mut groups, &mut classes, layer_classes)
		}

		Ok(Classification {
			groups: groups.into_vec(),
			classes,
		})
	}
}

#[derive(Default)]
pub(crate) struct LayerGroups {
	list: Vec<class::group::Description>,
}

impl LayerGroups {
	fn add(&mut self, group: class::group::Description) -> usize {
		let i = self.list.len();
		self.list.push(group);
		i
	}

	fn sort(&mut self) -> Vec<usize> {
		let mut indexes: Vec<_> = (0..self.list.len()).collect();
		indexes.sort_unstable_by_key(|i| &self.list[*i]);
		let mut substitution = Vec::new();
		substitution.resize(self.list.len(), 0);
		for (j, i) in indexes.into_iter().enumerate() {
			substitution[i] = j;

			let k = if i < j { substitution[i] } else { i };
			self.list.swap(j, k)
		}

		// check that the list is sorted.
		debug_assert!(self.list.windows(2).all(|w| w[0] <= w[1]));

		substitution
	}

	fn insert_all(
		mut self,
		groups: &mut Groups,
		classes: &mut HashMap<Id, Class>,
		layer_classes: HashMap<Id, LayerClass>,
	) {
		let substitution = self.sort();
		let group_ids: Vec<_> = self.list.into_iter().map(|g| groups.insert(g)).collect();
		for (id, layer_class) in layer_classes {
			let class = Class::new(
				group_ids[substitution[layer_class.group]],
				layer_class.member,
			);
			classes.insert(id, class);
		}
	}
}

pub(crate) struct LayerClass {
	group: usize,
	member: u32,
}

impl LayerClass {
	pub fn new(group: usize, member: u32) -> Self {
		Self { group, member }
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ResourceNode {
	Anonymous(Id),
	Named(Id),
}

impl ResourceNode {
	fn new<V: Vocabulary, D: Module<V>, S>(builder: &Builder<V, D, S>, id: Id) -> Self {
		let r = builder.local_interpretation().get(id).unwrap();
		if r.is_anonymous() {
			Self::Anonymous(id)
		} else {
			Self::Named(id)
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Node {
	Resource(ResourceNode),
	Triple(Signed<(Id, ResourceNode, ResourceNode)>),
}

struct Graph {
	nodes: HashMap<Node, HashSet<Node>>,
}

impl SccGraph for Graph {
	type Vertex = Node;

	type Vertices<'a> = std::iter::Copied<hashbrown::hash_map::Keys<'a, Node, HashSet<Node>>>;

	type Successors<'a> = std::iter::Copied<hashbrown::hash_set::Iter<'a, Node>>;

	fn vertices(&self) -> Self::Vertices<'_> {
		self.nodes.keys().copied()
	}

	fn successors(&self, v: Self::Vertex) -> Self::Successors<'_> {
		self.nodes.get(&v).unwrap().iter().copied()
	}
}
