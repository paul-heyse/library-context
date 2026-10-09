# Graph compilation, persisted execution and selective reuse

Command excerpts identify verification scope; original invocations remain in the cited logs and Git history.

**Accepted persisted foundation and corrective target; production integrated, focused acceptance incomplete, 2026-10-08. No performance measurement.**
**Proposed physical-execution refinement, 2026-10-09:** the [populated-journey companion](populated-journey-execution-amplification-plan_2026-10-09.md) develops operation-shaped native enforcement and final canonical preparation. This coordinator remains the sole scheduled-finding and integrated-acceptance owner. Operator acceptance of its RC01/RC02 does not apply the new schema or establish implementation.
**Proposed replacement target, 2026-10-09:** the operator accepted graph/hash RC01 and RC02.
The [compiler companion](graph-compilation-kernels-and-hashing-plan_2026-10-09.md) and
[reuse companion](graph-compilation-reuse-and-invalidation-plan_2026-10-09.md) replace conflicting
open per-root/SQL-only and dependency-foundations-only routes with model-owned relation compilation,
shared prepared demand and selective cross-run reuse. Earlier work is not a prerequisite to design
this replacement; its surviving guarantees and acceptance obligations remain owned here.
This document coordinates the next hard design pivot. Its source is the
[catalog compilation speed review](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md),
especially F01–F05 and RC01/RC02, together with the operator's 2026-10-07 selection of persisted
graph compilation and comparable corrections throughout the codebase. The operator confirmed
direct sealing, internal contribution/view identity, and dependency foundations on 2026-10-07.
The 2026-10-09 accepted reuse horizon below supersedes that choice's cross-run deferral. Native
persistence and direct sealing remain foundations; the new operation compiler does not restore a
second canonical store. The earlier review's diagnoses and preservation obligations remain applicable.

The [correction-causes review](../design_review/reviews/design_review_persisted-execution-correction-causes_2026-10-07.md) subsequently identified incomplete admission/completion contracts and native execution amplification. Its [supporting correction plan](persisted-execution-corrections-plan_2026-10-07.md) defines PC0–PC6, integrated below. Both correction-causes RC01/RC02 were operator-accepted on2026-10-07. They are different from the catalog-speed rule impacts above.

This plan owns the sole scheduled disposition of its source reviews and the additional source-inspected inefficiencies in §8.
The existing graph-native and evidence/evaluation coordinators retain their original findings and
dated receipts. They do not close the new findings. The architectural collection owns enduring
contracts; PG0 changes those contracts before dependent implementation. The operator resumed PC0–PC6 production work on 2026-10-08; ADR-0138 now consolidates its accepted target. The targeted acceptance boundary in §9 remains authoritative. Operator adoption remains held; the interrupted pilot has no successful verdict.

## 1. Outcome, baseline and boundaries

Persist captured inputs, completed typed products and their dependency relationships in the
managed local SurrealDB graph during compilation. Use indexed set-shaped access, graph adjacency
and suitable native computation over those inputs; retain model-owned Rust kernels and bulk
Arrow/DataFusion computation where they fit the actual operation. Compile, admit and seal the
same private native database. Ordinary publication must not export, reconstruct and re-ingest
the compiler's own output.

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

Use the existing managed authenticated SurrealDB3.3 RocksDB server and remote Rust gRPC SDK.
One compilation owns one STRICT private database from first persisted output through final
sealing. `lctx-surrealdb` owns mechanical schemas/codecs/access/write finality; `cpg-core` owns
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
| Typed row key | Relation plus complete nominal semantic key; native graph target is a mechanical mapping. Same key/different payload is a conflict; identical rows share one value. Hash equality alone never authorizes conflicting payloads. |
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
under unchanged premises; an unknown write/COMMIT acknowledgement fails the disposable attempt.
There is no resumability journal. Drain and abandon only that owned private database; report an
unselected orphan if cleanup fails, never sweep unrelated operator databases or claim success.
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

Install identity/enforcement and compiler lookup indexes before their first use. Build serving-only
secondary/full-text/HNSW indexes after their rows are available and before sealing. Initial builds
use checked synchronous DEFINE INDEX as a simple readiness barrier; concurrent builds are not
selected for this private-load lifecycle. Scope indexes used by compilation stay available early.
Do not postpone uniqueness constraints or imply index construction is free.

