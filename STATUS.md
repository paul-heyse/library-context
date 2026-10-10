# Status

_Updated 2026-10-09 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: unified persistent SurrealDB UP0–UP9 execution in progress, 2026-10-09.**
[ADR-0143](docs/adr/0143-unified-persistent-content-and-execution.md) supersedes ADR-0138/0140
for accepted RC01–RC07. The [unified companion](docs/plans/unified-persistent-surrealdb-plan_2026-10-09.md)
owns packages; [persisted coordinator §8/§9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns Open findings and actual receipts. Shared-main changes implement immutable payloads/anchors,
exact views, fenced attempts, admitted attachment, manifest publication, immutable definition epochs,
data-only restore, native pins/retirement and stable service integration. Whole-operation portable
replay is retired; selected kernels retain partitioned portable products. Patched gRPC remains initial.
**passed:** current installer build, guard-free native refresh and `just ready`; actual owned-service
installation/check; simultaneous logical attachments; equal-definition retry/drift refusal; protected
cold backup/default restore validation/applied restore/final readiness. Four tooling modules passed123
controls; scoped tooling types passed. The schema migration snapshot was read, accepted and rerun.
Independent integration review approved examined source subject to runtime checks.
**failed / corrections in progress:** initial native controls exposed masked transaction errors,
obsolete cold payload reconstruction and invalid native lock targets. Search controls exposed stale
partial-fixture scope construction. Corrections compile; focused native/search/core/publication reruns
are active. UP9 `just qualify`, final extension refresh and leaves remain pending. No whole-plan
closure or measured benefit is claimed; BC3 remains separate.
The new owned service is installed outside checkouts; existing operator/PSE-arrow data, selection,
protected evaluation and real-library pilots remain held. Next: finish focused runtime repairs,
refresh final clients guard-free, then coordinated UP9 acceptance.

**Storage lifecycle plan refreshed, 2026-10-09:** the [plan §9](docs/plans/storage-lifecycle-management-plan_2026-10-09.md#9-finding-disposition-owner)
owns scheduled storage F01–F03; the [independent target review](docs/design_review/reviews/design_review_storage-lifecycle-plan_2026-10-09.md)
accepts the corrected **Proposed / scoped** design. Operator-confirmed choices preserve warm
caches, detailed profiling and performance settings, and select automatic rules-based lifecycle
management across repository and shared host assets. Target-review F01/F02 are resolved in the
Proposed target; operational closure remains open. AF1 retains its delivered cleanup/recovery slice.
The plan now covers persistent-service attachments, native/recovery ownership and the maintenance
executable's checkout dependency. Next: SM0/SM1 consume AF1/current service observations, then
producer/dependency migration, legacy adoption and qualified automation. BC3/evidence stay protected.
Cleanup, archival, configuration/agent-policy implementation and product controls **not_run**;
this scope changes the plan only. Original publication **passed:** `just docs`,369pages, zero link errors;
scoped `git diff --check` passed. Full `just docs-check` **failed** on the already-stale concurrent
ADR index, which this task leaves untouched.

**Current scope: native-execution efficiency NE0–NE9 implemented; coordinated acceptance open.**
The [native-efficiency plan](docs/plans/native-execution-efficiency-plan_2026-10-09.md)
owns execution scope. [Persisted coordinator §8/§9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07)
is the sole scheduled-finding disposition and verification-receipt owner. ADR-0141 records
accepted RC01–RC05, exact membership access, shared immutable preparation, scoped producing
completion, stable live flights and explicit Selected/Complete observation. Independent source
reviews accepted the corrected examined implementation; integrated NE9/GK7/GR6 remains open.
GK0–GK6/GR0–GR5 are committed in `2a9a3771`; SDK backport is committed in `18ae9076`.
Current NE implementation and follow-up corrections are committed in `05cc139b`; integrated
acceptance remains open.

The SDK3.3.0 now has a narrow vendored gRPC physical-terminal backport. Successful application
End retains the transport through checked EOF and late errors. SDK3.3.2 retains the same gap.
PSE-arrow's patched WebSocket buffers query results; progressive server `query_stream` would
need an adapter with equivalent cancellation and backup support. PSE-arrow is unchanged.
No dependency version or public artifact schema changed; [pins](docs/pins.md) owns the SDK hold.

**Agent effectiveness AF0–AF9 implemented / focused-Tested, 2026-10-09:** the
[follow-up plan §8/§9](docs/plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner)
owns F01–F05, AE-24, selected capabilities and current receipts. ADR-0142 governs owned readiness,
independent cleanup and detachable display. [Independent integrated review](docs/design_review/reviews/design_review_agent-effectiveness-integrated_2026-10-09.md)
accepts the examined scope after closing its additional fixture-lock/retention and leader-reaping
findings. Read-only freshness, checked SQL diagnostics and invocation-scoped native Codex MCP
are implemented; process skills and permanent client registrations are unchanged.
**passed:** nine affected tooling modules,313 controls (`20261009T231848.005Z-9bf4ac`);
final fixture correction selection,20 controls; post-cleanup harness/run/verify,139 controls;
publisher/ADR `just docs-test`,78 controls. Plan §9 retains exact scopes and earlier failures.
Scoped Ruff/types, `just adr-lint` and `just docs-check` **passed** (374 pages, zero link errors).
Comprehensive product tests and measured effectiveness **not_run**, per operator instruction.
Next: reuse these owners in storage/unified-service work; existing product acceptance stays separate.

**Earlier verification boundaries, 2026-10-09:** exact commands, source limits and failed composites
remain in [coordinator §9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07).
Patched native transport controls and extraction determinism passed focused reruns. Earlier assembled
qualifications remain failed/cancelled partial receipts, including disk exhaustion, compiler/store/MCP
failures and cache-matrix/analytics timeouts. Those results do not close the unified target. Current
native ownership, search, restore and representative journeys must pass on the replacement source.
BC3 candidate-profile qualification and unrelated real-library adoption remain `not_run`.

**Existing qualification follow-up:** resolve source-specific failures without weakening controls.
Qualify BC3 separately before
installing the ordinary test profile. Record source-specific closure; retain failed composites.
The operator restored40GiB free after the filesystem fell below1GiB; focused validation resumed.
No measured speed or whole-plan completion is claimed.

**Preservation and boundaries:** preserve operator hashing references, PC3's dirty isolated
planner-control copy, shared Cargo intermediates and compiler/profiling/benchmark captures.
No operator databases/configuration/client registration, protected gold/heldout, Qwen, real
FastMCP/Q1 pilot, wheels, activation or push. Commands use normal available compiler/test
parallelism and unchanged stacks/timeouts; native checks borrow the installed validation database with logical isolation.
Maintenance follows [AGENTS.md](AGENTS.md#commands).
