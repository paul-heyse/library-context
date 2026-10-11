//! Independent facts producers into exact persisted completed contribution views.
use crate::workspace::{ProducerOutput, Workspace, WorkspaceOptions};
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use lctx_model::domain::{
    ContentHash, ModelError, ValidatedModel,
    batching::TransferLimits,
    stages::{Profile, Schedule},
};
use std::sync::Arc;
use tracing::Instrument;

/// Run the production facts providers, accepting their enumeration in any order. Dependency
/// ordering is computed from declarations; runtime reads bind exact persisted completed inputs.
pub async fn compile_facts(
    workspace: &Arc<Workspace>,
    captured: &Arc<CapturedInputs>,
    profile: Profile,
    providers: Vec<Box<dyn ProviderStage<ProducerOutput>>>,
    limits: TransferLimits,
) -> Result<(), ModelError> {
    bundle::refuse_ambient(std::env::vars_os())?;
    captured.config().check_profile(profile)?;
    captured.config().check_budget(workspace.budget())?;
    for input in captured.inputs() {
        input.captured().verify().map_err(ModelError::codec)?;
    }
    let mut offered: Vec<_> = providers
        .into_iter()
        .filter(|p| p.declaration(profile).profiles.contains(&profile))
        .collect();
    let sources = cpg_extract::contracts::captured_sources(captured)?;
    let declarations = offered
        .iter()
        .map(|provider| {
            let mut stage = provider.declaration(profile);
            if let Some(binding) = &mut stage.captured_binding {
                binding.sources = sources.clone();
            }
            stage
        })
        .collect();
    let schedule = Schedule::build(workspace.model(), declarations, &[], profile)?;
    for declaration in schedule.stages() {
        let position = offered
            .iter()
            .position(|p| p.declaration(profile).name == declaration.name)
            .ok_or_else(|| {
                ModelError::Invalid(format!("missing facts provider {}", declaration.name))
            })?;
        let provider = offered.swap_remove(position);
        let inputs = workspace.inputs(
            declaration.name,
            profile,
            declaration.inputs.iter().map(|r| r.name()),
        )?;
        let output = Arc::new(workspace.producer(declaration, profile, inputs));
        let span = tracing::info_span!(target: "lctx_phase", "facts_provider", producer = declaration.name);
        async {
            let execution = lctx_surrealdb::phase::Phase::begin("provider_execution");
            let result = bundle::run_provider(
                provider,
                profile,
                output.clone(),
                workspace.model().clone(),
                captured.clone(),
                workspace.budget().clone(),
                limits,
            )
            .await;
            execution.finish_result(&result);
            result?;
            let completion = lctx_surrealdb::phase::Phase::begin("provider_completion");
            let result = async {
                Arc::try_unwrap(output)
                    .map_err(|_| {
                        ModelError::Invalid("native provider retained output ownership".into())
                    })?
                    .complete()
                    .await
            }
            .await;
            completion.finish_result(&result);
            result
        }
        .instrument(span)
        .await?;
    }
    Ok(())
}

pub fn providers(configuration: ContentHash) -> Vec<Box<dyn ProviderStage<ProducerOutput>>> {
    cpg_extract::contracts::executables(configuration)
}

