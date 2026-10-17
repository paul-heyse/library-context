//! Independent facts producers into exact persisted completed contribution views.
use crate::workspace::{ProducerOutput, Workspace, WorkspaceOptions};
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use lctx_model::domain::{
    ContentHash, ModelError, ValidatedModel,
    batching::TransferLimits,
    stages::{Profile, Schedule},
};
use lctx_tracing::InContext;
use std::{collections::BTreeSet, sync::Arc};

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
    let captured_run = bundle::CapturedTreeRunGuard::open(captured.clone())?;
    let mut offered: Vec<_> = providers
        .into_iter()
        .filter(|p| p.declaration(profile).profiles.contains(&profile))
        .collect();
    let sources = cpg_extract::contracts::captured_sources(captured)?;
    let declarations = offered
        .iter()
        .map(|provider| {
            cpg_extract::contracts::bind_stage_with_sources(
                provider.declaration(profile),
                captured,
                &sources,
            )
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    let schedule = Schedule::build(workspace.model(), declarations, &[], profile)?;
    let mut completed = BTreeSet::new();
    let mut started = BTreeSet::new();
    let mut pending = FuturesUnordered::new();
    let mut failure = None;
    let mut terminal = lctx_model::domain::completion::Completion::default();
    loop {
        if failure.is_none() {
            for declaration in schedule.stages() {
                if started.contains(declaration.name)
                    || !schedule.ready(declaration.name, &completed)?
                {
                    continue;
                }
                let position = offered
                    .iter()
                    .position(|p| p.declaration(profile).name == declaration.name)
                    .ok_or_else(|| {
                        ModelError::Invalid(format!("missing facts provider {}", declaration.name))
                    })?;
                let provider = offered.swap_remove(position);
                started.insert(declaration.name);
                let declaration = declaration.clone();
                let workspace = workspace.clone();
                let captured_run = captured_run.clone();
                pending.push(provider_task(
                    workspace,
                    provider,
                    declaration,
                    profile,
                    captured_run,
                    limits,
                ));
            }
        }
        let Some((name, result)) = pending.next().await else {
            break;
        };
        match result {
            Ok(()) => {
                completed.insert(name);
            }
            Err(error) if failure.is_none() => {
                workspace.cancellation().cancel();
                failure = Some(error);
            }
            Err(error) => terminal.step("sibling facts producer completion", Err(error)),
        }
    }
    if let Some(error) = failure {
        return lctx_model::domain::completion::complete(Err(error), terminal);
    }
    if completed.len() != schedule.stages().len() {
        return Err(ModelError::Conflict("facts readiness made no progress"));
    }

    Ok(())
}

/// Erase the complete owned task at the facts boundary, rather than exposing its
/// provider/output completion coroutine's Send proof to every downstream consumer.
/// This is the scheduler's existing allocation; setup and effects remain lazy.
fn provider_task(
    workspace: Arc<Workspace>,
    provider: Box<dyn ProviderStage<ProducerOutput>>,
    declaration: lctx_model::domain::stages::Stage,
    profile: Profile,
    captured_run: bundle::CapturedTreeRunGuard,
    limits: TransferLimits,
) -> BoxFuture<'static, (&'static str, Result<(), ModelError>)> {
    async move {
        let name = declaration.name;
        let result = run_provider(
            &workspace,
            provider,
            declaration,
            profile,
            captured_run,
            limits,
        )
        .await;
        (name, result)
    }
    .boxed()
}

