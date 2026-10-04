//! Provider normalization and complete normalized event policies, with preserved independent answers.
#[path = "fixtures/events.rs"]
mod event_fixture;
use lctx_model::domain::normalized::events::{CallPolicy, PhaseGroup};
use lctx_model::domain::{
    assertion::*, attribution::*, calls::*, conditions::Diagram, input::*, source::*, *,
};
use std::collections::{BTreeMap, BTreeSet};

fn occurrence(n: i64) -> Occurrence {
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "m.py".into(), b"call(a, b)").unwrap();
    Occurrence {
        source: source.id(),
        start: n,
        end: n + 1,
        syntax_kind: if n == 0 {
            SyntaxKind::ExprCall
        } else {
            SyntaxKind::ExprName
        },
        role: if n == 0 {
            OccurrenceRole::Call
        } else {
            OccurrenceRole::Argument
        },
        structural_path: vec![n as i32],
    }
}
struct World {
    base: AssertionQualification,
    a: Provider,
    b: Provider,
    context: AnalysisContext,
}
impl World {
    fn new() -> Self {
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"config"),
            environment_digest: ContentHash::of(b"env"),
            lock_digest: None,
        };
        let scope = CoverageScope::Input {
            input: InputRevision::from_entries(vec![]).unwrap().id(),
        };
        let base = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: scope.id(),
            condition: Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let provider = |tool: &str| Provider {
            tool: tool.into(),
            revision: "1".into(),
            build_digest: ContentHash::of(tool.as_bytes()),
        };
        Self {
            base,
            a: provider("pysa"),
            b: provider("other"),
            context,
        }
    }
    fn symbol(&self, provider: &Provider, key: &str, kind: SymbolKind) -> ProviderSymbol {
        ProviderSymbol {
            provider: provider.id(),
            context: self.context.id(),
            module: ProviderModule::Bundled {
                provider: provider.id(),
                bundle: ModuleBundle::Typeshed,
                name: "m".into(),
            }
            .id(),
            native_key: key.into(),
            name: key.into(),
            kind,
        }
    }
}
fn callee(phase: CallPhase, channel: CallChannel, symbol: &ProviderSymbol) -> NativeCallee {
    NativeCallee {
        phase,
        channel,
        destination: CallDestination::Resolved {
            symbol: symbol.id(),
        },
        receiver: ReceiverEvidence {
            passing: Some(ReceiverPassing::NotPassed),
            static_method: None,
            class_method: None,
            actual: None,
        },
        implicit: false,
        modality: Modality::Definite,
        receiver_class: None,
    }
}
struct Report {
    rows: NormalizedSite,
    supports: BTreeMap<Id<CallTarget>, Vec<CallTargetSupport>>,
}
fn report(
    base: &AssertionQualification,
    callees: &[NativeCallee],
    complete: &[(CallChannel, CallPhase)],
    unresolved: Option<ObligationKind>,
) -> Report {
    let w = World::new();
    let (run, _) = ProviderRun::new(
        w.a.id(),
        w.context.id(),
        InputRevision::from_entries(vec![]).unwrap().id(),
        w.context.config_digest,
        [FactFamily::Calls],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: w.a.id(),
        family: FactFamily::Calls,
        name: "calls".into(),
    };
    let complete: BTreeSet<_> = complete
        .iter()
        .map(|(channel, phase)| (channel.id(), *phase))
        .collect();
    let rows = normalize_site(
        base,
        occurrence(0).id(),
        CallOrigin::explicit(),
        callees,
        &complete,
        unresolved.map(|reason| (reason, None)),
    )
    .unwrap();
    let evidence = Evidence::Occurrence {
        occurrence: occurrence(0).id(),
    }
    .id();
    let supports = rows
        .targets
        .iter()
        .map(|t| {
            (
                t.id(),
                vec![CallTargetSupport {
                    assertion: t.id(),
                    run: run.id(),
                    surface: surface.id(),
                    evidence,
                    origin: Origin::AnalyzerAssertion,
                    mode: ExtractionMode::ReportDecode,
                    fidelity: Fidelity::NativeStructural,
                }],
            )
        })
        .collect();
    Report { rows, supports }
}
fn site(reports: &[Report], symbols: &[ProviderSymbol]) -> event_fixture::Evaluated {
    let reports: Vec<_> = reports.iter().map(|r| (&r.rows, &r.supports)).collect();
    let (data, budget) = event_fixture::data(&reports, symbols);
    event_fixture::evaluate(data, budget)
}
fn admitted(site: &event_fixture::Evaluated, policy: CallPolicy) -> usize {
    site.admitted(policy).len()
}

