# Semantic-model cutover, P1.5–P1.7 — PostgreSQL generation store lifecycle (bounded review)

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | `git diff a594e8a..6df6f41` for `crates/lctx-postgres` and `crates/lctx-model/src/domain/{mod,stages,admission}.rs`, `crates/cpg-extract/src/typed_stages.rs`. Commits `cd22ca6` (P1.5 service baseline, verified owner), `b00fb52` (P1.6 generated install, live-catalog `check`, `reset`), `6df6f41` (P1.7 attempt-owned lifecycle, failed state, facts admission, frontier-scoped lowering, typed failure classes). Line citations refer to revision `6df6f41`. **Dirty tree:** the working tree has uncommitted P1.8 work (`generations/catalog.rs`, two lines in `generations/mod.rs`). It is not reviewed, and is cited only as evidence of how a consumer is already taking up the P1.7 contract |
| Standard | [Core 3.0 and template](../design_principles/core/design-principles.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md) with [review additions](../design_principles/profiles/code-intelligence/review.md), [repository binding](../design_principles/binding/library-context.md) |
| Tier · purpose | **Change · conformance**, the binding's bounded review after P1.7. It is run at design depth because P1.5–P1.7 establish the store's lifecycle mechanism: six scenarios and all slots are used. Judged against [DESIGN §15.11](../../design/sections/semantic-model.md#section-15-11), [ADR-0086](../../adr/0086-immutable-postgresql-generations.md), [ADR-0087](../../adr/0087-clean-semantic-reconstruction.md), and cutover plan §3.2, §4.1.1 (P1.5–P1.7, T10–T12), §4.2 and §8 |
| Reviewer · date | Fresh-context `design-reviewer` subagent (not the author), 2026-09-29 |
| Maturity and outcome | Phase 1 of a hard layered cutover. The decision says whether the attempt-owned lifecycle, the service baseline, and the install/check/reset operations are the right contracts for P1.8–P1.13 and P2 to build on. It also says whether core C01 (lifecycle half) and P0 exit F02/F07 close |
| Supported scope | Service baseline and owner verification; generated control schema and frontier-scoped generation lowering; install, `check`, `reset`; the attempt typestates `begin` → `copy` → `seal` → `validate` (with admission) → `publish`, plus `fail`; failed and interrupted states; select, retire, abort; the reader lease; typed failure classes. CI concerns: CI-04 (outside the frontier ≠ empty), CI-13 (pinned reads). No facts are produced and no claims are served |
| Exclusions | P1.8 catalog, P1.9 fork, P1.10 provider sessions, P1.11 CLI, P1.12/P1.13 operations (each examined only as an adjacent consumer); P2 producers; integrated gates and pilots (binding acceptance timing) |
| Expected changes | The six requested scenarios (§5): P1.10 provider sessions; P2 `compile --through facts`; a P3/P4 frontier; `store check`/`reset` on a live store; a crash mid-attempt or mid-publication; the P1.13 transition and a model change requiring reset |
| Baseline | [Core review](design_review_cutover-core_2026-09-29.md) C01/C02; [P0 exit review](design_review_p0-exit_2026-09-29.md) F02/F07 (Accept scoped); [deployment review](design_review_semantic-deployment_2026-09-29.md) F01 (acknowledged lease release); [rollback review](design_review_semantic-generation-rollback_2026-09-29.md) F01 |
| Method and coverage | Read in full at `6df6f41`: `generations/{mod,lifecycle,receipts,ddl,install,verify}.rs` and `control.sql`; `lib.rs`, `roles.rs`, `testing.rs`; the baseline migration; `tests/{lifecycle,installation,services}.rs` and the P1.5–P1.7 diffs of `generations.rs`/`generation_stages.rs`. Also read: plan §3.2, §4.1.1, §4.2 (P1.5–P1.7 receipts), §8; §15.11; ADR-0086/0087; the prior findings; `admission.rs` (frontier contract); `operations.rs` (`runs`). Two transient reviewer probes were executed against the pinned PostgreSQL 18.6 image (§10). The PG suites were not rerun (§10). Not examined: the dormant serving/import modules |

This review is evidence, not a status register. Current disposition belongs in plan §8 (binding §4). §11 proposes the rows.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Coherent responsibility and hidden decisions | Consumer contract | Direction | Expected reason for change |
|---|---|---|---|---|
| `generations::ddl` | Pure lowering of (model, relation scope, generation, schema, control) into phase-grouped statements. Also the control template rendered from `STATES`, `FAILURE_CLASSES`, `ProviderOutcome::ALL`, `Profile::ALL` and `Frontier::ALL`; per-frontier scopes; the physical digest (`ddl.rs:9-78, 170-174`) | `Lowering::phase/through`, `scopes`, `control` | Depends on the model; SeaQuery | A relation, field, frontier or lifecycle phase |
| `generations::{mod,receipts}` | Registry, the five lifecycle steps under the lock order installation → selection → generation → relations, receipts, admission binding, select/retire/abort, the lease, COPY | `GenerationStore`, `GenerationLease` | SQLx, pgpq | P1.8 catalog, P1.10 lease protocol, a new frontier |
| `generations::lifecycle` | Attempt ownership: a lifecycle connection holding the session attempt lock; typestates; failure recording (`lifecycle.rs:1-180`) | `begin`/`begin_conformance` → `GenerationAttempt: StageSink` → `SealedAttempt` → `ValidatedAttempt` | Over `mod`/`receipts` | P2 driver needs |
| `generations::{install,verify}` | Install/confirm, reset inventory and drop, live-catalog `check` against rolled-back shadow installs | `install`, `reset_plan`, `reset`, `check` → `CheckReport` | Verified `OwnerPool` only | Provisioning contract, lowering |
| `lib.rs` `OwnerPool`, `MigrationStore`; `roles.rs` | Verified non-elevated owner; service baseline with legacy-history refusal; runtime role configs with session limits | `OwnerPool::verify`, `migrate`, `RoleConfig` | — | P1.11/P1.12 wiring |
| `lctx_model::domain::{admission, stages, ModelError::Infrastructure}` | The facts frontier contract, preflight and admission; provider-outcome codes; the infrastructure class | `FrontierContract`, `FactsAdmission`, `Infrastructure` | Pure | A family, frontier or outcome |

Compile-time direction is sound: `lctx-model` ← `lctx-postgres`, and the store takes only a verified `OwnerPool` (compile-fail doctest `mod.rs:100-105`). Runtime coordination uses four advisory keys: the installation key `(1279476824,0)`, the generation key (id bytes 0–7), the attempt key (id bytes 8–15), and the selection row lock.

| Concept | Semantic authority | Update boundary | Derived forms and consumers | Second copy? |
|---|---|---|---|---|
| Lifecycle states | `ddl::STATES` + `FAILED` → control CHECK | Physical digest | Steps, `check` shadows | **Two refusal semantics selected by `owned` (F01)** |
| What a facts generation holds | `facts_relations()` via `FrontierContract::facts` | Contract and physical digests | `ddl::scopes`, receipts, invariants, grants, lease refusal | The store recomputes its scope instead of taking it from the contract (F08) |
| Selectability | `select` (`mod.rs:173`) | — | P1.11, P5 | Policy held by the effect owner (F08) |
| Failure class | `FAILURE_CLASSES` (`ddl.rs:17-18`) | Physical digest | `failure()` string match (`lifecycle.rs:75-86`) | **Restated strings (F05)** |
| Family availability | `FactsAdmission::availability` (in memory) | Admission | Stored as Rust `Debug` text (`receipts.rs:146`); P1.8 parses it | **Untyped projection (F05)** |
| Attempt outcome | Registry state, `failures`, attempt lock | Lifecycle transaction | `interrupted()`; P1.8 (uncommitted, via `pg_locks`) | Retained `lctx_ops.attempts/events` keep an unlinked outcome vocabulary (F09) |

**Fact and fidelity (CI profile).** Not applicable to extraction: this slice stores no provider facts. It carries coverage and admission (availability per family) and enforces the frontier physically. See scenario 5 of the P0 exit review.

## 3. Contracts, constraints and testing boundaries

| Contract | Inputs/outputs | Preconditions and enforcement | Effects, lifecycle, failure | Isolated verification |
|---|---|---|---|---|
| `begin(writer, &mut Execution, &FrontierContract, budget)` | Schedule → `GenerationAttempt` | Model digest; preflight before any effect (`lifecycle.rs:17-23`); exclusive sink binding. The session attempt lock is taken before the registry row is visible (`:40-49`) | Registry row, staging schema and planned outputs in one transaction; a refused preflight leaves no row (tested) | `lifecycle.rs:185-210` |
| `GenerationAttempt::copy` | Permit + batch | Permit identity and model; state `staging`, frontier scope and schedule digest checked under the shared generation lock (`mod.rs:281-334`) | A writer COPY transaction. An error is returned to the stage with its class (`mod.rs:61-71`), not recorded | `lifecycle.rs:303-313` |
| `seal(receipt)` | Receipt → `SealedAttempt` | Receipt identity/model/schedule; written == planned (`lifecycle.rs:110-125`); waits out writes (exclusive generation lock, `LOCK TABLE`), then revokes the writer (`receipts.rs:15-41`) | Any refusal → `failed` (terminal, abort only) | `generation_stages`, `lifecycle` |
| `validate()` | Stored content → content digest + `FactsAdmission` | Every in-scope invariant and, for facts, `AdmissionCheck` over stored rows (`receipts.rs:45-103`) | Receipts and validator receipts; the admission is held in memory until publish | `lifecycle.rs:137-210, 236-260` |
| `publish()` | → published | Relation receipts = scope, validator set = scoped invariants, planned = stage receipts, admission = stored model/content/schedule/profile (`receipts.rs:106-157`) | Admission insert, reader grant and transition in one transaction; the generation is never selected here | `lifecycle.rs:263-283` (pre-effect faults, F06) |
| `select` / `retire` / `abort` | Pointer / drop | Select: published and facts. Retire: published, not selected, no lease, no live attempt. Abort: unpublished or failed (`mod.rs:166-220`) | Drop of schema plus control records in one transaction; retired is terminal by absence | `lifecycle.rs:335-408` |
| `pin` → `GenerationLease` | Published generation → typed batches | Installation compatibility, model and frontier-scoped physical digest, published state; frontier relation set (`mod.rs:262-276, 383-390`) | Session-shared generation lock held until acknowledged release | `lifecycle.rs:164-170, 335-357` |
| `check` | Live catalog vs shadows | Exclusive installation lock; shadow per (state, frontier); roles, database ACL, history, inventory (`verify.rs:58-130`) | Always rolls back | `installation.rs:85-195` |
| `reset(confirm)` | Drops the inventory, reinstalls | Exclusive installation lock; database-name confirmation; try-locks each generation and attempt key (`install.rs:65-88`) | **One transaction for every generation (F02)** | `installation.rs:197-267` |

**Absence states at this boundary.** Distinct:
- failed, with from-state and class;
- interrupted (owned, unpublished, attempt key free);
- not selectable (typed `Frontier`);
- outside the frontier (typed `Frontier` on lease and copy; `42P01` on raw SQL);
- a retire/abort of an already absent generation (`AlreadyAbsent`).

Collapsed:
- a pin or select of an absent generation returns the same `State` as an unpublished one (F04);
- manual conformance generations have no liveness state (F01);
- availability is text (F05).

## 4. Composition and execution

The attempt composes as `begin` → stage-bound `copy` through `StageSink` → `seal(receipt)` → `validate` → `publish` | `fail`. Every step after `begin` runs on the attempt's own lifecycle connection, which holds the attempt key for the attempt's whole life (`lifecycle.rs:37-50`). A dropped or crashed attempt closes that connection, which frees the key, so the generation lists as interrupted. The typestates make a late write by the owner unrepresentable (`seal` consumes `self`). Raw writer SQL is waited out by `LOCK TABLE` and then refused by the revoke (`receipts.rs:35-41`).

Observed lock order:

| Operation | Order |
|---|---|
| copy | installation(S) → generation(S) → relation |
| steps | installation(S) → generation(X) → relations |
| select | installation(S) → selection row → generation(S) |
| cleanup | installation(S) → selection row → try generation(X) ∧ try attempt |
| pin | installation(S) → generation(S, xact) → generation(S, session) |
| check / reset | installation(X) |

`open` takes the attempt key before the installation key (`lifecycle.rs:43-44`), which differs from T10's stated order. It is safe because the id is private until commit and every other acquirer uses `pg_try_*` (O1). No wait cycle exists.

Production limits are relevant here:

| Pool | Session options |
|---|---|
| Owner | Two connections (`lib.rs:218`); `statement_timeout` ≤ 300 s, `lock_timeout` ≤ 60 s, `idle_in_transaction_session_timeout` 30 s (`lib.rs:201-206`) |
| Runtime roles | The same limits plus `transaction_timeout` 30 s (`roles.rs:102-120`) |
| Test pools | Plain `PgPool::connect` with none of these (`testing.rs:48-49`) |

## 5. Change and failure scenarios

| Scenario and trigger | Owning component | Contract change | Expected vs observed | Hidden knowledge / test setup | Evidence |
|---|---|---|---|---|---|
| **1. P1.10 provider sessions** read a pinned generation through their own (fork-driver) connections | `lctx-postgres` lease protocol; `cpg-core::generation_read` | Expose the protocol so each provider connection can take the lease | Expected: provider connections reuse one lease definition. Observed: the lease is sound — installation(S) + digests + published + session-shared key + frontier set. It is written inline as SQLx statements in `pin` (`mod.rs:262-276`). The keys are private functions (`mod.rs:89-91`); the installation key literal appears five times. The uncommitted P1.8 already restates the key-to-`pg_locks` mapping. The reader role's `transaction_timeout` of 30 s ends any scan longer than 30 s, taking the lease with it (T13 `Lost`) | Restating is required until P1.10 extracts the protocol (F07) | Code read. **Unresolved → P1.10** (planned deliverable) |
| **2. P2 `compile --through facts`** runs a multi-stage facts attempt | `lifecycle`, `receipts`, `admission` | None | Expected: compose the existing contract. Observed: preflight before effects; the typestates; a Partial family publishes and records its availability. A required failure ends as `fail` → `store.abort(g)`: two transactions; after a crash between them the row stays `failed` (abort only); the stored class is `producer` (F04). No registry row survives a completed abort | Owned `abort(self)` would make T9 one step (F04) | `lifecycle.rs:137-210` (historical receipt). **Satisfied (contract)** |
| **3. P3/P4 frontier** (`normalized`/`analysis`) | Model `Frontier` + store | New scope, lowering, admission kind, selectability | Expected: a new frontier is a new semantic category, so several owners may change legitimately. Observed: the store dispatches on `Frontier::Facts` at five sites — `ddl::scopes`, `select`, the validate/publish arms, the `Lifecycle`/`ValidatedAttempt` types, and the facts-shaped `admissions` columns. It recomputes the facts scope from `FrontierContract::facts(model, Catalog)` rather than from the contract `begin` receives | Store holds frontier policy (F08) | Code read. **Unresolved → P3 design** |
| **4. `store check`/`reset` on a live store** with leases and attempts | `verify`, `install` | None | Expected: consistent report; destructive operations refused while in use. Observed: correct. `check` holds the installation key exclusively and shadows roll back; `reset` refuses leases and live attempts with `Busy`. But both *queue* for the exclusive key: a queued exclusive request blocks every new shared request (probe 2). A check issued during a long validation therefore stalls pins, copies and steps store-wide, and under production `lock_timeout` those fail as `refused`. `reset` waits and then refuses `Busy` on the attempt key anyway. The reset twin for "a lifecycle transaction in flight" does not model real transactions (F06) | Test pools have no `lock_timeout` | Probe 2; code read (F03) |
| **5. Crash** mid-attempt and mid-publication | `lifecycle` | None | Expected: interrupted, abort only; publication all or nothing. Observed: holds. A terminated lifecycle backend yields `Transport` and lists the generation interrupted; manual calls cannot advance it (`lifecycle.rs:286-302`). Publication is one transaction, so a crash leaves either published-with-admission or validated → interrupted. An unconfirmed commit is typed `Unconfirmed`. Caveats: manual conformance generations have no liveness (F01); a half-open TCP connection keeps the attempt key until server keepalive (O4) | — | Tested (terminated backend, historical); publication by construction |
| **6. P1.13 transition; model change needing reset** | `MigrationStore`, `install`, `reset` | Model digest | Expected: legacy history refused; another model refused; reset drops and reinstalls; old binaries refused. Observed: all hold (`lib.rs:346-360`; `install.rs:23-37`; `mod.rs:255-260`; `installation.rs:251-267`). **But reset drops every generation in one transaction**, and after a model change it is the only removal route, because the new binary's `lock_installation` refuses to retire old generations. Probe 1: about 2,500 locks per 110-table generation; six exceed PostgreSQL's default lock table ("out of shared memory"). The failure is atomic but opaque (F04) | Test resets cover at most two full generations | Probe 1. **Defeated beyond roughly five generations (F02)** |

**Code-intelligence journeys.** *A module full of unresolved references*, applied to the frontier:
- a facts generation physically lacks analysis relations;
- the lease refuses them with `Frontier`, and raw reader SQL gets `42P01`, never zero rows (`lifecycle.rs:145-170`).

A catalog-profile generation holds empty Flow relations inside the facts frontier (NotRequested). This is disclosed only through `ProviderCoverage` rows and the text availability; the lease offers no typed availability (F05).

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | pass | One lowering drives DDL, the digest and `check` shadows. Scopes derive from `facts_relations()`. Publication recomputes the content digest from receipts. Latent: `FAILURE_CLASSES` is restated by `failure()` strings (F05), and the ops attempt vocabulary is unlinked (F09, no current dual writer) | F05, F09 |
| G2 Semantic fidelity | pass | Failed, interrupted, not selectable and outside-frontier are distinct and typed. Diagnostics conflate causes (F04) and availability is text (F05); both Low | F04, F05 |
| G3 Validity | pass | Private typestates; the manual path refuses owned generations (`receipts.rs:20, 50, 110`; schedule check); CHECKs on state, from-state, class and digests; publish requires every receipt set | — |
| G4 Hidden behavior | pass | `check` shadows roll back and `reset_plan` is read-only. Minor: `interrupted()` briefly takes each candidate's attempt key, so a concurrent `abort` can see a spurious `Busy` (F07) | F07 |
| G5 Consistency and recovery | pass | Publication is one transaction (by construction). Retirement is atomic under a post-effect fault (Tested). Lease and select both race retire safely in either order (Tested). Interrupted attempts are never published; failed is terminal; unconfirmed commits are typed. F02 fails atomically and loudly; it is an operability defect, not an inconsistency | F02 |
| G6 Transformation and reuse | pass | Frontier-scoped lowering and physical digest per scope; shadows per (state, frontier); no caches | — |
| G7 Truthful capability claims | pass, with wording | Two receipt statements overstate their evidence: "a generation in a lifecycle transaction is refused" (plan §4.2 P1.6) and "both orderings occur" for the seal race (P1.7) (F06) | F06 |
| G8 Library leverage | pass | PostgreSQL advisory locks and transactions, SeaQuery and pgpq are used where they fit. The catalog diff is about 60 lines over `pg_get_*def` against a shadow of the same lowering (§8) | — |
| CI-G1 Fidelity | pass | Outside the frontier is never read as empty; NotRequested is distinct in admission and coverage | F05 (typed availability for readers) |
| CI-G2 Evidence closure | n.a. | No served claims | P5 |
| CI-G3 Evaluation integrity | n.a. | No evaluation inputs in scope | — |

## 7. Findings

Severity:
- **Medium:** a contract or architectural defect with a concrete consequence in a requested scenario.
- **Low:** diagnostics, evidence or an extension barrier for a later package.

<a id="F01"></a>
### F01 · Medium · `lctx-postgres::generations` — the public manual conformance path is a second lifecycle semantics

- **Principles · judgment/gate:** DP-19 (MUST: mutable runtime state belongs to one identified attempt; retries are defined), DP-16 (no parallel path kept as evidence), FP-04, FP-05 · A2, A3.
- **Evidence.**
  - `create_conformance`, `seal`, `validate`, `publish` and `copy` are unconditionally `pub` and advance an *unowned* generation (`mod.rs:134-165, 278-280`). The doc states: "Unlike an attempt, a refusal here rolls back to the prior state" (`:135-136`).
  - Every shared step carries `owned: bool` plus optional schedule, outcomes and admission (`receipts.rs:15-20, 45-50, 106-110`). The registry stores `owned` (`control.sql:21-22`), and `interrupted()` considers only owned rows (`mod.rs:244`).
  - T10 and plan §3.2 say "Failed attempts are never repaired in place" (plan lines 71, 198), and §15.11 says "retry creates a new generation" (`semantic-model.md:418`).
  - The R2 control "tiny budget refuses, funded retry validates the same stored contents" was moved onto the manual path to keep it passing (`generation_stages.rs:140-151`). That is retry-in-place on a sealed generation.
  - There is no non-test caller: every use is in the 13 `domain_*` suites and the store tests (`git grep create_conformance 6df6f41 -- 'crates/*/src'` is empty).
  - The uncommitted P1.8 catalog already adds a third liveness class, `Writer::Manual` (`catalog.rs:102`, working tree).
- **Consequence.**
  - What a refusal means depends on who created the generation. The same validator refusal is terminal `failed` for an attempt, but for a manual generation it rolls back to `sealed` and may be retried in place.
  - Scenario 5: a crashed manual run leaves a `staging` generation that no observer can classify as live or interrupted.
  - Scenario 4: `reset`/`abort` drop a live manual run between steps, because it has no attempt key. "Reset refuses a live attempt" therefore holds only for owned generations.
  - The two semantics now propagate into P1.8's contract and will reach P1.11's `generation list/show`.
  - Any production caller (P1.11, the P2 driver) can create generations outside attempt ownership.
  - Conformance generations are never selectable, so no published answer is affected. The damage is architectural and operational.
- **Correction** (owner `lctx-postgres::generations` plus the `testing` feature; mechanical across suites; a one-line §15.11 amendment).
  1. Make every generation attempt-owned. Add a `testing` helper that builds a one-stage fixture schedule over the relations a suite writes and runs it through `begin_conformance`. Port the domain suites and store tests to the typestates.
  2. Delete the public manual steps, `create_conformance`, the `owned` column and the `owned`/`Option` branches.
  3. Re-express R2's control as tiny budget → `failed(resource)` → abort → a new funded attempt with an equal content digest.
  - Acceptable alternative: keep a manual harness only behind `#[cfg(feature = "testing")]`, with attempt refusal semantics (refusal → `failed`), so production has one meaning.
- **Closure evidence.**
  - No `create_conformance` or manual step exists outside `testing`.
  - Lifecycle steps take no `owned` parameter.
  - The R2 control passes as a new-attempt retry.
  - P1.8 needs no `Manual` writer class.

<a id="F02"></a>
### F02 · Medium · `generations::install::reset` — reset drops every generation in one transaction; the default lock table caps it at about five generations

- **Principles · judgment/gate:** DP-20 (shared resources are budgeted), DP-19 · A1 (scenario 6); G5 pass (atomic, loud).
- **Evidence.**
  - `reset` drops each inventoried schema with `DROP SCHEMA … CASCADE` and reinstalls control, all in one transaction (`install.rs:67-86`). `DROP … CASCADE` takes an AccessExclusive lock on every dependent object: tables, TOAST, indexes, constraints, defaults and row types.
  - After a model change, the new binary cannot retire or abort old generations, because `lock_installation` refuses a different installation (`mod.rs:255-260`). Reset is then the only removal route.
  - **Probe 1** (reviewer, 2026-09-29, pinned `postgres:18.6-bookworm`, defaults `max_connections=100`, `max_locks_per_transaction=64`). Each schema had 110 lowering-like tables: bytea key, a generation default, 7 CHECKs, PK and one FK.
    - One `DROP SCHEMA … CASCADE` held 2,529 locks.
    - Five in one transaction held 12,633.
    - Six failed with `out of shared memory` (hint: `max_locks_per_transaction`).
  - The real lowering adds a CHECK per coded or Id/Digest field and an FK per reference. Conformance generations lower the whole model, and P2 adds relations. The footprint per generation is therefore likely larger than in the probe.
  - The reset tests cover at most two full generations (`installation.rs:220-248`).
- **Consequence.** Scenario 6: after P2's pilots (both profiles, a repeated behavioral run, conformance evidence), a model change on a store with about six or more generations cannot be reset on a default-configured server. Nothing changes, which is correct. But the error surfaces as `refused` / "PostgreSQL operation failed" (F04), the new binary cannot remove generations one by one, and the old binary may be gone. The operator has to discover `max_locks_per_transaction` alone. Single-generation retire (about 2,500 locks at 110 relations) fits today; `check`'s shadow installs (up to eight shapes in one transaction) were not measured.
- **Correction** (owner `install`; small).
  1. Run reset in phases. First, under the exclusive installation key, mark the installation `resetting` (or delete its row) so every `lock_installation` refuses with `Contract`.
  2. Drop one inventoried schema plus its control records per transaction. This is idempotent, so a rerun of `reset` continues.
  3. Finally, drop and reinstall control.
  - Alternatively, preflight the lock need from `pg_depend` and refuse with a typed `Limit` that names the setting. Also record the requirement in P1.12's provisioning and `store check`.
- **Closure evidence.** On a default-configured PG18 container, a reset over at least eight full-model generations completes. An injected interruption between generations leaves a store that a rerun of `reset` finishes, with `check` clean afterwards.

<a id="F03"></a>
### F03 · Low · `generations::{verify,install}` — `check` and `reset` queue for the exclusive installation key, stalling the whole store

- **Principles · judgment/gate:** DP-20, DP-21 · A1 (scenario 4).
- **Evidence.**
  - `check` and `reset` block in `pg_advisory_xact_lock(1279476824,0)` (`verify.rs:67`; `install.rs:10, 68`).
  - Every copy, lifecycle step, pin, create and cleanup takes the same key shared (`mod.rs:255-256`). `validate_step` holds it for the whole validation (`receipts.rs:47`); the E1 subset took 18.4 s of validation.
  - Production pools set `lock_timeout` ≤ 60 s (`lib.rs:205`, `roles.rs:108`). The test pools set none (`testing.rs:48-49`), so the lifecycle controls cannot observe this.
  - **Probe 2** (reviewer, 2026-09-29, PG 18.6). Session A holds the key shared; B requests it exclusively and queues; C requests it shared (compatible with A) with `lock_timeout=3s`. C was cancelled by lock timeout after 3.04 s, and B was granted only after A committed.
- **Assessment of the lock choice.** Taking the key exclusively rather than using REPEATABLE READ is correct. `pg_get_*def`, `format_type` and `pg_get_functiondef` read the latest catalog, not the transaction snapshot; this is the same reason `pg_dump` locks. A shared-key-plus-per-generation variant reintroduces create/cleanup races in the orphan inventory. The defect is only that the request *waits in the queue*.
- **Consequence.** An operator's `store check` during a P2 validation holds up new pins (P1.10/P5 readers), other attempts' copies and lifecycle steps for up to the check's own `lock_timeout` plus its run time.
  - Those operations fail with 55P03, recorded as class `refused`.
  - A healthy attempt whose next step times out is recorded `failed`, or `interrupted` if the failure record also times out.
  - `check` itself fails with an opaque database error rather than "busy".
  - `reset` waits and then refuses `Busy` on the attempt key anyway.
- **Correction.** Acquire the exclusive key with `pg_try_advisory_xact_lock`, using a short bounded retry of about a second, and return `Busy` or a "store busy" report. Keep exclusive semantics while the key is held. Build the lifecycle suites' pools with the production session options.
- **Closure evidence.** With a lifecycle transaction held open, `check` and `reset` return `Busy` promptly, while a concurrent pin and copy under a 1 s `lock_timeout` succeed.

<a id="F04"></a>
### F04 · Low · `generations` error and failure recording — persisted failures lose or conflate their cause

- **Principles · judgment/gate:** DP-21, DP-02 · G2 (diagnostics).
- **Evidence.**
  1. `Error::Database` displays only "PostgreSQL operation failed" (`mod.rs:23`). The stored detail of every database failure therefore has no SQLSTATE, constraint or relation (`lifecycle.rs:87-89`).
  2. `class()` maps every SQLSTATE that is not a transport error to `Refused` (`mod.rs:46-52`). That single class covers: a dangling FK found by validation (the test's answer is `("sealed","refused")`, `lifecycle.rs:249`), a privilege denial (42501), a lock or statement timeout (55P03/57014), and lock-table exhaustion (53200, F02).
  3. `fail(detail: &str)` stores class `producer` (`lifecycle.rs:127, 137`). A writer transport loss that the stage saw as `Transport` (`lifecycle.rs:311`) is therefore persisted as `producer`.
  4. `registered()` maps a missing row to `State` (`mod.rs:233`). A pin or select of a retired or unknown id looks like an unpublished one.
- **Consequence.** P1.8's `generation show`, P1.11's exit codes and P2's "required provider failure vs infrastructure" decision cannot tell invalid content from provisioning, contention or a lost writer without guessing. P0 exit F07 asked for exactly this distinction.
- **Correction.**
  - Record SQLSTATE and the constraint or table name. These are safe metadata; server text stays out.
  - Map class 23 to `invalid`, 53/54 to `limit`, 55P03/57014 to a contention class, and 42501 to `refused`.
  - Change `fail(self, cause)` to take the typed cause.
  - Add `Error::Absent` for a missing registry row.
  - Optionally add an owned `GenerationAttempt::abort(self)` that removes the generation on the lifecycle connection, which already holds the attempt key. T9's "required failure leaves no registry row" then becomes one step.
- **Closure evidence.**
  - The validation FK control stores `invalid` with its constraint.
  - The writer-transport control stores `transport`.
  - Pinning a retired id returns `Absent`.

<a id="F05"></a>
### F05 · Low · control schema vocabularies — availability is `Debug` text and failure classes are restated strings

- **Principles · judgment/gate:** DP-02, DP-01, CI-04 · A2.
- **Evidence.**
  - Publication stores availability as `format!("{family:?}={availability:?}")` joined by `;` in an unconstrained `text` column (`receipts.rs:146`; `control.sql:77`). The uncommitted P1.8 splits it back into string pairs (`catalog.rs:80-83`, working tree).
  - `FAILURE_CLASSES` (`ddl.rs:17-18`) and the strings returned by `failure()` (`lifecycle.rs:75-86`) are maintained independently.
  - A mismatch makes `fail_step` violate the CHECK. `Lifecycle::fail` discards that error (`lifecycle.rs:65`), so the generation silently lists as interrupted instead of failed.
  - By contrast, `stage_outcomes` stores typed codes with a rendered CHECK (`control.sql:62-67`).
- **Consequence.**
  - Review focus (2) — NotRequested, Unavailable, Partial and complete-empty distinct through to `generation show` — is met only as text. Consumers couple to Rust `Debug` formatting of model enums; the model digest bounds its lifetime, but not its parsing.
  - Readers (P1.10, P5) get no typed per-family availability. A catalog generation's empty Flow relations read through the lease with nothing beside them to disclose NotRequested.
- **Correction.**
  - Add `admission_families(generation_id, family smallint, availability smallint)` rows, with CHECKs rendered from the `FactFamily` codebook and an append-only `Availability` code, inserted in the publication transaction.
  - Add a `FailureClass` enum owning `name()` and `ALL`, which both render the CHECK and replace `failure()`'s strings.
  - Expose `availability(family)` and the relation's family on the lease for P1.10.
- **Closure evidence.**
  - P1.8 reads typed rows.
  - A class or availability outside the enums cannot be constructed.

<a id="F06"></a>
### F06 · Low · store tests and receipts — three negative twins do not discriminate the claimed protocol

- **Principles · judgment/gate:** DP-23, DP-22 · G7 (wording).
- **Evidence and gap.**
  1. `reset_refuses_live_lease_and_attempt` simulates "a lifecycle transaction in flight" with a writer-role transaction holding only the generation key (`installation.rs:209-212`). Every real lifecycle transaction first takes the installation key shared (`receipts.rs:17, 47, 107, 161`), so a real reset *waits* for it (F03) and then proceeds, or refuses on the attempt key. The P1.6 receipt, "a generation in a lifecycle transaction is refused before any change" (plan §4.2), describes a lock state that production never creates.
  2. All three `failed_publication_is_atomic` faults are caught by publish's pre-effect checks (`receipts.rs:114-141`), before the admission insert, grants and transition, and they run on conformance attempts (`lifecycle.rs:263-283`). Atomicity of grant + admission + transition, and the facts admission insert, are not exercised under a fault. Atomicity holds by construction (one transaction), but the named control does not show it. The retire control shows the discriminating form: a fault after the drop (`lifecycle.rs:391-408`).
  3. `late_write_vs_seal_race` prints its counts but does not assert that both orderings occurred (`lifecycle.rs:233`). The receipt says "both orderings occur, counted".
- **Correction.**
  - Give the reset twin a real in-flight transition (installation shared + generation exclusive) and assert the actual behavior.
  - Add a post-effect publication fault, for example a trigger raising on `admissions` INSERT or on the `published` event, on a *facts* attempt; assert that no grant, admission or state survives.
  - Assert both orderings in the seal race.
  - Correct the two receipt sentences.
- **Closure evidence.** Those controls, and the corrected receipts.

<a id="F07"></a>
### F07 · Low · lock protocol — keys are private, and the installation key and liveness are restated

- **Principles · judgment/gate:** DP-01, FP-02 · A1 (scenario 1).
- **Evidence.**
  - The installation key literal appears at `mod.rs:109, 256`, `install.rs:10, 60` and `verify.rs:67`.
  - The generation and attempt keys are private functions (`mod.rs:89-91`). Tests re-derive them (`installation.rs:40-44`), and the uncommitted P1.8 restates the `pg_locks` classid/objid mapping.
  - "Interrupted" is derived by try-locking (`mod.rs:241-254`). This briefly holds each candidate's attempt key, so a concurrent `abort` can see a spurious `Busy`. P1.8 derives the same state a second way.
- **Consequence.** P1.10's provider connections (another driver) must take the same installation and generation locks and run the same checks. P1.8, P1.10 and tests each restate keys and liveness; a key-derivation change silently desynchronizes them.
- **Correction** (with P1.10's planned driver-neutral lease protocol).
  - One `locks` module owning the keys, the acquire/try statements and a read-only `pg_locks` liveness probe, used by `interrupted()`, P1.8 and `check`.
  - A `LeaseTerms` value (schema, relation set, model/physical digests, key) that P1.10's `GenerationTable` consumes.
- **Closure evidence.** The literal appears once; P1.8 and P1.10 consume the module.

<a id="F08"></a>
### F08 · Low (P3 barrier) · `generations` — frontier policy is dispatched inside the store

- **Principles · judgment/gate:** FP-01, FP-03, DP-17 · A1, A3 (scenario 3).
- **Evidence.**
  - `ddl::scopes` hard-codes conformance and facts, and recomputes the facts scope from `FrontierContract::facts(model, Profile::Catalog)` (`ddl.rs:26-33`) instead of taking it from the contract passed to `begin` (`lifecycle.rs:17-23`).
  - `select` allows only facts (`mod.rs:173`).
  - The validate and publish arms pair `Frontier::Facts` with `FactsAdmission` (`receipts.rs:52-54, 142-154`).
  - `Lifecycle.preflight` and `ValidatedAttempt.admission` are facts-typed (`lifecycle.rs:57, 162`), and the `admissions` columns are facts-shaped (`control.sql:69-78`).
- **Consequence.** Adding a normalized or analysis frontier edits these five store sites as well as the model. Selectability, a serving policy, lives in the effect owner. If the frontier contract's relation set ever becomes profile- or frontier-parameterized, the store's scope diverges from it silently.
- **Correction.**
  - The model's frontier owns its relation set, `selectable()` and its admission kind; the store iterates `Frontier::ALL` and takes the scope from the contract.
  - Decide first whether a P3 generation contains its facts or references a published facts generation.
- **Closure evidence.** A traced P3 frontier addition that edits no store dispatch site. **Deferred**; trigger: P3 frontier design.

<a id="F09"></a>
### F09 · Low (deferred) · service baseline — the retained ops attempt record duplicates the generation outcome vocabulary

- **Principles · judgment/gate:** DP-01, DP-04 · G1 (latent).
- **Evidence.**
  - The baseline keeps `lctx_ops.attempts` keyed by `store_path`, a Delta-era concept, together with `events.kind ∈ {published, failed, interrupted, …}` (`202609300014_service_baseline.sql:26-46`).
  - `runs` derives an outcome from these events (`operations.rs:67-68`).
  - Nothing links them to generation ids, while the registry, `failures` and the attempt key now own attempt outcome.
- **Consequence.** If P2's compile also records a run there, one attempt's outcome is written twice with no reconciliation. For example, a crash after publish commits but before the ops event makes `runs` report "unfinished" for a published generation.
- **Correction.** Before Dc or P1.11 `runs`, choose one:
  - `lctx_ops.attempts` references `generation_id` and `runs` derives the outcome from the registry; or
  - ops attempts stay historical-only and new compiles never write them.
- **Closure evidence.** The decision, plus a `runs` control over a published and a failed generation. Trigger: Dc or P1.11 `runs`.

**Foundation and rule verdicts.**

| Principle | Verdict | Evidence |
|---|---|---|
| FP-01, DP-17 | satisfied, except F08 (deferred) | Lowering pure; effects in `lctx-postgres`; frontier policy in the store (F08) |
| FP-02 | satisfied | `StageSink` and typestates are narrow; the lease protocol is extracted at P1.10 (F07) |
| FP-03 | satisfied for P2 | Scenario 2 composes |
| FP-04, DP-01 | violated (bounded, F01); latent F05/F09 | Two refusal semantics |
| FP-05, DP-02, DP-03 | satisfied, with F04/F05 | States, classes and frontier explicit |
| FP-06 | satisfied | The lowering and admission are store-free; lifecycle tests need PG by nature |
| DP-19 | violated for manual generations (F01); satisfied for attempts | — |
| DP-20 | violated at scale (F02, F03) | Probes 1–2 |
| DP-21 | F04 | — |
| DP-23 | F06 | — |
| CI-04 | satisfied at store level | F05 for readers |
| CI-13 | satisfied for leases | P1.10 provider sessions open |

**Observations** (no finding).

- **O1.** `open` takes the attempt key before the installation key (`lifecycle.rs:43-44`), unlike T10's stated order. This is deadlock-free because the id is unpublished until commit and every other acquirer uses try-locks. State this in the lock-order doc (`receipts.rs:3-4`).
- **O2.** The production owner pool has two connections (`lib.rs:218`), and each live attempt holds one for its whole life. One compile per process fits. A second concurrent attempt, or an owner command in the same process, waits until `acquire_timeout`. Tests use default pools. Revisit at Dc/P1.11.
- **O3.** Leases are advisory. `lctx_serving` has SELECT on every published schema (`ddl.rs:164`), so a reader without a lease skips the digest check and can see the schema vanish at retire. This is acceptable while serving credentials are used only by lctx; P1.10/P1.11 must always pin.
- **O4.** Detecting an interrupted attempt depends on the server noticing a dead client. A half-open TCP connection keeps the attempt key until server keepalive, so abort sees `Busy`. Consider `tcp_keepalives_*` in P1.12 provisioning.
- **O5.** Validation is one owner transaction under `idle_in_transaction_session_timeout=30s`. A client-side `check.finish()` longer than 30 s ends the session, and the attempt becomes interrupted. The outcome is safe; measure at Q.
- **O6.** `publish_step` zips name-ordered receipt rows with model order (`receipts.rs:114-134`). This is correct only because `ValidatedModel` sorts relations and invariants by name (`model.rs:47, 167`); a comment should say so.
- **O7.** A validated but unpublished facts generation stores no admission, which is held in memory. After a crash it lists interrupted without availability. It can only be aborted, so this is acceptable.

### Re-confirmation of the claimed closures

| Source finding | Assessment (2026-09-29) | Evidence | Remaining route |
|---|---|---|---|
| Core [C01](design_review_cutover-core_2026-09-29.md#C01), lifecycle half | **Closed (Implemented; focused-Tested, historical receipts)** | One schema per generation; retire drops schema and control records in one transaction (`mod.rs:190-220`). A self-referencing relation is published and retired (`generations.rs:79, 135-143`). Select and retire are serialized, with both orderings seen (`lifecycle.rs:359-383`). A lease blocks retire (`:335-357`). Retire is atomic under a post-effect fault (`:391-408`). A read of a retired generation is refused, never empty | Lease half → P1.10 (provider sessions). Operability of reset → F02 |
| P0 exit [F02](design_review_p0-exit_2026-09-29.md#F02) | **Closed at store level** | The facts schema holds exactly `facts_relations()` (`ddl.rs:23-33`; `lifecycle.rs:145-150`). Receipts, invariants, grants and digest are scoped (`mod.rs:225-229`; `receipts.rs:56-57, 112-113`). The lease refuses with `Frontier` (`mod.rs:384`; `lifecycle.rs:166`); raw reader SQL gets `42P01` (`:169-170`). `check` shadows per (state, frontier) (`verify.rs:111-125`) | P1.10 `GenerationTable` and P1.11 `query` map the missing relation to the typed `Frontier` refusal, not a raw SQL error |
| P0 exit [F07](design_review_p0-exit_2026-09-29.md#F07) | **Closed for the in-memory class** | `ModelError::Infrastructure{class}`; `From<Error>` preserves the class through `StageSink` (`mod.rs:61-71`); capture I/O is `Io`. Controls cover a terminated backend and a closed writer pool (`lifecycle.rs:296-311`) | Persisted class and SQLSTATE → F04; provider-session transport → P1.10 |

## 8. Library fit and total complexity

| Capability and owner | Consumer | Candidates | Pinned fit and gaps | Burden | Choice |
|---|---|---|---|---|---|
| Coordination (`generations`) | All lifecycle operations | PostgreSQL advisory locks, row locks, transactions | Fit. Queue semantics make exclusive requests block later shared ones (probe 2); try-locks do not | None | Keep; try-lock for operator commands (F03) |
| DDL (`ddl`) | Store, `check` | SeaQuery 1.0.2 | Fits | Small | Keep |
| Live catalog comparison (`verify`) | `store check` | `pg_dump --schema-only` diff; migra (unmaintained); stripe `pg-schema-diff` (Go) | The external tools need a binary, normalization, and a second model of roles and ACLs. The bespoke ~60-line `DESCRIBE` compares a shadow built from the *same* lowering | ~250 lines, rolled back | Keep; bespoke reason recorded in the module doc |
| Schema removal (`install`, `cleanup`) | Reset, retire | `DROP SCHEMA … CASCADE` | Fits per generation; the lock footprint is about 23 per table (probe 1) | — | Phase the reset (F02) |
| Test database (`testing`) | All PG suites | testcontainers | Fits; the pools lack production session options (F03) | Small | Build pools from `Config`/`RoleConfig` |

## 9. Alternatives and tradeoffs

| Alternative | Change propagation and local reasoning | Authority and composition | Test / substitution | Machinery and risk | Decision |
|---|---|---|---|---|---|
| Current baseline | Attempt path localizes scenarios 2 and 5 | Two refusal semantics (F01) | Real PG controls | `owned` branches | Revise F01 |
| Attempt-only lifecycle plus a `testing` fixture helper | P1.8 and P1.11 see one liveness model | One refusal meaning | Domain suites run through typestates | Deletes code | **Recommended** |
| `check` under a REPEATABLE READ snapshot | No lock | Catalog functions read the latest catalog, so descriptors can mix states | — | Wrong answers under concurrent DDL | Rejected (author's rationale confirmed) |
| `check` with shared installation key and per-generation keys | No store-wide pause | Orphan inventory races with create/cleanup | Harder to test | More code | Rejected |
| Exclusive key by try-lock, `Busy` when held | No convoy | Same | One control | Trivial | **Recommended** (F03) |
| Reset as one transaction (current) vs phased reset | Phased resumes after interruption | Same inventory | Resume control | A `resetting` marker | **Phased** (F02) |

## 10. Verification and uncertainty

| Claim or scenario | Label / date | Command or inspection | Outcome |
|---|---|---|---|
| **Probe 1: reset lock footprint** (F02) | **Measured (reviewer probe, approximate schema), 2026-09-29.** Transient; not retained, because this review is the only artifact | `docker run postgres:18.6-bookworm` with default settings; a plpgsql procedure created 8 schemas × 110 tables (bytea key, default, 7 CHECKs, PK, FK to `t0`); then `BEGIN; DROP SCHEMA g1..gN CASCADE; SELECT count(*) FROM pg_locks WHERE pid=pg_backend_pid(); ROLLBACK` | **passed** for N=1 (2,529 locks) through N=5 (12,633); **failed** for N=6 and N=8 with `out of shared memory` |
| **Probe 2: convoy behind a queued exclusive key** (F03) | **Tested (reviewer probe), 2026-09-29**, same container | A: `BEGIN; pg_advisory_xact_lock_shared(1279476824,0); pg_sleep(8)`. B: `pg_advisory_xact_lock(1279476824,0)` with `lock_timeout 20s`. C: `SET lock_timeout='3s'; pg_advisory_xact_lock_shared(1279476824,0)` | C cancelled by lock timeout after 3.04 s; B acquired after A committed |
| P1.5–P1.7 PG suites (`services` 8, `installation` 7, `lifecycle` 11, `generations`, `generation_stages`, 13 `domain_*`, `typed_conformance`, `typed_limits`) | Tested, **historical author receipts**, 2026-09-29 (plan §4.2) | As recorded | **not_run** here. The working tree carries uncommitted P1.8 edits to the same crate, so a run would not test `6df6f41` without a separate worktree; no finding depends on rerunning them |
| Lock order, atomic publication, frontier scoping, crash → interrupted | Implemented, code read 2026-09-29 | Cited lines | As assessed in §5–§7 |
| `just fmt`, `just test-all`, facts pilots | — | — | **not_run**: functional scope incomplete (binding acceptance timing) |

Closure controls for F02, F03 and F06 must use a default-configured server and production pool options; the answers must be written before the controls run.

## 11. Authority changes and dispositions

Proposed plan §8 rows. The plan is the single disposition owner; this review cannot edit it.

| Required change | Route and owner | Source | Proposed disposition | Closure evidence or revisit trigger |
|---|---|---|---|---|
| Attempt-only lifecycle; `testing` fixture helper; delete the manual steps and `owned`; §15.11 line | Implementation in `lctx-postgres::generations` and `testing`; in-place §15.11 amendment (ADR-0086 delegates lifecycle detail) | F01 | open → **before P1.8 lands** | F01 closure |
| Phased reset, or preflight with typed refusal; lock-table requirement in provisioning | `install`; P1.12 bootstrap | F02 | open → before P1.11 `store reset` | Reset over at least 8 generations on a default server; resumable after interruption |
| Try-lock `check`/`reset`; production pool options in tests | `verify`, `install`, `testing` | F03 | open → P1.11 | Busy control; concurrent pin succeeds |
| SQLSTATE-aware classes; typed `fail`; `Absent`; optional owned `abort` | `generations` | F04, P0 exit F07 (persisted part) | open → P1.8/P1.11 | F04 controls |
| Typed availability rows; `FailureClass` enum | `generations`, `admission` | F05 | open → P1.8 | P1.8 reads typed rows |
| Discriminating twins; receipt wording | Tests; plan §4.2 | F06 | open → now | Controls, corrected receipts |
| One `locks` module; `LeaseTerms` | P1.10 | F07 | open → P1.10 | P1.8 and P1.10 consume it |
| Frontier-owned scope and selectability | P3 design | F08 | deferred; trigger: P3 frontier design | Traced frontier addition |
| Ops attempt record vs registry | Dc / P1.11 `runs` | F09 | deferred; trigger: Dc or P1.11 `runs` | Decision plus `runs` control |
| Core C01 lifecycle half | Plan §8 core table | C01 | **closed (lifecycle half)**; lease half → P1.10 | This review's evidence |
| P0 exit F02 | Plan §8 | F02 (P0 exit) | **closed at store level**; reader mapping → P1.10/P1.11 | Typed refusal in `GenerationTable`/`query` |
| P0 exit F07 | Plan §8 | F07 (P0 exit) | **closed for the in-memory class**; persisted class → this review's F04; provider transport → P1.10 | — |

No exception records are requested.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | **unresolved** | Satisfied for scenarios 2 and 5. Scenario 4 is correct but stalls the store (F03). Scenario 6 is defeated beyond about five generations (F02). Scenario 1 awaits P1.10's lease extraction (F07). Scenario 3 dispatches in the store (F08, deferred) | F02, F03 now; F07 at P1.10; F08 at P3 |
| A2 Encode meaning structurally | **violated (bounded)** | Lifecycle states, frontier, failure and interruption are explicit, but one decision — what a refusal means — has two answers selected by `owned` (F01). Availability and failure classes are untyped projections (F05) | F01; F05 at P1.8 |
| A3 Extend through composition | **violated (bounded)** | P2 composes `begin` → sink → typestates with no new store rule. The manual path is parallel machinery with no production consumer, kept to preserve an obsolete retry control (F01, DP-16) | F01 |

**Bounded change decision: Revise.** Correct F01 and F02 before P1.8 and P1.11 build on them; F03–F07 are Low and routed; F08 and F09 are deferred with triggers.

The core protocol is sound, and needs no change:
- the lock order and try-lock refusals;
- attempt ownership through a lock held by the lifecycle connection;
- failed as a terminal, abort-only state;
- atomic publication and retirement;
- frontier-scoped schemas with typed and physical refusal;
- digest-checked leases that block retire;
- a verified non-elevated owner and a refused legacy history.

Core C01's lifecycle half and P0 exit F02 close at store level. P0 exit F07 closes for the in-memory class.

The manual path is not a justified second semantics. It has no production consumer, contradicts T10 and DP-19, and is already spreading into P1.8. The fix is mechanical and removes code.

**Enclosing architecture: unresolved.** The P1 store/runtime is not certified by this slice:
- provider sessions and the driver-neutral lease (P1.10, scenario 1);
- the operator surface (P1.8, P1.11);
- the transition (P1.12/P1.13);
- P2's multi-stage driver.

Each has its own route. The P1.10 bounded review is the next architectural checkpoint. Review acceptance is not release qualification.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Attempt-only lifecycle (`generations`, `testing`) | F01 | No manual steps outside `testing`; R2 as a new-attempt retry |
| 2 | Phased or preflighted reset (`install`) | F02 | Reset over at least 8 generations on a default server |
| 3 | Try-lock operator commands; production pool options in tests | F03, F06 | Busy and concurrency controls |
| 4 | Typed failure classes and availability | F04, F05 | P1.8 reads typed rows; FK → `invalid` |
| 5 | `locks` module and `LeaseTerms` with P1.10 | F07 | P1.10 consumes it |
| — | Frontier ownership (P3), ops attempts (Dc) | F08, F09 | Triggers as stated |

**Next step:** the store owner (`lctx-postgres::generations`) corrects F01 and F02, then P1.8 continues on the single lifecycle.

<a id="re-inspection"></a>
## Re-inspection (2026-09-29)

| Field | Value |
|---|---|
| Subject | Commit `5beafd8` (attempt-only lifecycle, phased reset, typed failures), read as `git diff 1ffe05c 5beafd8` for `crates/lctx-postgres`, `crates/lctx-model{,-macros}` and `crates/cpg-core/tests/generation_read.rs`, with plan §4.2 ("Store-lifecycle review corrections") and §8 ("P1 store-lifecycle review findings") and DESIGN §15.11. Line citations are at `5beafd8`. HEAD moved during the re-inspection (P1.11 `244eda4`, P1.12/P1.13 `c65c0c7`…`dcbc593`). Of the corrected store code, those commits change only `generations/mod.rs` (`Error::NotInstalled`, `GenerationStore::open`, `GenerationId::from_hex`) and one rename in `lifecycle.rs`; neither affects a finding below |
| Reviewer | Fresh-context `design-reviewer` subagent (not the author) |
| Method | For each finding: its own closure evidence, read in source and re-run where a control carries it (checks below). The `testing` harness was read in full to see whether it restores a second lifecycle semantics. `cargo tree` was used for the feature boundary |

### Per-finding outcome

| Finding | Closure evidence the review required | Observed at `5beafd8` | Outcome |
|---|---|---|---|
| [F01](#F01) | No manual step outside `#[cfg(feature = "testing")]`; no `owned`; R2 as a new-attempt retry; no `Manual` writer | `git grep create_conformance` over `crates/` is empty at `5beafd8` and at HEAD. The only registration is `lifecycle.rs:43-58` (`register`), which takes the attempt lock (`:50`) in the transaction that creates the row; `begin`, `begin_conformance` and `begin_harness` (`:215-219`) all use it. `owned` is gone from `control.sql:19-33`, and `seal_step`/`validate_step`/`publish_step`/`fail_step` (`receipts.rs:15, 43, 104, 161`) take no ownership or optional-schedule parameter. R2 (`generation_stages.rs:136-157`): a tiny budget fails validation with stored class `resource`; a second validation is refused `State` (the typestate is consumed); the generation is aborted; a new funded attempt over the same rows validates to the stage path's content digest. The catalog's `Writer` is Live, Interrupted or Ended (`catalog.rs:20-27`). DESIGN §15.11 is amended (`semantic-model.md:418-424`) | **closed** |
| [F02](#F02) | Reset over at least 8 full-model generations on a default-configured PG18; an interrupted reset finished by a rerun; `check` clean | `install.rs:63-111` runs in three phases. (1) Under the try-exclusive installation lock, it confirms, try-locks every generation and attempt key (`Busy`), and deletes the installation row, so every step and lease refuses `Contract`. (2) It removes one generation per transaction: schema, control rows, selection pointer; this phase is idempotent. (3) It drops and reinstalls control. `reset_is_phased_and_resumable` (`installation.rs:282-305`) covers eight published whole-model generations on the testcontainers default server (only `fsync=off` changed). A trigger fault at the last generation leaves exactly that schema, and a new attempt on the withdrawn installation is refused `Contract`; the rerun completes and `check` is clean. **passed** (fresh run). The control discriminates because of this review's probe 1 footprint (about 2,500 locks per 110-table generation, six over the default lock table). That footprint was not re-measured on the real lowering | **closed** (O8) |
| [F03](#F03) | `Busy` by try-lock; a concurrent pin under a 1 s `lock_timeout` succeeds | `locks.rs:26-36` retries `pg_try_advisory_xact_lock` for about 1 s, then refuses `Busy`. It is used by install (`mod.rs:118`), `check` (`verify.rs:69`) and reset (`install.rs:77, 88, 104`). In `installation.rs:310-329`, a step in flight holds the key shared; `check` returns `Busy`, while a reader with `lock_timeout=1s` pins during check's retries. A blocking exclusive request would have queued ahead of that pin (probe 2), so the control discriminates. Reset refuses within 3 s (`:216-221`). Test pools carry lock, statement and idle-in-transaction limits (`testing.rs:48-53`). Shadows are built and rolled back one savepoint at a time (`verify.rs:116-125`). A concurrent *copy* is not exercised; it takes the same shared key by the same statement | **closed** (O9) |
| [F04](#F04) | Validation FK stores `invalid` with its constraint; writer transport stores `transport`; pinning a retired id returns `Absent` | Class and safe detail: `failure.rs:61-74, 82-96`. `fail(&ModelError)` and the owned `abort` (T9): `lifecycle.rs:139-141`. `Absent`: `mod.rs:37-38, 225`. Controls passed on a fresh run: `tests/lifecycle.rs:119-142` (`("sealed","invalid")`, detail `SQLSTATE 23503 constraint ref_… table releases`), `:205-219` (`"transport"`), `:238-241`, and the retire race arm `:261` (`Absent`) | **closed against its closure evidence**; residual R1 |
| [F05](#F05) | P1.8 reads typed rows; no class or availability outside the enums can be constructed | `admission_families` (`control.sql:79-85`) has CHECKs rendered from the `FactFamily` codebook and `Availability::ALL` (`ddl.rs:103-104`) and is written in the publication transaction (`receipts.rs:147-150`). The catalog decodes through `Codebook::from_code`, `Availability::from_code` and `FailureClass::parse`, and refuses an unknown code with `Contract` (`catalog.rs:78-91`). `FailureClass` (`failure.rs:7-50`) renders the class CHECK (`ddl.rs:102`) and is the only source of stored names. The `Codebook` trait is derived (`lctx-model-macros/src/lib.rs:326-329`) | **closed against its closure evidence**; residual R2 |
| [F06](#F06) | Real reset twins; a post-effect publication fault; asserted race orderings; corrected receipts | A live harness attempt (attempt key) and a step in flight (installation key shared) both make reset refuse `Busy` promptly (`installation.rs:212-221`). A trigger on the `published` event fires after the admission rows and the grant, on a facts attempt; nothing survives, and the generation is failed (`tests/lifecycle.rs:167-187`). The seal race asserts both orderings (`:116`). The receipt sentences are corrected (plan lines 722, 742). All passed on a fresh run | **closed** (O10) |
| [F07](#F07) | The key literal once; P1.8 and P1.10 consume one module | `locks.rs` owns the installation key (three statement constants, `:12-15`), the key derivations (`:19-24`), the try-locks (`:26-43`) and the read-only `pg_locks` probes (`:45-61`). `interrupted()` (`mod.rs:234-246`) and the catalog (`catalog.rs:100-103`) use the probes. No test re-derives a key. `LeaseContract` (`lease.rs`) is consumed by `pin` and by provider sessions. Plain acquire statements for the generation and attempt keys stay inline (`lease.rs:61, 73, 78`; `lifecycle.rs:50, 93`; `mod.rs:426-429`); they take their keys from `locks`. The lease terms omit the relation set, which P1.10 recomputes: [P1.10 review F06](design_review_p1-provider-sessions_2026-09-29.md#F06) | **closed** |
| [F08](#F08), [F09](#F09) | — | Unchanged | deferred (triggers as stated) |

**R1 (F04 residual).** On the stage path, a store error crosses `StageSink` through `From<Error> for ModelError` (`mod.rs:74-83`) as `Infrastructure{class: Error::class(), detail: Display}`:
- `Error::Database` displays only "PostgreSQL operation failed" (`mod.rs:30`);
- `Error::class` maps SQLSTATE 23, 53 and 54 to `Refused` (`:53-63`).

A producer's `fail(&cause)` therefore persists a COPY constraint violation or a server resource refusal as `refused`, without a SQLSTATE (`failure.rs:51-60, 97`). The SQLSTATE-aware mapping exists only at lifecycle steps, which is all F04's closure evidence covered. P1.10's provider driver restates a third, divergent mapping. The shared cause is that SQLSTATE classification has no single owner. It is carried as [P1.10 review F03](design_review_p1-provider-sessions_2026-09-29.md#F03).

**R2 (F05 residual).** The third bullet of the F05 correction is not implemented: expose availability and the relation's family on the lease for P1.10. Neither `GenerationLease` nor `GenerationSession` discloses that a catalog generation's Flow relations are NotRequested. This is carried as [P1.10 review F07](design_review_p1-provider-sessions_2026-09-29.md#F07).

### The `testing` harness

The harness (`GenerationStore::begin_harness`, `GenerationAttempt::put`/`seal_harness`, `SealedAttempt::validate_with`, `lctx_postgres::testing::Harness`) is an alternative writer front end over the same attempt lifecycle. It is not a second lifecycle semantics:

- **Ownership and liveness are the same.** `begin_harness` uses `register` (`lifecycle.rs:215-219`), so the attempt lock is held before the row is visible. A dropped harness lists as interrupted, and the catalog's writer classes apply unchanged.
- **Refusal is the same.** `seal_harness` runs `seal_step`, `validate_with` runs `validate_step` through `validate_charged`, and publish runs `publish_step`. Every refusal goes through `Lifecycle::fail` to `failed` (`:162-175, 244-247`). No path restores a prior state. The typestate is consumed, so retrying in place cannot be expressed; `Harness` then refuses `State` (`testing.rs:115-138`). A failed or interrupted harness generation is removed only by `abort`.
- **Guards hold.** `put` and `seal_harness` refuse an attempt that has an execution identity (`lifecycle.rs:224, 233`). A stage `copy` or `seal(receipt)` on a harness attempt fails on identity (`:113, 125`) and records `failed`. Harness generations are Conformance and never selectable (`mod.rs:164`).
- **Relaxed only as test conveniences, and only in input binding:**
  - the planned outputs are whatever was written, fixed at seal (`:237-240`), so a harness generation cannot fail publication's planned-output check;
  - no stage outcomes are recorded;
  - the schedule digest is constant and the caller chooses the profile;
  - validation can be charged to a caller's budget.

  Controls for planned-output and stage-receipt enforcement must therefore use the stage path. They do: `generation_stages.rs`, and the "a stage receipt missing" fault in `failed_publication_is_atomic`.
- **The feature boundary holds.** Everything is gated on `testing` (`lifecycle.rs:209`, `lib.rs:20-21`). Under workspace feature unification, `testing` enters only through dev-dependencies (`cpg-core`, `cpg-extract`, `lctx-postgres` itself). `cargo tree -p lctx -e features -i lctx-postgres --edges normal` shows only `default` for the CLI build (Interface-checked, 2026-09-29).

### Observations (no finding)

- **O8.** An interrupted reset leaves the installation withdrawn. `store check` then reports "installed from a different model or lowering" (`verify.rs:84-86`), and `install` and every step return `Contract`. The operator has to infer that rerunning `reset` finishes the job. A distinct report such as "reset in progress; rerun `store reset`" would make the state self-describing. Route: P1.11 messages or the P1.12 runbook.
- **O9.** `mixed_lifecycle_does_not_deadlock` (`tests/lifecycle.rs:344-346`) treats a `Busy` from `check` as a failure. Since F03, `Busy` is check's contract under concurrent steps, so a slower validation can fail the control spuriously. The control should accept `Busy`.
- **O10.** The historical P1.7 and P1.8 receipts in plan §4.2 still describe the deleted manual path: "registers an `owned` facts generation" (line 725) and "the writer: Manual, …" (line 757). A one-line pointer to the corrections receipt would keep the working set truthful (ADR-0042).
- The added `Lifecycle::end` (`lifecycle.rs:89-95`) confirms the unlock with a round trip before closing. It is sound: the attempt lock is gone before a caller's immediate abort or retire, and a lost connection has already released it server-side.

### Checks (2026-09-29)

| Command (each prefixed as in AGENTS.md) | Tree | Outcome |
|---|---|---|
| `cargo test --release -p lctx-postgres --test installation --test lifecycle` | `244eda4`, plus the then-uncommitted P1.12 owner-pool edit to `crates/lctx-postgres/src/lib.rs`, committed as `c65c0c7`; not on these tests' pools | **passed** (installation 9, lifecycle 12) |
| `cargo test --release -p lctx-postgres --test generation_stages --test generation_catalog --test generations` | `51aae78`, clean | **passed** (1, 2, 2) |
| `cargo tree -p lctx -e features -i lctx-postgres --edges normal` | `244eda4` | `testing` absent from the CLI build |
| `services`, the 13 `domain_*` suites, `--doc`, `cpg-extract`, `lctx-model` | — | **not_run** here; the author's receipt (plan §4.2, 2026-09-29, run twice) is historical |
| `just fmt`, `just test-all`, facts pilots | — | **not_run** (binding acceptance timing) |

### Updated dispositions (proposed for plan §8)

| Finding | Proposed disposition | Evidence |
|---|---|---|
| F01 | closed | This re-inspection; `generation_stages` R2 control |
| F02 | closed | `reset_is_phased_and_resumable`; P1.12 records that no provisioning note is needed |
| F03 | closed | `check_and_reset_refuse_busy_without_stalling_the_store`; reset twin |
| F04 | closed; residual R1 → [P1.10 review F03](design_review_p1-provider-sessions_2026-09-29.md#F03) | FK → `invalid` with SQLSTATE; transport; `Absent` |
| F05 | closed; residual R2 → [P1.10 review F07](design_review_p1-provider-sessions_2026-09-29.md#F07) | Typed `admission_families`; `FailureClass` |
| F06 | closed | Three controls; corrected receipts |
| F07 | closed; lease relation set → [P1.10 review F06](design_review_p1-provider-sessions_2026-09-29.md#F06) | `locks` module; `LeaseContract` |
| F08, F09 | unchanged: deferred with their triggers | — |

### Updated judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | **satisfied** for scenarios 1, 2, 4, 5 and 6; scenario 3 is deferred (F08) | Phased reset (6); try-locked operator commands (4); one lease protocol and lock module consumed by P1.10 (1) |
| A2 Encode meaning structurally | **satisfied** | One refusal meaning for every generation; failure classes and availability are enums with rendered CHECKs |
| A3 Extend through composition | **satisfied** | P2 composes `begin` → sink → typestates; the manual machinery is deleted; the harness is test-only and adds no lifecycle rule |

**Bounded change decision (re-inspection): Accept scoped.**
- F01 and F02 are corrected, with passing controls.
- F03–F07 close against their closure evidence. Two residuals (R1, R2) are carried to the P1.10 review, which owns the reader and shares their causes.
- The P3 frontier addition (F08) and the ops attempt record (F09) are excluded, with their triggers.

**Enclosing architecture: unresolved.** Still open: the P2 multi-stage driver (A0), the P3 frontier (F08) and P5 serving. The reader is judged separately in the [P1.10 provider-session review](design_review_p1-provider-sessions_2026-09-29.md). Review acceptance is not release qualification.
