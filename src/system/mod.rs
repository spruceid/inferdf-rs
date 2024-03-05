pub use crate::{
	pattern,
	rule::{Path, Rule},
	Modal,
};
use crate::{
	pattern::TripleMatching, FallibleSignedPatternMatchingDataset, Signed,
	SignedPatternMatchingDataset, Validation, ValidationError,
};
use educe::Educe;
use rdf_types::{
	interpretation::{LiteralInterpretationMut, ReverseTermInterpretation},
	InterpretationMut, Term, Triple, VocabularyMut,
};
use std::{collections::HashMap, hash::Hash};

mod deduction;
pub use deduction::*;

mod deduction_intstance;
pub use deduction_intstance::*;

/// Deduction system.
#[derive(Debug, Educe)]
#[educe(Default)]
pub struct System<T = Term> {
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

			for (p, pattern) in rule.hypothesis.patterns.iter().enumerate() {
				self.paths.insert(pattern.clone().cast(), Path::new(i, p));
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

impl<T: Clone + Eq + Hash> System<T> {
	/// Deduce new facts from the given triple.
	///
	/// This function only uses existential rules to deduce facts.
	pub fn deduce_from_triple<D>(&self, dataset: &D, triple: Signed<Triple<&T>>) -> Deduction<T>
	where
		D: SignedPatternMatchingDataset<Resource = T>,
	{
		self.try_deduce_from_triple(dataset, triple).unwrap()
	}

	/// Deduce new facts from the given triple.
	///
	/// This function only uses existential rules to deduce facts.
	pub fn try_deduce_from_triple<D>(
		&self,
		dataset: &D,
		triple: Signed<Triple<&T>>,
	) -> Result<Deduction<T>, D::Error>
	where
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let mut deduction = Deduction::default();

		for &path in self.paths.get(triple) {
			if let Some(d) = self.try_deduce_from_path(dataset, triple, path)? {
				deduction.merge_with(d)
			}
		}

		Ok(deduction)
	}

	/// Deduce facts from the given rule path.
	fn try_deduce_from_path<D>(
		&self,
		dataset: &D,
		triple: Signed<Triple<&T>>,
		path: Path,
	) -> Result<Option<Deduction<T>>, D::Error>
	where
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let rule = self.get(path.rule).unwrap();
		let pattern = &rule.hypothesis.patterns[path.pattern];
		let mut substitution = pattern::PatternSubstitution::new();

		assert!(pattern
			.value()
			.triple_matching(&mut substitution, triple.into_value()));

		rule.try_deduce_from(dataset, substitution, Some(path.pattern))
	}

	/// Deduce new facts from the given triple.
	///
	/// This function only uses existential rules to deduce facts.
	pub fn try_validate<V, I, D>(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		dataset: &D,
	) -> Result<Validation, ValidationError<D::Error>>
	where
		V: VocabularyMut,
		V::Iri: PartialEq,
		I: InterpretationMut<V, Resource = T>
			+ LiteralInterpretationMut<V::Literal>
			+ ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		for rule in &self.rules {
			rule.try_validate_with(vocabulary, interpretation, dataset)?;
		}

		Ok(Validation::Ok)
	}
}
