//! Closed physical schemas generated from the selected canonical graph vocabulary.
use lctx_model::domain::{Record,Field,Scalar,ContentHash};
use std::collections::BTreeSet;
fn scalar(field:&Field)->String{
    let ty=match field.scalar(){Scalar::Text=>"string".into(),Scalar::Bool=>"bool".into(),Scalar::Int16|Scalar::Int32|Scalar::Int64=>{
        if field.codes().is_empty(){"int".into()}else{field.codes().iter().map(|(v,_)|v.to_string()).collect::<Vec<_>>().join(" | ")}
    },Scalar::Id=>"array<int,16>".into(),Scalar::Digest=>"array<int,32>".into(),Scalar::Binary=>"array<int>".into(),Scalar::FiniteF64=>"float".into()};
    let ty=if field.list(){format!("array<{ty}>")}else{ty};
    if field.nullable(){format!("{ty} | null")}else{ty}
}
fn declaration<R:Record>()->String{
    let fields=R::fields().into_iter().filter(|f|f.scalar()!=Scalar::Binary).collect::<Vec<_>>();
    if let Some(sum)=R::sum(){
        sum.arms.into_iter().map(|arm|{
            let mut parts=vec![format!("__type: '{}'",R::NAME)];
            for field in &fields{
                let ty=if field.name()==sum.tag{arm.code.to_string()}else if let Some(active)=arm.fields.iter().find(|a|a.name==field.name()){
                    let native=scalar(field);if active.required{native.replace(" | null","")}else{native}
                }else{"null".into()};parts.push(format!("`{}`: {ty}",field.name()));
            }format!("{{ {} }}",parts.join(", "))
        }).collect::<Vec<_>>().join(" | ")
    }else{let mut parts=vec![format!("__type: '{}'",R::NAME)];parts.extend(fields.iter().map(|f|format!("`{}`: {}",f.name(),scalar(f))));format!("{{ {} }}",parts.join(", "))}
}
macro_rules! entities{($($variant:ident:$ty:ty,)*)=>{fn entity_shapes()->Vec<String>{vec![$(declaration::<$ty>(),)*]}}}
lctx_model::graph_entity_records!(entities);
macro_rules! assertions{($($variant:ident:$ty:ty,)*)=>{fn assertion_shapes()->Vec<String>{vec![$(declaration::<$ty>(),)*]}}}
lctx_model::graph_assertion_records!(assertions);
/// Atomic fields selected for indexed ownership/capture joins; other typed predicates use the type index.
pub const SCOPE_FIELDS:&[&str]=&["member","exposure","candidate","callable","variant","invocation","domain","unit","corpus","fragment","specification","input","origin","package","access","context","root","source","target","qualification","slot","parent","inventory","characterization","event","diagnostic","entry","trace","attempt","use_","view","assessment","observation","assertion","release","distribution","module","artifact","brief","support","set","universe"];
pub fn canonical_schema()->String{
    let mut sql=String::new();
    for (table,shapes) in [("entity",entity_shapes()),("assertion",assertion_shapes())]{
        let mut shapes: BTreeSet<String>=shapes.into_iter().collect();if table=="assertion"{shapes.insert("{__type:'__graph_assertion'}".into());}
        sql.push_str(&format!("DEFINE TABLE {table} TYPE NORMAL SCHEMAFULL; DEFINE FIELD semantic_type ON {table} TYPE string; DEFINE FIELD semantic_key ON {table} TYPE string; DEFINE FIELD kind ON {table} TYPE int; DEFINE FIELD subtype ON {table} TYPE int | null; DEFINE FIELD content ON {table} TYPE string; DEFINE FIELD canonical ON {table} TYPE bytes; DEFINE FIELD body ON {table} TYPE {}; DEFINE INDEX semantic_key ON {table} FIELDS semantic_type,semantic_key UNIQUE;",shapes.into_iter().collect::<Vec<_>>().join(" | ")));
    }
    // Whole semantic IDs are atomic index keys. Array indexes flatten each byte and cannot
    // implement whole-ID IN selection; retain canonical typed arrays in body unchanged.
    for table in ["entity","assertion"] {
        for field in SCOPE_FIELDS {
            sql.push_str(&format!("DEFINE FIELD `scope_{field}` ON {table} TYPE option<string> VALUE IF body.`{field}` IS NONE THEN NONE ELSE <string>body.`{field}` END; DEFINE INDEX by_{field} ON {table} FIELDS semantic_type,`scope_{field}`,semantic_key;"));
        }
    }
    for (table,input) in [("participant","assertion"),("reference","entity")]{
        sql.push_str(&format!("DEFINE TABLE {table} TYPE RELATION IN {input} OUT entity | assertion | external ENFORCED SCHEMAFULL; DEFINE FIELD field ON {table} TYPE string; DEFINE FIELD role ON {table} TYPE int; DEFINE FIELD position ON {table} TYPE int | null; DEFINE INDEX incoming ON {table} FIELDS out,field,in; DEFINE INDEX outgoing ON {table} FIELDS in,field,out,position;"));
    }
    sql.push_str("DEFINE TABLE external TYPE NORMAL SCHEMAFULL; DEFINE FIELD canonical ON external TYPE bytes; DEFINE TABLE original TYPE NORMAL SCHEMAFULL; DEFINE FIELD content ON original TYPE string; DEFINE FIELD byte_len ON original TYPE int; DEFINE TABLE original_chunk TYPE NORMAL SCHEMAFULL; DEFINE FIELD source ON original_chunk TYPE record<original>; DEFINE FIELD start ON original_chunk TYPE int; DEFINE FIELD bytes ON original_chunk TYPE bytes; DEFINE FIELD content ON original_chunk TYPE string; DEFINE INDEX position ON original_chunk FIELDS source,start UNIQUE; DEFINE TABLE publication TYPE NORMAL SCHEMAFULL; DEFINE FIELD handle ON publication TYPE string; DEFINE FIELD manifest ON publication TYPE bytes;");sql
}
pub fn realization_identity(native_definitions:&str)->ContentHash{
    let mut bytes=canonical_schema().into_bytes();bytes.extend_from_slice(native_definitions.as_bytes());ContentHash::of(&bytes)
}
