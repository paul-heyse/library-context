# Agent test evidence and causal analysis

**Design/target review · 2026-10-11**

## 1. Scope and decision

**Decision: Revise the current architecture for the requested evidence workflow.** Preserve the
execution, fixture and storage owners. Extend verification with native test artifacts, explicit
capture coverage, selective evidence access and valid rerun planning.

The recommended architecture is **Proposed**: native records remain authoritative for what their
producers captured; a thin, rebuildable index supports selective inspection. JUnit is a useful
completed-result projection, but cannot independently satisfy live inspection, interruption
recovery, phase fidelity or exact raw-output requirements. A replacement execution harness or
telemetry database adds machinery without resolving these missing contracts more directly.

| Field | Assessed scope |
|---|---|
| Functional target | Codex can recover what executed, failed and remains unknown; cite original evidence; distinguish evidenced common conditions from causal hypotheses and unrelated failures; inspect live/interrupted runs; construct minimal valid reruns; compare source/configuration contexts without repeating the full suite or loading full logs |
| Baseline | HEAD `48088a95ebd7e99b5ebc857792748766a918abed`; dirty `.config/nextest.toml` and untracked retained workspace-failure evidence preserved |
| Standard | Core/template3.3, heuristics1.0, code-intelligence profile/review additions1.5 and library-context binding |
| Reviewers | Independent `design-reviewer` assessment; coordinator inspected decisive source, reconciled library research and performed the reporting probe |
| Mechanisms | `just test-all`, `just verify`, `just runs`, Nextest, pytest, stable SurrealDB fixture attachments and managed storage |
| Workload | Thousands of Rust cases, Python phases, repeated failures, large outputs, interrupted execution and concurrent checkouts; selective follow-up across Codex sessions on one workstation |
| Evidence strength | Repository source **Implemented**, inspected2026-10-11; Nextest0.9.146 pinned interfaces/source **Interface-checked**; named synthetic pytest composition **Tested**,2026-10-11 |
| Exclusions | Product correctness, complete diagnosis of the supplied failure set, full-suite/release acceptance, service operations, CPU attribution and measured agent productivity |

The review judges the functional target against the [design principles](../design_principles/core/design-principles.md)
and [efficient-architecture heuristics](../design_principles/core/efficient-architecture-heuristics.md).
Current policies describe the subject; §11 addresses their implications separately.

## 2. Responsibilities, meaning and authority

| Owner | Current responsibility |
|---|---|
| `scripts/verify.py` | Boundary catalog, resolved command plan, requirements, fixture composition, step outcomes and verification summary |
| `scripts/runs.py`, `scripts/harness.py` | Durable process identity, launch/supervisor failures, actual child exit, interruption, cleanup and file capture |
| Nextest / pytest | Native discovery, execution and test-result semantics |
| Fixture/service owners | Installed service identity, logical attachment, borrower ownership and evidenced substrate outcomes |
| Storage owner | Availability obligations, release, reader admission and retirement of independently owned components |
| Codex / human reviewer | Evidence interpretation, causal hypotheses and further investigation |

This separation is valuable. `verify.execute` continues independent boundaries, `_run_steps`
preserves dependencies inside a boundary, and `LaunchResult` distinguishes launch failure from
an executable returning127. `runs.view` combines recorded state with owner liveness. Preserve
these mechanisms (`scripts/verify.py:1010–1299`; `scripts/runs.py:125–225`).

The expanded workflow needs deliberate distinctions:

- Repository run, boundary, command invocation, native runner UUID, case, retry and phase are different identities.
- Command exit, case result, run termination, cleanup, fixture state and evidence completeness are independent observations.
- Planned targets/files, collected cases, selected cases and observed execution are different sets.
- An exception chain, repeated message, shared prerequisite and causal conclusion are different evidence kinds.
- Recorded-output replay and current-code execution have different effects and reproducibility requirements.
- No failures differs from no readable results, not executed, not collected and incomplete capture.

These meanings should govern verification-owned operations and queries. Ordinary records and
functions suffice; no universal ontology or new product-model subsystem is required.

