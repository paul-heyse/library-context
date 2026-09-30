# Semantic-model cutover, P1.10: generation-bound provider sessions (bounded review)

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Commits `1ffe05c` (P1.10) and `5beafd8` (its adjustment by the store-lifecycle corrections):<br>• `crates/lctx-postgres/src/generations/{lease.rs,locks.rs}` and the lease-related parts of `ddl.rs`/`mod.rs`<br>• `crates/cpg-core/src/generation_read.rs`<br>• `crates/cpg-core/tests/generation_read.rs`<br>• `StageSession::register` in `crates/cpg-core/src/model_runtime.rs`<br>• the owned provider fork `build/datafusion-table-providers-owned` at `09cc8a8` (`crates/postgres/src/{bounded.rs,conn.rs,pool.rs,arrow_sql_gen/mod.rs}`; pinned in the workspace `Cargo.toml:24`).<br>Line citations are at `5beafd8` (fork at `09cc8a8`). **HEAD moved during the review:** P1.11 (`244eda4`) changed `InspectionSession` to register out-of-frontier relations as a refusing `OutsideFrontier` provider, which shifts `generation_read.rs` lines after 153 by about +16; P1.12/P1.13 (`c65c0c7`…`dcbc593`) did not touch the subject. The P1.11 change is examined as an adjacent consumer because it breaks a P1.10 control (F01) |
| Standard | [Core 3.0 and template](../design_principles/core/design-principles.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md) with [review additions](../design_principles/profiles/code-intelligence/review.md), [repository binding](../design_principles/binding/library-context.md) |
| Tier · purpose | **Change · conformance**: the binding's bounded review after P1.10 (plan §4.1.1 order), run at design depth because it fixes the reader contract that P3–P5 build on. Judged against:<br>• plan §4.1.1 rows P1.9/P1.10 and T13 (`semantic-model-cutover-plan_2026-09-29.md:201, 263-264`);<br>• core review [C01](design_review_cutover-core_2026-09-29.md#C01) (lease half) and [C02](design_review_cutover-core_2026-09-29.md#C02);<br>• P0 exit [F02](design_review_p0-exit_2026-09-29.md#F02) and [F07](design_review_p0-exit_2026-09-29.md#F07);<br>• [DESIGN §15.11](../../design/sections/semantic-model.md#section-15-11) |
| Reviewer · date | Fresh-context `design-reviewer` subagent (not the author), 2026-09-29 |
| Maturity and outcome | Phase 1 of the hard layered cutover. The decision says whether the lease protocol, the bound provider pool and the table/scan contract are the right reader for P3/P4 stage inputs, `lctx query` and P5 serving, and whether C01 (lease half), C02 and P0 exit F02 (reader half) close |
| Supported scope | The driver-neutral lease; `GenerationSession` (open, table, health, close); `GenerationTable` with its closed pushdown algebra; `GenerationScan` over the fork's reserved, bounded, drain-on-drop stream; `InspectionSession`; the stage-registration seam. CI concerns: CI-04 (unknown is not absent at the reader) and CI-13 (a reader is pinned to one generation). No facts are produced and no claims are served |
| Exclusions | P1.11 CLI semantics beyond the `InspectionSession` change; P2 producers; P5 serving implementation; integrated gates and pilots (binding acceptance timing) |
| Expected changes | The five requested scenarios (§5):<br>1. P2/P3/P4 stage reads;<br>2. P5 serving and `lctx query --generation`;<br>3. retire or selection change during a long read, transport loss, cancellation;<br>4. a new frontier and new relation shapes;<br>5. memory pressure |
| Baseline | [Store-lifecycle review](design_review_p1-store-lifecycle_2026-09-29.md) and its [re-inspection](design_review_p1-store-lifecycle_2026-09-29.md#re-inspection) (Accept scoped). Its F07 lease-terms correction, R1 and R2 residuals, and deferred F08 (frontier policy in the store) bear on this slice |
| Method and coverage | Read in full:<br>• `lease.rs`, `locks.rs`, `generation_read.rs` (at `5beafd8` and `244eda4`), the tests, `model_runtime.rs:1-60`, `testing.rs` fixtures;<br>• the fork diff `09cc8a8` (`bounded.rs`, `conn.rs:520-670`, `pool.rs` bound-pool parts, `arrow_sql_gen::conform_declared`);<br>• DataFusion 55.1 `sql_with_options`/`execute_logical_plan`;<br>• bb8 0.9.1 `build`/`add_connection`;<br>• the P1.11 `query.rs` consumer.<br>Checks were run where doubt was material (§10). Not examined: fork modules outside the diff, and the dormant serving/native-executor code |

This review is evidence, not a status register. Current disposition belongs in plan §8 (binding §4); §11 proposes the rows.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility and hidden decisions | Consumer contract | Direction | Expected reason for change |
|---|---|---|---|---|
| `generations::lease` | The lease protocol, independent of the driver. Order: installation lock (S, xact) → installation digests → generation lock (S, xact) → registry state, frontier and frontier-scoped digests → live column signature → generation lock (S, session) → COMMIT. Release is confirmed. Hides the statement text and a one-text-column row protocol (`lease.rs:37-82`) | `LeaseContract::{new, acquire, release, generation}`, `LeaseDriver{text, batch}`, `LeaseParam` | → `ddl` (scopes, `LIVE_COLUMNS`), `locks`; no driver | A lease term (relation set, availability), a new frontier |
| `generations::locks` | Keys, try-locks, read-only `pg_locks` probes (`locks.rs:12-61`) | Crate-private | — | Key derivation |
| `generations::ddl` (scopes) | Per-frontier relation set, physical digest and expected column signature (`ddl.rs:26-58`) | `Scope` | → model | A frontier, a scalar type |
| `cpg-core::generation_read` | Opens a bound pool of N leased connections through `Binder` (`:75-95, 105-124`). Tables carry exactly the declared schema and push down only the closed algebra (`:211-292`); scans go through the fork (`:295-325`). Also: the inspection context (`:155-169`), the tokio error classifier (`:54-58`), the pinned driver session (`:182-205`), and the frontier's held set (`:171-180`) | `GenerationSession::{open, table, frontier, generation, health, close}`, `GenerationTable`, `GenerationScan`, `InspectionSession::{new, sql, close}`, `ReadError`, `admits_expr` | → `lctx-postgres` (lease, roles), fork, DataFusion | P3/P4 inputs, P5 serving, a new frontier |
| Fork `bounded`, `conn::query_arrow_bounded`, `pool::new_bound` | Row- and byte-bounded chunks; a reserved stream that owns its connection and drains on drop; a prefilled pool whose connections are each bound once, counted by `remaining` so none is replaced, with a terminal `lost` flag (`conn.rs:529-670`; `pool.rs:312-322, 345-409, 487-560, 783-786`) | `ChunkLimits`, `BoundLimits`, `SessionBinder`, `PoolHealth`, `query_arrow_bounded`, `connect_direct`, `health` | → tokio-postgres, bb8 0.9.1, DataFusion 55.1 | Transport, cancellation and budget policy |
| `model_runtime::StageSession::register` | Binds a provider to a stage read permit: stage identity, `R::schema` equality, one registration per relation (`model_runtime.rs:32-37`) | `register(&ReadPermit<R>, Arc<dyn TableProvider>)` | → DataFusion | P3/P4 cross-generation inputs |

Compile-time direction is sound: `cpg-core` → `lctx-postgres` (lease terms) → `lctx-model`. The fork is reached only from `cpg-core`. The lease protocol has one owner and two drivers, which is the planned consumer that justifies the two-method `LeaseDriver` seam.

| Concept | Semantic authority | Derived forms and consumers | Second copy? |
|---|---|---|---|
| What a reader must confirm | `LeaseContract` built from `ddl::scopes` (`lease.rs:29-34`; `mod.rs:264-266`) | `pin` over SQLx; the provider `Binder` over tokio-postgres | No |
| A frontier's relation set | `FrontierContract::facts` through `ddl::scopes` (`ddl.rs:26-33`) | Store scope, `GenerationLease.relations` | **Recomputed at the reader by `held()` (`generation_read.rs:171-180`): F06** |
| Live column shape | `ddl::lower` (`ddl.rs:109-196`) | `column_signature` (`:36-54`) restates the scalar-to-type map | Restated, fails closed (O3) |
| Pushdown semantics | `admits_expr` (`generation_read.rs:222-240`) | `supports_filters_pushdown` and the `scan` recheck (`:274-282`) | No |
| Failure class | `Error::class` (`mod.rs:53-71`), `FailureClass::of` (`failure.rs:61-74`) | The tokio `classify` (`generation_read.rs:54-58`); scan errors reach callers as fork types | **Three SQLSTATE tables that disagree: F03** |
| Family availability | `admission_families` (store, typed) | The catalog | **Not exposed at the reader: F07** |
| Session liveness | The fork's `lost` flag and `PoolHealth` | `GenerationSession::health` | `close` is not represented: F04 |

**Fact and fidelity (CI profile).** No fact families are produced here. The slice carries two fidelity obligations. A relation outside the frontier must never read as empty: it refuses with a typed `Frontier` (P0 exit F02). A NotRequested family should not read as absent (CI-04, F07).

## 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs | Preconditions and enforcement | Effects, lifecycle, failure | Isolated verification |
|---|---|---|---|---|
| `LeaseContract::acquire(driver)` | → the generation's `Frontier` | Installation model/physical digest; generation registered (`Absent`), published (`State`), model digest and frontier-scoped physical digest (`Contract`); live columns equal the lowering (`Contract`) (`lease.rs:55-75`) | One transaction. The session lock is taken last, so any refusal leaves no lock; a failed COMMIT is `Commit`/`Unconfirmed`, and the caller discards the connection (`:41-52`) | `generation_read` refusal cases; `lifecycle` lease races |
| `GenerationSession::open(RoleConfig, model, g, options)` | → session over N leased connections | Serving role only; `connections ≤ provider_connections ≤ 2` (`roles.rs:51-60`); URL options closed (`generation_read.rs:182-205`) | Every refusal happens before any scan. The binder keeps the first typed refusal (`:88-93, 114-119`). bb8 retries a refused bind, but the `remaining` counter turns the retry into `Lost` rather than an extra connection (O2) | `digest_and_column_mismatch_rejected_before_scan` |
| `table::<R>()` | → `Arc<GenerationTable>` | `R` in the model and in the held set, else `ReadError::Frontier` (`:129-139`) | The table holds the pool `Arc` and a fixed `schema.relation` source | `pin_survives…` (line 150) |
| `GenerationTable::scan` | Projection, filters, limit → `GenerationScan` | Filters rechecked against the algebra (`:280-281`); the declared schema only | SQL `SELECT cols FROM "lctx_g…"."rel" [WHERE …] [LIMIT n]`; one partition | `closed_filter_pushdown` |
| `GenerationScan::execute` | → a bounded stream | A reservation from the task's pool (`:318`); chunks of at most 4096 rows / 8 MiB, one row at most 64 MiB | The stream owns its connection: it returns on completion, drains on drop, and is lost on transport failure (§4) | `byte_bounded…`, `cancellation…`, `transport_loss…` |
| `close()` | → confirmed release of N leases | Not already `Lost` | Returns connections to the pool; **does not close it** (F04) | `lease_blocks_retire_close_releases` |
| `InspectionSession::sql` | SQL → `DataFrame` | DDL, DML and statements refused (`:166`); at `244eda4` every model relation is registered and out-of-frontier ones refuse at physical planning | Read-only, hermetic context; **unbounded memory pool** (F05) | `pin_survives…`; P1.11 `store_cli` |

**The closed pushdown algebra** (`generation_read.rs:211-265`). Each element is judged against DataFusion's meaning:

| Element | Unparsed PostgreSQL | Fidelity | Executed known answer |
|---|---|---|---|
| Qualified column | Unqualified quoted identifier (`:243-249`); a scan names one relation | Exact | Yes (SQL path) |
| Text `=`/`<>`, with DataFusion's `CAST(col AS Utf8View)` | The cast is admitted only from `Utf8` and dropped (`:251-259`), giving text equality; database collations are deterministic, so equality is bytewise | Exact | `=` yes; `<>` yes (stage test) |
| Text ordering, `LIKE`, functions, `IN`, cross-type casts, list or sum columns | Not admitted, so DataFusion filters (`:217-240`) | Conservative | The order twin does not discriminate (F02) |
| Integer ordering and equality, same type Int16/32/64 | Same operators | Exact | `ordinal >= 64` → 2 |
| Bytea literal (Binary, FSB16/32) against a Binary/FSB column | `CAST('\x…' AS BYTEA)` (`:260-263`), not the bit-string `X'…'` | Exact **only with `standard_conforming_strings=on`** (F02) | Classified only (F02) |
| `AND`/`OR`/`NOT`, `IS [NOT] NULL`, NULL operands | Identical SQL three-valued logic; WHERE drops NULL as DataFusion's filter does | Exact | Classified only (F02) |
| `LIMIT` | Pushed by DataFusion only above a scan whose filters are all Exact | Exact | Not directly |

**Live-column check** (`lease.rs:70-72`; `ddl.rs:36-58`). It compares every ordinary table in the generation schema: each live column's name, `format_type` and `attnotnull`, in C-collated relation order and `attnum` order. Views are excluded, and their base tables are covered. The expected string restates the lowering's scalar map. Both maps are exhaustive over `Scalar`, so drift refuses every lease (fails closed, O3). It does not cover constraints, defaults, collation, RLS, policies or triggers, nor owner DDL after acquisition (leases are advisory; store-lifecycle O3). `store check` covers those (`verify.rs:211-237`). **Sound** for its purpose, C02: the reader refuses a lowering other than its own before any scan. The added-column twin discriminates it.

**Absence states at the reader.** These are distinct:
- absent (`Absent`);
- unpublished (`State`);
- another model or lowering, or tampered columns (`Contract`);
- outside the frontier (`ReadError::Frontier`; at `244eda4` also in `InspectionSession`);
- lost (`PoolHealth::Lost`, with no typed scan error: F03).

Collapsed: NotRequested and Unavailable families read as empty tables (F07). After `close`, a retained table reads without a lease until the schema vanishes, then fails with a raw `42P01` (F04).

## 4. Composition and execution

`open` → `new_bound` (bb8 `min_idle = max_size = N`, no reaper) → N × (`connect` → `configure_session` → `Binder::bind` → `LeaseContract::acquire`) → `table::<R>()` → DataFusion plans → `scan` → `execute` → `connect_direct` → `query_arrow_bounded` → chunks → batches.

**Lock order against the store lifecycle**:

| Operation | Order | Can it wait? |
|---|---|---|
| Lease acquire (`pin`, `Binder`) | installation S xact → generation S xact → generation S session | Only behind an exclusive generation lock. A lifecycle step holds one on an *unpublished* generation, and the lease then refuses `State`; retire holds one briefly, and the lease then refuses `Absent` (`lease_vs_retire_race`) |
| Retire, abort | installation S → selection row → *try* generation X ∧ attempt X | Never waits on a lease: `Busy` |
| Seal, validate, publish, fail | installation S → generation X on their own unpublished generation | A lease holds S xact there only momentarily, and is refused before the session lock |
| `check`, `reset`, `install` | *try* installation X for about 1 s | Never queues. Reset withdraws the installation, and later leases refuse `Contract` |

Every blocking acquisition runs installation → generation. A session lock exists only on a published generation, which no blocking step targets. Removal uses try-locks. So there is no wait cycle, and T10's order is preserved.

**Drain-and-return and terminal loss** (fork at `09cc8a8`):

| Event | Path | Outcome | Evidence |
|---|---|---|---|
| Stream completes | `None` → `lease.end()` → `finish_request`, connection returned (`conn.rs:641-643`) | Ready | Tested (every readback) |
| Consumer drops mid-read | `Drop for Lease` spawns CancelRequest plus a confirming `simple_query("")` within `drain_timeout`, then returns the connection or marks the pool lost (`conn.rs:545-566`; `pool.rs:312-316`) | Ready, or Lost | Tested (three drops, same PIDs); failed-drain branch by code read |
| Reservation or row-bound refusal | `yield Err; return` drops the Lease, which drains and returns | Ready | Tested (1 MiB pool) |
| Server error mid-read | `Some(Err)` → `end()`; the connection returns, or is lost if closed (`:636-640`) | Ready | Code read |
| Transport closed | `end()` sees `is_closed` → `mark_lost`; a broken connection marks lost in `has_broken`; bb8's replenish `connect` hits `remaining = 0` → `Lost` (`pool.rs:350-353, 402-409`) | Lost, never replaced | Tested (terminate before read; no new backend) |
| Drop while `query_raw` is still planning, before the Lease exists | `start_request` precedes the await (`conn.rs:596-597`); the connection returns while streaming, `has_broken` fires | **Lost** (safe, but not drain-and-return: F04) | Code read |
| `close` | N × `connect_direct` + confirmed unlock | Leases released; **pool still usable** (F04) | Tested |

The session lock survives CancelRequest and the transaction abort it causes, so a drained connection still holds its lease. No test asserts this (F01). The cancel can arrive after the query has finished, while the confirming empty query runs; that query then fails and the pool is lost spuriously. This is safe, but it costs liveness (O5).

**Analysis record (CI profile, slot 4)**:

| Question | Projection | Method | Exactness and model | Budgets and partial results | Output linkage |
|---|---|---|---|---|---|
| Rows of relation R in leased generation g | Universe: R in g's schema. Selector: the closed algebra; the rest is filtered by DataFusion | Exact SQL | Exact under the algebra (§3) | Chunks ≤ 4096 rows / 8 MiB, one row ≤ 64 MiB (refused above), charged 2 × retained to the stage pool. Every limit is an error, never a truncated result; `LIMIT` only when DataFusion pushes it | The table's source is fixed to `lctx_g<hex>.R` at `table()`; the generation id is the lease's |

## 5. Change and failure scenarios

| Scenario and trigger | Owner | Contract change | Expected vs observed | Hidden knowledge / test setup | Evidence |
|---|---|---|---|---|---|
| **1. P2 stages read prior-stage relations; P3/P4 read published facts generations** | `model_runtime`, `generation_read` | P2: none. P3/P4: declare an input generation | **P2:** a stage's in-attempt inputs are D0 handoffs or retained batches, not provider sessions. Sessions require `published`, and staging schemas grant the writer INSERT only (`ddl.rs:191`). The scenario, as phrased, is not the design (O6). **P3/P4:** a published facts generation composes through `open` → `table` → `StageSession::register` (Tested, `stage_session_registers_generation_table`). But `register` binds *any* provider under a permit, by relation name. The test itself registers another generation's `packages` for a relation the same schedule's `source` stage produced. The attempt records neither the input generation nor its content digest, and `held()` restates the frontier scope (F06) | The input generation's identity lives only in orchestration | Code read. **Unresolved → P3 frontier design** (with store-lifecycle F08) |
| **2. P5 serving and `lctx query --generation`** | `InspectionSession` | None | Read-only, hermetic, one generation per session (CI-13). Frontier refusal is typed at `244eda4`. The context uses DataFusion's unbounded pool, so the scan's reservation can never refuse (F05). A long-lived server must reopen after `Lost`; that is explicit, never a silent reconnect. The P1.11 consumer classifies by downcast and a `"not supported"` string match (`crates/lctx/src/query.rs`, `244eda4`; F03) | Error classes need fork types | Tested (P1.10); P1.11 historical receipt |
| **3. A long read during retire or selection change; transport loss mid-read; cancellation** | Lease, fork | None | A lease blocks retire (Tested); selection is irrelevant by construction; transport loss is terminal and never replaced (Tested); a mid-stream drop drains to the same backends (Tested). Gaps:<br>• the pin-survives control cannot tell the two generations apart (F01);<br>• the lease surviving a drain is unasserted (F01);<br>• cancellation before the first response loses the session (F04);<br>• tables outlive `close` (F04);<br>• scan errors are untyped (F03) | Consumers must call `health()` to learn `Lost` | Tested / code read |
| **4. A new frontier; new relation shapes** | Model, `ddl`, `generation_read`, fork | New `Frontier` arm | *Frontier:* `LeaseContract` is generic over the `scopes` map. `held()` gets an exhaustive `match`, so the compiler flags it, but it restates the scope computation (F06) on top of store-lifecycle F08's five store sites. *Shapes:* lists conform element by element (`conform_declared`, fork unit tests; `AnalysisContext` lists read back); sums read back (`CoverageScope`); reference collections are relationship rows by model rule; list and sum columns are never pushed down; a new `Scalar` breaks exhaustive matches in `lower`, `column_signature` and the fork type map, so the failure is loud | None beyond F06 | Tested (lists, sums); code read |
| **5. Memory pressure** | `GenerationScan`, `AttemptRuntime` | None | A stage session's pool *is* the attempt `ResourceBudget` (`model_runtime.rs:12-17`), and scans reserve from it (Tested: a 1 MiB pool refuses and the session stays Ready). Peak scan charge is about 2 × (8 MiB + 64 MiB). Yielded batches are charged by their consumers (DataFusion convention). `InspectionSession` is unbounded (F05) | — | Tested |

**CI journey — a module full of unresolved references**, applied to families. A catalog-profile facts generation holds Flow relations inside its frontier; they are empty, and coverage marks the family NotRequested. `session.table::<FlowUse>()` and `SELECT count(*) FROM flow_uses` return zero rows, with nothing beside them to say "not requested" (F07).

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | pass | One lease definition from `ddl::scopes` serves both drivers. Latent: the reader recomputes the frontier scope (F06), and three SQLSTATE classifiers exist (F03). Neither yet disagrees on a supported input | F03, F06 |
| G2 Semantic fidelity | pass | The algebra is faithful under the deployed defaults (§3), and declared-schema conformance refuses nulls and width mismatches. It depends on an unpinned GUC (F02) | F02 |
| G3 Validity | pass | Installation, state, digests and live columns are all checked before any scan (Tested); declared non-null and width are enforced at decode (fork tests) | — |
| G4 Hidden behavior | pass | Tables discover nothing from the database; the inspection context cannot write or register; URL options are closed. `standard_conforming_strings` is an ambient server input the unparsed SQL depends on (F02, default-safe) | F02 |
| G5 Consistency and recovery | pass | A lease blocks retire; loss is terminal; drain returns the connection; no reader can see a partial or retired generation as data. F04 (unleased reads after `close`) fails loudly | F04 |
| G6 Transformation and reuse | pass | Exact pushdown preserves meaning by construction; the conservative fallback is DataFusion's own filter. Executed evidence for some branches is missing (F02) | F02 |
| G7 Truthful capability claims | **unresolved** | Two named controls do not establish what they claim. The review-focus #4 / P0 exit F02 control (`pin_survives_selection_change_and_frontier_is_enforced`) cannot tell which generation it read. Its `InspectionSession` negative passed at `5beafd8` on an unknown-table planning error, and it **fails at HEAD** (§10). The plan §4.2 receipt ("refused by type and by SQL", line 797) and the §8 rows cite it | F01 |
| G8 Library leverage | pass | DataFusion `TableProvider`/`Unparser`, bb8, tokio-postgres cancellation and DataFusion reservations are used where they fit. The bespoke admission layer is about 60 lines, justified by fidelity (§8) | — |
| CI-G1 Fidelity | pass (current scope) | Outside the frontier refuses. The NotRequested disclosure at the reader is a latent CI-04 gap for P3–P5 consumers (F07); no published result reads it today | F07 |
| CI-G2 Evidence closure | n.a. | No served claims | P5 |
| CI-G3 Evaluation integrity | n.a. | No evaluation inputs | — |

## 7. Findings

Severity:
- **Medium:** a contract or evidence defect with a concrete consequence in a requested scenario, or a failing control behind a claimed closure.
- **Low:** a diagnostic, a default-safe hidden dependency, or a barrier to a later package.

<a id="F01"></a>
### F01 · Medium · `cpg-core/tests/generation_read.rs`: the reader controls for review focus #4 and P0 exit F02 do not discriminate, and one fails at HEAD

- **Principles · judgment/gate:** DP-23, DP-22, FP-04 (independent controls) · G7.
- **Evidence.**
  1. `pin_survives_selection_change_and_frontier_is_enforced` (`tests/generation_read.rs:135-158`) publishes `first` and `second` with `facts.published` from the same fixture (`testing.rs:258-295`), so both hold identical rows. The known answer "the session still reads its own generation", two coverage rows (`:145-146`), is equally true if the table read `second`.
  2. `:152` asserts `inspection.sql("SELECT count(*) FROM transfer_keys").await.is_err()`, which accepts any error:
     - at `5beafd8`, `InspectionSession` registered only held relations (`generation_read.rs:158-164`), so the error was DataFusion's unknown-table planning error, not a typed `Frontier`;
     - at `244eda4`, P1.11 registers `OutsideFrontier`, which refuses in `scan`, so `sql()` plans successfully (DataFusion 55.1 `sql_with_options` does not create a physical plan) and the assertion fails. **failed** (§10).
  3. `cancellation_mid_stream_drains_and_returns` (`:196-217`) asserts that the connections return to the same two PIDs and that a later read succeeds. It does not assert that each connection still holds its lease. A drain that reset the session (for example `DISCARD ALL`, which releases advisory locks on the same backend) would pass.
- **Consequence.** Plan §8 cites this suite as closure evidence: the P0 exit F02 reader row (line 1104) and the C01/C02 rows ("closure pending the P1.10 bounded review", lines 1139-1140). The plan §4.2 P1.10 receipt says "refused by type and by SQL". At HEAD the suite is red. Several regressions would pass the P1.10 controls:
   - a table that resolves through the selection pointer (a plausible P5 change);
   - a typed refusal that decays into an untyped planning error;
   - a drain that drops the lease.
- **Correction** (owner: the `cpg-core` test; no production change).
  - Give the two generations distinguishable content, for example one extra provider or coverage row in `second`, and assert `first`'s exact rows after `select(second)`.
  - Assert the refusal after `collect()` and downcast the root to `ReadError::Frontier`, keeping successful planning as the expected P1.11 behaviour.
  - After the three drops, assert that `retire` is `Busy` and that the catalog counts two readers.
  - Correct the "by SQL" receipt sentence.
- **Closure evidence.** The corrected suite passes at HEAD, and each new assertion fails against a deliberately wrong twin: the same content, a planning-only assertion, an unlock in the drain.

<a id="F02"></a>
### F02 · Low · `GenerationTable` pushdown: executed known answers are missing for bytea literals and NULL logic, and the SQL depends on an unpinned GUC

- **Principles · judgment/gate:** DP-08, DP-23, DP-18 · G2, G4, G6.
- **Evidence.**
  - `closed_filter_pushdown` (`tests/generation_read.rs:85-115`) *classifies* the bytea equality (`id = FixedSizeBinary`) and `IS NOT NULL`, but executes only text equality, text order (Unsupported), `LIKE` (Unsupported) and integer order. The bytea rewrite `CAST('\x…' AS BYTEA)` (`generation_read.rs:260-263`) never runs against PostgreSQL.
  - The negative twin for text order uses lowercase ASCII names, whose order is the same under C byte order and a linguistic collation. It would not detect text ordering pushed to a non-C database (the test database is the pgvector image's default).
  - The provider session pins `search_path`, read-only mode and timeouts (`:202-203`), but not `standard_conforming_strings`. With it off, `'\x…'` and backslashes in text literals become escape strings, and equality silently changes.
- **Consequence.** Scenario 3 and P3/P4 joins keyed on ids depend on the untested branch. A regression in the hex rewrite (for example a doubled backslash) would return zero rows for a pushed-down id equality, with no error. A server with the GUC off would give wrong Exact answers.
- **Correction.**
  - Add `-c standard_conforming_strings=on` to `driver_config`.
  - Add executed controls:
    - `id = <bytes of beta>` returns exactly `beta`;
    - over a nullable column holding a NULL row, `NOT (col = x)` and `col <> x` return DataFusion's answer;
    - a mixed-case pair (`'Beta'`, `'alpha'`) where byte order and `en_US` order differ, whose answer is the byte-order one.
- **Closure evidence.** Those controls pass, and the option is present in `driver_config`.

<a id="F03"></a>
### F03 · Low · SQLSTATE classification has three owners that disagree, and provider scan errors reach consumers untyped

- **Principles · judgment/gate:** DP-21, DP-01, FP-02 · A1 (scenario 3); G1 (latent). This finding also carries [store-lifecycle R1](design_review_p1-store-lifecycle_2026-09-29.md#re-inspection).
- **Evidence.**
  - The tokio `classify` (`generation_read.rs:54-58`) maps anything that is not transport to `Refused`, including 55P03, 57014, 40P01 and 40001. `Error::class` maps those to `Contention` (`mod.rs:61`). `FailureClass::of` adds 23 → `invalid` and 53/54 → `limit`, but only for lifecycle steps (`failure.rs:61-74`).
  - On the stage path, `From<Error> for ModelError` (`mod.rs:74-83`) keeps `Error::class()` (23/53/54 → `Refused`) and a detail of "PostgreSQL operation failed" (`:30`). A producer's `fail(&cause)` therefore stores a COPY constraint violation as `refused` with no SQLSTATE.
  - Scan failures reach consumers as `DataFusionError::External` wrapping fork-private types: `pool::Error::BoundPoolLost`, `PostgresError::QueryError`. `ReadError` has no `Lost` variant; loss is visible only through the fork's `PoolHealth`.
  - The first consumer already compensates: `crates/lctx/src/query.rs` (`244eda4`) downcasts `ReadError::Frontier` and string-matches `"not supported"`.
- **Consequence.** Several consumers decide retry versus report versus abort on this class: the P3/P4 stages and P5 reading through sessions, and the A0/T9 driver deciding what a required provider's failure means. They receive the same lock timeout as `Contention` from one driver and `Refused` from the other. Telling `Lost` from a refusal needs fork types. A producer's invalid-row COPY is persisted as `refused` without its constraint.
- **Correction** (owner: `generations::failure`; small).
  - One SQLSTATE → class function, used by `Error::class`, `FailureClass::of` and, exported, by cpg-core's tokio `classify`.
  - Carry the safe `Failure` detail and the SQLSTATE-derived class across `StageSink`, for example by extending `Infrastructure` or mapping class 23 to `ModelError::Invalid`.
  - In `GenerationScan`, map stream errors into `ReadError` (`Lost`, `Store(Driver{class})`) inside `DataFusionError::External`, so consumers downcast one owned type.
- **Closure evidence.**
  - A stage COPY CHECK violation persists `invalid` with its SQLSTATE and constraint.
  - A provider-session 55P03 classifies as `Contention`.
  - A scan after transport loss yields a downcastable `ReadError::Lost`.
  - `query.rs` drops its string match.

<a id="F04"></a>
### F04 · Low · Session lifecycle edges: `close` releases the leases but leaves the pool usable, and a cancellation before the first response loses the session

- **Principles · judgment/gate:** DP-19, FP-05 · G5 (fails loud, so pass).
- **Evidence.**
  1. `GenerationTable` holds `Arc<PostgresConnectionPool>` (`generation_read.rs:137-138`). `close` releases each lease and returns the connection (`:142-152`), but nothing marks the pool closed, and `health()` stays Ready. A table retained elsewhere, for example in a `StageSession`, keeps scanning on unleased connections. `stage_session_registers_generation_table` keeps `stage` alive past `session.close()` (`tests/generation_read.rs:255-260`).
  2. `query_arrow_bounded` sets `streaming` before awaiting `query_raw` and builds its drain guard only afterwards (`conn.rs:596-597`, `Lease` after). A drop during parse or plan returns a streaming connection, and `has_broken` marks the pool `Lost` (`pool.rs:402-409`); it is not drained and returned.
- **Consequence.**
  - Scenario 3: after `close`, a retire succeeds while a retained table still exists. That table's next scan fails with a raw `42P01`, not a typed `Lost`/`Absent`. Or, if a scan is in flight, the retire's DROP waits on it for up to the owner's `lock_timeout`. §3.2's "a reader … holds a lease for its lifetime" does not hold for tables that outlive `close`.
  - A cancelled query in its planning window makes a healthy session terminal. That is safe, but it contradicts T13's "a confirmed drain returns the connection".
  - No wrong data is possible, because schemas are immutable and never reused.
- **Correction.**
  - `close` marks the pool terminally closed (a fork `close()` sharing the `lost` flag with a distinct health variant), so later acquisitions refuse with a typed error.
  - Build the `Lease` guard before `query_raw`, so a drop in that window drains like any other.
- **Closure evidence.**
  - A table retained after `close` refuses to scan with a typed error, and `retire` succeeds.
  - A stream dropped before its first batch leaves the session Ready, on the same PIDs.

<a id="F05"></a>
### F05 · Low · `InspectionSession` runs on DataFusion's unbounded memory pool

- **Principles · judgment/gate:** DP-20 · A1 (scenario 5).
- **Evidence.** `InspectionSession::new` uses `SessionContext::new()` (`generation_read.rs:159`), whose `RuntimeEnv` pool is unbounded. The scan's `MemoryReservation` (`:318`) therefore never refuses, and sorts, joins and aggregates are unbudgeted. `lctx query` (P1.11) uses it directly.
- **Consequence.** A join or sort over a large generation in `lctx query`, or later in P5, grows until the process is killed instead of failing with "Resources exhausted". The bounded chunks limit only the per-batch transfer.
- **Correction.** `InspectionSession::new` takes a memory limit (or `RuntimeOptions`) and builds its context with a bounded pool, as `AttemptRuntime` does. `query` passes the serving configuration's budget.
- **Closure evidence.** An inspection query over the 65 MiB artifact under a 1 MiB limit refuses with Resources exhausted, and the session stays Ready.

<a id="F06"></a>
### F06 · Low (P3 barrier, deferred with store-lifecycle F08) · The reader recomputes the frontier scope, and stage registration does not bind the input generation

- **Principles · judgment/gate:** FP-04, DP-21, DP-09, CI-01 · A2, A3 (scenarios 1 and 4).
- **Evidence.**
  - `held()` (`generation_read.rs:171-180`) recomputes `FrontierContract::facts(model, Profile::Catalog)`, as `ddl::scopes` does (`ddl.rs:26-33`). `LeaseContract` carries each frontier's digest and column signature but not its relation set, and `acquire` returns only the `Frontier` (`lease.rs:24-28, 37`). The lease terms that store-lifecycle F07 proposed included the relation set.
  - `StageSession::register` checks permit identity, schema and uniqueness (`model_runtime.rs:32-37`), but not which generation, or whether the attempt's own output, backs the permit.
- **Consequence.**
  - Scenario 4: a new or profile-dependent frontier edits the reader's copy as well as the store's (F08). A divergence fails loud (raw `42P01`) or closed (a spurious `Frontier`).
  - Scenario 1 (P3/P4): an attempt that reads published facts generation G records neither G nor its content digest in its schedule, producer digest or registry. Two normalized generations built from different facts generations have the same lineage (DP-21, CI-01), and any reuse keyed on the schedule would be incomplete (DP-09).
- **Correction.**
  - Now, and cheap: `LeaseContract` carries each frontier's relation set from `ddl::Scope`, and `acquire` returns it; the session's held set is the lease's.
  - At the P3 frontier design: an attempt declares its external input generations (id and content digest) in the schedule or registry, and `register` accepts a `GenerationTable` only from a declared input generation. Decide this together with F08's "does a P3 generation contain its facts or reference a published facts generation?"
- **Closure evidence.** No scope computation in `cpg-core`, and a traced P3 input whose generation id reaches the produced generation's registry record. **Deferred**; trigger: the P3 frontier design (as F08).

<a id="F07"></a>
### F07 · Low · Family availability is not exposed at the reader, so NotRequested reads as empty

- **Principles · judgment/gate:** CI-04, DP-02 · CI-G1 (latent). This finding also carries [store-lifecycle R2](design_review_p1-store-lifecycle_2026-09-29.md#re-inspection).
- **Evidence.**
  - `GenerationSession` exposes `frontier`, `generation`, `health` and `table` (`generation_read.rs:125-139`), but no availability.
  - A catalog-profile facts generation holds the Flow relations inside its frontier (`ddl.rs:29-31`): empty, and marked NotRequested only in `ProviderCoverage` and `admission_families`.
  - The third bullet of the store-lifecycle F05 correction (availability and family on the lease) was not implemented.
- **Consequence.** A P4 analysis stage reading `flow_*` from a catalog generation, or a P5 tool, sees zero rows, which is the "unknown served as absent" shape. `lctx query` returns `0` for `SELECT count(*) FROM flow_uses`. That is acceptable on a raw SQL surface only if the operator knows to join coverage.
- **Correction.**
  - At `open`, read the generation's `admission_families` inside the lease transaction; the serving role can already select the control tables. Expose `availability(FactFamily)` and each relation's family.
  - `table::<R>()` either refuses a NotRequested or Unavailable family unless the caller acknowledges it, or returns the availability with the table. `InspectionSession` can register such relations as refusing, as it does outside the frontier, or annotate them.
- **Closure evidence.** On a catalog facts generation, `availability(Flow) == NotRequested`, and a typed consumer cannot obtain `flow_uses` without handling it. Trigger: before the first P3/P4 stage or P5 tool that reads a family-scoped relation.

**Foundation and rule verdicts.**

| Principle | Verdict | Evidence |
|---|---|---|
| FP-01, DP-17 | satisfied | The lease is in the store, the session in `cpg-core`, transport in the fork; each boundary hides its mechanism |
| FP-02 | satisfied, except F03 | `LeaseDriver` and `SessionBinder` are narrow and have real second implementations; scan errors expose fork types (F03) |
| FP-03 | satisfied for P2 and inspection; unresolved for P3 inputs (F06) | `register` composes; input lineage is not bound |
| FP-04, DP-01 | satisfied, with latent F03 and F06 | One lease definition; classifier and scope copies |
| FP-05, DP-19 | satisfied, except F04 | Terminal loss is explicit; `close` is not |
| FP-06 | satisfied | Pushdown admission is a pure function of `Expr` and schema; session tests need PostgreSQL by nature |
| DP-20 | satisfied for stage sessions; violated for inspection (F05) | Reservation from the attempt pool |
| DP-23 | violated (F01, F02) | Non-discriminating known answers |
| CI-04 | satisfied for the frontier; F07 for NotRequested | — |
| CI-13 | satisfied | One generation per session, never reconnected |

**Observations** (no finding).

- **O1.** With N ≤ 2 (`roles.rs:55-56`), a plan that needs more concurrent generation scans than N waits in `connect_direct`. Examples are a three-way streaming join and a correlated self-join. It fails with an acquire timeout, not a typed capacity error. Revisit at the first multi-relation P3/P4 stage query (T13 defers sizing to measurement).
- **O2.** bb8's `retry_connection` defaults to true (bb8 0.9.1 `api.rs:244`). A refused bind is retried, and the `remaining` counter turns the retry into `Lost` without opening a new connection (`pool.rs:350-353`). Setting `.retry_connection(false)` in `new_bound` would state "never reconnects" directly.
- **O3.** `column_signature` restates the lowering's scalar-to-type map (`ddl.rs:45-48` against `:123-127`). Both are exhaustive, so drift fails closed; deriving both from one function would remove the copy.
- **O4.** The live-column check is at acquisition only. Leases do not block owner DDL, and constraints, collations, RLS and triggers are `store check`'s concern. This is consistent with store-lifecycle O3.
- **O5.** The drain's CancelRequest can land on the confirming `simple_query("")`, which then fails and loses the pool spuriously. Safe; a rare cost to liveness.
- **O6.** Scenario 1's P2 half: in-attempt inputs travel as D0 handoffs or retained batches. There is no store-backed re-read of the attempt's own staging relations: the writer has INSERT only, and leases require `published`. A P2 stage input larger than the retention budget has no route. This is an A0 design question (trigger: A0), not a P1.10 defect.
- **O7.** Closure effects:
  - C02 closes: another model is refused at the installation digest and tampered columns at the live check, each typed `Contract` before any scan (Tested, fresh run).
  - C01's lease half closes: a lease blocks retire, the race resolves in either order, and a retired id is `Absent` (Tested, fresh run).
  - P0 exit F02 at the provider table closes (`table::<TransferKey>()` → `Frontier`, `tests/generation_read.rs:150`, executed before the failing line). Its `InspectionSession` evidence waits on F01.

## 8. Library fit and total complexity

| Capability and owner | Consumer | Candidates | Pinned fit and gaps | Burden | Choice |
|---|---|---|---|---|---|
| Table and pushdown (`generation_read`) | Stages, inspection | DataFusion `TableProvider` + `Unparser` (55.1); the fork's generic `SqlTable`/federation (removed, ADR-0090) | The generic path would push text ordering (collation) and unparse binary literals as bit strings, both wrong answers. The closed admission layer (about 60 lines) prevents both | Small | Keep; add F02's controls |
| Bound pool (fork) | Sessions | bb8 with `SessionBinder` and a `remaining` counter; SQLx pool; deadpool | Arrow decoding in the fork is tokio-postgres-based, so an SQLx pool cannot feed it. bb8 fits once replenishment is neutralized by the counter (O2) | About 200 lines in the fork | Keep |
| Cancellation | Scans | tokio-postgres `CancelToken` + a confirming round trip | Fits; the protocol has no acknowledgement, so the round trip is required | Small | Keep; F04 item 2 |
| Memory | Scans | DataFusion `MemoryReservation` | Fits for stage sessions; inspection lacks a bounded pool (F05) | — | Keep |
| Lease protocol | `pin`, sessions | Rust `LeaseContract` over a two-method driver; a server-side PL/pgSQL `lease()` in the control template | See §9 | About 100 lines | Keep |

## 9. Alternatives and tradeoffs

| Alternative | Change propagation | Authority and composition | Test / substitution | Machinery and risk | Decision |
|---|---|---|---|---|---|
| Current: Rust lease over `LeaseDriver`; one session per generation with N leased connections | A lease term changes in one place | One definition, two drivers | Both drivers exercised on real PG | A text-row protocol (string-joined registry row) | **Keep** |
| A server-side `lctx_model_store.lease(g, …)` function rendered in `control.sql` | Protocol changes land in control DDL, so a store reset follows | Covered by the physical digest and `store check`; one statement for any driver | Typed errors need custom SQLSTATEs | Logic in PL/pgSQL | Not now; revisit if a third driver appears (for example the P5 native executor) |
| A lease per scan, not per session | Every scan re-validates | Same | Simpler `close` | A round trip per scan; reconnect semantics blur T13 | Rejected (T13) |
| Unrestricted unparser pushdown | — | Collation and literal semantics leak | — | Silent wrong answers | Rejected (§8) |

## 10. Verification and uncertainty

| Claim or scenario | Label / date | Command or inspection | Outcome |
|---|---|---|---|
| P1.10 provider suite at HEAD | Tested, 2026-09-29, `244eda4`, plus the then-uncommitted P1.12 owner-pool edit in `lctx-postgres/src/lib.rs` (not on these tests' paths) | `cargo test --release -p cpg-core --test generation_read` | **failed**: 8 passed, 1 failed (`pin_survives_selection_change_and_frontier_is_enforced` at `:152`, `inspection.sql(...).is_err()` false). Reproduced by a single-test run of the same command filtered to that test |
| Lease vs retire, select vs retire, lease blocks retire, atomic retire | Tested, 2026-09-29, same tree | `cargo test --release -p lctx-postgres --test installation --test lifecycle` | **passed** (9, 12) |
| Catalog reader counts, stage path, generations | Tested, 2026-09-29, `51aae78` | `cargo test --release -p lctx-postgres --test generation_stages --test generation_catalog --test generations` | **passed** (1, 2, 2) |
| `testing` absent from the CLI build | Interface-checked, 2026-09-29 | `cargo tree -p lctx -e features -i lctx-postgres --edges normal` | `default` only |
| Suite at P1.10's own revision | Historical author receipt (plan §4.2, 2026-09-29) | As recorded | generation_read 9 passed at `1ffe05c` and `5beafd8`; not re-run here. The `:152` pass is explained by the unknown-table error (F01) |
| Fork units (`bounded_chunks`, declared lists, bound limits) | Historical author receipt (P1.9) | `cargo test -p datafusion-table-providers-postgres --no-default-features --lib` | 64 passed; **not_run** here |
| Pushdown fidelity, drain protocol, lock order | Implemented; code read 2026-09-29 | Cited lines; DataFusion 55.1 and bb8 0.9.1 sources | As assessed in §3–§4 |
| `just fmt`, `just test-all`, facts pilots | — | — | **not_run** (binding acceptance timing) |

F01's corrected controls must state their answers first and must each fail against a wrong twin. F02's collation twin must run on the pinned image's default database collation.

## 11. Authority changes and dispositions

Proposed plan §8 rows. The plan is the single disposition owner.

| Required change | Route and owner | Source | Proposed disposition | Closure evidence or revisit trigger |
|---|---|---|---|---|
| Discriminating reader controls; typed refusal after `collect`; lease-after-drain assertion; receipt sentence | `cpg-core` test; plan §4.2 | F01 | open → **now** (required) | F01 closure |
| Pin `standard_conforming_strings`; executed bytea, NULL and collation controls | `generation_read` | F02 | open → with F01 | F02 closure |
| One SQLSTATE classifier; class and detail across `StageSink`; typed scan errors | `generations::failure`, `generation_read` | F03, store-lifecycle R1 | open → before A0's T9 failure handling | F03 closure |
| `close` closes the pool; drain guard before `query_raw` | Fork `pool`/`conn`; `generation_read` | F04 | open → before P3/P4 stage sessions | F04 closure |
| Bounded memory for `InspectionSession` | `generation_read`, `lctx query` | F05 | open → now (P1.11 consumer live) | F05 closure |
| Relation set in the lease terms; declared input generations for stages | `lease`, `model_runtime`; P3 design | F06, store-lifecycle F07/F08 | relation set: open → with F03. Input lineage: **deferred**; trigger: P3 frontier design | Traced P3 input |
| Reader-side availability | `generation_read` (lease transaction) | F07, store-lifecycle R2 | open → before the first P3/P4/P5 reader of a family-scoped relation | F07 closure |
| Core C01, lease half | Plan §8 core table | C01 | **closed** (O7) | This review, §10 |
| Core C02 | Plan §8 core table | C02 | **closed** (O7) | `digest_and_column_mismatch_rejected_before_scan`, fresh run |
| P0 exit F02, reader half | Plan §8 | F02 (P0 exit) | provider tables **closed**; the `InspectionSession` control → F01; `query` → P1.11 `store_cli` (historical) | F01 |

No exception records are requested.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | **satisfied** for scenarios 2–5 (bounded Low items) | One lease owner for two drivers. The fork absorbs transport and cancellation, and the stage pool is the attempt budget. Error classes leak fork types (F03), and inspection memory is unbounded (F05) | F03, F05 |
| A2 Encode meaning structurally | **satisfied**, with latent copies | The lease terms, closed algebra and terminal loss are explicit. The frontier scope is recomputed at the reader (F06). Availability (F07) and `close` (F04) are not represented | F04, F06, F07 |
| A3 Extend through composition | **satisfied** for published-generation reads in stage and inspection sessions; **unresolved** for P3/P4 cross-generation inputs (scoped out) | `register` composes, but input-generation lineage is unbound (F06) | F06, at the P3 frontier design |

**Bounded change decision: Revise.**
- The reader's architecture is sound and needs no structural change:
  - the driver-neutral lease with a fixed lock order;
  - digests and live columns checked before any scan;
  - one-shot bound connections, drain-and-return, and terminal `Lost`;
  - a closed, faithful pushdown algebra;
  - reservation from the attempt pool.
- G7 is unresolved: the named controls behind review focus #4 and the P0 exit F02 reader closure do not discriminate, and one fails at HEAD (F01). F01 is required, and cheap. F02–F05 and F07 are Low and routed; F06 is deferred with the P3 frontier design.
- Core C02 and C01's lease half close on this review's evidence. The plan may record them now, independently of F01.

**Enclosing architecture: unresolved.** Still open:
- P3/P4 cross-generation input lineage and frontier ownership (F06, store-lifecycle F08);
- the P2 driver's in-attempt input route beyond the retention budget (O6);
- P5 serving over sessions;
- the reader's availability disclosure (F07).

The assembled P0–P2 review is the next architectural checkpoint. Review acceptance is not release qualification.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Discriminating reader controls (`cpg-core` test) | F01 | Corrected suite green at HEAD, wrong twins red |
| 2 | Pushdown GUC pin and executed controls (`generation_read`) | F02 | Controls |
| 3 | Bounded inspection memory (`generation_read`, `lctx query`) | F05 | Resources exhausted under a small limit |
| 4 | One SQLSTATE classifier; typed scan errors (`generations::failure`) | F03, R1 | Classes on both drivers and across the sink |
| 5 | `close` closes the pool; early drain guard (fork, `generation_read`) | F04 | Retained table refused after `close` |
| — | Availability at the reader; frontier scope in the lease; input lineage | F07, F06 | First family-scoped P3–P5 reader; P3 frontier design |

**Next step:** the `cpg-core` read owner corrects F01 (with F02's controls in the same test pass), then re-runs `generation_read` at HEAD. A0 proceeds, with F03 scheduled before T9's failure handling.
