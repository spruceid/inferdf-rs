use crate::{
	class::{
		group::{self, Member},
		Reference,
	},
	Class, Id, Signed,
};
use normal_form::Normalize;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonNormalizedDescription {
	members: Vec<Member>,
	len: u32,
	neighbors: Vec<Vec<u32>>,
}

impl NonNormalizedDescription {
	pub fn new(members: Vec<Member>) -> Self {
		let mut neighbors = Vec::with_capacity(members.len());

		for m in &members {
			let mut m_neighbors = Vec::new();

			for Signed(_, (a, b)) in &m.properties.0 {
				if let Reference::Group(a) = a {
					m_neighbors.push(*a)
				}

				if let Reference::Group(b) = b {
					m_neighbors.push(*b)
				}
			}

			neighbors.push(m_neighbors);
		}

		for n in &mut neighbors {
			n.sort_unstable();
			n.dedup();
		}

		let len = members.len() as u32;

		Self {
			members,
			len,
			neighbors,
		}
	}
}

impl Normalize for NonNormalizedDescription {
	type Elements = u32;

	type Color = Color;

	type Morphed = group::Description;

	type Cache = Cache;

	fn elements(&self) -> &Self::Elements {
		&self.len
	}

	fn initial_coloring(&self) -> Vec<Color> {
		self.members
			.iter()
			.map(|m| {
				let mut picker = ColorPicker::with_capacity(m.len());
				for &binding in m {
					picker.insert(binding)
				}
				picker.pick()
			})
			.collect()
	}

	fn initialize_cache(&self) -> Self::Cache {
		let mut map = Vec::new();
		map.resize(self.len as usize, 0usize);

		Cache {
			stack: Vec::new(),
			map,
		}
	}

	fn refine_coloring(
		&self,
		cache: &mut Self::Cache,
		coloring: &mut normal_form::ReversibleColoring<Self::Elements>,
	) {
		coloring.make_equitable_with(&mut cache.stack, &mut cache.map, |i| {
			&self.neighbors[*i as usize]
		})
	}

	fn apply_morphism<F>(&self, morphism: F) -> Self::Morphed
	where
		F: Fn(&u32) -> usize,
	{
		let mut members = self.members.clone();

		fn apply_morphism_on_reference(reference: &mut Reference, f: impl Fn(&u32) -> usize) {
			if let Reference::Group(x) = reference {
				*x = f(x) as u32
			}
		}

		for m in &mut members {
			for Signed(_, (a, b)) in m {
				apply_morphism_on_reference(a, &morphism);
				apply_morphism_on_reference(b, &morphism)
			}
		}

		members.sort_unstable();
		group::Description::from_normalized_members(members)
	}
}

pub struct Cache {
	stack: Vec<usize>,
	map: Vec<usize>,
}

/// Group member color.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Color(Vec<Pigment>);

/// Color builder.
#[derive(Default)]
struct ColorPicker(Vec<Pigment>);

impl ColorPicker {
	pub fn with_capacity(capacity: usize) -> Self {
		Self(Vec::with_capacity(capacity))
	}

	pub fn pick(self) -> Color {
		let mut pigments = self.0;
		pigments.sort_unstable();
		Color(pigments)
	}

	pub fn insert(&mut self, pigment: impl Into<Pigment>) {
		self.0.push(pigment.into())
	}
}

/// Color pigment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Pigment(Signed<(AnonymousReference, AnonymousReference)>);

impl From<Signed<(Reference, Reference)>> for Pigment {
	fn from(Signed(sign, (a, b)): Signed<(Reference, Reference)>) -> Self {
		Self(Signed(sign, (a.into(), b.into())))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum AnonymousReference {
	Singleton(Id),
	Class(Class),
	Group,
}

impl From<Reference> for AnonymousReference {
	fn from(value: Reference) -> Self {
		match value {
			Reference::Singleton(id) => Self::Singleton(id),
			Reference::Class(id) => Self::Class(id),
			Reference::Group(_) => Self::Group,
		}
	}
}
