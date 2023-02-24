use crate::Id;

pub mod map;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pattern {
	AnySubject(AnySubject),
	GivenSubject(Id, GivenSubject)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubject {
	AnyPredicate(AnySubjectAnyPredicate),
	SameAsSubject(AnySubjectGivenPredicate),
	GivenPredicate(Id, AnySubjectGivenPredicate)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectAnyPredicate {
	AnyObject,
	GivenObject(Id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectGivenPredicate {
	AnyObject,
	SameAsSubject,
	GivenObject(Id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubject {
	AnyPredicate(GivenSubjectAnyPredicate),
	GivenPredicate(Id, GivenSubjectGivenPredicate)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectAnyPredicate {
	AnyObject,
	SameAsPredicate,
	GivenObject(Id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectGivenPredicate {
	AnyObject,
	GivenObject(Id)
}