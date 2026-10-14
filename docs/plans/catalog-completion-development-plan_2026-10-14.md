# Catalog completion: development execution and operations

**Proposed implementation, 2026-10-14; authoring baseline `f330bf1d972ddab4962ffd7d9b1d4bd92b9d71b2`.**
This companion develops T01–T04 for the [Catalog completion coordinator](catalog-completion-plan_2026-10-14.md).
Its [§6 checkpoint](catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint)
alone owns current disposition, execution state and receipts. This document specifies the target,
preserved platform decisions and future controls; it is not another progress ledger.

The basis is the [implementation-friction review](../design_review/reviews/design_review_implementation-friction-target_2026-10-14.md),
particularly F09/F10, O01–O05 and RC01/02/05, with its [retained evidence](../design_review/evidence/2026-10-14_implementation-friction/README.md).
Core/template3.3, the efficiency heuristics1.0 and applicable code-intelligence1.5 constraints govern
the design. [DESIGN §1.2](../design/DESIGN.md), [workspace storage](../workspace-storage.md),
[storage/publication](../design/sections/storage-and-publication.md) and their decisions remain
architectural owners. The predecessor operations companion's OP/LO/LD obligations and C01–C11
decisions are carried below. Plan publication implements no host, configuration, schema or service change.

## 1. Purpose, baseline and preserved boundaries

Give one operator using Codex a reliable route from an exact command to its actual execution,
selected evidence and settled ownership. The workload includes concurrent builds and source edits,
commands that delegate to fixture supervisors, long native controls, retained historical runs and
repeated small inspections. Normal available command/test parallelism remains. This scope adds no
universal supervisor, scheduler, worker/job/thread cap, healthy-command timeout or cache purge.

Existing owners provide sound foundations:

| Owner | Reused contract and required extension |
|---|---|
| [Harness](../../scripts/harness.py) | Kernel process identity, directly owned groups, unreaped-leader pinning and conservative historical observation; qualify delegated containment without numeric-PID guessing |
| [Managed runs](../../scripts/runs.py) | One-writer receipts, direct capture and selected cancellation; compose delegated host obligations and make discovery selective |
| [Fixture](../../scripts/surrealdb_fixture.py) | Attachment-owned command group and distinct native-completion observation; expose the exact delegation to its enclosing run |
| [Service](../../scripts/surrealdb_service.py) | Actual installation/generation, maintenance and executor/disposal authority; persistent native service lifetime never becomes a disposable run child |
| [Verification evidence](../../scripts/verify_evidence/capture.py) | Optional native capture adapters and independent command/capture/finalization observations; reuse for an exact ad hoc command |
| [Development worker](../../scripts/surrealdb_development.py) | Exact owned campaign/worker provenance and generated beta assets; adapt only changed seams |

Historical cancellation proves the missing parent/delegated cleanup composition: outer run cleanup
was Confirmed while the fixture compiler group survived. It does not establish the exact repeated
signal, guard-interruption or grace-period cause. The direct-group receipt is scoped truth. Local
recovery's refusal after losing authenticated group identity and its separate native-Unknown refusal
are correct. Whole-target disposal settles only its authenticated target, preserving original outcomes.

Source observations, post-command executable hashes and configured models are similarly scoped
observations. They do not prove executed-source closure or an existing thread's model. No stronger
provenance or effectiveness claim follows from this plan. Improvements remain Proposed until the
controls below pass; installed systemd255 containment is Interface-checked only.

## 2. Approved rule changes and execution route

The operator explicitly approved these changes on **2026-10-14**. Record their enduring decisions
and update their actual consumers before dependent implementation relies on the new rule. This
authoring turn changes no ADR decision or actual configuration; the coordinator records the
operator-authorized navigation-only exception for accepted ADR-0165–0169.

