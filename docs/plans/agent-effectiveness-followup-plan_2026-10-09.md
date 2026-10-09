# Agent effectiveness follow-up plan

**Unified persistence integration, Proposed, 2026-10-09:** the
[unified companion](unified-persistent-surrealdb-plan_2026-10-09.md) replaces disposable
server/database provisioning with stable service attachment and logical test attempts. Preserve
AF1/AF4 run/child/cleanup certainty, recovery and truthful observations; UP1/UP7 consume their
current working interfaces. AF6/AF7 arbitrary SQL/native MCP cannot inherit canonical-store
access: ordinary access uses the Rust exact-view boundary; privileged diagnostics use only
synthetic validation under maintenance, credentials excluded from main, and existing native-output
limits. UP7 rebases AF6/AF7/AF8 command examples and scoped outcomes. Disposable wording below
records the currently implemented baseline, not a shared-store fallback. AF findings/receipts
remain here; unified findings stay with the persisted coordinator. This addition does not claim
the concurrent AF implementation was reviewed or run by the unified-plan author.

**Status, 2026-10-09: AF0–AF9 implemented and accepted at focused scope.** This document schedules the
[follow-up review](../design_review/reviews/design_review_agent-effectiveness-followup_2026-10-09.md)
and its [command retrospective](../design_review/evidence/2026-10-09_agent-effectiveness-followup/retrospective.md).
The operator confirmed RC01–RC04 during plan preparation and selected resolution of each
capability proposal. Those decisions settle the target, not implementation or verified closure.
The [earlier workspace plan](agent-workspace-effectiveness-plan_2026-10-07.md) retains its
historical implementation receipts and other obligations. This plan's §8 is the sole current
disposition owner for follow-up F01–F05, the selected recommendations and transferred AE-24.
The [independent target review](../design_review/reviews/design_review_agent-effectiveness-plan_2026-10-09.md)
accepts the corrected target at **Proposed, scoped** strength. The source review retains its
dated Revise judgment; §8/§9 own current implementation and focused acceptance.

## 1. Outcome, scope and baseline

Make agent work easier to express, observe and recover on this personal workstation, where
Codex is the primary coding agent and Claude Code is used occasionally. Preserve bare tools,
native argument grammar, explicit environment preparation, concurrent execution and intentional
overrides. Improve existing owners before composing new diagnostic capabilities over them.
The mechanisms are **Implemented / focused-Tested, 2026-10-09**, within §9's boundaries;
no productivity or speed gain has been measured. A smaller instruction file, more wrappers
or fewer parallel workers is not a goal.

**Implemented / source-inspected baseline, 2026-10-09:** clean `main` at `2a9a3771` when
authoring began. The October 7–8 workspace work supplies requirement-scoped environment
ownership, composable verification, native disposable fixtures, durable run directories,
optional worktrees, scoped maintenance and compilation profiling. The follow-up independently
found defects in supported exception, output, concurrent-publication and outcome paths. Existing
product failures in [STATUS](../../STATUS.md) are separate; this plan neither diagnoses nor closes
native authentication, stream, admission, timeout or cache-equivalence failures.

Work includes the repository harness/help and justified Codex/user/workstation wiring. It
excludes product semantic changes, operator databases/client registrations, real-library pilots,
heldout/gold changes, wholesale dependency refresh and model selection. No scheduler, universal
workspace model, permanent MCP bridge, mandatory startup audit, new resource/thread/concurrency
cap or healthy-command timeout is introduced. Native runtime continuation remains the first
choice for ephemeral commands; durable runs serve cross-session observation and recovery.

The workload premise is one operator with several concurrent tasks, short source queries,
long builds and controls, a mutable native extension, large logs and observers that may disappear
or stop reading. The applicable [standard](../design_review/design_principles/standard.toml)
is core/template 3.3, heuristics 1.0, code-intelligence profile 1.5 and the repository binding.
The operator's exclusion of harness caps governs the DP-20 conflict documented in the source
review. Assessment is proportional to the consequential lifecycle and information boundaries.

## 2. Accepted rule changes and architectural route

All decisions below were **explicitly accepted by the operator on 2026-10-09** during plan
preparation. Do not ask again merely because execution starts. A newly discovered rule impact
still follows `create-plan`'s confirmation route.

| Source impact | Accepted consequence | Execution route, before dependent implementation |
|---|---|---|
| Review RC01 | Codex primary, limited Claude; host/user/shared-skill customization eligible. Replaces equal-runtime/repository-only scope for this follow-up. | AF0 updates the current workflow/instruction scope and links this plan; historical October 7 choices remain attributed to their date. No gratuitous Claude parity work. |
| Review RC02 | Managed discovery participates in environment ownership; readiness used for execution is observed under that ownership. Queued writers recheck after exclusive acquisition. | AF0 records the changed workflow contract through an ADR and owning DESIGN §1 workflow prose, superseding affected ADR-0134 terms if necessary. AF3 implements it. Observation remains read-only; synchronization remains explicit. |
| Review RC03 | Process exit and cleanup certainty are independent; failed/unknown cleanup remains recoverable and protected from pruning. | AF0 records the run contract/retention decision and actual consumers through the ADR/owner route; AF1 migrates them together. |
| Review RC04 | Owned full logs are authoritative; live output is detachable and best effort outside supervision. | AF0 records the presentation/supervision boundary through the ADR/owner route; AF2 replaces synchronous mirroring. |

AF0 uses the `adr` skill and the existing architectural collection, not a new policy register.
One coherent ADR may cover coupled run changes; environment publication may merit a separate
decision. Accepted ADR text remains immutable. F04/F05 and AE-24 repair existing contracts;
they do not authorize weakening blocked evidence or making freshness mutate its inputs.

