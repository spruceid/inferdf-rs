use inferdf_core::{
	interpretation::{Interpret, InterpretationMut},
	pattern::{self, Instantiate, PatternSubstitution},
	Cause, Entailment, Id, Quad, Signed, Triple,
};

pub mod inference;

use inference::rule::TripleStatement;
use locspan::Meta;
use rdf_types::{InsertIntoVocabulary, MapLiteral, Vocabulary};
use serde::{Deserialize, Serialize};

pub trait Context {
	type Error;
	type PatternMatching<'a>: 'a + Iterator<Item = Result<Quad, Self::Error>>
	where
		Self: 'a;

	fn pattern_matching(&self, pattern: Signed<pattern::Canonical>) -> Self::PatternMatching<'_>;

	fn new_resource(&mut self) -> Id;
}

pub trait Semantics {
	fn deduce<C: Context>(
		&self,
		context: &mut C,
		triple: Signed<Triple>,
		entailment_index: impl FnMut(Entailment) -> u32,
		new_triple: impl FnMut(Meta<MaybeTrusted<Signed<TripleStatement>>, Cause>),
	) -> Result<(), C::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Trust {
	Trusted,
	Untrusted,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MaybeTrusted<T> {
	Trusted(T),
	Untrusted(T),
}

impl<T> MaybeTrusted<T> {
	pub fn map<U>(self, f: impl FnOnce(T) -> U) -> MaybeTrusted<U> {
		match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(f(t)),
			Self::Untrusted(t) => MaybeTrusted::Untrusted(f(t)),
		}
	}
}

impl<V: Vocabulary, T: InsertIntoVocabulary<V>> InsertIntoVocabulary<V> for MaybeTrusted<T> {
	type Inserted = MaybeTrusted<T::Inserted>;

	fn insert_into_vocabulary(self, vocabulary: &mut V) -> Self::Inserted {
		match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(t.insert_into_vocabulary(vocabulary)),
			Self::Untrusted(t) => MaybeTrusted::Untrusted(t.insert_into_vocabulary(vocabulary)),
		}
	}
}

impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for MaybeTrusted<T> {
	type Output = MaybeTrusted<T::Output>;

	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
		match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(t.map_literal(f)),
			Self::Untrusted(t) => MaybeTrusted::Untrusted(t.map_literal(f)),
		}
	}
}

impl<V: Vocabulary, T: Interpret<V>> Interpret<V> for MaybeTrusted<T> {
	type Interpreted = MaybeTrusted<T::Interpreted>;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		Ok(match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(t.interpret(vocabulary, interpretation)?),
			Self::Untrusted(t) => MaybeTrusted::Untrusted(t.interpret(vocabulary, interpretation)?),
		})
	}
}

impl<T: Instantiate> Instantiate for MaybeTrusted<T> {
	type Output = MaybeTrusted<T::Output>;

	fn instantiate(
		&self,
		substitution: &mut PatternSubstitution,
		new_id: impl FnMut() -> Id,
	) -> Self::Output {
		match self {
			Self::Trusted(t) => MaybeTrusted::Trusted(t.instantiate(substitution, new_id)),
			Self::Untrusted(t) => MaybeTrusted::Untrusted(t.instantiate(substitution, new_id)),
		}
	}
}
