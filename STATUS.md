# Status

_Updated 2026-10-10 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: SurrealDB architecture SA0–SA9 execution,2026-10-10; shared main is dirty.**
The [architecture companion](docs/plans/surrealdb-architecture-and-capability-leverage-plan_2026-10-10.md)
owns scope; [coordinator §8/§9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns dispositions and receipts. [ADR-0146](docs/adr/0146-recoverable-native-upgrade-progress.md)
records accepted recovery progress and explicit format4→5 history-receipt evolution.
Implemented source: complete bounded native recovery/provenance verification, separate atomic pass/page
progress, immutable host reconciliation successor, operation-local catalog preparation, selective
membership/FULLTEXT nomination, batched occurrence hydration and retirement registration.
Selected-relation nomination passed12 focused model/native controls and actual indexed sparse access; enclosing acceptance stays open.
Earlier scoped reviews retain their scope. Format5 history refuses old schemas before durable intent; private atomic rollback,
controlled acknowledgement-loss and changed-parent controls are source-accepted and qualified on current page code.
Enclosing acceptance remains open. **passed**,2026-10-10: integrated release all-test check (`205122…8bce5e`), format4 CLI build
(`205515…0a28b6`, before final native verifier edits),156 host controls (`210205…d4556f`),
and167 final host controls (`213908…523cb7`), followed by4 retry/legacy controls (`214033…0574dc`).
Matching-source31 format4 pure controls passed (`210857…702d00`); installer2201b8d7 is frozen.
Host executable-epoch sequencing is source-accepted. Restored candidate4 preserves exact target/protocol
and acknowledged prefixes:11 controls (`214956…98f48c`), normal CLI
build (`220003…793541`) and exact CLI/contract comparison (`220859…53038c`) **passed**.
Retained reproducible source bundle `220233…7befdb/source` has its named replay hold. Actual nonempty
observation/changed-body/missing-record/rollback/ack-reconciliation qualification **passed**
(`220921…907c37`), preserving original native/credential identities and main publication.
Checked successor `aab8e187…` resumed from revision3727; `221005…875935` then failed at revision5120
(preflight/native_guard) on10s transaction timeout during severe host memory/swap pressure. Same-candidate retry
`223906…f13581` was gracefully cancelled (exit143), revision12211/preflight/native_hold retained.
Range qualification `232349…c2a264` passed exact128-row/body/order equality. Format5 finite controls passed
before later assurance additions.39 finite controls (`224700…53f78f`),4 CLI controls (`224625…1d0bc0`),
804 model controls (`225757…9af32f`),14 analytics and43 flow passed after stale finite fixtures were corrected.
The ty oracle awaits its fixture. Main5 run (`233609…85699f`) passed2 pure controls, CLI build8m27s and native discovery.
Exact CLI identity/source capture passed; native rerun (`002810…4b66fd`)7passed/1failed on sparse-reader immutable address collision.
Repaired run/service fixture isolation passed57 lifecycle controls (`215936…26def5`), corrected
formatter fixtures passed48 (`220402…21aca4`), and full tooling rerun passed (`220453…f6a787`).
Current HTTP/service fixture controls passed203+5, final HTTP5 (`231916…a4df0a`); SM8 pure controls passed16.
The latest full tooling rerun passed (`231238…1fac84`); original format4 recovery now passed separately.
Corrected4 composite (`235000…320a5c`) passed2 pure controls/8m38s CLI build; contracts/source capture (`000334…8d8bec`) passed.
Actual Protect rollback/ack/stale qualification (`000519…b5cc8e`) and full4 recovery (`000538…638d38`, exit0/cleanup confirmed) passed.
After SA assurance, operator authorized **all remaining repository testing**,2026-10-10, with failures
corrected/rerun toward all passing; BC3/SM8 retain separately attributed outcomes and activation boundaries.

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
Earlier focused passes, source reviews and failed composites retain exact meaning in coordinator §9.1.
Installed scopes are exact format5/0dcc; source4 refusal and owner readbacks passed. Nonce JWT rotation (`002712…3b030f`) passed; original old-JWT gap remains.
Sparse/serving2/ty2 passed (`004041…3dbf50`); publisher cold/catalog/history parse/current CLI passed (`005233…7b60ee`).
HS6 inventory/five nonce controls passed; assembled run cancelled after provider/core failures. New completion/sort corrections await runtime; disk exhausted, builds blocked.

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
Lifecycle inventory (`234536…65cb09`) passed:354 objects/0 actions; no retirement authorized. Acquisition crash receipts and sampled
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

**Agent effectiveness AF0–AF9 implemented / focused-Tested,2026-10-09:**
[Follow-up plan §8/§9](docs/plans/agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner)
owns receipts and findings; ADR-0142 governs owned readiness, independent cleanup and detachable display.
Independent integrated review accepted examined scope. Focused tooling, publisher/resolver, lint/types,
ADR and documentation checks passed; comprehensive product acceptance remains separately open.

**Existing qualification follow-up:** source-specific failures require correction and affected reruns.
Coordinator §9.1 retains failures/timeouts and source-accepted corrections; resume focused completion/sort/provider controls after capacity is freed.
Qualify BC3 separately before installing the ordinary test profile; SM8 service survival remains separate.
No measured speed or whole-plan completion is claimed.

**Preservation and boundaries:** preserve operator hashing references, PC3's dirty isolated
planner-control copy, shared Cargo intermediates and compiler/profiling/benchmark captures.
No operator databases/configuration/client registration, protected gold/heldout, Qwen, real
FastMCP/Q1 pilot, wheels, activation or push. Commands use normal available compiler/test
parallelism and unchanged stacks/timeouts; native checks borrow the installed validation database with logical isolation.
Maintenance follows [AGENTS.md](AGENTS.md#commands).
