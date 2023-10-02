use inferdf_core::{pattern::IdOrVar, Sign, Signed};
use inferdf_inference::semantics::{
	inference::{self, rule::Variable},
	MaybeTrusted,
};
use std::{collections::HashMap, hash::Hash};

use iref::{InvalidIri, Iri, IriBuf, IriRef};
use locspan::Meta;
use rdf_types::{IriInterpretationMut, IriVocabulary, IriVocabularyMut, Vocabulary, VocabularyMut};

use crate::{
	Conclusion, Document, Exists, Expr, ForAll, Formula, Group, Hypothesis, IriReference, Item,
	MaybeTrustedStatement, Pattern, Rule, SignedPattern, SignedStatement, Statement, VarIdent,
};

pub trait BuildInterpretation<V: IriVocabulary> {
	type Error;

	type Resource: Clone + Eq + Hash;

	fn interpret_owned_lexical_iri<E>(
		&mut self,
		vocabulary: &mut V,
		iri: IriBuf,
	) -> Result<Self::Resource, Self::Error>
	where
		V: IriVocabularyMut;
}

pub struct StandardInterpretation<'a, I>(pub &'a mut I);

impl<'a, V: IriVocabulary, I: IriInterpretationMut<V::Iri>> BuildInterpretation<V>
	for StandardInterpretation<'a, I>
where
	I::Resource: Clone + Eq + Hash,
{
	type Error = std::convert::Infallible;

	type Resource = I::Resource;

	fn interpret_owned_lexical_iri<E>(
		&mut self,
		vocabulary: &mut V,
		iri: IriBuf,
	) -> Result<Self::Resource, Self::Error>
	where
		V: IriVocabularyMut,
	{
		Ok(self.0.interpret_owned_lexical_iri(vocabulary, iri))
	}
}

impl<V: Vocabulary, D: inferdf_core::Module<V>> BuildInterpretation<V>
	for inferdf_inference::builder::BuilderInterpretation<V, D>
where
	V::Iri: Copy + Eq + Hash,
	V::BlankId: Copy + Eq + Hash,
	V::Literal: Copy + Eq + Hash,
{
	type Error = D::Error;

	type Resource = inferdf_core::Id;

	fn interpret_owned_lexical_iri<E>(
		&mut self,
		vocabulary: &mut V,
		iri: IriBuf,
	) -> Result<Self::Resource, Self::Error>
	where
		V: IriVocabularyMut,
	{
		use inferdf_core::interpretation::InterpretationMut;
		let v_iri = vocabulary.insert_owned(iri);
		self.insert_term(vocabulary, rdf_types::Term::Id(rdf_types::Id::Iri(v_iri)))
	}
}

#[derive(Debug, thiserror::Error)]
pub enum Error<E = std::convert::Infallible> {
	#[error("no base IRI")]
	NoBaseIri,

	#[error(transparent)]
	InvalidIri(InvalidIri<String>),

	#[error(transparent)]
	Interpretation(E),
}

pub type MetaError<M, E = std::convert::Infallible> = Meta<Error<E>, M>;

pub type BuildResult<T, M, E = std::convert::Infallible> = Result<T, MetaError<M, E>>;

#[derive(Default)]
pub struct Context {
	base_iri: Option<IriBuf>,
	iri_prefixes: HashMap<String, IriBuf>,
}

impl Context {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn insert_prefix(&mut self, prefix: String, iri: IriBuf) {
		self.iri_prefixes.insert(prefix, iri);
	}

	pub fn resolve_iri_prefix(&self, prefix: &str) -> Option<&Iri> {
		self.iri_prefixes.get(prefix).map(IriBuf::as_iri)
	}

	pub fn resolve<E, M: Clone>(
		&self,
		iri_ref: &IriRef,
		meta: &M,
	) -> Result<IriBuf, MetaError<M, E>> {
		match iri_ref.as_iri() {
			Some(iri) => Ok(iri.to_owned()),
			None => match &self.base_iri {
				Some(base_iri) => Ok(iri_ref.resolved(base_iri)),
				None => Err(Meta(Error::NoBaseIri, meta.clone())),
			},
		}
	}
}

