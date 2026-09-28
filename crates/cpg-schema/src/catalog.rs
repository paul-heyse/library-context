//! Mandatory public contract catalog (ADR-0072). Canonical rows, never authored knowledge.
use crate::{
    id::{Digest, Id},
    table::table,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompileProfile {
    #[default]
    Catalog,
    Behavioral,
}
impl CompileProfile {
    pub fn name(self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::Behavioral => "behavioral",
        }
    }
    pub fn behavioral(self) -> bool {
        self == Self::Behavioral
    }
}

/// Capability presence is selected and validated, never inferred from nonempty rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub catalog: bool,
    pub behavioral_claims: bool,
    pub native_value_paths: bool,
    pub briefs: bool,
}
impl Capabilities {
    pub fn for_profile(profile: CompileProfile) -> Self {
        Self {
            catalog: true,
            behavioral_claims: profile.behavioral(),
            native_value_paths: profile.behavioral(),
            briefs: profile.behavioral(),
        }
    }
    pub fn valid(&self) -> bool {
        self.catalog
            && (!self.native_value_paths || self.behavioral_claims)
            && (!self.briefs || self.behavioral_claims)
    }
}

pub fn validate_roots(roots: &[String]) -> Result<(), String> {
    if roots.is_empty() {
        return Err("at least one explicit public root is required".into());
    }
    for root in roots {
        if !root.split('.').all(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
                && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        }) {
            return Err(format!("{root:?} is not a dotted Python public root"));
        }
    }
    Ok(())
}
pub fn under_roots(path: &str, roots: &[String]) -> bool {
    roots
        .iter()
        .any(|root| path == root || path.strip_prefix(root).is_some_and(|s| s.starts_with('.')))
}

table!(
    /// Exactly one selection/provenance row per attempt, even for an empty catalog.
    CatalogCompilation, CatalogCompilationRow = "catalog_compilation", family = Findings,
    key = [snapshot_id], checks = [("profile", "profile = 'catalog' OR profile = 'behavioral'")],
    { snapshot_id: Id, profile: String, public_roots: Vec<String>, input_digest: Digest }
);
table!(
    /// A public exposure. Resolution knowledge never enters member identity.
    CatalogMembers, CatalogMembersRow = "catalog_members", family = Findings,
    key = [snapshot_id, member_id], checks = [("path", "access_path <> ''")],
    { snapshot_id: Id, member_id: Id, access_path: String, owner_path: String,
      operation_node_id: Option<Id>, kind: String, resolution: String,
      brief_status: String, brief_reason: Option<String> }
);
table!(
    /// Attributed alternatives: source observations are not invented runtime possibilities.
    CatalogBindings, CatalogBindingsRow = "catalog_bindings", family = Findings,
    key = [snapshot_id, member_id, binding_id], checks = [],
    { snapshot_id: Id, member_id: Id, binding_id: Id, declaration_node_id: Option<Id>,
      source_fact_id: Id, role: String, own: bool, defining_path: Option<String> }
);
table!(
    /// One signature observation, shared by all exposures of its callable.
    CatalogSignatures, CatalogSignaturesRow = "catalog_signatures", family = Findings,
    key = [snapshot_id, signature_id], checks = [],
    { snapshot_id: Id, signature_id: Id, callable_node_id: Id, declaration_node_id: Option<Id>,
      constructor_class_id: Option<Id>, module_node_id: Id, function_key: Option<String>,
      signature_index: Option<i64>, role: String, form: String, source_fact_id: Id,
      reason: Option<String>, return_annotation: Option<String>, docstring: Option<String> }
);
table!(
    /// Ordered formals with source and provider observations kept separate.
    CatalogParameters, CatalogParametersRow = "catalog_parameters", family = Findings,
    key = [snapshot_id, signature_id, ordinal], checks = [("ordinal", "ordinal >= 0")],
    { snapshot_id: Id, signature_id: Id, ordinal: i64, formal_node_id: Option<Id>,
      name: Option<String>, kind: Option<String>, required: Option<bool>,
      syntax_fact_id: Option<Id>, semantics_fact_id: Option<Id>, provider_name: Option<String>,
      provider_kind: Option<String>, annotation_text: Option<String>, provider_annotation: Option<String>,
      default_state: String, default_text: Option<String>, literal_json: Option<String>,
      documentation: Option<String>, reason: Option<String> }
);
table!(
    /// Original pinned bytes reached from catalog subjects without synthetic assertions.
    CatalogEvidence, CatalogEvidenceRow = "catalog_evidence", family = Findings,
    key = [snapshot_id, evidence_id], checks = [("span", "start_byte >= 0 AND end_byte >= start_byte")],
    { snapshot_id: Id, evidence_id: Id, subject_node_id: Id, source_fact_id: Id,
      source_digest: Digest, path: String, start_byte: i64, end_byte: i64,
      role: String, text: String }
);

