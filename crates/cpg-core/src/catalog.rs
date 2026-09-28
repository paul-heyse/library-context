//! Mandatory catalog assembly from canonical evidence; optional behavior enriches the same rows.
use crate::behavior::{BehaviorRows, WINDOW_BYTES};
use crate::{CoreError, sql};
use cpg_schema::behavior::{
    self as b, OperationDocumentsRow, OperationFacetStatusRow, OperationFacetsRow,
    OperationSourceRow, OperationsRow,
};
use cpg_schema::catalog::*;
use cpg_schema::catalog::{self, CompileProfile};
use cpg_schema::codebook::{BehaviorKind, DeclarationKind, EmbeddingView, OperationFacet, Verdict};
use cpg_schema::findings::PublicPathsRow;
use cpg_schema::metrics::Stages;
use cpg_schema::query::QueryRow;
use cpg_schema::{Codebook, Id, IdHasher, Table};
use cpg_schema::{derived, tables as raw};
use datafusion::prelude::SessionContext;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct CompileInputs {
    pub public_roots: Vec<String>,
    pub profile: CompileProfile,
    pub embedder: Option<Arc<dyn crate::embed::Embedder>>,
    pub embedding_cache: Option<crate::postgres::Store>,
}
impl CompileInputs {
    pub fn from_analysis(analysis: Option<&crate::analyze::Analysis>) -> Self {
        Self {
            public_roots: analysis
                .map(|a| a.config.subsystem.public_roots.clone())
                .unwrap_or_default(),
            profile: if analysis.is_some() {
                CompileProfile::Behavioral
            } else {
                CompileProfile::Catalog
            },
            embedder: analysis.and_then(|a| a.embedder.clone()),
            embedding_cache: analysis.and_then(|a| a.embedding_cache.clone()),
        }
    }
    pub fn digest(&self, analysis: Option<&crate::analyze::Analysis>) -> cpg_schema::id::Digest {
        let mut h = IdHasher::new("catalog-compile-inputs");
        h.str(self.profile.name())
            .strs(self.public_roots.iter().map(String::as_str));
        if let Some(a) = analysis {
            h.digest_field(crate::analyze::variant_config_digest(
                a.config.digest(),
                &a.techniques,
            ));
        }
        h.finish_digest()
    }
}

cpg_schema::query_row! { struct TextRow { module_node_id: Id, text: Option<String> } }
cpg_schema::query_row! { struct QualifiedRow { node_id: Id, qualified_name: String } }
cpg_schema::relations! {
    inventory relations;
    module_texts = "catalog_module_texts", deps = ["source_files"],
        sql = "SELECT DISTINCT module_node_id, text FROM source_files WHERE array_has($ids, module_node_id) ORDER BY module_node_id".into();
    qualified_names = "catalog_qualified_names", deps = ["declarations"],
        sql = "SELECT node_id, qualified_name FROM declarations WHERE array_has($ids, node_id) ORDER BY node_id".into();
}

