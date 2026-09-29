# Plan: the semantic model cutover

**Status: Active, 2026-09-29. Execution owner** for [ADR-0082](../adr/0082-relation-centric-semantic-model.md)
(relation-centric model), [ADR-0083](../adr/0083-postgresql-relational-store.md) (PostgreSQL single
store) and [ADR-0084](../adr/0084-layered-hard-cutover.md) (hard layered cutover).

**Target.** The target is [§15](../design/sections/semantic-model.md). This plan owns the order, the
work packages, the adapters, parity, the deletion obligations, and the disposition of findings F01–F13
from the [semantic data model review](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md).

**Relation to the forward plan.** The [forward plan](behavioral-model-forward-plan_2026-09-24.md)
keeps the product context and every other finding. Its queue points here until phase 5 exits.
**Product work (PR6 and new features) is paused until then.**

## 1. Outcome and completion

**The end state** is §15, fully implemented:
- `lctx-model` declarations are the single authority;
- PostgreSQL 18 generations are the only store;
- DataFusion computes at intake;
- every semantic question has one owner;
- serving reads generated views over the pinned generation.

**The cutover is complete when all of the following hold.** Each is checked mechanically where
possible, by `rg` over `crates/` and `python/`:
- no `deltalake` dependency, and no `delta`/`snapshot` store code;
- no `cpg-schema` crate;
- no `for_each_table!`/`Derived`/`EdgeSource` legacy registries;
- no legacy adapter, declared quirk or legacy-ID relation;
- no hand-written serving file, inventory or foreign-key list;
- no semantic question answered outside its §15 owner:
  - the call edge, owner and binding;
  - transfer composition;
  - verdict, obligation priority and discharge validity;
  - atom identity and conditions.

**Working rules**
- **Order.** Phases run strictly in order. Within a phase, work packages may overlap.
- **Code standard.** Nothing is blended. New code is written to the target contracts. Legacy code is
  only ever deleted or bridged by a declared adapter; it is never extended.
- **Contracts.** The core contracts (phase 0) evolve freely until the layer that produces and consumes
  them cuts over. They are frozen at that phase's exit.
- **Performance.** Publication and read performance is recorded at each phase exit and tuned later.
  It is never a phase gate.
- **No rollback.** A failing phase is fixed forward. Git holds the legacy code.

## 2. Baseline (`fedd4a0`, 2026-09-29)

**Code.**
- 8 Rust crates (about 105k lines) and 3 Python packages.
- 485 Rust tests and 204 Python tests.
- 201 `table!` contracts: 57 raw, 22 derived, 121 analysis, plus `snapshots`.
- 72 serving files (18 derived, 49 hand-written, plus retrieval), 29 native IPC files, 12 PostgreSQL
  migrations.
- 10 MCP tools and 1 resource.

**Pilot (behavioral profile).** Measured in P8 and in the PR5 qualification log:

| Measure | Value |
|---|---|
| Rows | 7,737,960 |
| Canonical bytes | 321.5 MiB |
| Compile time | 473.9 s |
| Validation time | 177.2 s |
| Summary flows | 23 |
| Discharges proved | 0 of 2,107 |

The duplicated semantic decisions and served-fidelity findings are recorded in review §2 and §7.

## 3. Global mechanisms, built in phase 0 and used throughout

### 3.1 `lctx-model` (new crate)

**Purity.** Pure, with no DataFusion, SQLx or I/O. Dependencies: `arrow-*`, `blake3`,
`biodivine-lib-bdd`, `serde`/`serde_json`, `schemars`.

| Module | Owns |
|---|---|
| `decl` | `relation!` and `codebook!`, the `RelationId` registry, generated Rust/Arrow types, and a declaration metadata API (layer, roles, coverage scope, polarity, fidelity, serving exposure) |
| `id` | The `IdKind` enumeration, `IdHasher` v2 (which requires an `IdKind`), recipes, and known-answer vectors |
| `vocab` | Entity and occurrence kinds; `Place`, `PlaceRoot`, `AccessPath` (k ≤ 2, unknown suffix); the owner rule; the innermost-region join rule |
| `calls` | The five call policies; the binder (a pure function over call syntax, target alternative and signature variant); the effective-callable contract |
| `transfer` | Kinds, the composition table, provenance classes, and the call-site composition operator |
| `condition` | The ported bounded kernel and primitive theory; atom identity v2; rendering from capped satisfying paths |
| `obligation` | The obligation codebook (a union of the legacy boundary and refusal reasons, with a legacy mapping), priority, named budgets, the verdict function and the discharge-validity function |
| `derivation` | The derivation-source registry and generated `derivations`/`derivation_premises` view text |
| `projection` | Projection declarations: universe, arc sources from policies, direction, parallel and unresolved policies |
| `stage` | The stage table, scheduler, writer-uniqueness and read-before-write checks, and code identity |
| `ddl` | Generated PostgreSQL DDL, constraints, codebook tables, indexes, grants and view text |
| `legacy` (temporary) | Adapter declarations, declared quirks and divergences, legacy-ID relation declarations |

