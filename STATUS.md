# Status

_Updated 2026-10-10 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: SurrealDB architecture plan authored,2026-10-10; shared main is dirty.**
The [architecture companion](docs/plans/surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md)
develops SA0–SA9 for all seven source findings and nine capability avenues, integrated with HS/UP/NE.
The source review remains **Revise**; [independent target assessment](docs/design_review/reviews/design_review_surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md) accepts the amended **Proposed**
contracts after correcting repeated-preflight recovery. Review RC01 and supplemental SA-RC04 were
operator-accepted2026-10-10; transport/coordination retain their existing conditional decisions.
Coordinator §8 owns SA/HS dispositions; §9.1 owns document and runtime receipts. Native/product acceptance stays open.
Publication verification: `just docs-check` **passed**,2026-10-10,385 pages/zero link errors, including the independent review.
Next: SA0's decision/owner/runbook changes and SA1–SA3's complete recoverable migration; independently ready access work may proceed.
Authoring performed no migration restart, service change, production remediation or product tests.

**Holistic state-management HS0–HS11 execution is interrupted.**
The [holistic companion](docs/plans/holistic-state-management-plan_2026-10-10.md) owns this scope.
[ADR-0145](docs/adr/0145-holistic-state-ownership.md) supersedes ADR-0143 with selected coherent
logical backup and fenced history disposal. Coordinator [§8/§9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns scheduled dispositions and actual receipts. HS-F09 is focused-Tested/closed; other HS findings remain Open.
Implemented source includes model-owned recovery closure, shared audit capture/owned finalizers,
bounded selection, transaction-bound backup/independent cold admission, phased retirement, original
issuance eras/outcome references/history collection, compiler release and charged ranked retention.
Writer fencing, recovery lineage, physical-terminal classification, search scratch and terminal-attempt
cleanup corrections are source-accepted. The earlier assembled review's sparse first-request, family
dispatch and budget repairs are source-accepted; functional qualification remains pending.
**passed**,2026-10-10, historical receipts:99 host/storage and5 model controls (`162158…fe134a`);
repaired all-test check (`163414…857f59`); repaired installer (`163332…1b9615`). Main migrated,
but validation twice crashed the original server on transaction-timeout stack cancellation.
Concurrent-index installation and exact same-target/original-native-ID recovery are implemented;
114 mocked recovery controls passed. Separately owned exact3.3.0 server correction preserves runtime limits.
Actual-size export admission/shared-pool pressure corrections passed67 pure controls (`171523…9388db`);
all-test check (`170340…db2755`) passed. The locked DiskANN build defect received an exact local correction.
Patched server build/adoption passed;185 composed tooling controls passed. Page-local origin reuse
passed its three affected pure controls; final affected binaries compiled/listed, and guarded native sync passed.
**failed**,2026-10-10: `20261010T174350.233Z-05d48c` validation translation exited1/cleaned at19:02:54UTC:
`native_retirement.root_count` expected `int`, found `NONE`. Main's format4 checkpoint remains;
validation stays format3/admission closed. Queued `181816…d35925` and `182220…e52348` failed
prerequisite waits; their native test bodies are **not_run**. The password-fence observer still waits for format4.
Python/product acceptance, native timeout regression and HS11 assembled qualification are **not_run**.
Next: execute architecture SA0–SA3 before correcting/resuming the immutable migration;
then source-matching revealing controls and scheduled HS11 acceptance. Earlier source acceptance is not runtime closure.

**Unified persistent SurrealDB UP0–UP9 / NE / PC / GK / GR integrated acceptance remains open.**
The [unified companion](docs/plans/unified-persistent-surrealdb-plan_2026-10-09.md) owns surviving
packages. Exact immutable payloads/views, scoped attempts, retained attachment, manifest publication,
executable epochs, reader pins and one stable service remain the foundation; patched gRPC remains initial.
Prior focused passes and failed composites retain their dated scope in coordinator §9.1. Earlier
publication selection had11 passes/1 failure/4 timeouts/1 ignored; MCP native journeys had2 failures/
1 ignored, including absent-library ResourceRefused and reconciliation array amplification. Their
replacement controls and Python follow-up must pass on current source. Earlier accepted source reviews
and readiness passes do not promote these incomplete runtime results. Operator/PSE-arrow data,
selections, real-library adoption, shared caches and recovery evidence stay protected.

**Storage lifecycle implementation in progress, 2026-10-09:**
[ADR-0144](docs/adr/0144-managed-storage-lifetimes.md) governs warm-cache preservation and
owner-backed retirement; [plan §9/§10](docs/plans/storage-lifecycle-management-plan_2026-10-09.md#10-current-checkpoint)
owns findings and actual acceptance. Policy, lifecycle/admission/journals, producer integration,
repository checkout bindings and archive/restore mechanisms are implemented with focused
controls. The bootstrap uses pinned Python3.14.7 without the project venv; no3.12 helper.
Only this repository and its two checkouts are registered; eight legacy roots and five retained captures have
protective descriptors. Central skill stores and other repositories are excluded.
Backlog cleanup retired five obsolete captures (about98GiB raw samples), preserving/hash-verifying reports and receipts. Verification passed; automation remains off.
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
No measured speed or whole-plan completion is claimed.

**Preservation and boundaries:** preserve operator hashing references, PC3's dirty isolated
planner-control copy, shared Cargo intermediates and compiler/profiling/benchmark captures.
No operator databases/configuration/client registration, protected gold/heldout, Qwen, real
FastMCP/Q1 pilot, wheels, activation or push. Commands use normal available compiler/test
parallelism and unchanged stacks/timeouts; native checks borrow the installed validation database with logical isolation.
Maintenance follows [AGENTS.md](AGENTS.md#commands).
