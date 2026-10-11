//! Recorded fact admission from the model-owned semantic inventory.
use lctx_model::domain::{ContentHash, ModelError, ValidatedModel, stages::Profile};
pub struct RecordedCoverage {
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
pub struct RecordedFacts<'a> {
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
pub fn recorded_coverage(
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
    let contracts = lctx_model::domain::producer_contract::facts::facts();
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

