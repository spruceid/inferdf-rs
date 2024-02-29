pub use crate::{
	pattern,
	rule::{Formula, Path, Rule, TripleStatement},
	MaybeTrusted,
};
use crate::{
	pattern::TripleMatching,
	rule,
	utils::{IteratorExt, IteratorSearch, OnceOrMore},
	Entailment, FactRef, FallibleSignedPatternMatchingDataset, Signed,
	SignedPatternMatchingDataset,
};
use educe::Educe;
use rdf_types::{
	dataset::FallibleDataset,
	interpretation::{
		fallible::TraversableFallibleInterpretation, FallibleInterpretation,
		TraversableInterpretation,
	},
	Triple,
};
use std::{collections::HashMap, hash::Hash};

use self::pattern::{ApplyPartialSubstitution, PatternSubstitution, ResourceOrVar};

mod deduction;
pub use deduction::*;

mod deduction_intstance;
pub use deduction_intstance::*;

/// Deduction system.
#[derive(Debug, Educe)]
#[educe(Default)]
pub struct System<T> {
	/// List of rules.
	rules: Vec<Rule<T>>,

	/// Map a rule to its unique index in `rules`.
	map: HashMap<Rule<T>, usize>,

	/// Maps each pattern of interest to its path(s) in the system.
	paths: pattern::BipolarMap<Path, T>,
}

impl<T> System<T> {
	/// Creates a new empty deduction system.
	pub fn new() -> Self {
		Self::default()
	}

	/// Returns the number of rules in the deduction system.
	pub fn len(&self) -> usize {
		self.rules.len()
	}

	/// Checks if the deduction system is empty.
	pub fn is_empty(&self) -> bool {
		self.rules.is_empty()
	}

	pub fn get(&self, i: usize) -> Option<&Rule<T>> {
		self.rules.get(i)
	}

	/// Inserts the given rule in the system.
	pub fn insert(&mut self, rule: Rule<T>) -> usize
	where
		T: Clone + Eq + Hash,
	{
		*self.map.entry(rule).or_insert_with_key(|rule| {
			let i = self.rules.len();
			self.rules.push(rule.clone());

			if let Some(implication) = rule.as_existential_implication() {
				for (p, pattern) in implication.hypothesis_patterns().enumerate() {
					self.paths.insert(pattern.clone().cast(), Path::new(i, p));
				}
			}

			i
		})
	}

	/// Returns an iterator over the rules of the system.
	pub fn iter(&self) -> std::slice::Iter<Rule<T>> {
		self.rules.iter()
	}

	/// Appends the `other` system to `self`.
	pub fn append(&mut self, other: Self)
	where
		T: Clone + Eq + Hash,
	{
		for rule in other {
			self.insert(rule);
		}
	}
}

impl<'a, T> IntoIterator for &'a System<T> {
	type IntoIter = std::slice::Iter<'a, Rule<T>>;
	type Item = &'a Rule<T>;

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<T> IntoIterator for System<T> {
	type IntoIter = std::vec::IntoIter<Rule<T>>;
	type Item = Rule<T>;

	fn into_iter(self) -> Self::IntoIter {
		self.rules.into_iter()
	}
}

type DeductionResult<T, I, D> = Result<
	Option<Deduction<T>>,
	DeductionError<<I as FallibleInterpretation>::Error, <D as FallibleDataset>::Error>,
>;

type SubstitutionSearchResult<T, I, D> = Result<
	Vec<PatternSubstitution<T>>,
	DeductionError<<I as FallibleInterpretation>::Error, <D as FallibleDataset>::Error>,
>;

#[derive(Debug, thiserror::Error)]
pub enum DeductionError<I, D> {
	#[error(transparent)]
	Dataset(D),

	#[error(transparent)]
	Interpretation(I),
}

impl<T: Clone + Eq + Hash> System<T> {
	/// Deduce new facts from the given triple.
	///
	/// This function only uses existential rules to deduce facts.
	pub fn deduce_from_triple<I, D>(
		&self,
		interpretation: &I,
		dataset: &D,
		triple: Signed<Triple<&T>>,
	) -> Deduction<T>
	where
		I: TraversableInterpretation<Resource = T>,
		D: SignedPatternMatchingDataset<Resource = T>,
	{
		self.try_deduce_from_triple(interpretation, dataset, triple)
			.unwrap()
	}

	/// Deduce new facts from the given triple.
	///
	/// This function only uses existential rules to deduce facts.
	pub fn try_deduce_from_triple<I, D>(
		&self,
		interpretation: &I,
		dataset: &D,
		triple: Signed<Triple<&T>>,
	) -> Result<Deduction<T>, DeductionError<I::Error, D::Error>>
	where
		I: TraversableFallibleInterpretation<Resource = T>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let mut deduction = Deduction::default();

		for &path in self.paths.get(triple) {
			if let Some(d) = self.try_deduce_from_path(interpretation, dataset, triple, path)? {
				deduction.merge_with(d)
			}
		}

		Ok(deduction)
	}

	/// Deduce new facts from universal rules.
	pub fn deduce_from_universal_rules<I, D>(
		&self,
		interpretation: &mut I,
		dataset: &D,
	) -> Deduction<T>
	where
		I: TraversableInterpretation<Resource = T>,
		D: SignedPatternMatchingDataset<Resource = T>,
	{
		self.try_deduce_from_universal_rules(interpretation, dataset)
			.unwrap()
	}