| Evidence family | Authority and fidelity | Identity/coverage | Consumer |
|---|---|---|---|
| Run lifecycle | Run owner; observed | Run/process identity; cleanup can remain unknown | Status/recovery |
| Boundary/step result | Verification owner; observed command result | Boundary/invocation; currently no case ingestion | Summary/boundary rerun |
| Nextest result | Native producer; observed | UUID, binary, test, attempt; artifact-dependent completeness | Case inspection/replay |
| Pytest report | Native/reportlog phase observation | Invocation, node ID, phase; not a test-start record | Phase inspection/accounting |
| Source/configuration context | Captured input observations | Recorded bytes/identities or explicit missing coverage | Comparison/attribution |
| Failure cluster | Derived grouping | Declared grouping rule and full membership | Navigation |
| Causal explanation | Investigator interpretation | Supporting/conflicting references, assumptions and unknowns | Diagnosis/correction |

## 3. Current outputs and concrete gaps

**Implemented, inspected2026-10-11:** runs retain `record.json`, `output.log` and progress;
verification retains its plan, per-boundary logs, profile/command information, step exits and
`summary.json`. Summaries update after each boundary returns. A long-running or disappeared
boundary may have useful log/progress evidence without a completed result. Missing results
cannot establish that its tests never started (`scripts/runs.py:10–27,331–350`;
`scripts/verify.py:637–648,1243–1250,1730–1774`).

All steps in one boundary append to one log, marked with their commands. The receipt does not
provide step offsets or case-level evidence references. Raw logs can still contain the exact
panic, failing case or timeout; the absence is selective structured access, not all diagnosis
information (`scripts/verify.py:1154–1203,1275–1299`).

The dirty Nextest configuration provides `target/nextest/default/junit.xml`. Its fixed path is
overwritten per invocation and verification does not bind its UUID/path/digest to a step. It
cannot safely identify evidence across multiple boundaries or concurrent runs. Recording is
enabled in this workstation's user configuration, not established by the managed runner.

**Implemented:** `test_scope_coverage.py:79–85,186–198` honestly accounts for targets/files, not
executed cases. Preserve that limit. `verify.py:1685–1727` repeats failed, blocked and unreached
boundaries; it is not a minimal case rerun.

Fixture `source_inputs` captures HEAD, tracked-diff digest, untracked names and server identity
for a narrower retained-content purpose (`surrealdb_fixture.py:157–168,599,630`). It does not
capture untracked bytes or establish general run provenance. In contrast, opt-in compilation
profiling already captures source hashes/patch, tool versions and selected environment fields
(`compile_profile.py:201–275`); reuse suitable mechanics rather than copying its entire profiling
lifecycle into every ordinary test.

The coordinator independently matched the supplied local XML and portable recording UUID
`06aaf844-e8f2-4df0-a60b-9b88aaebae6a`:2041cases,684failures; XML SHA-256
`9cab4f4bfff5d2d46f01c8cc282da226f371e1a93b8a6dad5fe09ac0ec0c8ad2`.
That recording retains command/build scope but not exact Git source provenance. These are dated
observations of the preserved untracked `2026-10-11_workspace-failure-causes/raw/` inputs, not
a claim that their source matches current HEAD or that their classifications prove causes.

## 4. Library fit and capture configuration

### Nextest0.9.146

