use indexmap::IndexSet;
use locspan::Meta;
use rdf_types::Vocabulary;
use std::hash::Hash;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::{self, composite, InterpretationMut},
	uninterpreted, Entailment, Fact, Id, Module, Quad, ReplaceId, Sign, Signed, TryCollect,
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

pub struct Builder<V: Vocabulary, D: Module<V>, S> {
	interpretation: composite::Interpretation<V>,
	data: Data<V, D>,
	semantics: S,
	entailments: IndexSet<Entailment>,
}

impl<V: Vocabulary, D: Module<V>, S> Builder<V, D, S> {
	pub fn new(
		dependencies: Dependencies<V, D>,
		interpretation: composite::Interpretation<V>,
		semantics: S,
	) -> Self {
		Self {
			interpretation,
			data: Data {
				set: dataset::LocalDataset::new(),
				dependencies,
			},
			semantics,
			entailments: IndexSet::new(),
		}
	}

	pub fn interpretation(&self) -> &composite::Interpretation<V> {
		&self.interpretation
	}

	pub fn dataset(&self) -> &dataset::LocalDataset {
		&self.data.set
	}
}

impl<'a, V: Vocabulary, D: Module<V>, S: Semantics> InterpretationMut<'a, V> for Builder<V, D, S>
where
	V::Iri: Copy + Eq + Hash,
	V::BlankId: Copy + Eq + Hash,
	V::Literal: Copy + Eq + Hash,
{
	type Error = D::Error;

	fn insert_term(
		&mut self,
		vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Id, Self::Error> {
		self.insert_term(vocabulary, term)
	}
}

impl<V: Vocabulary, D: Module<V>, S: Semantics> Builder<V, D, S> {
	pub fn insert_term(
		&mut self,
		vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Id, D::Error>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		self.interpretation
			.insert_term(vocabulary, &self.data.dependencies, term)
	}

	pub fn insert_quad(
		&mut self,
		vocabulary: &mut V,
		quad: uninterpreted::Quad<V>,
	) -> Result<Quad, D::Error>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		self.interpretation
			.insert_quad(vocabulary, &self.data.dependencies, quad)
	}
}

pub enum Error<E> {
	Contradiction(Contradiction),
	Dependency(E),
}

impl<E> From<dataset::Contradiction> for Error<E> {
	fn from(value: dataset::Contradiction) -> Self {
		Self::Contradiction(value.into())
	}
}

impl<E> From<interpretation::Contradiction> for Error<E> {
	fn from(value: interpretation::Contradiction) -> Self {
		Self::Contradiction(value.into())
	}
}

impl<E> From<dependency::Error<E>> for Error<E> {
	fn from(value: dependency::Error<E>) -> Self {
		match value {
			dependency::Error::Contradiction(c) => c.into(),
			dependency::Error::Module(e) => Self::Dependency(e),
		}
	}
}

impl<V: Vocabulary, D: Module<V>, S: Semantics> Builder<V, D, S> {
	/// Insert a new quad in the built dataset.
	pub fn insert(
		&mut self,
		vocabulary: &mut V,
		Meta(Signed(sign, quad), cause): Fact,
	) -> Result<(), Error<D::Error>>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
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
						let (_, _, inserted) =
							self.data.set.insert(Meta(Signed(sign, quad), cause))?;

						if inserted {
							let mut context = Context::new(&mut self.interpretation, &self.data);
							self.semantics
								.deduce(
									&mut context,
									Signed(sign, triple),
									|e| self.entailments.insert_full(e).0 as u32,
									|Meta(Signed(sign, statement), cause)| {
										stack.push(Meta(
											Signed(sign, statement.with_graph(g)),
											cause,
										))
									},
								)
								.map_err(Error::Dependency)?
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

					for signed_quad in self
						.data
						.resource_facts(&self.interpretation, a)
						.map_err(Error::Dependency)?
					{
						let (d, _, Meta(Signed(sign, quad), _)) =
							signed_quad.map_err(Error::Dependency)?;
						let triple = self
							.interpretation
							.import_triple(
								vocabulary,
								&self.data.dependencies,
								d,
								quad.into_triple().0,
							)
							.map_err(Error::Dependency)?;

						let mut context = Context::new(&mut self.interpretation, &self.data);
						self.semantics
							.deduce(
								&mut context,
								Signed(sign, triple),
								|e| self.entailments.insert_full(e).0 as u32,
								|Meta(Signed(sign, statement), cause)| {
									stack.push(Meta(Signed(sign, statement.with_graph(g)), cause))
								},
							)
							.map_err(Error::Dependency)?;
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

pub struct Data<V: Vocabulary, D: Module<V>> {
	set: dataset::LocalDataset,
	dependencies: Dependencies<V, D>,
}

impl<V: Vocabulary, D: Module<V>> Data<V, D> {
	pub fn resource_facts(
		&self,
		interpretation: &composite::Interpretation<V>,
		id: Id,
	) -> Result<ResourceFacts<D::Dataset<'_>>, D::Error> {
		let toplevel = self.set.resource_facts(id);
		let dependencies = self
			.dependencies
			.iter()
			.flat_map(move |(i, d)| {
				interpretation
					.dependency_ids(i, id)
					.filter_map(move |local_id| {
						let mut facts = d.dataset().resource_facts(local_id);
						match facts.is_empty() {
							Ok(true) => None,
							Ok(false) => Some(Ok((i, local_id, facts))),
							Err(e) => Some(Err(e)),
						}
					})
			})
			.try_collect()?;

		Ok(ResourceFacts {
			id,
			toplevel,
			dependencies,
		})
	}
}

/// Iterator over all the facts about the given resource, and the dependency it comes from.
///
/// Facts are given in the dependency interpretation, not the top level interpretation.
pub struct ResourceFacts<'a, D: Dataset<'a>> {
	id: Id,
	toplevel: dataset::local::ResourceFacts<'a>,
	dependencies: Vec<(usize, Id, dataset::ResourceFacts<'a, D>)>,
}

impl<'a, D: Dataset<'a>> Iterator for ResourceFacts<'a, D> {
	type Item = Result<(Option<usize>, Id, Fact), D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.toplevel
			.next()
			.map(|fact| Ok((None, self.id, fact)))
			.or_else(|| {
				while let Some((d, local_id, facts)) = self.dependencies.last_mut() {
					match facts.next() {
						Some(Err(e)) => return Some(Err(e)),
						Some(Ok((_, fact))) => return Some(Ok((Some(*d), *local_id, fact))),
						None => {
							self.dependencies.pop();
						}
					}
				}

				None
			})
	}
}
