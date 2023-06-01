mod module;
mod page;
mod writer;

use std::cmp::Ordering;

pub use module::Module;

/// Header tag value.
pub const HEADER_TAG: [u8; 4] = [b'B', b'R', b'D', b'F'];

/// Implemented version.
pub const VERSION: u32 = 0;

pub struct Header {
	pub tag: Tag,
	pub version: Version,
	pub page_size: u32,
	pub resource_count: u32,
	pub resource_page_count: u32,
	pub iri_count: u32,
	pub iri_page_count: u32,
	pub literal_count: u32,
	pub literal_page_count: u32,
	pub graph_count: u32,
	pub graph_page_count: u32,
	pub default_graph: page::graphs::Description,
}

impl Header {
	pub const LEN: usize = Tag::LEN + Version::LEN + 4 * 9 + page::graphs::Description::LEN;
}

/// Header tag, used to recognize the file format.
pub struct Tag;

impl Tag {
	pub const LEN: usize = 4;
}

/// Version number.
pub struct Version;

impl Version {
	pub const LEN: usize = 4;
}

pub struct Sections {
	pub first_page_offset: u64,
	pub resources: u32,
	pub iris: u32,
	pub literals: u32,
	pub graphs: u32,
	pub default_graph: u32,
}

impl Sections {
	pub fn new(header: &Header) -> Self {
		let mut first_page_offset = Header::LEN as u64;
		let m = first_page_offset % header.page_size as u64;
		if m > 0 {
			first_page_offset += header.page_size as u64 - m;
		}

		Self {
			first_page_offset,
			resources: 0,
			iris: header.resource_page_count,
			literals: header.resource_page_count + header.iri_page_count,
			graphs: header.resource_page_count + header.iri_page_count + header.literal_page_count,
			default_graph: header.resource_page_count
				+ header.iri_page_count
				+ header.literal_page_count
				+ header.graph_page_count,
		}
	}
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
