pub mod rule;

use std::{cell::RefCell, hash::Hash};

use educe::Educe;
use hashbrown::HashMap;
use locspan::Meta;
use rdf_types::{IriVocabularyMut, Vocabulary};
pub use rule::{Path, Rule};

use inferdf::{
	module::sub_module::ResourceGenerator,
	pattern::{self, Instantiate, Matching, PatternSubstitution},
	semantics::{Context, ContextReservation, MaybeTrusted, Semantics, TripleStatement},
	Cause, Entailment, Fact, Id, IteratorSearch, IteratorWith, Signed, Triple, TryCollectWith,
};

use rule::Formula;

/// Induction rules.
#[derive(Debug, Educe)]
#[educe(Default)]
pub struct System<T = Id> {
	/// List of rules.
	rules: Vec<Rule<T>>,

	/// Map a rule to its unique index in `rules`.
	map: HashMap<Rule<T>, usize>,

	/// Maps each pattern of interest to its path(s) in the system.
	paths: pattern::BipolarMap<Path, T>,
}

impl<T> System<T> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn len(&self) -> usize {
		self.rules.len()
	}

	pub fn is_empty(&self) -> bool {
		self.rules.is_empty()
	}

	pub fn get(&self, i: usize) -> Option<&Rule<T>> {
		self.rules.get(i)
	}

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

	pub fn iter(&self) -> std::slice::Iter<Rule<T>> {
		self.rules.iter()
	}

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

#[derive(Default)]
pub struct Deduction(Vec<SubDeduction>);

impl Deduction {
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn merge_with(&mut self, other: Self) {
		self.0.extend(other.0)
	}

	pub fn collect(
		self,
		mut entailment_index: impl FnMut(Entailment) -> u32,
		mut new_triple: impl FnMut(Meta<MaybeTrusted<Signed<TripleStatement>>, Cause>),
	) {
		for s in self.0 {
			let e = entailment_index(s.entailment);
			for statement in s.statements {
				new_triple(Meta(statement, Cause::Entailed(e)))
			}
		}
	}
}

impl From<SubDeduction> for Deduction {
	fn from(value: SubDeduction) -> Self {
		Self(vec![value])
	}
}

pub struct SubDeduction {
	pub entailment: Entailment,
	pub statements: Vec<MaybeTrusted<Signed<TripleStatement>>>,
}

impl SubDeduction {
	pub fn new(entailment: Entailment) -> Self {
		Self {
			entailment,
			statements: Vec::new(),
		}
	}

	pub fn insert(&mut self, statement: MaybeTrusted<Signed<TripleStatement>>) {
		self.statements.push(statement)
	}

	pub fn merge_with(&mut self, other: Deduction) {
		for s in other.0 {
			self.statements.extend(s.statements)
		}
	}
}

