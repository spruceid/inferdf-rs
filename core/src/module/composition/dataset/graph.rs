use std::marker::PhantomData;

use derivative::Derivative;
use rdf_types::Vocabulary;

use crate::{module::Composition, Module, GraphFact, Id};

pub mod resource;

pub use resource::Resource;

#[derive(Derivative)]
#[derivative(Clone(bound=""))]
pub struct Graph<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::dataset::Graph<'a> for Graph<'a, V, M> {
	type Error = M::Error;

	type Resource = Resource<'a, V, M>;

	type Resources = Resources<'a, V, M>;

	type Triples = Triples<'a, V, M>;

	fn get_resource(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		todo!()
	}

	fn resources(&self) -> Self::Resources {
		todo!()
	}

	fn get_triple(&self, index: u32) -> Result<Option<GraphFact>, Self::Error> {
		todo!()
	}

	fn triples(&self) -> Self::Triples {
		todo!()
	}
}

pub struct Resources<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for Resources<'a, V, M> {
	type Item = Result<(Id, Resource<'a, V, M>), M::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}

pub struct Triples<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for Triples<'a, V, M> {
	type Item = Result<(u32, GraphFact), M::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}