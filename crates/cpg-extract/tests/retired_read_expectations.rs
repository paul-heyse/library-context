//! Independent dormant P4 answers over actual pinned native providers and the typed Base owner.
#[path = "fixtures/transfer_composition.rs"]
mod fixture;
#[path = "fixtures/read_diagnostics.rs"]
mod read_diagnostics;
#[path = "fixtures/source_data.rs"]
mod source_fixture;
use lctx_model::domain::{
    analysis,
    execution::{self, read_channels::*},
    normalized::entities::*,
    source::Occurrence,
    *,
};

fn text(f: &fixture::NativeFixture, id: Id<Occurrence>) -> String {
    let row = f.data.occurrences.get(id).unwrap();
    String::from_utf8(f.source_bytes(row.source)[row.start as usize..row.end as usize].to_vec())
        .unwrap()
}

fn reads(f: &fixture::NativeFixture) -> ReadRecords {
    let data = source_fixture::data(f);
    read_from(f, &data)
}

fn read_from(
    f: &fixture::NativeFixture,
    data: &execution::source_call_records::SourceCallData,
) -> ReadRecords {
    let run = data
        .flow
        .runs
        .get(
            data.flow
                .coverage
                .iter()
                .find(|c| c.family == attribution::FactFamily::Flow && c.run.is_some())
                .unwrap()
                .run
                .unwrap(),
        )
        .unwrap();
    let (_, definition) = execution::configuration::base_evaluation();
    let (invocation, _) = analysis::base_evaluation::AnalysisInvocation::new(
        run.input,
        run.context,
        definition.id(),
        None,
        [],
    );
    let roots = data
        .flow
        .artifacts
        .iter()
        .filter(|a| a.input == run.input)
        .map(Record::id)
        .collect();
    produce(&data.evaluation, &data.flow, &invocation, &roots, &f.budget).unwrap()
}

#[tokio::test]
async fn complete_formal_negatives_require_one_exact_callable_body_assessment() {
    let f = fixture::native_from("retired_read_expectations").await;
    let mut data = source_fixture::data(&f);
    let concrete = f.data.callables.iter().find(|c| matches!(c, CallableEntity::Source { declaration, .. } if text(&f, *declaration).starts_with("def run(self, unused):\n        return 1"))).unwrap().id();
    let retained = data
        .evaluation
        .callable_assessments
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let assert_open = |output: ReadRecords| {
        let selected = output.formals.iter().filter(|r| matches!(f.data.refs.get(r.owner), Some(EntityRef::Callable { callable }) if *callable == concrete)).collect::<Vec<_>>();
        assert_eq!(selected.len(), 2);
        assert!(
            selected
                .iter()
                .all(|r| r.status == ReadAssessment::Unknown && r.reason.is_some())
        );
    };
    data.evaluation.callable_assessments = normalized::Rows::new(&f.budget);
    assert_open(read_from(&f, &data));
    for mut row in retained {
        if row.callable == concrete {
            let mut context = f
                .rows::<attribution::AnalysisContext>()
                .into_iter()
                .find(|c| c.id() == row.context)
                .unwrap();
            context.config_digest = ContentHash::of(b"foreign callable body context");
            row.context = context.id();
        }
        data.evaluation.callable_assessments.insert(row).unwrap();
    }
    assert_open(read_from(&f, &data));
}

