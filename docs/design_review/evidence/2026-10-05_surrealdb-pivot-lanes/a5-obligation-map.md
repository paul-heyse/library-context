# A5: obligations, mechanisms and what exists only because of them

Lane A5 of the SurrealDB adoption design review (design tier, target purpose). This lane reports repository facts and does not render a verdict. Investigated 2026-10-05.

**Baseline.** HEAD is `f66eb15a`. Its diff from the briefed `1158ebe2` touches only `AGENTS.md` and `.config/library-skills.toml`, so all code and design citations hold at 1158ebe2.

**What was read.**
- Source: `crates/*`, `python/lctx_storage`, `scripts/*` and `justfile`.
- Owners: DESIGN §1.1/§B2–§B14, `sections/storage-and-publication.md`, `sections/semantic-model.md` §15.3/§15.10–§15.13, the CI profile principles, `docs/postgresql.md`, `docs/pins.md`, and ADR-0086/0094/0103/0105/0108/0114/0126.
- Read-only `cargo tree --offline --locked`.
- One read-only `psql` query against the control schema (`PGOPTIONS=-c default_transaction_read_only=on`).

**What was not run.** No compiles, tests or mutating recipes.

**Labels.** **[obs]** means observed in source, docs or the DB. **[int]** means my interpretation.

**Builds on the Phase A reports.**
- A1: store surface, line counts, live catalog.
- A2: graph-kernel classes and timing.
- A3: SurrealDB 3.3 capabilities and traps.
- A3b: snapshot representations M1–M4 and delta-compositional validation.

Their inventories are not repeated; this report adds the obligation-to-dependent map and the second-order deltas.

---

## 0. Headline facts this lane adds

1. **Nominal-reference closure is enforced four times. One invariant ("every Id reference resolves to an existing, subtype-correct target") has four realizations [obs]:**

   | # | Realization | Location |
   |---|---|---|
   | 1 | Generated PG FKs, trial-installed in a SAVEPOINT at each checkpoint and vocabulary close | trials: `receipts.rs:374-381`, `vocabulary.rs:292-324` |
   | 1 | The same FKs, installed permanently at `validated` | `ddl.rs:119` |
   | 2 | A per-field anti-join SQL check before a stage may read its inputs, acknowledged as the `nominal_references` proof | `stage_validation.rs:179-260` |
   | 3 | The same anti-join in explicit `lctx generation audit` | `audit.rs:109-137` |
   | 4 | A pure-Rust check in `MemoryGeneration` | `memory.rs:637-660` |

   The model's validators do not include reference closure; the store supplies it. This is the clearest category-(c) duplication.
2. **Record IDs are semantic keys, not content addresses** [obs].
   - An `Id` is BLAKE3 over the `#[model(key)]` fields under the namespace `lctx-semantic/v3` plus the type (`identity.rs:167-174`; macros `lib.rs:296-325`).
   - Relation content is the sequential hash of `(id, payload digest)` (`model.rs:740-756`).
   - Two generations can therefore hold the same `Id` with different payloads. Under (iii) a structurally shared record must be addressed by `(Id, payload digest)`, not by `Id` alone [int].
3. **Provider identity covers the whole workspace source, so sharing across binaries is close to zero for provider-attributed facts** [obs]. The chain:
   - `Provider` is keyed by `tool`, `revision` and `build_digest` (`attribution.rs:79-86`).
   - `build_digest` hashes `LCTX_PRODUCER_SOURCE_DIGEST` + `Cargo.lock` + both analyzer patches + provider sources (`cpg-extract/src/bundle.rs:30-45`).
   - `LCTX_PRODUCER_SOURCE_DIGEST` walks every crate's `Cargo.toml`, `src`, `sql`, `migrations`, `models` and `build.rs`, plus `Cargo.lock`, `rust-toolchain.toml`, `third_party` and `specs` (`scripts/producer_fingerprint.rs:37-60`). That includes `crates/lctx-postgres/src` and the PostgreSQL image pins in `specs/`.
   - `ProviderRun` keys include `provider` (`attribution.rs:107-118`), and every support is keyed by `run` (macros `lib.rs:153`).

   Consequences:
   - Any edit anywhere in `crates/`, or a lockfile move, gives every support, observation and assertion a new identity [int, from the key chain].
   - Store-code edits perturb fact identity: a change to `lctx-postgres` changes the IDs of facts it merely stores [obs].
   - Structural sharing (iii) only pays where the binary is unchanged, or after the identity scope is narrowed. Narrowing is a model decision, independent of the store [int].
4. **Every `lctx-model` edit, and every `Cargo.lock` move, forces an operator store reset** [obs].
   - The model digest includes `owned-semantics`, a hash of every `.rs` byte in `lctx-model/src` and `lctx-model-macros/src` plus `Cargo.toml` and `Cargo.lock` (`build.rs:1-45`; `model.rs:332-340`).
   - `store install` refuses with `Contract` on a model or physical digest mismatch (`install.rs:33-38`).
   - Readers check model and physical digests under the lease (`lease.rs:1-6`).
   - The runbook says "A model change is a `store reset` followed by a rebuild" (`docs/postgresql.md:144`).
   - `just upgrade` therefore also invalidates the store [int]. This follows from the model-identity choice, not the substrate. Under any store, digest-bound installations reset the same way, unless identity is narrowed to declarations (§15.3 already calls the fingerprint "conservative").
