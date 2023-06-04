use locspan::Meta;

use crate::{pattern, Fact, FailibleIterator, GraphFact, Id, Sign, Signed, Triple};

pub mod graph;
pub mod local;

pub use graph::Graph;
pub use local::LocalDataset;

#[derive(Debug, thiserror::Error)]
#[error("statement contradiction")]
pub struct Contradiction(pub Triple);

/// RDF dataset.
pub trait Dataset<'a>: Clone {
	type Error;

	type Graph: Graph<'a, Error = Self::Error>;

	type Graphs: 'a + Iterator<Item = Result<(Option<Id>, Self::Graph), Self::Error>>;

	fn graphs(&self) -> Self::Graphs;

	fn graph(&self, id: Option<Id>) -> Result<Option<Self::Graph>, Self::Error>;

	fn resource_facts(&self, id: Id) -> ResourceFacts<'a, Self> {
		ResourceFacts {
			id,
			graph: self.graphs(),
			current: None,
		}
	}

	fn find_triple(&self, triple: Triple) -> Result<Option<(TripleId, Fact)>, Self::Error> {
		for g in self.graphs() {
			let (g, graph) = g?;
			if let Some((i, Meta(Signed(sign, t), meta))) = graph.find_triple(triple)? {
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
	) -> Matching<'a, Self> {
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
	) -> Matching<'a, Self> {
		self.full_pattern_matching(pattern, Some(sign))
	}

	fn unsigned_pattern_matching(&self, pattern: pattern::Canonical) -> Matching<'a, Self> {
		self.full_pattern_matching(pattern, None)
	}

	fn iter(&self) -> Iter<'a, Self> {
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

pub struct ResourceFacts<'a, D: Dataset<'a>> {
	id: Id,
	graph: D::Graphs,
	current: Option<(Option<Id>, graph::ResourceFacts<'a, D::Graph>)>,
}

impl<'a, D: Dataset<'a>> ResourceFacts<'a, D> {
	pub fn is_empty(&mut self) -> Result<bool, D::Error> {
		loop {
			match self.current.as_mut() {
				Some((_, current)) => {
					if current.is_empty() {
						self.current = None
					} else {
						break Ok(false);
					}
				}
				None => match self.graph.next() {
					Some(Err(e)) => break Err(e),
					Some(Ok((g, graph))) => {
						self.current = Some((g, graph.resource_facts(self.id)?))
					}
					None => break Ok(true),
				},
			}
		}
	}
}

impl<'a, D: Dataset<'a>> FailibleIterator for ResourceFacts<'a, D> {
	type Error = D::Error;
	type Item = (TripleId, Fact);

	fn try_next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			match self.current.as_mut() {
				Some((g, current)) => match current.next().transpose()? {
					Some((i, Meta(Signed(sign, t), meta))) => {
						break Ok(Some((
							TripleId::new(*g, i),
							Meta(Signed(sign, t.into_quad(*g)), meta),
						)))
					}
					None => self.current = None,
				},
				None => match self.graph.next() {
					Some(Err(e)) => break Err(e),
					Some(Ok((g, graph))) => {
						self.current = Some((g, graph.resource_facts(self.id)?))
					}
					None => break Ok(None),
				},
			}
		}
	}
}

impl<'a, D: Dataset<'a>> Iterator for ResourceFacts<'a, D> {
	type Item = Result<(TripleId, Fact), D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.try_next().transpose()
	}
}

pub struct Matching<'a, D: Dataset<'a>> {
	pattern: pattern::Canonical,
	graphs: D::Graphs,
	current: Option<(Option<Id>, graph::Matching<'a, D::Graph>)>,
	sign: Option<Sign>,
}

impl<'a, D: Dataset<'a>> Matching<'a, D> {
	pub fn into_quads(self) -> MatchingQuads<'a, D> {
		MatchingQuads(self)
	}
}

impl<'a, D: Dataset<'a>> FailibleIterator for Matching<'a, D> {
	type Item = (TripleId, Fact);
	type Error = D::Error;

	fn try_next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			match self.current.as_mut() {
				Some((g, m)) => match m.next().transpose()? {
					Some((i, Meta(Signed(sign, triple), meta))) => {
						break Ok(Some((
							TripleId::new(*g, i),
							Meta(Signed(sign, triple.into_quad(*g)), meta),
						)))
					}
					None => self.current = None,
				},
				None => match self.graphs.next() {
					Some(Err(e)) => break Err(e),
					Some(Ok((g, graph))) => {
						self.current =
							Some((g, graph.full_pattern_matching(self.pattern, self.sign)?))
					}
					None => break Ok(None),
				},
			}
		}
	}
}

impl<'a, D: Dataset<'a>> Iterator for Matching<'a, D> {
	type Item = Result<(TripleId, Fact), D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.try_next().transpose()
	}
}

pub struct MatchingQuads<'a, D: Dataset<'a>>(Matching<'a, D>);

impl<'a, D: Dataset<'a>> FailibleIterator for MatchingQuads<'a, D> {
	type Item = Fact;
	type Error = D::Error;

	fn try_next(&mut self) -> Result<Option<Self::Item>, D::Error> {
		Ok(self.0.try_next()?.map(|(_, q)| q))
	}
}

impl<'a, D: Dataset<'a>> Iterator for MatchingQuads<'a, D> {
	type Item = Result<Fact, D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.try_next().transpose()
	}
}

pub struct Iter<'a, D: Dataset<'a>> {
	graphs: D::Graphs,
	current: Option<(Option<Id>, <D::Graph as Graph<'a>>::Triples)>,
}

impl<'a, D: Dataset<'a>> Iter<'a, D> {
	pub fn into_quads(self) -> Quads<'a, D> {
		Quads(self)
	}
}

impl<'a, D: Dataset<'a>> Iterator for Iter<'a, D> {
	type Item = Result<(Option<Id>, u32, GraphFact), D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some((g, m)) => match m.next() {
					Some(Err(e)) => break Some(Err(e)),
					Some(Ok((i, triple))) => break Some(Ok((*g, i, triple))),
					None => self.current = None,
				},
				None => match self.graphs.next() {
					Some(Err(e)) => break Some(Err(e)),
					Some(Ok((g, graph))) => self.current = Some((g, graph.triples())),
					None => break None,
				},
			}
		}
	}
}

pub struct Quads<'a, D: Dataset<'a>>(Iter<'a, D>);

impl<'a, D: Dataset<'a>> Iterator for Quads<'a, D> {
	type Item = Result<Fact, D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|f| {
			f.map(|(g, _, Meta(Signed(sign, triple), cause))| {
				Meta(Signed(sign, triple.into_quad(g)), cause)
			})
		})
	}
}