#[tokio::test]
async fn aliased_abstract_and_protocol_placeholders_never_supply_complete_no_read() {
    let f = fixture::native_from("retired_read_expectations").await;
    let output = reads(&f);
    for (signature, placeholder) in [
        ("def run(self, unused): ...", true),
        ("def dispatch(self, unused): ...", true),
        ("def run(self, unused):\n        return 1", false),
        ("def not_implemented_value(self, unused):", false),
    ] {
        let owner = f.data.callables.iter().find(|c| matches!(c, CallableEntity::Source { declaration, .. } if text(&f, *declaration).contains(signature))).unwrap_or_else(|| panic!("missing {signature}; declarations: {:?}", f.data.callables.iter().filter_map(|c| match c { CallableEntity::Source { declaration, .. } => Some(text(&f, *declaration)), _ => None }).collect::<Vec<_>>())).id();
        let selected = output.formals.iter().filter(|row| matches!(f.data.refs.get(row.owner), Some(EntityRef::Callable { callable }) if *callable == owner)).collect::<Vec<_>>();
        assert_eq!(selected.len(), 2, "{signature}");
        for row in selected {
            if placeholder {
                assert_eq!(
                    row.status,
                    ReadAssessment::Unknown,
                    "placeholder: {signature}: {row:?}"
                );
                assert!(
                    row.reason.is_some(),
                    "placeholder refusal must remain explicit"
                );
            } else {
                assert_eq!(
                    row.status,
                    ReadAssessment::CompleteNoReadUnderModel,
                    "concrete: {signature}: {row:?}"
                );
            }
        }
    }
}

#[tokio::test]
async fn resolved_builtin_twins_shadowing_and_computed_names_preserve_read_soundness() {
    let f = fixture::native_from("retired_attribute_expectations").await;
    let output = reads(&f);
    let data = source_fixture::data(&f);
    let class = |prefix: &str| {
        data.evaluation.classes.iter().find(|c| matches!(c, ClassEntity::Source { declaration } if text(&f, *declaration).starts_with(prefix))).unwrap().id()
    };
    let settings = class("class Settings:");
    for name in ["bare_only", "qualified_only", "aliased_only", "direct_only"] {
        let row = output
            .fields
            .assessments
            .iter()
            .find(|r| r.class == settings && r.name == name)
            .unwrap();
        assert_eq!(row.status, ReadAssessment::ObservedRead, "{name}: {row:?}");
    }
    let unread = output
        .fields
        .assessments
        .iter()
        .find(|r| r.class == settings && r.name == "unread_only")
        .unwrap();
    if unread.status != ReadAssessment::CompleteNoReadUnderModel {
        read_diagnostics::dump(&f, &data, &output);
    }
    assert_eq!(
        unread.status,
        ReadAssessment::CompleteNoReadUnderModel,
        "{unread:?}"
    );
    let dynamic = class("class DynamicSettings:");
    let computed = output
        .dynamic
        .iter()
        .filter(|r| {
            r.declared_class == Some(dynamic)
                && r.kind == execution::read_dynamic::DynamicKind::GetAttr
        })
        .collect::<Vec<_>>();
    assert_eq!(computed.len(), 3);
    for expected in [
        "getattr(self, \"computed_\" + suffix)",
        "getattr(self, \"{}\".format(suffix))",
        "getattr(self, \"\".join(parts))",
    ] {
        assert!(
            computed.iter().any(|r| text(&f, r.site) == expected),
            "missing {expected}"
        );
    }
    let row = output
        .fields
        .assessments
        .iter()
        .find(|r| r.class == dynamic && r.name == "computed_name")
        .unwrap();
    assert_eq!(row.status, ReadAssessment::Unknown);
    assert_eq!(row.reason, Some(obligation::ObligationKind::DynamicAccess));
}

#[tokio::test]
async fn lambda_shadowing_keeps_real_nested_builtin_and_uncertain_parameter_or_rebinding() {
    let f = fixture::native_from("retired_shadow_boundaries").await;
    let output = reads(&f);
    let owner = |row: &execution::read_dynamic::DynamicAccessObservation| {
        let EntityRef::Callable { callable } = f.data.refs.get(row.owner).unwrap() else {
            panic!("noncallable owner")
        };
        let CallableEntity::Source { declaration, .. } = f.data.callables.get(*callable).unwrap()
        else {
            panic!("nonsource owner")
        };
        text(&f, *declaration)
    };
    assert!(output.dynamic.iter().any(
        |r| owner(r).starts_with("def parameter(") && text(&f, r.site) == "getattr(obj, name)"
    ));
    assert!(output.dynamic.iter().any(
        |r| owner(r).starts_with("def reassigned(") && text(&f, r.site) == "getattr(obj, name)"
    ));
    if output
        .dynamic
        .iter()
        .any(|r| owner(r).starts_with("def nested(") && text(&f, r.site) == "getattr(obj, name)")
    {
        read_diagnostics::dump(&f, &source_fixture::data(&f), &output);
    }
    assert!(
        !output.dynamic.iter().any(
            |r| owner(r).starts_with("def nested(") && text(&f, r.site) == "getattr(obj, name)"
        )
    );
    assert!(
        output
            .dynamic
            .iter()
            .any(|r| text(&f, r.site) == "builtins.getattr(item, field)"
                && r.kind == execution::read_dynamic::DynamicKind::GetAttr)
    );
}