#[test]
fn supported_cross_provider_identity_agreement_is_not_raw_symbol_equality() {
    use lctx_model::domain::normalized::{entities::*, event_normalization};
    let w = World::new();
    let first = w.symbol(&w.a, "native-f", SymbolKind::Function);
    let second = w.symbol(&w.b, "other-native-f", SymbolKind::Function);
    let reports = [
        report(
            &w.base,
            &[callee(CallPhase::Call, CallChannel::Direct, &first)],
            &[(CallChannel::Direct, CallPhase::Call)],
            None,
        ),
        report(
            &w.base,
            &[callee(CallPhase::Call, CallChannel::Direct, &second)],
            &[(CallChannel::Direct, CallPhase::Call)],
            None,
        ),
    ];
    let original = site(&reports, &[first, second.clone()]);
    assert!(!original.assessment().unique);
    let event_fixture::Evaluated {
        mut data, budget, ..
    } = original;
    let declaration = occurrence(17).id();
    let entity = EntityRef::Callable {
        callable: CallableEntity::Source {
            declaration,
            kind: CallableKind::Function,
        }
        .id(),
    };
    data.refs.insert(entity.clone()).unwrap();
    let old: Vec<_> = data.symbol_resolutions.iter().cloned().collect();
    data.symbol_resolutions = normalized::Rows::new(&budget);
    for row in old {
        data.symbol_resolutions
            .insert(SymbolEntityResolution {
                entity: Some(entity.id()),
                reason: EntityReason::DeclarationAgreement,
                ..row
            })
            .unwrap();
    }
    let agreed = event_fixture::evaluate(data, budget);
    assert!(agreed.assessment().unique);
    assert_eq!(agreed.admitted(CallPolicy::Summary).len(), 2);
    event_normalization::validate(&agreed.data, &agreed.output, &agreed.budget).unwrap();
    assert!(agreed.assessment().complete && agreed.assessment().unique);
    let mut output = agreed.output;
    output.alternative_evidence = normalized::Rows::new(&agreed.budget);
    assert!(
        event_normalization::validate(&agreed.data, &output, &agreed.budget).is_err(),
        "omitted duplicate evidence cannot yield a complete token"
    );
}

#[test]
fn an_unresolved_constructor_sibling_and_a_potential_remainder_prevent_summary() {
    let w = World::new();
    let new = w.symbol(&w.a, "new", SymbolKind::Function);
    let init = w.symbol(&w.a, "init", SymbolKind::Method);
    let mut unresolved = callee(CallPhase::Init, CallChannel::Direct, &init);
    unresolved.destination = CallDestination::Unresolved {
        reason: ObligationKind::UnresolvedTarget,
        native: Some(PysaUnresolvedReason::UnexpectedInitMethod),
    };
    let reports = [report(
        &w.base,
        &[
            callee(CallPhase::New, CallChannel::Direct, &new),
            unresolved,
        ],
        &[
            (CallChannel::Direct, CallPhase::New),
            (CallChannel::Direct, CallPhase::Init),
        ],
        None,
    )];
    let result = site(&reports, &[new.clone(), init.clone()]);
    assert!(result.assessment().unresolved);
    assert!(result.admitted(CallPolicy::Summary).is_empty());
    assert_eq!(result.admitted(CallPolicy::Association).len(), 2);
    let potential = NativeCallee {
        modality: Modality::Potential,
        ..callee(CallPhase::Call, CallChannel::Direct, &init)
    };
    let reports = [report(
        &w.base,
        &[
            callee(CallPhase::Call, CallChannel::Direct, &new),
            potential,
        ],
        &[(CallChannel::Direct, CallPhase::Call)],
        None,
    )];
    let result = site(&reports, &[new, init]);
    assert!(!result.assessment().unique);
    assert!(result.admitted(CallPolicy::Summary).is_empty());
    assert_eq!(result.admitted(CallPolicy::Invocation).len(), 1);
    assert_eq!(result.admitted(CallPolicy::Association).len(), 2);
}