#[allow(
    clippy::too_many_arguments,
    reason = "catalog inputs and explicit optional enrichment"
)]
pub async fn populate(
    ctx: &SessionContext,
    embeddings: &mut crate::embed::Session,
    snapshot_id: Id,
    embedder: Option<&dyn crate::embed::Embedder>,
    contracts: &Contracts,
    out: &mut BehaviorRows,
    stages: &mut Stages,
) -> Result<(), CoreError> {
    let sources: Vec<OperationSourceRow> =
        sql::fetch(ctx, &b::operation_sources(), sql::Params::new()).await?;
    let callables: Vec<_> = sources
        .iter()
        .filter(|s| s.kind != DeclarationKind::Class)
        .map(|s| s.node_id)
        .collect();
    let parameters_of = crate::analyze::seed_parameters(ctx, &callables).await?;
    if out.operations.is_empty() {
        out.operations = sources
            .iter()
            .map(|s| OperationsRow {
                snapshot_id,
                node_id: s.node_id,
                access_path: s.access_path.clone(),
                kind: s.kind,
                is_method: s.is_method,
                qualified_name: s.qualified_name.clone(),
                module: s.module.clone(),
                docstring_summary: s.docstring.as_deref().and_then(b::docstring_summary),
                behavior_status: Verdict::NotAnalyzed,
                boundary_reason: Some(cpg_schema::codebook::BoundaryReason::NotRequested),
                status_reason: Some("behavioral analysis not requested".into()),
            })
            .collect();
    }
    let partners: BTreeSet<_> = out
        .behaviors
        .iter()
        .filter_map(|r| r.callee_node_id)
        .collect();
    let qualified: BTreeMap<_, _> = sql::fetch::<QualifiedRow>(
        ctx,
        &qualified_names(),
        sql::Params::new().ids("ids", partners.iter().copied()),
    )
    .await?
    .into_iter()
    .map(|r| (r.node_id, r.qualified_name))
    .collect();
    let paths: BTreeMap<_, _> = sources
        .iter()
        .map(|s| (s.node_id, s.access_path.clone()))
        .collect();
    let name_of = |id| paths.get(&id).or_else(|| qualified.get(&id)).cloned();
    // Facets, each with the best verdict of what it comes from (increment 3's deep review, F3).
    let mut facets: BTreeMap<(Id, OperationFacet, String), Verdict> = BTreeMap::new();
    let mut put = |node: Id, facet: OperationFacet, value: String, verdict: Verdict| {
        let e = facets.entry((node, facet, value)).or_insert(verdict);
        *e = (*e).min(verdict);
    };
    for s in &sources {
        let kind = match (s.kind, s.is_method) {
            (DeclarationKind::Class, _) => "class",
            (_, true) => "method",
            _ => "function",
        };
        put(
            s.node_id,
            OperationFacet::Kind,
            kind.to_owned(),
            Verdict::Established,
        );
        put(
            s.node_id,
            OperationFacet::Module,
            s.module.clone(),
            Verdict::Established,
        );
        if s.kind == DeclarationKind::AsyncFunction {
            put(
                s.node_id,
                OperationFacet::Async,
                "true".to_owned(),
                Verdict::Established,
            );
        }
        if s.kind == DeclarationKind::Class {
            for d in &s.decorators {
                put(
                    s.node_id,
                    OperationFacet::Decorator,
                    d.clone(),
                    Verdict::Established,
                );
            }
        }
    }
    let attributes = crate::analyze::collect_attributes(ctx, &callables).await?;
    let mut declared: BTreeMap<Id, Vec<(OperationFacet, String)>> = BTreeMap::new();
    for (node, attribute) in attributes {
        let (facet, value) = attribute
            .facet()
            .ok_or_else(|| CoreError::Analysis("invalid declared attribute".into()))?;
        put(node, facet, value.to_owned(), Verdict::Established);
        declared
            .entry(node)
            .or_default()
            .push((facet, value.to_owned()));
    }
    let mut constructor = BTreeMap::new();
    for signature in &contracts.signatures {
        let nodes: Vec<_> = std::iter::once(signature.callable_node_id)
            .chain(
                contracts
                    .constructors
                    .iter()
                    .filter(|c| c.signature_id == signature.signature_id)
                    .map(|c| c.class_node_id),
            )
            .collect();
        for node in nodes {
            if !sources.iter().any(|s| s.node_id == node) {
                continue;
            }
            if node != signature.callable_node_id {
                constructor.insert(node, signature.callable_node_id);
            }
            for p in contracts
                .parameters
                .iter()
                .filter(|p| p.signature_id == signature.signature_id)
            {
                if let Some(name) = &p.name {
                    put(
                        node,
                        OperationFacet::Parameter,
                        name.clone(),
                        Verdict::Established,
                    );
                    if let Some(annotation) = p
                        .annotation_text
                        .as_ref()
                        .or(p.provider_annotation.as_ref())
                    {
                        put(
                            node,
                            OperationFacet::ParameterType,
                            format!("{name}: {annotation}"),
                            Verdict::Established,
                        );
                    }
                }
            }
        }
    }
    for r in &out.behaviors {
        if r.kind == BehaviorKind::ReadsSetting
            && let Some(setting) = &r.target_name
        {
            put(
                r.operation_node_id,
                OperationFacet::ReadsSetting,
                setting.clone(),
                r.verdict,
            );
            continue;
        }
        let facet = match r.kind {
            BehaviorKind::Delegates => OperationFacet::DelegatesTo,
            BehaviorKind::Forwards => OperationFacet::ForwardsTo,
            BehaviorKind::HandsOffTo => OperationFacet::HandsOffTo,
            BehaviorKind::TakesFrom => OperationFacet::TakesFrom,
            _ => continue,
        };
        if let Some(name) = r.callee_node_id.and_then(name_of) {
            put(r.operation_node_id, facet, name, r.verdict);
        }
    }
    out.facets = facets
        .into_iter()
        .map(|((node_id, facet, value), verdict)| OperationFacetsRow {
            snapshot_id,
            node_id,
            facet,
            value,
            verdict,
        })
        .collect();

    // Whether each operation's rows for each facet are complete (F3, F4): the served authority
    // for `find_operations`' `complete` and its `unknown` list.
    let status_of: BTreeMap<Id, (Verdict, Option<String>)> = out
        .operations
        .iter()
        .map(|o| (o.node_id, (o.behavior_status, o.status_reason.clone())))
        .collect();
    for s in &sources {
        let class = s.kind == DeclarationKind::Class;
        let (scan, scan_reason) = status_of
            .get(&s.node_id)
            .cloned()
            .unwrap_or((Verdict::NotAnalyzed, None));
        for &facet in <OperationFacet as Codebook>::all() {
            let (verdict, reason) = match facet {
                OperationFacet::Kind
                | OperationFacet::Module
                | OperationFacet::Async
                | OperationFacet::Decorator => (Verdict::Established, None),
                OperationFacet::Parameter | OperationFacet::ParameterType
                    if class && !constructor.contains_key(&s.node_id) =>
                {
                    (
                        Verdict::NotAnalyzed,
                        Some("constructor contract unavailable".to_owned()),
                    )
                }
                OperationFacet::Parameter | OperationFacet::ParameterType => {
                    let signatures: Vec<_> = contracts
                        .signatures
                        .iter()
                        .filter(|sig| {
                            sig.callable_node_id == s.node_id
                                || contracts.constructors.iter().any(|c| {
                                    c.class_node_id == s.node_id
                                        && c.signature_id == sig.signature_id
                                })
                        })
                        .collect();
                    if signatures.is_empty()
                        || signatures
                            .iter()
                            .any(|sig| !matches!(sig.form.as_str(), "list" | "source_list"))
                    {
                        (
                            Verdict::Unknown,
                            Some("parameter list unresolved or symbolic".into()),
                        )
                    } else {
                        (Verdict::Established, None)
                    }
                }
                OperationFacet::Returns if class => (
                    Verdict::NotAnalyzed,
                    Some("a class: its constructor returns the instance".to_owned()),
                ),
                OperationFacet::Returns => (Verdict::Established, None),
                OperationFacet::Raises => (
                    Verdict::Unknown,
                    Some("only a typed raise directly in the body is a row".to_owned()),
                ),
                OperationFacet::DelegatesTo | OperationFacet::ForwardsTo => {
                    (scan, scan_reason.clone())
                }
                OperationFacet::HandsOffTo | OperationFacet::TakesFrom => (
                    Verdict::Unknown,
                    Some("official usage is read in two handoff shapes only".to_owned()),
                ),
                OperationFacet::ReadsSetting if class => (
                    Verdict::NotAnalyzed,
                    Some("a class: its reads are its constructor's".to_owned()),
                ),
                OperationFacet::ReadsSetting => (
                    Verdict::Unknown,
                    Some("reads in its own body only: reads in callees are Stage 3's".to_owned()),
                ),
            };
            let (verdict, reason) = if scan_reason.as_deref()
                == Some("behavioral analysis not requested")
                && matches!(
                    facet,
                    OperationFacet::Raises
                        | OperationFacet::DelegatesTo
                        | OperationFacet::ForwardsTo
                        | OperationFacet::HandsOffTo
                        | OperationFacet::TakesFrom
                        | OperationFacet::ReadsSetting
                ) {
                (
                    Verdict::NotAnalyzed,
                    Some("behavioral analysis not requested".into()),
                )
            } else {
                (verdict, reason)
            };
            out.facet_status.push(OperationFacetStatusRow {
                snapshot_id,
                node_id: s.node_id,
                facet,
                verdict,
                reason,
            });
        }
    }
    stages.mark("catalog: operations and facets");

    // Documents: the signature-and-docstring view and the source-body view of each callable.
    let modules: BTreeSet<Id> = sources.iter().map(|s| s.module_node_id).collect();
    let texts: BTreeMap<Id, String> = sql::fetch::<TextRow>(
        ctx,
        &module_texts(),
        sql::Params::new().ids("ids", modules.iter().copied()),
    )
    .await?
    .into_iter()
    .filter_map(|r| r.text.map(|t| (r.module_node_id, t)))
    .collect();
    for s in sources.iter() {
        let names: Vec<&str> = parameters_of
            .get(&s.node_id)
            .map(|ps| ps.iter().map(|p| p.name.as_str()).collect())
            .unwrap_or_default();
        let signature = lctx_analytics::neighbours::api_text(
            &s.access_path,
            Some(&names.join(", ")),
            s.docstring.as_deref(),
        );
        let body = texts
            .get(&s.module_node_id)
            .and_then(|t| t.get(s.start_byte as usize..s.end_byte as usize))
            .unwrap_or_default()
            .to_owned();
        for (view, text) in [
            (EmbeddingView::SignatureDoc, signature),
            (EmbeddingView::SourceBody, body),
        ] {
            for (chunk, window) in lctx_analytics::neighbours::windows(&text, WINDOW_BYTES)
                .into_iter()
                .enumerate()
            {
                out.documents.push(OperationDocumentsRow {
                    snapshot_id,
                    node_id: s.node_id,
                    embedding_view: view,
                    chunk: chunk as i64,
                    text: window.to_owned(),
                    spec_hash: None,
                    input_hash: None,
                });
            }
        }
    }
    if let Some(embedder) = embedder {
        let spec = embedder.spec();
        let spec_hash = spec.hash();
        let texts: Vec<String> = out.documents.iter().map(|d| d.text.clone()).collect();
        embeddings
            .texts(embedder, &texts, crate::embed::Usage::Operation)
            .await?;
        for d in &mut out.documents {
            d.spec_hash = Some(spec_hash);
            d.input_hash = Some(crate::embed::input_hash(&spec.document_text(&d.text)));
        }
        stages.mark("catalog: embed operation documents");
    }
    Ok(())
}