#[tokio::test]
async fn lambda_shadow_screen_requires_actual_binding_and_lexical_support() {
    let f = fixture::native_from("retired_attribute_expectations").await;
    for mutation in [
        "binding support",
        "lexical support",
        "inexact binding qualification",
    ] {
        let mut data = source_fixture::data(&f);
        let call = data
            .evaluation
            .call_syntax
            .iter()
            .find(|c| text(&f, c.site) == "getattr(self, \"unread_only\")")
            .unwrap();
        let resolution = data
            .evaluation
            .lexical_resolutions
            .iter()
            .find(|r| {
                r.read == call.callee
                    && data.evaluation.lexical_resolution_supports.iter().any(|s| {
                        s.assertion == r.id()
                            && s.origin == attribution::Origin::AnalyzerAssertion
                            && s.mode == attribution::ExtractionMode::NativeTraversal
                            && s.fidelity == attribution::Fidelity::NativeStructural
                    })
            })
            .unwrap();
        let resolution_id = resolution.id();
        let lexical::LexicalTarget::Binding { event } = data
            .evaluation
            .lexical_targets
            .get(resolution.target)
            .unwrap()
        else {
            panic!("shadow not lexically bound")
        };
        let binding = data
            .evaluation
            .ruff_bindings
            .iter()
            .find(|b| b.event == *event)
            .unwrap()
            .id();
        if mutation == "inexact binding qualification" {
            let mut row = data.evaluation.ruff_bindings.get(binding).unwrap().clone();
            let mut q = data
                .flow
                .qualifications
                .get(row.qualification)
                .unwrap()
                .clone();
            q.approximation = assertion::Approximation::Over;
            row.qualification = q.id();
            data.evaluation.qualifications.insert(q.clone()).unwrap();
            data.flow.qualifications.insert(q).unwrap();
            let retained = data
                .evaluation
                .ruff_bindings
                .iter()
                .filter(|b| b.id() != binding)
                .cloned()
                .collect::<Vec<_>>();
            data.evaluation.ruff_bindings = normalized::Rows::new(&f.budget);
            for prior in retained {
                data.evaluation.ruff_bindings.insert(prior).unwrap();
            }
            // Keep the original native support: it cannot support this changed assertion.
            data.evaluation.ruff_bindings.insert(row).unwrap();
        } else if mutation == "lexical support" {
            let retained = data
                .evaluation
                .lexical_resolution_supports
                .iter()
                .filter(|s| s.assertion != resolution_id)
                .cloned()
                .collect::<Vec<_>>();
            data.evaluation.lexical_resolution_supports = normalized::Rows::new(&f.budget);
            for support in retained {
                data.evaluation
                    .lexical_resolution_supports
                    .insert(support)
                    .unwrap();
            }
        } else {
            let retained = data
                .evaluation
                .ruff_binding_supports
                .iter()
                .filter(|s| s.assertion != binding)
                .cloned()
                .collect::<Vec<_>>();
            data.evaluation.ruff_binding_supports = normalized::Rows::new(&f.budget);
            for support in retained {
                data.evaluation
                    .ruff_binding_supports
                    .insert(support)
                    .unwrap();
            }
        }
        let output = read_from(&f, &data);
        assert!(
            output
                .dynamic
                .iter()
                .any(|r| text(&f, r.site) == "getattr(self, \"unread_only\")"
                    && r.declared_class.is_none())
        );
        let unread = output
            .fields
            .assessments
            .iter()
            .find(|r| r.name == "unread_only")
            .unwrap();
        assert_eq!(unread.status, ReadAssessment::Unknown);
        if unread.reason != Some(obligation::ObligationKind::DynamicAccess) {
            read_diagnostics::dump(&f, &data, &output);
        }
        assert_eq!(
            unread.reason,
            Some(obligation::ObligationKind::DynamicAccess)
        );
    }
}

