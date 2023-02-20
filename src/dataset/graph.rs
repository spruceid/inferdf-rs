use std::collections::BTreeSet;

use derivative::Derivative;
use hashbrown::HashMap;
use slab::Slab;

use crate::{Triple, Pattern, Cause, TripleExt, Id};

#[derive(Derivative, Debug, Clone)]
#[derivative(Default(bound=""))]
pub struct Graph<M> {
	/// All the facts in the graph.
	facts: Slab<Fact<M>>,
	resources: HashMap<Id, Resource>
}

#[derive(Debug, Clone)]
pub struct Fact<M> {
	pub triple: Triple,
	pub positive: bool,
	pub cause: Cause<M>
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

pub struct Contradiction(Triple);

impl<M> Graph<M> {
	pub fn insert(&mut self, triple: Triple, positive: bool, cause: Cause<M>) -> Result<(usize, bool), Contradiction> {
		match self.find_triple_mut(triple) {
			Some((i, fact)) => {
				if fact.is_positive() == positive {
					Ok((i, false))
				} else {
					Err(Contradiction(triple))
				}
			}
			None => {
				let i = self.facts.insert(Fact { triple, positive, cause });
				self.resources.get_mut(triple.subject()).unwrap().as_subject.insert(i);
				self.resources.get_mut(triple.predicate()).unwrap().as_predicate.insert(i);
				self.resources.get_mut(triple.object()).unwrap().as_object.insert(i);
				Ok((i, true))
			}
		}
	}

	pub fn try_extend(
		&mut self,
		facts: impl IntoIterator<Item = Fact<M>>
	) -> Result<Vec<(usize, bool)>, Contradiction> {
		let mut indexes = Vec::new();

		for s in facts {
			indexes.push(self.insert(s.triple, s.positive, s.cause)?);
		}

		Ok(indexes)
	}

	pub fn remove_triple_with(&mut self, triple: Triple, f: impl FnOnce(&Fact<M>) -> bool) -> Option<Fact<M>> {
		let i = self.find_triple(triple).filter(|(_, s)| f(s)).map(|(i, _)| i);
		i.map(|i| {
			self.resources.get_mut(triple.subject()).unwrap().as_subject.remove(&i);
			self.resources.get_mut(triple.predicate()).unwrap().as_predicate.remove(&i);
			self.resources.get_mut(triple.object()).unwrap().as_object.remove(&i);
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

	fn matching(&self, rdf_types::Triple(s, p, o): Pattern) -> Matching<M> {
		if s.is_none() && p.is_none() && o.is_none() {
			Matching::All(self.facts.iter())
		} else {
			Matching::Constrained {
				facts: &self.facts,
				subject: s.map(|s| self.resources[&s].as_subject.iter()),
				predicate: p.map(|p| self.resources[&p].as_predicate.iter()),
				object: o.map(|o| self.resources[&o].as_object.iter())
			}
		}
	}

	fn matching_mut(&mut self, rdf_types::Triple(s, p, o): Pattern) -> MatchingMut<M> {
		if s.is_none() && p.is_none() && o.is_none() {
			MatchingMut::All(self.facts.iter_mut())
		} else {
			MatchingMut::Constrained {
				facts: &mut self.facts,
				subject: s.map(|s| self.resources[&s].as_subject.iter()),
				predicate: p.map(|p| self.resources[&p].as_predicate.iter()),
				object: o.map(|o| self.resources[&o].as_object.iter())
			}
		}
	}
}

pub enum Matching<'a, M> {
	All(slab::Iter<'a, Fact<M>>),
	Constrained {
		facts: &'a Slab<Fact<M>>,
		subject: Option<std::collections::btree_set::Iter<'a, usize>>,
		predicate: Option<std::collections::btree_set::Iter<'a, usize>>,
		object: Option<std::collections::btree_set::Iter<'a, usize>>
	}
}

impl<'a, M> Iterator for Matching<'a, M> {
	type Item = (usize, &'a Fact<M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::All(iter) => iter.next(),
			Self::Constrained { facts, subject, predicate, object } => {
				enum State {
					Subject,
					Predicate,
					Object
				}

				impl State {
					fn next(self) -> Self {
						match self {
							Self::Subject => Self::Predicate,
							Self::Predicate => Self::Object,
							Self::Object => Self::Subject
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
						State::Object => object.as_mut()
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
											break
										}
									}
									None => {
										candidate = Some(i);
										break;
									}
								},
								None => return None
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
	All(slab::IterMut<'a, Fact<M>>),
	Constrained {
		facts: &'a mut Slab<Fact<M>>,
		subject: Option<std::collections::btree_set::Iter<'a, usize>>,
		predicate: Option<std::collections::btree_set::Iter<'a, usize>>,
		object: Option<std::collections::btree_set::Iter<'a, usize>>
	}
}

impl<'a, M> Iterator for MatchingMut<'a, M> {
	type Item = (usize, FactMut<'a, M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::All(iter) => iter.next().map(|(i, s)| (i, FactMut(s))),
			Self::Constrained { facts, subject, predicate, object } => {
				enum State {
					Subject,
					Predicate,
					Object
				}

				impl State {
					fn next(self) -> Self {
						match self {
							Self::Subject => Self::Predicate,
							Self::Predicate => Self::Object,
							Self::Object => Self::Subject
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
						State::Object => object.as_mut()
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
											break
										}
									}
									None => {
										candidate = Some(i);
										break;
									}
								},
								None => return None
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