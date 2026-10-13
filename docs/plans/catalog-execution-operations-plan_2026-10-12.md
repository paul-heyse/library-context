# Catalog execution: lifecycle and development operations

**Accepted target / inherited foundations, 2026-10-12.** Companion to the
[coordinator](catalog-execution-plan_2026-10-12.md), the sole owner of dispositions and receipts.
This document preserves the useful lifecycle taxonomy, diagnostics, SurrealKit integration and
capability decisions from the retired lifecycle plans. It specifies their remaining integration
with the revised completion contract; it does not restart completed work or claim new runtime passes.
[ADR-0165](../adr/0165-prepared-completion-and-owned-lifecycle.md) carries the enduring decisions.

## 1. Responsibility and foundation assessment

Execution, retention and semantic eligibility are independent. Acquisition inputs, attempts,
contributions, exact views, admitted artifacts, publications, selections, pins, reusable products,
backups, native obligations and host attachments have different owners. At their actual decision
interfaces retain authority, mutability, visibility, eligibility, retention consumer, liveness,
uncertainty and recovery obligation. No universal lifecycle enum, registry or second scheduler follows.

The existing model completion values, exact native holds/incarnations, publication pins and host
service/run/storage owners remain foundations. Native certainty is not host process status; a
successful body is not successful cleanup. OP01 consumes their observations without deciding release.
OP02 contains beta private clients at a whole-service boundary. OP03 records complete-route choices,
including justified non-adoptions; platform availability alone does not create work.

Current source routes are [model completion](../../crates/lctx-model/src/domain/completion.rs),
[native diagnostics/effects](../../crates/lctx-native/src/control/authorization.rs),
[signal writer](../../crates/lctx-native/src/signal.rs), [fixture observer](../../scripts/surrealdb_fixture.py),
[service owner](../../scripts/surrealdb_service.py) and
[development worker](../../tools/lctx-surreal-dev/src/main.rs).
The coordinator distinguishes their implemented and focused-Tested scopes from remaining acceptance.

## 2. OP01 — diagnostics, inspection and provenance integration

Keep the model-owned bounded diagnostic projection: stable class/code, stage, bounded secondary
causes, optional known native request/attempt/generation/era identities, and safe owner-specific
inspection or recovery route. Preserve opaque pre-admission/third-party causes and absent identities.
Human text is presentation, not a machine protocol. Truncation is explicit; recovery information
never grants an effect or disposal authority. Codes remain append-only where codebooks apply.

The actual native signal writer and host/report readers already preserve these values. When CP00–CP04
change completion boundaries, migrate their diagnostic producers and consumers together: identify
contribution preparation, internal selection or retained-boundary failure without fabricating a
native attempt for pure work. Preserve original typed errors and independent local/remote/storage/
committed-effect axes. No historical receipt translator or backend SQL/credential leakage is needed.

Compose existing status/inspection routes for target/service generation, host attachment/run,
native operation, exact boundary/publication/pin, backup/retirement and retained evidence where those
identities exist. Show actual owner, observation source/date, uncertainty and valid next route.
Closed maintenance reports unobserved native state plus its maintenance diagnostic route; inspection
must not reopen admission or query a closed installation implicitly. Missing receipt, dead worker,
logical DROP and empty host borrowers do not prove remote drainage.

Retain on-demand native plan/index inspection and coarse phase correlation. Plain EXPLAIN is an
inspection; EXPLAIN ANALYZE executes and needs its actual authorized workload owner. Versioned plan
output is diagnostic, not a stable product wire contract. Keep server source/recipe/binary/generation,
SDK/RPC patches, beta tool version, configured capabilities, schema/validity epoch, executable epoch
and publication identity separate. Borrowed executable observations use the original authenticated
executable owner, not the campaign directory. A recipe does not identify a running binary.

**Remaining delivery:** adapt changed completion identities/diagnostics and run only affected controls.
Retain real writer-to-host round-trip coverage, opaque/absent identity and truncation cases, command
failure with committed effects, Unknown remote state and cleanup failure. Inspection must remain
read-only, credential-safe and honest under missing target or permission refusal. Previously passed
unchanged cases need not rerun solely because plans were superseded.

## 3. OP02 — generated schema and explicitly owned development targets

