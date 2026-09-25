//! Conservative entry-formal to test-leaf identity proofs (ADR-0025).
//!
//! The first origin deliberately covers only a single direct reaching parameter in a complete
//! function scope with no intervening operation that could change the observed value. Prior
//! predicates are barriers too: comparison/truthiness may dispatch Python methods without a
//! `call_syntax` row. A type trace locates the tested operand; its type is not used to prove
//! identity or exact runtime type.

use std::collections::{BTreeMap, BTreeSet};

use cpg_schema::behavior::FlowTestValueLinksRow;
use cpg_schema::codebook::{
    BindingKind, Codebook, CoverageStatus, FactFamily, SyntaxKind, TestValueLinkOrigin,
};
use cpg_schema::id::{Id, IdHasher};
use datafusion::prelude::SessionContext;

use crate::{CoreError, sql};

const EFFECT_RULE_REVISION: &[u8] = b"direct-entry-no-effect/v2";

cpg_schema::query_row! {
    struct TestUse {
        operation_node_id: Id,
        module_node_id: Id,
        leaf_fact_id: Id,
        atom_id: Id,
        use_id: Id,
        use_fact_id: Id,
        operand_start_byte: i64,
        operand_end_byte: i64,
        place: String,
        condition_id: Id,
    }
}

cpg_schema::query_row! {
    struct Reach {
        use_id: Id,
        reaching_fact_id: Id,
        definition_fact_id: Option<Id>,
        formal_node_id: Option<Id>,
        function_node_id: Option<Id>,
        definition_end_byte: Option<i64>,
        condition_id: Id,
        approximated: bool,
        loop_carried: bool,
        stated: bool,
    }
}

cpg_schema::query_row! {
struct Barrier {
        module_node_id: Id,
        operation_node_id: Id,
        start_byte: i64,
    }
}

cpg_schema::query_row! {
    struct PriorTest {
        module_node_id: Id,
        operation_node_id: Id,
        end_byte: i64,
    }
}

cpg_schema::relations! {
    inventory relations;
    test_uses = "entry_links_test_uses",
        deps = ["flow_test_types", "flow_test_leaves", "flow_uses", "declarations", "public_paths", "coverage"],
        sql = format!(
            "SELECT DISTINCT d.node_id AS operation_node_id, u.module_node_id, \
                    t.leaf_fact_id, t.atom_id, t.use_id, t.use_fact_id, \
                    t.operand_start_byte, t.operand_end_byte, t.place, l.condition_id \
             FROM flow_test_types t \
             JOIN flow_test_leaves l ON l.fact_id = t.leaf_fact_id \
             JOIN flow_uses u ON u.use_id = t.use_id AND u.fact_id = t.use_fact_id \
             JOIN declarations d ON d.module_node_id = u.module_node_id \
               AND d.name_start_byte = u.scope_start_byte \
               AND d.name_end_byte = u.scope_end_byte \
             JOIN public_paths p ON p.node_id = d.node_id \
             JOIN coverage c ON c.scope_node_id = u.module_node_id \
               AND c.fact_family = {flow} AND c.status = {complete} \
             WHERE NOT u.annotation ORDER BY 1, 2, 3, 5",
            flow = FactFamily::Flow.code(),
            complete = CoverageStatus::CompleteUnderStatedModel.code(),
        );
    reaches = "entry_links_reaches",
        deps = ["flow_reaching", "flow_definitions", "bindings", "parameter_syntax", "conditions"],
        sql = format!(
            "SELECT r.use_id, r.fact_id AS reaching_fact_id, \
                    d.fact_id AS definition_fact_id, b.site_node_id AS formal_node_id, \
                    ps.function_node_id, d.end_byte AS definition_end_byte, \
                    r.condition_id, r.approximated, r.loop_carried, \
                    (c.root_id IS NOT NULL AND c.boundary_reason IS NULL) AS stated \
             FROM flow_reaching r \
             LEFT JOIN flow_definitions d ON d.definition_id = r.definition_id \
             LEFT JOIN bindings b ON b.module_node_id = d.module_node_id \
               AND b.start_byte = d.start_byte AND b.end_byte = d.end_byte \
               AND b.kind = {parameter} AND d.kind = {parameter} \
             LEFT JOIN parameter_syntax ps ON ps.node_id = b.site_node_id \
             JOIN conditions c ON c.condition_id = r.condition_id \
             ORDER BY r.use_id, r.fact_id",
            parameter = BindingKind::Parameter.code(),
        );
    barriers = "entry_links_barriers",
        deps = ["call_syntax", "flow_definitions", "declarations", "syntax_nodes"],
        sql = format!(
            "SELECT module_node_id, operation_node_id, start_byte FROM ( \
               SELECT module_node_id, owner_node_id AS operation_node_id, start_byte \
                 FROM call_syntax WHERE owner_node_id IS NOT NULL \
               UNION ALL \
               SELECT d.module_node_id, s.node_id AS operation_node_id, d.start_byte \
                 FROM flow_definitions d JOIN declarations s \
                   ON s.module_node_id = d.module_node_id \
                  AND s.name_start_byte = d.scope_start_byte \
                  AND s.name_end_byte = d.scope_end_byte \
                 WHERE d.kind <> {parameter} \
               UNION ALL \
               SELECT n.module_node_id, n.owner_node_id AS operation_node_id, n.start_byte \
                 FROM syntax_nodes n LEFT JOIN declarations owner \
                   ON owner.node_id = n.owner_node_id \
                  AND owner.module_node_id = n.module_node_id \
                WHERE n.owner_node_id IS NOT NULL \
                  AND n.kind IN ({barrier_kinds}) \
                  AND (n.kind <> {stmt_expr} OR owner.docstring_start_byte IS NULL \
                       OR n.start_byte <> owner.docstring_start_byte \
                       OR n.end_byte <> owner.docstring_end_byte) \
             ) ORDER BY 1, 2, 3",
            parameter = BindingKind::Parameter.code(),
            stmt_expr = SyntaxKind::StmtExpr.code(),
            barrier_kinds = [
                SyntaxKind::StmtFunctionDef, SyntaxKind::StmtClassDef, SyntaxKind::StmtReturn,
                SyntaxKind::StmtWith, SyntaxKind::StmtTry, SyntaxKind::StmtFor,
                SyntaxKind::StmtWhile, SyntaxKind::StmtMatch, SyntaxKind::StmtExpr,
                SyntaxKind::StmtGlobal, SyntaxKind::StmtNonlocal, SyntaxKind::ExprAwait,
                SyntaxKind::ExprYield, SyntaxKind::ExprYieldFrom,
            ].iter().map(|k| k.code().to_string()).collect::<Vec<_>>().join(", "),
        );
    prior_tests = "entry_links_prior_tests",
        deps = ["flow_test_leaves", "declarations"],
        sql = "SELECT DISTINCT l.module_node_id, d.node_id AS operation_node_id, \
                      l.leaf_end_byte AS end_byte \
               FROM flow_test_leaves l JOIN declarations d \
                 ON d.module_node_id = l.module_node_id \
                AND d.name_start_byte = l.scope_start_byte \
                AND d.name_end_byte = l.scope_end_byte \
               ORDER BY 1, 2, 3".to_owned();
}

