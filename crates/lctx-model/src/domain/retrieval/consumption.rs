//! Retrieval owns its exact vector uses; both consumers replay one immutable winner recipe.
use super::{
    build::{Data, Output, invalid, need},
    *,
};
use crate::Domain;
use crate::domain::{
    analysis::retrieval::{AnalysisInvocation, AnalysisOutcome},
    embedding::{
        DocumentRecipe, EmbeddingSpec,
        analytic::VectorAvailability,
        configuration::ServiceConfiguration,
        consumption::{PublishedValue, SelectedConsumption, ValueIndex},
        projection::{ProjectedValue, ProjectionDefinition},
        value,
    },
    normalized::Rows,
    obligation::ObligationKind,
    resources::ResourceBudget,
};
#[derive(Debug, Clone, PartialEq, Eq, Domain, serde::Serialize, serde::Deserialize)]
#[model(name="retrieval_embedding_uses",validate=validate_use,invariant_refs=invariants_refs)]
pub struct RetrievalEmbeddingUse {
    #[model(key)]
    pub invocation: Id<AnalysisInvocation>,
    #[model(key)]
    pub window: Id<SearchWindow>,
    pub specification: Id<EmbeddingSpec>,
    pub input: ContentHash,
    pub availability: VectorAvailability,
    pub admitted_tokens: Option<i64>,
    pub document: Id<DocumentRecipe>,
    pub value: Option<Id<value::FullValue>>,
    pub projection: Option<Id<ProjectedValue>>,
}
fn validate_use(r: &RetrievalEmbeddingUse) -> Result<(), ModelError> {
    let value = r.value.is_some() && r.projection.is_some();
    let absent = r.value.is_none() && r.projection.is_none();
    let valid = match r.availability {
        VectorAvailability::Available => value && r.admitted_tokens.is_some_and(|n| n >= 0),
        VectorAvailability::ServiceUnavailable => absent && r.admitted_tokens.is_none(),
        VectorAvailability::TokenLimit => absent && r.admitted_tokens.is_some_and(|n| n >= 0),
    };
    if !valid {
        return Err(invalid(
            "retrieval vector availability and canonical references disagree",
        ));
    }
    Ok(())
}
impl RetrievalEmbeddingUse {
    pub fn admit_into(
        rows: &mut Rows<Self>,
        invocation: Id<AnalysisInvocation>,
        window: Id<SearchWindow>,
        selected: SelectedConsumption<'_>,
        value: &PublishedValue,
        b: &ResourceBudget,
    ) -> Result<Id<Self>, ModelError> {
        let expected = Id::of(&value::FullValueKey {
            encoder: selected.encoder.id(),
            input: value.input,
        });
        let projection = Id::of(&embedding::projection::ProjectedValueKey {
            value: expected,
            definition: selected.projection.id(),
        });
        if value.value != expected
            || value.projection != projection
            || value.tokens > 2048
            || i64::from(value.tokens) > selected.document.max_tokens
        {
            return Err(invalid(
                "retrieval value changed selected recipes or window cap",
            ));
        }
        let _copy = b.reserve("retrieval-use-copy", size_of::<Self>())?;
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
/// Availability is retained once per native frame; no window text or vector survives its unit.
#[derive(Default)]
pub struct Disposition {
    reason: Option<ObligationKind>,
}
impl Disposition {
    pub fn observe_availability(&mut self, availability: VectorAvailability) {
        self.reason =
            crate::domain::obligation::first(self.reason.into_iter().chain(match availability {
                VectorAvailability::Available => None,
                VectorAvailability::ServiceUnavailable => {
                    Some(ObligationKind::EmbeddingServiceUnavailable)
                }
                VectorAvailability::TokenLimit => Some(ObligationKind::EmbeddingTokenLimit),
            }));
    }
    pub fn observe(&mut self, uses: &Rows<RetrievalEmbeddingUse>) {
        self.reason = crate::domain::obligation::first(self.reason.into_iter().chain(
            uses.iter().filter_map(|row| match row.availability {
                VectorAvailability::Available => None,
                VectorAvailability::ServiceUnavailable => {
                    Some(ObligationKind::EmbeddingServiceUnavailable)
                }
                VectorAvailability::TokenLimit => Some(ObligationKind::EmbeddingTokenLimit),
            }),
        ));
    }
    pub fn outcome(&self, invocation: &AnalysisInvocation) -> AnalysisOutcome {
        AnalysisOutcome {
            invocation: invocation.id(),
            status: if self.reason.is_some() {
                analysis::AnalysisStatus::Partial
            } else {
                analysis::AnalysisStatus::Completed
            },
            reason: self.reason,
        }
    }
}
/// Necessary vector properties for one actual unit, using the same canonical request/value
/// admission as the independent ordered cross-consumer winner check.
pub fn verify_uses(
    output: &Output,
    invocation: &AnalysisInvocation,
    selected: Option<SelectedConsumption<'_>>,
    uses: &Rows<RetrievalEmbeddingUse>,
    b: &ResourceBudget,
) -> Result<(), ModelError> {
    let Some(selected) = selected else {
        if !uses.is_empty() {
            return Err(invalid("retrieval has unrequested vector uses"));
        }
        return Ok(());
    };
    let specification = selected.encoder;
    let spec = selected.document.configuration(specification)?;
    let mut expected = charged::ChargedSet::default();
    let mut charge = charged::StateCharge::new(b, "retrieval-unit-use-membership");
    for window in output.windows.iter().filter(|window| {
        output.units.iter().any(|unit| {
            unit.id() == window.unit
                && unit.input == invocation.input
                && unit.context == invocation.context
        })
    }) {
        let id = Id::of(&RetrievalEmbeddingUseKey {
            invocation: invocation.id(),
            window: window.id(),
        });
        expected.insert(&mut charge, id)?;
        let row = need(uses, id)?;
        row.validate()?;
        if row.specification != specification.id() || row.document != selected.document.id() {
            return Err(invalid("retrieval use changed selected specification"));
        }
        let bound = window
            .text
            .len()
            .checked_mul(spec.document_template.matches("{text}").count())
            .and_then(|n| n.checked_add(spec.document_template.len()))
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| invalid("retrieval request overflow"))?;
        let _request = b.reserve("retrieval-consumption-request", bound)?;
        if spec.document_text(window.text.as_str()) != window.input_text.as_str() {
            return Err(invalid(
                "retrieval complete encoder input differs from selected processing",
            ));
        }
        if row.input != value::input_hash(&spec.document_text(window.text.as_str())) {
            return Err(invalid("retrieval input differs from completed window"));
        }
        if window.availability == WindowAvailability::LexicalOnly
            && row.availability != VectorAvailability::TokenLimit
        {
            return Err(invalid("oversized semantic window cannot admit a vector"));
        }
        match row.availability {
            VectorAvailability::Available => {
                let expected = Id::of(&value::FullValueKey {
                    encoder: specification.id(),
                    input: row.input,
                });
                let projection = Id::of(&embedding::projection::ProjectedValueKey {
                    value: expected,
                    definition: selected.projection.id(),
                });
                if row.value != Some(expected)
                    || row.projection != Some(projection)
                    || row
                        .admitted_tokens
                        .is_none_or(|n| n > 2048 || n > selected.document.max_tokens)
                    || row.admitted_tokens != window.tokens
                {
                    return Err(invalid(
                        "retrieval canonical reference keys or complete token count differ",
                    ));
                }
            }
            VectorAvailability::TokenLimit
                if row.admitted_tokens.is_none_or(|n| n <= 2048)
                    || row.admitted_tokens != window.tokens =>
            {
                return Err(invalid("retrieval token refusal within selected cap"));
            }
            _ => {}
        }
    }
    if expected.len() != uses.len() {
        return Err(invalid("retrieval use domain missing or unexpected rows"));
    }
    Ok(())
}
/// One necessary E0 admission dispatch, with fixed frames and fresh actual-unit kernels.
#[derive(Debug, Clone, Copy)]
pub enum Scope {
    Consumption,
}

pub struct ConsumptionData {
    pub render: Data,
    pub output: Output,
    pub specifications: Rows<EmbeddingSpec>,
    pub services: Rows<ServiceConfiguration>,
    pub documents: Rows<DocumentRecipe>,
    pub projections: Rows<ProjectionDefinition>,
    pub values: ValueIndex,
    pub sources: Rows<crate::domain::analysis::retrieval::InvocationSource>,
    pub parents: Rows<crate::domain::analysis::retrieval::AnalysisInput>,
}
impl ConsumptionData {
    pub fn new(b: &ResourceBudget) -> Self {
        Self {
            render: Data::new(b),
            output: Output::new(b),
            specifications: Rows::new(b),
            services: Rows::new(b),
            documents: Rows::new(b),
            projections: Rows::new(b),
            values: ValueIndex::new(b),
            sources: Rows::new(b),
            parents: Rows::new(b),
        }
    }
    pub fn inputs() -> Vec<ValidationInput> {
        let mut r = Data::inputs();
        r.extend(Output::inputs());
        r.extend([
            ValidationInput::of::<EmbeddingSpec>(&["id"]),
            ValidationInput::of::<ServiceConfiguration>(&["id"]),
            ValidationInput::of::<DocumentRecipe>(&["id"]),
            ValidationInput::of::<ProjectionDefinition>(&["id"]),
            ValidationInput::of::<value::FullValue>(&["id"])
                .at_epoch(stages::PublicationBoundary::Retrieval),
            ValidationInput::of::<ProjectedValue>(&["id"])
                .at_epoch(stages::PublicationBoundary::Retrieval),
            ValidationInput::of::<crate::domain::analysis::retrieval::InvocationSource>(&["id"]),
            ValidationInput::of::<crate::domain::analysis::retrieval::AnalysisInput>(&["id"]),
        ]);
        r
    }
    pub fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<bool, ModelError> {
        if self.render.visit(n, b)? || self.output.visit(n, b)? {
            return Ok(true);
        }
        macro_rules! row {($($f:ident:$ty:ty),*)=>{$(if n==<$ty>::NAME {self.$f.decode(b)?;return Ok(true);})*};}
        row!(specifications:EmbeddingSpec,services:ServiceConfiguration,documents:DocumentRecipe,projections:ProjectionDefinition,sources:crate::domain::analysis::retrieval::InvocationSource,parents:crate::domain::analysis::retrieval::AnalysisInput);
        if n == value::FullValue::NAME {
            for row in value::FullValue::decode(b)? {
                let encoder = self.selected_spec()?.clone();
                let policy = self.policy()?.clone();
                self.values.admit_full(&row, &encoder, &policy)?;
            }
            return Ok(true);
        }
        if n == ProjectedValue::NAME {
            for row in ProjectedValue::decode(b)? {
                self.values.admit_projection(&row)?;
            }
            return Ok(true);
        }
        Ok(false)
    }
    pub fn visit_input(
        &mut self,
        input: &ValidationInput,
        b: &arrow_array::RecordBatch,
    ) -> Result<bool, ModelError> {
        if crate::domain::stages::is_vocabulary(input.name()) {
            self.render.visit_input(input, b)
        } else {
            self.visit(input.name(), b)
        }
    }
    pub fn selected_spec(&self) -> Result<&EmbeddingSpec, ModelError> {
        if self.specifications.len() != 1 || self.services.len() != 1 {
            return Err(invalid(
                "requested retrieval needs one selected service and specification",
            ));
        }
        let spec = self.specifications.iter().next().unwrap();
        if self.services.iter().next().unwrap().specification != spec.id() {
            return Err(invalid("retrieval service changed selected specification"));
        }
        Ok(spec)
    }
    pub fn document(&self) -> Result<&DocumentRecipe, ModelError> {
        let service = self
            .services
            .iter()
            .next()
            .ok_or_else(|| invalid("retrieval selected service absent"))?;
        need(&self.documents, service.document)
    }
    pub fn policy(&self) -> Result<&ProjectionDefinition, ModelError> {
        let service = self
            .services
            .iter()
            .next()
            .ok_or_else(|| invalid("retrieval selected service absent"))?;
        need(&self.projections, service.projection)
    }
    pub fn selected_consumption(&self) -> Result<SelectedConsumption<'_>, ModelError> {
        Ok(SelectedConsumption {
            encoder: self.selected_spec()?,
            document: self.document()?,
            projection: self.policy()?,
        })
    }
    pub fn verify_canonical_uses(
        &self,
        uses: &Rows<RetrievalEmbeddingUse>,
    ) -> Result<(), ModelError> {
        if !self.render.selected()?.embedding_requested {
            if !uses.is_empty() {
                return Err(invalid("unrequested canonical retrieval uses"));
            }
            return Ok(());
        }
        let selected = self.selected_consumption()?;
        for row in uses.iter() {
            row.validate()?;
            if row.specification != selected.encoder.id() || row.document != selected.document.id()
            {
                return Err(invalid("retrieval canonical use changed recipes"));
            }
            if row.availability == VectorAvailability::Available {
                self.values.verify_use(
                    row.specification,
                    row.input,
                    row.admitted_tokens.unwrap(),
                    row.value.unwrap(),
                    row.projection.unwrap(),
                    selected.projection.id(),
                )?;
            }
        }
        Ok(())
    }
    pub fn owns(&self, i: &AnalysisInvocation, f: &SearchWindow) -> bool {
        self.output
            .units
            .iter()
            .any(|u| u.id() == f.unit && u.input == i.input && u.context == i.context)
    }
    pub fn outcome(
        &self,
        i: &AnalysisInvocation,
        uses: &Rows<RetrievalEmbeddingUse>,
    ) -> Result<AnalysisOutcome, ModelError> {
        let reason = if !self.render.selected()?.embedding_requested {
            None
        } else {
            crate::domain::obligation::first(
                uses.iter()
                    .filter(|r| r.invocation == i.id())
                    .filter_map(|r| match r.availability {
                        VectorAvailability::Available => None,
                        VectorAvailability::ServiceUnavailable => {
                            Some(ObligationKind::EmbeddingServiceUnavailable)
                        }
                        VectorAvailability::TokenLimit => Some(ObligationKind::EmbeddingTokenLimit),
                    }),
            )
        };
        Ok(AnalysisOutcome {
            invocation: i.id(),
            status: if reason.is_some() {
                analysis::AnalysisStatus::Partial
            } else {
                analysis::AnalysisStatus::Completed
            },
            reason,
        })
    }
    /// Exact finite frame and parent metadata, independent of the number of rendered units.
    pub fn verify_frames(
        &self,
        invocations: &Rows<AnalysisInvocation>,
        outcomes: &Rows<AnalysisOutcome>,
        expected_outcomes: &Rows<AnalysisOutcome>,
        b: &ResourceBudget,
    ) -> Result<(), ModelError> {
        let mut charge = charged::StateCharge::new(b, "retrieval-frame-membership");
        let mut frames = charged::ChargedSet::default();
        let mut actual = charged::ChargedSet::default();
        for run in self.render.source.facts.runs.iter() {
            frames.insert(&mut charge, (run.input, run.context))?;
        }
        let mut sources = Rows::new(b);
        let mut parents = Rows::new(b);
        for invocation in invocations.iter() {
            if invocation.definition != build::definition().1.id()
                || invocation.subject.is_some()
                || !frames.contains(&(invocation.input, invocation.context))
                || !actual.insert(&mut charge, (invocation.input, invocation.context))?
            {
                return Err(invalid(
                    "retrieval invocation differs from fixed native frame",
                ));
            }
            let mut ids = [None; 2];
            for (ordinal, source) in self
                .render
                .parents(invocation.input, invocation.context)?
                .into_iter()
                .enumerate()
            {
                let id = sources.insert(source)?;
                ids[ordinal] = Some(id);
                parents.insert(crate::domain::analysis::retrieval::AnalysisInput {
                    invocation: invocation.id(),
                    parent: id,
                })?;
            }
            if invocation.inputs
                != AnalysisInvocation::new(
                    invocation.input,
                    invocation.context,
                    invocation.definition,
                    None,
                    ids.into_iter().flatten(),
                )
                .0
                .inputs
            {
                return Err(invalid(
                    "retrieval parent digest differs from exact S0/C1 frames",
                ));
            }
            let outcome = outcomes
                .iter()
                .find(|outcome| outcome.invocation == invocation.id())
                .ok_or_else(|| invalid("retrieval outcome absent"))?;
            outcome.validate()?;
        }
        if !sources.same(&self.sources)
            || !parents.same(&self.parents)
            || frames.len() != actual.len()
            || outcomes.len() != invocations.len()
            || !outcomes.same(expected_outcomes)
        {
            return Err(invalid(
                "retrieval frame/outcome domain missing or unexpected rows",
            ));
        }
        Ok(())
    }
    pub fn verify(
        &self,
        invocations: &Rows<AnalysisInvocation>,
        outcomes: &Rows<AnalysisOutcome>,
        uses: &Rows<RetrievalEmbeddingUse>,
        b: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.output.matches(&build::build(&self.render, b)?)?;
        self.verify_completion(invocations, outcomes, uses, b)
    }
    /// Completion checks exact frames, parents, outcomes, nominal uses and winning bytes.
    /// Canonical construction is already owned by the renderer; independent equivalence replay
    /// remains available through `verify` for corruption and known-answer controls.
    pub fn verify_completion(
        &self,
        invocations: &Rows<AnalysisInvocation>,
        outcomes: &Rows<AnalysisOutcome>,
        uses: &Rows<RetrievalEmbeddingUse>,
        b: &ResourceBudget,
    ) -> Result<(), ModelError> {
        self.output.verify_completion(&self.render, b)?;
        let selected = self.render.selected()?.embedding_requested;
        let mut charge = charged::StateCharge::new(b, "retrieval-consumption-membership");
        let mut expected = charged::ChargedSet::default();
        let mut frames = charged::ChargedSet::default();
        let mut actual = charged::ChargedSet::default();
        for run in self.render.source.facts.runs.iter() {
            frames.insert(&mut charge, (run.input, run.context))?;
        }
        let mut expected_sources = Rows::new(b);
        let mut expected_parents = Rows::new(b);
        for i in invocations.iter() {
            if i.definition != build::definition().1.id()
                || i.subject.is_some()
                || !frames.contains(&(i.input, i.context))
                || !actual.insert(&mut charge, (i.input, i.context))?
            {
                return Err(invalid(
                    "retrieval invocation differs from fixed native frame",
                ));
            }
            let mut parent_ids = [None; 2];
            for (ordinal, source) in self
                .render
                .parents(i.input, i.context)?
                .into_iter()
                .enumerate()
            {
                let parent = expected_sources.insert(source)?;
                parent_ids[ordinal] = Some(parent);
                expected_parents.insert(crate::domain::analysis::retrieval::AnalysisInput {
                    invocation: i.id(),
                    parent,
                })?;
            }
            if i.inputs
                != AnalysisInvocation::new(
                    i.input,
                    i.context,
                    i.definition,
                    None,
                    parent_ids.into_iter().flatten(),
                )
                .0
                .inputs
            {
                return Err(invalid(
                    "retrieval parent digest differs from exact S0/C1 frames",
                ));
            }
            let outcome = self.outcome(i, uses)?;
            if outcomes.get(outcome.id()) != Some(&outcome) {
                return Err(invalid("retrieval outcome differs from exact availability"));
            }
            if !selected {
                continue;
            }
            let mut frame_uses = Rows::new(b);
            for row in uses.iter().filter(|r| r.invocation == i.id()) {
                frame_uses.insert(row.clone())?;
                expected.insert(&mut charge, row.id())?;
            }
            verify_uses(
                &self.output,
                i,
                Some(self.selected_consumption()?),
                &frame_uses,
                b,
            )?;
        }
        if !expected_sources.same(&self.sources)
            || !expected_parents.same(&self.parents)
            || frames.len() != actual.len()
            || expected.len() != uses.len()
            || outcomes.len() != invocations.len()
        {
            return Err(invalid(
                "retrieval consumption domain missing or unexpected rows",
            ));
        }
        self.verify_canonical_uses(uses)?;
        Ok(())
    }
}
pub fn relations() -> Vec<Relation> {
    vec![Relation::of::<RetrievalEmbeddingUse>()]
}
pub fn invariants() -> Vec<Invariant> {
    let mut inputs = ConsumptionData::inputs();
    inputs.extend([
        ValidationInput::of::<AnalysisInvocation>(&["id"]),
        ValidationInput::of::<AnalysisOutcome>(&["id"]),
        ValidationInput::of::<RetrievalEmbeddingUse>(&["id"]),
    ]);
    vec![Invariant {
        purpose: crate::domain::InvariantPurpose::Admission,
        revision: 5,
        name: "retrieval_embedding_consumption_and_winners",
        inputs,
        create: std::sync::Arc::new(|b| {
            Box::new(Check {
                data: ConsumptionData::new(b),
                invocations: Rows::new(b),
                outcomes: Rows::new(b),
                uses: Rows::new(b),
                budget: b.clone(),
            })
        }),
    }]
}
struct Check {
    data: ConsumptionData,
    invocations: Rows<AnalysisInvocation>,
    outcomes: Rows<AnalysisOutcome>,
    uses: Rows<RetrievalEmbeddingUse>,
    budget: ResourceBudget,
}
impl InvariantCheck for Check {
    fn retrieval_scope(&self) -> Option<Scope> {
        Some(Scope::Consumption)
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if crate::domain::stages::is_vocabulary(input.name()) {
            if self.data.visit_input(input, batch)? {
                Ok(())
            } else {
                Err(invalid("undeclared retrieval vocabulary input"))
            }
        } else {
            self.visit(input.name(), batch)
        }
    }
    fn visit(&mut self, n: &str, b: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        if self.data.visit(n, b)? {
            return Ok(());
        }
        if n == AnalysisInvocation::NAME {
            self.invocations.decode(b)?;
        } else if n == AnalysisOutcome::NAME {
            self.outcomes.decode(b)?;
        } else if n == RetrievalEmbeddingUse::NAME {
            self.uses.decode(b)?;
        } else {
            return Err(invalid("undeclared retrieval consumption input"));
        }
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.data
            .verify_completion(&self.invocations, &self.outcomes, &self.uses, &self.budget)
    }
}

pub(crate) fn invariants_refs() -> Vec<&'static str> {
    vec!["retrieval_embedding_consumption_and_winners"]
}
