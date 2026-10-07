//! Behavioral terminal facts remain scoped typing claims through the persisted compiler.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
#[tokio::test]
async fn behavioral_terminal_frontiers_retain_scoped_basis_and_unknown_twins() {
    let fixture = catalog_runtime::compile(
        "behavioral_frontiers",
        Profile::Behavioral,
        Frontier::Analysis,
        catalog_runtime::settings("cases"),
        None,
    )
    .await;
    let frontiers:Vec<(String,i64,bool,bool,i16)>=catalog_runtime::query(&fixture, "SELECT spelling.spelling,basis.count,f.effects_unknown,f.exceptions_unknown,f.question FROM conditional_terminal_frontiers f JOIN assertion_qualifications q ON q.id=f.qualification JOIN assumption_sets basis ON basis.id=q.assumptions JOIN entity_refs e ON e.id=f.owner JOIN callable_entities c ON c.id=e.callable_callable JOIN declaration_observations d ON d.declaration=c.source_declaration JOIN syntax_observations spelling ON spelling.occurrence=d.name ORDER BY 1").await;
    eprintln!("frontiers={frontiers:?}");
    type TargetDiagnostic = (
        String,
        i16,
        Option<i16>,
        i16,
        i16,
        Option<i16>,
        Option<i16>,
        Option<i16>,
    );
    let diagnostic:Vec<TargetDiagnostic>=catalog_runtime::query(&fixture, "SELECT spelling.spelling,t.basis,t.reason,a.outcome,a.authority_reason,eff.identity,eff.descriptor,eff.signatures FROM closed_target_assessments t JOIN call_binding_attempts a ON a.id=t.attempt JOIN normalized_call_events event ON event.id=t.event JOIN occurrence_ownership o ON o.id=event.owner JOIN entity_refs e ON e.id=o.entity JOIN callable_entities c ON c.id=e.callable_callable JOIN declaration_observations d ON d.declaration=c.source_declaration JOIN syntax_observations spelling ON spelling.occurrence=d.name LEFT JOIN effective_callable_assessments eff ON eff.id=a.effective ORDER BY 1,2,3").await;
    eprintln!("target_diagnostic={diagnostic:?}");
    let exits:Vec<(bool,i16,i64)>=catalog_runtime::query(&fixture, "SELECT e.exceptional,e.characterization,count(*) FROM native_exit_characterizations e GROUP BY 1,2 ORDER BY 1,2").await;
    eprintln!("exits={exits:?}");
    let expected = std::collections::BTreeSet::from([
        ("declared_use".to_owned(), 1, true, true, 0),
        ("final_use".to_owned(), 3, true, true, 0),
        ("final_method_use".to_owned(), 3, true, true, 0),
    ]);
    assert_eq!(
        frontiers
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        expected
    );
    let contradictory_mro:Vec<i16>=catalog_runtime::query(&fixture, "SELECT ancestry.linearization FROM class_ancestry_observations ancestry JOIN provider_symbols symbol ON symbol.id=ancestry.class WHERE symbol.name='MroGap' AND ancestry.relation=1").await;
    assert_eq!(
        contradictory_mro,
        vec![symbols::Linearization::Prefix as i16],
        "the MRO refusal uses actual contradictory-C3 recovery evidence"
    );
    let conditional_bindings:Vec<(String,i16,i16,i16,bool,bool)>=catalog_runtime::query(&fixture, "SELECT spelling.spelling,a.outcome,a.authority,a.authority_reason,dispatch.open,bindings.unique FROM conditional_terminal_frontiers f JOIN closed_target_assessments t ON t.id=f.target JOIN call_binding_attempts a ON a.id=t.attempt JOIN normalized_dispatch_assessments dispatch ON dispatch.target=t.target AND dispatch.event=t.event JOIN binding_set_members member ON member.attempt=a.id JOIN binding_variant_assessments variant ON variant.id=member.variant JOIN binding_set_assessments bindings ON bindings.id=variant.set JOIN entity_refs e ON e.id=f.owner JOIN callable_entities c ON c.id=e.callable_callable JOIN declaration_observations d ON d.declaration=c.source_declaration JOIN syntax_observations spelling ON spelling.occurrence=d.name WHERE t.basis=1 ORDER BY 1").await;
    assert_eq!(
        conditional_bindings,
        vec![
            ("final_method_use".to_owned(), 0, 0, 10, true, false),
            ("final_use".to_owned(), 0, 0, 10, true, false)
        ],
        "named source binding never promotes open runtime dispatch or binding-set completeness"
    );
    let routing_and_question:Vec<(i16,i16,i16,i16,i64)>=catalog_runtime::query(&fixture, "SELECT original.modality,original.approximation,receiver.modality,question.modality,count(*) FROM closed_target_assessments t JOIN conditional_terminal_frontiers frontier ON frontier.target=t.id JOIN call_target_observations call ON call.id=t.target JOIN assertion_qualifications original ON original.id=t.original_qualification AND original.id=call.qualification JOIN type_observations observation ON observation.id=t.receiver_observation AND observation.role=8 JOIN assertion_qualifications receiver ON receiver.id=t.receiver_qualification AND receiver.id=observation.qualification JOIN assertion_qualifications question ON question.id=t.qualification WHERE t.basis=1 GROUP BY 1,2,3,4").await;
    assert_eq!(
        routing_and_question,
        vec![(1, 0, 0, 0, 2)],
        "original Overrides candidates retain their qualification while exact receiver evidence owns the conditional question"
    );
    let phases:Vec<(i16,i16)>=catalog_runtime::query(&fixture, "SELECT DISTINCT action,phase FROM protocol_action_assessments WHERE action IN (3,4,9) ORDER BY 1,2").await;
    assert_eq!(
        phases,
        vec![(3, 0), (4, 0), (9, 0)],
        "native iterator creation, next and operator actions preserve their call phase"
    );
    let witnesses:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM summary_terminal_witnesses w JOIN summary_claims c ON c.id=w.claim WHERE c.kind=3 AND w.question=0").await;
    assert_eq!(
        witnesses, 3,
        "first Summary consumer preserves the three scoped claims"
    );
    let runtime_negative:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM conditional_terminal_frontiers f JOIN assertion_qualifications q ON q.id=f.qualification JOIN assumption_sets s ON s.id=q.assumptions WHERE s.count=0 OR NOT f.exceptions_unknown OR NOT f.effects_unknown").await;
    assert_eq!(
        runtime_negative, 0,
        "typing frontier never becomes unconditional completion"
    );
    let universe:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM closed_target_assessments t JOIN assumption_universe_supports s ON s.id=t.universe_support JOIN authored_models m ON m.id=s.model JOIN model_catalogs c ON c.id=s.catalog WHERE t.basis=1").await;
    assert!(
        universe > 0,
        "final premise cites the actual late pinned definition"
    );
    assert_eq!(
        exits,
        vec![
            (false, 0, 1),
            (false, 1, 2),
            (false, 2, 2),
            (true, 0, 1),
            (true, 1, 2),
            (true, 2, 2)
        ],
        "declared Literal[True], Literal[False] and None characterize typing only; Any and async remain unknown"
    );
    let suppress: i64 = catalog_runtime::one(
        &fixture,
        "SELECT count(*) FROM native_exit_characterizations WHERE runtime_completion_admitted",
    )
    .await;
    assert_eq!(suppress, 0);
}

#[path = "fixtures/native.rs"]
mod native_fixture;