pub fn digest() -> cpg_schema::id::Digest {
    let mut hasher = IdHasher::new("entry-links");
    hasher.bytes(EFFECT_RULE_REVISION);
    for relation in relations() {
        hasher.str(relation.name).str(&relation.sql);
    }
    hasher.finish_digest()
}

/// A missing link means unknown, never that the entry value and test value differ.
pub async fn run(
    ctx: &SessionContext,
    snapshot_id: Id,
) -> Result<Vec<FlowTestValueLinksRow>, CoreError> {
    let tests: Vec<TestUse> = sql::fetch(ctx, &test_uses(), sql::Params::new()).await?;
    if tests.is_empty() {
        return Ok(Vec::new());
    }
    let reaches: Vec<Reach> = sql::fetch(ctx, &reaches(), sql::Params::new()).await?;
    let barriers: Vec<Barrier> = sql::fetch(ctx, &barriers(), sql::Params::new()).await?;
    let prior_tests: Vec<PriorTest> = sql::fetch(ctx, &prior_tests(), sql::Params::new()).await?;
    let mut by_use: BTreeMap<Id, Vec<Reach>> = BTreeMap::new();
    for row in reaches {
        by_use.entry(row.use_id).or_default().push(row);
    }
    let mut blocked: BTreeMap<(Id, Id), BTreeSet<i64>> = BTreeMap::new();
    for row in barriers {
        blocked
            .entry((row.module_node_id, row.operation_node_id))
            .or_default()
            .insert(row.start_byte);
    }
    let mut tested_before: BTreeMap<(Id, Id), BTreeSet<i64>> = BTreeMap::new();
    for row in prior_tests {
        tested_before
            .entry((row.module_node_id, row.operation_node_id))
            .or_default()
            .insert(row.end_byte);
    }
    let effect_model_digest = digest();
    let mut out = Vec::new();
    for test in tests {
        let Some(reaches) = by_use.get(&test.use_id) else {
            continue;
        };
        let [reach] = reaches.as_slice() else {
            continue;
        };
        let (Some(formal), Some(function), Some(definition), Some(start)) = (
            reach.formal_node_id,
            reach.function_node_id,
            reach.definition_fact_id,
            reach.definition_end_byte,
        ) else {
            continue;
        };
        if function != test.operation_node_id
            || reach.approximated
            || reach.loop_carried
            || !reach.stated
            || start >= test.operand_start_byte
            || blocked
                .get(&(test.module_node_id, function))
                .is_some_and(|sites| sites.range(start..test.operand_start_byte).next().is_some())
            || tested_before
                .get(&(test.module_node_id, function))
                .is_some_and(|ends| {
                    ends.iter()
                        .any(|end| *end > start && *end <= test.operand_start_byte)
                })
        {
            continue;
        }
        let link_id = IdHasher::new("flow-test-value-link")
            .id(test.operation_node_id)
            .id(formal)
            .id(test.leaf_fact_id)
            .id(test.use_id)
            .finish_id();
        out.push(FlowTestValueLinksRow {
            snapshot_id,
            link_id,
            operation_node_id: function,
            formal_node_id: formal,
            module_node_id: test.module_node_id,
            leaf_fact_id: test.leaf_fact_id,
            atom_id: test.atom_id,
            use_id: test.use_id,
            use_fact_id: test.use_fact_id,
            reaching_fact_id: reach.reaching_fact_id,
            definition_fact_id: definition,
            operand_start_byte: test.operand_start_byte,
            operand_end_byte: test.operand_end_byte,
            place: test.place,
            condition_id: test.condition_id,
            origin: TestValueLinkOrigin::DirectParameterReachNoEffect,
            effect_model_digest,
        });
    }
    out.sort_by_key(|row| {
        (
            row.operation_node_id,
            row.formal_node_id,
            row.leaf_fact_id,
            row.use_id,
        )
    });
    out.dedup_by_key(|row| {
        (
            row.operation_node_id,
            row.formal_node_id,
            row.leaf_fact_id,
            row.use_id,
        )
    });
    Ok(out)
}
