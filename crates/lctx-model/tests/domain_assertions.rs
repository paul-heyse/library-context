use arrow_array::RecordBatch;
use lctx_model::domain::{assertion::*, attribution::*, conditions::*, input::*, source::*, *};
use std::collections::BTreeMap;

fn context(version: &str) -> AnalysisContext {
    AnalysisContext {
        python_version: version.into(),
        python_platform: "linux".into(),
        search_path: vec![],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"config"),
        environment_digest: ContentHash::of(b"env"),
        lock_digest: None,
    }
}
fn insert<R: Record>(
    model: &ValidatedModel,
    batches: &mut BTreeMap<&'static str, RecordBatch>,
    rows: Vec<R>,
) {
    batches.insert(
        R::NAME,
        Batch::new(model, rows, &budget()).unwrap().arrow().clone(),
    );
}
fn check(invariant: &Invariant, batches: &BTreeMap<&str, RecordBatch>) -> Result<(), ModelError> {
    let mut check = (invariant.create)(&budget());
    for input in &invariant.inputs {
        if let Some(batch) = batches.get(input.name()) {
            check.visit(input.name(), batch)?;
        }
    }
    check.finish()
}

#[test]
fn assertion_support_vocabulary_belongs_to_native_or_exact_derived_publication() {
    use stages::PublicationBoundary::{Facts, Local, Model, Summary};
    let model = model().unwrap();
    let derived = [
        (transfer::local::TransferSupport::NAME, Local),
        (transfer::local::ControlSupport::NAME, Local),
        (transfer::model::TransferSupport::NAME, Model),
        (transfer::summary::TransferSupport::NAME, Summary),
        (transfer::summary::ControlSupport::NAME, Summary),
    ];
    let mut observed = std::collections::BTreeSet::new();
    let mut native = 0;
    for invariant in model.invariants() {
        let Some(scope) = (invariant.create)(&budget()).support_scope() else {
            continue;
        };
        let expected = derived
            .iter()
            .find(|(name, _)| *name == scope.support.name())
            .map_or(Facts, |(_, epoch)| *epoch);
        if expected == Facts {
            native += 1;
        } else {
            observed.insert(scope.support.name());
        }
        assert_eq!(invariant.revision, 2);
        assert!(
            invariant
                .inputs
                .iter()
                .any(|input| stages::is_vocabulary(input.name()))
        );
        for input in invariant
            .inputs
            .iter()
            .filter(|input| stages::is_vocabulary(input.name()))
        {
            assert_eq!(
                input.prefix(),
                Some(expected),
                "{} needs its own {} role",
                invariant.name,
                input.name()
            );
        }
    }
    assert!(native > 0);
    assert_eq!(observed, derived.iter().map(|(name, _)| *name).collect());
}

