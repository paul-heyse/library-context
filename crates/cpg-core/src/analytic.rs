//! A1 owns optional nominal results over stored A0 frames and admitted E1 winning bytes.
use crate::{
    analysis_graphs::PreparedGraphs,
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::{AttemptRuntime, StageSession},
};
use futures::TryStreamExt;
use lctx_model::domain::{
    analysis::{self, analytic as owner},
    analytics::{self as semantic, build, frames},
    stages::*,
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::{collections::BTreeSet, sync::Arc};
async fn load<R: Record>(
    access: &StageAccess<'_, '_>,
    reader: &AttemptSession,
    session: &StageSession,
    data: &mut build::Data,
    admission: &mut analysis::expected::CoverageAdmission<'_>,
    seen: &mut BTreeSet<&'static str>,
) -> Result<(), ModelError> {
    if !access.stage().reads::<R>() || !seen.insert(R::NAME) {
        return Ok(());
    }
    let permit = access.read::<R>()?;
    session.register(&permit, reader.table(&permit).map_err(ModelError::codec)?)?;
    let expected = [
        analysis::settings::AnalyticsConfiguration::NAME,
        input::InputRevision::NAME,
        input::ArtifactUse::NAME,
        source::SourceArtifact::NAME,
        source::CoverageScope::NAME,
        attribution::ProviderCoverage::NAME,
        normalized::coverage::NormalizationComputation::NAME,
        normalized::coverage::NormalizationCoverage::NAME,
    ]
    .contains(&R::NAME);
    let query = session
        .query(&format!("SELECT * FROM \"{}\"", R::NAME))
        .await
        .map_err(ModelError::codec)?;
    let mut stream = query.execute_stream().await.map_err(ModelError::codec)?;
    while let Some(batch) = stream.try_next().await.map_err(ModelError::codec)? {
        data.visit(R::NAME, &batch)?;
        if expected {
            admission.visit(&permit, &batch)?;
        }
    }
    Ok(())
}
pub async fn produce(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    config: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
    graphs: &PreparedGraphs,
) -> Result<(), ModelError> {
    let sources = analysis::sources::CapturedSources::capture(&access, runtime.budget())?;
    let mut admission = analysis::expected::CoverageAdmission::new(&sources, runtime.budget())?;
    let reader = AttemptSession::open(
        config,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let session = runtime.session(&access);
    let mut data = build::Data::new(runtime.budget());
    let mut seen = BTreeSet::new();
    macro_rules! read{($($t:ty),*)=>{$(load::<$t>(&access,&reader,&session,&mut data,&mut admission,&mut seen).await?;)*};}
    macro_rules! inventory{($($f:ident:$t:ty,)*)=>{read!($($t),*);};}
    lctx_model::normalized_binding_inputs!(inventory);
    lctx_model::structural_outputs!(inventory);
    lctx_model::analytic_extra_inputs!(inventory);
    lctx_model::analytic_consumption_inputs!(inventory);
    read!(
        projection::ProjectionSourceAssessment,
        input::InputRevision,
        normalized::coverage::NormalizationComputation,
        normalized::coverage::NormalizationCoverage
    );
    drop(session);
    reader.close().await.map_err(ModelError::codec)?;
    let settings = data.configuration()?.clone();
    let declared = build::stage(access.profile(), &settings, model)?;
    if declared.code != access.stage().code
        || declared.configuration != access.stage().configuration
        || declared.name != access.stage().name
    {
        return Err(ModelError::Invalid(
            "analytic stage differs from selected immutable settings/rules".into(),
        ));
    }
    let mut context = frames::Context::new(runtime.budget());
    let mut receipts = normalized::Rows::new(runtime.budget());
    let mut projections = normalized::Rows::new(runtime.budget());
    let mut results = semantic::Output::new(runtime.budget());
    for sf in data.structural.frames.iter() {
        let parent = data
            .structural_invocations
            .get(sf.invocation)
            .ok_or_else(|| ModelError::Invalid("analytic structural parent absent".into()))?;
        for method in build::METHODS {
            let (parameters, definition) = build::definition(&settings, method)?;
            if data.definitions.get(definition.id()) != Some(&definition)
                || data.parameters.get(parameters.id()) != Some(&parameters)
            {
                return Err(ModelError::Invalid(
                    "analytic canonical definition absent".into(),
                ));
            }
            let parents = frames::parents(&data, sf, method)?;
            let (invocation, inputs, source_rows, projection_rows) = owner::Invocation::admitted(
                parent.input,
                parent.context,
                definition.id(),
                None,
                parents.iter().map(Record::id),
                &sources,
                [analysis::ProjectionDefinition::builtin(
                    projection::ProjectionName::CallableInvocation,
                )
                .id()],
                runtime.budget(),
            )?;
            for p in parents {
                context.sources.insert(p)?;
            }
            for row in inputs {
                context.inputs.insert(row)?;
            }
            for row in source_rows {
                receipts.insert(row)?;
            }
            for row in projection_rows {
                projections.insert(row)?;
            }
            context.invocations.insert(invocation)?;
        }
        let frame = semantic::AnalyticFrame {
            structural: sf.id(),
            configuration: settings.id(),
        };
        let graph = graphs.graph(
            &access,
            runtime.budget(),
            projection::normalization::ProjectionKey {
                input: parent.input,
                context: parent.context,
                name: projection::ProjectionName::CallableInvocation,
            },
        )?;
        results.extend(build::produce(
            &data,
            &frame,
            &context.invocations,
            graph,
            runtime.budget(),
        )?)?;
    }
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    macro_rules! write {
        ($t:ty,$rows:expr) => {{
            output.declare::<$t>()?;
            for row in $rows.iter() {
                output.push(row.clone()).await?;
            }
        }};
    }
    macro_rules! result{($($f:ident:$t:ty,)*)=>{$(write!($t,results.$f);)*};}
    lctx_model::analytic_outputs!(result);
    write!(assertion::AssertionQualification, results.qualifications);
    output.declare::<conditions::Condition>()?;
    output.declare::<conditions::ConditionNode>()?;
    if !results.qualifications.is_empty() {
        let (c, nodes) = conditions::Diagram::always().records();
        output.push(c).await?;
        for n in nodes {
            output.push(n).await?;
        }
    }
    write!(owner::Invocation, context.invocations);
    write!(owner::InvocationSource, context.sources);
    write!(owner::AnalysisInput, context.inputs);
    write!(owner::SourceReceipt, receipts);
    write!(owner::ProjectionInput, projections);
    macro_rules! declare{($($t:ty),*)=>{$(output.declare::<$t>()?;)*};}
    declare!(
        owner::AnalysisOutcome,
        owner::AnalysisCoverage,
        owner::CoverageSource,
        owner::AnalysisCoveragePremise,
        owner::CoverageRequirement,
        owner::CoverageRequiredSource
    );
    for result in results.results.iter() {
        let invocation = context.invocations.get(result.invocation).unwrap();
        let definition = data.definitions.get(invocation.definition).unwrap();
        let outcome = frames::outcome(result);
        let domain = owner::coverage::admit(
            invocation,
            definition,
            build::capability(result.method)?,
            &admission,
            runtime.budget(),
        )?;
        for scope in domain.scopes() {
            let (r, members) = scope.expectation().records()?;
            output.push(r).await?;
            for r in members {
                output.push(r).await?;
            }
            for r in scope.observations() {
                output.push(r.source().clone()).await?;
            }
            let (r, members) = owner::coverage::assess(
                scope.expectation(),
                scope.observations(),
                outcome.status,
                outcome.reason,
                runtime.budget(),
            )?;
            output.push(r).await?;
            for r in members {
                output.push(r).await?;
            }
        }
        output.push(outcome).await?;
    }
    drop(results);
    drop(context);
    drop(data);
    drop(receipts);
    drop(projections);
    drop(admission);
    drop(sources);
    output.finish(ProviderOutcome::Complete).await
}
