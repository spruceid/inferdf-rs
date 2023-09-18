use educe::Educe;
use locspan::Meta;
use rdf_types::Vocabulary;
use smallvec::SmallVec;

use crate::{
	module::{
		sub_module::{IntoLocal, TryIntoGlobal},
		Composition, composition::CompositionSubModule,
	},
	Dataset, GraphFact, Id, IteratorWith, Module,
};

pub mod resource;

pub use resource::Resource;

#[derive(Educe)]
#[educe(Clone)]
pub(crate) struct SelectedGraph<'a, V: Vocabulary, M: Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	triples_offset: u32,
	graph: <M::Dataset<'a> as Dataset<'a, V>>::Graph,
}

impl<'a, V: Vocabulary, M: Module<V>> SelectedGraph<'a, V, M> {
	pub fn new(
		module: &'a CompositionSubModule<V, M>,
		triples_offset: u32,
		graph: <M::Dataset<'a> as Dataset<'a, V>>::Graph,
	) -> Self {
		Self {
			module,
			triples_offset,
			graph,
		}
	}
}

#[derive(Educe)]
#[educe(Clone)]
pub struct Graph<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	selection: SmallVec<[SelectedGraph<'a, V, M>; 8]>,
}

impl<'a, V: Vocabulary, M: Module<V>> Graph<'a, V, M> {
	pub(crate) fn new(
		composition: &'a Composition<V, M>,
		selection: SmallVec<[SelectedGraph<'a, V, M>; 8]>,
	) -> Self {
		Self {
			composition,
			selection,
		}
	}
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::dataset::Graph<'a, V> for Graph<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Error = M::Error;

	type Resource = Resource<'a, V, M>;

	type Resources = Resources<'a, V, M>;

	type Triples = Triples<'a, V, M>;

	fn get_resource(&self, global_id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		let mut result = Resource::new();

		for g in &self.selection {
			if let Some(local_id) = global_id.into_local(g.module.interface()) {
				if let Some(r) = g.graph.get_resource(local_id)? {
					result.insert(g.triples_offset, r)
				}
			}
		}

		if result.is_empty() {
			Ok(None)
		} else {
			Ok(Some(result))
		}
	}

	fn resources(&self) -> Self::Resources {
		let mut result = Resources {
			composition: self.composition,
			selection: SmallVec::with_capacity(self.selection.len()),
		};

		for s in &self.selection {
			result.selection.push(SubResources {
				module: s.module,
				triples_offset: s.triples_offset,
				iter: s.graph.resources(),
				pending: None,
			})
		}

		result
	}

	fn len(&self) -> u32 {
		let mut len = 0;

		for s in &self.selection {
			len += s.graph.len()
		}

		len
	}

	fn get_triple(&self, vocabulary: &mut V, index: u32) -> Result<Option<GraphFact>, Self::Error> {
		for s in &self.selection {
			if s.triples_offset <= index && s.triples_offset + s.graph.len() < index {
				let Meta(triple, cause) = s
					.graph
					.get_triple(vocabulary, index - s.triples_offset)?
					.unwrap();

				let triple = triple.try_into_global(s.module.interface(), |local_id| {
					self.composition
						.import_resource(vocabulary, s.module, local_id)
				})?;

				// TODO map cause.

				return Ok(Some(Meta(triple, cause)));
			}
		}

		Ok(None)
	}

	fn triples(&self) -> Self::Triples {
		Triples {
			composition: self.composition,
			selection: self.selection.clone().into_iter(),
			current: None,
		}
	}
}

struct SubResources<'a, V: Vocabulary, M: Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	triples_offset: u32,
	iter: <<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resources,
	pending: Option<SubResourcesNext<'a, V, M>>,
}

impl<'a, V: 'a + Vocabulary, M: 'a + Module<V>> SubResources<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	pub fn peek(
		&mut self,
		vocabulary: &mut V,
		composition: &Composition<V, M>,
	) -> Result<Option<&SubResourcesNext<'a, V, M>>, M::Error> {
		match self.pending {
			Some(ref r) => Ok(Some(r)),
			None => match self.iter.next_with(vocabulary) {
				Some(Ok((local_id, r))) => {
					let global_id = composition.import_resource(
						vocabulary,
						self.module,
						local_id,
					)?;
					self.pending = Some(SubResourcesNext {
						global_id,
						resource: r,
					});

					Ok(self.pending.as_ref())
				}
				Some(Err(e)) => Err(e),
				None => Ok(None),
			},
		}
	}

	pub fn next(
		&mut self,
		vocabulary: &mut V,
	) -> Result<Option<DatasetResource<'a, V, M>>, M::Error> {
		match self.pending.take() {
			Some(n) => Ok(Some(n.resource)),
			None => match self.iter.next_with(vocabulary) {
				Some(Ok((_, r))) => Ok(Some(r)),
				Some(Err(e)) => Err(e),
				None => Ok(None),
			},
		}
	}
}

pub type DatasetResource<'a, V, M> = <<<M as Module<V>>::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resource;

struct SubResourcesNext<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	global_id: Id,
	resource: <<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Resource,
}

pub struct Resources<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	selection: SmallVec<[SubResources<'a, V, M>; 8]>,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Resources<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Item = Result<(Id, Resource<'a, V, M>), M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		let mut min = None;

		for r in &self.selection {
			if let Some(n) = &r.pending {
				if min.is_none() || min.unwrap() > n.global_id {
					min = Some(n.global_id)
				}
			}
		}

		match min {
			Some(id) => {
				let mut result = Resource::new();

				for r in &mut self.selection {
					match r.peek(vocabulary, self.composition) {
						Ok(Some(n)) => {
							if n.global_id == id {
								match r.next(vocabulary) {
									Ok(resource) => {
										result.insert(r.triples_offset, resource.unwrap());
									}
									Err(e) => return Some(Err(e)),
								}
							}
						}
						Ok(None) => (),
						Err(e) => return Some(Err(e)),
					}
				}

				Some(Ok((id, result)))
			}
			None => None,
		}
	}
}

struct SelectedGraphTriples<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	triples_offset: u32,
	iter: <<M::Dataset<'a> as Dataset<'a, V>>::Graph as crate::dataset::Graph<'a, V>>::Triples,
}

pub struct Triples<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	selection: smallvec::IntoIter<[SelectedGraph<'a, V, M>; 8]>,
	current: Option<SelectedGraphTriples<'a, V, M>>,
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Triples<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Item = Result<(u32, GraphFact), M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some(g) => match g.iter.next_with(vocabulary) {
					Some(Ok((local_triple_id, Meta(triple, cause)))) => {
						let global_triple_id = g.triples_offset + local_triple_id;

						let triple = triple.try_into_global(g.module.interface(), |local_id| {
							self.composition.import_resource(
								vocabulary,
								g.module,
								local_id,
							)
						});

						break match triple {
							Ok(triple) => {
								// TODO map cause.
								Some(Ok((global_triple_id, Meta(triple, cause))))
							}
							Err(e) => Some(Err(e)),
						};
					}
					Some(Err(e)) => break Some(Err(e)),
					None => self.current = None,
				},
				None => match self.selection.next() {
					Some(g) => {
						use crate::dataset::Graph;
						self.current = Some(SelectedGraphTriples {
							module: g.module,
							triples_offset: g.triples_offset,
							iter: g.graph.triples(),
						})
					}
					None => break None,
				},
			}
		}
	}
}