pub async fn inspect(
    captured: Arc<CapturedInputs>,
    options: WorkspaceOptions,
    profile: Profile,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
) -> Result<(Arc<ValidatedModel>, Arc<Workspace>), ModelError> {
    inspect_with(
        captured,
        options,
        profile,
        providers(ContentHash::of(b"facts-inspection")),
        TransferLimits::default(),
        native,
    )
    .await
}
pub async fn inspect_with(
    captured: Arc<CapturedInputs>,
    options: WorkspaceOptions,
    profile: Profile,
    providers: Vec<Box<dyn ProviderStage<ProducerOutput>>>,
    limits: TransferLimits,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
) -> Result<(Arc<ValidatedModel>, Arc<Workspace>), ModelError> {
    let setup = (|| {
        let model = Arc::new(lctx_model::domain::model()?);
        let workspace = Workspace::with_budget(
            model.clone(),
            options,
            captured.config().budget().clone(),
            native.clone(),
        )?;
        Ok::<_, ModelError>((model, workspace))
    })();
    let (model, workspace) = match setup {
        Ok(value) => value,
        Err(error) => {
            native.fail();
            return lctx_model::domain::completion::complete(
                Err(error),
                native.drain_report().await,
            );
        }
    };
    if let Err(error) = compile_facts(&workspace, &captured, profile, providers, limits).await {
        return lctx_model::domain::completion::complete(
            Err(error),
            workspace.drain_report().await,
        );
    }
    Ok((model, workspace))
}