### 3.2 Store kernel (`lctx-postgres`)

**Layout.**
- Schema `lctx` holds one list-partitioned parent per canonical relation. The partition key is
  `generation_id`; legacy relations keep their `snapshot_id` column name until re-declared (§4.1 D6).
- Codebook tables live alongside them.
- A `generations` registry records state, profile, `ddl_digest`, compiler, producer and content
  digests, timestamps and receipts. `generation_relations` holds per-relation receipts,
  `generation_events` holds transitions and stage receipts, and `relations` is the installed
  relation catalog.
- Each generation's partitions live in their own schema, `lctx_g<32hex>`.
- **Roles.** The abstract roles are owner, writer (staging only) and reader (published only). They
  are the existing `lctx_migrator`, `lctx_importer` and `lctx_serving` roles (§4.1 D5).
- **DDL.** The canonical DDL is generated from the declarations and installed by an explicit
  `lctx store install`, which records its digest. A contract change is an explicit `lctx store reset`
  (§4.1 D3).

**Lifecycle API:** `create_generation` (owner-created staging tables with partition CHECKs) →
`copy_batches` (binary COPY via pgpq, straight into staging) → `mark_validated` (indexes built;
DataFusion validators passed) or `fail_generation` → `publish_generation` (one SECURITY DEFINER
transaction that attaches every partition and revokes writer access) → `select_generation` →
`retire_generation` (`DETACH … CONCURRENTLY`, then drop the schema).

**How constraints are checked.**
- COPY enforces NOT NULL and CHECKs.
- Index builds before `mark_validated` enforce keys.
- ATTACH validates references.

Loading unlogged, deferring validation and tuning index builds are later performance options.

**Reading from DataFusion.** A DataFusion `SessionContext` is registered over a pinned generation
through the owned provider fork. The canonical read mode lives in `cpg-core::store_read` (§4.1 D14).
`lctx query --generation` uses it. A DataFusion memory pool is configured for every session, which
also bounds recursive CTEs (review F07).

**Tests** run on testcontainers PostgreSQL 18. They cover:
- the lifecycle;
- an aborted attempt staying invisible;
- a retry creating a new generation;
- injected violations of keys, references, codebooks and CHECKs being refused;
- writes to published partitions being denied;
- a pinned reader being unaffected by a later publication.

### 3.3 Stage table and orchestrator

**Declarations.**
- Every stage declares:
  - its input relations and output relations;
  - its **transients**: in-memory handoffs that are never published;
  - its declared **context** keys: digested non-relation attempt inputs;
  - its effect class (pure, extraction, store, embedding). Embedding stages share one embedding
    capability, and a single receipt stage writes the consumed-vector relations;
  - the profiles it runs in;
  - its code identity.
- Legacy stages are declared the same way during the migration, marked `legacy`.

**What is derived and checked.**
- `attempt::finish` is replaced by a scheduler derived from the table: a pure Kahn order with a
  deterministic tie-break.
- The scheduler refuses:
  - a published relation without exactly one writer per profile;
  - a transient read before it is written;
  - cycles.
- At run time, each stage's session holds only its declared inputs, so an undeclared read fails.
- Code identity generalizes the existing `build.rs` digests. The build-digest omission (review F11) is
  fixed: producer identity covers every canonical producer.
- Reuse keeps recompute-and-compare admission.

### 3.4 Legacy adapters

**What an adapter declares**
- the legacy relation it produces;
- its new input relations;
- its computation (DataFusion SQL, or Rust where a legacy encoding needs it);
- its **declared quirks**: legacy behavior deliberately reproduced;
- its **declared divergences**: intentional differences. Each names its review finding or reason and a
  row predicate.

**Rules**
- An adapter's output feeds only legacy consumers.
- Build an adapter only where a legacy consumer outlives its producer by a phase. Otherwise migrate the
  producer and consumer together (co-migration).
- Delete the adapter in the phase where its last consumer cuts over.
- Every adapter is listed in §5 with its planned deletion phase.

