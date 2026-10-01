//! Captured native twins challenge the normalized/Local/uncertain Summary ownership boundary.
#[path="fixtures/transfer_composition.rs"] mod fixture;
use lctx_model::domain::{analysis,attribution::*,execution::summary_production::SummaryData,
    local_semantics,normalized::{self,callable_aspects::*,symbolic_fields::*},*,};

fn aspects(f:&fixture::NativeFixture)->AspectOutput {
    let mut d=AspectData::new(&f.budget);
    macro_rules! raw{($($field:ident:$ty:ty,)*)=>{$(for row in f.rows::<$ty>(){d.$field.insert(row).unwrap();})*};}
    lctx_model::callable_aspect_inputs!(raw);
    macro_rules! normalized{($($field:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(normalized);
    let mut entities=normalized::entity_normalization::EntityData::new(&f.budget);
    macro_rules! facts{($($field:ident:$ty:ty=>$family:ident,)*)=>{$(for row in f.rows::<$ty>(){entities.$field.insert(row).unwrap();})*};}
    lctx_model::normalized_entity_inputs!(facts);
    let entities=normalized::entity_normalization::normalize(entities.inputs(),&f.budget).unwrap();
    macro_rules! entities{($($field:ident:$ty:ty,)*)=>{$(d.visit(<$ty>::NAME,&<$ty as Record>::encode(&entities.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_entity_outputs!(entities);
    normalize(&d,&f.budget).unwrap()
}

#[tokio::test]
async fn exact_native_store_keeps_three_reader_conditions_without_finite_authority(){
    let f=fixture::native_from("transfer_alternatives").await;
    let aspects=aspects(&f);
    let mut local=local_semantics::LocalData::new(&f.budget);
    let mut summary=SummaryData::new(&f.budget);
    macro_rules! feed{($ty:ty,$rows:expr)=>{{let b=<$ty as Record>::encode(&$rows).unwrap();local.visit(<$ty>::NAME,&b).unwrap();summary.visit(<$ty>::NAME,&b).unwrap();}};}
    for (name,batch) in f.tables.lock().unwrap().iter(){local.visit(name,batch).unwrap();summary.visit(name,batch).unwrap();}
    macro_rules! normalized{($($field:ident:$ty:ty,)*)=>{$(feed!($ty,f.data.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::normalized_binding_inputs!(normalized);
    macro_rules! aspect_rows{($($field:ident:$ty:ty,)*)=>{$(feed!($ty,aspects.$field.iter().cloned().collect::<Vec<_>>());)*};}
    lctx_model::callable_aspect_outputs!(aspect_rows);
    let mut inventory=analysis::native::NativeInventory::new(&f.budget);
    for (name,batch) in f.tables.lock().unwrap().iter(){
        if analysis::native::NativeInventory::inputs().iter().any(|i|i.name()==*name){inventory.visit(name,batch).unwrap();}
    }
    let inventory=inventory.collect().unwrap();
    feed!(analysis::native::NativeQualification,inventory.qualifications.iter().cloned().collect::<Vec<_>>());
    let input=f.rows::<input::InputRevision>()[0].id();
    let context=f.data.event_events.iter().next().unwrap().context;
    let (_,definition)=local_semantics::definition();
    let (invocation,_)=analysis::local::AnalysisInvocation::new(input,context,definition.id(),None,[]);
    feed!(analysis::local::AnalysisInvocation,vec![invocation.clone()]);
    let rows=local_semantics::produce(&local,&invocation,&definition,&f.budget).unwrap();
    let record=aspects.symbolic_classes.iter().find(|c|f.rows::<symbols::ClassTraitObservation>().iter()
        .any(|t|t.id()==c.traits&&f.data.symbols.get(t.symbol).is_some_and(|s|s.name=="RecordHolder"))).unwrap();
    assert!(record.supported_record);
    let store=aspects.symbolic_stores.iter().find(|s|s.class==record.class).unwrap();
    let receipts=rows.fields.symbolic_stores.iter().filter(|s|s.store==store.id()).collect::<Vec<_>>();
    assert_eq!(receipts.len(),1,"unchanged store needs exact value and receiver Entry proofs");
    let receipt=receipts[0];
    assert!(rows.entries.get(receipt.value_entry).is_some());
    assert!(rows.entries.get(receipt.receiver_entry).is_some());
    feed!(local_symbolic::SymbolicFieldStore,rows.fields.symbolic_stores.iter().cloned().collect::<Vec<_>>());
    let catalog=models::Catalog::committed().unwrap();
    let (_,definition)=execution::configuration::summaries(catalog.declaration().id(),execution::configuration::SummaryLimits{depth:2,..Default::default()}).unwrap();
    let (invocation,_)=analysis::summary::AnalysisInvocation::new(input,context,definition.id(),None,[]);
    let alternatives=execution::summary_symbolic::derive(&summary,&invocation,&f.budget).unwrap();
    let selected=alternatives.iter().filter(|a|summary.symbolic_links.get(a.link).is_some_and(|l|
        summary.symbolic_associations.get(l.association).is_some_and(|a|a.class==record.id()))).collect::<Vec<_>>();
    assert_eq!(selected.len(),3,"three qualified reader paths must survive");
    let conditions=selected.iter().map(|a|summary.entry.qualifications.get(a.reader_qualification).unwrap().condition).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(conditions.len(),3);
    for a in &selected{
        assert_eq!(a.depth,2);assert_eq!(a.reason,obligation::ObligationKind::ScopeBoundary);
        assert_eq!(a.store,Some(receipt.id()));
        assert_ne!(a.constructor,a.reader);
        let rq=summary.entry.qualifications.get(a.reader_qualification).unwrap();
        let cq=summary.entry.qualifications.get(a.constructor_qualification).unwrap();
        assert_eq!(rq.scope,cq.scope,"same artifact scope must be preserved");
        assert_ne!(rq.condition,cq.condition,"reader guards cannot become constructor guards");
    }
    assert!(alternatives.iter().all(|a|summary.symbolic_links.get(a.link).is_some_and(|l|
        summary.symbolic_associations.get(l.association).is_some_and(|a|summary.symbolic_classes.get(a.class).is_some_and(|c|c.supported_record)))));
    // Removing a completed Local premise preserves source association but changes the refusal.
    summary.symbolic_local_stores=normalized::Rows::new(&f.budget);
    let refused=execution::summary_symbolic::derive(&summary,&invocation,&f.budget).unwrap();
    assert!(refused.iter().all(|a|a.reason==obligation::ObligationKind::MissingEvidence));
}
