mod page;
mod reader;
mod writer;

pub use reader::Reader;

/// Byte size of the header.
pub const HEADER_LEN: usize = 4 * 13;

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
	pub default_graph_triple_count: u32,
	pub default_graph_triple_page_count: u32,
}

/// Header tag, used to recognize the file format.
pub struct Tag;

/// Version number.
pub struct Version;

pub struct Sections {
	pub resources: u32,
	pub iris: u32,
	pub literals: u32,
	pub graphs: u32,
	pub default_graph: u32,
}

impl Sections {
	pub fn new(header: &Header) -> Self {
		Self {
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