### 3.5 Legacy identities

Where a new recipe changes an ID (atoms, places, occurrences, flow facts, transfers):
- The new producer emits a temporary side relation `legacy_ids_<family>(new_id, legacy_id)`, computed
  with the legacy recipe during dual-run.
- Adapters join through it, so legacy IDs are reproduced exactly.
- All side relations are deleted in phase 5.

### 3.6 Parity harness

**Command:** `lctx parity --phase N --corpus <fixtures|pilot|all> --out <dir>`, plus
`just parity N` for the full corpus.

**Corpus**
- every fixture package under `fixtures/python/` (compiled with `compile-fixture`, behavioral
  profile);
- the review's P0 fixture, now `fixtures/python/semantic_shapes/`;
- the FastMCP pilot in the catalog and behavioral profiles.

**How it runs.** Dual-run in one attempt: the legacy producer, then the new producer plus adapters.

**What it compares**
- every adapted legacy relation, as an exact multiset of canonically sorted rows, IDs included;
- the served output: bundle/serving rows until phase 5, then MCP packet JSON for the PR5.7 journey set
  plus fixed request sets per tool.

**Verdicts**
- Declared divergences are applied as predicates.
- Any undeclared difference fails.
- A self-test injects a divergence (the run must fail), declares it (the run must pass), and corrupts
  an ID mapping (the run must fail).

**Report.** JSON and a Markdown summary, archived in
`docs/design_review/evidence/<date>_cutover-phase-N/`. They record, for each relation: equal, the
declared-divergence row count, and failure samples.

### 3.7 Tests

**Test tiers**
- **Pure unit tests (store-free):**
  - `lctx-model` declarations;
  - generated DDL, as a snapshot;
  - policies, the binder, the composition table, the kernel, obligations and verdicts;
  - the known-answer shape library: the review's P0 shapes and known-answer list as Arrow-level
    inputs.
- **Store tests:** testcontainers PostgreSQL 18.
- **Fixture and pilot compiles:** the disposable dev cluster from `just pg-dev` (phase 1).

**Keeping controls independent.** Existing tests move with their layer. Independent semantic controls
stay independent (CI-12): the Pysa TITO control, the flow-soundness oracle and runtime challenges.
Parity never substitutes for them.

## 4. Phases

Each phase lists work packages (WP) with their owning crate or module and deletion obligations.

**Timing of checks**
- During a phase: compile checks and focused tests only (AGENTS.md).
- **Phases 0 and 1 are one implementation scope for check timing** (operator, 2026-09-29):
  - the phase-0 exit runs its targeted tests and the WP0.10 review only;
  - the integrated gates below run once, at the phase-1 exit.
- At the phase exit, the phase is the authorized scope for integrated gates. The exit runs, in order:
  1. the parity report;
  2. `just fmt`;
  3. `just test-all`;
  4. a pilot compile in both profiles;
  5. the MCP journey set;
  6. recorded publication and read cost;
  7. the review listed for the phase.

Outcomes are reported as passed, failed, blocked or not_run, with the command.

### Phase 0 — Core contracts, store kernel and migration tooling

| WP | Scope and owner | Done when |
|---|---|---|
| 0.1 | `crates/lctx-model` scaffold; workspace and Hakari registration; `lctx-model` depends on neither `cpg-schema` nor `cpg-core`. Shared primitives (`Id`, `Digest`, `HashField`, `ArrowColumn`, `Codebook`) move into it and `cpg-schema` re-exports them (§4.1 D1) | builds; every `cpg-schema` snapshot unchanged (`just deps` runs at the phase-1 exit) |
| 0.2 | `relation!`/`codebook!`, the `RelationId` registry, generated types, DDL and view text (`decl`, `ddl`) | snapshot of generated DDL for the sample declarations; no hand inventory API exists |
| 0.3 | `IdKind`, `IdHasher` v2, recipes; `lctx_id` v2 UDF adapter in `cpg-core` with shared known answers | identity property tests: identity columns change the ID, provenance columns do not |
| 0.4 | Vocabulary and policies (`vocab`, `calls`, `transfer`, `obligation`, `derivation`, `projection`): owner rule, innermost-region join, five call policies, binder, effective-callable contract, composition table, obligation codebook with legacy mapping, priority, named budgets, verdict and discharge validity | known-answer suites: binder over positional, keyword, default, varargs, kwargs, receiver and ambiguous calls; the policy admission matrix; the composition table; verdict cases |
| 0.5 | Condition kernel ported to `condition` with atom identity v2 and rendering; the legacy kernel in `cpg-schema` stays for legacy code until phase 4 | ported kernel tests pass; the rendering truncation control passes |
| 0.6 | Store kernel (§3.2) with DataFusion memory pool; the canonical provider read mode and session factory in `cpg-core`; the owned provider fork decodes declared `List` columns | the store test list passes, including the Arrow → COPY → provider type matrix |
| 0.7 | Stage table and scheduler (§3.3); the legacy stage table declared as data, with a declared-dependency audit; per-stage code identity (F11) | refusal controls for missing, double and read-before-write writers; the legacy table's published outputs equal the 200 legacy relations |
| 0.8 | Adapter, legacy-ID and parity frameworks (§3.4–§3.6); `lctx parity` | the parity self-test passes |
| 0.9 | Known-answer shape library (§3.7) | used by 0.4 and 0.5 |
| 0.10 | **Design/target review** of the core contracts against §15 (fresh `design-reviewer`) | accepted, or its revisions applied |

