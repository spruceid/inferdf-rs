pub mod rule;

use hashbrown::HashMap;
use locspan::Meta;
pub use rule::{Path, Rule};

use inferdf_core::{
	pattern::{self, Instantiate, Matching},
	Cause, Entailment, IteratorSearch, Signed, Triple,
};

use self::rule::TripleStatement;

use super::{Context, Semantics};

/// Induction rules.
#[derive(Debug, Default)]
pub struct System {
	/// List of rules.
	rules: Vec<Rule>,

	/// Map a rule to ite unique index in `rules`.
	map: HashMap<Rule, usize>,

	/// Maps each pattern of interest to its path(s) in the system.
	paths: pattern::BipolarMap<Path>,
}

impl System {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get(&self, id: usize) -> Option<&Rule> {
		self.rules.get(id)
	}

	pub fn insert(&mut self, rule: Rule) -> usize {
		*self.map.entry(rule).or_insert_with_key(|rule| {
			let i = self.rules.len();
			self.rules.push(rule.clone());

			for (p, pattern) in rule.hypothesis.patterns.iter().enumerate() {
				self.paths.insert(pattern.cast(), Path::new(i, p));
			}

			i
		})
	}

	/// Deduce new facts from the given triple.
	pub fn deduce(
		&self,
		context: &mut impl Context,
		triple: Signed<Triple>,
		mut entailment_index: impl FnMut(Entailment) -> u32,
		mut new_triple: impl FnMut(Meta<Signed<TripleStatement>, Cause>),
	) {
		for &path in self.paths.get(triple) {
			self.deduce_from(
				context,
				triple,
				path,
				&mut entailment_index,
				&mut new_triple,
			)
		}
	}

	fn deduce_from(
		&self,
		context: &mut impl Context,
		triple: Signed<Triple>,
		path: Path,
		entailment_index: &mut impl FnMut(Entailment) -> u32,
		new_triple: &mut impl FnMut(Meta<Signed<TripleStatement>, Cause>),
	) {
		let rule = self.get(path.rule).unwrap();
		let pattern = rule.hypothesis.patterns[path.pattern];
		let mut substitution = pattern::PatternSubstitution::new();
		assert!(pattern
			.value()
			.matching(&mut substitution, triple.into_value()));

		let substitutions: Vec<_> = rule
			.hypothesis
			.patterns
			.iter()
			.copied()
			.enumerate()
			.filter_map(|(i, pattern)| {
				if i == path.pattern {
					None
				} else {
					Some(
						context
							.pattern_matching(pattern.cast())
							.map(move |m| (pattern, m)),
					)
				}
			})
			.search(substitution, |substitution, (pattern, m)| {
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
			.collect();

		substitutions
			.into_iter()
			.flat_map(|mut substitution| {
				// create new blank nodes.
				let statements: Vec<_> = rule
					.conclusion
					.statements
					.iter()
					.map(|statement| {
						statement.instantiate(&mut substitution, || context.new_resource())
					})
					.collect();

				let cause = Cause::Entailed(entailment_index(Entailment::new(
					rule.id,
					substitution.into_vec(),
				)));

				statements.into_iter().map(move |s| Meta(s, cause))
			})
			.for_each(new_triple)
	}
}

impl Semantics for System {
	fn deduce(
		&self,
		context: &mut impl Context,
		triple: Signed<Triple>,
		entailment_index: impl FnMut(Entailment) -> u32,
		new_triple: impl FnMut(Meta<Signed<TripleStatement>, Cause>),
	) {
		self.deduce(context, triple, entailment_index, new_triple)
	}
}
