//! Pyrefly's Pysa collectors → `pyrefly-pysa` raw tables (DESIGN §4.2.3).
//!
//! Every Pyrefly enum is matched exhaustively with no wildcard arm: a new upstream variant fails
//! the build instead of degrading silently (§3.5, DM-42).
#![deny(clippy::wildcard_enum_match_arm)]

use std::collections::{HashMap, HashSet};

use cpg_schema::codebook::{
    AncestryRelation, Fidelity, ImplicitReceiver, InvocationPhase, Modality, Origin, ParameterKind,
    PysaCalleeKind, PysaSiteKind, PysaTargetKind, PysaUnresolvedReason, SignatureForm,
};
use cpg_schema::id::Id;
use cpg_schema::tables::{
    ClassAncestry, ClassAncestryRow, ParameterSemantics, ParameterSemanticsRow, PysaCalls,
    PysaCallsRow, PysaFunctions, PysaFunctionsRow,
};
use pyrefly::report::pysa::call_graph::{
    CallCallees, ExpressionCallees, ExpressionIdentifier, ImplicitReceiver as PyImplicitReceiver,
    OriginKind, PysaCallTarget, Target, Unresolved, UnresolvedReason,
};
use pyrefly::report::pysa::class::{ClassRef, PysaClassMro};
use pyrefly::report::pysa::function::{FunctionParameter, FunctionParameters, FunctionRef};
use pyrefly::report::pysa::location::PysaLocation;
use pyrefly::report::pysa::module::ModuleId;
use pyrefly::report::pysa::types::PysaType;
use pyrefly::report::pysa::{PysaModuleCallGraphs, PysaModuleDefinitions};
use pyrefly_python::module_name::ModuleName;
use ruff_source_file::{LineIndex, OneIndexed, PositionEncoding, SourceLocation};
use ruff_text_size::TextSize;

use crate::facts::{FactSink, Provenance, Surface, fact_row};

#[derive(Default)]
pub(crate) struct PysaOut {
    pub functions: Vec<PysaFunctionsRow>,
    pub parameters: Vec<ParameterSemanticsRow>,
    pub ancestry: Vec<ClassAncestryRow>,
    pub calls: Vec<PysaCallsRow>,
    /// Byte ranges of regular call sites Pysa described (the S5 join key).
    pub regular_call_ranges: HashSet<(i64, i64)>,
}

/// Converts Pysa locations (1-based line, 1-based UTF-8 byte column) back to byte offsets with
/// the module's own line index: the exact inverse of `PysaLocation::from_text_range` (§3.4).
pub(crate) struct Locator<'m> {
    pub line_index: &'m LineIndex,
    pub text: &'m str,
}

impl Locator<'_> {
    fn offset(&self, line: u32, col: u32) -> i64 {
        let at: TextSize = self.line_index.offset(
            SourceLocation {
                line: OneIndexed::new(line as usize).unwrap_or(OneIndexed::MIN),
                character_offset: OneIndexed::new(col as usize).unwrap_or(OneIndexed::MIN),
            },
            self.text,
            PositionEncoding::Utf8,
        );
        i64::from(at.to_u32())
    }

    fn range(&self, loc: &PysaLocation) -> (i64, i64) {
        (
            self.offset(loc.line(), loc.col()),
            self.offset(loc.end_line(), loc.end_col()),
        )
    }
}

fn pysa(origin: Origin, modality: Modality) -> Provenance {
    Provenance {
        surface: Surface::PyreflyPysa,
        origin,
        modality,
        fidelity: Fidelity::ReportProjection,
    }
}

/// References across modules, keyed as Pysa keys them: by module id, never by name alone
/// (DESIGN §4.2.3, review F3). A file of the release reads `@<release-relative path>`, because a
/// `.py` and its `.pyi` share a module name; any other module reads as its name, which resolves
/// to one file in a context. `@` never starts a module name.
pub(crate) struct ModuleRefs {
    pub release_files: HashMap<ModuleId, String>,
}

impl ModuleRefs {
    fn module(&self, id: ModuleId, name: ModuleName) -> String {
        self.release_files
            .get(&id)
            .map_or_else(|| name.to_string(), |path| format!("@{path}"))
    }

    /// `<module ref>:<name>#<ClassId>`: the id keys it, the name is for reading.
    fn class(&self, c: &ClassRef) -> String {
        format!(
            "{}:{}#{}",
            self.module(c.module_id, c.class.module_name()),
            c.class.name(),
            c.class_id.to_int()
        )
    }