Phase 0 deletes nothing; legacy stays whole.

### Phase 1 — Store cutover (existing relations onto PostgreSQL)

| WP | Scope and owner | Done when |
|---|---|---|
| 1.0 | One declared type regime: the legacy read-back registers declared-type batches; `Params::ids` binds `List<FixedSizeBinary(n)>`; every session comes from the session factory (§4.1 D8) | existing compile, analysis and behavior tests and snapshots unchanged |
| 1.1 | Legacy DDL shim: generate partitioned tables, keys, NOT NULL and CHECKs for the 200 legacy `table!` contracts. The `snapshots` contract is replaced by the registry. Legacy validation stays in the DataFusion rules; no legacy FKs are added | shim DDL snapshot; install on PG18; every legacy schema round-trips through COPY and the provider |
| 1.2 | `pipeline::compute` (in memory, stage-table scheduled, input-restricted sessions) and `publish` through the generation lifecycle. Stage bodies are extracted verbatim so the legacy `finish` and the pipeline share them. Compile tests become store-free; publication semantics become store tests (§4.1 D7–D9) | converted tests pass; publication store tests pass |
| 1.3 | Readers moved to pinned-generation provider reads: `rebuild` (reuse by server-side copy), `stage_cache`, `lctx query`/`diff`/`generations`, validation read paths, `db report`; `lctx_ops` and `db reconcile` deleted (§4.1 D11); `scripts/postgres_recovery.py`, `postgres_backup.py`, `tests/scripts/test_semantic_soundness.py` | reader tests on PostgreSQL |
| 1.4 | Serving from the generation: the bundle queries run over the `Computed` session (compile) or the provider session (`serving materialize`/`export`); the existing `lctx_serving` loader is fed from memory; native IPC artifacts come from the generation; manifest and `generation_digest` semantics are kept. The portable export and `import-bundle` remain as declared test-continuity compatibility until phase 5 (§4.1 D10) | both sessions materialize identical manifests; the export rebuilds to the same bytes |
| 1.5 | Dev environment: `just pg-dev` (disposable local PostgreSQL 18 in Docker); `just pilot` and `compile-fixture` use it; one role bootstrap file | fixture compile runs without the operator's database |
| 1.6 | **Parity:** all 200 relations, Delta against PostgreSQL, across the corpus; registry receipts; serving manifests; MCP journeys in both profiles; a cross-binary row-count and journey sanity check against the `fedd4a0` pilot | report archived |
| 1.7 | **Delete Delta.** Remove `delta.rs`, `snapshot.rs`, the legacy `finish`, Delta tests (their semantics rewritten as store tests) and the `deltalake` dependency. Edit `build.rs` `ENGINES`. Record the pin change through `pin-check`, with a family record superseding ADR-0002. Update `scripts/check_family.py` and `deny.toml`. Retire the ast-grep rules `delta-write-path.yml` and `no-raw-parquet-scan.yml`. Drop `deltalake` from `.config/library-skills.toml`. Rewrite storage §6 to the implemented store. Update AGENTS.md and the binding §5/§6 rows | `rg -i 'deltalake\|delta_kernel\|buoyant_kernel\|DeltaTable'` over code and configuration is empty |
| 1.8 | **Integrated gates, run once for phases 0–1**, then the **change/conformance review** | accepted |
| 1.9 | Operator cutover: migrate the operator database, recompile and select the pilot, delete superseded stores and generations (ADR-0078). Needs the operator's confirmation at that time | recorded |

### 4.1 Phase 0–1 execution decisions (2026-09-29)

