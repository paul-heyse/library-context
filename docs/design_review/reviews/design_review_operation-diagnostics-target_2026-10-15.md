# Design review: operation diagnostics for Catalog-completion operations

**Design / target review, 2026-10-15.** Fresh `design-reviewer`. Standard: core principles and
template 3.3, Heuristics for Efficient Architecture 1.0, code-intelligence profile 1.5, and the
[library-context binding](../design_principles/binding/library-context.md).
Baseline: main `24b0d08b`.

## Integrated assessment

**What diagnostics must do.** The Catalog scope runs long native operations: Summary, Base,
independent admission, publication, cold restore and the pinned FastMCP pilot. These run under
nextest, the managed-run harness and owned SurrealDB campaigns. Agents (Claude Code and Codex)
must be able to troubleshoot them in two situations:

- **while they run:** is it progressing, where is it, and what is it waiting on?
- **after they end, however they end:** what was open at the deadline, where did the time go, did
  the cancel take effect, and which owner did what?

**What already works.** The codebase has strong authorities for *finality* and *failure*:

- [`Completion`](../../../crates/lctx-model/src/domain/completion.rs) and the native drain receipts
  ([`signal.rs`](../../../crates/lctx-native/src/signal.rs)) keep local, remote, storage and
  Unknown outcomes separate.
- [`FailureDiagnostic`](../../../crates/lctx-model/src/domain/diagnostic.rs) gives bounded, coded,
  secret-free failure evidence.
- [`Phase`](../../../crates/lctx-native/src/phase.rs) never invents a terminal for dropped work.

These are preservation constraints for any remedy.

**What is missing.** There is no comparable owned model, carrier or reader for *operational
observation*: progress, waits, time attribution and interruption. The 2026-10-15 incidents show the
consequence.

- **Silence where the time went.** In run `12d184` the recursive Base stage logged nothing for 410 s
  after a 0.58 s native call. When nextest's SIGTERM arrived, its phase closed `failed` with the
  error `Interrupted: "native read delivery cancelled"`. Static source cannot tell which of about
  six uninstrumented reads or synchronous sections consumed the 410 s.
- **Truncation that invents a stall.** In replay `bd6eb5`, a 10 MB output cap dropped 26.8 MB from
  the middle of the run. Of the 22,018 retained lines, 21,982 are per-call phase text. The
  truncation left a false open phase that the readers label "truncation: unknown", even though the
  log contains an explicit `[truncated 26805625 bytes]` marker.
- **A cancel that did nothing.** In `8a3347` the operator's cancel at 18:26:36 produced no stop
  request. The test ran on to nextest's 900 s timeout. The run owner records only its final state,
  so the evidence cannot explain why.
- **The pattern recurs.** Four plans record that the timeout cause remained unestablished:
  native-execution-efficiency, efficiency-alignment O-F41, persisted-graph and Catalog §6.

**The requirement.** DP-21 says cost telemetry needs "a separate functional or operational
requirement". The operator's request and these repeated, unexplained timeouts supply it. §1.1 below
defines it as ten diagnostic questions with a proportionality envelope. Every proposed signal is
judged against those questions.

**Seven structural causes.**

