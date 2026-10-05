# A3b — Snapshot as a versioned graph: structural sharing, diffs and comparators

Lane A3b, a follow-up to A3. Researched 2026-10-05. This lane reports capabilities, costs and questions. It does not give an adoption verdict.

## Intent being served

The operator's statement of what a snapshot is:
- a complete, closed, attributable state for pinned inputs and producers;
- never visible partially or before validation;
- immutable once published;
- pinnable by a reader for its whole operation;
- reproducible from pinned inputs;
- with selection separate from publication.

Record identities are content-derived (BLAKE3 typed semantic keys, `semantic-model.md` §15.3). Generation IDs identify *attempts*. The physical `(generation_id, id)` key is a storage choice.

## Evidence labels

- **[doc]** vendor documentation
- **[src]** source inspected
- **[probe]** skill probe from 2026-10-05
- **[A3-run]** a new scratch check. All A3-run checks used the pinned `surrealdb/surrealdb@sha256:681c6c22…` image (3.3.0) on 2026-10-05, either on the memory engine or on `surrealkv:///tmp/kv?versioned=true`, over HTTP `/sql`. The container is removed. These are quick checks, not skill probes.
- **[issue]** GitHub issue
- **[vendor-perf]** vendor performance claim
- **[3rd]** third-party source
- **[repo]** this repository's source, read-only

---

## 0. The store-independent shape

Content-derived identity already gives a Git/Merkle-style structure. Under it, a snapshot is a **root** with three parts:

1. A *manifest*: for each relation, a membership set and a digest. It also carries the input, producer, model and validation-definition identities.
2. *Shared, immutable content records*, keyed by content ID. A record is stored once however many snapshots contain it.
3. *Selection*: a separate pointer to a root, with publication being the act of making the root readable after validation.

Membership can be represented in several ways. Each choice sets the cost of creating, diffing and retiring snapshots:

| Representation | Snapshot creation | Identity diff | Retirement (GC) | Notes |
|---|---|---|---|---|
| **M1 explicit membership rows** `(snapshot, relation, id)` | O(N) rows per snapshot (~11M) | Merge of two sorted ranges, O(N) | Drop that snapshot's rows, then sweep content with refcount 0 | Simplest; supports any snapshot DAG (branches) |
| **M2 validity intervals** `valid_from`/`valid_to` on each shared record (or a membership row per run of snapshots) | O(changed) | Records whose `from` or `to` lies between A and B: O(changed) with an index | Delete rows with `to <= oldest pinned`; middle snapshots cannot be dropped independently | Needs a *linear* snapshot order per lineage; this is how XTDB and Lance work |
| **M3 Merkle/prolly tree over sorted IDs** per relation | O(changed · log N) new nodes | Tree walk skips identical subtrees: O(changed · log N) | Refcount or mark-and-sweep over nodes | Dolt's prolly trees and TerminusDB's layers are instances; gives an O(Δ) root digest as well |
| **M4 base + delta layers** (snapshot = parent + adds + deletes) | O(changed) | Read the layers between A and B | Periodic rollup/compaction | TerminusDB, Lance fragments + deletion vectors, Iceberg manifests |

Two facts in the current model bear on this:

- **The relation content digest is a sequential hash**, so it cannot be updated from a delta. `RelationContent` and `hash_rows` in `crates/lctx-model/src/domain/model.rs` hash `(id, payload digest)` in strictly increasing ID order plus the row count [repo]. Today any change means rehashing the whole relation. M3, or a set/multiset hash (additive or XOR over per-row digests), would make the root digest O(Δ). That is a model-level choice, independent of the store.
- **Sharing is only as good as identity stability.** The same inputs plus the same producers give identical IDs, so sharing is complete. Across library versions, IDs change wherever content changes, and that includes spans and byte offsets. So sharing across versions is an **empirical fraction, not a given**. It needs measuring (probe B1).

---

## 1. Structural sharing in SurrealDB 3.3

### 1a. Shared tables plus per-snapshot membership (M1/M2) — works, application-defined

