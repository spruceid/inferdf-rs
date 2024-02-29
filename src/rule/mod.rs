use serde::{Deserialize, Serialize};

mod conclusion;
mod hypothesis;

pub use conclusion::*;
pub use hypothesis::*;

use crate::{Pattern, Signed};

/// Deduction rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Rule<T> {
	/// Rule identifier.
	pub id: T,

	/// Formula.
	pub formula: Formula<T>,
}

impl<T> Rule<T> {
	/// Checks if this formula does not contain any universal quantifier.
	pub fn is_existential(&self) -> bool {
		self.formula.is_fully_existential()
	}

	pub fn as_existential_implication(&self) -> Option<ExistentialImplication<T>> {
		self.formula
			.existential_implication_conclusion()
			.map(|conclusion| ExistentialImplication {
				formula: &self.formula,
				conclusion,
			})
	}
}

/// Formula variable description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Variable {
	/// Index of the variable.
	pub index: usize,

	/// Optional variable name.
	pub name: Option<String>,
}

/// Universally bound formula.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ForAll<T> {
	pub variables: Vec<Variable>,
	pub constraints: Hypothesis<T>,
	pub inner: Box<Formula<T>>,
}

/// Existentially bound formula.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Exists<T> {
	variables: Vec<Variable>,
	hypothesis: Hypothesis<T>,
	inner: Box<Formula<T>>,
}

impl<T> Exists<T> {
	pub fn new(variables: Vec<Variable>, hypothesis: Hypothesis<T>, inner: Formula<T>) -> Self {
		Self {
			variables,
			hypothesis,
			inner: Box::new(inner),
		}
	}

	pub fn variables(&self) -> &[Variable] {
		&self.variables
	}

	pub fn hypothesis(&self) -> &Hypothesis<T> {
		&self.hypothesis
	}

	pub fn inner(&self) -> &Formula<T> {
		&self.inner
	}

	pub fn extend_variables(&mut self, v: impl IntoIterator<Item = Variable>) {
		self.variables.extend(v);
		self.variables.sort_unstable_by_key(|x| x.index)
	}

	fn hypothesis_pattern_from(&self, i: usize) -> Option<&Signed<Pattern<T>>> {
		if i < self.hypothesis.patterns.len() {
			self.hypothesis.patterns.get(i)
		} else {
			self.inner
				.hypothesis_pattern_from(i - self.hypothesis.patterns.len())
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Formula<T> {
	ForAll(ForAll<T>),
	Exists(Exists<T>),
	Conclusion(Conclusion<T>),
}

impl<T> Formula<T> {
	pub fn is_fully_existential(&self) -> bool {
		match self {
			Self::ForAll(_) => false,
			Self::Exists(e) => e.inner.is_fully_existential(),
			Self::Conclusion(_) => true,
		}
	}

	pub fn is_universal(&self) -> bool {
		matches!(self, Self::ForAll(_))
	}

	pub fn is_existential(&self) -> bool {
		matches!(self, Self::Exists(_))
	}

	pub fn as_existential_mut(&mut self) -> Option<&mut Exists<T>> {
		match self {
			Self::Exists(e) => Some(e),
			_ => None,
		}
	}

	pub fn is_conclusion(&self) -> bool {
		matches!(self, Self::Conclusion(_))
	}

	pub fn conclusion_mut(&mut self) -> &mut Conclusion<T> {
		match self {
			Self::ForAll(a) => a.inner.conclusion_mut(),
			Self::Exists(e) => e.inner.conclusion_mut(),
			Self::Conclusion(c) => c,
		}
	}

	pub fn visit_variables(&self, mut f: impl FnMut(&Self, usize)) {
		match self {
			Self::ForAll(a) => {
				a.constraints.visit_variables(|x| f(self, x));
				a.inner.visit_variables(f)
			}
			Self::Exists(e) => {
				e.hypothesis.visit_variables(|x| f(self, x));
				e.inner.visit_variables(f)
			}
			Self::Conclusion(c) => c.visit_variables(|x| f(self, x)),
		}
	}

	pub fn visit_declared_variables(&self, mut f: impl FnMut(usize)) {
		match self {
			Self::ForAll(a) => {
				for x in &a.variables {
					f(x.index)
				}

				a.inner.visit_declared_variables(f)
			}
			Self::Exists(e) => {
				for x in &e.variables {
					f(x.index)
				}

				e.inner.visit_declared_variables(f)
			}
			Self::Conclusion(_) => (),
		}
	}

	pub fn normalize(&mut self) {
		self.normalize_with(Some)
	}

	pub fn normalize_with(
		&mut self,
		mut f: impl FnMut(Signed<Pattern<T>>) -> Option<Signed<Pattern<T>>>,
	) {
		match self {
			Self::ForAll(a) => {
				a.inner
					.normalize_with(normalize_to_hypothesis(&mut a.constraints));
				a.constraints.patterns =
					std::mem::take(&mut a.constraints.patterns)
						.into_iter()
						.filter_map(|p| {
							if p.1 .0.is_id_or(|x| {
								a.variables.binary_search_by_key(x, |y| y.index).is_ok()
							}) || p.1 .1.is_id_or(|x| {
								a.variables.binary_search_by_key(x, |y| y.index).is_ok()
							}) || p.1 .2.is_id_or(|x| {
								a.variables.binary_search_by_key(x, |y| y.index).is_ok()
							}) {
								Some(p)
							} else {
								f(p)
							}
						})
						.collect();

				if a.constraints.is_empty() {
					panic!("unconstrained universal quantifier")
				}
			}
			Self::Exists(e) => {
				e.inner
					.normalize_with(normalize_to_hypothesis(&mut e.hypothesis));
				e.hypothesis.patterns =
					std::mem::take(&mut e.hypothesis.patterns)
						.into_iter()
						.filter_map(|p| {
							if p.1 .0.is_id_or(|x| {
								e.variables.binary_search_by_key(x, |y| y.index).is_ok()
							}) || p.1 .1.is_id_or(|x| {
								e.variables.binary_search_by_key(x, |y| y.index).is_ok()
							}) || p.1 .2.is_id_or(|x| {
								e.variables.binary_search_by_key(x, |y| y.index).is_ok()
							}) {
								Some(p)
							} else {
								f(p)
							}
						})
						.collect();
			}
			Self::Conclusion(_) => (),
		}
	}

	fn existential_implication_conclusion(&self) -> Option<&Conclusion<T>> {
		match self {
			Self::ForAll(_) => None,
			Self::Exists(e) => e.inner.existential_implication_conclusion(),
			Self::Conclusion(c) => Some(c),
		}
	}

	fn hypothesis_pattern_from(&self, i: usize) -> Option<&Signed<Pattern<T>>> {
		match self {
			Self::ForAll(_) => None,
			Self::Exists(e) => e.hypothesis_pattern_from(i),
			Self::Conclusion(_) => None,
		}
	}
}

fn normalize_to_hypothesis<T>(
	h: &mut Hypothesis<T>,
) -> impl '_ + FnMut(Signed<Pattern<T>>) -> Option<Signed<Pattern<T>>> {
	|p| {
		h.patterns.push(p);
		None
	}
}

pub struct ExistentialImplication<'a, T> {
	formula: &'a Formula<T>,
	conclusion: &'a Conclusion<T>,
}

impl<'a, T> ExistentialImplication<'a, T> {
	pub fn hypothesis_patterns(&self) -> ExistentialImplicationHypothesisPatterns<'a, T> {
		ExistentialImplicationHypothesisPatterns {
			formula: self.formula,
			current: None,
		}
	}

