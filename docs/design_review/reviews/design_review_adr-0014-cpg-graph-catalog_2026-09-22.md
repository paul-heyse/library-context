# Design review: ADR-0014, the CPG graph catalog, and CPG slice C1 (standard + compact)

**Date:** 2026-09-23 (file dated to the ADR) · **Depth:** standard, which also serves as C1's
compact slice review · **Mode:** document plus code, as committed in `6bf223e` (ADR, DESIGN,
ADDENDUM) and `ce26580` (C1). ADR-0014 changes §B6's scope and §3.2's family → node/edge mapping,
so a `standard` review is owed before it is accepted (ADR-0001).
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`6bf223e` and `ce26580`.
**Prior reviews that bear on this:**
- `design_review_inc1-slice2-derive-validate-publish_2026-09-22.md`: F1 (a release target that
  names nothing must say why; oracle `semantic:release-target-explained`), O4 (boundary
  provenance), O8 (generate `REFERENCES` and the node/edge mapping from one declaration).
- `design_review_adr-0013-library-acquisition_2026-09-22.md`: "one production path for any
  Python library".

C1 closes slice-2 O4. It claims to close O8 and does not (F4). It removes slice-2 F1's rejection
and puts nothing that can fire in its place (F1).

## 1. Decision and scope

**Proposal.** ADR-0014 (`proposed`, `evidence: Tested`) supersedes ADR-0008:
- The typed family tables stay the only writable authority. `nodes` and `edges` become Stage-D
  derived tables (family `graph`), generated from one registry, `cpg_schema::graph`.
- Every edge gets a content-derived `edge_id = H(edge, kind, src, dst, ordinal, discriminator)`.
  Every endpoint is a typed node, which adds `external_module`, `external_symbol` (from a second
  Pyrefly check over the referenced dependency modules) and `synthetic_callable`.
- Ids are computed in SQL by the `lctx_id` UDF.
- Every Delta table turns expired-log cleanup off. Boundary facts become surface `compare` with
  `relational_derivation`.
- `lctx compile` reports per-stage metrics, and `lctx query` inspects a published snapshot.
- ADR-0004 is amended: CPG slices C1–C6 now run before Pass A.

**Status of the claims.**
- DESIGN §3.8 is **Implemented** and **Tested** for C1, with a **Measured** pilot paragraph.
- §3.4.1's "Ids in SQL" is **Implemented** and **Tested**, as are §6.1 retention and §8's graph
  rules.
- The ADR says `evidence: Tested`.

**Observable outcome.** Projections become selections over one node relation and one edge
relation:
- isolates survive;
- parallel sites keep distinct identities;
- every relationship traces to a raw evidence fact;
- published snapshots stay loadable at their recorded versions indefinitely.

**Baseline.** ADR-0008 had typed family tables and derived joins. The mapping was deferred, and so
were edge ids and node kinds. A call target outside the release was a null node with a null reason.

**Supported scope and non-goals.**
- In scope: C1's 10 node kinds and 13 edge kinds over the increment-1 families, the registry
  rules, the UDF, retention and metrics.
- Out of scope: C2–C5 (still Proposed), the `GraphProjectionSpec` (lands with the first
  projection), and the operator's decision to put the CPG before the analytics. That decision is
  taken as given. Only its proportionality is noted, in §8.

**Constraints and uncertainty.**
- Pilot: FastMCP 4.0.5 (275 modules).
- Identity: ids must be stable across runs, locations and module order.
- Publication: the protocol is ADR-0009's and is unchanged.

### Method and coverage

**Read in full:**
- ADR-0014;
- DESIGN §1.2, §B6, §3.1–§3.8, §4.0 (post-C1), §4.3, §5, §6.1–§6.3 and §8;
- the operator's guidelines and ADDENDUM §5;
- `graph.rs`, `derived.rs`, `rules.rs`, `udf.rs`, `context.rs` and `id.rs`;
- `cpg-core/tests/graph.rs` and its snapshot.

**Read as diffs** (`git show ce26580`): `tables.rs`, `codebook.rs`, `metrics.rs`, `attempt.rs`,
`delta.rs`, `snapshot.rs`, `pysa_map.rs`, `walk.rs`, `facts.rs`, `public.rs`, `library.rs`,
`lib.rs`, `lctx/main.rs`, and the `compile.rs` and `delta.rs` tests.

**Checks run in this session** (2026-09-23):

| Check | Command | Outcome |
|---|---|---|
| Workspace tests | `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass` | **passed** (70/70) |
| Default loop | `just check` | **passed**: fmt-check, clippy `-D warnings`, nextest 70/70, pytest 21/21, pyrefly, rules-scan, rules-test 4/4, lint-agents, adr lint (14 records) |
| Rest of `test-all` | `just fixtures-check deps gold` | **passed**: 27 fixture files; family, cargo-deny, pyrefly-fork and gold all ok |
| Pilot | `just pilot` | **not_run**: it writes a new snapshot into `build/store`. The existing published snapshot `d45f60ba08671ea472ffb8229626a2e7` was probed instead. `target/release/lctx` was built at 00:24. `graph.rs` and `derived.rs` are unchanged since then; `lctx/main.rs` and `attempt.rs` were touched at 00:28 |

**Probes.**
- **P1–P7** used `target/release/lctx query --store build/store --snapshot d45f…`:
  - P1: the node and edge counts per kind match §3.8's Measured line (47,145 nodes, 65,176 edges).
    No call, ancestry or override target has a reason.
  - P2: every published `edge_id` recomputes from its row plus the evidence's `payload_id`
    (65,176 of 65,176).
  - P3: the SQL recipe does not reproduce the Rust role ids (F3).
  - P4: the published `call_targets` SQL, replayed over two mutations of `pysa_calls` (F1).
  - P5: `pysa_calls` partitioned by lineage class (F5).
  - P6: an audit of the 335 `variable_origin` exports.
  - P7: synthetic callables as traversal stops (F5).
- **P8** ran entirely in the session scratchpad; nothing in the repo changed:
  - `lctx library init attrs --requirement 'attrs==26.1.0'`, then `lctx compile attrs`, each with
    `--libraries`, `--envs` and `--store` pointing into the scratchpad;
  - then `datafusion-cli` over that scratch store's Parquet files to name the failing rows. This
    scan is allowed only because the attempt is unpublished and in a scratch store with one
    version per table; §6.2 forbids it on the canonical store, and `lctx query` reads only
    published snapshots.

**Not inspected:**
- the untracked `crates/cpg-extract/src/syntax.rs`, which is C2 work in progress and not in
  `lib.rs`;
