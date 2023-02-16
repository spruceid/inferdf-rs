use std::marker::PhantomData;

use hashbrown::HashMap;

use iref::{IriBuf, Iri};
use rdf_types::{Object, BlankIdBuf};

pub mod dataset;
pub mod rule;

pub type Triple = rdf_types::Triple<Id, Id, Id>;
pub type Quad = rdf_types::Quad<Id, Id, Id, Id>;

pub trait MapTriple {
	fn map_with(&self, m: &HashMap<Id, Id>) -> Self;
}

impl MapTriple for Triple {
	fn map_with(&self, m: &HashMap<Id, Id>) -> Self {
		todo!()
	}
}

impl MapTriple for Rule {
	fn map_with(&self, m: &HashMap<Id, Id>) -> Self {
		todo!()
	}
}

pub use dataset::{LocalDataset, CachedGraph};
use rule::Rule;

pub struct AnonymousGraph;

pub struct AnonymousGraphId(usize);

pub struct AnonymousNode(AnonymousGraphId, usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);

/// Dataset vocabulary.
pub struct Vocabulary {
	resources: Vec<Object>,
	anonymous_graphs: Vec<AnonymousGraph>,
	by_iri: HashMap<IriBuf, Id>,
	by_blank: HashMap<BlankIdBuf, Id>,
	by_literal: HashMap<LiteralValue, Id>,
	by_anonymous_graph: HashMap<AnonymousGraph, AnonymousGraphId>,
	by_anonymous_node: HashMap<AnonymousNode, (Id, BlankIdBuf)>
}

impl Vocabulary {
	pub fn get_iri(&self, iri: Iri) -> Option<Id> {
		self.by_iri.get(&iri).copied()
	}
}

pub trait Context<'d, M: 'd>: Clone + Copy {
	type Graphs<'c>: Iterator<Item = &'d CachedGraph<M>> where Self: 'c;

	fn vocabulary(&self) -> &Vocabulary;

	fn graphs(&self) -> Self::Graphs<'_>;

	fn matches(&self, pattern: rule::Pattern) -> Matches<'d, M> {
		todo!()
	}

	fn insert_rule(&mut self, rule: Rule) {
		// ...
	}
}

pub struct Matches<'d, M>(PhantomData<&'d M>);

impl<'d, M> Iterator for Matches<'d, M> {
	type Item = (Triple, Cause<&'d M>);

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}

pub struct Substitution {
	pub context_to_dataset: HashMap<Id, Id>,
	pub dataset_to_context: HashMap<Id, Id>
}

pub struct Resource<M> {
	properties: HashMap<Id, PropertyValues<M>>
}

pub struct PropertyValues<M> {
	value: Vec<(Id, Cause<M>)>
}

pub enum Cause<M> {
	Stated(M),
	Entailed(M)
}

impl<M> Cause<M> {
	pub fn metadata(&self) -> &M {
		match self {
			Self::Stated(m) => m,
			Self::Entailed(m) => m
		}
	}
}

pub struct LiteralValue {
	// ...
}

pub struct Datasets<K, M> {
	datasets: HashMap<K, LocalDataset<M>>
}