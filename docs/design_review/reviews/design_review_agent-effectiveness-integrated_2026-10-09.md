# Integrated agent-effectiveness design review

**Design · target · independent assembled review · 2026-10-09.**

The implementation has a credible route to making concurrent agent work easier to observe and
recover. It keeps native commands and existing owners, separates presentation from supervision,
and makes uncertainty visible instead of converting it into success. These are useful functional
improvements on a personal Codex-primary workstation; no measured productivity or speed claim
follows. **Accept scoped, at Implemented and named focused-Tested strength.** F01's fixture
recovery/deletion gap and F02's direct-owner process-identity defect were corrected during this
review and independently reread. Neither correction requires an additional service or workflow rule.

## 1. Scope and decision boundary

The subject is AF0–AF9 of the [active plan](../../plans/agent-effectiveness-followup-plan_2026-10-09.md),
its implementation in the current `main` working tree, and the actual adjacent consumers. The
inspected base was `23daeaa8e7b7b777ff16dee91acfc08aed4e8b49`; implementation and integration
changes were uncommitted and concurrent. This is a dated source assessment, not certification of
an immutable release. The plan's authoring baseline `2a9a3771` remains historical.
The [source follow-up review](design_review_agent-effectiveness-followup_2026-10-09.md)'s F01–F05
and transferred freshness obligation supply the original failure scenarios; the resulting
implementation and real consumers, rather than its prior verdict, supply this review's judgment.

The applied [standard](../design_principles/standard.toml) is core/template 3.3,
heuristics 1.0, code-intelligence profile 1.5 and the library-context binding, through the
`design-review` and `design-review-code-intelligence` skills. The workflow owner is
[DESIGN §1.2](../../design/DESIGN.md), with [ADR-0142](../../adr/0142-agent-execution-ownership-and-observation.md)'s lifecycle and readiness decision. The target
is the plan's functional outcome, rather than agreement with those chosen mechanisms.

The workload is one operator with several concurrent tasks: short queries, long native builds
and controls, a mutable Python/native environment, large logs, and observers that can disappear
or stop reading. Durable recovery is optional; ephemeral work can use its runtime's native
continuation. There is no requirement for a scheduler, universal workspace model, global client
registration, resource cap or healthy-command timeout.

Coverage includes `harness.py`, `runs.py`, `workspace_env.py`, `verify.py`, `freshness.py`,
`docs.py`, `surrealdb_fixture.py`, `compile_profile.py`, `worktree.py`, their changed focused
controls, task guidance and the current owners. This review does not qualify product extraction,
semantic analyses, real-library compilation, operator databases, heldout evaluation or production
serving. The operator explicitly excluded comprehensive repository testing, whole verification
families and `qualify`; acceptance here concerns the changed functionality and its first-principles
value. Existing unrelated product failures remain outside this decision.

## 2. Responsibilities and domain meaning

| Owner | Decisions and authoritative operations | Consumers and change boundary |
|---|---|---|
| `harness.py` | Direct process-group ownership, unreaped exit observation, cleanup observation, file display and selective log reading | Run, verification and fixture owners reuse mechanisms; no product verdict is assigned here. |
| `runs.py` | Durable run identity, actual child result, supervisor/launch failure, cleanup certainty, cancellation, retention and selected recovery | CLI observation, verification pointers, profiling and worktree protection consume this meaning. |
| `workspace_env.py` | Requirement-scoped environment ownership, readiness and explicit publication | Verification discovery/execution and managed preparation use the same shared/exclusive boundary. |
| `verify.py` | Selected boundary execution and outcomes, summary and rerun selection | Reuses lifecycle/environment owners; `summary.json` is not a second process-identity authority. |
| `surrealdb_fixture.py` | Disposable server/attachment ownership, scope admission, server-end policy, diagnostics and invocation-scoped native MCP | Direct CLI and verification share policy; fixture inventory governs downstream protection. |
| `freshness.py`, `docs.py` | Read-only generated-output observations; captured publication inputs and their receipt | Existing native generators remain owners of output. A receipt certifies input agreement, not rendered semantic correctness. |
| `compile_profile.py`, `worktree.py` | Capture completeness and checkout removal, respectively | Neither may reinterpret an exited process as confirmed cleanup or erase another owner's records to manufacture completion. |

This small model is adequate without a new class hierarchy: execution, launch, supervision,
cleanup, readiness, selection and publication are different phenomena, governed by actual
functions and persistent observations. In particular, `cleanup_of`, `recorded_group`,
`server_end_outcome`, `capture_completeness` and `live_state` drive decisions rather than merely
adding domain labels to output. F01 and F02 expose where those distinctions initially stopped
short of their operational authority.

