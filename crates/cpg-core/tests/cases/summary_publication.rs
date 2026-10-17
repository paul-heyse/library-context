//! Semantic compiler owners consume completed native and normalized streams.
use crate::catalog_runtime;
use futures::FutureExt;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
async fn run(profile: Profile) {
    run_fixture(profile, "phase4_summaries").await;
}
async fn run_fixture(profile: Profile, case: &str) {
    let root = catalog_runtime::root(case);
    let fixture = catalog_runtime::compile(
        case,
        profile,
        Frontier::Analysis,
        catalog_runtime::settings("cases"),
        None,
    )
    .await;
    let result = std::panic::AssertUnwindSafe(async {
    if profile == Profile::Catalog {
        assert_catalog_compact_summary(&fixture).await;
        return;
    }
    let runs: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_runs").await;
    assert!(runs > 0);
    let proofs: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_transfer_witnesses").await;
    let statuses: Vec<i16> =
        catalog_runtime::query(&fixture, "SELECT status FROM summary_analysis_outcomes").await;
    let uncovered_runs: i64 = catalog_runtime::one(&fixture,
        "SELECT count(*) FROM summary_runs r WHERE NOT EXISTS (SELECT 1 FROM summary_analysis_coverage c WHERE c.invocation=r.invocation)").await;
    assert_eq!(
        uncovered_runs, 0,
        "each Summary frame retains admitted coverage after result discharge and publication"
    );
    let missing_premises: i64 = catalog_runtime::one(&fixture,
        "SELECT count(*) FROM summary_analysis_coverage_premises p LEFT JOIN summary_analysis_coverage c ON c.id=p.coverage WHERE c.id IS NULL").await;
    assert_eq!(
        missing_premises, 0,
        "coverage premises refer to the emitted canonical assessment"
    );
    if profile == Profile::Behavioral {
        assert!(statuses.iter().all(|s| *s == 1));
    }
    if profile == Profile::Behavioral && case == "phase4_summaries" {
        assert_conditional_atom_summary(&fixture, &root).await;
        assert!(proofs > 0);
        assert!(statuses.iter().all(|s| *s == 1));
        let alternatives:Vec<(Vec<u8>,Vec<u8>,i64,i16)>=catalog_runtime::query(&fixture, "SELECT constructor_qualification,reader_qualification,depth,reason FROM summary_symbolic_field_alternatives").await;
        assert_eq!(
            alternatives.len(),
            4,
            "three readers retain the call argument and its separate returned-value qualification"
        );
        assert!(alternatives.iter().all(|(_, _, d, reason)| *d == 2
            && *reason == obligation::ObligationKind::ScopeBoundary as i16));
        let original_qualifications:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_symbolic_field_alternatives a JOIN flow_value_observations v ON v.id=a.value WHERE a.reader_qualification=v.qualification").await;
        assert_eq!(
            original_qualifications, 4,
            "each reader retains its exact native value qualification, including a legitimately unconditional argument"
        );
        let matrix:Vec<(i16,i16,bool,i64)>=catalog_runtime::query(&fixture, "SELECT kind,transfer,through_call,count(*) FROM summary_symbolic_field_alternatives GROUP BY 1,2,3 ORDER BY 1,2,3").await;
        assert_eq!(
            matrix,
            vec![
                (1, 0, false, 1),
                (2, 0, false, 1),
                (2, 1, false, 1),
                (2, 1, true, 1)
            ]
        );
        let readers: i64 = catalog_runtime::one(
            &fixture,
            "SELECT count(DISTINCT link) FROM summary_symbolic_field_alternatives",
        )
        .await;
        assert_eq!(readers, 3);
        let conclusions:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_behavioral_conclusions c JOIN summary_obligation_subjects s ON s.id=c.subject JOIN summary_claims q ON q.id=s.summaryclaim_transfer JOIN summary_symbolic_field_alternatives a ON a.id=q.symbolicfieldassociation_alternative WHERE c.verdict=3 AND c.proof IS NULL AND c.qualification=a.reader_qualification AND c.reason=a.reason").await;
        assert_eq!(conclusions, 4);
    }
    if case == "exact_exception_shapes" {
        let results: Vec<(String, Option<i16>)> = catalog_runtime::query(&fixture, "SELECT spelling.spelling,result.exception FROM summary_exception_outcomes result JOIN entity_refs e ON e.id=result.owner JOIN callable_entities c ON c.id=e.callable_callable JOIN declaration_observations d ON d.declaration=c.source_declaration JOIN syntax_observations spelling ON spelling.occurrence=d.name ORDER BY 1").await;
        eprintln!("B4_SUMMARY {results:?}");
        for (name, exception) in [
            ("bare_class", Some(1)),
            ("broad_first", None),
            ("tuple_match", None),
            ("unmatched", Some(1)),
            ("final_return", None),
            ("final_raise", Some(2)),
            ("reraised", Some(1)),
            ("named_disposal", None),
            ("argument_order", Some(1)),
        ] {
            assert!(
                results
                    .iter()
                    .any(|row| row == &(name.to_owned(), exception)),
                "missing actual finite Summary {name}: {results:?}"
            );
        }
        type StoredException = (
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            Option<i16>,
            i16,
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            i16,
            i16,
        );
        let stored: Vec<StoredException> = catalog_runtime::query(&fixture, "SELECT s.id,s.invocation,s.body,s.input,s.context,s.owner,s.qualification,s.outcome,s.exception,s.status,q.assumptions,q.scope,q.condition,q.modality,q.approximation FROM summary_exception_outcomes s JOIN assertion_qualifications q ON q.id=s.qualification LIMIT 32").await;
        assert!(!stored.is_empty() && stored.len() < 32);
        fn nominal<T>(bytes: Vec<u8>) -> Id<T> {
            serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
                _,
                serde::de::value::Error,
            >::new(bytes.into_iter()))
            .unwrap()
        }
        for (
            record,
            invocation,
            body,
            input,
            context,
            owner,
            qualification,
            outcome,
            exception,
            status,
            assumptions,
            scope,
            condition,
            modality,
            approximation,
        ) in stored
        {
            let result = execution::summary_exceptions::SummaryExceptionOutcome {
                invocation: nominal(invocation),
                body: nominal(body),
                input: nominal(input),
                context: nominal(context),
                owner: nominal(owner),
                qualification: nominal(qualification),
                outcome: nominal(outcome),
                exception: exception
                    .map(|value| serde_json::from_value(serde_json::json!(value)).unwrap()),
                status: serde_json::from_value(serde_json::json!(status)).unwrap(),
            };
            let q = assertion::AssertionQualification {
                context: result.context,
                assumptions: nominal(assumptions),
                scope: nominal(scope),
                condition: nominal(condition),
                modality: serde_json::from_value(serde_json::json!(modality)).unwrap(),
                approximation: serde_json::from_value(serde_json::json!(approximation)).unwrap(),
            };
            assert_eq!(result.id().bytes().as_slice(), record);
            assert_eq!(result.qualification, q.id());
            let packet = serving::BehavioralExceptionPacket::from_canonical(&result, &q).unwrap();
            assert_eq!(packet.exception.0, result.exception);
            assert!(packet.under_body_entry && packet.claim_basis.definitions.is_empty());
            assert_eq!(
                packet.claim_basis.set,
                assumptions::AssumptionSet::empty_id()
            );
            assert_eq!(packet.proof.len(), 2);
            let mut missing = serde_json::to_value(&packet).unwrap();
            missing.as_object_mut().unwrap().remove("claim_basis");
            assert!(serde_json::from_value::<serving::BehavioralExceptionPacket>(missing).is_err());
        }
        for name in [
            "dynamic_constructor",
            "argument_opaque",
            "unsupported_group",
            "shadowed_constructor",
            "invalid_handler",
            "named_retained",
            "starred_constructor",
            "handler_lookup_failure",
        ] {
            assert!(
                !results.iter().any(|row| row.0 == name),
                "unsupported body became Summary absence"
            );
        }
    }
    if case == "stable_capture_shapes" && profile == Profile::Behavioral {
        let rows:Vec<(String,i16,i16,i16,bool)>=catalog_runtime::query(&fixture, "SELECT spelling.spelling, source.kind, q.modality, q.approximation, capture.under_caller_entry FROM captured_entry_bindings capture JOIN captured_value_sources source ON source.id=capture.value_source JOIN entity_refs er ON er.id=capture.caller JOIN callable_entities ce ON ce.id=er.callable_callable JOIN declaration_observations decl ON decl.declaration=ce.source_declaration JOIN syntax_observations spelling ON spelling.occurrence=decl.name JOIN assertion_qualifications q ON q.id=capture.qualification ORDER BY 1").await;
        eprintln!("B2_CAPTURE {rows:?}");
        assert!(rows.iter().any(|r| r.0 == "captured_entry" && r.1 == 0));
        assert!(rows.iter().any(|r| r.0 == "captured_literal" && r.1 == 1));
        assert!(rows.iter().all(|r| r.2 == 0 && r.3 == 0 && r.4));
        let summaries:Vec<(String,i16,i16,i16)>=catalog_runtime::query(&fixture, "SELECT spelling.spelling, k.kind, q.modality, q.approximation FROM summary_capture_witnesses w JOIN summary_transfer_keys k ON k.id=w.transfer JOIN entity_refs er ON er.id=k.owner JOIN callable_entities ce ON ce.id=er.callable_callable JOIN declaration_observations decl ON decl.declaration=ce.source_declaration JOIN syntax_observations spelling ON spelling.occurrence=decl.name JOIN assertion_qualifications q ON q.id=w.qualification JOIN summary_capture_contributions c ON c.witness=w.id JOIN summary_transfer_alternatives a ON a.id=c.alternative JOIN summary_transfer_supports s ON s.assertion=a.id ORDER BY 1").await;
        eprintln!("B2_SUMMARY {summaries:?}");
        assert!(
            summaries
                .iter()
                .any(|r| r == &("captured_entry".into(), 0, 0, 0)),
            "actual supported identity transfer absent"
        );
        assert!(
            summaries
                .iter()
                .any(|r| r == &("captured_literal".into(), 0, 0, 0)),
            "literal capture transfer absent"
        );
        for name in [
            "mutation",
            "call_before_assignment",
            "escaped",
            "delayed",
            "nonlocal_write",
            "global_read",
            "loop_capture",
            "nested_scope",
        ] {
            assert!(
                !rows.iter().any(|r| r.0 == name),
                "unsupported capture hydrated: {name}"
            );
            assert!(
                !summaries.iter().any(|r| r.0 == name),
                "unsupported capture became Summary: {name}"
            );
        }
        let wrong_roots:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_capture_witnesses w JOIN captured_entry_bindings b ON b.id=w.binding JOIN captured_value_sources source ON source.id=b.value_source JOIN summary_transfer_keys k ON k.id=w.transfer JOIN places p ON p.id=k.input JOIN place_roots r ON r.id=p.root WHERE (source.kind=0 AND (r.kind<>9 OR r.entry_declaration IS DISTINCT FROM source.entry_declaration)) OR (source.kind=1 AND (r.kind<>7 OR r.occurrence_occurrence IS DISTINCT FROM source.literal_value))").await;
        assert_eq!(
            wrong_roots, 0,
            "formal and literal sources retain their actual distinct input roots"
        );
        let formal_origin_edges:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM captured_entry_bindings b JOIN captured_value_sources source ON source.id=b.value_source JOIN flow_definition_observations d ON d.id=b.origin JOIN flow_definitions f ON f.id=d.definition JOIN occurrences native ON native.id=f.occurrence JOIN occurrences formal ON formal.id=source.entry_declaration JOIN syntax_placements p ON p.occurrence=native.id AND p.parent=formal.id WHERE source.kind=0 AND formal.syntax_kind=80 AND formal.role=2 AND native.syntax_kind=93 AND native.id<>formal.id AND native.source=formal.source AND native.start=formal.start AND native.\"end\"=formal.\"end\" AND p.field=24 AND p.ordinal=0").await;
        assert_eq!(
            formal_origin_edges, 1,
            "same-range Identifier origin must use the actual Parameter child edge while retaining its distinct formal root"
        );
        let forged:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM entry_value_witnesses w JOIN occurrence_ownership owner ON owner.occurrence=w.access WHERE owner.entity<>w.owner").await;
        assert_eq!(forged, 0, "capture never forges a native caller read");
    }
    }).catch_unwind().await;
    fixture.complete(result).await;
}
#[tokio::test]
async fn behavioral_summaries_publish_finite_proofs_and_validate() {
    run(Profile::Behavioral).await;
}
#[tokio::test]
async fn catalog_summaries_are_explicitly_not_requested() {
    // NotRequested is profile-owned; unrelated Behavioral examples add no premise.
    run_fixture(Profile::Catalog, "scoped_summary_preparation").await;
}

