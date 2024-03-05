use rdf_types::vocabulary::EmbedIntoVocabulary;
use serde::{Deserialize, Serialize};

#[cfg(feature = "paged")]
use paged::Paged;

use crate::pattern::{ApplyPartialSubstitution, ApplySubstitution};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "paged", derive(Paged), paged(
	context(C),
	bounds(T: paged::EncodeSized),
	encode_bounds(T: paged::Encode<C> + paged::EncodeOnHeap<C>),
	decode_bounds(T: paged::Decode<C> + paged::DecodeFromHeap<C>)
))]
pub struct Signed<T>(pub Sign, pub T);

impl<T> Signed<T> {
	pub fn positive(t: T) -> Self {
		Self(Sign::Positive, t)
	}

	pub fn negative(t: T) -> Self {
		Self(Sign::Negative, t)
	}

	pub fn is_positive(&self) -> bool {
		self.0.is_positive()
	}

	pub fn is_negative(&self) -> bool {
		self.0.is_negative()
	}

	pub fn sign(&self) -> Sign {
		self.0
	}

	pub fn into_sign(self) -> Sign {
		self.0
	}

	pub fn value(&self) -> &T {
		&self.1
	}

	pub fn value_mut(&mut self) -> &mut T {
		&mut self.1
	}

	pub fn into_value(self) -> T {
		self.1
	}

	pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Signed<U> {
		Signed(self.0, f(self.1))
	}

	pub fn cast<U>(self) -> Signed<U>
	where
		T: Into<U>,
	{
		self.map(Into::into)
	}

	pub fn as_ref(&self) -> Signed<&T> {
		Signed(self.0, &self.1)
	}
}

impl<V, T: EmbedIntoVocabulary<V>> EmbedIntoVocabulary<V> for Signed<T> {
	type Embedded = Signed<T::Embedded>;

	fn embed_into_vocabulary(self, vocabulary: &mut V) -> Self::Embedded {
		Signed(self.0, self.1.embed_into_vocabulary(vocabulary))
	}
}

// impl<L, M, T: MapLiteral<L, M>> MapLiteral<L, M> for Signed<T> {
// 	type Output = Signed<T::Output>;

// 	fn map_literal(self, f: impl FnMut(L) -> M) -> Self::Output {
// 		Signed(self.0, self.1.map_literal(f))
// 	}
// }

// impl<V: Vocabulary, T: Interpret<V>> Interpret<V> for Signed<T> {
// 	type Interpreted = Signed<T::Interpreted>;

// 	fn interpret<'a, I: InterpretationMut<'a, V>>(
// 		self,
// 		vocabulary: &mut V,
// 		interpretation: &mut I,
// 	) -> Result<Self::Interpreted, I::Error> {
// 		Ok(Signed(
// 			self.0,
// 			self.1.interpret(vocabulary, interpretation)?,
// 		))
// 	}
// }

impl<T, U: ApplySubstitution<T>> ApplySubstitution<T> for Signed<U> {
	type Output = Signed<U::Output>;

	fn apply_substitution(
		&self,
		substitution: &crate::pattern::PatternSubstitution<T>,
	) -> Option<Self::Output> {
		Some(Signed(self.0, self.1.apply_substitution(substitution)?))
	}
}

impl<T, U: ApplyPartialSubstitution<T>> ApplyPartialSubstitution<T> for Signed<U> {
	fn apply_partial_substitution(
		&self,
		substitution: &crate::pattern::PatternSubstitution<T>,
	) -> Self {
		Signed(self.0, self.1.apply_partial_substitution(substitution))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "paged", derive(Paged))]
pub enum Sign {
	Positive,
	Negative,
}

impl Sign {
	pub fn is_positive(&self) -> bool {
		matches!(self, Self::Positive)
	}

	pub fn is_negative(&self) -> bool {
		matches!(self, Self::Negative)
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bipolar<T> {
	pub positive: T,
	pub negative: T,
}

impl<T> Bipolar<T> {
	pub fn get(&self, sign: Sign) -> &T {
		match sign {
			Sign::Positive => &self.positive,
			Sign::Negative => &self.negative,
		}
	}

	pub fn get_mut(&mut self, sign: Sign) -> &mut T {
		match sign {
			Sign::Positive => &mut self.positive,
			Sign::Negative => &mut self.negative,
		}
	}
}

impl<I: Iterator> Iterator for Bipolar<I> {
	type Item = Signed<I::Item>;

	fn next(&mut self) -> Option<Self::Item> {
		self.positive
			.next()
			.map(Signed::positive)
			.or_else(|| self.negative.next().map(Signed::negative))
	}
}

// impl<T: ReplaceId> ReplaceId for Bipolar<T> {
// 	fn replace_id(&mut self, a: Id, b: Id) {
// 		self.positive.replace_id(a, b);
// 		self.negative.replace_id(a, b)
// 	}
// }

pub struct PositiveIterator<I>(pub I);

impl<I: Iterator> Iterator for PositiveIterator<I> {
	type Item = Signed<I::Item>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(Signed::positive)
	}
}