## 3. Target responsibilities and contracts

### 3.1 Run ownership, recovery and stored observations

`scripts/runs.py` remains the owner of `build/runs/ID/record.json`, `output.log`, process
identity, cancellation and retention; `scripts/harness.py` supplies shared process primitives.
`verify.py` owns boundary outcomes and its separate `summary.json`. A run's process exit is
an execution observation, not a product verdict or proof that descendants are gone.

The run owner distinguishes child launch failure, a launched child's exit, supervisor/recording
failure and cleanup observation (`confirmed`, `failed`, `unknown`). Cleanup includes owned group
members after leader exit. A cleanup attempt is not confirmation. Catch ordinary exceptions after
successful spawn, including identity capture, initial record publication and progress failures;
retain the child PID/group immediately so a later fallible operation cannot precede cleanup
responsibility. Apply the same guard to `surrealdb_fixture.run_attached`. Parent-death signaling
is a backstop, not proof of process-group cleanup.

The current writer advances the run-record schema to 2 with these fields. Preserve run IDs,
paths, original child exits and retained captures. Ordinary observation leaves schema-1 records
byte-for-byte unchanged. Explicit `cancel`/recovery may atomically upgrade the one selected record
under its owner lock, retaining its original fields and unknown fields and adding current recovery
evidence; it must not replace the original child exit with a cancellation result. The **single
current reader** treats absent cleanup evidence as unknown; it never infers success from an old
`termination` field. This is a named current consumer of historical run/profiling evidence, not
a parallel old-format reader service. No verification-summary schema change follows merely
from the run-record version. On explicit recovery, trustworthy boot/process identities may prove
the owned group gone; inability to establish identity is unknown, not permission to signal a
possibly reused PID. No bulk migration, deletion or automatic recovery scan is required.

`state_of`/`view` expose process state and cleanup independently. `cancel` retries failed/unknown
cleanup even after process exit; its result distinguishes confirmed stop from unresolved recovery.
`prune` excludes unresolved cleanup and retained data. Recovery preserves the one-writer owner
lock, observes a live owner's request protocol, and never races a second record writer.
Receipt publication failure cannot bypass cleanup; where a record cannot be written, stderr and
the command exit report the supervisor failure without inventing a persisted successful receipt.
A successful child plus failed supervisor is an unsuccessful wrapper operation, while the stored
child exit stays zero. A failing child retains its actual code and separate supervisor evidence.

Migrate actual consumers in the same slice: run list/status/cancel/prune, verification's run and
summary pointers, compile-profile status/report/attach/retention, and worktree removal guards.
Worktree removal must consider a live owner/group and unresolved relevant cleanup, not just
`state == running`. Compile-profile partial/completion meaning must not turn process exit into
capture completeness. Existing profiling/benchmark captures have a current consumer and remain.

### 3.2 Output observation and selective log access

The child writes full output directly to its owned file. Supervision polls child/group, cancellation
and progress independently of a terminal reader. Use a separate file-following display observer
with bounded internal buffering and a detachable lifecycle; the supervising process performs no
potentially blocking terminal writes in its critical loop, including startup/progress feedback.
The observer can be stopped when its sink breaks or stalls without stopping the child. It must
not accumulate an unbounded queue. Full disk logs remain authoritative; a detached observer may
miss live presentation, which the final path/record makes recoverable. This is an observation
algorithm, not a workload/output/duration cap.

`verify --live` uses the same boundary: remove its child PIPE → synchronous stdout dependency;
capture the boundary log directly and follow it independently. Nested `just run -- just verify`
does not create two competing child owners. Observer startup/failure/cleanup cannot determine a
boundary pass. Process-group cleanup excludes or separately identifies observer processes so
an observer is never mistaken for a leaking product child.

A shared file helper reads the last N lines backwards in blocks and follows from a byte offset.
Use it in `runs logs --tail` and verification's failure excerpt. Handle empty files, N=0, an
unterminated final line, CRLF, multibyte text and a line larger than one block. Memory/work follows
the requested suffix, including legitimately large requested lines, rather than the entire log.
Follow handles an incomplete trailing character/line and file replacement/truncation explicitly;
it does not reread all previous output. Full log access remains available.

### 3.3 Environment ownership spans the observation it justifies

`workspace_env.py` remains the publication/readiness/ownership authority. A managed operation
acquires its effective requirement-scoped shared ownership **before** readiness observation and
holds it through discovery/import/execution. Nested managed consumers reuse a valid ancestor's
ownership. Pure Rust boundaries take no Python lock. Static `verify --print` stays an advisory,
non-building preview and cannot be presented as execution admission.

`verify --list` is execution: nextest discovery can compile and pytest collection can import.
Resolve the selected requirements, acquire them, observe readiness and discover under the same
ownership. Do not sync to make discovery pass. Queued `sync` observes again after exclusive
acquisition, decides whether work is still needed, publishes and validates while exclusive
ownership remains held. An earlier advisory observation cannot justify a later decision.
Independent readers retain overlap; no global command serialization or new resource limit follows.
Foreign environment/target overrides and unmanaged native tools retain their documented routes.

### 3.4 Truthful launch, child and fixture outcomes

Use a small launch result at the existing subprocess boundary with actual launch error versus
launched child exit. Missing/non-executable executable is evidenced launch failure and can be
`blocked` with a repair. An actual child exit 127 is an ordinary failed child unless its own
contract establishes more; do not reconstruct launch failure from an integer. Carry the
distinction through step results, summaries, feedback and rerun without a second error framework.

The fixture owner supplies a shared server-end observation/policy to direct fixtures and
verification. Evidenced OOM/infrastructure unavailability is `blocked` (fixture exit 75);
unexpected unknown-cause server exit is `failed` with unknown cause, not an asserted product
defect. Keep the child outcome separately when the fixture ends. Intentional cancellation is
identified and unexecuted controls stay `not_run`. Schema/query refusal remains a query outcome,
not proof of launch failure. Redact credentials and parameter values from causes.

