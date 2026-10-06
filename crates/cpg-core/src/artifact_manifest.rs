//! Compact semantic manifest from completed owners. Diagnostics and physical realizations do not
//! enter identity; nominal outcomes and exact consumed embedding values do.
use crate::workspace::Workspace;
use datafusion::arrow::array::{Array, BinaryArray};
use futures::TryStreamExt;
use lctx_model::domain::{
    self as d,
    admission::Frontier,
    graph::{self, *},
    stages::Profile,
    *,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn invalid(message: impl Into<String>) -> ModelError {
    ModelError::Invalid(message.into())
}
fn visit<R: Record>(
    workspace: &Workspace,
    mut accept: impl FnMut(&R) -> Result<(), ModelError>,
) -> Result<(), ModelError> {
    for batch in workspace
        .completed::<R>()?
        .read::<R>(workspace.model().clone(), workspace.budget().clone())?
    {
        let batch = batch?;
        for row in batch.rows() {
            accept(row)?;
        }
    }
    Ok(())
}
fn add_outcome(
    outcomes: &mut BTreeMap<OutcomeKey, Outcome>,
    charge: &mut charged::StateCharge,
    value: Outcome,
) -> Result<(), ModelError> {
    if outcomes.contains_key(&value.key) {
        return Err(invalid("duplicate manifest outcome obligation"));
    }
    charge.grow(
        size_of::<Outcome>()
            + value.key.producer.len()
            + value.detail.as_ref().map_or(0, String::len)
            + 64,
    )?;
    outcomes.insert(value.key.clone(), value);
    Ok(())
}
fn settings(workspace: &Workspace, acquisition: ContentHash) -> Result<ContentHash, ModelError> {
    let mut sink = KeySink::new("graph-compiler-settings/v1");
    acquisition.encode(&mut sink);
    // Model-owned configuration streams only. Endpoint/runtime diagnostics are deliberately absent.
    let mut names = vec![
        d::analysis::AnalysisDefinition::NAME,
        d::analysis::MethodParameters::NAME,
        d::analysis::ProjectionDefinition::NAME,
        d::analysis::settings::AnalyticsConfiguration::NAME,
        d::models::ModelCatalog::NAME,
        d::models::AuthoredTarget::NAME,
        d::models::AuthoredModel::NAME,
        d::models::AuthoredContextProtocol::NAME,
        d::embedding::EmbeddingSpec::NAME,
        d::embedding::text::TextDefinition::NAME,
        d::retrieval::RetrievalDefinition::NAME,
    ];
    let completed = workspace
        .completed_relations()?
        .iter()
        .map(|r| r.name())
        .collect::<BTreeSet<_>>();
    names.sort_unstable();
    for name in names {
        if completed.contains(name) {
            sink.part(b"relation", name.as_bytes());
            workspace.relation(name)?.content().encode(&mut sink);
        }
    }
    Ok(sink.finish())
}
/// Build the versioned manifest after graph stream admission. Metadata indexes contain only
/// definitions, scoped obligations and vector digests, never resident graph payloads.
pub async fn populate(
    workspace: &Arc<Workspace>,
    frontier: Frontier,
    profile: Profile,
    acquisition_config: ContentHash,
    mut captures: Vec<EntityId>,
    mut originals: Vec<Original>,
    mut families: Vec<FamilyContent>,
) -> Result<(Manifest, charged::StateCharge), ModelError> {
    let mut charge = charged::StateCharge::new(workspace.budget(), "artifact-manifest");
    charge.grow(
        size_of::<Manifest>()
            + captures.len() * size_of::<EntityId>()
            + originals.len() * size_of::<Original>()
            + families.len() * size_of::<FamilyContent>(),
    )?;
    let mut producers = BTreeMap::new();
    for relation in workspace.completed_relations()? {
        let configuration = relation.configuration().ok_or_else(|| {
            invalid(format!(
                "{} has no declared producer configuration",
                relation.name()
            ))
        })?;
        let value = ProducerImplementation {
            producer: relation.producer().into(),
            implementation: relation.implementation(),
            configuration,
        };
        if let Some(old) = producers.get(&value.producer) {
            if old != &value {
                return Err(invalid(
                    "conflicting producer implementation or configuration",
                ));
            }
        } else {
            charge.grow(size_of::<ProducerImplementation>() + value.producer.len() + 32)?;
            producers.insert(value.producer.clone(), value);
        }
    }
    let outcomes = semantic_outcomes(workspace, frontier, profile, acquisition_config, &captures, &mut charge)?;
    let projections = projections(workspace, frontier, &mut charge)?;
    let embeddings = embeddings(workspace, profile, &mut charge).await?;
    captures.sort_unstable();
    if captures.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid("duplicate manifest capture"));
    }
    originals.sort_by_key(|r| r.source);
    if originals.windows(2).any(|w| w[0].source == w[1].source) {
        return Err(invalid("duplicate manifest original"));
    }
    families.sort_by_key(|r| r.family);
    if families.windows(2).any(|w| w[0].family == w[1].family) {
        return Err(invalid("duplicate manifest family"));
    }
    let manifest = Manifest {
        format_version: ARTIFACT_FORMAT_VERSION,
        frontier,
        profile,
        captures,
        semantic_contract: graph::semantic_contract(workspace.model()),
        producers: producers.into_values().collect(),
        settings: settings(workspace, acquisition_config)?,
        families,
        required_outcomes: outcomes.keys().cloned().collect(),
        outcomes: outcomes.into_values().collect(),
        originals,
        projections,
        embeddings,
    };
    manifest.validate()?;
    Ok((manifest, charge))
}

