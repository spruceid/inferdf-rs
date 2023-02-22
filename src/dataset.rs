pub mod graph;

use std::{marker::PhantomData, hash::Hash};

pub use graph::Graph;
use hashbrown::HashMap;

use crate::{Id, interpretation::{Interpretation, CompositeInterpretation}, GlobalQuad, Vocabulary, Quad, Cause, GlobalTriple, GlobalTerm, GlobalQuadExt, GlobalTermExt, Triple};

use self::graph::{ReplaceId, Contradiction};

#[derive(Debug, Clone)]
pub struct Fact<M> {
	pub quad: Quad,
	pub positive: bool,
	pub cause: Cause<M>
}

impl<M> Fact<M> {
	pub fn new(
		quad: Quad,
		positive: bool,
		cause: Cause<M>
	) -> Self {
		Self {
			quad,
			positive,
			cause
		}
	}

	pub fn triple(&self) -> Triple {
		self.quad.into_triple().0
	}

	pub fn into_graph_fact(self) -> (graph::Fact<M>, Option<Id>) {
		let (triple, g) = self.quad.into_triple();
		(graph::Fact::new(triple, self.positive, self.cause), g)
	}
}

pub struct Dataset<M> {
	default_graph: Graph<M>,
	named_graphs: HashMap<Id, Graph<M>>
}

impl<M> Dataset<M> {
	pub fn contains_triple(&self, triple: Triple, positive: bool) -> bool {
		self.default_graph.contains(triple, positive) || self.named_graphs.values().any(|g| g.contains(triple, positive))
	}

	pub fn graph(&self, id: Option<Id>) -> Option<&Graph<M>> {
		match id {
			None => Some(&self.default_graph),
			Some(id) => self.named_graphs.get(&id)
		}
	}

	pub fn get_or_insert_graph_mut(&mut self, id: Option<Id>) -> &mut Graph<M> {
		match id {
			None => &mut self.default_graph,
			Some(id) => self.named_graphs.entry(id).or_default()
		}
	}

	pub fn graphs_mut(&mut self) -> GraphsMut<M> {
		GraphsMut {
			default_graph: Some(&mut self.default_graph),
			named_graphs: self.named_graphs.iter_mut()
		}
	}

	pub fn remove_graph(&mut self, id: Id) -> Option<Graph<M>> {
		self.named_graphs.remove(&id)
	}

	pub fn insert_graph(&mut self, id: Id, graph: Graph<M>) -> Option<Graph<M>> {
		self.named_graphs.insert(id, graph)
	}

	pub fn insert(&mut self, fact: Fact<M>) -> Result<(Option<Id>, usize, bool), Contradiction> {
		let (fact, g) = fact.into_graph_fact();
		match g {
			None => {
				let (i, b) = self.default_graph.insert(fact)?;
				Ok((None, i, b))
			},
			Some(g) => {
				let (i, b) = self.named_graphs.entry(g).or_default().insert(fact)?;
				Ok((Some(g), i, b))
			}
		}
	}
}

pub struct GraphsMut<'a, M> {
	default_graph: Option<&'a mut Graph<M>>,
	named_graphs: hashbrown::hash_map::IterMut<'a, Id, Graph<M>>
}

impl<'a, M> Iterator for GraphsMut<'a, M> {
	type Item = (Option<Id>, &'a mut Graph<M>);

	fn next(&mut self) -> Option<Self::Item> {
		self.default_graph.take().map(|g| (None, g)).or_else(|| self.named_graphs.next().map(|(id, g)| (Some(*id), g)))
	}
}

pub trait Semantics<M> {
	fn deduce(&self, triple: Triple, meta: &M, f: impl FnMut(graph::Fact<M>));
}

pub enum Statement<M> {
	Fact(Fact<M>),
	Eq(Id, Id),
	Neq(Id, Id)
}

pub trait Context<V: Vocabulary, M> {
	/// Returns the context dependencies.
	fn dependencies(&self) -> &HashMap<usize, Dependency<V, M>>;

