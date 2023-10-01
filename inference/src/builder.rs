use indexmap::IndexSet;
use locspan::Meta;
use rdf_types::Vocabulary;
use std::hash::Hash;

use inferdf_core::{
	dataset,
	interpretation::{self, InterpretationMut},
	module::{sub_module::SubModuleError, SubModule},
	uninterpreted, Cause, Entailment, Fact, Id, IteratorWith, Module, Quad, ReplaceId, Sign,
	Signed,
};

use crate::semantics::{inference::rule::TripleStatement, MaybeTrusted, Semantics};

mod class;
mod context;
// mod dependency;

pub use context::*;
// pub use dependency::*;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
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

pub struct BuilderInterpretation<V: Vocabulary, D> {
	dependency: SubModule<V, D>,
	interpretation: interpretation::Local<V>,
}

impl<V: Vocabulary, D> BuilderInterpretation<V, D> {
	pub fn new(dependency: D) -> Self {
		Self {
			dependency: SubModule::new(dependency),
			interpretation: interpretation::Local::new(),
		}
	}

	pub fn into_builder<S>(self, semantics: S) -> Builder<V, D, S> {
		Builder {
			dependency: self.dependency,
			interpretation: self.interpretation,
			dataset: dataset::LocalDataset::new(),
			semantics,
			entailments: IndexSet::new(),
			to_check: Vec::new(),
		}
	}
}

fn insert_term<V: Vocabulary, D: Module<V>>(
	vocabulary: &mut V,
	interpretation: &mut interpretation::Local<V>,
	dependency: &SubModule<V, D>,
	term: uninterpreted::Term<V>,
) -> Result<Id, D::Error>
where
	V::Iri: Clone + Eq + Hash,
	V::BlankId: Clone + Eq + Hash,
	V::Literal: Clone + Eq + Hash,
{
	use inferdf_core::Interpretation;
	if term.is_blank() {
		Ok(interpretation.insert_term(term))
	} else {
		match dependency
			.module()
			.interpretation()
			.term_interpretation(vocabulary, term.clone())?
		{
			Some(dep_id) => {
				let global_id = dependency
					.interface()
					.get_or_insert_global(dep_id, || interpretation.insert_term(term.clone()));
				interpretation.get_mut(global_id).unwrap().add_term(term);
				Ok(global_id)
			}
			None => Ok(interpretation.insert_term(term)),
		}
	}
}

impl<'a, V: Vocabulary, D: Module<V>> InterpretationMut<'a, V> for BuilderInterpretation<V, D>
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
		insert_term(vocabulary, &mut self.interpretation, &self.dependency, term)
	}
}

pub struct Builder<V: Vocabulary, D, S> {
	dependency: SubModule<V, D>,
	interpretation: interpretation::Local<V>,
	dataset: dataset::LocalDataset,
	semantics: S,
	entailments: IndexSet<Entailment>,
	to_check: Vec<Meta<Signed<TripleStatement>, u32>>,
}

impl<V: Vocabulary, D, S> Builder<V, D, S> {
	pub fn new(dependency: D, semantics: S) -> Self {
		Self {
			dependency: SubModule::new(dependency),
			interpretation: interpretation::Local::new(),
			dataset: dataset::LocalDataset::new(),
			semantics,
			entailments: IndexSet::new(),
			to_check: Vec::new(),
		}
	}

	pub fn local_interpretation(&self) -> &interpretation::Local<V> {
		&self.interpretation
	}

	pub fn local_dataset(&self) -> &dataset::LocalDataset {
		&self.dataset
	}

	pub fn entailment(&self, i: u32) -> Option<&Entailment> {
		self.entailments.get_index(i as usize)
	}

	pub fn entailments(&self) -> impl Iterator<Item = (u32, &Entailment)> {
		self.entailments
			.iter()
			.enumerate()
			.map(|(i, e)| (i as u32, e))
	}

	pub fn check(&mut self, vocabulary: &mut V) -> Result<(), MissingStatement>
	where
		V::Literal: Clone,
		D: Module<V>,
	{
		let mut context =
			BuilderContext::new(&self.dependency, &mut self.interpretation, &self.dataset);
		let mut reservation = context.begin_reservation();

		for Meta(Signed(sign, statement), cause) in std::mem::take(&mut self.to_check) {
			match statement {
				TripleStatement::Triple(triple) => {
					if context
						.pattern_matching(&mut reservation, Signed(sign, triple.into()))
						.next_with(vocabulary)
						.is_none()
					{
						return Err(MissingStatement(Signed(sign, statement), cause));
					}
				}
				TripleStatement::Eq(_, _) => {
					todo!()
				}
			}
		}

		context.apply_reservation(reservation.end());

		Ok(())
	}
}

#[derive(Debug, thiserror::Error)]
#[error("missing statement")]
pub struct MissingStatement(pub Signed<TripleStatement>, pub u32);

impl<'a, V: Vocabulary, D: Module<V>, S: Semantics<V>> InterpretationMut<'a, V> for Builder<V, D, S>
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

