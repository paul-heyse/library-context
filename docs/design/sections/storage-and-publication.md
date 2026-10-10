# Storage and publication

**Accepted shared-content target / implementation in progress, 2026-10-09 (ADR-0143).**
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

> Decision: ADR-0143

**Implemented compiler and published native projections, 2026-10-05.** Named projection contracts specify their
completed source, semantic roles, contexts, vertex universe, direction, multiplicity, weights,
coverage, provenance and declared simplifications. Vertices are independent of edges; isolates and
parallel attributed assertions survive. Dense indices are private and map back to semantic IDs.
Compatible compiler consumers share compact topology; rich evidence is hydrated separately.

The accepted persisted target constructs these compiler views from exact completed native memberships, without a
publication handle. Indexed native selection and projected Arrow streams feed model-owned kernels. Summary, Structural and optional analytical consumers wait for their actual
semantic/vector predecessors. Published export uses scoped native queries and terminally successful
responses under the same projection contract. External destinations remain consumer-triggered.

> Decision: ADR-0143, ADR-0103, ADR-0044, ADR-0085

## §6 Persistence and publication

**Accepted target / implementation in progress, 2026-10-09 (ADR-0143).** One managed service
hosts `main` and `validation`. Explicit maintenance installs compatible schemas and credentials;
ordinary clients only attach. Tests use logical attempts and publications in validation, with no
scratch service or per-test database fallback. Compiler, store, publisher and serving retain their
existing semantic responsibilities. Patched gRPC preserves progressive streaming, cancellation
and checked physical terminality. Optional transports must preserve those contracts.

> Decision: ADR-0143

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
installation epoch per page. Ordinary open-attempt and pin authorization remain separate.

A short guarded transition publishes an immutable manifest over admitted views and one executable
definition epoch. Handles bind semantic/realization identities, publication, exact view set and
service generation. Definitions are installed under immutable epoch names. Ordinary publication
cannot install DDL, mutate unrelated content or replace another publication's functions. Selection
remains explicit. Later cleanup failure preserves an acknowledged committed publication.

Logical backup uses the native engine's single whole-main read transaction and the requested
publication's reader pin. A completed engine dump is grammar-decoded once and compacted to the
requested manifest's complete dependency/payload/original closure before atomic logical-output
publication. Shared preparation describes claimed input, not admission; every restore freshly
prepares and independently admits its own data. Physical engine export still transfers/spools
whole-database table data, an explicit growth cost of this initial route.
RocksDB snapshot reads retain concurrently retired rows; unrelated retirement need not wait for
logical export. Protected cold service recovery retains exclusive
maintenance and drainage. Restore treats its input as data, never as
Root-authorized SurrealQL: closed literal data is lowered through typed staging and independent
admission. Definition metadata selects trusted executable generation. Grammar-owned comparison
normalizes only declaration application modifiers (`OVERWRITE`/`IF NOT EXISTS`); schema, bodies,
permissions and other attributes remain exact, and imported definitions never execute.
Imported runtime control
records cannot restore grants, attempts, fences, pins or visibility. An explicitly selected manifest
chooses the recovery closure; ambiguous multi-publication convenience restore refuses.

Native reachability owns publication roots, admitted reusable content, active attempts, reader pins
and recovery holds. Retirement and pin acquisition serialize on exact guard records. Collection
rechecks reachability and full identity under guards; local manager policy delegates rather than
performing its own database deletion. Shared surviving content is never removed by attempt cleanup.
Completed contributors retain exact prerequisite views as well as their output payloads. Durable
attempt/effect/pin authorization epochs and permanent retired-through watermarks permit a later
valid lifecycle to recreate identical content while excluding delayed earlier operations, even
after reactivation. Logical content backups reference separately protected cold-service recovery
assets; recovery metadata is not authority to restore live clients or execute imported commands.

> Decision: ADR-0143, ADR-0088, ADR-0126

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

> Decision: ADR-0141, ADR-0143, ADR-0126

### §6.3 Schema evolution

**Implemented physical lowering / focused Tested, 2026-10-09.** Fixed top-level SCHEMAFULL fields, unique indexes and enforced role edges remain database obligations. Flexible semantic object bodies and supplied scope fields are produced by selected declaration-derived adapters, preserving missing/NULL, bytes/text, local sums, originals and aliases. Complete independent expected-row reconstruction rejects extra envelope/body/scope content and coherent corruption. Standalone arbitrary raw-SQL body rejection is deliberately lost; supported ingress and exposure remain fail-closed. Physical definition identity changes together with DDL and lowering; semantic artifact/contribution formats remain unchanged.

Typed declarations and explicit canonical codecs own contracts; Arrow IPC/container bytes and
engine coercions do not define semantic identity. Snapshot changes are explicit migrations;
codebooks remain append-only. Rebuild fresh from pinned inputs without old-format readers, legacy
IDs, dual writes or rollback/runtime archives. Quiesce actual readers before replacing their state.

> Decision: ADR-0143, ADR-0087, ADR-0048

### §6.4 Serving realizations

**Implemented S2/S3, 2026-10-05; scoped functional evidence remains distinct from operator activation.** Serving uses the pinned native realization,
indexed restrictions and coarse connected queries/functions. Rust owns meaning, even where
SurrealQL is the only executor; Python remains a thin adapter. Complex kernels consume appropriate
batched inputs. Search eligibility precedes channel limits; exact analytical neighbors remain a
separate contract from approximate discovery. Resources and continuations pin complete realization.

> Decision: ADR-0143, ADR-0071, ADR-0131, ADR-0049

<a id="section-6-5"></a>

### §6.5 Services and capability adoption

**Accepted target / implementation in progress, 2026-10-09 (ADR-0143).** One stable local
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

> Decision: ADR-0143, ADR-0073, ADR-0131
