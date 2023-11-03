use clap::Parser;
use codespan_reporting::{
	diagnostic::{Diagnostic, Label},
	files::SimpleFiles,
	term::{
		self,
		termcolor::{ColorChoice, StandardStream},
	},
};
use contextual::WithContext;
use inferdf::{
	builder::{BuilderInterpretation, MissingStatement},
	class::classification,
	module,
	semantics::TripleStatement,
	uninterpreted, Builder, Cause, Id, Sign, Signed,
};
use locspan::Meta;
use nquads_syntax::Parse;
use rdf_types::{
	vocabulary::BlankIdIndex, BlankIdBuf, BlankIdVocabularyMut, IndexVocabulary,
	InsertIntoVocabulary, MapLiteral, RdfDisplay,
};
use std::{
	collections::HashMap,
	fs,
	io::{self, BufReader, BufWriter, Write},
	path::PathBuf,
	process::ExitCode,
	str::FromStr,
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

	/// Close the input dataset with universally-quantified rules.
	#[arg(short, long)]
	close: bool,

	/// Turn debugging information on.
	#[arg(short, long = "verbose", action = clap::ArgAction::Count)]
	verbosity: u8,

	/// Output module file path.
	#[arg(short, long)]
	output: Option<PathBuf>,

	#[arg(short, long, default_value = "nquads")]
	format: Format,

	/// Output module page size.
	#[arg(long, default_value = "4096")]
	page_size: u32,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown format `{0}`")]
struct UnknownFormat(String);

#[derive(Debug, Clone, Copy)]
enum Format {
	NQuads,
	BinaryRdf,
}

impl FromStr for Format {
	type Err = UnknownFormat;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"nquads" => Ok(Self::NQuads),
			"brdf" => Ok(Self::BinaryRdf),
			other => Err(UnknownFormat(other.to_owned())),
		}
	}
}

struct FormatOptions {
	page_size: u32,
}

#[derive(Debug, thiserror::Error)]
enum Error<InputError, InterpretationError> {
	#[error(transparent)]
	Parsing(inferdf_rdfs::parsing::Error<InputError>),

	#[error(transparent)]
	Build(inferdf_rdfs::building::Error<InterpretationError>),
}

type ExternFile = BufReader<fs::File>;
type ExternModule = inferdf_storage::Module<IndexVocabulary, ExternFile>;