#[tokio::test]
async fn literal_builtin_field_closure_requires_supported_unmixed_native_target() {
    let f = fixture::native_from("retired_attribute_expectations").await;
    let baseline = source_fixture::data(&f);
    let call = baseline
        .evaluation
        .call_syntax
        .iter()
        .find(|c| text(&f, c.site) == "getattr(self, \"bare_only\")")
        .unwrap()
        .clone();
    let target = baseline
        .evaluation
        .call_targets
        .iter()
        .find(|t| t.site == call.site && t.phase == calls::CallPhase::Call)
        .unwrap()
        .clone();
    assert!(
        baseline
            .evaluation
            .call_target_supports
            .iter()
            .any(|s| s.assertion == target.id()
                && s.origin == attribution::Origin::AnalyzerAssertion
                && s.mode == attribution::ExtractionMode::NativeTraversal
                && s.fidelity == attribution::Fidelity::NativeStructural)
    );
    let assessment = |records: &ReadRecords| {
        records
            .fields
            .assessments
            .iter()
            .find(|r| r.name == "unread_only")
            .unwrap()
            .clone()
    };
    assert_eq!(
        assessment(&read_from(&f, &baseline)).status,
        ReadAssessment::CompleteNoReadUnderModel
    );
    for mutation in [
        "missing target support",
        "foreign target support",
        "mixed target",
        "partial flow",
    ] {
        let mut data = source_fixture::data(&f);
        if mutation == "missing target support" || mutation == "foreign target support" {
            let retained = data
                .evaluation
                .call_target_supports
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            data.evaluation.call_target_supports = normalized::Rows::new(&f.budget);
            for mut support in retained {
                let at_site = data
                    .evaluation
                    .call_targets
                    .get(support.assertion)
                    .is_some_and(|t| t.site == call.site);
                if at_site {
                    if mutation == "missing target support" {
                        continue;
                    }
                    let mut run = data.flow.runs.get(support.run).unwrap().clone();
                    let mut context = f
                        .rows::<attribution::AnalysisContext>()
                        .into_iter()
                        .find(|c| c.id() == run.context)
                        .unwrap();
                    context.config_digest = ContentHash::of(b"foreign literal builtin context");
                    run.context = context.id();
                    support.run = run.id();
                    data.flow.runs.insert(run).unwrap();
                }
                data.evaluation
                    .call_target_supports
                    .insert(support)
                    .unwrap();
            }
        } else if mutation == "mixed target" {
            let destination = calls::CallDestination::Unresolved {
                reason: obligation::ObligationKind::MissingEvidence,
                native: Some(calls::PysaUnresolvedReason::Mixed),
            };
            let mut competing = target.clone();
            competing.destination = destination.id();
            data.evaluation
                .call_destinations
                .insert(destination)
                .unwrap();
            data.evaluation.call_targets.insert(competing).unwrap();
        } else {
            let retained = data.flow.coverage.iter().cloned().collect::<Vec<_>>();
            data.flow.coverage = normalized::Rows::new(&f.budget);
            for mut coverage in retained {
                if coverage.family == attribution::FactFamily::Flow {
                    coverage.status = attribution::CoverageStatus::Partial;
                    coverage.reason = Some(obligation::ObligationKind::IncompleteCoverage);
                    coverage.diagnostic =
                        Some("field closure refusal control: incomplete global flow".into());
                }
                data.flow.coverage.insert(coverage).unwrap();
            }
        }
        let row = assessment(&read_from(&f, &data));
        assert_eq!(row.status, ReadAssessment::Unknown, "{mutation}: {row:?}");
        assert_eq!(
            row.reason,
            Some(obligation::ObligationKind::IncompleteCoverage),
            "{mutation}: {row:?}"
        );
    }
}