- `derive.rs` and `validate.rs`, which C1 does not change;
- the pytest and Python side;
- the full `pysa_map.rs` beyond its diff;
- DISPOSITION and STATUS edits.

**Asserted, not attacked:**
- "two runs give one `content_digest`" on the pilot (not re-run; the fixture determinism test
  passed);
- that `higher_order_index` is the argument ordinal in general. All 394 pilot rows map, but no
  argument was built to break it;
- retention at real scale: 100-commit checkpoints and a 30-day clock. The test uses interval 2
  and zero retention, and it proves the two properties together, not `interval 36500 days`
  parsing on its own;
- concurrent compiles into one store.

**Attacked, and held:**
- inner joins in the catalog SQL dropping endpoints. Every dropping join lands in an endpoint or
  lineage failure (`nodes` parameter and argument sources; the `edges` joins can't drop);
- `fact_id` leaking into `edge_id` (P2 plus the SQL);
- a failed compile publishing anything (P8: "nothing published");
- boundary provenance (the pilot's 207 boundary facts are `compare`, `relational_derivation`,
  `derived_analysis`).

## 2. Authority and lifecycle map

| Concept or fact | Semantic type and identity | Authority / owner | Revision or snapshot boundary | Permitted update path | Derived representations |
|---|---|---|---|---|---|
| Raw family rows (declarations, `pysa_calls`, `public_names`, …) | typed Arrow rows; `fact_id` per run | the extractor, one surface per table | per attempt, `snapshot_id` | an append per attempt | every Stage-C/D table |
| Dependency context (`context_modules`, `context_definitions`) | `external_module` = H(owner, version, name); `external_symbol` = H(module, def kind, Pysa key), both in the **plain** encoding (F3) | the extractor (`context.rs`), filtered to what the release references (`context.rs:246-248`) | per attempt | an append | `external_symbol`/`external_module` nodes, and typed targets |
| The registry | Rust values in `graph::node_sources`/`edge_sources` | `cpg_schema::graph`, one declaration | `compiler_digest`, **only** through the SQL and rules it generates. `derivation` and `direction` are outside it (F6) | code change | the `nodes`/`edges` SQL; 67 of 247 rules |
| Derived resolution rows (`call_targets`, `ancestry_targets`, `override_targets`, `exports`, `synthetic_callables`) | keys + evidence `fact_id`s + a reason | the Stage-D SQL in `derived.rs` | per attempt | re-derivation only | the edges; the reasons read by analytics |
| `nodes` / `edges` | `node_id` from each kind's existence source; `edge_id` content-derived, stable across runs (P2; the fixture determinism test) | derived from the registry. The family tables stay the authority | per attempt, validated before the `snapshots` append | re-derivation only | future projections |
| Id recipes | `H(kind, fields…)`: plain encoding in Rust (§3.4.1 "Encoding"), opt_* in SQL (§3.4.1 "Ids in SQL") | `IdHasher`, and `lctx_id` (`udf.rs`) | `udf::VERSION` (hand-bumped); `lctx-id/v1` | a migration | every id column |
| Node-valued references | column → allowed targets | **two lists**: `rules.rs:52-146` `REFERENCES` (hand-written) and the registry's endpoint kinds (F4) | the rules text in `compiler_digest` | code change | `ref:` and `endpoint:` rules |
| Retention | two Delta table properties | `delta::retention()` | set at create; exact-string verify at open | none (a table that differs is refused) | — |
| Stage metrics | wall time, `VmHWM` | `metrics.rs` | not stored (not content) | — | `lctx compile` output |

**Deliberately opaque behavior.** Pyrefly's collectors (Pysa definitions, call graphs, public
names) and the Ruff walk, both behind ADR-0012's contract. `context.rs` checks that each dependency
handle is the one Pysa numbered (`context.rs:133-138`), which keeps the opaque part honest about
*which* module it describes.

**Identity behavior.**
- Rename or move: node ids include release-relative paths or qualified names, as before.
- Moved environment: external ids use the owning distribution and version, never the path
  (Tested: the location test).
- Module order: Tested.
- Duplication: two public-name rows for one access path and one target collide (F2).
- Regeneration under a new Pyrefly:
  - external symbol ids and call edge ids change, because Pysa keys and the payload digest are
    producer-scoped;
  - this is stated for external symbols, and not for edges (O6).

## 3. Semantic contracts and invariants

| Contract or invariant | Representation | Enforcement boundary | Failure behavior | Verification evidence |
|---|---|---|---|---|
| One id, one kind | `key:nodes` over `(snapshot_id, node_id)` | Stage-D validation | abort, nothing published | the rules snapshot. A same-kind collision is merged, not rejected (O2) |
| An edge is unique by content | `key:edges` | validation | abort | **fails on attrs 26.1.0 for a legitimate shape** (F2) |
| Endpoints exist and have allowed kinds | `endpoint:<kind>`, generated | validation | abort | Tested: a raw parent rewritten to a call site (`compile.rs`), and a doctored `src_kind` (`graph.rs`). The existence half is tautological wherever the endpoint was joined from its own existence source (9 of 13 kinds); the kind half is real |
| Evidence belongs to the declared table | `evidence:<kind>` joins `facts.table_name` | validation | abort | Tested (doctored) |
| One edge per evidence row; no forbidden parallels | generated | validation | abort | Tested (doctored) |
| Every raw row yields its edge or is explained | `lineage:<kind>`: expected raw ids ⊆ edge evidence ∪ (derived rows with **any** reason) | validation | abort | Tested for a row the span join drops. **Cannot fail** for a joined row whose target doesn't resolve, because every derivation ends in a reason (F1) |
| A null target always says why | `typed:<table>` | validation | abort | only a doctored table can fail it. The derivations cannot produce null/null (F1; P4) |
| Pending Pysa rows are a declared class | a `WHERE` filter in `expected` (`graph.rs:432-434`) | none | none | 49% of pilot `pysa_calls` rows are excluded without a count (F5) |
| Retention for pinned reads | `enableExpiredLogCleanup=false`, `logRetentionDuration=interval 36500 days` | create; verify at open | open refuses (`CoreError::Retention`) | Tested (`retention_keeps_old_versions_loadable`) |
| `lctx_id` accepts only lossless types | `return_type` refuses Int32, UInt64 and floats | plan time | plan error | Tested (`unsupported_types_are_refused_at_plan_time`) |
| Rust and SQL compute one id per recipe | claim (§3.4.1 L523-524, `udf.rs:1-4`) | none | — | false for every Rust-computed C1 id (F3, P3) |

**Absence and uncertainty.** For one Pysa call record, the published states are:

