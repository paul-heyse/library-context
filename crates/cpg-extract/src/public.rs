//! Public names by Pyrefly's own definition (DESIGN §4.2.3): (access path → origin) pairs from
//! `trace_export_origin`, checked on every run against `compute_public_fqns`, the function behind
//! `coverage report --public-only`. Pyrefly stays the only authority for "public".

use std::collections::{BTreeSet, HashMap};

use cpg_schema::codebook::{ExtractionMode, Fidelity, Modality, Origin, SymbolKind};
use cpg_schema::id::Id;
use cpg_schema::tables::{PublicNames, PublicNamesRow};
use pyrefly::commands::coverage::collect::{
    EXCLUDED_MODULE_DUNDERS, compute_public_fqns, is_public_module, is_public_name,
    trace_export_origin,
};
use pyrefly::export::exports::ExportLocation;
use pyrefly::state::state::Transaction;
use pyrefly_build::handle::Handle;
use pyrefly_python::symbol_kind::SymbolKind as PyreflySymbolKind;

use crate::ExtractError;
use crate::facts::{FactSink, Provenance, Surface, fact_row};

/// Pyrefly's symbol kind as the codebook value: an exhaustive match (§3.5).
#[deny(clippy::wildcard_enum_match_arm)]
fn symbol_kind(k: PyreflySymbolKind) -> SymbolKind {
    match k {
        PyreflySymbolKind::Module => SymbolKind::Module,
        PyreflySymbolKind::Attribute => SymbolKind::Attribute,
        PyreflySymbolKind::Variable => SymbolKind::Variable,
        PyreflySymbolKind::Constant => SymbolKind::Constant,
        PyreflySymbolKind::Parameter => SymbolKind::Parameter,
        PyreflySymbolKind::TypeParameter => SymbolKind::TypeParameter,
        PyreflySymbolKind::TypeAlias => SymbolKind::TypeAlias,
        PyreflySymbolKind::Function => SymbolKind::Function,
        PyreflySymbolKind::Method => SymbolKind::Method,
        PyreflySymbolKind::Class => SymbolKind::Class,
    }
}

/// The kind Pyrefly records for `name` where the origin module defines it.
fn origin_kind(
    handle: &Handle,
    name: &ruff_python_ast::name::Name,
    txn: &Transaction<'_>,
) -> Option<SymbolKind> {
    match txn.get_exports(handle).get(name)? {
        ExportLocation::ThisModule(e) => e.symbol_kind.map(symbol_kind),
        ExportLocation::OtherModule(..) => None,
    }
}

fn provenance() -> Provenance {
    Provenance {
        surface: Surface::PyreflyPublic,
        mode: ExtractionMode::NativeTraversal,
        origin: Origin::AnalyzerAssertion,
        modality: Modality::Definite,
        fidelity: Fidelity::NativeStructural,
    }
}

/// A module outside the release a public export traces to, and the name in it.
pub(crate) type ExportOrigin = (Handle, String);

/// `release_files` maps each module handle of the release to its `source_files` node: the file an
/// origin traces to, since a `.py` and its `.pyi` share a module name. Also returns the modules
/// outside the release that origins trace to (their definitions become `context_definitions`).
pub(crate) fn public_names(
    handles: &[Handle],
    release_files: &HashMap<Handle, Id>,
    txn: &Transaction<'_>,
    sink: &mut FactSink,
) -> Result<(Vec<PublicNamesRow>, Vec<ExportOrigin>), ExtractError> {
    let mut rows = Vec::new();
    let mut outside = Vec::new();
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
            let origin = trace_export_origin(handle, name.clone(), txn);
            let origin_path = origin
                .as_ref()
                .map(|(h, n)| format!("{}.{}", h.module(), n));
            let origin_module_node_id = origin
                .as_ref()
                .and_then(|(h, _)| release_files.get(h).copied());
            if let Some((h, n)) = &origin
                && !release_files.contains_key(h)
            {
                outside.push((h.clone(), n.to_string()));
            }
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
                    origin_module_node_id,
                    via_dunder_all: via_all,
                    origin_module: origin.as_ref().map(|(h, _)| h.module().to_string()),
                    origin_name: origin.as_ref().map(|(_, n)| n.to_string()),
                    access_module_node_id: release_files[handle],
                    origin_symbol_kind: origin.as_ref().and_then(|(h, n)| origin_kind(h, n, txn)),
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
    Ok((rows, outside))
}
