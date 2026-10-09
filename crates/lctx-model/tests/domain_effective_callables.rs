//! Pure N3 controls: no native frontend, acquisition, store or runtime is required.
use lctx_model::domain::{
    assertion::*,
    attribution::*,
    calls::*,
    conditions::Diagram,
    input::*,
    normalized::{Rows, callable_normalization::*, callables::*, entities::*},
    resources::ResourceBudget,
    source::*,
    symbols::*,
    syntax::*,
    types::*,
    *,
};
fn fixture() -> (
    CallableData,
    ResourceBudget,
    ProviderSymbol,
    AssertionQualification,
) {
    let budget = ResourceBudget::fixed(8 << 20).unwrap();
    let mut data = CallableData::new(&budget);
    let bytes = b"def f(value=4):\n    return value\n";
    let input = InputRevision::from_entries(vec![ManifestEntry {
        path: "pure.py".into(),
        content: ContentHash::of(bytes),
        byte_len: bytes.len() as i64,
    }])
    .unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "pure.py".into(), bytes).unwrap();
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    data.scopes.insert(scope.clone()).unwrap();
    let context = AnalysisContext {
        python_version: "3.14".into(),
        python_platform: "pure".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: input.manifest,
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let provider = Provider {
        tool: "pure-callable".into(),
        revision: "1".into(),
        build_digest: input.manifest,
    };
    let (run, _) = ProviderRun::new(
        provider.id(),
        context.id(),
        input.id(),
        input.manifest,
        [
            FactFamily::Syntax,
            FactFamily::Signatures,
            FactFamily::Types,
        ],
    )
    .unwrap();
    data.coverage
        .insert(ProviderCoverage {
            scope: scope.id(),
            provider: Some(provider.id()),
            context: context.id(),
            family: FactFamily::Syntax,
            run: Some(run.id()),
            status: CoverageStatus::CompleteUnderStatedModel,
            reason: None,
            diagnostic: None,
        })
        .unwrap();
    let (condition, _) = Diagram::always().records();
    let q = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: context.id(),
        scope: scope.id(),
        condition: condition.id(),
        modality: Modality::Definite,
        approximation: Approximation::Exact,
    };
    data.qualifications.insert(q.clone()).unwrap();
    let function = Occurrence {
        source: source.id(),
        start: 0,
        end: bytes.len() as i64,
        syntax_kind: SyntaxKind::StmtFunctionDef,
        role: OccurrenceRole::Declaration,
        structural_path: vec![0, 0],
    };
    data.occurrences.insert(function.clone()).unwrap();
    let name = Occurrence {
        start: 4,
        end: 5,
        syntax_kind: SyntaxKind::Identifier,
        role: OccurrenceRole::Syntax,
        structural_path: vec![0, 0, 0],
        ..function.clone()
    };
    data.occurrences.insert(name.clone()).unwrap();
    data.declarations
        .insert(DeclarationObservation {
            qualification: q.id(),
            declaration: function.id(),
            name: name.id(),
            kind: DeclarationKind::Function,
            parent: None,
            overload: false,
            docstring: None,
        })
        .unwrap();
    let module = Module {
        source: source.id(),
        qualified_name: "pure".into(),
    };
    let provider_module = ProviderModule::Acquired {
        module: module.id(),
    };
    let symbol = ProviderSymbol {
        provider: provider.id(),
        context: context.id(),
        module: provider_module.id(),
        native_key: "f".into(),
        name: "f".into(),
        kind: SymbolKind::Function,
    };
    let callable = CallableEntity::Source {
        declaration: function.id(),
        kind: CallableKind::Function,
    };
    data.callables.insert(callable.clone()).unwrap();
    let entity = EntityRef::Callable {
        callable: callable.id(),
    };
    data.refs.insert(entity.clone()).unwrap();
    data.resolutions
        .insert(SymbolEntityResolution {
            symbol: symbol.id(),
            context: context.id(),
            policy: normalized::policy_revision(),
            status: ResolutionStatus::Resolved,
            entity: Some(entity.id()),
            reason: EntityReason::DeclarationAgreement,
        })
        .unwrap();
    data.traits
        .insert(FunctionTraitObservation {
            qualification: q.id(),
            symbol: symbol.id(),
            overload: false,
            staticmethod: false,
            classmethod: false,
            property_getter: false,
            property_setter: false,
            stub: false,
            origin: FunctionOrigin::DefStatement,
            defining_class: None,
            overrides: None,
        })
        .unwrap();
    data.bodies
        .insert(FunctionBodyObservation {
            qualification: q.id(),
            declaration: function.id(),
            body: FunctionBodyKind::Other,
            abstract_method: false,
            in_protocol_class: false,
            in_type_checking_block: false,
            overload: false,
        })
        .unwrap();
    let shape = ParameterShape {
        name: Some("value".into()),
        kind: ParameterKind::PositionalOrKeyword,
        required: false,
    };
    data.shapes.insert(shape.clone()).unwrap();
    let (signature, parameters) = Signature::new(
        &q,
        lctx_model::domain::calls::SignatureRole::Source,
        None,
        symbol.id(),
        0,
        SignatureForm::List,
        &[shape],
    )
    .unwrap();
    data.signatures.insert(signature).unwrap();
    for parameter in parameters {
        data.parameters.insert(parameter).unwrap();
    }
    (data, budget, symbol, q)
}

