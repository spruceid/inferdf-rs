use rdf_types::Vocabulary;

use crate::{Classification, Dataset, Interpretation};

pub mod composition;
pub mod local;
pub mod sub_module;

pub use composition::Composition;
pub use local::{LocalModule as Local, LocalModuleRef as LocalRef};
pub use sub_module::SubModule;

/// RDF module.
pub trait Module<V: Vocabulary> {
	type Error;
	type Dataset<'a>: Dataset<'a, V, Error = Self::Error>
	where
		Self: 'a,
		V: 'a;
	type Interpretation<'a>: Interpretation<'a, V, Error = Self::Error>
	where
		Self: 'a,
		V: 'a;
	type Classification<'a>: Classification<'a, V, Error = Self::Error>
	where
		Self: 'a,
		V: 'a;

	fn dataset<'a>(&'a self) -> Self::Dataset<'a>
	where
		V: 'a;

	fn interpretation<'a>(&'a self) -> Self::Interpretation<'a>
	where
		V: 'a;

	fn classification<'a>(&'a self) -> Self::Classification<'a>
	where
		V: 'a;
}
