use std::fmt;

use inferdf_inference::semantics::Trust;
use iref::{IriBuf, IriRefBuf};
use locspan::Meta;

pub mod building;
pub mod parsing;

pub use building::Build;
pub use parsing::Parse;

pub struct Document<M> {
	pub doc: Documentation,
	pub items: Vec<Meta<Item<M>, M>>,
}

#[derive(Debug)]
pub enum Keyword {
	Base,
	Prefix,
	Group,
	Rule,
	ForAll,
	Exists,
}

impl Keyword {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Base => "base",
			Self::Prefix => "prefix",
			Self::Group => "group",
			Self::Rule => "rule",
			Self::ForAll => "forall",
			Self::Exists => "exists",
		}
	}
}

impl fmt::Display for Keyword {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.as_str().fmt(f)
	}
}

pub enum Item<M> {
	Base(IriBuf),
	Prefix(PrefixBinding<M>),
	Group(Group<M>),
	Rule(Rule<M>),
}

pub struct Prefix(String);

impl Prefix {
	pub fn as_str(&self) -> &str {
		&self.0
	}
}

pub struct PrefixBinding<M> {
	pub prefix: Meta<Prefix, M>,
	pub colon: Meta<Colon, M>,
	pub iri: Meta<IriRefBuf, M>,
}

#[derive(Debug)]
pub struct Documentation {
	pub lines: Vec<String>,
}

pub struct Group<M> {
	pub id: Meta<IriReference, M>,
	pub doc: Documentation,
	pub items: Meta<Vec<Meta<Item<M>, M>>, M>,
}

pub struct Rule<M> {
	pub id: Meta<IriReference, M>,
	pub doc: Documentation,
	pub formula: Meta<Formula<M>, M>,
}

pub struct Colon;

pub enum IriReference {
	Compact(IriRefBuf),
	Expanded(IriRefBuf),
}

pub struct VarIdent(String);

pub enum Expr {
	Var(VarIdent),
	IriRef(IriReference),
}

pub struct Pattern<M> {
	pub subject: Meta<Expr, M>,
	pub predicate: Meta<Expr, M>,
	pub object: Meta<Expr, M>,
}

pub enum Statement<M> {
	Pattern(Pattern<M>),
	Eq(Meta<Expr, M>, Meta<Expr, M>),
}

pub struct SignedStatement<M> {
	pub negative: Option<M>,
	pub statement: Meta<Statement<M>, M>,
}

pub struct MaybeTrustedStatement<M> {
	pub statement: Meta<SignedStatement<M>, M>,
	pub trust: Meta<Trust, M>,
}

pub struct SignedPattern<M> {
	pub negative: Option<M>,
	pub pattern: Meta<Pattern<M>, M>,
}

pub struct Dot;

pub enum Formula<M> {
	ForAll(ForAll<M>),
	Exists(Exists<M>),
	Conclusion(Conclusion<M>),
}

pub struct ForAll<M> {
	pub variables: Vec<Meta<VarIdent, M>>,
	pub constraints: Meta<Hypothesis<M>, M>,
	pub inner: Box<Meta<Formula<M>, M>>,
}

pub struct Exists<M> {
	pub variables: Vec<Meta<VarIdent, M>>,
	pub hypothesis: Meta<Hypothesis<M>, M>,
	pub inner: Box<Meta<Formula<M>, M>>,
}

pub struct Hypothesis<M> {
	pub patterns: Vec<Meta<SignedPattern<M>, M>>,
}

pub struct Conclusion<M> {
	pub statements: Vec<Meta<MaybeTrustedStatement<M>, M>>,
}
