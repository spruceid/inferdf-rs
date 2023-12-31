use std::hash::Hash;

use derivative::Derivative;
use hashbrown::HashMap;
use rdf_types::Vocabulary;

use crate::{
	pattern::{self, IdOrVar, IdOrVarIter},
	uninterpreted, Id, IteratorWith, Module, Pattern, Quad, Triple,
};

use super::{
	local, Contradiction, Interpretation as InterpretationRef, InterpretationMut,
	Resource as AnyResource,
};

pub use local::Resource;

/// Composite interpretation dependencies.
pub trait Dependencies<V: Vocabulary> {
	type Error;
	type Dependency: Module<V, Error = Self::Error>;
	type Iter<'a>: Iterator<Item = (usize, &'a Self::Dependency)>
	where
		Self: 'a,
		Self::Dependency: 'a;

	fn get(&self, i: usize) -> Option<&Self::Dependency>;

	fn iter(&self) -> Self::Iter<'_>;
}

/// Composite interpretation.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Interpretation<V: Vocabulary> {
	/// Final interpretation.
	interpretation: local::Interpretation<V>,

	/// Interfaces with dependency interpretations.
	interfaces: HashMap<usize, Interface>,
}

impl<V: Vocabulary> Interpretation<V> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn local_interpretation(&self) -> &local::Interpretation<V> {
		&self.interpretation
	}

	pub fn interface(&self, d: usize) -> Option<&Interface> {
		self.interfaces.get(&d)
	}

	pub fn get_mut(&mut self, id: Id) -> Option<&mut Resource<V>> {
		self.interpretation.get_mut(id)
	}

	pub fn new_resource(&mut self) -> Id {
		self.interpretation.new_resource()
	}

	pub fn term_interpretation(&self, term: uninterpreted::Term<V>) -> Option<Id>
	where
		V::Iri: Eq + Hash,
		V::BlankId: Eq + Hash,
		V::Literal: Eq + Hash,
	{
		self.interpretation.term_interpretation(term)
	}

	/// Import a resource in the composite interpretation.
	pub fn import_resource<D: Dependencies<V>>(
		&mut self,
		vocabulary: &mut V,
		dependencies: &D,
		d: Option<usize>,
		id: Id,
	) -> Result<Id, D::Error>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		match d {
			Some(d) => {
				let interface = self.interfaces.entry(d).or_default();
				match interface.target.get(&id) {
					Some(imported_id) => Ok(*imported_id),
					None => {
						let dependency = dependencies.get(d).unwrap();
						match dependency
							.interpretation()
							.terms_of(id)?
							.next_with(vocabulary)
						{
							Some(Err(e)) => Err(e),
							Some(Ok(term)) => self.insert_term(vocabulary, dependencies, term),
							None => {
								todo!()
							}
						}
					}
				}
			}
			None => Ok(id),
		}
	}

	pub fn import_triple<D: Dependencies<V>>(
		&mut self,
		vocabulary: &mut V,
		dependencies: &D,
		d: Option<usize>,
		triple: Triple,
	) -> Result<Triple, D::Error>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		Ok(Triple::new(
			self.import_resource(vocabulary, dependencies, d, triple.0)?,
			self.import_resource(vocabulary, dependencies, d, triple.1)?,
			self.import_resource(vocabulary, dependencies, d, triple.2)?,
		))
	}

	pub fn insert_term<D: Dependencies<V>>(
		&mut self,
		vocabulary: &mut V,
		dependencies: &D,
		term: uninterpreted::Term<V>,
	) -> Result<Id, D::Error>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		match self.term_interpretation(term) {
			Some(id) => Ok(id),
			None => {
				let id = self.interpretation.insert_term(term);

				for (d, dependency) in dependencies.iter() {
					if let Some(dependency_id) = dependency
						.interpretation()
						.term_interpretation(vocabulary, term)?
					{
						let i = self.interfaces.entry(d).or_default();
						i.source.insert(id, vec![dependency_id]);
						i.target.insert(dependency_id, id);

						// import interpretation inequality constraints.
						for other_dependency_id in dependency
							.interpretation()
							.get(dependency_id)?
							.unwrap()
							.different_from()
							.iter_with(vocabulary)
						{
							let other_dependency_id = other_dependency_id?;
							if let Some(&other_id) = i.target.get(&other_dependency_id) {
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
						let mut terms = dependency.interpretation().terms_of(dependency_id)?;
						while let Some(other_term) = terms.next_with(vocabulary) {
							self.interpretation.set_term_interpretation(other_term?, id);
						}
					}
				}

				Ok(id)
			}
		}
	}

	pub fn insert_quad<D: Dependencies<V>>(
		&mut self,
		vocabulary: &mut V,
		dependencies: &D,
		rdf_types::Quad(s, p, o, g): uninterpreted::Quad<V>,
	) -> Result<Quad, D::Error>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		Ok(rdf_types::Quad(
			self.insert_term(vocabulary, dependencies, s)?,
			self.insert_term(vocabulary, dependencies, p)?,
			self.insert_term(vocabulary, dependencies, o)?,
			g.map(|g| self.insert_term(vocabulary, dependencies, g))
				.transpose()?,
		))
	}

	/// Merge the two given interpreted resources.
	///
	/// Returns the `Id` of the merged resource, followed by the `Id` of the
	/// removed resource.
	pub fn merge(&mut self, a: Id, b: Id) -> Result<(Id, Id), Contradiction>
	where
		V::Iri: Copy + Eq + Hash,
		V::BlankId: Copy + Eq + Hash,
		V::Literal: Copy + Eq + Hash,
	{
		if a == b {
			Ok((a, b))
		} else {
			let (a, b) = self.interpretation.merge(a, b)?;

			// merge in interfaces.
			for interface in self.interfaces.values_mut() {
				let more_dependency_ids = interface.source.remove(&b).unwrap();
				for &dependency_id in &more_dependency_ids {
					interface.target.insert(dependency_id, a);
				}

				let dependency_ids = interface.source.get_mut(&a).unwrap();
				dependency_ids.extend(more_dependency_ids);
			}

			Ok((a, b))
		}
	}

	pub fn split(&mut self, a: Id, b: Id) -> Result<bool, Contradiction> {
		self.interpretation.split(a, b)
	}

	/// Converts a global identifier `id` into identifiers local to the
	/// dependency `d`.
	pub fn dependency_ids(&self, d: usize, id: Id) -> DependencyIds {
		match self.interfaces.get(&d) {
			Some(i) => match i.source.get(&id) {
				Some(ids) => DependencyIds::Some(ids.iter()),
				None => DependencyIds::None,
			},
			None => DependencyIds::None,
		}
	}

	/// Converts an identifier local to the dependency `d` to a global
	/// identifier, if any.
	///
	/// Returns `Some(global_id)` if a global identifier exists, or `None`
	/// if the corresponding resource has not been imported from the dependency.
	pub fn id_from_dependency(&self, d: usize, id: Id) -> Option<Id> {
		self.interfaces
			.get(&d)
			.and_then(|i| i.target.get(&id).copied())
	}

	pub fn dependency_triples(&self, d: usize, triple: Triple) -> DependencyTriples {
		DependencyTriples {
			subjects: self.dependency_ids(d, triple.0),
			predicates: self.dependency_ids(d, triple.1),
			objects: self.dependency_ids(d, triple.2),
			current: None,
		}
	}

	pub fn dependency_patterns(&self, d: usize, pattern: Pattern) -> DependencyPatterns {
		DependencyPatterns {
			subjects: pattern
				.0
				.map(|id| self.dependency_ids(d, id))
				.into_wrapping_iter(),
			predicates: pattern
				.1
				.map(|id| self.dependency_ids(d, id))
				.into_wrapping_iter(),
			objects: pattern
				.2
				.map(|id| self.dependency_ids(d, id))
				.into_wrapping_iter(),
			current: None,
		}
	}

	pub fn dependency_canonical_patterns(
		&self,
		d: usize,
		pattern: pattern::Canonical,
	) -> DependencyCanonicalPatterns {
		DependencyCanonicalPatterns(self.dependency_patterns(d, pattern.into()))
	}

	pub fn with_dependencies_mut<'a, D>(
		&'a mut self,
		dependencies: &'a D,
	) -> WithDependenciesMut<'a, V, D> {
		WithDependenciesMut {
			interpretation: self,
			dependencies,
		}
	}
}

