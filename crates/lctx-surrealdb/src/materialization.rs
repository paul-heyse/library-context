//! Answer-affecting physical search records and installation policy.
use lctx_model::domain::{
    ContentHash, Id, ModelError,
    attribution::AnalysisContext,
    catalog::CatalogMember,
    retrieval::{ContentPart, Family, OriginalAnchor, SearchWindow, Unit, WindowBinding},
};
use serde::{Deserialize, Serialize};
use surrealdb::types::RecordId;

/// Shared only within a family, by exact window bytes. Corpus/window identity remains on occurrences.
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
    pub dependencies: Vec<RecordId>,
    pub unit_payload: RecordId,
    pub id: RecordId,
    pub r#in: RecordId,
    pub out: RecordId,
    pub family: Family,
    pub unit: Id<Unit>,
    pub unit_node: RecordId,
    pub window: Id<SearchWindow>,
    pub part: Id<ContentPart>,
    pub binding: Option<Id<WindowBinding>>,
    pub exact_name: String,
    pub exact_path: String,
    pub exact_option: String,
    pub context: Id<AnalysisContext>,
    pub member: Option<Id<CatalogMember>>,
    pub anchor: Option<Id<OriginalAnchor>>,
    pub input: [u8; 16],
    pub eligible: bool,
    pub occurrence_key: String,
}
/// One derived1024 row per projection/library-input/family cohort; canonical full4096 stays in graph payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchVector {
    pub dependencies: Vec<RecordId>,
    pub id: RecordId,
    pub encoder_hash: String,
    pub policy_key: String,
    pub library_input: String,
    pub family: i16,
    pub full_key: String,
    pub projection_key: String,
    pub embedding: Vec<f32>,
}

