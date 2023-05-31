use locspan::Meta;
use rdf_types::Vocabulary;

use inferdf_core::{
	dataset::{self, Dataset},
	interpretation::{
		composite::{self, DependencyCanonicalPatterns, Interface},
	},
	pattern, Cause, Id, Quad, Sign, Signed,
};

use crate::semantics;

use super::{Data, DependenciesIter, Dependency};

pub struct Context<'a, V: Vocabulary, D: Dependency<V>> {
	interpretation: &'a mut composite::Interpretation<V>,
	data: &'a Data<V, D>,
}

impl<'a, V: Vocabulary, D: Dependency<V>> Context<'a, V, D> {
	pub fn new(
		interpretation: &'a mut composite::Interpretation<V>,
		data: &'a Data<V, D>,
	) -> Self {
		Self {
			interpretation,
			data,
		}
	}
}

impl<'a, V: Vocabulary, D: Dependency<V>> semantics::Context for Context<'a, V, D> {
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

impl<'a, D: Dataset<'a>> Iterator for DependencyPatternMatching<'a, D> {
	type Item = Quad;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.next() {
					Some(Meta(Signed(_, quad), _)) => {
						break Some(self.interface.quad_from_dependency(quad).unwrap())
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
					None => break None,
				},
			}
		}
	}
}

pub struct PatternMatching<'a, V: Vocabulary, D: Dependency<V>> {
	interpretation: &'a composite::Interpretation<V>,
	dataset_iter: dataset::standard::MatchingQuads<'a, Cause<D::Metadata>>,
	dependencies: DependenciesIter<'a, V, D>,
	current: Option<DependencyPatternMatching<'a, D::Dataset<'a>>>,
	pattern: Signed<pattern::Canonical>,
}

impl<'a, V: Vocabulary, D: Dependency<V>> Iterator for PatternMatching<'a, V, D> {
	type Item = Quad;

	fn next(&mut self) -> Option<Self::Item> {
		self.dataset_iter.next().or_else(|| loop {
			match self.current.as_mut() {
				Some(current) => match current.next() {
					Some(quad) => break Some(quad),
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
					None => break None,
				},
			}
		})
	}
}
