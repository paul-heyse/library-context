//! Pass B's declared relations (DESIGN §9.2): the argument flows and the parameter guards it
//! reads, as SQL over the CPG (C2 syntax, C3 bindings, the invocation arcs' call targets), stated
//! before any worklist runs. Their digest joins the compiler digest.
//!
//! **Argument flows.** One row per argument of a call arc (phase `call` or `init`) whose argument
//! maps to one formal of the target, with how its value arises:
//! - `parameter`: a name bound, once in its scope, by a parameter of the caller;
//! - `alias`: a name bound once, by an assignment directly in the caller's body, from such a
//!   parameter (one identity alias in straight-line code);
//! - `literal`: a string, number, boolean or `None` literal, as written;
//! - `other`: anything else, never followed.
//!
//! The mapping is per candidate target, the implicit receiver counted: a positional argument
//! before any `*` argument maps to the positional formal at its index (plus one for a bound
//! receiver), a keyword argument to the positional-or-keyword or keyword-only formal of that
//! name. A starred argument, one after it, a `**` argument, and one no formal takes (a `*args` or
//! `**kwargs` catch-all) are never mapped. Each row says whether its call site lies inside a
//! `try` of the caller, where a handler may catch what the callee raises.
//!
//! **Guards.** One row per `if` directly in a function's body that tests one of its parameters
//! (bound once) and holds a `raise` directly in its body, the test reading only that function's
//! parameters and builtins. A guard on a rebound parameter is no guard of its argument.

