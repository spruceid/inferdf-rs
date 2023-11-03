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

pub trait IteratorWith<V>: Sized {
	type Item;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item>;

	fn find_with(
		&mut self,
		vocabulary: &mut V,
		f: impl Fn(&Self::Item) -> bool,
	) -> Option<Self::Item> {
		while let Some(t) = self.next_with(vocabulary) {
			if f(&t) {
				return Some(t);
			}
		}

		None
	}

	fn iter_with(self, vocabulary: &mut V) -> IterWith<Self, V> {
		IterWith {
			iter: self,
			vocabulary,
		}
	}

	fn map<F>(self, f: F) -> Map<Self, F> {
		Map { inner: self, f }
	}

	fn try_flat_map<E, J, F, T, U>(self, f: F) -> TryFlatMap<Self, J, F>
	where
		Self: IteratorWith<V, Item = Result<T, E>>,
		J: IteratorWith<V, Item = Result<U, E>>,
		F: Fn(T) -> J,
	{
		TryFlatMap {
			inner: self,
			f,
			current: None,
		}
	}
}

impl<V, T> IteratorWith<V> for std::iter::Empty<T> {
	type Item = T;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		None
	}
}

pub struct TryFlatMap<I, J, F> {
	inner: I,
	f: F,
	current: Option<J>,
}

impl<T, U, E, I, J, F, V> IteratorWith<V> for TryFlatMap<I, J, F>
where
	I: IteratorWith<V, Item = Result<T, E>>,
	J: IteratorWith<V, Item = Result<U, E>>,
	F: FnMut(T) -> J,
{
	type Item = Result<U, E>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(j) => match j.next_with(vocabulary) {
					Some(u) => break Some(u),
					None => self.current = None,
				},
				None => match self.inner.next_with(vocabulary) {
					Some(Ok(t)) => self.current = Some((self.f)(t)),
					Some(Err(e)) => break Some(Err(e)),
					None => break None,
				},
			}
		}
	}
}

pub struct IterWith<'a, I, V> {
	iter: I,
	vocabulary: &'a mut V,
}

impl<'a, I: IteratorWith<V>, V> Iterator for IterWith<'a, I, V> {
	type Item = I::Item;

	fn next(&mut self) -> Option<Self::Item> {
		self.iter.next_with(self.vocabulary)
	}
}

pub struct Map<I, F> {
	inner: I,
	f: F,
}

impl<V, I: IteratorWith<V>, U, F: FnMut(I::Item) -> U> IteratorWith<V> for Map<I, F> {
	type Item = U;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.inner.next_with(vocabulary).map(&mut self.f)
	}
}

pub trait FailibleIteratorWith<V> {
	type Item;
	type Error;

	fn try_next_with(&mut self, vocabulary: &mut V) -> Result<Option<Self::Item>, Self::Error>;
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

pub trait TryCollectWith<V> {
	type Item;
	type Error;

	fn try_collect_with(self, vocabulary: &mut V) -> Result<Vec<Self::Item>, Self::Error>;
}

impl<V, I: IteratorWith<V, Item = Result<J, E>>, J, E> TryCollectWith<V> for I {
	type Item = J;
	type Error = E;

	fn try_collect_with(mut self, vocabulary: &mut V) -> Result<Vec<Self::Item>, Self::Error> {
		let mut result = Vec::new();

		while let Some(item) = self.next_with(vocabulary) {
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