async fn assert_catalog_compact_summary(fixture: &catalog_runtime::Fixture) {
    use analysis::owner::AnalysisOwner;
    use std::collections::BTreeSet;
    use uniform_outcome::{
        CoverageAuthority, ScopedUniformOutcome, UniformKind, UniformNotRequested,
    };
    catalog_runtime::assert_compact_owners(
        fixture,
        &[
            AnalysisOwner::Local,
            AnalysisOwner::Base,
            AnalysisOwner::Completion,
            AnalysisOwner::SourceCalls,
            AnalysisOwner::Enriched,
            AnalysisOwner::Models,
            AnalysisOwner::Summary,
            AnalysisOwner::Embedding,
            AnalysisOwner::Analytic,
        ],
    )
    .await;
    let workspace = &fixture.workspace;
    let budget = workspace.budget();
    let descriptors = crate::scoped_summary::rows::<UniformNotRequested>(
        &fixture.session,
        UniformNotRequested::NAME,
        budget,
    )
    .await;
    let summaries = descriptors
        .iter()
        .filter(|row| row.owner == AnalysisOwner::Summary.name())
        .collect::<Vec<_>>();
    let runs = crate::scoped_summary::rows::<attribution::ProviderRun>(
        &fixture.session,
        attribution::ProviderRun::NAME,
        budget,
    )
    .await;
    let frames = runs
        .iter()
        .map(|run| (run.input, run.context))
        .collect::<BTreeSet<_>>();
    assert!(
        !frames.is_empty(),
        "the real source fixture must establish actual provider frames"
    );
    assert_eq!(
        summaries.len(),
        frames.len(),
        "one compact Summary owner per actual frame"
    );
    assert_eq!(
        summaries
            .iter()
            .map(|row| (row.input, row.context))
            .collect::<BTreeSet<_>>(),
        frames
    );
    let definitions = crate::scoped_summary::rows::<analysis::AnalysisDefinition>(
        &fixture.session,
        analysis::AnalysisDefinition::NAME,
        budget,
    )
    .await;
    let selected = definitions
        .iter()
        .filter(|row| row.method == analysis::AnalysisMethod::Summaries)
        .collect::<Vec<_>>();
    assert_eq!(
        selected.len(),
        1,
        "one actual selected Summary method definition"
    );
    let definition = selected[0].content_digest();
    let artifacts = crate::scoped_summary::rows::<source::SourceArtifact>(
        &fixture.session,
        source::SourceArtifact::NAME,
        budget,
    )
    .await;
    let uses = crate::scoped_summary::rows::<input::ArtifactUse>(
        &fixture.session,
        input::ArtifactUse::NAME,
        budget,
    )
    .await;
    let completed = workspace.completed_relations().unwrap();
    let access = workspace
        .inputs(
            "catalog-summary-compact",
            Profile::Catalog,
            completed.iter().map(|relation| relation.name()),
        )
        .unwrap();
    let sources =
        analysis::sources::CapturedSources::capture(Profile::Catalog, access.snapshots(), budget)
            .unwrap();
    // This consumer obtains actual selected native owner proofs. Descriptor rows
    // alone cannot establish a compact NotRequested result.
    let mut coverage = workspace
        .coverage_admission(&access, &sources)
        .await
        .unwrap();
    macro_rules! expected_inputs {($($field:ident:$ty:ty,)*)=>{$(
        let permit = access.read::<$ty>().unwrap();
        let mut batches = access.relation::<$ty>().unwrap().batches_async().await.unwrap();
        while let Some(batch) = batches.next_async().await {
            coverage.visit(&permit, &batch.unwrap()).unwrap();
        }
    )*};}
    lctx_model::expected_domain_inputs!(expected_inputs);
    for descriptor in summaries {
        assert_eq!(descriptor.kind, UniformKind::AnalysisOwner);
        assert_eq!(descriptor.family, None);
        let domain = coverage
            .uniform_analysis(
                AnalysisOwner::Summary,
                descriptor.input,
                descriptor.context,
                &definitions,
                budget,
            )
            .unwrap()
            .expect("actual admitted compact Summary owner");
        assert_eq!(
            domain.descriptor_row().unwrap(),
            *descriptor,
            "the admitted owner retains its exact declaration, ordered sources and selected definitions"
        );
        let input_scope = source::CoverageScope::Input {
            input: descriptor.input,
        }
        .id();
        let mut expected = BTreeSet::from([(input_scope, definition)]);
        for usage in uses.iter().filter(|usage| usage.input == descriptor.input) {
            let artifact = artifacts.get(usage.artifact).unwrap();
            let class = admission::ArtifactClass::PythonSource;
            if class.requested_by(usage.role)
                && admission::ArtifactClass::of(&artifact.path) == Some(class)
            {
                expected.insert((
                    source::CoverageScope::Artifact {
                        artifact: usage.artifact,
                    }
                    .id(),
                    definition,
                ));
            }
        }
        assert!(
            expected.len() > 1,
            "fixture Summary domain includes Python source and input projection scopes"
        );
        assert!(!domain.is_explicit_empty());
        assert_eq!(domain.members().copied().collect::<BTreeSet<_>>(), expected);
        assert_eq!(descriptor.members, expected.len() as i64);
        let authority = CoverageAuthority::Uniform {
            descriptor: descriptor.id(),
        };
        assert_eq!(domain.authority(), authority);
        for (scope, definition) in expected {
            assert_eq!(
                domain.scoped(scope, definition),
                ScopedUniformOutcome::NotRequested(authority)
            );
            let evidence = domain.evidence(scope, definition).unwrap();
            assert_eq!(
                (
                    evidence.descriptor(),
                    evidence.scope(),
                    evidence.context(),
                    evidence.family(),
                    evidence.definition()
                ),
                (descriptor.id(), scope, descriptor.context, None, definition)
            );
        }
        let foreign = ContentHash::of(b"unselected-summary-definition");
        assert_ne!(foreign, definition);
        assert_eq!(
            domain.scoped(input_scope, foreign),
            ScopedUniformOutcome::OutsideDomain,
            "compact authority does not extend to an unselected method definition"
        );
    }
    macro_rules! ordinary_publication {($($record:ident,)*)=>{$(
        assert_eq!(workspace.completed::<analysis::summary::$record>().unwrap().rows(), 0,
            "Catalog compact Summary has no per-method {} rows", <analysis::summary::$record>::NAME);
    )*};}
    lctx_model::analysis_publication!(ordinary_publication);
    macro_rules! requested_outputs {($($field:ident:$ty:ty,)*)=>{$(
        assert_eq!(workspace.completed::<$ty>().unwrap().rows(), 0,
            "Catalog Summary request-only output {} is empty", <$ty>::NAME);
    )*};}
    lctx_model::summary_outputs!(requested_outputs);
}

