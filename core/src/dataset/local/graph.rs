use std::{
	collections::BTreeSet,
	hash::Hash,
	iter::{Copied, Peekable},
};

use derivative::Derivative;
use hashbrown::{Equivalent, HashMap};
use locspan::Meta;
use slab::Slab;

use crate::{
	dataset::Contradiction, pattern, Cause, Id, IteratorWith, ReplaceId, Sign, Signed, Triple,
	TripleExt,
};

pub type Fact = Meta<Signed<Triple>, Cause>;

pub trait FactWithGraph {
	fn with_graph(self, g: Option<Id>) -> super::Fact;
}

impl FactWithGraph for Fact {
	fn with_graph(self, g: Option<Id>) -> super::Fact {
		let Meta(Signed(sign, triple), meta) = self;
		Meta(Signed(sign, triple.into_quad(g)), meta)
	}
}

#[derive(Derivative, Debug, Clone)]
#[derivative(Default(bound = ""))]
pub struct Graph {
	/// All the facts in the graph.
	facts: Slab<Fact>,
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
	as_subject: BTreeSet<u32>,
	as_predicate: BTreeSet<u32>,
	as_object: BTreeSet<u32>,
}

impl Resource {
	pub fn iter_as_subject(&self) -> impl '_ + Iterator<Item = u32> {
		self.as_subject.iter().copied()
	}

	pub fn iter_as_predicate(&self) -> impl '_ + Iterator<Item = u32> {
		self.as_predicate.iter().copied()
	}

	pub fn iter_as_object(&self) -> impl '_ + Iterator<Item = u32> {
		self.as_object.iter().copied()
	}
}

impl Graph {
	pub fn contains(&self, triple: Triple, sign: Sign) -> bool {
		self.find_triple(triple)
			.map(|(_, f)| f.sign() == sign)
			.unwrap_or(false)
	}

	pub fn len(&self) -> usize {
		self.facts.len()
	}

	pub fn is_empty(&self) -> bool {
		self.facts.is_empty()
	}

	pub fn get(&self, i: usize) -> Option<&Fact> {
		self.facts.get(i)
	}

	pub fn get_resource(&self, id: Id) -> Option<&Resource> {
		self.resources.get(&id)
	}

	pub fn iter(&self) -> slab::Iter<Fact> {
		self.facts.iter()
	}

	pub fn resource_count(&self) -> usize {
		self.resources.len()
	}

	pub fn iter_resources(&self) -> impl Iterator<Item = (Id, &Resource)> {
		self.resources.iter().map(|(id, r)| (*id, r))
	}

	pub fn insert(&mut self, Meta(fact, meta): Fact) -> Result<(u32, bool), Contradiction> {
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
				let i = self.facts.insert(Meta(fact, meta)) as u32;
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
		facts: impl IntoIterator<Item = Fact>,
	) -> Result<Vec<(u32, bool)>, Contradiction> {
		let mut indexes = Vec::new();

		for s in facts {
			indexes.push(self.insert(s)?);
		}

		Ok(indexes)
	}

