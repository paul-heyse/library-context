//! Semantic compiler owners consume completed native and normalized streams.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
async fn run(profile: Profile, completion: bool) {run_case(profile,completion,false,false).await;}
async fn run_case(profile: Profile, completion: bool, source_calls: bool, enriched: bool) {run_fixture(profile,completion,source_calls,enriched,"execution_channels","cases.py").await;}
async fn run_fixture(profile: Profile, completion: bool, source_calls: bool, enriched: bool, case: &str, file: &str) {
    let root = catalog_runtime::root(case);
    let fixture = catalog_runtime::compile(case,profile,Frontier::Analysis,catalog_runtime::settings("cases"),None).await;
    if case == "exact_exception_shapes" {
        let native:Vec<(String,i64)> = catalog_runtime::query(&fixture, "SELECT s.name,count(a.id) FROM provider_symbols s LEFT JOIN class_ancestry_observations a ON a.class=s.id WHERE s.name IN ('ValueError','RuntimeError','TypeError','Exception','BaseException') GROUP BY s.name ORDER BY s.name").await;
        let refused:Vec<(i16,i64)> = catalog_runtime::query(&fixture, "SELECT reason,count(*) FROM execution_body_boundaries GROUP BY reason ORDER BY reason").await;
        eprintln!("B4_NATIVE {native:?}; B4_REFUSED {refused:?}");
    }
    let counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM base_expression_evaluations),(SELECT count(*) FROM base_evaluation_boundaries),(SELECT count(*) FROM base_evaluation_runs)").await;
    assert!(counts.2 > 0);
    let statuses: Vec<i16> = catalog_runtime::query(&fixture, "SELECT status FROM base_evaluation_analysis_outcomes").await;
    assert!(!statuses.is_empty());
    if profile == Profile::Behavioral {
        assert!(counts.0 > 0 && counts.1 > 0);
        assert!(statuses.iter().all(|s| *s == 1));
        let entries: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM base_evaluation_sources WHERE kind=1").await;
        if case != "exact_exception_shapes" {
            assert!(entries > 0);
        }
    } else {
        assert_eq!((counts.0, counts.1), (0, 0));
        assert!(statuses.iter().all(|s| *s == 3));
    }
    let read_counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM base_read_observations),(SELECT count(*) FROM base_formal_read_assessments),(SELECT count(*) FROM base_attribute_reads)").await;
    if profile == Profile::Behavioral {
        if case != "exact_exception_shapes" {
            assert!(read_counts.0 > 0 && read_counts.1 > 0 && read_counts.2 > 0);
        }
    } else {
        assert_eq!(read_counts, (0, 0, 0));
    }
    if case == "transfer_alternatives" && profile == Profile::Behavioral {
        let native_negative: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM base_formal_read_assessments WHERE status=1").await;
        assert!(native_negative > 0);
        let dynamic:(i64,i64)=catalog_runtime::one(&fixture, "SELECT count(*),count(*) FILTER (WHERE inspection=0) FROM base_dynamic_access_observations").await;
        eprintln!("B3_DYNAMIC {dynamic:?}");
        for relation in [
            symbols::FunctionTraitSupport::NAME,
            declarations::SymbolDeclarationSupport::NAME,
            declarations::ParameterDeclarationSupport::NAME,
            flow::FlowUseSupport::NAME,
        ] {
            let attribution:Vec<(i16,i16,i16,i64)>=catalog_runtime::query(&fixture, format!("SELECT origin,mode,fidelity,count(*) FROM {relation} GROUP BY 1,2,3 ORDER BY 1,2,3")).await;
            eprintln!("B3_INSPECTION_SUPPORT {relation} {attribution:?}");
        }
        assert!(dynamic.0 > 0 && dynamic.1 > 0);
    }
    if case == "field_read_screen" {
        let counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM base_field_read_assessments WHERE status=1),(SELECT count(*) FROM base_field_read_assessments WHERE status=0),(SELECT count(*) FROM base_global_field_read_assessments WHERE status=1)").await;
        if profile == Profile::Behavioral {
            assert!(counts.0 >= 2 && counts.1 > 0 && counts.2 >= 2);
        } else {
            assert_eq!(counts, (0, 0, 0));
        }
    }
    if completion {
        let counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM base_statement_completions),(SELECT count(*) FROM base_completion_boundaries),(SELECT count(*) FROM base_completion_runs)").await;
        assert!(counts.2 > 0);
        let statuses: Vec<i16> = catalog_runtime::query(&fixture, "SELECT status FROM base_completion_analysis_outcomes").await;
        if profile == Profile::Behavioral {
            let body_counts:(i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM base_source_body_completions),(SELECT count(*) FROM base_source_body_boundaries)").await;
            if case != "exact_exception_shapes" {
                assert!(body_counts.0 > 0 && body_counts.1 > 0);
            }
            if case != "exact_exception_shapes" {
                assert!(counts.0 > 0 && counts.1 > 0);
            }
            assert!(statuses.iter().all(|s| *s == 1));
            let sources: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM base_completion_sources WHERE kind=1").await;
            assert!(sources > 0);
        } else {
            assert_eq!((counts.0, counts.1), (0, 0));
            assert!(statuses.iter().all(|s| *s == 3));
        }
    }
    if source_calls {
        let statuses: Vec<i16> = catalog_runtime::query(&fixture, "SELECT status FROM source_call_analysis_outcomes").await;
        assert!(!statuses.is_empty());
        assert!(
            statuses
                .iter()
                .all(|s| *s == if profile == Profile::Behavioral { 1 } else { 3 })
        );
        let counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM source_call_headers),(SELECT count(*) FROM source_call_boundaries),(SELECT count(*) FROM source_call_runs)").await;
        assert!(counts.2 > 0);
        if profile == Profile::Behavioral {
            if case != "exact_exception_shapes" {
                assert!(counts.0 > 0 && counts.1 > 0);
            }
            let invoked: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM source_call_invocations").await;
            if case != "exact_exception_shapes" {
                assert!(invoked > 0);
            }
        } else {
            assert_eq!((counts.0, counts.1), (0, 0));
        }
    }
    if enriched {
        let statuses: Vec<i16> = catalog_runtime::query(&fixture, "SELECT status FROM enriched_execution_analysis_outcomes").await;
        assert!(!statuses.is_empty());
        assert!(
            statuses
                .iter()
                .all(|s| *s == if profile == Profile::Behavioral { 1 } else { 3 })
        );
        let counts:(i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM statement_executions),(SELECT count(*) FROM execution_sources WHERE kind=2),(SELECT count(*) FROM execution_runs)").await;
        assert!(counts.2 > 0);
        if profile == Profile::Behavioral {
            if case != "exact_exception_shapes" {
                assert!(counts.0 > 0 && counts.1 > 0);
            }
            if case != "exact_exception_shapes" {
                let modeled: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM modeled_call_evaluations").await;
                assert!(modeled > 0);
                let fresh: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM source_execution_invocations").await;
                assert!(fresh > 0);
                let args: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM source_frame_arguments").await;
                assert!(args > 0);
                let defaults: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM definition_evaluations").await;
                assert!(defaults > 0);
                let contexts: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM context_executions").await;
                assert!(contexts > 0);
                let suppressed: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM context_execution_items WHERE suppressed").await;
                assert!(suppressed > 0);
                let bindings: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM context_entry_bindings").await;
                assert!(bindings > 0);
                let retained_bindings:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM context_entry_bindings WHERE release=1 AND entry_actual IS NOT NULL").await;
                assert!(retained_bindings > 0);
            }
        } else {
            assert_eq!((counts.0, counts.1), (0, 0));
        }
    }
    if case == "execution_channels" && profile == Profile::Behavioral && enriched {
        let transitions: Vec<(String,i64,i16,i16,bool)> = catalog_runtime::query(&fixture, "SELECT name.spelling,item.ordinal,before.kind,after.kind,item.suppressed FROM context_executions execution JOIN context_execution_items item ON item.execution=execution.id JOIN execution_outcomes before ON before.id=item.exit_input JOIN execution_outcomes after ON after.id=item.exit_output JOIN entity_refs e ON e.id=execution.owner JOIN callable_entities c ON c.id=e.callable_callable JOIN declaration_observations d ON d.declaration=c.source_declaration JOIN syntax_observations name ON name.occurrence=d.name WHERE name.spelling IN ('with_preserve','with_preserve_return','with_suppress','with_nonmatch','with_multiple','with_finalizer_return') ORDER BY 1,2").await;
        assert_eq!(
            transitions,
            vec![
                ("with_finalizer_return".to_owned(), 0, 1, 1, false),
                ("with_multiple".to_owned(), 0, 0, 0, false),
                ("with_multiple".to_owned(), 1, 2, 0, true),
                ("with_nonmatch".to_owned(), 0, 2, 2, false),
                ("with_preserve".to_owned(), 0, 0, 0, false),
                ("with_preserve_return".to_owned(), 0, 1, 1, false),
                ("with_suppress".to_owned(), 0, 2, 0, true),
            ],
            "Python reverse exits pass the inner suppressed outcome to the outer manager; a finalizer return replaces the exception before exits"
        );
        let unsupported_contexts:i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM context_executions execution JOIN entity_refs e ON e.id=execution.owner JOIN callable_entities c ON c.id=e.callable_callable JOIN declaration_observations d ON d.declaration=c.source_declaration JOIN syntax_observations name ON name.occurrence=d.name WHERE name.spelling IN ('with_unknown_body','with_async','with_missing_target_constructor')").await;
        assert_eq!(
            unsupported_contexts, 0,
            "missing body/constructor evidence and async cleanup never acquire exact context completion"
        );
    }
    if case == "exact_exception_shapes" && profile == Profile::Behavioral && enriched {
        let rows: Vec<(String, i16, Option<i16>, Option<i64>)> = catalog_runtime::query(&fixture, "SELECT so.spelling, o.kind, o.raise_exception, returned.start FROM body_executions b JOIN entity_refs e ON e.id=b.owner JOIN callable_entities c ON c.id=e.callable_callable JOIN declaration_observations d ON d.declaration=c.source_declaration JOIN syntax_observations so ON so.occurrence=d.name JOIN execution_outcomes o ON o.id=b.outcome LEFT JOIN occurrences returned ON returned.id=o.return_site ORDER BY so.spelling").await;
        eprintln!("B4_BODY {rows:?}");
        let expected = [
            ("normal_plain", 1, None),
            ("bare_class", 2, Some(1)),
            ("broad_first", 1, None),
            ("tuple_match", 1, None),
            ("unmatched", 2, Some(1)),
            ("final_return", 1, None),
            ("final_raise", 2, Some(2)),
            ("reraised", 2, Some(1)),
            ("named_disposal", 1, None),
            ("normal_else", 2, Some(2)),
            ("no_else_after_handler", 0, None),
            ("argument_failure", 2, Some(1)),
            ("argument_order", 2, Some(1)),
            ("explicit_cause", 2, Some(1)),
            ("invalid_cause", 2, Some(0)),
        ];
        for (name, kind, exception) in expected {
            assert!(
                rows.iter()
                    .any(|row| (row.0.as_str(), row.1, row.2) == (name, kind, exception)),
                "missing Python expected body {name}: {rows:?}"
            );
        }
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        for (name, returned) in [
            ("broad_first", "return 1"),
            ("tuple_match", "return 3"),
            ("final_return", "return 5"),
        ] {
            let expected = source.find(returned).unwrap() as i64;
            assert!(
                rows.iter()
                    .any(|row| row.0 == name && row.3 == Some(expected)),
                "wrong Python-selected handler/finalizer for {name}: {rows:?}"
            );
        }
        for name in [
            "dynamic_constructor",
            "argument_opaque",
            "shadowed_constructor",
            "unsupported_group",
            "invalid_handler",
            "named_retained",
            "starred_constructor",
            "handler_lookup_failure",
        ] {
            assert!(
                !rows.iter().any(|row| row.0 == name),
                "unsupported body {name} incorrectly admitted"
            );
        }
        // A call in an except test has conditional ty reaching evidence. This source-call
        // contract requires unconditional evidence and retains the located approximation.
        let lookup = source.find("except lookup():").unwrap() as i64 + 7;
        let lookup_end = lookup + 8;
        let lookup_refused: i64 = catalog_runtime::one(&fixture, format!("SELECT count(*) FROM source_call_boundaries b JOIN normalized_call_events e ON e.id=b.event JOIN occurrences o ON o.id=e.site WHERE o.start={lookup} AND o.end={lookup_end} AND b.reason=48")).await;
        assert_eq!(lookup_refused, 1);
    }
}
#[tokio::test]
async fn behavioral_base_evaluation_uses_closed_native_local_sources() {
    run(Profile::Behavioral, false).await;
}
#[tokio::test]
async fn catalog_base_evaluation_is_explicitly_not_requested() {
    run(Profile::Catalog, false).await;
}

