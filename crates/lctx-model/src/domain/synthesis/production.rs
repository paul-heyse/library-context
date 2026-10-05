//! Inputs of the single S0 producer. Every frame requires real completed nominal parents.
use super::{automatic, documentary, frames};
use crate::domain::{
    analysis::{self, settings::AnalyticsConfiguration, synthesis as owner},
    normalized::Rows,
    resources::ResourceBudget,
    stages::*,
    *,
};
pub struct Data {
    pub frames: frames::Data,
    pub documentary: documentary::Data,
    pub automatic: automatic::Data,
    pub observations: super::observations::Data,
    pub controls: super::assertions::ControlData,
    pub summary: super::summary::Data,
    pub terminal: super::terminal::Data,
    pub patterns: super::patterns::Data,
    pub public: Rows<structural::PublicCandidate>,
}
impl Data {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            frames: frames::Data::new(b),
            documentary: documentary::Data::new(b),
            automatic: automatic::Data::new(b),
            observations: super::observations::Data::new(b),
            controls: super::assertions::ControlData::new(b),
            summary: super::summary::Data::new(b),
            terminal: super::terminal::Data::new(b),
            patterns: super::patterns::Data::new(b),
            public: Rows::new(b),
        }
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        let f = self.frames.visit(n, b)?;
        let d = self.documentary.visit(n, b)?;
        let a = self.automatic.visit(n, b)?;
        let p = if n == structural::PublicCandidate::NAME {
            self.public.decode(b)?;
            true
        } else {
            false
        };
        let o = self.observations.visit(n, b)?;
        let controls = self.controls.visit(n, b)?;
        let summary = self.summary.visit(n, b)?;
        let terminal = self.terminal.visit(n, b)?;
        let patterns = self.patterns.visit(n, b)?;
        Ok(f || d || a || p || o || summary || terminal || patterns || controls)
    }
    /// Native source checks and retained conclusions have distinct completed vocabulary owners.
    pub fn visit_input(&mut self,input:&ValidationInput,b:&arrow_array::RecordBatch)->Result<bool,ModelError>{
        let n=input.name();
        if !is_vocabulary(n){return self.visit(n,b);}
        match input.prefix(){
            Some(PublicationBoundary::Facts)=>{let docs=self.documentary.visit(n,b)?;let patterns=self.patterns.visit(n,b)?;Ok(docs||patterns)},
            Some(PublicationBoundary::Analytic)=>{let observations=self.observations.visit(n,b)?;let terminal=self.terminal.visit(n,b)?;Ok(observations||terminal)},
            _=>Err(ModelError::Invalid(format!("synthesis input {n} changes its completed vocabulary view")))
        }
    }
    pub fn consumed_inputs(profile: Profile) -> Vec<ValidationInput> {
        let mut inputs = Self::inputs(profile);
        inputs.extend(analysis::expected::inputs(
            analysis::AnalysisMethod::Synthesis,
        ));
        inputs
    }
    pub fn inputs(profile: Profile) -> Vec<ValidationInput> {
        let mut rows = frames::Data::inputs();
        rows.extend(documentary::Data::validation_inputs().into_iter().map(|i|if is_vocabulary(i.name()){i.at_epoch(PublicationBoundary::Facts)}else{i}));
        rows.extend(automatic::Data::inputs());
        rows.extend(frames::AnalyticParents::inputs());
        rows.extend(super::observations::Data::inputs().into_iter().map(|i|if is_vocabulary(i.name()){i.at_epoch(PublicationBoundary::Analytic)}else{i}));
        rows.extend(
            super::assertions::ControlData::inputs()
                .into_iter()
                .filter(|input| {
                    profile == Profile::Behavioral
                        || ![
                            flow::FlowTestLeafObservation::NAME,
                            flow::FlowRegionObservation::NAME,
                        ]
                        .contains(&input.name())
                }),
        );
        rows.extend(super::summary::Data::inputs());
        rows.extend(super::terminal::Data::inputs().into_iter().map(|i|if is_vocabulary(i.name()){i.at_epoch(PublicationBoundary::Analytic)}else{i}));
        rows.extend(super::patterns::Data::inputs(profile).into_iter().map(|i|if is_vocabulary(i.name()){i.at_epoch(PublicationBoundary::Facts)}else{i}));
        rows.push(ValidationInput::of::<structural::PublicCandidate>(&["id"]));
        rows.sort_by_key(|r| (r.name(), r.prefix(), r.order().to_vec()));
        rows.dedup_by(|a, b| {
            a.name() == b.name() && a.prefix() == b.prefix() && a.order() == b.order()
        });
        rows
    }
}
pub fn stage(
    profile: Profile,
    settings: &AnalyticsConfiguration,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<Stage, ModelError> {
    use std::collections::BTreeSet;
    settings.validate()?;
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    macro_rules! outputs{($($ty:ty),*)=>{$(outputs.push(RelationUse::of::<$ty>());)*};}
    outputs.extend(
        owner::publication_relations()
            .iter()
            .map(RelationUse::of_relation),
    );
    outputs!(assertion::AssertionQualification);
    macro_rules! observation_outputs{($($f:ident:$t:ty,)*)=>{$(outputs.push(RelationUse::of::<$t>());)*};}
    crate::synthesis_observation_outputs!(observation_outputs);
    outputs.sort_by_key(|r| r.name());
    outputs.dedup_by_key(|r| r.name());
    let own = outputs.iter().map(|r| r.name()).collect::<BTreeSet<_>>();
    let mut requested = Data::inputs(profile);
    requested.extend(analysis::expected::inputs(
        analysis::AnalysisMethod::Synthesis,
    ));
    let relation = |name| {
        model
            .relation(name)
            .ok_or_else(|| ModelError::Invalid(format!("S0 relation absent: {name}")))
    };
    let mut inputs = vec![];
    for row in requested {
        if !own.contains(row.name()) || is_vocabulary(row.name()) {
            let mut use_ = RelationUse::of_relation(relation(row.name())?).completed_store();
            if let Some(epoch) = row.prefix() {
                use_ = use_.at_epoch(epoch);
            }
            inputs.push(use_);
        }
    }
    let roots = dependency_closure::DependencyClosure::roots_from_uses(model, &inputs)?;
    let inputs = dependency_closure::DependencyClosure::grants(
        model,
        roots,
        inputs,
        &outputs,
        PublicationBoundary::Analytic,
        dependency_closure::LowerLayerPolicy::OmitInferredOrdinaryFacts,
        order,
    )?;
    Ok(Stage {
        name: "synthesis",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: Effect::Pure,
        code: super::build::definition().1.semantic_version,
        configuration: ContentHash::of(settings.id().bytes()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn final_stage_declares_real_parent_closure_and_one_vocabulary_epoch() {
        let model = crate::domain::model().unwrap();
        let settings = super::super::seeds::tests::settings(vec![], 0);
        for profile in [Profile::Catalog, Profile::Behavioral] {
            let stage = stage(
                profile,
                &settings,
                &model,
                &crate::domain::stages::PublicationOrder::registered(
                    crate::domain::ContentHash::of(b"fixture publication order"),
                    &[
                        (0, crate::domain::stages::PublicationBoundary::Facts),
                        (1, crate::domain::stages::PublicationBoundary::Local),
                        (2, crate::domain::stages::PublicationBoundary::Model),
                        (3, crate::domain::stages::PublicationBoundary::Summary),
                        (4, crate::domain::stages::PublicationBoundary::Structural),
                        (5, crate::domain::stages::PublicationBoundary::Analytic),
                        (6, crate::domain::stages::PublicationBoundary::Synthesis),
                    ],
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(stage.name, "synthesis");
            assert!(
                stage
                    .inputs
                    .iter()
                    .any(|r| r.name() == analysis::summary::AnalysisOutcome::NAME)
            );
            assert!(
                stage
                    .inputs
                    .iter()
                    .any(|r| r.name() == analytics::TechniqueResult::NAME)
            );
            assert!(
                stage
                    .inputs
                    .iter()
                    .any(|r| r.name() == catalog::evidence::DocumentAssociation::NAME)
            );
            assert!(
                stage
                    .outputs
                    .iter()
                    .any(|r| r.name() == super::super::briefs::Brief::NAME)
            );
            assert!(
                stage
                    .outputs
                    .iter()
                    .any(|r| r.name() == owner::SourceReceipt::NAME)
            );
        }
    }
}
