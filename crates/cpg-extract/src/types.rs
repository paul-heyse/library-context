//! Pyrefly's native types → the `types` family (`pyrefly-types`; CPG slice C4, DESIGN §3.2,
//! §3.5.1).
//!
//! A `Type` becomes a term per distinct structure: `type_terms` (one row per term) and
//! `type_term_args` (its children). The term id is a Merkle id over kind, display, detail, class,
//! type-variable identity, binder and the children's ids, so the same structure anywhere is one
//! node. A class is a (module ref, class key) pair and a recursive alias a reference to its name,
//! so a term is finite; a depth cap makes that a guarantee rather than an expectation.
//!
//! Observations attach terms to the walker's nodes by exact span or name: parameters and returns
//! of every `def`, call results, argument values and raised exceptions. Record fields come from
//! Pyrefly's class metadata. Every Pyrefly enum is matched exhaustively with no wildcard arm: a
//! new upstream variant fails the build (§3.5, DM-42).
#![deny(clippy::wildcard_enum_match_arm)]

use std::collections::{HashMap, HashSet};

use cpg_schema::codebook::{
    BoundaryReason, DeclarationKind, ExtractionMode, Fidelity, Modality, Origin, ParameterKind,
    RecordKind, SyntaxField, SyntaxKind, TypeArgRole, TypeRole, TypeTermKind,
};
use cpg_schema::id::{Id, IdHasher, kind, recipe};
use cpg_schema::tables::{
    RecordFields, RecordFieldsRow, TypeObservations, TypeObservationsRow, TypeTermArgs,
    TypeTermArgsRow, TypeTerms, TypeTermsRow,
};
use pyrefly::alt::answers::Solutions;
use pyrefly::alt::types::class_metadata::DataclassKind;
use pyrefly::binding::binding::{Key, KeyClassMetadata};
use pyrefly::report::pysa::class::{
    ClassRef, get_all_classes, get_class_field_from_current_class_only,
};
use pyrefly::report::pysa::context::ModuleContext;
use pyrefly::report::pysa::function::get_all_decorated_functions;
use pyrefly_types::callable::{Callable, Param, ParamList, Params, PrefixParam, Required};
use pyrefly_types::class::Class;
use pyrefly_types::quantified::{Quantified, QuantifiedKind, QuantifiedOrigin};
use pyrefly_types::tuple::Tuple;
use pyrefly_types::type_alias::TypeAliasData;
use pyrefly_types::typed_dict::TypedDict;
use pyrefly_types::types::{
    AnyStyle, BoundMethodType, Forallable, NeverStyle, OverloadType, TArgs, Type,
};
use ruff_python_ast::statement_visitor::{StatementVisitor, walk_stmt};
use ruff_python_ast::{ModModule, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::facts::{FactSink, Provenance, Surface, fact_row};
use crate::pysa_map::ModuleRefs;
use crate::walk::WalkOut;

/// Deeper structure than this is cut to a `truncated` term (display only).
const MAX_DEPTH: u32 = 32;

#[derive(Default)]
pub(crate) struct TypesOut {
    pub terms: Vec<TypeTermsRow>,
    pub args: Vec<TypeTermArgsRow>,
    pub observations: Vec<TypeObservationsRow>,
    pub fields: Vec<RecordFieldsRow>,
    /// Terms already emitted in this run.
    emitted: HashSet<Id>,
}

/// Something the provider describes that the walker's nodes do not carry: a boundary.
pub(crate) struct Miss {
    pub reason: BoundaryReason,
    pub span: (i64, i64),
    pub detail: String,
}

pub(crate) struct ModuleTypes<'a> {
    pub context: &'a ModuleContext<'a>,
    pub solutions: Option<&'a Solutions>,
    pub refs: &'a ModuleRefs,
    pub module_name: &'a str,
    pub module_node_id: Id,
    pub walk: &'a WalkOut,
}

fn pyrefly_types(fidelity: Fidelity) -> Provenance {
    Provenance {
        surface: Surface::PyreflyTypes,
        mode: ExtractionMode::NativeTraversal,
        origin: Origin::AnalyzerAssertion,
        modality: Modality::Definite,
        fidelity,
    }
}

fn range(start: i64, end: i64) -> TextRange {
    TextRange::new(
        TextSize::new(u32::try_from(start).unwrap_or_default()),
        TextSize::new(u32::try_from(end).unwrap_or_default()),
    )
}

fn span(r: TextRange) -> (i64, i64) {
    (i64::from(r.start().to_u32()), i64::from(r.end().to_u32()))
}

