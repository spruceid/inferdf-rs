use std::io::{Read, Seek};

use derivative::Derivative;
use inferdf_core::{FailibleIterator, GetOrTryInsertWith, Id};
use rdf_types::Vocabulary;

use crate::{binary_search_page, page};

use super::{cache, Error, Module};

pub mod graph;

pub use graph::Graph;

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""))]
pub struct Dataset<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
}

impl<'a, V: Vocabulary, R> Dataset<'a, V, R> {
	pub fn new(module: &'a Module<V, R>) -> Self {
		Self { module }
	}
}

impl<'a, V: Vocabulary, R: Read + Seek> inferdf_core::Dataset<'a> for Dataset<'a, V, R> {
	type Error = Error;
	type Graph = Graph<'a, V, R>;
	type Graphs = Graphs<'a, V, R>;

	fn graph(&self, id: Option<Id>) -> Result<Option<Self::Graph>, Error> {
		match id {
			Some(id) => binary_search_page(
				self.module.sections.graphs,
				self.module.sections.graphs + self.module.header.named_graph_page_count,
				|p| {
					let page_graph_count = page_graph_count(
						self.module.header.named_graph_count,
						p,
						self.module.sections.graphs_per_page,
					);
					let page = self.module.get_graph_page(p, page_graph_count)?;
					Ok(page.find(id).map(|e| {
						let entry = page.get(e).unwrap();
						Graph::new(self.module, entry.description)
					}))
				},
			),
			None => Ok(Some(Graph::new(
				self.module,
				self.module.header.default_graph,
			))),
		}
	}

	fn graphs(&self) -> Self::Graphs {
		Graphs {
			module: self.module,
			default_graph: Some(self.module.header.default_graph),
			page_index: self.module.sections.graphs,
			next_page_index: self.module.sections.graphs
				+ self.module.header.named_graph_page_count,
			current: None,
		}
	}
}

pub struct Graphs<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	default_graph: Option<page::graphs::Description>,
	page_index: u32,
	next_page_index: u32,
	current: Option<cache::Aliasing<'a, page::graphs::Iter<'a>>>,
}

fn page_graph_count(graph_count: u32, page_index: u32, graphs_per_page: u32) -> u32 {
	std::cmp::min(graph_count - page_index * graphs_per_page, graphs_per_page)
}

impl<'a, V: Vocabulary, R: Read + Seek> FailibleIterator for Graphs<'a, V, R> {
	type Item = (Option<Id>, Graph<'a, V, R>);
	type Error = Error;

	fn try_next(&mut self) -> Result<Option<Self::Item>, Error> {
		match self
			.default_graph
			.take()
			.map(|desc| (Option::<Id>::None, Graph::new(self.module, desc)))
		{
			Some(g) => Ok(Some(g)),
			None => {
				while self.page_index < self.next_page_index {
					let iter = self.current.get_or_try_insert_with::<Error>(|| {
						let page_graph_count = page_graph_count(
							self.module.header.named_graph_count,
							self.page_index,
							self.module.sections.graphs_per_page,
						);
						Ok(cache::Ref::aliasing_map(
							self.module
								.get_graph_page(self.page_index, page_graph_count)?,
							|p| p.iter(),
						))
					})?;

					match iter.next() {
						Some(entry) => {
							return Ok(Some((
								Some(entry.id),
								Graph::new(self.module, entry.description),
							)))
						}
						None => {
							self.current = None;
							self.page_index += 1
						}
					}
				}

				Ok(None)
			}
		}
	}
}

impl<'a, V: Vocabulary, R: Read + Seek> Iterator for Graphs<'a, V, R> {
	type Item = Result<(Option<Id>, Graph<'a, V, R>), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.try_next().transpose()
	}
}
