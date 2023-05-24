use rdf_types::{BlankIdVocabulary, IriVocabulary, LiteralVocabulary}; //, LanguageTagVocabulary};

// /// Uninterpreted literal value.
// pub type Literal<V> = rdf_types::Literal<<V as IriVocabulary>::Iri, <V as LanguageTagVocabulary>::LanguageTag>;

/// Uninterpreted node identifier.
pub type Id<V> = rdf_types::Id<<V as IriVocabulary>::Iri, <V as BlankIdVocabulary>::BlankId>;

/// Uninterpreted term.
pub type Term<V> = rdf_types::Term<Id<V>, <V as LiteralVocabulary>::Literal>;

/// Uninterpreted triple.
pub type Triple<V> = rdf_types::Triple<Term<V>, Term<V>, Term<V>>;

/// Uninterpreted quad.
pub type Quad<V> = rdf_types::Quad<Term<V>, Term<V>, Term<V>, Term<V>>;
