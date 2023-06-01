use core::fmt;
use std::{
	cell::{Cell, RefCell},
	hash::{Hash, Hasher},
	ops::{Deref, DerefMut},
};

use super::{Busy, NotEnoughMemory, DEFAULT_CHUNK_LEN};

pub struct ConstSlab<T, const N: usize = DEFAULT_CHUNK_LEN> {
	chunks: RefCell<Vec<Chunk<T, N>>>,
	capacity: usize,
	len: Cell<usize>,
	next: Cell<Option<usize>>,
}

impl<T, const N: usize> Default for ConstSlab<T, N> {
	fn default() -> Self {
		Self::with_capacity(usize::MAX)
	}
}

impl<T, const N: usize> ConstSlab<T, N> {
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			chunks: RefCell::new(Vec::new()),
			capacity,
			len: Cell::new(0),
			next: Cell::new(None),
		}
	}

	fn add_chunk(&self) -> Result<(), NotEnoughMemory<()>> {
		let mut chunks = self.chunks.borrow_mut();

		if chunks.len() < self.capacity {
			chunks.push(Chunk::new(self.len.get()));
			Ok(())
		} else {
			Err(NotEnoughMemory(()))
		}
	}

	fn chunk_index(&self, index: usize) -> (usize, usize) {
		(index / N, index % N)
	}

	pub fn get(&self, index: usize) -> Result<Option<Ref<T>>, Busy> {
		let (c, j) = self.chunk_index(index);
		let chunks = self.chunks.borrow();

		match chunks.get(c) {
			Some(chunk) => unsafe {
				// Safety: the chunk won't be deleted before the returned reference
				// if dropped.
				chunk.get(j)
			},
			None => Ok(None),
		}
	}

	pub fn get_mut(&self, index: usize) -> Result<Option<RefMut<T>>, Busy> {
		let (c, j) = self.chunk_index(index);
		let chunks = self.chunks.borrow();

		match chunks.get(c) {
			Some(chunk) => unsafe {
				// Safety: the chunk won't be deleted before the returned reference
				// if dropped.
				chunk.get_mut(j)
			},
			None => Ok(None),
		}
	}

	pub fn insert(&self, value: T) -> Result<usize, NotEnoughMemory<T>> {
		let i = match self.next.get() {
			Some(i) => i,
			None => {
				if self.add_chunk().is_err() {
					return Err(NotEnoughMemory(value));
				}

				self.len.get()
			}
		};

		let (c, j) = self.chunk_index(i);

		let chunks = self.chunks.borrow();
		self.next.set(chunks[c].insert(j, value));
		self.len.set(self.len.get() + 1);
		Ok(i)
	}

	pub fn remove(&self, index: usize) -> Result<Option<T>, Busy> {
		let (c, j) = self.chunk_index(index);
		let chunks = self.chunks.borrow();
		match chunks.get(c) {
			Some(chunk) => match chunk.remove(j, self.next.get())? {
				Some(result) => {
					self.next.set(Some(index));
					self.len.set(self.len.get() - 1);
					Ok(Some(result))
				}
				None => Ok(None),
			},
			None => Ok(None),
		}
	}
}

pub enum Entry<T> {
	Vacant(Option<usize>),
	Occupied(OccupiedEntry<T>),
}

pub struct OccupiedEntry<T> {
	value: T,
	borrow: Cell<BorrowState>,
}

impl<T> Entry<T> {
	/// Borrow the given entry.
	///
	/// # Safety
	///
	/// The input `ptr` must be a valid pointer to an entry.
	unsafe fn borrow<'a>(ptr: *const Entry<T>) -> Result<Option<Ref<'a, T>>, Busy> {
		let entry = &*ptr;

		match entry {
			Self::Occupied(e) => {
				e.borrow.set(e.borrow.get().add_reader()?);
				Ok(Some(Ref {
					value: &e.value,
					borrow: &e.borrow,
				}))
			}
			Self::Vacant(_) => Ok(None),
		}
	}

