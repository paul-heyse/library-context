//! Facts/Local publication schedule used by catalog evidence, selection and retrieval controls.
use lctx_model::domain::{ValidatedModel, stages::*};
pub fn schedule(model: &ValidatedModel, stages: Vec<Stage>, profile: Profile) -> Schedule {
    let facts = stages
        .iter()
        .filter(|s| s.name != "analyze_local" && s.outputs.iter().any(|r| is_vocabulary(r.name())))
        .map(|s| s.name)
        .collect();
    Schedule::build_with_publications(
        model,
        stages,
        &[],
        profile,
        vec![
            PublicationGroup::new(PublicationBoundary::Facts, facts),
            PublicationGroup::new(PublicationBoundary::Local, vec!["analyze_local"]),
        ],
    )
    .unwrap()
}
