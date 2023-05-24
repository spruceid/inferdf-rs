use locspan::Meta;
use rdf_types::Vocabulary;
use std::hash::Hash;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::{self, CompositeInterpretation, InterpretationMut},
	uninterpreted, Cause, Id, Quad, ReplaceId, Sign, Signed,
};

use crate::semantics::Semantics;

mod context;
mod dependency;

pub use context::*;
pub use dependency::*;

pub enum Contradiction {
	Data(dataset::Contradiction),
	Interpretation(interpretation::Contradiction),
}

impl From<dataset::Contradiction> for Contradiction {
	fn from(value: dataset::Contradiction) -> Self {
		Self::Data(value)
	}
}

impl From<interpretation::Contradiction> for Contradiction {
	fn from(value: interpretation::Contradiction) -> Self {
		Self::Interpretation(value)
	}
}

pub enum QuadStatement {
	Quad(Quad),
	Eq(Id, Id, Option<Id>),
}

impl ReplaceId for QuadStatement {
	fn replace_id(&mut self, a: Id, b: Id) {
		match self {
			Self::Quad(t) => t.replace_id(a, b),
			Self::Eq(c, d, g) => {
				c.replace_id(a, b);
				d.replace_id(a, b);
				g.replace_id(a, b);
			}
		}
	}
}

pub struct Builder<V: Vocabulary, M, S> {
	interpretation: CompositeInterpretation<V>,
	data: Data<V, M>,
	semantics: S,
}

impl<V: Vocabulary, M, S> Builder<V, M, S> {
	pub fn new(
		dependencies: Dependencies<V, M>,
		interpretation: CompositeInterpretation<V>,
		semantics: S,
	) -> Self {
		Self {
			interpretation,
			data: Data {
				set: Dataset::new(),
				dependencies,
			},
			semantics,
		}
	}

	pub fn interpretation(&self) -> &CompositeInterpretation<V> {
		&self.interpretation
	}

	pub fn dataset(&self) -> &Dataset<Cause<M>> {
		&self.data.set
	}
}

impl<V: Vocabulary, M, S: Semantics> InterpretationMut<V> for Builder<V, M, S>
where
	V::Iri: Copy + Eq + Hash,
	V::BlankId: Copy + Eq + Hash,
	V::Literal: Copy + Eq + Hash,
{
	fn insert_term(&mut self, term: uninterpreted::Term<V>) -> Id {
		self.insert_term(term)
	}
}

impl<V: Vocabulary, M, S: Semantics> Builder<V, M, S> {
	pub fn insert_term(&mut self, term: uninterpreted::Term<V>) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		self.interpretation
			.insert_term(&self.data.dependencies, term)
	}

	pub fn insert_quad(&mut self, quad: uninterpreted::Quad<V>) -> Quad
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		self.interpretation
			.insert_quad(&self.data.dependencies, quad)
	}

	/// Insert a new quad in the built dataset.
	pub fn insert(
		&mut self,
		Meta(Signed(sign, quad), cause): dataset::Fact<Cause<M>>,
	) -> Result<(), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
		M: Clone,
	{
		let mut stack = vec![Meta(Signed(sign, QuadStatement::Quad(quad)), cause)];

		while let Some(Meta(statement, cause)) = stack.pop() {
			match statement {
				Signed(sign, QuadStatement::Quad(quad)) => {
					let (triple, g) = quad.into_triple();
					if self
						.data
						.dependencies
						.filter(&self.interpretation, triple, sign)?
					{
						let meta = cause.metadata().clone();
						let (_, _, inserted) =
							self.data.set.insert(Meta(Signed(sign, quad), cause))?;

						if inserted {
							let mut context = Context::new(&mut self.interpretation, &self.data);
							self.semantics.deduce(
								&mut context,
								Signed(sign, triple),
								|Signed(sign, statement)| {
									stack.push(Meta(
										Signed(sign, statement.with_graph(g)),
										Cause::Entailed(meta.clone()),
									))
								},
							)
						}
					}
				}
				Signed(Sign::Positive, QuadStatement::Eq(a, b, g)) => {
					let (a, b) = self.interpretation.merge(a, b)?;
					self.data
						.set
						.replace_id(b, a, |Meta(Signed(sign, triple), _)| {
							self.data
								.dependencies
								.filter(&self.interpretation, *triple, *sign)
						})?;
					stack.replace_id(b, a);

					for (d, _, Meta(Signed(sign, quad), cause)) in
						self.data.resource_facts(&self.interpretation, a)
					{
						let triple = self.interpretation.import_triple(
							&self.data.dependencies,
							d,
							quad.into_triple().0,
						);

						let meta = cause.metadata();
						let mut context = Context::new(&mut self.interpretation, &self.data);
						self.semantics.deduce(
							&mut context,
							Signed(sign, triple),
							|Signed(sign, statement)| {
								stack.push(Meta(
									Signed(sign, statement.with_graph(g)),
									Cause::Entailed(meta.clone()),
								))
							},
						);
					}
				}
				Signed(Sign::Negative, QuadStatement::Eq(a, b, _)) => {
					self.interpretation.split(a, b)?;
				}
			}
		}

		Ok(())
	}
}

pub struct Data<V: Vocabulary, M> {
	set: Dataset<Cause<M>>,
	dependencies: Dependencies<V, M>,
}

impl<V: Vocabulary, M> Data<V, M> {
	pub fn resource_facts(
		&self,
		interpretation: &CompositeInterpretation<V>,
		id: Id,
	) -> ResourceFacts<M> {
		let toplevel = self.set.resource_facts(id);
		let dependencies = self
			.dependencies
			.iter()
			.flat_map(move |(i, d)| {
				interpretation
					.dependency_ids(i, id)
					.filter_map(move |local_id| {
						let facts = d.dataset().resource_facts(local_id);
						if facts.is_empty() {
							None
						} else {
							Some((i, local_id, facts))
						}
					})
			})
			.collect();

		ResourceFacts {
			id,
			toplevel,
			dependencies,
		}
	}
}

/// Iterator over all the facts about the given resource, and the dependency it comes from.
///
/// Facts are given in the dependency interpretation, not the top level interpretation.
pub struct ResourceFacts<'a, M> {
	id: Id,
	toplevel: dataset::ResourceFacts<'a, Cause<M>>,
	dependencies: Vec<(usize, Id, dataset::ResourceFacts<'a, Cause<M>>)>,
}

impl<'a, M> Iterator for ResourceFacts<'a, M> {
	type Item = (Option<usize>, Id, dataset::Fact<&'a Cause<M>>);

	fn next(&mut self) -> Option<Self::Item> {
		self.toplevel
			.next()
			.map(|fact| (None, self.id, fact))
			.or_else(|| {
				while let Some((d, local_id, facts)) = self.dependencies.last_mut() {
					match facts.next() {
						Some(fact) => return Some((Some(*d), *local_id, fact)),
						None => {
							self.dependencies.pop();
						}
					}
				}

				None
			})
	}
}