#[tokio::test]
async fn behavioral_completion_consumes_actual_closed_base_evaluations() {
    run(Profile::Behavioral, true).await;
}
#[tokio::test]
async fn catalog_completion_is_explicitly_not_requested() {
    run(Profile::Catalog, true).await;
}

#[tokio::test]
async fn behavioral_source_calls_publish_only_replayed_fresh_bindings() {
    run_case(Profile::Behavioral, true, true, false).await;
}
#[tokio::test]
async fn catalog_source_calls_are_explicitly_not_requested() {
    run_case(Profile::Catalog, true, true, false).await;
}

#[tokio::test]
async fn behavioral_enriched_replays_source_outcomes_in_caller_frame() {
    run_case(Profile::Behavioral, true, true, true).await;
}
#[tokio::test]
async fn catalog_enriched_is_explicitly_not_requested() {
    run_case(Profile::Catalog, true, true, true).await;
}

#[tokio::test]
async fn behavioral_read_channels_preserve_native_observation_and_negative_inventory() {
    run_fixture(
        Profile::Behavioral,
        false,
        false,
        false,
        "transfer_alternatives",
        "transferpkg/__init__.py",
    )
    .await;
}
#[tokio::test]
async fn catalog_read_channels_are_not_requested() {
    run_fixture(
        Profile::Catalog,
        false,
        false,
        false,
        "transfer_alternatives",
        "transferpkg/__init__.py",
    )
    .await;
}

#[tokio::test]
async fn behavioral_field_read_screen_preserves_source_model_negatives() {
    run_fixture(
        Profile::Behavioral,
        false,
        false,
        false,
        "field_read_screen",
        "cases.py",
    )
    .await;
}
#[tokio::test]
async fn catalog_field_read_screen_is_not_requested() {
    run_fixture(
        Profile::Catalog,
        false,
        false,
        false,
        "field_read_screen",
        "cases.py",
    )
    .await;
}

#[tokio::test]
async fn exact_exception_handlers_preserve_order_and_runtime_completion() {
    run_fixture(
        Profile::Behavioral,
        true,
        true,
        true,
        "exact_exception_shapes",
        "cases.py",
    )
    .await;
}

