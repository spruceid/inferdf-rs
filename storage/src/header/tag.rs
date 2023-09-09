use paged::{Decode, DecodeFromHeap, Encode, EncodeOnHeap, EncodeSized};
use std::io;

/// Header tag value.
pub const TAG: [u8; 4] = [b'B', b'R', b'D', b'F'];

/// Header tag, used to recognize the file format.
pub struct Tag;

impl EncodeSized for Tag {
	const ENCODED_SIZE: u32 = 4;
}

impl<C> Encode<C> for Tag {
	fn encode(&self, context: &C, output: &mut impl io::Write) -> io::Result<u32> {
		TAG.encode(context, output)
	}
}

impl<C> EncodeOnHeap<C> for Tag {
	fn encode_on_heap(
		&self,
		context: &C,
		_heap: &mut paged::Heap,
		output: &mut impl io::Write,
	) -> io::Result<u32> {
		self.encode(context, output)
	}
}

impl<C> Decode<C> for Tag {
	fn decode<R: io::Read>(input: &mut R, _context: &mut C) -> io::Result<Self> {
		let mut buffer = [0; 4];
		input.read(&mut buffer)?;

		if buffer == TAG {
			Ok(Self)
		} else {
			Err(io::ErrorKind::InvalidData.into())
		}
	}
}

impl<C> DecodeFromHeap<C> for Tag {
	fn decode_from_heap<R: io::Seek + io::Read>(
		input: &mut paged::reader::Cursor<R>,
		context: &mut C,
		_heap: paged::HeapSection,
	) -> io::Result<Self> {
		Self::decode(input, context)
	}
}