| State | Representation | Distinct? |
|---|---|---|
| (a) Resolved to a node | an edge | yes |
| (b) The provider describes it, with no `def` of its own | `synthetic_callable` | yes |
| (c) Pysa's key maps to a release `def` Stage C cannot place | the Stage-C reason (`provider_disagreement`) | yes |
| (d) Pysa names a function **our tables do not contain** (a release path or dependency key we failed to collect) | `missing_evidence` | **no**: indistinguishable from (e) (F1) |
| (e) The provider has no record | `missing_evidence` on `resolutions` | yes, as its own meaning |
| (f) Unresolved in Pysa's own model | `resolutions.unresolved_reason` and remainder | yes |
| (g) A site class pending C2/C3 | absent from `call_targets` and `edges`, with no marker in data | **no** (F5) |

**Equivalence requirements.**
- Ids are byte-equal across runs, locations and module order (Tested on the fixture).
- Edge ids recompute from the published row (P2).
- Rust and SQL recipes are *claimed* equal and are not (F3).

## 4. Derivation and execution design

| Stage or operation | Input revisions and dependencies | Output contract | Preconditions / assumptions | Effects and mutable ownership | Provenance / invalidation |
|---|---|---|---|---|---|
| Release check + extraction | constructed Pyrefly config; release handles | raw tables (C1 adds `pysa_classes`, `payload_id`, typed pairs, `arguments.node_id`) | ADR-0012 driver | in-process; the attempt aborts on panic | `run_id` (the producer's output version bumped to 6) |
| Dependency check + definitions (`context.rs`) | the referenced module set, collected while mapping; export origins | `context_modules`, `context_definitions` | each module resolves to the handle Pysa numbered (else an error) | a second `txn.run` at `Require::Everything` (+3.7 s, +0.7 GB) | the same run. Filtered to the referenced keys, so existence is not independent of what references it (F1) |
| Stage C/D derivations | the attempt's tables at their written versions, filtered to the snapshot | 13 derived tables, `nodes` and `edges` last | the `lctx_id` UDF registered | written through `DeltaTable::write` | the SQL is in `compiler_digest` |
| Validation | all tables | 247 rules | read-only helper | none | the rules are in `compiler_digest` |
| Publication | validated versions | one `snapshots` append | validation passed | the only publication act | unchanged (ADR-0009) |

**Relationship structures.** DM-34 is satisfied. Each of the 13 edge kinds has its own direction
sentence and endpoint kinds. Ownership (`declares`, `encloses_call`, `has_parameter`,
`has_argument`), resolution (`call_target`, `higher_order_target`, `overrides`, ancestry) and
provenance (`declared_in`, `stub_for`) stay separate kinds. The graph-readiness reader selects
`encloses_call` ∘ `call_target` rather than traversing a union.

**Provider selection and limitations.** C1 adds none beyond ADR-0012. The dependency check reuses
the one constructed config.

**Boundary contracts.**
- Delta ↔ Arrow: unchanged.
- SQL ↔ Rust ids: the one new boundary, and the one with a false equivalence claim (F3).

**Coherent publication.** Unchanged. The catalogs are part of the attempt and validated before
the append (P8: a failed attrs attempt published nothing).

## 5. Representative journeys

**Ordinary extension: C2's `argument_value` (argument → expression node).**
- New semantic content:
  - a `syntax_node` `NodeSource`;
  - one `EdgeSource`;
  - two codebook appends;
  - a fixture.
- Generated: the SQL, plus the endpoint, evidence, one-per-evidence, no-parallel and lineage
  rules. That is five rules from one declaration, the leverage DM-52 asks for.
- Not generated:
  - the `REFERENCES` entries (F4);
  - the pending-class bookkeeping (F5).
- Hazard: if the syntax-node id is computed in Rust (the walker) and any SQL joins on
  `lctx_id('syntax', …)`, the join matches nothing (F3), and F1's catch-all turns that into
  reasons rather than a failure.

**Meaningful change: a Pyrefly bump.**
- It changes `FunctionId::serialize_to_string` or any Pysa payload field.
- Every call edge's `payload_id`, and so its `edge_id`, changes (O6), as do the external-symbol
  ids (stated).
- Node ids of release declarations survive.
- If the dependency-key path and the target-key path drift apart, up to 13,122 of 17,174 pilot
  targets become `missing_evidence` and validation passes (F1, P4b).

**Boundary: SQL ↔ Rust ids.** On the pilot, the recipe the DESIGN states for `argument` gives
0 of 19,493 matches when computed in SQL (P3). No C1 query crosses the boundary, so nothing breaks
yet.

**Interruption or failure.**
- The attrs compile (P8) fails `key:edges` and `no-parallel:exports`, publishes nothing, and
  reports the rule names. This is correct failure behavior for a defect that should not exist
  (F2).
- Retention: a published snapshot remains loadable after checkpoints and cleanup would have run
  (Tested).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 — Authority | **pass** | The family tables stay the only writable authority. The catalogs carry no payload and are regenerated from one registry. Every edge recomputes from its row (P2). Two items are latent: `stub_for` re-expresses the `exports` seed rank (O1); the id encoding is split by computation site (F3). Neither disagrees in published data today. `REFERENCES` vs the registry (F4) are two validators, not two data authorities | Close F3 before any id is computed on both sides (C2) |
| G2 — Semantic fidelity | **fail** (narrow) | `missing_evidence` now carries two states: "Pysa has no record" and "Pysa named something our tables lack" (F1; §3 lattice rows d/e). Pending Pysa rows have no data-level marker (F5, partial) | F1 correction |
| G3 — Validity | **fail** | A Pysa target naming a nonexistent release file, or a dependency key the collectors never described, publishes. Slice-2's `semantic:release-target-explained` rejected the first case; C1 removed it and rewrote its negative test. The replacement `typed:*` rules cannot fail on derived data. Replay: 4,052 and 13,122 null targets, 0 violations (P4). This is ADR-0014's own revisit trigger | F1 correction plus the restored negative tests |
| G4 — Hidden behavior | **pass** | The second Pyrefly check uses the constructed config; dependency handles are checked against Pysa's numbering; metrics read `/proc` but are declared non-content and never stored; `lctx query` goes through the read-only helper | — |
| G5 — Consistency and recovery | **pass** | Protocol unchanged. Retention makes old snapshots loadable (Tested). Open refuses a table without the properties. A failed attempt published nothing (P8) | — |
| G6 — Transformation and reuse | **pass** | `edge_id` has no run-scoped input (`graph.rs:676-678`; `payload_id` hashed over zeroed ids). Determinism across location and reversed order is Tested. The UDF refuses lossy types. The Rust/SQL encoding split (F3) affects no C1 path | F3 before C2 |
| G7 — Truthful capability claims | **fail** | The any-library path and F5's `.py`/`.pyi` support fail on attrs 26.1.0 (F2). "An id computed in Rust equals the one computed in SQL" is false (F3). "Node-valued references generated … closing O8" is false (F4). "A declared pending class" does not exist (F5). ADDENDUM §5 names an oracle that does not exist (F6). §8 lists a removed rule (O3). Metric wording (O4) | narrow or implement each claim |

## 7. Principle findings

Ordered by severity: correctness and validity first, then identity and capability, then claims and
extension locality.

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| F1 | Target-resolution failures on our side publish as `missing_evidence`. The `typed:*` rules, and the "explained" half of the lineage rules, cannot fail on derived data. Slice-2 F1's rejection was removed without a working replacement. | DM-07, DM-08, DM-60 · G3, G2 | **The catch-alls:** `derived.rs:526-535` (`function_target_reason` ends `ELSE {missing}`), `derived.rs:617-620` (ancestry, `ELSE {missing}`) and `derived.rs:188-192` (exports, `ELSE {variable}`). No path yields a null node with a null reason. **The rules:** `graph.rs:759-779` (`… IS NULL AND reason IS NULL`); lineage "explained" = any row `WHERE reason IS NOT NULL` (`graph.rs:436-439`, `470`, `510`, `576`, `352`). **The removal:** `semantic:release-target-explained` (`rules.rs` at `494557c`, L139-141) and its `@nowhere.py` mutation were removed; the mutation case now shifts spans for `lineage:call_target`. `cpg-core/tests/graph.rs:480-486` tests `typed:` only on a doctored table. **The existence source:** `context.rs:246-248` keeps only definitions the release references, and nothing counts a referenced key left undescribed. `tables.rs:193` says "independent of the columns that reference them". **Replay (P4)**, the published `call_targets` SQL over pilot `pysa_calls`: (a) every `@` target moved to `@nowhere.py` gives 4,052 null targets, all `missing_evidence`, 0 `typed:` violations; (b) every dependency key perturbed gives 13,122 of 17,174 null, all `missing_evidence`, 0 violations | A regression in `context.rs`'s filter, or a Pyrefly bump that desynchronizes key serialization between the call graph and the definitions, publishes most of the pilot's external call edges as "the provider had no evidence". Every rule passes. Pass A and the briefs (§3.7: "read these rows instead of treating 'the analysis stopped' as absence") then state a limit that is really our bug. The 4.4 s / 0.7 GB second check buys an existence confirmation whose negative result nothing acts on | Make "names something inside the analyzed universe but unresolved" distinguishable and rejectable. (i) In `context.rs`, error, or emit a boundary, when a referenced `(module, kind, key)` is not described; P1 and the pilot say that is zero today. (ii) In the derivations, a release-file key Stage C lacks gets `provider_disagreement`, and `missing_evidence` is reserved for "no Pysa record". (iii) A generated rule per typed-target table: a target whose module is in `source_files` ∪ `context_modules` has a node or a Stage-C reason. Surface: three `CASE` edits, about 10 lines in `context.rs`, one rule generator. `variable_origin` stays a declared catch-all until C3 (§10) | **test:** restore the `@nowhere.py` mutation and add a dependency-key perturbation in `every_rule_kind_rejects_its_violation`. Both must fail validation; today both pass (P4) |
| F2 | `exports` edges have no discriminator. A `.py`/`.pyi` public-module pair that re-exports one origin gives two edges with one `edge_id`, so **attrs 26.1.0 cannot publish** | DM-15, DM-54, DM-59 · G7 | **The registry:** `graph.rs:328-356` (`exports`: `parallel: false`, discriminator `None`), and `edge_id` = `lctx_id('edge', kind, src, dst, ordinal, disc)` (`graph.rs:677`). **The node:** export node = `H(export, release, access path)` (`derived.rs:183`); one `exports` row per `public_names` row (F5 of slice 2). `public_names` has no public-file column, so a pair yields two rows only when payloads differ. **P8:** `lctx compile attrs` (26.1.0, scratch store) failed with `key:edges (14 rows), no-parallel:exports (14 rows)`, nothing published. The 14 are `attrs.Attribute`, `attrs.assoc`, …, `attr.cmp_using`. Each has a `.py` row (`__all__`, `via_dunder_all = true`) and a `.pyi` row (`X as X`, `false`) tracing to one `attr` declaration. **Coverage:** graph_shapes' only pair (`typed.py`/`.pyi`) defines its own declarations, so it cannot reach this shape. In the pilot environment, `attr`, `attrs`, `cachetools`, `more_itertools`, `cyclopts` and `jaraco.functools` ship `__init__.py` beside `__init__.pyi`. DESIGN §4.0 records attrs 25.3.0 publishing before C1 | Any library with a stub beside a package `__init__` that re-exports a submodule's names cannot compile, against ADR-0013's "one production path for any Python library". Whether it compiles hangs on an incidental field: identical payloads merge by `fact_id`, differing ones collide | Give `exports` a discriminator and `parallel: true`. Preferably add the public file's `module_node_id` to `public_names` (which also makes "one row per file" true) and discriminate on it; otherwise use the row's run-independent payload digest, as `pysa_calls.payload_id` does. One column, one registry line, a schema migration | **test:** a graph_shapes package with `__init__.py` (`__all__`) and `__init__.pyi` (`from .impl import f as f`) re-exporting from a `.py`-only submodule and from `depmod`. It must publish with two export edges. Optional: a second library in `libraries/` compiled by `just test-all` (operator's call) |
| F3 | One id notation, two encodings. The extractor's C1 role ids use the plain encoding; `lctx_id` uses `opt_*`. DESIGN and the UDF both say they agree, and neither side's C1 recipes are pinned | DM-15, DM-51, DM-59 · G7 (latent G1/G6) | **Rust (plain):** `walk.rs:383-386` `IdHasher::new(kind::ARGUMENT).id(node_id).i64(ordinal)`; `context.rs:209-213` and `251-255`. **SQL (opt_*):** `udf.rs:68-98`. **The claims:** `udf.rs:1-4` "an id computed in SQL equals the same recipe computed in Rust"; DESIGN §3.4.1 L523-524. **P3:** `lctx_id('argument', call_node_id, ordinal) = node_id` for 0 of 19,493 arguments; the external-module recipe for 0 of 329; SQL-computed `synthetic_callable` for 539 of 539. **Pinning:** `udf.rs:173-202` checks the UDF only against `opt_*` callers in the same build. `extractor_id_recipes_snapshot` has no argument or external rows. `ids.rs` pins only the plain encoding: no hex known answer pins `opt_*`, and no SQL-computed id (edge, export, synthetic) is pinned. `udf::VERSION` is bumped by hand | A later pass that computes a role id on the other side gets ids that name no node. Examples: C2 SQL deriving `lctx_id('argument', …)`, C3's `reference = H(reference, syntax id)`, or Pass A in Rust computing `H(export, release, "fastmcp.FastMCP.tool")` to find its seed. Through F1 those misses become reasons, not failures. Separately, a silent edit to either encoding re-keys every id and no oracle fails | One function per derived-id kind in `cpg_schema::id`, in one encoding, called by the extractor. A test asserts each equals `lctx_id` on the same inputs. Add the C1 kinds, plus one edge id and one export id, as hex known answers. Fix the DESIGN sentence to say which encoding each recipe uses | **test:** per-kind Rust == UDF (fails today for argument, external_module and external_symbol); hex known answers in the ids snapshot |
| F4 | Node-valued references are not generated from the registry, so slice-2 O8 is **not** closed. `REFERENCES` is hand-written and already allows kinds the registry forbids | DM-02, DM-52, DM-59 · G7 | ADR L124 ("Generated from it: … the node-valued references (closing slice-2 review O8)"); DESIGN §8 L1323-1325 ("so the references and the mapping are one authority"). `graph::rules()` (`graph.rs:690-781`) generates no `ref:` rule. `rules.rs:52-146` is hand-written, and C1 hand-added 20 entries. `CALLABLE` (`rules.rs:36-40`) admits any declaration and any context definition, classes included, while the registry's `call_target`/`overrides` targets are `Function \| SyntheticCallable \| ExternalSymbol` (`graph.rs:206`) | Each new target kind (C3's `binding` for variable exports, DESIGN §3.2 L425) is edited in two lists. A row with a null source never reaches the endpoint rule, so there the looser `ref:` list is the only check. Mostly extension friction and an untrue closure claim | Generate the node-valued `REFERENCES` from a column → kinds map declared once beside the registry, or narrow the ADR and DESIGN sentences and keep O8 deferred with its trigger | **test:** extend `references_name_real_columns` so every registry endpoint column has a `ref:` whose targets are exactly its kinds' existence sources |
| F5 | The lineage rule's "declared pending class" and the synthetic-callable stop are declared nowhere a consumer can read. 17,282 of 35,057 pilot Pysa call rows (49%) have no graph representation, and 112 synthetic callables end 279 call edges with no completeness marker | DM-08, DM-43, DM-59 · G7 (narrow); guidelines §2, §7 | **The filter:** `expected` = `regular_calls() AND higher_order_index IS NULL` (`graph.rs:432-434`); `Lineage` (`graph.rs:56-61`) has no pending field. **The claim:** DESIGN §3.2 L433, §3.8 L717-718 and §8 L1329-1330 say "a declared pending class". **P5:** artificial calls 7,999; identifiers 5,755; stringify 1,942; attribute access at regular sites 1,502 (property getters and setters, which §4.2.3 maps to targets with phase `property_get`/`property_set`); artificial attribute 84. None is counted. **Other silent exclusions:** cyclic-MRO rows (`graph.rs:572`, `derived.rs:630`; 0 on the pilot); unresolved higher-order rows inner-joined to `arguments` (`derived.rs:486-487`; 0 lost on the pilot). **P7:** 539 synthetic callables with 0 outgoing edges; §3.7 L645-651 lists external nodes, not synthetic ones, as where the graph stops; FastMCP has 3 `__post_init__` defs a synthesized `__init__` runs | A projection over `edges` (§5's invocation projection, Pass A) misses delegation through property getters, and ends a dataclass constructor's `__post_init__` path, with nothing in data or in §3.7's list to say the graph stopped there. Guidelines §7's "partial is not complete" rests on prose | Add `pending: [(name, SQL, resolving slice)]` to `Lineage`. Generate a partition rule: every evidence-table row is in exactly one of expected or pending. Print pending counts in `lctx compile`. Add synthetic callables to §3.7's list | **test:** the generated partition rule, plus graph_shapes asserting pending counts for an `@property` read and a `with` statement |
| F6 | The registry's `derivation` class and `direction` meaning have no consumer or test, and are outside `compiler_digest`. ADDENDUM §5's oracle for them does not exist | DM-31, DM-51, DM-60 · — | `graph.rs:68-72` declares the fields, and nothing reads them. `compiler_digest_of` (`attempt.rs`) hashes SQL, contracts and rules, none of which contains them, and `edges` has no derivation column. ADDENDUM L129 names `semantic:call-target-typed`, which is not a rule (the rule is `typed:call_targets`), and "the codebook snapshot", which lists names only | DESIGN §3.2 says projections select "by kind, derivation class and evidence". A projection built later reads the class from its own build, so an edit (say `stub_for` from `joined` to `analyzer`) re-selects old snapshots with nothing recording it. Low today: there is no consumer | An insta snapshot of the registry (kind, endpoint kinds, direction, parallel, derivation, evidence table, lineage presence) whose text feeds `compiler_digest`. Fix ADDENDUM's oracle names. Or label the two fields Proposed until the projection spec lands | **test:** the registry snapshot |

