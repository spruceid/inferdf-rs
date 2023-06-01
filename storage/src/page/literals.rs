use std::cmp::Ordering;

use inferdf_core::Id;
use rdf_types::{Literal, LiteralVocabulary};

use crate::{
	module::{self, Decode, DecodeWith},
	writer::Encode,
};

pub struct LiteralsPage<L = Literal>(Vec<Entry<L>>);

impl<L> LiteralsPage<L> {
	pub fn get(&self, i: usize) -> Option<&Entry<L>> {
		self.0.get(i)
	}

	pub fn find<V: LiteralVocabulary<Literal = L>>(
		&self,
		vocabulary: &V,
		literal: &Literal<V::Type, V::Value>,
	) -> Result<usize, Ordering>
	where
		V::Type: Ord,
		V::Value: Ord,
	{
		if self.0.is_empty() {
			Err(Ordering::Equal)
		} else if vocabulary.literal(&self.0[0].literal).unwrap() > literal {
			Err(Ordering::Greater)
		} else if vocabulary
			.literal(&self.0[self.0.len() - 1].literal)
			.unwrap() < literal
		{
			Err(Ordering::Less)
		} else {
			match self
				.0
				.binary_search_by_key(&literal, |e| vocabulary.literal(&e.literal).unwrap())
			{
				Ok(i) => Ok(i),
				Err(_) => Err(Ordering::Equal),
			}
		}
	}
}

pub struct Entry<L> {
	pub literal: L,
	pub interpretation: Id,
}

impl<V, L: Encode<V>> Encode<V> for LiteralsPage<L> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V, L: DecodeWith<V>> DecodeWith<V> for LiteralsPage<L> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, module::decode::Error> {
		Ok(Self(Vec::decode_with(vocabulary, input)?))
	}
}

impl<V, L: Encode<V>> Encode<V> for Entry<L> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.literal.encode(vocabulary, output)?;
		self.interpretation.encode(vocabulary, output)
	}
}

impl<V, L: DecodeWith<V>> DecodeWith<V> for Entry<L> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, module::decode::Error> {
		Ok(Self {
			literal: L::decode_with(vocabulary, input)?,
			interpretation: Id::decode(input)?,
		})
	}
}
