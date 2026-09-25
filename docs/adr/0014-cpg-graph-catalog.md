---
id: ADR-0014
title: The CPG is typed family tables plus derived node and edge catalogs; every edge has a persistent id and every endpoint is a typed node
status: superseded
date: 2026-09-22
supersedes: [ADR-0008]
superseded-by: ADR-0047
design: [§1.2, §B6, §3.1, §3.2, §3.4.1, §3.5, §3.7, §3.8, §4.3, §6.1, §8]
evidence: Tested
revisit: A pass needs a relationship that neither a family table nor the edge catalog can express without a second writable copy (carried from ADR-0008); two runs over the same inputs give a different node_id or edge_id; an endpoint rule or lineage rule can pass vacuously (its target built from its own source column); or `nodes`/`edges` derivation dominates compile time or memory on the pilot.
---

## Context

ADR-0008 made the typed fact-family tables the only writable authority, with DataFusion
derivations over them. It declared the family → node/edge mapping but deferred it, and the
generic views with it, "until the first projection". Slices 1–3 built five families on that
basis. No node kind, edge kind or edge identity exists in code.

On 2026-09-22 the operator added the Rust code-intelligence guidelines
(`docs/design_review/design_principles/rust_code_intelligence_data_graph_guidelines.md`) and
decided to **complete the CPG before the analytics** (slices C1–C6; ADR-0004 amendment). The
guidelines' MUSTs that the current store does not meet:

- **§2.** Canonical relationships are first-class facts with a persistent `edge_id`, typed
  endpoints, a relation kind, evidence, and snapshot scope. `(src, dst)` is not an identity.
  Isolates survive through an explicit node relation. Today:
  - relationship rows (`call_targets`, `exports`, `signatures`, `class_ancestry`) carry no edge
    id or kind;
  - endpoints are implicit in column names;
  - a call target outside the release is a null node with a null reason.
- **§2.** Extracted, resolved, derived and heuristic information stay distinguishable, and
  unknown targets are explicit. Boundary facts are attributed to the source surface although
  they compare two providers (slice-1 review O4).
- **§11.** Retention must keep the files that live snapshots need. delta-rs's defaults (a
  checkpoint every 100 commits, then expired-log cleanup after 30 days) delete the log a pinned
  read of an old snapshot needs. Read in the pinned sources, 2026-09-22:
  `kernel/transaction/mod.rs:1064-1104`, `protocol/checkpoints.rs:114-214`, kernel
  `log_segment/mod.rs:372-419`.
- **§12.** Known-answer graph shapes must be tested: isolates, parallel edges, self-loops,
  reconvergent paths, cross-file cycles, unresolved targets, mixed configurations. So must source
  lineage after every transformation.

## Options

1. **The simpler alternative: keep ADR-0008.** Each relationship table gains an `edge_id` column
   and nothing else, and projections read the family tables directly. It loses:
   - there is no node relation, so isolates and endpoint kinds stay implicit;
   - every consumer re-derives the same union of relationship tables;
   - the endpoint and lineage rules would be written per table, by hand.
