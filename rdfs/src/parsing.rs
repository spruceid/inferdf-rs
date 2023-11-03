use core::fmt;

use decoded_char::{DecodedChar, DecodedChars};
use inferdf::semantics::Trust;
use iref::{iri::InvalidIriRef, InvalidIri, IriBuf, IriRefBuf};
use locspan::{Meta, Span};

use crate::{
	Colon, Conclusion, Document, Documentation, Dot, Exists, Expr, ForAll, Formula, Group,
	Hypothesis, IriReference, Item, Keyword, MaybeTrustedStatement, Pattern, Prefix, PrefixBinding,
	Rule, SignedPattern, SignedStatement, Statement, VarIdent,
};

#[derive(Debug)]
pub struct Unexpected(Option<char>);

impl From<Option<char>> for Unexpected {
	fn from(value: Option<char>) -> Self {
		Self(value)
	}
}

impl fmt::Display for Unexpected {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self.0 {
			Some(c) => write!(f, "unexpected char `{c}`"),
			None => f.write_str("unexpected end of stream"),
		}
	}
}

impl std::error::Error for Unexpected {}

#[derive(Debug, thiserror::Error)]
pub enum Error<E> {
	#[error(transparent)]
	Input(E),

	#[error(transparent)]
	InvalidIriRef(InvalidIriRef<String>),

	#[error(transparent)]
	InvalidIri(InvalidIri<String>),

	#[error(transparent)]
	Unexpected(Unexpected),

	#[error("unexpected keyword `{0}`")]
	UnexpectedKeyword(Keyword),

	#[error("unknown keyword `{0}`")]
	UnknownKeyword(String),
}

pub type ParseResult<T, M, E = std::convert::Infallible> = Result<Meta<T, M>, Meta<Error<E>, M>>;

pub type MetaError<M, E = std::convert::Infallible> = Meta<Error<E>, M>;

pub struct Parser<'a, I, M> {
	input: I,
	pending1: Option<DecodedChar>,
	pending2: Option<DecodedChar>,
	last_position: usize,
	position: usize,
	f: Box<dyn 'a + FnMut(Span) -> M>,
}

impl<'a, I, M> Parser<'a, I, M> {
	pub fn new(input: I, f: impl 'a + FnMut(Span) -> M) -> Self {
		Self {
			input,
			pending1: None,
			pending2: None,
			last_position: 0,
			position: 0,
			f: Box::new(f),
		}
	}

	pub fn last_char_span(&self) -> Span {
		Span::new(self.last_position, self.position)
	}

	pub fn position(&self) -> usize {
		self.position
	}

	pub fn last_char_metadata(&mut self) -> M {
		self.build_metadata(self.last_char_span())
	}

	pub fn build_metadata(&mut self, span: Span) -> M {
		(self.f)(span)
	}
}

impl<'a, E, I: Iterator<Item = Result<DecodedChar, E>>, M> Parser<'a, I, M> {
	fn pull_next(&mut self) -> Result<Option<DecodedChar>, MetaError<M, E>> {
		self.input.next().transpose().map_err(|e| {
			Meta(
				Error::Input(e),
				(self.f)(Span::new(self.position, self.position)),
			)
		})
	}

	fn pull_pending2(&mut self) -> Result<Option<DecodedChar>, MetaError<M, E>> {
		match self.pending2.take() {
			Some(c) => Ok(Some(c)),
			None => self.pull_next(),
		}
	}

	fn pull_pending1(&mut self) -> Result<Option<DecodedChar>, MetaError<M, E>> {
		match self.pending1.take() {
			Some(c) => {
				self.pending1 = self.pending2.take();
				Ok(Some(c))
			}
			None => self.pull_next(),
		}
	}

	pub fn peek2(&mut self) -> Result<Option<char>, MetaError<M, E>> {
		if self.pending2.is_none() {
			self.pending2 = self.pull_next()?
		}

		Ok(self.pending2.map(DecodedChar::into_char))
	}

	pub fn peek(&mut self) -> Result<Option<char>, MetaError<M, E>> {
		if self.pending1.is_none() {
			self.pending1 = self.pull_pending2()?
		}

		Ok(self.pending1.map(DecodedChar::into_char))
	}

