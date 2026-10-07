# Status

_Updated 2026-10-07 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: persisted graph pivot PG0–PG9 implementation is paused for design review.**
The [persisted graph execution plan](docs/plans/persisted-graph-execution-plan_2026-10-07.md)
owns this scope and F01–F05/D01–D04 disposition. [ADR-0133](docs/adr/0133-persisted-graph-compilation.md)
accepts native persisted compilation and direct sealing. Implementation receipts below are bounded;
this pivot is not accepted or measured yet.

The requested [correction-causes review](docs/design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md)
assesses recent implementation failures, architectural causes and existing-library capabilities.
Its verdict is **Revise**, with six findings and two explicit design choices. It owns its new
unscheduled findings; the execution plan retains existing scheduled dispositions.
The review does not authorize remediation or resume the paused acceptance runs.

**Review documentation, 2026-10-07:** **passed** `UV_NO_SYNC=1 just docs-check` (335 pages,
zero link errors after repairing the stale ADR index and one retired historical source link).
Product tests/builds/probes **not_run** for this source-and-receipt review; paused edits preserved.

**Workspace capability review, 2026-10-07:** [follow-up review](docs/design_review/reviews/design_review_agent-workspace-effectiveness-capabilities_2026-10-07.md)
concludes **Revise** on the [workspace proposal](docs/plans/agent-workspace-effectiveness-plan_2026-10-07.md).
It owns unscheduled corrections and capability additions pending proposal revision; no harness,
runtime configuration or memory changes were implemented. Next: revise the proposal's affected
contracts. Product tests/builds/fixtures **not_run** for this documentation-only scope.
**passed:** `UV_NO_SYNC=1 just docs-check` (336 pages, zero link errors) and `git diff --check`.
`just turn-end` **not_run**: its whole-tree formatter would modify the paused concurrent edits.

**Integrated on main:** immutable native contributions/views and frozen bindings; complete typed
codec/Arrow bridge; all compiler frontiers/profiles and selected analytical preparation; native
selected closure; shared request-local serving indexes; direct same-database sealing;
complete artifact/backup/restore and inspection; bounded final encoding; native readiness routes.
[The plan §9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07)
owns actual verification and remaining scope.

**Current verification, 2026-10-07:** the plan §9.1 owns commands, logs and boundaries.

- **passed:** exact model contributions/views, identity/encoding/declaration controls and
  twelve readiness-launcher controls. The fixtures retain independent expected values.
- **passed:** affected native persistence, reconciliation, original-owner and lifecycle controls,
  including actual full4096 values and the original default-stack cancellation boundary.
- **passed:** declaration-derived backing for all88 required non-graph families, opaque snapshot
  transport and byte-exact numeric canonicalization; empty input retains explicit no-scope.
- **passed:** actual4099-row selected closure under its unchanged4MiB native read budget,
  unrelated4MiB body, corruption, pending exclusion and stream-lifetime assertions. Subsequent
  point-reader/grouped-field corrections also passed both final native closure controls (173.38s).
- **passed:**23 pure selected serving controls, actual capability-union isolation, native
  missing-store refusal, CLI early option/destination refusals and exact bounded wire controls.
- **passed:** fresh dedicated native `compiler_views`, eight controls on `32a2b9fb`, including
  >128 key/field demands, residual filtering, frozen overlap and coherent membership renaming.
- **passed:** current point-source Rust/Python development bridge build, atomic installation,
  fresh-process import/exit and `just ready`. No wheel or environment synchronization.
- **passed:** all-target Clippy on the preceding prepared-query/serving source (1m29s).
  Latest parser/core changes, formatting, generated ADR/features and docs leaves remain open.
- **passed:** all three native closure controls and the actual CallScope orphan/skew control.
  Exact owner presence retains the unchanged large-read budget and frozen epochs.
- **passed:** finite model parameter-render and S0 named-parameter selection controls; latest
  core/lib/affected-test compile check. Revised availability reuse control passed on `c327adba`;
  recorded-provider admission is being corrected and needs a subsequent focused rerun.