    fn function(&self, f: &FunctionRef) -> String {
        format!(
            "{}::{}",
            self.module(f.module_id, f.module_name),
            f.function_id.serialize_to_string()
        )
    }
}

/// The file being mapped.
pub(crate) struct Here<'m> {
    pub module_name: &'m str,
    pub module_node_id: Id,
    pub refs: &'m ModuleRefs,
    pub loc: Locator<'m>,
}

/// Our own rendering of Pysa's `OriginKind` (the text upstream's `Display` gives): an exhaustive
/// match, so a new kind fails the build instead of passing through unseen (§3.5, review F5).
fn origin_kind(k: &OriginKind) -> String {
    let leaf = match k {
        OriginKind::GetAttrConstantLiteral => "get-attr-constant-literal",
        OriginKind::ComparisonOperator => "comparison",
        OriginKind::GeneratorIter => "generator-iter",
        OriginKind::GeneratorNext => "generator-next",
        OriginKind::WithEnter => "with-enter",
        OriginKind::ForDecoratedTarget => "for-decorated-target",
        OriginKind::SubscriptGetItem => "subscript-get-item",
        OriginKind::SubscriptSetItem => "subscript-set-item",
        OriginKind::BinaryOperator => "binary",
        OriginKind::AugmentedAssignDunderCall => "augmented-assign-dunder-call",
        OriginKind::AugmentedAssignRHS => "augmented-assign-rhs",
        OriginKind::AugmentedAssignStatement => "augmented-assign-statement",
        OriginKind::ForIter => "for-iter",
        OriginKind::ForNext => "for-next",
        OriginKind::ForAssign => "for-assign",
        OriginKind::ReprCall => "repr-call",
        OriginKind::AbsCall => "abs-call",
        OriginKind::IterCall => "iter-call",
        OriginKind::NextCall => "next-call",
        OriginKind::StrCallToDunderMethod => "str-call-to-dunder-method",
        OriginKind::Slice => "slice",
        OriginKind::ChainedAssign { index } => return format!("chained-assign:{index}"),
        OriginKind::Nested { head, tail } => {
            return format!("{}>{}", origin_kind(tail), origin_kind(head));
        }
    };
    leaf.to_owned()
}

fn reason(r: UnresolvedReason) -> PysaUnresolvedReason {
    match r {
        UnresolvedReason::LambdaArgument => PysaUnresolvedReason::LambdaArgument,
        UnresolvedReason::UnexpectedPyreflyTarget => PysaUnresolvedReason::UnexpectedPyreflyTarget,
        UnresolvedReason::EmptyPyreflyCallTarget => PysaUnresolvedReason::EmptyPyreflyCallTarget,
        UnresolvedReason::UnknownClassField => PysaUnresolvedReason::UnknownClassField,
        UnresolvedReason::ClassFieldOnlyExistInObject => {
            PysaUnresolvedReason::ClassFieldOnlyExistInObject
        }
        UnresolvedReason::UnsupportedFunctionTarget => {
            PysaUnresolvedReason::UnsupportedFunctionTarget
        }
        UnresolvedReason::UnexpectedDefiningClass => PysaUnresolvedReason::UnexpectedDefiningClass,
        UnresolvedReason::UnexpectedInitMethod => PysaUnresolvedReason::UnexpectedInitMethod,
        UnresolvedReason::UnexpectedNewMethod => PysaUnresolvedReason::UnexpectedNewMethod,
        UnresolvedReason::UnexpectedCalleeExpression => {
            PysaUnresolvedReason::UnexpectedCalleeExpression
        }
        UnresolvedReason::UnresolvedMagicDunderAttr => {
            PysaUnresolvedReason::UnresolvedMagicDunderAttr
        }
        UnresolvedReason::UnresolvedMagicDunderAttrDueToNoBase => {
            PysaUnresolvedReason::UnresolvedMagicDunderAttrDueToNoBase
        }
        UnresolvedReason::UnresolvedMagicDunderAttrDueToNoAttribute => {
            PysaUnresolvedReason::UnresolvedMagicDunderAttrDueToNoAttribute
        }
        UnresolvedReason::Mixed => PysaUnresolvedReason::Mixed,
    }
}

