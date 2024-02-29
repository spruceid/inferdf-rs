# InfeRDF

InfeRDF is an RDF Engine, a tool to complete RDF interpretations from deduction
rules describing the semantics of resources and properties.

The Resource Description Framework (RDF) is a very simple graph data model used
to represent arbitrary pieces of information. Nodes of the graph are called
*resources*, and resources are connected together using *relations*, which are
resources themselves. To express resources in a textual way, each resource is
given a lexical representation, a *term*, which can either be
  - a literal value (such as a number, a text string, etc.),
  - an International Resource Identifier (IRI); or
  - a blank node identifier, which is local to the document.

Here is a example defining a single edge of an RDF graph, a *triple*
subject-predicate-object, stating that the name of this repository is
"InfeRDF":
```
<https://github.com/spruceid/inferdf-rs> <http://schema.org/name> "InfeRDF"
```

Thanks to this simplicity, writing RDF datasets is straight-forward. However
processing an RDF dataset from its textual representation is not so.
Triples may have a meaning that implies the presence of other omitted triples.
A semantics that should be captured by the processor. In the example above, it
is implied that `http://schema.org/name` is a property, which is specified by
the following omitted triple:
```
<http://schema.org/name> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>
```

InfeRDF can help you infer those omitted triples.
