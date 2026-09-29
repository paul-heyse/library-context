//! Pure contextual evidence derivation, composed with the API contract catalog (ADR-0076).
use crate::{
    CoreError,
    catalog::{CatalogFacts, Contracts},
    sql,
};
use cpg_schema::{
    Codebook, Id, IdHasher, Table,
    codebook::{
        BindingKind, BoundaryReason, CoverageStatus, EdgeKind, FactFamily, MentionClass, Modality,
        SourceRole, SyntaxField, SyntaxKind,
    },
    column::Blob,
    evidence::*,
    tables as raw,
};
use datafusion::prelude::SessionContext;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
pub struct EvidenceInputs {
    pub captured: Vec<CapturedArtifactsRow>,
    pub documents: Vec<raw::DocumentsRow>,
    pub passages: Vec<raw::PassagesRow>,
    pub blocks: Vec<raw::CodeBlocksRow>,
    pub mentions: Vec<raw::MentionsRow>,
    pub edges: Vec<cpg_schema::graph::EdgesRow>,
    pub coverage: Vec<raw::CoverageRow>,
    pub boundaries: Vec<raw::BoundariesRow>,
    pub facts: Vec<raw::FactsRow>,
    pub context_definitions: Vec<raw::ContextDefinitionsRow>,
    pub context_modules: Vec<raw::ContextModulesRow>,
    pub calls: Vec<raw::PysaCallsRow>,
    pub arguments: Vec<raw::ArgumentsRow>,
}
async fn rows<T: Table>(ctx: &SessionContext) -> Result<Vec<T::Row>, CoreError>
where
    T::Row: cpg_schema::query::QueryRow,
{
    sql::fetch(
        ctx,
        &cpg_schema::query::Relation {
            name: T::NAME,
            deps: T::DEPS,
            sql: format!("SELECT * FROM {}", T::NAME),
        },
        sql::Params::new(),
    )
    .await
}
macro_rules! evidence_inputs {
    ($($field:ident: $table:ty),+ $(,)?) => {
        pub fn input_dependencies()->Vec<&'static str> {vec![$(<$table as Table>::NAME),+]}
        pub async fn load(ctx:&SessionContext)->Result<EvidenceInputs,CoreError> {
            Ok(EvidenceInputs { $($field:rows::<$table>(ctx).await?,)+ })
        }
    };
}
evidence_inputs! {
    captured: CapturedArtifacts,
    documents: raw::Documents,
    passages: raw::Passages,
    blocks: raw::CodeBlocks,
    mentions: raw::Mentions,
    edges: cpg_schema::graph::Edges,
    coverage: raw::Coverage,
    boundaries: raw::Boundaries,
    facts: raw::Facts,
    context_definitions: raw::ContextDefinitions,
    context_modules: raw::ContextModules,
    calls: raw::PysaCalls,
    arguments: raw::Arguments,
}
#[derive(Default, Debug, Clone, PartialEq)]
pub struct EvidenceRows {
    pub artifacts: Vec<CatalogArtifactsRow>,
    pub spans: Vec<CatalogSpansRow>,
    pub scenarios: Vec<CatalogScenariosRow>,
    pub deployments: Vec<CatalogDeploymentsRow>,
    pub associations: Vec<CatalogAssociationsRow>,
}
fn encoded<T: serde::Serialize>(value: &T) -> Result<String, CoreError> {
    serde_json::to_string(value).map_err(|e| CoreError::Analysis(e.to_string()))
}
fn span(
    out: &mut EvidenceRows,
    index: &mut BTreeMap<Id, usize>,
    snapshot: Id,
    artifact: Id,
    start: i64,
    end: i64,
    extraction: &str,
) -> Id {
    let id = span_id(artifact, start, end);
    if let Some(position) = index.get(&id) {
        let existing = &mut out.spans[*position];
        if extraction < existing.extraction.as_str() {
            existing.extraction = extraction.into();
        }
    } else {
        index.insert(id, out.spans.len());
        out.spans.push(CatalogSpansRow {
            snapshot_id: snapshot,
            span_id: id,
            artifact_id: artifact,
            start_byte: start,
            end_byte: end,
            extraction: extraction.into(),
        });
    }
    id
}
#[allow(
    clippy::too_many_arguments,
    reason = "one explicit canonical association row with independent typed identity fields"
)]
fn associate(
    out: &mut EvidenceRows,
    snapshot: Id,
    member: Id,
    evidence: Id,
    kind: &str,
    role: &str,
    basis: &str,
    site: Option<Id>,
    intent: &str,
) {
    let id = IdHasher::new("evidence-association")
        .opt_id(Some(member))
        .opt_id(None)
        .id(evidence)
        .str(kind)
        .str(role)
        .str(basis)
        .opt_id(site)
        .str(intent)
        .finish_id();
    out.associations.push(CatalogAssociationsRow {
        snapshot_id: snapshot,
        association_id: id,
        member_id: Some(member),
        release_id: None,
        evidence_id: evidence,
        evidence_kind: kind.into(),
        role: role.into(),
        basis: basis.into(),
        site_id: site,
        intent: intent.into(),
        support: "[]".into(),
    });
}
fn original(text: &str, start: i64, end: i64) -> Option<&str> {
    text.get(usize::try_from(start).ok()?..usize::try_from(end).ok()?)
}

