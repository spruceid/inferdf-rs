use inferdf_core::{Cause, Signed, Triple};
use locspan::Meta;

use crate::{
	module::{self, Decode, DecodeSized},
	writer::Encode,
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

impl<V> Encode<V> for TriplesPage {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl DecodeSized for TriplesPage {
	fn decode_sized(
		input: &mut impl std::io::Read,
		len: u32,
	) -> Result<Self, module::decode::Error> {
		let mut triples = Vec::with_capacity(len as usize);

		for _i in 0..len {
			triples.push(Meta::decode(input)?)
		}

		Ok(Self(triples))
	}
}

impl<V> Encode<V> for Triple {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)?;
		self.1.encode(vocabulary, output)?;
		self.2.encode(vocabulary, output)
	}
}

impl Decode for Triple {
	fn decode(input: &mut impl std::io::Read) -> Result<Self, module::decode::Error> {
		let s = u32::decode(input)?;
		let p = u32::decode(input)?;
		let o = u32::decode(input)?;
		Ok(Self::new(s.into(), p.into(), o.into()))
	}
}