### 3.5 Freshness is a read-only input observation

AE-24 has two inseparable obligations. First, Hakari's `--diff`/`--dry-run` do not protect
Cargo metadata's lockfile writes. Choose an **owned disposable metadata workspace** for the
existing pinned Hakari computation: capture the current manifests, target-discovery inputs,
lock/config and necessary path layout, use offline Cargo, and perform any lock normalization
only in that copy. No source manifest/lock is writable through a symlink back to the checkout.
Do not copy build intermediates, venvs or captures; no compilation or fetching is needed.
The existing Hakari tool computes the proposed feature union/dependency edits, rather than a
new graph implementation. Compare its result with the captured inputs; if required inputs or
offline dependencies are missing, report `not_run` with repair, never `clean`. Detect a concurrent
source-input change and report the observation stale/inconclusive rather than restoring anything.
Do not use detect-then-rollback. A future native locked-metadata option could replace isolation
after its complete write path is qualified; it is not an assumption of this target.

Second, `docs.py` owns a compact publication receipt inside the same candidate/atomic replacement
as `build/docs/site`. It fingerprints actual discovered inputs, site/book configuration, theme,
publisher implementation, declared and actual installed tool identities and output-affecting Git revision/dirty
annotations. Include publication selection so a partial artifact cannot certify a full site.
Record the checked tool version and resolved binary identity at publication; observation must
detect a changed/missing binary rather than assume matching declarations prove the same tool.
Capture the source bytes and output-affecting context used by staging once, and derive the
receipt from that same capture. Recheck source/context/tool identities before replacement;
if they changed during rendering, refuse candidate publication with a stale-input explanation,
preserving the prior site. A subsequent edit is detected against the captured receipt. Never
hash newer live inputs into a receipt for older rendered bytes.
`freshness.py` calls a read-only publisher observation function to compare current inputs with
that receipt; it never builds or invokes `docs-check`. Missing receipt is `not_run`, mismatch is
`stale`, and `clean` means only the named publication inputs match. This is not link acceptance,
tool installation or architectural validation. Repair remains `just docs-check`.

## 4. Capability decisions and support boundaries

Every source-review §8 proposal has a chosen route. Supporting exact-version research is in
the [Codex/Rust](../design_review/evidence/2026-10-09_agent-effectiveness-followup/capabilities-codex-rust.md)
and [SurrealDB](../design_review/evidence/2026-10-09_agent-effectiveness-followup/capabilities-surrealdb.md)
briefs. These are **Interface-checked** where stated, not exercised integrations.

| Proposal | Decision and concrete consumer | Qualification / explicit deferral |
|---|---|---|
| SurrealQL syntax validation | **Adopt native guidance:** installed SurrealDB 3.3 `validate --stdin` before a drafted query/schema attempt. No custom grammar/validator. | One valid and invalid draft; distinguish syntax from schema/type/runtime acceptance. AF6. |
| Fixture schema/index/query-plan inspection | **Adopt thin fixture operation:** explicit live attachment/retained-content configuration plus SQL input; reuse existing `/sql` authentication/envelopes. INFO/index and bounded EXPLAIN are inspection operations, while arbitrary SQL remains explicitly effectful. | Known disposable content, wrong scope, ended attachment, query failure and cleanup. Fresh `--attach ID` makes a new namespace; it cannot silently inspect previous content. AF6. |
| Interactive fixture MCP | **Adopt task-scoped built-in HTTP MCP launcher** at the same fixture owner, after HTTP diagnostics and run/fixture lifetime corrections. Dynamic URL/private auth derive from the current attachment; Codex invocation overrides avoid global registration. | Actual handshake/tool visibility, explicit scope/auth, mixed errors, exit/interruption and no lingering registration. MCP failure leaves `/sql` usable; no claim of adoption until qualification passes. AF7. |
| Per-statement errors | **Adopt sanitized diagnostics at the harness `/sql` result boundary:** indexed status, safe category and transport/statement distinction. Reuse SDK/native envelopes rather than parsing error strings into product meaning. Native MCP retains its native error output and is limited to non-sensitive disposable fixture data. | HTTP failure versus mixed statement success/failure; harness causes exclude SQL/bind/credential values. MCP `is_error=false` alone is insufficient; inspect `has_errors` and statement statuses, without claiming native messages are sanitized. AF6/AF7. |
| Rust semantics and rename | **Adopt qualified existing-tool guidance.** Adapter 0.4.0 symbols/definition/hover help concrete questions; rename returns reviewable edits and waits for loading. References/code actions can forward before loading completes. | Empty cold references are inconclusive: use `rg`/ast-grep. Scratch rename/refusal/file-operation controls before claiming the installed path exercised. **Defer adapter patch** until a reproducible consumer needs missing health/loading or action resolution; then use upstream or managed pinned source, never registry edits. AF8. |
| Package/target/compiler evidence | **Adopt native structured routes:** locked/offline `cargo metadata --no-deps --format-version 1`, selected nextest list JSON, compiler JSON during an already authorized compile. | Metadata lists packages/targets, not resolved dependency graph; nextest list builds; `build-finished` is not test success. Preserve exact installed grammar and native flags. AF8. |
| Selective receipt recovery | **Adopt existing files/status JSON and compact examples; defer new selector CLI.** Read authoritative `summary.json` and boundary logs through a file/structured channel, returning selected fields and full-data paths. | T1 demonstrates display truncation, not inability to select an existing file. Reopen a selector only for a repeated concrete consumer that cannot use these channels; no second receipt store. AF8. |
| Codex configuration diagnostics | **Adopt native guidance:** 0.162.0 strict config, redacted `doctor --json`, configured `mcp list/get --json`, active TUI `/mcp` and native `exec --json`. No repository diagnostic middleware. | Configured, active and exercised are separate. Run connectivity/auth diagnostics for an actual discrepancy, not every startup; changed config does not alter an existing session. AF8. |
| Compilation attribution | **Adopt existing compile-profile status/report/readers and fresh-artifact evidence.** Reuse retained captures first; record a new profile only for a concrete compilation question. | Shared sccache counters do not attribute concurrent work; incremental workspace crates are not cacheable rustc work. No cache clearing, incremental disable or new profiler registry. AF8. |

