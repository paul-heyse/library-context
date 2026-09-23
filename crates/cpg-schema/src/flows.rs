//! Pass B's and Pass C's declared relations (DESIGN §9.2, §9.3): the argument flows, the
//! parameter guards, the parameter reads Pass B does not follow, the receiver parameters and the
//! usage handoffs, as SQL over the CPG (C2 syntax, C3 bindings, the invocation arcs' call targets),
//! stated before any worklist runs. Their digest joins the compiler digest.
//!
//! **Arcs.** Call targets under the invocation projection's accepted modality, origin and fidelity
//! (`projection::invocation`), phase `call` or `init`, into a function.
//!
//! **The argument → formal mapping** (one fragment, `maps_formal`, for Pass B and Pass C): per
//! candidate target, the implicit receiver counted, a positional argument before any `*` argument
//! maps to the positional formal at its index (plus one for a bound receiver), and a keyword
//! argument to the positional-or-keyword or keyword-only formal of that name. A starred argument,
//! a positional one after it, a `**` argument, and one no formal takes (a `*args` or `**kwargs`
//! catch-all) are never mapped.
//!
//! **Argument flows.** One row per mapped argument of an arc, with how its value arises:
//! - `parameter`: a name bound, once in its scope, by a parameter of the caller;
//! - `alias`: a name bound once, by an assignment directly in the caller's body, from such a
//!   parameter (one identity alias in straight-line code);
//! - `literal`: a string, number, boolean or `None` literal, as written;
//! - `other`: anything else, never followed.
//!
//! Each row says where its call site lies in the caller (slice 2.1 review F1): inside a `try` or
//! a `with` (`may_catch`: a handler or a context manager may absorb what the callee raises);
//! inside a conditional construct (`conditional`: an `if`, loop, `match`, conditional expression,
//! boolean operator, comprehension, lambda or `except` clause, so the caller makes the call only on
//! some paths); and whether such a construct also reads the flowing value outside the call
//! (`value_tested`: the caller may call only for values the callee accepts).
//!
//! **Guards.** One row per `if` directly in a function's body that tests one of its parameters
//! (bound once) and holds a `raise` directly in its body, the test a **supported predicate**
//! (review F5): names of that function's parameters and of builtins, literals, comparisons,
//! boolean, unary and binary operators, tuples, lists and sets, and calls whose callee is a
//! builtin's name. An attribute, subscript, other call, lambda, comprehension or walrus is not
//! supported. A guard on a rebound parameter is no guard of its argument.
//!
//! **Parameter reads** (review F4). One row per argument of an arc whose value reads a name bound
//! in the caller's scope under one of its parameters' names, with whether that name is rebound in
//! the scope, whether the value is the bare name, and whether the argument is unpacked. Pass B
//! reports those it does not follow as `unfollowed_argument`.
//!
//! **Receivers** (review F8). A method's first positional parameter, unless Pysa says the method
//! is static; decided by the declaration's kind, never by the parameter's name.

use crate::codebook::{
    ArgumentKind, BindingKind, Codebook, EdgeKind, ImplicitReceiver, InvocationPhase, NodeKind,
    ParameterKind, SourceRole, SyntaxField, SyntaxKind,
};
use crate::id::{Digest, IdHasher};

/// How an argument's value arises (`value_class`).
pub mod value_class {
    pub const PARAMETER: i16 = 0;
    pub const ALIAS: i16 = 1;
    pub const LITERAL: i16 = 2;
    pub const OTHER: i16 = 3;
}

fn list(xs: &[i16]) -> String {
    xs.iter().map(i16::to_string).collect::<Vec<_>>().join(", ")
}

pub(crate) fn codes<C: Codebook>(xs: &[C]) -> String {
    list(&xs.iter().map(|c| c.code()).collect::<Vec<_>>())
}

/// The invocation projection's accepted evidence, over the facts row `f`.
fn accepted() -> String {
    let spec = crate::projection::invocation();
    format!(
        "f.modality IN ({}) AND f.origin IN ({}) AND f.fidelity IN ({})",
        codes(spec.modalities),
        codes(spec.origins),
        codes(spec.fidelities)
    )
}

