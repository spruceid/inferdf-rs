pub mod graph;

use derivative::Derivative;
pub use graph::{Contradiction, Graph};
use hashbrown::HashMap;
use locspan::Meta;

use crate::{pattern, Id, Quad, ReplaceId, Sign, Signed, Triple};

use self::graph::FactWithGraph;

pub type Fact<M> = Meta<Signed<Quad>, M>;

pub trait IntoGraphFact<M> {
	fn into_graph_fact(self) -> (graph::Fact<M>, Option<Id>);
}

impl<M> IntoGraphFact<M> for Meta<Signed<Quad>, M> {
	fn into_graph_fact(self) -> (graph::Fact<M>, Option<Id>) {
		let Meta(Signed(sign, quad), meta) = self;
		let (triple, g) = quad.into_triple();
		(Meta(Signed(sign, triple), meta), g)
	}
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Dataset<M> {
	default_graph: Graph<M>,
	named_graphs: HashMap<Id, Graph<M>>,
}

impl<M> Dataset<M> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn contains_triple(&self, triple: Triple, sign: Sign) -> bool {
		self.default_graph.contains(triple, sign)
			|| self.named_graphs.values().any(|g| g.contains(triple, sign))
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

	pub fn insert(
		&mut self,
		fact: Meta<Signed<Quad>, M>,
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

	pub fn matching(&self, pattern: pattern::Canonical) -> Matching<M> {
		Matching {
			pattern,
			graphs: self.graphs(),
			current: None,
			sign: None,
		}
	}

	pub fn signed_matching(
		&self,
		Signed(sign, pattern): Signed<pattern::Canonical>,
	) -> Matching<M> {
		Matching {
			pattern,
			graphs: self.graphs(),
			current: None,
			sign: Some(sign),
		}
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
				Some((_, fact)) => return Some(fact.borrow_metadata().with_graph(*g)),
				None => {
					self.list.pop();
				}
			}
		}

		None
	}
}

pub struct Matching<'a, M> {
	pattern: pattern::Canonical,
	graphs: Graphs<'a, M>,
	current: Option<(Option<Id>, graph::Matching<'a, M>)>,
	sign: Option<Sign>,
}

impl<'a, M> Matching<'a, M> {
	pub fn into_quads(self) -> MatchingQuads<'a, M> {
		MatchingQuads(self)
	}
}

impl<'a, M> Iterator for Matching<'a, M> {
	type Item = (Option<Id>, usize, &'a graph::Fact<M>);

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

pub struct MatchingQuads<'a, M>(Matching<'a, M>);

impl<'a, M> Iterator for MatchingQuads<'a, M> {
	type Item = Quad;

	fn next(&mut self) -> Option<Self::Item> {
		self.0
			.next()
			.map(|(g, _, Meta(Signed(_, triple), _))| triple.into_quad(g))
	}
}
