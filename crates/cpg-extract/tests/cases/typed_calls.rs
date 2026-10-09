//! Known answers for native call events, independent of producer iteration order (A10).
use crate::typed_driver;
use crate::inspector;
use lctx_model::domain::{
    assertion::*, attribution::*, calls::*, source::*, syntax::SubjectBoundary, *,
};
use typed_driver::{files, rows, run};
inspector!(
    Calls,
    lctx_model::domain::calls::ProviderCallable,
    lctx_model::domain::calls::ProviderCallSite,
    lctx_model::domain::calls::ProviderSymbol,
    lctx_model::domain::calls::CallTarget,
    lctx_model::domain::calls::CallResolution,
    lctx_model::domain::calls::CallChannel,
    lctx_model::domain::calls::CallDestination,
    lctx_model::domain::calls::Receiver,
    lctx_model::domain::assertion::AssertionQualification,
    lctx_model::domain::source::SourceArtifact,
    lctx_model::domain::source::Occurrence,
    lctx_model::domain::attribution::ProviderCoverage,
    lctx_model::domain::syntax::SubjectBoundary,
    lctx_model::domain::calls::CallOriginStep,
    lctx_model::domain::source::Module,
    lctx_model::domain::calls::ProviderModule
);
#[tokio::test]
async fn variants_keep_channels_phases_and_native_owners() {
    let tables = typed_driver::Tables::default();
    run(&files("pysa_variants"), Calls(tables.clone()))
        .await
        .unwrap();
    let targets = rows::<CallTarget>(&tables);
    let channels = rows::<CallChannel>(&tables);
    let qualifications = rows::<AssertionQualification>(&tables);
    let destinations = rows::<CallDestination>(&tables);
    assert!(!targets.is_empty());
    let mut higher = 0;
    let mut dispatch = 0;
    for target in &targets {
        let q = qualifications
            .iter()
            .find(|q| q.id() == target.qualification)
            .unwrap();
        let channel = channels.iter().find(|c| c.id() == target.channel).unwrap();
        if matches!(channel, CallChannel::HigherOrder { .. }) {
            higher += 1;
            assert_eq!(q.modality, Modality::Potential);
        }
        if matches!(
            destinations
                .iter()
                .find(|d| d.id() == target.destination)
                .unwrap(),
            CallDestination::Overrides { .. }
        ) {
            dispatch += 1;
            assert_ne!(q.modality, Modality::Definite);
            assert!(target.receiver_class.is_some());
        }
    }
    assert!(higher > 0, "map callback");
    assert!(dispatch > 0, "virtual method");
    assert!(targets.iter().any(|t| t.phase == CallPhase::New));
    assert!(targets.iter().any(|t| t.phase == CallPhase::Init));
    let callers = rows::<ProviderCallable>(&tables);
    assert!(
        callers
            .iter()
            .any(|c| matches!(c, ProviderCallable::ModuleBody { .. }))
    );
    assert!(rows::<ProviderSymbol>(&tables).iter().all(|s| !matches!(
        s.kind,
        SymbolKind::ModuleBody | SymbolKind::ClassBody | SymbolKind::DecoratorApplication
    )));
}
#[tokio::test]
async fn annotations_without_call_graph_evidence_disclose_a_boundary() {
    let files = std::collections::BTreeMap::from([(
        "example.py".into(),
        b"def annotation(): return int\ndef f(x: annotation()): return x\n".to_vec(),
    )]);
    let tables = typed_driver::Tables::default();
    run(&files, Calls(tables.clone())).await.unwrap();
    let boundaries = rows::<SubjectBoundary>(&tables);
    assert!(boundaries.iter().any(|b| b.family == FactFamily::Calls
        && b.reason == lctx_model::domain::obligation::ObligationKind::OutsideProviderModel));
    assert!(
        rows::<ProviderCoverage>(&tables)
            .iter()
            .any(|c| c.family == FactFamily::Calls && c.status == CoverageStatus::Partial)
    );
}

