//! Scoped production kernels and independent admission; no provider or database prerequisites.
use lctx_model::domain::{*, source::*, attribution::*, assertion::*, symbols::*, normalized::{entity_normalization::*, entities::*}, resources::ResourceBudget};
fn source() -> SourceArtifact {
    SourceArtifact::from_bytes(input::InputRevision::from_entries(vec![]).unwrap().id(),"scope.py".into(),b"pass\n").unwrap()
}
fn occurrence(source: &SourceArtifact, path: Vec<i32>, kind: SyntaxKind) -> Occurrence {
    Occurrence { source:source.id(), start:0,end:5,syntax_kind:kind,role:OccurrenceRole::Syntax,structural_path:path }
}
#[test]
fn ordered_ownership_preserves_headers_bodies_and_releases_wide_sources() {
    let source=source();
    let rows=vec![occurrence(&source,vec![0],SyntaxKind::ModModule), occurrence(&source,vec![0,0],SyntaxKind::StmtFunctionDef), occurrence(&source,vec![0,0,0],SyntaxKind::Parameters), occurrence(&source,vec![0,0,0,0],SyntaxKind::Parameter), occurrence(&source,vec![0,0,1],SyntaxKind::StmtReturn), occurrence(&source,vec![0,0,1,0],SyntaxKind::ExprName), occurrence(&source,vec![0,1],SyntaxKind::StmtClassDef), occurrence(&source,vec![0,1,0],SyntaxKind::StmtFunctionDef), occurrence(&source,vec![0,1,0,0],SyntaxKind::Parameters), occurrence(&source,vec![0,1,0,1],SyntaxKind::StmtReturn)];
    let budget=ResourceBudget::fixed(64<<10).unwrap();
    let mut sweep=OwnershipSweep::new(&budget);
    for row in &rows {
        let result=sweep.push(row,None,&budget).unwrap();
        assert_eq!(result.owners.iter().next().unwrap().owner,occurrence_owner::owner_of(row,&rows).unwrap());
    }
    drop(sweep);
    assert_eq!(budget.reserved(),0);
    let mut sweep=OwnershipSweep::new(&budget);
    drop(sweep.push(&rows[0],None,&budget).unwrap());
    for index in 0..20_000 {
        let row=occurrence(&source,vec![0,index],SyntaxKind::StmtPass);
        let output=sweep.push(&row,None,&budget).unwrap();
        assert_eq!(output.owners.iter().next().unwrap().owner,rows[0].id());
        drop(output);
        assert!(budget.reserved()<4096,"wide source must retain only its active path");
    }
    drop(sweep);
    assert_eq!(budget.reserved(),0);
}
#[test]
fn ordered_ownership_refuses_gaps_duplicates_and_reversed_sources() {
    let source=source();
    let root=occurrence(&source,vec![0],SyntaxKind::ModModule);
    let budget=ResourceBudget::fixed(64<<10).unwrap();
    let mut sweep=OwnershipSweep::new(&budget);
    drop(sweep.push(&root,None,&budget).unwrap());
    assert!(sweep.push(&root,None,&budget).is_err());
    let mut sweep=OwnershipSweep::new(&budget);
    drop(sweep.push(&root,None,&budget).unwrap());
    assert!(sweep.push(&occurrence(&source,vec![0,1,0],SyntaxKind::ExprName),None,&budget).is_err());
    let other=SourceArtifact::from_bytes(source.input,"other.py".into(),b"pass\n").unwrap();
    let (first,last)=if source.id().bytes()<other.id().bytes() {(&source,&other)} else {(&other,&source)};
    let mut sweep=OwnershipSweep::new(&budget);
    drop(sweep.push(&occurrence(last,vec![0],SyntaxKind::ModModule),None,&budget).unwrap());
    assert!(sweep.push(&occurrence(first,vec![0],SyntaxKind::ModModule),None,&budget).is_err());
}
fn check(data:&EntityData, output:&EntityOutput, budget:&ResourceBudget) -> Result<(),ModelError> {
    let invariant=invariants().into_iter().find(|row| row.name=="normalized_entity_membership").unwrap();
    assert_eq!(invariant.purpose,InvariantPurpose::Admission);
    let mut check=(invariant.create)(budget);
    macro_rules! facts { ($($field:ident: $ty:ty => $family:ident,)*) => { $(if invariant.inputs.iter().any(|input| input.name()==<$ty>::NAME) { check.visit(<$ty>::NAME,&<$ty as Record>::encode(&data.$field.iter().cloned().collect::<Vec<_>>())?)?; })* }; }
    lctx_model::normalized_entity_inputs!(facts);
    macro_rules! outputs { ($($field:ident: $ty:ty,)*) => { $(if invariant.inputs.iter().any(|input| input.name()==<$ty>::NAME) { check.visit(<$ty>::NAME,&<$ty as Record>::encode(&output.$field.iter().cloned().collect::<Vec<_>>())?)?; })* }; }
    lctx_model::normalized_entity_outputs!(outputs);
    check.finish()
}
#[test]
fn required_entity_admission_refuses_kind_membership_and_public_fidelity_without_replay() {
    let budget=ResourceBudget::fixed(2<<20).unwrap();
    let mut data=EntityData::new(&budget);
    let source=source();
    let module=Module {source:source.id(),qualified_name:"scope".into()};
    let root=occurrence(&source,vec![0],SyntaxKind::ModModule);
    data.modules.insert(module.clone()).unwrap(); data.occurrences.insert(root.clone()).unwrap();
    let baseline=normalize(data.inputs(),&budget).unwrap(); check(&data,&baseline,&budget).unwrap();
    let mut output=normalize(data.inputs(),&budget).unwrap();
    let wrong=CallableEntity::Source { declaration:root.id(),kind:CallableKind::Function };
    output.callables.insert(wrong.clone()).unwrap(); output.refs.insert(EntityRef::Callable {callable:wrong.id()}).unwrap();
    assert!(check(&data,&output,&budget).is_err()); drop(output);
    let empty=EntityOutput::new(&budget); assert!(check(&data,&empty,&budget).is_err()); drop(empty);
    let context=AnalysisContext {python_version:"3.14".into(),python_platform:"linux".into(),search_path:vec![],site_package_path:vec![],config_digest:ContentHash::of(b"config"),environment_digest:ContentHash::of(b"env"),lock_digest:None};
    let q=AssertionQualification {assumptions:assumptions::AssumptionSet::empty_id(),context:context.id(),scope:CoverageScope::Artifact {artifact:source.id()}.id(),condition:conditions::Diagram::always().id(),modality:Modality::Candidate,approximation:Approximation::Exact};
    data.qualifications.insert(q.clone()).unwrap();
    let public=PublicNameObservation {qualification:q.id(),access:module.id(),name:"uncertain".into(),via_dunder_all:false,origin:ExportOrigin::Untraced.id()};
    data.public_names.insert(public).unwrap();data.export_origins.insert(ExportOrigin::Untraced).unwrap();
    let mut output=normalize(data.inputs(),&budget).unwrap();check(&data,&output,&budget).unwrap();
    let mut exposure=output.exposures.iter().next().unwrap().clone(); exposure.publicity=PublicPathKnowledge::Known;
    // A fresh result avoids Rows correctly refusing an in-place conflicting identity.
    output=normalize_scope(data.inputs(),EntityKernel::Symbol,&budget).unwrap();
    output.refs.insert(EntityRef::Occurrence {occurrence:root.id()}).unwrap();output.refs.insert(EntityRef::Module {module:module.id()}).unwrap();output.exposures.insert(exposure).unwrap();
    assert!(check(&data,&output,&budget).is_err());
}