pub async fn inspect_with_budget(
    captured: Arc<CapturedInputs>,
    options: WorkspaceOptions,
    profile: Profile,
    providers: Vec<Box<dyn ProviderStage<ProducerOutput>>>,
    limits: TransferLimits,
    budget: lctx_model::domain::resources::ResourceBudget,
    native: Arc<lctx_surrealdb::compiler::NativeCompilerStore>,
) -> Result<(Arc<ValidatedModel>, Arc<Workspace>), ModelError> {
    let setup = (|| {
        let model = Arc::new(lctx_model::domain::model()?);
        let workspace = Workspace::with_budget(model.clone(), options, budget, native.clone())?;
        Ok::<_, ModelError>((model, workspace))
    })();
    let (model, workspace) = match setup {
        Ok(value) => value,
        Err(error) => {
            native.fail();
            return lctx_model::domain::completion::complete(
                Err(error),
                native.drain_report().await,
            );
        }
    };
    let result = async {
        compile_facts(&workspace, &captured, profile, providers, limits).await?;
        workspace.validate().await?;
        Ok::<(), ModelError>(())
    }
    .await;
    if let Err(error) = result {
        return lctx_model::domain::completion::complete(
            Err(error),
            workspace.drain_report().await,
        );
    }
    Ok((model, workspace))
}
pub(crate) struct RecordedCoverage {
    pub contract: ContentHash,
    pub expected: std::collections::BTreeSet<lctx_model::domain::admission::Expected>,
    pub reporting: std::collections::BTreeMap<
        (
            lctx_model::domain::attribution::FactFamily,
            Option<lctx_model::domain::Id<lctx_model::domain::attribution::Provider>>,
        ),
        &'static str,
    >,
    pub charge: lctx_model::domain::charged::StateCharge,
}
/// Expand pinned family/scope policy using the exact provider authority of completed facts.
/// Executable build hashes identify new extraction, not admission of an immutable capture.
pub(crate) struct RecordedFacts<'a> {
    pub inputs: &'a [lctx_model::domain::input::InputRevision],
    pub artifacts: &'a [lctx_model::domain::source::SourceArtifact],
    pub uses: &'a [lctx_model::domain::input::ArtifactUse],
    pub recorded: &'a [lctx_model::domain::attribution::Provider],
    pub runs: &'a [lctx_model::domain::attribution::ProviderRun],
    pub families: &'a [lctx_model::domain::attribution::RunFamily],
    pub contexts: &'a [lctx_model::domain::attribution::AnalysisContext],
    pub contributions: &'a [lctx_model::domain::completed::CompletedContribution],
    pub authority: &'a std::collections::BTreeSet<ContentHash>,
    pub bound_sources: &'a std::collections::BTreeMap<
        String,
        lctx_model::domain::analysis::sources::SourceSnapshot,
    >,
    pub observed: &'a [lctx_model::domain::attribution::ProviderCoverage],
}
pub(crate) fn recorded_coverage(
    model: &ValidatedModel,
    profile: Profile,
    facts: &RecordedFacts<'_>,
    budget: &lctx_model::domain::resources::ResourceBudget,
) -> Result<RecordedCoverage, ModelError> {
    let RecordedFacts {
        inputs,
        artifacts,
        uses,
        recorded,
        runs,
        families,
        contexts,
        contributions,
        authority,
        bound_sources,
        observed,
    } = *facts;
    use lctx_model::domain::producer_contract::ConfigurationBinding;
    use lctx_model::domain::{
        Record, admission::FrontierContract, attribution::*, source::CoverageScope,
    };
    use std::collections::{BTreeMap, BTreeSet};
    let contracts = cpg_extract::contracts::facts();
    let refuse = |message: &str| ModelError::Invalid(format!("recorded facts coverage: {message}"));
    let input_ids = inputs.iter().map(Record::id).collect::<BTreeSet<_>>();
    let mut bindings = BTreeMap::new();
    let mut source_bindings = None;
    let mut reporting = BTreeMap::new();
    let mut suppliers = BTreeMap::new();
    let mut charge =
        lctx_model::domain::charged::StateCharge::new(budget, "facts-coverage-reporting");
    for contract in &contracts {
        if !contract.profiles.contains(&profile) {
            continue;
        }
        let mut owners = contributions.iter().filter(|row| {
            row.spec.producer == contract.name
                && row
                    .identity()
                    .is_ok_and(|identity| authority.contains(&identity))
        });
        let owner = owners
            .next()
            .ok_or_else(|| refuse("missing completed declared producer"))?;
        if owners.next().is_some() {
            return Err(refuse("ambiguous completed declared producer"));
        }
        let binding = owner
            .spec
            .captured_binding
            .as_ref()
            .ok_or_else(|| refuse("facts producer has no captured semantic binding"))?;
        contract.validate_binding(binding, owner.spec.configuration, recorded)?;
        let inventory = contract
            .outputs
            .iter()
            .chain(&contract.contributes)
            .map(|relation| relation.name().to_owned())
            .collect::<BTreeSet<_>>();
        if owner.spec.model != model.digest()
            || owner.spec.profile != profile
            || owner.spec.outputs != inventory
        {
            return Err(refuse(
                "producer differs from declared model/profile/inventory",
            ));
        }
        let declared_inputs = contract
            .inputs
            .iter()
            .map(|input| input.name())
            .collect::<BTreeSet<_>>();
        let actual_inputs = owner
            .spec
            .inputs
            .iter()
            .map(|input| input.relation())
            .collect::<BTreeSet<_>>();
        if actual_inputs != declared_inputs
            || actual_inputs.len() != owner.spec.inputs.len()
            || owner.spec.inputs.iter().any(|input| {
                input.model() != model.digest()
                    || bound_sources
                        .get(input.relation())
                        .is_some_and(|bound| bound != input)
            })
        {
            return Err(refuse(
                "producer changes its exact declared captured inputs",
            ));
        }
        if binding
            .sources
            .iter()
            .map(|source| source.input)
            .collect::<BTreeSet<_>>()
            != input_ids
        {
            return Err(refuse("producer changes its captured source universe"));
        }
        for source in &binding.sources {
            let mut matched = contexts
                .iter()
                .filter(|context| context.id() == source.context);
            let context = matched
                .next()
                .ok_or_else(|| refuse("captured source context is absent"))?;
            if matched.next().is_some() || context.config_digest != source.configuration {
                return Err(refuse("captured source context configuration differs"));
            }
        }
        if source_bindings
            .as_ref()
            .is_some_and(|sources| *sources != &binding.sources)
        {
            return Err(refuse("producer captured source/context bindings disagree"));
        }
        source_bindings = Some(&binding.sources);
        bindings.insert(contract.name, binding);
        for role in &contract.suppliers {
            let provider = binding.suppliers[role.name];
            if suppliers
                .insert(provider, (contract, role, binding))
                .is_some()
            {
                return Err(refuse("ambiguous supplier role authority"));
            }
            for family in &role.families {
                charge.grow(
                    std::mem::size_of::<(
                        (FactFamily, Option<lctx_model::domain::Id<Provider>>),
                        &'static str,
                    )>() + 32,
                )?;
                if reporting
                    .insert((*family, Some(provider)), contract.name)
                    .is_some()
                {
                    return Err(refuse("ambiguous family reporting owner"));
                }
            }
        }
    }
    // The selected binding inventories, never coverage rows or current builds, define suppliers.
    if recorded.iter().map(Record::id).collect::<BTreeSet<_>>()
        != suppliers.keys().copied().collect()
        || recorded.len() != suppliers.len()
    {
        return Err(refuse("missing or extra selected supplier records"));
    }
    for contract in &contracts {
        for family in &contract.not_requested {
            if FrontierContract::facts(model, profile)?.requested(*family) {
                continue;
            }
            charge.grow(
                std::mem::size_of::<(
                    (FactFamily, Option<lctx_model::domain::Id<Provider>>),
                    &'static str,
                )>() + 32,
            )?;
            if reporting.insert((*family, None), contract.name).is_some() {
                return Err(refuse("ambiguous unrequested reporting owner"));
            }
        }
    }
    let preflight =
        FrontierContract::facts(model, profile)?.captured_preflight(&contracts, &bindings)?;
    let expected = preflight.expected_coverage(inputs, artifacts, uses)?;
    // Validate all invocations, including a declared family with no selected artifact scope.
    let mut invocation_keys = BTreeSet::new();
    for run in runs {
        let (_, role, binding) = suppliers
            .get(&run.provider)
            .ok_or_else(|| refuse("invocation has no declared captured supplier"))?;
        let source = binding
            .sources
            .iter()
            .find(|source| source.input == run.input)
            .ok_or_else(|| refuse("invocation source is outside capture"))?;
        let configuration = match role.configuration {
            ConfigurationBinding::CapturedAnalysisContext => source.configuration,
            ConfigurationBinding::ProducerSettings => binding.producer_settings,
            ConfigurationBinding::Fixed(value) => value,
        };
        let requested = families
            .iter()
            .filter(|member| member.run == run.id())
            .map(|member| member.family)
            .collect::<BTreeSet<_>>();
        if run.context != source.context
            || run.configuration != configuration
            || requested != role.families.iter().copied().collect()
            || !invocation_keys.insert((run.provider, run.input))
        {
            return Err(refuse(
                "invocation changes declared family/source/configuration binding",
            ));
        }
    }
    for (provider, (_, _, binding)) in &suppliers {
        for source in &binding.sources {
            if !invocation_keys.contains(&(*provider, source.input)) {
                return Err(refuse("missing declared supplier invocation"));
            }
        }
    }
    let mut obligations = Vec::new();
    for (key, scope) in &expected {
        let input = match scope {
            CoverageScope::Input { input } => *input,
            CoverageScope::Artifact { artifact } => {
                artifacts
                    .iter()
                    .find(|row| row.id() == *artifact)
                    .ok_or_else(|| refuse("coverage artifact is absent"))?
                    .input
            }
            _ => return Err(refuse("foreign facts coverage scope grain")),
        };
        let source = source_bindings
            .ok_or_else(|| refuse("missing captured source bindings"))?
            .iter()
            .find(|source| source.input == input)
            .ok_or_else(|| refuse("coverage source outside capture"))?;
        let run = key
            .provider
            .map(|provider| {
                runs.iter()
                    .find(|run| run.provider == provider && run.input == input)
                    .map(Record::id)
                    .ok_or_else(|| refuse("missing covering invocation"))
            })
            .transpose()?;
        obligations.push(CoverageExpectation {
            input,
            scope: key.scope,
            provider: key.provider,
            context: source.context,
            family: key.family,
            run,
        });
    }
    validate_coverage_contract(&obligations, observed, runs, families, budget)?;
    Ok(RecordedCoverage {
        contract: preflight.contract().digest(),
        expected: expected.into_keys().collect(),
        reporting,
        charge,
    })
}

