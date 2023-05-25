use locspan::Meta;

use crate::{pattern, Id, Quad, Sign, Signed, Triple};

pub mod graph;
pub mod standard;

pub use graph::Graph;
pub use standard::Standard;

pub struct Contradiction(pub Triple);

pub type Fact<M> = Meta<Signed<Quad>, M>;

/// RDF dataset.
pub trait Dataset {
	type Metadata;

	type Graph<'a>: Graph<'a, Metadata = Self::Metadata>
	where
		Self: 'a;

	type Graphs<'a>: Iterator<Item = (Option<Id>, Self::Graph<'a>)>
	where
		Self: 'a;

	fn graphs(&self) -> Self::Graphs<'_>;

	fn graph(&self, id: Option<Id>) -> Option<Self::Graph<'_>>;

	fn resource_facts(&self, id: Id) -> ResourceFacts<Self> {
		ResourceFacts {
			id,
			graph: self.graphs(),
			current: None,
		}
	}

	fn find_triple(
		&self,
		triple: Triple,
	) -> Option<(TripleId, Meta<Signed<Quad>, &Self::Metadata>)> {
		for (g, graph) in self.graphs() {
			if let Some((i, Meta(Signed(sign, t), meta))) = graph.find_triple(triple) {
				return Some((
					TripleId::new(g, i),
					Meta(Signed(sign, t.into_quad(g)), meta),
				));
			}
		}

		None
	}

	fn full_pattern_matching(
		&self,
		pattern: pattern::Canonical,
		sign: Option<Sign>,
	) -> Matching<Self> {
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
	) -> Matching<Self> {
		self.full_pattern_matching(pattern, Some(sign))
	}

	fn unsigned_pattern_matching(&self, pattern: pattern::Canonical) -> Matching<Self> {
		self.full_pattern_matching(pattern, None)
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

pub struct ResourceFacts<'a, D: 'a + ?Sized + Dataset> {
	id: Id,
	graph: D::Graphs<'a>,
	current: Option<(Option<Id>, graph::ResourceFacts<'a, D::Graph<'a>>)>,
}

impl<'a, D: ?Sized + Dataset> ResourceFacts<'a, D> {
	pub fn is_empty(&mut self) -> bool {
		loop {
			match self.current.as_mut() {
				Some((_, current)) => {
					if current.is_empty() {
						self.current = None
					} else {
						break false;
					}
				}
				None => match self.graph.next() {
					Some((g, graph)) => self.current = Some((g, graph.resource_facts(self.id))),
					None => break true,
				},
			}
		}
	}
}

impl<'a, D: 'a + ?Sized + Dataset> Iterator for ResourceFacts<'a, D> {
	type Item = (TripleId, Meta<Signed<Quad>, &'a D::Metadata>);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some((g, current)) => match current.next() {
					Some((i, Meta(Signed(sign, t), meta))) => {
						break Some((
							TripleId::new(*g, i),
							Meta(Signed(sign, t.into_quad(*g)), meta),
						))
					}
					None => self.current = None,
				},
				None => match self.graph.next() {
					Some((g, graph)) => self.current = Some((g, graph.resource_facts(self.id))),
					None => break None,
				},
			}
		}
	}
}

pub struct Matching<'a, D: 'a + ?Sized + Dataset> {
	pattern: pattern::Canonical,
	graphs: D::Graphs<'a>,
	current: Option<(Option<Id>, graph::Matching<'a, D::Graph<'a>>)>,
	sign: Option<Sign>,
}

impl<'a, D: 'a + Dataset> Matching<'a, D> {
	pub fn into_quads(self) -> MatchingQuads<'a, D> {
		MatchingQuads(self)
	}
}

impl<'a, D: 'a + Dataset> Iterator for Matching<'a, D> {
	type Item = (TripleId, Meta<Signed<Quad>, &'a D::Metadata>);

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some((g, m)) => match m.next() {
					Some((i, Meta(Signed(sign, triple), meta))) => {
						break Some((
							TripleId::new(*g, i),
							Meta(Signed(sign, triple.into_quad(*g)), meta),
						))
					}
					None => self.current = None,
				},
				None => match self.graphs.next() {
					Some((g, graph)) => {
						self.current =
							Some((g, graph.full_pattern_matching(self.pattern, self.sign)))
					}
					None => break None,
				},
			}
		}
	}
}

pub struct MatchingQuads<'a, D: ?Sized + Dataset>(Matching<'a, D>);

impl<'a, D: Dataset> Iterator for MatchingQuads<'a, D> {
	type Item = Meta<Signed<Quad>, &'a D::Metadata>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(_, q)| q)
	}
}