#[tokio::test]
async fn exact_exception_summary_projects_only_actual_completed_bodies() {
    run_fixture(Profile::Behavioral, "exact_exception_shapes").await;
}

// Independent source locations distinguish repeated evaluations and premises through real storage.
async fn assert_conditional_atom_summary(
    fixture: &catalog_runtime::Fixture,
    root: &std::path::Path,
) {
    let text = std::fs::read_to_string(root.join("cases.py")).unwrap();
    let decisions:Vec<(i64,Vec<u8>,i16)>=catalog_runtime::query(fixture, "SELECT o.start,d.leaf,d.outcome FROM local_atom_decisions d JOIN flow_test_leaf_observations l ON l.id=d.leaf JOIN occurrences o ON o.id=l.test").await;
    for (function, expected) in [
        ("finite_zero", 0_i16),
        ("finite_one", 1),
        ("uninhabited", 3),
    ] {
        let start = text.find(&format!("def {function}(")).unwrap();
        let test = (start + text[start..].find("if value == 0:").unwrap() + 3) as i64;
        assert!(
            decisions
                .iter()
                .any(|(at, _, outcome)| *at == test && *outcome == expected),
            "{function} exact native decision: {decisions:?}"
        );
    }
    let start = text.find("def finite_repeated(").unwrap();
    let end = text[start..].find("def effectful_repeated(").unwrap() + start;
    let repeated = decisions
        .iter()
        .filter(|(at, _, _)| (*at as usize) >= start && (*at as usize) < end)
        .map(|(_, leaf, _)| leaf)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        repeated.len(),
        2,
        "repeated equal source predicates remain two evaluations"
    );
    let forbidden:i64=catalog_runtime::one(fixture, "SELECT count(*) FROM local_atom_restrictions r JOIN local_atom_decisions d ON d.id=r.decision WHERE d.outcome IN (2,3,4)").await;
    assert_eq!(
        forbidden, 0,
        "mixed, uninhabited and refused answers never prune"
    );
    let alternatives:Vec<(i64,i64,i16)>=catalog_runtime::query(fixture, "SELECT o.start,b.count,n.kind FROM summary_transfer_alternatives a JOIN summary_transfer_keys k ON k.id=a.transfer JOIN entity_refs e ON e.id=k.owner JOIN callable_entities callable ON callable.id=e.callable_callable JOIN occurrences o ON o.id=callable.source_declaration JOIN assertion_qualifications q ON q.id=a.qualification JOIN assumption_sets b ON b.id=q.assumptions JOIN conditions c ON c.id=q.condition JOIN condition_nodes n ON n.id=c.root").await;
    let start = text.find("def finite_zero(").unwrap() as i64;
    let end = text.find("def finite_one(").unwrap() as i64;
    let zero = alternatives
        .iter()
        .filter(|(at, _, _)| *at >= start && *at < end)
        .collect::<Vec<_>>();
    assert!(
        zero.iter()
            .any(|(_, basis, condition)| *basis == 1 && *condition == 1),
        "typing premise must change a real Summary guard to true: {zero:?}"
    );
    assert!(
        zero.iter().any(|(_, basis, _)| *basis == 0),
        "the unconditional runtime alternative survives: {zero:?}"
    );
    // Local alternatives need their own finite consequences even when no composed Summary
    // witness is emitted. Distinguish the provider typing world from the original runtime seed.
    let consequences: Vec<(i64, i64, i16)> = catalog_runtime::query(fixture, "SELECT o.start,b.count,c.verdict FROM summary_behavioral_conclusions c JOIN summary_claim_proofs proof ON proof.id=c.proof AND proof.kind=0 JOIN summary_transfer_premises premise ON premise.id=proof.finite_source AND premise.kind=0 JOIN local_transfer_alternatives a ON a.id=premise.local_alternative JOIN local_transfer_keys k ON k.id=a.transfer JOIN entity_refs e ON e.id=k.owner JOIN callable_entities callable ON callable.id=e.callable_callable JOIN occurrences o ON o.id=callable.source_declaration JOIN assertion_qualifications q ON q.id=c.qualification JOIN assumption_sets b ON b.id=q.assumptions").await;
    let zero_consequences = consequences
        .iter()
        .filter(|(at, _, _)| *at >= start && *at < end)
        .collect::<Vec<_>>();
    assert!(
        zero_consequences.iter().any(|(_, basis, verdict)| {
            *basis == 1 && *verdict == lctx_model::domain::obligation::Verdict::Established as i16
        }),
        "the exact typing restriction establishes its Local consequence under that premise: {zero_consequences:?}"
    );
    assert!(
        zero_consequences.iter().any(|(_, basis, verdict)| {
            *basis == 0 && *verdict == lctx_model::domain::obligation::Verdict::Conditional as i16
        }),
        "the original runtime guard remains a separate conditional Local consequence: {zero_consequences:?}"
    );
    let start = text.find("def effectful_repeated(").unwrap() as i64;
    let end = text.find("def nonconforming_runtime(").unwrap() as i64;
    assert!(
        !alternatives
            .iter()
            .any(|(at, basis, _)| *at >= start && *at < end && *basis > 0),
        "effectful predicate calls cannot inherit a parameter truth premise"
    );
    assert!(
        !consequences
            .iter()
            .any(|(at, basis, _)| *at >= start && *at < end && *basis > 0),
        "effectful predicate calls cannot acquire a typing-qualified Local consequence"
    );
}

