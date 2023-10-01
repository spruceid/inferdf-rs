use std::{fs, io, path::PathBuf, process::ExitCode, str::FromStr};

use clap::{Parser, Subcommand};
use codespan_reporting::{
	diagnostic::{Diagnostic, Label},
	files::SimpleFiles,
	term::{
		self,
		termcolor::{ColorChoice, StandardStream},
	},
};
use inferdf_rdfs::{
	building::{self, StandardInterpretation},
	parsing, Build, Document, Parse,
};
use iref::IriBuf;
use locspan::{Meta, Span};
use rdf_types::{interpretation, IndexVocabulary};

mod render;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Input file.
	input: PathBuf,

	/// Turn debugging information on.
	#[arg(short, long = "verbose", action = clap::ArgAction::Count)]
	verbosity: u8,

	/// Render the RDFs rules (into HTML for instance).
	#[command(subcommand)]
	command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
	Render {
		#[arg(short, long)]
		base_iri: Option<IriBuf>,

		#[arg(short, long = "prefix")]
		prefixes: Vec<PrefixBinding>,
	},
}

#[derive(Clone)]
pub struct PrefixBinding {
	prefix: String,
	value: String,
}

impl FromStr for PrefixBinding {
	type Err = InvalidPrefixBinding;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.split_once('=') {
			Some((prefix, value)) => Ok(Self {
				prefix: prefix.to_owned(),
				value: value.to_owned(),
			}),
			None => Err(InvalidPrefixBinding),
		}
	}
}

#[derive(Debug, thiserror::Error)]
#[error("invalid prefix binding")]
pub struct InvalidPrefixBinding;

fn main() -> ExitCode {
	let args = Args::parse();

	stderrlog::new()
		.verbosity(args.verbosity as usize)
		.init()
		.expect("unable to initialize logger");

	let mut vocabulary: IndexVocabulary = Default::default();
	let mut interpretation = rdf_types::interpretation::Indexed::default();

	match run(&mut vocabulary, &mut interpretation, args) {
		Ok(code) => code,
		Err(e) => {
			log::error!("{e}");
			ExitCode::FAILURE
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error(transparent)]
	IO(#[from] io::Error),

	#[error("parse error: {0}")]
	Parsing(#[from] parsing::MetaError<Span>),

	#[error("compile error: {0}")]
	Building(#[from] building::MetaError<Span>),
}

fn run(
	vocabulary: &mut IndexVocabulary,
	interpretation: &mut interpretation::Indexed,
	args: Args,
) -> Result<ExitCode, Error> {
	let input = fs::read_to_string(&args.input)?;

	let mut files = SimpleFiles::new();
	let file_id = files.add(args.input.to_string_lossy(), input);

	match process_input(
		vocabulary,
		interpretation,
		files.get(file_id).unwrap().source().as_str(),
		args.command,
	) {
		Ok(()) => Ok(ExitCode::SUCCESS),
		Err(Error::Parsing(Meta(e, span))) => {
			let diagnostic = Diagnostic::error()
				.with_message(e.to_string())
				.with_labels(vec![Label::primary(file_id, span.range())]);

			let writer = StandardStream::stderr(ColorChoice::Auto);
			let config = codespan_reporting::term::Config::default();

			term::emit(&mut writer.lock(), &config, &files, &diagnostic).unwrap();
			Ok(ExitCode::FAILURE)
		}
		Err(e) => Err(e),
	}
}

fn process_input(
	vocabulary: &mut IndexVocabulary,
	interpretation: &mut interpretation::Indexed,
	input: &str,
	command: Option<Command>,
) -> Result<(), Error> {
	let document = Document::parse_str(input, |span| span)?;
	let mut context = building::Context::new();
	let mut build_interpretation = StandardInterpretation(interpretation);
	let system = document.build(vocabulary, &mut build_interpretation, &mut context)?;

	match command {
		Some(Command::Render { base_iri, prefixes }) => {
			let mut context = render::Context::default();
			context.set_base_iri(base_iri);
			for b in prefixes {
				context.declare_prefix(b.prefix, b.value)
			}

			render::system(
				&mut io::stdout(),
				vocabulary,
				interpretation,
				&context,
				&system,
			)?
		}
		None => {
			println!("{} rule(s)", system.len());
		}
	}

	Ok(())
}
