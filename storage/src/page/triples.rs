use inferdf_core::{Cause, Signed, Triple};
use locspan::Meta;

use crate::{
	decode::{self, Decode, DecodeSized},
	encode::{Encode, EncodedLen},
};

const SIGN_LEN: u32 = 1;
const TRIPLE_LEN: u32 = 4 * 3;
const CAUSE_LEN: u32 = 1 + 4;
pub const FACT_LEN: u32 = SIGN_LEN + TRIPLE_LEN + CAUSE_LEN;

pub struct TriplesPage(Vec<Meta<Signed<Triple>, Cause>>);

pub fn page_triple_count(triple_count: u32, page_index: u32, triples_per_page: u32) -> u32 {
	std::cmp::min(
		triple_count - page_index * triples_per_page,
		triples_per_page,
	)
}

impl TriplesPage {
	/// Returns the page index and the index in the page of the triple
	/// identified by the given graph triple index `i`.
	pub fn triple_page_index(triples_per_page: u32, i: u32) -> (u32, u32) {
		let page = i / triples_per_page;
		let local_i = i % triples_per_page;
		(page, local_i)
	}

	pub fn get(&self, i: u32) -> Option<Meta<Signed<Triple>, Cause>> {
		self.0.get(i as usize).copied()
	}

	pub fn iter(&self) -> Iter {
		self.0.iter().copied()
	}
}

pub type Iter<'a> = std::iter::Copied<std::slice::Iter<'a, Meta<Signed<Triple>, Cause>>>;

impl Encode for TriplesPage {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		let mut len = 0;

		for t in &self.0 {
			len += t.encode(output)?
		}

		Ok(len)
	}
}

impl DecodeSized for TriplesPage {
	fn decode_sized(input: &mut impl std::io::Read, len: u32) -> Result<Self, decode::Error> {
		let mut triples = Vec::with_capacity(len as usize);

		for _i in 0..len {
			triples.push(Meta::decode(input)?)
		}

		Ok(Self(triples))
	}
}

impl Encode for Triple {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		Ok(self.0.encode(output)? + self.1.encode(output)? + self.2.encode(output)?)
	}
}

impl Decode for Triple {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, decode::Error> {
		let s = u32::decode(input)?;
		let p = u32::decode(input)?;
		let o = u32::decode(input)?;
		Ok(Self::new(s.into(), p.into(), o.into()))
	}
}

pub struct Pages<E> {
	page_len: u32,
	entries: E,
	page_index: u32,
	current_page: Option<(TriplesPage, u32)>,
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

impl<E: Iterator<Item = Meta<Signed<Triple>, Cause>>> Iterator for Pages<E> {
	type Item = TriplesPage;

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
								let page = TriplesPage(vec![entry]);
								self.current_page = Some((page, entry_len));
								break result;
							}
						}
						None => {
							let page = TriplesPage(vec![entry]);
							self.current_page = Some((page, entry_len))
						}
					}
				}
				None => break self.current_page.take().map(|(page, _)| page),
			}
		}
	}
}