fn supporting_callable_fixture() -> (CallableData, ResourceBudget, Id<CallableEntity>, Id<CallableEntity>) {
    use lctx_model::domain::{lexical::*, normalized::links::*};
    let (mut data, budget, symbol, q) = fixture();
    let requested = data.callables.iter().next().unwrap().id();
    let support_symbol = ProviderSymbol { native_key: "g".into(), name: "g".into(), ..symbol };
    let support = CallableEntity::External { symbol: support_symbol.id() };
    let entity = EntityRef::Callable { callable: support.id() };
    data.callables.insert(support.clone()).unwrap();
    data.refs.insert(entity.clone()).unwrap();
    data.resolutions.insert(SymbolEntityResolution {
        symbol: support_symbol.id(), context: q.context, policy: normalized::policy_revision(),
        status: ResolutionStatus::Resolved, entity: Some(entity.id()), reason: EntityReason::ProviderExternal,
    }).unwrap();
    let declaration = data.declarations.iter().next().unwrap().declaration;
    let mut read = data.occurrences.get(declaration).unwrap().clone();
    read.syntax_kind = SyntaxKind::ExprName;
    read.role = OccurrenceRole::Syntax;
    read.structural_path.push(1);
    data.occurrences.insert(read.clone()).unwrap();
    data.decorators.insert(DeclarationDecorator {
        qualification: q.id(), declaration, decorator: read.id(), ordinal: 0,
    }).unwrap();
    let reference = ReferenceObservation {
        qualification: q.id(), read: read.id(), scope: LexicalScope {
            owner: declaration, kind: LexicalScopeKind::Function,
        }.id(), parent: declaration, field: SyntaxField::Decorator, name: "g".into(),
    };
    data.references.insert(reference.clone()).unwrap();
    let binding = BindingEvent { site: read.id(), name: "g".into() };
    let lexical_target = LexicalTarget::Binding { event: binding.id() };
    data.lexical_targets.insert(lexical_target.clone()).unwrap();
    let raw = LexicalResolution { qualification: q.id(), read: read.id(), target: lexical_target.id(), captured: false };
    data.lexical_resolutions.insert(raw.clone()).unwrap();
    let target = ReferenceEntityTarget::Binding { event: binding.id(), entity: entity.id() };
    data.reference_targets.insert(target.clone()).unwrap();
    let assessment = ReferenceEntityAssessment {
        reference: reference.id(), status: ResolutionStatus::Resolved, reason: LinkReason::ExplicitIdentity,
    };
    data.reference_assessments.insert(assessment.clone()).unwrap();
    data.reference_candidates.insert(ReferenceEntityCandidate {
        assessment: assessment.id(), resolution: raw.id(), target: target.id(),
    }).unwrap();
    (data, budget, requested, support.id())
}