use crate::codebook::{
    ArgumentKind, BindingKind, Codebook, EdgeKind, Fidelity, ImplicitReceiver, InvocationPhase,
    Modality, NodeKind, Origin, ParameterKind, SourceRole, SyntaxField, SyntaxKind,
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

/// The argument-flows query, totally ordered by `(caller, call site, edge, argument)`.
pub fn argument_flows_sql() -> String {
    let accepted = format!(
        "f.modality IN ({}) AND f.origin = {} AND f.fidelity = {}",
        list(&[Modality::Definite.code(), Modality::Candidate.code()]),
        Origin::AnalyzerAssertion.code(),
        Fidelity::ReportProjection.code()
    );
    let receivers = list(&[
        ImplicitReceiver::TrueWithClassReceiver.code(),
        ImplicitReceiver::TrueWithObjectReceiver.code(),
    ]);
    let literals = list(&[
        SyntaxKind::ExprStringLiteral.code(),
        SyntaxKind::ExprNumberLiteral.code(),
        SyntaxKind::ExprBooleanLiteral.code(),
        SyntaxKind::ExprNoneLiteral.code(),
    ]);
    format!(
        "WITH {single}, \
         arcs AS ( \
           SELECT ec.src_node_id AS caller_node_id, ct.src_node_id AS call_site_node_id, \
                  ct.dst_node_id AS target_node_id, ct.edge_id, f.modality, p.phase, \
                  CASE WHEN p.implicit_receiver IN ({receivers}) THEN 1 ELSE 0 END AS receiver \
           FROM edges ct \
           JOIN edges ec ON ec.dst_node_id = ct.src_node_id AND ec.edge_kind = {encloses} \
           JOIN pysa_calls p ON p.fact_id = ct.evidence_fact_id \
           JOIN facts f ON f.fact_id = ct.evidence_fact_id \
           WHERE ct.edge_kind = {call_target} AND ct.dst_kind = {function} AND {accepted} \
             AND p.phase IN ({call}, {init})), \
         starred AS ( \
           SELECT call_node_id, min(ordinal) AS first FROM arguments \
           WHERE kind = {starred_kind} GROUP BY call_node_id), \
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
           SELECT b.node_id AS binding_id, p.parameter_node_id, b.name AS alias_name \
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
         tries AS ( \
           SELECT owner_node_id, module_node_id, start_byte, end_byte FROM syntax_nodes \
           WHERE kind = {try_}), \
         in_try AS ( \
           SELECT DISTINCT cs.node_id FROM call_syntax cs JOIN tries t \
             ON t.module_node_id = cs.module_node_id AND t.owner_node_id = cs.owner_node_id \
            AND t.start_byte <= cs.start_byte AND cs.end_byte <= t.end_byte), \
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
                  al.alias_name, \
                  CASE WHEN r.value_kind IN ({literals}) THEN r.value_detail END AS value_text, \
                  it.node_id IS NOT NULL AS in_try \
           FROM arcs a \
           JOIN resolved r ON r.call_node_id = a.call_site_node_id \
           LEFT JOIN starred sr ON sr.call_node_id = a.call_site_node_id \
           JOIN parameter_syntax fm ON fm.function_node_id = a.target_node_id \
             AND ((r.kind = {positional} AND (sr.first IS NULL OR r.ordinal < sr.first) \
                   AND fm.kind IN ({posonly}, {pos_or_kw}) \
                   AND fm.ordinal = r.ordinal + a.receiver) \
               OR (r.kind = {keyword} AND fm.name = r.keyword \
                   AND fm.kind IN ({pos_or_kw}, {kw_only}))) \
           LEFT JOIN params pr ON pr.binding_id = r.binding_id \
           LEFT JOIN aliases al ON al.binding_id = r.binding_id \
           LEFT JOIN in_try it ON it.node_id = a.call_site_node_id) \
         SELECT DISTINCT * FROM mapped \
         ORDER BY caller_node_id, call_site_node_id, edge_id, argument_node_id, formal_node_id",
        single = single_bindings(),
        encloses = EdgeKind::EnclosesCall.code(),
        call_target = EdgeKind::CallTarget.code(),
        argument_value = EdgeKind::ArgumentValue.code(),
        function = NodeKind::Function.code(),
        call = InvocationPhase::Call.code(),
        init = InvocationPhase::Init.code(),
        starred_kind = ArgumentKind::Starred.code(),
        positional = ArgumentKind::Positional.code(),
        keyword = ArgumentKind::Keyword.code(),
        posonly = ParameterKind::PositionalOnly.code(),
        pos_or_kw = ParameterKind::PositionalOrKeyword.code(),
        kw_only = ParameterKind::KeywordOnly.code(),
        assign = SyntaxKind::StmtAssign.code(),
        name = SyntaxKind::ExprName.code(),
        assignment = BindingKind::Assignment.code(),
        try_ = SyntaxKind::StmtTry.code(),
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
         names AS ( \
           SELECT ifs.if_node_id, n.node_id AS name_node_id, p.parameter_node_id, \
                  rr.builtin_name \
           FROM ifs JOIN tests ON tests.if_node_id = ifs.if_node_id \
           JOIN syntax_nodes n ON n.module_node_id = tests.module_node_id \
                AND n.owner_node_id = ifs.function_node_id AND n.kind = {name} \
                AND n.start_byte >= tests.test_start_byte AND n.end_byte <= tests.test_end_byte \
           LEFT JOIN references rf ON rf.name_node_id = n.node_id \
           LEFT JOIN reference_resolutions rr ON rr.reference_id = rf.node_id \
           LEFT JOIN params p ON p.binding_id = rr.binding_id AND NOT rr.captured), \
         supported AS ( \
           SELECT if_node_id FROM names GROUP BY if_node_id \
           HAVING count(*) = count(parameter_node_id) + count(builtin_name) \
              AND count(parameter_node_id) > 0) \
         SELECT DISTINCT ifs.function_node_id, ifs.if_node_id, tests.test_node_id, \
                raises.raise_node_id, names.parameter_node_id, ifs.module_node_id, \
                tests.test_start_byte, tests.test_end_byte, raises.raise_start_byte, \
                raises.raise_end_byte \
         FROM ifs JOIN supported s ON s.if_node_id = ifs.if_node_id \
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
    )
}

