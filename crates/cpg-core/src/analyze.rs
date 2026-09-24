//! Stage E (DESIGN §4.1, §5, §9; ADR-0019): run the declared projections on the attempt's
//! session, hand their batches to `lctx-analytics`, and return its rows for the attempt to write.
//!
//! Analysis results are a run of producer `lctx-compiler` over the library release and its
//! context. Its `runs` and `producers` rows are made here, before any raw table is written, so
//! `content_digest` includes the analytics config through the run's config digest.

use std::collections::BTreeMap;

use arrow_array::{Array, FixedSizeBinaryArray, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use cpg_schema::codebook::{
    AnalyticMethod, Codebook, CoverageStatus, DeclarationKind, ExtractionMode, FindingKind,
    MemberRole, NodeKind, SourceRole,
};
use cpg_schema::communities::LayerSpec;
use cpg_schema::findings::{
    AnalysisInvocationsRow, FindingMembersRow, FindingsRow, PublicPathsRow, WitnessesRow,
    recipe as findings,
};
use cpg_schema::id::{Digest, Id, IdHasher, content_digest, recipe};
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
/// kept set: since the keep rule (ADR-0020), Passes A–C, direct usage and selection only, every
/// technique here off. A variant adds or removes techniques by name (`+communities,+fca`). The
/// whole set, not its difference from the default, joins the compiler run's config digest
/// ([`variant_config_digest`]; the ADR-0020 review's F3), so two technique sets never share a run
/// or a content digest, whatever the default. Finding and assertion ids do not depend on it, so
/// ablation diffs are joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "kebab-case")]
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
        // RCA adds attributes to FCA's context; without FCA it does nothing (ADR-0020 review F8).
        if out.rca && !out.fca {
            return Err("rca needs fca".to_owned());
        }
        Ok(out)
    }

    /// The techniques that are on, in declaration order (`communities,fca`), or `none`.
    pub fn label(&self) -> String {
        let mut this = *self;
        let on: Vec<&str> = this
            .flags()
            .into_iter()
            .filter(|(_, on)| **on)
            .map(|(n, _)| n)
            .collect();
        if on.is_empty() {
            "none".to_owned()
        } else {
            on.join(",")
        }
    }

    /// The canonical JSON of the whole set (field order is declaration order).
    pub fn json(&self) -> String {
        serde_json::to_string(self).expect("techniques serialize")
    }
}

/// The compiler run's config digest: the analytics config's digest with the whole technique set
/// (the ADR-0020 review's F3). Never the bare config digest.
pub fn variant_config_digest(config: Digest, techniques: &Techniques) -> Digest {
    IdHasher::new("analytics-techniques")
        .digest_field(config)
        .str(&techniques.json())
        .finish_digest()
}

impl std::fmt::Debug for Analysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Analysis")
            .field("config", &self.config)
            .field(
                "embedder",
                &self.embedder.as_ref().map(|e| e.spec().model.clone()),
            )
            .field("techniques", &self.techniques.label())
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
    /// Each seed's FCA attributes, as the context held them (RCA's included).
    pub seed_attributes: BTreeMap<Id, std::collections::BTreeSet<String>>,
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

/// Each seed resolved to its declaration and the access paths that name it (DESIGN §9.1 step 1):
/// its `public_paths` row (the holistic assessment's A1). The paths naming it through the same
/// container, the same exported class (or, for a direct export, the module level), are its
/// aliases, each with the export it extends, sorted by path. A name with no row is refused, never
/// guessed: it is private or unexported, or `public_paths` refuses it (an unresolved or
/// outside-release base before its definition, or a class binding it by an assignment).
fn resolve_seeds(
    public: &[PublicPathsRow],
    seeds: &[String],
) -> Result<BTreeMap<String, Seed>, CoreError> {
    let by_path: BTreeMap<&str, &PublicPathsRow> =
        public.iter().map(|r| (r.access_path.as_str(), r)).collect();
    let container = |path: &str| {
        path.rsplit_once('.')
            .and_then(|(c, _)| by_path.get(c))
            .filter(|r| r.kind == DeclarationKind::Class)
            .map(|r| r.node_id)
    };
    let mut out = BTreeMap::new();
    for seed in seeds {
        let row = by_path.get(seed.as_str()).ok_or_else(|| {
            CoreError::Analysis(format!(
                "seed {seed}: not a public path (private or unexported; or a member past an \
                 unresolved or outside-release base, or bound by an assignment)"
            ))
        })?;
        let owner = container(seed);
        let aliases: BTreeMap<&str, Id> = public
            .iter()
            .filter(|r| r.node_id == row.node_id && container(&r.access_path) == owner)
            .map(|r| (r.access_path.as_str(), r.export_node_id))
            .collect();
        out.insert(
            seed.clone(),
            Seed {
                node: row.node_id,
                aliases: aliases
                    .into_iter()
                    .map(|(p, n)| (n, p.to_owned()))
                    .collect(),
            },
        );
    }
    Ok(out)
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
    /// The whole technique set, so every snapshot records which techniques made it.
    techniques: &'a Techniques,
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
    /// RCA's relational scaling (slice 3.2, `+rca`); absent in the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    rca: Option<&'a str>,
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
    /// A variant's extra layers and their weight rule (slice 3.2); absent in the default, so its
    /// parameters, and so its invocation ids, are unchanged.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extra_layers: Vec<(&'a str, &'a str)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight_rule: Option<&'a str>,
    /// The kNN layer's lineage (the holistic assessment's A2(c)): the embedding spec and the
    /// search parameters behind its pairs. Absent without that layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    knn_layer: Option<KnnLayerLineage>,
}

