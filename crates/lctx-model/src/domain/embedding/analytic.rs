//! Nominal analytic vector consumption over the completed original-text owner.
use super::{
    DocumentRecipe, EmbeddingSpec, configuration,
    consumption::{PublishedValue, SelectedConsumption, ValueIndex},
    projection::{ProjectedValue, ProjectionDefinition},
    text,
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
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="analysis_embedding_uses",validate=validate_use,invariant_refs=invariants_refs)]
pub struct AnalysisEmbeddingUse {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub window: Id<TextWindow>,
    pub specification: Id<EmbeddingSpec>,
    pub input: ContentHash,
    pub availability: VectorAvailability,
    pub admitted_tokens: Option<i64>,
    pub document: Id<DocumentRecipe>,
    pub value: Option<Id<value::FullValue>>,
    pub projection: Option<Id<ProjectedValue>>,
}
fn invalid(message: &str) -> ModelError {
    ModelError::Invalid(message.into())
}
fn validate_use(row: &AnalysisEmbeddingUse) -> Result<(), ModelError> {
    let value = row.value.is_some() && row.projection.is_some();
    let no_value = row.value.is_none() && row.projection.is_none();
    let valid = match row.availability {
        VectorAvailability::Available => value && row.admitted_tokens.is_some_and(|v| v >= 0),
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
        selected: SelectedConsumption<'_>,
        value: &PublishedValue,
        budget: &ResourceBudget,
    ) -> Result<Id<Self>, ModelError> {
        let expected = Id::of(&value::FullValueKey {
            encoder: selected.encoder.id(),
            input: value.input,
        });
        let projection = Id::of(&super::projection::ProjectedValueKey {
            value: expected,
            definition: selected.projection.id(),
        });
        if value.value != expected
            || value.projection != projection
            || i64::from(value.tokens) > selected.document.max_tokens
        {
            return Err(invalid(
                "analytic canonical consumption differs from selected recipes",
            ));
        }
        let _copy = budget.reserve("analytic-use-copy", size_of::<Self>())?;
        rows.insert(Self {
            invocation,
            window,
            specification: selected.encoder.id(),
            document: selected.document.id(),
            input: value.input,
            availability: VectorAvailability::Available,
            admitted_tokens: Some(value.tokens.into()),
            value: Some(value.value),
            projection: Some(value.projection),
        })
    }
}
/// The nominal outcome depends on availability facts, not retained text or vector payloads.
/// This accumulator is shared by streamed construction and independent admission.
#[derive(Clone, Copy, Default)]
pub struct FrameOutcome {
    selected: bool,
    reason: Option<ObligationKind>,
}
impl HeapSize for FrameOutcome {
    fn heap_bytes(&self) -> usize {
        0
    }
}
impl FrameOutcome {
    pub fn new(selected: bool) -> Self {
        Self {
            selected,
            reason: None,
        }
    }
    fn observe(&mut self, reason: Option<ObligationKind>) {
        self.reason = obligation::first(self.reason.into_iter().chain(reason));
    }
    pub fn unavailable_text(&mut self) {
        self.observe(Some(ObligationKind::AnalyticTextUnavailable));
    }
    pub fn consumed(&mut self, row: &AnalysisEmbeddingUse) {
        self.observe(match row.availability {
            VectorAvailability::Available => None,
            VectorAvailability::ServiceUnavailable => {
                Some(ObligationKind::EmbeddingServiceUnavailable)
            }
            VectorAvailability::TokenLimit => Some(ObligationKind::EmbeddingTokenLimit),
        });
    }
    pub fn outcome(&self, invocation: Id<AnalysisInvocation>) -> AnalysisOutcome {
        let reason = if self.selected {
            self.reason
        } else {
            Some(ObligationKind::NotRequested)
        };
        AnalysisOutcome {
            invocation,
            status: match reason {
                Some(ObligationKind::NotRequested) => analysis::AnalysisStatus::NotRequested,
                Some(_) => analysis::AnalysisStatus::Partial,
                None => analysis::AnalysisStatus::Completed,
            },
            reason,
        }
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
            documents:$crate::domain::embedding::DocumentRecipe,
            projections:$crate::domain::embedding::projection::ProjectionDefinition,
        }
    };
}
macro_rules! data {($($field:ident:$ty:ty,)*)=>{
    pub struct ConsumptionData {$(pub $field:Rows<$ty>,)* pub values:ValueIndex, pub projected_values:Rows<ProjectedValue>,}
    impl ConsumptionData {
        pub fn new(budget:&ResourceBudget)->Self {Self {$($field:Rows::new(budget),)*values:ValueIndex::new(budget),projected_values:Rows::new(budget),}}
        pub fn visit(&mut self,name:&str,batch:&arrow_array::RecordBatch)->Result<bool,ModelError> {$(if name==<$ty>::NAME {self.$field.decode(batch)?;return Ok(true);})*
            if name==value::FullValue::NAME {for row in value::FullValue::decode(batch)? {let encoder=self.specification()?.clone();let policy=self.policy()?.clone();self.values.admit_full(&row,&encoder,&policy)?;}return Ok(true);}
            if name==ProjectedValue::NAME {for row in ProjectedValue::decode(batch)? {self.values.admit_projection(&row)?;self.projected_values.insert(row)?;}return Ok(true);}
            Ok(false)}
        pub fn stage_inputs()->Vec<stages::RelationUse> {vec![$(stages::RelationUse::completed::<$ty>()),*]}
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
    pub fn document(&self) -> Result<&DocumentRecipe, ModelError> {
        let service = self
            .services
            .iter()
            .next()
            .ok_or_else(|| invalid("missing selected service"))?;
        self.documents
            .get(service.document)
            .ok_or_else(|| invalid("missing selected document recipe"))
    }
    pub fn policy(&self) -> Result<&ProjectionDefinition, ModelError> {
        let service = self
            .services
            .iter()
            .next()
            .ok_or_else(|| invalid("missing selected service"))?;
        self.projections
            .get(service.projection)
            .ok_or_else(|| invalid("missing selected projection policy"))
    }
    pub fn configuration(&self) -> Result<super::Spec, ModelError> {
        self.document()?.configuration(self.specification()?)
    }
    pub fn verify_use(&self, row: &AnalysisEmbeddingUse) -> Result<(), ModelError> {
        if row.specification != self.specification()?.id() || row.document != self.document()?.id()
        {
            return Err(invalid("analytic use changed selected recipes"));
        }
        if row.availability == VectorAvailability::Available {
            self.values.verify_use(
                row.specification,
                row.input,
                row.admitted_tokens
                    .ok_or_else(|| invalid("missing admitted tokens"))?,
                row.value
                    .ok_or_else(|| invalid("missing canonical full reference"))?,
                row.projection
                    .ok_or_else(|| invalid("missing projection reference"))?,
                self.policy()?.id(),
            )?;
            if row
                .admitted_tokens
                .is_none_or(|n| n > self.document().map(|d| d.max_tokens).unwrap_or(0))
            {
                return Err(invalid("analytic full winner exceeds document cap"));
            }
        }
        Ok(())
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
        let mut disposition = FrameOutcome::new(self.selected()?);
        for row in self.assessments.iter().filter(|row| {
            row.input == invocation.input
                && row.context == invocation.context
                && row.availability == TextAvailability::Unavailable
        }) {
            let _ = row;
            disposition.unavailable_text();
        }
        for row in uses.iter().filter(|row| row.invocation == invocation.id()) {
            disposition.consumed(row);
        }
        Ok(disposition.outcome(invocation.id()))
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
            let spec = self.configuration()?;
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
                        self.verify_use(row)?;
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
    // Relation order establishes immutable metadata before payloads. The ordered use stream
    // makes exact first-winner comparison local to one request group, independent of cardinality.
    let inputs = vec![
        ValidationInput::of::<EmbeddingSpec>(&["id"]),
        ValidationInput::of::<configuration::ServiceConfiguration>(&["id"]),
        ValidationInput::of::<DocumentRecipe>(&["id"]),
        ValidationInput::of::<ProjectionDefinition>(&["id"]),
        ValidationInput::of::<value::FullValue>(&["id"])
            .at_epoch(stages::PublicationBoundary::AnalyticEmbedding),
        ValidationInput::of::<ProjectedValue>(&["id"])
            .at_epoch(stages::PublicationBoundary::AnalyticEmbedding),
        ValidationInput::of::<text::TextDefinition>(&["id"]),
        ValidationInput::of::<attribution::ProviderRun>(&["id"]),
        ValidationInput::of::<text::TextAssessment>(&["id"]),
        ValidationInput::of::<AnalysisInvocation>(&["id"]),
        ValidationInput::of::<TextWindow>(&["id"]),
        ValidationInput::of::<AnalysisEmbeddingUse>(&[
            "specification",
            "input",
            "invocation",
            "window",
        ]),
        ValidationInput::of::<AnalysisOutcome>(&["id"]),
    ];
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 2,
        name: "analytic_embedding_consumption",
        inputs,
        create: std::sync::Arc::new(|budget| Box::new(Check::new(budget))),
    }]
}
type Frame = (Id<input::InputRevision>, Id<attribution::AnalysisContext>);
struct Check {
    metadata: ConsumptionData,
    frames: charged::ChargedSet<Frame>,
    unavailable: charged::ChargedSet<Frame>,
    invocations: Rows<AnalysisInvocation>,
    admitted_frames: charged::ChargedSet<Frame>,
    windows: charged::ChargedMap<Id<TextWindow>, (Frame, ContentHash)>,
    consumed: charged::ChargedSet<Id<TextWindow>>,
    dispositions: charged::ChargedMap<Id<AnalysisInvocation>, FrameOutcome>,
    outcomes: Rows<AnalysisOutcome>,
    charge: charged::StateCharge,
    budget: ResourceBudget,
}
impl Check {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            metadata: ConsumptionData::new(budget),
            frames: Default::default(),
            unavailable: Default::default(),
            invocations: Rows::new(budget),
            admitted_frames: Default::default(),
            windows: Default::default(),
            consumed: Default::default(),
            dispositions: Default::default(),
            outcomes: Rows::new(budget),
            charge: charged::StateCharge::new(budget, "analytic-consumption-identities"),
            budget: budget.clone(),
        }
    }
    fn window(&mut self, row: TextWindow) -> Result<(), ModelError> {
        if !self.metadata.selected()? {
            return Ok(());
        }
        let assessment = self
            .metadata
            .assessments
            .get(row.assessment)
            .ok_or_else(|| invalid("analytic window assessment absent"))?;
        let frame = (assessment.input, assessment.context);
        if !self.frames.contains(&frame) {
            return Ok(());
        }
        let spec = self.metadata.configuration()?;
        let bound = row
            .text
            .len()
            .checked_mul(spec.document_template.matches("{text}").count())
            .and_then(|n| n.checked_add(spec.document_template.len()))
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| invalid("analytic request allocation overflow"))?;
        let _request = self.budget.reserve("analytic-admission-request", bound)?;
        let input = value::input_hash(&spec.document_text(row.text.as_str()));
        self.windows
            .insert(&mut self.charge, row.id(), (frame, input))?;
        Ok(())
    }
    fn consumed(&mut self, row: AnalysisEmbeddingUse) -> Result<(), ModelError> {
        row.validate()?;
        if !self.metadata.selected()? {
            return Err(invalid("analytic consumption contains unexpected rows"));
        }
        let invocation = self
            .invocations
            .get(row.invocation)
            .ok_or_else(|| invalid("analytic use invocation absent"))?;
        let (frame, input) = self
            .windows
            .get(&row.window)
            .ok_or_else(|| invalid("expected analytic window consumption absent"))?;
        if *frame != (invocation.input, invocation.context)
            || *input != row.input
            || !self.consumed.insert(&mut self.charge, row.window)?
        {
            return Err(invalid(
                "analytic use changed original window membership/input",
            ));
        }
        let specification = self.metadata.specification()?;
        if row.specification != specification.id() {
            return Err(invalid("analytic use changed the selected specification"));
        }
        self.metadata.verify_use(&row)?;
        let spec = self.metadata.configuration()?;
        if row.availability == VectorAvailability::TokenLimit
            && row
                .admitted_tokens
                .is_none_or(|n| n <= i64::from(spec.max_document_tokens))
        {
            return Err(invalid("analytic token refusal is within the selected cap"));
        }
        self.dispositions
            .update(&mut self.charge, row.invocation, |disposition| {
                disposition.consumed(&row)
            })?;
        Ok(())
    }
}
impl InvariantCheck for Check {
    fn visit(&mut self, name: &str, batch: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        macro_rules! each {($ty:ty,$row:ident,$body:block)=>{if name==<$ty>::NAME {
            let _decode=self.budget.reserve("analytic-admission-decode",decode_allowance::<$ty>(batch)?)?;
            for $row in <$ty>::decode(batch)? $body
            return Ok(());
        }};}
        each!(attribution::ProviderRun, row, {
            self.frames
                .insert(&mut self.charge, (row.input, row.context))?;
        });
        each!(text::TextAssessment, row, {
            if row.availability == TextAvailability::Unavailable {
                self.unavailable
                    .insert(&mut self.charge, (row.input, row.context))?;
            }
            self.metadata.assessments.insert(row)?;
        });
        each!(AnalysisInvocation, row, {
            let frame = (row.input, row.context);
            if row.definition != definition().1.id()
                || row.subject.is_some()
                || !self.frames.contains(&frame)
                || !self.admitted_frames.insert(&mut self.charge, frame)?
            {
                return Err(invalid(
                    "analytic invocation differs from expected source frame",
                ));
            }
            let mut disposition = FrameOutcome::new(self.metadata.selected()?);
            if self.unavailable.contains(&frame) {
                disposition.unavailable_text();
            }
            self.dispositions
                .insert(&mut self.charge, row.id(), disposition)?;
            self.invocations.insert(row)?;
        });
        each!(TextWindow, row, {
            self.window(row)?;
        });
        each!(value::FullValue, row, {
            let encoder = self.metadata.specification()?.clone();
            let policy = self.metadata.policy()?.clone();
            self.metadata.values.admit_full(&row, &encoder, &policy)?;
        });
        each!(ProjectedValue, row, {
            self.metadata.values.admit_projection(&row)?;
        });
        each!(AnalysisEmbeddingUse, row, {
            self.consumed(row)?;
        });
        if name == AnalysisOutcome::NAME {
            self.outcomes.decode(batch)?;
            return Ok(());
        }
        if matches!(
            name,
            EmbeddingSpec::NAME
                | configuration::ServiceConfiguration::NAME
                | text::TextDefinition::NAME
                | DocumentRecipe::NAME
                | ProjectionDefinition::NAME
                | value::FullValue::NAME
                | ProjectedValue::NAME
        ) && self.metadata.visit(name, batch)?
        {
            return Ok(());
        }
        Err(invalid("undeclared analytic consumption input"))
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.metadata.selected()?;
        if self.windows.len() != self.consumed.len()
            || self.frames.len() != self.invocations.len()
            || self.outcomes.len() != self.invocations.len()
        {
            return Err(invalid(
                "analytic consumption contains missing/unexpected rows",
            ));
        }
        for invocation in self.invocations.iter() {
            let outcome = self
                .dispositions
                .get(&invocation.id())
                .ok_or_else(|| invalid("analytic frame disposition absent"))?
                .outcome(invocation.id());
            if self.outcomes.get(outcome.id()) != Some(&outcome) {
                return Err(invalid(
                    "analytic embedding outcome differs from actual text and values",
                ));
            }
        }
        Ok(())
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
pub fn stage(
    profile: stages::Profile,
    requested: bool,
    model: &ValidatedModel,
    order: &stages::PublicationOrder,
) -> Result<stages::Stage, ModelError> {
    use stages::*;
    let text_definition = super::text::TextDefinition {
        requested,
        ..super::text::TextDefinition::builtin()
    };
    let mut inputs = super::text::stage(profile, &text_definition, model, order)?.inputs;
    inputs.extend(ConsumptionData::stage_inputs());
    inputs.extend(
        super::text::relations()
            .iter()
            .map(|r| RelationUse::of_relation(r).completed_input()),
    );
    inputs.extend(
        analysis::preparation::configuration_relations()
            .iter()
            .map(|r| RelationUse::of_relation(r).completed_input()),
    );
    macro_rules! add {($($ty:ty),*)=>{$(inputs.push(RelationUse::completed::<$ty>());)*};}
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
    let mut outputs = vec![
        RelationUse::of::<AnalysisEmbeddingUse>(),
        RelationUse::of::<value::FullValue>(),
        RelationUse::of::<ProjectedValue>(),
    ];
    outputs.extend(
        analysis::analytic_embedding::publication_relations()
            .iter()
            .map(RelationUse::of_relation),
    );
    Ok(Stage {
        captured_binding: None,
name: "analytic_embedding",
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![profile],
        effect: if requested {
            Effect::Embedding
        } else {
            Effect::Pure
        },
        code: ContentHash::of(include_bytes!("analytic.rs")),
        configuration: ContentHash::of(if requested { b"requested" } else { b"disabled" }),
    })
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["analytic_embedding_consumption"]
}
