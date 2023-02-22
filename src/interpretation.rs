use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use slab::Slab;

use crate::{Id, Vocabulary, LiteralVocabulary, SemiInterpretedQuad, Quad, SemiInterpretedTerm, SemiInterpretedLiteral, Triple, GlobalTerm, GlobalLiteral, GlobalTriple, GlobalTermExt};

pub type LiteralValue<V> = SemiInterpretedLiteral<V>;

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
	pub fn terms_of(&self, id: Id) -> TermsOf<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy
	{
		let r = &self.resources[id.0];
		
		TermsOf {
			interpretation: self,
			as_iri: r.as_iri.iter(),
			as_blank: r.as_blank.iter(),
			as_literal: r.as_literal.iter(),
			current_literal: None
		}
	}

	pub fn global_triple_of(&self, triple: Triple) -> Vec<GlobalTriple<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy
	{
		let mut result = Vec::new();

		for s in self.terms_of(triple.0) {
			for p in self.terms_of(triple.0) {
				for o in self.terms_of(triple.0) {
					result.push(GlobalTriple::new(s.clone(), p.clone(), o))
				}
			}
		}

		result
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut Resource<V>> {
		self.resources.get_mut(id.0)
	}

	/// Returns the interpretation of `literal` along with the interpretation of its type.
	pub fn literal_interpretation(&self, literal: SemiInterpretedLiteral<V>) -> Option<Id>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::StringLiteral: Eq + Hash
	{
		self.resources[literal.type_.0].lexical_values.get(&literal.value).copied()
	}

	pub fn insert_literal(&mut self, literal: SemiInterpretedLiteral<V>) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		match self.resources[literal.type_.0].lexical_values.get(&literal.value) {
			Some(id) => *id,
			None => {
				let id = Id(self.resources.insert(Resource::from_literal(literal.value, literal.type_)));
				self.resources[literal.type_.0].lexical_values.insert(literal.value, id);
				id
			}
		}
	}

	pub fn set_literal_interpretation(&mut self, literal: SemiInterpretedLiteral<V>, id: Id)
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		assert!(self.resources[literal.type_.0].lexical_values.insert(literal.value, id).is_none());
		self.resources[id.0].as_literal.insert(literal);
	}

	pub fn term_interpretation(&self, term: SemiInterpretedTerm<V>) -> Option<Id>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::StringLiteral: Eq + Hash
	{
		match term {
			rdf_types::Term::Iri(iri) => self.by_iri.get(&iri).copied(),
			rdf_types::Term::Blank(blank) => self.by_blank.get(&blank).copied(),
			rdf_types::Term::Literal(literal) => self.literal_interpretation(literal)
		}
	}

	pub fn insert_term(&mut self, term: SemiInterpretedTerm<V>) -> Id
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

	pub fn insert_term_with_dependencies(
		&mut self,
		sources: &[Interpretation<V>],
		term: SemiInterpretedTerm<V>,
	) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		// ...

		if sources.is_empty() {
			self.insert_term(term)
		} else {
			match self.term_interpretation(term) {
				Some(id) => id,
				None => {
					for source in sources {
						if let Some(source_id) = source.term_interpretation(term) {
							for source_term in source.terms_of(source_id) {
								if let Some(source_term) = source_term.try_interpret_literal_type_with(|t| self.term_interpretation(t)) {
									if let Some(id) = self.term_interpretation(source_term) {
										self.set_term_interpretation(term, id);
										return id
									}
								}
							}
						}
					}
	
					self.insert_term(term)
				}
			}
		}
	}

	pub fn set_term_interpretation(&mut self, term: SemiInterpretedTerm<V>, id: Id)
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		match term {
			rdf_types::Term::Iri(iri) => {
				assert!(self.by_iri.insert(iri, id).is_none());
				self.resources[id.0].as_iri.insert(iri);
			},
			rdf_types::Term::Blank(blank) => {
				assert!(self.by_blank.insert(blank, id).is_none());
				self.resources[id.0].as_blank.insert(blank);
			},
			rdf_types::Term::Literal(literal) => {
				self.set_literal_interpretation(literal, id)
			}
		}
	}

	pub fn quad_interpretation(&mut self, rdf_types::Quad(s, p, o, g): SemiInterpretedQuad<V>) -> Option<Quad>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::StringLiteral: Eq + Hash
	{
		Some(rdf_types::Quad(
			self.term_interpretation(s)?,
			self.term_interpretation(p)?,
			self.term_interpretation(o)?,
			match g {
				Some(g) => Some(self.term_interpretation(g)?),
				None => None
			}
		))
	}

	pub fn insert_quad(&mut self, rdf_types::Quad(s, p, o, g): SemiInterpretedQuad<V>) -> Quad
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

	pub fn insert_quad_with_dependencies(
		&mut self,
		sources: &[Interpretation<V>],
		rdf_types::Quad(s, p, o, g): SemiInterpretedQuad<V>
	) -> Quad
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash
	{
		rdf_types::Quad(
			self.insert_term_with_dependencies(sources, s),
			self.insert_term_with_dependencies(sources, p),
			self.insert_term_with_dependencies(sources, o),
			g.map(|g| self.insert_term_with_dependencies(sources, g))
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
			self.resources[literal.type_.0].lexical_values.insert(literal.value, a);
		}

		(a, b, resource.lexical_values)
	}
}

pub struct TermsOf<'a, V: Vocabulary> {
	interpretation: &'a Interpretation<V>,
	as_iri: hashbrown::hash_set::Iter<'a, V::Iri>,
	as_blank: hashbrown::hash_set::Iter<'a, V::BlankId>,
	as_literal: hashbrown::hash_set::Iter<'a, LiteralValue<V>>,
	current_literal: Option<(&'a LiteralValue<V>, Box<Self>)>
}

impl<'a, V: Vocabulary> Iterator for TermsOf<'a, V>
where
	V::Iri: Copy,
	V::BlankId: Copy,
	V::StringLiteral: Copy
{
	type Item = GlobalTerm<V>;

	fn next(&mut self) -> Option<Self::Item> {
		self.as_iri.next().map(|iri| GlobalTerm::Iri(*iri))
		.or_else(|| {
			self.as_blank.next().map(|blank| GlobalTerm::Blank(*blank))
		})
		.or_else(|| {
			let literal = loop {
				match &mut self.current_literal {
					Some((literal, type_terms)) => {
						match type_terms.next() {
							Some(type_) => break Some(GlobalLiteral::new(literal.value, type_)),
							None => self.current_literal = None
						}
					}
					None => {
						match self.as_literal.next() {
							Some(literal) => self.current_literal = Some((literal, Box::new(self.interpretation.terms_of(literal.type_)))),
							None => break None
						}
					}
				}
			};

			literal.map(GlobalTerm::Literal)
		})
	}
}