/// Immutable per-pass indexes. Alternatives remain sets rather than arbitrary winners.
pub struct PreparedEvidence<'a> {
    pub facts: &'a CatalogFacts,
    pub inputs: &'a EvidenceInputs,
    nodes: BTreeMap<Id, &'a raw::SyntaxNodesRow>,
    source: BTreeMap<Id, &'a raw::SourceFilesRow>,
    targets: BTreeMap<Id, BTreeSet<Id>>,
    edges: BTreeMap<Id, Vec<&'a cpg_schema::graph::EdgesRow>>,
    arguments: BTreeMap<Id, Vec<&'a raw::ArgumentsRow>>,
    children: BTreeMap<Id, Vec<&'a raw::SyntaxNodesRow>>,
    module_nodes: BTreeMap<Id, Vec<&'a raw::SyntaxNodesRow>>,
    module_bindings: BTreeMap<Id, Vec<&'a raw::BindingsRow>>,
}
impl<'a> PreparedEvidence<'a> {
    pub fn new(facts: &'a CatalogFacts, inputs: &'a EvidenceInputs) -> Self {
        let mut targets = BTreeMap::<Id, BTreeSet<Id>>::new();
        let mut edges = BTreeMap::<_, Vec<_>>::new();
        let mut arguments = BTreeMap::<_, Vec<_>>::new();
        for a in &inputs.arguments {
            arguments.entry(a.call_node_id).or_default().push(a);
        }
        for edge in &inputs.edges {
            if matches!(edge.edge_kind, EdgeKind::CallTarget | EdgeKind::SiteTarget) {
                targets
                    .entry(edge.src_node_id)
                    .or_default()
                    .insert(edge.dst_node_id);
                edges.entry(edge.src_node_id).or_default().push(edge);
            }
        }
        let mut children = BTreeMap::<_, Vec<_>>::new();
        let mut module_nodes = BTreeMap::<_, Vec<_>>::new();
        let mut module_bindings = BTreeMap::<_, Vec<_>>::new();
        for node in &facts.nodes {
            children.entry(node.parent_node_id).or_default().push(node);
            module_nodes
                .entry(node.module_node_id)
                .or_default()
                .push(node);
        }
        for binding in &facts.lexical_bindings {
            module_bindings
                .entry(binding.module_node_id)
                .or_default()
                .push(binding);
        }
        // Provider batches are sets. Original coordinates, rather than provider fact IDs
        // or arrival order, define the order of contextual imports and parameters.
        for nodes in module_nodes.values_mut().chain(children.values_mut()) {
            nodes.sort_by_key(|n| (n.start_byte, n.end_byte, n.node_id));
        }
        for bindings in module_bindings.values_mut() {
            bindings.sort_by(|a, b| {
                (a.start_byte, a.end_byte, &a.name).cmp(&(b.start_byte, b.end_byte, &b.name))
            });
        }
        Self {
            facts,
            inputs,
            nodes: facts.nodes.iter().map(|n| (n.node_id, n)).collect(),
            source: facts.source.iter().map(|s| (s.module_node_id, s)).collect(),
            targets,
            edges,
            arguments,
            children,
            module_nodes,
            module_bindings,
        }
    }
    fn ancestors(&self, site: Id) -> Vec<&raw::SyntaxNodesRow> {
        let mut out = Vec::new();
        let mut at = site;
        let mut seen = BTreeSet::new();
        while seen.insert(at) {
            let Some(n) = self.nodes.get(&at) else { break };
            out.push(*n);
            at = n.parent_node_id;
        }
        out
    }
    fn target(&self, n: &raw::SyntaxNodesRow, text: &str) -> Option<String> {
        let p =
            ruff_python_parser::parse_expression(original(text, n.start_byte, n.end_byte)?).ok()?;
        crate::surface::resolved(&p.syntax().body, n.module_node_id, n.start_byte, self.facts)
    }
    fn parse_status(&self, module: Id) -> CheckStatus {
        if self
            .inputs
            .boundaries
            .iter()
            .any(|b| b.module_node_id == module && b.reason == BoundaryReason::SyntaxError)
        {
            return CheckStatus::Failed;
        }
        if self.inputs.coverage.iter().any(|c| {
            c.scope_node_id == module
                && c.fact_family == FactFamily::Syntax
                && c.status == CoverageStatus::CompleteUnderStatedModel
        }) {
            CheckStatus::Passed
        } else {
            CheckStatus::Blocked
        }
    }
    fn expected_manager(&self, expr: &raw::SyntaxNodesRow, text: &str) -> bool {
        if matches!(
            self.target(expr, text).as_deref(),
            Some(
                "pytest.raises"
                    | "unittest.TestCase.assertRaises"
                    | "unittest.TestCase.assertRaisesRegex"
                    | "unittest.case.TestCase.assertRaises"
                    | "unittest.case.TestCase.assertRaisesRegex"
            )
        ) {
            return true;
        }
        // Instance spelling and a TestCase base do not prove the selected method: overrides,
        // MRO and attribute replacement remain possible without a resolved provider target.
        self.edges.get(&expr.node_id).is_some_and(|edges| {
            edges.iter().any(|edge| {
                self.inputs.facts.iter().any(|fact| {
                    fact.fact_id == edge.evidence_fact_id && fact.modality == Modality::Definite
                }) && self.inputs.calls.iter().any(|call| {
                    call.fact_id == edge.evidence_fact_id && call.unresolved_reason.is_none()
                }) && self.inputs.context_definitions.iter().any(|definition| {
                    definition.symbol_node_id == edge.dst_node_id
                        && matches!(
                            format!("{}.{}", definition.module_name, definition.qualified_name)
                                .as_str(),
                            "unittest.case.TestCase.assertRaises"
                                | "unittest.case.TestCase.assertRaisesRegex"
                        )
                })
            })
        })
    }

