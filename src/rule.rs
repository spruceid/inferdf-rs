use hashbrown::{HashMap, HashSet};

use crate::{Id, Context};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Term {
	Variable(usize),
	Id(Id)
}

pub type Pattern = rdf_types::Triple<Term, Term, Term>;

pub type StatementPattern = rdf_types::Triple<Term, SignedVerb, Term>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SignedVerb {
	Positive(Verb),
	Negative(Verb)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verb {
	Equality,
	Term(Term)
}

/// Inference rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rule {
	hypothesis: Hypothesis,
	conclusion: Conclusion
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hypothesis {
	variables_count: usize,
	patterns: Vec<Pattern>
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Conclusion {
	variables_count: usize,
	statements: Vec<Statement>
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Statement {
	pattern: StatementPattern,
	asserted: bool
}

pub trait Matching {
	fn matching(&self, substitution: &mut PatternSubstitution, t: crate::Triple) -> bool;
}

impl Matching for Pattern {
	fn matching(&self, substitution: &mut PatternSubstitution, t: crate::Triple) -> bool {
		todo!()
	}
}

pub struct PatternSubstitution;

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
	rule: usize,
	pattern: usize,
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
	map: HashMap<Rule, usize>,
	paths: HashMap<Pattern, HashSet<Path>>
}

impl Rules {
	pub fn get(&self, id: usize) -> Option<&Rule> {
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
	pub fn deduce<'d, M: 'd>(
		&self,
		context: &impl Context<'d, M>,
		rules: &Rules,
		mut substitution: PatternSubstitution,
		result: &mut Vec<crate::Triple>
	) {
		let rule = rules.get(self.rule).unwrap();
		let pattern = rule.hypothesis.patterns[self.pattern];

		result.extend(rule.hypothesis.patterns.iter().copied().enumerate().filter_map(|(i, pattern)| {
			if i == self.rule {
				None
			} else {
				Some(context.matches(pattern))
			}
		}).search(substitution, |substitution, m| {
			todo!()
		}).map(|substitution| {
			// create new blank nodes.
			rule.conclusion.statements.iter().map(|statement| {
				todo!()
			})
		}).flatten())
	}
}

pub trait IteratorSearch<T, F: Fn(T, <Self::Item as Iterator>::Item) -> T>: Sized + Iterator where Self::Item: Iterator {
	fn search(self, initial_value: T, f: F) -> Search<Self, T, F>;
}

impl<I: Sized + Iterator, T, F: Fn(T, <Self::Item as Iterator>::Item) -> T> IteratorSearch<T, F> for I where I::Item: Iterator {
	fn search(self, initial_value: T, f: F) -> Search<Self, T, F> {
		todo!()
	}
}

pub struct Search<I, T, F> {
	iter: I,
	value: T,
	f: F
}

impl<I, T, F> Iterator for Search<I, T, F> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
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