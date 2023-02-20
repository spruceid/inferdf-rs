use std::hash::Hash;

use derivative::Derivative;
use hashbrown::{HashMap, HashSet};
use slab::Slab;

use crate::{Id, Vocabulary, LiteralVocabulary, GlobalQuad, Quad, GlobalTerm, GlobalLiteral};

#[derive(Derivative)]
#[derivative(
	Debug(bound="V::StringLiteral: std::fmt::Debug"),
	Clone(bound="V::StringLiteral: Clone"),
	Copy(bound="V::StringLiteral: Copy"),
	PartialEq(bound="V::StringLiteral: PartialEq"),
	Eq(bound="V::StringLiteral: Eq"),
	PartialOrd(bound="V::StringLiteral: PartialOrd"),
	Ord(bound="V::StringLiteral: Ord"),
	Hash(bound="V::StringLiteral: Hash")
)]
pub struct LiteralValue<V: LiteralVocabulary> {
	lexical: V::StringLiteral,
	type_: Id
}

impl<V: LiteralVocabulary> LiteralValue<V> {
	pub fn new(
		lexical: V::StringLiteral,
		type_: Id
	) -> Self {
		Self {
			lexical,
			type_
		}
	}
}

/// RDF interpretation.
pub struct Interpretation<V: Vocabulary> {
	resources: Slab<Resource<V>>,
	by_iri: HashMap<V::Iri, Id>,
	by_blank: HashMap<V::BlankId, Id>
}

pub struct Resource<V: Vocabulary> {
	as_iri: HashSet<V::Iri>,
	as_blank: HashSet<V::BlankId>,
	as_literal: HashSet<LiteralValue<V>>,
	lexical_values: ResourceLiteralInstances<V>
}

impl<V: Vocabulary> Resource<V> {
	pub fn from_iri(iri: V::Iri) -> Self where V::Iri: Eq + Hash {
		Self {
			as_iri: std::iter::once(iri).collect(),
			as_blank: HashSet::new(),
			as_literal: HashSet::new(),
			lexical_values: ResourceLiteralInstances::<V>::new()
		}
	}

	pub fn from_blank(blank: V::BlankId) -> Self where V::BlankId: Eq + Hash {
		Self {
			as_iri: HashSet::new(),
			as_blank: std::iter::once(blank).collect(),
			as_literal: HashSet::new(),
			lexical_values: ResourceLiteralInstances::<V>::new()
		}
	}

	pub fn from_literal(value: V::StringLiteral, type_: Id) -> Self where V::StringLiteral: Eq + Hash {
		Self {
			as_iri: HashSet::new(),
			as_blank: HashSet::new(),
			as_literal: std::iter::once(LiteralValue::new(value, type_)).collect(),
			lexical_values: ResourceLiteralInstances::<V>::new()
		}
	}

	pub fn insert_lexical_value(&mut self, value: V::StringLiteral, id: Id) -> Option<Id> where V::StringLiteral: Eq + Hash {
		self.lexical_values.insert(value, id)
	}
}

pub type ResourceLiteralInstances<V> = HashMap<<V as LiteralVocabulary>::StringLiteral, Id>;

impl<V: Vocabulary> Interpretation<V> {
	pub fn get_mut(&mut self, id: Id) -> Option<&mut Resource<V>> {
		self.resources.get_mut(id.0)
	}

	pub fn insert_literal(&mut self, literal: GlobalLiteral<V>) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		let type_ = *self.by_iri.entry(literal.type_).or_insert_with(|| {
			Id(self.resources.insert(Resource::from_iri(literal.type_)))
		});

		match self.resources[type_.0].lexical_values.get(&literal.value) {
			Some(id) => *id,
			None => {
				let id = Id(self.resources.insert(Resource::from_literal(literal.value, type_)));
				self.resources[type_.0].lexical_values.insert(literal.value, id);
				id
			}
		}
	}

	pub fn insert_term(&mut self, term: GlobalTerm<V>) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		match term {
			rdf_types::Term::Iri(iri) => *self.by_iri.entry(iri).or_insert_with(|| {
				Id(self.resources.insert(Resource::from_iri(iri)))
			}),
			rdf_types::Term::Blank(blank) => *self.by_blank.entry(blank).or_insert_with(|| {
				Id(self.resources.insert(Resource::from_blank(blank)))
			}),
			rdf_types::Term::Literal(literal) => self.insert_literal(literal)
		}
	}

	pub fn insert_quad(&mut self, rdf_types::Quad(s, p, o, g): GlobalQuad<V>) -> Quad
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		rdf_types::Quad(
			self.insert_term(s),
			self.insert_term(p),
			self.insert_term(o),
			g.map(|g| self.insert_term(g))
		)
	}

	/// Merge the two given interpreted resources.
	/// 
	/// Returns the `Id` of the merged resource, followed by the `Id` of the
	/// removed resource and the removed resource literal instances.
	pub fn merge(&mut self, mut a: Id, mut b: Id) -> (Id, Id, ResourceLiteralInstances<V>)
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		if b < a {
			std::mem::swap(&mut a, &mut b);
		}

		let resource = self.resources.remove(b.0);

		for iri in resource.as_iri {
			self.by_iri.insert(iri, a);
			self.resources[a.0].as_iri.insert(iri);
		}

		for blank in resource.as_blank {
			self.by_blank.insert(blank, a);
			self.resources[a.0].as_blank.insert(blank);
		}

		for literal in resource.as_literal {
			self.resources[literal.type_.0].lexical_values.insert(literal.lexical, a);
		}

		(a, b, resource.lexical_values)
	}
}