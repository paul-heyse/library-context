//! Stage E (DESIGN §4.1, §5, §9; ADR-0019): run the declared projections on the attempt's
//! session, hand their batches to `lctx-analytics`, and return its rows for the attempt to write.
//!
//! Analysis results are a run of producer `lctx-compiler` over the library release and its
//! context. Its `runs` and `producers` rows are made here, before any raw table is written, so
//! `content_digest` includes the analytics config through the run's config digest.

use std::collections::BTreeMap;

use arrow_array::{Array, BooleanArray, FixedSizeBinaryArray, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use cpg_schema::codebook::{
    AnalyticMethod, Codebook, CoverageStatus, ExtractionMode, FindingKind, MemberRole, NodeKind,
    SourceRole,
};
use cpg_schema::findings::{
    AnalysisInvocationsRow, FindingMembersRow, FindingsRow, WitnessesRow, recipe as findings,
};
use cpg_schema::id::{Digest, Id, content_digest, recipe};
use cpg_schema::projection::{self, ProjectionSpec, schemas};
use cpg_schema::tables::{ProducersRow, RunsRow};
use datafusion::prelude::SessionContext;
use lctx_analytics::config::AnalyticsConfig;
use lctx_analytics::graph::Projection;
use lctx_analytics::pass_a::{self, Budgets, Seed};
use lctx_analytics::pass_b::{self, Flows, SeedParameter};
use lctx_analytics::pass_c::{self, Handoffs};
use lctx_analytics::{communities, concepts, neighbours, ranking, selection};
use serde::Serialize;

use crate::delta::to_schema;
use crate::{CoreError, sql};

/// The compiler's producer tool name.
pub const TOOL: &str = "lctx-compiler";

/// What an attempt analyzes: the pre-registered config (DESIGN §1.4, §9), and the embedder
/// brief documents are embedded with (none: the documents carry no `input_hash`).
#[derive(Clone)]
pub struct Analysis {
    pub config: AnalyticsConfig,
    pub embedder: Option<std::sync::Arc<dyn crate::embed::Embedder>>,
    /// Which analytics techniques run (the §9.8 ablation's variants; slice 3.2).
    pub techniques: Techniques,
}

/// The analytics techniques a compile runs (DESIGN §9.8; slices 3.2, 3.3). The default is the
/// kept set; a variant adds or removes techniques by name (`-knn`, `+rca`), and its label joins
/// the compiler run's config digest, so a variant's snapshot never shares a content digest with
/// the default's. Finding and assertion ids do not depend on it, so ablation diffs are joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Techniques {
    pub communities: bool,
    /// Delegation PageRank orders seeds and Related instead of direct usage (§9.5; the
    /// increment-2 review's U1 keeps it only for the ablation).
    pub pagerank: bool,
    pub fca: bool,
    pub knn: bool,
    /// FCA's relational scaling: `calls X` and `hands off to X` attributes (§9.6, 3.2).
    pub rca: bool,
    /// A community layer of shared declared parameter types (§9.4, 3.2).
    pub type_layer: bool,
    /// A community layer of doc co-mentions (§9.4, 3.2; C5 O1).
    pub mention_layer: bool,
    /// A community layer of API–API embedding neighbours (§9.4, §9.7, 3.2).
    pub knn_layer: bool,
}

impl Default for Techniques {
    fn default() -> Self {
        Techniques {
            communities: true,
            pagerank: false,
            fca: true,
            knn: true,
            rca: false,
            type_layer: false,
            mention_layer: false,
            knn_layer: false,
        }
    }
}

impl Techniques {
    fn flags(&mut self) -> [(&'static str, &mut bool); 8] {
        [
            ("communities", &mut self.communities),
            ("pagerank", &mut self.pagerank),
            ("fca", &mut self.fca),
            ("knn", &mut self.knn),
            ("rca", &mut self.rca),
            ("type-layer", &mut self.type_layer),
            ("mention-layer", &mut self.mention_layer),
            ("knn-layer", &mut self.knn_layer),
        ]
    }

    /// `default`, or changes to it: a comma list of `+name` and `-name`.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut out = Techniques::default();
        if spec.trim() == "default" || spec.trim().is_empty() {
            return Ok(out);
        }
        for part in spec.split(',').map(str::trim) {
            let (on, name) = match part.split_at_checked(1) {
                Some(("+", n)) => (true, n),
                Some(("-", n)) => (false, n),
                _ => return Err(format!("{part}: expected +name or -name")),
            };
            let mut flags = out.flags();
            let flag = flags
                .iter_mut()
                .find(|(n, _)| *n == name)
                .ok_or_else(|| format!("{name}: no such technique"))?;
            *flag.1 = on;
        }
        if !out.communities && (out.type_layer || out.mention_layer || out.knn_layer) {
            return Err("a community layer needs communities".to_owned());
        }
        Ok(out)
    }

    /// The canonical changes from the default (`+rca,-knn`), or `None` for the default.
    pub fn label(&self) -> Option<String> {
        let mut default = Techniques::default();
        let mut this = *self;
        let changes: Vec<String> = this
            .flags()
            .into_iter()
            .zip(default.flags())
            .filter(|((_, a), (_, b))| **a != **b)
            .map(|((n, a), _)| format!("{}{n}", if *a { "+" } else { "-" }))
            .collect();
        (!changes.is_empty()).then(|| changes.join(","))
    }
}

impl std::fmt::Debug for Analysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Analysis")
            .field("config", &self.config)
            .field(
                "embedder",
                &self.embedder.as_ref().map(|e| e.spec().model.clone()),
            )
            .finish()
    }
}

/// The `lctx-compiler` run of one attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerRun {
    pub run_id: Id,
    pub producer_id: Id,
}

impl CompilerRun {
    pub(crate) fn model(&self, surface: &str) -> String {
        format!("{}/{surface}", self.producer_id.hex())
    }
}

/// The compiler run and producer rows for the library release and context (the extractor run
/// that declares `exports`). The run declares no families (ADR-0019).
pub fn compiler_rows(
    snapshot_id: Id,
    release_id: Id,
    context_id: Id,
    config_digest: Digest,
) -> (CompilerRun, RunsRow, ProducersRow) {
    let compiler = crate::attempt::compiler_digest();
    let revision = compiler.hex();
    let producer_id = recipe::producer(TOOL, &revision, compiler);
    let run_id = recipe::run(release_id, context_id, producer_id, &[], config_digest);
    (
        CompilerRun {
            run_id,
            producer_id,
        },
        RunsRow {
            snapshot_id,
            run_id,
            release_id,
            context_id,
            producer_id,
            families: Vec::new(),
            config_digest,
        },
        ProducersRow {
            snapshot_id,
            producer_id,
            tool: TOOL.to_owned(),
            revision,
            build_digest: compiler,
        },
    )
}

