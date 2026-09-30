#[path = "fixtures/events.rs"] mod event_fixture;
use lctx_model::domain::normalized::{events::CallPolicy, event_normalization};
use lctx_model::domain::{
    assertion::*, attribution::*, calls::*, conditions::Diagram, input::*, source::*, *,
};
use std::collections::BTreeMap;

struct Fixture {
    model: ValidatedModel,
    qualification: AssertionQualification,
    symbol: ProviderSymbol,
    destination: CallDestination,
    channel: CallChannel,
    receiver: Receiver,
    target: CallTarget,
    support: CallTargetSupport,
    run: ProviderRun,
}
fn occurrence(n: i64) -> Occurrence {
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "a.py".into(), b"f(x)").unwrap();
    Occurrence {
        source: source.id(),
        start: n,
        end: n + 1,
        syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Read,
        structural_path: vec![n as i32],
    }
}
impl Fixture {
    fn new() -> Self {
        let model = model().unwrap();
        let context = AnalysisContext {
            python_version: "3.14".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"cfg"),
            environment_digest: ContentHash::of(b"env"),
            lock_digest: None,
        };
        let provider = Provider {
            tool: "pinned-provider".into(),
            revision: "revision".into(),
            build_digest: ContentHash::of(b"build"),
        };
        let input = InputRevision::from_entries(vec![]).unwrap();
        let (run, _) = ProviderRun::new(
            provider.id(),
            context.id(),
            input.id(),
            context.config_digest,
            [FactFamily::Calls],
        )
        .unwrap();
        let scope = CoverageScope::Input { input: input.id() };
        let qualification = AssertionQualification {
            context: context.id(),
            scope: scope.id(),
            condition: Diagram::always().id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let symbol = ProviderSymbol {
            provider: provider.id(),
            context: context.id(),
            module: ProviderModule::Bundled {
                provider: provider.id(),
                bundle: ModuleBundle::Typeshed,
                name: "a".into(),
            }
            .id(),
            native_key: "function:f".into(),
            name: "f".into(),
            kind: SymbolKind::Function,
        };
        let destination = CallDestination::Resolved {
            symbol: symbol.id(),
        };
        let channel = CallChannel::Direct;
        let receiver = Receiver::None;
        let target = CallTarget {
            qualification: qualification.id(),
            site: occurrence(0).id(),
            destination: destination.id(),
            channel: channel.id(),
            phase: CallPhase::Call,
            receiver: receiver.id(),
            implicit: false,
            origin: CallOrigin::explicit(),
            receiver_class: None,
            passing: Some(ReceiverPassing::NotPassed),
            class_method: None,
            static_method: None,
        };
        let surface = ProviderSurface {
            provider: provider.id(),
            family: FactFamily::Calls,
            name: "targets".into(),
        };
        let evidence = Evidence::Occurrence {
            occurrence: target.site,
        };
        let support = CallTargetSupport {
            assertion: target.id(),
            run: run.id(),
            surface: surface.id(),
            evidence: evidence.id(),
            origin: Origin::AnalyzerAssertion,
            mode: ExtractionMode::NativeTraversal,
            fidelity: Fidelity::NativeStructural,
        };
        Self {
            model,
            qualification,
            symbol,
            destination,
            channel,
            receiver,
            target,
            support,
            run,
        }
    }
    fn event(&self, resolution: &CallResolution, targets: Vec<CallTarget>, destinations: Vec<CallDestination>, channel: CallChannel, supports: Vec<CallTargetSupport>) -> event_fixture::Evaluated {
        let members = targets.iter().map(|target| CallResolutionMember { resolution: resolution.id(), target: target.id() }).collect();
        let rows = NormalizedSite { qualifications: vec![self.qualification.clone()], channels: vec![channel], destinations, receivers: vec![self.receiver.clone()], targets, resolutions: vec![resolution.clone()], members };
        let mut evidence: BTreeMap<_, Vec<_>> = BTreeMap::new(); for support in supports { evidence.entry(support.assertion).or_default().push(support); }
        let (data, budget) = event_fixture::data(&[(&rows, &evidence)], std::slice::from_ref(&self.symbol)); event_fixture::evaluate(data, budget)
    }
    /// The call's syntax and complete ordered arguments, asserted by the fixture's qualification.
    fn call(&self, actuals: &[Actual]) -> (CallSyntax, Vec<CallArgument>) {
        CallSyntax::new(
            self.qualification.id(),
            self.target.site,
            occurrence(99).id(),
            false,
            actuals,
        )
        .unwrap()
    }
    fn signature(
        &self,
        parameters: &[ParameterShape],
    ) -> (
        Signature,
        Vec<SignatureParameter>,
        BTreeMap<Id<ParameterShape>, ParameterShape>,
    ) {
        let (signature, members) = Signature::new(
            &self.qualification,
            self.symbol.id(),
            0,
            SignatureForm::List,
            parameters,
        )
        .unwrap();
        (
            signature,
            members,
            parameters.iter().map(|p| (p.id(), p.clone())).collect(),
        )
    }
}
fn shape(name: &str, kind: ParameterKind, required: bool) -> ParameterShape {
    ParameterShape {
        name: Some(name.into()),
        kind,
        required,
    }
}
fn actual(n: i64, kind: ArgumentKind, name: Option<&str>) -> Actual {
    Actual {
        occurrence: occurrence(n).id(),
        kind,
        keyword: name.map(str::to_owned),
    }
}