The selected development library is **SurrealKit 1.0.0-beta.1**, not a beta product SDK. Preserve the
exact macro pin and manifest-only vendor patch documented in
[SurrealKit provenance](../../third_party/surrealkit/README.lctx.md). Product SDK/RPC stay on the
patched 3.3.0 family. Development selects gRPC/HTTP/rustls, without unused SDK defaults/local engine/
JWKS; an enabled feature is not qualified operation support. The product CLI has no SurrealKit edge.
New version moves, if a real need emerges, require narrow lock changes and affected controls rather
than a wholesale re-resolution.

Direct Sync calls use separately configured, directly owned Any sessions under the host-issued
binding. A clone is a different session; Any invalidation clears authentication, not rollback.
Beta's public tester creates private clients and does not accept injected repository clients.
Contain it inside an exclusively owned remote service with actual authentication and server-side
capability ceilings. Do not claim per-session cancellation ownership for those private clients.
No buffered transport fallback supplies patched product progressive/finality promises.

Use the existing worker's exact endpoint/namespace/database/generation/purpose/credential-reference/
effect binding. Campaign and validation-installer descriptors must agree before effects, and actual
validation marker readback verifies generation/schema. Current support is managed local gRPC and a
fresh `lifecycle_<32-lowercase-hex>` database in `library_context`. Remote/TLS or API-request support
requires a concrete new consumer and explicit qualification; it is not an unfinished default feature.

Native/model definitions remain the sole schema/query authority. The explicit generator emits
ordered `schema/<responsibility>/000.surql` assets for control, canonical, physical search, compiler
and completion; `build.rs` tracks directory membership and included content. Avoid a native build
dependency that duplicates the host closure. Ordinary Sync consumes embedded bytes without writing
a checkout. Generated assets exclude the same shared mutable function templates as native base
installation; immutable published function epochs remain publication-owned.

Named-module Sync and the tester's generated default-module projection consume the full ordered
requested-module dependency closure. Refuse unsupported combinations instead of passing an ignored
selector. Render lifecycle-sensitive variables into SQL before beta hashing. Compare required
installed definitions against native authority, normalizing only application modifiers, preserving
function bodies, permissions, indexes, nested SQL and lifecycle-sensitive names. Permit beta
bookkeeping/explicit fixtures without claiming equality of unrelated database objects. Parsing is
syntax evidence; installed permissions/functions and definition identity require actual controls.

When CP04 changes physical definitions, regenerate these same assets and update projection/parser/
installed-definition controls in the same package. No second semantic schema or introspection-based
authority is introduced. A transformation gap must be explicit; a narrow native application of the
same generated asset on the same owned target is preferable to silently dropping definitions.

**CP04/CU03 source integration, 2026-10-13.** The native installer and host readiness now require
schema8. The detached-copy discriminator preserves multiple copies of one completed logical
descriptor. Generated completion definitions target the v2 compound contributor/selector inventories;
compiler definitions add `contribution_nodes` alongside `contribution_rows`. Fixed index inspection
and the declarative sample query canonical contribution membership rather than removed per-view
member tables. Exact generation and owned-target readback outcomes belong to coordinator §6; a
successful generator alone is not installed-definition evidence. No product SDK or beta pin changed.

The worker exposes Sync and declarative tests, with `fail_fast=false`, normal available suite
parallelism, `keep_db=true` and JSON reports. Watch, seed, rollout, type generation, API requests,
embedded-engine and selectable fail-fast are unexposed, not implied remaining adoption obligations.
Future support needs its named consumer and matching controls. A setup failure can produce no
report; a completed failed assertion can produce healthy capture. Neither means success.

## 4. Whole-target ownership, recovery and focused acceptance

Reuse executable-generation, managed-process/run and storage owners. Register exact purpose,
generation, endpoint, datastore/credentials and consumer before launch. Ordinary product attempts
use installed logical validation isolation. Privileged schema/permission/destructive beta suites use
a wholly owned disposable campaign; private root clients never attach to shared/operator targets.
Existing old paused campaigns are not recovery prerequisites or spare test environments.

The parent retains the whole target even if beta produces no report or creates unexpected suite
databases. Record worker exit, capture health, local cleanup, native certainty and target disposal
independently. Stop/drain authenticated workers and submitted owners before exact daemon shutdown
and storage release. Existing disposal-only logical close/drain actions are bounded under their
owner, journaled before launch and reconciled on resume. Failed logical drain plus confirmed whole
executor shutdown/discard can settle disposal while preserving the failed/Unknown original records.
Unknown local/native executor drainage retains the current obligation; cancellation is not disposal.
Generic sweeps do not retire native databases.