impl IriReference {
	pub fn resolve<E, M: Clone>(
		&self,
		context: &mut Context,
		meta: &M,
	) -> Result<IriBuf, MetaError<M, E>> {
		let iri_ref = match self {
			Self::Expanded(iri_ref) => iri_ref,
			Self::Compact(compact_iri_ref) => match compact_iri_ref.scheme() {
				Some(scheme) => match context.resolve_iri_prefix(scheme) {
					Some(prefix) => {
						let suffix = compact_iri_ref.split_once(':').unwrap().1;
						return IriBuf::new(format!("{prefix}{suffix}"))
							.map_err(|e| Meta(Error::InvalidIri(e), meta.clone()));
					}
					None => compact_iri_ref,
				},
				None => compact_iri_ref,
			},
		};

		context.resolve(iri_ref, meta)
	}
}

pub trait Build<V: Vocabulary, I: BuildInterpretation<V>, M> {
	type Target;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
	) -> BuildResult<Self::Target, M, I::Error>;
}

pub trait BuildScoped<V: Vocabulary, I: BuildInterpretation<V>, M> {
	type Target;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error>;
}

pub trait BuildScopedWith<V: Vocabulary, I: BuildInterpretation<V>, M> {
	type Target;

	fn build_with(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
		meta: &M,
	) -> BuildResult<Self::Target, M, I::Error>;
}

impl<T: BuildScopedWith<V, I, M>, V: Vocabulary, I: BuildInterpretation<V>, M> BuildScoped<V, I, M>
	for Meta<T, M>
{
	type Target = T::Target;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		self.0
			.build_with(vocabulary, interpretation, context, scope, &self.1)
	}
}

#[derive(Default)]
pub struct Scope {
	bindings: HashMap<String, usize>,
}

impl Scope {
	pub fn new<M>(formula: &Formula<M>) -> Self {
		let mut result = Self::default();
		result.declare_formula(formula);
		result
	}

	fn variables(&self) -> Vec<Variable> {
		let mut result: Vec<_> = self
			.bindings
			.iter()
			.map(|(name, &index)| Variable {
				index,
				name: Some(name.clone()),
			})
			.collect();
		result.sort_unstable_by_key(|x| x.index);
		result
	}

	fn declare_formula<M>(&mut self, formula: &Formula<M>) {
		match formula {
			Formula::ForAll(a) => {
				for p in &a.constraints.0.patterns {
					self.declare_pattern(&p.pattern)
				}

				self.declare_formula(&a.inner)
			}
			Formula::Exists(e) => {
				for p in &e.hypothesis.0.patterns {
					self.declare_pattern(&p.pattern)
				}

				self.declare_formula(&e.inner)
			}
			Formula::Conclusion(c) => {
				for s in &c.statements {
					match s.statement.value().statement.value() {
						Statement::Eq(a, b) => {
							self.declare_expr(a);
							self.declare_expr(b)
						}
						Statement::Pattern(p) => self.declare_pattern(p),
					}
				}
			}
		}
	}

	fn declare_expr(&mut self, expr: &Expr) {
		if let Expr::Var(name) = expr {
			if !self.bindings.contains_key(&name.0) {
				let x = self.bindings.len();
				self.bindings.insert(name.0.clone(), x);
			}
		}
	}

	fn declare_pattern<M>(&mut self, pattern: &Pattern<M>) {
		self.declare_expr(&pattern.subject);
		self.declare_expr(&pattern.predicate);
		self.declare_expr(&pattern.object)
	}

