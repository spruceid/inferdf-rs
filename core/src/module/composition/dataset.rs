use derivative::Derivative;
use rdf_types::Vocabulary;
use smallvec::SmallVec;

use crate::{
	module::sub_module::IntoLocal,
	Id, IteratorWith, Module,
};

use self::graph::SelectedGraph;

use super::{Composition, CompositionSubModule};

pub mod graph;

pub use graph::Graph;

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Dataset<'a, V, M> {
	composition: &'a Composition<V, M>,
}

impl<'a, V, M> Dataset<'a, V, M> {
	pub(crate) fn new(composition: &'a Composition<V, M>) -> Self {
		Self { composition }
	}
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::Dataset<'a, V> for Dataset<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Error = M::Error;

	type Graph = Graph<'a, V, M>;

	type Graphs = Graphs<'a, V, M>;

	fn graphs(&self) -> Self::Graphs {
		let mut sub_graphs = SmallVec::with_capacity(self.composition.modules.len());
		for m in &self.composition.modules {
			sub_graphs.push(SubGraphs {
				module: m,
				iter: m.module().dataset().graphs(),
				pending: None,
			})
		}

		Graphs {
			composition: self.composition,
			sub_graphs,
		}
	}

	fn graph(&self, global_id: Option<Id>) -> Result<Option<Self::Graph>, Self::Error> {
		let mut selection = SmallVec::new();

		let mut triples_offset = 0;
		for m in &self.composition.modules {
			if let Some(local_id) = global_id.into_local(m.interface()) {
				if let Some(graph) = m.module().dataset().graph(local_id)? {
					use crate::dataset::Graph;
					let graph_len = graph.len();

					selection.push(SelectedGraph::new(m, triples_offset, graph));

					triples_offset += graph_len
				}
			}
		}

		if selection.is_empty() {
			Ok(None)
		} else {
			Ok(Some(Graph::new(self.composition, selection)))
		}
	}
}

struct SubGraphs<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	iter: <M::Dataset<'a> as crate::Dataset<'a, V>>::Graphs,
	pending: Option<NextSubGraph<'a, V, M>>,
}

impl<'a, V: Vocabulary, M: Module<V>> SubGraphs<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	pub fn peek(
		&mut self,
		vocabulary: &mut V,
		composition: &Composition<V, M>,
	) -> Result<Option<&NextSubGraph<'a, V, M>>, M::Error> {
		if self.pending.is_none() {
			match self.iter.next_with(vocabulary) {
				Some(Ok((local_id, graph))) => {
					let global_id = match local_id {
						Some(local_id) => Some(composition.import_resource(
							vocabulary,
							self.module,
							local_id,
						)?),
						None => None,
					};

					self.pending = Some(NextSubGraph { global_id, graph })
				}
				Some(Err(e)) => return Err(e),
				None => (),
			}
		}

		Ok(self.pending.as_ref())
	}

	pub fn next(
		&mut self,
		vocabulary: &mut V,
	) -> Result<Option<<M::Dataset<'a> as crate::Dataset<'a, V>>::Graph>, M::Error> {
		match self.pending.take() {
			Some(n) => Ok(Some(n.graph)),
			None => match self.iter.next_with(vocabulary) {
				Some(Ok((_, graph))) => Ok(Some(graph)),
				Some(Err(e)) => Err(e),
				None => Ok(None),
			},
		}
	}
}

struct NextSubGraph<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	global_id: Option<Id>,
	graph: <M::Dataset<'a> as crate::Dataset<'a, V>>::Graph,
}

pub struct Graphs<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	sub_graphs: SmallVec<[SubGraphs<'a, V, M>; 8]>,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Graphs<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Item = Result<(Option<Id>, Graph<'a, V, M>), M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		let mut id = None;
		for g in &mut self.sub_graphs {
			match g.peek(vocabulary, self.composition) {
				Ok(Some(n)) => {
					if id.is_none() || id.unwrap() > n.global_id {
						id = Some(n.global_id)
					}
				}
				Ok(None) => (),
				Err(e) => return Some(Err(e)),
			}
		}

		match id {
			Some(id) => {
				let mut selection = SmallVec::new();

				let mut triples_offset = 0;
				for g in &mut self.sub_graphs {
					match g.peek(vocabulary, self.composition) {
						Ok(Some(n)) => {
							if n.global_id == id {
								match g.next(vocabulary) {
									Ok(Some(graph)) => {
										use crate::dataset::Graph;

										let graph_len = graph.len();
										selection.push(SelectedGraph::new(
											g.module,
											triples_offset,
											graph,
										));

										triples_offset += graph_len;
									}
									Ok(None) => panic!("expected graph"),
									Err(e) => return Some(Err(e)),
								}
							}
						}
						Ok(None) => (),
						Err(e) => return Some(Err(e)),
					}
				}

				Some(Ok((id, Graph::new(self.composition, selection))))
			}
			None => None,
		}
	}
}