Direct sealing accepts an owned completed native authority, not a public caller-supplied marker.
All writers/read streams drain; final semantic admission, independently stored content/state
reconciliation, search readiness and executable-definition inventory succeed; writable ownership
ends; VIEWER reconnect and complete marker checks succeed before emitting an **unselected** handle.
Reuse the existing publisher lifecycle and server; ordinary compilation performs no portable
export/re-import or final bulk database re-ingestion. Selection and reader-drained retirement stay
explicit. No whole extraction/compute/inference transaction or mutable selected snapshot.

| Interface | Target behavior |
|---|---|
| `lctx compile … --through … --profile …` | Same cumulative frontiers/profiles; runtime config/native readiness now apply to every compilation target. Outputs an unselected sealed persisted result. |
| `--artifact-only --output DIR` | One native compiler produces a new-format complete portable export without selected-snapshot adoption. The native dependency is explicit; no store-free fallback compiler. Owned private staging is discarded after verified export/file completion. |
| `publish-artifact` | Fresh private import of the new complete graph plus completed state; external semantic/reference/state validation remains. No old-format reader. |
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

GK0/GR0 schedule architectural decisions and owner changes; this plan-authoring turn applies
neither production changes nor new runtime policy. Operator databases/configuration and activation
remain held. Native persistence does not require every domain operation to be a native query,
and compiled graph intent does not require every algorithm to be serialized into a universal IR.

## 8. Sole finding disposition

### Graph/hash review findings transferred on 2026-10-09

Both findings are **Open; corrective target Proposed and rule changes operator-accepted**.
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

### Plan-target review finding transferred on 2026-10-09

The [independent target-plan review](../design_review/reviews/design_review_graph-compilation-and-reuse-plan_2026-10-09.md)
accepts the revised combined target at **Proposed/interface strength**. Its F01 is distinct from
source graph/hash F01. The document correction resolves its target-order gap; implementation
and the revealing lifecycle control remain **Open / not_run** under GR5/GR6.

| Source finding | Responsible component and disposition | Required implementation evidence |
|---|---|---|
| [Plan-target F01](../design_review/reviews/design_review_graph-compilation-and-reuse-plan_2026-10-09.md#F01) — cache insertion racing viewer retirement | Viewer/cache lifecycle, GR5/GR6. **Target corrected at Proposed strength, 2026-10-09**: fence admission/retries, drain initialization/insertion owners, release final cache references, then wait external leases. | Race completing initialization against retirement; no cache-owned pin/charge survives final release, while an external borrower retains its own pin/charge until release. Production closure requires that control and actual consumer integration. |

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

**Current execution dependencies, Proposed 2026-10-09:** PC1/PC2's integrated completion/captured-binding contracts are working foundations for PJ0–PJ3. PJ2 replaces PC3's physical enforcement route only after PJ0, and preserves PC4's exact selection/access obligations. PJ3 consumes current retained setup and acknowledged ordering ownership; its mutation-only freeze is distinct from final global drainage. PJ1 runtime/phase controls can proceed independently. PJ2/PJ3 share native and cold/publication editing surfaces even when logically ready in parallel. PC5 SQL unit/request semantics remain selected; no data-only transport is introduced. Integrate PJ2/PJ3 before repeating the populated journey and reuse only valid matching-source/profile acceptance at this coordinator.

PJ5 and existing PC6/CU6/BC5 consume the resulting native/CLI/cold/MCP controls; no plan's receipt silently closes another plan's obligations. BC3 retains its candidate/default qualification gate and active release route. The operator-deferred timeout follow-up remains deferred. Assembled release qualification required by changed shared trust contracts stays an explicit open completion obligation until its deferred scope is reactivated; do not silently run it during authoring or substitute focused passes for it.

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

**2026-10-09: graph/hash replacement target planned; production paused.** The operator accepted
both source-review rule changes and instructed stronger designs to supersede overlapping open
routes. GK/GR companions now own the detailed target; §7.1 maps replacement and preserved
acceptance, §8 owns transferred F01/F02. Static/interface evidence supports the chosen compiler
and content-addressed reuse direction; implementation/benefit remain Proposed/unmeasured.
The [independent target-plan review](../design_review/reviews/design_review_graph-compilation-and-reuse-plan_2026-10-09.md)
accepts the revised target at Proposed/interface strength, after correcting the GR5 retirement
order; §8 retains its separate implementation obligation. Documentation outcomes are recorded
in STATUS. Next execution planning uses GK0/GR0/GK1 and working prerequisites, not a return to
the older deferred normalization route.

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
