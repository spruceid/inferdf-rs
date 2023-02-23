use std::hash::Hash;

use hashbrown::{HashMap, HashSet};
use slab::Slab;

use crate::{
	GlobalLiteral, GlobalTerm, GlobalTermExt, GlobalTriple, Id, LiteralVocabulary, Quad,
	SemiInterpretedLiteral, SemiInterpretedQuad, SemiInterpretedTerm, Triple, Vocabulary,
};

pub type LiteralValue<V> = SemiInterpretedLiteral<V>;

/// RDF interpretation.
pub struct Interpretation<V: Vocabulary> {
	resources: Slab<Resource<V>>,
	by_iri: HashMap<V::Iri, Id>,
	by_blank: HashMap<V::BlankId, Id>,
}

pub struct Resource<V: Vocabulary> {
	as_iri: HashSet<V::Iri>,
	as_blank: HashSet<V::BlankId>,
	as_literal: HashSet<LiteralValue<V>>,
	lexical_values: ResourceLiteralInstances<V>,
	different_from: HashSet<Id>,
}

impl<V: Vocabulary> Resource<V> {
	pub fn from_iri(iri: V::Iri) -> Self
	where
		V::Iri: Eq + Hash,
	{
		Self {
			as_iri: std::iter::once(iri).collect(),
			as_blank: HashSet::new(),
			as_literal: HashSet::new(),
			lexical_values: ResourceLiteralInstances::<V>::new(),
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
			lexical_values: ResourceLiteralInstances::<V>::new(),
			different_from: HashSet::new(),
		}
	}

	pub fn from_literal(value: V::StringLiteral, type_: Id) -> Self
	where
		V::StringLiteral: Eq + Hash,
	{
		Self {
			as_iri: HashSet::new(),
			as_blank: HashSet::new(),
			as_literal: std::iter::once(LiteralValue::new(value, type_)).collect(),
			lexical_values: ResourceLiteralInstances::<V>::new(),
			different_from: HashSet::new(),
		}
	}

	pub fn insert_lexical_value(&mut self, value: V::StringLiteral, id: Id) -> Option<Id>
	where
		V::StringLiteral: Eq + Hash,
	{
		self.lexical_values.insert(value, id)
	}
}

pub type ResourceLiteralInstances<V> = HashMap<<V as LiteralVocabulary>::StringLiteral, Id>;

pub struct Contradiction(pub Id, pub Id);

impl<V: Vocabulary> Interpretation<V> {
	pub fn terms_of(&self, id: Id) -> TermsOf<V>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
	{
		let r = &self.resources[id.0];

		TermsOf {
			interpretation: self,
			as_iri: r.as_iri.iter(),
			as_blank: r.as_blank.iter(),
			as_literal: r.as_literal.iter(),
			current_literal: None,
		}
	}

	pub fn global_triple_of(&self, triple: Triple) -> Vec<GlobalTriple<V>>
	where
		V::Iri: Copy,
		V::BlankId: Copy,
		V::StringLiteral: Copy,
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

	pub fn get(&self, id: Id) -> Option<&Resource<V>> {
		self.resources.get(id.0)
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut Resource<V>> {
		self.resources.get_mut(id.0)
	}

	/// Returns the interpretation of `literal` along with the interpretation of its type.
	pub fn literal_interpretation(&self, literal: SemiInterpretedLiteral<V>) -> Option<Id>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::StringLiteral: Eq + Hash,
	{
		self.resources[literal.type_.0]
			.lexical_values
			.get(&literal.value)
			.copied()
	}

	pub fn insert_literal(&mut self, literal: SemiInterpretedLiteral<V>) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		match self.resources[literal.type_.0]
			.lexical_values
			.get(&literal.value)
		{
			Some(id) => *id,
			None => {
				let id = Id(self
					.resources
					.insert(Resource::from_literal(literal.value, literal.type_)));
				self.resources[literal.type_.0]
					.lexical_values
					.insert(literal.value, id);
				id
			}
		}
	}

