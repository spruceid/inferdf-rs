use locspan::Meta;
use rdf_types::Vocabulary;
use std::hash::Hash;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::{self, InterpretationMut, composite},
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

pub struct Builder<V: Vocabulary, D: Dependency<V>, S> {
	interpretation: composite::Interpretation<V>,
	data: Data<V, D>,
	semantics: S,
}

impl<V: Vocabulary, D: Dependency<V>, S> Builder<V, D, S> {
	pub fn new(
		dependencies: Dependencies<V, D>,
		interpretation: composite::Interpretation<V>,
		semantics: S,
	) -> Self
	where
		D: Default,
	{
		Self {
			interpretation,
			data: Data {
				set: dataset::Standard::new(),
				dependencies,
			},
			semantics,
		}
	}

	pub fn interpretation(&self) -> &composite::Interpretation<V> {
		&self.interpretation
	}

	pub fn dataset(&self) -> &dataset::Standard<Cause<D::Metadata>> {
		&self.data.set
	}
}

impl<'a, V: Vocabulary, D: Dependency<V>, S: Semantics> InterpretationMut<'a, V> for Builder<V, D, S>
where
	V::Iri: Copy + Eq + Hash,
	V::BlankId: Copy + Eq + Hash,
	V::Literal: Copy + Eq + Hash,
{
	fn insert_term(&mut self, term: uninterpreted::Term<V>) -> Id {
		self.insert_term(term)
	}
}

impl<V: Vocabulary, D: Dependency<V>, S: Semantics> Builder<V, D, S> {
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
}

impl<V: Vocabulary, D: Dependency<V>, S: Semantics> Builder<V, D, S> {
	/// Insert a new quad in the built dataset.
	pub fn insert(
		&mut self,
		Meta(Signed(sign, quad), cause): dataset::Fact<Cause<D::Metadata>>,
	) -> Result<(), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
		D::Metadata: Clone
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

pub struct Data<V: Vocabulary, D: Dependency<V>> {
	set: dataset::Standard<Cause<D::Metadata>>,
	dependencies: Dependencies<V, D>,
}

impl<V: Vocabulary, D: Dependency<V>> Data<V, D> {
	pub fn resource_facts(
		&self,
		interpretation: &composite::Interpretation<V>,
		id: Id,
	) -> ResourceFacts<D::Metadata, D::Dataset<'_>> {
		let toplevel = self.set.resource_facts(id);
		let dependencies = self
			.dependencies
			.iter()
			.flat_map(move |(i, d)| {
				interpretation
					.dependency_ids(i, id)
					.filter_map(move |local_id| {
						let mut facts = d.dataset().resource_facts(local_id);
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
pub struct ResourceFacts<'a, M, D: Dataset<'a>> {
	id: Id,
	toplevel: dataset::standard::ResourceFacts<'a, Cause<M>>,
	dependencies: Vec<(usize, Id, dataset::ResourceFacts<'a, D>)>,
}

impl<'a, M, D: Dataset<'a, Metadata = Cause<M>>> Iterator for ResourceFacts<'a, M, D> {
	type Item = (Option<usize>, Id, dataset::Fact<&'a Cause<M>>);

	fn next(&mut self) -> Option<Self::Item> {
		self.toplevel
			.next()
			.map(|fact| (None, self.id, fact))
			.or_else(|| {
				while let Some((d, local_id, facts)) = self.dependencies.last_mut() {
					match facts.next() {
						Some((_, fact)) => return Some((Some(*d), *local_id, fact)),
						None => {
							self.dependencies.pop();
						}
					}
				}

				None
			})
	}
}