/// The call targets the relations read, one row per (call site, target, edge): the evidence's
/// modality, the phase and whether the target binds an implicit receiver.
pub(crate) fn call_targets() -> String {
    format!(
        "SELECT ct.src_node_id AS call_site_node_id, ct.dst_node_id AS target_node_id, \
                ct.edge_id, f.modality, p.phase, \
                CASE WHEN p.implicit_receiver IN ({receivers}) THEN 1 ELSE 0 END AS receiver \
         FROM edges ct \
         JOIN pysa_calls p ON p.fact_id = ct.evidence_fact_id \
         JOIN facts f ON f.fact_id = ct.evidence_fact_id \
         WHERE ct.edge_kind = {call_target} AND ct.dst_kind = {function} AND {accepted} \
           AND p.phase IN ({call}, {init})",
        receivers = codes(&[
            ImplicitReceiver::TrueWithClassReceiver,
            ImplicitReceiver::TrueWithObjectReceiver,
        ]),
        call_target = EdgeKind::CallTarget.code(),
        function = NodeKind::Function.code(),
        accepted = accepted(),
        call = InvocationPhase::Call.code(),
        init = InvocationPhase::Init.code(),
    )
}

/// The call targets with their enclosing caller (`encloses_call`).
fn arcs() -> String {
    format!(
        "SELECT ec.src_node_id AS caller_node_id, t.call_site_node_id, t.target_node_id, \
                t.edge_id, t.modality, t.phase, t.receiver \
         FROM ({targets}) t \
         JOIN edges ec ON ec.dst_node_id = t.call_site_node_id AND ec.edge_kind = {encloses}",
        targets = call_targets(),
        encloses = EdgeKind::EnclosesCall.code(),
    )
}

/// The first starred argument of each call.
fn starred() -> String {
    format!(
        "SELECT call_node_id, min(ordinal) AS first FROM arguments \
         WHERE kind = {} GROUP BY call_node_id",
        ArgumentKind::Starred.code()
    )
}

/// The argument → formal join condition: formal `fm` (`parameter_syntax`) of the target takes
/// argument `arg` (its `kind`, `ordinal` and `keyword`), given the call's first starred argument
/// `star` (`first`, NULL when none) and the arc's `receiver` expression.
fn maps_formal(fm: &str, arg: &str, star: &str, receiver: &str) -> String {
    format!(
        "(({arg}.kind = {positional} AND ({star}.first IS NULL OR {arg}.ordinal < {star}.first) \
           AND {fm}.kind IN ({posonly}, {pos_or_kw}) AND {fm}.ordinal = {arg}.ordinal + {receiver}) \
          OR ({arg}.kind = {keyword} AND {fm}.name = {arg}.keyword \
           AND {fm}.kind IN ({pos_or_kw}, {kw_only})))",
        positional = ArgumentKind::Positional.code(),
        keyword = ArgumentKind::Keyword.code(),
        posonly = ParameterKind::PositionalOnly.code(),
        pos_or_kw = ParameterKind::PositionalOrKeyword.code(),
        kw_only = ParameterKind::KeywordOnly.code(),
    )
}

/// Names bound exactly once in their scope, and the parameters among them.
fn single_bindings() -> String {
    format!(
        "single AS (SELECT scope_id, name FROM bindings GROUP BY scope_id, name \
                    HAVING count(*) = 1), \
         params AS (SELECT b.node_id AS binding_id, b.site_node_id AS parameter_node_id, \
                           b.scope_id, b.name \
                    FROM bindings b JOIN single s ON s.scope_id = b.scope_id AND s.name = b.name \
                    WHERE b.kind = {parameter})",
        parameter = BindingKind::Parameter.code()
    )
}

/// The syntax kinds inside which a call happens only on some paths of its caller.
const CONDITIONAL: &[SyntaxKind] = &[
    SyntaxKind::StmtIf,
    SyntaxKind::StmtWhile,
    SyntaxKind::StmtFor,
    SyntaxKind::StmtMatch,
    SyntaxKind::ExprIf,
    SyntaxKind::ExprBoolOp,
    SyntaxKind::ExprListComp,
    SyntaxKind::ExprSetComp,
    SyntaxKind::ExprDictComp,
    SyntaxKind::ExprGenerator,
    SyntaxKind::ExprLambda,
    SyntaxKind::ExceptHandlerExceptHandler,
];