	pub fn set_literal_interpretation(&mut self, literal: SemiInterpretedLiteral<V>, id: Id)
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		assert!(self.resources[literal.type_.0]
			.lexical_values
			.insert(literal.value, id)
			.is_none());
		self.resources[id.0].as_literal.insert(literal);
	}

	pub fn term_interpretation(&self, term: SemiInterpretedTerm<V>) -> Option<Id>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::StringLiteral: Eq + Hash,
	{
		match term {
			rdf_types::Term::Iri(iri) => self.by_iri.get(&iri).copied(),
			rdf_types::Term::Blank(blank) => self.by_blank.get(&blank).copied(),
			rdf_types::Term::Literal(literal) => self.literal_interpretation(literal),
		}
	}

	pub fn insert_term(&mut self, term: SemiInterpretedTerm<V>) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		match term {
			rdf_types::Term::Iri(iri) => *self
				.by_iri
				.entry(iri)
				.or_insert_with(|| Id(self.resources.insert(Resource::from_iri(iri)))),
			rdf_types::Term::Blank(blank) => *self
				.by_blank
				.entry(blank)
				.or_insert_with(|| Id(self.resources.insert(Resource::from_blank(blank)))),
			rdf_types::Term::Literal(literal) => self.insert_literal(literal),
		}
	}

	pub fn set_term_interpretation(&mut self, term: SemiInterpretedTerm<V>, id: Id)
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		match term {
			rdf_types::Term::Iri(iri) => {
				assert!(self.by_iri.insert(iri, id).is_none());
				self.resources[id.0].as_iri.insert(iri);
			}
			rdf_types::Term::Blank(blank) => {
				assert!(self.by_blank.insert(blank, id).is_none());
				self.resources[id.0].as_blank.insert(blank);
			}
			rdf_types::Term::Literal(literal) => self.set_literal_interpretation(literal, id),
		}
	}

	pub fn quad_interpretation(
		&mut self,
		rdf_types::Quad(s, p, o, g): SemiInterpretedQuad<V>,
	) -> Option<Quad>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::StringLiteral: Eq + Hash,
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

	pub fn insert_quad(&mut self, rdf_types::Quad(s, p, o, g): SemiInterpretedQuad<V>) -> Quad
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
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
	pub fn merge(
		&mut self,
		mut a: Id,
		mut b: Id,
	) -> Result<(Id, Id, ResourceLiteralInstances<V>), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		if b < a {
			std::mem::swap(&mut a, &mut b);
		}

		let resource = self.resources.remove(b.0);

		for id in resource.different_from {
			if id == a {
				return Err(Contradiction(a, b));
			} else {
				let different_resource = &mut self.resources[id.0];
				different_resource.different_from.remove(&b);
				different_resource.different_from.insert(a);
			}
		}

		for iri in resource.as_iri {
			self.by_iri.insert(iri, a);
			self.resources[a.0].as_iri.insert(iri);
		}

		for blank in resource.as_blank {
			self.by_blank.insert(blank, a);
			self.resources[a.0].as_blank.insert(blank);
		}

		for literal in resource.as_literal {
			self.resources[literal.type_.0]
				.lexical_values
				.insert(literal.value, a);
		}

		Ok((a, b, resource.lexical_values))
	}

	pub fn split(&mut self, a: Id, b: Id) -> Result<bool, Contradiction> {
		if a == b {
			Err(Contradiction(a, b))
		} else {
			self.resources[a.0].different_from.insert(b);
			Ok(self.resources[b.0].different_from.insert(a))
		}
	}
}

pub struct TermsOf<'a, V: Vocabulary> {
	interpretation: &'a Interpretation<V>,
	as_iri: hashbrown::hash_set::Iter<'a, V::Iri>,
	as_blank: hashbrown::hash_set::Iter<'a, V::BlankId>,
	as_literal: hashbrown::hash_set::Iter<'a, LiteralValue<V>>,
	current_literal: Option<(&'a LiteralValue<V>, Box<Self>)>,
}

