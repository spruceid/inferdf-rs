pub enum Cause<M> {
	Stated(M),
	Entailed(M)
}

impl<M> Cause<M> {
	pub fn metadata(&self) -> &M {
		match self {
			Self::Stated(m) => m,
			Self::Entailed(m) => m
		}
	}
}