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
	fn deduce(&self, triple: GlobalTriple<V>, graph: Option<GlobalTerm<V>>, stack: &mut Vec<Statement<V, M>>);
}

pub enum Statement<V: Vocabulary, M> {
	Fact(GlobalQuad<V>, bool, Cause<M>),
	Eq(GlobalTerm<V>, GlobalTerm<V>)
}

pub trait Context<V: Vocabulary, M> {
	fn contains(&self, triple: GlobalTriple<V>) -> bool;
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
				Statement::Fact(quad @ rdf_types::Quad(s, p, o, g), positive, cause) => {
					let triple = rdf_types::Triple(s, p, o);
			
					if !self.context.contains(triple) {
						let local_quad = self.dataset.interpretation.insert_quad(quad);
						self.dataset.insert(local_quad, positive, cause.clone())?;
						self.semantics.deduce(triple, g, &mut stack);
					}
				}
				Statement::Eq(a, b) => {
					let a = self.dataset.interpretation.insert_term(a);
					let b = self.dataset.interpretation.insert_term(b);
					let new_facts = self.dataset.merge(a, b)?;
					for (g, i) in new_facts {
						let triple = self.dataset.graph(g).unwrap().get(i);
						stack.push(Statement::Quad(GlobalQuad::new(
							// ...
						)))
					}
				}
			}
		}

		Ok(())
	}
}