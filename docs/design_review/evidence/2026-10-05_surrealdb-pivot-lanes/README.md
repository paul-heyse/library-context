# SurrealDB pivot review: Phase A evidence lanes

**Question.** What do the current store, compute, graph and serving responsibilities actually
consist of, and can SurrealDB 3.3 (and Neo4j Community 2026.09 with GDS) deliver the system's
functional outcomes and guarantees natively? These lanes supplied repository facts and library
evidence. They render no verdict.

**Consumer.** The [SurrealDB graph-store design review](../../reviews/design_review_surrealdb-graph-store_2026-10-05.md)
and its [snapshot-model supporting document](../../reviews/design_review_surrealdb-graph-store_2026-10-05_snapshot-model.md).

**Baseline.** `main` at `1158ebe2`/`f66eb15a`; code is identical for every cited path. Library pins:
SurrealDB 3.3.0 (crates and `surrealdb/surrealdb:v3.3.0`), Neo4j 2026.09.0 Community with GDS,
the Python driver 6.3.1, and `neo4rs` 0.9.0-rc.10. All work ran on 2026-10-05 on the operator's
workstation.

## Lanes

| Lane | Report | Question | Method |
|---|---|---|---|
| A1 | [a1-store-surface.md](a1-store-surface.md) | Which code exists because of the PostgreSQL/DataFusion substrate, by responsibility? What are the catalog facts, and what are the store-related fix themes over three weeks? | Source reading; read-only `psql`; `git log` |
| A2 | [a2-graph-compute.md](a2-graph-compute.md) | Which operations are graph computations, by class? Where does existing time evidence place the cost? | Source reading; existing logs and receipts |
| A3 | [a3-surrealdb-capabilities.md](a3-surrealdb-capabilities.md) | Obligation → SurrealDB 3.3 mechanism → fit, traps, maturity | `neo4j-surrealdb` skill probes, docs, 3.3.0 crate source, issues; A3-run scratch checks |
| A3b | [a3b-snapshot-representations.md](a3b-snapshot-representations.md) | Snapshots as versioned graphs: structural sharing, diffs, Dolt/TerminusDB/XTDB/Lance, validation incrementality | Docs, source and scratch checks |
| A4 | [a4-neo4j-alternatives.md](a4-neo4j-alternatives.md) | GDS coverage of our analytics; integration cost; Rust graph libraries; PostgreSQL comparators | Skill, docs and release pages |
| A5 | [a5-obligation-map.md](a5-obligation-map.md) | Obligation → mechanism → dependents; second-order deltas per candidate | Source, tests, recipes and runbooks |
| A6 | [a6-dependency-fit.md](a6-dependency-fit.md) | SurrealDB SDK dependency resolution, families, footprint and removal ledger | `cargo metadata`/`tree`/`deny` and `check_family.py` on scratch manifest copies (not retained) |
| A7 | [a7-relation-shapes.md](a7-relation-shapes.md) | How graph-shaped is the declared model? | `lctx model describe` plus a scratch binary over `lctx-model`; classifier [a7classify.py](a7classify.py) and [a7load.py](a7load.py); output [raw/a7classes.json](raw/a7classes.json) |
| A8 | [a8-serving-query-shapes.md](a8-serving-query-shapes.md) | Serving read patterns, semantic decisions between hops, expressibility in SurrealQL and set-based SQL | Source reading; line classifier [scratch-checks/classify.py](scratch-checks/classify.py) |

`scratch-checks/` keeps the small helpers behind the A3 checks (`q.sh`, `bulk.py`). The generated
1,100-table DDL and the A6 manifest copies were regenerable scratch and are not retained. Paths in
the reports that point at the session scratchpad record where the work ran. Their durable copies
are the files in this folder.

## Results

The individual reports hold the results. The principal review states how they were weighed. Every
performance statement is **Proposed**: by operator direction, no benchmark was run.