/// A `def` or `class` header (decorators through the colon: signature, bases, type parameters)
/// and whether a `def` annotates its return.
struct Header {
    name: TextRange,
    header: TextRange,
    returns: bool,
}

#[derive(Default)]
struct Headers(Vec<Header>);

impl<'a> StatementVisitor<'a> for Headers {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let declaration = if let Stmt::FunctionDef(f) = stmt {
            Some((f.name.range(), &f.body, f.returns.is_some()))
        } else if let Stmt::ClassDef(c) = stmt {
            Some((c.name.range(), &c.body, false))
        } else {
            None
        };
        if let Some((name, body, returns)) = declaration {
            let end = body.first().map_or(stmt.end(), Ranged::start);
            self.0.push(Header {
                name,
                header: TextRange::new(stmt.start(), end),
                returns,
            });
        }
        walk_stmt(self, stmt);
    }
}

/// One term under construction: its payload and its children.
struct Term {
    kind: TypeTermKind,
    detail: Option<String>,
    class: Option<(String, String)>,
    variable: Option<String>,
    binder: Option<Id>,
    args: Vec<Arg>,
}

struct Arg {
    role: TypeArgRole,
    ordinal: i64,
    child: Id,
    name: Option<String>,
    parameter_kind: Option<ParameterKind>,
    required: Option<bool>,
}

impl Term {
    fn new(kind: TypeTermKind) -> Self {
        Self {
            kind,
            detail: None,
            class: None,
            variable: None,
            binder: None,
            args: Vec::new(),
        }
    }

    fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = Some(d.into());
        self
    }
}

fn any_style(s: AnyStyle) -> &'static str {
    match s {
        AnyStyle::Explicit => "explicit",
        AnyStyle::Implicit => "implicit",
        AnyStyle::Error => "error",
    }
}

fn never_style(s: NeverStyle) -> &'static str {
    match s {
        NeverStyle::NoReturn => "NoReturn",
        NeverStyle::Never => "Never",
    }
}

fn origin(o: QuantifiedOrigin) -> &'static str {
    match o {
        QuantifiedOrigin::ScopedLegacy => "legacy",
        QuantifiedOrigin::Pep695 => "pep695",
        QuantifiedOrigin::Synthetic { is_self: false } => "synthetic",
        QuantifiedOrigin::Synthetic { is_self: true } => "synthetic-self",
        QuantifiedOrigin::MapIntTuplesParameter => "map-int-tuples",
        QuantifiedOrigin::NormalizedMapIntTuplesParameter => "normalized-map-int-tuples",
    }
}

fn required(r: &Required) -> bool {
    match r {
        Required::Required => true,
        Required::Optional(_) => false,
    }
}

/// Builds the terms of one module's types.
struct Builder<'a, 'o> {
    m: &'a ModuleTypes<'a>,
    sink: &'o mut FactSink,
    out: &'o mut TypesOut,
    headers: &'a [Header],
    /// Each declaration's name span → its node id.
    decls: &'a HashMap<(i64, i64), Id>,
}

impl Builder<'_, '_> {
    fn class_ref(&self, class: &Class) -> (String, String) {
        self.m
            .refs
            .class_pair(&ClassRef::from_class(class, self.m.context))
    }

    /// The `def` or `class` of this module whose header holds the variable's scope anchor.
    fn binder(&self, q: &Quantified) -> Option<Id> {
        let identity = q.identity();
        let anchored = match identity.origin {
            QuantifiedOrigin::ScopedLegacy | QuantifiedOrigin::Pep695 => true,
            QuantifiedOrigin::Synthetic { .. }
            | QuantifiedOrigin::MapIntTuplesParameter
            | QuantifiedOrigin::NormalizedMapIntTuplesParameter => false,
        };
        if !anchored || identity.module.as_str() != self.m.module_name {
            return None;
        }
        let anchor = identity.anchor.range;
        self.headers
            .iter()
            .find(|h| h.header.contains_range(anchor))
            .and_then(|h| self.decls.get(&span(h.name)).copied())
    }

    fn variable(&self, q: &Quantified, kind: TypeTermKind, detail: String) -> Term {
        let identity = q.identity();
        let mut t = Term::new(kind).detail(detail);
        t.variable = Some(format!(
            "{}:{}-{}#{}/{}",
            identity.module,
            identity.anchor.range.start().to_u32(),
            identity.anchor.range.end().to_u32(),
            identity.anchor.index,
            origin(identity.origin)
        ));
        t.binder = self.binder(q);
        t
    }