async fn run_provider(
    workspace: &Arc<Workspace>,
    provider: Box<dyn ProviderStage<ProducerOutput>>,
    declaration: lctx_model::domain::stages::Stage,
    profile: Profile,
    captured_run: bundle::CapturedTreeRunGuard,
    limits: TransferLimits,
) -> Result<(), ModelError> {
    let inputs = workspace.stage_inputs(&declaration, profile)?;
    let output = Arc::new(workspace.producer(&declaration, profile, inputs));
    output.captured_request()?;
    let span = tracing::info_span!(target: "lctx_phase", "facts_provider", producer = declaration.name, attempt=%workspace.native().attempt().hex());
    async move {
        let execution = lctx_native::phase::Phase::begin("provider_execution");
        let result = bundle::run_provider_bound_guarded(
            provider,
            declaration.clone(),
            profile,
            output.clone(),
            workspace.model().clone(),
            captured_run.clone(),
            workspace.budget().clone(),
            limits,
        )
        .await;
        execution.finish_result(&result);
        if let Err(error) = result {
            let mut completion = lctx_model::domain::completion::Completion::default();
            completion.step(
                "failed provider accepted ingress drain",
                output.finish_ingress().await,
            );
            return lctx_model::domain::completion::complete(Err(error), completion);
        }
        let completion = lctx_native::phase::Phase::begin("provider_completion");
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
    .with_tracing_span(span)
    .await
}

pub fn providers(configuration: ContentHash) -> Vec<Box<dyn ProviderStage<ProducerOutput>>> {
    cpg_extract::contracts::executables(configuration)
}

pub async fn inspect(
    captured: Arc<CapturedInputs>,
    options: WorkspaceOptions,
    profile: Profile,
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
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
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
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
    native: Arc<lctx_compiler_store::compiler::NativeCompilerStore>,
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
#[cfg(test)]
use lctx_compiler_store::facts::{RecordedFacts, recorded_coverage};

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
        let contracts = lctx_model::domain::producer_contract::facts::facts();
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
                context: Some(context.id()),
                configuration: Some(context.config_digest),
                acquisition:
                    lctx_model::domain::producer_contract::acquisition::CapturedAcquisition::tree(
                        input.clone(),
                        "captured fixture".into(),
                    )
                    .into(),
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
            contributions.push(CompletedContribution { assurance: lctx_model::domain::producer_assurance::ProducerAssurance::CurrentProviderInterface,
                spec: ContributionSpec {
                    captured_binding: stage.captured_binding.clone(),
                    producer: stage.name.into(),
                    profile:lctx_model::domain::completed::OutputProfile::Specific(profile),
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
        let captured_contract = frontier
            .captured_preflight(&contracts, &bound)
            .unwrap()
            .contract()
            .digest();
        assert_ne!(
            captured_contract, admitted.contract,
            "captured contracts and admitted declarations retain distinct authority domains"
        );
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
            captured_contract,
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
            captured_contract,
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

#[cfg(test)]
mod ready_controls {
    use super::*;
    use cpg_extract::bundle::{Declared, StageContext};
    use lctx_model::domain::{
        input::{Package, Release},
        stages::*,
        *,
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    use std::time::Duration;

    enum Work {
        Wait(mpsc::Receiver<()>),
        Signal(mpsc::Sender<()>),
        FailAfterIngress(mpsc::Receiver<()>),
        IngressUntilCancelled(mpsc::Sender<()>, Arc<AtomicBool>),
    }
    struct Provider {
        name: &'static str,
        package: bool,
        work: Work,
    }
    impl Declared for Provider {
        fn declaration(&self, _: Profile) -> Stage {
            Stage {
                name: self.name,
                inputs: vec![],
                outputs: if self.package {
                    vec![RelationUse::of::<Package>()]
                } else {
                    vec![RelationUse::of::<Release>()]
                },
                contributes: vec![],
                coverage: vec![],
                profiles: Profile::ALL.to_vec(),
                effect: Effect::Extraction,
                code: ContentHash::of(b"controlled-native-producer"),
                configuration: ContentHash::of(b"controlled-native-producer"),
                captured_binding: None,
            }
        }
    }
    impl ProviderStage<ProducerOutput> for Provider {
        fn run(
            &mut self,
            context: &mut StageContext<ProducerOutput>,
        ) -> Result<ProviderOutcome, ModelError> {
            if self.package {
                context.declare::<Package>()?;
            } else {
                context.declare::<Release>()?;
            }
            let wait = |rx: &mpsc::Receiver<()>| {
                rx.recv_timeout(Duration::from_secs(10))
                    .map_err(|error| ModelError::Cause(Box::new(error)))
            };
            match &self.work {
                Work::Wait(rx) => wait(rx)?,
                Work::Signal(tx) => tx.send(()).map_err(ModelError::codec)?,
                Work::FailAfterIngress(rx) => {
                    wait(rx)?;
                    return Err(ModelError::Invalid(
                        "deliberate failure after accepted ingress".into(),
                    ));
                }
                Work::IngressUntilCancelled(tx, joined) => {
                    let (cancel, cancelled) = mpsc::channel();
                    let _registration = context.register_cancellation(move || {
                        let _ = cancel.send(());
                    });
                    let batch = Batch::new(
                        context.model(),
                        vec![Package {
                            name: "accepted".into(),
                        }],
                        context.budget(),
                    )?;
                    context.emit_batch(batch)?;
                    tx.send(()).map_err(ModelError::codec)?;
                    wait(&cancelled)?;
                    joined.store(true, Ordering::Release);
                    context.check_cancelled()?;
                }
            }
            Ok(ProviderOutcome::Complete)
        }
    }
    fn setup() -> (Arc<Workspace>, Arc<CapturedInputs>) {
        let model = Arc::new(
            ValidatedModel::declared(vec![Relation::of::<Package>(), Relation::of::<Release>()])
                .unwrap(),
        );
        let workspace =
            Workspace::new(model, Default::default(), crate::test_native::store()).unwrap();
        let config = cpg_extract::native_context::NativeContextConfig::committed(
            Profile::Catalog,
            workspace.budget(),
        )
        .unwrap();
        (workspace, Arc::new(CapturedInputs::new(vec![], config)))
    }
    #[tokio::test]
    async fn ready_providers_enter_while_an_independent_peer_is_blocked() {
        let (workspace, captured) = setup();
        let (tx, rx) = mpsc::channel();
        compile_facts(
            &workspace,
            &captured,
            Profile::Catalog,
            vec![
                Box::new(Provider {
                    name: "a_waiting",
                    package: true,
                    work: Work::Wait(rx),
                }),
                Box::new(Provider {
                    name: "b_ready",
                    package: false,
                    work: Work::Signal(tx),
                }),
            ],
            Default::default(),
        )
        .await
        .unwrap();
        assert_eq!(workspace.completed::<Package>().unwrap().rows(), 0);
        assert_eq!(workspace.completed::<Release>().unwrap().rows(), 0);
    }
    #[tokio::test]
    async fn sibling_failure_cancels_and_joins_after_native_ingress_was_accepted() {
        let (workspace, captured) = setup();
        let (tx, rx) = mpsc::channel();
        let joined = Arc::new(AtomicBool::new(false));
        let error = compile_facts(
            &workspace,
            &captured,
            Profile::Catalog,
            vec![
                Box::new(Provider {
                    name: "a_ingress",
                    package: true,
                    work: Work::IngressUntilCancelled(tx, joined.clone()),
                }),
                Box::new(Provider {
                    name: "b_failure",
                    package: false,
                    work: Work::FailAfterIngress(rx),
                }),
            ],
            Default::default(),
        )
        .await
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("deliberate failure after accepted ingress"),
            "{error}"
        );
        assert!(
            joined.load(Ordering::Acquire),
            "coordinator must join the cancelled sibling before returning"
        );
        assert!(workspace.completed::<Package>().is_err());
        assert!(workspace.completed::<Release>().is_err());
        assert_eq!(
            workspace.drain_report().await.local,
            completion::LocalState::Terminal
        );
    }
}
