# Status

_Updated 2026-10-09 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: native-execution efficiency plan authored; production implementation/testing paused.**
The [compiler/kernel plan](docs/plans/graph-compilation-kernels-and-hashing-plan_2026-10-09.md),
[reuse companion](docs/plans/graph-compilation-reuse-and-invalidation-plan_2026-10-09.md#7-implemented-contract-refinements-2026-10-09)
and [persisted coordinator §8/§9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07)
own implementation scope, scheduled finding disposition and exact failed/repaired receipts.
The [native-efficiency plan](docs/plans/native-execution-efficiency-plan_2026-10-09.md) develops
NE0–NE9; source-review F01–F08 now have their sole scheduled disposition in coordinator §8.
Operator RC01–RC05 acceptance is recorded individually; production decisions/corrections remain Proposed.
The [independent target review](docs/design_review/reviews/design_review_native-execution-efficiency-plan_2026-10-09.md)
accepts the Proposed / Interface-checked target, without new blocking findings or implementation closure.
GK0–GK6 and GR0–GR5 are committed in `2a9a3771`;
prior GK0–GK5 foundations are committed as `da7c4747`. GK7/GR6 acceptance remains open.
Preserve the operator's hashing references. No push or operator activation requested.

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

**Implemented / focused Tested or source-inspected, 2026-10-09:** ADR-0140 integrates selective
portable products with model-owned scope programs. Requests carry exact role/order/prefix,
model/code/configuration/profile and canonical output contracts; BLAKE3 durable identity and
XXH3 exact-equality interning retain distinct responsibilities. Complete selected membership,
content and missing/empty domains precede rich hydration. The graph records actual selected
frozen epochs and computational/provenance edges; it is not a general incremental scheduler.

Replay checks typed canonical rows and necessary current semantic predicates before ingress,
then creates fresh contributions and checked owners. Portable products carry no private hints.
Behavioral Local/Base/Body/SourceCall remain Fresh because complete replay validation repeats
their kernels; Catalog NotRequested rows are eligible. SCC schedules compute once per prepared
graph and bind through a private materialization identity; persisted SCC replay is retired because
its validator repeats traversal and condensation ordering. Canonical topology identities remain.

The optional native private cache has complete keys/chunks, acknowledged canonical readback,
bounded fixed lock stripes, separate quota, entry/lifecycle leases and unknown-effect quarantine.
Persistent directory ownership, point-fenced atomic schema reset and a shared administrative
transaction token prevent stale administration after newer install. Explicit installation alone
reconciles interrupted administration; ordinary connection neither installs nor repairs state.
No completed-artifact or published physical-schema migration is introduced by cache metadata.

Viewer-owned Moka preparation coalesces exact-pin requests within one retention generation, with independently cancellable
waiters and charged insertion/value owners. Pressure replaces and drops optional cache retention;
external borrowers retain charge and reader pin. NE7 schedules stable flights across pressure replacement. Close drains preparation before reader
invalidation. Existing native serving and Python session consumers use this lifecycle.
The direct pinned Moka0.12.16 adds only its narrow lock closure; no existing version changes.

**Current verification, 2026-10-09:**

- **passed:** affected release compile checks; repaired pure/domain/canonical/replay/serving
  controls have aggregate passing evidence. Original failed composites remain failed.
- **passed:** final13 actual native cache controls, run `20261009T151353.336Z-db7117`, including
  late admin fencing, unknown effects, stripe collisions and committed-but-ack-lost recovery.
  Three exact dependency-graph controls passed; readonly independent review accepts the bounded
  final native protocol and graph-bound SCC consumer at source-inspected strength.
- **passed:** final SCC-refined candidate25 model controls, run `20261009T154435.789Z-dc2a91`.
  Graph moves, identical/changed rematerialization refusal and separate charge release covered.
- **passed:** typed entity demand and SCC controls, six finite cases plus core root refusal,
  run `20261009T160744.103Z-5fd0d5`. Both native profiles subsequently completed normalization.
- **failed / corrected source, runtime unqualified:** both normalized artifacts refused callable admission:
  supporting callables' advertised claims were mixed into the requested owner. Model-owned
  advertised-owner selection is implemented; release compile check passed in
  `20261009T163428.407Z-d6a8d2`. Both new finite/adversarial controls passed in
  `20261009T163757.029Z-9b0811`; both native normalized artifacts completed normalization,
  content freeze and facts admission, then timed out during invariant admission at900s.
  No new assertion or artifact pass was established. Earlier candidate normalization
  conflicts/timeouts and terminated Behavioral Facts retain their original failed boundary.
- **failed:** earlier six-attempt Catalog and Behavioral compiler reuse matrices exceeded300s;
  the retained logs do not establish the failing phase. Attempt/compiler phase diagnostics now
  improve the next run. No full cache-off/cold/hit/reload/changed-input equivalence claim.
- **passed:** verification routing45 Python controls; generated CLI feature union and ADR index.
  Final-source guard-free `just ready` passed in `20261009T165830.221Z-835b15`,7m04s,
  rebuilding the native extension after SCC/entity/callable ownership corrections.
- **failed / canceled partial:** assembled release `just qualify`, `20261009T170649.056Z-032c30`,
  canceled at17:27:50UTC after21m01s, exit143. Model798, analytics14 and flow controls passed;
  extract retained one authentication failure and21timeouts; core retained147passes and two
  300s matrix timeouts in their first cache-off attempts. SIGTERM cases are cancellations.
  Remaining families/leaves were not_run; no assembled pass or whole-plan completion.
- **Implemented / not_run:** stable SDK-session streaming correction retains the authenticated
  client through explicit drainage. Its new native control and final-source compile check have
  not run. The previous readiness receipt predates this patch; authentication causality is unproven.
- **passed:** plan authoring `just docs-check`, 2026-10-09,360canonical pages, zero link errors;
  `git diff --check`. The earlier ADR-0140/§B3 reference mismatch and two new broken links were
  repaired, then the actual publication check rerun. This is documentation acceptance only.
- **passed:** preservation check:73 pre-existing unowned files remain byte-for-byte unchanged.
  Production compile/native controls are **not_run** for this documentation-only task.

**Next:** plan detailed execution from NE0/NE1 and their actual query/transport/lifetime prerequisite slices. No tests or
production work resume through authoring. Retain failed boundaries and release default until BC3
passes. Structural corrections are Proposed; latency attribution and improvement remain unmeasured.
Source code, focused controls, candidate profile qualification and whole-plan acceptance remain
separate claims; no measured speed or whole-plan completion is claimed.

**Preservation and boundaries:** retain PC3's dirty isolated planner-control copy, shared build
intermediates and compiler/profiling/benchmark captures. Prior coordinator receipts include
19core300s/six-upper900s/large compiled-export300s timeouts; preserve their original source scope.
PC6/CU6/BC3/BC5 acceptance is mapped to GK7/GR6. Operator databases/configuration, client
registration, Qwen, protected gold/heldout, real FastMCP/Q1 pilot, wheels, benchmark campaign,
activation and push remain held. Maintenance follows [AGENTS.md](AGENTS.md#commands).
