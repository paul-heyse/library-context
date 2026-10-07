# Persisted graph compilation and shared efficient execution

**Proposed implementation target, 2026-10-07. Not implemented or performance-measured.**
This document coordinates the next hard design pivot. Its source is the
[catalog compilation speed review](../design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md),
especially F01–F05 and RC01/RC02, together with the operator's 2026-10-07 selection of persisted
graph compilation and comparable corrections throughout the codebase. The operator confirmed
direct sealing, internal contribution/view identity, and dependency foundations now rather than
cross-run incremental execution. Those choices replace the review's preferred in-process compiler
alternative; its diagnoses, preservation obligations and uncertainty remain applicable.

This plan owns these scheduled findings and the additional source-inspected inefficiencies in §8.
The existing graph-native and evidence/evaluation coordinators retain their original findings and
dated receipts. They do not close the new findings. The architectural collection owns enduring
contracts; PG0 changes those contracts before dependent implementation. This plan does not change
production code, activate operator state or promote the interrupted pilot into a successful result.

## 1. Outcome, baseline and boundaries

Persist captured inputs, completed typed products and their dependency relationships in the
managed local SurrealDB graph during compilation. Use indexed set-shaped access, graph adjacency
and suitable native computation over those inputs; retain model-owned Rust kernels and bulk
Arrow/DataFusion computation where they fit the actual operation. Compile, admit and seal the
same private native database. Ordinary publication must not export, reconstruct and re-ingest
the compiler's own output.

The stored option is selected for useful graph access now and a durable foundation for future
selective invalidation and other consumers. Persistence alone does not remove repeated work.
The target also replaces the identified variants with common access, conversion, batching,
preparation and validity mechanisms. Domain policies remain with their semantic owners.

Inspected baseline: main `ccbcae1f06610d00564d8b74078ae642a5c548e9`, 2026-10-07. Open edits in
AGENTS, STATUS, DESIGN, existing plans/runbook and a test docstring were preserved. Production
sources remain the review's `9fecc014` baseline. The existing implementation already persists its
final graph; the changed boundary is compilation itself and reuse of its exact completed state.
Native serving, publication, embedding/cache, retrieval and the primary programmatic evaluator
have the focused receipts recorded by their coordinators, not new acceptance from this plan.

The real Catalog attempt stopped after65m31s with SIGINT/exit130 and no verdict. A brief stack
sample did not identify its query/stage or dominant cost. Static findings justify removing known
work without a latency estimate or mandatory benchmark. The proposed speed and reuse benefits
are unmeasured. Rust build time, GPU/model upgrades and a performance campaign are not this scope.

Migrate all affected compiler frontiers (`facts`, `normalized`, `analysis`, `catalog`), Catalog
and Behavioral profiles, shared projection/analysis/admission consumers, and the non-compiler
paths in §2. Behavioral functionality is preserved, not expanded. Cross-run invalidation,
automatic recompilation, resumable attempts, distributed execution, compatibility readers,
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
| Review RC01 | **Accepted:** ordinary compilation seals its own admitted persisted realization; external imports and stored reconciliation remain independent exposures. | PG0 supersedes ADR-0128's portable-roundtrip/post-compile-only publication clauses and updates storage §6.1. |
| Review RC02 | **Accepted:** exact immutable contribution/view identity replaces eager whole-prefix content identity internally. Deterministic final content identity remains. | PG0/PG1 revise `SourceSnapshot`, completed-input bindings, manifests, observation/cache identities and their consumers. Do not relabel membership as canonical content. |
| Native compiler placement | **Selected explicitly:** store-free compilation is displaced. | PG0 replaces DESIGN §B3/§B7/§B12, semantic-model §15.1/§15.11, storage §5/§6/§6.2 and associated instructions. Restate surviving ADR-0128 obligations in the replacement ADR. |
| Reuse horizon | **Foundations now:** persist exact completed inputs, meaningful dependencies and products; no cross-run scheduler/invalidation executor. | PG1/PG7 give the persisted state real inspection/transport consumers; defer the executor with a named trigger in §10. |

Plan approval is not implementation or finding closure. The old architectural owners still describe
the implemented store-free baseline until PG0 installs the selected target with explicit labels.
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
table per kind and not a duplicate of registered graph payloads. `QualityStep` is a known such
record; originals/`ArtifactChunk` use the existing bounded original-chunk owner rather than a
second full-byte copy. PG1 must reconcile the complete model relation and graph registries;
undeclared required records fail registration instead of disappearing through dispatch.

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

## 5. Shared operation mechanisms and efficient placement

### 5.1 Native access and request-local indexing

Extend the existing native reader/loader capabilities rather than add a backend-neutral provider
framework. Internal operation context binds either an exact private completed view or a sealed
read-only snapshot, plus demand, budget, cancellation and lifetime. A private input view is not a
published `SnapshotHandle`; do not forge a handle or bypass its publication-marker check.

Selections express finite typed ID sets, declared reference fields/direction, required columns,
source ranges or a model-owned set operation. Private reads include completed-view membership;
sealed reads include snapshot and semantic-family membership. Domain owners choose closure and
coverage. Values remain SDK bindings; arbitrary request-controlled SQL is not introduced.

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
until statement and outer transport completion. Dependent outputs remain private and are discarded
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
invalidate document values. No generic new cache, GPU upgrade or automatic cross-run reuse.

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

