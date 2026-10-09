//! Exact linked class traits add native support without changing the report assertion.
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    assertion::ProviderSurface,
    attribution::{ExtractionMode, Fidelity, Origin, ProviderRun},
    calls::{ProviderSymbol, SymbolKind},
    symbols::{ClassTraitObservation, ClassTraitSupport},
    *,
};
use typed_driver::{files, rows, run};
inspector!(
    Traits,
    lctx_model::domain::symbols::ClassTraitObservation,
    lctx_model::domain::symbols::ClassTraitSupport,
    lctx_model::domain::calls::ProviderSymbol,
    lctx_model::domain::attribution::ProviderRun,
    lctx_model::domain::assertion::ProviderSurface
);

#[tokio::test]
async fn exact_native_class_traits_preserve_report_provenance_and_all_flags() {
    let tables = typed_driver::Tables::default();
    run(&files("native_class_traits"), Traits(tables.clone()))
        .await
        .unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let traits = rows::<ClassTraitObservation>(&tables);
    let supports = rows::<ClassTraitSupport>(&tables);
    let runs = rows::<ProviderRun>(&tables);
    let surfaces = rows::<ProviderSurface>(&tables);
    // Independent declaration oracle: synthesized is the native BindingClass variant,
    // rather than the presence of an assignment or a recognized name.
    for (name, synthesized, dataclass, named_tuple, typed_dict) in [
        ("Plain", false, false, false, false),
        ("Data", false, true, false, false),
        ("TupleRecord", false, false, true, false),
        ("DictRecord", false, false, false, true),
        ("FunctionalTuple", true, false, true, false),
        ("FunctionalDict", true, false, false, true),
    ] {
        let symbol = symbols
            .iter()
            .find(|s| s.name == name && s.kind == SymbolKind::Class)
            .unwrap();
        let matched = traits
            .iter()
            .filter(|t| t.symbol == symbol.id())
            .collect::<Vec<_>>();
        assert_eq!(
            matched.len(),
            1,
            "the same qualified assertion is written only once: {name}"
        );
        let row = matched[0];
        assert_eq!(
            (
                row.synthesized,
                row.dataclass,
                row.named_tuple,
                row.typed_dict
            ),
            (synthesized, dataclass, named_tuple, typed_dict),
            "{name}"
        );
        let mut fidelity = supports
            .iter()
            .filter(|s| s.assertion == row.id())
            .map(|s| s.fidelity as i16)
            .collect::<Vec<_>>();
        fidelity.sort();
        assert_eq!(
            fidelity,
            vec![
                Fidelity::NativeStructural as i16,
                Fidelity::ReportProjection as i16
            ],
            "{name} retains distinct native and report support"
        );
        let native = supports
            .iter()
            .find(|s| s.assertion == row.id() && s.fidelity == Fidelity::NativeStructural)
            .unwrap();
        assert_eq!(
            (native.origin, native.mode),
            (Origin::AnalyzerAssertion, ExtractionMode::NativeTraversal)
        );
        let run = runs.iter().find(|r| r.id() == native.run).unwrap();
        let surface = surfaces.iter().find(|s| s.id() == native.surface).unwrap();
        assert_eq!(
            (run.provider, run.context),
            (symbol.provider, symbol.context)
        );
        assert_eq!(
            (surface.provider, surface.family),
            (symbol.provider, attribution::FactFamily::Signatures)
        );
    }
}
