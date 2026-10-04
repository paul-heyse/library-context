use lctx_model::domain::{self, Record, execution::{protocol_interpretation::{ConditionalTerminalFrontier, NormalContinuationRestriction}, summary_terminal::SummaryTerminalWitness}};

#[test]
fn cumulative_catalog_model_declares_the_terminal_summary_question_chain() {
    // Validate the production cumulative owner, rather than a test's hand-assembled subset.
    let model = domain::model().unwrap();
    for name in [ConditionalTerminalFrontier::NAME, NormalContinuationRestriction::NAME, SummaryTerminalWitness::NAME] {
        assert!(model.relations().iter().any(|relation| relation.name() == name), "terminal question owner absent: {name}");
    }
}