#[test]
fn complete_alternatives_precede_policies_and_higher_order_never_becomes_direct() {
    let f = Fixture::new();
    let resolution = |targets: &[CallTarget], channel: &CallChannel| CallResolution::new(&f.qualification, f.target.site, CallOrigin::explicit(), channel.id(), CallPhase::Call, true, targets).unwrap().0;
    let initial = resolution(std::slice::from_ref(&f.target), &f.channel);
    let known = f.event(&initial, vec![f.target.clone()], vec![f.destination.clone()], f.channel.clone(), vec![f.support.clone()]);
    assert_eq!(known.admitted(CallPolicy::Summary), vec![f.target.id()]); assert_eq!(known.admitted(CallPolicy::Invocation), vec![f.target.id()]);
    let unresolved = CallDestination::Unresolved { reason: ObligationKind::UnresolvedTarget, native: None };
    let unknown = CallTarget { destination: unresolved.id(), ..f.target.clone() };
    let support = CallTargetSupport { assertion: unknown.id(), ..f.support.clone() };
    let complete = resolution(&[f.target.clone(), unknown.clone()], &f.channel);
    let set = f.event(&complete, vec![f.target.clone(), unknown.clone()], vec![f.destination.clone(), unresolved], f.channel.clone(), vec![f.support.clone(), support]);
    assert!(set.admitted(CallPolicy::Summary).is_empty()); assert_eq!(set.admitted(CallPolicy::Dataflow), vec![f.target.id()]);
    let mut data = set.data; data.members = lctx_model::domain::normalized::Rows::new(&set.budget);
    data.members.insert(CallResolutionMember { resolution: complete.id(), target: f.target.id() }).unwrap();
    assert!(event_normalization::normalize(&data, &set.budget).is_err(), "a filtered set cannot manufacture uniqueness");
    let channel = CallChannel::HigherOrder { argument_index: 0 };
    let higher = CallTarget { channel: channel.id(), ..f.target.clone() };
    let support = CallTargetSupport { assertion: higher.id(), ..f.support.clone() };
    let complete = resolution(std::slice::from_ref(&higher), &channel);
    let set = f.event(&complete, vec![higher.clone()], vec![f.destination.clone()], channel, vec![support]);
    for policy in [CallPolicy::Invocation, CallPolicy::Dataflow, CallPolicy::Summary] { assert!(set.admitted(policy).is_empty()); }
    assert_eq!(set.admitted(CallPolicy::Association), vec![higher.id()]);
    let arrow = Batch::new(&f.model, vec![higher.clone(), unknown], &budget()).unwrap();
    assert_eq!(CallTarget::decode(arrow.arrow()).unwrap(), arrow.rows());
}

#[test]
fn binder_requires_whole_variant_and_preserves_all_formals_without_inventing_values() {
    let f = Fixture::new();
    let definitions = vec![
        shape("x", ParameterKind::PositionalOnly, true),
        shape("y", ParameterKind::PositionalOrKeyword, false),
        shape("args", ParameterKind::VarPositional, false),
        shape("flag", ParameterKind::KeywordOnly, false),
        shape("kwargs", ParameterKind::VarKeyword, false),
    ];
    let (signature, members, shapes) = f.signature(&definitions);
    let args = [actual(1, ArgumentKind::Positional, None)];
    let bind_with = |parameters: &[SignatureParameter], actuals: &[Actual]| {
        let (call, arguments) = f.call(actuals);
        bind(BindingInput {
            target: &f.target,
            qualification: &f.qualification,
            signature_qualification: &f.qualification,
            destination: &f.destination,
            channel: &f.channel,
            receiver: &f.receiver,
            signature: &signature,
            parameters,
            shapes: &shapes,
            call: &call,
            arguments: &arguments,
        })
    };
    let bound = bind_with(&members, &args).unwrap();
    assert_eq!(bound.bindings().len(), 5);
    assert_eq!(
        bound
            .bindings()
            .iter()
            .filter(|b| b.source == BindingSource::Default)
            .count(),
        2
    );
    assert_eq!(bound.site(), f.target.site);
    assert_eq!(bound.target(), f.target.id());
    assert_eq!(bound.signature(), signature.id());
    assert!(
        bound
            .bindings()
            .iter()
            .any(|b| b.source == BindingSource::EmptyVarargs)
    );
    assert!(
        bound
            .bindings()
            .iter()
            .any(|b| b.source == BindingSource::EmptyKwargs)
    );
    assert!(
        bind_with(&members[..1], &args).is_err(),
        "successful subset does not certify invocation"
    );
    assert!(bind_with(&members, &[]).is_err(), "required formal absent");
    assert!(
        bind_with(&members, &[actual(1, ArgumentKind::Keyword, Some("x"))]).is_err(),
        "positional-only remains unbound even if x enters kwargs"
    );
    assert_eq!(
        bind_with(&members, &[actual(1, ArgumentKind::Starred, None)]).unwrap_err(),
        ObligationKind::UnsupportedUnpacking
    );
    assert!(
        bind_with(
            &members,
            &[
                actual(1, ArgumentKind::Positional, None),
                actual(2, ArgumentKind::Keyword, Some("extra")),
                actual(3, ArgumentKind::Keyword, Some("extra"))
            ]
        )
        .is_err()
    );
    let many = [
        actual(1, ArgumentKind::Positional, None),
        actual(2, ArgumentKind::Positional, None),
        actual(3, ArgumentKind::Positional, None),
        actual(4, ArgumentKind::Positional, None),
    ];
    let bound = bind_with(&members, &many).unwrap();
    assert_eq!(
        bound
            .bindings()
            .iter()
            .filter(|b| b.formal == members[2].id())
            .count(),
        2,
        "all varargs values survive"
    );
    let mut crossed = members.clone();
    crossed[0].signature = Signature {
        variant: 1,
        ..signature.clone()
    }
    .id();
    assert!(bind_with(&crossed, &args).is_err());
    let (empty, empty_members, empty_shapes) = f.signature(&[]);
    assert!(
        bind(BindingInput {
            target: &f.target,
            qualification: &f.qualification,
            signature_qualification: &f.qualification,
            destination: &f.destination,
            channel: &f.channel,
            receiver: &f.receiver,
            signature: &empty,
            parameters: &empty_members,
            shapes: &empty_shapes,
            call: &f.call(&[]).0,
            arguments: &f.call(&[]).1
        })
        .is_ok(),
        "a declared zero-argument signature is different from missing parameters"
    );
}