#[derive(Default)]
pub struct Contracts {
    pub members: Vec<CatalogMembersRow>,
    pub constructors: Vec<CatalogConstructorsRow>,
    pub bindings: Vec<CatalogBindingsRow>,
    pub signatures: Vec<CatalogSignaturesRow>,
    pub parameters: Vec<CatalogParametersRow>,
    pub evidence: Vec<CatalogEvidenceRow>,
    pub types: Vec<CatalogTypesRow>,
    pub type_args: Vec<CatalogTypeArgsRow>,
    pub type_observations: Vec<CatalogTypeObservationsRow>,
}

async fn rows<T: Table>(ctx: &SessionContext) -> Result<Vec<T::Row>, CoreError>
where
    T::Row: QueryRow,
{
    let relation = cpg_schema::query::Relation {
        name: T::NAME,
        deps: &[],
        sql: format!("SELECT * FROM {}", T::NAME),
    };
    sql::fetch(ctx, &relation, sql::Params::new()).await
}

fn member_id(export: Id, path: &str) -> Id {
    IdHasher::new("public-member")
        .id(export)
        .str(path)
        .finish_id()
}

/// Exact literal decoding never evaluates Python. Unrecognized spelling remains an expression.
fn default_value(text: Option<&str>, required: Option<bool>) -> (String, Option<String>) {
    let Some(text) = text else {
        return (
            match required {
                Some(true) => "absent",
                Some(false) => "optional_expression_unavailable",
                None => "unknown",
            }
            .into(),
            None,
        );
    };
    let value = match text.trim() {
        "None" => Some(serde_json::Value::Null),
        "True" => Some(serde_json::Value::Bool(true)),
        "False" => Some(serde_json::Value::Bool(false)),
        s => serde_json::from_str::<serde_json::Value>(s)
            .ok()
            .filter(|v| v.is_string() || v.is_number()),
    };
    match value {
        Some(v) => (
            if v.is_null() {
                "literal_none"
            } else {
                "literal"
            }
            .into(),
            Some(v.to_string()),
        ),
        None => ("source_expression".into(), None),
    }
}