**CI fact/fidelity applicability.** No extracted code-fact family, canonical graph identity or
analysis algorithm changes. The relevant evidence distinctions are:

| Evidence | Provider / fidelity | Coverage and identity | Consumer limit |
|---|---|---|---|
| Child exit and cleanup | Native process observations; separately recorded results | Run/fixture identity, boot/start identity and observed group members; unknown remains unknown | Exit does not prove descendant cleanup or product success. |
| Readiness | Requirement-scoped observation under ownership | Selected checkout/environment and requested capabilities | Advisory printing is not admission; verification does not prepare. |
| Freshness | Captured bytes/context and derived equality | Complete declared publication inputs, tool identity and receipt integrity | Clean means agreement with the checked inputs; no renderer, link or product-quality guarantee. |
| SQL diagnostic | Native transport/envelope and indexed statement outcomes | Explicit live disposable or checked retained scope | Successful requested results remain data; error bodies, SQL and credentials are omitted. |
| Native MCP | Installed native server/client interfaces and executed protocol observations | Invocation-only disposable synthetic/non-sensitive attachment | Native messages are unsanitized; administrative transport is not tenant isolation. |
| Retrospective | Partial native-command trace and manual/classifier interpretation | Attributed window and examples | Neither a complete friction census nor measured effectiveness. |

## 3. Contracts and composition

`SpawnGuard` owns every successfully spawned direct group before fallible identity/receipt work.
`observe_exit` uses `waitid(...WNOWAIT)` so the leader remains waitable until cleanup completes;
loss of that pin refuses numeric signaling. Actual child return code survives cleanup or recording
failure. Persisted historical recovery instead uses recorded boot/start/member identities;
absence of usable authority is a refusal, not permission to signal a reused PID. Schema-1 run
observation remains read-only. Explicit selected recovery upgrades one record under its owner
lock, preserving original and unknown fields and captures. This is a current consumer of retained
records, rather than a general compatibility subsystem.

Run/verification children write directly to owned full files. `FileDisplay` is a separate,
detachable process with a bounded block and no output queue; a blocked or broken terminal cannot
hold the command's supervision loop. Its own shutdown is independent of the product process
group. `tail_bytes` searches backwards for the requested suffix; a very large final line requires
reading that line, while a small tail of ordinary logs avoids complete-file hydration. Follow
uses byte offsets and handles truncation/replacement without making display authoritative.

Managed discovery acquires the same shared environment ownership as execution before readiness
and collection; collection cannot import a partially published extension. A queued exclusive
writer re-observes after acquisition and retains ownership through preparation/postvalidation.
Independent readers overlap, nested ownership is reused, and pure Rust selections avoid an
unneeded Python environment lock. Readiness never synchronizes. `LaunchResult` distinguishes an
actual launched exit 127 from `OSError` launch failure. Fixture policy distinguishes evidenced
substrate blockage, unknown server failure, actual child result and intentional cancellation.

Hakari observation runs in an owned copied metadata workspace, with offline inputs and escape
checks, rather than changing the live lockfile and attempting rollback. Documentation publication
stages the bytes it fingerprints and rechecks before replacement. The schema-2 receipt digest
rejects accidental deletion/tampering of its dynamic-input inventory; it is integrity checking,
not authentication. The current freshness path does not execute a publisher or tool just to
report status. Conservative hashing/copying can invalidate more than strictly necessary, but is
credible for these explicitly invoked repository-sized observations; it is not introduced on
every product request or as another artifact cache.

SQL diagnostics reuse native `/sql`, one indexed result per statement and checked scope, without
inventing a query language or parsing prose into causes. Native MCP composes existing disposable
fixture lifecycle with installed Codex invocation overrides and child-only authorization. Every
native tool call must explicitly supply namespace/database. Existing same-user runtime-directory
and bus-socket discovery supplies only missing task-local systemd routing; it creates no session,
service or global configuration. This preserves native concurrency and intentional selections.

CI projection/analysis record columns are not applicable: these changes neither introduce an
analytic nor transform a code-intelligence graph. Importing those product mechanisms into this
tooling boundary would add machinery without a consumer.

## 4. Expected changes and failure scenarios