**MCP interface choice, Interface-checked 2026-10-09:** SurrealDB 3.3's
[HTTP MCP handler](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/ntw/mcp.rs)
uses the running datastore; `surreal mcp` starts a different datastore and is not this route.
The [native tool scope](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/mcp/src/tools/mod.rs)
allows explicit namespace/database on each call; avoid mutable `use` state. A thin fixture launcher
reads its private `LCTX_SURREAL_TEST_CONFIG`, derives endpoint `/mcp`, puts the Basic header value
in a child-only environment variable, and passes only that variable's name in Codex `-c` argv.
Use a fresh local `codex --no-daemon` task so invocation overrides belong to the child; the
attachment remains owned until that child exits. Admin credentials are already fixture-owned;
do not claim least-privilege authorization or provision a new account implicitly.
Native MCP can return full database error messages in both text and structured content; the
launcher does not intercept or sanitize tool responses. Its supported diagnostic scope is an
owned disposable attachment with synthetic/non-sensitive inputs and content. Keep credentials
out of SQL/tool arguments and logs; child-only auth protects transport configuration, not arbitrary
server error text. Sensitive or uncertain retained content uses the sanitized harness diagnostic
route instead. Expanding MCP beyond that scope requires a separately justified native protection
or design change; no sanitizing bridge is added by this plan.

Installed Codex 0.162.0 accepted the following **read-only synthetic configuration decode**
(exit 0, 2026-10-09); no connection, service or registration was made:

```sh
codex -c 'mcp_servers.fixture_probe.url="http://127.0.0.1:1/mcp"' \
  -c 'mcp_servers.fixture_probe.env_http_headers={Authorization="LCTX_FIXTURE_MCP_AUTH"}' \
  mcp get fixture_probe --json
```

