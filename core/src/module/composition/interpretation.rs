use derivative::Derivative;
use rdf_types::Vocabulary;
use smallvec::SmallVec;

use crate::{module::sub_module::IntoLocal, Id, IteratorWith, Module};

use self::resource::SubResource;

use super::{Composition, CompositionSubModule};

pub mod resource;

pub use resource::Resource;

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Interpretation<'a, V, M> {
	composition: &'a Composition<V, M>,
}

impl<'a, V, M> Interpretation<'a, V, M> {
	pub(crate) fn new(composition: &'a Composition<V, M>) -> Self {
		Self { composition }
	}
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::Interpretation<'a, V> for Interpretation<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Error = M::Error;

	type Resource = Resource<'a, V, M>;

	type Resources = Iter<'a, V, M>;

	type Iris = Iris<'a, V, M>;

	type Literals = Literals<'a, V, M>;

	fn resources(&self) -> Result<Self::Resources, Self::Error> {
		let mut sub_resources = SmallVec::with_capacity(self.composition.modules.len());
		for module in &self.composition.modules {
			sub_resources.push(SubResources {
				module,
				iter: module.module().interpretation().resources()?,
				pending: None,
			})
		}

		Ok(Iter {
			composition: self.composition,
			sub_resources,
		})
	}

	fn get(&self, global_id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		let mut selection = SmallVec::new();

		for m in &self.composition.modules {
			if let Some(local_id) = global_id.into_local(m.interface()) {
				if let Some(resource) = m.module().interpretation().get(local_id)? {
					selection.push(SubResource::new(m, resource));
				}
			}
		}

		if selection.is_empty() {
			Ok(None)
		} else {
			Ok(Some(Resource::new(self.composition, selection)))
		}
	}

	fn iris(&self) -> Result<Self::Iris, Self::Error> {
		Ok(Iris {
			composition: self.composition,
			sub_modules: self.composition.modules.iter(),
			current: None
		})
	}

	fn iri_interpretation(
		&self,
		vocabulary: &mut V,
		iri: V::Iri,
	) -> Result<Option<Id>, Self::Error> {
		for sub in self.composition.sub_modules() {
			if let Some(local_id) = sub
				.module()
				.interpretation()
				.iri_interpretation(vocabulary, iri.clone())?
			{
				return self
					.composition
					.import_resource(vocabulary, sub, local_id)
					.map(Some);
			}
		}

		Ok(None)
	}

	fn literals(&self) -> Result<Self::Literals, Self::Error> {
		Ok(Literals {
			composition: self.composition,
			sub_modules: self.composition.modules.iter(),
			current: None
		})
	}

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		literal: <V>::Literal,
	) -> Result<Option<Id>, Self::Error> {
		for module in self.composition.sub_modules() {
			if let Some(local_id) = module
				.module()
				.interpretation()
				.literal_interpretation(vocabulary, literal.clone())?
			{
				return self
					.composition
					.import_resource(vocabulary, module, local_id)
					.map(Some);
			}
		}

		Ok(None)
	}
}

struct SubResources<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	iter: <M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resources,
	pending: Option<NextSubResource<'a, V, M>>,
}

impl<'a, V: Vocabulary, M: Module<V>> SubResources<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	pub fn peek(
		&mut self,
		vocabulary: &mut V,
		composition: &Composition<V, M>,
	) -> Result<Option<&NextSubResource<'a, V, M>>, M::Error> {
		if self.pending.is_none() {
			match self.iter.next_with(vocabulary) {
				Some(Ok((local_id, resource))) => {
					let global_id =
						composition.import_resource(vocabulary, self.module, local_id)?;
					self.pending = Some(NextSubResource {
						global_id,
						resource,
					})
				}
				Some(Err(e)) => return Err(e),
				None => (),
			}
		}

		Ok(self.pending.as_ref())
	}

	pub fn next(
		&mut self,
		vocabulary: &mut V,
	) -> Result<Option<<M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource>, M::Error>
	{
		match self.pending.take() {
			Some(n) => Ok(Some(n.resource)),
			None => match self.iter.next_with(vocabulary) {
				Some(Ok((_, resource))) => Ok(Some(resource)),
				Some(Err(e)) => Err(e),
				None => Ok(None),
			},
		}
	}
}

