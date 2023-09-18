pub mod classification;
pub mod dataset;
pub mod interpretation;

use std::cell::RefCell;

pub use classification::Classification;
pub use dataset::Dataset;
pub use interpretation::Interpretation;
use rdf_types::Vocabulary;

use crate::{uninterpreted, Id, IteratorWith, Module};

use self::classification::CompositionGlobalClassification;

use super::{SubModule, sub_module::{ClassificationInterface, Interface}};

pub struct CompositionSubModule<V, M> {
	sub_module: SubModule<V, M>,
	classification_interface: ClassificationInterface
}

impl<V, M> CompositionSubModule<V, M> {
	pub fn new(module: M) -> Self {
		Self {
			sub_module: SubModule::new(module),
			classification_interface: ClassificationInterface::new()
		}
	}

	pub fn as_sub_module(&self) -> &SubModule<V, M> {
		&self.sub_module
	}

	pub fn module(&self) -> &M {
		self.sub_module.module()
	}

	pub fn interface(&self) -> &Interface {
		self.sub_module.interface()
	}

	pub fn classification_interface(&self) -> &ClassificationInterface {
		&self.classification_interface
	}
}

/// Module composition.
pub struct Composition<V, M> {
	modules: Vec<CompositionSubModule<V, M>>,
	count: RefCell<u32>,
	global_classification: CompositionGlobalClassification
}

impl<V, M> Composition<V, M> {
	pub fn new(modules: impl IntoIterator<Item = M>) -> Self {
		Self {
			modules: modules.into_iter().map(CompositionSubModule::new).collect(),
			count: RefCell::new(0),
			global_classification: CompositionGlobalClassification::new()
		}
	}

	pub fn sub_modules(&self) -> &[CompositionSubModule<V, M>] {
		&self.modules
	}

	pub fn interpretation(&self) -> Interpretation<V, M> {
		Interpretation::new(self)
	}

	fn new_resource(&self) -> Id {
		Id(self.count.replace_with(|index| *index + 1))
	}
}

impl<V: Vocabulary, M: Module<V>> Composition<V, M> {
	fn find_iri_global_id(&self, vocabulary: &mut V, iri: V::Iri) -> Result<Option<Id>, M::Error>
	where
		V::Iri: Clone,
	{
		for m in &self.modules {
			use crate::Interpretation;
			if let Some(local_id) = m
				.module()
				.interpretation()
				.iri_interpretation(vocabulary, iri.clone())?
			{
				if let Some(global_id) = m.interface().global_id(local_id) {
					return Ok(Some(global_id));
				}
			}
		}

		Ok(None)
	}

	fn import_iri(&self, vocabulary: &mut V, iri: V::Iri) -> Result<Id, M::Error>
	where
		V::Iri: Clone,
	{
		let global_id = self
			.find_iri_global_id(vocabulary, iri.clone())?
			.unwrap_or_else(|| self.new_resource());

		for m in &self.modules {
			use crate::Interpretation;
			if let Some(local_id) = m
				.module()
				.interpretation()
				.iri_interpretation(vocabulary, iri.clone())?
			{
				m.interface().set_global_id(local_id, global_id)
			}
		}

		Ok(global_id)
	}

	fn find_literal_global_id(
		&self,
		vocabulary: &mut V,
		literal: V::Literal,
	) -> Result<Option<Id>, M::Error>
	where
		V::Literal: Clone,
	{
		for m in &self.modules {
			use crate::Interpretation;
			if let Some(local_id) = m
				.module()
				.interpretation()
				.literal_interpretation(vocabulary, literal.clone())?
			{
				if let Some(global_id) = m.interface().global_id(local_id) {
					return Ok(Some(global_id));
				}
			}
		}

		Ok(None)
	}

	fn import_literal(&self, vocabulary: &mut V, literal: V::Literal) -> Result<Id, M::Error>
	where
		V::Literal: Clone,
	{
		let global_id = self
			.find_literal_global_id(vocabulary, literal.clone())?
			.unwrap_or_else(|| self.new_resource());

		for m in &self.modules {
			use crate::Interpretation;
			if let Some(local_id) = m
				.module()
				.interpretation()
				.literal_interpretation(vocabulary, literal.clone())?
			{
				m.interface().set_global_id(local_id, global_id)
			}
		}

		Ok(global_id)
	}

