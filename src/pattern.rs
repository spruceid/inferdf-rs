use crate::{interpretation::Interpret, uninterpreted, Id, Triple};

pub mod map;

pub use map::BipolarMap;
use rdf_types::{InsertIntoVocabulary, Vocabulary, MapLiteral};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IdOrVar<T = Id> {
	Id(T),
	Var(usize),
}

impl<T> IdOrVar<T> {
	pub fn map<U>(self, f: impl Fn(T) -> U) -> IdOrVar<U> {
		match self {
			Self::Id(t) => IdOrVar::Id(f(t)),
			Self::Var(x) => IdOrVar::Var(x),
		}
	}

	pub fn into_wrapping_iter(self) -> IdOrVarIter<T>
	where
		T: Iterator,
	{
		match self {
			Self::Id(iter) => IdOrVarIter::Id(iter),
			Self::Var(x) => IdOrVarIter::Var(Some(x)),
		}
	}
}

impl IdOrVar {
	pub fn matching(&self, substitution: &mut PatternSubstitution, id: Id) -> bool {
		match self {
			Self::Id(i) => *i == id,
			Self::Var(x) => substitution.bind(*x, id),
		}
	}
}

impl<T> From<T> for IdOrVar<T> {
	fn from(value: T) -> Self {
		Self::Id(value)
	}
}

impl<V, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for IdOrVar<T> {
	type Inserted = IdOrVar<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		match self {
			Self::Id(term) => IdOrVar::Id(term.insert_into_vocabulary(vocabulary)),
			Self::Var(x) => IdOrVar::Var(x),
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for IdOrVar<T> {
	type Output = IdOrVar<T::Output>;

	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
		match self {
			Self::Id(term) => IdOrVar::Id(term.map_literal(f)),
			Self::Var(x) => IdOrVar::Var(x),
		}
	}
}

impl<V: Vocabulary> Interpret<V> for IdOrVar<uninterpreted::Term<V>> {
	type Interpreted = IdOrVar;

	fn interpret(
		self,
		interpretation: &mut impl crate::interpretation::InterpretationMut<V>,
	) -> Self::Interpreted {
		match self {
			Self::Id(term) => IdOrVar::Id(interpretation.insert_term(term)),
			Self::Var(x) => IdOrVar::Var(x),
		}
	}
}

pub type Pattern<T = Id> = rdf_types::Triple<IdOrVar<T>, IdOrVar<T>, IdOrVar<T>>;

impl<V: Vocabulary> Interpret<V> for Pattern<uninterpreted::Term<V>> {
	type Interpreted = Pattern;

	fn interpret(
		self,
		interpretation: &mut impl crate::interpretation::InterpretationMut<V>,
	) -> Self::Interpreted {
		rdf_types::Triple(
			self.0.interpret(interpretation),
			self.1.interpret(interpretation),
			self.2.interpret(interpretation),
		)
	}
}

#[derive(Debug, Clone)]
pub enum IdOrVarIter<T = Id> {
	Id(T),
	Var(Option<usize>),
}

impl<I: Iterator> Iterator for IdOrVarIter<I> {
	type Item = IdOrVar<I::Item>;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Id(iter) => iter.next().map(IdOrVar::Id),
			Self::Var(x) => x.take().map(IdOrVar::Var),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Canonical {
	AnySubject(AnySubject),
	GivenSubject(Id, GivenSubject),
}

impl From<Pattern> for Canonical {
	fn from(value: Pattern) -> Self {
		Self::from_pattern(value)
	}
}

impl From<Canonical> for Pattern {
	fn from(value: Canonical) -> Self {
		let s = match value.subject() {
			PatternSubject::Any => IdOrVar::Var(0),
			PatternSubject::Given(id) => IdOrVar::Id(id),
		};

		let p = match value.predicate() {
			PatternPredicate::Any => IdOrVar::Var(1),
			PatternPredicate::SameAsSubject => IdOrVar::Var(0),
			PatternPredicate::Given(id) => IdOrVar::Id(id),
		};

		let o = match value.object() {
			PatternObject::Any => IdOrVar::Var(2),
			PatternObject::SameAsSubject => IdOrVar::Var(0),
			PatternObject::SameAsPredicate => IdOrVar::Var(1),
			PatternObject::Given(id) => IdOrVar::Id(id),
		};

		rdf_types::Triple(s, p, o)
	}
}

impl Canonical {
	pub fn from_triple(triple: Triple) -> Self {
		Self::GivenSubject(
			triple.0,
			GivenSubject::GivenPredicate(
				triple.1,
				GivenSubjectGivenPredicate::GivenObject(triple.2),
			),
		)
	}