    fn intent(&self, site: Id, text: &str, role: SourceRole) -> &'static str {
        let mut child = site;
        for parent in self.ancestors(site).into_iter().skip(1) {
            let Some(immediate) = self.nodes.get(&child) else {
                break;
            };
            if parent.kind == SyntaxKind::StmtWith && immediate.field == SyntaxField::Body {
                for item in self
                    .children
                    .get(&parent.node_id)
                    .into_iter()
                    .flatten()
                    .filter(|n| n.field == SyntaxField::Item)
                {
                    for expr in self
                        .children
                        .get(&item.node_id)
                        .into_iter()
                        .flatten()
                        .filter(|n| n.field == SyntaxField::Value)
                    {
                        if self.expected_manager(expr, text) {
                            return "expected_failure";
                        }
                    }
                }
            }
            if parent.kind == SyntaxKind::StmtFunctionDef && immediate.field == SyntaxField::Body {
                for decorator in self
                    .children
                    .get(&parent.node_id)
                    .into_iter()
                    .flatten()
                    .filter(|n| n.field == SyntaxField::Decorator)
                {
                    // Decorator nodes include @; its expression child owns the exact parse range.
                    let target = self.target(decorator, text).or_else(|| {
                        self.children
                            .get(&decorator.node_id)
                            .into_iter()
                            .flatten()
                            .find_map(|expr| self.target(expr, text))
                    });
                    if matches!(
                        target.as_deref(),
                        Some("pytest.mark.skip" | "pytest.mark.skipif" | "pytest.mark.xfail")
                    ) {
                        return "skip_xfail";
                    }
                }
                break; // A deferred body never inherits a surrounding with context.
            }
            if parent.kind == SyntaxKind::ExprGenerator {
                use ruff_text_size::Ranged;
                // Only the outermost iterable is evaluated when a generator is constructed.
                let eager = original(text, parent.start_byte, parent.end_byte)
                    .and_then(|source| ruff_python_parser::parse_expression(source).ok())
                    .is_some_and(|parsed| match &*parsed.syntax().body {
                        ruff_python_ast::Expr::Generator(generator) => {
                            generator.generators.first().is_some_and(|first| {
                                self.nodes.get(&site).is_some_and(|site| {
                                    site.start_byte
                                        >= parent.start_byte
                                            + i64::from(first.iter.start().to_u32())
                                        && site.end_byte
                                            <= parent.start_byte
                                                + i64::from(first.iter.end().to_u32())
                                })
                            })
                        }
                        _ => false,
                    });
                if !eager {
                    break;
                }
            }
            if parent.kind == SyntaxKind::ExprLambda {
                break;
            }
            child = parent.node_id;
        }
        if role == SourceRole::Test {
            "assertion_test"
        } else {
            "demonstration"
        }
    }
    pub fn derive(&self, snapshot: Id, catalog: &Contracts) -> Result<EvidenceRows, CoreError> {
        let mut out = EvidenceRows::default();
        let mut span_index = BTreeMap::new();
        let members_by_path: BTreeMap<_, _> = catalog
            .members
            .iter()
            .map(|m| (m.access_path.as_str(), m))
            .collect();
        let declarations: BTreeMap<_, _> = self
            .facts
            .declarations
            .iter()
            .map(|d| (d.node_id, d))
            .collect();
        let mut qualified_members = BTreeMap::<&str, Vec<Id>>::new();
        for member in &catalog.members {
            if let Some(d) = member
                .operation_node_id
                .and_then(|id| declarations.get(&id))
            {
                qualified_members
                    .entry(&d.qualified_name)
                    .or_default()
                    .push(member.member_id);
            }
        }
        for a in &self.inputs.captured {
            out.artifacts.push(CatalogArtifactsRow {
                snapshot_id: snapshot,
                artifact_id: a.artifact_id,
                release_id: a.release_id,
                path: a.path.clone(),
                source_kind: a.source_kind.clone(),
                source_digest: a.source_digest,
                byte_len: a.byte_len,
                body: a.body.clone(),
                alignment: a.alignment.clone(),
                provenance: a.provenance.clone(),
            });
        }
        let mut module_artifact = BTreeMap::new();
        for s in &self.facts.source {
            // Synthetic Python is analyzed evidence, not original documentation bytes.
            if s.role == SourceRole::DocBlock {
                continue;
            }
            if let Some(text) = &s.text {
                let artifact = artifact_id(s.release_id, &s.path, s.content_digest);
                module_artifact.insert(s.module_node_id, artifact);
                out.artifacts.push(CatalogArtifactsRow {
                    snapshot_id: snapshot,
                    artifact_id: artifact,
                    release_id: s.release_id,
                    path: s.path.clone(),
                    source_kind: s.role.text().into(),
                    source_digest: s.content_digest,
                    byte_len: s.byte_len,
                    body: Blob(text.as_bytes().to_vec()),
                    alignment: if s.role == SourceRole::Release {
                        "exact"
                    } else {
                        "mapped_with_evidence"
                    }
                    .into(),
                    provenance: format!(
                        "source fact {}; {}",
                        s.fact_id.hex(),
                        self.facts
                            .releases
                            .iter()
                            .find(|r| r.release_id == s.release_id)
                            .and_then(|r| r.label.as_deref().or(r.requirement.as_deref()))
                            .unwrap_or("release identity unavailable")
                    ),
                });
            }
        }
        let documents: BTreeMap<_, _> = self
            .inputs
            .documents
            .iter()
            .map(|d| (d.node_id, d))
            .collect();
        let mut passages = BTreeMap::new();
        for p in &self.inputs.passages {
            if let Some(d) = documents.get(&p.document_node_id) {
                let a = artifact_id(d.release_id, &d.path, d.content_digest);
                passages.insert(
                    p.node_id,
                    span(
                        &mut out,
                        &mut span_index,
                        snapshot,
                        a,
                        p.start_byte,
                        p.end_byte,
                        "original_document_passage",
                    ),
                );
            }
        }
        let mut module_scenario = BTreeMap::new();
        for b in &self.inputs.blocks {
            let Some(d) = documents.get(&b.document_node_id) else {
                continue;
            };
            let a = artifact_id(d.release_id, &d.path, d.content_digest);
            let primary = span(
                &mut out,
                &mut span_index,
                snapshot,
                a,
                b.start_byte,
                b.end_byte,
                "original_document_fence",
            );
            if let Some(path) = &b.module_path {
                let source = self
                    .facts
                    .source
                    .iter()
                    .find(|s| s.release_id == d.release_id && &s.path == path);
                let id = IdHasher::new("contextual-scenario-v1")
                    .id(primary)
                    .finish_id();
                let mut spans = vec![primary];
                if let Some(p) = passages.get(&b.passage_node_id)
                    && *p != primary
                {
                    spans.push(*p);
                }
                let detail=ScenarioDetail {spans,context:ContextStatus::ContextDependent,intent:Intent::Demonstration,
                    checks:Checks {parse:source.map_or(CheckStatus::Blocked,|s|self.parse_status(s.module_node_id)),..Checks::default()},
                    requirements:vec![ContextRequirement {kind:"document_context".into(),expression:"A fenced block does not prove a standalone runnable setup; consult its enclosing passage.".into(),evidence:passages.get(&b.passage_node_id).copied()}],
                    extraction:"markdown code value materialized as Python; original fence coordinates only".into(),analysis_module:source.map(|s|s.module_node_id),option_bindings:vec![],omitted_options:0,omitted_requirements:0};
                out.scenarios.push(CatalogScenariosRow {
                    snapshot_id: snapshot,
                    scenario_id: id,
                    primary_span_id: primary,
                    detail: encoded(&detail)?,
                });
                if let Some(s) = source {
                    module_scenario.insert(s.module_node_id, id);
                }
            } else if matches!(
                b.language.as_deref(),
                Some("bash" | "sh" | "shell" | "console" | "json" | "toml" | "yaml" | "yml")
            ) {
                let detail=DeploymentDetail {distribution:None,version:None,field:format!("original_{}_snippet",b.language.as_deref().unwrap()),original:b.code.clone(),name:None,extras:vec![],marker:None,constraint:None,interpretation:CheckStatus::NotRun,diagnostic:Some("Original source data; launch behavior and prerequisites are not inferred by executing it.".into()),environment_digest:None,lock_digest:None,task:None,referenced_path:None};
                let id = IdHasher::new("deployment-snippet").id(primary).finish_id();
                out.deployments.push(CatalogDeploymentsRow {
                    snapshot_id: snapshot,
                    deployment_id: id,
                    span_id: primary,
                    ordinal: 0,
                    derivation: "snippet".into(),
                    detail: encoded(&detail)?,
                });
                for mention in self
                    .inputs
                    .mentions
                    .iter()
                    .filter(|m| m.passage_node_id == b.passage_node_id)
                {
                    if let Some(member) = mention
                        .access_path
                        .as_deref()
                        .and_then(|p| members_by_path.get(p))
                    {
                        associate(
                            &mut out,
                            snapshot,
                            member.member_id,
                            id,
                            "deployment",
                            "suggests",
                            "same_document_passage",
                            None,
                            "unknown",
                        );
                    }
                }
            }
        }
        let mut members_by_target = BTreeMap::<Id, BTreeSet<Id>>::new();
        for m in &catalog.members {
            if let Some(n) = m.operation_node_id {
                members_by_target.entry(n).or_default().insert(m.member_id);
            }
        }
        for b in &catalog.bindings {
            if let Some(n) = b.declaration_node_id {
                members_by_target.entry(n).or_default().insert(b.member_id);
            }
        }
        let context_modules: BTreeMap<_, _> = self
            .inputs
            .context_modules
            .iter()
            .map(|m| (m.module_node_id, m))
            .collect();
        let mut by_qualified = BTreeMap::<&str, Vec<_>>::new();
        for declaration in &self.facts.declarations {
            by_qualified
                .entry(&declaration.qualified_name)
                .or_default()
                .push(declaration);
        }
        for definition in &self.inputs.context_definitions {
            let Some(context) = context_modules.get(&definition.module_node_id) else {
                continue;
            };
            for declaration in by_qualified
                .get(format!("{}.{}", definition.module_name, definition.qualified_name).as_str())
                .into_iter()
                .flatten()
            {
                let Some(source) = self.source.get(&declaration.module_node_id) else {
                    continue;
                };
                if source.role == SourceRole::Release
                    && context.path.as_deref() == Some(source.path.as_str())
                    && context.distribution == source.distribution
                    && self.facts.releases.iter().any(|r| {
                        r.release_id == source.release_id
                            && (source.distribution.is_none() && context.version.is_none()
                                || source
                                    .distribution
                                    .as_ref()
                                    .zip(context.version.as_ref())
                                    .is_some_and(|(d, v)| {
                                        r.distributions.contains(&format!("{d}=={v}"))
                                    }))
                    })
                    && format!("{}.{}", definition.module_name, definition.qualified_name)
                        == declaration.qualified_name
                {
                    let members = members_by_target
                        .get(&declaration.node_id)
                        .cloned()
                        .unwrap_or_default();
                    members_by_target
                        .entry(definition.symbol_node_id)
                        .or_default()
                        .extend(members);
                }
            }
        }
        let fact_map: BTreeMap<_, _> = self.inputs.facts.iter().map(|f| (f.fact_id, f)).collect();
        let calls: BTreeMap<_, _> = self.inputs.calls.iter().map(|c| (c.fact_id, c)).collect();
        for (site, targets) in &self.targets {
            let Some(node) = self.nodes.get(site) else {
                continue;
            };
            let Some(source) = self.source.get(&node.module_node_id) else {
                continue;
            };
            if source.role == SourceRole::Release {
                continue;
            }
            let Some(text) = &source.text else { continue };
            let members: BTreeSet<_> = targets
                .iter()
                .flat_map(|t| members_by_target.get(t).into_iter().flatten().copied())
                .collect();
            if members.is_empty() {
                continue;
            }
            let scenario = if let Some(s) = module_scenario.get(&source.module_node_id) {
                *s
            } else {
                let Some(a) = module_artifact.get(&source.module_node_id) else {
                    continue;
                };
                let ancestors = self.ancestors(*site);
                let enclosing = ancestors.iter().rev().find(|n| {
                    matches!(
                        n.kind,
                        SyntaxKind::StmtFunctionDef | SyntaxKind::StmtClassDef
                    )
                });
                let (start, end) =
                    enclosing.map_or((0, source.byte_len), |n| (n.start_byte, n.end_byte));
                let primary = span(
                    &mut out,
                    &mut span_index,
                    snapshot,
                    *a,
                    start,
                    end,
                    "original_enclosing_context",
                );
                let id = IdHasher::new("contextual-scenario-v1")
                    .id(primary)
                    .finish_id();
                if !out.scenarios.iter().any(|s| s.scenario_id == id) {
                    let mut requirements=vec![ContextRequirement{kind:"runtime_inputs".into(),expression:"Source context is preserved; external inputs, file access and runtime bindings have not been executed.".into(),evidence:Some(primary)}];
                    let mut spans = vec![primary];
                    // Imports are original separate context spans; they are never stitched into executable code.
                    for import in self
                        .module_nodes
                        .get(&source.module_node_id)
                        .into_iter()
                        .flatten()
                        .filter(|n| {
                            matches!(n.kind, SyntaxKind::StmtImport | SyntaxKind::StmtImportFrom)
                                && n.end_byte <= start
                        })
                    {
                        spans.push(span(
                            &mut out,
                            &mut span_index,
                            snapshot,
                            *a,
                            import.start_byte,
                            import.end_byte,
                            "original_import_context",
                        ));
                    }
                    if start > 0 {
                        let module = span(
                            &mut out,
                            &mut span_index,
                            snapshot,
                            *a,
                            0,
                            source.byte_len,
                            "original_module_context",
                        );
                        requirements.push(ContextRequirement {kind:"enclosing_module".into(),expression:"Module initialization and enclosing bindings are not proved by this function excerpt.".into(),evidence:Some(module)});
                    }
                    for binding in self
                        .module_bindings
                        .get(&source.module_node_id)
                        .into_iter()
                        .flatten()
                        .filter(|b| {
                            b.start_byte >= start
                                && b.end_byte <= end
                                && b.kind == BindingKind::Parameter
                        })
                    {
                        requirements.push(ContextRequirement {
                            kind: if source.role == SourceRole::Test {
                                "test_parameter_or_fixture"
                            } else {
                                "function_parameter"
                            }
                            .into(),
                            expression: binding.name.clone(),
                            evidence: Some(primary),
                        });
                    }
                    if source.role == SourceRole::Test {
                        requirements.push(ContextRequirement {kind:"test_environment".into(),expression:"Fixture, parametrization, conftest and plugin setup has not been executed.".into(),evidence:Some(primary)});
                    }
                    let detail = ScenarioDetail {
                        spans,
                        context: if requirements.is_empty() {
                            ContextStatus::Complete
                        } else {
                            ContextStatus::ContextDependent
                        },
                        intent: if source.role == SourceRole::Test {
                            Intent::AssertionTest
                        } else {
                            Intent::Demonstration
                        },
                        checks: Checks {
                            parse: self.parse_status(source.module_node_id),
                            ..Checks::default()
                        },
                        requirements,
                        extraction:
                            "original enclosing Python context; independent import references"
                                .into(),
                        analysis_module: Some(source.module_node_id),
                        option_bindings: vec![],
                        omitted_options: 0,
                        omitted_requirements: 0,
                    };
                    out.scenarios.push(CatalogScenariosRow {
                        snapshot_id: snapshot,
                        scenario_id: id,
                        primary_span_id: primary,
                        detail: encoded(&detail)?,
                    });
                }
                id
            };
            let intent = self.intent(*site, text, source.role);
            for edge in self.edges.get(site).into_iter().flatten() {
                let Some(fact) = fact_map.get(&edge.evidence_fact_id) else {
                    continue;
                };
                let call = calls.get(&edge.evidence_fact_id);
                let context_span_id = out
                    .scenarios
                    .iter()
                    .find(|s| s.scenario_id == scenario)
                    .unwrap()
                    .primary_span_id;
                let support = AssociationSupport {
                    context_span_id,
                    analysis_module: source.module_node_id,
                    start_byte: node.start_byte,
                    end_byte: node.end_byte,
                    coordinate_space: if source.role == SourceRole::DocBlock {
                        "synthetic_analysis"
                    } else {
                        "original"
                    }
                    .into(),
                    edge_id: edge.edge_id,
                    fact_id: edge.evidence_fact_id,
                    support_fact_id: edge.support_fact_id,
                    target_id: edge.dst_node_id,
                    modality: fact.modality.text().into(),
                    phase: call.map_or("unknown", |c| c.phase.text()).into(),
                    unresolved_reason: call
                        .and_then(|c| c.unresolved_reason)
                        .map(|r| r.text().into()),
                };
                for member in members_by_target
                    .get(&edge.dst_node_id)
                    .into_iter()
                    .flatten()
                {
                    let basis = if fact.modality == Modality::Definite
                        && call.is_some_and(|c| c.unresolved_reason.is_none())
                    {
                        "resolved_target"
                    } else {
                        "candidate_targets"
                    };
                    associate(
                        &mut out,
                        snapshot,
                        *member,
                        scenario,
                        "scenario",
                        if intent == "expected_failure" {
                            "tests_failure"
                        } else {
                            "invokes"
                        },
                        basis,
                        Some(*site),
                        intent,
                    );
                    let row = out.associations.last_mut().unwrap();
                    row.support = encoded(&vec![support.clone()])?;
                    row.association_id = IdHasher::new("evidence-association-supported")
                        .id(row.association_id)
                        .id(edge.edge_id)
                        .finish_id();
                }
            }
            let mut options = Vec::new();
            for argument in self.arguments.get(site).into_iter().flatten() {
                let Some(expression) =
                    original(text, argument.value_start_byte, argument.value_end_byte)
                else {
                    continue;
                };
                let original_span = module_artifact.get(&source.module_node_id).map(|artifact| {
                    span(
                        &mut out,
                        &mut span_index,
                        snapshot,
                        *artifact,
                        argument.value_start_byte,
                        argument.value_end_byte,
                        "original_argument_expression",
                    )
                });
                options.push(OptionBinding {
                    site_id: *site,
                    ordinal: argument.ordinal,
                    keyword: argument.keyword.clone(),
                    kind: argument.kind.text().into(),
                    expression: expression.into(),
                    span_id: original_span,
                    coordinate_space: if original_span.is_some() {
                        "original"
                    } else {
                        "synthetic_analysis"
                    }
                    .into(),
                });
            }
            if let Some(row) = out.scenarios.iter_mut().find(|s| s.scenario_id == scenario) {
                let mut detail: ScenarioDetail = serde_json::from_str(&row.detail)
                    .map_err(|e| CoreError::Analysis(e.to_string()))?;
                detail.option_bindings.extend(options);
                detail
                    .option_bindings
                    .sort_by_key(|o| (o.site_id, o.ordinal));
                detail.option_bindings.dedup();
                row.detail = encoded(&detail)?;
            }
        }
        for mention in &self.inputs.mentions {
            let Some(evidence) = passages.get(&mention.passage_node_id) else {
                continue;
            };
            let members: BTreeSet<Id> = mention
                .access_path
                .as_deref()
                .and_then(|p| members_by_path.get(p))
                .map(|m| m.member_id)
                .into_iter()
                .chain(
                    mention
                        .qualified_name
                        .as_deref()
                        .and_then(|q| qualified_members.get(q))
                        .into_iter()
                        .flatten()
                        .copied(),
                )
                .collect();
            let ambiguous = members.len() > 1;
            for member in members {
                associate(
                    &mut out,
                    snapshot,
                    member,
                    *evidence,
                    "span",
                    "documents",
                    if mention.class == MentionClass::Exact && !ambiguous {
                        "exact_textual_reference"
                    } else {
                        "ambiguous_textual_mention"
                    },
                    None,
                    "unknown",
                );
            }
        }
        // Match the catalog contract's complete subject closure, separately from callable
        // exposure. A private inherited constructor is evidence for a public class, not an
        // assertion that a direct call of that private class invokes the public subclass.
        let mut evidence_members = BTreeMap::<Id, BTreeSet<Id>>::new();
        for member in &catalog.members {
            let declarations: BTreeSet<_> = catalog
                .bindings
                .iter()
                .filter(|b| b.member_id == member.member_id)
                .filter_map(|b| b.declaration_node_id)
                .chain(member.operation_node_id)
                .collect();
            let constructors: BTreeSet<_> = catalog
                .constructors
                .iter()
                .filter(|c| declarations.contains(&c.class_node_id))
                .map(|c| c.signature_id)
                .collect();
            let signatures: Vec<_> = catalog
                .signatures
                .iter()
                .filter(|s| {
                    s.declaration_node_id
                        .is_some_and(|d| declarations.contains(&d))
                        || constructors.contains(&s.signature_id)
                })
                .collect();
            let signature_ids: BTreeSet<_> = signatures.iter().map(|s| s.signature_id).collect();
            let mut subjects = declarations;
            subjects.extend(signatures.iter().filter_map(|s| s.declaration_node_id));
            subjects.extend(signatures.iter().filter_map(|s| s.constructor_class_id));
            subjects.extend(
                catalog
                    .parameters
                    .iter()
                    .filter(|p| signature_ids.contains(&p.signature_id))
                    .filter_map(|p| p.formal_node_id),
            );
            let fields: Vec<_> = catalog
                .configurations
                .iter()
                .filter(|c| subjects.contains(&c.class_node_id))
                .map(|c| c.field_id)
                .collect();
            subjects.extend(fields);
            for subject in subjects {
                evidence_members
                    .entry(subject)
                    .or_default()
                    .insert(member.member_id);
            }
        }
        // Existing declaration evidence becomes a reusable original span association.
        for evidence in &catalog.evidence {
            for s in
                self.facts.source.iter().filter(|s| {
                    s.path == evidence.path && s.content_digest == evidence.source_digest
                })
            {
                let Some(a) = module_artifact.get(&s.module_node_id) else {
                    continue;
                };
                let id = span(
                    &mut out,
                    &mut span_index,
                    snapshot,
                    *a,
                    evidence.start_byte,
                    evidence.end_byte,
                    "original_catalog_source",
                );
                for member in evidence_members
                    .get(&evidence.subject_node_id)
                    .into_iter()
                    .flatten()
                {
                    associate(
                        &mut out,
                        snapshot,
                        *member,
                        id,
                        "span",
                        &evidence.role,
                        "source_fact",
                        None,
                        "unknown",
                    );
                }
            }
        }
        for captured in &self.inputs.captured {
            let mut observations: Vec<DeploymentDetail> =
                serde_json::from_str(&captured.observations)
                    .map_err(|e| CoreError::Analysis(e.to_string()))?;

            if matches!(captured.source_kind.as_str(), "document" | "invalid_python")
                && !out
                    .spans
                    .iter()
                    .any(|s| s.artifact_id == captured.artifact_id)
            {
                span(
                    &mut out,
                    &mut span_index,
                    snapshot,
                    captured.artifact_id,
                    0,
                    captured.byte_len,
                    "original_uninterpreted_source",
                );
            }
            if observations.is_empty()
                && (captured.source_kind == "configuration" || captured.source_kind == "asset")
            {
                observations.push(DeploymentDetail{distribution:None,version:None,field:"original_configuration".into(),original:captured.path.clone(),name:None,extras:vec![],marker:None,constraint:None,interpretation:CheckStatus::NotRun,diagnostic:Some("Selected original configuration; effective launch semantics and prerequisite sufficiency are unproved.".into()),environment_digest:None,lock_digest:None,task:None,referenced_path:None});
            }
            for (ordinal, detail) in observations.iter().enumerate() {
                let primary = span(
                    &mut out,
                    &mut span_index,
                    snapshot,
                    captured.artifact_id,
                    0,
                    captured.byte_len,
                    "original_acquired_artifact",
                );
                let id = IdHasher::new("deployment-metadata")
                    .id(primary)
                    .i64(ordinal as i64)
                    .finish_id();
                out.deployments.push(CatalogDeploymentsRow {
                    snapshot_id: snapshot,
                    deployment_id: id,
                    span_id: primary,
                    ordinal: ordinal as i64,
                    derivation: "metadata".into(),
                    detail: encoded(detail)?,
                });
                if let Some(path) = &detail.referenced_path {
                    let artifacts: BTreeSet<_> = out
                        .artifacts
                        .iter()
                        .filter(|a| a.release_id == captured.release_id && a.path == *path)
                        .map(|a| a.artifact_id)
                        .collect();
                    let scenarios: BTreeSet<_> = out
                        .scenarios
                        .iter()
                        .filter(|s| {
                            out.spans.iter().any(|p| {
                                p.span_id == s.primary_span_id && artifacts.contains(&p.artifact_id)
                            })
                        })
                        .map(|s| s.scenario_id)
                        .collect();
                    let members: BTreeSet<_> = out
                        .associations
                        .iter()
                        .filter(|a| {
                            a.evidence_kind == "scenario" && scenarios.contains(&a.evidence_id)
                        })
                        .filter_map(|a| a.member_id)
                        .collect();
                    for member in members {
                        associate(
                            &mut out,
                            snapshot,
                            member,
                            id,
                            "deployment",
                            "configuration",
                            "explicit_config_reference",
                            None,
                            "unknown",
                        );
                    }
                }
                if let Some(task) = &detail.task {
                    let scenarios: BTreeSet<_> = out
                        .scenarios
                        .iter()
                        .filter(|s| {
                            out.spans.iter().any(|span| {
                                span.span_id == s.primary_span_id
                                    && span.artifact_id == task.target_artifact
                            })
                        })
                        .map(|s| s.scenario_id)
                        .collect();
                    let associated: Vec<_> = out
                        .associations
                        .iter()
                        .filter(|a| {
                            a.evidence_kind == "scenario" && scenarios.contains(&a.evidence_id)
                        })
                        .filter_map(|a| a.member_id.map(|member| (member, a.site_id)))
                        .collect();
                    for (member, site) in associated {
                        associate(
                            &mut out,
                            snapshot,
                            member,
                            id,
                            "deployment",
                            "observes",
                            "task_observation",
                            site,
                            "unknown",
                        );
                    }
                }
                if detail.distribution.as_deref().is_some_and(|distribution| {
                    self.facts.releases.iter().any(|r| {
                        r.release_id == captured.release_id
                            && r.distributions.iter().any(|d| {
                                d.split_once("==")
                                    .is_some_and(|(name, _)| name == distribution)
                            })
                    })
                }) {
                    let release = captured.release_id;
                    let association_id = IdHasher::new("evidence-association")
                        .opt_id(None)
                        .opt_id(Some(release))
                        .id(id)
                        .str("deployment")
                        .str("declares")
                        .str("release_distribution")
                        .opt_id(None)
                        .str("unknown")
                        .finish_id();
                    out.associations.push(CatalogAssociationsRow {
                        snapshot_id: snapshot,
                        association_id,
                        member_id: None,
                        release_id: Some(release),
                        evidence_id: id,
                        evidence_kind: "deployment".into(),
                        role: "declares".into(),
                        basis: "release_distribution".into(),
                        site_id: None,
                        intent: "unknown".into(),
                        support: "[]".into(),
                    });
                }
            }
        }
        let mut artifacts: BTreeSet<_> = out.spans.iter().map(|s| s.artifact_id).collect();
        for deployment in &out.deployments {
            let d: DeploymentDetail = serde_json::from_str(&deployment.detail)
                .map_err(|e| CoreError::Analysis(e.to_string()))?;
            if let Some(t) = d.task {
                artifacts.insert(t.target_artifact);
            }
        }
        for scenario in &mut out.scenarios {
            let mut d: ScenarioDetail = serde_json::from_str(&scenario.detail)
                .map_err(|e| CoreError::Analysis(e.to_string()))?;
            let intents: BTreeSet<_> = out
                .associations
                .iter()
                .filter(|a| a.evidence_kind == "scenario" && a.evidence_id == scenario.scenario_id)
                .map(|a| a.intent.as_str())
                .collect();
            if intents.len() > 1 {
                d.intent = Intent::Mixed;
            } else if let Some(intent) = intents.first() {
                d.intent = serde_json::from_value(serde_json::Value::String((*intent).into()))
                    .map_err(|e| CoreError::Analysis(e.to_string()))?;
            }
            if d.option_bindings.len() > 128 || d.requirements.len() > 128 {
                d.omitted_options = d.option_bindings.len().saturating_sub(128) as u64;
                d.omitted_requirements = d.requirements.len().saturating_sub(128) as u64;
                d.option_bindings.truncate(128);
                d.requirements.truncate(128);
                d.context = ContextStatus::Truncated;
            }
            scenario.detail = encoded(&d)?;
        }
        out.artifacts.retain(|a| artifacts.contains(&a.artifact_id));
        out.artifacts.sort_by_key(|r| r.artifact_id);
        out.artifacts.dedup_by_key(|r| r.artifact_id);
        out.spans.sort_by_key(|r| r.span_id);
        out.scenarios.sort_by_key(|r| r.scenario_id);
        out.deployments.sort_by_key(|r| r.deployment_id);
        out.associations.sort_by_key(|r| r.association_id);
        out.associations.dedup_by_key(|r| r.association_id);
        cpg_schema::evidence::validate(
            &out.artifacts,
            &out.spans,
            &out.scenarios,
            &out.deployments,
            &out.associations,
        )
        .map_err(CoreError::Analysis)?;
        Ok(out)
    }
}