#[test]
fn receiver_classification_refuses_incomplete_evidence_and_missing_actuals() {
    let base = ReceiverEvidence {
        passing: None,
        static_method: None,
        class_method: None,
        actual: None,
    };
    assert!(matches!(classify_receiver(base), Receiver::Unknown { .. }));
    assert!(matches!(
        classify_receiver(ReceiverEvidence {
            passing: Some(ReceiverPassing::Object),
            ..base
        }),
        Receiver::Unknown { .. }
    ));
    assert_eq!(
        classify_receiver(ReceiverEvidence {
            passing: Some(ReceiverPassing::NotPassed),
            ..base
        }),
        Receiver::None
    );
    assert_eq!(
        classify_receiver(ReceiverEvidence {
            static_method: Some(true),
            ..base
        }),
        Receiver::None
    );
    assert!(matches!(
        classify_receiver(ReceiverEvidence {
            static_method: Some(true),
            class_method: Some(true),
            ..base
        }),
        Receiver::Unknown { .. }
    ));
    let actual = occurrence(0).id();
    assert_eq!(
        classify_receiver(ReceiverEvidence {
            passing: Some(ReceiverPassing::Object),
            actual: Some(actual),
            ..base
        }),
        Receiver::Bound { actual }
    );
    // Review F04: `c.cm()` passes the object's class, which no actual denotes; `C.cm()` passes `C`.
    assert!(matches!(
        classify_receiver(ReceiverEvidence {
            passing: Some(ReceiverPassing::Object),
            class_method: Some(true),
            actual: Some(actual),
            ..base
        }),
        Receiver::Unknown { .. }
    ));
    assert_eq!(
        classify_receiver(ReceiverEvidence {
            passing: Some(ReceiverPassing::Class),
            class_method: Some(true),
            actual: Some(actual),
            ..base
        }),
        Receiver::Bound { actual }
    );
    let mut f = Fixture::new();
    f.receiver = classify_receiver(base);
    f.target.receiver = f.receiver.id();
    let (signature, members, shapes) =
        f.signature(&[shape("self", ParameterKind::PositionalOnly, true)]);
    assert_eq!(
        bind(BindingInput {
            target: &f.target,
            qualification: &f.qualification,
            signature_qualification: &f.qualification,
            destination: &f.destination,
            channel: &f.channel,
            receiver: &f.receiver,
            signature: &signature,
            parameters: &members,
            shapes: &shapes,
            call: &f.call(&[]).0,
            arguments: &f.call(&[]).1
        })
        .unwrap_err(),
        ObligationKind::AmbiguousBinding
    );
}

#[test]
fn stored_membership_checks_refuse_missing_parameters_and_missing_call_alternatives() {
    let f = Fixture::new();
    let params = [
        shape("x", ParameterKind::PositionalOnly, true),
        shape("flag", ParameterKind::KeywordOnly, true),
    ];
    let (signature, members, _) = f.signature(&params);
    let check_signature = |members: Vec<SignatureParameter>| {
        let invariant = Signature::invariants().remove(0);
        let mut check = (invariant.create)(&budget());
        macro_rules! visit {
            ($ty:ty,$rows:expr) => {
                check
                    .visit(
                        <$ty>::NAME,
                        Batch::new(&f.model, $rows, &budget()).unwrap().arrow(),
                    )
                    .unwrap()
            };
        }
        visit!(AssertionQualification, vec![f.qualification.clone()]);
        visit!(ProviderSymbol, vec![f.symbol.clone()]);
        visit!(ParameterShape, params.to_vec());
        visit!(Signature, vec![signature.clone()]);
        // The stored validator scans this relation in its declared signature/ordinal order.
        check
            .visit(
                SignatureParameter::NAME,
                &SignatureParameter::encode(&members).unwrap(),
            )
            .unwrap();
        check.finish()
    };
    check_signature(members.clone()).unwrap();
    assert!(check_signature(members[..1].to_vec()).is_err());
    let mut changed = members.clone();
    changed[1].shape = changed[0].shape;
    assert!(check_signature(changed).is_err());
    let (resolution, members) = CallResolution::new(
        &f.qualification,
        f.target.site,
        CallOrigin::explicit(),
        f.channel.id(),
        CallPhase::Call,
        true,
        std::slice::from_ref(&f.target),
    )
    .unwrap();
    for include in [true, false] {
        let invariant = CallResolution::invariants().remove(0);
        let mut check = (invariant.create)(&budget());
        check
            .visit(
                AssertionQualification::NAME,
                Batch::new(&f.model, vec![f.qualification.clone()], &budget())
                    .unwrap()
                    .arrow(),
            )
            .unwrap();
        check
            .visit(
                CallTarget::NAME,
                Batch::new(&f.model, vec![f.target.clone()], &budget())
                    .unwrap()
                    .arrow(),
            )
            .unwrap();
        check
            .visit(
                CallResolution::NAME,
                Batch::new(&f.model, vec![resolution.clone()], &budget())
                    .unwrap()
                    .arrow(),
            )
            .unwrap();
        if include {
            check
                .visit(
                    CallResolutionMember::NAME,
                    Batch::new(&f.model, members.clone(), &budget())
                        .unwrap()
                        .arrow(),
                )
                .unwrap();
        }
        assert_eq!(check.finish().is_ok(), include);
    }
}

#[test]
fn binding_context_and_receiver_varargs_are_explicit() {
    let mut f = Fixture::new();
    f.receiver = Receiver::Bound {
        actual: occurrence(0).id(),
    };
    f.target.receiver = f.receiver.id();
    let (signature, members, shapes) =
        f.signature(&[shape("args", ParameterKind::VarPositional, false)]);
    let (call, arguments) = f.call(&[actual(1, ArgumentKind::Positional, None)]);
    let input = |qualification| BindingInput {
        target: &f.target,
        qualification: &f.qualification,
        signature_qualification: qualification,
        destination: &f.destination,
        channel: &f.channel,
        receiver: &f.receiver,
        signature: &signature,
        parameters: &members,
        shapes: &shapes,
        call: &call,
        arguments: &arguments,
    };
    assert_eq!(
        bind(input(&f.qualification)).unwrap().bindings().len(),
        2,
        "bound receiver and explicit value both reach args"
    );
    let other = AssertionQualification {
        modality: Modality::Potential,
        ..f.qualification.clone()
    };
    assert!(bind(input(&other)).is_err());
    let mut f = Fixture::new();
    f.qualification.modality = Modality::Potential;
    f.target.qualification = f.qualification.id();
    f.support.assertion = f.target.id();
    let (resolution, _) = CallResolution::new(
        &f.qualification,
        f.target.site,
        CallOrigin::explicit(),
        f.channel.id(),
        CallPhase::Call,
        true,
        &[f.target.clone()],
    )
    .unwrap();
    let set = f.event(&resolution, vec![f.target.clone()], vec![f.destination.clone()], f.channel.clone(), vec![f.support.clone()]);
    assert!(set.admitted(CallPolicy::Dataflow).is_empty());
    assert!(set.admitted(CallPolicy::Invocation).is_empty());
    assert_eq!(set.admitted(CallPolicy::Association), vec![f.target.id()]);
}

