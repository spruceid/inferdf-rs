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
pub use resource_terms::ResourceTermsPage;
pub use resource_triples::ResourceTriplesPage;
pub use triples::TriplesPage;

pub enum Page<V: Vocabulary> {
	Iris(IrisPage<V::Iri>),
	Literals(LiteralsPage<V::Literal>),
	ResourceTerms(ResourceTermsPage),
	Graphs(GraphsPage),
	Triples(TriplesPage),
	ResourceTriples(ResourceTriplesPage),
}

impl<V: Vocabulary> Page<V> {
	pub fn as_iris_page(&self) -> Option<&IrisPage<V::Iri>> {
		match self {
			Self::Iris(p) => Some(p),
			_ => None
		}
	}

	pub fn as_literals_page(&self) -> Option<&LiteralsPage<V::Literal>> {
		match self {
			Self::Literals(p) => Some(p),
			_ => None
		}
	}
}