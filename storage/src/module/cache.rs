use std::{
	cell::{Cell, RefCell},
	collections::HashMap,
	hash::Hash,
};

mod slab;

use slab::ConstSlab;

pub use slab::{Aliasing, IntoIterEscape, Ref, RefMut};

const DEFAULT_CHUNK_LEN: usize = 64;

#[derive(Debug)]
pub struct NotEnoughMemory<T>(pub T);

#[derive(Debug)]
pub struct Busy;

pub struct CacheMap<K, V, const N: usize = DEFAULT_CHUNK_LEN> {
	slab: ConstSlab<V, N>,
	map: RefCell<HashMap<K, usize>>,
	priority: PriorityList,
}

impl<K, V, const N: usize> Default for CacheMap<K, V, N> {
	fn default() -> Self {
		Self {
			slab: ConstSlab::default(),
			map: RefCell::new(HashMap::new()),
			priority: PriorityList::default(),
		}
	}
}

impl<K, V, const N: usize> CacheMap<K, V, N> {
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			slab: ConstSlab::with_capacity(capacity),
			map: RefCell::new(HashMap::new()),
			priority: PriorityList::default(),
		}
	}

	pub fn new() -> Self {
		Self::default()
	}
}

impl<K: Hash + Eq, V, const N: usize> CacheMap<K, V, N> {
	fn get_index<E>(&self, key: K, value: impl FnOnce() -> Result<V, E>) -> Result<usize, Error<V, E>> {
		use std::collections::hash_map::Entry;
		let mut map = self.map.borrow_mut();
		match map.entry(key) {
			Entry::Vacant(e) => match self.slab.insert(value().map_err(Error::IO)?) {
				Ok(index) => {
					self.priority.insert(index);
					Ok(*e.insert(index))
				}
				Err(NotEnoughMemory(v)) => match self.priority.tail() {
					Some(mut i) => loop {
						match self.slab.remove(i) {
							Ok(_) => {
								self.priority.remove(i);
								break match self.slab.insert(v) {
									Ok(index) => {
										self.priority.insert(index);
										Ok(*e.insert(index))
									}
									Err(NotEnoughMemory(v)) => Err(Error::NotEnoughMemory(v)),
								};
							}
							Err(Busy) => match self.priority.prev(i) {
								Some(j) => i = j,
								None => break Err(Error::NotEnoughMemory(v)),
							},
						}
					},
					None => Err(Error::NotEnoughMemory(v)),
				},
			},
			Entry::Occupied(e) => {
				let index = *e.get();
				self.priority.remove(index);
				self.priority.insert(index);
				Ok(index)
			}
		}
	}

	pub fn get<E>(&self, key: K, value: impl FnOnce() -> Result<V, E>) -> Result<Ref<V>, Error<V, E>> {
		let index = self.get_index(key, value)?;
		Ok(self.slab.get(index)?.unwrap())
	}

	pub fn get_mut<E>(&self, key: K, value: impl FnOnce() -> Result<V, E>) -> Result<RefMut<V>, Error<V, E>> {
		let index = self.get_index(key, value)?;
		Ok(self.slab.get_mut(index)?.unwrap())
	}
}

#[derive(Debug)]
pub enum Error<T, E> {
	IO(E),
	NotEnoughMemory(T),
	Busy,
}

impl<T, E> From<NotEnoughMemory<T>> for Error<T, E> {
	fn from(NotEnoughMemory(t): NotEnoughMemory<T>) -> Self {
		Self::NotEnoughMemory(t)
	}
}

impl<T, E> From<Busy> for Error<T, E> {
	fn from(_value: Busy) -> Self {
		Self::Busy
	}
}

#[derive(Default)]
struct PriorityList {
	map: RefCell<HashMap<usize, Priority>>,
	head: Cell<Option<usize>>,
	tail: Cell<Option<usize>>,
}

impl PriorityList {
	fn tail(&self) -> Option<usize> {
		self.tail.get()
	}

	fn prev(&self, i: usize) -> Option<usize> {
		let map = self.map.borrow();
		map.get(&i).unwrap().prev
	}

	fn insert(&self, i: usize) {
		let mut map = self.map.borrow_mut();
		map.insert(i, Priority::new(None, self.head.get()));

		if let Some(h) = self.head.get() {
			map.get_mut(&h).unwrap().prev = Some(i)
		}
		self.head.set(Some(i));

		if self.tail.get().is_none() {
			self.tail.set(Some(i));
		}
	}

	fn remove(&self, i: usize) {
		let mut map = self.map.borrow_mut();
		let p = map.remove(&i).unwrap();

		match p.prev {
			Some(j) => map.get_mut(&j).unwrap().next = p.next,
			None => self.head.set(p.next),
		}

		match p.next {
			Some(j) => map.get_mut(&j).unwrap().prev = p.prev,
			None => self.tail.set(p.prev),
		}
	}
}

struct Priority {
	prev: Option<usize>,
	next: Option<usize>,
}

impl Priority {
	fn new(prev: Option<usize>, next: Option<usize>) -> Self {
		Self { prev, next }
	}
}

#[cfg(test)]
mod tests {
	use std::convert::Infallible;

use super::*;

	#[test]
	fn insert_small_chunks() {
		let map = CacheMap::<u32, (), 1>::new();
		map.get::<Infallible>(0, || Ok(())).unwrap();
		map.get::<Infallible>(1, || Ok(())).unwrap();
		map.get::<Infallible>(2, || Ok(())).unwrap();
	}

	#[test]
	fn insert_low_capacity() {
		let map = CacheMap::<u32, char, 1>::with_capacity(2);
		let a = map.get::<Infallible>(0, || Ok('A')).unwrap();
		let b = map.get::<Infallible>(1, || Ok('B')).unwrap();
		assert!(map.get::<Infallible>(2, || Ok('C')).is_err());
		eprintln!("{a}, {b}")
	}

	#[test]
	fn borrow_rules_1() {
		let map = CacheMap::<u32, char>::new();
		let a = map.get_mut::<Infallible>(0, || Ok('A')).unwrap();
		assert!(map.get::<Infallible>(0, || Ok('B')).is_err());
		eprintln!("{a}")
	}

	#[test]
	fn borrow_rules_2() {
		let map = CacheMap::<u32, char>::new();
		let a = map.get::<Infallible>(0, || Ok('A')).unwrap();
		let b = map.get::<Infallible>(0, || Ok('B')).unwrap();
		assert_eq!(a, b)
	}

	#[test]
	fn drop_reference() {
		let map = CacheMap::<u32, char>::new();
		let a = map.get_mut::<Infallible>(0, || Ok('A')).unwrap();
		assert!(map.get::<Infallible>(0, || Ok('B')).is_err());
		eprintln!("{a}");
		std::mem::drop(a);
		map.get::<Infallible>(0, || Ok('B')).unwrap();
	}
}
