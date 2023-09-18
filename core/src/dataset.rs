use locspan::Meta;

use crate::{
	pattern, Fact, FailibleIteratorWith, GraphFact, Id, IteratorWith, Sign, Signed, Triple,
};

pub mod graph;
pub mod local;

pub use graph::Graph;
pub use local::LocalDataset;

#[derive(Debug, thiserror::Error)]
#[error("statement contradiction")]
pub struct Contradiction(pub Triple);

/// RDF dataset.
pub trait Dataset<'a, V>: Clone {
	type Error;

	type Graph: Graph<'a, V, Error = Self::Error>;

	type Graphs: 'a + IteratorWith<V, Item = Result<(Option<Id>, Self::Graph), Self::Error>>;

	fn graphs(&self) -> Self::Graphs;

	fn graph(&self, id: Option<Id>) -> Result<Option<Self::Graph>, Self::Error>;

	fn default_graph(&self) -> Result<Self::Graph, Self::Error> {
		Ok(self.graph(None)?.unwrap())
	}

	fn resource_facts(&self, id: Id) -> ResourceFacts<'a, V, Self> {
		ResourceFacts {
			id,
			graph: self.graphs(),
			current: None,
		}
	}

	fn find_triple(
		&self,
		vocabulary: &mut V,
		triple: Triple,
	) -> Result<Option<(TripleId, Fact)>, Self::Error> {
		let mut graphs = self.graphs();
		while let Some(g) = graphs.next_with(vocabulary) {
			let (g, graph) = g?;
			if let Some((i, Meta(Signed(sign, t), meta))) = graph.find_triple(vocabulary, triple)? {
				return Ok(Some((
					TripleId::new(g, i),
					Meta(Signed(sign, t.into_quad(g)), meta),
				)));
			}
		}

		Ok(None)
	}

	fn full_pattern_matching(
		&self,
		pattern: pattern::Canonical,
		sign: Option<Sign>,
	) -> Matching<'a, V, Self> {
		Matching {
			pattern,
			graphs: self.graphs(),
			current: None,
			sign,
		}
	}

	fn pattern_matching(
		&self,
		Signed(sign, pattern): Signed<pattern::Canonical>,
	) -> Matching<'a, V, Self> {
		self.full_pattern_matching(pattern, Some(sign))
	}

	fn unsigned_pattern_matching(&self, pattern: pattern::Canonical) -> Matching<'a, V, Self> {
		self.full_pattern_matching(pattern, None)
	}

	fn iter(&self) -> Iter<'a, V, Self> {
		Iter {
			graphs: self.graphs(),
			current: None,
		}
	}
}

/// Triple identifier in a dataset.
pub struct TripleId {
	/// Identifier of the graph the triple is in.
	pub graph: Option<Id>,

	/// Index of the triple in the graph.
	pub index: u32,
}

impl TripleId {
	pub fn new(graph: Option<Id>, index: u32) -> Self {
		Self { graph, index }
	}
}

pub struct ResourceFacts<'a, V, D: Dataset<'a, V>> {
	id: Id,
	graph: D::Graphs,
	current: Option<GraphResourceFacts<'a, V, D>>,
}

struct GraphResourceFacts<'a, V, D: Dataset<'a, V>> {
	id: Option<Id>,
	facts: graph::ResourceFacts<'a, V, D::Graph>,
}

impl<'a, V, D: Dataset<'a, V>> ResourceFacts<'a, V, D> {
	pub fn is_empty(&mut self, vocabulary: &mut V) -> Result<bool, D::Error> {
		loop {
			match self.current.as_mut() {
				Some(current) => {
					if current.facts.is_empty() {
						self.current = None
					} else {
						break Ok(false);
					}
				}
				None => match self.graph.next_with(vocabulary) {
					Some(Err(e)) => break Err(e),
					Some(Ok((g, graph))) => {
						self.current = Some(GraphResourceFacts {
							id: g,
							facts: graph.resource_facts(self.id)?,
						})
					}
					None => break Ok(true),
				},
			}
		}
	}
}

impl<'a, V, D: Dataset<'a, V>> FailibleIteratorWith<V> for ResourceFacts<'a, V, D> {
	type Error = D::Error;
	type Item = (TripleId, Fact);

	fn try_next_with(&mut self, vocabulary: &mut V) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.facts.next_with(vocabulary).transpose()? {
					Some((i, Meta(Signed(sign, t), meta))) => {
						break Ok(Some((
							TripleId::new(current.id, i),
							Meta(Signed(sign, t.into_quad(current.id)), meta),
						)))
					}
					None => self.current = None,
				},
				None => match self.graph.next_with(vocabulary) {
					Some(Err(e)) => break Err(e),
					Some(Ok((g, graph))) => {
						self.current = Some(GraphResourceFacts {
							id: g,
							facts: graph.resource_facts(self.id)?,
						})
					}
					None => break Ok(None),
				},
			}
		}
	}
}

