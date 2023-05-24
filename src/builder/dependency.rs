use derivative::Derivative;
use hashbrown::HashMap;
use rdf_types::Vocabulary;

use crate::{
	dataset::{self, Dataset},
	interpretation::{CompositeInterpretation, Interpretation},
	Cause, Sign, Triple,
};

pub struct Dependency<V: Vocabulary, M> {
	interpretation: Interpretation<V>,
	dataset: Dataset<Cause<M>>,
}

impl<V: Vocabulary, M> Dependency<V, M> {
	pub fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}

	pub fn dataset(&self) -> &Dataset<Cause<M>> {
		&self.dataset
	}
}

impl<V: Vocabulary, M> crate::interpretation::composite::Dependency<V> for Dependency<V, M> {
	fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Dependencies<V: Vocabulary, M>(HashMap<usize, Dependency<V, M>>);

impl<V: Vocabulary, M> Dependencies<V, M> {
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
				if let Some((_, _, fact)) = dependency.dataset.find_triple(dependency_triple) {
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

	pub fn iter(&self) -> DependenciesIter<V, M> {
		DependenciesIter(self.0.iter())
	}
}

impl<V: Vocabulary, M> crate::interpretation::composite::Dependencies<V> for Dependencies<V, M> {
	type Dependency = Dependency<V, M>;
	type Iter<'a> = DependenciesIter<'a, V, M> where Self: 'a, Self::Dependency: 'a;

	fn get(&self, i: usize) -> Option<&Self::Dependency> {
		self.0.get(&i)
	}

	fn iter(&self) -> Self::Iter<'_> {
		self.iter()
	}
}

pub struct DependenciesIter<'a, V: Vocabulary, M>(
	hashbrown::hash_map::Iter<'a, usize, Dependency<V, M>>,
);

impl<'a, V: Vocabulary, M> Iterator for DependenciesIter<'a, V, M> {
	type Item = (usize, &'a Dependency<V, M>);

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(i, d)| (*i, d))
	}
}
