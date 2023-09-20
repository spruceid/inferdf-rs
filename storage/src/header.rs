mod tag;
mod version;

use std::io;

use educe::Educe;
use inferdf_core::{
	class::{self, GroupId},
	Cause, Class, DivCeil, Signed,
};
use iref::IriBuf;
use langtag::LanguageTagBuf;
use paged::{
	cache::UnboundRef, encode_string_on_heap, Encode, EncodeSized, HeapSection, Paged, Section,
	UnboundSliceIter,
};
use rdf_types::{
	literal, IriVocabulary, IriVocabularyMut, Literal, LiteralVocabulary, Vocabulary, VocabularyMut,
};
pub use tag::Tag;
pub use version::Version;

use inferdf_core::Id;

#[derive(Paged, Educe)]
#[educe(Debug)]
pub struct Header<V: Vocabulary> {
	pub tag: Tag,
	pub version: Version,
	pub page_size: u32,
	pub interpretation: Interpretation<V>,
	pub dataset: Dataset,
	pub classification: Classification,
	pub heap: HeapSection,
}

impl<V: Vocabulary> Header<V> {
	pub fn first_page_offset(&self) -> u32 {
		DivCeil::div_ceil(Self::ENCODED_SIZE, self.page_size) * self.page_size
	}
}

#[derive(Paged, Educe)]
#[educe(Debug)]
pub struct Interpretation<V: Vocabulary> {
	pub iris: Section<IriEntry<V>>,
	pub literals: Section<LiteralEntry<V>>,
	pub resources: Section<InterpretedResource>,
}

#[derive(Paged)]
#[paged(context(V), heap, decode_bounds(V: IriVocabularyMut))]
pub struct IriEntry<V: IriVocabulary> {
	pub iri: EncodedIri<V>,
	pub interpretation: Id,
}

impl<V: IriVocabulary> IriEntry<V> {
	pub fn iri(&self) -> &V::Iri {
		&self.iri.0
	}
}

#[derive(Educe)]
#[educe(Clone(bound = "V::Iri: Clone"))]
pub struct EncodedIri<V: IriVocabulary>(pub V::Iri);

impl<V: IriVocabulary> paged::EncodeSized for EncodedIri<V> {
	const ENCODED_SIZE: u32 = paged::heap::Entry::ENCODED_SIZE;
}

impl<V: IriVocabulary> paged::EncodeOnHeap<V> for EncodedIri<V> {
	fn encode_on_heap(
		&self,
		context: &V,
		heap: &mut paged::Heap,
		output: &mut impl std::io::Write,
	) -> std::io::Result<u32> {
		let value = context.iri(&self.0).unwrap().as_str();
		let offset = heap.insert(context, value)?;
		offset.sized(value.len() as u32).encode(context, output)
	}
}

impl<V: IriVocabularyMut> paged::DecodeFromHeap<V> for EncodedIri<V> {
	fn decode_from_heap<R: io::Seek + io::Read>(
		input: &mut paged::reader::Cursor<R>,
		context: &mut V,
		heap: HeapSection,
	) -> io::Result<Self> {
		let string = String::decode_from_heap(input, context, heap)?;
		let iri = IriBuf::new(string).map_err(|_| io::ErrorKind::InvalidData)?;
		Ok(Self(context.insert_owned(iri)))
	}
}

#[derive(Paged)]
#[paged(
	context(V),
	heap,
	bounds(V: Vocabulary + LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>),
	encode_bounds(V::Value: AsRef<str>),
	decode_bounds(V: VocabularyMut, V::Value: From<String>)
)]
pub struct LiteralEntry<V: LiteralVocabulary> {
	pub literal: EncodedLiteral<V>,
	pub interpretation: Id,
}

impl<V: LiteralVocabulary> LiteralEntry<V> {
	pub fn literal(&self) -> &V::Literal {
		&self.literal.0
	}
}

#[derive(Educe)]
#[educe(Clone(bound = "V::Literal: Clone"))]
pub struct EncodedLiteral<V: LiteralVocabulary>(pub V::Literal);

impl<V: LiteralVocabulary> paged::EncodeSized for EncodedLiteral<V> {
	const ENCODED_SIZE: u32 =
		paged::heap::Entry::ENCODED_SIZE + 1 + paged::heap::Entry::ENCODED_SIZE;
}

