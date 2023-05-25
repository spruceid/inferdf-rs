use std::{
	collections::BTreeSet,
	hash::Hash,
	iter::{Copied, Peekable},
};

use derivative::Derivative;
use hashbrown::{Equivalent, HashMap};
use locspan::Meta;
use slab::Slab;

use crate::{dataset::Contradiction, pattern, Id, ReplaceId, Sign, Signed, Triple, TripleExt};

pub type Fact<M> = Meta<Signed<Triple>, M>;

pub trait FactWithGraph<M> {
	fn with_graph(self, g: Option<Id>) -> super::Fact<M>;
}

impl<M> FactWithGraph<M> for Fact<M> {
	fn with_graph(self, g: Option<Id>) -> super::Fact<M> {
		let Meta(Signed(sign, triple), meta) = self;
		Meta(Signed(sign, triple.into_quad(g)), meta)
	}
}

#[derive(Derivative, Debug, Clone)]
#[derivative(Default(bound = ""))]
pub struct Graph<M> {
	/// All the facts in the graph.
	facts: Slab<Fact<M>>,
	resources: HashMap<Id, Resource>,
}

fn get_opt<'a, K, V, Q>(map: &'a HashMap<K, V>, key: Option<&Q>) -> Option<Option<&'a V>>
where
	K: Eq + Hash,
	Q: Hash + Equivalent<K>,
{
	match key {
		Some(key) => map.get(key).map(Some),
		None => Some(None),
	}
}

#[derive(Debug, Default, Clone)]
pub struct Resource {
	as_subject: BTreeSet<usize>,
	as_predicate: BTreeSet<usize>,
	as_object: BTreeSet<usize>,
}

impl<M> Graph<M> {
	pub fn contains(&self, triple: Triple, sign: Sign) -> bool {
		self.find_triple(triple)
			.map(|(_, f)| f.sign() == sign)
			.unwrap_or(false)
	}

	pub fn get(&self, i: usize) -> Option<&Fact<M>> {
		self.facts.get(i)
	}

	pub fn iter(&self) -> slab::Iter<Fact<M>> {
		self.facts.iter()
	}

	pub fn insert(&mut self, Meta(fact, meta): Fact<M>) -> Result<(usize, bool), Contradiction> {
		match self.find_triple(*fact.value()) {
			Some((i, current_fact)) => {
				if current_fact.sign() == fact.sign() {
					Ok((i, false))
				} else {
					Err(Contradiction(*fact.value()))
				}
			}
			None => {
				let triple = *fact.value();
				let i = self.facts.insert(Meta(fact, meta));
				self.resources
					.entry(*triple.subject())
					.or_default()
					.as_subject
					.insert(i);
				self.resources
					.entry(*triple.predicate())
					.or_default()
					.as_predicate
					.insert(i);
				self.resources
					.entry(*triple.object())
					.or_default()
					.as_object
					.insert(i);
				Ok((i, true))
			}
		}
	}

	pub fn try_extend(
		&mut self,
		facts: impl IntoIterator<Item = Fact<M>>,
	) -> Result<Vec<(usize, bool)>, Contradiction> {
		let mut indexes = Vec::new();

		for s in facts {
			indexes.push(self.insert(s)?);
		}

		Ok(indexes)
	}

	pub fn remove_triple_with(
		&mut self,
		triple: Triple,
		f: impl FnOnce(&Fact<M>) -> bool,
	) -> Option<Fact<M>> {
		let i = self
			.find_triple(triple)
			.filter(|(_, s)| f(s))
			.map(|(i, _)| i);
		i.map(|i| {
			self.resources
				.get_mut(triple.subject())
				.unwrap()
				.as_subject
				.remove(&i);
			self.resources
				.get_mut(triple.predicate())
				.unwrap()
				.as_predicate
				.remove(&i);
			self.resources
				.get_mut(triple.object())
				.unwrap()
				.as_object
				.remove(&i);
			self.facts.remove(i)
		})
	}

	pub fn remove_triple(&mut self, triple: Triple) -> Option<Fact<M>> {
		self.remove_triple_with(triple, |_| true)
	}

	pub fn remove_positive_triple(&mut self, triple: Triple) -> Option<Fact<M>> {
		self.remove_triple_with(triple, |s| s.is_positive())
	}

	pub fn remove_negative_triple(&mut self, triple: Triple) -> Option<Fact<M>> {
		self.remove_triple_with(triple, |s| s.is_negative())
	}

	pub fn remove_resource(&mut self, id: Id) -> Vec<Fact<M>> {
		let mut facts = Vec::new();

		if let Some(r) = self.resources.remove(&id) {
			for i in r.as_subject {
				if let Some(s) = self.facts.try_remove(i) {
					facts.push(s)
				}
			}

			for i in r.as_predicate {
				if let Some(s) = self.facts.try_remove(i) {
					facts.push(s)
				}
			}

			for i in r.as_object {
				if let Some(s) = self.facts.try_remove(i) {
					facts.push(s)
				}
			}
		}

		facts
	}

