# Semantic data model target review — probes and reconstruction

**Consumer:** [semantic data model target review](../../reviews/design_review_semantic-data-model_2026-09-29.md)
(F01–F13, §8–§10), and the target ADR and forward-plan track that follow it if the operator accepts
it. Evidence, never authority.

**Question.** Can the current relations support one shared semantic model? Specifically:
- Where are semantic decisions re-derived?
- Which library capabilities fit the proposed relations, projections and operators at the pinned
  versions?
- What would storing derivations cost?

**Baseline.** Code at `35afc09`; `crates/` and `python/` are unchanged through `fedd4a0`. Pilot store
`build/store` → `build/pr5-qualified/behavioral/store`, snapshot `d0cf20ff376d8b2f18f1535977a1b97d`
(behavioral profile). Machine: the operator's workstation, pinned toolchain `nightly-2026-09-29`
from `rust-toolchain.toml`. Date: 2026-09-29.

## P0 — one transfer relation over the existing relations (known-answer fixture)

**Question.** Do the existing relations express:
- option → field → reader;
- guarded per-branch supply through a summarized callee;
- facade delegation?

And do the controls hold?

**Fixture.** [`p0-fixture/p0shapes/__init__.py`](p0-fixture/p0shapes/__init__.py). It has:
- `Config(timeout, title)` and a field reader;
- `identity` called twice in `pair`;
- the external review's `select_timeout`/`fetch`;
- a function facade and a method facade;
- an unresolved callee.

**Run** from the repository root, with the release CLI built:

```sh
docs/design_review/evidence/2026-09-29_semantic-data-model/p0_queries.sh <scratch-dir>
```

**Passed 2026-09-29.** Output is in `raw/p0-queries.txt`. That committed run is snapshot `13b64de0e789c124d821884852cce393`; snapshot ids are random per compile, and an earlier identical run was `d426f237…`. The results are canonical Delta rows; MCP rendering was not traced.

**Controls**
- `title` never reaches `timeout` (separate `Stores` rows).
- The two identity calls never cross: `first` does not reach `pair`'s return.
- `fetch`'s return through the unresolved `transport` stays `unknown`/`call_transfer`.
- `select_timeout` has two conditional summary flows with the exact guards.

**Findings**
- `facade` is served as `returns x, computed, established` although its summary proves an
  identity (review F04).
- `read_timeout` records `config → return` with no field access path, so option → field → reader
  cannot be joined.
- `fetch` passes `timeout`/`fallback` to `transport(timeout=…)` only as through-call rows with
  condition `true`. The per-branch supply the external review describes is not derivable
  (review F06).
- The method facade's `unknown` is `override_dispatch`, which is correct open-world behavior.
- A predicted boundary/coverage reason divergence did not reproduce: both tables say `call_transfer`.

## P3 — graph and reasoning library capabilities at the pins

**Question.** Library fit for the target's topology, condition and rule mechanisms (review §8).

**Sources.**
- [`p3-graph-reasoning/`](p3-graph-reasoning/): petgraph 0.8.3, biodivine-lib-bdd 0.6.3,
  ascent 0.8.1 (default features off), fixedbitset 0.5.7.
- [`p3-rustworkx/`](p3-rustworkx/): rustworkx-core 0.18.1 on petgraph 0.8.3.

These are isolated workspaces, not members of the product workspace.

**Run:**

```sh
export CARGO_TARGET_DIR=<scratch>/p3-target
E=docs/design_review/evidence/2026-09-29_semantic-data-model
for b in p3-graph-reasoning views views_neg sizes; do cargo run --locked --quiet --manifest-path $E/p3-graph-reasoning/Cargo.toml --bin $b; done
for a in "kosaraju 1000000" "toposort 1000000" "tarjan 1000" "tarjan 100000"; do (ulimit -s 8192; $CARGO_TARGET_DIR/debug/deep $a); done
cargo run --locked --quiet --manifest-path $E/p3-rustworkx/Cargo.toml
```

**Passed 2026-09-29** (`raw/p3-graph-reasoning.txt`). Each control came out the other way, as intended:
- **PageRank:** petgraph's `page_rank` differs from the repository's weighted power iteration by up
  to 3.196e-2 once a dangling node and a parallel arc are present. The control, a 3-cycle, differs
  by 0.
- **`Csr`:** refuses a duplicate pair (`from_sorted_edges` errors; a second `add_edge` returns
  `false`).
- **`condensation(…, true)`:** keeps one weight per component pair.
- **Views:** `NodeFiltered`, `EdgeFiltered` and `Reversed` compose with Tarjan, Kosaraju, `toposort`,
  `has_path_connecting`, `simple_fast` (post-dominators over `Reversed`) and `dijkstra`. The negative
  control `views_neg` (`page_rank` over `NodeFiltered`) fails to compile, as intended (E0277
  `NodeCount`).
