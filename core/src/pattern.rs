use crate::{
	interpretation::{Interpret, InterpretationMut},
	uninterpreted, Id, Triple,
};

pub mod map;

pub use map::BipolarMap;
use rdf_types::{InsertIntoVocabulary, MapLiteral, Vocabulary};
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

	pub fn is_id_or(&self, f: impl FnOnce(usize) -> bool) -> bool {
		match self {
			Self::Id(_) => true,
			Self::Var(x) => f(*x),
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

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		match self {
			Self::Id(term) => Ok(IdOrVar::Id(interpretation.insert_term(vocabulary, term)?)),
			Self::Var(x) => Ok(IdOrVar::Var(x)),
		}
	}
}

pub type Pattern<T = Id> = rdf_types::Triple<IdOrVar<T>, IdOrVar<T>, IdOrVar<T>>;

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
pub enum Canonical<T = Id> {
	AnySubject(AnySubject<T>),
	GivenSubject(T, GivenSubject<T>),
}

impl<T> From<rdf_types::Triple<T, T, T>> for Canonical<T> {
	fn from(value: rdf_types::Triple<T, T, T>) -> Self {
		Self::from_triple(value)
	}
}

impl<T> From<rdf_types::Triple<Option<T>, Option<T>, Option<T>>> for Canonical<T> {
	fn from(value: rdf_types::Triple<Option<T>, Option<T>, Option<T>>) -> Self {
		Self::from_option_triple(value)
	}
}

impl<T> From<Pattern<T>> for Canonical<T> {
	fn from(value: Pattern<T>) -> Self {
		Self::from_pattern(value)
	}
}

impl<T: Clone> From<Canonical<T>> for Pattern<T> {
	fn from(value: Canonical<T>) -> Self {
		let s = match value.subject().cloned() {
			PatternSubject::Any => IdOrVar::Var(0),
			PatternSubject::Given(id) => IdOrVar::Id(id),
		};

		let p = match value.predicate().cloned() {
			PatternPredicate::Any => IdOrVar::Var(1),
			PatternPredicate::SameAsSubject => IdOrVar::Var(0),
			PatternPredicate::Given(id) => IdOrVar::Id(id),
		};

		let o = match value.object().cloned() {
			PatternObject::Any => IdOrVar::Var(2),
			PatternObject::SameAsSubject => IdOrVar::Var(0),
			PatternObject::SameAsPredicate => IdOrVar::Var(1),
			PatternObject::Given(id) => IdOrVar::Id(id),
		};

		rdf_types::Triple(s, p, o)
	}
}

impl<T> Canonical<T> {
	pub fn from_triple(triple: rdf_types::Triple<T, T, T>) -> Self {
		Self::GivenSubject(
			triple.0,
			GivenSubject::GivenPredicate(
				triple.1,
				GivenSubjectGivenPredicate::GivenObject(triple.2),
			),
		)
	}

	pub fn from_option_triple(triple: rdf_types::Triple<Option<T>, Option<T>, Option<T>>) -> Self {
		match triple.0 {
			Some(s) => Self::GivenSubject(s, GivenSubject::from_option_triple(triple.1, triple.2)),
			None => Self::AnySubject(AnySubject::from_option_triple(triple.1, triple.2)),
		}
	}

	pub fn from_pattern(pattern: Pattern<T>) -> Self {
		match pattern.0 {
			IdOrVar::Id(s) => {
				Self::GivenSubject(s, GivenSubject::from_pattern(pattern.1, pattern.2))
			}
			IdOrVar::Var(s) => Self::AnySubject(AnySubject::from_pattern(s, pattern.1, pattern.2)),
		}
	}

	pub fn subject(&self) -> PatternSubject<&T> {
		match self {
			Self::AnySubject(_) => PatternSubject::Any,
			Self::GivenSubject(id, _) => PatternSubject::Given(id),
		}
	}

	pub fn predicate(&self) -> PatternPredicate<&T> {
		match self {
			Self::AnySubject(t) => t.predicate(),
			Self::GivenSubject(_, t) => t.predicate(),
		}
	}

