# Storage and publication

This owner covers what happens to validated rows: how an attempt becomes an immutable published
snapshot in the Delta store, how readers see exactly one snapshot, how the schema contract is
enforced over time, how analytics read the graph through declared projections, and how a serving
generation is derived from a published snapshot. Inputs are the attempt's raw, derived and
analysis batches ([§4.3](acquisition-and-extraction.md#section-4-3)) and the table contracts of
[§3](facts-and-identity.md#section-3); consumers are the analytic passes
([§9](analytics.md#section-9)), synthesis and the serving interface
([§10–§11](synthesis-and-serving.md#section-10)), and `lctx query`. Effects are confined here:
transformations upstream never write Delta, and the server never reads it. Code:
`crates/cpg-core/src/` (`delta.rs` create/verify/write/merge, `snapshot.rs` pinned readers,
`attempt.rs` the attempt protocol, `bundle.rs` generations), `crates/cpg-schema/src/`
(`table.rs` read modes, `projection.rs`, `bundle.rs`), `crates/lctx-analytics/src/graph.rs` (the
projection adapter). Tests: `crates/cpg-core/tests/` (`delta.rs`, `compile.rs`, `bundle.rs`,
`analysis.rs`). The map is in the [architecture README](../README.md).

## §5 Projections

**Implemented** and **Tested** for the invocation projection, its declared spec
(`cpg_schema::projection`) and the adapter (`lctx_analytics::graph`), 2026-09-23:
`the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`;
`pass_a_finds_the_known_answers_on_analysis_shapes` checks the property-getter arc, the
override-open candidate arc and a module-level caller on the real pipeline. The petgraph
behaviour cited is **Interface-checked** (petgraph skill and pinned source).

**Declaring a projection.** A projection declares:
- the snapshot;
- node and edge kinds;
- the accepted origins and fidelities;
- candidate-target and unknown-target policies;
- the algorithm and parameter set.

The vertex universe is selected separately from the edges, so isolated public APIs survive. The
spec is recorded by digest on the invocation that uses it, and its SQL is part of the
`compiler_digest` (§3.4.1).

**The invocation projection**
- It is built by joining call-site ownership with call targets in DataFusion: `encloses_call` ⋈
  `call_target`, plus the non-potential `site_target` arcs of property getters and setters, whose
  site's owner is read from `syntax_nodes`.
- **Definition arcs:** a function → each function it declares (`declares`), with the nested
  callable as the arc's site, no phase and `arc_kind = definition`. A decorator factory's
  `decorator`, or a closure it registers, is then reachable, and so is what it calls. A
  definition arc is never a `direct_delegation`. Every arc carries its `arc_kind` (`call` |
  `definition`).
- Call and property arcs read the site's owner through one fragment
  (`cpg_schema::graph::owner_of`), so they cannot disagree about who calls.
- It keeps `call_site_id` (which is also the resolution's id: a `ResolutionSet` is keyed by its
  call site, §3.6), `edge_id`, `invocation_phase`, the arc's modality and
  `has_unresolved_remainder` on every arc.
- **Parallel call sites are preserved.**
- **Excluded:** `potential` targets (so every `higher_order_target`) and `synthetic_model` arcs.
- **Constructor phases stay tagged.**
- **Candidate arcs** (Pysa `Overrides` dispatch) are kept with their modality, so a pass can
  treat them as candidates (§3.6).
- **Unknown targets.** A call site with no target, or with an unresolved remainder, gets no arc
  to a placeholder. It is listed in the projection's `unresolved_sites` relation, which Pass A
  turns into `incomplete_resolution` findings.
- **Callers** are the enclosing function, or the module or class whose body holds the call, as
  typed vertices.
- The projection's rows are persisted where a result cites them: `witnesses` holds each path
  step's call site, callee and `edge_id` (§3.4.1).

**Three identities**

| Identity | Scope | Persisted? |
|---|---|---|
| Canonical `node_id` / `fact_id` | persistent | yes |
| Projection row | links an arc to its evidence rows | yes |
| petgraph `NodeIndex` / `EdgeIndex` | temporary coordinates | **never** |

**Container and adapter.** `petgraph::Graph<(), u32, Directed, u32>`, whose edge weight is the
arc's row index (the arc columns stay in Arrow).
- **Two queries on the pinned snapshot:** the vertex universe (`ORDER BY node_id`, isolates
  included) and the arcs with the columns above (`ORDER BY src, dst, call_site_id, edge_id`:
  total, since `edge_id` is unique).
- **The dense index is the sorted domain ids:** read the `FixedSizeBinaryArray` directly (no hex
  strings); domain → dense is a `binary_search`.
- `Graph::with_capacity(n, m)`, then `try_add_node` in order so `NodeIndex(i) == i`, and
  `try_add_edge` in canonical arc order so `EdgeIndex(k)` is arc row k.
- Immutable once built, parallel edges kept, nothing ever removed (removal silently re-points held
  indices).
- **Scopes and callers without copies:** `EdgeFiltered`/`NodeFiltered`, and `Reversed` (which keeps
  the original edge ids). Filtered views keep the base graph's `node_bound`.
- **Not** `Csr` or `GraphMap` (they drop parallel edges), nor `StableGraph` (unneeded, and it blocks
  several algorithms); no `rayon` or `serde-1` features.
- **Order is a precondition, not a repair:** the adapter refuses arcs out of canonical order, and
  the arcs query's total `ORDER BY` supplies it. Tests: the refusal
  (`the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`), and whole-attempt identity
  across module order (`pass_a_is_identical_across_module_order_and_location`).

**Determinism rules**
- `edges_directed` returns the newest edge first. So `Bfs` visits siblings newest-first, while
  `Dfs` pushes every successor and pops the last, taking the oldest first. Probe (2026-09-23):
  with arcs r→1, r→2, r→3, Bfs = [0,3,2,1,4] and Dfs = [0,1,4,2,3].
- So traversals never rely on walker order. They collect a node's edges and **sort by
  (target canonical id, arc key)** before choosing.
- SCC members are sorted by canonical id. Which SCC routine the summaries use, and its stack
  safety, is owned by the analytics side and still open
  ([plan W13](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

> Decision: ADR-0044, ADR-0047

---

## §6 Persistence and publication

> Decision: ADR-0047

**Implemented** in `cpg-core` and **Tested** where a line names a test or says so
(`cpg-core/tests/delta.rs`, `compile.rs`, `bundle.rs`; 2026-09-22 onward). Lines about delta-rs
behaviour that no repository test asserts are **Interface-checked** against the pinned delta-rs and
kernel sources (deltalake skill, 2026-09-22).


### §6.1 Canonical tables and publication

- **Family tables.** Each fact family is a set of append-only Delta tables. Every
  snapshot-qualified row carries `snapshot_id` as a plain column; tables are not partitioned
  (partitioning on the Binary id returned wrong results, and a hex column would migrate every
  contract).
- **A compile attempt:**
  1. writes its rows, each snapshot-qualified table in **exactly one commit** carrying
     `lctx.snapshot_id` in its `commitInfo`;
  2. runs local and cross-table validation (§8);
  3. then, **and only then**, appends one row per table to `snapshots`: (snapshot_id,
     content_digest, table, Delta version, schema digest, compiler digest, row count), in a
     single commit (**Tested**: a validation failure publishes nothing, and the published
     snapshot's reader sees only its rows). A snapshot is published at most once: `publish`
     refuses a `snapshot_id` that `snapshots` already holds.
- **That commit is the publication act** (Delta commits are atomic per table; there is no
  multi-table commit). The row set also records the `embedding_cache` version the attempt read.
- **An error on the `snapshots` append itself** is ambiguous (`delta.commit.1`). Re-read
  `snapshots`, classify the attempt as published or unpublished, and only then retry or build
  the generation (**Tested**: `a_failed_snapshots_append_is_classified_by_rereading`).
- **Any write error aborts the attempt.** A retry is a new attempt with a new `snapshot_id`,
  because a Delta write error does not mean nothing committed (`delta.commit.1`). Rows from
  unpublished attempts are invisible to readers.
- **Adapter failure on a module** gives `coverage.status = failed` for that module and family. The
  snapshot can still publish, with that gap visible.
- **Writes go through `DeltaTable::write` only** (§4.3), except the global `embedding_cache`.
  Both paths enforce the tables' CHECK constraints and `delta.appendOnly` where the table
  features are present, and the open-time verify asserts they are. DataFusion `INSERT INTO` does
  not, so it is never used (**Tested**).
- **The global table** (`embedding_cache`, read mode `global`, §6.2) has no `snapshot_id` column
  and may be committed more than once per attempt. It is written by an insert-only MERGE on
  `(spec_hash, input_hash)` (`DeltaTable::merge(..).when_not_matched_insert(..)`), with a retry on a
  commit conflict: delta-rs never marks a commit a blind append, so two concurrent appends of one
  key would both commit, and because served vectors differ slightly between requests a duplicate
  key would make the generation's vector choice, and its byte-identical rebuild, unstable. The
  merge reads the target, so a racing merge conflicts and re-runs (probed before use,
  2026-09-23). Source rows pass the local type, length, finiteness and norm checks first. The
  snapshot's row for it records the version read after the last write and `row_count` = the rows
  the snapshot **used**. Key uniqueness over the whole table is a validation rule; since the table
  is append-only and validators never repair, a violation's declared recovery is a migration to a
  new table keeping the earliest-version row per key (an accepted contingency, not implemented).
  If two concurrent merges could ever leave two rows, the accepted fallback is an exclusive lock
  file in the store around an anti-join and append. **Known gap:** the two cache-fill entry points
  apply different token admission, and one returns locally computed vectors instead of the
  committed winner ([plan W9](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition),
  RFU/F06); the cache's embedding semantics are owned by
  [§11.1](synthesis-and-serving.md#section-11-1).
- **`SaveMode::Ignore` is never used.** It appends to existing tables (`delta.write.3`).
- **No vacuum or optimize on fact tables.** Vacuum defaults to `dry_run=false` and Lite mode.
- **Retention keeps every published snapshot readable** (**Tested** by
  `retention_keeps_old_versions_loadable`, 2026-09-22).
  - **The problem.** By default every commit's post-commit hook writes a checkpoint every 100
    versions and runs expired-log cleanup (`delta.enableExpiredLogCleanup` true,
    `delta.logRetentionDuration` 30 days), which deletes the commits and checkpoints below the
    newest checkpoint older than the cutoff. After that, `with_version(v).load()` of an older `v`
    fails ("No files in log segment"), so a snapshot published more than 30 days ago would be
    unreadable at its recorded version, and the per-commit reads of §6.2 would lose their commits.
  - **The rule.** Every table, `snapshots` included, is created with
    `delta.enableExpiredLogCleanup = false` and `delta.logRetentionDuration = interval 36500 days`,
    set through `with_configuration_property` (never `with_configuration`, which replaces the
    whole map). Open-time verify compares both strings exactly, because delta-rs silently falls
    back to its default on a value it cannot parse. Checkpoints stay on, so a load replays at most
    99 commits.
  - The test creates a table with `checkpointInterval = 2` and zero retention: an old version is
    unloadable under the defaults and loadable under ours.


### §6.2 Readers

**Implemented** in `cpg_core::snapshot` and **Tested** (`reads_pin_the_version_and_filter_the_snapshot`,
`a_pinned_read_opens_only_its_commits_files`, `a_rejected_attempt_is_inspected_at_its_own_commits`).

1. Resolve the snapshot's row set in `snapshots`.
2. Load each table **at its recorded version** with
   `DeltaTableBuilder::from_url(..)?.with_version(v).load()`, then assert
   `table.version() == Some(v)`. A provider built on an already-loaded handle ignores the
   requested version (`delta.open.2`, `delta.read.4`).
3. Honour the table's declared **read mode** (`snapshot` or `global`, in `cpg-schema`); every
   reader (`register`, `session`, `attempt_versions`, the generation builder) does.
   - **A snapshot-qualified table: read only that commit's files** (`snapshot::commit_provider`:
     `LogStore::read_commit_entry(v)` + `logstore::get_actions` → the `Add` actions →
     `TableProviderBuilder::with_adds`). A snapshot's rows of a table are exactly the commit at
     its recorded version (one commit per table per attempt, §6.1). Before reading,
     `commit_adds` confirms that the commit's `lctx.snapshot_id` names the snapshot and refuses
     otherwise (`ForeignCommit`), never reading it as empty; `snapshots` remains the authority for
     *which* version, and the commit metadata only confirms it. A selected file that is gone fails
     the scan rather than returning fewer rows. Then **filter `snapshot_id`** as the row
     predicate. The reason: Delta log statistics skip the Binary `snapshot_id`, so a filter alone
     skips no file and costs one footer read per file of every snapshot, a cost that grows with
     the store (a 200-snapshot table answered in 4.7 ms instead of 15.7 ms, Measured 2026-09-23).
   - **A global table** (`embedding_cache`) is read at its recorded version over **all** its
     active files, with no commit selection, no `lctx.snapshot_id` check and no snapshot filter.
     Its recorded version may be another attempt's commit.
4. Project columns by name through the DataFusion provider. Never use `scan_table().with_columns`,
   which returned the wrong column for a partition-first schema (`delta.read.3`).
5. Never scan the Parquet directory directly (`delta.read.2`).
6. Register one object store per table root per session (`delta.storage.4`).

**An unpublished attempt** is readable only for inspection (`lctx query --unpublished`, §4.0), at
the commits carrying its own `lctx.snapshot_id` (`snapshot::attempt_versions`, which walks each
table's kept JSON commits). Tables the attempt did not write are left out, so a query naming them
fails instead of reading empty.

**Limit.** The contract rests on one commit per snapshot-qualified table per attempt. A writer
that splits a table's write across commits (a streaming derive, a retried raw write) must record
every commit, or this contract must change. There is no relational read across snapshots; a diff
compares two snapshots read separately.


### §6.3 Schema evolution

**Implemented** and **Tested** for strict verification (`cpg_core::delta::verify`;
`cpg-core/tests/delta.rs`). The fresh-store rebuild policy is **Interface-checked**; its first
execution after the `function_implementations` schema migration is pending the integrated gate.

- **What is implemented: strict verification, no evolution.** When an attempt opens a table,
  `verify` compares the stored Delta schema with the table's declared `cpg-schema` contract and
  refuses any difference (`SchemaDrift`), as it refuses a changed `appendOnly`, retention or CHECK
  set. No code evolves a table's schema in place: there is no `SchemaMode::Merge` write and no
  additive-column migration
  ([plan W15](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition), RF/F16).
- **A schema change is a reviewed migration.** It shows as a changed contract snapshot, is
  accepted by reading the `.snap.new` and is named as a migration in its commit. In practice it
  needs a fresh store. The old store can be moved aside temporarily for rollback; the new compile
  produces its own `embedding_specs` and current snapshot. Historical snapshot reads and keeping
  the old binary are not supported requirements.
- **A change to a table's CHECK set is a migration** in the same way, because the open-time verify
  refuses a table whose constraints differ from the declared set.
- **Schema digests** are `cpg-schema`'s digest of the declared contract, never of Arrow read back,
  because read-back changes `Utf8` → `Utf8View` and renames list children to `element`.
- **Rebuild policy** ([ADR-0048](../../adr/0048-schema-rebuild-policy.md), W15). Build a fresh
  store from pinned inputs under new producer/compiler identity and publish the current library's
  new snapshot. Do not copy old analysis tables or snapshots. The global embedding cache may be
  reused only through its independently verified contract. The old store can be removed after
  acceptance. A real historical-read consumer reopens this policy; no in-place merge is implied.

> Decision: ADR-0048


### §6.4 Serving generations

**Implemented** and **Tested** for the declared files of the current format
(`a_generation_rebuilds_to_the_same_bytes`, `a_changed_generation_is_refused`,
`mixed_embedding_specs_are_refused`, `serving_schema_digests_are_the_shared_known_answers`;
2026-09-23 onward); the files marked below as later-stage are **Proposed**.

- **Derivation.** The generation is built **only after** the `snapshots` append succeeds, by
  reading the published snapshot at its recorded versions, including `embedding_cache`. That is
  its one derivation path. Rebuilding a generation from Delta gives byte-identical files.
- **Bundle.** It is a directory `generations/<key>/` of Arrow IPC files, with declared schemas
  from `cpg-schema` (`cpg_schema::bundle`). Codebook values are served as their text. The files:
  - `briefs`, with each brief's Outcome and its status;
  - `assertions`, one row per (brief, ordinal), with kind, section and status;
  - `supports`, each assertion's findings (by kind) and evidence, by role and ordinal;
  - `evidence`, with its resolved text and its file's or document's path;
  - `brief_members`, and `symbol_map` (exact public access path → brief): every public path of
    a brief's seed, own and inherited, with `own` (`semantic:brief-member-public`), so any public
    spelling promotes the brief (`fastmcp.FastMCP.http_app` promotes `TransportMixin.http_app`'s).
    The server shows a brief by its own paths;
  - `public_paths`: the whole public surface (node, path, kind, `own`, `preferred`), what a
    query's or a gold operation's spelling resolves against;
  - `lexical_text`: each brief's documents, then the **distinct tokens** of its seed's **own**
    public names (inherited spellings promote but name nothing here), each once however many
    spellings or splits produce it (the path, its segments and their words at underscores and
    case changes; `cpg_schema::bundle::name_tokens`), for BM25 (§11.2). The tokenizer is Unicode
    lower-casing, then ASCII letter-and-digit runs, identical in Rust and Python, with shared
    known answers in `specs/serving/tokens.json`;
  - `embedding_spec` and `vectors` (§11.1), whose vector type is `fixed_size_list(float32 not null
    "item", D)`, `D` the spec's dimensions. The spec comes from the snapshot's `embedding_specs`
    row, and each document's key is `(spec_hash, input_hash)`;
  - the behavior and summary files: `operations`, `operation_facets`, `operation_facet_status`,
    `operation_parameters`, `operation_text`, `operation_vectors`, `behaviors`, `singletons`,
    `ambient_reads`, `place_claims`, `conditions`, `condition_nodes`, `analysis_conditions`,
    `analysis_condition_nodes`, `summary_flows`, `summary_flow_steps`, `summary_boundaries`,
    `flow_test_leaves` and `flow_test_value_links`. Their meaning is owned by
    [§3.9](behavior-model.md#section-3-9), [§9.9](behavioral-analysis.md#section-9-9) and
    [§11](synthesis-and-serving.md#section-11); `cpg_schema::bundle::files` declares the list,
    keys and schemas and is authoritative.
  - Usage patterns are `usage_pattern` assertions in `assertions` and `evidence`, so no separate
    served file exists.
  - **Proposed** additions: `concepts`, `concept_members` and `vocabulary` (the forward plan's
    Stage 4).
- **`MANIFEST.json`** has sorted keys. It lists:
  - the format version (`cpg_core::bundle::FORMAT`), the snapshot, and its content and compiler
    digests;
  - the spec hash (none for a lexical-only generation);
  - per file, its sha256, rows and **serving schema digest**. That digest is a SHA-256 over a
    language-neutral canonical form: field name, a declared type grammar, nullability and sorted
    metadata. Python recomputes it with its standard library, and the known answers are shared
    by the Rust and Python tests (`specs/serving/schema_digests.json`). It is not the store's
    `canonical_schema` (§6.3), whose Rust type display Python cannot reproduce;
  - a coverage summary: coverage by scope, family and status; boundaries by reason; invocations
    by completion; briefs by review state and analysis backing; unresolved and absent slots by
    section (the §B11 gap metric).
  - The **generation key** is the first 16 hex digits of the SHA-256 of the manifest without its
    key. It moves with any file, and with the snapshot's provenance.
- **Normalization**, so a rebuild is byte-identical:
  - each file is one query, sorted by its declared key;
  - every column is cast to its declared type and rebuilt through a builder, so no view type,
    scan metadata or byte under a null slot reaches the file;
  - one record batch per file, written in the Arrow IPC file format: V5, 64-byte alignment,
    uncompressed.
  - One generation never mixes vector spaces: a snapshot with two specs is refused, by
    `semantic:one-embedding-spec` and by the builder.
  - `a_generation_rebuilds_to_the_same_bytes` rebuilds from the store, and from a second compile
    in another location and module order.
  - `lctx compile` builds the generation after publishing; `lctx bundle` rebuilds it.
  - A reader session registers its own `snapshots` rows, so the manifest's digests are read
    from the store.
- **Activation.**
  - The bundle is smoke-queried by the serving code's own test entry point.
  - Then the `generations/active` symlink is switched by atomic rename.
  - A running server keeps the generation it loaded; restarting it picks up the new one.
- **Derived search indexes** (an FTS table, any ANN index) are rebuilt from the generation's Arrow
  files, keyed by the generation key, outside the byte-identical manifest.
- **Known gap:** the native loader decodes positional tuples and re-whitelists proof kinds, so a
  reader's admission can drift from the schema contract
  ([plan W1](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition), ARC-01).

> Decision: ADR-0047, ADR-0043
