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
        let patterns = self.patterns.visit(n, b)?;
        Ok(f || d || a || p || o || summary || patterns || controls)
    }
    pub fn inputs(profile: Profile) -> Vec<ValidationInput> {
        let mut rows = frames::Data::inputs();
        rows.extend(documentary::Data::validation_inputs());
        rows.extend(automatic::Data::inputs());
        rows.extend(super::observations::Data::inputs());
        rows.extend(super::assertions::ControlData::inputs());
        rows.extend(super::summary::Data::inputs());
        rows.extend(super::patterns::Data::inputs(profile));
        rows.push(ValidationInput::of::<structural::PublicCandidate>(&["id"]));
        rows.sort_by_key(|r| r.name());
        rows.dedup_by_key(|r| r.name());
        rows
    }
}
pub fn stage(
    profile: Profile,
    settings: &AnalyticsConfiguration,
    model: &ValidatedModel,
) -> Result<Stage, ModelError> {
    use std::collections::BTreeSet;
    settings.validate()?;
    let mut outputs = super::relations()
        .iter()
        .map(RelationUse::of_relation)
        .collect::<Vec<_>>();
    macro_rules! outputs{($($ty:ty),*)=>{$(outputs.push(RelationUse::of::<$ty>());)*};}
    outputs!(
        owner::Invocation,
        owner::InvocationSource,
        owner::AnalysisInput,
        owner::ProjectionInput,
        owner::SourceReceipt,
        owner::AnalysisOutcome,
        owner::AnalysisCoverage,
        owner::CoverageSource,
        owner::AnalysisCoveragePremise,
        owner::CoverageRequirement,
        owner::CoverageRequiredSource,
        assertion::AssertionQualification
    );
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
            .relations()
            .iter()
            .find(|r| r.name() == name)
            .ok_or_else(|| ModelError::Invalid(format!("S0 relation absent: {name}")))
    };
    let mut inputs = vec![];
    for row in requested {
        if !own.contains(row.name()) || is_vocabulary(row.name()) {
            inputs.push(RelationUse::of_relation(relation(row.name())?).completed_store());
        }
    }
    let facts = facts_relations()
        .iter()
        .map(Relation::name)
        .collect::<BTreeSet<_>>();
    let mut pending = inputs.iter().map(|r| r.name()).collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        let row = relation(name)?;
        for required in row
            .fields()
            .iter()
            .filter_map(|f| f.target().map(|(_, n)| n))
            .chain(
                row.invariants()
                    .iter()
                    .flat_map(|i| i.inputs.iter().map(ValidationInput::name)),
            )
        {
            if own.contains(required) && !is_vocabulary(required) {
                return Err(ModelError::Invalid(format!(
                    "S0 predecessor requires own output: {required}"
                )));
            }
            if (!facts.contains(required) || is_vocabulary(required))
                && !inputs.iter().any(|r| r.name() == required)
            {
                inputs.push(RelationUse::of_relation(relation(required)?).completed_store());
                pending.push(required);
            }
        }
    }
    for row in &mut inputs {
        if is_vocabulary(row.name()) {
            *row = row.clone().at_epoch(PublicationBoundary::Analytic);
        }
    }
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    Ok(Stage {
        name: "synthesis",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
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
            let stage = stage(profile, &settings, &model).unwrap();
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
