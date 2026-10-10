# Graph compilation, persisted execution and selective reuse

**Unified persistence target, Proposed, 2026-10-09:** the
[unified companion](unified-persistent-surrealdb-plan_2026-10-09.md) develops operator-accepted
RC01–RC07 and UP0–UP9. One durable service, shared immutable payloads/exact views, checked
attachment, manifest publication and durable effect/pin/retirement replace conflicting open
private-database, mandatory-replay and database-wide-reader prescriptions. Existing code and
dated receipts below remain the implemented baseline, not acceptance of this replacement.
§7.3 coordinates the new target; §8 owns its source findings; §9.1 retains all actual receipts.

Command excerpts identify verification scope; original invocations remain in the cited logs and Git history.

**Implemented, 2026-10-09:** persisted completion/publication and populated correction foundations,
plus GK0–GK6 and GR0–GR5 program/product/dependency/viewer integration. ADR-0138–ADR-0140
own the accepted execution boundaries; focused receipts and remaining acceptance are in §9.1.
No performance measurement or whole-plan qualification is claimed.
The [compiler companion](graph-compilation-kernels-and-hashing-plan_2026-10-09.md) and
[reuse companion](graph-compilation-reuse-and-invalidation-plan_2026-10-09.md) replace conflicting
open per-root/SQL-only and dependency-foundations-only routes. Persistent products, complete
selected-domain reuse, dependency metadata and viewer caching are integrated; GK7/GR6 final
qualification, BC3 and failed full-compiler equivalence controls remain open.
This coordinator remains the sole scheduled-finding and integrated-acceptance owner; earlier
receipts retain their original source boundaries and surviving obligations.
This document coordinates the next hard design pivot. Its source is the
[catalog compilation speed review](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md),
especially F01–F05 and RC01/RC02, together with the operator's 2026-10-07 selection of persisted
graph compilation and comparable corrections throughout the codebase. The operator confirmed
direct sealing, internal contribution/view identity, and dependency foundations on 2026-10-07.
The 2026-10-09 accepted reuse horizon below supersedes that choice's cross-run deferral. Native
persistence and publication without ordinary self-export/import remain foundations. UP0–UP9
replace database sealing with exact manifest publication; no second canonical store is restored.
The earlier review's diagnoses and preservation obligations remain applicable.

The [correction-causes review](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md) subsequently identified incomplete admission/completion contracts and native execution amplification. Its [supporting correction plan](persisted-execution-corrections-plan_2026-10-07.md) defines PC0–PC6, integrated below. Both correction-causes RC01/RC02 were operator-accepted on2026-10-07. They are different from the catalog-speed rule impacts above.

This plan owns the sole scheduled disposition of its source reviews and the additional source-inspected inefficiencies in §8.
The existing graph-native and evidence/evaluation coordinators retain their original findings and
dated receipts. They do not close the new findings. The architectural collection owns enduring
contracts; PG0 changes those contracts before dependent implementation. The operator resumed PC0–PC6 production work on 2026-10-08; ADR-0138 now consolidates its accepted target. The targeted acceptance boundary in §9 remains authoritative. Operator adoption remains held; the interrupted pilot has no successful verdict.

## 1. Outcome, baseline and boundaries

Persist captured inputs, completed typed products and their dependency relationships in the
managed local SurrealDB graph during compilation. Use indexed set-shaped access, graph adjacency
and suitable native computation over those inputs; retain model-owned Rust kernels and bulk
Arrow/DataFusion computation where they fit the actual operation. Compile and admit exact views
in shared immutable native content, then publish their manifest without sealing the whole database.
Ordinary publication must not export, reconstruct and re-ingest the compiler's own output.

The stored option supplies useful graph access and exact dependency foundations. Selective
invalidation and reuse are now scheduled through GR0–GR6. Persistence alone does not remove repeated work.
The target also replaces the identified variants with common access, conversion, batching,
preparation and validity mechanisms. Domain policies remain with their semantic owners.

Original plan baseline: main `ccbcae1f06610d00564d8b74078ae642a5c548e9`, 2026-10-07. Open edits in
AGENTS, STATUS, DESIGN, existing plans/runbook and a test docstring were preserved. Production
sources remain the review's `9fecc014` baseline. The existing implementation already persists its
final graph; the changed boundary is compilation itself and reuse of its exact completed state.
Native serving, publication, embedding/cache, retrieval and the primary programmatic evaluator
have the focused receipts recorded by their coordinators, not new acceptance from this plan.

Correction authoring inspected `43cd6501` plus the four unchanged dirty files fingerprinted by correction-causes review §10. PC1–PC6 develop the integrated code, not a return to the original pre-pivot baseline. The existing exact-view, point-read, typed-port, S0 and nine-premise admission improvements remain foundations; final cold admission and native journeys remain open.

The real Catalog attempt stopped after65m31s with SIGINT/exit130 and no verdict. A brief stack
sample did not identify its query/stage or dominant cost. Static findings justify removing known
work without a latency estimate or mandatory benchmark. The proposed speed and reuse benefits
are unmeasured. Rust build time, GPU/model upgrades and a performance campaign are not this scope.

The [Rust compilation-cost plan](rust-compilation-costs-plan_2026-10-08.md) separately owns its source-review F01–F04 and BC0–BC5. BC1 consumes current PC1 completion and PC2 captured-binding contracts, including charged cancellation/drain ownership; BC2 scopes provider provenance and records workspace execution composition without replacing cold supplier semantics. Those compiler corrections can proceed before unrelated PC6 acceptance, but touched native/CLI/cold controls need final-source evidence. BC5 and PC6 may share a valid optimized receipt; this coordinator retains all PG/PC findings, restored-coverage and native/MCP obligations. Compilation-plan authoring neither resumes tests nor closes them.

Its [coroutine companion](rust-coroutine-compilation-amplification-plan_2026-10-08.md) develops
structural and confirmed recurring loader/producer containment within BC1. CU1–CU4 consume the
integrated PC completion/binding/selected-closure contracts; they preserve stage policy, charged
lifetimes and native terminal drainage. They need not wait for unrelated PC6 journeys, but any
touched PC contract requires revealing final-source controls. CU6/BC5 and PC6 may share matching
optimized receipts without duplicating execution; local/candidate or older-tree evidence does not
replace PC6 acceptance. Coroutine F01 disposition remains solely with the compilation coordinator.

Migrate all affected compiler frontiers (`facts`, `normalized`, `analysis`, `catalog`), Catalog
and Behavioral profiles, shared projection/analysis/admission consumers, and the non-compiler
paths in §2. Behavioral functionality is preserved, not expanded. Cross-run invalidation and
pure-product reuse are included; background automatic recompilation, resumable attempts,
distributed execution, compatibility readers,
historical runtime archives and a second storage backend are not introduced. Protected evaluation
populations remain sealed; the primary programmatic and outer agentic roles remain unchanged.

## 2. Foundation assessment and codebase coverage

Assessment uses core3.3, heuristics1.0, code-intelligence1.5 and the repository binding. Ownership
and exact semantic contracts are useful foundations. The deficient composition is repeated broad
physical work beneath scoped operations, eager preparation and repeated boundary conversion.
The durable graph is an execution substrate, not a new authority over program meaning.

| Consumer family and actual entry points | Source-inspected assessment | Go-forward route / disposition |
|---|---|---|
| Acquisition/extraction: captured receipts, `facts`, native adapters | Pinned captures and attributed bounded outputs are retained. Their ordering/coverage has real consumers. | PG3 moves the handoff to bounded native writes; no provider algorithm or blanket parallelism change. |
| Normalization and catalog: `normalize`, C0/C1/C2/S0/E0 | Review F01–F03: rich IPC scans per grain, prefix rebuilding and repeated lower admission. Callable scopes repeat forward edge requests; E0 overlapping windows repeat physical chunk reads. | PG1–PG4: exact native views, indexed/batched selection, physical-demand deduplication and carried validity. |
| Compiler projections/analysis: `analysis_graphs`, `analytical_scopes`, `analytic`, `analytic_text`, `structural` | Prepared topology/nominal adjacency is already shared. `PreparedGraphs::load` repeatedly filters assessment/snapshot/chunk collections; F04 prepares disabled A1 inputs. | PG4: method-demand union, shared indexed selected topology and explicit NotRequested outcomes. |
| Behavioral execution/summaries: `semantic_execution`, `semantic_summaries`, `scoped_execution`, `scoped_aspects` | Owner-grained closures and global SCC/proof universes have semantic purposes. They consume the changing workspace contract. | PG3/PG4 migrate every actual input/admission path; do not infer that required whole-universe work is waste. |
| Serving/retrieval: `scope`, `source_evidence`, `source_usage`, `source_characterization`, `behavior`, defaults | One keyed union closure and coarse canonical hydration already exist. Cached typed families are repeatedly scanned by `read_ids`/`read_for`; source-usage calls are nested. | PG6 adopts shared request-local typed ID/reference indexes and batched selection; no claim of one RPC per current helper call. |
| Native loading/reconciliation: `codec`, `loader`, `reconciliation` | Canonical/role inserts are bulk. F05 external endpoints are scalar. `codec::view` invokes `R::encode` on one row for every native body; loader and reconciliation both consume it. | PG2/PG5 share bounded batch body conversion and external endpoint batches, preserving independent actual-state reconciliation. |
| Backup/restore/projection export: `backup`, `projections`, `inspection` | Backup already uses checked gRPC export and staged-file publication. Restore original transfer uses serial64KiB reads despite an existing verified batch-range API. | PG5/PG7 preserve completed state and adopt batched byte transfer. Endpoint-before-edge passes remain necessary. |
| Embedding/cache/search: `embedding_realization`, `cache`, `operations`, `materialization`, publisher search | Exact-input deduplication, batched inference/cache admission, native scoped search and winner reconciliation already exist. | PG4/PG6 reuse them. Coordinate selected demand and physical input batches without a second cache or incidental ranking change. |
| Rust/PyO3/MCP transport: `service`, model `serving::dispatch`, `delivery` | Normal and retained service paths count JSON bytes, then serialize the same response again. Exact envelope sizing and delivery stabilization have separate purposes. | PG8 replaces repeated final-shape encoding with one bounded encoder; preserves necessary changed-shape delivery passes. |
| Primary evaluation: `lctx-eval`, MCP observation and programmatic runner | Finite case/segment/witness populations and independent expectations are deliberate. No repeated native RPC defect established. | PG6 migrates altered typed/native contracts and shared selected access where actually consumed. Keep finite evaluation algorithms unless a concrete amplification remains. |
| CLI/admin: compile/import, snapshot query/list/show/audit/select/retire, store readiness | Whole-namespace inventory, explicit cold audit and selection checks serve requested guarantees. | PG5/PG7/PG9 migrate completed-state and lifecycle contracts; retain required full scans, indexed search and existing read-only queries. |

These observations are static, not measured hotspots. New confirmed corrections are scoped in §8;
the table also records where an analogous defect was not established. During execution, expand
the affected-caller map when a shared change reveals another consumer. Closure requires migration
of every actual caller, not just the representative entry point or a favorable text search.

Decisive sources: [workspace](../../crates/cpg-core/src/workspace.rs), [consumed rows](../../crates/cpg-core/src/consumed_rows.rs),
[native codec](../../crates/lctx-surrealdb/src/codec.rs), [serving evidence](../../crates/lctx-serving/src/source_evidence.rs),
[source usage](../../crates/lctx-serving/src/source_usage.rs), [restore](../../crates/lctx-publisher/src/backup.rs),
[service](../../crates/lctx-serving/src/service.rs) and [response dispatch](../../crates/lctx-model/src/domain/serving/dispatch.rs).
The two direct codec writer consumers are loader entities/assertions; reconciliation regenerates
expected bodies independently. Programmatic evaluation remains an adjacent contract consumer.

## 3. Settled choices and architectural rule changes