/// Reuse canonical flat schemas for these PG/IPC relations, omitting snapshot context only.
pub fn serving_files() -> Vec<crate::bundle::ServingFile> {
    use crate::Table;
    fn file<T: Table>() -> crate::bundle::ServingFile {
        crate::bundle::ServingFile {
            name: T::NAME,
            schema: std::sync::Arc::new(arrow_schema::Schema::new(
                T::schema()
                    .fields()
                    .iter()
                    .filter(|f| f.name() != "snapshot_id")
                    .cloned()
                    .collect::<Vec<_>>(),
            )),
            key: &T::key()[1..],
        }
    }
    vec![
        file::<CatalogMembers>(),
        file::<CatalogBindings>(),
        file::<CatalogSignatures>(),
        file::<CatalogParameters>(),
        file::<CatalogEvidence>(),
        file::<CatalogTypes>(),
        file::<CatalogTypeArgs>(),
        file::<CatalogTypeObservations>(),
        file::<CatalogConstructors>(),
    ]
}

table!(
    /// Reachable structural type terms, not display strings promoted to type identity.
    CatalogTypes, CatalogTypesRow = "catalog_types", family = Findings,
    key = [snapshot_id, term_id], checks = [],
    { snapshot_id: Id, term_id: Id, source_fact_id: Id, kind: String, display: String,
      detail: Option<String>, class_module: Option<String>, class_key: Option<String>, variable: Option<String> }
);
table!(
    CatalogTypeArgs, CatalogTypeArgsRow = "catalog_type_args", family = Findings,
    key = [snapshot_id, parent_term_id, role, ordinal], checks = [("ordinal", "ordinal >= 0")],
    { snapshot_id: Id, parent_term_id: Id, role: String, ordinal: i64, child_term_id: Id,
      source_fact_id: Id, name: Option<String>, parameter_kind: Option<String>, required: Option<bool> }
);
table!(
    CatalogTypeObservations, CatalogTypeObservationsRow = "catalog_type_observations", family = Findings,
    key = [snapshot_id, source_fact_id], checks = [],
    { snapshot_id: Id, source_fact_id: Id, subject_node_id: Id, role: String, declared: bool, term_id: Id }
);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_preserve_dotted_member_scope_and_segment_boundaries() {
        let roots = vec!["pkg.a".into(), "pkg.a.Class.method".into()];
        validate_roots(&roots).unwrap();
        assert!(under_roots("pkg.a", &roots));
        assert!(under_roots("pkg.a.Class.method", &roots));
        assert!(!under_roots("pkg.ab", &roots));
        assert!(!under_roots("pkg", &roots));
        assert!(validate_roots(&["pkg..a".into()]).is_err());
    }
    #[test]
    fn native_requires_behavioral_capability() {
        let mut caps = Capabilities::for_profile(CompileProfile::Catalog);
        assert!(caps.valid());
        caps.native_value_paths = true;
        assert!(!caps.valid());
        assert!(Capabilities::for_profile(CompileProfile::Behavioral).valid());
    }
}

/// Finite wire vocabularies, shared by canonical validation, IPC import and PostgreSQL DDL.
pub fn vocabulary(relation: &str, field: &str) -> Option<Vec<&'static str>> {
    use crate::codebook::{Codebook, ParameterKind, TypeArgRole, TypeRole, TypeTermKind};
    Some(match (relation, field) {
        ("catalog_compilation", "profile") => vec!["catalog", "behavioral"],
        ("catalog_members", "brief_status") => {
            vec!["not_requested", "not_selected", "skipped", "available"]
        }
        ("catalog_members", "resolution") => {
            vec!["unresolved", "source_known_effective_unresolved"]
        }
        ("catalog_bindings", "role") => vec![
            "selected_source",
            "overload",
            "shadowed_source",
            "stub_source",
            "source_alternative",
            "provider_public_observation",
        ],
        ("catalog_signatures", "role") => vec!["source", "overload", "provider_constructor"],
        ("catalog_signatures", "form") => vec!["source_list", "list", "ellipsis", "param_spec"],
        ("catalog_parameters", "default_state") => vec![
            "absent",
            "literal_none",
            "literal",
            "source_expression",
            "optional_expression_unavailable",
            "unknown",
        ],
        ("catalog_parameters", "kind" | "provider_kind")
        | ("catalog_type_args", "parameter_kind") => {
            ParameterKind::all().iter().map(|v| v.text()).collect()
        }
        ("catalog_evidence", "role") => vec!["declares"],
        ("catalog_types", "kind") => TypeTermKind::all().iter().map(|v| v.text()).collect(),
        ("catalog_type_observations", "role") => TypeRole::all().iter().map(|v| v.text()).collect(),
        ("catalog_type_args", "role") => TypeArgRole::all().iter().map(|v| v.text()).collect(),
        _ => return None,
    })
}

/// Tables produced in both profiles. Everything else in the analysis family is enrichment.
pub fn mandatory_table(name: &str) -> bool {
    name.starts_with("catalog_")
        || matches!(
            name,
            "public_paths"
                | "operations"
                | "operation_facets"
                | "operation_facet_status"
                | "operation_documents"
                | "embedding_specs"
                | "used_embeddings"
                | "embedding_uses"
                | "assertion_policy"
        )
}

table!(
    /// A class exposure's constructor signatures, including attributed inherited contracts.
    CatalogConstructors, CatalogConstructorsRow = "catalog_constructors", family = Findings,
    key = [snapshot_id, class_node_id, signature_id], checks = [],
    { snapshot_id: Id, class_node_id: Id, signature_id: Id, own: bool, ancestry_fact_id: Option<Id> }
);