/// The syntax kinds a supported guard predicate is built from (review F5).
const PREDICATE: &[SyntaxKind] = &[
    SyntaxKind::ExprName,
    SyntaxKind::ExprCompare,
    SyntaxKind::ExprBoolOp,
    SyntaxKind::ExprUnaryOp,
    SyntaxKind::ExprBinOp,
    SyntaxKind::ExprCall,
    SyntaxKind::ExprTuple,
    SyntaxKind::ExprList,
    SyntaxKind::ExprSet,
    SyntaxKind::ExprStringLiteral,
    SyntaxKind::ExprBytesLiteral,
    SyntaxKind::ExprNumberLiteral,
    SyntaxKind::ExprBooleanLiteral,
    SyntaxKind::ExprNoneLiteral,
    SyntaxKind::ExprEllipsisLiteral,
];

/// The argument-flows query, totally ordered by `(caller, call site, edge, argument)`.
pub fn argument_flows_sql() -> String {
    let literals = codes(&[
        SyntaxKind::ExprStringLiteral,
        SyntaxKind::ExprNumberLiteral,
        SyntaxKind::ExprBooleanLiteral,
        SyntaxKind::ExprNoneLiteral,
    ]);
    let within = |outer: &str, inner: &str| {
        format!(
            "{outer}.module_node_id = {inner}.module_node_id \
             AND {outer}.owner_node_id = {inner}.owner_node_id \
             AND {outer}.start_byte <= {inner}.start_byte AND {inner}.end_byte <= {outer}.end_byte"
        )
    };
    format!(
        "WITH {single}, \
         arcs AS ({arcs}), \
         starred AS ({starred}), \
         valued AS ( \
           SELECT a.node_id AS argument_node_id, a.call_node_id, a.ordinal, a.kind, a.keyword, \
                  s.kind AS value_kind, s.detail AS value_detail, s.node_id AS value_node_id \
           FROM arguments a \
           JOIN edges v ON v.edge_kind = {argument_value} AND v.src_node_id = a.node_id \
           JOIN syntax_nodes s ON s.node_id = v.dst_node_id), \
         resolved AS ( \
           SELECT v.*, b.node_id AS binding_id, b.kind AS binding_kind \
           FROM valued v \
           LEFT JOIN references rf ON rf.name_node_id = v.value_node_id \
           LEFT JOIN reference_resolutions rr ON rr.reference_id = rf.node_id \
                 AND NOT rr.captured \
           LEFT JOIN bindings b ON b.node_id = rr.binding_id), \
         aliases AS ( \
           SELECT b.node_id AS binding_id, p.parameter_node_id, p.binding_id AS source_binding_id, \
                  b.name AS alias_name \
           FROM bindings b \
           JOIN single s1 ON s1.scope_id = b.scope_id AND s1.name = b.name \
           JOIN syntax_nodes tgt ON tgt.node_id = b.site_node_id \
           JOIN syntax_nodes st ON st.node_id = tgt.parent_node_id AND st.kind = {assign} \
           JOIN declarations fn ON fn.node_id = st.parent_node_id \
           JOIN syntax_nodes rhs ON rhs.module_node_id = b.module_node_id \
                AND rhs.start_byte = b.value_start_byte AND rhs.end_byte = b.value_end_byte \
                AND rhs.kind = {name} \
           JOIN references rf ON rf.name_node_id = rhs.node_id \
           JOIN reference_resolutions rr ON rr.reference_id = rf.node_id AND NOT rr.captured \
           JOIN params p ON p.binding_id = rr.binding_id \
           WHERE b.kind = {assignment}), \
         handlers AS ( \
           SELECT owner_node_id, module_node_id, start_byte, end_byte FROM syntax_nodes \
           WHERE kind IN ({handlers})), \
         may_catch AS ( \
           SELECT DISTINCT cs.node_id FROM call_syntax cs JOIN handlers t ON {in_handler}), \
         conds AS ( \
           SELECT owner_node_id, module_node_id, start_byte, end_byte FROM syntax_nodes \
           WHERE kind IN ({conditional})), \
         conditional AS ( \
           SELECT DISTINCT cs.node_id FROM call_syntax cs JOIN conds c ON {in_cond}), \
         mapped AS ( \
           SELECT a.caller_node_id, a.call_site_node_id, a.target_node_id, a.edge_id, \
                  a.modality, a.phase, r.argument_node_id, fm.node_id AS formal_node_id, \
                  fm.name AS formal_name, \
                  CASE WHEN pr.parameter_node_id IS NOT NULL THEN {c_param} \
                       WHEN al.parameter_node_id IS NOT NULL THEN {c_alias} \
                       WHEN r.value_kind IN ({literals}) THEN {c_literal} \
                       ELSE {c_other} END AS value_class, \
                  COALESCE(pr.parameter_node_id, al.parameter_node_id) \
                    AS source_parameter_node_id, \
                  COALESCE(pr.binding_id, al.source_binding_id) AS source_binding_id, \
                  r.binding_id AS value_binding_id, \
                  al.alias_name, \
                  CASE WHEN r.value_kind IN ({literals}) THEN r.value_detail END AS value_text, \
                  mc.node_id IS NOT NULL AS may_catch, \
                  cd.node_id IS NOT NULL AS conditional \
           FROM arcs a \
           JOIN resolved r ON r.call_node_id = a.call_site_node_id \
           LEFT JOIN starred sr ON sr.call_node_id = a.call_site_node_id \
           JOIN parameter_syntax fm ON fm.function_node_id = a.target_node_id \
             AND {maps} \
           LEFT JOIN params pr ON pr.binding_id = r.binding_id \
           LEFT JOIN aliases al ON al.binding_id = r.binding_id \
           LEFT JOIN may_catch mc ON mc.node_id = a.call_site_node_id \
           LEFT JOIN conditional cd ON cd.node_id = a.call_site_node_id), \
         followed AS ( \
           SELECT call_site_node_id, argument_node_id, source_binding_id AS binding_id \
           FROM mapped WHERE source_binding_id IS NOT NULL AND conditional \
           UNION \
           SELECT call_site_node_id, argument_node_id, value_binding_id AS binding_id \
           FROM mapped WHERE source_binding_id IS NOT NULL AND conditional), \
         tested AS ( \
           SELECT DISTINCT fw.call_site_node_id, fw.argument_node_id \
           FROM followed fw \
           JOIN call_syntax cs ON cs.node_id = fw.call_site_node_id \
           JOIN reference_resolutions rr ON rr.binding_id = fw.binding_id AND NOT rr.captured \
           JOIN references rf ON rf.node_id = rr.reference_id \
           JOIN syntax_nodes n ON n.node_id = rf.name_node_id \
           JOIN conds c ON {in_cond} AND c.start_byte <= n.start_byte AND n.end_byte <= c.end_byte \
           WHERE n.end_byte <= cs.start_byte OR n.start_byte >= cs.end_byte) \
         SELECT DISTINCT m.caller_node_id, m.call_site_node_id, m.target_node_id, m.edge_id, \
                m.modality, m.phase, m.argument_node_id, m.formal_node_id, m.formal_name, \
                m.value_class, m.source_parameter_node_id, m.alias_name, m.value_text, \
                m.may_catch, m.conditional, t.argument_node_id IS NOT NULL AS value_tested \
         FROM mapped m \
         LEFT JOIN tested t ON t.call_site_node_id = m.call_site_node_id \
           AND t.argument_node_id = m.argument_node_id \
         ORDER BY caller_node_id, call_site_node_id, edge_id, argument_node_id, formal_node_id",
        single = single_bindings(),
        arcs = arcs(),
        starred = starred(),
        maps = maps_formal("fm", "r", "sr", "a.receiver"),
        in_handler = within("t", "cs"),
        in_cond = within("c", "cs"),
        handlers = codes(&[SyntaxKind::StmtTry, SyntaxKind::StmtWith]),
        conditional = codes(CONDITIONAL),
        argument_value = EdgeKind::ArgumentValue.code(),
        assign = SyntaxKind::StmtAssign.code(),
        name = SyntaxKind::ExprName.code(),
        assignment = BindingKind::Assignment.code(),
        c_param = value_class::PARAMETER,
        c_alias = value_class::ALIAS,
        c_literal = value_class::LITERAL,
        c_other = value_class::OTHER,
    )
}

