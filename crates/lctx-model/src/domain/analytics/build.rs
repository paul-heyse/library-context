//! Canonical conversions shared by producer and stored replay.
// Increment for a meaning/rule change; implementation source bytes live in producer provenance.
const SEMANTIC_RULE_REVISION: i64 = 1;
use super::*;
use crate::domain::{
    analysis::{self, analytic as owner, settings::AnalyticsConfiguration},
    normalized::{Rows, binding_normalization::BindingData, entities::*},
    projection::snapshot::MaterializedGraph,
    resources::ResourceBudget,
    *,
};
use petgraph::visit::{EdgeRef, IntoEdgeReferences, IntoNodeIdentifiers};
use std::collections::{BTreeMap, BTreeSet};
pub(super) fn invalid(s: impl Into<String>) -> ModelError {
    ModelError::Invalid(s.into())
}
pub(super) fn need<R: Record>(rows: &Rows<R>, id: Id<R>) -> Result<&R, ModelError> {
    rows.get(id)
        .ok_or_else(|| invalid(format!("analytic predecessor absent: {}", R::NAME)))
}
pub const METHODS: [analysis::AnalysisMethod; 5] = [
    analysis::AnalysisMethod::PageRank,
    analysis::AnalysisMethod::Communities,
    analysis::AnalysisMethod::Concepts,
    analysis::AnalysisMethod::RelationalConcepts,
    analysis::AnalysisMethod::Neighbours,
];
pub const MAX_WORK: u64 = policy::RETAINED.max_work;
pub fn selected(s: &AnalyticsConfiguration, m: analysis::AnalysisMethod) -> bool {
    match m {
        analysis::AnalysisMethod::PageRank => s.pagerank,
        analysis::AnalysisMethod::Communities => s.communities,
        analysis::AnalysisMethod::Concepts => s.fca,
        analysis::AnalysisMethod::RelationalConcepts => s.rca,
        analysis::AnalysisMethod::Neighbours => s.knn,
        _ => false,
    }
}
pub fn capability(m: analysis::AnalysisMethod) -> Result<analysis::AnalysisCapability, ModelError> {
    Ok(match m {
        analysis::AnalysisMethod::PageRank => analysis::AnalysisCapability::PageRank,
        analysis::AnalysisMethod::Communities => analysis::AnalysisCapability::Communities,
        analysis::AnalysisMethod::Concepts => analysis::AnalysisCapability::Concepts,
        analysis::AnalysisMethod::RelationalConcepts => {
            analysis::AnalysisCapability::RelationalConcepts
        }
        analysis::AnalysisMethod::Neighbours => analysis::AnalysisCapability::Neighbours,
        _ => return Err(invalid("unknown analytic method")),
    })
}
pub fn definition(
    settings: &AnalyticsConfiguration,
    method: analysis::AnalysisMethod,
) -> Result<(analysis::MethodParameters, analysis::AnalysisDefinition), ModelError> {
    definition_with_policy(settings, method, policy::RETAINED)
}
/// Select only the attribute projection; graph mechanics and weights retain their existing policy.
pub fn definition_with_attribute_policy(
    settings: &AnalyticsConfiguration,
    method: analysis::AnalysisMethod,
    attributes: policy::AttributePolicy,
) -> Result<(analysis::MethodParameters, analysis::AnalysisDefinition), ModelError> {
    definition_with_policy(
        settings,
        method,
        policy::RetainedPolicy {
            attributes,
            ..policy::RETAINED
        },
    )
}
pub fn definition_with_policy(
    settings: &AnalyticsConfiguration,
    method: analysis::AnalysisMethod,
    policy: policy::RetainedPolicy,
) -> Result<(analysis::MethodParameters, analysis::AnalysisDefinition), ModelError> {
    settings.validate()?;
    capability(method)?;
    let numeric = matches!(
        method,
        analysis::AnalysisMethod::PageRank
            | analysis::AnalysisMethod::Communities
            | analysis::AnalysisMethod::Neighbours
    );
    let p = policy.parameters(method)?;
    let mut key = KeySink::new("analytic-model-v1");
    settings.id().encode(&mut key);
    method.encode(&mut key);
    p.id().encode(&mut key);
    analysis::ProjectionDefinition::builtin(projection::ProjectionName::CallableInvocation)
        .id()
        .encode(&mut key);
    policy.recipe()?.encode(&mut key);
    SEMANTIC_RULE_REVISION.encode(&mut key);
    Ok((
        p.clone(),
        analysis::AnalysisDefinition {
            method,
            parameters: p.id(),
            semantic_version: key.finish(),
            interpretation: if numeric {
                analysis::Interpretation::Heuristic
            } else {
                analysis::Interpretation::ExactUnderContext
            },
        },
    ))
}
#[macro_export]
macro_rules! analytic_extra_inputs {
    ($m:ident) => {
        $m! {
         members:$crate::domain::catalog::CatalogMember,
         uses:$crate::domain::input::ArtifactUse,
         corpus_libraries:$crate::domain::input::CorpusLibrary,
         parameter_syntax:$crate::domain::syntax::ParameterSyntaxObservation,
         type_supports:$crate::domain::types::TypeSupport,
         class_metadata:$crate::domain::class_metadata::ClassMetadataObservation,
         metadata_supports:$crate::domain::class_metadata::ClassMetadataSupport,
         class_members:$crate::domain::class_metadata::ClassMemberObservation,
         member_supports:$crate::domain::class_metadata::ClassMemberSupport,
         record_options:$crate::domain::class_metadata::RecordOptions,
         native_signature_supports:$crate::domain::types::NativeSignatureSupport,
         port_supports:$crate::domain::types::SignatureTypeSupport,
         captures:$crate::domain::captures::CaptureObservation,
         capture_supports:$crate::domain::captures::CaptureSupport,
         exits:$crate::domain::protocols::NativeExitObservation,
         exit_supports:$crate::domain::protocols::NativeExitSupport,
         terminals:$crate::domain::protocols::NativeTerminalObservation,
         terminal_supports:$crate::domain::protocols::NativeTerminalSupport,
         type_sequences:$crate::domain::types::TypeSequence,
         type_members:$crate::domain::types::TypeSequenceMember,
         callable_type_slots:$crate::domain::types::CallableParameter,
         text_subjects:$crate::domain::embedding::text::TextSubject,
         settings:$crate::domain::analysis::settings::AnalyticsConfiguration,
         definitions:$crate::domain::analysis::AnalysisDefinition,
         parameters:$crate::domain::analysis::MethodParameters,
         embedding_invocations:$crate::domain::analysis::analytic_embedding::Invocation,
         embedding_outcomes:$crate::domain::analysis::analytic_embedding::AnalysisOutcome,
         embedding_uses:$crate::domain::embedding::analytic::AnalysisEmbeddingUse,
         structural_invocations:$crate::domain::analysis::structural::Invocation,
        }
    };
}
macro_rules! data{($($f:ident:$t:ty,)*)=>{
 pub struct Data{pub native:BindingData,pub structural:structural::Output,pub vectors:embedding::analytic::ConsumptionData,pub graphs:projection::normalization::ProjectionOutput,$(pub $f:Rows<$t>,)*}
 impl Data{pub fn new(b:&ResourceBudget)->Self{Self{native:BindingData::new(b),structural:structural::Output::new(b),vectors:embedding::analytic::ConsumptionData::new(b),graphs:projection::normalization::ProjectionOutput::new(b),$($f:Rows::new(b),)*}}
 pub fn visit(&mut self,n:&str,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{let a=self.native.visit(n,b)?;let c=self.structural.visit(n,b)?;let d=self.vectors.visit(n,b)?;let e=self.graphs.visit(n,b)?;$(if n==<$t>::NAME{self.$f.decode(b)?;return Ok(true);})*Ok(a||c||d||e)}
 pub fn validation_inputs()->Vec<ValidationInput>{let mut v=BindingData::validation_inputs();v.extend(structural::Output::validation_inputs().into_iter().map(|input|if stages::is_vocabulary(input.name()){input.at_epoch(stages::PublicationBoundary::Structural)}else{input}));v.extend(projection::normalization::ProjectionOutput::validation_inputs());v.extend(vec![$(ValidationInput::of::<$t>(&["id"]),)*]);v.extend(vector_inputs());v.sort_by_key(|i|(i.name(),i.prefix()));v.dedup_by_key(|i|(i.name(),i.prefix()));v}
 pub fn configuration(&self)->Result<&AnalyticsConfiguration,ModelError>{let mut i=self.settings.iter();let s=i.next().ok_or_else(||invalid("analytic selected settings absent"))?;if i.next().is_some(){return Err(invalid("analytic selected settings ambiguous"));}s.validate()?;Ok(s)}
 }
};}
crate::analytic_extra_inputs!(data);
impl Data {
    pub fn consumed_inputs(profile: stages::Profile) -> Vec<ValidationInput> {
        let mut inputs = Self::validation_inputs();
        inputs.retain(|i| {
            ![
                projection::ProjectionSnapshot::NAME,
                projection::ProjectionSnapshotChunk::NAME,
            ]
            .contains(&i.name())
                && (profile != stages::Profile::Catalog
                    || ![
                        flow::FlowValuePathObservation::NAME,
                        flow::FlowCallStep::NAME,
                        flow::FlowTestLeafObservation::NAME,
                    ]
                    .contains(&i.name()))
        });
        for method in METHODS {
            inputs.extend(analysis::expected::inputs(method));
        }
        inputs
    }
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if stages::is_vocabulary(input.name()) {
            match input.prefix() {
                Some(stages::PublicationBoundary::Facts) => {
                    self.native.visit(input.name(), batch)?;
                }
                Some(stages::PublicationBoundary::Structural) => {
                    self.structural.visit(input.name(), batch)?;
                }
                _ => {
                    return Err(invalid(format!(
                        "analytic input {} changes its completed vocabulary view",
                        input.name()
                    )));
                }
            }
        } else {
            self.visit(input.name(), batch)?;
        }
        Ok(())
    }
}

