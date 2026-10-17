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

use crate::native::{Atom, EvaluationSite, Value};
use ruff_db::parsed::ParsedModuleRef;
use ruff_python_ast_ty::{self as ast, CmpOp, Expr};
use ruff_text_size_ty::Ranged;
use ty_python_core::predicate::{PatternPredicate, PatternPredicateKind, Predicate, PredicateNode};
use ty_python_core::reachability_constraints::ScopedReachabilityConstraintId;
use ty_python_core::{FileScopeId, UseDefMap};

use crate::admitted::{Resources, hash_slot};
use crate::db::FlowDb;
use crate::{FlowCondition as Condition, RuntimeBindings, RuntimeContext, SENTINEL, Span};
use lctx_model::domain::charged::StateCharge;

/// A predicate ty records but no test decides (an or-pattern's alternative, a star import).
pub(crate) const UNDECIDED: &str = "<undecided by the flow provider>";

pub(crate) struct Translator<'a> {
    pub db: &'a FlowDb,
    pub module: &'a ParsedModuleRef,
    /// The module's text as written (the sentinel undone): opaque atoms quote it.
    pub original: &'a str,
    pub context: &'a RuntimeContext,
    pub runtime: &'a RuntimeBindings,
    pub resources: &'a Resources<'a>,
}

