use std::{borrow::Cow, collections::HashMap, io};

use inferdf_core::{pattern::IdOrVar, Sign};
use inferdf_inference::semantics::inference::{
	self,
	rule::{Formula, Hypothesis, StatementPattern, Variable},
	Rule,
};
use rdf_types::{
	interpretation::{self, ResourceIndex, ReverseIriInterpretation},
	literal, BlankIdVocabulary, Id, IndexVocabulary, IriVocabulary, LanguageTagVocabulary,
	LiteralVocabulary, RdfDisplay, ReverseTermInterpretation, Term, Triple,
};

use super::Context;

pub fn render(
	out: &mut impl io::Write,
	vocabulary: &IndexVocabulary,
	interpretation: &interpretation::Indexed,
	context: &Context,
	system: &inference::System<ResourceIndex>,
) -> io::Result<()> {
	write!(out, "<ul class=\"rdfs-rules\">")?;
	for rule in system {
		write!(out, "<li>")?;
		render_rule(out, vocabulary, interpretation, context, rule)?;
		write!(out, "</li>")?;
	}
	write!(out, "</ul>")
}

fn render_rule(
	out: &mut impl io::Write,
	vocabulary: &IndexVocabulary,
	interpretation: &interpretation::Indexed,
	context: &Context,
	rule: &Rule<ResourceIndex>,
) -> io::Result<()> {
	write!(out, "<div class=\"rdfs-rule\">")?;

	let iri = vocabulary
		.iri(interpretation.iris_of(&rule.id).next().unwrap())
		.unwrap();
	let iri_ref = context.iri_ref(iri);
	write!(out, "<span class=\"rdfs-rule-id\">{iri_ref}<a class=\"rdfs-anchor\" href=\"{iri_ref}\">§</a></span>")?;

	let mut scope = HashMap::new();
	write!(out, "<div class=\"rdfs-formula\">")?;
	render_formula(
		out,
		vocabulary,
		interpretation,
		context,
		&mut scope,
		&rule.formula,
	)?;
	write!(out, "</div></div>")
}

fn render_formula(
	out: &mut impl io::Write,
	vocabulary: &IndexVocabulary,
	interpretation: &interpretation::Indexed,
	context: &Context,
	scope: &mut HashMap<usize, String>,
	formula: &Formula<ResourceIndex>,
) -> io::Result<()> {
	match formula {
		Formula::Exists(e) => {
			render_bindings(
				out,
				vocabulary,
				interpretation,
				context,
				scope,
				"rdfs-exists",
				'∃',
				e.variables(),
				e.hypothesis(),
			)?;
			render_formula(out, vocabulary, interpretation, context, scope, e.inner())
		}
		Formula::ForAll(a) => {
			render_bindings(
				out,
				vocabulary,
				interpretation,
				context,
				scope,
				"rdfs-forall",
				'∀',
				&a.variables,
				&a.constraints,
			)?;
			render_formula(out, vocabulary, interpretation, context, scope, &a.inner)
		}
		Formula::Conclusion(c) => {
			write!(out, "<div class=\"rdfs-conclusion\">")?;
			render_variables(out, scope, '∃', &c.variables)?;

			write!(out, "<ul class=\"rdfs-statements\">")?;
			for p in &c.statements {
				let (p, _trust) = p.as_parts();

				write!(out, "<li>")?;

				render_sign(out, p.sign())?;

				match &p.1 {
					StatementPattern::Eq(s, o) => {
						render_id_or_var(out, vocabulary, interpretation, context, scope, s)?;
						write!(out, "<span class=\"rdfs-eq\">=</span>")?;
						render_id_or_var(out, vocabulary, interpretation, context, scope, o)?;
					}
					StatementPattern::Triple(Triple(s, p, o)) => {
						render_id_or_var(out, vocabulary, interpretation, context, scope, s)?;
						render_id_or_var(out, vocabulary, interpretation, context, scope, p)?;
						render_id_or_var(out, vocabulary, interpretation, context, scope, o)?;
					}
				}

				write!(out, "</li>")?;
			}
			write!(out, "</ul></div>")
		}
	}
}

