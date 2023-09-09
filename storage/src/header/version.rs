use paged::{Decode, DecodeFromHeap, Encode, EncodeOnHeap, EncodeSized};
use std::io;

/// Implemented version.
pub const VERSION: u32 = 0;

/// Version number.
pub struct Version;

impl EncodeSized for Version {
	const ENCODED_SIZE: u32 = u32::ENCODED_SIZE;
}

impl<C> Encode<C> for Version {
	fn encode(&self, context: &C, output: &mut impl io::Write) -> io::Result<u32> {
		VERSION.encode(context, output)
	}
}

impl<C> EncodeOnHeap<C> for Version {
	fn encode_on_heap(
		&self,
		context: &C,
		_heap: &mut paged::Heap,
		output: &mut impl io::Write,
	) -> io::Result<u32> {
		self.encode(context, output)
	}
}

impl<C> Decode<C> for Version {
	fn decode<R: io::Read>(input: &mut R, context: &mut C) -> io::Result<Self> {
		let value = u32::decode(input, context)?;

		if value == VERSION {
			Ok(Self)
		} else {
			Err(io::ErrorKind::InvalidData.into())
		}
	}
}

impl<C> DecodeFromHeap<C> for Version {
	fn decode_from_heap<R: io::Seek + io::Read>(
		input: &mut paged::reader::Cursor<R>,
		context: &mut C,
		_heap: paged::HeapSection,
	) -> io::Result<Self> {
		Self::decode(input, context)
	}
}