/// Installed before publication; this definition is part of the physical realization identity.
pub fn native_definitions() -> String {
    let mut sql =
        String::from("DEFINE ANALYZER lctx_discovery TOKENIZERS class,camel FILTERS lowercase;\n");
    for table in [
        "search_api_options",
        "search_documentation_deployment",
        "search_scenario",
        "search_source",
    ] {
        sql.push_str(&format!("DEFINE TABLE {table} SCHEMAFULL;\nDEFINE FIELD text ON {table} TYPE string;\nDEFINE FIELD digest ON {table} TYPE array<int,32>;\nDEFINE FIELD scope_digest ON {table} TYPE string VALUE <string>digest;\nDEFINE INDEX exact_text ON {table} FIELDS scope_digest UNIQUE;\nDEFINE INDEX lexical ON {table} FIELDS text FULLTEXT ANALYZER lctx_discovery BM25(1.5,0.75);\n"));
    }
    sql.push_str("DEFINE TABLE vector SCHEMAFULL;\nDEFINE FIELD encoder_hash ON vector TYPE string;\nDEFINE FIELD policy_key ON vector TYPE string;\nDEFINE FIELD library_input ON vector TYPE string;\nDEFINE FIELD family ON vector TYPE int;\nDEFINE FIELD full_key ON vector TYPE string;\nDEFINE FIELD projection_key ON vector TYPE string;\nDEFINE FIELD dependencies ON vector TYPE array<record<entity|assertion|compiler_record>>;\nDEFINE FIELD embedding ON vector TYPE array<float,1024>;\nDEFINE INDEX cohort ON vector FIELDS projection_key,library_input,family;\nDEFINE INDEX encoder_scope ON vector FIELDS encoder_hash;\nDEFINE INDEX policy_scope ON vector FIELDS policy_key;\nDEFINE INDEX library_scope ON vector FIELDS library_input;\nDEFINE INDEX family_scope ON vector FIELDS family;\nDEFINE INDEX neighbor ON vector FIELDS embedding HNSW DIMENSION 1024 DIST COSINE TYPE F32;\n");
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
        sql.push_str(&format!("DEFINE TABLE {table} TYPE RELATION IN {input} OUT entity_anchor ENFORCED SCHEMAFULL;\nDEFINE FIELD family ON {table} TYPE int;\nDEFINE FIELD unit ON {table} TYPE array<int,16>;\nDEFINE FIELD unit_node ON {table} TYPE record<entity_anchor>;\nDEFINE FIELD unit_payload ON {table} TYPE record<entity>;\nDEFINE FIELD dependencies ON {table} TYPE array<record<entity|assertion|compiler_record>>;\nDEFINE INDEX exact_unit_payload ON {table} FIELDS unit_payload;\nDEFINE FIELD window ON {table} TYPE array<int,16>;\nDEFINE FIELD part ON {table} TYPE array<int,16>;\nDEFINE FIELD binding ON {table} TYPE option<array<int,16>|null>;\nDEFINE FIELD exact_name ON {table} TYPE string;\nDEFINE FIELD exact_path ON {table} TYPE string;\nDEFINE FIELD exact_option ON {table} TYPE string;\nDEFINE INDEX exact_name ON {table} FIELDS exact_name;\nDEFINE INDEX exact_path ON {table} FIELDS exact_path;\nDEFINE INDEX exact_option ON {table} FIELDS exact_option;\nDEFINE FIELD context ON {table} TYPE array<int,16>;\nDEFINE FIELD input ON {table} TYPE array<int,16>;\nDEFINE FIELD member ON {table} TYPE option<array<int,16>|null>;\nDEFINE FIELD anchor ON {table} TYPE option<array<int,16>|null>;\nDEFINE FIELD eligible ON {table} TYPE bool;\nDEFINE FIELD occurrence_key ON {table} TYPE string;\nDEFINE FIELD scope_input ON {table} TYPE string VALUE <string>input;\nDEFINE FIELD scope_member ON {table} TYPE string VALUE <string>member;\nDEFINE FIELD scope_context ON {table} TYPE string VALUE <string>context;\nDEFINE INDEX occurrence ON {table} FIELDS {occurrence_fields} UNIQUE;\nDEFINE INDEX eligible_input ON {table} FIELDS eligible,scope_input,family,scope_member,scope_context;\nDEFINE INDEX document_occurrences ON {table} FIELDS in,eligible,scope_input,scope_member,scope_context,occurrence_key;\nDEFINE INDEX target_occurrences ON {table} FIELDS out,family,occurrence_key;\nDEFINE FIELD scope_window ON {table} TYPE string VALUE <string>window;\nDEFINE INDEX window_occurrences ON {table} FIELDS scope_window,eligible;\n"));
    }
    sql.push_str(&crate::lexical_stats::definitions());
    sql
}

/// Native-owned read-only first-party capture program used by the serving blueprint.
pub fn library_definitions() -> &'static str {
    r#"
DEFINE FUNCTION fn::lctx_library_roots($name: option<string|null>) {
 LET $packages=SELECT VALUE id FROM entity WHERE semantic_type='packages' AND ($name=NONE OR $name=NULL OR body.name=$name);
 LET $releases=SELECT VALUE in FROM reference WHERE field='package' AND out IN $packages;
 LET $all_distributions=SELECT VALUE in FROM participant WHERE field='release' AND out IN $releases;
 LET $distributions=SELECT VALUE id FROM assertion WHERE id IN $all_distributions AND semantic_type='input_distributions' AND body.role=0;
 LET $inputs=SELECT VALUE out FROM participant WHERE in IN $distributions AND field='input';
 LET $corpora=SELECT VALUE in FROM participant WHERE out IN $inputs AND field='library' AND in.semantic_type='corpus_libraries';
 LET $corpus_inputs=SELECT VALUE out FROM participant WHERE in IN $corpora AND field='corpus';
 LET $all_inputs=array::distinct(array::concat($inputs,$corpus_inputs));
 LET $runs=SELECT VALUE in FROM reference WHERE out IN $all_inputs AND field='input' AND in.semantic_type='provider_runs';
 LET $sources=SELECT VALUE in FROM reference WHERE out IN $all_inputs AND field='input' AND in.semantic_type='source_artifacts';
 LET $modules=SELECT VALUE in FROM reference WHERE out IN $sources AND field='source' AND in.semantic_type='modules';
 LET $scope_targets=array::distinct(array::concat($all_inputs,$releases,$sources,$modules));
 LET $scopes=SELECT VALUE in FROM reference WHERE out IN $scope_targets AND field IN ['input','release','artifact','module'] AND in.semantic_type='coverage_scopes';
 LET $coverage=array::distinct(array::concat((SELECT VALUE in FROM participant WHERE out IN $runs AND field='run' AND in.semantic_type='provider_coverage'),(SELECT VALUE in FROM participant WHERE out IN $scopes AND field='scope' AND in.semantic_type='provider_coverage')));
 RETURN array::distinct(array::concat($distributions,$inputs,$corpora,$corpus_inputs,$runs,$coverage));
};
"#
}

