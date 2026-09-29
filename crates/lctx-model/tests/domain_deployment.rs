#[path = "fixtures/deployment.rs"] mod fixture;
use fixture::Fixture;
use lctx_model::domain::{*,deployment::*};
#[test]
fn reports_preserve_full_duration_and_separate_environment_claims_from_runtime() {
    let f = Fixture::new(false);
    for name in ["report_collection_membership","report_collection_roles",TaskReportSupport::NAME,DeploymentSupport::NAME] {
        f.check(f.model.invariants().iter().find(|i| i.name == name).unwrap()).unwrap();
    }
    let report = TaskReport::decode(Batch::new(&f.model,vec![f.report.clone()]).unwrap().arrow()).unwrap().remove(0);
    assert_eq!(report.elapsed_ms,Milliseconds(u64::MAX)); assert_eq!(report.execution,CheckStatus::Passed);
    let environment = &f.rows::<ReportedEnvironment>()[0];
    let context = &f.rows::<lctx_model::domain::attribution::AnalysisContext>()[0];
    assert_ne!(environment.environment_digest,context.environment_digest);
    assert_ne!(environment.python_version,context.python_version);
}
#[test]
fn report_membership_requires_complete_unique_ordered_children_and_correct_roles() {
    let mut f = Fixture::new(false);
    let mut entries = f.rows::<ReportEntry>(); entries.pop(); f.put_entries(entries);
    assert!(f.check(&ReportCollection::invariants()[0]).is_err());
    let mut f = Fixture::new(false);
    let mut entries = f.rows::<ReportEntry>(); entries[0].ordinal += 1; f.put_entries(entries);
    assert!(f.check(&ReportCollection::invariants()[0]).is_err());
    let mut f = Fixture::new(false);
    let invocation = f.rows::<ReportCollection>().into_iter().find(|c| c.kind == ReportCollectionKind::Invocation).unwrap();
    let mut envs = f.rows::<ReportedEnvironment>(); envs[0].metadata = invocation.id(); f.put(envs);
    assert!(f.check(&ReportedEnvironment::invariants()[0]).is_err());
    assert!(ReportCollection::new(ReportCollectionKind::Invocation,vec![ReportValue::Command { ordinal: 1,text: "skipped".into() }]).is_err());
    assert!(ReportCollection::new(ReportCollectionKind::Invocation,vec![ReportValue::Argument { name: "same".into(),value: 1 },ReportValue::Argument { name: "same".into(),value: 2 }]).is_err());
    let (empty,values,entries) = ReportCollection::new(ReportCollectionKind::Invocation,vec![]).unwrap();
    let mut f = Fixture::new(false); f.put(vec![empty]); f.put(values); f.put_entries(entries);
    f.check(&ReportCollection::invariants()[0]).unwrap();
}
#[test]
fn report_association_requires_captured_input_ownership() {
    let f = Fixture::new(true);
    assert!(f.check(&TaskReportSupport::invariants()[0]).is_err());
    // A different reported path is retained as evidence text; it never silently redirects target.
    let mut f = Fixture::new(false); f.report.source_path = "/reported/foreign/environment.py".into();
    f.observation.report = f.report.id(); f.support.assertion = f.observation.id();
    f.put(vec![f.report.clone()]); f.put(vec![f.observation.clone()]); f.put(vec![f.support.clone()]);
    f.check(&TaskReportSupport::invariants()[0]).unwrap();
}
