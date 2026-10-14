//! Current product admission over a protected exact source closure. This owner
//! establishes no whole-artifact admission and persists no admitted flag.
use super::*;
use lctx_model::domain::{
    Key, KeySink,
    analysis::sources::SourceSnapshot,
    compilation_product::DependencyKind,
    producer_contract::request::{Candidate, CapturedRequest, Eligibility},
};
use lctx_native::{
    control::HeldTerminalProduct,
    surrealdb::{Surreal, engine::remote::grpc::Client},
};

pub(crate) struct CandidateAdmission {
    descriptor: ContentHash,
    request: ContentHash,
    _scope: ContentHash,
    _charge: StateCharge,
}
impl CandidateAdmission {
    pub(crate) fn require(&self, descriptor: &CompletedContribution) -> Result<(), ModelError> {
        if descriptor.identity()? != self.descriptor || descriptor.outcome != 0 {
            return Err(ModelError::Conflict(
                "terminal candidate executed admission differs",
            ));
        }
        let _actual_current_request = self.request;
        Ok(())
    }
}
fn binding(
    model: &ValidatedModel,
    view: &CompletedView,
    boundary: Option<PublicationBoundary>,
) -> Result<CompletedBinding, ModelError> {
    let relation = model
        .relation(&view.relation)
        .ok_or(ModelError::Schema("candidate exact relation"))?;
    Ok(CompletedBinding {
        boundary: boundary.map(|p| p.name().into()),
        source: SourceSnapshot::of_completed_view(relation, model.digest(), view)?,
        view: view.clone(),
        configuration: None,
    })
}
fn boundary(name: Option<&str>) -> Result<Option<PublicationBoundary>, ModelError> {
    name.map(|name| {
        PublicationBoundary::ALL
            .into_iter()
            .find(|p| p.name() == name)
            .ok_or(ModelError::Conflict(
                "candidate unknown captured dependency epoch",
            ))
    })
    .transpose()
}

