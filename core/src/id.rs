use paged::Paged;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Paged)]
pub struct Id(pub u32);

impl Id {
	pub fn index(&self) -> usize {
		self.0 as usize
	}
}

impl From<u32> for Id {
	fn from(value: u32) -> Self {
		Self(value)
	}
}

impl From<Id> for u32 {
	fn from(value: Id) -> Self {
		value.0
	}
}