**Observations.** Each is low severity and has an oracle, or says it has none.

- **O1. `stub_for` re-expresses the `exports` seed rank.** `derived.rs:167-172` and
  `graph.rs:276-289` repeat `ORDER BY d.is_overload, k.node_id IS NULL, d.start_byte DESC,
  d.node_id` under different partitions. §3.8 defines `stub_for`'s target as "the declaration the
  exports seed rank picks", so an edit to one diverges silently.
  - Correction: one shared CTE builder.
  - Oracle: test (on the keys fixture, `stub_for` dst equals the `.py` `exports` seed).
- **O2. `nodes` merges same-kind id collisions.** `graph.rs:624-634` deduplicates per
  `(node_id, node_kind)` for *every* kind. §3.4.1 L528 says "Collisions are validator failures,
  never silently merged". The source keys include `fact_id` or spans, so a duplicated id within
  one kind is absorbed. It is caught only when an incoming extracted edge also collides.
  - Correction: restrict the dedupe to `export`.
  - Oracle: test (a duplicated `arguments.node_id` must fail).
- **O3. Stale spine.** DESIGN §8 L1318 still lists "a release call target names a declaration or
  gives a reason", which C1 removed. ADDENDUM L129 names a nonexistent oracle.
  - Oracle: prose.
- **O4. Metric wording and cost attribution.**
  - §4.3 L1099-1101 says "extraction per family"; the code reports a per-module total plus the walk
    and Pysa sub-stages. It says "peak RSS per stage"; the value is `VmHWM` so far, which only
    grows.
  - §3.8 L731-733 attributes the +5.7 s to the dependency check and definitions, which account
    for 4.4 s; validation is 1.21 s (DM-39).
  - Oracle: prose.
