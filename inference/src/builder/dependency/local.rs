pub struct Dependency<V: Vocabulary, D> {
	interpretation: Interpretation<V>,
	dataset: D,
}

impl<V: Vocabulary, D> Dependency<V, D> {
	pub fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}

	pub fn dataset(&self) -> &D {
		&self.dataset
	}
}

impl<V: Vocabulary, M> interpretation::composite::Dependency<V> for Dependency<V, M> {
	fn interpretation(&self) -> &Interpretation<V> {
		&self.interpretation
	}
}