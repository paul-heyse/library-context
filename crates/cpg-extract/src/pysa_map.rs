//! Pyrefly's Pysa collectors → `pyrefly-pysa` raw tables (DESIGN §4.2.3).
//!
//! Every Pyrefly enum is matched exhaustively with no wildcard arm: a new upstream variant fails
//! the build instead of degrading silently (§3.5, DM-42).
#![deny(clippy::wildcard_enum_match_arm)]

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use cpg_schema::codebook::ExtractionMode;
use cpg_schema::codebook::{
    AncestryRelation, DefinitionKind, Fidelity, ImplicitReceiver, InvocationPhase, Modality,
    Origin, ParameterKind, PysaCalleeKind, PysaSiteKind, PysaTargetKind, PysaUnresolvedReason,
    SignatureForm,
};
use cpg_schema::id::{Id, IdHasher, kind};
use cpg_schema::tables::{
    ClassAncestry, ClassAncestryRow, ParameterSemantics, ParameterSemanticsRow, PysaCalls,
    PysaCallsRow, PysaClasses, PysaClassesRow, PysaFunctions, PysaFunctionsRow,
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
    pub classes: Vec<PysaClassesRow>,
    pub calls: Vec<PysaCallsRow>,
    /// Byte ranges of regular call sites Pysa described (the S5 join key).
    pub regular_call_ranges: HashSet<(i64, i64)>,
    /// Byte ranges of Pysa's other sites except identifiers (attribute accesses, artificial and
    /// format-string sites): the syntax walk places a node at each (CPG slice C2).
    pub site_ranges: HashSet<(i64, i64)>,
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
        mode: ExtractionMode::NativeTraversal,
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
    /// Every dependency module a mapped row referenced, with the id Pysa gave it: the modules
    /// whose definitions become `context_definitions` (DESIGN §3.8).
    pub dependencies: RefCell<BTreeMap<String, ModuleId>>,
    /// The dependency definitions referenced: (module name, kind, Pysa key).
    pub referenced: RefCell<BTreeSet<(String, DefinitionKind, String)>>,
}

impl ModuleRefs {
    pub fn new(release_files: HashMap<ModuleId, String>) -> Self {
        Self {
            release_files,
            dependencies: RefCell::default(),
            referenced: RefCell::default(),
        }
    }

    pub(crate) fn module(&self, id: ModuleId, name: ModuleName) -> String {
        match self.release_files.get(&id) {
            Some(path) => format!("@{path}"),
            None => {
                let name = name.to_string();
                self.dependencies.borrow_mut().insert(name.clone(), id);
                name
            }
        }
    }

    fn pair(
        &self,
        id: ModuleId,
        name: ModuleName,
        kind: DefinitionKind,
        key: String,
    ) -> (String, String) {
        let module = self.module(id, name);
        if !module.starts_with('@') {
            self.referenced
                .borrow_mut()
                .insert((module.clone(), kind, key.clone()));
        }
        (module, key)
    }

    /// (module ref, `ClassId`): the typed form of a class reference.
    fn class_pair(&self, c: &ClassRef) -> (String, String) {
        self.pair(
            c.module_id,
            c.class.module_name(),
            DefinitionKind::Class,
            c.class_id.to_int().to_string(),
        )
    }

    /// (module ref, `FunctionId`): the typed form of a function reference.
    fn function_pair(&self, f: &FunctionRef) -> (String, String) {
        self.pair(
            f.module_id,
            f.module_name,
            DefinitionKind::Function,
            f.function_id.serialize_to_string(),
        )
    }

    /// `<module ref>:<name>#<ClassId>`: the id keys it, the name is for reading.
    fn class(&self, c: &ClassRef) -> String {
        let (module, key) = self.class_pair(c);
        format!("{module}:{}#{key}", c.class.name())
    }

