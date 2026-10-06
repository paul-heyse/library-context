use lctx_model::domain::{
    analysis::{
        catalog_core,
        expected::CoverageAdmission,
        local,
        sources::{CapturedSources, CompletedInput, SourceSnapshot},
        *,
    },
    attribution::{CoverageStatus, FactFamily, ProviderCoverage},
    input::{ArtifactUse, InputRevision, SourceRole},
    normalized::coverage::{
        Capability, EvidenceAvailability, NormalizationComputation, NormalizationCoverage,
    },
    source::{CoverageScope, SourceArtifact},
    stages::*,
    *,
};
use std::collections::BTreeMap;
fn nominal<T>(v: u8) -> Id<T> {
    serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<
        _,
        serde::de::value::Error,
    >::new([v; 16].into_iter()))
    .unwrap()
}
fn budget() -> resources::ResourceBudget {
    resources::ResourceBudget::fixed(64 << 20).unwrap()
}
fn frame<R: Record>(rows: &[R]) -> (&'static str, arrow_array::RecordBatch) {
    (R::NAME, R::encode(rows).unwrap())
}
struct Universe {
    input: InputRevision,
    artifacts: Vec<SourceArtifact>,
    uses: Vec<ArtifactUse>,
    scopes: Vec<CoverageScope>,
    computations: Vec<NormalizationComputation>,
    normalized: Vec<NormalizationCoverage>,
    native: Vec<ProviderCoverage>,
    definition: AnalysisDefinition,
}
impl Universe {
    fn new(profile: Profile, method: AnalysisMethod, count: usize) -> Self {
        let input = InputRevision::from_entries(
            (0..count)
                .map(|n| input::ManifestEntry {
                    path: format!("{n}.py"),
                    content: ContentHash::of(b"x=1\n"),
                    byte_len: 4,
                })
                .collect(),
        )
        .unwrap();
        let artifacts = (0..count)
            .map(|n| SourceArtifact::from_bytes(input.id(), format!("{n}.py"), b"x=1\n").unwrap())
            .collect::<Vec<_>>();
        let uses = artifacts
            .iter()
            .map(|a| ArtifactUse {
                input: input.id(),
                artifact: a.id(),
                role: SourceRole::Release,
            })
            .collect();
        let mut scopes = artifacts
            .iter()
            .map(|a| CoverageScope::Artifact { artifact: a.id() })
            .collect::<Vec<_>>();
        scopes.push(CoverageScope::Input { input: input.id() });
        let capabilities = if method == AnalysisMethod::LocalTransfers {
            vec![
                Capability::FlowLinks,
                Capability::FlowEvents,
                Capability::Bindings,
            ]
        } else {
            vec![
                Capability::PublicExposure,
                Capability::Symbols,
                Capability::Ancestry,
                Capability::Types,
                Capability::Callables,
            ]
        };
        let computations = capabilities
            .into_iter()
            .map(|capability| NormalizationComputation {
                capability,
                policy: ContentHash::of(b"normalized policy"),
                producer: "normalized".into(),
                declaration: ContentHash::of(b"normalized control"),
                profile: profile.name().into(),
                availability: EvidenceAvailability::Complete,
            })
            .collect::<Vec<_>>();
        let mut normalized = Vec::new();
        for (n, artifact) in artifacts.iter().enumerate() {
            for computation in &computations {
                normalized.push(NormalizationCoverage {
                    computation: computation.id(),
                    scope: CoverageScope::Artifact {
                        artifact: artifact.id(),
                    }
                    .id(),
                    context: nominal(2),
                    availability: if method == AnalysisMethod::LocalTransfers
                        && profile == Profile::Catalog
                        && matches!(
                            computation.capability,
                            Capability::FlowLinks | Capability::FlowEvents
                        ) {
                        EvidenceAvailability::NotRequested
                    } else if n == 1 {
                        EvidenceAvailability::Partial
                    } else {
                        EvidenceAvailability::Complete
                    },
                });
            }
        }
        let native = if method == AnalysisMethod::LocalTransfers {
            artifacts
                .iter()
                .map(|a| ProviderCoverage {
                    scope: CoverageScope::Artifact { artifact: a.id() }.id(),
                    provider: (profile == Profile::Behavioral).then(|| nominal(5)),
                    context: nominal(2),
                    family: FactFamily::Flow,
                    run: (profile == Profile::Behavioral).then(|| nominal(6)),
                    status: if profile == Profile::Behavioral {
                        CoverageStatus::CompleteUnderStatedModel
                    } else {
                        CoverageStatus::NotRequested
                    },
                    reason: None,
                    diagnostic: None,
                })
                .collect()
        } else {
            vec![]
        };
        let definition = AnalysisDefinition {
            method,
            semantic_version: ContentHash::of(b"contract control"),
            parameters: nominal(3),
            interpretation: Interpretation::Structural,
        };
        Self {
            input,
            artifacts,
            uses,
            scopes,
            computations,
            normalized,
            native,
            definition,
        }
    }
    fn frames(&self) -> BTreeMap<&'static str, arrow_array::RecordBatch> {
        BTreeMap::from([
            frame(std::slice::from_ref(&self.input)),
            frame(&self.artifacts),
            frame(&self.uses),
            frame(&self.scopes),
            frame(&self.computations),
            frame(&self.normalized),
            frame(&self.native),
            frame(std::slice::from_ref(&self.definition)),
        ])
    }
}
struct CompletedFrames(BTreeMap<&'static str, SourceSnapshot>);
impl CompletedFrames {
    fn read<R: Record>(&self) -> Result<CompletedInput<R>, ModelError> {
        let source = self
            .0
            .get(R::NAME)
            .ok_or_else(|| ModelError::Invalid("missing completed input".into()))?;
        CompletedInput::new(
            source.producer().to_owned(),
            source.model(),
            source.implementation(),
            source.content(),
            source.rows().try_into().unwrap(),
        )
    }
}
fn with_capture_omitting<T>(
    universe: &Universe,
    profile: Profile,
    omit: Option<&str>,
    f: impl FnOnce(
        &CapturedSources,
        &CoverageAdmission<'_>,
        &[SourceSnapshot],
        &ValidatedModel,
        &resources::ResourceBudget,
        &CompletedFrames,
    ) -> T,
) -> T {
    let model = model().unwrap();
    let budget = budget();
    let mut inputs = CompletedFrames(BTreeMap::new());
    macro_rules! completed {
        ($r:ty,$rows:expr) => {
            if Some(<$r>::NAME) != omit {
                let batch = Batch::<$r>::new(&model, $rows, &budget).unwrap();
                let relation = Relation::of::<$r>();
                let mut content = relation.content();
                relation.hash_rows(batch.arrow(), &mut content).unwrap();
                let (rows, content) = content.finish();
                inputs.0.insert(
                    <$r>::NAME,
                    SourceSnapshot::of_relation(
                        &relation,
                        "captured",
                        model.digest(),
                        ContentHash::of(b"fixture implementation"),
                        content,
                        rows,
                    )
                    .unwrap(),
                );
            }
        };
    }
    completed!(InputRevision, vec![universe.input.clone()]);
    completed!(SourceArtifact, universe.artifacts.clone());
    completed!(ArtifactUse, universe.uses.clone());
    completed!(CoverageScope, universe.scopes.clone());
    completed!(NormalizationComputation, universe.computations.clone());
    completed!(NormalizationCoverage, universe.normalized.clone());
    completed!(ProviderCoverage, universe.native.clone());
    completed!(AnalysisDefinition, vec![universe.definition.clone()]);
    let sources = inputs.0.values().cloned().collect::<Vec<_>>();
    let captured = CapturedSources::capture(profile, sources.clone(), &budget).unwrap();
    let mut admission = CoverageAdmission::new(&captured, &budget).unwrap();
    macro_rules! visit {
        ($r:ty,$rows:expr) => {
            if Some(<$r>::NAME) != omit {
                admission
                    .visit_if_expected(
                        &inputs.read::<$r>().unwrap(),
                        &<$r as Record>::encode($rows).unwrap(),
                    )
                    .unwrap();
            }
        };
    }
    visit!(InputRevision, std::slice::from_ref(&universe.input));
    visit!(SourceArtifact, &universe.artifacts);
    visit!(ArtifactUse, &universe.uses);
    visit!(CoverageScope, &universe.scopes);
    visit!(NormalizationComputation, &universe.computations);
    visit!(NormalizationCoverage, &universe.normalized);
    visit!(ProviderCoverage, &universe.native);
    f(&captured, &admission, &sources, &model, &budget, &inputs)
}
fn with_capture<T>(
    universe: &Universe,
    profile: Profile,
    f: impl FnOnce(
        &CapturedSources,
        &CoverageAdmission<'_>,
        &[SourceSnapshot],
        &ValidatedModel,
        &resources::ResourceBudget,
        &CompletedFrames,
    ) -> T,
) -> T {
    with_capture_omitting(universe, profile, None, f)
}
fn publication<R: Record>(
    mut frames: BTreeMap<&'static str, arrow_array::RecordBatch>,
    sources: &[SourceSnapshot],
    profile: Profile,
    model: &ValidatedModel,
    budget: &resources::ResourceBudget,
) -> Result<(), ModelError> {
    let descriptor = lctx_model::domain::validation::publication_checks_for::<R>()
        .into_iter()
        .find(|c| c.name.ends_with("coverage_frontier"))
        .unwrap();
    let mut check = (descriptor.create)(budget);
    for input in descriptor.inputs {
        let batch = frames.remove(input.name()).unwrap_or_else(|| {
            arrow_array::RecordBatch::new_empty(
                model
                    .relations()
                    .iter()
                    .find(|r| r.name() == input.name())
                    .unwrap()
                    .schema()
                    .clone(),
            )
        });
        check.visit(input.name(), &batch)?;
    }
    check.finish(sources, profile)
}
#[test]
fn local_frontier_derives_every_scope_and_refuses_coupled_shrink() {
    let universe = Universe::new(Profile::Behavioral, AnalysisMethod::LocalTransfers, 2);
    with_capture(
        &universe,
        Profile::Behavioral,
        |captured, admission, sources, model, budget, _| {
            use local::coverage::*;
            let (inv, _, _, _) = local::Invocation::admitted(
                universe.input.id(),
                nominal(2),
                universe.definition.id(),
                None,
                [],
                captured,
                [],
                budget,
            )
            .unwrap();
            let admitted = admit(
                &inv,
                &universe.definition,
                AnalysisCapability::Transfers,
                admission,
                budget,
            )
            .unwrap();
            assert_eq!(admitted.scopes().len(), 2);
            let mut requirements = Vec::new();
            let mut required = Vec::new();
            let mut coverage = Vec::new();
            let mut premises = Vec::new();
            let mut lower = Vec::new();
            for scope in admitted.scopes() {
                let (r, m) = scope.expectation().records().unwrap();
                requirements.push(r);
                required.extend(m);
                let (c, p) = assess(
                    scope.expectation(),
                    scope.observations(),
                    AnalysisStatus::Completed,
                    None,
                    budget,
                )
                .unwrap();
                coverage.push(c);
                premises.extend(p);
                lower.extend(scope.observations().iter().map(|r| r.source().clone()));
            }
            assert!(
                coverage
                    .iter()
                    .any(|r| r.availability == EvidenceAvailability::Partial)
            );
            let outcome = local::Outcome {
                invocation: inv.id(),
                status: AnalysisStatus::Completed,
                reason: None,
            };
            let mut frames = universe.frames();
            frames.extend([
                frame(std::slice::from_ref(&inv)),
                frame(&[outcome]),
                frame(&requirements),
                frame(&required),
                frame(&coverage),
                frame(&premises),
                frame(&lower),
            ]);
            publication::<local::Invocation>(
                frames.clone(),
                sources,
                Profile::Behavioral,
                model,
                budget,
            )
            .unwrap();
            let removed = coverage
                .iter()
                .find(|r| r.availability == EvidenceAvailability::Partial)
                .unwrap();
            let scope = removed.scope;
            let removed_id = removed.id();
            let ids = requirements
                .iter()
                .filter(|r| r.scope == scope)
                .map(Record::id)
                .collect::<Vec<_>>();
            let cut_sources = required
                .iter()
                .filter(|r| ids.contains(&r.requirement))
                .map(|r| r.source)
                .collect::<Vec<_>>();
            lower.retain(|r| !cut_sources.contains(&r.id()));
            requirements.retain(|r| r.scope != scope);
            required.retain(|r| !ids.contains(&r.requirement));
            coverage.retain(|r| r.scope != scope);
            premises.retain(|r| r.coverage != removed_id);
            let mut changed = frames;
            changed.extend([
                frame(&requirements),
                frame(&required),
                frame(&coverage),
                frame(&premises),
                frame(&lower),
            ]);
            assert!(
                publication::<local::Invocation>(
                    changed,
                    sources,
                    Profile::Behavioral,
                    model,
                    budget
                )
                .is_err()
            );
            assert!(
                admit(
                    &inv,
                    &universe.definition,
                    AnalysisCapability::ControlInfluence,
                    admission,
                    budget
                )
                .is_err()
            );
        },
    );
}
#[test]
fn catalog_profile_request_and_empty_domain_are_not_observation_choices() {
    let local_universe = Universe::new(Profile::Catalog, AnalysisMethod::LocalTransfers, 1);
    with_capture(
        &local_universe,
        Profile::Catalog,
        |captured, admission, _, _, budget, _| {
            let (inv, _, _, _) = local::Invocation::admitted(
                local_universe.input.id(),
                nominal(2),
                local_universe.definition.id(),
                None,
                [],
                captured,
                [],
                budget,
            )
            .unwrap();
            let admitted = local::coverage::admit(
                &inv,
                &local_universe.definition,
                AnalysisCapability::Transfers,
                admission,
                budget,
            )
            .unwrap();
            assert!(!admitted.scopes()[0].expectation().requested);
            let (row, _) = local::coverage::assess(
                admitted.scopes()[0].expectation(),
                admitted.scopes()[0].observations(),
                AnalysisStatus::NotRequested,
                Some(obligation::ObligationKind::NotRequested),
                budget,
            )
            .unwrap();
            assert_eq!(row.availability, EvidenceAvailability::NotRequested);
        },
    );
    let catalog = Universe::new(Profile::Catalog, AnalysisMethod::Catalog, 0);
    with_capture(
        &catalog,
        Profile::Catalog,
        |captured, admission, sources, model, budget, _| {
            let (inv, _, _, _) = catalog_core::Invocation::admitted(
                catalog.input.id(),
                nominal(2),
                catalog.definition.id(),
                None,
                [],
                captured,
                [],
                budget,
            )
            .unwrap();
            let admitted = catalog_core::coverage::admit(
                &inv,
                &catalog.definition,
                AnalysisCapability::Catalog,
                admission,
                budget,
            )
            .unwrap();
            let scope = &admitted.scopes()[0];
            assert!(scope.expectation().no_scope);
            let (requirement, required) = scope.expectation().records().unwrap();
            let (coverage, premises) = catalog_core::coverage::assess(
                scope.expectation(),
                scope.observations(),
                AnalysisStatus::Completed,
                None,
                budget,
            )
            .unwrap();
            assert_eq!(coverage.availability, EvidenceAvailability::NoScope);
            let mut frames = catalog.frames();
            frames.extend([
                frame(std::slice::from_ref(&inv)),
                frame(&[catalog_core::Outcome {
                    invocation: inv.id(),
                    status: AnalysisStatus::Completed,
                    reason: None,
                }]),
                frame(&[requirement]),
                frame(&required),
                frame(&[coverage]),
                frame(&premises),
            ]);
            publication::<catalog_core::Invocation>(
                frames,
                sources,
                Profile::Catalog,
                model,
                budget,
            )
            .unwrap();
        },
    );
    assert!(CoverageAdmission::new(&CapturedSources::new(&budget()), &budget()).is_err());
}

#[test]
fn catalog_python_domain_keeps_partiality_without_flow_and_missing_lower_refuses() {
    let universe = Universe::new(Profile::Catalog, AnalysisMethod::Catalog, 2);
    with_capture(
        &universe,
        Profile::Catalog,
        |captured, admission, sources, model, budget, _| {
            use catalog_core::coverage::*;
            let (inv, _, _, _) = catalog_core::Invocation::admitted(
                universe.input.id(),
                nominal(2),
                universe.definition.id(),
                None,
                [],
                captured,
                [],
                budget,
            )
            .unwrap();
            let admitted = admit(
                &inv,
                &universe.definition,
                AnalysisCapability::Catalog,
                admission,
                budget,
            )
            .unwrap();
            assert_eq!(admitted.scopes().len(), 2);
            let mut requirements = Vec::new();
            let mut required = Vec::new();
            let mut coverage = Vec::new();
            let mut premises = Vec::new();
            let mut lower = Vec::new();
            for scope in admitted.scopes() {
                assert!(scope.observations().iter().all(|r| matches!(
                    r.source(),
                    catalog_core::CoverageSource::Normalized { .. }
                )));
                let (r, m) = scope.expectation().records().unwrap();
                requirements.push(r);
                required.extend(m);
                let (c, p) = assess(
                    scope.expectation(),
                    scope.observations(),
                    AnalysisStatus::Completed,
                    None,
                    budget,
                )
                .unwrap();
                coverage.push(c);
                premises.extend(p);
                lower.extend(scope.observations().iter().map(|r| r.source().clone()));
            }
            assert!(
                coverage
                    .iter()
                    .any(|r| r.availability == EvidenceAvailability::Partial)
            );
            let mut frames = universe.frames();
            frames.extend([
                frame(std::slice::from_ref(&inv)),
                frame(&[catalog_core::Outcome {
                    invocation: inv.id(),
                    status: AnalysisStatus::Completed,
                    reason: None,
                }]),
                frame(&requirements),
                frame(&required),
                frame(&coverage),
                frame(&premises),
                frame(&lower),
            ]);
            publication::<catalog_core::Invocation>(
                frames,
                sources,
                Profile::Catalog,
                model,
                budget,
            )
            .unwrap();
        },
    );
    let mut missing = Universe::new(Profile::Catalog, AnalysisMethod::Catalog, 1);
    missing.normalized.remove(0);
    with_capture(
        &missing,
        Profile::Catalog,
        |captured, admission, _, _, budget, _| {
            let (inv, _, _, _) = catalog_core::Invocation::admitted(
                missing.input.id(),
                nominal(2),
                missing.definition.id(),
                None,
                [],
                captured,
                [],
                budget,
            )
            .unwrap();
            assert!(
                catalog_core::coverage::admit(
                    &inv,
                    &missing.definition,
                    AnalysisCapability::Catalog,
                    admission,
                    budget
                )
                .is_err()
            );
        },
    );
}

#[test]
fn conditional_routing_skips_unrelated_and_refuses_malformed_or_foreign_before_mutation() {
    use lctx_model::domain::analysis::expected::VisitResult;
    let universe = Universe::new(Profile::Catalog, AnalysisMethod::Catalog, 1);
    with_capture(
        &universe,
        Profile::Catalog,
        |captured, _, _, _, budget, access| {
            let mut route = CoverageAdmission::new(captured, budget).unwrap();
            let definition = access.read::<AnalysisDefinition>().unwrap();
            let unrelated =
                AnalysisDefinition::encode(std::slice::from_ref(&universe.definition)).unwrap();
            assert_eq!(
                route.visit_if_expected(&definition, &unrelated).unwrap(),
                VisitResult::Skipped
            );
            assert!(route.visit(&definition, &unrelated).is_err());
            let permit = access.read::<InputRevision>().unwrap();
            assert!(route.visit_if_expected(&permit, &unrelated).is_err());
            let original = permit.source();
            let wrong = CompletedInput::<InputRevision>::new(
                "foreign",
                original.model(),
                original.implementation(),
                original.content(),
                original.rows().try_into().unwrap(),
            )
            .unwrap();
            let batch = InputRevision::encode(std::slice::from_ref(&universe.input)).unwrap();
            assert!(route.visit_if_expected(&wrong, &batch).is_err());
            // Neither error inserted a row: the correct source remains admissible exactly once.
            let batch = InputRevision::encode(std::slice::from_ref(&universe.input)).unwrap();
            assert_eq!(
                route.visit_if_expected(&permit, &batch).unwrap(),
                VisitResult::Handled
            );
            assert!(route.visit_if_expected(&permit, &batch).is_err());
        },
    );
}
#[test]
fn conditional_routing_does_not_infer_missing_required_capture_from_observed_callbacks() {
    let universe = Universe::new(Profile::Catalog, AnalysisMethod::Catalog, 1);
    with_capture_omitting(
        &universe,
        Profile::Catalog,
        Some(NormalizationCoverage::NAME),
        |captured, admission, _, _, budget, access| {
            assert!(access.read::<NormalizationCoverage>().is_err());
            let (inv, _, _, _) = catalog_core::Invocation::admitted(
                universe.input.id(),
                nominal(2),
                universe.definition.id(),
                None,
                [],
                captured,
                [],
                budget,
            )
            .unwrap();
            assert!(
                catalog_core::coverage::admit(
                    &inv,
                    &universe.definition,
                    AnalysisCapability::Catalog,
                    admission,
                    budget
                )
                .is_err()
            );
        },
    );
}
