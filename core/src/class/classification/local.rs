use hashbrown::HashMap;

use crate::{
	class::{group, Class},
	Id,
};

pub struct Classification {
	pub layers: Vec<Layer>,
	pub classes: HashMap<Id, Class>,
}

pub struct Layer {
	pub groups: Vec<group::Description>,
}

impl Layer {
	pub fn new(groups: Vec<group::Description>) -> Self {
		Self { groups }
	}
}
