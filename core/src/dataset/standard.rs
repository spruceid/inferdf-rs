pub mod graph;

use derivative::Derivative;
pub use graph::Graph;
use hashbrown::HashMap;
use locspan::Meta;

use crate::{pattern, Cause, Id, Quad, ReplaceId, Sign, Signed, Triple};

use self::graph::FactWithGraph;

use super::Contradiction;

pub type Fact = Meta<Signed<Quad>, Cause>;

pub trait IntoGraphFact {
	fn into_graph_fact(self) -> (graph::Fact, Option<Id>);
}

impl IntoGraphFact for Meta<Signed<Quad>, Cause> {
	fn into_graph_fact(self) -> (graph::Fact, Option<Id>) {
		let Meta(Signed(sign, quad), meta) = self;
		let (triple, g) = quad.into_triple();
		(Meta(Signed(sign, triple), meta), g)
	}
}

/// Standard dataset.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Standard {
	default_graph: Graph,
	named_graphs: HashMap<Id, Graph>,
}

impl Standard {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn contains_triple(&self, triple: Triple, sign: Sign) -> bool {
		self.default_graph.contains(triple, sign)
			|| self.named_graphs.values().any(|g| g.contains(triple, sign))
	}

	pub fn find_triple(&self, triple: Triple) -> Option<(Option<Id>, usize, &graph::Fact)> {
		self.default_graph
			.find_triple(triple)
			.map(|(i, t)| (None, i, t))
			.or_else(|| {
				self.named_graphs
					.iter()
					.find_map(|(g, graph)| graph.find_triple(triple).map(|(i, t)| (Some(*g), i, t)))
			})
	}

	pub fn graph(&self, id: Option<Id>) -> Option<&Graph> {
		match id {
			None => Some(&self.default_graph),
			Some(id) => self.named_graphs.get(&id),
		}
	}

	pub fn get_or_insert_graph_mut(&mut self, id: Option<Id>) -> &mut Graph {
		match id {
			None => &mut self.default_graph,
			Some(id) => self.named_graphs.entry(id).or_default(),
		}
	}

	pub fn iter(&self) -> Iter {
		Iter {
			graphs: self.graphs(),
			current: None,
		}
	}

	pub fn graphs(&self) -> Graphs {
		Graphs {
			default_graph: Some(&self.default_graph),
			named_graphs: self.named_graphs.iter(),
		}
	}

	pub fn graphs_mut(&mut self) -> GraphsMut {
		GraphsMut {
			default_graph: Some(&mut self.default_graph),
			named_graphs: self.named_graphs.iter_mut(),
		}
	}

	pub fn remove_graph(&mut self, id: Id) -> Option<Graph> {
		self.named_graphs.remove(&id)
	}

	pub fn insert_graph(&mut self, id: Id, graph: Graph) -> Option<Graph> {
		self.named_graphs.insert(id, graph)
	}

	pub fn insert(
		&mut self,
		fact: Meta<Signed<Quad>, Cause>,
	) -> Result<(Option<Id>, usize, bool), Contradiction> {
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
		filter: impl Fn(&graph::Fact) -> Result<bool, Contradiction>,
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

	pub fn resource_facts(&self, id: Id) -> ResourceFacts {
		let mut list = Vec::new();

		for (g, graph) in self.graphs() {
			let facts = graph.resource_facts(id);
			if !facts.is_empty() {
				list.push((g, facts))
			}
		}

		ResourceFacts { list }
	}

	pub fn matching(&self, pattern: pattern::Canonical) -> Matching {
		Matching {
			pattern,
			graphs: self.graphs(),
			current: None,
			sign: None,
		}
	}

	pub fn signed_matching(&self, Signed(sign, pattern): Signed<pattern::Canonical>) -> Matching {
		Matching {
			pattern,
			graphs: self.graphs(),
			current: None,
			sign: Some(sign),
		}
	}
}

pub struct Graphs<'a> {
	default_graph: Option<&'a Graph>,
	named_graphs: hashbrown::hash_map::Iter<'a, Id, Graph>,
}

impl<'a> Iterator for Graphs<'a> {
	type Item = (Option<Id>, &'a Graph);

	fn next(&mut self) -> Option<Self::Item> {
		self.default_graph
			.take()
			.map(|g| (None, g))
			.or_else(|| self.named_graphs.next().map(|(id, g)| (Some(*id), g)))
	}
}

pub struct GraphsMut<'a> {
	default_graph: Option<&'a mut Graph>,
	named_graphs: hashbrown::hash_map::IterMut<'a, Id, Graph>,
}

impl<'a> Iterator for GraphsMut<'a> {
	type Item = (Option<Id>, &'a mut Graph);

	fn next(&mut self) -> Option<Self::Item> {
		self.default_graph
			.take()
			.map(|g| (None, g))
			.or_else(|| self.named_graphs.next().map(|(id, g)| (Some(*id), g)))
	}
}

pub struct ResourceFacts<'a> {
	list: Vec<(Option<Id>, graph::ResourceFacts<'a>)>,
}

impl<'a> ResourceFacts<'a> {
	pub fn is_empty(&self) -> bool {
		self.list.is_empty() || (self.list.len() == 1 && self.list[0].1.is_empty())
	}
}

impl<'a> Iterator for ResourceFacts<'a> {
	type Item = Fact;

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

pub struct Matching<'a> {
	pattern: pattern::Canonical,
	graphs: Graphs<'a>,
	current: Option<(Option<Id>, graph::Matching<'a>)>,
	sign: Option<Sign>,
}

impl<'a> Matching<'a> {
	pub fn into_quads(self) -> MatchingQuads<'a> {
		MatchingQuads(self)
	}
}

impl<'a> Iterator for Matching<'a> {
	type Item = (Option<Id>, usize, &'a graph::Fact);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some((g, m)) => match m.next() {
					Some((i, triple)) => break Some((*g, i, triple)),
					None => self.current = None,
				},
				None => match self.graphs.next() {
					Some((g, graph)) => {
						self.current = Some((g, graph.full_matching(self.pattern, self.sign)))
					}
					None => break None,
				},
			}
		}
	}
}

pub struct MatchingQuads<'a>(Matching<'a>);

impl<'a> Iterator for MatchingQuads<'a> {
	type Item = Quad;

	fn next(&mut self) -> Option<Self::Item> {
		self.0
			.next()
			.map(|(g, _, Meta(Signed(_, triple), _))| triple.into_quad(g))
	}
}

pub struct Iter<'a> {
	graphs: Graphs<'a>,
	current: Option<(Option<Id>, graph::Iter<'a>)>,
}

impl<'a> Iter<'a> {
	pub fn into_quads(self) -> IterQuads<'a> {
		IterQuads(self)
	}
}

impl<'a> Iterator for Iter<'a> {
	type Item = (Option<Id>, usize, &'a graph::Fact);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some((g, m)) => match m.next() {
					Some((i, triple)) => break Some((*g, i, triple)),
					None => self.current = None,
				},
				None => match self.graphs.next() {
					Some((g, graph)) => self.current = Some((g, graph.iter())),
					None => break None,
				},
			}
		}
	}
}

pub struct IterQuads<'a>(Iter<'a>);

impl<'a> Iterator for IterQuads<'a> {
	type Item = Quad;

	fn next(&mut self) -> Option<Self::Item> {
		self.0
			.next()
			.map(|(g, _, Meta(Signed(_, triple), _))| triple.into_quad(g))
	}
}