These decisions execute ADR-0083 and ADR-0084 without changing them.

| # | Decision | Reason |
|---|---|---|
| D1 | `lctx-model` sits below `cpg-schema`. `Id`, `Digest`, `HashField`, `ArrowColumn` and `Codebook` move into it, and `cpg-schema` re-exports them. The v1 `IdHasher` stays legacy | One `Id` type, so adapters never convert |
| D2 | `relation!` is `macro_rules!` with one `model!` registry. Everything but row types is generated at run time from const metadata | No proc-macro crate; one registry |
| D3 | Canonical DDL is generated at run time. It is installed by `lctx store install` with a recorded `ddl_digest`, which `create_generation` checks. A contract change is `lctx store reset` (current-only). The static kernel stays in SQLx migrations | Successor of Delta's create/verify. No committed generated copy exists to drift (F10) |
| D4 | One schema `lctx_g<hex>` per generation. The owner creates staging tables with partition CHECKs; the writer COPYs directly; indexes are built at `mark_validated`; publish is one ATTACH transaction plus a revoke; retire is `DETACH … CONCURRENTLY` plus DROP | No double write or per-row trigger. Immutability comes from privileges. Names stay within 63 bytes |
| D5 | The roles keep their existing names: owner `lctx_migrator`, writer `lctx_importer`, reader `lctx_serving`, cache `lctx_app`. One bootstrap SQL file | Avoids churning the serving grants before phase 5 regenerates migrations |
| D6 | Legacy relations keep `snapshot_id` as the partition-key column; its value is the generation id (a declared quirk) | Renaming would be throwaway before phases 2–5 |
| D7 | `pipeline::compute` holds every relation in memory as `Arc`'d declared-type batches, then `publish` stages all of them. The generation is validated only if the in-memory validators passed; otherwise it fails and stays inspectable (`--unpublished`) | ADR-0083 compute model; keeps the Tested rejected-attempt inspection |
| D8 | The parity legacy side (declared compatibility, deleted in 1.7): one declared type regime first (1.0), then verbatim stage extraction, so the legacy `finish` and the pipeline share stage bodies. Dual-run in one process over the same raw batches and generation id | Parity then isolates exactly the store and orchestration change |
| D9 | Compile tests become store-free; publication and reader semantics become testcontainers store tests (`#[ignore]`, `just test-postgres`) | `just test` stays Docker-free |
| D10 | Serving: bundle queries over the `Computed` or provider session; the `lctx_serving` loader is fed from memory; the portable export and `import-bundle` stay as the Python tests' independent oracle until phase 5 | Test outcomes stay continuous; legacy serving is removed in phase 5 |
| D11 | `lctx_ops` and `db reconcile` are deleted; the registry and `generation_events` supersede them | Discovery and journaling existed because the store was outside PostgreSQL |
| D12 | `--store <dir>` becomes `--database` plus `--work <dir>`; `--snapshot` becomes `--generation` | The store is the database |
| D13 | `GenerationId` (16 bytes, canonical) and `ProjectionDigest` (32 bytes, legacy serving) are never blurred | "Generation" already had three meanings |
| D14 | The canonical provider session and session factory live in `cpg-core`; `lctx-postgres` keeps SQL effects only, amending §15.1's owner line | The PyO3 `lctx_storage` wheel must not link DataFusion |

### Phase 2 — Facts (L0 observations)

| WP | Scope and owner | Done when |
|---|---|---|
| 2.1 | Declare the L0 relations in `lctx-model`: sources and modules, syntax occurrences, declarations, Pysa call facts (origin, implicit flag), ty flow facts with structured places, occurrence-keyed atoms and test leaves, types, documents, package metadata, runs, contexts, producers, facts, coverage | DDL snapshot; store tests |
| 2.2 | Rewrite extraction (`cpg-extract`, `cpg-flow`) to emit them: one innermost-region join; parity rules for name, attribute and subscript places; no ty-internal indices in identities; legacy-ID side relations for atoms, flow facts and occurrences | focused extractor tests on the fixtures |
| 2.3 | Raw legacy adapters for every raw table still read by legacy stages (legacy atom encodings and DNF inputs are declared quirks) | adapter list matches §5 |
| 2.4 | **Parity** across the corpus | report archived |
| 2.5 | Delete legacy extraction writers and legacy raw `table!` producers; raw legacy tables now exist only as adapter outputs | `rg` shows no legacy raw writer in the extraction crates |
| 2.6 | **Change/conformance review** | accepted |

### Phase 3 — Normalized relations (the CPG)