**Accepted refinement, 2026-10-13 (ADR-0167):** exact owned campaign disposal may transfer a strict
private dead ordinary-maintenance marker after fresh current participant and executor authentication,
journaled under existing storage/coordination exclusion. Missing historical marker-child ancestry is
not an independent prerequisite. Specialized recovery/SOURCE_JOIN and original Unknown remain intact.
Future ordinary maintenance enrolls managed children before spawn; explicitly supplied older run
references add current-quiescence obligations without claims about their target or ancestry. Required
runtime disposal and source/host acceptance are recorded only in coordinator §6.

Token substitution journals the original placeholder before mutation. Restore on ordinary errors
and after authenticated worker drainage using exact campaign/generation/run/binding/storage/private
path checks and atomic recovery. Retain the existing recovery consumer until restoration; never
adopt an older project without its journal. Recovery precedes credential-removal completion.

**Remaining acceptance:** validate changed generated definitions and any changed diagnostics/ownership
seams using revealing cases. Preserve existing actual root/record/token authentication, Sync,
required-definition readback, declarative report, setup failure and token-SIGTERM restoration evidence
within their scopes. A cancellation case observes a committed marker before SLEEP, then distinguishes
worker cleanup from private-session finality and checked target discard. Report failures before
capture, after commit, during restoration and during disposal separately. No broad tooling family,
container deployment, beta upgrade, historical capture or new observability platform is required.

Pure transformations establish meaning only; owned schema suites establish generated definitions,
permissions and tool containment; actual product controls establish persistence/admission/publication.
None grants operator adoption or a measured speed claim. The coordinator owns integrated acceptance,
current-state regeneration and handback.

## 5. OP03 — supported capability decisions and reconsideration

The operator chose **decide each fit**, not blanket adoption. The decisions below were source-assessed on **2026-10-12**, against repository baseline `9a7ad768` and the pinned 3.3.0 server source identified in [versioned research](../design_review/evidence/2026-10-12_surrealdb-lifecycle-target/library-contracts.md). They are supported architectural choices at **Proposed / Interface-checked** strength, not runtime substitutions or Tested claims. Existing and proposed required routes remain subject to their package acceptance. Source assessment is sufficient where the candidate adds obligations without supplying the complete consumer contract; no probe is required merely to reject that substitution. Use Context7 first for new documentation questions, then pinned primary source where current docs differ.

For every row, record the actual consumer, existing complete route, candidate mechanism, meaning/finality/resource/operational differences, decision and reconsideration trigger. Source is sufficient when it settles fit. A needed probe uses OP02's owned target, fresh data and normal concurrency; no shared/operator service or historical capture is required. Negative search alone is not proof of absence.

