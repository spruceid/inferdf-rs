use rdf_types::Vocabulary;

use crate::{Dataset, Interpretation, Classification};

pub mod composition;

pub use composition::Composition;

/// RDF module.
pub trait Module<V: Vocabulary> {
	type Error;
	type Dataset<'a>: Dataset<'a, Error = Self::Error>
	where
		Self: 'a, V: 'a;
	type Interpretation<'a>: Interpretation<'a, V, Error = Self::Error>
	where
		Self: 'a, V: 'a;
	type Classification<'a>: Classification<'a, Error = Self::Error> where Self: 'a, V: 'a;

	fn dataset<'a>(&'a self) -> Self::Dataset<'a> where V: 'a;

	fn interpretation<'a>(&'a self) -> Self::Interpretation<'a> where V: 'a;

	fn classification<'a>(&'a self) -> Self::Classification<'a> where V: 'a;
}