#[test]
fn output_validation_vocabulary_follows_each_actual_owner() {
    use stages::PublicationBoundary as P;
    let definitions = validation::definitions();
    let families = [
        ("local", P::Local),
        ("base_evaluation", P::BaseEvaluation),
        ("base_completion", P::BaseCompletion),
        ("source_call", P::SourceCall),
        ("enriched_execution", P::EnrichedExecution),
        ("model", P::Model),
        ("summary", P::Summary),
        ("structural", P::Structural),
        ("analytic_embedding", P::AnalyticEmbedding),
        ("analytic", P::Analytic),
        ("catalog_core", P::Facts),
        ("catalog_evidence", P::Local),
        ("selection", P::Local),
        ("synthesis", P::Synthesis),
        ("retrieval", P::Retrieval),
    ];
    let assert_role = |name: &str, epoch| {
        let invariant = definitions
            .invariants
            .iter()
            .find(|invariant| invariant.name == name)
            .unwrap();
        assert_eq!(invariant.revision, 2, "{name}");
        let vocabulary = invariant
            .inputs
            .iter()
            .filter(|input| stages::is_vocabulary(input.name()))
            .collect::<Vec<_>>();
        assert!(!vocabulary.is_empty(), "{name}");
        for input in vocabulary {
            assert_eq!(input.prefix(), Some(epoch), "{name}/{}", input.name());
        }
    };
    for ((family, epoch), owner) in families
        .into_iter()
        .zip(analysis::owner::AnalysisOwner::ALL)
    {
        assert_eq!(owner.output_vocabulary_boundary(), epoch);
        assert_role(&format!("{family}_qualified_derivation"), epoch);
        assert_role(&format!("{family}_analysis_discharge"), epoch);
    }
    for (family, epoch) in [
        ("local", P::Local),
        ("model", P::Model),
        ("summary", P::Summary),
    ] {
        assert_role(&format!("{family}_transfer_keys"), epoch);
        if family != "model" {
            assert_role(&format!("{family}_transfer_selections"), epoch);
        }
    }
    assert_role("finding_emitter", P::Synthesis);
    for native in [
        "complete_signature_membership",
        "complete_call_alternatives",
        "call_target_ownership",
    ] {
        assert_role(native, P::Facts);
    }
    for native in [
        "native_signature_enumeration",
        "declaration_links",
        "generic_specialization_basis",
        "selected_type_location_shapes",
        "native_overload_original_membership",
        "flow_capture_inventory",
        "flow_use_inventory_admission",
        "flow_view_and_narrowing_precision",
        "flow_source_structure",
        "export_enumeration_membership",
        "analytic_text_original_membership",
    ] {
        assert_role(native, P::Facts);
    }
    assert_role("actual_pinned_assumption_universe", P::Model);
    assert_role("checked_finite_refutation_membership", P::Summary);
    assert_role("structural_source_fidelity", P::Structural);
    for (name, epoch) in [
        ("execution_expression_fidelity", P::BaseEvaluation),
        ("execution_statement_fidelity", P::BaseCompletion),
        ("execution_body_fidelity", P::BaseCompletion),
        ("execution_header_fidelity", P::SourceCall),
        ("execution_release_fidelity", P::SourceCall),
        ("execution_invocation_fidelity", P::SourceCall),
        ("enriched_statement_fidelity", P::EnrichedExecution),
        ("enriched_body_fidelity", P::EnrichedExecution),
        ("enriched_fresh_fidelity", P::EnrichedExecution),
        ("enriched_modeled_fidelity", P::EnrichedExecution),
        ("enriched_definition_fidelity", P::EnrichedExecution),
        ("enriched_binding_fidelity", P::EnrichedExecution),
        ("enriched_context_fidelity", P::EnrichedExecution),
        ("enriched_capture_fidelity", P::EnrichedExecution),
    ] {
        assert_role(name, epoch);
    }
    let composition = definitions
        .invariants
        .iter()
        .find(|check| check.name == "call_composition_frames")
        .unwrap();
    assert_eq!(composition.revision, 2);
    assert!(composition.inputs.iter().any(|input| input.name()
        == assertion::AssertionQualification::NAME
        && input.prefix() == Some(P::Facts)));
    assert!(composition.inputs.iter().any(|input| input.name()
        == assertion::AssertionQualification::NAME
        && input.prefix() == Some(P::Summary)));
    assert!(
        composition
            .inputs
            .iter()
            .filter(|input| stages::is_vocabulary(input.name()))
            .all(|input| matches!(input.prefix(), Some(P::Facts | P::Summary)))
    );
}