| Item | Operator decision, 2026-10-07 | Required route before dependent work |
|---|---|---|
| Catalog-speed RC01 | **Accepted:** ordinary compilation seals its own admitted persisted realization; external imports and stored reconciliation remain independent exposures. | PG0 supersedes ADR-0128's portable-roundtrip/post-compile-only publication clauses and updates storage §6.1. |
| Catalog-speed RC02 | **Accepted:** exact immutable contribution/view identity replaces eager whole-prefix content identity internally. Deterministic final content identity remains. | PG0/PG1 revise `SourceSnapshot`, completed-input bindings, manifests, observation/cache identities and their consumers. Do not relabel membership as canonical content. |
| Native compiler placement | **Selected explicitly:** store-free compilation is displaced. | PG0 replaces DESIGN §B3/§B7/§B12, semantic-model §15.1/§15.11, storage §5/§6/§6.2 and associated instructions. Restate surviving ADR-0128 obligations in the replacement ADR. |
| Reuse horizon | **Superseded, 2026-10-09:** exact persisted dependencies now feed selective cross-run pure-product reuse and dependency propagation. | GR0–GR6 replace the earlier foundations-only deferral, retaining independent current ownership/admission. |
| [Correction-causes RC01](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#rc01) | **Accepted:** table-specific scope inventories and scalar projections only for actual consumers. | PC0 complementary decision/owner updates; PC3 physical schema, reconciliation and realization cutover. |
| [Correction-causes RC02](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#rc02) | **Accepted:** later independent units may execute within a failed private restore request. | PC0 records the changed failure boundary; PC1 preserves complete outcomes; PC5 batches units and abandons failed staging. |
| [Graph/hash RC01](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#RC01) | **Accepted, 2026-10-09:** model-owned relation operations may lower to graph kernels, indexed native queries or relational execution. | GK0/GK1 replace §15.10's topology-only/SQL-only restriction and duplicated scope interpretations; §B3 remains compatible. |
| [Graph/hash RC02](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#RC02) | **Accepted, 2026-10-09:** selective cross-run reuse is a current target. | GK0/GR0 route complementary ADR/owner changes for §15.1 and §14.2 before dependent implementation. Native canonical authority and independent admission remain. |

Plan approval is not implementation or finding closure. PG0 has installed the selected target through ADR-0138; implementation and verification of its dependent packages remain in progress.
The new ADR must preserve graph identity/role fidelity, programmatic assertions, native query
selection, explicit publication versus selection, pinned readers, exact values/projections,
append-only codebooks and rebuilding from pinned inputs. Retire only its replaced execution rules.

## 4. Persisted graph and exact completed inputs

### 4.1 One database, one payload owner

Use one managed authenticated SurrealDB3.3 RocksDB service and remote Rust gRPC SDK.
The [unified target §3–§4](unified-persistent-surrealdb-plan_2026-10-09.md#3-one-service-explicit-attachment-and-maintenance)
replaces the implemented private-database lifecycle with stable main/validation responsibilities,
immutable payloads/nominal anchors and exact view-relative resolution. `lctx-surrealdb` owns
mechanical schemas/codecs/access/write finality; `cpg-core` owns
orchestration and pure operation adapters; `lctx-model` owns records, operations, dependencies
and validation. `lctx-publisher` owns external visibility and import/restore. Dependency direction
is `cpg-core → lctx-surrealdb → lctx-model`, with no reverse core dependency in native storage.

Use canonical-first storage for records already covered by the model's graph entity/assertion
registries: mechanically lower them once into the existing native families and retain the original
typed nominal key/type alongside the canonical graph target. Their generated native body supplies
queryable fields; canonical bytes supply exact export and independent reconciliation. These are
derived execution forms of one immutable value, not separately editable facts.

Extend the batch codec and closed body schema to preserve **every declared intermediate field**,
including opaque binary as native bytes. The current serving body deliberately omits those fields
and is insufficient for compiler inputs. Decode integers at their declared widths, IDs/digests at
their exact widths, finite floats, null/list/sum arms and logical UTF8Text correctly. Native
SCHEMAFULL/coercions do not replace semantic shape checks. Derive atomic index keys mechanically;
an array-element byte index cannot establish whole nominal-ID equality.

Actual completed non-graph records use one fixed generated `compiler_record` family, not one
table per kind and not a duplicate of registered graph payloads. Its inventory derives from
declared catalog relations minus graph entity/assertion declarations; writing, schema, scanning
and cold validation use that same classification. The current inventory has88 families, including
projection snapshot headers/chunks and QualityStep. Originals/`ArtifactChunk` retain the bounded
original-chunk byte owner. Cold validation groups bounded native rows by relation, invokes the
model canonicalization and shared batch body codec once per type slice, and compares regenerated
canonical bytes exactly; native numeric Value equality is insufficient for that comparison.
Undeclared required records fail registration instead of disappearing through dispatch.

Preserve frontier-dependent intrinsic graph lowering: normalized-or-higher Place endpoint aliases
are derived only when that frontier permits them. Facts do not gain normalized entities early.
Canonical family membership is explicit and excludes pending writes/operational controls even
if those rows inhabit the same private database. Retained non-graph values and state have their
own declared family; they are not counted as program entities or semantic assertions.

### 4.2 Contributions, views and retained dependencies

Declare neutral completed-state contracts in `lctx-model`; mechanically realize them natively.
They describe existing completed products, not a historical event/receipt ledger.

| Contract | Required meaning and physical realization |
|---|---|
| Typed row key | Relation plus complete nominal semantic key; native payload address is a separate model/codec/content mapping. Same key/different payload in one view is a conflict; different views may select different revisions. Equal canonical payloads share. Hash equality alone never authorizes conflicting payloads. |
| Producer contribution | Stable producer/operation identity, profile, model/record contract, implementation/settings, exact predecessor views, declared outputs and actual outcome. Compact membership links select its emitted rows. |
| Completed view | Exact immutable set of completed contribution identities for each relation and publication boundary; overlapping memberships deduplicate nominal rows. Later contributions cannot widen it. |
| Dependency | Product-to-input view/selection and answer-affecting model/configuration/implementation/specification/original dependencies. Absence-sensitive reads bind the membership universe, not merely the rows returned. |
| Completed state | Completed descriptors, memberships, dependencies and retained non-graph backing values. Its own deterministic digest is referenced by the durable manifest. |

Physical contribution IDs and membership IDs are deterministic under producer identity and full
typed keys; process clock, database name, batch boundaries and delivery order are not logical
identity. A contribution computes its own new-content digest once from ordered compact keys and
payload digests, with exact duplicate/conflict checks. Views bind contribution descriptors; they
do not copy or re-hash a growing rich union. Rows/counts refer to deduplicated actual membership,
not the sum of overlapping contribution counts. Keep final canonical family hashing separate.

Persist coarse declared dependency views where that is the actual dependency. For owner-grained
products whose relevant selection is already known, retain that selection and membership/absence
basis. Do not fabricate precise row dependency claims from incomplete instrumentation or add a
dependency edge for every primitive read. Conservative complete dependencies are correct future
invalidation foundations; their granularity is explicit.

PC2 adds declared producer/supplier semantic contracts and exact captured role/configuration bindings inside ContributionSpec. Cold admission no longer constructs current executable prototypes. The spec digest domain becomes v2, completed-state format2/content-domain v2 and artifact format3; exact contribution/view identities carry the new bindings. Old metadata is refused before reconstruction and rebuilt from pinned inputs. The nine-view/profile admitted-facts cache and reporting product remain shared, including declared NotRequested ownership. [Correction plan §3.2](persisted-execution-corrections-plan_2026-10-07.md#32-captured-suppliers-bind-a-declared-semantic-contract--f02) owns the contract and migration scope.

Retain completed-state metadata and its needed backing values with the snapshot. Current consumers
are exact compiler view access, dependency inspection through snapshot show/read-only query, and
portable export/backup/restore. Future selective invalidation is an additional intended consumer,
not a reason to retain pending writes, temp sort runs, event history or obsolete workspaces.
Graph-family identity and completed-state identity remain distinct; the aggregate artifact/handle
identity deliberately includes both. This is a manifest/schema migration, not an unchanged old hash.

### 4.3 Completion is a semantic gate, not a long transaction

The attempt has one completion owner. Producers bind exact predecessor views before execution.
They shape-check and bulk-write bounded values/memberships into pending contributions. New and
existing IDs are compared in bounded sets; identical payloads deduplicate and different payloads
fail. Keep the uniqueness/admission constraints active. No blanket `INSERT IGNORE` for semantic
rows, and no last-writer-wins replacement.

Inputs see only selected completed contributions. Pending output does not enter ordinary indexed
seed queries, closure traversal, negative membership answers or global scans. Completion waits
for every declared output, including known-empty, every input-stream terminal, all output writes,
provider outcome handling and cancellation/task drainage. A short owned transition freezes the
descriptors and exposes the contribution. Partial/Unavailable/NotRequested remain explicit under
the existing stage availability policy; successful transport does not upgrade those outcomes.

Forward/cyclic data references may remain typed pending links while endpoints are incomplete.
Do not manufacture external placeholders for internal keys. Create enforced native role edges
once the required endpoints exist; unresolved external targets retain their actual typed meaning.
Producer completion is not final reference closure. Final admission must establish required
endpoint kinds/subtypes, all applicable semantics and the separate derivation-DAG obligation.
Legal data cycles do not legalize proof cycles.

Keep transactions around bounded writes/completion controls only. Extraction, streaming reads,
CPU kernels, inference and external file writes happen outside them. Preserve durable Every sync,
terminal SDK checks and writer ownership. A known aborted transaction may retry its bounded write
under unchanged premises; unknown write/COMMIT acknowledgment is reconciled through its durable
operation identity before retry or cleanup. [UP3](unified-persistent-surrealdb-plan_2026-10-09.md#5-attempts-checked-reuse-and-durable-recovery)
retains completed prerequisites and protects unresolved staging/effects; it introduces neither
arbitrary pending resume nor whole-database abandonment. No sweep touches unrelated operator data.
PC1 completes this behavior with one model-owned outcome composition: primary cause, every material finalization failure, local terminality, remote-effect certainty and private-resource disposition remain distinct. Preserve already sealed/persisted effects when later cleanup fails; no error implies rollback. Construction, compile/inspection, publication/import, backup and restore siblings all consume it. The [supporting contract](persisted-execution-corrections-plan_2026-10-07.md#31-one-complete-operation-outcome--f01) owns the details.

**Implemented / targeted verification in progress, 2026-10-07:** compiler, loader and retained-state
import share native row/byte windows for candidate comparisons and checked insertion. Indexed
write transactions target128 rows while logical Arrow transfers retain4096; existing byte and
maximum-single-row bounds remain. An admitted larger row travels alone. Metadata replay compares
complete immutable rows and inserts only missing values; no per-row UPSERT or blanket IGNORE.
Completion owns visibility after every window, so transaction boundaries do not redefine identity.
Current overlapping writes are source-ordered; a future concurrent-overlap replay-success contract
must address that ownership explicitly. This is an execution policy, not a speed measurement.

## 5. Shared operation mechanisms and efficient placement

### 5.1 Native access and request-local indexing

**Replacement scope contract, Proposed, 2026-10-09:** GK1/GK2 govern roots, body/context/coverage
membership and negative domains once in the model, then compile shared root partitions to these
native providers or compact Rust kernels. The access/index capabilities below remain useful;
separately authored per-consumer scope SQL is retired during GK3–GK5. Native selection is a
lowering of meaning, not the owner of meaning. GR1–GR5 add qualified program/product/pin reuse.

Extend the existing native reader/loader capabilities rather than add a backend-neutral provider
framework. Internal operation context binds either an exact private completed view or a sealed
read-only snapshot, plus demand, budget, cancellation and lifetime. A private input view is not a
published `SnapshotHandle`; do not forge a handle or bypass its publication-marker check.

Selections express finite typed ID sets, declared reference fields/direction, required columns,
source ranges or a model-owned set operation. Private reads include completed-view membership;
sealed reads include snapshot and semantic-family membership. Domain owners choose closure and
coverage. Values remain SDK bindings; arbitrary request-controlled SQL is not introduced.

PC3 makes planner-sensitive preparation, result-statement positions and terminal inventory part of one native lowering used by compiler/provider, published reader, projection and serving-function consumers. It co-designs scope layout with those predicates: derive each backing table's relevant fields, retain graph scope_context and independently owned search fields, and retire irrelevant scalar copies. DDL, construction, reconciliation and realization identity migrate together; [correction plan §4](persisted-execution-corrections-plan_2026-10-07.md#4-one-native-demand-interpretation-and-bounded-candidate-execution) supplies the selected route.

Native indexes reach selected memberships/keys/roles before rich decoding. Use a shared batched
one-hop primitive and full relation-qualified visited keys for exact multi-hop closure. Preserve
inverse ownership separately from forward dependencies, reconvergence, isolates, parallel roles
and positions. Normalize duplicate physical edge requests before preparation. SurrealDB3.3
`+collect`'s hash-only visitation is not the exact closure implementation.

When a Rust kernel repeatedly accesses one already scoped set, construct a typed ID index once
and lazily construct field/target posting indexes for fields actually queried. Scope them to the
exact view/request and charge their lifetime to the existing budget. Select row positions/references
first; clone/pack only the demanded output. `NativePackets`, source usage/characterization,
behavior packets and selected topology use this one prepared-row mechanism. An index cannot
silently replace a required full domain or merge distinct relationship occurrences.

### 5.2 Native sets and a bounded Arrow bridge

Execute indexed selection, one-hop adjacency, eligible finite filtering/grouping/aggregation and
native search beside persisted data where the owned contract is preserved. Complex resolution,
BDD/transfer/SCC/proof/analytic operations remain model-owned kernels. Ordinary finite operations
must not become a loop of scalar remote reads just because a native backend was selected.

For necessary DataFusion joins/sorts, stream **projected native field values** into generated Arrow
builders once. Retain the existing spillable runtime and common ResourceBudget. Share a projected
scan or bounded spillable transfer for the same immutable view among actual consumers; carry
known row counts/statistics to DataFusion. Do not reconstruct all rich canonical Entity/Assertion
objects before Arrow, retain a whole-store Vec of batches, or stage a new full IPC relation for
every selected grain. Scratch spill/columnar working data is derived and expires after last use.

The pinned remote SDK does not expose a usable Arrow-payload path; native-to-Arrow means one
field conversion, not zero-copy IPC. Use existing terminal-aware `NativeRows`; rows are provisional
until statement and outer transport completion. The current candidate-array route is replaced by PC4: single-equality scope branches (one active stream), or exact contributor membership streams, feed compact native external ordering and adjacent deduplication. Bounded exact membership point reads precede late projected hydration and residual filtering. Reuse the native external-run kernel rather than introduce another DataFusion runtime behind generic native reads. Canonical physical-ID order/aliases and typed nominal-key order retain distinct adapters. No whole-match LET array, union-index seen set or pre-intersection LIMIT establishes this correction. Compact constant preparation and every statement/error terminal remain checked. PC4 also delegates safe DataFusion fetch without claiming it cancels earlier preparation. Dependent outputs remain private and are discarded
on late failure. Server buffers, sort/group work and frame allocation are separate from client
batch bounds; use selective field width and the actual server envelope rather than claim global
memory safety from a small output limit.

### 5.3 Batching, originals, codecs and demand

Reuse declared TransferLimits and existing bounded array-insert helpers. Group a native codec
window by record type, call `R::encode` once for each actual type slice, extract aligned bodies
and restore source order. Loader and reconciliation use the same mechanical conversion but
reconciliation derives expected values from actual stored canonical bytes. Preserve source-less
assertion arms, nullable/sum fields, opaque/textual binary distinctions and exact payload equality.

Batch external endpoint identities/payloads before enforced edges. Preserve terminal failures,
full target keys, role multiplicity and same-key payload conflicts. Keep endpoint-first and
edge-second passes where forward references genuinely require them; no enormous transaction.

Use one shared source-range access path: union physical `(artifact,ordinal)` demand, fetch/decode
each chunk once per bounded scope, verify it, then deliver every original ordered logical range.
E0 provenance ranges stay distinct. Restore uses the existing verified32-range/256KiB route
instead of serial64KiB calls; stream complete originals into admission without retaining all bytes.
Original header reads join the same bounded preparation where useful.

Resolve selected method/effect dependencies before preparation. All-disabled A1 produces total
owned NotRequested outcomes and necessary frame provenance without kernel-only hydration/indexes.
A0's required call/definition topology remains. Selected methods prepare their actual union once.
Reuse exact-input inference/cache batching and winning values; query-only policy changes do not
invalidate document values. Embedding cache ownership remains distinct. GR2–GR4 add qualified
pure compiler products; no GPU upgrade or duplicate vector cache is scheduled.

For S0, retain a compact spool of completed documentary findings/support and emitter keys after
its first necessary pass; the second phase uses that output plus later embedding/literal results.
Do not retain all rich inputs merely to avoid recomputation. The spool is bounded/spillable and
attempt-owned, preserving global requests and exact support while retiring the second documentary
rebuild. Selected graph snapshots/chunks use the same indexed preparation instead of per-assessment
full collection scans; graph chunk order and validation remain unchanged.

### 5.4 Validity and final response encoding

Carry internal checked properties by their exact dependency descriptor and owned lifetime.
Admission still invokes authoritative validators. At upper completion run newly applicable or
changed-premise checks, new reference obligations and final frontier obligations. Extend shared
target/reference views as needed rather than reconstruct unchanged lower premises. A changed
shared membership or policy invalidates every check that actually depends on it; merely matching
relation names does not preserve validity.

External artifact import, retained-state decoding and persisted realization reconciliation remain
genuine boundaries. Counts, digests and a completed flag are not semantic admission. Independent
stored checks still compare canonical payloads, derived bodies, role mappings, originals and
completed-state references. No certificate service, replay ledger or producer re-execution.

Provide one bounded response encoder after delivery/cursor fields have their final values. It
grows its existing-budget reservation before allocating each bounded buffer expansion, rejects
the exact output limit during writing, and returns the charged encoded bytes for transport.
Normal and retained-response paths reuse those bytes instead of `json_len` followed by `to_json`.
Do not reserve the entire configured maximum eagerly. Each genuinely distinct wire shape (raw
response versus MCP envelope) still needs its own encoding; deliberate delivery stabilization
after changed content is not the same repeated-final-shape defect. Preserve exact serialized
delivery maps, escaping, deadline checks, retained-result admission and refusal/error meanings.

## 6. Publication, transport and public behavior

After final semantic admission, compute deterministic graph family content once using ordered
full keys and exact conflict/dedup rules. Compute completed-state content independently and bind
both in the manifest. This scans necessary final content but never reconstructs every preceding
prefix. Original bytes and consumed embedding values retain their complete ownership and policy.

Install identity/enforcement and compiler lookup indexes before their first use. UP5 replaces
the private-load index/seal lifecycle with immutable named definition epochs and per-manifest
readiness. Shared search must qualify exact-view eligibility before limits, including actual
approximate candidate behavior; an explicit exact eligible-vector policy is the initial safe
route where that behavior is unqualified. Scope indexes stay available early. Do not postpone
uniqueness constraints or imply index construction is free.

Manifest publication accepts owned completed native authority, not a public caller-supplied marker.
Its exact read set and producing effects freeze/drain; final semantic admission, stored content/state
reconciliation, search readiness and executable-definition inventory succeed before a short
guarded visibility transition emits an **unselected** handle. Other attempts can continue writing.
Internal VIEWER credentials are not a raw snapshot capability. UP5 migrates publisher, reader,
CLI/PyO3/MCP/evaluator and every evidence/cursor route together. Ordinary compilation performs
no portable export/re-import or final bulk database re-ingestion. Selection and pin-safe retirement
stay explicit. No whole extraction/compute/inference transaction or mutable selected snapshot.

| Interface | Target behavior |
|---|---|
| `lctx compile … --through … --profile …` | Same cumulative frontiers/profiles; explicit runtime readiness; outputs an unselected immutable published manifest over shared content. |
| `--artifact-only --output DIR` | Native compiler produces a complete portable export without selection. No store-free fallback. Attempt-scoped staging/pins release only after actual terminality; retained shared content follows native policy. |
| `publish-artifact` | Untrusted logical import through stable validation, then independent semantic/reference/state admission into shared main content. No private database or old-format reader. |
| Native compiler API | Exact completed-view read capability and bounded producer-write capability; pure model operations still accept explicit values without a server. Ordinary publication consumes owned completed-native state. |
| Snapshot show/query/audit | Show graph and completed-state identities and summaries; read-only queries inspect dependency/product membership. Cold audit checks actual stored state, not an internal compiler replay. |
| Backup/restore | Include graph/original content, compact completed-state family and necessary non-graph backing values. Restore checks exact membership/dependencies and rebuilds derived definitions/indexes. No missing state silently inferred from final graph. |
| MCP/PyO3/evaluator | Existing product tools and independent evaluation meanings remain; changed manifest/native contexts and bounded serialization migrate together. No consumer-controlled generic query contract is added. |

PC5 aggregates grammar-owned whole import units into sequential checked requests:8MiB target/4096 payload statements, with a single larger indivisible unit permitted only under the existing64MiB complete-request cap. Transactions, import options, limits and independent admission remain. Accepted correction-causes RC02 permits later effects inside a failed private request; stop subsequent requests and retain cleanup certainty under PC1. The [restore contract](persisted-execution-corrections-plan_2026-10-07.md#5-whole-import-units-in-bounded-requests--f06) owns exact boundaries.

Portable exports/backups declare the new completed-state format and digest. Same artifact identity
survives restore under different physical database names. Diagnostic/selected projection exports
remain incomplete views and cannot be imported as a complete compilation. Rebuild existing project
snapshots from pinned inputs on authorized adoption; do not retain legacy formats or compatibility
copies. Root access/configuration is never emitted as a reader capability or committed.

## 7. Execution packages and dependencies

Each package delivers working producers and its actual consumers, not just interface agreement.
Root owns shared model/schema/manifests, integration and acceptance. Parallel implementation needs
explicit disjoint ownership; ordinary work stays on main. No worktree is needed for plan authoring.

| Package | Prerequisite actually required | Delivered behavior and consumer/deletion boundary |
|---|---|---|
| **PG0 — decisions and owners** | Operator confirmations in §3; independent target-plan review resolved | Superseding ADR and coherent DESIGN/section/instruction boundaries. Existing receipts stay historical. No dependency on new production code for recording the target. |
| **PG1 — exact native completed state** | PG0 | Model-owned typed row/contribution/view/dependency/state contracts; native complete bodies, registry/graph mappings, memberships, completion and conflict semantics. SourceSnapshot/manifest identity consumers migrate in this package. |
| **PG2 — common native operations** | PG1 read/write/view contract implemented for a small actual producer/consumer | Bounded keyed/reference/field/range reads, terminal streams, bulk writes, batch codec and lazy scoped indexes. Test real native paths; share finite helpers rather than new storage/provider framework. |
| **PG3 — producer and lower compiler migration** | PG1/PG2 working; acquisition/provider contracts remain | Captured/facts and all nine normalization stages write/read native views. Both profiles and Facts/Normalized frontiers work. Remove completed IPC registry/prefix rewriting as each replacement becomes usable; no dual compiler. |
| **PG4 — upper/analytic/behavioral migration** | PG3 normalized inputs; PG2 access/Arrow transfer; each operation's actual predecessors | C0/C1/A0/selected A1/C2/S0/E0 and selected embedding use native views/shared preparation; execution/summaries and admission migrate. Removes per-grain full scans, duplicate preparation, overlapping chunk fetches and S0 second documentary build. |
| **PG5 — direct sealing and external boundaries** | PG3 for lower journey; PG4 for upper completion; PG1 manifest/state plus PG2 codec/write | Same DB final admission/reconciliation/index build/sealing. Bulk external endpoints, native-backed artifact-only export, external import and CLI/runtime consumers. Retire ordinary self-export/readmission/re-ingestion and obsolete graph-stream construction. |
| **PG6 — non-compiler selected access** | PG2 prepared indexed rows; current sealed serving contract | NativePackets/source usage/characterization/behavior, retained topology and relevant retrieval/evaluator adapters use shared selected access. Remove local repeated scans/duplicate lookups; preserve existing native search and embedding cache batches. |
| **PG7 — complete-state transport and originals** | PG1 completed-state format; PG2 byte ranges; PG5 import/seal boundary | Artifact/backup/restore and show/query/audit cover retained state; restore coalesces physical byte reads. Removes serial range route and all old-format assumptions while preserving complete admission. |
| **PG8 — bounded final encoding** | Existing delivery/paging contract, independent of compiler migration | One budget-aware final encoding in normal/retained service and relevant bridge transport. Retire duplicate serialization for the same final shape; retain required envelope/delivery controls. |
| **PG9 — integration and retirement** | All actual PG1–PG8 consumers migrated | Readiness/test recipes, config/CLI/docs and Python/native builds use persisted compilation. Remove abandoned compiler APIs/IPC authority, outdated rules/tests and compatibility-only paths. Finish coverage map and actual native/MCP/evaluator journeys. |

The original PG packages remain the combined scope; [PC0–PC6](persisted-execution-corrections-plan_2026-10-07.md#6-packages-dependencies-and-migration) complete their exposed boundaries. PC0 records decisions; PC1/PC2 supply completion and captured admission; PC3/PC4 jointly migrate scope layout/lowering and bounded candidates; PC5 consumes completion for import batching; PC6 joins their actual consumers into final acceptance. Existing PG9 completion now also requires the correction findings below. Agreement on an interface is not a working prerequisite, and local parser success is not complete restore.

PG2 may begin with PG1's narrow working slice, but PG1's remaining contracts/consumers stay open.
PG3/PG4 milestone boundaries do not create one transaction/task/table per relation. PG6 and PG8
can advance independently when their real prerequisites exist. PG7 designs transport with PG1
and lands before complete publication/backup claims; PG5 is not complete on a metadata-losing
restore. Shared model/native-reader/codec files need one writer even when logical work is parallel.

PG0 must replace conflicting future instructions in the graph-native coordinator and its four
supporting plans. Those plans retain semantic responsibilities and original receipts; this document
owns the combined new execution and finding disposition. The evidence/evaluation plans consume
the changed native boundary without becoming a second correction ledger.

### 7.1 Replacement execution and cross-plan contracts, 2026-10-09

The two graph/hash companions define GK0–GK7 and GR0–GR6. Their precedence applies to open
target/implementation routes, not historical evidence. The combined result is one model-owned
operation compiler, shared prepared roots/layouts, native canonical persistence, exact pure
product reuse and current independent completion/admission/publication.

| Existing work | Replacement/preservation and completion route |
|---|---|
| PG2/PG4/PG6 and PC3/PC4 selected access | GK1/GK2 own complete semantic scopes and compile indexed native/DataFusion or compact graph demand; GK3–GK5 migrate actual consumers. Existing table lowering, terminality, bounded candidate ordering/residuals and exact hydration remain mechanisms or required behavior under a replacement. |
| PJ4 normalization/validation/browse deferrals | GK3/GK5 schedule these reviewed repeated-access routes now. PJ2 fixed envelopes/typed ingress and PJ3 freeze/pointer-run preparation remain implemented foundations; no new global mutable generation cache is inferred. |
| BC1/CU1–CU4 containment | New compiler uses shared non-generic orchestration and thin synchronous typed leaves. GK4/GK7 preserve exact cardinality, order, charging, first-error and compiler-boundary evidence; no duplicate loader migration is required when the new target replaces that route. |
| BC2 conservative source/provenance membership | Retain full implementation and supplier provenance; GR0/GR4 introduce explicitly qualified computational/value domains. Old view identity is never reused because rows merely look equal. BC2 mutation/cold controls remain independently required. |
| Dependency-foundations-only horizon | GR0–GR4 deliver portable products, complete dependency tokens/reverse graph, invalidation and current ownership/admission. Embedding cache, publication effects and read capabilities retain their existing owners. |
| PC6/CU6/BC3/BC5/PG9/PJ5 acceptance | GK7/GR6 join surviving behavior controls on final source/profile. Existing failed timeouts remain failures until affected reruns pass; obsolete routes can be superseded only with a replacement behavior/control map. No historical pass qualifies the new compiler. |

Execution begins with GK0/GR0 decisions and GK1 scope semantics, then GK2/GR1 working shared
selection/program identity. GK3–GK5 migrate concrete consumers; GR2 supplies exact portable
reuse, GR3/GR4 integrate dependency propagation and finer domains, GR5 supplies pinned-request
reuse. GK6 assembles them; GK7/GR6 finish affected qualification and retirement. Producers and
actual consumers land together, with one owner for shared model/schema/native/manifest edits.
Available independent work depends on delivered contract slices, not completion of older plans.

GK0–GK6 and GR0–GR5 are implemented on the 2026-10-09 tree, with focused Tested
and source-review boundaries in §9.1. ADR-0139 owns operation compilation; ADR-0140 owns
selective product and viewer reuse. Exact request identity selects replay or fresh computation;
the charged dependency graph resolves implicit input selectors to selected frozen epochs while
keeping explicit epochs distinct. It is not a general incremental scheduler. Selected Entity,
Callable and Aspect domains are freshly discovered before rich hydration. SCC schedules execute
once per prepared graph, bound by its private materialization identity; canonical nominal topology
identities remain available without repeated binding hashes or a persisted SCC cutoff. Behavioral Local/Base/Body/SourceCall remain Fresh because their
complete private-owner validation repeats the kernels. Their obsolete portable hints are removed.
Viewer Moka initialization, pressure reclamation and close-before-reader invalidation are integrated.
GK7/GR6 and BC3/BC5/PC6/CU6 qualification remain open until their matching controls pass.
Operator databases/configuration and activation remain held. Native persistence does not require
every domain operation to be a native query; graph intent is not a universal serialized algorithm IR.

### 7.2 Native-efficiency replacement and common acceptance, 2026-10-09

The [native-efficiency companion](native-execution-efficiency-plan_2026-10-09.md) develops
NE0–NE9 from the independent source review. RC01–RC05 were explicitly operator-accepted on
2026-10-09. NE0 installs the complementary access/local-completion decision and owner updates;
plan authoring does not itself adopt those production changes. Existing GK/GR/PJ/PC/BC/CU IDs
and dated evidence remain intact.

The [independent target-plan review](../design_review/reviews/design_review_native-execution-efficiency-plan_2026-10-09.md)
accepts this coordinated target at Proposed / Interface-checked strength on 2026-10-09, without
new blocking findings. It does not establish implementation, native-query/session/lifetime
qualification or measured benefit; all eight source findings below remain Open.

NE1 shares exact resolved input bindings; NE2 qualifies actual native membership access without
owner Cartesian or union-index seen state; NE3 shares physical admission inputs while keeping
independent checks/Full negative universes; NE4 shares tokens and typed replay; NE5/NE6 provide
wake-driven handoffs and contribution-relevant completion; NE7 separates live flights from
optional retention; NE8 makes observation purpose explicit. These refine current GK/GR consumers,
not another canonical store, scheduler or mutable validity authority.

NE1's prepared binding and NE0's decision/transport slices precede their actual consumers.
NE5 and NE7 can advance on independent ready contracts; shared native/core files retain one
writer. NE4 does not gate cold-path fixes. NE9/GK7/GR6 retain their final-source retirement and
acceptance obligations; §7.3 integrates them into UP9's new common acceptance, including surviving
PC6/CU6/BC5 and affected CLI/native/MCP/transport controls. BC3 retains its separate candidate-profile gate and release defaults until qualified.
Old timeouts and canceled/not_run boundaries remain unqualified; optional reuse cannot explain
first cache-off timeouts. No new native run, environment synchronization or production change
is authorized by this document integration.

### 7.3 Unified persistence replacement and common acceptance, 2026-10-09

The [unified companion](unified-persistent-surrealdb-plan_2026-10-09.md) owns UP0–UP9.
Its source RC01–RC07 were individually operator-accepted on2026-10-09. UP0 changes decision/model
contracts before dependent implementation; UP1 supplies stable service attachment; UP2 provides
actual payload/anchor/exact-view resolution; UP3 supplies fenced current attachment/pins/effects.
UP4 migrates eligible compiler reuse, UP5 manifest publication and scoped serving, UP6 portable
recovery, UP7 stable tests/worktrees/diagnostics and UP8 native retention/composed resources.
UP9 owns the new combined cutover/acceptance, absorbing affected remaining NE9/GK7/GR6/PJ5/PC6
obligations rather than requiring superseded private-database designs to finish first. Unrelated
BC3 candidate-profile and real-library/operator adoption boundaries remain separate.

The same current domain/dependency declarations govern meaning; there is no second scheduler,
canonical store, mutable cache-validity authority or findings ledger. AF1/AF4 ownership/outcomes,
AF6/AF7 diagnostic integration and storage-management delegation are prerequisites at their
specific boundaries, not whole-plan barriers. Preserve concurrent AF corrections and rebase its
interfaces; arbitrary SQL/native MCP is not safe merely because a stable store is attached.
No service, test run, store, selection or cleanup action is authorized by this document update.

## 8. Sole finding disposition

### Unified-persistence findings transferred on 2026-10-09

This is the sole current disposition of source-review F01–F08. All are **Open / scheduled**;
the [independent target-plan assessment](../design_review/reviews/design_review_unified-persistent-surrealdb-plan_2026-10-09.md)
accepts the corrected Proposed target, not implementation. Package coverage in the companion is not another status
table. Target-plan acceptance or an accepted rule does not establish implementation closure.

| Source finding | Responsible component / packages | Closure evidence |
|---|---|---|
| [Unified F01](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F01) | Service/fixture/native content; UP0/1/2/7/9 | Two worktrees attach to one existing service/content without provisioning/replay; close/restart preserves other consumers and admitted data. |
| [Unified F02](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F02) | Model/native exact resolver; UP0/2/5/6 | Equal payload sharing and cross-view revisions; same-view conflicts, forward/reverse foreign endpoints and missing originals reject; isolates/parallel arcs retained. |
| [Unified F03](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F03) | Publisher/reader/serving; UP0/3/5/6 | B publishes while A remains pinned with exact definitions/evidence/cursors; pending/unrelated content and search candidates cannot leak into A. |
| [Unified F04](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F04) | Model/core reuse/current binding; UP2/3/4/9 | No-replay unchanged attachment; changed membership/deletion/coverage/model/code/provenance agrees with independently specified cold outcomes. |
| [Unified F05](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F05) | Native attempt/effect recovery; UP0/3/6/8 | Delayed writes, lost commit response, client death, concurrent conflicts and restart settle exact committed identity or explicit uncertainty. |
| [Unified F06](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F06) | Fixture/test/run/worktree; UP1/3/7/9 | Complete setup/teardown migration, ordinary concurrent isolation, fresh negative/cold controls and explicit disruptive maintenance; no scratch fallback. |
| [Unified F07](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F07) | Core/native/serving resource composition; UP2/4/5/7/8 | Revealing cold/warm, high-degree/global cases plus source inspection of simultaneous representations, backpressure/release and suitable kernel placement. |
| [Unified F08](../design_review/reviews/design_review_unified-persistent-surrealdb_2026-10-09.md#F08) | Native lifecycle/storage delegation; UP3/5/6/8 | Shared two-view retention, pin/retire races, interrupted cleanup, unresolved effects and worktree removal preserve protected content. |

The [target-plan TF01](../design_review/reviews/design_review_unified-persistent-surrealdb-plan_2026-10-09.md#TF01)
is **resolved in the Proposed document / implementation Open**, owned by publisher/native UP6/UP9.
It replaces ordinary Root-based dump execution with data-only lowering, definition comparison,
fresh mutable control identities and explicit maintenance recovery. Closure needs concurrent
same-dump imports/unrelated tests and refusal of USE/DDL/control-ID injection plus complete cold
admission. This refinement does not close source F02/F05/F06 or create a second finding ledger.


### Native-efficiency findings transferred on 2026-10-09

The [source review](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md)
retains its static dated assessment. The rows below are the sole current disposition owner;
all eight findings remain **Open at enclosing qualification; source corrections Implemented, 2026-10-09**.
ADR-0141 records the accepted target. Independent implementation review found and rechecked
corrections for global producing-root admission, owner lookup complexity, bridge task reaping,
candidate catalog isolation and partial Full-input cleanup. Static acceptance is not runtime closure.

| Source finding | Responsible component / package and decision route | Required closure evidence |
|---|---|---|
| [Native-efficiency F01](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f01) — absent owner/key enumeration | Native exact-view reader and completion/token consumers, NE0/NE2/NE9; accepted RC01 | Qualified equality-prefix/one-owner/verified-pointer or exact-view prepared route without relocated Cartesian or native union-seen state; independent sparse/missing/foreign/overlap/conflict/order/projection and terminal controls; unrelated-owner/startup work justified |
| [Native-efficiency F02](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f02) — repeated admission preparation/input | Core/model support and normalization/general admission, NE1/NE3/NE9; accepted RC03 | Compatible exact providers/logical preparation and union typed input shared across separate checks; Full ownership, stored-output orphans, missing qualifications and context/epoch differences still refuse independently |
| [Native-efficiency F03](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f03) — repeated selected tokens | Core prepared root domains / NE4/NE9; accepted RC03 | One union token read per exact input/group, unchanged independent root keys/roles/missing and complete negative domains, hit/miss/deletion/empty controls. Enabled lookup only; not a cache-off explanation |
| [Native-efficiency F04](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f04) — repeated product representations | Core/model canonical/current replay and capture / NE4/NE9; accepted RC03 | One charged typed decode through canonical/current pre-effect checks and fresh ingress; wrong-but-canonical rows refuse, uncertain effects remain fatal. Cold capture decision preserves actual completion/readback without duplicate representation or unbounded residency |
| [Native-efficiency F05](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f05) — polling/fine handoffs | Native bridge/provider/core writers and batches / NE5/NE9; accepted RC04 | Actual completion/cancellation wakeups and suitable coarse async/blocking boundaries; queue-full/drop/late-error controls retain charged batches, runtime/client and terminal acknowledgements |
| [Native-efficiency F06](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f06) — global scan wait at local completion | Native/core contribution lifecycle / NE0/NE6/NE9; accepted RC02 | Own relevant read/write descendants gate local completion; unrelated retained immutable readers do not. Late errors/global poisoning and final freeze/seal/abandon drainage remain, with no premature publication |
| [Native-efficiency F07](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f07) — pressure splits initialization | Viewer preparation/pin lifecycle / NE7/NE9; accepted RC04 | Same exact key stays one live flight across retention replacement; independent cancellation, completion/removal/close races and external borrows retain charges/pins; no deferred rich result ownership after flight terminality |
| [Native-efficiency F08](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md#f08) — whole-inventory selected observation | Typed extraction harness / NE8/NE9; accepted RC05 | Explicit selected versus complete observation; selected inspectors avoid unrelated rich scans while independent unexpected-row/orphan/inventory controls remain native and complete |

Additional execution findings below were identified on2026-10-09; they do not rename or rewrite
source-review F01–F08. This remains their sole disposition owner.

| Execution finding | Responsible component / current disposition | Closure obligation |
|---|---|---|
| NE-I01 — failed local preparation retained as immutable result | Native membership preparation / NE2/NE9; focused native recovery control passed2026-10-09 | A pre-native local refusal releases entry/setup charges and does not refuse a later healthy same-view consumer; stale waiters cannot erase replacement entries; global native failure/uncertainty stays sticky |
| NE-I02 — native export syntax cause flattened | Extraction coverage / NE8/NE9; focused coverage control passed2026-10-09 | Explicit Ruff/Pyrefly coverage assertions retain clean, syntax-error and undecodable distinctions without depending on row ordering |
| NE-I03 — transfer size controls compute repartitioning | Workspace/native provider statistics / NE1/NE3/NE9; physical-plan, conservative-statistics and transfer7/128MiB controls passed2026-10-09 | Tiny/selected inputs avoid gratuitous parallel sorts, substantial inputs remain eligible for available-CPU parallelism, transfer7 and128MiB stay unchanged; inexact zero cannot eliminate native authority checks |
| NE-I04 — optional capture mutates canonical view state | Native contribution observation/core products / NE4/NE9; focused native state-identity/refusal control passed2026-10-09; both full matrices timed out before equivalence completed | Exact singleton read leaves completed-state identity unchanged; invalid/foreign/undeclared output still refuses, registered dependency/publication views remain strict, cache-off/cold/hit manifests agree |
| NE-I05 — analytics fixture omits local tokenizer | Compiler analytics test adapter / NE9; delegation Implemented, runtime pending | The existing fixture tokenizer is available independently of simulated remote-service availability; selected algorithms and deliberate service failures reach their intended controls without Qwen assets |
| NE-I06 — graph-only tamper is not coherent detached state | Compiler artifact controls / NE9; separate physical and coherent semantic negatives passed2026-10-09 | Graph/state disagreement refuses at exact native backing conflict; a separately completed neutral candidate passes full cold state audit and then refuses exact unsupported-support semantics, without a captured-producer authority claim |
| NE-I07 — preparation silently increases partition target | NominalClosure preparation / NE3/NE9; undocumented minimum removed, runtime pending | Preserve caller/available-CPU partition target, full257×400 edge/grain correctness and existing8MiB budget; no hidden second sort/merge or compute/transfer recoupling |
| NE-I08 — aliases amplify immutable preparation | Core reference validation and logical templates / NE1/NE3/NE9; source corrected, runtime pending | One physical branch per exact target view without merging distinct views/roles; templates validate their actual captured ports, including replacement during planning, without unrelated catalog sweeps or relaxed authority |
| NE-I09 — closed binding hash depends on discovery order | Model binding normalization/admission / NE9; pure permutation/content/missing-member control passed2026-10-09, native confirmation pending | Production and independent stored-row admission share canonical attempt-ID order; permutation preserves exact eligibility/member content, changed content and missing attempts remain distinct, charges release |
| NE-I10 — effect batch ignores available memory | Embedding realization / NE9; budget-derived microbatches Implemented, runtime pending | Existing4096D/4MiB scenario admits multiple effect batches, nonadjacent texts share one winner, unadmittable single request fails before effects; service ceiling and current compute/CPU targets remain |
| NE-I11 — construction arena survives executable preparation | Model scope factories/core normalization / NE1/NE3/NE9; retained-metadata reconciliation and factory release Implemented, runtime pending | Finished factories retain sound nested/external metadata charges, release construction scratch, and end before lowering/async execution; unchanged empty8MiB/callable2MiB controls pass |

The companion maps the review's other recommendations, libraries, alternatives and investigation
triggers without duplicating mutable status. SDK-session qualification and unresolved historical
timeouts remain explicit NE0/NE9 obligations in §9.1. Existing finding closure is not inferred
from these new source IDs or an accepted rule decision.

### Graph/hash review findings transferred on 2026-10-09

Both findings are **Open at enclosing scope; GK0–GK6/GR0–GR5 source correction Implemented / focused Tested, 2026-10-09**.
This is their sole current disposition owner. Their source assessment remains dated and is not
rewritten as implementation acceptance. The new target supersedes overlapping open routes;
PC6/CU6/BC3/BC5 and remaining timed-out boundaries survive until matching evidence resolves them.

| Source finding | Responsible components and packages | Required closure evidence |
|---|---|---|
| [Graph/hash F01](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#F01) — repeated per-root planning/hydration | Model/compiler/native selected-access; GK2–GK7, GR1–GR6 | Shared prepared selection and bounded hydration reach every applicable consumer with exact owner partitions, complete negative domains and conflict/cancellation/skew controls. Same/cold/hit/changed-input products match independent expected membership and clean computation; no latency attribution inferred. |
| [Graph/hash F02](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#F02) — independently interpreted body/coverage scope | Normalized scope/ClassInventory and compiler/admission adapters; GK1–GK3/GK7 | One complete owned body/initializer/qualification/coverage operation, independent nested/foreign/missing cases, rule extension and contrasting lowering without recovering domain meaning from the adapter; competing interpretations removed. |

The seven opportunity routes and library/investigation decisions are developed in the two
companions, not another status register. Scope ownership is prerequisite to aggressive sharing;
reuse does not close the older catalog-speed, correction or populated findings automatically.

### Assembled scope-compiler review, 2026-10-09

The [independent assembled review](../design_review/reviews/design_review_assembled-graph-scope-compilation_2026-10-09.md)
assesses the integrated GK0–GK5 tree at static / Implemented strength. These source-qualified
IDs have this coordinator as their sole disposition owner. Source correction is distinct from
the runtime evidence in §9.1 and from enclosing GK7/GR6 qualification.

| Source finding | Responsible owner / current disposition | Revealing evidence obligation |
|---|---|---|
| VS-F01 — independently reconstructed ClassInventory scope | Model normalization / GK1, source corrected / focused Tested, 2026-10-09 | Execute the actual aspect program through the finite evaluator; nested/member/coverage and rule-extension cases retain independent expected sets. |
| C-F01 — absent seed suppresses a subsequently reached nominal | Finite/native selection / GK2, source corrected / focused Tested, 2026-10-09 | Independent absent/virtual outcomes and reached-missing reverse membership agree across realizations. |
| C-F02 — optional discovery history consumes required-work budget | Shared finite/native preparation / GK2, source corrected / focused Tested, 2026-10-09 | Optional retained discovery memo removed; repeated, reordered and skewed windows preserve exact membership and release charges. |
| C-F03 — unsupported finite scalar/list/NULL/parameter states become false joins | Finite lowering / GK1–GK2, source corrected / focused Tested, 2026-10-09 | Explicit scalar and NULL adapters or early refusal; list scalars and missing/wrong-kind parameters reject, including empty input. |
| C-F04 — missing occurrence adapter becomes false structural selection | Finite lowering / GK1–GK2, source corrected / focused Tested, 2026-10-09 | Up-front matching occurrence requirement, nearest ranking and canonical occurrence controls. |
| C-F05 — semantic sum-reference name differs from declared storage column | Record/macro projection and finite lowering / GK1–GK2, source corrected / focused Tested, 2026-10-09 | Preserve semantic graph roles; generated physical references match active encoded columns, optional/inactive omission, exact scalar joins and native/finite TypeOf child membership. |
| GK-F01 — core independently authors family roots/ranking | Model factories / GK3–GK4, source corrected / focused Tested, 2026-10-09 | Aspect, E0, S0, Model and SourceCall known-answer inventories and negative domains; native/finite primitives preserve the same intent. |
| GK-F02 — Model still loads independently per selected root | Model consumer / GK4, source corrected / focused Tested, 2026-10-09 | Shared selected-demand window; duplicate targets, exact independent catalog/symbol sets and absent-context refusal. |
| GK-F03 — construction/SQL/canonical storage admitted after or far beyond actual allocation | Model factories/core lowering / GK1–GK4, source corrected / focused Tested, 2026-10-09 | Reservations precede copies/formatting and survive readers; exact canonical count/digest/capacity, unchanged focused budgets and final release. |

Structural and Analytic retain one demanded complete frame per exact provider/input/context;
Summary retains its complete SCC/fixpoint universe. These are justified whole-universe kernels,
not unresolved per-root migration. Normalization borrows typed union views. C1/C2/S0/Local decode
the union once but retain charged exact current-partition copies for private kernels. SourceCall,
E0 and Model share rich native columns and still decode each private partition. No claim that all
rich decoding occurs once is made. SourceCall's set DAG is relational/native in this slice;
primitive scopes have contrasting finite/native lowerings. Persistent products and reusable
cross-request preparation are now integrated; the reuse companion §7 records eligibility and
current-owner refinements. Their enclosing qualification remains open.

### Plan-target review finding transferred on 2026-10-09

The [independent target-plan review](../design_review/reviews/design_review_graph-compilation-and-reuse-plan_2026-10-09.md)
accepts the revised combined target at **Proposed/interface strength**. Its F01 is distinct from
source graph/hash F01. The document correction resolves its target-order gap; implementation
and the revealing lifecycle control are **Implemented / focused Tested, 2026-10-09** under GR5.
Actual native consumer integration passed its focused repaired control; GR6 enclosing acceptance
remains open.

| Source finding | Responsible component and disposition | Required implementation evidence |
|---|---|---|
| [Plan-target F01](../design_review/reviews/design_review_graph-compilation-and-reuse-plan_2026-10-09.md#F01) — cache insertion racing viewer retirement | Viewer/cache lifecycle, GR5/GR6. **Implemented / focused Tested, 2026-10-09**: fence admission/retries, drain initialization/insertion owners, release final cache references, then wait external leases. Moka retention generation is dropped under pressure/close rather than assuming prompt logical-invalidation reclamation. | Race completing initialization against retirement; no cache-owned pin/charge survives final release, while an external borrower retains its own pin/charge until release. Production closure requires that control and actual consumer integration. |

### Compiled-product contract review, 2026-10-09

The [independent source review](../design_review/reviews/design_review_compiled-product-reuse-contracts_2026-10-09.md)
retains its initial findings and dated follow-ups. This coordinator owns their scheduled
disposition; source corrections and bounded passing controls do not close GK7/GR6.

| Source finding | Responsible component / disposition | Closure evidence and limit |
|---|---|---|
| Reuse-contract F01 — native unknown effects, quota and coordination | Native products / GR2–GR6. Source corrected; focused native controls passed on the recorded revisions. | Exact ACK/readback, point-stage/generation fences, uncertain reservations, cross-process borrowed capacity, fixed stripe collision refusal and lifecycle drain. Reset follow-up requires persistent coordination owner and explicit pre-DDL fence; final revised control recorded in §9.1. No operator installation. |
| Reuse-contract F02 — uncharged queued owners and retained optional capacity | Viewer preparation / GR5. Source corrected / focused Tested after repairs. | Insertion owner charged before spawn; canceled waiter and close races drain owners. Whole optional Moka generation releases unborrowed values before required-work retry; retained external borrows remain charged. Initial failing composite is retained. |
| Reuse-contract F03 — portable private state fidelity/completeness | Behavioral products / GR4. Source route retired, 2026-10-09. | All four private-owner families remain Fresh; obsolete hint interfaces/private portable fields removed. No avoided-computation claim. Full Behavioral pipeline acceptance is still open. |
| Reuse-contract F04 — canonical decoder field/identity validation gaps | Model/native typed replay / GR2–GR4. Source corrected / focused Tested. | Typed canonical re-encoding rejects unknown fields and duplicate IDs across bounded windows before semantic callbacks or ingress. Current predicates validate Receiver/Event/Binding. Full compiler matrices timed out and do not establish integrated equivalence. |

### Retained catalog-speed and additional findings

Catalog-speed F01–F05 and D01–D04 remain **Open** while integrated acceptance proceeds. The additional implementation-review F01 passed its original two-family cold-backing boundary; required projection/metadata families have reopened the broader backing boundary below. Bounded implemented/tested slices in §9.1 do not establish enclosing finding closure. Acceptance of a decision is not verified closure. Source-review IDs retain their original scope; additional IDs below are this
plan's source-inspected defects, not retroactive edits to the original independent review.

| Source / plan finding | Responsible component and packages | Required closure evidence |
|---|---|---|
| [Review F01](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md#f01) | Native access + compiler adapters, PG1–PG4/PG6 | Actual ID/view selection reaches indexed bounded physical demand; known counts reach bulk planning; no per-grain rich whole-source scans. Duplicate forward preparation and overlapping physical chunks removed. S0 uses its compact completed spool with preserved support and lifetime. |
| [Review F02](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md#f02) | Workspace/completion/codec, PG1–PG3/PG5 | Immutable native membership replaces prefix re-sort/rewrite/hash. Batch conversion replaces singleton row packing. Exact duplicate/conflict, deterministic final identity and old selected views pass targeted controls. |
| [Review F03](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md#f03) | Model admission/core/publisher, PG1/PG4/PG5/PG7 | Exact unchanged premises retain validation; changed upper/membership premises recheck. No ordinary self-import; external tamper and stored corruption still reject. Completed-state transport retains true reference/dependency closure. |
| [Review F04](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md#f04) | Analytic owner/adapters, PG4 | All-disabled and one-selected cases preserve total outcomes/frames and skip unrequested kernel-only preparation. Required A0 topology is retained. |
| [Review F05](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md#f05) | Native loader, PG2/PG5 | Bounded external endpoint dedup/bulk insertion precedes enforced edges; repeated targets, distinct relationships and terminal failure retain exact behavior. |
| **D01 — repeated scoped typed-family scans**: `source_evidence.rs:82–117`, nested `source_usage` callers | Shared prepared rows + serving, PG2/PG6 | One lifetime-bound ID/reference index reaches selected row positions; all actual packet consumers migrate. Missing/foreign IDs, duplicate roles and pin boundaries retain their meanings. |
| **D02 — scalar original restore transfer**: `backup.rs:296–330` versus existing `original_bytes_batch` | Original range owner + restore, PG2/PG7 | Bounded coalesced restore uses shared verified range reads; ordered complete original-byte admission, hashes and late failure remain correct. |
| **D03 — duplicate final response serialization**: `service.rs:124–143,236–250`, dispatch count/encode | Model wire + service/bridge, PG8 | Same final shape encoded once with tracked growth/exact cap; normal/retained paths and final delivery maps pass. Different envelopes and changed-shape stabilization remain intentional. |
| **D04 — singleton Arrow conversion inside native batches**: `codec.rs:50–81`, loader and reconcile | Shared batch codec, PG2/PG5 | Each bounded type slice encoded once, correct aligned bodies/order including opaque binary; independently regenerated actual-state equality still rejects altered bytes/fields/edges. |
| [Implementation-review F01](../design_review/reviews/design_review_persisted-graph-implementation_2026-10-07.md#f01) — retained non-graph backing fidelity | Native compiler state, PG1/PG5/PG7 | **Prior two-family boundary targeted Tested, 2026-10-07; broader backing closure reopened by required snapshot/metadata families:** `ef47b7c2` + `e73183dd` compare closed typed bodies, full nominal key/physical ID, exact canonical bytes/content and actual original-page digests. Owned `compiler_views` three controls passed (161.36s), including valid body/key/export-row alteration and coherent physical-original tamper. No singleton Arrow re-lowering or new ordinary rich-prefix pass. Whole publication/restore acceptance remains separate. |

### Correction-causes findings transferred on2026-10-07

All six are **Open; corrective design accepted through ADR-0138, production implementation integrated, enclosing acceptance pending**. This is their sole mutable disposition owner; the source review retains its dated judgment. PC packages do not reset earlier finding IDs or treat prior focused receipts as closure.

| Source finding | Responsible owner and package | Closure evidence / dependency |
|---|---|---|
| [Correction-causes F01](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#f01) | Model completion + native/core/publisher/CLI, PC1 | All setup/finalization siblings preserve typed primary/secondary failures, terminality/effect certainty and committed-resource disposition; failure combinations and interrupted drain controls. |
| [Correction-causes F02](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#f02) | Model/provider declaration and admission, PC2/PC6 | No prototype/exception classifier; multi-supplier/NotRequested cases, supported captured-build admission, explicit old-format refusal and new-format cold transport/reporting. |
| [Correction-causes F03](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#f03) | Shared native lowering and every published consumer, PC3/PC4/PC6 | Prepared result positions/finality, context/null/residual fidelity and actual selected plans across compiler/reader/projection/serving. |
| [Correction-causes F04](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#f04) | Native candidate ordering/access, PC4 | No whole-match arrays or result-sized seen set; bounded/spillable compact ordering, exact membership/alias/order controls, cancellation and late failure. |
| [Correction-causes F05](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#f05) | Native schema/lowering/reconciliation, PC3/PC6 | Accepted RC01; table-specific extension locality, retained real scalar/search consumers, independent imported-field corruption controls and new realization qualification. |
| [Correction-causes F06](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md#f06) | Publisher restore, PC5/PC6 | Accepted RC02 and PC1; multi-unit/whole-transaction limits and failed-request isolation, followed by complete fresh restored admission. |

The [integrated correction review](../design_review/reviews/design_review_persisted-execution-corrections-integrated_2026-10-08.md) accepts the examined architecture at static Implemented / Interface-checked strength after independent inspection of the following four source corrections. Their named execution evidence remains open. Its findings remain distinct from the correction-causes review.

| Integrated-review finding | Responsible owner / package | Current disposition and remaining evidence |
|---|---|---|
| [F01](../design_review/reviews/design_review_persisted-execution-corrections-integrated_2026-10-08.md#f01) — cold registry completeness | Native compiler validation, PC3/PC6 | Source corrected to the table-specific registry. Captured transport, compiler-only QualityStep cold backing, original-byte corruption and graph-backed completed-family selection passed focused controls; populated transport/journey also passed (§9.1). Whole-plan acceptance remains open. |
| [F02](../design_review/reviews/design_review_persisted-execution-corrections-integrated_2026-10-08.md#f02) — retained member state | Serving selection, PC4/PC6 | Owned charged members, roots and requested identities are integrated. Three low-budget/lifetime/refusal controls, actual 260-member multi-window serving controls and populated journeys passed (§9.1). Whole-plan acceptance remains open. |
| [F03](../design_review/reviews/design_review_persisted-execution-corrections-integrated_2026-10-08.md#f03) — capture companion lowering | Native schema / serving scope, PC3/PC6 | Canonical body inputs and shared lowering are integrated. Independent native library eligibility passed multiple releases/captures, role0 input distributions, excluded orphan/role1 and absent/unfiltered requests; populated corpus/public journey also passed (§9.1). Whole-plan acceptance remains open. |
| [F04](../design_review/reviews/design_review_persisted-execution-corrections-integrated_2026-10-08.md#f04) — published physical hydration | Native reader / serving selection, PC4/PC6 | Checked bounded physical-node hydration preserves exact correspondence, order and retained charges. Independent source review, actual selected native plans, multi-window serving, finite budget/refusal controls and populated native/MCP journey passed (§9.1). Whole-plan acceptance remains open. |

Secondary fetch delegation is scheduled in PC4; retained Arrow/Tokio/shared-join mechanisms and triggered parser/native-order specializations are explicitly disposed in correction plan §4.3/§7. No separate capability or investigation status register is introduced.

### Populated-journey findings transferred on 2026-10-09

Both scheduled findings are **Open; corrective target accepted through ADR-0138, implementation integrated and targeted acceptance in progress**, developed in the [populated companion](populated-journey-execution-amplification-plan_2026-10-09.md). Source-qualified IDs are distinct from the earlier reviews' F01/F02; no earlier finding is renumbered or automatically closed.

The [independent design/target plan review](../design_review/reviews/design_review_populated-journey-execution-plan_2026-10-09.md), 2026-10-09, concludes **Accept scoped at Proposed strength**. Its separate plan-review F01 corrected the proposed finalization order and workspace/native handoff before publication. No remaining blocking target finding was identified; neither that static correction nor document acceptance closes scheduled source findings or qualifies production.

| Source finding | Responsible owner and packages | Required closure evidence |
|---|---|---|
| [Populated-review F01](../design_review/reviews/design_review_populated-journey-execution-amplification_2026-10-08.md#f01) — vocabulary-wide physical dispatch | Native schema/codec/ingress and independent reconstruction; PJ0/PJ2/PJ5, extending PC3/PC4 | Selected-record adapters and fixed native envelope/supplied scopes across every writer and cold consumer; unrelated-kind extension does not expand existing write programs; malformed typed ingress and private native tamper reject at the declared boundaries; fresh-schema native acceptance and independent full physical reconciliation. |
| [Populated-review F02](../design_review/reviews/design_review_populated-journey-execution-amplification_2026-10-08.md#f02) — repeated completed canonical preparation | Native mutation/read/scratch owner and artifact/publisher consumers; PJ0/PJ3/PJ5, extending PC6 | Atomic final content freeze rejects pending/failed state and late mutations while reads/derived publication remain supported; one complete alias-aware pointer preparation per family reused by independent charged cursors for header/payload/export/sealing; exact empty/conflict/cancellation/cold controls and no post-admission canonical reload. Independent actual-state reconciliation remains. |

The [independent assembled review](../design_review/reviews/design_review_assembled-populated-execution-correction_2026-10-09.md)
identified and independently source-reassessed these distinct integration findings on 2026-10-09.
Its final verdict is **Accept scoped at static / Implemented strength**; matching-source focused and populated runtime evidence is recorded in §9.1, with the stated limits.

| Assembled-review finding | Responsible owner | Current disposition and closure obligation |
|---|---|---|
| F01, known-export inspection coupled to freeze | Core artifact | Source-resolved / focused Tested, 2026-10-09; intact export accepted and tampered/missing originals plus invalid assertions refused (`20261009T062646.015Z-b2b5ea`). Inspection no longer mints a publication capability. |
| F02, physical freeze minting publication authority | Core admission / publisher | Source-resolved; private publisher workflow accepts only store/manifest-bound admitted owners, retains its own private session, checks the exact endpoint, and keeps marker writing private. Current populated direct publication and fresh backup/restore passed (`20261009T060142.629Z-c9c159`); foreign captured detached admission passed in the earlier current-source composite. The large compiled-export publication control retains its 300s timeout. |
| F03, admitted read refused by global closure | Native read/preparation | Source-resolved; retained-parent preparation permits descendants during closure, new external reads remain closed. Focused cached/first-owner lifetime controls passed in the 54-control selection (`20261009T053546.212Z-22b6e9`); that run retains two separately repaired backing assertion failures. |
| F04, cancellation losing acknowledged publication identity | Native completion / publisher | Source-resolved; marker acknowledgement records exact committed identity before another await, drainage/refusal retain it and abandon cannot remove committed content. Guard cancellation controls passed in the same native selection; no injected production marker/readback cancellation is claimed. |
| F05, acknowledged failures logged as incomplete | Phase owners | Tested for the selected boundaries, 2026-10-09: real core admission rejection passed (`20261009T055352.008Z-869ea3`); current native authentication refusal/filter/attribution/abort controls passed (`20261009T055723.301Z-c3525e`, four). Binary macro/callsite suppression required explicit captured-Dispatch emission; its upstream cause is unproven. Drop stays silent. Both populated success paths and restored publication emitted attributed terminals and passed (`20261009T060142.629Z-c9c159`). |
| F06, unguarded post-closure state read | Native finalization | Source-resolved; read requires a borrowed live same-store finalization guard with private origin flag. Label-spoof, unrelated-parent and retained drainage control passed in the native selection; it exercises local guard ownership, not a completed database query. |


[Populated-review F03](../design_review/reviews/design_review_populated-journey-execution-amplification_2026-10-08.md#f03) remains **Deferred in its source review**, triggered by supported independent pre-final overlap or demonstrated unrelated blocking. Companion §6 routes all other investigations without creating another finding register. RC01/RC02 are operator-accepted; RC03/RC04 remain unchanged. PJ0 has installed ADR-0138 and the semantic/storage owner changes before dependent production work.

Retain source links and responsible owners when combining related corrections. Do not close F01
solely on a native database rename, F02 on moving the same prefix work into an index builder, or
F03 by deleting checks. Additional exposed consumers join the responsible package before closure.

## 9. Targeted functional verification and completion

**Current execution dependencies, accepted target 2026-10-09:** UP0–UP9 supersede the private
database realization while retaining PC/PJ completion, captured-binding, exact selection and
bounded terminality guarantees. Native content/views and fenced effects precede admitted
attachment, manifest publication and scoped serving. UP6 replaces ordinary raw SQL restore with
data-only lowering and fresh logical staging; privileged whole-service recovery remains explicit
maintenance. UP7 supplies stable validation attachment to the migrated controls. Integrate these
consumers before UP9's populated journeys and reuse only valid matching-source/profile evidence.

PJ5 and existing PC6/CU6/BC5 consume the resulting native/CLI/cold/MCP controls; no plan's receipt silently closes another plan's obligations. BC3 retains its candidate/default qualification gate and active release route. The earlier operator-deferred timeout receipts remain failed at their original source boundary. The current authorization to execute all remaining GK/GR scope reactivates matching final-source assembled release qualification; it does not establish causal runtime attribution or measured benefit. Authoring alone never activates this gate, and focused passes cannot substitute for it.

These are **planned checks, not newly executed tests**. Use touched-crate compile checks and
explicit affected family/filter controls throughout implementation. The operator's hard-pivot
scope uses meaningful targeted functional testing; no legacy suite parity, matching wheel or
work-accounting proof is required. Applicable non-functional leaves run once at functional scope
end, with root `just turn-end` last. Pure model/algorithm controls stay independent of a server;
native persistence/transport controls use owned authenticated persistent fixtures.

| Boundary | Revealing cases and evidence |
|---|---|
| Completion/views/identity | Pending rows excluded from IDs, roots, closure, negative answers and bulk scans; multiple outputs including complete-empty; each availability outcome; frozen view unchanged by later overlap; full-key conflict versus equal duplicate; shuffled batches yield identical final content/state. |
| Complete body/graph mapping | Every registered required record kind; QualityStep backing; opaque/non-UTF8 originals versus logical text; exact IDs/digests/nullable sums/integer widths/finite floats; Facts without normalized Place aliases and Normalized with them. |
| Dependencies and validity | New membership or missing-to-present lookup changes an absence-sensitive premise; different model/settings/spec invalidates the affected check; unrelated unchanged exact views remain valid; dangling internal references and proof cycles fail independently of legal data cycles. |
| Shared selected access | High-degree/shared owners, reconvergence, cross-module premises, isolates/parallel edges, missing/foreign keys, incoming ownership versus outgoing dependencies, scoped indexes under another pin, overlapping and out-of-order logical ranges. |
| Selected computation | All-disabled A1 and each affected selected dependency route; A0 independent; S0 compact spool preserves global requests/support; catalog and behavioral lower/upper controls; exact embedding winner/spec/projection reuse without fake live-service claims. |
| Native finality/lifecycle | Late read/write/statement/transport failure; known aborted batch versus unknown ACK; cleanup failure reports orphan; no incomplete marker/selected result; authenticated persistent restart; drained writer cannot mutate sealed result; viewer pin survives another snapshot publication. |
| Complete external transport | New-format owned export and detached tamper; graph plus completed-state identity across physical names; missing/altered/extra membership or compiler backing; original-byte coalescing and corruption; actual fresh backup/restore/audit/projection/MCP journey. |
| Wire/evaluation | UTF8/escaping/null/numeric payloads, exact below/at/over byte cap, refused reservation/deadline, normal and retained cursor path, final serialized original-byte maps; independent evaluator observations retain task/expected-answer separation. |

The [correction acceptance table](persisted-execution-corrections-plan_2026-10-07.md#7-verification-and-remaining-investigations) adds complete failure combinations, captured semantic binding/version cutover, shared query-result positioning, table-local schema growth, spill/dedup lifetime and batched-import failure cases. PC6 and PG9 require these as well as the preserved journeys above. Do not run final old-format acceptance before its replacement producers/consumers exist.

Use existing shared validators, original-byte verifiers and independent small expected graphs.
Do not mirror a new lowering in test-only code or compare only counts. Functional controls verify
meaning/failure boundaries; source inspection establishes removal of repeated work. A representative
completed real-library timing is needed before a Measured speed claim, not before selecting native
compilation. Real Catalog/Behavioral pilots and operator selection remain held for separately
authorized adoption after this implementation; do not restart them during plan authoring.

Complete PG9 only when all applicable callers in §2 have the chosen route or a source-supported
keep decision, all §8 obligations have closure evidence, stored state survives transport, obsolete
paths are deleted, and targeted actual native/MCP/evaluator journeys plus affected leaves pass.
Report stopped, failed and not_run evidence honestly. Update STATUS from that actual boundary.

### 9.1 Current execution checkpoint, 2026-10-07

**Unified execution in progress, 2026-10-09, baseline `20bc8cf9`.** ADR-0143 supersedes
ADR-0138/0140 for shared native content, exact published views, admitted attachment and stable
service ownership. Source changes span UP0–UP8; runtime closure and UP9 remain Open. New owned
service installation is authorized; existing operator/PSE-arrow state, selection and protected
inputs are outside this cutover. No fixed service memory cap or test/compiler parallelism cap
is introduced. Patched gRPC3.3.0 remains the transport.

**passed on intermediate source:** `cargo check --release -p cpg-core --tests`;
`cargo check --release -p lctx-serving --tests`; `cargo check -p lctx-surrealdb --lib
--test native --test cache --test projections --profile release`. These are typing checks,
not persistence or integrated acceptance. Subsequent reuse/partition/retirement refinements
require refreshed checks. Tooling controls passed with `uv run --no-sync pytest -q
 tests/scripts/test_surrealdb_fixture.py tests/scripts/test_surrealdb_service.py
 tests/scripts/test_verify.py tests/scripts/test_worktree.py` (99 controls before the final
shared cache installation refinement). `just verify --print --select store:rust` resolves the
stable validation attachment. `just service check` was **blocked**, exit75: the new installation
was not yet created. The updated release CLI build is required before explicit installation.

Whole-operation portable replay/capture is retired in favor of admitted native attachment.
Selected fine-grain kernel products retain their distinct portable consumer, partitioned inside
stable tables. Normalization attachment reconstructs required ephemeral verification owners
from charged typed reads without canonical ingress. Legacy partial graph controls use scoped
kernel readers; they do not fabricate admitted publication handles. Actual manifest/admission
journeys retain their independent obligations. Global definition corruption controls move to
explicit exclusive maintenance with ordinary native pins, then restore and revalidate definitions.
These changes do not close source findings or promote historical timeout receipts.

**Integration review corrections, 2026-10-09, Implemented / runtime acceptance pending:**
exact search occurrences carry selected payload dependencies; immutable view/family lexical
statistics replace global full-text scores before ranking limits. Publication identity checks
include endpoint, namespace, database and generation. Restore requires selected payloads to
exist in the dump even when matching rows are already installed. Whole-service cold recovery
protects private configuration, credentials, selections and coordination separately from the
logical content dump. Independent source review accepts those corrections. It exposed two
remaining lifecycle defects: permanent retirement tombstones also block legitimate recreation,
and ordinary retained contributions omit prerequisite-view retention. Both are corrected in
source with original authorization epochs/permanent retired-through watermarks and guarded
exact input-view holds. The final independent source review also confirmed corrected lexical
endpoint installation order and approved the examined source subject to runtime controls.
Fresh resurrection, stale-effect exclusion and retained-dependency controls are required before
closing Unified F04/F05/F08. No source review is runtime closure.

Additional intermediate typing checks passed: `cargo check --release -p lctx-publisher -p
cpg-core -p lctx --tests` (9.27s), and the focused serving/publisher tests check after frozen
lexical statistics. `cargo build --release -p lctx` passed5m41s before the retirement-epoch
schema correction; it is not the final installer. Stable-service tooling controls now cover
protected cold backup and exact-installation restore. Actual service installation and UP9
remain pending; source-specific commands/results are updated here after runtime execution.

**Installer/runtime prerequisite corrections, 2026-10-09:** `cargo build --release -p lctx`
passed3m28s, guard-free `just sync native` passed1m41s and `just ready` passed. The first
extension refresh had completed compilation but failed freshness while production sources
were still changing; the guard-free retry repaired that boundary. Actual new owned-service
installation then refused before schema SQL because systemd did not accept a quoted
`EnvironmentFile=` path. The corrected path passed actual `systemd-analyze --user verify`.
The next installer attempt exposed an index declared before its `content` field; corrected
declaration ordering was included in a release CLI build that passed1m10s. Retrying the
partial persistent installation then correctly refused existing declarations. The maintenance
installer now compares normalized actual definitions, accepts exact matches and creates only
missing declarations, including cache schemas; conflicting definitions are never overwritten.
The owned installation remains closed pending that installer build and runtime checks.
Private failure receipts retain each installer digest and partial-effect boundary.

The focused pure selection (`cargo nextest run --locked --release --no-fail-fast
--no-tests=fail -p lctx-surrealdb -p cpg-core -p lctx-publisher --lib -E ...`) ran7 controls:
6 **passed**, the schema snapshot **failed** on the intended payload/anchor/view changes.
Its corrected `.snap.new` was read and explicitly accepted with `cargo insta accept --snapshot
lctx_surrealdb__schema__tests__fixed_native_envelopes.snap`; the selected rerun remains pending.
This is a physical schema migration, not a snapshot-only test repair. Subsequent restore ingress
also batches by bytes and charges retained decoded canonical rows, rather than accumulating128
arbitrarily large rows. Affected tests compile (`cargo check --locked --release -p lctx
-p lctx-publisher -p lctx-serving --tests`,7.77s); actual restore acceptance remains pending.

**passed after repair:** the selected schema control on the accepted snapshot, Nextest
`e582fd4c-7cd8-4561-b5ea-5858b02254dc`,1 control in0.017s (5m30s including queued build).
The updated installer CLI (`cargo build --locked --release -p lctx`) passed9m13s including
artifact-lock queuing and the newly shared parser dependency build. The parser's function-boundary
and non-DDL refusal control also passed, Nextest `e066ef00-88ee-4b2b-9b85-ebb4c19ebc86`.
Final focused tooling selection passed123 controls in9.09s, with the four modules named above.
These passes do not establish actual installation, recovery or product runtime acceptance.

**Owned stable service and recovery, 2026-10-09 — focused Tested:** the explicit installer
created `library-context-surrealdb.service`, installation
`58abe9ea-2088-4109-95ab-8440a3749c51`, under the private host state root. Both stable
`main`/`validation` databases passed readiness with base schema
`feefd2ea2f71ea348d9fe6bc61df193200e5523ea5eb550ed60c6f8e95349db4`.
Two concurrent logical attachments shared that installation/generation; both released without
stopping the service. Under drained explicit native-client maintenance, the actual ignored
installer equality/retry/drift-refusal control passed, Nextest
`9f114c00-3a59-4a94-be6b-fb1c795bb1ba`,1 control,0.177s. Protected cold backup, default
restore validation, explicit applied restore and final readiness passed. The protected private
qualification archive remains a current evidence consumer outside Git; credentials are never
published. This was an empty-service structural recovery qualification, not every populated
semantic audit. Its exact installer identity predates subsequent production repairs; refresh
that checkpoint after the final CLI build and client drainage before claiming current recovery
readiness. Preserve the original archive as qualification evidence, not a runtime fallback.
The final scoped tooling rerun passed123 controls in4.42s; scoped Pyrefly passed. Ruff still
requires scope-end formatting/import and long-line corrections.

**Actual product controls and repairs, 2026-10-09 — enclosing acceptance Open:** the first
11 native controls ran2passed/9failed (Nextest `de075531-e6c5-4987-92f2-c1cf866b72f3`).
SDK transaction checking selected early `NotExecuted` wrappers and concealed real transaction
errors; native checking now preserves all errors, chooses the real primary and retries only
confirmed conflict-only failures. Cold backing reconstruction also still used an obsolete
payload address; it now uses the current model/content address. The next expanded13-control
run passed7/failed6 (`3cfb6eeb-9432-4885-93a5-a038f69689c7`), exposing actual invalid field-
expression `FOR UPDATE` targets. Explicit record-ID bindings corrected those targets. The next
11-control run passed10/failed1 (`3c4c16de-3c3c-40c3-b51d-8bfcbe9c4a71`,4.267s): global
backup exclusion overlapped an ordinary retirement assertion. Database-wide backup exclusion
is moving to explicit drained maintenance; ordinary shared-child retention remains concurrent.
Neither moving the disruptive assertion nor these partial passes closes the lifecycle findings.

The five root core/publication/serving controls ran2passed/3failed, Nextest
`3dd961c9-27ef-43ea-b9c1-278d52aadecd`,11.493s. Both scoped serving companions passed;
core/publication exposed logical-versus-attempt-owned producing contribution binding, while
cold transport retained an obsolete state-format assertion. Corrections are in progress.
The first four search controls ran1passed/3failed: partial fixtures derived empty binding-root
scopes instead of using their actual completed-view IDs. Corrected fixtures then exposed a
retained synthetic-row uniqueness collision and a real10s native query timeout. Fresh synthetic
negative rows now own unique payloads. The smallest diagnostic rerun attributes the query timeout
to derived-search actual-row reconciliation, before frozen lexical-statistics construction.
Indexed exact-scope preparation is being corrected; deadlines and parallelism are unchanged.
Guard-free final client refresh, remaining focused controls and coordinated UP9 `just qualify`
are still **not_run** on the replacement final source. No measured speedup or whole-plan closure
is claimed. A build attempted during an intermediate `IndexedResults` typing correction failed
before execution; it is not a product-control result.

**Focused foundation confirmation, 2026-10-09:** ordinary native controls now **passed**,
Nextest `044cfeeb-10ba-4cca-8bc3-4eb096a33c14`,11passed/1explicit-maintenance ignored,
3.787s bodies after1m24s build (`just fixture -- cargo nextest run --locked --release
-p lctx-surrealdb --test native_control --no-fail-fast`). The earlier retirement control
also passed alone (`f54c2c1e-ab92-43ab-ac93-e0ed317270d9`,0.200s). Native completion now
maps logical producer identities to attempt-owned physical IDs before establishing producing
scope ownership. Actual cold import/admission subsequently reached an obsolete full-vector
ordering assertion (`911756c3-d7a1-4ad7-861a-ee43f3c91e65`,16.648s,1failed): the control now
sorts full validated logical descriptors/bindings without deduplication; rerun remains pending.
The failed search diagnostic is `440b871b-df57-4e76-bfbc-64c48a6b0f19`,10.951s; preceding
four-control search result is `7f76ed1e-8060-4b55-8bad-4a8e91ed48e3`,1passed/3failed,26.227s.

The exact derived-selection correction resolves selected payloads once, uses indexed exact
`unit_payload` candidates and complete dependency eligibility, then point-selects actual rows.
Lexical statistics, winning/rescore and exact-vector nomination consume the shared preparation.
Fresh malformed extras remain visible to actual-content comparison when their dependencies
belong to the selected scope. Focused native and integrated consumer compile checks passed;
new lexical/vector payload indexes require explicit additive installation before runtime checks.
Base schema/service generation is unchanged; immutable executable blueprint epochs advance.
Maintenance child environments now retain attachment-owned compiler/test configurations even
when the parent exports foreign paths; its real-child regression passed1 control in0.65s.

Ordinary logical backup no longer holds database-wide retirement exclusion. Version-matched
3.3.0 source establishes one read-only export transaction across all tables
(`surrealdb-core/src/kvs/ds.rs:5562–5580`, `kvs/export.rs:557`) and RocksDB snapshot-backed
point/range reads (`surrealdb-kvs-rocksdb/src/lib.rs:783–836,994–1003`). Retired rows remain
visible to that captured read transaction. The requested publication's durable reader pin,
checked stream terminal and provisional-file semantics remain; protected cold whole-service
recovery still requires exclusive maintenance and drainage. Source support is not a measured
benefit. Final independent delta review, index installation, focused consumers and UP9 remain
pending. External commit `6b173569` captured the integrated implementation checkpoint; subsequent
runtime repairs remain on shared main.

**Maintenance recovery prerequisite, 2026-10-09 — implementation pending runtime:** the
current CLI build passed3m21s and guard-free extension refresh passed4m56s before the additional
recovery API. The producing-scope lifecycle control passed, Nextest
`6ffdfbcd-d2ad-4afe-8692-1197f6c51eae`,1 control,0.445s after4m52s queued build.
The explicit additive index installation then refused **before any DDL**: validation contained
three unreleased synthetic reader pins from earlier failed controls. No backup holds were active.
Host borrowers were zero; exact failed-command process identities were dead, their attachment
receipts released and child cleanup confirmed with no survivors. Admission and the failed
maintenance marker remain closed; an empty-view marker alone does not authorize release.

The native owner now provides explicit Root-only named maintenance reconciliation, requiring
closed admission and an asserted completed predecessor drain. It fences unresolved effects first,
then guards each exact supplied identity, retaining the epoch watermark, unrelated pins and
partial committed progress on a later refusal. The CLI exposes named repeated `--pin` and
`--backup-hold` flags plus required `--readers-stopped`; it never implicitly releases all readers.
Service recovery owns the exclusive host locks and released-command proof, with explicit database
and inventory. Its initial40 functional controls passed1.34s. Independent source review accepted
the native/CLI delta but found missing installer-config target validation in the service path;
that preflight and failure-closed ordering are now independently reviewed. Four bounded tooling
modules passed140 controls, including stale/swapped installer-target zero-effect refusal and
unknown-cleanup admission closure. Final CLI build passed5m40s, guard-free native refresh
passed2m55s, and `just ready` passed. One intermediate extension refresh compiled but correctly
failed freshness after the partial-commit correction changed production during its build; its
retry does not turn that failed freshness result into a product-control failure or pass.
The exact three-pin repair **passed** (`just service maintenance --recover` with three explicit
validation pin identities and stopped-client proof, session93088). Both databases then had no
unreleased pins or active backup holds. Additive index installation subsequently **failed** on
an existing `fn::lctx_operation_definition` declaration: main's two exact-payload indexes were
installed before refusal; validation's were not, and no new immutable executable epoch was
installed. The outer maintenance owner reclosed and drained both databases and retained its
failure marker. Shared installation now validates the complete generated blueprint before any
DDL, installs physical declarations only, and delegates executable helpers to the existing
publisher-owned immutable epoch installer. Independent delta review approved the corrected
source; `cargo check --locked --release -p lctx-surrealdb --tests` passed1.84s. Raw unversioned
library-roots/operation-definition helpers have no production callers and await explicit
maintenance retirement after the new epoch is verified.
The corrected CLI rebuild **passed**3m24s. Ordinary explicit recovery without pin flags
passed (session62286), followed by additive installation/check under exclusive maintenance
(session32520). Both databases now contain both `exact_unit_payload` indexes and immutable
epoch `fa8442e94aafcc4aa59e63c8252cff3604b37ad0467d4374100ee9c934a3f0cb`;
base generation remains unchanged. Exact reviewed raw-helper retirement passed (session12509):
only the two obsolete literal names were removed, with all four remaining named definitions
identical before/after in each database. No content DML was performed. Final `just service check`
passed, the maintenance marker cleared, and focused consumers resumed with ordinary parallelism.
Ignored maintenance controls, remaining focused consumers and UP9 remain **not_run** on final
source; no readiness waiver or blanket pin release bypassed the refusal.

**Build-storage interruption, 2026-10-09:** the physical-blueprint control passed
(`88656f49-16fa-448d-8671-c832256d0572`,1 control,0.005s after5m32s build).
The subsequent seven-consumer no-run build failed101 (session43190) when the entire shared
`/home/paul/.cargo/build/library-context` directory disappeared during active rustc output;
errors were missing output directories, not product assertions or disk exhaustion. Available
space rose from25GiB to225GiB. The queued focused consumer run acquired the lock afterward
and began reconstructing intermediates normally. Persistent service data, the current CLI,
profiling captures and the protected recovery archive were not removed by this task. Ordinary
migration controls remain queued; enclosing qualification stays Open.

**Post-cutover focused consumers, 2026-10-09:** the reconstructed consumer build passed16m22s;
Nextest `7eded87a-3753-41f6-97d6-f2aae91a652b` ran7 controls,3passed/4failed in105.961s.
Native admission/attachment ownership, independent cold transport and frozen-view BM25 passed.
Failures exposed a retained synthetic occurrence-key collision, SDK-native Object decoding through
the serde result path, an attempt-abandonment20s timeout after successful hydration assertions,
and a physical/logical completed-owner key mismatch at publication admission. Fresh logical fixture
keys and native prepared conversion are corrected; workspace ownership now uses logical spec
identity and retains full descriptor comparison. The existing admission control additionally
freezes fresh and reused products. No uniqueness, completeness or deadline rule was weakened.

The seven migration controls (`5a2b47cc-4dc3-4a1e-aac8-47ca8ed321df`) ran4passed/3failed in3.360s,
after18m40s queue/reconstruction. Remaining failures were missing binding roots before portable
export, a negative freeze test incorrectly demanding successful cleanup despite its retained
primary failure, and top-level `RETURN` skipping the intended later sleep/error. Controls now
establish actual binding authority, inspect retained failure plus confirmed cleanup, and execute
the intended later terminals. Post-run inventories in both databases had no unreleased pins,
active backup holds or pin-owned holds; no additional reader release was needed.

Static cleanup review found unindexed owner-hold scans and a global product/contribution
dereference. Native correction adds exact owner/attempt/contribution indexes and scoped two-stage
cleanup while preserving admitted products and unrelated holds. These declarations intentionally
advance the exact base-schema identity. The current CLI's exact bytes are retained as a named
maintenance migration asset: predecessor close/drain must precede candidate installation under
one host-exclusive lease, with database-specific failure closure if only one scope migrates.
Native source review accepted the examined correction; concrete migration review, runtime reruns,
cross-worktree reuse, disruptive maintenance controls and UP9 remain pending.

The concrete maintenance procedure and its `store init --keep-closed` support passed independent
static review. Installation retains closed native admission throughout; only the outer checked
maintenance owner reopens after both databases install, check and drain. A failed initializer
retains its completion uncertainty separately from observed gate closure, and mixed-schema
cleanup uses exact pinned predecessor/candidate binaries per database. Exact canonical index
readback precedes reopening. Runtime migration **passed** (`62048`): both stable databases
advanced from `feefd2ea…` to `464537d3…`, with service generation unchanged, all three canonical
index definitions confirmed and checked admission reopened. Candidate CLI build **passed**
(`87912`,1m53s); affected native/publisher/serving test compile checks **passed** (`32620`).
Focused reruns and UP9 still have their own pending boundaries.

Post-migration ordinary native controls ran12passed/1failed (`99a90a7a-6d37-4b1d-8a79-95db085d77a3`):
all prior controls and prepared-native conversion passed. The new cleanup control repeated
abandonment after the first call invalidated its session; exact cleanup through a separate
authenticated client repaired the test, and its rerun passed (`f6b95e0a-f59b-4c99-bfe5-66c39fde6573`).
Actual index plans, admitted retention and unrelated holds were checked.
The seven consumer rerun (`4deda5a0-0281-437a-8c73-3929b2550f02`) ran5passed/1failed/1timed_out.
Publication reached graph admission and exposed a remaining nominal-ID/payload-ID assumption
in the canonical reader/header lookup. The search control now executes its94 cold audits and
hit the unchanged300s case deadline. Its cases are being separated by independently isolated
corruption responsibility, retaining the assertions and ordinary parallelism.
The seven migration rerun (`08edc7b0-c7a9-498a-838e-0e353ac28352`) ran5passed/2failed.
SurrealDB3.3's actual SleepPlan requires Root edit authority; pending cancellation moves to an
explicit maintenance control. Projection export exposed a false1.9GiB charge for a declared
1MiB opaque chunk: physical ordering multiplied the entire encoded frame by Value-node size.
The correction charges frame plus native containers and uses the existing64MiB portable-row
contract for charged singleton spills; ordinary compact windows retain their bounds.
These are current correction obligations, not successful enclosing acceptance.

The canonical payload/nominal identity repair passed its pure tampering control
(`82ef83d9-9db7-4e89-b53a-314263d6fa41`). Reconciliation now orders exact physical
anchors and payload IDs rather than an unprojected anchor dereference; the actual native
sparse-corruption/terminal control passed (`34ef090e-7bf8-42ce-8d56-583cbdf0d968`,21.290s).
Ordering/acknowledgement controls passed17 (`b0b150a7-b159-490b-9866-48ead064935e`).
Portable ordering now composes headroom for three merge heads, acknowledged input and one
singleton frame, charging actual reservations. Encoded-frame and native-row bounds remain
separate; this does not promise that every maximum native row has an admissible encoding.

The next focused publication/search/projection run **failed**
(`7c1e8b8f-c5a8-4b5b-88f4-40fffb1ca96d`,14failed,153.756s after3m44s build).
Projection export reached its intended immutable collision, but its assertion expected a
retired local error label rather than the checked SDK throw. Publication exposed a near1MiB
portable frame whose retained key/entry metadata exceeded the ordinary run threshold; the
explicit portable singleton exception now uses total retained bytes. Its revealing boundary
control and all17 ordering/acknowledgement controls **passed** on the corrected source
(`04168042-3b7c-4ce4-a42f-d7e7dc36fe5a`,0.097s after2m18s build).
The publication runtime rerun is pending. All12 independently isolated search controls timed out during
canonical lowering. Static diagnosis found repeated global entity/assertion scans with inline
view-membership expansion. Indexed point selection and exact prepared-result terminal dispatch
are implemented using existing indexes and passed independent static review plus the three-crate
test compile check (`32426`,4.02s). The same point-source preparation serves CLI/evidence relation
reads, including compiler records. New runtime controls cover real ingress aliases, separate-view
same-nominal revisions, entity/assertion result positions and preparatory refusal with explicit
typed-stream drainage. Their runtime execution is pending exclusive maintenance completion.
Alias preparation collects the exact selected aliases before the final family/type filter; no
claim of family-only preparation or measured speedup is made. No parallelism cap or deadline increase is used;
disruptive maintenance controls and final-source UP9 acceptance remain separate pending work.

The compiler-state correction also passed compile checks (`20780`,2.59s) and independent
static review: hashing, export and audit each capture exact bindings/dependency closure once
per operation, use indexed candidate/point sources and check all prepared terminals. The
extended `completed_state_is_exact_binding_closure_in_current_and_cold_owners` control covers
unbound exclusion, changed-binding recapture and preserved cold inventory; runtime is pending.
NativeSession explicit close now drains active requests and service clones, releases its reader
pin, then invalidates the client. Concurrent closers share the same terminal result. Compile
check (`52219`,10.66s), Python syntax check and independent static review passed; its actual
pending-call/two-closer/pin-release Python control awaits the final native refresh.

The eight explicitly admitted maintenance controls finished **6passed/2failed**. Backup exclusion,
named reader reconciliation, extra-original-envelope refusal, read-only scope-definition drift,
installer equality/drift and explicit pending-poll cancellation/three-terminal/EOF drainage passed.
Analyzer drift failed during publication admission before mutation (`55df26e0-61df-4397-b3bf-1a1367e19e8f`,
121.452s); executable-epoch drift timed out during full catalog fixture construction before
publication/mutation (`e393b948-c5ae-4ca0-8935-078efeab1334`,300.026s). Both intended assertions
remain unexercised. The frozen private receipt is
`maintenance-eight-controls-fecfd3ef799a4786bc8e0a47f4153a04.json` under the owned service state root,
SHA256 `bc560c2a3273662248633958f6bccff3c034777af8acfb1af0d4ee5ed01100ca`.
All eight restored exact definitions and confirmed drainage; zero borrowers, pins, backup holds
and unresolved effects remained. `just service maintenance --recover` **passed** (`87111`),
revalidating both databases and reopening admission without forced reader release.
The executable-drift control will consume an exact retained publication from the independently
qualified ordinary MCP producer; it need not rebuild that prerequisite merely to mutate/check
its pinned definition. The full ordinary catalog journey remains required by UP9.

Final-source preparation: scoped `just turn-end --paths ...` regenerated ADR/Hakari (no feature
change), formatted72 task Rust files and three Python files, excluding concurrent acquisition
owners. Mechanical service-script lint corrections preserve messages and conditions. The
retained-view drift control passed `cargo check --locked --release -p lctx-serving --test native_journey`
(`73003`,1.49s) and independent static review after strict SDK Value/Object decoding. The private
F01 wrapper passed syntax/static review and verifies explicit main interpreter/native-artifact
provenance; independent worktree-built clients are outside that claim.
The corrected17-control run (`20261010T034621.579Z-841e29`) selects the12 `streamed_` cases,
publication, projection backing and the three new reader/state controls named above. Its
`cargo build --profile release --locked -p lctx --bin lctx` **passed**,5m27s. Consumer runtime
finished **15passed/2failed** (`9648509a-f112-41d1-ac8f-5aacaba3ba93`,105.815s after2m30s
test build). All12 search controls and the three reader/state controls passed under ordinary
parallelism; no native query timeout occurred in that selection. Projection's exact typed THROW
assertion omitted the SDK's `An error occurred:` prefix; the pinned source/SDK control confirms
the prefix and the test correction compiles. Publication progressed past admission but exposed
`conflicting nominal candidate pointer` in Normalized compilation: atomic-scope candidates are
sorted before exact completed-view filtering, so unrelated same-nominal payload revisions can
falsely conflict. The native owner is correcting that access boundary and adding a revealing
actual control. Final native refresh, maintenance8, F01, populated recovery and assembled UP9
remain unqualified.

**2026-10-10 focused follow-up:** run `20261010T040306.825Z-fa0912`, Nextest
`efd6fb96-2140-4c8e-941a-a8c4206cb39c`, finished **2passed/2failed** in153.235s after2m44s
build. Projection backing and existing same-view revision-conflict refusal passed. The new
atomic exact-view control initially supplied hex strings instead of native reference arrays;
its fixture now derives values through the production batch codec. Publication passed compile,
export and cold verification, then failed final publication seal with a10s native query timeout.
The exact-view correction preserves the strict downstream membership/collision validators.

The phase-logged rerun `20261010T041013.131Z-c15f1b`, Nextest
`ee891bfb-9f1f-4a42-a1e2-0e974e4c6f16`, finished **1passed/1failed** in167.128s after18.16s
build. The corrected actual atomic exact-view control passed. Publication failed specifically
in `publication_final_reconciliation` after10002.8ms; state hashing completed beforehand, and
publication marker/realization phases were not reached. Static review identified global canonical
and outgoing-role scans with repeated view expansion still in reconciliation. The native owner
is migrating those reads to the shared exact-node preparation and indexed role sources while
preserving full canonical/family/role reconciliation. No deadline or parallelism change is used.

The exact-node reconciliation rerun `20261010T041912.896Z-a41f00`, Nextest
`bfe0fe8d-f6b9-46ef-a547-99700cba9999`, finished **1passed/1failed** in172.183s after1m54s
build. The extended sparse-corruption control passed, including independently missing and
unexpected participant/reference rows followed by restored positive readback. Independent source
review approved the examined exact-source/alias/prepared-terminal patch. Publication's final
reconciliation passed3406ms, realization passed14ms and its marker passed7490ms. The subsequent
attempt cleanup exceeded the unchanged10s query deadline; the structured failure preserves the
acknowledged unselected publication identity. Abandonment repeats the cleanup timeout. Indexed
owner enumeration still feeds an unbounded transactional delete in `control::close_attempt`;
bounded, fenced, retryable cleanup is the next correction. This receipt does not establish whole
publication success or a measured speedup.

The bounded cleanup correction fences the exact attempt/epoch before deleting indexed owner-hold
and product windows. Unadmitted products drain before attempt roots, so interruption and concurrent
retirement cannot erase their discovery path; admitted products remain retained. Existing frozen
and maintenance-fenced states remain terminal. Independent review identified and corrected the
initial release-before-product race and a maintenance-state compatibility omission. The extended
actual control covers1025 target holds, partial cleanup, late-write refusal, repeated close,
admitted-product retention and unrelated-reader preservation. Final compile check **passed**
(`76351`,1.52s); runtime is pending. The precursor14-control build
`20261010T043054.710Z-5363ff` was **cancelled / not_run**, exit143 after40.8s, before controls
started, to incorporate those review corrections. No test failure or pass is attributed to it.
Lexical statistics readback also replaces a remaining shared-document scan with indexed nomination
of all actual scoped members plus exact document point reads. Its full sorted comparisons and
prepared terminals remain; independent static review and compile check **passed** (`67261`,1.32s).
Runtime reruns include all12 isolated search controls rather than inheriting their earlier passes.

Run `20261010T043334.260Z-b5c688`, Nextest `75697021-f7fe-4e67-82cd-34dd17e4c517`,
finished **11passed/3failed**,216.599s after2m01s build. Multi-window cleanup passed, as did ten
search controls. Publication completed sealing, cleanup and fresh reader readback without a
native query timeout, then listing refused a malformed retained-control placeholder marker
(`Codec: Invalid character 'n' at position0`). The generic ownership test had inserted
`handle='native-owner-control'` into the authoritative publication table. It is being migrated
to a generic guard root, retaining its prerequisite/product retirement assertions; exact leftover
synthetic roots need checked native retirement under drained maintenance. Listing remains fail-closed.

Both occurrence controls failed because their fixed `family=3` mutation can equal an already
Source-family witness. The test now chooses another valid family, asserts actual physical change,
retains both refusal/read-only checks and restores witnesses before negative assertions.
Independent static review and scoped compile **passed**. Their corrected runtime rerun
`20261010T044103.370Z-32fe0e`, Nextest `7aa6cc3c-2b89-4be4-9b95-304698ad8eb5`,
**passed2** in35.108s after29.71s build. These are test-precondition corrections, not a weakened
reconciliation contract. Guard-free `just ready` **passed**, rebuilding and installing the current
native extension before readiness checks; no live client was replaced. Final-source
MCP/maintenance/F01/recovery/UP9 obligations remain pending.

The one-time exact placeholder-retirement wrapper and temporary native helper passed independent
static review. Its first maintenance invocation **failed before native helper execution or content
effects**, at the host check for the sole current fixture attachment. Outer maintenance retained
closed admission; the daemon remained active and ordinary borrowers drained. The invocation had
nested `just fixture` inside service maintenance, which already creates an attachment. Exact-one
proof correctly refused the resulting two attachments. Standard `just service maintenance --recover`
**passed** (`65280`) without forced release. Retry uses the direct maintenance-owned command;
fail-closed publication listing and exact-one attachment proof remain unchanged.

The direct repair **passed**, Nextest `27b4b2d9-8cff-41c1-9e0d-028a90dba69d`, one ignored
maintenance control in0.438s after5.23s build. The exact old synthetic publication
`82a7c62e89415bafc18e9ed280d9363d7543a4e0bb92f7ca1f2b9586905fdbd3` was retired through
native terminal-attempt closure/reachability; no raw deletion or listing filter was added.
Private row/source/progress evidence and terminal receipt remain under service state
`native-owner-marker-repair-2188c8423e3245cbaf1c264ee0e590a9/`; wrapper SHA256
`4f0c9e42f3a6d038f9ba2551753e8ebe95ecd4f0e8119255744762142b904f54`.
The wrapper verified the requested marker absent and native admission closed before outer
maintenance checked drainage/readiness and reopened both databases. The temporary helper is
removed from repository code (post-removal compile **passed**, `81b37b`,0.49s); the generic native-guard fixture retains actual product
and prerequisite-retention assertions. Focused publication and final UP9 acceptance remain pending.

Final acceptance source audit identified two missing direct cases: concurrent ordinary imports of
the same dump (TF01), and the committed shared-effect reconciliation branch after controlled
acknowledgement discard/session loss (Unified F05). Existing actual delayed transactions,
pin protection/retirement/resume and source inspection cover their separate lifecycle branches;
they are not evidence of arbitrary process death or a directly forced pin/retirement race.
These revealing controls are being integrated before final qualification. The two existing
compiler reuse matrices also now exercise removal of an inserted declaration, comparing retained
attachment against independent cold compilation and the original populated result; compile
check **passed** (`30191`,4.82s), runtime pending. No new runtime cap or production hook is added.
Independent source review accepted concurrent ordinary same-dump restore, its fresh mutable
owner checks, independent audits and unchanged selection bytes, plus both-profile deletion
equivalence. Their runtime evidence remains pending. Unified F03's explicit distinct-publication
during live catalog cursor scenario is also being added using a tiny Facts-only second input;
existing same-handle restore and foreign physical candidates do not establish that exact trigger.

Focused run `20261010T045427.832Z-9dd5df`, Nextest
`674ea14a-019b-4ec2-93a9-54977c7ec5a2`, finished **1passed/1failed**,232.206s after the
final CLI build passed in1m26s and test build in4.82s. The corrected generic-root retention
control passed. Publication passed sealing/listing/audit/readback, then its raw test-side
`entity WHERE id IN (...)` selection timed out at10s. This precedes the newly added concurrent
restore assertions; that binary predates those test-only extensions. The test now nominates
physical candidates through the exact pinned-view index and point-reads the entity, checking
both prepared terminals and independent selected membership before immutable collision mutation.
Compile **passed** (`3f5520`,0.47s); runtime remains pending. No production timeout or test deadline changed.

Final bounded source review accepted the correction tree at `75e8e5bf` plus the examined changes.
It found no remaining executable ordinary scratch-database provisioning/teardown, database
sealing as view authority, or canonical payload-replay route in inspected crates/scripts/Python.
The existing qualifier covers all eight profile/frontier admission/export combinations, both
reuse matrices, native/publisher/serving, Python sessions and applicable leaves. The approved
Facts-only B/live catalog A extension compiles (`native_journey`,0.86s) and preserves the exact
saved cursor, operation packet, attributed evidence, A handle/selection and epoch while B is
published; structured cleanup retires only B. These are source/compile outcomes, not runtime
closure. Final focused rerun, fresh MCP, maintenance8, F01, populated recovery and UP9 remain pending.

Run `20261010T050208.462Z-b150ac`, Nextest `c13d499e-998b-428b-b12e-897f876e0551`,
finished **1passed/1failed**,256.086s after1m26s build. Controlled committed-acknowledgement
discard/session-loss reconciliation passed. Publication's indexed point selection passed;
the omission helper then rejected a zero-row parser frame. `DataDump` intentionally emits such
frames for ignored derived records. The test now accepts bounded empty/multirow frames, filters
each actual row and requires an actual omission from every target table before the unchanged
negative restore check. Compile (`54ce2b`,0.69s) and independent review **passed**; runtime pending.
Guard-free `just ready` **passed** (`439667`); native extension already current.
Installer stabilization initially **blocked before transfer** when actual native drain found
pins left by two failed publication test runtimes. Exact read-only inventory, prior zero-pin
baseline and confirmed dead run owners/empty descendant cleanup grounded named recovery.
`just service maintenance --recover --reconcile-database validation` with the two exact
`--reconcile-pin` values **passed** (`91881`); no unknown reader was released. Inventory
`failed-publication-pin-inventory-b7be5d2abd6b425f9978b068a9ceb482/before.json` under the owned
service state retains both identities and306 holds each, SHA256
`415075bf07caf1f45b49219b5e606390b06a9e6fb5dfb4e10196110f6888945e`.
The set is attributed to the two failed controls; individual pin-to-run pairing is epoch-order
inference, not PID/time metadata in native rows.
`just service maintenance --stabilize-installer` then **passed** (`57371`),2026-10-10:
checked executable transferred outside the checkout to service-owned `surrealdb.tools/lctx/`
with SHA256 `a1732564eb34aab4d0df8b64fd473f0590885120eb1f1b572ff12f84fbf87d58`.
Both database checks/drain/reopen passed, generation/schema unchanged, admission open and no
borrowers. Publication assertion cleanup is being made awaited before runtime exit; production
best-effort ReaderPin Drop and unknown-effect policy remain unchanged. The reviewed implementation/correction tree is committed in `4a13db4b`;
UP9 remains open. Publication assertion cleanup now awaits close and invalidation of all three
known readers outside the caught assertion future, preserves the primary panic and reports
secondary cleanup failures. The same finalizer's contained-panic control independently observes
the exact durable pin released while its reader object remains alive. The excess-derived helper
borrows the existing exact viewer rather than acquiring another pin. Compile **passed**
(`cargo check --locked --release -p lctx-publisher --test publication`,0.51s) and independent
bounded source review accepted the change; runtime pending. Abrupt test-process termination is
outside this assertion-panic guarantee. Two compiler test targets now nominate canonical physical
rows through indexed contribution membership before point reads/updates, preserving all tamper,
restoration and terminal checks; compile **passed** (`7d0a2c`,0.40s) and independent review accepted
examined source, runtime pending.
The publication settings nonce is retained to isolate final-retirement controls
under concurrent copies/worktrees; a stable shared handle alone would undermine that isolation.

Focused publication run `20261010T052010.244Z-dbf748`, Nextest
`81a0b388-3ef7-413a-bd44-b640b0033e5a`, **failed**,251.517s after36.48s build.
Publication sealing/readback and the contained assertion-panic exact-pin release check completed.
The omission helper then received a definition mismatch before its intended missing-payload
refusal. Exact3.3.0 core `kvs/export.rs:265–274` emits field definitions with `OVERWRITE` for
idempotent administrative reimport; installed INFO definitions omit that application modifier.
The ordinary data-only decoder retains it, causing legitimate exported metadata to compare
unequal. The correction is one grammar-owned comparison normalization of application kind only,
shared by imported and installed definition metadata; definition bodies remain exact, unsupported
administrative forms still refuse and no imported SQL executes. The correction passed compile
(`cargo check --locked --release -p lctx-publisher --lib`,0.60s), independent bounded review and
all eight actual pure controls: `cargo nextest run --locked --release -p lctx-publisher --lib
-E 'test(backup_import::tests) | test(restore::definition_tests)'`, Nextest
`52a22b51-978d-4fd6-8233-89b0fb20ec0b`,0.005s after5.50s build. Controls cover all five
allowed declaration kinds/modifier roundtrips, literal retention, semantic drift, valid-but-forbidden
administrative grammar and both imported/INFO comparison paths. Native publication rerun pending.
The run cleaned its attachment and no ordinary borrower remains. A subsequent
`just service maintenance --native-clients -- true` **passed** (`20297`), draining/reopening both
databases without named reader recovery. This establishes the awaited cleanup path on the real
later assertion failure, not abrupt process termination. Concurrent restores and derived excess
assertions were not reached. Earlier failed composites remain failed.

Publication rerun `20261010T052948.598Z-5683cb`, Nextest
`c1e16573-3b6b-47ba-9f90-09e5302a09e4`, **failed / timed out** at300.024s after43.68s build.
The corrected definition comparison reached selected missing-entity refusal. The second omission
case timed out during local decode: retained source dump `/tmp/.tmpTDawd0/snapshot.surql`
is597,992,608bytes, while the interrupted `/tmp/.tmp4XYgiC` spool already contains214,449
memberships and roughly94MB of contribution metadata. Both directories have the named current
consumer of this correction diagnosis/qualification and remain protected; no broad cleanup.
The immediate amplification is native table-only export including unrelated shared-store data,
then repeated grammar decoding/re-encoding/spooling before selecting the requested graph.

A bounded independent design judgment accepts selected closed-content compaction under
ADR-0143/UP6: retain the native engine's single whole-main read snapshot and requested reader pin,
prepare the claimed exact dependency closure from that completed dump, then publish a separate
completed compact logical stage containing only the requested manifest and required data.
The shared preparation is not admission authority; ordinary restore freshly decodes/prepares its
own input before attempt creation, then still performs typed ingress/full cold admission.
No ambient database payload fallback, schema/terminal waiver or larger deadline is selected.
The engine export's global transfer/parse cost remains an explicit limit of this initial route.
A separate bounded cleanup refinement uses the terminal attempt's exact original epoch for
cleanup receipts, preserving intent/execution fences and reconciliation without allocating a new
global installation epoch per page. That private terminal-owner correction passed compile
(`cargo check --locked --release -p lctx-surrealdb --test native_control`,1.35s) and independent
bounded source review. Its focused multi-window receipt/retention control **passed**:
`just verify --select store --nextest-args "--test native_control -E
'test(indexed_abandonment_removes_only_unadmitted_products_and_owned_holds)'"`,
run `20261010T054722.768Z-c17257`, Nextest `8a570cd0-ed03-4087-b554-4d03cb2a9d59`,
3.007s after1m04s build. The1025-target partial-cleanup, late-write refusal, admitted/unrelated
retention and original terminal-owner epoch assertions remain. The receipt observations are
exact-owner filters over shared effect history, not indexed access or a measured speed comparison. Backup compaction remains in progress; no measured benefit is claimed.

Timeout cleanup confirmed empty descendants and released attachment. Exact inventory
`timeout-publication-pin-inventory-af43660aa09f4c42a685c8da0b97828e/before.json` under owned
service state, SHA256 `54f68618af4e7a662343aeacfd04d1cfe6b80cc4426e3f9bd27d4529388357bb`,
identified only pin `c514528cc29bb8c6b9850115a143d945b1518ec1cc93e7f13266c1cffce8f3ae`:
305 exact protected views and their one publication,0 backup holds/borrowers. Attribution uses the
actual zero-pin maintenance20297 baseline and sole subsequent native run, not heartbeat age.
Independent review approved only that identity. Standard named recovery **passed** (`73287`),
draining/reopening both databases without changing generation/schema. Protected publication
metadata is retained beside the inventory, SHA256
`52562a2782517812cff7e8603931c9e873d91249f993c728065f5af7bcbd3c50`.

**NE0–NE9 execution in progress, 2026-10-09, baseline `ebf1328e`.** ADR-0141 records the
operator-accepted complementary target. Exact bindings, membership access, typed replay,
shared admission, wake-driven handoffs, scoped completion, stable flights and explicit
observation are being integrated; source work alone does not close the findings above.

**passed:** `just verify --select store:rust --nextest-args "--test native -E
'test(prepared_streams_share_authenticated_session_and_keep_it_alive_through_drainage)'"`,
run `20261009T194456.871Z-78a0ae`, one native control passed. It verifies same authenticated
session across concurrent reads, initiator drop, late failure and drainage. The initial compile
exposed two unsupported `Value::from(&str)` test conversions, corrected to `Value::String`.
This does not establish the historical authentication failure's cause or integrated NE2–NE9 acceptance.

**passed:** `cargo check --locked --release -p cpg-extract -p cpg-core -p lctx-surrealdb
-p lctx-serving --tests --message-format short`, 2026-10-09. A later native descendant-edge
refinement and new controls still require the final-source runtime build.

**failed / corrected source, rerun pending:** pre-review focused library run, nextest
`1a537221-aefb-4c2d-aef9-6cc9ad6f3ca9`: 36 controls, 32 passed and four support-admission
planning failures. A temporary-directory dot entered an unquoted generated alias; all
non-identifier characters are now sanitized. Bridge/cancellation, serving-flight and prior
scope/product passes retain that source boundary. New independent-review fixes require their
new revealing controls, not inherited passes.

**failed:** focused workspace run `20261009T200746.445Z-b2c242`, release build13m33s,
65controls in40.511s: 53passed,12failed. Seven failures share generated table-name casing:
DataFusion normalized registration while quoted SQL preserved uppercase. Aliases now use
lowercase ASCII and a new control checks both actual lookup paths and cleanup. Two inherited
workspace controls now require the exact typed primary conflict, unchanged completed views and
Terminal/Confirmed drainage with retained failure, matching the existing sticky native contract.
Independent source review accepts these expectation corrections. Native controls corrected a
malformed empty-view descriptor and the legacy EXPLAIN shape: a disposable3.3 probe confirms
`SelectProject → IndexScan`, `member_keys` and the exact scalar prefix. A selected-read RPC
cancellation remains unresolved, with added branch context and no weakened finality assertion.

**failed composite; corrected core/extraction/serving boundaries passed:** run
`20261009T202627.015Z-6acf27`, release build4m02s, Nextest
`8b3d5067-71c3-4333-b943-01ac5782a039`: 66controls,65passed,1failed,73.606s bodies.
Shared admission, generated-alias lookup/cleanup, typed product preparation, bridge wakeups,
stable serving flights, Selected/Complete observation and scoped completion passed. The large
exact-key/atomic-field native selection still failed with canceled RPC/missing terminal.
Isolated reproductions `20261009T203224.691Z-49d6ed` and
`20261009T203450.873Z-ecb186` also failed without concurrent selected tests, in2.222s/1.846s.
This establishes a real transport/finality failure, not its cause or a reason to cap parallelism.

**Implemented / source-inspected; actual transport rerun pending:** the local SDK3.3.0
backport retains gRPC query/export/subscription streams from application End through checked
transport EOF and preserves late status/error frames. Independent review found no material
source defect in the correction. SDK3.3.2 retains the upstream gap. PSE-arrow's current WS SDK
still buffers queries; adopting progressive server `query_stream` requires a separate adapter,
cancellation and backup integration. No PSE files, endpoint or dependency versions changed.
Locked offline metadata passed with only the SDK source/checksum lock entries replaced by the
vendored source. Helper-unit invocation `20261009T204248.635Z-5d1c56` was **blocked** because
Cargo refuses dev-dependency tests for a non-workspace package; no helper pass is claimed.
Actual patched native controls **passed**, `20261009T204438.092Z-2dfd57`, release build5m46s,
Nextest `20dda8bb-387a-4713-8203-c612f5cc77a5`:16passed in6.534s. This includes the repeatedly
failing large exact-key/atomic-field control, actual scalar-index plan, authenticated session,
local producing scopes, unrelated-reader separation and final global drainage. It supports the
physical-terminal correction at this boundary, not universal attribution of historical timeouts.

**failed / corrected source, rerun pending:** explicit full-output extraction determinism,
`20261009T205101.852Z-74e7dd`, Nextest `f65d4ec5-db15-4a5b-b8dd-e28392997742`, failed
in0.924s during captured-source preflight. Two differently labeled acquisitions contain the
same immutable input, context and configuration. `captured_sources` now deduplicates only full
binding equality after sorting; same-input context/configuration disagreements still conflict.
The unchanged shuffle/relocation/transfer/profile assertions rerun in
`20261009T205258.138Z-fd8ce2`. Independent bounded source assessment accepts the correction:
input identity excludes acquisition labels, both acquisition/provenance rows remain, and full
binding equality cannot merge disagreeing context/configuration. The exact rerun **passed**,
Nextest `482a7087-d592-44ff-b6f5-158f210df782`,127.744s bodies after53.50s release build;
complete output agrees across both profiles, input order/relocation, repetition and transfer sizes.
Guard-free final-source `just ready` **passed**, `20261009T205626.201Z-cab667`, rebuilding the
native extension in1m11s and verifying the pinned tools/environment. NE9/GK7/GR6 assembled
release qualification `20261009T205753.562Z-7c4894` **failed / canceled partial**, exit143:
stopped before filesystem exhaustion at2.9GiB free. Model, analytics and flow passed.
Extraction **failed composite**, Nextest `a76d7ec6-471e-452d-8f25-c07e8f666676`:69controls,
68passed/1failed in176.256s bodies; no timeout. The failing coverage control identifies native
Pyrefly Exports projection discarding its known
parse-error cause in favor of generic OutsideProviderModel. A bounded correction is prepared
outside the production tree while qualification continued, then integrated after cancellation.
Core built in2m43s; Nextest partial219/479 in395.752s retained163passes,7assertion failures,
17timeouts and32SIGTERM cancellations, with260not_run. The reported39failed includes those
32cancellations; they are not assertion failures. The two2231-byte final-charge failures retain
the native store's legitimate compact preparation; controls now drop that final owner before
unchanged zero assertions. Fresh importer controls preserve exact detached tamper refusal and
positive admission, avoiding reuse of an already frozen content lane. Other failures include
DataFusion per-partition sort reservation exhaustion at128MiB, cache-off/cold completed-state
digest mismatch and two missing-local-tokenizer prerequisites. The17unchanged300s timeouts
remain failures; their relation to sort/preparation amplification is unproven. Remaining
CLI/store/serving/MCP/oracle/tooling/leaves were not_run. NE-I01–NE-I04 above own the source
corrections. The operator restored40GiB after free space fell below1GiB; focused compilation and
correction controls resumed. Independent source review accepts the dedicated capture boundary.
The analytics failures were traced to the test ContractEmbedder omitting delegation of its
existing in-process fixture tokenizer; delegation now preserves tokenizer availability even
when the separately simulated embedding service is unavailable. No Qwen assets are needed.
Optional capture now uses a dedicated checked contribution scan
without persisting singleton view metadata; arbitrary public views still require registration.
Native Python is stale after these corrections
and must refresh in a guard-free interval. BC3 candidate qualification stays separate and pending.
No universal latency attribution or numerical benefit is claimed.

**failed composite / eight focused controls passed, 2026-10-09:**
`just fixture -- cargo nextest run --locked --release --workspace --lib --test compiler_views
--test compiler_artifacts --test extraction_contracts --no-fail-fast --no-tests=fail -E …`,
run `20261009T212411.991Z-b4297a`, Nextest `5042a823-055a-4d8c-b4f3-281786504634`.
Release build4m24s, bodies299.669s:8passed/2failed. Exact argv and results remain in that run.
Native failed-preparation recovery, read-only singleton identity/refusals, both hydration-owner
controls, explicit provider coverage and physical partition/empty-authority planning passed.
The original transfer7/128MiB graph equality passed292.629s; fresh-importer intact/tampered
export verification passed299.669s. No budgets/deadlines/parallelism changed.
The added statistics control incorrectly constructed its native owner outside Tokio; its test
annotation is corrected, rerun pending. Removing SyntaxSupport assertions while updating only
the public family hash correctly refuses earlier at completed membership backing/visibility:
the retained native memberships still point to removed graph rows. Separate coherent detached
semantic-negative construction is implemented through ordinary typed ingress/completion, actual
cold state import/audit, restored bindings and shared semantic admission. The exact unsupported-support obligation is
retained rather than weakening its expected reason. Final both-profile cache matrices, analytics
wrapper controls, native Python refresh and remaining assembled boundaries are pending.

**failed composite / three corrected controls passed, 2026-10-09:**
`just fixture -- cargo nextest run --locked --release -p cpg-core -p lctx-surrealdb --lib
--test compiler_artifacts --test compiler_analysis --no-fail-fast --no-tests=fail -E …`,
run `20261009T213536.960Z-54edcb`, Nextest `4a44c6d6-625d-45f7-92ab-516a46fc9864`;
release build3m12s, bodies300.170s:3passed/5timed out, no assertion failures. Statistics passed;
physical graph/state disagreement passed158.681s; coherent cold native import followed by exact
unsupported-support semantic refusal passed193.467s. Independent source review accepts the
separated obligations; no captured-producer/frontier authority is claimed for neutral lineage.
Both cache matrices and the three analytics controls timed out at unchanged300s. Catalog reached
its cold attempt's checked semantic admission; Behavioral reached checked admission in its first
cache-off attempt. Neither completed the required equivalence matrix, and the tokenizer wrapper
controls remain runtime-unqualified. Timeout causes and measured benefit remain unestablished.
SDK backport committed as `18ae9076`; guard-free native Python refresh
`20261009T214454.980Z-f24fba` passed in18s. `just verify --rerun
20261009T205753.562Z-7c4894` resumed failed/interrupted/unrun assembled release boundaries in
`20261009T214543.898Z-81f4a8`; preceding successful independent boundaries were not repeated.
Its `providers:extract` boundary **passed**, Nextest `74c120a2-c1f9-4b0c-bf57-8035033981f2`:
69passed/85skipped,272.776s bodies after1m14s release build,349.5s total. This reruns the
earlier failed coverage boundary without reducing its selection or changing runtime limits.
The compiler boundary and remaining qualification are still running.
The compiler build completed in1m33s and selected481controls. Its pre-correction binary found
an8MiB nominal-closure sort reservation failure: preparation silently changed the control's
requested partition target from1to2, introducing two non-spillable sorter merges and a final
sort-preserving merge after compute batching was decoupled. The undocumented floor is removed;
ordinary available-CPU targets remain unchanged. Independent pinned DataFusion55.1 source review
accepts one-partition join/distribution semantics and the removal. Existing small-memory/full-edge
assertions remain, with added caller-target preservation assertions. The affected control needs
a new binary/rerun. The same full boundary is also retaining300s frontier/analytics timeouts;
their cause is not established by this separate sort correction. Current files must not be
mistaken for the earlier compiler test binary's source boundary.

The same pre-correction compiler binary exposed a separate exact binding assertion:
`base_execution::behavioral_read_channels_preserve_native_observation_and_negative_inventory`
failed287.063s during compilation with `binding set eligibility changed`. Source inspection
finds producer enumeration order differing from independent attempt-ID order while both hash
the same closed membership. NE-I09 canonicalizes a charged copy of IDs before assessment;
all exact content, eligibility, coverage and negative-domain checks remain. Its pure permutation
control **passed** in `20261009T221009.683Z-ef2bcc`, Nextest
`41582029-03ac-4f35-80d3-1a175f502011`: `cargo nextest run --locked --release -p lctx-model
--lib --no-fail-fast --no-tests=fail -E 'test(set_order_controls::)'`,1passed/246skipped,
0.024s bodies after3m02s build. Changed content, missing membership and final charge release
also remain asserted. Native confirmation of the original read-channel control is pending.
Cold-admission review also found duplicate physical reference branches for identical exact
target views and full-catalog template port sweeps. NE-I08 shares only exact target branches
and checks actual captured template providers on misses/hits; distinct epochs, provider
replacement refusal and independent admission remain. Runtime checks are pending. Neither
correction establishes the dominant timeout cause or a measured latency benefit.
The final correction tree **passed** `cargo check --locked --release -p cpg-core --tests
--message-format short`, `20261009T221605.902Z-6996d5`,47.31s. This type-check does not relink
the core executables still owned by the running qualification boundary. New physical/template
controls and the original binding control require their own rebuilt runtime checks.

**failed compiler composite, 2026-10-09:** the resumed boundary completed all481controls,
Nextest `a201a8b0-b3e7-44e9-bcc3-1408a7cc7a5d`,1667.319s bodies:400passed,6assertion failures,
75timeouts,0skipped. Besides the sort and binding assertions above, four small-budget controls
failed: embedding effects requested10,650,840bytes under4MiB; empty normalization could not
reserve593,920bytes for semantic-edge coalescing under8MiB; callable metadata could not admit
another1,528-byte compact command output under2MiB; wide normalization could not admit a
1,344-byte projected Arrow buffer under2MiB. This binary predates NE-I07–NE-I11 corrections.
NE-I10 sizes effect-owned microbatches independently of compute/transfer batches. NE-I11 drops
factories after compilation and reconciles construction versus actual retained metadata, with
selector nodes, normalization roots and synthesis parameters explicitly retained/charged.
The unchanged affected controls need a rebuilt focused rerun. Wide normalization also exposes
a possible shared-pool sort/input pressure cycle; reserve-before-mutation partial native emission
is an investigation only if the corrected rerun still fails. No native pressure-flush mechanism
is implemented or qualified. Qualification proceeded to CLI and remaining independent boundaries;
its concurrent builds do not replace stable-source focused confirmation of these corrections.


**Native-efficiency documentation authoring, 2026-10-09:** NE0–NE9 and cross-plan replacement
routes are authored, with [independent scoped target acceptance](../design_review/reviews/design_review_native-execution-efficiency-plan_2026-10-09.md).
`just docs-check` **passed** on 360 canonical pages with zero link errors after repairing the
existing ADR-0140/§B3 reference mismatch and two new section links; `git diff --check` **passed**.
All73 pre-existing unowned files are unchanged. Product builds/native controls are **not_run**
for this documentation-only scope. RC01–RC05 direction is accepted; NE0 decisions and all eight
corrections remain Proposed/Open. These receipts do not replace any historical failure below.

**Current compiled-product/viewer integration, 2026-10-09:** GK0–GK6 and GR0–GR5 are
Implemented with focused Tested/source-inspected evidence. ADR-0140 and the reuse companion §7
own the refined eligibility, exact selected domains, current admission, fresh graph-bound SCC schedules and
viewer lifecycle. The [independent source review](../design_review/reviews/design_review_compiled-product-reuse-contracts_2026-10-09.md)
retains all initial and subsequent findings. Behavioral private Local/Base/Body/SourceCall hints
are fully retired; these kernels remain Fresh. No general mutable-query scheduler, portable
checked capability, operator adoption or measured latency claim follows.

Commands use available Cargo/compiler/test parallelism, unchanged normal stacks/timeouts and
owned disposable native fixtures. The session uses the host user-manager environment
`XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus` only to
make that fixture prerequisite available. Exact arguments/outcomes remain in each
`build/runs/<ID>/summary.json` and adjacent logs.

| Command / current boundary | Outcome, 2026-10-09 |
|---|---|
| `cargo check --release -p lctx-model -p lctx-surrealdb -p cpg-core -p lctx-serving -p lctx --tests --message-format short` | **passed**, run `20261009T140913.635Z-80b5f9`, 1m10s. Subsequent bounded native/core/serving release compile checks also passed after integration repairs; later source is exercised by the actual reruns below. |
| Focused release model/core/native/serving reuse selection | `20261009T141350.380Z-c60fda`: **failed composite**, 28 passed / 3 failed / 3 timed out. Exposed retained optional Moka capacity/close and incomplete normalization-test ancestry; source and fixture corrected without budget or predicate relaxation. |
| Same affected selection plus compiler matrices and pressure controls | `20261009T143114.145Z-f74363`: **failed composite**, 37 passed / 2 timed out. All completed primitive/domain/native/serving/canonical controls passed after repairs. Catalog and Behavioral six-attempt real-provider compiler matrices exceeded unchanged300s; end-to-end off/cold/hit/reload/changed-input equivalence remains unqualified. Phase/checkpoint diagnostics added for subsequent integrated runs, without narrowing behavior. |
| Final 38-control focused selection, full matrices excluded | `20261009T144502.670Z-8a071a`: **failed composite**, 37 passed / 1 failed, 8.349s bodies. Production fixed-stripe collision control passed; the subprocess borrowed-capacity control still used the obsolete full-key lock path. Its shared production-path correction and exact-two rerun **passed**, `20261009T144833.859Z-e1a97f`, 0.291s, Nextest `4b303dc8-55cc-4e32-ab20-5337baed1a1f`. Aggregate selected controls have passing evidence after repairs; the original composite stays failed. |
| `just verify --select store:rust --nextest-args='--lib -E "test(product_cache::tests::native_)"'` | Native schema4 obsolete-required-field reset control and all12 product controls **passed**, `20261009T145640.086Z-3e9392`, 0.456s bodies. Later review found reset ownership/late-administration gaps; this receipt does not qualify the final reset protocol. |
| Native schema5 persistent-owner reset plus `compiler:producer --lib -E 'test(compilation::reuse::tests::)'` | **passed**, `20261009T150136.950Z-f6c00f`: all12 native controls, 0.634s, and all3 graph controls, 0.006s. Actual graph-only implicit-epoch selection keeps cache identity unchanged and explicit equal-content epochs separate. Reset now has persistent directory ownership; atomic administration refinements follow below. |
| `just fixture -- cargo nextest run --release --no-fail-fast -p lctx-surrealdb --lib -E 'test(product_cache::tests::native_)'` | Atomic reset/retire fencing13controls **passed**, `20261009T150731.463Z-931937`, 0.453s; final pending-marker/shared administration-epoch protocol13controls **passed**, `20261009T150919.600Z-955ea5`, 0.513s, Nextest `92db742b-b9bc-4f9d-90c6-e5aaa85c7358`. Real pending SDK transactions cannot commit reset DDL or retirement after a newer install; readonly connection cannot clear quarantine. Final bounded stale-ACK cleanup/control **passed**, `20261009T151353.336Z-db7117`, all13controls, 0.532s, Nextest `7809d7fa-1890-444c-98aa-d1c9c32f46d5`; committed-but-ack-lost retirement preserves quarantine until compatible explicit recovery reclaims obsolete receipts. |
| SCC refinement and final candidate model controls | **passed**, `20261009T154435.789Z-dc2a91`: 25 model controls, 0.041s, Nextest `f0424d38-9fc5-4ac2-afec-87c8ee05bedd`. Includes graph moves, identical/changed rematerialization refusal and separate graph/schedule charge release. Independent bounded source check accepts exact private materialization binding and Summary consumer ownership. Candidate compiler run cancelled for the concrete normalization repair after the failures below; CLI not_run. |
| Actual BC3 candidate `just verify` model/compiler/CLI selection | Cancelled owned run `20261009T151658.341Z-97de35` (exit143) for the SCC eligibility refinement before compiler bodies. Model22controls **passed** on preceding source, 0.039s, Nextest `5f1636d1-1ee4-487b-8305-c8aa79ec9251`; compiler/CLI gate **not_run** on final source. `effective-profile.json` records manifest/candidate settings and sampled actual compiler O1 workspace/O3 imported flags, available frontend/backend workers. These partial receipts do not install a default. |
| SCC-refined candidate compiler selection | `20261009T154435.789Z-dc2a91`: **failed partial**, Nextest `e6e1f467-e91d-4d03-bcaa-6a0ea7810728`;20 selected,10 passed,7 normalization conflicts,2 structural300s timeouts and1 remaining Behavioral Facts terminated when root cancelled the owned pipeline for repair (exit143). Logs identify `normalize_entities` / `Conflict("symbol_entity_resolutions")` in both profiles. Static tracing found incidental lookup symbols incorrectly emitted as computational roots without their full candidate families. Explicit typed output demand is the correction; no conflict suppression or recursive whole-graph expansion. CLI gate not_run on this source. |
| Explicit entity output-demand correction | `cargo check --release -p lctx-model -p cpg-core --tests --message-format short` **passed**, `20261009T160553.448Z-ad044c`,1m09s. Typed Symbol/SyntaxField/Public/Enumeration demands retain nominal lookup without incidental output, Public ambiguity, semantic refusals and full-normalizer first-error order. Independent bounded source check accepts exact root/miss mapping and output ownership. All6 selected finite/model controls **passed**, `20261009T160744.103Z-5fd0d5`,0.038s, Nextest `508e407f-aaf0-486d-8754-5f8e55fd8663`, including three new demanded-versus-full normalization controls and SCC ownership. Core root-binding refusal **passed**. Actual Catalog and Behavioral normalized-frontier controls **failed** at independent artifact admission after every normalization stage passed (578.412s/827.768s), Nextest `e1bd8092-2400-43e7-ae2e-1169f9bb963a`; both report `selected callable owner claims differ from actual source premises`. This repairs the original Entity conflict but does not qualify artifacts. Source tracing found admission mixing supporting callables' advertised claims into the requested owner family; model-owned advertised-owner selection is the correction, keeping global orphan and strict same-owner completeness checks. |
| `uv run --no-sync pytest -q tests/scripts/test_verify.py` | **passed**, all45 verification routing controls. Mocked routing does not qualify BC3 or a product boundary. |
| Model-owned advertised callable-owner correction | `cargo check --release -p lctx-model -p cpg-core --tests --message-format short` **passed**, `20261009T163428.407Z-d6a8d2`,1m11s. Owner selection follows actual assessments and child/premise links, retaining broad source premises and strict same-owner completeness/global orphan checks; independent bounded source assessment accepts ownership, charging and retained global orphan probes. Both new finite supporting-callable/adversarial controls **passed**, `20261009T163757.029Z-9b0811`,0.002s, Nextest `d32faa69-be1f-4ce6-8468-1ed36fb8118f`. Composite **failed**: both native normalized artifacts timed out at unchanged900s, Nextest `5d8f73d4-da3e-421a-8d08-cc0081f3b4ce`. Both completed normalization, content freeze and facts admission, then timed out in `artifact_invariant_admission` without a new assertion. This is not successful callable/artifact admission; candidate default adoption remains blocked by its required failing boundary. No whole candidate/CLI gate pass is inferred or installed. |
| Guard-free `just ready` | **passed**, `20261009T151547.473Z-0bab4b`, 45s, on final native source; earlier full rebuild `20261009T151034.207Z-3d69b3` passed2m36s before the narrow receipt cleanup. Final preparation rebuilt the extension, selected checkout Python3.14.7 and verified tools/skill links. |
| Final-source guard-free readiness | **passed**, `20261009T165830.221Z-835b15`,7m04s. `just ready` rebuilt the native extension after SCC/entity/callable ownership corrections, retained locked Python3.14.7 and verified tools/skill links. No live native guards or operator state were used. Assembled `just qualify` follows on this source with the release profile; its result remains pending. |
| Final-source assembled `just qualify` | **failed / canceled partial**, `20261009T170649.056Z-032c30`, canceled17:27:50UTC after21m01s, exit143. Full model family **passed**, all798 tests across87 binaries,2.213s bodies after3m34s release build, Nextest `302bde20-36ea-4e46-851f-535e36819598`; analytics14 and flow controls passed. Extract retained45passes, one authentication failure and21timeouts; core retained147passes and two300s matrix timeouts, both in their first cache-off attempt. SIGTERM outcomes are cancellations; remaining families/journeys/leaves were not_run. Ordinary available parallelism and owned disposable fixtures were used. This receipt predates the unverified stable SDK-session correction; no assembled pass or authentication-cause proof. |
| `just build-features`; `just adr-index` | **passed**, regenerated the CLI feature union and ADR index for Moka/ADR-0140 before final acceptance. The direct pinned Moka0.12.16 dependency adds only its narrow lock closure; existing library versions remain unchanged. |

**Remaining acceptance:** BC3 cannot qualify while its required repaired native normalized
boundary still times out; its complete model/compiler/CLI-main gate remains open and no default
cutover is installed. Assembled release qualification is paused for the
[native-efficiency design review](../design_review/reviews/design_review_native-execution-efficiency_2026-10-09.md);
NE0–NE9 now schedule its corrections, with all eight findings Open here in §8. A passing
release control does not qualify the different candidate profile. Native Python preparation must refresh after final Rust source
in a guard-free interval; its prior pass retains the source boundary above. Release remains
the default until the actual candidate gate passes; neither routing tests nor compiler captures
install it. Historical19core300s/six-upper900s/large compiled-export300s timeouts retain their
failed source boundaries. No whole-plan qualification is claimed.

**Previous graph compiler slice, 2026-10-09:** GK0–GK5 and required GR1 foundations are
**Implemented / focused Tested**, committed on main as `da7c4747` from baseline `4d94339f`.
ADR-0139 records model-owned
operation compilation; no record/physical schema, existing digest framing or codebook changed.
The added direct `xxhash-rust =0.8.19` declaration uses the already locked version; Hakari now
unifies its existing `xxh3` and `xxh64` features. XXH3 only selects buckets; full canonical bytes
establish equality and BLAKE3 remains portable identity. Request-owned serving preparation is
implemented; persistent reuse/invalidation and cross-request Moka caching remain later work.

Exact argv/environment and retained logs live in `build/verification/graph-scope-2026-10-09/`.
All Cargo/Nextest commands use normal available parallelism and cached release code. Native
controls use owned disposable fixtures, not operator state. Review findings §8 carry their
bounded source and focused-test dispositions; no historical journey qualifies this compiler.

| Current command / boundary | Outcome, 2026-10-09 |
|---|---|
| `cargo check --release -p lctx-model -p cpg-core -p lctx-serving --tests` | **passed**, 1m29s (`compile-check.log`). Final SourceCall-only lifetime correction is also compiled by the passing final all-target Clippy below. Earlier declaration/caller compile failures were repaired; this is not a clean-first-pass claim. |
| `just fixture -- cargo nextest run --release --no-fail-fast -p lctx-model -p cpg-core -p lctx-surrealdb -p lctx-serving --lib --test stream_reconciliation --test native_companions --test native_search --test executable_identity -E <exact selection in commands.json>` | **failed composite:** 288 selected, 286 passed / 2 failed, 6.332s bodies; Nextest `f21e2b3d-4b46-4c6b-8e23-0417773ddeb0`, `combined-controls.log`. All 242 model, 36 core and eight native/serving controls passed. SourceCall's two constructor budget refusals were repaired and exactly rerun below. |
| `cargo nextest run --release --no-fail-fast -p cpg-core --lib -E 'test(event_and_owner_closures_keep_all_alternatives_and_project_unused_docstring) \| test(empty_root_stream_and_private_spool_preserve_exact_issuer_and_release_resources)'` with `INSTA_UPDATE=no` | **passed repaired boundary:** two controls, 1.967s; Nextest `fc73c7f9-65a3-4925-a867-dcff3ddf504d`, `sourcecall-repair-2.log`. The preceding exact rerun `3848ef3d-6f65-4924-ad7b-cf8e51356c63` failed both at physical lowering (`sourcecall-repair-1.log`). Raw factory state now drops before lowering; prepared topology retains its owners before lowering scratch drops and selectors compile. Test budgets/predicates are unchanged. The aggregate selection has passing evidence after repairs; the original composite remains failed. |
| `cargo clippy --release --keep-going -p lctx-model -p lctx-model-macros -p cpg-core -p lctx-surrealdb -p lctx-serving --all-targets --no-deps -- -D warnings` | **passed**, final source including SourceCall phase correction, 2.52s (`clippy-final.log`). Covers affected declarations, callers and controls, not full-workspace Clippy. |
| `just deps`; `just rules-test`; `just rules-scan`; `just adr-lint` | **passed**, `deps.log`, `rules-test-final.log`, `rules-scan-final.log`, `adr-lint-final.log`. Initial dependency leaf exposed stale Hakari; generated feature union repaired it. No dependency version bump or warning suppression. |

The model controls include forced hash collisions/full-byte equality, canonical buffer count and
ownership release, independent expected scopes, scalar/list/NULL/absence/virtual and physical
sum-field semantics. Core controls challenge borrowed admission, exact batch partitions, Model
known-answer inventories, S0/E0/SourceCall selected projections and cancellation/late corrupt
hydration. Native controls independently challenge actual statement terminality/cancellation,
260-member bounded hydration and the same serving template bound to two distinct reader databases.
SourceCall's complete set DAG is tested relationally; primitive finite/native contrast does not
establish a second complete finite set-DAG implementation.

**Source boundary:** these focused bodies and final Clippy precede mechanical scoped formatting.
Scoped `just turn-end --paths …` **passed** (70 changed Rust files; ADR/Hakari refreshed),
`just docs-check` **passed** (355 canonical pages, zero link errors), and guard-free `just ready`
**passed** on the formatted source (`turn-end.log`, `docs-check.log`, `ready-final.log`). Readiness
rebuilt the native extension and confirmed checkout-specific Python 3.14.7/environment/skill links.
No long journey was repeated solely for formatting. Earlier failed core/native composites exposed
allocation/lifetime and retired-field assertion issues; corrected affected cases passed in the
combined selection or exact rerun above. No production budget, worker cap or semantic validator
was relaxed. Complete-frame Structural/Analytic/Summary exceptions remain justified in §8.

**At that earlier checkpoint, not_run:** GR2–GR5 products/invalidation/request caching, GK6 reuse composition and GK7/GR6
assembled release qualification. The current checkpoint above supersedes only that implementation state. Prior 19 core 300s timeouts, six upper-frontier 900s timeouts and
the large compiled-export publication 300s timeout remain failed historical boundaries below;
no new attribution or measured speed claim closes them. No operator adoption or real-library pilot.

**Current populated correction, 2026-10-09:** PJ0–PJ3 and the assembled review remedies are committed locally on main as `2a62e817` (starting baseline `e98fb4a9`); targeted PJ5 acceptance is recorded below, with six upper-frontier timeouts and enclosing qualification still open. The accepted physical schema is a migration: the reviewed fixed-envelope snapshot was accepted, while completed-state format2, artifact format3 and semantic digest domains remain unchanged. Declaration-selected lowering, full physical reconstruction, mutation-only content freeze, shared immutable pointer runs and private admitted-owner publication replace the old routes. No dependency version moved. The assembled review and its captured-phase followup accept scoped at static / Implemented strength; quantitative benefit and enclosing qualification remain unmeasured/open.

Commands below inherit normal compiler/test parallelism, `INSTA_UPDATE=no` and disposable native fixtures. This session repairs the user-manager prerequisite with `XDG_RUNTIME_DIR=/run/user/1000` and `DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus`; no operator state is inspected. Exact selectors and command arguments are retained in `build/runs/<ID>/summary.json` and the adjacent logs.

| Current command / boundary | Outcome, 2026-10-09 |
|---|---|
| `cargo check --release --locked -p lctx -p lctx-serving -p lctx-publisher --tests`; `cargo check --release --locked -p lctx-surrealdb --tests` | **passed** after the final live-parent correction. These compile checks preceded the diagnostics emission repair; final all-target Clippy below covers that source. |
| `cargo nextest run --release -p lctx-surrealdb --lib -E 'test(codec::tests::) \| test(schema::tests::) \| test(adapter::tests::) \| test(compiler_scope_tests::) \| test(reconciliation::tests::)'` | **passed:** 12 pure controls, Nextest `e2d141f7-017f-4463-9ce0-8ab07cd3a12a`. Read all 59 statements in the snapshot diff before `cargo insta accept`; this is physical-schema migration evidence, not publication qualification. |
| Focused `just verify --select store:rust` native/ordered/lifecycle/cold boundaries | Initial build race **failed before tests** (`20261009T052621.392Z-3b8832`). Corrected source rerun `20261009T053546.212Z-22b6e9`: **52 passed / 2 failed**, 25.082s bodies. Both failures were stale bare-error/successful-abandonment expectations; corrected controls retain the exact primary cause, Terminal/Confirmed, exact Removed identity and independently absent database. Exact-two rerun **passed**, 4.378s (`20261009T054808.562Z-cbcc63`). No checks were weakened. |
| Exact core phase-rejection control through `compiler:producer --test compiler_artifacts` | Initial control and diagnostic rerun **failed**: native phase events missing despite a captured ordinary scoped INFO event. Captured-Dispatch emission correction **passed**, one real admission rejection with begin/failed and matching ID, 1.393s (`20261009T055352.008Z-869ea3`). Four standalone source controls passed as a narrower probe. |
| `just verify --select store:rust --nextest-args "--lib --test compiler_lifecycle -E 'test(phase::tests::) \| test(acknowledged_setup_authentication_failure_has_a_failed_phase_terminal)'"` | **passed:** four current-emission controls, 0.023s (`20261009T055723.301Z-c3525e`): actual refused native authentication plus filtering, concurrent attribution and abort-only begin. |
| Focused `compiler:producer`, publisher and `compiler:cli` selection (`20261009T052757.430Z-f56701`), plus supplemental frontier run (`20261009T052936.197Z-d405a4`) | **failed composites:** core selection had the phase-control failure and nine 300s timeouts; supplemental ten frontier controls timed out at 300s. Publisher foreign captured binding admission **passed**, compiled-export publication timed out at 300s. All three CLI cases **passed**, 230.750s aggregate bodies: Catalog Facts 144.176s, Catalog Normalized 198.791s, Behavioral Facts 230.749s. These are separate bounds, not one clean integrated pass. |
| Single Catalog Facts compile/admit/export control | **passed**, 138.296s (`20261009T054911.783Z-475010`). Static tracing found no supported universal freeze/read wait cycle. Concurrent pressure/repeated work remains possible, not a proven cause of the other timeouts. The split frontier selector had incorrectly missed qualified test names; corrected regex preserves the original 900s allowance, without increasing a deadline. The remaining seven original-frontier controls finished (`20261009T055800.496Z-d3005d`): Behavioral Facts **passed**, 488.727s; six Catalog/Behavioral Normalized, Analysis or Catalog cases **timed out at the unchanged 900s deadline**. Phase logs place observed long intervals before final admission: Catalog callable-aspect normalization took 342.592s; Behavioral upper cases had begun entity normalization. Small borrowed CPU regions separated by long gaps do not identify the intervening effect/transfer cause. |
| Applicable leaves: lint-agents, adr-lint, fixtures-check, gold, rules-test, Ruff, types, deps | **passed** in `20261009T055149.947Z-e78c68`; rules-scan initially **failed** on an existing private field formatter named `sql`, not a DataFusion execution bypass. Renamed it `sql_expression`; exact rules rerun **passed** (`20261009T055253.859Z-ae5317`). Generated Hakari adds an existing console feature union; no version change. The final dependency replay initially failed (`20261009T062538.926Z-19d474`): cargo-shear does not scan the included external native fixture. Documented its real tracing dependency alongside the existing shared-fixture exemption; exact dependency rerun **passed**, 3.2s (`20261009T062616.617Z-e74670`). |
| `just clippy` through leaf:clippy and affected reruns | Final full keep-going workspace/all-target Clippy **passed**, 44.0s (`20261009T055818.261Z-c08c03`), after factoring the preparation future alias, using checked standard divisibility and adding tracing to the publisher test dependency for the shared fixture. No warning suppression. |
| Independent intact/tampered known-export control (`just verify --select compiler:producer --nextest-args "--test compiler_artifacts -E 'test(graph_artifact::transported_graph_refuses_missing_and_tampered_originals)'"`) | **passed**, `20261009T062646.015Z-b2b5ea`: intact exported graph accepted; tampered/missing original and invalid assertions stream refused, 247.378s under the unchanged 300s deadline; Nextest `d6bff1d4-2e63-49a1-b793-2e9d97fd984f`. |
| Guard-free native extension refresh, then one `just verify --select serving:mcp` | **passed**, managed `20261009T060142.629Z-c9c159`: guard-free native refresh passed, 1m22s. Locked release CLI/evaluator build passed (197.045s); native test build plus execution passed (1889.241s), with **two populated journeys passed in 1741.50s bodies**; fixture restart then **35 Python/MCP/native-evaluator controls passed**, 27.378s. The boundary took 2116.565s; the outer runner also waited for a preceding fixture before refresh, outside this boundary. No guard ran during refresh. |


**Final maintenance/source boundary, 2026-10-09:** the integrated receipt above precedes
mechanical scoped Rust formatting. Whole-tree maintenance initially formatted 165 previously
clean paths outside this correction; that diff was retained under build, and only those unrelated
formatting edits were restored. Scoped `just turn-end --paths …` passed ADR/Hakari generation
and the 40 changed Rust files; Ruff was skipped because no Python source changed. The first
post-format native preparation (`20261009T065736.910Z-51f100`) was cancelled during this
maintenance correction, before any tests; completed build artifacts were preserved.
Final guard-free preparation `20261009T070023.218Z-35efa2` **passed**: native sync 152.386s;
locked release CLI/evaluator build 214.577s; 15 pure schema/codec/adapter/reconstruction/phase
controls, 0.028s (Nextest `62c2a774-5a85-4264-91c3-f74aa88448b8`); four rooted producer
fingerprint controls, 0.033s (Nextest `8e6cce13-9094-4907-a9d6-45014d5ab29f`). `just ready`
**passed**, confirming checkout-specific Python 3.14.7, current native extension and shared skill
links. The long populated run was not repeated solely for formatting; its exact source boundary
remains explicit. `just docs-check` **passed**, 349 canonical pages and zero link errors.

The diagnostics repair changes observations only. Existing timeouts remain failed receipts rather than attributed performance conclusions; deferred timeout followup and assembled `qualify` remain **not_run**. Independent intact/tampered known-export transport and populated current-schema direct publication/backup/restore passed. Both-profile upper-frontier controls retain six 900s timeouts, and the large compiled-export publication control retains its 300s timeout; broader findings stay open. The current packages do not install the BC3 candidate profile or close PC6/CU6/BC3/BC5.

**Preceding continuation, source/date boundary 2026-10-08:** the following evidence describes the implementation before PJ2/PJ3; it is not current-schema populated acceptance.

**Continuation, 2026-10-08:** the operator committed and pushed the preceding PC/BC/CU tree as `dcb505d5a8c8c4760893f4d49b459b3d0bfce69d`. The current implementation refines first-native-failure attribution and restore disposition, borrowed/lazy native selection, streamed library eligibility and fresh graph-index reuse; correction plan §7 owns the mechanisms. An independent implementation review of these changes and adjacent consumers found no material findings. Cold control fixture corrections retain literal expected supplier/coverage evidence and the exact typed thrown-error assertion. Review is static evidence, not whole-journey acceptance.

**passed (2026-10-08; UTC run IDs are2026-10-09):** six-crate all-test `cargo check` (`20261009T015013.375Z-2767ff`);45 verification-harness pytest controls;21 focused projection/completion/captured-coverage release controls (`20261009T015057.419Z-d919f6`);8 native demand controls including two4096-key windows (Nextest `566b534f-d242-4476-b568-62bb79212bba`, `/tmp/native-demand-focused.log`); the actual streamed-library discovery/index-plan/late-failure control (Nextest `5c6ee50e-7778-458c-a455-e4b2c259202f`); and the known-removal/session-failure composition control (Nextest `48e17b4d-b524-4788-ac28-a955d197af6a`). These receipts precede final scoped formatting and native preparation. The preceding unchanged native-search target passed both controls, with its earlier mixed-source boundary retained.

**Additional repaired receipts:** the publisher scope covers ten distinct controls across repaired runs. The first selection passed seven and failed two fixtures; subsequent corrections fixed attribution-view union, SDK THROWN classification and typed assertion construction. Actual restore-owned failed HTTP request passed, retaining its exact primary/Removed outcome and absent private database (`14287cee-7214-4a35-ade2-7b1a3d9cd124`). Ordinary foreign captured admission, old-format refusal and new-format transport under a different native database passed (`77838c48-0595-4a45-bc13-4065657bda77`,6.672s). This independent literal supplier/family/outcome control has an empty artifact universe; it is not extraction quality or a full populated backup/restore journey. Provider/compiler fingerprint isolation passed four controls (`20261009T015716.679Z-2b5a35`); both correctly selected streamed normalization controls passed (`20261009T015212.535Z-c8a818`,33.534s).

**Native cancellation correction:** first CLI Catalog/Facts rerun failed at38.575s with an attributed unfinished `scan_rows` setup (`20261009T015212.158Z-c22649`). A retained setup driver repaired this actual cancellation failure, and the same CLI case passed78.679s (`20261009T021513.189Z-07eb77`). Subsequent independent review found a lost reservation handoff; sized ownership now follows the result/descendant streams, and a buffered-result owner closes the send/cancel error race. Final review found no remaining material findings. The final34-control native selection passed gated setup cancellation, dropped transport tail, late failure, buffered error and a300KiB moved binding, plus exact native multi-window demand. The unchanged4099-row/4MiB closure control also passed. No write acknowledgement or cleanup limit was relaxed.

**Final focused receipts (2026-10-08):** commands use normal available compiler/test parallelism, the pinned toolchain, locked release artifacts and owned disposable native fixtures. Run records retain the exact filter arguments. Generated Hakari changes introduce no lockfile/version move. The native refresh preceded resumed guards; subsequent production change was the equivalent restore-buffer clamp, outside the Python extension's dependency closure.

Source is committed locally as `e6a73d12` (native ownership/demand), `a148758c` (streamed eligibility, graph index and cold outcomes) and `70b8bf53` (verification/prerequisites/maintenance). Their messages explicitly retain the integrated-tree test boundary and deferred timeouts. No additional push is requested.

| Actual command / selected boundary | Outcome and receipt |
|---|---|
| `just sync native` | **passed**, `20261009T022532.759Z-fe6d4c`,3m21s. |
| `just fixture -- cargo nextest run --locked --release --no-tests=fail -p lctx-surrealdb -p lctx-publisher -p lctx-serving --lib --test compiler_views --test cold_bindings --test native_search -E …` | **passed:**34 controls,54.017s, `20261009T023103.222Z-6b45dc` / Nextest `b20765ed-4927-414f-ad1d-87f36bad73d6`. Includes completion/provider controls, large/exact-empty demand, failed import, restored disposition, captured bindings, streamed eligibility and both native-search cases. |
| `just fixture -- cargo nextest run --locked --release --no-tests=fail -p cpg-core --test native_closure_selection -E 'test(=native_closure_windows_projection_pin_and_stream_lifetime)'` | **passed:**1 control,34.561s, `20261009T023103.219Z-93de53` / Nextest `ad3ab0e0-be80-4a50-95d2-df4b2371058d`. |
| `just fixture -- cargo nextest run --locked --release --no-tests=fail -p lctx-surrealdb --test compiler_views --test compiler_lifecycle --test stream_reconciliation -E …` | Initial **5 passed / 3 failed**,9.458s, `20261009T023534.113Z-b2bd85`. Three stale expectations assumed bare errors or successful abandonment after an intentionally failed write/import. Exact typed conflicts now use the retained primary cause; abandonment independently asserts Terminal/Confirmed, exact Removed identity and absent database. All three corrected controls **passed**,6.713s, `20261009T024349.735Z-d55830` / Nextest `9b4f1649-eef5-4c9d-9e10-c5c5d86b4cfc`. Independent test-integrity review found no material findings. |
| `cargo nextest run --locked --release --no-tests=fail -p lctx-serving --lib -E …` retained-member/classification lifetime/refusal selection | **passed:**3 controls,0.006s, `20261009T023605.821Z-9328ad` / Nextest `45be8ab9-f53a-4952-99f8-fa592293809f`. The preceding wrong module-name filter selected zero and establishes no verdict. |
| `uv run --no-sync pytest tests/scripts/test_verify.py -q` | **passed:**45 controls, `20261009T023103.223Z-0514bd`. |
| `just docs-check` | **passed:**346 canonical pages, `20261009T024859.985Z-400e62`; documentation publication only. |
| `just verify --select leaf:ruff --select leaf:types --select leaf:deps --select leaf:clippy`, then affected leaf reruns | Types/deps **passed** in `20261009T023103.219Z-7f7db3`; Ruff **passed** after formatting earlier profiling controls (`20261009T023130.068Z-3134eb`); full keep-going workspace/all-target Clippy **passed** after artifact fixture-import and manual-clamp corrections (`20261009T023325.452Z-d1f95d`), and after the final cold-test correction (`20261009T024433.296Z-30769b`). |
| `just fixture -- cargo nextest run --locked --release --no-tests=fail -p lctx --test compile_artifact -E 'test(/^native_cli_catalog_/) \| test(=native_cli_behavioral_facts) \| test(=native_cli_behavioral_normalized)'` | **failed:**3 passed and3 timed out, `20261009T023103.219Z-3ed231` / Nextest `ab16a3a5-8ebc-455e-ba5e-b9102d959ad2`. Catalog Facts110.244s, Catalog Normalized204.905s and Behavioral Facts293.175s passed. Catalog Analysis/Catalog and Behavioral Normalized exceeded unchanged300s. No completed comparison or acceptance is inferred for those cases. |
| `just verify --select serving:mcp` | **passed**, `20261009T023103.223Z-bda1ff`, 2439.61s total. Locked release CLI/evaluator build 232.785s; native journey command 2165.532s including its 4m42s build; both populated native cases passed in 1883.02s of test execution; Python MCP/native evaluator selection 35 passed, 38.488s. Actual fresh compile/admit/direct seal, scoped tools and backup/restore are covered with deterministic contract vectors. No internal phase timing or live-model/whole-plan qualification follows. |

**Current failures / remaining acceptance:** the fixture-backed flow oracle reaches execution but exceeds its unchanged10s subprocess limit (`20261009T015732.548Z-550748`), without a semantic comparison. The three CLI timeouts above join the operator's deferred timeout follow-up; four earlier300s Behavioral controls remain deferred. The populated native/MCP/evaluator boundary passed without cancellation; its31m23s native execution prompted the [populated journey target review](../design_review/reviews/design_review_populated-journey-execution-amplification_2026-10-08.md). That review owns its new, unscheduled findings and investigations; this table retains the existing execution receipts and §8 dispositions. Timeout reruns, assembled `qualify` and real-library qualification are not_run in this review scope. Whole PC6/CU6/BC3/BC5 acceptance and existing source findings remain open until their stated evidence is complete.

**Preceding-source execution receipts and recovery:** the following retains its original source/date boundary; queued work described there is not the current run inventory.

**Correction baseline, 2026-10-08.** PC0 was committed as `607fef2e` with its then-current operation-contract decision; ADR-0138 now consolidates that decision. PC1–PC5 and preceding integration repairs are included in the operator's `dcb505d5` checkpoint. They provide shared completion outcomes, captured supplier contracts and admitted reporting, prepared query/table layout, compact spill ordering with exact membership, and whole-unit restore batching. Independent integration corrections include cold graph-backed view lookup, charged retained member state and canonical input companion hydration. Source implementation is not PC6 acceptance; source findings remain open pending their named controls.

**Current scoped receipts, 2026-10-08:** PC2 `cargo check -p cpg-core -p lctx-model -p cpg-extract --tests` passed before its final manifest changes. Its first release model selection ran 45 controls: 44 passed and one stale corruption fixture failed because it mutated an ignored `content` field rather than the actual `view` identity. That fixture now changes `view` and proves the corrupted record differs. The current-setting targeted rerun passed all five selected graph-contract/producer-contract controls (run `5870cf72-7f97-4ef5-b457-8fa677c7bc39`, `/tmp/pc2-model-rerun-owned-target.log`); 18 controls were outside its explicit selection. The first core captured-binding control failed because its empty-input fixture omitted the independently required Input-grained Signatures outcome (run `217b5507-7090-4b21-950b-3813fc37e5fb`, `/tmp/pc2-core-controls.log`). That fixture is corrected without relaxing admission; its current-setting rerun passed (one selected control, 96 outside its filter; run `359244c8-f9c9-41e5-81ea-d80d293a00a5`, `/tmp/pc2-core-rerun-owned-target.log`). This is a captured-binding admission control, not the complete native cold transport journey. PC3's isolated source passed 11 pure controls and the actual `stream_reconciliation` terminal/sparse-scope/index-plan control (run `4bca46f3-e0a2-4bd3-8b87-e9ccfc23a432`); integrated replay remains pending. Main completion/sort/restore and retained-member controls are queued/running; no aggregate pass is claimed.

**Execution resources:** ADR-0136 records available build/test parallelism and the operator-approved 16 GiB disposable fixture default with an 8 GiB tracked-memory threshold. Fixture/build-helper Python controls, an automatic-worker pinned compiler probe and actual native fixture startup passed. These are harness evidence, not PC6. Historical receipts below remain evidence for their original named source and logs.

**Build recovery, 2026-10-08:** the current native-family run `20261008T060150.642Z-c7a734` encountered a Cargo build deadlock before tests. Kernel lock records showed a cycle among Cargo processes `2177224`, `2264333` and `3305958`: each retained shared unit locks needed exclusively by another, and none had active compiler children. After explicit operator authorization, only the latter two deadlocked Cargo subprocesses received SIGTERM. Their retained-member/core checks are **cancelled_before_tests**, not failed test receipts; their logs and completed artifacts remain preserved. The existing main build immediately resumed compilation. Restarts use the ordinary checkout artifact directory with unrestricted compiler/test workers; the queued four-package library batch already includes the three retained-member controls. This is operational recovery, not a Cargo fix. Independent exact-source inspection of pinned Cargo `3d7cf6e937d6127d0f49881bf689c560b36d35c4` and current upstream `33f504c39f7a041a263d563ae4e4f856baf42090` found the same affected lock protocol and the matching open [upstream issue #17508](https://github.com/rust-lang/cargo/issues/17508); no delivered fix was found in that inspected scope. Main compilation and the queued native/library/import checks still have no new test verdict.

PG0 is accepted through [ADR-0138](../adr/0138-operation-shaped-native-finalization.md). PG1–PG9
production routes are integrated on main and independently source-reviewed. Final targeted
compiler, publication, CLI and native/MCP/evaluator acceptance remains in progress. The plan
owns enclosing finding closure; successful slices do not close it automatically. PC3 worker changes are integrated on main; its isolated checkout remains until final retirement.

Functional commands below use `INSTA_UPDATE=no` and `UV_NO_SYNC=1`. Owned native commands run
through `uv run --no-sync python /tmp/persisted-core-controls.py` or the committed
`scripts/native_controls.py` launcher, with authenticated persistent disposable fixtures and
private runtime configuration. They do not use or reset the operator store. Current Nextest selections use `--no-tests=fail` and ordinary available test parallelism; dated commands below retain their original execution settings.

| Actual command / selected boundary | Outcome, 2026-10-07 |
|---|---|
| `cargo test --release --locked -p lctx-model --test completed_views`; model lib `demand_tests` | **passed:** three exact-view/completion controls and one analytical demand control. |
| `cargo nextest run --release -p lctx-model --lib --test serving_contracts` with the seven encoding/wire boundary selections | **passed:** seven exact encoding/cap/envelope controls; run `68035e40-13e7-4397-a16b-6e1360e91227`. |
| `cargo nextest run --release --locked -p lctx-model --lib -E 'test(domain::record::contract_encoding_tests::)'` | **passed:** three independent declaration-vector controls; run `c5720523-175c-42d7-9bff-7fefe318fb10`. Stale expected vectors were corrected to include the already migrated UTF8 tag; production encoding/codebooks were unchanged (`2649cc71`). |
| `uv run --no-sync pytest tests/scripts/test_verify.py -q` | **passed:** twelve launcher controls. Explicit Cargo targets replace family defaults, and temporary Cargo config is forwarded without changing committed profiles (`b29ae511`, `abbb746f`). |
| `just ready` | **passed:** prerequisite preparation, including recheck after the native transport environment failure, before producer guards resumed. No environment synchronization while guards were live. |
| Owned native `cargo nextest run --release --locked -p lctx-surrealdb --lib --test compiler_views --test native --test stream_reconciliation` | **passed:** 24 library, three view, two native and one stream controls after shared128-row native windows (`2aab0cc0`); run `18f1cb10-2a65-4e6c-a85e-feaeaf156dc6`, `/tmp/persisted-native-shared-window-controls.log`, 543.839s. Includes cold backing/original corruption, identical pending replay and conflicting membership rejection. An earlier exact closing-admission lifecycle control also **passed**. |
| Owned core `cargo nextest run --release -p cpg-core --test facts_admission -E 'test(real_pyrefly_document_predecessor_uses_exact_completed_native_views)'` | **passed:** actual Pyrefly/document/Assemble predecessor in both profiles after correcting the ownerless fixture (`0a827712`); run `14ad5c0d-a9cc-4d33-8979-85f66a5e8e3a`, 163.241s. Not a whole-library pilot. |
| `cargo check --locked -p cpg-core --tests --message-format short` | **passed:** after the unchanged async API gained a library-owned boxed driver (`3c00275e`), 1m53s. Two bounded preparation future erasures address Rust lifetime checking without copying rows/vectors. |
| Selected27 core controls, temporary core-only O0 | **partial:** build passed, 15 unchanged controls passed; duplicate-demand fixture errors, O0 test-stack aborts and host ENOSPC interrupted the remainder. Exact log `/tmp/persisted-core-selected-o0-controls.log`, run `37cde719-021f-4410-b1da-0469a02a17d8`. Corrected/remaining boundaries are separate rows below. |
| Core default-stack cancellation handoff | **passed after correction:** native write polling remains behind the native library's boxed boundary (`0aed7109`); run `3f9abc35-2fdb-4ee8-8dde-7e704a7c2c41`, 52.668s. |
| Core distinct-edge sort and duplicate-follow selections | **passed after fixture repair:** 102800 genuine distinct edges retain the8MiB pool, plus independent duplicate-demand deduplication (`df2d462b`); run `422d1bfa-1b5d-407f-a2a0-4d7db2f44af1`, 58.767s. Both workspace-input controls also **passed**. |
| Owned core `native_closure_selection` | **passed after correction:** shared write windows, documented128MiB gRPC ceiling and one-time compact selection/direct RecordId hydration preserve4099 rows, unrelated4MiB body, default stack/timeout,4MiB native read budget, pending exclusion, ordering, corrupt unrelated reference and stream-lifetime assertions. Run `43632f82-3b09-4b3f-ac13-94c7a87bb782`, `/tmp/persisted-core-native-materialized-closure.log`, 114.888s. Actual planner/physical-provider assertions also passed; no Binary-literal translation change was needed. Affected frozen/empty/corruption views also **passed**, three controls/449.743s, run `35f01136-50e6-452f-a68d-777ae4fb607c`; scanner correction `6cd9c8ae` was independently source-reviewed. |
| `cargo nextest run --release -p lctx-serving --lib` with the23 pure selected-row/delivery/original controls | **passed:** 23 controls; run `91fb0224-5923-4ab5-9798-a54b96592ec9`, 0.237s. Native capability union remains pending. |
| `cargo build --release --locked -p lctx-semantics`, direct editable extension installation/import | **passed:** final complete-backing/model source built in3m08s and the installed editable extension imported in a guard-free gap; `/tmp/persisted-python-native-complete-backing-build.log`. No wheel or environment synchronization. Publication, CLI and MCP journey targets also built earlier; build-only success is not runtime acceptance. |
| External publication/backup/restore journey | **failed before correction:** compilation on the old nested scanner refused a request before reaching publication, `/tmp/persisted-publication-controls.log`, 761.92s. **failed at backing coverage:** rerun with corrected scanner reached the omitted projection snapshot chunk schema arm after1647s, `/tmp/persisted-publication-materialized-controls.log`. The shared backing correction precedes the next run; neither attempt reached external transport acceptance. |
| Analytical/empty driver selections | **failed / corrected empty boundary:** two compound analytics controls hit the runner300s cap without assertion/native errors; the empty control found omitted snapshot backing. After the backing schema correction, empty input reached an obsolete producer-label receipt lookup. Snapshot views now have neutral attribution and actual producer identity is contribution-owned; model/core lookups now use exact relation views; the fresh empty control passed below. Log `/tmp/persisted-core-driver-materialized-controls.log`; no analytical acceptance from these attempts. |
| Required non-graph backing and neutral completed-view receipts | **passed:** declaration-derived88-family inventory replaces the manual two-arm schema/scanner (`bd29b7ae`). Pure both-profile neutral-view receipt control passed. Final owned `compiler_backing` two controls plus `normalized_generation::empty_captured_scope_completes_explicit_no_scope_and_empty_snapshots` passed, run `bcbb56c1-7069-41a0-a25b-7b0a22666f38`, `/tmp/persisted-native-backing-neutral-empty-controls.log`,303.851s after2m50 build. Covers actual1MiB opaque snapshot payload, >8MiB serialized state envelope and singleton import under the unchanged64MiB bound; coherent negative-zero body/canonical/content tamper fails model regeneration before membership checking. Empty compilation retains explicit no-scope and exact snapshot receipts. Independent source review accepted both the shared backing extension and the core/model receipt migration; producer attribution remains contribution-owned. |
| CLI early option/configuration/destination refusals | **passed after correction:** `cargo nextest run --release --locked -p lctx --bin lctx --test compile_artifact` selecting three refusal controls plus two `compile_options::tests` controls; run `a9fa4537-c4e9-4ced-b5e5-db35c92c0475`, `/tmp/persisted-cli-preflight-controls.log`,0.062s after2m46 build. Earlier selection had two passes and one failure: lower-frontier flags were masked by missing runtime configuration. Shared `validate_frontier` now runs before native setup and is reused by option preparation; no native compiler/model change. Native missing-store refusal later passed below; final both-profile/frontier acceptance remains pending. |
| Native runtime progress diagnosis | **stopped diagnostic, not accepted:** `/tmp/persisted-publication-complete-backing-controls.log` and `/tmp/persisted-core-final-six-controls.log` were stopped by terminating only their owned Rust children after bounded GDB snapshots. Publication was synchronously polling `NativeBatches` from normalization coverage on its current-thread client runtime. Locked SurrealDB3.3/tonic0.14.6 source confirms the connection background worker stays on its creation runtime; moving request futures does not migrate it. Coverage now awaits the existing async availability path. The analytic fixture uses a permanent two-worker client runtime; its epoll snapshot does not establish the same cause, and source inspection found no `wait_scans` cycle in normalized admission. The fresh separate analytic diagnostic has reached marked reference and invariant admission checks and continues advancing; no separate deadlock has been established. Temporary source logging was removed after locating that stage. Both fresh older-source diagnostic binaries were then stopped after the final focused native controls passed; only owned Rust children were terminated, both wrappers exited101 and tore down their fixtures. Logs `/tmp/persisted-publication-async-coverage-controls.log` and `/tmp/persisted-core-admission-diagnostic.log` are stopped diagnostics, not acceptance. No separate admission deadlock or measured latency cause was established. No assertion, query timeout, test flavor or model contract was relaxed. |
| Incremental union counts and canonical/cold graph selection | **implemented / focused verification pending:** completion probes at most128 new keys against exact prior physical contributors during the necessary ordered content fold; it no longer invokes a correlated prior-membership subquery for each current row. Canonical scans select completed memberships, one-hop entity aliases and physical family IDs once before payload hydration. Cold view cardinality audit similarly resolves contributors once while retaining its independent membership scan. Both scan forms share statement/error finality handling. Actual `compiler_views` controls **passed** for multi-window overlapping contributors/frozen views/empty output (69.80s) and canonical family/one-hop alias selection (62.45s), `/tmp/persisted-native-window-controls.log`, after1m54 build. Independent exact-source review accepted the counting/selection/finality changes. A further shared shortcut preserves authority/view/field/projection validation for explicit empty keys/fields and unfiltered zero-row views; arbitrary SQL keeps normal execution. The selected provider lowers known empty intersections to explicit empty keys. The focused empty-read view/refusal/lifecycle control **passed** (65.01s), `/tmp/persisted-native-empty-controls.log`; the existing pending/frozen/selected/state-transport control also **passed** (126.89s) on that final source. Final direct native extension build passed (1m35s); atomic direct installation and fresh import/process exit passed in the subsequent owned guard-free gap, without a wheel or environment synchronization. The first install command overwrote the library mapped by its own importer and exited139 after a successful child import; the corrected installer obtains the path in a finished subprocess and atomically replaces the file before fresh-process validation. This was an installer lifecycle failure, not a passed command. This is qualitative work reduction, not a measured speed claim. |
| `cargo check --locked -p cpg-core --tests --message-format short`, final async coverage/query changes | **passed:**1m05s, `/tmp/persisted-core-final-async-check.log`; temporary admission diagnostics were removed from final source. |
| Current-source native CLI and capability controls | **partial:** missing-store refusal passed (0.09s) and actual native capability-union document isolation passed (50.88s), `/tmp/persisted-final-cli-capability.log`. Unchanged CLI frontier control passed Catalog Facts/Normalized, then its Analysis child main thread overflowed under temporary coreO0 with the default OS stack; runner exited101 after565.04s. `RUST_MIN_STACK` covers spawned test threads, not CLI main. The same eight cases are rerunning with child-only32MiB `RLIMIT_STACK`, without a committed runtime/build change. This is functional O0 evidence, not default production stack or performance evidence. |
| Final non-functional leaves, first execution | **passed:** types, agent instructions,193 fixture parses, gold alignment and both rule controls. **failed:** Ruff formatting/import findings, stale ADR index (also blocks docs publication), stale generated Hakari features, and one manual-clamp Clippy finding. The read-only SQL planning test in the lower foundation could not depend upward on core's helper; it now supplies explicit read-only options under one scoped rule suppression, and `just rules-scan` rerun passed. Clamp expression was corrected equivalently. Formatting/ADR/Hakari regeneration remains at final `just turn-end`; failed leaves will rerun. Logs `/tmp/persisted-final-leaves.log`, `/tmp/persisted-final-clippy.log`, `/tmp/persisted-final-rules-rerun.log`. |
| All-target source integration and affected pure controls | **passed after correction:** `UV_NO_SYNC=1 just clippy`, `/tmp/persisted-final-clippy-sixth.log`,6.38s final cached rerun. Earlier failures exposed native/core/serving lint findings and stale test integration: two `SourceSnapshot.content` getters, `Workspace.content`, and an extra native-store argument in the shared provider test driver. Tests now consume exact view/workspace identities and the driver creates one store. Contiguous bounded canonical batches remain intentional with a specific allocation rationale. `cargo test --release --locked -p lctx-model --test analysis_expected` passed five controls (0.03s), `/tmp/persisted-model-analysis-views.log`; model `domain::serving::dispatch::encoding_tests` passed three controls (0.00s), `/tmp/persisted-final-encoding-controls.log`; native `compiler_provider::tests::textual_predicates_push_down_before_projection` passed (0.00s), `/tmp/persisted-final-read-only-planning-control.log`. These are pure/source boundaries, not native integrated scope acceptance. |
| Locked native planner and exact point-selection correction | **implemented / focused native controls passed:** locked SurrealDB3.3 source (`exec/index/analysis.rs`, select/dynamic scan) only extends compound equality prefixes through singleton `IN`; existing relation candidates prevent its separate multi-value expansion. The shared reader and overlap count now reuse writer-derived deterministic membership RecordIds, bounded128 physical pointers, verified logical-to-physical contributor metadata, and deduplication before payload hydration. Key demand survives provider lowering as `KeysSql`; residual filters run on the exact selected payloads. Recognized atomic-field selection excludes the competing family-prefix index through `WITH INDEX by_scope`, uses at most32 containment values per indexed selection, then deduplicates/orders candidate keys before exact membership point reads. Field-only provider demand is consumed once, preventing repeated full field demand across transfer windows. General SQL/non-atomic fields keep their existing compact candidate route; no generic index claim is made. Independent correctness review found two missing obligations: partial candidate keys must survive cancelling/resuming `next()`, and cold admission must establish the deterministic membership ID. Both are corrected in the shared owner; cold identity validation joins the already necessary contribution membership fold/import checks. New actual native controls cover large keys/fields, frozen overlap, projection/residual filtering and coherent membership renaming. The initial large-field control failed on candidate ordering; an explicit deduplicated/grouped ordered key selection corrected it. The borrowed rerun passed five controls, then its last three lost transport when the enclosing CLI fixture closed; that run was not a suite pass. Fresh dedicated `cargo test --release --locked -p lctx-surrealdb --test compiler_views -- --nocapture` passed all eight controls (647.45s), `/tmp/persisted-point-native-owned-final.log`, including both new controls and actual state/original transport. Correctness review corrections are implemented; cancellation-resumption has source-reviewed ownership evidence, without a timing-based test claiming to isolate partial gathering. Source diagnosis establishes a physical mismatch, not its share of elapsed compilation time or a Measured speed benefit. |
| Final compiler control environment and current-build preparation | **failed / stopped:** `/tmp/persisted-core-final-five.log` recorded an O0 test-thread8MiB overflow in `default_off_is_explicit_in_both_profiles`; the following selected analytics case was stopped together with its owned loop before continuing on that setup. Wrapper exited143 and tore down its fixture. The fresh rerun will use temporary child-only32MiB spawned-thread/main stacks on the corrected native source, with unchanged assertions and production settings. `cargo build --release --locked -p lctx-semantics` passed2m39s, `/tmp/persisted-python-native-integrated-final-build.log`; publication/native-journey targets also built, `/tmp/persisted-final-native-journey-build.log`. These preceding-source builds are not final query-correction runtime acceptance. `UV_NO_SYNC=1 just ready` passed after the stack-shaped environment failure without environment synchronization. |
| Final CLI stack rerun and current development bridge | **failed / partial, before point-read correction:** the unchanged CLI cases completed all four Catalog frontiers and Behavioral Facts/Normalized; Behavioral Analysis failed with unconfirmed cleanup after3830.82s, `/tmp/persisted-cli-frontiers-main-stack.log`. This did not establish a server-death cause; its original failure was obscured by cleanup. The CLI now reports original and cleanup errors together. No assertion, timeout or fixture budget was relaxed. Final point-source `cargo build --release --locked -p lctx-semantics` passed1m29s, `/tmp/persisted-python-native-point-final-build.log`; atomic direct installation and fresh-process import/exit passed before Python producer/consumer guards resumed. `UV_NO_SYNC=1 just ready` passed, `/tmp/persisted-point-final-ready.log`. The fresh actual native/MCP/evaluator and selected compiler controls are running with matching query source and temporary32MiB O0 spawned-thread stacks; no wheel or environment synchronization is involved. |
| Actual native MCP journey on point-source | **failed:** both finite native journeys completed compilation, admission and direct sealing, then `get_operation` refused canonical evidence. `/tmp/persisted-point-final-mcp-evaluator.log`,1451.62s; Python MCP/evaluator followups were not reached. Source diagnosis found semantic role names passed to physical flattened `SignatureTypeSubject` field selection. The shared serving caller now selects canonical Parameter/Return subject IDs before typed observation postings (`acc1d0ef`). The independent finite `signature_typing_selection_distinguishes_sum_ports_and_preserves_observations` passed (0.00s after2m27 build) under the temporary coreO0 control configuration; includes equal raw bytes across roles, multiple observations and missing/foreign ports. Source review accepted the correction, and the bounded direct caller-pattern check found no matching second sum-field mismatch. Actual journey rerun remains pending; no validator is relaxed. |
| Atomic scope planning and grouped forward fields | **implemented / native rerun pending:** actual owned3.3 `EXPLAIN` showed inline `map` and concatenation/cast expressions produce `TableScan` despite the scope index hint. A preceding compact `LET` yields `IndexScan` on `by_scope`; this is plan-shape evidence, not elapsed-time attribution or a speed measurement. Atomic field windows and DataFusion Eq/IN lowering now prepare native scope constants before selection, retaining schema-owned native string casts and all statement terminals. Shared closure projects each source/frontier's unique forward fields once while retaining typed target epochs and separate indexed reverse reads. Independent source review accepted the grouping and its finite null/shared-target/epoch control; core and test compile checks passed before final helper cleanup. The first prepared-index large-field control passed71.13s; subsequent compact residual preparation and mandatory DataFusion Eq/IN driver retention passed in the final expanded rerun below. The pinned planner ranks prepared family/scope compound prefixes equally, so recognized mandatory atomic Eq/IN uses the explicit field driver/index; arbitrary OR/SQL retains general execution. The expanded actual native control checks Eq/IN/OR filters under frozen, nominal and field selections. Both actual native closure controls passed173.38s on the corrected grouped source (`/tmp/persisted-prepared-native-controls.log`), including unchanged4099 rows/4MiB transfer and the new null/shared-target/distinct-epoch case. Expanded atomic Eq/IN/OR control passed70.71s (`9d7f4d27`, `/tmp/persisted-expanded-atomic-filter-controls.log`), covering frozen-only, nominal and field-bound providers with exact ordered expected rows. Grouped closure is committed as `af9a0339`. Current development bridge build passed23.74s and atomic installation/fresh-process import passed (`/tmp/persisted-python-native-prepared-build.log`, `/tmp/persisted-native-prepared-install.log`). Twelve launcher controls passed after adding immediate MCP failure output. The preceding32MiB compiler loop was stopped on its confirmed exact child/loop and its wrapper exited143/cleaned its store, so it cannot establish acceptance; the fresh loop uses current prepared native operations. |
| Final-source all-target Clippy | **passed:** `UV_NO_SYNC=1 just clippy`, `/tmp/persisted-prepared-final-clippy.log`,1m29s on the prepared-query/grouped-forward/serving-port source. The preceding one collapsible conditional was corrected without changing drain ownership. Functional journeys and final generated/docs leaves remain separate. |
| Strict owner presence, grouped native closure and event-grain selection | **passed:** all three `native_closure_selection` controls,246.38s, and `normalize::call_scope::call_scope_controls::event_grain_keeps_orphans_and_declared_skew_without_unrelated_source_payload`,62.83s, `/tmp/persisted-retained-core-reuse-presence-rerun.log`. Existing4099-row/4MiB demand, null/shared targets and distinct epochs remain; the added control rejects missing/dangling ownership at the selected epoch. The first availability regression attempted forbidden replacement of a completed owner and **failed**; its fixture is corrected to preserve immutable ownership and independently reject invalid admission. Its corrected rerun **passed**,195.09s after1m34 build on `c327adba`, `/tmp/persisted-retained-availability-final.log`; the later recorded-provider premise extension needs its own rerun. A preceding absent-runtime selection failed its prerequisite, and an obsolete module filter selected zero tests; neither establishes acceptance. |
| Exact availability reuse and S0 parameter selection | **implemented / independently source-reviewed:** one successful availability value is reused only under the profile and all five exact completed views. S0 includes incoming `ParameterEntityLink.entity`; ordinary and contextual ownership policies remain separate. Core/lib/affected-test compile check **passed**,40.09s, `/tmp/persisted-core-s0-final-check.log`. Finite model parameter rendering **passed**, one actual control,0.00s after2m13 build, `/tmp/persisted-s0-parameter-render-test.log`; finite S0 grain selection **passed**, one actual control,0.34s after4m15 build, `/tmp/persisted-s0-parameter-scope-test.log`. Error detail improves without relaxing rejection. The generic compiler fixture now performs semantic admission; explicitly selected diagnostic replay remains. |
| Actual native/MCP/evaluator on the retained owned snapshot | **partial native journey:** compilation/admission/direct sealing and all ten tool assertions completed; backup passed, but the first journey failed at restore HTTP import20s. The second native browse-scope/vocabulary journey **passed**; combined Rust result1passed/1failed,1125.50s, `/tmp/persisted-prepared-final-mcp-evaluator.log`. The launcher did not reach Python followups. Root retained and restarted that exact owned persistent fixture with its supervisor paused. The separate actual Python MCP/native evaluator selection returned33passed/2failed because the root omitted the native fault-fixture config; both failed fault/delayed-read selections then **passed** with the missing prerequisite. Logs `/tmp/persisted-retained-native-mcp-evaluator.log` and `/tmp/persisted-retained-native-safe-failures-rerun.log`. Thus35 bounded selections are covered after correction, not one clean assembled pass. These use actual native state and finite independent vectors, not live Qwen or a wheel. |
| Bounded SurrealDB dump import and retained restore retry | **implemented:** pinned `surrealdb-syn`/`surrealdb-sql`3.3.0 own incremental statement parsing and ordered execution units. Each HTTP import preserves `OPTION IMPORT`; explicit transactions remain whole and statement/transaction/request limits remain unchanged. Independent source review found no new parser/finality defect. Publisher compile and both parser controls **passed**, `/tmp/lctx-bounded-import-check.log` and `/tmp/lctx-bounded-import-tests.log`; original/cleanup error preservation is corrected in `03380c43`. Current CLI build **passed**,2m31, `/tmp/persisted-bounded-restore-cli-build.log`. The retained backup **passed**,0.411s. Its bounded restore got through HTTP import, cold state validation and native copy, then **failed** exact expected coverage at restored artifact admission, `/tmp/persisted-retained-bounded-cli-restore.log`. Source diagnosis: `cpg-extract::bundle::build_digest` includes Cargo.lock, so parser pins changed the running executable's provider IDs; cold admission incorrectly regenerated those IDs instead of binding the recorded facts producers. The five exact inputs did not widen. Core admission is being corrected to retain independent family/profile/scope obligations over captured provider identities. Coverage semantics remain enforced; restoration acceptance is open. |
| Both-profile/frontier controls; external transport rerun; CLI; native capability union; MCP/programmatic evaluator; applicable leaves | **pending:** final integration acceptance. The obsolete analytic loop is stopped: `default_off_is_explicit_in_both_profiles` failed optional `Workspace::validate` producer replay after successful compilation,1797.91s; its next selected-analytic run was stopped before the loop advanced. `/tmp/persisted-core-prepared-final-five.log` is not final semantic-admission acceptance. Current-source reruns use the corrected fixture and retain selected diagnostics. |

Functional core controls use the explicit temporary
`--config 'profile.release.package.cpg-core.opt-level=0'`; other workspace O2 and dependency O3
settings remain unchanged. Two optimized code-generation attempts were stopped before runtime
without compiler errors; they are readiness observations, not catalog measurements. GDB localized
O0 entry frames: native write polling was corrected at its library boundary; full-driver controls
use child-only `RUST_MIN_STACK=8388608` for the temporary unoptimized driver. Neither change alters
committed build profiles or runtime stack settings. There is no speed claim from these timings.

Host space was restored after owned fixture teardown; only two confirmed inactive obsolete IPC/
spill scratch directories were removed. Shared caches, benchmark captures and unrelated host
processes remain preserved. The independent implementation review's original two-family cold
non-graph F01 has targeted Tested evidence; the fresh30 native controls above retain its actual
corruption checks. Broader required backing coverage remains under final targeted verification.
Enclosing publication/transport and whole-plan acceptance remain separate. Real FastMCP/Q1 pilots,
live Qwen qualification, protected evaluation activation, operator adoption and push remain held.

## 10. Library fit, remaining investigations and future change

The comprehensive neo4j-surrealdb skill research from the source review was reused, then deepened
for persisted contribution/view storage, generated complete bodies, streaming, constraints and
concurrency. Context7 resolved `/surrealdb/docs.surrealdb.com` and queried streaming, transactions
and schemas separately; current documentation is a lead, exact3.3.0 source controls transferred
contracts. No new dependency, backend or server configuration is selected in this planning work.

| Capability / alternative | Selected placement and limit |
|---|---|
| Indexed RecordIds, compound predicates, native sets/adjacency | Shared selected access over completed membership. Full nominal closure remains exact batched one-hop plus Rust visited set. |
| Query executor/planner | Exact3.3.0 source inspection, 2026-10-07: gRPC streaming reaches the executor's new planner under the default best-effort strategy, with legacy fallback for unsupported/unimplemented planning. The fixture does not override this default. Nominal reference Eq/IN already lowers to indexed prefixed `scope_keys CONTAINS` predicates. This establishes the implemented route, not a particular chosen index or absence of fallback for every query. No mandatory per-query proof is added. [Server default](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/dbs/mod.rs#L295), [gRPC streaming route](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/rpc/grpc.rs#L1609). |
| gRPC SDK streaming and message limits | Reuse `stream_items` and existing statement/transport finality. Native projected Values enter generated Arrow builders; pinned SDK rejects unsolicited Arrow payloads. The SDK follows the advertised server message ceiling. The managed deployment and owned fixture require `SURREAL_GRPC_MAX_MESSAGE_SIZE=128MiB` for the existing64MiB native single-row bound plus framing; transactions retain row/byte limits. Operator deployment remains held. [Exact SDK source](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/grpc.rs#L748-L764), [server configuration](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/cnf/mod.rs). |
| Index maintenance and build timing | Compiler/enforcement indexes early; serving-only secondary indexes after load with a checked synchronous barrier. [Official index documentation](https://surrealdb.com/docs/reference/query-language/statements/define/indexes); exact versioned source/test evidence is in the source review. |
| INLINE selectors / INLINE EDGES | Classic attributed edges and their existing indexes are initial storage. Investigate narrow INLINE field/role/position selectors only if a migrated query has a material hydration issue; test fallback/missing/null/multiplicity. Do not add table-wide adjacency caches by default. |
| LIGHTWEIGHT / `+collect` / import shortcut | Not replacements for attributed exact relationships, full-key closure or checked loading. Preserve the source review's capability limits. |
| Native storage caches/durability | Reuse managed RocksDB/shared native caches and Every durability; no application block cache or weaker sync policy. |
| Embedded primary store / raw Arrow protocol | Not selected: second lifecycle or unstable core/new protocol scope without a current requirement. Remote persisted host already serves compiler and consumers. |
| GQL, MCP, SurrealKit, live/changefeeds and extra frontends | No demonstrated role in the required corrections. Revisit only for an actual consumer; no new migration/coordination service for fresh generated databases. |

Native persistence/direct sealing remains selected. The scope compiler can replace physical
routes where its owned operation is better served by graph or relational kernels; the correction
plan's §7 still owns actual single-prefix/contributor/published-plan, failed-import and captured-build
admission investigations. PC4's bounded external ordering remains a useful lowering. Supersession
requires an explicit preservation/consumer/evidence map, not deletion of these obligations.

The earlier bounded investigations remain where their implementation evidence is still incomplete:
PG1 verifies complete registry coverage and membership index/key layout with real field types;
PG2 inspects chosen native plan shapes when selectivity or ordering is uncertain; PG4 verifies the
compact S0 spool and actual selected-method dependency union; PG8 verifies allocation/retention
ordering and necessary delivery stabilization. Resolve them before their dependent package is
declared ready; they do not reopen native query adoption or introduce mandatory per-query proof.

Release/configuration changes use GR3/GR4's declared dependency graph to identify affected pure
products. Selective cross-run execution is explicitly requested and scheduled; no overlap deferral
remains. It handles deletions, changed membership, missing targets and negative selections, not
only updated returned rows. Exact-view dependencies remain until a complete finer domain is
implemented; current provenance/admission refresh even when a pure value is equal.
An added analytic requests its actual inputs and reuses the same access/preparation; a new original
consumer reuses verified range batches; a new record extends the model and mechanical codecs,
not a second repository/classifier. A new transport consumes completed typed output through the
bounded encoder, without redefining semantics. These are extension scenarios, not additional
features silently scheduled here.

## 11. Current plan checkpoint

**2026-10-09: unified persistence implementation in progress.** UP0–UP8 source changes provide
shared immutable content, exact scoped readers, checked admitted attachment, durable effects,
manifest publication, data-only restore and stable service/recovery tooling. Integration review
corrections and actual installation/controls remain in progress. Source findings remain Open in §8.
The common final acceptance is UP9 with the surviving obligations identified in §7.3. Historical
timeouts, integrated implementation and bounded passes below are not rewritten or promoted.

**2026-10-09: GK0–GK6/GR0–GR5 integrated, Implemented / focused Tested; enclosing qualification open.**
ADR-0139 and the architectural owners install model-owned relation scope compilation, canonical
program identity and exact-equality XXH3 interning. Finite/native root partitions and bounded
shared hydration reach normalization/admission, upper consumers and pinned native serving.
The [independent assembled review](../design_review/reviews/design_review_assembled-graph-scope-compilation_2026-10-09.md)
accepts the inspected source at Implemented strength; §8 owns its source-qualified findings.
The 288-control focused selection has passing evidence after the repaired exact-two SourceCall
rerun; its initial composite remains failed. Affected all-target Clippy and applicable leaves passed
(§9.1). This is not whole-plan qualification or measured benefit.
ADR-0140 adds portable products, selected-domain reuse, current dependency metadata and
viewer preparation. The independent reuse-contract review and repaired controls are recorded
in §8/§9.1. GK7/GR6 qualification and BC3 remain open; full compiler matrices timed out.
Prior timeouts and PC/CU/BC obligations survive under §7.1.
No operator database/configuration, activation, real-library pilot or push is authorized.

**2026-10-08: PC0–PC6 execution resumed; corrective target accepted under ADR-0138.** The [independent correction-plan review](../design_review/reviews/design_review_persisted-execution-corrections-plan_2026-10-07.md) accepted PC0–PC6 at Proposed strength, with no remaining blocking design finding; runtime closure remains open. Native compiler and adjacent shared operations are on main; current targeted receipts and remaining checks are in §9.1. The correction plan develops all six new findings and both accepted rule changes; §8 owns their current disposition. The independent
[target-plan review](../design_review/reviews/design_review_persisted-graph-execution-plan_2026-10-07.md)
accepted the Proposed architecture. Its frozen-universe, deferred-reference, exact-statistics
and complete-body obligations remain binding during implementation. Source acceptance and
bounded successful controls do not establish the whole pivot or measured compilation speed.

PC0–PC5 contracts and consumer migrations remain integrated in the committed baseline. PJ0–PJ3
and bounded assembled-review corrections are implemented on main as `2a62e817`; prior targeted
acceptance and its remaining timeouts/qualification limits stay in §9.1. Earlier populated passes
describe the preceding physical schema. The independent assembled review accepts its scoped
static/Implemented source, not the new GK/GR target or whole-plan performance. Normalization and
validation amplification now have scheduled replacement packages GK2/GK3/GR4; causal runtime
attribution remains unproven. Assembled `qualify`, affected timeout reruns and PC6/CU6/BC3/BC5 are
open under the final-source route in §7.1. PC3's dirty worktree and shared profiling/build assets
remain preserved. No push or operator adoption is requested.
