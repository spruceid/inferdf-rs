pub mod rule;

use hashbrown::HashMap;
pub use rule::{Path, Rule};

use crate::{
	pattern::{self, Instantiate, Matching},
	IteratorSearch, Signed, Triple,
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
		mut f: impl FnMut(Signed<TripleStatement>),
	) {
		for &path in self.paths.get(triple) {
			self.deduce_from(context, triple, path, &mut f)
		}
	}

	fn deduce_from(
		&self,
		context: &mut impl Context,
		triple: Signed<Triple>,
		path: Path,
		f: &mut impl FnMut(Signed<TripleStatement>),
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

				statements.into_iter()
			})
			.for_each(f)
	}
}

impl Semantics for System {
	fn deduce(
		&self,
		context: &mut impl Context,
		triple: Signed<Triple>,
		f: impl FnMut(Signed<TripleStatement>),
	) {
		self.deduce(context, triple, f)
	}
}