#[test]
fn ordinary_admission_vocabulary_roles_remain_explicit_beside_distinct_epochs() {
    let model = model().unwrap();
    let roots = model
        .relations()
        .iter()
        .flat_map(|relation| {
            let input = ValidationInput::of_relation(relation, &["id"]);
            if stages::is_vocabulary(relation.name()) {
                vec![
                    input.clone().at_epoch(stages::PublicationBoundary::Facts),
                    input.at_epoch(stages::PublicationBoundary::Local),
                ]
            } else {
                vec![input]
            }
        })
        .collect::<Vec<_>>();
    let unresolved = model
        .admission_candidates_for_views(&roots)
        .unwrap()
        .into_iter()
        .flat_map(|check| {
            check
                .inputs
                .into_iter()
                .filter(|input| stages::is_vocabulary(input.name()) && input.prefix().is_none())
                .map(move |input| (check.name, input.name()))
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        unresolved.is_empty(),
        "ordinary admission declarations need exact vocabulary roles: {unresolved:?}"
    );
}

#[test]
fn generated_derivation_keeps_shared_roles_in_one_cross_relation_check() {
    use stages::PublicationBoundary::{Facts, Local};
    let model = model().unwrap();
    let roots = [
        ValidationInput::of::<assumptions::Assumption>(&["id"]).at_epoch(Facts),
        ValidationInput::of::<assumptions::Assumption>(&["id"]).at_epoch(Local),
        ValidationInput::of::<assumptions_universe::AssumptionUniverseSupport>(&["id"]),
    ];
    let checks = model.admission_candidates_for_views(&roots).unwrap();
    let generated = checks
        .iter()
        .filter(|check| check.name == "derivation_acyclic")
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 1);
    let generated = generated[0];
    assert_eq!(generated.revision, 2);
    let roles = generated
        .inputs
        .iter()
        .map(|input| (input.name(), input.prefix()))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        roles,
        [
            (assumptions::Assumption::NAME, Some(Facts)),
            (assumptions::Assumption::NAME, Some(Local)),
            (assumptions_universe::AssumptionUniverseSupport::NAME, None),
        ]
        .into()
    );
}

#[test]
fn actual_admission_scope_programs_bind_their_exact_declared_roles() {
    let model = model().unwrap();
    let budget = resources::ResourceBudget::fixed(128 << 20).unwrap();
    let mut executions = 0;
    let mut supports = 0;
    let mut failures = Vec::new();
    for invariant in model
        .invariants()
        .iter()
        .filter(|check| check.purpose == InvariantPurpose::Admission)
    {
        let kernel = (invariant.create)(&budget);
        if let Some(scope) = kernel.execution_scope() {
            executions += 1;
            if let Err(error) = scope.program(invariant.inputs.clone(), &model, &budget) {
                failures.push((invariant.name, error.to_string()));
            }
        }
        if let Some(scope) = kernel.support_scope() {
            supports += 1;
            if let Err(error) = scope.program(invariant.inputs.clone(), &model, &budget) {
                failures.push((invariant.name, error.to_string()));
            }
        }
    }
    assert!(executions > 0 && supports > 0);
    assert!(
        failures.is_empty(),
        "actual selected admission scope endpoints must match declared input roles: {failures:?}"
    );
    assert_eq!(budget.reserved(), 0);
}