| WP | Scope and owner | Done when |
|---|---|---|
| 3.1 | L1 relations and derivations: entities (with symbol keys), occurrences with the owner rule, places, call sites, targets and resolutions (implicit invocations disclosed), the five call-policy views, effective callables (decorator/wrapper normalization moved out of `surface.rs`), signatures and parameters, call bindings from the one binder, exports and public paths | known-answer and fixture tests |
| 3.2 | Role-generated graph catalog (`nodes`/`edges` materialized, `edge_kinds` with a reader or retired) and projection declarations | catalog equality against legacy through the adapter |
| 3.3 | Derived legacy adapters: the 22 derived tables, and the legacy call-edge variants still read by analysis stages (`flows` arcs, `summary_call_arcs`, `model_applications` input, usage candidates, evidence edges), each declared as a quirk mapped to its policy | adapter list matches §5 |
| 3.4 | **Parity** | report archived |
| 3.5 | Delete legacy `Derived` implementations, the `EdgeSource`/`NodeSource` registry, `ProjectionSpec`, and the legacy binders' SQL where their consumers co-migrate | `rg` check |
| 3.6 | **Change/conformance review** | accepted |

### Phase 4 — Analysis and catalog (L2/L3)

| WP | Scope and owner | Done when |
|---|---|---|
| 4.0 | **Ablation triage** (§6). Operator decisions are recorded before any migration in this phase | §6 complete |
| 4.1 | Canonical conditions: catalog, atoms, renderings, `condition_id` everywhere; delete the legacy DNF computation and the legacy kernel in `cpg-schema` | no `BoundedCondition` legacy half |
| 4.2 | Transfers (`flow_local`) from the flow model; control influences; obligations and coverage; behaviors as a derivation over transfers, rendered from the final composed kind (review F04) | P0 shapes: `facade` renders `unchanged`; controls hold |
| 4.3 | Summaries and composition: `derived_summary` and `composed` transfers through bindings and the SCC schedule; a bounded frontier that distinguishes the cost cap from dominance (review F07); discharge through the one validity function (producer, validator and native share it) | P0 rerun answers per-branch supply; a cost-cap control reports a proof limit |
| 4.4 | Models as data: authored models become `authored_model` transfers keyed by symbol key; Pysa TITO becomes `provider_summary`; the FastMCP registration recognizer becomes a named, pinned authored model (review F13) | model known answers; TITO control |
| 4.5 | Catalog on the new relations. Field links become `catalog_field_link` transfers, generalized to plain classes. Contracts and tri-state parameter defaults (review F01). Associations carry basis and origin through every projection, with a storage role (F02, F03). Scenarios, deployments, selection domains. The classifier evaluates tri-state evidence and reports a budget reason (F01, F07). Retrieval units become a declared relation | F01/F03 controls; provider-constructor, factory-default and candidate-only fixtures return `Unresolved` |
| 4.6 | Analytics: the projection runtime (shared dense index, adjacency in both directions, arc IDs, views); Pass A on `invocation`; Pass B/C on `dataflow` and transfers; communities, ranking, FCA/RCA and kNN as §6 decides; one findings emitter with input invocations; `MemberKey` fixed, with a finding-ID recompute rule (F08, F09) | shuffled-input determinism; lineage rules |
| 4.7 | Derivation sources declared for every step and proof relation; `derivations`/`derivation_premises` views; negative claims gated by coverage scope | a "why unresolved" query answers from the views |
| 4.8 | Legacy analysis adapters only for serving-read tables; **parity**; delete the legacy producers, including retired engines and their tests | report archived; `rg` check |
| 4.9 | **Change/conformance review** | accepted |

### Phase 5 — Serving and zero legacy

| WP | Scope and owner | Done when |
|---|---|---|
| 5.1 | Generated serving views, grants and lookup indexes. `lctx-postgres/queries` rewritten over views. Set-based hydration. A generation-scoped prepared catalog built once per pinned generation (review F12). Browse discloses unknown ownership; search discloses the reason for lexical-only fallback; `EmptyUnderCoverage` cites association coverage | real-PostgreSQL query tests |
| 5.2 | Wire DTOs derived from relation rows; Python keeps rendering models only; all MCP tools switch; served conditions carry `condition_id`; the explanation tool or section uses the derivation views | schema, Serde and MCP parity |
| 5.3 | Native executor reads generation relations (optional derived IPC cache) and uses the `lctx-model` verdict, discharge and condition functions | native tests |
| 5.4 | Retrieval on declared relations; lexical parameters in the policy digest (F13) | retrieval controls |
| 5.5 | **Delete** all adapters, legacy-ID relations and legacy declarations; `cpg-schema` (wire, selection and retrieval contracts moved to `lctx-model`); `bundle.rs`, `serving_projection` inventories, the `lctx_serving` materialization, `NATIVE_FILES` and every hand inventory. Regenerate migrations fresh (current-only, ADR-0078) | the §1 completion checks pass |
| 5.6 | Qualification: `just fmt`, `just test-all`, pilot in both profiles, the PR5.7 journey set plus new journeys (served condition IDs, why-unresolved explanation, plain-class configuration), `just docs-check`, generation cutover and obsolete runtime deletion | recorded outcomes |
| 5.7 | **Assembled design/target review.** Handoff; resume PR6 in the forward plan | accepted |

