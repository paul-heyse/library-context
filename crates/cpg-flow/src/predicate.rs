//! ty's predicates and reachability diagrams, read as our conditions (ADR-0022 §Conditions).
//!
//! A reachability constraint is a ternary decision diagram over predicates. At runtime every
//! predicate is true or false, so a diagram's condition is the disjunction, over its paths to the
//! true terminal, of the conjunction of the predicates taken on the way; the `if_ambiguous` edges
//! are never taken. ty's ambiguous terminal is admitted as may-behavior.
//!
//! **The runtime view** decides only names our lexical resolution binds to typing's
//! `TYPE_CHECKING`, or the stdlib `sys` and `os` modules. A name's spelling alone is insufficient.
//!
//! **Evaluation identity.** Every source test has its evaluation site's identity. A reaching-
//! definition set alone cannot prove stability across calls that change a closure or global
//! binding. Synthetic predicates use their provider predicate identities (ADR-0022 §Places).

use std::collections::HashMap;

use cpg_schema::condition::{Atom, EvaluationIdentity, Value};
use cpg_schema::id::IdHasher;
use ruff_db::parsed::ParsedModuleRef;
use ruff_python_ast_ty::{self as ast, CmpOp, Expr};
use ruff_text_size_ty::Ranged;
use ty_python_core::place::PlaceExpr;
use ty_python_core::predicate::{
    PatternPredicateKind, Predicate, PredicateNode, ScopedPredicateId,
};
use ty_python_core::reachability_constraints::ScopedReachabilityConstraintId;
use ty_python_core::{FileScopeId, UseDefMap};

use crate::db::FlowDb;
use crate::{Condition, RuntimeBindings, RuntimeContext, SENTINEL, Span};

/// A predicate ty records but no test decides (an or-pattern's alternative, a star import).
pub(crate) const UNDECIDED: &str = "<undecided by the flow provider>";
/// ty's non-empty-iterable predicate (a `for` over `range(...)`).
const NON_EMPTY: &str = "<the iterable is non-empty>";
/// ty's context-manager suppression predicate.
const SUPPRESSES: &str = "<the context manager suppresses the exception>";
/// ty's finally-normal-path predicate.
const FINALLY: &str = "<no normal path enters the finally suite>";

pub(crate) struct Translator<'a> {
    pub db: &'a FlowDb,
    pub module: &'a ParsedModuleRef,
    /// The module's text as written (the sentinel undone): opaque atoms quote it.
    pub original: &'a str,
    pub context: &'a RuntimeContext,
    pub runtime: &'a RuntimeBindings,
    pub module_key: String,
}

