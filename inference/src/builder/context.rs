use locspan::Meta;
use rdf_types::Vocabulary;
use std::hash::Hash;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::{
		self,
		composite::{DependencyCanonicalPatterns, Interface},
	},
	module::{
		sub_module::{self, ResourceGenerator},
		SubModule,
	},
	pattern, Fact, FailibleIteratorWith, Id, Interpretation, IteratorWith, Module, Sign, Signed,
};

use crate::semantics::{self, ContextReservation};

// use super::{Data, DependenciesIter};

pub struct BuilderContext<'a, V: Vocabulary, D> {
	dependency: &'a SubModule<V, D>,
	interpretation: &'a mut interpretation::Local<V>,
	// interpretation_reservation: RefCell<interpretation::local::Reservation<'a, V>>,
	dataset: &'a dataset::LocalDataset,
}

impl<'a, V: Vocabulary, D: Module<V>> BuilderContext<'a, V, D> {
	pub fn new(
		dependency: &'a SubModule<V, D>,
		interpretation: &'a mut interpretation::Local<V>,
		dataset: &'a dataset::LocalDataset,
	) -> Self {
		// let reservation = interpretation.begin_reservation();

		Self {
			dependency,
			interpretation,
			// interpretation_reservation: RefCell::new(reservation),
			dataset,
		}
	}

	pub fn begin_reservation(&self) -> interpretation::local::Reservation<'_, V> {
		self.interpretation.begin_reservation()
	}

	pub fn apply_reservation(&mut self, generator: interpretation::local::CompletedReservation<V>) {
		generator.apply(self.interpretation).unwrap()
	}

	pub fn pattern_matching<'c, 'r>(
		&'c self,
		reservation: &'r mut interpretation::local::Reservation<'c, V>,
		pattern: Signed<pattern::Canonical>,
	) -> PatternMatching<'c, V, D, &'r mut interpretation::local::Reservation<'c, V>> {
		PatternMatching {
			local_matching: self.dataset.signed_matching(pattern).into_quads(),
			dependency_matching: self.dependency.pattern_matching(pattern, reservation),
		}
	}
}

impl<'r, V: Vocabulary> ContextReservation for interpretation::local::Reservation<'r, V> {
	type CompletedReservation = interpretation::local::CompletedReservation<V>;

	fn end(self) -> Self::CompletedReservation {
		self.end()
	}
}

impl<'a, V: Vocabulary, D: Module<V>> semantics::Context<V> for BuilderContext<'a, V, D>
where
	V::Iri: Clone + Eq + Hash,
	V::BlankId: Clone + Eq + Hash,
	V::Literal: Clone + Eq + Hash,
{
	type Error = D::Error;
	type PatternMatching<'r, G: ResourceGenerator> = PatternMatching<'r, V, D, G> where Self: 'r, G: 'r;

	type Reservation<'r> = interpretation::local::Reservation<'r, V> where Self: 'r;
	type CompletedReservation = interpretation::local::CompletedReservation<V>;

	fn begin_reservation(&self) -> Self::Reservation<'_> {
		self.begin_reservation()
	}

	fn apply_reservation<'c>(
		&'c mut self,
		generator: <Self::Reservation<'c> as ContextReservation>::CompletedReservation,
	) {
		self.apply_reservation(generator)
	}

	fn pattern_matching<'r, G: 'r + ResourceGenerator>(
		&'r self,
		generator: G,
		pattern: Signed<pattern::Canonical>,
	) -> Self::PatternMatching<'r, G> {
		PatternMatching {
			local_matching: self.dataset.signed_matching(pattern).into_quads(),
			dependency_matching: self.dependency.pattern_matching(pattern, generator),
		}
	}

	fn insert_iri(&mut self, vocabulary: &mut V, iri: V::Iri) -> Result<Id, Self::Error> {
		let term = rdf_types::Term::Id(rdf_types::Id::Iri(iri));
		super::insert_term(vocabulary, self.interpretation, self.dependency, term)
	}

	fn new_resource(&mut self) -> Id {
		self.interpretation.new_resource()
	}

	fn literal_interpretation(
		&self,
		vocabulary: &mut V,
		id: Id,
	) -> Result<Option<V::Literal>, Self::Error> {
		match self.interpretation.get(id) {
			Some(r) => match r.as_literal.iter().next() {
				Some(l) => Ok(Some(l.clone())),
				None => match self.dependency.interface().local_id(id) {
					Some(local_id) => {
						match self.dependency.module().interpretation().get(local_id)? {
							Some(r) => {
								use interpretation::Resource;
								r.as_literal().next_with(vocabulary).transpose()
							}
							None => Ok(None),
						}
					}
					None => Ok(None),
				},
			},
			None => Ok(None),
		}
	}
}

struct DependencyPatternMatching<'a, V, D: Dataset<'a, V>> {
	dataset: D,
	interface: &'a Interface,
	patterns: DependencyCanonicalPatterns<'a>,
	current: Option<dataset::MatchingQuads<'a, V, D>>,
	sign: Sign,
}

impl<'a, V, D: Dataset<'a, V>> FailibleIteratorWith<V> for DependencyPatternMatching<'a, V, D> {
	type Item = Fact;
	type Error = D::Error;

	fn try_next_with(&mut self, vocabulary: &mut V) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.next_with(vocabulary).transpose()? {
					Some(Meta(Signed(sign, quad), cause)) => {
						break Ok(Some(Meta(
							Signed(sign, self.interface.quad_from_dependency(quad).unwrap()),
							cause,
						)))
					}
					None => self.current = None,
				},
				None => match self.patterns.next() {
					Some(pattern) => {
						self.current = Some(
							self.dataset
								.pattern_matching(Signed(self.sign, pattern))
								.into_quads(),
						)
					}
					None => break Ok(None),
				},
			}
		}
	}
}

impl<'a, V, D: Dataset<'a, V>> IteratorWith<V> for DependencyPatternMatching<'a, V, D> {
	type Item = Result<Fact, D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.try_next_with(vocabulary).transpose()
	}
}

pub struct Generator<'r, 'a, V: Vocabulary> {
	reservation: &'r mut interpretation::local::Reservation<'a, V>,
}

impl<'r, 'a, V: Vocabulary> sub_module::ResourceGenerator for Generator<'r, 'a, V> {
	fn new_resource(&mut self) -> Id {
		self.reservation.new_resource()
	}
}

pub struct PatternMatching<'a, V: Vocabulary, D: 'a + Module<V>, G> {
	local_matching: dataset::local::MatchingQuads<'a>,
	dependency_matching: sub_module::PatternMatching<'a, V, D, G>,
}

impl<'a, V: Vocabulary, D: Module<V>, G: ResourceGenerator> FailibleIteratorWith<V>
	for PatternMatching<'a, V, D, G>
{
	type Item = (Fact, bool);
	type Error = D::Error;

	fn try_next_with(&mut self, vocabulary: &mut V) -> Result<Option<Self::Item>, D::Error> {
		match self.local_matching.next() {
			Some(quad) => Ok(Some((quad, false))),
			None => match self.dependency_matching.try_next_with(vocabulary)? {
				Some((_, quad)) => Ok(Some((quad, true))),
				None => Ok(None),
			},
		}
	}
}

impl<'r, V: Vocabulary, D: Module<V>, G: ResourceGenerator> IteratorWith<V>
	for PatternMatching<'r, V, D, G>
{
	type Item = Result<(Fact, bool), D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.try_next_with(vocabulary).transpose()
	}
}