#[test]
fn native_signature_and_call_support_cannot_switch_provider_namespace() {
    let f = Fixture::new();
    let (signature, _, _) = f.signature(&[]);
    let alien = Provider {
        tool: "other-provider".into(),
        revision: "same".into(),
        build_digest: ContentHash::of(b"other"),
    };
    for call_support in [false, true] {
        for wrong in [false, true] {
            let invariant = ProviderSymbol::invariants().remove(0);
            let mut check = (invariant.create)(&budget());
            let run = if wrong {
                ProviderRun {
                    provider: alien.id(),
                    ..f.run.clone()
                }
            } else {
                f.run.clone()
            };
            macro_rules! visit {
                ($ty:ty,$rows:expr) => {
                    check
                        .visit(
                            <$ty>::NAME,
                            Batch::new(&f.model, $rows, &budget()).unwrap().arrow(),
                        )
                        .unwrap()
                };
            }
            visit!(ProviderSymbol, vec![f.symbol.clone()]);
            visit!(ProviderRun, vec![run.clone()]);
            visit!(CallDestination, vec![f.destination.clone()]);
            visit!(Signature, vec![signature.clone()]);
            visit!(CallTarget, vec![f.target.clone()]);
            let result = if call_support {
                let support = CallTargetSupport {
                    run: run.id(),
                    ..f.support.clone()
                };
                check.visit(
                    CallTargetSupport::NAME,
                    Batch::new(&f.model, vec![support], &budget())
                        .unwrap()
                        .arrow(),
                )
            } else {
                let support = SignatureSupport {
                    assertion: signature.id(),
                    run: run.id(),
                    surface: f.support.surface,
                    evidence: f.support.evidence,
                    origin: f.support.origin,
                    mode: f.support.mode,
                    fidelity: f.support.fidelity,
                };
                check.visit(
                    SignatureSupport::NAME,
                    Batch::new(&f.model, vec![support], &budget())
                        .unwrap()
                        .arrow(),
                )
            };
            assert_eq!(result.is_ok(), !wrong);
            if wrong {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("different provider")
                );
            } else {
                check.finish().unwrap();
            }
        }
    }
}

#[test]
fn binder_checks_shape_lookup_identity_and_retains_aggregate_coordinates() {
    let f = Fixture::new();
    let required = shape("value", ParameterKind::PositionalOnly, true);
    let (signature, members, mut shapes) = f.signature(std::slice::from_ref(&required));
    shapes.insert(
        required.id(),
        ParameterShape {
            required: false,
            ..required
        },
    );
    assert_eq!(
        bind(BindingInput {
            target: &f.target,
            qualification: &f.qualification,
            signature_qualification: &f.qualification,
            destination: &f.destination,
            channel: &f.channel,
            receiver: &f.receiver,
            signature: &signature,
            parameters: &members,
            shapes: &shapes,
            call: &f.call(&[]).0,
            arguments: &f.call(&[]).1
        })
        .unwrap_err(),
        ObligationKind::MissingEvidence
    );
    let (signature, members, shapes) =
        f.signature(&[shape("kwargs", ParameterKind::VarKeyword, false)]);
    let mut bindings = vec![];
    for name in ["left", "right"] {
        let args = [actual(1, ArgumentKind::Keyword, Some(name))];
        let bound = bind(BindingInput {
            target: &f.target,
            qualification: &f.qualification,
            signature_qualification: &f.qualification,
            destination: &f.destination,
            channel: &f.channel,
            receiver: &f.receiver,
            signature: &signature,
            parameters: &members,
            shapes: &shapes,
            call: &f.call(&args).0,
            arguments: &f.call(&args).1,
        })
        .unwrap();
        assert_eq!(bound.bindings()[0].kind, BindingKind::Kwargs);
        assert_eq!(
            bound.bindings()[0].projection,
            BindingProjection::Keyword { name: name.into() }
        );
        bindings.push(bound.bindings().to_vec());
    }
    assert_ne!(bindings[0], bindings[1]);
    let mut f = Fixture::new();
    f.receiver = Receiver::Bound {
        actual: occurrence(0).id(),
    };
    f.target.receiver = f.receiver.id();
    let (signature, members, shapes) =
        f.signature(&[shape("args", ParameterKind::VarPositional, false)]);
    let args = [
        actual(1, ArgumentKind::Positional, None),
        actual(2, ArgumentKind::Implicit, None),
    ];
    let bound = bind(BindingInput {
        target: &f.target,
        qualification: &f.qualification,
        signature_qualification: &f.qualification,
        destination: &f.destination,
        channel: &f.channel,
        receiver: &f.receiver,
        signature: &signature,
        parameters: &members,
        shapes: &shapes,
        call: &f.call(&args).0,
        arguments: &f.call(&args).1,
    })
    .unwrap();
    assert_eq!(
        bound
            .bindings()
            .iter()
            .map(|binding| binding.kind.clone())
            .collect::<Vec<_>>(),
        vec![
            BindingKind::Receiver,
            BindingKind::Varargs,
            BindingKind::Implicit
        ]
    );
    for (index, binding) in bound.bindings().iter().enumerate() {
        assert_eq!(
            binding.projection,
            BindingProjection::Positional {
                index: index as i64
            }
        );
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}

#[test]
fn provider_modules_distinguish_origin_and_belong_to_their_provider() {
    use lctx_model::domain::types::TypeVariable;
    let model = model().unwrap();
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    };
    let other_context = AnalysisContext {
        python_platform: "darwin".into(),
        ..context.clone()
    };
    let provider = Provider {
        tool: "pinned-provider".into(),
        revision: "1".into(),
        build_digest: ContentHash::of(b"build"),
    };
    let alien = Provider {
        tool: "other-provider".into(),
        ..provider.clone()
    };
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "pkg/mod.py".into(), b"x = 1").unwrap();
    let acquired = ProviderModule::Acquired {
        module: Module {
            source: source.id(),
            qualified_name: "pkg.mod".into(),
        }
        .id(),
    };
    let bundled = ProviderModule::Bundled {
        provider: provider.id(),
        bundle: ModuleBundle::Typeshed,
        name: "pkg.mod".into(),
    };
    let unresolved = ProviderModule::Unresolved {
        provider: provider.id(),
        context: context.id(),
        name: "pkg.mod".into(),
    };
    let ids = [acquired.id(), bundled.id(), unresolved.id()];
    assert!(
        ids[0] != ids[1] && ids[1] != ids[2] && ids[0] != ids[2],
        "the same spelling in different origins is not one module"
    );
    assert!(
        ProviderModule::Bundled {
            provider: provider.id(),
            bundle: ModuleBundle::Typeshed,
            name: String::new()
        }
        .validate()
        .is_err()
    );
    let symbol =
        |owner: &Provider, context: &AnalysisContext, module: &ProviderModule| ProviderSymbol {
            provider: owner.id(),
            context: context.id(),
            module: module.id(),
            native_key: "pkg.mod.f".into(),
            name: "f".into(),
            kind: SymbolKind::Function,
        };
    let check = |modules: Vec<ProviderModule>, symbols: Vec<ProviderSymbol>| {
        let invariant = model
            .invariants()
            .iter()
            .find(|i| i.name == "provider_module_owners")
            .unwrap();
        let mut check = (invariant.create)(&budget());
        check.visit(
            ProviderModule::NAME,
            Batch::new(&model, modules, &budget()).unwrap().arrow(),
        )?;
        check.visit(
            ProviderSymbol::NAME,
            Batch::new(&model, symbols, &budget()).unwrap().arrow(),
        )?;
        check.visit(
            TypeVariable::NAME,
            Batch::<TypeVariable>::new(&model, vec![], &budget())
                .unwrap()
                .arrow(),
        )?;
        check.finish()
    };
    let modules = vec![acquired.clone(), bundled.clone(), unresolved.clone()];
    check(
        modules.clone(),
        vec![
            symbol(&provider, &context, &acquired),
            symbol(&provider, &context, &bundled),
            symbol(&provider, &context, &unresolved),
            symbol(&alien, &context, &acquired),
        ],
    )
    .unwrap();
    assert!(
        check(modules.clone(), vec![symbol(&alien, &context, &bundled)]).is_err(),
        "a bundled module belongs to its provider"
    );
    assert!(
        check(
            modules.clone(),
            vec![symbol(&provider, &other_context, &unresolved)]
        )
        .is_err(),
        "an unresolved module belongs to its context"
    );
    assert!(
        check(vec![], vec![symbol(&provider, &context, &bundled)]).is_err(),
        "a symbol's module must be present"
    );
}

