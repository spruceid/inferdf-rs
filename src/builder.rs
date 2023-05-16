use std::hash::Hash;

use hashbrown::HashMap;

use crate::{
	dataset::{self, Dataset},
	interpretation::{self, CompositeInterpretation, Interpretation},
	Cause, Id, Triple, Vocabulary, ReplaceId,
};

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

pub trait Semantics<M> {
	fn deduce(&self, triple: Triple, f: impl FnMut(Triple, bool));
}

pub enum Statement<M> {
	Fact(dataset::Fact<M>),
	Eq(Id, Id, Option<Id>),
	Neq(Id, Id),
}

impl<M> ReplaceId for Statement<M> {
	fn replace_id(&mut self, a: Id, b: Id) {
		match self {
			Self::Fact(fact) => fact.replace_id(a, b),
			Self::Eq(c, d, g) => {
				c.replace_id(a, b);
				d.replace_id(a, b);
				g.replace_id(a, b);
			}
			Self::Neq(c, d) => {
				c.replace_id(a, b);
				d.replace_id(a, b);
			}
		}
	}
}

pub struct Builder<V: Vocabulary, M, S> {
	interpretation: CompositeInterpretation<V>,
	data: Data<V, M>,
	semantics: S,
}

pub struct Data<V: Vocabulary, M> {
	set: Dataset<M>,
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
			.map(move |(i, d)| {
				interpretation
					.dependency_ids(i, id)
					.filter_map(move |local_id| {
						let facts = d.dataset.resource_facts(local_id);
						if facts.is_empty() {
							None
						} else {
							Some((i, local_id, facts))
						}
					})
			})
			.flatten()
			.collect();

		ResourceFacts {
			id,
			toplevel,
			dependencies,
		}
	}
}

impl<V: Vocabulary, M, S: Semantics<M>> Builder<V, M, S> {
	pub fn insert(&mut self, fact: dataset::Fact<M>) -> Result<(), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
		M: Clone,
	{
		let mut stack = vec![Statement::Fact(fact)];

		while let Some(statement) = stack.pop() {
			match statement {
				Statement::Fact(fact) => {
					let (triple, g) = fact.quad.into_triple();
					if self
						.data
						.dependencies
						.filter(&self.interpretation, triple, fact.positive)?
					{
						let meta = fact.cause.metadata().clone();
						let (_, _, inserted) = self.data.set.insert(fact)?;

						if inserted {
							self.semantics.deduce(triple, |triple, positive| {
								let fact = dataset::Fact::new(
									triple.into_quad(g),
									positive,
									Cause::Entailed(meta.clone()),
								);
								stack.push(Statement::Fact(fact))
							});
						}
					}
				}
				Statement::Eq(a, b, g) => {
					let (a, b, removed_literal_instances) = self.interpretation.merge(a, b)?;
					self.data.set.replace_id(b, a, |fact| {
						self.data.dependencies.filter(
							&self.interpretation,
							fact.triple,
							fact.positive,
						)
					})?;
					stack.replace_id(b, a);

					for (value, id) in removed_literal_instances {
						if let Some(other_id) = self
							.interpretation
							.get_mut(a)
							.unwrap()
							.insert_lexical_value(value, id)
						{
							stack.push(Statement::Eq(id, other_id, g))
						}
					}

					for (d, _, fact) in self.data.resource_facts(&self.interpretation, a) {
						let triple = self.interpretation.import_triple(
							&self.data.dependencies.0,
							d,
							fact.triple(),
						);
						let meta = fact.cause.into_metadata();
						self.semantics.deduce(triple, |triple, positive| {
							let fact = dataset::Fact::new(
								triple.into_quad(g),
								positive,
								Cause::Entailed(meta.clone()),
							);
							stack.push(Statement::Fact(fact))
						});
					}
				}
				Statement::Neq(a, b) => {
					self.interpretation.split(a, b)?;
				}
			}
		}

		Ok(())
	}
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

pub struct Dependencies<V: Vocabulary, M>(HashMap<usize, Dependency<V, M>>);

impl<V: Vocabulary, M> Dependencies<V, M> {
	pub fn filter(
		&self,
		interpretation: &CompositeInterpretation<V>,
		triple: Triple,
		positive: bool,
	) -> Result<bool, dataset::Contradiction> {
		for (&d, dependency) in &self.0 {
			for dependency_triple in interpretation.dependency_triples(d, triple) {
				if let Some((_, _, fact)) = dependency.dataset.find_triple(dependency_triple) {
					if fact.positive == positive {
						return Ok(false);
					} else {
						return Err(dataset::Contradiction(triple));
					}
				}
			}
		}

		Ok(true)
	}

	pub fn iter(&self) -> impl Iterator<Item = (usize, &Dependency<V, M>)> {
		self.0.iter().map(|(i, d)| (*i, d))
	}
}

/// Iterator over all the facts about the given resource, and the dependency it comes from.
///
/// Facts are given in the dependency interpretation, not the top level interpretation.
pub struct ResourceFacts<'a, M> {
	id: Id,
	toplevel: dataset::ResourceFacts<'a, M>,
	dependencies: Vec<(usize, Id, dataset::ResourceFacts<'a, M>)>,
}

impl<'a, M> Iterator for ResourceFacts<'a, M> {
	type Item = (Option<usize>, Id, dataset::Fact<&'a M>);

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
