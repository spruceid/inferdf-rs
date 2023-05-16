pub mod graph;

pub use graph::{Contradiction, Graph};
use hashbrown::HashMap;

use crate::{Cause, Id, Quad, Triple, ReplaceId};

#[derive(Debug, Clone)]
pub struct Fact<M> {
	pub quad: Quad,
	pub positive: bool,
	pub cause: Cause<M>,
}

impl<M> Fact<M> {
	pub fn new(quad: Quad, positive: bool, cause: Cause<M>) -> Self {
		Self {
			quad,
			positive,
			cause,
		}
	}

	pub fn triple(&self) -> Triple {
		self.quad.into_triple().0
	}

	pub fn into_graph_fact(self) -> (graph::Fact<M>, Option<Id>) {
		let (triple, g) = self.quad.into_triple();
		(graph::Fact::new(triple, self.positive, self.cause), g)
	}
}

impl<M> ReplaceId for Fact<M> {
	fn replace_id(&mut self, a: Id, b: Id) {
		self.quad.replace_id(a, b)
	}
}

pub struct Dataset<M> {
	default_graph: Graph<M>,
	named_graphs: HashMap<Id, Graph<M>>,
}

impl<M> Dataset<M> {
	pub fn contains_triple(&self, triple: Triple, positive: bool) -> bool {
		self.default_graph.contains(triple, positive)
			|| self
				.named_graphs
				.values()
				.any(|g| g.contains(triple, positive))
	}

	pub fn find_triple(&self, triple: Triple) -> Option<(Option<Id>, usize, &graph::Fact<M>)> {
		self.default_graph
			.find_triple(triple)
			.map(|(i, t)| (None, i, t))
			.or_else(|| {
				self.named_graphs
					.iter()
					.find_map(|(g, graph)| graph.find_triple(triple).map(|(i, t)| (Some(*g), i, t)))
			})
	}

	pub fn graph(&self, id: Option<Id>) -> Option<&Graph<M>> {
		match id {
			None => Some(&self.default_graph),
			Some(id) => self.named_graphs.get(&id),
		}
	}

	pub fn get_or_insert_graph_mut(&mut self, id: Option<Id>) -> &mut Graph<M> {
		match id {
			None => &mut self.default_graph,
			Some(id) => self.named_graphs.entry(id).or_default(),
		}
	}

	pub fn graphs(&self) -> Graphs<M> {
		Graphs {
			default_graph: Some(&self.default_graph),
			named_graphs: self.named_graphs.iter(),
		}
	}

	pub fn graphs_mut(&mut self) -> GraphsMut<M> {
		GraphsMut {
			default_graph: Some(&mut self.default_graph),
			named_graphs: self.named_graphs.iter_mut(),
		}
	}

	pub fn remove_graph(&mut self, id: Id) -> Option<Graph<M>> {
		self.named_graphs.remove(&id)
	}

	pub fn insert_graph(&mut self, id: Id, graph: Graph<M>) -> Option<Graph<M>> {
		self.named_graphs.insert(id, graph)
	}

	pub fn insert(&mut self, fact: Fact<M>) -> Result<(Option<Id>, usize, bool), Contradiction> {
		let (fact, g) = fact.into_graph_fact();
		match g {
			None => {
				let (i, b) = self.default_graph.insert(fact)?;
				Ok((None, i, b))
			}
			Some(g) => {
				let (i, b) = self.named_graphs.entry(g).or_default().insert(fact)?;
				Ok((Some(g), i, b))
			}
		}
	}

	/// Replaces occurences of `b` with `a`.
	pub fn replace_id(
		&mut self,
		a: Id,
		b: Id,
		filter: impl Fn(&graph::Fact<M>) -> Result<bool, Contradiction>,
	) -> Result<(), Contradiction> {
		for (_, graph) in self.graphs_mut() {
			graph.replace_id(a, b, &filter)?
		}

		if let Some(graph) = self.named_graphs.remove(&b) {
			for (_, mut fact) in graph {
				fact.replace_id(a, b);
				if filter(&fact)? {
					self.named_graphs.entry(a).or_default().insert(fact)?;
				}
			}
		}

		Ok(())
	}

	pub fn resource_facts(&self, id: Id) -> ResourceFacts<M> {
		let mut list = Vec::new();

		for (g, graph) in self.graphs() {
			let facts = graph.resource_facts(id);
			if !facts.is_empty() {
				list.push((g, facts))
			}
		}

		ResourceFacts { list }
	}
}

pub struct Graphs<'a, M> {
	default_graph: Option<&'a Graph<M>>,
	named_graphs: hashbrown::hash_map::Iter<'a, Id, Graph<M>>,
}

impl<'a, M> Iterator for Graphs<'a, M> {
	type Item = (Option<Id>, &'a Graph<M>);

	fn next(&mut self) -> Option<Self::Item> {
		self.default_graph
			.take()
			.map(|g| (None, g))
			.or_else(|| self.named_graphs.next().map(|(id, g)| (Some(*id), g)))
	}
}

pub struct GraphsMut<'a, M> {
	default_graph: Option<&'a mut Graph<M>>,
	named_graphs: hashbrown::hash_map::IterMut<'a, Id, Graph<M>>,
}

impl<'a, M> Iterator for GraphsMut<'a, M> {
	type Item = (Option<Id>, &'a mut Graph<M>);

	fn next(&mut self) -> Option<Self::Item> {
		self.default_graph
			.take()
			.map(|g| (None, g))
			.or_else(|| self.named_graphs.next().map(|(id, g)| (Some(*id), g)))
	}
}

pub struct ResourceFacts<'a, M> {
	list: Vec<(Option<Id>, graph::ResourceFacts<'a, M>)>,
}

impl<'a, M> ResourceFacts<'a, M> {
	pub fn is_empty(&self) -> bool {
		self.list.is_empty() || (self.list.len() == 1 && self.list[0].1.is_empty())
	}
}

impl<'a, M> Iterator for ResourceFacts<'a, M> {
	type Item = Fact<&'a M>;

	fn next(&mut self) -> Option<Self::Item> {
		while let Some((g, top)) = self.list.last_mut() {
			match top.next() {
				Some((_, fact)) => return Some(fact.with_graph(*g)),
				None => {
					self.list.pop();
				}
			}
		}

		None
	}
}