#[test]
fn advertised_callable_owner_selection_keeps_supporting_inputs_without_supporting_claims() {
    let (data, budget, requested, support) = supporting_callable_fixture();
    let output = normalize(&data, &budget).unwrap();
    assert_eq!(output.assessments.len(), 2);
    assert!(data.reference_targets.iter().any(|target| matches!(target,
        normalized::links::ReferenceEntityTarget::Binding { entity, .. }
            if matches!(data.refs.get(*entity), Some(EntityRef::Callable { callable }) if *callable == support))),
        "the requested decorator's nominal reference retains the supporting callable");
    assert!(admit_callable_view(&data.view(), &output.view(), requested, &budget).is_err(),
        "nominally reached supporting owner claims are not the requested owner's family");
    let before = budget.reserved();
    for owner in [requested, support] {
        let selection = CallableOwnerSelection::new(&output, owner, &budget).unwrap();
        let view = selection.view().unwrap();
        let expected = normalize_callable_view(&data.view(), owner, &budget).unwrap();
        assert!(data.callables.get(support).is_some(), "supporting input remains available");
        view.matches(&expected).unwrap();
        admit_callable_view(&data.view(), &view, owner, &budget).unwrap();
        assert!(view.assessments.iter().all(|row| row.callable == owner));
        assert!(view.evidence.iter().all(|row| view.assessments.get(row.assessment).is_some()));
        assert!(view.premises.iter().all(|row| view.evidence.iter().any(|link| link.premise == row.id())));
    }
    assert_eq!(budget.reserved(), before, "borrowed membership reservations end with the selection");
}

