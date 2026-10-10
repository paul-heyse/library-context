# Status

_Updated 2026-10-10 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: unified persistent SurrealDB UP0–UP9 execution in progress, 2026-10-10.**
[ADR-0143](docs/adr/0143-unified-persistent-content-and-execution.md) supersedes ADR-0138/0140
for accepted RC01–RC07. The [unified companion](docs/plans/unified-persistent-surrealdb-plan_2026-10-09.md)
owns packages; [persisted coordinator §8/§9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns Open findings and actual receipts. Implementation spans immutable payloads/exact views,
fenced attempts, checked retained attachment, manifest publication, executable definition epochs,
data-only restore, reader pins/retirement and one stable service. Patched gRPC remains initial.
**passed:** owned installation/check, simultaneous logical attachments, equal-definition retry/drift
refusal, protected backup/default validation/applied restore and readiness. Current guard-free
`just ready` rebuilt the formatted batching source and all readiness checks passed. Tooling controls and schema migration
passed; actual receipts and source boundaries remain in the coordinator. Independent integration
review accepts examined source subject to remaining runtime acceptance.
Focused corrections passed ordering/acknowledgement17, exact-view/state-closure controls,
projection immutable-collision refusal, sparse role corruption and multi-window ownership cleanup.
Indexed canonical point selection, scoped reconciliation, exact lexical document nomination and
prepared-stream terminal dispatch preserve full actual-state checks. All12 search controls have
passed, including corrected occurrence mutations that prove an actual physical change.
Publication passes sealing, bounded cleanup and readback; subsequent listing rejected an old
malformed synthetic publication from a retention test. The test now uses a generic native guard.
Explicit drained maintenance retired the one exact obsolete marker through native APIs, preserved
private row/source/progress evidence, and checked reopening. The temporary repair helper is removed.
**remaining acceptance:** final CLI build, generic-root retention and committed acknowledgement-
discard/session-loss reconciliation passed. Publication's indexed point selection passed; its dump
omission helper handles bounded empty/multirow frames. The rerun passed its exact-pin panic cleanup
check, then exposed exported FIELD OVERWRITE metadata comparing unequal to installed INFO.
Grammar-owned comparison correction passed compile, eight pure controls and independent review.
Its rerun passed entity-omission refusal, then timed out decoding the next large shared-store dump.
Terminal-owner cleanup passed its multi-window retention/receipt control. Selected closed-content
backup preparation passed compile,23 pure controls and independent review;
full publication/concurrent restore remain pending. Buffering passed compile/24 pure controls/review;
its native rerun completed compact backup/three omission refusals before timing out in valid restore.
Scoped terminal cleanup passed interruption and large admitted/unadmitted controls, but its elapsed
phase remains slow. Imported membership roots and test decomposition are implemented; their focused
selection finished9passed/8timeouts, including a restore ordering refusal before timeout.
Legal singletons now spill with unchanged bounds;14 pure ordering controls passed. Bounded view
registration is statically accepted; both new native ownership controls passed. Publication rerun
finished9passed/7timeouts. Canonical marker ordering, portable restore framing and guarded retirement
windows are implemented/static-reviewed; their focused native/pure controls are running. The last run's
two identified reader pins were recovered through checked maintenance. Fresh MCP native journeys failed at the missing executable epoch;
explicit checked maintenance installed current933b… names, preserved prior functions and reopened.
Guard-free final-source `just ready` passed, including the rebuilt native extension. NativeSession
close drains calls, releases its pin and shares terminal results across closers;
compile/static review passed, actual Python acceptance is not_run after the failed MCP producer. Earlier eight
maintenance controls finished6passed/2failed before intended drift assertions; all eight need a
final-source rerun. Next: fresh retained MCP producer/Python suite, maintenance8, two-worktree
reuse, populated backup/restore/restart, then unfiltered UP9 `just verify --qualify --cli` and applicable leaves.
No whole-plan closure or measured benefit is claimed; BC3 profile adoption remains separate.
Operator/PSE-arrow data, selections, real-library adoption, shared caches and recovery evidence stay protected.

**Storage lifecycle implementation in progress, 2026-10-09:**
[ADR-0144](docs/adr/0144-managed-storage-lifetimes.md) governs warm-cache preservation and
owner-backed retirement; [plan §9/§10](docs/plans/storage-lifecycle-management-plan_2026-10-09.md#10-current-checkpoint)
owns findings and actual acceptance. Policy, lifecycle/admission/journals, producer integration,
repository checkout bindings and archive/restore mechanisms are implemented with focused
controls. The bootstrap uses pinned Python3.14.7 without the project venv; no3.12 helper.
Only this repository and its two checkouts are registered; eight legacy roots and five retained captures have
protective descriptors. Central skill stores and other repositories are excluded.
No production content was removed and automation remains off.
**passed:**163 producer controls,63 lifecycle/build/service controls,34 archive/CLI/replay controls;
native acquisition compile/five controls; scoped lint/types, ADR lint and docs publication.
Independent bounded review accepted examined mechanisms. Acquisition crash receipts and sampled
import without meaningful symbols remain protected. Installed-service executable transfer
**passed**,2026-10-10, through the native owner's checked stabilization route; maintenance is released.
SM8 service-survival acceptance and coordinated matching-source `just qualify`/assembled review
remain **not_run**. Scheduling stays off until that acceptance. UP9/BC3/operator acceptance retains its owners.

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
Qualify BC3 separately before installing the ordinary test profile. Record source-specific closure; retain failed composites.
The operator restored40GiB free after the filesystem fell below1GiB; focused validation resumed.
No measured speed or whole-plan completion is claimed.

**Preservation and boundaries:** preserve operator hashing references, PC3's dirty isolated
planner-control copy, shared Cargo intermediates and compiler/profiling/benchmark captures.
No operator databases/configuration/client registration, protected gold/heldout, Qwen, real
FastMCP/Q1 pilot, wheels, activation or push. Commands use normal available compiler/test
parallelism and unchanged stacks/timeouts; native checks borrow the installed validation database with logical isolation.
Maintenance follows [AGENTS.md](AGENTS.md#commands).