| Scenario / kind | Governing boundary and propagation | Assessment and evidence strength |
|---|---|---|
| Replace the live observer / mechanism substitution | Change file display behind owned logs; run/verify supervision and child grammar remain unchanged | Implemented separation and real blocked/broken-sink controls support replacement without another execution policy. |
| Add a native fixture diagnostic / domain extension | Extend checked scope and structured diagnostic result in fixture owner; CLI renders it and verification reuses server-end policy | Existing SQL/native MCP composition is concrete evidence. A new diagnostic need not edit run outcomes or add a server. |
| Change a publication input / binding or dependency change | Captured input discovery/receipt owns invalidation; staged renderer uses those bytes | Dynamic asset, partial-receipt and during-render drift controls exercise the important boundary. Whole-repository mutable state is not silently declared exact. |
| Recover a selected legacy run / policy-preserving recovery | Runs owns lock, original observations and recovery with identity checks; profiling/worktree consume resulting cleanup | Read-only legacy observation and explicit selected upgrade preserve captured evidence and refuse foreign/reused identity. |
| Owner exits while fixture command survives / recovery failure | Fixture must preserve command intent/identity/outcome, freeze writers, recover under locks and publish protection to worktree | F01; deleting a dead server record or worktree cannot substitute for command cleanup. |
| Leader exits and its PID could be reused / ownership failure | Direct guard retains waitable leader through group cleanup; lost pin refuses signaling | F02 correction independently inspected, with real leader/descendant and unrelated-group controls. |
| Ask native MCP to handle sensitive or retained content / unsupported extension | Reject routing at the invocation boundary; independently useful checked SQL diagnostic remains available | Honest limit. Support would require a new protection/design decision; successful HTTP fallback is not native MCP acceptance. |
| Grow logs or run parallel tasks / execution growth | Direct file writes and suffix access avoid full-log mirroring; requirement locks preserve independent readers and ordinary native workers | Credible physical route for this workload. No global scheduler/cap or claimed throughput measurement. |

These routes localize changes by meaning. They do not promise every extension is one file or
that a new semantic concept requires no contract change. Administrative native MCP credentials
also remain capable of more than a default header scope; the disposable/non-sensitive support
limit is therefore necessary, rather than a documentation substitute for a security boundary.

## 5. Findings

The [active plan §8](../../plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner)
owns current execution disposition. The following are stable findings from this review, distinct
from identically numbered findings in the source follow-up and plan-authoring reviews.

<a id="F01"></a>

### F01 — Fixture recovery authority stops short of record deletion and concurrent writers

**Diagnosis, Implemented / source-inspected, 2026-10-09.** `run_attached` owns a command group
separate from both the outer launcher and native server. Initially, failed/unknown command cleanup
was merely reported; unconditional attachment release, server destruction and sweep could erase
the remaining authority. Dead run-owner/ended kept-server fixtures were also missed by the actual
worktree-removal consumer. A surviving command could therefore lose both selected recovery and
checkout protection despite an unsuccessful cleanup observation.

The first correction persists prelaunch intent, actual child outcome/leader and command cleanup,
installs immediate in-memory protection, and preserves unresolved state across release/destroy/
inventory. Selected recovery takes per-fixture exclusive ownership plus every attachment lock,
rereads records and refuses held locks even with `--force`. Worktree protection now precedes
server/owner liveness. These are necessary parts of one recovery contract, not optional features.

The first correction did not cover all mutation consumers. The subsequent reread found automatic
run recovery before lock acquisition, kept attachment deletion without equivalent exclusion,
incomplete-directory deletion from an age/`lock_held` snapshot, and an open/flock stale-inode race.
Those paths could still race selected recovery, erase unreadable evidence or create an attachment
against a deleted fixture directory. Passing earlier controls did not close these instances.

**Dated closure assessment, 2026-10-09:** `_current_lock` checks the acquired inode against its
current path, and `_current_fixture` also revalidates the exact server record. `Server.destroy`
holds fixture exclusive ownership through final deletion and obtains/validates every attachment's
exclusive lock before fresh reads, recovery writes or removal. Automatic run sweep now delegates
to that operation. Kept sweep holds fixture shared ownership plus each selected attachment's
exclusive ownership and rereads its receipt. Incomplete sweep removes only an old lock-only
pre-record creation under current exclusive ownership; unreadable receipts and attachment evidence
remain protected. `--force` does not bypass held locks or unavailable/reused identity.