2. **Generic `nodes`/`edges` as the writable authority**, with a property bag (the research
   input's store). Rejected again, for ADR-0008's reason: signatures rebuilt from string keys, and
   untyped relationship attributes.
3. **Chosen: derived catalogs.** The typed family tables stay the only writable authority, and
   `nodes`/`edges` are Stage-D derived tables generated from one declared registry. They carry
   identity, kind, endpoints and evidence, never a payload. The family tables are the guidelines'
   "typed extension tables".

## Decision

Carried forward from ADR-0008 unchanged:
- one writable authority per fact, one producer per table;
- merged records as derivations that carry keys, the `fact_id`s they join and what the join
  decides;
- append-only `Int16` codebooks;
- `coverage` and `boundaries`;
- validation generated from the contracts, read-only, shared by tests and publication;
- total `ORDER BY`, and `safe: false` casts.

New or changed:

- **Families land by CPG slice** (ADR-0004 amendment), each naming the consumer, pass and
  columns that read it:
  - C1: the catalogs over the existing families;
  - C2: `syntax`;
  - C3: `lexical`, including our recognizer's full Python name resolution;
  - C4: `types`, including type structure and record fields;
  - C5: `docs`, and the examples and tests usage corpus.

  CFG, dataflow and alias tables stay deferred (§1.3).
- **Catalogs, family `graph`.** A new `FactFamily` code, excluded from coverage like
  `publication`.
  - `nodes(snapshot_id, node_id, node_kind, module_node_id?, existence_fact_id)`.
  - `edges(snapshot_id, edge_id, edge_kind, src_node_id, src_kind, dst_node_id, dst_kind,
    ordinal?, evidence_fact_id, support_fact_id?)`.
  - `ordinal` is key-bearing, as in `parameters`. `src_kind`/`dst_kind` are what the join with
    `nodes` decides.
  - Edges get no `facts` rows. Their edge id, evidence `fact_id`s and the snapshot's
    `compiler_digest` trace them, as they do every derived row.
- **Edge identity.** `edge_id = H('edge', edge_kind, src, dst, discriminator…)`.
  - It is content-derived and stable across runs and snapshots, and never contains a `fact_id`,
    which is run-scoped.
  - Parallel call sites stay distinct because the source is the call site.
  - Where several provider rows join one site, the discriminator is the provider row's
    run-independent payload digest (for Pysa call rows, `pysa_calls.payload_id`).
- **One node, one kind, one id.**
  - A role gets its own kind tag derived from its carrier: argument =
    `H(argument, call node, ordinal)`, reference = `H(reference, name syntax id)`, parameter as
    today.
  - Syntax already represented as a declaration, call site or parameter is never re-emitted
    under a second id.
  - `key:nodes` rejects a collision.
- **Every endpoint is a typed node.**
  - `external_module` and `external_symbol`: outside the analyzed universe. The identity includes
    the owning distribution and version, or Pyrefly's bundled typeshed and the producer revision.
    The existence source is a raw `context_definitions` table: the Pysa definitions of each
    dependency module the release references (probe P1 below). A `declared_in` edge links symbol
    → module.
  - `synthetic_callable`: a Pysa function with no `def`.
  - An unresolved target produces **no** edge. It stays explicit on `resolutions` (whose id is the
    call-site id, §3.6), on `argument_resolutions` (higher-order arguments), and on `boundaries`.
  - The §3.2 meaning "a null node with a null reason is outside the release" is retired.
  - **A reason only where a provider gives one.**
    - Stage C's own reason, or, for an export, Pyrefly's symbol kind of the origin
      (`variable_origin` for variable-like kinds; `missing_evidence` when there is no origin or
      no kind).
    - Any other unmapped target keeps a null reason, and a `typed:*` rule rejects the snapshot. No
      derivation has a catch-all reason.
- **The registry, `cpg_schema::graph`.**
  - Per node kind, one existence source **independent of the columns that reference it**.
  - Per edge kind:
    - its source relation and columns;
    - the allowed endpoint kinds;
    - its direction meaning;
    - whether parallel edges are allowed;
    - its derivation class (`extracted`, `analyzer`, `joined`, `recognizer`);
    - its evidence columns, ordinal and discriminator.
  - Generated from it:
    - the `nodes`/`edges` SQL;
    - the node-valued references, one per `node_columns` entry, checked against `nodes` with
      their kinds (closing slice-2 review O8);
    - the endpoint-kind, evidence and support rules;
    - the lineage rules from raw rows (every raw row yields its declared edges, or its derived
      row carries a provider's reason);
    - the partition of `pysa_calls` (lineage, counted remainder, or published gap);
    - the `edge_kinds` table (each kind's derivation class, direction, parallel policy, evidence
      table and endpoint kinds, published per snapshot);
    - the `graph_gaps` table (the raw rows the graph does not represent yet, with their reason
      and the slice that will).
  - A rule whose target is built from its own source column is never generated, because it
    cannot fail.
- **Ids in SQL and in Rust.** The Rust-computed ids whose inputs are also columns (argument,
  external module, external symbol) are `cpg_schema::id::recipe` functions in the `opt_*`
  encoding. A generated `id:*` rule recomputes each in SQL on every compile.
- **The UDF.** One scalar UDF, `lctx_id(kind, …)`, registered in every session.
  - It implements `IdHasher` exactly: the kind through `new`, every other argument in the `opt_*`
    encoding.
  - It accepts Utf8, Int16/Int64, Binary and Boolean families only, and rejects UInt64, Int32 and
    floats at plan time.
  - Known-answer vectors are shared with the Rust `IdHasher` tests. Its version is part of
    `compiler_digest`.
- **Retention.** Every table, `snapshots` included, is created with
  `delta.enableExpiredLogCleanup = false` and `delta.logRetentionDuration = interval 36500 days`.
  `verify` compares the stored strings exactly, as it does `delta.appendOnly`, because a value
  delta-rs cannot parse silently falls back to its default. Checkpoints stay on.
- **Boundary provenance.** Boundary facts are the extractor's comparison of two surfaces: surface
  `compare`, `relational_derivation` (closing slice-1 review O4).

## Consequences

- **Projections become a selection** over `nodes`/`edges` by kind, derivation class and evidence.
  The node relation keeps isolates, and the edge id keeps parallel sites. The
  `GraphProjectionSpec` itself (guidelines §3, DESIGN §5) is written with its first consumer
  (increment 1, slice 4).
- **The cost.**
  - Two derived tables, whose size is of the order of the relationship rows they index.
  - Dependency definitions add a second Pyrefly check over referenced dependency modules.
    **Tested** (probe P1, FastMCP 4.0.5, release build, 2026-09-22): 301 dependency modules,
    1,276 referenced functions. Every module resolved by `import_handle` to the handle Pysa had
    numbered, and every referenced function was found with its name and defining class. The
    added cost was 3.6 s for the check at `Require::Everything` plus 1.1 s for the collectors.
- **Schema migrations.** The following are contract changes, snapshot-reviewed:
  - `arguments.node_id`;
  - `pysa_calls.payload_id`;
  - typed (module ref, key) pairs beside Pysa's reference strings;
  - `pysa_classes` (a class without bases has no `class_ancestry` row);
  - `source_files.distribution` (closing ADR-0013's deferred module → distribution row);
  - `context_modules` and `context_definitions`.

  Old stores are rebuilt.
- **Oracles** (C1, `cpg-core/tests/graph.rs`):
  - the `graph_shapes` fixture's catalogs as insta snapshots;
  - two parallel sites give two edge ids;
  - the isolate is present;
  - an unresolved call gives no edge, plus its reason and boundary;
  - identical catalogs across reruns, reversed module order and moved environments;
  - every registry rule rejects an injected violation;
  - UDF known answers;
  - a retention test (an old version unloadable under delta-rs's defaults, loadable under ours);
  - a graph-readiness reader that builds a petgraph projection and checks isolates, parallel
    edges, the self-loop, a cross-file SCC and each arc's lineage.
- **C1 as built** (2026-09-22). Three shapes chosen while building, each recorded in DESIGN:
  - `context_definitions` carries each external symbol's node id, computed in the extractor, so it
    is itself the existence source and no derived `external_symbols` table exists;
  - an external symbol is keyed by its external module, definition kind and Pysa key, because
    conditional definitions can share a qualified name;
  - bases, MRO entries and overridden methods resolve through `ancestry_targets` and
    `override_targets`, the same typed-target shape as `call_targets`, so every unresolved end
    keeps a reason.
- **C1 evidence.**
  - **Tested:** `just check` passes (70 tests), with the `graph_shapes` oracles above,
    `retention_keeps_old_versions_loadable`, and the UDF known answers.
  - **Measured** (FastMCP 4.0.5, release build, 2026-09-22):
    - 47,145 nodes and 65,176 edges, every rule passing, and every call, ancestry and override
      target typed;
    - 13.6 s at 2.53 GB peak RSS (before: 7.9 s and 1.61 GB), the difference being the
      dependency check and definitions;
    - one `content_digest` across two runs.
- **Standard review** (2026-09-23,
  `docs/design_review/reviews/design_review_adr-0014-cpg-graph-catalog_2026-09-22.md`). It
  recommended Revise:
  - F1: catch-all reasons made the typed rules unfalsifiable;
  - F2: `exports` edges had no discriminator, so `attrs` 26.1.0 could not publish;
  - F3: Rust and SQL used two id encodings;
  - F4: slice-2 O8 was not closed;
  - F5: undeclared pending rows;
  - F6: unused registry fields.

  All six were fixed as the Decision now reads, each with its oracle:
  - raw mutations of a release and of a dependency target fail `typed:call_targets`;
  - a `.py`/`.pyi` re-export pair in `graph_shapes` gives two edges, and `attrs` 26.1.0
    publishes;
  - Rust = SQL known answers, and the `id:*` rules;
  - the generated `ref:*->nodes` rules;
  - `graph_gaps` and the partition rules;
  - `edge_kinds` in the derivations snapshot.

  On the pilot every tightened rule passes: 261 rules, no reason left to a catch-all, 17,282
  published gaps.
- **Superseded.** ADR-0008's deferral of the family → node/edge mapping and its generic views. All
  its other decisions are carried forward here.

## Amendments

- 2026-09-23: C6 deep review F1. The `revisit:` trigger "a lineage rule can pass vacuously" fired
  at C3 and went unrecorded. The C3 review (O2) answered it: a lineage rule that re-reads its edge
  kind's own unfiltered source guards an edit of that edge's SQL, not a data condition, and is
  kept as such. Seventeen lineage rules and `partition:pysa_calls-gaps` (empty by construction
  since C3) are these **edit guards**. They are declared in `cpg_schema::rules::EDIT_GUARDS` and
  counted apart in DESIGN §8. The Decision's "a rule whose target is built from its own source
  column is never generated" holds for references only. `every_rule_is_exercised_or_declared_an_edit_guard`
  requires every other hand-written rule to reject an injected violation, and every generated
  template to have a case. The deferred `higher_order_index` row of this ADR's standard review is
  closed: its trigger fired (229 of 1,786 higher-order argument sites follow a keyword or starred
  argument on the pilot), and Pysa numbers arguments with the same `iter_source_order().enumerate()`
  as `arguments.ordinal` (Pyrefly `6a93da3`, read by the C6 review).
- 2026-09-23: scope sentence, pointer only. ADR-0019 decides that analysis results (findings,
  assertions, briefs) are not `facts` rows either: they carry their provenance in-row and stay
  outside the `nodes`/`edges` catalogs, which Stage D derives before analysis runs. `facts` rows
  are the extracted assertions and operator review verdicts.
