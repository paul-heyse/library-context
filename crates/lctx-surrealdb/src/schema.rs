//! Fixed physical envelopes; typed declarations govern ingress and independent reconstruction.
use lctx_model::domain::{ContentHash, Field, Record, Relation};
use std::collections::BTreeSet;
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
/// The physical owner selects its own declared field inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeTable {
    Entity,
    Assertion,
    CompilerRecord,
}
impl ScopeTable {
    pub fn name(self) -> &'static str {
        match self {
            Self::Entity => "entity",
            Self::Assertion => "assertion",
            Self::CompilerRecord => "compiler_record",
        }
    }
    pub fn from_name(name: &str) -> Result<Self, lctx_model::domain::ModelError> {
        match name {
            "entity" => Ok(Self::Entity),
            "assertion" => Ok(Self::Assertion),
            "compiler_record" => Ok(Self::CompilerRecord),
            _ => Err(lctx_model::domain::ModelError::Schema("native scope table")),
        }
    }
    pub fn for_relation(name: &str) -> Result<Self, lctx_model::domain::ModelError> {
        if name == "__graph_assertion" {
            return Ok(Self::Assertion);
        }
        Ok(crate::adapter::select(name)?.table)
    }
    pub fn relations(self) -> &'static [Relation] {
        static ENTITIES: std::sync::OnceLock<Vec<Relation>> = std::sync::OnceLock::new();
        static ASSERTIONS: std::sync::OnceLock<Vec<Relation>> = std::sync::OnceLock::new();
        match self {
            Self::Entity=>ENTITIES.get_or_init(||{let mut relations=Vec::new();macro_rules! collect {($($variant:ident:$ty:ty,)*)=>{$(relations.push(Relation::of::<$ty>());)*};} lctx_model::graph_entity_records!(collect);relations}),
            Self::Assertion=>ASSERTIONS.get_or_init(||{let mut relations=Vec::new();macro_rules! collect {($($variant:ident:$ty:ty,)*)=>{$(relations.push(Relation::of::<$ty>());)*};} lctx_model::graph_assertion_records!(collect);relations}),
            Self::CompilerRecord=>compiler_relations(),
        }
    }
    /// Explicit historical scope semantics apply only to actual non-list physical columns.
    /// Artifact chunks replace body bytes with a physical original pointer, which is not a
    /// nominal reference and must never enter this inventory.
    fn layout(self) -> &'static std::collections::BTreeMap<&'static str, BTreeSet<&'static str>> {
        type Layout = std::collections::BTreeMap<&'static str, BTreeSet<&'static str>>;
        static ENTITY: std::sync::OnceLock<Layout> = std::sync::OnceLock::new();
        static ASSERTION: std::sync::OnceLock<Layout> = std::sync::OnceLock::new();
        static COMPILER: std::sync::OnceLock<Layout> = std::sync::OnceLock::new();
        let owner = match self {
            Self::Entity => &ENTITY,
            Self::Assertion => &ASSERTION,
            Self::CompilerRecord => &COMPILER,
        };
        owner.get_or_init(|| {
            self.relations()
                .iter()
                .map(|relation| {
                    let fields = relation
                        .fields()
                        .iter()
                        .filter(|field| {
                            !field.list()
                                && (field.target().is_some()
                                    || SCOPE_FIELDS.contains(&field.name()))
                        })
                        .map(Field::name)
                        .collect();
                    (relation.name(), fields)
                })
                .collect()
        })
    }
    pub fn relation_fields(self, relation: &str) -> BTreeSet<&'static str> {
        self.layout().get(relation).cloned().unwrap_or_default()
    }
    pub fn fields(self) -> BTreeSet<&'static str> {
        self.layout().values().flatten().copied().collect()
    }
    pub fn contains(self, relation: &str, field: &str) -> bool {
        self.layout()
            .get(relation)
            .is_some_and(|fields| fields.contains(field))
    }
}
pub fn atomic_scope_field(relation: &str, field: &str) -> bool {
    ScopeTable::for_relation(relation).is_ok_and(|table| table.contains(relation, field))
}
/// All model-required families without a graph lowering share one closed native backing.
/// Derive this inventory from the same declarations that own compiler model membership.
pub(crate) fn compiler_relations() -> &'static [Relation] {
    static RELATIONS: std::sync::OnceLock<Vec<Relation>> = std::sync::OnceLock::new();
    RELATIONS.get_or_init(|| {
        let mut graph = BTreeSet::new();
        macro_rules! collect {($($variant:ident:$ty:ty,)*)=>{$(graph.insert(<$ty>::NAME);)*};}
        lctx_model::graph_entity_records!(collect);
        lctx_model::graph_assertion_records!(collect);
        let mut relations = lctx_model::domain::catalog_frontier_relations()
            .into_iter()
            .filter(|relation| !graph.contains(relation.name()))
            .collect::<Vec<_>>();
        relations.sort_by_key(Relation::name);
        relations.dedup_by_key(|relation| relation.name());
        relations
    })
}
/// Original bytes have their own physical owner; all other private families retain declared fields.
pub fn compiler_record_schema() -> String {
    let mut sql="DEFINE TABLE compiler_record SCHEMAFULL; DEFINE FIELD semantic_type ON compiler_record TYPE string; DEFINE FIELD semantic_key ON compiler_record TYPE string; DEFINE FIELD content ON compiler_record TYPE string; DEFINE FIELD canonical ON compiler_record TYPE bytes; DEFINE FIELD body ON compiler_record TYPE object FLEXIBLE; DEFINE INDEX compiler_record_key ON compiler_record FIELDS semantic_type,semantic_key;".to_string();
    sql.push_str(&scope_schema("compiler_record"));
    sql
}
fn scope_schema(table: &str) -> String {
    let layout = ScopeTable::from_name(table).expect("generated canonical scope table");
    let mut sql = String::new();
    if layout != ScopeTable::CompilerRecord {
        sql.push_str(&format!(
            "DEFINE FIELD scope_context ON {table} TYPE option<string>;"
        ));
    }
    sql.push_str(&format!("DEFINE FIELD scope_keys ON {table} TYPE array<string>; DEFINE INDEX by_scope ON {table} FIELDS scope_keys.*,semantic_key;"));
    sql
}
pub fn canonical_schema() -> String {
    let mut sql = String::new();
    for table in ["entity", "assertion"] {
        sql.push_str(&format!("DEFINE TABLE {table} TYPE NORMAL SCHEMAFULL; DEFINE FIELD semantic_type ON {table} TYPE string; DEFINE FIELD semantic_key ON {table} TYPE string; DEFINE FIELD anchor ON {table} TYPE record<entity_anchor | assertion_anchor>; DEFINE FIELD kind ON {table} TYPE int; DEFINE FIELD subtype ON {table} TYPE int | null; DEFINE FIELD content ON {table} TYPE string; DEFINE INDEX anchor_payload ON {table} FIELDS anchor,content; DEFINE FIELD canonical ON {table} TYPE bytes; DEFINE FIELD body ON {table} TYPE object FLEXIBLE; DEFINE INDEX semantic_key ON {table} FIELDS semantic_type,semantic_key; DEFINE TABLE {table}_anchor SCHEMAFULL; DEFINE FIELD family ON {table}_anchor TYPE string; DEFINE FIELD nominal ON {table}_anchor TYPE string; DEFINE INDEX anchor_nominal ON {table}_anchor FIELDS nominal UNIQUE;"));
    }
    // Whole semantic IDs are atomic index keys. Array indexes flatten each byte and cannot
    // implement whole-ID IN selection; retain canonical typed arrays in body unchanged.
    for table in ["entity", "assertion"] {
        sql.push_str(&scope_schema(table));
    }
    for (table, input) in [("participant", "assertion"), ("reference", "entity")] {
        sql.push_str(&format!("DEFINE TABLE {table} TYPE RELATION IN {input} OUT entity_anchor | assertion_anchor | external ENFORCED SCHEMAFULL; DEFINE FIELD field ON {table} TYPE string; DEFINE FIELD role ON {table} TYPE int; DEFINE FIELD position ON {table} TYPE int | null; DEFINE INDEX incoming ON {table} FIELDS out,field,in; DEFINE INDEX outgoing ON {table} FIELDS in,field,out,position;"));
    }
    sql.push_str("DEFINE TABLE external TYPE NORMAL SCHEMAFULL; DEFINE FIELD canonical ON external TYPE bytes; DEFINE TABLE original TYPE NORMAL SCHEMAFULL; DEFINE FIELD content ON original TYPE string; DEFINE FIELD byte_len ON original TYPE int; DEFINE TABLE original_chunk TYPE NORMAL SCHEMAFULL; DEFINE FIELD source ON original_chunk TYPE record<original>; DEFINE FIELD start ON original_chunk TYPE int; DEFINE FIELD bytes ON original_chunk TYPE bytes; DEFINE FIELD content ON original_chunk TYPE string; DEFINE INDEX position ON original_chunk FIELDS source,start UNIQUE; DEFINE TABLE publication TYPE NORMAL SCHEMAFULL; DEFINE FIELD handle ON publication TYPE string; DEFINE FIELD manifest ON publication TYPE bytes; DEFINE FIELD views ON publication TYPE bytes; DEFINE FIELD definition_epoch ON publication TYPE string;");
    sql
}
pub fn realization_identity(native_definitions: &str) -> ContentHash {
    let mut bytes = canonical_schema().into_bytes();
    bytes.extend_from_slice(compiler_record_schema().as_bytes());
    bytes.extend_from_slice(native_definitions.as_bytes());
    ContentHash::of(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::types::ToSql;
    #[test]
    fn scope_inventory_is_table_specific_and_keeps_physical_semantics() {
        use lctx_model::domain::{
            analytics::QualityStep, artifact::ArtifactChunk,
            embedding::analytic::AnalysisEmbeddingUse, input::Release, source::CoverageScope,
        };
        assert_eq!(
            ScopeTable::for_relation(QualityStep::NAME).unwrap(),
            ScopeTable::CompilerRecord
        );
        assert!(ScopeTable::CompilerRecord.contains(QualityStep::NAME, "run"));
        assert!(!ScopeTable::Entity.contains(QualityStep::NAME, "run"));
        assert!(ScopeTable::Entity.contains(Release::NAME, "package"));
        assert!(ScopeTable::CompilerRecord.contains(ArtifactChunk::NAME, "artifact"));
        assert!(!ScopeTable::CompilerRecord.contains(ArtifactChunk::NAME, "original"));
        assert!(!ScopeTable::CompilerRecord.contains(ArtifactChunk::NAME, "body"));
        let raw_relation = Relation::of::<AnalysisEmbeddingUse>();
        assert!(
            raw_relation
                .fields()
                .iter()
                .any(|field| field.name() == "input"
                    && field.target().is_none()
                    && field.scalar() == lctx_model::domain::Scalar::Digest)
        );
        assert!(ScopeTable::Assertion.contains(AnalysisEmbeddingUse::NAME, "input"));
        let table = ScopeTable::for_relation(CoverageScope::NAME).unwrap();
        let fields = table.relation_fields(CoverageScope::NAME);
        assert!(fields.contains("input_input"));
        assert!(fields.contains("artifact_artifact"));
        assert!(!fields.contains("input"));
        for table in [
            ScopeTable::Entity,
            ScopeTable::Assertion,
            ScopeTable::CompilerRecord,
        ] {
            let sql = scope_schema(table.name());
            assert_eq!(
                sql.contains("DEFINE FIELD scope_context"),
                table != ScopeTable::CompilerRecord
            );
            assert!(!sql.contains("DEFINE FIELD `scope_"));
            for relation in table.relations() {
                for field in relation.fields() {
                    if !field.list() && SCOPE_FIELDS.contains(&field.name()) {
                        assert!(table.contains(relation.name(), field.name()));
                    }
                }
            }
        }
        let mut bytes = canonical_schema().into_bytes();
        bytes.extend_from_slice(compiler_record_schema().as_bytes());
        bytes.extend_from_slice(b"native functions");
        assert_eq!(
            realization_identity("native functions"),
            ContentHash::of(&bytes)
        );
    }
    #[test]
    fn inactive_sum_fields_and_original_pointer_do_not_become_scope_keys() {
        use lctx_model::domain::{artifact::ArtifactChunk, source::CoverageScope};
        use surrealdb::types::{Object, RecordId, Value};
        let table = ScopeTable::for_relation(CoverageScope::NAME).unwrap();
        let mut body = Object::new();
        body.insert("input_input", vec![3i64; 16]);
        body.insert("artifact_artifact", Value::Null);
        let mut row = Object::new();
        crate::reconciliation::add_scope_fields(
            &mut row,
            &Value::Object(body),
            CoverageScope::NAME,
            table,
        )
        .unwrap();
        assert_eq!(row.len(), 1);
        assert_eq!(
            row.get("scope_keys"),
            Some(&Value::from_t(vec![format!(
                "{}|input_input|{}",
                CoverageScope::NAME,
                Value::from_t(vec![3i64; 16]).to_sql()
            )]))
        );
        let mut body = Object::new();
        body.insert("artifact", vec![4i64; 16]);
        body.insert("original", RecordId::new("original", "physical"));
        let mut row = Object::new();
        crate::reconciliation::add_scope_fields(
            &mut row,
            &Value::Object(body),
            ArtifactChunk::NAME,
            ScopeTable::CompilerRecord,
        )
        .unwrap();
        let Value::Array(keys) = row.get("scope_keys").unwrap() else {
            panic!("scope keys");
        };
        assert_eq!(keys.len(), 1);
        assert!(!row.contains_key("scope_artifact"));
    }
    #[test]
    fn native_envelopes_are_fixed_and_scopes_are_supplied() {
        let canonical = canonical_schema();
        let compiler = compiler_record_schema();
        for sql in [&canonical, &compiler] {
            assert!(sql.contains("TYPE object FLEXIBLE"));
            assert!(!sql.contains(" VALUE "));
            assert!(!sql.contains("__type"));
        }
        let statements = format!("{canonical}{compiler}")
            .split(';')
            .filter(|statement| !statement.trim().is_empty())
            .map(|statement| format!("{};", statement.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!("fixed_native_envelopes", statements);
    }
}
