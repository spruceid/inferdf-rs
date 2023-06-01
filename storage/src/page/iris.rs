use std::cmp::Ordering;

use inferdf_core::Id;
use iref::{Iri, IriBuf};
use rdf_types::IriVocabulary;

use crate::{
	module::{self, Decode, DecodeWith},
	writer::Encode,
};

pub struct IrisPage<I = IriBuf>(Vec<Entry<I>>);

impl<I> IrisPage<I> {
	pub fn get(&self, i: usize) -> Option<&Entry<I>> {
		self.0.get(i)
	}

	pub fn find(
		&self,
		vocabulary: &impl IriVocabulary<Iri = I>,
		iri: Iri,
	) -> Result<usize, Ordering> {
		if self.0.is_empty() {
			Err(Ordering::Equal)
		} else if vocabulary.iri(&self.0[0].iri).unwrap() > iri {
			Err(Ordering::Greater)
		} else if vocabulary.iri(&self.0[self.0.len() - 1].iri).unwrap() < iri {
			Err(Ordering::Less)
		} else {
			match self
				.0
				.binary_search_by_key(&iri, |e| vocabulary.iri(&e.iri).unwrap())
			{
				Ok(i) => Ok(i),
				Err(_) => Err(Ordering::Equal),
			}
		}
	}
}

pub struct Entry<I> {
	pub iri: I,
	pub interpretation: Id,
}

impl<V, I: Encode<V>> Encode<V> for IrisPage<I> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.0.encode(vocabulary, output)
	}
}

impl<V, I: DecodeWith<V>> DecodeWith<V> for IrisPage<I> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, module::decode::Error> {
		Ok(Self(Vec::decode_with(vocabulary, input)?))
	}
}

impl<V, I: Encode<V>> Encode<V> for Entry<I> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.iri.encode(vocabulary, output)?;
		self.interpretation.encode(vocabulary, output)
	}
}

impl<V, I: DecodeWith<V>> DecodeWith<V> for Entry<I> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, module::decode::Error> {
		Ok(Self {
			iri: I::decode_with(vocabulary, input)?,
			interpretation: Id::decode(input)?,
		})
	}
}
