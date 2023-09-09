use derivative::Derivative;
use rdf_types::Vocabulary;

use crate::{Module, module::composition::{Composition, SubModule}, Id, IteratorWith};

#[derive(Derivative)]
#[derivative(Clone(bound=""))]
pub struct Resource<'a, V, M> {
	module: &'a Composition<V, M>,
	id: Id
}

impl<'a, V, M> Resource<'a, V, M> {
	pub fn new(
		module: &'a Composition<V, M>,
		id: Id
	) -> Self {
		Self {
			module,
			id
		}
	}
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::interpretation::Resource<'a, V> for Resource<'a, V, M> {
	type Error = M::Error;

	type Iris = Iris<'a, V, M>;

	type Literals = Literals<'a, V, M>;

	type DifferentFrom = DifferentFrom<'a, V, M>;

	fn as_iri(&self) -> Self::Iris {
		todo!()
	}

	fn as_literal(&self) -> Self::Literals {
		todo!()
	}

	fn different_from(&self) -> Self::DifferentFrom {
		todo!()
	}

	fn terms(&self) -> crate::interpretation::ResourceTerms<'a, V, Self> {
		todo!()
	}
}

pub struct Iris<'a, V: 'a + Vocabulary, M: Module<V>> {
	sub_modules: std::iter::Enumerate<std::slice::Iter<'a, SubModule<V, M>>>,
	current: Option<(Id, <<M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource as crate::interpretation::Resource<'a, V>>::Iris)>,
	id: Id
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Iris<'a, V, M> {
	type Item = Result<V::Iri, M::Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		todo!()
	}
}

pub struct Literals<'a, V: 'a + Vocabulary, M: Module<V>> {
	sub_modules: std::iter::Enumerate<std::slice::Iter<'a, SubModule<V, M>>>,
	current: Option<(Id, <<M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource as crate::interpretation::Resource<'a, V>>::Literals)>,
	id: Id
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Literals<'a, V, M> {
	type Item = Result<V::Literal, M::Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		todo!()
	}
}

pub struct DifferentFrom<'a, V: 'a + Vocabulary, M: Module<V>> {
	sub_modules: std::iter::Enumerate<std::slice::Iter<'a, SubModule<V, M>>>,
	current: Option<(Id, <<M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource as crate::interpretation::Resource<'a, V>>::DifferentFrom)>,
	id: Id
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for DifferentFrom<'a, V, M> {
	type Item = Id;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}