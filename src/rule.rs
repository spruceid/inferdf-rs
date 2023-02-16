use hashbrown::{HashMap, HashSet};

use crate::{Id, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
	Variable(usize),
	Id(Id)
}

pub type Pattern = rdf_types::Triple<Term, Term, Term>;

pub trait Matching {
	fn matching(&self, t: crate::Triple) -> Option<PatternSubstitution>;
}

impl Matching for Pattern {
	fn matching(&self, t: crate::Triple) -> Option<PatternSubstitution> {
		todo!()
	}
}

pub struct PatternSubstitution;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rule {
	bindings: usize,
	conjunctions: Vec<Conjunction>
}

impl Rule {
	pub fn is_false(&self) -> bool {
		self.conjunctions.is_empty()
	}

	pub fn without_conjunction(&self, i: usize) -> Self {
		Self {
			bindings: self.bindings,
			conjunctions: self.conjunctions.iter().enumerate().filter_map(|(j, f)| {
				if i == j {
					None
				} else {
					Some(f.clone())
				}
			}).collect()
		}
	}

	pub fn apply_substitution(&mut self, s: PatternSubstitution) {
		todo!()
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Conjunction {
	bindings: usize,
	factors: Vec<Factor>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Factor {
	assert: bool,
	atom: Atom
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path {
	rule: RuleId,
	term: usize,
	factor: usize
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Atom {
	Positive(Pattern),
	Negative(Pattern)
}

impl Atom {
	pub fn is_negative(&self) -> bool {
		matches!(self, Self::Negative(_))
	}

	pub fn pattern(&self) -> Pattern {
		match self {
			Self::Positive(p) => *p,
			Self::Negative(p) => *p
		}
	}
}

pub struct Rules {
	rules: Vec<Rule>,
	map: HashMap<Rule, RuleId>,
	paths: HashMap<Pattern, HashSet<Path>>
}

impl Rules {
	pub fn get(&self, id: RuleId) -> Option<&Rule> {
		todo!()
	}

	/// Deduce new facts from the given triple.
	pub fn deduce(
		&self,
		triple: crate::Triple,
		result: &mut Vec<Rule>
	) {
		// triple.0
	}
}

impl Path {
	pub fn deduce(
		&self,
		rules: &Rules,
		triple: crate::Triple,
		result: &mut Vec<Rule>
	) {
		let rule = rules.get(self.rule).unwrap();
		let f = rule.conjunctions[self.term].factors[self.factor];
		if f.atom.is_negative() {
			if let Some(substitution) = f.atom.pattern().matching(triple) {
				let mut new_rule = rule.without_conjunction(self.term);
				if new_rule.is_false() {
					panic!("contradiction")
				} else {
					new_rule.apply_substitution(substitution);
					result.push(new_rule)
				}
			}
		}
	}
}

impl Rule {
	pub fn deduce_from_insertion<'d, M: 'd>(
		&self,
		context: impl Context<'d, M>,
		result: &mut Vec<(Rule, &'d M)>
	) {
		for (i, c) in self.conjunctions.iter().enumerate() {
			for f in &c.factors {
				if f.atom.is_negative() {
					for (triple, cause) in context.matches(f.atom.pattern()) {
						if let Some(new_rule) = deduce_from_pattern(triple, self, i, f.atom.pattern()) {
							result.push((new_rule, cause.metadata()))
						}
					}
				}
			}
		}
	}
}

fn deduce_from_pattern(
	triple: crate::Triple,
	rule: &Rule,
	conjunction_index: usize,
	pattern: Pattern
) -> Option<Rule> {
	if let Some(substitution) = pattern.matching(triple) {
		let mut new_rule = rule.without_conjunction(conjunction_index);
		if new_rule.is_false() {
			panic!("contradiction")
		} else {
			new_rule.apply_substitution(substitution);
			return Some(new_rule)
		}
	}

	None
}