impl<'a, V, D: Dataset<'a, V>> IteratorWith<V> for ResourceFacts<'a, V, D> {
	type Item = Result<(TripleId, Fact), D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.try_next_with(vocabulary).transpose()
	}
}

pub struct Matching<'a, V, D: Dataset<'a, V>> {
	pattern: pattern::Canonical,
	graphs: D::Graphs,
	current: Option<GraphMatching<'a, V, D>>,
	sign: Option<Sign>,
}

struct GraphMatching<'a, V, D: Dataset<'a, V>> {
	id: Option<Id>,
	matching: graph::Matching<'a, V, D::Graph>,
}

impl<'a, V, D: Dataset<'a, V>> Matching<'a, V, D> {
	pub fn into_quads(self) -> MatchingQuads<'a, V, D> {
		MatchingQuads(self)
	}
}

impl<'a, V, D: Dataset<'a, V>> FailibleIteratorWith<V> for Matching<'a, V, D> {
	type Item = (TripleId, Fact);
	type Error = D::Error;

	fn try_next_with(&mut self, vocabulary: &mut V) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.matching.next_with(vocabulary).transpose()? {
					Some((i, Meta(Signed(sign, triple), meta))) => {
						break Ok(Some((
							TripleId::new(current.id, i),
							Meta(Signed(sign, triple.into_quad(current.id)), meta),
						)))
					}
					None => self.current = None,
				},
				None => match self.graphs.next_with(vocabulary) {
					Some(Err(e)) => break Err(e),
					Some(Ok((g, graph))) => {
						self.current = Some(GraphMatching {
							id: g,
							matching: graph.full_pattern_matching(self.pattern, self.sign)?,
						})
					}
					None => break Ok(None),
				},
			}
		}
	}
}

impl<'a, V, D: Dataset<'a, V>> IteratorWith<V> for Matching<'a, V, D> {
	type Item = Result<(TripleId, Fact), D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.try_next_with(vocabulary).transpose()
	}
}

pub struct MatchingQuads<'a, V, D: Dataset<'a, V>>(Matching<'a, V, D>);

impl<'a, V, D: Dataset<'a, V>> FailibleIteratorWith<V> for MatchingQuads<'a, V, D> {
	type Item = Fact;
	type Error = D::Error;

	fn try_next_with(&mut self, vocabulary: &mut V) -> Result<Option<Self::Item>, D::Error> {
		Ok(self.0.try_next_with(vocabulary)?.map(|(_, q)| q))
	}
}

impl<'a, V, D: Dataset<'a, V>> IteratorWith<V> for MatchingQuads<'a, V, D> {
	type Item = Result<Fact, D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.try_next_with(vocabulary).transpose()
	}
}

pub struct Iter<'a, V, D: Dataset<'a, V>> {
	graphs: D::Graphs,
	current: Option<GraphTriples<'a, V, D>>,
}

struct GraphTriples<'a, V, D: Dataset<'a, V>> {
	id: Option<Id>,
	triples: <D::Graph as Graph<'a, V>>::Triples,
}

impl<'a, V, D: Dataset<'a, V>> Iter<'a, V, D> {
	pub fn into_quads(self) -> Quads<'a, V, D> {
		Quads(self)
	}
}

impl<'a, V, D: Dataset<'a, V>> IteratorWith<V> for Iter<'a, V, D> {
	type Item = Result<(Option<Id>, u32, GraphFact), D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.triples.next_with(vocabulary) {
					Some(Err(e)) => break Some(Err(e)),
					Some(Ok((i, triple))) => break Some(Ok((current.id, i, triple))),
					None => self.current = None,
				},
				None => match self.graphs.next_with(vocabulary) {
					Some(Err(e)) => break Some(Err(e)),
					Some(Ok((g, graph))) => {
						self.current = Some(GraphTriples {
							id: g,
							triples: graph.triples(),
						})
					}
					None => break None,
				},
			}
		}
	}
}

pub struct Quads<'a, V, D: Dataset<'a, V>>(Iter<'a, V, D>);

impl<'a, V, D: Dataset<'a, V>> IteratorWith<V> for Quads<'a, V, D> {
	type Item = Result<Fact, D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.0.next_with(vocabulary).map(|f| {
			f.map(|(g, _, Meta(Signed(sign, triple), cause))| {
				Meta(Signed(sign, triple.into_quad(g)), cause)
			})
		})
	}
}