impl<V: Vocabulary> paged::EncodeOnHeap<V> for EncodedLiteral<V>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Value: AsRef<str>,
{
	fn encode_on_heap(
		&self,
		context: &V,
		heap: &mut paged::Heap,
		output: &mut impl std::io::Write,
	) -> std::io::Result<u32> {
		let lit = context.literal(&self.0).unwrap();
		encode_string_on_heap(heap, output, lit.value().as_ref())?;
		match lit.type_() {
			literal::Type::Any(i) => {
				let iri = context.iri(i).unwrap();
				0u8.encode(context, output)?;
				encode_string_on_heap(heap, output, iri.as_str())?;
				Ok(Self::ENCODED_SIZE)
			}
			literal::Type::LangString(t) => {
				let tag = context.language_tag(t).unwrap();
				1u8.encode(context, output)?;
				encode_string_on_heap(heap, output, tag.as_str())?;
				Ok(Self::ENCODED_SIZE)
			}
		}
	}
}

impl<V: VocabularyMut> paged::DecodeFromHeap<V> for EncodedLiteral<V>
where
	V: LiteralVocabulary<Type = literal::Type<V::Iri, V::LanguageTag>>,
	V::Value: From<String>,
{
	fn decode_from_heap<R: io::Seek + io::Read>(
		input: &mut paged::reader::Cursor<R>,
		context: &mut V,
		heap: HeapSection,
	) -> io::Result<Self> {
		use paged::Decode;
		let value = String::decode_from_heap(input, context, heap)?;
		let discriminant = u8::decode(input, context)?;
		let type_ = match discriminant {
			0u8 => {
				let iri = IriBuf::new(String::decode_from_heap(input, context, heap)?)
					.map_err(|_| io::ErrorKind::InvalidData)?;
				literal::Type::Any(context.insert_owned(iri))
			}
			1u8 => {
				let tag = LanguageTagBuf::new(
					String::decode_from_heap(input, context, heap)?.into_bytes(),
				)
				.map_err(|_| io::ErrorKind::InvalidData)?;
				literal::Type::LangString(context.insert_owned_language_tag(tag))
			}
			_ => return Err(io::ErrorKind::InvalidData.into()),
		};
		Ok(Self(
			context.insert_owned_literal(Literal::new(value.into(), type_)),
		))
	}
}

#[derive(Paged)]
#[paged(heap)]
pub struct InterpretedResource {
	pub id: Id,
	pub iris: Vec<u32>,
	pub literals: Vec<u32>,
	pub ne: Vec<Id>,
	pub class: Option<Class>,
}

impl InterpretedResource {
	pub fn new(
		id: Id,
		iris: Vec<u32>,
		literals: Vec<u32>,
		ne: Vec<Id>,
		class: Option<Class>,
	) -> Self {
		Self {
			id,
			iris,
			literals,
			ne,
			class,
		}
	}
}

pub struct InterpretationResourceIrisBinder;

impl<'a> paged::cache::Binder<'a, UnboundRef<InterpretedResource>, UnboundSliceIter<u32>>
	for InterpretationResourceIrisBinder
{
	fn bind<'t>(self, t: &'t InterpretedResource) -> std::slice::Iter<'t, u32>
	where
		'a: 't,
	{
		t.iris.iter()
	}
}

pub struct InterpretationResourceLiteralsBinder;

impl<'a> paged::cache::Binder<'a, UnboundRef<InterpretedResource>, UnboundSliceIter<u32>>
	for InterpretationResourceLiteralsBinder
{
	fn bind<'t>(self, t: &'t InterpretedResource) -> std::slice::Iter<'t, u32>
	where
		'a: 't,
	{
		t.literals.iter()
	}
}

pub struct InterpretationResourceNeBinder;

impl<'a> paged::cache::Binder<'a, UnboundRef<InterpretedResource>, UnboundSliceIter<Id>>
	for InterpretationResourceNeBinder
{
	fn bind<'t>(self, t: &'t InterpretedResource) -> std::slice::Iter<'t, Id>
	where
		'a: 't,
	{
		t.ne.iter()
	}
}

