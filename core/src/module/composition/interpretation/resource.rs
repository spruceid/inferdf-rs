use educe::Educe;
use rdf_types::Vocabulary;
use smallvec::SmallVec;

use crate::{module::composition::{Composition, CompositionSubModule}, Id, IteratorWith, Module};

#[derive(Educe)]
#[educe(Clone)]
pub(crate) struct SubResource<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	resource: <M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource,
}

impl<'a, V: 'a + Vocabulary, M: 'a + Module<V>> SubResource<'a, V, M> {
	pub fn new(
		module: &'a CompositionSubModule<V, M>,
		resource: <M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource,
	) -> Self {
		Self {
			module,
			resource,
		}
	}
}

#[derive(Educe)]
#[educe(Clone)]
pub struct Resource<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	selection: SmallVec<[SubResource<'a, V, M>; 8]>,
}

impl<'a, V: Vocabulary, M: Module<V>> Resource<'a, V, M> {
	pub(crate) fn new(
		composition: &'a Composition<V, M>,
		selection: SmallVec<[SubResource<'a, V, M>; 8]>,
	) -> Self {
		Self {
			composition,
			selection,
		}
	}
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::interpretation::Resource<'a, V>
	for Resource<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
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
}

pub struct Iris<'a, V: 'a + Vocabulary, M: Module<V>> {
	sub_modules: smallvec::IntoIter<[SubResource<'a, V, M>; 8]>,
	current: Option<<<M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource as crate::interpretation::Resource<'a, V>>::Iris>,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Iris<'a, V, M> {
	type Item = Result<V::Iri, M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		use crate::interpretation::Resource;
		loop {
			match &mut self.current {
				Some(iter) => match iter.next_with(vocabulary) {
					Some(iri) => break Some(iri),
					None => self.current = None,
				},
				None => match self.sub_modules.next() {
					Some(s) => self.current = Some(s.resource.as_iri()),
					None => break None,
				},
			}
		}
	}
}

pub struct Literals<'a, V: 'a + Vocabulary, M: Module<V>> {
	sub_modules: smallvec::IntoIter<[SubResource<'a, V, M>; 8]>,
	current: Option<<<M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource as crate::interpretation::Resource<'a, V>>::Literals>,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Literals<'a, V, M> {
	type Item = Result<V::Literal, M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		use crate::interpretation::Resource;
		loop {
			match &mut self.current {
				Some(iter) => match iter.next_with(vocabulary) {
					Some(literal) => break Some(literal),
					None => self.current = None,
				},
				None => match self.sub_modules.next() {
					Some(s) => self.current = Some(s.resource.as_literal()),
					None => break None,
				},
			}
		}
	}
}

struct SubResourceDifferentFrom<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	iter: <<M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource as crate::interpretation::Resource<'a, V>>::DifferentFrom
}

pub struct DifferentFrom<'a, V: 'a + Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	sub_modules: smallvec::IntoIter<[SubResource<'a, V, M>; 8]>,
	current: Option<SubResourceDifferentFrom<'a, V, M>>,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for DifferentFrom<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Item = Result<Id, M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		use crate::interpretation::Resource;
		loop {
			match &mut self.current {
				Some(r) => match r.iter.next_with(vocabulary) {
					Some(Ok(local_id)) => {
						break Some(self.composition.import_resource(
							vocabulary,
							r.module,
							local_id,
						))
					}
					Some(Err(e)) => break Some(Err(e)),
					None => self.current = None,
				},
				None => match self.sub_modules.next() {
					Some(s) => {
						self.current = Some(SubResourceDifferentFrom {
							module: s.module,
							iter: s.resource.different_from(),
						})
					}
					None => break None,
				},
			}
		}
	}
}
