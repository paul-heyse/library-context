//! Public names by Pyrefly's own definition (DESIGN §4.2.3): (access path → origin) pairs from
//! `trace_export_origin`, checked on every run against `compute_public_fqns`, the function behind
//! `coverage report --public-only`. Pyrefly stays the only authority for "public".

use std::collections::BTreeSet;

use cpg_schema::codebook::{Fidelity, Modality, Origin};
use cpg_schema::id::Id;
use cpg_schema::tables::{PublicNames, PublicNamesRow};
use pyrefly::commands::coverage::collect::{
    EXCLUDED_MODULE_DUNDERS, compute_public_fqns, is_public_module, is_public_name,
    trace_export_origin,
};
use pyrefly::export::exports::ExportLocation;
use pyrefly::state::state::Transaction;
use pyrefly_build::handle::Handle;

use crate::ExtractError;
use crate::facts::{FactSink, Provenance, Surface, fact_row};

fn provenance() -> Provenance {
    Provenance {
        surface: Surface::PyreflyPublic,
        origin: Origin::AnalyzerAssertion,
        modality: Modality::Definite,
        fidelity: Fidelity::NativeStructural,
    }
}

pub(crate) fn public_names(
    handles: &[Handle],
    txn: &Transaction<'_>,
    sink: &mut FactSink,
) -> Result<Vec<PublicNamesRow>, ExtractError> {
    let mut rows = Vec::new();
    for handle in handles.iter().filter(|h| is_public_module(h.module())) {
        let data = txn.get_exports_data(handle);
        let exports = txn.get_exports(handle);
        let (names, via_all): (Vec<_>, bool) = match data.explicit_dunder_all_names() {
            Some(all) => (all.iter().cloned().collect(), true),
            None => (
                exports
                    .iter()
                    .filter(|(n, loc)| {
                        is_public_name(n.as_str())
                            && (matches!(loc, ExportLocation::ThisModule(_))
                                || data.is_explicit_reexport(n))
                    })
                    .map(|(n, _)| n.clone())
                    .collect(),
                false,
            ),
        };
        let module = handle.module().to_string();
        for name in names {
            if EXCLUDED_MODULE_DUNDERS.contains(&name.as_str()) {
                continue;
            }
            let origin_path = trace_export_origin(handle, name.clone(), txn)
                .map(|(h, n)| format!("{}.{}", h.module(), n));
            rows.push(fact_row!(
                sink,
                PublicNames,
                provenance(),
                PublicNamesRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    access_path: format!("{module}.{name}"),
                    access_module: module.clone(),
                    name: name.to_string(),
                    origin_path,
                    via_dunder_all: via_all,
                }
            ));
        }
    }
    let flattened: BTreeSet<String> = rows
        .iter()
        .flat_map(|r| std::iter::once(r.access_path.clone()).chain(r.origin_path.clone()))
        .collect();
    let (authority, _) = compute_public_fqns(handles, txn);
    let authority: BTreeSet<String> = authority.into_iter().collect();
    if flattened != authority {
        let diff: Vec<_> = flattened
            .symmetric_difference(&authority)
            .take(5)
            .cloned()
            .collect();
        return Err(ExtractError::PublicMismatch(diff.join(", ")));
    }
    Ok(rows)
}
