pub mod graphs;
pub mod iris;
pub mod literals;
pub mod resource_terms;
pub mod resource_triples;
pub mod triples;

pub use graphs::GraphsPage;
pub use iris::IrisPage;
pub use literals::LiteralsPage;
use rdf_types::Vocabulary;
pub use resource_terms::ResourcesTermsPage;
pub use resource_triples::ResourcesTriplesPage;
pub use triples::TriplesPage;

pub enum Page<V: Vocabulary> {
	Iris(IrisPage<V::Iri>),
	Literals(LiteralsPage<V::Literal>),
	ResourcesTerms(ResourcesTermsPage),
	Graphs(GraphsPage),
	Triples(TriplesPage),
	ResourcesTriples(ResourcesTriplesPage),
}

impl<V: Vocabulary> Page<V> {
	pub fn as_iris_page(&self) -> Option<&IrisPage<V::Iri>> {
		match self {
			Self::Iris(p) => Some(p),
			_ => None,
		}
	}

	pub fn as_literals_page(&self) -> Option<&LiteralsPage<V::Literal>> {
		match self {
			Self::Literals(p) => Some(p),
			_ => None,
		}
	}

	pub fn as_resources_terms_page(&self) -> Option<&ResourcesTermsPage> {
		match self {
			Self::ResourcesTerms(p) => Some(p),
			_ => None,
		}
	}

	pub fn as_graphs_page(&self) -> Option<&GraphsPage> {
		match self {
			Self::Graphs(p) => Some(p),
			_ => None,
		}
	}

	pub fn as_triples_page(&self) -> Option<&TriplesPage> {
		match self {
			Self::Triples(p) => Some(p),
			_ => None,
		}
	}

	pub fn as_resources_triples_page(&self) -> Option<&ResourcesTriplesPage> {
		match self {
			Self::ResourcesTriples(p) => Some(p),
			_ => None,
		}
	}
}