	/// Deduce new facts from universal rules.
	pub fn try_deduce_from_universal_rules<I, D>(
		&self,
		interpretation: &mut I,
		dataset: &D,
	) -> Result<Deduction<T>, DeductionError<I::Error, D::Error>>
	where
		I: TraversableFallibleInterpretation<Resource = T>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let mut deduction = Deduction::default();

		for rule in &self.rules {
			if !rule.is_existential() {
				if let Some(d) = self.try_deduce_from_rule(
					interpretation,
					dataset,
					rule,
					PatternSubstitution::new(),
				)? {
					deduction.merge_with(d)
				}
			}
		}

		Ok(deduction)
	}

	/// Deduce facts from the given rule path.
	fn try_deduce_from_path<I, D>(
		&self,
		interpretation: &I,
		dataset: &D,
		triple: Signed<Triple<&T>>,
		path: Path,
	) -> DeductionResult<T, I, D>
	where
		I: TraversableFallibleInterpretation<Resource = T>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let rule = self.get(path.rule).unwrap();
		let pattern = rule
			.as_existential_implication()
			.unwrap()
			.hypothesis_pattern(path.pattern)
			.unwrap();
		let mut substitution = pattern::PatternSubstitution::new();

		assert!(pattern
			.value()
			.triple_matching(&mut substitution, triple.into_value()));

		self.try_deduce_from_rule(interpretation, dataset, rule, substitution)
	}

	fn try_deduce_from_rule<I, D>(
		&self,
		interpretation: &I,
		dataset: &D,
		rule: &Rule<T>,
		substitution: PatternSubstitution<T>,
	) -> DeductionResult<T, I, D>
	where
		I: TraversableFallibleInterpretation<Resource = T>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		self.try_deduce_from_formula(
			interpretation,
			dataset,
			&rule.id,
			&rule.formula,
			substitution,
		)
	}

	fn try_deduce_from_formula<I, D>(
		&self,
		interpretation: &I,
		dataset: &D,
		rule_id: &T,
		formula: &rule::Formula<T>,
		substitution: PatternSubstitution<T>,
	) -> DeductionResult<T, I, D>
	where
		I: TraversableFallibleInterpretation<Resource = T>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		match formula {
			Formula::Exists(e) => {
				let new_substitutions = self.try_find_substitutions(
					interpretation,
					dataset,
					e.variables(),
					e.hypothesis(), // TODO &e.extended_hypothesis(), // use extended hypothesis to pre-filter variables.
					substitution,
					None,
				)?;

				let mut deduction = None;

				for s in new_substitutions {
					// TODO s.retain(|x| !new_substitutions.extended_variables().contains(x));
					if let Some(d) = self.try_deduce_from_formula(
						interpretation,
						dataset,
						rule_id,
						e.inner(),
						s,
					)? {
						deduction
							.get_or_insert_with(Deduction::default)
							.merge_with(d);
					}
				}

				Ok(deduction)
			}
			Formula::ForAll(a) => {
				let mut deduction =
					SubDeduction::new(Entailment::new(rule_id.clone(), substitution.to_vec()));

				let substitutions = self.try_find_substitutions(
					interpretation,
					dataset,
					&a.variables,
					&a.constraints,
					substitution,
					None,
				)?;

				for s in substitutions {
					match self.try_deduce_from_formula(
						interpretation,
						dataset,
						rule_id,
						&a.inner,
						s,
					)? {
						Some(d) => deduction.merge_with(d),
						None => return Ok(None),
					}
				}

				Ok(Some(deduction.into()))
			}
			Formula::Conclusion(conclusion) => {
				let mut deduction =
					SubDeduction::new(Entailment::new(rule_id.clone(), substitution.to_vec()));

				for statement in &conclusion.statements {
					deduction.insert(statement.apply_partial_substitution(&substitution))
				}

				Ok(Some(deduction.into()))
			}
		}
	}

	fn try_find_substitutions<I, D>(
		&self,
		interpretation: &I,
		dataset: &D,
		variables: &[rule::Variable],
		hypothesis: &rule::Hypothesis<T>,
		initial_substitution: PatternSubstitution<T>,
		excluded_pattern: Option<usize>,
	) -> SubstitutionSearchResult<T, I, D>
	where
		I: TraversableFallibleInterpretation<Resource = T>,
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

						Some(dataset.try_pattern_matching(canonical_pattern).map(
							move |m: Result<FactRef<T>, D::Error>| {
								m.map(|Signed(_, m)| (pattern, m))
									.map_err(DeductionError::Dataset)
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
				.try_flat_map(|s| {
					let r = if variables.iter().all(|x| s.contains(x.index)) {
						OnceOrMore::<
							Result<PatternSubstitution<T>, DeductionError<I::Error, D::Error>>,
							_,
						>::Once(Some(Ok(s)))
					} else {
						let continue_s = s.clone();
						OnceOrMore::More(
							variables
								.iter()
								.filter_map(move |x| {
									if s.contains(x.index) {
										None
									} else {
										Some(interpretation.try_resources().map(
											move |r: Result<&T, I::Error>| {
												r.map(|id| (x.index, id))
													.map_err(DeductionError::Interpretation)
											},
										))
									}
								})
								.search(continue_s, |substitution, (x, id)| {
									let mut substitution = substitution.clone();
									substitution.bind(x, id.clone());
									Some(substitution)
								}),
						)
					};

					r
				})
				.collect::<Result<Vec<_>, _>>()?
		};

		Ok(substitutions)
	}
}
