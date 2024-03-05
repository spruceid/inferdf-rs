use std::hash::Hash;

use educe::Educe;
use rdf_types::{
	interpretation::{LiteralInterpretationMut, ReverseTermInterpretation},
	InterpretationMut, VocabularyMut,
};
use xsd_types::{ParseXsd, XSD_BOOLEAN};

use crate::{
	expression::{self, Eval},
	pattern::{ApplySubstitution, PatternSubstitution},
	rule::TripleStatementPattern,
	Entailment, FallibleSignedPatternMatchingDataset, Reason, Sign, Signed,
	SignedPatternMatchingDataset, TripleStatement, Validation, ValidationError,
};

use super::{DeductionInstance, SubDeductionInstance};

#[derive(Educe)]
#[educe(Default)]
pub struct Deduction<'r, T>(Vec<SubDeduction<'r, T>>);

impl<'r, T> Deduction<'r, T> {
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	pub fn push(&mut self, s: SubDeduction<'r, T>) {
		self.0.push(s)
	}

	pub fn merge_with(&mut self, other: Self) {
		self.0.extend(other.0)
	}

	/// Evaluates the expressions in the deducted statements.
	pub fn eval<V, I>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<DeductionInstance<'r, T>, expression::Error>
	where
		T: Clone + PartialEq,
		V: VocabularyMut,
		V::Iri: PartialEq,
		I: InterpretationMut<V, Resource = T>
			+ LiteralInterpretationMut<V::Literal>
			+ ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
		I::Resource: PartialEq,
	{
		Ok(DeductionInstance(
			self.0
				.into_iter()
				.map(|s| s.eval(vocabulary, interpretation))
				.collect::<Result<_, _>>()?,
		))
	}
}

impl<'r, T: Clone + Eq + Hash> Deduction<'r, T> {
	pub fn validate<V, I, D>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
		dataset: &D,
	) -> Result<Validation<T>, expression::Error>
	where
		V: VocabularyMut,
		V::Iri: PartialEq,
		I: InterpretationMut<V, Resource = T>
			+ LiteralInterpretationMut<V::Literal>
			+ ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
		D: SignedPatternMatchingDataset<Resource = T>,
	{
		self.try_validate(vocabulary, interpretation, dataset)
			.map_err(Into::into)
	}

	pub fn try_validate<V, I, D>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
		dataset: &D,
	) -> Result<Validation<T>, ValidationError<D::Error>>
	where
		V: VocabularyMut,
		V::Iri: PartialEq,
		I: InterpretationMut<V, Resource = T>
			+ LiteralInterpretationMut<V::Literal>
			+ ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
		D: FallibleSignedPatternMatchingDataset<Resource = T>,
	{
		let deduction = self
			.eval(vocabulary, interpretation)
			.map_err(ValidationError::Expression)?;
		for group in deduction {
			for Signed(sign, stm) in group.statements {
				match stm {
					TripleStatement::Triple(triple) => {
						if !dataset
							.try_contains_signed_triple(Signed(sign, triple.as_ref()))
							.map_err(ValidationError::Dataset)?
						{
							return Ok(Validation::Invalid(Reason::MissingTriple(Signed(
								sign, triple,
							))));
						}
					}
					TripleStatement::Eq(a, b) => match sign {
						Sign::Positive => {
							if a != b {
								return Ok(Validation::Invalid(Reason::NotEq(a, b)));
							}
						}
						Sign::Negative => {
							if a == b {
								return Ok(Validation::Invalid(Reason::NotNe(a, b)));
							}
						}
					},
					TripleStatement::True(r) => {
						let expected = sign.is_positive();

						let mut found = false;
						for l in interpretation.literals_of(&r) {
							let literal = vocabulary.literal(l).unwrap();
							let type_ = literal.type_.as_lexical_type_ref_with(vocabulary);
							if type_.is_iri(XSD_BOOLEAN) {
								match xsd_types::Boolean::parse_xsd(&literal.value) {
									Ok(xsd_types::Boolean(b)) => {
										if b == expected {
											found = true;
										}
									}
									Err(_) => {
										return Err(ValidationError::Expression(
											expression::Error::InvalidLiteral,
										))
									}
								}
							}
						}

						if !found {
							return Ok(Validation::Invalid(if expected {
								Reason::NotTrue(r.clone())
							} else {
								Reason::NotFalse(r.clone())
							}));
						}
					}
				}
			}
		}

		Ok(Validation::Ok)
	}
}

pub enum EvalError<I> {
	Expression(expression::Error),
	Interpretation(I),
}

impl<'r, T> From<SubDeduction<'r, T>> for Deduction<'r, T> {
	fn from(value: SubDeduction<'r, T>) -> Self {
		Self(vec![value])
	}
}

/// Deduced statements with a common cause.
pub struct SubDeduction<'r, T> {
	/// Rule and variable substitution triggering this deduction.
	pub entailment: Entailment<'r, T>,

	/// Deduced statements.
	pub statements: Vec<Signed<TripleStatementPattern<T>>>,
}

impl<'r, T> SubDeduction<'r, T> {
	pub fn new(entailment: Entailment<'r, T>) -> Self {
		Self {
			entailment,
			statements: Vec::new(),
		}
	}

	pub fn insert(&mut self, statement: Signed<TripleStatementPattern<T>>) {
		self.statements.push(statement)
	}

	pub fn merge_with(&mut self, other: Deduction<T>) {
		for s in other.0 {
			self.statements.extend(s.statements)
		}
	}

	/// Evaluates the expressions in the deducted statements.
	pub fn eval<V, I>(
		self,
		vocabulary: &mut V,
		interpretation: &mut I,
	) -> Result<SubDeductionInstance<'r, T>, expression::Error>
	where
		T: Clone + PartialEq,
		V: VocabularyMut,
		V::Iri: PartialEq,
		I: InterpretationMut<V, Resource = T>
			+ LiteralInterpretationMut<V::Literal>
			+ ReverseTermInterpretation<Iri = V::Iri, BlankId = V::BlankId, Literal = V::Literal>,
		I::Resource: PartialEq,
	{
		let rule = self.entailment.rule;
		let mut substitution = PatternSubstitution::new();
		for i in 0..rule.conclusion.variables {
			let x = i + rule.variables;
			substitution.bind(x, interpretation.new_resource(vocabulary));
		}

		let mut statements = Vec::with_capacity(self.statements.len());
		for stm in self.statements {
			statements.push(
				stm.apply_substitution(&substitution)
					.unwrap()
					.eval_and_instantiate(vocabulary, interpretation)?,
			);
		}

		Ok(SubDeductionInstance {
			entailment: self.entailment,
			statements,
		})
	}
}
