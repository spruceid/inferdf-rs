//! Node class.
//!
//! Classes are algebraic structures associated to anonymous (blank) nodes.
//! They are stored in modules, just like IRIs and literals so that structurally
//! identical blank nodes are merged across modules.

use crate::Id;

pub mod classification;
pub mod group;

pub use classification::Classification;
pub use group::GroupId;
use paged::Paged;

/// Class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Paged)]
pub struct Class {
	/// Resource group.
	pub group: GroupId,

	/// Group member.
	pub member: u32,
}

impl Class {
	pub fn new(group: GroupId, member: u32) -> Self {
		Self { group, member }
	}
}

/// Group member or class reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Paged)]
pub enum Reference {
	Singleton(Id),
	Class(Class),
	Group(u32),
}

impl Reference {
	pub fn layer(&self) -> u32 {
		match self {
			Self::Singleton(_) | Self::Group(_) => 0,
			Self::Class(c) => c.group.layer + 1
		}
	}
}