---
id: ADR-0007
title: Multi-distribution releases, explicit run contract, BLAKE3 v1 identities
status: accepted
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§3.4.1, §4.0]
evidence: Proposed
revisit: A proptest or rerun shows the same inputs producing different node_id or fact_id, or an analyzer answer changes without context_id changing.
---

## Context

Baseline review finding **F4**: the run's identity inputs and the per-ID derivation were
undeclared.
- A rerun under a new Pyrefly revision could reuse `fact_id`s.
- Node IDs could be snapshot-scoped, which would make cross-snapshot comparison impossible.
- Analyzer config discovery (upward `pyproject.toml` search) was an undeclared input (G4).

FastMCP 4.0.3 is also three distributions, not one: the `fastmcp` facade, `fastmcp-slim` (which
holds the code) and `fastmcp-tasks`.

## Options

1. **The simpler alternative: content-hash everything, including `snapshot_id`,** so an identical
   rerun gets the same snapshot. Rejected: the rerun would re-append identical keys to the same
   snapshot, and uniqueness validation fails. It also conflates execution identity with content
   (DM-12).
2. **Random IDs everywhere.** Rejected: there would be no stable identity across snapshots
   (DM-11).
3. **Content-derived `node_id` and `fact_id`, snapshot-qualified keys, a per-attempt random
   `snapshot_id` plus a `content_digest`.** Chosen.

## Decision

- **Releases.** A release is a set of distributions, each with its artifact sha256. Code comes
  from the wheel bytes. Docs, examples and tests come from the tag tarball (sha256), with a path
  map. Acquisition follows the fastmcp skill's pattern (a locked manifest plus
  `ACQUISITION.json`).
- **Context.** A uv venv built from a lock. The context records the Python version, platform,
  ordered search paths, lock digest and config digests. **Extractors receive their environment
  and config only as arguments.** Analyzers get generated configs, so discovery never runs.
- **ID encoding.** BLAKE3 over `"lctx-id/v1"`, a kind tag and u64-LE length-prefixed fields. IDs
  are the first 16 bytes; digests are all 32. The derivation per ID is in DESIGN §3.4.1:
  - `run_id` includes `release_id`;
  - finding, assertion, brief and evidence IDs are content-derived, with **no config digest**, so
    ablation diffs are joins and `capability_id = brief_id` is stable when content is;
  - the compiler is itself a run (producer `lctx-compiler`), and the analytics config is its
    config;
  - an overloaded callable is one declaration node with one signature per overload.
- **Snapshot identity.** `snapshot_id` is random per attempt. `content_digest` covers:
  - the sorted run ids;
  - the acquisition-manifest digest;
  - the compiler build;
  - the analytics config;
  - the embedding spec;
  - the `embedding_cache` version.

  Reruns can be compared with it.
- **Keys.** All keys are snapshot-qualified: uniqueness is checked on `(snapshot_id, key)`.

## Consequences

- **What we gain.** An identical rerun produces the same `node_id` and `fact_id` in a new
  snapshot, so diffing two snapshots is a join. A new analyzer revision always produces new
  `fact_id`s.
- **The tag string is a contract.** Changing the ID encoding means `lctx-id/v2` and a migration.
- **Oracles:**
  - a proptest that the same inputs give the same IDs and that same-range syntax nodes get
    distinct IDs;
  - a test that changing any single context or producer input changes `run_id`.
