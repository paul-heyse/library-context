//! The declared-dependency audit (cutover plan WP0.7): every legacy SQL relation's logical plan
//! scans exactly the tables its declaration names, so a stage table built from declarations never
//! misses a read. The pinned report is the known discrepancy list: the legacy stage table (WP1.2)
//! declares the scanned inputs, and a new discrepancy is a reviewed snapshot change. Derived tables
//! declare no inputs; they must read only raw and earlier derived tables.

use std::collections::BTreeSet;

use cpg_schema::query::Relation;
use cpg_schema::table::Table;
use datafusion::common::tree_node::TreeNodeRecursion;
use datafusion::logical_expr::LogicalPlan;
use datafusion::prelude::SessionContext;

fn session() -> SessionContext {
    let ctx = cpg_core::session::session();
    macro_rules! register {
        ($($t:ty),+) => {$(
            ctx.register_batch(<$t as Table>::NAME, <$t as Table>::to_batch(&[]).unwrap()).unwrap();
        )+};
    }
    cpg_schema::for_each_table!(register);
    cpg_schema::for_each_derived_table!(register);
    cpg_schema::for_each_analysis_table!(register);
    register!(cpg_schema::tables::Snapshots);
    ctx
}

/// The registered tables a plan scans. A recursive CTE's work table is not a table.
fn scans(ctx: &SessionContext, plan: &LogicalPlan) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    plan.apply_with_subqueries(|node| {
        if let LogicalPlan::TableScan(scan) = node {
            let name = scan.table_name.table();
            if ctx.table_exist(name).unwrap_or(false) {
                out.insert(name.to_owned());
            }
        }
        Ok(TreeNodeRecursion::Continue)
    })
    .unwrap();
    out
}

fn relations() -> Vec<(&'static str, Relation)> {
    let mut out = Vec::new();
    let mut add = |module: &'static str, relations: Vec<Relation>| {
        out.extend(relations.into_iter().map(|r| (module, r)));
    };
    add("cpg_schema::public", cpg_schema::public::all());
    add(
        "cpg_schema::public::candidates",
        cpg_schema::public::candidate_relations(),
    );
    add(
        "cpg_schema::public::members",
        cpg_schema::public::member_relations(),
    );
    add("cpg_schema::behavior", cpg_schema::behavior::all());
    add(
        "cpg_schema::behavior::boundaries",
        cpg_schema::behavior::boundaries(),
    );
    add(
        "cpg_schema::behavior::open_reads",
        cpg_schema::behavior::open_reads(),
    );
    add("cpg_core::catalog", cpg_core::catalog::relations());
    add("cpg_core::entry_links", cpg_core::entry_links::relations());
    add("cpg_core::behavior", cpg_core::behavior::relations());
    add("cpg_core::usage", cpg_core::usage::relations());
    add("cpg_core::summaries", cpg_core::summaries::relations());
    add("cpg_core::synth", cpg_core::synth::relations());
    add("cpg_core::flow_model", cpg_core::flow_model::relations());
    add("cpg_core::analyze", cpg_core::analyze::relations());
    out
}

#[tokio::test]
async fn every_relation_declares_exactly_what_it_scans() {
    let ctx = session();
    let mut report = Vec::new();
    for (module, relation) in relations() {
        let plan = ctx
            .sql(&relation.sql)
            .await
            .map(|df| df.logical_plan().clone());
        let plan = match plan {
            Ok(plan) => plan,
            Err(e) => {
                report.push(format!("{module}::{}: does not plan: {e}", relation.name));
                continue;
            }
        };
        let scanned = scans(&ctx, &plan);
        let declared: BTreeSet<String> = relation.deps.iter().map(|d| (*d).to_owned()).collect();
        let undeclared: Vec<_> = scanned.difference(&declared).cloned().collect();
        let unused: Vec<_> = declared.difference(&scanned).cloned().collect();
        if !undeclared.is_empty() || !unused.is_empty() {
            report.push(format!(
                "{module}::{}: undeclared {undeclared:?}, declared but unscanned {unused:?}",
                relation.name
            ));
        }
    }
    insta::assert_debug_snapshot!(report);
}

#[tokio::test]
async fn derived_tables_scan_only_raw_and_earlier_derived_tables() {
    let ctx = session();
    let raw: BTreeSet<&str> = {
        macro_rules! names { ($($t:ty),+) => { [$(<$t as Table>::NAME),+].into_iter().collect() }; }
        cpg_schema::for_each_table!(names)
    };
    let mut earlier: BTreeSet<String> = BTreeSet::new();
    let mut report = Vec::new();
    for (name, sql) in cpg_schema::derived::derivations() {
        let plan = ctx.sql(&sql).await.unwrap().logical_plan().clone();
        let scanned = scans(&ctx, &plan);
        let late: Vec<_> = scanned
            .iter()
            .filter(|t| !raw.contains(t.as_str()) && !earlier.contains(*t))
            .cloned()
            .collect();
        report.push(format!(
            "{name}: {} scans{}",
            scanned.len(),
            if late.is_empty() {
                String::new()
            } else {
                format!(", reads before written: {late:?}")
            }
        ));
        earlier.insert(name.to_owned());
    }
    insta::assert_debug_snapshot!(report);
}
