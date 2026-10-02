//! JSON schemas for existing nominal and finite selection values; no independent predicate meaning.
use crate::domain::*;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde_json::{Value, json};
use std::borrow::Cow;
impl<T: Record> JsonSchema for Id<T> {
    fn schema_name() -> Cow<'static,str> { format!("{}Id",T::NAME).into() }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":16,"maxItems":16,"x-nominal-relation":T::NAME})
    }
}
impl<T: SumRecord,const CODE:i16> JsonSchema for ArmId<T,CODE> {
    fn schema_name()->Cow<'static,str>{format!("{}Arm{}Id",T::NAME,CODE).into()}
    fn json_schema(_: &mut SchemaGenerator)->Schema{json_schema!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":16,"maxItems":16,"x-nominal-relation":T::NAME,"x-nominal-arm":CODE})}
}
impl JsonSchema for ContentHash {
    fn schema_name() -> Cow<'static,str> { "ContentHash".into() }
    fn json_schema(_: &mut SchemaGenerator) -> Schema { json_schema!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":32,"maxItems":32}) }
}
macro_rules! code_schema { ($($ty:ty),* $(,)?)=>{$(
    impl JsonSchema for $ty {
        fn schema_name()->Cow<'static,str>{ stringify!($ty).replace("::","_").into() }
        fn json_schema(_: &mut SchemaGenerator)->Schema {json_schema!({"type":"integer","enum":<$ty as FieldValue>::codes().iter().map(|(c,_)| *c).collect::<Vec<_>>()})}
    }
)*}; }
code_schema!(selection::EvidenceBasis,selection::Mode,selection::JointPolicy,selection::Quantifier,selection::Outcome,
    selection::Reason,selection::JointApplicability,selection::Facet,selection::MemberKind,
    selection::InvocationForm,selection::DefaultState,selection::ConfigurationScope,
    selection::FieldRelationship,selection::RelationRole,selection::Fidelity,
    selection::CheckAxis,selection::DeploymentField,calls::ParameterKind,calls::SignatureForm,
    catalog::evidence::Intent,deployment::CheckStatus,types::RecordKind,
    normalized::callables::SignatureAdjustment,normalized::callables::Knowledge,
    catalog::evidence::AssociationBasis,catalog::CatalogContractBasis,retrieval::Family,analysis::policy::EvidenceStatus,analysis::policy::AssertionKind,analysis::policy::BriefSection,analysis::policy::SupportRole,
    obligation::Verdict,obligation::ObligationKind);
fn object(fields: Vec<(&str, Value)>) -> Value {
    let required: Vec<_> = fields.iter().map(|(n,_)|n.to_string()).collect();
    let properties: serde_json::Map<_,_> = fields.into_iter().map(|(n,v)|(n.to_string(),v)).collect();
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
enum_schema!(selection::Predicate,
    FacetMembership {facet:selection::Facet,value:String}, PublicPath {path:Vec<String>}, PublicModule {module:String}, ClassOwner {path:Vec<String>},
    MemberKind {kind:selection::MemberKind}, InvocationForm {form:selection::InvocationForm}, DeclaresParameter {name:String},
    ParameterKind {name:String,kind:calls::ParameterKind}, ParameterRequired {name:String,required:bool},
    ParameterDefaultState {name:String,state:selection::DefaultState}, ParameterDefault {name:String,value:Id<value::Literal>},
    ParameterType {name:String,r#type:selection::StructuralType}, DeclaresConfigurationField {name:String}, ConfigurationOwner {path:Vec<String>},
    ConfigurationScope {scope:selection::ConfigurationScope}, ConfigurationRecordKind {kind:types::RecordKind},
    ConfigurationDefault {name:String,value:Id<value::Literal>}, ConfigurationLiteral {name:String,value:Id<value::Literal>},
    ConfigurationRelationship {name:String,kind:selection::FieldRelationship,target:selection::FieldTarget},
    Relationship {role:selection::RelationRole,target:selection::RelationTarget,fidelity:selection::Fidelity},
    ScenarioIntent {intent:catalog::evidence::Intent}, ScenarioCheck {check:selection::CheckAxis,status:deployment::CheckStatus},
    SourceAlignment {exact:bool}, ReleaseVersion {distribution:String,version:String}, DeploymentDeclaration {field:selection::DeploymentField,name:String});
impl JsonSchema for selection::Requirement {
    fn schema_name()->Cow<'static,str>{"SelectionRequirement".into()}
    fn json_schema(g:&mut SchemaGenerator)->Schema {object(vec![("predicate",g.subschema_for::<selection::Predicate>().to_value()),("quantifier",g.subschema_for::<selection::Quantifier>().to_value())]).try_into().expect("object schema")}
}
impl JsonSchema for selection::Selection {
    fn schema_name()->Cow<'static,str>{"Selection".into()}
    fn json_schema(g:&mut SchemaGenerator)->Schema {object(vec![("requirements",g.subschema_for::<Vec<selection::Requirement>>().to_value()),("mode",g.subschema_for::<selection::Mode>().to_value()),("joint",g.subschema_for::<selection::JointPolicy>().to_value())]).try_into().expect("object schema")}
}
