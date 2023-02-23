pub mod builder;
pub mod cause;
pub mod dataset;
pub mod interpretation;
// pub mod rule;

pub use cause::Cause;
use derivative::Derivative;
use rdf_types::{BlankIdVocabulary, IriVocabulary};

pub type Triple = rdf_types::Triple<Id, Id, Id>;
pub type Pattern = rdf_types::Triple<Option<Id>, Option<Id>, Option<Id>>;
pub type Quad = rdf_types::Quad<Id, Id, Id, Id>;

pub trait Vocabulary: IriVocabulary + BlankIdVocabulary + LiteralVocabulary {}

pub trait LiteralVocabulary {
	type StringLiteral;
	type LanguageTag;
}

#[derive(Derivative)]
#[derivative(
	Debug(
		bound = "V::StringLiteral: std::fmt::Debug, V::Iri: std::fmt::Debug, V::BlankId: std::fmt::Debug"
	),
	Clone(bound = "V::StringLiteral: Clone, V::Iri: Clone, V::BlankId: Clone"),
	PartialEq(bound = "V::StringLiteral: PartialEq, V::Iri: PartialEq, V::BlankId: PartialEq"),
	Eq(bound = "V::StringLiteral: Eq, V::Iri: Eq, V::BlankId: Eq"),
	PartialOrd(bound = "V::StringLiteral: PartialOrd, V::Iri: PartialOrd, V::BlankId: PartialOrd"),
	Ord(bound = "V::StringLiteral: Ord, V::Iri: Ord, V::BlankId: Ord"),
	Hash(
		bound = "V::StringLiteral: std::hash::Hash, V::Iri: std::hash::Hash, V::BlankId: std::hash::Hash"
	)
)]
pub struct GlobalLiteral<V: Vocabulary> {
	pub value: <V as LiteralVocabulary>::StringLiteral,
	pub type_: Box<GlobalTerm<V>>,
}

impl<V: Vocabulary> GlobalLiteral<V> {
	pub fn new(value: <V as LiteralVocabulary>::StringLiteral, type_: GlobalTerm<V>) -> Self {
		Self {
			value,
			type_: Box::new(type_),
		}
	}

	pub fn with_interpreted_type(self, id: Id) -> SemiInterpretedLiteral<V> {
		SemiInterpretedLiteral::new(self.value, id)
	}

	pub fn interpret_type_with_mut(
		&self,
		f: &mut impl FnMut(SemiInterpretedTerm<V>) -> Id,
	) -> SemiInterpretedLiteral<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		let a = self.type_.interpret_literal_type_with(&mut *f);
		SemiInterpretedLiteral::new(self.value, f(a))
	}

	pub fn try_interpret_type_with_mut(
		&self,
		f: &mut impl FnMut(SemiInterpretedTerm<V>) -> Option<Id>,
	) -> Option<SemiInterpretedLiteral<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		let a = self.type_.try_interpret_literal_type_with(&mut *f)?;
		Some(SemiInterpretedLiteral::new(self.value, f(a)?))
	}

	pub fn interpret_type_with(
		&self,
		mut f: impl FnMut(SemiInterpretedTerm<V>) -> Id,
	) -> SemiInterpretedLiteral<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		self.interpret_type_with_mut(&mut f)
	}

	pub fn try_interpret_type_with(
		&self,
		mut f: impl FnMut(SemiInterpretedTerm<V>) -> Option<Id>,
	) -> Option<SemiInterpretedLiteral<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		self.try_interpret_type_with_mut(&mut f)
	}
}

#[derive(Derivative)]
#[derivative(
	Debug(bound = "V::StringLiteral: std::fmt::Debug"),
	Clone(bound = "V::StringLiteral: Clone"),
	Copy(bound = "V::StringLiteral: Copy"),
	PartialEq(bound = "V::StringLiteral: PartialEq"),
	Eq(bound = "V::StringLiteral: Eq"),
	PartialOrd(bound = "V::StringLiteral: PartialOrd"),
	Ord(bound = "V::StringLiteral: Ord"),
	Hash(bound = "V::StringLiteral: std::hash::Hash")
)]
pub struct SemiInterpretedLiteral<V: Vocabulary> {
	pub value: <V as LiteralVocabulary>::StringLiteral,
	pub type_: Id,
}

impl<V: Vocabulary> SemiInterpretedLiteral<V> {
	pub fn new(value: <V as LiteralVocabulary>::StringLiteral, type_: Id) -> Self {
		Self { value, type_ }
	}
}

