# Library leverage and displacement: review evidence (2026-10-04)

**Question.** Which generic mechanisms in the workspace could be displaced by more of a pinned
library's surface or by a new library, and which library-enabled design enhancements have a named
product consumer? Consumed by the
[library-leverage design/target review](../../reviews/design_review_library-leverage_2026-10-04.md).

**Baseline.** `main` at `948b2a8884650dca273b3bd7b9a373d58c4fcf55` (2026-10-04 14:37 -0400), with
concurrent analytical-enrichment repair work landing on the same branch. Static source, type,
documentation and interface inspection only unless a lane file records otherwise; no product
build, test or probe outcome is claimed (`not_run`).

**Contents.** Each file states its narrower role and baseline. Inventories record repository facts.
Lane files report library capabilities and fit. Verdicts belong to the review.

| File | Lane | Content |
|---|---|---|
| [`b0-baseline.md`](b0-baseline.md) | B0, coordinator | Standing library choices and their recorded reasons, open triggers, environment leads |
| [`m1-inventory.md`](m1-inventory.md) | M1, code-mapper | `lctx-model` execution, analysis, conditions and analytics: fixpoints, scheduling, ranking, FCA, BDD bounds, charging, in-memory validation |
| [`m2-inventory.md`](m2-inventory.md) | M2, code-mapper | `lctx-model` relational containers and joins, stage-port and relation-list repetition (workspace macro survey), canonical hashing, schemas, wire DTOs, ranking, text and rendering |
| [`m3-inventory.md`](m3-inventory.md) | M3, code-mapper | `cpg-extract`/`cpg-flow`: lexical recognizer, Ruff semantic facts, static branches, runtime predicates, `__all__`, docstrings, packaging |
| [`m4-inventory.md`](m4-inventory.md) | M4, code-mapper | `cpg-core`, `lctx-postgres`, CLI, embedding client, Python/PyO3: SQL composition survey, DDL, catalog diff, locks, runtime, config, errors, retry |
| [`l1-compute-storage.md`](l1-compute-storage.md) | L1, library-research | Pinned DataFusion/Arrow, sqlx, sea-query, pgpq, serde_arrow, schemars, tokio(-util), tracing |
| [`l2-analyzers.md`](l2-analyzers.md) | L2, library-research | Pinned Ruff, ty, Pyrefly and salsa surface; prior rejection reasons rechecked |
| [`l3-reasoning-graphs.md`](l3-reasoning-graphs.md) | L3, library-research | petgraph, rustworkx-core, leiden-rs, graphops, biodivine, OxiDD, ascent, datafrog, fcars/odis, typed-index and heap-size crates. Probes are in [`l3-probe/`](l3-probe/) |
| [`l4-serving.md`](l4-serving.md) | L4, library-research | FastMCP 4.0.5, MCP SDK, PyO3 and pyo3-async-runtimes |
| [`n1-rust-mechanisms.md`](n1-rust-mechanisms.md) | N1, library-research | New Rust mechanism crates mapped to inventory items: derives and reflection, relational containers, canonical encoding and hex, checked scalars, rendering, interval indexes, store tooling, testing |
| [`n2-ecosystem.md`](n2-ecosystem.md) | N2, library-research | New code-intelligence and Python-ecosystem tools, unenabled shared skills, comparable systems |

**Probe.** [`l3-probe/`](l3-probe/) is a small isolated Cargo package. It covers biodivine capped
quantification, rustworkx-core lexicographic order on a petgraph condensation, leiden-rs and
graphops PageRank, and an Ascent frontier lattice. It passed on 2026-10-04 (`cargo run --offline
--release`, rustc nightly 2026-09-28), and its output is recorded in `l3-probe/output.txt`. Rerun it
with `CARGO_TARGET_DIR=build/evidence-target cargo run --offline --release --manifest-path
docs/design_review/evidence/2026-10-04_library-leverage/l3-probe/Cargo.toml`. It does not join the
product workspace.
