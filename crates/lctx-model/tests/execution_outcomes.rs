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

#[test]
fn summary_and_enriched_factories_bind_nominal_catalog_and_independent_limits() {
    use lctx_model::domain::{execution::configuration::*,models::ModelCatalog};
    let catalog=ModelCatalog{source_name:"rules.toml".into(),format:1,content:ContentHash::of(b"models = []\n"),source:"models = []\n".into()};
    let limits=SummaryLimits::default();let(parameters,definition)=summaries(catalog.id(),limits).unwrap();
    assert_eq!(parameters.model_catalog,Some(catalog.id()));assert_eq!(parameters.depth,Some(8));assert_eq!(parameters.proof_steps,Some(64));assert_eq!(definition.parameters,parameters.id());
    let mut alternate=catalog.clone();alternate.source_name="other.toml".into();assert_ne!(summaries(alternate.id(),limits).unwrap().1.id(),definition.id());
    assert_ne!(summaries(catalog.id(),SummaryLimits{depth:0,..limits}).unwrap().1.id(),definition.id());
    assert!(summaries(catalog.id(),SummaryLimits{work:0,..limits}).is_err());assert!(summaries(catalog.id(),SummaryLimits{members:0,..limits}).is_err());assert!(summaries(catalog.id(),SummaryLimits{proof_steps:0,..limits}).is_err());
    let(p,d)=enriched_execution(catalog.id());assert_eq!(p.model_catalog,Some(catalog.id()));assert!(enriched_kernel(&d));assert_ne!(enriched_execution(alternate.id()).1.id(),d.id());
}
