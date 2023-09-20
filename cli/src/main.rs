use clap::Parser;
use contextual::WithContext;
use inferdf_core::{interpretation::Interpret, module, Cause, Sign, Signed};
use inferdf_inference::{
	builder::{BuilderInterpretation, MissingStatement},
	semantics::{
		self,
		inference::{rule::TripleStatement, Rule},
	},
};
use locspan::Meta;
use nquads_syntax::Parse;
use rdf_types::{IndexVocabulary, InsertIntoVocabulary, MapLiteral, RdfDisplay};
use std::{
	fs,
	io::{BufReader, BufWriter},
	path::PathBuf,
	process::ExitCode,
};
use yansi::Paint;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Input files.
	inputs: Vec<PathBuf>,

	/// Dependency module.
	#[arg(short)]
	dependencies: Vec<PathBuf>,

	/// Input semantics files.
	#[arg(short)]
	semantics: Vec<PathBuf>,

	/// Turn debugging information on.
	#[arg(short, long = "verbose", action = clap::ArgAction::Count)]
	verbosity: u8,

	/// Output module file path.
	#[arg(short, long, default_value = "out.brdf")]
	output: PathBuf,

	/// Output module page size.
	#[arg(long, default_value = "4096")]
	page_size: u32,
}

fn main() -> ExitCode {
	let args = Args::parse();

	stderrlog::new()
		.verbosity(args.verbosity as usize)
		.init()
		.expect("unable to initialize logger");

	let mut vocabulary: IndexVocabulary = Default::default();

	let mut dependencies: Vec<inferdf_storage::Module<IndexVocabulary, BufReader<fs::File>>> =
		Vec::with_capacity(args.dependencies.len());
	for path in args.dependencies {
		let input = std::fs::File::open(path).expect("unable to read file");
		let buffered_input = BufReader::new(input);
		let module =
			inferdf_storage::Module::new(buffered_input).expect("unable to read BRDF module");
		dependencies.push(module)
	}

	let mut interpretation = BuilderInterpretation::new(module::Composition::new(dependencies));

	let mut system = semantics::inference::System::new();
	for filename in args.semantics {
		let content = std::fs::read_to_string(filename).expect("unable to read file");
		let rules: Vec<Rule<rdf_types::Term>> = ron::from_str(&content).unwrap();
		let rules = rules
			.map_literal(|l| l.insert_type_into_vocabulary(&mut vocabulary))
			.insert_into_vocabulary(&mut vocabulary)
			.interpret(&mut vocabulary, &mut interpretation)
			.unwrap();

		for rule in rules {
			system.insert(rule);
		}
	}

	let mut builder = interpretation.into_builder(system);

	for input in args.inputs {
		let buffer = std::fs::read_to_string(input).expect("unable to read file");
		match nquads_syntax::Document::parse_str(&buffer, |_| ()) {
			Ok(quads) => {
				for quad in quads.into_value() {
					let quad = quad
						.into_value()
						.strip_all_but_predicate()
						.map_literal(|l| l.insert_type_into_vocabulary(&mut vocabulary))
						.insert_into_vocabulary(&mut vocabulary)
						.into_grdf();

					let quad = builder
						.insert_quad(&mut vocabulary, quad)
						.expect("insertion failed");

					if let Err(_e) = builder.insert(
						&mut vocabulary,
						Meta(Signed(Sign::Positive, quad), Cause::Stated(0)),
					) {
						panic!("contradiction")
					}
				}
			}
			Err(_) => {
				panic!("unable to parse input files")
			}
		}
	}

	let interpretation = builder.local_interpretation();
	for q in builder.local_dataset().iter().into_quads() {
		let s = interpretation.terms_of(*q.subject()).next().unwrap();
		let p = interpretation.terms_of(*q.predicate()).next().unwrap();
		let o = interpretation.terms_of(*q.object()).next().unwrap();
		let g = q
			.graph()
			.map(|g| interpretation.terms_of(*g).next().unwrap());
		println!("{} .", rdf_types::Quad(s, p, o, g).with(&vocabulary))
	}

	if let Err(MissingStatement(Signed(_sign, statement), e)) = builder.check(&mut vocabulary) {
		match statement {
			TripleStatement::Triple(t) => {
				let interpretation = builder.local_interpretation();
				let s = interpretation.terms_of(*t.subject()).next().unwrap();
				let p = interpretation.terms_of(*t.predicate()).next().unwrap();
				let o = interpretation.terms_of(*t.object()).next().unwrap();

				let entailment = builder.entailment(e).unwrap();
				let rule_id = interpretation.terms_of(entailment.rule).next().unwrap();

				eprintln!(
					"{}: {}",
					Paint::red("error").bold(),
					Paint::new("missing required statement:").bold()
				);
				eprintln!();
				eprintln!(
					"\t{} .",
					Paint::new(rdf_types::Triple(s, p, o).with(&vocabulary)).bold()
				);
				eprintln!();
				eprintln!("required by {}", rule_id.with(&vocabulary).rdf_display());
			}
			TripleStatement::Eq(_, _) => {
				todo!()
			}
		}

		return ExitCode::FAILURE;
	}

	let classification = builder
		.classify_anonymous_nodes()
		.expect("unable to classify nodes");

	let module = module::LocalRef::new(
		builder.local_interpretation(),
		builder.local_dataset(),
		&classification,
	);

	let mut output = BufWriter::new(fs::File::create(args.output).expect("unable to open file"));

	inferdf_storage::build(
		&mut vocabulary,
		&module,
		&mut output,
		inferdf_storage::build::Options {
			page_size: args.page_size,
		},
	)
	.expect("unable to write BRDF module");

	ExitCode::SUCCESS
}