fn render_variables(
	out: &mut impl io::Write,
	scope: &mut HashMap<usize, String>,
	binding_symbol: char,
	variables: &[Variable],
) -> io::Result<()> {
	if variables.is_empty() {
		Ok(())
	} else {
		write!(
			out,
			"<div class=\"rdfs-vars\"><span class=\"rdfs-quantifer\">{binding_symbol}</span><ul>"
		)?;
		for x in variables {
			let name = x
				.name
				.as_deref()
				.map(Cow::Borrowed)
				.unwrap_or_else(|| Cow::Owned(format!("x{}", x.index)));

			scope.insert(x.index, name.as_ref().to_owned());

			write!(out, "<li><span class=\"rdfs-var\">{name}</span></li>")?;
		}
		write!(out, "</ul></div>")
	}
}

#[allow(clippy::too_many_arguments)]
fn render_bindings(
	out: &mut impl io::Write,
	vocabulary: &IndexVocabulary,
	interpretation: &interpretation::Indexed,
	context: &Context,
	scope: &mut HashMap<usize, String>,
	binding_class: &str,
	binding_symbol: char,
	variables: &[Variable],
	hypothesis: &Hypothesis<ResourceIndex>,
) -> io::Result<()> {
	write!(out, "<div class=\"rdfs-binding {binding_class}\">")?;

	render_variables(out, scope, binding_symbol, variables)?;

	write!(out, "<ul class=\"rdfs-statements\">")?;
	for p in &hypothesis.patterns {
		write!(out, "<li>")?;

		render_sign(out, p.sign())?;
		render_id_or_var(out, vocabulary, interpretation, context, scope, &p.1 .0)?;
		render_id_or_var(out, vocabulary, interpretation, context, scope, &p.1 .1)?;
		render_id_or_var(out, vocabulary, interpretation, context, scope, &p.1 .2)?;

		write!(out, "</li>")?;
	}
	write!(out, "</ul>")?;

	write!(out, "</div>")
}

fn render_sign(out: &mut impl io::Write, sign: Sign) -> io::Result<()> {
	match sign {
		Sign::Positive => {
			write!(out, "<span class=\"rdfs-sign rdfs-positive\"></span>")
		}
		Sign::Negative => {
			write!(out, "<span class=\"rdfs-sign rdfs-negative\">¬</span>")
		}
	}
}

fn render_id_or_var(
	out: &mut impl io::Write,
	vocabulary: &IndexVocabulary,
	interpretation: &interpretation::Indexed,
	context: &Context,
	scope: &HashMap<usize, String>,
	value: &IdOrVar<ResourceIndex>,
) -> io::Result<()> {
	match value {
		IdOrVar::Id(id) => render_id(out, vocabulary, interpretation, context, *id),
		IdOrVar::Var(x) => {
			let name = scope.get(x);
			let name = name
				.map(|s| Cow::Borrowed(s.as_str()))
				.unwrap_or_else(|| Cow::Owned(format!("x{x}")));
			write!(out, "<span class=\"rdfs-var\">{name}</span>")
		}
	}
}

fn render_id(
	out: &mut impl io::Write,
	vocabulary: &IndexVocabulary,
	interpretation: &interpretation::Indexed,
	context: &Context,
	id: ResourceIndex,
) -> io::Result<()> {
	let term = interpretation.term_of(&id).unwrap();
	match term {
		Term::Id(Id::Iri(i)) => {
			let iri = vocabulary.iri(i).unwrap();
			let value = context.compact_iri(iri);
			write!(out, "<a class=\"rdfs-iri\" href=\"{iri}\">{value}</a>")
		}
		Term::Id(Id::Blank(b)) => {
			let blank_id = vocabulary.blank_id(b).unwrap();
			write!(out, "<span class=\"rdfs-blank\">{blank_id}</span>")
		}
		Term::Literal(l) => {
			write!(out, "<span class=\"rdfs-literal\">")?;
			let literal = vocabulary.literal(l).unwrap();
			let value = literal.value();

			write!(
				out,
				"<span class=\"rdfs-value\">{}</span>",
				value.rdf_display()
			)?;

			match literal.type_() {
				literal::Type::Any(i) => {
					let iri = vocabulary.iri(i).unwrap();
					let ty = context.compact_iri(iri);
					write!(out, "<span class=\"rdfs-type rdfs-iri-type\">{ty}</span>")?;
				}
				literal::Type::LangString(t) => {
					let tag = vocabulary.language_tag(t).unwrap();
					write!(out, "<span class=\"rdfs-type rdfs-lang-type\">{tag}</span>")?;
				}
			}

			write!(out, "</span>")
		}
	}
}
