//! Node class.
//!
//! Classes are algebraic structures associated to anonymous (blank) nodes.
//! They are stored in modules, just like IRIs and literals so that structurally
//! identical blank nodes are merged across modules.

use crate::Id;

pub mod group;

pub use group::GroupId;

/// Class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Class {
	/// Resource group.
	group: GroupId,

	/// Group member.
	member: u32,
}

impl Class {
	pub fn new(group: GroupId, member: u32) -> Self {
		Self { group, member }
	}
}

/// Group member of class reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reference {
	Singleton(Id),
	Class(Class),
	Group(u32),
}