/// The guards query, totally ordered by `(function, if, raise, parameter)`.
pub fn guards_sql() -> String {
    format!(
        "WITH {single}, \
         ifs AS ( \
           SELECT i.node_id AS if_node_id, i.owner_node_id AS function_node_id, i.module_node_id \
           FROM syntax_nodes i JOIN declarations d ON d.node_id = i.parent_node_id \
           WHERE i.kind = {if_} AND i.field = {body}), \
         raises AS ( \
           SELECT r.node_id AS raise_node_id, r.parent_node_id AS if_node_id, \
                  r.start_byte AS raise_start_byte, r.end_byte AS raise_end_byte \
           FROM syntax_nodes r WHERE r.kind = {raise} AND r.field = {body}), \
         tests AS ( \
           SELECT t.node_id AS test_node_id, t.parent_node_id AS if_node_id, \
                  t.start_byte AS test_start_byte, t.end_byte AS test_end_byte, t.module_node_id \
           FROM syntax_nodes t WHERE t.field = {test}), \
         candidates AS ( \
           SELECT DISTINCT ifs.if_node_id, ifs.function_node_id, tests.module_node_id, \
                  tests.test_start_byte, tests.test_end_byte \
           FROM ifs JOIN tests ON tests.if_node_id = ifs.if_node_id \
           JOIN raises ON raises.if_node_id = ifs.if_node_id), \
         unsupported AS ( \
           SELECT DISTINCT c.if_node_id FROM candidates c \
           JOIN syntax_nodes x ON x.module_node_id = c.module_node_id \
                AND x.owner_node_id = c.function_node_id \
                AND x.start_byte >= c.test_start_byte AND x.end_byte <= c.test_end_byte \
           WHERE x.kind NOT IN ({predicate})), \
         names AS ( \
           SELECT c.if_node_id, n.node_id AS name_node_id, n.field, p.parameter_node_id, \
                  rr.builtin_name \
           FROM candidates c \
           JOIN syntax_nodes n ON n.module_node_id = c.module_node_id \
                AND n.owner_node_id = c.function_node_id AND n.kind = {name} \
                AND n.start_byte >= c.test_start_byte AND n.end_byte <= c.test_end_byte \
           LEFT JOIN references rf ON rf.name_node_id = n.node_id \
           LEFT JOIN reference_resolutions rr ON rr.reference_id = rf.node_id \
           LEFT JOIN params p ON p.binding_id = rr.binding_id AND NOT rr.captured), \
         supported AS ( \
           SELECT if_node_id FROM names GROUP BY if_node_id \
           HAVING count(*) = count(parameter_node_id) + count(builtin_name) \
              AND count(parameter_node_id) > 0 \
              AND sum(CASE WHEN field = {callee} AND builtin_name IS NULL THEN 1 ELSE 0 END) \
                  = 0) \
         SELECT DISTINCT ifs.function_node_id, ifs.if_node_id, tests.test_node_id, \
                raises.raise_node_id, names.parameter_node_id, ifs.module_node_id, \
                tests.test_start_byte, tests.test_end_byte, raises.raise_start_byte, \
                raises.raise_end_byte \
         FROM ifs JOIN supported s ON s.if_node_id = ifs.if_node_id \
         LEFT ANTI JOIN unsupported u ON u.if_node_id = ifs.if_node_id \
         JOIN tests ON tests.if_node_id = ifs.if_node_id \
         JOIN raises ON raises.if_node_id = ifs.if_node_id \
         JOIN names ON names.if_node_id = ifs.if_node_id AND names.parameter_node_id IS NOT NULL \
         ORDER BY ifs.function_node_id, ifs.if_node_id, raises.raise_node_id, \
                  names.parameter_node_id",
        single = single_bindings(),
        if_ = SyntaxKind::StmtIf.code(),
        raise = SyntaxKind::StmtRaise.code(),
        name = SyntaxKind::ExprName.code(),
        body = SyntaxField::Body.code(),
        test = SyntaxField::Test.code(),
        callee = SyntaxField::Callee.code(),
        predicate = codes(PREDICATE),
    )
}

