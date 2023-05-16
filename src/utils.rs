mod replace_id;
mod search;
mod sign;

use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
pub use replace_id::*;
pub use search::*;
pub use sign::*;

pub trait Union {
	fn union_with(&mut self, other: Self);
}

impl<T: Eq + Hash> Union for HashSet<T> {
	fn union_with(&mut self, other: Self) {
		self.extend(other)
	}
}

impl<K: Eq + Hash, V: Union> Union for HashMap<K, V> {
	fn union_with(&mut self, other: Self) {
		for (k, v) in other {
			match self.entry(k) {
				hashbrown::hash_map::Entry::Vacant(entry) => {
					entry.insert(v);
				}
				hashbrown::hash_map::Entry::Occupied(mut entry) => entry.get_mut().union_with(v),
			}
		}
	}
}
