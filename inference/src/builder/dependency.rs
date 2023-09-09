use std::marker::PhantomData;

use derivative::Derivative;
use hashbrown::HashMap;
use rdf_types::Vocabulary;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::composite,
	Module, Sign, Triple,
};

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Dependencies<V, D> {
	map: HashMap<usize, D>,
	v: PhantomData<V>,
}

impl<V, D> Dependencies<V, D> {
	pub fn iter(&self) -> DependenciesIter<V, D> {
		DependenciesIter {
			map: self.map.iter(),
			v: PhantomData,
		}
	}
}

pub enum Error<E> {
	Contradiction(dataset::Contradiction),
	Module(E),
}

impl<E> From<dataset::Contradiction> for Error<E> {
	fn from(value: dataset::Contradiction) -> Self {
		Self::Contradiction(value)
	}
}

impl<V: Vocabulary, D: Module<V>> Dependencies<V, D> {
	/// Filter the given signed triple by looking for a similar triple in the
	/// dependencies datasets.
	///
	/// Returns `Ok(true)` if the signed triple is not in any dependency
	/// dataset. Returns `Ok(false)` if it is in some dependency dataset.
	/// Returns `Err(Contradiction)` if the opposite triple is found, which
	/// means the input triple cannot be added without causing contradiction.
	pub fn filter(
		&self,
		interpretation: &composite::Interpretation<V>,
		triple: Triple,
		sign: Sign,
	) -> Result<bool, Error<D::Error>> {
		for (&d, dependency) in &self.map {
			for dependency_triple in interpretation.dependency_triples(d, triple) {
				if let Some((_, fact)) = dependency
					.dataset()
					.find_triple(dependency_triple)
					.map_err(Error::Module)?
				{
					if fact.sign() == sign {
						return Ok(false);
					} else {
						return Err(Error::Contradiction(dataset::Contradiction(triple)));
					}
				}
			}
		}

		Ok(true)
	}
}

impl<V: Vocabulary, D: Module<V>> composite::Dependencies<V> for Dependencies<V, D> {
	type Error = D::Error;
	type Dependency = D;
	type Iter<'a> = DependenciesIter<'a, V, D> where Self: 'a;

	fn get(&self, i: usize) -> Option<&Self::Dependency> {
		self.map.get(&i)
	}

	fn iter(&self) -> Self::Iter<'_> {
		self.iter()
	}
}

pub struct DependenciesIter<'a, V, D> {
	map: hashbrown::hash_map::Iter<'a, usize, D>,
	v: PhantomData<V>,
}

impl<'a, V: Vocabulary, D: Module<V>> Iterator for DependenciesIter<'a, V, D> {
	type Item = (usize, &'a D);

	fn next(&mut self) -> Option<Self::Item> {
		self.map.next().map(|(i, d)| (*i, d))
	}
}
