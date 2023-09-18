use locspan::Meta;

use crate::{Id, Quad, Signed, Triple};

use super::Interface;

pub trait TryIntoGlobal: Sized {
	fn try_into_global<E>(
		self,
		interface: &Interface,
		new_global_resource: impl FnMut(Id) -> Result<Id, E>,
	) -> Result<Self, E>;
}

impl TryIntoGlobal for Id {
	fn try_into_global<E>(
		self,
		interface: &Interface,
		new_global_resource: impl FnMut(Id) -> Result<Id, E>,
	) -> Result<Self, E> {
		interface.get_or_try_insert_global(self, new_global_resource)
	}
}

impl<T: TryIntoGlobal> TryIntoGlobal for Option<T> {
	fn try_into_global<E>(
		self,
		interface: &Interface,
		new_global_resource: impl FnMut(Id) -> Result<Id, E>,
	) -> Result<Self, E> {
		self.map(|t| t.try_into_global(interface, new_global_resource))
			.transpose()
	}
}

impl TryIntoGlobal for Triple {
	fn try_into_global<E>(
		self,
		interface: &Interface,
		mut new_global_resource: impl FnMut(Id) -> Result<Id, E>,
	) -> Result<Self, E> {
		Ok(Self(
			self.0
				.try_into_global(interface, &mut new_global_resource)?,
			self.1
				.try_into_global(interface, &mut new_global_resource)?,
			self.2.try_into_global(interface, new_global_resource)?,
		))
	}
}

impl TryIntoGlobal for Quad {
	fn try_into_global<E>(
		self,
		interface: &Interface,
		mut new_global_resource: impl FnMut(Id) -> Result<Id, E>,
	) -> Result<Self, E> {
		Ok(Self::new(
			self.0
				.try_into_global(interface, &mut new_global_resource)?,
			self.1
				.try_into_global(interface, &mut new_global_resource)?,
			self.2
				.try_into_global(interface, &mut new_global_resource)?,
			self.3.try_into_global(interface, new_global_resource)?,
		))
	}
}

impl<T: TryIntoGlobal> TryIntoGlobal for Signed<T> {
	fn try_into_global<E>(
		self,
		interface: &Interface,
		new_global_resource: impl FnMut(Id) -> Result<Id, E>,
	) -> Result<Self, E> {
		Ok(Self(
			self.0,
			self.1.try_into_global(interface, new_global_resource)?,
		))
	}
}

impl<T: TryIntoGlobal, M> TryIntoGlobal for Meta<T, M> {
	fn try_into_global<E>(
		self,
		interface: &Interface,
		new_global_resource: impl FnMut(Id) -> Result<Id, E>,
	) -> Result<Self, E> {
		Ok(Self(
			self.0.try_into_global(interface, new_global_resource)?,
			self.1,
		))
	}
}