impl<'a, V: Vocabulary> Iterator for TermsOf<'a, V>
where
	V::Iri: Copy,
	V::BlankId: Copy,
	V::StringLiteral: Copy,
{
	type Item = GlobalTerm<V>;

	fn next(&mut self) -> Option<Self::Item> {
		self.as_iri
			.next()
			.map(|iri| GlobalTerm::Iri(*iri))
			.or_else(|| self.as_blank.next().map(|blank| GlobalTerm::Blank(*blank)))
			.or_else(|| {
				let literal = loop {
					match &mut self.current_literal {
						Some((literal, type_terms)) => match type_terms.next() {
							Some(type_) => break Some(GlobalLiteral::new(literal.value, type_)),
							None => self.current_literal = None,
						},
						None => match self.as_literal.next() {
							Some(literal) => {
								self.current_literal = Some((
									literal,
									Box::new(self.interpretation.terms_of(literal.type_)),
								))
							}
							None => break None,
						},
					}
				};

				literal.map(GlobalTerm::Literal)
			})
	}
}

/// Interpretation dependency.
pub trait Dependency<V: Vocabulary> {
	fn interpretation(&self) -> &Interpretation<V>;
}

pub struct CompositeInterpretation<V: Vocabulary> {
	/// Final interpretation.
	interpretation: Interpretation<V>,

	/// Interfaces with dependency interpretations.
	interfaces: HashMap<usize, Interface>,
}

impl<V: Vocabulary> CompositeInterpretation<V> {
	pub fn get_mut(&mut self, id: Id) -> Option<&mut Resource<V>> {
		self.interpretation.get_mut(id)
	}

	pub fn term_interpretation(&self, term: SemiInterpretedTerm<V>) -> Option<Id>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::StringLiteral: Eq + Hash,
	{
		self.interpretation.term_interpretation(term)
	}

	pub fn import_resource<D: Dependency<V>>(
		&mut self,
		dependencies: &HashMap<usize, D>,
		d: Option<usize>,
		id: Id,
	) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		match d {
			Some(d) => {
				let interface = self.interfaces.entry(d).or_default();
				match interface.target.get(&id) {
					Some(imported_id) => *imported_id,
					None => {
						let dependency = &dependencies[&d];
						match dependency.interpretation().terms_of(id).next() {
							Some(term) => {
								let term = term.interpret_literal_type_with(|t| {
									self.insert_term(dependencies, t)
								});
								self.insert_term(dependencies, term)
							}
							None => {
								todo!()
							}
						}
					}
				}
			}
			None => id,
		}
	}

	pub fn import_triple<D: Dependency<V>>(
		&mut self,
		dependencies: &HashMap<usize, D>,
		d: Option<usize>,
		triple: Triple,
	) -> Triple
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		Triple::new(
			self.import_resource(dependencies, d, triple.0),
			self.import_resource(dependencies, d, triple.1),
			self.import_resource(dependencies, d, triple.2),
		)
	}

	pub fn insert_term<D: Dependency<V>>(
		&mut self,
		dependencies: &HashMap<usize, D>,
		term: SemiInterpretedTerm<V>,
	) -> Id
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		match self.term_interpretation(term) {
			Some(id) => id,
			None => {
				let id = self.interpretation.insert_term(term);

				for (&d, dependency) in dependencies {
					if let Some(dependency_id) =
						dependency.interpretation().term_interpretation(term)
					{
						let i = self.interfaces.entry(d).or_default();
						i.source.insert(id, vec![dependency_id]);
						i.target.insert(dependency_id, id);

						// import interpretation inequality constraints.
						for other_dependency_id in &dependency
							.interpretation()
							.get(dependency_id)
							.unwrap()
							.different_from
						{
							if let Some(&other_id) = i.target.get(other_dependency_id) {
								self.interpretation
									.get_mut(id)
									.unwrap()
									.different_from
									.insert(other_id);
								self.interpretation
									.get_mut(other_id)
									.unwrap()
									.different_from
									.insert(id);
							}
						}

						// import all known representations.
						for other_term in dependency.interpretation().terms_of(dependency_id) {
							let other_term = other_term
								.interpret_literal_type_with(|t| self.insert_term(dependencies, t));
							self.interpretation.set_term_interpretation(other_term, id);
						}
					}
				}

				id
			}
		}
	}

	pub fn insert_quad<D: Dependency<V>>(
		&mut self,
		dependencies: &HashMap<usize, D>,
		rdf_types::Quad(s, p, o, g): SemiInterpretedQuad<V>,
	) -> Quad
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		rdf_types::Quad(
			self.insert_term(dependencies, s),
			self.insert_term(dependencies, p),
			self.insert_term(dependencies, o),
			g.map(|g| self.insert_term(dependencies, g)),
		)
	}

	/// Merge the two given interpreted resources.
	///
	/// Returns the `Id` of the merged resource, followed by the `Id` of the
	/// removed resource and the removed resource literal instances.
	pub fn merge(
		&mut self,
		a: Id,
		b: Id,
	) -> Result<(Id, Id, ResourceLiteralInstances<V>), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::StringLiteral: Copy + Eq + Hash,
	{
		let (a, b, removed_lexical_values) = self.interpretation.merge(a, b)?;

		// merge in interfaces.
		for interface in self.interfaces.values_mut() {
			let more_dependency_ids = interface.source.remove(&b).unwrap();
			for &dependency_id in &more_dependency_ids {
				interface.target.insert(dependency_id, a);
			}

			let dependency_ids = interface.source.get_mut(&a).unwrap();
			dependency_ids.extend(more_dependency_ids);
		}

		Ok((a, b, removed_lexical_values))
	}

	pub fn split(&mut self, a: Id, b: Id) -> Result<bool, Contradiction> {
		self.interpretation.split(a, b)
	}

	pub fn dependency_ids(&self, d: usize, id: Id) -> DependencyIds {
		match self.interfaces.get(&d) {
			Some(i) => match i.source.get(&id) {
				Some(ids) => DependencyIds::Some(ids.iter()),
				None => DependencyIds::None,
			},
			None => DependencyIds::None,
		}
	}

	pub fn dependency_triples(&self, d: usize, triple: Triple) -> DependencyTriples {
		DependencyTriples {
			subjects: self.dependency_ids(d, triple.0),
			predicates: self.dependency_ids(d, triple.1),
			objects: self.dependency_ids(d, triple.2),
			current: None,
		}
	}
}

