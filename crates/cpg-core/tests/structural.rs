//! Structural graph kernels consume actual completed native and normalized compiler inputs.
#[path = "fixtures/catalog_runtime.rs"]
mod catalog_runtime;
use lctx_model::domain::{admission::Frontier, stages::Profile, *};
async fn run(profile: Profile) {
    let mut settings = catalog_runtime::settings("api");
    settings.configured_seeds = vec!["api.Client".into(),"api.missing".into()];
    let fixture = catalog_runtime::compile("structural_usage",profile,Frontier::Catalog,settings,None).await;
    let frames: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_frames").await;
    assert!(frames > 0);
    let invocations: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_analysis_invocations").await;
    assert_eq!(invocations, frames * 4);
    let public: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_public_candidates").await;
    assert!(public > 0);
    let missing:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_configured_seeds WHERE path='api.missing' AND candidates=0").await;
    assert_eq!(missing, frames);
    let handoffs: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_handoff_occurrences").await;
    assert!(
        handoffs > 0,
        "actual official nested result handoff must survive"
    );
    let named:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_handoff_occurrences h JOIN structural_handoff_values v ON v.id=h.value WHERE v.kind=1").await;
    if profile == Profile::Behavioral {
        assert!(
            named > 0,
            "actual reaching-definition result handoff must survive"
        );
        let region_lineage: i64 = catalog_runtime::one_with(&fixture, "SELECT count(*) FROM structural_handoff_occurrences h JOIN structural_handoff_values v ON v.id=h.value JOIN flow_region_observations r ON r.id=v.named_region JOIN flow_region_supports rs ON rs.id=v.named_region_support AND rs.assertion=r.id JOIN flow_use_supports us ON us.id=v.named_support AND us.run=rs.run JOIN assertion_qualifications q ON q.id=r.qualification WHERE v.kind=1 AND q.condition<>$1", vec![datafusion::common::ScalarValue::FixedSizeBinary(16,Some(conditions::Diagram::always().id().bytes().to_vec()))]).await;
        assert!(
            region_lineage > 0,
            "named call results retain the native call-return execution region and same-run support"
        );
    } else {
        assert_eq!(
            named, 0,
            "catalog must not claim named result identity without Flow"
        );
    }
    let false_spare:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_handoff_assessments a JOIN occurrences o ON o.id=a.argument WHERE a.value IS NOT NULL AND o.syntax_kind=48").await;
    assert_eq!(false_spare, 0, "same type is not producer-result identity");
    let controls:(i64,i64,i64,i64)=catalog_runtime::one(&fixture, "SELECT (SELECT count(*) FROM structural_argument_flows) AS fixture_column_0,(SELECT count(*) FROM structural_argument_flows WHERE alias IS NOT NULL) AS fixture_column_1,(SELECT count(*) FROM structural_conditional_raises) AS fixture_column_2,(SELECT count(*) FROM structural_unfollowed_arguments) AS fixture_column_3").await;
    if profile == Profile::Behavioral {
        assert!(controls.0 > 0, "actual direct parameter forwarding");
        assert!(controls.1 > 0, "one native identity alias");
        let conditional_alias: i64 = catalog_runtime::one_with(&fixture, "SELECT count(*) FROM structural_argument_flows f JOIN structural_public_candidates p ON p.entity=f.caller AND p.frame=f.frame JOIN structural_handoff_values v ON v.id=f.alias JOIN flow_region_observations r ON r.id=v.named_region JOIN assertion_qualifications q ON q.id=f.qualification JOIN assertion_qualifications rq ON rq.id=r.qualification WHERE p.path='api.alias_forward' AND f.conditional AND rq.condition<>$1 AND q.condition=rq.condition", vec![datafusion::common::ScalarValue::FixedSizeBinary(16,Some(conditions::Diagram::always().id().bytes().to_vec()))]).await;
        assert!(
            conditional_alias > 0,
            "alias forwarding retains the intervening native call-return condition"
        );
        assert!(controls.2 > 0, "qualified local conditional raise");
        assert!(controls.3 > 0, "computed/rebound argument boundaries");
        let qualified:i64=catalog_runtime::one_with(&fixture, "SELECT count(*) FROM structural_argument_flows f JOIN structural_public_candidates p ON p.entity=f.caller AND p.frame=f.frame JOIN local_flow_contributions l ON l.id=f.contribution JOIN assertion_qualifications q ON q.id=f.qualification JOIN assertion_qualifications lq ON lq.id=l.qualification JOIN flow_value_observations v ON v.id=l.value JOIN flow_use_inventory_observations i ON i.use_=v.use_ WHERE p.path='api.tested' AND i.complete AND i.native_count=1 AND i.mapped_count=1 AND f.conditional AND q.condition<>$1 AND q.condition=lq.condition AND q.modality>=lq.modality AND q.approximation=lq.approximation", vec![datafusion::common::ScalarValue::FixedSizeBinary(16,Some(conditions::Diagram::always().id().bytes().to_vec()))]).await;
        assert!(
            qualified > 0,
            "actual native singleton Local qualification remains conditional in persisted Structural flow"
        );
        let caught:i64=catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_conditional_raises r JOIN structural_control_paths p ON p.id=r.path JOIN structural_control_traversals t ON t.id=p.traversal JOIN structural_public_candidates c ON c.entity=t.seed AND c.frame=t.frame WHERE c.path IN ('api.caught','api.swallowed','api.tested')").await;
        assert_eq!(
            caught, 0,
            "catching, swallowed and value-tested paths must suppress propagated raise claim"
        );
    } else {
        assert_eq!(controls, (0, 0, 0, 0));
    }
    if profile == Profile::Behavioral {
        let stopped: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_control_traversals WHERE stop IS NOT NULL").await;
        assert!(stopped > 0, "bounded forwarding retains its depth stop");
        let literal: i64 = catalog_runtime::one(&fixture, "SELECT count(*) FROM structural_literal_arguments").await;
        assert!(literal > 0, "exact native literal argument retained");
        let unpacked:i64=catalog_runtime::one_with(&fixture, "SELECT count(*) FROM structural_unfollowed_arguments WHERE binding IS NULL AND reason=$1", vec![datafusion::common::ScalarValue::Int16(Some(obligation::ObligationKind::UnsupportedUnpacking.code()))]).await;
        assert!(
            unpacked > 0,
            "unmapped parameter unpacking remains explicit"
        );
    }
}
#[tokio::test]
async fn structural_candidates_paths_and_usage_compile_in_catalog() {run(Profile::Catalog).await;}
#[tokio::test]
async fn structural_candidates_paths_and_usage_compile_in_behavioral() {run(Profile::Behavioral).await;}

