use derivative::Derivative;
use hashbrown::HashMap;
use rdf_types::Vocabulary;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::{self, CompositeInterpretation, Interpretation},
	Sign, Triple,
};

pub struct Dependency<V: Vocabulary, D> {
	interpretation: Interpretation<V>,
	dataset: D,
}

impl<V: Vocabulary, D> Dependency<V, D> {
	pub fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}

	pub fn dataset(&self) -> &D {
		&self.dataset
	}
}

impl<V: Vocabulary, M> interpretation::composite::Dependency<V> for Dependency<V, M> {
	fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Dependencies<V: Vocabulary, D>(HashMap<usize, Dependency<V, D>>);

impl<V: Vocabulary, D> Dependencies<V, D> {
	pub fn iter(&self) -> DependenciesIter<V, D> {
		DependenciesIter(self.0.iter())
	}
}

impl<V: Vocabulary, D: Dataset> Dependencies<V, D> {
	/// Filter the given signed triple by lookgin for a similar triple in the
	/// dependencies datasets.
	///
	/// Returns `Ok(true)` if the signed triple is not in any dependency
	/// dataset. Returns `Ok(false)` if it is in some dependency dataset.
	/// Returns `Err(Contradiction)` if the opposite triple is found, which
	/// means the input triple cannot be added without causing contradiction.
	pub fn filter(
		&self,
		interpretation: &CompositeInterpretation<V>,
		triple: Triple,
		sign: Sign,
	) -> Result<bool, dataset::Contradiction> {
		for (&d, dependency) in &self.0 {
			for dependency_triple in interpretation.dependency_triples(d, triple) {
				if let Some((_, fact)) = dependency.dataset.find_triple(dependency_triple) {
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

impl<V: Vocabulary, D> interpretation::composite::Dependencies<V> for Dependencies<V, D> {
	type Dependency = Dependency<V, D>;
	type Iter<'a> = DependenciesIter<'a, V, D> where Self: 'a, Self::Dependency: 'a;

	fn get(&self, i: usize) -> Option<&Self::Dependency> {
		self.0.get(&i)
	}

	fn iter(&self) -> Self::Iter<'_> {
		self.iter()
	}
}

pub struct DependenciesIter<'a, V: Vocabulary, D>(
	hashbrown::hash_map::Iter<'a, usize, Dependency<V, D>>,
);

impl<'a, V: Vocabulary, D> Iterator for DependenciesIter<'a, V, D> {
	type Item = (usize, &'a Dependency<V, D>);

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(i, d)| (*i, d))
	}
}