## 5. Legacy relation map

Only relations with a legacy consumer surviving a phase get adapters. The inventory is generated from
the stage table in WP0.7 and kept current there; this table fixes the policy.

| Legacy group | Count | Produced by the new model from | Adapter lifetime | Co-migration |
|---|---|---|---|---|
| Raw tables (`for_each_table!`) | 57 | Phase 2 | Phases 2–3; those read by analysis live until phase 4 | Delete as soon as no legacy stage reads them |
| Derived tables (`for_each_derived_table!`) | 22 | Phase 3 | Phase 3 to phase 4 | Graph-catalog tables co-migrate with Pass A where possible |
| Analysis, catalog and evidence tables (`for_each_analysis_table!`) | 121 | Phase 4 | Phase 4 to phase 5, only for serving-read tables | Unserved tables co-migrate with their consumers |
| Serving files and native files | 72 + 29 | Phase 5 | None; replaced by generated views. The portable export and `import-bundle` remain the Python tests' oracle until then (§4.1 D10) | — |

## 6. Research-engine and analytics triage (phase 4 entry)

**Protocol.** For each engine, publish the pilot (both profiles) and the full fixture corpus with the
engine disabled. Its outputs are then empty, and its dependent obligations stay open. Compare:
- served MCP output for the fixed request sets;
- the retained regression controls.

If both are unchanged, retire the engine: delete its code, tests and relations, and supersede its ADR.
Otherwise migrate it onto transfers and obligations. **The operator confirms each row.**

| Engine or variant | Pilot yield (P8) | Main consumer today | Expectation | Decision |
|---|---|---|---|---|
| Finite value summaries and local composition (`summaries/finite.rs`) | 23 flows | `inspect_value_paths`, discharge | migrate: its composition is the §15.6 operator | — |
| Discharge (`summaries/discharge.rs`) | 0 of 2,107 proved | behaviors | migrate as the one validity function | — |
| Completion kernel (`completion.rs`) and expression evaluator (`evaluation.rs`) | 25,648 / 120,262 step rows | summary admission premises | ablation decides | — |
| Frame exits, source-call normals, call execution | 13 / 4 / 7 rows | summary admission | ablation decides | — |
| Context protocols and context values | — | summary admission | ablation decides | — |
| Modeled and parameter identities | — | modeled reads | ablation decides | — |
| Actions (triggers, postconditions) | — | models | ablation decides | — |
| Two-stage source-call execution (`execution.rs`) | — | normals | ablation decides | — |
| Pass B from every public callable | 3.7 s | behaviors | migrate onto the `dataflow` policy and transfers | — |
| Communities, PageRank, FCA/RCA, kNN (off by default) | — | Related support (`+variants`) | ablation decides, under ADR-0071's consumer rule | — |

## 7. Tooling, pins, skills and documents by phase

| Phase | Changes |
|---|---|
| 0 | New crate `lctx-model` (pins unchanged; `pin-check` for any new direct dependency); new `lctx parity` subcommand; `just parity` |
| 1 | Remove `deltalake`, with a family record superseding ADR-0002; `just pg-dev`; `check_family.py`; retire Delta ast-grep rules and the `deltalake` skill selection; storage §6 and §B7/§B12 implemented text; AGENTS.md "The pieces" and Commands; binding §5/§6 |
| 2 | facts-and-identity §3.1–§3.7 and behavior-model §3.9 implemented text for observations and atoms |
| 3 | facts-and-identity §3.8 and storage §5 implemented text; analytics §9 projection text |
| 4 | behavioral-analysis §9.9, behavior-model §3.9, product §14.3–§14.8 implemented text; ADR supersessions for retired engines |
| 5 | synthesis-and-serving §10–§11, §B13/§B14, product §14.9–§14.11; retire `cpg-schema` references in AGENTS.md; regenerate the architecture map |

