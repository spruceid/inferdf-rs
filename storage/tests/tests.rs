use std::{
	fs,
	io::{BufReader, BufWriter, Cursor},
};

use contextual::WithContext;
use inferdf_core::{
	interpretation, uninterpreted, Cause, Dataset, Interpretation, Module, Sign, Signed,
};
use inferdf_inference::{builder, semantics, Builder};
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
	let dependencies = builder::Dependencies::<
		IndexVocabulary,
		inferdf_storage::Module<IndexVocabulary, fs::File>,
	>::default();
	let mut builder = Builder::new(
		dependencies,
		interpretation::Composite::new(),
		semantics::inference::System::new(),
	);

	for quad in quads {
		let quad = builder.insert_quad(vocabulary, quad.clone()).unwrap();

		builder
			.insert(
				vocabulary,
				Meta(Signed(Sign::Positive, quad), Cause::Stated(0)),
			)
			.unwrap();
	}

	let classification = builder.classify_anonymous_nodes().unwrap();

	let mut cursor = Cursor::new(Vec::new());

	{
		let mut output = BufWriter::new(&mut cursor);
		inferdf_storage::build(
			&*vocabulary,
			builder.interpretation().local_interpretation(),
			builder.dataset(),
			&classification,
			&mut output,
			inferdf_storage::BuildOptions { page_size: 512 },
		)
		.unwrap();
	}

	cursor.into_inner()
}

fn read_module(
	vocabulary: &mut IndexVocabulary,
	bytes: &[u8],
) -> Vec<uninterpreted::Quad<IndexVocabulary>> {
	let mut cursor = Cursor::new(bytes);
	let module =
		inferdf_storage::Module::<IndexVocabulary, _>::new(BufReader::new(&mut cursor)).unwrap();
	let mut result = Vec::new();

	for quad in module.dataset().iter().into_quads() {
		let Meta(Signed(sign, quad), _) = quad.unwrap();
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
