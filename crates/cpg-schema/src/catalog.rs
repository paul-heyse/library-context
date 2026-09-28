//! Mandatory public contract catalog (ADR-0072). Canonical rows, never authored knowledge.
use crate::{
    id::{Digest, Id},
    table::table,
};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema,
)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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
    CatalogCompilation, CatalogCompilationRow = "catalog_compilation", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id], checks = [("profile", "profile = 'catalog' OR profile = 'behavioral'")],
    { #[serde(skip)] snapshot_id: Id, #[serde(deserialize_with="crate::wire::decode_profile")] #[schemars(with="crate::wire::Profile")] profile: String, public_roots: Vec<String>, input_digest: Digest }
);
table!(
    /// A public exposure. Resolution knowledge never enters member identity.
    CatalogMembers, CatalogMembersRow = "catalog_members", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, member_id], checks = [("path", "access_path <> ''")],
    { #[serde(skip)] snapshot_id: Id, member_id: Id, access_path: String, owner_path: String,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] operation_node_id: Option<Id>, kind: String, #[serde(deserialize_with="crate::wire::decode_binding_resolution")] #[schemars(with="crate::wire::BindingResolution")] resolution: String,
      #[serde(deserialize_with="crate::wire::decode_brief_state")] #[schemars(with="crate::wire::BriefState")] brief_status: String, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] brief_reason: Option<String> }
);
table!(
    /// Attributed alternatives: source observations are not invented runtime possibilities.
    CatalogBindings, CatalogBindingsRow = "catalog_bindings", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, member_id, binding_id], checks = [],
    { #[serde(skip)] snapshot_id: Id, member_id: Id, binding_id: Id, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] declaration_node_id: Option<Id>,
      source_fact_id: Id, #[serde(deserialize_with="crate::wire::decode_binding_role")] #[schemars(with="crate::wire::BindingRole")] role: String, own: bool, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] defining_path: Option<String> }
);
table!(
    /// One signature observation, shared by all exposures of its callable.
    CatalogSignatures, CatalogSignaturesRow = "catalog_signatures", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, signature_id], checks = [],
    { #[serde(skip)] snapshot_id: Id, signature_id: Id, callable_node_id: Id, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] declaration_node_id: Option<Id>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] constructor_class_id: Option<Id>, module_node_id: Id, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] function_key: Option<String>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<i64>")] signature_index: Option<i64>, #[serde(deserialize_with="crate::wire::decode_signature_role")] #[schemars(with="crate::wire::SignatureRole")] role: String, #[serde(deserialize_with="crate::wire::decode_signature_form")] #[schemars(with="crate::wire::SignatureForm")] form: String, source_fact_id: Id,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] reason: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] return_annotation: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] docstring: Option<String> }
);
table!(
    /// Ordered formals with source and provider observations kept separate.
    CatalogParameters, CatalogParametersRow = "catalog_parameters", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, signature_id, ordinal], checks = [("ordinal", "ordinal >= 0")],
    { #[serde(skip)] snapshot_id: Id, signature_id: Id, ordinal: i64, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] formal_node_id: Option<Id>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] name: Option<String>, #[serde(deserialize_with="crate::wire::decode_parameter_kind_nullable")] #[schemars(with="crate::wire::Nullable<crate::wire::ParameterKind>")] kind: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<bool>")] required: Option<bool>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] syntax_fact_id: Option<Id>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] semantics_fact_id: Option<Id>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] provider_name: Option<String>,
      #[serde(deserialize_with="crate::wire::decode_provider_parameter_kind_nullable")] #[schemars(with="crate::wire::Nullable<crate::wire::ProviderParameterKind>")] provider_kind: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] annotation_text: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] provider_annotation: Option<String>,
      #[serde(deserialize_with="crate::wire::decode_default_state")] #[schemars(with="crate::wire::DefaultState")] default_state: String, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] default_text: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] literal_json: Option<String>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] documentation: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] reason: Option<String> }
);
table!(
    /// Original pinned bytes reached from catalog subjects without synthetic assertions.
    CatalogEvidence, CatalogEvidenceRow = "catalog_evidence", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, evidence_id], checks = [("span", "start_byte >= 0 AND end_byte >= start_byte")],
    { #[serde(skip)] snapshot_id: Id, evidence_id: Id, subject_node_id: Id, source_fact_id: Id,
      source_digest: Digest, path: String, start_byte: i64, end_byte: i64,
      #[serde(deserialize_with="crate::wire::decode_evidence_role")] #[schemars(with="crate::wire::EvidenceRole")] role: String, text: String }
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
        file::<CatalogSurfaces>(),
        file::<CatalogConfigurations>(),
        file::<CatalogFieldLinks>(),
    ]
}

