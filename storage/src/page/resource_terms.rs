use std::cmp::Ordering;

use inferdf_core::Id;

use crate::{
	decode::{self, Decode},
	encode::{Encode, EncodedLen},
	module::{IriPath, LiteralPath},
};

/// A Resource terms page.
///
/// Resources pages list resources and their known terms.
pub struct ResourcesTermsPage(Vec<Entry>);

impl ResourcesTermsPage {
	pub fn get(&self, i: usize) -> Option<&Entry> {
		self.0.get(i)
	}

	pub fn find(&self, id: Id) -> Result<usize, Ordering> {
		if self.0.is_empty() {
			Err(Ordering::Equal)
		} else if self.0[0].id > id {
			Err(Ordering::Greater)
		} else if self.0[self.0.len() - 1].id < id {
			Err(Ordering::Less)
		} else {
			match self.0.binary_search_by_key(&id, |e| e.id) {
				Ok(i) => Ok(i),
				Err(_) => Err(Ordering::Equal),
			}
		}
	}
}

pub struct Entry {
	pub id: Id,
	pub known_iris: Vec<IriPath>,
	pub known_literals: Vec<LiteralPath>,
	pub different_from: Vec<Id>,
}

impl Entry {
	pub fn new(
		id: Id,
		known_iris: Vec<IriPath>,
		known_literals: Vec<LiteralPath>,
		different_from: Vec<Id>,
	) -> Self {
		Self {
			id,
			known_iris,
			known_literals,
			different_from,
		}
	}

	pub fn iter_known_iris(&self) -> IriPaths {
		self.known_iris.iter().copied()
	}

	pub fn iter_known_literals(&self) -> LiteralPaths {
		self.known_literals.iter().copied()
	}

	pub fn iter_different_from(&self) -> DifferentFrom {
		self.different_from.iter().copied()
	}
}

pub type IriPaths<'a> = std::iter::Copied<std::slice::Iter<'a, IriPath>>;

pub type LiteralPaths<'a> = std::iter::Copied<std::slice::Iter<'a, LiteralPath>>;

pub type DifferentFrom<'a> = std::iter::Copied<std::slice::Iter<'a, Id>>;

impl Encode for ResourcesTermsPage {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<(), std::io::Error> {
		self.0.encode(output)
	}
}

impl Decode for ResourcesTermsPage {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, decode::Error> {
		Ok(Self(Vec::decode(input)?))
	}
}

impl Encode for Entry {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<(), std::io::Error> {
		self.id.encode(output)?;
		self.known_iris.encode(output)?;
		self.known_literals.encode(output)?;
		self.different_from.encode(output)
	}
}

impl EncodedLen for Entry {
	fn encoded_len(&self) -> u32 {
		self.id.encoded_len()
			+ self.known_iris.encoded_len()
			+ self.known_literals.encoded_len()
			+ self.different_from.encoded_len()
	}
}

impl Decode for Entry {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, decode::Error> {
		Ok(Self {
			id: Id::decode(input)?,
			known_iris: Vec::decode(input)?,
			known_literals: Vec::decode(input)?,
			different_from: Vec::decode(input)?,
		})
	}
}

pub struct Pages<E> {
	page_len: u32,
	entries: E,
	page_index: u32,
	current_page: Option<(ResourcesTermsPage, u32)>,
}

impl<E> Pages<E> {
	pub fn new(page_len: u32, entries: E) -> Self {
		Self {
			page_len,
			entries,
			page_index: 0,
			current_page: None,
		}
	}
}

impl<E: Iterator<Item = Entry>> Iterator for Pages<E> {
	type Item = ResourcesTermsPage;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.entries.next() {
				Some(entry) => {
					let entry_len = entry.encoded_len();
					match self.current_page.as_mut() {
						Some((page, len)) => {
							if *len + entry_len <= self.page_len {
								*len += entry_len;
								page.0.push(entry);
							} else {
								let result = self.current_page.take().map(|(page, _)| page);
								self.page_index += 1;
								let page = ResourcesTermsPage(vec![entry]);
								self.current_page = Some((page, 4 + entry_len));
								break result;
							}
						}
						None => {
							let page = ResourcesTermsPage(vec![entry]);
							self.current_page = Some((page, 4 + entry_len))
						}
					}
				}
				None => break self.current_page.take().map(|(page, _)| page),
			}
		}
	}
}