5. **`lctx_ops` (attempt history) is historical** [obs].
   - `OperationsDb::start_attempt` has no production caller; the repo-wide grep shows only its definition. The one other writer is `lctx runs mark-interrupted` (`crates/lctx/src/runs.rs:57`).
   - ADR-0094 says new compiles never append.
   - Live `lctx_ops.attempts` and `events` hold 0 rows (A1).
   - Still kept alive for it: `operations.rs` (115), `runs.rs` (69), part of the service-baseline migration, two of the five tables fingerprinted by `postgres_backup.py` (`:13-19`), the test `services::attempts_events_runs_mark_interrupted`, `verify.rs`'s `SERVICE_SCHEMAS` (`:23`), and the runbook's "Retained services" section. Category (d).
6. **Two PostgreSQL driver stacks and two TLS stacks** [obs].
   - SQLx 0.9 with rustls handles lifecycle, COPY and serving.
   - The owned DataFusion provider fork handles compile-time stage reads and `lctx query` (`cpg-core/src/generation_read.rs`). It brings tokio-postgres 0.7.18, bb8-postgres, and postgres-native-tls → native-tls → **openssl-sys**. `cargo tree -i openssl-sys` reaches cpg-core only through the fork.
   - The lease protocol is driver-neutral (`LeaseDriver`, `lease.rs:1-6,24-30`) only because both drivers must run it.
   - `lctx-postgres`'s normal closure is 237 packages, against 116 for `lctx-model` (`cargo tree -e normal`); the lockfile has 857 packages.
7. **Most PG-dependent tests are not in any `verify-*` family** [obs].
   - 116 `DisposableDatabase::start()` calls in 63 files, about 27.3k lines of test files.
   - These `lctx-postgres` targets are not named in `scripts/verify.py`, so `just qualify` and `verify-store` never select them: 15 `domain_*` round-trip files, `services`, `unicode_values`, `finite_metrics` and `native_inventory` (19 targets, 4,195 lines).
   - Also unselected: 24 `cpg-core` targets (e.g. `summary_publication`, `analytic`, `generation_read`, `projection_hydration`) and 6 `lctx` targets (`compile_facts`, `store_cli`, `serving_vectors`, …).
   - Search scope: target names against `scripts/verify.py`. Family filters cannot add targets, because they are appended to fixed `--test` lists (`verify.py:255-262`).
8. **The model already has a store-free write and validate path, but reads are PG-only** [obs].
   - `StageSink` (`stages.rs:1819`) is implemented by `GenerationAttempt` (`lifecycle.rs:498`) and `MemoryGeneration` (`memory.rs:253`).
   - `MemoryGeneration` keeps private deltas, sealed sets and prefix snapshots in memory (`memory.rs:17-24`), so a vocabulary epoch already has a second, store-free realization.
   - Stage reads (`AttemptSession`, `consumed_rows.rs`, `generation_read.rs`) exist only over the PG provider fork. Normalized and higher frontiers therefore need a container in every test [int].

---

## 1. Obligations (the "why" column)

| ID | Obligation | Authority | Store-independent? |
|---|---|---|---|
| O1 | **Snapshot intent**: complete, closed, attributable state for pinned inputs and producers. Never visible partially or before validation; immutable once published; pinnable for a reader's whole operation; reproducible; selection separate from publication | §B7, §15.11 Lifecycle, ADR-0086, CI-13, A3b §intent | Yes; the realization is the store's |
| O2 | **Evidence closure (CI-11)**: every served claim cites rows that exist in the same snapshot. Needs reference closure plus single-generation pinning | CI-11/CI-G2, §15.12 Integrity rules, §B13 | Yes |
| O3 | **Provenance (CI-01)**: provider, revision, configuration, run, span on every fact | §B6, CI-01, `attribution.rs` keys | Yes; model-owned, and the store only persists it |
| O4 | **Unknown ≠ absent (CI-04)**: a missing or corrupt required relation refuses; coverage per family and scope; explicitly frozen empty relations; required-null refusal | CI-04, §6.2, §15.11 facts frontier, §15.12 | Yes; parts are realized by store refusal paths |
| O5 | **Determinism and reproducibility**: deterministic IDs and content digests, canonical orderings, CI-10 hermetic inputs. CI-09 (heuristic governance: seeds, parameters, run-local labels) covers analytics determinism; the brief's "determinism CI-09" maps to it only partly | §15.3, CI-09, CI-10 | Yes |
| O6 | **Bounded serving (§B13, CI-08)**: row, node, depth, response budgets; one pinned generation per process; guard loss is terminal; bounded connections and CPU | §B13, ADR-0114, §15.12 Startup | Yes |
| O7 | **Embedding spec (§B14, CI-13)**: one hashed spec; exact consumed vectors in the generation; one immutable cache winner per spec and input | §B14, §6.5 Cache and replay | Yes; the cache is a cost obligation |
| O8 | **Stage authority**: one writer per output; no read before the writer completes; reads only of acknowledged stored inputs; immutable vocabulary prefixes; old receipts verify their original prefix | §15.11 stage table, ADR-0105/0108/0116 | The rules are domain; the store only realizes them |
| O9 | **Attempt resource budget**: charged state, typed refusals instead of truncation (CI-08) | §15.11, `domain::resources` | Yes |
| O10 | **Contract evolution, current-only**: a schema snapshot change is a migration; append-only codebooks; rebuild from pinned inputs, no old-format readers | §6.3, ADR-0078/0087 | Yes |
| O11 | **Typed failure, no partial publication**: failed/interrupted attempts are invisible; typed failure classes; retry creates a new generation | ADR-0094, §15.11 | Yes |
| O12 | **Operator inspection** (`lctx query`, `generation list/show`, audit) | AGENTS.md Commands, runbook | Convenience; `audit` is ADR-0126 |
| O13 | **Retained operational history** (`lctx_ops`) | ADR-0094 ("historical") | None current (d) |