	pub fn find_triple(&self, triple: Triple) -> Option<(usize, &Fact<M>)> {
		self.matching(triple.into_pattern().into()).next()
	}

	pub fn find_triple_mut(
		&mut self,
		triple: Triple,
	) -> Option<(usize, Meta<&Signed<Triple>, &mut M>)> {
		self.matching_mut(triple.into_pattern().into()).next()
	}

	pub fn resource_facts(&self, id: Id) -> ResourceFacts<M> {
		match self.resources.get(&id) {
			Some(r) => ResourceFacts::Some {
				facts: &self.facts,
				subject: r.as_subject.iter().copied().peekable(),
				predicate: r.as_predicate.iter().copied().peekable(),
				object: r.as_object.iter().copied().peekable(),
			},
			None => ResourceFacts::None,
		}
	}

	pub fn full_matching(&self, pattern: pattern::Canonical, sign: Option<Sign>) -> Matching<M> {
		let s = pattern.subject().id();
		let p = pattern.predicate().id();
		let o = pattern.object().id();

		let inner = if s.is_none() && p.is_none() && o.is_none() {
			InnerMatching::All(self.facts.iter())
		} else {
			get_opt(&self.resources, s.as_ref())
				.and_then(|s| {
					get_opt(&self.resources, p.as_ref()).and_then(|p| {
						get_opt(&self.resources, o.as_ref()).map(|o| InnerMatching::Constrained {
							facts: &self.facts,
							subject: s.map(|r| r.as_subject.iter()),
							predicate: p.map(|r| r.as_predicate.iter()),
							object: o.map(|r| r.as_object.iter()),
						})
					})
				})
				.unwrap_or(InnerMatching::None)
		};

		Matching {
			inner,
			constraints: MatchingConstraints {
				predicate: pattern.predicate(),
				object: pattern.object(),
				sign,
			},
		}
	}

	pub fn matching(&self, pattern: pattern::Canonical) -> Matching<M> {
		self.full_matching(pattern, None)
	}

	pub fn signed_matching(
		&self,
		Signed(sign, pattern): Signed<pattern::Canonical>,
	) -> Matching<M> {
		self.full_matching(pattern, Some(sign))
	}

	pub fn matching_mut(&mut self, pattern: pattern::Canonical) -> MatchingMut<M> {
		let s = pattern.subject().id();
		let p = pattern.predicate().id();
		let o = pattern.object().id();

		if s.is_none() && p.is_none() && o.is_none() {
			MatchingMut::All(self.facts.iter_mut())
		} else {
			get_opt(&self.resources, s.as_ref())
				.and_then(|s| {
					get_opt(&self.resources, p.as_ref()).and_then(|p| {
						get_opt(&self.resources, o.as_ref()).map(|o| MatchingMut::Constrained {
							facts: &mut self.facts,
							subject: s.map(|r| r.as_subject.iter()),
							predicate: p.map(|r| r.as_predicate.iter()),
							object: o.map(|r| r.as_object.iter()),
						})
					})
				})
				.unwrap_or(MatchingMut::None)
		}
	}

	pub fn replace_id(
		&mut self,
		a: Id,
		b: Id,
		filter: impl Fn(&Fact<M>) -> Result<bool, Contradiction>,
	) -> Result<(), Contradiction> {
		for mut fact in self.remove_resource(b) {
			fact.replace_id(a, b);
			if filter(&fact)? {
				self.insert(fact)?;
			}
		}

		Ok(())
	}
}

impl<M> IntoIterator for Graph<M> {
	type IntoIter = slab::IntoIter<Fact<M>>;
	type Item = (usize, Fact<M>);

	fn into_iter(self) -> Self::IntoIter {
		self.facts.into_iter()
	}
}

pub type IntoIter<'a, M> = slab::IntoIter<Fact<M>>;

impl<'a, M> IntoIterator for &'a Graph<M> {
	type IntoIter = slab::Iter<'a, Fact<M>>;
	type Item = (usize, &'a Fact<M>);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub type Iter<'a, M> = slab::Iter<'a, Fact<M>>;

pub enum ResourceFacts<'a, M> {
	None,
	Some {
		facts: &'a Slab<Fact<M>>,
		subject: Peekable<Copied<std::collections::btree_set::Iter<'a, usize>>>,
		predicate: Peekable<Copied<std::collections::btree_set::Iter<'a, usize>>>,
		object: Peekable<Copied<std::collections::btree_set::Iter<'a, usize>>>,
	},
}

impl<'a, M> ResourceFacts<'a, M> {
	pub fn is_empty(&self) -> bool {
		match self {
			Self::None => true,
			Self::Some {
				subject,
				predicate,
				object,
				..
			} => subject.len() == 0 && predicate.len() == 0 && object.len() == 0,
		}
	}
}

impl<'a, M> Iterator for ResourceFacts<'a, M> {
	type Item = (usize, &'a Fact<M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::Some {
				facts,
				subject,
				predicate,
				object,
			} => {
				let mut min = None;

				if let Some(&s) = subject.peek() {
					min = Some(min.map_or(s, |m| std::cmp::min(s, m)))
				}

				if let Some(&p) = predicate.peek() {
					min = Some(min.map_or(p, |m| std::cmp::min(p, m)))
				}

				if let Some(&o) = object.peek() {
					min = Some(min.map_or(o, |m| std::cmp::min(o, m)))
				}

				min.map(|m| {
					if subject.peek().copied() == Some(m) {
						subject.next();
					}

					if predicate.peek().copied() == Some(m) {
						predicate.next();
					}

					if object.peek().copied() == Some(m) {
						object.next();
					}

					(m, &facts[m])
				})
			}
		}
	}
}

