use educe::Educe;

#[derive(Educe, Clone)]
#[educe(Default)]
pub struct ReservableSlab<T> {
	items: Vec<Item<T>>,
	head: usize,
	len: usize,
}

#[derive(Clone)]
enum Item<T> {
	Vacant(usize),
	Occupied(T),
}

impl<T> Item<T> {
	pub fn as_occupied(&self) -> Option<&T> {
		match self {
			Self::Vacant(_) => None,
			Self::Occupied(t) => Some(t),
		}
	}
}

impl<T> ReservableSlab<T> {
	pub fn len(&self) -> usize {
		self.len
	}

	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	pub fn get(&self, i: usize) -> Option<&T> {
		if i < self.items.len() {
			match &self.items[i] {
				Item::Vacant(_) => None,
				Item::Occupied(t) => Some(t),
			}
		} else {
			None
		}
	}

	pub fn get_mut(&mut self, i: usize) -> Option<&mut T> {
		if i < self.items.len() {
			match &mut self.items[i] {
				Item::Vacant(_) => None,
				Item::Occupied(t) => Some(t),
			}
		} else {
			None
		}
	}

	pub fn insert(&mut self, t: T) -> usize {
		let i = self.head;
		if i < self.items.len() {
			self.head = match std::mem::replace(&mut self.items[i], Item::Occupied(t)) {
				Item::Vacant(new_head) => new_head,
				Item::Occupied(_) => unreachable!(),
			};
		} else {
			self.items.push(Item::Occupied(t));
			self.head = self.items.len()
		}

		self.len += 1;
		i
	}

	pub fn remove(&mut self, i: usize) -> Option<T> {
		if i < self.items.len() {
			match std::mem::replace(&mut self.items[i], Item::Vacant(self.head)) {
				Item::Vacant(head) => {
					self.items[i] = Item::Vacant(head);
					None
				}
				Item::Occupied(t) => {
					self.head = i;
					self.len -= 1;
					Some(t)
				}
			}
		} else {
			None
		}
	}

	pub fn begin_reservation(&self) -> Reservation<T> {
		Reservation {
			slab: self,
			head: self.head,
			new_items: Vec::new(),
		}
	}

	pub fn iter(&self) -> Iter<T> {
		Iter(self.items.iter().enumerate())
	}
}

impl<T> std::ops::Index<usize> for ReservableSlab<T> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		self.get(index).unwrap()
	}
}

impl<T> std::ops::IndexMut<usize> for ReservableSlab<T> {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		self.get_mut(index).unwrap()
	}
}

pub struct Reservation<'a, T> {
	slab: &'a ReservableSlab<T>,
	head: usize,
	new_items: Vec<(T, usize)>,
}

impl<'a, T> Reservation<'a, T> {
	pub fn insert(&mut self, t: T) -> usize {
		let i = self.head;
		if self.head < self.slab.len() {
			self.head = match &self.slab.items[self.head] {
				Item::Vacant(new_head) => *new_head,
				Item::Occupied(_) => unreachable!(),
			};
		} else {
			self.head += 1;
		}

		self.new_items.push((t, i));
		i
	}

	pub fn end(self) -> CompletedReservation<T> {
		CompletedReservation {
			new_items: self.new_items,
		}
	}
}

#[derive(Debug, thiserror::Error)]
#[error("invalid reservation")]
pub struct InvalidReservation;

pub struct CompletedReservation<T> {
	new_items: Vec<(T, usize)>,
}

impl<T> CompletedReservation<T> {
	pub fn apply(self, slab: &mut ReservableSlab<T>) -> Result<(), InvalidReservation> {
		for (t, i) in self.new_items {
			if slab.insert(t) != i {
				return Err(InvalidReservation);
			}
		}

		Ok(())
	}
}

pub struct Iter<'a, T>(std::iter::Enumerate<std::slice::Iter<'a, Item<T>>>);

impl<'a, T> Iterator for Iter<'a, T> {
	type Item = (usize, &'a T);

	fn next(&mut self) -> Option<Self::Item> {
		self.0
			.find_map(|(i, item)| item.as_occupied().map(|t| (i, t)))
	}
}
