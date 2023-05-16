pub mod rule;

use hashbrown::HashMap;
pub use rule::{Rule, Path};

use crate::{Triple, Signed, pattern::{self, Matching}, IteratorSearch};

use self::rule::Statement;

use super::Context;

/// Induction rules.
pub struct System {
	/// List of rules.
	rules: Vec<Rule>,

	/// Map a rule to ite unique index in `rules`.
	map: HashMap<Rule, usize>,

	/// Maps each pattern of interest to its path(s) in the system.
	paths: pattern::BipolarMap<Path>
}

impl System {
	pub fn get(&self, id: usize) -> Option<&Rule> {
		todo!()
	}

	/// Deduce new facts from the given triple.
	pub fn deduce(
		&self,
		context: &impl Context,
		triple: Signed<Triple>,
		mut f: impl FnMut(Signed<Statement>)
	) {
		for &path in self.paths.get(triple) {
			self.deduce_from(context, triple, path, &mut f)
		}
	}

	fn deduce_from(
		&self,
		context: &impl Context,
		triple: Signed<Triple>,
		path: Path,
		f: &mut impl FnMut(Signed<Statement>)
	) {
		let rule = self.get(path.rule).unwrap();
		let pattern = rule.hypothesis.patterns[path.pattern];
		let mut substitution = pattern::PatternSubstitution;
		assert!(pattern.value().matching(&mut substitution, triple.into_value()));

		let triples = rule.hypothesis.patterns.iter().copied().enumerate().filter_map(|(i, pattern)| {
			if i == path.pattern {
				None
			} else {
				Some(context.pattern_matching(pattern))
			}
		}).search(substitution, |substitution, m| {
			todo!()
		}).map(|substitution| {
			// create new blank nodes.
			rule.conclusion.statements.iter().map(|statement| {
				todo!()
			})
		}).flatten();

		for triple in triples {
			f(triple)
		}
	}
}