fn implicit_receiver(r: PyImplicitReceiver) -> ImplicitReceiver {
    match r {
        PyImplicitReceiver::TrueWithClassReceiver => ImplicitReceiver::TrueWithClassReceiver,
        PyImplicitReceiver::TrueWithObjectReceiver => ImplicitReceiver::TrueWithObjectReceiver,
        PyImplicitReceiver::False => ImplicitReceiver::False,
    }
}

fn unresolved(u: &Unresolved) -> Option<PysaUnresolvedReason> {
    match u {
        Unresolved::False => None,
        Unresolved::True(r) => Some(reason(*r)),
    }
}

fn annotation(refs: &ModuleRefs, t: &PysaType) -> (Vec<String>, Option<bool>, Vec<String>) {
    let mut classes: Vec<String> = t
        .class_names
        .classes
        .iter()
        .map(|c| refs.class(&c.class))
        .collect();
    classes.sort();
    classes.dedup();
    let s = &t.scalar_type_properties;
    let scalar = [
        (s.is_bool, "bool"),
        (s.is_int, "int"),
        (s.is_float, "float"),
        (s.is_enum, "enum"),
    ]
    .iter()
    .filter(|(on, _)| *on)
    .map(|(_, n)| (*n).to_owned())
    .collect();
    (classes, Some(t.class_names.is_exhaustive), scalar)
}

pub(crate) fn map_definitions(
    here: &Here<'_>,
    defs: &PysaModuleDefinitions,
    sink: &mut FactSink,
    out: &mut PysaOut,
) {
    let refs = here.refs;
    for (fid, def) in defs.function_definitions.as_map() {
        let b = &def.base;
        let key = fid.serialize_to_string();
        let name_span = b.name_location.as_ref().map(|l| here.loc.range(l));
        out.functions.push(fact_row!(
            sink,
            PysaFunctions,
            pysa(Origin::AnalyzerAssertion, Modality::Definite),
            PysaFunctionsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id: here.module_node_id,
                module_name: here.module_name.to_owned(),
                function_key: key.clone(),
                name: b.name.to_string(),
                name_start_byte: name_span.map(|s| s.0),
                name_end_byte: name_span.map(|s| s.1),
                is_overload: b.is_overload,
                is_staticmethod: b.is_staticmethod,
                is_classmethod: b.is_classmethod,
                is_property_getter: b.is_property_getter,
                is_property_setter: b.is_property_setter,
                is_stub: b.is_stub,
                is_def_statement: b.is_def_statement,
                defining_class: b.defining_class.as_ref().map(|c| refs.class(c)),
                overridden_base: def
                    .overridden_base_method
                    .as_ref()
                    .map(|f| refs.function(f)),
            }
        ));
        for (si, sig) in def.undecorated_signatures.iter().enumerate() {
            let rows: Vec<(SignatureForm, Option<i64>, Option<&FunctionParameter>)> =
                match &sig.parameters {
                    FunctionParameters::List(ps) => ps
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (SignatureForm::List, Some(i as i64), Some(p)))
                        .collect(),
                    FunctionParameters::Ellipsis => vec![(SignatureForm::Ellipsis, None, None)],
                    FunctionParameters::ParamSpec => vec![(SignatureForm::ParamSpec, None, None)],
                };
            for (form, ordinal, param) in rows {
                let (pkind, name, required, ann) = match param {
                    None => (None, None, None, None),
                    Some(FunctionParameter::PosOnly {
                        name,
                        annotation,
                        required,
                    }) => (
                        Some(ParameterKind::PositionalOnly),
                        name.as_ref().map(ToString::to_string),
                        Some(*required),
                        Some(annotation),
                    ),
                    Some(FunctionParameter::Pos {
                        name,
                        annotation,
                        required,
                    }) => (
                        Some(ParameterKind::PositionalOrKeyword),
                        Some(name.to_string()),
                        Some(*required),
                        Some(annotation),
                    ),
                    Some(FunctionParameter::VarArg { name, annotation }) => (
                        Some(ParameterKind::VarPositional),
                        name.as_ref().map(ToString::to_string),
                        None,
                        Some(annotation),
                    ),
                    Some(FunctionParameter::KwOnly {
                        name,
                        annotation,
                        required,
                    }) => (
                        Some(ParameterKind::KeywordOnly),
                        Some(name.to_string()),
                        Some(*required),
                        Some(annotation),
                    ),
                    Some(FunctionParameter::Kwargs { name, annotation }) => (
                        Some(ParameterKind::VarKeyword),
                        name.as_ref().map(ToString::to_string),
                        None,
                        Some(annotation),
                    ),
                };
                let (classes, exhaustive, scalar) =
                    ann.map(|a| annotation(refs, a)).unwrap_or_default();
                out.parameters.push(fact_row!(
                    sink,
                    ParameterSemantics,
                    pysa(Origin::AnalyzerAssertion, Modality::Definite),
                    ParameterSemanticsRow {
                        snapshot_id: Id::ZERO,
                        fact_id: Id::ZERO,
                        module_node_id: here.module_node_id,
                        module_name: here.module_name.to_owned(),
                        function_key: key.clone(),
                        signature_index: si as i64,
                        form,
                        ordinal,
                        kind: pkind,
                        name,
                        required,
                        annotation: ann.map(|a| a.string.clone()),
                        annotation_classes: classes,
                        annotation_classes_exhaustive: exhaustive,
                        annotation_scalar: scalar,
                    }
                ));
            }
        }
    }
    for (cid, cdef) in &defs.class_definitions {
        let class_key = cid.to_int().to_string();
        let name_span = here.loc.range(&cdef.name_location);
        let mut rows: Vec<(AncestryRelation, Option<i64>, Option<String>, bool)> = cdef
            .bases
            .iter()
            .enumerate()
            .map(|(i, c)| {
                (
                    AncestryRelation::Base,
                    Some(i as i64),
                    Some(refs.class(c)),
                    false,
                )
            })
            .collect();
        match &cdef.mro {
            PysaClassMro::Resolved(classes) => {
                rows.extend(classes.iter().enumerate().map(|(i, c)| {
                    (
                        AncestryRelation::Mro,
                        Some(i as i64),
                        Some(refs.class(c)),
                        false,
                    )
                }))
            }
            PysaClassMro::Cyclic => rows.push((AncestryRelation::Mro, None, None, true)),
        }
        for (relation, ordinal, ancestor, cyclic) in rows {
            out.ancestry.push(fact_row!(
                sink,
                ClassAncestry,
                pysa(Origin::AnalyzerAssertion, Modality::Definite),
                ClassAncestryRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    module_node_id: here.module_node_id,
                    module_name: here.module_name.to_owned(),
                    class_key: class_key.clone(),
                    class_name: cdef.name.clone(),
                    name_start_byte: Some(name_span.0),
                    name_end_byte: Some(name_span.1),
                    relation,
                    ordinal,
                    ancestor,
                    mro_cyclic: cyclic,
                }
            ));
        }
    }
}