fn main() -> ExitCode {
	let args = Args::parse();

	stderrlog::new()
		.verbosity(args.verbosity as usize)
		.init()
		.expect("unable to initialize logger");

	let mut vocabulary: IndexVocabulary = Default::default();

	let mut dependencies: Vec<ExternModule> = Vec::with_capacity(args.dependencies.len());
	for path in args.dependencies {
		let input = std::fs::File::open(path).expect("unable to read file");
		let buffered_input = BufReader::new(input);
		let module =
			inferdf_storage::Module::new(buffered_input).expect("unable to read BRDF module");
		dependencies.push(module)
	}

	let mut interpretation = BuilderInterpretation::new(module::Composition::new(dependencies));

	let mut system = inferdf_deduction::System::new();
	let mut files = SimpleFiles::new();
	for filename in args.semantics {
		use inferdf_rdfs::{Build, Parse};
		let content = std::fs::read_to_string(&filename).expect("unable to read file");
		let file_id = files.add(filename.to_string_lossy().into_owned(), content);
		let input = files.get(file_id).unwrap().source().as_str();

		let result = inferdf_rdfs::Document::parse_str(input, |span| span)
			.map_err(|Meta(e, span)| Meta(Error::Parsing(e), span))
			.and_then(|document| {
				let mut context = inferdf_rdfs::building::Context::new();
				document
					.build(&mut vocabulary, &mut interpretation, &mut context)
					.map_err(|Meta(e, span)| Meta(Error::Build(e), span))
			});

		match result {
			Ok(sub_system) => system.append(sub_system),
			Err(Meta(e, span)) => {
				let diagnostic = Diagnostic::error()
					.with_message(e.to_string())
					.with_labels(vec![Label::primary(file_id, span.range())]);

				let writer = StandardStream::stderr(ColorChoice::Auto);
				let config = codespan_reporting::term::Config::default();
				term::emit(&mut writer.lock(), &config, &files, &diagnostic).unwrap();
				return ExitCode::FAILURE;
			}
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

	if args.close {
		builder
			.close(&mut vocabulary)
			.expect("unable to close dataset")
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

	// let classification = builder
	// 	.classify_anonymous_nodes()
	// 	.expect("unable to classify nodes");
	let classification = classification::Local::new(Vec::new(), Default::default());

	// TODO apply classification.

	let format_options = FormatOptions {
		page_size: args.page_size,
	};
	let output = match args.output {
		Some(path) => Output::File(BufWriter::new(
			fs::File::create(path).expect("unable to open file"),
		)),
		None => Output::StdOut(BufWriter::new(io::stdout().lock())),
	};

	match produce_output(
		&mut vocabulary,
		builder,
		classification,
		args.format,
		format_options,
		output,
	) {
		Ok(()) => ExitCode::SUCCESS,
		Err(e) => {
			log::error!("unable to produce output: {e}");
			ExitCode::FAILURE
		}
	}
}

pub enum Output {
	File(BufWriter<fs::File>),
	StdOut(BufWriter<io::StdoutLock<'static>>),
}

impl Write for Output {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match self {
			Self::File(f) => f.write(buf),
			Self::StdOut(s) => s.write(buf),
		}
	}

	fn flush(&mut self) -> io::Result<()> {
		match self {
			Self::File(f) => f.flush(),
			Self::StdOut(s) => s.flush(),
		}
	}
}

#[derive(Debug, thiserror::Error)]
enum OutputError {
	#[error(transparent)]
	IO(#[from] io::Error),

	#[error("invalid output")]
	InvalidOutput,
}

fn term_for(
	vocabulary: &mut IndexVocabulary,
	builder: &Builder<
		IndexVocabulary,
		module::Composition<IndexVocabulary, ExternModule>,
		inferdf_deduction::System,
	>,
	scope: &mut HashMap<Id, BlankIdIndex>,
	id: Id,
) -> uninterpreted::Term<IndexVocabulary> {
	match builder.local_interpretation().terms_of(id).next() {
		Some(term) => term,
		None => {
			let len = scope.len();
			let v_blank = *scope.entry(id).or_insert_with(|| {
				let blank_id = BlankIdBuf::from_suffix(&format!("gen:{}", len)).unwrap();
				vocabulary.insert_owned_blank_id(blank_id)
			});

			rdf_types::Term::Id(rdf_types::Id::Blank(v_blank))
		}
	}
}

fn produce_output(
	vocabulary: &mut IndexVocabulary,
	builder: Builder<
		IndexVocabulary,
		module::Composition<IndexVocabulary, ExternModule>,
		inferdf_deduction::System,
	>,
	classification: classification::Local,
	format: Format,
	format_options: FormatOptions,
	mut output: Output,
) -> Result<(), OutputError> {
	match format {
		Format::NQuads => {
			let mut scope = HashMap::new();
			for q in builder.local_dataset().iter().into_quads() {
				let s = term_for(vocabulary, &builder, &mut scope, *q.subject());
				let p = term_for(vocabulary, &builder, &mut scope, *q.predicate());
				let o = term_for(vocabulary, &builder, &mut scope, *q.object());
				let g = q
					.graph()
					.map(|g| term_for(vocabulary, &builder, &mut scope, *g));

				writeln!(
					output,
					"{} .",
					rdf_types::Quad(s, p, o, g).with(&*vocabulary)
				)?
			}

			Ok(())
		}
		Format::BinaryRdf => match output {
			Output::File(mut output) => {
				let module = module::LocalRef::new(
					builder.local_interpretation(),
					builder.local_dataset(),
					&classification,
				);

				inferdf_storage::build(
					vocabulary,
					&module,
					&mut output,
					inferdf_storage::build::Options {
						page_size: format_options.page_size,
					},
				)
				.expect("unable to write BRDF module");

				Ok(())
			}
			Output::StdOut(_) => Err(OutputError::InvalidOutput),
		},
	}
}
