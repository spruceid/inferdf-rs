use std::io::{Read, Seek};

use derivative::Derivative;
use inferdf_core::Id;
use rdf_types::Vocabulary;

use crate::{binary_search_page, page};

use super::{cache, Module};

pub mod graph;

pub use graph::Graph;

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""))]
pub struct Dataset<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
}

impl<'a, V: Vocabulary, R> Dataset<'a, V, R> {
	pub fn new(module: &'a Module<V, R>) -> Self {
		Self {
			module
		}
	}
}

impl<'a, V: Vocabulary, R: Read + Seek> inferdf_core::Dataset<'a> for Dataset<'a, V, R> {
	type Graph = Graph<'a, V, R>;
	type Graphs = Graphs<'a, V, R>;

	fn graph(&self, id: Option<Id>) -> Option<Self::Graph> {
		match id {
			Some(id) => binary_search_page(
				self.module.sections.graphs,
				self.module.sections.graphs + self.module.header.graph_page_count,
				|p| {
					let page = self.module.get_graph_page(p).unwrap();
					page.find(id).map(|e| {
						let entry = page.get(e).unwrap();
						Graph::new(self.module, entry.description)
					})
				},
			),
			None => Some(Graph::new(self.module, self.module.header.default_graph)),
		}
	}

	fn graphs(&self) -> Self::Graphs {
		Graphs {
			module: self.module,
			default_graph: Some(self.module.header.default_graph),
			page_index: self.module.sections.graphs,
			next_page_index: self.module.sections.graphs + self.module.header.graph_page_count,
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

impl<'a, V: Vocabulary, R: Read + Seek> Iterator for Graphs<'a, V, R> {
	type Item = (Option<Id>, Graph<'a, V, R>);

	fn next(&mut self) -> Option<Self::Item> {
		self.default_graph
			.take()
			.map(|desc| (Option::<Id>::None, Graph::new(self.module, desc)))?;

		while self.page_index < self.next_page_index {
			let iter = self.current.get_or_insert_with(|| {
				cache::Ref::aliasing_map(
					self.module.get_graph_page(self.page_index).unwrap(),
					|p| p.iter(),
				)
			});

			match iter.next() {
				Some(entry) => {
					return Some((Some(entry.id), Graph::new(self.module, entry.description)))
				}
				None => {
					self.current = None;
					self.page_index += 1
				}
			}
		}

		None
	}
}
