# Inference

## Inference rules

### [Rule 1] Merging cannot trigger rule

### [Rule 2] Resources cannot be aliased in dependencies

## Universal rules

```
rule forall ?y {
	?x prop ?y
	...
}
```

This rule is triggered after stabilization and only for all `?x` such that
`prop` is non locked. After the rule is applied, `prop` will be locked for `?x`.
Adding new triples of the form `?x prop ?z` will trigger an error.