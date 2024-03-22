#[macro_export]
#[doc(hidden)]
macro_rules! unexpected_token {
	() => {};
}

/// Creates a new triple pattern.
///
/// The resulting value has type [`Pattern`](crate::Pattern).
#[macro_export]
macro_rules! pattern {
	// Parse a pattern.
	{
		@from ($($acc:tt)*) ? $id:ident $($rest:tt)*
	} => {
		$crate::pattern!(@from ($($acc)* $crate::pattern::ResourceOrVar::Var(
			$id
		),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) < $iri:literal > $($rest:tt)*
	} => {
		$crate::pattern!(@from ($($acc)* $crate::pattern::ResourceOrVar::Resource(
			<$crate::rdf_types::Term>::iri($crate::static_iref::iri!($iri).to_owned())
		),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) _ : $id:literal $($rest:tt)*
	} => {
		$crate::pattern!(@from ($($acc)* $crate::pattern::ResourceOrVar::Resource(
			<$crate::rdf_types::Term>::blank($crate::rdf_types::BlankIdBuf::from_suffix($id).unwrap())
		),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) $value:literal ^^ $ty:literal $($rest:tt)*
	} => {
		$crate::pattern!(@from ($($acc)* $crate::pattern::ResourceOrVar::Resource(
			<$crate::rdf_types::Term>::Literal($crate::rdf_types::Literal::new(
				$value.to_owned(),
				$crate::rdf_types::LiteralType::Any(
					$crate::static_iref::iri!($ty).to_owned()
				)
			))
		),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) $value:literal $($rest:tt)*
	} => {
		$crate::pattern!(@from ($($acc)* $crate::pattern::ResourceOrVar::Resource(
			<$crate::rdf_types::Term>::Literal($crate::rdf_types::Literal::new(
				$value.to_owned(),
				$crate::rdf_types::LiteralType::Any(
					$crate::rdf_types::XSD_STRING.to_owned()
				)
			))
		),) $($rest)*)
	};
	{
		@from ($($acc:tt)*)
	} => {
		$crate::rdf_types::Triple($($acc)*)
	};
	// Main rules.
	{
		! $($t:tt)*
	} => {
		$crate::Signed($crate::Sign::Negative, $crate::pattern!(@from () $($t)*))
	};
	{
		$($t:tt)*
	} => {
		$crate::Signed($crate::Sign::Positive, $crate::pattern!(@from () $($t)*))
	};
}

/// Creates a list of patterns.
///
/// The resulting value has type [`Vec<Pattern>`](crate::Pattern).
#[macro_export]
macro_rules! patterns {
	// Tokenize patterns.
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] ! $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* !] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] ? $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* ?] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] $i:ident $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* $i] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] < $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* <] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] > $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* >] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] _ $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* _] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] : $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* :] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] ^ $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* ^] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] $l:literal $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)*] [$($current)* $l] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] . $($rest:tt)*
	} => {
		$crate::patterns!(@tokenize [$($acc)* ( $($current)* )] [] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] []
	} => {
		$crate::patterns!(@from [] $($acc)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] $t:tt $($rest:tt)*
	} => {
		$crate::unexpected_token!($t)
	};
	// Parse a tokenized pattern list.
	{
		@from [$($acc:tt)*] ($($pattern:tt)*) $($rest:tt)*
	} => {
		$crate::patterns!(@from [$($acc)* $crate::pattern!($($pattern)*),] $($rest)*)
	};
	{
		@from [$($acc:tt)*]
	} => {
		vec![$($acc)*]
	};
	// Main rule.
	{
		$($patterns:tt)*
	} => {
		$crate::patterns!(@tokenize [] [] $($patterns)*)
	};
}