	pub fn get(&self, name: &VarIdent) -> Option<usize> {
		self.bindings.get(&name.0).copied()
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> Build<V, I, M> for Document<M> {
	type Target = inference::System<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
	) -> BuildResult<Self::Target, M, I::Error> {
		for Meta(item, _) in &self.items {
			if let Item::Base(iri) = item {
				context.base_iri = Some(iri.clone())
			}
		}

		for Meta(item, meta) in &self.items {
			if let Item::Prefix(binding) = item {
				let iri = context.resolve(binding.iri.value(), meta)?;
				context.insert_prefix(binding.prefix.as_str().to_owned(), iri)
			}
		}

		let mut system = inference::System::new();

		for Meta(item, _) in &self.items {
			match item {
				Item::Base(_) => (),
				Item::Prefix(_) => (),
				Item::Group(group) => {
					for rule in group.build(vocabulary, interpretation, context)? {
						system.insert(rule);
					}
				}
				Item::Rule(rule) => {
					let rule = rule.build(vocabulary, interpretation, context)?;
					system.insert(rule);
				}
			}
		}

		Ok(system)
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> Build<V, I, M> for Group<M> {
	type Target = Vec<inference::Rule<I::Resource>>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
	) -> BuildResult<Self::Target, M, I::Error> {
		let mut iri = self.id.resolve(context, self.id.metadata())?;
		iri.set_query(None);
		iri.set_fragment(None);
		iri.path_mut().push(iref::iri::Segment::EMPTY);
		let old_base_iri = context.base_iri.replace(iri);

		let mut rules = Vec::new();

		for Meta(item, _) in self.items.value() {
			match item {
				Item::Base(_) => (),
				Item::Prefix(_) => (),
				Item::Group(group) => {
					rules.extend(group.build(vocabulary, interpretation, context)?)
				}
				Item::Rule(rule) => {
					let rule = rule.build(vocabulary, interpretation, context)?;
					rules.push(rule);
				}
			}
		}

		context.base_iri = old_base_iri;

		Ok(rules)
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> Build<V, I, M> for Rule<M> {
	type Target = inference::Rule<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
	) -> BuildResult<Self::Target, M, I::Error> {
		let iri = self.id.resolve(context, self.id.metadata())?;
		let id = interpretation
			.interpret_owned_lexical_iri::<I::Error>(vocabulary, iri)
			.map_err(|e| Meta(Error::Interpretation(e), self.id.metadata().clone()))?;

		let scope = Scope::new(&self.formula);

		let mut formula = self
			.formula
			.build(vocabulary, interpretation, context, &scope)?;

		#[derive(Clone, Copy)]
		enum Declaration {
			Undeclared,
			ImplicitlyDeclared,
			Declared,
		}

		impl Declaration {
			fn add(&mut self, o: Self) {
				*self = match (*self, o) {
					(Self::Declared, _) | (_, Self::Declared) => Self::Declared,
					(Self::ImplicitlyDeclared, _) | (_, Self::ImplicitlyDeclared) => {
						Self::ImplicitlyDeclared
					}
					_ => Self::Undeclared,
				}
			}
		}

		let variables = scope.variables();
		let mut declaration = Vec::new();
		declaration.resize(variables.len(), Declaration::Undeclared);
		formula.visit_declared_variables(|x| {
			if let Ok(i) = variables.binary_search_by_key(&x, |y| y.index) {
				declaration[i] = Declaration::Declared
			}
		});
		formula.visit_variables(|f, x| {
			if !f.is_conclusion() {
				if let Ok(i) = variables.binary_search_by_key(&x, |y| y.index) {
					declaration[i].add(Declaration::ImplicitlyDeclared)
				}
			}
		});

		if declaration
			.iter()
			.any(|d| !matches!(d, Declaration::Declared))
		{
			let mut outer_variables = Vec::new();
			let mut inner_variables = Vec::new();

			for (x, d) in variables.into_iter().zip(declaration) {
				match d {
					Declaration::Undeclared => inner_variables.push(x),
					Declaration::ImplicitlyDeclared => outer_variables.push(x),
					Declaration::Declared => (),
				}
			}

			if !outer_variables.is_empty() {
				match formula.as_existential_mut() {
					Some(e) => {
						e.extend_variables(outer_variables);
					}
					None => {
						formula = inference::rule::Formula::Exists(inference::rule::Exists::new(
							outer_variables,
							inference::rule::Hypothesis::default(),
							formula,
						));
					}
				}
			}

			formula.conclusion_mut().variables.extend(inner_variables)
		}

		formula.normalize();

		Ok(inference::Rule::new(id, formula))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M> for Formula<M> {
	type Target = inference::rule::Formula<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		match self {
			Self::ForAll(a) => a
				.build(vocabulary, interpretation, context, scope)
				.map(inference::rule::Formula::ForAll),
			Self::Exists(e) => e
				.build(vocabulary, interpretation, context, scope)
				.map(inference::rule::Formula::Exists),
			Self::Conclusion(c) => c
				.build(vocabulary, interpretation, context, scope)
				.map(inference::rule::Formula::Conclusion),
		}
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M> for ForAll<M> {
	type Target = inference::rule::ForAll<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		let variables = self
			.variables
			.iter()
			.map(|x| Variable {
				index: scope.get(x).unwrap(),
				name: Some(x.0 .0.clone()),
			})
			.collect();
		let constraints = self
			.constraints
			.build(vocabulary, interpretation, context, scope)?;
		let inner = self
			.inner
			.build(vocabulary, interpretation, context, scope)?;

		Ok(inference::rule::ForAll {
			variables,
			constraints,
			inner: Box::new(inner),
		})
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M> for Exists<M> {
	type Target = inference::rule::Exists<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		let variables = self
			.variables
			.iter()
			.map(|x| Variable {
				index: scope.get(x).unwrap(),
				name: Some(x.0 .0.clone()),
			})
			.collect();
		let hypothesis = self
			.hypothesis
			.build(vocabulary, interpretation, context, scope)?;
		let inner = self
			.inner
			.build(vocabulary, interpretation, context, scope)?;

		Ok(inference::rule::Exists::new(variables, hypothesis, inner))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M> for Hypothesis<M> {
	type Target = inference::rule::Hypothesis<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		let mut patterns = Vec::with_capacity(self.patterns.len());
		for p in &self.patterns {
			patterns.push(p.build(vocabulary, interpretation, context, scope)?)
		}

		Ok(inference::rule::Hypothesis::new(patterns))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M> for Conclusion<M> {
	type Target = inference::rule::Conclusion<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		let mut patterns = Vec::with_capacity(self.statements.len());
		for p in &self.statements {
			patterns.push(p.build(vocabulary, interpretation, context, scope)?)
		}

		Ok(inference::rule::Conclusion::new(Vec::new(), patterns))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M>
	for MaybeTrustedStatement<M>
{
	type Target = MaybeTrusted<Signed<inference::rule::StatementPattern<I::Resource>>>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		Ok(MaybeTrusted::new(
			self.statement
				.build(vocabulary, interpretation, context, scope)?,
			*self.trust,
		))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M>
	for SignedStatement<M>
{
	type Target = Signed<inference::rule::StatementPattern<I::Resource>>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		let sign = if self.negative.is_some() {
			Sign::Negative
		} else {
			Sign::Positive
		};

		Ok(Signed(
			sign,
			self.statement
				.build(vocabulary, interpretation, context, scope)?,
		))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M> for Statement<M> {
	type Target = inference::rule::StatementPattern<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		match self {
			Self::Eq(a, b) => Ok(inference::rule::StatementPattern::Eq(
				a.build(vocabulary, interpretation, context, scope)?,
				b.build(vocabulary, interpretation, context, scope)?,
			)),
			Self::Pattern(p) => Ok(inference::rule::StatementPattern::Triple(p.build(
				vocabulary,
				interpretation,
				context,
				scope,
			)?)),
		}
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M>
	for SignedPattern<M>
{
	type Target = Signed<inferdf_core::Pattern<I::Resource>>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		let sign = if self.negative.is_some() {
			Sign::Negative
		} else {
			Sign::Positive
		};

		Ok(Signed(
			sign,
			self.pattern
				.build(vocabulary, interpretation, context, scope)?,
		))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScoped<V, I, M> for Pattern<M> {
	type Target = inferdf_core::Pattern<I::Resource>;

	fn build(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
	) -> BuildResult<Self::Target, M, I::Error> {
		Ok(inferdf_core::Pattern::new(
			self.subject
				.build(vocabulary, interpretation, context, scope)?,
			self.predicate
				.build(vocabulary, interpretation, context, scope)?,
			self.object
				.build(vocabulary, interpretation, context, scope)?,
		))
	}
}

impl<M: Clone, V: VocabularyMut, I: BuildInterpretation<V>> BuildScopedWith<V, I, M> for Expr {
	type Target = IdOrVar<I::Resource>;

	fn build_with(
		&self,
		vocabulary: &mut V,
		interpretation: &mut I,
		context: &mut Context,
		scope: &Scope,
		meta: &M,
	) -> BuildResult<Self::Target, M, I::Error> {
		match self {
			Self::IriRef(iri_ref) => {
				let iri = iri_ref.resolve(context, meta)?;
				let id = interpretation
					.interpret_owned_lexical_iri::<I::Error>(vocabulary, iri)
					.map_err(|e| Meta(Error::Interpretation(e), meta.clone()))?;
				Ok(IdOrVar::Id(id))
			}
			Self::Var(name) => Ok(IdOrVar::Var(scope.get(name).unwrap())),
		}
	}
}
