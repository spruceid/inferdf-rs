pub mod cache;
pub mod dataset;
pub mod interpretation;

use std::{
	cell::RefCell,
	io::{BufReader, Read, Seek},
};

pub use cache::CacheMap;
pub use dataset::Dataset;
pub use interpretation::Interpretation;
use rdf_types::Vocabulary;

use crate::{
	decode::{self, Decode},
	page, DecodeSized, DecodeWith, Header, Sections,
};

pub use page::Page;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("not enough memory")]
	NotEnoughMemory,

	#[error(transparent)]
	Decode(decode::Error),
}

impl<T> From<cache::Error<T, decode::Error>> for Error {
	fn from(value: cache::Error<T, decode::Error>) -> Self {
		match value {
			cache::Error::IO(e) => Self::Decode(e),
			cache::Error::NotEnoughMemory(_) => Self::NotEnoughMemory,
			cache::Error::Busy => panic!("cache page is busy"),
		}
	}
}

pub struct Module<V: Vocabulary, R> {
	inner: RefCell<BufReader<R>>,
	header: Header,
	sections: Sections,
	cache: CacheMap<u32, Page<V>>,
}

impl<V: Vocabulary, R: Read + Seek> Module<V, R> {
	pub fn new(reader: BufReader<R>) -> Result<Self, decode::Error> {
		Self::with_cache_capacity(reader, usize::MAX)
	}

	pub fn with_cache_capacity(
		mut reader: BufReader<R>,
		capacity: usize,
	) -> Result<Self, decode::Error> {
		let header = Header::decode(&mut reader)?;
		let sections = Sections::new(&header);

		Ok(Self {
			inner: RefCell::new(reader),
			header,
			sections,
			cache: CacheMap::with_capacity(capacity),
		})
	}

	fn read_page<P: Decode>(&self, i: u32) -> Result<P, decode::Error> {
		let mut inner = self.inner.borrow_mut();
		inner.seek(std::io::SeekFrom::Start(
			self.sections.first_page_offset + i as u64 * self.header.page_size as u64,
		))?;
		P::decode(&mut *inner)
	}

	fn read_sized_page<P: DecodeSized>(&self, i: u32, len: u32) -> Result<P, decode::Error> {
		let mut inner = self.inner.borrow_mut();
		inner.seek(std::io::SeekFrom::Start(
			self.sections.first_page_offset + i as u64 * self.header.page_size as u64,
		))?;
		P::decode_sized(&mut *inner, len)
	}

	fn read_page_with<P: DecodeWith<V>>(
		&self,
		vocabulary: &mut V,
		i: u32,
	) -> Result<P, decode::Error> {
		let mut inner = self.inner.borrow_mut();
		inner.seek(std::io::SeekFrom::Start(
			self.sections.first_page_offset + i as u64 * self.header.page_size as u64,
		))?;
		P::decode_with(vocabulary, &mut *inner)
	}

	fn get_resources_page(&self, i: u32) -> Result<cache::Ref<page::ResourcesTermsPage>, Error> {
		Ok(cache::Ref::map(
			self.cache
				.get(i, || Ok(Page::ResourcesTerms(self.read_page(i)?)))?,
			|p| p.as_resources_terms_page().unwrap(),
		))
	}

	fn get_graph_page(&self, i: u32, len: u32) -> Result<cache::Ref<page::GraphsPage>, Error> {
		Ok(cache::Ref::map(
			self.cache
				.get(i, || Ok(Page::Graphs(self.read_sized_page(i, len)?)))?,
			|p| p.as_graphs_page().unwrap(),
		))
	}

	fn get_triples_page(&self, i: u32, len: u32) -> Result<cache::Ref<page::TriplesPage>, Error> {
		Ok(cache::Ref::map(
			self.cache
				.get(i, || Ok(Page::Triples(self.read_sized_page(i, len)?)))?,
			|p| p.as_triples_page().unwrap(),
		))
	}

	fn get_resource_triples_page(
		&self,
		i: u32,
	) -> Result<cache::Ref<page::ResourcesTriplesPage>, Error> {
		Ok(cache::Ref::map(
			self.cache
				.get(i, || Ok(Page::ResourcesTriples(self.read_page(i)?)))?,
			|p| p.as_resources_triples_page().unwrap(),
		))
	}
}

impl<V: Vocabulary, R: Read + Seek> Module<V, R>
where
	V::Iri: DecodeWith<V>,
	V::Literal: DecodeWith<V>,
{
	fn get_iris_page(
		&self,
		vocabulary: &mut V,
		i: u32,
	) -> Result<cache::Ref<page::IrisPage<V::Iri>>, Error> {
		Ok(cache::Ref::map(
			self.cache
				.get(i, || Ok(Page::Iris(self.read_page_with(vocabulary, i)?)))?,
			|p| p.as_iris_page().unwrap(),
		))
	}

	fn get_literals_page(
		&self,
		vocabulary: &mut V,
		i: u32,
	) -> Result<cache::Ref<page::LiteralsPage<V::Literal>>, Error> {
		Ok(cache::Ref::map(
			self.cache.get(i, || {
				Ok(Page::Literals(self.read_page_with(vocabulary, i)?))
			})?,
			|p| p.as_literals_page().unwrap(),
		))
	}
}

impl<V: Vocabulary, R: Read + Seek> Module<V, R>
where
	V::Iri: Copy + DecodeWith<V>,
	V::Literal: Copy + DecodeWith<V>,
{
	/// Get the IRI identifier by the given path.
	pub fn get_iri(&self, vocabulary: &mut V, path: IriPath) -> Result<Option<V::Iri>, Error> {
		Ok(self
			.get_iris_page(vocabulary, path.page)?
			.get(path.index as usize)
			.map(|e| e.iri))
	}

	/// Get the literal identifier by the given path.
	pub fn get_literal(
		&self,
		vocabulary: &mut V,
		path: LiteralPath,
	) -> Result<Option<V::Literal>, Error> {
		Ok(self
			.get_literals_page(vocabulary, path.page)?
			.get(path.index as usize)
			.map(|e| e.literal))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IriPath {
	pub page: u32,
	pub index: u32,
}

impl IriPath {
	pub fn new(page: u32, index: u32) -> Self {
		Self { page, index }
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiteralPath {
	pub page: u32,
	pub index: u32,
}

impl LiteralPath {
	pub fn new(page: u32, index: u32) -> Self {
		Self { page, index }
	}
}

impl<V: Vocabulary, R: Read + Seek> inferdf_core::Module<V> for Module<V, R>
where
	V::Type: Ord,
	V::Value: Ord,
	V::Iri: Copy + DecodeWith<V>,
	V::Literal: Copy + DecodeWith<V>,
{
	type Error = Error;
	type Dataset<'a> = Dataset<'a, V, R> where Self: 'a;
	type Interpretation<'a> = Interpretation<'a, V, R> where Self: 'a;

	fn dataset(&self) -> Self::Dataset<'_> {
		Dataset::new(self)
	}

	fn interpretation(&self) -> Self::Interpretation<'_> {
		Interpretation::new(self)
	}
}