| ID / avenue | Consumer, complete-route comparison and decision | Reconsideration trigger / bounded delivery |
|---|---|---|
| C01 — record references / retention exclusion | **Retain native holds/incarnations; reject REFERENCE as their replacement in this target.** The consumer is shared contribution/blob/endpoint retention during creation and bounded retirement. Native references provide reverse lookup/deletion policy, but do not replace issuance, exact incarnation, active-phase exclusion, cleanup nomination or recoverable request outcomes. Inspected backlink insertion plus deletion-range scanning does not establish the necessary common exclusion. Adding references alongside the remaining protocol would add a second retention representation. | Reopen for a specific relation whose complete protocol becomes simpler, with source-established common exclusion plus owned create/delete/recreation controls. Do not infer that the engine is broken from the unqualified concurrency premise. CP00 remains prerequisite to any future substitution; no CASCADE on shared evidence. |
| C02 — events, changefeeds, live queries and history | **Use on-demand owner inspection; do not add a canonical event queue, refresh subscription or database history retention.** The present consumer is OP01's operator/agent inspection of current outcomes. Events cannot acknowledge host cleanup; 3.3.0 retry exhaustion can delete queued events, changefeeds require a separately owned cursor/gap recovery route and live queries are transient. Existing immutable publication/receipt identities already supply the required observation and recovery contract. | Reopen when a named consumer needs proactive refresh or durable mutation replay that polling cannot reasonably serve. Specify deduplication, bounded exhaustion, cursor retention gaps and reconciliation first, then qualify only that auxiliary consumer. No current claim depends on 3.3.1 event behavior or versioned SELECT. |
| C03 — bounded sweeps and timers | **Keep explicit owner-directed sweeps; do not activate a timer.** The consumer is an operator releasing eligible host outputs or advancing native retirement. Existing effect owners already recheck current records and admission/owner guards; a timer cannot replace those decisions and adds unattended execution/reporting. Expiry is nomination only. Current one-operator execution needs explicit cleanup after owned campaigns, not a new scheduler. | Reopen on a concrete recurring unattended cleanup need. Preserve the same checked owner effect boundary, fresh native/borrower observations and interrupted-operation recovery; timer delivery must have an inspectable run/outcome. This decision does not qualify every existing sweep race or allow cached references to become permanent deletion authority. |
| C04 — GQL, lightweight relations, inline/count/materialized views | **Retain indexed protected native selection plus existing Rust graph kernels; no additional graph representation or query front end now.** The consumer is selected operation/evidence hydration and finite analytics. Existing participant/reference indexes already express attributed direction/role and exact selected membership. Property-free edges would either lose assertion identity/multiplicity or require another rebuildable projection. GQL's real HTTP endpoint still adds a session/response boundary; counts/materializations add selected-universe invalidation duties. Inline adjacency is suitable only under its actual fixed-cap coherence contract, not a general replacement for selected filtering. | Reopen for a named complete traversal/count whose native lowering removes material current work while preserving selected universe, isolates, direction, attributed multiplicity and terminal ownership. A cap change must drain writers or qualify its race; high-degree spilled caches remain a supported case. No new physical stage per semantic family. |
| C05 — lexical/vector/hybrid retrieval | **Retain and qualify the existing composed native lowering; reject a direct scorer/fusion/ANN swap.** Consumers are publication-scoped lexical/vector search and winning-evidence hydration. Current code already uses native analyzer/FULLTEXT nomination and cosine. It scores against frozen exact-view/family corpus statistics, applies eligibility before caps, and fuses channel then family ranks with context-specific winning witnesses. Table-wide BM25, one-level generic RRF and approximate KNN do not supply that complete contract by themselves. Adding ANN now would introduce index readiness/memory and approximation semantics without removing those owners. | Reopen for an exact equivalent lowering or an explicitly selected approximate retrieval contract, with independent fresh known-answer eligibility/tie/witness cases and actual encoder/projection/index readiness. Larger inputs may justify ANN or DiskANN, but a benchmark is not a prerequisite to identifying that need. No historical numerical fixtures or broader retrieval redesign is imported. |
| C06 — DEFINE API / HTTP / GraphQL front ends | **Keep Rust-owned domain operations and existing transport; do not add a database-hosted product API.** The consumer is exact selection/get/search/compare/evidence under publication pins. A database handler would need its own complete authorization and pin/cancellation/deployment composition. Pinned DEFINE API disables table/field checks inside the handler after endpoint authorization, and HTTP permits only NONE/bytes/string response bodies. This shifts security and serialization responsibilities rather than deleting the native/host owners. A generic GraphQL front end also does not supply the selected-domain contract. | Reopen for a complete operation demonstrably simplified by database placement; explicitly specify endpoint permissions, auth limits, response encoding, retained definitions/pins and checked terminal cleanup. `api::invoke` object behavior is not HTTP qualification. No general endpoint is enabled by this decision. |
| C07 — Surrealism / WASM modules | **Keep current pure Rust kernels; do not relocate extraction or add a module framework.** Consumers are finite analysis, ranking and admission computations already local to their typed inputs. WASM relocation adds unsigned executable provenance, bucket/package lifecycle, state scope and experimental runtime deployment without removing semantic authority or data-movement requirements. Pinned module definition performs only best-effort runtime warming, so successful DDL would not settle usable computation. | Reopen for a particular bounded computation whose complete in-database route reduces transfer or other machinery. Before adoption, qualify executable identity/trust, module state, actual invocation, cancellation and retirement under retained definition epochs. No inference of module runtime success from DEFINE alone. |
| C08 — file buckets / bulk originals | **Retain corrected native blobs/layouts; reject a bucket migration for current originals.** Consumers require exact source windows, immutable digest/inventory, selected holds and coherent selected backup. Buckets separate byte operations from record/control transactions. Pinned REMOVE BUCKET deletes the definition only, and export emits bucket definitions rather than object bytes. A bucket therefore adds independently owned byte admission, backup and disposal instead of eliminating originals' semantic associations and protection. | Reopen when a concrete large-object workload exceeds the suitable native route or a complete bucket design demonstrably simplifies it. Require object/record failure semantics, immutable exact reads, selected byte inventory/backup/restore and independent disposal. No universal atomicity claim is made; that proof is unnecessary to reject the present pointer-only replacement. |
| C09 — Studio, EXPLAIN, schema/index/parser tools and agent skills | **Adopt on-demand native plan/index inspection in OP01 and generated schema parsing in OP02; keep Studio/agent integrations optional.** Consumers are foundation developers checking actual set-oriented access and generated definitions. Native EXPLAIN/index state answer those questions without a second data model or dashboard. CLI parsing is a useful syntax boundary, while installed permission/function behavior remains separate. | Required delivery stays in OP01/OP02: explicit authorized target, least-privilege read-only templates, current generation/version, bounded capture and honest unsupported/not-inspected states. No stable public protocol is inferred from EXPLAIN JSON. Reopen Studio/custom agent wiring only for an operator workflow needing interactive inspection; edition-specific audit features are not assumed. |
| C10 — native MCP, telemetry, Cloud/control tooling | **Keep domain MCP and current owned logs/receipts; do not deploy generic DB MCP, telemetry collectors or Cloud tooling by default.** Consumers are agent-facing pinned evidence and lifecycle diagnosis. Pinned `surreal mcp` starts its own datastore and uses an owner-level stdio session; it is not an attachment to the managed product service. Native HTTP MCP is a general database surface. Metrics/traces may aid diagnosis but cannot establish exact native owner finality or replace the receipt. No current delivery requires cloud control or collector infrastructure. | Reopen for a named diagnostic question unavailable from OP01 plans/receipts/logs, or an actual remote deployment. Choose the minimal instrument, explicit target/auth/capabilities, redaction, retained signal consumer and release route. No license-based exclusion; no accidental embedded datastore or general SQL authority through product MCP. |
| C11 — complete storage/backup alternatives | **Retain corrected shared SurrealDB with beta development tooling and the existing distinct backup contracts.** Consumers are shared immutable contribution reuse, exact publication selection, connected evidence/search and independent restore. Files plus a small index simplify immutable bytes but require a new coordination/hold/selected-index/backup composition; a specialized search/graph service duplicates selected identity, readiness and disposal. Generic export does not provide the application's exact pinned closure or recreate native runtime authority. The current design already separates selected logical backup from a cold service recovery archive. | Reopen for a concrete complete product operation substantially simpler on another mechanism, or an unmet engine contract that corrections cannot resolve. Compare full admission, selected graph/search, retention and recovery, not only byte writes. Current-state regeneration remains available; neither whole-service archives nor historical export compatibility is a new prerequisite. |