/// Creates a deduction rule.
#[macro_export]
macro_rules! rule {
	// Parse a conclusion.
	{
		@conclusion ($($offset:tt)*) { $($statements:tt)* }
	} => {
		$crate::rule!(@conclusion ($($offset)*) for { $($statements)* } )
	};
	{
		@conclusion ($($offset:tt)*) for $(?$id:ident),* { $($statements:tt)* }
	} => {
		{
			$crate::rule!(@bind ($($offset)*) $($id)*);
			$crate::rule::Conclusion::new(
				$crate::rule!(@count $($id)*),
				$crate::statements!($($statements)*)
			)
		}
	};
	// Count the number of tokens.
	{
		@count $($t:tt)*
	} => {
		$crate::rule!(@count_from (0) $($t)*)
	};
	{
		@count_from ($($n:tt)*) $first:tt $($rest:tt)*
	} => {
		$crate::rule!(@count_from ($($n)* + 1usize) $($rest)*)
	};
	{
		@count_from ($($n:tt)*)
	} => {
		$($n)*
	};
	// Bind variables.
	{
		@bind ($($n:tt)*) $first:tt $($rest:tt)*
	} => {
		let $first = $($n)*;
		$crate::rule!(@bind ($($n)* + 1usize) $($rest)*)
	};
	{
		@bind ($($n:tt)*)
	} => {};
	// Main rules
	{
		for $(?$id:ident),* { $($hypothesis:tt)* } => $($conclusion:tt)*
	} => {
		{
			$crate::rule!(@bind (0) $($id)*);
			$crate::Rule::new(
				$crate::rule!(@count $($id)*),
				$crate::rule::Hypothesis::new($crate::patterns!($($hypothesis)*)),
				$crate::rule!(@conclusion ($crate::rule!(@count $($id)*)) $($conclusion)*)
			)
		}
	};
	{
		{ $($hypothesis:tt)* } => $($conclusion:tt)*
	} => {
		$crate::rule!(for { $($hypothesis)* } => $($conclusion)*)
	};
}

