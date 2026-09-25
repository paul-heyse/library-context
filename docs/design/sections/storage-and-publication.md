# Storage and publication

<!-- owner-intro -->

## §5 Projections

**Interface-checked** (petgraph skill). Source: IP L1335–L1415, L2233–L2362. The invocation
projection, its declared spec (`cpg_schema::projection`) and the adapter (`lctx_analytics::graph`)
are **Implemented** and **Tested** (slice 1.4, 2026-09-23: `the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`;
`pass_a_finds_the_known_answers_on_analysis_shapes` checks the property-getter arc, the
override-open candidate arc and a module-level caller on the real pipeline).

**Declaring a projection.** A projection declares:
- the snapshot;
- node and edge kinds;
- the accepted origins and fidelities;
- candidate-target and unknown-target policies;
- the algorithm and parameter set.

The vertex universe is selected separately from the edges, so isolated public APIs survive.

**The invocation projection (v1)**
- It is built by joining call-site ownership with call targets in DataFusion: `encloses_call` ⋈
  `call_target`, plus the non-potential `site_target` arcs of property getters and setters, whose
  site's owner is read from `syntax_nodes`.
- **Definition arcs** (slice 1.4 review F1): a function → each function it declares
  (`declares`), with the nested callable as the arc's site, no phase and `arc_kind = definition`.
  A decorator factory's `decorator`, or a closure it registers, is then reachable, and so is what
  it calls. A definition arc is never a `direct_delegation`. Every arc carries its `arc_kind`
  (`call` | `definition`).
- Call and property arcs read the site's owner through one fragment
  (`cpg_schema::graph::owner_of`), so they cannot disagree about who calls (review O7).
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
  step's call site, callee and `edge_id` (ADR-0019).

**Three identities**

| Identity | Scope | Persisted? |
|---|---|---|
| Canonical `node_id` / `fact_id` | persistent | yes |
| Projection row | links an arc to its evidence rows | yes |
| petgraph `NodeIndex` / `EdgeIndex` | temporary coordinates | **never** |

**Container and adapter** (the library-leverage review, D4; the recipe slice 4 builds).
`petgraph::Graph<(), u32, Directed, u32>`, whose edge weight is the arc's row index (the arc columns
stay in Arrow).
- **Two queries on the pinned snapshot:** the vertex universe (`ORDER BY node_id`, isolates
  included) and the arcs with the columns above (`ORDER BY src, dst, call_site_id, edge_id`: total, since `edge_id` is unique; H1 review F9).
- **The dense index is the sorted domain ids:** read the `FixedSizeBinaryArray` directly (no hex
  strings); domain → dense is a `binary_search`.
- `Graph::with_capacity(n, m)`, then `try_add_node` in order so `NodeIndex(i) == i`, and
  `try_add_edge` in canonical arc order so `EdgeIndex(k)` is arc row k (`graph_impl/mod.rs:656-681`).
- Immutable once built, parallel edges kept, nothing ever removed (removal silently re-points held
  indices).
- **Scopes and callers without copies:** `EdgeFiltered`/`NodeFiltered`, and `Reversed` (which keeps
  the original edge ids). Filtered views keep the base graph's `node_bound`.
- **Not** `Csr` or `GraphMap` (they drop parallel edges), nor `StableGraph` (unneeded, and it blocks
  9 algorithms); no `rayon` or `serde-1` features.
- **Order is a precondition, not a repair** (review O4): the adapter refuses arcs out of
  canonical order, and the arcs query's total `ORDER BY` supplies it. Tests: the refusal
  (`the_adapter_keeps_parallel_arcs_isolates_and_refuses_disorder`), and whole-attempt identity
  across module order (`pass_a_is_identical_across_module_order_and_location`).

**Determinism rules**
- `edges_directed` returns the newest edge first (`graph_impl/mod.rs:919, 982`). So `Bfs` visits
  siblings newest-first (`visit/traversal.rs:294-306`), while `Dfs` pushes every successor and
  pops the last, taking the oldest first (`:108-121`). Probe (2026-09-23): with arcs r→1, r→2, r→3,
  Bfs = [0,3,2,1,4] and Dfs = [0,1,4,2,3].
- So traversals never rely on walker order. They collect a node's edges and **sort by
  (target canonical id, arc key)** before choosing.
- SCC members are sorted by canonical id.

> Decision: ADR-0011, ADR-0019

---

## §6 Persistence and publication

> Decision: ADR-0017

**Tested** where a line cites a spike or slice 2 (`cpg-core/tests/compile.rs`, 2026-09-22),
otherwise **Interface-checked** (deltalake skill probes). Source: IP L1603–L1623, L2967–L3006. The ADR-0009 Delta probe ran in full on 2026-09-22: S6
(CHECK, read cast), P1 (Binary statistics), P2 (a failed validation publishes nothing), P3 (an
ambiguous append is classified by re-reading) and P4 (a byte-identical bundle rebuild).


### §6.1 Canonical tables and publication

> Decision: ADR-0012, ADR-0014

