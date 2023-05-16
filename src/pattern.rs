use crate::{Id, Triple};

pub mod map;

pub use map::BipolarMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pattern {
	AnySubject(AnySubject),
	GivenSubject(Id, GivenSubject)
}

impl Pattern {
	pub fn from_triple(triple: Triple) -> Self {
		Self::GivenSubject(
			triple.0,
			GivenSubject::GivenPredicate(
				triple.1,
				GivenSubjectGivenPredicate::GivenObject(triple.2)
			)
		)
	}

	pub fn subject(&self) -> PatternSubject {
		match self {
			Self::AnySubject(_) => PatternSubject::Any,
			Self::GivenSubject(id, _) => PatternSubject::Given(*id)
		}
	}

	pub fn predicate(&self) -> PatternPredicate {
		match self {
			Self::AnySubject(t) => t.predicate(),
			Self::GivenSubject(_, t) => t.predicate()
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnySubject(t) => t.object(),
			Self::GivenSubject(_, t) => t.object()
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternSubject {
	Any,
	Given(Id)
}

impl PatternSubject {
	pub fn id(&self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::Given(id) => Some(*id)
		}
	}

	pub fn into_id(self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::Given(id) => Some(id)
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternPredicate {
	Any,
	SameAsSubject,
	Given(Id)
}

impl PatternPredicate {
	pub fn id(&self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::SameAsSubject => None,
			Self::Given(id) => Some(*id)
		}
	}

	pub fn into_id(self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::SameAsSubject => None,
			Self::Given(id) => Some(id)
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternObject {
	Any,
	SameAsSubject,
	SameAsPredicate,
	Given(Id)
}

impl PatternObject {
	pub fn id(&self) -> Option<Id> {
		match self {
			Self::Given(id) => Some(*id),
			_ => None
		}
	}

	pub fn into_id(self) -> Option<Id> {
		match self {
			Self::Given(id) => Some(id),
			_ => None
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubject {
	AnyPredicate(AnySubjectAnyPredicate),
	SameAsSubject(AnySubjectGivenPredicate),
	GivenPredicate(Id, AnySubjectGivenPredicate)
}

impl AnySubject {
	pub fn predicate(&self) -> PatternPredicate {
		match self {
			Self::AnyPredicate(_) => PatternPredicate::Any,
			Self::SameAsSubject(_) => PatternPredicate::SameAsSubject,
			Self::GivenPredicate(id, _) => PatternPredicate::Given(*id)
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyPredicate(t) => t.object(),
			Self::SameAsSubject(t) => t.object(),
			Self::GivenPredicate(_, t) => t.object()
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectAnyPredicate {
	AnyObject,
	GivenObject(Id)
}

impl AnySubjectAnyPredicate {
	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::GivenObject(id) => PatternObject::Given(*id)
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectGivenPredicate {
	AnyObject,
	SameAsSubject,
	GivenObject(Id)
}

impl AnySubjectGivenPredicate {
	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsSubject => PatternObject::SameAsSubject,
			Self::GivenObject(id) => PatternObject::Given(*id)
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubject {
	AnyPredicate(GivenSubjectAnyPredicate),
	GivenPredicate(Id, GivenSubjectGivenPredicate)
}

impl GivenSubject {
	pub fn predicate(&self) -> PatternPredicate {
		match self {
			Self::AnyPredicate(_) => PatternPredicate::Any,
			Self::GivenPredicate(id, _) => PatternPredicate::Given(*id)
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyPredicate(t) => t.object(),
			Self::GivenPredicate(_, t) => t.object()
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectAnyPredicate {
	AnyObject,
	SameAsPredicate,
	GivenObject(Id)
}

impl GivenSubjectAnyPredicate {
	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsPredicate => PatternObject::SameAsPredicate,
			Self::GivenObject(id) => PatternObject::Given(*id)
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectGivenPredicate {
	AnyObject,
	GivenObject(Id)
}

impl GivenSubjectGivenPredicate {
	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::GivenObject(id) => PatternObject::Given(*id)
		}
	}
}

pub trait Matching {
	fn matching(&self, substitution: &mut PatternSubstitution, t: crate::Triple) -> bool;
}

impl Matching for Pattern {
	fn matching(&self, substitution: &mut PatternSubstitution, t: crate::Triple) -> bool {
		todo!()
	}
}

pub struct PatternSubstitution;