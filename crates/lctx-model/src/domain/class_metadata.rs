//! Native class typing metadata. None of these declarations closes runtime dispatch or heap state.
use super::{
    assertion::AssertionQualification, attribution::FactFamily, calls::ProviderSymbol,
    types::RecordKind, *,
};
use crate::{Assertion, Domain, DomainCode};

/// The semantic basis is explicit even when source/effective values happen to agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum MetadataBasis {
    SourceDeclaration = 0,
    NativeEffective = 1,
    Inherited = 2,
    Synthesized = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "record_options")]
pub struct RecordOptions {
    #[model(key)]
    pub init: bool,
    #[model(key)]
    pub eq: bool,
    #[model(key)]
    pub order: bool,
    #[model(key)]
    pub frozen: bool,
    #[model(key)]
    pub match_args: bool,
    #[model(key)]
    pub kw_only: bool,
    #[model(key)]
    pub unsafe_hash: bool,
    #[model(key)]
    pub slots: bool,
    #[model(key)]
    pub extra: bool,
    #[model(key)]
    pub strict: bool,
    #[model(key)]
    pub auto_attribs: Option<bool>,
    #[model(key)]
    pub attrs_setattr_frozen: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "record_transform_defaults", validate = validate_transform, invariant_refs = transform_invariants_refs)]
pub struct RecordTransformDefaults {
    #[model(key)]
    pub eq: bool,
    #[model(key)]
    pub order: bool,
    #[model(key)]
    pub kw_only: bool,
    #[model(key)]
    pub frozen: bool,
    /// Field-specifier identity is not reconstructed from rendered native text.
    #[model(key)]
    pub field_specifier_count: i64,
    #[model(key)]
    pub field_specifier_shape: ContentHash,
    #[model(key)]
    pub field_specifier_identity_reason: Option<super::obligation::ObligationKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "class_metadata_observations", validate = validate_metadata)]
#[assertion(support = ClassMetadataSupport, name = "class_metadata_supports", family = FactFamily::Types, subjects(class, metaclass, custom_metaclass))]
pub struct ClassMetadataObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub class: Id<ProviderSymbol>,
    pub basis: MetadataBasis,
    pub metaclass: Id<ProviderSymbol>,
    pub custom_metaclass: Option<Id<ProviderSymbol>>,
    pub final_declaration: bool,
    pub protocol: bool,
    pub runtime_checkable: bool,
    pub new_type: bool,
    pub enumeration: bool,
    pub explicitly_abstract: bool,
    /// Known positive members survive recursive/partial answers. An empty native fallback
    /// cannot certify absence; the current public native API supplies no status discriminator.
    pub abstract_members: Vec<String>,
    pub abstract_absence_known: bool,
    pub protocol_members: Vec<String>,
    pub explicit_slots: bool,
    /// None means the native slot names are unavailable, distinct from a known empty list.
    pub slots: Option<Vec<String>>,
    pub record: Option<RecordKind>,
    pub record_options: Option<Id<RecordOptions>>,
    pub transform: Option<Id<RecordTransformDefaults>>,
    pub deprecated: bool,
    pub deprecation_message: Option<String>,
}
fn validate_metadata(row: &ClassMetadataObservation) -> Result<(), ModelError> {
    let sorted =
        |v: &[String]| v.iter().all(|s| !s.is_empty()) && v.windows(2).all(|w| w[0] < w[1]);
    if !sorted(&row.abstract_members)
        || !sorted(&row.protocol_members)
        || row.slots.as_deref().is_some_and(|s| !sorted(s))
        || row.abstract_absence_known
        || (row.runtime_checkable && !row.protocol)
        || (row.deprecation_message.is_some() && !row.deprecated)
        || (row.record_options.is_some() && row.record.is_none())
    {
        return Err(ModelError::Invalid("invalid class metadata shape".into()));
    }
    Ok(())
}
/// One question-selected option operation. Source consumers supply actual source observations;
/// effective constructor/type consumers consume the native metadata, without source fallback.
pub fn record_options(
    observation: &ClassMetadataObservation,
    question: MetadataBasis,
) -> Option<Id<RecordOptions>> {
    (observation.basis == question)
        .then_some(observation.record_options)
        .flatten()
}