- **Family tables.** Each fact family is a set of append-only Delta tables. Every row carries
  `snapshot_id` as a plain column; tables are not partitioned.
- **A compile attempt:**
  1. writes its rows;
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
  the bundle.
- **Any write error aborts the attempt.** A retry is a new attempt with a new `snapshot_id`,
  because a Delta write error does not mean nothing committed (`delta.commit.1`). Rows from
  unpublished attempts are invisible to readers.
- **Adapter failure on a module** gives `coverage.status = failed` for that module and family. The
  snapshot can still publish, with that gap visible.
- **Writes go through `DeltaTable::write` only** (§4.3), except the global `embedding_cache`,
  written by an insert-only `DeltaTable::merge` so that concurrent attempts cannot duplicate a
  key (ADR-0017 amendment; probed in slice 1.6 before use). Both paths enforce the tables' CHECK
  constraints and `delta.appendOnly` where the table features are present, and the open-time
  verify asserts they are. DataFusion `INSERT INTO` does not, so it is never used (**Tested**,
  spike S6).
- **`SaveMode::Ignore` is never used.** It appends to existing tables (`delta.write.3`).
- **No vacuum or optimize on fact tables** in stage 1. Vacuum defaults to `dry_run=false` and Lite
  mode.
- **Retention keeps every published snapshot readable** (C1, ADR-0014; read in the pinned
  delta-rs 58f07cd and kernel 8ba063f sources; **Tested** by
  `retention_keeps_old_versions_loadable`, 2026-09-22).
  - **The problem.**
    - By default, every commit's post-commit hook writes a checkpoint every 100 versions.
    - It also runs expired-log cleanup (`delta.enableExpiredLogCleanup` true,
      `delta.logRetentionDuration` 30 days), which deletes the commits and checkpoints below the
      newest checkpoint older than the cutoff.
    - After that, `with_version(v).load()` of an older `v` fails ("No files in log segment"). A
      snapshot published more than 30 days ago would then be unreadable at its recorded version,
      breaking §6.2.
  - **The fix.**
    - Every table, `snapshots` included, is created with `delta.enableExpiredLogCleanup =
      false` and `delta.logRetentionDuration = interval 36500 days`, set through
      `with_configuration_property`. Never `with_configuration`, which replaces the whole map.
    - Open-time verify compares both strings exactly, because delta-rs silently falls back to its
      default on a value it cannot parse.
    - Checkpoints stay on, so a load replays at most 99 commits.
  - Tested by a table created with `checkpointInterval = 2` and zero retention: an old version is
    unloadable under the defaults and loadable under ours.


### §6.2 Readers

> Decision: ADR-0012

1. Resolve the snapshot's row set in `snapshots`.
2. Load each table **at its recorded version** with
   `DeltaTableBuilder::from_url(..)?.with_version(v).load()`, then assert
   `table.version() == Some(v)`.

   A provider built on an already-loaded handle ignores the requested version
   (`delta.open.2`, `delta.read.4`).
3. **A snapshot-qualified table: read only that commit's files** (`snapshot::commit_provider`, H1
   P3; ADR-0017). A snapshot's rows of a table are exactly the commit at its recorded version (one
   commit per table per attempt), and the commit's `lctx.snapshot_id` must name the snapshot:
   another snapshot's commit is refused (`ForeignCommit`), never read as empty (H1 review F2).
   Then **filter `snapshot_id`** as the row predicate. **A global table that accumulates across
   attempts** (`embedding_cache`) is read at its recorded version over **all** its active files,
   with no commit selection and no snapshot filter (H1 review F3). Each table declares its read
   mode (`snapshot` or `global`) in `cpg-schema`, and every reader honours it (ADR-0017
   amendment).
4. Project columns by name through the DataFusion provider. Never use `scan_table().with_columns`,
   which returned the wrong column for a partition-first schema (`delta.read.3`).
5. Never scan the Parquet directory directly (`delta.read.2`).
6. Register one object store per table root per session (`delta.storage.4`).


### §6.3 Schema evolution

> Decision: ADR-0012

- **Additive nullable columns only**, via `SchemaMode::Merge`. Anything else is a new table
  (`edges_v2`), with the move recorded in `snapshots`.
- Schema digests are `cpg-schema`'s digest of the declared contract, never of Arrow read back, because read-back
  changes `Utf8` → `Utf8View` and renames list children to `element`.
- **A change to a table's CHECK set is a migration**, like a column change: a new table, or an
  explicit constraint step recorded by ADR. The open-time verify (§4.3) refuses a table whose
  constraints differ from the declared set.


### §6.4 Serving generations

- **Derivation.** The bundle is built **only after** the `snapshots` append succeeds, by reading
  the published snapshot at its recorded versions, including `embedding_cache`. That is its one
  derivation path. Rebuilding a generation from Delta gives byte-identical files.
