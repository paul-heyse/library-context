# B0: settled library decisions, open triggers and leads

Supporting evidence for the
[library-leverage review](../../reviews/design_review_library-leverage_2026-10-04.md). Baseline
`948b2a88`. Compiled by the coordinator from current owners. Retired records are cited only where
they hold the reasoning behind a still-standing choice (recovered with `git show`). It records what
earlier decisions rest on. It does not judge them: whether a reason still holds is for the review.

## 1. Current authorities

- **Library policy:** [§14.11](../../../design/sections/api-and-evidence-product.md#section-14-11).
  It says to reuse the pinned Ruff/Pyrefly/ty, Arrow/DataFusion, PostgreSQL/SQLx/pgpq, graph and
  condition kernels, PyO3 adapters, FastMCP transport and embedding codecs. It selects no parallel
  analyzer, ORM, graph store, semantic Python engine or broad orchestration framework. It names
  conditional candidates with their triggers: Salsa, Ascent/Differential Dataflow, Moka,
  dense-index/interner/bitmap/persistent/small-vector crates, SeaQuery, validation/builder
  helpers, rewrite/unification/solver engines, new parsers.
- **Trigger tables:** the [forward plan §7](../../../plans/behavioral-model-forward-plan_2026-09-24.md#7-deferred-each-with-a-trigger)
  catalog-library and PostgreSQL triggers, and the §5 library table (lines ~850–866).
- **Binding:** K1 says product need and implementation fit are judged separately, and that
  library availability never establishes need. K2 says licences are never a rejection reason.
- **Principles:** DP-13/14/15/16 and G8, core §C/§F.

## 2. Standing choices and their recorded reasons

| Choice | Current owner | Recorded reason | Recorded reopen condition |
|---|---|---|---|
| Our own weighted PageRank (power iteration) | ADR-0044; `lctx-model::domain::analytics::ranking` | Retired 2026-09-23 leverage review D1. petgraph `page_rank` takes no weights, credits parallel arcs once and reports no convergence. graphops' petgraph feature pulls petgraph 0.6.5 | None stated beyond ADR-0044 |
| Our own NextClosure/Duquenne–Guigues FCA; fcars and odis as dev-only oracles | ADR-0044/0011; `analytics::concepts`; `docs/pins.md` (odis row, 2026-10-04) | D5: no Rust crate enumerates concepts plus a basis within an admitted allocation/work contract. odis was originally rejected for its AGPL licence; it is now a dev-only oracle consistent with K2 | FCbO if the concept count exceeds the budget; a sparse context above 64 attributes (W10) |
| leiden-rs fed from our own dense index, RBER, fixed seeds, never `from_petgraph` | ADR-0044 | D2: unnormalized orientation changed partitions; `from_petgraph` misreads arc weights | — |
| Condensation built from SCC membership, never petgraph `condensation` | ADR-0044 | D3: `update_edge` merges parallel arcs | When a consumer beyond recursion labelling appears |
| Our own BFS with parent pointers; not `Csr`/`GraphMap`/`StableGraph` | ADR-0044 | D4: parallel arcs and edge-id stability | — |
| Iterative petgraph SCC discovery | ADR-0052 | Stack safety | — |
| SCC-local bounded worklist for the first recursive value relation | ADR-0053; forward plan W12 | A deterministic bespoke worklist with explicit refusals. The [2026-09-26 same-state probe](../2026-09-26_summary-engine-comparison/README.md) compared Ascent 0.8.1, datafrog 2.0.1 and the petgraph worklist on one finite-base relation. All three agreed, but neither Datalog engine carried the ordered proof, the BDD conjunction, per-origin refusal or work accounting | **Open, all-channel only:** the comparison on the shared all-channel contract at S4 (W12 partial). ADR-0053 is revisited when effect, exception and role channels share a genuine multi-relation recursive rule. *Correction:* an earlier draft of this table said the Ascent comparison had never run; that was wrong. Lane L3's 2026-10-04 probes add to the 2026-09-26 evidence rather than being the first W12 evidence |
| biodivine-lib-bdd as the condition kernel; expand its use | Forward plan §5 table, W6/W11 | Bounded decisions, structural ids | — |
| OxiDD | Rejected for this workload (reasoning review 2026-09-25 §8) | A fixed-capacity manager fails as a whole; no ZBDD/MTBDD consumer | Pilot evidence that biodivine is too slow or too large |
| z3 | Deferred | No relational or ordered atom kind | A registered query needs a two-place or ordered atom; z3 would sit behind the theory, never become the kernel |
| datafrog | Rejected in favour of Ascent (2026-09-25) but still listed in the W12 comparison | Its value is run-time rule assembly, which nothing here needs | — |
| rustworkx-core | Keep bespoke keyed ordering (W13) | No pagerank or communities; randomly seeded components | A consumer for centralities beyond the current ones |
| Ruff `SemanticModel` port | **Rejected 2026-09-23** (retired review §8) | `ruff_python_semantic` 0.0.11; filled only by `ruff_linter`'s crate-private `Checker`; one binding per name. *Since then the workspace has linked an independent latest-Ruff fork, including `ruff_linter`, and ty's semantic index (ADR-0117)* | Not stated |
| Pyrefly `Bindings`/`find_definition` for name resolution | Rejected 2026-09-23 | Prunes statically decided branches; flow-sensitive | — |
| `ruff_python_stdlib` builtin lists | Rejected 2026-09-23 | Would be a second authority beside Pyrefly's builtins | — |
| `types.rs` term builder, independent `__all__` detector, textual `is_overload` | Kept 2026-09-23 | Pyrefly's visitor loses child roles, ordinals and parameter names; `is_overload` is retained as a second assertion | — |
| Codebook/column macros and `ArrowColumn` | Kept 2026-09-23 | serde_arrow would only replace builder loops; typed-arrow had no Arrow 59 support; strum/num_enum would split the codebook form. *serde_arrow is now adopted for the `Domain` derive* | — |
| `IdHasher`-style canonical encoding | Kept 2026-09-23 | Encoding is a contract; arrow-row bytes are not stable across releases | — |
| Recursive CTEs; typed-plan rewrite of generated SQL; DataFusion `Constraints` | Rejected 2026-09-23 (Delta era) | No recursion limit in `RecursiveQueryExec`; SQL text was hashed and snapshot-tested; constraints are informational. *The forward plan §5 later lists "DataFusion `WITH RECURSIVE` (compile time)" as a Stage 4 adoption, so the records disagree* | — |
| SQLx as the only driver; no ORM; Clorinde/Cornucopia not selected | ADR-0068; catalog library-fit review (2026-09-28) | One effect owner | F6 for Python psycopg/SQLAlchemy |
| Schemars and Rust `jsonschema` for wire schemas | ADR-0073 (adopted) | One Rust schema authority | — |
| gix; `ignore`; criterion/divan, miette, metrics, sysinfo | Rejected 2026-09-23 | Burden without a consumer | A second git operation; any consumer of their output |

## 3. Open triggers to test against current code

From forward plan §7 and §14.11. Each needs evidence that it has fired, not just that a library
is available.

| Candidate | Trigger as recorded |
|---|---|
| Salsa (catalog derivation) | Repeated fine-grained build workload shows coarse reuse is insufficient |
| Ascent / Differential Dataflow | Genuine supported recursive multi-relation rules outgrow a clear worklist / actual continuous insert-delete views |
| Moka | Measured repeated immutable-generation selection/hydration cost |
| Typed dense collections (TiVec, cranelift-entity) | Multiple dense index domains are actually needed |
| lasso, roaring | Allocation or sparse-set cost demonstrated |
| trybuild | Meaningful cross-module ID/state invariants justify compile-fail cases |
| nutype, garde, bon | A repeated checked scalar, contextual form or required-field builder consumer |
| Generated Python models, Typify, Specta | Python needs typed domain manipulation, or an external schema or client consumer exists |
| egg/egglog, ena, Z3/SAT/OxiDD | A pure rewrite/unification or reasoning gap beyond the bounded models |
| strum, enum-map, serde_with, derive_more; imbl, rpds, ecow, small vectors | Repeated local construction, or a measured cloning/branching/allocation consumer |
| SeaQuery (F4) | Substantive typed variable joins/expressions beyond clear SQLx queries |
| F6 psycopg/SQLAlchemy; F7 LISTEN/NOTIFY jobs; F8 pgrx; F11 ADBC; F12 DataFusion client protocols; F13 PostgreSQL FTS, trigram or LanceDB | Each as listed in forward plan §7 "PostgreSQL adoption triggers" |
| Pyrefly Glean collector | A consumer needs attribute cross-references (retired review §7). *Corrected by lane N2:* the retired review's companion deferral of `parse_parameter_documentation` has already been resolved, because `crates/cpg-extract/src/docstrings.rs` uses it |
| ty as a second type provider; the `bytecode` CFG | A question Pyrefly's types cannot answer; an effectful-frame case |

## 4. Leads from the environment (no adoption implied)

- **Shared skill store**, not enabled here: `arrow-polars-duckdb`, `cognee`, `deltalake`,
  `gliner2-spacy`, `hf-tokenizers`, `jena`, `lancedb`, `markdown-latex`, `mineru-docvortex`,
  `native-solver-libraries`, `neo4j`, `pydantic`, `python-postgres`, `stats-metrics`,
  `symbolica-faer-oximo`, `vllm`.
- **`docs/library-utilization.jsonl`** (entries verified 2026-10-03): 68 libraries, 47 used, 18
  `not-used` with an indexing skill (ascent, datafrog, oxidd, z3, graphina, graphops, raphtory,
  rust-igraph, rustworkx-core, cargo_metadata, rust-analyzer, rustdoc-types, ast-grep libraries,
  datafusion-tracing, opentelemetry, object_store direct, sqlparser direct, salsa macros) and 2
  test-only (bitvec, fcars). Some wrapper paths in it lag the tree; for example, `crates/cpg-schema`
  no longer exists.
- **Transitively compiled already** (`crates/lctx-workspace-hack/Cargo.toml`): `dashmap`,
  `indexmap`, `itertools`, `smallvec`, `thin-vec`, `compact_str`, `strum`, `serde_with`,
  `similar`, `regex`, `aho-corasick`, `rustc-hash`, `foldhash`, `arc-swap`, `parking_lot`,
  `tokio-util`, `tokio-stream`, `uuid`, `time`, `chrono`, `rust_decimal`, `bigdecimal`, `zstd`,
  `zip`, `unicode-normalization`, `unicode-width`. Being compiled already lowers integration cost;
  it does not establish adoption.
