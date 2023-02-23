#[derive(Debug, Clone)]
pub enum Cause<M> {
	Stated(M),
	Entailed(M),
}

impl<M> Cause<M> {
	pub fn metadata(&self) -> &M {
		match self {
			Self::Stated(m) => m,
			Self::Entailed(m) => m,
		}
	}

	pub fn into_metadata(self) -> M {
		match self {
			Self::Stated(m) => m,
			Self::Entailed(m) => m,
		}
	}

	pub fn as_ref(&self) -> Cause<&M> {
		match self {
			Self::Stated(m) => Cause::Stated(m),
			Self::Entailed(m) => Cause::Entailed(m),
		}
	}
}

impl<'a, M> Cause<&'a M> {
	pub fn cloned(&self) -> Cause<M>
	where
		M: Clone,
	{
		match self {
			Self::Stated(m) => Cause::Stated((*m).clone()),
			Self::Entailed(m) => Cause::Entailed((*m).clone()),
		}
	}
}