/// Describes how a shared resources are interpreted in a dependency
/// interpretation.
#[derive(Default)]
pub struct Interface {
	/// From composite id to dependency ids.
	source: HashMap<Id, Vec<Id>>,

	/// From dependency id to composite id.
	target: HashMap<Id, Id>,
}

#[derive(Debug, Clone)]
pub enum DependencyIds<'a> {
	Some(std::slice::Iter<'a, Id>),
	None,
}

impl<'a> Iterator for DependencyIds<'a> {
	type Item = Id;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Some(iter) => iter.next().copied(),
			Self::None => None,
		}
	}
}

pub struct DependencyTriples<'a> {
	subjects: DependencyIds<'a>,
	predicates: DependencyIds<'a>,
	objects: DependencyIds<'a>,
	current: Option<DependencyPO<'a>>,
}

impl<'a> Iterator for DependencyTriples<'a> {
	type Item = Triple;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(current) => match current.next() {
					Some(t) => break Some(t),
					None => self.current = None,
				},
				None => match self.subjects.next() {
					Some(s) => {
						self.current = Some(DependencyPO {
							subject: s,
							predicates: self.predicates.clone(),
							objects: self.objects.clone(),
							current: None,
						})
					}
					None => break None,
				},
			}
		}
	}
}

struct DependencyPO<'a> {
	subject: Id,
	predicates: DependencyIds<'a>,
	objects: DependencyIds<'a>,
	current: Option<DependencyO<'a>>,
}

impl<'a> Iterator for DependencyPO<'a> {
	type Item = Triple;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(current) => match current.next() {
					Some(t) => break Some(t),
					None => self.current = None,
				},
				None => match self.predicates.next() {
					Some(p) => {
						self.current = Some(DependencyO {
							subject: self.subject,
							predicate: p,
							objects: self.objects.clone(),
						})
					}
					None => break None,
				},
			}
		}
	}
}

struct DependencyO<'a> {
	subject: Id,
	predicate: Id,
	objects: DependencyIds<'a>,
}

impl<'a> Iterator for DependencyO<'a> {
	type Item = Triple;

	fn next(&mut self) -> Option<Self::Item> {
		self.objects
			.next()
			.map(|o| rdf_types::Triple(self.subject, self.predicate, o))
	}
}
