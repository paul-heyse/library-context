//! The dependency context a release's facts reference (DESIGN §3.2, §3.8; ADR-0014):
//! `context_modules`, `context_definitions` and their pinned `context_parameters`, the
//! `external_module` and `external_symbol` nodes.
//!
//! Every dependency module a mapped row referenced (by Pysa module id), and every module a public
//! export traces to, is resolved to its handle from the release's own import context and checked
//! to be the handle Pysa numbered. Those modules are then checked at `Require::Everything`, and
//! Pyrefly's own collectors describe them: each referenced function and class, and each
//! module-level definition an export names. So the definitions are an existence source independent
//! of the columns that reference them. Probe P1 (FastMCP 4.0.5, 2026-09-22): 301 modules, 1,276
//! referenced functions, all resolved and described.
#![deny(clippy::wildcard_enum_match_arm)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use cpg_schema::codebook::{
    DefinitionKind, ExtractionMode, Fidelity, Modality, ModuleOrigin, Origin,
};
use cpg_schema::id::{Id, recipe};
use cpg_schema::metrics::Stages;
use cpg_schema::models::{Catalog, Rule};
use cpg_schema::tables::{
    ContextClassMro, ContextClassMroRow, ContextDefinitions, ContextDefinitionsRow, ContextModules,
    ContextModulesRow, ContextParameters, ContextParametersRow,
};
use pyrefly::report::pysa::captured_variable::collect_captured_variables_for_module;
use pyrefly::report::pysa::class::PysaClassMro;
use pyrefly::report::pysa::context::{ModuleAnswersContext, ModuleContext, PysaResolver};
use pyrefly::report::pysa::export_module_definitions;
use pyrefly::report::pysa::override_graph::create_reversed_override_graph_for_module;
use pyrefly::report::pysa::scope::ScopeParent;
use pyrefly::state::require::Require;
use pyrefly::state::state::Transaction;
use pyrefly_build::handle::Handle;
use pyrefly_python::module_name::ModuleName;
use pyrefly_python::module_path::ModulePathDetails;

use crate::ExtractError;
use crate::config::{ExtractInput, PYREFLY_REV};
use crate::facts::{FactSink, Provenance, Surface, fact_row};
use crate::pysa_map::{ModuleRefs, parameter_shapes};

#[derive(Default)]
pub(crate) struct ContextOut {
    pub modules: Vec<ContextModulesRow>,
    pub definitions: Vec<ContextDefinitionsRow>,
    pub parameters: Vec<ContextParametersRow>,
    pub mro: Vec<ContextClassMroRow>,
}

fn provenance(fidelity: Fidelity) -> Provenance {
    Provenance {
        surface: Surface::PyreflyPysa,
        mode: ExtractionMode::NativeTraversal,
        origin: Origin::AnalyzerAssertion,
        modality: Modality::Definite,
        fidelity,
    }
}

/// A `context_modules` row joins Pyrefly's resolution of the module with Stage A's `RECORD` index
/// (its distribution and version): the extractor's own comparison of two sources (review O5).
fn module_provenance() -> Provenance {
    Provenance {
        surface: Surface::Compare,
        mode: ExtractionMode::RelationalDerivation,
        origin: Origin::AnalyzerAssertion,
        modality: Modality::Definite,
        fidelity: Fidelity::NativeStructural,
    }
}

/// Where the module's file comes from, and its path relative to that origin's root.
fn locate(handle: &Handle, input: &ExtractInput) -> (ModuleOrigin, Option<String>) {
    let relative = |p: &Path| -> (bool, String) {
        for root in &input.site_packages {
            if let Ok(rel) = p.strip_prefix(root) {
                return (true, rel.display().to_string());
            }
        }
        match p.strip_prefix(&input.release.root) {
            Ok(rel) => (false, rel.display().to_string()),
            Err(_) => (false, p.display().to_string()),
        }
    };
    match handle.path().details() {
        ModulePathDetails::FileSystem(p) => {
            let (site, rel) = relative(p);
            let origin = if site {
                ModuleOrigin::SitePackages
            } else {
                ModuleOrigin::SearchPath
            };
            (origin, Some(rel))
        }
        ModulePathDetails::Namespace(p) => (ModuleOrigin::Namespace, Some(relative(p).1)),
        ModulePathDetails::Memory(_) => (ModuleOrigin::Memory, None),
        ModulePathDetails::BundledTypeshed(p) => {
            (ModuleOrigin::BundledTypeshed, Some(p.display().to_string()))
        }
        ModulePathDetails::BundledTypeshedThirdParty(p) => (
            ModuleOrigin::BundledTypeshedThirdParty,
            Some(p.display().to_string()),
        ),
        ModulePathDetails::BundledThirdParty(p) => (
            ModuleOrigin::BundledThirdParty,
            Some(p.display().to_string()),
        ),
    }
}

