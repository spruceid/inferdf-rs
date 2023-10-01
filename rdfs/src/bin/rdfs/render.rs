use std::{collections::BTreeMap, io};

use inferdf_inference::semantics::inference;
use iref::{Iri, IriBuf, IriRefBuf};
use rdf_types::{
	interpretation::{self, ResourceIndex},
	IndexVocabulary,
};

mod html;

#[derive(Default)]
pub struct Context {
	base_iri: Option<IriBuf>,
	prefixes: BTreeMap<String, String>,
}

impl Context {
	pub fn set_base_iri(&mut self, iri: Option<IriBuf>) {
		self.base_iri = iri
	}

	pub fn declare_prefix(&mut self, prefix: String, value: String) {
		self.prefixes.insert(value, prefix);
	}

	pub fn iri_ref(&self, iri: &Iri) -> IriRefBuf {
		match &self.base_iri {
			Some(b) => iri.relative_to(b),
			None => iri.to_owned().into(),
		}
	}

	pub fn compact_iri(&self, iri: &Iri) -> String {
		for (value, prefix) in &self.prefixes {
			if let Some(suffix) = iri.strip_prefix(value) {
				return format!("{prefix}:{suffix}");
			}
		}

		iri.as_str().to_owned()
	}
}

pub fn system(
	out: &mut impl io::Write,
	vocabulary: &IndexVocabulary,
	interpretation: &interpretation::Indexed,
	context: &Context,
	system: &inference::System<ResourceIndex>,
) -> io::Result<()> {
	html::render(out, vocabulary, interpretation, context, system)
}
