use std::{hash::Hash, ops::Deref};

use derivative::Derivative;
use educe::Educe;
use hashbrown::{HashMap, HashSet};
use rdf_types::{IriVocabulary, LiteralVocabulary, Vocabulary};

mod reservable_slab;
use reservable_slab::ReservableSlab;

mod reservation;
pub use reservation::{CompletedReservation, Reservation};

use crate::{class::classification, uninterpreted, Id, IteratorWith, Quad, Triple};

use super::{Contradiction, InterpretationMut};

/// RDF interpretation.
#[derive(Educe)]
#[educe(Default)]
pub struct Interpretation<V: Vocabulary> {
	resources: ReservableSlab<Resource<V>>,
	by_iri: HashMap<V::Iri, Id>,
	by_blank: HashMap<V::BlankId, Id>,
	by_literal: HashMap<V::Literal, Id>,
}

impl<V: Vocabulary> Interpretation<V> {
	pub fn new() -> Self {
		Self::default()
	}
}

impl<'a, V: Vocabulary> crate::Interpretation<'a, V> for &'a Interpretation<V>
where
	V::Iri: Clone + Eq + Hash,
	V::Literal: Clone + Eq + Hash,
{
	type Error = std::convert::Infallible;

	type Resource = &'a Resource<V>;

	type Iris = Iris<'a, V>;
	type Literals = Literals<'a, V>;

	type Resources = Resources<'a, V>;

	fn get(&self, id: Id) -> Result<Option<Self::Resource>, Self::Error> {
		Ok(self.resources.get(id.0 as usize))
	}

	fn resources(&self) -> Result<Self::Resources, Self::Error> {
		Ok(Resources(self.resources.iter()))
	}

	fn iri_interpretation(
		&self,
		_vocabulary: &mut V,
		iri: <V>::Iri,
	) -> Result<Option<Id>, Self::Error> {
		Ok(self.by_iri.get(&iri).copied())
	}

	fn iris(&self) -> Result<Self::Iris, Self::Error> {
		Ok(Iris(self.by_iri.iter()))
	}

	fn literal_interpretation(
		&self,
		_vocabulary: &mut V,
		literal: <V>::Literal,
	) -> Result<Option<Id>, Self::Error> {
		Ok(self.by_literal.get(&literal).copied())
	}

	fn literals(&self) -> Result<Self::Literals, Self::Error> {
		Ok(Literals(self.by_literal.iter()))
	}
}

pub struct Iris<'a, V: IriVocabulary>(hashbrown::hash_map::Iter<'a, V::Iri, Id>);

impl<'a, V: IriVocabulary> Iterator for Iris<'a, V>
where
	V::Iri: Clone,
{
	type Item = Result<(V::Iri, Id), std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(i, id)| Ok((i.clone(), *id)))
	}
}

impl<'a, V: IriVocabulary> IteratorWith<V> for Iris<'a, V>
where
	V::Iri: Clone,
{
	type Item = Result<(V::Iri, Id), std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct Literals<'a, V: LiteralVocabulary>(hashbrown::hash_map::Iter<'a, V::Literal, Id>);

impl<'a, V: LiteralVocabulary> Iterator for Literals<'a, V>
where
	V::Literal: Clone,
{
	type Item = Result<(V::Literal, Id), std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(i, id)| Ok((i.clone(), *id)))
	}
}

impl<'a, V: LiteralVocabulary> IteratorWith<V> for Literals<'a, V>
where
	V::Literal: Clone,
{
	type Item = Result<(V::Literal, Id), std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct Resources<'a, V: Vocabulary>(reservable_slab::Iter<'a, Resource<V>>);

impl<'a, V: Vocabulary> Iterator for Resources<'a, V> {
	type Item = Result<(Id, &'a Resource<V>), std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|(id, r)| Ok((Id(id as u32), r)))
	}
}