impl System {
	/// Deduce new facts from the given triple.
	pub fn deduce_from_triple<V: Vocabulary, C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		triple: Signed<Triple>,
	) -> Result<Deduction, C::Error>
	where
		V: IriVocabularyMut,
		V::Value: AsRef<str>,
	{
		let mut deduction = Deduction::default();

		for &path in self.paths.get(&triple) {
			let d = self.deduce_from_path(vocabulary, context, triple, path)?;

			deduction.merge_with(d)
		}

		Ok(deduction)
	}

	fn deduce_from_universal_rules<V: Vocabulary, C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
	) -> Result<Deduction, C::Error>
	where
		V: IriVocabularyMut,
		V::Value: AsRef<str>,
	{
		let mut deduction = Deduction::default();

		for rule in &self.rules {
			if !rule.is_existential() {
				let d =
					self.deduce_from_rule(vocabulary, context, rule, PatternSubstitution::new())?;
				deduction.merge_with(d)
			}
		}

		Ok(deduction)
	}

	fn deduce_from_path<V: Vocabulary, C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		triple: Signed<Triple>,
		path: Path,
	) -> Result<Deduction, C::Error>
	where
		V: IriVocabularyMut,
		V::Value: AsRef<str>,
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
			.matching(&mut substitution, triple.into_value()));

		self.deduce_from_rule(vocabulary, context, rule, substitution)
	}

	fn deduce_from_rule<V: Vocabulary, C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		rule: &Rule,
		substitution: PatternSubstitution,
	) -> Result<Deduction, C::Error>
	where
		V: IriVocabularyMut,
		V::Value: AsRef<str>,
	{
		self.deduce_from_formula(vocabulary, context, rule.id, &rule.formula, substitution)
	}

	fn deduce_from_formula<V: Vocabulary, C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		rule_id: Id,
		formula: &rule::Formula,

		mut substitution: PatternSubstitution,
	) -> Result<Deduction, C::Error>
	where
		V: IriVocabularyMut,
		V::Value: AsRef<str>,
	{
		match formula {
			Formula::Exists(e) => {
				let new_substitutions = self.find_substitutions(
					vocabulary,
					context,
					e.variables(),
					e.hypothesis(), // TODO &e.extended_hypothesis(), // use extended hypothesis to pre-filter variables.
					substitution,
					None,
				)?;

				let mut deduction = Deduction::default();

				for s in new_substitutions {
					// TODO s.retain(|x| !new_substitutions.extended_variables().contains(x));
					let d = self.deduce_from_formula(vocabulary, context, rule_id, e.inner(), s)?;
					deduction.merge_with(d);
				}

				Ok(deduction)
			}
			Formula::ForAll(a) => {
				let mut deduction =
					SubDeduction::new(Entailment::new(rule_id, substitution.to_vec()));

				let substitutions = self.find_substitutions(
					vocabulary,
					context,
					&a.variables,
					&a.constraints,
					substitution,
					None,
				)?;

				for s in substitutions {
					let d = self.deduce_from_formula(vocabulary, context, rule_id, &a.inner, s)?;
					if d.is_empty() {
						// TODO `d` might be empty even if the conclusion is reached.
						return Ok(Deduction::default());
					} else {
						deduction.merge_with(d)
					}
				}

				Ok(deduction.into())
			}
			Formula::Conclusion(conclusion) => {
				let mut deduction =
					SubDeduction::new(Entailment::new(rule_id, substitution.to_vec()));

				for x in &conclusion.variables {
					substitution.bind(x.index, context.new_resource());
				}

				for statement in &conclusion.statements {
					if let Some(s) = statement.instantiate(&substitution) {
						deduction.insert(s)
					}
				}

				Ok(deduction.into())
			}
		}
	}

	fn find_substitutions<V: Vocabulary, C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		variables: &[rule::Variable],
		hypothesis: &rule::Hypothesis,
		initial_substitution: PatternSubstitution,
		excluded_pattern: Option<usize>,
	) -> Result<Vec<PatternSubstitution>, C::Error> {
		let reservation = RefCell::new(context.begin_reservation());

		struct Generator<'c, 'a, V: Vocabulary, C: 'c + Context<V>>(
			&'a RefCell<C::Reservation<'c>>,
		);

		impl<'c, 'a, V: Vocabulary, C: 'c + Context<V>> ResourceGenerator for Generator<'c, 'a, V, C> {
			fn new_resource(&mut self) -> Id {
				let mut r = self.0.borrow_mut();
				r.new_resource()
			}
		}

		let substitutions = {
			hypothesis
				.patterns
				.iter()
				.copied()
				.enumerate()
				.filter_map(|(i, pattern)| {
					if excluded_pattern == Some(i) {
						None
					} else {
						Some(
							context
								.pattern_matching(Generator::<V, C>(&reservation), pattern.cast())
								.map(move |m: Result<(Fact, bool), C::Error>| {
									m.map(|(Meta(Signed(_, m), _), _)| (pattern, m))
								}),
						)
					}
				})
				.search(initial_substitution, |substitution, (pattern, m)| {
					let mut substitution = substitution.clone();
					if pattern
						.into_value()
						.matching(&mut substitution, m.into_triple().0)
					{
						Some(substitution)
					} else {
						None
					}
				})
				.try_flat_map(|s| {
					let r = if variables.iter().all(|x| s.contains(x.index)) {
						OnceOrMore::<Result<PatternSubstitution, C::Error>, _>::Once(Some(Ok(s)))
					} else {
						let continue_s = s.clone();
						let context = &*context;
						let reservation = &reservation;
						OnceOrMore::more::<V>(
							variables
								.iter()
								.filter_map(move |x| {
									if s.contains(x.index) {
										None
									} else {
										Some(context.resources(Generator::<V, C>(reservation)).map(
											move |r: Result<Id, C::Error>| {
												r.map(|id| (x.index, id))
											},
										))
									}
								})
								.search(continue_s, |substitution, (x, id)| {
									let mut substitution = substitution.clone();
									substitution.bind(x, id);
									Some(substitution)
								}),
						)
					};

					r
				})
				.try_collect_with(vocabulary)?
		};

		// for s in substitutions {
		// 	variables
		// 		.iter()
		// 		.filter_map(|x| {
		// 			if s.contains(x) {
		// 				None
		// 			} else {
		// 				Some(context
		// 					.resources(Generator::<V, C>(&reservation))
		// 					.map(move |r: Result<Id, C::Error>| r.map(|id| (x.index, id))))
		// 			}
		// 		})
		// 		.search(initial_substitution, |substitution, (x, id)| {
		// 			let mut substitution = substitution.clone();
		// 			substitution.bind(x, id);
		// 			Some(substitution)
		// 		})
		// 		.try_collect_with(vocabulary)?
		// }

		let reservation = reservation.into_inner().end();
		context.apply_reservation(reservation);

		Ok(substitutions)
	}
}

impl<V: Vocabulary> Semantics<V> for System
where
	V: IriVocabularyMut,
	V::Value: AsRef<str>,
{
	fn deduce<C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		triple: Signed<Triple>,
		entailment_index: impl FnMut(Entailment) -> u32,
		new_triple: impl FnMut(Meta<MaybeTrusted<Signed<TripleStatement>>, Cause>),
	) -> Result<(), C::Error> {
		let deduction = self.deduce_from_triple(vocabulary, context, triple)?;
		deduction.collect(entailment_index, new_triple);
		Ok(())
	}

	fn close<C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		entailment_index: impl FnMut(Entailment) -> u32,
		new_triple: impl FnMut(Meta<MaybeTrusted<Signed<TripleStatement>>, Cause>),
	) -> Result<(), C::Error> {
		let deduction = self.deduce_from_universal_rules(vocabulary, context)?;
		deduction.collect(entailment_index, new_triple);
		Ok(())
	}
}

enum OnceOrMore<T, I> {
	Once(Option<T>),
	More(I),
}

impl<T, I> OnceOrMore<T, I> {
	pub fn more<V>(i: I) -> Self
	where
		I: IteratorWith<V, Item = T>,
	{
		Self::More(i)
	}
}

impl<T, I: IteratorWith<V, Item = T>, V> IteratorWith<V> for OnceOrMore<T, I> {
	type Item = T;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		match self {
			Self::Once(t) => t.take(),
			Self::More(i) => i.next_with(vocabulary),
		}
	}
}
