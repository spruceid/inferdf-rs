use locspan::Meta;

use crate::{Id, Quad, Signed, Triple};

use super::Interface;

pub trait IntoGlobal {
	fn into_global(self, interface: &Interface, new_global_resource: impl FnMut() -> Id) -> Self;
}

impl IntoGlobal for Id {
	fn into_global(self, interface: &Interface, new_global_resource: impl FnMut() -> Id) -> Self {
		interface.get_or_insert_global(self, new_global_resource)
	}
}

impl<T: IntoGlobal> IntoGlobal for Option<T> {
	fn into_global(self, interface: &Interface, new_global_resource: impl FnMut() -> Id) -> Self {
		self.map(|t| t.into_global(interface, new_global_resource))
	}
}

impl IntoGlobal for Triple {
	fn into_global(
		self,
		interface: &Interface,
		mut new_global_resource: impl FnMut() -> Id,
	) -> Self {
		Self(
			self.0.into_global(interface, &mut new_global_resource),
			self.1.into_global(interface, &mut new_global_resource),
			self.2.into_global(interface, new_global_resource),
		)
	}
}

impl IntoGlobal for Quad {
	fn into_global(
		self,
		interface: &Interface,
		mut new_global_resource: impl FnMut() -> Id,
	) -> Self {
		Self::new(
			self.0.into_global(interface, &mut new_global_resource),
			self.1.into_global(interface, &mut new_global_resource),
			self.2.into_global(interface, &mut new_global_resource),
			self.3.into_global(interface, new_global_resource),
		)
	}
}

impl<T: IntoGlobal> IntoGlobal for Signed<T> {
	fn into_global(self, interface: &Interface, new_global_resource: impl FnMut() -> Id) -> Self {
		Self(self.0, self.1.into_global(interface, new_global_resource))
	}
}

impl<T: IntoGlobal, M> IntoGlobal for Meta<T, M> {
	fn into_global(self, interface: &Interface, new_global_resource: impl FnMut() -> Id) -> Self {
		Self(self.0.into_global(interface, new_global_resource), self.1)
	}
}