impl<'a, V: Vocabulary> IteratorWith<V> for Resources<'a, V> {
	type Item = Result<(Id, &'a Resource<V>), std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

impl<'a, V: Vocabulary> InterpretationMut<'a, V> for Interpretation<V>
where
	V::Iri: Copy + Eq + Hash,
	V::BlankId: Copy + Eq + Hash,
	V::Literal: Copy + Eq + Hash,
{
	type Error = std::convert::Infallible;

	fn insert_term(
		&mut self,
		_vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Id, Self::Error> {
		Ok(self.insert_term(term))
	}
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Resource<V: Vocabulary> {
	pub as_iri: HashSet<V::Iri>,
	pub as_blank: HashSet<V::BlankId>,
	pub as_literal: HashSet<V::Literal>,
	pub different_from: HashSet<Id>,
}

impl<V: Vocabulary> Resource<V> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_term(&mut self, term: uninterpreted::Term<V>)
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::Literal: Eq + Hash,
	{
		match term {
			rdf_types::Term::Id(rdf_types::Id::Iri(iri)) => {
				self.as_iri.insert(iri);
			}
			rdf_types::Term::Id(rdf_types::Id::Blank(blank_id)) => {
				self.as_blank.insert(blank_id);
			}
			rdf_types::Term::Literal(lit) => {
				self.as_literal.insert(lit);
			}
		}
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

	pub fn is_anonymous(&self) -> bool {
		self.as_iri.is_empty() && self.as_literal.is_empty()
	}
}

impl<'a, V: Vocabulary> crate::interpretation::Resource<'a, V> for &'a Resource<V>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Error = std::convert::Infallible;

	type Iris = ResourceIris<'a, V>;
	type Literals = ResourceLiterals<'a, V>;
	type DifferentFrom = ResourceDifferentFrom<'a>;

	fn as_iri(&self) -> Self::Iris {
		ResourceIris(self.as_iri.iter())
	}

	fn as_literal(&self) -> Self::Literals {
		ResourceLiterals(self.as_literal.iter())
	}

	fn different_from(&self) -> Self::DifferentFrom {
		ResourceDifferentFrom(self.different_from.iter())
	}
}

pub struct ResourceIris<'a, V: IriVocabulary>(hashbrown::hash_set::Iter<'a, V::Iri>);

impl<'a, V: IriVocabulary> Iterator for ResourceIris<'a, V>
where
	V::Iri: Clone,
{
	type Item = Result<V::Iri, std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().cloned().map(Ok)
	}
}

impl<'a, V: IriVocabulary> IteratorWith<V> for ResourceIris<'a, V>
where
	V::Iri: Clone,
{
	type Item = Result<V::Iri, std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct ResourceLiterals<'a, V: LiteralVocabulary>(hashbrown::hash_set::Iter<'a, V::Literal>);

impl<'a, V: LiteralVocabulary> Iterator for ResourceLiterals<'a, V>
where
	V::Literal: Clone,
{
	type Item = Result<V::Literal, std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().cloned().map(Ok)
	}
}

impl<'a, V: LiteralVocabulary> IteratorWith<V> for ResourceLiterals<'a, V>
where
	V::Literal: Clone,
{
	type Item = Result<V::Literal, std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub struct ResourceDifferentFrom<'a>(hashbrown::hash_set::Iter<'a, Id>);

impl<'a> Iterator for ResourceDifferentFrom<'a> {
	type Item = Result<Id, std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().cloned().map(Ok)
	}
}

impl<'a, V> IteratorWith<V> for ResourceDifferentFrom<'a> {
	type Item = Result<Id, std::convert::Infallible>;

	fn next_with(&mut self, _vocabulary: &mut V) -> Option<Self::Item> {
		self.next()
	}
}

pub type ResourceLiteralInstances<V> = HashMap<<V as LiteralVocabulary>::Literal, Id>;

impl<V: Vocabulary> Interpretation<V> {
	pub fn with_classification(
		self,
		classification: classification::Local,
	) -> WithClassification<V> {
		WithClassification::new(self, classification)
	}

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