/// Assemble attributed observations. Selection views are consulted only to label the legacy
/// winner; raw declarations/public-name observations retain all source alternatives first.
pub async fn contracts(
    ctx: &SessionContext,
    snapshot_id: Id,
    roots: &[String],
    public: &[PublicPathsRow],
) -> Result<Contracts, CoreError> {
    let declarations = rows::<raw::Declarations>(ctx).await?;
    let source = rows::<raw::SourceFiles>(ctx).await?;
    let names = rows::<raw::PublicNames>(ctx).await?;
    let candidates: Vec<cpg_schema::public::ExportCandidateRow> = sql::fetch(
        ctx,
        &cpg_schema::public::export_candidates(),
        sql::Params::new(),
    )
    .await?;
    let signatures = rows::<derived::Signatures>(ctx).await?;
    let parameters = rows::<derived::Parameters>(ctx).await?;
    let syntax = rows::<raw::ParameterSyntax>(ctx).await?;
    let semantics = rows::<raw::ParameterSemantics>(ctx).await?;
    let docs = rows::<raw::ParameterDocs>(ctx).await?;
    let functions = rows::<raw::PysaFunctions>(ctx).await?;
    let synthetic = rows::<derived::SyntheticCallables>(ctx).await?;
    let classes = rows::<raw::PysaClasses>(ctx).await?;
    let class_map = rows::<derived::ProviderClassMap>(ctx).await?;
    let ancestry = rows::<raw::ClassAncestry>(ctx).await?;
    let ancestry_targets = rows::<derived::AncestryTargets>(ctx).await?;
    let scopes = rows::<raw::Scopes>(ctx).await?;
    let lexical_bindings = rows::<raw::Bindings>(ctx).await?;
    let by_decl: BTreeMap<_, _> = declarations.iter().map(|r| (r.node_id, r)).collect();
    let by_file: BTreeMap<_, _> = source.iter().map(|r| (r.module_node_id, r)).collect();
    let by_syntax: BTreeMap<_, _> = syntax.iter().map(|r| (r.fact_id, r)).collect();
    let by_semantics: BTreeMap<_, _> = semantics.iter().map(|r| (r.fact_id, r)).collect();
    type AncestorEntry = (i64, Option<Id>, Option<Id>);
    let mut chains: BTreeMap<Id, Vec<AncestorEntry>> = BTreeMap::new();
    for class in declarations
        .iter()
        .filter(|d| d.kind == DeclarationKind::Class)
    {
        chains.insert(class.node_id, vec![(-1, Some(class.node_id), None)]);
    }
    for target in &ancestry_targets {
        let Some(class) = target.class_node_id else {
            continue;
        };
        let Some(a) = ancestry.iter().find(|a| {
            a.fact_id == target.ancestry_fact_id
                && a.relation == cpg_schema::codebook::AncestryRelation::Mro
        }) else {
            continue;
        };
        chains.entry(class).or_default().push((
            a.ordinal.unwrap_or(0),
            target.ancestor_node_id,
            Some(a.fact_id),
        ));
    }
    for chain in chains.values_mut() {
        chain.sort_by_key(|a| a.0);
    }
    let rebound = |class: Id| -> BTreeSet<&str> {
        let scope_ids: BTreeSet<_> = scopes
            .iter()
            .filter(|s| {
                s.owner_node_id == class && s.kind == cpg_schema::codebook::LexicalScopeKind::Class
            })
            .map(|s| s.node_id)
            .collect();
        lexical_bindings
            .iter()
            .filter(|b| {
                scope_ids.contains(&b.scope_id)
                    && !matches!(
                        b.kind,
                        cpg_schema::codebook::BindingKind::FunctionDef
                            | cpg_schema::codebook::BindingKind::ClassDef
                            | cpg_schema::codebook::BindingKind::AnnotationOnly
                    )
            })
            .map(|b| b.name.as_str())
            .collect()
    };
    let mut out = Contracts::default();
    let mut members: BTreeMap<String, CatalogMembersRow> = BTreeMap::new();
    let mut selected = BTreeSet::new();
    for path in public {
        let mid = member_id(path.export_node_id, &path.access_path);
        members
            .entry(path.access_path.clone())
            .or_insert_with(|| CatalogMembersRow {
                snapshot_id,
                member_id: mid,
                access_path: path.access_path.clone(),
                owner_path: path
                    .access_path
                    .rsplit_once('.')
                    .map_or("", |(p, _)| p)
                    .into(),
                operation_node_id: Some(path.node_id),
                kind: path.kind.text().into(),
                resolution: "source_known_effective_unresolved".into(),
                brief_status: "not_requested".into(),
                brief_reason: None,
            });
        let declaration = by_decl
            .get(&path.node_id)
            .ok_or_else(|| CoreError::Analysis("public declaration missing".into()))?;
        // Expand the source declaration group before its preferred representative was chosen.
        for candidate in declarations.iter().filter(|d| {
            d.module_node_id == declaration.module_node_id
                && d.qualified_name == declaration.qualified_name
        }) {
            selected.insert(candidate.node_id);
            let role = if candidate.is_overload {
                "overload"
            } else if candidate.node_id == path.node_id {
                "selected_source"
            } else {
                "shadowed_source"
            };
            out.bindings.push(CatalogBindingsRow {
                snapshot_id,
                member_id: mid,
                binding_id: IdHasher::new("public-binding")
                    .id(mid)
                    .id(candidate.fact_id)
                    .str(role)
                    .finish_id(),
                declaration_node_id: Some(candidate.node_id),
                source_fact_id: candidate.fact_id,
                role: role.into(),
                own: path.own,
                defining_path: Some(candidate.qualified_name.clone()),
            });
        }
    }
    // Public observations with no selected operation still exist. .py/.pyi observations keep
    // their provenance; provider alternatives are never collapsed into a runtime union.
    for name in names
        .iter()
        .filter(|n| catalog::under_roots(&n.access_path, roots))
    {
        let file = by_file
            .get(&name.access_module_node_id)
            .ok_or_else(|| CoreError::Analysis("public access module missing".into()))?;
        let export = IdHasher::new("export")
            .id(file.release_id)
            .str(&name.access_path)
            .finish_id();
        let member = members
            .entry(name.access_path.clone())
            .or_insert_with(|| CatalogMembersRow {
                snapshot_id,
                member_id: member_id(export, &name.access_path),
                access_path: name.access_path.clone(),
                owner_path: name.access_module.clone(),
                operation_node_id: None,
                kind: name
                    .origin_symbol_kind
                    .map_or("unknown", |k| k.text())
                    .into(),
                resolution: "unresolved".into(),
                brief_status: "not_requested".into(),
                brief_reason: None,
            });
        out.bindings.push(CatalogBindingsRow {
            snapshot_id,
            member_id: member.member_id,
            binding_id: IdHasher::new("public-observation")
                .id(member.member_id)
                .id(name.fact_id)
                .finish_id(),
            declaration_node_id: None,
            source_fact_id: name.fact_id,
            role: "provider_public_observation".into(),
            own: true,
            defining_path: name.origin_path.clone(),
        });
        for candidate in candidates
            .iter()
            .filter(|c| c.public_fact_id == name.fact_id)
        {
            let Some(d) = candidate
                .declaration_node_id
                .and_then(|id| by_decl.get(&id).copied())
            else {
                continue;
            };
            selected.insert(d.node_id);
            let role = if d.is_overload {
                "overload"
            } else if by_file[&d.module_node_id].is_stub {
                "stub_source"
            } else if member.operation_node_id == Some(d.node_id) {
                "selected_source"
            } else {
                "source_alternative"
            };
            out.bindings.push(CatalogBindingsRow {
                snapshot_id,
                member_id: member.member_id,
                binding_id: IdHasher::new("public-binding")
                    .id(member.member_id)
                    .id(d.fact_id)
                    .str(role)
                    .finish_id(),
                declaration_node_id: Some(d.node_id),
                source_fact_id: d.fact_id,
                role: role.into(),
                own: true,
                defining_path: Some(d.qualified_name.clone()),
            });
        }
    }
    // Enumerate members of every attributed class candidate before consulting the legacy
    // preferred path. A .pyi-only member remains inspectable as a source observation.
    let name_by_fact: BTreeMap<_, _> = names.iter().map(|n| (n.fact_id, n)).collect();
    let mut class_candidates = Vec::new();
    for candidate in &candidates {
        let Some(class) = candidate
            .declaration_node_id
            .and_then(|id| by_decl.get(&id).copied())
        else {
            continue;
        };
        if class.kind != DeclarationKind::Class {
            continue;
        }
        let name = name_by_fact[&candidate.public_fact_id];
        if !catalog::under_roots(&name.access_path, roots)
            && !roots
                .iter()
                .any(|root| root.starts_with(&(name.access_path.clone() + ".")))
        {
            continue;
        }
        let release = by_file[&name.access_module_node_id].release_id;
        let export = IdHasher::new("export")
            .id(release)
            .str(&name.access_path)
            .finish_id();
        class_candidates.push((name.access_path.clone(), class.node_id, export));
    }
    let mut visited_classes = BTreeSet::new();
    while let Some((path, class, export)) = class_candidates.pop() {
        if !visited_classes.insert((path.clone(), class)) {
            continue;
        }
        let mut hidden = BTreeSet::new();
        for (_, ancestor, _) in &chains[&class] {
            let Some(ancestor) = ancestor.filter(|a| by_decl.contains_key(a)) else {
                break;
            };
            hidden.extend(rebound(ancestor));
            let mut declared = BTreeSet::new();
            for d in declarations.iter().filter(|d| {
                d.parent_node_id == Some(ancestor)
                    && (!d.name.starts_with('_')
                        || cpg_schema::public::DUNDER_MEMBERS.contains(&d.name.as_str()))
            }) {
                if hidden.contains(d.name.as_str()) {
                    continue;
                }
                declared.insert(d.name.as_str());
                let access_path = format!("{path}.{}", d.name);
                if d.kind == DeclarationKind::Class {
                    class_candidates.push((access_path.clone(), d.node_id, export));
                }
                if !catalog::under_roots(&access_path, roots) {
                    continue;
                }
                let member =
                    members
                        .entry(access_path.clone())
                        .or_insert_with(|| CatalogMembersRow {
                            snapshot_id,
                            member_id: member_id(export, &access_path),
                            access_path,
                            owner_path: path.clone(),
                            operation_node_id: None,
                            kind: d.kind.text().into(),
                            resolution: "source_known_effective_unresolved".into(),
                            brief_status: "not_requested".into(),
                            brief_reason: None,
                        });
                selected.insert(d.node_id);
                let role = if d.is_overload {
                    "overload"
                } else if by_file[&d.module_node_id].is_stub {
                    "stub_source"
                } else if member.operation_node_id == Some(d.node_id) {
                    "selected_source"
                } else {
                    "source_alternative"
                };
                out.bindings.push(CatalogBindingsRow {
                    snapshot_id,
                    member_id: member.member_id,
                    binding_id: IdHasher::new("public-binding")
                        .id(member.member_id)
                        .id(d.fact_id)
                        .str(role)
                        .finish_id(),
                    declaration_node_id: Some(d.node_id),
                    source_fact_id: d.fact_id,
                    role: role.into(),
                    own: ancestor == class,
                    defining_path: Some(d.qualified_name.clone()),
                });
            }
            hidden.extend(declared);
        }
    }
    // Select the first declared/provider constructor along each class's reported MRO.
    // Ancestor definitions remain supporting evidence even outside the requested roots.
    let selected_classes: BTreeSet<_> = selected
        .iter()
        .filter(|id| {
            by_decl
                .get(id)
                .is_some_and(|d| d.kind == DeclarationKind::Class)
        })
        .copied()
        .collect();
    let provider_constructor_classes: BTreeSet<_> = functions
        .iter()
        .filter(|f| f.name == "__init__")
        .filter_map(|f| {
            classes.iter().find(|c| {
                c.module_node_id == f.module_node_id
                    && by_file.get(&c.module_node_id).is_some_and(|source| {
                        f.defining_class_module.as_deref()
                            == Some(format!("@{}", source.path).as_str())
                    })
                    && Some(&c.class_key) == f.defining_class_key.as_ref()
            })
        })
        .filter_map(|c| {
            class_map
                .iter()
                .find(|m| m.module_node_id == c.module_node_id && m.class_key == c.class_key)
                .and_then(|m| m.node_id)
        })
        .collect();
    let mut constructor_owners = BTreeMap::new();
    for class in &selected_classes {
        for (_, ancestor, fact) in &chains[class] {
            let Some(ancestor) = ancestor.filter(|a| by_decl.contains_key(a)) else {
                break;
            };
            if rebound(ancestor).contains("__init__") {
                break;
            }
            if provider_constructor_classes.contains(&ancestor)
                || declarations
                    .iter()
                    .any(|d| d.parent_node_id == Some(ancestor) && d.name == "__init__")
            {
                constructor_owners.insert(*class, (ancestor, *fact));
                selected.insert(ancestor);
                break;
            }
        }
    }
    // Constructor contracts belong to their class even when __init__ has no public path.
    let constructors: Vec<_> = declarations
        .iter()
        .filter(|d| {
            d.name == "__init__" && d.parent_node_id.is_some_and(|id| selected.contains(&id))
        })
        .map(|d| d.node_id)
        .collect();
    selected.extend(constructors);
    let source_signatures: Vec<_> = signatures
        .iter()
        .filter(|s| {
            selected.contains(&s.callable_node_id) || selected.contains(&s.signature_node_id)
        })
        .collect();
    for s in source_signatures {
        let d = by_decl[&s.signature_node_id];
        selected.insert(d.node_id);
        let signature_id = s.signature_node_id;
        out.signatures.push(CatalogSignaturesRow {
            snapshot_id,
            signature_id,
            callable_node_id: s.callable_node_id,
            declaration_node_id: Some(d.node_id),
            constructor_class_id: (d.name == "__init__").then_some(d.parent_node_id).flatten(),
            module_node_id: s.module_node_id,
            function_key: s.function_key.clone(),
            signature_index: s.signature_index,
            role: if d.is_overload { "overload" } else { "source" }.into(),
            form: s.form.map_or("source_list", |f| f.text()).into(),
            source_fact_id: s.declaration_fact_id,
            reason: s.reason.map(|r| r.text().into()),
            return_annotation: None,
            docstring: d.docstring.clone(),
        });
        for p in parameters
            .iter()
            .filter(|p| p.signature_node_id == signature_id)
        {
            let syn = p.syntax_fact_id.and_then(|id| by_syntax.get(&id).copied());
            let sem = p
                .semantics_fact_id
                .and_then(|id| by_semantics.get(&id).copied());
            let name = syn
                .map(|p| p.name.clone())
                .or_else(|| sem.and_then(|p| p.name.clone()));
            let required = syn
                .map(|p| {
                    p.default_text.is_none()
                        && !matches!(
                            p.kind,
                            cpg_schema::codebook::ParameterKind::VarPositional
                                | cpg_schema::codebook::ParameterKind::VarKeyword
                        )
                })
                .or_else(|| sem.and_then(|p| p.required));
            let default = syn.and_then(|p| p.default_text.clone());
            let (mut default_state, literal_json) = default_value(default.as_deref(), required);
            if syn.is_some() && default.is_none() {
                default_state = "absent".into();
            }
            out.parameters.push(CatalogParametersRow {
                snapshot_id,
                signature_id,
                ordinal: p.ordinal,
                formal_node_id: syn.map(|p| p.node_id),
                name: name.clone(),
                kind: syn
                    .map(|p| p.kind)
                    .or_else(|| sem.and_then(|p| p.kind))
                    .map(|k| k.text().into()),
                required,
                syntax_fact_id: p.syntax_fact_id,
                semantics_fact_id: p.semantics_fact_id,
                provider_name: sem.and_then(|p| p.name.clone()),
                provider_kind: sem.and_then(|p| p.kind).map(|k| k.text().into()),
                annotation_text: syn.and_then(|p| p.annotation_text.clone()),
                provider_annotation: sem.and_then(|p| p.annotation.clone()),
                default_state,
                default_text: default,
                literal_json,
                documentation: docs
                    .iter()
                    .find(|p| p.function_node_id == d.node_id && Some(&p.name) == name.as_ref())
                    .map(|p| p.text.clone()),
                reason: p.reason.map(|r| r.text().into()),
            });
        }
    }
    for synthetic in synthetic {
        let Some(f) = functions.iter().find(|f| {
            f.module_node_id == synthetic.module_node_id && f.function_key == synthetic.function_key
        }) else {
            continue;
        };
        if f.name != "__init__" {
            continue;
        }
        let class = classes
            .iter()
            .find(|c| {
                c.module_node_id == f.module_node_id
                    && by_file.get(&c.module_node_id).is_some_and(|source| {
                        f.defining_class_module.as_deref()
                            == Some(format!("@{}", source.path).as_str())
                    })
                    && Some(&c.class_key) == f.defining_class_key.as_ref()
            })
            .and_then(|c| {
                class_map
                    .iter()
                    .find(|m| m.module_node_id == c.module_node_id && m.class_key == c.class_key)
            })
            .and_then(|c| c.node_id);
        let Some(class) = class.filter(|id| selected.contains(id)) else {
            continue;
        };
        for index in 0..f.signature_count {
            let signature_id = IdHasher::new("provider-signature")
                .id(synthetic.node_id)
                .i64(index)
                .finish_id();
            let params: Vec<_> = semantics
                .iter()
                .filter(|p| {
                    p.module_node_id == f.module_node_id
                        && p.function_key == f.function_key
                        && p.signature_index == index
                })
                .collect();
            out.signatures.push(CatalogSignaturesRow {
                snapshot_id,
                signature_id,
                callable_node_id: synthetic.node_id,
                declaration_node_id: None,
                constructor_class_id: Some(class),
                module_node_id: f.module_node_id,
                function_key: Some(f.function_key.clone()),
                signature_index: Some(index),
                role: "provider_constructor".into(),
                form: params.first().map_or("list", |p| p.form.text()).into(),
                source_fact_id: f.fact_id,
                reason: None,
                return_annotation: None,
                docstring: None,
            });
            for p in params {
                let Some(ordinal) = p.ordinal else { continue };
                let (default_state, literal_json) = default_value(None, p.required);
                out.parameters.push(CatalogParametersRow {
                    snapshot_id,
                    signature_id,
                    ordinal,
                    formal_node_id: None,
                    name: p.name.clone(),
                    kind: p.kind.map(|k| k.text().into()),
                    required: p.required,
                    syntax_fact_id: None,
                    semantics_fact_id: Some(p.fact_id),
                    provider_name: p.name.clone(),
                    provider_kind: p.kind.map(|k| k.text().into()),
                    annotation_text: None,
                    provider_annotation: p.annotation.clone(),
                    default_state,
                    default_text: None,
                    literal_json,
                    documentation: None,
                    reason: None,
                });
            }
        }
    }
    let mut constructor_links = BTreeMap::new();
    for sig in &out.signatures {
        if let Some(owner) = sig.constructor_class_id {
            for (class, (ancestor, fact)) in &constructor_owners {
                if owner == *ancestor {
                    constructor_links.insert((*class, sig.signature_id), (*class == owner, *fact));
                }
            }
        }
    }
    out.constructors = constructor_links
        .into_iter()
        .map(
            |((class_node_id, signature_id), (own, ancestry_fact_id))| CatalogConstructorsRow {
                snapshot_id,
                class_node_id,
                signature_id,
                own,
                ancestry_fact_id,
            },
        )
        .collect();
    let mut typed_subjects = selected.clone();
    typed_subjects.extend(out.parameters.iter().filter_map(|p| p.formal_node_id));
    let observations = rows::<raw::TypeObservations>(ctx).await?;
    let terms = rows::<raw::TypeTerms>(ctx).await?;
    let args = rows::<raw::TypeTermArgs>(ctx).await?;
    let mut by_term: BTreeMap<Id, &raw::TypeTermsRow> = BTreeMap::new();
    for term in &terms {
        if let Some(prior) = by_term.get(&term.node_id) {
            let mut same = (*prior).clone();
            same.fact_id = term.fact_id;
            if same != *term {
                return Err(CoreError::Invalid(vec![crate::validate::Violation {
                    rule: "unique:type_terms".into(),
                    rows: 1,
                    sample: "one term identity has conflicting source observations".into(),
                }]));
            }
            if prior.fact_id < term.fact_id {
                continue;
            }
        }
        by_term.insert(term.node_id, term);
    }
    let mut reachable = BTreeSet::new();
    for o in observations
        .iter()
        .filter(|o| typed_subjects.contains(&o.subject_node_id))
    {
        reachable.insert(o.term_node_id);
        out.type_observations.push(CatalogTypeObservationsRow {
            snapshot_id,
            source_fact_id: o.fact_id,
            subject_node_id: o.subject_node_id,
            role: o.role.text().into(),
            declared: o.declared,
            term_id: o.term_node_id,
        });
    }
    loop {
        let before = reachable.len();
        let children: Vec<_> = args
            .iter()
            .filter(|a| reachable.contains(&a.parent_node_id))
            .map(|a| a.child_node_id)
            .collect();
        reachable.extend(children);
        if reachable.len() == before {
            break;
        }
    }
    for term in &reachable {
        let t = by_term
            .get(term)
            .ok_or_else(|| CoreError::Analysis("catalog type closure missing term".into()))?;
        out.types.push(CatalogTypesRow {
            snapshot_id,
            term_id: *term,
            source_fact_id: t.fact_id,
            kind: t.kind.text().into(),
            display: t.display.clone(),
            detail: t.detail.clone(),
            class_module: t.class_module.clone(),
            class_key: t.class_key.clone(),
            variable: t.variable.clone(),
        });
    }
    for a in args
        .iter()
        .filter(|a| reachable.contains(&a.parent_node_id))
    {
        out.type_args.push(CatalogTypeArgsRow {
            snapshot_id,
            parent_term_id: a.parent_node_id,
            role: a.role.text().into(),
            ordinal: a.ordinal,
            child_term_id: a.child_node_id,
            source_fact_id: a.fact_id,
            name: a.name.clone(),
            parameter_kind: a.parameter_kind.map(|k| k.text().into()),
            required: a.required,
        });
    }
    out.type_args.sort_by(|a, b| {
        (&a.parent_term_id, &a.role, a.ordinal, a.source_fact_id).cmp(&(
            &b.parent_term_id,
            &b.role,
            b.ordinal,
            b.source_fact_id,
        ))
    });
    for pair in out.type_args.windows(2) {
        let [a, b] = pair else { unreachable!() };
        if a.parent_term_id == b.parent_term_id && a.role == b.role && a.ordinal == b.ordinal {
            let mut same = a.clone();
            same.source_fact_id = b.source_fact_id;
            if same != *b {
                return Err(CoreError::Analysis(
                    "conflicting catalog type argument observations".into(),
                ));
            }
        }
    }
    out.type_args.dedup_by(|a, b| {
        a.parent_term_id == b.parent_term_id && a.role == b.role && a.ordinal == b.ordinal
    });
    for s in &mut out.signatures {
        let declared: BTreeSet<_> = observations
            .iter()
            .filter(|o| {
                Some(o.subject_node_id) == s.declaration_node_id
                    && o.role == cpg_schema::codebook::TypeRole::Return
                    && o.declared
            })
            .map(|o| o.term_node_id)
            .collect();
        if declared.len() == 1 {
            s.return_annotation = declared
                .first()
                .and_then(|id| by_term.get(id))
                .map(|t| t.display.clone());
        }
    }
    for id in selected {
        let Some(d) = by_decl.get(&id) else { continue };
        let f = by_file[&d.module_node_id];
        let Some(text) = &f.text else { continue };
        let text = text
            .get(d.start_byte as usize..d.end_byte as usize)
            .ok_or_else(|| {
                CoreError::Analysis("catalog declaration outside pinned source bytes".into())
            })?;
        out.evidence.push(CatalogEvidenceRow {
            snapshot_id,
            evidence_id: IdHasher::new("catalog-evidence").id(d.fact_id).finish_id(),
            subject_node_id: id,
            source_fact_id: d.fact_id,
            source_digest: f.content_digest,
            path: f.path.clone(),
            start_byte: d.start_byte,
            end_byte: d.end_byte,
            role: "declares".into(),
            text: text.into(),
        });
    }
    out.members = members.into_values().collect();
    out.bindings.sort_by_key(|b| (b.member_id, b.binding_id));
    out.bindings.dedup_by_key(|b| (b.member_id, b.binding_id));
    Ok(out)
}

