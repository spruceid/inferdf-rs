pub mod cache;
pub mod decode;

use std::{
	io::{self, BufReader},
	marker::PhantomData,
};

pub use cache::Cache;
pub use decode::*;
use inferdf_core::{uninterpreted, Id, Signed, Triple};
use rdf_types::Vocabulary;

use crate::{page, Header, Sections};

pub struct Reader<V, R> {
	inner: BufReader<R>,
	header: Header,
	sections: Sections,
	cache: Cache,
	v: PhantomData<V>,
}

impl<V: Vocabulary, R: io::Read> Reader<V, R> {
	pub fn new(mut reader: BufReader<R>) -> Result<Self, Error> {
		let header = Header::decode(&mut (), &mut reader)?;
		let sections = Sections::new(&header);

		Ok(Self {
			inner: reader,
			header,
			sections,
			cache: Cache::new(),
			v: PhantomData,
		})
	}

	/// Iterate over the interpretations of the given term.
	pub fn interpretations_of(
		&self,
		vocabulary: &V,
		term: uninterpreted::Term<V>,
	) -> InterpretationsOf<R> {
		todo!()
	}

	/// Get the resource behind the given identifier.
	pub fn get_resource(&self, id: Id) -> Option<&page::resource_triples::Entry> {
		todo!()
	}

	/// Get the IRI identifier by the given index.
	pub fn get_iri(&self, index: u32) -> Option<V::Iri> {
		todo!()
	}

	/// Get the literal identifier by the given index.
	pub fn get_literal(&self, index: u32) -> Option<V::Literal> {
		todo!()
	}

	/// Get the triple identifier by the given index.
	pub fn get_triple(&self, index: u32) -> Option<Signed<Triple>> {
		todo!()
	}
}

pub struct InterpretationsOf<'a, R> {
	p: PhantomData<&'a R>,
}