---

## 2. Mechanism map: obligation → mechanism → dependents

Line counts are file lines unless marked src (A1 src-only counts in brackets).

Categories:
- **(a)** compensates for the substrate;
- **(b)** a genuine domain guarantee that any design must keep;
- **(c)** duplicated enforcement of one invariant;
- **(d)** historical or transitional.

### M1 One PG schema per generation, privilege-enforced immutability, phased grants — serves O1, O11

Realization: `ddl.rs` phases `staging → sealed (revoke writer) → validated (FKs) → published (grant reader)` (`ddl.rs:18,107-127,412-431`) and `lifecycle.rs` (907).

| Dependent | Size / place | Cat. |
|---|---|---|
| Four service roles plus bootstrap. The roles are `lctx_migrator`, `lctx_app`, `lctx_importer` and `lctx_serving`. Bootstrap needs sudo; the protected configs are 4 JSON files at mode 0600. | `roles.rs` 166, `bootstrap.rs` 31, `scripts/postgres_bootstrap.py` 190, runbook §Local deployment | (a); grant-based immutability. A credential split is generic hygiene. |
| Grant/revoke choreography: per-table `GRANT INSERT` while staging, revoke at seal, reader grant at publish | `ddl.rs:28-34,412-431,585` | (a) |
| Write draining before transitions: `LOCK TABLE … ACCESS EXCLUSIVE` per output | `receipts.rs:64-69,357-362` | (a), needed because writers are concurrent server sessions |
| `store check`: compares the live catalog with a rolled-back shadow lowering; verifies the role and ACL matrix | `verify.rs` 650; `just store-check` | (a). It exists because the schema is generated DDL living in an external mutable server. |
| `store install`/`reset`: phased, resumable, one generation per transaction | `install.rs` 177; runbook §Reset | (a); reset exists because each generation is many catalog objects |
| `max_locks_per_transaction=512` operator setting. Reset hit the default of 64 at 1,086 tables and 11,914 constraints. | `docs/postgresql.md` §Catalog lock capacity | (a) |
| Per-generation physical DDL, regenerated per attempt: 784 canonical tables, 302 delta tables, 1,061 indexes, 126 views (A1 §2) | — | (a) for the per-generation copy; the declarations themselves are (b) |
| Tests asserting privileges and ACLs | `services::service_grant_matrix`, `superuser_and_elevated_owners_refused`, `generation_catalog::leases_are_counted_and_the_reader_cannot_mutate_control`; grep: 3 files with 42501 / permission denied, 6 with GRANT/REVOKE | (a) |
| Lifecycle tests | `lifecycle.rs` 828: `late_write_vs_seal_race`, `lease_vs_retire_race`, `select_vs_retire_race`, `retire_is_atomic_under_fault`, `mixed_lifecycle_does_not_deadlock`, … | Mixed: the race and deadlock tests are (a); failure typing and atomic publication are (b) |