	pub fn hypothesis_pattern(&self, i: usize) -> Option<&'a Signed<Pattern<T>>> {
		self.formula.hypothesis_pattern_from(i)
	}

	pub fn conclusion(&self) -> &'a Conclusion<T> {
		self.conclusion
	}
}

pub struct ExistentialImplicationHypothesisPatterns<'a, T> {
	formula: &'a Formula<T>,
	current: Option<std::slice::Iter<'a, Signed<Pattern<T>>>>,
}

impl<'a, T> Iterator for ExistentialImplicationHypothesisPatterns<'a, T> {
	type Item = &'a Signed<Pattern<T>>;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(current) => match current.next() {
					Some(p) => break Some(p),
					None => self.current = None,
				},
				None => match self.formula {
					Formula::Exists(e) => {
						self.formula = &*e.inner;
						self.current = Some(e.hypothesis.patterns.iter())
					}
					_ => break None,
				},
			}
		}
	}
}

impl<T> Rule<T> {
	pub fn new(id: T, formula: Formula<T>) -> Self {
		Self { id, formula }
	}
}

// impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for Rule<T> {
// 	type Inserted = Rule<T::Inserted>;

// 	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
// 		Rule {
// 			id: self.id.insert_into_vocabulary(vocabulary),
// 			hypothesis: self.hypothesis.insert_into_vocabulary(vocabulary),
// 			conclusion: self.conclusion.insert_into_vocabulary(vocabulary),
// 		}
// 	}
// }

// impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Rule<T> {
// 	type Output = Rule<T::Output>;

// 	fn map_literal(self, mut f: impl FnMut(L) -> M) -> Self::Output {
// 		Rule {
// 			id: self.id.map_literal(&mut f),
// 			hypothesis: self.hypothesis.map_literal(&mut f),
// 			conclusion: self.conclusion.map_literal(f),
// 		}
// 	}
// }

// impl<V: Vocabulary> Interpret<V> for Rule<uninterpreted::Term<V>> {
// 	type Interpreted = Rule;

// 	fn interpret<'a, I: InterpretationMut<'a, V>>(
// 		self,
// 		vocabulary: &mut V,
// 		interpretation: &mut I,
// 	) -> Result<Self::Interpreted, I::Error> {
// 		Ok(Rule {
// 			id: self.id.interpret(vocabulary, interpretation)?,
// 			hypothesis: self.hypothesis.interpret(vocabulary, interpretation)?,
// 			conclusion: self.conclusion.interpret(vocabulary, interpretation)?,
// 		})
// 	}
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Path {
	pub rule: usize,
	pub pattern: usize,
}

impl Path {
	pub fn new(rule: usize, pattern: usize) -> Self {
		Self { rule, pattern }
	}
}
