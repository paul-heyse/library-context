use lctx_model::domain::{execution::{outcome::*, ExactRuntimeException}, source::{Occurrence, OccurrenceRole, SourceArtifact, SyntaxKind}, input::InputRevision, ContentHash, Record, obligation::ObligationKind};

fn site(n: i64) -> lctx_model::domain::Id<Occurrence> {
    let input = InputRevision::from_entries(vec![lctx_model::domain::input::ManifestEntry {path:"outcomes.py".into(),content:ContentHash::of(b"pass\n"),byte_len:5}]).unwrap();
    let source = SourceArtifact::from_bytes(input.id(),"outcomes.py".into(),b"pass\n").unwrap();
    Occurrence {source:source.id(),start:n,end:n+1,syntax_kind:SyntaxKind::StmtPass,role:OccurrenceRole::Syntax,structural_path:vec![n as i32]}.id()
}

#[test]
fn finalizer_normal_retains_each_pending_outcome_and_abrupt_replaces_it() {
    let outcomes = [PendingOutcome::Normal, PendingOutcome::Return{site:site(1)}, PendingOutcome::Raise{site:site(2),exception:ExactRuntimeException::TypeError}, PendingOutcome::Break{site:site(3)}, PendingOutcome::Continue{site:site(4)}];
    for pending in outcomes { for finalizer in outcomes { assert_eq!(pending.after_finalizer(finalizer),if finalizer.is_normal(){pending}else{finalizer}); } }
}

#[test]
fn suite_stops_before_unentered_refusal_and_never_turns_entered_refusal_normal() {
    assert_eq!(ordered_suite([Ok(PendingOutcome::Normal),Ok(PendingOutcome::Return{site:site(1)}),Err(ObligationKind::MissingEvidence)]),Ok(PendingOutcome::Return{site:site(1)}));
    assert_eq!(ordered_suite([Ok(PendingOutcome::Normal),Err(ObligationKind::CompletionWorkLimit),Ok(PendingOutcome::Return{site:site(1)})]),Err(ObligationKind::CompletionWorkLimit));
}

#[test]
fn handler_cleanup_is_a_separate_required_premise_even_after_return() {
    let returning = PendingOutcome::Return{site:site(1)};
    assert_eq!(after_handler(Ok(returning),Err(ObligationKind::HandlerNameCleanup)),Err(ObligationKind::HandlerNameCleanup));
    assert_eq!(after_handler(Ok(returning),Ok(())),Ok(returning));
}