#[derive(Serialize)]
struct KnnLayerLineage {
    spec_hash: String,
    k: usize,
    min_similarity: f64,
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
    public: &[PublicPathsRow],
    stages: &mut cpg_schema::metrics::Stages,
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
    stages.mark("analyze: projection");
    // Communities (§9.4): Leiden over the subsystem's invocation and co-use layers, at the
    // pre-registered resolutions and seeds; the consensus's communities cite it.
    let community_digest = cpg_schema::communities::digest();
    // Every public callable's paths (the holistic assessment's A1: `public_paths`, written before
    // Stage E), and the one name each is shown by (the increment-2 review's F4).
    let paths: Vec<(Id, String)> = public
        .iter()
        .filter(|r| cpg_schema::public::CALLABLE_KINDS.contains(&r.kind))
        .map(|r| (r.node_id, r.access_path.clone()))
        .collect();
    let preferred = cpg_schema::public::preferred_callables(public).map_err(CoreError::Analysis)?;
    let techniques = analysis.techniques;
    stages.mark("analyze: public callables");
    // E0 (§9.7; slice 3.1): the corpus passages and the subsystem's public APIs embedded through
    // the cache, in windows under the document cap, when kNN or the kNN layer reads them.
    let knn = neighbours::Params::preregistered();
    if techniques.knn_layer && analysis.embedder.is_none() {
        return Err(CoreError::Analysis(
            "the kNN community layer needs an embedder".to_owned(),
        ));
    }
    let mut embedded = None;
    if let (true, Some(embedder)) = (techniques.knn || techniques.knn_layer, &analysis.embedder) {
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
        embedded = Some((embedder, apis, passages));
    }
    stages.mark("analyze: embed (E0)");
    if techniques.communities {
        // A variant's extra layers (slice 3.2; §9.4): shared declared parameter types, doc
        // co-mentions (C5 O1) and API–API embedding neighbours.
        let mut extra: Vec<communities::ExtraLayer> = Vec::new();
        for (on, spec, policy, sql) in [
            (
                techniques.type_layer,
                LayerSpec::Type,
                communities::TYPE_LAYER_POLICY,
                cpg_schema::communities::shared_types_sql(),
            ),
            (
                techniques.mention_layer,
                LayerSpec::Mention,
                communities::MENTION_LAYER_POLICY,
                cpg_schema::communities::co_mention_sql(),
            ),
        ] {
            if on {
                let rows = collect(
                    ctx,
                    &sql,
                    &cpg_schema::communities::schemas::scope_targets(),
                )
                .await?;
                extra.push(communities::ExtraLayer {
                    spec,
                    policy,
                    pairs: communities::scope_pairs(&rows, |_| true)
                        .map_err(|e| CoreError::Analysis(e.to_string()))?,
                });
            }
        }
        if let (true, Some((embedder, apis, _))) = (techniques.knn_layer, &embedded) {
            extra.push(communities::ExtraLayer {
                spec: LayerSpec::Knn {
                    spec_hash: embedder.spec().hash().hex(),
                    k: knn.k,
                    min_similarity: knn.min_similarity,
                },
                policy: communities::KNN_LAYER_POLICY,
                pairs: neighbours::api_neighbours(apis, knn.k, knn.min_similarity)
                    .into_iter()
                    .map(|(a, b)| (a, b, a.max(b)))
                    .collect(),
            });
        }
        let input = communities::Input::build(
            &p,
            &subsystem,
            &collect(
                ctx,
                &cpg_schema::communities::co_use_sql(),
                &cpg_schema::communities::schemas::co_use(),
            )
            .await?,
            &preferred,
            &extra,
        )
        .map_err(|e| CoreError::Analysis(e.to_string()))?;
        let layers: Vec<LayerSpec> = extra.iter().map(|l| l.spec.clone()).collect();
        let community_digest = if extra.is_empty() {
            community_digest
        } else {
            cpg_schema::communities::extra_digest(community_digest, &layers)
        };
        let params = communities::Params::preregistered();
        let parameters = serde_json::to_string(&CommunityParameters {
            communities: &params,
            module_prefixes: &config.subsystem.module_prefixes,
            public_roots: &config.subsystem.public_roots,
            extra_layers: extra.iter().map(|l| (l.spec.name(), l.policy)).collect(),
            weight_rule: (!extra.is_empty()).then_some(communities::EXTRA_WEIGHT_RULE),
            knn_layer: layers.iter().find_map(|l| match l {
                LayerSpec::Knn {
                    spec_hash,
                    k,
                    min_similarity,
                } => Some(KnnLayerLineage {
                    spec_hash: spec_hash.clone(),
                    k: *k,
                    min_similarity: *min_similarity,
                }),
                LayerSpec::Type | LayerSpec::Mention => None,
            }),
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
    stages.mark("analyze: communities");
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
    let usage = ranking::run_usage(&p, &subsystem, &preferred, snapshot_id, usage_invocation)
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
    stages.mark("analyze: direct usage");
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
            &preferred,
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
    stages.mark("analyze: pagerank");
    // kNN (§9.7; slice 3.1): exact kNN links each API to its nearest passages and labels each
    // community by its centroid's nearest heading.
    if let (true, Some((embedder, apis, passages))) = (techniques.knn, &embedded) {
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
        let found = neighbours::run(apis, passages, &groups, &knn, snapshot_id, invocation_id)
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
    stages.mark("analyze: kNN");
    // Seed selection (§9.4, §9.5; the increment-2 review's U1): the configured seeds, then, while
    // the brief budget allows, the eligible public APIs by rank (direct usage, or PageRank in its
    // variant), each community capped at its share (`lctx_analytics::selection`).
    let choices = selection::Params::preregistered();
    let configured = config.seeds();
    let configured_nodes: Vec<Id> = {
        let resolved = resolve_seeds(public, &configured)?;
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
        resolve_seeds(public, &named)?
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
        techniques: &techniques,
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
    let resolved = resolve_seeds(public, &seeds)?;
    stages.mark("analyze: seed selection");
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
    stages.mark("analyze: Pass A");
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
    stages.mark("analyze: Pass B");
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
    stages.mark("analyze: Pass C");
    // Concepts (§9.6): FCA of each seed's structural scope, the public APIs of the exported class
    // or module namespace that its access path names (`fastmcp.FastMCP` for
    // `fastmcp.FastMCP.tool`). A scope is its node (the increment-2 review's F1): paths naming
    // one class are one scope, labelled by the one with the fewest segments, then the least.
    let containers: std::collections::BTreeSet<String> = rows
        .seeds
        .iter()
        .filter_map(|(name, _)| name.rsplit_once('.').map(|(c, _)| c.to_owned()))
        .collect();
    // An exported class is its public-path row (the holistic assessment's A1).
    let mut scope_nodes: BTreeMap<String, Id> = public
        .iter()
        .filter(|r| r.kind == DeclarationKind::Class && containers.contains(&r.access_path))
        .map(|r| (r.access_path.clone(), r.node_id))
        .collect();
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
    let mut attributes = concepts::attributes_of(
        &collect(
            ctx,
            &cpg_schema::concepts::attributes_sql(&all),
            &cpg_schema::concepts::schemas::attributes(),
        )
        .await?,
    )
    .map_err(|e| CoreError::Analysis(e.to_string()))?;
    let fca_params = concepts::Params::preregistered();
    let mut fca_digest = cpg_schema::concepts::digest();
    // RCA (§9.6; slice 3.2, `+rca`): each object's calls into the subsystem and the handoffs
    // official usage shows, as attributes of the same FCA.
    if techniques.rca {
        let wanted: std::collections::BTreeSet<Id> = all.iter().copied().collect();
        let calls: Vec<(Id, Id)> = p
            .arcs
            .iter()
            .filter(|a| {
                a.arc_kind == cpg_schema::codebook::ArcKind::Call
                    && wanted.contains(&p.ids[a.src as usize])
                    && p.kinds[a.dst as usize] == NodeKind::Function
                    && subsystem.contains(a.dst as usize)
            })
            .map(|a| (p.ids[a.src as usize], p.ids[a.dst as usize]))
            .collect();
        let passed: Vec<(Id, Id)> = handoffs
            .rows
            .iter()
            .map(|h| (h.producer, h.consumer))
            .collect();
        let partners: std::collections::BTreeSet<Id> = calls
            .iter()
            .map(|c| c.1)
            .chain(passed.iter().flat_map(|(a, b)| [*a, *b]))
            .collect();
        let mut names: BTreeMap<Id, String> = partners
            .iter()
            .filter_map(|n| preferred.get(n).map(|path| (*n, path.clone())))
            .collect();
        let unnamed: Vec<Id> = partners
            .iter()
            .copied()
            .filter(|n| !names.contains_key(n))
            .collect();
        if !unnamed.is_empty() {
            for b in collect(
                ctx,
                &format!(
                    "SELECT node_id, qualified_name FROM declarations WHERE node_id IN ({}) \
                     ORDER BY node_id",
                    hex_list(unnamed.iter().copied())
                ),
                &schema(&[
                    ("node_id", DataType::FixedSizeBinary(16)),
                    ("qualified_name", DataType::Utf8),
                ]),
            )
            .await?
            {
                let name = str_col(&b, "qualified_name")?;
                for (i, node) in id_col(&b, "node_id")?.into_iter().enumerate() {
                    names.insert(node, name.value(i).to_owned());
                }
            }
        }
        for (node, extra) in concepts::relational(&all, &calls, &passed, &names) {
            attributes.entry(node).or_default().extend(extra);
        }
        fca_digest = cpg_schema::id::IdHasher::new("concept-attributes-rca")
            .digest_field(fca_digest)
            .str(concepts::RCA_POLICY)
            .finish_digest();
    }
    if techniques.fca {
        for (_, seed) in &rows.seeds {
            if let Some(own) = attributes.get(seed) {
                rows.seed_attributes.insert(*seed, own.clone());
            }
        }
    }
    for scope in scopes.iter().filter(|_| techniques.fca) {
        let parameters = serde_json::to_string(&ConceptParameters {
            fca: &fca_params,
            scope: &scope.label,
            rca: techniques.rca.then_some(concepts::RCA_POLICY),
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
    stages.mark("analyze: concepts");
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::Techniques;

    #[test]
    fn every_technique_set_has_its_own_config_digest() {
        use cpg_schema::id::content_digest;
        let config = content_digest(b"config");
        let mut seen = std::collections::BTreeSet::new();
        for bits in 0u16..256 {
            let mut t = Techniques::default();
            for (i, (_, flag)) in t.flags().into_iter().enumerate() {
                *flag = bits >> i & 1 == 1;
            }
            let d = super::variant_config_digest(config, &t);
            assert_ne!(d, config, "{}", t.label());
            assert!(seen.insert(d), "{} collides", t.label());
        }
        assert_eq!(seen.len(), 256);
    }

    #[test]
    fn a_variant_is_the_default_changed_by_name_and_labelled_canonically() {
        assert_eq!(Techniques::parse("default"), Ok(Techniques::default()));
        assert_eq!(Techniques::default().label(), "none");
        let v = Techniques::parse("+fca, +rca, +communities,+type-layer").unwrap();
        assert!(v.rca && v.communities && v.type_layer && v.fca && !v.knn);
        // The label names what is on, in declaration order, whatever the spelling's order.
        assert_eq!(v.label(), "communities,fca,rca,type-layer");
        let respelled: Vec<String> = v.label().split(',').map(|n| format!("+{n}")).collect();
        assert_eq!(Techniques::parse(&respelled.join(",")), Ok(v));
        // Setting a technique to its default is no change.
        assert_eq!(Techniques::parse("-fca").unwrap(), Techniques::default());
        assert!(Techniques::parse("knn").is_err());
        assert!(Techniques::parse("-louvain").is_err());
        assert!(Techniques::parse("+knn-layer").is_err());
        // RCA without FCA would do nothing (ADR-0020 review F8).
        assert!(Techniques::parse("+rca").is_err());
        assert_eq!(
            v.json().matches(':').count(),
            8,
            "every technique, on or off"
        );
    }
}