/// Accept only the exact physical programs generated by the current publication consumers.
/// The operation-definition digest is data inside one fixed function body, never caller SQL.
pub fn validate_native_definitions(definitions: &str) -> Result<(), ModelError> {
    let search = native_definitions();
    if definitions == search {
        return Ok(());
    }
    let operation = definitions
        .strip_prefix(search.as_str())
        .and_then(|suffix| suffix.strip_prefix(library_definitions()))
        .and_then(|suffix| {
            suffix.strip_prefix("DEFINE FUNCTION fn::lctx_operation_definition() { RETURN '")
        })
        .and_then(|suffix| suffix.strip_suffix("'; };\n"));
    if operation.is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }) {
        Ok(())
    } else {
        Err(ModelError::Conflict(
            "native publication executable blueprint",
        ))
    }
}

#[cfg(test)]
mod blueprint_controls {
    use super::*;

    fn serving(digest: &str) -> String {
        format!(
            "{}{}DEFINE FUNCTION fn::lctx_operation_definition() {{ RETURN '{digest}'; }};\n",
            native_definitions(),
            library_definitions()
        )
    }

    #[test]
    fn generated_search_and_serving_blueprints_are_the_only_installation_routes() {
        validate_native_definitions(&native_definitions()).unwrap();
        for digest in ["0".repeat(64), "0123456789abcdef".repeat(4)] {
            validate_native_definitions(&serving(&digest)).unwrap();
        }
        let full = serving(&"a".repeat(64));
        for unsupported in [
            "".to_owned(),
            "INSERT INTO entity { body: {} };".to_owned(),
            "DEFINE FIELD body ON entity TYPE object FLEXIBLE;".to_owned(),
            native_definitions() + "INSERT INTO compiler_record { body: {} };",
            full.clone() + "INSERT INTO entity { body: {} };",
            full.replace("RETURN '", "INSERT INTO entity { body: {} }; RETURN '"),
            full.replace(
                "DEFINE TABLE vector SCHEMAFULL",
                "DEFINE TABLE entity SCHEMALESS",
            ),
            full.clone() + "DEFINE FIELD body ON assertion TYPE object FLEXIBLE;",
            format!("{}{}", native_definitions(), library_definitions()),
            full.trim_end().to_owned(),
        ] {
            assert!(
                validate_native_definitions(&unsupported).is_err(),
                "unsupported blueprint accepted"
            );
        }
    }

    #[test]
    fn operation_digest_is_exact_lowercase_hex_and_cannot_change_the_function_body() {
        for digest in [
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
            "é".repeat(32),
            format!(
                "{}'; INSERT INTO entity {{ body: {{}} }}; RETURN 'x",
                "a".repeat(64)
            ),
        ] {
            assert!(
                validate_native_definitions(&serving(&digest)).is_err(),
                "malformed operation digest accepted"
            );
        }
    }
}
