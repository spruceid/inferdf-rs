use rdf_types::Vocabulary;

use crate::{uninterpreted, Id, IteratorWith, Quad, Triple};

pub mod composite;
pub mod local;

pub use composite::Interpretation as Composite;
pub use local::Interpretation as Local;

#[derive(Debug, thiserror::Error)]
#[error("equality contradiction")]
pub struct Contradiction(pub Id, pub Id);

pub trait Resource<'a, V: Vocabulary>: Clone {
	type Error;
	type Iris: 'a + IteratorWith<V, Item = Result<V::Iri, Self::Error>>;
	type Literals: 'a + IteratorWith<V, Item = Result<V::Literal, Self::Error>>;
	type DifferentFrom: 'a + IteratorWith<V, Item = Result<Id, Self::Error>>;

	fn as_iri(&self) -> Self::Iris;

	fn as_literal(&self) -> Self::Literals;

	fn different_from(&self) -> Self::DifferentFrom;

	fn terms(&self) -> ResourceTerms<'a, V, Self> {
		ResourceTerms {
			as_iri: self.as_iri(),
			as_literal: self.as_literal(),
		}
	}
}

pub struct ResourceTerms<'a, V: Vocabulary, R: Resource<'a, V>> {
	as_iri: R::Iris,
	as_literal: R::Literals,
}

impl<'a, V: Vocabulary, R: Resource<'a, V>> IteratorWith<V> for ResourceTerms<'a, V, R> {
	type Item = Result<uninterpreted::Term<V>, R::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		match self.as_iri.next_with(vocabulary) {
			Some(Ok(iri)) => Some(Ok(uninterpreted::Term::<V>::Id(rdf_types::Id::Iri(iri)))),
			Some(Err(e)) => Some(Err(e)),
			None => match self.as_literal.next_with(vocabulary) {
				Some(Ok(literal)) => Some(Ok(uninterpreted::Term::<V>::Literal(literal))),
				Some(Err(e)) => Some(Err(e)),
				None => None,
			},
		}
	}
}

/// Iterator over the (uninterpreted) terms representing the given resource.
pub enum OptionalResourceTerms<'a, V: Vocabulary, R: Resource<'a, V>> {
	None,
	Some(ResourceTerms<'a, V, R>),
}

impl<'a, V: Vocabulary, R: Resource<'a, V>> IteratorWith<V> for OptionalResourceTerms<'a, V, R> {
	type Item = Result<uninterpreted::Term<V>, R::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		match self {
			Self::None => None,
			Self::Some(i) => i.next_with(vocabulary),
		}
	}
}

