//! Single S0 writer; rendering never reconstructs graphs or invents completed parents.
use crate::{
    generation_read::{AttemptSession, ProviderOptions},
    model_runtime::AttemptRuntime,
    synthesis_preparation,
};
use lctx_model::domain::{
    analysis::{self, synthesis::*},
    normalized::Rows,
    stages::*,
    synthesis::{self, production::Data},
    *,
};
use lctx_postgres::{generations::GenerationAttempt, roles::RoleConfig};
use std::sync::Arc;
pub async fn produce(
    access: StageAccess<'_, '_>,
    attempt: &GenerationAttempt,
    roles: &RoleConfig,
    runtime: &AttemptRuntime,
    model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let captured = analysis::sources::CapturedSources::capture(&access, runtime.budget())?;
    let mut admission = analysis::expected::CoverageAdmission::new(&captured, runtime.budget())?;
    let reader = AttemptSession::open(
        roles,
        attempt,
        &access,
        model.clone(),
        ProviderOptions::default(),
    )
    .await
    .map_err(ModelError::codec)?;
    let mut data = Data::new(runtime.budget());
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        Data::consumed_inputs(access.profile()),
        runtime.budget(),
    )?;
    macro_rules! read{($($f:ident:$ty:ty,)*)=>{$(while let Some((_input,permit))=consumed.next::<$ty>(&access)? {
        let session=runtime.session(&access);
        crate::consumed_rows::stream(&permit,&reader,&session,|permit,batch| {
            admission.visit_if_expected(permit,batch)?;
            data.visit(<$ty>::NAME,batch)?;
            Ok(())
        }).await?;
    })*};}
    lctx_model::synthesis_frame_inputs!(read);
    lctx_model::synthesis_documentary_inputs!(read);
    lctx_model::synthesis_automatic_inputs!(read);
    lctx_model::synthesis_observation_inputs!(read);

    lctx_model::synthesis_summary_inputs!(read);
    lctx_model::synthesis_pattern_inputs!(read);
    lctx_model::synthesis_setup_inputs!(read);
    macro_rules! named_read{($($f:ident:$ty:ty,)*)=>{$(if access.stage().inputs.iter().any(|r|r.name()==<$ty>::NAME){read!{$f:$ty,}})*};}
    lctx_model::synthesis_pattern_named_inputs!(named_read);
    lctx_model::synthesis_control_text_inputs!(named_read);
    read! {public:structural::PublicCandidate,}
    lctx_model::expected_domain_inputs!(named_read);
    consumed.finish()?;
    reader.close().await.map_err(ModelError::codec)?;
    let (_, definition) = synthesis::build::definition();
    let settings = data.frames.configuration()?;
    if access.stage().configuration != ContentHash::of(settings.id().bytes()) {
        return Err(ModelError::Invalid(
            "S0 selected configuration differs from completed settings".into(),
        ));
    }
    let parents = synthesis::frames::parents(&data.frames, runtime.budget())?;
    let docs = synthesis::documentary::build(&data.documentary, runtime.budget())?;
    let mut invocations = Rows::new(runtime.budget());
    let mut frames = Rows::new(runtime.budget());
    let mut sources = Rows::new(runtime.budget());
    let mut inputs = Rows::new(runtime.budget());
    let mut receipts = Rows::new(runtime.budget());
    let mut seeds = synthesis::seeds::Output::new(runtime.budget());
    for parent in &parents {
        let mut source_ids = charged::ChargedSet::default();
        let mut charge = charged::StateCharge::new(runtime.budget(), "synthesis-parent-identities");
        for source in &parent.sources {
            source_ids.insert(&mut charge, sources.insert(source.clone())?)?;
        }
        let (invocation, members, source_receipts, projections) = Invocation::admitted(
            parent.input,
            parent.context,
            definition.id(),
            None,
            source_ids.iter().copied(),
            &captured,
            [],
            runtime.budget(),
        )?;
        if !projections.is_empty() {
            return Err(ModelError::Invalid("S0 has no projection parameter".into()));
        }
        for row in members {
            inputs.insert(row)?;
        }
        for row in source_receipts {
            receipts.insert(row)?;
        }
        frames.insert(synthesis::frames::frame(parent, invocation.id()))?;
        let mut selected = synthesis::seeds::configured(
            &data.documentary,
            &data.public,
            &data.frames.structural,
            &data.frames.structural_invocations,
            settings,
            &invocation,
            runtime.budget(),
        )?;
        synthesis::automatic::complete(
            &data.documentary,
            &docs,
            &data.public,
            &data.frames.structural,
            &data.frames.structural_invocations,
            &data.automatic,
            &data.frames.analytic_parents,
            settings,
            &invocation,
            &mut selected,
            runtime.budget(),
        )?;
        macro_rules! extend{($($f:ident),*)=>{$(for row in selected.$f.iter(){seeds.$f.insert(row.clone())?;})*};}
        extend!(plans, automatic, decisions, candidates, sources, selected);
        invocations.insert(invocation)?;
    }
    synthesis::frames::verify(
        &data.frames,
        &frames,
        &invocations,
        &sources,
        &inputs,
        runtime.budget(),
    )?;
    let mut coverages = Rows::new(runtime.budget());
    for invocation in invocations.iter() {
        let admitted = coverage::admit(
            invocation,
            &definition,
            analysis::AnalysisCapability::Synthesis,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            coverages.insert(
                coverage::assess(
                    scope.expectation(),
                    scope.observations(),
                    analysis::AnalysisStatus::Completed,
                    None,
                    runtime.budget(),
                )?
                .0,
            )?;
        }
    }
    let observations = synthesis::observations::build_all(
        &data.observations,
        &data.summary,
        &data.documentary,
        &frames,
        &invocations,
        &coverages,
        runtime.budget(),
    )?;
    let assertions = synthesis::assertions::build_all(
        &data.documentary,
        &docs,
        &data.observations,
        &data.controls,
        &data.summary,
        &data.patterns,
        &data.public,
        &frames,
        &invocations,
        runtime.budget(),
    )?;
    let (facets, _) = synthesis::summary::build(
        &data.summary,
        &data.documentary,
        &frames,
        &invocations,
        runtime.budget(),
    )?;
    let patterns = synthesis::patterns::build(
        &data.patterns,
        &data.documentary,
        &frames,
        &invocations,
        runtime.budget(),
    )?;
    let briefs = synthesis::briefs::build_with_summary(
        &data.documentary,
        &docs,
        &assertions,
        &seeds,
        &data.summary,
        &facets,
        &frames,
        &patterns,
        runtime.budget(),
    )?;
    let mut output = StageOutput::new(
        access,
        attempt,
        model,
        runtime.budget().clone(),
        Default::default(),
    )?;
    synthesis_preparation::publish_documentary(&mut output, &docs).await?;
    macro_rules! write{($($ty:ty=>$rows:expr),*)=>{$(output.declare::<$ty>()?;for row in $rows.iter(){output.push(row.clone()).await?;})*};}
    write!(synthesis::summary::SummaryFacet=>facets,synthesis::frames::Frame=>frames,InvocationSource=>sources,AnalysisInput=>inputs,SourceReceipt=>receipts,
    synthesis::seeds::SeedPlan=>seeds.plans,synthesis::automatic::Decision=>seeds.automatic,synthesis::seeds::ConfiguredSeedDecision=>seeds.decisions,synthesis::seeds::ConfiguredSeedCandidate=>seeds.candidates,synthesis::seeds::SelectedSeedSource=>seeds.sources,synthesis::seeds::SelectedSeed=>seeds.selected,
    synthesis::assertions::ProgrammaticAssertion=>assertions.assertions,synthesis::assertions::AssertionTemplate=>assertions.templates,synthesis::assertions::AssertionSource=>assertions.sources,synthesis::assertions::ProgrammaticAssertionSupport=>assertions.supports,
    synthesis::briefs::Brief=>briefs.briefs,synthesis::briefs::BriefAssertion=>briefs.assertions,synthesis::briefs::BriefSource=>briefs.sources,synthesis::briefs::BriefSummary=>briefs.summary,synthesis::briefs::BriefCodeBoundary=>briefs.code_boundaries,synthesis::briefs::BriefDocument=>briefs.documents,synthesis::briefs::BriefOmission=>briefs.omissions);
    macro_rules! patterns_write{($($f:ident:$t:ty,)*)=>{$(if <$t>::NAME!=assertion::AssertionQualification::NAME{output.declare::<$t>()?;}for row in patterns.$f.iter(){output.push(row.clone()).await?;})*};}
    lctx_model::synthesis_pattern_outputs!(patterns_write);
    macro_rules! observation_write{($($f:ident:$t:ty,)*)=>{$(if ![assertion::AssertionQualification::NAME].contains(&<$t>::NAME){output.declare::<$t>()?;}for row in observations.$f.iter(){output.push(row.clone()).await?;})*};}
    lctx_model::synthesis_observation_outputs!(observation_write);
    macro_rules! declare{($($ty:ty),*)=>{$(output.declare::<$ty>()?;)*};}
    declare!(
        Invocation,
        ProjectionInput,
        AnalysisOutcome,
        AnalysisCoverage,
        CoverageSource,
        AnalysisCoveragePremise,
        CoverageRequirement,
        CoverageRequiredSource
    );
    for invocation in invocations.iter() {
        let admitted = coverage::admit(
            invocation,
            &definition,
            analysis::AnalysisCapability::Synthesis,
            &admission,
            runtime.budget(),
        )?;
        for scope in admitted.scopes() {
            let (requirement, members) = scope.expectation().records()?;
            output.push(requirement).await?;
            for row in members {
                output.push(row).await?;
            }
            for row in scope.observations() {
                output.push(row.source().clone()).await?;
            }
            let (coverage, members) = coverage::assess(
                scope.expectation(),
                scope.observations(),
                analysis::AnalysisStatus::Completed,
                None,
                runtime.budget(),
            )?;
            output.push(coverage).await?;
            for row in members {
                output.push(row).await?;
            }
        }
        output
            .push(AnalysisOutcome {
                invocation: invocation.id(),
                status: analysis::AnalysisStatus::Completed,
                reason: None,
            })
            .await?;
        output.push(invocation.clone()).await?;
    }
    drop(patterns);
    drop(facets);
    drop(observations);
    drop(coverages);
    drop(briefs);
    drop(assertions);
    drop(seeds);
    drop(receipts);
    drop(inputs);
    drop(sources);
    drop(frames);
    drop(invocations);
    drop(docs);
    drop(parents);
    drop(data);
    drop(admission);
    drop(captured);
    output.finish(ProviderOutcome::Complete).await
}
