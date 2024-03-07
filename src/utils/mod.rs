//! Utility types and traits.
mod search;
pub use search::*;

pub struct InfallibleIterator<I>(pub I);

impl<I: Iterator> Iterator for InfallibleIterator<I> {
	type Item = Result<I::Item, std::convert::Infallible>;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(Ok)
	}
}

pub trait IteratorExt: Sized {
	fn try_flat_map<E, J, F, T, U>(self, f: F) -> TryFlatMap<Self, J, F>
	where
		Self: Iterator<Item = Result<T, E>>,
		J: Iterator<Item = Result<U, E>>,
		F: Fn(T) -> J,
	{
		TryFlatMap {
			inner: self,
			f,
			current: None,
		}
	}
}

impl<I: Iterator> IteratorExt for I {}

pub struct TryFlatMap<I, J, F> {
	inner: I,
	f: F,
	current: Option<J>,
}

impl<T, U, E, I, J, F> Iterator for TryFlatMap<I, J, F>
where
	I: Iterator<Item = Result<T, E>>,
	J: Iterator<Item = Result<U, E>>,
	F: FnMut(T) -> J,
{
	type Item = Result<U, E>;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match &mut self.current {
				Some(j) => match j.next() {
					Some(u) => break Some(u),
					None => self.current = None,
				},
				None => match self.inner.next() {
					Some(Ok(t)) => self.current = Some((self.f)(t)),
					Some(Err(e)) => break Some(Err(e)),
					None => break None,
				},
			}
		}
	}
}

pub enum OnceOrMore<T, I> {
	Once(Option<T>),
	More(I),
}

impl<T, I: Iterator<Item = T>> Iterator for OnceOrMore<T, I> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Once(t) => t.take(),
			Self::More(i) => i.next(),
		}
	}
}

pub struct OptionIterator<I>(pub Option<I>);

impl<I: Iterator> Iterator for OptionIterator<I> {
	type Item = I::Item;

	fn next(&mut self) -> Option<Self::Item> {
		self.0.as_mut().and_then(I::next)
	}
}
