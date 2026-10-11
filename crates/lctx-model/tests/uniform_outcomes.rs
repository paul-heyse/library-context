use lctx_model::domain::{*,attribution::{AnalysisContext,FactFamily},input::*,source::{SourceArtifact,CoverageScope},producer_contract::{*,acquisition::CapturedAcquisition,facts},completed::{ContributionSpec,CompletedContribution,OutputContent,OutputProfile},stages::Profile,uniform_outcome::*};
use std::sync::Arc;
fn context()->AnalysisContext{AnalysisContext{python_version:"3.14.7".into(),python_platform:"linux".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"context"),environment_digest:ContentHash::of(b"environment"),lock_digest:None}}
fn fixture(paths:&[&str],budget:&resources::ResourceBudget)->(CapturedAcquisition,AnalysisContext,UniformNotRequested,CompletedContribution){
    let input=InputRevision::from_entries(paths.iter().map(|path|ManifestEntry{path:(*path).into(),content:ContentHash::of(b""),byte_len:0}).collect()).unwrap();
    let mut capture=CapturedAcquisition::tree(input.clone(),"owned uniform control".into());
    let acquisition=InputAcquisition{input:input.id(),origin:capture.origin.id()}.id();
    for path in paths{let artifact=SourceArtifact::from_bytes(input.id(),(*path).into(),b"").unwrap();capture.unowned.push(UnownedArtifact{artifact:artifact.id(),acquisition});capture.uses.push(ArtifactUse{artifact:artifact.id(),input:input.id(),role:SourceRole::Release});capture.artifacts.push(artifact);}
    capture.canonicalize();capture.validate().unwrap();let context=context();
    let contract=facts::assembly();let settings=ContentHash::of(b"assemble");let mut stage=contract.bind(ContentHash::of(b"pure control executable"),settings,[]);
    let binding=stage.captured_binding.as_mut().unwrap();binding.context_dependency=CapturedContextDependency::SemanticContext;binding.sources.push(CapturedSourceBinding{input:input.id(),context:Some(context.id()),configuration:Some(context.config_digest),acquisition:Arc::new(capture.clone())});
    let (_,domain)=ExpectedUniform::captured_family(&stage,Profile::Catalog,input.id(),context.id(),FactFamily::Flow,budget).unwrap();let descriptor=domain.declaration().unwrap();
    let spec=ContributionSpec{captured_binding:stage.captured_binding,producer:stage.name.into(),profile:OutputProfile::Specific(Profile::Catalog),model:ContentHash::of(b"pure model"),implementation:stage.code,configuration:Some(settings),inputs:vec![],outputs:contract.outputs.iter().map(|relation|relation.name().to_owned()).collect()};
    let outputs=spec.outputs.iter().map(|name|(name.clone(),OutputContent{rows:u64::from(name==UniformNotRequested::NAME),content:ContentHash::of(b"semantic-only fixture; native membership is independent")})).collect();
    (capture,context,descriptor,CompletedContribution{spec,outcome:0,outputs})
}
#[test]
fn compact_uniform_scoped_operation_requires_actual_semantic_owner_and_exact_domain(){
    let budget=resources::ResourceBudget::fixed(64<<20).unwrap();let (capture,context,row,completed)=fixture(&["a.py","b.py"],&budget);
    let owner=ValidatedUniformOwner::validate_semantic_binding(&completed,&row,Profile::Catalog,&budget).unwrap();
    let registry=UniformOwners::new(&budget);assert!(registry.require(&row).is_err());
    let mut registry=registry;registry.include(&owner).unwrap();assert_eq!(registry.require(&row).unwrap().contribution(),completed.identity().unwrap());
    let domain=ExpectedUniform::native_inputs(&owner,&capture.revision,&context,&capture.artifacts,&capture.uses,Profile::Catalog,&budget).unwrap();assert_eq!(row.members,2);
    let scope=CoverageScope::Artifact{artifact:capture.artifacts[0].id()}.id();assert_eq!(domain.scoped(scope,row.definitions),ScopedUniformOutcome::NotRequested(CoverageAuthority::Uniform{descriptor:row.id()}));
    assert_eq!(domain.scoped(CoverageScope::Input{input:capture.revision.id()}.id(),row.definitions),ScopedUniformOutcome::OutsideDomain);
    assert_eq!(domain.scoped(scope,ContentHash::of(b"foreign definition")),ScopedUniformOutcome::OutsideDomain);
    assert!(ExpectedUniform::native_inputs(&owner,&capture.revision,&context,&capture.artifacts[..1],&capture.uses[..1],Profile::Catalog,&budget).is_err());
    assert!(ExpectedUniform::native_inputs(&owner,&capture.revision,&context,&capture.artifacts,&capture.uses,Profile::Behavioral,&budget).is_err());
    let mut changed=row.clone();changed.sources=ContentHash::of(b"foreign capture");assert!(ValidatedUniformOwner::validate_semantic_binding(&completed,&changed,Profile::Catalog,&budget).is_err());
    changed=row.clone();changed.definitions=ContentHash::of(b"foreign policy");assert!(ValidatedUniformOwner::validate_semantic_binding(&completed,&changed,Profile::Catalog,&budget).is_err());
    let mut incomplete=completed.clone();incomplete.outputs.pop_first();assert!(ValidatedUniformOwner::validate_semantic_binding(&incomplete,&row,Profile::Catalog,&budget).is_err());
}
#[test]
fn explicit_empty_uniform_domain_is_not_missing_coverage_and_retains_its_budget_owner(){
    let budget=resources::ResourceBudget::fixed(16<<20).unwrap();let (capture,context,row,completed)=fixture(&[],&budget);assert_eq!(row.members,0);
    {
        let owner=ValidatedUniformOwner::validate_semantic_binding(&completed,&row,Profile::Catalog,&budget).unwrap();let copy=owner.clone();drop(owner);assert!(budget.reserved()>0);
        let domain=ExpectedUniform::native_inputs(&copy,&capture.revision,&context,&[],&[],Profile::Catalog,&budget).unwrap();assert!(domain.is_explicit_empty());assert_eq!(domain.members().count(),0);
        assert_eq!(domain.scoped(CoverageScope::Input{input:capture.revision.id()}.id(),row.definitions),ScopedUniformOutcome::OutsideDomain);
    }
    assert_eq!(budget.reserved(),0);
}
#[test]
fn scoped_availability_preserves_real_compact_authority_and_refuses_missing_obligations(){
    use std::collections::{BTreeMap,BTreeSet};
    use admission::{Expected,ScopedAvailability,Availability};
    let budget=resources::ResourceBudget::fixed(64<<20).unwrap();
    let (capture,context,row,completed)=fixture(&["a.py","b.py"],&budget);
    let owner=ValidatedUniformOwner::validate_semantic_binding(&completed,&row,Profile::Catalog,&budget).unwrap();
    let domain=ExpectedUniform::native_inputs(&owner,&capture.revision,&context,&capture.artifacts,&capture.uses,Profile::Catalog,&budget).unwrap();
    let scopes=capture.artifacts.iter().map(|artifact|{let scope=CoverageScope::Artifact{artifact:artifact.id()};(scope.id(),scope)}).collect::<BTreeMap<_,_>>();
    let expected=scopes.keys().map(|scope|Expected{scope:*scope,family:FactFamily::Flow,provider:None}).collect::<BTreeSet<_>>();
    assert!(ScopedAvailability::from_completed(Profile::Catalog,&expected,&[],&scopes,&budget).is_err());
    let available=ScopedAvailability::from_completed_with_uniform(Profile::Catalog,&expected,&[],&scopes,&[(&row,&domain,&owner)],&budget).unwrap();
    assert_eq!(available.evidence().len(),2);
    assert!(available.evidence().iter().all(|evidence|evidence.coverage==CoverageAuthority::Uniform{descriptor:row.id()} && evidence.availability==Availability::NotRequested && evidence.provider.is_none()));
    assert_eq!(available.empty_universe(FactFamily::Flow),None);
    let mut missing=expected.clone();missing.pop_first();
    assert!(ScopedAvailability::from_completed_with_uniform(Profile::Catalog,&missing,&[],&scopes,&[(&row,&domain,&owner)],&budget).is_err());
    assert!(ScopedAvailability::from_completed_with_uniform(Profile::Catalog,&expected,&[],&scopes,&[(&row,&domain,&owner),(&row,&domain,&owner)],&budget).is_err());
    drop(owner);drop(domain);
    // Availability owns the shared semantic proof, including its domain lease.
    assert!(budget.reserved()>0);drop(available);assert_eq!(budget.reserved(),0);
}
#[test]
fn library_metadata_requires_explicit_owner_and_preserves_empty_compact_domain(){
    use serving::{LibraryAdmissionData,PreparedLibraryDomains,Availability};
    for paths in [&["a.py"][..],&[][..]] {
        let budget=resources::ResourceBudget::fixed(64<<20).unwrap();
        let (capture,context,row,completed)=fixture(paths,&budget);
        let owner=ValidatedUniformOwner::validate_semantic_binding(&completed,&row,Profile::Catalog,&budget).unwrap();
        let mut data=LibraryAdmissionData::new(&budget);
        let input=data.inputs.insert(capture.revision.clone()).unwrap();
        let package=data.packages.insert(Package{name:"compact-demo".into()}).unwrap();
        let release=data.releases.insert(Release{package,version:"1".into()}).unwrap();
        data.distributions.insert(InputDistribution{input,release,role:DistributionRole::FirstParty}).unwrap();
        data.contexts.insert(context).unwrap();
        for artifact in &capture.artifacts {
            data.artifacts.insert(artifact.clone()).unwrap();
        }
        for usage in &capture.uses {data.uses.insert(usage.clone()).unwrap();}
        data.uniform.insert(row.clone()).unwrap();
        assert!(PreparedLibraryDomains::prepare(&data,&budget).is_err(),"a deserialized descriptor cannot establish its owner");
        data.admit_uniform_owner(&owner).unwrap();
        if !paths.is_empty(){assert!(PreparedLibraryDomains::prepare(&data,&budget).is_err(),"a compact domain cannot invent absent canonical scopes");}
        for artifact in &capture.artifacts {data.scopes.insert(CoverageScope::Artifact{artifact:artifact.id()}).unwrap();}
        let prepared=PreparedLibraryDomains::prepare(&data,&budget).unwrap();
        let resolved=prepared.resolve(None).unwrap();
        let capture=&resolved.domains()[0].captures[0];
        assert!(matches!(capture.collection,Availability::NotRequested{}));
        assert!(capture.unavailable_inputs.is_empty());
        let packet=&capture.coverage[0];
        assert_eq!(packet.coverage,CoverageAuthority::Uniform{descriptor:row.id()});
        assert_eq!(packet.observations,paths.len() as u64);
        assert_eq!(packet.scope.0.is_none(),paths.is_empty());
    }
}
