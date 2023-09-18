use locspan::Meta;
use rdf_types::Vocabulary;
use std::cell::RefCell;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::{
		self,
		composite::{DependencyCanonicalPatterns, Interface},
	},
	module::{sub_module, SubModule},
	pattern, Fact, FailibleIteratorWith, Id, IteratorWith, Module, Sign, Signed,
};

use crate::semantics;

// use super::{Data, DependenciesIter};

pub struct BuilderContext<'a, V: Vocabulary, D> {
	dependency: &'a SubModule<V, D>,
	interpretation: RefCell<interpretation::local::Reservation<'a, V>>,
	dataset: &'a dataset::LocalDataset,
}

impl<'a, V: Vocabulary, D: Module<V>> BuilderContext<'a, V, D> {
	pub fn new(
		dependency: &'a SubModule<V, D>,
		interpretation: &'a interpretation::Local<V>,
		dataset: &'a dataset::LocalDataset,
	) -> Self {
		Self {
			dependency,
			interpretation: RefCell::new(interpretation.begin_reservation()),
			dataset,
		}
	}

	pub fn end(self) -> interpretation::local::CompletedReservation<V> {
		self.interpretation.into_inner().end()
	}
}

impl<'a, V: Vocabulary, D: Module<V>> semantics::Context<V> for BuilderContext<'a, V, D> {
	type Error = D::Error;
	type PatternMatching<'r> = PatternMatching<'r, 'a, V, D> where Self: 'r;

	fn pattern_matching(&self, pattern: Signed<pattern::Canonical>) -> Self::PatternMatching<'_> {
		PatternMatching {
			local_matching: self.dataset.signed_matching(pattern).into_quads(),
			dependency_matching: self.dependency.pattern_matching(
				pattern,
				Generator {
					interpretation: &self.interpretation,
				},
			),
		}
	}

	fn new_resource(&self) -> Id {
		let mut r = self.interpretation.borrow_mut();
		r.new_resource()
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
	interpretation: &'r RefCell<interpretation::local::Reservation<'a, V>>,
}

impl<'r, 'a, V: Vocabulary> sub_module::ResourceGenerator for Generator<'r, 'a, V> {
	fn new_resource(&mut self) -> Id {
		let mut r = self.interpretation.borrow_mut();
		r.new_resource()
	}
}

pub struct PatternMatching<'r, 'a, V: Vocabulary, D: 'a + Module<V>> {
	local_matching: dataset::local::MatchingQuads<'a>,
	dependency_matching: sub_module::PatternMatching<'a, V, D, Generator<'r, 'a, V>>,
}

impl<'r, 'a, V: Vocabulary, D: Module<V>> FailibleIteratorWith<V>
	for PatternMatching<'r, 'a, V, D>
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

impl<'r, 'a, V: Vocabulary, D: Module<V>> IteratorWith<V> for PatternMatching<'r, 'a, V, D> {
	type Item = Result<(Fact, bool), D::Error>;

	fn next_with(&mut self, vocabulary: &mut V) -> Option<Self::Item> {
		self.try_next_with(vocabulary).transpose()
	}
}
