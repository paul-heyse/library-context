//! Canonical logical rows for the six-family v4 identity. This is effect-free data
//! normalization, never a native physical row codec or an admission capability.
use lctx_model::domain::{self as d,ContentHash,ModelError,completed::{CompletedContribution,CompletedView,CompletedBinding}};
use std::collections::BTreeMap;
use surrealdb::types::{Object,RecordId,Value,Bytes};

/// Callers independently validate native physical rows before this lowering. Contribution
/// ownership becomes the full completed identity, including outcome and all output digests;
/// no synthetic native attempt or spec-only physical address is used for logical identity.
pub fn normalize(table:&str,value:&Value,owners:&BTreeMap<RecordId,CompletedContribution>)->Result<Value,ModelError>{
    let object=value.as_object().ok_or(ModelError::Schema("logical completed row"))?;
    let mut row=Object::new();
    match table {
        "compiler_contribution"=>{
            let descriptor:CompletedContribution=decode(object)?;let logical=descriptor.identity()?;
            row.insert("id",RecordId::new(table,logical.hex()));
            row.insert("descriptor",Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?));
        }
        "compiler_membership"=>{
            let Some(Value::RecordId(owner))=object.get("contribution") else{return Err(ModelError::Schema("logical membership physical owner"));};
            let logical=owners.get(owner).ok_or(ModelError::Conflict("logical membership undeclared exact physical owner"))?.identity()?;
            let Some(Value::String(relation))=object.get("relation") else{return Err(ModelError::Schema("logical membership relation"));};
            let Some(Value::String(key))=object.get("semantic_key") else{return Err(ModelError::Schema("logical membership key"));};
            row.insert("id",crate::completed_rows::membership_id(logical,relation,&crate::completed_rows::decode_nominal_key(key)?));
            row.insert("contribution",RecordId::new("compiler_contribution",logical.hex()));
            for name in ["relation","semantic_key","node","content"] {
                row.insert(name,object.get(name).cloned().ok_or(ModelError::Schema("logical membership field"))?);
            }
        }
        "compiler_view"=>{
            let descriptor:CompletedView=decode(object)?;descriptor.validate()?;
            row.insert("id",RecordId::new(table,descriptor.identity.hex()));
            row.insert("descriptor",Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?));
        }
        "compiler_binding"=>{
            let descriptor:CompletedBinding=decode(object)?;descriptor.validate()?;
            row.insert("id",RecordId::new(table,descriptor.key().hex()));
            row.insert("descriptor",Bytes::from(serde_json::to_vec(&descriptor).map_err(ModelError::codec)?));
        }
        "compiler_record"|"compiler_alias"=>return Ok(value.clone()),
        _=>return Err(ModelError::Conflict("not a logical completed family")),
    }
    Ok(Value::Object(row))
}
fn decode<T:serde::de::DeserializeOwned>(row:&Object)->Result<T,ModelError>{
    let Some(Value::Bytes(bytes))=row.get("descriptor") else{return Err(ModelError::Schema("logical completed descriptor"));};
    serde_json::from_slice(bytes).map_err(ModelError::codec)
}
#[cfg(test)]
mod tests {
    use super::*;
    use d::{completed::{ContributionSpec,OutputContent,OutputProfile},stages::Profile};
    fn contribution()->CompletedContribution {
        CompletedContribution{ assurance: lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,spec:ContributionSpec{captured_binding:None,producer:"provider".into(),profile:OutputProfile::Specific(Profile::Catalog),model:ContentHash::of(b"model"),implementation:ContentHash::of(b"implementation"),configuration:None,inputs:vec![],outputs:["packages".into()].into()},outcome:0,outputs:[("packages".into(),OutputContent{rows:0,content:ContentHash::of(b"complete empty EOF")})].into()}
    }
    fn physical(physical:&str,descriptor:&CompletedContribution)->Value{
        let mut row=Object::new();row.insert("id",RecordId::new("compiler_contribution",physical));row.insert("attempt",RecordId::new("native_attempt",physical));
        row.insert("descriptor",Bytes::from(serde_json::to_vec(descriptor).unwrap()));Value::Object(row)
    }
    #[test]
    fn same_spec_different_completed_outcome_or_empty_output_never_collapses_logical_owner(){
        let one=contribution();let mut outcome=one.clone();outcome.outcome=2;
        let mut output=one.clone();output.outputs.get_mut("packages").unwrap().content=ContentHash::of(b"another explicit empty output meaning");
        let base=normalize("compiler_contribution",&physical("copy-one",&one),&BTreeMap::new()).unwrap();
        assert_eq!(base,normalize("compiler_contribution",&physical("copy-two",&one),&BTreeMap::new()).unwrap(),"native attempt/copy does not rename logical content");
        for other in [outcome,output] {
            assert_eq!(one.spec.identity().unwrap(),other.spec.identity().unwrap());
            assert_ne!(base,normalize("compiler_contribution",&physical("copy-one",&other),&BTreeMap::new()).unwrap(),"full completed outcome/output is logical ownership");
        }
    }
    #[test]
    fn membership_normalization_requires_its_exact_physical_mapping_and_keeps_multiplicity_semantics(){
        let one=contribution();let owner=RecordId::new("compiler_contribution","exact-copy");
        let mut member=Object::new();member.insert("id",RecordId::new("compiler_membership","physical-member"));member.insert("contribution",owner.clone());member.insert("relation","packages");member.insert("semantic_key",hex::encode([1u8;16]));member.insert("node",RecordId::new("entity","payload"));member.insert("content",ContentHash::of(b"payload").hex());let member=Value::Object(member);
        assert!(normalize("compiler_membership",&member,&BTreeMap::new()).is_err(),"a logical match elsewhere cannot satisfy an unmapped physical owner");
        let first=normalize("compiler_membership",&member,&[(owner.clone(),one.clone())].into()).unwrap();
        let mut other=one;other.outcome=2;
        let second=normalize("compiler_membership",&member,&[(owner,other)].into()).unwrap();
        assert_ne!(first,second,"identical member bytes under distinct completed contributors retain different owner identity");
        assert!(normalize("native_completed_state",&member,&BTreeMap::new()).is_err(),"installation completion never enters logical roots");
    }
}