- **O5. `context_modules` provenance.** Its facts carry surface `pyrefly-pysa`, but `distribution`
  and `version` are the extractor's join against Stage A's `RECORD` index (`context.rs:169-181`)
  (DM-46).
  - Oracle: none mechanical; a `model_id` test if it matters.
- **O6. Call edge ids are producer-scoped.** Call and higher-order `edge_id`s depend on Pysa keys
  through `payload_id`. §3.4.1 says "stable across snapshots and runs" without the producer scope
  it gives external symbols. This matters for guidelines §10's `path_steps(edge_id)` across a
  Pyrefly bump.
  - Oracle: prose.
- **O7. Process.** ADR-0004's amendment records a new sequencing decision, while
  `scripts/adr.py:47` reserves amendments for factual corrections. ADR-0014 records the same
  decision, so nothing is lost.
  - Oracle: none; `adr lint` checks structure only.

**Strengths, stated as what would break without them.**
- The registry generates 67 of the 247 rules (13 endpoint, 13 evidence, 13 one-per-evidence,
  11 no-parallel, 12 lineage, 1 support, 4 typed) from 13 declarations. Without it, each C2–C5
  edge kind needs five hand-written queries.
- The raw-row lineage catches a Pysa record whose span matches no call site: the rewritten
  mutation, a real failure mode the span join could otherwise hide.
