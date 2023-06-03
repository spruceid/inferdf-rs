mod build;
mod decode;
mod encode;
mod module;
mod page;

pub use decode::{Decode, DecodeSized, DecodeWith};
pub use encode::Encode;
use encode::StaticEncodedLen;

use std::cmp::Ordering;

pub use module::Module;

pub use build::{build, Options as BuildOptions, DEFAULT_PAGE_SIZE};

/// Header tag value.
pub const HEADER_TAG: [u8; 4] = [b'B', b'R', b'D', b'F'];

/// Implemented version.
pub const VERSION: u32 = 0;

pub struct Header {
	pub tag: Tag,
	pub version: Version,
	pub page_size: u32,
	pub iri_count: u32,
	pub iri_page_count: u32,
	pub literal_count: u32,
	pub literal_page_count: u32,
	pub resource_count: u32,
	pub resource_page_count: u32,
	pub named_graph_count: u32,
	pub named_graph_page_count: u32,
	pub default_graph: page::graphs::Description,
}

impl StaticEncodedLen for Header {
	const ENCODED_LEN: u32 =
		Tag::ENCODED_LEN + Version::ENCODED_LEN + 4 * 9 + page::graphs::Description::ENCODED_LEN;
}

/// Header tag, used to recognize the file format.
pub struct Tag;

impl StaticEncodedLen for Tag {
	const ENCODED_LEN: u32 = 4;
}

/// Version number.
pub struct Version;

impl StaticEncodedLen for Version {
	const ENCODED_LEN: u32 = 4;
}

pub struct Sections {
	pub first_page_offset: u64,
	pub graphs_per_page: u32,
	pub triples_per_page: u32,
	pub iris: u32,
	pub literals: u32,
	pub resources: u32,
	pub graphs: u32,
	pub default_graph: u32,
}

impl Sections {
	pub fn new(header: &Header) -> Self {
		Self {
			first_page_offset: first_page_offset(header.page_size),
			graphs_per_page: graphs_per_page(header.page_size),
			triples_per_page: triples_per_page(header.page_size),
			iris: 0,
			literals: header.iri_page_count,
			resources: header.iri_page_count + header.literal_page_count,
			graphs: header.iri_page_count + header.literal_page_count + header.resource_page_count,
			default_graph: header.iri_page_count
				+ header.literal_page_count
				+ header.resource_page_count
				+ header.named_graph_page_count,
		}
	}
}

fn graphs_per_page(page_size: u32) -> u32 {
	page_size / page::graphs::Entry::ENCODED_LEN
}

fn triples_per_page(page_size: u32) -> u32 {
	page_size / page::triples::FACT_LEN
}

fn first_page_offset(page_size: u32) -> u64 {
	let mut first_page_offset = Header::ENCODED_LEN as u64;
	let m = first_page_offset % page_size as u64;
	if m > 0 {
		first_page_offset += page_size as u64 - m;
	}

	first_page_offset
}

fn binary_search_page<T, E>(
	first_page: u32,
	next_page: u32,
	mut get_page: impl FnMut(u32) -> Result<Result<T, Ordering>, E>,
) -> Result<Option<T>, E> {
	let mut a = first_page;
	let mut b = next_page;

	while a < b {
		let pivot = (a + b) / 2;
		let (new_a, new_b) = match get_page(pivot)? {
			Ok(t) => return Ok(Some(t)),
			Err(Ordering::Equal) => return Ok(None),
			Err(Ordering::Greater) => (a, pivot),
			Err(Ordering::Less) => (pivot + 1, b),
		};

		a = new_a;
		b = new_b
	}

	Ok(None)
}