#[tokio::test]
async fn capture_timing_stable_entries_reach_actual_finite_summaries() {
    run_fixture(Profile::Behavioral, "stable_capture_shapes").await;
}

// This oracle comes from the fixture's two explicit return statements. Comparing two
// invocations of produce alone would preserve a shared port/guard caching defect.
pub(crate) async fn assert_repeated_prepared_call_pairs(fixture: &catalog_runtime::Fixture) {
    use std::collections::BTreeSet;
    let source = std::fs::read_to_string(
        catalog_runtime::root("scoped_summary_preparation").join("cases.py"),
    )
    .unwrap();
    let function = source.find("def guarded_pair_relay(").unwrap();
    let value = (function + source[function..].find("value").unwrap()) as i64;
    let other = (function + source[function..].find("other").unwrap()) as i64;
    let call = source.find("fresh(value, other, gate)").unwrap() as i64;
    let attempts: Vec<Vec<u8>> = catalog_runtime::query(fixture, format!(
        "SELECT DISTINCT a.id FROM call_binding_attempts a JOIN normalized_call_events event ON event.id=a.event JOIN occurrence_ownership owner ON owner.id=event.owner JOIN occurrences site ON site.id=owner.occurrence WHERE site.start={call} AND a.outcome=0"
    )).await;
    assert_eq!(
        attempts.len(),
        1,
        "one actual admitted call is repeatedly composed"
    );
    // Finite witnesses below and independent endpoint assertions test the actual
    // product. Rejected pair enumeration is no longer a mandatory publication.
    let witnesses: Vec<(i64, Vec<u8>, i16, Vec<u8>)> = catalog_runtime::query(fixture, format!(
        "SELECT entry.start,q.condition,node.kind,w.id FROM summary_transfer_witnesses w JOIN call_binding_attempts a ON a.id=w.attempt JOIN normalized_call_events event ON event.id=a.event JOIN occurrence_ownership owner ON owner.id=event.owner JOIN occurrences site ON site.id=owner.occurrence JOIN summary_transfer_keys k ON k.id=w.transfer JOIN places input ON input.id=k.input JOIN place_roots root ON root.id=input.root JOIN occurrences entry ON entry.id=root.entry_declaration JOIN assertion_qualifications q ON q.id=w.qualification JOIN conditions c ON c.id=q.condition JOIN condition_nodes node ON node.id=c.root WHERE site.start={call} AND root.kind=9"
    )).await;
    assert_eq!(
        witnesses.iter().map(|w| w.0).collect::<BTreeSet<_>>(),
        BTreeSet::from([value, other]),
        "only the two explicit data ports transfer; gate is a condition"
    );
    assert_eq!(
        witnesses
            .iter()
            .map(|w| &w.1)
            .collect::<BTreeSet<_>>()
            .len(),
        2,
        "the true and false callee branches keep distinct rebased guards across preparation reuse"
    );
    assert!(
        witnesses.iter().all(|w| w.2 == 2),
        "neither guarded result becomes unconditional: {witnesses:?}"
    );
    let guards: Vec<(i64, Vec<u8>, i16, i16)> = catalog_runtime::query(fixture, format!(
        "SELECT DISTINCT entry.start,node.branch_atom,low.kind,high.kind FROM summary_transfer_witnesses w JOIN call_binding_attempts a ON a.id=w.attempt JOIN normalized_call_events event ON event.id=a.event JOIN occurrence_ownership owner ON owner.id=event.owner JOIN occurrences site ON site.id=owner.occurrence JOIN summary_transfer_keys k ON k.id=w.transfer JOIN places input ON input.id=k.input JOIN place_roots root ON root.id=input.root JOIN occurrences entry ON entry.id=root.entry_declaration JOIN assertion_qualifications q ON q.id=w.qualification JOIN conditions c ON c.id=q.condition JOIN condition_nodes node ON node.id=c.root JOIN condition_nodes low ON low.id=node.branch_low JOIN condition_nodes high ON high.id=node.branch_high WHERE site.start={call} AND root.kind=9"
    )).await;
    assert_eq!(
        guards
            .iter()
            .map(|g| (g.0, g.2, g.3))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([(value, 0, 1), (other, 1, 0)]),
        "literal finite meaning: flag is None carries value; flag is not None carries other"
    );
    assert_eq!(
        guards.iter().map(|g| &g.1).collect::<BTreeSet<_>>().len(),
        1,
        "both branch answers restate the same call's flag evaluation"
    );
    let residuals: i64 = catalog_runtime::one(fixture, format!(
        "SELECT count(*) FROM summary_residuals r JOIN normalized_call_events event ON event.id=r.event JOIN occurrence_ownership owner ON owner.id=event.owner JOIN occurrences site ON site.id=owner.occurrence WHERE site.start={call}"
    )).await;
    assert_eq!(
        residuals, 0,
        "complete finite port coverage does not manufacture exhaustion"
    );
    let unaccounted: i64 = catalog_runtime::one(fixture,
        "SELECT count(*) FROM summary_runs r WHERE r.work<=0 OR r.proofs<=0 OR r.residuals<>(SELECT count(*) FROM summary_residuals residual WHERE residual.invocation=r.invocation)"
    ).await;
    assert_eq!(
        unaccounted, 0,
        "actual Summary work, proofs and residual accounting remain published"
    );
    assert_unsupported_truthy_call_pairs(fixture, &source).await;
}