    fn quantified(&self, q: &Quantified) -> Term {
        let kind = match q.kind {
            QuantifiedKind::TypeVar | QuantifiedKind::IntVar => TypeTermKind::TypeVar,
            QuantifiedKind::ParamSpec => TypeTermKind::ParamSpec,
            QuantifiedKind::TypeVarTuple => TypeTermKind::TypeVarTuple,
        };
        self.variable(q, kind, q.name.to_string())
    }

    fn arg(&mut self, t: &mut Term, role: TypeArgRole, ordinal: usize, ty: &Type, depth: u32) {
        let child = self.term(ty, depth + 1);
        t.args.push(Arg {
            role,
            ordinal: ordinal as i64,
            child,
            name: None,
            parameter_kind: None,
            required: None,
        });
    }

    fn targs(&mut self, t: &mut Term, targs: &TArgs, depth: u32) {
        for (i, a) in targs.as_slice().iter().enumerate() {
            self.arg(t, TypeArgRole::Argument, i, a, depth);
        }
    }

    /// A callable parameter: its type, and its name, kind and requiredness.
    fn param(
        &mut self,
        t: &mut Term,
        ordinal: usize,
        ty: &Type,
        (name, kind, req): (Option<String>, ParameterKind, Option<bool>),
        depth: u32,
    ) {
        let child = self.term(ty, depth + 1);
        t.args.push(Arg {
            role: TypeArgRole::Parameter,
            ordinal: ordinal as i64,
            child,
            name,
            parameter_kind: Some(kind),
            required: req,
        });
    }

    fn prefix(&mut self, t: &mut Term, prefix: &[PrefixParam], depth: u32) {
        for (i, p) in prefix.iter().enumerate() {
            let (ty, meta) = match p {
                PrefixParam::PosOnly(n, ty, r) => (
                    ty,
                    (
                        n.as_ref().map(ToString::to_string),
                        ParameterKind::PositionalOnly,
                        Some(required(r)),
                    ),
                ),
                PrefixParam::Pos(n, ty, r) => (
                    ty,
                    (
                        Some(n.to_string()),
                        ParameterKind::PositionalOrKeyword,
                        Some(required(r)),
                    ),
                ),
            };
            self.param(t, i, ty, meta, depth);
        }
    }

    fn params(&mut self, t: &mut Term, list: &ParamList, depth: u32) {
        for (i, p) in list.items().iter().enumerate() {
            let meta = match p {
                Param::PosOnly(n, _, r) => (
                    n.as_ref().map(ToString::to_string),
                    ParameterKind::PositionalOnly,
                    Some(required(r)),
                ),
                Param::Pos(n, _, r) => (
                    Some(n.to_string()),
                    ParameterKind::PositionalOrKeyword,
                    Some(required(r)),
                ),
                Param::Varargs(n, _) => (
                    n.as_ref().map(ToString::to_string),
                    ParameterKind::VarPositional,
                    None,
                ),
                Param::KwOnly(n, _, r) => (
                    Some(n.to_string()),
                    ParameterKind::KeywordOnly,
                    Some(required(r)),
                ),
                Param::Kwargs(n, _) => (
                    n.as_ref().map(ToString::to_string),
                    ParameterKind::VarKeyword,
                    None,
                ),
            };
            self.param(t, i, p.as_type(), meta, depth);
        }
    }

    /// A callable's parameters and return; the parameter form joins `detail`.
    fn callable(&mut self, t: &mut Term, c: &Callable, depth: u32) {
        let form = match &c.params {
            Params::List(list) => {
                self.params(t, list, depth);
                None
            }
            Params::Partial(list) => {
                self.params(t, list, depth);
                Some("partial")
            }
            Params::Ellipsis => Some("..."),
            Params::Materialization => Some("materialization"),
            Params::ParamSpec(prefix, p) => {
                self.prefix(t, prefix, depth);
                self.arg(t, TypeArgRole::ParamSpec, 0, p, depth);
                None
            }
        };
        if let Some(form) = form {
            t.detail = Some(match t.detail.take() {
                Some(name) => format!("{name} {form}"),
                None => form.to_owned(),
            });
        }
        self.arg(t, TypeArgRole::Return, 0, &c.ret, depth);
    }

    /// The term for `ty`: its id, with its rows emitted once per run.
    fn term(&mut self, ty: &Type, depth: u32) -> Id {
        if depth > MAX_DEPTH {
            return self.finish(ty, Term::new(TypeTermKind::Truncated));
        }
        let t = self.build(ty, depth);
        self.finish(ty, t)
    }