/// Corrupt only selected completed structural results and use the model's shared replay.
#[tokio::test]
async fn structural_replay_refuses_support_invocation_and_condition_forgery() {
 use lctx_model::domain::{analysis::structural as owner,structural::controls::ArgumentFlow};
 let mut settings=catalog_runtime::settings("api");settings.configured_seeds=vec!["api.Client".into(),"api.missing".into()];
 let fixture=catalog_runtime::compile("structural_usage",Profile::Behavioral,Frontier::Catalog,settings,None).await;
 let invariant=fixture.workspace.model().invariants().iter().find(|i|i.name=="structural_replay").unwrap();
 for corruption in 0..6 {
  let mut check=(invariant.create)(fixture.workspace.budget());let mut changed=0;
  for input in &invariant.inputs {
   let relation=fixture.workspace.input_relation(input).unwrap();
   for batch in relation.batches().unwrap(){let batch=batch.unwrap();let mut forwarded=batch.clone();
    if corruption==1 && input.name()==owner::AnalysisOutcome::NAME {let mut rows=owner::AnalysisOutcome::decode(&batch).unwrap();for row in &mut rows{if row.status==analysis::AnalysisStatus::Partial{row.status=analysis::AnalysisStatus::Completed;row.reason=None;changed+=1;}}forwarded=owner::AnalysisOutcome::encode(&rows).unwrap();}
    if corruption==2 && input.name()==owner::AnalysisOutcome::NAME && changed==0 {let mut rows=owner::AnalysisOutcome::decode(&batch).unwrap();if !rows.is_empty(){rows.remove(0);changed+=1;}forwarded=owner::AnalysisOutcome::encode(&rows).unwrap();}
    if corruption==3 && input.name()==owner::Invocation::NAME && changed==0 {let mut rows=owner::Invocation::decode(&batch).unwrap();if let Some(original)=rows.first(){let mut extra=original.clone();extra.subject=Some(fixture.workspace.completed::<normalized::entities::EntityRef>().unwrap().batches().unwrap().flat_map(|b|normalized::entities::EntityRef::decode(&b.unwrap()).unwrap()).find(|r|matches!(r,normalized::entities::EntityRef::Callable{..})).unwrap().id());assert_ne!(extra.id(),original.id());rows.push(extra);changed+=1;}forwarded=owner::Invocation::encode(&rows).unwrap();}
    if corruption==4 && input.name().starts_with("structural_"){changed+=batch.num_rows();forwarded=batch.slice(0,0);}
    if corruption==5 && input.name()==ArgumentFlow::NAME {let mut rows=ArgumentFlow::decode(&batch).unwrap();for row in &mut rows{if row.conditional{row.conditional=false;row.qualification=fixture.workspace.completed::<assertion::AssertionQualification>().unwrap().batches().unwrap().flat_map(|b|assertion::AssertionQualification::decode(&b.unwrap()).unwrap()).find(|q|q.condition==conditions::Diagram::always().id()).unwrap().id();changed+=1;}}forwarded=ArgumentFlow::encode(&rows).unwrap();}
    check.visit_input(input,&forwarded).unwrap();
   }
  }
  let result=check.finish();if corruption==0{result.unwrap();}else{assert!(changed>0,"corruption {corruption} must exercise actual rows");assert!(result.is_err(),"corruption {corruption} accepted");}
 }
}