#[test]
fn local_variable_places_are_shared_within_a_scope_and_distinct_across_scopes() {
    use lctx_model::domain::{flow::*, value::*};
    let model = model().unwrap();
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(
        input.id(),
        "m.py".into(),
        b"def f():\n x = 1\n x = 2\n return x\ndef g():\n x = 3\n",
    )
    .unwrap();
    let occurrence = |start: i64, kind: SyntaxKind, path: Vec<i32>| Occurrence {
        source: source.id(),
        start,
        end: start + 1,
        syntax_kind: kind,
        role: OccurrenceRole::Syntax,
        structural_path: path,
    };
    let (f, g) = (
        occurrence(0, SyntaxKind::StmtFunctionDef, vec![0]),
        occurrence(36, SyntaxKind::StmtFunctionDef, vec![1]),
    );
    let local = |scope: &Occurrence| PlaceRoot::Local {
        scope: scope.id(),
        name: "x".into(),
    };
    let place = |root: &PlaceRoot| Place {
        root: root.id(),
        path: AccessPath::empty().id(),
    };
    let (in_f, in_g) = (place(&local(&f)), place(&local(&g)));
    assert_ne!(
        in_f.id(),
        in_g.id(),
        "the same name in a sibling scope is another variable"
    );
    assert!(
        PlaceRoot::Local {
            scope: f.id(),
            name: String::new()
        }
        .validate()
        .is_err()
    );
    let first = FlowDefinition {
        occurrence: occurrence(10, SyntaxKind::ExprName, vec![0, 0]).id(),
        place: in_f.id(),
    };
    let second = FlowDefinition {
        occurrence: occurrence(17, SyntaxKind::ExprName, vec![0, 1]).id(),
        place: in_f.id(),
    };
    let sibling = FlowDefinition {
        occurrence: occurrence(42, SyntaxKind::ExprName, vec![1, 0]).id(),
        place: in_g.id(),
    };
    let read = FlowUse {
        occurrence: occurrence(31, SyntaxKind::ExprName, vec![0, 2]).id(),
        place: in_f.id(),
    };
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    };
    let q = AssertionQualification {
        context: context.id(),
        scope: CoverageScope::Input { input: input.id() }.id(),
        condition: Diagram::always().id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    let reaching = |definition: &FlowDefinition| {
        (
            ReachingDefinition::Bound {
                definition: definition.id(),
            },
            FlowReachingObservation {
                qualification: q.id(),
                use_: read.id(),
                target: ReachingDefinition::Bound {
                    definition: definition.id(),
                }
                .id(),
                loop_carried: false,
            },
        )
    };
    let check = |targets: Vec<(ReachingDefinition, FlowReachingObservation)>| {
        let invariant = model
            .invariants()
            .iter()
            .find(|i| i.name == "flow_reaching_places")
            .unwrap();
        let mut check = (invariant.create)(&budget());
        let (definitions, observations): (Vec<_>, Vec<_>) = targets.into_iter().unzip();
        check.visit(
            FlowUse::NAME,
            Batch::new(&model, vec![read.clone()], &budget())
                .unwrap()
                .arrow(),
        )?;
        check.visit(
            FlowDefinition::NAME,
            Batch::new(
                &model,
                vec![first.clone(), second.clone(), sibling.clone()],
                &budget(),
            )
            .unwrap()
            .arrow(),
        )?;
        check.visit(
            ReachingDefinition::NAME,
            Batch::new(&model, definitions, &budget()).unwrap().arrow(),
        )?;
        check.visit(
            FlowReachingObservation::NAME,
            Batch::new(&model, observations, &budget()).unwrap().arrow(),
        )?;
        check.finish()
    };
    check(vec![reaching(&first), reaching(&second)]).unwrap();
    assert!(
        check(vec![reaching(&sibling)]).is_err(),
        "a use cannot be reached by another scope's variable"
    );
}

