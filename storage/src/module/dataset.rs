use std::io;

use educe::Educe;
use inferdf_core::Id;
use paged::no_context_mut;
use rdf_types::Vocabulary;

use crate::header;

use super::{Error, Module};

pub mod graph;

pub use graph::Graph;

#[derive(Educe)]
#[educe(Clone)]
pub struct Dataset<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
}

impl<'a, V: Vocabulary, R> Dataset<'a, V, R> {
	pub(crate) fn new(
		module: &'a Module<V, R>,
	) -> Self {
		Self {
			module
		}
	}
}

impl<'a, V: Vocabulary, R: io::Seek + io::Read> inferdf_core::Dataset<'a> for Dataset<'a, V, R> {
	type Error = Error;

	type Graph = Graph<'a, V, R>;

	type Graphs = Graphs<'a, V, R>;

	fn graphs(&self) -> Self::Graphs {
		Graphs {
			module: self.module,
			default_graph: Some(self.module.header.dataset.default_graph),
			r: self.module.reader.iter(
				self.module.header.dataset.named_graphs,
				&self.module.cache.named_graphs,
				self.module.header.heap,
			),
		}
	}

	fn graph(&self, id: Option<Id>) -> Result<Option<Self::Graph>, Self::Error> {
		match id {
			Some(id) => self
				.module
				.reader
				.binary_search_by_key(
					self.module.header.dataset.named_graphs,
					&self.module.cache.named_graphs,
					no_context_mut(),
					self.module.header.heap,
					|graph, _| graph.id.cmp(&id),
				)
				.map(|r| r.map(|g| Graph::new(self.module, g.description))),
			None => Ok(Some(Graph::new(
				self.module,
				self.module.header.dataset.default_graph,
			))),
		}
	}
}

pub struct Graphs<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	default_graph: Option<header::GraphDescription>,
	r: paged::Iter<'a, 'a, R, header::Graph>,
}

impl<'a, V: Vocabulary, R: io::Seek + io::Read> Iterator for Graphs<'a, V, R> {
	type Item = Result<(Option<Id>, Graph<'a, V, R>), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		match self.default_graph.take() {
			Some(desc) => Some(Ok((None, Graph::new(self.module, desc)))),
			None => self
				.r
				.next()
				.map(|r| r.map(|g| (Some(g.id), Graph::new(self.module, g.description)))),
		}
	}
}