	pub fn next_char(&mut self) -> Result<Option<char>, MetaError<M, E>> {
		match self.pull_pending1()? {
			Some(c) => {
				self.last_position = self.position;
				self.position += c.len();
				Ok(Some(c.into_char()))
			}
			None => Ok(None),
		}
	}

	pub fn expect(&mut self) -> Result<char, MetaError<M, E>> {
		self.next_char()?.ok_or_else(|| {
			Meta(
				Error::Unexpected(None.into()),
				(self.f)(Span::new(self.position, self.position)),
			)
		})
	}

	pub fn skip_whitespaces(&mut self) -> Result<usize, MetaError<M, E>> {
		while let Some(c) = self.peek()? {
			if c.is_whitespace() {
				self.next_char()?;
			} else if c == '#' && !matches!(self.peek2()?, Some('#' | '!')) {
				while let Some(c) = self.next_char()? {
					if c == '\n' {
						break;
					}
				}
			} else {
				break;
			}
		}

		Ok(self.position)
	}
}

pub trait Parse<M>: Sized {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E>;

	fn parse_str(input: &str, f: impl FnMut(Span) -> M) -> ParseResult<Self, M> {
		Self::try_parse(input.decoded_chars().map(Ok), f)
	}

	fn try_parse<E, I: IntoIterator<Item = Result<DecodedChar, E>>>(
		input: I,
		f: impl FnMut(Span) -> M,
	) -> ParseResult<Self, M, E> {
		let mut parser = Parser::new(input.into_iter(), f);
		Self::parse_with(&mut parser)
	}
}

impl<M> Parse<M> for Document<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		let doc = parse_documentation(parser, '!')?.into_value();
		let mut items = Vec::new();

		parser.skip_whitespaces()?;
		while let Some(c) = parser.peek()? {
			debug_assert!(!c.is_whitespace());
			let item = Item::parse_with(parser)?;
			items.push(item);
			parser.skip_whitespaces()?;
		}

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(Document { doc, items }, meta))
	}
}

impl<M> Parse<M> for IriReference {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		match parser.peek()? {
			Some('<') => {
				let Meta(iri_ref, meta) = IriRefBuf::parse_with(parser)?;
				Ok(Meta(Self::Expanded(iri_ref), meta))
			}
			_ => {
				let mut buffer = String::new();

				while let Some(c) = parser.peek()? {
					if c.is_whitespace() {
						break;
					} else {
						buffer.push(parser.next_char()?.unwrap())
					}
				}

				let span = Span::new(start, parser.position());
				let meta = parser.build_metadata(span);

				match IriRefBuf::new(buffer) {
					Ok(iri_ref) => Ok(Meta(Self::Compact(iri_ref), meta)),
					Err(e) => Err(Meta(Error::InvalidIriRef(e), meta)),
				}
			}
		}
	}
}

fn is_iri_ref_successor(c: Option<char>) -> bool {
	match c {
		Some(c) => c.is_whitespace(),
		None => true,
	}
}

impl<M> Parse<M> for IriRefBuf {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;

		match parser.next_char()? {
			Some('<') => {
				let mut buffer = String::new();

				loop {
					match parser.expect()? {
						'>' if is_iri_ref_successor(parser.peek()?) => break,
						c => buffer.push(c),
					}
				}

				let span = Span::new(start, parser.position());
				let meta = parser.build_metadata(span);

				match IriRefBuf::new(buffer) {
					Ok(iri_ref) => Ok(Meta(iri_ref, meta)),
					Err(e) => Err(Meta(Error::InvalidIriRef(e), meta)),
				}
			}
			u => Err(Meta(
				Error::Unexpected(u.into()),
				parser.last_char_metadata(),
			)),
		}
	}
}

impl<M> Parse<M> for IriBuf {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;

		match parser.next_char()? {
			Some('<') => {
				let mut buffer = String::new();

				loop {
					match parser.expect()? {
						'>' if is_iri_ref_successor(parser.peek()?) => break,
						c => buffer.push(c),
					}
				}

				let span = Span::new(start, parser.position());
				let meta = parser.build_metadata(span);

				match IriBuf::new(buffer) {
					Ok(iri) => Ok(Meta(iri, meta)),
					Err(e) => Err(Meta(Error::InvalidIri(e), meta)),
				}
			}
			u => Err(Meta(
				Error::Unexpected(u.into()),
				parser.last_char_metadata(),
			)),
		}
	}
}