The correction was also challenged by ordinary healthy teardown: a direct fixture context already
owns its attachment lock. Reacquiring it through a separate descriptor initially refused its own
teardown in actual native controls. The final context releases its own attachment under its
still-held server exclusive ownership before destruction; unresolved cleanup retains records and
protection rather than deleting attachment state. External owners retain the same refusal contract.
Legacy idle attachments remain idle, while unidentified commands remain protected.

Independently inspected closure evidence includes real surviving groups with refused cleanup,
identity/initial-receipt exceptions and selected retry, foreign/reused actual-group refusal,
live-owner exclusion, changed receipts after inventory, stale lock/record replacement, unreadable
evidence retention and downstream actual worktree preservation. The final 20 selected controls
include native diagnostics, ordinary context teardown and interrupted native client cleanup.
This closes F01 within the reviewed lifecycle routes; it is not general product qualification.

**Principles and judgment:** FP-02/04/05/06; DP-02/03/04/19/21; A2/A3; G2/G5/G7.

<a id="F02"></a>

### F02 — Reaping the direct leader drops numeric process-group authority before cleanup

**Diagnosis, Implemented / source-inspected, 2026-10-09.** The previous `poll`/`wait` loop reaped
the group leader before `SpawnGuard.cleanup_group`. Once the original group emptied, rapid PID/
group reuse could make a fresh membership sample look like an owned group. Direct parenthood at
spawn did not establish authority over that replacement. This was a material identity gap, even
though the reuse window is narrow.

**Dated closure assessment, 2026-10-09:** shared `observe_exit` leaves the exited leader waitable
with `WNOWAIT`; `SpawnGuard` is its sole reaper, cleans while identity is pinned, and reaps only
after confirmation. Already-reaped leaders and `ECHILD`/observation errors refuse numeric cleanup.
Runs and fixture command loops use that observation and preserve `guard.returncode` on exceptions.
The correction uses the existing direct-owner boundary rather than pidfd/framework machinery.

Independently inspected controls use actual positive/signal exits, repeated waitid observation,
an exited leader with a real descendant, and a real unrelated process group after both `Popen.wait`
and external `waitpid` consume the pin. The run owner also asserts that its leader is still
waitable at cleanup. This closes the diagnosed direct-owner premise; it does not prove arbitrary
historical process recovery, which retains its separate recorded-identity refusal contract.

**Principles and judgment:** FP-02/04/05; DP-03/04/19; A2; G3/G5.

## 6. Library fit, alternatives and execution tradeoffs

The implementation reuses Python's native subprocess/process/file/lock facilities, native Cargo/
Hakari generation, existing documentation engines, SurrealDB HTTP and installed Codex HTTP MCP.
The shared file suffix helper and direct-process guard supply real missing contracts; they do not
justify a generic filesystem or task framework. This review inspects their executing composition,
not an unbounded claim about every feature of those tools. No dependency version change or new
library adoption is necessary for the corrections.

| Alternative | Benefit and burden | Target judgment / revisit condition |
|---|---|---|
| Native ephemeral handles alone | Least machinery for one active session; insufficient for selected cross-session recovery and retained profiling | Keep as ordinary first choice; durable run owner remains optional and useful. |
| Synchronous display or a writer thread | Small implementation but a blocked sink can retain the supervisor or an unjoinable thread | Separate detachable display fits the stated disappearing/blocked observer better. |
| Persistent scheduler, MCP bridge or global client registration | Could centralize many operations but introduces a service, mutable registry and another lifecycle | No supported consumer warrants it; invocation overrides and existing owners suffice. |
| Timeout/caps/global serialization | Can force some cases to end but change native command behavior and useful parallelism | Does not repair identity, cleanup evidence or publication ownership. No new cap is recommended. |
| Live regeneration with rollback | Avoids scratch preparation but races concurrent edits and can overwrite work | Owned copied inputs plus equality observation is the simpler trustworthy read-only contract. |
| Existing owner-local recovery fixes | Some lock/receipt code, but decisions stay with the owner that can validate identity and preserve records | Preferred. F01 must include deletion paths so the extra code buys actual recovery safety. |

FP-01/03/06/07 and DP-08/09/10/13–18/21–24 are satisfied for the examined routes: ownership
and policy remain local, native capabilities compose, effects and evidence limits are explicit,
and physical work matches large-log/concurrent-task needs. FP-02/04/05 and DP-01–05/19 are
satisfied after the F01/F02 corrections: lifecycle distinctions govern deletion and recovery as
well as display. DP-06/07/11 are satisfied at the scoped structured records and
indexed native result boundary; DP-12's recursive-analysis semantics are not applicable.
The DP-20 no-cap conflict is deliberate operator scope: coordination through meaningful ownership
is retained, without a new agent worker/resource limit. It creates no evidence for throughput or
resource capacity. CI-01/02/04/10/13's attribution, uncertainty and pinning concerns apply only to
the observed tooling boundaries; product-specific graph/analysis rules are not newly certified.

