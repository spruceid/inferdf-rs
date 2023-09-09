use std::marker::PhantomData;

use derivative::Derivative;
use rdf_types::Vocabulary;

use crate::{module::Composition, Module};

#[derive(Derivative)]
#[derivative(Clone(bound=""))]
pub struct Resource<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::dataset::graph::Resource<'a> for Resource<'a, V, M> {
	type AsSubject = AsSubject<'a, V, M>;
	type AsPredicate = AsPredicate<'a, V, M>;
	type AsObject = AsObject<'a, V, M>;

	fn as_subject(&self) -> Self::AsSubject {
		todo!()
	}

	fn as_predicate(&self) -> Self::AsPredicate {
		todo!()
	}

	fn as_object(&self) -> Self::AsObject {
		todo!()
	}
}

pub struct AsSubject<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for AsSubject<'a, V, M> {
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}

pub struct AsPredicate<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for AsPredicate<'a, V, M> {
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}

pub struct AsObject<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for AsObject<'a, V, M> {
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}