/// Reconstruct mandatory contracts from canonical source facts. Shared by publication/tests;
/// projection receipts then bind these already-validated bytes through import and restore.
pub async fn validate(
    ctx: &SessionContext,
    selection: &CatalogCompilationRow,
) -> Result<Vec<crate::validate::Violation>, CoreError> {
    let public: Vec<PublicPathsRow> = sql::fetch(
        ctx,
        &cpg_schema::public::public_paths(),
        sql::Params::new().texts("roots", &selection.public_roots),
    )
    .await?;
    let expected = contracts(ctx, selection.snapshot_id, &selection.public_roots, &public).await?;
    let mut failures = Vec::new();
    macro_rules! check {
        ($table:ty, $expected:expr) => {{
            let actual = rows::<$table>(ctx).await?;
            let a =
                cpg_schema::table::canonical_sort(&<$table>::to_batch(&actual)?, <$table>::key())?;
            let e = cpg_schema::table::canonical_sort(
                &<$table>::to_batch(&$expected)?,
                <$table>::key(),
            )?;
            if a != e {
                failures.push(crate::validate::Violation {
                    rule: format!("catalog-source-equality:{}", <$table>::NAME),
                    rows: 1,
                    sample: "stored catalog differs from pinned source reconstruction".into(),
                });
            }
        }};
    }
    check!(cpg_schema::findings::PublicPaths, public);
    check!(CatalogBindings, expected.bindings);
    check!(CatalogSignatures, expected.signatures);
    check!(CatalogConstructors, expected.constructors);
    check!(CatalogParameters, expected.parameters);
    check!(CatalogEvidence, expected.evidence);
    check!(CatalogTypes, expected.types);
    check!(CatalogTypeArgs, expected.type_args);
    check!(CatalogTypeObservations, expected.type_observations);
    let mut actual_members = rows::<CatalogMembers>(ctx).await?;
    for member in &mut actual_members {
        // Rendering state depends on optional synthesis; identity and binding do not.
        member.brief_status = "not_requested".into();
        member.brief_reason = None;
    }
    let actual = cpg_schema::table::canonical_sort(
        &CatalogMembers::to_batch(&actual_members)?,
        CatalogMembers::key(),
    )?;
    let expected = cpg_schema::table::canonical_sort(
        &CatalogMembers::to_batch(&expected.members)?,
        CatalogMembers::key(),
    )?;
    if actual != expected {
        failures.push(crate::validate::Violation {
            rule: "catalog-source-equality:catalog_members".into(),
            rows: 1,
            sample: "public exposure/binding differs from source reconstruction".into(),
        });
    }
    Ok(failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_distinct_and_never_executed() {
        assert_eq!(
            default_value(Some("None"), Some(false)),
            ("literal_none".into(), Some("null".into()))
        );
        assert_eq!(default_value(None, Some(true)).0, "absent");
        assert_eq!(
            default_value(None, Some(false)).0,
            "optional_expression_unavailable"
        );
        assert_eq!(default_value(None, None).0, "unknown");
        assert_eq!(
            default_value(Some("factory()"), Some(false)).0,
            "source_expression"
        );
        assert_eq!(default_value(Some("42"), Some(false)).1, Some("42".into()));
    }
}
