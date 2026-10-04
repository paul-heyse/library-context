//! JSON schemas for existing nominal and finite selection values; no independent predicate meaning.
use crate::domain::*;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde_json::{Value, json};
use std::borrow::Cow;
impl<T: Record> JsonSchema for Id<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("{}Id", T::NAME).into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":16,"maxItems":16,"x-nominal-relation":T::NAME})
    }
}
impl<T: SumRecord, const CODE: i16> JsonSchema for ArmId<T, CODE> {
    fn schema_name() -> Cow<'static, str> {
        format!("{}Arm{}Id", T::NAME, CODE).into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":16,"maxItems":16,"x-nominal-relation":T::NAME,"x-nominal-arm":CODE})
    }
}
impl JsonSchema for ContentHash {
    fn schema_name() -> Cow<'static, str> {
        "ContentHash".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":32,"maxItems":32})
    }
}
macro_rules! code_schema { ($($ty:ty),* $(,)?)=>{$(
    impl JsonSchema for $ty {
        fn schema_name()->Cow<'static,str>{ stringify!($ty).replace("::","_").into() }
        fn json_schema(_: &mut SchemaGenerator)->Schema {json_schema!({"type":"integer","enum":<$ty as FieldValue>::codes().iter().map(|(c,_)| *c).collect::<Vec<_>>(),"description":<$ty as FieldValue>::codes().iter().map(|(c,label)|format!("{c} = {label}")).collect::<Vec<_>>().join("; ")})}
    }
)*}; }
code_schema!(
    attribution::Fidelity,
    attribution::Origin,
    attribution::ExtractionMode,
    execution::ExactRuntimeException,
    execution::protocol_interpretation::InvocationQuestion,
    execution::closed_targets::TargetBasis,
    types::TypeRole,
    selection::EvidenceBasis,
    selection::Mode,
    selection::JointPolicy,
    selection::Quantifier,
    selection::Outcome,
    selection::Reason,
    selection::JointApplicability,
    selection::Facet,
    selection::ClassFacet,
    selection::MemberKind,
    selection::InvocationForm,
    selection::DefaultState,
    selection::ConfigurationScope,
    selection::FieldRelationship,
    selection::RelationRole,
    selection::Fidelity,
    selection::CheckAxis,
    selection::DeploymentField,
    calls::ParameterKind,
    calls::SignatureForm,
    calls::SignatureRole,
    attribution::Modality,assertion::Approximation,
    calls::PysaUnresolvedReason,calls::ArgumentKind,calls::BindingKind,calls::CallPhase,calls::ReceiverPassing,
    normalized::entities::ResolutionStatus,normalized::links::LinkReason,
    normalized::bindings::BindingOutcome,normalized::bindings::BindingReason,
    normalized::signature_applicability::BindingAuthority,normalized::signature_applicability::AuthorityReason,
    diagnostics::DiagnosticSeverity,diagnostics::DiagnosticChannel,diagnostics::DiagnosticLocation,diagnostics::SelectedRuffRule,diagnostics::NativeBaselineStatus,diagnostics::DefinitionAnswer,diagnostics::NativeParameterRole,diagnostics::NativeDefinitionMetadata,diagnostics::NativeDefinitionSymbolKind,
    captures::CaptureOrigin,
    captures::CaptureTiming,
    flow_capture::FlowCaptureOrigin,
    flow_capture::FlowSnapshotState,
    lexical::BindingEventKind,
    catalog::evidence::Intent,
    deployment::CheckStatus,
    types::RecordKind,
    normalized::callables::SignatureAdjustment,
    normalized::callables::Knowledge,
    catalog::evidence::AssociationBasis,
    catalog::CatalogContractBasis,
    retrieval::Family,
    analysis::policy::EvidenceStatus,
    analysis::policy::AssertionKind,
    analysis::policy::BriefSection,
    analysis::policy::SupportRole,
    obligation::Verdict,
    obligation::ObligationKind
);
fn annotated(mut schema: Value, description: &str) -> Value {
    schema
        .as_object_mut()
        .expect("schema object")
        .insert("description".into(), Value::String(description.into()));
    schema
}
fn object(fields: Vec<(&str, Value)>) -> Value {
    let required: Vec<_> = fields.iter().map(|(n, _)| n.to_string()).collect();
    let properties: serde_json::Map<_, _> = fields
        .into_iter()
        .map(|(n, v)| (n.to_string(), v))
        .collect();
    json!({"type":"object","additionalProperties":false,"properties":properties,"required":required})
}
macro_rules! enum_schema { ($module:ident::$ty:ident, $($variant:ident {$($field:ident:$ft:ty),* $(,)?}),* $(,)?)=>{
    // The finite encoding declaration must cover every canonical variant, field and field type.
    // Additions therefore cannot silently decode successfully while disappearing from the schema.
    const _: fn(&$module::$ty) = |value| match value {
        $($module::$ty::$variant {$($field),*}=>{$(let _: &$ft=$field;)*}),*
    };
    impl JsonSchema for $module::$ty {
        fn schema_name()->Cow<'static,str>{ stringify!($ty).replace("::","_").into() }
        fn json_schema(generator:&mut SchemaGenerator)->Schema {
            let arms: Vec<Value> = vec![$(object(vec![(stringify!($variant),object(vec![$((stringify!($field).trim_start_matches("r#"),generator.subschema_for::<$ft>().to_value())),*]))])),*];
            json_schema!({"oneOf":arms})
        }
    }
}; }
enum_schema!(selection::StructuralType,
    CanonicalTerm {term:Id<types::TypeTerm>}, Category {kind:i16}, NominalIdentity {module:String,name:String}, DeclaredUnionMember {term:Id<types::TypeTerm>});