**Interface-checked:** recording/export/replay preserve richer native lifecycle information than
JUnit or libtest JSON. Recording finalization can warn independently of test exit; an exit-zero
test command does not attest readable evidence. Recording is experimental and should be
version-qualified. [Native overview](https://nexte.st/docs/features/record-replay-rerun/),
[pinned finalization](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/record/session.rs#L141).

Native output recording retains head/tail and marks removed bytes above its configured limit.
The pinned configuration clamps `max-output-size` to1000bytes–256MiB: zero is not unlimited.
This is a producer limitation to expose, not a repository test-memory restriction to introduce.
JUnit receives full live captured text independently of that cap, but applies UTF-8 replacement
and XML sanitation and is written only at `RunFinished`.
[Recording policy](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/user_config/elements/record.rs#L253),
[truncation](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/record/recorder.rs#L672),
[JUnit live-output assignment](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/reporter/aggregator/junit.rs#L510).

**Proposed default composition:** explicit managed recording configuration, uniquely located
per-step JUnit with successful/failed text, native recording/export for richer replay, and a
compact completed-case index over the qualified JUnit projection. This deliberately duplicates
some captured text to obtain selective access without maintaining a private ZIP decoder.
Compare that cost against a pinned native-reader integration if output duplication becomes
material; do not assume such a reader is already supplied by the CLI.

Recommended recording configuration, **Proposed**, not installed:

```toml
[experimental]
record = true

[record]
enabled = true
max-output-size = "256MiB"
```

This uses the producer's largest supported recording value to reduce avoidable loss. It is
not a completeness guarantee or a memory reservation. Existing owner lifecycle must retain the
required export before native cache pruning can remove it; age/count/size pruning alone cannot
govern cited evidence. Record the effective policy and actual truncation/finalization status.

Recommended per-invocation report settings, **Proposed**:

```toml
[store]
dir = "/absolute/owned/RUN/STEP/nextest"

[profile.default.junit]
path = "junit.xml"
store-success-output = true
store-failure-output = true
report-skipped = "all"
flaky-fail-status = "failure"
```

Apply to the selected Nextest profile, which is distinct from Cargo's compilation profile.
Skipped cases retain their reasons and must not become expected-execution obligations merely
because they appear in XML. Record effective per-test overrides and partition semantics.

`[store].dir` isolates profile/JUnit outputs without changing Cargo's target/build directory.
`--config-file` replaces repository discovery, so any generated effective file must preserve
the existing execution policies. Tool configuration ranks lower than repository configuration
and cannot silently guarantee an override. `--user-config-file` separately replaces user
discovery; preserve deliberate operator preferences and report the selected effective settings.
Do not silently introduce a timeout, retry, scheduling or threading policy through evidence
configuration. [Pinned config layering](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/config/core/sources.rs#L22),
[profile output location](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/config/core/imp.rs#L823).

Use explicit native UUIDs. At this revision `latest` selects the greatest start time across
indexed runs, including incomplete/running ones, despite help describing a completed run.
Capture the native UUID and validate artifact identity; do not correlate by wall-clock
proximity or `latest`. [Pinned selector](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/record/store.rs#L962).

Libtest JSON-plus can supply an incremental partial view, but omits setup/slow events,
intermediate retry details and native run metadata. Native recording is asynchronous and
requires finalization; neither is a guaranteed complete live journal after abrupt death.
Declare coverage and readable prefixes rather than interpreting missing events as absence.
[Pinned structured reporter](https://github.com/nextest-rs/nextest/blob/8af696ddcce8fff2962d6a5168b6d138b8616a35/nextest-runner/src/reporter/structured/libtest.rs#L244).

### Pytest9.1.1 / Python3.14.7

**Tested,2026-10-11:** the [synthetic capability probe](../evidence/2026-10-11_agent-test-evidence/README.md)
loaded the hash-checked `pytest-reportlog==1.0.0` wheel from a temporary directory without
installing it or running project tests. It preserved completed setup/call/teardown reports,
collection errors, exception-chain text, warnings and passing stdout/fd stderr/log sections.
Seven items produced eight JUnit rows because call plus teardown failure creates two rows for
one item. A terminated child retained the completed JSONL prefix, but no session finish or
JUnit; its unfinished call output was absent. Reportlog emitted no test-start event.

**Proposed:** pin reportlog1.0.0 as a development dependency and explicitly request one uncompressed
`--report-log` JSONL per step. Preserve default fd capture, full native traceback evidence and
warnings in reports; retain successful sections for comparisons. JUnit is optional interchange,
with `junit_logging=all` if requested. Minimal lifecycle hooks for exact starts, deselection and
collection membership may supplement reportlog; reuse pytest's native serializer rather than
write another general reporting implementation. [Reportlog upstream](https://github.com/pytest-dev/pytest-reportlog),
[exact release](https://pypi.org/project/pytest-reportlog/1.0.0/),
[pytest reporting configuration](https://docs.pytest.org/en/stable/reference/reference.html#junit-logging).

The maintained line-oriented producer is a better fit for completed-prefix recovery than a
final-session JSON report. `pytest-json-report==1.5.0` is an available alternative, but adds
pytest-metadata and does not resolve missing live-start/capture guarantees by being JSON.
It was source/documentation-inspected, not composition-tested here. A bespoke general hook
serializer duplicates pytest/plugin responsibilities. Native JUnit alone loses phase-level
accounting and abrupt-termination availability demonstrated by the probe.

### Build, documentation and native diagnostic channels

Compilation, discovery, doctest, compile-fail, lint and other command failures remain first-class
step outcomes even if no test report exists. For commands with supported machine diagnostics,
retain them separately from human stderr and preserve compiler source locations/codes and
artifact identities. Introduce no generic parser that fabricates cases from every console line.
The existing compile-profile owner remains the route for actual compiler CPU/query analysis.
Cargo's `compiler-message`, `compiler-artifact` and `build-finished` JSON messages provide
native build evidence; `--message-format=json` does not convert arbitrary test/tool output to
JSON. [Cargo external-tool contracts](https://doc.rust-lang.org/cargo/reference/external-tools.html).

**Proposed diagnostics defaults:** Rust backtraces when not explicitly configured otherwise;
pytest captured output, warnings and long tracebacks; retained structured error meaning and
coarse native phase observations where available. Record diagnostic settings. Deep tracing,
verbose row/SQL logging, resource sampling and CPU profiling are selectable investigation
capabilities, not routine prerequisites. Runner timing is not CPU attribution.

Native phase IDs in `crates/lctx-surrealdb/src/phase.rs` are process-local. Correlate them with
run/invocation/process identity, existing service generation and attachment/operation identity;
do not reinterpret equal phase IDs across processes as one operation. Preserve primary/secondary
errors, terminality and unknown remote effects already represented by the domain owners.

## 5. Target outputs and agent access

All changes in this section are **Proposed**.

| Product | Minimum useful meaning |
|---|---|
| Invocation manifest | Run/boundary/step and native UUID, tool/version, argv array, cwd, profiles/selection, relevant source/configuration context, fixture identities and artifact locators |
| Capture status | Requested/effective channels, available/missing/partial/truncated/unsupported evidence, producer/finalization errors and readable watermark |
| Case/phase projection | Native case identity, invocation, attempt/phase, observed result, supplied timing and original evidence references |
| Evidence response | Bounded excerpt, exact artifact identity/locator, transformation/loss, coverage and continuation |
| Timing/events view | Observed build/discovery/setup/execution/cleanup intervals, current known work and qualified native timelines |
| Rerun plan | Selected obligations, actual prerequisites/dependencies, argv arrays, expected scope, context changes and blocked/unsupported reasons |
| Comparison | Recorded outcome/context differences and unavailable fields; no automatic claim that differences caused a failure |

Keep native outcome enums and raw exit codes alongside repository outcomes. Evidence-health
failure must not rewrite a passing test into a test failure, nor hide an incomplete evidence
request behind test success. Preserve skips, xfail/xpass, retries, timeout policy, cancellation
and cleanup as distinct observations.

Capture source/configuration context once per appropriate lifetime. Record relevant tracked and
untracked content identity, lock/toolchain identities, effective runner policies and non-secret
configuration. Reuse service/environment owners. A launch HEAD or start/end source observation
does not prove which bytes an already-built binary contains or that no intermediate edit
occurred. Distinguish observed checkout from compiled-artifact provenance, and flag missing
build identity or observed changes. Avoid a global editing lock or full workspace copy per case.

The index is an access path over originals. Prepare it once per artifact version in a streaming
pass; retain compact identities and locators and hydrate selected evidence. Do not repeatedly
decode a full archive/XML for each requested case. A compact run-local materialization can
suffice; an additional database is not a requirement. Reader construction must settle concrete
locators/access before claiming selective access; native ZIP internals are not an assumed API.

For live reads, return an explicit prefix/watermark. An earlier citation must keep referring to
the same observed prefix after append. Protect readers and cited originals through existing
storage admission/consumer obligations; receipt survival does not establish log availability.
Expose original removal/change explicitly. Keep reports/indexes independently rebuildable and
avoid permanent copies of every rendered excerpt.

### Proposed command surface

Keep test-domain inspection under verification and generic process lifecycle under `runs`.
The following are target interfaces, **not available commands**:

```text
just verify report RUN --json
just verify cases RUN --outcome failed --limit 50 --json
just verify evidence RUN --case CASE --phase call --limit-bytes N --json
just verify events RUN --step STEP --json
just verify compare RUN_A RUN_B --json
just verify rerun-plan RUN --case CASE --print --json
just verify export RUN --output PATH
```

Responses should carry stable IDs, schema/version, filters, coverage, omitted counts and
continuations. Run overview is small; failure membership and full evidence remain accessible.
Allow grouping by declared signatures or evidenced prerequisite identity while retaining every
member and exceptional case. Grouping organizes investigation; it never establishes causation.

`rerun-plan` prints a plan by default. Preserve the existing boundary meaning of
`just verify --rerun RUN`. Exact case execution must retain profile, ignored policy, fixtures
and actual producer prerequisites; a serving consumer cannot safely omit its required producer.
Current-code recomputation is distinct from recorded-output replay. Source/config changes and
outstanding obligations remain visible; a focused pass cannot certify the original full scope.

Recommended Codex journey: inspect lifecycle/evidence health; list distinct failures; retrieve
representative and exceptional cases plus passing comparisons; inspect relevant source/context;
state observations, hypotheses and conflicting/unknown evidence; print the smallest valid next
experiment; run only the chosen scope under its owner. No mandatory causal classifier,
annotations database, model-generated summary or semantic search is needed.

### Existing native commands

These installed Nextest0.9.146 interfaces were inspected; recording/export/trace commands below
are useful routes, not operations performed during this review. Use an explicit captured UUID
or recording path. Export paths require the existing output owner/admission.

```sh
NEXTEST_EXPERIMENTAL_RECORD=1 cargo nextest --no-pager store info UUID
NEXTEST_EXPERIMENTAL_RECORD=1 cargo nextest --no-pager replay -R UUID \
  --success-output immediate --failure-output immediate
NEXTEST_EXPERIMENTAL_RECORD=1 cargo nextest --no-pager replay -R UUID --exit-code
NEXTEST_EXPERIMENTAL_RECORD=1 cargo nextest store export UUID --archive-file OWNED_PATH.zip
NEXTEST_EXPERIMENTAL_RECORD=1 cargo nextest store export-chrome-trace UUID \
  --group-by slot -o OWNED_TRACE.json
```

Replay normally exits0 when inspection succeeds, even for a failed recorded run; `--exit-code`
requests original status and can return106 for incomplete/unknown execution. Native rerun can
return5 when selected work passed but outstanding obligations remain; it does not track source
changes or revalidate previous passes. Retain parent UUID and original obligations.
[Replay](https://nexte.st/docs/features/record-replay-rerun/replay/),
[rerun](https://nexte.st/docs/features/record-replay-rerun/rerun/).

Native Chrome/Perfetto export already supplies runner timelines, slots, attempts and concurrency
observations. Use its exporter rather than write a converter. A human viewer is optional;
structured trace inspection or Perfetto SQL can support targeted agent queries when needed.
This is not a CPU stack/resource profiler. [Native traces](https://nexte.st/docs/features/record-replay-rerun/perfetto-chrome-traces/).

Current repository routes remain `just runs status RUN --json`, `just runs logs RUN`,
`just verify --rerun RUN` and `just compile-profile status/report` for their existing scopes.
Their availability does not imply the proposed case/evidence commands are implemented.

## 6. Change, failure and growth scenarios

| Scenario | Required owner behavior / settling evidence |
|---|---|
| New verification boundary | Existing catalog supplies selection/prerequisites; adapter supplies supported evidence. No second test catalog |
| Call and teardown both fail | Preserve independent phase reports under one item; probe demonstrates why row-counting/dedup by test name is wrong |
| Runner killed mid-case | Preserve valid prefix; terminal/capture gaps stay unknown. Never convert missing completion into success or definite nonexecution |
| Recording fails while tests pass | Capture health remains separate; qualified failed-artifact control |
| Concurrent runs/checkouts share build/profile | Invocation-owned reports and explicit UUID; no fixed-path/latest join or Cargo target isolation merely for reporting |
| Dirty/untracked bytes change | Record changed identity or uncertainty; distinguish checkout observations from actual executable identity |
| Stable service restarts/another borrower fails | Existing installation/generation/attachment and observation intervals establish context; proximity alone does not establish shared cause |
| One failure needs prior native production | Rerun resolves existing real dependencies; minimal means semantically sufficient |
| More unrelated cases/log bytes | Index once; hydrate selected evidence rather than repeated full replay/parsing |
| One enormous output | Distinguish complete text, native truncation, sanitation and missing unfinished output; bounded responses retain continuations |
| Producer/artifact format changes | Qualified adapter refuses unsupported interpretation rather than guessing current semantics |
| Citation outlives ordinary log retention | Existing storage hold covers the original needed for that citation; missing evidence is explicit |

A domain extension such as subtest/multi-phase results belongs in the result adapter and typed
projection. Replacing the reporter belongs at the producer adapter and its conformance controls;
it must not propagate private format parsing into every agent command. Full enumeration examines
a run once; that does not justify examining all its output for each subsequent question.

## 7. Findings and closure

These are gaps against the requested target, not claims that existing commands advertise all
of it or that an existing classifier has issued false causal conclusions.

### <a id="F01"></a>F01 — Case/phase execution and capture coverage lack a governing read contract

**Principles:** FP-04/05, DP-02/08/21, CI-01/04; A2, G2/G7.
**Evidence:** boundary/step results and post-boundary summary updates
(`verify.py:1046–1065,1243–1250,1730–1774`); no case ingestion in the inspected runner;
producer coverage and interruption differences in §4.

**Consequence:** consumers reconstruct case/phase/execution completeness from prose or separate
artifacts. A stopped boundary cannot be faithfully described from its summary alone. Fixed
JUnit paths and unverified user-only recording cannot establish invocation identity.

**Correction:** verification owns small case/phase and capture-status contracts, realized by
native adapters and invocation-owned artifact paths. Reuse qualified native formats/reportlog.
**Closure:** independent setup/call/teardown, same names across binaries, retry/flaky/skip,
collection failure, interruption, malformed/truncated input and absent-artifact controls;
distinct test and capture results and concurrent invocation identity.

### <a id="F02"></a>F02 — General run context cannot support exact source/configuration comparison

**Principles:** FP-04/06, DP-04/09/21, CI-01/10; A2, G7.
**Evidence:** cwd/argv/profile receipts (`runs.py:331–350`; `verify.py:637–648`), narrower
fixture fingerprint (`surrealdb_fixture.py:157–168`), opt-in profiling counterexample (§3).

**Consequence:** nominally identical runs can differ through dirty/untracked sources, user
configuration and environment. Later inspection cannot recover those differences reliably.

**Correction:** capture relevant context once per actual lifetime, with explicit coverage;
reuse source/environment/service owners and record native identities without credentials.
Distinguish source observations, actual binary provenance and unresolved concurrent drift.
**Closure:** same HEAD/different tracked bytes, different untracked bytes, changed effective
config and concurrent edit are distinguishable or explicitly uncertain. Historical evidence
stays attributable without falsely promising exact recomputation.

### <a id="F03"></a>F03 — Boundary reruns are too coarse for the requested investigation workflow

**Principles:** FP-02/03/07, DP-08/16/21; A3/A4.
**Evidence:** `rerun_selection` reconstructs nonpassed boundary selections against the current
catalog (`verify.py:1685–1727,1815–1824`).

**Consequence:** one failed case can repeat its entire boundary; an ad-hoc smaller command can
omit prerequisite production, profile, fixture or ignored-test policy.

**Correction:** a separate verification-owned rerun-plan operation resolves exact case identity
and existing dependency owners. Preserve boundary-rerun semantics and original outcomes.
**Closure:** printed argv selects exact Rust package/binary/test or pytest node ID, quotes
parameter IDs, retains prerequisites/profile, refuses unsupported identities, exposes changed
context and remaining obligations, and launches nothing while planning.

### <a id="F04"></a>F04 — Evidence retrieval lacks a selective, pinned lifecycle

**Principles:** FP-02/06/07, DP-01/08/19/21, CI-08/11/13; A1/A4, G5/G6.
**Evidence:** generic log tail/stream and append-only boundary logs without case locators
(`runs.py:776–800`; `verify.py:1275–1299`); existing distinct receipt/log lifetimes in
[workspace storage](../../workspace-storage.md).

**Consequence:** questions repeat log search/full replay. A mutable report path is not a stable
citation, and receipt survival does not ensure evidence availability.

**Correction:** qualified artifact identities/locators, live-prefix watermarks, bounded reads,
explicit missing/truncated states and existing consumer/reader protection. Inspection/index
rebuild must not rewrite originals. **Closure:** index deletion/rebuild preserves answers;
append does not reinterpret prior references; concurrent runs stay separate; cited readers
prevent retirement; changed/missing originals are explicit; selective requests do not repeat
unrelated full-input processing.

**Disposition transferred, 2026-10-11:** the standalone
[implementation plan §8](../../plans/agent-test-evidence-and-causal-analysis-plan_2026-10-11.md#8-sole-current-finding-disposition-and-checkpoint)
owns current scheduled status for F01–F04 using these source IDs. This review retains its dated
assessment and evidence; planning is not implementation or closure. F01/F02 establish meaning/attribution;
F03 removes unnecessary execution and F04 unnecessary inspection. Their closure obligations
remain distinct.

## 8. Alternatives and total machinery

| Alternative | Judgment and tradeoff |
|---|---|
| Current logs/boundary receipts | Preserve execution foundation; insufficient for complete case/context access |
| JUnit alone | Useful completed interchange; insufficient for interrupted execution, phase accounting and raw-byte fidelity |
| Native artifacts plus thin rebuildable index | **Recommended, Proposed.** Reuses native execution semantics while adding cross-tool access/context contracts. Qualify concrete locators, normalization and capture completeness |
| Native commands without index | Useful whole-run replay; insufficient selective cross-tool/cross-session access. Use them to reduce index responsibilities |
| Custom rich harness/telemetry database | Disproportionate now: duplicates phases/retries/capture and adds schema, coordination and lifecycle obligations. Reconsider for a demonstrated unserved consumer |

No new persistent database or graph is needed for this capability. Actual runner dependencies
and existing state identities can supply factual relations; a typed edge does not prove causation
or authorize execution. A separate native reader may later replace duplicated JUnit output if
its qualified access and total lifecycle are simpler. Additional unfinished-output capture is
a separate selectable capability where investigation needs it, not a claimed native guarantee.

## 9. Causal analysis and assurance workflow

The supplied failure inventory is useful navigation, not proof one prerequisite repair closes
684failures. Distinguish direct observed configuration failure; a shared evidenced prerequisite;
the hypothesis that repair will expose or resolve subsequent behavior; independently observed
assertions; and unknown behavior after setup. Keep every membership and original result.
Similar text does not prove one cause, different text does not prove independence, and a later
pass does not change the earlier observation.

Recommended implementation order is contractual rather than a new execution plan: invocation
identity/capture and context first; native capture/adapters and artifact lifecycle next; selective
inspection and rerun planning over those contracts. Each meaningful boundary needs revealing
independent controls. Test malformed records, output loss, multi-phase failures, unrelated cases
and dependency-sensitive reruns rather than merely round-tripping production serializers.

Further investigations: concrete locator/index strategy; unsupported producer versions;
source-to-binary provenance coverage; minimal starts/collection hooks; qualifying output capture
if unfinished live bytes become required; actual trace-query benefit before adding another tool.
None requires another full product test campaign to settle basic reporting architecture.

## 10. Foundations, gates and evidence limits

| Architectural judgment | Current architecture against the full requested target |
|---|---|
| A1 Localize change | **Unresolved:** coherent owners, but consumers reconstruct case/evidence interpretation |
| A2 Encode domain meaning | **Violated:** required case/phase, capture completeness and context distinctions do not govern the full workflow |
| A3 Extend through composition | **Unresolved:** useful primitives; minimal case rerun and evidence operations not implemented |
| A4 Fit execution | **Violated:** boundary-wide reruns and repeated log recovery amplify selective investigation |

FP-01/03 are supported by current ownership. FP-02/04/05/06 need evidence contracts;
FP-07 needs selective access and sufficient rerun units. Relevant DP-01/02/04/08/09/19/21/24
and CI-01/02/04/08/11/13 are addressed by F01–F04 and target invariants; this does not certify
unexamined product behavior.

| Gate | Verdict and scope |
|---|---|
| G1 Authority | **Pass**, inspected owner separation; index must remain derived |
| G2 Semantic fidelity | **Unresolved**, new cross-tool phase/retry/unknown contracts need F01 |
| G3 Validity | **Unresolved**, proposed evidence admission/rejection not implemented |
| G4 Hidden behavior | **Pass**, inspected observation/planning separation; keep new inspection free of builds/sync/service effects |
| G5 Consistency/recovery | **Unresolved**, target citations/live coverage need F04; current cleanup distinctions are strengths |
| G6 Transformation/reuse | **Unresolved**, index rebuilding and normalization/locator qualification remain |
| G7 Truthful claims | **Pass** for inspected bounded current claims; new capability remains Proposed |
| G8 Library leverage | **Pass** for recommended direction at Interface-checked strength; reuse native reporting and reportlog |
| CI-G1 Fidelity | **Unresolved** for target incomplete execution and causal interpretation |
| CI-G2 Evidence closure | **Unresolved** for selective cross-session references |
| CI-G3 Evaluation integrity | **Not applicable** to product gold/heldout evaluation; adapter controls still need independent meanings |

**passed**,2026-10-11: read-only source/configuration/artifact inspection and installed Nextest
version/help qualification; the principal reviewer inspected decisive pinned sources and the
coordinator's probe. **passed**: four synthetic pytest-format scenarios, run
`20261011T052916.677Z-80b055`, command in the [evidence README](../evidence/2026-10-11_agent-test-evidence/README.md).
These deliberate fixture failures are probe inputs, not repository test failures.
**not_run:** new native recording/export/interruption integration, full repository tests,
assembled qualification and quantitative effectiveness/performance measurements.
**passed**,2026-10-11: `just docs-check` (publication/resolution, zero link errors) and
`git diff --check`; these establish documentation validity, not product acceptance.

The probe establishes completed-phase behavior at its named composition. Source inspection
establishes interface behavior, not a complete crash-durability guarantee. Completed capture
can still be normalized/truncated. Killed execution can lose unfinished output; disclose it.

## 11. Current-rule implications

These are proposed extensions/clarifications, not applied authority changes. Implementation
planning should determine whether an actual architectural decision change requires an ADR;
this review creates no permission gate for ordinary tool additions.

| ID | Current rule | Recommended impact | If unchanged |
|---|---|---|---|
| <a id="RC01"></a>RC01 | ADR-0142 / DESIGN §1.2 file-log authority | Extend documentation to qualified native artifacts and derived projections; preserve log authority for its channel | New reports remain limited supplements, not complete evidence claims |
| <a id="RC02"></a>RC02 | AGENTS/README boundary-rerun guidance | Add separate minimal valid case planning while retaining `--rerun` semantics | Manual smaller selections remain; F03 open |
| <a id="RC03"></a>RC03 | Dirty Nextest comment assumes user recording/fixed JUnit destination | Managed capture selects explicit effective settings and invocation-owned destinations, preserving operator execution policy | Recording remains environment-dependent and fixed-path correlation unsafe |
| <a id="RC04"></a>RC04 | Separate receipt/log storage categories | Reuse owner admission and consumer obligations for new native artifacts/indexes/citations; add categories only if lifetime differs | New artifact availability stays unsupported/protected until an owner is qualified |

No uploader, scheduler, semantic search service, mandatory graph database, resource cap or new
broad product gate follows from these recommendations. Tooling evidence cannot be relabelled
whole-product qualification.

## 12. Final judgment

**Revise** the evidence subsystem for the full requested workflow. The existing implementation
is a sound execution-observation foundation. **Native artifacts plus a thin, rebuildable
verification read model is the recommended Proposed direction.** Capture once, prepare selective
access, hydrate requested evidence and repeat only necessary execution.

Settle the invocation/capture contract and concrete case-access mechanism, then carry F01–F04
into one implementation-plan disposition owner. Preserve independent outcomes, original
captures and explicit unknowns. Product architecture, release qualification, full-suite
correctness and measured agent effectiveness remain outside this judgment.
