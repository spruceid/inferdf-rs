use std::hash::Hash;

use derivative::Derivative;
use hashbrown::{HashMap, HashSet};
use rdf_types::{LiteralVocabulary, Vocabulary};
use slab::Slab;

use crate::{uninterpreted, Id, Quad, Triple};

pub mod composite;
pub use composite::CompositeInterpretation;

pub trait InterpretationMut<V: Vocabulary> {
	fn insert_term(&mut self, term: uninterpreted::Term<V>) -> Id;
}

pub trait Interpret<V: Vocabulary> {
	type Interpreted;

	fn interpret(self, interpretation: &mut impl InterpretationMut<V>) -> Self::Interpreted;
}

impl<V: Vocabulary> Interpret<V> for uninterpreted::Term<V> {
	type Interpreted = Id;

	fn interpret(
		self,
		interpretation: &mut impl crate::interpretation::InterpretationMut<V>,
	) -> Self::Interpreted {
		interpretation.insert_term(self)
	}
}

impl<V: Vocabulary, T: Interpret<V>> Interpret<V> for Vec<T> {
	type Interpreted = Vec<T::Interpreted>;

	fn interpret(
		self,
		interpretation: &mut impl crate::interpretation::InterpretationMut<V>,
	) -> Self::Interpreted {
		self.into_iter()
			.map(|t| t.interpret(interpretation))
			.collect()
	}
}

/// RDF interpretation.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Interpretation<V: Vocabulary> {
	resources: Slab<Resource<V>>,
	by_iri: HashMap<V::Iri, Id>,
	by_blank: HashMap<V::BlankId, Id>,
	by_literal: HashMap<V::Literal, Id>,
}

impl<V: Vocabulary> InterpretationMut<V> for Interpretation<V>
where
	V::Iri: Copy + Eq + Hash,
	V::BlankId: Copy + Eq + Hash,
	V::Literal: Copy + Eq + Hash,
{
	fn insert_term(&mut self, term: uninterpreted::Term<V>) -> Id {
		self.insert_term(term)
	}
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Resource<V: Vocabulary> {
	as_iri: HashSet<V::Iri>,
	as_blank: HashSet<V::BlankId>,
	as_literal: HashSet<V::Literal>,
	different_from: HashSet<Id>,
}

impl<V: Vocabulary> Resource<V> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn from_iri(iri: V::Iri) -> Self
	where
		V::Iri: Eq + Hash,
	{
		Self {
			as_iri: std::iter::once(iri).collect(),
			as_blank: HashSet::new(),
			as_literal: HashSet::new(),
			different_from: HashSet::new(),
		}
	}

	pub fn from_blank(blank: V::BlankId) -> Self
	where
		V::BlankId: Eq + Hash,
	{
		Self {
			as_iri: HashSet::new(),
			as_blank: std::iter::once(blank).collect(),
			as_literal: HashSet::new(),
			different_from: HashSet::new(),
		}
	}

	pub fn from_literal(value: V::Literal) -> Self
	where
		V::Literal: Eq + Hash,
	{
		Self {
			as_iri: HashSet::new(),
			as_blank: HashSet::new(),
			as_literal: std::iter::once(value).collect(),
			different_from: HashSet::new(),
		}
	}
}

pub type ResourceLiteralInstances<V> = HashMap<<V as LiteralVocabulary>::Literal, Id>;

pub struct Contradiction(pub Id, pub Id);

impl<V: Vocabulary> Interpretation<V> {
	pub fn terms_of(&self, id: Id) -> TermsOf<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::Literal: Copy,
	{
		let r = &self.resources[id.index()];

		TermsOf {
			as_iri: r.as_iri.iter(),
			as_blank: r.as_blank.iter(),
			as_literal: r.as_literal.iter(),
		}
	}

	pub fn global_triple_of(&self, triple: Triple) -> Vec<uninterpreted::Triple<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::Literal: Copy,
	{
		let mut result = Vec::new();

		for s in self.terms_of(triple.0) {
			for p in self.terms_of(triple.1) {
				for o in self.terms_of(triple.2) {
					result.push(uninterpreted::Triple::<V>::new(s, p, o))
				}
			}
		}

		result
	}

