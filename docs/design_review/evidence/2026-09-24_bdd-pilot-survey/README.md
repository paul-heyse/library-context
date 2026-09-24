# BDD conversion survey over the published FastMCP pilot

This is a read-only design probe. It loads `conditions` through the snapshot's pinned
Delta/DataFusion reader, parses the existing Stage 2 encoding, builds a BDD where possible,
and validates its structural node closure. It reports conversion boundaries, canonical
merges, node counts and wall time. The pilot compiler still uses DNF; this probe cannot
recover conditions already recorded as `over_budget` or measure a Stage 3 product compile.

Run from the repository root:

```bash
CARGO_TARGET_DIR="$PWD/target" cargo run --locked \
  --manifest-path docs/design_review/evidence/2026-09-24_bdd-pilot-survey/Cargo.toml -- \
  build/store ecf8b9cdc11ebaf4c1dae61fa9b35dee
```

**Observed 2026-09-24, snapshot `ecf8b9cdc11ebaf4c1dae61fa9b35dee`:**

```text
condition_rows=13776 stated=13775 bdd_converted=13775
conversion_failures={"SourceOverBudget": 1}
bdd_roots_unique=13775 canonical_merges=0
bdd_nodes_unique=23760
bdd_nodes_per_root_p50=1 p95=8 max=53 sum=36217
conversion_seconds=0.967
```

The first cold debug build took 2m 05s; a locked rerun after a probe-only edit took
2.66s to build and 0.97s to convert/validate. Every *stated* Stage 2 condition fits the
current 128-atom/50,000-node kernel limits. One `over_budget` row has already lost its
source Boolean function, so conversion of materialized rows alone cannot remove its
downstream `budget_reached` claims. The BDD writer must move before Stage 2's DNF cap.
The 23,760 distinct node IDs are a lower-level storage-size estimate, not a published
table measurement. No observed pair of distinct Stage 2 rows canonicalized to one root.

A separate read-only `lctx query` against the same snapshot joined the single
`encoding = 'over_budget'` condition row back to its uses: **237 `flow_regions` and
846 `flow_reaching` rows**, spread across modules. The largest region counts were
`fastmcp/server/auth/identity_assertion.py` (40),
`fastmcp/utilities/openapi/schemas.py` (25), and
`fastmcp/utilities/json_schema_type.py` (24). These are references to one collapsed
sentinel, not evidence of one underlying Boolean expression. This strengthens the need
to construct BDDs before the DNF limit and retain per-expression identity.

The count query was:

```sql
SELECT 'flow_reaching' AS relation, count(*) AS rows
FROM flow_reaching r JOIN conditions c ON r.condition_id = c.condition_id
WHERE c.encoding = 'over_budget'
UNION ALL
SELECT 'flow_regions', count(*)
FROM flow_regions r JOIN conditions c ON r.condition_id = c.condition_id
WHERE c.encoding = 'over_budget';
```

The standalone probe has its own `Cargo.lock`; the workspace source crates and their
direct engine pins match the product (checked for BDD 0.6.3, DataFusion 55.1.0,
Arrow 59.3.0, delta-rs `58f07cd`, and Tokio 1.53.1), while some unrelated transitive
versions may differ.
Treat this as a design survey, not compiler or publication acceptance.
