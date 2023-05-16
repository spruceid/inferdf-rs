pub trait IteratorSearch<T, F: Fn(T, <Self::Item as Iterator>::Item) -> T>: Sized + Iterator where Self::Item: Iterator {
	fn search(self, initial_value: T, f: F) -> Search<Self, T, F>;
}

impl<I: Sized + Iterator, T, F: Fn(T, <Self::Item as Iterator>::Item) -> T> IteratorSearch<T, F> for I where I::Item: Iterator {
	fn search(self, initial_value: T, f: F) -> Search<Self, T, F> {
		todo!()
	}
}

pub struct Search<I, T, F> {
	iter: I,
	value: T,
	f: F
}

impl<I, T, F> Iterator for Search<I, T, F> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		todo!()
	}
}