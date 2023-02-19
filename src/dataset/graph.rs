use std::collections::BTreeSet;

use hashbrown::HashMap;
use slab::Slab;

use crate::{Triple, Pattern, Cause, TripleExt, Id};

pub struct Graph<M> {
	/// All the triples in the graph.
	statements: Slab<Statement<M>>,
	resources: HashMap<Id, Resource>
}

pub struct Statement<M> {
	pub triple: Triple,
	pub positive: bool,
	pub cause: Cause<M>
}

pub struct StatementMut<'a, M>(&'a mut Statement<M>);

impl<'a, M> StatementMut<'a, M> {
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

pub struct Resource {
	as_subject: BTreeSet<usize>,
	as_predicate: BTreeSet<usize>,
	as_object: BTreeSet<usize>,
}

pub struct Contradiction(Triple);

impl<M> Graph<M> {
	pub fn insert(&mut self, triple: Triple, positive: bool, cause: Cause<M>) -> Result<usize, Contradiction> {
		match self.find_triple_mut(triple) {
			Some((i, statement)) => {
				if statement.is_positive() == positive {
					Ok(i)
				} else {
					Err(Contradiction(triple))
				}
			}
			None => {
				let i = self.statements.insert(Statement { triple, positive, cause });
				self.resources.get_mut(triple.subject()).unwrap().as_subject.insert(i);
				self.resources.get_mut(triple.predicate()).unwrap().as_predicate.insert(i);
				self.resources.get_mut(triple.object()).unwrap().as_object.insert(i);
				Ok(i)
			}
		}
	}

	pub fn remove_triple_with(&mut self, triple: Triple, f: impl FnOnce(&Statement<M>) -> bool) -> Option<Statement<M>> {
		let i = self.find_triple(triple).filter(|(_, s)| f(s)).map(|(i, _)| i);
		i.map(|i| {
			self.resources.get_mut(triple.subject()).unwrap().as_subject.remove(&i);
			self.resources.get_mut(triple.predicate()).unwrap().as_predicate.remove(&i);
			self.resources.get_mut(triple.object()).unwrap().as_object.remove(&i);
			self.statements.remove(i)
		})
	}

	pub fn remove_triple(&mut self, triple: Triple) -> Option<Statement<M>> {
		self.remove_triple_with(triple, |_| true)
	}

	pub fn remove_positive_triple(&mut self, triple: Triple) -> Option<Statement<M>> {
		self.remove_triple_with(triple, |s| s.positive)
	}

	pub fn remove_negative_triple(&mut self, triple: Triple) -> Option<Statement<M>> {
		self.remove_triple_with(triple, |s| !s.positive)
	}

	pub fn remove_resource(&mut self, id: Id) -> Vec<Statement<M>> {
		let mut statements = Vec::new();

		if let Some(r) = self.resources.remove(&id) {
			for i in r.as_subject {
				if let Some(s) = self.statements.try_remove(i) {
					statements.push(s)
				}
			}

			for i in r.as_predicate {
				if let Some(s) = self.statements.try_remove(i) {
					statements.push(s)
				}
			}

			for i in r.as_object {
				if let Some(s) = self.statements.try_remove(i) {
					statements.push(s)
				}
			}
		}

		statements
	}

	pub fn find_triple(&self, triple: Triple) -> Option<(usize, &Statement<M>)> {
		self.matching(triple.into_pattern()).next()
	}

	pub fn find_triple_mut(&mut self, triple: Triple) -> Option<(usize, StatementMut<M>)> {
		self.matching_mut(triple.into_pattern()).next()
	}

	fn matching(&self, rdf_types::Triple(s, p, o): Pattern) -> Matching<M> {
		if s.is_none() && p.is_none() && o.is_none() {
			Matching::All(self.statements.iter())
		} else {
			Matching::Constrained {
				statements: &self.statements,
				subject: s.map(|s| self.resources[&s].as_subject.iter()),
				predicate: p.map(|p| self.resources[&p].as_predicate.iter()),
				object: o.map(|o| self.resources[&o].as_object.iter())
			}
		}
	}

	fn matching_mut(&mut self, rdf_types::Triple(s, p, o): Pattern) -> MatchingMut<M> {
		if s.is_none() && p.is_none() && o.is_none() {
			MatchingMut::All(self.statements.iter_mut())
		} else {
			MatchingMut::Constrained {
				statements: &mut self.statements,
				subject: s.map(|s| self.resources[&s].as_subject.iter()),
				predicate: p.map(|p| self.resources[&p].as_predicate.iter()),
				object: o.map(|o| self.resources[&o].as_object.iter())
			}
		}
	}
}

pub enum Matching<'a, M> {
	All(slab::Iter<'a, Statement<M>>),
	Constrained {
		statements: &'a Slab<Statement<M>>,
		subject: Option<std::collections::btree_set::Iter<'a, usize>>,
		predicate: Option<std::collections::btree_set::Iter<'a, usize>>,
		object: Option<std::collections::btree_set::Iter<'a, usize>>
	}
}

impl<'a, M> Iterator for Matching<'a, M> {
	type Item = (usize, &'a Statement<M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::All(iter) => iter.next(),
			Self::Constrained { statements, subject, predicate, object } => {
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

				candidate.map(|i| (i, &statements[i]))
			}
		}
	}
}

pub enum MatchingMut<'a, M> {
	All(slab::IterMut<'a, Statement<M>>),
	Constrained {
		statements: &'a mut Slab<Statement<M>>,
		subject: Option<std::collections::btree_set::Iter<'a, usize>>,
		predicate: Option<std::collections::btree_set::Iter<'a, usize>>,
		object: Option<std::collections::btree_set::Iter<'a, usize>>
	}
}

impl<'a, M> Iterator for MatchingMut<'a, M> {
	type Item = (usize, StatementMut<'a, M>);

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::All(iter) => iter.next().map(|(i, s)| (i, StatementMut(s))),
			Self::Constrained { statements, subject, predicate, object } => {
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
					let statement: &'a mut Statement<M> = unsafe {
						// This is safe because the iterator does not yield
						// aliased items.
						std::mem::transmute(&mut statements[i])
					};

					(i, StatementMut(statement))
				})
			}
		}
	}
}