#[tokio::test]
async fn fstring_events_share_coordinates_without_merging_their_origins() {
    let input = std::collections::BTreeMap::from([(
        "example.py".into(),
        b"def g() -> str: return 'x'\nx = f'{g()}'\n".to_vec(),
    )]);
    let tables = typed_driver::Tables::default();
    run(&input, Calls(tables.clone())).await.unwrap();
    let sites = rows::<ProviderCallSite>(&tables);
    let origins = rows::<CallOriginStep>(&tables);
    let call = sites
        .iter()
        .find(|s| s.kind == PysaSiteKind::Regular && s.callee == PysaCalleeKind::Call)
        .unwrap();
    let stringify = sites
        .iter()
        .find(|s| s.kind == PysaSiteKind::FormatStringStringify)
        .unwrap();
    assert_eq!(call.site, stringify.site);
    assert_ne!(call.origin, stringify.origin);
    assert!(
        origins
            .iter()
            .any(|s| s.origin == stringify.origin && s.step == OriginStep::FormatStringStringify)
    );
    assert!(
        sites
            .iter()
            .any(|s| s.kind == PysaSiteKind::FormatStringArtificial)
    );
}

#[tokio::test]
async fn variant_modalities_keep_potential_remainders_and_conditional_properties() {
    let tables = typed_driver::Tables::default();
    run(&files("pysa_variants"), Calls(tables.clone()))
        .await
        .unwrap();
    let sites = rows::<ProviderCallSite>(&tables);
    let targets = rows::<CallTarget>(&tables);
    let qualifications = rows::<AssertionQualification>(&tables);
    let destinations = rows::<CallDestination>(&tables);
    let mut conditional = 0;
    let mut remainders = 0;
    for target in &targets {
        let q = qualifications
            .iter()
            .find(|q| q.id() == target.qualification)
            .unwrap();
        for site in sites
            .iter()
            .filter(|s| s.site == target.site && s.origin == target.origin)
        {
            if matches!(
                site.callee,
                PysaCalleeKind::Identifier | PysaCalleeKind::AttributeAccess
            ) && target.phase == CallPhase::Call
            {
                assert_eq!(q.modality, Modality::Potential);
                if matches!(
                    destinations
                        .iter()
                        .find(|d| d.id() == target.destination)
                        .unwrap(),
                    CallDestination::Unresolved { .. }
                ) {
                    remainders += 1;
                }
            }
            if site.is_attribute == Some(true)
                && matches!(
                    target.phase,
                    CallPhase::PropertyGet | CallPhase::PropertySet
                )
            {
                assert_ne!(q.modality, Modality::Definite);
                conditional += 1;
            }
        }
    }
    assert!(conditional > 0);
    assert!(remainders > 0);
    assert!(
        sites
            .iter()
            .any(|s| s.kind == PysaSiteKind::ArtificialAttributeAccess)
    );
}

#[tokio::test]
async fn receiver_classes_and_targets_keep_resolved_stub_and_source_files() {
    let tables = typed_driver::Tables::default();
    run(&files("pysa_keys"), Calls(tables.clone()))
        .await
        .unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    let targets = rows::<CallTarget>(&tables);
    let destinations = rows::<CallDestination>(&tables);
    let occurrences = rows::<Occurrence>(&tables);
    let artifacts = rows::<SourceArtifact>(&tables);
    let modules = rows::<Module>(&tables);
    let native_modules = rows::<ProviderModule>(&tables);
    let symbol = |id| symbols.iter().find(|s| s.id() == id).unwrap();
    let receivers: std::collections::BTreeSet<_> = targets
        .iter()
        .filter(|t| {
            destinations
                .iter()
                .find(|d| d.id() == t.destination)
                .unwrap()
                .symbol()
                .is_some_and(|s| symbol(s).name == "m")
        })
        .filter_map(|t| t.receiver_class)
        .collect();
    assert_eq!(receivers.len(), 2);
    let path = |source| {
        artifacts
            .iter()
            .find(|a| a.id() == source)
            .unwrap()
            .path
            .clone()
    };
    let mut observed = std::collections::BTreeMap::new();
    for target in targets {
        let Some(id) = destinations
            .iter()
            .find(|d| d.id() == target.destination)
            .unwrap()
            .symbol()
        else {
            continue;
        };
        let s = symbol(id);
        if !["f", "g"].contains(&s.name.as_str()) {
            continue;
        }
        let ProviderModule::Acquired { module } =
            native_modules.iter().find(|m| m.id() == s.module).unwrap()
        else {
            continue;
        };
        let callee = modules.iter().find(|m| m.id() == *module).unwrap();
        let caller = occurrences.iter().find(|o| o.id() == target.site).unwrap();
        observed.insert((path(caller.source), s.name.clone()), path(callee.source));
    }
    assert_eq!(
        observed[&("keys/use_dual.py".into(), "g".into())],
        "keys/dual.pyi"
    );
    assert_eq!(
        observed[&("keys/dual.py".into(), "f".into())],
        "keys/dual.py"
    );
}

