# B-W2 snapshot-diff: narrative (2026-10-05, stopped at operator direction)

Evidence folder: `docs/design_review/evidence/2026-10-05_snapshot-diff/`. Its README is the
authoritative record with commands and raw outputs. Status: the static probes (2, 3) and the
synthetic diff harnesses in four stores are done. The real runs used the facts layer of the failed,
unpublished generation `786cd6d5…` (staging, unvalidated, L0). PG and Dolt completed and passed;
SurrealDB passed the identity diff only. The remainder is `not_run (stopped at operator direction,
2026-10-05)`. No timing was recorded, by operator direction. All probe containers, volumes and the
probe database have been removed.

## 1. Identity diff (probe 1, synthetic)

**Corpus.** Ten relations shaped like the model's keys:
- source artifacts keyed by content hash;
- occurrences keyed by source and span;
- `Source` entities keyed by their declaration occurrence;
- provider symbols keyed by native key;
- signatures keyed by (symbol, parameters digest), as the real `Signature` is (`calls.rs` l.296-312).

**States.** A has 612k rows. B applies three file edits that cascade, plus about 1% adds, removes
and same-id payload changes, plus about 1% signature parameter changes. The reference diff is
computed in Python and cross-checked by DuckDB.

**Result: exact added/removed/changed sets in every store and form tried** (v1 and v2):
- PG 18.6:
  - M0 full copies, both `EXCEPT` and a full outer join;
  - M1 shared content (id, pdig) plus membership (snap, rel, id, pdig);
  - M2 `int4range` validity intervals.
- SurrealDB 3.3.0 (SurrealKV):
  - M1 content `<rel>:[id, pdig]` plus `member:[snap, rel, id]` with a link to the version.
    The diff is keyset-paged range scans of both snapshots merged in the client.
  - The same diff in-database with `array::complement`, which completed for 300k-element arrays.
  - M2 as indexed `lo`/`hi` fields.
- Dolt 2.4.1: each state is a full rebuild (delete everything, reinsert) committed and tagged;
  `dolt_diff(tagA, tagB, t)` and `dolt_diff_stat` agree with the reference.
- TerminusDB 12.0.7: each state is one `full_replace` commit; `/api/diff` with
  before/after data versions gives Insert/Delete/field patches that map exactly.

**Structural sharing.**
- Logically, 606,691 of 612,137 B row versions are shared (99.1%).
- PG M1 needs 617k content rows for 1.22M membership rows. Physically, though, the membership
  rows are as wide as the narrow, id-dominated content rows. After `VACUUM FULL`, M1 is 92 MB of
  content plus 151 MB of membership, against **168 MB for two full copies**; M2 is 110 MB.
- **So explicit membership does not save storage for this model's row shapes.** Intervals (M2)
  do, but they need a linear history and mutate lifecycle columns on shared rows.
- A compact membership (a dense surrogate plus a per-snapshot bitmap) is the Proposed alternative,
  untested.
- Dolt grew 40.7% of A for B after `dolt_gc`: about 1% scattered churn rewrote about 40% of the
  chunks.
- TerminusDB's storage was 85 MB for both states.

**Correctness and integration traps.**
1. Shared content must be keyed by (id, payload digest), because the model allows the same id with
   a different non-key payload. SurrealDB `INSERT IGNORE` on id alone silently keeps a stale
   payload, and plain `INSERT` fails the whole batch on one existing id.
2. Under sharing, referential integrity is per snapshot and is not a store FK. A FK over shared
   content proves "exists in some snapshot" only. Membership anti-joins (PG) and
   `record::exists(type::record('member', [s, rel, ref]))` (SurrealDB) caught the dangling
   referrers a FK would miss. A SurrealDB record link to a content version is ambiguous across
   snapshots, so links should target the identity and resolve via membership.
3. Dolt enforces FK and CHECK on statements. Under the documented bulk mode
   `FOREIGN_KEY_CHECKS=0`, a dangling row is accepted **and `DOLT_COMMIT` commits it**.
   `DOLT_VERIFY_CONSTRAINTS('--all')` then fails with 1105 unless
   `@@dolt_force_transaction_commit=1`, after which it reports the violation. A parents-first load
   with FK checks on worked for 612k rows.
4. TerminusDB enforces typed links (`SchemaCheckFailure`). A `PUT … full_replace` keeps old
   documents, so a state must be a `POST … full_replace=true`.
5. SurrealDB: SCHEMAFULL types and `ASSERT` are enforced; plain string references are not. With
   an undefined namespace, the first statement failed while later statements in the same request
   ran.

**Feasibility (qualitative).** All completed. Server memory after the runs:
- SurrealDB: 3.4 GiB (v1) and 6.6 GiB of an 8 GiB cap (v2). A real-scale run needs a deliberate
  larger cap.
- Dolt: up to 1.2 GiB.
- TerminusDB: peaked at 1.7 GiB.

## 2. Semantic diff (probe 2)

**Static review.**
- Callable, class and parameter entities (`Source` variants) are keyed by their declaration
  `Occurrence`, which is keyed by (source content hash, byte span, kind, role, structural path).
  Any edit to a file re-identifies every entity in it, and a new input revision re-identifies all
  of them. **No declared cross-version correspondence key exists for them.**
- Modules have `qualified_name`, which collides for .py/.pyi pairs, so the key needs a stub flag.
- `ProviderSymbol.native_key` (Pysa-style qualified key, per provider and context) and
  `PublicNameObservation.name` under the module's `qualified_name` are the best derived keys.
