#pragma endian big

struct GraphDescription {
	u32 triple_count;
	u32 triple_page_count;
	u32 resource_count;
	u32 resource_page_count;
	u32 first_page;
};

struct Graph {
	u32 id;
	GraphDescription description;
};

struct Header {
	u32 tag;
	u32 version;
	u32 page_size;
	u32 iri_count;
	u32 iri_page_count;
	u32 literal_count;
	u32 literal_page_count;
	u32 resource_count;
	u32 resource_page_count;
	u32 named_graph_count;
	u32 named_graph_page_count;
	GraphDescription default_graph;
};

struct Vec<T> {
	u32 size;
	T data[size];
};

struct Iri {
	u32 size;
	u8 data[size];
};

struct IriEntry {
	Iri iri;
	u32 interpretation;
};

struct IrisPage {
	Vec<IriEntry> entries;
};

struct GraphsPage<auto size> {
	Graph entries[size];
};

struct Cause {
	u8 cause_tag;
	u32 cause_data;
};

struct Triple {
	u32 subject;
	u32 predicate;
	u32 object;
};

enum Sign : u8 {
	Positive,
	Negative
};

struct SignedTriple {
	Sign sign;
	Triple triple;
};

struct MetaSignedTriple {
	SignedTriple signed_triple;
	Cause cause;
};

struct GraphTriplesPage<auto size> {
	MetaSignedTriple entries[size];
};

struct Resource {
	Vec<u32> as_subject;
	Vec<u32> as_predicate;
	Vec<u32> as_object;
	Vec<u32> different_from;
};

struct GraphResourcesPage {
	Vec<Resource> resources;
};