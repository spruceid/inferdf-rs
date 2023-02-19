use hashbrown::{HashMap, HashSet};
use iref::IriBuf;
use rdf_types::BlankIdBuf;

use crate::Id;

pub struct LiteralValue {
	lexical: String,
	type_: Id
}

/// RDF interpretation.
pub struct Interpretation {
	count: usize,
	resources: Vec<Resource>,
	by_iri: HashMap<IriBuf, Id>,
	by_blank: HashMap<BlankIdBuf, Id>,
	by_literal: HashMap<LiteralValue, Id>
}

pub struct Resource {
	as_iri: HashSet<IriBuf>,
	as_blank: HashSet<BlankIdBuf>,
	as_literal: HashSet<LiteralValue>
}

impl Interpretation {
	/// Merge the two given interpreted resources.
	pub fn merge(&mut self, a: Id, b: Id) -> Id {
		todo!()
	}
}