	/// Checks if the context contains the given triple.
	fn contains(&self, triple: &GlobalTriple<V>) -> bool;
}

pub struct Dependency<V: Vocabulary, M> {
	interpretation: Interpretation<V>,
	dataset: Dataset<M>,
}

impl<V: Vocabulary, M> crate::interpretation::Dependency<V> for Dependency<V, M> {
	fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}
}

pub struct DatasetExtension<V: Vocabulary, M, S> {
	interpretation: CompositeInterpretation<V>,
	dataset: Dataset<M>,
	dependencies: HashMap<usize, Dependency<V, M>>,
	semantics: S
}

impl<V: Vocabulary, M, S: Semantics<M>> DatasetExtension<V, M, S> {
	pub fn contains(
		&self,
		triple: Triple,
		positive: bool
	) -> bool {
		if self.dataset.contains_triple(triple, positive) {
			return true
		}

		for (&d, dependency) in &self.dependencies {
			for dependency_triple in self.interpretation.dependency_triples(d, triple) {
				if dependency.dataset.contains_triple(dependency_triple, positive) {
					return true
				}
			}
		}

		false
	}

	pub fn insert(
		&mut self,
		fact: Fact<M>
	) -> Result<(), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
		M: Clone
	{
		let mut stack = vec![Statement::Fact(fact)];

		while let Some(statement) = stack.pop() {
			match statement {
				Statement::Fact(fact) => {
					let (triple, g) = fact.quad.into_triple();
					if !self.contains(triple, fact.positive) {
						self.semantics.deduce(triple, fact.cause.metadata(), |fact| stack.push(Statement::Fact(fact.in_graph(g))));
						self.dataset.insert(fact);
					}
				}
				Statement::Eq(a, b) => {
					let si_a = a.interpret_literal_type_with(|t| self.interpretation.insert_term(self.context.dependencies(), t));
					let si_b = b.interpret_literal_type_with(|t| self.interpretation.insert_term(self.context.dependencies(), t));
					let a = self.interpretation.insert_term(self.context.dependencies(), si_a);
					let b = self.interpretation.insert_term(self.context.dependencies(), si_b);
					let (a, b, removed_literal_instances) = self.interpretation.merge(a, b);

					let mut facts = Vec::new();

					for (_, g) in self.dataset.graphs_mut() {
						facts.extend(g.remove_resource(a));
						facts.extend(g.remove_resource(b));
					}

					if let Some(removed_graph) = self.dataset.remove_graph(b) {
						let g = self.dataset.get_or_insert_graph_mut(Some(a));
						for (_, mut fact) in removed_graph {
							g.insert(fact.triple, fact.positive, fact.cause)?;
						}
					}
			
					for (value, id) in removed_literal_instances {
						if let Some(other_id) = self.interpretation.get_mut(a).unwrap().insert_lexical_value(value, id) {
							new_statements.extend(self.merge(id, other_id)?)
						}
					}

					// let a = self.dataset.interpretation.insert_term(&a);
					// let b = self.dataset.interpretation.insert_term(&b);
					// let new_facts = self.dataset.merge(a, b)?;
					// for (g, i) in new_facts {
					// 	let fact = self.dataset.graph(g).unwrap().get(i).unwrap();
					// 	for rdf_types::Triple(s, p, o) in self.dataset.interpretation.global_triple_of(fact.triple) {
					// 		match g {
					// 			None => {
					// 				stack.push(Statement::Fact(
					// 					GlobalQuad::new(s, p, o, None),
					// 					fact.positive,
					// 					fact.cause.clone()
					// 				))
					// 			}
					// 			Some(g) => for g in self.dataset.interpretation.terms_of(g) {
					// 				stack.push(Statement::Fact(
					// 					GlobalQuad::new(s.clone(), p.clone(), o.clone(), Some(g)),
					// 					fact.positive,
					// 					fact.cause.clone()
					// 				))
					// 			}
					// 		}
					// 	}
					// }
				}
				Statement::Neq(_a, _b) => {
					todo!()
				}
			}
		}

		Ok(())
	}
}