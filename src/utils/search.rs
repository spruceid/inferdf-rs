pub trait IteratorSearch<T, F: Fn(&T, <Self::Item as Iterator>::Item) -> Option<T>>:
	Sized + Clone + Iterator
where
	Self::Item: Iterator,
{
	fn search(self, initial_value: T, f: F) -> Search<Self, T, F>;
}

impl<I: Sized + Clone + Iterator, T, F: Fn(&T, <Self::Item as Iterator>::Item) -> Option<T>>
	IteratorSearch<T, F> for I
where
	I::Item: Iterator,
{
	fn search(self, initial_value: T, f: F) -> Search<Self, T, F> {
		Search {
			stack: vec![Frame {
				value: initial_value,
				rest: self,
			}],
			f,
		}
	}
}

struct Frame<I, T> {
	value: T,
	rest: I,
}

pub struct Search<I, T, F> {
	stack: Vec<Frame<I, T>>,
	f: F,
}

impl<I: Clone + Iterator, T, F> Iterator for Search<I, T, F>
where
	I::Item: Iterator,
	F: Fn(&T, <I::Item as Iterator>::Item) -> Option<T>,
{
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		while let Some(mut frame) = self.stack.pop() {
			match frame.rest.next() {
				Some(items) => {
					for item in items {
						if let Some(next) = (self.f)(&frame.value, item) {
							self.stack.push(Frame {
								value: next,
								rest: frame.rest.clone(),
							})
						}
					}
				}
				None => return Some(frame.value),
			}
		}

		None
	}
}