#[derive(Debug, Paged)]
pub struct Dataset {
	pub default_graph: GraphDescription,
	pub named_graphs: Section<Graph>,
}

#[derive(Debug, Paged)]
pub struct Graph {
	pub id: Id,
	pub description: GraphDescription,
}

#[derive(Debug, Clone, Copy, Paged)]
pub struct GraphDescription {
	pub facts: Section<GraphFact>,
	pub resources: Section<GraphResource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Paged)]
pub struct Triple(pub Id, pub Id, pub Id);

impl From<Triple> for inferdf_core::Triple {
	fn from(value: Triple) -> Self {
		Self(value.0, value.1, value.2)
	}
}

impl From<inferdf_core::Triple> for Triple {
	fn from(value: inferdf_core::Triple) -> Self {
		Self(value.0, value.1, value.2)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Paged)]
pub struct GraphFact {
	pub triple: Signed<Triple>,
	pub cause: Cause,
}

impl From<GraphFact> for inferdf_core::GraphFact {
	fn from(value: GraphFact) -> Self {
		Self::new(value.triple.cast(), value.cause)
	}
}

#[derive(Debug, Paged)]
#[paged(heap)]
pub struct GraphResource {
	pub id: Id,
	pub as_subject: Vec<u32>,
	pub as_predicate: Vec<u32>,
	pub as_object: Vec<u32>,
}

pub struct GraphResourceAsSubjectBinder;

impl<'a> paged::cache::Binder<'a, UnboundRef<GraphResource>, UnboundSliceIter<u32>>
	for GraphResourceAsSubjectBinder
{
	fn bind<'t>(self, t: &'t GraphResource) -> std::slice::Iter<'t, u32>
	where
		'a: 't,
	{
		t.as_subject.iter()
	}
}

pub struct GraphResourceAsPredicateBinder;

impl<'a> paged::cache::Binder<'a, UnboundRef<GraphResource>, UnboundSliceIter<u32>>
	for GraphResourceAsPredicateBinder
{
	fn bind<'t>(self, t: &'t GraphResource) -> std::slice::Iter<'t, u32>
	where
		'a: 't,
	{
		t.as_predicate.iter()
	}
}

pub struct GraphResourceAsObjectBinder;

impl<'a> paged::cache::Binder<'a, UnboundRef<GraphResource>, UnboundSliceIter<u32>>
	for GraphResourceAsObjectBinder
{
	fn bind<'t>(self, t: &'t GraphResource) -> std::slice::Iter<'t, u32>
	where
		'a: 't,
	{
		t.as_object.iter()
	}
}

#[derive(Debug, Paged)]
pub struct Classification {
	pub groups_by_desc: Section<GroupByDesc>,
	pub groups_by_id: Section<GroupById>,
	pub representatives: Section<Representative>,
}

pub struct GetDescriptionBinder;

#[derive(Paged)]
#[paged(heap)]
pub struct GroupByDesc {
	pub layer: u32,
	pub description: class::group::Description,
	pub index: u32,
}

impl GroupByDesc {
	pub fn new(description: class::group::Description, id: GroupId) -> Self {
		Self {
			layer: id.layer,
			description,
			index: id.index,
		}
	}
}

impl<'a> paged::cache::Binder<'a, UnboundRef<GroupByDesc>, UnboundRef<class::group::Description>>
	for GetDescriptionBinder
{
	fn bind<'t>(self, t: &'t GroupByDesc) -> &'t class::group::Description
	where
		'a: 't,
	{
		&t.description
	}
}

#[derive(Paged)]
#[paged(heap)]
pub struct GroupById {
	pub id: GroupId,
	pub description: class::group::Description,
}

impl GroupById {
	pub fn new(id: GroupId, description: class::group::Description) -> Self {
		Self { id, description }
	}
}

impl<'a> paged::cache::Binder<'a, UnboundRef<GroupById>, UnboundRef<class::group::Description>>
	for GetDescriptionBinder
{
	fn bind<'t>(self, t: &'t GroupById) -> &'t class::group::Description
	where
		'a: 't,
	{
		&t.description
	}
}

#[derive(Paged)]
pub struct Representative {
	pub class: Class,
	pub resource: Id,
}

impl Representative {
	pub fn new(class: Class, resource: Id) -> Self {
		Self { class, resource }
	}
}
