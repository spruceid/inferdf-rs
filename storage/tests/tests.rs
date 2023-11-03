use std::io::{BufReader, BufWriter, Cursor};

use contextual::WithContext;
use inferdf::Builder;
use inferdf::{
	class::classification, module, uninterpreted, Cause, Dataset, Interpretation, IteratorWith,
	Module, Sign, Signed,
};
use locspan::Meta;
use nquads_syntax::Parse;
use rdf_types::{IndexVocabulary, InsertIntoVocabulary, MapLiteral};

fn load(
	vocabulary: &mut IndexVocabulary,
	input: &str,
) -> Vec<uninterpreted::Quad<IndexVocabulary>> {
	let buffer = std::fs::read_to_string(input).unwrap();
	let quads = nquads_syntax::Document::parse_str(&buffer, |_| ()).unwrap();

	let mut quads: Vec<_> = quads
		.into_value()
		.into_iter()
		.map(|quad| {
			quad.into_value()
				.strip_all_but_predicate()
				.map_literal(|l| l.insert_type_into_vocabulary(vocabulary))
				.insert_into_vocabulary(vocabulary)
				.into_grdf()
		})
		.collect();

	quads.sort_unstable();
	quads
}

fn build_module(
	vocabulary: &mut IndexVocabulary,
	quads: &[uninterpreted::Quad<IndexVocabulary>],
) -> Vec<u8> {
	let mut builder = Builder::new((), inferdf_deduction::System::new());

	for quad in quads {
		let quad = builder.insert_quad(vocabulary, quad.clone()).unwrap();

		builder
			.insert(
				vocabulary,
				Meta(Signed(Sign::Positive, quad), Cause::Stated(0)),
			)
			.unwrap();
	}

	// let classification = builder.classify_anonymous_nodes().unwrap();
	let classification = classification::Local::default();

	let mut cursor = BufWriter::new(Cursor::new(Vec::new()));

	{
		let module = module::LocalRef::new(
			builder.local_interpretation(),
			builder.local_dataset(),
			&classification,
		);

		inferdf_storage::build(
			vocabulary,
			&module,
			&mut cursor,
			inferdf_storage::build::Options::default(),
		)
		.expect("unable to write BRDF module");
	}

	cursor.into_inner().unwrap().into_inner()
}

fn read_module(
	vocabulary: &mut IndexVocabulary,
	bytes: &[u8],
) -> Vec<uninterpreted::Quad<IndexVocabulary>> {
	let mut cursor = Cursor::new(bytes);
	let module =
		inferdf_storage::Module::<IndexVocabulary, _>::new(BufReader::new(&mut cursor)).unwrap();
	let mut result = Vec::new();

	let mut facts = module.dataset().iter();
	while let Some(fact) = facts.next_with(vocabulary) {
		let (graph, _, Meta(Signed(sign, triple), _)) = fact.unwrap();
		let quad = triple.into_quad(graph);
		assert!(sign.is_positive());
		result.extend(
			module
				.interpretation()
				.uninterpreted_quads_of(vocabulary, quad)
				.unwrap(),
		)
	}

	result.sort_unstable();
	result
}

fn test(input: &str) {
	let mut vocabulary: IndexVocabulary = Default::default();
	let input_quads = load(&mut vocabulary, input);
	let module = build_module(&mut vocabulary, &input_quads);
	let module_quads = read_module(&mut vocabulary, &module);

	if input_quads != module_quads {
		eprintln!("module quads:");
		for quad in &module_quads {
			eprintln!("{} .", quad.with(&vocabulary))
		}

		std::fs::write(format!("{input}.brdf"), module).unwrap();
	}

	assert_eq!(input_quads, module_quads)
}

#[test]
fn t01() {
	test("tests/inputs/t01.nq")
}

#[test]
fn t02() {
	test("tests/inputs/t02.nq")
}

#[test]
fn t03() {
	test("tests/inputs/t03.nq")
}

#[test]
fn t04() {
	test("tests/inputs/t04.nq")
}