// The original bare truthiness guard remains an explicit refusal, independently of the
// supported identity test above. Both source return arms must actually reach composition.
async fn assert_unsupported_truthy_call_pairs(fixture: &catalog_runtime::Fixture, source: &str) {
    use std::collections::BTreeSet;
    let outer = source.find("def unsupported_truthy_pair_relay(").unwrap();
    let inner = source.find("def truthy_fresh(").unwrap();
    let parameter =
        |function: usize, name: &str| (function + source[function..].find(name).unwrap()) as i64;
    let value = parameter(outer, "value");
    let other = parameter(outer, "other");
    let flag = parameter(inner, "flag");
    let test = (inner + source[inner..].find("if flag:").unwrap() + 3) as i64;
    let call = source.find("truthy_fresh(value, other, gate)").unwrap() as i64;
    let unsupported = obligation::ObligationKind::ConditionTransferUnsupported as i16;

    let atoms: Vec<(Vec<u8>, i16, i16, i64)> = catalog_runtime::query(fixture, format!(
        "SELECT DISTINCT atom.id,predicate.kind,root.kind,formal.start FROM evaluation_atoms atom JOIN predicates predicate ON predicate.id=atom.predicate JOIN occurrences evaluation ON evaluation.id=atom.evaluation JOIN places operand ON operand.id=atom.operand JOIN place_roots root ON root.id=operand.root JOIN occurrences formal ON formal.id=root.formal_declaration JOIN access_paths path ON path.id=operand.path WHERE evaluation.start={test} AND path.first IS NULL AND path.second IS NULL AND NOT path.unknown_suffix"
    )).await;
    assert_eq!(
        atoms.len(),
        1,
        "the original source guard retains one actual formal evaluation: {atoms:?}"
    );
    assert_eq!(
        (atoms[0].1, atoms[0].2, atoms[0].3),
        (4, 0, flag),
        "bare flag stays Truthy over its whole formal, without relabeling it IsNone"
    );
    let attempts: Vec<Vec<u8>> = catalog_runtime::query(fixture, format!(
        "SELECT DISTINCT a.id FROM call_binding_attempts a JOIN normalized_call_events event ON event.id=a.event JOIN occurrences site ON site.id=event.site JOIN source_call_headers header ON header.attempt=a.id AND header.event=event.id JOIN occurrences declaration ON declaration.id=header.declaration WHERE site.start={call} AND declaration.start={inner} AND a.outcome=0"
    )).await;
    assert_eq!(
        attempts.len(),
        1,
        "the unsupported guard still has one admitted fresh source call"
    );
    let witnesses: i64 = catalog_runtime::one(fixture, format!(
        "SELECT count(*) FROM summary_transfer_witnesses w JOIN call_binding_attempts a ON a.id=w.attempt JOIN normalized_call_events event ON event.id=a.event JOIN occurrences site ON site.id=event.site WHERE site.start={call}"
    )).await;
    assert_eq!(
        witnesses, 0,
        "unsupported truthiness never manufactures a finite transfer witness"
    );
    let residuals: Vec<i16> = catalog_runtime::query(fixture, format!(
        "SELECT r.reason FROM summary_residuals r JOIN normalized_call_events event ON event.id=r.event JOIN occurrences site ON site.id=event.site WHERE site.start={call}"
    )).await;
    assert!(
        !residuals.is_empty() && residuals.iter().all(|reason| *reason == unsupported),
        "the actual unsupported call retains exact ConditionTransferUnsupported residuals: {residuals:?}"
    );
    let residual_ports: Vec<i64> = catalog_runtime::query(fixture, format!(
        "SELECT DISTINCT entry.start FROM summary_residuals r JOIN normalized_call_events event ON event.id=r.event JOIN occurrences site ON site.id=event.site JOIN places input ON input.id=r.input JOIN place_roots root ON root.id=input.root JOIN occurrences entry ON entry.id=root.entry_declaration WHERE site.start={call} AND root.kind=9"
    )).await;
    assert_eq!(
        residual_ports.into_iter().collect::<BTreeSet<_>>(),
        BTreeSet::from([value, other]),
        "both original data ports retain their refusal boundary"
    );
}