- **passed after prerequisite correction:**35 actual Python MCP/native-evaluator selections on
  the retained owned snapshot; first selection33passed/2failed, both fault controls reran passed.
  Persistent restart passed. This is bounded native evidence, not a clean assembled gate.
- **partial / paused:** final compiler, CLI, external publication/transport and actual native
  MCP/programmatic evaluation. No enclosing acceptance or measured speed claim is established.

**Corrections integrated:** locked native planner inspection found that multi-value `IN` could
retain only the compound relation prefix. Selected reads and overlap counts now use bounded
writer-derived membership RecordIds and verified contributor metadata. Atomic field windows
use their scope index; residual filters and exact frozen membership remain. Independent review
confirmed the subsequent cancellation-resumption ownership and cold membership-ID corrections.
Forward traversal fields now share one projected read over the exact table/frontier; both
actual closure controls pass, including nullable links and distinct target epochs. Actual
`EXPLAIN` found inline scope expressions bypassing its index; preceding compact native constants
correct the plan shape. Expanded atomic Eq/IN/OR controls pass (`9d7f4d27`); the updated development
bridge builds, atomically installs and imports successfully.

**Remaining acceptance:** earlier publication controls exposed omitted backing and a synchronous
coverage read on a current-thread client runtime; both source defects are corrected. Older stopped
publication/analytic diagnostics are not acceptance. Temporary coreO0 controls exceeded their
8MiB test/main stacks; current controls use child-only32MiB stacks, with unchanged production
profiles and runtime settings. The preceding CLI rerun completed six profile/frontier cases,
then failed Behavioral Analysis with unconfirmed cleanup. Its error path now preserves both
original and cleanup failures; the current-source rerun remains pending. The first direct extension
installer replaced its own loaded mapping and crashed; the corrected atomic installer passed.
Readiness/check logs distinguish these failures from successful subsequent controls.
The final native journey completed compilation/admission/direct sealing and all ten tool assertions;
its backup passed, then restore hit the20s HTTP import deadline. The second browse-scope/vocabulary
journey passed. Exact pinned statement parsing now bounds imports while keeping transactions whole;
parser controls and current CLI build pass. Retained-snapshot restore got through import, cold
state validation and native copy, then failed exact expected coverage during semantic admission.
That mismatch remains under investigation; no check or runtime limit was weakened.
The stopped older analytic failure was optional producer replay after compilation, not production
admission. Generic fixtures now use semantic admission; selected diagnostics remain explicit.
S0's missing incoming parameter-link selection is corrected and independently reviewed.

**Next:** decide the review's corrective direction and plan its selected findings before resuming
implementation. Remaining acceptance: resolve restored coverage, then finish targeted both-profile/frontier, direct-seal,
external-state/original transport and actual native/MCP/programmatic evaluation controls;
resolve independent findings, update dispositions/docs and run applicable leaves.
All worker ancestry is merged on main, and only the main checkout remains. Fully merged worker
worktrees and their branches have been removed. Operator stores/configuration, Qwen,
protected gold/heldout, shared caches and unrelated host processes remain preserved.
No wheel, real FastMCP/Q1 pilot, performance campaign, operator adoption or push is requested.

The earlier ER/EV and graph-native audit receipts remain with their original coordinators:
[evidence/evaluation §6](docs/plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md#6-finding-disposition-investigation-outcomes-and-completion),
[graph-native §7/§8.1](docs/plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities).
The [evaluation plan](docs/plans/programmatic-evaluation-plan_2026-10-06.md) continues to own the
independent primary programmatic loop, with agentic input to both system and evaluator revisions.
Earlier FastMCP catalog attempts were stopped without a verdict; real Catalog/Behavioral pilots,
Q1 usefulness and selected-snapshot adoption remain held.

**Prior plan-authoring documentation,2026-10-07:** **passed** `UV_NO_SYNC=1 just docs-check` (329 pages, zero link errors after one anchor correction), `UV_NO_SYNC=1 just lint-agents` and `git diff --check`. Product tests/builds/pilots **not_run** for plan authoring; preexisting open edits preserved.
No push is requested; shared caches and unrelated host processes remain preserved.