pub fn digest<R: Key>(label: &str, rows: &[R]) -> ContentHash {
    let mut key = KeySink::new(label);
    for row in rows {
        row.encode(&mut key);
    }
    (rows.len() as i64).encode(&mut key);
    key.finish()
}
fn result(
    frame: &AnalyticFrame,
    invocation: &owner::Invocation,
    method: analysis::AnalysisMethod,
    s: &AnalyticsConfiguration,
) -> TechniqueResult {
    let selected = selected(s, method);
    TechniqueResult {
        frame: frame.id(),
        method,
        invocation: invocation.id(),
        selected,
        status: if selected {
            analysis::AnalysisStatus::Completed
        } else {
            analysis::AnalysisStatus::NotRequested
        },
        stop: if selected {
            Stop::Converged
        } else {
            Stop::NotRequested
        },
        iterations: 0,
        examined: 0,
        residual: None,
        input_partial: false,
    }
}
fn contribute(
    out: &mut Output,
    frame: Id<AnalyticFrame>,
    layer: Layer,
    left: Id<EntityRef>,
    right: Id<EntityRef>,
    source: PairSource,
) -> Result<(), ModelError> {
    if left == right {
        return Ok(());
    }
    let source = out.pair_sources.insert(source)?;
    out.contributions.insert(PairContribution {
        frame,
        layer,
        source,
        left: left.min(right),
        right: left.max(right),
    })?;
    Ok(())
}
pub fn produce(
    d: &Data,
    f: &AnalyticFrame,
    invocations: &Rows<owner::Invocation>,
    graph: &MaterializedGraph,
    b: &ResourceBudget,
) -> Result<Output, ModelError> {
    produce_with_policy(d, f, invocations, graph, b, policy::RETAINED.attributes)
}
/// Actual optional attribute projection over the same source callable universe and graph frame.
pub fn produce_with_policy(
    d: &Data,
    f: &AnalyticFrame,
    invocations: &Rows<owner::Invocation>,
    graph: &MaterializedGraph,
    b: &ResourceBudget,
    attributes: policy::AttributePolicy,
) -> Result<Output, ModelError> {
    let s = d.configuration()?;
    let sf = need(&d.structural.frames, f.structural)?;
    let parent = need(&d.structural_invocations, sf.invocation)?;
    if sf.configuration != s.id()
        || f.configuration != s.id()
        || graph.key().input != parent.input
        || graph.key().context != parent.context
        || graph.key().name != projection::ProjectionName::CallableInvocation
    {
        return Err(invalid(
            "analytic graph/configuration is foreign to structural frame",
        ));
    }
    let reserve = graph
        .vertex_count()
        .checked_mul(2048)
        .and_then(|n| n.checked_add(graph.arc_count().checked_mul(2048)?))
        .and_then(|n| n.checked_add(d.native.terms.len().checked_mul(1024)?))
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(|| invalid("analytic conversion allocation overflow"))?;
    let mut scratch = b.reserve("analytic-conversion", reserve)?;
    let mut out = Output::new(b);
    out.frames.insert(f.clone())?;
    let mut scope = BTreeSet::new();
    for row in d.structural.scope.iter().filter(|r| r.frame == sf.id()) {
        scope.insert(row.entity);
    }
    let mut public = BTreeSet::new();
    for row in d
        .structural
        .public
        .iter()
        .filter(|r| r.frame == sf.id() && r.in_subsystem)
    {
        public.insert(row.entity);
        out.selectors.insert(PublicSelector {
            frame: f.id(),
            candidate: row.id(),
        })?;
    }
    let mut graph_vertices = BTreeSet::new();
    graph.with_native_graph(|view| -> Result<(), ModelError> {
        for node in view.node_identifiers() {
            graph_vertices.insert(view.entity_id(node));
        }
        for edge in view.edge_references() {
            let projection::ArcId::Invocation(alternative) = *edge.weight() else {
                return Err(invalid("callable graph has non-invocation arc"));
            };
            out.arcs.insert(GraphArc {
                frame: f.id(),
                alternative,
                source: view.entity_id(edge.source()),
                target: view.entity_id(edge.target()),
            })?;
        }
        Ok(())
    })?;
    let mut methods = BTreeMap::new();
    for method in METHODS {
        let (parameters, definition) = definition_with_attribute_policy(s, method, attributes)?;
        if d.parameters.get(parameters.id()) != Some(&parameters)
            || d.definitions.get(definition.id()) != Some(&definition)
        {
            return Err(invalid("analytic canonical authored definition absent"));
        }
        let mut rows = invocations.iter().filter(|r| {
            r.definition == definition.id()
                && r.input == parent.input
                && r.context == parent.context
                && r.subject.is_none()
        });
        let inv = rows
            .next()
            .ok_or_else(|| invalid("analytic invocation missing"))?;
        if rows.next().is_some() {
            return Err(invalid("analytic invocation ambiguous"));
        }
        methods.insert(method as i16, result(f, inv, method, s));
    }
    // Rank uses every selected release callable, including isolated/dangling vertices. Parallel
    // original graph alternatives remain on GraphArc; the kernel adds their integer counts.
    let graph_partial = need(&d.graphs.assessments, sf.invocation_graph)?.availability
        == projection::ProjectionAvailability::Partial;
    for r in methods.values_mut() {
        r.input_partial = graph_partial
            && matches!(
                r.method,
                analysis::AnalysisMethod::PageRank
                    | analysis::AnalysisMethod::Communities
                    | analysis::AnalysisMethod::RelationalConcepts
            );
        if r.selected && r.input_partial {
            r.status = analysis::AnalysisStatus::Partial;
        }
    }
    let mut ranking = methods
        .remove(&(analysis::AnalysisMethod::PageRank as i16))
        .unwrap();
    if ranking.selected {
        let vertices = scope.iter().copied().collect::<Vec<_>>();
        let pairs = out
            .arcs
            .iter()
            .filter(|a| scope.contains(&a.source) && scope.contains(&a.target))
            .map(|a| ranking::Pair {
                source: a.source,
                target: a.target,
                weight: 1,
            })
            .collect::<Vec<_>>();
        let r = ranking::rank(&vertices, &pairs, policy::RETAINED.ranking()?, b)?;
        ranking.iterations = r.iterations() as i64;
        ranking.examined = r.work() as i64;
        ranking.residual = r.residual();
        ranking.stop = match r.stop() {
            ranking::Stop::Converged => Stop::Converged,
            ranking::Stop::Empty => Stop::EmptyDomain,
            ranking::Stop::IterationLimit => Stop::IterationLimit,
            ranking::Stop::WorkLimit => Stop::WorkLimit,
        };
        if matches!(ranking.stop, Stop::IterationLimit | Stop::WorkLimit) {
            ranking.status = analysis::AnalysisStatus::Partial;
        }
        for score in r.scores() {
            out.ranks.insert(RankScore {
                result: ranking.id(),
                target: score.entity,
                score: score.value,
            })?;
        }
    }
    out.results.insert(ranking)?;
    let mut nearest = methods
        .remove(&(analysis::AnalysisMethod::Neighbours as i16))
        .unwrap();
    let layer_invocation = s
        .knn_layer
        .then_some(methods[&(analysis::AnalysisMethod::Communities as i16)].invocation);
    let prepared = super::vectors::produce(
        d,
        f,
        parent,
        &public,
        &mut nearest,
        layer_invocation,
        &mut out,
        b,
    )?;
    out.results.insert(nearest)?;
    let mut communities = methods
        .remove(&(analysis::AnalysisMethod::Communities as i16))
        .unwrap();
    if communities.selected {
        for arc in out
            .arcs
            .iter()
            .filter(|a| scope.contains(&a.source) && scope.contains(&a.target))
            .cloned()
            .collect::<Vec<_>>()
        {
            contribute(
                &mut out,
                f.id(),
                Layer::Invocation,
                arc.source,
                arc.target,
                PairSource::Invocation { arc: arc.id() },
            )?;
        }
        // Retained co-use: one target pair per exact official evaluation scope, across its
        // call sites. Original UsageEvidence and OccurrenceOwnership retain both call premises.
        let _co_use = b.reserve(
            "analytic-co-use-scopes",
            d.structural
                .usage_evidence
                .len()
                .checked_mul(768)
                .ok_or_else(|| invalid("co-use scope overflow"))?,
        )?;
        let mut scopes = BTreeMap::<
            Id<source::Occurrence>,
            BTreeMap<Id<EntityRef>, Id<structural::UsageEvidence>>,
        >::new();
        for e in d.structural.usage_evidence.iter() {
            let site = need(&d.structural.usage_sites, e.site)?;
            if site.frame != sf.id() || !scope.contains(&e.target) {
                continue;
            }
            let event = need(&d.native.event_events, e.event)?;
            let ownership = need(&d.native.owners, event.owner)?;
            scopes
                .entry(ownership.owner)
                .or_default()
                .entry(e.target)
                .or_insert(e.id());
        }
        let required = scopes
            .values()
            .try_fold(0u64, |n, rows| {
                n.checked_add((rows.len() as u64).checked_mul(rows.len() as u64)?)
            })
            .ok_or_else(|| invalid("co-use pair work overflow"))?;
        if required > MAX_WORK {
            communities.status = analysis::AnalysisStatus::Partial;
            communities.stop = Stop::WorkLimit;
        } else {
            for (owner, targets) in scopes {
                let values = targets.into_iter().collect::<Vec<_>>();
                for (i, (left, le)) in values.iter().enumerate() {
                    for (right, re) in &values[i + 1..] {
                        contribute(
                            &mut out,
                            f.id(),
                            Layer::CoUse,
                            *left,
                            *right,
                            PairSource::CoUse {
                                scope: owner,
                                left: *le,
                                right: *re,
                            },
                        )?;
                    }
                }
            }
        }

        if s.type_layer {
            super::attributes::type_layer(d, f, &scope, &mut out, b)?;
        }
        if s.mention_layer {
            super::attributes::mention_layer(d, f, &scope, &mut out, b)?;
        }
        if s.knn_layer {
            for row in out.layer_neighbours.iter().cloned().collect::<Vec<_>>() {
                contribute(
                    &mut out,
                    f.id(),
                    Layer::Nearest,
                    row.query,
                    row.target,
                    PairSource::NearestLayer {
                        neighbour: row.id(),
                    },
                )?;
            }
            let r = out
                .layer_results
                .iter()
                .find(|r| r.frame == f.id() && r.layer == Layer::Nearest)
                .unwrap();
            if r.status != analysis::AnalysisStatus::Completed {
                communities.status = analysis::AnalysisStatus::Partial;
                communities.stop = r.stop;
            }
        }
        let _layers = b.reserve(
            "analytic-collapsed-layer-pairs",
            out.contributions
                .len()
                .checked_mul(1024)
                .ok_or_else(|| invalid("layer pair allocation overflow"))?,
        )?;
        let vertices = scope.iter().copied().collect::<Vec<_>>();
        let layer_count =
            2 + usize::from(s.type_layer) + usize::from(s.mention_layer) + usize::from(s.knn_layer);
        let mut touched = BTreeSet::new();
        let mut combined = BTreeMap::<(Id<EntityRef>, Id<EntityRef>), f64>::new();
        for layer in [
            Layer::Invocation,
            Layer::CoUse,
            Layer::Type,
            Layer::Mention,
            Layer::Nearest,
        ] {
            let mut counts = BTreeMap::<(Id<EntityRef>, Id<EntityRef>), u64>::new();
            for row in out.contributions.iter().filter(|r| r.layer == layer) {
                let v = counts.entry((row.left, row.right)).or_default();
                *v = v
                    .checked_add(1)
                    .ok_or_else(|| invalid("layer count overflow"))?;
            }
            let counts = counts
                .into_iter()
                .map(|((a, c), n)| (a, c, n))
                .collect::<Vec<_>>();
            let weights = communities::normalize(&vertices, &counts, b)?;
            for pair in weights.pairs() {
                let count = counts
                    .iter()
                    .find(|r| r.0 == pair.left && r.1 == pair.right)
                    .unwrap()
                    .2;
                out.layer_pairs.insert(LayerPair {
                    frame: f.id(),
                    layer,
                    left: pair.left,
                    right: pair.right,
                    count: count as i64,
                    normalized_weight: pair.weight,
                })?;
                *combined.entry((pair.left, pair.right)).or_default() +=
                    pair.weight.get() / layer_count as f64;
                touched.insert(pair.left);
                touched.insert(pair.right);
            }
        }
        let pairs = combined
            .into_iter()
            .map(|((left, right), weight)| {
                Ok(communities::Pair {
                    left,
                    right,
                    weight: FiniteF64::new(weight)?,
                })
            })
            .collect::<Result<Vec<_>, ModelError>>()?;
        for p in &pairs {
            out.combined.insert(CombinedPair {
                frame: f.id(),
                left: p.left,
                right: p.right,
                weight: p.weight,
            })?;
        }
        let vertices = touched.iter().copied().collect::<Vec<_>>();
        let required = (vertices.len() as u64 + pairs.len() as u64)
            .checked_mul(
                policy::RETAINED
                    .community_work_multiplier()
                    .ok_or_else(|| invalid("Leiden policy work overflow"))?,
            )
            .and_then(|n| {
                n.checked_add(
                    (public.len() as u64)
                        .checked_mul(public.len() as u64)?
                        .checked_mul(9)?,
                )
            })
            .ok_or_else(|| invalid("Leiden work overflow"))?;
        if required > MAX_WORK {
            communities.status = analysis::AnalysisStatus::Partial;
            communities.stop = Stop::WorkLimit;
        } else {
            let p = communities::partition(&vertices, &pairs, MAX_WORK, b)?;
            communities.iterations = p.runs().iter().map(|r| r.iterations as i64).sum();
            communities.examined = required as i64;
            if communities.stop == Stop::Converged {
                if vertices.is_empty() {
                    communities.stop = Stop::EmptyDomain;
                } else if !p.chosen() {
                    communities.stop = Stop::DegeneratePartition;
                }
            }
            if p.runs().iter().any(|r| !r.converged) {
                communities.status = analysis::AnalysisStatus::Partial;
                if matches!(
                    communities.stop,
                    Stop::Converged | Stop::DegeneratePartition
                ) {
                    communities.stop = Stop::IterationLimit;
                }
            }
            super::partitions::publish(&p, &communities, &public, &mut out, b)?;
        }
    }
    out.results.insert(communities)?;
    if s.knn {
        super::vectors::labels(d, f, parent, prepared.as_ref(), &mut out, b)?;
    }
    for method in [
        analysis::AnalysisMethod::Concepts,
        analysis::AnalysisMethod::RelationalConcepts,
    ] {
        let mut r = methods.remove(&(method as i16)).unwrap();
        if r.selected {
            super::attributes::concepts(d, sf, &public, &mut r, &mut out, b, attributes)?;
        }
        out.results.insert(r)?;
    }
    let touched = out
        .contributions
        .iter()
        .flat_map(|r| [r.left, r.right])
        .collect::<BTreeSet<_>>();
    for entity in graph_vertices
        .union(&scope)
        .chain(public.iter())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        out.universe.insert(UniverseMember {
            frame: f.id(),
            entity,
            graph: graph_vertices.contains(&entity),
            release_scope: scope.contains(&entity),
            public: public.contains(&entity),
            community_touched: touched.contains(&entity),
            excluded_isolate: s.communities
                && scope.contains(&entity)
                && !touched.contains(&entity),
        })?;
    }
    // Conversion scratch is charged before allocation; row inventories retain their own allowance.
    super::conclusions::produce(d, &mut out, b)?;
    scratch.try_resize(0)?;
    Ok(out)
}
pub(super) fn add_pair(
    out: &mut Output,
    f: Id<AnalyticFrame>,
    layer: Layer,
    a: Id<EntityRef>,
    b: Id<EntityRef>,
    source: PairSource,
) -> Result<(), ModelError> {
    contribute(out, f, layer, a, b, source)
}