| Source item | Approved target | Route before dependent execution |
|---|---|---|
| friction#RC01 | Compose delegated host cleanup with run/capture ownership; retain direct-group, delegated-host and native-completion axes | T01 records a successor/complementary ADR to ADR-0142 and updates DESIGN §1.2 plus storage/run/fixture guidance. Accepted ADR text is immutable; explicitly supersede a changed decision rather than silently amend it |
| friction#RC02 | Compact bounded default run discovery, explicit full history and selective access | T02 migrates parser/help, README and agent command guidance alongside implementation. Preserve detailed selected status and make changed list completeness explicit |
| friction#RC05 | One operator-selected Codex model inherited by roles; retain functional responsibilities and independent judgment | T04 records the scoped change to ADR-0109 routing and DESIGN/role guidance, then removes competing project/default/role model pins from their actual owners. Qualify a fresh session before claiming effective inheritance |

Existing native finality, authenticated recovery, exact publication pins and storage retention are
preservation constraints. No rule change is needed to retain those safeguards or to expose an
optional evidence adapter. A newly discovered material rule impact goes to the coordinator before
dependent execution; it is not inferred approved from these three decisions.

## 3. Target packages and dependencies

| Package | Delivered behavior | Actual prerequisite and consumer boundary |
|---|---|---|
| [T01](#t01) | Delegated host cleanup, capture-writer lifetime and selected containment | Approved RC01 route and a bounded mechanism decision; migrate run/fixture/storage consumers together before reporting aggregate cleanup |
| [T02](#t02) | Compact bounded run discovery with explicit completeness | Approved RC02; independent of T01 for ordinary metadata selection. Aggregate delegated cleanup fields consume working T01, never inferred substitutes |
| [T03](#t03) | Optional exact-command evidence and useful owner-correlated diagnostics; affected generated/beta seam preservation | Existing capture adapters are usable independently. Capture release over delegated commands consumes working T01; production phase/resource changes migrate their real writers and readers together |
| [T04](#t04) | Single-model configuration, concise current checkpoint and correct command/agent routes | Approved RC05 route; fresh-session qualification after configuration migration. Model choice does not depend on T01–T03 or authorize changes to shared library skills/other repositories |

Logical independence is not permission for conflicting edits to `runs.py`, fixture or guidance.
The coordinator assigns shared-file ownership. Early contracts may enable consumer work before a
whole package closes; disposition and the outstanding enclosing obligations stay at coordinator §6.

<a id="t01"></a>
### T01 — compose delegated host ownership and settle containment

Before a fixture command becomes usable, bind its existing command owner to the exact enclosing
managed run. The binding identifies the run, attachment/command lifetime and authenticated owner;
it is not a copied mutable process fact. The child remains its receipt's writer. Parent cancellation
requests that owner to stop, observes its settled result, and composes all registered delegated
host commands before releasing run/capture obligations. Hold storage admission through all writers'
cleanup. Report direct-group cleanup, delegated-host cleanup and native completion separately;
neither child exit nor an empty group/cgroup proves a native request terminal.

Unknown/failed delegated cleanup protects capture and remains explicitly recoverable. Recovery
revalidates exact boot/namespace/start identity and owner lock; disappearing leaders, reused PIDs,
foreign namespaces and malformed bindings cannot authorize blind signals. Preserve actual child
exit, supervisor/launch errors, original cancellation and later recovery as separate observations.
Do not enroll persistent native services or shared cache daemons as disposable command descendants.
Direct `just fixture` invocation still needs its own honest supervision contract outside a managed run.

**Bounded mechanism decision.** Compare a small explicit delegated-owner reference/request protocol
using existing owners against an installed-systemd255 transient user-service adapter. Local255.4
manuals and version255 source qualify `ExitType=cgroup` and `KillMode=control-group` at
Interface-checked strength: a new process session need not escape its execution cgroup. That is not
proof of the proposed integration. Do not transfer Context7's newer version261.3 as the host contract.

The explicit-owner route is the baseline fallback because it preserves current supervision and
keeps service/native meaning local. A transient adapter is selected only if an inert, wholly owned
process exercise demonstrates simpler complete containment while preserving exact exits,
environment, durable file capture, direct fixture behavior, authenticated recovery and storage
release. Exercise live leader, leader exit with live descendants, nested session creation, repeated
TERM and owner/publication failure. Include a separately owned daemon witness that must survive;
a newly started shared cache daemon must not accidentally acquire the run's disposable lifetime.
Compare the full launch/cancel/recover route and resulting receipts, not only successful termination.

Register the probe's run/output owner and named consumer before launch; use authenticated owned
cleanup and no database. Record the question, observed result, selected mechanism and fallback at
the coordinator, with ADR/owner rationale before production integration. If the adapter cannot
separate shared lifetimes, preserve results and use explicit-owner composition. An unavailable
user manager or unsettled identity is an explicit unsupported/blocked condition, never an unnoticed
different guarantee. Do not build a general mechanism-selection framework for this choice.

**Migration and future controls.** Migrate `runs` status/cancel/protection, fixture launch/recovery,
storage completion/prune and inherited evidence writers in the same contract slice. Do not infer
new delegated bindings for historical receipts; preserve their scoped observations and exact recovery
routes without a bulk rewrite or old-format runtime service. Revealing controls must demonstrate:

- cancellation cannot complete aggregate host cleanup or start capture expiry while an authenticated
  delegated writer lives, including after leader exit and repeated TERM;
- launch/identity/receipt failure still leaves a protected obligation or confirmed cleanup;
- reused/foreign identities are refused; direct and aggregate observations remain distinguishable;
- a clean host stop with native Unknown still retains native protection, while authenticated
  whole-target disposal preserves original failed/interrupted records.

The process-only mechanism decision precedes any necessary focused real fixture integration.
Use existing installed logical validation or an explicitly authorized wholly owned campaign for the
actual fixture case; do not create a scratch fallback or touch operator/shared targets. Longer grace
alone cannot close friction#F09. Parent-owned capture finalization is part of its closure.

<a id="t02"></a>
### T02 — select run summaries before live hydration

Make ordinary `runs list` a compact bounded recent page. Its records expose identity, label,
command/state summary, relevant cleanup uncertainty and continuation/completeness; selected
`status ID` hydrates the supported detailed record. Provide explicit full history and active/unresolved
and label selection. Apply cheap directory/metadata predicates before live process observation where
possible. Reuse one request-scoped process snapshot for fields sharing that premise instead of
repeated per-record `/proc` scans. Keep process observations explicitly time-specific.

An active/unresolved query may require examining all owner metadata. State examined scope honestly;
bounded output is not bounded work. Corrupt/missing/unknown receipts remain discoverable through
the selection's declared semantics. Continuations bind their ordering/filter/read-consistency
premises and report change/invalidation rather than silently omit records. A directory iterator,
summary projection and request snapshot suffice initially; introduce no second persistent run store.

Future filesystem-backed controls use a large inert history and an independent access witness:
recent-count discovery does not hydrate unrelated old runs; each selected observation is reused;
explicit full history remains available; labels, corrupt/unresolved records and continuation under
the declared consistency rule have no omissions or duplication. Detailed status retains current
fields and T01's axes when available. No timing ratio, shortened retention or history deletion is
needed to establish friction#F10's correction.

<a id="t03"></a>
### T03 — reuse exact-command capture and diagnose actual owner boundaries

Expose an opt-in exact-command capture route over existing `CaptureOwner` adapters for native
Nextest/pytest/Cargo evidence. Consume the caller's exact command/working context and actual fixture
binding; reuse current configuration composition, capture destinations, invocation IDs, selective
inspection and storage owners. Preserve native arguments, ordinary plugin/policy discovery and
command outcomes. Unsupported capture is explicit and cannot convert a native pass/failure into a
different command result. Command exit, capture health, postprocessing, fixture cleanup and native
completion remain separate. Bare tools and boundary-based `verify` remain first-class; do not add
another runner or require permanent family declarations for ad hoc controls.

Capture the invoked/available executable observations with their precise scope. A dirty-source
digest before/after execution and a later executable hash do not establish source-to-executed-byte
closure; preserve the existing uncertainty and intermediate-edit limits. If a consumer requires
stronger closure, scope a separate execution handoff using actual retained build inputs/selected
executable, rather than relabel ordinary observations.

At existing phase/progress/report seams, distinguish compilation/lock waiting from test execution
and carry invocation/producer/owner identities through async wrappers. Pair phase begin/terminal by
ID, not log adjacency. Report a known pending native batch, producer join or completion gate as that
observation; do not invent a closed wait cycle or require a universal wait graph. Preserve typed
request/used/limit and resource owner context. DataFusion55.1 `report_top` returns formatted text:
retain that existing bounded diagnostic once at the refusal boundary, alongside typed facts, without
advertising a structured consumer-vector API or confusing reservations with heap/RSS.

Migrate affected real native writer→signal→fixture/report readers together. Retain append-only
diagnostic codes, safe bounded class/stage/secondary cause, absent/opaque third-party identities and
honest truncation. Human text is not an effect protocol. Closed maintenance exposes unobserved
native state and the supported diagnostic route without reopening admission. Keep source/recipe/
binary/running generation, SDK/RPC patches, beta version/capabilities, schema and executable epochs
distinct; no SQL/credential leakage, copied validity registry or pervasive collector is needed.

Future controls cover one exact native command plus interruption, independently known case/phase
membership, failed capture with unchanged native result, unsupported capture, stable selective access
and capture lifetime through T01 drainage. Exercise a phase failure/timeout with independently paired
IDs and a resource refusal whose displayed owner/request/used/limit agrees with its typed source.
The newly added mutation-phase logging is Implemented, not already qualified by cancelled reruns.
Where production changes alter diagnostics/generated definitions/beta seams, retain the required
affected round trips and owned-target controls in §4 and the coordinator's acceptance; unchanged
passed infrastructure is not a new project to restart.

<a id="t04"></a>
### T04 — align model choice, current work and command use

Keep one operator-owned Codex model selection and inherit it into functional roles. Remove
competing project coordinator/default-subagent/custom-role model defaults only through the approved
RC05 route; retain shared role responsibility/effect contracts, selective delegation and independent
review. Retain role effort settings unless the selected model/runtime requires a deliberate supported
adjustment; do not hide another model selection inside effort guidance.
This repository-only migration does not rewrite shared library skills, other repositories or the
operator's broader host configuration without authorization. Configuration changes do not alter
already running agents.

Qualify effective inheritance in a fresh Codex session and a minimal role invocation, observing
actual thread metadata and precedence, not only TOML declarations. Preserve an explicit operator
model override and disclose unsupported runtime inheritance. The review's root and reviewers were
observed using the same model despite mixed repository defaults; do not describe that review as an
actual model mismatch or infer comparative model quality. No calibration campaign is required.

Use the current runtime's task activation operation for an idle agent; sending a message alone is
not activation. Inspect returned state and add the concise activation/message distinction to existing
role guidance.
Encrypted messages/counts do not establish the earlier exact delay. Add no repository scheduler.

Keep coordinator §6 concise: current source, actual active owners, open obligations, source-qualified
receipts and next executable action. Transfer surviving obligations before retiring superseded
narratives; their consumed causal evidence belongs in the review/evidence and Git. Batch coherent
source changes before rebuilding consumers; label superseded builds, build completion and assertion
execution separately. Cold compilation after `cargo clean` is expected, and ENOSPC grants no cache
purge authority.

Prefer existing owning help/preview at command use. Implement narrow actionable parser hints for
`service dev` → `surreal-dev` and `service status --json` → `service status`; do not add aliases or
another command registry. Service status already emits structured output; the unsupported spellings
were agent misuse. Carry run `30bb8e`'s existing14 ADR-reference defects/stale index to coordinator §6
as a documentation obligation, with affected publication or separately authorized documentation
repair as its revisit trigger. Repair publication defects introduced by this work; an unrelated
failed check keeps its actual outcome and owner rather than triggering automatic quality remediation.

Future checks inspect effective fresh-session model choice and role inheritance, explicit override,
minimal task activation versus message semantics, no-effect erroneous command hints, and the current
checkpoint's retained obligations. They establish configuration/workflow behavior, not product or
whole-agent effectiveness. Remove obsolete model-routing guidance alongside its replacement.

## 4. Preserved generated schema, beta integration and native containment

Exact **SurrealKit1.0.0-beta.1** remains development-only; product SDK/RPC remain the patched3.3.0
family. Preserve the exact macro pin, narrowed remote HTTP/gRPC/Rustls profile and
[vendored provenance](../../third_party/surrealkit/README.lctx.md). No product CLI dependency edge,
beta upgrade, embedded engine/JWKS/default feature closure or automatic dependency change follows.

Native/model definitions remain sole schema authority, currently schema8. Existing generation emits
ordered control/canonical/search/compiler/completion modules with content/directory tracking and
embedded bytes. Named Sync/default tester projections use the complete ordered requested-module
closure, refuse unsupported selectors, render lifecycle variables before beta hashing and leave
ordinary checkout state untouched. Required installed definitions compare against native authority,
normalizing only application modifiers and retaining function bodies, permissions, indexes, nested
SQL and lifecycle names. Beta bookkeeping/fixture definitions and syntax parsing do not establish
unrelated installed permission/function behavior. Mutable native templates and publication-owned
immutable executable epochs retain their owners; no introspection-derived second schema exists.

Direct Sync uses separately configured owned Any sessions; clones are distinct sessions and auth
invalidation is not transaction rollback. Beta's public tester creates private clients outside
repository injection: contain privileged/schema/destructive controls in an explicitly registered
wholly owned remote service with actual auth and server-side capability ceilings. Bind exact purpose,
endpoint/namespace/database/generation/credential-reference/effect and installer/worker provenance
before effects; validate the installed marker by actual readback. Retain managed local gRPC and fresh
`lifecycle_<32-lowercase-hex>` databases under `library_context`. Sync/declarative runs retain
fail_fast=false, ordinary parallelism, keep_db=true and JSON reports. Watch/seed/rollout/typegen,
embedded engine and selectable fail-fast remain unexposed; remote/TLS/API-request support is
demand-triggered. Beta cannot claim product patched progressive/cancellation finality.

Parent campaign ownership survives setup failure, absent reports and unexpected suite databases.
Authenticate/drain actual workers and submitted native owners before exact executor shutdown and
storage/credential/installer release. Journal bounded disposal-only logical effects before launch;
reconcile exact retries. Failed logical drain plus authenticated whole-executor discard may settle
that target's disposal while original failure/Unknown remains. Unknown local/native drainage retains
its obligation. Generic sweeps, local attachment release and cancellation are not native disposal.
ADR-0167's narrow dead-maintenance transfer retains current participant/executor authentication,
journaled exclusion and all live/foreign/malformed/specialized recovery/restore/SOURCE_JOIN refusals.
Do not require obsolete marker ancestry when current supported participants are authenticated and
quiescent; do enroll managed children before spawn and preserve exact current run/target bindings.

Token substitution journals original placeholders before effects and restores atomically only after
authenticated worker drainage and exact private path/campaign/generation/run/storage checks.
Restoration precedes credential-removal completion. No old project is adopted without its journal.

When a selected production change alters definitions, version deliberately and regenerate/migrate
these same assets plus parser/projection/installed-definition/index consumers in that package.
Otherwise preserve schema8 receipts at their original scope without pretending to rerun migration.
A narrow native application of the same asset on the same owned target is preferable to silently
dropping an unsupported beta transformation. Required affected acceptance covers real diagnostic
round trips, missing/opaque identities/truncation, failed command with committed effects, native
Unknown, permission refusal, generated Sync/readback/declarative reports, setup-before-capture failure,
commit-before-cancel, token-SIGTERM restoration and interrupted disposal. These are future affected
controls, not permission to inspect/adopt shared/operator targets or restart old campaigns.

## 5. Preserved C01–C11 decisions and reopening triggers

These complete-route choices retain their source-led2026-10-12 basis at `9a7ad768` and pinned3.3.0
[library evidence](../design_review/evidence/2026-10-12_surrealdb-lifecycle-target/library-contracts.md).
They are not fresh version qualification or an adoption backlog. The coordinator owns any reopened
decision and its evidence; a supported keep-current outcome closes only that decision, never an
unsatisfied mandatory finding. New library questions use Context7 first and exact pinned primary
source where the current index differs. Negative search is not absence evidence.

| ID | Preserved complete-route decision | Concrete trigger and required evidence before changing it |
|---|---|---|
| C01 — references/retention | Retain native holds/incarnations; REFERENCE does not replace issuance, exact incarnation, active exclusion, cleanup nomination and recoverable outcomes. A second reference representation does not simplify that remaining protocol | A named relation becomes simpler end-to-end with source-established common exclusion and owned create/delete/recreation controls. Settle exact selected-input ownership first; no CASCADE on shared evidence. Unqualified reference concurrency is not an engine-defect claim |
| C02 — events/changefeeds/live/history | Keep on-demand owner inspection; no canonical event queue, refresh subscription or database history retention. Events cannot acknowledge host cleanup; retry exhaustion, cursor/gap retention and transient live queries add distinct obligations | A named proactive-refresh or durable replay consumer cannot reasonably use polling. Specify deduplication/exhaustion, cursor gaps and reconciliation, then qualify that auxiliary route. No reliance on unqualified3.3.1 behavior/versioned SELECT |
| C03 — timers/sweeps | Keep explicit owner-directed sweeps; expiry nominates, current owners authorize effects. No timer activation | A concrete recurring unattended cleanup need. Retain current admission/owner/native/borrower rechecks, inspectable run outcomes and interrupted-effect recovery. Existing sweep races are not certified by this decision |
| C04 — GQL/relations/views | Retain indexed exact native selection and Rust graph kernels, with attributed direction/role, identity, multiplicity and selected membership; no new graph representation/front end. Counts/materializations add universe invalidation; inline adjacency's fixed-cap coherence is limited | A complete traversal/count removes material work while preserving selected universe, isolates, direction, attribution and terminal ownership. Qualify cap changes with drained writers or their race; support high-degree spill. No stage per semantic family |
| C05 — lexical/vector/hybrid | Keep the composed native lowering: frozen exact-view/family statistics, eligibility before caps, channel-then-family fusion and winning witnesses. Table-wide BM25, generic one-level RRF and approximate KNN are not equivalent substitutes | Exact equivalent lowering or explicitly selected approximation with independent eligibility/tie/witness known answers and actual encoder/projection/index readiness. Large demand may justify ANN/DiskANN; no historical numerical corpus or broader retrieval redesign prerequisite |
| C06 — API/HTTP/GraphQL | Keep Rust-owned operations and current transport. A DB-hosted API must still own selected pins/auth/cancellation/deployment; pinned DEFINE API changes table/field enforcement and HTTP body representation constraints remain | A complete operation becomes simpler with explicit endpoint permission/auth limits, response encoding, retained definitions/pins and checked terminal cleanup. Object behavior of `api::invoke` is not HTTP qualification; no general endpoint is enabled |
| C07 — Surrealism/WASM | Keep typed pure Rust kernels; no extraction relocation/module framework. Executable provenance, package/state lifecycle and experimental runtime deployment remain real costs; best-effort DDL warming is not invocation success | One bounded in-database computation removes transfer/machinery. Qualify executable trust/identity, module state, real invocation, cancellation and retirement under definition epochs |
| C08 — buckets/originals | Keep corrected native blobs/layouts for exact windows, digest/inventory, selected holds and coherent backup. Bucket metadata deletion/export does not settle object bytes, admission, backup or disposal | A concrete large-object workload exceeds the suitable native route or a complete bucket design is simpler. Establish object/record failures, immutable reads, selected byte inventory/backup/restore and independent disposal. No pointer-only replacement or universal atomicity claim |
| C09 — plans/schema/Studio | Retain positive adoption of on-demand native plan/index inspection and generated schema parsing; Studio/custom agent wiring stays optional. Parsing and EXPLAIN JSON are diagnostic, not installed behavior/stable public protocol | Required changed seams use exact authorized target/generation/version, least-privilege read templates, bounded capture and honest unsupported/not-inspected states. EXPLAIN ANALYZE executes and needs its own authorized workload. Interactive operator need reopens Studio/wiring; no assumed edition audit capability |
| C10 — MCP/telemetry/Cloud | Keep domain MCP and owned logs/receipts; no generic DB MCP, collector or Cloud deployment by default. Pinned stdio `surreal mcp` owns another datastore/session; HTTP MCP is a general DB surface, and metrics cannot prove native finality | A named question is unavailable from plans/receipts/logs or actual remote deployment is needed. Choose minimal instrument, explicit target/auth/capabilities, redaction, signal consumer and release route. No license rejection, accidental datastore or general SQL authority through product MCP |
| C11 — storage/backup | Keep corrected shared SurrealDB plus beta development tooling and distinct selected logical backup/cold service archive contracts. Files+index or separate search/graph service must replace complete admission/selection/holds/readiness/recovery, not only bytes | A complete product operation is substantially simpler elsewhere or a required engine contract cannot be corrected. Compare full selected graph/search, admission, retention and recovery. Current-state regeneration remains available; historical compatibility/whole-service archives are not new prerequisites |

If a trigger is met, record examined evidence and a supported decision before adoption. Add the
selected bounded implementation, consumer migration and acceptance to the affected package and
coordinator. Uncertainty in correctness-sensitive substitutions retains the protected current route.
Do not invent infrastructure merely to use a feature or delay required Catalog corrections for an
unrelated optional mechanism. C09 remains required affected integration; the other decisions do not
waive diagnostics/generated-beta/ownership delivery or their actual acceptance.

## 6. Acceptance, cutover and handback

All controls described here are **not_run by plan authoring**. During implementation, use compile
checks for touched Rust owners and the minimum revealing affected harness/run/fixture/evidence/
configuration controls. Pure filesystem/process tests need no native service. Verification observes
prerequisites and names repair routes; it does not synchronize, install, restart or silently choose
another target. Applicable leaves run at scope end. Independent review follows the repository's
cadence on the stable composed change, including T01's real writer/storage consumers and T03's capture.

The coordinator's [combined acceptance](catalog-completion-plan_2026-10-14.md#acceptance) owns the
functional completion boundary, actual current-source Catalog/pinned FastMCP exits, scoped composed
acceptance of changed shared contracts and current-state regeneration. A tooling pass cannot
substitute for product admission/publication/cold/original evidence. Changed receipt/trust contracts
must be exercised through their actual affected writers, readers and storage consumers, including
failure/cancellation, irrespective of whether their files are Python. The selected boundary is T01–T04
controls plus V01–V05's applicable composition and leaves; no broad qualify/test-all, unrelated quality
campaign or historical data/benchmark prerequisite is added.

Cut over all changed consumers with their contracts and remove replaced code/guidance/configuration;
retain no parallel supervisor or compatibility-only runtime path without a current named consumer.
Retained historical run evidence keeps its explicit current reader/recovery limits and original bytes.
Inventory/quiesce actual project borrowers before migrating useful views or regenerating pinned
inputs; native/operator state remains with its owner. Do not infer disposal or release from age,
absence, model change, successful body or earlier campaign receipts.

Hand T01–T04's delivered contracts, exact scoped evidence and unresolved limits to coordinator §6.
That owner reconciles inherited OP/LO/LD scope and older plans only against fulfilled obligations,
preserving SA-F02/BC3/SM8 and other unsatisfied or paused work. Administrative link transfer and this
plan's publication establish neither that handback nor execution authorization beyond the approved scope.
