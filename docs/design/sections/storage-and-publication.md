# Storage and publication

**Accepted shared-content target / implementation in progress, 2026-10-09 (ADR-0145).**
The [unified plan](../../plans/unified-persistent-surrealdb-plan_2026-10-09.md) develops durable
service attachment, immutable payloads/exact views, manifest publication, admitted reuse and
native pin-safe retirement. The persisted coordinator owns actual receipts. Prior private-database
receipts retain their original scope and do not qualify shared persistence.

## §5 Projections

**Implemented bounded access, 2026-10-09; enclosing planner qualification remains open.** Native prepared
demand owns bindings, result positions and expected terminals. Scope layout follows each physical
table's actual inventory; graph scope_context and independently owned search columns remain.
Compact atomic candidates first restrict physical pointers to the exact completed contributors,
then use bounded external ordering, strict membership checks, projected hydration and residual filtering.
Unrelated same-nominal revisions cannot create a conflict before that restriction. Whole-match arrays and growing native union
deduplication are retired. Physical export order and nominal selected order retain distinct adapters.
DDL, constructed rows, independent imported-value reconciliation and realization identity change
together; the [correction plan PC3–PC4](../../plans/persisted-execution-corrections-plan_2026-10-07.md#4-one-native-demand-interpretation-and-bounded-candidate-execution)
owns implementation and actual planner qualification.

> Decision: ADR-0145

**Implemented compiler and published native projections, 2026-10-05.** Named projection contracts specify their
completed source, semantic roles, contexts, vertex universe, direction, multiplicity, weights,
coverage, provenance and declared simplifications. Vertices are independent of edges; isolates and
parallel attributed assertions survive. Dense indices are private and map back to semantic IDs.
Compatible compiler consumers share compact topology; rich evidence is hydrated separately.

The accepted persisted target constructs these compiler views from exact completed native memberships, without a
publication handle. Indexed native selection and projected Arrow streams feed model-owned kernels. Summary, Structural and optional analytical consumers wait for their actual
semantic/vector predecessors. Published export uses scoped native queries and terminally successful
responses under the same projection contract. External destinations remain consumer-triggered.

> Decision: ADR-0145, ADR-0103, ADR-0044, ADR-0085

## §6 Persistence and publication

**Accepted target / implementation in progress, 2026-10-09 (ADR-0145).** One managed service
hosts `main` and `validation`. Explicit maintenance installs compatible schemas and credentials;
ordinary clients only attach. Tests use logical attempts and publications in validation, with no
scratch service or per-test database fallback. Compiler, store, publisher and serving retain their
existing semantic responsibilities. Patched gRPC preserves progressive streaming, cancellation
and checked physical terminality. Optional transports must preserve those contracts.

**Implemented recovery refinement / integrated qualification open, 2026-10-10 (ADR-0146).**
Native upgrades preserve atomic page effects and progress under immutable checked host successor
authority. Schema evolution in §6.3 owns the complete transition and recovery contract;
the persisted coordinator retains actual migration receipts and enclosing acceptance.

> Decision: ADR-0145, ADR-0146

### §6.1 Admitted content and native publication

Immutable payload identity includes model/codec, relation, nominal key and full content digest.
Hash hits compare canonical bytes. Nominal anchors contain identity without view existence.
Derived role edges connect source payloads to target anchors; exact completed views resolve the
selected target payload before forward or reverse traversal. Compact view mappings preserve
full keys, conflicts, absence and every generated backing family, including originals.

Durable attempts and effect fences own pending contributions. Contribution completion is local;
private core admission establishes the exact semantic boundary. Compatible retained admitted
content can attach through checked current ownership without canonical replay. Current provenance,
exact dependencies and independent cold admission remain mandatory. Unknown acknowledgement is
reconciled through an operation identity, never inferred from local process cleanup.
Attempt closure fences writes before bounded ownership cleanup. Unadmitted product cleanup retains
attempt prerequisite roots until every product is removed, so interrupted cleanup remains discoverable;
admitted products survive. Remaining ownership rows provide conservative retry progress.
Cleanup effects use the exact terminal attempt's original epoch, checking owner/state/epoch at
both durable intent and execution; deleting owned holds/products does not allocate a new global
installation epoch per page. A durable hold-cleanup intent spans bounded guarded deletion pages;
only the final empty-scope proof marks it completed. Reconciliation fences further pages of an
incomplete operation, while a fresh intent can resume the remaining ownership rows. Partial
cleanup is not rollback. Ordinary open-attempt and pin authorization remain separate.
Imported memberships retain `attempt→contribution→membership` reachability, including partial
ingress. They need no duplicate direct attempt root once their already-rooted contributor owns
them; canonical, alias and backing-row ownership retain their separate existing protections.
View registration groups descriptor writes and ownership edges in row/byte-bounded windows,
while each exact membership stream still reaches checked EOF and its declared cardinality is
compared with actual rows. A claimed empty descriptor does not establish emptiness. Views enter
the local completed inventory only after their ownership operations succeed.

A short guarded transition publishes an immutable manifest over admitted views and one executable
definition epoch. Handles bind semantic/realization identities, publication, exact view set and
service generation. Captured bindings are validated and ordered by their logical binding keys
before marker serialization, so fresh physical attempt IDs cannot change an equal manifest's bytes.
Definitions are installed under immutable epoch names. Ordinary publication
cannot install DDL, mutate unrelated content or replace another publication's functions. Selection
remains explicit. Later cleanup failure preserves an acknowledged committed publication.

**Accepted target, 2026-10-10 (ADR-0145); implementation/qualification open.** Logical
backup resolves the requested publication's model-declared recoverable closure through bounded
indexed windows or exact batches within one explicit transaction-bound snapshot and reader pin.
Manifest and definition comparison metadata use that same transaction. The typed data-only writer
publishes staged output only after statement/outer terminals, physical EOF and explicit transaction
cancellation; it does not spool unrelated main content. Shared selection is not shared admission:
every restore independently checks actual typed data, originals and completed semantics.
RocksDB snapshot reads retain concurrently retired rows; unrelated retirement need not wait for
logical export. Protected cold service recovery retains exclusive
maintenance and drainage. Restore treats its input as data, never as
Root-authorized SurrealQL: closed literal data is lowered through typed staging and independent
admission. Definition metadata selects trusted executable generation. Grammar-owned comparison
normalizes only declaration application modifiers (`OVERWRITE`/`IF NOT EXISTS`); schema, bodies,
permissions and other attributes remain exact, and imported definitions never execute.
Completed-state ordering uses the existing portable row allowance and the restore's composed
budget. Subsequent whole-line envelope and decoded native-weight admission remain independent.
Imported runtime control
records cannot restore grants, attempts, fences, pins or visibility. An explicitly selected manifest
chooses the recovery closure; ambiguous multi-publication convenience restore refuses.

Native reachability owns publication roots, admitted reusable content, active attempts, reader pins
and recovery holds. Retirement and pin acquisition serialize on exact guard records. Collection
rechecks reachability and full identity under guards; local manager policy delegates rather than
performing its own database deletion. Shared surviving content is never removed by attempt cleanup.
**Accepted lifecycle target, 2026-10-10 (ADR-0145); qualification open.** Native-control
version 4 captures issuance era before submission and retains original owner epochs through retry.
Permanent closed-through fencing, outcome references and native guards have distinct lifetimes.
Fresh retirement invocations claim exact incarnations, atomically nominate bounded outgoing-hold
pages and finalize only after an empty proof. Retiring objects reject both hold ends and reactivation.
Same-era resume retains authority; post-cut maintenance successors reconcile predecessors and claim
only durable remaining scope, preserving original cutoffs and completed pages. Indexed live-state
observation precedes drained-maintenance history collection. Referenced attempts, unresolved work,
necessary outcomes, guards and watermarks remain protected; age or a resolved flag is insufficient.
Compiler/audit selection follows one model closure declaration with native/dump adapters, while
actual stored rows and cold admission remain independent checks. An audit owns its read owner,
pin and session until admitted tails drain, including failed preparation and cancellation.
Logical content backups reference separately protected cold-service recovery
assets; recovery metadata is not authority to restore live clients or execute imported commands.

> Decision: ADR-0145, ADR-0088, ADR-0126

### §6.2 Readers

Compiler readers bind explicit immutable native completed views, including exact contribution
membership and frozen semantic boundaries. Pending values cannot enter selections or absence
answers. Compact projected streams and scoped typed indexes avoid repeated rich reconstruction.
Native reads preserve explicit nominal-key demand and exact immutable membership, and deduplicate
physical IDs before payload hydration. **Implemented / focused Tested, 2026-10-09 (ADR-0141); coordinated acceptance open:** verified
single-owner point reads, qualified scalar equality-prefix access and verified exact-view pointers
replace multi-owner Cartesian enumeration. Repeated broad demands can share charged compact
selection with independent cursors; sparse demand does not require whole-view startup. Actual
native plans must preserve bounded external ordering without relocating expansion into Union
distinct state. Recognized atomic
field selections use bounded indexed scope windows. Compact native scope constants are prepared
before index selection; inline value mapping/casts remain outside its predicate. General filters retain compact native selection. Explicit empty demands and unfiltered zero-row views retain authority, exact-view and
shape validation while avoiding a row query; arbitrary SQL retains normal execution semantics.
Compiler sessions carry attempt ownership rather than a published snapshot handle. Cancellation drains owned reads,
writes and provider workers before the attempt is removed; ambiguity fails that owned attempt.

**Implemented / focused native Tested, 2026-10-09; full cache equivalence open.** Optional product capture reads one
completed contribution through a dedicated checked singleton scan. The store reconstructs its
selection from the actual completed descriptor; capture registers no dependency view and changes
no canonical retained state. Public dependency/publication reads still require registered views.

Published readers use a trusted Rust boundary to pin exact published views and immutable executable definitions; database VIEWER credentials alone are not a capability for one publication. Functions, analyzers, index specs, engine identity and adopted module bytes
cannot change beneath a pinned handle. Streamed results remain provisional until terminal success.
A partial diagnostic subgraph reports its boundary and cannot claim global compiler completeness.

> Decision: ADR-0141, ADR-0145, ADR-0126

### §6.3 Schema evolution

**Implemented recovery refinement, 2026-10-10 (ADR-0146); integrated qualification open.**
An explicit installer replacement follows owned-daemon restart/drainage and immutable predecessor
inventory. Unpublished scopes require exact closed source markers and absent/intent-only native
journals; potentially translated unpublished scope refuses replacement. Before any publication,
a successor may select a corrected target with a new native operation. Once a scope is published,
the complete candidate target must remain identical. Only an exact closed current-generation target
marker, matching published native journal and durable completed-scope checkpoint allow that scope
to be preserved and skipped. A distinct immutable host plan/executable/operation retains the original
native migration operation for remaining intent-only scopes. It never refreshes native authority or
reruns proven completed work. Credentials, predecessor evidence and private diagnostics remain
protected; durable successor creation precedes maintenance-owner transfer. Ordinary attachment and
same-candidate retry cannot silently substitute an executable.

A distinct checked reconciliation successor may adopt recognized partially translated unpublished
state. Its immutable execution contract binds original native/credential operations, successor,
scope/generation, exact source/target and overlay/protocol/preflight/verifier identities. Actual
bounded reconciliation produces new successor progress, never fabricated predecessor receipts.
Unknown mixed forms, foreign identities, live predecessors and uncertain unfenced effects refuse.
Complete row assignments preserve protected legacy scope without relying on defaults to backfill.

One migration-only overlay record per original migration/database records atomic page effects and
progress, including preflight and nested cleanup cursors. Closed exclusive effects establish reuse
validity; relevant source/predicate changes invalidate preflight, and verifier changes invalidate
verification. The overlay has independently checked declarations and is outside runtime/content
identity because ordinary consumers do not use it. Privileged cold recovery includes it. Native
marker/journal/overlay sealing, executable-definition completion and host checkpoint are separate
completion boundaries. Named recovery consumers govern later exact overlay retirement.

The exact format3→4 target completes before the explicit format4→5 runtime transition that adds
bounded history-page receipts. One collector advancement commits one bounded page and revision;
expected-revision replay recovers exact removed/protected outcomes without per-item effect history.
Its typed declaration belongs in runtime schema identity. Original eras, horizons and cursors survive.

Index application may use execution-only concurrent construction without changing canonical schema
identity. Catalog presence alone does not establish usability: every desired index, including one
present on a retry, must report terminal readiness before installation advances. Unknown/error states
keep admission closed. The known3.3.0 executor timeout-stack defect requires a narrowly patched,
exactly identified server generation; batching alone is not a server correctness fix. Source, locked
build recipe, patch and executable provenance belong to the existing native executable owner; PSE
assets remain external and unchanged. Runtime limits and compiler/test parallelism remain unchanged.

**Implemented physical lowering / focused Tested, 2026-10-09.** Fixed top-level SCHEMAFULL fields, unique indexes and enforced role edges remain database obligations. Flexible semantic object bodies and supplied scope fields are produced by selected declaration-derived adapters, preserving missing/NULL, bytes/text, local sums, originals and aliases. Complete independent expected-row reconstruction rejects extra envelope/body/scope content and coherent corruption. Standalone arbitrary raw-SQL body rejection is deliberately lost; supported ingress and exposure remain fail-closed. Physical definition identity changes together with DDL and lowering; semantic artifact/contribution formats remain unchanged.

Typed declarations and explicit canonical codecs own contracts; Arrow IPC/container bytes and
engine coercions do not define semantic identity. Snapshot changes are explicit migrations;
codebooks remain append-only. Rebuild fresh from pinned inputs without old-format readers, legacy
IDs, dual writes or rollback/runtime archives. Quiesce actual readers before replacing their state.

> Decision: ADR-0146, ADR-0145, ADR-0087, ADR-0048

### §6.4 Serving realizations

**Implemented S2/S3, 2026-10-05; scoped functional evidence remains distinct from operator activation.** Serving uses the pinned native realization,
indexed restrictions and coarse connected queries/functions. Rust owns meaning, even where
SurrealQL is the only executor; Python remains a thin adapter. Complex kernels consume appropriate
batched inputs. Search eligibility precedes channel limits; exact analytical neighbors remain a
separate contract from approximate discovery. Resources and continuations pin complete realization.

> Decision: ADR-0145, ADR-0071, ADR-0131, ADR-0049

<a id="section-6-5"></a>

### §6.5 Services and capability adoption

**Accepted target / implementation in progress, 2026-10-09 (ADR-0145).** One stable local
SurrealDB3.3 RocksDB service serves main and validation. Host state and coordination live outside
checkouts. Install/check/status/maintenance distinguish observation from privileged mutation;
maintenance excludes borrowers and requires actual drainage plus identity revalidation. No fixed
memory, compiler-job or test-thread limits are selected. Embedding and optional portable-product
caches use distinct tables in the same configured database. Exact consumed vector winners remain
canonical content and cannot depend on mutable cache survival.

Native graph/documents/functions/search/bulk operations are selected. Query planning tools are
available for concrete uncertainties, not mandatory per-operation accounting or adoption proofs.
Custom APIs, buckets, reactive enrichment, connectors and external exporters require a named
consumer. Historical runs/operation records are retired rather than reconstructed as another service.
The [operator runbook](../../surrealdb.md) documents configuration, publication and explicit selection.
Operator reconstruction, live vectors and activation belong to separately authorized Q1 work.

> Decision: ADR-0145, ADR-0073, ADR-0131