	pub fn remove_triple_with(
		&mut self,
		triple: Triple,
		f: impl FnOnce(&Fact) -> bool,
	) -> Option<Fact> {
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
			self.facts.remove(i as usize)
		})
	}

	pub fn remove_triple(&mut self, triple: Triple) -> Option<Fact> {
		self.remove_triple_with(triple, |_| true)
	}

	pub fn remove_positive_triple(&mut self, triple: Triple) -> Option<Fact> {
		self.remove_triple_with(triple, |s| s.is_positive())
	}

	pub fn remove_negative_triple(&mut self, triple: Triple) -> Option<Fact> {
		self.remove_triple_with(triple, |s| s.is_negative())
	}

	pub fn remove_resource(&mut self, id: Id) -> Vec<Fact> {
		let mut facts = Vec::new();

		if let Some(r) = self.resources.remove(&id) {
			for i in r.as_subject {
				if let Some(s) = self.facts.try_remove(i as usize) {
					facts.push(s)
				}
			}

			for i in r.as_predicate {
				if let Some(s) = self.facts.try_remove(i as usize) {
					facts.push(s)
				}
			}

			for i in r.as_object {
				if let Some(s) = self.facts.try_remove(i as usize) {
					facts.push(s)
				}
			}
		}

		facts
	}

	pub fn find_triple(&self, triple: Triple) -> Option<(u32, &Fact)> {
		self.matching(triple.into_pattern().into()).next()
	}

	// pub fn find_triple_mut(
	// 	&mut self,
	// 	triple: Triple,
	// ) -> Option<(usize, Meta<&Signed<Triple>, &mut Cause>)> {
	// 	self.matching_mut(triple.into_pattern().into()).next()
	// }

	pub fn resource_facts(&self, id: Id) -> ResourceFacts {
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

	pub fn full_matching(&self, pattern: pattern::Canonical, sign: Option<Sign>) -> Matching {
		let s = pattern.subject().into_id();
		let p = pattern.predicate().into_id();
		let o = pattern.object().into_id();

		let inner = if s.is_none() && p.is_none() && o.is_none() {
			InnerMatching::All(self.facts.iter())
		} else {
			get_opt(&self.resources, s)
				.and_then(|s| {
					get_opt(&self.resources, p).and_then(|p| {
						get_opt(&self.resources, o).map(|o| InnerMatching::Constrained {
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
				predicate: pattern.predicate().cloned(),
				object: pattern.object().cloned(),
				sign,
			},
		}
	}

	pub fn matching(&self, pattern: pattern::Canonical) -> Matching {
		self.full_matching(pattern, None)
	}

	pub fn signed_matching(&self, Signed(sign, pattern): Signed<pattern::Canonical>) -> Matching {
		self.full_matching(pattern, Some(sign))
	}

	// pub fn matching_mut(&mut self, pattern: pattern::Canonical) -> MatchingMut {
	// 	let s = pattern.subject().id();
	// 	let p = pattern.predicate().id();
	// 	let o = pattern.object().id();

	// 	if s.is_none() && p.is_none() && o.is_none() {
	// 		MatchingMut::All(self.facts.iter_mut())
	// 	} else {
	// 		get_opt(&self.resources, s.as_ref())
	// 			.and_then(|s| {
	// 				get_opt(&self.resources, p.as_ref()).and_then(|p| {
	// 					get_opt(&self.resources, o.as_ref()).map(|o| MatchingMut::Constrained {
	// 						facts: &mut self.facts,
	// 						subject: s.map(|r| r.as_subject.iter()),
	// 						predicate: p.map(|r| r.as_predicate.iter()),
	// 						object: o.map(|r| r.as_object.iter()),
	// 					})
	// 				})
	// 			})
	// 			.unwrap_or(MatchingMut::None)
	// 	}
	// }

	pub fn replace_id<E: From<Contradiction>>(
		&mut self,
		a: Id,
		b: Id,
		mut filter: impl FnMut(&Fact) -> Result<bool, E>,
	) -> Result<(), E> {
		for mut fact in self.remove_resource(b) {
			fact.replace_id(a, b);
			if filter(&fact)? {
				self.insert(fact)?;
			}
		}

		Ok(())
	}
}

impl IntoIterator for Graph {
	type IntoIter = slab::IntoIter<Fact>;
	type Item = (usize, Fact);

	fn into_iter(self) -> Self::IntoIter {
		self.facts.into_iter()
	}
}

pub type IntoIter<'a> = slab::IntoIter<Fact>;

impl<'a> IntoIterator for &'a Graph {
	type IntoIter = slab::Iter<'a, Fact>;
	type Item = (usize, &'a Fact);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub type Iter<'a> = slab::Iter<'a, Fact>;

pub enum ResourceFacts<'a> {
	None,
	Some {
		facts: &'a Slab<Fact>,
		subject: Peekable<Copied<std::collections::btree_set::Iter<'a, u32>>>,
		predicate: Peekable<Copied<std::collections::btree_set::Iter<'a, u32>>>,
		object: Peekable<Copied<std::collections::btree_set::Iter<'a, u32>>>,
	},
}

impl<'a> ResourceFacts<'a> {
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

impl<'a> Iterator for ResourceFacts<'a> {
	type Item = (u32, &'a Fact);

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

					(m, &facts[m as usize])
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

pub struct Matching<'a> {
	inner: InnerMatching<'a>,
	constraints: MatchingConstraints,
}

impl<'a> Iterator for Matching<'a> {
	type Item = (u32, &'a Fact);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner
			.find(|(_, Meta(t, _))| self.constraints.filter(*t))
	}
}

enum InnerMatching<'a> {
	None,
	All(slab::Iter<'a, Fact>),
	Constrained {
		facts: &'a Slab<Fact>,
		subject: Option<std::collections::btree_set::Iter<'a, u32>>,
		predicate: Option<std::collections::btree_set::Iter<'a, u32>>,
		object: Option<std::collections::btree_set::Iter<'a, u32>>,
	},
}

impl<'a> Iterator for InnerMatching<'a> {
	type Item = (u32, &'a Fact);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::All(iter) => iter.next().map(|(i, f)| (i as u32, f)),
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

				candidate.map(|i| (i, &facts[i as usize]))
			}
		}
	}
}

impl<'a> crate::dataset::graph::Resource<'a> for &'a Resource {
	type AsSubject = std::iter::Copied<std::collections::btree_set::Iter<'a, u32>>;
	type AsPredicate = std::iter::Copied<std::collections::btree_set::Iter<'a, u32>>;
	type AsObject = std::iter::Copied<std::collections::btree_set::Iter<'a, u32>>;

	fn as_subject(&self) -> Self::AsSubject {
		self.as_subject.iter().copied()
	}

	fn as_predicate(&self) -> Self::AsPredicate {
		self.as_predicate.iter().copied()
	}

	fn as_object(&self) -> Self::AsObject {
		self.as_object.iter().copied()
	}
}

impl<'a, V> crate::dataset::Graph<'a, V> for &'a Graph {
	type Error = std::convert::Infallible;

	type Resource = &'a Resource;

	type Resources = Resources<'a>;

	type Triples = Facts<'a>;

	fn get_resource(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		Ok(self.resources.get(&id))
	}

	fn resources(&self) -> Self::Resources {
		Resources(self.resources.iter())
	}

	fn get_triple(&self, _vocabulary: &mut V, index: u32) -> Result<Option<Fact>, Self::Error> {
		Ok(self.facts.get(index as usize).copied())
	}

	fn len(&self) -> u32 {
		self.facts.len() as u32
	}

	fn triples(&self) -> Self::Triples {
		Facts(self.facts.iter())
	}
}

pub struct Resources<'a>(hashbrown::hash_map::Iter<'a, Id, Resource>);

impl<'a> Iterator for Resources<'a> {
	type Item = Result<(Id, &'a Resource), std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(id, r)| Ok((*id, r)))
	}
}

impl<'a, V> IteratorWith<V> for Resources<'a> {
	type Item = Result<(Id, &'a Resource), std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct Facts<'a>(slab::Iter<'a, Fact>);

impl<'a> Iterator for Facts<'a> {
	type Item = Result<(u32, Fact), std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(i, r)| Ok((i as u32, *r)))
	}
}

impl<'a, V> IteratorWith<V> for Facts<'a> {
	type Item = Result<(u32, Fact), std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}
