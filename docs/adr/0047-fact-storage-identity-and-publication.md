---
id: ADR-0047
title: Typed family tables with derived graph catalogs, content identity, snapshot publication and in-row analysis provenance
status: accepted
date: 2026-09-25
supersedes: [ADR-0012, ADR-0013, ADR-0014, ADR-0017, ADR-0019]
superseded-by: null
design: [§1.2, §B2, §B6, §B7, §B12, §3.1, §3.2, §3.3, §3.4, §3.4.1, §3.5, §3.7, §3.8, §4.0, §4.1, §4.3, §5, §6, §8, §9, §10]
evidence: Tested
revisit: A pass needs a relationship that neither a family table nor the edge catalog can express without a second writable copy; two runs over the same inputs give a different node_id or edge_id; `nodes`/`edges` derivation dominates compile time or memory on the pilot; one attempt writes a snapshot-qualified table in more than one commit, or a reader needs rows of a table across snapshots; an injected failure test finds a reader seeing unpublished rows; a consumer needs a finding, assertion or brief as a catalog node or edge; an analysis result cannot be rebuilt from its snapshot, analytics config and compiler digest; the language-neutral serving schema form and the store's canonical schema diverge in a way a reader notices; or the first schema change that must keep older snapshots readable.
---

## Context

This record replaces ADR-0014, ADR-0017 and ADR-0019 with their surviving clauses, together with
the storage and identity-encoding clauses of ADR-0012 and ADR-0013 (whose front-end and
acquisition clauses are ADR-0046's). It governs what is authoritative in the store, how every
node, edge, fact and result is identified, how an attempt is published and read, and where
analysis results live ([§3](../design/sections/facts-and-identity.md#section-3),
[§5–§6](../design/sections/storage-and-publication.md#section-5), §B2, §B6, §B7, §B12).

The forces:
- **One authority per fact, with typed payloads.** Signatures, resolutions and types have rich,
  typed structure that a generic property bag would reduce to strings; merged records need to
  keep both providers' evidence.
- **A graph with explicit shape.** Consumers need isolates, parallel call sites, typed endpoints
  and explicit unknown targets, each with lineage back to a provider row. `(src, dst)` is not an
  identity.
- **Reproducibility and comparison.** Two runs over the same inputs must produce the same ids, and
  results must survive a parameter change so that an ablation is a join.
- **Delta's properties.** Commits are atomic per table only; log statistics skip Binary columns,
  so a `snapshot_id` filter alone skips no file; default log cleanup deletes the commits a pinned
  read of an old snapshot needs; DataFusion `INSERT INTO` bypasses CHECK constraints; and
  `delta.constraints.*` cannot be set at create.
- **Analysis results are rebuildable, not inputs.** Findings, assertions and briefs are
  determined by the snapshot, the analytics configuration and the compiler; `facts` is committed
  once per attempt, before any derivation.

## Options

1. **Generic `nodes`/`edges` as the writable authority**, with a property bag. Rejected: signatures
   rebuilt from string keys, untyped relationship attributes.
2. **Family tables only, each relationship table gaining an `edge_id`.** No node relation, so
   isolates and endpoint kinds stay implicit; every consumer re-derives the same union; endpoint
   and lineage rules are written per table by hand.
3. **Typed family tables as the authority, with derived `nodes`/`edges` catalogs generated from
   one registry** (chosen).

For reading a snapshot: a `snapshot_id` filter over every file (correct, but each read costs one
footer per file of every snapshot, growing with the store); partitioning by `snapshot_id` (as
Binary it silently returned wrong results; as hex it migrates every contract); or **reading each
snapshot's own commit and checking that it is** (chosen).

For analysis results: `facts` rows (restructures the attempt, undoes releasing raw batches after
writing, and mixes inputs with derived output); nodes and edges of the catalogs (a second graph
authority, since the catalogs are derived before analysis); or **typed analysis tables with
in-row provenance, outside the catalogs** (chosen).

## Decision

**Authority.**
- The typed family tables are the only writable authority, one producer per table; `runs`,
  `contexts`, `producers` and `facts` are registries each producer appends to. A family is added
  with the consumer, pass and columns that read it.
- A merged record is a Stage C/D DataFusion derivation that carries keys, the `fact_id`s it joins
  and what the join decides, never a copied raw payload. A derived row is not a `facts` row: it is
  traced by the fact ids it cites and its snapshot's `compiler_digest`, and rebuildable from them.
- `facts` rows are the extracted assertions and operator review verdicts (a `manual-review`
  producer's facts, written in the raw write). Independent assertions are kept, including
  disagreement.
- Codebooks are append-only `Int16`. `coverage` and `boundaries` state what was analyzed and where
  it stopped; missing output is never negative evidence. Boundary facts that compare two surfaces
  are surface `compare`, `relational_derivation`.
- **Every endpoint is a typed node.** Targets outside the release are `external_module` and
  `external_symbol` nodes, whose identity includes the owning distribution and version, or
  Pyrefly's bundled typeshed and the producer revision; their existence source is the raw
  `context_definitions` table, linked by `declared_in`. A function with no `def` is a
  `synthetic_callable`. An unresolved target produces no edge and stays explicit on
  `resolutions`, `argument_resolutions` and `boundaries`. A reason appears only where a provider
  gives one; any other unmapped target keeps a null reason and a `typed:*` rule rejects the
  snapshot. No derivation has a catch-all reason.

**The graph catalogs.** `nodes(snapshot_id, node_id, node_kind, module_node_id?, existence_fact_id)`
and `edges(snapshot_id, edge_id, edge_kind, src_node_id, src_kind, dst_node_id, dst_kind,
ordinal?, evidence_fact_id, support_fact_id?)` are Stage-D derived tables of family `graph` (not a
coverage unit). They carry identity, kind, endpoints and evidence, never a payload; edges get no
`facts` rows.
- **The registry** (`cpg_schema::graph`) declares, per node kind, one existence source
  independent of the columns that reference it, and per edge kind its source relation and
  columns, allowed endpoint kinds, direction meaning, parallel policy, derivation class
  (`extracted`, `analyzer`, `joined`, `recognizer`), evidence columns, ordinal and discriminator.
- **Generated from it:** the catalog SQL; node-valued references (one per `node_columns` entry,
  checked against `nodes` with their kinds); endpoint-kind, evidence and support rules; lineage
  rules from raw rows; the partition of `pysa_calls`; and the published `edge_kinds` and
  `graph_gaps` tables.
- **Rules must be able to fail.** A reference whose target is built from its own source column is
  never generated. A lineage rule that re-reads its edge kind's own unfiltered source guards an
  edit of that edge's SQL rather than a data condition; it is kept as a declared edit guard
  (`cpg_schema::rules::EDIT_GUARDS`, counted apart), and every other hand-written rule and every
  generated template must reject an injected violation.
- One node has one kind and one id. A role gets its own kind derived from its carrier (argument,
  reference); syntax already represented as a declaration, call site or parameter is never
  re-emitted under a second id. `key:nodes` rejects a collision.

**Identity.**
- Ids are `BLAKE3("lctx-id/v1" ‖ length-prefixed kind tag ‖ length-prefixed fields…)`, the first
  16 bytes; digests are all 32. `cpg-schema` depends only on `arrow-*` and `blake3` (the crate
  Pyrefly pins, so one version).
- `node_id`, `fact_id` and `edge_id` are content-derived. `edge_id = H('edge', edge_kind, src, dst,
  discriminator…)`, where the discriminator is an ordinal or, where several provider rows join one
  site, the provider row's run-independent payload digest; it never contains a `fact_id`, which is
  run-scoped. Parallel call sites stay distinct because the source is the call site.
- `snapshot_id` is a fresh random value per attempt, and keys are snapshot-qualified, so an
  identical rerun re-emits the same ids in a new snapshot.
- Ids computed in Rust whose inputs are also columns are `cpg_schema::id::recipe` functions in
  the `opt_*` encoding, and a generated `id:*` rule recomputes each in SQL on every compile through
  one scalar UDF, `lctx_id(kind, …)`, registered in every session. It implements `IdHasher`
  exactly, accepts only Utf8, Int16/Int64, Binary and Boolean families (rejecting UInt64, Int32
  and floats at plan time), shares known-answer vectors with the Rust tests, and its version is
  part of `compiler_digest`.
- **Analysis-result ids** are content-derived with **no analytics-config digest**, so an unchanged
  result keeps its id when parameters change. Each analysis contract declares identity columns and
  lineage columns (ids such as `edge_id` and `invocation_id`, and values such as scores); a
  finding's id hashes its identity columns plus its witness steps (call site, callee, modality,
  arc kind, phase) and members; an evidence id hashes the resolved text, with a cited `fact_id`
  as lineage. These recipes are Rust-only, each with a property test that an identity column
  changes the id and a lineage column does not. `capability_id = brief_id`; a changed brief is a
  new brief. Two seeds that name one declaration refuse the compile.
- `content_digest` takes the attempt's sorted run ids, the compiler digest, and, when embeddings
  are used, the spec hash and a digest of the sorted `(spec_hash, input_hash)` keys used; never the
  shared cache's version, which another library's compile can move.

**Physical storage.**
- Only immutable per-row CHECKs (span order, non-negative offsets) are added, with
  `add_constraint()`; codebook membership stays a validator, because codebooks grow.
- Every table, `snapshots` included, is created append-only with
  `delta.enableExpiredLogCleanup = false` and `delta.logRetentionDuration = interval 36500 days`,
  set property by property; checkpoints stay on.
- When a table is opened, `verify` compares its schema with the declared contract and its CHECK
  set, `delta.appendOnly` and retention properties with the declared strings exactly, and refuses
  any difference. There is no in-place schema evolution; a contract change is a reviewed
  migration. A migration or rebuild policy that keeps older snapshots readable in one store is an
  **open choice**, not decided here
  ([plan W15](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- Snapshot-qualified tables are written only through `DeltaTable::write`, in exactly one commit
  per table per attempt, recording `lctx.snapshot_id` in `commitInfo`. DataFusion `INSERT INTO`,
  `write_table` and delta-rs's low-level writers are never used on fact tables. Ids are read back
  through a two-step `safe: false` cast. Every published, compared or snapshot-tested relation has
  a total `ORDER BY`. Validation is generated from the contracts, read-only, and shared by tests
  and publication.

**Publication.** Attempts abort on any error, and a retry is a new attempt with a new
`snapshot_id`. After validation passes, one append to `snapshots` (one row per table: version,
schema digest, compiler digest, row count, content digest) is the only publication act; an
ambiguous append is classified by re-reading `snapshots`. No `SaveMode::Ignore`, vacuum or
optimize on fact tables. Serving generations are built only from a published snapshot, and are a
derived, byte-reproducible projection of it.

**Readers.** Each table declares a read mode in `cpg-schema`, and every reader honours it.
- A **snapshot-qualified** table is read at its recorded version, opening only the files that
  version's commit added; the commit's `lctx.snapshot_id` must name the snapshot being read
  (`ForeignCommit` otherwise, never an empty read); a missing file fails the scan; then
  `snapshot_id` is filtered and columns projected by name. `snapshots` stays the authority for
  which version.
- A **global** table that accumulates across attempts (`embedding_cache`) has no `snapshot_id`,
  is read at its recorded version over all active files, and may be committed more than once per
  attempt. It is written by an insert-only MERGE on its key with a retry on conflict, after the
  local type, length, finiteness and norm checks. The snapshot row records the version read after
  the last write and the rows the snapshot used. Key uniqueness is a validation rule; its declared
  recovery is a migration to a new table keeping the earliest-version row per key, and if two
  concurrent merges could ever leave two rows, the fallback is an exclusive store lock around an
  anti-join and append.
- An unpublished attempt is readable only for inspection, at the commits carrying its own
  `lctx.snapshot_id`; tables it did not write are left out.

**Analysis results.** Family `findings` is its own table group (`for_each_analysis_table!`),
with contracts in `cpg_schema::findings`: `analysis_invocations` (method, run and model,
parameters as canonical JSON with their digest, projection digest, library versions, seed,
diagnostics, completion), `findings`, `finding_members`, `witnesses` (keyed step by step; the
`edge_id` is lineage), `evidence` (resolved text, because the server reads no Delta),
`assertions`, `assertion_support` (role `support` or `scope`), `briefs` (`review_state` outside
the id), `brief_assertions`, `brief_members`, `brief_documents` (with `spec_hash`),
`embedding_specs` and `assertion_policy`, with further tables joining the group as their producers
land.
- **Provenance in-row.** Analysis tables have no `fact_id` column (a cited fact is
  `cited_fact_id`); their `run_id` and `model_id` are checked by generated rules. The run is the
  `lctx-compiler` run over the library release: its config digest is the analytics
  configuration's, its tool revision the `compiler_digest`, its `runs` and `producers` rows go in
  the raw write, and it declares no families; its completion lives in `analysis_invocations`.
  Without an analytics configuration there is no compiler run and the analysis tables are written
  empty, so "not requested" is readable.
- **Outside the catalogs.** Results reference catalog nodes and edges by id and never enter
  `nodes`/`edges`.
- **Completion.** The depth bound and the stated subsystem, dependency and synthetic boundaries are
  the model: a result inside them is complete under it. A vertex or arc budget is operational
  truncation (the invocation is `partial` with its stop reason, and synthesis states it as a
  limit); the witness cap is presentation only.
- **Composition.** Stage E composes in memory: later methods read earlier methods' rows from
  memory, never from Delta, and each analysis table is written once, after synthesis, in the
  attempt's one commit. A finding kind has one method. A relation computed from the snapshot's
  facts and the analytics configuration alone (such as `public_paths`) is an analysis table too,
  never a `facts` row.
- **The serving schema digest** is SHA-256 over a language-neutral canonical form (field name,
  declared type grammar, nullability, sorted metadata) that Python recomputes with its standard
  library, with shared known answers; the store's canonical schema digest is unchanged.
- **Crates.** `lctx-analytics` holds the projection adapter and analysis kernels, Arrow in and
  Arrow out, with no DataFusion or Delta; `lctx-embed` holds the embedding client behind a trait
  `cpg-core` declares; Stage F lives in `cpg-core::synth`.

## Consequences

- Projections are selections over the catalogs by kind, derivation class and evidence, so
  isolates, parallel sites and unknown targets survive without a second authority. The catalogs
  cost two derived tables of the order of the relationship rows they index (on the pilot,
  2026-09-23: 0.5 s and 1.0 s of a 29.9 s compile).
- Reads cost what the snapshot holds, not what the store holds, and old snapshots stay loadable at
  their recorded versions. A future writer that splits a table's write across commits must record
  all of them, or the reader contract changes.
- Analysis ablations are joins on content ids across snapshots; every new analysis table gets its
  generated key, reference and codebook rules through its table group.
- Contract changes are migrations that today need a fresh store (W15). The cache-fill path has two
  entry points with different admission and one returns locally computed rather than committed
  vectors ([plan W9](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- **Evidence (Tested, 2026-09-22 onward):** `cpg-core/tests/graph.rs` (graph shapes, parallel
  edges, isolates, unresolved calls, identical catalogs across reruns, module order and
  locations, every registry rule rejecting an injected violation, `retention_keeps_old_versions_loadable`);
  `cpg-core/tests/delta.rs` and `compile.rs` (CHECK and append-only enforcement, the `INSERT INTO`
  bypass, schema drift refused, validation failure publishes nothing, ambiguous append classified,
  `a_pinned_read_opens_only_its_commits_files`, `reads_pin_the_version_and_filter_the_snapshot`,
  `a_rejected_attempt_is_inspected_at_its_own_commits`); UDF and recipe known answers;
  per-contract identity property tests; shuffled input giving byte-identical analysis tables;
  `every_rule_is_exercised_or_declared_an_edit_guard`; the serving schema digests' shared known
  answers.