impl<M> Parse<M> for Keyword {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		let mut buffer = String::new();

		match parser.next_char()? {
			Some(c) if c.is_alphabetic() => {
				buffer.push(c);
				while let Some(c) = parser.peek()? {
					if c.is_whitespace() {
						break;
					} else {
						buffer.push(parser.next_char()?.unwrap())
					}
				}
			}
			u => {
				return Err(Meta(
					Error::Unexpected(u.into()),
					parser.last_char_metadata(),
				))
			}
		}

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		match buffer.as_str() {
			"base" => Ok(Meta(Self::Base, meta)),
			"prefix" => Ok(Meta(Self::Prefix, meta)),
			"group" => Ok(Meta(Self::Group, meta)),
			"rule" => Ok(Meta(Self::Rule, meta)),
			"forall" => Ok(Meta(Self::ForAll, meta)),
			"exists" => Ok(Meta(Self::Exists, meta)),
			u => Err(Meta(Error::UnknownKeyword(u.to_owned()), meta)),
		}
	}
}

fn parse_delimited_list<T: Parse<M>, M, E, I: Iterator<Item = Result<DecodedChar, E>>>(
	parser: &mut Parser<'_, I, M>,
) -> ParseResult<Vec<Meta<T, M>>, M, E> {
	let start = parser.skip_whitespaces()?;

	match parser.next_char()? {
		Some('{') => {
			let mut items = Vec::new();

			let end = loop {
				parser.skip_whitespaces()?;
				match parser.peek()? {
					Some('}') => {
						parser.next_char()?;
						break parser.position();
					}
					_ => items.push(T::parse_with(parser)?),
				}
			};

			let span = Span::new(start, end);
			let meta = parser.build_metadata(span);

			Ok(Meta(items, meta))
		}
		u => Err(Meta(
			Error::Unexpected(u.into()),
			parser.last_char_metadata(),
		)),
	}
}

fn parse_documentation<M, E, I: Iterator<Item = Result<DecodedChar, E>>>(
	parser: &mut Parser<'_, I, M>,
	prefix: char,
) -> ParseResult<Documentation, M, E> {
	let start = parser.skip_whitespaces()?;

	let mut lines = Vec::new();
	while let Some(c) = parser.peek()? {
		if c == '#' && parser.peek2()? == Some(prefix) {
			parser.next_char()?;
			parser.next_char()?;

			let mut buffer = String::new();
			while let Some(c) = parser.next_char()? {
				if c == '\n' {
					break;
				}

				buffer.push(c);
			}

			lines.push(buffer);
			parser.skip_whitespaces()?;
		} else {
			break;
		}
	}

	let span = Span::new(start, parser.position());
	let meta = parser.build_metadata(span);

	Ok(Meta(Documentation { lines }, meta))
}

impl<M> Parse<M> for Documentation {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		parse_documentation(parser, '#')
	}
}

impl<M> Parse<M> for Prefix {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		let mut buffer = String::new();

		match parser.next_char()? {
			Some(c) if c.is_alphabetic() => {
				buffer.push(c);
				while let Some(c) = parser.peek()? {
					if c.is_alphanumeric() || matches!(c, '+' | '-' | '.') {
						buffer.push(parser.next_char()?.unwrap())
					} else {
						break;
					}
				}
			}
			u => {
				return Err(Meta(
					Error::Unexpected(u.into()),
					parser.last_char_metadata(),
				))
			}
		}

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(Self(buffer), meta))
	}
}