## 7. Evidence and uncertainty

All execution results below are **Tested, attributed, 2026-10-09**, rather than reviewer reruns.
The reviewer read decisive production paths and focused test bodies. Reviewer execution of tests,
product families and comprehensive qualification is **not_run**, by the assignment's boundary.

| Command / evidence | Outcome and exact scope |
|---|---|
| `just run --background --label agent-followup-affected-controls -- uv run --no-sync pytest tests/scripts/test_harness.py tests/scripts/test_runs.py tests/scripts/test_verify.py tests/scripts/test_workspace_env.py tests/scripts/test_compile_profile.py tests/scripts/test_worktree.py tests/scripts/test_docs.py tests/scripts/test_freshness.py tests/scripts/test_surrealdb_fixture.py -ra -o addopts=''` | **passed**, 313 affected controls in 52.47s. Reviewer directly read `build/runs/20261009T231848.005Z-9bf4ac/{record.json,output.log}`: child exit 0 and confirmed cleanup. This precedes the last F01 fixture corrections and later publisher type narrowing. |
| `uv run --no-sync pytest tests/scripts/test_harness.py tests/scripts/test_runs.py -ra -o addopts=''` | Lifecycle executor reports **passed**, 85 controls, including the real unreaped-leader controls inspected here. |
| `uv run --no-sync pytest tests/scripts/test_verify.py tests/scripts/test_workspace_env.py -q -o addopts=''` | Integrator reports **passed**, 70 controls, including discovery ownership, launch errors and direct blocked/broken 2 MiB sinks. |
| `just run --background --label agent-followup-verification -- just verify --select tooling:python --pytest-args '-k "test_listing_observes_and_collects_under_real_ownership or test_actual_child_exit_127_is_failed_not_launch_failure or test_verify_live_sink_cannot_stall_capture_or_child_completion"'` | **passed**, 4 selected controls. Reviewer directly read `build/runs/20261009T230142.374Z-18e4c0/{record.json,summary.json,verify/tooling-python.log}`: child exit 0, schema-2 cleanup confirmed, boundary passed. Launch and eventual result are separate. |
| `just verify --list --select tooling:python --pytest-args '-k test_listing_observes_and_collects_under_real_ownership'` | Integrator reports **passed**, one selected node ID. Clearing collection addopts corrected a discovered double-quiet false empty result. |
| `just docs-test` | Integrator reports **passed**, 78 publisher/resolver controls, including schema-2 dynamic-inventory integrity. Reviewer read the tamper and during-render input-drift controls. |
| `uv run --no-sync pytest tests/scripts/test_worktree.py -q -o addopts=''` | Integrator reports **passed**, 13 controls. Reviewer read actual worktree preservation with protected dead-run/kept/incomplete fixture projections and owner refusal. |
| `uv run --no-sync pytest -o addopts='' -q tests/scripts/test_surrealdb_fixture.py -k 'sweep_removes_only or dead_run_owned_fixture_terminates or previous_boot_launch or launcher_sigkill or failed_fixture_cleanup or force_recovery or recovery_locks or native_mcp_launcher or disposable_diagnostics'` | Fixture executor reports **passed**, 12 selected, 29 deselected. These controls preceded the final F01 reread gaps and do not close those gaps by passing. |
| `uv run --no-sync pytest -o addopts='' -q tests/scripts/test_surrealdb_fixture.py -k 'incomplete_sweep or kept_sweep_rereads or run_sweep_recovers or stale_fixture_lock or sweep_removes_only or dead_run_owned_fixture_terminates or recovery_locks_freeze or previous_boot_launch or kept_attachment_with_live or failed_fixture_cleanup or force_recovery or native_mcp_launcher or disposable_diagnostics or unknown_native_server_end'` | Fixture executor reports **passed**, 20 selected, 28 deselected in 11.27s, after final F01 corrections and the ordinary-teardown regression fix. Reviewer independently read the final source and revealing test assertions. |

