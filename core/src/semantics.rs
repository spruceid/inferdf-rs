use crate::{
	interpretation::{Interpret, InterpretationMut},
	module::sub_module::ResourceGenerator,
	pattern::{self, Instantiate, PatternSubstitution},
	Cause, Entailment, Fact, Id, IteratorWith, Quad, ReplaceId, Signed, Triple,
};

use locspan::Meta;
use rdf_types::{InsertIntoVocabulary, MapLiteral, Vocabulary};
use serde::{Deserialize, Serialize};

pub trait Semantics<V: Vocabulary> {
	fn deduce<C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		triple: Signed<Triple>,
		entailment_index: impl FnMut(Entailment) -> u32,
		new_triple: impl FnMut(Meta<MaybeTrusted<Signed<TripleStatement>>, Cause>),
	) -> Result<(), C::Error>;

	fn close<C: Context<V>>(
		&self,
		vocabulary: &mut V,
		context: &mut C,
		entailment_index: impl FnMut(Entailment) -> u32,
		new_triple: impl FnMut(Meta<MaybeTrusted<Signed<TripleStatement>>, Cause>),
	) -> Result<(), C::Error>;
}

pub trait Context<V: Vocabulary> {
	type Error;

	type Resources<'a, G: ResourceGenerator>: 'a + IteratorWith<V, Item = Result<Id, Self::Error>>
	where
		Self: 'a,
		G: 'a;

	type PatternMatching<'a, G: ResourceGenerator>: 'a
		+ IteratorWith<V, Item = Result<(Fact, bool), Self::Error>>
	where
		Self: 'a,
		G: 'a;

	type Reservation<'r>: ContextReservation<CompletedReservation = Self::CompletedReservation>
	where
		Self: 'r;
	type CompletedReservation;

	fn begin_reservation(&self) -> Self::Reservation<'_>;

	fn apply_reservation(&mut self, generator: Self::CompletedReservation);

	fn resources<'r, G: 'r + ResourceGenerator>(&'r self, generator: G) -> Self::Resources<'r, G>;

	fn pattern_matching<'a, G: 'a + ResourceGenerator>(
		&'a self,
		generator: G,
		pattern: Signed<pattern::Canonical>,
	) -> Self::PatternMatching<'a, G>;

	fn new_resource(&mut self) -> Id;

	fn insert_iri(&mut self, vocabulary: &mut V, iri: V::Iri) -> Result<Id, Self::Error>;

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		id: Id,
	) -> Result<Option<V::Literal>, Self::Error>;
}

pub trait ContextReservation: ResourceGenerator {
	type CompletedReservation;

	fn end(self) -> Self::CompletedReservation;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TripleStatement {
	Triple(Triple),
	Eq(Id, Id),
}

impl TripleStatement {
	pub fn with_graph(self, g: Option<Id>) -> QuadStatement {
		match self {
			Self::Triple(t) => QuadStatement::Quad(t.into_quad(g)),
			Self::Eq(a, b) => QuadStatement::Eq(a, b, g),
		}
	}
}

pub enum QuadStatement {
	Quad(Quad),
	Eq(Id, Id, Option<Id>),
}

impl ReplaceId for QuadStatement {
	fn replace_id(&mut self, a: Id, b: Id) {
		match self {
			Self::Quad(t) => t.replace_id(a, b),
			Self::Eq(c, d, g) => {
				c.replace_id(a, b);
				d.replace_id(a, b);
				g.replace_id(a, b);
			}
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Trust {
	Trusted,
	Untrusted,
}

impl Trust {
	pub fn is_trusted(&self) -> bool {
		matches!(self, Self::Trusted)
	}

	pub fn is_untrusted(&self) -> bool {
		matches!(self, Self::Untrusted)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MaybeTrusted<T> {
	Trusted(T),
	Untrusted(T),
}

impl<T> MaybeTrusted<T> {
	pub fn new(value: T, trust: Trust) -> Self {
		match trust {
			Trust::Trusted => Self::Trusted(value),
			Trust::Untrusted => Self::Untrusted(value),
		}
	}

	pub fn value(&self) -> &T {
		match self {
			Self::Trusted(t) => t,
			Self::Untrusted(t) => t,
		}
	}

	pub fn as_parts(&self) -> (&T, Trust) {
		match self {
			Self::Trusted(t) => (t, Trust::Trusted),
			Self::Untrusted(t) => (t, Trust::Untrusted),
		}
	}

	pub fn into_parts(self) -> (T, Trust) {
		match self {
			Self::Trusted(t) => (t, Trust::Trusted),
			Self::Untrusted(t) => (t, Trust::Untrusted),
		}
	}

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

	fn instantiate(&self, substitution: &PatternSubstitution) -> Option<Self::Output> {
		match self {
			Self::Trusted(t) => Some(MaybeTrusted::Trusted(t.instantiate(substitution)?)),
			Self::Untrusted(t) => Some(MaybeTrusted::Untrusted(t.instantiate(substitution)?)),
		}
	}
}
