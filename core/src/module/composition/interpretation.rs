use derivative::Derivative;
use rdf_types::Vocabulary;

use crate::{Module, Id, Interpretation as InterpretationTrait};

use super::Composition;

pub mod resource;

pub use resource::Resource;

#[derive(Derivative)]
#[derivative(Clone(bound=""))]
pub struct Interpretation<'a, V, M> {
	module: &'a Composition<V, M>
}

impl<'a, V, M> Interpretation<'a, V, M> {
	pub fn new(module: &'a Composition<V, M>) -> Self {
		Self {
			module
		}
	}
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> crate::Interpretation<'a, V> for Interpretation<'a, V, M>
where
	V::Iri: Clone,
	V::Literal: Clone
{
	type Error = M::Error;

	type Resource = Resource<'a, V, M>;

	type Resources = Iter<'a, V, M>;

	fn resources(&self) -> Result<Self::Resources, Self::Error> {
		todo!()
	}

	fn get(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		todo!()
	}

	fn iri_interpretation(
		&self,
		vocabulary: &mut V,
		iri: V::Iri,
	) -> Result<Option<Id>, Self::Error> {
		for (m, sub) in self.module.sub_modules().iter().enumerate() {
			if let Some(m_id) = sub.module().interpretation().iri_interpretation(vocabulary, iri.clone())? {
				return Ok(Some(self.module.import(m, m_id)))
			}
		}

		Ok(None)
	}

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		literal: <V>::Literal,
	) -> Result<Option<Id>, Self::Error> {
		for (m, module) in self.module.sub_modules().iter().enumerate() {
			if let Some(m_id) = module.module().interpretation().literal_interpretation(vocabulary, literal.clone())? {
				return Ok(Some(self.module.import(m, m_id)))
			}
		}

		Ok(None)
	}
}

/// Resources iterator.
pub struct Iter<'a, V: 'a + Vocabulary, M: Module<V>> {
	module: &'a Composition<V, M>,
	sub_modules: std::iter::Enumerate<std::slice::Iter<'a, M>>,
	current: Option<(usize, <M::Interpretation<'a> as crate::Interpretation<'a, V>>::Resources)>
}

impl<'a, V: 'a + Vocabulary, M: Module<V>> Iterator for Iter<'a, V, M> {
	type Item = Result<(Id, Resource<'a, V, M>), M::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some((m, resources)) => match resources.next() {
					Some(Ok((m_id, _))) => {
						let id = self.module.import(*m, m_id);
						break Some(Ok((id, Resource::new(self.module, id))))
					}
					Some(Err(e)) => break Some(Err(e)),
					None => self.current = None,
				}
				None => match self.sub_modules.next() {
					Some((m, sub_module)) => match sub_module.interpretation().resources() {
						Ok(resources) => self.current = Some((m, resources)),
						Err(e) => break Some(Err(e))
					}
					None => break None
				}
			}
		}
	}
}