The inspected disposable diagnostic control actually starts native SurrealDB, exercises schema/
index/content, syntax/runtime/mixed failures, checked/incorrect scope, native HTTP MCP handshake/
tool operations/auth refusal and cleanup. Installed Codex app-server separately initializes and
discovers that invocation's tools; launcher interruption tests remove the child fixture and do not
persist transport auth in argv/records. This is useful native composition evidence. It is not a
model-mediated production query, sanitized native-error guarantee, sensitive-content qualification
or operator adoption. Exact control assertions provide evidence; no retained external raw protocol
capture or performance experiment is claimed.

The source retrospective's 22,211 completions/115 threads, 1,727 nonzero results, nine malformed
lines and partial last day motivate concrete examples only. Classification false positives and
independent code edits preclude failure-rate or same-code causal improvement claims. Native Rust/
Codex guidance similarly distinguishes metadata, configured clients, cold references, active
connections and exercised capabilities; it does not claim warm whole-workspace semantic health.

## 8. Independent gates, rule impacts and architectural judgment

| Gate | Verdict | Scoped evidence / required action |
|---|---|---|
| G1 Authority | pass | Existing owners govern run, environment, native scope and publication observations; no competing receipt store. |
| G2 Semantic fidelity | pass | Child/launch/server/cleanup distinctions govern outcomes and retention; final F01 correction preserves the evidence needed for recovery. |
| G3 Validity | pass | Scope and input receipt admission, structured launch errors and F02's refusal on lost leader pin. |
| G4 Hidden behavior | pass | Readiness/discovery do not sync; freshness does not mutate live inputs; native child configuration is invocation scoped. |
| G5 Consistency and recovery | pass | F01/F02 corrected and independently reread; unresolved cleanup remains protected, and recovery/deletion exclude current writers. |
| G6 Transformation and reuse | pass | Shared suffix/policy/ownership helpers, complete-input receipt integrity and conservative invalidation. No graph transformation changed. |
| G7 Truthful capability claims | pass | Native MCP/content limits and evidence labels are honest; lifecycle observations distinguish completed execution from confirmed cleanup. |
| G8 Library leverage | pass | Existing native tools and standard primitives implement the needed contracts without another service/framework. |
| CI-G1 Fidelity | pass, scoped | Tool observations retain uncertainty and provenance; no product-code fact fidelity claim is accepted here. |
| CI-G2 Synthesis | n.a. | No brief/assertion synthesis changed. |
| CI-G3 Evaluation | n.a. | Heldout/gold/evaluator inputs and policies are untouched. |

The recommendations do not introduce further workflow authority or process prescriptions. F01
and F02 finish the accepted lifecycle contract in its existing owners; ADR-0142/§1.2 remain the
decision route if that contract changes. The plan's §8 owns remediation; this document retains
the dated diagnoses. No execute-plan JavaScript/runtime rule is recommended: T2 now deliberately
has no process-skill change. A future sensitive/retained native MCP consumer, repeated unavailable
semantic-tool capability or inability to select existing complete receipts would reopen only its
named capability decision, rather than mandate a startup audit or new middleware.

| Judgment | Verdict | Architectural reason |
|---|---|---|
| A1 Localize change | satisfied | Observer replacement, native diagnostics and publication inputs have coherent owner-local change routes and actual consumers. |
| A2 Encode domain meaning explicitly | satisfied | Adequate lifecycle/scope/input distinctions govern actual execution, deletion and recovery after F01/F02 corrections. |
| A3 Extend through composition | satisfied | Native diagnostic/client composition and every reviewed sweep/removal path reuse the relevant ownership authority. |
| A4 Fit execution to workload | satisfied, scoped | Direct file capture, selective suffix reads, meaningful reader/writer ownership and native invocation preserve useful parallel work without unnecessary full-log hydration, global services or mandatory preparation. |

**Bounded decision: Accept scoped.** The named mechanisms are Implemented, independently
source-inspected and focused-Tested under the conditions above. F01/F02 are corrected rather
than excluded from supported behavior. The design offers sufficient qualitative benefit for its
local ownership/receipt cost; no numerical effectiveness result is needed to recognize the
specific removed failure mechanisms. Sensitive/retained native MCP remains explicitly unsupported,
with a new protection/design decision required before extending that capability.

**Enclosing architecture:** the named agent-execution scenarios are assessed here; full product
architecture, production qualification, quantitative performance and operator adoption are not.
The next step belongs to the integrator: reconcile the plan's sole disposition table, retain the
named receipts and complete applicable scoped maintenance/publication. No further architectural
mechanism or general repository test campaign is required by this review. Reopen the affected
boundary if a new removal consumer bypasses owner protection, supported content exceeds the native
MCP limit, or a concrete workload defeats the stated physical route.
