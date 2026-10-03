//! Populated canonical Ruff facts and the first semantic recognition use the actual provider.
#[path = "typed_driver/mod.rs"]
mod typed_driver;
use lctx_model::domain::{*, source::*, attribution::*, syntax::*, ruff::*};
use typed_driver::{files,rows,run};
inspector!(Context, RuffContextObservation,RuffContextSupport,Occurrence,SyntaxObservation,DeclarationObservation,DeclarationDecorator,Provider,ProviderRun,ProviderCoverage);
#[tokio::test]
async fn aliased_overloads_and_local_spelling_keep_distinct_semantics() {
    let tables=typed_driver::Tables::default();
    run(&files("ruff_context"),Context(tables.clone())).await.unwrap();
    let contexts=rows::<RuffContextObservation>(&tables);
    let occurrences=rows::<Occurrence>(&tables);
    let spellings=rows::<SyntaxObservation>(&tables);
    let declarations=rows::<DeclarationObservation>(&tables);
    let named=|declaration:&DeclarationObservation| spellings.iter().find(|row|row.occurrence==declaration.name).unwrap().spelling.as_str();
    let typed:Vec<_>=declarations.iter().filter(|row|named(row)=="typed").collect();
    assert_eq!(typed.len(),3);
    assert_eq!(typed.iter().filter(|row|row.overload).count(),2);
    assert!(!declarations.iter().find(|row|named(row)=="ordinary").unwrap().overload);
    let ov=contexts.iter().filter(|row|row.phase==ContextPhase::ActiveNode && row.qualified_name.as_deref()==Some(&["typing".into(),"overload".into()])).count();
    assert_eq!(ov,2);
    let typing_value=spellings.iter().find(|row|row.spelling=="typing_value").unwrap().occurrence;
    assert!(contexts.iter().any(|row|row.subject==typing_value && row.type_checking == Some(true) && row.typing == Some(true)));
    assert!(contexts.iter().filter(|row|row.phase==ContextPhase::FinalReference).count()>0);
    let providers=rows::<Provider>(&tables);
    let ruff=providers.iter().find(|provider|provider.tool=="ruff").unwrap();
    let runs=rows::<ProviderRun>(&tables);
    let run=runs.iter().find(|run|run.provider==ruff.id()).unwrap();
    assert!(rows::<RuffContextSupport>(&tables).iter().all(|support|support.run==run.id()));
    assert!(contexts.iter().all(|row|occurrences.iter().any(|occurrence|occurrence.id()==row.subject)));
    assert!(rows::<ProviderCoverage>(&tables).iter().any(|row|row.family==FactFamily::Syntax && row.provider==Some(ruff.id()) && row.status==CoverageStatus::CompleteUnderStatedModel));
}
