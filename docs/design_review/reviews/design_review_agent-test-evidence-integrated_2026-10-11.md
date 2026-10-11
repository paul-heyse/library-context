# Integrated native test evidence and causal investigation review

**Interface-checked · 2026-10-11 · design/target · Accept scoped**

The assembled TE0–TE6 tooling has a sound architecture for selective investigation of native
test evidence. Native producers retain result meaning; verification adds invocation observations,
rebuildable access and read-only experiment planning. Corrections identified below fit those
owners and have been inspected in the resulting source. This judgment accepts the bounded
tooling architecture. Functional acceptance remains the coordinator's named focused controls;
it establishes neither product qualification nor measured agent effectiveness.

## 1. Scope and evidence boundary

The subject is the [standalone plan](../../plans/agent-test-evidence-and-causal-analysis-plan_2026-10-11.md),
[DESIGN §1.2](../../design/DESIGN.md#12-increments), ADR-0147, and the concrete
`scripts/verify_evidence/` implementation with its `verify.py` and `runs.py` consumers.
The inspected baseline is HEAD `01858e4e104e19e0ba27bd3db1017e9e3092e990` plus the concurrent
uncommitted TE0–TE6 changes on 2026-10-11. The unrelated dirty `.config/nextest.toml` and
workspace-failure evidence were preserved and are not corrections or acceptance subjects.

The standard is core/template 3.3, heuristics 1.0, code-intelligence 1.5 and the repository
binding declared in [standard.toml](../design_principles/standard.toml). The independent reviewer
read the owner code, adjacent lifecycle/environment contracts and decisive control sources.
No product build, database activation, cleanup, dependency synchronization or test execution was
performed by this reviewer. Changing source during review was followed through the affected
contracts; unrelated product architecture was not reassessed.

The workload premise is thousands of case/phase observations, large captured outputs, repeated
small evidence requests, interrupted producers and concurrent mutable checkouts on one host.
Power-loss durability, xdist/retry-plugin support, exact executable source closure, automatic
common-cause diagnosis and quantitative productivity claims are outside the supported scope.

## 2. Responsibilities, domain meaning and contracts

| Owner | Responsibility and consumers | Expected reason for change |
|---|---|---|
| `verify.py` | Existing boundary catalog, selected commands, prerequisite declarations and repository outcomes | Add a boundary or change its real production/consumption obligations |
| `runs.py`, `harness.py`, fixture/environment owners | Process identity, cancellation, cleanup, readiness, environment and substrate attachment | Change supervision or substrate lifecycle |
| `capture.py`, `context.py` | Invocation allocation, frozen launch observations, capture preparation and durable command/post-step distinctions | Add an observation or capture binding |
| Nextest/pytest/Cargo adapters | Interpret qualified native formats, retaining producer-specific losses and identities | Upgrade a producer or add a genuinely new native phase/result kind |
| `contracts.py`, `projection.py` | Checked evidence identity, committed prefixes, rebuildable metadata/payload access | Change reference semantics or the physical access mechanism |
| `cli.py` | Reporting, selection, continuation, comparison, export and named citation holds | Add a presentation or evidence journey |
| `rerun.py` | Exact selection over current verification prerequisites, with owned environment binding obligations | Qualify another exact selector or prerequisite operation |
| Existing storage owner | Admission, independent component lifetimes, holds, confirmed release and retirement | Change a real consumer or retention obligation |

The model represents consequential distinctions through ordinary operations and checked
references rather than requiring another model service. `LaunchRequest` separates launch effects
from capture preparation. `EvidenceReference` binds invocation, native generation/range,
case/attempt/phase and interpretation; `PrefixWatermark` separates native and derived committed
ends. Command exit, capture availability, postprocessing, boundary outcome and cleanup remain
separate observations. Freezing the invocation's fixture dictionary prevents a later restart from
rewriting the command's launch attachment.

| Fact family | Authority and fidelity | Identity and limits |
|---|---|---|
| Nextest results | Nextest 0.9.146 JUnit/native recording; extracted observations | Native UUID and exact completed binary/test identity; normalized text and truncation limits remain explicit |
| Nextest live JSON | Qualified libtest-json-plus prefix | Invocation/range and synthetic case identity; omitted exact binary identity, intermediate retries and unfinished output are not invented |
| Python results | Pytest 9.1.1 / reportlog 1.0.0 native reports | Invocation/node/phase/attempt/range; completed reports are distinct from supplemental membership/start observations |
| Build diagnostics | Supported Cargo JSON records | Invocation/native byte range; no completed-case inference from a build result |
| Source/configuration context | Observed content and effective configuration identities | Dirty/untracked content and external selected configuration remain distinguishable; executable source closure stays unknown |
| Groups and comparison | Derived transparent membership and observed differences | Neither similar text nor a later pass proves a common cause or rewrites prior outcomes |

References and checkpoints have explicit validation boundaries. Corrupt or unavailable native
inputs, metadata, payloads and incompatible sidecars cannot silently become a clean empty answer.
Reader admission covers the independently disposable raw and receipt scopes. Named citation
holds protect originals and necessary context; an exported receipt alone is not raw-evidence
availability.

## 3. Composition, execution fit and alternatives

Capture allocates invocation-owned destinations before native launch and keeps direct file
capture under existing run ownership. Command completion is saved before native export, context
re-observation, serving registration or fixture restart. Unsupported composition retains the
original command and reports a separate capture gap. This preserves the native execution policy
and available parallelism without making successful capture a condition for a native pass.

Preparation streams native inputs into compact immutable metadata segments and a seekable
payload stream, publishing their committed prefix through a checkpoint. Unchanged preparation
reuses its result; JSONL append processes the new complete suffix. Selected reads inspect compact
metadata and hydrate/validate selected native and payload ranges. They do not decode unrelated
large output again. Metadata scanning per page remains proportional to case metadata, and the
implementation purchases a useful access boundary without adding a query service. Huge selected
native records still require validating that selected record; excerpt size is not an execution
resource limit.

The projection's question is exact membership and selected captured text under the recorded
adapter interpretation, retaining phase/attempt multiplicity. It is not a unique-execution count
or behavioral analysis. Failure grouping is a transparent grouping operation; comparison reports
observations and unknowns. Native timing/trace remains observational and does not establish CPU
attribution or a total causal order.

The simplest native alternative is retained logs/reportlog/JUnit plus native replay on demand.
It has fewer access products, but repeated selected reads must rediscover and decode unrelated
payload, and callers must independently reconcile native phase/result semantics. The chosen
projection removes that repeated interpretation and payload work. Its cost is duplicated normalized
text, checkpoint/segment validation and explicit retention. Those costs are justified by this
workload and remain local to verification-owned tooling.

Stdlib JSON/XML/file operations and pinned TOML composition discharge generic parsing and
configuration work. Native recording export/replay avoids a private ZIP reader. A telemetry
database, generic reporter or workflow framework would add services, lifecycle and semantic
duplication without a named capability gap. A supported native selective reader is worth
reconsidering if it can eliminate substantial projection duplication while preserving these
cross-tool contracts. No such replacement is required for the examined target.

## 4. Change and failure scenarios

| Scenario and kind | Owner and expected propagation | Assessment |
|---|---|---|
| New producer or phase — domain extension | New interpretation belongs in its adapter; reference/projection contracts change only for genuinely new meaning | Existing CLI consumes common metadata and payload operations; native semantics are not reconstructed in every handler |
| Substitute a native format — mechanism replacement | Adapter qualification plus actual support-version change | Preserve result/identity/loss semantics or refuse capture independently of test execution; no private format coupling is required |
| Interrupt, append, replace or corrupt input — lifecycle change | Invocation snapshots, committed prefixes and projection validation | Complete prefixes survive; new complete records append; changed generations invalidate incompatible continuation; corrupt membership is explicit |
| Relocate evidence or rebuild access — physical binding change | Export validation and projection reconstruction | Originals and semantic references survive independently of derived offsets; imported indexes do not become authority |
| Grow to thousands of cases and large outputs — workload growth | Compact metadata scans and selected payload/range reads | Credible operation-shaped access; no repeated unrelated payload decoding, resource caps or additional service machinery |
| Select a serving consumer — composition | Current Step consumption/production declarations, fixture owner and environment templates | Include real producers once, keep their filters independent, and expose fixture/artifact binding obligations rather than print a misleading bare consumer command |

## 5. Material findings encountered during integration

These IDs preserve this review's diagnoses. The [plan §8](../../plans/agent-test-evidence-and-causal-analysis-plan_2026-10-11.md#8-sole-current-finding-disposition-and-checkpoint)
owns scheduled disposition and functional closure; this table records the inspected correction,
not a competing execution ledger.

| ID | Diagnosis and consequence | Principles / gates | Inspected correction and revealing evidence |
|---|---|---|---|
| <a id="F01"></a>F01 | An integer event offset over freshly collected milestones/native rows could duplicate or omit observations after append, and continuation coverage could describe a newer prefix | FP-04/05; DP-19; G5/G6; CI-13 | `cli.events` freezes milestones, membership counts, generation/revision, filter and coverage; case continuations also retain pinned coverage. Control: `test_event_cursor_pins_native_prefix_milestones_and_coverage` |
| <a id="F02"></a>F02 | Root source hashes/original argv omitted effective external runner configuration; unavailable source context could erase the whole comparison row | FP-04; DP-21; G2/G7 | `capture.configuration` records normalized effective Nextest repository/user policy and selected external tool/Python config inputs. `compare` retains invocation rows independently of source-context availability and reports tool/config differences. Source-to-binary uncertainty remains explicit |
| <a id="F03"></a>F03 | Swallowed metadata-read errors could yield reduced membership while the unchanged-source preparation path still reported structured evidence available | FP-04/05; DP-03/19; G3/G5; CI-04 | `_rows` validates an invocation's metadata before yielding it and propagates explicit per-invocation gaps; independent invocations remain readable. Unknown manifest schemas are rejected rather than interpreted as current |
| <a id="F04"></a>F04 | Correct prerequisite argv alone omitted the environment bindings that make serving production and consumption meaningful | FP-03/04; DP-08; G2/G6 | `rerun.plan` derives `environment_templates` from current Step declarations, preserves conditional settings, and names current artifact/fixture/serving binding obligations without exporting credentials |
| <a id="F05"></a>F05 | An invocation aliased the boundary's mutable fixture dictionary; a successful restart could later serialize the earlier command under the replacement attachment | FP-04/05; DP-04/21; G1/G2 | `CaptureOwner.start` deep-copies launch fixture observations. Later restart and finalization remain separate observations; the launch attachment cannot change through shared mutable state |

Native duration/start/stop fields were also restored in the adapters instead of being silently
dropped by selective access. This is an adapter fidelity correction within the same ownership.

## 6. Independent judgments and gates

| Judgment | Verdict | Evidence and boundary |
|---|---|---|
| A1 Localize change | Satisfied | Native interpretation, access, lifecycle and selection have separate owners; producer changes do not require a new CLI semantic implementation |
| A2 Encode domain meaning explicitly | Satisfied | Invocation/native/capture/cleanup distinctions govern capture and reads; checked references, committed-prefix operations and the corrected fixture snapshot govern behavior |
| A3 Extend through composition | Satisfied | Existing launch, fixture, environment, storage and verification prerequisite owners compose without another orchestrator |
| A4 Fit execution to the workload | Satisfied | One streaming preparation, compact metadata and selected range hydration are credible for the declared workload; duplicate normalized text buys selective access and remains owner-local |

FP-01–FP-07 are satisfied for the examined scenarios after the inspected corrections. In
particular DP-01/04/05 authority, DP-08/09 transformation/reuse, DP-18/19 effects/publication and
DP-13–16 library/extension economy have concrete governing mechanisms. No numerical performance
claim follows from this qualitative assessment.

| Gate | Verdict | Own evidence or scope reason |
|---|---|---|
| G1 Authority | Pass | Native records govern results; projection governs access only; launch attachment is frozen |
| G2 Semantic fidelity | Pass | Phase/attempt/supplemental membership, native policy result and individual execution remain distinct; configuration and environment obligations are explicit |
| G3 Validity | Pass | Reference/range/checkpoint checks reject invalid access; damaged metadata and unsupported schema are explicit |
| G4 Hidden behavior | Pass | Inspection/planning perform declared derived preparation and retention only; no test launch, synchronization or operator activation |
| G5 Consistency and recovery | Pass | Checkpoints commit complete prefixes; continuations pin membership/coverage; original and reader lifetimes stay explicit |
| G6 Transformation and reuse | Pass | Rebuildable physical offsets do not become semantic identity; exact selection preserves obligations and current prerequisite closure |
| G7 Truthful capability claims | Pass | Producer versions and losses are named; unknown binary provenance, unsupported capture and incomplete execution are not promoted to success |
| G8 Library leverage | Pass | Native export/replay, reportlog, stdlib readers and TOML composition leave only the cross-tool contract gap bespoke |
| CI-G1 Fidelity | Pass | Supplemental starts/membership and heuristic grouping are not execution/causal proof; unavailable observations remain explicit |
| CI-G2 Evidence closure | Pass | Evidence references bind originals and interpretation; missing/replaced originals cannot masquerade as available evidence |
| CI-G3 Evaluation integrity | Not applicable | No product evaluator, private reference truth or protected-population tuning is changed; source observation excludes protected input bytes |

## 7. Verification, uncertainty and decision

**Interface-checked, 2026-10-11:** the reviewer inspected all new evidence modules, changed
verification/run consumers and native/focused controls in
`tests/scripts/test_verify_evidence_{nextest,pytest,projection,integration,cli}.py`.
The controls include native retention/self-pruning, native interruption, configuration precedence,
phase/attempt fidelity, selective range access, corruption/rebuild, exact selectors, dependency
closure and exported replay after original source/private-store removal. Reading their source is
not a claim that they passed in this review. Reviewer-executed tests: **not_run**. The coordinator
must retain the actual focused-control command/result and applicable scope-end leaves in the
plan's acceptance receipt.

A hard-stopped verification owner can leave preserved private native state and readable JSON
without reaching native UUID/export finalization. This review does not claim automatic native
export recovery in that case. Missing completion/export remains unknown or partial; native
recovery requires the existing supported native tools. Process interruption/prefix durability
does not establish power-loss durability. Historical binary reconstruction and complete ambient
environment equality are not promised.

**Rule impacts: none.** The recommendations repair the additive implementation within the
functional target; they require no replacement or retirement of a repository rule. ADR-0147
and the existing lifecycle owners remain the decision route, while the plan owns current
finding disposition. No additional acceptance gate, document type or permanent registry is
recommended.

**Bounded decision: Accept scoped**, at Interface-checked strength, for the standalone native
test-evidence and causal-investigation tooling target after the source corrections above.
The surrounding compiler, scientific/product behavior, operator adoption, paused failure
inventory and whole-product qualification are **not assessed**. Functional claims advance to
Tested only through the coordinator's actual focused receipts; productivity/performance remains
unmeasured.
