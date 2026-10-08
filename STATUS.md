# Status

_Updated 2026-10-08 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: compiler introspection and Rust compilation-cost BC0–BC5 execution, 2026-10-08.**
The [plan](docs/plans/rust-compilation-costs-plan_2026-10-08.md) owns all compilation-cost findings (§9).
[ADR-0137](docs/adr/0137-qualified-local-compilation.md) records confirmed RC01/RC02 and the qualified
local-default transition. The [target review](docs/design_review/reviews/design_review_rust-compilation-costs-plan_2026-10-08.md)
accepts this architecture. BC0 decisions are implemented; BC1–BC5 implementation/acceptance is in progress.
Available compiler/test parallelism and optimized production acceptance remain requirements;
current verifier defaults remain release until candidate qualification. PC dirty work is preserved.
**passed (authoring, 2026-10-08):** `just docs-check` (343 pages, zero link errors), exact grouped inventory.
**passed (implementation, 2026-10-08):** locked core/extraction all-test compile check and 56 focused verifier/build-environment Python controls. BC1/BC2 source and BC4 groups are integrated on the preserved PC baseline.
**Interrupted:** owned candidate compilation stopped before tests for operator-authorized diagnostics; no test verdict.
**passed (tooling, 2026-10-08):**83 controls, actual sampling/rotation/owner-death retention, tiny Cargo/nextest timelines, types/lint/docs ([plan §11](docs/plans/rust-compilation-costs-plan_2026-10-08.md#11-opt-in-compilation-diagnostics)).
**Stopped / retained:** diagnostic candidate `20261008T181813.329Z-ddd158`, no tests started. Both partial compiler profiles decode;42 sample chunks retained. **Measured hotspot:** C1 producer LLVM PHI optimization ([plan §12](docs/plans/rust-compilation-costs-plan_2026-10-08.md#12-interrupted-capture-assessment-and-remaining-correction)). **Next:** C1 coroutine/residual adapter correction, candidate qualification, BC3; **not_run:** BC5 release acceptance.

**Persisted graph correction packages PC0–PC6 remain pending acceptance.**
The [persisted graph execution plan](docs/plans/persisted-graph-execution-plan_2026-10-07.md)
owns this scope and all scheduled source-review finding dispositions. [ADR-0133](docs/adr/0133-persisted-graph-compilation.md)
accepts native persisted compilation and direct sealing. Implementation receipts below are bounded;
this pivot is not accepted or measured yet.
The [correction plan](docs/plans/persisted-execution-corrections-plan_2026-10-07.md) integrates all six
correction-causes findings, secondary capabilities and investigations. Both RC01/RC02 are accepted.
The [independent target review](docs/design_review/reviews/design_review_persisted-execution-corrections-plan_2026-10-07.md)
accepts its Proposed architecture; [coordinator §8](docs/plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns the still-open findings. ADR-0135 installs the accepted operation-contract target. Production
migration and focused acceptance are in progress; no finding is closed by the decision alone.

**Execution resources, 2026-10-08 (carried forward by ADR-0137): implemented / focused tested.** Available Cargo/compiler/test parallelism; disposable SurrealDB defaults to16GiB with8GiB tracked threshold and explicit overrides.
**passed (original resource scope):** fixture/build-helper pytest, automatic-worker probe, actual fixture startup, ruff, lint-agents, adr-lint and docs-check (340 pages, zero link errors).

**Workspace effectiveness, 2026-10-08: implemented** with design-phase acceptance ([plan](docs/plans/agent-workspace-effectiveness-plan_2026-10-07.md):
§9 dispositions, §12 checkpoint; [ADR-0134](docs/adr/0134-scoped-preparation-and-maintenance.md); [implementation review](docs/design_review/reviews/design_review_agent-workspace-implementation_2026-10-08.md) findings fixed).
Commands and readiness follow [AGENTS.md](AGENTS.md#commands); `just fresh` reports 103 committed unformatted Rust files (AE-23) for a whole-tree `turn-end`.
**Checks, 2026-10-08:** **passed** lint-agents, adr-lint, types, deps, ruff (harness) and docs-check (338 pages), plus harness unit tests and each packet's premise/value checks (plan §9).
**not_run:** qualify, wide families and the full mcp journey (design-phase scope); whole-tree turn-end (paused files dirty; the scoped form was used).

**Integrated on main:** immutable native contributions/views and frozen bindings; typed codec/Arrow bridge;
all compiler frontiers/profiles, selected analytical preparation and native selected closure; shared
serving indexes; direct sealing; artifact/backup/restore/inspection; bounded final encoding and readiness.
[Plan §9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07) owns verification and remaining scope.

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

**Current execution:** PC0 is committed; PC1–PC5 production/contracts and their controls are
integrated as dirty main changes. Independent review corrections cover cold table inventories,
charged member retention, canonical input companions and physical member hydration.
Authorized Cargo lock-cycle recovery is recorded in coordinator §9.1; test verdicts remain pending.
PC6 fresh restored coverage, both-profile/frontier and native/MCP/evaluator acceptance remains open.
The coordinator §9.1 owns current receipts. PC3's isolated worktree remains pending retirement
once its integrated changes and acceptance are secured. Operator stores/configuration, Qwen,
protected gold/heldout, shared caches and unrelated host processes remain preserved.
No wheel, real FastMCP/Q1 pilot, performance campaign, operator adoption or push is requested.

The earlier ER/EV and graph-native audit receipts remain with their original coordinators:
[evidence/evaluation §6](docs/plans/evidence-retrieval-and-evaluation-plan_2026-10-06.md#6-finding-disposition-investigation-outcomes-and-completion),
[graph-native §7/§8.1](docs/plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities).
The [evaluation plan](docs/plans/programmatic-evaluation-plan_2026-10-06.md) continues to own the
independent primary programmatic loop, with agentic input to both system and evaluator revisions.
Earlier FastMCP catalog attempts were stopped without a verdict; real Catalog/Behavioral pilots,
Q1 usefulness and selected-snapshot adoption remain held.