	pub fn object(&self) -> PatternObject<&T> {
		match self {
			Self::AnySubject(t) => t.object(),
			Self::GivenSubject(_, t) => t.object(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternSubject<T = Id> {
	Any,
	Given(T),
}

impl<T> PatternSubject<T> {
	pub fn id(&self) -> Option<&T> {
		match self {
			Self::Any => None,
			Self::Given(id) => Some(id),
		}
	}

	pub fn into_id(self) -> Option<T> {
		match self {
			Self::Any => None,
			Self::Given(id) => Some(id),
		}
	}
}

impl<'a, T> PatternSubject<&'a T> {
	pub fn cloned(self) -> PatternSubject<T>
	where
		T: Clone,
	{
		match self {
			Self::Any => PatternSubject::Any,
			Self::Given(t) => PatternSubject::Given(t.clone()),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternPredicate<T = Id> {
	Any,
	SameAsSubject,
	Given(T),
}

impl<T> PatternPredicate<T> {
	pub fn id(&self) -> Option<&T> {
		match self {
			Self::Any => None,
			Self::SameAsSubject => None,
			Self::Given(id) => Some(id),
		}
	}

	pub fn into_id(self) -> Option<T> {
		match self {
			Self::Any => None,
			Self::SameAsSubject => None,
			Self::Given(id) => Some(id),
		}
	}
}

impl<'a, T> PatternPredicate<&'a T> {
	pub fn cloned(self) -> PatternPredicate<T>
	where
		T: Clone,
	{
		match self {
			Self::Any => PatternPredicate::Any,
			Self::SameAsSubject => PatternPredicate::SameAsSubject,
			Self::Given(t) => PatternPredicate::Given(t.clone()),
		}
	}
}

impl PatternPredicate {
	pub fn filter_triple(&self, triple: Triple) -> bool {
		match self {
			Self::Any => true,
			Self::SameAsSubject => triple.1 == triple.0,
			Self::Given(id) => triple.1 == *id,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternObject<T = Id> {
	Any,
	SameAsSubject,
	SameAsPredicate,
	Given(T),
}

impl<T> PatternObject<T> {
	pub fn id(&self) -> Option<&T> {
		match self {
			Self::Given(id) => Some(id),
			_ => None,
		}
	}

	pub fn into_id(self) -> Option<T> {
		match self {
			Self::Given(id) => Some(id),
			_ => None,
		}
	}
}

impl<'a, T> PatternObject<&'a T> {
	pub fn cloned(self) -> PatternObject<T>
	where
		T: Clone,
	{
		match self {
			Self::Any => PatternObject::Any,
			Self::SameAsSubject => PatternObject::SameAsSubject,
			Self::SameAsPredicate => PatternObject::SameAsPredicate,
			Self::Given(t) => PatternObject::Given(t.clone()),
		}
	}
}

impl PatternObject {
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
pub enum AnySubject<T = Id> {
	AnyPredicate(AnySubjectAnyPredicate<T>),
	SameAsSubject(AnySubjectGivenPredicate<T>),
	GivenPredicate(T, AnySubjectGivenPredicate<T>),
}

impl<T> AnySubject<T> {
	pub fn from_option_triple(p: Option<T>, o: Option<T>) -> Self {
		match p {
			Some(p) => Self::GivenPredicate(p, AnySubjectGivenPredicate::from_option(o)),
			None => Self::AnyPredicate(AnySubjectAnyPredicate::from_option(o)),
		}
	}

	pub fn from_pattern(s: usize, p: IdOrVar<T>, o: IdOrVar<T>) -> Self {
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

	pub fn predicate(&self) -> PatternPredicate<&T> {
		match self {
			Self::AnyPredicate(_) => PatternPredicate::Any,
			Self::SameAsSubject(_) => PatternPredicate::SameAsSubject,
			Self::GivenPredicate(id, _) => PatternPredicate::Given(id),
		}
	}

	pub fn object(&self) -> PatternObject<&T> {
		match self {
			Self::AnyPredicate(t) => t.object(),
			Self::SameAsSubject(t) => t.object(),
			Self::GivenPredicate(_, t) => t.object(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectAnyPredicate<T = Id> {
	AnyObject,
	SameAsSubject,
	SameAsPredicate,
	GivenObject(T),
}

impl<T> AnySubjectAnyPredicate<T> {
	pub fn from_option(o: Option<T>) -> Self {
		match o {
			Some(o) => Self::GivenObject(o),
			None => Self::AnyObject,
		}
	}

	pub fn from_pattern(s: usize, p: usize, o: IdOrVar<T>) -> Self {
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

	pub fn object(&self) -> PatternObject<&T> {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsSubject => PatternObject::SameAsSubject,
			Self::SameAsPredicate => PatternObject::SameAsPredicate,
			Self::GivenObject(id) => PatternObject::Given(id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AnySubjectGivenPredicate<T = Id> {
	AnyObject,
	SameAsSubject,
	GivenObject(T),
}

impl<T> AnySubjectGivenPredicate<T> {
	pub fn from_option(o: Option<T>) -> Self {
		match o {
			Some(o) => Self::GivenObject(o),
			None => Self::AnyObject,
		}
	}

	pub fn from_pattern(s: usize, o: IdOrVar<T>) -> Self {
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

	pub fn object(&self) -> PatternObject<&T> {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsSubject => PatternObject::SameAsSubject,
			Self::GivenObject(id) => PatternObject::Given(id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubject<T = Id> {
	AnyPredicate(GivenSubjectAnyPredicate<T>),
	GivenPredicate(T, GivenSubjectGivenPredicate<T>),
}

impl<T> GivenSubject<T> {
	pub fn from_option_triple(p: Option<T>, o: Option<T>) -> Self {
		match p {
			Some(p) => Self::GivenPredicate(p, GivenSubjectGivenPredicate::from_option(o)),
			None => Self::AnyPredicate(GivenSubjectAnyPredicate::from_option(o)),
		}
	}

	pub fn from_pattern(p: IdOrVar<T>, o: IdOrVar<T>) -> Self {
		match p {
			IdOrVar::Id(p) => Self::GivenPredicate(p, GivenSubjectGivenPredicate::from_pattern(o)),
			IdOrVar::Var(p) => Self::AnyPredicate(GivenSubjectAnyPredicate::from_pattern(p, o)),
		}
	}

	pub fn predicate(&self) -> PatternPredicate<&T> {
		match self {
			Self::AnyPredicate(_) => PatternPredicate::Any,
			Self::GivenPredicate(id, _) => PatternPredicate::Given(id),
		}
	}

	pub fn object(&self) -> PatternObject<&T> {
		match self {
			Self::AnyPredicate(t) => t.object(),
			Self::GivenPredicate(_, t) => t.object(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectAnyPredicate<T = Id> {
	AnyObject,
	SameAsPredicate,
	GivenObject(T),
}

impl<T> GivenSubjectAnyPredicate<T> {
	pub fn from_option(o: Option<T>) -> Self {
		match o {
			Some(o) => Self::GivenObject(o),
			None => Self::AnyObject,
		}
	}

	pub fn from_pattern(p: usize, o: IdOrVar<T>) -> Self {
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

	pub fn object(&self) -> PatternObject<&T> {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::SameAsPredicate => PatternObject::SameAsPredicate,
			Self::GivenObject(id) => PatternObject::Given(id),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GivenSubjectGivenPredicate<T = Id> {
	AnyObject,
	GivenObject(T),
}

impl<T> GivenSubjectGivenPredicate<T> {
	pub fn from_option(o: Option<T>) -> Self {
		match o {
			Some(o) => Self::GivenObject(o),
			None => Self::AnyObject,
		}
	}

	pub fn from_pattern(o: IdOrVar<T>) -> Self {
		match o {
			IdOrVar::Id(o) => Self::GivenObject(o),
			IdOrVar::Var(_) => Self::AnyObject,
		}
	}

	pub fn object(&self) -> PatternObject<&T> {
		match self {
			Self::AnyObject => PatternObject::Any,
			Self::GivenObject(id) => PatternObject::Given(id),
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

	pub fn contains(&self, x: usize) -> bool {
		self.0.contains_key(&x)
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

	pub fn to_vec(&self) -> Vec<Option<Id>> {
		let mut result = Vec::with_capacity(self.0.len());
		for i in 0..self.0.len() {
			result.push(self.0.get(&i).copied())
		}
		result
	}

	pub fn into_vec(self) -> Vec<Option<Id>> {
		let mut result = Vec::with_capacity(self.0.len());
		for i in 0..self.0.len() {
			result.push(self.0.get(&i).copied())
		}
		result
	}
}

pub trait Instantiate {
	type Output;

	fn instantiate(&self, substitution: &PatternSubstitution) -> Option<Self::Output>;
}

impl Instantiate for IdOrVar {
	type Output = Id;

	fn instantiate(&self, substitution: &PatternSubstitution) -> Option<Self::Output> {
		match self {
			Self::Id(id) => Some(*id),
			Self::Var(x) => substitution.get(*x),
		}
	}
}

impl Instantiate for Pattern {
	type Output = Triple;

	fn instantiate(&self, substitution: &PatternSubstitution) -> Option<Self::Output> {
		Some(rdf_types::Triple(
			self.0.instantiate(substitution)?,
			self.1.instantiate(substitution)?,
			self.2.instantiate(substitution)?,
		))
	}
}
