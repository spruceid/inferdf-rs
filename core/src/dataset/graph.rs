use std::iter::Peekable;

use locspan::Meta;

use crate::{pattern, GraphFact, Id, Sign, Signed, Triple};

pub trait Resource<'a> {
	type TripleIndexes: 'a + Iterator<Item = u32>;

	fn as_subject(&self) -> Self::TripleIndexes;

	fn as_predicate(&self) -> Self::TripleIndexes;

	fn as_object(&self) -> Self::TripleIndexes;
}

pub trait Graph<'a>: Clone {
	type Error;
	type Resource: Resource<'a>;

	type Triples: Iterator<Item = Result<(u32, GraphFact), Self::Error>>;

	fn get_resource(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error>;

	fn get_triple(&self, index: u32) -> Result<Option<GraphFact>, Self::Error>;

	fn triples(&self) -> Self::Triples;

	fn find_triple(&self, triple: Triple) -> Result<Option<(u32, GraphFact)>, Self::Error> {
		self.unsigned_pattern_matching(triple.into())?
			.next()
			.transpose()
	}

	fn resource_facts(&self, id: Id) -> Result<ResourceFacts<'a, Self>, Self::Error> {
		match self.get_resource(id)? {
			Some(r) => Ok(ResourceFacts::Some {
				graph: (*self).clone(),
				subject: r.as_subject().peekable(),
				predicate: r.as_predicate().peekable(),
				object: r.as_object().peekable(),
			}),
			None => Ok(ResourceFacts::None),
		}
	}

	fn full_pattern_matching(
		&self,
		pattern: pattern::Canonical,
		sign: Option<Sign>,
	) -> Result<Matching<'a, Self>, Self::Error> {
		Ok(Matching {
			inner: RawMatching::new((*self).clone(), pattern)?,
			constraints: MatchingConstraints {
				predicate: pattern.predicate(),
				object: pattern.object(),
				sign,
			},
		})
	}

	fn pattern_matching(
		&self,
		Signed(sign, pattern): Signed<pattern::Canonical>,
	) -> Result<Matching<'a, Self>, Self::Error> {
		self.full_pattern_matching(pattern, Some(sign))
	}

	fn unsigned_pattern_matching(
		&self,
		pattern: pattern::Canonical,
	) -> Result<Matching<'a, Self>, Self::Error> {
		self.full_pattern_matching(pattern, None)
	}
}

pub enum ResourceFacts<'a, G: Graph<'a>> {
	None,
	Some {
		graph: G,
		subject: Peekable<<G::Resource as Resource<'a>>::TripleIndexes>,
		predicate: Peekable<<G::Resource as Resource<'a>>::TripleIndexes>,
		object: Peekable<<G::Resource as Resource<'a>>::TripleIndexes>,
	},
}

impl<'a, G: Graph<'a>> ResourceFacts<'a, G> {
	pub fn is_empty(&mut self) -> bool {
		match self {
			Self::None => true,
			Self::Some {
				subject,
				predicate,
				object,
				..
			} => subject.peek().is_none() && predicate.peek().is_none() && object.peek().is_none(),
		}
	}
}

impl<'a, G: Graph<'a>> Iterator for ResourceFacts<'a, G> {
	type Item = Result<(u32, GraphFact), G::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::Some {
				graph,
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

					Ok((m, graph.get_triple(m)?.unwrap()))
				})
			}
		}
	}
}

pub struct MatchingQuads<'a, G: Graph<'a>>(Matching<'a, G>);

impl<'a, G: Graph<'a>> Iterator for MatchingQuads<'a, G> {
	type Item = Result<GraphFact, G::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|r| r.map(|(_, q)| q))
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

pub struct Matching<'a, G: Graph<'a>> {
	inner: RawMatching<'a, G>,
	constraints: MatchingConstraints,
}

impl<'a, G: Graph<'a>> Iterator for Matching<'a, G> {
	type Item = Result<(u32, GraphFact), G::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.find(|r| match r {
			Ok((_, Meta(t, _))) => self.constraints.filter(*t),
			Err(_) => true,
		})
	}
}

enum RawMatching<'a, G: Graph<'a>> {
	None,
	All(G::Triples),
	Constrained {
		graph: G,
		subject: Option<<G::Resource as Resource<'a>>::TripleIndexes>,
		predicate: Option<<G::Resource as Resource<'a>>::TripleIndexes>,
		object: Option<<G::Resource as Resource<'a>>::TripleIndexes>,
	},
}

fn get_resource_opt<'a, G: Graph<'a>>(
	graph: &G,
	id: Option<Id>,
) -> Result<Option<Option<G::Resource>>, G::Error> {
	match id {
		Some(id) => Ok(graph.get_resource(id)?.map(Some)),
		None => Ok(Some(None)),
	}
}

impl<'a, G: Graph<'a>> RawMatching<'a, G> {
	fn new(graph: G, pattern: pattern::Canonical) -> Result<Self, G::Error> {
		let s = pattern.subject().id();
		let p = pattern.predicate().id();
		let o = pattern.object().id();

		if s.is_none() && p.is_none() && o.is_none() {
			Ok(Self::All(graph.triples()))
		} else {
			match get_resource_opt(&graph, s)? {
				Some(s) => match get_resource_opt(&graph, p)? {
					Some(p) => match get_resource_opt(&graph, o)? {
						Some(o) => Ok(Self::Constrained {
							graph,
							subject: s.map(|r| r.as_subject()),
							predicate: p.map(|r| r.as_predicate()),
							object: o.map(|r| r.as_object()),
						}),
						None => Ok(Self::None),
					},
					None => Ok(Self::None),
				},
				None => Ok(Self::None),
			}
		}
	}
}

impl<'a, G: Graph<'a>> Iterator for RawMatching<'a, G> {
	type Item = Result<(u32, GraphFact), G::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::All(iter) => iter.next(),
			Self::Constrained {
				graph,
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
							match iter.next() {
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

				candidate.map(|i| Ok((i, graph.get_triple(i)?.unwrap())))
			}
		}
	}
}
