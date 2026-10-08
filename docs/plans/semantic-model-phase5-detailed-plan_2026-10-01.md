# Phase 5: generation-bound serving — detailed design and execution plan

Command excerpts identify verification scope; original invocations remain in the cited logs and Git history.

**Lexical-only qualification stopped at the operator’s code-facts review pivot, 2026-10-03.** Full hybrid Q0 is not claimed; live embeddings and real-library vector preparation are deliberately deferred pending inspection of analytical outputs. This plan reconstructs serving after the semantic model
cutover. It is subordinate to the [cutover plan](semantic-model-cutover-plan_2026-09-29.md), which
owns cross-phase sequence and finding disposition. [DESIGN §15.12](../design/sections/semantic-model.md#section-15-12)
and [§11](../design/sections/synthesis-and-serving.md#section-11) own accepted architecture;
[ADR-0114](../adr/0114-generation-serving-contracts.md) accepts the refinements below.
The [foundation enhancements plan](semantic-model-foundation-enhancements_2026-10-01.md) develops
the reusable classifier prerequisite and independently executable adapter improvements. This
document owns Phase 5 packages and assembled completion. The operator authorized full execution
on 2026-10-02 using bounded parallel work after foundation acceptance.

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
| [Canonical leases](https://github.com/paul-heyse/library-context/blob/16ffb9e4ab3e8c5a5e7bec97668f7dfcfb307abe/crates/lctx-postgres/src/generations/lease.rs) and [generation store](https://github.com/paul-heyse/library-context/blob/16ffb9e4ab3e8c5a5e7bec97668f7dfcfb307abe/crates/lctx-postgres/src/generations/mod.rs) | Published-state/frontier/model/physical admission and retirement protection | Expose a serving-role read admission route; a descriptor alone is not a live pin |
| [Generation lowering](https://github.com/paul-heyse/library-context/blob/16ffb9e4ab3e8c5a5e7bec97668f7dfcfb307abe/crates/lctx-postgres/src/generations/ddl.rs) | Generated tables, grants, derivation views and policy views | Extend its phases and shadow inspection for serving views/indexes |
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

**Accepted ADR-0114 choice:** a narrow disposable PostgreSQL vector artifact, containing only the
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
The PostgreSQL skill (`sqlx-postgres`, no longer selected) distinguishes crate and extension
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

Broader serving packages are **authorized / not_run** until their implementation receipts below. The companion foundation scope implements
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
| Q0 — root / qualification owner | All Phase 5 functional packages and X0 | Both-profile real-library/native/MCP journeys, assembled `just qualify`, full keep-going Clippy, applicable leaves, assembled scoped target review and handoff |
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
At complete functional scope run affected controls and applicable leaves; this shared-contract
scope requires assembled `just qualify` on one tree (ADR-0126), with real disposable PG/native/MCP,
compile-fail/doc contracts and full keep-going Clippy. Repair failures and rerun affected boundaries
or named leaves. End-of-turn automation owns formatting and generated-file refreshes. Plan/doc-only
work does not trigger product qualification. Dated receipts below retain their original scope;
superseded broad aliases are not current instructions.

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

**Target-alignment dependency update, 2026-10-04 — Implemented, qualification pending.** The
[serving-alignment plan](serving-provenance-presentation-plan_2026-10-04.md) owns S1 requested-library
admission, S2 generation-bearing resource rendering, S3 value presentation and IS2 startup
preparation controls. [Target-alignment Q0](target-implementation-alignment-plan_2026-10-04.md#5-migration-and-verification)
coordinates current fixture/schema/artifact/full-gate acceptance; its coordinator owns the new
review findings. Existing receipts here retain their dated scope. These dependencies neither
restart the stopped library reconstruction nor authorize selection, registration, live vectors
or PR6. Qualification/activation here remains separate and stopped until operator authorization.


The parent §8 remains the sole current owner for F05 served condition identity, F10 competing serving
authority, F12 repeated preparation/round trips, F13 served policy identity and the C13 lookup/grant
remainder. This plan supplies D0/N0/M0, C0/E0 and R0/T0/X0 closure routes respectively; writing or
accepting a plan closes none of them. Product findings remain at forward-plan §6.

F1–F3 are scoped enhancement package identifiers, not new source-review finding IDs. Any material
formal review findings keep their review-prefixed IDs and get one linked disposition owner.

**Current state, 2026-10-02:** foundation F1–F3 are implemented with completed functional
qualification and composite hygiene; the companion §5.4 owns exact receipts and the post-hook
source boundary. D0 accepts ADR-0114 and the resource/identity contracts in §§3–4. Broader serving
is authorized but remains unavailable. Production packages integrate only after working contracts;
C0 reuses the existing admitted F1 classifier and shares one process guard across consumers.

Execution baseline: main `7ec3e3c6` plus 118 preserved dirty tracked paths, frozen as temporary
snapshot `bd22f66c71023cf7ca642eea8e82689941e5ac44`. Preservation copies and binary diff are at
`/tmp/lctx-phase5-baseline-2026-10-02/`. This snapshot isolates concurrent production work;
it is not a broad commit of unrelated changes on main. One root owns integration, declarations,
lowering, manifests, query metadata and final acceptance. No formatting/generator exception is assumed.

| Package | Current implementation / acceptance |
|---|---|
| D0 | Accepted target, 2026-10-02; ADR-0114 and architecture updated. |
| M0 | Implemented / scoped Tested. Current closed wire controls passed 16; all five actual packet controls passed, including direct mixed-status canonical comparison. The assembled gate covers later ordering integration. |
| L0 | Implemented / scoped Tested. Original guard, view/index/ACL controls, cancellation/drain and cumulative 30-second deadline passed within their recorded actual compiler scope. Current both-profile native/transport fixture also passed after the ordering repair. |
| C0/E0 | Implemented / scoped Tested. Actual catalog/packet controls, eligibility and ranked-winner original bytes passed. Corrected actual BaseEvaluation receipt-bound DAG, Unicode paging and damaged-view/captured-byte control passed 1 / zero skipped, 92.778s. Full gate covers the latest shared read snapshot. |
| R0 | Implemented / scoped Tested. Pure ranking controls passed 16, including shared complete-family corpus projection and retained tokenizer known answers. Actual retrieval fixture passed; Python numerical/transport controls passed within their scopes. No retrieval-quality claim. |
| V0 | Implemented / scoped Tested. Full original corrected actual fixture passed 1 / zero skipped, 120.119s, run `ee4c8c95-37b3-45c2-9d3e-3fb002ef230f`; controlled numerical HTTP seam only. Composite receipt retains physical inventory and isolated dependency-sync failures. Later integration is covered by the assembled gate. |
| N0/N1 | Implemented / scoped Tested. Pure native/condition/selection/evidence controls passed 50. Current ordering-discriminator actual fixture passed 1 / zero skipped, 200.266s; both profiles passed all ten transport controls. The unchanged generated CPython original-path oracle passed 1 / zero skipped, 795.987s after the production ordering repair. |
| T0/X0 | Implemented / scoped Tested. Current ten-tool/resource bridges and exact final codecs passed in actual transport. 375 inventoried obsolete source files, 21 metadata entries and three obsolete wire assets retired. Independent expectations are re-homed; real retained-service and matcher/preregistration controls passed as scoped composite receipts. Generated-program controls passed 2; actual native oracle now passed. |
| Q0 | Lexical-only real-library qualification stopped at the operator’s code-facts review pivot, 2026-10-03. Current execution receipt below owns the resumed scope. The prior stop remains a dated incomplete receipt. Alignment subsequently passed a complete functional gate and composite hygiene; it does not establish real-library qualification. Live embeddings, real-library vector artifacts and hybrid retrieval are deliberately deferred pending analytical-output review. |

**Build coordination, 2026-10-02:** simultaneous release builds in isolated production worktrees
formed a verified fine-grain Cargo artifact lock cycle: vector build held shared `allocative` and
workspace-feature locks while waiting for the model lock; native build held the model lock while
waiting for those dependencies. Scoped interrupted commands establish no test result. Rust builds
are now serialized while independent implementation continues; shared paths, optimization policy,
frontend threads and caches remain unchanged. Neither test parallelism nor the earlier operator
cleanup is established as the cause of this lock cycle.

**Retirement control routes (X0), 2026-10-02, pending migration acceptance:** legacy bundle/IPC
encodings, format numbers and frozen projection parity have no target consumer. Their independent
expectations have moved to current owners before deleting the obsolete tests and producers:

| Independent expectation | Current control owner / remaining qualification |
|---|---|
| Schema closure, structural rejection, final UTF-8 MCP bytes, model/wire identities | `lctx-model/tests/serving_contracts.rs`; latest 14-control receipt pending; actual serializers at T0 |
| Finite literal restrictions, five verdicts, work refusal, exact original condition/proof, formal and context rejection | `lctx-model/tests/native_requests.rs` (16 passed), actual `lctx/tests/serving_native.rs` (both-profile rerun pending); retained SummaryRun replay owns stored discharge/finalizer proofs |
| Explicit selection, reader retirement protection, atomic generation publication and corrupt/missing canonical rows | Existing real generation-store controls plus L0 cancellation/loss and `serving_admission.rs`; later integrated gate remains pending |
| Spec/domain isolation, exact vector values, negative/zero cosine, tampered physical cache and cleanup | `lctx/tests/serving_vectors.rs`; repaired full original fixture rerun pending |
| Eligibility before ranking, complete family DF, contiguous ranks, exact-path promotion, channel/cursor identity | `lctx-model/tests/ranking_policy.rs`, actual `serving_retrieval.rs`, Python numerical arithmetic; latest integrated controls pending |
| Canonical catalog specificity, whole required signatures, source ranges and grounded optional packets | `serving_catalog.rs`, `serving_packets.rs`, `serving_evidence.rs`; original capture/proof controls pending |

These routes retain qualification obligations after source retirement; deletion does not
establish that their replacement expectations passed. Legacy cancellation/import-resume controls for the removed Arrow-bundle importer
are replaced by the generation store's transaction/lifecycle controls; no compatibility importer
is retained. Q0 must include the new serving binaries and remove the blanket Phase 5 skips.

**Integration check, 2026-10-02:** normalized `cargo check --release -p lctx --test
serving_evidence --test serving_retrieval --test serving_packets -p lctx-semantics` is composite
passed (14.52 seconds on corrected source). Initial errors were the SQLx Connection trait import,
a non-serializable intermediate ScenarioAssociation page, the Catalog distribution accessor,
and the migrated embedding schema helper's missing model dependency. Actual CLI test compilation
subsequently exposed an AttemptId import through a dev-only dependency; it now uses the existing
`cpg_core::postgres` owner re-export and the actual test command is rerunning. Later receipt-bound
canonical proof-stream and bridge argument-allocation changes still require their scoped compile.


**Focused integration updates, 2026-10-02:** M0 closed wire contracts passed 15/15. Catalog and
packet actual fixtures form a composite receipt: catalog 1 passed and packet 4 passed / 1 failed;
that optional-section failure was corrected by preserving the producer's exact deployment field
names, and its original test passed on the named rerun (65.84 seconds). Terminal body pages omit
absent continuations and reject explicit null. Set-based behavior hydration is implemented after
that receipt; its compile and affected behavioral rerun remain not_run. The selector's four
metadata field spellings now match the actual hyphenated producer output; focused selector
controls and subsequent fresh generations remain pending.

X0's independent inspection identified four expectations to preserve before old test deletion:
a live native MCP structured roundtrip, a cumulative deadline across individually short phases,
distinct same-observation sibling origins, and a ranked evidence winner resolving to original
bytes. Current controls have been added to `current_transport.py`, `serving_native.rs`,
`compile_facts.rs` and `serving_retrieval.rs`; their actual qualification is pending. Old bundle
encoding and IPC parity have no current consumer and do not require reproducing their format.


**X0 source retirement, implemented / not yet qualified, 2026-10-02:** scoped unchanged-input
inventory `/tmp/lctx-phase5-x0-source-inventory_2026-10-02.json` records 375 file hashes. Current
replacements own the independent expectations before deletion of `cpg-schema`, its old
model declaration/identity machinery, dormant bundle tests/runtime, legacy PostgreSQL
import/projection/readiness/query consumers, old native IPC and Python semantic owners. The 21
fixed-query metadata entries were matched to the retired SQL text; no current macro consumer
exists, so an empty metadata generator is not an acceptance gate. New serving binaries are
included in `test-postgres`, and the blanket Python serving skip is removed.

Retained-service backup now uses an explicit format-4 scope over the five original cache/operation
service tables. Canonical generation state and disposable vector artifacts are rebuilt from
pinned inputs, with no old ready-state, manifest, locator, bundle recovery or compatibility reader.
The old bundle-dependent evaluation/workload runners are retired. Preregistered question/request
sets, heldout inputs, gold matcher/source-span arithmetic and captured evaluation evidence remain;
current-model product comparison runners are activated with that independent scope, not Q0.
The independent CPython served-claim oracle is being migrated separately and remains pending.


**Retained-service/evaluation controls, composite passed, 2026-10-02:** normalized
`uv run --no-sync pytest tests/scripts/test_postgres_serving.py -q` passed split credentials and
existing-database refusal, but backup initially failed its closed inventory because the disposable
vector tables were included. The first repair excluded them; the restore then exposed pg_dump's
`--table` filter overriding the mixed schema selection. The exact named backup test passed after
using the five fixed table arguments and explicitly creating their two namespaces in the
 disposable restore. Logs retain both failures and the final pass:
`/tmp/lctx-phase5-retained-services_2026-10-02.log`,
`/tmp/lctx-phase5-retained-services_corrected_2026-10-02.log`,
`/tmp/lctx-phase5-retained-services_table-selection_2026-10-02.log`.
`uv run --no-sync pytest tests/scripts/test_gold_match.py tests/scripts/test_structured_eval_guard.py -q`
passed 9 retained preregistration/matcher/source-span controls; no live product quality claim.


**Assembled review, 2026-10-02:** the independent
[Design/Target review](../design_review/reviews/design_review_phase5-assembled_2026-10-02.md)
was **Revise** at its initial inspected boundary; Appendices A/B accept its corrections with scoped actual transport evidence. Final Appendix C is **Accept scoped** for the assembled design at Implemented maturity, with Q0 and activation pending. Its findings have this one disposition owner:

| Stable source finding | Owner / current disposition | Required closure evidence |
|---|---|---|
| phase5-assembled F01 | T0/runtime codec integration; Closed / scoped Tested, 2026-10-02 | Borrowed JSON admission and pinned final SDK serialization under the original CPU/deadline/reservation, oversized/Unicode/cancellation and exact writer controls; bounded reviewer follow-up |
| phase5-assembled F02 | M0/E0/T0 capability representation; Closed / scoped Tested, 2026-10-02; Appendix B | Original per-assertion status/kind/qualification/text/support in structured packets and resource presentation, mixed-status canonical comparison, final byte bounds and presentation identity; bounded reviewer follow-up |

The original authored BriefDocument bytes and digest remain canonical. Capability resources append
model-owned assertion evidence metadata from the same hydrated packet; ranking cannot supply or
promote status. This is a wire/presentation evolution, not a canonical codebook or relation migration.

Actual native fixture on the final pre-review transport source failed two Catalog serializer
controls and passed seven. Both writers exposed the SDK's negotiated-version reshaping and modern
serverInfo metadata occurring after the former byte check. The complete failure log is
`/tmp/lctx-phase5-native-transport_2026-10-02.log`; both CPython 3.14 ABIs compiled and were installed
(`/tmp/lctx-phase5-native-abi-final_2026-10-02.log`). Behavioral/sibling controls were not_run at that
failure boundary. The fixture now collects both-profile transport failures through independent
native checks and teardown, without weakening its final all-required-controls assertion.
The public SDK version serializer correction passed nine pure transport/numerical controls
(`/tmp/lctx-phase5-t0-public_version_serializer_2026-10-02.log`); actual full qualification is pending.


**V0 original fixture, composite passed, 2026-10-02:**
`INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo nextest run
--release -p lctx --test serving_vectors` passed 1 / 0 skipped, runtime 120.119 seconds, release
build 7m18. Run `ee4c8c95-37b3-45c2-9d3e-3fb002ef230f`; complete log
`/tmp/lctx-phase5-v0-serving_vectors_corrected_final_2026-10-02.log`. The fixture covers both
canonical vector owners, signed-zero bytes, exact positive/negative/orthogonal cosine, read-only
startup, foreign/duplicate/nonfinite/width refusal, physical metadata/content/ACL tamper,
explicit rebuild, original-guard binding, cancellation/drain and retirement cleanup. The earlier
inventory failure and missing isolated `lctx-embed` source sync remain retained failures. This
is a controlled HTTP numerical seam, not live query embedding or retrieval-quality evidence.
The later assertion-status representation changes were not in this package's dependency snapshot;
full assembled qualification will exercise the current mappings.

**Final codec/native receipt, passed, 2026-10-02:** the complete original
`cargo nextest run --release -p lctx --test serving_native` fixture on the final source passed
1 / 0 skipped, runtime 177.59 seconds. Catalog and Behavioral each passed all ten actual MCP
transport controls. The final native ABI build passed in 10.94 seconds. Logs:
`/tmp/lctx-phase5-native-final-transport_2026-10-02.log` and
`/tmp/lctx-phase5-native-resource-final-abi_2026-10-02.log`. Reservations, exact builtin admission,
valid Unicode, cancellation retention, final writer bytes, same-original distinct path identity and
canonical assertion/resource presentation were exercised. Appendix A independently accepts the
F01/F02 source corrections; full Q0 and direct mixed-status comparison remain pending.

**Current pure model controls, composite passed, 2026-10-02:**
`INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo nextest run
--release -p lctx-model --test serving_contracts --test ranking_policy` passed 32 / 0 skipped
(16 per binary), 0.026 seconds, build 1m28, run `a5da448b-2003-43e8-956f-74ce144d0f02`.
The initial compile failed because the new schema-validation control omitted the existing pinned
jsonschema dev dependency. The missing dependency was added; the unchanged control passed. Logs:
`/tmp/lctx-phase5-model-final_2026-10-02.log` and
`/tmp/lctx-phase5-model-final-corrected_2026-10-02.log`. This includes numeric assertion status
fidelity, resource labels, full-family document frequency and retained tokenizer known answers.

**Current serving fixtures, composite in progress, 2026-10-02:** the complete focused command
`INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo nextest run
--release -p lctx --test compile_facts --test serving_evidence --test serving_packets
--test serving_retrieval --test serving_soundness --no-fail-fast` finished 9 passed, 1 failed,
1 timed out, 0 skipped, 540.059 seconds (run `3278b8f1-1d44-4ac2-a6e5-ff7f668a24fd`).
The nine passes include all packet controls, the nonvacuous mixed-status canonical comparison
(73.223 seconds), actual eligibility and ranked-winner original bytes, both-profile canonical
admission/loss/cancellation and the cumulative deadline. Complete receipt:
`/tmp/lctx-phase5-serving-focused-original-artifact_2026-10-02.log`. Earlier test compile
failures (missing Serde dependency/module import and incomplete/wrong-owner typed query arguments)
remain in `/tmp/lctx-phase5-serving-focused_2026-10-02.log` and
`/tmp/lctx-phase5-serving-focused-corrected_2026-10-02.log`; production behavior and assertions
were not relaxed to repair them.

The original evidence fixture incorrectly required Local AnalysisInput parents, although Local
declares no earlier analysis parents. Selecting Behavioral alone did not repair that assumption;
this retained failure is `/tmp/lctx-phase5-evidence-soundness-corrected_2026-10-02.log`. The corrected
control uses an actual admitted BaseEvaluation parent, keeps the same canonical receipt-bound DAG
and damaged-view/captured-byte challenges, and **passed** 1 / 0 skipped in 92.778 seconds:
`INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo nextest run
--release -p lctx --test serving_evidence`, log
`/tmp/lctx-phase5-evidence-parent-corrected_2026-10-02.log`, run
`989dc486-972a-48ea-a837-31a03a3965d8`. The Unicode paging portion had already passed before the
wrong-parent assertion. The generated CPython driver exceeded the generic 300-second harness limit. Its scoped
900-second retry completed compilation of the same nine grouped programs / 52 functions, then
failed native preparation after 833.640 seconds: canonical receipt hashing rejected call arguments
read in validator grouping order instead of strictly increasing ID order. This is a production
ordering defect, not a passing oracle or solely a harness timeout. The retained failure is
`/tmp/lctx-phase5-evidence-soundness-corrected_2026-10-02.log`, run
`f58ce200-0596-4a03-b654-53becf6ce080`.

The correction verifies receipt content in ID order, then streams declared validator grouping
order when different. Both bounded passes use one read-only repeatable-read transaction on the
original leased connection; no relation-wide sort, replacement guard or larger product resource
limit is introduced. SQLx owns rollback on error/cancellation. Normalized
`cargo check --release -p lctx-postgres` passed in 39.63 seconds
(`/tmp/lctx-phase5-ordering-check_2026-10-02.log`), and the current CPython 3.14 storage ABI build
passed in 19.18 seconds (`/tmp/lctx-phase5-ordering-abi_2026-10-02.log`). The actual native fixture
now independently demonstrates differing ID and call/ordinal order with two eight-argument calls;
its original transport, guard, path and tamper controls passed on the repaired source:
`INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo nextest run
--release -p lctx --test serving_native`, 1 passed / zero skipped, 200.266 seconds, build 7m37,
run `e6db33d6-e0d8-4197-860d-23515c7e774e`, log
`/tmp/lctx-phase5-native-ordering_2026-10-02.log`. Both profiles passed all ten actual transport
controls and the original changed-operand refusal. The unchanged original CPython oracle subsequently **passed** 1 / zero skipped in 795.987
seconds (build 10.34s), run `3cf0ff2a-18ef-4745-8188-4b7aa56145d8`, command
`INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo nextest run
--release -p lctx --test serving_soundness`, log
`/tmp/lctx-phase5-soundness-ordering_2026-10-02.log`. Its original nonvacuous identity, guard,
return and composition assertions remain. Product limits and every oracle assertion are unchanged. Appendix B independently closes F02
within its scoped Tested evidence.

Additive pure model contracts are committed as `4acf8551`; the preserved initial dirty formatting
remains unstaged. This commit establishes no full Phase 5 gate or serving activation.

Canonical serving/store/CLI/bridge integration and the coupled ownership-boundary retirement
are committed as `91751d1c`. Semantic-only index copies preserve the initial dirty baseline rather
than folding its unrelated formatting into the cutover. This establishes implemented scope with
bounded current native and generated CPython evidence. All functional packages and X0 are now
implemented and focused-qualified; `just test-all` is running
(`/tmp/lctx-phase5-test-all_2026-10-02.log`). Complete functional/hygiene gates, final-source real
FastMCP journeys and activation remain pending.

**Complete-gate compile repair, 2026-10-02:** the first Phase 5
`just test-all` stopped during workspace compilation with duplicate
`ContentHash` imports in `lctx-postgres/tests/services.rs` (E0252). It establishes no workspace
test result. The redundant direct import was removed, retaining the existing grouped model
import and all service assertions. Original log: `/tmp/lctx-phase5-test-all_2026-10-02.log`.
The corrected full gate **failed** at the Rust boundary: 924 run, 916 passed (25 slow), eight
failed, two skipped, 1226.441 seconds; run `2b4edd1e-4ca8-4ade-8f7e-b2213fde4576`, log
`/tmp/lctx-phase5-test-all-corrected_2026-10-02.log`. Downstream Python, separate PostgreSQL
and doctests were not_run. Failures were the exact AttemptId hex parser, model digest snapshot,
three stale service baseline expectations, reset schema inventory, per-migration checksum
restoration, and the one-member summary fixture's assumption that a call pair is admitted.
The repairs retain original strict grants/drift/reset/identity/conditional/open-sibling controls;
the summary fixture adds an independently source-selected work-limit pair rather than inventing
closure for rejected source states. Complete Q0 remains pending.

**Bounded Q0 prerequisite, Proposed, 2026-10-02:** the preliminary real FastMCP Catalog compile
spent more than two hours CPU-active in callable normalization. Static current input counts are
78 relations / 7,251,684 rows, 4,087 decorator members and 142,032 call targets. Each admitted
class re-hashes the entire input inventory; each decorator scans all targets. This identifies
repeated work, not a measured speedup or a new Phase 0–4 review. Root owns a mechanical model
refactor: prepare the exact framed ContentHash stream once per invocation, lazily at the first
existing class inventory point, including unsupported classes and otherwise unused siblings;
index target candidates solely by source/start/end, retaining duplicate targets, ID order,
context/resolution checks and original missing-premise error order. Both caches reserve their
allocation envelope before allocation and return it on drop/error. Identity framing stays in
its current owner with no new aggregate hash or list tag. Independent scoped advice accepts
this approach at Proposed maturity and requires byte equivalence, unused-sibling invalidation,
span/duplicate/context/missing-premise and budget controls. No new semantic authority, persisted
cache, budget increase or ADR is introduced. Fresh model/producer/generation and full gate
receipts are required after these source changes; the old preliminary generation cannot qualify
the final ABI by implication.


**Bounded prerequisite controls, passed, 2026-10-02:** normalized
`INSTA_UPDATE=no cargo nextest run --release -p lctx-model --lib
--test domain_callable_aspects -E 'test(prepared_hashes) | test(full_inventory) |
test(exact_span) | binary(domain_callable_aspects)' --no-fail-fast` passed nine / zero selected
skipped (90 outside the filter), run `f5b18abb-d581-4b33-baff-92d145926e5b`, log
`/tmp/lctx-phase5-normalization-controls_2026-10-02.log`. Original independent callable/default
controls, explicit byte framing, original inventory replay, unused-sibling invalidation,
duplicate/context/span candidates, absent premises, empty targets and allocation refusal/drop
are exercised. Compile check of `lctx-model` and `lctx-postgres` passed in 36.77 seconds,
`/tmp/lctx-phase5-qualification-repairs-check_2026-10-02.log`. The target-index reservation then
added a fixed root-node envelope for small cardinalities; final assembled controls must cover
that source. No speedup is measured or claimed.


**Original symbolic/summary controls, passed, 2026-10-02:** normalized
`INSTA_UPDATE=no cargo nextest run --release -p cpg-extract
--test symbolic_fields --test summary_consequences --no-fail-fast` passed six / zero skipped,
4.480 seconds, build 1m52, run `eae8a153-1817-457e-aaca-464ca1dab4f6`, log
`/tmp/lctx-phase5-symbolic-summary-repairs_2026-10-02.log`. This covers actual native extraction,
exact/unsupported source fields, the revised global one-member premise and the independently
source-selected open call pair. Full gate remains pending; no performance claim is established.


**Exact store controls, passed, 2026-10-02:** normalized
`INSTA_UPDATE=no cargo nextest run --release -p lctx-postgres
--test services --test installation --no-fail-fast` passed 17 / zero skipped, 122.228 seconds,
build 9.70 seconds, run `30e527fd-2cdf-4c5c-8f54-78f63fb63120`, log
`/tmp/lctx-phase5-store-expectation-repairs_2026-10-02.log`. Exact schemas, service migration
versions, vector artifact objects, extension and grant matrix are retained. The drift challenge
now restores each original migration checksum to its own version; final clean inspection,
phased reset and untouched foreign objects remain mandatory.


**Preliminary Q0 source transition, interrupted, 2026-10-02:** the original N0 CLI
PID 1693662 received a scoped SIGINT only after verification of its owned executable and after
the new normalization controls passed. It exited 130; its backend, locks and other local lctx
sessions are absent. The original unselected staging generation
`aed6c594573c68b317428c1212383869`, 204 receipts and unchanged log are preserved. It had spent
2h50 elapsed / 2h36 CPU before stopping in callable normalization. Receipt:
`/tmp/lctx-phase5-q0-preliminary-interruption_2026-10-02.json`. Reason is changed source/model,
not stage failure. Its Model/producer hashes remain original; fresh final-source both-profile
journeys are required. No store reset, abort, drop or new compile occurred at quiescence.

**Hygiene, composite passed, 2026-10-02:** the earlier `just hygiene` failure at
ADR lint is retained in `/tmp/lctx-phase5-hygiene_2026-10-02.log`. After the automatic
ADR/formatting hook, `just hygiene` reached Ruff and found eight residual line lengths:
`/tmp/lctx-phase5-hygiene-current_2026-10-02.log`. The recovery SQL/constants and embedded
oracle AST remain unchanged after those repairs; nine focused Python controls passed in
`/tmp/lctx-phase5-hygiene-script-repairs-corrected_2026-10-02.log`. The initial wrong test-path
invocation collected nothing and is not a pass. `just ruff` and `just types` passed their
named final reruns. The authorized `just build-features` refreshed the retired dependency
union; `just deps` passed, `/tmp/lctx-phase5-hygiene-deps-final_2026-10-02.log`.

The final `PYO3_PYTHON=.venv/bin/python just hygiene` passed all source/policy checks,
including workspace/all-target `cargo clippy --release --workspace --all-targets --quiet --
-D warnings`, and stopped only at the obsolete local installation:
`/tmp/lctx-phase5-hygiene-final_2026-10-02.log`. The CLI and both CPython bridges were
rebuilt from the refreshed tree and installed; build passed in 8m32s, model
`dac961ba186c11b01144120a749e0be4d3c7aec8fe7bd908a052850955fbf025`.
Original gate/repair receipts retain their earlier source boundary; this is not a full
functional or real-library qualification.

The scoped reset inventoried only the interrupted, unselected `aed6c594573c68b317428c1212383869`
generation. Its 205 generation/output receipt records were captured in
`/tmp/lctx-phase5-hygiene-obsolete-generation-receipts_2026-10-02.jsonl` before retirement.
Reset initially exhausted PostgreSQL's default shared lock table at 1,086 tables / 11,914
constraints, after withdrawing installation. Local administration configured
`max_locks_per_transaction=512`; the operator restarted the system service because the
agent OS account lacks passwordless sudo. Readback verified `512`, `pending_restart=false`.
The resumable reset then passed, `/tmp/lctx-phase5-hygiene-store-reset-resumed_2026-10-02.log`,
and named `just store-check` passed with zero generations / zero findings,
`/tmp/lctx-phase5-hygiene-store-final_2026-10-02.log`. Its final build and CLI steps both passed. This closes the live-store component
of the same-tree composite hygiene receipt. The runbook owns the current local capacity
prerequisite. No formatter was run manually, and no runtime compatibility generation remains.


**Model digest snapshot migration, Tested, 2026-10-02:** the original model-description
control produced a new snapshot with only the model digest changed from `e66cccc2` to
`dac961ba`; the full `.snap.new` diff and parsed relation/invariant/codebook bodies were
reviewed and verified unchanged. Scoped `cargo insta accept --manifest-path
crates/lctx/Cargo.toml --snapshot model_describe__model_describe.snap` accepted that contract
migration. The original `INSTA_UPDATE=no ... cargo nextest run --release -p lctx --test
model_describe --no-fail-fast` rerun passed one / zero skipped:
`/tmp/lctx-phase5-model-snapshot-accepted_2026-10-02.log`. The pre-acceptance failure is
retained in `/tmp/lctx-phase5-model-snapshot-final_2026-10-02.log`; this is not a new
semantic relation or codebook change.


**Current model/native/wire repairs, passed, 2026-10-02:** normalized
`INSTA_UPDATE=no cargo nextest run --release -p lctx-model --lib
--test serving_contracts --test ranking_policy --test native_requests -E 'test(prepared_hashes) |
test(full_inventory) | test(exact_span) | binary(serving_contracts) | binary(ranking_policy) |
binary(native_requests)' --no-fail-fast` passed 51 selected tests (90 outside the filter),
0.028 seconds, build 2m39, run `fe0fd80f-b598-4790-ad40-62cb2df3c0fa`, log
`/tmp/lctx-phase5-final-model-repair-controls_2026-10-02.log`. This covers the final root-node
allocation envelope and signed-cursor byte rejection while retaining all original assertions.
After the explicit HTTP context composition repair, `just types` passed and the scoped SIM117
check passed; actual transport is deferred to the rebuilt ABI/full gate, not claimed by a
mocked provider. Complete Q0 and generated-file refreshes remain pending.


**Exact attempt-ID parser, passed, 2026-10-02:** normalized
`INSTA_UPDATE=no cargo nextest run --release -p lctx --bin lctx
-E 'test(an_attempt_id_is_exactly_32_hex_digits)'` passed one selected test (six outside the
filter), 0.002 seconds, build 6m03, run `14d92c69-661a-43f5-9da2-a858e9783d4c`, log
`/tmp/lctx-phase5-attempt-id-parser_2026-10-02.log`. The original control keeps uppercase legal
hex and rejects signed byte pairs, malformed/Unicode and incorrect widths. Complete Q0 remains
pending the scoped source stabilization and final ABI/gates/journeys.

**Operator stop and final receipt, 2026-10-02 (local date):** the operator accepted the current
work as sufficient for now and explicitly stopped further qualification. All owned qualification
processes are stopped; no new checks, compile or activation are scheduled in this session.
The implemented scope and scoped assembled acceptance remain; full Q0 is not claimed.

`PYO3_PYTHON=.venv/bin/python just test-all` rebuilt successfully and
its workspace run `768e3cf4-610c-4e52-bdb8-b50d479fbe30` **passed** 927 / two skipped,
1145.650s. This covers all earlier eight gate repairs and current native/transport/vector controls;
the generated CPython oracle passed in 766.063s. Python **passed** 228 in 20.58s.
The separate PostgreSQL run `1fce640e-74b6-488f-9347-7756c45d0496` was deliberately interrupted:
349 passed, eight SIGINT exits, two skipped, 27 not_run, 591.635s. Nextest/just exited 100.
Those signal exits are cancellation, not eight new source defects. Doctests are **not_run**.
The named complete gate is therefore **interrupted / not completed**, not passed. Complete log:
`/tmp/lctx-phase5-test-all-final_2026-10-02.log`. Composite hygiene remains **passed**, including
all-target Clippy. The final assembled review/owner reconciliation docs check passed 260 pages
and zero link errors (`/tmp/lctx-phase5-q0-reviewed-docs_2026-10-02.log`); this stop handoff did
not start another check.

The pinned final-source CLI (SHA-256 `924d7db6f0aa5f001400624b0145112ba54895a09c8fb232745cd62d7f8d2beb`)
ran `compile fastmcp --through catalog --profile catalog --embedder fake --techniques +knn`.
It was stopped by scoped SIGINT after 38m54s elapsed / 32m02s CPU and exited 130. Its process,
database backend and locks are gone. Generation `de1e0c2b09a319134fcef595a9bcbe89` remains
staging / Catalog / unselected, with 204 receipts. Last completed stage was normalize_receivers;
normalize_callable_aspects had 51 read checks but no outputs. No stage failure is reported.
Model is `dac961ba186c11b01144120a749e0be4d3c7aec8fe7bd908a052850955fbf025`;
producer/schedule is `a8d31b6223edfb5d0c505da809aad9d458f51a12e8afab23979fce6b6be39d1f`.
Interruption receipt: `/tmp/lctx-phase5-q0-final-interruption_2026-10-03.json` (UTC filename);
raw compile stdout/stderr are `/tmp/lctx-phase5-q0-final-catalog-compile.{json,log}`.

Behavioral compilation, explicit vector preparation, both final real-library native/MCP
journeys and operator selection/activation are **not_run / deferred at operator request**.
The controlled fake seam establishes no live embedding or product quality. The current staging
generation and inactive execution worktrees are preserved as restart context; no reset, abort,
drop or selection was performed at this stop. Resume only on a new operator request, first
reconciling this checkpoint with the current tree; omitted gates and journeys still bound any
future full Phase 5 qualification claim. Parent finding remainders are not closed by this stop.


### Current Q0 execution — lexical-only campaign, 2026-10-03

The operator authorized execution after selecting local stdio activation with no client-registration
changes, and deliberately deferred embeddings until the analytical outputs can inform what is
embedded. The requested FastMCP 4.0.5 catalog and behavioral generations would include communities, PageRank,
FCA/RCA and type/mention layers. kNN, its community layer and all embeddings are off. This is
a scoped lexical qualification/activation boundary; full hybrid Q0 remains separate. No new
semantic authority, public route, vector policy, dependency pin or unrequested analysis is added.

Baseline is clean main `6885dbf6a8c0083bc95162ac83c91b225680589d`. Persistent source/input
manifest and command receipts are under
`/home/paul/.cache/lctx-phase5-qualification/2026-10-03/`; `baseline.json` captures manifest,
lockfile, toolchain and analytic-configuration hashes. The previous interrupted generation
`de1e0c2b09a319134fcef595a9bcbe89` was retired during alignment after receipt archival; the initial
read-only inventory was empty. It cannot be resumed or used to qualify current source.

| Command / scope | Current result |
|---|---|
| `just test-cli-build` | **passed**; current release CLI, model `12242a5558134418fc3b54fdb526628d1715720e60e6199a8a6079f6066582da` |
| `just native-adapter-ready` and explicit embedded-source comparison | **passed**; both locked native members rebuilt/installed; CLI and installed storage match all 240 current model sources (`adapter-source-agreement.json`) |
| `just postgres-test-ready` | **passed**; both pinned images present |
| `target/release/lctx store check` | **passed**; zero generations/findings; role configuration files are regular mode0600 |
| `compile fastmcp --through catalog --profile catalog --embedder none --techniques '+communities,+pagerank,+fca,+rca,+type-layer,+mention-layer,-knn,-knn-layer'` | **interrupted**, scoped SIGINT/exit130 at the operator pivot; unselected staging `d3a3fa026a1237a1c4e175b9b1019eeb` has 204 stage receipts; `catalog-compile.json/.log/.exit` |
| Same compilation with `--profile behavioral` | **not_run**; stopped at operator pivot |
| Real-library runner, journeys, final complete gate/hygiene and stdio selection/activation | **not_run** for real-library journeys/gates/activation; runner focused controls passed, preserved below |
| Live embedding, real-library vector preparation and hybrid retrieval | **not_run / deliberately deferred**; analytical-output review precedes embedding-target work |

One production writer owns the qualification runner and focused real-PG fixture control; root
owns campaign, integration, lifecycle, shared documentation and acceptance. Independent
source/output mapping proceeds read-only. Native/default uncertainty, original byte evidence,
lexical-only channel reporting, profile distinction and final response bounds must survive the
real-library journeys. Requested analytical outcomes are reported faithfully, not converted
into success by an empty result or an unavailable optional native model. Production identity
changes require fresh affected generations. Full gates wait until functional runner/repairs
are complete; the hook retains formatting/generator ownership.


**Operator pivot, 2026-10-03:** qualification stopped in favor of planning a follow-up to the
[code-facts review](../design_review/reviews/design_review_code-facts-opportunity_2026-10-03.md).
The new target assumes upgraded Pyrefly and ty plus latest Ruff as an independent fact provider;
the operator directed removal of the restriction to Pyrefly’s internally linked Ruff version.
This is target-review scope, not a pin migration or implementation claim. The compile process is
gone, the generation reports writer `interrupted`, readers zero, and the local database has no other
sessions. No publication, selection or activation occurred. Stop state and all digests are captured
in `operator-pivot-interruption.json` and `catalog-interruption-generation.json` in the campaign
folder above. No staging schema or qualification evidence was deleted.

Preserved uncommitted implementation: `scripts/qualify_serving.py` exercises actual stdio, Rust
inventory/DTO schemas, complete discovery, independently sourced signatures/defaults and original
bytes, scalar native requests, all ten tools, lifetime protocol capture and natural shutdown.
Its real disposable-PG both-profile control passed one / zero skipped, 173.281s, Nextest
`4f52cfcc-31db-4f12-acb0-72f381019355` (`runner-focused-final.log`); generation/evidence/channel/
literal-default faults are challenged against received replies. `uv run --no-sync pytest -q
tests/scripts/test_qualify_serving.py` passed two transcript/redaction controls
(`runner-transcript-focused.log`). These fixture receipts do not qualify FastMCP.

The runner exposed a source-signature default overwrite: packet hydration replaced an already
known source literal with the effective slot’s canonical Unknown. Removing that overwrite
preserves the separate source/effective contracts without altering stored relations/model identity.
Normalized `INSTA_UPDATE=no cargo nextest run --release -p lctx
--test serving_packets -E 'test(mandatory_packet_preserves_defaults_formals_contexts_and_set_hydration)'
--no-fail-fast` passed one (four outside filter), 62.490s, run
`1ed7b371-7840-4cd1-90c1-17eb08f48d7f` (`source-default-packet-control.log`).
`just native-adapter-ready` passed after refreshing storage (`source-default-native-refresh.log`).

**Current fixture integration, 2026-10-05 — composite passed.** ADR-0126's explicit serving
family includes `serving_qualification`; actual Catalog/Behavioral producer/store/stdio qualification
passed after strict Corpus correspondence and empty-domain repairs. Empty `SearchEvidenceRequest.families`
now uses the route default; immutable cohort and current native/MCP controls passed. These replace
the two pending integration obligations above. The [assurance coordinator](testing-architecture-pivot-plan_2026-10-04.md#7-current-contractcontrol-map-and-execution-checkpoint)
owns the current schema/adapters, independent oracle and full keep-going Clippy repair receipt.
Real-library compilation/journeys, analytical-output review, live embedding and selected-startup
activation remain **not_run / stopped**. Fixture qualification does not select or activate FastMCP.