impl<M> Parse<M> for Item<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let doc = Documentation::parse_with(parser)?.into_value();
		let start = parser.skip_whitespaces()?;
		match Keyword::parse_with(parser)? {
			Meta(Keyword::Base, _) => {
				let Meta(iri, meta) = IriBuf::parse_with(parser)?;
				Ok(Meta(Self::Base(iri), meta))
			}
			Meta(Keyword::Prefix, _) => {
				let binding = PrefixBinding {
					prefix: Prefix::parse_with(parser)?,
					colon: Colon::parse_with(parser)?,
					iri: IriRefBuf::parse_with(parser)?,
				};

				let span = Span::new(start, parser.position());
				let meta = parser.build_metadata(span);

				Ok(Meta(Self::Prefix(binding), meta))
			}
			Meta(Keyword::Group, _) => {
				let group = Group {
					id: IriReference::parse_with(parser)?,
					doc,
					items: parse_delimited_list(parser)?,
				};

				let span = Span::new(start, parser.position());
				let meta = parser.build_metadata(span);

				Ok(Meta(Self::Group(group), meta))
			}
			Meta(Keyword::Rule, _) => {
				let id = IriReference::parse_with(parser)?;
				let formula = Formula::parse_with(parser)?;

				let rule = Rule { id, doc, formula };

				let span = Span::new(start, parser.position());
				let meta = parser.build_metadata(span);

				Ok(Meta(Self::Rule(rule), meta))
			}
			Meta(other, meta) => Err(Meta(Error::UnexpectedKeyword(other), meta)),
		}
	}
}

impl<M> Parse<M> for Formula<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		match parser.peek()? {
			Some('{') => {
				let hypothesis = Hypothesis::parse_with(parser)?;

				parser.skip_whitespaces()?;
				let conclusion = match parser.next_char()? {
					Some('=') => match parser.next_char()? {
						Some('>') => {
							let Meta(conclusion, meta) = Conclusion::parse_with(parser)?;
							Meta(Self::Conclusion(conclusion), meta)
						}
						other => {
							return Err(Meta(
								Error::Unexpected(other.into()),
								parser.last_char_metadata(),
							))
						}
					},
					other => {
						return Err(Meta(
							Error::Unexpected(other.into()),
							parser.last_char_metadata(),
						))
					}
				};

				let span = Span::new(start, parser.position());
				Ok(Meta(
					Self::Exists(Exists {
						variables: Vec::new(),
						hypothesis,
						inner: Box::new(conclusion),
					}),
					parser.build_metadata(span),
				))
			}
			Some('=') => {
				parser.next_char()?;
				match parser.next_char()? {
					Some('>') => {
						let Meta(conclusion, meta) = Conclusion::parse_with(parser)?;
						Ok(Meta(Self::Conclusion(conclusion), meta))
					}
					other => Err(Meta(
						Error::Unexpected(other.into()),
						parser.last_char_metadata(),
					)),
				}
			}
			other => {
				let span = parser.last_char_span();
				match Keyword::parse_with(parser)? {
					Meta(Keyword::ForAll, _) => {
						parser.skip_whitespaces()?;

						let mut variables = Vec::new();
						while let Some('?') = parser.peek()? {
							variables.push(VarIdent::parse_with(parser)?);
							parser.skip_whitespaces()?;
						}

						let constraints = Hypothesis::parse_with(parser)?;
						let inner = Formula::parse_with(parser)?;
						let span = Span::new(start, parser.position());

						Ok(Meta(
							Self::ForAll(ForAll {
								variables,
								constraints,
								inner: Box::new(inner),
							}),
							parser.build_metadata(span),
						))
					}
					Meta(Keyword::Exists, _) => {
						parser.skip_whitespaces()?;

						let mut variables = Vec::new();
						while let Some('?') = parser.peek()? {
							variables.push(VarIdent::parse_with(parser)?);
							parser.skip_whitespaces()?;
						}

						let hypothesis = Hypothesis::parse_with(parser)?;
						let inner = Formula::parse_with(parser)?;
						let span = Span::new(start, parser.position());

						Ok(Meta(
							Self::Exists(Exists {
								variables,
								hypothesis,
								inner: Box::new(inner),
							}),
							parser.build_metadata(span),
						))
					}
					_ => Err(Meta(
						Error::Unexpected(other.into()),
						parser.build_metadata(span),
					)),
				}
			}
		}
	}
}

impl<M> Parse<M> for Hypothesis<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let Meta(patterns, meta) = parse_delimited_list(parser)?;
		Ok(Meta(Self { patterns }, meta))
	}
}

impl<M> Parse<M> for Conclusion<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let Meta(statements, meta) = parse_delimited_list(parser)?;
		Ok(Meta(Self { statements }, meta))
	}
}