impl<V: Vocabulary, D: Module<V>, S: Semantics<V>> Builder<V, D, S> {
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
		use inferdf_core::Interpretation;
		if term.is_blank() {
			Ok(self.interpretation.insert_term(term))
		} else {
			match self
				.dependency
				.module()
				.interpretation()
				.term_interpretation(vocabulary, term)?
			{
				Some(dep_id) => {
					let global_id = self
						.dependency
						.interface()
						.get_or_insert_global(dep_id, || self.interpretation.insert_term(term));
					self.interpretation
						.get_mut(global_id)
						.unwrap()
						.add_term(term);
					Ok(global_id)
				}
				None => Ok(self.interpretation.insert_term(term)),
			}
		}
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
		Ok(Quad::new(
			self.insert_term(vocabulary, quad.0)?,
			self.insert_term(vocabulary, quad.1)?,
			self.insert_term(vocabulary, quad.2)?,
			quad.3
				.map(|g| self.insert_term(vocabulary, g))
				.transpose()?,
		))
	}
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum Error<D> {
	Contradiction(Contradiction),
	Dependency(D),
}

impl<D> From<dataset::Contradiction> for Error<D> {
	fn from(value: dataset::Contradiction) -> Self {
		Self::Contradiction(value.into())
	}
}

impl<D> From<interpretation::Contradiction> for Error<D> {
	fn from(value: interpretation::Contradiction) -> Self {
		Self::Contradiction(value.into())
	}
}

impl<D> From<SubModuleError<D>> for Error<D> {
	fn from(value: SubModuleError<D>) -> Self {
		match value {
			SubModuleError::Module(e) => Error::Dependency(e),
			SubModuleError::Contradiction(c) => Error::Contradiction(c.into()),
		}
	}
}

impl<V: Vocabulary, D: Module<V>, S: Semantics<V>> Builder<V, D, S> {
	/// Insert a new quad in the module's dataset.
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

		while let Some(statement) = stack.pop() {
			self.insert_deduced_statement(vocabulary, &mut stack, statement)?;
		}

		Ok(())
	}

	/// Close the dataset.
	pub fn close(&mut self, vocabulary: &mut V) -> Result<(), Error<D::Error>>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		let mut stack = Vec::new();

		self.close_further(vocabulary, &mut stack)?;

		while !stack.is_empty() {
			while let Some(statement) = stack.pop() {
				self.insert_deduced_statement(vocabulary, &mut stack, statement)?;
			}

			if !stack.is_empty() {
				self.close_further(vocabulary, &mut stack)?;
			}
		}

		Ok(())
	}

	fn insert_deduced_statement(
		&mut self,
		vocabulary: &mut V,
		stack: &mut Vec<Meta<Signed<QuadStatement>, Cause>>,
		Meta(statement, cause): Meta<Signed<QuadStatement>, Cause>,
	) -> Result<(), Error<D::Error>>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		match statement {
			Signed(sign, QuadStatement::Quad(quad)) => {
				let (triple, g) = quad.into_triple();
				if self.dependency.filter_triple(vocabulary, triple, sign)? {
					let (_, _, inserted) = self.dataset.insert(Meta(Signed(sign, quad), cause))?;

					if inserted {
						let mut context = BuilderContext::new(
							&self.dependency,
							&mut self.interpretation,
							&self.dataset,
						);
						self.semantics
							.deduce(
								vocabulary,
								&mut context,
								Signed(sign, triple),
								|e| self.entailments.insert_full(e).0 as u32,
								|Meta(statement, cause)| match statement {
									MaybeTrusted::Trusted(Signed(sign, statement)) => stack
										.push(Meta(Signed(sign, statement.with_graph(g)), cause)),
									MaybeTrusted::Untrusted(signed_statement) => {
										self.to_check.push(Meta(
											signed_statement,
											cause.into_entailed().unwrap(),
										))
									}
								},
							)
							.map_err(Error::Dependency)?;
					}
				}
			}
			Signed(Sign::Positive, QuadStatement::Eq(a, b, _)) => {
				let (a, b) = self.interpretation.merge(a, b)?;
				self.dataset
					.replace_id(b, a, |Meta(Signed(sign, triple), _)| {
						self.dependency.filter_triple(vocabulary, *triple, *sign)
					})?;
				stack.replace_id(b, a);

				for signed_quad in self.dataset.resource_facts(a) {
					let Meta(Signed(sign, quad), cause) = signed_quad;
					stack.push(Meta(Signed(sign, QuadStatement::Quad(quad)), cause));
				}
			}
			Signed(Sign::Negative, QuadStatement::Eq(a, b, _)) => {
				self.interpretation.split(a, b)?;
			}
		}

		Ok(())
	}

	fn close_further(
		&mut self,
		vocabulary: &mut V,
		stack: &mut Vec<Meta<Signed<QuadStatement>, Cause>>,
	) -> Result<(), Error<D::Error>>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		let mut context =
			BuilderContext::new(&self.dependency, &mut self.interpretation, &self.dataset);

		self.semantics
			.close(
				vocabulary,
				&mut context,
				|e| self.entailments.insert_full(e).0 as u32,
				|Meta(statement, cause)| match statement {
					MaybeTrusted::Trusted(Signed(sign, statement)) => {
						stack.push(Meta(Signed(sign, statement.with_graph(None)), cause))
					}
					MaybeTrusted::Untrusted(signed_statement) => self
						.to_check
						.push(Meta(signed_statement, cause.into_entailed().unwrap())),
				},
			)
			.map_err(Error::Dependency)?;

		Ok(())
	}
}