	fn import_term(&self, vocabulary: &mut V, term: uninterpreted::Term<V>) -> Result<Id, M::Error>
	where
		V::Iri: Clone,
		V::Literal: Clone,
	{
		match term {
			rdf_types::Term::Id(rdf_types::Id::Iri(iri)) => self.import_iri(vocabulary, iri),
			rdf_types::Term::Id(rdf_types::Id::Blank(_)) => Ok(self.new_resource()),
			rdf_types::Term::Literal(lit) => self.import_literal(vocabulary, lit),
		}
	}

	/// Import a resource from its class.
	/// 
	/// The resource must have an assigned class in `sub_module`, which is only
	/// required if the resource is anonymous (without any non-blank lexical
	/// term).
	fn import_resource_from_class(
		&self,
		vocabulary: &mut V,
		sub_module: &CompositionSubModule<V, M>,
		local_id: Id,
	) -> Result<Id, M::Error>
	where
		V::Iri: Clone,
		V::Literal: Clone,
	{
		use crate::Classification;
		let mut global_id = None;
		let local_class = sub_module.module().classification().resource_class(local_id)?.unwrap();
		let global_class = sub_module.classification_interface().global_class(
			&self.global_classification,
			sub_module.as_sub_module(),
			local_class,
			|id| {
				if id == local_id {
					if global_id.is_none() {
						global_id = Some(self.new_resource());
					}

					Ok(global_id.unwrap())
				} else {
					self.import_resource(
						vocabulary,
						sub_module,
						local_id
					)
				}
			}
		)?;

		match global_id {
			Some(id) => {
				self.global_classification.set_class_representative(global_class, id);
				Ok(id)
			},
			None => {
				Ok(self.global_classification.get_or_insert_class_representative(global_class, || self.new_resource()))
			}
		}
	}

	/// Import a resource from its non-blank lexical representation.
	/// 
	/// Returns `None` if the resource has no known non-blank lexical
	/// representation (it is an anonymous resource).
	fn import_resource_from_lexical_terms(
		&self,
		vocabulary: &mut V,
		sub_module: &CompositionSubModule<V, M>,
		local_id: Id,
	) -> Result<Option<Id>, M::Error>
	where
		V::Iri: Clone,
		V::Literal: Clone,
	{
		use crate::Interpretation;
		let mut terms = sub_module.module().interpretation().terms_of(local_id)?;
		if let Some(term) = terms.next_with(vocabulary) {
			let term = term?;
			if !term.is_blank() {
				let global_id = self.import_term(vocabulary, term)?;
				sub_module.interface().set_global_id(local_id, global_id);
				return Ok(Some(global_id));
			}
		}

		// let global_id = Id(self.count.replace_with(|index| *index + 1));
		// sub_module.interface().set_global_id(local_id, global_id);
		// Ok(global_id)
		Ok(None)
	}

	/// Import a resource from a sub module using its identifier `id` in the
	/// sub module. Returns the global identifier.
	fn import_resource(
		&self,
		vocabulary: &mut V,
		sub_module: &CompositionSubModule<V, M>,
		local_id: Id,
	) -> Result<Id, M::Error>
	where
		V::Iri: Clone,
		V::Literal: Clone,
	{
		sub_module
			.interface()
			.get_or_try_insert_global(local_id, |local_id| {
				match self.import_resource_from_lexical_terms(vocabulary, sub_module, local_id)? {
					Some(global_id) => Ok(global_id),
					None => self.import_resource_from_class(vocabulary, sub_module, local_id)
				}
			})
	}
}

impl<V: Vocabulary, M: Module<V>> Module<V> for Composition<V, M>
where
	V::Iri: Clone,
	V::Literal: Clone,
{
	type Error = M::Error;

	type Dataset<'a> = Dataset<'a, V, M>
	where
		Self: 'a, V: 'a;
	type Interpretation<'a> = Interpretation<'a, V, M>
	where
		Self: 'a, V: 'a;
	type Classification<'a> = Classification<'a, V, M> where Self: 'a, V: 'a;

	fn dataset<'a>(&'a self) -> Self::Dataset<'a>
	where
		V: 'a,
	{
		Dataset::new(self)
	}

	fn interpretation<'a>(&'a self) -> Self::Interpretation<'a>
	where
		V: 'a,
	{
		Interpretation::new(self)
	}

	fn classification<'a>(&'a self) -> Self::Classification<'a>
	where
		V: 'a,
	{
		Classification::new(self)
	}
}