	pub fn uninterpreted_triples_of(&self, triple: Triple) -> Vec<uninterpreted::Triple<V>>
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

	pub fn len(&self) -> u32 {
		self.resources.len() as u32
	}

	pub fn is_empty(&self) -> bool {
		self.resources.is_empty()
	}

	pub fn get(&self, id: Id) -> Option<&Resource<V>> {
		self.resources.get(id.index())
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut Resource<V>> {
		self.resources.get_mut(id.index())
	}

	/// Iterate over the resource, ordered by id.
	pub fn iter(&self) -> Iter<V> {
		Iter {
			inner: self.resources.iter(),
		}
	}

	pub fn new_resource(&mut self) -> Id {
		Id(self.resources.insert(Resource::new()) as u32)
	}

	pub fn terms_by_iri(&self) -> &HashMap<V::Iri, Id> {
		&self.by_iri
	}

	pub fn terms_by_literal(&self) -> &HashMap<V::Literal, Id> {
		&self.by_literal
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
		V::Iri: Clone + Eq + Hash,
		V::BlankId: Clone + Eq + Hash,
		V::Literal: Clone + Eq + Hash,
	{
		match term {
			rdf_types::Term::Id(rdf_types::Id::Iri(iri)) => {
				*self.by_iri.entry(iri).or_insert_with_key(|iri| {
					Id(self.resources.insert(Resource::from_iri(iri.clone())) as u32)
				})
			}
			rdf_types::Term::Id(rdf_types::Id::Blank(blank)) => {
				*self.by_blank.entry(blank).or_insert_with_key(|blank| {
					Id(self.resources.insert(Resource::from_blank(blank.clone())) as u32)
				})
			}
			rdf_types::Term::Literal(literal) => *self
				.by_literal
				.entry(literal)
				.or_insert_with_key(|literal| {
					Id(self
						.resources
						.insert(Resource::from_literal(literal.clone())) as u32)
				}),
		}
	}

	pub fn set_term_interpretation(&mut self, term: uninterpreted::Term<V>, id: Id)
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::Literal: Eq + Hash,
	{
		match term {
			rdf_types::Term::Id(rdf_types::Id::Iri(iri)) => {
				// assert!(self.by_iri.insert(iri, id).is_none());
				self.resources[id.index()].as_iri.insert(iri);
			}
			rdf_types::Term::Id(rdf_types::Id::Blank(blank)) => {
				// assert!(self.by_blank.insert(blank, id).is_none());
				self.resources[id.index()].as_blank.insert(blank);
			}
			rdf_types::Term::Literal(literal) => {
				// assert!(self.by_literal.insert(literal, id).is_none());
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
		if a == b {
			return Ok((a, b));
		}

		if b < a {
			std::mem::swap(&mut a, &mut b);
		}

		let resource = self.resources.remove(b.index()).unwrap();

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

	pub fn begin_reservation(&self) -> Reservation<V> {
		Reservation::new(
			// self,
			self.resources.begin_reservation(),
		)
	}
}

pub struct Iter<'a, V: Vocabulary> {
	inner: reservable_slab::Iter<'a, Resource<V>>,
}

impl<'a, V: Vocabulary> Iterator for Iter<'a, V> {
	type Item = (Id, &'a Resource<V>);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(|(i, r)| (Id(i as u32), r))
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

pub struct WithClassification<V: Vocabulary> {
	interpretation: Interpretation<V>,
	classification: classification::Local,
}

impl<V: Vocabulary> WithClassification<V> {
	pub fn new(interpretation: Interpretation<V>, classification: classification::Local) -> Self {
		Self {
			interpretation,
			classification,
		}
	}

	pub fn classification(&self) -> &classification::Local {
		&self.classification
	}
}

impl<V: Vocabulary> Deref for WithClassification<V> {
	type Target = Interpretation<V>;

	fn deref(&self) -> &Self::Target {
		&self.interpretation
	}
}