PG2 may begin with PG1's narrow working slice, but PG1's remaining contracts/consumers stay open.
PG3/PG4 milestone boundaries do not create one transaction/task/table per relation. PG6 and PG8
can advance independently when their real prerequisites exist. PG7 designs transport with PG1
and lands before complete publication/backup claims; PG5 is not complete on a metadata-losing
restore. Shared model/native-reader/codec files need one writer even when logical work is parallel.

PG0 must replace conflicting future instructions in the graph-native coordinator and its four
supporting plans. Those plans retain semantic responsibilities and original receipts; this document
owns the combined new execution and finding disposition. The evidence/evaluation plans consume
the changed native boundary without becoming a second correction ledger.

## 8. Sole finding disposition

All corrections below are **Open / Proposed**. Acceptance of a decision or this document is not
verified closure. Source-review IDs retain their original scope; additional IDs below are this
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

Retain source links and responsible owners when combining related corrections. Do not close F01
solely on a native database rename, F02 on moving the same prefix work into an index builder, or
F03 by deleting checks. Additional exposed consumers join the responsible package before closure.

## 9. Targeted functional verification and completion

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

## 10. Library fit, remaining investigations and future change

The comprehensive neo4j-surrealdb skill research from the source review was reused, then deepened
for persisted contribution/view storage, generated complete bodies, streaming, constraints and
concurrency. Context7 resolved `/surrealdb/docs.surrealdb.com` and queried streaming, transactions
and schemas separately; current documentation is a lead, exact3.3.0 source controls transferred
contracts. No new dependency, backend or server configuration is selected in this planning work.

| Capability / alternative | Selected placement and limit |
|---|---|
| Indexed RecordIds, compound predicates, native sets/adjacency | Shared selected access over completed membership. Full nominal closure remains exact batched one-hop plus Rust visited set. |
| gRPC SDK streaming | Reuse `stream_items` and existing statement/transport finality. Native projected Values enter generated Arrow builders; pinned SDK rejects unsolicited Arrow payloads. [Exact source](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/grpc.rs#L748-L764). |
| Index maintenance and build timing | Compiler/enforcement indexes early; serving-only secondary indexes after load with a checked synchronous barrier. [Official index documentation](https://surrealdb.com/docs/reference/query-language/statements/define/indexes); exact versioned source/test evidence is in the source review. |
| INLINE selectors / INLINE EDGES | Classic attributed edges and their existing indexes are initial storage. Investigate narrow INLINE field/role/position selectors only if a migrated query has a material hydration issue; test fallback/missing/null/multiplicity. Do not add table-wide adjacency caches by default. |
| LIGHTWEIGHT / `+collect` / import shortcut | Not replacements for attributed exact relationships, full-key closure or checked loading. Preserve the source review's capability limits. |
| Native storage caches/durability | Reuse managed RocksDB/shared native caches and Every durability; no application block cache or weaker sync policy. |
| Embedded primary store / raw Arrow protocol | Not selected: second lifecycle or unstable core/new protocol scope without a current requirement. Remote persisted host already serves compiler and consumers. |
| GQL, MCP, SurrealKit, live/changefeeds and extra frontends | No demonstrated role in the required corrections. Revisit only for an actual consumer; no new migration/coordination service for fresh generated databases. |

The major route is settled. Remaining bounded investigations refine implementation within it:
PG1 verifies complete registry coverage and membership index/key layout with real field types;
PG2 inspects chosen native plan shapes when selectivity or ordering is uncertain; PG4 verifies the
compact S0 spool and actual selected-method dependency union; PG8 verifies allocation/retention
ordering and necessary delivery stabilization. Resolve them before their dependent package is
declared ready; they do not reopen native query adoption or introduce mandatory per-query proof.

Future release/configuration changes can use persisted dependencies to identify affected products;
the executor remains deferred until that incremental workflow is explicitly requested. It must
handle deletions, changed membership and negative selections, not only updated returned rows.
An added analytic requests its actual inputs and reuses the same access/preparation; a new original
consumer reuses verified range batches; a new record extends the model and mechanical codecs,
not a second repository/classifier. A new transport consumes completed typed output through the
bounded encoder, without redefining semantics. These are extension scenarios, not additional
features silently scheduled here.

## 11. Current plan checkpoint

**2026-10-07: plan authoring/static investigation only.** No production changes, operator actions,
new compilation, product tests, performance probes or dependency moves were performed for this
document. Existing current-tree receipts retain their original boundaries. All PG packages and
§8 corrections remain Open / Proposed. The independent
[target-plan review](../design_review/reviews/design_review_persisted-graph-execution-plan_2026-10-07.md)
accepted the architecture and dependency sequence at Proposed/static-inspected strength, with
no additional blocking findings. Its deferred-reference, frozen-universe, exact-statistics and
complete-body preservation obligations constrain PG1/PG2.

Documentation controls, 2026-10-07: **passed** `UV_NO_SYNC=1 just docs-check` (329 canonical
pages; zero link errors after correcting one section anchor), **passed**
`UV_NO_SYNC=1 just lint-agents`, and **passed** `git diff --check`.
Logs: `/tmp/lctx-persisted-graph-plan-docs-check.log` and
`/tmp/lctx-persisted-graph-plan-agents-check.log`. These establish proposal/publication quality,
not implementation. Product builds, tests and pilots are **not_run** for this documentation scope.

Next: execute PG0–PG2's real shared foundations before dependent compiler and consumer migration.
No clean-tree claim is made for the preexisting open edits; preserve them during execution.
