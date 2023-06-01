use std::io::{self, Write};

use inferdf_core::{Cause, Id, Sign, Signed};
use iref::{Iri, IriBuf};
use langtag::LanguageTag;
use locspan::Meta;
use rdf_types::{
	literal,
	vocabulary::{IriIndex, LanguageTagIndex, LiteralIndex},
	IriVocabulary, LanguageTagVocabulary, Literal, LiteralVocabulary,
};

use crate::{Header, Tag, Version, HEADER_TAG, VERSION, module::{IriPath, LiteralPath}};

pub trait Encode<V> {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error>;
}

impl<V> Encode<V> for Header {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.tag.encode(vocabulary, output)?;
		self.version.encode(vocabulary, output)?;
		self.page_size.encode(vocabulary, output)?;
		self.resource_count.encode(vocabulary, output)?;
		self.resource_page_count.encode(vocabulary, output)?;
		self.iri_count.encode(vocabulary, output)?;
		self.iri_page_count.encode(vocabulary, output)?;
		self.literal_count.encode(vocabulary, output)?;
		self.literal_page_count.encode(vocabulary, output)?;
		self.graph_count.encode(vocabulary, output)?;
		self.graph_page_count.encode(vocabulary, output)?;
		self.default_graph.encode(vocabulary, output)
	}
}

impl<V> Encode<V> for Tag {
	fn encode(&self, _vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		output.write_all(&HEADER_TAG)
	}
}

impl<V> Encode<V> for Version {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		VERSION.encode(vocabulary, output)
	}
}

impl<V> Encode<V> for u8 {
	fn encode(&self, _vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		output.write_all(std::slice::from_ref(self))
	}
}

impl<V> Encode<V> for u32 {
	fn encode(&self, _vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		let bytes = self.to_be_bytes();
		output.write_all(&bytes)
	}
}

impl<V> Encode<V> for Id {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		let i: u32 = (*self).into();
		i.encode(vocabulary, output)
	}
}

impl<V> Encode<V> for IriPath {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.page.encode(vocabulary, output)?;
		self.index.encode(vocabulary, output)
	}
}

impl<V> Encode<V> for LiteralPath {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.page.encode(vocabulary, output)?;
		self.index.encode(vocabulary, output)
	}
}

impl<'a, V> Encode<V> for &'a [u8] {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		(self.len() as u32).encode(vocabulary, output)?;
		output.write_all(self)
	}
}

impl<V, T: Encode<V>> Encode<V> for Vec<T> {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		(self.len() as u32).encode(vocabulary, output)?;

		for entry in self {
			entry.encode(vocabulary, output)?
		}

		Ok(())
	}
}

impl<V, T: Encode<V>, M: Encode<V>> Encode<V> for Meta<T, M> {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.0.encode(vocabulary, output)?;
		self.1.encode(vocabulary, output)
	}
}

impl<V, T: Encode<V>> Encode<V> for Signed<T> {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.0.encode(vocabulary, output)?;
		self.1.encode(vocabulary, output)
	}
}

impl<V> Encode<V> for Sign {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		match self {
			Self::Positive => 0u8.encode(vocabulary, output),
			Self::Negative => 1u8.encode(vocabulary, output),
		}
	}
}

impl<V> Encode<V> for Cause {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		match self {
			Self::Stated(i) => {
				0u8.encode(vocabulary, output)?;
				i.encode(vocabulary, output)
			}
			Self::Entailed(i) => {
				1u8.encode(vocabulary, output)?;
				i.encode(vocabulary, output)
			}
		}
	}
}

impl<'a, V> Encode<V> for &'a str {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.as_bytes().encode(vocabulary, output)
	}
}

impl<V> Encode<V> for String {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.as_str().encode(vocabulary, output)
	}
}

impl<'a, V> Encode<V> for Iri<'a> {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.as_bytes().encode(vocabulary, output)
	}
}

impl<V> Encode<V> for IriBuf {
	fn encode(
		&self,
		vocabulary: &V,
		output: &mut impl std::io::Write,
	) -> Result<(), std::io::Error> {
		self.as_iri().encode(vocabulary, output)
	}
}

impl<V, T: Encode<V>, S: Encode<V>> Encode<V> for Literal<T, S> {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.type_().encode(vocabulary, output)?;
		self.value().encode(vocabulary, output)
	}
}

impl<V, I: Encode<V>, L: Encode<V>> Encode<V> for literal::Type<I, L> {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		match self {
			Self::Any(ty) => {
				output.write_all(&[0u8])?;
				ty.encode(vocabulary, output)
			}
			Self::LangString(tag) => {
				output.write_all(&[1u8])?;
				tag.encode(vocabulary, output)
			}
		}
	}
}

impl<'a, V> Encode<V> for LanguageTag<'a> {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		self.as_bytes().encode(vocabulary, output)
	}
}

impl<V: IriVocabulary<Iri = Self>> Encode<V> for IriIndex {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		vocabulary.iri(self).unwrap().encode(vocabulary, output)
	}
}

impl<V: LiteralVocabulary<Literal = Self>> Encode<V> for LiteralIndex
where
	V::Type: Encode<V>,
	V::Value: Encode<V>,
{
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		vocabulary.literal(self).unwrap().encode(vocabulary, output)
	}
}

impl<V: LanguageTagVocabulary<LanguageTag = Self>> Encode<V> for LanguageTagIndex {
	fn encode(&self, vocabulary: &V, output: &mut impl Write) -> Result<(), io::Error> {
		vocabulary
			.language_tag(self)
			.unwrap()
			.encode(vocabulary, output)
	}
}
