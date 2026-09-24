//! Read-only pilot survey of Stage 2 condition rows through the pinned snapshot reader.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use cpg_core::{snapshot, sql};
use cpg_schema::condition::Condition;
use cpg_schema::condition_kernel::{Diagram, validate_nodes};
use cpg_schema::id::Id;
use cpg_schema::query::Relation;

cpg_schema::query_row! {
    struct ConditionRow {
        condition_id: Id,
        encoding: String,
        stated: bool,
    }
}

fn percentile(sorted: &[usize], numerator: usize, denominator: usize) -> usize {
    if sorted.is_empty() {
        return 0;
    }
    let index = (sorted.len() - 1) * numerator / denominator;
    sorted[index]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let store = args
        .next()
        .expect("usage: bdd-pilot-survey STORE SNAPSHOT_HEX");
    let snapshot_id = Id::from_hex(&args.next().expect("snapshot hex required"))
        .expect("snapshot must be 32 hex digits");
    assert!(args.next().is_none(), "unexpected argument");
    let (_, ctx) = snapshot::published(Path::new(&store), snapshot_id)
        .await?
        .expect("snapshot is not published");
    let relation = Relation {
        name: "bdd_pilot_survey",
        sql: "SELECT condition_id, encoding, stated FROM conditions ORDER BY condition_id"
            .to_owned(),
        deps: &["conditions"],
    };
    let rows: Vec<ConditionRow> = sql::fetch(&ctx, &relation, sql::Params::new()).await?;
    let start = Instant::now();
    let mut failures: BTreeMap<String, usize> = BTreeMap::new();
    let mut stated = 0usize;
    let mut checked = 0usize;
    let mut unique_roots = BTreeSet::new();
    let mut unique_nodes = BTreeSet::new();
    let mut node_counts = Vec::new();
    let mut total_nodes = 0usize;
    for row in &rows {
        let parsed = match Condition::parse(&row.encoding) {
            Ok(value) => value,
            Err(_) => {
                *failures.entry("parse_error".to_owned()).or_default() += 1;
                continue;
            }
        };
        assert_eq!(row.stated, parsed != Condition::OverBudget);
        assert_eq!(row.condition_id, parsed.id());
        if row.stated {
            stated += 1;
        }
        match Diagram::from_condition(&parsed) {
            Ok(diagram) => {
                let (root, nodes) = diagram.root_and_nodes();
                validate_nodes(root, &nodes).expect("kernel-produced nodes must validate");
                unique_roots.insert(root);
                unique_nodes.extend(nodes.iter().map(|node| node.node_id));
                node_counts.push(nodes.len());
                total_nodes += nodes.len();
                checked += 1;
            }
            Err(reason) => *failures.entry(format!("{reason:?}")).or_default() += 1,
        }
    }
    node_counts.sort_unstable();
    println!("snapshot={}", snapshot_id.hex());
    println!(
        "condition_rows={} stated={} bdd_converted={checked}",
        rows.len(),
        stated
    );
    println!("conversion_failures={failures:?}");
    println!(
        "bdd_roots_unique={} canonical_merges={}",
        unique_roots.len(),
        checked.saturating_sub(unique_roots.len())
    );
    println!("bdd_nodes_unique={}", unique_nodes.len());
    println!(
        "bdd_nodes_per_root_p50={} p95={} max={} sum={total_nodes}",
        percentile(&node_counts, 50, 100),
        percentile(&node_counts, 95, 100),
        node_counts.last().copied().unwrap_or(0)
    );
    println!("conversion_seconds={:.3}", start.elapsed().as_secs_f64());
    Ok(())
}