struct Site<'h> {
    here: &'h Here<'h>,
    caller: String,
    kind: PysaSiteKind,
    detail: Option<String>,
    span: (i64, i64),
    synthetic: bool,
    /// Pysa's `is_attribute`, for an attribute access.
    is_attribute: Option<bool>,
}

struct Emit<'a> {
    site: &'a Site<'a>,
    callee_kind: PysaCalleeKind,
    phase: InvocationPhase,
    higher_order_index: Option<i64>,
    modality: Modality,
}

fn push_target(
    sink: &mut FactSink,
    out: &mut PysaOut,
    e: &Emit<'_>,
    t: Option<&PysaCallTarget<FunctionRef>>,
    reason: Option<PysaUnresolvedReason>,
) {
    let (target_kind, f) = match t.map(|t| &t.target) {
        None => (PysaTargetKind::Unresolved, None),
        Some(Target::Function(f)) => (PysaTargetKind::Function, Some(f)),
        Some(Target::Overrides(f)) => (PysaTargetKind::Overrides, Some(f)),
        // `Target::FormatString` is not carried (DESIGN §4.2.3): a synthetic formatting model.
        Some(Target::FormatString) => return,
    };
    // `Overrides` is a dispatch set, never one callee (§3.6).
    let modality = if target_kind == PysaTargetKind::Overrides && e.modality == Modality::Definite {
        Modality::Candidate
    } else {
        e.modality
    };
    let origin = if e.site.synthetic {
        Origin::SyntheticModel
    } else {
        Origin::AnalyzerAssertion
    };
    let refs = e.site.here.refs;
    out.calls.push(fact_row!(
        sink,
        PysaCalls,
        pysa(origin, modality),
        PysaCallsRow {
            snapshot_id: Id::ZERO,
            fact_id: Id::ZERO,
            module_node_id: e.site.here.module_node_id,
            module_name: e.site.here.module_name.to_owned(),
            caller_key: e.site.caller.clone(),
            site_kind: e.site.kind,
            callee_kind: e.callee_kind,
            site_detail: e.site.detail.clone(),
            start_byte: e.site.span.0,
            end_byte: e.site.span.1,
            phase: e.phase,
            higher_order_index: e.higher_order_index,
            target_kind,
            target_module: f.map(|f| refs.module(f.module_id, f.module_name)),
            target_key: f.map(|f| f.function_id.serialize_to_string()),
            target_name: f.map(|f| f.function_name.to_string()),
            receiver_class: t.and_then(|t| t.receiver_class.as_ref().map(|c| refs.class(c))),
            implicit_receiver: t.map(|t| implicit_receiver(t.implicit_receiver)),
            implicit_dunder_call: t.map(|t| t.implicit_dunder_call),
            is_class_method: t.map(|t| t.is_class_method),
            is_static_method: t.map(|t| t.is_static_method),
            unresolved_reason: reason,
            is_attribute: e.site.is_attribute,
        }
    ));
}

