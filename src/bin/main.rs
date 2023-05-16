fn main() {
	// let vocabulary: rdf_types::IndexVocabulary = Default::default();

	let rules = inferdf::semantics::inference::System::new();

	let _builder = inferdf::Builder::new(
		inferdf::builder::Dependencies::<inferdf::IndexVocabulary, ()>::default(),
		rules,
	);
}
