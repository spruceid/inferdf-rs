use std::cmp::Ordering;

use inferdf_core::Id;

use crate::{
	decode::{self, Decode, DecodeSized},
	encode::{Encode, EncodedLen, StaticEncodedLen},
};

pub struct GraphsPage(Vec<Entry>);

impl GraphsPage {
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
		self.0.iter().copied()
	}
}

pub type Iter<'a> = std::iter::Copied<std::slice::Iter<'a, Entry>>;

#[derive(Default, Debug, Clone, Copy)]
pub struct Description {
	pub triple_count: u32,
	pub triple_page_count: u32,
	pub resource_count: u32,
	pub resource_page_count: u32,
	pub first_page: u32,
}

impl Description {
	pub fn page_count(&self) -> u32 {
		self.triple_page_count + self.resource_page_count
	}
}

impl StaticEncodedLen for Description {
	const ENCODED_LEN: u32 = 4 * 5;
}

#[derive(Debug, Clone, Copy)]
pub struct Entry {
	pub id: Id,
	pub description: Description,
}

impl StaticEncodedLen for Entry {
	const ENCODED_LEN: u32 = Id::ENCODED_LEN + Description::ENCODED_LEN;
}

impl Encode for GraphsPage {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		self.0.encode(output)
	}
}

impl DecodeSized for GraphsPage {
	fn decode_sized(input: &mut impl std::io::Read, len: u32) -> Result<Self, decode::Error> {
		let mut graphs = Vec::new();

		for _i in 0..len {
			graphs.push(Entry::decode(input)?)
		}

		Ok(Self(graphs))
	}
}

impl Encode for Description {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		self.triple_count.encode(output)?;
		self.triple_page_count.encode(output)?;
		self.resource_count.encode(output)?;
		self.resource_page_count.encode(output)?;
		self.first_page.encode(output)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Decode for Description {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, decode::Error> {
		Ok(Self {
			triple_count: u32::decode(input)?,
			triple_page_count: u32::decode(input)?,
			resource_count: u32::decode(input)?,
			resource_page_count: u32::decode(input)?,
			first_page: u32::decode(input)?,
		})
	}
}

impl Encode for Entry {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		self.id.encode(output)?;
		self.description.encode(output)?;
		Ok(Self::ENCODED_LEN)
	}
}

impl Decode for Entry {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, decode::Error> {
		Ok(Self {
			id: Id::decode(input)?,
			description: Description::decode(input)?,
		})
	}
}

pub struct Pages<E> {
	page_len: u32,
	entries: E,
	page_index: u32,
	current_page: Option<(GraphsPage, u32)>,
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
	type Item = GraphsPage;

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
								let page = GraphsPage(vec![entry]);
								self.current_page = Some((page, entry_len));
								break result;
							}
						}
						None => {
							let page = GraphsPage(vec![entry]);
							self.current_page = Some((page, entry_len))
						}
					}
				}
				None => break self.current_page.take().map(|(page, _)| page),
			}
		}
	}
}
