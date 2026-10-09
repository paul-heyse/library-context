# Agent effectiveness follow-up: independent design and target review

**Date:** 2026-10-09, America/New_York. **Tier / purpose:** design / target.
**Principal reviewer:** fresh delegated design-reviewer, not an implementation author; coordinator publishes the returned assessment.
**Decision: Revise.**

The workspace already gives capable agents useful operations: explicit preparation, native-tool
argument forwarding, composable verification selections, disposable native fixtures, durable run
handles, optional checkouts and scoped maintenance. Those are the right foundations for a personal
project using Codex primarily and Claude occasionally. The best next design keeps these operations
and makes their boundary behavior dependable. It does not add another scheduler, task registry,
command interpreter or mandatory startup procedure.

Five source-inspected defects remain. Run ownership does not cover ordinary exceptions after
launch or accurately preserve unsuccessful cleanup. Terminal output can block supervision.
Environment observations and discovery occur outside the ownership that is supposed to protect
managed consumers. Small log requests read complete logs. Finally, verification interprets any
exit 127 as proof that a command could not launch; direct fixture and family paths also classify
the same server-exit observation differently. These defects undermine readable recovery and
local reasoning even when normal commands produce correct results.

There are also useful **Proposed** additions: fixture-scoped schema/query/EXPLAIN inspection,
native SurrealQL syntax validation, readiness-qualified Rust semantic navigation, selective
structured receipt access, and short examples of the existing command compositions. The command
retrospective supports these opportunities without proving a productivity gain or a wrapper cause
for every failed invocation.

This review authorizes no implementation or configuration change. Its dated assessment is retained;
F01–F05 and selected opportunities now have their sole scheduled disposition in the
[follow-up plan §8](../../plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner).
That plan also receives AE-24; the existing workspace and testing plans retain their other
finding IDs, ownership and historical receipts.

