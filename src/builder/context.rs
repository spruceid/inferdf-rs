use rdf_types::Vocabulary;

use crate::{
	dataset::{self, Dataset},
	interpretation::{
		composite::{DependencyCanonicalPatterns, Interface},
		CompositeInterpretation,
	},
	pattern, semantics, Cause, Quad, Sign, Signed,
};

use super::{Data, DependenciesIter};

pub struct Context<'a, V: Vocabulary, M> {
	interpretation: &'a mut CompositeInterpretation<V>,
	data: &'a Data<V, M>,
}

impl<'a, V: Vocabulary, M> Context<'a, V, M> {
	pub fn new(interpretation: &'a mut CompositeInterpretation<V>, data: &'a Data<V, M>) -> Self {
		Self {
			interpretation,
			data,
		}
	}
}

impl<'a, V: Vocabulary, M> semantics::Context for Context<'a, V, M> {
	type PatternMatching<'r> = PatternMatching<'r, V, M> where Self: 'r;

	fn pattern_matching(&self, pattern: Signed<pattern::Canonical>) -> Self::PatternMatching<'_> {
		PatternMatching {
			interpretation: self.interpretation,
			dataset_iter: self.data.set.signed_matching(pattern).into_quads(),
			dependencies: self.data.dependencies.iter(),
			current: None,
			pattern,
		}
	}

	fn new_resource(&mut self) -> crate::Id {
		self.interpretation.new_resource()
	}
}

struct DependencyPatternMatching<'a, M> {
	dataset: &'a Dataset<Cause<M>>,
	interface: &'a Interface,
	patterns: DependencyCanonicalPatterns<'a>,
	current: Option<dataset::MatchingQuads<'a, Cause<M>>>,
	sign: Sign,
}

impl<'a, M> Iterator for DependencyPatternMatching<'a, M> {
	type Item = Quad;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.current.as_mut() {
				Some(current) => match current.next() {
					Some(quad) => break Some(self.interface.quad_from_dependency(quad).unwrap()),
					None => self.current = None,
				},
				None => match self.patterns.next() {
					Some(pattern) => {
						self.current = Some(
							self.dataset
								.signed_matching(Signed(self.sign, pattern))
								.into_quads(),
						)
					}
					None => break None,
				},
			}
		}
	}
}

pub struct PatternMatching<'a, V: Vocabulary, M> {
	interpretation: &'a CompositeInterpretation<V>,
	dataset_iter: dataset::MatchingQuads<'a, Cause<M>>,
	dependencies: DependenciesIter<'a, V, M>,
	current: Option<DependencyPatternMatching<'a, M>>,
	pattern: Signed<pattern::Canonical>,
}

impl<'a, V: Vocabulary, M> Iterator for PatternMatching<'a, V, M> {
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
