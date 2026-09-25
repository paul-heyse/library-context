//! Conservative entry-formal to test-leaf identity proofs (ADR-0025).
//!
//! The first origin deliberately covers only a single direct reaching parameter in a complete
//! function scope with no intervening operation that could change the observed value. Prior
//! predicates are barriers too: comparison/truthiness may dispatch Python methods without a
//! `call_syntax` row. A type trace locates the tested operand; its type is not used to prove
//! identity or exact runtime type.

use std::collections::{BTreeMap, BTreeSet};

use cpg_schema::behavior::{FlowTestExactOriginsRow, FlowTestValueLinksRow};
use cpg_schema::codebook::{
    BindingKind, Codebook, CoverageStatus, ExactValueOrigin, FactFamily, SyntaxKind,
    TestValueLinkOrigin,
};
use cpg_schema::condition::Atom;
use cpg_schema::condition_kernel::{Diagram, DiagramNode};
use cpg_schema::id::{Id, IdHasher};
use cpg_schema::tables::{ConditionNodesRow, ConditionsRow};
use datafusion::prelude::SessionContext;

use crate::{CoreError, sql};

const EFFECT_RULE_REVISION: &[u8] = b"entry-type-guard-and-path-stability/v4";

cpg_schema::query_row! {
    struct TestUse {
        operation_node_id: Id,
        module_node_id: Id,
        leaf_fact_id: Id,
        atom_id: Id,
        atom: String,
        leaf_start_byte: i64,
        leaf_end_byte: i64,
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
        end_byte: i64,
        call_barrier: bool,
        positional_count: Option<i64>,
        keyword_count: Option<i64>,
    }
}

cpg_schema::query_row! {
    struct PriorTest {
        module_node_id: Id,
        operation_node_id: Id,
        leaf_fact_id: Id,
        end_byte: i64,
    }
}

cpg_schema::query_row! {
    struct ExactLeaf {
        fact_id: Id,
        atom_id: Id,
        atom: String,
        condition_id: Id,
        module_node_id: Id,
        leaf_start_byte: i64,
        leaf_end_byte: i64,
    }
}

