use rdf_types::Vocabulary;

use crate::{module::sub_module::ResourceGenerator, Id};

use super::{reservable_slab, Interpretation, Resource};

pub use reservable_slab::InvalidReservation;

pub struct Reservation<'a, V: Vocabulary> {
	// interpretation: &'a Interpretation<V>,
	inner: reservable_slab::Reservation<'a, Resource<V>>,
}

impl<'a, V: Vocabulary> Reservation<'a, V> {
	pub fn new(
		// interpretation: &'a Interpretation<V>,
		inner: reservable_slab::Reservation<'a, Resource<V>>,
	) -> Self {
		Self { inner }
	}

	pub fn new_resource(&mut self) -> Id {
		Id(self.inner.insert(Resource::new()) as u32)
	}

	pub fn end(self) -> CompletedReservation<V> {
		CompletedReservation {
			inner: self.inner.end(),
		}
	}
}

impl<'a, V: Vocabulary> ResourceGenerator for Reservation<'a, V> {
	fn new_resource(&mut self) -> Id {
		self.new_resource()
	}
}

pub struct CompletedReservation<V: Vocabulary> {
	inner: reservable_slab::CompletedReservation<Resource<V>>,
}

impl<V: Vocabulary> CompletedReservation<V> {
	pub fn apply(self, interpretation: &mut Interpretation<V>) -> Result<(), InvalidReservation> {
		self.inner.apply(&mut interpretation.resources)
	}
}