- **Content records**: one record per content ID, as an array/object or string record id `t:[blake3_bytes]`. A3 trap 5 applies: normalise nested numeric types in ids [doc 3.2→3.3 guide].
- **Membership as array-id ranges (M1).** `member:[snap, 'rel', id]` with range scan `member:[g, NONE]..[g, ..]` returned exactly snapshot g's rows [A3-run v1]. Two snapshots diffed with `array::complement` [A3-run] gave the correct `{added, removed}` [A3-run].
  - Cost: about 11M membership keys per snapshot. Each key carries namespace/database/table/id encoding, roughly 100 bytes or more; the size per key is not measured.
  - The diff then materialises two 11M-element arrays in one query. That is not viable in-database; diff by streaming the two ranges into a client merge.
- **Membership as edges.** A `LIGHTWEIGHT` relation `snapshot->contains->record` gives the same O(N) and reads as a traversal. Its `REMOVE TABLE` is refused while edges exist, and it accepts no fields [doc]. It offers no advantage over M1.
- **Membership as validity intervals (M2).** `valid_from` and `valid_to` fields, indexed, on shared records and edges. Point-in-snapshot reads become `WHERE valid_from <= g AND (valid_to = NONE OR valid_to > g)`; the diff selects rows with `from` or `to` in (A, B]. Plain SurrealQL, nothing special. The same pattern works in PostgreSQL.
- **Immutability.** Content records are write-once by convention. SurrealDB has no frozen table or database (A3 §4), so the only enforcement is "never UPDATE" through credentials and permissions. M2 has a weak point here: retiring or advancing a snapshot *mutates* `valid_to` on shared rows, so "immutable once published" becomes "immutable except for lifecycle columns".
- **Retirement.**
  - M1: delete the snapshot's range, then sweep orphaned content. SurrealDB has no refcount or GC primitive, so the sweep is an anti-join, e.g. `<~` back-references from a `REFERENCE` field, or `count(<-contains)` answered from adjacency in 3.3 [vendor].
  - A3 issue risk applies: RocksDB freezes after a large delete transaction (#7576, open). Deletes must be batched.

### 1b. VERSION time travel (versioned SurrealKV/RocksDB) — a timestamp, not a snapshot

- `SELECT … VERSION d'<datetime>'` reads the MVCC state at that time.
  - Plain reads at time T1 returned the old values and the record deleted later [A3-run v1].
  - The graph traversal `SELECT VALUE ->dep->item FROM ONLY item:a VERSION T1` returned the old edge target [A3-run v1].
- **Trap (new): an indexed predicate under VERSION returns wrong results silently.**
  - Query: `SELECT id FROM item WHERE k = 'c' VERSION T1`, with k indexed and item:c existing at T1. It returned `[]`. `EXPLAIN` showed `VersionScope → IndexScan ik` [A3-run v1].
  - Controls: `WHERE v = 1 VERSION T1` with no index returned a, b, c. `WITH NOINDEX WHERE k = 'c' VERSION T1` returned `[item:c]` [A3-run v1].
  - The engine has a `SURREAL_SURREALKV_VERSIONED_INDEX` knob, "whether to enable versioned index", which applies only to SurrealKV and is off by default [doc env vars l.2077-2080]. It was not tested.
- **Retention is one global duration** (`retention=30d`, `SURREAL_DATASTORE_RETENTION`) [doc start.mdx l.214-249]. There is no per-snapshot pin and no tag. A snapshot older than retention vanishes regardless of readers, and an unwanted middle snapshot cannot be dropped early.
- **Addressing is by datetime only** (`VERSION <datetime>`). A timestamp that never existed returns `[]` [doc select.mdx l.1120].
- **Partial visibility.** A load spanning many transactions has intermediate timestamps that show partial state. The intent can still be met if only a validated commit timestamp is ever published, but that is a convention.
- **Not supported on mem** (3.3) (SB002/SB040).
- **Fit.** VERSION is a crash-recovery and audit facility, not a generation mechanism. It offers no branching, no explicit snapshot identity, no reference-based GC, and indexes are wrong unless versioned indexes work.

### 1c. Changefeeds and versionstamps

- `DEFINE TABLE … CHANGEFEED <dur> [INCLUDE ORIGINAL]` (also definable on a database), read with `SHOW CHANGES FOR TABLE t SINCE <versionstamp | datetime> [LIMIT n]` [doc show.mdx].
  - Entries come per commit with a monotone `versionstamp` and `update`/`delete` payloads.
  - With `INCLUDE ORIGINAL` they are **reverse diffs**.
  - Edge tables carry their own feed [A3-run v1: both `item` and `dep` feeds listed creates as `update` entries].
- **Limits.**
  - SINCE only; there is no UNTIL, so a diff between two snapshot versionstamps is a client-side filter.
  - Retention is bounded by the expiry duration.
  - The feed is per table, and SHOW CHANGES is per table, so ~1,000 relations means ~1,000 feeds.
  - A table without a CHANGEFEED returns `[]` silently (SB043).
  - Not allowed on LIGHTWEIGHT relations [doc].
- **Fit.** It is a log of mutations, which suits M2 maintenance or replication. It is not a content-addressed snapshot diff: two snapshots built independently from scratch share no log.

---

## 2. Diff

### 2a. Identity diff (records/edges added and removed between two snapshots)
- **Store-independent.** With M1 it is a sorted merge of the two membership sets; with M2 an index range on from/to; with M3 a tree walk. Edges are records with content IDs too, so the same applies to them.
- **SurrealDB 3.3** has no snapshot-diff primitive. Available pieces:
  - `array::complement(a, b)` is directional [A3-run].
  - **Trap:** `set::difference` is *symmetric*. `set::difference(<set>$b, <set>$a)` returned both the added and the removed elements [A3-run v1].
  - `array::union`, `array::intersect`.
  - SHOW CHANGES between versionstamps, filtered by the client.
  - VERSION reads of two timestamps, compared by the client.
  - All in-database forms materialise arrays, so a diff at 11M scale belongs in a streaming client merge.
- **Neo4j Community / APOC.**
  - APOC *Core* has only `apoc.diff.nodes(leftNode, rightNode) :: MAP`, a per-node property diff. It is the only `apoc.diff*` entry in the skill's captured 2026.09 function and procedure catalogs [skill `content/index/functions.tsv`; corpus `apoc/core/diff/Diff.java`].
  - APOC *Extended*, a separate unsupported Labs package, adds `apoc.diff.relationships` and the procedure `apoc.diff.graphs(sourceQuery, destQuery, config)`. That procedure compares counts per label and relationship type, then matches nodes by keys or internal id and compares labels and properties [doc neo4j.com/labs/apoc/5/comparing-graphs]. It reports differences but has no snapshot model.
  - Neo4j Community has no time travel. A search of the captured procedures and settings found no `db.cdc` procedure in Community; CDC is an Enterprise or Aura feature (not verified beyond the catalog grep).
  - So there is **no native whole-graph snapshot diff in either store**.

### 2b. Semantic diff ("the same API changed between versions")
- Content IDs change with content, so a semantic diff needs a **declared correspondence key** per entity kind, such as a stable logical identity (qualified name plus kind plus owning module) separate from the content key. Matching then compares payloads by that key: pairs that match are "changed" or "unchanged", the rest are "added" or "removed", and moves or renames need declared heuristics.
- **No candidate store provides this.** All of them diff by primary key:
  - Dolt diffs by primary key;
  - TerminusDB diffs documents by `@id`;
  - Lance diffs by stable row id, which is store-assigned and not semantic;
  - `apoc.diff.graphs` matches by declared keys.
- If the correspondence key is a declared column with an index, every store reduces the semantic diff to a keyed full outer join over two snapshots. DataFusion and PostgreSQL do that natively; in SurrealDB it would be two keyed range reads merged by the client, since no JOIN exists. Whether a correspondence key exists in the model per entity kind is a model question (§15.3 separates entity identity from proposition identity [repo]) and is outside this lane.

---

## 3. Purpose-built comparators

Versions and dates were checked on 2026-10-05 through GitHub releases and the crates.io API.

| | TerminusDB | Dolt / Doltgres | XTDB | Lance | Iceberg |
|---|---|---|---|---|---|
| Version, activity | v12.0.7 (2026-08-10), v12.0.6 (2026-06-24). Now maintained by DFRNT, with an enterprise edition [GitHub, README] | Dolt v2.4.1 (2026-10-02), releases every 1–2 weeks. Doltgres v1.4.0 (2026-10-01) | v2.1.0 stable (2025-12-01). 2.2 has been in beta/rc since 2026-04 (v2.2.0-beta3, 2026-09-28) | crate `lance` 12.0.0 (2026-09-17), new majors roughly monthly; 13.0.0-rc.1 tagged 2026-09-30 | `iceberg` (Rust) 0.10.1 (2026-08-01) |
| Model | Immutable layered RDF/document graph with a Git commit graph. Branches, clone/push/pull, diff and patch between commits, branches or documents [doc] | MySQL-compatible SQL (Doltgres: PostgreSQL wire) on prolly trees. Commits, branches, merges. `dolt_diff()`, `dolt_commit_diff_<t>`, `dolt_history_<t>` [doc] | Bitemporal SQL:2011 (system and valid time) over PG wire. Every table is bitemporal; columnar on Arrow [doc] | Columnar dataset format. Every write makes a version. Tags exempt their version from `cleanup_old_versions`. Branches with their own linear histories. Shallow clone references the source's fragments [doc Context7] | Table format with snapshots, manifests, branches and tags |
| Rust / Python access | Rust client listed; core is Prolog plus Rust `terminusdb-store`. Python client v12 [README] | Rust and Python through MySQL (Doltgres: PG) wire. sqlx works [inference, not probed] | Rust/Python through PG wire (sqlx, psycopg). Engine is JVM/Clojure | Native Rust crate plus Python. **Requires Arrow ^58 / DataFusion ^54**, while the workspace pins Arrow =59.3.0 / DataFusion =55.1.0 [crates.io API; `Cargo.toml` l.30-34]. Type-sharing conflict until a matching major ships | Rust 0.10.1 needs `arrow-array ^58`: same conflict. pyiceberg for Python |
| Scale vs ~11M records / ~1,000 relations | Each branch must fit in memory (no paging). Vendor figure ~13 B/triple on billion-triple sets [vendor-perf]. 11M records × fields ≈ 10⁸–10⁹ triples, so roughly 1–13 GB resident (estimate) | Prolly trees share structure across commits. Known: `dolt_diff` with ORDER BY is slow for big diffs (#11959) [issue]; query diff is O(n²) [doc] | Built for large append-heavy data. 1,000 tables plausible; untested here | Per-dataset scale is a strength. **~1,000 relations means ~1,000 datasets with no cross-dataset atomic commit**, so a root manifest (pointer flip) is still application-owned | Per-table snapshots. Multi-table atomic commit only through a catalog supporting a commit-transaction call (REST catalog); application-owned otherwise |
| Schema / integrity | Schema documents enforced, "advanced typing" [README] | Full MySQL DDL: PKs, **enforced FKs**, CHECK, UNIQUE [doc]. Merges can leave FK-inconsistent states that need checking [doc] | **No foreign keys**; referential integrity must be implemented by hand [doc/3rd] | Arrow schema only; no FK, UNIQUE or CHECK | Schema evolution only; no constraints |
| Diff semantics | Structural patch (triple- or field-level insert, delete, modify) between commits | Row-level diff by primary key between any two commits: added, modified or removed, with both before and after values | As-of queries at a system time; a diff is two as-of queries | `dataset.delta()` / `DatasetDelta`: `get_inserted_rows`, `get_updated_rows` via `_row_created_at_version` / `_row_last_updated_at_version` (stable row ids) [doc Context7]. A deleted-rows API was not found | Incremental append scans between snapshots; changelog views mostly engine-side (Spark) |
| Graph query | WOQL (Datalog with path queries), GraphQL | Recursive CTEs only | SQL and XTQL; no graph traversal operators | None (scan, filter, vector/FTS index); graph work stays in DataFusion and petgraph | None |
| Maturity risks | Maintainer change, small community; whole-branch-in-memory model | Mature, with high release cadence; MySQL dialect (Doltgres younger) | 2.2 has been pre-release for months; JVM service | Format and API churn: monthly majors, API removals (e.g. `diff_meta` removed in 0.39 [doc]); Arrow major lag | iceberg-rust is pre-1.0 |

Fit against the intent:
- **Dolt and TerminusDB match the Git semantics natively.** They provide commits as immutable roots, structural sharing, diff, branch and tag. Dolt alone also gives relational integrity (FKs, CHECK) and SQL, but no graph query beyond recursive CTEs. TerminusDB gives graph queries and diff, but holds the whole branch in memory.
- **Lance gives versioned, structurally shared columnar storage with tags (pins) and row lineage**, and it sits closest to the existing Arrow/DataFusion stack. It enforces no integrity, has no multi-dataset atomic root, and currently mismatches the pinned Arrow major.
- **XTDB is time-travel (M2-like), not Git.** It has no FKs.
- **None provides semantic (correspondence-key) diff or graph analytics.**

---

## 4. What the intent requires whatever the store

The model owns validation as pure state machines.

- **The shape.** `Invariant { inputs: Vec<ValidationInput{relation, order, prefix}>, create }` produces an `InvariantCheck` that `visit`s ordered Arrow batches of entire declared input relations, then `finish`es [repo `model.rs` l.764-880].
- **The scale.** `validation.rs::definitions()` aggregates the per-domain invariant families: about 290 `extend` registrations and about 160 distinct invariant names [repo grep].
- **The memory store.** `MemoryGeneration` validates unique keys, complete and subtype-correct references, the ordered content digest, and every invariant fed in declared input order [repo `memory.rs` header].
- **Already partly prefix-based.** Vocabulary inputs can be pinned `at_epoch(PublicationBoundary)`.

Whether validation over a delta is compositional depends on the class of check:

| Class | Example in the model | Delta-compositional? |
|---|---|---|
| Row-local (field validity, ranges, codebook membership) | per-record checks | **Yes**: validate only new records; shared content was validated once, *provided the validation-definition digest is unchanged* (definitions are digested: `Invariant::digest`) |
| Key uniqueness | unique IDs per relation | Yes with an index over the snapshot membership |
| Referential completeness (FK, subtype) | complete references | **Partly**: new references check that their target exists in the new snapshot. *Removed* records need a reverse index of referrers so no surviving record dangles |
| Content digest | `RelationContent` | **No** under today's sequential hash. Yes under M3 or a set hash |
| Closed-world membership, ownership and coverage (exactly-one, set equality against a manifest) | `input_manifest_membership`, `artifact_ownership_input`, `coverage_scope_ownership`, `provider_invocation_membership` [repo names] | **Not in general.** They quantify over the whole relation. They become incremental only when partitioned by a key (e.g., per input manifest or per invocation) and the state per partition is kept, i.e. incremental view maintenance. Otherwise they need full revalidation |
| Order-dependent streaming folds (inputs in declared sort order) | most `InvariantCheck`s | **No** as written: the check API consumes complete ordered inputs. Reusing a prior result needs memoising per-partition summaries keyed by (definition digest, partition content digest) |

So structural sharing removes most *storage and copy* cost directly. *Validation* cost falls only to the extent that invariants are partitioned, and their partition summaries plus a delta-capable digest are memoised. That is model and validator work in any store; no candidate store supplies it. The current model's §6.2 checkpoint frame reuse ("reuse requires matching … content, physical layout…") is already a coarse form of this memoisation (`storage-and-publication.md` §6.2) [repo].

---

## Traps found by this lane (additions to A3)

1. **VERSION plus an index returns wrong results silently** (SurrealKV, 3.3, versioned-index knob at its default). Use `WITH NOINDEX` or test `SURREAL_SURREALKV_VERSIONED_INDEX` [A3-run].
2. **Retention is global and time-based.** There is no per-snapshot pin; old snapshots expire whether or not readers hold them [doc].
3. **SHOW CHANGES has SINCE but no UNTIL**, works per table, and is limited by expiry [doc].
4. **`set::difference` is a symmetric difference**; use `array::complement` for directional diffs [A3-run].
5. **APOC Core diffs one node pair only.** Whole-graph comparison (`apoc.diff.graphs`) is APOC Extended (Labs) and is count- and key-based [doc, skill catalog].
6. **Lance 12 and iceberg-rust 0.10 need Arrow 58**; the workspace pins Arrow 59.3 / DataFusion 55.1 [crates.io].

## Unresolved

- The actual sharing fraction between successive generations: same inputs with changed producers, and adjacent library versions.
- Whether `SURREAL_SURREALKV_VERSIONED_INDEX=true` makes VERSION and index reads correct, and at what cost.
- Lance: whether a deleted-rows delta exists; whether a multi-dataset commit exists (LanceDB namespaces); when an Arrow-59 major ships.
- TerminusDB resident memory at this record count. The vendor figure is per triple; the triple count here is an estimate.
- Dolt `dolt_diff` cost at about 11M rows; whether the O(n²) applies to table diffs or only to "query diff".
- Whether every entity kind in the model has a declared cross-version correspondence key.

## Candidate Phase B probes

| ID | Question | Minimal setup | Control | Outcome that would change the judgment |
|---|---|---|---|---|
| B1 | How much do successive generations share? | Two existing generations: the same library recompiled with a changed producer, and two adjacent library versions. Intersect the content-ID sets per relation (DataFusion over PG) | The same intersection on (generation_id, id) rows | Under about 30% sharing makes structural sharing a storage nicety; over 70% makes it a major cost lever |
| B2 | Is a set or Merkle relation digest equivalent and O(Δ)? | Implement an additive/XOR multiset digest and a sorted-key Merkle digest in scratch over one relation | The current sequential `RelationContent` digest on the same rows | Equal-content detection plus O(Δ) update would let the root digest compose |
| B3 | How many invariants are partition-compositional? | Classify the ~160 invariants by quantifier scope (row, key, reference, partitioned closed-world, global) | Manual review of 10 | The global share sets the floor of full revalidation per snapshot |
| B4 | Do VERSION reads with indexes give correct results? | Versioned SurrealKV with `SURREAL_SURREALKV_VERSIONED_INDEX=true`; also RocksDB `versioned=true` | `WITH NOINDEX` results | Correct results with acceptable write overhead would make VERSION usable for audit reads; otherwise rule it out |
| B5 | M1 compared with M2 in SurrealDB and PG | 11M content rows, then a second snapshot with 10% change. M1 membership ranges against M2 intervals; measure creation, diff (streamed client merge) and retirement plus orphan sweep (batched deletes, RocksDB) | The same two schemes in PostgreSQL 18 | If PG matches or beats SurrealDB on both, sharing does not argue for a store change |
| B6 | Dolt as the snapshot store | Load one generation (about 1,000 tables with FKs) into Dolt via the MySQL wire, commit, load the next and commit; run `dolt_diff` on large tables; check FK enforcement | PG load time; manual diff | Acceptable load and diff time makes Dolt the closest native match for relational integrity plus Git semantics |
| B7 | Lance as a versioned columnar snapshot | Write relations as Lance datasets (Arrow 58 behind an IPC boundary); tag a generation; read row-lineage deltas; time `cleanup_old_versions` with tags | Parquet full copies | Proves or refutes a lightweight Arrow-native snapshot layer, and the Arrow-major friction |
| B8 | TerminusDB memory at this scale | Import one generation as documents; measure resident memory, diff time and schema enforcement | Vendor 13 B/triple figure | Resident memory well over the dataset size rules it out |
| B9 | Semantic diff prerequisites | For three entity kinds (e.g., callable, class, module), test whether a declared correspondence key gives a 1:1 match across two library versions | Hand-checked API change list | Missing or ambiguous keys make semantic diff a model task whatever the store |

## Files written

- `/tmp/claude-1000/-home-paul-library-context/3307aa74-dd84-437f-93be-1d92a29d0cce/scratchpad/a3b-snapshot-representations.md` (this report).
- Scratch checks reused `.../scratchpad/q.sh`.

No repository, skill or evidence files were modified.