#[test]
fn advertised_callable_owner_selection_preserves_extra_and_missing_claim_refusal() {
    let (data, budget, requested, support) = supporting_callable_fixture();
    for mutation in 0..5 {
        let mut output = normalize(&data, &budget).unwrap();
        let assessment = output.assessments.iter().find(|row| row.callable == requested).unwrap().clone();
        let support_premise = EffectiveCallablePremise::Resolution {
            resolution: data.resolutions.iter().find(|row| row.entity.is_some_and(|id|
                matches!(data.refs.get(id), Some(EntityRef::Callable { callable }) if *callable == support))).unwrap().id(),
        };
        match mutation {
            0 => { let mut extra = assessment.clone(); extra.decorators = ContentHash::of(b"unadvertised-chain"); output.assessments.insert(extra).unwrap(); }
            1 => { output.premises.insert(support_premise.clone()).unwrap(); output.evidence.insert(EffectiveCallableEvidence {
                assessment: assessment.id(), premise: support_premise.id(),
            }).unwrap(); }
            2 => { let original = output.decorators.iter().find(|row| row.assessment == assessment.id()).unwrap().clone();
                let mut extra = original; extra.observation = DeclarationDecorator {
                    qualification: data.qualifications.iter().next().unwrap().id(),
                    declaration: data.declarations.iter().next().unwrap().declaration,
                    decorator: data.decorators.iter().next().unwrap().decorator, ordinal: 1,
                }.id(); output.decorators.insert(extra).unwrap(); }
            3 => { let removed = output.evidence.iter().find(|row| row.assessment == assessment.id()).unwrap().id();
                let retained = output.evidence.iter().filter(|row| row.id() != removed).cloned().collect::<Vec<_>>();
                output.evidence = Rows::new(&budget); for row in retained { output.evidence.insert(row).unwrap(); } }
            _ => { let removed = output.evidence.iter().find(|row| row.assessment == assessment.id()).unwrap().premise;
                let retained = output.premises.iter().filter(|row| row.id() != removed).cloned().collect::<Vec<_>>();
                output.premises = Rows::new(&budget); for row in retained { output.premises.insert(row).unwrap(); } }
        }
        match CallableOwnerSelection::new(&output, requested, &budget) {
            Ok(selection) => assert!(admit_callable_view(&data.view(), &selection.view().unwrap(), requested, &budget).is_err(),
                "same-owner mutation {mutation} must remain visible to independent admission"),
            Err(_) => assert_eq!(mutation, 4, "only absent advertised premises refuse selection itself"),
        };
    }
}
#[test]
fn inspectable_unknown_forms_never_authorize_effective_body_invocation() {
    for form in [
        SignatureForm::List,
        SignatureForm::Ellipsis,
        SignatureForm::ParamSpec,
        SignatureForm::NativeUnavailable,
    ] {
        let (mut data, budget, symbol, q) = fixture();
        if form != SignatureForm::List {
            data.signatures = Rows::new(&budget);
            data.parameters = Rows::new(&budget);
            let (signature, _) = Signature::new(
                &q,
                lctx_model::domain::calls::SignatureRole::Source,
                None,
                symbol.id(),
                0,
                form,
                &[],
            )
            .unwrap();
            data.signatures.insert(signature).unwrap();
        }
        let output = normalize(&data, &budget).unwrap();
        assert_eq!(output.variants.len(), 1);
        let assessment = output.assessments.iter().next().unwrap();
        assert_eq!(
            assessment.signatures,
            if form == SignatureForm::List {
                Knowledge::Known
            } else {
                Knowledge::Unknown
            }
        );
        assert_eq!(assessment.body_admitted, form == SignatureForm::List);
        assert_eq!(
            assessment.descriptor_kind,
            Some(DescriptorKind::Function),
            "metadata recognition is independent of signature completeness"
        );
    }
}
#[test]
fn body_exclusions_and_missing_evidence_are_independent_of_identity() {
    for mode in 0..5 {
        let (mut data, budget, _, _) = fixture();
        let original = data.bodies.iter().next().unwrap().clone();
        data.bodies = Rows::new(&budget);
        if mode != 0 {
            let mut row = original;
            match mode {
                1 => row.abstract_method = true,
                2 => row.in_protocol_class = true,
                3 => row.in_type_checking_block = true,
                _ => row.overload = true,
            }
            data.bodies.insert(row).unwrap();
        }
        let output = normalize(&data, &budget).unwrap();
        let a = output.assessments.iter().next().unwrap();
        assert_eq!(a.identity, Knowledge::Known);
        assert!(!a.body_admitted);
        assert_eq!(
            a.body,
            if mode == 0 {
                Knowledge::Unknown
            } else {
                Knowledge::Known
            }
        );
    }
}
#[test]
fn cross_provider_signature_agreement_and_conflict_preserve_all_variants() {
    for conflict in [false, true] {
        let (mut data, budget, symbol, q) = fixture();
        let provider = Provider {
            tool: "independent".into(),
            revision: "1".into(),
            build_digest: ContentHash::of(b"independent"),
        };
        let other = ProviderSymbol {
            provider: provider.id(),
            ..symbol
        };
        let original = data.resolutions.iter().next().unwrap().clone();
        data.resolutions
            .insert(SymbolEntityResolution {
                symbol: other.id(),
                ..original
            })
            .unwrap();
        let mut shape = data.shapes.iter().next().unwrap().clone();
        shape.required = conflict;
        data.shapes.insert(shape.clone()).unwrap();
        let (signature, parameters) = Signature::new(
            &q,
            lctx_model::domain::calls::SignatureRole::Source,
            None,
            other.id(),
            0,
            SignatureForm::List,
            &[shape],
        )
        .unwrap();
        data.signatures.insert(signature).unwrap();
        for parameter in parameters {
            data.parameters.insert(parameter).unwrap();
        }
        let output = normalize(&data, &budget).unwrap();
        assert_eq!(output.variants.len(), 2);
        let a = output.assessments.iter().next().unwrap();
        assert_eq!(
            a.signatures,
            if conflict {
                Knowledge::Conflicting
            } else {
                Knowledge::Known
            }
        );
        assert_eq!(a.body_admitted, !conflict);
    }
}
#[test]
fn missing_syntax_coverage_cannot_prove_an_empty_decorator_chain() {
    let (mut data, budget, _, _) = fixture();
    data.coverage = Rows::new(&budget);
    let output = normalize(&data, &budget).unwrap();
    let a = output.assessments.iter().next().unwrap();
    assert_eq!(a.identity, Knowledge::Unknown);
    assert_eq!(a.identity_reason, CallableReason::IncompleteSyntax);
    assert!(!a.body_admitted);
    assert_eq!(output.variants.len(), 1);
}
#[test]
fn uncertain_qualifications_and_conflicting_native_traits_do_not_admit_a_body() {
    for mode in 0..4 {
        let (mut data, budget, symbol, q) = fixture();
        if mode == 0 {
            let candidate = AssertionQualification {
                modality: Modality::Candidate,
                ..q
            };
            data.qualifications.insert(candidate.clone()).unwrap();
            let body = data.bodies.iter().next().unwrap().clone();
            data.bodies = Rows::new(&budget);
            data.bodies
                .insert(FunctionBodyObservation {
                    qualification: candidate.id(),
                    ..body
                })
                .unwrap();
        } else if mode == 1 {
            let other_q = AssertionQualification {
                approximation: Approximation::Under,
                ..q
            };
            data.qualifications.insert(other_q.clone()).unwrap();
            let native = data.traits.iter().next().unwrap().clone();
            data.traits
                .insert(FunctionTraitObservation {
                    qualification: other_q.id(),
                    ..native
                })
                .unwrap();
        } else if mode == 2 {
            let other = ProviderSymbol {
                native_key: "other-f".into(),
                ..symbol
            };
            let resolution = data.resolutions.iter().next().unwrap().clone();
            data.resolutions
                .insert(SymbolEntityResolution {
                    symbol: other.id(),
                    ..resolution
                })
                .unwrap();
            let native = data.traits.iter().next().unwrap().clone();
            data.traits
                .insert(FunctionTraitObservation {
                    symbol: other.id(),
                    stub: true,
                    ..native
                })
                .unwrap();
        } else {
            let conditional = AssertionQualification {
                condition: Diagram::never().id(),
                ..q
            };
            data.qualifications.insert(conditional.clone()).unwrap();
            let body = data.bodies.iter().next().unwrap().clone();
            data.bodies = Rows::new(&budget);
            data.bodies
                .insert(FunctionBodyObservation {
                    qualification: conditional.id(),
                    ..body
                })
                .unwrap();
        }
        let output = normalize(&data, &budget).unwrap();
        let a = output.assessments.iter().next().unwrap();
        assert!(!a.body_admitted);
        assert_eq!(
            a.body,
            if mode == 2 {
                Knowledge::Conflicting
            } else {
                Knowledge::Unknown
            }
        );
        if mode == 0 {
            assert_eq!(a.identity, Knowledge::Known);
        }
    }
}

