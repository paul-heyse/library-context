//! Actual captured native/normalized/Local inputs drive the finite summary kernel.
#[path = "fixtures/transfer_composition.rs"]
mod fixture;
use lctx_model::domain::{
    analysis,
    execution::{self, summary_production::*},
    projection::{
        self,
        normalization::{ProjectionData, ProjectionKey},
        snapshot::MaterializedGraph,
    },
    stages::{Profile, PublicationBoundary},
    *,
};
#[tokio::test]
async fn native_finite_claims_keep_raw_identity_conditional_paths_and_open_siblings() {
    let f = fixture::native_from("phase4_summaries").await;
    let budget = &f.budget;
    let input = f.rows::<input::InputRevision>()[0].id();
    let context = f.data.event_events.iter().next().unwrap().context;
    let mut local = local_semantics::LocalData::new(budget);
    let mut summary = SummaryData::new(budget);
    let mut projection = ProjectionData::new(budget);
    let mut inventory = analysis::native::NativeInventory::new(budget);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        local.visit(name, batch).unwrap();
        projection.visit(name, batch).unwrap();
        if analysis::native::NativeInventory::inputs()
            .iter()
            .any(|i| i.name() == *name)
        {
            inventory.visit(name, batch).unwrap();
        }
    }
    macro_rules! load {
        ($ty:ty,$rows:expr) => {{
            let batch = <$ty as Record>::encode(($rows).as_ref()).unwrap();
            local.visit(<$ty>::NAME, &batch).unwrap();
            projection.visit(<$ty>::NAME, &batch).unwrap();
            let i = ValidationInput::of::<$ty>(&["id"]);
            if stages::is_vocabulary(i.name()) {
                summary
                    .visit_input(&i.clone().at_epoch(PublicationBoundary::Facts), &batch)
                    .unwrap();
                summary
                    .visit_input(&i.at_epoch(PublicationBoundary::Model), &batch)
                    .unwrap();
            } else {
                summary.visit_input(&i, &batch).unwrap();
            }
        }};
    }
    macro_rules! facts{($($field:ident:$ty:ty,)*)=>{$(load!($ty,f.data.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_inputs!(facts);
    macro_rules! output{($($field:ident:$ty:ty,)*)=>{$(load!($ty,f.output.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_outputs!(output);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        for i in SummaryData::inputs().iter().filter(|i| i.name() == *name) {
            summary.visit_input(i, batch).unwrap();
        }
    }
    let inventory = inventory.collect().unwrap();
    load!(
        analysis::native::NativeQualification,
        inventory.qualifications.iter().cloned().collect::<Vec<_>>()
    );
    load!(
        analysis::native::NativeAssertionPremise,
        inventory.premises.iter().cloned().collect::<Vec<_>>()
    );
    let (_, definition) = local_semantics::definition();
    let (invocation, _) =
        analysis::local::AnalysisInvocation::new(input, context, definition.id(), None, []);
    load!(
        analysis::local::AnalysisInvocation,
        std::slice::from_ref(&invocation)
    );
    load!(analysis::AnalysisDefinition, std::slice::from_ref(&definition));
    let output = local_semantics::produce(&local, &invocation, &definition, budget).unwrap();
    load!(
        analysis::local::AnalysisOutcome,
        [analysis::local::AnalysisOutcome {
            invocation: invocation.id(),
            status: analysis::AnalysisStatus::Partial,
            reason: Some(obligation::ObligationKind::IncompleteDomain)
        }]
    );
    macro_rules! local_rows{($($field:ident:$ty:ty,)*)=>{$(let batch=<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>()).unwrap();let i=ValidationInput::of::<$ty>(&["id"]);summary.visit_input(&if stages::is_vocabulary(i.name()){i.at_epoch(PublicationBoundary::Model)}else{i},&batch).unwrap();)*};}
    lctx_model::local_semantic_outputs!(local_rows);
    let key = ProjectionKey {
        input,
        context,
        name: projection::ProjectionName::CallableInvocation,
    };
    let graph = MaterializedGraph::build(
        &projection::normalization::describe(&projection, key, budget).unwrap(),
        budget,
    )
    .unwrap();
    let catalog = models::Catalog::committed().unwrap();
    // Select the sibling control from captured declarations and source syntax, before any
    // bounded Summary output. Canonical IDs may reorder which seed a one-member cap admits.
    let relay_declarations = summary
        .entry
        .symbol_declarations
        .iter()
        .filter(|r| summary.entry.symbols.get(r.symbol).is_some_and(|s| s.name == "relay"))
        .map(|r| r.declaration)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(relay_declarations.len(), 1);
    let relay_declaration = *relay_declarations.iter().next().unwrap();
    let relay = summary.entry.occurrences.get(relay_declaration).unwrap();
    let relay_parameters = summary
        .entry
        .declarations
        .iter()
        .filter(|r| summary.entry.occurrences.get(r.declaration).is_some_and(|o| {
            o.source == relay.source && o.start >= relay.start && o.end <= relay.end
                && o.structural_path.starts_with(&relay.structural_path)
        }))
        .map(|r| r.declaration)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(relay_parameters.len(), 1);
    let relay_events = summary
        .bindings
        .event_events
        .iter()
        .filter(|event| {
            let site = summary.entry.occurrences.get(event.site).unwrap();
            summary.entry.owners.get(event.owner).is_some_and(|owner| owner.owner == relay_declaration)
                && event.context == context
                && f.source_bytes(site.source)[site.start as usize..site.end as usize] == *b"identity(value)"
        })
        .map(Record::id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(relay_events.len(), 1, "actual relay-to-identity call event");
    let relay_event = *relay_events.iter().next().unwrap();
    let mut baseline = None;
    let default_work = execution::configuration::SummaryLimits::default().work;
    for (depth, members, work) in [(0, 1 << 16, default_work), (2, 1 << 16, default_work), (2, 1, default_work), (2, 1 << 16, 1)] {
        let (p, d) = execution::configuration::summaries(
            catalog.declaration().id(),
            execution::configuration::SummaryLimits {
                depth,
                members,
                work,
                ..Default::default()
            },
        )
        .unwrap();
        summary.parameters.insert(p).unwrap();
        let (invocation, _) =
            analysis::summary::AnalysisInvocation::new(input, context, d.id(), None, []);
        let output = produce(
            &summary,
            &invocation,
            &d,
            Profile::Behavioral,
            &graph,
            budget,
        )
        .unwrap();
        let conclusions = execution::summary_consequences::derive(
            &summary,
            &output,
            &invocation,
            &d,
            Profile::Behavioral,
            budget,
        )
        .unwrap();
        eprintln!(
            "claims depth={depth} members={members} work={work} finite={} closure={} verdicts={:?}",
            conclusions
                .claims
                .iter()
                .filter(|c| matches!(
                    c,
                    execution::summary_consequences::SummaryClaim::FiniteAlternative { .. }
                ))
                .count(),
            conclusions
                .claims
                .iter()
                .filter(|c| matches!(
                    c,
                    execution::summary_consequences::SummaryClaim::CallClosure { .. }
                ))
                .count(),
            conclusions
                .conclusions
                .iter()
                .map(|r| (r.verdict, r.reason))
                .collect::<Vec<_>>()
        );
        assert!(
            conclusions
                .conclusions
                .iter()
                .any(|c| c.verdict == obligation::Verdict::Established)
        );
        assert!(
            conclusions
                .conclusions
                .iter()
                .any(|c| c.verdict == obligation::Verdict::Conditional),
            "actual native guard remains conditional"
        );
        let identity = summary
            .entry
            .symbol_declarations
            .iter()
            .filter(|r| {
                summary
                    .entry
                    .symbols
                    .get(r.symbol)
                    .is_some_and(|s| s.name == "identity")
            })
            .map(|r| r.declaration)
            .collect::<std::collections::BTreeSet<_>>();
        let direct=conclusions.conclusions.iter().filter(|r|matches!(conclusions.subjects.get(r.subject),Some(analysis::summary::ObligationSubject::SummaryClaim{transfer})if matches!(conclusions.claims.get(*transfer),Some(execution::summary_consequences::SummaryClaim::FiniteAlternative{transfer,..})if conclusions.keys.get(*transfer).is_some_and(|k|summary.vocabulary.places.get(&k.output).and_then(|p|summary.vocabulary.roots.get(&p.root)).is_some_and(|r|matches!(r,value::PlaceRoot::Return{callable}if identity.contains(callable))))))).map(|r|(r.subject,r.verdict,r.reason)).collect::<Vec<_>>();
        assert!(
            !direct.is_empty(),
            "raw identity Return must survive depth0"
        );
        if let Some(ref baseline) = baseline {
            assert_eq!(
                &direct, baseline,
                "later limits cannot revoke same exact raw alternative"
            );
        } else {
            baseline = Some(direct);
        }
        if members == 1 {
            // Rejected source states have transfer boundaries even when no caller/callee
            // pair survives to define a closure query. Do not choose a survivor by ID order.
            assert_eq!(output.outcome.status, analysis::AnalysisStatus::Partial);
            assert_eq!(output.outcome.reason, Some(obligation::ObligationKind::IncompleteCoverage));
            let boundaries = output.origin_boundaries.iter().filter(|r| {
                r.reason == obligation::ObligationKind::SummaryProofLimit
            }).collect::<Vec<_>>();
            assert!(!boundaries.is_empty(), "one-member cap retains explicit source boundaries");
            for boundary in boundaries {
                assert_eq!(boundary.invocation, invocation.id());
                assert!(output.origins.get(boundary.origin).is_some());
                assert_eq!(output.keys.get(boundary.transfer).unwrap().context, context);
                let q = output.vocabulary.qualifications.get(&boundary.qualification).unwrap();
                assert_eq!(q.context, context);
                assert_eq!(q.approximation, assertion::Approximation::Over);
                let subject = analysis::summary::ObligationSubject::SummaryTransfer {
                    transfer: boundary.transfer,
                };
                assert_eq!(output.subjects.get(subject.id()), Some(&subject));
                assert!(output.obligations.iter().any(|o| {
                    o.invocation == invocation.id() && o.subject == subject.id()
                        && o.qualification == boundary.qualification && o.reason == boundary.reason
                        && o.responsible == analysis::AnalysisMethod::Summaries
                        && o.channel == analysis::AnalysisChannel::Value && o.phase == calls::CallPhase::Call
                }), "each refused source retains its qualified transfer obligation");
            }
        }
        if depth > 0 && members > 1 {
            assert!(conclusions.conclusions.iter().any(|r|matches!(conclusions.subjects.get(r.subject),Some(analysis::summary::ObligationSubject::SummaryClaim{transfer})if matches!(conclusions.claims.get(*transfer),Some(execution::summary_consequences::SummaryClaim::CallClosure{..})))&&r.verdict==obligation::Verdict::Unknown),"open set sibling remains unknown");
        }
        if work == 1 {
            // Queue admission consumes the sole unit; processing then refuses every actual
            // pending pair. This opens a known call without depending on which ID sorts first.
            let residuals = output.residuals.iter().filter(|r| {
                r.event == relay_event && r.reason == obligation::ObligationKind::SummaryPairWorkLimit
                    && summary.vocabulary.places.get(&r.input).is_some_and(|p| {
                        p.path == value::AccessPath::empty().id()
                            && matches!(summary.vocabulary.roots.get(&p.root), Some(value::PlaceRoot::Entry { declaration }) if relay_parameters.contains(declaration))
                    })
                    && summary.vocabulary.places.get(&r.output).is_some_and(|p| {
                        p.path == value::AccessPath::empty().id()
                            && matches!(summary.vocabulary.roots.get(&p.root), Some(value::PlaceRoot::Return { callable }) if identity.contains(callable))
                    })
            }).collect::<Vec<_>>();
            assert!(!residuals.is_empty(), "actual relay Entry-to-identity Return pair remains explicitly open");
            for residual in residuals {
                let claim = conclusions.claims.iter().find(|claim| matches!(claim,
                    execution::summary_consequences::SummaryClaim::CallClosure { event, input, output, qualification, channel, phase, .. }
                    if *event == relay_event && *input == residual.input && *output == residual.output
                        && *qualification == residual.qualification && *channel == analysis::AnalysisChannel::Value
                        && *phase == calls::CallPhase::Call
                )).expect("known exhausted pair retains its closure claim");
                let subject = analysis::summary::ObligationSubject::SummaryClaim { transfer: claim.id() };
                let conclusion = conclusions.conclusions.iter().find(|r| r.subject == subject.id()).unwrap();
                assert_eq!(conclusion.verdict, obligation::Verdict::Unknown);
                assert!(conclusion.proof.is_none(), "open sibling cannot acquire a closure proof");
                assert!(conclusions.members.iter().filter(|m| m.claim == claim.id()).any(|member| {
                    conclusions.standings.iter().any(|s| s.member == member.id()
                        && s.standing == execution::summary_consequences::MemberStanding::Open
                        && s.proof.is_none() && s.reason.is_some())
                }), "the independently selected call retains its open member standing");
            }
        }
        if depth == 2 && members > 1 && work > 1 {
            let closed=conclusions.conclusions.iter().find(|r|r.proof.is_some()&&matches!(conclusions.subjects.get(r.subject),Some(analysis::summary::ObligationSubject::SummaryClaim{transfer})if matches!(conclusions.claims.get(*transfer),Some(execution::summary_consequences::SummaryClaim::CallClosure{..})))).expect("actual complete identity call must close");
            assert!(matches!(
                closed.verdict,
                obligation::Verdict::Established | obligation::Verdict::Conditional
            ));
            let proof = closed.proof.unwrap();
            let qid = closed.qualification.unwrap();
            let q = output
                .vocabulary
                .qualifications
                .get(&qid)
                .or_else(|| summary.vocabulary.qualifications.get(&qid))
                .unwrap();
            let observed = summary
                .entry
                .coverage
                .iter()
                .filter(|r| {
                    r.context == q.context
                        && r.scope == q.scope
                        && r.family == attribution::FactFamily::Flow
                        && r.run
                            .and_then(|id| summary.entry.runs.get(id))
                            .is_some_and(|r| r.input == input)
                })
                .map(analysis::summary::coverage::CoverageObservation::native)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(!observed.is_empty());
            let expectation = analysis::summary::coverage::CoverageExpectation {
                invocation: invocation.id(),
                capability: analysis::AnalysisCapability::Summaries,
                scope: q.scope,
                context: q.context,
                requested: true,
                no_scope: false,
                sources: observed.iter().map(|r| r.source().id()).collect(),
            };
            let (coverage, _) = analysis::summary::coverage::assess(
                &expectation,
                &observed,
                output.outcome.status,
                output.outcome.reason,
                budget,
            )
            .unwrap();
            let expected=conclusions.derivations.iter().find(|d|conclusions.derivation_premises.iter().any(|p|p.derivation==d.id()&&matches!(conclusions.sources.get(p.source),Some(analysis::summary::SupportSource::ClaimProof{witness})if *witness==proof))).unwrap();
            assert!(
                conclusions
                    .pending
                    .iter()
                    .filter(|p| p.matches(&coverage))
                    .filter_map(|p| p.admit(&coverage).ok())
                    .any(|(_, d)| d.derivation == expected.id() && d.coverage == coverage.id()),
                "closed claim discharges only through actual assessed coverage"
            );
            assert!(
                analysis::summary::coverage::assess(
                    &expectation,
                    &[],
                    output.outcome.status,
                    output.outcome.reason,
                    budget
                )
                .is_err()
            );
        }
        assert!(output.call_members.iter().any(|m| m.reason.is_some()));
        if members > 1 {
            assert!(
                conclusions
                    .standings
                    .iter()
                    .any(|s| s.standing == execution::summary_consequences::MemberStanding::Open)
                    || depth == 0
            );
        }
        assert!(
            execution::summary_consequences::derive(
                &summary,
                &output,
                &invocation,
                &d,
                Profile::Behavioral,
                &resources::ResourceBudget::fixed(1).unwrap()
            )
            .is_err()
        );
        let unrequested =
            produce(&summary, &invocation, &d, Profile::Catalog, &graph, budget).unwrap();
        let catalog = execution::summary_consequences::derive(
            &summary,
            &unrequested,
            &invocation,
            &d,
            Profile::Catalog,
            budget,
        )
        .unwrap();
        assert_eq!(catalog.conclusions.len(), 1);
        assert_eq!(
            catalog.conclusions.iter().next().unwrap().verdict,
            obligation::Verdict::NotAnalyzed
        );
        assert!(catalog.proofs.is_empty() && catalog.pending.is_empty());
    }
}