/// Creates a list of statement expressions.
#[macro_export]
macro_rules! expressions {
	// Parse a list of expressions.
	{
		@from ($($acc:tt)*) ? $id:ident $($rest:tt)*
	} => {
		$crate::expressions!(@from ($($acc)* ($crate::expression!(? $id)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) < $iri:literal > $($rest:tt)*
	} => {
		$crate::expressions!(@from ($($acc)* ($crate::expression!(< $iri >)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) _ : $id:literal $($rest:tt)*
	} => {
		$crate::expressions!(@from ($($acc)* ($crate::expression!(_ : $id)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) $value:literal ^^ $ty:literal $($rest:tt)*
	} => {
		$crate::expressions!(@from ($($acc)* ($crate::expression!($value ^^ $ty)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) $value:literal $($rest:tt)*
	} => {
		$crate::expressions!(@from ($($acc)* ($crate::expression!($value)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) /$value:literal/ $($rest:tt)*
	} => {
		$crate::expressions!(@from ($($acc)* ($crate::expression!(/$value/)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) ($($g:tt)*) $($rest:tt)*
	} => {
		$crate::expressions!(@from ($($acc)* ($crate::expression!(($($g)*))),) $($rest)*)
	};
	{
		@from ($($acc:tt)*)
	} => {
		vec![$($acc)*]
	};
	// Main rule.
	{
		$($t:tt)*
	} => {
		$crate::expressions!(@from () $($t)*)
	}
}

/// Creates a statement expression.
#[macro_export]
macro_rules! expression {
	// Main rules.
	{
		? $id:ident
	} => {
		$crate::Expression::Resource($crate::pattern::ResourceOrVar::Var(
			$id
		))
	};
	{
		< $iri:literal >
	} => {
		$crate::Expression::Resource($crate::pattern::ResourceOrVar::Resource(
			<$crate::rdf_types::Term>::iri($crate::static_iref::iri!($iri).to_owned())
		))
	};
	{
		_ : $id:literal
	} => {
		$crate::Expression::Resource($crate::pattern::ResourceOrVar::Resource(
			<$crate::rdf_types::Term>::blank($crate::rdf_types::BlankIdBuf::from_suffix($id).unwrap())
		))
	};
	{
		$value:literal ^^ $ty:literal
	} => {
		$crate::Expression::Resource($crate::pattern::ResourceOrVar::Resource(
			<$crate::rdf_types::Term>::Literal($crate::rdf_types::Literal::new(
				$value.to_owned(),
				$crate::rdf_types::LiteralType::Any(
					$crate::static_iref::iri!($ty).to_owned()
				)
			))
		))
	};
	{
		$value:literal
	} => {
		$crate::Expression::Literal($value.into())
	};
	{
		/$value:literal/
	} => {
		$crate::Expression::Literal($crate::expression::Literal::Regex(
			$crate::expression::Regex::new($value).unwrap()
		))
	};
	{
		(!= $($args:tt)*)
	} => {
		$crate::Expression::Call(
			$crate::expression::BuiltInFunction::Compare(
				$crate::expression::ComparisonOperator::Ne
			),
			$crate::expressions!($($args)*)
		)
	};
	{
		(= $($args:tt)*)
	} => {
		$crate::Expression::Call(
			$crate::expression::BuiltInFunction::Compare(
				$crate::expression::ComparisonOperator::Eq
			),
			$crate::expressions!($($args)*)
		)
	};
	{
		(>= $($args:tt)*)
	} => {
		$crate::Expression::Call(
			$crate::expression::BuiltInFunction::Compare(
				$crate::expression::ComparisonOperator::Geq
			),
			$crate::expressions!($($args)*)
		)
	};
	{
		(> $($args:tt)*)
	} => {
		$crate::Expression::Call(
			$crate::expression::BuiltInFunction::Compare(
				$crate::expression::ComparisonOperator::Gt
			),
			$crate::expressions!($($args)*)
		)
	};
	{
		(<= $($args:tt)*)
	} => {
		$crate::Expression::Call(
			$crate::expression::BuiltInFunction::Compare(
				$crate::expression::ComparisonOperator::Leq
			),
			$crate::expressions!($($args)*)
		)
	};
	{
		(< $($args:tt)*)
	} => {
		$crate::Expression::Call(
			$crate::expression::BuiltInFunction::Compare(
				$crate::expression::ComparisonOperator::Lt
			),
			$crate::expressions!($($args)*)
		)
	};
	{
		(matches $($args:tt)*)
	} => {
		$crate::Expression::Call(
			$crate::expression::BuiltInFunction::Matches,
			$crate::expressions!($($args)*)
		)
	};
}

/// Creates a triple statement.
#[macro_export]
macro_rules! statement {
	// Parse a list of expressions.
	{
		@from ($($acc:tt)*) ? $id:ident $($rest:tt)*
	} => {
		$crate::statement!(@from ($($acc)* ($crate::expression!(? $id)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) < $iri:literal > $($rest:tt)*
	} => {
		$crate::statement!(@from ($($acc)* ($crate::expression!(< $iri >)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) _ : $id:literal $($rest:tt)*
	} => {
		$crate::statement!(@from ($($acc)* ($crate::expression!(_ : $id)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) $value:literal ^^ $ty:literal $($rest:tt)*
	} => {
		$crate::statement!(@from ($($acc)* ($crate::expression!($value ^^ $ty)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) $value:literal $($rest:tt)*
	} => {
		$crate::statement!(@from ($($acc)* ($crate::expression!($value)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) /$value:literal/ $($rest:tt)*
	} => {
		$crate::statement!(@from ($($acc)* ($crate::expression!(/$value/)),) $($rest)*)
	};
	{
		@from ($($acc:tt)*) ($($g:tt)*) $($rest:tt)*
	} => {
		$crate::statement!(@from ($($acc)* ($crate::expression!(($($g)*))),) $($rest)*)
	};
	{
		@from (($($s:tt)*),) = $($rest:tt)*
	} => {
		$crate::TripleStatement::Eq(
			$($s)*,
			$crate::expression!($($rest)*)
		)
	};
	{
		@from (($($s:tt)*), ($($p:tt)*), ($($o:tt)*),)
	} => {
		$crate::TripleStatement::Triple($crate::rdf_types::Triple($($s)*, $($p)*, $($o)*))
	};
	{
		@from (($($s:tt)*),)
	} => {
		$crate::TripleStatement::True($($s)*)
	};
	{
		@from $acc:tt $t:tt $($rest:tt)*
	} => {
		$crate::unexpected_token!($t)
	};
	// Main rules.
	{
		! $($t:tt)*
	} => {
		$crate::Signed($crate::Sign::Negative, $crate::statement!(@from () $($t)*))
	};
	{
		$($t:tt)*
	} => {
		$crate::Signed($crate::Sign::Positive, $crate::statement!(@from () $($t)*))
	};
}

/// Creates a list of triple statements.
#[macro_export]
macro_rules! statements {
	// Tokenize statements.
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] ! $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* !] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] = $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* =] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] ? $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* ?] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] $i:ident $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* $i] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] < $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* <] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] > $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* >] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] _ $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* _] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] : $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* :] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] ^ $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* ^] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] / $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* /] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] $l:literal $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* $l] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] ($($g:tt)*) $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)*] [$($current)* ($($g)*)] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] . $($rest:tt)*
	} => {
		$crate::statements!(@tokenize [$($acc)* ($($current)*)] [] $($rest)*)
	};
	{
		@tokenize [$($acc:tt)*] []
	} => {
		$crate::statements!(@from [] $($acc)*)
	};
	{
		@tokenize [$($acc:tt)*] [$($current:tt)*] $t:tt $($rest:tt)*
	} => {
		$crate::unexpected_token!($t)
	};
	// Parse a tokenized statement list.
	{
		@from [$($acc:tt)*] ($($statement:tt)*) $($rest:tt)*
	} => {
		$crate::statements!(@from [$($acc)* $crate::statement!($($statement)*),] $($rest)*)
	};
	{
		@from [$($acc:tt)*]
	} => {
		vec![$($acc)*]
	};
	// Main rule.
	{
		$($stm:tt)*
	} => {
		$crate::statements!(@tokenize [] [] $($stm)*)
	};
}

