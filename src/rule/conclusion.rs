use rdf_types::{vocabulary::EmbedIntoVocabulary, Term, Vocabulary};
use serde::{Deserialize, Serialize};

use crate::{expression::Expression, pattern::ResourceOrVar, Signed, TripleStatement};

/// Rule conclusion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Conclusion<T> {
	/// Number of variables introduced in the conclusion.
	pub variables: usize,

	/// Concluded statements.
	pub statements: Vec<Signed<TripleStatementPattern<T>>>,
}

impl<T> Conclusion<T> {
	pub fn new(variables: usize, statements: Vec<Signed<TripleStatementPattern<T>>>) -> Self {
		Self {
			variables,
			statements,
		}
	}

	pub fn visit_variables(&self, mut f: impl FnMut(usize)) {
		for Signed(_, v) in &self.statements {
			match v {
				TripleStatementPattern::Eq(s, o) => {
					s.visit_variables(&mut f);
					o.visit_variables(&mut f);
				}
				TripleStatementPattern::Triple(rdf_types::Triple(s, p, o)) => {
					s.visit_variables(&mut f);
					p.visit_variables(&mut f);
					o.visit_variables(&mut f);
				}
				TripleStatement::True(r) => r.visit_variables(&mut f),
			}
		}
	}
}

impl<V: Vocabulary, T: EmbedIntoVocabulary<V>> EmbedIntoVocabulary<V> for Conclusion<T> {
	type Embedded = Conclusion<T::Embedded>;

	fn embed_into_vocabulary(self, vocabulary: &mut V) -> Self::Embedded {
		Conclusion {
			variables: self.variables,
			statements: self.statements.embed_into_vocabulary(vocabulary),
		}
	}
}

pub type TripleStatementPattern<T = Term> = TripleStatement<Expression<ResourceOrVar<T>>>;
