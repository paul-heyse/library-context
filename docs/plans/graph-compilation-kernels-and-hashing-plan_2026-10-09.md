# Model-owned graph compilation and shared relation kernels

**GK0–GK6 and GR0–GR5 integration Implemented / focused Tested, 2026-10-09;
GK7/GR6 integrated qualification remains open.** This plan develops the
[graph compilation and hashing review](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md).
The [persisted-execution coordinator](persisted-graph-execution-plan_2026-10-07.md) owns the
combined target, source-qualified finding disposition, cross-plan sequencing and acceptance.
The [reuse companion](graph-compilation-reuse-and-invalidation-plan_2026-10-09.md) owns portable
products, fingerprints, dependency propagation and cross-run reuse. Neither companion is a
second progress ledger.

**Unified persistence integration, Proposed, 2026-10-09:** the
[unified companion](unified-persistent-surrealdb-plan_2026-10-09.md) supplies shared native
payload/anchor/exact-view resolution and checked attachment. GK retains model-owned operations,
complete membership/absence, canonical programs, exact-equality hashing and finite kernels.
UP2/UP4 migrate physical binding consumers; UP5 migrates scoped reader pins. UP7's stable logical
fixtures replace disposable acceptance setup; UP9 integrates surviving GK7/GR6 obligations.
Existing compiled code and receipts remain their dated baseline, not unified-store qualification.

**Holistic refinement, Proposed, 2026-10-10:** [holistic companion](holistic-state-management-plan_2026-10-10.md) §3/§4/§7
adds shared recovery-closure meaning, complete array-free native selection, graph/binding release
after the last actual consumer and charged ranked borrowers with selected continuation policy.
Full projection universe, exact membership/absence and existing GK7 qualification survive.

## 1. Outcome and assessed foundations

Compile model-owned relation operations into shared prepared execution. The operation owner
decides roots, membership, qualification, coverage, required absence domains and outcomes once;
physical lowerings select indexed native access, relational execution or compact Rust graph
kernels. Multiple logical roots can share physical selection and decoding while retaining
separate result partitions, validity and admission outcomes. Hashing supports canonical program
sharing and the reuse companion's products.

Authoring baseline: main `8f11d9a0`, 2026-10-09, with only the supplied
`docs/graph_hashing_reference/` untracked. The reviewed code baseline is `0ee13f4c`; intervening
commit `8f11d9a0` added the review and handoff, not production changes. Core/template3.3,
heuristics1.0 and code-intelligence1.5 apply through
[standard.toml](../design_review/design_principles/standard.toml). The reference review's static
diagnoses are reused; no build, probe, operator action or performance measurement was run here.

Relevant foundations are exact completed contributions/views, captured suppliers, typed
invariant and scope declarations, canonical identities, shared topology, native indexed providers,
bounded Arrow transfer, spillable ordering and contained synchronous kernels. They are useful
implemented mechanisms, not proof of the new compiler's readiness. Current `PreparedEdges`
shares dependency preparation but still traverses/queries/hydrates per grain. `AspectScope`
declares roots while core's SQL and model `ClassInventory` separately interpret body/coverage
membership. F02 supplies the operation-authority correction needed for aggressive F01 sharing.

The workload is pinned-library compilation through many overlapping roots, large classes and
high-degree outliers, optional upper analyses, and repeated native requests on immutable views.
The target removes repeated preparation and crossing; it does not narrow required semantic
universes or promise a numerical speedup. Cold construction, conversion, retained state and Rust
code generation are assessed separately from repeated execution.

## 2. Confirmed decisions and supersession