For an adopted mechanism, add its bounded implementation/consumer migration and verification to the affected companion and coordinator §4/§6 before execution. Resolve changed rule impacts explicitly. Adoption must not introduce a new dependency on old data/evidence, delay the highest-priority F01 repair, or hide unbounded movement behind a small returned limit. A rejection/deferral closes only the fit decision; it cannot close an unsatisfied mandatory finding.

### Source basis and limits of the decisions

The decisive evidence was read directly, not inferred from feature names or an empty search:

- **Retention/events:** pinned `core/src/doc/field.rs:877–944` writes reference keys and optional inline state; `doc/purge.rs:455–542` scans backlinks and applies REJECT/CASCADE. `doc/event.rs:146–172,489–526` persists asynchronous enqueue and removes exhausted/final-error events. Application [retirement](../../crates/lctx-native/src/control/retirement.rs) retains exact incarnation nomination and recoverable effects. Native reference concurrency remains unqualified; that is not a demonstrated engine defect and is not a premise of the keep-current decision.
- **Sweeps:** [Storage.retire](../../scripts/storage_lifecycle.py), lines 1085 onward, reacquires exclusive storage admission, owner and metadata guards before current disposition and its journaled effect. `sweep`, lines 1198 onward, invokes that owner per candidate. Its operation-level reference snapshot is not claimed to prove every concurrent reference edit safe. Timer adoption cannot improve that semantic boundary; it would need the same current rechecks and independent interruption controls.
- **Graphs/retrieval:** pinned `server/src/ntw/gql.rs:1–85` is an active session/capability/timeout HTTP route; `idx/src/inline_cache/mod.rs:1–37` documents snapshot equivalence, spill and a cap-change invalidation race. [SelectedPayloads](../../crates/lctx-native/src/selection.rs) already owns exact physical selection. [Search](../../crates/lctx-serving/src/search.rs), lines 291–345 and 384–586, retains eligible occurrences, exact-view scoring and exact vector identity. [Lexical statistics](../../crates/lctx-native/src/lexical_stats.rs) and [ranking](../../crates/lctx-model/src/domain/serving/ranking.rs), especially lines 435–475, establish why table-wide statistics or one-level RRF is not an equivalent replacement. Approximation is a semantic choice, not merely an index toggle.
- **APIs/modules/buckets:** pinned `core/src/api/invocation.rs:117–185` changes permission enforcement; `server/src/ntw/api.rs:129–145` restricts HTTP body representations. `core/src/legacy/expr/statements/define/module.rs:100–137` logs failed module warming while retaining the definition. `remove/bucket.rs:43–56` removes only metadata; `core/src/kvs/export.rs:147–150` emits bucket definitions. These concrete additional obligations support the negative substitution decisions without pretending their complete runtime behavior was tested.
- **Diagnostics/MCP/storage alternatives:** pinned `server/src/cli/validate.rs:30–91` parses without executing definitions; `cli/mcp.rs:20–141` creates a datastore and owner session. `ntw/mcp.rs` provides a capability-gated HTTP general database surface; `telemetry/mod.rs` supplies actual community OTel/log configuration. `core/src/exec/operators/explain.rs:49–54,85–88` distinguishes plain EXPLAIN, which formats a plan without execution, from EXPLAIN ANALYZE, which executes it. Analyze must therefore be an explicitly authorized bounded read workload under its actual owner, not an arbitrary SQL side effect hidden inside inspection. [Selected backup](../../crates/lctx-publisher/src/backup.rs), lines 13–92 and 95–138, retains its pin, one transaction-bound export, explicit cancellation/invalidation and distinct cold-recovery metadata. The [storage/publication owner](../design/sections/storage-and-publication.md) defines the complete shared catalog route compared in C11.

