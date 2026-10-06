//! Answer-affecting physical search records and installation policy.
use lctx_model::domain::{
    ContentHash, Id,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    retrieval::{Family, Fragment, OriginalAnchor, Unit},
};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// Shared only within a family, by exact fragment bytes. Corpus/window identity remains on occurrences.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchDocument {
    pub id: RecordId,
    pub text: String,
    pub digest: ContentHash,
}
/// Eligible contextual occurrence. Both endpoints and nominal IDs are intentional:
/// graph traversal uses endpoints; wire packets retain their original nominal semantic identities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchOccurrence {
    pub id: RecordId,
    pub r#in: RecordId,
    pub out: RecordId,
    pub family: Family,
    pub unit: Id<Unit>,
    pub fragment: Id<Fragment>,
    pub context: Id<AnalysisContext>,
    pub member: Option<Id<CatalogMember>>,
    pub anchor: Option<Id<OriginalAnchor>>,
    pub input: [u8; 16],
    pub eligible: bool,
    pub occurrence_key: String,
}
/// The admitted exact 1024-dimensional value is shared by its spec/input, never by a label.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchVector {
    pub id: RecordId,
    pub specification: ContentHash,
    pub input: ContentHash,
    pub digest: ContentHash,
    pub bytes: Vec<u8>,
    pub embedding: Vec<f32>,
}

/// Installed before publication; this definition is part of the physical realization identity.
pub fn native_definitions() -> String {
    let mut sql =
        String::from("DEFINE ANALYZER lctx_discovery TOKENIZERS class FILTERS lowercase;\n");
    for table in [
        "search_api_options",
        "search_documentation_deployment",
        "search_scenario",
        "search_source",
    ] {
        sql.push_str(&format!("DEFINE TABLE {table} SCHEMAFULL;\nDEFINE FIELD text ON {table} TYPE string;\nDEFINE FIELD digest ON {table} TYPE array<int,32>;\nDEFINE FIELD scope_digest ON {table} TYPE string VALUE <string>digest;\nDEFINE INDEX exact_text ON {table} FIELDS scope_digest UNIQUE;\nDEFINE INDEX lexical ON {table} FIELDS text FULLTEXT ANALYZER lctx_discovery BM25(1.5,0.75);\n"));
    }
    sql.push_str("DEFINE TABLE vector SCHEMAFULL;\nDEFINE FIELD specification ON vector TYPE array<int,32>;\nDEFINE FIELD input ON vector TYPE array<int,32>;\nDEFINE FIELD digest ON vector TYPE array<int,32>;\nDEFINE FIELD bytes ON vector TYPE bytes;\nDEFINE FIELD embedding ON vector TYPE array<float,1024>;\nDEFINE FIELD scope_specification ON vector TYPE string VALUE <string>specification;\nDEFINE FIELD scope_input ON vector TYPE string VALUE <string>input;\nDEFINE INDEX exact_value ON vector FIELDS scope_specification,scope_input UNIQUE;\nDEFINE INDEX neighbor ON vector FIELDS embedding HNSW DIMENSION 1024 DIST COSINE TYPE F32;\n");
    for (table, input) in [
        (
            "lex_occurs",
            "search_api_options|search_documentation_deployment|search_scenario|search_source",
        ),
        ("vec_occurs", "vector"),
    ] {
        let occurrence_fields = if table == "vec_occurs" {
            "in,occurrence_key"
        } else {
            "occurrence_key"
        };
        sql.push_str(&format!("DEFINE TABLE {table} TYPE RELATION IN {input} OUT entity ENFORCED SCHEMAFULL;\nDEFINE FIELD family ON {table} TYPE int;\nDEFINE FIELD unit ON {table} TYPE array<int,16>;\nDEFINE FIELD fragment ON {table} TYPE array<int,16>;\nDEFINE FIELD context ON {table} TYPE array<int,16>;\nDEFINE FIELD input ON {table} TYPE array<int,16>;\nDEFINE FIELD member ON {table} TYPE option<array<int,16>|null>;\nDEFINE FIELD anchor ON {table} TYPE option<array<int,16>|null>;\nDEFINE FIELD eligible ON {table} TYPE bool;\nDEFINE FIELD occurrence_key ON {table} TYPE string;\nDEFINE FIELD scope_input ON {table} TYPE string VALUE <string>input;\nDEFINE FIELD scope_member ON {table} TYPE string VALUE <string>member;\nDEFINE FIELD scope_context ON {table} TYPE string VALUE <string>context;\nDEFINE INDEX occurrence ON {table} FIELDS {occurrence_fields} UNIQUE;\nDEFINE INDEX eligible_input ON {table} FIELDS eligible,scope_input,family,scope_member,scope_context;\nDEFINE INDEX document_occurrences ON {table} FIELDS in,eligible,scope_input,scope_member,scope_context,occurrence_key;\nDEFINE INDEX target_occurrences ON {table} FIELDS out,family,occurrence_key;\n"));
    }
    sql
}
