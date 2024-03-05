use im::hashmap::Entry;
use rdf_types::{Term, Triple};

pub use rdf_types::pattern::CanonicalTriplePattern as Canonical;

pub mod map;
pub use map::BipolarMap;

/// Resource or variable.
pub type ResourceOrVar<T = Term> = rdf_types::pattern::ResourceOrVar<T, usize>;

/// Pattern.
pub type Pattern<T> = Triple<ResourceOrVar<T>>;

pub trait TripleMatching<T> {
	fn triple_matching(&self, substitution: &mut PatternSubstitution<T>, t: Triple<&T>) -> bool;
}

impl<T, U: Matching<T>> TripleMatching<T> for Triple<U> {
	fn triple_matching(&self, substitution: &mut PatternSubstitution<T>, t: Triple<&T>) -> bool {
		self.0.matching(substitution, t.0)
			&& self.1.matching(substitution, t.1)
			&& self.2.matching(substitution, t.2)
	}
}

pub trait Matching<T> {
	fn matching(&self, substitution: &mut PatternSubstitution<T>, t: &T) -> bool;
}

impl<T: Clone + PartialEq> Matching<T> for ResourceOrVar<T> {
	fn matching(&self, substitution: &mut PatternSubstitution<T>, t: &T) -> bool {
		match self {
			Self::Resource(r) => r == t,
			Self::Var(x) => substitution.bind(*x, t.clone()),
		}
	}
}

#[derive(Debug, Clone)]
pub struct PatternSubstitution<T>(im::HashMap<usize, T>);

impl<T> Default for PatternSubstitution<T> {
	fn default() -> Self {
		Self(im::HashMap::default())
	}
}

impl<T> PatternSubstitution<T> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn contains(&self, x: usize) -> bool {
		self.0.contains_key(&x)
	}

	pub fn get(&self, x: usize) -> Option<&T> {
		self.0.get(&x)
	}

	pub fn len(&self) -> usize {
		self.0
			.keys()
			.copied()
			.reduce(usize::max)
			.map(|l| l + 1)
			.unwrap_or_default()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl<T: Clone> PatternSubstitution<T> {
	/// Bind the variable `x` to the given identifier, unless it is already
	/// bound to a different identifier.
	///
	/// Returns wether the binding succeeded.
	pub fn bind(&mut self, x: usize, id: T) -> bool
	where
		T: PartialEq,
	{
		match self.0.entry(x) {
			Entry::Occupied(e) => *e.get() == id,
			Entry::Vacant(e) => {
				e.insert(id);
				true
			}
		}
	}

	pub fn get_or_insert_with(&mut self, x: usize, f: impl FnOnce() -> T) -> &T {
		self.0.entry(x).or_insert_with(f)
	}

	pub fn to_vec(&self) -> Vec<Option<T>> {
		let mut result = Vec::new();
		result.resize_with(self.len(), || None);

		for (i, value) in &self.0 {
			result[*i] = Some(value.clone())
		}

		result
	}

	pub fn into_vec(self) -> Vec<Option<T>> {
		let mut result = Vec::new();
		result.resize_with(self.len(), || None);

		for (i, value) in &self.0 {
			result[*i] = Some(value.clone())
		}

		result
	}
}

pub trait ApplySubstitution<T> {
	type Output;

	fn apply_substitution(&self, substitution: &PatternSubstitution<T>) -> Option<Self::Output>;
}

impl<T: Clone> ApplySubstitution<T> for ResourceOrVar<T> {
	type Output = T;

	fn apply_substitution(&self, substitution: &PatternSubstitution<T>) -> Option<Self::Output> {
		match self {
			Self::Resource(id) => Some(id.clone()),
			Self::Var(x) => substitution.get(*x).cloned(),
		}
	}
}

impl<T, U: ApplySubstitution<T>> ApplySubstitution<T> for Triple<U, U, U> {
	type Output = Triple<U::Output, U::Output, U::Output>;

	fn apply_substitution(&self, substitution: &PatternSubstitution<T>) -> Option<Self::Output> {
		Some(rdf_types::Triple(
			self.0.apply_substitution(substitution)?,
			self.1.apply_substitution(substitution)?,
			self.2.apply_substitution(substitution)?,
		))
	}
}

pub trait ApplyPartialSubstitution<T>: Sized {
	fn apply_partial_substitution(&self, substitution: &PatternSubstitution<T>) -> Self;
}

impl<T: Clone> ApplyPartialSubstitution<T> for ResourceOrVar<T> {
	fn apply_partial_substitution(&self, substitution: &PatternSubstitution<T>) -> Self {
		match self {
			Self::Resource(id) => Self::Resource(id.clone()),
			Self::Var(x) => substitution
				.get(*x)
				.cloned()
				.map(Self::Resource)
				.unwrap_or(Self::Var(*x)),
		}
	}
}

impl<T, U: ApplyPartialSubstitution<T>> ApplyPartialSubstitution<T> for Triple<U, U, U> {
	fn apply_partial_substitution(&self, substitution: &PatternSubstitution<T>) -> Self {
		Self(
			self.0.apply_partial_substitution(substitution),
			self.1.apply_partial_substitution(substitution),
			self.2.apply_partial_substitution(substitution),
		)
	}
}
