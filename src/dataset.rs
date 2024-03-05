use rdf_types::{
	dataset::{FallibleDataset, PatternMatchingDataset},
	Dataset, Quad, Triple,
};

use crate::{
	pattern::Canonical,
	utils::{InfallibleIterator, OptionIterator},
	PositiveIterator, Sign, Signed,
};

pub trait TraversableSignedDataset: Dataset {
	type SignedQuads<'a>: Iterator<Item = Signed<Quad<&'a Self::Resource>>>
	where
		Self: 'a;

	fn signed_quads(&self) -> Self::SignedQuads<'_>;
}

pub trait SignedPatternMatchingDataset: Dataset {
	type SignedPatternMatching<'a, 'p>: Iterator<Item = Signed<Quad<&'a Self::Resource>>>
	where
		Self: 'a,
		Self::Resource: 'p;

	fn signed_pattern_matching<'p>(
		&self,
		pattern: Signed<Canonical<&'p Self::Resource>>,
	) -> Self::SignedPatternMatching<'_, 'p>;

	fn contains_signed_triple(&self, triple: Signed<Triple<&Self::Resource>>) -> bool {
		self.signed_pattern_matching(triple.map(Into::into))
			.next()
			.is_some()
	}
}

impl<D: PatternMatchingDataset> SignedPatternMatchingDataset for D {
	type SignedPatternMatching<'a, 'p> = OptionIterator<PositiveIterator<D::QuadPatternMatching<'a, 'p>>> where Self: 'a, Self::Resource: 'p;

	fn signed_pattern_matching<'p>(
		&self,
		Signed(sign, pattern): Signed<Canonical<&'p Self::Resource>>,
	) -> Self::SignedPatternMatching<'_, 'p> {
		match sign {
			Sign::Positive => OptionIterator(Some(PositiveIterator(
				self.quad_pattern_matching(pattern.with_any_graph()),
			))),
			Sign::Negative => OptionIterator(None),
		}
	}
}

pub trait FallibleSignedPatternMatchingDataset: FallibleDataset {
	type TrySignedPatternMatching<'a, 'p>: Iterator<
		Item = Result<Signed<Quad<&'a Self::Resource>>, Self::Error>,
	> where
		Self: 'a,
		Self::Resource: 'p;

	fn try_signed_pattern_matching<'p>(
		&self,
		pattern: Signed<Canonical<&'p Self::Resource>>,
	) -> Self::TrySignedPatternMatching<'_, 'p>;

	fn try_contains_signed_triple(
		&self,
		triple: Signed<Triple<&Self::Resource>>,
	) -> Result<bool, Self::Error> {
		Ok(self
			.try_signed_pattern_matching(triple.map(Into::into))
			.next()
			.transpose()?
			.is_some())
	}
}

impl<D: SignedPatternMatchingDataset> FallibleSignedPatternMatchingDataset for D {
	type TrySignedPatternMatching<'a, 'p> = InfallibleIterator<D::SignedPatternMatching<'a, 'p>> where Self: 'a, Self::Resource: 'p;

	fn try_signed_pattern_matching<'p>(
		&self,
		pattern: Signed<Canonical<&'p Self::Resource>>,
	) -> Self::TrySignedPatternMatching<'_, 'p> {
		InfallibleIterator(self.signed_pattern_matching(pattern))
	}

	fn try_contains_signed_triple(
		&self,
		triple: Signed<Triple<&Self::Resource>>,
	) -> Result<bool, Self::Error> {
		Ok(self.contains_signed_triple(triple))
	}
}

/// Mutable dataset.
pub trait SignedDatasetMut: Dataset {
	fn insert(&mut self, quad: Signed<Quad<Self::Resource>>);
}

pub trait FallibleTraversableSignedDataset: FallibleDataset {
	type TrySignedQuads<'a>: Iterator<Item = Result<Signed<Quad<&'a Self::Resource>>, Self::Error>>
	where
		Self: 'a;

	fn try_signed_quads(&self) -> Self::TrySignedQuads<'_>;
}

impl<D: TraversableSignedDataset> FallibleTraversableSignedDataset for D {
	type TrySignedQuads<'a> = InfallibleIterator<D::SignedQuads<'a>> where Self: 'a;

	fn try_signed_quads(&self) -> Self::TrySignedQuads<'_> {
		InfallibleIterator(self.signed_quads())
	}
}

/// Fallible mutable dataset.
pub trait FallibleSignedDatasetMut: FallibleDataset {
	fn try_insert(&mut self, quad: Signed<Quad<Self::Resource>>) -> Result<(), Self::Error>;
}

impl<D: SignedDatasetMut> FallibleSignedDatasetMut for D {
	fn try_insert(&mut self, quad: Signed<Quad<Self::Resource>>) -> Result<(), Self::Error> {
		self.insert(quad);
		Ok(())
	}
}
