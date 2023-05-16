use std::hash::Hash;

use derivative::Derivative;
use hashbrown::HashMap;
use locspan::Meta;

use crate::{
	dataset::{self, Dataset},
	interpretation::{self, CompositeInterpretation, Interpretation},
	semantics::Semantics,
	Cause, Id, Quad, ReplaceId, Sign, Signed, Triple, Vocabulary,
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
	pub fn new(dependencies: Dependencies<V, M>, semantics: S) -> Self {
		Self {
			interpretation: CompositeInterpretation::new(),
			data: Data {
				set: Dataset::new(),
				dependencies,
			},
			semantics,
		}
	}
}

impl<V: Vocabulary, M, S: Semantics> Builder<V, M, S> {
	pub fn insert(
		&mut self,
		Meta(Signed(sign, quad), cause): dataset::Fact<Cause<M>>,
	) -> Result<(), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
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
							self.semantics.deduce(
								&self.data.set,
								Signed(sign, triple),
								|| self.interpretation.new_resource(),
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
					let (a, b, removed_literal_instances) = self.interpretation.merge(a, b)?;
					self.data
						.set
						.replace_id(b, a, |Meta(Signed(sign, triple), _)| {
							self.data
								.dependencies
								.filter(&self.interpretation, *triple, *sign)
						})?;
					stack.replace_id(b, a);

					for (value, id) in removed_literal_instances {
						if let Some(other_id) = self
							.interpretation
							.get_mut(a)
							.unwrap()
							.insert_lexical_value(value, id)
						{
							stack.push(Meta(
								Signed(sign, QuadStatement::Eq(id, other_id, g)),
								Cause::Entailed(cause.metadata().clone()),
							))
						}
					}

					for (d, _, Meta(Signed(sign, quad), cause)) in
						self.data.resource_facts(&self.interpretation, a)
					{
						let triple = self.interpretation.import_triple(
							&self.data.dependencies.0,
							d,
							quad.into_triple().0,
						);

						let meta = cause.metadata();
						self.semantics.deduce(
							&self.data.set,
							Signed(sign, triple),
							|| self.interpretation.new_resource(),
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
						let facts = d.dataset.resource_facts(local_id);
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

pub struct Dependency<V: Vocabulary, M> {
	interpretation: Interpretation<V>,
	dataset: Dataset<Cause<M>>,
}

impl<V: Vocabulary, M> crate::interpretation::Dependency<V> for Dependency<V, M> {
	fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Dependencies<V: Vocabulary, M>(HashMap<usize, Dependency<V, M>>);

impl<V: Vocabulary, M> Dependencies<V, M> {
	pub fn filter(
		&self,
		interpretation: &CompositeInterpretation<V>,
		triple: Triple,
		sign: Sign,
	) -> Result<bool, dataset::Contradiction> {
		for (&d, dependency) in &self.0 {
			for dependency_triple in interpretation.dependency_triples(d, triple) {
				if let Some((_, _, fact)) = dependency.dataset.find_triple(dependency_triple) {
					if fact.sign() == sign {
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
