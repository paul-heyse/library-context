# Graph-native model and store-free compiler

**Accepted target / implementation in progress, 2026-10-05 (ADR-0128).** Supporting plan for the [replacement coordinator](graph-native-pivot-plan_2026-10-05.md).
It develops M1, C1, C2-N and C2-U. The coordinator owns package state, cross-plan dependencies
and finding disposition. The design basis is its two named reviews, not another model review.

## 1. Foundation assessment and resulting boundary

`lctx-model::domain` already owns attributed observations, nominal identities, source/context
correspondence, five-way verdicts, conditions, typed catalog/selection, projection semantics and
wire contracts. Keep these meanings and useful pure kernels. They are foundations to reshape,
not a requirement to preserve every declaration, relation, stage, epoch or validator attachment.

**Implemented; integrated functional acceptance pending, 2026-10-05:**
`cpg-core::compilation::compile` builds immutable completed Arrow IPC streams in a private,
spillable workspace. Explicit semantic predecessor selectors keep native Facts separate from
Local, Model and Analytic vocabulary. Typed graph lowering and artifact admission/export are
separate from publication. The PostgreSQL compiler, runtime grants and `MemoryGeneration` are
retired; canonical declarations and semantic policy revisions own model compatibility.
The coordinator's current checkpoint owns actual verification outcomes.

The compiler instead receives explicit captured inputs, profile/method settings, semantic
definitions, resource/runtime services and optional embedder/cache effects. It owns a private
workspace and returns a completed **admitted graph**. No database connection, database role,
stage grant, readback or publication transaction is required to extract, normalize or analyze.
The publisher consumes the artifact through its [realization contract](graph-native-surrealdb-realization-plan_2026-10-05.md).
Selecting a snapshot is outside both operations.

## 2. Graph semantics and identity — M1

### 2.1 Author the smallest sufficient graph

The Rust model declares typed kinds/properties, identity recipes, endpoint/participant roles,
provenance, qualifications, coverage, domain operations and admission rules. An attributed
assertion is distinct from its subject entity. Provider disagreement adds separately supported
assertions; it does not overwrite an entity property. A public exposure is distinct from its
declaration, invocation variant and contextual occurrence.

| Graph meaning | Required representation |
|---|---|
| Entity | Independent identity and typed intrinsic data; exists without incident edges |
| Binary assertion | Its own ID, kind, ordered endpoints/roles, scope, qualification and evidence; parallel assertions survive |
| N-ary assertion / derivation | Addressable assertion with role-labelled participants; derivation additionally names rule/revision, ordered premises, assumptions and outcome |
| Source / evidence / run | Shared immutable context, exact original bytes/spans and provider/environment identity; compact links from assertions |
| Coverage / outcome | Expected domain and observed result with complete-empty, partial, unavailable, failed, refused and NotRequested distinctions |
| Retrieval / vector use | Text/unit identity, contextual occurrences, family/member and exact embedding consumption; heuristic witness separate from requirement evidence |

Small exclusively owned values can remain nested typed payloads. Independently cited or reused
values remain addressable. Give n-ary structures real roles instead of flattening them to binary
edges. Classic relation assertions can be referenced by other assertions where the adapter's
encoding supports it; reification is also available through the same semantic mapping.

Retire relation-registry-as-published-schema and operational vocabulary history. Typed record
declarations and Arrow batches may remain useful internal computation views; their number does
not dictate physical families or execution stages. Do not automatically wrap all old tables in
generic payload nodes. For each current producer, map its required semantics to these graph forms,
merge incidental bookkeeping, and remove data without a current semantic/serving consumer.

Conditions retain canonical evaluation-atom/BDD meaning, not library-local indices. A private
attempt may assemble a deterministic vocabulary with dependency-local completed views. Final
admission checks the complete referenced vocabulary. No persisted epoch/grant/receipt choreography
is needed to enforce a compiler dependency. Codebook allocations remain append-only.

### 2.2 Separate identity scopes

