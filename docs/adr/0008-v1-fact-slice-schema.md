---
id: ADR-0008
title: Authoritative fact-family tables, declared view mapping, codebooks, coverage and boundaries
status: proposed
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§3.2, §3.5, §3.7, §8]
evidence: Proposed
revisit: A pass needs a relationship that neither a family table nor the derived edge view can express without a second writable copy.
---

## Context

Baseline review findings:
- **F1:** column authority was handed to an empty crate. The research input's two parts disagree
  (`edge_kind` is `utf8` in one and `Int16` in the other), and `extraction_mode`, `modality`,
  `model` and the resolution `status`/`domain` had no values anywhere.
- **F5:** coverage and "resolution issues" had a vocabulary but no table.

The research input proposes both a generic `nodes`/`edges` store and dedicated tables
(IP L469–L537, L1102–L1117) without saying which is authoritative. That is a G1 risk.

## Options

1. **Generic `nodes`/`edges`/`facts` as the only authority,** with a property bag. Rejected:
   agents and passes would rebuild signatures from string keys (IP L1115), and relationship
   attributes (argument keyword, invocation phase) would be untyped.
2. **Both generic and dedicated tables writable.** Rejected: two authorities for the same fact
   (DM-02).
3. **The simpler alternative, chosen:**
   - typed fact-family tables are the only writable authority, with one producer per table;
   - merged records are DataFusion derivations carrying both `fact_id`s;
   - `cpg-schema` declares the family → node/edge mapping for endpoint validation, but the
     generic views are **not materialized** until a pass reads them;
   - families are introduced only when a consumer lands.

## Decision

- **Families by increment.** The family table in DESIGN §3.2 is binding:
  - increment 1: provenance, exports, signatures, calls, coverage, findings (including
    `evidence` and `assertion_support`), and the global `embedding_cache`;
  - increment 2: syntax, lexical;
  - increment 3: types, docs.
- **One term.** A fact family is also the Delta table group, the coverage unit and the unit an
  extractor declares.
- **Codebooks.** The values in DESIGN §3.5 are binding and **append-only `Int16`**. They fill the
  research input's unsourced gaps:
  - `extraction_mode`, `modality`, `invocation_phase`, `evidence_status`;
  - `resolution_status`, `resolution_domain`, `pysa_unresolved_reason`;
  - `boundary_reason`, including `provider_disagreement`.

  `model_id` is a validated string, `<producer_id>/<surface>`. The increment-1
  `assertion_kind`s and their policy are fixed in DESIGN §10.2.
- **Coverage.** `coverage` has one row per declared family and module in scope; absence is never
  implicit.
- **Boundaries.** `boundaries` holds the extractor's resolution issues and analysis stops, keyed
  by fact. A derivation (Stage C/D) keeps an unmapped or disagreeing row in its own table, with a
  null node and a `reason` column, so each table keeps one producer.
- **Derived rows.** A derived table carries keys, the `fact_id`s of the rows it joins and what the
  join decides, never a copy of a raw payload column. Its SQL lives in `cpg-schema` next to its
  contract and is snapshot-tested with it.
- **Producers.** A fact table has one producer. `runs`, `contexts`, `producers` and `facts` are
  registries each producer appends its own rows to; `source_files` is the extractor's until
  Stage A lands.
- **The family → node/edge mapping** and the endpoint-kind rule land with the first projection
  (§5), their first reader.
- **Validation** (DESIGN §8), generated from the contracts:
  - one DataFusion query per rule: key uniqueness, declared references, fact links in both
    directions, codebook membership and coverage completeness;
  - snapshot-qualified uniqueness;
  - total `ORDER BY` everywhere;
  - `safe: false` casts;
  - read-only, and shared by tests and publication.

## Consequences

- **The research input's 66 tables** become a target to grow into, not a prerequisite.
  `record_fields` and the full type graph wait for a consumer.
- **Schema snapshots** (insta) and the append-only codebook test become the oracles for this ADR.
- **Endpoint-kind validation** reads the mapping. Materializing views is deferred, so there is
  no regeneration contract to maintain until a reader exists.
- **Evidence** (slice 2, 2026-09-22, `cpg-core/tests/compile.rs`): each rule kind rejects an
  injected violation and nothing publishes; the Stage-C key is unique on three fixtures,
  including a `.py`/`.pyi` pair.