[Official Codex MCP guidance](https://learn.chatgpt.com/docs/extend/mcp?surface=cli), installed
help and local adapter source constrain applicability. For authoring, live server availability, actual Basic
authentication, tool schema/exposure, source-to-installed adapter equivalence and client lifetime
were **not_run**. Subsequent disposable implementation observations are recorded in §9 and
[the diagnostic guide](../surrealdb.md#disposable-agent-diagnostics); adapter provenance beyond the
installed scratch request remains unqualified. Native gRPC stream-terminal acceptance remains with product owners; HTTP/MCP
diagnostic success cannot substitute for it.

Rustdoc JSON, HIR and MIR remain distinct optional tools for public API, resolved semantic and
control-flow questions. Use the selected pinned `rust-code-model` skill only when needed and check
its toolchain/profile fit. No standing extraction index or mandatory semantic warm-up is planned.
Do not force indexing by asking for full diagnostics. Upstream
[server status](https://github.com/rust-lang/rust-analyzer/blob/master/docs/book/src/contributing/lsp-extensions.md)
distinguishes quiescence from health; adapter quiescence alone proves neither complete nor healthy
analysis. A returned code action may require resolution/commands rather than containing edits.

## 5. Command evidence and proportionate changes

The frozen retrospective contains 22,211 native completions in 115 attributed threads,
1,727 nonzero exits and nine malformed JSON lines. These are **historical observations**, not
an adjudicated failure census or effectiveness rate. The window is partial October 9, excludes
this review's root/descendants and covers older Codex 0.160–0.161 activity; installed help is
0.162.0. Preserve the retrospective's event identities and classification limits.

| Trace | Chosen change / owner | Acceptance and limit |
|---|---|---|
| U1: `runs retain RUN --json` rejected | AF8 adds discoverable per-operation output options and an existing valid retention example at run help. Keep plain retention; no new JSON option without a caller. | Exact documented command parses; retention success says nothing about the underlying workload. |
| U2: repeated global Cargo `--tests` | AF8 documents repeated `-p` with one `--tests`, package discovery and native argument forwarding. | Help/parser example uses installed grammar. Later success also edited code; no same-code repair claim. |
| U3: `verify --background` rejected | AF8 verification help shows `just run --background --label LABEL -- just verify --select leaf:deps`. | Launch receipt and boundary result remain separate; no second background owner inside verify. |
| E1: Python 3.12 parse error, then independent HTTP 400 | AF8 retains `uv run --no-sync python` routing; AF6 explains transport/statement failures. | No Python 3.12 compatibility rewrite or invented HTTP cause. |
| T1: displayed JSON truncation caused parser failure | AF8 uses files/structured channels for complete JSON and compact displayed receipts with selected fields/log paths. | A display warning never becomes a producer failure. Preserve batching and arbitrary output; no global output cap. |
| T2: malformed orchestration JavaScript | No process-skill prescription is added (operator correction, 2026-10-09). A rejected outer expression is a runtime observation; revisit only for a concrete repeated capability gap. | Rejected outer syntax executed no shell command. No new workflow rule or always-loaded handbook. |
| T3: unknown native process handle | AF8 distinguishes native session IDs, code-mode cell IDs and durable run IDs; documents durable recovery when required. | Original cause remains unresolved; do not attribute it to cancellation or a harness bug. |

Retired Docker/root-owned SST cleanup and an earlier absent native binary remain historical,
not current native-fixture defects. Assertions, compiler errors, authentication refusals,
timeouts and canceled product journeys remain with their actual owners. No model change or
parallelism restriction is a supported remedy. Shared library skills remain repo-agnostic;
repository wiring/examples live in repository help/runbooks. Preserve overlapping useful
guidance with links; context reduction alone has no established benefit.

## 6. Execution fit, alternatives and change scenarios

The target keeps one durable owner, direct file capture, small result observations and native
tools. It repairs authority crossings instead of adding another runtime. A small tail reads a
suffix; follow advances an offset. Concurrent readiness binds to the resource lifetime that
justifies it. Inspection reuses the actual native server/content, rather than copying it into
an embedded MCP store. The extra display observer and metadata scratch copy have explicit
owners/cleanup and no persistent service. These are qualitative execution-fit judgments, not
measured speed or memory claims.

Instructions alone are insufficient for F01–F05. Bare tools/runtime handles remain the simplest
route for ephemeral work, but cannot replace durable cross-session recovery. A new workflow
engine, receipt store, mandatory semantic index or permanent MCP registration would add state
without a demonstrated consumer. A custom Rust/Cargo argument parser would duplicate native
grammar. The Hakari scratch route costs capture/metadata work but prevents mutation without
reimplementing feature resolution; missing offline inputs should produce an honest non-result.

| Credible change/failure | Expected propagation |
|---|---|
| Add a new verification boundary with Python collection | Declare requirements once; discovery and execution reuse environment ownership. Native filters remain native. |
| Process exits but descendants survive; record write fails | One run owner attempts cleanup, preserves uncertainty, enables retry and protects captures; no consumer independently infers success. |
| Terminal stops reading during a long build | Display detaches; log capture, cancellation and supervision continue. No child/job cap or shorter duration. |
| Replace live presentation mechanism | Observer changes while process ownership, records and full logs stay the same. |
| Add a diagnostic for a retained fixture | Select exact checked content configuration; reuse SQL/MCP envelopes and lifetime. No namespace guessing or operator-store activation. |
| Add another site input or publisher option | Publisher updates its own fingerprint; freshness consumes that owner instead of another inventory. |
| A repeated Rust reference task needs loading health | Reopen the adapter deferral using the actual request/source version and managed provenance; no unrelated toolchain migration. |

## 7. Work packages, dependencies and acceptance

The packages below are implemented and accepted within the operator's focused scope. Dependencies mean implemented-and-focused-tested
contracts unless explicitly described as design only. Ready independent work may proceed in
parallel, but `runs.py`/`harness.py`, `verify.py`, fixture and shared documentation each need one
writer at a time. The root owns shared contracts, integration and acceptance; package ordering
is not a global synchronization barrier.

| Package | Result, actual files/consumers and retirement | Prerequisite and revealing acceptance |
|---|---|---|
| AF0 — decisions and current owners | Record §2 through ADR/owning workflow prose, update applicable AGENTS/help links and execution checkpoint. Preserve historical receipts; no implementation closure from decision acceptance. | Operator RC01–04 already accepted. Review decision metadata/owner consistency and affected docs publication; no product gate solely for this package. |
| AF1 — cleanup and recovery | Implement §3.1 in `harness.py`, `runs.py`, fixture post-spawn guard; migrate status/cancel/prune, compile-profile, verify pointer handling and worktree removal together. Retire terminal-implies-cleanup shortcuts. | AF0 run decision. Inject failures immediately after spawn, identity capture and record write; leader exits with survivors; termination refuses/unknown; retry and prune protection; foreign/reused PID refusal. Observation preserves old record bytes; explicit recovery persists one record upgrade without losing fields/exits or touching captures. Exercise real disposable child/group behavior, not only mocked booleans. |
| AF2 — display and log observation | Implement §3.2 in run/verify owners and a shared file helper; retire synchronous terminal mirroring and whole-file tail reads. | AF1 cleanup/observer ownership slice. Connect a non-consuming pipe and a broken pipe; child completes/log is complete and cancellation remains usable. Tail parity for partial/CRLF/multibyte/large-line cases and follow rotation; failure excerpt uses the helper. |
| AF3 — owned readiness/discovery | Implement §3.3 in `workspace_env.py`/`verify.py`; retire pre-lock readiness admission and unowned discovery. Preserve nested reuse, pure Rust/no-lock and shared readers. | AF0 environment decision. Controlled reader/writer interleavings: list holds ownership; writer queued behind reader rechecks newer state after acquisition; discovery cannot import a half-published extension. Verify no sync is invoked and independent readers overlap. |
| AF4 — execution/fixture outcomes | Implement §3.4 in launch/verify/fixture owners; summaries/rerun consume actual launch and server observations. Retire integer-127 launch inference and duplicate server-end policy. | AF1 exception guard; coordinate AF2/AF3 shared verify edits. Missing executable, permission-denied launch, launched exit127, evidenced OOM, unknown server exit and intentional cancellation; direct/family paths agree and child outcome survives. |
| AF5 — read-only freshness | Implement §3.5 in `freshness.py` and `docs.py`, existing recipes/help and their controls. Retire live Hakari lock mutation and publication-without-input-receipt. No new artifact registry. | Independent of AF1–4. Stale lock and manifest inputs remain byte-identical; concurrent edits are preserved; missing offline inputs report non-result. Site fingerprint stable when unchanged, stale on each relevant source/context change, missing/partial receipt never clean. An edit between staging and publication cannot certify old rendered bytes as current. Freshness performs no publication/build and no rollback. |
| AF6 — native syntax and fixture diagnostics | Add the thin explicit-scope SQL diagnostic operation and native validation guidance; preserve indexed outcomes/redaction at the existing fixture API/CLI. Reuse current authentication and attachment/content routes. | AF1 fixture guard and AF4 outcome contract before integrated diagnostic acceptance; syntax guidance can proceed earlier. Owned disposable known schema/index/content, correct/wrong scope, syntax-valid/runtime-invalid query, mixed statuses, transport refusal, useful redaction and cleanup. No operator or real-library data. |
| AF7 — invocation-scoped MCP | Add §4 launcher to the fixture owner with child-only auth environment and native Codex overrides; no new global config, stdio datastore, plugin or bridge. Document explicit namespace/database calls and the native-output/non-sensitive-content support limit. | AF1/AF2 lifecycle, AF4 outcomes and AF6 checked scope/HTTP diagnostics. Exercise a fresh Codex session on one disposable synthetic attachment: handshake/INFO/EXPLAIN, positive/negative auth, mixed failures, child exit/interruption and no residual registration/transport credentials in output. Inspect native errors without claiming redaction; reject unsupported content routing. If blocked, record prerequisite/repair; AF7 is not completed by HTTP-only fallback. |
| AF8 — precise native/help routes | Update task-owned run/verification/environment/compiler/Codex guidance for §4–5. Keep native versions/effects/overrides explicit; do not put detailed examples into always-loaded AGENTS unnecessarily. No selector/adapter patch/diagnostic CLI is implemented. | Mostly independent; final examples consume AF1–7 interfaces where relevant. Parser/help checks without expensive workload; scratch semantic rename/reference cases only for claimed exercised abilities. Native output/fixture scope examples must distinguish configured/active/exercised and syntax/compile/test success. |
| AF9 — integrate and qualify | Reconcile actual consumers, remove replaced implementation paths, update single finding table/owners/STATUS; independent assembled review of the implemented scope. | Relevant AF1–8 packages focused-tested, AF0 routes in place. Run affected harness controls and applicable leaves once, under the operator acceptance boundary below. No performance or product-failure closure inferred. |

During implementation use compile checks only for touched Rust (none is currently planned),
focused controls and minimal revealing cases. Named Python test owners are
`tests/scripts/test_harness.py`, `test_runs.py`, `test_verify.py`, `test_surrealdb_fixture.py`,
`test_workspace_env.py`, `test_compile_profile.py`, `test_worktree.py`, `test_freshness.py`
and `test_docs.py`. Select the actual affected files/cases through
`uv run --no-sync pytest ...` or `just verify --select tooling:python --pytest-args "..."`;
verify readiness first through its ordinary owned route. Do not sync while readers are live.
`just docs-test` applies to AF5's publisher changes; `just docs-check` applies to published docs.
Tests must expose effects/interleavings, not simply mirror the implementation's classification.

**Operator execution override, 2026-10-09:** comprehensive repository acceptance is explicitly
excluded while existing failures are being addressed. AF9 establishes that the intended changes
function as premised, and that first-principles reasoning plus direct execution observations
support their expected benefit. Run revealing affected controls and actual disposable command
journeys, with independent implemented-scope review; do not run `just qualify`, full families or
a comprehensive repository suite for this task. This supersedes this plan's earlier AF9 assembled
qualification requirement, not ADR-0126's general policy. Preserve existing product failures at
their owners. AF7's disposable task-scoped client check is in scope; real-library qualification,
operator activation and quantified effectiveness are not.

Benefits remain unmeasured. A future before/after task sample or compilation comparison is useful
only when the operator asks for a quantitative claim; it is not a prerequisite for these concrete
correctness and capability changes. No new measurement campaign is scheduled.

## 8. Finding and recommendation disposition owner

Stable source IDs below are retained; linked sources own the dated evidence. This table alone
owns current scheduled disposition. Package acceptance requires the full closure evidence,
not merely an accepted ADR or a passing mock.

| Source obligation | Current disposition / responsible owner | Closure evidence or revisit trigger |
|---|---|---|
| Follow-up F01 | Closed, focused-Tested — AF1/AF9 run/fixture lifecycle, 2026-10-09 | Real post-spawn failures, descendant cleanup/refusal/retry, legacy observation/selected upgrade, preserved records and downstream profile/worktree protection; integrated F01/F02 corrections below. |
| Follow-up F02 | Closed, focused-Tested — AF2 run/verification display, 2026-10-09 | Actual stalled/broken sinks, complete logs, cancellation and independently ended observers; affected harness/run/verify controls in §9. |
| Follow-up F03 | Closed, focused-Tested — AF3 environment/verification, 2026-10-09 | Real shared/exclusive interleavings, owned discovery and post-wait readiness; independent readers overlap without synchronization. |
| Follow-up F04 | Closed, focused-Tested — AF2 shared log reader, 2026-10-09 | Both consumers use suffix/offset access; empty, partial, CRLF, multibyte, large-line and replacement/truncation controls pass. |
| Follow-up F05 | Closed, focused-Tested — AF4 launch/fixture outcome, 2026-10-09 | Actual missing/non-executable launch versus child127, unknown/native OOM endings and cancellation preserve distinct outcomes. |
| Earlier AE-24 | Closed, focused-Tested — AF5 publisher/freshness, 2026-10-09 | Scratch Hakari leaves live bytes unchanged; publisher-captured inputs and schema2 receipt-integrity controls reject partial inventories and publication drift. |
| Follow-up §8 capability recommendations | Adopted within §4 limits — AF6/AF7/AF8, 2026-10-09 | Disposable SQL/native MCP and scratch semantic requests exercised; native help/metadata/composition checked. Adapter patch, selector CLI and diagnostic middleware retain §4 triggers. |
| Retrospective U1/U2/U3/E1/T1/T2/T3 | Guidance implemented / interface-checked — AF8/AF6, 2026-10-09 | §5/§10 examples and the actual background/retention path. T2 has no skill rule per operator correction; T3 cause remains unknown. Historical workload failures are not closed by guidance. |
| Plan target review F01/F02 | Closed in Proposed target — §3.1/§4 and AF1/AF7; [independent review](../design_review/reviews/design_review_agent-effectiveness-plan_2026-10-09.md#7-stable-authoring-findings-and-their-resolution) | Static rereading confirmed the explicit selected legacy-record upgrade and native MCP output/content support limit. Implementation closure is not established. |
| Integrated review F01 | Closed, focused-Tested — fixture lifecycle and worktree consumer, 2026-10-09 | [Independent source closure](../design_review/reviews/design_review_agent-effectiveness-integrated_2026-10-09.md#F01): unresolved records survive teardown/sweep, acquired current locks and reread protect every recovery/deletion path; actual child retry and checkout protection controls pass. |
| Integrated review F02 | Closed, focused-Tested — shared direct-owner guard, runs and fixture, 2026-10-09 | [Independent source closure](../design_review/reviews/design_review_agent-effectiveness-integrated_2026-10-09.md#F02): non-reaping exit observation retains group authority through cleanup; lost pin refuses signaling; real descendant and unrelated-group controls pass. |
| Loaded-extension `build_identity()` | Routed, unchanged — earlier workspace D1 deferral; later native-extension product change | A concrete loaded-versus-on-disk identity discrepancy can reopen that route; no new import gate is scheduled by this follow-up. |
| Earlier AE-12/AE-25 coverage | Routed, unchanged — [testing architecture plan](testing-architecture-pivot-plan_2026-10-04.md) via earlier workspace §9 | Coverage owner revises families; this plan tests its own contracts without duplicating that backlog. |
| Product native/stream/timeouts/equivalence | Routed, unchanged — STATUS and native/persisted graph coordinators | Their original acceptance and authorization; no closure from this plan. |

## 9. Execution checkpoint

**Proposed / Interface-checked, 2026-10-09:** current source/consumer mapping and native capability
research settled the choices above. `cargo hakari --help`, `generate --help` and
`manage-deps --help` passed as interface inspection; their dry-run flags do not establish
lockfile immutability. Codex synthetic config parsing passed as described in §4. No service,
handshake, semantic request, build, test, sync, install, pilot or operator action ran for authoring.

**passed, 2026-10-09:** independent target review at Proposed/scoped strength, `just docs-check`
(364 canonical pages, zero link errors), and `git diff --check`. These establish authoring and
publication acceptance only; product acceptance is **not_run**. These historical checks establish the authored target only. Execution was subsequently authorized
on the current main tree; no separate checkout was used.

**Implementation acceptance, 2026-10-09:** affected verification/environment controls **passed**
(`uv run --no-sync pytest tests/scripts/test_verify.py tests/scripts/test_workspace_env.py -q -o addopts=''`,
70 cases). This includes real reader/writer locks, post-wait readiness, actual missing/permission
launch failures versus launched127, direct verification display with stalled/broken sinks and
complete2MiB logs, and unknown fixture endings without inventing a cause. A direct owned
`--list` invocation collected the requested node ID; the first execution exposed and repaired
pytest's duplicate-quiet false-empty observation.

The actual background composition in §10 **passed** for four selected harness cases, run
`20261009T230142.374Z-18e4c0`; its `summary.json` records the passed selection and its schema-2
`record.json` records child0, confirmed cleanup and no remaining group. Plain retention also
**passed**. These exercise the launcher/verification composition, not dependency policy or a
whole tooling family. The receipt is retained under `build/runs/`.

AF5 controls **passed**,53 cases after independent review found and corrected incomplete dynamic
receipt inventories. Schema2 validates a canonical receipt-body digest before consuming its
inventory; omitted asset entries now yield `not_run`. This is accidental-integrity checking,
not authentication. `just docs-test` **passed**,78 cases after that correction; after the
publisher source-path type narrowing, its40 affected controls also **passed**.
`just fresh hakari docs` observed Hakari **clean** and the older docs receipt **not_run**;
it did not modify the live lockfile or publish docs.

Independent source reviews accepted AF1/AF2/launch and AF3/AF5 within their assigned bounds.
The assembled review found fixture-cleanup retention/ownership gaps (integrated review F01)
and premature leader reaping before cleanup (F02). F02 is corrected and independently closed:
direct owners observe terminal state with `waitid(WNOWAIT)`, clean while the leader pins the
group ID, and reap only after confirmation; a lost pin refuses signaling. F01 is also corrected
and independently closed: current fixture/attachment locks freeze writers, reread identities and
govern recovery, sweeping and deletion; unresolved/unreadable evidence remains protected.
Native HTTP/MCP and scratch semantic
observations are described in §4/§10 and [the diagnostic guide](../surrealdb.md#disposable-agent-diagnostics).
No process-skill change is retained. Comprehensive repo tests, product qualification and measured
effectiveness remain **not_run** under the operator's scope.

The nine affected tooling modules **passed**,313 controls in52.47s, through this actual durable
composition (2026-10-09). Run `20261009T231848.005Z-9bf4ac` is retained; its `output.log` contains
the controls and its `record.json` records child0 and confirmed cleanup. This is the affected
tooling scope, not a comprehensive repository test run. Later fixture ownership corrections
have their own narrow rerun below.

```sh
just run --background --label agent-followup-affected-controls -- \
  uv run --no-sync pytest tests/scripts/test_harness.py tests/scripts/test_runs.py \
  tests/scripts/test_verify.py tests/scripts/test_workspace_env.py \
  tests/scripts/test_compile_profile.py tests/scripts/test_worktree.py \
  tests/scripts/test_docs.py tests/scripts/test_freshness.py \
  tests/scripts/test_surrealdb_fixture.py -ra -o addopts=''
```

Final fixture corrections **passed**,20 selected controls,28 deselected in11.27s. The first
correction rerun was **failed composite** (18 passed,2 failed): stronger locking exposed the
context retaining its own attachment lock during teardown. Releasing that attachment while
still holding the server owner lock repaired ordinary teardown without weakening external
recovery refusal. Deterministic replaced-lock/changed-record controls and actual native
SQL/MCP, interruption, unknown server end and surviving-child cleanup/refusal/retry now pass:

```sh
uv run --no-sync pytest -o addopts='' -q tests/scripts/test_surrealdb_fixture.py \
  -k 'incomplete_sweep or kept_sweep_rereads or run_sweep_recovers or stale_fixture_lock or sweep_removes_only or dead_run_owned_fixture_terminates or recovery_locks_freeze or previous_boot_launch or kept_attachment_with_live or failed_fixture_cleanup or force_recovery or native_mcp_launcher or disposable_diagnostics or unknown_native_server_end'
```

The equivalent lint cleanup in shared harness/run code was followed by **passed** affected
controls: `uv run --no-sync pytest tests/scripts/test_harness.py tests/scripts/test_runs.py
tests/scripts/test_verify.py -q -o addopts=''`,139 cases in12.22s. The
[independent integrated review](../design_review/reviews/design_review_agent-effectiveness-integrated_2026-10-09.md)
accepts the examined implementation at scoped strength; F01/F02 retain their dated diagnoses
and closure evidence. No whole-repository or quantitative claim follows.

**Final non-functional scope, passed 2026-10-09:** Ruff on the18 affected production/test Python
files; pyrefly on the9 affected production scripts (zero errors); `just adr-lint` (78 current
records); `just docs-check` (374 canonical pages, zero link errors); scoped `git diff --check`.
`just turn-end --paths ...` regenerated the ADR index and formatted only this work's Python
files; build-feature generation and Rust formatting were skipped because no Cargo/Rust file
changed. The post-publication observation, `just fresh docs`, reported **stale** while concurrent
documentation inputs continued changing; it did not publish. Process-skill diff is empty.
These leaves establish their named scope, not existing
product-test closure.


## 10. Task-owned command examples

Use installed help for the selected operation. Examples retain native arguments and authority;
these routes do not synchronize, qualify the product or imply measured effectiveness.

```sh
# Complete package/target data in a file; no build or dependency graph resolution.
cargo metadata --locked --offline --no-deps --format-version 1 > build/cargo-metadata.json
# Preview only; repeated -p PACKAGE uses one global --tests with native Cargo/nextest.
just verify --print --select model --nextest-args '--tests'
# Discovery compiles Rust binaries or collects Python cases under environment ownership.
just verify --list --select tooling:python --pytest-args '-k NAME'
# Durable background ownership composes around verification.
just run --background --label deps -- just verify --select leaf:deps
just runs status RUN --json
just runs logs RUN --tail 30
just runs retain RUN
just verify --rerun RUN
# Read-only generated-input observations; docs freshness never publishes.
just fresh hakari docs
# Use existing captures before recording another compilation workload.
just compile-profile status RUN
just compile-profile report RUN
```

`retain` has plain output and no `--json`. Run `list`/`status` have JSON output. Child exit,
termination and cleanup are independent fields; a terminal child can still have protected,
unresolved cleanup. `runs cancel RUN` requests live cancellation or explicitly recovers a
selected dead owner; merely reading status never upgrades historical records. Full `output.log`
and verification `summary.json` live at the paths in status. Select the fields needed from those
files using `uv run --no-sync python`, rather than parsing a truncated displayed blob. Bare
launchers take argv: use `env NAME=value COMMAND`, or an explicitly quoted shell when shell
operators are intended. Native process, code-cell and durable-run handles are distinct; use the continuation API
associated with the returned handle.

For compiler events, request Cargo's native JSON during an already authorized compile and retain
its package/target/span/code/artifact fields. `build-finished` establishes compilation only;
nextest's list JSON establishes selection only. Do not add `--no-capture` for visibility: installed
nextest runs serially with that option. `--live` observes the full file independently of the child.
Shared sccache totals do not attribute concurrent work; workspace incremental Rust is not
cacheable rustc work. Keep the existing compilation profile and retained captures.

For a concrete Codex configuration discrepancy, inspect installed `codex --help`,
`codex doctor --json` (redacted native diagnostics), `codex mcp list --json` and
`codex mcp get NAME --json`. Use `--strict-config` to refuse unknown keys. Configured servers,
active session `/mcp` entries and exercised tool calls are different observations. Config changes
apply to a fresh session. No repository configuration validator or global registration is added.

For Rust semantics, use the current rust-analyzer MCP symbols/definition/hover for a specific
question; references can be inconclusive while cold. Rename returns reviewable changes with
`applied: false`; review both text edits and `file_operations`, including position encoding,
before applying them. Do not force full-workspace diagnostics as a warm-up. `rg` and ast-grep
remain the fallback. Use the selected `rust-code-model` skill for a concrete rustdoc/HIR/MIR need,
checking its pinned compiler/profile against this checkout first.

**Exercised, 2026-10-09:** in `build/agent-followup-semantic`, a disposable two-file Cargo package
using the checkout toolchain, the installed rust-analyzer MCP returned both declaration/reference
locations; function rename returned two edits across two files, module rename included the file
rename, and a non-symbol position was refused. No returned edits were applied; source bytes
remained unchanged. The MCP workspace was restored to the repository. This qualifies the small
scratch path, not full-workspace indexing health or arbitrary code-action execution.