    fn build(&mut self, ty: &Type, depth: u32) -> Term {
        use TypeTermKind as K;
        match ty {
            Type::Literal(_) => Term::new(K::Literal).detail(ty.to_string()),
            Type::LiteralString(_) => Term::new(K::Literal).detail("LiteralString"),
            Type::Callable(c) => {
                let mut t = Term::new(K::Callable);
                self.callable(&mut t, c, depth);
                t
            }
            Type::Function(f) => {
                let mut t =
                    Term::new(K::Callable).detail(f.metadata.kind.function_name().to_string());
                self.callable(&mut t, &f.signature, depth);
                t
            }
            Type::BoundMethod(b) => {
                let mut t = Term::new(K::BoundMethod);
                self.arg(&mut t, TypeArgRole::Receiver, 0, &b.obj, depth);
                let func = match &b.func {
                    BoundMethodType::Function(f) => Type::Function(Box::new(f.clone())),
                    BoundMethodType::Forall(f) => BoundMethodType::Forall(f.clone()).as_type(),
                    BoundMethodType::Overload(o) => Type::Overload(o.clone()),
                };
                self.arg(&mut t, TypeArgRole::Function, 0, &func, depth);
                t
            }
            Type::Overload(o) => {
                let mut t =
                    Term::new(K::Overload).detail(o.metadata.kind.function_name().to_string());
                for (i, s) in o.signatures.iter().enumerate() {
                    let sig = match s {
                        OverloadType::Function(_) | OverloadType::Forall(_) => s.as_type(),
                    };
                    self.arg(&mut t, TypeArgRole::Signature, i, &sig, depth);
                }
                t
            }
            Type::Union(u) => {
                let mut t = Term::new(K::Union);
                for (i, m) in u.members.iter().enumerate() {
                    self.arg(&mut t, TypeArgRole::Member, i, m, depth);
                }
                t
            }
            Type::Intersect(x) => {
                let mut t = Term::new(K::Intersection);
                for (i, m) in x.0.iter().enumerate() {
                    self.arg(&mut t, TypeArgRole::Member, i, m, depth);
                }
                t
            }
            Type::ClassDef(c) => {
                let mut t = Term::new(K::ClassObject);
                t.class = Some(self.class_ref(c));
                t
            }
            Type::ClassType(c) => {
                let mut t = Term::new(K::ClassInstance);
                t.class = Some(self.class_ref(c.class_object()));
                self.targs(&mut t, c.targs(), depth);
                t
            }
            Type::SelfType(c) => {
                let mut t = Term::new(K::SelfType);
                t.class = Some(self.class_ref(c.class_object()));
                self.targs(&mut t, c.targs(), depth);
                t
            }
            Type::TypedDict(d) | Type::PartialTypedDict(d) => {
                let mut t = Term::new(K::TypedDict);
                if matches!(ty, Type::PartialTypedDict(_)) {
                    t.detail = Some("partial".to_owned());
                }
                match d {
                    TypedDict::TypedDict(inner) => {
                        t.class = Some(self.class_ref(inner.class_object()));
                        self.targs(&mut t, inner.targs(), depth);
                    }
                    TypedDict::Anonymous(inner) => {
                        t.detail = Some(match t.detail.take() {
                            Some(d) => format!("{d} anonymous"),
                            None => "anonymous".to_owned(),
                        });
                        for (i, (name, field)) in inner.fields.iter().enumerate() {
                            let child = self.term(&field.ty, depth + 1);
                            t.args.push(Arg {
                                role: TypeArgRole::Member,
                                ordinal: i as i64,
                                child,
                                name: Some(name.to_string()),
                                parameter_kind: None,
                                required: Some(field.required),
                            });
                        }
                    }
                }
                t
            }
            Type::Tuple(x) => {
                let mut t = Term::new(K::Tuple);
                match x {
                    Tuple::Concrete(elts) => {
                        for (i, e) in elts.iter().enumerate() {
                            self.arg(&mut t, TypeArgRole::Element, i, e, depth);
                        }
                    }
                    Tuple::Unbounded(e) => self.arg(&mut t, TypeArgRole::Variadic, 0, e, depth),
                    Tuple::Unpacked(parts) => {
                        let n = parts.prefix().len();
                        for (i, e) in parts.prefix().iter().enumerate() {
                            self.arg(&mut t, TypeArgRole::Element, i, e, depth);
                        }
                        self.arg(&mut t, TypeArgRole::Variadic, n, parts.middle(), depth);
                        for (j, e) in parts.suffix().iter().enumerate() {
                            self.arg(&mut t, TypeArgRole::Element, n + 1 + j, e, depth);
                        }
                    }
                }
                t
            }
            Type::Module(m) => Term::new(K::Module).detail(
                m.parts()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            Type::Forall(f) => {
                let mut t = Term::new(K::Generic);
                for (i, q) in f.tparams.as_vec().iter().enumerate() {
                    let child = self.finish_quantified(q);
                    t.args.push(Arg {
                        role: TypeArgRole::TypeParameter,
                        ordinal: i as i64,
                        child,
                        name: None,
                        parameter_kind: None,
                        required: None,
                    });
                }
                let body = match &f.body {
                    Forallable::TypeAlias(a) => Type::TypeAlias(Box::new(a.clone())),
                    Forallable::Function(func) => Type::Function(Box::new(func.clone())),
                    Forallable::Callable(c) => Type::Callable(Box::new(c.clone())),
                };
                self.arg(&mut t, TypeArgRole::Target, 0, &body, depth);
                t
            }
            Type::Quantified(q) => self.quantified(q),
            Type::QuantifiedValue(q) => {
                self.variable(q, K::SpecialForm, format!("{} (value)", q.name))
            }
            Type::ElementOfTypeVarTuple(q) => {
                self.variable(q, K::Other, format!("element of {}", q.name))
            }
            Type::Args(q) => self.variable(q, K::ParamSpec, format!("{}.args", q.name)),
            Type::Kwargs(q) => self.variable(q, K::ParamSpec, format!("{}.kwargs", q.name)),
            Type::ArgsValue(q) => {
                self.variable(q, K::SpecialForm, format!("{}.args (value)", q.name))
            }
            Type::KwargsValue(q) => {
                self.variable(q, K::SpecialForm, format!("{}.kwargs (value)", q.name))
            }
            Type::TypeGuard(x) | Type::TypeIs(x) => {
                let mut t = Term::new(K::TypeGuard).detail(if matches!(ty, Type::TypeIs(_)) {
                    "TypeIs"
                } else {
                    "TypeGuard"
                });
                self.arg(&mut t, TypeArgRole::Target, 0, x, depth);
                t
            }
            Type::Annotated(x, _) => {
                let mut t = Term::new(K::Annotated);
                self.arg(&mut t, TypeArgRole::Target, 0, x, depth);
                t
            }
            Type::Unpack(x) => {
                let mut t = Term::new(K::Unpack);
                self.arg(&mut t, TypeArgRole::Target, 0, x, depth);
                t
            }
            Type::Type(x) => {
                let mut t = Term::new(K::TypeOf);
                self.arg(&mut t, TypeArgRole::Target, 0, x, depth);
                t
            }
            Type::TypeForm(x) => {
                let mut t = Term::new(K::TypeOf).detail("TypeForm");
                self.arg(&mut t, TypeArgRole::Target, 0, x, depth);
                t
            }
            Type::Concatenate(prefix, p) => {
                let mut t = Term::new(K::ParamList).detail("Concatenate");
                self.prefix(&mut t, prefix, depth);
                self.arg(&mut t, TypeArgRole::ParamSpec, 0, p, depth);
                t
            }
            Type::ParamSpecValue(list) => {
                let mut t = Term::new(K::ParamList);
                self.params(&mut t, list, depth);
                t
            }
            Type::TypeVar(_) => Term::new(K::SpecialForm).detail(ty.to_string()),
            Type::ParamSpec(_) => Term::new(K::SpecialForm).detail(ty.to_string()),
            Type::TypeVarTuple(_) => Term::new(K::SpecialForm).detail(ty.to_string()),
            Type::SpecialForm(_) => Term::new(K::SpecialForm).detail(ty.to_string()),
            Type::Ellipsis => Term::new(K::SpecialForm).detail("..."),
            Type::Any(s) => Term::new(K::Any).detail(any_style(*s)),
            Type::Never(s) => Term::new(K::Never).detail(never_style(*s)),
            Type::None => Term::new(K::None),
            Type::TypeAlias(a) | Type::UntypedAlias(a) => {
                let untyped = matches!(ty, Type::UntypedAlias(_));
                let mut t = Term::new(K::TypeAlias);
                match a.as_ref() {
                    TypeAliasData::Value(v) => {
                        t.detail = Some(if untyped {
                            format!("{} (untyped)", v.name)
                        } else {
                            v.name.to_string()
                        });
                        self.arg(&mut t, TypeArgRole::Target, 0, &v.as_type(), depth);
                    }
                    // A recursive reference is its name and arguments, never an expansion.
                    TypeAliasData::Ref(r) => {
                        t.detail = Some(format!(
                            "{}.{} (ref{})",
                            r.module_name,
                            r.name,
                            if untyped { ", untyped" } else { "" }
                        ));
                        if let Some(args) = &r.args {
                            self.targs(&mut t, args, depth);
                        }
                    }
                }
                t
            }
            // Outside the stated model: solver-internal or experimental forms, kept as display.
            Type::CallableResidual(_) => Term::new(K::Other).detail("callable_residual"),
            Type::TypeLevelDslCall(_) => Term::new(K::Other).detail("type_level_dsl_call"),
            Type::ShapedArray(_) => Term::new(K::Other).detail("shaped_array"),
            Type::IntTuple(_) => Term::new(K::Other).detail("int_tuple"),
            Type::NNModule(_) => Term::new(K::Other).detail("nn_module"),
            Type::DataFrame(_) => Term::new(K::Other).detail("data_frame"),
            Type::Series(_) => Term::new(K::Other).detail("series"),
            Type::Int(_) => Term::new(K::Other).detail("int"),
            Type::Var(_) => Term::new(K::Other).detail("var"),
            Type::Sentinel(_) => Term::new(K::Other).detail("sentinel"),
            Type::SuperInstance(_) => Term::new(K::Other).detail("super_instance"),
            Type::KwCall(_) => Term::new(K::Other).detail("kw_call"),
            Type::Materialization => Term::new(K::Other).detail("materialization"),
        }
    }

    fn finish_quantified(&mut self, q: &Quantified) -> Id {
        let t = self.quantified(q);
        self.finish(&Type::Quantified(Box::new(q.clone())), t)
    }

    /// Hash the term, emit its rows once, and return its id.
    fn finish(&mut self, ty: &Type, t: Term) -> Id {
        let display = ty.to_string();
        let (class_module, class_key) = t.class.clone().unzip();
        let mut h = IdHasher::new(kind::TYPE);
        h.i64(i64::from(cpg_schema::Codebook::code(t.kind)))
            .str(&display)
            .opt_str(t.detail.as_deref())
            .opt_str(class_module.as_deref())
            .opt_str(class_key.as_deref())
            .opt_str(t.variable.as_deref())
            .opt_id(t.binder)
            .i64(t.args.len() as i64);
        for a in &t.args {
            h.i64(i64::from(cpg_schema::Codebook::code(a.role)))
                .i64(a.ordinal)
                .id(a.child)
                .opt_str(a.name.as_deref())
                .opt_i64(
                    a.parameter_kind
                        .map(|k| i64::from(cpg_schema::Codebook::code(k))),
                )
                .opt_bool(a.required);
        }
        let node_id = h.finish_id();
        if !self.out.emitted.insert(node_id) {
            return node_id;
        }
        let fidelity = match t.kind {
            TypeTermKind::Other | TypeTermKind::Truncated => Fidelity::DisplayOnly,
            TypeTermKind::ClassInstance
            | TypeTermKind::ClassObject
            | TypeTermKind::TypeOf
            | TypeTermKind::TypedDict
            | TypeTermKind::Union
            | TypeTermKind::Intersection
            | TypeTermKind::Callable
            | TypeTermKind::Overload
            | TypeTermKind::BoundMethod
            | TypeTermKind::Generic
            | TypeTermKind::Tuple
            | TypeTermKind::Literal
            | TypeTermKind::TypeVar
            | TypeTermKind::ParamSpec
            | TypeTermKind::TypeVarTuple
            | TypeTermKind::Module
            | TypeTermKind::Any
            | TypeTermKind::Never
            | TypeTermKind::None
            | TypeTermKind::TypeAlias
            | TypeTermKind::SelfType
            | TypeTermKind::Annotated
            | TypeTermKind::Unpack
            | TypeTermKind::TypeGuard
            | TypeTermKind::ParamList
            | TypeTermKind::SpecialForm => Fidelity::NativeStructural,
        };
        let row = fact_row!(
            self.sink,
            TypeTerms,
            pyrefly_types(fidelity),
            TypeTermsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                node_id,
                kind: t.kind,
                display,
                detail: t.detail,
                class_module,
                class_key,
                variable: t.variable,
                binder_node_id: t.binder,
            }
        );
        self.out.terms.push(row);
        for a in t.args {
            let row = fact_row!(
                self.sink,
                TypeTermArgs,
                pyrefly_types(Fidelity::NativeStructural),
                TypeTermArgsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    parent_node_id: node_id,
                    role: a.role,
                    ordinal: a.ordinal,
                    child_node_id: a.child,
                    name: a.name,
                    parameter_kind: a.parameter_kind,
                    required: a.required,
                }
            );
            self.out.args.push(row);
        }
        node_id
    }

    fn observe(&mut self, subject: Id, role: TypeRole, declared: bool, ty: &Type) {
        let term_node_id = self.term(ty, 0);
        let row = fact_row!(
            self.sink,
            TypeObservations,
            pyrefly_types(Fidelity::NativeStructural),
            TypeObservationsRow {
                snapshot_id: Id::ZERO,
                fact_id: Id::ZERO,
                module_node_id: self.m.module_node_id,
                subject_node_id: subject,
                role,
                declared,
                term_node_id,
            }
        );
        self.out.observations.push(row);
    }
}