| Source-qualified decision | Operator outcome, 2026-10-09 | Route before dependent implementation |
|---|---|---|
| [Graph/hash RC01](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#RC01) | **Accepted:** relation operations may compile to graph kernels, indexed native queries or relational operators | GK0 records the placement/composition decision and updates semantic-model §15.10; DESIGN §B3 is already a compatible seam. Accepted ADRs are immutable: add a complementary decision, superseding only genuinely conflicting clauses through the ADR workflow. |
| [Graph/hash RC02](../design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md#RC02) | **Accepted:** selective cross-run reuse is in the target now | GK0/GR0 replace semantic-model §15.1 and product §14.2 deferrals, preserving one canonical semantic authority and independent admission. The reuse companion specifies the selected engine and store. |

The compiler/program and shared-demand target supersedes open per-root preparation routes in
PG2/PG4/PG6, PC3/PC4 and the populated companion's normalization/validation deferral. Existing
completion, body reconciliation, candidate ordering and pointer-run mechanisms remain foundations
and acceptance obligations. PC6/CU6/BC3/BC5 need final-source evidence after affected migrations;
they do not have to finish before this replacement is designed or implemented.

The source review's selector-only remedy remains a comparison baseline. It is not a second
implementation track. The selected scope compiler supplies a bounded semantic seam across
normalization, admission and serving; it can retain ordinary owned functions for complete
operations. No whole-language interpreter, plugin framework, generic task engine or conversion
of BDD/FCA algorithms into an operation language is required.

## 3. The model governs operations

### 3.1 Three graphs, distinct contracts

The semantic graph retains entities, identity-bearing arcs, roles, qualifications and evidence.
The producer graph retains readiness, exact completed sources and stage effects. The new
operation graph composes selections, membership, projection, kernel application and validation.
An operation node is neither a record kind nor automatically a task/table/native transaction.
Semantic cycles use their owned fixed-point or refusal policy; the orchestration DAG does not
reinterpret them as scheduling errors.

`lctx-model::domain` owns the bounded typed operation definitions. `cpg-core` binds exact inputs
and requested output demand, compiles physical plans and invokes kernels. `lctx-surrealdb` owns
native indexing, transport, terminal checks and mechanical field lowering. `lctx-analytics`
and existing model kernels retain algorithms. Publication remains an external effect owner.

### 3.2 Complete scope operations

An operation declaration consumes its named model/vocabulary and ordered input declarations,
typed root parameters and policy. It produces selected typed memberships plus explicit missing,
empty, unsupported or refused root outcomes, and declares the full inputs required for validity.
Its minimum composition includes:

- Typed relation/field references and scalar/list cardinality; forward references and owned
  reverse membership are distinct. Runtime `TypeId` can dispatch locally but is not a portable
  identifier or cache encoding.
- Logical root partitions and separate advertised admission roots. Class, callable assessment
  and field initializer roots remain distinct even when their physical work overlaps.
- Body containment over exact source, span and structural role/path; class initializer expansion;
  owner links and qualification(scope, context) to coverage joins. These meanings move from
  independently authored core SQL into the normalized model owner.
- Typed unions/projections and explicit closure/fixpoint composition. Deduplication uses the
  declared relation identity, preserving parallel evidence, positional/list semantics and
  meaningful repeated declarations. Only duplicate physical demand is freely coalesced.
- Complete negative/global domains, including missing target namespaces and advertised rows
  without owners. A selected positive set is not proof of completeness or absence.
- Ordered result application, model-owned kernel/validator revision and expected output inventory.
  Selection never substitutes for `SupportCheck`, reference admission or other actual predicates.

Start with `AspectScope`, `ExecutionScope` and `SupportScope`, extending their existing typed
contracts rather than adding a parallel policy registry. Move body/coverage predicates and class
inventory selection to this owner. A finite Rust reference evaluator and all physical lowerings
consume it. Domain algorithms remain ordinary functions where an inspectable composition adds
no useful capability. No arbitrary SQL, unrestricted expression evaluator or request-provided
code is exposed by the program contract.

### 3.3 Binding, preparation and result partitions

Bind every input to its exact completed descriptor, prefix, model, profile and policy. Reject
ambiguous/missing bindings explicitly. Compile the immutable program once per applicable model
and operation shape; prepare source-specific access/layout once per exact dependency lifetime.
Root IDs parameterize the prepared plan; they do not require rebuilding its semantic program.

**Proposed physical refinement, 2026-10-09:** the
[native-efficiency companion](native-execution-efficiency-plan_2026-10-09.md#3-target-responsibilities-and-shared-contracts)
NE1 resolves ordered role/epoch selectors once into exact immutable bindings consumed by provider
registration, products and native resolution. NE2 removes amplified owner/key enumeration using
qualified equality-prefix/verified-pointer or compact exact-view selection; native union-IN
is not presumed bounded. NE3 shares captured providers, checked logical preparation and compatible
union inputs across independent admission grains, including complete OwnershipRows::Full domains.
Physical grouping does not share semantic acceptance or mutate catalogs beneath other grains.

The shared root executor returns `(root partition, relation namespace, nominal key)` memberships,
explicit root outcomes and a shared compact union for physical demand. Roots that exist only as
advertised admission obligations are processed and refused when appropriate, never omitted.
Owner-specific views expose the exact rows/order needed by each kernel. Shared hydration of
immutable premises uses bounded windows or compact handles; all rich data are not retained just
to avoid another decode. Existing `.find` versus multi-prefix traversal cardinality and ordered
`ProductionScope`/merge behavior are preserved unless a separately justified correction changes them.

Batch compatible roots using their common program/input bindings, then retain per-root ownership.
An outlier may use streaming individual demand against the same prepared program. Construction
and retained memberships are charged to the existing budget; spill existing sortable compact
state where necessary. Batch boundaries are physical choices, not new semantic scope or validity.
Cancellation checks occur in CPU traversal/batch boundaries, and submitted transports drain
before their reservations or scratch are released. Available thread/compiler parallelism remains.

### 3.4 Contrasting physical lowerings

| Lowering | Selected use and preserved contract | Construction/lifetime tradeoff |
|---|---|---|
| Native indexed + DataFusion | Sparse selected access, nominal membership, exact context joins, projection and spillable set operations; reuse existing prepared constants, result positions and terminal inventory | Avoid broad rich hydration. Retain `Exact` filter semantics, residual evaluation and safe fetch placement; multiple statement failures/late errors remain visible. |
| Compact Rust graph/index kernel | Repeated traversal and set membership over one stable selected universe; shared both-direction adjacency, dense indices and owner partitions | Build once when reuse amortizes conversion. Keep typed nominal reverse mapping, isolated vertices, full intermediates and identity-bearing parallel arcs. Charge indexes/visit state; retain spill/indexed access for excessive compact state. |
| Existing transfer/fixpoint kernel | BDD, execution, Summary SCCs, FCA and governed analytics with their existing semantic operations | Feed prepared selected inputs; preserve assumptions, five verdicts, non-convergence/refusal, seed/settings and witness order. No topological-reachability substitution. |

Use petgraph0.8.3 and fixedbitset0.5.7 for existing graph/dense-set contracts. A CSR physical quotient
is allowed only where edge multiplicity is semantically a set and exact ArcId/evidence mapping is
retained. It cannot replace the canonical multigraph wholesale. Universe generation and length
belong to bitset/visit state. Preserve existing native indexes and DataFusion55.1.0 optimizer
capabilities; do not build a second optimizer or assume every graph must be fully loaded.

Lowering selection is explicit per operation family at compilation, based on its access/reuse
shape, not a new adaptive cost model. GK2 delivers the two contrasting scope lowerings and
independent known-answer equivalence; consumer packages choose one production lowering and
retire their old interpretation. Existing graph projections keep their independent universes.

### 3.5 Hashing and Rust build consequences

GR1 supplies canonical program encoding and an XXH3-128 in-memory interner with exact structural
equality. Dense indices and prepared adjacency remove hash lookups where possible; ordered maps
remain where canonical order or sparse lookup is useful. No global map-hasher replacement is
scheduled. Persisted identities, product manifests and actual content remain BLAKE3.

Use one bounded non-generic physical execution loop and synchronous typed leaves. Retain coarse
async request/stream boundaries and thin adapters before task submission. Do not generate one
generic async branch per row/relation/plan alternative. Program hashing and serialization describe
bounded intent; native machine-code specialization and e-graphs are not required by these
operations. Existing BDD canonicalization is reused before proposing another expression interner.

## 4. Consumer migration and concrete packages

| Family / exact migration boundary | Target and preservation |
|---|---|
| Facts/provider contracts and availability | Bind captured declarations and complete source/coverage domains into the common operation/dependency inventory. Providers retain native integration and attributed outcomes; no new provider algorithm. |
| Entity/event/binding/callable/field normalization | Compile owner/body/context selectors once, batch compatible roots and apply pure normalizers to owner views. Missing owners, unresolved attachments, prefix cardinality and source identity remain explicit. |
| Execution, Local, Model, Summary and support admission | Compile their existing ordinary/contextual memberships and negative/global domains. Preserve ordered application/merge, BDD/transfer semantics, independent support predicates and global references. |
| Structural, projections and selected Analytic | Reuse prepared topology and actual method-input union; retain isolates/parallel arcs/SCC policy, complete graph chunks, optional NotRequested and governed uncertainty. |
| C0/C1/C2, S0 and E0/retrieval preparation | Compile shared metadata/closure demand; preserve original artifact/member/context partitions, witnesses, selected columns/ranges, compact S0 spool and existing embedding-cache ownership. |
| Native inspection/browse/evidence/selection | Reuse the same model-owned programs against one pinned realization; batch typed demand and use the reuse companion's viewer-owned preparation. Ranking, complete alternatives/unknowns, evidence and cursors retain their contracts. |
| Workspace admission, cold import/restore, publication | Derive access from the owned scope program, retain full actual-state and negative/reference obligations, drain/freeze and independent admission. Never cache admitted capabilities or publication effects. |

The family map is a completion inventory, not permission to stop after callable aspects. Each
package records every actual caller and its negative/ordering/policy dependencies. Unaffected
finite kernels and transports stay as consumers of the improved preparation, with a documented
reason where no operation program is necessary.

| Package | Working prerequisites | Delivered behavior and closure boundary |
|---|---|---|
| **GK0 — decisions and combined target** | RC01/RC02 confirmed; target-plan review resolved | Complementary ADR/owner changes, supersession map, revision/cutover rules and shared model/compiler/reuse contracts before dependent code. |
| **GK1 — authoritative scope model** | GK0 | Complete Aspect/Execution/Support operation composition and finite evaluator; ClassInventory and compiler callers stop independently authoring body/coverage meaning. F02 extension/substitution cases pass. |
| **GK2 — shared compiled selection** | GK1; existing PC completion/access contracts | Native/DataFusion and compact Rust scope lowerings, owner partitions, shared discovery/hydration and exact root outcomes; overlapping and skew controls establish F01's replacement. HS3/HS4 extend shared closure selectors through complete multi-window roles/occurrences; streaming alone is insufficient physical evidence. |
| **GK3 — normalization and admission integration** | Working GK2; GR1 program identity | All normalization and execution/support selector consumers migrate, including entity/admission paths exposed by timeouts; no correctness inference from faster completion. |
| **GK4 — upper compiler and graph kernels** | GK2; needed GK3 outputs | Structural/Local/Model/Summary/Analytic and C0–C2/S0/E0 consumers use their actual shared demand and retained pure kernels; old duplicate selectors retire alongside migration. HS8 inventories actual optional/cache-hit consumers and releases each heavy graph/binding preparation at last use. |
| **GK5 — pinned serving integration** | GK2 and published exact-input binding; GR5 for cached preparation | Native inspection/browse/evidence/selection use model programs and reusable prepared inputs; public contract unchanged, independent request and pin controls. HS2/HS9 compose ranked retention/replay with service charges, selected limits and request/close ownership. |
| **GK6 — compiler/reuse composition** | GK3–GK5 plus working GR2–GR5 products | Driver uses qualified products and dependency propagation; ownership/admission/provenance remain current after skipped pure computation. HS8 preserves small async phases and finite kernels; conditional cross-attempt runtime injection requires a real repeated production consumer. |
| **GK7 — integrated retirement and qualification** | All actual consumers, GR6 and affected PC/CU/BC contracts | Replaced interpretations/preparation removed; final both-profile/frontier/native/MCP/transport controls and applicable leaves; coordinator closes only evidence-established findings. |

GK1's semantic correction precedes aggressive sharing. GK2 can precede unrelated older journeys;
GK3 and GK4 consumers depend on delivered input slices rather than whole-document completion.
GK5 can use working scope bindings independently of cross-run compiler reuse. One root owns
shared declarations, manifests/schema, integration and acceptance; logical independence does not
permit competing writers in the same files.

The [native-efficiency packages NE0–NE9](native-execution-efficiency-plan_2026-10-09.md#4-packages-and-prerequisites)
refine implemented GK foundations before GK7/GR6 qualification. NE0 records accepted access/local
completion decisions; NE1–NE3 improve actual binding/access/admission; NE5/NE6 preserve contained
native ownership while replacing polling/global local waits; NE7 refines GK5 preparation. None
resets GK identities or closes the older source findings. The coordinator owns replacement
mapping and all final-source receipts.

## 5. Acceptance, cutover and investigations

**Planned / not_run.** During implementation use compile checks and minimal revealing controls
on pinned release artifacts, normal available parallelism and owned disposable fixtures.
Resolve actual target/filter commands with `just verify --print --select ...` or the relevant
bare command under `just fixture`. Never run broad families after each slice. At GK7/GR6 schedule
one affected assembled `just qualify` at the NE9/GK7/GR6 boundary because shared model/dependency/admission contracts change,
plus surviving PC6/CU6/BC3/BC5 obligations on matching final source/profile. Separate operator
adoption, real-library/Qwen/protected evaluation and performance measurement remain unauthorized.

| Behavior | Independent revealing evidence |
|---|---|
| One semantic owner | Nested class, equal spans with different structural roles, field initializer, foreign context/coverage, absent advertised owner; extend one rule and substitute native versus graph lowering without independently changing semantics. |
| Shared selection | Overlapping class/field/assessment and event roots with independent expected memberships; compare per-owner results, incomplete/empty roots and conflicts. Include a skewed outlier, reordered inputs and scalar/list/null cases. |
| Graph fidelity | Isolates, parallel identity-bearing arcs, self-loop, reconvergence, cross-module cycle and filtered intermediate vertices; canonical witnesses/order and exact nominal mapping. |
| Lifecycle | Cancel one consumer while another reads shared state; cancellation/late terminal failure during hydration and writes; live charges/scratch persist to terminality, no publication before final admission. |
| Kernel/build preservation | Ordered model merge, first-error/cardinality, selected analytic inputs and five verdicts; inspect emitted parent/typed-child boundaries in necessary builds and retain compilation receipts' scope. |
| Serving | Same-pin complete answers/unknowns/evidence/cursors across overlapping requests; old/new pins never share attempt-bound providers or permissions. |

Changing physical lowerings or introducing derived program/cache schemas uses fresh fixtures and
version-qualified disposable products; obsolete derived formats are rebuilt, with no compatibility
reader. Any required published schema/semantic contract change is explicit in GK0 and snapshots;
do not advance completed-state/artifact/codebooks merely for private cache metadata.

The review's remaining questions have implementation routes: GK2 compares sparse native access
and dense preparation over the same domain; GK3 addresses normalized reference/admission demand;
GK4 resolves batch/whole-universe and optional-method fit; GK5 addresses repeated serving
preparation. GR0–GR6 settle the seven hashing/reuse opportunities. No route waits for an older
plan to finish simply because its scope overlaps. Existing timeout receipts remain failed/open;
the share attributable to these changes remains unproven until separately measured.

**Authoring completion:** independent design/target review of both companions and the revised
coordinator; `just docs-check`, scoped `just turn-end` and handoff. These checks establish document
integrity, not implementation, measured benefit or closure of F01/F02.

**Target reviewed, 2026-10-09:** the [independent plan-target review](../design_review/reviews/design_review_graph-compilation-and-reuse-plan_2026-10-09.md)
accepts the combined target at Proposed/interface strength. Its separate GR5 retirement-order
finding is corrected in the target; the coordinator §8 retains the runtime control obligation.
Source graph/hash F01/F02 remain open until implementation and closure evidence exist.


**Execution refinement, 2026-10-09:** GK6 composes shared model-owned programs and selected
layouts with the reuse companion's exact products and complete selected domains. Current
ownership and admission remain independent. Behavioral private Local/Base/Body/SourceCall
owners execute afresh because complete semantic validation would repeat their kernels; no
portable private hint or second authority remains. SCC schedules execute once per prepared graph
and bind through its private materialization identity: validating a portable schedule would
repeat graph traversal and condensation ordering, then add hashing and lookup. Complete nominal
topology identities remain available for beneficial pure contracts. The driver dependency graph describes current
computational/provenance edges; it does not claim a general incremental scheduler. Viewer-owned
Moka preparation is integrated through the same immutable reader, with charged insertion owners,
pressure reclamation and close-before-reader-invalidates ordering. See the reuse companion §7
for the complete family and lifecycle boundary and coordinator §8/§9.1 for review disposition
and dated evidence. GK7/GR6, BC3 and failed full-compiler equivalence controls remain open;
focused passes do not qualify the whole pivot or establish measured benefit.

**Integrated demand correction, 2026-10-09 (Implemented / acceptance in progress).** Candidate native
controls exposed `symbol_entity_resolutions` conflicts in both profiles. The entity scope already
separates virtual computational roots from nominal lookup dependencies; the kernel incorrectly
emitted resolutions for all hydrated symbols, including support symbols lacking their reverse
candidate families. Typed output demand now carries the requested root through kernel execution,
including surviving mixed-cache partitions. Supporting facts remain available for lookup; public
demand retains all origin-matched alternatives, and parameter/field computation follows the
demanded symbol owner. The correction neither suppresses conflicting rows nor recursively expands
every supporting symbol. Revealing finite demanded-versus-full normalization controls and the
affected actual native candidate gate establish closure; coordinator §9.1 retains failed receipts.

**Advertised callable-owner correction, 2026-10-09 (Implemented / acceptance in progress).**
After the entity correction, both native normalized frontiers completed normalization and
failed independent artifact admission: nominal reachability had also selected a supporting
callable's advertised claims. Model-owned `CallableOwnerSelection` now selects the requested
callable's actual advertised assessment/evidence family, independently of recomputed expected
IDs. Broad source premises, strict same-owner extra/missing checks and global orphan probes
remain unchanged. This is the same separation of computational ownership and lookup dependency;
finite supporting-callable/adversarial controls and actual native artifact admission establish
the correction's evidence boundary in coordinator §9.1.