/// Canonical outcome membership is derived from retained captures, profile, definitions and
/// nominal invocations. Transport metadata never supplies its own expected key universe.
fn semantic_outcomes(
    workspace: &Workspace, frontier: Frontier, profile: Profile, acquisition_config: ContentHash,
    captures: &[EntityId], charge: &mut charged::StateCharge,
) -> Result<BTreeMap<OutcomeKey, Outcome>, ModelError> {
    let available = workspace.facts_availability(profile)?;
    let reporting = crate::facts::providers(acquisition_config)
        .into_iter()
        .flat_map(|provider| {
            let stage = provider.declaration(profile);
            stage
                .coverage
                .into_iter()
                .map(move |coverage| ((coverage.family, coverage.provider), stage.name))
        })
        .collect::<BTreeMap<_, _>>();
    let mut expected = BTreeMap::new();
    for evidence in available.evidence() {
        let producer = match evidence.provider {
            Some(provider) => *reporting
                .get(&(evidence.family, provider))
                .ok_or_else(|| invalid("coverage lacks declared native owner"))?,
            None => cpg_extract::assembly::ASSEMBLE,
        };
        let mut domain = KeySink::new("native-outcome-domain/v1");
        evidence.family.encode(&mut domain);
        evidence.provider.encode(&mut domain);
        let key = OutcomeKey {
            producer: producer.into(),
            scope: EntityId::of(evidence.scope),
            domain: domain.finish(),
        };
        charge.grow(
            size_of::<(Id<d::attribution::ProviderCoverage>, OutcomeKey)>() + producer.len() + 32,
        )?;
        if expected.insert(evidence.coverage, key).is_some() {
            return Err(invalid("duplicate native obligation"));
        }
    }
    let mut outcomes = BTreeMap::new();
    visit::<d::attribution::ProviderCoverage>(workspace, |row| {
        let key = expected
            .remove(&row.id())
            .ok_or_else(|| invalid("extra native outcome"))?;
        let status = match row.status {
            d::attribution::CoverageStatus::CompleteUnderStatedModel => OutcomeKind::Complete,
            d::attribution::CoverageStatus::Partial => OutcomeKind::Partial,
            d::attribution::CoverageStatus::Unavailable => OutcomeKind::Unavailable,
            d::attribution::CoverageStatus::NotRequested => OutcomeKind::NotRequested,
            d::attribution::CoverageStatus::Failed => OutcomeKind::Failed,
        };
        add_outcome(
            &mut outcomes,
            charge,
            Outcome {
                key,
                status,
                observed: None,
                detail: row.reason.map(|reason| format!("{reason:?}")),
            },
        )
    })?;
    if !expected.is_empty() {
        return Err(invalid("missing native outcome"));
    }
    let mut definitions = BTreeSet::new();
    if matches!(frontier, Frontier::Analysis | Frontier::Catalog) {
        visit::<d::analysis::AnalysisDefinition>(workspace, |row| {
            charge.grow(size_of::<Id<d::analysis::AnalysisDefinition>>() + 32)?;
            definitions.insert(row.id());
            Ok(())
        })?;
    }
    let capture_set = captures.iter().copied().collect::<BTreeSet<_>>();
    macro_rules! upper {
        ($owner:ident) => {{
            workspace.completed::<d::analysis::$owner::AnalysisInvocation>()?;
            let producer = crate::compilation::UpperStage::for_outcome_relation(d::analysis::$owner::AnalysisInvocation::NAME)
                .ok_or(ModelError::Schema("analysis outcome owner"))?.name();
            workspace.completed::<d::analysis::$owner::AnalysisOutcome>()?;
            let mut invocations = BTreeMap::new();
            let mut invocation_charge =
                charged::StateCharge::new(workspace.budget(), "manifest-invocation-keys");
            visit::<d::analysis::$owner::AnalysisInvocation>(workspace, |row| {
                if !definitions.contains(&row.definition)
                    || !capture_set.contains(&EntityId::of(row.input))
                {
                    return Err(invalid("analysis invocation lacks definition or capture"));
                }
                let mut sink = KeySink::new("analysis-outcome-domain/v1");
                sink.part(
                    b"relation",
                    d::analysis::$owner::AnalysisInvocation::NAME.as_bytes(),
                );
                row.content_digest().encode(&mut sink);
                let key = OutcomeKey {
                    producer: producer.into(),
                    scope: EntityId::of(row.input),
                    domain: sink.finish(),
                };
                invocation_charge.grow(
                    size_of::<(Id<d::analysis::$owner::AnalysisInvocation>, OutcomeKey)>()
                        + key.producer.len()
                        + 32,
                )?;
                if invocations.insert(row.id(), key).is_some() {
                    return Err(invalid("duplicate analysis invocation"));
                }
                Ok(())
            })?;
            visit::<d::analysis::$owner::AnalysisOutcome>(workspace, |row| {
                let key = invocations
                    .remove(&row.invocation)
                    .ok_or_else(|| invalid("extra or duplicate analysis outcome"))?;
                let status = match row.status {
                    d::analysis::AnalysisStatus::Completed => OutcomeKind::Complete,
                    d::analysis::AnalysisStatus::Partial => OutcomeKind::Partial,
                    d::analysis::AnalysisStatus::Unavailable => OutcomeKind::Unavailable,
                    d::analysis::AnalysisStatus::NotRequested => OutcomeKind::NotRequested,
                };
                add_outcome(
                    &mut outcomes,
                    charge,
                    Outcome {
                        key,
                        status,
                        observed: None,
                        detail: row.reason.map(|reason| format!("{reason:?}")),
                    },
                )
            })?;
            if !invocations.is_empty() {
                return Err(invalid(concat!(
                    stringify!($owner),
                    " lacks exact outcome membership"
                )));
            }
        }};
    }
    if matches!(frontier, Frontier::Analysis | Frontier::Catalog) {
        upper!(local);
        upper!(base_evaluation);
        upper!(base_completion);
        upper!(source_call);
        upper!(enriched_execution);
        upper!(model);
        upper!(summary);
        upper!(structural);
        upper!(analytic_embedding);
        upper!(analytic);
        upper!(catalog_core);
        upper!(catalog_evidence);
    }
    if frontier == Frontier::Catalog {
        upper!(selection);
        upper!(synthesis);
        upper!(retrieval);
    }
    Ok(outcomes)
}
pub(crate) fn verify_outcomes(workspace: &Workspace, manifest: &Manifest) -> Result<(), ModelError> {
    let mut charge = charged::StateCharge::new(workspace.budget(), "detached-outcome-admission");
    let actual = semantic_outcomes(workspace, manifest.frontier, manifest.profile, manifest.settings, &manifest.captures, &mut charge)?;
    if actual.keys().cloned().collect::<Vec<_>>() != manifest.required_outcomes
        || actual.into_values().collect::<Vec<_>>() != manifest.outcomes
    { return Err(ModelError::Conflict("artifact required semantic outcome membership")); }
    Ok(())
}

