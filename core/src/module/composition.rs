pub mod interpretation;
pub mod dataset;
pub mod classification;

use std::{cell::RefCell, marker::PhantomData};

use hashbrown::HashMap;
pub use interpretation::Interpretation;
pub use dataset::Dataset;
pub use classification::Classification;
use rdf_types::Vocabulary;

use crate::{Id, Module, Quad, Sign};

/// Module composition.
pub struct Composition<V, M> {
	modules: Vec<SubModule<V, M>>,
	count: RefCell<u32>
}

impl<V, M> Composition<V, M> {
	pub fn sub_modules(&self) -> &[SubModule<V, M>] {
		&self.modules
	}

	pub fn interpretation(&self) -> Interpretation<V, M> {
		Interpretation::new(self)
	}

	/// Import a resource from a sub module using its identifier `id` in the
	/// sub module. Returns the global identifier.
	fn import(&self, sub_module: usize, local_id: Id) -> Id {
		let mut i = self.modules[sub_module].interface().borrow_mut();
		i.get_or_insert_global(local_id, || {
			Id(self.count.replace_with(|index| *index + 1))
		})
	}
}

impl<V: Vocabulary, M: Module<V>> Module<V> for Composition<V, M>
where
	V::Iri: Clone,
	V::Literal: Clone
{
	type Error = M::Error;
	
	type Dataset<'a> = Dataset<'a, V, M>
	where
		Self: 'a, V: 'a;
	type Interpretation<'a> = Interpretation<'a, V, M>
	where
		Self: 'a, V: 'a;
	type Classification<'a> = Classification<'a, V, M> where Self: 'a, V: 'a;

	fn dataset<'a>(&'a self) -> Self::Dataset<'a> where V: 'a {
		todo!()
	}

	fn interpretation<'a>(&'a self) -> Self::Interpretation<'a> where V: 'a {
		todo!()
	}

	fn classification<'a>(&'a self) -> Self::Classification<'a> where V: 'a {
		todo!()
	}
}

pub struct SubModule<V, M> {
	module: M,
	interface: RefCell<Interface>,
	vocabulary: PhantomData<V>
}

impl<V, M> SubModule<V, M> {
	pub fn new(module: M) -> Self {
		Self {
			module,
			interface: RefCell::new(Interface::new()),
			vocabulary: PhantomData
		}
	}

	pub fn module(&self) -> &M {
		&self.module
	}

	pub fn interface(&self) -> &RefCell<Interface> {
		&self.interface
	}
}

impl<V: Vocabulary, M: Module<V>> SubModule<V, M> {
	pub fn sign_of(&self, global_quad: Quad) -> Option<Sign> {
		todo!()
		// self.interface.borrow().get_local_quad(global_quad).and_then(|q| self.module.dataset().sign_of(q))
	}
}

#[derive(Default)]
pub struct Interface {
	/// Maps sub-module-local identifiers to composition-global identifiers.
	local_to_global: HashMap<Id, Id>,

	/// Maps composition-global identifiers to sub-module-local identifiers.
	global_to_local: HashMap<Id, Id>
}

impl Interface {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get_or_insert_global(&mut self, local_id: Id, f: impl FnOnce() -> Id) -> Id {
		*self.local_to_global.entry(local_id).or_insert_with(f)
	}

	pub fn get_local(&self, global_id: Id) -> Option<Id> {
		self.global_to_local.get(&global_id).copied()
	}

	pub fn get_local_quad(&self, global_quad: Quad) -> Option<Quad> {
		let g = match global_quad.3 {
			Some(g) => Some(self.get_local(g)?),
			None => None
		};

		Some(Quad::new(
			self.get_local(global_quad.0)?,
			self.get_local(global_quad.1)?,
			self.get_local(global_quad.2)?,
			g
		))
	}
}