# Phase 5: generation-bound serving — detailed design and execution plan

**Proposed execution target, 2026-10-01.** This plan reconstructs serving after the semantic model
cutover. It is subordinate to the [cutover plan](semantic-model-cutover-plan_2026-09-29.md), which
owns cross-phase sequence and finding disposition. [DESIGN §15.12](../design/sections/semantic-model.md#section-15-12)
and [§11](../design/sections/synthesis-and-serving.md#section-11) own accepted architecture;
[ADR-0114](../adr/0114-generation-serving-contracts.md) proposes the refinements below.
The [foundation enhancements plan](semantic-model-foundation-enhancements_2026-10-01.md) develops
the reusable classifier prerequisite and independently executable adapter improvements. This
document owns Phase 5 packages and assembled completion; it does not authorize production execution.

**Interface-checked, 2026-10-01:** source inspection against main at 16ffb9e4 plus the existing dirty
tree. Earlier Phase 4 qualification applies only to its recorded source and finite envelope.
The current Rust differences include apparent automatic formatting; they have not been requalified
here. Plan creation ran no product tests or pilots. New contracts named below are proposed,
including names used illustratively; no type or command is claimed implemented merely by appearing here.

## 1. Outcome, boundaries and document route

Restore all ten retained tools against one published canonical generation per server process:
search_operations, find_operations, get_operation, browse_library, get_evidence, search_evidence,
compare_operations, search_capabilities, get_capability and inspect_value_paths. Catalog access
is mandatory; brief/native enrichment advertises its actual generation-specific availability.
The capability resource and structured capability packet must consume the same evidence closure.

The result remains one semantic model and one PostgreSQL relational store. Serving views are not
imported or copied semantic tables. Request classification, finite native meaning, packet mappings
and ranking policy belong to lctx-model; PostgreSQL effects belong to lctx-postgres; Python supplies
FastMCP transport, presentation and library-backed numerical scoring. DataFusion remains compile
compute, not a second serving query backend.

No new semantic Serving frontier, lifecycle state, readiness registry, import pipeline, historical
ID bridge, compatibility decoder, universal projection language or new product tool is introduced.
Catalog is the minimum frontier for the complete core interface in this scope. Analysis-only and
lower generations remain unavailable to that interface rather than answering with empty catalogs.

PR6 comparisons, concept/explain tools, ANN, arbitrary SQL/predicate DSL, general runtime heap/alias
identity and R4's broader multi-variant Summary admission remain outside this plan. Existing research
obligations retain their current triggers. Fresh end-to-end library journeys are required; comparative
quality, total RSS and performance claims require separate evidence.

Read this combined target first, the foundation plan for F1–F3, then §8 for executable dependencies.
The small companion exists because it changes existing canonical components and has independent
completion; it is not another Phase 5 status or finding register.

## 2. Baseline and focused foundation assessment

| Existing owner / inspected source | Reusable guarantee | Consequence for this plan |
|---|---|---|
| Model records, generated codecs and shared invariants | Nominal references, append-only codebooks and one declaration per semantic question | Declare serving mappings/DTOs at the model boundary; use current records, not renamed legacy rows |
| [Canonical leases](../../crates/lctx-postgres/src/generations/lease.rs) and [generation store](../../crates/lctx-postgres/src/generations/mod.rs) | Published-state/frontier/model/physical admission and retirement protection | Expose a serving-role read admission route; a descriptor alone is not a live pin |
| [Generation lowering](../../crates/lctx-postgres/src/generations/ddl.rs) | Generated tables, grants, derivation views and policy views | Extend its phases and shadow inspection for serving views/indexes |
| [Finite selection](../../crates/lctx-model/src/domain/selection/evaluate.rs) | Shared outcomes, contextual witnesses and conjunction | Separate classification inputs from replay inputs; prepare and index once |
| [Conditions](../../crates/lctx-model/src/domain/conditions/mod.rs), Entry proofs and finite summaries | Occurrence-keyed atoms, canonical BDDs and qualified proof links | Reuse these owners; add the missing exact-request-scalar operation before migrating native inspection |
| [Retrieval](../../crates/lctx-model/src/domain/retrieval/mod.rs) and [embedding uses](../../crates/lctx-model/src/domain/retrieval/consumption.rs) | Addressable units, original anchors, exact spec/input/value receipts | Rank canonical units and derive pgvector representation from actual RetrievalEmbeddingUse rows |
| Retained repository, Python generation and native executor | Independent wire/journey expectations, worker and cancellation controls | Recovery inputs only; their cpg-schema IDs, bundles and readiness tables are not the replacement contracts |

The assessment identified three warranted foundation improvements, developed in the companion:
F1 separates and indexes classification data; F2 removes repeated expected-domain routing; F3
makes consumed-row loading follow inventories and vocabulary epochs. F1 supplies the working
classifier consumed by serving. F2/F3 can proceed in parallel with most serving work and do not
gate activation unless integration reveals a concrete dependency. Their benefit is reduced
independent interpretation and more local changes; speed improvements are hypotheses.

Two additional target seams are material. Retained selection reconstructs PreparedCatalog for
each request (lctx-postgres/src/selection.rs), and retained PinnedGeneration carries old manifest
descriptors without a canonical process lease (repository.rs). Native SemanticExecutor still uses
the legacy condition catalog and primitive theory. These are existing Phase 5 obligations, not
fresh claims that the reconstructed Phase 0–4 engines are defective.

### 2.1 Selected change scenarios

- Add a supported predicate: its model definition and classification inventory change; wire
  schemas and declared loading follow. Python and SQL do not add separate applicability rules.
- Add an optional packet field: change the owned DTO/mapping and needed hydration; preserve the
  mandatory signature and unrelated routes. Physical schema must follow declared source types.
- Change ranking/display policy: rebuild service preparation and reject old cursors without
  re-extracting unchanged canonical source facts.
- Replace the numerical lexical scorer: preserve admitted occurrence scores, policy, fusion and
  winning witnesses; no semantic classifier or generation lifecycle changes.
- Test exact-value path assessment: use explicit canonical rows/proofs and a resource budget;
  no library execution, PostgreSQL or MCP setup is needed for the pure operation.

These scenarios apply FP-01–FP-06 and DP-01/03/08/09/10/16/18/19/20/23. They are bounded reasoning
targets, not a promise to implement new predicates or alternate scorers in this scope.

## 3. Serving declarations, identity and preparation

### 3.1 Small model-owned mappings

Introduce cohesive serving and wire modules under lctx-model. Simple row views use the existing
typed relation/field metadata, nominal references and declared keys. A finite mapping inventory
names source dependencies, minimum frontier, required capabilities, output type and revision.
PostgreSQL lowers applicable mappings with SeaQuery/existing quoted SQL composition. This inventory
does not redefine defaults, signatures, association basis, selection outcomes or condition meaning.

Nested packets use ordinary typed DTOs and named transformations over canonical rows. Derive
Serde/Schemars schemas from those declarations, retaining distinct serialization/deserialization
schemas, closed objects, tags, missing/null distinctions, bounded text and nominal IDs. Do not force
packet nesting or formatting through a universal SQL/projection DSL. Do not derive all DTOs by
flattening arbitrary storage rows: wire operations have their own explicit behavior.

Migrate the useful dispatch pattern from cpg-schema::wire, replacing all nominal inner types with
current domain identities. One finite tool-to-request/result declaration supplies native dispatch,
schemas and Python registration. Structural decoding precedes effects; same-generation membership,
roles, capabilities and evidence closure remain contextual checks.

### 3.2 Identity boundaries

Keep these distinct:

| Identity | Meaning and dependency |
|---|---|
| Generation and model | Existing canonical publication and semantic declarations |
| Serving mapping / physical realization | Required views, grants, indexes, column/tag translations and canonical dependencies |
| Serving policy | Tokenizer/BM25 settings, fusion K60, tie/order rules, enabled channels and selected spec |
| Wire representation | Request/result encodings and packet rendering contract |
| Prepared consumer | Generation, consumed relation/epoch content, mappings, policy and relevant spec |
| Request / continuation | Canonical decoded request, generation, group/section/member, ordering, representation and channel state |

Canonical request hashing uses decoded finite values and an owned deterministic encoding, not raw
JSON key order or arbitrary Value.to_string(). Page size does not identify selection semantics;
a continuation must still pass current request and response budgets. Policy and representation
changes invalidate dependent preparation/cursors. Unit rendering/spec changes follow the stronger
canonical invalidation rules in §14.10; a display-policy change does not reinterpret stored facts.

Stored Condition IDs remain model-owned. A request-restricted diagram is a distinct request result
identified by generation, original condition, exact-input/model assumptions and derivation; it is
never serialized as though it were another stored Condition row. Rendered terms are a bounded
presentation carrying truncation independently from semantic uncertainty.

### 3.3 No separate readiness transition

Install applicable semantic views, grants and lookup indexes through the existing canonical
lowering before publication. Include them in physical identity and shadow/live inspection.
Their source tables remain canonical. Changing the lowering requires fresh current generations;
do not bolt a legacy serving schema onto an existing generation.

At startup derive a service descriptor from the published Catalog frontier/profile, actual scoped
availability, matching validation/content receipts, live structures, mapping/wire/policy identity,
required canonical artifacts and selected embedding spec. No startup write, acquisition, rebuild or
operator selection occurs. Missing required structures/content refuse; an unrequested optional
capability remains explicitly unavailable. Optional absence and corrupt advertised capability are
different outcomes.

Publication, operator selection and successful process admission stay distinct. A descriptor is
rebuildable and immutable within the process, not a new database readiness authority.

## 4. Runtime ownership and resource contracts

### 4.1 Generation guard and request leases

Expose canonical pin/admission through RoleConfig for lctx_serving without requiring OwnerPool,
migration credentials or a writable repository. Retain one lock-holding canonical generation
guard for the process lifespan. Explicit generation selection and default operator selection both
resolve once; later selection changes cannot repin that process.

Short-lived request connections separately admit the same generation and drain before classification,
embedding waits, fusion or native CPU work. The dedicated process guard connection remains held
intentionally. Reserve capacity for it separately from query connections.

Workers retain the process guard and memory/worker reservations until actual completion, including
after caller cancellation. Failed query connections are discarded after cancellation/drain checks;
there is no transparent request replay. A later request may acquire a fresh query lease while the
original process guard is healthy. Guard loss is terminal: poison admission, reject new work,
suppress successful delivery after detected loss and never reacquire that guard.

Use bounded guard liveness supervision and a final successful liveness confirmation before delivering
a CPU-derived result. Supervision must not contend with request queries on the guard connection.
Shutdown stops admission, drains workers and request cleanup, then acknowledges guard release.
No claim of instantaneous detection of a broken network is made.

### 4.2 One resource owner per service execution

Keep one service CPU admission owner in the Rust repository/runtime, with two default admitted jobs.
PyO3 async/native calls use that same owner; do not nest an independent two-job Python pool around
another unconstrained native execution path. Detach from Python for pure CPU execution, and return
only declared bridge values. Domain operations remain pure and budget-taking.

Use two default query connections plus the dedicated guard; validate role/database capacity at
startup. Retain a 30-second total request deadline and one-second worker-admission wait, drawn from
one serving configuration exported to Python. Statement and CPU deadlines consume that total rather
than independently restarting it. Cancellation cannot interrupt every native operation; the slot
and guard stay retained until it finishes.

Start with a 256 MiB shared service reservation pool, a 128 MiB preparation ceiling and a
64 MiB retained-input/hydration ceiling per admitted request. Charge indexes, decoded rows, conversions, lexical state
and native state together; two concurrent requests are not each promised the entire process pool.
Keep existing bounded kernel work/node limits. These are admission allowances, not total RSS bounds
or measured capacity. Preparation exhaustion refuses startup; request exhaustion is ResourceRefused,
not complete-empty. Changes to defaults need explicit configuration/identity and focused controls.

Packet limits remain 20 default / 100 maximum browse/evidence rows, 16 requirements, one to five
comparison candidates, and 32 KiB default / 256 KiB expanded final MCP response bytes. Required
signatures are indivisible. Optional omissions, continuation and unavailable sections are explicit.

### 4.3 Stable catalog, deferred bodies

F1 supplies a model-owned classification inventory narrower than C2 production/replay data.
The production repository loads it once from the leased generation, checks content against matching
stored relation/epoch receipts and requires the C2 declaration-closure validation receipt. Its private
admitted owner holds owned data/output/indexes; methods borrow them without self-referential objects.

This is not a caller-provided "validated" boolean. A lease checks state and shape, not all live row
contents; classification loading must detect changed/missing consumed rows against receipts.
Strict broad C2 replay remains available for qualification and explicit integrity diagnosis.
Arbitrary rows supplied outside the store go through that strict route.

Prepare the complete supported-generation metadata domain initially. Original source bodies and full
proof expansions remain on-demand. Do not silently prune generation-wide membership, missing lookups
or completeness by request predicate. If representative preparation exceeds the declared envelope,
stop that package and settle a complete predicate/domain-scoped preparation design with its consumer
and acceptance before proceeding; raising a budget alone is not evidence of an adequate design.

## 5. Query, packet and evidence behavior

### 5.1 Shared selection, different extent

find_operations uses the complete admitted domain and model classifier, returning supported,
unresolved and conflicting groups in Discovery, or supported-only Strict results. Contradicted
requirements exclude ordinary candidates; compare_operations may display them in caller order.
Preserve the model's context-compatible conjunction, witness basis and coverage. Complete signature
absence does not become runtime keyword rejection. No best verdict is selected across incompatible
overloads, bindings, configurations or instances.

search_operations nominates/ranks eligible candidates and labels its ranked extent. Every leg filters
eligibility before its final limit. Safe SQL filters are allowed only with a stated equivalence to
the complete requested semantic domain. A lexical/vector top-k cannot become exhaustive eligibility,
a complete total, or absence evidence. EmptyUnderCoverage and unknown ownership remain explicit.

Cursors bind generation, normalized request, policy, representation, group/section and actual channel
state. A change in embedding availability or query-vector identity between pages rejects the cursor;
this first implementation does not add retained request-result sessions. Deterministic unchanged
channels may be recomputed. Verify channel identity before returning continued results.

### 5.2 Catalog and implementation packets

get_operation resolves public identity without choosing a convenient ambiguous exposure or instance.
Mandatory core contains identity/release, import/access provenance, invocation, complete source/effective
signature variants, defaults/options and limits. Optional independently paged sections add at most two
positive original scenarios, relevant deployment prerequisites, at most five relationship links,
local conflicts, grounded briefs and behavior.

browse_library uses deterministic canonical module/class/member ownership, scoped vocabulary and
counts. Unknown ownership does not imply exclusion or guessed hierarchy. compare_operations shares
the classifier and keeps one to five requested members and their ambiguity/contradiction.

Hydrate sets of requested nominal IDs rather than one query per field or witness. Use fixed checked
SQLx queries for stable reads and declaration-derived mapping for dynamic relation shapes. Query
parameters are data; relation/field names come only from admitted declarations. Required missing
rows refuse. Detect closure membership and same-generation roles before returning nested output.

### 5.3 Original evidence and bounded explanations

get_evidence reconstructs exact requested bytes through SourceArtifact/ArtifactChunk closure, preserving
original byte ranges, digest, encoding, release/context and evidence status. It does not serve an E0
fragment as though it were the original body. search_evidence independently ranks addressable units,
including material with no guessed member association.

Use the generated derivations and derivation_premises views for bounded recursive lookup. Preserve
source relation, rule, conclusion, premise relation and role. Default explanation expansion is
depth 8, 256 nodes, 1,024 premise edges, bounded by the request byte budget; reaching a limit is
explicit continuation/truncation. Foreign or missing required support is corruption. Follow the
relation-qualified DAG, never enumerate all proof paths or substitute untyped ID lookups.

The explanation lookup is an internal operation supporting retained packets/native tools; it does
not activate a new general explain route.

## 6. Retrieval policy and physical vector realization

### 6.1 Reuse numerical libraries; own ranking once

Retain bm25s 0.3.11 with the pinned numpy backend for numerical BM25 scoring. Rust owns the typed
policy, exports numerical settings, validates returned occurrence IDs/scores and performs best-fragment
selection, per-family/channel aggregation, contiguous ranks, equal-weight family-normalized RRF K60,
exact-path promotion, deterministic ties and witness construction. Python does not independently fuse
results or decide eligibility. Preserve the existing hand-expected arithmetic cases as independent
controls, rather than retaining two production fusion implementations.

Deduplicate family/text for scoring while retaining all contextual occurrences. One best contribution
per member/family/channel prevents duplicate examples from adding votes. Evidence search uses unit
identity rather than forcing all evidence into a member. Winning witnesses retain unit/fragment,
context, family/channel/rank and generation/policy. They are not requirement witnesses.

A scorer receives only admitted text/occurrence inputs and declared settings; validate finite scores
and exact membership before fusion. Charge library arrays/indexes and input/output conversions.
Prepare lexical indexes once. Replacing BM25 numerical implementation need not change the semantic
classifier or fusion owner.

### 6.2 Actual canonical vectors and the bytea gap

Canonical vector values are fields on AnalysisEmbeddingUse and RetrievalEmbeddingUse, not a separate
VectorValue relation. Codec 1 is exact little-endian f32 bytes; spec, input and value_digest identify
the consumed winner. Phase 5 retrieval consumes RetrievalEmbeddingUse and preserves both consumers'
existing winner-consistency invariant. It does not introduce another semantic vector owner.

The pinned SQLx/pgvector boundary binds decoded f32 arrays through pgvector::Vector, but the inspected
contracts offer no built-in canonical bytea-to-vector cast. Do not claim an unimplemented
vector_from_bytes view function or author an independent SQL float decoder.

**Proposed ADR-0114 choice:** a narrow disposable PostgreSQL vector artifact, containing only the
generation-qualified retrieval-use key and numerical vector representation, plus its artifact
manifest. All member/context/spec/text/support semantics remain canonical views. The artifact is
derived through domain::embedding::value decode/validation and is never manually authoritative.

Provide an explicit effectful CLI preparation operation, proposed as lctx serving prepare --generation ID.
It pins the published generation, derives and validates canonical input closure, checks the pinned
extension and writes bounded vectors/manifest transactionally under an artifact-key lock. It does
not select a generation or call an embedding service. The operator runs it before starting a
vector-enabled service; read-only startup never creates or repairs it.

Artifact identity covers canonical generation/model, source retrieval-use content, selected spec,
codec/conversion definition and physical vector contract. A repeated key must match canonical
derivation; wrong-but-valid disposable content is recomputed by explicit preparation. Corrupt
canonical required input refuses instead of being repaired from an old cache.

Use a cache-owned namespace, not copied semantic tables in lctx_serving or foreign keys into generation
schemas. Artifact publication is atomic and generation retirement/abort cleanup removes matching
artifacts through the store's existing cleanup owner. Do not introduce another canonical readiness
state or permit the cache to retain a generation after the last legitimate lease ends.

Start with the currently selected 1,024-dimensional exact profile. Canonical codecs keep their wider
domain; serving rejects an unsupported vector profile before scanning or explicitly selects a
lexical-only policy. No claim about a broader extension dimension ceiling is required. Re-check
dimensions, finite components, norm, digest and signed-zero representation through the shared codec;
do not independently renormalize.

Startup validates the artifact against canonical inputs and manifests, including numeric readback.
If vector capability is advertised, missing/corrupt required artifact refuses. An explicitly selected
lexical-only policy needs no vector artifact. Query-service unavailability may disclose lexical-only
results under an admitted degradation policy; it must alter channel/cursor identity.

Exact cosine scoring covers every eligible physical vector with deterministic ties; ANN candidate
limits are not used. Lexical and vector budgets refuse explicitly rather than silently pretending
to search the full domain. Missing optional vector uses stay addressable through original evidence.

### 6.3 Library choices and evidence boundary

| Capability | Selected existing mechanism | Why / qualification needed |
|---|---|---|
| SQL effects and checked stable reads | SQLx 0.9.0 | Existing owner; regenerate only migrated query metadata, preserve cancellation/drain |
| Generated physical SQL | SeaQuery 1.0.2 plus existing audited generation lowering | Derive types/references; no handwritten second schema |
| Typed rows / Arrow | Existing Domain derive, Arrow 59.3.0, serde_arrow 0.15.1 | Keep explicit matching schemas and nominal identities |
| Vector effects | pgvector Rust 0.4.2, extension 0.8.6 | Exact 1,024 profile and shared-codec artifact; conversion/readback controls required |
| Wire schemas | Serde 1.0.229 / Schemars 1.2.2 | Existing concrete DTO pattern; actual list/invoke schema parity still required |
| Python/native bridge | PyO3 0.29.2 / async runtimes 0.29.0 | Coarse calls, detach pure CPU; retain lifetimes through cancellation |
| MCP transport | FastMCP 4.0.5 | Existing schema-backed Tool/ToolResult adapter; final byte checking remains ours |
| Conditions | biodivine-lib-bdd 0.6.3 through the current domain kernel | No second DNF/legacy condition interpreter |

Pinned local capability references and current source informed fit, not fresh runtime conformance.
The [PostgreSQL skill](../../.claude/skills/sqlx-postgres/SKILL.md) distinguishes crate and extension
pins; the [FastMCP skill](../../.claude/skills/fastmcp/SKILL.md) distinguishes listed/invoked/result
representations and warns that lossy response middleware is not a hard serialized-byte bound.
Primary library sources: [pgvector](https://github.com/pgvector/pgvector),
[Schemars](https://docs.rs/schemars/1.2.2/schemars/), and [pinned crate/source declarations](../../Cargo.toml).
No dependency pin change or exhaustive catalog refresh is scheduled.

## 7. Native inspection and FastMCP transport

### 7.1 Restore finite exact-input behavior through the current model

The old native exact-input operation is narrower than executing Python but stronger than applying
an arbitrary BDD restriction. Define a pure model-owned operation consuming resolved member/formal
context, admitted finite path/proof, Entry/rebase provenance, exact request scalar, checked predicates,
builtin-model assumptions and a resource/work budget. It must establish why the requested input
governs an evaluation atom before assigning it.

Support the retained finite builtin-literal envelope: None, booleans, non-bool integers and strings
where checked equality/membership predicates and operand links justify an assignment. Preserve
bool/int distinctions and admitted CPython equality semantics from the retained independent controls.
Other primitive operations/types remain explicit unsupported/unknown unless a retained expectation
requires a separately justified implementation. Request values do not become source observations.

Reuse current Condition/Diagram, source-binding Entry proof, guard rebase, path identity, obligations
and finite proof owners. Source/stub, unresolved/decorated targets, changed entry bindings, default
stability, computed operands and unsupported builtin namespaces cannot be guessed into exact links.
A call to shadowed builtins remains unknown. No allocation/alias/mutation/temporal proof is inferred
from the accepted standard-record association.

Return the five behavioral verdicts with basis, assumptions, original condition, proof/support,
work and explicit unexamined/truncated state. Exact-input false restriction may refute that admitted
path under its model. A nonfalse restricted diagram is compatibility under a may-model, not feasible
execution or operation-wide support. Enumerating every inspected path still does not prove absence
of every possible Python execution. Finite negative S0 proof semantics remain distinct.

Re-home legacy exact-value/condition/value-path expectations into pure domain and actual generation
native tests before retiring primitive_theory/SemanticExecutor. Where a retained case needs a new
link rather than a new algorithm, add that explicit typed projection/invariant in this package;
do not restore a legacy ID map. Missing required links are a concrete package prerequisite.

### 7.2 Thin bridge and transport

Migrate lctx_semantics to current owned prepared native inputs and model query operations; migrate
lctx_storage to the read-only admitted repository/guard. Preserve importable package names and
coarse bridge operations where useful, without promising compatibility for old wire encodings.

Register all ten routes from the finite Rust tool declaration. Python Contract/Packet remains a
presentation view of native-validated JSON. FastMCP lifecycle owns start/shutdown; ToolResult returns
concise text plus complete structured content. Validate both input and final output with native
contracts, then check final serialized MCP bytes including text/content duplication. Do not use
lossy response-limiting middleware to discard required structured output.

Read-only/idempotent annotations, protocol-only stdout, masked internal exceptions, explicit domain
ToolError/resource errors and total deadlines remain. Disable response caching unless an explicit
generation/policy/channel-qualified cache contract is later justified; a cache cannot hide degraded
availability. Markdown renders existing status; it does not classify requirements or reinterpret
verdicts. No source/library execution occurs on the query path.

## 8. Dependency-ordered execution packages

Broader serving packages remain **Proposed / not_run**. The companion foundation scope implements
F1–F3 separately; its current acceptance boundary is recorded there. Contract agreement enables
dependent drafting; only implemented and focused-verified contracts enable production integration.
No whole-document barrier is implied.

| Package / owner | Required input | Delivered capability and local acceptance |
|---|---|---|
| D0 — root / model and store owners | This plan, ADR-0114 disposition | Settle mapping/policy/identity/resource contracts; update architecture and accept or revise the proposal before production implementation |
| F1 — model classifier owner | Companion §5 settled foundation admission | Companion F1 narrow indexed classifier, strict replay and minimal canonical consumer; broader C0 reuses these foundations |
| M0 — model/store declaration owner | D0 | Finite serving mappings, DTOs/tool inventory, generated views/grants/indexes and physical inspection; field-addition and wrong-null/tag/type controls |
| L0 — store/runtime owner | M0 admission contract | Serving-role process guard, bounded same-generation query readers and shutdown/cancellation/loss state; real PG18 lease/retirement tests |
| N0 — model native owner | D0 and current P4 finite owners | Pure exact-input assessment and native inventory contract; independent literal/unsupported/context/budget controls |
| C0 — repository/catalog owner | F1, M0, L0 | Generation-wide admitted metadata preparation; find/resolve/browse/compare with complete domains and deterministic continuation |
| E0 — repository/evidence owner | M0, L0 | Set-based mandatory/optional packets, original-byte evidence, bounded derivation lookup; closure and indivisible-signature controls |
| V0 — store/vector owner | M0, L0, shared value codec | Explicit vector artifact preparation/admission/cleanup, exact scoring, selected profile and read-only startup controls |
| R0 — model ranking + numerical adapter owner | C0 eligibility, E0 witnesses, V0 vector contract | Numerical BM25 seam and one Rust fusion owner, exact-name promotion, channel-bound cursors; duplicate neutrality and filter-before-limit controls |
| N1 — native bridge owner | N0, M0, L0, E0 | Prepared native query over actual generation records; five verdicts, original conditions and same-generation proofs |
| T0 — Python transport owner | M0 schemas, L0 lifecycle; integrate C0/E0/R0/N1 as ready | Actual ten-tool/list/resource dispatch, final byte admission and shared cancellation/runtime ownership |
| X0 — root / retiring consumers | Replacements and independent controls integrated | Remove last old serving authorities and named dormant runtime sources; verify active dependency/reference inventory |
| Q0 — root / qualification owner | All Phase 5 functional packages and X0 | Both-profile real-library/native/MCP journeys, complete functional gate, hygiene, assembled scoped target review and handoff |
| F2/F3 — canonical adapter owner | Companion foundation contracts | Independent improvements; integrate before Q0 only if part of the executed scope, otherwise keep their completion separate |

A useful sequence is D0, then M0/F1/N0 in parallel. L0 follows M0's working admission contract.
C0/E0/V0/N1 branches become ready independently; T0 can register schemas and build the lifecycle seam
before every route is complete. R0 needs working eligibility and numerical inputs, not completed
Markdown rendering. X0 retirement accompanies consumer migration rather than postponing all deletion
until the end.

One writer owns shared model declarations, generation lowering, manifests and query metadata.
F2/F3 overlap current producer files and should share an adapter owner or serialize those edits.
F1 mainly owns selection modules/tests; native N0 owns new pure query contracts; neither grants
permission to rewrite unrelated source analysis. Separate worktrees only for genuinely concurrent
production editing. The root remains design/integration/acceptance owner.

## 9. Preservation, activation and qualification

### 9.1 Retirement routes

| Retained input | Replacement and deletion obligation |
|---|---|
| cpg-core::bundle, bundle format/verification and dormant bundle tests | Generated mappings/canonical generation admission; move independent expectations, then remove bundle build/import/file fallback |
| cpg-schema wire/catalog/selection/retrieval/serving_projection/native condition and theory modules | Current model DTOs/classifier/policy/query operations; migrate each live native/store/Python consumer before deleting its last authority |
| Old PostgreSQL import/projection/ready/selection tables and dormant fixed queries | Existing canonical lifecycle plus generated views and narrow artifact cache; no legacy readiness registry |
| Python generation/native IPC inventories | Declared current canonical/native input mappings and checked prepared owner; no old-file loader |
| Python eligibility/fusion/cursor interpretation | Model classifier/ranking/cursor owner, with numerical scoring and presentation retained |
| Cursor, packet, evidence, native, serving/pr4 and MCP/oracle controls | Re-home independent expected semantics under new identities; delete obsolete setup/snapshots only after mapping |

X0 starts with a scoped source/test/reference inventory, including remaining Cargo dependencies and
the 21 frozen dormant SQLx query entries. Retire only entries whose consumers migrated. Regenerate
migrated fixed-query metadata under the end-of-turn policy; restore sqlx-check to the appropriate
integrated gate only after the replacement queries exist. A schema snapshot change is a migration:
read .snap.new first, then accept explicitly; never renumber codebooks.

No parity-to-bug requirement applies. Preserve expected correct behavior and document intentional
changes in identity, encoding, availability or semantics. Current-only operator activation quiesces
readers, prepares fresh Catalog generations and required artifacts, validates, explicitly selects
and restarts serving. Remove obsolete runtime/cache assets only after a scoped inventory; preserve
benchmark/evaluation evidence and unrelated dirty work. Do not retain rollback-only generations
or unconsumed old schemas. Publication/preparation never selects automatically.

### 9.2 Revealing acceptance scenarios

| Risk | Evidence required |
|---|---|
| Second schema/meaning authority | Add a mapped field through its owner; derived SQL/schema/native/Python forms agree without independent semantic edits |
| Incorrect generation readiness | Real canonical-only startup succeeds; lower frontier, wrong schema/digest, missing required rows/artifact, forged view/grant and changed content refuse |
| Cross-generation support | Foreign member/condition/witness/original span refuses through actual store, native and MCP paths |
| Scope or absence loss | Partial/NotRequested/unknown ownership, unresolved wrappers and incompatible overload/configuration conjunction survive listing and invocation |
| Preparation hides a stale result | Narrow admitted classifier equals strict C2 replay on hand-expected cases; wrong-but-valid rows, missing membership/coverage and shuffled input challenged |
| Ranking changes eligibility or votes | Noneligible high scorer excluded before limits; duplicate evidence does not inflate family rank; exact-path/tie arithmetic and winning original units preserved |
| Physical cache becomes authority | Cold/warm/rebuilt artifact agrees with canonical exact bytes; valid-width wrong vector/digest, missing row, changed spec and zero/nonfinite vectors rejected |
| Continuations silently change meaning | Generation/request/group/section/policy/representation/channel/query-vector mismatches reject; unchanged channels reproduce pages |
| Native overclaim | Exact scalar path-local refutation, may-compatible result, no bridge, shadowed builtin, bool/int distinction, foreign context, unsupported link and work exhaustion |
| Lifecycle/resource leak | Selection change leaves old server pinned; retire cannot remove leased generation; guard loss terminal; SQL/native cancellation retains capacity until drain/completion |
| Response truncation or transport drift | Actual FastMCP list/call/resource schemas and structured output; mandatory signature over budget refuses; optional sections continue; final MCP bytes checked |

During implementation run pinned compile checks and targeted release-profile tests/probes only.
Use real disposable PostgreSQL18 for store controls and pure model tests for finite operations.
At complete functional scope run just test-all and just hygiene for the same assembled tree, fixing
failures with named reruns. End-of-turn automation owns formatting and generated-file refreshes.
Plan/doc-only work does not trigger these product gates.

Q0 additionally compiles the pinned FastMCP library through Catalog for both catalog and behavioral
profiles, prepares the required physical artifact explicitly, and runs actual native/MCP journeys
against each fresh generation. It covers core catalog access without brief seeds, optional native
availability, original evidence and exact-profile ranking; a fake embedding effect qualifies only
its seam. Live query embedding is separately recorded passed/failed/blocked/not_run; an admitted
lexical-only journey does not qualify live hybrid behavior.

The [serving-scoped target review](../design_review/reviews/design_review_phase5-target_2026-10-01.md)
is **Accept scoped, 2026-10-01**, at Proposed target-design maturity. Obtain an assembled review
at Q0 under the binding. Neither is a whole-system retrospective Phase0–4 audit. Product comparisons,
retrieval quality, latency/hydration/peak RSS, ANN and broad semantic measurements are independent,
explicitly not_run until activated. They are not invented completion gates for this scope.

## 10. Finding routes, limits and current state

The parent §8 remains the sole current owner for F05 served condition identity, F10 competing serving
authority, F12 repeated preparation/round trips, F13 served policy identity and the C13 lookup/grant
remainder. This plan supplies D0/N0/M0, C0/E0 and R0/T0/X0 closure routes respectively; writing or
accepting a plan closes none of them. Product findings remain at forward-plan §6.

F1–F3 are scoped enhancement package identifiers, not new source-review finding IDs. Any material
formal review findings keep their review-prefixed IDs and get one linked disposition owner.

**Current state, 2026-10-02:** companion F1–F3 are implemented with composite focused
acceptance, including both-profile real Catalog admission for the minimal canonical consumer.
Their separate scope-end foundation qualification is in progress. Broader serving packages remain
Proposed / not_run. The scoped target review accepts the proposed direction; ADR-0114 still needs
acceptance or revision through D0 before broader serving implementation. Begin D0 contract
integration, then ready M0/N0 work; C0 reuses qualified F1 preparation. Parallel foundation work need not delay
Phase 5. Record future package progress here and in the companion only for its own packages.
