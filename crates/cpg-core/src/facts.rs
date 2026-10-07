//! Independent facts producers into exact persisted completed contribution views.
use crate::workspace::{ProducerOutput, Workspace, WorkspaceOptions};
use cpg_extract::bundle::{self, CapturedInputs, ProviderStage};
use lctx_model::domain::{
    ContentHash, ModelError, ValidatedModel,
    batching::TransferLimits,
    stages::{Profile, Schedule},
};
use std::sync::Arc;

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
    let schedule = Schedule::build(
        workspace.model(),
        offered.iter().map(|p| p.declaration(profile)).collect(),
        &[],
        profile,
    )?;
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
        bundle::run_provider(
            provider,
            profile,
            output.clone(),
            workspace.model().clone(),
            captured.clone(),
            workspace.budget().clone(),
            limits,
        )
        .await?;
        Arc::try_unwrap(output)
            .map_err(|_| ModelError::Invalid("native provider retained output ownership".into()))?
            .complete()
            .await?;
    }
    Ok(())
}

pub fn providers(configuration: ContentHash) -> Vec<Box<dyn ProviderStage<ProducerOutput>>> {
    vec![
        Box::new(cpg_extract::acquisition::Acquire::new(configuration)),
        Box::new(cpg_extract::pyrefly_stage::Pyrefly::new(
            cpg_extract::typed_syntax::SyntaxLimits::default(),
        )),
        Box::new(cpg_extract::ty_flow::TyFlow::default()),
        Box::new(cpg_extract::document_parser::Documents),
        Box::new(cpg_extract::deployment::Deployment),
        Box::new(cpg_extract::assembly::Assemble),
    ]
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
    let model = Arc::new(lctx_model::domain::model()?);
    let workspace =
        Workspace::with_budget(model.clone(), options, captured.config().budget().clone(), native)?;
    if let Err(error)=compile_facts(&workspace,&captured,profile,providers,limits).await {
        let _=workspace.drain().await;return Err(error);
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
    let model = Arc::new(lctx_model::domain::model()?);
    let workspace = Workspace::with_budget(model.clone(), options, budget, native)?;
    let result=async{
        compile_facts(&workspace,&captured,profile,providers,limits).await?;
        workspace.validate().await?;Ok::<(),ModelError>(())
    }.await;
    if let Err(error)=result {let _=workspace.drain().await;return Err(error);}
    Ok((model, workspace))
}
/// Expand pinned family/scope policy using the exact provider authority of completed facts.
/// Executable build hashes identify new extraction, not admission of an immutable capture.
pub(crate) fn recorded_coverage(
    model:&ValidatedModel,profile:Profile,
    inputs:&[lctx_model::domain::input::InputRevision],artifacts:&[lctx_model::domain::source::SourceArtifact],uses:&[lctx_model::domain::input::ArtifactUse],
    recorded:&[lctx_model::domain::attribution::Provider],runs:&[lctx_model::domain::attribution::ProviderRun],families:&[lctx_model::domain::attribution::RunFamily],contexts:&[lctx_model::domain::attribution::AnalysisContext],
    contributions:&[lctx_model::domain::completed::CompletedContribution],authority:&std::collections::BTreeSet<ContentHash>,
    bound_sources:&std::collections::BTreeMap<String,lctx_model::domain::analysis::sources::SourceSnapshot>,
    observed:&[lctx_model::domain::attribution::ProviderCoverage],budget:&lctx_model::domain::resources::ResourceBudget,
)->Result<std::collections::BTreeSet<lctx_model::domain::admission::Expected>,ModelError>{
    use lctx_model::domain::{Record,attribution::*,admission::{Expected,FrontierContract},source::CoverageScope};
    use std::collections::{BTreeMap,BTreeSet};
    let declarations=providers(ContentHash::of(b"facts-coverage-contract")).iter().map(|provider|provider.declaration(profile)).collect::<Vec<_>>();
    let schedule=Schedule::build(model,declarations,&[],profile)?;
    let expected=FrontierContract::facts(model,profile)?.preflight(&schedule)?.expected_coverage(inputs,artifacts,uses)?;
    let required=expected.keys().filter_map(|key|key.provider).collect::<BTreeSet<_>>();
    let prototypes=[cpg_extract::acquisition::acquisition_provider(),cpg_extract::pyrefly_stage::pyrefly_provider(),cpg_extract::ruff_context::provider(),cpg_extract::ty_flow::provider(),cpg_extract::document_parser::provider(),cpg_extract::deployment::provider()];
    let refuse=|message:&str|ModelError::Invalid(format!("recorded facts coverage: {message}"));
    let mut mapped=BTreeMap::new();
    for stage in schedule.stages().iter().filter(|stage|stage.coverage.iter().any(|grant|required.contains(&grant.provider))) {
        let mut owners=contributions.iter().filter(|row|row.spec.producer==stage.name && row.identity().is_ok_and(|identity|authority.contains(&identity)));
        let owner=owners.next().ok_or_else(||refuse("missing completed covering producer"))?;
        if owners.next().is_some(){return Err(refuse("ambiguous completed covering producer"));}
        let inventory=stage.outputs.iter().chain(&stage.contributes).map(|relation|relation.name().to_owned()).collect::<BTreeSet<_>>();
        if !authority.contains(&owner.identity()?) || owner.spec.model!=model.digest() || owner.spec.profile!=profile || owner.spec.outputs!=inventory
            || owner.spec.configuration.is_none() || (![cpg_extract::acquisition::ACQUIRE,cpg_extract::pyrefly_stage::PYREFLY,cpg_extract::ty_flow::TY_FLOW].contains(&stage.name) && owner.spec.configuration!=Some(stage.configuration)) {
            return Err(refuse("covering producer differs from pinned declaration or captured view"));
        }
        let declared_inputs=stage.inputs.iter().map(|input|input.name()).collect::<BTreeSet<_>>();
        let actual_inputs=owner.spec.inputs.iter().map(|input|input.relation()).collect::<BTreeSet<_>>();
        if actual_inputs!=declared_inputs || actual_inputs.len()!=owner.spec.inputs.len() || owner.spec.inputs.iter().any(|input|input.model()!=model.digest() || bound_sources.get(input.relation()).is_some_and(|bound|bound!=input)) {
            return Err(refuse("covering producer changes its exact declared captured inputs"));
        }
        for grant in &stage.coverage {
            if !required.contains(&grant.provider){continue;}
            let prototype=prototypes.iter().find(|provider|provider.id()==grant.provider).ok_or_else(||refuse("unknown pinned covering provider"))?;
            let mut candidates=recorded.iter().filter(|provider|provider.tool==prototype.tool && provider.revision==prototype.revision
                && (prototype.build_digest!=stage.code || provider.build_digest==owner.spec.implementation));
            let provider=candidates.next().ok_or_else(||refuse("missing recorded covering provider"))?;
            if candidates.next().is_some(){return Err(refuse("ambiguous recorded covering provider"));}
            if mapped.insert(grant.provider,provider.id()).is_some_and(|old|old!=provider.id()){return Err(refuse("covering provider changes between declarations"));}
        }
    }
    let expected=expected.into_iter().map(|(key,scope)|Ok((Expected{provider:key.provider.map(|id|mapped.get(&id).copied().ok_or_else(||refuse("unbound covering provider"))).transpose()?,..key},scope))).collect::<Result<BTreeMap<_,_>,ModelError>>()?;
    // Native adapter invocations bind their captured analysis context, not producer limits.
    for run in runs.iter().filter(|run|mapped.values().any(|provider|*provider==run.provider)) {
        let context=contexts.iter().find(|context|context.id()==run.context).ok_or_else(||refuse("covering invocation context is absent"))?;
        if run.configuration!=context.config_digest{return Err(refuse("covering invocation changes its captured context configuration"));}
    }
    // Run membership remains an independent typed premise; coverage never chooses its supplier.
    let mut obligations=Vec::new();
    for (key,scope) in &expected {
        let input=match scope{CoverageScope::Input{input}=>*input,CoverageScope::Artifact{artifact}=>artifacts.iter().find(|row|row.id()==*artifact).ok_or_else(||refuse("coverage artifact is absent"))?.input,_=>return Err(refuse("foreign facts coverage scope grain"))};
        let (provider,context,run)=if let Some(provider)=key.provider {
            let mut matches=runs.iter().filter(|run|run.provider==provider && run.input==input && families.iter().any(|member|member.run==run.id() && member.family==key.family));
            let run=matches.next().ok_or_else(||refuse("missing covering invocation"))?;
            if matches.next().is_some(){return Err(refuse("ambiguous covering invocation"));}
            (provider,run.context,Some(run.id()))
        }else{
            let captured_runs=runs.iter().filter(|run|run.input==input && mapped.values().any(|provider|*provider==run.provider)).collect::<Vec<_>>();
            let contexts=captured_runs.iter().map(|run|run.context).collect::<BTreeSet<_>>();
            if contexts.len()!=1{return Err(refuse("unrequested scope has no exact captured context"));}
            // The validator ignores provider for an unrequested outcome; retain a captured
            // supplier here rather than introducing an executable-generated placeholder.
            (captured_runs[0].provider,*contexts.first().expect("one context"),None)
        };
        obligations.push(CoverageExpectation{input,scope:key.scope,provider,context,family:key.family,run});
    }
    validate_coverage_contract(&obligations,observed,runs,families,budget)?;
    Ok(expected.into_keys().collect())
}

#[cfg(test)]
mod recorded_coverage_controls {
    use super::*;
    use lctx_model::domain::{Record,attribution::*,completed::*,input::InputRevision,source::CoverageScope,resources::ResourceBudget};
    use std::collections::{BTreeMap,BTreeSet};
    #[test]
    fn restored_provider_builds_retain_exact_obligations_and_refuse_foreign_authority() {
        let model=lctx_model::domain::model().unwrap();let profile=Profile::Catalog;
        let budget=ResourceBudget::fixed(8<<20).unwrap();
        let input=InputRevision::from_entries(vec![]).unwrap();let scope=CoverageScope::Input{input:input.id()};
        let captured_context=AnalysisContext{python_version:"3.14.7".into(),python_platform:"linux".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"capture"),environment_digest:ContentHash::of(b"capture"),lock_digest:None};
        let context=captured_context.id();
        let mut acquire=cpg_extract::acquisition::acquisition_provider();let mut deployment=cpg_extract::deployment::provider();let mut pyrefly=cpg_extract::pyrefly_stage::pyrefly_provider();
        acquire.build_digest=ContentHash::of(b"recorded extraction before executable lock change");
        deployment.build_digest=ContentHash::of(b"recorded deployment before executable lock change");
        assert_ne!(acquire.id(),cpg_extract::acquisition::acquisition_provider().id());
        pyrefly.build_digest=ContentHash::of(b"recorded pyrefly before executable lock change");
        let providers=vec![acquire.clone(),deployment.clone(),pyrefly.clone()];
        let mut contributions=Vec::new();
        for stage in self::super::providers(ContentHash::of(b"fixture")).iter().map(|provider|provider.declaration(profile)).filter(|stage|[cpg_extract::acquisition::ACQUIRE,cpg_extract::deployment::DEPLOYMENT,cpg_extract::pyrefly_stage::PYREFLY].contains(&stage.name)) {
            let implementation=if stage.name==cpg_extract::acquisition::ACQUIRE{acquire.build_digest}else if stage.name==cpg_extract::deployment::DEPLOYMENT{deployment.build_digest}else{pyrefly.build_digest};
            let outputs=stage.outputs.iter().chain(&stage.contributes).map(|row|row.name().to_owned()).collect::<BTreeSet<_>>();
            let content=outputs.iter().map(|name|(name.clone(),OutputContent{rows:0,content:ContentHash::of(name.as_bytes())})).collect();
            contributions.push(CompletedContribution{spec:ContributionSpec{producer:stage.name.into(),profile,model:model.digest(),implementation,configuration:Some(stage.configuration),inputs:stage.inputs.iter().map(|input|lctx_model::domain::analysis::sources::SourceSnapshot::of_relation(model.relation(input.name()).unwrap(),"finite captured dependency",model.digest(),ContentHash::of(b"recorded dependency"),ContentHash::of(input.name().as_bytes()),0).unwrap()).collect(),outputs},outcome:0,outputs:content});
        }
        let authority=contributions.iter().map(|row|row.identity().unwrap()).collect();
        let mut runs=Vec::new();let mut families=Vec::new();let mut observed=Vec::new();
        for (provider,family) in [(&acquire,FactFamily::Artifacts),(&deployment,FactFamily::Deployment),(&pyrefly,FactFamily::Signatures)] {
            let requests=if provider.id()==pyrefly.id(){cpg_extract::pyrefly_stage::FAMILIES.into_iter().filter(|family|*family!=FactFamily::Syntax).collect::<Vec<_>>()}else{vec![family]};
            let (run,members)=ProviderRun::new(provider.id(),context,input.id(),ContentHash::of(b"capture"),requests).unwrap();
            observed.push(ProviderCoverage{scope:scope.id(),provider:Some(provider.id()),context,family,run:Some(run.id()),status:CoverageStatus::CompleteUnderStatedModel,reason:None,diagnostic:None});
            runs.push(run);families.extend(members);
        }
        let check=|providers:&[Provider],contributions:&[CompletedContribution],observed:&[ProviderCoverage]|recorded_coverage(&model,profile,std::slice::from_ref(&input),&[],&[],providers,&runs,&families,std::slice::from_ref(&captured_context),contributions,&authority,&BTreeMap::new(),observed,&budget);
        let expected=check(&providers,&contributions,&observed).unwrap();
        assert_eq!(expected.iter().filter_map(|key|key.provider).collect::<BTreeSet<_>>(),providers.iter().map(Record::id).collect());
        lctx_model::domain::admission::ScopedAvailability::from_completed(profile,&expected,&observed,&BTreeMap::from([(scope.id(),scope)]),&budget).unwrap();
        assert!(check(&providers[1..],&contributions,&observed).is_err(),"missing recorded supplier is refused");
        let mut ambiguous=providers.clone();let mut duplicate=acquire.clone();duplicate.build_digest=ContentHash::of(b"other build");ambiguous.push(duplicate);
        // Only the descriptor-bound primary build can be the covering supplier.
        assert!(check(&ambiguous,&contributions,&observed).is_ok());
        let mut unselected=contributions.clone();let mut other=contributions[0].clone();other.spec.implementation=ContentHash::of(b"unselected same producer");unselected.push(other);
        assert!(check(&providers,&unselected,&observed).is_ok(),"unselected same-producer metadata cannot widen captured authority");
        let mut custom=contributions.clone();
        let owner=custom.iter_mut().find(|row|row.spec.producer==cpg_extract::pyrefly_stage::PYREFLY).unwrap();owner.spec.configuration=Some(ContentHash::of(b"captured custom SyntaxLimits"));
        let custom_authority=custom.iter().map(|row|row.identity().unwrap()).collect();
        let custom_check=|owners:&[CompletedContribution],authority:&BTreeSet<ContentHash>|recorded_coverage(&model,profile,std::slice::from_ref(&input),&[],&[],&providers,&runs,&families,std::slice::from_ref(&captured_context),owners,authority,&BTreeMap::new(),&observed,&budget);
        assert!(custom_check(&custom,&custom_authority).is_ok(),"captured configurable limits are independent of executable defaults");
        let owner=custom.iter_mut().find(|row|row.spec.producer==cpg_extract::pyrefly_stage::PYREFLY).unwrap();owner.spec.configuration=None;
        let no_configuration_authority=custom.iter().map(|row|row.identity().unwrap()).collect();
        assert!(custom_check(&custom,&no_configuration_authority).is_err(),"covering producer requires captured configuration");
        let mut selected_owners=contributions.clone();let mut selected_duplicate=contributions[0].clone();selected_duplicate.spec.implementation=ContentHash::of(b"second selected same producer");selected_owners.push(selected_duplicate);
        let selected_authority=selected_owners.iter().map(|row|row.identity().unwrap()).collect();
        assert!(custom_check(&selected_owners,&selected_authority).is_err(),"two selected covering owners are ambiguous");
        let mut wrong_runs=runs.clone();let old_run=wrong_runs[0].id();wrong_runs[0].configuration=ContentHash::of(b"foreign context settings");let new_run=wrong_runs[0].id();
        let mut wrong_memberships=families.clone();for row in &mut wrong_memberships{if row.run==old_run{row.run=new_run;}}
        let mut wrong_outcomes=observed.clone();for row in &mut wrong_outcomes{if row.run==Some(old_run){row.run=Some(new_run);}}
        assert!(recorded_coverage(&model,profile,std::slice::from_ref(&input),&[],&[],&providers,&wrong_runs,&wrong_memberships,std::slice::from_ref(&captured_context),&contributions,&authority,&BTreeMap::new(),&wrong_outcomes,&budget).is_err(),"internally consistent outcomes cannot change the captured invocation context configuration");
        let mut malformed=contributions.clone();malformed[0].spec.model=ContentHash::of(b"foreign model");
        assert!(check(&providers,&malformed,&observed).is_err(),"foreign descriptor authority is refused");
        let mut foreign=observed.clone();foreign[0].provider=Some(cpg_extract::acquisition::acquisition_provider().id());
        assert!(check(&providers,&contributions,&foreign).is_err(),"current executable identity cannot replace the recorded supplier");
        let mut wrong_family=observed.clone();wrong_family[0].family=FactFamily::Syntax;
        assert!(check(&providers,&contributions,&wrong_family).is_err());
        assert!(check(&providers,&contributions,&observed[1..]).is_err(),"missing coverage is refused");
        let mut extra=observed.clone();extra.push(observed[0].clone());
        assert!(check(&providers,&contributions,&extra).is_err(),"extra coverage is refused");
    }
}