async fn capture_companions(
    runtime: &AdmissionRuntime,
    owner: &HeldTerminalProduct,
    descriptor: &CompletedContribution,
    charge: &mut StateCharge,
) -> Result<BTreeMap<&'static str, CompanionTable>, ModelError> {
    use lctx_model::domain::{input::*, source::SourceArtifact};
    let Some(binding) = descriptor.spec.captured_binding.as_ref() else {
        return Ok(BTreeMap::new());
    };
    let acquisition =
        descriptor.spec.producer == lctx_model::domain::producer_contract::facts::ACQUIRE;
    let mut needed = BTreeSet::new();
    for held in owner.views().values() {
        let relation = runtime
            .model
            .relation(&held.completion().view.relation)
            .ok_or(ModelError::Schema("candidate selected relation"))?;
        for field in relation.fields() {
            if let Some((_, name)) = field.target() {
                needed.insert(name);
            }
        }
    }
    let declarations = [
        Relation::of::<Package>(),
        Relation::of::<Release>(),
        Relation::of::<InputRevision>(),
        Relation::of::<InputOrigin>(),
        Relation::of::<InputAcquisition>(),
        Relation::of::<CorpusLibrary>(),
        Relation::of::<InputDistribution>(),
        Relation::of::<DistributionVerification>(),
        Relation::of::<EnvironmentFingerprint>(),
        Relation::of::<SourceArtifact>(),
        Relation::of::<ArtifactOwnership>(),
        Relation::of::<UnownedArtifact>(),
        Relation::of::<DerivedArtifact>(),
        Relation::of::<ArtifactUse>(),
    ];
    if acquisition {
        needed.extend(declarations.iter().map(Relation::name));
    }
    loop {
        let mut changed = false;
        for relation in &declarations {
            if needed.contains(relation.name()) {
                for field in relation.fields() {
                    if let Some((_, target)) = field.target() {
                        changed |= needed.insert(target);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    for source in &binding.sources {
        source.acquisition.validate()?;
        if source.input != source.acquisition.revision.id() {
            return Err(ModelError::Conflict("candidate captured source revision"));
        }
    }
    let mut tables = BTreeMap::new();
    macro_rules! add {
        ($ty:ty,$rows:expr) => {
            if needed.contains(<$ty>::NAME) {
                let rows = captured_inventory::<$ty>($rows, charge)?;
                let table = captured_table::<$ty>(
                    owner,
                    descriptor,
                    rows,
                    acquisition,
                    runtime.budget(),
                    charge,
                )
                .await?;
                tables.insert(<$ty>::NAME, table);
            }
        };
    }
    add!(
        InputRevision,
        binding
            .sources
            .iter()
            .map(|s| s.acquisition.revision.clone())
    );
    add!(
        InputOrigin,
        binding.sources.iter().map(|s| s.acquisition.origin.clone())
    );
    add!(
        InputAcquisition,
        binding.sources.iter().map(|s| InputAcquisition {
            input: s.acquisition.revision.id(),
            origin: s.acquisition.origin.id()
        })
    );
    add!(
        CorpusLibrary,
        binding
            .sources
            .iter()
            .filter_map(|s| s.acquisition.library.map(|(library, _)| CorpusLibrary {
                corpus: s.acquisition.revision.id(),
                library
            }))
    );
    macro_rules! field {
        ($ty:ty,$field:ident) => {
            add!(
                $ty,
                binding
                    .sources
                    .iter()
                    .flat_map(|s| s.acquisition.$field.iter().cloned())
            );
        };
    }
    field!(Package, packages);
    field!(Release, releases);
    field!(InputDistribution, distributions);
    field!(DistributionVerification, verifications);
    field!(EnvironmentFingerprint, fingerprints);
    field!(SourceArtifact, artifacts);
    field!(ArtifactOwnership, ownership);
    field!(UnownedArtifact, unowned);
    field!(DerivedArtifact, derivations);
    field!(ArtifactUse, uses);
    Ok(tables)
}
fn captured_inventory<R: Record>(
    rows: impl Iterator<Item = R>,
    charge: &mut StateCharge,
) -> Result<BTreeMap<lctx_model::domain::Id<R>, R>, ModelError> {
    let mut expected = BTreeMap::new();
    for row in rows {
        charge.grow(
            lctx_model::domain::HeapSize::heap_bytes(&row)
                .saturating_add(std::mem::size_of::<R>())
                .saturating_mul(4)
                .saturating_add(256),
        )?;
        row.validate()?;
        if expected
            .insert(row.id(), row.clone())
            .is_some_and(|old| old != row)
        {
            return Err(ModelError::Conflict(
                "candidate conflicting captured nominal row",
            ));
        }
    }
    Ok(expected)
}
async fn captured_table<R: Record>(
    owner: &HeldTerminalProduct,
    descriptor: &CompletedContribution,
    expected: BTreeMap<lctx_model::domain::Id<R>, R>,
    acquisition: bool,
    budget: &ResourceBudget,
    charge: &mut StateCharge,
) -> Result<CompanionTable, ModelError> {
    if acquisition
        && descriptor
            .outputs
            .get(R::NAME)
            .is_none_or(|output| output.rows != expected.len() as u64)
    {
        return Err(ModelError::Conflict(
            "candidate acquisition actual typed inventory count",
        ));
    }
    let mut protected = Vec::new();
    for row in expected.into_values() {
        if owner.verify_captured_row(&row, budget).await? {
            protected.push(row);
        } else if acquisition {
            return Err(ModelError::Conflict(
                "candidate acquisition typed inventory missing backing",
            ));
        }
    }
    let batch = R::encode(&protected)?;
    charge.grow(lctx_model::domain::logical_batch_bytes(&batch)?.saturating_add(1024))?;
    let mut identity = KeySink::new("candidate-captured-reference-table/v1");
    identity.part(b"relation", R::NAME.as_bytes());
    for row in &protected {
        identity.part(b"key", row.id().bytes());
        identity.part(b"content", &row.content_digest().0);
    }
    Ok(CompanionTable {
        relation: Relation::of::<R>(),
        batches: vec![batch],
        identity: identity.finish(),
    })
}

fn admit_acquisition_declaration(
    descriptor: &CompletedContribution,
    profile: Profile,
) -> Result<(), ModelError> {
    let declaration = lctx_model::domain::producer_contract::facts::acquisition();
    declaration.validate()?;
    descriptor.spec.validate_profile(profile)?;
    let binding = descriptor
        .spec
        .captured_binding
        .as_ref()
        .ok_or(ModelError::Conflict(
            "candidate acquisition captured binding absent",
        ))?;
    binding.validate()?;
    declaration.validate_context_dependency(binding, descriptor.spec.profile)?;
    if descriptor.spec.producer != declaration.name
        || !descriptor.spec.inputs.is_empty()
        || binding.contract != declaration.identity()
        || binding.semantic_revision != declaration.semantic_revision
        || descriptor.spec.configuration != Some(binding.producer_settings)
        || descriptor.spec.outputs
            != declaration
                .outputs
                .iter()
                .map(|output| output.name().to_owned())
                .collect()
        || binding
            .suppliers
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            != declaration
                .suppliers
                .iter()
                .map(|supplier| supplier.name)
                .collect()
    {
        return Err(ModelError::Conflict(
            "candidate fixed acquisition declaration differs",
        ));
    }
    // Acquisition supplier/run records belong to Assembly. This check binds the
    // authentic caller's current recipe and capture without inventing those rows.
    Ok(())
}

pub(crate) fn admit<'a>(
    client: Arc<Surreal<Client>>,
    namespace: lctx_model::domain::serving::Name,
    database: lctx_model::domain::serving::Name,
    owner: Arc<HeldTerminalProduct>,
    request: &'a CapturedRequest,
    model: Arc<ValidatedModel>,
    profile: Profile,
    budget: &'a ResourceBudget,
) -> BoxFuture<'a, Result<CandidateAdmission, ModelError>> {
    Box::pin(async move {
        owner.ensure_current(budget).await?;
        let descriptor = owner.descriptor();
        if descriptor.spec != *request.contribution()
            || descriptor.spec.model != model.digest()
            || descriptor.outcome != 0
        {
            return Err(ModelError::Conflict(
                "candidate actual current producer request",
            ));
        }
        descriptor.spec.validate_profile(profile)?;
        let mut charge = StateCharge::new(budget, "terminal candidate admission metadata");
        charge.grow(
            request
                .product()
                .retained_bytes()?
                .saturating_mul(4)
                .saturating_add(owner.views().len().saturating_mul(4096)),
        )?;
        // Recompute the same conservative current typed result contract, rather than
        // accepting a different contract attached to otherwise equal request data.
        let mut contract = KeySink::new("compiler-product-row-contract/v1");
        for name in &descriptor.spec.outputs {
            let relation = model
                .relation(name)
                .ok_or(ModelError::Schema("candidate output model"))?;
            relation.name().to_owned().encode(&mut contract);
            model.digest().encode(&mut contract);
        }
        if contract.finish() != request.product().result_contract {
            return Err(ModelError::Conflict(
                "candidate current typed result contract",
            ));
        }
        let native =
            NativeCompilerStore::terminal_read_owner(client, namespace, database, &owner, budget)?;
        let runtime = AdmissionRuntime::with_budget(
            model.clone(),
            AdmissionOptions {
                memory_bytes: budget.limit(),
                ..AdmissionOptions::default()
            },
            budget.clone(),
            native,
        )?;
        let result = async {
            // A relation appearing at several epochs has no implicit latest/default.
            // Direct captured roles below name their exact selected epoch independently.
            let mut by_name = BTreeMap::<String, Vec<&CompletedView>>::new();
            for held in owner.views().values() {
                by_name
                    .entry(held.completion().view.relation.clone())
                    .or_default()
                    .push(&held.completion().view);
            }
            let mut selected =
                BTreeMap::<(String, Option<PublicationBoundary>), CompletedBinding>::new();
            for (name, views) in by_name {
                if let [view] = views.as_slice() {
                    selected.insert((name, None), binding(&model, view, None)?);
                }
            }
            let dependencies = &request.product().dependencies;
            if dependencies.len() != descriptor.spec.inputs.len() {
                return Err(ModelError::Conflict(
                    "candidate captured dependency inventory",
                ));
            }
            for (role, (snapshot, token)) in
                descriptor.spec.inputs.iter().zip(dependencies).enumerate()
            {
                if token.kind != DependencyKind::ExactView
                    || token.role != format!("input/{role}")
                    || token.relation != snapshot.relation()
                    || token.identity != snapshot.identity()
                {
                    return Err(ModelError::Conflict(
                        "candidate captured ordered dependency role",
                    ));
                }
                let held = owner
                    .views()
                    .get(&snapshot.view())
                    .ok_or(ModelError::Conflict(
                        "candidate protected dependency absent",
                    ))?;
                let prefix = boundary(token.prefix.as_deref())?;
                let exact = binding(&model, &held.completion().view, prefix)?;
                if &exact.source != snapshot {
                    return Err(ModelError::Conflict("candidate actual dependency snapshot"));
                }
                let key = (token.relation.clone(), prefix);
                if selected.get(&key).is_some_and(|old| old != &exact) {
                    return Err(ModelError::Conflict(
                        "candidate conflicting captured epoch selection",
                    ));
                }
                selected.insert(key, exact);
            }
            let logical = descriptor.identity()?;
            for (name, output) in &descriptor.outputs {
                let view =
                    CompletedView::new(name.clone(), BTreeSet::from([logical]), output.rows)?;
                if owner
                    .views()
                    .get(&view.identity)
                    .is_none_or(|held| held.completion().view != view)
                {
                    return Err(ModelError::Conflict("candidate protected singleton output"));
                }
                let output = binding(&model, &view, None)?;
                let key = (name.clone(), None);
                if selected
                    .get(&key)
                    .is_some_and(|previous| previous != &output)
                {
                    return Err(ModelError::Conflict(
                        "candidate output conflicts with captured dependency selection",
                    ));
                }
                selected.insert(key, output);
            }
            let mut selections = Vec::with_capacity(selected.len());
            let mut completed = BTreeMap::new();
            let mut frozen = BTreeMap::new();
            for ((_, prefix), binding) in selected {
                charge.grow(
                    serde_json::to_vec(&binding)
                        .map_err(ModelError::codec)?
                        .len()
                        .saturating_mul(8)
                        .saturating_add(1024),
                )?;
                let source = runtime.exact(binding.clone())?;
                if let Some(prefix) = prefix {
                    frozen.insert((prefix, source.name()), source);
                } else {
                    completed.insert(source.name(), source);
                }
                selections.push(SelectedBinding {
                    role: selections.len(),
                    requested: prefix,
                    resolved: prefix,
                    binding,
                });
            }
            *runtime.completed.lock().map_err(|_| poisoned())? = completed;
            *runtime.frozen_shared.lock().map_err(|_| poisoned())? = frozen;
            {
                let mut owners = runtime.owners.lock().map_err(|_| poisoned())?;
                for (id, contribution) in owner.contributors() {
                    contribution.identity()?;
                    if contribution.spec.model != model.digest() {
                        return Err(ModelError::Conflict("candidate dependency model"));
                    }
                    owners.1.grow(
                        serde_json::to_vec(contribution)
                            .map_err(ModelError::codec)?
                            .len()
                            .saturating_mul(4)
                            .saturating_add(1024),
                    )?;
                    owners.0.insert(*id, contribution.clone());
                }
            }
            // Fixed facts declarations validate the actual selected supplier records
            // and captured bindings without requiring unrelated providers to complete.
            let companions = capture_companions(&runtime, &owner, descriptor, &mut charge).await?;
            if descriptor.spec.producer == lctx_model::domain::producer_contract::facts::ACQUIRE {
                admit_acquisition_declaration(descriptor, profile)?;
            } else if let Some(contract) = lctx_model::domain::producer_contract::facts::facts()
                .iter()
                .find(|c| c.name == descriptor.spec.producer)
            {
                admit_supplier_product(&runtime, descriptor, contract, profile, &mut charge)
                    .await?;
            }
            let mut inputs = runtime.selected(profile, selections)?;
            inputs.companions = companions;
            runtime
                .validate_normalization_declarations(profile, &runtime.completed_relations()?)
                .await?;
            let scope = runtime
                .validate_scope(profile, true, Some(&inputs), None, false)
                .await?
                .identity;
            owner.ensure_current(budget).await?;
            match request.eligibility(Candidate::Completed {
                request: request.identity()?,
                descriptor,
                admitted: true,
            }) {
                Eligibility::Hit => Ok(CandidateAdmission {
                    descriptor: descriptor.identity()?,
                    request: request.identity()?,
                    _scope: scope,
                    _charge: charge,
                }),
                Eligibility::Miss(reason) => Err(ModelError::Invalid(reason.into())),
                Eligibility::Corrupt(error) => Err(error),
            }
        }
        .await;
        let terminal = runtime.drain_report().await;
        let result = lctx_model::domain::completion::complete(result, terminal);
        if let Err(error) = &result {
            if !error.permits_storage_cleanup() {
                owner.retain_unknown();
            }
        }
        result
    })
}

async fn admit_supplier_product(
    runtime: &AdmissionRuntime,
    descriptor: &CompletedContribution,
    contract: &lctx_model::domain::producer_contract::ProducerContract,
    profile: Profile,
    charge: &mut StateCharge,
) -> Result<(), ModelError> {
    use lctx_model::domain::{
        admission::{FrontierContract, reconcile},
        attribution::*,
        input::InputRevision,
        producer_contract::ConfigurationBinding,
        source::{CoverageScope, SourceArtifact},
        stages::ProviderOutcome,
    };
    macro_rules! rows {
        ($ty:ty) => {{
            let source = runtime.relation(<$ty>::NAME)?;
            runtime.coverage_rows_async::<$ty>(&source).await?
        }};
    }
    let (providers, _providers) = rows!(Provider);
    let declaration =
        lctx_model::domain::producer_assurance::AdmittedProducerDeclaration::for_facts(
            descriptor,
            contract,
            &providers,
            runtime.model.digest(),
            profile,
        )?;
    let (runs, _runs) = rows!(ProviderRun);
    let (families, _families) = rows!(RunFamily);
    let (coverage, _coverage) = rows!(ProviderCoverage);
    let (scopes, _scopes) = rows!(CoverageScope);
    let (contexts, _contexts) = rows!(AnalysisContext);
    let sources = declaration.source_bindings()?.collect::<Vec<_>>();
    for source in &sources {
        let captured = source.complete_acquisition().ok_or(ModelError::Conflict(
            "candidate supplier lacks complete actual acquisition",
        ))?;
        captured.validate()?;
        if captured.revision.id() != source.input() {
            return Err(ModelError::Conflict("candidate supplier capture revision"));
        }
        if !contexts.iter().any(|context| {
            Some(context.id()) == source.context()
                && Some(context.config_digest) == source.configuration()
        }) {
            return Err(ModelError::Conflict(
                "candidate supplier actual context/configuration",
            ));
        }
    }
    // Preserve the full ordered capture in the request, while coverage domains
    // count nominal inputs/artifacts/uses once. Contradictory copies still refuse.
    let inputs = captured_inventory::<InputRevision>(
        sources.iter().map(|source| {
            source
                .complete_acquisition()
                .expect("checked actual acquisition")
                .revision
                .clone()
        }),
        charge,
    )?
    .into_values()
    .collect::<Vec<_>>();
    let artifacts = captured_inventory::<SourceArtifact>(
        sources.iter().flat_map(|source| {
            source
                .complete_acquisition()
                .expect("checked actual acquisition")
                .artifacts
                .iter()
                .cloned()
        }),
        charge,
    )?
    .into_values()
    .collect::<Vec<_>>();
    let uses = captured_inventory::<lctx_model::domain::input::ArtifactUse>(
        sources.iter().flat_map(|source| {
            source
                .complete_acquisition()
                .expect("checked actual acquisition")
                .uses
                .iter()
                .cloned()
        }),
        charge,
    )?
    .into_values()
    .collect::<Vec<_>>();
    let frontier = FrontierContract::facts(&runtime.model, profile)?;
    let mut invocations = BTreeSet::new();
    let mut suppliers = BTreeSet::new();
    let mut expected = Vec::new();
    for role in declaration.suppliers() {
        let provider = declaration.supplier(role.name)?;
        if !suppliers.insert(provider) {
            return Err(ModelError::Conflict("candidate ambiguous supplier role"));
        }
        for source in declaration.source_bindings()? {
            let configuration = match role.configuration {
                ConfigurationBinding::CapturedAnalysisContext
                | ConfigurationBinding::CurrentAssemblyContext => {
                    source.configuration().ok_or(ModelError::Conflict(
                        "candidate supplier missing semantic context configuration",
                    ))?
                }
                ConfigurationBinding::ProducerSettings => declaration.producer_settings()?,
                ConfigurationBinding::Fixed(value) => value,
            };
            let mut matched = runs.iter().filter(|run| {
                run.provider == provider
                    && run.input == source.input()
                    && Some(run.context) == source.context()
                    && run.configuration == configuration
            });
            let run = matched.next().ok_or(ModelError::Conflict(
                "candidate missing actual supplier invocation",
            ))?;
            if matched.next().is_some() {
                return Err(ModelError::Conflict(
                    "candidate repeated actual supplier invocation",
                ));
            }
            let requested = families
                .iter()
                .filter(|member| member.run == run.id())
                .map(|member| member.family)
                .collect::<BTreeSet<_>>();
            if Some(run.context) != source.context()
                || run.configuration != configuration
                || requested != role.families.iter().copied().collect()
            {
                return Err(ModelError::Conflict(
                    "candidate supplier changes captured invocation or family inventory",
                ));
            }
            invocations.insert(run.id());
        }
        let domain =
            frontier.supplier_coverage(provider, &role.families, &inputs, &artifacts, &uses)?;
        charge.grow(domain.len().saturating_mul(512).saturating_add(1024))?;
        for (key, scope) in domain {
            let input = match &scope {
                CoverageScope::Input { input } => *input,
                CoverageScope::Artifact { artifact } => {
                    artifacts
                        .iter()
                        .find(|a| a.id() == *artifact)
                        .ok_or(ModelError::Conflict(
                            "candidate supplier artifact scope absent",
                        ))?
                        .input
                }
                _ => {
                    return Err(ModelError::Conflict(
                        "candidate supplier foreign coverage grain",
                    ));
                }
            };
            if !scopes.iter().any(|actual| actual == &scope) {
                return Err(ModelError::Conflict(
                    "candidate actual supplier scope declaration absent",
                ));
            }
            let mut covering = false;
            for run in runs.iter().filter(|run| {
                run.provider == provider && run.input == input && invocations.contains(&run.id())
            }) {
                covering = true;
                expected.push(CoverageExpectation {
                    input,
                    scope: key.scope,
                    provider: Some(provider),
                    context: run.context,
                    family: key.family,
                    run: Some(run.id()),
                });
            }
            if !covering {
                return Err(ModelError::Conflict("candidate covering invocation absent"));
            }
        }
    }
    if runs.len() != invocations.len()
        || families
            .iter()
            .any(|member| !invocations.contains(&member.run))
        || providers.iter().map(Record::id).collect::<BTreeSet<_>>() != suppliers
    {
        return Err(ModelError::Conflict(
            "candidate supplier extra provider/run/family rows",
        ));
    }
    validate_coverage_contract(&expected, &coverage, &runs, &families, runtime.budget())?;
    reconcile(
        ProviderOutcome::Complete,
        &coverage.iter().collect::<Vec<_>>(),
    )
}

#[cfg(test)]
mod controls {
    use super::*;
    use lctx_model::domain::input::{Package, Release};
    fn companion<R: Record>(rows: &[R]) -> CompanionTable {
        let mut sink = KeySink::new("companion-reference-control");
        sink.part(b"relation", R::NAME.as_bytes());
        for row in rows {
            sink.part(b"row", &row.content_digest().0);
        }
        CompanionTable {
            relation: Relation::of::<R>(),
            batches: vec![R::encode(rows).unwrap()],
            identity: sink.finish(),
        }
    }
    #[test]
    fn captured_nominal_domain_deduplicates_equal_rows_and_refuses_contradictions() {
        use lctx_model::domain::{input::InputRevision, source::SourceArtifact};
        let budget = ResourceBudget::fixed(1 << 20).unwrap();
        let mut charge = StateCharge::new(&budget, "capture nominal control");
        let input = InputRevision::from_entries(vec![]).unwrap();
        let row = SourceArtifact::from_bytes(input.id(), "mod.py".into(), b"x").unwrap();
        let equal =
            captured_inventory([row.clone(), row.clone()].into_iter(), &mut charge).unwrap();
        assert_eq!(equal.len(), 1);
        let mut contradictory = row.clone();
        contradictory.byte_len = 2;
        assert!(captured_inventory([row, contradictory].into_iter(), &mut charge).is_err());
    }
    #[tokio::test]
    async fn captured_reference_tables_require_actual_targets_and_never_supply_epochs() {
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        let native = NativeCompilerStore::from_existing(
            Arc::new(Surreal::init()),
            lctx_model::domain::serving::Name::new("pure_companion").unwrap(),
            lctx_model::domain::serving::Name::new("pure_companion").unwrap(),
        );
        let runtime = AdmissionRuntime::with_budget(
            model,
            AdmissionOptions {
                memory_bytes: budget.limit(),
                ..AdmissionOptions::default()
            },
            budget,
            native,
        )
        .unwrap();
        let package = Package {
            name: "exact-package".into(),
        };
        let release = Release {
            package: package.id(),
            version: "1.0".into(),
        };
        let mut inputs = ValidationInputs {
            relations: BTreeMap::new(),
            profile: Profile::Catalog,
            companions: BTreeMap::from([
                (Package::NAME, companion::<Package>(&[])),
                (Release::NAME, companion(std::slice::from_ref(&release))),
            ]),
        };
        let source =
            lctx_model::domain::ValidationInput::of_relation(&Relation::of::<Package>(), &[]);
        assert!(
            inputs.validation_source(&source).is_err(),
            "captured references never become completed snapshots"
        );
        assert!(
            inputs
                .validation_table(&source.clone().at_epoch(PublicationBoundary::Facts))
                .is_err(),
            "a capture cannot substitute for a missing explicit epoch"
        );
        let session = inputs.session(&runtime).await.unwrap();
        assert!(
            runtime
                .validate_references(&session, &[], &inputs, Some(&session))
                .await
                .is_err(),
            "a missing actual reference target must fail"
        );
        inputs
            .companions
            .insert(Package::NAME, companion(std::slice::from_ref(&package)));
        let session = inputs.session(&runtime).await.unwrap();
        runtime
            .validate_references(&session, &[], &inputs, Some(&session))
            .await
            .unwrap();
        assert!(
            runtime.bindings().unwrap().is_empty(),
            "reference companions create no completed bindings"
        );
        lctx_model::domain::completion::complete(Ok(()), runtime.drain_report().await).unwrap();
    }
}