/// The rows Stage E produces, per analysis table, and the seeds it resolved (access path and
/// declaration, in config order) for Stage F.
#[derive(Debug, Default)]
pub struct AnalysisRows {
    pub seeds: Vec<(String, Id)>,
    pub invocations: Vec<AnalysisInvocationsRow>,
    pub findings: Vec<FindingsRow>,
    pub members: Vec<FindingMembersRow>,
    pub witnesses: Vec<WitnessesRow>,
    /// E0's embedding keys (slice 3.1): the snapshot's key set includes them.
    pub embedded_keys: Vec<cpg_schema::id::Digest>,
    /// Each seed's FCA scope: its node and label (the increment-2 review's F1). Stage F states
    /// only that scope's concepts and implications for the seed.
    pub seed_scopes: BTreeMap<Id, (Id, String)>,
}

async fn collect(
    ctx: &SessionContext,
    query: &str,
    schema: &SchemaRef,
) -> Result<Vec<RecordBatch>, CoreError> {
    sql::query(ctx, query)
        .await?
        .collect()
        .await?
        .iter()
        .map(|b| to_schema(b, schema))
        .collect()
}

fn hex_list(ids: impl IntoIterator<Item = Id>) -> String {
    ids.into_iter()
        .map(|i| format!("X'{}'", i.hex()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn quoted(names: impl IntoIterator<Item = String>) -> String {
    // Every name was checked to be a dotted Python identifier (the config refuses anything else).
    names
        .into_iter()
        .map(|n| format!("'{n}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn id_col(batch: &RecordBatch, name: &str) -> Result<Vec<Id>, CoreError> {
    let a = batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
        .ok_or_else(|| CoreError::Analysis(format!("column {name}")))?;
    (0..a.len())
        .map(|i| {
            <[u8; 16]>::try_from(a.value(i))
                .map(Id)
                .map_err(|_| CoreError::Analysis(format!("column {name}")))
        })
        .collect()
}

fn str_col<'a>(batch: &'a RecordBatch, name: &str) -> Result<&'a StringArray, CoreError> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| CoreError::Analysis(format!("column {name}")))
}

fn schema(fields: &[(&str, DataType)]) -> SchemaRef {
    std::sync::Arc::new(Schema::new(
        fields
            .iter()
            .map(|(n, t)| Field::new(*n, t.clone(), true))
            .collect::<Vec<_>>(),
    ))
}

/// One export row: an access path naming a declaration.
struct ExportRow {
    access_path: String,
    declaration: Id,
    is_stub: bool,
}

/// A seed resolved to its declaration and the access paths that name it (DESIGN §9.1 step 1):
/// the longest exported prefix of the seed's path names a declaration, and each remaining name is
/// that declaration's member, found in its own body or else along its MRO. Among several `def`s
/// of one name, the exports seed rank decides: an implementation before an `@overload` stub, then
/// the one Pysa describes, then the last in source order.
async fn resolve_seeds(
    ctx: &SessionContext,
    seeds: &[String],
) -> Result<BTreeMap<String, Seed>, CoreError> {
    let prefixes: Vec<String> = seeds
        .iter()
        .flat_map(|s| {
            let parts: Vec<&str> = s.split('.').collect();
            (1..=parts.len())
                .map(|k| parts[..k].join("."))
                .collect::<Vec<_>>()
        })
        .collect();
    let export_schema = schema(&[
        ("access_path", DataType::Utf8),
        ("declaration_node_id", DataType::FixedSizeBinary(16)),
        ("is_stub", DataType::Boolean),
    ]);
    let exports_sql = format!(
        "SELECT e.access_path, e.declaration_node_id, s.is_stub \
         FROM exports e JOIN declarations d ON d.node_id = e.declaration_node_id \
         JOIN source_files s ON s.module_node_id = d.module_node_id \
         WHERE e.access_path IN ({}) \
         ORDER BY e.access_path, s.is_stub, e.declaration_node_id",
        quoted(prefixes)
    );
    let mut exports: Vec<ExportRow> = Vec::new();
    for b in collect(ctx, &exports_sql, &export_schema).await? {
        let path = str_col(&b, "access_path")?;
        let decl = id_col(&b, "declaration_node_id")?;
        let stub = b
            .column_by_name("is_stub")
            .and_then(|c| c.as_any().downcast_ref::<BooleanArray>())
            .ok_or_else(|| CoreError::Analysis("column is_stub".to_owned()))?;
        for (i, declaration) in decl.into_iter().enumerate() {
            exports.push(ExportRow {
                access_path: path.value(i).to_owned(),
                declaration,
                is_stub: stub.value(i),
            });
        }
    }

    let mut out = BTreeMap::new();
    for seed in seeds {
        let parts: Vec<&str> = seed.split('.').collect();
        let (k, container) = (1..=parts.len())
            .rev()
            .find_map(|k| {
                let prefix = parts[..k].join(".");
                exports
                    .iter()
                    .filter(|e| e.access_path == prefix)
                    .min_by_key(|e| e.is_stub)
                    .map(|e| (k, e.declaration))
            })
            .ok_or_else(|| CoreError::Analysis(format!("seed {seed}: no exported prefix")))?;
        let mut node = container;
        for name in &parts[k..] {
            node = member(ctx, node, name)
                .await?
                .ok_or_else(|| CoreError::Analysis(format!("seed {seed}: no member {name}")))?;
        }
        // The access paths naming the exported container, each extended by the rest of the path.
        let rest: String = parts[k..].iter().map(|p| format!(".{p}")).collect();
        let mut aliases: BTreeMap<String, Id> = BTreeMap::new();
        let alias_sql = format!(
            "SELECT access_path, export_node_id FROM exports WHERE declaration_node_id = X'{}' \
             ORDER BY access_path, export_node_id",
            container.hex()
        );
        let alias_schema = schema(&[
            ("access_path", DataType::Utf8),
            ("export_node_id", DataType::FixedSizeBinary(16)),
        ]);
        for b in collect(ctx, &alias_sql, &alias_schema).await? {
            let path = str_col(&b, "access_path")?;
            let node = id_col(&b, "export_node_id")?;
            for (i, export) in node.into_iter().enumerate() {
                aliases
                    .entry(format!("{}{rest}", path.value(i)))
                    .or_insert(export);
            }
        }
        out.insert(
            seed.clone(),
            Seed {
                node,
                aliases: aliases.into_iter().map(|(p, n)| (n, p)).collect(),
            },
        );
    }
    Ok(out)
}

/// A declaration's member of a name (DESIGN §9.1 step 1): its own `def` or `class` by the seed
/// rank, else, for a class, the first ancestor along its MRO that declares it. The walk **refuses**
/// rather than guesses (slice 1.4 review F2) when an ancestor it would pass is outside the release
/// or unresolved (the name may be defined there), or when a class along the way binds the name by
/// anything but a `def` or `class` (an assignment such as `tool = helper`).
async fn member(ctx: &SessionContext, owner: Id, name: &str) -> Result<Option<Id>, CoreError> {
    let refuse = |why: String| Err(CoreError::Analysis(format!("member {name}: {why}")));
    let mro_schema = schema(&[
        ("ancestor_node_id", DataType::FixedSizeBinary(16)),
        ("ordinal", DataType::Int64),
    ]);
    let mro_sql = format!(
        "SELECT t.ancestor_node_id, a.ordinal FROM ancestry_targets t \
         JOIN class_ancestry a ON a.fact_id = t.ancestry_fact_id \
         WHERE t.class_node_id = X'{}' AND a.relation = {} \
         ORDER BY a.ordinal, t.ancestor_node_id",
        owner.hex(),
        cpg_schema::codebook::AncestryRelation::Mro.code()
    );
    let mut chain: Vec<Option<Id>> = vec![Some(owner)];
    for b in collect(ctx, &mro_sql, &mro_schema).await? {
        let a = b
            .column_by_name("ancestor_node_id")
            .and_then(|c| c.as_any().downcast_ref::<FixedSizeBinaryArray>())
            .ok_or_else(|| CoreError::Analysis("column ancestor_node_id".to_owned()))?;
        for i in 0..a.len() {
            chain.push(
                (!a.is_null(i))
                    .then(|| <[u8; 16]>::try_from(a.value(i)).map(Id))
                    .transpose()
                    .map_err(|_| CoreError::Analysis("column ancestor_node_id".to_owned()))?,
            );
        }
    }
    chain.dedup();
    let known: Vec<Id> = chain.iter().flatten().copied().collect();
    let decl_schema = schema(&[
        ("node_id", DataType::FixedSizeBinary(16)),
        ("parent_node_id", DataType::FixedSizeBinary(16)),
        ("rank", DataType::Int64),
    ]);
    let decl_sql = format!(
        "SELECT d.node_id, d.parent_node_id, \
                CAST(CASE WHEN d.is_overload THEN 0 ELSE 2 END \
                   + CASE WHEN m.node_id IS NULL THEN 0 ELSE 1 END AS BIGINT) AS rank \
         FROM declarations d LEFT JOIN provider_node_map m ON m.node_id = d.node_id \
         WHERE d.parent_node_id IN ({}) AND d.name = '{name}' \
         ORDER BY d.parent_node_id, rank DESC, d.start_byte DESC, d.node_id",
        hex_list(known.iter().copied())
    );
    let mut best: BTreeMap<Id, Id> = BTreeMap::new();
    for b in collect(ctx, &decl_sql, &decl_schema).await? {
        let node = id_col(&b, "node_id")?;
        let parent = id_col(&b, "parent_node_id")?;
        for (i, n) in node.into_iter().enumerate() {
            // Rows arrive best-first per parent; keep the first.
            best.entry(parent[i]).or_insert(n);
        }
    }
    // Which chain entries are release classes, and which classes bind the name otherwise.
    let class_sql = format!(
        "SELECT node_id FROM declarations WHERE node_id IN ({}) AND kind = {}",
        hex_list(known.iter().copied()),
        cpg_schema::codebook::DeclarationKind::Class.code()
    );
    let mut classes = std::collections::BTreeSet::new();
    for b in collect(
        ctx,
        &class_sql,
        &schema(&[("node_id", DataType::FixedSizeBinary(16))]),
    )
    .await?
    {
        classes.extend(id_col(&b, "node_id")?);
    }
    let bound_sql = format!(
        "SELECT s.owner_node_id FROM bindings b JOIN scopes s ON s.node_id = b.scope_id \
         WHERE s.owner_node_id IN ({}) AND s.kind = {} AND b.name = '{name}' \
           AND b.kind NOT IN ({}, {}, {})",
        hex_list(known.iter().copied()),
        cpg_schema::codebook::LexicalScopeKind::Class.code(),
        cpg_schema::codebook::BindingKind::FunctionDef.code(),
        cpg_schema::codebook::BindingKind::ClassDef.code(),
        cpg_schema::codebook::BindingKind::AnnotationOnly.code()
    );
    let mut rebound = std::collections::BTreeSet::new();
    for b in collect(
        ctx,
        &bound_sql,
        &schema(&[("owner_node_id", DataType::FixedSizeBinary(16))]),
    )
    .await?
    {
        rebound.extend(id_col(&b, "owner_node_id")?);
    }
    for (at, entry) in chain.iter().enumerate() {
        let Some(c) = *entry else {
            return refuse("an unresolved base precedes any definition of it".to_owned());
        };
        if rebound.contains(&c) {
            return refuse(format!(
                "{} binds it by an assignment or import, not a def",
                c.hex()
            ));
        }
        if let Some(found) = best.get(&c) {
            return Ok(Some(*found));
        }
        // A module or function owner has no MRO; an ancestor outside the release may define it.
        if at > 0 && !classes.contains(&c) {
            return refuse(format!(
                "ancestor {} is outside the analyzed release and may define it",
                c.hex()
            ));
        }
    }
    Ok(None)
}

/// Pass A's parameters, as recorded on each invocation (fixed field order): every config field
/// the method reads (ADR-0019 review O3).
#[derive(Serialize)]
struct PassAParameters<'a> {
    seed: &'a str,
    max_depth: u32,
    max_vertices: u32,
    max_edges: u32,
    max_witnesses: u32,
    module_prefixes: &'a [String],
    public_roots: &'a [String],
}

#[derive(Serialize)]
struct PassCParameters<'a> {
    seed: &'a str,
    max_occurrences: u32,
}

#[derive(Serialize)]
struct KnnParameters<'a> {
    knn: &'a neighbours::Params,
    /// The embedding spec whose vectors the search reads.
    spec_hash: String,
}