/// The parameter-reads query (review F4), totally ordered by `(caller, call site, edge,
/// argument, parameter)`: every argument of an arc whose value reads a name bound in the caller's
/// scope under a parameter's name.
pub fn parameter_reads_sql() -> String {
    format!(
        "WITH single AS (SELECT scope_id, name FROM bindings GROUP BY scope_id, name \
                         HAVING count(*) = 1), \
         arcs AS ({arcs}), \
         declared AS ( \
           SELECT b.site_node_id AS parameter_node_id, b.scope_id, b.name, \
                  s.name IS NULL AS rebound \
           FROM bindings b LEFT JOIN single s ON s.scope_id = b.scope_id AND s.name = b.name \
           WHERE b.kind = {parameter}), \
         valued AS ( \
           SELECT a.node_id AS argument_node_id, a.call_node_id, a.kind, \
                  s.node_id AS value_node_id, s.module_node_id, s.owner_node_id, s.start_byte, \
                  s.end_byte \
           FROM arguments a \
           JOIN edges v ON v.edge_kind = {argument_value} AND v.src_node_id = a.node_id \
           JOIN syntax_nodes s ON s.node_id = v.dst_node_id), \
         reads AS ( \
           SELECT DISTINCT v.argument_node_id, v.call_node_id, v.kind, d.parameter_node_id, \
                  d.rebound, n.node_id = v.value_node_id AS bare \
           FROM valued v \
           JOIN syntax_nodes n ON n.module_node_id = v.module_node_id \
                AND n.owner_node_id = v.owner_node_id AND n.kind = {name} \
                AND n.start_byte >= v.start_byte AND n.end_byte <= v.end_byte \
           JOIN references rf ON rf.name_node_id = n.node_id \
           JOIN reference_resolutions rr ON rr.reference_id = rf.node_id AND NOT rr.captured \
           JOIN bindings b ON b.node_id = rr.binding_id \
           JOIN declared d ON d.scope_id = b.scope_id AND d.name = b.name) \
         SELECT DISTINCT a.caller_node_id, a.call_site_node_id, a.target_node_id, a.edge_id, \
                a.modality, a.phase, r.argument_node_id, r.parameter_node_id, r.rebound, r.bare, \
                r.kind IN ({unpacked}) AS unpacked \
         FROM reads r JOIN arcs a ON a.call_site_node_id = r.call_node_id \
         ORDER BY caller_node_id, call_site_node_id, edge_id, argument_node_id, \
                  parameter_node_id",
        arcs = arcs(),
        parameter = BindingKind::Parameter.code(),
        argument_value = EdgeKind::ArgumentValue.code(),
        name = SyntaxKind::ExprName.code(),
        unpacked = codes(&[ArgumentKind::Starred, ArgumentKind::DoubleStarred]),
    )
}

