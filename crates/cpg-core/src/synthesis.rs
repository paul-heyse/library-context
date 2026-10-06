//! Single S0 writer; rendering never reconstructs graphs or invents completed parents.
use crate::{
    synthesis_preparation,
    workspace::{CompletedInputs, ProducerOutput, Workspace},
};
use lctx_model::domain::{
    analysis::{self, synthesis::*},
    normalized::Rows,
    stages::*,
    synthesis::{self, production::Data},
    *,
};
use std::sync::Arc;
// Decoder reachability is separate from the model-owned consumed source inventory.
macro_rules! decoder_inputs {
    ($apply:ident) => {
        lctx_model::synthesis_frame_inputs!($apply);
        lctx_model::synthesis_documentary_inputs!($apply);
        lctx_model::synthesis_automatic_inputs!($apply);
        lctx_model::synthesis_observation_inputs!($apply);
        lctx_model::synthesis_summary_inputs!($apply);
        lctx_model::synthesis_terminal_inputs!($apply);
        lctx_model::synthesis_pattern_inputs!($apply);
        lctx_model::synthesis_setup_inputs!($apply);
        lctx_model::synthesis_pattern_named_inputs!($apply);
        lctx_model::synthesis_control_text_inputs!($apply);
        lctx_model::ownership_scope_inputs!($apply);
        lctx_model::expected_domain_inputs!($apply);
        $apply! {public:structural::PublicCandidate,}
    };
}
pub async fn produce(
    access: CompletedInputs,
    mut output: ProducerOutput,
    runtime: &Workspace,
    _model: &Arc<ValidatedModel>,
) -> Result<(), ModelError> {
    let captured = analysis::sources::CapturedSources::capture(
        access.profile(),
        access.snapshots(),
        runtime.budget(),
    )?;
    let mut admission = analysis::expected::CoverageAdmission::new(&captured, runtime.budget())?;
    let mut data = Data::new(runtime.budget());
    let session = access.session(runtime).await?;
    let mut consumed = crate::consumed_rows::ConsumedInputs::new(
        Data::consumed_inputs(access.profile()),
        runtime.budget(),
    )?;
    macro_rules! read{($($f:ident:$ty:ty,)*)=>{$(while let Some((input,permit))=consumed.next::<$ty>(&access)? {
        crate::consumed_rows::stream_at(&permit,&input,&access,&session,|permit,batch| {
            admission.visit_if_expected(permit,batch)?;
            data.visit_input(&input,batch)?;
            Ok(())
        }).await?;
    })*};}
    decoder_inputs!(read);
    consumed.finish(access.name())?;
    let (_, definition) = synthesis::build::definition();
    let settings = data.frames.configuration()?;
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
        &data.terminal,
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
        &data.observations.qualifications,
        &facets,
        &frames,
        &patterns,
        runtime.budget(),
    )?;
    synthesis_preparation::publish_documentary(&mut output, &docs).await?;
    macro_rules! common_publication {($($record:ident,)*)=>{fn common_type(type_id: std::any::TypeId)->bool {false $(||type_id==std::any::TypeId::of::<analysis::synthesis::$record>())*} $(output.declare::<analysis::synthesis::$record>()?;)*};}
    lctx_model::analysis_publication!(common_publication);
    macro_rules! write{($($ty:ty=>$rows:expr),*)=>{$(if !common_type(std::any::TypeId::of::<$ty>()) {output.declare::<$ty>()?;}for row in $rows.iter(){output.push(row.clone()).await?;})*};}
    write!(synthesis::summary::SummaryFacet=>facets,synthesis::frames::Frame=>frames,InvocationSource=>sources,AnalysisInput=>inputs,SourceReceipt=>receipts,
    synthesis::seeds::SeedPlan=>seeds.plans,synthesis::automatic::Decision=>seeds.automatic,synthesis::seeds::ConfiguredSeedDecision=>seeds.decisions,synthesis::seeds::ConfiguredSeedCandidate=>seeds.candidates,synthesis::seeds::SelectedSeedSource=>seeds.sources,synthesis::seeds::SelectedSeed=>seeds.selected,
    synthesis::assertions::ProgrammaticAssertion=>assertions.assertions,synthesis::assertions::AssertionTemplate=>assertions.templates,synthesis::assertions::AssertionSource=>assertions.sources,synthesis::assertions::ProgrammaticAssertionSupport=>assertions.supports,
    synthesis::briefs::Brief=>briefs.briefs,synthesis::briefs::BriefAssertion=>briefs.assertions,synthesis::briefs::BriefSource=>briefs.sources,synthesis::briefs::BriefSummary=>briefs.summary,synthesis::briefs::BriefCodeBoundary=>briefs.code_boundaries,synthesis::briefs::BriefDocument=>briefs.documents,synthesis::briefs::BriefOmission=>briefs.omissions);
    macro_rules! patterns_write{($($f:ident:$t:ty,)*)=>{$(if <$t>::NAME!=assertion::AssertionQualification::NAME{output.declare::<$t>()?;}for row in patterns.$f.iter(){output.push(row.clone()).await?;})*};}
    lctx_model::synthesis_pattern_outputs!(patterns_write);
    macro_rules! observation_write{($($f:ident:$t:ty,)*)=>{$(if ![assertion::AssertionQualification::NAME].contains(&<$t>::NAME){output.declare::<$t>()?;}for row in observations.$f.iter(){output.push(row.clone()).await?;})*};}
    lctx_model::synthesis_observation_outputs!(observation_write);

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

#[cfg(test)]
mod decoder_tests {
    use super::*;
    #[test]
    fn declared_sources_have_decoder_reachability_in_both_profiles() {
        let mut decoders = std::collections::BTreeSet::new();
        macro_rules! collect {($($field:ident:$ty:ty,)*)=>{$(decoders.insert(std::any::TypeId::of::<$ty>());)*};}
        decoder_inputs!(collect);
        for profile in Profile::ALL {
            crate::consumed_rows::assert_decoder_reachability(
                Data::consumed_inputs(profile),
                &decoders,
            );
        }
    }
}
