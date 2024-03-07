//! Deduction rules.
use std::hash::Hash;

use rdf_types::{
	generator,
	interpretation::{LiteralInterpretationMut, ReverseTermInterpretation},
	InterpretationMut, Quad, Term, VocabularyMut,
};
use serde::{Deserialize, Serialize};

mod conclusion;
mod hypothesis;

pub use conclusion::*;
pub use hypothesis::*;

use crate::{
	expression,
	pattern::{ApplyPartialSubstitution, PatternSubstitution, ResourceOrVar, TripleMatching},
	system::{Deduction, Deductions},
	utils::IteratorSearch,
	Entailment, FallibleSignedPatternMatchingDataset, Signed, SignedPatternMatchingDataset,
	Validation, ValidationError,
};

/// Deduction rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Rule<T = Term> {
	pub variables: usize,

	pub hypothesis: Hypothesis<T>,

	pub conclusion: Conclusion<T>,
}

impl<T> Rule<T> {
	pub fn new(variables: usize, hypothesis: Hypothesis<T>, conclusion: Conclusion<T>) -> Self {
		Self {
			variables,
			hypothesis,
			conclusion,
		}
	}
}

impl<T: Clone + Eq + Hash> Rule<T> {
	/// Deduces triples using this rule against the given dataset.
	///
	/// Returns all the `Deduction` instances representing each substitutions
	/// satisfying the rule's hypotheses. Each deduction also include the
	/// partially substituted conclusions.
	pub fn deduce<D>(&self, dataset: &D) -> Deductions<T>
	where
		D: SignedPatternMatchingDataset<Resource = T>,
	{
		self.try_deduce_from(dataset, PatternSubstitution::new(), None)
			.unwrap()
	}

	/// Deduces triples using this rule against the given dataset.
	///
	/// Returns all the `Deduction` instances representing each substitutions
	/// satisfying the rule's hypotheses. Each deduction also include the
	/// partially substituted conclusions.
	pub fn try_deduce<D>(&self, dataset: &D) -> Result<Deductions<T>, D::Error>
	where
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		self.try_deduce_from(dataset, PatternSubstitution::new(), None)
	}

	/// Deduces triples using this rule against the given dataset from the
	/// given `initial_substitution`.
	///
	/// Returns all the `Deduction` instances representing each substitutions
	/// derived from `initial_substitution` satisfying the rule's hypotheses,
	/// except `excluded_hypothesis` (if provided). Each deduction also include
	/// the partially substituted conclusions.
	pub fn try_deduce_from<D>(
		&self,
		dataset: &D,
		initial_substitution: PatternSubstitution<T>,
		excluded_hypothesis: Option<usize>,
	) -> Result<Deductions<T>, D::Error>
	where
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let substitutions = self.try_find_substitutions(
			dataset,
			&self.hypothesis,
			initial_substitution,
			excluded_hypothesis,
		)?;

		let mut deduction = Deductions::default();

		for substitution in substitutions {
			let mut d = Deduction::new(Entailment::new(self, substitution.to_vec()));

			for statement in &self.conclusion.statements {
				d.insert(statement.apply_partial_substitution(&substitution))
			}

			deduction.push(d);
		}

		Ok(deduction)
	}

	/// Validates the given dataset against this rule.
	///
	/// Returns `Validation::Ok` if and only if any triple deduced from the
	/// dataset is already in the dataset.
	pub fn validate_with<V, I, D>(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		dataset: &D,
	) -> Result<Validation<T>, expression::Error>
	where
		V: VocabularyMut,
		V::Iri: PartialEq,
		I: InterpretationMut<V, Resource = T>
			+ LiteralInterpretationMut<V::Literal>
			+ ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
		D: SignedPatternMatchingDataset<Resource = T>,
	{
		self.try_validate_with(vocabulary, interpretation, dataset)
			.map_err(Into::into)
	}

	/// Validates the given dataset against this rule.
	///
	/// Returns `Validation::Ok` if and only if any triple deduced from the
	/// dataset is already in the dataset.
	pub fn try_validate_with<V, I, D>(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		dataset: &D,
	) -> Result<Validation<T>, ValidationError<D::Error>>
	where
		V: VocabularyMut,
		V::Iri: PartialEq,
		I: InterpretationMut<V, Resource = T>
			+ LiteralInterpretationMut<V::Literal>
			+ ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let deductions = self.try_deduce(dataset).map_err(ValidationError::Dataset)?;
		if let Validation::Invalid(reason) =
			deductions.try_validate(vocabulary, interpretation, dataset)?
		{
			return Ok(Validation::Invalid(reason));
		}

		Ok(Validation::Ok)
	}

	fn try_find_substitutions<D>(
		&self,
		dataset: &D,
		hypothesis: &Hypothesis<T>,
		initial_substitution: PatternSubstitution<T>,
		excluded_pattern: Option<usize>,
	) -> Result<Vec<PatternSubstitution<T>>, D::Error>
	where
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let substitutions = {
			hypothesis
				.patterns
				.iter()
				.enumerate()
				.filter_map(|(i, pattern)| {
					if excluded_pattern == Some(i) {
						None
					} else {
						let canonical_pattern = pattern
							.as_ref()
							.map(|t| t.as_ref().map(ResourceOrVar::as_ref))
							.cast();

						Some(dataset.try_signed_pattern_matching(canonical_pattern).map(
							move |m: Result<Signed<Quad<&T>>, D::Error>| {
								m.map(|Signed(_, m)| (pattern, m.into_triple().0))
							},
						))
					}
				})
				.search(initial_substitution, |substitution, (pattern, m)| {
					let mut substitution = substitution.clone();
					if pattern
						.as_ref()
						.into_value()
						.triple_matching(&mut substitution, m)
					{
						Some(substitution)
					} else {
						None
					}
				})
				.collect::<Result<Vec<_>, _>>()?
		};

		Ok(substitutions)
	}
}

impl Rule {
	/// Validates the given dataset against this rule.
	///
	/// Returns `Validation::Ok` if and only if any triple deduced from the
	/// dataset is already in the dataset.
	pub fn validate<D>(&self, dataset: &D) -> Result<Validation, expression::Error>
	where
		D: SignedPatternMatchingDataset<Resource = Term>,
	{
		self.try_validate(dataset).map_err(Into::into)
	}

	/// Validates the given dataset against this rule.
	///
	/// Returns `Validation::Ok` if and only if any triple deduced from the
	/// dataset is already in the dataset.
	pub fn try_validate<D>(&self, dataset: &D) -> Result<Validation, ValidationError<D::Error>>
	where
		D: FallibleSignedPatternMatchingDataset<Resource = Term>,
	{
		let mut interpretation = rdf_types::interpretation::WithGenerator::new(
			(),
			generator::Blank::new_with_prefix("inferdf:validation".to_owned()),
		);

		self.try_validate_with(&mut (), &mut interpretation, dataset)
	}
}

/// Path to an rule's pattern hypothesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path {
	/// Rule index.
	pub rule: usize,

	/// Hypothesis index.
	pub pattern: usize,
}

impl Path {
	pub fn new(rule: usize, pattern: usize) -> Self {
		Self { rule, pattern }
	}
}