/// The receiver parameters (review F8), ordered by parameter: a method's first positional
/// parameter unless Pysa says the method is static, whatever the parameter is called.
pub fn receivers_sql() -> String {
    format!(
        "SELECT DISTINCT ps.node_id AS parameter_node_id, ps.function_node_id \
         FROM parameter_syntax ps \
         JOIN provider_node_map m ON m.node_id = ps.function_node_id \
         JOIN pysa_functions f ON f.module_node_id = m.module_node_id \
           AND f.function_key = m.function_key \
         WHERE ps.ordinal = 0 AND ps.kind IN ({positional}) \
           AND f.defining_class IS NOT NULL AND NOT f.is_staticmethod \
         ORDER BY parameter_node_id",
        positional = codes(&[
            ParameterKind::PositionalOnly,
            ParameterKind::PositionalOrKeyword
        ]),
    )
}

/// Pass C's handoffs query (DESIGN §9.3), over the official usage code (examples, tests, doc
/// blocks), totally ordered by `(consumer, producer, formal, module path, consumer site)`.
///
/// One row per occurrence in which what a release callable returns reaches an argument of
/// another: either `x = producer(...)` with `x` bound once in its scope and read once, by
/// `consumer(..., x)` in a later statement of the same block (the call being that statement's
/// value, awaited or not: no `with` item, no nesting); or `consumer(..., producer(...))`. The
/// argument maps to one formal of the consumer by the flows' mapping (`maps_formal`). A read of
/// `x` as a receiver (`x.method()`, `@x.tool`) configures it: setup, not another consumer. A
/// reassigned `x`, one read anywhere else too (another argument, a return, a store), and a use
/// inside a `with` item are no handoff.
pub fn handoffs_sql() -> String {
    let usage = codes(&[SourceRole::Example, SourceRole::Test, SourceRole::DocBlock]);
    format!(
        "WITH usage AS (SELECT module_node_id, role, path FROM source_files \
                        WHERE role IN ({usage})), \
         single AS (SELECT scope_id, name FROM bindings GROUP BY scope_id, name \
                    HAVING count(*) = 1), \
         reads AS ( \
           SELECT rr.binding_id, count(*) AS n FROM reference_resolutions rr \
           JOIN references rf ON rf.node_id = rr.reference_id \
           JOIN syntax_nodes nm ON nm.node_id = rf.name_node_id \
           LEFT JOIN syntax_nodes par ON par.node_id = nm.parent_node_id \
           WHERE rr.binding_id IS NOT NULL \
             AND NOT (COALESCE(par.kind, -1) = {attribute} AND nm.field = {value_field}) \
           GROUP BY rr.binding_id), \
         targets AS ( \
           SELECT call_site_node_id AS site, target_node_id AS target, edge_id, modality, \
                  receiver \
           FROM ({targets})), \
         starred AS ({starred}), \
         bound AS ( \
           SELECT b.node_id AS binding_id, c.node_id AS producer_site, \
                  st.parent_node_id AS block, st.field AS block_field, st.ordinal AS after \
           FROM bindings b JOIN usage u ON u.module_node_id = b.module_node_id \
           JOIN single s ON s.scope_id = b.scope_id AND s.name = b.name \
           JOIN reads x ON x.binding_id = b.node_id AND x.n = 1 \
           JOIN syntax_nodes tgt ON tgt.node_id = b.site_node_id \
           JOIN syntax_nodes st ON st.node_id = tgt.parent_node_id AND st.kind = {assign} \
           JOIN call_syntax c ON c.module_node_id = b.module_node_id \
                AND c.start_byte = b.value_start_byte AND c.end_byte = b.value_end_byte \
           WHERE b.kind = {assignment}), \
         args AS ( \
           SELECT a.node_id AS argument_node_id, a.call_node_id AS consumer_site, a.ordinal, \
                  a.kind, a.keyword, v.dst_node_id AS value_node_id \
           FROM arguments a \
           JOIN edges v ON v.edge_kind = {argument_value} AND v.src_node_id = a.node_id), \
         statement_of AS ( \
           SELECT c.node_id AS site, \
                  CASE WHEN p1.kind IN ({expr_stmt}, {assign}) THEN p1.node_id \
                       WHEN p1.kind = {await_} AND p2.kind IN ({expr_stmt}, {assign}) \
                       THEN p2.node_id END AS statement_node_id \
           FROM syntax_nodes c JOIN syntax_nodes p1 ON p1.node_id = c.parent_node_id \
           LEFT JOIN syntax_nodes p2 ON p2.node_id = p1.parent_node_id \
           JOIN usage u ON u.module_node_id = c.module_node_id), \
         named AS ( \
           SELECT bd.producer_site, a.consumer_site, a.ordinal, a.kind, a.keyword, \
                  true AS named \
           FROM args a JOIN references rf ON rf.name_node_id = a.value_node_id \
           JOIN reference_resolutions rr ON rr.reference_id = rf.node_id \
           JOIN bound bd ON bd.binding_id = rr.binding_id \
           JOIN statement_of so ON so.site = a.consumer_site \
           JOIN syntax_nodes st ON st.node_id = so.statement_node_id \
           WHERE st.parent_node_id = bd.block AND st.field = bd.block_field \
             AND st.ordinal > bd.after), \
         nested AS ( \
           SELECT a.value_node_id AS producer_site, a.consumer_site, a.ordinal, a.kind, \
                  a.keyword, false AS named \
           FROM args a JOIN call_syntax c ON c.node_id = a.value_node_id \
           JOIN usage u ON u.module_node_id = c.module_node_id), \
         occurrences AS (SELECT * FROM named UNION ALL SELECT * FROM nested) \
         SELECT DISTINCT ct.target AS consumer_node_id, pt.target AS producer_node_id, \
                fm.node_id AS formal_node_id, fm.name AS formal_name, u.path, u.role, \
                cs.start_byte AS consumer_start_byte, o.consumer_site AS consumer_site_node_id, \
                o.producer_site AS producer_site_node_id, o.named, \
                ct.modality AS consumer_modality \
         FROM occurrences o \
         JOIN targets pt ON pt.site = o.producer_site \
         JOIN targets ct ON ct.site = o.consumer_site \
         JOIN call_syntax cs ON cs.node_id = o.consumer_site \
         JOIN usage u ON u.module_node_id = cs.module_node_id \
         LEFT JOIN starred sr ON sr.call_node_id = o.consumer_site \
         JOIN parameter_syntax fm ON fm.function_node_id = ct.target AND {maps} \
         ORDER BY consumer_node_id, producer_node_id, formal_node_id, u.path, \
                  consumer_start_byte, consumer_site_node_id, producer_site_node_id",
        targets = call_targets(),
        starred = starred(),
        maps = maps_formal("fm", "o", "sr", "ct.receiver"),
        argument_value = EdgeKind::ArgumentValue.code(),
        assign = SyntaxKind::StmtAssign.code(),
        expr_stmt = SyntaxKind::StmtExpr.code(),
        await_ = SyntaxKind::ExprAwait.code(),
        assignment = BindingKind::Assignment.code(),
        attribute = SyntaxKind::ExprAttribute.code(),
        value_field = SyntaxField::Value.code(),
    )
}
/// The relations' identity, for the compiler digest and each Pass B or C invocation.
pub fn digest() -> Digest {
    IdHasher::new("pass-b-relations")
        .str(&argument_flows_sql())
        .str(&guards_sql())
        .str(&parameter_reads_sql())
        .str(&receivers_sql())
        .str(&handoffs_sql())
        .finish_digest()
}