	pub fn from_pattern(pattern: Pattern) -> Self {
		match pattern.0 {
			IdOrVar::Id(s) => {
				Self::GivenSubject(s, GivenSubject::from_pattern(pattern.1, pattern.2))
			}
			IdOrVar::Var(s) => Self::AnySubject(AnySubject::from_pattern(s, pattern.1, pattern.2)),
		}
	}

	pub fn subject(&self) -> PatternSubject {
		match self {
			Self::AnySubject(_) => PatternSubject::Any,
			Self::GivenSubject(id, _) => PatternSubject::Given(*id),
		}
	}

	pub fn predicate(&self) -> PatternPredicate {
		match self {
			Self::AnySubject(t) => t.predicate(),
			Self::GivenSubject(_, t) => t.predicate(),
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnySubject(t) => t.object(),
			Self::GivenSubject(_, t) => t.object(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternSubject {
	Any,
	Given(Id),
}

impl PatternSubject {
	pub fn id(&self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::Given(id) => Some(*id),
		}
	}

	pub fn into_id(self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::Given(id) => Some(id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternPredicate {
	Any,
	SameAsSubject,
	Given(Id),
}

impl PatternPredicate {
	pub fn id(&self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::SameAsSubject => None,
			Self::Given(id) => Some(*id),
		}
	}

	pub fn into_id(self) -> Option<Id> {
		match self {
			Self::Any => None,
			Self::SameAsSubject => None,
			Self::Given(id) => Some(id),
		}
	}

	pub fn filter_triple(&self, triple: Triple) -> bool {
		match self {
			Self::Any => true,
			Self::SameAsSubject => triple.1 == triple.0,
			Self::Given(id) => triple.1 == *id,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternObject {
	Any,
	SameAsSubject,
	SameAsPredicate,
	Given(Id),
}

impl PatternObject {
	pub fn id(&self) -> Option<Id> {
		match self {
			Self::Given(id) => Some(*id),
			_ => None,
		}
	}

	pub fn into_id(self) -> Option<Id> {
		match self {
			Self::Given(id) => Some(id),
			_ => None,
		}
	}

	pub fn filter_triple(&self, triple: Triple) -> bool {
		match self {
			Self::Any => true,
			Self::SameAsSubject => triple.2 == triple.0,
			Self::SameAsPredicate => triple.2 == triple.1,
			Self::Given(id) => triple.2 == *id,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubject {
	AnyPredicate(AnySubjectAnyPredicate),
	SameAsSubject(AnySubjectGivenPredicate),
	GivenPredicate(Id, AnySubjectGivenPredicate),
}

impl AnySubject {
	pub fn from_pattern(s: usize, p: IdOrVar, o: IdOrVar) -> Self {
		match p {
			IdOrVar::Id(p) => Self::GivenPredicate(p, AnySubjectGivenPredicate::from_pattern(s, o)),
			IdOrVar::Var(p) => {
				if p == s {
					Self::SameAsSubject(AnySubjectGivenPredicate::from_pattern(s, o))
				} else {
					Self::AnyPredicate(AnySubjectAnyPredicate::from_pattern(s, p, o))
				}
			}
		}
	}

	pub fn predicate(&self) -> PatternPredicate {
		match self {
			Self::AnyPredicate(_) => PatternPredicate::Any,
			Self::SameAsSubject(_) => PatternPredicate::SameAsSubject,
			Self::GivenPredicate(id, _) => PatternPredicate::Given(*id),
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyPredicate(t) => t.object(),
			Self::SameAsSubject(t) => t.object(),
			Self::GivenPredicate(_, t) => t.object(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectAnyPredicate {
	AnyObject,
	SameAsSubject,
	SameAsPredicate,
	GivenObject(Id),
}

impl AnySubjectAnyPredicate {
	pub fn from_pattern(s: usize, p: usize, o: IdOrVar) -> Self {
		match o {
			IdOrVar::Id(o) => Self::GivenObject(o),
			IdOrVar::Var(o) => {
				if o == s {
					Self::SameAsSubject
				} else if o == p {
					Self::SameAsPredicate
				} else {
					Self::AnyObject
				}
			}
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsSubject => PatternObject::SameAsSubject,
			Self::SameAsPredicate => PatternObject::SameAsPredicate,
			Self::GivenObject(id) => PatternObject::Given(*id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectGivenPredicate {
	AnyObject,
	SameAsSubject,
	GivenObject(Id),
}

impl AnySubjectGivenPredicate {
	pub fn from_pattern(s: usize, o: IdOrVar) -> Self {
		match o {
			IdOrVar::Id(o) => Self::GivenObject(o),
			IdOrVar::Var(o) => {
				if o == s {
					Self::SameAsSubject
				} else {
					Self::AnyObject
				}
			}
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsSubject => PatternObject::SameAsSubject,
			Self::GivenObject(id) => PatternObject::Given(*id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubject {
	AnyPredicate(GivenSubjectAnyPredicate),
	GivenPredicate(Id, GivenSubjectGivenPredicate),
}

impl GivenSubject {
	pub fn from_pattern(p: IdOrVar, o: IdOrVar) -> Self {
		match p {
			IdOrVar::Id(p) => Self::GivenPredicate(p, GivenSubjectGivenPredicate::from_pattern(o)),
			IdOrVar::Var(p) => Self::AnyPredicate(GivenSubjectAnyPredicate::from_pattern(p, o)),
		}
	}

	pub fn predicate(&self) -> PatternPredicate {
		match self {
			Self::AnyPredicate(_) => PatternPredicate::Any,
			Self::GivenPredicate(id, _) => PatternPredicate::Given(*id),
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyPredicate(t) => t.object(),
			Self::GivenPredicate(_, t) => t.object(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectAnyPredicate {
	AnyObject,
	SameAsPredicate,
	GivenObject(Id),
}

impl GivenSubjectAnyPredicate {
	pub fn from_pattern(p: usize, o: IdOrVar) -> Self {
		match o {
			IdOrVar::Id(o) => Self::GivenObject(o),
			IdOrVar::Var(o) => {
				if p == o {
					Self::SameAsPredicate
				} else {
					Self::AnyObject
				}
			}
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsPredicate => PatternObject::SameAsPredicate,
			Self::GivenObject(id) => PatternObject::Given(*id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectGivenPredicate {
	AnyObject,
	GivenObject(Id),
}

impl GivenSubjectGivenPredicate {
	pub fn from_pattern(o: IdOrVar) -> Self {
		match o {
			IdOrVar::Id(o) => Self::GivenObject(o),
			IdOrVar::Var(_) => Self::AnyObject,
		}
	}

	pub fn object(&self) -> PatternObject {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::GivenObject(id) => PatternObject::Given(*id),
		}
	}
}

pub trait Matching {
	fn matching(&self, substitution: &mut PatternSubstitution, t: crate::Triple) -> bool;
}

impl Matching for Pattern {
	fn matching(&self, substitution: &mut PatternSubstitution, t: crate::Triple) -> bool {
		self.0.matching(substitution, t.0)
			&& self.1.matching(substitution, t.1)
			&& self.2.matching(substitution, t.2)
	}
}

#[derive(Debug, Default, Clone)]
pub struct PatternSubstitution(im::HashMap<usize, Id>);

impl PatternSubstitution {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get(&self, x: usize) -> Option<Id> {
		self.0.get(&x).copied()
	}

	/// Bind the variable `x` to the given identifier, unless it is already
	/// bound to a different identifier.
	///
	/// Returns wether the binding succeeded.
	pub fn bind(&mut self, x: usize, id: Id) -> bool {
		*self.0.entry(x).or_insert(id) == id
	}

	pub fn get_or_insert_with(&mut self, x: usize, f: impl FnOnce() -> Id) -> Id {
		*self.0.entry(x).or_insert_with(f)
	}
}

pub trait Instantiate {
	type Output;

	fn instantiate(
		&self,
		substitution: &mut PatternSubstitution,
		new_id: impl FnMut() -> Id,
	) -> Self::Output;
}

impl Instantiate for IdOrVar {
	type Output = Id;

	fn instantiate(
		&self,
		substitution: &mut PatternSubstitution,
		new_id: impl FnMut() -> Id,
	) -> Self::Output {
		match self {
			Self::Id(id) => *id,
			Self::Var(x) => substitution.get_or_insert_with(*x, new_id),
		}
	}
}

impl Instantiate for Pattern {
	type Output = Triple;

	fn instantiate(
		&self,
		substitution: &mut PatternSubstitution,
		mut new_id: impl FnMut() -> Id,
	) -> Self::Output {
		rdf_types::Triple(
			self.0.instantiate(substitution, &mut new_id),
			self.1.instantiate(substitution, &mut new_id),
			self.2.instantiate(substitution, &mut new_id),
		)
	}
}