| Identity | Owned inputs and invalidation |
|---|---|
| Logical entity/assertion | Declared semantic key, contextual/source identity where meaningful; physical database names never enter it |
| Captured source | Exact bytes, release/environment/acquisition identity and source coordinates |
| Semantic contract | Canonical declaration structure, explicit semantic policy/invariant revisions, codebooks and operation meanings |
| Producer implementation | Provider/compiler implementation and relevant dependency/configuration identities; provenance and safe reuse dependency |
| Graph content | Canonical complete payloads and links, outcomes/coverage, sources and consumed embedding values/spec |
| Physical realization | Codec/layout, functions, indexes/analyzers, engine and optional module artifacts; owned by publisher |

Current `Id<T>` is structurally keyed, not directly a source-text hash. Preserve the nominal/key
principle while deliberately revising recipes where the new meaning needs it. Remove broad
source/manifests/lock hashing from **semantic contract compatibility**. Keep implementation
fingerprints where a producer/reuse result actually depends on them. An unrelated comment or
storage edit must not rename unrelated entities or invalidate semantic installation; a changed
semantic rule must invalidate its consumers. No old-ID reader or translation table is added.

Use explicit canonical encodings with stable ordering and discriminants. Compare complete
payloads when keys collide: identical observations deduplicate, conflicting same-key payloads
refuse unless a named model merge owns that case. Human formatting/Debug/ordinary JSON and
engine numeric coercions do not define identity. The physical adapter maps IDs mechanically to
typed canonical-string record keys.

### 2.3 Admission product

The manifest identifies input captures, contract/implementation/settings, requested frontier and
profile, graph-family content/counts, coverage/outcomes, originals, selected projection/analysis
definitions and exact consumed embedding spec/values. The admission result binds that immutable
content and the checks completed by their semantic owners. It is not a certificate of unrestricted
Python truth or a performance proof.

Construction enforces local shape/ranges, nominal roles and key consistency. Completion checks
reference closure, required domain outcomes, coverage and cross-element invariants once over
completed inputs. External unresolved Python targets are typed uncertainty; absent internal
endpoints are invalid. Forward/cyclic references stay private until closure succeeds. Decodeable
partial producer output is not admitted merely because its records are well formed.

Retain a cheap algorithm-specific certificate/check only when it establishes a property that
publication genuinely needs. For SCCs, component order alone does not establish strong
connectivity; for total outputs, compare actual keys with the independently defined obligation
universe, not just counts. A post-fixpoint check does not establish a least fixpoint. Witnesses
establish their declared witness claim, not path absence. Preserve exactness where required;
keep a narrowly necessary validation when no adequate cheaper replacement exists. Do not invent
a generic proof system or replay every producer on every compile. Independent small controls
primarily establish algorithm correctness.

## 3. Bounded workspace and producer migration — C1/C2

### 3.1 Physical compilation route

Use attempt-owned columnar batches and immutable temporary segments. Reuse Arrow/DataFusion
for bulk joins, grouping, sorting and set membership where suitable, and charged native kernels
for finite semantics. Spill sorting/join inputs rather than collecting whole relations. Keep one
owner for buffers, segment lifetimes, CPU pools and cancellation/drain. Account for practical
resident intermediates and disk use without adding a detailed work-accounting subsystem.

Write each completed producer output into its final canonical segment stream once. Compatible
consumers share prepared ordered views; a different required ordering may justify an external
sort/index. Resolve references with batched sorted/indexed membership, not per-field remote
anti-joins. Share canonical content hashing with ordered admission streams where possible.
Domain dependencies establish which completed views a producer can consume. Temporary segments
are workspace data, not another authoritative published database.

Whole-topology algorithms necessarily materialize their named topology; that allocation is not
made scalable merely by refusing it. Reuse one compact prepared projection across compatible
methods, and keep unrelated evidence/payloads out of it. The [projection plan](graph-native-projections-plan_2026-10-05.md)
owns that representation and its analytical lifetimes. Discard temporary state once no consumer
needs it. Restart a failed disposable compile; persistent arbitrary-stage resumability is not
required.

### 3.2 Producer closure

