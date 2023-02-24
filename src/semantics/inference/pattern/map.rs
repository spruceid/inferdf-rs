use std::hash::Hash;

use derivative::Derivative;
use hashbrown::{HashMap, HashSet};

use crate::{Id, Triple};

use super::{Pattern, AnySubjectGivenPredicate, AnySubjectAnyPredicate, AnySubject, GivenSubjectGivenPredicate, GivenSubjectAnyPredicate, GivenSubject};

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct PatternMap<V> {
	any: AnySubjectMap<V>,
	given: HashMap<Id, GivenSubjectMap<V>>
}

impl<V: Eq + Hash> PatternMap<V> {
	pub fn insert(&mut self, pattern: Pattern, value: V) -> bool {
		match pattern {
			Pattern::AnySubject(rest) => self.any.insert(rest, value),
			Pattern::GivenSubject(id, rest) => self.given.entry(id).or_default().insert(rest, value)
		}
	}
}

impl<V> PatternMap<V> {
	pub fn get(&self, triple: Triple) -> Values<V> {
		Values {
			any: self.any.get(triple),
			given: self.given.get(triple.subject()).map(|s| s.get(triple))
		}
	}
}

pub struct Values<'a, V> {
	any: AnySubjectValues<'a, V>,
	given: Option<GivenSubjectValues<'a, V>>
}

impl<'a, V> Iterator for Values<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		self.any.next().or_else(|| {
			self.given.as_mut().and_then(|i| i.next())
		})
	}
}

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct GivenSubjectMap<V> {
	any: GivenSubjectAnyPredicateMap<V>,
	given: HashMap<Id, GivenSubjectGivenPredicateMap<V>>
}

impl<V: Eq + Hash> GivenSubjectMap<V> {
	pub fn insert(&mut self, pattern: GivenSubject, value: V) -> bool {
		match pattern {
			GivenSubject::AnyPredicate(rest) => self.any.insert(rest, value),
			GivenSubject::GivenPredicate(id, rest) => self.given.entry(id).or_default().insert(rest, value)
		}
	}
}

impl<V> GivenSubjectMap<V> {
	pub fn get(&self, triple: Triple) -> GivenSubjectValues<V> {
		GivenSubjectValues {
			any: self.any.get(triple),
			given: self.given.get(triple.predicate()).map(|p| p.get(triple))
		}
	}
}

pub struct GivenSubjectValues<'a, V> {
	any: GivenSubjectAnyPredicateValues<'a, V>,
	given: Option<GivenSubjectGivenPredicateValues<'a, V>>
}

impl<'a, V> Iterator for GivenSubjectValues<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		self.any.next().or_else(|| {
			self.given.as_mut().and_then(|i| i.next())
		})
	}
}

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct GivenSubjectAnyPredicateMap<V> {
	any: HashSet<V>,
	same_as_predicate: HashSet<V>,
	given: HashMap<Id, HashSet<V>>
}

impl<V: Eq + Hash> GivenSubjectAnyPredicateMap<V> {
	pub fn insert(&mut self, pattern: GivenSubjectAnyPredicate, value: V) -> bool {
		match pattern {
			GivenSubjectAnyPredicate::AnyObject => self.any.insert(value),
			GivenSubjectAnyPredicate::SameAsPredicate => self.same_as_predicate.insert(value),
			GivenSubjectAnyPredicate::GivenObject(id) => self.given.entry(id).or_default().insert(value)
		}
	}
}

impl<V> GivenSubjectAnyPredicateMap<V> {
	pub fn get(&self, triple: Triple) -> GivenSubjectAnyPredicateValues<V> {
		GivenSubjectAnyPredicateValues {
			any: self.any.iter(),
			same_as_predicate: if triple.predicate() == triple.object() {
				Some(self.same_as_predicate.iter())
			} else {
				None
			},
			given: self.given.get(triple.object()).map(|o| o.iter())
		}
	}
}

pub struct GivenSubjectAnyPredicateValues<'a, V> {
	any: hashbrown::hash_set::Iter<'a, V>,
	same_as_predicate: Option<hashbrown::hash_set::Iter<'a, V>>,
	given: Option<hashbrown::hash_set::Iter<'a, V>>
}

impl<'a, V> Iterator for GivenSubjectAnyPredicateValues<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		self.any.next().or_else(|| {
			self.same_as_predicate.as_mut().and_then(|i| i.next())
		}).or_else(|| {
			self.given.as_mut().and_then(|i| i.next())
		})
	}
}

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct GivenSubjectGivenPredicateMap<V> {
	any: HashSet<V>,
	given: HashMap<Id, HashSet<V>>
}

impl<V: Eq + Hash> GivenSubjectGivenPredicateMap<V> {
	pub fn insert(&mut self, pattern: GivenSubjectGivenPredicate, value: V) -> bool {
		match pattern {
			GivenSubjectGivenPredicate::AnyObject => self.any.insert(value),
			GivenSubjectGivenPredicate::GivenObject(id) => self.given.entry(id).or_default().insert(value)
		}
	}
}

impl<V> GivenSubjectGivenPredicateMap<V> {
	pub fn get(&self, triple: Triple) -> GivenSubjectGivenPredicateValues<V> {
		GivenSubjectGivenPredicateValues {
			any: self.any.iter(),
			given: self.given.get(triple.object()).map(|o| o.iter())
		}
	}
}

pub struct GivenSubjectGivenPredicateValues<'a, V> {
	any: hashbrown::hash_set::Iter<'a, V>,
	given: Option<hashbrown::hash_set::Iter<'a, V>>
}