**Fate under the candidates.**
- **(i) SurrealDB primary.**
  - Falls away: roles, grants, ACL verification, lock-capacity tuning.
  - New: SurrealDB has no frozen-database or revocation equivalent (A3 §4), so immutability becomes "credentials plus convention". That is weaker than O1's "immutable by privilege", unless O1 is redefined to trust-the-writer, as ADR-0126 already partly does for superuser tampering.
  - Still needed: a database per generation, `REMOVE DATABASE` (#7576 risk), and a DDL-drift check (`INFO FOR DB` comparison), with the same reason as `verify.rs`.
- **(ii) Projection alongside PG.** Nothing falls away. A second set is added: projection database per generation, pointer, purge with generation cleanup, its own credentials.
- **(iii) Content-addressed, write-once segments.**
  - Immutability holds by construction: segments are never rewritten, and readers verify digests. Grant phases, revoke-at-seal, draining locks, `store check`'s ACL half, reset-by-schema-drop and lock tuning fall away.
  - Kept: publication as a manifest/root write plus selection pointer (b).
  - New: GC of unreferenced content and a pin registry to protect leased roots.

### M2 Leases, locks and the serving guard — serves O1 (pinnable), O6

Realization: `lease.rs` 327 (session-level shared advisory lock plus digest checks), `locks.rs` 100 (lock order installation → selection → generation → attempt → relations), `guard.rs` 177, `runtime.rs` 304 (CPU admission).

| Dependent | Cat. |
|---|---|
| `LeaseDriver` driver-neutral protocol (sqlx and tokio-postgres) | (a), because there are two drivers (fact 6) |
| Owned DataFusion provider fork patch: Closing/Closed state, drain/release, cancellation guard, no reconnect. 1,398 lines, pins row `docs/pins.md:102`. | (a): the fork's lifecycle additions exist so provider sessions hold the lease through drain |
| Retirement refuses while leased; `lctx generation retire` | (b) for the rule; (a) for the advisory-lock realization |
| Guard: process-lifetime lease, terminal loss, CPU slots, startup drain (ADR-0114) | (b) bounded serving; the lease underneath is (a) |

**Fate under the candidates.**
- **(i)** No leases exist (A3 §4). A reader-registration table plus heartbeat is application code, i.e. bespoke leases. The DataFusion fork falls away (no PG provider), but a SurrealDB → Arrow read codec is new (A3 §5: no Arrow support).
- **(ii)** Unchanged for PG. The projection needs its own "which generation am I serving" binding, checked against the PG generation's digests (CI-13 "names its snapshot").
- **(iii)** The lease becomes "pin this root". It can be a lockfile/flock or a registry row, and GC must honour it. Any store needs this. The fork falls away only if compile reads come from Arrow/Parquet segments rather than PG.

### M3 Validation of stored sealed contents: framing, digests, receipts, proof reuse — serves O1 (never visible before validation), O2, O4, O5

Realization: `validation_session.rs` 2,135 (945 src), `receipts.rs` 545, `stage_validation.rs` 305, `publication_validation.rs` 152, `validation_views.rs` 52; model `validation.rs` (1,137) with about 160 invariants (A3b §4).

Each relation is re-read from PG at least three times [obs]:

| Read | Location | Purpose |
|---|---|---|
| Stage completion | `receipts.rs:93-104` | frame plus stage receipt |
| Checkpoint | `receipts.rs:316-327` | frame receipts |
| Final `validate_scope` | `receipts.rs:244-253` | frames every relation, then every scoped invariant |

Each invariant also streams its declared inputs in full. A2 infers roughly 250–300 s of the post-cutover facts compile is this phase.

| Dependent | Cat. |
|---|---|
| Framing and re-digesting of stored rows ("validators inspect stored sealed contents, not a caller promise", ADR-0086) | Mostly (a) [int]. It exists because a separate writer session's COPY into a mutable server is not trusted to equal what the producer encoded. A writer that computes the digest over the exact immutable bytes it commits makes re-framing a trust choice, not a necessity. Invariant execution itself is (b). |
| Checkpoint frame receipts and reuse (ADR-0126; control tables `checkpoints`, `checkpoint_frame_receipts`) | (a): a mitigation of repeated re-framing |
| Control tables `receipts`, `validation_receipts`, `stage_receipts`, `stage_outcomes`, `planned_outputs`, `stage_read_checks` (18 control tables in all, `control.sql` 182) | Content binding is (b); proof-reuse bookkeeping is (a)/(b) mixed |
| `lctx generation audit` (600; "explicit read-only recomputation. Normal consumers trust acknowledged owned frames") | (b) under ADR-0126's trust model. It would shrink to digest verification under self-verifying content (iii). |
| Nominal-reference closure ×4 (fact 1) | (c) |
| `ValidationStats` counted but never emitted (A2 §2.6) | (a) observability gap |

**Fate under the candidates.**
- **(i)**
  - `OPTION IMPORT` skips ASSERT, events and views (A3 trap 4). Plain INSERT pays ASSERT per row, and `record<t>` accepts dangling targets on write (A3 §1). A post-load validation pass therefore stays mandatory, and SurrealDB adds no FK-equivalent bulk check.
  - The Rust invariants still need streamed ordered inputs, now via SurrealQL range reads → SurrealValue → Arrow. That is a new codec, with no DataFusion.
  - Net: framing, receipts and validators stay. The FK DDL realization (1) is replaced by bespoke `record::exists` or anti-join passes.
- **(ii)** All of M3 stays. Add a projection-equivalence check (row counts and digests against PG receipts) before the projection may serve (CI-13).
- **(iii)**
  - Framing collapses to "digest computed at write over committed bytes".
  - Invariants stay (b). They become delta-compositional only where partitioned and memoised (A3b §4), and only if the sequential relation digest becomes set- or Merkle-based (A3b §0).
  - Reference closure can be one model-owned check, as in `MemoryGeneration`, removing (c).

### M4 Vocabulary epochs: private deltas, prefix views, publication groups — serves O8

Realization: `vocabulary.rs` 533; control tables `publication_groups`, `publication_outputs`, `epoch_receipts`; live 302 `__delta_*` tables and 84 `security_barrier` prefix views (A1).

Vocabulary is the finite whitelist of hash-consed terms in `stages.rs:492-517`: Assumption*, Literal*, Place*, PathSegment, AccessPath, Predicate, EvaluationAtom, ConditionNode, Condition, …

| Dependent | Cat. |
|---|---|
| Delta tables, grants and revokes, drain-then-merge, security-barrier views with literal prefix bounds | (a) |
| Deduplication of identical payloads, conflict failure, "existing rows never change", old receipts verify their old prefix | (b) |
| `introduced_epoch` hidden physical column plus 12 FKs to `publication_groups` (A1 §2) | (a) |
| `MemoryGeneration` deltas and prefixes (`memory.rs:17-24`) | (c), a second realization of the same rule |
| Tests: `vocabulary_epochs.rs` 1,433 (PG) and `lctx-model/tests/vocabulary_epochs.rs` 572 (memory); `cpg-core/tests/vocabulary_read.rs` 473 | Twin controls of (b), with PG-specific race tests (a) |
| Fix history: d4f0d593, 5262be03, d099aeba (A1 §4) | — |

**Fate under the candidates.**
- **(i)** No equivalent of a security-barrier view with literal bounds exists. A prefix becomes an indexed `introduced_epoch <= N` predicate. Readers cannot be forced to apply it without table PERMISSIONS on record users, and system users bypass them [int; A3 §4 permissions]. Dedup on insert is native for identical content IDs (record id = content key), but conflicting payloads still need detection.
- **(ii)** Unchanged.
- **(iii)** Most natural fit. Vocabulary is already content-addressed, and a prefix is "the union of immutable vocabulary segments up to N" (A3b M4 base+delta). Deltas, grants, views and the hidden epoch column fall away; dedup is native. Kept: the "no meaning change" rule and receipts bound to a prefix manifest.

### M5 DDL and physical lowering — serves O10 and structural parts of O4

Realization: `ddl.rs` 599, `physical_columns.rs` 192, sea-query; CHECK families (A1 §2: 6,274 CHECKs); serving lookup indexes (144) and views.

| Dependent | Cat. |
|---|---|
| CHECKs for Id/Digest width, codes, finite floats and negative zero, list shape, row bytes | (c): duplicates `Record::validate()` called in `Rows::insert` (`rows.rs:20`) and the macro-generated decoders [int]. Defence in depth by stated design. |
| CHECK `generation_id = literal` and composite PK `(generation_id, id)` on 784 tables | (a): per-generation schema plus generation-qualified keys |
| Row-wire-bytes CHECK (`MAX_ROW_BYTES`) | (b) resource policy, duplicated at COPY admission (`mod.rs:655-680`), so (c) |
| sea-query dependency (pins prose `docs/pins.md:105`) | (a) |
| Schema snapshots (insta) | (b) under any store |

**Fate under the candidates.**
- **(i)** Replaced by generated SurrealQL (`DEFINE TABLE … SCHEMAFULL`, typed fields, `ASSERT`, UNIQUE indexes). This is new code of similar size. A3 showed 1,100 tables are accepted on mem; not measured on RocksDB or SurrealKV.
- **(ii)** Added on top of `ddl.rs`.
- **(iii)** Shrinks to the Arrow schema, which is already model-owned (`record.rs:8-30`). Structural checks run once, in the model.

### M6 Load and read codecs — serves O1 and O5 (exact bytes)

Realization: pgpq binary COPY with row-at-a-time encoding (`mod.rs:583-711`), `codec.rs` 88, the fork's Arrow decode, `consumed_rows.rs` 234; pins pgpq `=0.12.0` (`pins.md:39`, must match Arrow).

**Fate under the candidates.**
- **(i)** There is no Arrow/CSV/Parquet loader (A3 §5), so a new Arrow ↔ SurrealValue codec in both directions is required. A3 measured HTTP `/sql` returning 413 at about 2 MB.

  Value fidelity narrows in our favour [obs: `record.rs:8-30`]. Scalars are Text, Bool, Int16/32/64, Id (16 bytes), Digest (32 bytes), Binary and FiniteF64:
  - no u64, so SB046 wrap does not apply;
  - finite floats only;
  - Ids are bytes, so A3 trap 6 (nested-number id collisions) is avoidable;
  - `-0.0` over the wire (A3 §3) still needs a control.
- **(ii)** A new exporter, PG → SurrealDB.
- **(iii)** Arrow IPC or Parquet segments are native to DataFusion. pgpq, COPY and `codec.rs` fall away.

### M7 Projection snapshots (Postcard) and hydration — serves O5 and CI-05

Realization:
- `projection/snapshot.rs` 399, `analysis_graphs.rs` 227;
- relations `projection_snapshots` plus chunks;
- pins `petgraph =0.8.3` and `postcard =1.1.3`, whose stated reason is "immutable typed graph snapshots serialize its structures" (`pins.md:36-37`);
- ADR-0103 option 2. Its stated purpose is to avoid "repeated topology construction and SQL reads" (`0103:24-27`).

| Dependent | Cat. |
|---|---|
| Postcard wrapper, chunking, decode validation, petgraph exact pin and postcard pin | (a): snapshot persistence exists to avoid re-reading edges from row storage. The topology contract (universe, parallel arcs, canonical order) is (b). |
| "Readers hydrate … do not requery" | (a) |
| Snapshot validated against the canonical projection before publication | (c), with the canonical arcs |

**Fate under the candidates.**
- **(i)** SurrealDB edges could replace persisted snapshots for traversal. A2 shows the kernels (SCC schedule, BDD summaries, Leiden, PageRank, FCA, kNN) still need an in-process graph, so either hydration from SurrealDB adjacency replaces Postcard hydration, or snapshots stay. The petgraph and postcard pin reasons would lapse only if snapshots are no longer serialized.
- **(ii)** Unchanged.
- **(iii)** Snapshots become one more content-addressed segment. They are shareable when topology is unchanged, but fact 3 means topology identities change whenever the binary does.

### M8 Serving hydration — serves O2, O4, O6

Realization: 244 `read_for/read_ids/visit_for` one-hop call sites in `lctx-postgres/src/generations` (top files: capability 43, source_usage 40, source_characterization 37, flow_inventory 28, packet_reads 25, operation_sections 23); generated views from `lctx-model/domain/serving/mappings.rs` (1,002); 144 `serving_*` indexes; disposable vector artifact (`vectors.rs` 673, about half of it ACL self-verification per A1); `serving_shape.rs` 69.

| Dependent | Cat. |
|---|---|
| Path declarations, budgets, refusals, packet semantics, ranking (~10k of the 11.2k src) | (b) |
| Chained one-hop reads as separate round trips | (a) |
| Vector artifact ACL and catalog self-verification (`vectors.rs:404-652`) | (a) |
| `lctx_storage` pyo3 binding (1,146 + 17 lines; depends on `lctx-postgres`) and its uv cache keys, which list `lctx-postgres/src/**`, `sql/**`, `migrations/**` and both PG image specs (`python/lctx_storage/pyproject.toml`) | The bridge is (b); the cache-key members are (a) |

**Fate under the candidates.**
- **(i)** Multi-hop paths become `->e->` traversals or `<~` back-references in one query; this is the largest native win (A1 headline 1). Constraints:
  - Single-process embedded engines mean the compiler and several MCP processes require a server (A3 §7).
  - Budgets need `{..N}` bounds, which truncate silently (A3 trap 1). Explicit refusal semantics (O6, CI-08) therefore need a count or probe query per bound [int].
  - Exact vectors need brute-force `<|k,COSINE|>` (A3 §6).
- **(ii)** As (i), plus a proof obligation that every served row exists in the canonical PG generation (CI-11/CI-13). Either re-check against PG, which keeps M8's PG reads, or trust a validated projection digest.
- **(iii)** Nothing changes by itself. Serving still needs an indexed lookup engine over immutable segments, i.e. DataFusion with indexes, or an in-memory index built at startup from the pinned root [int].

### M9 DataFusion provider and `lctx query` — serves O12 and compile stage reads (O8)

Realization: `generation_read.rs` 920, `model_runtime*` 585, `sql.rs` 26, `query.rs` 46; fork patch 1,398; all 19 compile query sites are `SELECT *` (A1 §h).

| Dependent | Cat. |
|---|---|
| Fork, pins row, tokio-postgres, bb8, native-tls and openssl stack | (a) |
| `lctx query` (operator SQL over a leased generation) | O12 convenience |
| Pushdown predicate algebra via `Unparser` (`generation_read.rs:646-760`) | (a) |

**Fate under the candidates.**
- **(i)** Falls away. `lctx query` becomes SurrealQL (or MCP or GraphQL), with no DataFusion over the canonical store.
- **(ii)** Stays.
- **(iii)** Falls away. DataFusion reads Arrow or Parquet directly (§B9 already pins it), and `lctx query` keeps SQL.

### M10 Retained services — serve O7 (cache) and O13 (`lctx_ops`)

| Dependent | Cat. |
|---|---|
| `lctx_cache` (`cache.rs` 178; 0 live rows): one insert-only winner per spec and input | (b) as a cost obligation; any KV fits |
| `lctx_ops` plus `runs` (fact 5) | (d) |
| `postgres_backup.py` 102 plus `postgres_recovery.py` 365: format-4 archive of the 5 retained tables, with restore drill | Only for M10. The archive is (b) for the cache, (d) for ops, and (a) for `_sqlx_migrations`. Semantic generations are never backed up (runbook §Backup). |
| pgvector 0.8.6 extension and `lctx_ext` schema, pinned extension image | (b) exact vector scoring (ADR-0114), PG-realized |

**Fate under the candidates.**
- **(i)** The cache moves into SurrealDB. Backup becomes `surreal export` (SurrealQL text only, A3 §4). Datastore upgrades are one-way (A3 §9), which matters little given rebuild-from-pins.
- **(ii)** Unchanged.
- **(iii)** The cache can be content-addressed files keyed by `(spec, input hash)`, and its backup is a file copy. Store-independent.

### M11 Test and qualification harness — serves every obligation's controls

| Dependent | Size / place | Cat. |
|---|---|---|
| `DisposableDatabase` (testcontainers PG18 with pgvector image; production-like roles; one container per `start()`) plus `Harness` | `testing.rs` 990 | (a) for provisioning, (b) for attempt semantics |
| PG-dependent test files | 63 files / 116 starts / ~27.3k lines; 59 controls in `verify-store` passed per STATUS:37 | The controls are mostly (b); the container setup is (a) |
| `lctx-postgres` substrate-protocol tests | lifecycle, generations, vocabulary_epochs, stage_reads, stage_validation, generation_stages, publication_checks, generation_catalog, analysis_publication, installation, services: ~6.55k lines | Largely (a) |
| `domain_*` PG round-trip tests (codec fidelity), outside every family | 15 files, 3,536 lines with 3 others | (a) codec fidelity; (d)-like exposure, since `qualify` never runs them |
| Recipes `postgres-test-setup`, `images-ready` (inside `just ready`), `postgres-test-ready`; `verify.py` `postgres` readiness and `BOUNDARY_REQUIREMENTS` | `justfile:251-267`, `verify.py:168-180,236-237` | (a) |
| `scripts/postgres_test_support.py` 104 (docker PG for Python tests); `tests/scripts/test_postgres_serving.py` 90 (bootstrap and backup) | — | (a) |
| `specs/postgres-image.txt`, `specs/postgres-vector-image.txt` plus two pins rows (`pins.md:98,101`) | — | (a) |
| Docker as a hard prerequisite: missing Docker is `blocked`; `doctor` checks `docker`, `psql`, `pg_dump`, `pg_restore` | `justfile:208` | (a) |

**Fate under the candidates.**
- **(i)**
  - Tests can use embedded SurrealKV or RocksDB on a tempdir in-process, with no Docker for store controls [int]. Embedded `mem` diverges from persistent engines: no VERSION, different conflict typing (A3 SB033), and engine-specific issues (#7536, #7424, #7430). The AGENTS rule "store tests go through the real store" would then require the production engine (SurrealKV or RocksDB), not mem.
  - Multi-process serving tests need a server, i.e. a container or child process.
  - Lifecycle race tests change shape to conflict-retry tests.
- **(ii)** Docker PG stays, a second engine is added, and projection-equivalence tests are new.
- **(iii)** Tempdir segments plus `MemoryGeneration`-style validation give most controls with no container [int]. Remaining real-store controls cover only pins/GC/concurrency.

### M12 Operational runbook and CLI — serves O1, O10, O12

Realization: `docs/postgresql.md` (175 lines); CLI `store.rs` 88, `generation.rs` 194, `database.rs` 60, `serving.rs` 36, `query.rs` 46, `runs.rs` 69. Steps that exist only for PG:
- package-pinned pgvector install;
- the sudo bootstrap;
- 4 protected configs;
- `sslmode=verify-full` rules;
- the `DATABASE_URL`/SQLX_OFFLINE split;
- lock-capacity tuning plus a service restart;
- the operator transition;
- the role table;
- the connection allowances (eight provider scans plus two lifecycle slots, serving 6 with 2 reserved, admin pool 4 with a 30-min statement timeout).

**Fate under the candidates.**
- **(i)** A different runbook of similar size: server process, users/levels, engine flags (`--graph-fold-interval`, cache sizes, `SURREAL_TRANSACTION_MAX_WRITE_KEYS`), retention, export.
- **(ii)** Both runbooks.
- **(iii)** A directory layout, permissions, a GC command and pins; the PG part stays only if the cache/ops stay in PG.

---

## 3. Category tally (approximate src lines; interpretation over A1's counts plus this lane's)

| Category | What | ≈ lines |
|---|---|---|
| (a) substrate compensation | roles/bootstrap/connection (~0.7k); grant and lock choreography in lifecycle/receipts/locks/install/verify (~2.0k of 2.4k); vocabulary delta/view realization (~0.4k); DDL per-generation and ACL parts (~0.4k); pgpq/codec (~0.3k); fork plus provider (~1.5k + 1,398 patch); LeaseDriver dual-driver part; vector-artifact ACL self-check (~0.25k); Postcard persistence (~0.4k); serving one-hop chaining (~1–2k); test provisioning (`testing.rs` ~0.5k, scripts ~0.3k); substrate-protocol tests (~6.5k test lines) | ~8–10k src + 1.4k patch + ~7k test |
| (b) domain guarantees any design keeps | lifecycle states, failure typing, selection pointer, receipt/content binding, admission, vocabulary immutability rules, budgets, serving semantics, invariants, cache winner rule | the bulk: all of `lctx-model` plus ~10k serving semantics |
| (c) duplicated enforcement | nominal-reference closure ×4; structural CHECKs vs `Record::validate`; row-bytes CHECK vs COPY admission; vocabulary rule in memory and PG; snapshot vs canonical arcs | ~0.3k direct code; the cost is runtime (re-reads) and test doubling |
| (d) historical/transitional | `lctx_ops`, `runs`, `operations.rs`, related backup tables and tests; `services` and `domain_*` targets outside every family (stale exposure, not dead code) | ~0.3k src + ~0.7k tests |

---

## 4. New obligations per candidate (beyond A3's list), tied to this map

| New obligation | (i) | (ii) | (iii) |
|---|---|---|---|
| Schema generation in a new dialect (SurrealQL) from `Relation` declarations; drift check via `INFO FOR DB` | yes | yes | no (Arrow schema) |
| Arrow ↔ SurrealValue codec in both directions (no Arrow import) | yes | export side | no |
| Post-load validation pass (IMPORT skips ASSERT; writes accept dangling `record<t>`) | yes | for equivalence | the model check stays |
| Conflict-retry loops (snapshot isolation, conflict on commit; SDK does not retry; mem error typing differs) | yes | at export | none for write-once segments |
| Bespoke leases and reader registry | yes | projection side | pin registry |
| GC of unreferenced records/segments; batched deletes (#7576) | for per-db retirement or sharing | per projection | yes, core |
| Server operation (multi-process readers); backup via SurrealQL export | yes | yes | no server |
| Build: `surrealdb-core` plus RocksDB C++ (or SurrealKV), non-SemVer core, so an exact pin plus pins row; a new type family outside §B9 (its Arrow-free API avoids family conflict) [int] | yes, if embedded | yes | none; DataFusion and Arrow are already pinned |
| Delta-capable relation digest (set/Merkle) to make sharing and incremental validation pay | — | — | yes (model change) |
| Narrowed provider/model identity scope so sharing exists at all (fact 3, fact 4) | — | — | yes (model decision) |
| ADR obligations: §B7/§B12/ADR-0086 (single PG store) and §B10 ("Excluded: a graph database", DESIGN.md:471) | supersede §B7/§B12/ADR-0086 and §B10 | §B10 exclusion plus §B12/§6.4 ("no copied serving tables"); B10 allows only a "rebuildable *search* projection" | supersede §B7 or amend §6.1 realization |

---

## 5. Second-order effects on the developer loop and operations

| Effect | Current [obs] | (i) SurrealDB primary [int] | (ii) Projection alongside PG [int] | (iii) Content-addressed segments [int] |
|---|---|---|---|---|
| Test store setup | Docker PG18 container per `DisposableDatabase::start()` (116 sites). Installation tests 100–190 s each; fixture compiles 75–115 s each (A2 §2.4–2.5). Docker missing is `blocked`. | Embedded engine on a tempdir; Docker only for server and multi-process tests. Production-engine parity is required (mem ≠ RocksDB/SurrealKV). | Both PG containers and a SurrealDB engine | Tempdir. The existing `MemoryGeneration` plus a segment reader could carry most `cpg-core` controls. |
| Store reset on model change | Any `lctx-model` source edit or `Cargo.lock` move changes the model digest. Install refuses, reset follows, and the real library rebuilds (facts alone 474–669 s, A2). | Unchanged unless identity is narrowed; reset becomes dropping databases | Unchanged plus a projection rebuild | Old roots stay readable only if their model digest is honoured. A new binary still cannot trust old validation without revalidation (definition digests). Reset becomes "new root; GC old". |
| Fact identity coupled to store code | `lctx-postgres` and `specs/` are in the producer digest (fact 3) | Store code stays inside the closure unless excluded | Same | Same; exclusion is a fingerprint-scope decision |
| Schema migration practice | Insta snapshots; "schema change = migration = reset" | Same plus a SurrealQL rendering snapshot | Two renderings | Arrow-schema snapshots only |
| Python adapter (`lctx_storage`) | pyo3 over `lctx-postgres`; uv cache keys include PG sources, SQL and image pins; native rebuild on any such change | Binding links a SurrealDB client (ws), or embeds core (heavy C++ build; then single-process, so MCP processes cannot share a store) | Both drivers in the binding, or serving moves fully to SurrealDB | Binding reads segments (DataFusion/Arrow already in the workspace); PG cache keys drop |
| Build dependencies | sqlx stack, tokio-postgres/bb8/native-tls/openssl via the fork, pgpq, pgvector, sea-query; 237-package `lctx-postgres` closure | Remove the PG closure, add `surrealdb` (+ core, RocksDB/C++ or SurrealKV) | Additive | Remove most of the PG closure if the cache leaves PG too |
| Backup/restore | Retained-service archive only; generations rebuilt | `surreal export` text; one-way datastore upgrades | Both | File copy; rebuild unchanged |
| Observability | Tracing per stage; the validation phase is uninstrumented (A2 §2.6); `lctx::postgres` cache tracing; pg_catalog | Not investigated beyond A3 (server logs, `INFO`) | Both | Application tracing only |
| Qualification families | `verify-store` and `verify-serving` need `postgres` readiness; `qualify` unions them | Replace `postgres` readiness with an engine/server readiness | Add a readiness | `postgres` readiness drops for store controls |

---

## 6. Material uncertainties and unresolved edges

1. **How many FKs a validated generation installs** is still unmeasured: the live generation is staging (A1 §2), and a new compile is running (the control schema now shows 4 `stage_outcomes`). The model-side count needs `Relation::fields()[].target()` enumeration; a grep for `Id<…>` (3,834 hits) is not a valid proxy.
2. **"Validation re-reads are substrate compensation"** is an interpretation of ADR-0086's "not a caller promise". The repository does not state why writer-side digests are insufficient. The reviewer should confirm the intended trust reason: tamper, codec fidelity, or concurrent writers.
3. **Sharing estimate.** Fact 3 shows provider-keyed records change with any workspace edit. I did not enumerate which relations carry no `run`/`provider` key (entities, occurrences, artifacts), so a narrowed-scope sharing fraction remains A3b probe B1.
4. **Tests outside the families.** I did not establish whether the 49 PG-dependent targets outside `verify-*` are intentionally retired from qualification or just unselected. STATUS reports only the family results.
5. **`audit.rs` line count.** It includes inline tests (A1: 522 src).
6. **Absence claims and their search scope:**
   - `start_attempt` with no production caller: grep of `crates/` and `python/` for `start_attempt|.event(`.
   - openssl only via the fork: `cargo tree -p cpg-core -i openssl-sys`. The other workspace roots (`lctx`, `python/*`) were not checked.
   - Nominal-reference realizations: grep of `lctx-postgres/src` and `lctx-model/src` for `nominal_references`, `target()` and `references(`. Other crates were not searched.
7. **Engine and capability claims are taken from A3/A3b, not re-verified.** That covers SurrealDB, VERSION and import behaviour.
