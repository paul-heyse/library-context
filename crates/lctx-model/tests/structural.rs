//! Hand-expected scope/membership controls; workspace topology is verified separately.
use lctx_model::domain::{
    analysis::{self, settings::AnalyticsConfiguration, structural as owner},
    attribution::*,
    catalog::*,
    normalized::{Rows, callables::*, entities::*},
    projection::{self, snapshot::hydrate},
    resources::ResourceBudget,
    source::*,
    structural::{
        build::{self, Data},
        frames::Context,
        *,
    },
    *,
};
fn id<T>(n: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([n; 16].into_iter()))
    .unwrap()
}
fn fixture() -> (ResourceBudget, Data, Context) {
    let b = ResourceBudget::fixed(64 << 20).unwrap();
    let mut d = Data::new(&b);
    let mut c = Context::new(&b);
    let settings = AnalyticsConfiguration {
        module_prefixes: vec!["api".into()],
        public_roots: vec!["api".into()],
        configured_seeds: vec!["api.run".into(), "api.missing".into()],
        depth: 2,
        vertices: 16,
        arcs: 32,
        witnesses: 2,
        brief_budget: 3,
        communities: false,
        pagerank: false,
        fca: false,
        rca: false,
        knn: false,
        type_layer: false,
        mention_layer: false,
        knn_layer: false,
    };
    c.settings.insert(settings.clone()).unwrap();
    let input = id(1);
    let context = id(2);
    let artifact =
        SourceArtifact::from_bytes(input, "api.py".into(), b"def run(): pass\n").unwrap();
    d.projection.artifacts.insert(artifact.clone()).unwrap();
    d.projection
        .scopes
        .insert(CoverageScope::Input { input })
        .unwrap();
    d.projection
        .scopes
        .insert(CoverageScope::Artifact {
            artifact: artifact.id(),
        })
        .unwrap();
    let module = d
        .projection
        .modules
        .insert(Module {
            source: artifact.id(),
            qualified_name: "api".into(),
        })
        .unwrap();
    d.uses
        .insert(input::ArtifactUse {
            input,
            artifact: artifact.id(),
            role: input::SourceRole::Release,
        })
        .unwrap();
    d.projection
        .runs
        .insert(ProviderRun {
            provider: id(3),
            context,
            input,
            configuration: ContentHash::of(b"c"),
            requested_families: ContentHash::of(b"r"),
        })
        .unwrap();
    let declaration = d
        .projection
        .occurrences
        .insert(Occurrence {
            source: artifact.id(),
            start: 0,
            end: 15,
            syntax_kind: SyntaxKind::StmtFunctionDef,
            role: OccurrenceRole::Syntax,
            structural_path: vec![0],
        })
        .unwrap();
    let callable = d
        .projection
        .callables
        .insert(CallableEntity::Source {
            declaration,
            kind: CallableKind::Function,
        })
        .unwrap();
    let entity = d
        .projection
        .refs
        .insert(EntityRef::Callable { callable })
        .unwrap();
    let core = analysis::catalog_core::Invocation::new(
        input,
        context,
        catalog::build::definition().1.id(),
        None,
        [],
    )
    .0;
    d.core_invocations.insert(core.clone()).unwrap();
    let local = c
        .local
        .insert(analysis::local::Invocation::new(input, context, id(4), None, []).0)
        .unwrap();
    c.local_outcomes
        .insert(analysis::local::AnalysisOutcome {
            invocation: local,
            status: analysis::AnalysisStatus::NotRequested,
            reason: Some(obligation::ObligationKind::NotRequested),
        })
        .unwrap();
    let member = d
        .members
        .insert(CatalogMember {
            input,
            access: module,
            path: vec!["run".into()],
            name: "run".into(),
        })
        .unwrap();
    d.core_links
        .insert(CatalogMemberInvocation {
            member,
            invocation: core.id(),
        })
        .unwrap();
    let assessment = d
        .assessments
        .insert(EffectiveCallableAssessment {
            callable,
            context,
            decorators: ContentHash::of(b"d"),
            policy: ContentHash::of(b"p"),
            identity: Knowledge::Unknown,
            identity_reason: CallableReason::UnsupportedDecorator,
            signatures: Knowledge::Known,
            signature_reason: CallableReason::EvidenceAgreement,
            descriptor: Knowledge::Unknown,
            descriptor_kind: None,
            descriptor_reason: CallableReason::UnsupportedDecorator,
            body: Knowledge::Unknown,
            body_admitted: false,
            body_reason: CallableReason::MissingBodyEvidence,
            asynchronous: None,
            generator: None,
        })
        .unwrap();
    d.callables
        .insert(CatalogCallable {
            member,
            candidate: id(5),
            assessment,
            basis: CatalogContractBasis::PublicCandidate,
        })
        .unwrap();
    c.graphs = projection::normalization::normalize(&d.projection, &b).unwrap();
    for method in build::methods() {
        let (parameters, definition) = build::definition(&settings, method).unwrap();
        c.parameters.insert(parameters).unwrap();
        c.definitions.insert(definition.clone()).unwrap();
        let parents = [
            owner::InvocationSource::Local { invocation: local },
            owner::InvocationSource::CatalogCore {
                invocation: core.id(),
            },
        ];
        let (row, links) = owner::Invocation::new(
            input,
            context,
            definition.id(),
            None,
            parents.iter().map(Record::id),
        );
        for parent in parents {
            c.sources.insert(parent).unwrap();
        }
        for link in links {
            c.inputs.insert(link).unwrap();
        }
        c.invocations.insert(row).unwrap();
    }
    assert_eq!(d.projection.refs.get(entity).unwrap().id(), entity);
    c.outcomes = outcomes::derive(&c, &produce(&d, &c, &b), &b).unwrap();
    (b, d, c)
}
fn produce(d: &Data, c: &Context, b: &ResourceBudget) -> Output {
    let core = d.core_invocations.iter().next().unwrap();
    let f = frames::frame(c, core).unwrap();
    let graph = |assessment| {
        let a = c.graphs.assessments.get(assessment).unwrap();
        let h = c
            .graphs
            .snapshots
            .iter()
            .find(|s| s.assessment == assessment)
            .unwrap();
        hydrate(h, a, &c.graphs.chunks, b).unwrap()
    };
    build::produce(
        d,
        &f,
        c.invocations.get(f.invocation).unwrap(),
        c.configuration().unwrap(),
        &graph(f.invocation_graph),
        &graph(f.definition_graph),
        b,
    )
    .unwrap()
}
#[test]
fn candidate_uncertainty_missing_authored_seed_and_exact_scope_survive_replay() {
    let (b, d, c) = fixture();
    let out = produce(&d, &c, &b);
    frames::verify(&d, &c, &out, &b).unwrap();
    assert_eq!(out.public.len(), 1);
    let candidate = out.public.iter().next().unwrap();
    assert_eq!(candidate.path, "api.run");
    assert!(candidate.in_subsystem);
    assert_eq!(
        out.configured
            .iter()
            .find(|r| r.path == "api.missing")
            .unwrap()
            .candidates,
        0
    );
    assert!(
        out.usage_scores.is_empty(),
        "missing observed use is not a stored zero score"
    );
    assert_eq!(
        d.assessments.iter().next().unwrap().identity,
        Knowledge::Unknown
    );
    let mut omitted = produce(&d, &c, &b);
    omitted.public = Rows::new(&b);
    assert!(frames::verify(&d, &c, &omitted, &b).is_err());
    let mut forged = produce(&d, &c, &b);
    let mut seed = forged
        .configured
        .iter()
        .find(|r| r.path == "api.missing")
        .unwrap()
        .clone();
    seed.candidates = 1;
    forged.configured = Rows::new(&b);
    forged.configured.insert(seed).unwrap();
    assert!(frames::verify(&d, &c, &forged, &b).is_err());
    let tiny = ResourceBudget::fixed(1).unwrap();
    let core = d.core_invocations.iter().next().unwrap();
    let f = frames::frame(&c, core).unwrap();
    let a = c.graphs.assessments.get(f.invocation_graph).unwrap();
    let h = c
        .graphs
        .snapshots
        .iter()
        .find(|s| s.assessment == a.id())
        .unwrap();
    assert!(hydrate(h, a, &c.graphs.chunks, &tiny).is_err());
    assert_eq!(tiny.reserved(), 0);
    drop(out);
    drop(omitted);
    drop(forged);
    drop(d);
    drop(c);
    assert_eq!(b.reserved(), 0);
}
#[test]
fn coupled_frame_removal_foreign_settings_and_parent_shrink_refuse() {
    let (b, d, mut c) = fixture();
    let out = produce(&d, &c, &b);
    c.invocations = Rows::new(&b);
    c.inputs = Rows::new(&b);
    c.sources = Rows::new(&b);
    assert!(frames::verify(&d, &c, &Output::new(&b), &b).is_err());
    drop(out);
    let (b, d, mut c) = fixture();
    let out = produce(&d, &c, &b);
    c.inputs = Rows::new(&b);
    assert!(frames::verify(&d, &c, &out, &b).is_err());
    let mut changed = c.configuration().unwrap().clone();
    changed.depth += 1;
    c.settings = Rows::new(&b);
    c.settings.insert(changed).unwrap();
    assert!(frames::verify(&d, &c, &out, &b).is_err());
}
#[test]
fn structural_outcomes_are_exact_replayed_semantic_results() {
    let (b, d, mut c) = fixture();
    let out = produce(&d, &c, &b);
    assert_eq!(c.outcomes.len(), 4);
    let control = c
        .outcomes
        .iter()
        .find(|r| r.status == analysis::AnalysisStatus::NotRequested)
        .unwrap()
        .clone();
    assert_eq!(
        control.reason,
        Some(obligation::ObligationKind::NotRequested)
    );
    assert_eq!(
        c.outcomes
            .iter()
            .filter(|r| r.status == analysis::AnalysisStatus::Completed)
            .count(),
        3
    );
    c.outcomes = Rows::new(&b);
    assert!(frames::verify(&d, &c, &out, &b).is_err());
    let correct = outcomes::derive(&c, &out, &b).unwrap();
    for row in correct.iter() {
        let mut row = row.clone();
        if row.invocation == control.invocation {
            row.status = analysis::AnalysisStatus::Completed;
            row.reason = None;
        }
        c.outcomes.insert(row).unwrap();
    }
    assert!(frames::verify(&d, &c, &out, &b).is_err());
    c.outcomes = correct;
    frames::verify(&d, &c, &out, &b).unwrap();
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(outcomes::derive(&c, &out, &tiny).is_err());
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn structural_stops_and_earlier_frontier_outcomes_keep_distinct_meanings() {
    let (b, d, c) = fixture();
    let mut out = produce(&d, &c, &b);
    let mut frame = out.frames.iter().next().unwrap().clone();
    frame.controls_requested = true;
    out.frames = Rows::new(&b);
    out.frames.insert(frame.clone()).unwrap();
    let no_stop = outcomes::derive(&c, &out, &b).unwrap();
    assert!(
        no_stop
            .iter()
            .any(|r| r.invocation == frame.control_invocation
                && r.status == analysis::AnalysisStatus::Partial
                && r.reason == Some(obligation::ObligationKind::IncompleteDomain))
    );
    out.traversals
        .insert(Traversal {
            frame: frame.id(),
            seed: id(99),
            stop: Some(TraversalStop::Depth),
            partial: true,
            examined_vertices: 1,
            examined_arcs: 0,
        })
        .unwrap();
    out.control_traversals
        .insert(controls::ControlTraversal {
            frame: frame.id(),
            seed: id(99),
            formal: id(98),
            stop: Some(TraversalStop::Arcs),
            vertices: 1,
            arcs: 0,
        })
        .unwrap();
    let stopped = outcomes::derive(&c, &out, &b).unwrap();
    for invocation in [frame.invocation, frame.control_invocation] {
        assert!(stopped.iter().any(|r| r.invocation == invocation
            && r.status == analysis::AnalysisStatus::Partial
            && r.reason == Some(obligation::ObligationKind::BudgetReached)));
    }
    let empty_data = Data::new(&b);
    let mut empty_context = Context::new(&b);
    let empty = Output::new(&b);
    frames::verify(&empty_data, &empty_context, &empty, &b).unwrap();
    empty_context
        .outcomes
        .insert(stopped.iter().next().unwrap().clone())
        .unwrap();
    assert!(frames::verify(&empty_data, &empty_context, &empty, &b).is_err());
}
#[test]
fn static_conclusions_retain_exact_source_and_refuse_promotion_or_erasure() {
    use analysis::{
        policy::{EvidenceStatus, FindingKind},
        support::DerivedEvidence,
    };
    let (b, d, c) = fixture();
    let mut out = produce(&d, &c, &b);
    let row = out
        .conclusions
        .iter()
        .find(|r| r.kind == FindingKind::PublicAlias)
        .unwrap()
        .clone();
    assert_eq!(
        row.source_facts().status,
        EvidenceStatus::StructurallyObserved
    );
    let q = out
        .conclusion_qualifications
        .get(row.qualification())
        .unwrap();
    assert_eq!(q.modality, Modality::Candidate);
    assert_eq!(q.approximation, assertion::Approximation::Over);
    assert_eq!(q.condition, conditions::Diagram::always().id());
    assert_eq!(
        q.scope,
        CoverageScope::Input {
            input: d.core_invocations.iter().next().unwrap().input
        }
        .id()
    );
    assert!(
        matches!(out.conclusion_sources.get(row.source),Some(ConclusionSource::Public{candidate}) if out.public.get(*candidate).is_some())
    );
    let mut forged = row;
    forged.kind = FindingKind::ConditionalRaise;
    out.conclusions = Rows::new(&b);
    out.conclusions.insert(forged).unwrap();
    assert!(frames::verify(&d, &c, &out, &b).is_err());
    let mut out = produce(&d, &c, &b);
    out.conclusions = Rows::new(&b);
    out.conclusion_sources = Rows::new(&b);
    assert!(frames::verify(&d, &c, &out, &b).is_err());
}

fn requested_controls() -> (
    ResourceBudget,
    Data,
    Context,
    Output,
    analysis::local::AnalysisCoverage,
) {
    let (budget, data, mut context) = fixture();
    let mut output = produce(&data, &context, &budget);
    let local = context.local.iter().next().unwrap().clone();
    context.local_outcomes = Rows::new(&budget);
    context
        .local_outcomes
        .insert(analysis::local::AnalysisOutcome {
            invocation: local.id(),
            status: analysis::AnalysisStatus::Completed,
            reason: None,
        })
        .unwrap();
    let mut frame = output.frames.iter().next().unwrap().clone();
    frame.controls_requested = true;
    output.frames = Rows::new(&budget);
    output.frames.insert(frame).unwrap();
    let (receipt, premises) = analysis::local::coverage::assess(
        &analysis::local::coverage::CoverageExpectation {
            invocation: local.id(),
            capability: analysis::AnalysisCapability::Transfers,
            scope: (CoverageScope::Input { input: local.input }).id(),
            context: local.context,
            requested: true,
            no_scope: true,
            sources: vec![],
        },
        &[],
        analysis::AnalysisStatus::Completed,
        None,
        &budget,
    )
    .unwrap();
    assert!(premises.is_empty());
    (budget, data, context, output, receipt)
}
fn control_outcome(
    context: &Context,
    output: &Output,
    budget: &ResourceBudget,
) -> owner::AnalysisOutcome {
    let invocation = output.frames.iter().next().unwrap().control_invocation;
    outcomes::derive(context, output, budget)
        .unwrap()
        .iter()
        .find(|row| row.invocation == invocation)
        .unwrap()
        .clone()
}
#[test]
fn controls_empty_domain_requires_exact_acknowledged_local_noscope() {
    let (budget, _, mut context, output, receipt) = requested_controls();
    let missing = control_outcome(&context, &output, &budget);
    assert_eq!(missing.status, analysis::AnalysisStatus::Partial);
    assert_eq!(
        missing.reason,
        Some(obligation::ObligationKind::IncompleteDomain)
    );
    context.local_coverage.insert(receipt.clone()).unwrap();
    let completed = control_outcome(&context, &output, &budget);
    assert_eq!(completed.status, analysis::AnalysisStatus::Completed);
    assert_eq!(completed.reason, None);
    assert!(
        Context::validation_inputs()
            .iter()
            .any(|input| input.name() == analysis::local::AnalysisCoverage::NAME)
    );
    let mut decoded = Context::new(&budget);
    assert!(
        decoded
            .visit(
                analysis::local::AnalysisCoverage::NAME,
                &analysis::local::AnalysisCoverage::encode(std::slice::from_ref(&receipt)).unwrap()
            )
            .unwrap()
    );
    assert_eq!(decoded.local_coverage.get(receipt.id()), Some(&receipt));
    let tiny = ResourceBudget::fixed(1).unwrap();
    assert!(outcomes::derive(&context, &output, &tiny).is_err());
    assert_eq!(tiny.reserved(), 0);
}
#[test]
fn controls_noscope_does_not_promote_foreign_or_nonempty_local_evidence() {
    use normalized::coverage::EvidenceAvailability as Availability;
    for mismatch in [
        "invocation",
        "input",
        "context",
        "capability",
        "scope",
        "artifact_scope",
        "complete",
        "partial",
        "unavailable",
        "not_requested",
        "subject",
        "duplicate_root",
    ] {
        let (budget, _, mut context, output, mut receipt) = requested_controls();
        let original = context.local.iter().next().unwrap().clone();
        match mismatch {
            "invocation" => receipt.invocation = id(90),
            "input" => {
                let foreign = context
                    .local
                    .insert(
                        analysis::local::Invocation::new(
                            id(91),
                            original.context,
                            original.definition,
                            None,
                            [],
                        )
                        .0,
                    )
                    .unwrap();
                receipt.invocation = foreign;
                receipt.scope = (CoverageScope::Input { input: id(91) }).id();
            }
            "context" => receipt.context = id(92),
            "capability" => receipt.capability = analysis::AnalysisCapability::ControlInfluence,
            "scope" => receipt.scope = (CoverageScope::Input { input: id(93) }).id(),
            "artifact_scope" => receipt.scope = (CoverageScope::Artifact { artifact: id(94) }).id(),
            "complete" => receipt.availability = Availability::Complete,
            "partial" => {
                receipt.availability = Availability::Partial;
                receipt.reason = Some(obligation::ObligationKind::IncompleteDomain);
            }
            "unavailable" => {
                receipt.availability = Availability::Unavailable;
                receipt.reason = Some(obligation::ObligationKind::IncompleteDomain);
            }
            "not_requested" => {
                receipt.availability = Availability::NotRequested;
                receipt.reason = Some(obligation::ObligationKind::NotRequested);
            }
            "subject" => {
                receipt.invocation = context
                    .local
                    .insert(
                        analysis::local::Invocation::new(
                            original.input,
                            original.context,
                            original.definition,
                            Some(id(95)),
                            [],
                        )
                        .0,
                    )
                    .unwrap();
            }
            "duplicate_root" => {
                context
                    .local
                    .insert(
                        analysis::local::Invocation::new(
                            original.input,
                            original.context,
                            id(96),
                            None,
                            [],
                        )
                        .0,
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        context.local_coverage.insert(receipt).unwrap();
        let result = control_outcome(&context, &output, &budget);
        assert_eq!(
            result.status,
            analysis::AnalysisStatus::Partial,
            "{mismatch}"
        );
        assert_eq!(
            result.reason,
            Some(obligation::ObligationKind::IncompleteDomain),
            "{mismatch}"
        );
    }
}
#[test]
fn controls_noscope_keeps_not_requested_and_budget_stop_precedence() {
    let (budget, _, mut context, mut output, receipt) = requested_controls();
    context.local_coverage.insert(receipt).unwrap();
    let mut frame = output.frames.iter().next().unwrap().clone();
    output
        .control_traversals
        .insert(controls::ControlTraversal {
            frame: frame.id(),
            seed: id(99),
            formal: id(98),
            stop: Some(TraversalStop::Arcs),
            vertices: 1,
            arcs: 0,
        })
        .unwrap();
    let bounded = control_outcome(&context, &output, &budget);
    assert_eq!(bounded.status, analysis::AnalysisStatus::Partial);
    assert_eq!(
        bounded.reason,
        Some(obligation::ObligationKind::BudgetReached)
    );
    frame.controls_requested = false;
    output.frames = Rows::new(&budget);
    output.frames.insert(frame.clone()).unwrap();
    output
        .control_traversals
        .insert(controls::ControlTraversal {
            frame: frame.id(),
            seed: id(99),
            formal: id(98),
            stop: Some(TraversalStop::Arcs),
            vertices: 1,
            arcs: 0,
        })
        .unwrap();
    let unrequested = control_outcome(&context, &output, &budget);
    assert_eq!(unrequested.status, analysis::AnalysisStatus::NotRequested);
    assert_eq!(
        unrequested.reason,
        Some(obligation::ObligationKind::NotRequested)
    );
}

fn portable_structural_checks(
    data: &Data,
    context: &Context,
    output: &Output,
    budget: &ResourceBudget,
) -> Result<(), ModelError> {
    let model = lctx_model::domain::model()?;
    let mut frames = (model.invariant("structural_frame_fidelity")?.create)(budget);
    let mut source = (model.invariant("structural_source_fidelity")?.create)(budget);
    macro_rules! visit {
        ($rows:expr) => {{
            let rows = $rows.iter().cloned().collect::<Vec<_>>();
            if let Some(first) = rows.first() {
                let batch = Record::encode(&rows)?;
                let name = derivation::RowRef::of(first.id()).relation();
                if model
                    .invariant("structural_frame_fidelity")?
                    .inputs
                    .iter()
                    .any(|input| input.name() == name)
                {
                    frames.visit(name, &batch)?;
                }
                if model
                    .invariant("structural_source_fidelity")?
                    .inputs
                    .iter()
                    .any(|input| input.name() == name)
                {
                    source.visit(name, &batch)?;
                }
            }
        }};
    }
    visit!(data.projection.runs);
    visit!(data.core_invocations);
    visit!(context.settings);
    visit!(context.definitions);
    visit!(context.parameters);
    visit!(context.local);
    visit!(context.local_outcomes);
    visit!(context.local_coverage);
    visit!(context.invocations);
    visit!(context.sources);
    visit!(context.inputs);
    visit!(context.outcomes);
    // Portable checks get assessments only; neither snapshot headers nor bytes are supplied.
    visit!(context.graphs.assessments);
    visit!(data.projection.artifacts);
    visit!(data.projection.scopes);
    visit!(data.projection.refs);
    visit!(data.projection.callables);
    visit!(data.projection.occurrences);
    visit!(data.projection.qualifications);
    macro_rules! visit_outputs { ($($field:ident:$ty:ty,)*) => {$(visit!(output.$field);)*}; }
    lctx_model::structural_outputs!(visit_outputs);
    visit!(output.conclusion_qualifications);
    frames.finish()?;
    source.finish()
}

#[test]
fn portable_structural_frame_source_and_boundary_checks_need_no_snapshot_replay() {
    let (budget, data, context) = fixture();
    let output = produce(&data, &context, &budget);
    portable_structural_checks(&data, &context, &output, &budget).unwrap();
    let model = lctx_model::domain::model().unwrap();
    for name in ["structural_frame_fidelity", "structural_source_fidelity"] {
        let invariant = model.invariant(name).unwrap();
        assert_eq!(invariant.purpose, InvariantPurpose::Admission);
        assert!(
            invariant
                .inputs
                .iter()
                .all(|input| input.name() != projection::ProjectionSnapshot::NAME
                    && input.name() != projection::ProjectionSnapshotChunk::NAME)
        );
    }
    assert_eq!(
        model.invariant("structural_replay").unwrap().purpose,
        InvariantPurpose::DiagnosticReplay
    );
}

#[test]
fn portable_structural_admission_refuses_existing_foreign_source_and_omitted_frame_parent() {
    let (budget, data, mut context) = fixture();
    let mut output = produce(&data, &context, &budget);
    let mut conclusions = Rows::new(&budget);
    let frame = output.frames.iter().next().unwrap();
    let row = output
        .conclusions
        .iter()
        .find(|row| row.invocation == frame.invocation)
        .unwrap()
        .clone();
    let mut changed = row.clone();
    // Existing same-frame invocation, but the source belongs to Delegation, not DirectUsage.
    changed.invocation = frame.usage_invocation;
    conclusions.insert(changed).unwrap();
    output.conclusions = conclusions;
    assert!(
        portable_structural_checks(&data, &context, &output, &budget)
            .unwrap_err()
            .to_string()
            .contains("source/frame/subject/method")
    );
    output.conclusions = Rows::new(&budget);
    output.conclusions.insert(row).unwrap();
    let mut inputs = Rows::new(&budget);
    for link in context.inputs.iter().skip(1) {
        inputs.insert(link.clone()).unwrap();
    }
    context.inputs = inputs;
    assert!(
        portable_structural_checks(&data, &context, &output, &budget)
            .unwrap_err()
            .to_string()
            .contains("complete frame/parent")
    );
}

#[test]
fn portable_structural_admission_refuses_missing_frame_and_promoted_not_requested_outcome() {
    let (budget, data, mut context) = fixture();
    let mut output = produce(&data, &context, &budget);
    output.frames = Rows::new(&budget);
    assert!(
        portable_structural_checks(&data, &context, &output, &budget)
            .unwrap_err()
            .to_string()
            .contains("complete frame/parent")
    );
    let output = produce(&data, &context, &budget);
    let control = output.frames.iter().next().unwrap().control_invocation;
    let mut outcomes = Rows::new(&budget);
    for row in context.outcomes.iter() {
        let mut row = row.clone();
        if row.invocation == control {
            row.status = analysis::AnalysisStatus::Completed;
            row.reason = None;
        }
        outcomes.insert(row).unwrap();
    }
    context.outcomes = outcomes;
    assert!(
        portable_structural_checks(&data, &context, &output, &budget)
            .unwrap_err()
            .to_string()
            .contains("actual eligibility/stop/NoScope")
    );
}

#[test]
fn structural_admission_dispatches_every_declared_semantic_input() {
    let budget = ResourceBudget::fixed(64 << 20).unwrap();
    let model = lctx_model::domain::model().unwrap();
    let mut data = Data::new(&budget);
    let bridge = analysis::native::NativeAssertionPremise::NAME;
    let inputs = Data::validation_inputs();
    assert!(inputs.iter().all(|input| input.name() != bridge));
    for input in inputs {
        let relation = model.relation(input.name()).unwrap();
        let batch = arrow_array::RecordBatch::new_empty(relation.schema().clone());
        assert!(
            data.visit_input(&input, &batch).unwrap(),
            "{}",
            input.name()
        );
    }
    for profile in stages::Profile::ALL {
        assert!(
            Data::consumed_inputs(profile)
                .iter()
                .any(|input| input.name() == bridge)
        );
    }
}