/// Interpretation.
pub trait Interpretation<'a, V: Vocabulary>: Clone {
	type Error;

	type Resource: Resource<'a, V, Error = Self::Error>;

	/// Iterator over the interpreted resources.
	///
	/// Resources are ordered by identifier.
	type Resources: IteratorWith<V, Item = Result<(Id, Self::Resource), Self::Error>>; // ...

	/// Iterator over the IRI interpretations.
	///
	/// There is no guaranty over the order in which IRIs are presented, and
	/// the same IRI may be presented multiple times. However if it is presented
	/// multiple times, it will be with the same interpretation.
	type Iris: IteratorWith<V, Item = Result<(V::Iri, Id), Self::Error>>;

	/// Iterator over the literal interpretations.
	///
	/// There is no guaranty over the order in which literals are presented,
	/// and the same literal may be presented multiple times. However if it is
	/// presented multiple times, it will be with the same interpretation.
	type Literals: IteratorWith<V, Item = Result<(V::Literal, Id), Self::Error>>;

	/// Returns an iterator over all the interpreted resources.
	///
	/// The iterator may yield the same resource more than once.
	fn resources(&self) -> Result<Self::Resources, Self::Error>;

	fn get(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error>;

	/// Returns an iterator over the IRI interpretations.
	///
	/// There is no guaranty over the order in which IRIs are presented, and
	/// the same IRI may be presented multiple times. However if it is presented
	/// multiple times, it will be with the same interpretation.
	fn iris(&self) -> Result<Self::Iris, Self::Error>;

	fn iri_interpretation(
		&self,
		vocabulary: &mut V,
		iri: V::Iri,
	) -> Result<Option<Id>, Self::Error>;

	/// Returns an iterator over the literal interpretations.
	///
	/// There is no guaranty over the order in which literals are presented,
	/// and the same literal may be presented multiple times. However if it is
	/// presented multiple times, it will be with the same interpretation.
	fn literals(&self) -> Result<Self::Literals, Self::Error>;

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		literal: V::Literal,
	) -> Result<Option<Id>, Self::Error>;

	fn term_interpretation(
		&self,
		vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Option<Id>, Self::Error> {
		match term {
			uninterpreted::Term::<V>::Id(rdf_types::Id::Iri(iri)) => {
				self.iri_interpretation(vocabulary, iri)
			}
			uninterpreted::Term::<V>::Id(rdf_types::Id::Blank(_)) => Ok(None),
			uninterpreted::Term::<V>::Literal(l) => self.literal_interpretation(vocabulary, l),
		}
	}

	fn terms_of(
		&self,
		id: Id,
	) -> Result<OptionalResourceTerms<'a, V, Self::Resource>, Self::Error> {
		match self.get(id)? {
			Some(r) => Ok(OptionalResourceTerms::Some(r.terms())),
			None => Ok(OptionalResourceTerms::None),
		}
	}

	fn uninterpreted_triples_of(
		&self,
		vocabulary: &mut V,
		triple: Triple,
	) -> Result<Vec<uninterpreted::Triple<V>>, Self::Error>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::Literal: Copy,
	{
		let mut result = Vec::new();

		let mut subjects = self.terms_of(triple.0)?;
		while let Some(s) = subjects.next_with(vocabulary).transpose()? {
			let mut predicates = self.terms_of(triple.1)?;
			while let Some(p) = predicates.next_with(vocabulary).transpose()? {
				let mut objects = self.terms_of(triple.2)?;
				while let Some(o) = objects.next_with(vocabulary).transpose()? {
					result.push(uninterpreted::Triple::<V>::new(s, p, o))
				}
			}
		}

		Ok(result)
	}

	fn uninterpreted_quads_of(
		&self,
		vocabulary: &mut V,
		quad: Quad,
	) -> Result<Vec<uninterpreted::Quad<V>>, Self::Error>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::Literal: Copy,
	{
		let mut result = Vec::new();

		match quad.3 {
			Some(g) => {
				let mut graphs = self.terms_of(g)?;
				while let Some(g) = graphs.next_with(vocabulary).transpose()? {
					let mut subjects = self.terms_of(quad.0)?;
					while let Some(s) = subjects.next_with(vocabulary).transpose()? {
						let mut predicates = self.terms_of(quad.1)?;
						while let Some(p) = predicates.next_with(vocabulary).transpose()? {
							let mut objects = self.terms_of(quad.2)?;
							while let Some(o) = objects.next_with(vocabulary).transpose()? {
								result.push(uninterpreted::Quad::<V>::new(s, p, o, Some(g)))
							}
						}
					}
				}
			}
			None => {
				let mut subjects = self.terms_of(quad.0)?;
				while let Some(s) = subjects.next_with(vocabulary).transpose()? {
					let mut predicates = self.terms_of(quad.1)?;
					while let Some(p) = predicates.next_with(vocabulary).transpose()? {
						let mut objects = self.terms_of(quad.2)?;
						while let Some(o) = objects.next_with(vocabulary).transpose()? {
							result.push(uninterpreted::Quad::<V>::new(s, p, o, None))
						}
					}
				}
			}
		}

		Ok(result)
	}
}

pub trait InterpretationMut<'a, V: Vocabulary> {
	type Error;

	fn insert_term(
		&mut self,
		vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Id, Self::Error>;
}

pub trait Interpret<V: Vocabulary> {
	type Interpreted;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error>;
}

impl<V: Vocabulary, S: Interpret<V>, P: Interpret<V>, O: Interpret<V>> Interpret<V>
	for rdf_types::Triple<S, P, O>
{
	type Interpreted = rdf_types::Triple<S::Interpreted, P::Interpreted, O::Interpreted>;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		Ok(rdf_types::Triple(
			self.0.interpret(vocabulary, interpretation)?,
			self.1.interpret(vocabulary, interpretation)?,
			self.2.interpret(vocabulary, interpretation)?,
		))
	}
}

impl<V: Vocabulary> Interpret<V> for uninterpreted::Term<V> {
	type Interpreted = Id;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		interpretation.insert_term(vocabulary, self)
	}
}

impl<V: Vocabulary, T: Interpret<V>> Interpret<V> for Vec<T> {
	type Interpreted = Vec<T::Interpreted>;

	fn interpret<'a, I: InterpretationMut<'a, V>>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<Self::Interpreted, I::Error> {
		let mut result = Vec::with_capacity(self.len());

		for t in self {
			result.push(t.interpret(vocabulary, interpretation)?)
		}

		Ok(result)
	}
}
