use locspan::Meta;
use rdf_types::Vocabulary;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::composite::{self, DependencyCanonicalPatterns, Interface},
	pattern, FailibleIterator, Id, Module, Quad, Sign, Signed,
};

use crate::semantics;

use super::{Data, DependenciesIter};

pub struct Context<'a, V: Vocabulary, D: Module<V>> {
	interpretation: &'a mut composite::Interpretation<V>,
	data: &'a Data<V, D>,
}

impl<'a, V: Vocabulary, D: Module<V>> Context<'a, V, D> {
	pub fn new(interpretation: &'a mut composite::Interpretation<V>, data: &'a Data<V, D>) -> Self {
		Self {
			interpretation,
			data,
		}
	}
}

impl<'a, V: Vocabulary, D: Module<V>> semantics::Context for Context<'a, V, D> {
	type Error = D::Error;
	type PatternMatching<'r> = PatternMatching<'r, V, D> where Self: 'r;

	fn pattern_matching(&self, pattern: Signed<pattern::Canonical>) -> Self::PatternMatching<'_> {
		PatternMatching {
			interpretation: self.interpretation,
			dataset_iter: self.data.set.signed_matching(pattern).into_quads(),
			dependencies: self.data.dependencies.iter(),
			current: None,
			pattern,
		}
	}

	fn new_resource(&mut self) -> Id {
		self.interpretation.new_resource()
	}
}

struct DependencyPatternMatching<'a, D: Dataset<'a>> {
	dataset: D,
	interface: &'a Interface,
	patterns: DependencyCanonicalPatterns<'a>,
	current: Option<dataset::MatchingQuads<'a, D>>,
	sign: Sign,
}

impl<'a, D: Dataset<'a>> FailibleIterator for DependencyPatternMatching<'a, D> {
	type Item = Quad;
	type Error = D::Error;

	fn try_next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.next().transpose()? {
					Some(Meta(Signed(_, quad), _)) => {
						break Ok(Some(self.interface.quad_from_dependency(quad).unwrap()))
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

impl<'a, D: Dataset<'a>> Iterator for DependencyPatternMatching<'a, D> {
	type Item = Result<Quad, D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.try_next().transpose()
	}
}

pub struct PatternMatching<'a, V: Vocabulary, D: Module<V>> {
	interpretation: &'a composite::Interpretation<V>,
	dataset_iter: dataset::local::MatchingQuads<'a>,
	dependencies: DependenciesIter<'a, V, D>,
	current: Option<DependencyPatternMatching<'a, D::Dataset<'a>>>,
	pattern: Signed<pattern::Canonical>,
}

impl<'a, V: Vocabulary, D: Module<V>> FailibleIterator for PatternMatching<'a, V, D> {
	type Item = Quad;
	type Error = D::Error;

	fn try_next(&mut self) -> Result<Option<Self::Item>, D::Error> {
		match self.dataset_iter.next() {
			Some(quad) => Ok(Some(quad)),
			None => loop {
				match self.current.as_mut() {
					Some(current) => match current.try_next()? {
						Some(quad) => break Ok(Some(quad)),
						None => self.current = None,
					},
					None => match self.dependencies.next() {
						Some((d, dependency)) => {
							if let Some(interface) = self.interpretation.interface(d) {
								self.current = Some(DependencyPatternMatching {
									dataset: dependency.dataset(),
									interface,
									patterns: self
										.interpretation
										.dependency_canonical_patterns(d, self.pattern.1),
									current: None,
									sign: self.pattern.0,
								})
							}
						}
						None => break Ok(None),
					},
				}
			},
		}
	}
}

impl<'a, V: Vocabulary, D: Module<V>> Iterator for PatternMatching<'a, V, D> {
	type Item = Result<Quad, D::Error>;

	fn next(&mut self) -> Option<Self::Item> {
		self.try_next().transpose()
	}
}