**Subsequent implementation:** the [plan §9](../../plans/agent-effectiveness-followup-plan_2026-10-09.md#9-execution-checkpoint)
records focused acceptance of the corrections and selected capabilities on2026-10-09.
This source review's verdict remains the dated pre-remediation assessment.

## 1. Scope, functional target and evidence boundary

| Field | Review contract |
|---|---|
| Baseline | Assigned `d2341cc4`; HEAD advanced to `6c9755226a2c6960e4680a9db644e52f2abb3808` during concurrent product work. Source observations are inspection of the identified mixed tree on 2026-10-09, not clean-revision qualification. |
| Standard | Repository core/template 3.3, efficient-architecture heuristics 1.0, declared code-intelligence profile/review additions 1.5 and [library-context binding](../design_principles/binding/library-context.md), loaded through [standard.toml](../design_principles/standard.toml). |
| Functional intent | Reduce thought constructing commands and environment complications; parameterize related work; make outcomes and repair legible; preserve native tools and intentional overrides; provide useful capabilities as well as remove friction. Current user intent supersedes the previous plan's equal-runtime weighting and repository-only recommendation scope. |
| Subject | Agent-facing repository, workstation/user configuration, current commands/custom CLIs, native tools and capability composition; adjacent product consumers only at their workspace invocation, inspection and lifecycle boundaries. |
| Workload premise | One operator, Codex primary, limited Claude; a handful of concurrent tasks; short source queries and long builds/controls; mutable checkout and native extension; potentially large logs; observers may disappear or stop reading; optional worktrees and separately locked services. |
| Exclusions | Product semantic correctness, full test coverage acceptance, production performance, operator stores/registrations, installations/synchronization, builds/tests/pilots and remediation. No new resource, thread, agent-concurrency or healthy-duration caps. |
| Evidence method | Static source/type inspection, current owners, existing receipts, version-scoped capability research and selected original command events. No new workload or disposable probe was needed for the diagnoses. |

**Standards conflict:** core DP-20 contains resource-budget requirements. The user's explicit
exclusion of harness caps governs this review. Permitted resource use is not a defect. Semantic
synchronization, cancellation/drain, deterministic behavior and truthful outcomes remain applicable;
this review proposes no reduction of test parallelism or workload duration.

The [retrospective](../evidence/2026-10-09_agent-effectiveness-followup/retrospective.md) freezes
October 6 00:00 EDT through October 9 14:45:09 EDT, excluding this review root and descendants.
Its 22,211 native completions in 115 attributed Codex threads include 1,727 nonzero exits.
These are recorded historical counts, not a validated failure census or productivity metric.
Classification has documented false positives, nine malformed JSON lines were not recovered,
October 9 is partial, and no comparable Claude corpus was assembled. Successful log readers,
launchers and producers do not establish the underlying operation passed.

The [Codex/Rust](../evidence/2026-10-09_agent-effectiveness-followup/capabilities-codex-rust.md)
and [SurrealDB](../evidence/2026-10-09_agent-effectiveness-followup/capabilities-surrealdb.md)
briefs qualify installed/documented/configured/exposed/exercised boundaries. Original U1/U2/U3,
E1, T1/T2/T3 events were inspected at their stated physical JSONL lines; published excerpts keep
private configuration out of the repository. The corpus-intelligence October 8 review supplied
failure hypotheses, whose decisive library-context source was read independently. Its probe
receipts do not transfer here.

## 2. Responsibilities and the governing workspace model

The consequential domain is executing and understanding an operation in a selected context.
It needs small explicit contracts, not a universal workspace ontology. Checkout/environment,
command argv, verification selection, run attempt, native child exit, stored receipt, current
liveness, cleanup observation and product outcome answer different questions. Fixture substrate,
fresh attachment state and retained served content also have different identities and lifetimes.

| Owner | Responsibility and consumer contract | Expected reason for change |
|---|---|---|
| [justfile](../../../justfile), [build_environment.py](../../../scripts/build_environment.py) | Shortcuts and explicit normalization; native arguments retain their native interpreter. Bare Cargo remains first-class. Foreign environment/target overrides have explicit routes. | A useful composition or checkout/environment binding. |
| [workspace_env.py](../../../scripts/workspace_env.py) | Effective environment/extension identity, read-only readiness, explicit preparation, requirement-scoped shared/exclusive ownership and nested reuse. | Environment publication, readiness or coordination semantics. |
| [verify.py](../../../scripts/verify.py) | One boundary/step definition drives selection, help, preview, discovery, execution, summary and rerun. | Verification meaning, native selection or a newly needed composition. |
| [runs.py](../../../scripts/runs.py), [harness.py](../../../scripts/harness.py) | Durable attempt, owned process group, output file, observation, cancellation and retention. | Lifecycle/receipt/cleanup or presentation contract. |
| [surrealdb_fixture.py](../../../scripts/surrealdb_fixture.py) | Native server substrate, per-command mutable namespace/configuration and identified retained serving content. | Substrate, attachment or content lifecycle. |
| [worktree.py](../../../scripts/worktree.py), [maintenance.py](../../../scripts/maintenance.py), [freshness.py](../../../scripts/freshness.py) | Optional isolated revisions, preservation-aware maintenance and generated-output observations. | Checkout/preservation/freshness behavior. |
| [.codex/config.toml](../../../.codex/config.toml), [.claude/settings.json](../../../.claude/settings.json), shared roles | Runtime defaults and adapters over shared responsibility contracts. Session choices take precedence. | Concrete runtime availability or settings discrepancy. |
| Shared capability skills and installed native tools | Version-qualified library/tool functionality, with repository wiring separate from reusable documentation. | A consumed capability, version or adapter gap. |
| [lctx CLI](../../../crates/lctx/src/main.rs), native serving/compilation owners | Product requests, immutable snapshot tools and explicitly configured effects. Harness receipts do not redefine product success. | Product contract and its invocation/diagnostic consumers. |

Dependency direction is mostly sound: recipes bind operations; verification composes declared
steps; fixtures supply context; native tools own argument grammar; run observation reports execution
without owning product meaning. The gaps below occur where an observation is consumed as a stronger
premise than its owner established.

### Profile applicability and fidelity

The code-intelligence profile was loaded. Product extraction, analysis graphs, served answers and
protected evaluation are not certified by this review. Relevant scoped information is:

| Information family | Provider and fidelity | Coverage, identity and consumer |
|---|---|---|
| Historical command observations | Native completion events and original tool outputs; extraction counts, heuristic search classifications | Frozen window/thread/item/line identity; incomplete census; review evidence only. |
| Capability assertions | Installed help/versioned primary sources and local source; Interface-checked | Only named versions/interfaces; declaration is not live exposure or exercise. |
| Run/verification observations | Schema-1 records, native exits and derived liveness | Wrapper attempt and selected controls; product outcome is separately owned. |
| Rust semantic navigation | Installed rust-analyzer-mcp 0.4.0 and its source | Source/position/workspace loading affect interpretation; cold empty references are inconclusive. |

CI-01/04/06/11 apply to the extracted/heuristic observations and interpretation of semantic-tool
answers at this review boundary. Their evidence/fidelity properties are judged below for the
examined retrospective, rather than waived solely because the product is excluded. Actual adapter
semantic results were not exercised or accepted. This is not acceptance of the enclosing
code-intelligence product.

## 3. Contracts and preservation constraints

**Implemented, source-inspected 2026-10-09:** `Boundary`/`Step` and resolved selections in
`verify.py:65–125,737–858` keep family names, tool arguments, requirements and command plans
together. `--print` and compiling `--list` are explicitly different. Native package/target
selection and filter syntax should remain usable; avoid adding a second Cargo/nextest parser.

**Implemented, source-inspected:** `workspace_env.py:273–370` scopes locks to actual environment
and extension paths, reports waiting holders, permits overlapping readers, gives pending writers
a gate and reuses a live ancestor's ownership. Pure Rust work takes no Python ownership. These
are semantic safeguards, not build serialization. Keep explicit preparation and unmanaged override
freedom, and do not move synchronization into verification.

**Implemented, source-inspected:** run output goes directly to an owned file (`runs.py:329–340`);
native runtime continuation comes first; records support cross-session observation. Unique attempt
directories, process identities and owner locks are useful. Preserve direct log ownership when
changing live presentation. A native process handle, a code-mode cell ID and a durable run ID are
separate handle namespaces.

**Implemented, source-inspected:** attachments create fresh namespaces and `core`, `compiler_cache`
and `compiler_products` databases (`surrealdb_fixture.py:869–925`). `--attach ID` reuses server
substrate, not a prior command's database. Retained serving content has checked configuration and
identity (`:974–1100`). Native SQL/SDK/MCP diagnostics must select the actual producing scope and
hold its lifetime. Printed credentials or a permanent operator connection are unnecessary.

**Implemented, source-inspected:** scoped maintenance uses named Rust files with `--skip-children`
and reports skipped generation. Worktree carry is explicit and preserves staging/dirty ownership;
removal checks live managed effects. These boundaries matter in the current mixed tree and must
survive any simplification.

The [workspace plan](../../plans/agent-workspace-effectiveness-plan_2026-10-07.md#9-finding-dispositions)
routes previous findings; AE-24's current disposition has transferred to the follow-up plan §8.
The source defect is that freshness does not cover documentation publication,
and Hakari can rewrite a stale lockfile. `freshness.py:101–115` detects a changed `Cargo.lock` only
after calling Cargo, without preventing the write; it can still report `clean` with that detail.
The read-only contract therefore remains unfulfilled. Correct the invocation or isolate its mutable
metadata, without rolling back concurrent edits. This is the existing AE-24 obligation, not a new
finding ID. AE-12/AE-25 coverage work stays with the testing architecture owner. Deferred loaded
extension `build_identity()` remains distinct from on-disk freshness; no new import gate follows.

## 4. Composition and execution fit

For the supported workload, the current physical route is credible in normal execution: native
tools run directly, one durable owner records a long attempt, independent controls retain per-boundary
logs, and disposable fixtures supply state only where required. This avoids a daemon and keeps native
override routes visible. The source defects concern concrete growth and failure cases:

- A supervisor must keep observing/cancelling children when a receipt write or terminal sink fails.
- A twenty-line log request should not hydrate every byte of a long compile log.
- A managed importer needs readiness and import protected by the same publication ownership.
- A launched command's exit code cannot establish that no command was launched.

These are qualitative A4/A2 judgments, not speed measurements. H5/H6/H18/H19/H23/H25/H26 support
selective observation, lifetime-aligned readiness and proportional failure handling. Short chunks,
offsets or an incremental log reader describe the observation algorithm; they are not workload,
memory, thread or duration caps imposed on agents or tests.

The source count of the harness is not itself a defect. Keeping one small shared execution outcome
contract and using established subprocess/flock/file primitives is preferable to splitting the same
policy across more services. Likewise, reusing the existing server `/sql` or `/mcp` for diagnostics
can add an ability without another datastore or SDK bridge.

## 5. Change and failure scenarios

| Scenario / kind | Owning change and expected propagation | Observed route / evidence |
|---|---|---|
| Add a focused check across two existing families; composition | One selection binding and genuine new boundary semantics only; native flags remain native. | Existing repeatable `--select` and tool-scoped arguments support it. U2 shows the need to distinguish repeatable packages from global `--tests`, not a new Cargo grammar. |
| Add fixture schema/query-plan inspection; domain operation | Consume identified fixture/content scope and operation/query inputs; return structured statement outcomes. Fixture lifetime/configuration remains with the existing owner. | Fresh attachment semantics require explicit content scope. Existing `Server.query` checks statement status; native HTTP/MCP capability is qualified by supporting research. Integration remains Proposed. |
| Substitute ephemeral runtime continuation with durable observation; mechanism | Product command is unchanged; existing run owner supplies cross-session ID, logs, exit and recovery observations. | T3's unavailable process handle has unknown cause; existing durable records are the suitable alternative when cross-session survival is needed. |
| Terminal stops reading, or record publication throws after child launch; failure | Run owner retains cleanup responsibility; presentation cannot hold supervision hostage. | `runs.py:319–419`, `verify.py:1150–1174`; F01/F02. No historical occurrence claimed. |
| Managed sync overlaps discovery or queued execution; concurrent binding | Observe/import under resource ownership; recheck after writer acquisition. | `verify.py:1115–1125,1297–1335,1636`; `workspace_env.py:466–484`; F03. |
| Large accumulated log, small tail request; growth | Log reader selects recent bytes/lines without whole-file allocation. | `runs.py:638`, `verify.py:1139`; F04. |
| Launched child exits 127; outcome domain distinction | Launch result stays separate from child exit. | `Runtime.run` returns int, `_run_steps` assumes missing executable at127; F05. |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence and required action |
|---|---|---|
| G1 Authority | fail, scoped | Boundary/step definitions and environment identities have identifiable owners, but F05's direct fixture and verification routes maintain contradictory outcome classifications for ordinary server exit. No second product semantic owner is introduced. |
| G2 Semantic fidelity | fail | F05 conflates failure to launch with a launched child's exit127. F01 terminality does not preserve unsuccessful cleanup in derived state/recovery. |
| G3 Validity | unresolved | F03 permits execution/import based on readiness observed outside managed ownership. Source identifies an invalidating interleaving; it does not establish which package/import failed historically. |
| G4 Hidden behavior | fail, inherited | AE-24's advertised read-only freshness can rewrite `Cargo.lock`; current detection is after the effect. Discovery correctly advertises compilation; this is not itself hidden behavior. |
| G5 Consistency/recovery | fail | F01 exceptions and ignored cleanup results; F02 observer backpressure delays supervision/finalization. |
| G6 Transformation/reuse | unresolved, bounded | Existing rerun preserves selections/profiles and retained serving identity; F03's readiness premise can outlive the state observed. Product cache/transformation qualification remains excluded. |
| G7 Truthful capability claims | fail, scoped | F05 invents launch failure from an exit sentinel; F01 records termination without a dependable cleanup observation. Historical/Proposed distinctions in this review remain explicit. |
| G8 Library leverage | pass, scoped | Native argparse/subprocess/flock/Cargo/nextest and existing fixture mechanisms have real consumers; no clearly substitutable bespoke generic engine was established. F04 needs native or straightforward streaming file access, not a new service. |
| CI-G1 Fidelity | pass, scoped to reviewed observations | Retrospective heuristics are labelled as heuristics, missing/malformed events limit coverage, and successful readers/launchers are not operation passes. Cold semantic emptiness is inconclusive; live semantic answers and product facts remain unqualified. |
| CI-G2 Evidence closure | pass, scoped to reviewed evidence | Selected claims link frozen evidence/original event identity, whose decisive excerpts were read. No product generation or served-claim closure is accepted. |
| CI-G3 Evaluation integrity | n.a. | No gold/heldout/evaluator inputs or qualification changed. |

## 7. Findings and corrective contracts

All diagnoses below are **Implemented source inspected on 2026-10-09**; corrections are **Proposed**.
Current disposition is **Deferred in this review**, reopened when the operator selects harness
implementation planning or the affected consumer exposes the case. No source finding is closed
by accepting its recommendation.

<a id="F01"></a>

### F01 — Ownership stops short of exception-safe cleanup and observed terminality

**FP-02/04/05/06; DP-02/19/21; A2/A3; G2/G5/G7.**

`Owner.run` protects spawn errors but has no encompassing cleanup after successful launch
(`runs.py:319–362`). Child identity capture, record writes, the readiness callback, progress
folding and `_own` can throw while the child exists. The parent-death signal covers leader death,
not arbitrary descendant drainage or an exception caught by a still-live caller. Generic commands
are not required to implement a group-wide SIGTERM handler.

The adjacent direct fixture consumer has the same exposed interval: it launches the child, captures
identity and writes the attachment record before entering its wait/cleanup try
(`surrealdb_fixture.py:1346–1351`); its finally only drains a group when the leader has already
exited (`:1362–1364`). Ordinary exceptions while the leader remains live are not covered by that
finally. Correct this consumer's ownership interval too, reusing the existing group primitives.

After leader exit, `_own` records `survivors_terminated` before signalling and ignores
`signal_group`'s returned cleanup observation (`:381–386`; `harness.py:140–152`). The interrupted
cancel route records `stopped=false` yet writes a terminal record; `cmd_cancel` returns success
for any termination (`runs.py:695–719`). `view` checks survivors only for running/interrupted
states, cancellation declines terminal records and pruning trusts terminality (`:181,678,752`).
A surviving or unconfirmed group can therefore lose its normal recovery route and become prunable.
"Completed" is not test success; the defect is the cleanup/recovery claim and its consequences.

**Correction:** the existing run owner consumes command identity and owns child/group cleanup
from successful spawn through every exit path, independently of receipt/presentation errors.
Retain primary child exit, recording failure and cleanup observation separately. Terminal process
exit with unsuccessful/unknown cleanup must remain visible, recoverable and non-prunable until
resolved. Report attempted signalling as an attempt, not confirmed termination. Preserve parent-death
protection as a backstop; a new task service is unnecessary.

**Revealing legitimate case:** a child succeeds but record publication fails. Do not recast successful
product work as a failed product operation; preserve its exit and report a recording/cleanup gap.
Conversely, preserve the recovery obligation if a generic leader exits while descendants remain.
**Closure:** targeted fault controls after spawn/progress/finalization and an unsuccessful cleanup
observation; inspection of terminal reader, cancel and prune behavior. No new control was run here.

<a id="F02"></a>

### F02 — Live presentation is in the supervision path

**FP-01/02/06/07; DP-18/19; A3/A4; G5.**

`_own` polls the child, then `_drain` synchronously reads and writes/flushes stdout before observing
cancellation/progress (`runs.py:364–399`). A full pipe with a live non-consuming reader blocks;
it does not raise BrokenPipeError. Child logging continues to its file, but supervisory progress
and finalization wait for the observer. A separate cancel command can signal the group, but it cannot
unblock this owner or make it finalize. `verify --live` has a stronger coupling: child output is a
pipe read into the log and terminal sequentially (`verify.py:1162–1173`), so an observer can also
stall child output and log drainage. Other terminal/report writes share the same hazard.

**Correction:** direct log capture remains authoritative; live observation is a detachable/best-effort
reader outside the supervision and child-output path. A blocked/disconnected observer must not stop
cancel/drain or publication of lifecycle observations. Runtime streaming is a viable alternative
for ephemeral work; durable work still uses one run owner. Do not add an unbounded forwarding queue
or demand that healthy commands finish sooner.

**Revealing case:** an observer intentionally stops reading while a valid long command continues.
Keep the full log and command alive while the owner remains cancellable; losing live display is
distinct from losing execution. **Closure:** a non-consuming pipe and disconnected output case
with progress/cancel/terminal observation, plus `--live` log preservation. Source is sufficient for
the diagnosis; there is no local reproduced receipt.

<a id="F03"></a>

### F03 — Readiness and discovery are detached from environment publication ownership

**FP-02/04/05/06; DP-03/09/19; A2/A3; G3/G6.**

`execute` observes all requirements before boundary ownership; `run_boundary` consumes that observation
before entering the shared owner (`verify.py:974–989,1115–1125`). A managed writer can publish between
those steps. `--list` invokes nextest/pytest directly before entering the execution ownership route
(`:1297–1335,1636–1640`). Pytest discovery imports and some selected test binaries use the shared
environment; no fixture is needed to cross that publication boundary. A discovery command is still
a managed reader even when it does not execute test bodies.

`sync` observes before acquiring exclusivity and does not recheck inside it (`workspace_env.py:466–484`).
Two stale observers can queue two full sync invocations, and post-sync observation occurs after releasing
exclusivity. The source establishes needless repeated preparation and a stale-premise window; it does
not establish repeated rebuilding or a specific historical failure.

**Correction:** at the existing environment owner, make the readiness used for execution apply to
the held reader interval. Discovery obtains only the actual resources it imports. A writer rechecks
current state after acquiring exclusivity and establishes its postcondition before release. Advisory
static preview can remain non-owning and explicitly unobserved. This is not serialization of source
editing or pure Cargo work, and unmanaged commands retain their intentional escape route.

**Revealing case:** a pure Rust list must remain independent of Python ownership; a pytest list
must overlap other readers and wait for publication only. A queued second writer should find an
already-current environment without starting another preparation. **Closure:** those interleavings
and source inspection of the readiness-to-use interval. No builds/discovery/sync were executed here.

<a id="F04"></a>

### F04 — Small log observations scale with complete log size

**FP-06/07; DP-10/16/21; A4.**

`runs logs --tail N` reads/splits the complete file (`runs.py:638`), and verification failure feedback
reads/splits its complete boundary log to print the last lines (`verify.py:1139`). Repeated observation
of a long build therefore repeatedly allocates and scans unrelated output. This is source-established
amplification; it needs no latency claim or resource cap.

**Correction:** use a native `tail` where it fits, or seek/read recent blocks with incremental
line handling. Follow retains its offset; full-log requests remain available. Reuse the reader for
verification failure excerpts rather than creating another log store. **Closure:** inspect the
algorithm and exercise small-tail semantics over a large file, partial lines and follow; full output
and explicit requested detail remain intact. No benchmark is required to close the structural defect.

<a id="F05"></a>

### F05 — Failure classification lacks a complete, shared evidence contract

**FP-04/05/06; DP-01/02/21; A1/A2; G1/G2/G7.**

`Runtime.run` returns only an integer (`verify.py:909–913`). `launch` maps OSError to127, and
`_run_steps` classifies every127 as `blocked: cannot start argv[0]` (`:1081–1083,1165–1167`).
A successfully launched program or nested tool can exit127. The current fake test asserts the
sentinel interpretation (`tests/scripts/test_verify.py:282–286`) rather than challenging it.

The direct fixture `_finish` calls every ordinary server end `blocked: readiness`
(`surrealdb_fixture.py:1367–1377`), whereas verification calls that observation `failed` and its
docstring calls any non-OOM end a product failure (`verify.py:931–943`). Both intend to explain an
unexpected server end during an owned command. No distinct cause/contract is supplied to justify
the classification difference. An observed server exit establishes the fixture is unavailable,
not why it ended or which product defect caused it. The mismatch can send an agent down different
repair routes solely because it used a family rather than a bare command under the fixture.

**Correction:** the existing launch boundary returns distinct launch failure and child exit observations,
including a useful redacted cause. Verification classifies blocked only from actual missing
prerequisite/infrastructure evidence; a child127 retains the ordinary failed-child interpretation
unless its own contract supplies stronger evidence. The fixture owner supplies an authoritative
server-end observation and agreed outcome policy to both consumers, preserving the child's own
outcome and unknown exit cause. Do not assert product causality merely from non-OOM. A small result
type/function return suffices; no error framework is needed. **Closure:** missing executable,
non-executable launch and an actual child exit127 remain distinguishable in summary/feedback/rerun;
direct/family commands agree on evidenced OOM and unknown-cause server exit. These cases were not
executed in this review.

## 8. Library fit and useful capability additions

The opportunities below are **Proposed**, with inspected interface routes only. They add abilities
without making every task consult a catalog, open an MCP connection or initialize a semantic server.

| Capability / consumer | Best current route and alternative | Fit, burden and support limit |
|---|---|---|
| Drafted SurrealQL syntax check | Installed SurrealDB3.3 `validate --stdin`; use native CLI rather than custom grammar. | Useful before a query/schema attempt. Syntax validity is not schema/type/runtime qualification. One valid/invalid example would settle usability when authorized. |
| Fixture schema/index/query-plan inspection | Reuse checked `/sql` or SDK envelopes through a fixture-scoped diagnostic command; optional existing server `/mcp` for interactive tools. | Consume explicit fixture/content scope and own client lifetime. Fresh `--attach` does not see prior content automatically. Built-in HTTP MCP uses the running datastore; stdio `surreal mcp` creates a separate datastore. No permanent global connection or remote bridge is justified. |
| Per-statement failure explanation | Preserve statement indexes/statuses and a redacted error category from `/sql`/SDK; native MCP already supplies structured outcomes. | Existing `Server.query` rejects any failed status but keeps only kinds for privacy. Add useful redacted context without dumping bound secrets. MCP mixed statements can have `is_error=false` while `has_errors=true`; CLI JSON strings/exit alone are weaker evidence. Neither route substitutes for native gRPC stream-terminal qualification. |
| Exact Rust semantics and reviewable rename | Current rust-analyzer-mcp; `rg`/ast-grep remain immediate fallbacks. | Adapter0.4.0 lazily starts the server; references forwards immediately, rename waits for loading. Inspected local `src/mcp/server.rs:50`, `handlers.rs:115,221`. Expose/retain loading and health context for reference consumers if the cold-empty gap recurs. No parallel index, mandatory warming or diagnostics build solely to force indexing. |
| Package/target/compiler evidence | Native locked/offline Cargo metadata, nextest list JSON and Cargo diagnostic/artifact messages. | Metadata without dependencies builds nothing; nextest list builds selected binaries. Structured compiler success is not test success. Preserve native options and enabled-version grammar. |
| Selective run-result recovery | Existing run ID, `summary.json`, status JSON and boundary logs, with a small summary selector only if repeatedly useful. | Read the authoritative file/structured channel, return selected outcomes/log pointers and keep full data reachable. No second receipt store. Retain explicit product/wrapper distinction. |
| Settings discrepancy diagnosis | Installed Codex0.162 native strict-config/doctor/MCP configuration diagnostics, scoped to a concrete problem. | Installed CLI/configured tools differ from an already-running session. User/workstation settings are eligible here; choose the actual installed native diagnostic rather than repository validation middleware. Some diagnostics can inspect/connect, so their effect scope matters. |
| Compilation attribution | Existing `compile-profile` capture/status/report and exact measureme readers; Cargo fresh-artifact evidence. | Use retained captures before recording new work. Shared sccache counters cannot attribute concurrent agent activity; incremental workspace crates are not cacheable rustc work. No cache reset, incremental-disable or profiler registry follows. |

The supporting briefs name exact versions, primary-source routes and integration questions. The
review independently inspected the decisive installed adapter sources and captured SurrealDB HTTP
MCP/output sources. Live handshake, permissions, installed-source provenance and fixture attachment
behavior remain unqualified. The proposal can start with the cheaper existing `/sql` operation;
MCP is worthwhile when interactive schema/query-plan consumers justify client/session setup.

## 9. Alternatives and command evidence

| Alternative | Judgment |
|---|---|
| Keep current baseline and add instructions only | Small command examples help U1–U3/T2, but prose cannot repair supervision, stale ownership or outcome conflation. Insufficient for acceptance. |
| Correct existing owners and expose selected native capabilities | Recommended. Preserves native freedom and useful durable/context contracts while removing the demonstrated gaps. No new scheduler or universal model. |
| Use only runtime handles and bare tools | Simplest viable route for ephemeral work and well-understood native commands. Does not replace cross-session records, fixture context or selected verification receipts. Keep it first-class. |
| New workflow engine / permanent MCP bridge / blanket readiness gate | More state, configuration and consumer interpretation without a shown participant/recovery need. Existing native functions and the server route are sufficient candidates. Revisit only for a concrete unmet consumer. |

The recent mistakes are specific, and their corrections should stay proportionate:

- **U1:** `just runs retain RUN --json` failed with `unrecognized arguments: --json`; the same
  retention command without it exited zero. Make per-operation output options discoverable; add
  structured retention output if a real caller needs it, without demanding JSON on every operation.
- **U2:** repeated `--tests` in `cargo check --release --locked -p … --tests -p … --tests` failed;
  native Cargo accepts repeated packages with one global `--tests`. A short native example/metadata
  route suffices. The later composite also edited code, so its zero exit is not a same-code comparison.
- **U3:** `just verify --select leaf:deps --background` failed. The accepted composition was
  `just run --background --label LABEL -- just verify --select leaf:deps`. Put this example and the
  responsible owner in verification help/recovery. Do not duplicate background ownership inside verify.
- **E1:** system `python3` failed parsing project3.14 syntax; `uv run --no-sync python` got past import
  and then failed independently with HTTP400. Current interpreter routing addresses the first case.
  Structured query diagnostics address the second boundary without claiming its unknown cause.
- **T1:** a successful shell producer printed large JSON, then `JSON.parse(r.output)` failed on a
  display truncation warning. The corrected producer wrote a file and returned a compact receipt.
  Use structured channels/files for complete data; preserve batching and arbitrary displayed output.
- **T2:** outer JavaScript lacked a closing parenthesis; a valid `...}); text(r.output)` composition
  completed. A tiny valid example helps. No shell execution or native failure should be invented.
- **T3:** `write_stdin` reported unknown process92725 after prior observations used it. Cause remains
  unresolved; durable resumption is useful when required, but no runner blame or cancellation claim follows.

Retired Docker cleanup/root-owned files and the earlier missing native binary remain historical
facts. Current native fixtures cannot be condemned on those receipts. Test assertions, Rust type
errors, authentication refusals, 300s/900s timeouts and cancellations remain with their product
diagnosis/qualification owners. Model changes, more wrappers or thread caps are not demonstrated remedies.

Useful short examples belong in existing help/runbooks/skills at their owner, rather than another
always-loaded command handbook. The current instruction and memory routes can remain broad enough
for complex work. Context reduction alone has no demonstrated benefit, and overlapping worker
guidance should not be removed simply for duplication.

## 10. Verification and uncertainty

| Claim | Evidence and outcome |
|---|---|
| Current source defects and preservation constraints | **Implemented / inspected, 2026-10-09** using the source reads named above. No runtime reproduction; no historical causality claimed. |
| Frozen command counts and selected traces | **Measured historical observations**, window and limitations in retrospective/metrics. Extraction command passed; selected original events read by coordinator and independent reviewer. No effectiveness rate inferred. |
| Prior workspace implementation | Historical October7–8 controls and implementation-review corrections are attributed to [workspace plan §8–13](../../plans/agent-workspace-effectiveness-plan_2026-10-07.md#8-review-integration). They are not fresh assembled acceptance. |
| Proposed capabilities | **Interface-checked** routes in supporting briefs and decisive source inspection; actual integration and benefits **Proposed**. |
| Builds/tests/sync/pilots/probes/operator changes | **not_run**, per scope; no prerequisite was repaired or bypassed. Static inspection settled diagnoses. |
| Coordinator publication | **passed, 2026-10-09:** `just docs-check`, 362 canonical pages, zero link errors. Documentation acceptance only. |

Read-only inspection commands (`cat`, `rg`, `sed`, `nl`, `git status --short` and a standard-library
JSON event reader) passed for the cited files. Some guessed paths were absent and subsequent file
searches located the owners; those lookup misses establish no missing product capability. No repository
file was modified by the reviewer; only the review scratch artifact was written. No turn-end ran.

Unexamined breadth includes full product behavior, entire test-family coverage, every shared-skill
contract and live settings propagation. These are scope limits, not findings. Material uncertainty
affecting choices is narrower: live fixture MCP scope/auth/lifetime, adapter readiness/health after
loading, structured query redaction and whether repeated receipt consumers warrant a selector.
Choose those integrations from real tasks and qualify revealing cases when implementation is authorized.
The F01–F05 diagnoses do not depend on resolving these optional capability questions.

## 11. Rule impacts and dispositions

Rules are assessed after judgment, not used to narrow recommendations. None is changed by this review.

| ID | Current rule/location | Proposed change and dependent recommendation | If kept |
|---|---|---|---|
| RC01 | [Workspace plan §1](../../plans/agent-workspace-effectiveness-plan_2026-10-07.md#1-context): equal Codex/Claude weighting; repository-only proposals | Current user intent is Codex primary/limited Claude, with eligible workstation/user/shared-skill customization. Use that scope for selected additions and diagnostics. | A future repository-only implementation can take repo slices, but cannot claim to address this review's entire authorized scope. Equal-runtime work would consume effort without the current user premise. |
| RC02 | Workspace plan D1 managed readers enumerates verify/fixture consumers; readiness sampled separately in implementation | Include managed discovery and bind used readiness to owned publication intervals; recheck queued writers. F03. Preserve observation-only readiness and unmanaged escape routes. | Discovery remains outside the coordination guarantee and must be advertised as such; stale observation remains advisory and cannot justify execution readiness. |
| RC03 | Run schema1/readers/pruning treat any termination as terminal recovery eligibility (`runs.py:117,678,752`) | Represent cleanup success/failure/unknown independently; unresolved cleanup remains recoverable and protected from pruning. F01. Amend existing record contract/version as required; no new ledger. | Restrict terminal claims to process exit and explicitly retain manual cleanup/recovery obligations; the present normal cancel/prune behavior still needs correction. |
| RC04 | Current foreground/`--live` route synchronously mirrors output (`runs.py:389`, `verify.py:1168`) | Full owned logs stay authoritative; live display becomes detachable/best-effort outside supervision. F02. | Provide a separate non-streaming durable path and exclude stalled observers from the synchronous path's guarantee; that does not accept the currently supported foreground case. |

F05 refines an implementation outcome contract rather than changing the existing rule that blocked
requires evidence. F04 refines a reader algorithm. AE-24 repairs the existing read-only claim; no
permission to mutate freshness inputs is proposed. Optional native diagnostic additions need no
standing gate, catalog, resource restriction or tool mandate. Existing architectural changes follow
the repository ADR/owner route if the subsequently selected design changes a real architectural decision.

| Current obligation | Disposition owner / trigger |
|---|---|
| New F01–F05 | Transferred to [follow-up plan §8](../../plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner); original IDs and assessment retained. |
| Native/query/semantic/receipt/help opportunities | Choices, accepted rule impacts and sole disposition at follow-up plan §4/§8; no implementation closure from scheduling. |
| AE-24 freshness/documentation gap | Transferred through the existing workspace plan to follow-up plan §8; original ID retained. |
| AE-12/AE-25 test coverage | Existing testing architecture owner via workspace plan; no duplicate coverage acceptance here. |
| Product native authentication/stream/timeouts/cache equivalence | STATUS and its native-execution/persisted graph coordinators; this review supplies no closure or renewed execution authority. |

## 12. Architectural judgment and bounded decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | violated at outcome policy; otherwise satisfied, scoped | Native grammar, boundary/step owner, requirement-scoped environment, fixture identity and optional checkout keep ordinary new selections/context bindings local. F05 duplicates server-end classification across direct and family consumers; a changed outcome policy requires independent semantic edits. |
| A2 Encode domain meaning explicitly | violated | Launch failure/child exit and process terminality/cleanup certainty are not adequately represented/used (F01/F05); readiness is consumed beyond its protected premise (F03). |
| A3 Extend through composition | violated | The normal composition is useful, but observing output and publishing progress remain entangled with supervision, and managed discovery bypasses the environment operation contract (F01/F02/F03). |
| A4 Fit execution to supported workload | violated | Non-consuming observers can block lifecycle work; repeated small log observations scan/allocate complete logs (F02/F04). Normal native/direct routes otherwise have credible qualitative fit. No speed claim is made. |

Applicable foundations FP-01/02/04/05/06 are **violated at the named lifecycle/readiness/outcome
boundaries**, with the native selection/fixture/checkout separations **satisfied** in their stated
scenarios. FP-03 composition is **violated** at observer and managed-discovery integration.
FP-07 is **violated** by the demonstrated observation amplification. Supporting DP-02/03/09/18/19/21
and DP-10/16 have the scoped findings above; DP-01 is violated at F05's duplicate outcome policy;
DP-13/14/15/24 have inspected scoped owner/native
mechanisms, subject to the explicitly unqualified live additions. Applicable CI cautions are satisfied
by attribution/unknown/evidence handling for the examined observations, without certifying product gates.

**Bounded decision: Revise.** Correct the existing lifecycle, environment and outcome boundaries,
and carry forward AE-24. The enclosing agent workspace needs revision for the examined supported
failure/growth scenarios. Product architecture and release qualification are not assessed.
No positive local capability assessment offsets the failed gates or A2 gap.

Consequence-based priority starts with run ownership/cleanup and non-blocking supervision, then
truthful outcome/readiness contracts. Selective log access is independently worthwhile. A reliable
lifecycle and identified fixture content are prerequisites for trustworthy cross-session diagnostic
composition; they do not postpone cheap native syntax checks or short help examples. Decide optional
MCP/adapter/receipt additions on actual consumers, not on library availability alone.

The next architectural decision belongs to the coordinator/operator: select the existing-owner
corrections and useful diagnostic capabilities for subsequent planning, preserving native/override
freedom and the current held product/operator boundaries. This document is the independent principal
review; supporting evidence is not a second verdict or disposition register.