	pub fn get(&self, id: Id) -> Option<&Resource<V>> {
		self.resources.get(id.index())
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut Resource<V>> {
		self.resources.get_mut(id.index())
	}

	pub fn new_resource(&mut self) -> Id {
		Id(self.resources.insert(Resource::new()) as u32)
	}

	pub fn term_interpretation(&self, term: uninterpreted::Term<V>) -> Option<Id>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::Literal: Eq + Hash,
	{
		match term {
			rdf_types::Term::Id(rdf_types::Id::Iri(iri)) => self.by_iri.get(&iri).copied(),
			rdf_types::Term::Id(rdf_types::Id::Blank(blank)) => self.by_blank.get(&blank).copied(),
			rdf_types::Term::Literal(literal) => self.by_literal.get(&literal).copied(),
		}
	}

	pub fn insert_term(&mut self, term: uninterpreted::Term<V>) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		match term {
			rdf_types::Term::Id(rdf_types::Id::Iri(iri)) => *self
				.by_iri
				.entry(iri)
				.or_insert_with(|| Id(self.resources.insert(Resource::from_iri(iri)) as u32)),
			rdf_types::Term::Id(rdf_types::Id::Blank(blank)) => *self
				.by_blank
				.entry(blank)
				.or_insert_with(|| Id(self.resources.insert(Resource::from_blank(blank)) as u32)),
			rdf_types::Term::Literal(literal) => {
				*self.by_literal.entry(literal).or_insert_with(|| {
					Id(self.resources.insert(Resource::from_literal(literal)) as u32)
				})
			}
		}
	}

	pub fn set_term_interpretation(&mut self, term: uninterpreted::Term<V>, id: Id)
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		match term {
			rdf_types::Term::Id(rdf_types::Id::Iri(iri)) => {
				assert!(self.by_iri.insert(iri, id).is_none());
				self.resources[id.index()].as_iri.insert(iri);
			}
			rdf_types::Term::Id(rdf_types::Id::Blank(blank)) => {
				assert!(self.by_blank.insert(blank, id).is_none());
				self.resources[id.index()].as_blank.insert(blank);
			}
			rdf_types::Term::Literal(literal) => {
				assert!(self.by_literal.insert(literal, id).is_none());
				self.resources[id.index()].as_literal.insert(literal);
			}
		}
	}

	pub fn quad_interpretation(
		&mut self,
		rdf_types::Quad(s, p, o, g): uninterpreted::Quad<V>,
	) -> Option<Quad>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::Literal: Eq + Hash,
	{
		Some(rdf_types::Quad(
			self.term_interpretation(s)?,
			self.term_interpretation(p)?,
			self.term_interpretation(o)?,
			match g {
				Some(g) => Some(self.term_interpretation(g)?),
				None => None,
			},
		))
	}

	pub fn insert_quad(&mut self, rdf_types::Quad(s, p, o, g): uninterpreted::Quad<V>) -> Quad
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		rdf_types::Quad(
			self.insert_term(s),
			self.insert_term(p),
			self.insert_term(o),
			g.map(|g| self.insert_term(g)),
		)
	}

	/// Merge the two given interpreted resources.
	///
	/// Returns the `Id` of the merged resource, followed by the `Id` of the
	/// removed resource and the removed resource literal instances.
	pub fn merge(&mut self, mut a: Id, mut b: Id) -> Result<(Id, Id), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		if b < a {
			std::mem::swap(&mut a, &mut b);
		}

		let resource = self.resources.remove(b.index());

		for id in resource.different_from {
			if id == a {
				return Err(Contradiction(a, b));
			} else {
				let different_resource = &mut self.resources[id.index()];
				different_resource.different_from.remove(&b);
				different_resource.different_from.insert(a);
			}
		}

		for iri in resource.as_iri {
			self.by_iri.insert(iri, a);
			self.resources[a.index()].as_iri.insert(iri);
		}

		for blank in resource.as_blank {
			self.by_blank.insert(blank, a);
			self.resources[a.index()].as_blank.insert(blank);
		}

		for literal in resource.as_literal {
			self.by_literal.insert(literal, a);
			self.resources[a.index()].as_literal.insert(literal);
		}

		Ok((a, b))
	}

	pub fn split(&mut self, a: Id, b: Id) -> Result<bool, Contradiction> {
		if a == b {
			Err(Contradiction(a, b))
		} else {
			self.resources[a.index()].different_from.insert(b);
			Ok(self.resources[b.index()].different_from.insert(a))
		}
	}
}

/// Iterator over the (uninterpreted) terms representing the given resource.
pub struct TermsOf<'a, V: Vocabulary> {
	as_iri: hashbrown::hash_set::Iter<'a, V::Iri>,
	as_blank: hashbrown::hash_set::Iter<'a, V::BlankId>,
	as_literal: hashbrown::hash_set::Iter<'a, V::Literal>,
}

impl<'a, V: Vocabulary> Iterator for TermsOf<'a, V>
where
	V::Iri: Copy,
	V::BlankId: Copy,
	V::Literal: Copy,
{
	type Item = uninterpreted::Term<V>;

	fn next(&mut self) -> Option<Self::Item> {
		self.as_iri
			.next()
			.map(|iri| uninterpreted::Term::<V>::Id(rdf_types::Id::Iri(*iri)))
			.or_else(|| {
				self.as_blank
					.next()
					.map(|blank| uninterpreted::Term::<V>::Id(rdf_types::Id::Blank(*blank)))
			})
			.or_else(|| {
				self.as_literal
					.next()
					.map(|literal| uninterpreted::Term::<V>::Literal(*literal))
			})
	}
}
