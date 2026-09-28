# PR0 evaluation boundary review

## 1. Scope and outcome

**Change/conformance review; Interface-checked, 2026-09-28. Decision: Revise the trial boundary; accept the sealed packet only as an unqualified candidate.** This is a bounded evaluation review, not acceptance of PR1, the assembled product, or comparative value.

| Field | Scope |
|---|---|
| Standard | Core 3.0, code-intelligence profile 1.1, and the [repository binding](../design_principles/binding/library-context.md) |
| Reviewer | Independent `/root/pr0_confirmation_review` curator/reviewer session |
| Authority | [Product §14.12](../../design/sections/api-and-evidence-product.md#section-14-12), [PR0 queue and detailed execution](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) |
| Code examined | [protocol.json](../../../eval/product/fastmcp-4.0.5/protocol.json), SHA256 `bbcfbbb92cea174109a0227f0771b8e06802f0ca7f88a01f3d27bfd74aac5e02`; initial [product_eval.py](../../../scripts/product_eval.py), SHA256 `61b867feb3351a00d9f699df8171d4a05213e288c58ce9bfbe14f2c0161036d8` |
| Dirty-tree boundary | Concurrent PR1 implementation is in progress. Findings below concern the reviewed initial CLI runner; a replacement requires its own closure evidence. No production code edited by this reviewer. |
| Excluded inputs | Development tasks, behavioral gold, heldout, capability library skills, generated product catalogs and implementation task answers were not read. |
| Execution | No agent trials, task execution, formatting, lint or integrated gates. One pure telemetry probe and candidate integrity checks ran. |

The frozen protocol retains the required A/B/C conditions, model/reasoning, budgets, task counts and engineering acceptance thresholds. It correctly reserves confirmation execution for PR6 and keeps task success distinct from an unscored process exit. Its status text alone cannot establish when an input was frozen or whether the population is comparable.

### Responsibilities and expected changes

The evaluation owner authors protocol, task criteria and condition definitions. The runner owns controlled model/tool effects and complete attempt receipts; it does not judge its own answers. Independent graders consume submitted artifacts, original evidence and sealed checks. The product compiler must remain independent of all task answers. The original source revision owns library facts; the structured product is one experimental condition, not the oracle.

The relevant change scenario is replacing condition B's evidence service with condition C while keeping model, prompt, original corpus, environment and budgets fixed. That should require only a checked condition binding. In the reviewed runner, arbitrary CLI configuration and ambient environment can also change the tool universe, instructions and reasoning, so the boundary does not yet preserve that experiment. A second scenario is a missing/partial provider response: it must remain an observed incomplete attempt, never acquire zero usage by default.

| Fact / artifact | Authority and fidelity | Consumer and limit |
|---|---|---|
| Task rubric | Independent curator from pinned original evidence; authored evaluation criterion | Independent scorer; never compiler/retrieval input |
| Task count / strata | Frozen protocol; mechanically checked population | Trial planner; a count does not establish semantic distinctness |
| Condition identity | Explicit configuration plus independently observed release/corpus/tool availability | Comparability admission; an operator boolean alone is insufficient |
| Attempt outcome and costs | Provider/tool observations, with missing values explicitly unavailable | Score/cost aggregation; process completion is not task success |
| Sealed bytes and hashes | [Candidate manifest](../../../eval/product/fastmcp-4.0.5/confirmation-manifest.json) | Integrity/custody only; no execution or superiority claim |

## 6. Gates

These verdicts apply to the reviewed runner snapshot, not to the replacement under development.

| Gate | Verdict | Evidence / boundary |
|---|---|---|
| G1 Authority | unresolved | Protocol authority is clear; freeze membership and lifecycle admission remain incomplete (F03/F04). |
| G2 Semantic fidelity | failed | Missing telemetry becomes numeric zero and an empty model list (F02). |
| G3 Validity | failed | Isolation/parity claims are unchecked booleans; deployment membership is not constrained to implementation tasks (F01/F03/F04). |
| G4 Hidden behavior | failed | CLI overrides and copied ambient environment can affect the experimental condition (F01). |
| G5 Consistency/recovery | failed | A truncated JSONL record can prevent final receipt publication; subprocess descendants have no owned termination contract (F02). |
| G6 Transformation/reuse | unresolved | Same corpus/environment/model across conditions and reliable budget accounting have no runtime receipt. |
| G7 Truthful capability claims | unresolved | Explicit `not_scored` is sound; complete model/usage observation and fair-population admission remain missing. No superiority was claimed. |
| G8 Library fit | passed for bounded design | Standard JSON/hash/subprocess mechanisms are proportionate. An explicit API tool loop is a viable simpler isolation alternative; runtime qualification remains required. |
| CI-G1 Fidelity | unresolved | Negative-claim rules exist, but independent scoring has not run. Missing observations must remain unknown. |
| CI-G2 Evidence closure | not applicable to product serving | No served catalog claim was reviewed. Candidate evidence references were hash/range checked against original bytes only. |
| CI-G3 Evaluation integrity | failed for runnable comparison | Instructions cannot exclude shell-accessible answers or ambient context by themselves (F01). Candidate custody is separate from trial isolation. |

## 7. Findings

The [forward plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition) is the disposition owner when these are scheduled. The IDs below retain their original evidence and are not mutable status fields.

<a id="F01"></a>
### EVAL/F01 — Condition configuration does not enforce the experiment boundary

**High; FP-01/02/05/06, DP-03/18/21, CI-10/12; A1/A2, G3/G4/CI-G3.** In the reviewed runner, `isolation_verified` and `release_alignment` are trusted fields, `codex_config` accepts arbitrary strings, and they are appended after the purported fixed reasoning configuration (`run_trial`, original lines 85–118). The child inherits `os.environ.copy()` (line 126). Read-only sandboxing does not establish that only the declared evidence is readable; the output receipt itself includes task checks. Prompt instructions are not a tool or filesystem allowlist.

A condition change can therefore change instructions or reasoning and expose undeclared tools/files while retaining the same A/B/C label. This prevents attribution of an outcome to structured evidence. Correct the runner-owned condition boundary: accept a finite typed configuration, construct exactly the permitted model/tool invocation, isolate state and filesystem capabilities, and record the effective condition and corpus identity. Delete the arbitrary CLI override path. The implementation author selected a minimal API function-call loop after this finding; that selection is not closure.

Closure requires an observed effective-tool inventory and runtime attempts that cannot access excluded tools/files, plus rejection of attempted model/reasoning/tool overrides and a receipt retaining the exact condition binding. Do not use the confirmation packet for these controls.

<a id="F02"></a>
### EVAL/F02 — Incomplete observations can look budget-compliant or lose their terminal receipt

**High; FP-02/05/06, DP-02/19/20/21/22; A2, G2/G5/G7.** `events_summary` initializes usage and calls to zero, takes usage only from recognized completed events and does not require model identity. The focused probe `events_summary([], limits)` returned zero usage, zero calls, no model and `budget_exceeded=false`. A zero-exit process with missing telemetry can consequently produce a `completed` receipt whose budget evidence is absent. Reading JSONL after timeout has no malformed-tail handling, so a partial record can raise before `receipt.json` is written. Only the direct subprocess is governed by the timeout; descendants have no recorded lifecycle owner.

The attempt recorder must distinguish observed zero from missing usage/model identity, count attempted tool effects, retain terminal failure/blocked receipts on every path, and own timeout/cancellation of all effects. For an API replacement, apply the same requirements to provider errors, missing usage, unsupported model, tool failures and incomplete responses. A performer exit or answer cannot score itself.

Closure: independent missing-usage, wrong-model, failed-tool, timeout and partial-response controls leave inspectable receipts and cannot become successful budgeted trials. Full task scoring and paired comparison remain PR6 work.

<a id="F03"></a>
### EVAL/F03 — Sealing does not establish a comparable frozen confirmation population

**High for comparative claims; FP-04/05, DP-03/21/22, CI-12; A2, G3/G7/CI-G3.** Product §14.12 requires original evidence availability at the same target release across A/B/C before the confirmation population freezes. The inspected protocol correctly calls comparison blocked, but no task-level parity attestation was available. The source checkout matches the requested revision; that says nothing about Context7 access to materially comparable original evidence. A new sealed candidate also cannot retrospectively satisfy the planned pre-implementation timing when concurrent implementation has already begun.

The candidate manifest therefore records `sealed_candidate_comparison_blocked`, the actual exposure/timing boundary, and no execution. Independently establish material evidence parity for every task, retain all 24 in the denominator, and attest development/confirmation distinctness through a separate non-implementer. Keep diagnostic unmatched cases distinct. If comparable coverage cannot be established, comparative acceptance stays blocked; do not replace difficult cases after observing outcomes. The original candidate may stay sealed as an unqualified asset.

Closure requires a dated non-answer-bearing availability/distinctness attestation, protocol/freeze hashes and honest lifecycle declaration before any scoring. Confirmation execution remains PR6; no claim of PR0 confirmation qualification follows from ciphertext creation.

<a id="F04"></a>
### EVAL/F04 — Task-shape and freeze checks omit required membership invariants

**Medium; FP-04/05, DP-03/24; A2, G1/G3.** `load_tasks` counts any `deployment` flag across all strata (original lines 34–35), although the target requires at least two deployment tasks among the eight implementation tasks. `check_freeze` validates whichever entries occur in `freeze["sha256"]` without requiring that all mandatory frozen artifacts be present (lines 48–55). An omitted protocol/seal input or misplaced deployment flags can pass a check named `check-freeze`.

Keep the minimal validator, but make the required artifact set and task-shape invariants explicit and versioned. Count deployment only within implementation; distinguish candidate custody verification from comparable-population freeze admission. Do not infer that non-empty prose checks are executable or independent.

Closure requires negative controls for misplaced deployment flags, omitted required hash entries and changed mandatory bytes. The candidate itself has two implementation deployment tasks; this finding concerns the shared validation boundary.

## 8. Library fit and sealed candidate

The curator used the installed `cryptography.fernet.Fernet`, rather than bespoke cryptography. The committed ciphertext and manifest contain no task prompts or answers. Plaintext and curator working material stay under the protected, ignored `build/product-eval/sealed/` directory; the key stays in operator configuration with mode `0600`. The protection prevents accidental context/commit disclosure; it does not make the key inaccessible to another process running as the same operator.

The manifest records 24 tasks: eight discovery, eight implementation including two deployment tasks, four choice and four ambiguity tasks. All have original source-backed criteria and separate false-claim checks; all implementation tasks have independently authored executable assertions. Assertions were syntax-checked only, not executed or benchmark-qualified. Every cited file was compared to Git-tracked bytes at revision `004bf15a2ba99f077160993c00a404e1a9da83ea`; untracked generated blocks were excluded. Exact Context7 parity and semantic distinctness from the unseen development set remain unestablished.

**Codex configuration inspection, Interface-checked 2026-09-28.** Installed `codex-cli 0.157.1` help supports `--ignore-user-config`, `--ephemeral`, `--ignore-rules` and explicit overrides, but states that ignoring config still uses `CODEX_HOME` for authentication. Official documentation exposes shell, memory, app, collaboration and web-search controls plus per-MCP tool allowlists. It does not establish a single switch that removes every other tool. Context7's upstream source excerpt shows `apply_patch` gated independently by environment/model metadata, so `features.shell_tool=false` alone is insufficient. A custom model-catalog route was not qualified. [Official configuration reference](https://learn.chatgpt.com/docs/config-file/config-reference), [upstream tool construction](https://github.com/openai/codex/blob/main/codex-rs/core/src/tools/spec_plan.rs).

The simplest viable comparison runner can send only the condition's explicit function schemas to the fixed model and execute only allowlisted calls. This avoids general CLI tool discovery, but still needs exact model/access, source availability, budgets, tool-error handling, environment and receipt controls. No runtime isolation claim is made for this proposed replacement.

### Verification

| Check / command | Outcome on 2026-09-28 |
|---|---|
| `git -C build/sources/fastmcp/004bf15a2ba99f077160993c00a404e1a9da83ea rev-parse HEAD`; per-citation `git show <rev>:<path>` comparisons | **passed:** requested original revision and every selected original file matched; not Context7 parity |
| `uv run python build/product-eval/sealed/curate_confirmation.py`, followed by isolated source/range/hash/permissions and Fernet round-trip assertions | **passed:** 24 tasks, 8 executable assertion sets, required strata and deployment count, source ranges/hashes, ciphertext/plaintext hashes and protected custody; pre-delivery draft preserved only under the sealed directory |
| `uv run python -` pure `events_summary([], limits)` probe | **passed as defect reproduction:** missing telemetry was represented as zero and budget-not-exceeded |
| `codex --version`; `codex exec --help`; official documentation and Context7 upstream source lookup | **passed for interface inspection:** CLI controls checked; complete MCP-only isolation remains unresolved |
| Benchmark agent trials, implementation/deployment assertions, confirmation scoring, integrated gates | **not_run:** outside this curator/review authorization |

## 12. Architectural judgment and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | violated in reviewed runner | A condition binding also exposes unrelated CLI configuration and ambient state; evidence-service changes do not stay local (F01). |
| A2 Encode meaning structurally | violated | Isolation/parity and absent telemetry are encoded as assertions/defaults without their admission contracts (F01–F04). |
| A3 Extend through composition | unresolved | The small evaluation module is a suitable owner, but condition substitution and independent attempt/scoring composition have no controlled runtime evidence. |

**Bounded decision: Revise.** The required correction is an explicit experimental-condition and attempt-result boundary in evaluation tooling, not a new framework. **Accept scoped** applies only to candidate custody and structural verification. Original-source parity within the selected checkout is established; fair A/B/C parity is **blocked**, and confirmation execution and comparative success are **not_run**. The enclosing product architecture and PR1 implementation were not assessed.