pub struct WithDependenciesMut<'a, V: Vocabulary, D> {
	interpretation: &'a mut Interpretation<V>,
	dependencies: &'a D,
}

impl<'a, V: Vocabulary, D: Dependencies<V>> InterpretationMut<'a, V>
	for WithDependenciesMut<'a, V, D>
where
	V::Iri: Copy + Eq + Hash,
	V::BlankId: Copy + Eq + Hash,
	V::Literal: Copy + Eq + Hash,
{
	type Error = D::Error;

	fn insert_term(
		&mut self,
		vocabulary: &mut V,
		term: uninterpreted::Term<V>,
	) -> Result<Id, Self::Error> {
		self.interpretation
			.insert_term(vocabulary, self.dependencies, term)
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

impl Interface {
	/// Converts a global identifier `id` into identifiers local to the
	/// dependency.
	pub fn dependency_ids(&self, id: Id) -> DependencyIds {
		match self.source.get(&id) {
			Some(ids) => DependencyIds::Some(ids.iter()),
			None => DependencyIds::None,
		}
	}

	pub fn dependency_triples(&self, triple: Triple) -> DependencyTriples {
		DependencyTriples {
			subjects: self.dependency_ids(triple.0),
			predicates: self.dependency_ids(triple.1),
			objects: self.dependency_ids(triple.2),
			current: None,
		}
	}

	pub fn dependency_patterns(&self, pattern: Pattern) -> DependencyPatterns {
		DependencyPatterns {
			subjects: pattern
				.0
				.map(|id| self.dependency_ids(id))
				.into_wrapping_iter(),
			predicates: pattern
				.1
				.map(|id| self.dependency_ids(id))
				.into_wrapping_iter(),
			objects: pattern
				.2
				.map(|id| self.dependency_ids(id))
				.into_wrapping_iter(),
			current: None,
		}
	}

	pub fn dependency_canonical_patterns(
		&self,
		pattern: pattern::Canonical,
	) -> DependencyCanonicalPatterns {
		DependencyCanonicalPatterns(self.dependency_patterns(pattern.into()))
	}

	/// Converts an identifier local to the dependency to a global
	/// identifier, if any.
	///
	/// Returns `Some(global_id)` if a global identifier exists, or `None`
	/// if the corresponding resource has not been imported from the dependency.
	pub fn id_from_dependency(&self, id: Id) -> Option<Id> {
		self.target.get(&id).copied()
	}

	pub fn quad_from_dependency(&self, quad: Quad) -> Option<Quad> {
		let g = match quad.3 {
			Some(g) => Some(self.id_from_dependency(g)?),
			None => None,
		};

		Some(rdf_types::Quad(
			self.id_from_dependency(quad.0)?,
			self.id_from_dependency(quad.1)?,
			self.id_from_dependency(quad.2)?,
			g,
		))
	}
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
	current: Option<DependencyTriplesPO<'a>>,
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
						self.current = Some(DependencyTriplesPO {
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

struct DependencyTriplesPO<'a> {
	subject: Id,
	predicates: DependencyIds<'a>,
	objects: DependencyIds<'a>,
	current: Option<DependencyTriplesO<'a>>,
}

impl<'a> Iterator for DependencyTriplesPO<'a> {
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
						self.current = Some(DependencyTriplesO {
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

struct DependencyTriplesO<'a> {
	subject: Id,
	predicate: Id,
	objects: DependencyIds<'a>,
}

impl<'a> Iterator for DependencyTriplesO<'a> {
	type Item = Triple;

	fn next(&mut self) -> Option<Self::Item> {
		self.objects
			.next()
			.map(|o| rdf_types::Triple(self.subject, self.predicate, o))
	}
}

pub struct DependencyCanonicalPatterns<'a>(DependencyPatterns<'a>);

impl<'a> Iterator for DependencyCanonicalPatterns<'a> {
	type Item = pattern::Canonical;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(From::from)
	}
}

pub struct DependencyPatterns<'a> {
	subjects: IdOrVarIter<DependencyIds<'a>>,
	predicates: IdOrVarIter<DependencyIds<'a>>,
	objects: IdOrVarIter<DependencyIds<'a>>,
	current: Option<DependencyPatternsPO<'a>>,
}

impl<'a> Iterator for DependencyPatterns<'a> {
	type Item = Pattern;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(current) => match current.next() {
					Some(t) => break Some(t),
					None => self.current = None,
				},
				None => match self.subjects.next() {
					Some(s) => {
						self.current = Some(DependencyPatternsPO {
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

struct DependencyPatternsPO<'a> {
	subject: IdOrVar,
	predicates: IdOrVarIter<DependencyIds<'a>>,
	objects: IdOrVarIter<DependencyIds<'a>>,
	current: Option<DependencyPatternsO<'a>>,
}

impl<'a> Iterator for DependencyPatternsPO<'a> {
	type Item = Pattern;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(current) => match current.next() {
					Some(t) => break Some(t),
					None => self.current = None,
				},
				None => match self.predicates.next() {
					Some(p) => {
						self.current = Some(DependencyPatternsO {
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

struct DependencyPatternsO<'a> {
	subject: IdOrVar,
	predicate: IdOrVar,
	objects: IdOrVarIter<DependencyIds<'a>>,
}

impl<'a> Iterator for DependencyPatternsO<'a> {
	type Item = Pattern;

	fn next(&mut self) -> Option<Self::Item> {
		self.objects
			.next()
			.map(|o| rdf_types::Triple(self.subject, self.predicate, o))
	}
}
