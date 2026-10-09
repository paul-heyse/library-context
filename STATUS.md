# Status

_Updated 2026-10-09 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Storage lifecycle plan authored, 2026-10-09:** the [plan §9](docs/plans/storage-lifecycle-management-plan_2026-10-09.md#9-finding-disposition-owner)
owns scheduled storage F01–F03; the [independent target review](docs/design_review/reviews/design_review_storage-lifecycle-plan_2026-10-09.md)
accepts the corrected **Proposed / scoped** design. Operator-confirmed choices preserve warm
caches, detailed profiling and performance settings, and select automatic rules-based lifecycle
management across repository and shared host assets. Target-review F01/F02 are resolved in the
Proposed target; operational closure remains open. AF1 retains cleanup/recovery ownership;
BC3 and current evidence remain protected. Next: SM0 decisions/SM1 observation and the working
AF1 prerequisite, then producer migration, legacy adoption and qualified automation.
Cleanup, archival, configuration/agent-policy implementation and product controls **not_run**;
this task authors the plan only. Publication **passed:** `just docs`,369pages, zero link errors;
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
Remaining current NE implementation is uncommitted.

The SDK3.3.0 now has a narrow vendored gRPC physical-terminal backport. Successful application
End retains the transport through checked EOF and late errors. SDK3.3.2 retains the same gap.
PSE-arrow's patched WebSocket buffers query results; progressive server `query_stream` would
need an adapter with equivalent cancellation and backup support. PSE-arrow is unchanged.
No dependency version or public artifact schema changed; [pins](docs/pins.md) owns the SDK hold.

**Agent-effectiveness plan authored, 2026-10-09:** the [follow-up plan §8](docs/plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner)
owns scheduled F01–F05, selected capabilities/help and transferred AE-24; earlier receipts and
other obligations retain their [workspace-plan owner](docs/plans/agent-workspace-effectiveness-plan_2026-10-07.md#9-finding-dispositions).
The [independent target review](docs/design_review/reviews/design_review_agent-effectiveness-plan_2026-10-09.md)
accepts the corrected **Proposed / scoped** target. RC01–RC04 are individually operator-confirmed;
AF0 decisions and AF1 lifecycle corrections are next, with AF5/AF8 independently ready.
The [source review](docs/design_review/reviews/design_review_agent-effectiveness-followup_2026-10-09.md)
still judges implemented behavior **Revise**. Remediation, configuration and product controls **not_run**;
this task authors the plan only. Publication **passed**, 2026-10-09: `just docs-check`,
364 canonical pages, zero link errors; `git diff --check` passed. Product qualification is separate.

**Current verification, 2026-10-09:**

- **passed:** affected release compile check over core/extract/native/serving tests.
- **failed composite / repaired boundaries passed:** focused run `20261009T202627.015Z-6acf27`,
  66controls:65passed,1native transport failure. Admission/binding cleanup, typed products,
  bridge wakeups, serving flights and Selected/Complete observation passed. Earlier generated
  alias/casing and inherited primary-error/drainage assertions were corrected and rerun.
- **passed:** patched native run `20261009T204438.092Z-2dfd57`,16controls in6.534s after5m46s
  release build. Includes the large exact-key/atomic-field case that failed repeatedly in isolation,
  actual scalar-index plan, authenticated shared session, scoped completion and final drainage.
  This supports the correction at that boundary, not every historical timeout's attribution.
- **blocked:** SDK helper-unit invocation `20261009T204248.635Z-5d1c56`; Cargo refuses a
  non-workspace dependency's dev-dependency tests. No helper-unit pass is claimed.
- **passed after repair:** explicit extraction determinism, `20261009T205258.138Z-fd8ce2`,
 127.744s. Two acquisitions of identical input now share only exact source authority; differing
 context/configuration still conflict and both provenance records remain. Original failed run
 `20261009T205101.852Z-74e7dd` retains its boundary. Independent source review accepts the fix.
- **passed on preceding source:** guard-free `just ready`, `20261009T205626.201Z-cab667`, native
 extension rebuilt in1m11s. Subsequent Rust corrections require another guard-free refresh.
- **failed / canceled partial:** assembled release qualification, `20261009T205753.562Z-7c4894`,
 stopped at2.9GiB free. Model/analytics/flow passed; extraction68passed/1failed. Core retained
 163passes,7assertion failures,17timeouts and32cancellations;260not_run. Corrections for export
 coverage, retained-owner assertions, fresh detached importers, compute/transfer separation,
 failed-preparation eviction and read-only capture are implemented; focused reruns are pending.
- **failed composite / repaired boundaries passed:** focused correction run
  `20261009T212411.991Z-b4297a`:8passed/2failed. Read-only capture identity/refusals, preparation
  recovery, coverage, hydration, physical partition planning, transfer7/128MiB equality and
  fresh-importer export controls passed. Statistics test runtime setup is corrected; detached
  controls now separate native graph/state mismatch from coherent unsupported-support semantics.
  Follow-up `20261009T213536.960Z-54edcb`: statistics and both distinct detached controls passed;
  both cache matrices and all three analytics controls timed out at unchanged300s, no assertion
  failures. Full equivalence and analytics acceptance remain open.
- **passed:** guard-free native refresh `20261009T214454.980Z-f24fba`,18s.
- **running:** resumed release qualification `20261009T214543.898Z-81f4a8`, only previously
  failed/interrupted/unrun boundaries, through `just verify --rerun 20261009T205753.562Z-7c4894`.
  Extraction69controls passed; full compiler and remaining boundaries are in progress.
- **failed / canceled partial, historical:** prior assembled release `just qualify`,
  `20261009T170649.056Z-032c30`: model798/analytics14/flow passed; extract authentication/timeouts
  and core cache-off matrix timeouts retained. Remaining boundaries not_run. Both earlier native
  normalized artifacts reached invariant admission then timed out at900s. Exact source boundaries
  remain in coordinator §9.1; current focus does not establish their repaired integrated pass.
- **not_run on final source:** BC3 candidate qualification and remaining assembled boundaries
 and leaves. Release remains default.

**Next:** finish resumed qualification and resolve source-specific failures without weakening controls.
Qualify BC3 separately before
installing the ordinary test profile. Record source-specific closure; retain failed composites.
The operator restored40GiB free after the filesystem fell below1GiB; focused validation resumed.
No measured speed or whole-plan completion is claimed.

**Preservation and boundaries:** preserve operator hashing references, PC3's dirty isolated
planner-control copy, shared Cargo intermediates and compiler/profiling/benchmark captures.
No operator databases/configuration/client registration, protected gold/heldout, Qwen, real
FastMCP/Q1 pilot, wheels, activation or push. Commands use normal available compiler/test
parallelism and unchanged stacks/timeouts; native checks use owned disposable fixtures.
Maintenance follows [AGENTS.md](AGENTS.md#commands).
