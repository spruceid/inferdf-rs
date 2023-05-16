use crate::{pattern::Instantiate, Id, ReplaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
}

impl<T: Instantiate> Instantiate for Signed<T> {
	type Output = Signed<T::Output>;

	fn instantiate(
		&self,
		substitution: &mut crate::pattern::PatternSubstitution,
		new_id: impl FnMut() -> Id,
	) -> Self::Output {
		Signed(self.0, self.1.instantiate(substitution, new_id))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl<T: ReplaceId> ReplaceId for Bipolar<T> {
	fn replace_id(&mut self, a: Id, b: Id) {
		self.positive.replace_id(a, b);
		self.negative.replace_id(a, b)
	}
}