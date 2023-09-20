#pragma endian big

fn ceil_div(u32 a, u32 b) {
	return (a + b - 1) / b;
};

fn page_count(u32 page_size, u32 entry_count) {
	return ceil_div(entry_count, page_size);
};

fn entry_offset(u32 page_size, u32 entry_size, u32 page_index) {
	u32 entry_per_page = page_size / entry_size;
	return entry_per_page * page_index;
};

struct Page<T> {
	u32 page_index = (addressof(this) - parent.byte_offset) / parent.parent.parent.page_size;
	u32 entry_offset = entry_offset(parent.parent.parent.page_size, sizeof(T), this.page_index);
	u32 entry_count = parent.entry_count - this.entry_offset;
	
	T entries[this.entry_count];
};

struct Section<T> {
	u32 page_offset;
	u32 entry_count;
	
	u32 page_count = page_count(parent.parent.page_size, this.entry_count);
	u32 byte_offset = parent.parent.first_page_offset + this.page_offset * parent.parent.page_size;
	Page<T> pages[this.page_count] @ this.byte_offset;
};

struct Vec<T> {
	u32 offset;
	u32 len;
};

struct Option<T> {
	u8 discriminant;
	T value;
};

struct Entry {
	u32 offset;
	u32 len;
};

struct Iri {
	Entry iri;
	u32 interpretation;
};

struct Literal {
	Entry value;
	u8 type_variant;
	Entry type_value;
};

struct GroupId {
	u32 layer;
	u32 index;
};

struct Class {
	GroupId group_id;
	u32 member;
};

struct InterpretationResource {
	u32 id;
	Vec<u32> iris;
	Vec<u32> literals;
	Vec<u32> ne;
	Option<Class> class;
};

struct Interpretation {
	Section<Iri> iris;
	Section<Literal> literals;
	Section<InterpretationResource> resources;
};

struct Signed<T> {
	u8 sign;
	T value;
};

struct Triple {
	u32 subject;
	u32 predicate;
	u32 object;
};

struct Cause {
	u8 discriminant;
	u32 value;
};

struct Fact {
	Signed<Triple> triple;
	Cause cause;
};

struct GraphResource {
	u32 id;
	Vec<u32> as_subject;
	Vec<u32> as_predicate;
	Vec<u32> as_object;
};

struct GraphDescription {
	Section<Fact> facts;
	Section<GraphResource> resources;
};

struct NamedGraph {
	u32 id;
	GraphDescription description;
};

struct Dataset {
	u32 page_size = parent.page_size;
	u32 first_page_offset = parent.first_page_offset;
	
	GraphDescription default_graph;
	Section<NamedGraph> named_graphs;
};

struct GroupDescription {
	Vec<u32> members;
};

struct GroupByDesc {
	u32 layer;
	GroupDescription description;
	u32 index;
};

struct GroupById {
	GroupId id;
	GroupDescription description;
};

struct Representative {
	Class class;
	u32 resource;
};

struct Classification {
	Section<GroupByDesc> groups_by_desc;
	Section<GroupById> groups_by_id;
	Section<Representative> representatives;
};

struct HeapSection {
	u32 page_offset;
	u32 page_count;
};

struct Header {
	u32 tag;
	u32 version;
	u32 page_size;
	
	u32 first_page_offset = ceil_div(0x5C, this.page_size) * this.page_size;

	Interpretation interpretation;
	Dataset dataset;
	Classification classification;

	HeapSection heap;
};

Header header @ 0x00;