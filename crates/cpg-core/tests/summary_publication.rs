//! Semantic compiler owners consume completed native and normalized streams.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
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
    let runs: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_runs").await;
    assert!(runs > 0);
    let proofs: i64 =
        catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_transfer_witnesses").await;
    let statuses: Vec<i16> =
        catalog_runtime::query(&fixture, "SELECT status FROM summary_analysis_outcomes").await;
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
    } else if profile == Profile::Catalog {
        assert_eq!(proofs, 0);
        assert!(statuses.iter().all(|s| *s == 3));
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
}
#[tokio::test]
async fn behavioral_summaries_publish_finite_proofs_and_validate() {
    run(Profile::Behavioral).await;
}
#[tokio::test]
async fn catalog_summaries_are_explicitly_not_requested() {
    run(Profile::Catalog).await;
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