#[test]
fn qualified_assertions_preserve_alternatives_and_require_typed_attribution() {
    let model = model().unwrap();
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "a.py".into(), b"x").unwrap();
    let other = SourceArtifact::from_bytes(input.id(), "b.py".into(), b"y").unwrap();
    let occurrence = Occurrence {
        source: source.id(),
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Read,
        structural_path: vec![],
    };
    let provider = Provider {
        tool: "ruff".into(),
        revision: "0.0.11".into(),
        build_digest: ContentHash::of(b"build"),
    };
    let ctx = context("3.14");
    let (run, families) = ProviderRun::new(
        provider.id(),
        ctx.id(),
        input.id(),
        ctx.config_digest,
        [FactFamily::Syntax],
    )
    .unwrap();
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    let (condition, nodes) = Diagram::always().records();
    let qualification = AssertionQualification {
        assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
        context: ctx.id(),
        scope: scope.id(),
        condition: condition.id(),
        modality: Modality::Candidate,
        approximation: Approximation::Over,
    };
    let assertion = SyntaxObservation {
        qualification: qualification.id(),
        occurrence: occurrence.id(),
        spelling: "x".into(),
    };
    for qualified in [
        AssertionQualification {
            modality: Modality::Definite,
            ..qualification.clone()
        },
        AssertionQualification {
            approximation: Approximation::Exact,
            ..qualification.clone()
        },
        AssertionQualification {
            context: context("3.13").id(),
            ..qualification.clone()
        },
        AssertionQualification {
            condition: Diagram::never().id(),
            ..qualification.clone()
        },
    ] {
        assert_ne!(
            assertion.id(),
            SyntaxObservation {
                qualification: qualified.id(),
                ..assertion.clone()
            }
            .id()
        );
    }
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Syntax,
        name: "native-tree".into(),
    };
    let evidence = Evidence::Occurrence {
        occurrence: occurrence.id(),
    };
    let support = SyntaxSupport {
        assertion: assertion.id(),
        run: run.id(),
        surface: surface.id(),
        evidence: evidence.id(),
        origin: Origin::SourceObservation,
        mode: ExtractionMode::NativeTraversal,
        fidelity: Fidelity::NativeStructural,
    };
    let mut repeated = support.clone();
    repeated.mode = ExtractionMode::ReportDecode;
    assert_ne!(support.id(), repeated.id());
    assert_eq!(support.assertion, repeated.assertion);
    let roundtrip = Batch::new(&model, vec![support.clone(), repeated.clone()], &budget()).unwrap();
    assert_eq!(
        SyntaxSupport::decode(roundtrip.arrow()).unwrap(),
        roundtrip.rows()
    );
    let mut base = BTreeMap::new();
    insert(&model, &mut base, vec![assumptions::AssumptionSet::empty()]);
    insert(&model, &mut base, nodes);
    insert(&model, &mut base, vec![condition]);
    insert(&model, &mut base, vec![source.clone(), other.clone()]);
    insert(&model, &mut base, vec![occurrence.clone()]);
    insert(&model, &mut base, vec![scope]);
    insert(&model, &mut base, vec![qualification.clone()]);
    insert(&model, &mut base, vec![run.clone()]);
    insert(&model, &mut base, families);
    insert(&model, &mut base, vec![surface.clone()]);
    insert(&model, &mut base, vec![evidence]);
    insert(&model, &mut base, vec![assertion.clone()]);
    insert(&model, &mut base, vec![support.clone(), repeated]);
    let invariant = lctx_model::domain::validation::invariants_for::<SyntaxSupport>().remove(0);
    check(&invariant, &base).unwrap();
    for case in [
        "missing-support",
        "wrong-context",
        "wrong-provider",
        "wrong-family",
        "wrong-input",
        "wrong-evidence",
        "wrong-subject",
        "wrong-invocation-evidence",
    ] {
        let mut batches = base.clone();
        let mut changed_support = support.clone();
        match case {
            "missing-support" => {
                batches.remove(SyntaxSupport::NAME);
            }
            "wrong-context" => {
                let mut changed = qualification.clone();
                changed.context = context("3.13").id();
                let mut assertion = assertion.clone();
                assertion.qualification = changed.id();
                changed_support.assertion = assertion.id();
                insert(&model, &mut batches, vec![changed]);
                insert(&model, &mut batches, vec![assertion]);
                insert(&model, &mut batches, vec![changed_support]);
            }
            "wrong-provider" | "wrong-family" => {
                let mut changed = surface.clone();
                if case == "wrong-provider" {
                    changed.provider = Provider {
                        revision: "different".into(),
                        ..provider.clone()
                    }
                    .id();
                } else {
                    changed.family = FactFamily::Calls;
                }
                changed_support.surface = changed.id();
                insert(&model, &mut batches, vec![changed]);
                insert(&model, &mut batches, vec![changed_support]);
            }
            "wrong-input" => {
                let mut changed = run.clone();
                changed.input = InputRevision::from_entries(vec![ManifestEntry {
                    path: "x".into(),
                    content: ContentHash::of(b"x"),
                    byte_len: 1,
                }])
                .unwrap()
                .id();
                changed_support.run = changed.id();
                insert(
                    &model,
                    &mut batches,
                    vec![RunFamily {
                        run: changed.id(),
                        family: FactFamily::Syntax,
                    }],
                );
                insert(&model, &mut batches, vec![changed]);
                insert(&model, &mut batches, vec![changed_support]);
            }
            "wrong-evidence" | "wrong-invocation-evidence" => {
                let changed = if case == "wrong-evidence" {
                    Evidence::SourceSpan {
                        source: other.id(),
                        start: 0,
                        end: 1,
                    }
                } else {
                    Evidence::Invocation {
                        run: ProviderRun {
                            configuration: ContentHash::of(b"other"),
                            ..run.clone()
                        }
                        .id(),
                    }
                };
                changed_support.evidence = changed.id();
                insert(&model, &mut batches, vec![changed]);
                insert(&model, &mut batches, vec![changed_support]);
            }
            _ => {
                let changed = Occurrence {
                    source: other.id(),
                    ..occurrence.clone()
                };
                let changed_assertion = SyntaxObservation {
                    occurrence: changed.id(),
                    ..assertion.clone()
                };
                changed_support.assertion = changed_assertion.id();
                insert(&model, &mut batches, vec![changed, occurrence.clone()]);
                insert(&model, &mut batches, vec![changed_assertion]);
                insert(&model, &mut batches, vec![changed_support]);
            }
        }
        assert!(check(&invariant, &batches).is_err(), "{case}");
    }
    let foreign_input = InputRevision::from_entries(vec![ManifestEntry {
        path: "foreign.py".into(),
        content: ContentHash::of(b"z"),
        byte_len: 1,
    }])
    .unwrap();
    let corpus_input = InputRevision::from_entries(vec![ManifestEntry {
        path: "corpus.txt".into(),
        content: ContentHash::of(b"corpus"),
        byte_len: 6,
    }])
    .unwrap();
    let foreign =
        SourceArtifact::from_bytes(foreign_input.id(), "foreign.py".into(), b"z").unwrap();
    for case in [
        "same-input",
        "unrelated-input",
        "corpus-member",
        "missing-corpus-member",
        "outside-artifact-scope",
    ] {
        let mut batches = base.clone();
        let evaluation_source = match case {
            "same-input" => &source,
            "outside-artifact-scope" => &other,
            _ => &foreign,
        };
        let evaluation = Occurrence {
            source: evaluation_source.id(),
            role: OccurrenceRole::Predicate,
            ..occurrence.clone()
        };
        let atom = EvaluationAtom {
            evaluation: evaluation.id(),
            context: ctx.id(),
            predicate: super_predicate(),
            operand: None,
        };
        let (condition, nodes) = Diagram::from_atom(atom.id()).records();
        let corpus = matches!(case, "corpus-member" | "missing-corpus-member");
        let scope = if corpus {
            CoverageScope::Input {
                input: corpus_input.id(),
            }
        } else if case == "outside-artifact-scope" {
            CoverageScope::Artifact {
                artifact: source.id(),
            }
        } else {
            CoverageScope::Input { input: input.id() }
        };
        let qualified = AssertionQualification {
            scope: scope.id(),
            condition: condition.id(),
            ..qualification.clone()
        };
        let asserted = SyntaxObservation {
            qualification: qualified.id(),
            ..assertion.clone()
        };
        let invocation = ProviderRun {
            input: if corpus {
                corpus_input.id()
            } else {
                input.id()
            },
            ..run.clone()
        };
        let supporting = SyntaxSupport {
            assertion: asserted.id(),
            run: invocation.id(),
            ..support.clone()
        };
        insert(
            &model,
            &mut batches,
            vec![source.clone(), other.clone(), foreign.clone()],
        );
        insert(&model, &mut batches, vec![occurrence.clone(), evaluation]);
        insert(
            &model,
            &mut batches,
            vec![lctx_model::domain::value::Predicate::Truthy],
        );
        insert(&model, &mut batches, vec![atom]);
        insert(&model, &mut batches, nodes);
        insert(&model, &mut batches, vec![condition]);
        insert(&model, &mut batches, vec![scope]);
        insert(&model, &mut batches, vec![qualified]);
        insert(&model, &mut batches, vec![asserted]);
        insert(
            &model,
            &mut batches,
            vec![RunFamily {
                run: invocation.id(),
                family: FactFamily::Syntax,
            }],
        );
        insert(&model, &mut batches, vec![invocation]);
        insert(&model, &mut batches, vec![supporting]);
        if corpus {
            let mut members = vec![CorpusLibrary {
                corpus: corpus_input.id(),
                library: input.id(),
            }];
            if case == "corpus-member" {
                members.push(CorpusLibrary {
                    corpus: corpus_input.id(),
                    library: foreign_input.id(),
                });
            }
            insert(&model, &mut batches, members);
        }
        assert_eq!(
            check(&invariant, &batches).is_ok(),
            matches!(case, "same-input" | "corpus-member"),
            "{case}"
        );
    }
}

