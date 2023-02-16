use std::collections::{BTreeMap, BTreeSet, HashMap};

use derivative::Derivative;
use locspan::Meta;
use crate::{Id, Vocabulary, Context, Triple, Cause, Substitution, MapTriple, rule::{Rules, Rule}};

pub struct CachedDataset {
	// ...
}

/// TreeLDR dataset.
// #[async_trait]
pub trait Dataset {
	// ...
}

// #[async_trait]
// pub trait Predicates {
// 	type Error;

// 	type Objects<'a>: Stream<Item = Triple> where Self: 'a;

// 	/// Returns an iterator over the object of the given (subject, predicate) pair.
// 	async fn values_of(&self, property: Id) -> Result<Option<Self::Objects<'_>>, Self::Error>;
// }

#[derive(Derivative)]
#[derivative(Clone(bound="C: Copy"), Copy(bound="C: Copy"))]
pub struct LocalContext<'d, C: Copy, M> {
	parent: C,
	graph: &'d CachedGraph<M>
}

impl<'d, C: Copy, M> LocalContext<'d, C, M> {
	pub fn new(
		parent: C,
		graph: &'d CachedGraph<M>
	) -> Self {
		Self { parent, graph }
	}
}

impl<'d, C: Context<'d, M>, M> Context<'d, M> for LocalContext<'d, C, M> {
	type Graphs<'c> = LocalGraphs<'d, 'c, C, M> where Self: 'c;

	fn vocabulary(&self) -> &Vocabulary {
		self.parent.vocabulary()
	}

	fn graphs(&self) -> Self::Graphs<'_> {
		LocalGraphs {
			parent: self.parent.graphs(),
			graph: Some(self.graph)
		}
	}
}

pub struct LocalGraphs<'d, 'c, C: 'c + Context<'d, M>, M> {
	parent: C::Graphs<'c>,
	graph: Option<&'d CachedGraph<M>>
}

impl<'c, 'd, C: 'c + Context<'d, M>, M> Iterator for LocalGraphs<'d, 'c, C, M> {
	type Item = &'d CachedGraph<M>;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}

/// RDF graph.
pub struct CachedGraph<M> {
	by_subject: BTreeMap<Option<Id>, BTreeMap<Id, MetaObjects<M>>>,
	rules: Rules
}

struct MetaObjects<M>(BTreeSet<Meta<Id, Cause<M>>>);

/// RDF Dataset.
pub struct LocalDataset<M> {
	vocabulary: Vocabulary,
	default: CachedGraph<M>,
	named: BTreeMap<Id, CachedGraph<M>>
}

impl<M> LocalDataset<M> {
	pub fn vocabulary(&self) -> &Vocabulary {
		&self.vocabulary
	}

	pub fn insert(&mut self, quad: Meta<rdf_types::Quad, Cause<M>>) {
		todo!()
	}
}

impl<M> Extend<Meta<rdf_types::Quad, Cause<M>>> for LocalDataset<M> {
	fn extend<T: IntoIterator<Item = Meta<rdf_types::Quad, Cause<M>>>>(&mut self, iter: T) {
		for q in iter {
			self.insert(q)
		}
	}
}

impl<M> CachedGraph<M> {
	pub fn triples(&self) -> Triples<M> {
		todo!()
	}

	/// Insert the given fact into the dataset.
	pub fn insert<'d>(
		&'d self,
		context: impl Context<'d, M>,
		substitution: &Substitution,
		Meta(local_rule, _): Meta<crate::Rule, Cause<M>>
	) -> HashMap<Rule, M> where M: 'd + Clone {
		let local_context = LocalContext::new(context, self);
		
		let mut unprocessed_new_rules = Vec::new();
		let rule = local_rule.map_with(&substitution.dataset_to_context);
		rule.deduce_from_insertion(local_context, &mut unprocessed_new_rules);

		let mut new_rules = HashMap::new();
		while let Some((local_rule, cause)) = unprocessed_new_rules.pop() {
			if new_rules.insert(local_rule, cause.clone()).is_none() {
				// ...
			}
		}

		// let mut unprocessed_new_triples = Vec::new();
		// for (local_triple, cause) in self.triples() {
		// 	let triple = local_triple.map_with(&substitution.dataset_to_context);
		// 	unprocessed_new_triples.extend(S::apply(local_context, triple, cause.metadata()).into_iter().map(|(t, cause)| {
		// 		(t.map_with(&substitution.context_to_dataset), cause)
		// 	}));
		// }

		// while let Some((local_triple, cause)) = unprocessed_new_triples.pop() {
		// 	if new_triples.insert(local_triple, cause.clone()).is_none() {
		// 		let triple = local_triple.map_with(&substitution.dataset_to_context);
		// 		unprocessed_new_triples.extend(S::apply(local_context, triple, cause).into_iter().map(|(t, cause)| {
		// 			(t.map_with(&substitution.context_to_dataset), cause)
		// 		}));
		// 	}
		// }

		new_rules
	}
}

pub struct Triples<'a, M> {
	subjects: std::collections::btree_map::Iter<'a, Id, BTreeMap<Id, MetaObjects<M>>>,
	current: Option<(Id, InnerPredicates<'a, M>)>
}

impl<'a, M> Iterator for Triples<'a, M> {
	type Item = (Triple, &'a Cause<M>);

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}

struct InnerPredicates<'a, M> {
	predicates: std::collections::btree_map::Iter<'a, Id, MetaObjects<M>>,
	current: Option<(Id, InnerObjects<'a, M>)>
}

impl<'a, M> Iterator for InnerPredicates<'a, M> {
	type Item = (Id, Meta<Id, &'a Cause<M>>);

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}

type InnerObjects<'a, M> = std::collections::btree_set::Iter<'a, Meta<Id, M>>;