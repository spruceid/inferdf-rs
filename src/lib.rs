pub mod cause;
pub mod interpretation;
pub mod dataset;
// pub mod rule;

pub use cause::Cause;
use derivative::Derivative;
use rdf_types::{IriVocabulary, BlankIdVocabulary};

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
	Debug(bound="V::StringLiteral: std::fmt::Debug, V::Iri: std::fmt::Debug"),
	Clone(bound="V::StringLiteral: Clone, V::Iri: Clone"),
	Copy(bound="V::StringLiteral: Copy, V::Iri: Copy"),
	PartialEq(bound="V::StringLiteral: PartialEq, V::Iri: PartialEq"),
	Eq(bound="V::StringLiteral: Eq, V::Iri: Eq"),
	PartialOrd(bound="V::StringLiteral: PartialOrd, V::Iri: PartialOrd"),
	Ord(bound="V::StringLiteral: Ord, V::Iri: Ord"),
	Hash(bound="V::StringLiteral: std::hash::Hash, V::Iri: std::hash::Hash")
)]
pub struct GlobalLiteral<V: IriVocabulary + LiteralVocabulary> {
	value: <V as LiteralVocabulary>::StringLiteral,
	type_: <V as IriVocabulary>::Iri
}

pub type GlobalTerm<V> = rdf_types::Term<
	<V as IriVocabulary>::Iri,
	<V as BlankIdVocabulary>::BlankId,
	GlobalLiteral<V>
>;

pub type GlobalQuad<V> = rdf_types::Quad<
	GlobalTerm<V>,
	GlobalTerm<V>,
	GlobalTerm<V>,
	GlobalTerm<V>,
>;

pub type GlobalTriple<V> = rdf_types::Triple<
	GlobalTerm<V>,
	GlobalTerm<V>,
	GlobalTerm<V>
>;

pub trait TripleExt {
	fn into_pattern(self)-> Pattern;
}

impl TripleExt for Triple {
	fn into_pattern(self)-> Pattern {
		rdf_types::Triple(Some(self.0), Some(self.1), Some(self.2))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(usize);