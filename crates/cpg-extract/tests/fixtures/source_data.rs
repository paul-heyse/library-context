//! Captured native and normalized inputs for retained source controls.
use crate::fixture::NativeFixture;
use lctx_model::domain::{analysis, execution, *};
pub fn data(f: &NativeFixture) -> execution::source_call_records::SourceCallData {
    let mut data = execution::source_call_records::SourceCallData::new(&f.budget);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        data.visit(name, batch).unwrap();
    }
    macro_rules! native{($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.data.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_inputs!(native);
    macro_rules! bound{($($field:ident:$ty:ty,)*)=>{$(data.visit(<$ty>::NAME,&<$ty as Record>::encode(&f.output.$field.iter().cloned().collect::<Vec<_>>()).unwrap()).unwrap();)*};}
    lctx_model::normalized_binding_outputs!(bound);
    let mut inventory = analysis::native::NativeInventory::new(&f.budget);
    for (name, batch) in f.tables.lock().unwrap().iter() {
        if analysis::native::NativeInventory::inputs()
            .iter()
            .any(|input| input.name() == *name)
        {
            inventory.visit(name, batch).unwrap();
        }
    }
    let inventory = inventory.collect().unwrap();
    data.visit(
        analysis::native::NativeAssertionPremise::NAME,
        &<analysis::native::NativeAssertionPremise as Record>::encode(
            &inventory.premises.iter().cloned().collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    data.visit(
        analysis::native::NativeQualification::NAME,
        &analysis::native::NativeQualification::encode(
            &inventory.qualifications.iter().cloned().collect::<Vec<_>>(),
        )
        .unwrap(),
    )
    .unwrap();
    data
}