A section's implemented text changes in the phase where its layer cuts over. Until then, the §15
target and the legacy text coexist, each labelled.

## 8. Findings disposition

This table owns the current disposition of the review's findings. Each closes by construction in the
phase named, and only on its closure evidence.

| Review finding | Disposition | Responsible WP | Closure evidence |
|---|---|---|---|
| [F01](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F01) unknown default served as absent | open → phase 4 | 4.5 | tri-state evaluation; provider-constructor and factory fixtures `Unresolved` |
| [F02](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F02) call relation has no owner; origin hidden | open → phases 3–4 | 3.1, 3.5, 4.5 | five policy views; S1 as one policy edit; decorator-registration fixture shows its origin |
| [F03](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F03) association basis dropped | open → phase 4 | 4.5 | basis and origin required columns; candidate-only fixtures `Unresolved` |
| [F04](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F04) no transfer algebra; identity served as computed | open → phase 4 | 0.4, 4.2 | composition table; P0 `facade` renders `unchanged` |
| [F05](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F05) two condition representations | open → phases 2, 4, 5 | 2.2, 4.1, 5.2 | occurrence-keyed atoms; no parallel DNF; served `condition_id`; ty-upgrade identity control |
| [F06](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F06) no place/transfer vocabulary | open → phases 3–4 | 0.4, 3.1, 4.3, 4.5 | P0 rerun: plain-class option→field→reader and per-branch supply |
| [F07](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F07) distributed refusal/obligation; silent cost cap | open → phases 0, 4 | 0.4, 0.6, 4.3, 4.5 | one obligation owner; cost-cap control; budget reason; memory pool |
| [F08](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F08) no derived-result contract; lineage gaps | open → phase 4 | 4.6, 4.7 | one emitter; input invocations; finding-ID rule; derivation views |
| [F09](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F09) nominal projection layer | open → phases 3–4 | 3.2, 4.6 | projection by declaration; ADR-0044 amendment (made 2026-09-29) |
| [F10](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F10) serving is a second hand authority | open → phases 0, 5 | 0.2, 5.1, 5.5 | generated DDL, views and inventories; no hand serving file |
| [F11](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F11) implicit stage composition; producer identity mislabelled | open → phases 0–1 | 0.7, 1.2 | stage-table refusal controls; producer identity covers every canonical producer; the pipeline schedules from the table |
| [F12](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F12) per-request rebuilds and round trips | open → phase 5 | 5.1 | generation-scoped prepared catalog; set-based hydration |
| [F13](../design_review/reviews/design_review_semantic-data-model_2026-09-29.md#F13) pilot recognizer; lexical parameters outside the digest | open → phases 4–5 | 4.4, 5.4 | named authored model; policy digest |
| Review observations: browse unknown ownership, `EmptyUnderCoverage`, lexical-only reason | open → phase 5 | 5.1 | disclosed in responses |

**Operator decision (2026-09-29).** Do not repair these in the legacy code. The new contracts must
make them unrepresentable.

## 9. Risks

| Risk | Mitigation |
|---|---|
| Core design errors surface late | Contracts stay open until their layer cuts over; phase-0 design review; the known-answer library is the core's test suite from day one |
| Parity noise from nondeterminism | Canonical sort everywhere; fixed seeds; the self-test; shuffled-input controls kept |
| Loss of Tested semantics | Independent controls migrate with their layer; retirements only by ablation evidence plus operator confirmation |
| Publication or read cost on PostgreSQL | Recorded at each phase exit; tuning options are listed in §3.2; never a gate (operator) |
| Long product pause | The product keeps working on adapter output through phase 4; phase exits are small and objective |
| Adapter sprawl | Co-migration rule; §5 lifetimes; any adapter surviving its planned phase is a finding |
| Sealed evaluation assets | `eval/heldout` is never read or moved; gold and eval scripts move only in their tooling phase |

## 10. Deferred, each with a trigger

| Item | Trigger |
|---|---|
| Skip-on-key stage reuse | ADR-0081's measured workload trigger |
| Publication/read performance tuning | A phase-exit cost the operator judges too high |
| salsa, ascent, moka, typed-index-collections, roaring | Their §14.11 and forward-plan §7 triggers |
| CFG and post-dominators (petgraph `simple_fast` on `Reversed`) | A claim that ty reachability conditions cannot express |
| Cross-release diff, dependency-library summaries | The first consumer of symbol keys |
| PR6 comparative confirmation | Phase 5 exit |