impl Translator<'_> {
    fn source(&self, range: ruff_text_size_ty::TextRange) -> &str {
        &self.original[usize::from(range.start())..usize::from(range.end())]
    }

    fn uncommented(&self, range: ruff_text_size_ty::TextRange) -> String {
        self.resources.grow(usize::from(range.len()));
        let mut out = String::with_capacity(usize::from(range.len()));
        let mut at = usize::from(range.start());
        let end = usize::from(range.end());
        // Some native expressions originate inside string-annotation tokens. Avoid Tokens::in_range's
        // token-boundary assertion while still seeking the first possibly intersecting token.
        let tokens = self.module.tokens();
        let first = tokens.partition_point(|token| token.range().end() <= range.start());
        for token in tokens[first..]
            .iter()
            .take_while(|token| token.range().start() < range.end())
        {
            let token_range = token.range();
            if token.kind() == ruff_python_ast_ty::token::TokenKind::Comment
                && token_range.start() >= range.start()
                && token_range.end() <= range.end()
            {
                let start = usize::from(token_range.start());
                out.push_str(&self.original[at..start]);
                at = usize::from(token_range.end());
            }
        }
        out.push_str(&self.original[at..end]);
        out
    }

    fn site(&self, e: &Expr) -> EvaluationSite {
        EvaluationSite::Source(Span::from(e.range()))
    }
    pub(crate) fn pattern_site(&self, pattern: &PatternPredicate<'_>) -> EvaluationSite {
        self.resources.checkpoint();
        let subject = pattern.subject(self.db).node_ref(self.db).node(self.module);
        let mut ordinal = 0;
        let mut previous = pattern.previous_predicate(self.db);
        while let Some(p) = previous {
            ordinal += 1;
            if ordinal > 256 {
                return EvaluationSite::Unavailable {
                    reason: "pattern depth refused",
                };
            }
            self.resources.checkpoint();
            previous = p.previous_predicate(self.db);
        }
        struct Finder<'a> {
            resources: &'a Resources<'a>,
            subject: Span,
            ordinal: usize,
            found: Option<Span>,
        }
        impl<'a> ruff_python_ast_ty::visitor::source_order::SourceOrderVisitor<'a> for Finder<'_> {
            fn enter_node(
                &mut self,
                _node: ast::AnyNodeRef<'a>,
            ) -> ruff_python_ast_ty::visitor::source_order::TraversalSignal {
                self.resources.checkpoint();
                ruff_python_ast_ty::visitor::source_order::TraversalSignal::Traverse
            }
            fn visit_stmt(&mut self, stmt: &'a ast::Stmt) {
                if let ast::Stmt::Match(m) = stmt
                    && Span::from(m.subject.range()) == self.subject
                {
                    self.found = m
                        .cases
                        .get(self.ordinal)
                        .map(|case| Span::from(case.range()));
                }
                ruff_python_ast_ty::visitor::source_order::walk_stmt(self, stmt);
            }
        }
        let mut finder = Finder {
            resources: self.resources,
            subject: Span::from(subject.range()),
            ordinal,
            found: None,
        };
        use ruff_python_ast_ty::visitor::source_order::SourceOrderVisitor;
        finder.visit_body(self.module.suite());
        finder.found.map_or(
            EvaluationSite::Unavailable {
                reason: "pattern case coordinate unavailable",
            },
            EvaluationSite::Source,
        )
    }
    fn atom(&self, atom: Atom, whole: &Expr) -> Condition {
        Condition::atom(
            self.resources.budget,
            atom.evaluated_in(self.resources, self.site(whole)),
        )
    }

    /// An untranslatable test is always a fresh evaluation, even when its text repeats.
    fn opaque(&self, e: &Expr) -> Condition {
        self.atom(
            Atom::opaque_in(self.resources, &self.uncommented(e.range())),
            e,
        )
    }

    /// The place an expression names, as ty spells it, when it has at most two attribute
    /// segments and no subscript (DESIGN §3.9). Evaluation-site identity lives on the atom.
    pub(crate) fn place(&self, e: &Expr) -> Option<String> {
        self.plain(e)
    }

    /// The place an expression names, spelled plainly (see [`Self::place`]).
    fn plain(&self, e: &Expr) -> Option<String> {
        let place = self.resources.format(format_args!(
            "{}",
            crate::admitted::place_expr(e, self.resources)?
        ));
        let original = self.resources.compact(self.source(e.range()), "");
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
        self.resources.checkpoint();
        if let Some(fixed) = self.runtime(e) {
            return if fixed {
                Condition::terminal(self.resources.budget, Some(true))
            } else {
                Condition::terminal(self.resources.budget, Some(false))
            };
        }
        match e {
            Expr::BoolOp(b) => {
                let parts = b.values.iter().map(|v| self.test(v));
                match b.op {
                    ast::BoolOp::And => parts.fold(
                        Condition::terminal(self.resources.budget, Some(true)),
                        |a, p| a.and(&p),
                    ),
                    ast::BoolOp::Or => parts.fold(
                        Condition::terminal(self.resources.budget, Some(false)),
                        |a, p| a.or(&p),
                    ),
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
                    Condition::terminal(self.resources.budget, Some(true))
                } else {
                    Condition::terminal(self.resources.budget, Some(false))
                }
            }
            Expr::NoneLiteral(_) => Condition::terminal(self.resources.budget, Some(false)),
            _ => match self.place(e) {
                Some(place) => self.atom(Atom::Truthy { place }, e),
                None => self.opaque(e),
            },
        }
    }

    fn compare(&self, whole: &Expr, left: &Expr, op: CmpOp, right: &Expr) -> Condition {
        if matches!(op, CmpOp::Is | CmpOp::IsNot)
            && let Some((place, class)) = self
                .type_is_builtin(left, right)
                .or_else(|| self.type_is_builtin(right, left))
        {
            let condition = self.atom(Atom::TypeIs { place, class }, whole);
            return if op == CmpOp::Is {
                condition
            } else {
                condition.not()
            };
        }
        // The place on either side; a literal on the other.
        let (place, other) = match (self.place(left), self.place(right)) {
            (Some(p), _)
                if literal(right, self.resources).is_some()
                    || literal_set(right, self.resources).is_some() =>
            {
                (p, right)
            }
            (_, Some(p)) if literal(left, self.resources).is_some() => (p, left),
            _ => return self.opaque(whole),
        };
        let atom = match (
            op,
            literal(other, self.resources),
            literal_set(other, self.resources),
        ) {
            (CmpOp::Is | CmpOp::IsNot, Some(Value::None), _) => Atom::IsNone { place },
            (CmpOp::Is | CmpOp::IsNot, Some(v), _) => Atom::IsValue { place, value: v },
            (CmpOp::Eq | CmpOp::NotEq, Some(v), _) => Atom::Equals { place, value: v },
            (CmpOp::In | CmpOp::NotIn, _, Some(values)) if std::ptr::eq(other, right) => {
                Atom::member_of_in(self.resources, place, values)
            }
            _ => return self.opaque(whole),
        };
        let positive = matches!(op, CmpOp::Is | CmpOp::Eq | CmpOp::In);
        let c = self.atom(atom, whole);
        if positive { c } else { c.not() }
    }

    fn type_is_builtin(&self, candidate: &Expr, class: &Expr) -> Option<(String, String)> {
        let Expr::Call(call) = candidate else {
            return None;
        };
        let Expr::Name(function) = call.func.as_ref() else {
            return None;
        };
        if !self
            .runtime
            .builtin_type
            .contains(&Span::from(function.range()))
            || call.arguments.args.len() != 1
            || !call.arguments.keywords.is_empty()
        {
            return None;
        }
        let Expr::Name(class_name) = class else {
            return None;
        };
        let name = class_name.id.as_str();
        if !self
            .runtime
            .builtin_classes
            .get(name)?
            .contains(&Span::from(class.range()))
        {
            return None;
        }
        Some((
            self.place(&call.arguments.args[0])?,
            self.resources.string(name),
        ))
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
        let mut classes = Vec::new();
        match &call.arguments.args[1] {
            Expr::Tuple(t) => {
                for e in &t.elts {
                    self.resources.push(&mut classes, e);
                }
            }
            e => self.resources.push(&mut classes, e),
        };
        let mut out = Condition::terminal(self.resources.budget, Some(false));
        for c in classes {
            if !matches!(c, Expr::Name(_) | Expr::Attribute(_)) {
                return Some(self.opaque(whole));
            }
            let class = self.resources.compact(self.source(c.range()), "");
            out = out.or(&self.atom(
                Atom::IsInstance {
                    place: self.resources.string(&place),
                    class,
                },
                whole,
            ));
        }
        Some(out)
    }

    /// The exact operand whose value a modeled leaf tests. This follows only expression forms
    /// `test` lowers recursively, and never selects an arbitrary use inside a containing span.
    pub(crate) fn tested_place_operand(&self, e: &Expr, leaf: &Atom) -> Option<Span> {
        let Atom::Evaluated {
            atom,
            site: EvaluationSite::Source(span),
        } = leaf
        else {
            return None;
        };
        self.find_tested_place(e, *span, atom)
    }

    fn find_tested_place(&self, e: &Expr, site: Span, atom: &Atom) -> Option<Span> {
        if Span::from(e.range()) == site {
            let place = atom.place()?;
            return match (atom, e) {
                (Atom::Truthy { .. }, _) if self.place(e).as_deref() == Some(place) => {
                    Some(Span::from(e.range()))
                }
                (
                    Atom::IsNone { .. }
                    | Atom::IsValue { .. }
                    | Atom::Equals { .. }
                    | Atom::MemberOf { .. },
                    Expr::Compare(c),
                ) if c.ops.len() == 1 => {
                    let left = &c.operands[0];
                    let right = &c.operands[1];
                    let left_is_place = self.place(left).as_deref() == Some(place)
                        && (literal(right, self.resources).is_some()
                            || literal_set(right, self.resources).is_some());
                    let right_is_place = self.place(right).as_deref() == Some(place)
                        && literal(left, self.resources).is_some();
                    match (left_is_place, right_is_place) {
                        (true, false) => Some(Span::from(left.range())),
                        (false, true) => Some(Span::from(right.range())),
                        _ => None,
                    }
                }
                (Atom::IsInstance { .. }, Expr::Call(call))
                    if matches!(call.func.as_ref(), Expr::Name(f) if f.id.as_str() == "isinstance")
                        && call.arguments.args.len() == 2
                        && call.arguments.keywords.is_empty()
                        && self.place(&call.arguments.args[0]).as_deref() == Some(place) =>
                {
                    Some(Span::from(call.arguments.args[0].range()))
                }
                (Atom::TypeIs { .. }, Expr::Compare(compare))
                    if compare.ops.len() == 1
                        && matches!(compare.ops[0], CmpOp::Is | CmpOp::IsNot) =>
                {
                    let (candidate, class) = (&compare.operands[0], &compare.operands[1]);
                    self.type_is_builtin(candidate, class)
                        .filter(|(tested, _)| tested == place)
                        .and_then(|_| match candidate {
                            Expr::Call(call) => Some(Span::from(call.arguments.args[0].range())),
                            _ => None,
                        })
                        .or_else(|| {
                            self.type_is_builtin(class, candidate)
                                .filter(|(tested, _)| tested == place)
                                .and_then(|_| match class {
                                    Expr::Call(call) => {
                                        Some(Span::from(call.arguments.args[0].range()))
                                    }
                                    _ => None,
                                })
                        })
                }
                _ => None,
            };
        }
        match e {
            Expr::BoolOp(b) => b
                .values
                .iter()
                .find_map(|value| self.find_tested_place(value, site, atom)),
            Expr::UnaryOp(u) if u.op == ast::UnaryOp::Not => {
                self.find_tested_place(&u.operand, site, atom)
            }
            Expr::If(i) => self
                .find_tested_place(&i.test, site, atom)
                .or_else(|| self.find_tested_place(&i.body, site, atom))
                .or_else(|| self.find_tested_place(&i.orelse, site, atom)),
            Expr::Named(n) => self.find_tested_place(&n.target, site, atom),
            _ => None,
        }
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
                    let want = int_tuple(right, self.resources)?;
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
                let Some(Value::Str(s)) = literal(right, self.resources) else {
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
                    && let Some(Value::Str(s)) = literal(&call.arguments.args[0], self.resources)
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
    pub(crate) fn predicate(&self, p: &Predicate, synthetic: &EvaluationSite) -> Condition {
        let c = match &p.node {
            PredicateNode::Expression(x)
            | PredicateNode::Condition(x)
            | PredicateNode::ChainedComparisonCondition(x) => {
                self.test(x.node_ref(self.db).node(self.module))
            }
            PredicateNode::IsNonTerminalCall(call) => {
                let expression = call.call_expr(self.db).node_ref(self.db).node(self.module);
                self.atom(
                    Atom::NonTerminalCall {
                        awaiting: call.is_await(self.db),
                    },
                    expression,
                )
            }
            PredicateNode::IsNonEmptyIterable(expression) => {
                let expression = expression.node_ref(self.db).node(self.module);
                self.atom(Atom::NonEmptyIterable, expression)
            }
            PredicateNode::ContextManagerSuppresses {
                expression,
                is_async,
            } => {
                let expression = expression.node_ref(self.db).node(self.module);
                self.atom(
                    Atom::ContextManagerSuppresses {
                        asynchronous: *is_async,
                    },
                    expression,
                )
            }
            PredicateNode::FinallyNormalPathImpossible { .. } => Condition::atom(
                self.resources.budget,
                Atom::FinallyNormalPathImpossible.evaluated_in(self.resources, synthetic.clone()),
            ),
            PredicateNode::Pattern(pattern) => {
                let subject = pattern.subject(self.db).node_ref(self.db).node(self.module);
                let mut c =
                    self.pattern(subject, pattern.kind(self.db), &self.pattern_site(pattern));
                if let Some(guard) = pattern.guard(self.db) {
                    c = c.and(&self.test(guard.node_ref(self.db).node(self.module)));
                }
                c
            }
            PredicateNode::OrPatternAlternative(_)
            | PredicateNode::SubjectElementPattern(_)
            | PredicateNode::StarImportPlaceholder(_) => Condition::atom(
                self.resources.budget,
                Atom::opaque_in(self.resources, UNDECIDED)
                    .evaluated_in(self.resources, synthetic.clone()),
            ),
        };
        if p.is_positive { c } else { c.not() }
    }

    pub(crate) fn pattern(
        &self,
        subject: &Expr,
        kind: &PatternPredicateKind,
        evaluation: &EvaluationSite,
    ) -> Condition {
        let Some(place) = self.place(subject) else {
            return Condition::atom(
                self.resources.budget,
                Atom::opaque_in(self.resources, &self.uncommented(subject.range()))
                    .evaluated_in(self.resources, evaluation.clone()),
            );
        };
        match kind {
            PatternPredicateKind::Singleton(ast::Singleton::None) => Condition::atom(
                self.resources.budget,
                Atom::IsNone { place }.evaluated_in(self.resources, evaluation.clone()),
            ),
            PatternPredicateKind::Singleton(s) => Condition::atom(
                self.resources.budget,
                Atom::IsValue {
                    place,
                    value: Value::Bool(matches!(s, ast::Singleton::True)),
                }
                .evaluated_in(self.resources, evaluation.clone()),
            ),
            PatternPredicateKind::Value(v) => {
                match literal(v.node_ref(self.db).node(self.module), self.resources) {
                    Some(value) => Condition::atom(
                        self.resources.budget,
                        Atom::Equals { place, value }
                            .evaluated_in(self.resources, evaluation.clone()),
                    ),
                    None => Condition::atom(
                        self.resources.budget,
                        Atom::opaque_in(
                            self.resources,
                            &self.uncommented(v.node_ref(self.db).node(self.module).range()),
                        )
                        .evaluated_in(self.resources, evaluation.clone()),
                    ),
                }
            }
            PatternPredicateKind::Or(alternatives) => alternatives.iter().fold(
                Condition::terminal(self.resources.budget, Some(false)),
                |acc, k| acc.or(&self.pattern(subject, k, evaluation)),
            ),
            PatternPredicateKind::Class(class) => {
                let c = class.class.node_ref(self.db).node(self.module);
                let atom = Condition::atom(
                    self.resources.budget,
                    Atom::IsInstance {
                        place,
                        class: self.resources.compact(self.source(c.range()), ""),
                    }
                    .evaluated_in(self.resources, evaluation.clone()),
                );
                if class.is_empty() {
                    atom
                } else {
                    atom.and(&Condition::atom(
                        self.resources.budget,
                        Atom::opaque_in(self.resources, UNDECIDED)
                            .evaluated_in(self.resources, evaluation.clone()),
                    ))
                }
            }
            PatternPredicateKind::As(Some(inner), _) => self.pattern(subject, inner, evaluation),
            PatternPredicateKind::As(None, _) | PatternPredicateKind::Star(_) => {
                Condition::terminal(self.resources.budget, Some(true))
            }
            PatternPredicateKind::Mapping(_) | PatternPredicateKind::Sequence(_) => self
                .pattern_opaque(subject, evaluation)
                .and(&Condition::atom(
                    self.resources.budget,
                    Atom::opaque_in(self.resources, UNDECIDED)
                        .evaluated_in(self.resources, evaluation.clone()),
                )),
        }
    }

    fn pattern_opaque(&self, subject: &Expr, evaluation: &EvaluationSite) -> Condition {
        Condition::atom(
            self.resources.budget,
            Atom::opaque_in(self.resources, &self.uncommented(subject.range()))
                .evaluated_in(self.resources, evaluation.clone()),
        )
    }
}

/// A reachability diagram's condition, memoized per diagram node of one scope's use-def map.
pub(crate) fn diagram(
    t: &Translator<'_>,
    _fid: FileScopeId,
    map: &UseDefMap<'_>,
    memo: &mut HashMap<ScopedReachabilityConstraintId, Condition>,
    id: ScopedReachabilityConstraintId,
    charge: &mut StateCharge,
) -> Condition {
    diagram_at(t, map, memo, id, 0, charge)
}
fn diagram_at(
    t: &Translator<'_>,
    map: &UseDefMap<'_>,
    memo: &mut HashMap<ScopedReachabilityConstraintId, Condition>,
    id: ScopedReachabilityConstraintId,
    depth: usize,
    charge: &mut StateCharge,
) -> Condition {
    t.resources.checkpoint();
    if depth > 256 || memo.len() > 4096 {
        return Condition::terminal(t.resources.budget, None);
    }
    if id == ScopedReachabilityConstraintId::ALWAYS_TRUE {
        return Condition::terminal(t.resources.budget, Some(true));
    }
    if id == ScopedReachabilityConstraintId::ALWAYS_FALSE {
        return Condition::terminal(t.resources.budget, Some(false));
    }
    // ty's third value: reachability it leaves undecided (a `try` body, a loop over an unknown
    // iterable, a `with` exit). Verdicts state may-behavior, so it reads as `true` (ADR-0022
    // §Conditions; the Stage 2 review's F2).
    if id == ScopedReachabilityConstraintId::AMBIGUOUS {
        return Condition::terminal(t.resources.budget, Some(true)).with_approximation();
    }
    if let Some(c) = memo.get(&id) {
        return c.clone();
    }
    let node = map.reachability_constraints().get_interior_node(id);
    let synthetic = EvaluationSite::Unavailable {
        reason: "native synthetic predicate has no source coordinate",
    };
    let p = t.predicate(&map.predicates()[node.atom()], &synthetic);
    let on_true = diagram_at(t, map, memo, node.if_true(), depth + 1, charge);
    let on_false = diagram_at(t, map, memo, node.if_false(), depth + 1, charge);
    let c = p.and(&on_true).or(&p.not().and(&on_false));
    hash_slot(memo, charge);
    memo.insert(id, c.clone());
    c
}

/// Whether a diagram depends on a test our resolved runtime view decided. A false root is
/// counted as ty's false; otherwise this distinguishes a runtime-view skip from a contradiction
/// in translated stable atoms. The categories are diagnostic, in that precedence order.
pub(crate) fn runtime_in_diagram(
    t: &Translator<'_>,
    map: &UseDefMap<'_>,
    memo: &mut HashMap<ScopedReachabilityConstraintId, bool>,
    id: ScopedReachabilityConstraintId,
    charge: &mut StateCharge,
) -> bool {
    runtime_in_diagram_bounded(t, map, memo, id, 0, charge)
}
fn runtime_in_diagram_bounded(
    t: &Translator<'_>,
    map: &UseDefMap<'_>,
    memo: &mut HashMap<ScopedReachabilityConstraintId, bool>,
    id: ScopedReachabilityConstraintId,
    depth: usize,
    charge: &mut StateCharge,
) -> bool {
    t.resources.checkpoint();
    if depth >= 256 || memo.len() >= 4096 {
        return false;
    }
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
        || runtime_in_diagram_bounded(t, map, memo, node.if_true(), depth + 1, charge)
        || runtime_in_diagram_bounded(t, map, memo, node.if_false(), depth + 1, charge);
    hash_slot(memo, charge);
    memo.insert(id, found);
    found
}

/// A literal the condition language holds: `None`, `True`, `False`, an integer or a string.
pub(crate) fn literal(e: &Expr, resources: &Resources<'_>) -> Option<Value> {
    match e {
        Expr::NoneLiteral(_) => Some(Value::None),
        Expr::BooleanLiteral(b) => Some(Value::Bool(b.value)),
        Expr::NumberLiteral(n) => match &n.value {
            ast::Number::Int(i) => i.as_i64().map(Value::Int),
            _ => None,
        },
        Expr::UnaryOp(u) if u.op == ast::UnaryOp::USub => match literal(&u.operand, resources)? {
            Value::Int(i) => i.checked_neg().map(Value::Int),
            _ => None,
        },
        Expr::StringLiteral(s) => Some(Value::Str(resources.string(s.value.to_str()))),
        _ => None,
    }
}

/// A tuple, list or set of literals.
fn literal_set(e: &Expr, resources: &Resources<'_>) -> Option<Vec<Value>> {
    let elts = match e {
        Expr::Tuple(t) => &t.elts,
        Expr::List(l) => &l.elts,
        Expr::Set(s) => &s.elts,
        _ => return None,
    };
    let mut values = Vec::new();
    for e in elts {
        resources.push(&mut values, literal(e, resources)?);
    }
    Some(values)
}

fn int_tuple(e: &Expr, resources: &Resources<'_>) -> Option<Vec<i64>> {
    let Expr::Tuple(t) = e else { return None };
    let mut values = Vec::new();
    for x in &t.elts {
        let Value::Int(i) = literal(x, resources)? else {
            return None;
        };
        resources.push(&mut values, i);
    }
    Some(values)
}
