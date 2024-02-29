pub trait IteratorSearch<T, J, E, F: Fn(&T, J) -> Option<T>>: Sized + Clone + Iterator
where
	Self::Item: Iterator<Item = Result<J, E>>,
{
	fn search(self, initial_value: T, f: F) -> Search<Self, T, F>;
}

impl<I: Sized + Clone + Iterator, J, E, T, F: Fn(&T, J) -> Option<T>> IteratorSearch<T, J, E, F>
	for I
where
	I::Item: Iterator<Item = Result<J, E>>,
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

impl<I: Clone + Iterator, J, E, T, F> Iterator for Search<I, T, F>
where
	I::Item: Iterator<Item = Result<J, E>>,
	F: Fn(&T, J) -> Option<T>,
{
	type Item = Result<T, E>;

	fn next(&mut self) -> Option<Self::Item> {
		while let Some(mut frame) = self.stack.pop() {
			match frame.rest.next() {
				Some(items) => {
					let mut error = None;

					for item in items {
						match item {
							Ok(item) => {
								if let Some(next) = (self.f)(&frame.value, item) {
									self.stack.push(Frame {
										value: next,
										rest: frame.rest.clone(),
									})
								}
							}
							Err(e) => {
								if error.is_none() {
									error = Some(e)
								}
							}
						}
					}

					if let Some(e) = error {
						return Some(Err(e));
					}
				}
				None => return Some(Ok(frame.value)),
			}
		}

		None
	}
}