- **Recursion depth:** `kosaraju_scc` and `toposort` handle a 1M-node path. The control, recursive
  `tarjan_scc`, overflows an 8 MiB stack at 100k.
- **ascent:** accepts a BDD condition ordered by `Bdd::cmp_implies`, with bounded-OR join and
  `unknown` as top. On a cycle it gives `reach(1) = (a | b)`. `derived_by` holds one row per
  distinct rule match.
- **rustworkx-core:** `lexicographical_topological_sort` reproduces the repository's callee-first
  schedule.
- **Sizes:** `Graph` spends 8 B per node and 20 B per edge.

The full capability analysis is [`reconstruction/L1-graphs-reasoning.md`](reconstruction/L1-graphs-reasoning.md).
It also covers typed-index-collections versus cranelift-entity, roaring, biodivine substitution,
existential projection and serialization, and datafrog.

## P8 — derivation and graph-catalog volume in the pilot

**Question.** Is storing AND/OR derivation records affordable? What share do `nodes`/`edges` take?

**Run:** `python3 docs/design_review/evidence/2026-09-29_semantic-data-model/p8_volume.py build/store`.
It reads only the `snapshots` row counts and the on-disk sizes.

**Measured 2026-09-29** (`raw/p8-volume.txt`):

| Measure | Rows | Size | Share |
|---|---|---|---|
| All canonical tables | 7,737,960 | 321.5 MiB | — |
| Support and step tables | 158,765 | 7.0 MiB | 2.1% of rows, 2.2% of bytes |
| `nodes` + `edges` (never served) | 2,355,231 | 121.5 MiB | 38% of bytes |

**Finite summaries:** 23 `summary_flows` against 13,996 components, 3,705 boundaries and 73,636
origin-coverage rows.

**Compile stage costs** come from the other session's PR5 qualification run, cited as a historical
receipt (`build/pr5-qualified/behavioral.log`), not re-run:

| Stage | Time |
|---|---|
| Total | 473.9 s |
| Extraction | 36.3 s |
| Validation | 177.2 s |
| ContractNormalization (recomputed) | 58.6 s |
| Flow model | 32.2 s |
| `nodes` + `edges` derivation | 3.0 s |

## Paper probes (no execution)

- **P2 — shared fixed-point driver.** See the engine signature table in
  [`reconstruction/A2-behavioral.md`](reconstruction/A2-behavioral.md) §2. Only two of twelve engines
  iterate to a fixed point, and they differ on merge, cap semantics and the key/witness split. A
  shared driver is not justified. Refusal policy, named budgets and the SCC schedule can be shared.
- **P7 — registry second authorities.** See the aspect table for `edges`, `summary_flows`,
  `catalog_parameters`, `behaviors` and `retrieval_units` in
  [`reconstruction/A3-catalog-pipeline-serving.md`](reconstruction/A3-catalog-pipeline-serving.md) §3.
  18 catalog serving files already derive from `table!`; 49 other served files are written by hand.
  A code probe was unnecessary because the derived route already exists in the tree.

## Reconstruction notes and the external-review claim ledger

Read-only layer reconstructions, cited `path::symbol:line` at `35afc09`, each labelled Implemented
or Interface-checked by its author and spot-checked by the lead reviewer. The review re-read every
span it cites.

| File | Contents |
|---|---|
| [`reconstruction/A1-graph-analytics.md`](reconstruction/A1-graph-analytics.md) | Analysis records for 14 representations; call edge, caller, unresolved and modality maps; fidelity triage; graph-table readers; lineage; claims E1–E6 |
| [`reconstruction/A2-behavioral.md`](reconstruction/A2-behavioral.md) | Flow, condition, summary, behavior and model fidelity; engine signatures (P2); place and transfer inventory; binding, refusal, verdict, discharge, DNF/BDD and transfer maps; occurrence identity; support tables; claims E7–E11 |
| [`reconstruction/A3-catalog-pipeline-serving.md`](reconstruction/A3-catalog-pipeline-serving.md) | Catalog, association and retrieval fidelity; catalog analysis records; registry aspects (P7); stage table; identities; per-tool serving and failures; invariants; claims E12–E17 |
| [`reconstruction/L1-graphs-reasoning.md`](reconstruction/L1-graphs-reasoning.md) | Graph and reasoning library capabilities (P3) |
| [`reconstruction/L2-incremental-relational.md`](reconstruction/L2-incremental-relational.md) | salsa 0.28.2, DataFusion recursion and plan building, arrow-ipc, Delta CDF, PostgreSQL 18, moka, differential dataflow |

**Where a claim here differs from the review, the review's re-read governs.** Two differences:
- One reconstruction predicted that method facades are refused by a binder mismatch. P0 shows
  `override_dispatch`.
- The same reconstruction predicted a boundary/coverage reason divergence. It did not reproduce.
