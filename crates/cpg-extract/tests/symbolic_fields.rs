#[path = "fixtures/transfer_composition.rs"]
mod fixture;
use lctx_model::domain::{calls::*, flow::*, symbols::*, value::*, *};
#[tokio::test]
async fn inspect_symbolic_native_shape() {
    for name in ["transfer_alternatives", "catalog"] {
        let captured = fixture::native_from(name).await;
        for row in captured.rows::<ClassTraitObservation>() {
            let symbol = captured.data.symbols.get(row.symbol).unwrap();
            if ["Options", "RecordHolder", "Holder", "DirectOptions"]
                .contains(&symbol.name.as_str())
            {
                eprintln!("CLASS {} {:?}", symbol.name, row);
            }
        }
        for row in captured.rows::<FunctionTraitObservation>() {
            let symbol = captured.data.symbols.get(row.symbol).unwrap();
            if let Some(c) = row
                .defining_class
                .and_then(|c| captured.data.symbols.get(c))
            {
                if ["Options", "RecordHolder", "Holder", "DirectOptions"].contains(&c.name.as_str())
                {
                    eprintln!("FUNCTION {}.{} {:?}", c.name, symbol.name, row);
                    for s in captured
                        .data
                        .signatures
                        .iter()
                        .filter(|s| s.symbol == row.symbol)
                    {
                        eprintln!("SIGNATURE {:?}", s);
                    }
                }
            }
        }
        let places = captured.rows::<Place>();
        let roots = captured.rows::<PlaceRoot>();
        let uses = captured.rows::<FlowUse>();
        for v in captured.rows::<FlowValueObservation>() {
            let u = uses.iter().find(|u| u.id() == v.use_).unwrap();
            let p = places.iter().find(|p| p.id() == u.place).unwrap();
            let r = roots.iter().find(|r| r.id() == p.root).unwrap();
            if matches!(r, PlaceRoot::Field { .. }) || v.kind == FlowSinkKind::Definition {
                eprintln!("VALUE {:?} {:?}", r, v);
            }
        }
    }
}
