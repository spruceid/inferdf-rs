use std::hash::Hash;

use hashbrown::{HashMap, HashSet};

use crate::{Id, Triple, Quad};

pub trait ReplaceId {
	/// Replace id `a` with `b`.
	fn replace_id(&mut self, a: Id, b: Id);
}

impl ReplaceId for Id {
	fn replace_id(&mut self, a: Id, b: Id) {
		if *self == a {
			*self = b
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

impl<T: ReplaceId> ReplaceId for Vec<T> {
	fn replace_id(&mut self, a: Id, b: Id) {
		for t in self {
			t.replace_id(a, b)
		}
	}
}

impl<T: ReplaceId + Eq + Hash> ReplaceId for HashSet<T> {
	fn replace_id(&mut self, a: Id, b: Id) {
		for mut t in std::mem::take(self) {
			t.replace_id(a, b);
			self.insert(t);
		}
	}
}

impl<T: ReplaceId + super::Union> ReplaceId for HashMap<Id, T> {
	fn replace_id(&mut self, a: Id, b: Id) {
		for t in self.values_mut() {
			t.replace_id(a, b);
		}
		
		if let Some(t) = self.remove(&a) {
			match self.entry(b) {
				hashbrown::hash_map::Entry::Vacant(entry) => {
					entry.insert(t);
				}
				hashbrown::hash_map::Entry::Occupied(mut entry) => {
					entry.get_mut().union_with(t)
				}
			}
		}
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