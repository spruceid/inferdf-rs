use std::hash::Hash;

use rdf_types::Vocabulary;

use crate::{class::classification, dataset, interpretation};

/// Local module.
pub struct LocalModule<V: Vocabulary> {
	pub interpretation: interpretation::Local<V>,
	pub dataset: dataset::LocalDataset,
	pub classification: classification::Local,
}

impl<V: Vocabulary> LocalModule<V> {
	pub fn new(
		interpretation: interpretation::Local<V>,
		dataset: dataset::LocalDataset,
		classification: classification::Local,
	) -> Self {
		Self {
			interpretation,
			dataset,
			classification,
		}
	}
}

impl<V: Vocabulary> crate::Module<V> for LocalModule<V>
where
	V::Iri: Clone + Eq + Hash,
	V::Literal: Clone + Eq + Hash,
{
	type Error = std::convert::Infallible;

	type Interpretation<'a> = &'a interpretation::Local<V> where V: 'a;
	type Dataset<'a> = &'a dataset::LocalDataset where V: 'a;
	type Classification<'a> = &'a classification::Local where V: 'a;

	fn interpretation<'a>(&'a self) -> Self::Interpretation<'a>
	where
		V: 'a,
	{
		&self.interpretation
	}

	fn dataset<'a>(&'a self) -> Self::Dataset<'a>
	where
		V: 'a,
	{
		&self.dataset
	}

	fn classification<'a>(&'a self) -> Self::Classification<'a>
	where
		V: 'a,
	{
		&self.classification
	}
}

/// Local module reference.
pub struct LocalModuleRef<'r, V: Vocabulary> {
	pub interpretation: &'r interpretation::Local<V>,
	pub dataset: &'r dataset::LocalDataset,
	pub classification: &'r classification::Local,
}

impl<'r, V: Vocabulary> LocalModuleRef<'r, V> {
	pub fn new(
		interpretation: &'r interpretation::Local<V>,
		dataset: &'r dataset::LocalDataset,
		classification: &'r classification::Local,
	) -> Self {
		Self {
			interpretation,
			dataset,
			classification,
		}
	}
}

impl<'r, V: Vocabulary> crate::Module<V> for LocalModuleRef<'r, V>
where
	V::Iri: Clone + Eq + Hash,
	V::Literal: Clone + Eq + Hash,
{
	type Error = std::convert::Infallible;

	type Interpretation<'a> = &'a interpretation::Local<V> where V: 'a, Self: 'a;
	type Dataset<'a> = &'a dataset::LocalDataset where V: 'a, Self: 'a;
	type Classification<'a> = &'a classification::Local where V: 'a, Self: 'a;

	fn interpretation<'a>(&'a self) -> Self::Interpretation<'a>
	where
		V: 'a,
	{
		self.interpretation
	}

	fn dataset<'a>(&'a self) -> Self::Dataset<'a>
	where
		V: 'a,
	{
		self.dataset
	}

	fn classification<'a>(&'a self) -> Self::Classification<'a>
	where
		V: 'a,
	{
		self.classification
	}
}
