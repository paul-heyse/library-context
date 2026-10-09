# Unified persistent SurrealDB: shared content and scoped execution

**Proposed / Interface-checked foundations · 2026-10-09.** The operator explicitly accepted
source-review RC01–RC07 on2026-10-09. This document designs and schedules the target; it does
not install a service, change production contracts, activate an operator store or establish
runtime qualification. The [persisted coordinator §8](persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition)
owns scheduled findings; its §9.1 owns actual execution receipts. This companion owns the
unified target and UP0–UP9 packages, not another status or retention ledger.

## 1. Outcome, baseline and assessed foundations

Use one durable, authenticated library-context SurrealDB service across compilation, publication,
serving, verification and agent worktrees. Ordinary clients attach to installed compatible state;
they do not start scratch servers, install schemas or allocate databases. Unchanged compatible
completed content is attached through checked current bindings rather than replayed solely to
obtain new ownership. Pending writes, current attribution, admission and publication retain
distinct authorities inside shared storage.

The source is the [unified-persistence review](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md).
Its initial baseline was clean `05cc139b`; plan authoring starts at `23daeaa8` plus concurrent
AF lifecycle/diagnostic edits. Those edits include AGENTS, the SurrealDB guide, fixture and run
owners, verification and documentation tooling. Their contracts must be integrated from current
source; the old review did not assess them. No running qualification, operator database,
runtime credentials, PSE-arrow service or protected evaluation input was inspected or changed.

Assessment uses core/template3.3, efficient-architecture heuristics1.0 and code-intelligence1.5
through [the standard](../design_review/design_principles/standard.toml). Static source and
version-matched library inspection support the choices below; benefit remains unmeasured.

Useful foundations are already present: typed nominal keys and full content digests; generated
graph/reference codecs; exact contribution/view/binding descriptors; immutable selected inputs;
complete membership/absence tokens; current producer contracts; independent admission; bounded
original-byte verification; finite Rust graph/BDD kernels; prepared native selection; charged
handoffs; retained effect certainty; and explicit publication versus selection. These are reused,
not replaced by a generic database wrapper or workflow engine.

The actual mismatches are physical. Canonical table uniqueness currently permits only one payload
per nominal key in a database. Native edges resolve physical endpoints globally. A private database
supplies attempt cleanup, final sealing and reader isolation. Product hits replay typed rows.
Fixture attachments allocate fresh namespaces and coordination roots even on kept servers.
Those boundaries must change together; a longer-lived daemon alone cannot deliver this target.

The workload includes unchanged repeated compilation, analyzer/model variation in concurrent
worktrees, changed/absent inputs, high-degree support, global analyses, concurrent publication
and reads, cancellation and restart. Required whole-universe computation remains whole. Bounded
returned packets and server persistence do not bound examined work or concurrent client heaps.

## 2. Confirmed rules and decision routes

All rows were **explicitly Accepted, 2026-10-09**. Acceptance permits the proposed design; it does
not close findings or prove a working prerequisite. UP0 records the superseding/complementary
decisions and amends their governed owners before dependent implementation. Accepted ADR text
is immutable; affected parts of ADR-0138/0140 require the repository's supersession route, with
surviving guarantees carried forward. ADR-0141's useful access/local-completion contracts remain.