    fn function(&self, f: &FunctionRef) -> String {
        let (module, key) = self.function_pair(f);
        format!("{module}::{key}")
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
        let defining = b.defining_class.as_ref().map(|c| refs.class_pair(c));
        let overridden = def
            .overridden_base_method
            .as_ref()
            .map(|f| refs.function_pair(f));
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
                defining_class_module: defining.as_ref().map(|p| p.0.clone()),
                defining_class_key: defining.map(|p| p.1),
                overridden_module: overridden.as_ref().map(|p| p.0.clone()),
                overridden_key: overridden.map(|p| p.1),
                signature_count: def.undecorated_signatures.len() as i64,
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
        out.classes.push(fact_row!(
            sink,
            PysaClasses,
            pysa(Origin::AnalyzerAssertion, Modality::Definite),
            PysaClassesRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id: here.module_node_id,
                module_name: here.module_name.to_owned(),
                class_key: class_key.clone(),
                class_name: cdef.name.clone(),
                name_start_byte: name_span.0,
                name_end_byte: name_span.1,
                is_synthesized: cdef.is_synthesized,
                is_dataclass: cdef.is_dataclass,
                is_named_tuple: cdef.is_named_tuple,
                is_typed_dict: cdef.is_typed_dict,
            }
        ));
        let mut rows: Vec<(AncestryRelation, Option<i64>, Option<&ClassRef>, bool)> = cdef
            .bases
            .iter()
            .enumerate()
            .map(|(i, c)| (AncestryRelation::Base, Some(i as i64), Some(c), false))
            .collect();
        match &cdef.mro {
            PysaClassMro::Resolved(classes) => rows.extend(
                classes
                    .iter()
                    .enumerate()
                    .map(|(i, c)| (AncestryRelation::Mro, Some(i as i64), Some(c), false)),
            ),
            PysaClassMro::Cyclic => rows.push((AncestryRelation::Mro, None, None, true)),
        }
        for (relation, ordinal, ancestor_ref, cyclic) in rows {
            let ancestor = ancestor_ref.map(|c| refs.class(c));
            let pair = ancestor_ref.map(|c| refs.class_pair(c));
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
                    ancestor_module: pair.as_ref().map(|p| p.0.clone()),
                    ancestor_key: pair.map(|p| p.1),
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
    if !(e.site.kind == PysaSiteKind::Regular && e.callee_kind == PysaCalleeKind::Call)
        && e.callee_kind != PysaCalleeKind::Identifier
    {
        out.site_ranges.insert(e.site.span);
    }
    let target = f.map(|f| refs.function_pair(f));
    let receiver = t.and_then(|t| t.receiver_class.as_ref().map(|c| refs.class_pair(c)));
    let mut row = PysaCallsRow {
        snapshot_id: Id::ZERO,
        fact_id: Id::ZERO,
        payload_id: Id::ZERO,
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
        target_module: target.as_ref().map(|p| p.0.clone()),
        target_key: target.map(|p| p.1),
        target_name: f.map(|f| f.function_name.to_string()),
        receiver_class: t.and_then(|t| t.receiver_class.as_ref().map(|c| refs.class(c))),
        receiver_module: receiver.as_ref().map(|p| p.0.clone()),
        receiver_key: receiver.map(|p| p.1),
        implicit_receiver: t.map(|t| implicit_receiver(t.implicit_receiver)),
        implicit_dunder_call: t.map(|t| t.implicit_dunder_call),
        is_class_method: t.map(|t| t.is_class_method),
        is_static_method: t.map(|t| t.is_static_method),
        unresolved_reason: reason,
        is_attribute: e.site.is_attribute,
    };
    // The run-independent payload digest: every column but the ids, before the fact id is taken
    // (which then covers it too).
    let mut h = IdHasher::new(kind::PYSA_CALL);
    row.hash_fields(&mut h);
    row.payload_id = h.finish_id();
    out.calls
        .push(fact_row!(sink, PysaCalls, pysa(origin, modality), row));
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
