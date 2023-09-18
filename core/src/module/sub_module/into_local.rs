use crate::{pattern, Id, Quad, Signed, Triple};

use super::Interface;

pub trait IntoLocal: Sized {
	fn into_local(self, interface: &Interface) -> Option<Self>;
}

impl<T: IntoLocal> IntoLocal for Option<T> {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Some(t) => t.into_local(interface).map(Some),
			None => Some(None),
		}
	}
}

impl IntoLocal for Id {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		interface.local_id(self)
	}
}

impl IntoLocal for Triple {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		Some(Self::new(
			interface.local_id(self.0)?,
			interface.local_id(self.1)?,
			interface.local_id(self.2)?,
		))
	}
}

impl IntoLocal for Quad {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		let g = match self.3 {
			Some(g) => Some(interface.local_id(g)?),
			None => None,
		};

		Some(Self::new(
			interface.local_id(self.0)?,
			interface.local_id(self.1)?,
			interface.local_id(self.2)?,
			g,
		))
	}
}

impl IntoLocal for pattern::Canonical {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Self::AnySubject(po) => Some(Self::AnySubject(po.into_local(interface)?)),
			Self::GivenSubject(s, po) => Some(Self::GivenSubject(
				s.into_local(interface)?,
				po.into_local(interface)?,
			)),
		}
	}
}

impl IntoLocal for pattern::AnySubject {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Self::AnyPredicate(o) => Some(Self::AnyPredicate(o.into_local(interface)?)),
			Self::SameAsSubject(o) => Some(Self::SameAsSubject(o.into_local(interface)?)),
			Self::GivenPredicate(p, o) => Some(Self::GivenPredicate(
				p.into_local(interface)?,
				o.into_local(interface)?,
			)),
		}
	}
}

impl IntoLocal for pattern::GivenSubject {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Self::AnyPredicate(o) => Some(Self::AnyPredicate(o.into_local(interface)?)),
			Self::GivenPredicate(p, o) => Some(Self::GivenPredicate(
				p.into_local(interface)?,
				o.into_local(interface)?,
			)),
		}
	}
}

impl IntoLocal for pattern::AnySubjectAnyPredicate {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Self::AnyObject => Some(Self::AnyObject),
			Self::SameAsSubject => Some(Self::SameAsSubject),
			Self::SameAsPredicate => Some(Self::SameAsPredicate),
			Self::GivenObject(o) => Some(Self::GivenObject(o.into_local(interface)?)),
		}
	}
}

impl IntoLocal for pattern::AnySubjectGivenPredicate {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Self::AnyObject => Some(Self::AnyObject),
			Self::SameAsSubject => Some(Self::SameAsSubject),
			Self::GivenObject(o) => Some(Self::GivenObject(o.into_local(interface)?)),
		}
	}
}

impl IntoLocal for pattern::GivenSubjectAnyPredicate {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Self::AnyObject => Some(Self::AnyObject),
			Self::SameAsPredicate => Some(Self::SameAsPredicate),
			Self::GivenObject(o) => Some(Self::GivenObject(o.into_local(interface)?)),
		}
	}
}

impl IntoLocal for pattern::GivenSubjectGivenPredicate {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		match self {
			Self::AnyObject => Some(Self::AnyObject),
			Self::GivenObject(o) => Some(Self::GivenObject(o.into_local(interface)?)),
		}
	}
}

impl<T: IntoLocal> IntoLocal for Signed<T> {
	fn into_local(self, interface: &Interface) -> Option<Self> {
		Some(Self(self.0, self.1.into_local(interface)?))
	}
}