#[derive(Serialize)]
struct SelectionParameters<'a> {
    budget: u32,
    choices: &'a selection::Params,
    ranked_by: &'a str,
}

#[derive(Serialize)]
struct SelectionDiagnostics<'a> {
    configured: &'a [String],
    eligible: usize,
    selected: &'a [String],
    /// Selected APIs whose preferred path resolves to another declaration (the increment-2
    /// review's O1): recorded, not replaced.
    dropped: &'a [String],
}

#[derive(Serialize)]
struct UsageParameters<'a> {
    policy: &'a str,
    module_prefixes: &'a [String],
    public_roots: &'a [String],
}

#[derive(Serialize)]
struct ConceptParameters<'a> {
    fca: &'a concepts::Params,
    scope: &'a str,
}

#[derive(Serialize)]
struct RankingParameters<'a> {
    pagerank: &'a ranking::Params,
    weight_policy: &'a str,
    module_prefixes: &'a [String],
    public_roots: &'a [String],
}

#[derive(Serialize)]
struct CommunityParameters<'a> {
    communities: &'a communities::Params,
    module_prefixes: &'a [String],
    public_roots: &'a [String],
}

#[derive(Serialize)]
struct PassBParameters<'a> {
    seed: &'a str,
    max_depth: u32,
    module_prefixes: &'a [String],
    public_roots: &'a [String],
}