	/// Borrow the given entry mutably.
	///
	/// # Safety
	///
	/// The input `ptr` must be a valid pointer to an entry.
	unsafe fn borrow_mut<'a>(ptr: *mut Entry<T>) -> Result<Option<RefMut<'a, T>>, Busy> {
		let entry = &mut *ptr;

		match entry {
			Self::Occupied(e) => {
				e.borrow.set(e.borrow.get().add_writer()?);
				Ok(Some(RefMut {
					value: &mut e.value,
					borrow: &e.borrow,
				}))
			}
			Self::Vacant(_) => Ok(None),
		}
	}

	/// Insert a value in the given vacant entry.
	///
	/// # Safety
	///
	/// The input `ptr` must be a valid pointer to a vacant entry.
	unsafe fn insert(ptr: *mut Entry<T>, value: T) -> Option<usize> {
		let entry = &mut *ptr;
		match entry {
			Self::Occupied(_) => panic!("entry is occupied"),
			Self::Vacant(index) => {
				let i = *index;
				std::ptr::write(
					ptr,
					Entry::Occupied(OccupiedEntry {
						value,
						borrow: Cell::new(BorrowState::new()),
					}),
				);
				i
			}
		}
	}

	/// Remove the value at the given entry, if any.
	///
	/// # Safety
	///
	/// The input `ptr` must be a valid pointer to an entry.
	unsafe fn remove(
		ptr: *mut Entry<T>,
		next_free_index: Option<usize>,
	) -> Result<Option<T>, Busy> {
		let entry = &mut *ptr;
		match entry {
			Self::Occupied(e) => {
				if e.borrow.get().can_remove() {
					let result = std::ptr::read(&e.value);
					std::ptr::write(ptr, Entry::Vacant(next_free_index));
					Ok(Some(result))
				} else {
					Err(Busy)
				}
			}
			Self::Vacant(_) => Ok(None),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowState {
	read: usize,
	write: usize,
}

impl BorrowState {
	pub fn new() -> Self {
		Self { read: 0, write: 0 }
	}

	pub fn can_remove(&self) -> bool {
		self.read == 0 && self.write == 0
	}

	pub fn add_reader(self) -> Result<Self, Busy> {
		if self.write == 0 {
			Ok(Self {
				read: self.read + 1,
				write: 0,
			})
		} else {
			Err(Busy)
		}
	}

	pub fn remove_reader(self) -> Self {
		Self {
			read: self.read - 1,
			write: self.write,
		}
	}

	pub fn add_writer(self) -> Result<Self, Busy> {
		if self.read == 0 && self.write == 0 {
			Ok(Self { read: 0, write: 1 })
		} else {
			Err(Busy)
		}
	}

	pub fn remove_writer(self) -> Self {
		Self {
			read: self.read,
			write: self.write - 1,
		}
	}

	pub fn downgrade_writer(self) -> Self {
		Self {
			read: self.read + 1,
			write: self.write - 1,
		}
	}

	// pub fn split_writer(self) -> Self {
	// 	Self {
	// 		read: self.read,
	// 		write: self.write + 1
	// 	}
	// }

	pub fn clone_reader(self) -> Self {
		Self {
			read: self.read + 1,
			write: self.write,
		}
	}
}

struct Chunk<T, const N: usize = DEFAULT_CHUNK_LEN> {
	entries: *mut Entry<T>,
}

impl<T, const N: usize> Chunk<T, N> {
	fn new(first_index: usize) -> Self {
		let layout = std::alloc::Layout::array::<Entry<T>>(N).unwrap();
		let values: *mut Entry<T> = unsafe { std::alloc::alloc(layout) as *mut _ };

		for i in 0..N {
			unsafe {
				let ptr = values.add(i);
				let next = if i + 1 < N {
					Some(first_index + i + 1)
				} else {
					None
				};

				std::ptr::write(ptr, Entry::Vacant(next))
			}
		}

		Self { entries: values }
	}

	/// Get a mutable reference to the item at the given `index`.
	///
	/// # Safety
	///
	/// The chunk must outlive the lifetime `'a`.
	pub unsafe fn get<'a>(&self, index: usize) -> Result<Option<Ref<'a, T>>, Busy> {
		if index < N {
			unsafe { Entry::borrow(self.entries.add(index)) }
		} else {
			Ok(None)
		}
	}

	/// Get a mutable reference to the item at the given `index`.
	///
	/// # Safety
	///
	/// The chunk must outlive the lifetime `'a`.
	pub unsafe fn get_mut<'a>(&self, index: usize) -> Result<Option<RefMut<'a, T>>, Busy> {
		if index < N {
			unsafe { Entry::borrow_mut(self.entries.add(index)) }
		} else {
			Ok(None)
		}
	}

	pub fn insert(&self, index: usize, value: T) -> Option<usize> {
		assert!(index < N);
		unsafe { Entry::insert(self.entries.add(index), value) }
	}

	pub fn remove(&self, index: usize, next_free_index: Option<usize>) -> Result<Option<T>, Busy> {
		if index < N {
			unsafe { Entry::remove(self.entries.add(index), next_free_index) }
		} else {
			Ok(None)
		}
	}
}

impl<T, const N: usize> Drop for Chunk<T, N> {
	fn drop(&mut self) {
		let layout = std::alloc::Layout::array::<Entry<T>>(N).unwrap();
		unsafe { std::alloc::dealloc(self.entries as *mut u8, layout) }
	}
}

pub struct RefMut<'a, T> {
	value: &'a mut T,
	borrow: &'a Cell<BorrowState>,
}

impl<'a, T> RefMut<'a, T> {
	fn into_parts(r: Self) -> (&'a mut T, &'a Cell<BorrowState>) {
		unsafe {
			let value = std::ptr::read(&r.value);
			let borrow = std::ptr::read(&r.borrow);
			std::mem::forget(r);
			(value, borrow)
		}
	}

	pub fn downgrade(r: Self) -> Ref<'a, T> {
		let (value, borrow) = Self::into_parts(r);
		borrow.set(borrow.get().downgrade_writer());
		Ref { value, borrow }
	}
}

impl<'a, T> Deref for RefMut<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value
	}
}