/// Native member kind describes typing lookup, never a reaching heap-storage certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum MemberKind {
    Property = 0,
    InstanceAttribute = 1,
    Other = 2,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum MemberOrigin {
    Source = 0,
    Inherited = 1,
    Synthesized = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain, Assertion)]
#[model(name = "class_member_observations", validate = validate_member)]
#[assertion(support = ClassMemberSupport, name = "class_member_supports", family = FactFamily::Types, subjects(class, defining_class, term, enum_value), referents(declaration))]
pub struct ClassMemberObservation {
    #[model(key)]
    pub qualification: Id<AssertionQualification>,
    #[model(key)]
    pub class: Id<ProviderSymbol>,
    #[model(key)]
    /// Native member/key spelling, including non-identifier and empty TypedDict keys.
    pub name: String,
    pub defining_class: Id<ProviderSymbol>,
    pub basis: MetadataBasis,
    pub origin: MemberOrigin,
    pub kind: MemberKind,
    pub term: Id<super::types::TypeTerm>,
    /// The native enum literal keeps its raw assigned value type separately from member identity.
    pub enum_value: Option<Id<super::types::TypeTerm>>,
    pub declaration: Option<Id<super::source::Occurrence>>,
    pub abstract_declaration: bool,
    pub final_declaration: bool,
}

/// Explicit receiver/member query result. Candidates are typing facts, not runtime dispatch closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberQuery {
    TypedCandidates(Vec<Id<ClassMemberObservation>>),
    Unresolved,
}
/// Select inferred receiver evidence at an exact requested source/context. Expected types and
/// Any receivers never supply a member certificate; property kind survives on candidate rows.
pub fn receiver_members(
    observation: &super::types::TypeObservation,
    source: Id<super::source::Occurrence>,
    context: Id<super::attribution::AnalysisContext>,
    name: &str,
    qualifications: &super::normalized::Rows<AssertionQualification>,
    terms: &super::normalized::Rows<super::types::TypeTerm>,
    members: &super::normalized::Rows<ClassMemberObservation>,
) -> MemberQuery {
    use super::types::{TypeRole, TypeTerm};
    if observation.subject != source
        || observation.role != TypeRole::AttributeBase
        || observation.declared
        || qualifications
            .get(observation.qualification)
            .is_none_or(|q| q.context != context)
    {
        return MemberQuery::Unresolved;
    }
    let class = match terms.get(observation.term) {
        Some(TypeTerm::ClassInstance { class, .. } | TypeTerm::ClassObject { class }) => *class,
        _ => return MemberQuery::Unresolved,
    };
    let mut candidates: Vec<_> = members
        .iter()
        .filter(|m| {
            m.class == class
                && m.name == name
                && qualifications
                    .get(m.qualification)
                    .is_some_and(|q| q.context == context)
        })
        .map(Record::id)
        .collect();
    candidates.sort();
    candidates.dedup();
    if candidates.is_empty() {
        MemberQuery::Unresolved
    } else {
        MemberQuery::TypedCandidates(candidates)
    }
}

fn validate_member(row: &ClassMemberObservation) -> Result<(), ModelError> {
    if (row.origin == MemberOrigin::Inherited && row.class == row.defining_class)
        || (row.origin == MemberOrigin::Source && row.class != row.defining_class)
    {
        return Err(ModelError::Invalid("invalid native member origin".into()));
    }
    Ok(())
}

