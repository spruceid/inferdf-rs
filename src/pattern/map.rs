use educe::Educe;
use rdf_types::{
	pattern::{map::Values, TriplePatternMap},
	Triple,
};
use std::hash::Hash;

use super::Canonical;
use crate::{Bipolar, Signed};

#[derive(Debug, Educe)]
#[educe(Default)]
pub struct BipolarMap<V, T>(Bipolar<TriplePatternMap<V, T>>);

impl<V: Eq + Hash, T: Eq + Hash> BipolarMap<V, T> {
	pub fn insert(&mut self, Signed(sign, pattern): Signed<Canonical<T>>, value: V) -> bool {
		self.0.get_mut(sign).insert(pattern, value)
	}
}

impl<V, T: Eq + Hash> BipolarMap<V, T> {
	pub fn get(&self, Signed(sign, triple): Signed<Triple<&T>>) -> Values<V> {
		self.0.get(sign).get(triple)
	}
}

// impl<V: Eq + Hash + ReplaceId> ReplaceId for BipolarMap<V> {
// 	fn replace_id(&mut self, a: Id, b: Id) {
// 		self.0.replace_id(a, b)
// 	}
// }