- **Bundle.** It is a directory `generations/<key>/` of Arrow IPC files, with declared schemas
  from `cpg-schema` (`cpg_schema::bundle`). Codebook values are served as their text. The files:
  - `briefs`, with each brief's Outcome and its status;
  - `assertions`, one row per (brief, ordinal), with kind, section and status;
  - `supports`, each assertion's findings (by kind) and evidence, by role and ordinal;
  - `evidence`, with its resolved text and its file's or document's path;
  - `brief_members`, and `symbol_map` (exact public access path → brief). **`FORMAT` 2** (the
    holistic assessment's A1, 2026-09-24; pre-registered in ADR-0010's amendment): every public
    path of a brief's seed, own and inherited, with `own` (`semantic:brief-member-public`), so any
    public spelling promotes the brief (`fastmcp.FastMCP.http_app` promotes
    `TransportMixin.http_app`'s). The server shows a brief by its own paths. `FORMAT` 1 served
    the seed's aliases only;
  - `public_paths`: the whole public surface (node, path, kind, `own`, `preferred`), what a
    query's or a gold operation's spelling resolves against;
  - `lexical_text`: each brief's documents, then the **distinct tokens** of its seed's **own**
    public names (inherited spellings promote but name nothing here: ADR-0010's R2 F1 amendment),
    each once however many spellings or splits produce it (the path, its segments and their words at
    underscores and case changes; `cpg_schema::bundle::name_tokens`), for BM25 (§11.2). The
    tokenizer is Unicode lower-casing, then ASCII letter-and-digit runs, identical in Rust and
    Python, with shared known answers in `specs/serving/tokens.json`;
  - `embedding_spec` and `vectors` (§11.1), whose vector type is `fixed_size_list(float32 not null
    "item", D)`, `D` the spec's dimensions. The spec comes from the snapshot's `embedding_specs`
    row, and each document's key is `(spec_hash, input_hash)` (slice 1.7).
  - Usage patterns are `usage_pattern` assertions in `assertions` and `evidence` (slice 2.2,
    deviation log D24), so no separate served file exists.
- **`MANIFEST.json`** has sorted keys. It lists:
  - the format, the snapshot, and its content and compiler digests;
  - the spec hash (none for a lexical-only generation);
  - per file, its sha256, rows and **serving schema digest**. That digest is a SHA-256 over a
    language-neutral canonical form: field name, a declared type grammar, nullability and sorted
    metadata. Python recomputes it with its standard library, and the known answers are shared
    by the Rust and Python tests (`specs/serving/schema_digests.json`). It is not the store's
    `canonical_schema` (§6.3), whose Rust type display Python cannot reproduce (ADR-0019).
  - a coverage summary: coverage by scope, family and status; boundaries by reason; invocations
    by completion; briefs by review state and analysis backing; unresolved slots by section (the
    §B11 gap metric).
  - The **generation key** is the first 16 hex digits of the SHA-256 of the manifest without its
    key. It moves with any file, and with the snapshot's provenance.
- **Normalization** (slice 1.7), so a rebuild is byte-identical:
  - each file is one query, sorted by its declared key;
  - every column is cast to its declared type and rebuilt through a builder, so no view type,
    scan metadata or byte under a null slot reaches the file;
  - one record batch per file, written in the Arrow IPC file format: V5, 64-byte alignment,
    uncompressed.
  - One generation never mixes vector spaces: a snapshot with two specs is refused, by
    `semantic:one-embedding-spec` and by the builder.
  - **Tested** (2026-09-23): `a_generation_rebuilds_to_the_same_bytes` rebuilds from the store,
    and from a second compile in another location and module order. Also
    `a_changed_generation_is_refused`, `mixed_embedding_specs_are_refused` and
    `serving_schema_digests_are_the_shared_known_answers`.
  - `lctx compile` builds the generation after publishing; `lctx bundle` rebuilds it.
  - A reader session registers its own `snapshots` rows, so the manifest's digests are read
    from the store (C6 review O5).
- **Activation.**
  - The bundle is smoke-queried by the serving code's own test entry point.
  - Then the `generations/active` symlink is switched by atomic rename.
  - A running server keeps the generation it loaded; restarting it picks up the new one.
- **`FORMAT` 3** (ADR-0010 amendment, 2026-09-24; Stage 1's files **Implemented** and **Tested** by
  `a_generation_rebuilds_to_the_same_bytes` and the shared schema digests; later files **Proposed**)
  keeps every `FORMAT` 2 file and
  adds:
  - `operations`: each public operation's paths, signature, docstring summary and facets
    (Stage 1);
  - `behaviors`: the `behavior` family's rows for public operations, with evidence and verdicts
    (Stage 1);
  - `conditions`, `concepts`, `concept_members` and `vocabulary` (Stages 2–4);
  - per-view `vectors`, whose rows carry their view.

  Derived search indexes (an FTS table, any ANN index) are rebuilt from the generation's Arrow
  files, keyed by the generation key, outside the byte-identical manifest.

> Decision: ADR-0017 (superseding ADR-0009), ADR-0012, ADR-0019, ADR-0014, ADR-0010

---
