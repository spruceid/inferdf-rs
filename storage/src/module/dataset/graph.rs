use std::{
	io::{Read, Seek},
	iter::Copied,
};

use derivative::Derivative;
use inferdf_core::{Cause, FailibleIterator, GetOrTryInsertWith, Id, Signed, Triple};
use locspan::Meta;
use rdf_types::Vocabulary;

use crate::{
	binary_search_page,
	module::{cache, Error},
	page, Module,
};

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Copy(bound = ""))]
pub struct Graph<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	desc: page::graphs::Description,
}

impl<'a, V: Vocabulary, R: Read> Graph<'a, V, R> {
	pub fn new(module: &'a Module<V, R>, desc: page::graphs::Description) -> Self {
		Self { module, desc }
	}
}

impl<'a, V: Vocabulary, R: Read + Seek> inferdf_core::dataset::Graph<'a> for Graph<'a, V, R> {
	type Error = Error;
	type Resource = Resource<'a>;
	type Triples = Triples<'a, V, R>;

	fn get_resource(&self, id: Id) -> Result<Option<Self::Resource>, Error> {
		Resource::find(
			self.module,
			id,
			self.desc.first_page + self.desc.triple_page_count,
			self.desc.resource_page_count,
		)
	}

	fn get_triple(&self, i: u32) -> Result<Option<Meta<Signed<Triple>, Cause>>, Error> {
		let (p, page_i) =
			page::TriplesPage::triple_page_index(self.module.sections.triples_per_page, i);
		let page_triples_count = page::triples::page_triple_count(
			self.desc.triple_count,
			p,
			self.module.sections.triples_per_page,
		);
		if p < self.desc.triple_page_count {
			let page = self
				.module
				.get_triples_page(p + self.desc.first_page, page_triples_count)?;
			Ok(page.get(page_i))
		} else {
			Ok(None)
		}
	}

	fn triples(&self) -> Self::Triples {
		Triples {
			module: self.module,
			index: 0,
			first_page_index: self.desc.first_page,
			page_index: self.desc.first_page,
			next_page_index: self.desc.first_page + self.desc.triple_page_count,
			triple_count: self.desc.triple_count,
			page: None,
		}
	}
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Resource<'a> {
	entry: cache::Ref<'a, page::resource_triples::Entry>,
}

impl<'a> Resource<'a> {
	fn find<V: Vocabulary, R: Read + Seek>(
		module: &'a Module<V, R>,
		id: Id,
		first_page: u32,
		page_count: u32,
	) -> Result<Option<Self>, Error> {
		binary_search_page(first_page, first_page + page_count, |i| {
			let page = module.get_resource_triples_page(i)?;
			Ok(page.find(id).map(|r| Resource {
				entry: cache::Ref::map(page, |page| page.get(r).unwrap()),
			}))
		})
	}
}

impl<'a> inferdf_core::dataset::graph::Resource<'a> for Resource<'a> {
	type TripleIndexes = cache::IntoIterEscape<'a, Copied<std::slice::Iter<'a, u32>>>;

	fn as_subject(&self) -> Self::TripleIndexes {
		cache::Aliasing::into_iter_escape(cache::Ref::aliasing_map(self.entry.clone(), |e| {
			e.as_subject.iter().copied()
		}))
	}

	fn as_predicate(&self) -> Self::TripleIndexes {
		cache::Aliasing::into_iter_escape(cache::Ref::aliasing_map(self.entry.clone(), |e| {
			e.as_predicate.iter().copied()
		}))
	}

	fn as_object(&self) -> Self::TripleIndexes {
		cache::Aliasing::into_iter_escape(cache::Ref::aliasing_map(self.entry.clone(), |e| {
			e.as_object.iter().copied()
		}))
	}
}

pub struct Triples<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	index: u32,
	first_page_index: u32,
	page_index: u32,
	next_page_index: u32,
	triple_count: u32,
	page: Option<cache::Aliasing<'a, page::triples::Iter<'a>>>,
}

impl<'a, V: Vocabulary, R: Read + Seek> FailibleIterator for Triples<'a, V, R> {
	type Item = (u32, Meta<Signed<Triple>, Cause>);
	type Error = Error;

	fn try_next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
		while self.page_index < self.next_page_index {
			let iter = self.page.get_or_try_insert_with::<Error>(|| {
				let page_triples_count = page::triples::page_triple_count(
					self.triple_count,
					self.page_index - self.first_page_index,
					self.module.sections.triples_per_page,
				);
				Ok(cache::Ref::aliasing_map(
					self.module
						.get_triples_page(self.page_index, page_triples_count)?,
					|p| p.iter(),
				))
			})?;

			match iter.next() {
				Some(triple) => {
					let i = self.index;
					self.index += 1;
					return Ok(Some((i, triple)));
				}
				None => {
					self.page = None;
					self.page_index += 1
				}
			}
		}

		Ok(None)
	}
}

impl<'a, V: Vocabulary, R: Read + Seek> Iterator for Triples<'a, V, R> {
	type Item = Result<(u32, Meta<Signed<Triple>, Cause>), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.try_next().transpose()
	}
}