/// Each seed's parameters, the receiver aside (`cpg_schema::flows::receivers_sql`: decided by
/// the method's kind, never by the parameter's name; slice 2.1 review F8).
async fn seed_parameters(
    ctx: &SessionContext,
    seeds: &[Id],
) -> Result<BTreeMap<Id, Vec<SeedParameter>>, CoreError> {
    let batches = collect(
        ctx,
        &format!(
            "SELECT ps.function_node_id, ps.node_id, ps.name FROM parameter_syntax ps \
             LEFT ANTI JOIN ({receivers}) r ON r.parameter_node_id = ps.node_id \
             WHERE ps.function_node_id IN ({seeds}) \
             ORDER BY ps.function_node_id, ps.ordinal",
            receivers = cpg_schema::flows::receivers_sql(),
            seeds = hex_list(seeds.iter().copied())
        ),
        &schema(&[
            ("function_node_id", DataType::FixedSizeBinary(16)),
            ("node_id", DataType::FixedSizeBinary(16)),
            ("name", DataType::Utf8),
        ]),
    )
    .await?;
    let mut out: BTreeMap<Id, Vec<SeedParameter>> = BTreeMap::new();
    for batch in &batches {
        let functions = id_col(batch, "function_node_id")?;
        let nodes = id_col(batch, "node_id")?;
        let names = str_col(batch, "name")?;
        for i in 0..batch.num_rows() {
            out.entry(functions[i]).or_default().push(SeedParameter {
                node: nodes[i],
                name: names.value(i).to_owned(),
            });
        }
    }
    Ok(out)
}

/// Build a declared projection from the session.
pub async fn project(ctx: &SessionContext, spec: &ProjectionSpec) -> Result<Projection, CoreError> {
    let vertices = collect(ctx, &spec.vertices_sql, &schemas::vertices()).await?;
    let arcs = collect(ctx, &spec.arcs_sql, &schemas::arcs()).await?;
    let unresolved = collect(ctx, &spec.unresolved_sql, &schemas::unresolved()).await?;
    Projection::build(&vertices, &arcs, &unresolved).map_err(|e| CoreError::Analysis(e.to_string()))
}