/// Pass C's handoffs query (DESIGN §9.3), over the official usage code (examples, tests, doc
/// blocks), totally ordered by `(consumer, producer, formal, module path, consumer site)`.
///
/// One row per occurrence in which what a release callable returns reaches an argument of
/// another: either `x = producer(...)` with `x` bound once in its scope and read once, by
/// `consumer(..., x)` in a later statement of the same block (the call being that statement's
/// value, awaited or not: no `with` item, no nesting); or `consumer(..., producer(...))`. The
/// argument maps to one formal of the consumer as a flow's does. A read of `x` as a receiver
/// (`x.method()`, `@x.tool`) configures it: setup, not another consumer. A reassigned `x`, one
/// read anywhere else too (another argument, a return, a store), and a use inside a `with` item
/// are no handoff.
pub fn handoffs_sql() -> String {
    let accepted = format!(
        "f.modality IN ({}) AND f.origin = {} AND f.fidelity = {}",
        list(&[Modality::Definite.code(), Modality::Candidate.code()]),
        Origin::AnalyzerAssertion.code(),
        Fidelity::ReportProjection.code()
    );
    let receivers = list(&[
        ImplicitReceiver::TrueWithClassReceiver.code(),
        ImplicitReceiver::TrueWithObjectReceiver.code(),
    ]);
    let usage = list(&[
        SourceRole::Example.code(),
        SourceRole::Test.code(),
        SourceRole::DocBlock.code(),
    ]);
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
           SELECT ct.src_node_id AS site, ct.dst_node_id AS target, ct.edge_id, f.modality, \
                  CASE WHEN p.implicit_receiver IN ({receivers}) THEN 1 ELSE 0 END AS receiver \
           FROM edges ct JOIN pysa_calls p ON p.fact_id = ct.evidence_fact_id \
           JOIN facts f ON f.fact_id = ct.evidence_fact_id \
           WHERE ct.edge_kind = {call_target} AND ct.dst_kind = {function} AND {accepted} \
             AND p.phase IN ({call}, {init})), \
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
         JOIN parameter_syntax fm ON fm.function_node_id = ct.target \
           AND ((o.kind = {positional} AND fm.kind IN ({posonly}, {pos_or_kw}) \
                 AND fm.ordinal = o.ordinal + ct.receiver) \
             OR (o.kind = {keyword} AND fm.name = o.keyword \
                 AND fm.kind IN ({pos_or_kw}, {kw_only}))) \
         ORDER BY consumer_node_id, producer_node_id, formal_node_id, u.path, \
                  consumer_start_byte, consumer_site_node_id, producer_site_node_id",
        call_target = EdgeKind::CallTarget.code(),
        argument_value = EdgeKind::ArgumentValue.code(),
        function = NodeKind::Function.code(),
        call = InvocationPhase::Call.code(),
        init = InvocationPhase::Init.code(),
        assign = SyntaxKind::StmtAssign.code(),
        expr_stmt = SyntaxKind::StmtExpr.code(),
        await_ = SyntaxKind::ExprAwait.code(),
        assignment = BindingKind::Assignment.code(),
        positional = ArgumentKind::Positional.code(),
        keyword = ArgumentKind::Keyword.code(),
        posonly = ParameterKind::PositionalOnly.code(),
        pos_or_kw = ParameterKind::PositionalOrKeyword.code(),
        kw_only = ParameterKind::KeywordOnly.code(),
        attribute = SyntaxKind::ExprAttribute.code(),
        value_field = SyntaxField::Value.code(),
    )
}

/// The relations' identity, for the compiler digest and each Pass B or C invocation.
pub fn digest() -> Digest {
    IdHasher::new("pass-b-relations")
        .str(&argument_flows_sql())
        .str(&guards_sql())
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
            Field::new("in_try", DataType::Boolean, false),
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
