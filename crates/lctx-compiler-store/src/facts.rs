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
    pub uniform: Vec<RecordedUniform>,
}
pub struct RecordedUniform {
    pub descriptor: lctx_model::domain::uniform_outcome::UniformNotRequested,
    pub domain: lctx_model::domain::uniform_outcome::ValidatedUniformDomain,
    pub owner: lctx_model::domain::uniform_outcome::ValidatedUniformOwner,
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
    recorded_coverage_with_uniform(
        model,
        profile,
        facts,
        &[],
        &std::collections::BTreeSet::new(),
        budget,
    )
}
/// Compact rows use the exact selected uniform view's contributor inventory. Their
/// scope domain is reconstructed from the independently admitted captured inputs;
/// no synthetic per-scope ProviderCoverage records are manufactured.
pub fn recorded_coverage_with_uniform(
    model: &ValidatedModel,
    profile: Profile,
    facts: &RecordedFacts<'_>,
    compact: &[lctx_model::domain::uniform_outcome::UniformNotRequested],
    compact_authority: &std::collections::BTreeSet<ContentHash>,
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
    use lctx_model::domain::producer_assurance::{
        AdmittedProducerDeclaration, AdmittedSourceBinding,
    };
    let mut bindings: BTreeMap<&str, AdmittedProducerDeclaration<'_>> = BTreeMap::new();
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
        let declaration = AdmittedProducerDeclaration::for_facts(
            owner,
            contract,
            recorded,
            model.digest(),
            profile,
        )?;
        if owner.spec.inputs.iter().any(|input| {
            input.model() != model.digest()
                || bound_sources
                    .get(input.relation())
                    .is_some_and(|bound| bound != input)
        }) {
            return Err(refuse(
                "producer changes its exact declared captured inputs",
            ));
        }
        if declaration
            .source_bindings()?
            .map(|source| source.input())
            .collect::<BTreeSet<_>>()
            != input_ids
        {
            return Err(refuse("producer changes its captured source universe"));
        }
        for source in declaration.source_bindings()? {
            let Some(context_id) = source.context() else {
                continue;
            };
            let mut matched = contexts.iter().filter(|context| context.id() == context_id);
            let context = matched
                .next()
                .ok_or_else(|| refuse("captured source context is absent"))?;
            if matched.next().is_some() || Some(context.config_digest) != source.configuration() {
                return Err(refuse("captured source context configuration differs"));
            }
        }
        charge.grow(size_of::<AdmittedProducerDeclaration<'_>>() + 32)?;
        for role in declaration.suppliers() {
            let provider = declaration.supplier(role.name)?;
            for family in &role.families {
                charge.grow(
                    size_of::<(
                        (FactFamily, Option<lctx_model::domain::Id<Provider>>),
                        &'static str,
                    )>() + 32,
                )?;
                if reporting
                    .insert((*family, Some(provider)), declaration.name())
                    .is_some()
                {
                    return Err(refuse("ambiguous family reporting owner"));
                }
            }
            charge.grow(
                size_of::<lctx_model::domain::producer_contract::SupplierRole>()
                    + role.families.len() * size_of::<FactFamily>()
                    + 32,
            )?;
            if suppliers.insert(provider, (contract.name, role)).is_some() {
                return Err(refuse("ambiguous supplier role authority"));
            }
        }
        bindings.insert(contract.name, declaration);
    }
    let assembly = bindings
        .get(lctx_model::domain::producer_contract::facts::ASSEMBLE)
        .ok_or_else(|| refuse("missing Assembly semantic context binding"))?;
    charge.grow(assembly.source_bindings()?.count() * size_of::<AdmittedSourceBinding<'_>>())?;
    let current_sources = assembly.source_bindings()?.collect::<Vec<_>>();
    let semantic_sources = current_sources
        .iter()
        .map(|source| (source.input(), (source.context(), source.configuration())))
        .collect::<BTreeMap<_, _>>();
    let mut acquisition_sources: Option<
        &[lctx_model::domain::producer_contract::CapturedSourceBinding],
    > = None;
    for declaration in bindings.values() {
        if let Some(binding) = declaration.completed().spec.captured_binding.as_ref() {
            if let Some(prior) = acquisition_sources
                && (prior.len() != binding.sources.len()
                    || prior.iter().zip(&binding.sources).any(|(left, right)| {
                        left.input != right.input || left.acquisition != right.acquisition
                    }))
            {
                return Err(refuse(
                    "producer captured acquisition order or metadata disagree",
                ));
            }
            acquisition_sources = Some(&binding.sources);
            if matches!(
                binding.context_dependency,
                lctx_model::domain::producer_contract::CapturedContextDependency::AcquisitionOnly
            ) {
                continue;
            }
        }
        if declaration
            .source_bindings()?
            .map(|source| (source.input(), (source.context(), source.configuration())))
            .collect::<BTreeMap<_, _>>()
            != semantic_sources
        {
            return Err(refuse(
                "producer captured semantic contexts disagree with Assembly",
            ));
        }
    }
    if recorded.iter().map(Record::id).collect::<BTreeSet<_>>()
        != suppliers.keys().copied().collect()
        || recorded.len() != suppliers.len()
    {
        return Err(refuse("missing or extra selected supplier records"));
    }
    for declaration in bindings.values() {
        for family in declaration.not_requested().iter().filter(|family| {
            !FrontierContract::facts(model, profile)
                .is_ok_and(|contract| contract.requested(**family))
        }) {
            charge.grow(
                size_of::<(
                    (FactFamily, Option<lctx_model::domain::Id<Provider>>),
                    &'static str,
                )>() + 32,
            )?;
            if reporting
                .insert((*family, None), declaration.name())
                .is_some()
            {
                return Err(refuse("ambiguous unrequested reporting owner"));
            }
        }
    }
    for declaration in contracts
        .iter()
        .filter(|declaration| !declaration.profiles.contains(&profile))
    {
        for family in &declaration.not_requested {
            if !FrontierContract::facts(model, profile)?.requested(*family) {
                charge.grow(
                    size_of::<(
                        (FactFamily, Option<lctx_model::domain::Id<Provider>>),
                        &'static str,
                    )>() + 32,
                )?;
                if reporting
                    .insert((*family, None), declaration.name)
                    .is_some()
                {
                    return Err(refuse("ambiguous unrequested reporting owner"));
                }
            }
        }
    }
    let preflight = FrontierContract::facts(model, profile)?
        .admitted_preflight(&bindings.values().collect::<Vec<_>>())?;
    let expected = preflight.expected_coverage(inputs, artifacts, uses)?;
    let mut invocation_keys = BTreeSet::new();
    for run in runs {
        let (producer, role) = suppliers
            .get(&run.provider)
            .ok_or_else(|| refuse("invocation has no declared captured supplier"))?;
        let declaration = &bindings[producer];
        let source = if matches!(
            role.configuration,
            ConfigurationBinding::CurrentAssemblyContext
        ) {
            current_sources
                .iter()
                .copied()
                .find(|source| source.input() == run.input)
        } else {
            declaration
                .source_bindings()?
                .find(|source| source.input() == run.input)
        }
        .ok_or_else(|| refuse("invocation source is outside capture"))?;
        let configuration = match role.configuration {
            ConfigurationBinding::CapturedAnalysisContext
            | ConfigurationBinding::CurrentAssemblyContext => source
                .configuration()
                .ok_or_else(|| refuse("supplier has no exact context configuration"))?,
            ConfigurationBinding::ProducerSettings => declaration.producer_settings()?,
            ConfigurationBinding::Fixed(value) => value,
        };
        let requested = families
            .iter()
            .filter(|member| member.run == run.id())
            .map(|member| member.family)
            .collect::<BTreeSet<_>>();
        if Some(run.context) != source.context()
            || run.configuration != configuration
            || requested != role.families.iter().copied().collect()
            || !invocation_keys.insert((run.provider, run.input))
        {
            return Err(refuse(
                "invocation changes declared family/source/configuration binding",
            ));
        }
    }
    for (provider, (producer, _)) in &suppliers {
        for source in bindings[producer].source_bindings()? {
            if !invocation_keys.contains(&(*provider, source.input())) {
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
        let source = current_sources
            .iter()
            .find(|source| source.input() == input)
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
            context: source
                .context()
                .ok_or_else(|| refuse("coverage has no exact semantic context"))?,
            family: key.family,
            run,
        });
    }
    use lctx_model::domain::uniform_outcome::{ExpectedUniform, ValidatedUniformOwner};
    let _compact_scratch = budget.reserve(
        "recorded compact coverage keys",
        expected
            .len()
            .saturating_mul(128)
            .saturating_add(compact.len().saturating_mul(128)),
    )?;
    let mut compact_keys = BTreeSet::new();
    let mut descriptors = BTreeSet::new();
    let mut uniform = Vec::new();
    for descriptor in compact {
        if descriptor.kind != lctx_model::domain::uniform_outcome::UniformKind::NativeFamily {
            continue;
        }
        if !descriptors.insert(descriptor.id()) {
            return Err(refuse("duplicate compact coverage descriptor"));
        }
        let mut owners = contributions.iter().filter(|owner| {
            owner.spec.producer == descriptor.owner
                && owner
                    .identity()
                    .is_ok_and(|identity| compact_authority.contains(&identity))
        });
        let owner = owners
            .next()
            .ok_or_else(|| refuse("compact coverage has no exact selected completed owner"))?;
        if owners.next().is_some() {
            return Err(refuse("ambiguous compact coverage completed owner"));
        }
        let owner =
            ValidatedUniformOwner::validate_semantic_binding(owner, descriptor, profile, budget)?;
        let input = inputs
            .iter()
            .find(|input| input.id() == descriptor.input)
            .ok_or_else(|| refuse("compact coverage input absent"))?;
        let context = contexts
            .iter()
            .find(|context| context.id() == descriptor.context)
            .ok_or_else(|| refuse("compact coverage context absent"))?;
        let domain = ExpectedUniform::native_inputs(
            &owner, input, context, artifacts, uses, profile, budget,
        )?;
        let family = descriptor
            .family
            .ok_or_else(|| refuse("compact native coverage family absent"))?;
        for (scope, _) in domain.members() {
            let key = lctx_model::domain::admission::Expected {
                scope: *scope,
                family,
                provider: None,
            };
            if !expected.contains_key(&key) || !compact_keys.insert(key) {
                return Err(refuse(
                    "compact coverage changes or repeats expected scope domain",
                ));
            }
        }
        charge.grow(
            size_of::<RecordedUniform>() + lctx_model::domain::HeapSize::heap_bytes(descriptor),
        )?;
        uniform.push(RecordedUniform {
            descriptor: descriptor.clone(),
            domain,
            owner,
        });
    }
    obligations.retain(|obligation| {
        !compact_keys.contains(&lctx_model::domain::admission::Expected {
            scope: obligation.scope,
            family: obligation.family,
            provider: obligation.provider,
        })
    });
    validate_coverage_contract(&obligations, observed, runs, families, budget)?;
    Ok(RecordedCoverage {
        contract: preflight.contract().digest(),
        expected: expected.into_keys().collect(),
        reporting,
        charge,
        uniform,
    })
}