#[cfg(test)]
mod recorded_coverage_controls {
    use super::*;
    use lctx_model::domain::{
        Record, attribution::*, completed::*, input::InputRevision,
        producer_contract::CapturedSourceBinding, resources::ResourceBudget, source::CoverageScope,
    };
    use std::collections::{BTreeMap, BTreeSet};
    #[test]
    fn restored_bindings_preserve_build_independence_and_refuse_contradictory_authority() {
        let model = lctx_model::domain::model().unwrap();
        let profile = Profile::Catalog;
        let budget = ResourceBudget::fixed(16 << 20).unwrap();
        let input = InputRevision::from_entries(vec![]).unwrap();
        let context = AnalysisContext {
            python_version: "3.14.7".into(),
            python_platform: "linux".into(),
            search_path: vec![],
            site_package_path: vec![],
            config_digest: ContentHash::of(b"captured config"),
            environment_digest: ContentHash::of(b"env"),
            lock_digest: None,
        };
        let scope = CoverageScope::Input { input: input.id() };
        let contracts = cpg_extract::contracts::facts();
        let mut stages = providers(ContentHash::of(b"captured settings"))
            .into_iter()
            .map(|provider| provider.declaration(profile))
            .filter(|stage| stage.profiles.contains(&profile))
            .collect::<Vec<_>>();
        let mut recorded = vec![
            cpg_extract::acquisition::acquisition_provider(),
            cpg_extract::pyrefly_stage::pyrefly_provider(),
            cpg_extract::ruff_context::provider(),
            cpg_extract::document_parser::provider(),
            cpg_extract::deployment::provider(),
        ];
        let mut replacements = BTreeMap::new();
        for provider in &mut recorded {
            let old = provider.id();
            provider.build_digest = ContentHash::of(
                format!(
                    "captured {} build before unrelated current rebuild",
                    provider.tool
                )
                .as_bytes(),
            );
            replacements.insert(old, provider.id());
        }
        let mut contributions = Vec::new();
        let mut runs = Vec::new();
        let mut families = Vec::new();
        let mut observed = Vec::new();
        for stage in &mut stages {
            let contract = contracts
                .iter()
                .find(|contract| contract.name == stage.name)
                .unwrap();
            let binding = stage.captured_binding.as_mut().unwrap();
            binding.sources = vec![CapturedSourceBinding {
                input: input.id(),
                context: context.id(),
                configuration: context.config_digest,
            }];
            for provider in binding.suppliers.values_mut() {
                *provider = replacements[provider];
            }
            for role in &contract.suppliers {
                let provider = binding.suppliers[role.name];
                let (run, members) = ProviderRun::new(
                    provider,
                    context.id(),
                    input.id(),
                    context.config_digest,
                    role.families.clone(),
                )
                .unwrap();
                // These three obligations are input-grained even when the capture has no
                // artifacts. Keep this fixture inventory explicit and independent of admission.
                for family in &role.families {
                    if matches!(
                        family,
                        FactFamily::Artifacts | FactFamily::Signatures | FactFamily::Deployment
                    ) {
                        observed.push(ProviderCoverage {
                            scope: scope.id(),
                            provider: Some(provider),
                            context: context.id(),
                            family: *family,
                            run: Some(run.id()),
                            status: CoverageStatus::CompleteUnderStatedModel,
                            reason: None,
                            diagnostic: None,
                        });
                    }
                }
                runs.push(run);
                families.extend(members);
            }
            let outputs = stage
                .outputs
                .iter()
                .chain(&stage.contributes)
                .map(|row| row.name().to_owned())
                .collect::<BTreeSet<_>>();
            contributions.push(CompletedContribution {
                spec: ContributionSpec {
                    captured_binding: stage.captured_binding.clone(),
                    producer: stage.name.into(),
                    profile,
                    model: model.digest(),
                    implementation: ContentHash::of(
                        format!("captured {} implementation", stage.name).as_bytes(),
                    ),
                    configuration: Some(stage.configuration),
                    inputs: stage
                        .inputs
                        .iter()
                        .map(|input| {
                            lctx_model::domain::analysis::sources::SourceSnapshot::of_relation(
                                model.relation(input.name()).unwrap(),
                                "captured dependency",
                                model.digest(),
                                ContentHash::of(b"recorded dependency"),
                                ContentHash::of(input.name().as_bytes()),
                                0,
                            )
                            .unwrap()
                        })
                        .collect(),
                    outputs: outputs.clone(),
                },
                outcome: 0,
                outputs: outputs
                    .iter()
                    .map(|name| {
                        (
                            name.clone(),
                            OutputContent {
                                rows: 0,
                                content: ContentHash::of(name.as_bytes()),
                            },
                        )
                    })
                    .collect(),
            });
        }
        let authority = contributions
            .iter()
            .map(|row| row.identity().unwrap())
            .collect();
        let check = |providers: &[Provider],
                     owners: &[CompletedContribution],
                     authority: &BTreeSet<ContentHash>,
                     runs: &[ProviderRun],
                     families: &[RunFamily],
                     observed: &[ProviderCoverage]| {
            recorded_coverage(
                &model,
                profile,
                &RecordedFacts {
                    inputs: std::slice::from_ref(&input),
                    artifacts: &[],
                    uses: &[],
                    recorded: providers,
                    runs,
                    families,
                    contexts: std::slice::from_ref(&context),
                    contributions: owners,
                    authority,
                    bound_sources: &BTreeMap::new(),
                    observed,
                },
                &budget,
            )
        };
        let admitted = check(
            &recorded,
            &contributions,
            &authority,
            &runs,
            &families,
            &observed,
        )
        .unwrap();
        let bound = contributions
            .iter()
            .map(|owner| {
                (
                    owner.spec.producer.as_str(),
                    owner.spec.captured_binding.as_ref().unwrap(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let frontier =
            lctx_model::domain::admission::FrontierContract::facts(&model, profile).unwrap();
        let mut inactive_change = contracts.clone();
        let inactive = inactive_change
            .iter_mut()
            .find(|contract| contract.name == cpg_extract::ty_flow::TY_FLOW)
            .unwrap();
        inactive.suppliers[0].analyzer_revision = "future inactive analyzer";
        inactive.suppliers[0].semantic_revision += 1;
        assert_eq!(
            frontier
                .captured_preflight(&inactive_change, &bound)
                .unwrap()
                .contract()
                .digest(),
            admitted.contract,
            "inactive analyzer semantics cannot invalidate Catalog capture"
        );
        inactive_change
            .iter_mut()
            .find(|contract| contract.name == cpg_extract::ty_flow::TY_FLOW)
            .unwrap()
            .name = "changed-flow-reporting-owner";
        assert_ne!(
            frontier
                .captured_preflight(&inactive_change, &bound)
                .unwrap()
                .contract()
                .digest(),
            admitted.contract,
            "NotRequested reporting ownership remains bound"
        );
        let acquire = recorded
            .iter()
            .find(|provider| provider.tool == "lctx-acquire")
            .unwrap();
        assert_eq!(
            admitted
                .reporting
                .get(&(FactFamily::Artifacts, Some(acquire.id()))),
            Some(&cpg_extract::acquisition::ACQUIRE)
        );
        assert_eq!(
            admitted.reporting.get(&(FactFamily::Flow, None)),
            Some(&cpg_extract::ty_flow::TY_FLOW)
        );
        lctx_model::domain::admission::ScopedAvailability::from_completed(
            profile,
            &admitted.expected,
            &observed,
            &BTreeMap::from([(scope.id(), scope.clone())]),
            &budget,
        )
        .unwrap();
        assert!(
            check(
                &recorded[1..],
                &contributions,
                &authority,
                &runs,
                &families,
                &observed
            )
            .is_err()
        );
        let mut extra = recorded.clone();
        let mut foreign = acquire.clone();
        foreign.build_digest = ContentHash::of(b"extra selected supplier");
        extra.push(foreign);
        assert!(
            check(
                &extra,
                &contributions,
                &authority,
                &runs,
                &families,
                &observed
            )
            .is_err()
        );
        let mut unselected = contributions.clone();
        let mut other = contributions[0].clone();
        other.spec.implementation = ContentHash::of(b"unselected owner");
        unselected.push(other);
        assert!(
            check(
                &recorded,
                &unselected,
                &authority,
                &runs,
                &families,
                &observed
            )
            .is_ok()
        );
        let mut incompatible = contributions.clone();
        incompatible[0]
            .spec
            .captured_binding
            .as_mut()
            .unwrap()
            .semantic_revision += 1;
        let changed = incompatible
            .iter()
            .map(|row| row.identity().unwrap())
            .collect();
        assert!(
            check(
                &recorded,
                &incompatible,
                &changed,
                &runs,
                &families,
                &observed
            )
            .is_err()
        );
        let mut missing = contributions.clone();
        missing[0].spec.captured_binding = None;
        let changed = missing.iter().map(|row| row.identity().unwrap()).collect();
        assert!(check(&recorded, &missing, &changed, &runs, &families, &observed).is_err());
        let mut wrong_sources = contributions.clone();
        wrong_sources[0]
            .spec
            .captured_binding
            .as_mut()
            .unwrap()
            .sources
            .clear();
        let changed = wrong_sources
            .iter()
            .map(|row| row.identity().unwrap())
            .collect();
        assert!(
            check(
                &recorded,
                &wrong_sources,
                &changed,
                &runs,
                &families,
                &observed
            )
            .is_err()
        );
        let mut inventory = contributions.clone();
        inventory[0].spec.inputs.clear();
        inventory[0].spec.outputs.clear();
        inventory[0].outputs.clear();
        let changed = inventory
            .iter()
            .map(|row| row.identity().unwrap())
            .collect();
        assert!(check(&recorded, &inventory, &changed, &runs, &families, &observed).is_err());
        let mut ambiguity = contributions.clone();
        let mut other = contributions[0].clone();
        other.spec.implementation = ContentHash::of(b"second selected owner");
        ambiguity.push(other);
        let changed = ambiguity
            .iter()
            .map(|row| row.identity().unwrap())
            .collect();
        assert!(check(&recorded, &ambiguity, &changed, &runs, &families, &observed).is_err());
        let mut altered_runs = runs.clone();
        let old = altered_runs[0].id();
        altered_runs[0].configuration = ContentHash::of(b"foreign config");
        let new = altered_runs[0].id();
        let mut altered_members = families.clone();
        for member in &mut altered_members {
            if member.run == old {
                member.run = new;
            }
        }
        let mut altered_outcomes = observed.clone();
        for row in &mut altered_outcomes {
            if row.run == Some(old) {
                row.run = Some(new);
            }
        }
        assert!(
            check(
                &recorded,
                &contributions,
                &authority,
                &altered_runs,
                &altered_members,
                &altered_outcomes
            )
            .is_err()
        );
        assert!(
            check(
                &recorded,
                &contributions,
                &authority,
                &runs,
                &families,
                &observed[1..]
            )
            .is_err()
        );
        let mut duplicate = observed.clone();
        duplicate.push(observed[0].clone());
        assert!(
            check(
                &recorded,
                &contributions,
                &authority,
                &runs,
                &families,
                &duplicate
            )
            .is_err()
        );
        let mut wrong_revision = recorded.clone();
        wrong_revision[0].revision = "incompatible analyzer".into();
        assert!(
            check(
                &wrong_revision,
                &contributions,
                &authority,
                &runs,
                &families,
                &observed
            )
            .is_err()
        );
    }
}
