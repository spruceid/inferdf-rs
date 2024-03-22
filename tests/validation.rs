use inferdf::{rule, Validation};
use rdf_types::{dataset::IndexedBTreeGraph, grdf_triples};

#[test]
fn validate_comparison() {
	let dataset: IndexedBTreeGraph = grdf_triples![
		_:"0" <"https://example.org/#age"> "21"^^"http://www.w3.org/2001/XMLSchema#int" .
	]
	.into_iter()
	.collect();

	let rule = rule! {
		for ?x, ?age {
			?x <"https://example.org/#age"> ?age .
		} => {
			(>= ?age 18) .
		}
	};

	assert_eq!(rule.validate(&dataset).unwrap(), Validation::Ok);
}

#[test]
fn validate_regex() {
	let dataset: IndexedBTreeGraph = grdf_triples![
		_:"0" <"https://example.org/#email"> "user@domain.com" .
	]
	.into_iter()
	.collect();

	let rule = rule! {
		for ?x, ?email {
			?x <"https://example.org/#email"> ?email .
		} => {
			(matches /"^[\\w\\-\\.]+@([\\w-]+\\.)+[\\w-]{2,}$"/ ?email) .
		}
	};

	assert_eq!(rule.validate(&dataset).unwrap(), Validation::Ok);
}

#[test]
fn validation_failure() {
	let dataset: IndexedBTreeGraph = grdf_triples![
		_:"0" <"https://example.org/#age"> "12"^^"http://www.w3.org/2001/XMLSchema#int" .
	]
	.into_iter()
	.collect();

	let rule = rule! {
		for ?x, ?age {
			?x <"https://example.org/#age"> ?age .
		} => {
			(>= ?age 18) .
		}
	};

	assert!(rule.validate(&dataset).unwrap().is_invalid());
}