/// The old binder's answers, re-expressed: a call binds only as a whole variant, so surplus or
/// conflicting actuals refuse the variant instead of leaving unmapped rows.
#[test]
fn positional_keyword_receiver_and_refused_variants() {
    let f = Fixture::new();
    let bind_to = |f: &Fixture, shapes: &[ParameterShape], actuals: &[Actual]| {
        let (signature, members, shapes) = f.signature(shapes);
        let (call, arguments) = f.call(actuals);
        bind(BindingInput {
            target: &f.target,
            qualification: &f.qualification,
            signature_qualification: &f.qualification,
            destination: &f.destination,
            channel: &f.channel,
            receiver: &f.receiver,
            signature: &signature,
            parameters: &members,
            shapes: &shapes,
            call: &call,
            arguments: &arguments,
        })
        .map(|bound| (bound, members))
    };
    let formal_of = |bound: &BoundCall, n: i64| {
        bound
            .bindings()
            .iter()
            .find(|b| b.source == BindingSource::Actual(occurrence(n).id()))
            .map(|b| (b.formal, b.kind.clone()))
    };
    let ab = [
        shape("a", ParameterKind::PositionalOrKeyword, true),
        shape("b", ParameterKind::PositionalOrKeyword, true),
    ];
    // f(x, y) and f(b=y, a=x) bind the same formals.
    let (by_position, members) = bind_to(
        &f,
        &ab,
        &[
            actual(1, ArgumentKind::Positional, None),
            actual(2, ArgumentKind::Positional, None),
        ],
    )
    .unwrap();
    assert_eq!(
        formal_of(&by_position, 1),
        Some((members[0].id(), BindingKind::Positional))
    );
    let (by_name, _) = bind_to(
        &f,
        &ab,
        &[
            actual(2, ArgumentKind::Keyword, Some("b")),
            actual(1, ArgumentKind::Keyword, Some("a")),
        ],
    )
    .unwrap();
    assert_eq!(
        formal_of(&by_name, 1),
        Some((members[0].id(), BindingKind::Keyword))
    );
    assert_eq!(
        formal_of(&by_name, 2),
        Some((members[1].id(), BindingKind::Keyword))
    );
    // A formal bound twice (a TypeError at run time) refuses the variant.
    assert!(
        bind_to(
            &f,
            &ab,
            &[
                actual(1, ArgumentKind::Positional, None),
                actual(2, ArgumentKind::Keyword, Some("a"))
            ]
        )
        .is_err()
    );
    // Surplus positionals or unknown keywords without a collector refuse; they are never unmapped rows.
    let strict = [shape("a", ParameterKind::PositionalOrKeyword, true)];
    assert!(
        bind_to(
            &f,
            &strict,
            &[
                actual(1, ArgumentKind::Positional, None),
                actual(2, ArgumentKind::Positional, None)
            ]
        )
        .is_err()
    );
    assert!(
        bind_to(
            &f,
            &strict,
            &[
                actual(1, ArgumentKind::Positional, None),
                actual(2, ArgumentKind::Keyword, Some("z"))
            ]
        )
        .is_err()
    );
    // Keyword-only formals take only keywords.
    let keyword_only = [shape("k", ParameterKind::KeywordOnly, true)];
    assert!(
        bind_to(
            &f,
            &keyword_only,
            &[actual(1, ArgumentKind::Keyword, Some("k"))]
        )
        .is_ok()
    );
    assert!(
        bind_to(
            &f,
            &keyword_only,
            &[actual(1, ArgumentKind::Positional, None)]
        )
        .is_err()
    );
    // `**kw` unpacking is unsupported, never an error or an ambiguous partial binding.
    assert_eq!(
        bind_to(
            &f,
            &ab,
            &[
                actual(1, ArgumentKind::Positional, None),
                actual(2, ArgumentKind::DoubleStarred, None)
            ]
        )
        .unwrap_err(),
        ObligationKind::UnsupportedUnpacking
    );
    // A bound receiver takes the first formal; the next actual the second.
    let mut bound = Fixture::new();
    bound.receiver = Receiver::Bound {
        actual: occurrence(9).id(),
    };
    bound.target.receiver = bound.receiver.id();
    let (method, members) = bind_to(
        &bound,
        &[
            shape("self", ParameterKind::PositionalOrKeyword, true),
            shape("x", ParameterKind::PositionalOrKeyword, true),
        ],
        &[actual(2, ArgumentKind::Positional, None)],
    )
    .unwrap();
    assert_eq!(
        formal_of(&method, 9),
        Some((members[0].id(), BindingKind::Receiver))
    );
    assert_eq!(
        formal_of(&method, 2),
        Some((members[1].id(), BindingKind::Positional))
    );
}

fn refused(result: Result<(), ModelError>, why: &str, expected: &str) {
    assert!(
        matches!(&result, Err(ModelError::Invalid(message)) if message.contains(expected)),
        "{why}: expected `{expected}`, got {result:?}"
    );
}
/// Visit `rows` of each relation, in order, into the named invariant of `R`, and finish it.
macro_rules! check {
    ($model:expr, $owner:ty, $index:expr, [$(($ty:ty, $rows:expr)),+ $(,)?]) => {{
        let invariant = <$owner>::invariants().remove($index); let mut check = (invariant.create)(&budget());
        (|| -> Result<(), ModelError> {
            $( check.visit(<$ty>::NAME, Batch::new(&$model, $rows, &budget()).unwrap().arrow())?; )+
            check.finish()
        })()
    }};
}