impl Translator<'_> {
    fn source(&self, range: ruff_text_size_ty::TextRange) -> &str {
        &self.original[usize::from(range.start())..usize::from(range.end())]
    }

    fn site(&self, e: &Expr) -> EvaluationIdentity {
        EvaluationIdentity::Site {
            module: self.module_key.clone(),
            start: e.range().start().into(),
            end: e.range().end().into(),
        }
    }

    fn atom(&self, atom: Atom, whole: &Expr) -> Condition {
        Condition::atom(atom.evaluated(self.site(whole)))
    }

    /// An untranslatable test is always a fresh evaluation, even when its text repeats.
    fn opaque(&self, e: &Expr) -> Condition {
        self.atom(Atom::opaque(&strip_comments(self.source(e.range()))), e)
    }

    /// The place an expression names, as ty spells it, when it has at most two attribute
    /// segments and no subscript (DESIGN §3.9). Evaluation-site identity lives on the atom.
    pub(crate) fn place(&self, e: &Expr) -> Option<String> {
        self.plain(e)
    }

    /// The place an expression names, spelled plainly (see [`Self::place`]).
    fn plain(&self, e: &Expr) -> Option<String> {
        let place = PlaceExpr::try_from_expr(e)?.to_string();
        let original = self
            .source(e.range())
            .split_whitespace()
            .collect::<String>();
        // The sentinel is a name like any other to ty; our text is the module as written.
        let place = if place.contains(std::str::from_utf8(SENTINEL).expect("ASCII")) {
            original
        } else {
            place
        };
        let dots = place.matches('.').count();
        (dots <= 2 && !place.contains('[')).then_some(place)
    }

    /// The truth of `e` used as a test.
    pub(crate) fn test(&self, e: &Expr) -> Condition {
        if let Some(fixed) = self.runtime(e) {
            return if fixed {
                Condition::always()
            } else {
                Condition::never()
            };
        }
        match e {
            Expr::BoolOp(b) => {
                let parts = b.values.iter().map(|v| self.test(v));
                match b.op {
                    ast::BoolOp::And => parts.fold(Condition::always(), |a, p| a.and(&p)),
                    ast::BoolOp::Or => parts.fold(Condition::never(), |a, p| a.or(&p)),
                }
            }
            Expr::UnaryOp(u) if u.op == ast::UnaryOp::Not => self.test(&u.operand).not(),
            Expr::If(i) => {
                let t = self.test(&i.test);
                t.and(&self.test(&i.body))
                    .or(&t.not().and(&self.test(&i.orelse)))
            }
            Expr::Named(n) => self.test(&n.target),
            Expr::Compare(c) if c.ops.len() == 1 => {
                self.compare(e, &c.operands[0], c.ops[0], &c.operands[1])
            }
            Expr::Call(call) => self.isinstance(e, call).unwrap_or_else(|| self.opaque(e)),
            Expr::BooleanLiteral(b) => {
                if b.value {
                    Condition::always()
                } else {
                    Condition::never()
                }
            }
            Expr::NoneLiteral(_) => Condition::never(),
            _ => match self.place(e) {
                Some(place) => self.atom(Atom::Truthy { place }, e),
                None => self.opaque(e),
            },
        }
    }

    fn compare(&self, whole: &Expr, left: &Expr, op: CmpOp, right: &Expr) -> Condition {
        // The place on either side; a literal on the other.
        let (place, other) = match (self.place(left), self.place(right)) {
            (Some(p), _) if literal(right).is_some() || literal_set(right).is_some() => (p, right),
            (_, Some(p)) if literal(left).is_some() => (p, left),
            _ => return self.opaque(whole),
        };
        let atom = match (op, literal(other), literal_set(other)) {
            (CmpOp::Is | CmpOp::IsNot, Some(Value::None), _) => Atom::IsNone { place },
            (CmpOp::Is | CmpOp::IsNot, Some(v), _) => Atom::IsValue { place, value: v },
            (CmpOp::Eq | CmpOp::NotEq, Some(v), _) => Atom::Equals { place, value: v },
            (CmpOp::In | CmpOp::NotIn, _, Some(values)) if std::ptr::eq(other, right) => {
                Atom::member_of(place, values)
            }
            _ => return self.opaque(whole),
        };
        let positive = matches!(op, CmpOp::Is | CmpOp::Eq | CmpOp::In);
        let c = self.atom(atom, whole);
        if positive { c } else { c.not() }
    }

    fn isinstance(&self, whole: &Expr, call: &ast::ExprCall) -> Option<Condition> {
        let Expr::Name(f) = call.func.as_ref() else {
            return None;
        };
        if f.id.as_str() != "isinstance"
            || call.arguments.args.len() != 2
            || !call.arguments.keywords.is_empty()
        {
            return None;
        }
        let place = self.place(&call.arguments.args[0])?;
        let classes: Vec<&Expr> = match &call.arguments.args[1] {
            Expr::Tuple(t) => t.elts.iter().collect(),
            e => vec![e],
        };
        let mut out = Condition::never();
        for c in classes {
            if !matches!(c, Expr::Name(_) | Expr::Attribute(_)) {
                return Some(self.opaque(whole));
            }
            let class = self
                .source(c.range())
                .split_whitespace()
                .collect::<String>();
            out = out.or(&self.atom(
                Atom::IsInstance {
                    place: place.clone(),
                    class,
                },
                whole,
            ));
        }
        Some(out)
    }

    /// A test the runtime view decides (`TYPE_CHECKING`, `sys.version_info`, `sys.platform`,
    /// `os.name`); `None` for any other.
    fn runtime(&self, e: &Expr) -> Option<bool> {
        let sentinel = std::str::from_utf8(SENTINEL).expect("ASCII");
        match e {
            Expr::Name(_) if self.runtime.checking_names.contains(&Span::from(e.range())) => {
                Some(false)
            }
            Expr::Attribute(a)
                if a.attr.as_str() == sentinel
                    && self.root_is(&a.value, &self.runtime.typing_modules) =>
            {
                Some(false)
            }
            Expr::Compare(c) if c.ops.len() == 1 => {
                let (left, op, right) = (&c.operands[0], c.ops[0], &c.operands[1]);
                if self.resolved_attr(left, &self.runtime.sys_modules, "version_info") {
                    let want = int_tuple(right)?;
                    if want.len() > 3 {
                        return None;
                    }
                    let have = [
                        i64::from(self.context.python_version.0),
                        i64::from(self.context.python_version.1),
                        i64::from(self.context.python_version.2),
                    ];
                    // sys.version_info has five fields. Its known 3-field prefix is greater
                    // than an equal 3-field literal; longer literals stay undecided here.
                    let ord = match have.as_slice().cmp(want.as_slice()) {
                        std::cmp::Ordering::Equal => std::cmp::Ordering::Greater,
                        other => other,
                    };
                    return Some(match op {
                        CmpOp::Lt => ord.is_lt(),
                        CmpOp::LtE => ord.is_le(),
                        CmpOp::Gt => ord.is_gt(),
                        CmpOp::GtE => ord.is_ge(),
                        CmpOp::Eq => ord.is_eq(),
                        CmpOp::NotEq => ord.is_ne(),
                        _ => return None,
                    });
                }
                let value = if self.resolved_attr(left, &self.runtime.sys_modules, "platform") {
                    self.context.platform.as_str()
                } else if self.resolved_attr(left, &self.runtime.os_modules, "name") {
                    if self.context.platform == "win32" {
                        "nt"
                    } else {
                        "posix"
                    }
                } else {
                    return None;
                };
                let Some(Value::Str(s)) = literal(right) else {
                    return None;
                };
                match op {
                    CmpOp::Eq => Some(value == s),
                    CmpOp::NotEq => Some(value != s),
                    _ => None,
                }
            }
            Expr::Call(call) => {
                let Expr::Attribute(m) = call.func.as_ref() else {
                    return None;
                };
                if m.attr.as_str() == "startswith"
                    && self.resolved_attr(&m.value, &self.runtime.sys_modules, "platform")
                    && call.arguments.args.len() == 1
                    && let Some(Value::Str(s)) = literal(&call.arguments.args[0])
                {
                    return Some(self.context.platform.starts_with(s.as_str()));
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn runtime_decides(&self, e: &Expr) -> bool {
        if self.runtime(e).is_some() {
            return true;
        }
        // Follow exactly the expression forms `test` lowers recursively. Inspecting children
        // of an opaque call would attribute a decision that never affected its atom.
        match e {
            Expr::BoolOp(b) => b.values.iter().any(|v| self.runtime_decides(v)),
            Expr::UnaryOp(u) if u.op == ast::UnaryOp::Not => self.runtime_decides(&u.operand),
            Expr::If(i) => {
                self.runtime_decides(&i.test)
                    || self.runtime_decides(&i.body)
                    || self.runtime_decides(&i.orelse)
            }
            Expr::Named(n) => self.runtime_decides(&n.target),
            _ => false,
        }
    }

    fn root_is(&self, e: &Expr, names: &std::collections::BTreeSet<Span>) -> bool {
        matches!(e, Expr::Name(_)) && names.contains(&Span::from(e.range()))
    }

    fn resolved_attr(
        &self,
        e: &Expr,
        names: &std::collections::BTreeSet<Span>,
        attr: &str,
    ) -> bool {
        matches!(e, Expr::Attribute(a) if a.attr.as_str() == attr && self.root_is(&a.value, names))
    }

    /// The truth of one predicate, its polarity applied.
    fn predicate(&self, p: &Predicate, synthetic: &EvaluationIdentity) -> Condition {
        let c = match &p.node {
            PredicateNode::Expression(x)
            | PredicateNode::Condition(x)
            | PredicateNode::ChainedComparisonCondition(x) => {
                self.test(x.node_ref(self.db).node(self.module))
            }
            PredicateNode::IsNonTerminalCall(_) => Condition::always(),
            PredicateNode::IsNonEmptyIterable(_) => {
                Condition::atom(Atom::opaque(NON_EMPTY).evaluated(synthetic.clone()))
            }
            PredicateNode::ContextManagerSuppresses { .. } => {
                Condition::atom(Atom::opaque(SUPPRESSES).evaluated(synthetic.clone()))
            }
            PredicateNode::FinallyNormalPathImpossible { .. } => {
                Condition::atom(Atom::opaque(FINALLY).evaluated(synthetic.clone()))
            }
            PredicateNode::Pattern(pattern) => {
                let subject = pattern.subject(self.db).node_ref(self.db).node(self.module);
                let mut c = self.pattern(subject, pattern.kind(self.db), synthetic);
                if let Some(guard) = pattern.guard(self.db) {
                    c = c.and(&self.test(guard.node_ref(self.db).node(self.module)));
                }
                c
            }
            PredicateNode::OrPatternAlternative(_)
            | PredicateNode::SubjectElementPattern(_)
            | PredicateNode::StarImportPlaceholder(_) => {
                Condition::atom(Atom::opaque(UNDECIDED).evaluated(synthetic.clone()))
            }
        };
        if p.is_positive { c } else { c.not() }
    }

    pub(crate) fn pattern(
        &self,
        subject: &Expr,
        kind: &PatternPredicateKind,
        evaluation: &EvaluationIdentity,
    ) -> Condition {
        let Some(place) = self.place(subject) else {
            return Condition::atom(
                Atom::opaque(&strip_comments(self.source(subject.range())))
                    .evaluated(evaluation.clone()),
            );
        };
        match kind {
            PatternPredicateKind::Singleton(ast::Singleton::None) => {
                Condition::atom(Atom::IsNone { place }.evaluated(evaluation.clone()))
            }
            PatternPredicateKind::Singleton(s) => Condition::atom(
                Atom::IsValue {
                    place,
                    value: Value::Bool(matches!(s, ast::Singleton::True)),
                }
                .evaluated(evaluation.clone()),
            ),
            PatternPredicateKind::Value(v) => {
                match literal(v.node_ref(self.db).node(self.module)) {
                    Some(value) => {
                        Condition::atom(Atom::Equals { place, value }.evaluated(evaluation.clone()))
                    }
                    None => Condition::atom(
                        Atom::opaque(&strip_comments(
                            self.source(v.node_ref(self.db).node(self.module).range()),
                        ))
                        .evaluated(evaluation.clone()),
                    ),
                }
            }
            PatternPredicateKind::Or(alternatives) => {
                alternatives.iter().fold(Condition::never(), |acc, k| {
                    acc.or(&self.pattern(subject, k, evaluation))
                })
            }
            PatternPredicateKind::Class(class) => {
                let c = class.class.node_ref(self.db).node(self.module);
                let atom = Condition::atom(
                    Atom::IsInstance {
                        place,
                        class: self.source(c.range()).split_whitespace().collect(),
                    }
                    .evaluated(evaluation.clone()),
                );
                if class.is_empty() {
                    atom
                } else {
                    atom.and(&Condition::atom(
                        Atom::opaque(UNDECIDED).evaluated(evaluation.clone()),
                    ))
                }
            }
            PatternPredicateKind::As(Some(inner), _) => self.pattern(subject, inner, evaluation),
            PatternPredicateKind::As(None, _) | PatternPredicateKind::Star(_) => {
                Condition::always()
            }
            PatternPredicateKind::Mapping(_) | PatternPredicateKind::Sequence(_) => self
                .pattern_opaque(subject, evaluation)
                .and(&Condition::atom(
                    Atom::opaque(UNDECIDED).evaluated(evaluation.clone()),
                )),
        }
    }

    fn pattern_opaque(&self, subject: &Expr, evaluation: &EvaluationIdentity) -> Condition {
        Condition::atom(
            Atom::opaque(&strip_comments(self.source(subject.range())))
                .evaluated(evaluation.clone()),
        )
    }
}

/// A reachability diagram's condition, memoized per diagram node of one scope's use-def map.
pub(crate) fn diagram(
    t: &Translator<'_>,
    fid: FileScopeId,
    map: &UseDefMap<'_>,
    memo: &mut HashMap<ScopedReachabilityConstraintId, Condition>,
    id: ScopedReachabilityConstraintId,
) -> Condition {
    if id == ScopedReachabilityConstraintId::ALWAYS_TRUE {
        return Condition::always();
    }
    if id == ScopedReachabilityConstraintId::ALWAYS_FALSE {
        return Condition::never();
    }
    // ty's third value: reachability it leaves undecided (a `try` body, a loop over an unknown
    // iterable, a `with` exit). Verdicts state may-behavior, so it reads as `true` (ADR-0022
    // §Conditions; the Stage 2 review's F2).
    if id == ScopedReachabilityConstraintId::AMBIGUOUS {
        return Condition::always().with_approximation();
    }
    if let Some(c) = memo.get(&id) {
        return c.clone();
    }
    let node = map.reachability_constraints().get_interior_node(id);
    let synthetic = synthetic_identity(&t.module_key, fid, node.atom());
    let p = t.predicate(&map.predicates()[node.atom()], &synthetic);
    let on_true = diagram(t, fid, map, memo, node.if_true());
    let on_false = diagram(t, fid, map, memo, node.if_false());
    let c = p.and(&on_true).or(&p.not().and(&on_false));
    memo.insert(id, c.clone());
    c
}

pub(crate) fn synthetic_identity(
    module_key: &str,
    fid: FileScopeId,
    predicate_id: ScopedPredicateId,
) -> EvaluationIdentity {
    let predicate = IdHasher::new("flow-synthetic-predicate")
        .str(&format!("{fid:?}"))
        .str(&format!("{predicate_id:?}"))
        .finish_id()
        .hex();
    EvaluationIdentity::Synthetic {
        module: module_key.to_owned(),
        predicate,
    }
}

/// Whether a diagram depends on a test our resolved runtime view decided. A false root is
/// counted as ty's false; otherwise this distinguishes a runtime-view skip from a contradiction
/// in translated stable atoms. The categories are diagnostic, in that precedence order.
pub(crate) fn runtime_in_diagram(
    t: &Translator<'_>,
    map: &UseDefMap<'_>,
    memo: &mut HashMap<ScopedReachabilityConstraintId, bool>,
    id: ScopedReachabilityConstraintId,
) -> bool {
    if matches!(
        id,
        ScopedReachabilityConstraintId::ALWAYS_TRUE
            | ScopedReachabilityConstraintId::ALWAYS_FALSE
            | ScopedReachabilityConstraintId::AMBIGUOUS
    ) {
        return false;
    }
    if let Some(&found) = memo.get(&id) {
        return found;
    }
    let node = map.reachability_constraints().get_interior_node(id);
    let p = &map.predicates()[node.atom()];
    let here = match &p.node {
        PredicateNode::Expression(x)
        | PredicateNode::Condition(x)
        | PredicateNode::ChainedComparisonCondition(x) => {
            t.runtime_decides(x.node_ref(t.db).node(t.module))
        }
        _ => false,
    };
    let found = here
        || runtime_in_diagram(t, map, memo, node.if_true())
        || runtime_in_diagram(t, map, memo, node.if_false());
    memo.insert(id, found);
    found
}

/// A literal the condition language holds: `None`, `True`, `False`, an integer or a string.
pub(crate) fn literal(e: &Expr) -> Option<Value> {
    match e {
        Expr::NoneLiteral(_) => Some(Value::None),
        Expr::BooleanLiteral(b) => Some(Value::Bool(b.value)),
        Expr::NumberLiteral(n) => match &n.value {
            ast::Number::Int(i) => i.as_i64().map(Value::Int),
            _ => None,
        },
        Expr::UnaryOp(u) if u.op == ast::UnaryOp::USub => match literal(&u.operand)? {
            Value::Int(i) => i.checked_neg().map(Value::Int),
            _ => None,
        },
        Expr::StringLiteral(s) => Some(Value::Str(s.value.to_str().to_owned())),
        _ => None,
    }
}

/// A tuple, list or set of literals.
fn literal_set(e: &Expr) -> Option<Vec<Value>> {
    let elts = match e {
        Expr::Tuple(t) => &t.elts,
        Expr::List(l) => &l.elts,
        Expr::Set(s) => &s.elts,
        _ => return None,
    };
    elts.iter().map(literal).collect()
}

fn int_tuple(e: &Expr) -> Option<Vec<i64>> {
    let Expr::Tuple(t) = e else { return None };
    t.elts
        .iter()
        .map(|x| match literal(x)? {
            Value::Int(i) => Some(i),
            _ => None,
        })
        .collect()
}

/// A test's source without its comments: `#` to the end of the line, outside string literals.
pub(crate) fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut quote: Option<(char, bool)> = None;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match quote {
            Some((q, triple)) => {
                out.push(c);
                if c == '\\' {
                    if let Some((_, next)) = chars.next() {
                        out.push(next);
                    }
                } else if c == q && (!triple || text[i..].starts_with(&q.to_string().repeat(3))) {
                    if triple {
                        out.push(q);
                        out.push(q);
                        chars.next();
                        chars.next();
                    }
                    quote = None;
                }
            }
            None if c == '#' => {
                while chars.peek().is_some_and(|&(_, n)| n != '\n') {
                    chars.next();
                }
            }
            None if c == '"' || c == '\'' => {
                let triple = text[i..].starts_with(&c.to_string().repeat(3));
                out.push(c);
                if triple {
                    out.push(c);
                    out.push(c);
                    chars.next();
                    chars.next();
                }
                quote = Some((c, triple));
            }
            None => out.push(c),
        }
    }
    out
}
