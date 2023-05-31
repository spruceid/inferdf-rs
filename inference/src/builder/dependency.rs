use std::marker::PhantomData;

use derivative::Derivative;
use hashbrown::HashMap;
use rdf_types::Vocabulary;

use inferdf_core::{
	dataset::{self, Dataset},
	Sign, Triple, interpretation::composite
};

pub use composite::Dependency;

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Dependencies<V, D> {
	map: HashMap<usize, D>,
	v: PhantomData<V>
}

impl<V, D> Dependencies<V, D> {
	pub fn iter(&self) -> DependenciesIter<V, D> {
		DependenciesIter {
			map: self.map.iter(),
			v: PhantomData
		}
	}
}

impl<V: Vocabulary, D: Dependency<V>> Dependencies<V, D> {
	/// Filter the given signed triple by lookgin for a similar triple in the
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
	) -> Result<bool, dataset::Contradiction> {
		for (&d, dependency) in &self.map {
			for dependency_triple in interpretation.dependency_triples(d, triple) {
				if let Some((_, fact)) = dependency.dataset().find_triple(dependency_triple) {
					if fact.sign() == sign {
						return Ok(false);
					} else {
						return Err(dataset::Contradiction(triple));
					}
				}
			}
		}

		Ok(true)
	}
}

impl<V: Vocabulary, D: Dependency<V>> composite::Dependencies<V> for Dependencies<V, D> {
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
	v: PhantomData<V>
}

impl<'a, V: Vocabulary, D: Dependency<V>> Iterator for DependenciesIter<'a, V, D> {
	type Item = (usize, &'a D);

	fn next(&mut self) -> Option<Self::Item> {
		self.map.next().map(|(i, d)| (*i, d))
	}
}
