//! Immutable full binding meaning beneath replaceable attempt slots. These codecs
//! confer no admission or lifetime authority; completion earns and holds their rows.
use lctx_model::domain::{completed::{CompletedBinding, attempt_binding}, ContentHash, ModelError};
use surrealdb::types::{Bytes, Object, RecordId, Value};

pub fn record(binding:&CompletedBinding)->Result<RecordId,ModelError>{
    Ok(RecordId::new("compiler_binding_descriptor",binding.descriptor_identity()?.hex()))
}
pub fn value(binding:&CompletedBinding)->Result<Value,ModelError>{
    binding.validate()?;
    let mut row=Object::new();
    row.insert("id",record(binding)?);
    row.insert("descriptor",Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?));
    row.insert("view",RecordId::new("compiler_view",binding.view.identity.hex()));
    row.insert("source_identity",binding.source.identity().hex());
    Ok(Value::Object(row))
}
pub fn decode(row:&Value)->Result<CompletedBinding,ModelError>{
    let Value::Object(object)=row else{return Err(ModelError::Schema("canonical binding object"));};
    let Some(Value::Bytes(bytes))=object.get("descriptor") else{return Err(ModelError::Schema("canonical binding descriptor"));};
    let binding:CompletedBinding=serde_json::from_slice(bytes).map_err(ModelError::codec)?;
    if value(&binding)?!=*row{return Err(ModelError::Conflict("canonical binding full envelope"));}
    Ok(binding)
}
/// Logical transfer projection only. Its zero attempt is not a native owner and
/// destination import must independently validate/rebind it before use.
pub fn portable_row(binding:&CompletedBinding)->Result<Value,ModelError>{
    binding.validate()?;
    let zero=ContentHash([0;32]);
    let mut row=Object::new();
    row.insert("id",RecordId::new("compiler_binding",attempt_binding(zero,binding.key()).hex()));
    row.insert("attempt",RecordId::new("native_attempt",zero.hex()));
    row.insert("descriptor",Bytes::from(serde_json::to_vec(binding).map_err(ModelError::codec)?));
    for (field,value) in crate::completed_rows::binding_projection(binding){row.insert(field,value);}
    let row=Value::Object(row);
    crate::completed_rows::validate_state_row("compiler_binding",&row)?;
    Ok(row)
}
pub const fn schema_target()->&'static str{
    "DEFINE TABLE compiler_binding_descriptor SCHEMAFULL; DEFINE FIELD id ON compiler_binding_descriptor TYPE string; DEFINE FIELD descriptor ON compiler_binding_descriptor TYPE bytes; DEFINE FIELD view ON compiler_binding_descriptor TYPE record<compiler_view>; DEFINE FIELD source_identity ON compiler_binding_descriptor TYPE string; DEFINE INDEX descriptor_view ON compiler_binding_descriptor FIELDS view,id;"
}

#[cfg(test)]
mod controls{
    use super::*;
    use lctx_model::domain::{Relation,analysis::sources::SourceSnapshot,input::Package,completed::CompletedView};
    #[test]
    fn complete_binding_codec_refuses_retagging_and_extra_envelope_fields(){
        let relation=Relation::of::<Package>();
        let view=CompletedView::new(relation.name().into(),[ContentHash::of(b"contributor")].into(),0).unwrap();
        let binding=CompletedBinding{boundary:None,source:SourceSnapshot::of_completed_view(&relation,ContentHash::of(b"model"),&view).unwrap(),view,configuration:None};
        let row=value(&binding).unwrap();assert_eq!(decode(&row).unwrap(),binding);
        let changed=CompletedBinding{configuration:Some(ContentHash::of(b"different config")),..binding.clone()};
        let Value::Object(mut retagged)=row.clone() else{unreachable!()};
        retagged.insert("descriptor",Bytes::from(serde_json::to_vec(&changed).unwrap()));
        assert!(decode(&Value::Object(retagged)).is_err());
        let Value::Object(mut extra)=row else{unreachable!()};extra.insert("admitted",true);
        assert!(decode(&Value::Object(extra)).is_err());
        let portable=portable_row(&binding).unwrap();
        assert_ne!(portable,value(&binding).unwrap());
        crate::completed_rows::validate_state_row("compiler_binding",&portable).unwrap();
    }
}