`core/`, `idx/` and `server/` above resolve beneath the exact retained upstream checkout named in the [library-contract evidence](../design_review/evidence/2026-10-12_surrealdb-lifecycle-target/library-contracts.md#source-identities). Source was inspected without opening its installation or executing its server. No source inspection here is a new version qualification. Current diagrams/claims outside these selected consumers, arbitrary engine backends and future deployment needs remain unexamined.

### Delivery consequences

C09 is a positive adoption through already required OP01/OP02, not an optional production rollout or a separate status ledger. The other rows select the current complete route or a concrete demand-triggered optional integration; none requires an optional new production mechanism in the retained lifecycle foundation. This conclusion does not waive F01–F05, the generated-beta integration, actual diagnostics or their runtime acceptance. A supported future adoption must add its implementation/consumer migration and acceptance package before execution; the current decision rows are not perpetual prohibitions.

### Investigation results must be usable

A decision cannot read merely “investigate later.” State the question, evidence examined, which supported outcome follows, and what implementation is then required. For C01/C08 and other correctness-sensitive substitutions, uncertainty means retaining the established protected route while the decision package gathers evidence. No unsupported native primitive is allowed into a production prerequisite. If evidence disproves the benefit or shows no current consumer, record that result and a concrete future trigger instead of inventing infrastructure to use the feature.

A retained-witness decision belongs to CP00; this document does not create another witness owner. Beta feature/API correctness belongs to OP02. Common native definition/session/ownership premises are settled once and linked, not independently redefined for each capability.


## 6. Dependency and delivery boundary

OP01 and OP02 are largely reusable implemented foundations, not replacement projects. CP04 supplies
changed physical definitions and native boundary identities; OP01/OP02 migrate their actual
consumers alongside that package. OP03 decisions remain applicable unless the new complete route
changes a named premise; record any reopened decision and its dependent work in the coordinator.
The new contribution authority is operation-owned and not an adoption of a portable proof cache.

All required adaptations complete before CE01's scoped integrated acceptance. CE02 regenerates only
actual selected project state under real drainage; CE03 then reconciles older plans. A platform
non-adoption closes only its fit decision, never a mandatory finding or required functional assertion.
No progress ledger or new evidence registry belongs in this companion.

At operator closeout2026-10-13, schema8 host controls and checked owned campaign disposals retain
their exact outcomes in coordinator §6. Owned g's failed diagnostic left ordinary maintenance
closed; reviewed server-recipe and predecessor-child proof requirements still apply. Its protected
service/attachment obligation is not settled by other targets' successful disposals. No shared or
operator target was changed, and no source-recipe or maintenance-owner guard was weakened.