struct NextSubResource<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	global_id: Id,
	resource: <M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resource,
}

/// Resources iterator.
pub struct Iter<'a, V: 'a + Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	sub_resources: SmallVec<[SubResources<'a, V, M>; 8]>,
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> IteratorWith<V> for Iter<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Item = Result<(Id, Resource<'a, V, M>), M::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		let mut id = None;
		for g in &mut self.sub_resources {
			match g.peek(vocabulary, self.composition) {
				Ok(Some(n)) => {
					if id.is_none() || id.unwrap() > n.global_id {
						id = Some(n.global_id)
					}
				}
				Ok(None) => (),
				Err(e) => return Some(Err(e)),
			}
		}

		match id {
			Some(id) => {
				let mut selection = SmallVec::new();

				for g in &mut self.sub_resources {
					match g.peek(vocabulary, self.composition) {
						Ok(Some(n)) => {
							if n.global_id == id {
								match g.next(vocabulary) {
									Ok(Some(resource)) => {
										selection.push(SubResource::new(g.module, resource));
									}
									Ok(None) => panic!("expected resource"),
									Err(e) => return Some(Err(e)),
								}
							}
						}
						Ok(None) => (),
						Err(e) => return Some(Err(e)),
					}
				}

				Some(Ok((id, Resource::new(self.composition, selection))))
			}
			None => None,
		}
	}
}

pub struct Iris<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	sub_modules: std::slice::Iter<'a, CompositionSubModule<V, M>>,
	current: Option<SubModuleIris<'a, V, M>>
}

struct SubModuleIris<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	iris: <M::Interpretation<'a> as crate::Interpretation<'a, V>>::Iris
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Iris<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone
{
	type Item = Result<(V::Iri, Id), M::Error>;
	
	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(current) => match current.iris.next_with(vocabulary) {
					Some(Ok((iri, local_id))) => {
						match self.composition.import_resource(vocabulary, current.module, local_id) {
							Ok(global_id) => break Some(Ok((iri, global_id))),
							Err(e) => break Some(Err(e))
						}
					}
					Some(Err(e)) => break Some(Err(e)),
					None => self.current = None
				}
				None => match self.sub_modules.next() {
					Some(module) => {
						use crate::Interpretation;
						match module.module().interpretation().iris() {
							Ok(iris) => self.current = Some(SubModuleIris {
								module,
								iris
							}),
							Err(e) => break Some(Err(e))
						}
					},
					None => break None
				}
			}
		}
	}
}

pub struct Literals<'a, V: Vocabulary, M: Module<V>> {
	composition: &'a Composition<V, M>,
	sub_modules: std::slice::Iter<'a, CompositionSubModule<V, M>>,
	current: Option<SubModuleLiterals<'a, V, M>>
}

struct SubModuleLiterals<'a, V: 'a + Vocabulary, M: 'a + Module<V>> {
	module: &'a CompositionSubModule<V, M>,
	literals: <M::Interpretation<'a> as crate::Interpretation<'a, V>>::Literals
}

impl<'a, V: Vocabulary, M: Module<V>> IteratorWith<V> for Literals<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone
{
	type Item = Result<(V::Literal, Id), M::Error>;
	
	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(current) => match current.literals.next_with(vocabulary) {
					Some(Ok((iri, local_id))) => {
						match self.composition.import_resource(vocabulary, current.module, local_id) {
							Ok(global_id) => break Some(Ok((iri, global_id))),
							Err(e) => break Some(Err(e))
						}
					}
					Some(Err(e)) => break Some(Err(e)),
					None => self.current = None
				}
				None => match self.sub_modules.next() {
					Some(module) => {
						use crate::Interpretation;
						match module.module().interpretation().literals() {
							Ok(literals) => self.current = Some(SubModuleLiterals {
								module,
								literals
							}),
							Err(e) => break Some(Err(e))
						}
					},
					None => break None
				}
			}
		}
	}
}