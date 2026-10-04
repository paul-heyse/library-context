//! Model contract fixtures, not a qualified behavioral producer.
#[path = "../../lctx-model/tests/fixtures/analysis_support.rs"]
mod analysis_fixture;
use lctx_model::domain::{
    analysis::{expected::CoverageAdmission, local, sources::CapturedSources},
    artifact::*,
    assertion::*,
    assumptions::*,
    attribution::*,
    calls::*,
    conditions::*,
    flow::{FlowUse, FlowValueObservation, FlowValueSupport},
    input::*,
    normalized::{
        coverage::{
            Capability, EvidenceAvailability, NormalizationComputation, NormalizationCoverage,
        },
        entities::EntityRef,
    },
    source::*,
    stages::*,
    transfer::{local::*, *},
    value::*,
    *,
};
use lctx_postgres::generations::{
    Error, GenerationAttempt, GenerationId, GenerationStore, SealedAttempt, ValidatedAttempt,
};
use lctx_postgres::testing::DisposableDatabase;
use std::sync::Arc;

#[tokio::test]
async fn transfer_control_selection_survive_postgres_and_cross_scope_call_site_refuses() {
    let db = DisposableDatabase::start().await;
    let writer = db.writer.clone();
    let reader = db.reader.clone();
    let model = Arc::new(transfer_model());
    let store = GenerationStore::install(db.owner.clone(), model.clone())
        .await
        .unwrap();
    let schedule = transfer_schedule(&model);
    let bytes = b"x y";
    let input = InputRevision::from_entries(vec![
        ManifestEntry {
            path: "x.py".into(),
            content: ContentHash::of(bytes),
            byte_len: 3,
        },
        ManifestEntry {
            path: "other.py".into(),
            content: ContentHash::of(b"z"),
            byte_len: 1,
        },
    ])
    .unwrap();
    let origin = InputOrigin::Tree {
        label: "transfer-contract".into(),
    };
    let acquisition = InputAcquisition {
        input: input.id(),
        origin: origin.id(),
    };
    let source = SourceArtifact::from_bytes(input.id(), "x.py".into(), bytes).unwrap();
    let other = SourceArtifact::from_bytes(input.id(), "other.py".into(), b"z").unwrap();
    let scope = CoverageScope::Artifact {
        artifact: source.id(),
    };
    let context = AnalysisContext {
        python_version: "3.14.7".into(),
        python_platform: "linux".into(),
        search_path: vec!["$input".into()],
        site_package_path: vec![],
        config_digest: ContentHash::of(b"cfg"),
        environment_digest: input.manifest,
        lock_digest: None,
    };
    let provider = Provider {
        tool: "contract-fixture".into(),
        revision: "one".into(),
        build_digest: ContentHash::of(b"build"),
    };
    let (run, families) = ProviderRun::new(
        provider.id(),
        context.id(),
        input.id(),
        context.config_digest,
        [FactFamily::Flow],
    )
    .unwrap();
    let surface = ProviderSurface {
        provider: provider.id(),
        family: FactFamily::Flow,
        name: "flow".into(),
    };
    let module = ProviderModule::Bundled {
        provider: provider.id(),
        bundle: ModuleBundle::Typeshed,
        name: "x".into(),
    };
    let symbol = ProviderSymbol {
        provider: provider.id(),
        context: context.id(),
        module: module.id(),
        native_key: "f".into(),
        name: "f".into(),
        kind: SymbolKind::Function,
    };
    let source_module = Module {
        source: source.id(),
        qualified_name: "x".into(),
    };
    let entity = EntityRef::Module {
        module: source_module.id(),
    };
    let occurrences: Vec<_> = (0..3)
        .map(|i| Occurrence {
            source: source.id(),
            start: i,
            end: i + 1,
            syntax_kind: SyntaxKind::ExprName,
            role: OccurrenceRole::Read,
            structural_path: vec![i as i32],
        })
        .collect();
    let other_site = Occurrence {
        source: other.id(),
        start: 0,
        end: 1,
        ..occurrences[0].clone()
    };
    let roots: Vec<_> = occurrences
        .iter()
        .map(|o| PlaceRoot::Occurrence { occurrence: o.id() })
        .collect();
    let path = AccessPath::empty();
    let places: Vec<_> = roots
        .iter()
        .map(|r| Place {
            root: r.id(),
            path: path.id(),
        })
        .collect();
    let predicate = Predicate::Truthy;
    let other_root = PlaceRoot::Occurrence {
        occurrence: other_site.id(),
    };
    let other_place = Place {
        root: other_root.id(),
        path: path.id(),
    };
    // These nominal control fixtures exercise proofs without claiming a completed behavioral
    // producer. Their admitted domain records the unavailable normalization explicitly.
    let artifact_use = ArtifactUse {
        artifact: source.id(),
        input: input.id(),
        role: SourceRole::Release,
    };
    let provider_coverage = ProviderCoverage {
        scope: scope.id(),
        provider: Some(provider.id()),
        context: context.id(),
        family: FactFamily::Flow,
        run: Some(run.id()),
        status: CoverageStatus::Partial,
        reason: Some(obligation::ObligationKind::IncompleteCoverage),
        diagnostic: Some("nominal flow contract fixture".into()),
    };
    let computations: Vec<_> = [
        Capability::FlowLinks,
        Capability::FlowEvents,
        Capability::Bindings,
    ]
    .into_iter()
    .map(|capability| NormalizationComputation {
        capability,
        policy: ContentHash::of(b"nominal fixture"),
        producer: "fixture_facts".into(),
        declaration: ContentHash::of(b"unavailable normalization"),
        profile: Profile::Behavioral.name().into(),
        availability: EvidenceAvailability::Unavailable,
    })
    .collect();
    let normalized_coverage: Vec<_> = computations
        .iter()
        .map(|computation| NormalizationCoverage {
            computation: computation.id(),
            scope: scope.id(),
            context: context.id(),
            availability: EvidenceAvailability::Unavailable,
        })
        .collect();
    for boundary in 0..3 {
        let atom = EvaluationAtom {
            evaluation: occurrences[1].id(),
            context: context.id(),
            predicate: predicate.id(),
            operand: Some(if boundary == 2 {
                other_place.id()
            } else {
                places[1].id()
            }),
        };
        let diagram = Diagram::from_atom(atom.id());
        let (condition, nodes) = diagram.records();
        let qualification = AssertionQualification {
            assumptions: lctx_model::domain::assumptions::AssumptionSet::empty_id(),
            context: context.id(),
            scope: scope.id(),
            condition: condition.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
        };
        let influence = ControlInfluence {
            qualification: qualification.id(),
            input: places[1].id(),
            atom: atom.id(),
            evaluation: atom.evaluation,
        };
        let evidence = Evidence::Occurrence {
            occurrence: occurrences[0].id(),
        };
        let influence_evidence = Evidence::Occurrence {
            occurrence: atom.evaluation,
        };
        let mut foundation = analysis_fixture::SupportFixture::new(
            input.id(),
            run.id(),
            surface.id(),
            evidence.id(),
            &qualification,
            &diagram,
            occurrences[0].id(),
            places[0].id(),
        );
        let mut control_foundation = analysis_fixture::SupportFixture::new(
            input.id(),
            run.id(),
            surface.id(),
            influence_evidence.id(),
            &qualification,
            &diagram,
            atom.evaluation,
            influence.input,
        );
        let key = TransferKey {
            owner: entity.id(),
            input: places[0].id(),
            output: places[2].id(),
            context: context.id(),
            scope: scope.id(),
            modality: Modality::Definite,
            approximation: Approximation::Exact,
            kind: TransferKind::Identity,
            call_site: Some(if boundary == 1 {
                other_site.id()
            } else {
                occurrences[0].id()
            }),
            provenance: ProvenanceClass::FlowLocal,
        };
        let branch = TransferBranch::new(
            key.clone(),
            qualification.clone(),
            diagram.clone(),
            &budget(),
        )
        .unwrap();
        let alternative = branch.alternative();
        let selection = branch
            .selection(&influence, &qualification)
            .unwrap()
            .unwrap();
        let mut execution = schedule.execute();
        let attempt = store
            .begin_conformance(writer.clone(), &mut execution, budget())
            .await
            .unwrap();
        let g = attempt.generation();
        let mut native_inventory = analysis::native::NativeInventory::new(&budget());
        let mut access = execution.begin("fixture_facts").unwrap();
        macro_rules! copy { ($($row:expr),+ $(,)?) => { $(copy_rows(&mut access, &attempt, &model, vec![$row.clone()], &mut native_inventory).await.unwrap();)+ }; }
        macro_rules! copies { ($($rows:expr),+ $(,)?) => { $(copy_rows(&mut access, &attempt, &model, $rows.clone(), &mut native_inventory).await.unwrap();)+ }; }
        copy!(
            AssumptionSet::empty(),
            input,
            origin,
            acquisition,
            source,
            other,
            scope,
            context,
            provider,
            run,
            surface,
            module,
            source_module,
            entity,
            symbol,
            path,
            predicate,
            atom,
            condition,
            qualification,
            evidence,
            influence_evidence,
            other_site,
            other_root,
            other_place
        );
        copy!(
            foundation.parameters,
            foundation.definition,
            artifact_use,
            provider_coverage
        );
        copies!(computations, normalized_coverage);
        copy_rows::<Assumption>(&mut access, &attempt, &model, vec![], &mut native_inventory)
            .await
            .unwrap();
        copy_rows::<AssumptionUniverse>(
            &mut access,
            &attempt,
            &model,
            vec![],
            &mut native_inventory,
        )
        .await
        .unwrap();
        copy_rows::<AssumptionSetMember>(
            &mut access,
            &attempt,
            &model,
            vec![],
            &mut native_inventory,
        )
        .await
        .unwrap();
        copies!(
            vec![foundation.use_.clone(), control_foundation.use_.clone()],
            vec![
                foundation.observation.clone(),
                control_foundation.observation.clone()
            ],
            vec![
                foundation.support.clone(),
                control_foundation.support.clone()
            ],
            families,
            occurrences,
            roots,
            places,
            nodes
        );
        copies!(
            ArtifactChunk::split(&source, bytes)
                .unwrap()
                .collect::<Vec<_>>(),
            ArtifactChunk::split(&other, b"z")
                .unwrap()
                .collect::<Vec<_>>()
        );
        let projected = native_inventory.collect().unwrap();
        let batch = Batch::new(
            &model,
            projected.premises.iter().cloned().collect(),
            &budget(),
        )
        .unwrap();
        access
            .write::<analysis::native::NativeAssertionPremise, _>(async |permit| {
                attempt.copy(permit, &batch).await
            })
            .await
            .unwrap();
        let batch = Batch::new(
            &model,
            projected.qualifications.iter().cloned().collect(),
            &budget(),
        )
        .unwrap();
        access
            .write::<analysis::native::NativeQualification, _>(async |permit| {
                attempt.copy(permit, &batch).await
            })
            .await
            .unwrap();
        access
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .unwrap();
        let mut access = execution.begin("fixture_controls").unwrap();
        let admission_budget = budget();
        let captured = CapturedSources::capture(&access, &admission_budget).unwrap();
        let mut admission = CoverageAdmission::new(&captured, &admission_budget).unwrap();
        macro_rules! visit {
            ($ty:ty, $rows:expr) => {
                admission
                    .visit(
                        &access.read::<$ty>().unwrap(),
                        &<$ty as Record>::encode($rows).unwrap(),
                    )
                    .unwrap();
            };
        }
        visit!(InputRevision, std::slice::from_ref(&input));
        visit!(SourceArtifact, &[source.clone(), other.clone()]);
        visit!(ArtifactUse, std::slice::from_ref(&artifact_use));
        visit!(CoverageScope, std::slice::from_ref(&scope));
        visit!(ProviderCoverage, std::slice::from_ref(&provider_coverage));
        visit!(NormalizationComputation, &computations);
        visit!(NormalizationCoverage, &normalized_coverage);
        let (invocation, parents, source_receipts, projections) = local::Invocation::admitted(
            input.id(),
            context.id(),
            foundation.definition.id(),
            None,
            [],
            &captured,
            [],
            &admission_budget,
        )
        .unwrap();
        let admitted = local::coverage::admit(
            &invocation,
            &foundation.definition,
            analysis::AnalysisCapability::Transfers,
            &admission,
            &admission_budget,
        )
        .unwrap();
        let status = analysis::AnalysisStatus::Partial;
        let reason = Some(obligation::ObligationKind::IncompleteCoverage);
        let outcome = local::Outcome {
            invocation: invocation.id(),
            status,
            reason,
        };
        let mut requirements = Vec::new();
        let mut required = Vec::new();
        let mut coverage = Vec::new();
        let mut coverage_premises = Vec::new();
        let mut coverage_sources = Vec::new();
        for domain in admitted.scopes() {
            let (requirement, members) = domain.expectation().records().unwrap();
            requirements.push(requirement);
            required.extend(members);
            let (row, premises) = local::coverage::assess(
                domain.expectation(),
                domain.observations(),
                status,
                reason,
                &admission_budget,
            )
            .unwrap();
            coverage.push(row);
            coverage_premises.extend(premises);
            coverage_sources.extend(
                domain
                    .observations()
                    .iter()
                    .map(|observation| observation.source().clone()),
            );
        }
        bind_support(&mut foundation, &invocation, &qualification, &diagram);
        bind_support(
            &mut control_foundation,
            &invocation,
            &qualification,
            &diagram,
        );
        let control_support = ControlSupport {
            assertion: influence.id(),
            source: control_foundation.derived.id(),
        };
        let support = TransferSupport {
            assertion: alternative.id(),
            source: foundation.derived.id(),
        };
        let mut ignored_native = analysis::native::NativeInventory::new(&budget());
        // The control rows are nominal contract fixtures, not outputs of a behavioral producer.
        macro_rules! copy { ($($row:expr),+ $(,)?) => { $(copy_rows(&mut access, &attempt, &model, vec![$row.clone()], &mut ignored_native).await.unwrap();)+ }; }
        macro_rules! copies { ($($rows:expr),+ $(,)?) => { $(copy_rows(&mut access, &attempt, &model, $rows.clone(), &mut ignored_native).await.unwrap();)+ }; }
        copy!(
            foundation.invocation,
            influence,
            control_support,
            key,
            alternative,
            selection,
            support,
            outcome
        );
        copies!(
            parents,
            source_receipts,
            projections,
            requirements,
            required,
            coverage,
            coverage_premises,
            coverage_sources
        );
        copy_rows::<local::InvocationSource>(
            &mut access,
            &attempt,
            &model,
            vec![],
            &mut ignored_native,
        )
        .await
        .unwrap();
        copies!(
            vec![
                foundation.subject.clone(),
                control_foundation.subject.clone()
            ],
            vec![
                foundation.proposition.clone(),
                control_foundation.proposition.clone()
            ],
            vec![
                foundation.derivation.clone(),
                control_foundation.derivation.clone()
            ],
            vec![
                foundation.native.clone(),
                foundation.derived.clone(),
                control_foundation.native.clone(),
                control_foundation.derived.clone()
            ],
            foundation
                .members
                .iter()
                .chain(&control_foundation.members)
                .cloned()
                .collect::<Vec<_>>()
        );
        access
            .complete(&attempt, ProviderOutcome::Complete)
            .await
            .unwrap();
        let sealed = attempt.seal(execution.finish().unwrap()).await.unwrap();
        let mut g_h = FixturePublication {
            store: store.clone(),
            generation: g,
            sealed: Some(sealed),
            validated: None,
        };
        if boundary != 0 {
            let error = g_h.validate(&budget()).await.unwrap_err();
            assert!(
                matches!(error, Error::Model(_)) && error.to_string().contains("scope"),
                "{error}"
            );
            assert!(g_h.publish().await.is_err());
            g_h.abort().await.unwrap();
        } else {
            g_h.validate(&budget()).await.unwrap();
            g_h.publish().await.unwrap();
            let mut lease = store.pin(&reader, g, budget()).await.unwrap();
            assert_eq!(lease.read::<TransferKey>().await.unwrap().rows(), &[key]);
            assert_eq!(
                lease.read::<TransferAlternative>().await.unwrap().rows(),
                &[alternative]
            );
            assert_eq!(
                lease.read::<TransferSupport>().await.unwrap().rows(),
                &[support]
            );
            assert_eq!(
                lease.read::<ControlInfluence>().await.unwrap().rows(),
                std::slice::from_ref(&influence)
            );
            assert_eq!(
                lease.read::<ControlSupport>().await.unwrap().rows(),
                std::slice::from_ref(&control_support)
            );
            assert_eq!(
                lease.read::<Selection>().await.unwrap().rows(),
                std::slice::from_ref(&selection)
            );
            let derivation: (String, String, Vec<u8>) =
                sqlx::query_as(sqlx::AssertSqlSafe(format!(
                    "SELECT rule,conclusion_relation,conclusion_id FROM {}.derivations WHERE source_relation='{}'",
                    g.schema(), Selection::NAME
                )))
                .fetch_one(&reader)
                .await
                .unwrap();
            assert_eq!(
                derivation,
                (
                    "control_selects_transfer".into(),
                    Selection::NAME.into(),
                    selection.id().bytes().to_vec()
                )
            );
            let premises: Vec<(String,String,Vec<u8>)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "SELECT role,premise_relation,premise_id FROM {}.derivation_premises WHERE source_relation='{}' ORDER BY role",g.schema(),Selection::NAME))).fetch_all(&reader).await.unwrap();
            assert_eq!(
                premises,
                vec![
                    (
                        "alternative".into(),
                        TransferAlternative::NAME.into(),
                        branch.alternative().id().bytes().to_vec()
                    ),
                    (
                        "influence".into(),
                        ControlInfluence::NAME.into(),
                        influence.id().bytes().to_vec()
                    )
                ]
            );
        }
    }
}