#[test]
fn provider_call_sites_state_their_event_record_and_caller() {
    let f = Fixture::new();
    let (for_iter, steps) = CallOrigin::new(&[(OriginStep::ForIter, None)]).unwrap();
    let source = SourceArtifact::from_bytes(
        InputRevision::from_entries(vec![]).unwrap().id(),
        "a.py".into(),
        b"f(x)",
    )
    .unwrap();
    let module = Module {
        source: source.id(),
        qualified_name: "a".into(),
    };
    let acquired = ProviderModule::Acquired {
        module: module.id(),
    };
    let caller = ProviderCallable::ModuleBody {
        provider: f.symbol.provider,
        context: f.symbol.context,
        module: acquired.id(),
    };
    let site = ProviderCallSite {
        qualification: f.qualification.id(),
        site: occurrence(0).id(),
        origin: CallOrigin::explicit(),
        kind: PysaSiteKind::Regular,
        caller: caller.id(),
        callee: PysaCalleeKind::Call,
        is_attribute: None,
    };
    site.validate().unwrap();
    let artificial = ProviderCallSite {
        origin: for_iter.id(),
        kind: PysaSiteKind::ArtificialCall,
        ..site.clone()
    };
    artificial.validate().unwrap();
    for (row, why) in [
        (
            ProviderCallSite {
                kind: PysaSiteKind::ArtificialCall,
                ..site.clone()
            },
            "an artificial call without origin steps",
        ),
        (
            ProviderCallSite {
                origin: for_iter.id(),
                ..site.clone()
            },
            "a regular call with origin steps",
        ),
        (
            ProviderCallSite {
                kind: PysaSiteKind::Identifier,
                ..site.clone()
            },
            "an identifier site reported by a call record",
        ),
        (
            ProviderCallSite {
                is_attribute: Some(true),
                ..site.clone()
            },
            "a call record stating an attribute read",
        ),
    ] {
        assert!(row.validate().is_err(), "{why} is refused");
    }
    assert!(
        ProviderCallSite {
            callee: PysaCalleeKind::AttributeAccess,
            is_attribute: Some(false),
            ..site.clone()
        }
        .validate()
        .is_ok()
    );
    let stored = |caller: &ProviderCallable, row: &ProviderCallSite| {
        check!(
            f.model,
            ProviderCallSite,
            0,
            [
                (Module, vec![module.clone()]),
                (
                    ProviderModule,
                    vec![
                        acquired.clone(),
                        ProviderModule::Bundled {
                            provider: f.symbol.provider,
                            bundle: ModuleBundle::Typeshed,
                            name: "a".into()
                        }
                    ]
                ),
                (
                    ProviderSymbol,
                    vec![
                        f.symbol.clone(),
                        ProviderSymbol {
                            native_key: "a:class".into(),
                            kind: SymbolKind::Class,
                            ..f.symbol.clone()
                        }
                    ]
                ),
                (ProviderCallable, vec![caller.clone()]),
                (Occurrence, vec![occurrence(0)]),
                (ProviderCallSite, vec![row.clone()])
            ]
        )
    };
    stored(&caller, &site).expect("a module body calls in its own module");
    let class = ProviderCallable::Symbol {
        symbol: ProviderSymbol {
            native_key: "a:class".into(),
            kind: SymbolKind::Class,
            ..f.symbol.clone()
        }
        .id(),
    };
    refused(
        stored(
            &class,
            &ProviderCallSite {
                caller: class.id(),
                ..site.clone()
            },
        ),
        "a class is not a caller",
        "a call-site caller is a callable",
    );
    refused(
        stored(
            &ProviderCallable::Symbol {
                symbol: f.symbol.id(),
            },
            &ProviderCallSite {
                caller: ProviderCallable::Symbol {
                    symbol: f.symbol.id(),
                }
                .id(),
                ..site.clone()
            },
        ),
        "a caller of a bundled module",
        "defined in the site's module",
    );
    // Origin steps: stored membership, and an index exactly for a chained assignment.
    let membership = |steps: Vec<CallOriginStep>| {
        check!(
            f.model,
            CallOrigin,
            0,
            [
                (CallOrigin, vec![for_iter.clone()]),
                (CallOriginStep, steps)
            ]
        )
    };
    membership(steps.clone()).unwrap();
    refused(
        membership(vec![]),
        "an origin missing its step",
        "call origin has missing steps",
    );
    assert!(CallOrigin::new(&[(OriginStep::ChainedAssign, None)]).is_err());
    assert!(CallOrigin::new(&[(OriginStep::ForNext, Some(0))]).is_err());
    // Steps run from the outer context to the operation: `a[0] = b[1] = v` sets the second target's item.
    let (chained, _) = CallOrigin::new(&[
        (OriginStep::ChainedAssign, Some(1)),
        (OriginStep::SubscriptSetItem, None),
    ])
    .unwrap();
    assert_ne!(
        chained.id(),
        CallOrigin::new(&[
            (OriginStep::ChainedAssign, Some(0)),
            (OriginStep::SubscriptSetItem, None)
        ])
        .unwrap()
        .0
        .id()
    );
    // Review F01: `f"{g()}"` stringifies g's result at g()'s span. The stringify is its own event,
    // never an alternative of the explicit call.
    let (stringify, stringify_steps) =
        CallOrigin::new(&[(OriginStep::FormatStringStringify, None)]).unwrap();
    let format = ProviderCallSite {
        origin: stringify.id(),
        kind: PysaSiteKind::FormatStringStringify,
        callee: PysaCalleeKind::FormatStringStringify,
        ..site.clone()
    };
    format.validate().unwrap();
    assert!(
        ProviderCallSite {
            origin: CallOrigin::explicit(),
            ..format.clone()
        }
        .validate()
        .is_err(),
        "a format-string site on the explicit call"
    );
    assert_ne!(stringify.id(), CallOrigin::explicit());
    let sited = |row: &ProviderCallSite, steps: Vec<CallOriginStep>| {
        check!(
            f.model,
            ProviderCallSite,
            0,
            [
                (Module, vec![module.clone()]),
                (ProviderModule, vec![acquired.clone()]),
                (ProviderCallable, vec![caller.clone()]),
                (Occurrence, vec![occurrence(0)]),
                (CallOriginStep, steps),
                (ProviderCallSite, vec![row.clone()])
            ]
        )
    };
    sited(&format, stringify_steps.clone()).expect("a stringify under its own step");
    let for_steps = CallOrigin::new(&[(OriginStep::ForIter, None)]).unwrap().1;
    refused(
        sited(
            &ProviderCallSite {
                origin: for_iter.id(),
                ..format.clone()
            },
            for_steps.clone(),
        ),
        "a stringify under a for step",
        "exactly its format-string step",
    );
    refused(
        sited(
            &ProviderCallSite {
                origin: stringify.id(),
                kind: PysaSiteKind::ArtificialCall,
                callee: PysaCalleeKind::Call,
                ..site.clone()
            },
            stringify_steps,
        ),
        "an artificial call under a format-string step",
        "exactly its format-string step",
    );
}

#[path = "fixtures/callers.rs"]
mod caller_fixture;
#[test]
fn call_site_support_authenticates_implicit_and_symbolic_callers_transitively() {
    for form in ["module", "symbol", "class", "decorator"] {
        for mismatch in ["none", "provider", "context"] {
            let f = caller_fixture::fixture(form, mismatch);
            let result = f.check(&ProviderCallSiteSupport::invariants()[0]);
            assert_eq!(
                result.is_ok(),
                mismatch == "none",
                "{form}/{mismatch}: {result:?}"
            );
        }
    }
}

