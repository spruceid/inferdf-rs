use std::cmp::Ordering;

use inferdf_core::Id;
use iref::{Iri, IriBuf};
use rdf_types::IriVocabulary;

use crate::{
	decode::{self, Decode, DecodeWith},
	encode::{Encode, EncodedLen},
	module::IriPath,
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

pub struct Pages<I, E, F> {
	page_len: u32,
	entries: E,
	page_index: u32,
	current_page: Option<(IrisPage<I>, u32)>,
	on_allocation: F,
}

impl<I, E, F> Pages<I, E, F> {
	pub fn new(page_len: u32, entries: E, on_allocation: F) -> Self {
		Self {
			page_len,
			entries,
			page_index: 0,
			current_page: None,
			on_allocation,
		}
	}
}

impl<T, I: EncodedLen, E: Iterator<Item = (T, Entry<I>)>, F: FnMut(T, IriPath)> Iterator
	for Pages<I, E, F>
{
	type Item = IrisPage<I>;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.entries.next() {
				Some((t, entry)) => {
					let entry_len = entry.encoded_len();
					match self.current_page.as_mut() {
						Some((page, len)) => {
							if *len + entry_len <= self.page_len {
								*len += entry_len;
								(self.on_allocation)(
									t,
									IriPath::new(self.page_index, page.0.len() as u32),
								);
								page.0.push(entry);
							} else {
								let result = self.current_page.take().map(|(page, _)| page);
								self.page_index += 1;
								(self.on_allocation)(t, IriPath::new(self.page_index, 0));
								let page = IrisPage(vec![entry]);
								self.current_page = Some((page, 4 + entry_len));
								break result;
							}
						}
						None => {
							(self.on_allocation)(t, IriPath::new(self.page_index, 0));
							let page = IrisPage(vec![entry]);
							self.current_page = Some((page, 4 + entry_len))
						}
					}
				}
				None => break self.current_page.take().map(|(page, _)| page),
			}
		}
	}
}

pub struct Entry<I> {
	pub iri: I,
	pub interpretation: Id,
}

impl<I> Entry<I> {
	pub fn new(iri: I, interpretation: Id) -> Self {
		Self {
			iri,
			interpretation,
		}
	}
}

impl<I: EncodedLen> EncodedLen for Entry<I> {
	fn encoded_len(&self) -> u32 {
		self.iri.encoded_len() + self.interpretation.encoded_len()
	}
}

impl<I: Encode> Encode for IrisPage<I> {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<(), std::io::Error> {
		self.0.encode(output)
	}
}

impl<V, I: DecodeWith<V>> DecodeWith<V> for IrisPage<I> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, decode::Error> {
		Ok(Self(Vec::decode_with(vocabulary, input)?))
	}
}

impl<I: Encode> Encode for Entry<I> {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<(), std::io::Error> {
		self.iri.encode(output)?;
		self.interpretation.encode(output)
	}
}

impl<V, I: DecodeWith<V>> DecodeWith<V> for Entry<I> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, decode::Error> {
		Ok(Self {
			iri: I::decode_with(vocabulary, input)?,
			interpretation: Id::decode(input)?,
		})
	}
}
