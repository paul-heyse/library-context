//! Site-level call facts and provider normalization (C04, C10), with pre-written answers.
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
fn site<'a>(reports: &'a [Report], symbols: &'a [ProviderSymbol]) -> SiteTargets<'a> {
    let mut sets = Vec::new();
    for report in reports {
        let r = &report.rows;
        for resolution in &r.resolutions {
            let qualification = r
                .qualifications
                .iter()
                .find(|q| q.id() == resolution.qualification)
                .unwrap();
            let channel = r
                .channels
                .iter()
                .find(|c| c.id() == resolution.channel)
                .unwrap();
            let candidates = r
                .members
                .iter()
                .filter(|m| m.resolution == resolution.id())
                .map(|member| {
                    let target = r.targets.iter().find(|t| t.id() == member.target).unwrap();
                    let destination = r
                        .destinations
                        .iter()
                        .find(|d| d.id() == target.destination)
                        .unwrap();
                    let symbol = destination
                        .symbol()
                        .and_then(|symbol| symbols.iter().find(|s| s.id() == symbol));
                    CallCandidate {
                        target,
                        qualification: r
                            .qualifications
                            .iter()
                            .find(|q| q.id() == target.qualification)
                            .unwrap(),
                        destination,
                        symbol,
                        receiver: r
                            .receivers
                            .iter()
                            .find(|x| x.id() == target.receiver)
                            .unwrap(),
                        supports: &report.supports[&target.id()],
                    }
                })
                .collect();
            sets.push(TargetSet::new(resolution, qualification, channel, candidates).unwrap());
        }
    }
    SiteTargets::new(sets).unwrap()
}
fn admitted(site: &SiteTargets<'_>, policy: CallPolicy) -> usize {
    site.admitted(policy).len()
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
    assert!(agreement.facts().unique && agreement.facts().complete);
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
    assert!(disagreement.facts().disagreement && !disagreement.facts().unique);
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
    assert_eq!(constructed.facts().group, Some(PhaseGroup::Construct));
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
    assert!(!partial.facts().complete);
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
    assert!(mapped.facts().unique);
    assert_eq!(
        mapped.facts().targets[&CallPhase::Call],
        BTreeSet::from([map.id()])
    );
    let dataflow = mapped.admitted(CallPolicy::Dataflow);
    assert_eq!(dataflow.len(), 1, "f is never a direct dataflow call");
    assert_eq!(mapped.admitted(CallPolicy::Summary), dataflow);
    assert_eq!(admitted(&mapped, CallPolicy::Association), 2);
    // A potential target is excluded from site facts and from every call policy.
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
    assert!(!maybe.facts().unique);
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
        dispatched.facts().targets.is_empty(),
        "a dispatch set names no direct target"
    );
    assert!(dispatched.facts().dispatch && !dispatched.facts().unique);
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
    assert!(unique.facts().unique && !unique.facts().dispatch);
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
    assert!(site(&for_iter, &symbols).facts().unique && site(&for_next, &symbols).facts().unique);
    assert_ne!(
        for_iter[0].rows.targets[0].origin,
        for_next[0].rows.targets[0].origin
    );
    // Asked as one event they would disagree; the site view refuses to mix them.
    let both = [
        event(OriginStep::ForIter, &iter),
        event(OriginStep::ForNext, &next),
    ];
    let resolutions: Vec<_> = both
        .iter()
        .flat_map(|r| r.rows.resolutions.iter())
        .collect();
    assert_eq!(resolutions.len(), 2);
    let sets: Vec<_> = both
        .iter()
        .map(|report| {
            let r = &report.rows;
            let target = &r.targets[0];
            TargetSet::new(
                &r.resolutions[0],
                &r.qualifications[0],
                &r.channels[0],
                vec![CallCandidate {
                    target,
                    qualification: &r.qualifications[0],
                    destination: &r.destinations[0],
                    symbol: symbols
                        .iter()
                        .find(|s| Some(s.id()) == r.destinations[0].symbol()),
                    receiver: &r.receivers[0],
                    supports: &report.supports[&target.id()],
                }],
            )
            .unwrap()
        })
        .collect();
    assert!(
        matches!(SiteTargets::new(sets), Err(ModelError::Invalid(message)) if message.contains("mix call events"))
    );
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