#[cfg(test)]
mod tests {
	use rdf_types::Triple;

	use crate::{pattern::ResourceOrVar, rule::TripleStatementPattern, Signed};

	#[test]
	fn statement_macro() {
		let x = 0;
		let _: Signed<TripleStatementPattern> =
			statement!(?x <"http://example.org/#foo"> "hello"^^"http://example.org/#test");
	}

	#[test]
	fn statements_macro() {
		let x = 0;
		let y = 1;
		let _: Vec<Signed<TripleStatementPattern>> = statements! [
			?x <"http://example.org/#foo"> "hello"^^"http://example.org/#test" .
			?x = ?y .
			(= ?x ?y) .
		];
	}

	#[test]
	fn patterns_macro() {
		let x = 0;
		let y = 1;
		let _: Vec<Signed<Triple<ResourceOrVar>>> = patterns! [
			?x <"http://example.org/#foo"> "hello"^^"http://example.org/#test" .
			?y <"http://example.org/#bar"> "hello" .
		];
	}

	#[test]
	fn rule_macro() {
		let _ = rule! {
			for ?a, ?b {
				?a <"http://example.org/#foo"> "hello"^^"http://example.org/#test" .
				?b <"http://example.org/#bar"> "hello" .
			} => for ?c, ?d {
				?c <"http://example.org/#foo"> "hello"^^"http://example.org/#test" .
				?d <"http://example.org/#check"> (= ?a <"http://hey.org">) .
				_:"foo" <"http://example.org/#regex"> /"[a-z]*"/ .
				_:"bar" <"http://example.org/#string"> "some string" .
				?a = ?b .
				(= ?a ?b) .
			}
		};
	}
}