#[test]
fn stored_policy_mutation_and_short_resources_refuse_complete_event_admission() {
    use lctx_model::domain::normalized::{event_normalization, events::*};
    let w = World::new();
    let f = w.symbol(&w.a, "f", SymbolKind::Function);
    let reports = [report(
        &w.base,
        &[callee(CallPhase::Call, CallChannel::Direct, &f)],
        &[],
        None,
    )];
    let mut result = site(&reports, &[f]);
    assert!(result.admitted(CallPolicy::Summary).is_empty());
    let assessment = result
        .output
        .policy_assessments
        .iter()
        .find(|a| a.policy == CallPolicy::Summary)
        .unwrap()
        .id();
    let alternative = result.output.alternatives.iter().next().unwrap().id();
    result
        .output
        .admissions
        .insert(CallPolicyAdmission {
            assessment,
            alternative,
        })
        .unwrap();
    assert!(event_normalization::validate(&result.data, &result.output, &result.budget).is_err());
    let budget = lctx_model::domain::resources::ResourceBudget::fixed(64).unwrap();
    assert!(matches!(
        event_normalization::normalize(&result.data, &budget),
        Err(ModelError::Resource { .. })
    ));
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn flow_paths_link_exact_explicit_events_without_merging_distinct_paths_or_contexts() {
    use lctx_model::domain::{flow::*, normalized::events::FlowEventReason};
    let w = World::new();
    let f = w.symbol(&w.a, "f", SymbolKind::Function);
    let reports = [report(
        &w.base,
        &[callee(CallPhase::Call, CallChannel::Direct, &f)],
        &[(CallChannel::Direct, CallPhase::Call)],
        None,
    )];
    let event_fixture::Evaluated {
        mut data, budget, ..
    } = site(&reports, &[f]);
    let place = value::Place {
        root: value::PlaceRoot::Occurrence {
            occurrence: occurrence(1).id(),
        }
        .id(),
        path: value::AccessPath::empty().id(),
    };
    let use_ = FlowUse {
        occurrence: occurrence(1).id(),
        place: place.id(),
    };
    let value = FlowValueObservation {
        qualification: w.base.id(),
        use_: use_.id(),
        sink: occurrence(3).id(),
        kind: FlowSinkKind::Return,
        transfer: transfer::TransferKind::Derived,
        through_call: true,
    };
    for operand in [occurrence(1).id(), occurrence(2).id()] {
        let (path, steps) =
            FlowCallPath::new(&[(occurrence(0).id(), operand, FlowCallOperandRole::Argument)])
                .unwrap();
        for step in steps {
            data.steps.insert(step).unwrap();
        }
        data.paths
            .insert(FlowValuePathObservation {
                qualification: w.base.id(),
                value: value.id(),
                path: path.id(),
            })
            .unwrap();
    }
    let (missing, steps) = FlowCallPath::new(&[(
        occurrence(19).id(),
        occurrence(2).id(),
        FlowCallOperandRole::Callee,
    )])
    .unwrap();
    for step in steps {
        data.steps.insert(step).unwrap();
    }
    data.paths
        .insert(FlowValuePathObservation {
            qualification: w.base.id(),
            value: value.id(),
            path: missing.id(),
        })
        .unwrap();
    let foreign = AnalysisContext {
        config_digest: ContentHash::of(b"foreign flow context"),
        ..w.context
    };
    let foreign_q = AssertionQualification {
        context: foreign.id(),
        ..w.base
    };
    data.qualifications.insert(foreign_q.clone()).unwrap();
    let foreign_value = FlowValueObservation {
        qualification: foreign_q.id(),
        ..value
    };
    let path = data
        .paths
        .iter()
        .find(|p| p.path != missing.id())
        .unwrap()
        .path;
    data.paths
        .insert(FlowValuePathObservation {
            qualification: foreign_q.id(),
            value: foreign_value.id(),
            path,
        })
        .unwrap();
    let result = event_fixture::evaluate(data, budget);
    assert_eq!(result.output.flow_links.len(), 4);
    assert_eq!(
        result
            .output
            .flow_links
            .iter()
            .filter(|l| l.reason == FlowEventReason::ExactEvent)
            .count(),
        2
    );
    assert_eq!(
        result
            .output
            .flow_links
            .iter()
            .filter(|l| l.reason == FlowEventReason::NoReportedEvent)
            .count(),
        2
    );
}

#[test]
fn a_declared_provider_site_without_its_resolution_keeps_the_event_open() {
    let w = World::new();
    let f = w.symbol(&w.a, "f", SymbolKind::Function);
    let reports = [report(
        &w.base,
        &[callee(CallPhase::Call, CallChannel::Direct, &f)],
        &[(CallChannel::Direct, CallPhase::Call)],
        None,
    )];
    let event_fixture::Evaluated {
        mut data, budget, ..
    } = site(&reports, std::slice::from_ref(&f));
    let module = ProviderModule::Acquired {
        module: Module {
            source: occurrence(0).source,
            qualified_name: "m".into(),
        }
        .id(),
    };
    data.provider_modules.insert(module.clone()).unwrap();
    let caller = ProviderCallable::ModuleBody {
        provider: w.b.id(),
        context: w.context.id(),
        module: module.id(),
    };
    data.provider_callables.insert(caller.clone()).unwrap();
    let source = ProviderCallSite {
        qualification: w.base.id(),
        site: occurrence(0).id(),
        origin: CallOrigin::explicit(),
        kind: PysaSiteKind::Regular,
        caller: caller.id(),
        callee: PysaCalleeKind::Call,
        is_attribute: None,
    };
    data.call_sites.insert(source.clone()).unwrap();
    let (run, _) = ProviderRun::new(
        w.b.id(),
        w.context.id(),
        InputRevision::from_entries(vec![]).unwrap().id(),
        w.context.config_digest,
        [FactFamily::Calls],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: w.b.id(),
        family: FactFamily::Calls,
        name: "extra-call-report".into(),
    };
    data.site_supports
        .insert(ProviderCallSiteSupport {
            assertion: source.id(),
            run: run.id(),
            surface: surface.id(),
            evidence: Evidence::Occurrence {
                occurrence: source.site,
            }
            .id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::ReportDecode,
            fidelity: Fidelity::NativeStructural,
        })
        .unwrap();
    let result = event_fixture::evaluate(data, budget);
    assert!(!result.assessment().complete);
    assert!(result.admitted(CallPolicy::Summary).is_empty());
    assert_eq!(result.admitted(CallPolicy::Invocation).len(), 1);
}

#[test]
fn site_uniqueness_is_decided_over_every_resolution_at_the_site() {
    let w = World::new();
    let (f, g) = (
        w.symbol(&w.a, "f", SymbolKind::Function),
        w.symbol(&w.b, "g", SymbolKind::Function),
    );
    let (new, init) = (
        w.symbol(&w.a, "object.__new__", SymbolKind::Function),
        w.symbol(&w.a, "C.__init__", SymbolKind::Method),
    );
    let symbols = vec![f.clone(), g.clone(), new.clone(), init.clone()];
    let direct = CallChannel::Direct;
    let call = [(direct.clone(), CallPhase::Call)];
    // Same provider, two qualified reports naming one symbol: unique, and Summary admits both rows.
    let other = AssertionQualification {
        condition: Diagram::never().id(),
        ..w.base.clone()
    };
    let reports = [
        report(
            &w.base,
            &[callee(CallPhase::Call, direct.clone(), &f)],
            &call,
            None,
        ),
        report(
            &other,
            &[callee(CallPhase::Call, direct.clone(), &f)],
            &call,
            None,
        ),
    ];
    let agreement = site(&reports, &symbols);
    assert!(agreement.assessment().unique && agreement.assessment().complete);
    assert_eq!(admitted(&agreement, CallPolicy::Summary), 2);
    // Different symbols disagree: no Summary, but Dataflow keeps both alternatives.
    let reports = [
        report(
            &w.base,
            &[callee(CallPhase::Call, direct.clone(), &f)],
            &call,
            None,
        ),
        report(
            &w.base,
            &[callee(CallPhase::Call, direct.clone(), &g)],
            &call,
            None,
        ),
    ];
    let disagreement = site(&reports, &symbols);
    assert!(disagreement.assessment().disagreement && !disagreement.assessment().unique);
    assert_eq!(admitted(&disagreement, CallPolicy::Summary), 0);
    assert_eq!(admitted(&disagreement, CallPolicy::Dataflow), 2);
    // C(): __new__ and __init__ form one construction; each phase is unique, so both are summarized.
    let construct = [report(
        &w.base,
        &[
            callee(CallPhase::New, direct.clone(), &new),
            callee(CallPhase::Init, direct.clone(), &init),
        ],
        &[
            (direct.clone(), CallPhase::New),
            (direct.clone(), CallPhase::Init),
        ],
        None,
    )];
    let constructed = site(&construct, &symbols);
    assert_eq!(constructed.assessment().group, Some(PhaseGroup::Construct));
    assert_eq!(admitted(&constructed, CallPolicy::Summary), 2);
    // x(): a plain call and a construction are two events; nothing is summarized.
    let mixed = [report(
        &w.base,
        &[
            callee(CallPhase::Call, direct.clone(), &f),
            callee(CallPhase::Init, direct.clone(), &init),
        ],
        &[
            (direct.clone(), CallPhase::Call),
            (direct.clone(), CallPhase::Init),
        ],
        None,
    )];
    assert_eq!(admitted(&site(&mixed, &symbols), CallPolicy::Summary), 0);
    // An incomplete resolution is never summarized.
    let incomplete = [report(
        &w.base,
        &[callee(CallPhase::Call, direct.clone(), &f)],
        &[],
        None,
    )];
    let partial = site(&incomplete, &symbols);
    assert!(!partial.assessment().complete);
    assert_eq!(admitted(&partial, CallPolicy::Summary), 0);
    assert_eq!(admitted(&partial, CallPolicy::Dataflow), 1);
}

#[test]
fn higher_order_potential_and_unresolved_alternatives_never_become_direct_calls() {
    let w = World::new();
    let (map, f) = (
        w.symbol(&w.a, "map", SymbolKind::Function),
        w.symbol(&w.a, "f", SymbolKind::Function),
    );
    let symbols = vec![map.clone(), f.clone()];
    let higher = CallChannel::HigherOrder { argument_index: 0 };
    // map(f, xs): the site is unique on map; f is a higher-order alternative.
    let reports = [report(
        &w.base,
        &[
            callee(CallPhase::Call, CallChannel::Direct, &map),
            callee(CallPhase::Call, higher.clone(), &f),
        ],
        &[
            (CallChannel::Direct, CallPhase::Call),
            (higher, CallPhase::Call),
        ],
        None,
    )];
    let mapped = site(&reports, &symbols);
    assert!(mapped.assessment().unique);
    assert_eq!(
        mapped.phase_targets()[&CallPhase::Call],
        BTreeSet::from([event_fixture::entity(&map).id()])
    );
    let dataflow = mapped.admitted(CallPolicy::Dataflow);
    assert_eq!(dataflow.len(), 1, "f is never a direct dataflow call");
    assert_eq!(mapped.admitted(CallPolicy::Summary), dataflow);
    assert_eq!(admitted(&mapped, CallPolicy::Association), 2);
    // Potential evidence remains in the complete event and Association; it never grants Summary.
    let potential = NativeCallee {
        modality: Modality::Potential,
        ..callee(CallPhase::Call, CallChannel::Direct, &f)
    };
    let reports = [report(
        &w.base,
        &[potential],
        &[(CallChannel::Direct, CallPhase::Call)],
        None,
    )];
    let maybe = site(&reports, &symbols);
    assert!(!maybe.assessment().exact);
    assert_eq!(admitted(&maybe, CallPolicy::Association), 1);
    for policy in [
        CallPolicy::Invocation,
        CallPolicy::Dataflow,
        CallPolicy::Summary,
    ] {
        assert_eq!(admitted(&maybe, policy), 0);
    }
    // No reported callee is an unresolved, incomplete alternative, not an empty complete answer.
    let empty = report(
        &w.base,
        &[],
        &[(CallChannel::Direct, CallPhase::Call)],
        None,
    );
    assert_eq!(empty.rows.targets.len(), 1);
    assert!(!empty.rows.resolutions[0].complete);
    assert!(matches!(
        empty.rows.destinations[0],
        CallDestination::Unresolved {
            reason: ObligationKind::UnresolvedTarget,
            native: None
        }
    ));
    // A reported unresolved remainder keeps the resolved callee but forbids completeness.
    let remainder = report(
        &w.base,
        &[callee(CallPhase::Call, CallChannel::Direct, &f)],
        &[(CallChannel::Direct, CallPhase::Call)],
        Some(ObligationKind::UnresolvedTarget),
    );
    assert_eq!(remainder.rows.targets.len(), 2);
    assert!(!remainder.rows.resolutions[0].complete);
}

#[test]
fn receivers_come_only_from_the_one_classifier() {
    let w = World::new();
    let method = w.symbol(&w.a, "C.method", SymbolKind::Method);
    let obj = occurrence(1).id();
    let evidence = |passing, static_method, actual| ReceiverEvidence {
        passing,
        static_method,
        class_method: None,
        actual,
    };
    let cases = [
        (
            evidence(Some(ReceiverPassing::NotPassed), None, None),
            Receiver::None,
        ),
        (evidence(None, Some(true), None), Receiver::None),
        (
            evidence(None, None, None),
            Receiver::Unknown {
                reason: ObligationKind::AmbiguousBinding,
            },
        ),
        (
            evidence(Some(ReceiverPassing::Object), None, Some(obj)),
            Receiver::Bound { actual: obj },
        ),
        (
            evidence(Some(ReceiverPassing::Class), None, Some(obj)),
            Receiver::Bound { actual: obj },
        ),
    ];
    for (receiver, expected) in cases {
        let reports = [report(
            &w.base,
            &[NativeCallee {
                receiver,
                ..callee(CallPhase::Call, CallChannel::Direct, &method)
            }],
            &[(CallChannel::Direct, CallPhase::Call)],
            None,
        )];
        assert_eq!(reports[0].rows.receivers, vec![expected.clone()]);
        let unknown = matches!(expected, Receiver::Unknown { .. });
        assert_eq!(
            admitted(
                &site(&reports, std::slice::from_ref(&method)),
                CallPolicy::Summary
            ),
            usize::from(!unknown),
            "an unknown receiver is never summarized"
        );
    }
}

#[test]
fn an_override_dispatch_set_is_never_a_direct_target() {
    let w = World::new();
    let base = w.symbol(&w.a, "Base.m", SymbolKind::Method);
    let service = w.symbol(&w.a, "Service", SymbolKind::Class);
    let symbols = vec![base.clone(), service.clone()];
    let call = [(CallChannel::Direct, CallPhase::Call)];
    // `self.m()` in Service, as Pysa reports it: one dispatch set, Base.m or any override below Service.
    let dispatch = NativeCallee {
        destination: CallDestination::Overrides { symbol: base.id() },
        receiver_class: Some(service.id()),
        ..callee(CallPhase::Call, CallChannel::Direct, &base)
    };
    let reports = [report(&w.base, &[dispatch], &call, None)];
    let dispatched = site(&reports, &symbols);
    assert!(
        dispatched.phase_targets().is_empty(),
        "a dispatch set names no direct target"
    );
    assert!(dispatched.assessment().dispatch && !dispatched.assessment().unique);
    assert_eq!(
        admitted(&dispatched, CallPolicy::Invocation),
        1,
        "the invocation view keeps the call through its named member"
    );
    assert_eq!(
        admitted(&dispatched, CallPolicy::Dataflow),
        0,
        "flow waits for the set's expansion"
    );
    assert_eq!(admitted(&dispatched, CallPolicy::Summary), 0);
    assert_eq!(admitted(&dispatched, CallPolicy::Association), 1);
    // Its twin, a direct call of Base.m, is unique and flows.
    let reports = [report(
        &w.base,
        &[callee(CallPhase::Call, CallChannel::Direct, &base)],
        &call,
        None,
    )];
    let unique = site(&reports, &symbols);
    assert!(unique.assessment().unique && !unique.assessment().dispatch);
    assert_eq!(
        (
            admitted(&unique, CallPolicy::Summary),
            admitted(&unique, CallPolicy::Dataflow)
        ),
        (1, 1)
    );
}

#[test]
fn the_native_unresolved_reason_is_kept() {
    let w = World::new();
    let lambda = normalize_site(
        &w.base,
        occurrence(0).id(),
        CallOrigin::explicit(),
        &[],
        &BTreeSet::new(),
        Some((
            ObligationKind::UnresolvedTarget,
            Some(PysaUnresolvedReason::LambdaArgument),
        )),
    )
    .unwrap();
    assert_eq!(
        lambda.destinations,
        vec![CallDestination::Unresolved {
            reason: ObligationKind::UnresolvedTarget,
            native: Some(PysaUnresolvedReason::LambdaArgument)
        }]
    );
    let bare = normalize_site(
        &w.base,
        occurrence(0).id(),
        CallOrigin::explicit(),
        &[],
        &BTreeSet::new(),
        None,
    )
    .unwrap();
    assert_eq!(
        bare.destinations,
        vec![CallDestination::Unresolved {
            reason: ObligationKind::UnresolvedTarget,
            native: None
        }]
    );
    assert_ne!(
        lambda.destinations[0].id(),
        bare.destinations[0].id(),
        "a native reason is part of the destination"
    );
    assert_ne!(lambda.targets[0].id(), bare.targets[0].id());
}

#[test]
fn implicit_calls_at_one_site_are_distinct_events() {
    let w = World::new();
    let (iter, next) = (
        w.symbol(&w.a, "list.__iter__", SymbolKind::Method),
        w.symbol(&w.a, "list_iterator.__next__", SymbolKind::Method),
    );
    let symbols = vec![iter.clone(), next.clone()];
    let complete: BTreeSet<_> = [(CallChannel::Direct.id(), CallPhase::Call)]
        .into_iter()
        .collect();
    let event = |step: OriginStep, symbol: &ProviderSymbol| {
        let origin = CallOrigin::new(&[(step, None)]).unwrap().0.id();
        let rows = normalize_site(
            &w.base,
            occurrence(0).id(),
            origin,
            &[callee(CallPhase::Call, CallChannel::Direct, symbol)],
            &complete,
            None,
        )
        .unwrap();
        let (run, _) = ProviderRun::new(
            w.a.id(),
            w.context.id(),
            InputRevision::from_entries(vec![]).unwrap().id(),
            w.context.config_digest,
            [FactFamily::Calls],
        )
        .unwrap();
        let surface = ProviderSurface {
            provider: w.a.id(),
            family: FactFamily::Calls,
            name: "calls".into(),
        };
        let supports = rows
            .targets
            .iter()
            .map(|t| {
                (
                    t.id(),
                    vec![CallTargetSupport {
                        assertion: t.id(),
                        run: run.id(),
                        surface: surface.id(),
                        evidence: Evidence::Occurrence {
                            occurrence: occurrence(0).id(),
                        }
                        .id(),
                        origin: Origin::AnalyzerAssertion,
                        mode: ExtractionMode::ReportDecode,
                        fidelity: Fidelity::NativeStructural,
                    }],
                )
            })
            .collect();
        Report { rows, supports }
    };
    // `for x in xs`: __iter__ and __next__ at the iterable are two calls, each unique.
    let (for_iter, for_next) = (
        [event(OriginStep::ForIter, &iter)],
        [event(OriginStep::ForNext, &next)],
    );
    assert!(
        site(&for_iter, &symbols).assessment().unique
            && site(&for_next, &symbols).assessment().unique
    );
    assert_ne!(
        for_iter[0].rows.targets[0].origin,
        for_next[0].rows.targets[0].origin
    );
    // Normalization keeps both events distinct rather than collapsing their shared site.
    let both = [
        event(OriginStep::ForIter, &iter),
        event(OriginStep::ForNext, &next),
    ];
    let resolutions: Vec<_> = both
        .iter()
        .flat_map(|r| r.rows.resolutions.iter())
        .collect();
    assert_eq!(resolutions.len(), 2);
    let separate = site(&both, &symbols);
    assert_eq!(separate.output.events.len(), 2);
    assert!(separate.output.assessments.iter().all(|a| a.unique));
    // A resolution cannot hold a target of another event.
    let r = &both[0].rows;
    assert!(
        CallResolution::new(
            &w.base,
            occurrence(0).id(),
            CallOrigin::explicit(),
            r.channels[0].id(),
            CallPhase::Call,
            true,
            &r.targets
        )
        .is_err()
    );
}