impl<'a, T> DerefMut for RefMut<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.value
	}
}

impl<'a, T> Drop for RefMut<'a, T> {
	fn drop(&mut self) {
		self.borrow.set(self.borrow.get().remove_writer())
	}
}

impl<'a, T: PartialEq> PartialEq for RefMut<'a, T> {
	fn eq(&self, other: &Self) -> bool {
		self.value == other.value
	}
}

impl<'a, T: PartialEq> PartialEq<Ref<'a, T>> for RefMut<'a, T> {
	fn eq(&self, other: &Ref<'a, T>) -> bool {
		self.value == other.value
	}
}

impl<'a, T: Eq> Eq for RefMut<'a, T> {}

impl<'a, T: PartialOrd> PartialOrd for RefMut<'a, T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		(*self.value).partial_cmp(other.value)
	}
}

impl<'a, T: PartialOrd> PartialOrd<Ref<'a, T>> for RefMut<'a, T> {
	fn partial_cmp(&self, other: &Ref<'a, T>) -> Option<std::cmp::Ordering> {
		(*self.value).partial_cmp(other.value)
	}
}

impl<'a, T: Ord> Ord for RefMut<'a, T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(*self.value).cmp(other.value)
	}
}

impl<'a, T: Hash> Hash for RefMut<'a, T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.value.hash(state)
	}
}

impl<'a, T: fmt::Display> fmt::Display for RefMut<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.value.fmt(f)
	}
}

impl<'a, T: fmt::Debug> fmt::Debug for RefMut<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.value.fmt(f)
	}
}

pub struct Ref<'a, T> {
	value: &'a T,
	borrow: &'a Cell<BorrowState>,
}

impl<'a, T> Ref<'a, T> {
	fn into_parts(r: Self) -> (&'a T, &'a Cell<BorrowState>) {
		unsafe {
			let value = std::ptr::read(&r.value);
			let borrow = std::ptr::read(&r.borrow);
			std::mem::forget(r);
			(value, borrow)
		}
	}

	pub fn map<U>(r: Self, f: impl FnOnce(&'a T) -> &'a U) -> Ref<'a, U> {
		let (value, borrow) = Self::into_parts(r);

		Ref {
			value: f(value),
			borrow,
		}
	}

	pub fn aliasing_map<U>(r: Self, f: impl FnOnce(&'a T) -> U) -> Aliasing<'a, U> {
		let (value, borrow) = Self::into_parts(r);

		Aliasing {
			value: f(value),
			borrow,
		}
	}
}

impl<'a, T> Deref for Ref<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.value
	}
}