struct MatchingConstraints {
	predicate: pattern::PatternPredicate,
	object: pattern::PatternObject,
	sign: Option<Sign>,
}

impl MatchingConstraints {
	fn filter(&self, Signed(sign, triple): Signed<Triple>) -> bool {
		self.sign.map(|s| sign == s).unwrap_or(true)
			&& self.predicate.filter_triple(triple)
			&& self.object.filter_triple(triple)
	}
}

pub struct Matching<'a, M> {
	inner: InnerMatching<'a, M>,
	constraints: MatchingConstraints,
}

impl<'a, M> Iterator for Matching<'a, M> {
	type Item = (usize, &'a Fact<M>);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner
			.find(|(_, Meta(t, _))| self.constraints.filter(*t))
	}
}

enum InnerMatching<'a, M> {
	None,
	All(slab::Iter<'a, Fact<M>>),
	Constrained {
		facts: &'a Slab<Fact<M>>,
		subject: Option<std::collections::btree_set::Iter<'a, usize>>,
		predicate: Option<std::collections::btree_set::Iter<'a, usize>>,
		object: Option<std::collections::btree_set::Iter<'a, usize>>,
	},
}

impl<'a, M> Iterator for InnerMatching<'a, M> {
	type Item = (usize, &'a Fact<M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::All(iter) => iter.next(),
			Self::Constrained {
				facts,
				subject,
				predicate,
				object,
			} => {
				enum State {
					Subject,
					Predicate,
					Object,
				}

				impl State {
					fn next(self) -> Self {
						match self {
							Self::Subject => Self::Predicate,
							Self::Predicate => Self::Object,
							Self::Object => Self::Subject,
						}
					}
				}

				let mut state = State::Subject;
				let mut candidate = None;
				let mut count = 0;

				while count < 3 {
					let iter = match state {
						State::Subject => subject.as_mut(),
						State::Predicate => predicate.as_mut(),
						State::Object => object.as_mut(),
					};

					if let Some(iter) = iter {
						loop {
							match iter.next().copied() {
								Some(i) => match candidate {
									Some(j) => {
										if i >= j {
											if i > j {
												candidate = Some(i);
												count = 0
											}
											break;
										}
									}
									None => {
										candidate = Some(i);
										break;
									}
								},
								None => return None,
							}
						}
					}

					count += 1;
					state = state.next();
				}

				candidate.map(|i| (i, &facts[i]))
			}
		}
	}
}

pub enum MatchingMut<'a, M> {
	None,
	All(slab::IterMut<'a, Fact<M>>),
	Constrained {
		facts: &'a mut Slab<Fact<M>>,
		subject: Option<std::collections::btree_set::Iter<'a, usize>>,
		predicate: Option<std::collections::btree_set::Iter<'a, usize>>,
		object: Option<std::collections::btree_set::Iter<'a, usize>>,
	},
}

impl<'a, M> Iterator for MatchingMut<'a, M> {
	type Item = (usize, Meta<&'a Signed<Triple>, &'a mut M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::All(iter) => iter.next().map(|(i, Meta(t, m))| (i, Meta(&*t, m))),
			Self::Constrained {
				facts,
				subject,
				predicate,
				object,
			} => {
				enum State {
					Subject,
					Predicate,
					Object,
				}

				impl State {
					fn next(self) -> Self {
						match self {
							Self::Subject => Self::Predicate,
							Self::Predicate => Self::Object,
							Self::Object => Self::Subject,
						}
					}
				}

				let mut state = State::Subject;
				let mut candidate = None;
				let mut count = 0;

				while count < 3 {
					let iter = match state {
						State::Subject => subject.as_mut(),
						State::Predicate => predicate.as_mut(),
						State::Object => object.as_mut(),
					};

					if let Some(iter) = iter {
						loop {
							match iter.next().copied() {
								Some(i) => match candidate {
									Some(j) => {
										if i >= j {
											if i > j {
												candidate = Some(i);
												count = 0
											}
											break;
										}
									}
									None => {
										candidate = Some(i);
										break;
									}
								},
								None => return None,
							}
						}
					}

					count += 1;
					state = state.next();
				}

				candidate.map(|i| {
					let Meta(t, m): &'a mut Fact<M> = unsafe {
						// This is safe because the iterator does not yield
						// aliased items.
						std::mem::transmute(&mut facts[i])
					};

					(i, Meta(&*t, m))
				})
			}
		}
	}
}
