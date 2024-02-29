use rdf_types::{dataset::FallibleDataset, Dataset};

use crate::{pattern::Canonical, utils::InfallibleIterator, FactRef, Signed};

pub trait SignedPatternMatchingDataset: Dataset {
	type PatternMatching<'a>: Iterator<Item = FactRef<'a, Self::Resource>>
	where
		Self: 'a;

	fn pattern_matching(
		&self,
		pattern: Signed<Canonical<&Self::Resource>>,
	) -> Self::PatternMatching<'_>;
}

pub trait FallibleSignedPatternMatchingDataset: FallibleDataset {
	type TryPatternMatching<'a>: Iterator<Item = Result<FactRef<'a, Self::Resource>, Self::Error>>
	where
		Self: 'a;

	fn try_pattern_matching(
		&self,
		pattern: Signed<Canonical<&Self::Resource>>,
	) -> Self::TryPatternMatching<'_>;
}

impl<D: SignedPatternMatchingDataset> FallibleSignedPatternMatchingDataset for D {
	type TryPatternMatching<'a> = InfallibleIterator<D::PatternMatching<'a>> where Self: 'a;

	fn try_pattern_matching(
		&self,
		pattern: Signed<Canonical<&Self::Resource>>,
	) -> Self::TryPatternMatching<'_> {
		InfallibleIterator(self.pattern_matching(pattern))
	}
}
