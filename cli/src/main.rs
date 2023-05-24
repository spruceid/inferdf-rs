use clap::Parser;
use contextual::WithContext;
use inferdf_core::{
	interpretation::{self, Interpret}, Cause, Sign,
	Signed,
};
use inferdf_inference::{
	semantics::{self, inference::Rule},
	builder::{self, Builder}
};
use locspan::Meta;
use nquads_syntax::Parse;
use rdf_types::{IndexVocabulary, MapLiteral, InsertIntoVocabulary};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Input files.
	inputs: Vec<PathBuf>,

	/// Input semantics files.
	#[arg(short)]
	semantics: Vec<PathBuf>,

	/// Turn debugging information on.
	#[arg(short, long, action = clap::ArgAction::Count)]
	debug: u8,
}

fn main() {
	let args = Args::parse();

	stderrlog::new()
		.verbosity(args.debug as usize)
		.init()
		.expect("unable to initialize logger");

	let mut vocabulary: IndexVocabulary = Default::default();

	let dependencies = builder::Dependencies::<IndexVocabulary, ()>::default();
	let mut interpretation = interpretation::CompositeInterpretation::new();

	let mut system = semantics::inference::System::new();
	for filename in args.semantics {
		let content = std::fs::read_to_string(filename).expect("unable to read file");
		let rules: Vec<Rule<rdf_types::Term>> = ron::from_str(&content).unwrap();
		let rules = rules
			.map_literal(|l| l.insert_type_into_vocabulary(&mut vocabulary))
			.insert_into_vocabulary(&mut vocabulary)
			.interpret(&mut interpretation.with_dependencies_mut(&dependencies));

		for rule in rules {
			system.insert(rule);
		}
	}

	let mut builder = Builder::new(dependencies, interpretation, system);

	for input in args.inputs {
		let buffer = std::fs::read_to_string(input).expect("unable to read file");
		match nquads_syntax::Document::parse_str(&buffer, |_| ()) {
			Ok(quads) => {
				for quad in quads.into_value() {
					let quad = builder.insert_quad(
						quad.into_value()
							.strip_all_but_predicate()
							.map_literal(|l| l.insert_type_into_vocabulary(&mut vocabulary))
							.insert_into_vocabulary(&mut vocabulary)
							.into_grdf(),
					);

					if let Err(_e) =
						builder.insert(Meta(Signed(Sign::Positive, quad), Cause::Stated(())))
					{
						panic!("contradiction")
					}
				}

				let interpretation = builder.interpretation().inner_interpretation();
				for q in builder.dataset().iter().into_quads() {
					let s = interpretation.terms_of(*q.subject()).next().unwrap();
					let p = interpretation.terms_of(*q.predicate()).next().unwrap();
					let o = interpretation.terms_of(*q.object()).next().unwrap();
					let g = q
						.graph()
						.map(|g| interpretation.terms_of(*g).next().unwrap());
					println!("{} .", rdf_types::Quad(s, p, o, g).with(&vocabulary))
				}
			}
			Err(_) => {
				panic!("unable to parse input files")
			}
		}
	}
}