/// A fresh attempt budget; these controls do not share reservations across batches.
fn budget() -> lctx_model::domain::resources::ResourceBudget {
    lctx_model::domain::resources::ResourceBudget::fixed(
        lctx_model::domain::resources::DEFAULT_MEMORY_BYTES,
    )
    .unwrap()
}

/// This conformance control owns only Local transfer/control contracts, not later composition or
/// producer entry replay. Its empty witness targets retain the complete nominal FK and invariant
/// declarations of the Local support alternatives, without activating their producers.
fn transfer_model() -> ValidatedModel {
    let mut relations = facts_relations();
    relations.extend(analysis::early_relations());
    relations.extend(analysis::local::relations());
    relations.extend(transfer::local::relations());
    relations.extend([
        Relation::of::<ControlInfluence>(),
        Relation::of::<ControlSupport>(),
        Relation::of::<Selection>(),
        Relation::of::<local_semantics::LocalContribution>(),
        Relation::of::<local_semantics::LocalGuardContribution>(),
        Relation::of::<local_theory::TheoryWitness>(),
        Relation::of::<conditions::entry::EntryValueWitness>(),
        Relation::of::<conditions::entry::EntryAccessSource>(),
        Relation::of::<conditions::stability::StabilityWitness>(),
        Relation::of::<normalized::entities::ParameterEntityLink>(),
        Relation::of::<normalized::entities::OccurrenceOwnership>(),
        Relation::of::<normalized::links::TestOperandTypeAssessment>(),
        Relation::of::<normalized::links::TestOperandTypeLink>(),
        Relation::of::<local_theory::TypeDomain>(),
        Relation::of::<local_theory::BuiltinOperandWitness>(),
    ]);
    relations.extend(normalized::coverage::relations());
    relations.extend([
        Relation::of::<normalized::entities::CallableEntity>(),
        Relation::of::<normalized::entities::ClassEntity>(),
        Relation::of::<normalized::entities::ParameterEntity>(),
        Relation::of::<normalized::entities::FieldEntity>(),
        Relation::of::<EntityRef>(),
    ]);
    ValidatedModel::validate(relations).unwrap()
}