/// The `types` family for one module; returns what the walker's nodes could not carry.
pub(crate) fn module_types(
    m: &ModuleTypes<'_>,
    ast: &ModModule,
    sink: &mut FactSink,
    out: &mut TypesOut,
) -> Vec<Miss> {
    let mut misses = Vec::new();
    let ctx = &m.context.answers_context;
    let mut headers = Headers::default();
    headers.visit_body(&ast.body);
    let decls: HashMap<(i64, i64), Id> = m
        .walk
        .declarations
        .iter()
        .map(|d| ((d.name_start_byte, d.name_end_byte), d.node_id))
        .collect();
    let classes: HashMap<(i64, i64), Id> = m
        .walk
        .declarations
        .iter()
        .filter(|d| d.kind == DeclarationKind::Class)
        .map(|d| ((d.name_start_byte, d.name_end_byte), d.node_id))
        .collect();
    let returns: HashMap<(i64, i64), bool> = headers
        .0
        .iter()
        .map(|h| (span(h.name), h.returns))
        .collect();
    let params: HashMap<(Id, &str), (Id, bool)> = m
        .walk
        .parameter_syntax
        .iter()
        .map(|p| {
            (
                (p.function_node_id, p.name.as_str()),
                (p.node_id, p.annotation_text.is_some()),
            )
        })
        .collect();
    let mut b = Builder {
        m,
        sink,
        out,
        headers: &headers.0,
        decls: &decls,
    };

    // Parameters and returns of every `def`.
    for f in get_all_decorated_functions(ctx) {
        let name = span(f.undecorated.identifier.range());
        let Some(&function) = decls.get(&name) else {
            misses.push(Miss {
                reason: BoundaryReason::MissingEvidence,
                span: name,
                detail: "a def Pyrefly types has no declaration".to_owned(),
            });
            continue;
        };
        for p in &f.undecorated.params {
            let Some(pname) = p.name() else { continue };
            match params.get(&(function, pname.as_str())) {
                Some(&(node, annotated)) => {
                    b.observe(node, TypeRole::Parameter, annotated, p.as_type())
                }
                None => misses.push(Miss {
                    reason: BoundaryReason::MissingEvidence,
                    span: name,
                    detail: format!("parameter `{pname}` has no parameter node"),
                }),
            }
        }
        let idx = ctx
            .bindings
            .key_to_idx(&Key::ReturnType(f.undecorated.identifier));
        if let Some(ret) = ctx.answers.get_type_at(idx) {
            let declared = returns.get(&name).copied().unwrap_or(false);
            b.observe(function, TypeRole::Return, declared, &ret);
        }
    }

    // Call results and argument values (outside annotations), raised exceptions.
    let in_annotation: HashSet<Id> = m
        .walk
        .call_syntax
        .iter()
        .filter(|c| c.in_annotation)
        .map(|c| c.node_id)
        .collect();
    for c in &m.walk.call_syntax {
        if c.in_annotation {
            continue;
        }
        if let Some(t) = ctx.answers.get_type_trace(range(c.start_byte, c.end_byte)) {
            b.observe(c.node_id, TypeRole::CallResult, false, &t);
        }
    }
    for (argument, call, value) in &m.walk.argument_values {
        if in_annotation.contains(call) {
            continue;
        }
        if let Some(t) = ctx.answers.get_type_trace(*value) {
            b.observe(*argument, TypeRole::Argument, false, &t);
        }
    }
    let raises: HashSet<Id> = m
        .walk
        .syntax_nodes
        .iter()
        .filter(|s| s.kind == SyntaxKind::StmtRaise)
        .map(|s| s.node_id)
        .collect();
    for s in &m.walk.syntax_nodes {
        let parent = s.parent_node_id;
        if s.field != SyntaxField::Exc || !raises.contains(&parent) {
            continue;
        }
        if let Some(t) = ctx.answers.get_type_trace(range(s.start_byte, s.end_byte)) {
            b.observe(parent, TypeRole::Raised, false, &t);
        }
    }

    // Record fields: the fields each record class declares itself.
    let Some(solutions) = m.solutions else {
        return misses;
    };
    let heap = ctx.answers.heap();
    for class in get_all_classes(ctx) {
        let metadata = solutions.get(&KeyClassMetadata(class.index()));
        let (record, names): (RecordKind, Vec<(String, Option<bool>)>) =
            if let Some(td) = metadata.typed_dict_metadata() {
                (
                    RecordKind::TypedDict,
                    td.fields
                        .iter()
                        .map(|(n, total)| (n.to_string(), Some(*total)))
                        .collect(),
                )
            } else if let Some(nt) = metadata.named_tuple_metadata() {
                (
                    RecordKind::NamedTuple,
                    nt.elements.iter().map(|n| (n.to_string(), None)).collect(),
                )
            } else if let Some(dm) = metadata.dataclass_metadata() {
                let record = if metadata.is_pydantic_model() {
                    RecordKind::Pydantic
                } else {
                    match &dm.kind {
                        DataclassKind::Attrs { .. } => RecordKind::Attrs,
                        DataclassKind::Dataclass { .. } => RecordKind::Dataclass,
                    }
                };
                (
                    record,
                    dm.instance_fields()
                        .map(|n| (n.to_string(), None))
                        .collect(),
                )
            } else {
                continue;
            };
        let class_span = span(class.qname().range());
        let Some(&class_node_id) = classes.get(&class_span) else {
            misses.push(Miss {
                reason: BoundaryReason::NoSourceDeclaration,
                span: class_span,
                detail: format!(
                    "record fields of `{}`, a class with no `class` statement",
                    class.name()
                ),
            });
            continue;
        };
        let class_fields = &ctx.bindings.metadata().get_class(class.index()).fields;
        for (ordinal, (name, total)) in names.iter().enumerate() {
            let pname = ruff_python_ast::name::Name::new(name);
            // An inherited field is its base's row.
            let Some(field) = get_class_field_from_current_class_only(&class, &pname, ctx) else {
                continue;
            };
            let term_node_id = b.term(&field.ty(), 0);
            let decl = class_fields.field_decl_range(&pname).map(span);
            let (mut has_default, mut init, mut alias, mut kw_only) = (None, None, None, None);
            let (mut req, mut read_only) = (None, None);
            match record {
                RecordKind::Dataclass | RecordKind::Attrs | RecordKind::Pydantic => {
                    let flags = field.dataclass_flags_of(heap);
                    has_default = Some(flags.default.is_some());
                    init = Some(flags.init);
                    alias = flags.init_by_alias.as_ref().map(ToString::to_string);
                    kw_only = flags.kw_only;
                }
                RecordKind::NamedTuple => {
                    has_default = Some(!required(&field.as_named_tuple_requiredness()));
                }
                RecordKind::TypedDict => {
                    if let Some(info) = field.as_typed_dict_field_info(total.unwrap_or(true)) {
                        req = Some(info.required);
                        read_only = Some(info.read_only_reason.is_some());
                    }
                }
            }
            let row = fact_row!(
                b.sink,
                RecordFields,
                pyrefly_types(Fidelity::NativeStructural),
                RecordFieldsRow {
                    snapshot_id: Id::ZERO,
                    fact_id: Id::ZERO,
                    node_id: recipe::field(class_node_id, name),
                    class_node_id,
                    module_node_id: m.module_node_id,
                    record_kind: record,
                    name: name.clone(),
                    ordinal: ordinal as i64,
                    term_node_id,
                    declared: field.has_explicit_annotation(),
                    start_byte: decl.map(|d| d.0),
                    end_byte: decl.map(|d| d.1),
                    has_default,
                    init,
                    alias,
                    kw_only,
                    required: req,
                    read_only,
                }
            );
            b.out.fields.push(row);
        }
    }
    misses
}