impl<M> Parse<M> for VarIdent {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		match parser.next_char()? {
			Some('?') => {
				let mut buffer = String::new();
				match parser.next_char()? {
					Some(c) if c.is_alphabetic() => {
						buffer.push(c);
						while let Some(c) = parser.peek()? {
							if c.is_alphanumeric() {
								buffer.push(parser.next_char()?.unwrap())
							} else {
								break;
							}
						}

						let span = Span::new(start, parser.position());
						let meta = parser.build_metadata(span);
						Ok(Meta(Self(buffer), meta))
					}
					u => Err(Meta(
						Error::Unexpected(u.into()),
						parser.last_char_metadata(),
					)),
				}
			}
			e => Err(Meta(
				Error::Unexpected(e.into()),
				parser.last_char_metadata(),
			)),
		}
	}
}

impl<M> Parse<M> for Expr {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		parser.skip_whitespaces()?;
		match parser.peek()? {
			Some('?') => {
				let Meta(x, meta) = VarIdent::parse_with(parser)?;
				Ok(Meta(Self::Var(x), meta))
			}
			_ => {
				let Meta(iri_ref, meta) = IriReference::parse_with(parser)?;
				Ok(Meta(Self::IriRef(iri_ref), meta))
			}
		}
	}
}

impl<M> Parse<M> for SignedPattern<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		let negative = match parser.peek()? {
			Some('!') => {
				parser.next_char()?;
				Some(parser.last_char_metadata())
			}
			_ => None,
		};

		let pattern = Pattern::parse_with(parser)?;
		Dot::parse_with(parser)?;

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(Self { negative, pattern }, meta))
	}
}

impl<M> Parse<M> for Dot {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		parser.skip_whitespaces()?;
		match parser.next_char()? {
			Some('.') => Ok(Meta(Self, parser.last_char_metadata())),
			u => Err(Meta(
				Error::Unexpected(u.into()),
				parser.last_char_metadata(),
			)),
		}
	}
}

impl<M> Parse<M> for Pattern<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;

		let pattern = Self {
			subject: Expr::parse_with(parser)?,
			predicate: Expr::parse_with(parser)?,
			object: Expr::parse_with(parser)?,
		};

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(pattern, meta))
	}
}

impl<M> Parse<M> for Statement<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		let subject = Expr::parse_with(parser)?;

		parser.skip_whitespaces()?;
		let statement = match parser.peek()? {
			Some('=') => {
				parser.next_char()?;
				let object = Expr::parse_with(parser)?;
				Self::Eq(subject, object)
			}
			_ => {
				let predicate = Expr::parse_with(parser)?;
				let object = Expr::parse_with(parser)?;
				Self::Pattern(Pattern {
					subject,
					predicate,
					object,
				})
			}
		};

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(statement, meta))
	}
}

impl<M> Parse<M> for SignedStatement<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;
		let negative = match parser.peek()? {
			Some('!') => {
				parser.next_char()?;
				Some(parser.last_char_metadata())
			}
			_ => None,
		};

		let pattern = Statement::parse_with(parser)?;

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(
			Self {
				negative,
				statement: pattern,
			},
			meta,
		))
	}
}

impl<M> Parse<M> for MaybeTrustedStatement<M> {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;

		let pattern = Self {
			statement: SignedStatement::parse_with(parser)?,
			trust: Trust::parse_with(parser)?,
		};

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(pattern, meta))
	}
}

impl<M> Parse<M> for Trust {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;

		let t = match parser.next_char()? {
			Some('.') => Trust::Trusted,
			Some('!') => Trust::Untrusted,
			u => {
				return Err(Meta(
					Error::Unexpected(u.into()),
					parser.last_char_metadata(),
				))
			}
		};

		let span = Span::new(start, parser.position());
		let meta = parser.build_metadata(span);

		Ok(Meta(t, meta))
	}
}

impl<M> Parse<M> for Colon {
	fn parse_with<E, I: Iterator<Item = Result<DecodedChar, E>>>(
		parser: &mut Parser<'_, I, M>,
	) -> ParseResult<Self, M, E> {
		let start = parser.skip_whitespaces()?;

		match parser.next_char()? {
			Some(':') => {
				let span = Span::new(start, parser.position());
				let meta = parser.build_metadata(span);
				Ok(Meta(Self, meta))
			}
			u => Err(Meta(
				Error::Unexpected(u.into()),
				parser.last_char_metadata(),
			)),
		}
	}
}