pub type GlobalTerm<V> =
	rdf_types::Term<<V as IriVocabulary>::Iri, <V as BlankIdVocabulary>::BlankId, GlobalLiteral<V>>;

pub type SemiInterpretedTerm<V> = rdf_types::Term<
	<V as IriVocabulary>::Iri,
	<V as BlankIdVocabulary>::BlankId,
	SemiInterpretedLiteral<V>,
>;

pub trait GlobalTermExt<V: Vocabulary> {
	fn interpret_literal_type_with_mut(
		&self,
		f: &mut impl FnMut(SemiInterpretedTerm<V>) -> Id,
	) -> SemiInterpretedTerm<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy;

	fn try_interpret_literal_type_with_mut(
		&self,
		f: &mut impl FnMut(SemiInterpretedTerm<V>) -> Option<Id>,
	) -> Option<SemiInterpretedTerm<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy;

	fn interpret_literal_type_with(
		&self,
		mut f: impl FnMut(SemiInterpretedTerm<V>) -> Id,
	) -> SemiInterpretedTerm<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		self.interpret_literal_type_with_mut(&mut f)
	}

	fn try_interpret_literal_type_with(
		&self,
		mut f: impl FnMut(SemiInterpretedTerm<V>) -> Option<Id>,
	) -> Option<SemiInterpretedTerm<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		self.try_interpret_literal_type_with_mut(&mut f)
	}
}

impl<V: Vocabulary> GlobalTermExt<V> for GlobalTerm<V> {
	fn interpret_literal_type_with_mut(
		&self,
		f: &mut impl FnMut(SemiInterpretedTerm<V>) -> Id,
	) -> SemiInterpretedTerm<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		match self {
			Self::Iri(iri) => SemiInterpretedTerm::Iri(*iri),
			Self::Blank(blank) => SemiInterpretedTerm::Blank(*blank),
			Self::Literal(literal) => {
				SemiInterpretedTerm::Literal(literal.interpret_type_with_mut(f))
			}
		}
	}

	fn try_interpret_literal_type_with_mut(
		&self,
		f: &mut impl FnMut(SemiInterpretedTerm<V>) -> Option<Id>,
	) -> Option<SemiInterpretedTerm<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		match self {
			Self::Iri(iri) => Some(SemiInterpretedTerm::Iri(*iri)),
			Self::Blank(blank) => Some(SemiInterpretedTerm::Blank(*blank)),
			Self::Literal(literal) => Some(SemiInterpretedTerm::Literal(
				literal.try_interpret_type_with_mut(f)?,
			)),
		}
	}
}

pub type GlobalQuad<V> =
	rdf_types::Quad<GlobalTerm<V>, GlobalTerm<V>, GlobalTerm<V>, GlobalTerm<V>>;

pub trait GlobalQuadExt<V: Vocabulary> {
	fn interpret_literal_types_with(
		&self,
		f: impl FnMut(SemiInterpretedTerm<V>) -> Id,
	) -> SemiInterpretedQuad<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy;
}

impl<V: Vocabulary> GlobalQuadExt<V> for GlobalQuad<V> {
	fn interpret_literal_types_with(
		&self,
		mut f: impl FnMut(SemiInterpretedTerm<V>) -> Id,
	) -> SemiInterpretedQuad<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		rdf_types::Quad(
			self.subject().interpret_literal_type_with(&mut f),
			self.predicate().interpret_literal_type_with(&mut f),
			self.object().interpret_literal_type_with(&mut f),
			self.graph().map(|g| g.interpret_literal_type_with(f)),
		)
	}
}

pub type SemiInterpretedQuad<V> = rdf_types::Quad<
	SemiInterpretedTerm<V>,
	SemiInterpretedTerm<V>,
	SemiInterpretedTerm<V>,
	SemiInterpretedTerm<V>,
>;

pub type GlobalTriple<V> = rdf_types::Triple<GlobalTerm<V>, GlobalTerm<V>, GlobalTerm<V>>;

pub type SemiInterpretedTriple<V> =
	rdf_types::Triple<SemiInterpretedTerm<V>, SemiInterpretedTerm<V>, SemiInterpretedTerm<V>>;

pub trait TripleExt {
	fn into_pattern(self) -> Pattern;
}

impl TripleExt for Triple {
	fn into_pattern(self) -> Pattern {
		rdf_types::Triple(Some(self.0), Some(self.1), Some(self.2))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);
