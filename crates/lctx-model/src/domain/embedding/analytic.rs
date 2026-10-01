//! Nominal analytic vector consumption over the completed original-text owner.
use super::{
    EmbeddingSpec,
    consumption::{ValueReceipt, Winners},
    text::{TextAvailability, TextWindow},
    value,
};
use crate::domain::{
    analysis::{
        self,
        analytic_embedding::{AnalysisInvocation, AnalysisOutcome},
    },
    normalized::Rows,
    obligation::ObligationKind,
    resources::ResourceBudget,
    *,
};
use crate::{Domain, DomainCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DomainCode)]
#[repr(i16)]
pub enum VectorAvailability {
    Available = 0,
    ServiceUnavailable = 1,
    TokenLimit = 2,
}
#[derive(Debug, Clone, PartialEq, Eq, Domain)]
#[model(name="analysis_embedding_uses",validate=validate_use,invariants=invariants)]
pub struct AnalysisEmbeddingUse {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub window: Id<TextWindow>,
    pub specification: Id<EmbeddingSpec>,
    pub input: ContentHash,
    pub availability: VectorAvailability,
    pub admitted_tokens: Option<i64>,
    pub codec: Option<i16>,
    pub value_digest: Option<ContentHash>,
    pub bytes: Option<EvidenceBytes>,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_use(row: &AnalysisEmbeddingUse) -> Result<(), ModelError> {
    let value = row.codec.is_some() && row.value_digest.is_some() && row.bytes.is_some();
    let no_value = row.codec.is_none() && row.value_digest.is_none() && row.bytes.is_none();
    let valid = match row.availability {
        VectorAvailability::Available => {
            value
                && row.codec == Some(value::VALUE_CODEC)
                && row.admitted_tokens.is_some_and(|v| v >= 0)
        }
        VectorAvailability::ServiceUnavailable => no_value && row.admitted_tokens.is_none(),
        VectorAvailability::TokenLimit => no_value && row.admitted_tokens.is_some_and(|v| v >= 0),
    };
    if !valid {
        return Err(invalid(
            "analytic embedding availability and canonical value disagree",
        ));
    }
    Ok(())
}
impl AnalysisEmbeddingUse {
    pub fn admit_into(
        rows: &mut Rows<Self>,
        invocation: Id<AnalysisInvocation>,
        window: Id<TextWindow>,
        specification: &EmbeddingSpec,
        value: &value::AdmittedValue,
        budget: &ResourceBudget,
    ) -> Result<Id<Self>, ModelError> {
        if value.spec() != specification.service_hash {
            return Err(invalid("analytic value has another selected specification"));
        }
        let _copy = budget.reserve(
            "analytic-use-copy",
            size_of::<Self>().saturating_add(value.bytes().len()),
        )?;
        rows.insert(Self {
            invocation,
            window,
            specification: specification.id(),
            input: value.input(),
            availability: VectorAvailability::Available,
            admitted_tokens: Some(value.tokens().into()),
            codec: Some(value::VALUE_CODEC),
            value_digest: Some(value.digest()),
            bytes: Some(EvidenceBytes(value.bytes().to_vec())),
        })
    }
    pub fn receipt(&self) -> Result<ValueReceipt<'_>, ModelError> {
        self.validate()?;
        if self.availability != VectorAvailability::Available {
            return Err(invalid("analytic vector is unavailable"));
        }
        Ok(ValueReceipt {
            input: self.input,
            codec: self.codec.expect("validated codec"),
            digest: self.value_digest.expect("validated digest"),
            admitted_tokens: u32::try_from(self.admitted_tokens.expect("validated token count"))
                .map_err(ModelError::codec)?,
            bytes: &self.bytes.as_ref().expect("validated bytes").0,
        })
    }
}
#[macro_export]
macro_rules! analytic_consumption_inputs {
    ($m:ident) => {
        $m! {
            runs:$crate::domain::attribution::ProviderRun,
            text_definitions:$crate::domain::embedding::text::TextDefinition,
            assessments:$crate::domain::embedding::text::TextAssessment,
            windows:$crate::domain::embedding::text::TextWindow,
            specifications:$crate::domain::embedding::EmbeddingSpec,
            services:$crate::domain::embedding::configuration::ServiceConfiguration,
        }
    };
}
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
    pub struct ConsumptionData {$(pub $field:Rows<$ty>,)*}
    impl ConsumptionData {
        pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*Ok(false)}
        fn inputs()->Vec<ValidationInput> {vec![$(ValidationInput::of::<$ty>(&["id"]),)*]}
        pub fn stage_inputs()->Vec<stages::RelationUse> {vec![$(stages::RelationUse::stored::<$ty>()),*]}
    }
};}
crate::analytic_consumption_inputs!(data);
impl ConsumptionData {
    pub fn selected(&self) -> Result<bool, ModelError> {
        if self.text_definitions.len() != 1 {
            return Err(invalid(
                "analytic embedding needs one immutable text definition",
            ));
        }
        Ok(self
            .text_definitions
            .iter()
            .next()
            .expect("one definition")
            .requested)
    }
    pub fn specification(&self) -> Result<&EmbeddingSpec, ModelError> {
        if self.services.len() != 1 || self.specifications.len() != 1 {
            return Err(invalid(
                "requested analytic embedding needs one completed selected service",
            ));
        }
        let specification = self
            .specifications
            .iter()
            .next()
            .expect("one specification");
        if self
            .services
            .iter()
            .next()
            .expect("one service")
            .specification
            != specification.id()
        {
            return Err(invalid(
                "analytic embedding service has another specification",
            ));
        }
        Ok(specification)
    }
    pub fn owns(
        &self,
        invocation: &AnalysisInvocation,
        window: &TextWindow,
    ) -> Result<bool, ModelError> {
        let assessment = self
            .assessments
            .get(window.assessment)
            .ok_or_else(|| invalid("analytic window assessment absent"))?;
        Ok(assessment.input == invocation.input && assessment.context == invocation.context)
    }
    pub fn outcome(
        &self,
        invocation: &AnalysisInvocation,
        uses: &Rows<AnalysisEmbeddingUse>,
    ) -> Result<AnalysisOutcome, ModelError> {
        let reason = if !self.selected()? {
            Some(ObligationKind::NotRequested)
        } else {
            obligation::first(
                self.assessments
                    .iter()
                    .filter(|r| {
                        r.input == invocation.input
                            && r.context == invocation.context
                            && r.availability == TextAvailability::Unavailable
                    })
                    .map(|_| ObligationKind::AnalyticTextUnavailable)
                    .chain(
                        uses.iter()
                            .filter(|r| r.invocation == invocation.id())
                            .filter_map(|r| match r.availability {
                                VectorAvailability::Available => None,
                                VectorAvailability::ServiceUnavailable => {
                                    Some(ObligationKind::EmbeddingServiceUnavailable)
                                }
                                VectorAvailability::TokenLimit => {
                                    Some(ObligationKind::EmbeddingTokenLimit)
                                }
                            }),
                    ),
            )
        };
        Ok(AnalysisOutcome {
            invocation: invocation.id(),
            status: match reason {
                Some(ObligationKind::NotRequested) => analysis::AnalysisStatus::NotRequested,
                Some(_) => analysis::AnalysisStatus::Partial,
                None => analysis::AnalysisStatus::Completed,
            },
            reason,
        })
    }
    pub fn validate(
        &self,
        invocations: &Rows<AnalysisInvocation>,
        outcomes: &Rows<AnalysisOutcome>,
        uses: &Rows<AnalysisEmbeddingUse>,
        budget: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let selected = self.selected()?;
        let mut expected = charged::ChargedSet::default();
        let mut charge = charged::StateCharge::new(budget, "analytic-consumption-membership");
        let mut winners = Winners::new(budget);
        let mut frames = charged::ChargedSet::default();
        let mut actual_frames = charged::ChargedSet::default();
        for run in self.runs.iter() {
            frames.insert(&mut charge, (run.input, run.context))?;
        }
        for invocation in invocations.iter() {
            if invocation.definition != definition().1.id()
                || invocation.subject.is_some()
                || !frames.contains(&(invocation.input, invocation.context))
                || !actual_frames.insert(&mut charge, (invocation.input, invocation.context))?
            {
                return Err(invalid(
                    "analytic invocation differs from expected source frame",
                ));
            }
            let outcome = self.outcome(invocation, uses)?;
            if outcomes.get(outcome.id()) != Some(&outcome) {
                return Err(invalid(
                    "analytic embedding outcome differs from actual text and values",
                ));
            }
            if !selected {
                continue;
            }
            let specification = self.specification()?;
            let spec = specification.configuration()?;
            for window in self.windows.iter() {
                if !self.owns(invocation, window)? {
                    continue;
                }
                let key = AnalysisEmbeddingUseKey {
                    invocation: invocation.id(),
                    window: window.id(),
                };
                let id = Id::of(&key);
                expected.insert(&mut charge, id)?;
                let row = uses
                    .get(id)
                    .ok_or_else(|| invalid("expected analytic window consumption absent"))?;
                row.validate()?;
                if row.specification != specification.id() {
                    return Err(invalid("analytic use changed the selected specification"));
                }
                let bound = window
                    .text
                    .len()
                    .checked_mul(spec.document_template.matches("{text}").count())
                    .and_then(|n| n.checked_add(spec.document_template.len()))
                    .and_then(|n| n.checked_mul(2))
                    .ok_or_else(|| invalid("analytic replay request allocation overflow"))?;
                let _request = budget.reserve("analytic-consumption-request", bound)?;
                if row.input != value::input_hash(&spec.document_text(window.text.as_str())) {
                    return Err(invalid("analytic use changed original window input"));
                }
                match row.availability {
                    VectorAvailability::Available => {
                        drop(winners.replay(&spec, window.text.as_str(), row.receipt()?)?);
                    }
                    VectorAvailability::TokenLimit
                        if row
                            .admitted_tokens
                            .is_none_or(|n| n <= i64::from(spec.max_document_tokens)) =>
                    {
                        return Err(invalid("analytic token refusal is within the selected cap"));
                    }
                    _ => {}
                }
            }
        }
        if expected.len() != uses.len()
            || outcomes.len() != invocations.len()
            || frames.len() != actual_frames.len()
        {
            return Err(invalid("analytic consumption contains unexpected rows"));
        }
        Ok(())
    }
}
pub fn relations() -> Vec<Relation> {
    vec![Relation::of::<AnalysisEmbeddingUse>()]
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = ConsumptionData::inputs();
    inputs.extend([
        ValidationInput::of::<AnalysisInvocation>(&["id"]),
        ValidationInput::of::<AnalysisOutcome>(&["id"]),
        ValidationInput::of::<AnalysisEmbeddingUse>(&["id"]),
    ]);
    vec![Invariant {
        name: "analytic_embedding_consumption",
        inputs,
        create: std::sync::Arc::new(|budget| {
            Box::new(Check {
                data: ConsumptionData::new(budget),
                invocations: Rows::new(budget),
                outcomes: Rows::new(budget),
                uses: Rows::new(budget),
                budget: budget.clone(),
            })
        }),
    }]
}
struct Check {
    data: ConsumptionData,
    invocations: Rows<AnalysisInvocation>,
    outcomes: Rows<AnalysisOutcome>,
    uses: Rows<AnalysisEmbeddingUse>,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if name == AnalysisInvocation::NAME {
            self.invocations.decode(batch)?;
        } else if name == AnalysisOutcome::NAME {
            self.outcomes.decode(batch)?;
        } else if name == AnalysisEmbeddingUse::NAME {
            self.uses.decode(batch)?;
        } else if !self.data.visit(name, batch)? {
            return Err(invalid("undeclared analytic consumption input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.data
            .validate(&self.invocations, &self.outcomes, &self.uses, &self.budget)
    }
}

/// Early typed selection is independent of catalog, synthesis and later retrieval.
pub fn definition() -> (analysis::MethodParameters, analysis::AnalysisDefinition) {
    let parameters = analysis::MethodParameters {
        depth: None,
        proof_steps: None,
        work: None,
        members: None,
        seed: None,
        iterations: None,
        threshold: None,
        resolution: None,
        damping: None,
        model_catalog: None,
    };
    let definition = analysis::AnalysisDefinition {
        method: analysis::AnalysisMethod::AnalyticEmbedding,
        semantic_version: ContentHash::of(b"analytic-embedding/v1"),
        parameters: parameters.id(),
        interpretation: analysis::Interpretation::Heuristic,
    };
    (parameters, definition)
}
pub fn stage(profile: stages::Profile, requested: bool) -> stages::Stage {
    use analysis::analytic_embedding::*;
    use stages::*;
    let text_definition = super::text::TextDefinition {
        requested,
        ..super::text::TextDefinition::builtin()
    };
    let mut inputs = super::text::stage(profile, &text_definition)
        .expect("built-in text definition")
        .inputs;
    inputs.extend(ConsumptionData::stage_inputs());
    inputs.extend(
        super::text::relations()
            .iter()
            .map(|r| RelationUse::of_relation(r).completed_store()),
    );
    inputs.extend(
        analysis::preparation::configuration_relations()
            .iter()
            .map(|r| RelationUse::of_relation(r).completed_store()),
    );
    macro_rules! add {($($ty:ty),*)=>{$(inputs.push(RelationUse::stored::<$ty>());)*};}
    add!(
        analysis::AnalysisDefinition,
        analysis::MethodParameters,
        input::InputRevision,
        input::ArtifactUse,
        source::SourceArtifact,
        source::CoverageScope,
        attribution::ProviderRun,
        attribution::ProviderCoverage,
        normalized::coverage::NormalizationComputation,
        normalized::coverage::NormalizationCoverage
    );
    inputs.sort_by_key(|r| r.name());
    inputs.dedup_by_key(|r| r.name());
    let mut outputs = vec![RelationUse::of::<AnalysisEmbeddingUse>()];
    macro_rules! output {($($ty:ty),*)=>{$(outputs.push(RelationUse::of::<$ty>());)*};}
    output!(
        Invocation,
        InvocationSource,
        AnalysisInput,
        ProjectionInput,
        SourceReceipt,
        AnalysisOutcome,
        AnalysisCoverage,
        CoverageSource,
        AnalysisCoveragePremise,
        CoverageRequirement,
        CoverageRequiredSource
    );
    Stage {
        name: "analytic_embedding",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        provider: None,
        profiles: vec![profile],
        effect: if requested {
            Effect::Embedding
        } else {
            Effect::Pure
        },
        code: ContentHash::of(include_bytes!("analytic.rs")),
        configuration: ContentHash::of(if requested { b"requested" } else { b"disabled" }),
    }
}