impl<'a, T> Clone for Ref<'a, T> {
	fn clone(&self) -> Self {
		self.borrow.set(self.borrow.get().clone_reader());

		Self {
			value: self.value,
			borrow: self.borrow,
		}
	}
}

impl<'a, T> Drop for Ref<'a, T> {
	fn drop(&mut self) {
		self.borrow.set(self.borrow.get().remove_reader())
	}
}

impl<'a, T: PartialEq> PartialEq for Ref<'a, T> {
	fn eq(&self, other: &Self) -> bool {
		self.value == other.value
	}
}

impl<'a, T: PartialEq> PartialEq<RefMut<'a, T>> for Ref<'a, T> {
	fn eq(&self, other: &RefMut<'a, T>) -> bool {
		self.value == other.value
	}
}

impl<'a, T: Eq> Eq for Ref<'a, T> {}

impl<'a, T: PartialOrd> PartialOrd for Ref<'a, T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.value.partial_cmp(other.value)
	}
}

impl<'a, T: PartialOrd> PartialOrd<RefMut<'a, T>> for Ref<'a, T> {
	fn partial_cmp(&self, other: &RefMut<'a, T>) -> Option<std::cmp::Ordering> {
		self.value.partial_cmp(other.value)
	}
}

impl<'a, T: Ord> Ord for Ref<'a, T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.value.cmp(other.value)
	}
}

impl<'a, T: Hash> Hash for Ref<'a, T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.value.hash(state)
	}
}

impl<'a, T: fmt::Display> fmt::Display for Ref<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.value.fmt(f)
	}
}

impl<'a, T: fmt::Debug> fmt::Debug for Ref<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.value.fmt(f)
	}
}

pub struct Aliasing<'a, T> {
	value: T,
	borrow: &'a Cell<BorrowState>,
}

impl<'a, T> Aliasing<'a, T> {
	fn into_parts(r: Self) -> (T, &'a Cell<BorrowState>) {
		unsafe {
			let value = std::ptr::read(&r.value);
			let borrow = std::ptr::read(&r.borrow);
			std::mem::forget(r);
			(value, borrow)
		}
	}

	pub fn map<U>(r: Self, f: impl FnOnce(T) -> U) -> Aliasing<'a, U> {
		let (value, borrow) = Self::into_parts(r);
		Aliasing {
			value: f(value),
			borrow,
		}
	}

	pub fn into_iter_escape(r: Self) -> IntoIterEscape<'a, T> {
		IntoIterEscape(r)
	}
}

impl<'a, T> Deref for Aliasing<'a, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<'a, T> DerefMut for Aliasing<'a, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

impl<'a, T: Clone> Clone for Aliasing<'a, T> {
	fn clone(&self) -> Self {
		self.borrow.set(self.borrow.get().clone_reader());
		Self {
			value: self.value.clone(),
			borrow: self.borrow,
		}
	}
}

impl<'a, T> Drop for Aliasing<'a, T> {
	fn drop(&mut self) {
		self.borrow.set(self.borrow.get().remove_reader())
	}
}

impl<'a, T: PartialEq> PartialEq for Aliasing<'a, T> {
	fn eq(&self, other: &Self) -> bool {
		self.value == other.value
	}
}

impl<'a, T: Eq> Eq for Aliasing<'a, T> {}

impl<'a, T: PartialOrd> PartialOrd for Aliasing<'a, T> {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.value.partial_cmp(&other.value)
	}
}

impl<'a, T: Ord> Ord for Aliasing<'a, T> {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.value.cmp(&other.value)
	}
}

impl<'a, T: Hash> Hash for Aliasing<'a, T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.value.hash(state)
	}
}

impl<'a, T: fmt::Display> fmt::Display for Aliasing<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.value.fmt(f)
	}
}

impl<'a, T: fmt::Debug> fmt::Debug for Aliasing<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.value.fmt(f)
	}
}

pub struct IntoIterEscape<'a, T>(Aliasing<'a, T>);

impl<'a, T: Iterator> Iterator for IntoIterEscape<'a, T>
where
	T::Item: 'static,
{
	type Item = T::Item;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next()
	}
}