1. **No model of operational observation.** Cancellation is encoded at least three ways as
   sentinel errors, such as `ModelError::Invalid("compilation cancelled")`. These project as
   *invalid model* in the failure diagnostic and as `failed` in the phase stream. Phases have no
   parent, progress or interrupted state, and only inclusive time. ([F01](#F01))
2. **The wrong carrier.** Observations travel as human-format stderr inside nextest's capture.
   Nextest stores it only when the test ends, a user-level cap truncates it, and a Python regex
   parses it back. Span-carried identities are lost in that parse. ([F02](#F02))
3. **Delivery depends on convention.** Each of 66 spawn sites must remember to propagate a scoped
   dispatcher; events from sites that forget are silently lost. ([F03](#F03))
4. **Instrumentation sits at await seams, not diagnostic units.** That gives per-call volume where
   it is cheap and silence where the time goes. There is no progress, no aggregates and no
   efficiency counters. ([F04](#F04))
5. **The deadline is invisible as a deadline.** Nothing is captured at SIGTERM, and the 10 s grace
   kills the drain. ([F05](#F05)) Separately, the harness's own cancel protocol is unobservable.
   ([F06](#F06))
6. **Readers lose information.** They drop unattributed events, count phase rows twice and
   mislabel known truncation. ([F07](#F07))
7. **SurrealDB's useful server-side signals are unused.** The client also discards the
   per-statement server time it already receives. ([F08](#F08))

**Recommendation (Proposed).**

- Give the observation domain a small typed vocabulary.
- Give each process an append-only structured journal in an owner-supplied directory. This
  generalizes the existing drain-receipt pattern rather than adopting a collector.
- Write the journal from a process-level observer thread that records:
  - periodic heartbeats of the open-interval registry, progress and budgets;
  - per-stage aggregates;
  - an interrupt snapshot at the first signal.
- Let long model kernels report progress and observe cancellation through a model-owned,
  effect-free probe.
- Add an external `/proc` observer, with stack capture on demand.
- On the database side, use three sources: the client's per-statement server time, the campaign
  server's slow log, and root `/metrics` sampled at campaign boundaries.
- Serve all of this to both agents through CLI JSON readers.

None of this writes diagnostics into the workload's database, adds a per-row event stream, or
requires an OpenTelemetry collector.

**On the SurrealDB hypothesis.** SurrealDB earns a real place through its server-side signals:

- the slow log, which includes timed-out statements;
- root statement, KV and conflict metrics;
- `QueryStats`;
- `EXPLAIN ANALYZE` for SELECT.

Its HTTP `/mcp` surface is a useful *optional* inspection instrument. It is not the diagnostic
surface, for four reasons:

- the dominant questions concern facts that are not stored in the database;
- there is no view of statements currently running;
- VIEWER writes "succeed" silently with `[]`;
- results truncate at 256 KiB.

**Decision: Revise.** The architecture of operational diagnostics needs revision. A1–A4 are all
violated for the selected scenarios, and G1/G2/G5–G8 fail on their own evidence. Product semantics
are outside this review.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Diagnostics for operations in the [Catalog completion plan](../../plans/catalog-completion-plan_2026-10-14.md) and its production, development and acceptance companions. Producers: `lctx-native` phase/signal/diagnostic; `cpg-extract` logging; `cpg-core` stage runner, Base, Summary and workspace; `lctx-compiler-store` native calls and `borrowed_cpu`; control schema. Harness: `scripts/runs.py`, `run_delegation.py`, `surrealdb_service.py`/`surrealdb_fixture.py`, `catalog_completion_validate.py`, `verify_evidence/*`, `.config/nextest.toml`. Store: the patched SurrealDB 3.3.0 `405f6c02…` campaign servers |
| Standard | Core/template 3.3; heuristics 1.0; code-intelligence 1.5; library-context binding |
| Tier · purpose | Design · target |
| Reviewer · date | Fresh `design-reviewer` subagent, 2026-10-15; read-only |
| Maturity and outcome | Design phase. Catalog V01–V03 exits are open, with three 900 s timeouts and one unexplained cancel delay. This decision should fix the diagnostic architecture before those exits resume, so the next timeout is explainable from the evidence it leaves |
| Supported scope | Live and post-run diagnosis of native test controls, CLI compiles and the Catalog validation runner, on owned campaigns, under nextest process-per-test. Out of scope: serving/query-path diagnostics, paused SA9 queues, embedding/MCP-submitted runtime, and remediation planning |
| Workload premise | Native controls of 300–900 s, with a 3,600 s FastMCP pilot (builds excluded). Base and Summary run per-root loops over thousands of roots and several relations each. Several tests run concurrently on one owned campaign server. The disk is 98 % used (39 GB free on 2026-10-15) and `build/runs` holds 12 GB. Consumers are Claude Code and Codex agents using shell tools; Codex also has MCP configuration |
| Expected changes | Add a stage or kernel; substitute the carrier (text → JSON/OTLP/SurrealDB); grow to the pilot; run concurrent tests per campaign; failures (SIGTERM at the deadline, a DB stall, a provider deadlock or blocked runtime, a cancel that does not take effect) |
| Baseline | `fmt` text to stderr under the filter `warn,lctx_phase=info`, stored by nextest when the test ends, projected by regex. No OTLP or JSON output, no heartbeat, no server diagnostics beyond `--log=warn` in journald |
| Method and coverage | Source inspection at `24b0d08b`. Re-inspection of the decisive incident artifacts: `12d184` `evaluate_base` lines 8082–8090 and nextest TERMINATING/TIMEOUT lines; the `bd6eb5` truncation marker and line counts; `8a3347` `cancel.json`/`record.json`/delegated `recovery.json`. Assignment A ([SurrealDB E1–E20](../evidence/2026-10-15_operation-diagnostics-surrealdb/README.md)) and B ([instrumentation](../evidence/2026-10-15_operation-diagnostics-instrumentation/README.md)) probes, read in their evidence folders. Assignment C [incident retrospective](../evidence/2026-10-15_operation-diagnostics-incidents/README.md) (retained by the coordinator at publication). Interface checks of DataFusion 55.1.0 `TrackConsumersPool::metrics()` and the vendored SDK's per-statement stats. No new probes; three are requested in §10 |

### 1.1 The operational diagnostic requirement

DP-21 and H26 reject cost telemetry without a separate requirement. This review establishes that
requirement from the operator's 2026-10-15 request and the recurring evidence:

- Four plans state that the timeout cause remains unestablished.
- The 2026-10-11 test-evidence review's functional target included "inspect live/interrupted runs",
  which no implemented reader can do today.

The ten questions below revise the coordinator's Q1–Q18. They are the workload against which
proportionality is judged.

| ID | Question (consolidating Q#) | When | Delivery class |
|---|---|---|---|
| D1 | Is the operation progressing, and where? The open stage → phase → wait or kernel stack per process, with elapsed times; ordinal/total for bounded loops; last progress per stage (Q1, Q2, Q7) | Live, within one heartbeat; and post-run | Always on |
| D2 | Is it waiting or computing, and on what? Open waits by kind (native-call acknowledgement, admission permit, provider join, completion gate) with stage, query digest and elapsed; the open CPU kernel; process CPU and thread-state histogram (Q3) | Live and post-run | Always on (in-process and external `/proc`) |
| D3 | What is the current charge against each budget? Reserved/limit/peak per budget owner; top DataFusion consumers, structured (Q4, Q14) | Live and post-run | Always on (heartbeat) |
| D4 | What is the server doing? Per-campaign statement counts and durations by type, KV reads/writes and conflicts; statements that were slow or timed out (Q5, Q12) | Post-run; live at boundary granularity | Always on (cheap); there is no in-flight server view in 3.3.0 |
| D5 | What was open at the deadline or cancel? Signal kind and time; open stack with elapsed; last progress; per-owner time of cancellation observation and terminal (cancelled after drain, or interrupted while open); whether the journal closed (Q6, Q7, Q10) | At interrupt | Always on |
| D6 | What were the stacks and thread states? (Q9) | Live on demand; automatic at the deadline if probe P1 qualifies it | On demand |
| D7 | Did the harness deliver the cancel, and when? Run-owner stop decisions per delegated owner, signals sent, child exits (Q10, `8a3347`) | Live and post-run | Always on (tiny volume) |
| D8 | Where did time and work go per stage? Attributed time split into CPU kernels per producer, waits per kind, and native-call server execution versus client wall time, plus an explicit *unattributed* remainder. Counts of roots, rows, statements, retries/conflicts and reuse hits/misses with reason; budget peak per stage; slowest-k roots (Q8, Q11–Q14) | Post-run (stage terminal), partial in the heartbeat | Always on (aggregates) |
| D9 | Who did it, and how did it end? Compilation attempt, producer/stage and native attempt on every interval, retry and refusal; coded refusals from every pool kind; typed cancellation (Q15–Q17) | Post-run | Always on |
| D10 | How do the test outcome, run termination, delegated cleanup and native finality relate, in one reader, for any run (Q18)? | Post-run | Always on (reader) |

**Coverage is part of every answer.** Each reader states whether its observation stream is:

- complete;
- truncated, and by how many bytes;
- ended without a close marker (killed);
- partially dropped, with counts by reason;
- not captured.

An unobserved interval is "unknown", never "absent". This is the diagnostic analogue of CI-04,
enforced here under DP-02, DP-12 and G5.

**Proportionality envelope.** These are design bounds, not measured claims.

- **Volume** grows with stages, kernel families and wall time (one heartbeat per interval per
  process), not with roots, rows or calls. Per-call detail stays opt-in at DEBUG.
- **No diagnostic writes into the workload's database or transactions.** No diagnostic answer may
  depend on the database being responsive.
- **Survival:** observations survive SIGTERM, abort and SIGKILL up to the last completed write.
- **No secrets:** no SQL text, bound values or payloads in journals. Keep the existing query-digest
  discipline and the slow log's parameter deny-list.
- **On demand, not always on:** stacks, perf, `EXPLAIN ANALYZE` and generic database MCP.

**Explicitly not required:**

- per-row or per-call event streams;
- distributed tracing or a collector service;
- continuous profiling;
- a durable cross-run telemetry store;
- an in-flight server statement listing, which 3.3.0 Community does not provide (A, E12 and source
  search).

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility (current) | Consumer contract | Dependencies | Expected reason for change |
|---|---|---|---|---|
| `lctx_native::phase` | Interval observations (begin plus terminal, inclusive `elapsed_ms`), `wait` on pending futures, resource-refusal event; captures the dispatcher at begin | Free-form `&'static str` names; status strings; fields only on the event (identities live in spans) | `tracing` 0.1.44; process context from env/procfs | New observation kinds, terminal states, progress |
| `cpg_extract::logging` | Builds the `fmt` stderr subscriber; scoped `dispatch()` for tests, global `init_logging()` for CLI | Human text; the filter is the output boundary | `tracing-subscriber` (`env-filter` only; no `json`) | Carrier substitution |
| Stage runner (`cpg_core::compilation`) | One phase per declared stage (`Phase::begin(declaration.name)`, lines 780/1259); `finish_result` at 1034/1307 | Stage name derived from the declaration (a strength) | Model schedule declarations | Stage aggregates, attribution |
| `stage_runtime::borrowed_cpu` | Sync CPU kernels via `block_in_place`/inline; DEBUG begin/returned | DEBUG only since G-07 | tokio | Kernel time and progress |
| Model kernels (`lctx-model` production, summary, scopes) | Pure finite compute | No progress, no cancellation | None (no `tracing`) | Progress/cancel probe |
| `lctx_native::signal` | SIGINT/SIGTERM → cancellation callbacks; drain receipts (`create_new`, fsync, pid+start_ticks) in an owner-supplied directory | Receipt: state/signal/completion; no times | tokio signal; env `LCTX_NATIVE_DRAIN_DIRECTORY` | Interrupt snapshot, signal time |
| Diagnostic model (`lctx_model::domain::diagnostic`) | Coded, bounded failure projection | Append-only codes; class/stage/identities | `ModelError` | Typed cancellation code |
| Evidence projection (`verify_evidence/runtime_adapter.py`, `projection.py`) | Recovers phase rows from captured text (regex), and from JSON (dead branch) | `outcome: observed`; drops events lacking a matching invocation or phase id | nextest JUnit/`native.log`; capture invocation | Journal reader, coverage |
| Managed runs (`runs.py`, `run_delegation.py`) | Run ownership, cancel, delegated cleanup; `progress.json` from Python consumers | Final-state `record.json` | Process identity, locks | Lifecycle event log, live progress fold |
| Campaign service (`surrealdb_service.py`) | systemd unit, `--log=warn`, 20 s query / 10 s transaction timeouts | journald | patched 3.3.0 server | Slow log, metrics scrape |
| Control schema (`control/schema.surql`) | Native finality authority (attempt/effect/hold/pin/retirement) | No timestamps or stage columns | — | Not a diagnostic store (see F09) |

### Concepts and authority

| Concept | Current authority | Identity | Consumers and gaps |
|---|---|---|---|
| Stage | Model schedule declaration (`declaration.name`) | Declared name per compilation attempt | Phase names; good. No per-stage aggregates |
| Phase/interval | Call-site string plus per-process `AtomicU64` | `(pid, start_ticks, phase_id)` | No parent; kind (wait / kernel / stage) implicit in naming |
| Compilation attempt, producer, native attempt | Spans `compilation{attempt}`, `compiler_producer{producer,attempt}`, `native_call{attempt}` | 64-hex content hashes | Present only in span text; the text projection discards spans |
| Process / test / run / invocation | `/proc/self/stat`; `NEXTEST_*` env (unused); run directory; `LCTX_EVIDENCE_INVOCATION` (captured runs only) | Each separately | Joined by hand across four locations (C, G8) |
| Cancellation | **None.** At least three encodings, all sentinels: `Invalid("compilation cancelled")` (admission.rs:69, workspace.rs:1629, native_calls.rs:118), `Invalid("native transaction retry cancelled")` (retry.rs:174/214), `Cause(io::Interrupted "native read delivery cancelled")` (prepared.rs:14, owned_read.rs:14, selected_backup.rs:860) | — | Phase terminal → `failed`; diagnostic → Model/Invalid, or Model/Context + Opaque |
| Interruption (open at exit) | Implicit: a begin without a terminal | — | Indistinguishable from truncation, a dropped event or a different dispatcher |
| Progress | None | — | Live D1 impossible |
| Time | Inclusive wall `elapsed_ms` on terminals | — | No exclusive or attributed time; nested intervals are not additive (Catalog §6) |
| Resource charge | `ResourcePool::{reserved, limit, peak}`; `stage_peak` default `None`; refusal event at one boundary | Owner `&'static str` | No gauge emission; DataFusion structured metrics unused |
| Finality | `Completion`, drain receipts, control tables | Native ids | Correctly independent (preserve) |

**Observation and fidelity table** (the profile's fact table, applied by analogy to runtime
observations):

| Observation | Producer | Fidelity | Coverage/unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| Phase begin/terminal | `Phase::emit` via captured dispatch | Observed (owner-emitted) | Lost if output is truncated, never projected without an invocation id; a begin without terminal is ambiguous | `(pid, start_ticks, phase_id)` | Projection, humans |
| `observation=…` events (`base_selected_input_eof`, `completed_input_binding`) | `tracing::info!` macro, ambient dispatcher | Observed | Dropped by the projection (no `phase` field); lost in spawned tasks without propagation | None | Humans only |
| Retry terminal/conflict | `retry.rs` `tracing::info!` (digest, stage, effect id, retries, elapsed) | Observed, log₂-bounded | No invocation id → dropped; cancel terminals missing in I3 | Sequence, query digest | Humans only |
| `borrowed_cpu` | DEBUG `tracing` | Observed | Off by default | Producer name | None by default |
| Resource refusal | `phase::resource_refusal` | Typed values + library text | One pool path only | Owner name | Projection |
| Drain receipt | `signal::Receipt` | Owner finality | `signal` without time; "started" if killed during drain | pid/start_ticks/sequence | Fixture, runner |
| Server statement time | gRPC `QueryStats.execution_duration`, mapped by the vendored SDK | Observed (server) | Received and discarded | Statement index | None |
| Slow log, `/metrics`, OTLP | Server | Observed (server) | Not enabled or not scraped | ns/db/user labels; no session or trace id in the slow log | None |

**Model adequacy (A2).** The domain for these operations is not only Python semantics. It
includes their operational outcomes, and the core MUST applies wherever meaning is "established or
interpreted, including … orchestration … and presentation".

- **Finality and failure:** the distinctions are deliberate and authoritative. This is a strength.
- **Operational observation:** consequential distinctions have no representation. These are
  cancelled vs failed vs interrupted-open vs lost, inclusive vs attributed time, and progressing vs
  stalled.
- **Reconstructed meaning:** where meaning exists, a consumer reconstructs it. The Python adapter
  owns the terminal vocabulary (`TERMINAL`) and the field grammar (`FIELD` regex) independently of
  `phase.rs`.

## 3. Contracts, constraints and testing boundaries

| Contract | Expectations | Enforcement | Effects/failure | Isolated verification |
|---|---|---|---|---|
| Phase terminal only by its owner (phase.rs:130) | Dropped work stays incomplete | Type (`#[must_use]`, no Drop terminal); tests `abort_retains_begin…` | Correct; but `finish_result` maps every `Err`, cancellation included, to `failed` | Pure unit tests exist (good) |
| Phase observation reaches its owner's dispatcher | Begin and terminal go to the dispatcher captured at begin | Explicit `dispatch.event` (good) | Other observations use ambient macros; delivery depends on propagation at spawn sites | Per-dispatcher test exists; no spawn-propagation control |
| Failure diagnostic classification | DP-21: distinguish invalid model / unsupported / algorithmic / infrastructure / inconclusive | `model_cause` match | Cancellation is classified as `Invalid` (Model) or `Context`+`Opaque` | Unit tests do not cover cancellation |
| Drain receipt | Finality per native owner | `create_new`, fsync, rename | Killed at +10 s grace → stays "started" (I1, I4a) | Signal tests in `signal.rs` |
| Capture/projection | ADR-0147: originals authoritative, projection rebuildable, losses declared | Declared `losses` text; regex parse | Drops non-matching events silently; counts duplicated across channels; truncation "unknown" despite a marker | Python controls (scope 2026-10-11) |
| Run cancel | Owner honours `cancel.json`, settles delegated owners, then signals its child | `_own` loop: waits while delegated status is `active` before `signal_group` | Decisions are not recorded; `8a3347` unexplained | Python run controls exist |

**Absence lattice.** These are the states diagnostics must keep apart:

- not emitted;
- emitted but never stored (process killed before the write);
- stored but truncated;
- stored but dropped by the projection;
- open at exit (interrupted);
- terminal.

Today the first five collapse into "no terminal", or into "no row".

## 4. Composition and execution

**Signal path today.**

1. A tracing event is emitted.
2. The `fmt` layer writes it to stderr (text, with every span's fields repeated as a prefix; up to
   three 64-hex attempts per line in `12d184`).
3. Nextest buffers it per test and stores it only when the test ends. Live readers see nothing
   (B §4; C G1). The configured `status-level = "fail"` also hides SLOW lines.
4. The user record store caps it at 10 MB, keeping head and tail.
5. JUnit and the managed `native.log` both receive it.
6. The projection regex-parses both, keeps only events with `phase`, `phase_id`, a matching
   `invocation_id` and an integer `process_id`, and labels every row `observed`.

Each step is individually reasonable. Composed, they yield:

- no live view;
- a lost middle;
- dropped identities;
- 2× rows (127 MB of originals → ~400 MB of projection, per the coordinator's exploration);
- regex dependence on a presentation format.

**Physical work of observation.** Volume follows await seams:

- `phase::wait` emits a pair whenever a future is Pending;
- `base_selected_input` spans and `base_selected_input_eof` events fire per root per relation;
- `completed_input_binding` fires per binding.

That is O(roots × relations) lines of about 450 B (C: 15,203 eof plus 4,055 binding lines in I3;
21,982 of 22,018 retained lines in `bd6eb5`). The long synchronous and uninstrumented stretches
emit nothing:

- Base `load`/`consume_inventory!` reads;
- inline `evaluate_all_produced`/`evaluate_expression_scoped`;
- the Summary SCC fixpoint inside one DEBUG `borrowed_cpu`;
- `freeze_completed_inputs`;
- backup/restore.

The scoped Summary controls run on `#[tokio::test]`'s **current-thread** runtime
(`scoped_summary.rs:46,52`), so `cpu_placement()` is `Inline`. A synchronous kernel there blocks
the only runtime thread. No tokio-based heartbeat or signal listener can run until it yields.

**Execution fit (A4).** For the 3,600 s FastMCP pilot, roots and relations grow by orders of
magnitude over fixtures. The current design predictably:

- exceeds any fixed capture cap, so the middle is lost;
- grows projection storage at 2× originals on a 98 %-full disk;
- still says nothing during the dominant stretches.

That is avoidable amplification combined with missing signal. It is a structural A4 defect,
established statically without a benchmark (DP-22; H10, H26).

**Credible route (Proposed).** Volume becomes:

- heartbeat × time (on the order of one line per 5–10 s per process);
- plus stage aggregates (tens of lines per compilation);
- plus coarse interval pairs;
- with per-call detail at DEBUG.

The observation path holds no locks shared with the workload beyond a short registry update, and
writes no database rows.

The profile's analysis-record columns do not apply: no projection, analysis or synthesis stage of
code intelligence is in scope. The evidence projection is assessed above as composition.

## 5. Change and failure scenarios

| Scenario (kind) | Owner | Current route and propagation | Proposed route | Evidence |
|---|---|---|---|---|
| Add a long kernel or stage (domain extension) | Stage runner, kernel author | A stage phase comes for free (declared name, a strength). Progress, attributed time, counters and cancellation granularity all need hand-written `tracing` calls and string names. A kernel routed through `borrowed_cpu` is visible only at DEBUG | The stage runner records stage aggregates automatically. A kernel takes the model-owned probe (one declared input) and gets progress, cancellation and kernel time. No new reader code | Implemented (current, 2026-10-15); Proposed (route) |
| Text → JSON / OTLP / SurrealDB sink (mechanism substitution) | `logging`, projection | Readers depend on the `fmt` format via regex and on a `TERMINAL` set copied from `phase.rs`. Substitution propagates into Python; JSON is a dead branch today | Readers depend on the versioned observation vocabulary, not on the sink. A sink is a layer choice | Implemented / Proposed |
| FastMCP pilot, 3,600 s (growth) | Producers, capture | Per-root lines × larger corpus → truncation and 2× projection; silent kernels unchanged | Volume ∝ time and stages; heartbeat shows progress ordinal/total | Static (A4) |
| Concurrent tests on one campaign (concurrency) | Fixture, server | Server journald at WARN mixes tests; nothing attributes server work to a test | Client-side per-call server time and digest per process attribute work; root `/metrics` gives campaign totals and conflict rates. Slow-log lines carry no session id, so they correlate only by time window and normalized statement | Tested A E1/E5 (server facts); Proposed |
| 900 s SIGTERM (failure) | signal, Phase, nextest | SIGTERM → callbacks → typed-less cancellation → `failed` phases. Drain SIGKILLed at +10 s (I1 `TIMEOUT [910.085s]`); receipts "started" | The first signal writes an interrupt snapshot. Owners record cancellation observation and `cancelled` after drain; open intervals are `interrupted`. Grace sized to drain. A close marker, or its absence, distinguishes a kill | Re-inspected `12d184` 2026-10-15; Proposed |
| DB-side stall (failure) | native calls, server | Waits appear only as `native_call_acknowledgement` begin/returned pairs; no statement attribution | The heartbeat shows the open native call with stage, digest and elapsed. Server timeouts (20 s / 10 s) produce slow-log lines at timeout (A E2) and retry/conflict aggregates | Tested A E2; Proposed |
| Provider deadlock / blocked runtime (failure) | Runtime, kernels | Indistinguishable from slow CPU or a silent read | An OS-thread heartbeat (not tokio) still fires; open waits show no progress; external `/proc` shows futex-parked threads; on-demand gdb stacks | Tested B §5 (gdb, `/proc`); Proposed |
| Cancel does not take effect (failure) | `runs.py`, `run_delegation.py` | `8a3347`: no stop request until the 900 s timeout; final-state record only | Run lifecycle events record each `request_stop` result per owner and each signal; the process journal shows whether SIGTERM ever arrived | Re-inspected `8a3347` 2026-10-15 |
| Test the heartbeat or aggregation in isolation | phase/observer | n/a | Pure registry/aggregate tests with a scratch journal directory; no database | Proposed |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence | Required action |
|---|---|---|---|
| G1 Authority | **fail (narrow)** | The observation vocabulary is defined twice: `phase.rs` (`Terminal::name`, field set) and `runtime_adapter.py` (`TERMINAL`, `FIELD` grammar). There is no version or derivation. A new status such as `interrupted` would be dropped silently (`status not in TERMINAL\|{"begin"}` → `None`). Stage identity, native attempt and diagnostic codes have single authorities | F01, F07: one versioned vocabulary; readers refuse unknown versions |
| G2 Semantic fidelity | **fail** | Cancellation is carried by sentinels (three encodings) and projected as `failed` / `Invalid` / `Opaque`. Interrupted, truncated, dropped and failed collapse | F01, F05 |
| G3 Validity | pass (scope) | The observation path feeds no domain operation. Invocation ids are validated (phase.rs `valid_invocation`); `FailureDiagnostic` is bounded | — |
| G4 Hidden behaviour | pass, with constraints | Diagnostics do not change execution (FP-06). Remedies must keep it so: the journal directory and observer are observation-only, never facts or identities (CI-10 analogue); the kernel probe's cancel is a declared input with a typed outcome | Constraint on F04/F05 |
| G5 Consistency and recovery | **fail** | Truncated streams surface a false open phase; truncation is labelled "unknown" despite the marker; timeouts appear as plain `failed` with `reason ""`; the 10 s grace leaves drain receipts "started". Native Unknown itself is preserved correctly | F02, F05, F07 |
| G6 Transformation and reuse | **fail** | The projection doubles phase rows (JUnit plus `native.log`) and drops non-phase/unattributed events without counts, so derived counts and absence change meaning | F07 |
| G7 Truthful capability claims | **fail (narrow)** | O02/T03 are delivered as "associate coarse phases with the actual producer/owner and invocation". That association exists only in span text, which no reader recovers (the JSON branch is dead). T03 also asserts that DataFusion 55.1 offers only formatted `report_top` text, but `TrackConsumersPool::metrics()` returns structured `MemoryConsumerMetrics{name, can_spill, reserved, peak}` (Interface-checked 2026-10-15, `datafusion-execution-55.1.0/src/memory_pool/pool.rs:358,477`) | F04, F07; RC02 |
| G8 Library leverage | **fail (revise)** | Text written by `fmt` is reparsed by a bespoke regex, while `tracing-subscriber`'s `json` layer exists. Structured pool metrics are unused. The SDK's per-statement server time is received and discarded (`third_party/surrealdb/src/engine/remote/grpc.rs:1058-1065`). Nextest `status-level` hides SLOW lines | F02, F04, F08 |
| CI-G1 / CI-G2 / CI-G3 | n.a. | No served code-intelligence claim or evaluation is in scope. The analogous unknown≠absent obligations for diagnostics are judged under DP-02/G2/G5 | — |

## 7. Findings

### Index

| ID | Finding | Priority |
|---|---|---|
| [F01](#F01) | Operational observation has no owned model; cancellation is a sentinel error and outcome distinctions collapse | P1 |
| [F02](#F02) | Observations travel as human stderr text inside the test runner's capture: no live view, caps, prose parsing, lost identities | P1 |
| [F03](#F03) | Event delivery depends on per-spawn-site dispatcher propagation; events are silently lost | P2 |
| [F04](#F04) | Instrumentation follows await seams: per-call volume where cheap, silence where time goes; no progress, aggregates or efficiency counters | P1 |
| [F05](#F05) | The deadline and cancellation are invisible as such: no interrupt snapshot, signal time or interrupted state; the grace kills the drain | P1 |
| [F06](#F06) | Run-owner cancellation protocol decisions are not observable (`8a3347`) | P2 |
| [F07](#F07) | Readers drop, duplicate and mislabel observations; the coverage of an answer is not stated | P2 |
| [F08](#F08) | SurrealDB server-side signals are unused, and per-statement server time is discarded | P2 |
| [F09](#F09) | No least-privilege, consumer-neutral read route to live native control state; generic DB MCP is unsuited as the diagnostic surface | P3 |

P1 means it blocks explaining the next V01–V03 timeout. P2 is a material diagnostic gap. P3 is
an improvement.

<a id="F01"></a>
### F01: Operational observation has no owned model; cancellation is a sentinel

**Principles:** FP-04, DP-02, DP-19, DP-21 · A2 · G1, G2.

**Evidence.**

- **Three cancellation encodings.**
  - `ModelError::Invalid("compilation cancelled")` is returned at
    [`admission.rs:69`](../../../crates/lctx-compiler-store/src/admission.rs),
    `workspace.rs:1629` and `native_calls.rs:118`.
  - `Invalid("native transaction retry cancelled")` is returned at `control/retry.rs:174,214`.
  - `Cause(io::Error{Interrupted, "native read delivery cancelled"})` is returned at
    `lctx-native/src/prepared.rs:14`, `lctx-publisher/src/owned_read.rs:14` and
    `selected_backup.rs:860`.
- **How they are read.** `FailureDiagnostic`'s `model_cause` maps `Invalid` to
  (Model, `Invalid`) and the `Cause(io)` chain to (Model, `Context`) + (Opaque, `Opaque`)
  (`diagnostic.rs:175-210`). `Phase::finish_result` maps any `Err` to `failed` (phase.rs:187).
  `Terminal::Cancelled` exists but is used only in a test seam (`stage_runtime.rs:72`).
- **Re-inspected `12d184`.** The recursive `evaluate_base` closed `status="failed"
  elapsed_ms=410446` at 16:04:40.00, about at the 900 s SIGTERM. The test panicked with the
  `Interrupted` cause.
- **Phase lacks:** a parent, a kind (stage/wait/kernel), domain identities as fields (they live in
  spans), progress, an interrupted state and non-inclusive time.

**Consequence.** A deadline-driven cancellation is reported as an *invalid model* or opaque failure,
violating DP-21's required distinction between invalid model, infrastructure failure and
inconclusive check. Readers cannot answer D5/D9 and cannot tell slow-but-progressing from stalled
(D1). For example, I3 was still progressing at its deadline (C), but nothing distinguishes that run
from I1, which was still in Base. Any new consumer must re-derive cancellation from strings; the
string-matching test at `normalize/pipeline.rs:140` already does.

**Correction (intended behavior and boundary).** The responsibility splits in two.

*Typed cancellation (in `lctx-model`).*

- `ModelError` gains one typed cancellation outcome carrying a cause: process signal, operator
  request, owner failure, or deadline.
- `DiagnosticCode` gains an appended `Cancelled`; the codebook stays append-only.
- All sites above return it. Consumers, the string-matching test included, match the type.

*Observation vocabulary (owned once in `lctx_native::phase`, serialized with a schema version).*

- **Interval:** kind (stage / phase / wait / kernel / native_call), name, id, parent id, and the
  domain identities that apply (compilation attempt, producer, native attempt).
- **Terminal:** `passed`, `failed`, `returned`, `cancelled` (with cause and observed time), plus
  `interrupted`. Only the interrupt snapshot writes `interrupted` (F05). An owner never claims it,
  so it is not a fabricated terminal.
- **Progress:** unit, done, optional total.
- **Stage aggregate** (F04).
- **Coverage marker:** journal open and close.

*Boundaries.* `Phase::finish_result` distinguishes the typed cancellation. Readers interpret only
the versioned vocabulary. Finality remains with `Completion` and the receipts: an observation never
settles native state.

**Challenge.** Could a typed `Cancelled` hide committed effects? No. The operation outcome becomes
`Cancelled` while the Completion's committed, Unknown and storage axes stay untouched. That is the
existing separation, made truthful.

Could `interrupted` mask a real failure? Only if an owner wrote it. Owners cannot; only the
snapshot can, and a later owner terminal supersedes it.

**Closure evidence.** A SIGTERM'd pure control (no database) shows:

- typed cancellation through all three former encodings;
- `DiagnosticCode::Cancelled` in the projection;
- phase terminals `cancelled`;
- open intervals `interrupted` in the snapshot.

The reader refuses an unknown vocabulary version.

**Disposition:** unscheduled. Plan creation should assign it, likely under the Catalog coordinator
§6 as an extension of T03 (RC02).

<a id="F02"></a>
### F02: Observations travel as human stderr inside the test runner's capture

**Principles:** FP-02, FP-07, DP-13, DP-21, H16, H26 · A1, A4 · G5, G8.

**Evidence.**

- [`logging.rs`](../../../crates/cpg-extract/src/logging.rs) builds a `fmt` stderr subscriber;
  `tracing-subscriber` has only `env-filter` (Cargo.toml:117; no `json` in the lock).
- Nextest stores output only at test end. libtest-json-plus emits no output and no timeout output
  (B §4). `.config/nextest.toml:3` sets `status-level = "fail"`. The user record cap is 10 MB
  (`~/.config/nextest/config.toml`, read 2026-10-15).
- `bd6eb5` line 9437 contains the `[truncated 26805625 bytes]` marker.
  `runtime_adapter.py:10,45,58` parses with a regex and filters, and its JSON branch is never fed.
- Spans carry the attempt and producer identities; the text path keeps only event fields.
- Each `12d184` line repeats up to three 64-hex attempts in its span prefix.

**Consequence.**

- D1–D5 are unanswerable live. Post-run, a cap removes the middle and fabricates an open phase.
- Identity recovery needs a structured format that is never produced.
- The carrier substitution scenario propagates into Python readers (A1).
- Volume and repeated prefixes amplify storage on a 98 %-full disk (A4).

**Correction (Proposed).** Each long-running process writes its own append-only JSONL journal in an
owner-supplied directory. The pattern generalizes the drain receipts' `LCTX_NATIVE_DRAIN_DIRECTORY`
discipline: an absolute canonical directory, file name `pid-start_ticks.jsonl`, create-new.

*Header (written once):*

- vocabulary version;
- process identity;
- the identities that apply: `NEXTEST_RUN_ID`, `NEXTEST_ATTEMPT_ID`, `NEXTEST_EXECUTION_MODE`,
  run id, capture invocation, attachment, campaign;
- binary path.

*Body:* coarse interval begin/terminal records, heartbeat snapshots, stage aggregates, signal and
interrupt records, and a close marker on orderly exit.

*Write discipline:*

- Writes are direct `write_all` on an `O_APPEND` file. B §1 showed this keeps every event through
  SIGTERM and abort, whereas the non-blocking appender lost its tail even on normal exit with a
  slow sink.
- Volume follows the envelope (§1.1), so blocking writes are cheap.
- Use `tracing-subscriber`'s `json` layer with a per-layer filter for the observation target. B §2
  showed per-layer filtering works.

*Who owns what:*

- The run, fixture or capture owner that creates the directory records it in its own record, so
  discovery is complete.
- Stderr stays the human channel at WARN.
- The projection reads journals natively.

**Challenge.**

- *What if the directory is absent, for a bare `cargo test`?* The process emits no journal and
  behaves exactly as today. Readers report "not captured", not "no events".
- *Is the directory an ambient input that could change a fact?* It changes observations only
  (CI-10 analogue, G4).
- *What about concurrent journeys in one process?* That is not process-per-test. The header records
  the execution mode and attribution falls back to interval identities; this is a declared limit.

**Closure evidence.**

- During a running control, a reader shows the open stack and the latest heartbeat.
- After `kill -9`, the journal is readable up to its last heartbeat and lacks a close marker, and
  the reader says "killed before close".
- Identities appear as fields, with no regex.

**Disposition:** unscheduled (RC01, RC02).

<a id="F03"></a>
### F03: Event delivery depends on per-spawn-site dispatcher propagation

**Principles:** FP-05, DP-21 · A1 · G2.

**Evidence.**

- `logging::dispatch()` deliberately avoids the process-global subscriber; tests attach it with
  `WithSubscriber` (`scoped_summary.rs:46-57`).
- About 66 `tokio::spawn`/`runtime.spawn`/`spawn_blocking` sites exist in `cpg-core`,
  `lctx-compiler-store` and `lctx-native`. Only a handful use `.with_current_subscriber()`.
- B confirmed that a scoped `WithSubscriber` does not follow spawned or DataFusion tasks, and that
  `LogTracer` is absent under scoped dispatch.
- `reader.rs:3349` spawns the cancelled-read drain, which loses its error event.
- C's G5: the retry cancellation terminal in I3 is missing, consistent with a different dispatcher.
- `Phase` alone captures its dispatcher; macro events use the ambient one.

**Consequence.** Whether an observation exists depends on an undocumented obligation at every spawn
site. Loss is silent, which is unknown read as absent at the source. Each new spawn site is a
latent loss.

**Correction.** In processes that host one journey (nextest process-per-test, the CLI), install one
process-level subscriber, with the journal layer from F02. Attribution comes from interval
identities and spans, not from dispatcher scoping. Keep scoped dispatch only where a process hosts
concurrent independent journeys (serving), with one spawn helper that propagates.

An alternative fits equally well: the journal writer can be a process-global, dispatcher-independent
observer that `Phase` and the registry write to directly. That removes the dependence for
observations without changing human logging. Choose during planning; either closes the finding.

**Challenge.** Would mixing two tests' events become possible? Not under process-per-test. Under
plain `cargo test` threads it already happens on stderr; the header's execution mode declares it.

**Closure evidence.** An event emitted inside a freshly spawned task without per-site code reaches
the journal (pure control).

<a id="F04"></a>
### F04: Instrumentation follows await seams, not diagnostic units

**Principles:** FP-07, DP-21, DP-12, DP-15, H6, H10, H26 · A3, A4 · G7, G8.

**Evidence.**

- *Per-call volume where it is cheap:* `phase::wait` pairs per Pending; `base_selected_input` and
  `_eof` per root per relation (`base_scope.rs:159-172`); `completed_input_binding`
  (`workspace.rs:2206`).
- *Silence where the time goes:*
  - Base `load`/`consume_inventory!` reads (`semantic_execution.rs:222-282`);
  - inline `evaluate_all_produced` (line 395) and `evaluate_expression_scoped` (line 450), with
    only a per-root cancellation check (line 443);
  - Summary's `produce_prepared_scheduled` in one DEBUG `borrowed_cpu`
    (`semantic_summaries.rs:245`).
- *`lctx-model` has neither `tracing` nor a cancellation concept.*
- *G-07 was half-applied:* per-root `borrowed_cpu` moved to DEBUG without the per-stage aggregate
  its alternative proposed
  ([G-07](../evidence/2026-10-11_efficiency-alignment-codebase/assurance-identity-and-observation.md)).
- *Efficiency counters are off or absent:*
  - reuse disposition is DEBUG only (`compilation.rs:788-793`);
  - reuse counters are `#[cfg(test)]`;
  - `stage_peak` defaults to `None` (`resources.rs:27-29`);
  - DataFusion structured pool metrics are unused, and T03 misstates their availability (G7).

**Consequence.** I1's 410 s and I3's Base stretch are unlocalizable. Volume scales with roots ×
relations; D1, D2, D8 and D3 have no answer; skew among roots is invisible. Each new kernel needs
bespoke calls and still yields no progress (A3).

**Correction (Proposed).**

1. **A process observation registry.** Open intervals and per-stage counters are cheap atomics plus
   a short registry update on begin and finish. Fine-grained waits are counted (count, total, max)
   per (stage, wait kind) rather than emitted. A wait still open at heartbeat time, or longer than
   a threshold, is reported individually.
2. **An observer thread.** It is a std OS thread, not a tokio task, so it still runs when the only
   current-thread runtime is blocked in a kernel. At a fixed interval (5–10 s) it writes a
   heartbeat: open stack with elapsed, progress, counters, budget gauges, and DataFusion
   `metrics()` top-k.
3. **Stage aggregates at each stage terminal**, owned by the stage runner:
   - attributed time, split into kernel CPU per producer (`borrowed_cpu`, which also routes Base's
     inline kernels), waits per kind and native-call server versus client time (F08);
   - an explicit unattributed remainder;
   - counts: roots, rows, statements, retries/conflicts, reuse hits and misses with reason;
   - `stage_peak` (implement `reset_stage_peak`/`stage_peak` for `FixedPool`/`ScopedPool`);
   - the slowest-k roots.
4. **A model-owned kernel probe:** a small value with atomic progress units and a cancellation
   flag. Long kernels take it as a declared parameter and return the typed cancellation of F01 with
   no partial output. `lctx-model` gains no `tracing` dependency.

**Challenge.**

- *Aggregation hides an outlier root?* The slowest-k and max fields keep skew visible (H10).
- *Removing per-root INFO loses detail investigators used?* That detail stays at DEBUG, opt-in; the
  aggregates carry row totals.
- *The probe changes kernel results?* Only through the declared cancel, with no partial output
  (DP-12, DP-19).
- *Registry contention?* The observer uses `try_lock`, reports "registry busy" and never blocks
  execution.

**Closure evidence.**

- A Base/Summary control yields per-stage attributed and unattributed time, progress ordinals and
  slowest roots.
- Journal line count for a full control is bounded by stages + time/interval, not roots.
- Structured pool top-k appears in a heartbeat.

<a id="F05"></a>
### F05: The deadline and cancellation are invisible as such

**Principles:** DP-19, DP-21, H24 · A2 · G5.

**Evidence.**

- The signal owner converts SIGTERM into callbacks (`signal.rs:30-60`). The receipt records
  `signal` with no time (`signal.rs:248`).
- `12d184`: `TERMINATING [>900.000s]`, then `TIMEOUT [910.085s]`. The drain continued until it was
  killed at the +10 s default grace (C), and the receipts stayed "started".
- No stacks, `/proc` state or open-stack record exists anywhere (C, Q9).
- The runner reports a timeout as `failed` with `reason ""` (`catalog_completion_validate.py`
  outcome from exit, cleanup and termination only).
- The signal listener is a tokio task. Under the current-thread runtime of the Summary controls it
  runs only when the kernel yields.

**Consequence.**

- D5 is unanswerable: what was open, what made the last progress, and when each owner saw the
  cancel.
- Native finality after a timeout is Unknown more often than necessary, because the drain is
  SIGKILLed.
- Timeouts look like ordinary failures to every reader.

**Correction (Proposed).**

- **Snapshot at the first signal.** On the first SIGINT or SIGTERM, an OS-level path writes an
  interrupt record to the journal: signal kind, wall time, open stack with elapsed, last progress
  per stage, budgets and registered cancellation owners. Use the observer thread, woken by a
  signal-safe flag, or the signal owner. Do not depend on a tokio poll.
- **Cancellation observations.** Each owner records when it observed the cancellation and its later
  terminal (`cancelled` after drain).
- **Stacks.**
  - *On demand, live:* `just runs snapshot RUN` runs `gdb -batch thread apply all bt` plus
    `/proc/<pid>/task/*/{stat,wchan,syscall}` for the run's test processes and campaign daemon.
    B §5 found this works on release binaries (symbols, no line tables) with `ptrace_scope=0`.
  - *Automatically during grace*, if probe P1 qualifies it.
- **Grace period.** Size nextest's `grace-period` for the long overrides to drain plus snapshot
  (RC06). Allowances stay unchanged.
- **Runner classification.** Classify nextest `TIMEOUT` and JUnit `type="test timeout"` as a
  distinct timeout outcome with the configured allowance.

**Challenge.**

- *Does a longer grace hide hangs?* No. The interrupt snapshot is written at SIGTERM, and SIGKILL
  still follows.
- *Does gdb attach perturb draining?* That is why automatic capture waits on P1; on demand it is
  the agent's explicit choice.

**Closure evidence.** A synthetic long control terminated by nextest shows:

- an interrupt record with the open stack;
- per-owner cancellation times;
- drain completion within grace, with receipts "complete";
- the runner reporting `timeout`.

<a id="F06"></a>
### F06: Run-owner cancellation protocol decisions are not observable

**Principles:** DP-19, DP-21 · A2 · G5.

**Evidence.**

- `8a3347`'s `cancel.json` requested a stop at 18:26:36. All delegated `recovery.json` files say
  `requested: 18:29:10`, `members: []`; the record ends at 18:29:10 as `cancelled`.
- `runs.py` `_own` (lines 630-650) waits while `request_stop` reports delegated status `active`
  before signalling its own child.
- `request_stop` (`run_delegation.py:336-430`) skips entries in several cases: the owner is itself,
  the recovery lock is held, or the owner is not alive with its lock held. It still returns
  `observe(…)` status.
- The record keeps final state only.

**Consequence.** It cannot be established whether the cancel was undelivered (a protocol defect) or
delivered and slow to drain.

*Hypothesis (Proposed, unverified):* `observe` reports `active` for an entry that `request_stop`
skips, so the owner waits without delivering any stop until the child exits on its own.

A concurrent efficiency review's evidence folder
(`docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/delegated-stop-probe.py`)
contains a branch probe of `request_stop` selection. It was not consulted as evidence here, and may
settle this hypothesis.

**Correction.**

- The run owner appends lifecycle events to `build/runs/<id>/events.jsonl`:
  - cancel observed;
  - each `request_stop` decision per entry, including skipped entries with their reason;
  - signals sent;
  - child and delegated exits.
- `just runs status` shows the stop protocol state.

This is tiny volume, owner-written and append-only. `record.json` semantics are unchanged.

**Closure evidence.** Probe P2, or an equivalent control, shows the decisions. If the hypothesis is
confirmed, the protocol defect becomes a separate correctness finding for the harness owner.

<a id="F07"></a>
### F07: Readers drop, duplicate and mislabel observations

**Principles:** DP-01, DP-02, DP-12, H26 · A1 · G1, G5, G6.

**Evidence.**

- *Dropping:* `runtime_adapter.py:45` drops events without a matching `invocation_id` (every
  uncaptured run; I1, I2 and I4a have none), and line 58 drops non-phase observations. I3 lost
  20,989 lines, including retries and eof/binding counts.
- *Duplicating:* `projection.py:186-189` projects runtime rows from both the JUnit records and
  `native.log`. I3 has 101,614 rows for about 50.8k events.
- *Mislabelling:* nextest truncation is "unknown" (`nextest.py:319,428,502`) despite the marker.
  Every row is `outcome: observed`, so failed phases never surface in `cases --outcome failed`.
- *Filtering:* `events` filters only by step (`cli.py:390,720`).
- *Joining:* answering D10 needs four locations (C, G8). Only the Catalog runner's `summary.json`
  joins them (a strength to generalize).

**Consequence.**

- Counts, absence and stalls are misread.
- Agents fall back to raw grep and manual alignment across time zones (C: effort H for most
  questions).
- Storage amplifies on a 98 %-full disk.

**Correction.**

- **Source:** the projection consumes journals as its only phase source, de-duplicated by
  `(pid, start_ticks, seq)`.
- **Coverage:** state it per process — complete, truncated (bytes, from the marker), no close
  marker, dropped with counts by reason, or not captured.
- **Readers:** expose `open`, `interrupted`, `cancelled` and `failed` intervals, plus `--phase`,
  `--status` and `--since` filters, and a per-run timeline that joins:
  - the run lifecycle events (F06);
  - invocation;
  - delegated recovery;
  - attachment receipts;
  - journals.
- **Live:** `just runs status RUN` folds the latest heartbeat per process.
- **Format:** emit timestamps in UTC throughout.

**Challenge.** Does dropping the JUnit-derived phase rows lose anything? No. JUnit remains the
test-result original under ADR-0147, while phase observations come from their own original.

<a id="F08"></a>
### F08: SurrealDB server-side signals are unused

**Principles:** DP-13, DP-21, H4, H26 · A4 · G8.

**Evidence.**

- *Campaign servers:* they run `--log=warn` into journald with no slow-log flag
  (`surrealdb_service.py:647-653`), and journald is not retained per run.
- *What the server offers* (A, Tested 2026-10-15 on the private memory engine):
  - The slow log prints a WARN line with normalized SurrealQL and referenced params (deny-list
    honoured), including statements ended by timeout (E1–E4). It is therefore visible at
    `--log=warn` once enabled.
  - Root `/metrics` exposes statement, transaction, KV and conflict families labelled by
    statement_type/outcome/ns/db/user (E5).
  - gRPC `QueryStats.execution_duration` arrives per statement (E11), and the vendored SDK maps it
    into per-statement `time` (`grpc.rs:1058-1065`), reachable through `Query::with_stats`
    (`method/query.rs:547`). No project code reads it.
  - `EXPLAIN ANALYZE` gives per-operator metrics for SELECT; it is refused for UPDATE without
    executing (E13).
- *What the server does not offer:* no running-statement view (E12, source search). OTLP with
  `traceparent` would need an SDK change and a collector, and executor spans lack statement text
  in Community (E8, E9).

**Consequence.** D4 and the server-versus-client part of D8 are unanswerable. A DB stall cannot be
told apart from client CPU or client waits.

**Correction (Proposed, in order of value per cost).**

1. **Client side.** Record per native call: stage, query digest, statement count, rows,
   per-statement server `execution_time` and client wall time, into the stage aggregates (F04) and
   the heartbeat's open-call entries. This is attributable per process and test, and needs no
   server change. *Interface-checked* for non-streaming responses. Whether the streaming read path
   exposes per-query stats needs a source check; A's E11 shows `QueryEnd` carries the whole-query
   duration.
2. **Campaign server.** Enable `--slow-log-threshold` (for example 1 s) with a default-deny
   parameter list, and write the server log to a per-campaign file. The fixture or service owner
   collects it into the run's evidence before whole-target discard (RC05).
3. **Root `/metrics` scrape** by the fixture owner at campaign start and end, plus optionally on a
   coarse interval. Diffs give per-campaign statement, KV and conflict totals. There is no
   collector.
4. **`EXPLAIN ANALYZE`** of a suspect SELECT on a disposable campaign, on demand, under C09.
5. **OTLP plus `traceparent`: deferred.** Revisit if (1)–(3) cannot attribute a recurring stall.
   The A/B evidence shows export loss at kill and an SDK change are required.

**Challenge.**

- *Concurrent tests on one campaign?* Server metrics are campaign totals. Attribution is
  client-side, so the remedy's primary signal is the right one.
- *Slow-log secrets?* Default-deny parameters; owned disposable campaigns only.

**Closure evidence.** For one control, stage aggregates report server execution time against
client wall time per stage, and the run retains the campaign slow log and the `/metrics` start and
end diff.

<a id="F09"></a>
### F09: No least-privilege, consumer-neutral live read route to native control state

**Principles:** FP-02, DP-01, DP-05, H25 · A3.

**Evidence.**

- *The existing MCP route* (`surrealdb_fixture.py:1671-1700`; `docs/surrealdb.md` maintenance
  diagnostics) is maintenance-only, synthetic validation only, and launches `codex` specifically.
  An earlier review noted that admin credentials exceed the header scope.
- *A, Tested 2026-10-15:* VIEWER `CREATE`/`UPDATE`/`DELETE` return `ok []` silently (E18).
  Results truncate at 256 KiB (E16). Per-call audit is INFO, invisible at `--log=warn` (E20). The
  stateless 2026-07-28 protocol works (E17). A concurrent read during a long write returned
  committed state in 0.2 ms (E14, memory engine only).
- *Control tables* are finality authority, without timestamps or stage columns.
- *`inspect_campaign`* honestly reports native state as `not_inspected`
  (`surrealdb_development.py:410-440`).

**Consequence.** During a stall, "which attempts or effects are live or unresolved" is not readable
by either agent without maintenance. Ad hoc SurrealQL over control tables would reinterpret
finality semantics outside their owner.

**Correction.**

- **Primary:** a domain inspection (`lctx store inspect` or `just fixture --inspect-native`) reads
  the control tables of an owned campaign read-only with a campaign-scoped VIEWER and interprets
  them once. It is usable live and works for both agents through the shell.
- **Optional:** the SurrealDB HTTP `/mcp` with that same VIEWER for owned disposable campaigns
  only (never shared or operator targets). It is configured consumer-neutrally (Claude Code `.mcp`
  HTTP or Codex), and its documented limits are silent writes, truncation and audit at INFO.
- **Never:** do not write observations into the database or the control tables (DP-05; see §9).

**Challenge.** Does a live read perturb a RocksDB run? This is unverified (P3). The domain
inspection uses short snapshot reads with existing timeouts. Keep it on demand.

## 8. Synthesis: how the corrections fit

The findings share one missing contract: an **owned operational-observation model with its own
original** (F01 + F02). Everything else either feeds it or reads it:

- F04's registry, aggregates and probe, and F05's interrupt snapshot, *produce* records.
- F03 ensures they *arrive*.
- F07's readers and the F06 run events *serve* them.
- F08 adds client-side server time to the same aggregates. Its server-side files are separate
  originals held by the run owner.
- F09 is independent.

The corrections are compatible. None duplicates an owner, and none moves finality: the
receipts, `Completion` and control tables stay authoritative for native state. The journal is
observation only.

**Priority vs prerequisite.**

- Consequence priority is F01, F02, F04 and F05, together. These are what make the next V01–V03
  timeout explainable.
- Prerequisite order:
  1. typed cancellation and the minimal vocabulary (F01);
  2. the journal carrier (F02), with F03 resolved by either option;
  3. the registry/observer, heartbeat and interrupt snapshot (F04 core, F05);
  4. stage aggregates and the kernel probe (F04 remainder);
  5. readers (F07).
- F06 and F08 (client side, then server flags) are independent and can proceed in parallel. F09
  comes last.
- A lower-priority piece can enable an urgent one: F07's journal reader is less urgent than F05,
  but F05's value to agents arrives only through it.

**Strengths to preserve:**

- Phase never fabricates a terminal.
- Finality lives in `Completion` and the receipts.
- `FailureDiagnostic` is bounded and secret-free.
- Retry observations use query digests and log₂-bounded conflict logging.
- Stage phases take their names from declarations.
- Capture health is kept separate from command exit.
- The runner's per-step native-finality join.

**Foundation verdicts.**

| Foundation | Verdict |
|---|---|
| FP-01 | Violated: observation meaning is split between Rust emission and a Python regex |
| FP-02 | Violated: readers depend on the `fmt` presentation format |
| FP-03 | Violated: a new long operation cannot compose progress or aggregates |
| FP-04 | Violated (F01) |
| FP-05 | Violated: hidden dispatch obligation; deadline lifecycle implicit |
| FP-06 | Mixed: diagnostics do not change execution (satisfied), but a timeout cannot be reasoned about locally (violated) |
| FP-07 | Violated (F04) |

| Supporting rule | Verdict |
|---|---|
| DP-02, DP-19, DP-21 | Violated |
| DP-12 | Violated at the reader (truncation) |
| DP-05, DP-18 | Satisfied (strengths) |
| DP-13/14 | Violated (G8) |
| DP-15 | Violated, narrowly (DataFusion qualification) |
| DP-22 | Satisfied: plans make no timeout-cause claims |

## 9. Library fit and alternatives

### Library fit

| Capability | Candidates (resolved versions) | Fit and gaps | Choice |
|---|---|---|---|
| Structured process journal | `tracing-subscriber` 0.3.23 `json` + per-layer filter; direct `O_APPEND` file writer; `tracing-appender` 0.2.5 non-blocking | The direct writer kept all events through SIGTERM and abort (B §1). The non-blocking writer loses its tail even on normal exit with a slow sink (bounded 100 ms + 1,000 ms guard wait) | Adopt `json` + direct writer, or a dispatcher-independent writer (F03 option); reject non-blocking for the journal |
| Span timing | `FmtSpan::CLOSE` | Inclusive, string-typed, emitted only on close (B §3) | Reject as attribution; use the registry and aggregates |
| Distributed tracing | `tracing-opentelemetry` / `opentelemetry-otlp`; server OTLP + W3C `traceparent` | Strong parenting, but needs a collector, an SDK patch, and loses open or queued spans at kill (B; A E8–E10) | Defer (F08 trigger) |
| DataFusion operator detail | `datafusion-tracing` 55.0.0 | Per-operator spans at volume; process-global `JoinSetTracer` | Defer; on-demand only |
| Budget gauges | DataFusion 55.1 `TrackConsumersPool::metrics()` | Structured `{name, can_spill, reserved, peak}`; reservations, not RSS | Adopt (correct T03) |
| Runtime state | tokio taskdump / console / metrics | Taskdump needs `tokio_unstable` and may not complete with a blocked worker; console is infeasible under the release TRACE ceiling (B) | Reject; use the OS observer thread |
| Test runner | nextest 0.9.146: `status-level`, `grace-period`, SIGUSR1 dump, `NEXTEST_*` env, chrome-trace export (already used) | No pre-timeout hook; SIGTERM to the process group; 10 s default grace (B §4) | Adopt `status-level=slow` and grace overrides; header identities from env |
| Stacks / thread state | gdb batch, `/proc`, perf (existing `compile-profile` observer) | gdb works on release binaries with symbols only; perf is on-CPU only (B §5) | Adopt on demand; automatic pending P1 |
| Server signals | SurrealDB 3.3.0 slow log, root `/metrics`, `QueryStats`, `EXPLAIN ANALYZE`, `INFO FOR ROOT` | See F08; no in-flight view | Adopt slow log, scrape and client stats; `INFO FOR ROOT` unnecessary (`/proc` of the daemon is cheaper) |
| Agent DB access | Built-in HTTP `/mcp`; standalone SurrealMCP | Built-in: VIEWER silent writes, 256 KiB / 60 s. Standalone: retired, last tag on SDK 2.3.7 (A) | Optional built-in only; reject standalone |

### Alternatives

| Alternative | Change propagation | Authority and composition | Machinery and operational risk | Decision / revisit |
|---|---|---|---|---|
| Current baseline | Carrier change hits readers | Vocabulary duplicated; identities lost | Low machinery; no live view; truncation; 2× storage | Rejected (F01–F07) |
| **Proposed:** typed vocabulary, per-process journal, observer, aggregates, probe, snapshot; server slow log, scrape and client stats; CLI readers | Sink substitution local | One vocabulary; finality untouched | Modest: one observer thread, one file per process, small harness additions | **Recommended** |
| OpenTelemetry end to end (client, server, collector) | Local, via the standard | Strong cross-process parenting | Collector lifecycle under storage ownership; SDK patch for gRPC metadata; span loss at kill; agents still need a query layer | Defer; revisit when client stats cannot attribute a recurring stall |
| SurrealDB as sink (diagnostic namespace in the campaign, or the control tables plus live select) | Live SurrealQL / MCP queries | Mixes observation with authority (DP-05) | Perturbs the workload (conflicts already frequent: I1 804 conflicts / 506 commits), fails when the DB is the problem, is discarded with the campaign, and is subject to 256 KiB / 60 s MCP limits | Reject as primary. Revisit only for a cross-run analytical consumer, by ingesting journals post-run into a *separate* disposable instance |
| Simplest viable: JSON file layer, `status-level=slow`, grace, typed cancel | Partly local | Partial vocabulary | Minimal | Necessary but insufficient: silent stretches stay silent and volume stays per-call. Use it as the first increment of the proposed design |

## 10. Verification and uncertainty

| Claim | Label / date | Basis | Gap or settling evidence |
|---|---|---|---|
| Cancellation encodings and their projection | Implemented, inspected 2026-10-15 | Source cited in F01 | — |
| `12d184` silent 410 s; `failed` at SIGTERM with `Interrupted`; TIMEOUT 910 s | Historical receipt 2026-10-15, re-inspected | `build/runs/20261015T154938.160Z-12d184/output.log` lines 1–12, 8082–8090 | Cause of the 410 s remains unknown (that is F04) |
| `bd6eb5` truncation marker; 21,982 phase lines | Historical receipt, re-inspected | `output.log:9437`; `grep -c` | — |
| `8a3347` cancel at 18:26:36 with no stop request until 18:29:10 | Historical receipt, re-inspected | `cancel.json`, `record.json`, `delegations/*/recovery.json` | Cause: P2 |
| Direct writer survives SIGTERM/abort; non-blocking loses its tail | Tested by B, 2026-10-15 (synthetic) | B §1 | Workspace binaries not measured |
| Server slow log, metrics, `QueryStats`, MCP behavior | Tested by A, 2026-10-15 (private memory engine) | A E1–E20 | RocksDB not tested (P3) |
| DataFusion structured pool metrics | Interface-checked 2026-10-15 | Registry source 55.1.0 | — |
| SDK per-statement stats reachable | Interface-checked 2026-10-15 | Vendored SDK | Streaming path unconfirmed (source check) |
| Proposed design volume and perturbation | Proposed | Envelope §1.1 | Measure once implemented; no speed claim |

**Probe requests** for the coordinator to arrange (private, owned processes only):

- **P1, automatic stacks during grace.** The question: in a workspace release test binary running a
  current-thread tokio runtime blocked in a synchronous kernel, can the process's SIGTERM path
  spawn `gdb -batch -p <self> -ex 'thread apply all bt'` (or a sibling observer do so)? Does it
  complete within a 10 s grace without preventing the drain?
  - *Decisive:* completes in under about 3 s with all threads, and the drain proceeds → adopt
    automatic capture.
  - *Otherwise:* on demand only.
- **P2, run cancel delivery.** The question: does a managed run with a synthetic delegated child
  (no database) deliver a stop within one poll after `cancel.json`, or does `_own` wait on
  `active` with no recovery request written?
  - *Decisive:* a missing `recovery.json` at cancel time reproduces `8a3347` and turns F06's
    hypothesis into a harness correctness finding.
  - The concurrent `delegated-stop-probe.py` may already answer this.
- **P3 (low priority; needs authorization for a private RocksDB instance).** The question: does a
  VIEWER read during a long write transaction delay or conflict with the writer? It affects only
  whether F09's live inspection is safe.

**Uncertainty connected to decisions.**

- None of P1–P3 can change the overall Revise or F01–F05. P1 changes only whether stacks are
  automatic; P2 changes F06's classification; P3 changes F09's live-use advice.
- The possible JUnit-redirect freshness defect in the runner (coordinator exploration) was not
  examined. It is unexamined breadth, not a finding.
- Nextest record stores were not decoded.

## 11. Authority changes and dispositions

Disposition: findings F01–F09 are open and unscheduled. Plan creation assigns them; the likely
owner is the Catalog coordinator §6 through an extended T03. No finding is deferred by this review.

**Rule impacts.** These are proposed for operator confirmation; the review does not change these
rules.

<a id="RC01"></a>
**RC01: DESIGN §1.2 (native test evidence) and [ADR-0147](../../adr/0147-native-test-evidence.md).**

- *Change:* add product-emitted per-process observation journals as a native original, created in
  an owner-supplied directory and discovered from that owner's record. Phase, progress and
  interrupt observations come from journals. Captured stderr and JUnit keep the human and
  test-result channels, and the projection stops recovering phases from text.
- *Depends:* F02, F07.
- *If kept:* journals can still be added as an extra original, but readers keep parsing text, the
  double projection persists, and F07 narrows to text-path fixes.

<a id="RC02"></a>
**RC02: Development plan T03 (`catalog-completion-development-plan_2026-10-14.md`, T03).**

- *Change:*
  - extend T03 from "waits/refusals at existing seams" to the §1.1 requirement: vocabulary,
    journal, heartbeat/progress, stage aggregates, interrupt snapshot;
  - correct the DataFusion 55.1 clause, since structured `metrics()` exists. Use it for budget
    gauges, still not RSS.
- *Unchanged:* O02's "no per-row event stream or universal wait graph" stays and is consistent.
- *Depends:* F01–F05.
- *If kept:* D1, D2, D5 and D8 stay unanswerable and the findings stand unscheduled.

<a id="RC03"></a>
**RC03: Development plan §5 C10 (MCP/telemetry/Cloud).**

- *Change:* make campaign servers' slow log and a retained per-campaign server log default-on, and
  have the fixture owner scrape root `/metrics` at campaign boundaries. Both are owned signal
  consumers with no collector. C10's own trigger ("a named question is unavailable from
  plans/receipts/logs") is met by D4 and D8.
- *Unchanged:* still no default generic DB MCP, collector or Cloud; still "metrics cannot prove
  native finality".
- *Depends:* F08.
- *If kept:* F08 reduces to client-side `QueryStats`, and D4 is answered only from the client.

<a id="RC04"></a>
**RC04: `docs/surrealdb.md` maintenance diagnostics (lines 290–307) and `surrealdb_fixture.py`
`mcp_invocation`.**

- *Change:* allow campaign-scoped VIEWER read-only inspection of owned disposable campaigns outside
  maintenance, for either agent. The documented limits are silent `[]` writes, 256 KiB truncation
  and audit at INFO. The prohibitions on shared and operator targets and on `surreal mcp` remain.
- *Depends:* F09.
- *If kept:* F09 relies on the domain inspection route only.

<a id="RC05"></a>
**RC05: Campaign service configuration (`surrealdb_service.py` `unit_text` log-filter allowlist and
launch flags).**

- *Change:* admit `--slow-log-threshold` with a default-deny parameter list, and a per-campaign
  server log file collected into run evidence before whole-target discard.
- *Depends:* F08.
- *If kept:* no server-side statement evidence; journald slices only.

<a id="RC06"></a>
**RC06: [`.config/nextest.toml`](../../../.config/nextest.toml).**

- *Change:* set `status-level = "slow"`, and give the long overrides an explicit `grace-period`
  sized to drain plus snapshot. Allowances (`terminate-after`) are unchanged.
- *Depends:* F05, F02.
- *If kept:* the snapshot must fit in 10 s, and the drain continues to be SIGKILLed after
  timeouts, so native Unknown follows each timeout.

<a id="RC07"></a>
**RC07: `cpg_extract::logging` dispatch contract** (module doc: "without changing the
process-global subscriber… `WithSubscriber` so each poll has its own attribution") and the
matching test-fixture convention.

- *Change:* use a process-level subscriber in single-journey processes, with scoped dispatch plus a
  spawn helper only for multi-journey processes. Alternatively, use a dispatcher-independent
  journal writer.
- *Depends:* F03.
- *If kept:* the journal must use the dispatcher-independent writer, and tracing events in spawned
  tasks stay at risk.

<a id="RC08"></a>
**RC08: Kernel contract, DESIGN §B3 ("pure finite model kernels") and semantic §15.**

- *Change:* clarify that kernels may take a model-owned cooperative probe (progress units plus a
  cancellation flag) as a declared input and return the typed cancellation with no partial output.
  They stay pure apart from that declared input, and gain no logging dependency.
- *Depends:* F04, F05.
- *If kept:* kernel interiors stay opaque, the heartbeat reports only "in kernel X since t", and
  cancel latency stays bounded by kernel duration.

<a id="RC09"></a>
**RC09: DESIGN §1.2 managed runs, ADR-0172 and the `runs.py` file contract** (`record.json`
final-state; `progress.json` from consumers).

- *Change:* add an owner-written append-only `events.jsonl` for stop/cancel protocol decisions, and
  have `just runs status` fold live journal progress.
- *Depends:* F06, F07.
- *If kept:* `8a3347`-type delays remain unexplainable after the run.

<a id="RC10"></a>
**RC10: Record the operational diagnostic requirement itself.** It would sit in its architectural
owner: DESIGN §1.2 (verification and runs), or the storage/verification section the operator
prefers.

- *Change:* adopt §1.1's D1–D10, the coverage rule and the proportionality envelope as the DP-21
  "separate operational requirement", so later changes and reviews judge instrumentation against
  it.
- *Depends:* all findings.
- *If kept unrecorded:* each change re-litigates H26/DP-21, and diagnostics regress toward the
  minimum.

**Unchanged rules:** C02 (no events, changefeeds or live refresh), C09 (on-demand EXPLAIN; Studio
optional; `EXPLAIN ANALYZE` needs its own authorized workload), O02 (no per-row or
universal-wait-graph stream), the codebook append-only rule, and ADR-0142's distinct outcome axes.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | **violated** | Sink substitution propagates into Python regexes and a copied terminal set. Delivery depends on per-spawn-site obligations (F02, F03, F07) |
| A2 Encode domain meaning explicitly | **violated** | Operational outcomes (cancelled / failed / interrupted / lost), time attribution and progress lack representations. Cancellation is a sentinel interpreted by consumers (F01, F05, F06). Finality and failure models are adequate strengths |
| A3 Extend through composition | **violated** | A new long kernel or stage cannot compose progress, attribution or counters from existing primitives. The run, invocation, delegation and attachment join is one-off (F04, F07) |
| A4 Fit execution to the supported workload | **violated** | Observation volume ∝ roots × relations, with repeated span prefixes, 10 MB truncation and 2× projection on a 98 %-full disk. Zero signal in the dominant stretches; predictably worse for the 3,600 s pilot. The credible route is Proposed (§4) |

**Bounded decision: Revise.** In supported scope, A1–A4 are violated and G1, G2, G5, G6, G7 and
G8 fail.

**Enclosing architecture:** operational diagnostics need revision. Product semantics, finality and
storage authorities were not reassessed, and are preserved as strengths. This acceptance would be
architectural, distinct from release qualification.

| Priority | Change and responsible component | Findings | Closure evidence or trigger |
|---|---|---|---|
| 1 | Typed cancellation and observation vocabulary (`lctx-model` errors/diagnostic codes, `lctx_native::phase`) | F01 | SIGTERM'd pure control shows typed `cancelled`/`interrupted` |
| 1 | Per-process journal, observer thread, heartbeat, interrupt snapshot (`lctx_native` with the run/fixture directory owner) | F02, F03, F05 | Live open stack; post-kill journal readable to its last heartbeat |
| 1 | Stage aggregates, kernel probe, `stage_peak`, structured pool gauges (stage runner, `borrowed_cpu`, model kernels) | F04 | Attributed and unattributed time with bounded line count |
| 2 | Journal-native projection, coverage states, filters, timeline, live `runs status` (`verify_evidence`, `runs.py`) | F07, F06 | Reader answers D1–D10 for `12d184`-shaped runs without grep |
| 2 | Client `QueryStats`; campaign slow log, retained log and `/metrics` scrape (`native_calls`, service/fixture) | F08 | Per-stage server versus client time; retained slow log |
| 3 | Domain native inspection; optional VIEWER MCP (fixture/CLI) | F09 | Live read of attempts and effects by either agent |

**Next consequential decision:** the operator confirms or declines RC01–RC10, starting with RC10
(the requirement) and RC02 (T03 scope). Plan creation then schedules F01, F02, F04 and F05 ahead
of resuming the V01 recursive Summary and linked Catalog exits. Their next timeout should then
leave its own explanation.

**Intended path:**
`docs/design_review/reviews/design_review_operation-diagnostics-target_2026-10-15.md`.

## Coordinator publication notes

Added at publication on 2026-10-15. They change no judgment, finding, priority or rule impact above.

- **Incident retrospective retained.** Assignment C's matrix and gaps are in
  [2026-10-15_operation-diagnostics-incidents](../evidence/2026-10-15_operation-diagnostics-incidents/README.md),
  with the phase-pairing parser used to produce them.
- **P2, partially settled by concurrent evidence.**
  - *What it showed.* A concurrent efficiency review's branch control,
    `docs/design_review/evidence/2026-10-15_catalog-completion-efficiency/delegated-stop-probe.py`,
    passed on 2026-10-15 with exit 0. That review's evidence is another agent's, uncommitted at
    publication time. With completed-first ordering, the actual `run_delegation.request_stop` saw one
    completed and one active registration for the same owner. It requested no signal and persisted no
    live request. With live-first ordering, it requested SIGTERM and then SIGKILL.
  - *Strength.* Signals and liveness were substituted, so this is Tested at branch strength. Real
    process drainage is `not_run`.
  - *What this means for F06.* The control is consistent with F06's hypothesis. The protocol defect
    itself belongs to that concurrent review's owner. F06's correction, observable stop-protocol
    decisions, stands independently.
- **P1 and P3: `not_run` at publication.** P1 (automatic stacks during the grace period) needs a
  synthetic blocked workspace test binary. P3 (VIEWER read perturbation on RocksDB) needs separate
  authorization for a private RocksDB instance. Per §10, neither can change the decision or F01–F05.

