//! Public names by Pyrefly's own definition (cutover plan A9): each public analyzed module's names,
//! listed by `__all__` or by Pyrefly's definition of public, each with the origin
//! `trace_export_origin` finds, or untraced. The set is checked against `compute_public_fqns`, the
//! function behind `coverage report --public-only`: Pyrefly stays the only authority for "public".
#![deny(clippy::wildcard_enum_match_arm)]
use crate::natives::Natives;
use lctx_model::domain::{
    Id, ModelError,
    assertion::AssertionQualification,
    source::Module,
    symbols::{ExportKind, ExportOrigin, PublicNameObservation},
};
use pyrefly::commands::coverage::collect::{
    EXCLUDED_MODULE_DUNDERS, compute_public_fqns, is_public_module, is_public_name,
    trace_export_origin,
};
use pyrefly::export::exports::ExportLocation;
use pyrefly::state::state::Transaction;
use pyrefly_build::handle::Handle;
use pyrefly_python::symbol_kind::SymbolKind as PyreflySymbolKind;
use std::collections::BTreeSet;

fn invalid(message: String) -> ModelError {
    ModelError::Invalid(message)
}

/// Pyrefly's kind for an export's definition, as the model's code.
fn export_kind(kind: PyreflySymbolKind) -> ExportKind {
    match kind {
        PyreflySymbolKind::Module => ExportKind::Module,
        PyreflySymbolKind::Attribute => ExportKind::Attribute,
        PyreflySymbolKind::Variable => ExportKind::Variable,
        PyreflySymbolKind::Constant => ExportKind::Constant,
        PyreflySymbolKind::Parameter => ExportKind::Parameter,
        PyreflySymbolKind::TypeParameter => ExportKind::TypeParameter,
        PyreflySymbolKind::TypeAlias => ExportKind::TypeAlias,
        PyreflySymbolKind::Function => ExportKind::Function,
        PyreflySymbolKind::Method => ExportKind::Method,
        PyreflySymbolKind::Class => ExportKind::Class,
    }
}
/// The kind the origin module records for `name`, when it defines it.
fn origin_kind(
    handle: &Handle,
    name: &ruff_python_ast::name::Name,
    transaction: &Transaction<'_>,
) -> Option<ExportKind> {
    match transaction.get_exports(handle).get(name)? {
        ExportLocation::ThisModule(export) => export.symbol_kind.map(export_kind),
        ExportLocation::OtherModule(..) => None,
    }
}

/// One analyzed module whose public names are stated, under its qualification.
pub struct Access<'a> {
    pub handle: &'a Handle,
    pub module: Id<Module>,
    pub qualification: Id<AssertionQualification>,
}
/// The public names of the `analyzed` modules, their origins, and each traced origin's (handle,
/// name), for the dependency context.
pub struct PublicNames {
    pub names: Vec<PublicNameObservation>,
    pub origins: Vec<ExportOrigin>,
    pub traced: Vec<(Handle, String)>,
}
pub fn public_names(
    analyzed: &[Access<'_>],
    transaction: &Transaction<'_>,
    natives: &mut Natives,
) -> Result<PublicNames, ModelError> {
    let mut out = PublicNames {
        names: vec![],
        origins: vec![],
        traced: vec![],
    };
    let mut flattened = BTreeSet::new();
    for access in analyzed
        .iter()
        .filter(|a| is_public_module(a.handle.module()))
    {
        let handle = access.handle;
        let data = transaction.get_exports_data(handle);
        let exports = transaction.get_exports(handle);
        let (names, via_dunder_all): (Vec<_>, bool) = match data.explicit_dunder_all_names() {
            Some(all) => (all.iter().cloned().collect(), true),
            None => (
                exports
                    .iter()
                    .filter(|(name, location)| {
                        is_public_name(name.as_str())
                            && (matches!(location, ExportLocation::ThisModule(_))
                                || data.is_explicit_reexport(name))
                    })
                    .map(|(name, _)| name.clone())
                    .collect(),
                false,
            ),
        };
        let module = handle.module().to_string();
        for name in names {
            if EXCLUDED_MODULE_DUNDERS.contains(&name.as_str()) {
                continue;
            }
            flattened.insert(format!("{module}.{name}"));
            let origin = match trace_export_origin(handle, name.clone(), transaction) {
                Some((found, origin)) => {
                    flattened.insert(format!("{}.{origin}", found.module()));
                    let kind = origin_kind(&found, &origin, transaction);
                    out.traced.push((found.clone(), origin.to_string()));
                    ExportOrigin::Traced {
                        module: natives.module(&found.module().to_string(), found.path())?,
                        name: origin.to_string(),
                        kind,
                    }
                }
                None => ExportOrigin::Untraced,
            };
            out.names.push(PublicNameObservation {
                qualification: access.qualification,
                access: access.module,
                name: name.to_string(),
                via_dunder_all,
                origin: lctx_model::domain::Record::id(&origin),
            });
            out.origins.push(origin);
        }
    }
    let handles: Vec<Handle> = analyzed.iter().map(|a| a.handle.clone()).collect();
    let authority: BTreeSet<String> = compute_public_fqns(&handles, transaction)
        .0
        .into_iter()
        .collect();
    if flattened != authority {
        let differing: Vec<_> = flattened
            .symmetric_difference(&authority)
            .take(5)
            .cloned()
            .collect();
        return Err(invalid(format!(
            "public names differ from Pyrefly's public set: {}",
            differing.join(", ")
        )));
    }
    Ok(out)
}
