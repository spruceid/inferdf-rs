use std::{
	collections::BTreeSet,
	hash::Hash,
	iter::{Copied, Peekable},
};

use derivative::Derivative;
use hashbrown::{Equivalent, HashMap};
use slab::Slab;

use crate::{Cause, Id, Pattern, Quad, Triple, TripleExt};

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

#[derive(Debug, Clone)]
pub struct Fact<M> {
	pub triple: Triple,
	pub positive: bool,
	pub cause: Cause<M>,
}

impl<M> Fact<M> {
	pub fn new(triple: Triple, positive: bool, cause: Cause<M>) -> Self {
		Self {
			triple,
			positive,
			cause,
		}
	}

	pub fn with_graph(&self, g: Option<Id>) -> super::Fact<&M> {
		super::Fact::new(self.triple.into_quad(g), self.positive, self.cause.as_ref())
	}

	pub fn into_with_graph(self, g: Option<Id>) -> super::Fact<M> {
		super::Fact::new(self.triple.into_quad(g), self.positive, self.cause)
	}
}

pub struct FactMut<'a, M>(&'a mut Fact<M>);

impl<'a, M> FactMut<'a, M> {
	pub fn triple(&self) -> Triple {
		self.0.triple
	}

	pub fn is_positive(&self) -> bool {
		self.0.positive
	}

	pub fn cause(&self) -> &Cause<M> {
		&self.0.cause
	}

	pub fn cause_mut(&mut self) -> &mut Cause<M> {
		&mut self.0.cause
	}
}

pub trait ReplaceId {
	/// Replace id `a` with `b`.
	fn replace_id(&mut self, a: Id, b: Id);
}

impl<T: ReplaceId> ReplaceId for Vec<T> {
	fn replace_id(&mut self, a: Id, b: Id) {
		for t in self {
			t.replace_id(a, b)
		}
	}
}

impl<M> ReplaceId for Fact<M> {
	fn replace_id(&mut self, a: Id, b: Id) {
		self.triple.replace_id(a, b);
	}
}

impl ReplaceId for Triple {
	fn replace_id(&mut self, a: Id, b: Id) {
		self.subject_mut().replace_id(a, b);
		self.predicate_mut().replace_id(a, b);
		self.object_mut().replace_id(a, b);
	}
}

impl ReplaceId for Quad {
	fn replace_id(&mut self, a: Id, b: Id) {
		self.subject_mut().replace_id(a, b);
		self.predicate_mut().replace_id(a, b);
		self.object_mut().replace_id(a, b);
		if let Some(g) = self.graph_mut() {
			g.replace_id(a, b);
		}
	}
}

impl ReplaceId for Option<Id> {
	fn replace_id(&mut self, a: Id, b: Id) {
		if let Some(id) = self {
			id.replace_id(a, b)
		}
	}
}

impl ReplaceId for Id {
	fn replace_id(&mut self, a: Id, b: Id) {
		if *self == a {
			*self = b
		}
	}
}

#[derive(Debug, Clone)]
pub struct Resource {
	as_subject: BTreeSet<usize>,
	as_predicate: BTreeSet<usize>,
	as_object: BTreeSet<usize>,
}

pub struct Contradiction(pub Triple);

impl<M> Graph<M> {
	pub fn contains(&self, triple: Triple, positive: bool) -> bool {
		self.find_triple(triple)
			.map(|(_, f)| f.positive == positive)
			.unwrap_or(false)
	}

	pub fn get(&self, i: usize) -> Option<&Fact<M>> {
		self.facts.get(i)
	}

	pub fn insert(&mut self, fact: Fact<M>) -> Result<(usize, bool), Contradiction> {
		match self.find_triple_mut(fact.triple) {
			Some((i, current_fact)) => {
				if current_fact.is_positive() == fact.positive {
					Ok((i, false))
				} else {
					Err(Contradiction(fact.triple))
				}
			}
			None => {
				let triple = fact.triple;
				let i = self.facts.insert(fact);
				self.resources
					.get_mut(triple.subject())
					.unwrap()
					.as_subject
					.insert(i);
				self.resources
					.get_mut(triple.predicate())
					.unwrap()
					.as_predicate
					.insert(i);
				self.resources
					.get_mut(triple.object())
					.unwrap()
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
		self.remove_triple_with(triple, |s| s.positive)
	}

	pub fn remove_negative_triple(&mut self, triple: Triple) -> Option<Fact<M>> {
		self.remove_triple_with(triple, |s| !s.positive)
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
		self.matching(triple.into_pattern()).next()
	}

	pub fn find_triple_mut(&mut self, triple: Triple) -> Option<(usize, FactMut<M>)> {
		self.matching_mut(triple.into_pattern()).next()
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

	pub fn matching(&self, rdf_types::Triple(s, p, o): Pattern) -> Matching<M> {
		if s.is_none() && p.is_none() && o.is_none() {
			Matching::All(self.facts.iter())
		} else {
			get_opt(&self.resources, s.as_ref())
				.and_then(|s| {
					get_opt(&self.resources, p.as_ref()).and_then(|p| {
						get_opt(&self.resources, o.as_ref()).map(|o| Matching::Constrained {
							facts: &self.facts,
							subject: s.map(|r| r.as_subject.iter()),
							predicate: p.map(|r| r.as_predicate.iter()),
							object: o.map(|r| r.as_object.iter()),
						})
					})
				})
				.unwrap_or(Matching::None)
		}
	}

	pub fn matching_mut(&mut self, rdf_types::Triple(s, p, o): Pattern) -> MatchingMut<M> {
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

pub enum Matching<'a, M> {
	None,
	All(slab::Iter<'a, Fact<M>>),
	Constrained {
		facts: &'a Slab<Fact<M>>,
		subject: Option<std::collections::btree_set::Iter<'a, usize>>,
		predicate: Option<std::collections::btree_set::Iter<'a, usize>>,
		object: Option<std::collections::btree_set::Iter<'a, usize>>,
	},
}

impl<'a, M> Iterator for Matching<'a, M> {
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
	type Item = (usize, FactMut<'a, M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::All(iter) => iter.next().map(|(i, s)| (i, FactMut(s))),
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
					let fact: &'a mut Fact<M> = unsafe {
						// This is safe because the iterator does not yield
						// aliased items.
						std::mem::transmute(&mut facts[i])
					};

					(i, FactMut(fact))
				})
			}
		}
	}
}
