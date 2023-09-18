use educe::Educe;
use rdf_types::Vocabulary;
use smallvec::SmallVec;

use crate::{Dataset, Module};

#[derive(Educe)]
#[educe(Clone)]
struct SubResource<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	triples_offset: u32,
	resource: <<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resource,
}

struct SubResourceAsSubject<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	triples_offset: u32,
	iter: <<<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resource as crate::dataset::graph::Resource<'a>>::AsSubject
}

struct SubResourceAsPredicate<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	triples_offset: u32,
	iter: <<<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resource as crate::dataset::graph::Resource<'a>>::AsPredicate
}

struct SubResourceAsObject<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	triples_offset: u32,
	iter: <<<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resource as crate::dataset::graph::Resource<'a>>::AsObject
}

#[derive(Educe)]
#[educe(Default, Clone)]
pub struct Resource<'a, V: Vocabulary, M: Module<V>> {
	selection: SmallVec<[SubResource<'a, V, M>; 8]>,
}

impl<'a, V: Vocabulary, M: Module<V>> Resource<'a, V, M> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert(
		&mut self,
		triples_offset: u32,
		resource: <<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resource,
	) {
		self.selection.push(SubResource {
			triples_offset,
			resource,
		})
	}

	pub fn is_empty(&self) -> bool {
		self.selection.is_empty()
	}
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::dataset::graph::Resource<'a>
	for Resource<'a, V, M>
{
	type AsSubject = AsSubject<'a, V, M>;
	type AsPredicate = AsPredicate<'a, V, M>;
	type AsObject = AsObject<'a, V, M>;

	fn as_subject(&self) -> Self::AsSubject {
		AsSubject {
			selection: self.selection.clone().into_iter(),
			current: None,
		}
	}

	fn as_predicate(&self) -> Self::AsPredicate {
		AsPredicate {
			selection: self.selection.clone().into_iter(),
			current: None,
		}
	}

	fn as_object(&self) -> Self::AsObject {
		AsObject {
			selection: self.selection.clone().into_iter(),
			current: None,
		}
	}
}

pub struct AsSubject<'a, V: Vocabulary, M: Module<V>> {
	selection: smallvec::IntoIter<[SubResource<'a, V, M>; 8]>,
	current: Option<SubResourceAsSubject<'a, V, M>>,
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for AsSubject<'a, V, M> {
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.iter.next() {
					Some(local_triple_id) => break Some(current.triples_offset + local_triple_id),
					None => self.current = None,
				},
				None => match self.selection.next() {
					Some(r) => {
						use crate::dataset::graph::Resource;
						self.current = Some(SubResourceAsSubject {
							triples_offset: r.triples_offset,
							iter: r.resource.as_subject(),
						})
					}
					None => break None,
				},
			}
		}
	}
}

pub struct AsPredicate<'a, V: Vocabulary, M: Module<V>> {
	selection: smallvec::IntoIter<[SubResource<'a, V, M>; 8]>,
	current: Option<SubResourceAsPredicate<'a, V, M>>,
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for AsPredicate<'a, V, M> {
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.iter.next() {
					Some(local_triple_id) => break Some(current.triples_offset + local_triple_id),
					None => self.current = None,
				},
				None => match self.selection.next() {
					Some(r) => {
						use crate::dataset::graph::Resource;
						self.current = Some(SubResourceAsPredicate {
							triples_offset: r.triples_offset,
							iter: r.resource.as_predicate(),
						})
					}
					None => break None,
				},
			}
		}
	}
}

pub struct AsObject<'a, V: Vocabulary, M: Module<V>> {
	selection: smallvec::IntoIter<[SubResource<'a, V, M>; 8]>,
	current: Option<SubResourceAsObject<'a, V, M>>,
}

impl<'a, V: Vocabulary, M: Module<V>> Iterator for AsObject<'a, V, M> {
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.iter.next() {
					Some(local_triple_id) => break Some(current.triples_offset + local_triple_id),
					None => self.current = None,
				},
				None => match self.selection.next() {
					Some(r) => {
						use crate::dataset::graph::Resource;
						self.current = Some(SubResourceAsObject {
							triples_offset: r.triples_offset,
							iter: r.resource.as_object(),
						})
					}
					None => break None,
				},
			}
		}
	}
}
