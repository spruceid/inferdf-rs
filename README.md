# InfeRDF

InfeRDF is an RDF Engine, a tool to build modular RDF interpretations from the
lexical representation of datasets and inference rules describing the semantics
of resources and properties.

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
processing an RDF dataset from its textual representation is not so. That is for
two main reasons:

  - The same resource might have multiple lexical representations (multiple
    terms). This is especially true for different blank node identifiers in two
    different documents that may in fact reference the same resource. Such
    mapping from lexical representation to resource is called an
    *interpretation*.
  - Triples may have a meaning that implies the presence of other omitted
    triples. A semantics that should be captured by the processor. In the
    example above, it is implied that `http://schema.org/name` is a property,
    which is specified by the following omitted triple:
    ```
    <http://schema.org/name> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>
    ```

InfeRDF can help you deal with these technicalities to build sound and minimal
interpretations. These interpretations can then be put into *modules* and
combined together.

## Design

InfeRDF is split into separate building blocks:
  - `core`: Type and trait definitions for RDF modules, interpretations, etc.
  - `storage`: File format to store RDF modules.
  - `deduce`: Deduction rules and systems to define RDF semantics.
  - `rdfs`: Simple domain specific language to describe deduction rules.
  - `cli`: Command line interface.