#[test]
fn qualified_signatures_and_async_syntax_do_not_advertise_context_wide_knowledge() {
    for external in [false, true] {
        let (mut data, budget, symbol, q) = fixture();
        if external {
            let callable = CallableEntity::External {
                symbol: symbol.id(),
            };
            data.callables.insert(callable.clone()).unwrap();
            let entity = EntityRef::Callable {
                callable: callable.id(),
            };
            data.refs.insert(entity.clone()).unwrap();
            data.resolutions = Rows::new(&budget);
            data.resolutions
                .insert(SymbolEntityResolution {
                    symbol: symbol.id(),
                    context: q.context,
                    policy: normalized::policy_revision(),
                    status: ResolutionStatus::Resolved,
                    entity: Some(entity.id()),
                    reason: EntityReason::ProviderExternal,
                })
                .unwrap();
        }
        let conditional = AssertionQualification {
            condition: Diagram::never().id(),
            ..q
        };
        data.qualifications.insert(conditional.clone()).unwrap();
        let shape = data.shapes.iter().next().unwrap().clone();
        let (signature, parameters) = Signature::new(
            &conditional,
            lctx_model::domain::calls::SignatureRole::Source,
            None,
            symbol.id(),
            0,
            SignatureForm::List,
            &[shape],
        )
        .unwrap();
        data.signatures = Rows::new(&budget);
        data.parameters = Rows::new(&budget);
        data.signatures.insert(signature).unwrap();
        for parameter in parameters {
            data.parameters.insert(parameter).unwrap();
        }
        let output = normalize(&data, &budget).unwrap();
        let variant = output.variants.iter().next().unwrap();
        let assessment = output.assessments.get(variant.assessment.unwrap()).unwrap();
        assert_eq!(assessment.signatures, Knowledge::Unknown);
        assert_eq!(
            assessment.signature_reason,
            CallableReason::QualifiedUncertainty
        );
    }
    for conditional in [false, true] {
        let (mut data, budget, _, q) = fixture();
        let uncertain = if conditional {
            AssertionQualification {
                condition: Diagram::never().id(),
                ..q
            }
        } else {
            AssertionQualification {
                modality: Modality::Candidate,
                ..q
            }
        };
        data.qualifications.insert(uncertain.clone()).unwrap();
        let declaration = data.declarations.iter().next().unwrap().clone();
        data.declarations = Rows::new(&budget);
        data.declarations
            .insert(DeclarationObservation {
                qualification: uncertain.id(),
                kind: DeclarationKind::AsyncFunction,
                ..declaration
            })
            .unwrap();
        let output = normalize(&data, &budget).unwrap();
        let a = output.assessments.iter().next().unwrap();
        assert_eq!(a.asynchronous, None);
        assert_eq!(a.identity, Knowledge::Unknown);
        assert!(!a.body_admitted);
    }
}
