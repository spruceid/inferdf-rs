use std::cmp::Ordering;

use inferdf_core::Id;

use crate::{
	decode::{self, Decode},
	encode::{Encode, EncodedLen},
};

/// A Resource triples page.
///
/// Resources pages list resources of a graph and the triples they occur in.
pub struct ResourcesTriplesPage(Vec<Entry>);

impl ResourcesTriplesPage {
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

	pub fn iter(&self) -> Iter {
		self.0.iter()
	}
}

pub type Iter<'a> = std::slice::Iter<'a, Entry>;

pub struct Entry {
	pub id: Id,
	pub as_subject: Vec<u32>,
	pub as_predicate: Vec<u32>,
	pub as_object: Vec<u32>,
}

impl Entry {
	pub fn new(id: Id, as_subject: Vec<u32>, as_predicate: Vec<u32>, as_object: Vec<u32>) -> Self {
		Self {
			id,
			as_subject,
			as_predicate,
			as_object,
		}
	}
}

impl Encode for ResourcesTriplesPage {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		self.0.encode(output)
	}
}

impl Decode for ResourcesTriplesPage {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, decode::Error> {
		Ok(Self(Vec::decode(input)?))
	}
}

impl Encode for Entry {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		Ok(self.id.encode(output)?
			+ self.as_subject.encode(output)?
			+ self.as_predicate.encode(output)?
			+ self.as_object.encode(output)?)
	}
}

impl EncodedLen for Entry {
	fn encoded_len(&self) -> u32 {
		self.id.encoded_len()
			+ self.as_subject.encoded_len()
			+ self.as_predicate.encoded_len()
			+ self.as_object.encoded_len()
	}
}

impl Decode for Entry {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, decode::Error> {
		Ok(Self {
			id: Id::decode(input)?,
			as_subject: Vec::decode(input)?,
			as_predicate: Vec::decode(input)?,
			as_object: Vec::decode(input)?,
		})
	}
}

pub struct Pages<E> {
	page_len: u32,
	entries: E,
	page_index: u32,
	current_page: Option<(ResourcesTriplesPage, u32)>,
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
	type Item = ResourcesTriplesPage;

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
								let page = ResourcesTriplesPage(vec![entry]);
								self.current_page = Some((page, 4 + entry_len));
								break result;
							}
						}
						None => {
							let page = ResourcesTriplesPage(vec![entry]);
							self.current_page = Some((page, 4 + entry_len))
						}
					}
				}
				None => break self.current_page.take().map(|(page, _)| page),
			}
		}
	}
}