// Expectations below come from actual fixture source: identity/relay return their one input,
// recurse has a guarded base return, and no_base has no finite return seed at all.
// The whole-operation comparison separately checks every emitted record.
pub(crate) async fn assert_recursive_summary_answers(fixture: &catalog_runtime::Fixture) {
    use std::collections::BTreeSet;
    let text =
        std::fs::read_to_string(catalog_runtime::root("scoped_recursive_summary").join("cases.py"))
            .unwrap();
    let start = |name: &str| text.find(&format!("def {name}(")).unwrap() as i64;
    let owner_joins = "JOIN entity_refs owner ON owner.id=k.owner JOIN callable_entities callable ON callable.id=owner.callable_callable JOIN occurrences declaration ON declaration.id=callable.source_declaration";
    // Include the actual roots and empty paths: an entry-to-intermediate or derived field
    // answer is not a substitute for returning the source's original parameter value.
    type Transfer = (i64, i64, i16, i16, Vec<u8>);
    let transfers: Vec<Transfer> = catalog_runtime::query(fixture, format!(
        "WITH alternatives AS (SELECT transfer,qualification,id FROM local_transfer_alternatives UNION ALL SELECT transfer,qualification,id FROM summary_transfer_alternatives), keys AS (SELECT id,owner,input,output,kind FROM local_transfer_keys UNION ALL SELECT id,owner,input,output,kind FROM summary_transfer_keys) SELECT declaration.start,entry.start,k.kind,node.kind,a.id FROM alternatives a JOIN keys k ON k.id=a.transfer {owner_joins} JOIN places input ON input.id=k.input JOIN place_roots root ON root.id=input.root JOIN occurrences entry ON entry.id=root.entry_declaration JOIN access_paths input_path ON input_path.id=input.path JOIN places output ON output.id=k.output JOIN place_roots returned ON returned.id=output.root JOIN access_paths output_path ON output_path.id=output.path JOIN assertion_qualifications q ON q.id=a.qualification JOIN conditions condition ON condition.id=q.condition JOIN condition_nodes node ON node.id=condition.root WHERE root.kind=9 AND returned.kind=2 AND returned.return_callable=callable.source_declaration AND input_path.first IS NULL AND output_path.first IS NULL AND NOT input_path.unknown_suffix AND NOT output_path.unknown_suffix"
    )).await;
    for name in ["identity", "relay", "nested"] {
        let function = start(name);
        let parameter = function + text[function as usize..].find("value").unwrap() as i64;
        let answers = transfers
            .iter()
            .filter(|row| row.0 == function)
            .collect::<Vec<_>>();
        assert!(
            !answers.is_empty(),
            "actual {name} entry-to-return answer absent: {transfers:?}"
        );
        assert!(
            answers
                .iter()
                .all(|row| row.1 == parameter && row.2 == 0 && row.3 == 1),
            "{name} must preserve the original value under true, without invented derived ports: {answers:?}"
        );
    }
    let recursive = transfers
        .iter()
        .filter(|row| row.0 == start("recurse"))
        .collect::<Vec<_>>();
    assert!(
        !recursive.is_empty(),
        "guarded finite base return must survive recursive composition"
    );
    assert!(
        recursive.iter().all(|row| row.2 == 0 && row.3 == 2),
        "a finite recursive return keeps its condition; it does not prove total termination: {recursive:?}"
    );
    assert!(
        !transfers.iter().any(|row| row.0 == start("no_base")),
        "seedless recursion must not invent an entry-to-return transfer"
    );
    let base_test = start("recurse")
        + text[start("recurse") as usize..]
            .find("value is None")
            .unwrap() as i64;
    let base_guards: Vec<(i64, i16, i16, i16)> = catalog_runtime::query(fixture, format!(
        "SELECT evaluation.start,predicate.kind,low.kind,high.kind FROM local_transfer_alternatives a JOIN local_transfer_keys k ON k.id=a.transfer {owner_joins} JOIN places output ON output.id=k.output JOIN place_roots returned ON returned.id=output.root JOIN assertion_qualifications q ON q.id=a.qualification JOIN conditions condition ON condition.id=q.condition JOIN condition_nodes node ON node.id=condition.root JOIN condition_nodes low ON low.id=node.branch_low JOIN condition_nodes high ON high.id=node.branch_high JOIN evaluation_atoms atom ON atom.id=node.branch_atom JOIN predicates predicate ON predicate.id=atom.predicate JOIN occurrences evaluation ON evaluation.id=atom.evaluation WHERE declaration.start={} AND returned.kind=2", start("recurse")
    )).await;
    assert!(
        base_guards.contains(&(base_test, 0, 0, 1)),
        "the independent base-return guard means exactly the source is-None test being true: {base_guards:?}"
    );
    // A composed guard is a distinct evaluation in the declared finite model. Inspect
    // its actual source/binding mapping and the Proven witness support independently of
    // the producer/oracle's shared rebasing operation.
    let recursive_call =
        text.find("return recurse(value)").unwrap() as i64 + "return ".len() as i64;
    type GuardMapping = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, bool);
    let mapped: Vec<GuardMapping> = catalog_runtime::query(fixture, format!(
        "SELECT bound.id,source.id,w.id,condition.id,(p.kind=9 AND p.boundguard_source=source.id AND sp.kind=0 AND bound.id<>source.id AND bound.evaluation=event.site AND bound.operand=s.actual_place AND bound.context=source.context AND bound.context=event.context) FROM guard_substitutions s JOIN evaluation_atoms bound ON bound.id=s.atom JOIN predicates p ON p.id=bound.predicate JOIN evaluation_atoms source ON source.id=s.source_atom JOIN predicates sp ON sp.id=source.predicate JOIN occurrences source_site ON source_site.id=source.evaluation JOIN normalized_call_events event ON event.id=s.event JOIN occurrences call_site ON call_site.id=event.site JOIN source_call_headers h ON h.event=event.id JOIN summary_transfer_witnesses w ON w.attempt=h.attempt JOIN summary_transfer_keys k ON k.id=w.transfer JOIN places output ON output.id=k.output JOIN place_roots returned ON returned.id=output.root JOIN assertion_qualifications q ON q.id=w.qualification JOIN conditions condition ON condition.id=q.condition WHERE h.owner=h.callee AND source_site.start={base_test} AND call_site.start={recursive_call} AND returned.kind=2"
    )).await;
    assert!(
        !mapped.is_empty(),
        "an actual Proven recursive return must use the bound source guard"
    );
    let actual_conditions = crate::scoped_summary::rows::<conditions::Condition>(
        &fixture.session,
        "conditions",
        fixture.workspace.budget(),
    )
    .await;
    let actual_nodes = crate::scoped_summary::rows::<conditions::ConditionNode>(
        &fixture.session,
        "condition_nodes",
        fixture.workspace.budget(),
    )
    .await;
    let nodes = actual_nodes.iter().cloned().collect::<Vec<_>>();
    for (bound, source, witness, condition, correct_mapping) in &mapped {
        assert!(
            *correct_mapping,
            "recursive BoundGuard must retain its exact source, actual operand and call evaluation: {mapped:?}"
        );
        assert_ne!(
            bound, source,
            "callee and caller evaluations are not relabelled as the same atom"
        );
        let condition = actual_conditions
            .iter()
            .find(|row| row.id().bytes().as_slice() == condition)
            .unwrap();
        let diagram = conditions::Diagram::from_records(condition, &nodes).unwrap();
        assert!(
            diagram
                .support()
                .iter()
                .any(|atom| atom.bytes().as_slice() == bound),
            "Proven recursive witness {witness:?} must actually depend on its mapped BoundGuard"
        );
    }
    let opaque_start = start("opaque_two");
    let left = opaque_start + text[opaque_start as usize..].find("left()").unwrap() as i64;
    let right = opaque_start + text[opaque_start as usize..].find("right()").unwrap() as i64;
    let opaque_atoms: Vec<(i64, Vec<u8>)> = catalog_runtime::query(fixture, format!(
        "SELECT test.start,leaf.atom FROM flow_test_leaf_observations leaf JOIN occurrences test ON test.id=leaf.test WHERE test.start IN ({left},{right})"
    )).await;
    assert_eq!(
        opaque_atoms
            .iter()
            .map(|row| row.0)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([left, right])
    );
    assert_eq!(
        opaque_atoms
            .iter()
            .map(|row| &row.1)
            .collect::<BTreeSet<_>>()
            .len(),
        2,
        "the two effectful source predicates remain distinct evaluations before recursive guard rebasing"
    );

    // Both named recursive callables must inhabit their actual singleton SCC, tied to the
    // admitted self-call header, rather than merely observing some nonempty SCC elsewhere.
    let components: Vec<(i64, Vec<u8>, bool, i64)> = catalog_runtime::query(fixture,
        "SELECT declaration.start,c.id,c.closed,(SELECT count(*) FROM summary_component_members actual WHERE actual.component=c.id) FROM summary_components c JOIN summary_component_members m ON m.component=c.id JOIN entity_refs owner ON owner.id=m.entity JOIN callable_entities callable ON callable.id=owner.callable_callable JOIN occurrences declaration ON declaration.id=callable.source_declaration JOIN source_call_headers header ON header.owner=owner.id AND header.callee=owner.id"
    ).await;
    for name in ["recurse", "no_base"] {
        let ids = components
            .iter()
            .filter(|row| row.0 == start(name))
            .map(|row| &row.1)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            ids.len(),
            1,
            "one actual self-call SCC for {name}: {components:?}"
        );
        assert!(
            components
                .iter()
                .filter(|row| row.0 == start(name))
                .all(|row| row.3 == 1),
            "{name}'s actual SCC has exactly one member: {components:?}"
        );
    }
    assert_eq!(
        components
            .iter()
            .filter(|row| [start("recurse"), start("no_base")].contains(&row.0))
            .map(|row| &row.1)
            .collect::<BTreeSet<_>>()
            .len(),
        2,
        "recurse and no_base must occupy distinct SCCs"
    );
    assert!(
        components
            .iter()
            .filter(|row| row.0 == start("no_base"))
            .all(|row| row.2),
        "the empty finite seedless frontier converges, without inventing a return proof"
    );

    let exhausted: Vec<(i64, i16)> = catalog_runtime::query(fixture,
        "SELECT declaration.start,r.reason FROM summary_residuals r JOIN normalized_call_events event ON event.id=r.event JOIN occurrence_ownership site ON site.id=event.owner JOIN entity_refs owner ON owner.id=site.entity JOIN callable_entities callable ON callable.id=owner.callable_callable JOIN occurrences declaration ON declaration.id=callable.source_declaration"
    ).await;
    let bounded = |reason: i16| {
        [
            obligation::ObligationKind::SummaryDepthLimit as i16,
            obligation::ObligationKind::SummaryProofLimit as i16,
            obligation::ObligationKind::SummaryPairWorkLimit as i16,
        ]
        .contains(&reason)
    };
    for name in ["opaque", "opaque_two"] {
        assert!(
            exhausted
                .iter()
                .any(|row| row.0 == start(name) && bounded(row.1)),
            "{name}'s distinct rebased recursive guards must expose their own bounded suffix: {exhausted:?}"
        );
    }
    assert!(
        !exhausted.iter().any(|row| [
            start("identity"),
            start("relay"),
            start("nested"),
            start("no_base")
        ]
        .contains(&row.0)
            && bounded(row.1)),
        "converged finite or seedless work must not acquire an exhausted suffix: {exhausted:?}"
    );

    type ClosureObservation = (i64, i16, Option<Vec<u8>>, Option<i16>);
    let closures: Vec<ClosureObservation> = catalog_runtime::query(fixture,
        "SELECT declaration.start,conclusion.verdict,conclusion.proof,conclusion.reason FROM summary_claims claim JOIN summary_obligation_subjects subject ON subject.summaryclaim_transfer=claim.id JOIN summary_behavioral_conclusions conclusion ON conclusion.subject=subject.id JOIN normalized_call_events event ON event.id=claim.callclosure_event JOIN occurrence_ownership site ON site.id=event.owner JOIN entity_refs owner ON owner.id=site.entity JOIN callable_entities callable ON callable.id=owner.callable_callable JOIN occurrences declaration ON declaration.id=callable.source_declaration JOIN places output ON output.id=claim.callclosure_output JOIN place_roots returned ON returned.id=output.root WHERE claim.kind=1 AND returned.kind=2"
    ).await;
    for name in ["relay", "nested"] {
        assert!(
            closures.iter().any(|row| row.0 == start(name)
                && row.1 == obligation::Verdict::Established as i16
                && row.2.is_some()
                && row.3.is_none()),
            "{name}'s finite call must earn a complete closure proof: {closures:?}"
        );
    }
    assert!(
        !closures
            .iter()
            .any(|row| row.0 == start("no_base") && row.2.is_some()),
        "no finite return seed can produce a recursive return closure proof"
    );

    let recursive_proofs: i64 = catalog_runtime::one(fixture, format!(
        "SELECT count(*) FROM summary_transfer_witnesses witness JOIN source_call_headers header ON header.attempt=witness.attempt JOIN entity_refs owner ON owner.id=header.owner JOIN callable_entities callable ON callable.id=owner.callable_callable JOIN occurrences declaration ON declaration.id=callable.source_declaration WHERE header.owner=header.callee AND declaration.start={}", start("recurse")
    )).await;
    assert!(
        recursive_proofs > 0,
        "the guarded base seed must actually compose through the self call"
    );
    let unaccounted: i64 = catalog_runtime::one(fixture,
        "SELECT count(*) FROM summary_runs r WHERE r.components<>(SELECT count(*) FROM summary_components c WHERE c.invocation=r.invocation) OR r.work<=0 OR r.proofs<>((SELECT count(*) FROM summary_transfer_witnesses w WHERE w.invocation=r.invocation)+(SELECT count(*) FROM summary_path_witnesses w WHERE w.invocation=r.invocation)+(SELECT count(*) FROM summary_capture_witnesses w WHERE w.invocation=r.invocation)) OR r.residuals<>(SELECT count(*) FROM summary_residuals residual WHERE residual.invocation=r.invocation)"
    ).await;
    assert_eq!(
        unaccounted, 0,
        "published SCC, proof and residual totals match actual canonical records exactly"
    );
}