impl<'a, V> Iterator for GivenSubjectGivenPredicateValues<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		self.any.next().or_else(|| {
			self.given.as_mut().and_then(|i| i.next())
		})
	}
}

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct AnySubjectMap<V> {
	any: AnySubjectAnyPredicateMap<V>,
	same_as_subject: AnySubjectGivenPredicateMap<V>,
	given: HashMap<Id, AnySubjectGivenPredicateMap<V>>
}

impl<V: Eq + Hash> AnySubjectMap<V> {
	pub fn insert(&mut self, pattern: AnySubject, value: V) -> bool {
		match pattern {
			AnySubject::AnyPredicate(rest) => self.any.insert(rest, value),
			AnySubject::SameAsSubject(rest) => self.same_as_subject.insert(rest, value),
			AnySubject::GivenPredicate(id, rest) => self.given.entry(id).or_default().insert(rest, value)
		}
	}
}

impl<V> AnySubjectMap<V> {
	pub fn get(&self, triple: Triple) -> AnySubjectValues<V> {
		AnySubjectValues {
			any: self.any.get(triple),
			same_as_subject: if triple.subject() == triple.predicate() {
				Some(self.same_as_subject.get(triple))
			} else {
				None
			},
			given: self.given.get(triple.predicate()).map(|p| p.get(triple))
		}
	}
}

pub struct AnySubjectValues<'a, V> {
	any: AnySubjectAnyPredicateValues<'a, V>,
	same_as_subject: Option<AnySubjectGivenPredicateValues<'a, V>>,
	given: Option<AnySubjectGivenPredicateValues<'a, V>>
}

impl<'a, V> Iterator for AnySubjectValues<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		self.any.next().or_else(|| {
			self.same_as_subject.as_mut().and_then(|i| i.next())
		}).or_else(|| {
			self.given.as_mut().and_then(|i| i.next())
		})
	}
}

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct AnySubjectAnyPredicateMap<V> {
	any: HashSet<V>,
	same_as_subject: HashSet<V>,
	same_as_predicate: HashSet<V>,
	given: HashMap<Id, HashSet<V>>
}

impl<V: Eq + Hash> AnySubjectAnyPredicateMap<V> {
	pub fn insert(&mut self, pattern: AnySubjectAnyPredicate, value: V) -> bool {
		match pattern {
			AnySubjectAnyPredicate::AnyObject => self.any.insert(value),
			AnySubjectAnyPredicate::GivenObject(id) => self.given.entry(id).or_default().insert(value)
		}
	}
}

impl<V> AnySubjectAnyPredicateMap<V> {
	pub fn get(&self, triple: Triple) -> AnySubjectAnyPredicateValues<V> {
		AnySubjectAnyPredicateValues {
			any: self.any.iter(),
			same_as_subject: if triple.subject() == triple.object() {
				Some(self.same_as_subject.iter())
			} else {
				None
			},
			same_as_predicate: if triple.predicate() == triple.object() {
				Some(self.same_as_predicate.iter())
			} else {
				None
			},
			given: self.given.get(triple.object()).map(|o| o.iter())
		}
	}
}

pub struct AnySubjectAnyPredicateValues<'a, V> {
	any: hashbrown::hash_set::Iter<'a, V>,
	same_as_subject: Option<hashbrown::hash_set::Iter<'a, V>>,
	same_as_predicate: Option<hashbrown::hash_set::Iter<'a, V>>,
	given: Option<hashbrown::hash_set::Iter<'a, V>>
}

impl<'a, V> Iterator for AnySubjectAnyPredicateValues<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		self.any.next().or_else(|| {
			self.same_as_subject.as_mut().and_then(|i| i.next())
		}).or_else(|| {
			self.same_as_predicate.as_mut().and_then(|i| i.next())
		}).or_else(|| {
			self.given.as_mut().and_then(|i| i.next())
		})
	}
}

#[derive(Derivative)]
#[derivative(Default(bound=""))]
pub struct AnySubjectGivenPredicateMap<V> {
	any: HashSet<V>,
	same_as_subject: HashSet<V>,
	given: HashMap<Id, HashSet<V>>
}

impl<V: Eq + Hash> AnySubjectGivenPredicateMap<V> {
	pub fn insert(&mut self, pattern: AnySubjectGivenPredicate, value: V) -> bool {
		match pattern {
			AnySubjectGivenPredicate::AnyObject => self.any.insert(value),
			AnySubjectGivenPredicate::SameAsSubject => self.same_as_subject.insert(value),
			AnySubjectGivenPredicate::GivenObject(id) => self.given.entry(id).or_default().insert(value)
		}
	}
}

impl<V> AnySubjectGivenPredicateMap<V> {
	pub fn get(&self, triple: Triple) -> AnySubjectGivenPredicateValues<V> {
		AnySubjectGivenPredicateValues {
			any: self.any.iter(),
			same_as_subject: if triple.subject() == triple.object() {
				Some(self.same_as_subject.iter())
			} else {
				None
			},
			given: self.given.get(triple.object()).map(|o| o.iter())
		}
	}
}

pub struct AnySubjectGivenPredicateValues<'a, V> {
	any: hashbrown::hash_set::Iter<'a, V>,
	same_as_subject: Option<hashbrown::hash_set::Iter<'a, V>>,
	given: Option<hashbrown::hash_set::Iter<'a, V>>
}

impl<'a, V> Iterator for AnySubjectGivenPredicateValues<'a, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		self.any.next().or_else(|| {
			self.same_as_subject.as_mut().and_then(|i| i.next())
		}).or_else(|| {
			self.given.as_mut().and_then(|i| i.next())
		})
	}
}