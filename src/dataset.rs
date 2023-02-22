pub mod graph;

use std::{marker::PhantomData, hash::Hash};

pub use graph::Graph;
use hashbrown::HashMap;

use crate::{Id, interpretation::Interpretation, GlobalQuad, Vocabulary, Quad, Cause, GlobalTriple, GlobalTerm};

use self::graph::{ReplaceId, Contradiction};

pub struct Dataset<V: Vocabulary, M> {
	default_graph: Graph<M>,
	named_graphs: HashMap<Id, Graph<M>>,
	interpretation: Interpretation<V>
}

impl<V: Vocabulary, M> Dataset<V, M> {
	pub fn graph(&self, id: Option<Id>) -> Option<&Graph<M>> {
		match id {
			None => Some(&self.default_graph),
			Some(id) => self.named_graphs.get(&id)
		}
	}

	pub fn graphs_mut(&mut self) -> GraphsMut<M> {
		GraphsMut {
			default_graph: Some(&mut self.default_graph),
			named_graphs: self.named_graphs.iter_mut()
		}
	}

	pub fn insert(&mut self, rdf_types::Quad(s, p, o, g): Quad, positive: bool, cause: Cause<M>) -> Result<(Option<Id>, usize, bool), Contradiction> {
		let triple = rdf_types::Triple(s, p, o);
		match g {
			None => {
				let (i, b) = self.default_graph.insert(triple, positive, cause)?;
				Ok((None, i, b))
			},
			Some(g) => {
				let (i, b) = self.named_graphs.entry(g).or_default().insert(triple, positive, cause)?;
				Ok((Some(g), i, b))
			}
		}
	}
	
	pub fn merge(&mut self, a: Id, b: Id) -> Result<Vec<(Option<Id>, usize)>, Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		let (target_id, source_id, removed_literal_instances) = self.interpretation.merge(a, b);
		let mut new_statements = Vec::new();

		for (mut id, g) in self.graphs_mut() {
			id.replace_id(source_id, target_id);
			let mut removed_statements = g.remove_resource(source_id);
			removed_statements.replace_id(source_id, target_id);
			new_statements.extend(g.try_extend(removed_statements)?.into_iter().filter_map(|(i, is_new)| {
				if is_new {
					Some((id, i))
				} else {
					None
				}
			}));
		}

		if let Some(removed_graph) = self.named_graphs.remove(&source_id) {
			self.named_graphs.insert(target_id, removed_graph);
		}

		for (value, id) in removed_literal_instances {
			if let Some(other_id) = self.interpretation.get_mut(target_id).unwrap().insert_lexical_value(value, id) {
				new_statements.extend(self.merge(id, other_id)?)
			}
		}

		Ok(new_statements)
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

pub trait Semantics<V: Vocabulary, M> {
	fn deduce(&self, quad: GlobalQuad<V>, stack: &mut Vec<Statement<V, M>>);
}

pub enum Statement<V: Vocabulary, M> {
	Fact(GlobalQuad<V>, bool, Cause<M>),
	Eq(GlobalTerm<V>, GlobalTerm<V>),
	Neq(GlobalTerm<V>, GlobalTerm<V>)
}

pub trait Context<V: Vocabulary, M> {
	type EquivalentTerms<'a>: Iterator<Item = GlobalQuad<V>> where Self: 'a;

	/// Returns an iterator over all the known equivalent syntactic representations of the given term.
	fn equivalent_terms(&self, term: GlobalTerm<V>) -> Self::EquivalentTerms<'_>;

	/// Checks if the context contains the given triple.
	fn contains(&self, triple: &GlobalTriple<V>) -> bool;
}

pub struct DatasetExtension<V: Vocabulary, M, C, S> {
	dataset: Dataset<V, M>,
	context: C,
	vocabulary: PhantomData<V>,
	semantics: S
}

impl<V: Vocabulary, M, C: Context<V, M>, S: Semantics<V, M>> DatasetExtension<V, M, C, S> {
	pub fn insert(
		&mut self,
		quad: GlobalQuad<V>,
		positive: bool,
		cause: Cause<M>
	) -> Result<(), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
		M: Clone
	{
		let mut stack = vec![Statement::Fact(quad, positive, cause)];

		while let Some(statement) = stack.pop() {
			match statement {
				Statement::Fact(quad, positive, cause) => {
					let (triple, g) = quad.into_triple();
			
					if !self.context.contains(&triple) {
						let quad = triple.into_quad(g);
						let local_quad = self.dataset.interpretation.insert_quad_with_dependencies(&quad);
						let (_, _, inserted) = self.dataset.insert(local_quad, positive, cause.clone())?;
						if inserted {
							self.semantics.deduce(quad, &mut stack);
						}
					}
				}
				Statement::Eq(a, b) => {
					let a = self.dataset.interpretation.insert_term(&a);
					let b = self.dataset.interpretation.insert_term(&b);
					let new_facts = self.dataset.merge(a, b)?;
					for (g, i) in new_facts {
						let fact = self.dataset.graph(g).unwrap().get(i).unwrap();
						for rdf_types::Triple(s, p, o) in self.dataset.interpretation.global_triple_of(fact.triple) {
							match g {
								None => {
									stack.push(Statement::Fact(
										GlobalQuad::new(s, p, o, None),
										fact.positive,
										fact.cause.clone()
									))
								}
								Some(g) => for g in self.dataset.interpretation.terms_of(g) {
									stack.push(Statement::Fact(
										GlobalQuad::new(s.clone(), p.clone(), o.clone(), Some(g)),
										fact.positive,
										fact.cause.clone()
									))
								}
							}
						}
					}
				}
				Statement::Neq(_a, _b) => {
					todo!()
				}
			}
		}

		Ok(())
	}
}