#[tokio::test]
async fn import_cycle_return_types_reach_builtins_and_repeated_content_is_identical() {
    let tables = typed_driver::Tables::default();
    let first = run(&files("import_cycle"), Calls(tables.clone()))
        .await
        .unwrap();
    let symbols = rows::<ProviderSymbol>(&tables);
    for name in ["bit_length", "upper"] {
        let symbol = symbols.iter().find(|s| s.name == name).unwrap();
        assert!(
            rows::<CallDestination>(&tables)
                .iter()
                .any(|d| matches!(d,CallDestination::Resolved { symbol:id } if *id == symbol.id()))
        );
    }
    let second = run(
        &files("import_cycle"),
        Calls(typed_driver::Tables::default()),
    )
    .await
    .unwrap();
    assert_eq!(first, second);
}

#[tokio::test]
async fn typed_native_variant_table_snapshot() {
    let tables = typed_driver::Tables::default();
    run(&files("pysa_variants"), Calls(tables.clone()))
        .await
        .unwrap();
    let occurrences = rows::<Occurrence>(&tables);
    let symbols = rows::<ProviderSymbol>(&tables);
    let qualifications = rows::<AssertionQualification>(&tables);
    let sites = rows::<ProviderCallSite>(&tables);
    let steps = rows::<CallOriginStep>(&tables);
    let mut lines = Vec::new();
    for target in rows::<CallTarget>(&tables) {
        let occurrence = occurrences.iter().find(|o| o.id() == target.site).unwrap();
        let site = sites
            .iter()
            .find(|s| s.site == target.site && s.origin == target.origin)
            .unwrap();
        let destination = rows::<CallDestination>(&tables)
            .into_iter()
            .find(|d| d.id() == target.destination)
            .unwrap();
        let destination = match destination {
            CallDestination::Resolved { symbol } | CallDestination::Overrides { symbol } => {
                format!(
                    "{}:{}",
                    if matches!(destination, CallDestination::Overrides { .. }) {
                        "overrides"
                    } else {
                        "resolved"
                    },
                    symbols.iter().find(|s| s.id() == symbol).unwrap().name
                )
            }
            other => format!("{other:?}"),
        };
        let mut origin: Vec<_> = steps.iter().filter(|s| s.origin == target.origin).collect();
        origin.sort_by_key(|s| s.ordinal);
        let origin = origin
            .iter()
            .map(|s| format!("{:?}:{:?}", s.step, s.index))
            .collect::<Vec<_>>()
            .join("/");
        let q = qualifications
            .iter()
            .find(|q| q.id() == target.qualification)
            .unwrap();
        let channel = rows::<CallChannel>(&tables)
            .into_iter()
            .find(|c| c.id() == target.channel)
            .unwrap();
        lines.push(format!(
            "{}-{} {:?}/{:?} [{origin}] {:?} {channel:?} {destination} {:?} attr={:?}",
            occurrence.start,
            occurrence.end,
            site.kind,
            site.callee,
            target.phase,
            q.modality,
            site.is_attribute
        ));
    }
    lines.sort();
    insta::assert_snapshot!(lines.join("\n"));
}

#[tokio::test]
async fn unicode_bom_crlf_native_calls_attach_without_call_boundaries() {
    let tables = typed_driver::Tables::default();
    run(&files("unicode_bom"), Calls(tables.clone()))
        .await
        .unwrap();
    assert!(
        rows::<lctx_model::domain::syntax::SubjectBoundary>(&tables)
            .iter()
            .all(|b| b.family != lctx_model::domain::attribution::FactFamily::Calls)
    );
}
