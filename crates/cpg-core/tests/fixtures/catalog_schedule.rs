//! Publication schedule for the actual catalog parent producers selected by each control.
use lctx_model::domain::{ValidatedModel, stages::*};
pub fn schedule(model: &ValidatedModel, stages: Vec<Stage>, profile: Profile) -> Schedule {
    let facts = stages
        .iter()
        .filter(|s| {
            !matches!(
                s.name,
                "analyze_local" | "apply_models" | "analyze_summaries"
            ) && s.outputs.iter().any(|r| is_vocabulary(r.name()))
        })
        .map(|s| s.name)
        .collect();
    let mut publications = vec![
        PublicationGroup::new(PublicationBoundary::Facts, facts),
        PublicationGroup::new(PublicationBoundary::Local, vec!["analyze_local"]),
    ];
    for (boundary, name) in [
        (PublicationBoundary::Model, "apply_models"),
        (PublicationBoundary::Summary, "analyze_summaries"),
    ] {
        if stages.iter().any(|stage| stage.name == name) {
            publications.push(PublicationGroup::new(boundary, vec![name]));
        }
    }
    Schedule::build_with_publications(model, stages, &[], profile, publications).unwrap()
}
