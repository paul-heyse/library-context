//! Closed physical schemas generated from the selected canonical graph vocabulary.
use lctx_model::domain::{ContentHash, Field, Record, Scalar};
use std::collections::BTreeSet;
fn scalar(field: &Field) -> String {
    let ty = match field.scalar() {
        Scalar::Text => "string".into(),
        Scalar::Bool => "bool".into(),
        Scalar::Int16 | Scalar::Int32 | Scalar::Int64 => {
            if field.codes().is_empty() {
                "int".into()
            } else {
                field
                    .codes()
                    .iter()
                    .map(|(v, _)| v.to_string())
                    .collect::<Vec<_>>()
                    .join(" | ")
            }
        }
        Scalar::Id => "array<int,16>".into(),
        Scalar::Digest => "array<int,32>".into(),
        Scalar::Binary if field.textual() => "string".into(),
        Scalar::Binary => "bytes".into(),
        Scalar::FiniteF64 => "float".into(),
    };
    let ty = if field.list() {
        format!("array<{ty}>")
    } else {
        ty
    };
    if field.nullable() {
        format!("{ty} | null")
    } else {
        ty
    }
}
fn declaration<R: Record>() -> String {
    let fields = R::fields()
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(sum) = R::sum() {
        sum.arms
            .into_iter()
            .map(|arm| {
                let mut parts = vec![format!("__type: '{}'", R::NAME)];
                for field in &fields {
                    let ty = if field.name() == sum.tag {
                        arm.code.to_string()
                    } else if let Some(active) = arm.fields.iter().find(|a| a.name == field.name())
                    {
                        let native = scalar(field);
                        if active.required {
                            native.replace(" | null", "")
                        } else {
                            native
                        }
                    } else {
                        "null".into()
                    };
                    parts.push(format!("`{}`: {ty}", field.name()));
                }
                format!("{{ {} }}", parts.join(", "))
            })
            .collect::<Vec<_>>()
            .join(" | ")
    } else {
        let mut parts = vec![format!("__type: '{}'", R::NAME)];
        parts.extend(
            fields
                .iter()
                .map(|f| format!("`{}`: {}", f.name(), scalar(f))),
        );
        format!("{{ {} }}", parts.join(", "))
    }
}
macro_rules! entities{($($variant:ident:$ty:ty,)*)=>{fn entity_shapes()->Vec<String>{vec![$(declaration::<$ty>(),)*]}}}
lctx_model::graph_entity_records!(entities);
macro_rules! assertions{($($variant:ident:$ty:ty,)*)=>{fn assertion_shapes()->Vec<String>{vec![$(declaration::<$ty>(),)*]}}}
lctx_model::graph_assertion_records!(assertions);
/// Active typed scope keys share one array-element index; other predicates use the type index.
pub const SCOPE_FIELDS: &[&str] = &[
    "member",
    "exposure",
    "candidate",
    "callable",
    "variant",
    "invocation",
    "domain",
    "unit",
    "window",
    "entity",
    "corpus",
    "specification",
    "input",
    "origin",
    "package",
    "access",
    "context",
    "root",
    "source",
    "target",
    "qualification",
    "slot",
    "parent",
    "inventory",
    "characterization",
    "event",
    "diagnostic",
    "entry",
    "trace",
    "attempt",
    "use_",
    "view",
    "assessment",
    "observation",
    "assertion",
    "release",
    "distribution",
    "module",
    "artifact",
    "brief",
    "support",
    "set",
    "universe",
];
/// One atomic scope index covers every declared scalar nominal reference, including compiler
/// ownership fields. The model owns this inventory; existing scalar scope predicates remain.
pub fn atomic_scope_fields()->&'static std::collections::BTreeSet<&'static str> {
    static FIELDS:std::sync::OnceLock<std::collections::BTreeSet<&'static str>>=std::sync::OnceLock::new();
    FIELDS.get_or_init(||{
        let mut fields=SCOPE_FIELDS.iter().copied().collect();
        fn collect<R:Record>(fields:&mut std::collections::BTreeSet<&'static str>){
            fields.extend(R::fields().into_iter().filter(|field|field.target().is_some() && !field.list()).map(|field|field.name()));
        }
        macro_rules! collect_registry {($($variant:ident:$ty:ty,)*)=>{$(collect::<$ty>(&mut fields);)*};}
        lctx_model::graph_entity_records!(collect_registry);
        lctx_model::graph_assertion_records!(collect_registry);
        collect::<lctx_model::domain::analytics::QualityStep>(&mut fields);
        collect::<lctx_model::domain::artifact::ArtifactChunk>(&mut fields);
        fields
    })
}
/// Non-graph compiler backing has a closed model-generated body, just like graph records.
pub fn compiler_record_schema() -> String {
    let shapes = [declaration::<lctx_model::domain::analytics::QualityStep>(), "{__type:'artifact_chunks', artifact:array<int,16>, ordinal:int, original:record<original>, start:int, len:int, digest:string}".into()];
    let mut sql=format!("DEFINE TABLE compiler_record SCHEMAFULL; DEFINE FIELD semantic_type ON compiler_record TYPE string; DEFINE FIELD semantic_key ON compiler_record TYPE string; DEFINE FIELD content ON compiler_record TYPE string; DEFINE FIELD canonical ON compiler_record TYPE bytes; DEFINE FIELD body ON compiler_record TYPE {}; DEFINE INDEX compiler_record_key ON compiler_record FIELDS semantic_type,semantic_key UNIQUE;", shapes.join(" | "));
    sql.push_str(&scope_schema("compiler_record"));
    sql
}
fn scope_schema(table:&str)->String {
    let mut sql=String::new();
        for field in atomic_scope_fields() {
            sql.push_str(&format!("DEFINE FIELD `scope_{field}` ON {table} TYPE option<string> VALUE IF body.`{field}` IS NONE THEN NONE ELSE <string>body.`{field}` END;"));
        }
        let active=atomic_scope_fields().iter().map(|field|format!("IF body.`{field}` IS NONE OR body.`{field}` IS NULL THEN NONE ELSE semantic_type+'|{field}|'+<string>body.`{field}` END")).collect::<Vec<_>>().join(",");
        sql.push_str(&format!("DEFINE FIELD scope_keys ON {table} TYPE array<string> VALUE [{active}].filter(|$value| $value IS NOT NONE); DEFINE INDEX by_scope ON {table} FIELDS scope_keys.*,semantic_key;"));
    sql
}
pub fn canonical_schema() -> String {
    let mut sql = String::new();
    for (table, shapes) in [
        ("entity", entity_shapes()),
        ("assertion", assertion_shapes()),
    ] {
        let mut shapes: BTreeSet<String> = shapes.into_iter().collect();
        if table == "assertion" {
            shapes.insert("{__type:'__graph_assertion'}".into());
        }
        sql.push_str(&format!("DEFINE TABLE {table} TYPE NORMAL SCHEMAFULL; DEFINE FIELD semantic_type ON {table} TYPE string; DEFINE FIELD semantic_key ON {table} TYPE string; DEFINE FIELD kind ON {table} TYPE int; DEFINE FIELD subtype ON {table} TYPE int | null; DEFINE FIELD content ON {table} TYPE string; DEFINE FIELD canonical ON {table} TYPE bytes; DEFINE FIELD body ON {table} TYPE {}; DEFINE INDEX semantic_key ON {table} FIELDS semantic_type,semantic_key UNIQUE;",shapes.into_iter().collect::<Vec<_>>().join(" | ")));
    }
    // Whole semantic IDs are atomic index keys. Array indexes flatten each byte and cannot
    // implement whole-ID IN selection; retain canonical typed arrays in body unchanged.
    for table in ["entity","assertion"] {sql.push_str(&scope_schema(table));}
    for (table, input) in [("participant", "assertion"), ("reference", "entity")] {
        sql.push_str(&format!("DEFINE TABLE {table} TYPE RELATION IN {input} OUT entity | assertion | external ENFORCED SCHEMAFULL; DEFINE FIELD field ON {table} TYPE string; DEFINE FIELD role ON {table} TYPE int; DEFINE FIELD position ON {table} TYPE int | null; DEFINE INDEX incoming ON {table} FIELDS out,field,in; DEFINE INDEX outgoing ON {table} FIELDS in,field,out,position;"));
    }
    sql.push_str("DEFINE TABLE external TYPE NORMAL SCHEMAFULL; DEFINE FIELD canonical ON external TYPE bytes; DEFINE TABLE original TYPE NORMAL SCHEMAFULL; DEFINE FIELD content ON original TYPE string; DEFINE FIELD byte_len ON original TYPE int; DEFINE TABLE original_chunk TYPE NORMAL SCHEMAFULL; DEFINE FIELD source ON original_chunk TYPE record<original>; DEFINE FIELD start ON original_chunk TYPE int; DEFINE FIELD bytes ON original_chunk TYPE bytes; DEFINE FIELD content ON original_chunk TYPE string; DEFINE INDEX position ON original_chunk FIELDS source,start UNIQUE; DEFINE TABLE publication TYPE NORMAL SCHEMAFULL; DEFINE FIELD handle ON publication TYPE string; DEFINE FIELD manifest ON publication TYPE bytes;");
    sql
}
pub fn realization_identity(native_definitions: &str) -> ContentHash {
    let mut bytes = canonical_schema().into_bytes();
    bytes.extend_from_slice(native_definitions.as_bytes());
    ContentHash::of(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_native_schemas_keep_opaque_bytes_and_closed_compiler_metadata(){
        let literals=declaration::<lctx_model::domain::value::Literal>();
        assert!(literals.contains("bytes"));assert!(!literals.contains("FLEXIBLE"));
        let compiler=compiler_record_schema();
        assert!(compiler.contains("__type:'artifact_chunks'"));assert!(compiler.contains("original:record<original>"));assert!(compiler.contains("artifact:array<int,16>"));
        assert!(compiler.contains(&declaration::<lctx_model::domain::analytics::QualityStep>()));
        assert!(compiler.contains("scope_keys"));assert!(compiler.contains("ON compiler_record"));assert!(!compiler.contains("FLEXIBLE"));
    }
}