/// How one target list is read: its callee record, phase, the call's unresolved remainder,
/// whether its targets are only potential (`if_called`), and whether some flow at the site
/// invokes none of them (a property read where Pysa also sees a plain attribute).
#[derive(Clone, Copy)]
struct List {
    callee_kind: PysaCalleeKind,
    phase: InvocationPhase,
    rest: Option<PysaUnresolvedReason>,
    potential: bool,
    conditional: bool,
}

/// One target list: `definite` iff it is a single target, unconditionally invoked, with nothing
/// unresolved.
fn push_list(
    sink: &mut FactSink,
    out: &mut PysaOut,
    site: &Site<'_>,
    list: List,
    targets: &[PysaCallTarget<FunctionRef>],
) {
    let modality = if list.potential {
        Modality::Potential
    } else if targets.len() == 1 && list.rest.is_none() && !list.conditional {
        Modality::Definite
    } else {
        Modality::Candidate
    };
    let e = Emit {
        site,
        callee_kind: list.callee_kind,
        phase: list.phase,
        higher_order_index: None,
        modality,
    };
    for t in targets {
        push_target(sink, out, &e, Some(t), None);
    }
}

fn push_call_callees(
    sink: &mut FactSink,
    out: &mut PysaOut,
    site: &Site<'_>,
    callee_kind: PysaCalleeKind,
    cc: &CallCallees<FunctionRef>,
    potential: bool,
) {
    let rest = unresolved(&cc.unresolved);
    let lists = [
        (InvocationPhase::Call, &cc.call_targets),
        (InvocationPhase::Init, &cc.init_targets),
        (InvocationPhase::New, &cc.new_targets),
    ];
    for (phase, targets) in lists {
        let list = List {
            callee_kind,
            phase,
            rest,
            potential,
            conditional: false,
        };
        push_list(sink, out, site, list, targets);
    }
    let mut hops: Vec<_> = cc.higher_order_parameters.values().collect();
    hops.sort_by_key(|h| h.index);
    for h in hops {
        let e = Emit {
            site,
            callee_kind,
            phase: InvocationPhase::Call,
            higher_order_index: Some(i64::from(h.index)),
            modality: Modality::Potential,
        };
        for t in &h.call_targets {
            push_target(sink, out, &e, Some(t), None);
        }
        if let Some(r) = unresolved(&h.unresolved) {
            push_target(sink, out, &e, None, Some(r));
        }
    }
    if let Some(r) = rest {
        // The remainder of a potential list is potential too: the name may never be called.
        let e = Emit {
            site,
            callee_kind,
            phase: InvocationPhase::Call,
            higher_order_index: None,
            modality: if potential {
                Modality::Potential
            } else {
                Modality::Definite
            },
        };
        push_target(sink, out, &e, None, Some(r));
    }
}