/// The class chain above a definition, as `Outer.Inner.`; a function scope reads `<locals>.`.
fn prefix(parent: &ScopeParent, classes: &HashMap<u32, (String, &ScopeParent)>) -> String {
    match parent {
        ScopeParent::TopLevel => String::new(),
        ScopeParent::Function { .. } => "<locals>.".to_owned(),
        ScopeParent::Class { class_id } => match classes.get(&class_id.to_int()) {
            Some((name, up)) => format!("{}{name}.", prefix(up, classes)),
            None => "<class>.".to_owned(),
        },
    }
}

/// Resolve, check and describe the dependency modules the release references.
#[allow(
    clippy::too_many_arguments,
    reason = "the stage's inputs, each distinct"
)]
pub(crate) fn context_facts(
    txn: &mut Transaction<'_>,
    anchor: Option<&Handle>,
    refs: &ModuleRefs,
    export_origins: &[(Handle, String)],
    imported: &BTreeSet<String>,
    input: &ExtractInput,
    sink: &mut FactSink,
    stages: &mut Stages,
) -> Result<ContextOut, ExtractError> {
    // Module name → handle. A referenced module resolves from the release's import context and
    // must be the handle Pysa numbered; an export origin is already a handle.
    let mut handles: BTreeMap<String, Handle> = BTreeMap::new();
    {
        let module_ids = &txn
            .pysa_reporter()
            .expect("the reporter was installed before run")
            .module_ids;
        for (name, id) in refs.dependencies.borrow().iter() {
            let Some(anchor) = anchor else { break };
            let handle = txn
                .import_handle(anchor, ModuleName::from_str(name), None)
                .finding()
                .ok_or_else(|| {
                    ExtractError::Context(format!("dependency module {name} does not resolve"))
                })?;
            if module_ids.get_from_handle(&handle) != *id {
                return Err(ExtractError::Context(format!(
                    "dependency module {name} resolves to {}, not the module Pysa referenced",
                    handle.path()
                )));
            }
            handles.insert(name.clone(), handle);
        }
        for (handle, _) in export_origins {
            handles
                .entry(handle.module().to_string())
                .or_insert_with(|| handle.clone());
        }
    }
    // Modules only imported (C3's `imports_module`): a row each, no check and no definitions. A
    // module Pyrefly's finder cannot find (an optional dependency) keeps the finder's answer as a
    // `not_found` row with no node, so an import's reason is the provider's (C3 review F2).
    let mut imported_only: BTreeMap<String, Handle> = BTreeMap::new();
    let mut not_found: BTreeSet<String> = BTreeSet::new();
    if let Some(anchor) = anchor {
        for name in imported {
            if handles.contains_key(name) {
                continue;
            }
            match txn
                .import_handle(anchor, ModuleName::from_str(name), None)
                .finding()
            {
                Some(h) => {
                    imported_only.insert(name.clone(), h);
                }
                None => {
                    not_found.insert(name.clone());
                }
            }
        }
    }
    let exported: BTreeSet<(String, String)> = export_origins
        .iter()
        .map(|(h, n)| (h.module().to_string(), n.clone()))
        .collect();
    let referenced = refs.referenced.borrow().clone();
    let catalog = Catalog::committed().map_err(ExtractError::Context)?;
    let mut modeled_classes = BTreeSet::new();
    for compiled in &catalog.models {
        for rule in &compiled.model.rules {
            if let Rule::Exception {
                class, to_class, ..
            } = rule
            {
                for name in std::iter::once(class.as_str()).chain(to_class.as_deref()) {
                    if let Some((module, qualified)) = name.rsplit_once('.') {
                        modeled_classes.insert((module.to_owned(), qualified.to_owned()));
                    }
                }
            }
        }
    }

    let list: Vec<Handle> = handles.values().cloned().collect();
    txn.run(&list, Require::Everything, None);
    stages.mark("extract: pyrefly check (referenced dependencies)");
    let txn: &Transaction<'_> = txn;
    let module_ids = &txn
        .pysa_reporter()
        .expect("the reporter was installed before run")
        .module_ids;
    let owners = input.release.environment_library();

    let mut out = ContextOut::default();
    let mut all: BTreeMap<&String, (&Handle, bool)> =
        handles.iter().map(|(n, h)| (n, (h, true))).collect();
    for (n, h) in &imported_only {
        all.entry(n).or_insert((h, false));
    }
    for (name, (handle, described)) in all {
        let (origin, path) = locate(handle, input);
        let (distribution, version) = match (owners, origin, &path) {
            (Some(l), ModuleOrigin::SitePackages, Some(p)) => match l.owners.get(p) {
                Some(dist) => (
                    Some(dist.clone()),
                    l.distributions
                        .iter()
                        .find(|d| &d.name == dist)
                        .map(|d| d.version.clone()),
                ),
                None => (None, None),
            },
            _ => (None, None),
        };
        // Identity: the owning distribution and version, or Pyrefly's bundle and revision, or
        // (an unowned file) its content. Never where the environment sits.
        let (owner, owner_version) = match (&distribution, origin) {
            (Some(d), _) => (d.clone(), version.clone().unwrap_or_default()),
            (
                None,
                ModuleOrigin::BundledTypeshed
                | ModuleOrigin::BundledTypeshedThirdParty
                | ModuleOrigin::BundledThirdParty,
            ) => ("pyrefly-bundled".to_owned(), PYREFLY_REV.to_owned()),
            (
                None,
                ModuleOrigin::SitePackages
                | ModuleOrigin::SearchPath
                | ModuleOrigin::Namespace
                | ModuleOrigin::Memory
                | ModuleOrigin::NotFound,
            ) => {
                let text = txn
                    .get_module_info(handle)
                    .map(|m| m.lined_buffer().contents().to_string())
                    .unwrap_or_default();
                (
                    "unowned".to_owned(),
                    cpg_schema::id::content_digest(text.as_bytes()).hex(),
                )
            }
        };
        let module_node_id = recipe::external_module(&owner, &owner_version, name);
        out.modules.push(fact_row!(
            sink,
            ContextModules,
            module_provenance(),
            ContextModulesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id,
                module_name: name.clone(),
                origin,
                path,
                distribution,
                version,
            }
        ));

        if !described {
            continue;
        }
        let resolver = PysaResolver::new(txn, module_ids, handle.clone());
        let context = ModuleContext {
            answers_context: ModuleAnswersContext::create(handle.clone(), txn, module_ids),
            resolver: &resolver,
        };
        let captured = collect_captured_variables_for_module(&context);
        let overrides = create_reversed_override_graph_for_module(&context);
        let defs = export_module_definitions(&context, &captured, &overrides);
        let classes: HashMap<u32, (String, &ScopeParent)> = defs
            .class_definitions
            .iter()
            .map(|(cid, c)| (cid.to_int(), (c.name.clone(), &c.parent)))
            .collect();
        let mut emit = |kind_code: DefinitionKind,
                        key: String,
                        def_name: String,
                        parent: &ScopeParent,
                        signature_count: Option<i64>| {
            let top = matches!(parent, ScopeParent::TopLevel);
            let qualified_name = format!("{}{def_name}", prefix(parent, &classes));
            let wanted = referenced.contains(&(name.clone(), kind_code, key.clone()))
                || (top && exported.contains(&(name.clone(), def_name.clone())))
                || (kind_code == DefinitionKind::Class
                    && modeled_classes.contains(&(name.clone(), qualified_name.clone())));
            if !wanted {
                return;
            }
            let symbol_node_id = recipe::external_symbol(
                module_node_id,
                cpg_schema::Codebook::code(kind_code),
                &key,
            );
            out.definitions.push(fact_row!(
                sink,
                ContextDefinitions,
                provenance(Fidelity::ReportProjection),
                ContextDefinitionsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    symbol_node_id,
                    module_node_id,
                    module_name: name.clone(),
                    kind: kind_code,
                    key,
                    qualified_name,
                    name: def_name,
                    is_top_level: top,
                    signature_count,
                }
            ));
        };
        for (fid, def) in defs.function_definitions.as_map() {
            emit(
                DefinitionKind::Function,
                fid.serialize_to_string(),
                def.base.name.to_string(),
                &def.base.parent,
                Some(def.undecorated_signatures.len() as i64),
            );
        }
        for (cid, c) in &defs.class_definitions {
            emit(
                DefinitionKind::Class,
                cid.to_int().to_string(),
                c.name.clone(),
                &c.parent,
                None,
            );
        }
        let included: BTreeSet<Id> = out
            .definitions
            .iter()
            .filter(|d| d.module_node_id == module_node_id)
            .map(|d| d.symbol_node_id)
            .collect();
        for (cid, class) in &defs.class_definitions {
            let class_node_id = recipe::external_symbol(
                module_node_id,
                cpg_schema::Codebook::code(DefinitionKind::Class),
                &cid.to_int().to_string(),
            );
            if !included.contains(&class_node_id) {
                continue;
            }
            match &class.mro {
                PysaClassMro::Resolved(ancestors) if !ancestors.is_empty() => {
                    for (ordinal, ancestor) in ancestors.iter().enumerate() {
                        out.mro.push(fact_row!(
                            sink,
                            ContextClassMro,
                            provenance(Fidelity::ReportProjection),
                            ContextClassMroRow {
                                snapshot_id: Id::ZERO,
                                fact_id: Id::ZERO,
                                class_node_id,
                                module_node_id,
                                ordinal: Some(ordinal as i64),
                                ancestor_module: Some(
                                    ancestor.class.module_name().as_str().to_owned()
                                ),
                                ancestor_key: Some(ancestor.class_id.to_int().to_string()),
                                ancestor_name: Some(ancestor.class.name().as_str().to_owned()),
                                cyclic: false,
                            }
                        ));
                    }
                }
                PysaClassMro::Resolved(_) | PysaClassMro::Cyclic => {
                    out.mro.push(fact_row!(
                        sink,
                        ContextClassMro,
                        provenance(Fidelity::ReportProjection),
                        ContextClassMroRow {
                            snapshot_id: Id::ZERO,
                            fact_id: Id::ZERO,
                            class_node_id,
                            module_node_id,
                            ordinal: None,
                            ancestor_module: None,
                            ancestor_key: None,
                            ancestor_name: None,
                            cyclic: matches!(&class.mro, PysaClassMro::Cyclic),
                        }
                    ));
                }
            }
        }
        for (fid, def) in defs.function_definitions.as_map() {
            let symbol_node_id = recipe::external_symbol(
                module_node_id,
                cpg_schema::Codebook::code(DefinitionKind::Function),
                &fid.serialize_to_string(),
            );
            if !included.contains(&symbol_node_id) {
                continue;
            }
            for (signature_index, signature) in def.undecorated_signatures.iter().enumerate() {
                for shape in parameter_shapes(&signature.parameters) {
                    out.parameters.push(fact_row!(
                        sink,
                        ContextParameters,
                        provenance(Fidelity::ReportProjection),
                        ContextParametersRow {
                            snapshot_id: Id::ZERO,
                            fact_id: Id::ZERO,
                            symbol_node_id,
                            module_node_id,
                            signature_index: signature_index as i64,
                            form: shape.form,
                            ordinal: shape.ordinal,
                            kind: shape.kind,
                            name: shape.name,
                            required: shape.required,
                        }
                    ));
                }
            }
        }
    }
    for name in &not_found {
        out.modules.push(fact_row!(
            sink,
            ContextModules,
            module_provenance(),
            ContextModulesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id: recipe::external_module("not-found", "", name),
                module_name: name.clone(),
                origin: ModuleOrigin::NotFound,
                path: None,
                distribution: None,
                version: None,
            }
        ));
    }
    stages.mark("extract: dependency definitions");
    Ok(out)
}