#[test]
fn receiver_classes_and_dispatch_sets_name_the_right_kinds() {
    let f = Fixture::new();
    let class = ProviderSymbol {
        native_key: "class:C".into(),
        name: "C".into(),
        kind: SymbolKind::Class,
        ..f.symbol.clone()
    };
    let method = ProviderSymbol {
        native_key: "method:C.m".into(),
        name: "m".into(),
        kind: SymbolKind::Method,
        ..f.symbol.clone()
    };
    let stored_with = |destination: &CallDestination,
                       receiver_class: Option<Id<ProviderSymbol>>,
                       passing: ReceiverPassing| {
        let target = CallTarget {
            destination: destination.id(),
            receiver_class,
            passing: Some(passing),
            class_method: Some(false),
            static_method: Some(false),
            ..f.target.clone()
        };
        check!(
            f.model,
            CallTarget,
            0,
            [
                (AssertionQualification, vec![f.qualification.clone()]),
                (
                    ProviderSymbol,
                    vec![f.symbol.clone(), class.clone(), method.clone()]
                ),
                (Occurrence, vec![occurrence(0)]),
                (Receiver, vec![f.receiver.clone()]),
                (CallDestination, vec![destination.clone()]),
                (CallTarget, vec![target])
            ]
        )
    };
    // The fixture's receiver is None: a function called without an implicit receiver.
    let stored = |destination: &CallDestination, receiver_class| {
        stored_with(destination, receiver_class, ReceiverPassing::NotPassed)
    };
    refused(
        stored_with(&f.destination, None, ReceiverPassing::Object),
        "no receiver where the object is passed",
        "differs from what its native evidence classifies",
    );
    stored(
        &CallDestination::Resolved {
            symbol: method.id(),
        },
        Some(class.id()),
    )
    .expect("a method through its receiver class");
    refused(
        stored(
            &CallDestination::Resolved {
                symbol: method.id(),
            },
            Some(method.id()),
        ),
        "a method as a receiver class",
        "a receiver class is a class",
    );
    stored(
        &CallDestination::Overrides {
            symbol: method.id(),
        },
        Some(class.id()),
    )
    .expect("the overrides of a method");
    refused(
        stored(
            &CallDestination::Overrides {
                symbol: f.symbol.id(),
            },
            Some(class.id()),
        ),
        "the overrides of a plain function",
        "names a method and its receiver class",
    );
    refused(
        stored(
            &CallDestination::Overrides {
                symbol: method.id(),
            },
            None,
        ),
        "a dispatch set without its receiver class",
        "names a method and its receiver class",
    );
    // A receiver class is native to the supporting provider.
    let alien = Provider {
        tool: "other-provider".into(),
        revision: "same".into(),
        build_digest: ContentHash::of(b"other"),
    };
    let foreign = ProviderSymbol {
        provider: alien.id(),
        ..class.clone()
    };
    let target = CallTarget {
        receiver_class: Some(foreign.id()),
        ..f.target.clone()
    };
    let support = CallTargetSupport {
        assertion: target.id(),
        ..f.support.clone()
    };
    let native = check!(
        f.model,
        ProviderSymbol,
        0,
        [
            (ProviderSymbol, vec![f.symbol.clone(), foreign.clone()]),
            (ProviderRun, vec![f.run.clone()]),
            (CallDestination, vec![f.destination.clone()]),
            (Signature, Vec::<Signature>::new()),
            (CallTarget, vec![target]),
            (SignatureSupport, Vec::<SignatureSupport>::new()),
            (CallTargetSupport, vec![support])
        ]
    );
    assert!(
        matches!(native, Err(ModelError::Invalid(message)) if message.contains("different provider")),
        "a receiver class of another provider"
    );
}

#[test]
fn native_positional_kind_order_is_preserved_and_keyword_groups_cannot_go_backward() {
    let f = Fixture::new();
    let definitions = [
        shape("self", ParameterKind::PositionalOrKeyword, true),
        shape("__context", ParameterKind::PositionalOnly, true),
    ];
    let (signature, parameters, shapes) = f.signature(&definitions);
    let actuals = [
        actual(1, ArgumentKind::Positional, None),
        actual(2, ArgumentKind::Positional, None),
    ];
    let (call, arguments) = f.call(&actuals);
    let result = bind(BindingInput {
        target: &f.target,
        qualification: &f.qualification,
        signature_qualification: &f.qualification,
        destination: &f.destination,
        channel: &f.channel,
        receiver: &f.receiver,
        signature: &signature,
        parameters: &parameters,
        shapes: &shapes,
        call: &call,
        arguments: &arguments,
    });
    assert!(result.is_ok(), "{result:?}");
    let invalid = [
        shape("flag", ParameterKind::KeywordOnly, true),
        definitions[1].clone(),
    ];
    assert!(
        Signature::new(
            &f.qualification,
            f.symbol.id(),
            0,
            SignatureForm::List,
            &invalid
        )
        .is_err()
    );
}

#[test]
fn unavailable_native_slots_roundtrip_and_refuse_binding() {
    let f = Fixture::new();
    let definitions = [
        shape("content", ParameterKind::KeywordOnly, false),
        shape("content", ParameterKind::KeywordOnly, false),
    ];
    assert!(
        Signature::new(
            &f.qualification,
            f.symbol.id(),
            0,
            SignatureForm::List,
            &definitions
        )
        .is_err()
    );
    let (signature, parameters) = Signature::new(
        &f.qualification,
        f.symbol.id(),
        0,
        SignatureForm::NativeUnavailable,
        &definitions,
    )
    .unwrap();
    let shapes = definitions.iter().map(|s| (s.id(), s.clone())).collect();
    let (call, arguments) = f.call(&[]);
    let result = bind(BindingInput {
        target: &f.target,
        qualification: &f.qualification,
        signature_qualification: &f.qualification,
        destination: &f.destination,
        channel: &f.channel,
        receiver: &f.receiver,
        signature: &signature,
        parameters: &parameters,
        shapes: &shapes,
        call: &call,
        arguments: &arguments,
    });
    assert_eq!(result.unwrap_err(), ObligationKind::OutsideProviderModel);
    let batch = Batch::new(&f.model, vec![signature.clone()], &budget()).unwrap();
    assert_eq!(Signature::decode(batch.arrow()).unwrap(), vec![signature]);
    assert_eq!(parameters.len(), 2);
    assert_eq!(parameters[0].ordinal, 0);
    assert_eq!(parameters[1].ordinal, 1);
}