pub(crate) fn map_call_graphs(
    here: &Here<'_>,
    graphs: &PysaModuleCallGraphs,
    sink: &mut FactSink,
    out: &mut PysaOut,
) {
    for (fid, graph) in &graphs.call_graphs {
        let caller = fid.serialize_to_string();
        for (eid, callees) in graph.as_map() {
            let (kind, location, detail, synthetic) = match eid {
                ExpressionIdentifier::Regular(l) => (PysaSiteKind::Regular, l, None, false),
                ExpressionIdentifier::ArtificialCall(o) => (
                    PysaSiteKind::ArtificialCall,
                    &o.location,
                    Some(origin_kind(&o.kind)),
                    true,
                ),
                ExpressionIdentifier::ArtificialAttributeAccess(o) => (
                    PysaSiteKind::ArtificialAttributeAccess,
                    &o.location,
                    Some(origin_kind(&o.kind)),
                    true,
                ),
                ExpressionIdentifier::FormatStringArtificial(l) => {
                    (PysaSiteKind::FormatStringArtificial, l, None, true)
                }
                ExpressionIdentifier::FormatStringStringify(l) => {
                    (PysaSiteKind::FormatStringStringify, l, None, true)
                }
                ExpressionIdentifier::Identifier {
                    location,
                    identifier,
                } => (
                    PysaSiteKind::Identifier,
                    location,
                    Some(identifier.to_string()),
                    false,
                ),
            };
            let is_attribute = match callees {
                ExpressionCallees::AttributeAccess(ac) => Some(ac.is_attribute),
                ExpressionCallees::Call(_)
                | ExpressionCallees::Identifier(_)
                | ExpressionCallees::FormatStringArtificial(_)
                | ExpressionCallees::FormatStringStringify(_)
                | ExpressionCallees::Define(_)
                | ExpressionCallees::Return(_) => None,
            };
            let site = Site {
                here,
                caller: caller.clone(),
                kind,
                detail,
                span: here.loc.range(location),
                synthetic,
                is_attribute,
            };
            match callees {
                ExpressionCallees::Call(cc) => {
                    if kind == PysaSiteKind::Regular {
                        out.regular_call_ranges.insert(site.span);
                    }
                    push_call_callees(sink, out, &site, PysaCalleeKind::Call, cc, false);
                }
                ExpressionCallees::Identifier(ic) => push_call_callees(
                    sink,
                    out,
                    &site,
                    PysaCalleeKind::Identifier,
                    &ic.if_called,
                    true,
                ),
                ExpressionCallees::AttributeAccess(ac) => {
                    push_call_callees(
                        sink,
                        out,
                        &site,
                        PysaCalleeKind::AttributeAccess,
                        &ac.if_called,
                        true,
                    );
                    push_list(
                        sink,
                        out,
                        &site,
                        List {
                            callee_kind: PysaCalleeKind::AttributeAccess,
                            phase: InvocationPhase::PropertyGet,
                            rest: None,
                            potential: false,
                            conditional: ac.is_attribute,
                        },
                        &ac.property_getters,
                    );
                    push_list(
                        sink,
                        out,
                        &site,
                        List {
                            callee_kind: PysaCalleeKind::AttributeAccess,
                            phase: InvocationPhase::PropertySet,
                            rest: None,
                            potential: false,
                            conditional: ac.is_attribute,
                        },
                        &ac.property_setters,
                    );
                }
                ExpressionCallees::FormatStringArtificial(f) => push_list(
                    sink,
                    out,
                    &site,
                    List {
                        callee_kind: PysaCalleeKind::FormatStringArtificial,
                        phase: InvocationPhase::Call,
                        rest: None,
                        potential: false,
                        conditional: false,
                    },
                    &f.targets,
                ),
                ExpressionCallees::FormatStringStringify(f) => {
                    let rest = unresolved(&f.unresolved);
                    push_list(
                        sink,
                        out,
                        &site,
                        List {
                            callee_kind: PysaCalleeKind::FormatStringStringify,
                            phase: InvocationPhase::Call,
                            rest,
                            potential: false,
                            conditional: false,
                        },
                        &f.targets,
                    );
                    if let Some(r) = rest {
                        let e = Emit {
                            site: &site,
                            callee_kind: PysaCalleeKind::FormatStringStringify,
                            phase: InvocationPhase::Call,
                            higher_order_index: None,
                            modality: Modality::Definite,
                        };
                        push_target(sink, out, &e, None, Some(r));
                    }
                }
                // Not carried (DESIGN §4.2.3): `Define` links a nested `def` to the function it
                // creates, which `declarations` already records; return shims are synthetic.
                ExpressionCallees::Define(_) | ExpressionCallees::Return(_) => {}
            }
        }
    }
}
