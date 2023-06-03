use std::cmp::Ordering;

use inferdf_core::Id;
use rdf_types::{Literal, LiteralVocabulary};

use crate::{
	decode::{self, Decode, DecodeWith},
	encode::{Encode, EncodedLen},
	module::LiteralPath,
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

impl<L> Entry<L> {
	pub fn new(literal: L, interpretation: Id) -> Self {
		Self {
			literal,
			interpretation,
		}
	}
}

impl<L: Encode> Encode for LiteralsPage<L> {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		self.0.encode(output)
	}
}

impl<V, L: DecodeWith<V>> DecodeWith<V> for LiteralsPage<L> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, decode::Error> {
		Ok(Self(Vec::decode_with(vocabulary, input)?))
	}
}

impl<L: EncodedLen> EncodedLen for Entry<L> {
	fn encoded_len(&self) -> u32 {
		self.literal.encoded_len() + self.interpretation.encoded_len()
	}
}

impl<L: Encode> Encode for Entry<L> {
	fn encode(&self, output: &mut impl std::io::Write) -> Result<u32, std::io::Error> {
		Ok(self.literal.encode(output)? + self.interpretation.encode(output)?)
	}
}

impl<V, L: DecodeWith<V>> DecodeWith<V> for Entry<L> {
	fn decode_with(
		vocabulary: &mut V,
		input: &mut impl std::io::Read,
	) -> Result<Self, decode::Error> {
		Ok(Self {
			literal: L::decode_with(vocabulary, input)?,
			interpretation: Id::decode(input)?,
		})
	}
}

pub struct Pages<L, E, F> {
	page_len: u32,
	entries: E,
	page_index: u32,
	current_page: Option<(LiteralsPage<L>, u32)>,
	on_allocation: F,
}

impl<L, E, F> Pages<L, E, F> {
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

impl<T, L: EncodedLen, E: Iterator<Item = (T, Entry<L>)>, F: FnMut(T, LiteralPath)> Iterator
	for Pages<L, E, F>
{
	type Item = LiteralsPage<L>;

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
									LiteralPath::new(self.page_index, page.0.len() as u32),
								);
								page.0.push(entry);
							} else {
								let result = self.current_page.take().map(|(page, _)| page);
								self.page_index += 1;
								(self.on_allocation)(t, LiteralPath::new(self.page_index, 0));
								let page = LiteralsPage(vec![entry]);
								self.current_page = Some((page, 4 + entry_len));
								break result;
							}
						}
						None => {
							(self.on_allocation)(t, LiteralPath::new(self.page_index, 0));
							let page = LiteralsPage(vec![entry]);
							self.current_page = Some((page, 4 + entry_len))
						}
					}
				}
				None => break self.current_page.take().map(|(page, _)| page),
			}
		}
	}
}