- The model forbids cross-provider spelling joins.
- Signatures key on their parameters digest, so a parameter change is a remove plus an add at the
  identity level, and only a correspondence key reports "changed".

**Demonstration.** A keyed diff on (`native_key`) gave 1 added, 2 removed and 199 changed in
three stores, all `passed`:
- PG: SQL full outer join.
- SurrealDB: no JOIN, so a `type::record` lookup per row with a client merge.
- Dolt: `AS OF` per tag with a client merge.

**Pending (real generation).** Uniqueness of (provider, `native_key`, role, variant, form) and of
`qualified_name`.

## 3. Validation incrementality (probe 3, static)

All 119 checks were classified by reading `visit`/`finish`:

| Class | Count | Delta-checkable? |
|---|---|---|
| R streaming forward reference or row predicate | 21 | Yes forward; deletions need reverse indexes |
| K key-local parent-digest groups | 16 | Yes per parent key (`input_manifest_membership` is K, not global, correcting A3b) |
| F multi-hop forward closure | 20 | Mostly yes, with reverse indexes and ancestor expansion |
| G global/closed-world | 11 | No: acyclicity, no-unreferenced-node, profile/count checks |
| D derivation replay (`output == f(inputs)`) | 51 | No: cost equals rerunning the stage |

- 57 of 119 are delta-checkable once reverse indexes exist; 62 of 119 are closed-world.
- The 51 replays dominate. Several already iterate per invocation, so a partition exists.
- The relation content digest is sequential, so it cannot be updated from a delta.
- **Conclusion.** Structural sharing in any store removes copy and storage cost, but not most
  validation cost. That needs partitioned, memoised derivation plus a set or Merkle digest: model
  work that no candidate store supplies.

## Real-generation facts seen read-only (staging schema)

- One schema per generation, about 1,390 tables including `__vN_*` epoch tables.
- Rows carry `generation_id` and `introduced_epoch`.
- Constraints: 7,272 CHECK, 138 UNIQUE and 878 PK, but only 16 FKs, all to `publication_groups`.
  Cross-relation references are model-validated, not store-enforced.
- Staging tables are lock-held by the compile, and a read-only `count(*)` waited (cancelled).

## Uncertainties

- The synthetic row shapes are narrow like the real ones, but the real relations' widths and
  counts are unmeasured.
- The real perturbation simulates the cascade structurally (fresh ids, references inferred); it
  does not re-encode the model's keys.
- TerminusDB and Dolt were run only at 612k rows; ~11M rows is untested.
- The D/G classification is a manual reading; per-invocation partitionability of each replay was
  not verified individually.

## 4. Real facts-layer runs (generation 786cd6d5…, staging, facts-only, unvalidated)

**Export.** 15 facts relations, 3.53M rows. The perturbation **simulates** the model's keys and
does not re-encode the typed BLAKE3 keys:
- real ids and payloads are used, with key/payload partitions parsed from `#[model(key)]`;
- changed rows get fresh ids, and the cascade follows key-field references;
- B combines an edit to one source file (86 occurrences), 327 signature parameter changes, about
  1% leaf adds and removes, and about 1% payload changes;
- the reference diff (36,369 added, 26,776 removed, 1,813 changed; 98.9% shared) was cross-checked
  by DuckDB.

**Results.**
- **PG18: `passed`** (M0, M1, M2 and semantic).
  - Two full copies are 1,135 MB, M1 is 1,597 MB (967 MB of it membership) and M2 is 723 MB.
    Membership again costs more than the sharing saves.
  - The integrity probe found 1,592 dangling occurrences that a FK over shared content would miss.
- **Dolt: `passed`.** B grew the store by **49.4% of A** for about 1% churn. Peak memory 1.9 GiB,
  under a 32 GiB cap.
- **SurrealDB: identity diff `passed`** after fixes, under a 48 GiB cap with a peak of **26.7 GiB**.
  The integrity probe, enforcement, M2, semantic diff and `array::complement` are `not_run
  (stopped)`.
- **TerminusDB: `not_run (stopped)`.** Peak 7.1 GiB while loading, under a 32 GiB cap, and no
  completed state or diff before the stop.

**SurrealDB traps found only on real data.**
1. `INSERT IGNORE` silently drops rows with field-type violations (NONE in a non-option field)
   and reports nothing. The first attempt lost two whole relations and parts of three more.
   Because membership loaded, the membership-only diff "passed" over missing content, so a
   content-completeness check is required.
2. A 3.5M-row single `INSERT … SELECT` was refused: "Memtable arena is full" (SurrealKV).
3. `/sql` has a 1 MiB default body limit (HTTP 413; `SURREAL_HTTP_MAX_SQL_BODY_SIZE`).

**Probe 2 on real data (`real_keys.sql`).**
- The normalized entity kinds (callable, class, parameter and field entities) are `blocked`:
  0 rows.
- **`native_key` is a positional analyzer index** (`F:n`, `CF:class:n`, class ordinals).
  (provider, `native_key`, kind) is massively non-unique (948 distinct of 19,412 callables) and
  is unstable across versions by construction. This refutes the static assumption.
- Module `qualified_name` has 4 duplicates, all `__unknown__` example files.
- The **derived qualified path** (module plus enclosing declared definitions via
  `syntax_placements`, plus name) is unique for classes (1,995/1,995) and nearly unique for
  callables (13,862/13,950). The collisions are deliberate redefinitions.
- The public path (module, name) is unique (4,720).
- The within-version signature key (symbol plus Signature key minus parameters) is unique, but its
  symbol ids are not cross-version.
- Cross-version stability: `not_run`, because a second library version would be needed.
