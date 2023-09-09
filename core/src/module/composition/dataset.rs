use std::marker::PhantomData;

use derivative::Derivative;
use rdf_types::Vocabulary;

use crate::{Module, Id};

use super::Composition;

pub mod graph;

pub use graph::Graph;

#[derive(Derivative)]
#[derivative(Clone(bound=""))]
pub struct Dataset<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::Dataset<'a> for Dataset<'a, V, M> {
	type Error = M::Error;

	type Graph = Graph<'a, V, M>;

	type Graphs = Graphs<'a, V, M>;

	fn graphs(&self) -> Self::Graphs {
		todo!()
	}

	fn graph(&self, id: Option<Id>) -> Result<Option<Self::Graph>, Self::Error> {
		todo!()
	}
}

pub struct Graphs<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for Graphs<'a, V, M> {
	type Item = Result<(Option<Id>, Graph<'a, V, M>), M::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}