- `context.rs:133-138` refuses a dependency handle that is not the one Pysa numbered. Without it,
  a same-named module elsewhere on the path would be described in place of the referenced one.
- The retention change keeps §6.2's pinned reads valid past 30 days.

**Applicability.**
- Bore on this scope:
  - Group 1: the catalogs vs the family authority;
  - Group 2: the rules and absence (F1, F5);
  - Group 3: edge and role identity (F2, F3, O2);
  - Group 5: derivation contracts;
  - Group 7: DM-31 and DM-34;
  - Group 10: lineage;
  - Group 11: migrations and pinning (F3, F6);
  - Group 12: claims and proportionality.
- Bore lightly:
  - Group 4: the registry is a declaration; DM-16/DM-17 are satisfied;
  - Group 6: retention, and publication unchanged;
  - Group 8: DM-39 attribution only;
  - Group 9: `context.rs` as an adapter. The typed pairs replace string parsing, so DM-41/DM-42 are
    aligned.
- Did not bear: DM-35 (no new concurrency) and DM-45 (no new trust boundary; dependency files are
  `RECORD`-verified by Stage A).

**Principle verdicts** (applicable ones only):

| Verdict | Principles |
|---|---|
| Satisfied | DM-11 (location-independent ids, Tested); DM-23 (catalogs rebuildable, P2); DM-34 (edge kinds distinct with directions); DM-46 (an evidence fact per edge; the readiness reader checks call-edge lineage); DM-16 and DM-52 for SQL and rules |
| Violated | DM-07 (F1); DM-08 (F1, F5); DM-15 (F2, F3); DM-51 (F3's unpinned recipes); DM-54 (F2's shape untested; F1's negative test removed); DM-59 (F3–F5 claims, O4); DM-60 (F1 removed a regression control); DM-52 for references (F4, SHOULD) |
| Unresolved | DM-31 (F6: no consumer yet decides whether the derivation class is result-affecting) |

**Guidelines conformance (ADDENDUM §5), as found.**

| Guideline MUST | As built |
|---|---|
| §2: persistent `edge_id`, typed endpoints, relation kind, evidence, snapshot scope | **met**, except exports identity (F2) |
| §2: isolates | **met** (Tested) |
| §2: distinguishable derivations; unknown targets explicit | **partly**: the derivation class is unpublished (F6); our lookup failures are indistinguishable (F1); the omission of pending rows is not in data (F5) |
| §3: projection spec | consumer; the CPG's direction meanings are Rust strings only (F6) |
| §4: indices never identities | **met** |
| §7: no closures; partial ≠ complete | **met** for no closures; **partly** for partial ≠ complete (F5) |
| §10 | consumer |
| §11: Delta pinned reads; manifest after validation; retention | **met** (Tested) |
| §12: known-answer shapes; lineage; per-stage instrumentation | **met** for the listed shapes. Lineage is checked for call edges. Instrumentation is met, with O4's wording |

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Implementation / maintenance cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| Baseline: ADR-0008 plus an `edge_id` column per relationship table (ADR Option 1) | Each consumer re-unions the relationship tables; endpoint and lineage rules hand-written per table; no node relation | Isolates and endpoint kinds implicit; external targets null/null | Lowest now; grows per edge kind | none | Rejected. It fails guidelines §2's node-relation MUST |
| Proposed: materialized `nodes`/`edges` from one registry, plus dependency definitions | One declaration generates the SQL and 67 rules; `REFERENCES` (F4) and pending classes (F5) still hand-kept | F1, F2 and F3 as found; otherwise sound, and every edge is rebuildable (P2) | ~780 lines of registry; a UDF; a second Pyrefly pass | Measured: catalogs 0.13 s; dependency check and definitions 4.4 s and +0.7 GB of the +5.7 s / +0.92 GB | Selected, with F1–F3 corrected |
| **Simpler viable:** (a) `nodes`/`edges` as read-time views generated from the registry, not Delta tables; (b) no second Pyrefly check, with `external_symbol` nodes derived from the distinct `(target_module, target_key, target_name)` of the referencing rows | (a) is the same declaration with two fewer tables. (b) removes `context.rs` (293 lines) and two raw tables | (a) couples every reader to its own build's registry: a reader built after a registry edit sees a different catalog for an old snapshot (DM-14, DM-23). Materializing makes the catalog snapshot-coherent, which is worth 0.13 s. (b) makes the external existence source *built from the referencing columns*, the vacuous endpoint the ADR rejects, and loses definition kind and qualified names | (a) −2 tables. (b) −4.4 s, −0.7 GB, −1 raw family pair | (b) would recover 77% of C1's added wall time | (a) rejected: coherence wins. **(b) is the headline:** today the chosen design pays 4.4 s for an independent existence check whose failure no rule acts on (F1). The check pays for itself only once F1 makes "referenced but not described" fail. Without that, (b) gives the same guarantee for free |

**Abstractions justified by current needs.** The registry is justified: its consumers are the SQL
and rule generators, and the rules snapshot shows the leverage. The UDF is justified: Stage D needs
ids in SQL. Retention is justified, because it protects the publication contract. The registry's
`derivation` and `direction` fields have no consumer yet (F6). They are cheap, but they are claimed
as meeting a guideline MUST.

**What remains ordinary code.** The Pysa mapping, the dependency collector and the per-kind
existence SQL stay hand-written behind the registry contract. That placement is right (charter §F).

**Proportionality of "CPG first."** The operator's decision: C2–C5 move ahead of their consumers,
each naming its consumer. This review does not second-guess it. It notes that C1 already carries two
unconsumed registry fields (F6), and that each later slice should keep the ADR's "every family names
its consumer, pass and columns" test.

## 9. Verification and measurement plan

| Claim or risk | Evidence label | Test / analysis / benchmark | Conditions and expected result | Current result or remaining gap |
|---|---|---|---|---|
| Catalogs hold isolates, parallel sites, self-loops, a cross-file SCC and reconvergence | **Tested** | `graph.rs` (3 tests), insta snapshot | graph_shapes | passed (nextest 70/70) |
| Catalogs stable across location and module order | **Tested** | `the_catalogs_are_the_same_across_runs_order_and_location` | two paths, reversed order | passed |
| Every edge recomputes from its row | **Tested** (probe P2, 2026-09-23) | `lctx query` | pilot snapshot `d45f…` | 65,176 of 65,176 |
| Every registry rule rejects an injected violation | **Tested** for the rule text; **not established** as a guard for `typed:*` and lineage's explained half | `compile.rs`, `graph.rs` | raw and doctored mutations | F1: add the `@nowhere.py` and dependency-key mutations; both pass today by replay |
| `.py`/`.pyi` public pairs compile | **Tested (fails)** | P8 | attrs 26.1.0 | failed: `key:edges` 14, `no-parallel:exports` 14 (F2). Add the fixture |
| Rust and SQL ids agree | **Tested (false)** | P3 | pilot | 0 of 19,493 and 0 of 329 (F3). Add per-kind equality and hex known answers |
| Pending Pysa rows accounted | **Proposed** (claimed as declared) | P5 | pilot | 17,282 of 35,057 excluded, uncounted (F5). Generated partition rule |
| Retention keeps old versions loadable | **Tested** | `retention_keeps_old_versions_loadable` | checkpoint interval 2, zero retention | passed. `interval 36500 days` parsing is untested on its own |
| `lctx_id` refuses lossy types | **Tested** | `unsupported_types_are_refused_at_plan_time` | Int32, UInt64, floats | passed |
| Pilot counts (§3.8) | **Measured** (re-queried 2026-09-23) | P1 | `d45f…` | match: 47,145 nodes, 65,176 edges, 0 reasons on call, ancestry and override targets; 335 `variable_origin` (P6: 334 release variables, 1 dependency `TypeAlias`, none with a same-name declaration) |
| Compile time and RSS (§3.8, §4.0) | **Measured** by the author (`build/pilot.txt`) | `just pilot` | release build | not re-run (`not_run`). Attribution is partial (O4) |
| One `content_digest` across two runs | asserted | `just pilot` twice | — | not re-run |

**Cost accounting.**
- The material new costs are the dependency check (3.7 s, +0.7 GB peak) and validation
  (1.21 s at 247 rules).
- The catalogs themselves are 0.13 s and about 112k rows.
- Nothing here justifies streaming derive yet, and §4.3's deferral trigger (C6 RSS) is right.

## 10. Exceptions and unresolved decisions

**Proposed scoped exception: the `exports` catch-all reason until C3.**

- **Principle IDs:** DM-08.
- **Scope:** `exports.reason = variable_origin` as the `ELSE` branch (`derived.rs:188-192`).
- **Reason and alternatives:** until C3's bindings exist, nothing distinguishes "the origin is a
  variable" from "the origin is a `def` we failed to match". The alternative, failing the run, would
  reject every library with public variables.
- **Consequence and compensating controls:**
  - A declaration-matching failure would read as a variable.
  - P6 found none on the pilot: 334 release origins, 0 with a same-name declaration; 10 sampled,
    all variables, including `fastmcp.settings = Settings()`.
  - Compensating control: a rule that no `variable_origin` row has a declaration with that
    qualified name in its origin module. That query is P6's.
- **Evidence:** P6.
- **Owner:** the C3 slice.
- **Revisit trigger:** C3 lands `bindings`.

**Decisions for the author:**
1. F1: fail the run, or emit a distinct reason, when the analyzer names something inside the
   analyzed universe that our tables lack. The recommendation is to fail in `context.rs` and add a
   rule.
2. F2: which discriminator: the public file (a column on `public_names`), or a payload digest.
3. F3: which single encoding role ids use: `opt_*` everywhere, or plain with the UDF matching.
4. F4: generate the references, or narrow the claim and keep O8 deferred.

## 11. Decision and implementation changes

**Decision: Revise.** Do not accept ADR-0014 yet.

**Reason.**
- **What is sound.** The architecture is sound, and aligned with the charter and the operator's
  guidelines:
  - derived catalogs with no payload, generated from one registry;
  - content-derived edge ids with evidence;
  - typed external endpoints;
  - retention.
- **What fails.**
  - **G3:** removing slice-2's rejection, and relying on `typed:*` rules the derivations cannot
    violate, lets an internal resolution failure publish as provider-missing evidence. This is
    ADR-0014's own revisit trigger, and P4 demonstrates it.
  - **G7:** a legitimate and common library shape cannot publish (F2), and four claims are
    untrue (F3–F6).
- **The corrections are small and local:**
  - F1: three `CASE` edits, about 10 lines in `context.rs`, and one rule generator;
  - F2: one column and one registry line;
  - F3: one id module and one test;
  - F4–F6: a generator, a partition rule and a snapshot, or narrowed sentences.
- **Acceptance.** Accept when F1 and F2 are fixed with their tests, and F3–F5 are fixed or their
  sentences narrowed. The C1 compact slice review is satisfied on the same terms.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: distinguish and reject "names something in the universe we lack". Assert in `context.rs`; give `provider_disagreement` for release keys; a generated resolved-target rule | DM-07, DM-08, DM-60 | P4's two mutations fail validation | test: the restored `@nowhere.py` case plus a dependency-key case in `every_rule_kind_rejects_its_violation` |
| 2 | F2: a discriminator for `exports` (the public file on `public_names`), `parallel: true` | DM-15, DM-54 | attrs 26.1.0 publishes; the new fixture pair gives two export edges | test: the graph_shapes `__init__.py` + `__init__.pyi` re-export case (a schema migration: snapshot diff) |
| 3 | F3: one role-id module, one encoding; Rust == UDF per kind; hex known answers for C1 kinds and for an edge and export id | DM-15, DM-51 | per-kind equality passes | test: the ids snapshot plus the equality test |
| 4 | F5: `pending` on `Lineage`, a partition rule, counts in `lctx compile`; synthetic callables added to §3.7 | DM-08, DM-43 | the partition rule passes on the pilot, with counts printed | test: the partition rule plus graph_shapes pending counts |
| 5 | F4: generate the node-valued references from the registry, or narrow the ADR and §8 and keep O8 deferred | DM-02, DM-52 | the `ref:` set equals the registry's kinds | test: `references_name_real_columns` extended |
| 6 | F6: a registry snapshot fed into `compiler_digest`; fix ADDENDUM §5's oracle names | DM-31, DM-51 | the snapshot exists; the digest moves when a derivation class changes | test: the registry snapshot plus a `compiler_digest` unit case |
| 7 | O1–O4: the shared seed-rank CTE; dedupe only exports; DESIGN §8 and §4.3 wording | DM-02, DM-39 | — | tests as named in §7; prose |

**Deferred** (not acted on here, each with the trigger that reopens it):

| Item | Why deferred | Reopen when |
|---|---|---|
| O5: `context_modules` owner attribution under `pyrefly-pysa` | no consumer reads `model_id` per column | a brief cites a module's distribution |
| O6: producer-scoped call edge ids | stage 1 does not compare across Pyrefly revisions (§1.3) | a persisted artifact keys on `edge_id` (guidelines §10 `path_steps`) |
| O7: decision content in an ADR amendment | ADR-0014 records the decision | the next `adr.py` change |
| `higher_order_index` as the argument ordinal in general | all 394 pilot rows map | a starred or keyword-first higher-order call appears in a fixture or library |
| Retention's duration parsing on its own | cleanup is off, so the duration is a second guard | a table ever turns cleanup on |

**Final check.**
- The claims do not yet match the evidence in five places (F1–F5).
- The supported scope does not yet include the `.py`/`.pyi` re-export shape (F2).
- For C2 onward the extension path is clear and largely generated, once F3 fixes the id boundary
  and F4 and F5 bring the references and the pending classes into the registry.

## Disposition (author, 2026-09-23)

Every finding was fixed or narrowed below. ADR-0014 was amended in place and **accepted** in the
same commit, with evidence Tested.

| # | Outcome | What changed | Oracle |
|---|---|---|---|
| F1 | fixed | No derivation has a catch-all reason. `call_targets`, `ancestry_targets` and `override_targets` keep only Stage C's reason; any other unmapped target has a null reason. An export's reason comes from Pyrefly's own symbol kind of the origin (new `public_names.origin_symbol_kind`, an exhaustive match onto the appended `symbol_kind` codebook): `variable_origin` for variable-like kinds, `missing_evidence` when there is no origin or kind, null otherwise | test: the restored `@nowhere.py` mutation and a new dependency-key mutation both fail `typed:call_targets` (`compile.rs`). The pilot publishes with every target typed, and its 335 `variable_origin` exports are all Pyrefly `Variable`s |
| F2 | fixed | `public_names.access_module_node_id`. `exports` edges take it as the discriminator and allow parallel edges (one per access file). `exports.target_node_id` is typed and can also name a module | test: the `shapes/dual` `.py`/`.pyi` pair gives two export edges (`graph.rs`). `attrs` 26.1.0 publishes (4,061 nodes, 5,184 edges) |
| F3 | fixed | `cpg_schema::id::recipe` (argument, external module, external symbol) in the `opt_*` encoding, used by the extractor; generated `id:arguments`, `id:context_definitions` and `id:context_modules` rules recompute each in SQL | test: `every_rust_recipe_equals_its_sql_form`, with pinned hex values. The rules pass on the pilot |
| F4 | fixed | `graph::node_columns` declares every node-valued column with its allowed kinds; `ref:<table>.<column>->nodes` rules are generated from it, and `REFERENCES` keeps only fact, provenance and composite references. Slice-2 O8 is closed | the rules snapshot; the existing `ref:call_syntax.owner_node_id` mutation now fails the generated rule |
| F5 | fixed | a `graph_gaps` table publishes each raw row the graph does not represent yet, with reason `not_requested` and the slice that will; `partition:pysa_calls-gaps` and `partition:pysa_calls-remainders` put every Pysa row in one place. §3.7 lists synthetic callables as graph ends | the rules pass on the pilot, where the gaps are 17,282 rows: 8,083 artificial, 1,502 attribute, 1,942 format-string, 5,755 identifier |
| F6 | fixed | `derivation_class` is a codebook, and an `edge_kinds` table publishes each kind's derivation, direction, parallel policy, evidence table and endpoint kinds with every snapshot. Its SQL is in the derivations snapshot, so `compiler_digest` covers it. ADDENDUM §5 names real oracles | the derivations and contracts snapshots |
| O1 | fixed | `derived::seed_rank` and `KEYED` are shared by `exports` and `stub_for` | the derivations snapshot |
| O2 | fixed | `nodes` merges one id's rows only for export, external module and external symbol | test: a duplicated argument row fails `key:nodes` (`compile.rs`) |
| O3 | fixed | DESIGN §8 and ADDENDUM updated | prose |
| O4 | fixed | DESIGN §4.3 describes the stages as reported and "peak RSS so far"; §3.8 re-attributes the cost | prose |
| O5 | fixed | `context_modules` facts are surface `compare`, `relational_derivation` | the `facts` rows |
| O6 | fixed | DESIGN §3.4.1 states the producer scope of call edge ids | prose |
| O7 | accepted as is | ADR-0014 records the sequencing decision; the ADR-0004 amendment points to it | none |

**The simpler alternative (§8).** It was rejected. With F1 fixed, the dependency check is what
makes an `external_symbol` target falsifiable: a definition Pysa references but Pyrefly's own
collectors do not describe now fails `typed:call_targets`.

**Checks** (2026-09-23):

| Command | Outcome |
|---|---|
| `just check` | passed (nextest 71/71, pytest, rules, adr lint, lint-agents) |
| `just test-all` | see the commit |
| `lctx compile fastmcp` (release) | passed: 47,145 nodes, 65,176 edges, 261 rules, 14.4 s, 2.49 GB |
| `lctx compile attrs` (26.1.0, scratch store) | passed |
