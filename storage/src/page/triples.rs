use inferdf_core::{Cause, Signed, Triple};
use locspan::Meta;

use crate::{
	module::{self, Decode, DecodeSized},
	writer::Encode,
};

const SIGN_LEN: u32 = 1;
const TRIPLE_LEN: u32 = 4 * 3;
const CAUSE_LEN: u32 = 1 + 4;
const FACT_LEN: u32 = SIGN_LEN + TRIPLE_LEN + CAUSE_LEN;

pub struct TriplesPage(Vec<Meta<Signed<Triple>, Cause>>);

impl TriplesPage {
	/// Returns the page index and the index in the page of the triple
	/// identified by the given graph triple index `i`.
	pub fn triple_page_index(page_len: u32, i: u32) -> (u32, u32) {
		let triple_per_page = page_len / FACT_LEN;
		let page = i / triple_per_page;
		let local_i = i % triple_per_page;
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

impl<V> DecodeSized<V> for TriplesPage {
	fn decode_sized(
		vocabulary: &mut V,
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
