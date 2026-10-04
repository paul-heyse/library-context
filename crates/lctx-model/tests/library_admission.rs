//! Hand-expected domain captures, coverage and semantic refusal, independent of catalog members.
use lctx_model::domain::{self, attribution::*, input::*, resources::ResourceBudget, serving::*, source::CoverageScope, *};
fn id<T>(n:u8)->Id<T> {serde::Deserialize::deserialize(serde::de::value::SeqDeserializer::<_,serde::de::value::Error>::new([n;16].into_iter())).unwrap()}
fn capture(d:&mut LibraryAdmissionData,n:u8,name:&str,version:&str,role:DistributionRole)->(Id<InputRevision>,Id<Release>) {
    let input=InputRevision{manifest:ContentHash::of(&[n])};
    let input=d.inputs.insert(input).unwrap();
    let package=d.packages.insert(Package{name:name.into()}).unwrap();
    let release=d.releases.insert(Release{package,version:version.into()}).unwrap();
    d.distributions.insert(InputDistribution{input,release,role}).unwrap();
    (input,release)
}
#[test]
fn explicit_captures_admit_empty_domains_preserve_all_releases_and_exclude_dependencies() {
    let b=ResourceBudget::fixed(1<<20).unwrap();
    let mut d=LibraryAdmissionData::new(&b);
    let a=capture(&mut d,1,"demo","1",DistributionRole::FirstParty);
    let z=capture(&mut d,2,"demo","2",DistributionRole::FirstParty);
    let dependency=capture(&mut d,3,"dependency","9",DistributionRole::Dependency);
    let other=capture(&mut d,4,"other","1",DistributionRole::FirstParty);
    let index=PreparedLibraryDomains::prepare(&d,&b).unwrap();
    assert!(index.resolve(Some(&Name::new("dependency").unwrap())).is_err());
    assert!(index.resolve(Some(&Name::new("missing").unwrap())).is_err());
    let demo=index.resolve(Some(&Name::new("demo").unwrap())).unwrap();
    assert_eq!(demo.domains().len(),1);
    assert_eq!(demo.domains()[0].captures.len(),2);
    assert!(demo.contains_capture(a.0,a.1) && demo.contains_capture(z.0,z.1));
    assert!(!demo.contains_capture(a.0,z.1) && !demo.contains_release(dependency.1));
    assert!(!demo.contains_capture(other.0,other.1));
    assert!(demo.domains()[0].captures.iter().all(|c|matches!(c.collection,Availability::Unavailable{..})));
    let union=index.resolve(None).unwrap();
    assert_eq!(union.domains().iter().map(|d|d.name.as_str()).collect::<Vec<_>>(),["demo","other"]);
    assert!(union.contains_capture(other.0,other.1));
}
#[test]
fn corpus_only_evidence_is_admitted_without_members_and_coverage_keeps_collection_states() {
    let b=ResourceBudget::fixed(1<<20).unwrap();
    let mut d=LibraryAdmissionData::new(&b);
    let missing=capture(&mut d,1,"missing-coverage","1",DistributionRole::FirstParty);
    let partial=capture(&mut d,2,"partial","1",DistributionRole::FirstParty);
    let not_requested=capture(&mut d,3,"not-requested","1",DistributionRole::FirstParty);
    let corpus=d.inputs.insert(InputRevision{manifest:ContentHash::of(b"corpus")}).unwrap();
    d.corpora.insert(CorpusLibrary{library:partial.0,corpus}).unwrap();
    for (input,status) in [(partial.0,CoverageStatus::Partial),(not_requested.0,CoverageStatus::NotRequested)] {
        let scope=d.scopes.insert(CoverageScope::Input{input}).unwrap();
        let run=if status!=CoverageStatus::NotRequested {Some(d.runs.insert(ProviderRun{provider:id(8),context:id(7),input,configuration:ContentHash::of(b"config"),requested_families:ContentHash::of(b"docs")}).unwrap())}else{None};
        d.coverage.insert(ProviderCoverage{scope,provider:(status!=CoverageStatus::NotRequested).then(||id(8)),context:id(7),family:FactFamily::Docs,run,status,reason:(status==CoverageStatus::Partial).then_some(ObligationKind::IncompleteCoverage),diagnostic:None}).unwrap();
    }
    let index=PreparedLibraryDomains::prepare(&d,&b).unwrap();
    let partial=index.resolve(Some(&Name::new("partial").unwrap())).unwrap();
    assert!(partial.contains_corpus(corpus));
    assert!(!partial.contains_corpus(missing.0));
    let capture=&partial.domains()[0].captures[0];
    assert_eq!(capture.corpora,[corpus]);
    assert_eq!(capture.unavailable_inputs,[corpus]);
    assert_eq!(capture.coverage[0].input,partial.domains()[0].captures[0].release.input);
    assert!(matches!(capture.collection,Availability::Partial{..}));
    assert_eq!(capture.coverage[0].status,CoverageStatus::Partial);
    assert_eq!(capture.coverage[0].family,FactFamily::Docs);
    let not_requested=index.resolve(Some(&Name::new("not-requested").unwrap())).unwrap();
    assert!(matches!(not_requested.domains()[0].captures[0].collection,Availability::NotRequested{}));
    assert_eq!(not_requested.domains()[0].captures[0].coverage[0].status,CoverageStatus::NotRequested);
}
#[test]
fn prepared_and_request_indexes_are_charged_and_refusal_releases_partial_state() {
    let b=ResourceBudget::fixed(1<<20).unwrap();
    let mut d=LibraryAdmissionData::new(&b);
    capture(&mut d,1,"demo","1",DistributionRole::FirstParty);
    let before=b.reserved();
    let index=PreparedLibraryDomains::prepare(&d,&b).unwrap();
    assert!(b.reserved()>before);
    let prepared=b.reserved();
    let metadata=index.resolve(None).unwrap().metadata(&b).unwrap();
    assert!(b.reserved()>prepared);
    drop(metadata);assert_eq!(b.reserved(),prepared);
    drop(index);assert_eq!(b.reserved(),before);
    let small=ResourceBudget::fixed(16).unwrap();
    assert!(matches!(PreparedLibraryDomains::prepare(&d,&small),Err(ModelError::Resource{..})));
    assert_eq!(small.reserved(),0);
}
#[test]
fn schema_and_mapping_expose_admission_and_optional_search_filter() {
    assert_eq!(FailureKind::from_name("unknown_library"),Some(FailureKind::UnknownLibrary));
    assert_eq!(serde_json::to_value(PublicFailure::new(FailureKind::UnknownLibrary)).unwrap()["kind"],"unknown_library");
    for tool in ["search_operations","search_evidence","search_capabilities"] {
        let raw=if tool=="search_evidence" {r#"{"query":"api","families":[]}"#} else {r#"{"query":"api"}"#};
        let request=decode_request(tool,raw,&ResourceLimits::default()).unwrap();
        assert!(!serde_json::to_value(request).unwrap().to_string().contains("\"library\""));
    }
    assert!(decode_request("find_operations",r#"{}"#,&ResourceLimits::default()).is_err());
    let binding=mappings::prepared_binding(mappings::PreparedDependency::CatalogIdentity).lowered();
    let names=binding.sources.iter().map(|r|r.name()).collect::<std::collections::BTreeSet<_>>();
    for expected in ["input_revisions","packages","releases","input_distributions","corpus_libraries","provider_coverage","coverage_scopes","modules","source_artifacts"] {assert!(names.contains(expected),"{expected}");}
}

#[test]
fn grouped_counts_preserve_mixed_statuses_and_representatives_under_row_shuffle() {
    fn fixture(reverse:bool,b:&ResourceBudget)->LibraryAdmissionData {
        let mut d=LibraryAdmissionData::new(b);
        let (input,_)=capture(&mut d,1,"demo","1",DistributionRole::FirstParty);
        let run=d.runs.insert(ProviderRun{provider:id(8),context:id(7),input,
            configuration:ContentHash::of(b"config"),requested_families:ContentHash::of(b"docs")}).unwrap();
        let mut rows=Vec::new();
        for (n,status) in [(1,CoverageStatus::Partial),(2,CoverageStatus::Partial),
            (3,CoverageStatus::CompleteUnderStatedModel),(4,CoverageStatus::NotRequested),
            (5,CoverageStatus::Unavailable)] {
            let artifact=d.artifacts.insert(domain::source::SourceArtifact::from_bytes(input,format!("doc{n}.md"),&[n]).unwrap()).unwrap();
            let scope=d.scopes.insert(CoverageScope::Artifact{artifact}).unwrap();
            rows.push(ProviderCoverage{scope,context:id(7),provider:(status!=CoverageStatus::NotRequested).then(||id(8)),
                run:(status!=CoverageStatus::NotRequested).then_some(run),family:FactFamily::Docs,status,
                reason:matches!(status,CoverageStatus::Partial | CoverageStatus::Unavailable).then_some(ObligationKind::IncompleteCoverage),diagnostic:None});
        }
        if reverse {rows.reverse();}
        for row in rows {d.coverage.insert(row).unwrap();}
        d
    }
    let b=ResourceBudget::fixed(1<<20).unwrap();
    let forward=fixture(false,&b);let reverse=fixture(true,&b);
    let a=PreparedLibraryDomains::prepare(&forward,&b).unwrap();
    let z=PreparedLibraryDomains::prepare(&reverse,&b).unwrap();
    let a=a.resolve(None).unwrap();let z=z.resolve(None).unwrap();
    assert_eq!(a.domains(),z.domains());
    let capture=&a.domains()[0].captures[0];
    assert!(matches!(capture.collection,Availability::Partial{..}));
    assert!(capture.unavailable_inputs.is_empty());
    assert_eq!(capture.coverage.len(),4);
    assert_eq!(capture.coverage.iter().map(|c|c.observations).sum::<u64>(),5);
    let partial=capture.coverage.iter().find(|c|c.status==CoverageStatus::Partial).unwrap();
    assert_eq!(partial.observations,2);
    let representative=forward.coverage.get(partial.coverage).unwrap();
    assert_eq!(representative.status,CoverageStatus::Partial);
    assert_eq!(representative.scope,partial.scope);
    assert!(capture.coverage.iter().any(|c|c.status==CoverageStatus::NotRequested && c.provider.0.is_none()));
    assert!(capture.coverage.iter().any(|c|c.status==CoverageStatus::CompleteUnderStatedModel));
    assert!(capture.coverage.iter().any(|c|c.status==CoverageStatus::Unavailable));
}