// Preserve the existing lifecycle checks while delegating every transition to real store capabilities.
struct FixturePublication {
    store: GenerationStore,
    generation: GenerationId,
    sealed: Option<SealedAttempt>,
    validated: Option<ValidatedAttempt>,
}
impl FixturePublication {
    async fn validate(&mut self, budget: &resources::ResourceBudget) -> Result<ContentHash, Error> {
        let validated = self
            .sealed
            .take()
            .ok_or(Error::State)?
            .validate_with(budget)
            .await?;
        let content = validated.content();
        self.validated = Some(validated);
        Ok(content)
    }
    async fn publish(&mut self) -> Result<(), Error> {
        self.validated
            .take()
            .ok_or(Error::State)?
            .publish()
            .await
            .map(drop)
    }
    async fn abort(&mut self) -> Result<(), Error> {
        if let Some(validated) = self.validated.take() {
            validated.abort().await.map(drop)
        } else if let Some(sealed) = self.sealed.take() {
            sealed.abort().await.map(drop)
        } else {
            self.store.abort(self.generation).await.map(drop)
        }
    }
}
async fn copy_rows<R: Record>(
    access: &mut StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    model: &ValidatedModel,
    rows: Vec<R>,
    native: &mut analysis::native::NativeInventory,
) -> Result<(), ModelError> {
    let batch = Batch::new(model, rows, &budget())?;
    if analysis::native::NativeInventory::inputs()
        .iter()
        .any(|i| i.name() == R::NAME)
    {
        native.visit(R::NAME, batch.arrow())?;
    }
    access
        .write::<R, _>(async |permit| attempt.copy(permit, &batch).await)
        .await
}
fn transfer_schedule(model: &ValidatedModel) -> Schedule {
    use analysis::local::{
        AnalysisDerivation, AnalysisDerivationPremise, AnalysisInvocation, AnalysisProposition,
        ObligationSubject, support::SupportSource,
    };
    macro_rules! outputs { ($($ty:ty),+ $(,)?) => { vec![$(RelationUse::of::<$ty>()),+] }; }
    let facts = outputs!(
        InputRevision,
        InputOrigin,
        InputAcquisition,
        SourceArtifact,
        CoverageScope,
        AnalysisContext,
        Provider,
        ProviderRun,
        ProviderSurface,
        ProviderModule,
        Module,
        EntityRef,
        ProviderSymbol,
        AccessPath,
        Predicate,
        EvaluationAtom,
        Condition,
        AssertionQualification,
        AssumptionSet,
        AssumptionSetMember,
        Assumption,
        AssumptionUniverse,
        Evidence,
        Occurrence,
        PlaceRoot,
        Place,
        RunFamily,
        ConditionNode,
        ArtifactChunk,
        FlowUse,
        FlowValueObservation,
        FlowValueSupport,
        analysis::native::NativeAssertionPremise,
        analysis::native::NativeQualification,
        analysis::MethodParameters,
        analysis::AnalysisDefinition,
        ArtifactUse,
        ProviderCoverage,
        NormalizationComputation,
        NormalizationCoverage
    );
    let controls = outputs!(
        AnalysisInvocation,
        ControlInfluence,
        ControlSupport,
        TransferKey,
        TransferAlternative,
        Selection,
        TransferSupport,
        ObligationSubject,
        AnalysisProposition,
        AnalysisDerivation,
        SupportSource,
        AnalysisDerivationPremise,
        local::SourceReceipt,
        local::AnalysisInput,
        local::ProjectionInput,
        local::InvocationSource,
        local::Outcome,
        local::CoverageRequirement,
        local::CoverageRequiredSource,
        local::Coverage,
        local::AnalysisCoveragePremise,
        local::CoverageSource
    );
    let stage = |name, inputs, outputs| Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Behavioral],
        effect: Effect::Extraction,
        code: ContentHash::of(b"Local transfer conformance fixture"),
        configuration: ContentHash::of(b"fixture"),
    };
    let inputs = outputs!(
        InputRevision,
        SourceArtifact,
        ArtifactUse,
        CoverageScope,
        ProviderCoverage,
        NormalizationComputation,
        NormalizationCoverage,
        analysis::AnalysisDefinition
    )
    .into_iter()
    .map(RelationUse::completed_store)
    .collect();
    Schedule::build_with_publications(
        model,
        vec![
            stage("fixture_facts", vec![], facts),
            stage("fixture_controls", inputs, controls),
        ],
        &[],
        Profile::Behavioral,
        vec![PublicationGroup::new(
            PublicationBoundary::Facts,
            vec!["fixture_facts"],
        )],
    )
    .unwrap()
}

// Re-emit the same native premises after binding the proof to actual completed source receipts.
fn bind_support(
    fixture: &mut analysis_fixture::SupportFixture,
    invocation: &local::Invocation,
    qualification: &AssertionQualification,
    diagram: &Diagram,
) {
    use local::support::{
        AnalysisDerivation, EvidencePremise, QualificationOperation, SupportSource,
    };
    let (derivation, proposition, members, _) = AnalysisDerivation::emit(
        invocation,
        &fixture.definition,
        fixture.subject.id(),
        fixture.proposition.channel,
        fixture.proposition.phase,
        QualificationOperation::Conjunction,
        &[EvidencePremise::native(
            &fixture.native,
            &fixture.native_qualification,
            qualification,
            diagram,
        )
        .unwrap()],
        &budget(),
    )
    .unwrap();
    fixture.invocation = invocation.clone();
    fixture.derivation = derivation;
    fixture.proposition = proposition;
    fixture.members = members;
    fixture.derived = SupportSource::AnalysisDerivation {
        derivation: fixture.derivation.id(),
    };
}