#[test]
fn assertion_condition_context_and_evidence_bounds_are_stored_invariants() {
    let model = model().unwrap();
    let ctx = context("3.14");
    let other = context("3.13");
    let input = InputRevision::from_entries(vec![]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(), "x.py".into(), b"x").unwrap();
    let occurrence = Occurrence {
        source: source.id(),
        start: 0,
        end: 1,
        syntax_kind: SyntaxKind::ExprName,
        role: OccurrenceRole::Predicate,
        structural_path: vec![],
    };
    let atom = EvaluationAtom {
        evaluation: occurrence.id(),
        context: ctx.id(),
        predicate: super_predicate(),
        operand: None,
    };
    let (condition, nodes) = Diagram::from_atom(atom.id()).records();
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    let mut batches = BTreeMap::new();
    insert(
        &model,
        &mut batches,
        vec![assumptions::AssumptionSet::empty()],
    );
    insert(&model, &mut batches, vec![atom]);
    insert(&model, &mut batches, nodes);
    insert(&model, &mut batches, vec![condition.clone()]);
    for context in [ctx.id(), other.id()] {
        insert(
            &model,
            &mut batches,
            vec![AssertionQualification {
                assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
                context,
                scope: scope.id(),
                condition: condition.id(),
                modality: Modality::Definite,
                approximation: Approximation::Exact,
            }],
        );
        assert_eq!(
            check(
                &lctx_model::domain::validation::invariants_for::<AssertionQualification>()[0],
                &batches
            )
            .is_ok(),
            context == ctx.id()
        );
    }
    insert(&model, &mut batches, vec![source.clone()]);
    for end in [1, 2] {
        insert(
            &model,
            &mut batches,
            vec![Evidence::SourceSpan {
                source: source.id(),
                start: 0,
                end,
            }],
        );
        assert_eq!(
            check(
                &lctx_model::domain::validation::invariants_for::<Evidence>()[0],
                &batches
            )
            .is_ok(),
            end == 1
        );
    }
}
fn super_predicate() -> Id<lctx_model::domain::value::Predicate> {
    lctx_model::domain::value::Predicate::Truthy.id()
}

#[test]
fn assertion_model_requires_its_concrete_support_companion() {
    let complete = ValidatedModel::declared(facts_relations()).unwrap();
    let without_support = complete
        .relations()
        .iter()
        .filter(|r| r.name() != SyntaxSupport::NAME)
        .cloned()
        .collect();
    let error = ValidatedModel::declared(without_support).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("requires companion relation syntax_supports")
    );
    ValidatedModel::declared(complete.relations().to_vec()).unwrap();
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}
