# Compositional Interpretations

## Semantics extension

### Constraints

Simple constraints
  - Equality: `?x = ?y`
  - Inequality: `?x != ?y`
  - Statement: `(?x, ?y, ?x)`


- Implication: `(?x, owl:equiv, ?y) & (?x, type, class) & (?y, type, class) => !(?x, type, class) & !(?y, type, class) & (forall (?a, type, ?x) <=> (?a, type, ?y))`



`(?x, domain, ?y) & (?u, ?x, ?v) => !(?u, type, ?y)`


`(?x, owl:equiv, ?y) => (forall (?a, type, ?x) <=> (?a, type, ?y))`

- Inclusion: `?x : P(?y)`
- Relation: `(?x, ?y) : P(?z)`
- Implication: `A => B`

Rules: `forall x. (?x, )`

### Interpretation

Maps each name (IRI, blank node, literal value) to its interpretation (an index).
Specifies strong inequalities.

### Composition