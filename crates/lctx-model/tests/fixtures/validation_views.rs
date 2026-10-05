use lctx_model::domain::{
    normalized::Rows, resources::ResourceBudget, stages::*, value::Literal, *,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name="validation_view_probes",invariant_refs=invariants_refs,publication_refs=publications_refs,semantic_source=include_bytes!("validation_views.rs"))]
pub struct Probe {
    #[model(key)]
    pub marker: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, lctx_model::Domain)]
#[model(name="validation_reader_probes",semantic_source=include_bytes!("validation_views.rs"))]
pub struct ReadProbe {
    #[model(key)]
    pub marker: bool,
}
fn inputs() -> Vec<ValidationInput> {
    vec![
        ValidationInput::of::<Literal>(&["id"]).at_epoch(PublicationBoundary::Facts),
        ValidationInput::of::<Literal>(&["id"]),
        ValidationInput::of::<Probe>(&["id"]),
    ]
}
fn invariants() -> Vec<Invariant> {
    vec![Invariant {
        revision: 1,
        name: "frozen_and_current_vocabulary",
        inputs: inputs(),
        create: Arc::new(|budget| Box::new(Check::new(budget))),
    }]
}
fn publications() -> Vec<PublicationInvariant> {
    vec![PublicationInvariant {
        revision: 1,
        name: "published_frozen_and_current_vocabulary",
        inputs: inputs(),
        create: Arc::new(|budget| Box::new(Check::new(budget))),
    }]
}
struct Check {
    early: Rows<Literal>,
    current: Rows<Literal>,
    probes: Rows<Probe>,
}
impl Check {
    fn new(budget: &ResourceBudget) -> Self {
        Self {
            early: Rows::new(budget),
            current: Rows::new(budget),
            probes: Rows::new(budget),
        }
    }
    fn input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        if input.name() == Probe::NAME {
            return self.probes.decode(batch);
        }
        match input.prefix() {
            Some(PublicationBoundary::Facts) => self.early.decode(batch),
            None => self.current.decode(batch),
            _ => Err(ModelError::Invalid("unexpected test frame".into())),
        }
    }
    fn finish_check(&self) -> Result<(), ModelError> {
        if self.probes.len() != 1
            || self.early.len() != 1
            || self.early.get(Literal::None.id()).is_none()
            || self.current.len() != 2
            || self
                .current
                .get(Literal::Bool { value: true }.id())
                .is_none()
        {
            return Err(ModelError::Invalid(
                "earlier and current literal views were conflated".into(),
            ));
        }
        Ok(())
    }
}
impl InvariantCheck for Check {
    fn visit(&mut self, _: &str, _: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Err(ModelError::Invalid(
            "view-aware input dispatch is required".into(),
        ))
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.input(input, batch)
    }
    fn finish(self: Box<Self>) -> Result<(), ModelError> {
        self.finish_check()
    }
}
impl PublicationCheck for Check {
    fn visit(&mut self, _: &str, _: &arrow_array::RecordBatch) -> Result<(), ModelError> {
        Err(ModelError::Invalid(
            "view-aware publication dispatch is required".into(),
        ))
    }
    fn visit_input(
        &mut self,
        input: &ValidationInput,
        batch: &arrow_array::RecordBatch,
    ) -> Result<(), ModelError> {
        self.input(input, batch)
    }
    fn finish(self: Box<Self>, _: &[CompletedRelation], _: Profile) -> Result<(), ModelError> {
        self.finish_check()
    }
}
pub fn stage(name: &'static str, inputs: Vec<RelationUse>, outputs: Vec<RelationUse>) -> Stage {
    Stage {
        name,
        inputs,
        outputs,
        contributes: vec![],
        coverage: vec![],
        profiles: vec![Profile::Catalog],
        effect: Effect::Pure,
        code: ContentHash::of(b"validation-view-control"),
        configuration: ContentHash::of(b"two views"),
    }
}
pub fn schedule(model: &ValidatedModel) -> Schedule {
    Schedule::build_with_publications(
        model,
        vec![
            stage("facts", vec![], vec![RelationUse::of::<Literal>()]),
            stage("later", vec![], vec![RelationUse::of::<Literal>()]),
            stage(
                "probe",
                vec![RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Dispatch)],
                vec![RelationUse::of::<Probe>()],
            ),
            stage(
                "reader",
                vec![
                    RelationUse::stored::<Probe>(),
                    RelationUse::stored::<Literal>().at_epoch(PublicationBoundary::Dispatch),
                ],
                vec![RelationUse::of::<ReadProbe>()],
            ),
        ],
        &[],
        Profile::Catalog,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, vec!["facts"]),
            PublicationGroup::new(PublicationBoundary::Dispatch, vec!["later"]),
        ],
    )
    .unwrap()
}

fn invariants_refs() -> Vec<&'static str> {vec!["frozen_and_current_vocabulary"]}

fn publications_refs() -> Vec<&'static str> {vec!["published_frozen_and_current_vocabulary"]}

pub fn definitions() -> ValidationDefinitions { ValidationDefinitions { invariants: invariants(), publication_checks: publications() } }
