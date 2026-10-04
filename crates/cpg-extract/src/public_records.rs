//! Native public paths retain exact per-name origins, partial-known __all__ and fallback
//! candidates. Qualified module enumeration is independent from canonical source characterization.
#![deny(clippy::wildcard_enum_match_arm)]
use crate::natives::Natives;
use lctx_model::domain::{
    Id, ModelError,
    assertion::AssertionQualification,
    source::Module,
    symbols::{ExportKind, ExportOrigin, PublicNameObservation, ExportEnumerationObservation, ExportEnumerationStatus, ExportEnumerationBasis},
    attribution::Modality, value::{Literal,LiteralSet,LiteralSetMember}, Record,
};
use pyrefly::commands::coverage::collect::{
    EXCLUDED_MODULE_DUNDERS, is_public_module, is_public_name,
    trace_export_origin,
};
use pyrefly::export::{exports::{ExportLocation,Exports},definitions::{Definitions,DunderAllEntry}};
use pyrefly::state::state::Transaction;
use pyrefly_build::handle::Handle;
use pyrefly_python::symbol_kind::SymbolKind as PyreflySymbolKind;
use std::collections::BTreeSet;

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
    pub qualification: &'a AssertionQualification,
    pub native_parse_error: bool,
    pub native_exports_invalid: bool,
}
/// The public names of the `analyzed` modules, their origins, and each traced origin's (handle,
/// name), for the dependency context.
pub struct PublicNames {
    pub names: Vec<PublicNameObservation>,
    pub origins: Vec<ExportOrigin>,
    pub traced: Vec<(Handle, String)>,
    pub enumerations: Vec<ExportEnumerationObservation>,
    pub qualifications: Vec<AssertionQualification>,
    pub literals: Vec<Literal>,
    pub sets: Vec<(LiteralSet,Vec<LiteralSetMember>)>,
}
pub fn public_names(
    analyzed: &[Access<'_>], transaction: &Transaction<'_>, natives: &mut Natives,
) -> Result<PublicNames, ModelError> {
    let mut out=PublicNames {names:vec![],origins:vec![],traced:vec![],enumerations:vec![],qualifications:vec![],literals:vec![],sets:vec![]};
    for access in analyzed {
        let handle=access.handle;
        // No export adapter query is attempted when its retained parse is absent.
        let definitions=transaction.get_ast(handle).map(|ast|Definitions::new(&ast.body,handle.module(),handle.path().is_init(),*handle.sys_info()));
        if definitions.is_none() {
            let (set,members)=LiteralSet::of(std::iter::empty());
            out.enumerations.push(ExportEnumerationObservation {qualification:access.qualification.id(),access:access.module,names:set.id(),status:ExportEnumerationStatus::Unavailable,basis:ExportEnumerationBasis::Missing});
            out.sets.push((set,members));continue;
        }
        let data=transaction.get_exports_data(handle);
        let exports=transaction.get_exports(handle);
        // Reuse the provider's exact Definitions owner over retained native AST and SysInfo.
        let partial=definitions.as_ref().map(Exports::get_partially_known_dunder_all);
        let has_module_entries=definitions.as_ref().is_some_and(|d|d.dunder_all.entries.iter().any(|e|matches!(e,DunderAllEntry::Module(..))));
        let computed=data.unresolvable_dunder_all_range().is_some() || has_module_entries;
        let mut known:BTreeSet<ruff_python_ast::name::Name>=BTreeSet::new();
        let mut fallback=BTreeSet::new();
        let mut basis=ExportEnumerationBasis::Inferred;
        let mut status=ExportEnumerationStatus::Complete;
        if let Some(explicit)=data.explicit_dunder_all_names() {
            basis=ExportEnumerationBasis::ExplicitAll;known.extend(explicit.iter().cloned());
        } else if computed {
            basis=ExportEnumerationBasis::Computed;status=ExportEnumerationStatus::Partial;
            known.extend(partial.as_ref().into_iter().flat_map(|names|names.iter().cloned()));
        }
        if data.explicit_dunder_all_names().is_none() || computed {
            for (name,_) in exports.iter().filter(|(name,_)|is_public_name(name.as_str()) && !data.is_implicit_reexport(name)) {
                if computed {if !known.contains(name) {fallback.insert(name.clone());}}
                else {known.insert(name.clone());}
            }
        }
        if computed {status=ExportEnumerationStatus::Partial;basis=ExportEnumerationBasis::Computed;}
        if access.native_parse_error || access.native_exports_invalid || known.iter().any(|name|!exports.contains_key(name)) {status=ExportEnumerationStatus::Partial;basis=ExportEnumerationBasis::Invalid;}
        known.retain(|name|!name.as_str().is_empty() && !EXCLUDED_MODULE_DUNDERS.contains(&name.as_str()));
        fallback.retain(|name|!EXCLUDED_MODULE_DUNDERS.contains(&name.as_str()));
        if !is_public_module(handle.module()) {known.clear();fallback.clear();}
        let literals:Vec<Literal>=known.iter().map(|name|Literal::String {value:name.to_string().into()}).collect();
        let (set,members)=LiteralSet::of(literals.iter().map(Record::id));
        out.enumerations.push(ExportEnumerationObservation {qualification:access.qualification.id(),access:access.module,names:set.id(),status,basis});
        out.literals.extend(literals);out.sets.push((set,members));
        let candidate=AssertionQualification {modality:Modality::Candidate,..access.qualification.clone()};
        if !fallback.is_empty() {out.qualifications.push(candidate.clone());}
        for (names,qualified) in [(known,access.qualification),(fallback,&candidate)] {
            for name in names {
                let origin=match trace_export_origin(handle,name.clone(),transaction) {
                    Some((found,origin))=> {let kind=origin_kind(&found,&origin,transaction);out.traced.push((found.clone(),origin.to_string()));ExportOrigin::Traced {module:natives.module(&found.module().to_string(),found.path())?,name:origin.to_string(),kind}},
                    None=>ExportOrigin::Untraced,
                };
                out.names.push(PublicNameObservation {qualification:qualified.id(),access:access.module,name:name.to_string(),via_dunder_all:qualified.id()==access.qualification.id() && (data.explicit_dunder_all_names().is_some() || computed),origin:origin.id()});out.origins.push(origin);
            }
        }
    }
    // The old CLI coverage helper intentionally drops partial-known and wildcard paths.
    // It cannot reject additional paths supplied by these exact live native query surfaces.
    Ok(out)
}