fn projections(
    workspace: &Workspace,
    frontier: Frontier,
    charge: &mut charged::StateCharge,
) -> Result<Vec<graph::ProjectionDefinition>, ModelError> {
    if frontier == Frontier::Facts {
        return Ok(vec![]);
    }
    let mut names = BTreeSet::new();
    visit::<d::projection::ProjectionSourceAssessment>(workspace, |row| {
        names.insert(row.projection);
        Ok(())
    })?;
    let mut definitions = BTreeMap::new();
    if matches!(frontier, Frontier::Analysis | Frontier::Catalog) {
        visit::<d::analysis::ProjectionDefinition>(workspace, |row| {
            definitions.insert(row.id(), row.clone());
            Ok(())
        })?;
    }
    let mut uses = BTreeMap::<d::projection::ProjectionName, KeySink>::new();
    let completed = workspace
        .completed_relations()?
        .iter()
        .map(|r| r.name())
        .collect::<BTreeSet<_>>();
    macro_rules! use_projection {
        ($owner:ident) => {
            if completed.contains(d::analysis::$owner::ProjectionInput::NAME) {
                visit::<d::analysis::$owner::ProjectionInput>(workspace, |row| {
                    let definition = definitions
                        .get(&row.projection)
                        .ok_or_else(|| invalid("selected projection lacks semantic definition"))?;
                    if !names.contains(&definition.name) {
                        return Err(invalid("selected projection lacks source assessment"));
                    }
                    let sink = uses
                        .entry(definition.name)
                        .or_insert_with(|| KeySink::new("projection-selected-inputs/v1"));
                    sink.part(
                        b"relation",
                        d::analysis::$owner::ProjectionInput::NAME.as_bytes(),
                    );
                    row.content_digest().encode(sink);
                    Ok(())
                })?;
            }
        };
    }
    use_projection!(local);
    use_projection!(base_evaluation);
    use_projection!(base_completion);
    use_projection!(source_call);
    use_projection!(enriched_execution);
    use_projection!(model);
    use_projection!(summary);
    use_projection!(structural);
    use_projection!(analytic_embedding);
    use_projection!(analytic);
    use_projection!(catalog_core);
    use_projection!(catalog_evidence);
    use_projection!(selection);
    use_projection!(synthesis);
    use_projection!(retrieval);
    let mut output = Vec::new();
    for name in names {
        let definition = d::analysis::ProjectionDefinition::builtin(name);
        let mut membership = KeySink::new("projection-source-membership/v1");
        name.encode(&mut membership);
        // Exact completed source relations, including vertex universe and gap/coverage lineage.
        let mut source_names = d::projection::ProjectionSpec::builtin(name)
            .roles()
            .iter()
            .map(|role| role.relation())
            .collect::<BTreeSet<_>>();
        source_names.extend([
            d::normalized::entities::EntityRef::NAME,
            d::projection::ProjectionSourceAssessment::NAME,
            d::projection::ProjectionSourceCoverage::NAME,
            d::projection::ProjectionGap::NAME,
            d::projection::ProjectionGapSubject::NAME,
            d::attribution::ProviderCoverage::NAME,
        ]);
        for source in source_names {
            membership.part(b"relation", source.as_bytes());
            workspace
                .relation(source)?
                .content()
                .encode(&mut membership);
        }
        if let Some(selected) = uses.remove(&name) {
            selected.finish().encode(&mut membership);
        }
        // These are explicit properties of ProjectionSpec::accepts and topology snapshots:
        // the snapshot stores accepted endpoints and typed arcs, while source assertions retain
        // conditions, provider qualification and original evidence.
        let mut excluded = d::projection::ProjectionSpec::builtin(name)
            .excluded_categories()
            .map(|category| category.label())
            .collect::<Vec<_>>();
        excluded.sort_unstable();
        let mut losses=vec![format!("ProjectionSpec::accepts excludes {} entities",excluded.join(", ")),"topology omits conditions, provider qualifications and source evidence retained by assertions".into()];
        losses.sort();
        let value = graph::ProjectionDefinition {
            name: format!("{name:?}"),
            definition: definition.content_digest(),
            source_membership: membership.finish(),
            declared_losses: losses,
        };
        charge.grow(
            size_of::<graph::ProjectionDefinition>()
                + value.name.len()
                + value.declared_losses.iter().map(String::len).sum::<usize>(),
        )?;
        output.push(value);
    }
    output.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(output)
}

