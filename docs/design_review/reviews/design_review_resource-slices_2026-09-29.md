# Resource slices R1–R3: reserved batches, charged validator state, Ruff work bound — bounded review

## 1. Scope, outcome and coverage

**Change / conformance; Revise (bounded), 2026-09-29.** Fresh-context `design-reviewer` review of
the three resource slices that the [cutover plan §4.1.1](../../plans/semantic-model-cutover-plan_2026-09-29.md#411-detailed-remaining-execution-order)
schedules for a joint review after R3. Compressed template slots 1, 2, 5 (the four requested
scenarios), 6, 7, 8, 10–12, under
[core 3.0, code-intelligence profile 1.1 and the binding](../design_principles/standard.toml).
Authority: [ADR-0085](../../adr/0085-typed-semantic-domain.md),
[ADR-0086](../../adr/0086-immutable-postgresql-generations.md),
[ADR-0089](../../adr/0089-stage-contributions-and-input-closure.md),
[DESIGN §15.11](../../design/sections/semantic-model.md#section-15-11) and plan §3.3 ("one attempt
memory pool"). Finding under test:
[input-validation F02](design_review_semantic-input-validation_2026-09-29.md) (coordinated memory
accounting and a total work bound), currently "narrowed → P1.9, E1" in plan §8.

| Field | Value |
|---|---|
| Subject | `a35c1df` (R1), `3a9d7eb` (R2), `5fef886` (R3) on `main`. Source and line numbers are cited at HEAD `5fef886`. During the review a parallel session left uncommitted D1 edits in the working tree (`Record::family`, `Stage.provider`, untracked `admission.rs`). They do not touch `batching.rs`, `charged.rs`, `resources.rs`, `types.rs`, `typed_syntax.rs` or `lctx-postgres/src/generations`, which equal HEAD. R1: [`record.rs`](../../../crates/lctx-model/src/domain/record.rs) (`HeapSize`, `Batch::new/read`), [`batching.rs`](../../../crates/lctx-model/src/domain/batching.rs), [`stages.rs`](../../../crates/lctx-model/src/domain/stages.rs) (`StageSink`, `StageOutput`), [`resources.rs`](../../../crates/lctx-model/src/domain/resources.rs), the `HeapSize` derive in [`lctx-model-macros`](../../../crates/lctx-model-macros/src/lib.rs), [`generations/mod.rs`](https://github.com/paul-heyse/library-context/blob/5fef8866b48260e27a5ff71c2c6e6265e6a46778/crates/lctx-postgres/src/generations/mod.rs). R2: [`charged.rs`](../../../crates/lctx-model/src/domain/charged.rs), `Invariant.create` ([`model.rs`](../../../crates/lctx-model/src/domain/model.rs):223), all 31 `InvariantCheck` implementations, `GenerationStore::validate`. R3: [`typed_syntax.rs`](../../../crates/cpg-extract/src/typed_syntax.rs), [`typed_limits.rs`](../../../crates/cpg-extract/tests/typed_limits.rs) |
| Adjacent code read | [`cpg-core/src/model_runtime.rs`](https://github.com/paul-heyse/library-context/blob/5fef8866b48260e27a5ff71c2c6e6265e6a46778/crates/cpg-core/src/model_runtime.rs) (DataFusion `ComputePool`), [`memory.rs`](../../../crates/lctx-model/src/domain/memory.rs) (D0 `MemoryGeneration`), [`generations/codec.rs`](https://github.com/paul-heyse/library-context/blob/5fef8866b48260e27a5ff71c2c6e6265e6a46778/crates/lctx-postgres/src/generations/codec.rs), [`ddl.rs`](https://github.com/paul-heyse/library-context/blob/5fef8866b48260e27a5ff71c2c6e6265e6a46778/crates/lctx-postgres/src/generations/ddl.rs), [`typed_conformance.rs`](../../../crates/cpg-extract/tests/typed_conformance.rs), [`capture.rs`](../../../crates/cpg-extract/src/capture.rs); pinned library sources: ruff_python_ast 0.0.11 `source_order.rs`, arrow-buffer/arrow-array 59.3 `pool`, datafusion-execution 55.1 `runtime_env.rs`, sqlx-core 0.9.0 `net/socket/buffered.rs`, std `btree/node.rs` (nightly-2026-09-29) |
| Tier · purpose | Change · conformance (binding: bounded slice). Depth raised because the slices add one cross-crate accounting mechanism. **Does not certify the enclosing P0–P2 resource architecture** |
| Supported scope | Admission and release of typed batch memory, transfer-bounded writing, stage output streaming, validator state, PostgreSQL COPY/read buffers taking the attempt budget, per-artifact syntax traversal bounds. Fact family touched: syntax occurrences and identifier observations, with coverage reason `ResourceRefused`. **Excluded:** P1.9 fork, P1.10 provider session, A0 provider framework, E1 envelope (all Proposed), process RSS |
| Scenarios | (1) P2 producers emitting through `StageOutput`/`BatchWriter` from a provider thread over a bounded channel (A0). (2) P1.9/P1.10 DataFusion reads in byte-bounded chunks drawing on the same budget. (3) The fastmcp facts pilot (E1): large modules, budget exhausted mid-attempt. (4) A later phase adds an invariant |
| Method | Source read at HEAD; library interfaces read at the pinned versions. Reviewer ran two focused suites (compiled against the working tree described above) and one throwaway allocator probe (§10). PostgreSQL suites: historical author receipts |

**What holds.**
- Every reservation is RAII: `Batch`, `BatchWriter`, `StateCharge`, COPY/read locals and the
  DataFusion `MemoryReservation` all release on drop, error, unwind and future cancellation.
  `domain_resources` shows the budget returning to 0.
- Admission precedes allocation for:
  - pending writer rows;
  - encoding (the 2× variable allowance for growing builders);
  - decode (`Batch::read` reserves before `R::decode`);
  - retained validator entries;
  - COPY wire buffers.
- An unbound `StateCharge` fails closed.
- `HeapSize` is generated from the same declarations as the codecs. The transfer and row limits
  come from one module (`resources.rs`). The generated table `CHECK` uses the same
  `MAX_ROW_BYTES`, so stored rows cannot exceed the read ceiling.
- The attempt budget can be DataFusion's own pool through a thin adapter (`ComputePool`), so plan
  §3.3's "one attempt memory pool" has a real implementation route.
- R3 overrides all 33 dispatch methods of the pinned `SourceOrderVisitor`. This is complete
  against ruff_python_ast 0.0.11. After a halt, residual work is one immediate return per
  remaining child slot.
- Every retained validator structure goes through charged containers. The scan of all 31 checks
  found only model metadata and SequenceCheck's in-progress member list, which is bounded at 4096,
  outside them.

## 2. Responsibilities and accounting ownership

| Component | Responsibility | Consumer contract | Direction |
|---|---|---|---|
| `lctx-model::domain::resources` | Library-neutral fallible pool (`ResourcePool`, `Reservation`), `FixedPool`, limit constants | `ResourceBudget::reserve/from_pool`; `ModelError::Resource` | No dependency on DataFusion/SQLx |
| `lctx-model::domain::{record, batching}` | Typed batch admission (rows + encoding), transfer batching, cross-flush dedup | `Batch::new/read(.., &budget)`, `BatchWriter::push/finish` | Model only |
| `lctx-model::domain::stages` | Stage output streaming to a `StageSink`, handoffs, contributions | `StageOutput::declare/push/contribute/finish` | Model only; sinks implement the trait |
| `lctx-model::domain::charged` | Validator/index state admission | `StateCharge`, `ChargedMap/Set/Vec` | Model only |
| `lctx-postgres::generations` | COPY and read buffers, validation runner | `copy/pin/validate(.., budget)` | Depends on model; charges the caller's budget |
| `cpg-core::model_runtime` | DataFusion pool as the attempt budget | `AttemptRuntime::budget()` | DataFusion stays in cpg-core |
| `cpg-extract::typed_syntax` | Per-artifact traversal bounds, typed refusal | `admit`, `emit` → `SyntaxWork` / `SyntaxError::Refused` | Provider-local |

This is a coherent split. Mechanism-specific accounting (SQLx buffers, DataFusion reservations,
Ruff traversal) stays with the owner of that mechanism, and the model owns the neutral contract.
One decision has no owner: which holder is charged for a shared Arrow buffer (F07).

## 5. Change scenarios

| Scenario | Owner and route | Observed / proposed propagation | Evidence |
|---|---|---|---|
| **1** P2 producers on a provider thread, bounded channel (A0) | A0 provider framework + `StageOutput` | `Batch<R>` is `Send` and carries its reservation, and `BatchWriter` is `Send`, so a batch crossing a channel stays charged. But `emit` and the other producers call a synchronous row callback, while `StageOutput` accepts rows only through `async push(&mut self)`. The plan leaves open whether rows or batches cross the channel. If rows cross, in-flight rows are uncharged. If batches cross, `StageOutput` has no batch entry that preserves `written`, handoff retention and one-writer dedup (**F05**). A writer error does not poison the stage, so a pump that treats `Resource` as backpressure can lose rows silently (**F01**) | Implemented contracts; A0 Proposed |
| **2** P1.9/P1.10 DataFusion reads on the same budget | `cpg-core::AttemptRuntime` (budget = `runtime.memory_pool`, `TrackConsumersPool<GreedyMemoryPool>` in DF 55.1); fork `MemoryReservation` hooks | This composes without a model change: fork reservations and model reservations land in one pool. The following remain open. `Batch::read` of a provider-held `RecordBatch` charges shared buffers a second time. The lease path overlaps `visit_physical`'s 3× reservation with the batch's (**F07**). Validation and lease reads go through SQLx, not the fork, so P1.9 does not cover them, and SQLx keeps grown buffers after the charge is released (**F06**). DataFusion `ResourcesExhausted` must surface as the same typed `Resource` refusal (P1.10 obligation) | Interface-checked (DF 55.1, adapter Implemented); P1.9/P1.10 Proposed |
| **3** fastmcp facts pilot (E1): large modules, exhaustion mid-attempt | E1 `SyntaxProvider`, stages, `validate` | Exhaustion inside `push`/`copy`/`validate` gives `ModelError::Resource`. Propagated, it drops `StageAccess`, fails the attempt, publishes nothing and returns every reservation (validation stays sealed; PG receipt). Gaps: `admit` never runs before Pyrefly parses, so provider parse/solver memory is outside the budget and absent from F02's residual list (**F04**). A refused artifact's streamed prefix has three incompatible documented outcomes (**F02**). TypeIndex keeps a cumulative 1M closure cap that a large typed generation can hit, reported as an invalid model (**F03**). B-tree inline charges understate ascending-order fill by 1.35–1.58× for record-valued maps (**F08**) | Implemented; Tested in part (§10); E1 not_run |
| **4** A later phase adds an invariant | The new check's `create(budget)` | **Observed three times after R2:** C1 `DeclarationCheck`, C4 `StabilityCheck` and C5 `CompositionCheck` each take `StateCharge::new(budget, name)` and charged containers. The runner, store and budget code are unchanged, and the new state is charged. A check that used a plain std collection would bypass the charge. Only the convention and review prevent it; the unbound default fails closed only when a charged container is used. No new lint is proposed (repository preference); X0 re-inspects | Implemented, traced in source |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | pass | Limits have one source (`resources.rs`). The DDL `CHECK` uses the same `MAX_ROW_BYTES`. `HeapSize` is generated from declarations | — |
| G2 Semantic fidelity | **unresolved** | F02: the refusal outcome is documented as `Unavailable`, as attempt abort and as coverage, while the prefix is already emitted. F03/F09: operational limits are reported as `Invalid`/`Codec` | Fix contract text and refusal typing |
| G3 Validity | pass | Admission precedes encoding, decoding and retention. Unbound state refuses. Oversized stored rows are refused by `CHECK` | — |
| G4 Hidden behavior | pass | Budgets are explicit parameters. Validation reads no ambient state | — |
| G5 Consistency and recovery | **unresolved** | Release on drop/error/cancel passes. F01: after a refusal the writer keeps ids for dropped rows and the stage stays finishable, a silent-loss path. DP-20 total-memory coordination is not established until E1/P1.10 (F04, F06) | F01 before X0; the rest via E1/P1.10 |
| G6 Transformation and reuse | pass | Cross-flush dedup compares the full content digest; a conflicting payload refuses. No caches | — |
| G7 Truthful capability claims | **fail** | F04: "oversized source is refused, never parsed" (`typed_syntax.rs:58`), the R3 commit and the plan §8 row ("admits source size before parsing") have no route in the Pyrefly path. F03: the R2 receipt (plan §4.2) says global closure totals were removed; `TypeIndex::closure_work` remains | Narrow the claims now; implement in E1 |
| G8 Library leverage | pass | DataFusion's pool is adopted behind a thin trait with a real non-DataFusion consumer (`FixedPool` in model/extractor). The Ruff visitor is used as designed. §8 records one missed built-in (SQLx `shrink_buffers`, F06) | F06 |
| CI-G1 Fidelity | **unresolved (latent)** | F02 can publish up to `nodes` occurrences under `Unavailable` coverage, or abort a whole attempt for one module, depending on which sentence the E1/A4 author follows. Nothing is published yet | F02 |
| CI-G2 / CI-G3 | n.a. / pass | No served claims. Limit tests use synthetic Python; no gold path is touched | — |

## 7. Findings

Severity: **Medium** means a contract gap with a concrete consequence in a planned scenario.
**Low** means cost or calibration. No finding publishes wrong facts on a current path. F01–F04
and F09 are small and belong before X0; F05–F08 route to their packages.

<a id="F01"></a>
**F01 · Medium · owner `lctx-model::domain::{batching, stages}`.** A writer error leaves the writer
and the stage usable while rows are lost.
- *Evidence.*
  - `BatchWriter::push` flushes first (batching.rs:50–53), then resizes `seen_held` (:58) and
    `held` (:65) with `?`. If either resize fails, the flushed `Batch` is dropped on return. If
    the flush itself fails (`with_reservation` refuses the encoding growth, :83), the taken
    pending rows are dropped.
  - In both cases their ids stay in `seen`, so a later push of the same rows returns `Ok(None)`
    as an "equal repeat" (:45–46), and `finish` emits only the remainder.
  - `StageOutput::push` and `contribute` return the writer error without failing the stage
    (stages.rs:318, :336). Only `emit` failures set `execution.failed` (:224). The producer can
    keep pushing and `finish(Complete)`.
- *Consequence.* Under the current `?`-propagating tests the attempt fails and nothing is lost.
  Planned P2 composition invites recovery, though: T9 partial disclosure, R3's per-artifact
  refusal pattern, and an A0 pump treating `Resource` as backpressure. A stage then completes
  with up to 4096 rows of other artifacts missing, under coverage that says they were emitted
  (DP-19, DP-12, CI-08).
- *Correction.* Make both fail-stop:
  - `BatchWriter` records a failed state, and later `push`/`finish` return an error. Or make
    `push` transactional: reserve before flushing, and on a failed flush restore or forget the
    rows' ids.
  - Any `StageOutput` writer error sets the execution failed, as `write` already does.
- *Closure.* A `domain_resources` control: the budget refuses right after a full batch, then
  `push`/`finish`/`StageOutput::finish` return errors, `execution.finish()` fails, and the
  reservation is 0.

<a id="F02"></a>
**F02 · Medium · owner `cpg-extract::typed_syntax`; producer E1/A4.** A refused artifact has three
incompatible documented outcomes, but its prefix is already streamed.
- *Evidence.*
  - The module doc says "any callback/refusal aborts the attempt" (typed_syntax.rs:6).
  - `SyntaxError::Refused` says "its coverage is `Unavailable(ResourceRefused)`" (:47).
  - `reason()` says "other errors abort the attempt" (:54), implying that a refusal does not.
  - `emit` pushes every accepted node through `output` before a limit halts, up to `nodes`
    (default 1M; `typed_conformance.rs:135` sees 2 events before the refusal).
- *Consequence.* Following :47 yields stored occurrences and observations for an artifact whose
  Syntax coverage says Unavailable, so present facts are labelled absent (CI-04). Following :6
  aborts the facts compile for one oversized module, contrary to T9. The prefix cannot be
  withdrawn once `StageOutput` has streamed it to COPY.
- *Correction.* Choose one contract. The emitted prefix is ancestor-closed (`enter_node` emits a
  parent before its children), so the natural choice is: a refused artifact publishes its prefix
  with `Partial` coverage, reason `ResourceRefused`, and the `SyntaxWork` in the diagnostic. The
  alternative is per-artifact staging with zero rows and `Unavailable`. Make the three doc
  sentences agree.
- *Closure.* An E1 control with a refused artifact: coverage and stored rows agree with the
  chosen contract, and other artifacts are unaffected.

<a id="F03"></a>
**F03 · Medium · owner `lctx-model::domain::types` (`TypeIndex`).** A cumulative global work cap
survived R2 and refuses valid generations at scale as an invalid model.
- *Evidence.*
  - `closure_work` is a field incremented on every `term_support` step and never reset
    (types.rs:182, :208). It is limited to 1,000,000, and the frontier check uses the same total
    (:224). The error is `Invalid("type closure work limit")`.
  - Every generated `SupportCheck` owns a `TypeIndex`, and every support with a `TypeTerm`
    subject re-traverses its closure.
  - The R2 commit and plan §4.2 say that global closure/lineage/proof totals were removed. The
    guard-lineage and derivation totals were removed; this one was not.
  - The per-call `pending`/`seen` scratch is uncharged and bounded only by this cap.
- *Consequence.* A11 type producers on fastmcp plus dependencies will create many supported type
  observations over shared terms, for example 200k supports × ~5-term closures. A valid facts
  generation then fails validation as an invalid model. The refusal is scale-dependent and
  misclassified (DP-20, DP-21, DP-12).
- *Correction.* Memoize verified closures per (term, provider, context, display-only) in a
  `ChargedSet`, so total work is linear in the term DAG, and charge the per-call scratch. Remove
  the cumulative cap. If a per-closure bound stays, it becomes a typed limit (F09).
- *Closure.* A support check whose cumulative closure steps exceed 1M over shared terms
  validates. `closure_work` is gone. Plan §4.2's R2 receipt is corrected.

<a id="F04"></a>
**F04 · Medium · owner plan §8 row and `typed_syntax` docs now; E1 `SyntaxProvider` for implementation.**
The pre-parse admission claim has no route. Provider-internal memory is outside the budget and
missing from F02's residual list.
- *Evidence.*
  - `admit` is documented as "before it is handed to the provider: an oversized source is
    refused, never parsed" (typed_syntax.rs:58).
  - Its only callers are `emit` itself (:73), which receives an already parsed `ModModule`, and
    the tests.
  - The production-shaped harness builds Pyrefly handles for every captured file and runs
    `txn.run(.., Require::Everything)` (typed_conformance.rs:95) before any `emit`. Pyrefly also
    loads imported modules from the search path by itself.
  - Plan §8 says "R3 admits source size before parsing". The plan's own R3 row correctly says
    "before traversal".
- *Consequence.* A multi-MiB generated module in the captured closure is parsed and type-checked
  regardless of `SyntaxLimits.source_bytes`. Ruff AST and Pyrefly solver state for all modules
  are live together and invisible to `ResourceBudget`. F02's residual list ("P1.9, E1") does not
  name this term, so E1 could report a budget high-water that says nothing about peak RSS
  (DP-22, DP-20, G7).
- *Correction.*
  - Now: restate the claim as "admitted before traversal". Add "provider-internal parse/solver
    memory" as an explicit E1 term in the plan §8 row.
  - In E1/A4, choose and implement one of:
    - (a) `admit` every captured artifact before building handles, and record refused ones as
      `ResourceRefused` coverage. Declare that transitive loads of a refused module stay
      provider-internal.
    - (b) An aggregate admission on analyzed source bytes per provider session, calibrated by
      E1's bytes → peak-RSS relation.
- *Closure.* Corrected wording. The E1 evidence folder reports provider-internal memory against
  admitted source bytes. A control shows an oversized captured module refused before handle
  construction.

<a id="F05"></a>
**F05 · Medium · owner A0 (provider framework) with `lctx-model::domain::stages`.** The unit that
crosses the provider-thread channel is not defined in accounting terms.
- *Evidence.*
  - `emit` and future providers produce rows through a synchronous callback. `StageOutput`
    exposes only `async push(&mut self, row)` and `contribute(row)`.
  - `emit` (stages.rs:357), which writes and retains, is private.
  - A0's row says "bounded channel to an async pump holding `StageAccess`" without the element
    type.
- *Consequence.* A row channel holds up to capacity × row size, and a row can be up to 64 MiB,
  of uncharged memory. A provider-side `BatchWriter` producing `Batch<R>` is charged, but it
  cannot be handed to `StageOutput` without bypassing `written`, handoff retention and one-writer
  dedup.
- *Correction.*
  - Carry `Batch<R>` built by one provider-side `BatchWriter` per relation.
  - Add `StageOutput::emit_batch`, which routes through `emit` and marks the output written.
  - Keep dedup in exactly that writer. Alternatively, bound a row channel by admitted bytes
    through the budget.
- *Closure.* A0 controls add: a full channel with an exhausted budget gives a typed refusal and
  no uncharged queue; batch entry hands off to readers; reservations return to 0.

<a id="F06"></a>
**F06 · Low · owner `lctx-postgres::generations` (writer and validation connections); P1.10 for leases.**
SQLx keeps grown connection buffers after the charge is released.
- *Evidence.*
  - COPY charges `size × 2` per row and releases it after `send` (mod.rs:295–300).
    `visit_physical` resets `held` per batch (:436).
  - SQLx 0.9 grows `WriteBuffer`/`ReadBuffer` to the largest message and shrinks only on
    explicit `Connection::shrink_buffers()` (sqlx-core `buffered.rs`:142, :241–255, :313–327).
    `lctx-postgres` never calls it.
  - Reads are admitted after SQLx has materialized the row. The generated `CHECK` bounds this
    to one row.
- *Consequence.* A pooled writer or owner (validation) connection that carried a large row keeps
  up to about `MAX_ROW_BYTES` per direction, uncharged, for its pooled lifetime. A lease
  connection keeps it for the lease's lifetime, because it closes on drop (DP-20). P1.9's fork
  does not cover these SQLx paths, and T13's one-shot provider connections avoid it only for
  provider sessions.
- *Correction.* Call `shrink_buffers()` (or close) after an operation whose largest row exceeded
  SQLx's default buffer, or hold a per-connection high-water reservation until then. State the
  one-row pre-admission overshoot as a declared allowance.
- *Closure.* Inspection of the COPY/validate/lease paths, plus a PG control with a 20 MiB row that
  returns the budget to 0 with buffers shrunk.

<a id="F07"></a>
**F07 · Low · owner `lctx-model::domain::record` (`Batch::read`); P1.10 `GenerationTable`.** A shared
Arrow buffer is charged by every holder.
- *Evidence.*
  - A canonical `Batch::read` shares the caller's buffers and charges
    `get_array_memory_size()` for them again (record.rs:220–224). `domain_resources` asserts
    this sum (`batches_hold_their_reservation_and_refuse_before_encoding`).
  - `get_array_memory_size` counts whole parent buffers for sliced batches.
  - The lease `visit` keeps `visit_physical`'s 3× reservation live while `Batch::read` reserves.
  - `MemoryGeneration::store` charges buffers that the producer's batch still holds.
- *Consequence.* The double count is conservative for a lone attempt. In scenario 2 the fork holds
  the chunk under a DataFusion `MemoryReservation` and `Batch::read` charges it again from the same
  pool, so read-heavy stages refuse at roughly half the configured budget.
- *Correction.* Declare charge ownership. A typed boundary that receives provider-reserved
  buffers takes over that reservation (for example `Batch::read_owned(RecordBatch, reservation)`)
  or charges only its typed rows. arrow-rs 59.3's `pool` feature (`RecordBatch::claim`)
  de-duplicates shared buffers, but its `MemoryPool::reserve` is infallible. It fits tracking
  and diagnostics, not admission.
- *Closure.* A P1.10 control: N chunks read through `GenerationTable` plus `Batch::read` keep
  pool usage ≤ one charge per buffer plus typed rows.

<a id="F08"></a>
**F08 · Low · owner `lctx-model::domain::charged`; calibration at E1.** The charged-container size
model understates actual allocation.
- *Evidence.*
  - `NODE_ALLOWANCE` is 32 bytes per entry (charged.rs:10). Validators insert in `id` order.
  - std's split rule keeps 6 of 11 slots in a node after an append at the right edge
    (`btree/node.rs`:940–948).
  - Reviewer probe (§10): with 1M ascending entries the actual/charged inline ratio is 0.99 for
    K+V = 32 (Id→Id), **1.35** for K+V = 80 and **1.58** for K+V = 176. Whole-record values
    such as `Occurrence`, `ProviderSymbol`, `TransferKey` and `ConditionNode` are in the upper
    range.
  - `ChargedMap::update` charges nested growth after `mutate` has allocated it (:66–73). For
    example, `TypeIndex.members` runs before the 4096-member sequence check because invariants
    run in name order, so a large member list doubles before its charge is refused.
- *Consequence.* The budget high-water understates validator memory by up to ~1.6× on the inline
  part, plus one nested doubling. This is a bounded ratio, never unbounded, but it widens the gap
  E1 must explain.
- *Correction.* Charge `2 × (size_of K + size_of V) + 16` per B-tree entry, which is conservative
  for ascending fill. Pre-charge the next capacity step for `Vec` values in `update`, as
  `ChargedVec::push` does. Alternatively, record E1's measured factor.
- *Closure.* E1 reports the validation budget high-water against RSS, or a counting-allocator
  control in `domain_resources` stays within the charge.

<a id="F09"></a>
**F09 · Low · owner `lctx-model::ModelError`, `lctx-postgres::generations::Error`.** Limit
refusals are reported as invalid input or codec failures.
- *Evidence.*
  - `BatchWriter` refuses a row above `max_row` as `ModelError::Invalid` (batching.rs:41).
  - A stored row above the limit gives `Error::Codec` (mod.rs:433), and so does the small-read
    cap (:402).
  - F03's closure cap gives `Invalid`. Only budget refusals are `Resource`, and only R3 has a
    typed limit (`SyntaxLimit`).
- *Consequence.* P2 producers cannot turn a transport-ceiling refusal into `ResourceRefused`
  coverage the way R3 does without matching on message text (DP-21, DP-02).
- *Correction.* Add one typed limit refusal (owner, limit, observed) beside `Resource`, and use
  it for these ceilings.
- *Closure.* Existing controls, including the 70 MiB row, match the typed variant.

**Observations (no action required now).**
- O1 (D0, adjacent): `MemoryGeneration::validate` runs `concat_batches` before charging it, and
  its per-input `order` copies and `RowConverter` rows are uncharged (memory.rs:46–63).
  Revisit trigger: any use of `MemoryGeneration` beyond fixture-sized inputs, such as the B3
  corpus at scale.
- O2: per-row COPY reservations and per-insert charges register/grow DataFusion consumers under
  `TrackConsumersPool`'s lock when the budget is `ComputePool`. E1 measures the cost;
  performance is not a design gate.
- O3: `DEFAULT_MEMORY_BYTES` is a fixed 64 GiB (the host has about 188 GiB). E1 should set the
  default from the measured envelope.
- O4: the plan's R1 row names `Record::encoded_bytes_hint`, which does not exist. The code uses
  private `fixed_width` plus `heap_bytes`.
- O5: every generated `SupportCheck` rescans and re-indexes the shared inputs. Peak memory is one
  check because checks run sequentially, but time grows with the number of support relations.
  This is an E1 timing item.

## 8. Library fit and total complexity

| Capability and owner | Candidates | Pinned fit and gaps | Choice |
|---|---|---|---|
| Attempt memory pool (`resources`, `cpg-core`) | DataFusion `MemoryPool`; bespoke `FixedPool` | DF 55.1 `with_memory_limit` gives `TrackConsumersPool<GreedyMemoryPool>`, fallible `try_resize`. The model cannot depend on DataFusion, and extractor/model tests need a pool without it | Adopted via a thin trait with two implementations: sound |
| Shared-buffer accounting (F07) | arrow-rs 59.3 `pool` feature (`RecordBatch::claim`) | Counts each `Bytes` once, but reservation is infallible: tracking only | Not adopted; revisit if F07's double count causes P1.10 refusals |
| Validator state | Charged `BTreeMap` wrappers; allocator statistics (jemalloc `stats.allocated`) | Allocator totals are accurate but cannot refuse before allocating or attribute to an owner | Charged containers are right; calibrate (F08) |
| Traversal bound | Ruff `TraversalSignal::Skip` only | Skip stops descent, not sibling dispatch; overriding every `visit_*` is the narrow route at 0.0.11 | Sound and complete |
| Driver buffers | SQLx `Connection::shrink_buffers` (0.9) | Exists, unused | Use it (F06) |
| Provider parse memory | Pyrefly config excludes; admission before handles | Transitive loads cannot be excluded without changing resolution | E1 decision (F04) |

No wrapper here exists only to be a wrapper. `ChargedMap` exposes reads through `Deref` and
restricts only growth.

## 10. Verification and uncertainty

| Claim | Label / date | Command or analysis | Outcome |
|---|---|---|---|
| Batch/writer/stage-output reservations, charged state, drop to 0 | Tested, 2026-09-29 (reviewer; working tree = HEAD plus the uncommitted D1 edits, resource files equal to HEAD) | `python3 scripts/build_environment.py -- cargo test --release -p lctx-model --test domain_resources` | **passed** (9 tests) |
| Ruff traversal bounds (statement loop, list residual, depth, 17 MiB refusal, callback cap) | Tested, 2026-09-29 (reviewer; same working tree) | `python3 scripts/build_environment.py -- cargo test --release -p cpg-extract --test typed_limits` | **passed** (5 tests) |
| PG COPY/validate budget refusal leaves the generation sealed; funded retry; budget 0 | Tested, historical (author, 2026-09-29; plan §4.2 R1/R2 receipts) | `cargo test --release -p lctx-postgres --test generation_stages --test generations` plus the domain PG suites | **not_run** by reviewer (Docker PG18) |
| `typed_conformance` on the Pyrefly path | Tested, historical (author, 2026-09-29) | `cargo test --release -p cpg-extract --test typed_conformance` | **not_run** by reviewer |
| B-tree inline charge vs actual (F08) | Measured, 2026-09-29 (reviewer throwaway probe, not retained) | Single-file `rustc -O --edition 2024` program with a counting `GlobalAlloc` over `System`, toolchain nightly-2026-09-29: 1,000,000 `u128` keys into std `BTreeMap` in ascending and xorshift order with 16/64/160-byte values; live bytes ÷ entries vs `K+V+32` | ascending 63.6/151.6/327.6 bytes vs 64/112/208 charged; shuffled 50.6/121.0/261.8 |
| F01 silent loss after refusal | Implemented; failure path untested | Source trace (batching.rs:50–66, stages.rs:318/336) | Proposed control in F01 |
| Budget high-water vs process RSS; provider-internal memory | Not established | E1 | **not_run** (E1 not implemented; acceptance timing) |

## What input-validation F02 still lacks

F02 is **not closable** and remains *narrowed*. R1–R3 establish admission and release for typed
batches, validator state, COPY/read buffers and per-artifact traversal. Still missing:

1. **Slice corrections** (before X0): F01 fail-stop writer and stage; F02 one refusal contract;
   F03 removal of the cumulative type-closure cap; F04 claim narrowed to "before traversal"; F09
   typed limit refusals.
2. **A0** (F05): a charged unit across the provider channel, and a batch entry on `StageOutput`.
3. **P1.9/P1.10**: the fork's byte-bounded chunks on the shared pool; shared-buffer charge
   ownership (F07); DataFusion refusals typed as `Resource`. SQLx paths outside the fork, which
   are validation, lease and writer connections, need buffer shrink or a high-water charge (F06).
   P1.9 as written covers only the fork.
4. **E1 measured envelope** (both profiles on fastmcp; at least one refusal):
   - Per-stage budget high-water against peak RSS.
   - Named unaccounted terms: provider-internal parse/solver memory (F04), allocator retention,
     SQLx buffers, container calibration (F08).
   - The largest captured module.
   - A run exhausted mid-attempt that shows a typed refusal, reservations at 0 and nothing
     published.

   A default budget should be derived from this envelope (O3).

The plan §8 row owns F02's disposition and should list these four routes instead of
"P1.9, E1". This review does not edit it.

## 11–12. Dispositions, judgments and decision

Disposition owner: [plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
Until those rows are added, this review holds F01–F09 as **open**.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 (before X0) | Fail-stop writer/stage (`batching`, `stages`) | F01 | Refusal-after-flush control |
| 1 (before X0) | One refused-artifact contract (`typed_syntax`) | F02 | Doc agreement; E1 control |
| 1 (before X0) | Memoized, charged type closure; no cumulative cap (`types`) | F03 | >1M cumulative steps validate |
| 1 (before X0) | Narrow the R3/R2 claims (plan §8/§4.2, `typed_syntax` doc) | F04, F03 | Corrected text |
| 2 | Typed limit refusal (`ModelError`, PG `Error`) | F09 | Controls match the variant |
| 3 (A0) | Batch-carrying channel and `StageOutput::emit_batch` | F05 | A0 controls |
| 3 (P1.10) | Shared-buffer ownership; SQLx buffer shrink or high-water | F07, F06 | P1.10 controls; inspection |
| 4 (E1) | Envelope, provider memory, container calibration | F04, F08, O2, O3 | Dated evidence folder |

| Judgment | Verdict | Scenario evidence and scope |
|---|---|---|
| A1 Localize change | **satisfied** | Accounting sits with the owner of each mechanism behind one neutral contract. S4 was observed three times without runner or store edits, and S2 needs no model change. S1's gap is one addition in `stages` (F05) |
| A2 Encode meaning structurally | **unresolved** | Size model, limits and reservation lifecycle are derived or single-sourced. The failure semantics are not encoded: refusal outcome (F02), writer state after refusal (F01), charge ownership of shared buffers (F07) |
| A3 Extend through composition | **satisfied (S2, S4); S1 routed** | The DataFusion pool composes as the attempt budget, and new invariants compose through charged containers. The A0 channel needs the F05 entry point, which is Proposed with a clear owner |

**Bounded change decision: Revise.** G7 fails on the pre-parse and removed-totals claims. G2, G5
and CI-G1 are unresolved on the refusal and failure contracts, and A2 is unresolved. The
corrections are local: F01–F04 and F09 are a few dozen lines plus text. No boundary moves. After
them, an Accept of R1–R3 at slice scope is expected.

**Enclosing architecture: unresolved.** The DP-20 total-memory envelope for P0–P2 depends on A0,
P1.9/P1.10 and E1. A passing slice cannot certify it. The X0 assembled review should re-examine
scenario 3 with the E1 evidence.

**Next step.** R-fix commit (F01–F04, F09; owners above), then the plan §8 F02 row updated to the
routes in "What input-validation F02 still lacks".