cpg_schema::relations! {
    inventory relations;
    test_uses = "entry_links_test_uses",
        deps = ["flow_test_types", "flow_test_leaves", "flow_uses", "declarations", "public_paths", "coverage"],
        sql = format!(
            "SELECT DISTINCT d.node_id AS operation_node_id, u.module_node_id, \
                    t.leaf_fact_id, t.atom_id, l.atom, l.leaf_start_byte, l.leaf_end_byte, \
                    t.use_id, t.use_fact_id, \
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
            "SELECT module_node_id, operation_node_id, start_byte, end_byte, call_barrier, \
                    positional_count, keyword_count FROM ( \
               SELECT module_node_id, owner_node_id AS operation_node_id, start_byte, end_byte, \
                      true AS call_barrier, positional_count, keyword_count \
                 FROM call_syntax WHERE owner_node_id IS NOT NULL \
               UNION ALL \
               SELECT d.module_node_id, s.node_id AS operation_node_id, d.start_byte, d.end_byte, \
                      false AS call_barrier, NULL AS positional_count, NULL AS keyword_count \
                 FROM flow_definitions d JOIN declarations s \
                   ON s.module_node_id = d.module_node_id \
                  AND s.name_start_byte = d.scope_start_byte \
                  AND s.name_end_byte = d.scope_end_byte \
                 WHERE d.kind <> {parameter} \
               UNION ALL \
               SELECT n.module_node_id, n.owner_node_id AS operation_node_id, n.start_byte, n.end_byte, \
                      false AS call_barrier, NULL AS positional_count, NULL AS keyword_count \
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
                      l.fact_id AS leaf_fact_id, l.leaf_end_byte AS end_byte \
               FROM flow_test_leaves l JOIN declarations d \
                 ON d.module_node_id = l.module_node_id \
                AND d.name_start_byte = l.scope_start_byte \
                AND d.name_end_byte = l.scope_end_byte \
               ORDER BY 1, 2, 3".to_owned();
    exact_leaves = "entry_links_exact_leaves", deps = ["flow_test_leaves"],
        sql = "SELECT fact_id, atom_id, atom, condition_id, module_node_id, \
                      leaf_start_byte, leaf_end_byte FROM flow_test_leaves ORDER BY fact_id".to_owned();
    condition_roots = "entry_links_condition_roots", deps = ["conditions"],
        sql = "SELECT * FROM conditions".to_owned();
    condition_nodes = "entry_links_condition_nodes", deps = ["condition_nodes"],
        sql = "SELECT * FROM condition_nodes".to_owned();
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
    let mut blocked: BTreeMap<(Id, Id), Vec<Barrier>> = BTreeMap::new();
    for row in barriers {
        blocked
            .entry((row.module_node_id, row.operation_node_id))
            .or_default()
            .push(row);
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
        let resolved_type_operand = Atom::parse_encoded(&test.atom).ok().is_some_and(|atom| {
            let Atom::Evaluated { atom, .. } = atom else {
                return false;
            };
            matches!(*atom, Atom::TypeIs { place, ref class }
                if place == test.place && matches!(class.as_str(), "str" | "int" | "bool"))
        });
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
            || tested_before
                .get(&(test.module_node_id, function))
                .is_some_and(|ends| {
                    ends.iter()
                        .any(|end| *end > start && *end <= test.operand_start_byte)
                })
        {
            continue;
        }
        let mut exempted_type_call = false;
        let has_barrier = blocked
            .get(&(test.module_node_id, function))
            .is_some_and(|sites| {
                sites.iter().any(|barrier| {
                    if barrier.start_byte < start || barrier.start_byte >= test.operand_start_byte {
                        return false;
                    }
                    let own_resolved_type_call = resolved_type_operand
                        && barrier.call_barrier
                        && barrier.start_byte == test.leaf_start_byte
                        && barrier.end_byte > test.operand_end_byte
                        && barrier.end_byte <= test.leaf_end_byte
                        && barrier.positional_count == Some(1)
                        && barrier.keyword_count == Some(0);
                    if own_resolved_type_call {
                        exempted_type_call = true;
                    }
                    !own_resolved_type_call
                })
            });
        if has_barrier {
            continue;
        }
        // Keep the old direct origin intact. The new origin requires the exact call-syntax
        // witness, not merely the shape of the leaf atom.
        let origin = if exempted_type_call {
            TestValueLinkOrigin::ResolvedBuiltinTypeOperand
        } else {
            TestValueLinkOrigin::DirectParameterReachNoEffect
        };
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
            origin,
            effect_model_digest,
            stability_origin_id: None,
            stability_condition_id: None,
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

/// A positive exact-class fact conditional on the guard atom being true. This proof does not
/// transfer to a later use without another same-value witness.
pub async fn exact_origins(
    ctx: &SessionContext,
    snapshot_id: Id,
    links: &[FlowTestValueLinksRow],
) -> Result<Vec<FlowTestExactOriginsRow>, CoreError> {
    let leaves: Vec<ExactLeaf> = sql::fetch(ctx, &exact_leaves(), sql::Params::new()).await?;
    let by_id: BTreeMap<Id, ExactLeaf> = leaves
        .into_iter()
        .map(|leaf| (leaf.fact_id, leaf))
        .collect();
    let mut out = Vec::new();
    for link in links {
        if link.origin != TestValueLinkOrigin::ResolvedBuiltinTypeOperand {
            continue;
        }
        let Some(leaf) = by_id.get(&link.leaf_fact_id) else {
            continue;
        };
        if leaf.atom_id != link.atom_id || leaf.condition_id != link.condition_id {
            continue;
        }
        let Ok(Atom::Evaluated { atom, .. }) = Atom::parse_encoded(&leaf.atom) else {
            continue;
        };
        let Atom::TypeIs { place, class } = *atom else {
            continue;
        };
        if place != link.place || !matches!(class.as_str(), "str" | "int" | "bool") {
            continue;
        }
        out.push(FlowTestExactOriginsRow {
            snapshot_id,
            origin_id: IdHasher::new("flow-test-exact-origin")
                .id(link.link_id)
                .str(&class)
                .finish_id(),
            operation_node_id: link.operation_node_id,
            formal_node_id: link.formal_node_id,
            module_node_id: link.module_node_id,
            leaf_fact_id: link.leaf_fact_id,
            atom_id: link.atom_id,
            use_id: link.use_id,
            value_link_id: link.link_id,
            test_condition_id: link.condition_id,
            builtin_class: class,
            origin: ExactValueOrigin::ResolvedBuiltinTypeGuard,
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

/// A second positive link origin: a later test on the TRUE branch of a pure exact-type guard.
/// The current type guard must itself have a checked entry-value link. Source order alone is
/// never sufficient: the later condition must imply the guard atom, and every other earlier
/// call, binding, effect-bearing syntax or predicate remains a barrier.
pub async fn stable_after_exact_guards(
    ctx: &SessionContext,
    snapshot_id: Id,
    base_links: &[FlowTestValueLinksRow],
    origins: &[FlowTestExactOriginsRow],
) -> Result<Vec<FlowTestValueLinksRow>, CoreError> {
    if origins.is_empty() {
        return Ok(Vec::new());
    }
    let tests: Vec<TestUse> = sql::fetch(ctx, &test_uses(), sql::Params::new()).await?;
    let reaches: Vec<Reach> = sql::fetch(ctx, &reaches(), sql::Params::new()).await?;
    let barriers: Vec<Barrier> = sql::fetch(ctx, &barriers(), sql::Params::new()).await?;
    let prior_tests: Vec<PriorTest> = sql::fetch(ctx, &prior_tests(), sql::Params::new()).await?;
    let leaves: Vec<ExactLeaf> = sql::fetch(ctx, &exact_leaves(), sql::Params::new()).await?;
    let roots: Vec<ConditionsRow> = sql::fetch(ctx, &condition_roots(), sql::Params::new()).await?;
    let nodes: Vec<ConditionNodesRow> =
        sql::fetch(ctx, &condition_nodes(), sql::Params::new()).await?;
    let leaves: BTreeMap<Id, ExactLeaf> =
        leaves.into_iter().map(|row| (row.fact_id, row)).collect();
    let roots: BTreeMap<Id, Id> = roots
        .into_iter()
        .filter_map(|row| row.root_id.map(|root| (row.condition_id, root)))
        .collect();
    let nodes: std::collections::HashMap<Id, DiagramNode> = nodes
        .into_iter()
        .map(|row| {
            (
                row.node_id,
                DiagramNode {
                    node_id: row.node_id,
                    atom: row.atom,
                    low: row.low_id,
                    high: row.high_id,
                },
            )
        })
        .collect();
    let links: BTreeMap<Id, &FlowTestValueLinksRow> =
        base_links.iter().map(|link| (link.link_id, link)).collect();
    let mut guards: BTreeMap<(Id, Id), Vec<(&FlowTestExactOriginsRow, &ExactLeaf, Diagram)>> =
        BTreeMap::new();
    for origin in origins {
        let (Some(leaf), Some(link)) = (
            leaves.get(&origin.leaf_fact_id),
            links.get(&origin.value_link_id),
        ) else {
            continue;
        };
        if origin.module_node_id != leaf.module_node_id
            || origin.atom_id != leaf.atom_id
            || origin.test_condition_id != leaf.condition_id
            || link.origin != TestValueLinkOrigin::ResolvedBuiltinTypeOperand
        {
            continue;
        }
        let Ok(atom @ Atom::Evaluated { .. }) = Atom::parse_encoded(&leaf.atom) else {
            continue;
        };
        let Ok(diagram) = Diagram::from_atom(&atom) else {
            continue;
        };
        // A compound guard could evaluate another Python predicate with side effects.
        if diagram.id() != origin.test_condition_id {
            continue;
        }
        guards
            .entry((origin.operation_node_id, origin.formal_node_id))
            .or_default()
            .push((origin, leaf, diagram));
    }
    let mut by_use: BTreeMap<Id, Vec<Reach>> = BTreeMap::new();
    for row in reaches {
        by_use.entry(row.use_id).or_default().push(row);
    }
    let mut blocked: BTreeMap<(Id, Id), Vec<Barrier>> = BTreeMap::new();
    for row in barriers {
        blocked
            .entry((row.module_node_id, row.operation_node_id))
            .or_default()
            .push(row);
    }
    let mut prior: BTreeMap<(Id, Id), Vec<PriorTest>> = BTreeMap::new();
    for row in prior_tests {
        prior
            .entry((row.module_node_id, row.operation_node_id))
            .or_default()
            .push(row);
    }
    let mut diagrams: BTreeMap<Id, Diagram> = BTreeMap::new();
    let mut out = Vec::new();
    let effect_model_digest = digest();
    for test in tests {
        let Ok(atom @ Atom::Evaluated { .. }) = Atom::parse_encoded(&test.atom) else {
            continue;
        };
        let Atom::Evaluated { atom: inner, .. } = &atom else {
            unreachable!()
        };
        if !matches!(
            inner.as_ref(),
            Atom::IsNone { .. }
                | Atom::IsValue { .. }
                | Atom::Truthy { .. }
                | Atom::Equals {
                    value: cpg_schema::condition::Value::Str(_),
                    ..
                }
        ) {
            continue;
        }
        let Some([reach]) = by_use.get(&test.use_id).map(Vec::as_slice) else {
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
        {
            continue;
        }
        let Some(candidates) = guards.get(&(function, formal)) else {
            continue;
        };
        let Some(&root) = roots.get(&reach.condition_id) else {
            continue;
        };
        let condition = if let Some(found) = diagrams.get(&reach.condition_id) {
            found.clone()
        } else {
            let Ok(found) = Diagram::from_catalog(root, &nodes) else {
                continue;
            };
            diagrams.insert(reach.condition_id, found.clone());
            found
        };
        for (guard, guard_leaf, guard_atom) in candidates {
            if guard.module_node_id != test.module_node_id
                || guard_leaf.leaf_end_byte >= test.operand_start_byte
                || start >= guard_leaf.leaf_start_byte
                || condition.implies(guard_atom) != Ok(true)
            {
                continue;
            }
            let has_barrier = blocked
                .get(&(test.module_node_id, function))
                .is_some_and(|sites| {
                    sites.iter().any(|barrier| {
                        if barrier.start_byte < start
                            || barrier.start_byte >= test.operand_start_byte
                        {
                            return false;
                        }
                        !(barrier.call_barrier
                            && barrier.start_byte == guard_leaf.leaf_start_byte
                            && barrier.end_byte > guard_leaf.leaf_start_byte
                            && barrier.end_byte <= guard_leaf.leaf_end_byte
                            && barrier.positional_count == Some(1)
                            && barrier.keyword_count == Some(0))
                    })
                });
            let has_other_test = prior
                .get(&(test.module_node_id, function))
                .is_some_and(|rows| {
                    rows.iter().any(|row| {
                        row.end_byte > start
                            && row.end_byte <= test.operand_start_byte
                            && row.leaf_fact_id != guard.leaf_fact_id
                    })
                });
            if has_barrier || has_other_test {
                continue;
            }
            out.push(FlowTestValueLinksRow {
                snapshot_id,
                link_id: IdHasher::new("flow-test-value-link")
                    .id(function)
                    .id(formal)
                    .id(test.leaf_fact_id)
                    .id(test.use_id)
                    .finish_id(),
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
                place: test.place.clone(),
                condition_id: test.condition_id,
                origin: TestValueLinkOrigin::StableAfterExactTypeGuard,
                effect_model_digest,
                stability_origin_id: Some(guard.origin_id),
                stability_condition_id: Some(reach.condition_id),
            });
            break;
        }
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

/// Build the complete proof pair in dependency order, once per attempt or validation.
pub async fn all(
    ctx: &SessionContext,
    snapshot_id: Id,
) -> Result<(Vec<FlowTestValueLinksRow>, Vec<FlowTestExactOriginsRow>), CoreError> {
    let mut links = run(ctx, snapshot_id).await?;
    let origins = exact_origins(ctx, snapshot_id, &links).await?;
    let stable = stable_after_exact_guards(ctx, snapshot_id, &links, &origins).await?;
    links.extend(stable);
    links.sort_by_key(|row| {
        (
            row.operation_node_id,
            row.formal_node_id,
            row.leaf_fact_id,
            row.use_id,
        )
    });
    Ok((links, origins))
}
