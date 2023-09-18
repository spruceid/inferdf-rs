use std::io;

use educe::Educe;
use inferdf_core::{GraphFact, Id, IteratorWith};
use paged::{cache::UnboundRef, no_context_mut, UnboundSliceIter};
use rdf_types::Vocabulary;

use super::Error;
use crate::{header, Module};

#[derive(Educe)]
#[educe(Clone)]
pub struct Graph<'a, V: Vocabulary, R> {
	module: &'a Module<V, R>,
	desc: header::GraphDescription,
}

impl<'a, V: Vocabulary, R> Graph<'a, V, R> {
	pub(crate) fn new(module: &'a Module<V, R>, desc: header::GraphDescription) -> Self {
		Self { module, desc }
	}
}

impl<'a, V: Vocabulary, R: io::Seek + io::Read> inferdf_core::dataset::Graph<'a, V>
	for Graph<'a, V, R>
{
	type Error = Error;

	type Resource = Resource<'a>;

	type Resources = Resources<'a, R>;

	type Triples = Triples<'a, R>;

	fn resources(&self) -> Self::Resources {
		Resources {
			r: self.module.reader.iter(
				self.desc.resources,
				&self.module.cache.graph_resources,
				self.module.header.heap,
			),
		}
	}

	fn get_resource(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		let entry = self.module.reader.binary_search_by_key(
			self.desc.resources,
			&self.module.cache.graph_resources,
			no_context_mut(),
			self.module.header.heap,
			|entry, _| entry.id.cmp(&id),
		)?;

		Ok(entry.map(|r| Resource { r }))
	}

	fn len(&self) -> u32 {
		self.desc.facts.entry_count()
	}

	fn triples(&self) -> Self::Triples {
		Triples {
			r: self.module.reader.iter(
				self.desc.facts,
				&self.module.cache.graph_facts,
				self.module.header.heap,
			),
			index: 0,
		}
	}

	fn get_triple(
		&self,
		_vocabulary: &mut V,
		index: u32,
	) -> Result<Option<inferdf_core::GraphFact>, Self::Error> {
		self.module
			.reader
			.get(
				self.desc.facts,
				&self.module.cache.graph_facts,
				no_context_mut(),
				self.module.header.heap,
				index,
			)
			.map(|o| o.map(|t| (*t).into()))
	}
}

#[derive(Clone)]
pub struct Resource<'a> {
	r: paged::Ref<'a, header::GraphResource, UnboundRef<header::GraphResource>>,
}

impl<'a> inferdf_core::dataset::graph::Resource<'a> for Resource<'a> {
	type AsSubject = ResourceOccurrences<'a>;
	type AsPredicate = ResourceOccurrences<'a>;
	type AsObject = ResourceOccurrences<'a>;

	fn as_subject(&self) -> Self::AsSubject {
		ResourceOccurrences {
			inner: self.r.clone().map(header::GraphResourceAsSubjectBinder),
		}
	}

	fn as_predicate(&self) -> Self::AsPredicate {
		ResourceOccurrences {
			inner: self.r.clone().map(header::GraphResourceAsPredicateBinder),
		}
	}

	fn as_object(&self) -> Self::AsObject {
		ResourceOccurrences {
			inner: self.r.clone().map(header::GraphResourceAsObjectBinder),
		}
	}
}

pub struct ResourceOccurrences<'a> {
	inner: paged::Ref<'a, header::GraphResource, UnboundSliceIter<u32>>,
}

impl<'a> Iterator for ResourceOccurrences<'a> {
	type Item = u32;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(|item| item.copied().unwrap())
	}
}

pub struct Resources<'a, R> {
	r: paged::Iter<'a, 'a, R, header::GraphResource>,
}

impl<'a, R: io::Seek + io::Read> Iterator for Resources<'a, R> {
	type Item = Result<(Id, Resource<'a>), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.r.next().map(|r| {
			r.map(|resource| {
				let id = resource.id;
				(id, Resource { r: resource })
			})
		})
	}
}

impl<'a, V, R: io::Seek + io::Read> IteratorWith<V> for Resources<'a, R> {
	type Item = Result<(Id, Resource<'a>), Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct Triples<'a, R> {
	r: paged::Iter<'a, 'a, R, header::GraphFact>,
	index: u32,
}

impl<'a, R: io::Seek + io::Read> Iterator for Triples<'a, R> {
	type Item = Result<(u32, GraphFact), Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.r.next().map(|r| {
			r.map(|fact| {
				let i = self.index;
				self.index += 1;
				let fact: header::GraphFact = *fact;
				(i, fact.into())
			})
		})
	}
}

impl<'a, V, R: io::Seek + io::Read> IteratorWith<V> for Triples<'a, R> {
	type Item = Result<(u32, GraphFact), Error>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}