fn validate_transform(row: &RecordTransformDefaults) -> Result<(), ModelError> {
    if row.field_specifier_count < 0
        || ((row.field_specifier_count > 0) != row.field_specifier_identity_reason.is_some())
    {
        return Err(ModelError::Invalid(
            "transform field-specifier identity is explicit when unavailable".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum FieldSpecifierKind {
    Callable = 0,
    Function = 1,
    Class = 2,
    StaticMethod = 3,
    ClassMethod = 4,
    Property = 5,
    CachedProperty = 6,
    EnumMember = 7,
    EnumNonmember = 8,
    DataclassField = 9,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name = "record_transform_field_specifiers")]
pub struct RecordTransformFieldSpecifier {
    #[model(key)]
    pub transform: Id<RecordTransformDefaults>,
    #[model(key)]
    pub ordinal: i64,
    pub kind: FieldSpecifierKind,
    /// Native classification label, never a resolved class/function identity.
    pub name: Option<String>,
}
pub fn field_specifier_shape(values: &[(FieldSpecifierKind, Option<String>)]) -> ContentHash {
    let mut sink = KeySink::new("native-transform-field-specifier-classification");
    for (kind, name) in values {
        kind.encode(&mut sink);
        name.encode(&mut sink);
    }
    (values.len() as i64).encode(&mut sink);
    sink.finish()
}

pub(crate) fn transform_invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "native_transform_specifier_shape",
        inputs: vec![
            ValidationInput::of::<RecordTransformDefaults>(&["id"]),
            ValidationInput::of::<RecordTransformFieldSpecifier>(&["id"]),
        ],
        create: std::sync::Arc::new(|budget| {
            Box::new(TransformCheck {
                defaults: super::normalized::Rows::new(budget),
                specifiers: super::normalized::Rows::new(budget),
            })
        }),
    }]
}
struct TransformCheck {
    defaults: super::normalized::Rows<RecordTransformDefaults>,
    specifiers: super::normalized::Rows<RecordTransformFieldSpecifier>,
}
impl InvariantCheck for TransformCheck {
    fn visit(
        &mut self,
        relation: &str,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        match relation {
            RecordTransformDefaults::NAME => self.defaults.decode(batch),
            RecordTransformFieldSpecifier::NAME => self.specifiers.decode(batch),
            _ => Err(ModelError::Invalid(
                "undeclared transform specifier input".into(),
            )),
        }
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        for transform in self.defaults.iter() {
            let mut rows: Vec<_> = self
                .specifiers
                .iter()
                .filter(|s| s.transform == transform.id())
                .collect();
            rows.sort_by_key(|s| s.ordinal);
            let values: Vec<_> = rows.iter().map(|s| (s.kind, s.name.clone())).collect();
            if rows.len() as i64 != transform.field_specifier_count
                || rows
                    .iter()
                    .enumerate()
                    .any(|(ordinal, s)| s.ordinal != ordinal as i64)
                || field_specifier_shape(&values) != transform.field_specifier_shape
            {
                return Err(ModelError::Invalid(
                    "native transform field-specifier classification differs".into(),
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn transform_invariants_refs() -> Vec<&'static str> { vec!["native_transform_specifier_shape"] }

#[cfg(test)]
mod tests {
    use super::*;

    fn id<T>(n: u8) -> Id<T> {
        serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
            _,
            serde::de::value::Error,
        >::new([n; 16].into_iter()))
        .unwrap()
    }

    #[test]
    fn native_member_keys_keep_raw_spelling_without_weakening_origin_identity() {
        for name in ["", " ", "not-an-identifier"] {
            let mut member = ClassMemberObservation {
                qualification: id(1),
                class: id(2),
                name: name.into(),
                defining_class: id(2),
                basis: MetadataBasis::NativeEffective,
                origin: MemberOrigin::Source,
                kind: MemberKind::Other,
                term: id(3),
                enum_value: None,
                declaration: None,
                abstract_declaration: false,
                final_declaration: false,
            };
            member.validate().unwrap();
            member.origin = MemberOrigin::Inherited;
            assert!(
                member.validate().is_err(),
                "inherited origin needs a distinct defining class"
            );
            member.defining_class = id(4);
            member.validate().unwrap();
            member.origin = MemberOrigin::Source;
            assert!(
                member.validate().is_err(),
                "source origin needs its own defining class"
            );
        }
    }
}