async fn embeddings(
    workspace: &Workspace,
    profile: Profile,
    charge: &mut charged::StateCharge,
) -> Result<Vec<EmbeddingConsumption>, ModelError> {
    let mut specs = BTreeMap::new();
    let completed = workspace
        .completed_relations()?
        .iter()
        .map(|r| r.name())
        .collect::<BTreeSet<_>>();
    if !completed.contains(d::embedding::EmbeddingSpec::NAME) {
        if completed.contains(d::embedding::analytic::AnalysisEmbeddingUse::NAME)
            || completed.contains(d::retrieval::consumption::RetrievalEmbeddingUse::NAME)
        {
            return Err(invalid("embedding consumers lack specification relation"));
        }
        return Ok(vec![]);
    }
    visit::<d::embedding::EmbeddingSpec>(workspace, |row| {
        charge.grow(size_of::<d::embedding::Spec>() + row.heap_bytes() + 32)?;
        specs.insert(row.id(), row.configuration()?);
        Ok(())
    })?;
    let mut selected = BTreeSet::new();
    if completed.contains(d::embedding::configuration::ServiceConfiguration::NAME) {
        visit::<d::embedding::configuration::ServiceConfiguration>(workspace, |row| {
            if !specs.contains_key(&row.specification) {
                return Err(invalid(
                    "selected embedding configuration lacks specification",
                ));
            }
            selected.insert(row.specification);
            Ok(())
        })?;
    }
    let mut vectors = BTreeMap::new();
    macro_rules! consume {($record:ty,$text:ty,$field:ident)=>{if completed.contains(<$record>::NAME){
        let inputs=workspace.inputs("manifest-vector-binding",profile,[<$record>::NAME,<$text>::NAME])?;
        let context=inputs.session(workspace).await?;
        let query=format!("SELECT u.*, t.text AS consumed_text FROM {} u LEFT JOIN {} t ON u.{} = t.id",<$record>::NAME,<$text>::NAME,stringify!($field));
        let mut stream=crate::sql::query(&context,&query).await.map_err(ModelError::codec)?.execute_stream().await.map_err(ModelError::codec)?;
        while let Some(batch)=stream.try_next().await.map_err(ModelError::codec)?{
            let width=<$record>::schema().fields().len();
            let typed=batch.project(&(0..width).collect::<Vec<_>>()).map_err(ModelError::codec)?;
            let _decode=workspace.budget().reserve("manifest-vector-binding",decode_allowance::<$record>(&typed)?)?;
            let rows=<$record>::decode(&typed)?;
            let texts=batch.column(width).as_any().downcast_ref::<BinaryArray>().ok_or(ModelError::Schema(<$text>::NAME))?;
            for (index,row) in rows.iter().enumerate(){
                row.validate()?;if texts.is_null(index){return Err(invalid("embedding consumption lacks original text reference"));}
                let text=std::str::from_utf8(texts.value(index)).map_err(ModelError::codec)?;
                if !selected.contains(&row.specification){return Err(invalid("embedding consumption uses an unselected specification"));}
                let specification=specs.get(&row.specification).ok_or_else(||invalid("embedding consumption lacks selected specification"))?;
                let request_bytes=text.len().checked_mul(specification.document_template.matches("{text}").count())
                    .and_then(|n|n.checked_add(specification.document_template.len())).and_then(|n|n.checked_mul(2)).ok_or_else(||invalid("embedding request allocation overflow"))?;
                let _request=workspace.budget().reserve("manifest-vector-request",request_bytes)?;
                let request=specification.document_text(text);
                if row.input!=d::embedding::value::input_hash(&request){return Err(invalid("embedding consumption input differs from exact request text"));}
                if row.availability==d::embedding::analytic::VectorAvailability::TokenLimit && row.admitted_tokens.is_none_or(|tokens|tokens<=i64::from(specification.max_document_tokens)){
                    return Err(invalid("embedding token-limit status does not exceed selected cap"));
                }
                if row.availability==d::embedding::analytic::VectorAvailability::Available{
                    let receipt=row.receipt()?;let _value=d::embedding::value::decode(specification,receipt.bytes,receipt.digest,receipt.admitted_tokens,workspace.budget())?;
                    let value=EmbeddingConsumption {specification:specification.hash(),text:row.input,dimension:specification.dimensions,values:receipt.digest};
                    let key=(value.specification,value.text);
                    if let Some(old)=vectors.get(&key){if old!=&value{return Err(invalid("conflicting consumed vectors for one specification and input"));}}
                    else {charge.grow(size_of::<EmbeddingConsumption>()+64)?;vectors.insert(key,value);}
                }
            }
        }
    }}}
    consume!(
        d::embedding::analytic::AnalysisEmbeddingUse,
        d::embedding::text::TextWindow,
        window
    );
    consume!(
        d::retrieval::consumption::RetrievalEmbeddingUse,
        d::retrieval::Fragment,
        fragment
    );
    Ok(vectors.into_values().collect())
}