/// The declared output schemas: each result is cast to its schema (strictly) before the kernel
/// reads it.
pub mod schemas {
    use std::sync::Arc;

    use arrow_schema::{DataType, Field, Schema, SchemaRef};

    fn id(name: &str, nullable: bool) -> Field {
        Field::new(name, DataType::FixedSizeBinary(16), nullable)
    }

    pub fn flows() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("caller_node_id", false),
            id("call_site_node_id", false),
            id("target_node_id", false),
            id("edge_id", false),
            Field::new("modality", DataType::Int16, false),
            Field::new("phase", DataType::Int16, false),
            id("argument_node_id", false),
            id("formal_node_id", false),
            Field::new("formal_name", DataType::Utf8, false),
            Field::new("value_class", DataType::Int16, false),
            id("source_parameter_node_id", true),
            Field::new("alias_name", DataType::Utf8, true),
            Field::new("value_text", DataType::Utf8, true),
            Field::new("may_catch", DataType::Boolean, false),
            Field::new("conditional", DataType::Boolean, false),
            Field::new("value_tested", DataType::Boolean, false),
        ]))
    }

    pub fn parameter_reads() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("caller_node_id", false),
            id("call_site_node_id", false),
            id("target_node_id", false),
            id("edge_id", false),
            Field::new("modality", DataType::Int16, false),
            Field::new("phase", DataType::Int16, false),
            id("argument_node_id", false),
            id("parameter_node_id", false),
            Field::new("rebound", DataType::Boolean, false),
            Field::new("bare", DataType::Boolean, false),
            Field::new("unpacked", DataType::Boolean, false),
        ]))
    }

    pub fn receivers() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("parameter_node_id", false),
            id("function_node_id", false),
        ]))
    }

    pub fn handoffs() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("consumer_node_id", false),
            id("producer_node_id", false),
            id("formal_node_id", false),
            Field::new("formal_name", DataType::Utf8, false),
            Field::new("path", DataType::Utf8, false),
            Field::new("role", DataType::Int16, false),
            Field::new("consumer_start_byte", DataType::Int64, false),
            id("consumer_site_node_id", false),
            id("producer_site_node_id", false),
            Field::new("named", DataType::Boolean, false),
            Field::new("consumer_modality", DataType::Int16, false),
        ]))
    }

    pub fn guards() -> SchemaRef {
        Arc::new(Schema::new(vec![
            id("function_node_id", false),
            id("if_node_id", false),
            id("test_node_id", false),
            id("raise_node_id", false),
            id("parameter_node_id", false),
            id("module_node_id", false),
            Field::new("test_start_byte", DataType::Int64, false),
            Field::new("test_end_byte", DataType::Int64, false),
            Field::new("raise_start_byte", DataType::Int64, false),
            Field::new("raise_end_byte", DataType::Int64, false),
        ]))
    }
}