/// Stage E for one attempt: Pass A from every seed of the config.
pub async fn run(
    ctx: &SessionContext,
    root: &std::path::Path,
    snapshot_id: Id,
    analysis: &Analysis,
    compiler: CompilerRun,
) -> Result<AnalysisRows, CoreError> {
    let config = &analysis.config;
    let spec = projection::invocation();
    let projection_digest = spec.digest();
    let p = project(ctx, &spec).await?;
    let subsystem = p.mask(|i| {
        matches!(
            p.kinds[i],
            NodeKind::Function | NodeKind::Class | NodeKind::Module
        ) && p.roles[i] == Some(SourceRole::Release)
            && p.modules[i]
                .as_deref()
                .is_some_and(|m| config.in_subsystem(m))
    });
    let mut rows = AnalysisRows::default();
    // Communities (§9.4): Leiden over the subsystem's invocation and co-use layers, at the
    // pre-registered resolutions and seeds; the consensus's communities cite it.
    let community_digest = cpg_schema::communities::digest();
    let public = collect(
        ctx,
        &cpg_schema::communities::public_callables_sql(&config.subsystem.public_roots),
        &cpg_schema::communities::schemas::public_callables(),
    )
    .await?;
    let mut paths: Vec<(Id, String)> = Vec::new();
    for b in &public {
        let nodes = id_col(b, "node_id")?;
        let access = str_col(b, "access_path")?;
        for (i, node) in nodes.into_iter().enumerate() {
            paths.push((node, access.value(i).to_owned()));
        }
    }
    // One name per public callable (the increment-2 review's F4).
    let preferred = lctx_analytics::communities::preferred_paths(&public)
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let techniques = analysis.techniques;
    if techniques.communities {
        let input = communities::Input::build(
            &p,
            &subsystem,
            &collect(
                ctx,
                &cpg_schema::communities::co_use_sql(),
                &cpg_schema::communities::schemas::co_use(),
            )
            .await?,
            &public,
        )
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let params = communities::Params::preregistered();
        let parameters = serde_json::to_string(&CommunityParameters {
            communities: &params,
            module_prefixes: &config.subsystem.module_prefixes,
            public_roots: &config.subsystem.public_roots,
        })
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let parameters_digest = content_digest(parameters.as_bytes());
        let consensus_id = findings::invocation(
            AnalyticMethod::CommunityConsensus.code(),
            parameters_digest,
            Some(community_digest),
            None,
            None,
        );
        let outcome = communities::run(&input, &params, snapshot_id, consensus_id)
            .map_err(|e| CoreError::Analysis(e.to_string()))?;
        for run in &outcome.runs {
            let parameters = format!(
                "{{\"consensus\":\"{}\",\"run\":{}}}",
                parameters_digest.hex(),
                run.parameters
            );
            let run_digest = content_digest(parameters.as_bytes());
            rows.invocations.push(AnalysisInvocationsRow {
                snapshot_id,
                invocation_id: findings::invocation(
                    AnalyticMethod::Leiden.code(),
                    run_digest,
                    Some(community_digest),
                    None,
                    Some(run.seed as i64),
                ),
                run_id: compiler.run_id,
                model_id: compiler.model("leiden"),
                extraction_mode: ExtractionMode::GraphAnalysis,
                method: AnalyticMethod::Leiden,
                parameters,
                parameters_digest: run_digest,
                projection_digest: Some(community_digest),
                library_versions: lctx_analytics::libraries(),
                subject_node_id: None,
                seed: Some(run.seed as i64),
                iterations: Some(run.iterations),
                residual: None,
                converged: Some(run.converged),
                quality_history: run.quality_history.clone(),
                candidate_set_size: Some(outcome.vertices as i64),
                vertices_examined: Some(outcome.vertices as i64),
                arcs_examined: Some(outcome.pairs as i64),
                completion: if run.converged {
                    CoverageStatus::CompleteUnderStatedModel
                } else {
                    CoverageStatus::Partial
                },
                stop_reason: None,
                diagnostics: None,
            });
        }
        rows.invocations.push(AnalysisInvocationsRow {
            snapshot_id,
            invocation_id: consensus_id,
            run_id: compiler.run_id,
            model_id: compiler.model("community-consensus"),
            extraction_mode: ExtractionMode::GraphAnalysis,
            method: AnalyticMethod::CommunityConsensus,
            parameters,
            parameters_digest,
            projection_digest: Some(community_digest),
            library_versions: lctx_analytics::libraries(),
            subject_node_id: None,
            seed: None,
            iterations: None,
            residual: None,
            converged: None,
            quality_history: Vec::new(),
            candidate_set_size: Some(outcome.vertices as i64),
            vertices_examined: Some(outcome.vertices as i64),
            arcs_examined: Some(outcome.pairs as i64),
            completion: outcome.completion,
            stop_reason: None,
            diagnostics: Some(outcome.diagnostics),
        });
        rows.findings.extend(outcome.findings);
        rows.members.extend(outcome.members);
    }
    // Direct usage (§9.5; the increment-2 review's U1): each public API's definite calls from the
    // official usage code.
    let usage_digest = ranking::usage_digest(spec.digest());
    let usage_parameters = serde_json::to_string(&UsageParameters {
        policy: ranking::USAGE_POLICY,
        module_prefixes: &config.subsystem.module_prefixes,
        public_roots: &config.subsystem.public_roots,
    })
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let usage_parameters_digest = content_digest(usage_parameters.as_bytes());
    let usage_invocation = findings::invocation(
        AnalyticMethod::UsageCount.code(),
        usage_parameters_digest,
        Some(usage_digest),
        None,
        None,
    );
    let usage = ranking::run_usage(&p, &subsystem, &public, snapshot_id, usage_invocation)
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
    rows.invocations.push(AnalysisInvocationsRow {
        snapshot_id,
        invocation_id: usage_invocation,
        run_id: compiler.run_id,
        model_id: compiler.model("usage-count"),
        extraction_mode: ExtractionMode::GraphAnalysis,
        method: AnalyticMethod::UsageCount,
        parameters: usage_parameters,
        parameters_digest: usage_parameters_digest,
        projection_digest: Some(usage_digest),
        library_versions: lctx_analytics::libraries(),
        subject_node_id: None,
        seed: None,
        iterations: None,
        residual: None,
        converged: None,
        quality_history: Vec::new(),
        candidate_set_size: Some(usage.counts.len() as i64),
        vertices_examined: None,
        arcs_examined: Some(p.arcs.len() as i64),
        completion: CoverageStatus::CompleteUnderStatedModel,
        stop_reason: None,
        diagnostics: Some(usage.diagnostics.clone()),
    });
    rows.findings.extend(usage.findings.iter().cloned());
    if techniques.pagerank {
        // Centrality (§9.5): PageRank over the usage projection; each public API's rank.
        let usage_digest = ranking::projection_digest(spec.digest());
        let rank_params = ranking::Params::preregistered();
        let parameters = serde_json::to_string(&RankingParameters {
            pagerank: &rank_params,
            weight_policy: ranking::WEIGHT_POLICY,
            module_prefixes: &config.subsystem.module_prefixes,
            public_roots: &config.subsystem.public_roots,
        })
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let parameters_digest = content_digest(parameters.as_bytes());
        let invocation_id = findings::invocation(
            AnalyticMethod::PageRank.code(),
            parameters_digest,
            Some(usage_digest),
            None,
            None,
        );
        let ranked = ranking::run(
            &ranking::UsageGraph::build(&p, &subsystem),
            &public,
            &rank_params,
            snapshot_id,
            invocation_id,
        )
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        rows.invocations.push(AnalysisInvocationsRow {
            snapshot_id,
            invocation_id,
            run_id: compiler.run_id,
            model_id: compiler.model("pagerank"),
            extraction_mode: ExtractionMode::GraphAnalysis,
            method: AnalyticMethod::PageRank,
            parameters,
            parameters_digest,
            projection_digest: Some(usage_digest),
            library_versions: lctx_analytics::libraries(),
            subject_node_id: None,
            seed: None,
            iterations: Some(ranked.ranks.iterations),
            residual: Some(ranked.ranks.residual),
            converged: Some(ranked.ranks.converged),
            quality_history: Vec::new(),
            candidate_set_size: Some(ranked.vertices as i64),
            vertices_examined: Some(ranked.vertices as i64),
            arcs_examined: Some(ranked.arcs as i64),
            completion: ranked.completion,
            stop_reason: None,
            diagnostics: Some(ranked.diagnostics),
        });
        rows.findings.extend(ranked.findings);
    }
    // Embeddings in analytics (§9.7; slice 3.1): E0 embeds the corpus passages and the subsystem's
    // public APIs through the cache, in windows under the document cap; exact kNN links each API
    // to its nearest passages and labels each community by its centroid's nearest heading.
    if let (true, Some(embedder)) = (techniques.knn, &analysis.embedder) {
        let knn = neighbours::Params::preregistered();
        let mut texts: Vec<String> = Vec::new();
        // (node, label, first window, window count)
        let mut passage_spans: Vec<(Id, String, usize, usize)> = Vec::new();
        for b in collect(
            ctx,
            &cpg_schema::neighbours::passages_sql(),
            &cpg_schema::neighbours::schemas::passages(),
        )
        .await?
        {
            let nodes = id_col(&b, "node_id")?;
            let heading = str_col(&b, "heading")?;
            let path = str_col(&b, "path")?;
            let text = str_col(&b, "text")?;
            for (i, node) in nodes.into_iter().enumerate() {
                let label = if heading.is_null(i) {
                    path.value(i).to_owned()
                } else {
                    format!("{} § {}", path.value(i), heading.value(i))
                };
                let w = neighbours::windows(text.value(i), knn.window_bytes);
                passage_spans.push((node, label, texts.len(), w.len()));
                texts.extend(w);
            }
        }
        let api_nodes: Vec<Id> = preferred
            .keys()
            .copied()
            .filter(|n| {
                p.dense(*n).is_some_and(|d| {
                    p.kinds[d as usize] == NodeKind::Function && subsystem.contains(d as usize)
                })
            })
            .collect();
        let mut api_spans: Vec<(Id, usize, usize)> = Vec::new();
        if !api_nodes.is_empty() {
            for b in collect(
                ctx,
                &cpg_schema::neighbours::api_texts_sql(&api_nodes),
                &cpg_schema::neighbours::schemas::api_texts(),
            )
            .await?
            {
                let nodes = id_col(&b, "node_id")?;
                let docstring = str_col(&b, "docstring")?;
                let parameters = str_col(&b, "parameters")?;
                for (i, node) in nodes.into_iter().enumerate() {
                    let text = neighbours::api_text(
                        &preferred[&node],
                        (!parameters.is_null(i)).then(|| parameters.value(i)),
                        (!docstring.is_null(i)).then(|| docstring.value(i)),
                    );
                    let w = neighbours::windows(&text, knn.window_bytes);
                    api_spans.push((node, texts.len(), w.len()));
                    texts.extend(w);
                }
            }
        }
        let (vectors, keys, _) =
            crate::embed::embed_texts(root, snapshot_id, embedder.as_ref(), &texts).await?;
        rows.embedded_keys = keys;
        let item = |node: Id, first: usize, count: usize| neighbours::Item {
            node,
            vectors: vectors[first..first + count].to_vec(),
        };
        let apis: Vec<neighbours::Item> = api_spans
            .iter()
            .map(|&(n, first, count)| item(n, first, count))
            .collect();
        let passages: Vec<neighbours::Passage> = passage_spans
            .iter()
            .map(|(n, label, first, count)| neighbours::Passage {
                item: item(*n, *first, *count),
                label: label.clone(),
            })
            .collect();
        let groups: Vec<(Id, Vec<Id>)> = rows
            .findings
            .iter()
            .filter(|f| f.finding_kind == FindingKind::Community)
            .map(|f| {
                (
                    f.subject_node_id,
                    rows.members
                        .iter()
                        .filter(|m| {
                            m.finding_id == f.finding_id && m.role == MemberRole::CommunityMember
                        })
                        .filter_map(|m| m.node_id)
                        .collect(),
                )
            })
            .collect();
        let parameters = serde_json::to_string(&KnnParameters {
            knn: &knn,
            spec_hash: embedder.spec().hash().hex(),
        })
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let parameters_digest = content_digest(parameters.as_bytes());
        let knn_digest = cpg_schema::neighbours::digest();
        let invocation_id = findings::invocation(
            AnalyticMethod::Knn.code(),
            parameters_digest,
            Some(knn_digest),
            None,
            None,
        );
        let found = neighbours::run(&apis, &passages, &groups, &knn, snapshot_id, invocation_id)
            .map_err(|e| CoreError::Analysis(e.to_string()))?;
        rows.invocations.push(AnalysisInvocationsRow {
            snapshot_id,
            invocation_id,
            run_id: compiler.run_id,
            model_id: compiler.model("knn"),
            extraction_mode: ExtractionMode::GraphAnalysis,
            method: AnalyticMethod::Knn,
            parameters,
            parameters_digest,
            projection_digest: Some(knn_digest),
            library_versions: lctx_analytics::libraries(),
            subject_node_id: None,
            seed: None,
            iterations: None,
            residual: None,
            converged: None,
            quality_history: Vec::new(),
            candidate_set_size: Some(found.candidate_pairs as i64),
            vertices_examined: Some((apis.len() + passages.len()) as i64),
            arcs_examined: None,
            completion: neighbours::COMPLETION,
            stop_reason: None,
            diagnostics: Some(found.diagnostics),
        });
        rows.findings.extend(found.findings);
        rows.members.extend(found.members);
    }
    // Seed selection (§9.4, §9.5; the increment-2 review's U1): the configured seeds, then, while
    // the brief budget allows, the eligible public APIs by rank (direct usage, or PageRank in its
    // variant), each community capped at its share (`lctx_analytics::selection`).
    let choices = selection::Params::preregistered();
    let configured = config.seeds();
    let configured_nodes: Vec<Id> = {
        let resolved = resolve_seeds(ctx, &configured).await?;
        configured.iter().map(|n| resolved[n].node).collect()
    };
    let community_of: BTreeMap<Id, Id> = rows
        .findings
        .iter()
        .filter(|f| f.finding_kind == FindingKind::Community)
        .flat_map(|f| {
            rows.members
                .iter()
                .filter(move |m| {
                    m.finding_id == f.finding_id && m.role == MemberRole::CommunityMember
                })
                .filter_map(move |m| m.node_id.map(|n| (n, f.finding_id)))
        })
        .collect();
    // Eligible: a public API official usage calls directly whose docstring has a summary, the
    // Outcome its brief will state (`synth::summary_span`; the increment-2 review's O1).
    let called: Vec<Id> = usage.counts.keys().copied().collect();
    let mut eligible: Vec<Id> = Vec::new();
    if !called.is_empty() {
        for b in collect(
            ctx,
            &format!(
                "SELECT d.node_id, d.docstring_start_byte, d.docstring_end_byte, s.text \
                 FROM declarations d JOIN source_files s ON s.module_node_id = d.module_node_id \
                 WHERE d.docstring_start_byte IS NOT NULL AND d.node_id IN ({}) \
                 ORDER BY d.node_id",
                hex_list(called.iter().copied())
            ),
            &schema(&[
                ("node_id", DataType::FixedSizeBinary(16)),
                ("docstring_start_byte", DataType::Int64),
                ("docstring_end_byte", DataType::Int64),
                ("text", DataType::Utf8),
            ]),
        )
        .await?
        {
            let nodes = id_col(&b, "node_id")?;
            let text = str_col(&b, "text")?;
            let int = |name: &str| {
                b.column_by_name(name)
                    .and_then(|c| c.as_any().downcast_ref::<arrow_array::Int64Array>())
                    .ok_or_else(|| CoreError::Analysis(format!("column {name}")))
            };
            let (start, end) = (int("docstring_start_byte")?, int("docstring_end_byte")?);
            for (i, node) in nodes.into_iter().enumerate() {
                if !text.is_null(i)
                    && crate::synth::summary_span(
                        text.value(i),
                        start.value(i) as usize,
                        end.value(i) as usize,
                    )
                    .is_some()
                {
                    eligible.push(node);
                }
            }
        }
    }
    let (rank, ranked_by): (BTreeMap<Id, f64>, &str) = if techniques.pagerank {
        (
            rows.findings
                .iter()
                .filter(|f| f.finding_kind == FindingKind::Centrality)
                .filter_map(|f| f.score.map(|s| (f.subject_node_id, s)))
                .collect(),
            "pagerank",
        )
    } else {
        (usage.counts.clone(), "direct_usage")
    };
    let selected: Vec<Id> = selection::select(
        &configured_nodes,
        config.briefs.budget as usize,
        &eligible,
        &rank,
        &community_of,
        &choices,
    );
    let mut seeds = configured.clone();
    let named: Vec<String> = selected
        .iter()
        .filter_map(|n| preferred.get(n).cloned())
        .collect();
    let resolved_chosen = if named.is_empty() {
        BTreeMap::new()
    } else {
        resolve_seeds(ctx, &named).await?
    };
    // A path already chosen (a property's getter and setter share one) or resolving to another
    // declaration is dropped and recorded, never a second seed.
    let (mut chosen, mut dropped): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for n in named {
        if selected.contains(&resolved_chosen[&n].node) && !chosen.contains(&n) {
            chosen.push(n);
        } else {
            dropped.push(n);
        }
    }
    seeds.extend(chosen.iter().cloned());
    let selection_parameters = serde_json::to_string(&SelectionParameters {
        budget: config.briefs.budget,
        choices: &choices,
        ranked_by,
    })
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let selection_digest = content_digest(selection_parameters.as_bytes());
    rows.invocations.push(AnalysisInvocationsRow {
        snapshot_id,
        invocation_id: findings::invocation(
            AnalyticMethod::SeedSelection.code(),
            selection_digest,
            None,
            None,
            None,
        ),
        run_id: compiler.run_id,
        model_id: compiler.model("seed-selection"),
        extraction_mode: ExtractionMode::GraphAnalysis,
        method: AnalyticMethod::SeedSelection,
        parameters: selection_parameters,
        parameters_digest: selection_digest,
        projection_digest: None,
        library_versions: lctx_analytics::libraries(),
        subject_node_id: None,
        seed: None,
        iterations: None,
        residual: None,
        converged: None,
        quality_history: Vec::new(),
        candidate_set_size: Some(eligible.len() as i64),
        vertices_examined: None,
        arcs_examined: None,
        completion: CoverageStatus::CompleteUnderStatedModel,
        stop_reason: None,
        diagnostics: Some(
            serde_json::to_string(&SelectionDiagnostics {
                configured: &configured,
                eligible: eligible.len(),
                selected: &chosen,
                dropped: &dropped,
            })
            .map_err(|e| CoreError::Analysis(e.to_string()))?,
        ),
    });
    let resolved = resolve_seeds(ctx, &seeds).await?;
    // Two seeds naming one declaration would be one subject twice (ADR-0019 review F1): the
    // config is wrong, so the attempt stops and says which.
    let mut by_node: BTreeMap<Id, &str> = BTreeMap::new();
    for name in &seeds {
        if let Some(other) = by_node.insert(resolved[name].node, name) {
            return Err(CoreError::Analysis(format!(
                "seeds {other} and {name} name one declaration"
            )));
        }
    }
    let budgets = Budgets {
        max_depth: config.pass_a.max_depth,
        max_vertices: config.pass_a.max_vertices,
        max_edges: config.pass_a.max_edges,
        max_witnesses: config.pass_a.max_witnesses,
    };
    for name in &seeds {
        let seed = &resolved[name];
        let parameters = serde_json::to_string(&PassAParameters {
            seed: name,
            max_depth: budgets.max_depth,
            max_vertices: budgets.max_vertices,
            max_edges: budgets.max_edges,
            max_witnesses: budgets.max_witnesses,
            module_prefixes: &config.subsystem.module_prefixes,
            public_roots: &config.subsystem.public_roots,
        })
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let parameters_digest = content_digest(parameters.as_bytes());
        let invocation_id = findings::invocation(
            AnalyticMethod::PassABfs.code(),
            parameters_digest,
            Some(projection_digest),
            Some(seed.node),
            None,
        );
        let result = pass_a::run(&p, seed, &subsystem, budgets, snapshot_id, invocation_id)
            .map_err(|e| CoreError::Analysis(e.to_string()))?;
        rows.invocations.push(AnalysisInvocationsRow {
            snapshot_id,
            invocation_id,
            run_id: compiler.run_id,
            model_id: compiler.model("pass-a"),
            extraction_mode: ExtractionMode::GraphAnalysis,
            method: AnalyticMethod::PassABfs,
            parameters,
            parameters_digest,
            projection_digest: Some(projection_digest),
            library_versions: lctx_analytics::libraries(),
            subject_node_id: Some(seed.node),
            seed: None,
            iterations: None,
            residual: None,
            converged: None,
            quality_history: Vec::new(),
            candidate_set_size: None,
            vertices_examined: Some(result.vertices_examined),
            arcs_examined: Some(result.arcs_examined),
            completion: result.completion,
            stop_reason: result.stop_reason,
            diagnostics: None,
        });
        rows.seeds.push((name.clone(), seed.node));
        rows.findings.extend(result.findings);
        rows.members.extend(result.members);
        rows.witnesses.extend(result.witnesses);
    }
    // Pass B (§9.2) over the declared flows and guards, from every seed.
    let flows_digest = cpg_schema::flows::digest();
    let flow_rows = collect(
        ctx,
        &cpg_schema::flows::argument_flows_sql(),
        &cpg_schema::flows::schemas::flows(),
    )
    .await?;
    let guard_rows = collect(
        ctx,
        &cpg_schema::flows::guards_sql(),
        &cpg_schema::flows::schemas::guards(),
    )
    .await?;
    let read_rows = collect(
        ctx,
        &cpg_schema::flows::parameter_reads_sql(),
        &cpg_schema::flows::schemas::parameter_reads(),
    )
    .await?;
    let flows = Flows::build(&flow_rows, &guard_rows, &read_rows)
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let parameters_of =
        seed_parameters(ctx, &rows.seeds.iter().map(|s| s.1).collect::<Vec<_>>()).await?;
    let inside = |n: Id| p.dense(n).is_some_and(|i| subsystem.contains(i as usize));
    for (name, node) in rows.seeds.clone() {
        let parameters = serde_json::to_string(&PassBParameters {
            seed: &name,
            max_depth: budgets.max_depth,
            module_prefixes: &config.subsystem.module_prefixes,
            public_roots: &config.subsystem.public_roots,
        })
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let parameters_digest = content_digest(parameters.as_bytes());
        let invocation_id = findings::invocation(
            AnalyticMethod::PassBFlows.code(),
            parameters_digest,
            Some(flows_digest),
            Some(node),
            None,
        );
        let result = pass_b::run(
            &flows,
            node,
            parameters_of
                .get(&node)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            inside,
            budgets.max_depth,
            snapshot_id,
            invocation_id,
        )
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        rows.invocations.push(AnalysisInvocationsRow {
            snapshot_id,
            invocation_id,
            run_id: compiler.run_id,
            model_id: compiler.model("pass-b"),
            extraction_mode: ExtractionMode::GraphAnalysis,
            method: AnalyticMethod::PassBFlows,
            parameters,
            parameters_digest,
            projection_digest: Some(flows_digest),
            library_versions: lctx_analytics::libraries(),
            subject_node_id: Some(node),
            seed: None,
            iterations: None,
            residual: None,
            converged: None,
            quality_history: Vec::new(),
            candidate_set_size: None,
            vertices_examined: Some(result.states_examined),
            arcs_examined: Some(result.flows_examined),
            completion: result.completion,
            stop_reason: result.stop_reason,
            diagnostics: None,
        });
        rows.findings.extend(result.findings);
        rows.members.extend(result.members);
        rows.witnesses.extend(result.witnesses);
    }
    // Pass C (§9.3): handoffs in the official usage code, from every seed.
    let handoffs = Handoffs::build(
        &collect(
            ctx,
            &cpg_schema::flows::handoffs_sql(),
            &cpg_schema::flows::schemas::handoffs(),
        )
        .await?,
    )
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    for (name, node) in rows.seeds.clone() {
        let parameters = serde_json::to_string(&PassCParameters {
            seed: &name,
            max_occurrences: budgets.max_witnesses,
        })
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let parameters_digest = content_digest(parameters.as_bytes());
        let invocation_id = findings::invocation(
            AnalyticMethod::PassCHandoffs.code(),
            parameters_digest,
            Some(flows_digest),
            Some(node),
            None,
        );
        let result = pass_c::run(
            &handoffs,
            node,
            budgets.max_witnesses as usize,
            snapshot_id,
            invocation_id,
        )
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        rows.invocations.push(AnalysisInvocationsRow {
            snapshot_id,
            invocation_id,
            run_id: compiler.run_id,
            model_id: compiler.model("pass-c"),
            extraction_mode: ExtractionMode::GraphAnalysis,
            method: AnalyticMethod::PassCHandoffs,
            parameters,
            parameters_digest,
            projection_digest: Some(flows_digest),
            library_versions: lctx_analytics::libraries(),
            subject_node_id: Some(node),
            seed: None,
            iterations: None,
            residual: None,
            converged: None,
            quality_history: Vec::new(),
            candidate_set_size: Some(result.occurrences),
            vertices_examined: None,
            arcs_examined: None,
            completion: result.completion,
            stop_reason: None,
            diagnostics: None,
        });
        rows.findings.extend(result.findings);
        rows.members.extend(result.members);
    }
    // Concepts (§9.6): FCA of each seed's structural scope, the public APIs of the exported class
    // or module namespace that its access path names (`fastmcp.FastMCP` for
    // `fastmcp.FastMCP.tool`). A scope is its node (the increment-2 review's F1): paths naming
    // one class are one scope, labelled by the one with the fewest segments, then the least.
    let containers: std::collections::BTreeSet<String> = rows
        .seeds
        .iter()
        .filter_map(|(name, _)| name.rsplit_once('.').map(|(c, _)| c.to_owned()))
        .collect();
    let mut scope_nodes: BTreeMap<String, Id> = BTreeMap::new();
    for b in collect(
        ctx,
        &format!(
            "SELECT access_path, declaration_node_id FROM exports WHERE access_path IN ({}) \
             ORDER BY access_path, declaration_node_id",
            quoted(containers.iter().cloned())
        ),
        &schema(&[
            ("access_path", DataType::Utf8),
            ("declaration_node_id", DataType::FixedSizeBinary(16)),
        ]),
    )
    .await?
    {
        let access = str_col(&b, "access_path")?;
        for (i, node) in id_col(&b, "declaration_node_id")?.into_iter().enumerate() {
            scope_nodes
                .entry(access.value(i).to_owned())
                .or_insert(node);
        }
    }
    // A namespace no export names (a package's own path) is its module.
    for b in collect(
        ctx,
        &format!(
            "SELECT module_name AS access_path, module_node_id AS declaration_node_id \
             FROM source_files WHERE module_name IN ({}) ORDER BY module_name, module_node_id",
            quoted(containers.iter().cloned())
        ),
        &schema(&[
            ("access_path", DataType::Utf8),
            ("declaration_node_id", DataType::FixedSizeBinary(16)),
        ]),
    )
    .await?
    {
        let access = str_col(&b, "access_path")?;
        for (i, node) in id_col(&b, "declaration_node_id")?.into_iter().enumerate() {
            scope_nodes
                .entry(access.value(i).to_owned())
                .or_insert(node);
        }
    }
    let mut names: BTreeMap<Id, std::collections::BTreeSet<String>> = BTreeMap::new();
    for (container, node) in &scope_nodes {
        names.entry(*node).or_default().insert(container.clone());
    }
    let mut scopes: Vec<concepts::Scope> = Vec::new();
    for (node, names) in &names {
        let label = names
            .iter()
            .min_by_key(|n| (n.matches('.').count(), n.as_str()))
            .expect("a scope has a name")
            .clone();
        // Each object by its path under the label, else its least under another of the names.
        let mut objects: BTreeMap<Id, String> = BTreeMap::new();
        for under_label in [true, false] {
            for (id, path) in &paths {
                if let Some((c, _)) = path.rsplit_once('.')
                    && names.contains(c)
                    && (c == label) == under_label
                {
                    objects.entry(*id).or_insert_with(|| path.clone());
                }
            }
        }
        if objects.len() >= 2 {
            scopes.push(concepts::Scope {
                node: *node,
                label,
                objects: objects.into_iter().collect(),
            });
        }
    }
    for (name, seed) in &rows.seeds {
        let scope = name
            .rsplit_once('.')
            .and_then(|(c, _)| scope_nodes.get(c))
            .and_then(|n| scopes.iter().find(|s| s.node == *n));
        if let Some(scope) = scope.filter(|_| techniques.fca) {
            rows.seed_scopes
                .insert(*seed, (scope.node, scope.label.clone()));
        }
    }
    let all: Vec<Id> = scopes
        .iter()
        .flat_map(|s| s.objects.iter().map(|(id, _)| *id))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let attributes = concepts::attributes_of(
        &collect(
            ctx,
            &cpg_schema::concepts::attributes_sql(&all),
            &cpg_schema::concepts::schemas::attributes(),
        )
        .await?,
    )
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let fca_params = concepts::Params::preregistered();
    let fca_digest = cpg_schema::concepts::digest();
    for scope in scopes.iter().filter(|_| techniques.fca) {
        let parameters = serde_json::to_string(&ConceptParameters {
            fca: &fca_params,
            scope: &scope.label,
        })
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let parameters_digest = content_digest(parameters.as_bytes());
        let invocation_id = findings::invocation(
            AnalyticMethod::Fca.code(),
            parameters_digest,
            Some(fca_digest),
            Some(scope.node),
            None,
        );
        let result = concepts::run(scope, &attributes, &fca_params, snapshot_id, invocation_id)
            .map_err(|e| CoreError::Analysis(e.to_string()))?;
        rows.invocations.push(AnalysisInvocationsRow {
            snapshot_id,
            invocation_id,
            run_id: compiler.run_id,
            model_id: compiler.model("fca"),
            extraction_mode: ExtractionMode::GraphAnalysis,
            method: AnalyticMethod::Fca,
            parameters,
            parameters_digest,
            projection_digest: Some(fca_digest),
            library_versions: lctx_analytics::libraries(),
            subject_node_id: Some(scope.node),
            seed: None,
            iterations: None,
            residual: None,
            converged: None,
            quality_history: Vec::new(),
            candidate_set_size: Some(result.objects as i64),
            vertices_examined: Some(result.examined as i64),
            arcs_examined: Some(result.attributes as i64),
            completion: result.completion,
            stop_reason: result.stop_reason,
            diagnostics: Some(result.diagnostics),
        });
        rows.findings.extend(result.findings);
        rows.members.extend(result.members);
    }
    // Two seeds may reach the same finding only with different subjects, so ids are unique; the
    // key rules check it.
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::Techniques;

    #[test]
    fn a_variant_is_the_default_changed_by_name_and_labelled_canonically() {
        assert_eq!(Techniques::parse("default"), Ok(Techniques::default()));
        assert_eq!(Techniques::default().label(), None);
        let v = Techniques::parse("+rca, -knn,+type-layer").unwrap();
        assert!(v.rca && !v.knn && v.type_layer && v.fca);
        // The label is in declaration order, whatever the spelling's order.
        assert_eq!(v.label().as_deref(), Some("-knn,+rca,+type-layer"));
        assert_eq!(Techniques::parse(&v.label().unwrap()), Ok(v));
        // Setting a technique to its default is no change.
        assert_eq!(Techniques::parse("+fca").unwrap().label(), None);
        assert!(Techniques::parse("knn").is_err());
        assert!(Techniques::parse("-louvain").is_err());
        assert!(Techniques::parse("-communities,+knn-layer").is_err());
    }
}