| Source impact | Accepted behavior | Route before dependent implementation |
|---|---|---|
| [RC01](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#RC01) | Shared immutable content, scoped attempts and manifest publication replace private database ownership/sealing | UP0: B7, semantic §15.11 and storage §6; preserve one canonical native authority and no ordinary self-export/import |
| [RC02](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#RC02) | Exact published-view access and pinned executable definitions replace database-wide snapshot meaning | UP0: storage §6.1–6.2, serving/identity owners and runbook |
| [RC03](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#RC03) | Checked attachment may reuse admitted content without row replay; current attribution and independent cold admission remain | UP0: reuse decision/model owner; GR and NE consumer migration |
| [RC04](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#RC04) | Stable service/schema, logical test isolation and explicit disruptive maintenance | UP0/UP7: AGENTS, fixture/verification/runbook and AF diagnostic contracts |
| [RC05](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#RC05) | Reuse compatible completed prerequisites and reconcile unknown effects after failure | UP0/UP3: completion/recovery owners; no arbitrary pending continuation or durable task executor |
| [RC06](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#RC06) | Native owner governs shared-content reachability, pins and retirement; storage manager delegates | UP0/UP8: native lifecycle and storage-management contract |
| [RC07](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#RC07) | Retain patched gRPC initially; qualify optional WS/HTTP combinations against complete contracts | UP0/UP1: transport/deployment owner; no dependency bump or protocol replacement required now |

## 3. One service, explicit attachment and maintenance

Select a managed native RocksDB service, retaining the current authenticated gRPC route. Its
default host root is `${XDG_STATE_HOME:-~/.local/state}/library-context/surrealdb`, outside any
checkout; selected placement is recorded once by explicit installation. Use a stable
`library-context-surrealdb.service` identity. Endpoint, exact binary/configuration identity,
storage engine/durability settings, schema generation and credential references belong to the
installation descriptor, not to per-command scratch files. Credentials are not content IDs,
reader handles or committed configuration. No new fixed memory, job or thread caps are selected.

Use one stable namespace with `main` and `validation` databases. `main` co-locates canonical
payloads, anchors, memberships, completed products, manifests, attempts, effect fences and pins.
`validation` holds synthetic untrusted/control data under the same supported physical schema.
It is not a production clone or database per test. Both use the same daemon and integration.
Pure finite controls remain database-independent. Embedding cache semantics retain their owner;
consumed published vector values belong to exact canonical content, not an independently evictable
cache. No further logical database is a target prerequisite.

Installation/migration is explicit and maintenance-owned. Ordinary attachment verifies exact
service/schema compatibility, opens the required scoped client and acquires its application
ownership. Missing/incompatible state reports `blocked` with the explicit repair operation;
it never silently provisions, starts a substitute or changes selected content. A transport or
readiness refusal does not establish an OOM or other cause without evidence.

Reuse the AF1 run/child/cleanup owner. Separate its client attachment from the service lifetime:
command exit closes that client's scopes and pins, never the daemon or whole database. Worktree
removal cannot remove service storage or shared coordination. Client build/native-extension
provenance remains distinct from the service/schema generation and retains existing environment
guards. Do not copy PSE-arrow's host-admission or receiver framework.

The supervisor owns maintenance admission and actual predecessor drainage. Restart, incompatible
schema cutover, privileged global diagnostics and real crash controls exclude new borrowers,
drain existing ones, verify the exact service/storage identity, then act. An empty process group
or a missing client heartbeat alone is not proof of remote-effect termination. Reopening admission
requires readiness and generation revalidation. Ordinary tests/compiler workers keep their normal
available parallelism.

## 4. Shared immutable content and exact native graphs

### 4.1 Select content-addressed payloads with nominal anchors

Choose immutable content-addressed canonical payloads, separate nominal identity anchors and
compact exact membership. Compare this with closed immutable segments: segments can amortize
membership for nearly identical views, but complicate sparse selection and cross-contribution
sharing. The current deterministic contribution memberships, content hashes and native indexes
make the payload/anchor route the simpler initial integration. No general Merkle-tree framework
or segment optimizer is introduced. Repeated compact view mappings remain a real retention cost.

| Representation | Meaning and governing rule |
|---|---|
| Nominal anchor | Model-contract identity, nominal relation/type and full semantic key. No mutable canonical body and no implication of selected-view existence. |
| Immutable payload | Canonical typed bytes/body under an explicit model/codec identity. Address includes relation, nominal key and full BLAKE3 content identity; hash hits compare exact canonical bytes and declared fields before reuse. |
| Derived role edge | Source payload to nominal target anchor, preserving field, role, position, direction and relationship identity/multiplicity. Derived once per payload; not a rich graph copy per view. |
| Contribution membership | Exact contribution/relation/nominal key to payload address. Pending membership is not a completed read capability. |
| Completed view | Frozen selected contributions plus compact nominal-key to selected-payload resolution. Equal selected payloads deduplicate; two unequal payloads under one nominal key in that view are a conflict. |
| Originals | Source selection resolves to original content/header/chunks; exact content and range verification remain bounded and independent. Global source-anchor existence is insufficient. |

Different payload revisions may coexist across views. Initial compatibility requires the exact
declared model/codec contract; no automatic cross-version equivalence inference. Semantic
content, physical schema generation and runtime ownership have different identities. Preserve
existing contribution/view identities where their canonical meaning is unchanged. UP0 specifies
new manifest/address encoding and advances only affected serialized formats explicitly; reject
obsolete formats before reconstruction, with no compatibility-only readers or codebook renumbering.

Build the compact view resolver once at completion through bounded ordered merge of selected
membership, not owner-by-key Cartesian enumeration. Retain compiler membership provenance;
the resolver is a mechanically derived exact access index, not an independently editable source.
Its readiness and reference closure precede visibility. No native table is unique merely by
nominal key across all views; uniqueness is enforced at the appropriate immutable address or
view-selection boundary.

### 4.2 Native traversal remains view-relative

`ENFORCED` proves physical anchor existence, not semantic closure. Forward traversal resolves
target anchors through the exact selected input view. Reverse traversal filters source payloads
through that view too, excluding unrelated revisions. External/unresolved roles follow their
declared model contracts; they do not silently become internal endpoints. Preserve independent
vertices, isolates, parallel arcs, self-loops, role associations and original-source closure.

Use native composite indexes for exact view/nominal/payload and incoming/outgoing role access.
Lower model-owned selections through one shared resolver for compiler reads, admission,
reconciliation, publication, serving and export. No caller reconstructs its own view predicate.
Retain finite graph/SCC/BDD/fixpoint kernels and projected Arrow/DataFusion where they fit;
native indexed selection/joins/aggregation can reduce data before transfer. A graph traversal
does not replace the semantics of those kernels.

The concrete seams are typed `Record::content_digest`/KeySink, Entity/Assertion nominal versus
content IDs, generated codecs, FamilyHasher conflict rules, compiler memberships, Loader,
NativeReader, original verification and derived-search reconciliation. Migrate those actual
consumers with the storage change; do not call canonical row sharing complete after changing
only the compiler's writer.

## 5. Attempts, checked reuse and durable recovery

Each mutable attempt has an identity, service/schema generation, effect fence and explicit
outcome. Each effectful operation has an idempotent operation identity and retained terminal
disposition. Payload insertion and pending memberships use bounded transactions. Completion
closes/drains its producing work, checks current fenced state and publishes immutable descriptors
through a short guarded transition. Final admission freezes its exact read set and effects,
not every writer on the service. Preserve structured primary/secondary failure and acknowledged
committed identity from PC1/NE6.

All competing completion, attach, pin and retirement operations touch the corresponding guard
records. SurrealDB snapshot isolation and record-target FOR UPDATE do not supply arbitrary
predicate/range serialization. Do not introduce one global transaction lock. Retry a whole
guarded decision only when its failure is known retryable and no unknown effect is concealed.
Lost acknowledgment reconciles the original operation and committed identity before retry.
Client death never authorizes whole-database removal.

Recovery retains completed immutable prerequisites and re-executes missing operations under
current bindings. It does not deserialize live providers, continue arbitrary pending coroutines
or infer completion from partial rows. Staging and unresolved effects remain unreachable from
published views and protected from retirement until reconciliation settles them.

Checked attachment consumes retained content, exact dependencies, operation/model compatibility,
current provenance requirements and native retention eligibility. It atomically establishes
the consumer's current binding and pin, or returns a defined refusal. It does not borrow the old
attempt's runtime grants or relabel old provenance. Share full canonical results only when every
semantic field is equal; share a provenance-free computation product only where its owner
explicitly rebuilds all current attribution. Cold import/admission still checks actual data
independently. Existing optional portable products can remain where they have a distinct consumer;
do not retain a second serialized canonical copy merely to replay it.

The model's existing operation/dependency declarations govern both lookup and readiness. Retain
complete source, model/operation, implementation, parameter/policy, membership/absence/coverage,
provenance and physical-realization distinctions. Changed applicability does not make an older
immutable result false under its original inputs. Use exact reverse dependencies for actual
affected-domain/recovery/retention consumers; no second mutable validity graph or background
automatic recompilation service is introduced.

LIVE/changefeeds are optional wakeups for derived lookup state, not completion or validity.
Reconnect, expired cursor or feed gap requires authoritative-scope reconciliation before a
mutable lookup is considered current. Immutable pinned views do not require observing every
later write. Do not add whole-store rescans to ordinary attachment.

## 6. Manifest publication, scoped access and recovery assets

Publish an immutable manifest binding exact admitted content/completed state, originals,
coverage/outcomes, consumed vector values and answer-affecting executable definitions. Validate
and reconcile that view, ready its derived access structures, then atomically expose its
publication identity. Selection remains a separate explicit operation. Ordinary compilation
never exports/re-imports or rewrites canonical content to publish it.

Use a trusted Rust view-scoped access boundary for ordinary readers. Database VIEWER credentials
are internal and do not imply per-view authorization; never expose them as a snapshot capability
to arbitrary native SQL/MCP clients. Every compiled read, evidence range, resource and continuation
consumes the pinned manifest/view resolver. A serialized handle identifies content/realization;
it is not a reusable runtime permission. Replace the current database-equals-snapshot assumption
in SnapshotHandle/NativeReader, CLI, PyO3/MCP and evaluator adapters together.

Select immutable named definition epochs in the same stable database. Install new function/index
names rather than overwrite old meanings; compare actual definitions instead of trusting IF NOT
EXISTS. Pins retain the required epoch. Compatible additive installation must preserve old
readers; incompatible table/codec/service changes require explicit maintenance and rebuilding.
Do not rely on unqualified temporal-schema/history support for reader or restore correctness.

Search eligibility is exact-view-relative before channel quotas, ranking limits and evidence
hydration. Shared vectors or lexical rows cannot qualify through unrelated occurrences. UP5
must qualify the actual native scoped plan, including approximate index candidate behavior;
post-limit filtering is not equivalent. Until an approximate path establishes this contract,
use an explicitly identified exact eligible-vector operation under its own policy identity,
with bounded transfer/compute where needed. Never silently substitute a different search policy
or publish an unqualified approximate path. Preserve ranking fidelity and all required inputs.

Application snapshots are explicit immutable ordinary records, not engine temporal history.
The selected backup route is streamed logical export of `main` under a native retention hold
and one database read snapshot, with checked application and physical terminals. Include
manifests, payloads, memberships, definitions, originals and required completed products.
Keep service configuration/administrative credential recovery and external selection/coordination
assets explicitly separate, protected and referenced; no secrets in ordinary artifacts.
Active pins/effects are restored as recovery obligations, never as invented live clients.

Ordinary snapshot restore is **data-only lowering**, not native SQL execution on `validation`.
The current `backup_import::send` forwards every parser-owned SQL unit to HTTP import and the
current HTTP helper authenticates Root. Retain PC5's bounded parsing/transport/finality lessons,
not its administrative execution route after the private-database sandbox disappears.
Decode the version-qualified export grammar into declared typed records and definition metadata.
Compare definitions against the selected installed epoch; never execute imported DDL. Reject
USE, executable administrative effects, arbitrary expressions/functions and every unsupported
statement or value form before effects. Captured schema definitions are comparison metadata;
captured user/access definitions are never activated or used to replace current credentials.
Installation/security assets keep their separate recovery owner. Export transaction/option
syntax is framing, not authority to run
the original transaction. Validate canonical addresses and regenerate native role/index forms.

Allocate fresh logical staging/attempt identities and guards through scoped native operations.
Imported manifests/contributions remain claimed data until independent admission succeeds;
imported runtime attempts, pins, credentials and publication markers cannot acquire authority.
Select only the requested manifest's closed data, report any needed recovery obligations, and
establish current publication/pins under the new owner. Two concurrent imports of the same
dump can share immutable values without sharing mutable control identity. Literal bytes, typed
record IDs and every actual export record form must have an exact parser/codec contract; refusal
is required for unknown grammar, not an administrative fallback. Ordinary `publish-artifact`
continues its typed-data route under the same fresh logical staging/admission contract.

Whole-service recovery of approved definitions/configuration and control-state obligations is
a separate explicit maintenance operation after drainage, with independently scoped credentials,
verified service/backup identity and closed admission until reconciliation. It does not expose
the existing Root-based importer as ordinary restore. Neither a dump digest nor its claimed
origin substitutes for restricting executable input. UP6 removes or maintenance-confines that
administrative path when migrating the public restore consumers.

Validation data is reconstructible, not a second authoritative backup. Any multi-database recovery
campaign needs a quiescent common point or separately qualified whole-store snapshot. Logical
export does not promise temporal history or changefeed replay. Ordinary directory copying requires
a closed datastore unless a supported engine snapshot procedure is qualified. Restore/import
uses untrusted logical attempt staging in the stable validation responsibility, then independently
admits data into shared canonical storage. Failed import preserves isolated effects/uncertainty,
not a disposable database. Retained raw checkpoints/captures keep their existing evidence owners.

## 7. Verification, diagnostics, resources and retention

### 7.1 Stable fixtures without substituting cached success

Migrate every native test setup/reset/teardown, not just `just fixture`. Ordinary controls share
immutable named prerequisite inputs and own distinct attempts/effects. Tests of a producer,
derivation, admission or recovery execute that decision under test. Reuse unaffected setup,
not a retained pass. Fresh malformed inputs, missing endpoints and changed/empty domains still
exercise real native enforcement and independent expectations. Include both cache-disabled
production and warm attachment routes. No fixture may remove a whole shared namespace/database.

Raw corruption/deletion controls allocate fresh owned validation payloads and target exact IDs
plus their attempt predicates. An attempt ID alone cannot isolate an unqualified UPDATE or a
mutation of a content-addressed payload shared by other controls. Never tamper with shared
immutable prerequisite rows; tests requiring global schema corruption use maintenance instead.
Migrate the existing unqualified compiler_record tamper controls explicitly.

Keep AF1/AF4 cleanup certainty and truthful outcomes; adapt current AF6/AF7/AF8 interfaces rather
than revive their older script shapes. The current `--sql` accepts effectful arbitrary SQL and
native MCP can expose raw errors; neither is automatically safe on canonical shared content.
Ordinary agent access uses the existing Rust domain tool/view boundary. Privileged SQL/native MCP
is restricted to synthetic `validation` data under explicit maintenance admission, with scoped
credentials that cannot access `main`, complete native-output limits and existing non-sensitive
content requirements. Direct production record-access/MCP authorization is a separate qualified
capability, not a hidden prerequisite or implicit grant of a database VIEWER.

Global DDL mutation and fail-stop/restart tests run in maintenance after borrowers drain, against
the same durable service/storage. Boundary request-loss/fault injection can remain ordinary
attempt-scoped controls. Physical destructive engine corruption is not made safe by a logical
test ID; no such campaign is authorized by this plan. If required later, define an explicit
recovery contract rather than falling back to a scratch server.

### 7.2 Remove work and release representations

UP4/UP5/UP7 trace complete cold/warm compiler and serving operations. Shared server cache is not
shared client heap. Retain compact operation-shaped products only for real consumers; reduce
native rows before rich decoding, hydrate evidence separately and release conversions/kernel
scratch promptly. Preserve byte-aware backpressure, existing charges and cancellation/finality.
No job/thread caps or longer test deadlines compensate for a bad route. High-degree/global cases
must have credible access/kernel/spill behavior; honest refusal alone does not establish fit.

### 7.3 Native lifecycle owns internal retirement

The native owner computes retention from published manifests, declared completed-product holds,
active pins, evidence/replay obligations and unresolved effects. Pin acquisition and retirement
eligibility share native transactional guards so the two cannot race. Release of one consumer
does not release shared content used elsewhere. Generation changes or lost heartbeat alone
do not release an unresolved operation/pin; recovery needs actual drain and identity evidence.

Retirement marks the exact object/epoch unavailable to new attachment, rechecks reachability
and deletes bounded unreachable content under recoverable identity. Its compact progress belongs
to the native owner, not another filesystem history ledger. Orphan staging has its own protected
reconciliation route. The storage manager observes allocation/eligibility and invokes these
operations; it never deletes RocksDB internals. Warm authoritative content has no automatic age
expiry. Capacity pressure reports protected footprint and placement, not permission to evict it.

## 8. Packages, working prerequisites and consumer migration

| Package | Required input | Delivered behavior and revealing completion evidence |
|---|---|---|
| **UP0 — decisions and shared contracts** | Accepted RC01–RC07; current AF/PC/GK/GR/NE owners | Superseding/complementary ADRs, governed owner updates, versioned content/view/attempt/pin/publication contracts and source-qualified scope. Same-key revision, foreign endpoint, empty view and lost-commit cases specify outcomes before consumers change. |
| **UP1 — durable installation and attachment** | UP0 service/generation/maintenance contract; AF1/AF4 working ownership | One host service, stable main/validation and coordination roots; explicit installer/readiness/maintenance; attachment has no provisioning. Two worktrees share service identity; one closes without stopping the other; incompatible/missing generation refuses with repair route. |
| **UP2 — canonical payloads and exact views** | UP0 layout/codec contracts; UP1 working service; existing typed codecs | Loader/compiler membership/original/reference/read/reconciliation migrate together to payload/anchor storage and compact exact resolver. Equal content shares, cross-view variants coexist, same-view conflicts and foreign references reject, isolates/parallel roles/originals remain exact. |
| **UP3 — fenced attempts and current attachment** | UP0 effects contract; UP2 native membership primitives; PC1/NE6 retained finality | Guarded staging/completion/attach/pin and unknown-ack reconciliation used by actual compiler ingress. Cancelled/delayed/conflicting writes cannot become completed through an expired owner; acknowledged completion survives lost response. No global guard or arbitrary pending resume. |
| **UP4 — compiler reuse and dependency consumers** | UP2 exact reads; UP3 checked attachment; GK/GR/NE shared preparation | Providers/normalization/structural/behavioral/selected analytics/catalog/synthesis reuse eligible retained content and current bindings without canonical replay. Each operation has an exact eligibility or Fresh reason. Membership/deletion/provenance changes agree with independent clean recomputation. |
| **UP5 — publication and scoped serving** | UP2 closure/realization; UP3 guarded publication/pins; existing publisher/query owners | Manifest publication and selection, immutable definition epochs, scoped CLI/NativeReader/PyO3/MCP/evaluator/search/evidence/cursors. Publish B while A remains pinned; unrelated/pending B cannot alter A or consume its search quota. Qualify native index behavior and exact-vector policy explicitly. |
| **UP6 — portable import, backup and restart recovery** | UP3 recovery; UP5 manifests/definitions; PC5 bounded parser/terminal primitives | Single-main export plus protected recovery closure; data-only restore lowering validates definitions as metadata and creates fresh logical staging/guards, never administrative dump execution. Concurrent same-dump imports and unrelated tests remain isolated; attempted USE/DDL/control-ID injection rejects. Privileged whole-service recovery is explicit maintenance. Corruption, late errors and unknown effects retain disposition. |
| **UP7 — tests, worktrees and diagnostics** | UP1 attachment; UP2/UP3 logical isolation; AF1/4/6/7 current interfaces; UP5 read-only tool scope | All native fixture/test/verification/run/worktree owners consume stable service scopes. Fresh negative controls, parallel tests, cleanup identity and synthetic privileged maintenance diagnostics work; no scratch fallback or namespace/database teardown remains. |
| **UP8 — retention and composed resources** | UP3 pin/effect guards; UP4/UP5 actual references; storage-management owner contract | Native reachability/retirement plus manager delegation; byte-aware queues/release at affected adapters. Two-view sharing, pin-versus-retire, interrupted cleanup, unresolved writes and worktree removal preserve content. Growth/skew routes are inspected and focused-tested. |
| **UP9 — integrated cutover and acceptance** | Actual migrated UP1–UP8 consumers and their revealing controls | Remove private-database/replay-only/DB-wide-view paths and obsolete formats; rebuild project projections from pinned inputs under explicit maintenance. One final-source coordinated affected acceptance with surviving PC/PJ/GK/GR/NE/CU/BC obligations; no historical pass promoted or timeout waived. |

UP0 settles the common design before implementation; it is not a placeholder for incompatible
later choices. UP1 and codec/finite-contract implementation can proceed independently once their
consumed contracts exist. UP2 delivers actual resolver operations before UP3 relies on them.
UP3's pin/guard slice enables UP5 and UP7 without waiting for all UP4 consumer migrations.
UP6 and UP7 can progress independently on their delivered prerequisites; UP8's guard design is
early even though final integration consumes actual reference owners. Shared declarations,
schema/manifest and compiler/native integration have one writer. Package numbers do not impose
additional barriers beyond these dependencies.

The source review's existing findings, not package completion alone, determine closure.
UP9 absorbs the affected remaining final acceptance into the persisted coordinator rather than
requiring old private-database designs to finish first. Unrelated BC3 candidate-profile evidence
and real-library/operator adoption remain separate. No project store is inspected, rebuilt,
selected or deleted while authoring this document; execution must name the exact owned service,
protected content and maintenance boundary before cutover.

## 9. Library fit, bounded investigations and future change

The locked SDK/engine API is3.3.0 with the narrow gRPC terminal backport. Cached published core
and storage source is version-matched evidence, not installed-server identity. Context7 discovery
and primary source qualify these routes on2026-10-09; no runtime probe or dependency move occurred.

| Capability / alternative | Selected consequence and remaining qualification |
|---|---|
| Native record IDs, composite indexes, relations and transactions | Realize payload/anchor/membership; app-owned exact closure remains. Source: core3.3 `doc/edges.rs:43–81`; current schema/codecs/compiler memberships. UP2 qualifies actual indexed plans, not just DDL. |
| Authorized scope-switching transaction | Core `dbs/executor.rs:1143–1170,1570–1577,2006–2015` retains one transaction across USE; native edges are still DB-local. Main co-location avoids requiring this capability. Any future split needs exact authorization and rollback controls. |
| Trusted Rust reader versus direct native ACL | Core `ctx/context.rs:598–618` bypasses table predicates for scoped system roles. Keep raw VIEWER private. [Record access](https://surrealdb.com/docs/reference/query-language/statements/define/access/record) is a possible later direct-client route with authenticated claims, not mutable query variables. |
| Immutable named definition epochs versus temporal schema | Source `legacy/expr/statements/define/function.rs:35–49` makes IF NOT EXISTS a no-op, not equality verification. `exec/physical_expr/function/user_defined.rs:70–75` has version-stamped lookup, but complete historical index/restore behavior is unqualified. UP5 uses explicit named epochs and compares actual definitions. |
| Patched gRPC versus WebSocket | Existing grpc streaming/finality is selected. [Server query_stream](https://surrealdb.com/docs/reference/rest-api/rpc-protocol#query_stream) does not make the buffered Rust WS query path progressive. Optional adapter needs per-request rows/terminal/error/retraction, cancellation and reconnect contracts; shared socket close cannot stand in for gRPC EOF. |
| Session reuse | SDK Surreal::clone makes a new session; Arc cloning preserves the existing one. Share only compatible immutable principal/database context; bind view per operation. Never mutate shared session selection for concurrent callers. |
| Logical export/recovery | Core `kvs/ds.rs:5562–5580` exports one DB read transaction; `kvs/export.rs:588–589` exports latest records, not all temporal history, and `:111` redacts signing keys. [Recovery guidance](https://surrealdb.com/docs/manage/self-hosted/backups-and-recovery) requires version qualification; later3.3.1 safeguards cannot be assumed in3.3.0. UP6 qualifies complete recovery closure and exact terminality. |
| LIVE/changefeeds | [Changefeeds](https://surrealdb.com/docs/learn/querying/real-time/changefeeds) can drive wakeups with cursor/gap handling; exact manifests remain authoritative. Do not introduce a feed consumer unless it removes actual lookup work or serves a named consumer. |
| Native daemon/container; RocksDB/SurrealKV | Native RocksDB is selected to reuse existing integration. A pinned container with durable volume is viable if packaging yields concrete benefit; SurrealKV needs separate durability/workload qualification. [Storage engines](https://surrealdb.com/docs/build/embedding/storage-engines) do not establish a measured engine winner. |

PSE-arrow's baseline `46545b2a` plus dirty service/configuration changes informed lifetime and
bounded-client questions. Its per-test databases, disposable route and receiver/host framework
are not the selected target. No runtime result transfers across repositories.

UP0/UP2 settle concrete index keys and canonical encoding against real codecs; UP5 qualifies
shared lexical/ANN eligibility and named-definition coexistence; UP6 qualifies backup secrets,
external recovery assets and actual server version; UP7 inventories every destructive fixture
path; UP8 compares repeated compact mappings and retained representations over realistic skew.
These are bounded integration investigations with named consumers. A failure reopens that
specific realization and blocks its dependent package/UP9, not semantic guarantees or all
independent work. No additional engine, universal scheduler, standing benchmark campaign or
new runtime accounting framework is selected by default.

Adding a fact family should change its authoritative model/provider and generated lowering, not
service count. Adding an analytic should reuse exact selected inputs and declare its own settings
and fidelity. An analyzer/model revision should coexist when supported or refuse before effects.
A future transport substitution must preserve the same complete lifetime/recovery contract.

## 10. Verification, source coverage and authoring checkpoint

Implementation controls use the pinned toolchain/profile and normal available parallelism.
Resolve commands through current `just verify --print --select ...` and actual test targets;
UP7 changes fixture provisioning, not the usefulness of focused controls. During development
run touched compile checks and minimal revealing controls. Schedule one affected final-source
assembled acceptance at UP9 because model/receipt/trust/transport contracts change, integrating
the coordinator's surviving obligations. BC3 retains its separate candidate-profile gate and
release defaults until qualified. Do not run broad journeys after each package. Real-library,
Qwen, protected evaluation and operator activation retain their separate authorization.

| Source obligation | Package coverage; required distinction |
|---|---|
| [F01](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F01) | UP0/1/2/7/9; two worktrees share existing content/service, closing one neither destroys storage nor forces replay |
| [F02](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F02) | UP0/2/5/6; equal payload sharing, cross-view revision coexistence, same-view conflict, forward/reverse foreign-view exclusion, originals/isolates/parallel roles |
| [F03](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F03) | UP0/3/5/6; publish B during A, immutable definitions and evidence/cursors, scoped search before limits, no raw VIEWER capability |
| [F04](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F04) | UP2/3/4/9; unchanged no-replay attachment, changed membership/deletion/empty/coverage/model/code/provenance against independent clean results |
| [F05](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F05) | UP0/3/6/8; delayed writes, lost commit acknowledgment, client death, concurrent equal insertion/conflict and restart retain exact identity/certainty; data-only restore cannot inject old runtime guards/pins |
| [F06](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F06) | UP1/3/7/9; concurrent logical tests, true fresh/cold controls, complete setup/teardown inventory, explicit disruptive maintenance |
| [F07](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F07) | UP2/4/5/7/8; composed cold/warm and skew/global routes, retained kernels, bounded conversion/queues and timely release without thread caps |
| [F08](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F08) | UP3/5/6/8; two views sharing content, pin/retire race, interrupted cleanup, unresolved effects, worktree removal and owner-delegated storage management |

This coverage map is not a disposition table. Current status and closure evidence remain solely
in the coordinator §8/§9.1. The source review's domain-dependency composition, deployment/transport
alternatives, changefeed gaps, recovery assets and all ten change/failure scenarios are developed
above; no recommendation is deferred merely because earlier open work overlaps it.

**Authoring checkpoint, 2026-10-09:** plan and cross-plan integration are Proposed. The independent
[target-plan assessment](../design_review/reviews/design_review_unified-persistent-surrealdb-plan_2026-10-09.md)
accepts the corrected target at Proposed / Interface-checked strength. Its TF01 identified
ordinary raw-SQL restore's missing shared-staging boundary; §6/UP6 now select data-only lowering,
fresh control ownership and explicit maintenance recovery. TF01 is resolved in the document;
implementation evidence remains with the coordinator. Documentation **passed**, 2026-10-09:
`just docs-check`,374 canonical pages, zero link errors. Production builds/tests, services,
database operations, cleanup, dependency changes, activation and performance measurement are
**not_run** for this authoring task. Existing assertion failures/timeouts retain their coordinator
receipts; this design does not attribute all of them to startup, transport or missing persistence.
