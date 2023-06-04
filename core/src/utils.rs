mod replace_id;
mod search;
mod sign;

use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
pub use replace_id::*;
pub use search::*;
pub use sign::*;

/// Any collection or iterator that can accurately report if it is empty.
pub trait IsEmpty {
	fn is_empty(&self) -> bool;
}

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

pub trait FailibleIterator {
	type Item;
	type Error;

	fn try_next(&mut self) -> Result<Option<Self::Item>, Self::Error>;
}

pub trait IteratorWith<V> {
	type Item;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item>;
}

pub trait TryCollect {
	type Item;
	type Error;

	fn try_collect(self) -> Result<Vec<Self::Item>, Self::Error>;
}

impl<I: Iterator<Item = Result<J, E>>, J, E> TryCollect for I {
	type Item = J;
	type Error = E;

	fn try_collect(self) -> Result<Vec<Self::Item>, Self::Error> {
		let mut result = Vec::new();

		for item in self {
			result.push(item?);
		}

		Ok(result)
	}
}

pub trait GetOrTryInsertWith {
	type Item;

	fn get_or_try_insert_with<E>(
		&mut self,
		f: impl FnOnce() -> Result<Self::Item, E>,
	) -> Result<&mut Self::Item, E>;
}

impl<T> GetOrTryInsertWith for Option<T> {
	type Item = T;

	fn get_or_try_insert_with<E>(
		&mut self,
		f: impl FnOnce() -> Result<Self::Item, E>,
	) -> Result<&mut Self::Item, E> {
		match self {
			None => {
				*self = Some(f()?);
				Ok(unsafe { self.as_mut().unwrap_unchecked() })
			}
			Some(value) => Ok(value),
		}
	}
}

pub trait DivCeil {
	fn div_ceil(self, rhs: Self) -> Self;
}

impl DivCeil for u32 {
	fn div_ceil(self, rhs: Self) -> Self {
		(self + rhs - 1) / rhs
	}
}