| Slice | Result and migrated consumers |
|---|---|
| C1 facts | Acquisition and independent Ruff/Pyrefly/ty capture emit attributed typed graph records and original chunks; profile outcomes remain explicit. Facts admission works with no running database. |
| C2-N normalized | Occurrences/entities/places, exposure/declaration correspondence, signatures, call targets/bindings, conditions and projection bases consume completed facts/workspace views. Preserve all alternatives, roles and unresolved inventory. |
| C2-U semantic upper | Local, execution/transfer, finite models/Summary, Structural and their selected definitions/outcomes/premises consume explicit predecessor views. Remove PG checkpoint adapters and blanket stored replay. |
| C2-U product | C0 public universe/options/contracts, C1 source/scenario/deployment associations, C2 predicate-domain selection basis, S0 assertions/briefs, contextual retrieval and embedding uses construct the graph. Catalog does not depend on optional analytics or brief availability. |
| C2-U selected analytics | A1/A2 supply projection inputs/native kernels. Disabled methods emit NotRequested; actual selected failures/partial outcomes retain their meaning. |

These names identify existing semantic responsibilities, not mandatory new crates/stages.
Migrate producers **and all their consumers** with each boundary. `CompletedStore`, `StageAccess`,
store-bound DataFusion registrations, persisted stage grants, vocabulary delta tables and callback
replay factories disappear. Ordinary explicit completed-input views replace their dependency
meaning. No adapter emulates an old database grant over an in-memory graph.

For corpus evidence, preserve original enclosing context and declared corpus/library links;
same-input or same-database presence does not establish association. Retain native Bound/unattached
formulas without manufacturing Entry proof. Source field location, Terminal/raised typing,
navigation and source similarity remain their stated evidence, not runtime guarantees. Negative
claims retain complete scoped premises. This is domain fidelity, not preservation of old storage.

Embedding is an explicit effect after admitted token/text preparation and outside transactions.
The cache may be available, but facts/normalized and embedding-unrequested catalog compiles do not
open it. Exact consumed cache/service winners enter the artifact; later reconstruction uses those
bytes, never a live service lookup. Batch duplicate texts by complete spec/text identity and share
winning values across analytics/retrieval consumers without equating their semantic roles.

## 4. Packages, controls and completion

| Package | Implementation boundary and focused control |
|---|---|
| M1 | Revise declarations/identity/admission and generated wire/codec consumers together. Hand-authored graphs challenge conflicting supports, parallel assertions, isolates, n-ary roles, ordered premises and internal/external missing targets. |
| C1 | Workspace streaming/spill, captured originals and facts producers. Compile a fixture with no DB configuration; forced-small workspace gives the same canonical artifact as an ordinary workspace, and cancellation exposes no completed artifact. |
| C2-N | All normalization adapters use workspace inputs. Independent signature/binding/unresolved/source-context cases preserve known answers and unknowns; insertion/batch order does not change canonical meaning. |
| C2-U | All selected semantic/product consumers and A1/A2 integration. Missing obligation keys, partial coverage, damaged provenance and unavailable optional analyses stay distinct; catalog works independently of briefs. |

Identity controls distinguish comment/storage edits from semantic rule changes, and complete
payload changes from unchanged keys. Artifact controls include missing/reordered/tampered chunks,
non-UTF8 original bytes and exact source-span expansion. Certificate controls challenge their
actual claim: incorrectly merged SCC, valid post-fixpoint labelled least, missing required outcome,
and path witness promoted to absence are revealing small cases.

During slices run touched-crate compile checks and selected `verify-model`, `verify-providers`,
`verify-analytics` or `verify-oracles` controls. New pure-domain tests prepare no database/Python
adapters unless they actually exercise one. Reuse existing independent fixture expectations;
do not copy old stage/receipt tests mechanically. Review changed `.snap.new` before acceptance;
graph/identity/wire changes are explicit contract migrations.

Completion supplies store-free admitted frontiers for both profiles, upper catalog content and
selected analytical outcomes, not only a tiny facts demonstration. The publisher's full-content
roundtrip and actual serving are additional integration obligations at Q0. Real-library scale,
live vectors and activation remain the coordinator's explicitly authorized Q1 work.

Package state, actual current-tree verification and finding disposition remain at the coordinator.
The complete compiler stage is authorized; native persistence/publication/serving are subsequent work.