fn vector_inputs() -> Vec<ValidationInput> {
    macro_rules! rows{($($f:ident:$t:ty,)*)=>{vec![$(ValidationInput::of::<$t>(&["id"]),)*]};}
    crate::analytic_consumption_inputs!(rows)
}
/// Complete acyclic predecessor closure is installed from the actual model, never from a
/// dormant generic proof sum. Every consumed relation carries its source receipt.
pub fn stage(
    profile: stages::Profile,
    settings: &AnalyticsConfiguration,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    settings.validate()?;
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    macro_rules! output{($($t:ty),*)=>{$(outputs.push(RelationUse::of::<$t>());)*};}
    outputs.extend(
        owner::publication_relations()
            .iter()
            .map(RelationUse::of_relation),
    );
    output!(
        assertion::AssertionQualification,
        conditions::Condition,
        conditions::ConditionNode
    );
    let own = outputs
        .iter()
        .filter(|r| !is_vocabulary(r.name()))
        .map(|r| r.name())
        .collect::<BTreeSet<_>>();
    let mut requested = Data::validation_inputs();
    for method in METHODS {
        requested.extend(analysis::expected::inputs(method));
    }
    if profile == Profile::Catalog {
        requested.retain(|i| {
            ![
                flow::FlowValuePathObservation::NAME,
                flow::FlowCallStep::NAME,
                flow::FlowTestLeafObservation::NAME,
            ]
            .contains(&i.name())
        });
    }
    requested.push(ValidationInput::of::<analysis::ProjectionDefinition>(&[
        "id",
    ]));
    let relation = |name| {
        model
            .relation(name)
            .ok_or_else(|| invalid(format!("analytic relation absent: {name}")))
    };
    let mut inputs = vec![];
    for input in requested {
        if own.contains(input.name()) {
            continue;
        }
        let mut use_ = RelationUse::of_relation(relation(input.name())?).completed_input();
        if let Some(epoch) = input.prefix() {
            use_ = use_.at_epoch(epoch);
        }
        inputs.push(use_);
    }
    let roots = dependency_closure::DependencyClosure::roots_from_uses(model, &inputs)?;
    let inputs = dependency_closure::DependencyClosure::grants(
        model,
        roots,
        inputs,
        &outputs,
        PublicationBoundary::Structural,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    let mut key = KeySink::new("analytic-stage");
    settings.id().encode(&mut key);
    for method in METHODS {
        definition(settings, method)?.1.id().encode(&mut key);
    }
    Ok(Stage {
        name: "analyze_analytic",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: ContentHash::of(include_bytes!("build.rs")),
        configuration: key.finish(),
    })
}