table!(
    /// Reachable structural type terms, not display strings promoted to type identity.
    CatalogTypes, CatalogTypesRow = "catalog_types", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, term_id], checks = [],
    { #[serde(skip)] snapshot_id: Id, term_id: Id, source_fact_id: Id, #[serde(deserialize_with="crate::wire::decode_type_kind")] #[schemars(with="crate::wire::TypeKind")] kind: String, display: String,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] detail: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] class_module: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] class_key: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] variable: Option<String> }
);
table!(
    CatalogTypeArgs, CatalogTypeArgsRow = "catalog_type_args", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, parent_term_id, role, ordinal], checks = [("ordinal", "ordinal >= 0")],
    { #[serde(skip)] snapshot_id: Id, parent_term_id: Id, #[serde(deserialize_with="crate::wire::decode_type_argument_role")] #[schemars(with="crate::wire::TypeArgumentRole")] role: String, ordinal: i64, child_term_id: Id,
      source_fact_id: Id, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] name: Option<String>, #[serde(deserialize_with="crate::wire::decode_type_parameter_kind_nullable")] #[schemars(with="crate::wire::Nullable<crate::wire::TypeParameterKind>")] parameter_kind: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<bool>")] required: Option<bool> }
);
table!(
    CatalogTypeObservations, CatalogTypeObservationsRow = "catalog_type_observations", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, source_fact_id], checks = [],
    { #[serde(skip)] snapshot_id: Id, source_fact_id: Id, subject_node_id: Id, #[serde(deserialize_with="crate::wire::decode_type_observation_role")] #[schemars(with="crate::wire::TypeObservationRole")] role: String, declared: bool, term_id: Id }
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
    use crate::codebook::{
        Codebook, ParameterKind, RecordKind, TypeArgRole, TypeRole, TypeTermKind,
    };
    Some(match (relation, field) {
        ("catalog_surfaces", "binding_mode") => vec!["class", "static", "property"],
        ("catalog_surfaces", "accessor_role") => {
            vec!["getter", "setter", "deleter", "cached_getter"]
        }
        ("catalog_surfaces", "protocol") => vec!["context_manager", "async_context_manager"],
        ("catalog_surfaces", "registration") => vec!["tool", "resource", "prompt"],
        ("catalog_configurations", "record_kind") => {
            RecordKind::all().iter().map(|k| k.text()).collect()
        }
        ("catalog_configurations", "default_state") => vec![
            "absent",
            "literal_none",
            "literal",
            "source_expression",
            "optional_expression_unavailable",
            "unknown",
            "factory_expression",
        ],
        ("catalog_surfaces", "admission") => vec!["body_preserved", "withheld"],
        ("catalog_field_links", "kind") => {
            vec!["declared_parameter", "exact_storage", "exact_reader"]
        }
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
        ("catalog_evidence", "role") => vec!["declares", "surface", "configuration", "reader"],
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
    CatalogConstructors, CatalogConstructorsRow = "catalog_constructors", row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema], family = Findings,
    key = [snapshot_id, class_node_id, signature_id], checks = [],
    { #[serde(skip)] snapshot_id: Id, class_node_id: Id, signature_id: Id, own: bool, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] ancestry_fact_id: Option<Id> }
);

table!(
    /// Ordered resolved decorator observations; each aspect has independent meaning.
    CatalogSurfaces, CatalogSurfacesRow = "catalog_surfaces",
    row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema],
    family = Findings, key = [snapshot_id, declaration_node_id, ordinal],
    checks = [("ordinal", "ordinal >= 0")],
    { #[serde(skip)] snapshot_id: Id, declaration_node_id: Id, ordinal: i64,
      source_fact_id: Id, expression: String,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] related_node_id: Option<Id>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] resolved_target: Option<String>,
      #[serde(deserialize_with="crate::wire::decode_binding_mode_nullable")] #[schemars(with="crate::wire::Nullable<crate::wire::BindingMode>")] binding_mode: Option<String>, #[serde(deserialize_with="crate::wire::decode_accessor_role_nullable")] #[schemars(with="crate::wire::Nullable<crate::wire::AccessorRole>")] accessor_role: Option<String>, #[serde(deserialize_with="crate::wire::decode_protocol_form_nullable")] #[schemars(with="crate::wire::Nullable<crate::wire::ProtocolForm>")] protocol: Option<String>,
      #[serde(deserialize_with="crate::wire::decode_registration_form_nullable")] #[schemars(with="crate::wire::Nullable<crate::wire::RegistrationForm>")] registration: Option<String>, #[serde(deserialize_with="crate::wire::decode_body_admission")] #[schemars(with="crate::wire::BodyAdmission")] admission: String, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] reason: Option<String> }
);
table!(
    /// Attributed field declaration and original default/factory expression, never evaluated.
    CatalogConfigurations, CatalogConfigurationsRow = "catalog_configurations",
    row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema],
    family = Findings, key = [snapshot_id, field_id, source_fact_id], checks = [("ordinal", "ordinal >= 0")],
    { #[serde(skip)] snapshot_id: Id, field_id: Id, class_node_id: Id, source_fact_id: Id,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] syntax_fact_id: Option<Id>, #[serde(deserialize_with="crate::wire::decode_record_model")] #[schemars(with="crate::wire::RecordModel")] record_kind: String, name: String, ordinal: i64,
      term_id: Id, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] alias: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<bool>")] init: Option<bool>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<bool>")] kw_only: Option<bool>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<bool>")] required: Option<bool>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<bool>")] read_only: Option<bool>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] annotation_text: Option<String>,
      #[serde(deserialize_with="crate::wire::decode_field_default_state")] #[schemars(with="crate::wire::FieldDefaultState")] default_state: String, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] default_text: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] literal_json: Option<String>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] factory_text: Option<String>, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] reason: Option<String> }
);
table!(
    /// Exact constructor-formal associations are distinct from declaration-only associations.
    CatalogFieldLinks, CatalogFieldLinksRow = "catalog_field_links",
    row_derives = [serde::Serialize, serde::Deserialize, schemars::JsonSchema],
    family = Findings, key = [snapshot_id, link_id], checks = [("ordinal", "ordinal >= 0"),("reader", "(kind = 'exact_reader' AND reader_node_id IS NOT NULL) OR (kind <> 'exact_reader' AND reader_node_id IS NULL)")],
    { #[serde(skip)] snapshot_id: Id, link_id: Id, field_id: Id, class_node_id: Id,
      signature_id: Id, ordinal: i64, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] formal_node_id: Option<Id>,
      #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<Id>")] reader_node_id: Option<Id>, source_fact_id: Id, #[serde(deserialize_with="crate::wire::decode_field_link_kind")] #[schemars(with="crate::wire::FieldLinkKind")] kind: String, #[serde(deserialize_with="crate::wire::required_nullable")] #[schemars(with="crate::wire::Nullable<String>")] reason: Option<String> }
);