enum_schema!(selection::RelationTarget,
    Declaration {entity:Id<normalized::entities::EntityRef>}, Member {member:Id<catalog::CatalogMember>}, Original {source:Id<catalog::evidence::OriginalSource>});
enum_schema!(selection::FieldTarget,
    Declaration {entity:Id<normalized::entities::EntityRef>}, Parameter {slot:Id<normalized::callables::SignatureSlot>});
enum_schema!(selection::FacetValue,
    ParameterName {name:String},
    ParameterType {role:calls::SignatureRole,name:String,r#type:selection::StructuralType},
    ReturnType {role:calls::SignatureRole,r#type:selection::StructuralType},
    SpecializedParameterType {site:Id<source::Occurrence>,declaration:Id<types::NativeSignatureObservation>,parameter:Id<calls::SignatureParameter>,r#type:selection::StructuralType},
    SpecializedReturnType {site:Id<source::Occurrence>,declaration:Id<types::NativeSignatureObservation>,signature:Id<calls::Signature>,r#type:selection::StructuralType},
    DecoratorQualifiedName {module:String,path:Vec<String>}, Async {asynchronous:bool},
    ModulePath {module:String}, MemberKind {kind:selection::MemberKind}, RaisedClass {r#type:selection::StructuralType},
    ClassMetadata {trait_kind:selection::ClassFacet,present:bool}, Deprecation {role:calls::SignatureRole,deprecated:bool});
enum_schema!(selection::Predicate,
    BehavioralRaises {exception:execution::ExactRuntimeException},
    FacetMembership {facet:selection::Facet,value:selection::FacetValue}, PublicPath {path:Vec<String>}, PublicModule {module:String}, ClassOwner {path:Vec<String>},
    MemberKind {kind:selection::MemberKind}, InvocationForm {form:selection::InvocationForm}, DeclaresParameter {name:String},
    ParameterKind {name:String,kind:calls::ParameterKind}, ParameterRequired {name:String,required:bool},
    ParameterDefaultState {name:String,state:selection::DefaultState}, ParameterDefault {name:String,value:Id<value::Literal>},
    ParameterType {name:String,r#type:selection::StructuralType},
    VariantParameterType {role:calls::SignatureRole,name:String,r#type:selection::StructuralType}, VariantReturnType {role:calls::SignatureRole,r#type:selection::StructuralType}, SpecializedType {site:Id<source::Occurrence>,declaration:Id<types::NativeSignatureObservation>,subject:Id<types::SignatureTypeSubject>,r#type:selection::StructuralType}, DeclaresConfigurationField {name:String}, ConfigurationOwner {path:Vec<String>},
    ConfigurationScope {scope:selection::ConfigurationScope}, ConfigurationRecordKind {kind:types::RecordKind},
    ConfigurationDefault {name:String,value:Id<value::Literal>}, ConfigurationLiteral {name:String,value:Id<value::Literal>},
    ConfigurationRelationship {name:String,kind:selection::FieldRelationship,target:selection::FieldTarget},
    Relationship {role:selection::RelationRole,target:selection::RelationTarget,fidelity:selection::Fidelity},
    ScenarioIntent {intent:catalog::evidence::Intent}, ScenarioCheck {check:selection::CheckAxis,status:deployment::CheckStatus},
    SourceAlignment {exact:bool}, ReleaseVersion {distribution:String,version:String}, DeploymentDeclaration {field:selection::DeploymentField,name:String});
impl JsonSchema for selection::Requirement {
    fn schema_name() -> Cow<'static, str> {
        "SelectionRequirement".into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        object(vec![
            (
                "predicate",
                g.subschema_for::<selection::Predicate>().to_value(),
            ),
            (
                "quantifier",
                annotated(g.subschema_for::<selection::Quantifier>().to_value(), "Choose whether any or all declared candidates must satisfy this predicate; missing evidence remains unresolved."),
            ),
        ])
        .try_into()
        .expect("object schema")
    }
}
impl JsonSchema for selection::Selection {
    fn schema_name() -> Cow<'static, str> {
        "Selection".into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        object(vec![
            (
                "requirements",
                g.subschema_for::<Vec<selection::Requirement>>().to_value(),
            ),
            ("mode", annotated(g.subschema_for::<selection::Mode>().to_value(), "Discovery (default) retains supported, unresolved and conflicting candidates. Strict retains only supported results; incomplete evidence is not rejection.")),
            (
                "joint",
                annotated(g.subschema_for::<selection::JointPolicy>().to_value(), "Independent requirements may use different declarations; joint applicability requires their declared shared context. Select the policy for the comparison you need."),
            ),
        ])
        .try_into()
        .expect("object schema")
    }
}
