# PR5 agent journeys and coarse rebuilds — assembled implementation review

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Shared dirty `main`, based on `377712ec1e818823ccde3cbee283d12c83ce4116`; source inspected 2026-09-29. The implementation owner repaired findings during this review. |
| Standard | [Core 3.0](../design_principles/core/design-principles.md), [template 3.0](../design_principles/core/design-review-template.md), [code-intelligence 1.1](../design_principles/profiles/code-intelligence/principles.md), [repository binding](../design_principles/binding/library-context.md); `design-review` and `design-review-code-intelligence` skills. |
| Tier · purpose | **Design · target**, assembled PR5 against [ADR-0081](../../adr/0081-bounded-agent-journeys-and-coarse-rebuilds.md), [product §14.9–14.10](../../design/sections/api-and-evidence-product.md#section-14-9) and the authorized detailed PR5 sequence. |
| Reviewer | Independent `pr5_design_review` agent. Production changes belong to the implementation owner; this reviewer changed only this document. |
| Outcome | **Accept scoped at Implemented / source-inspected strength.** The four material findings below have inspected corrections. This accepts the bounded ownership and implementation routes, not product qualification or deployment completion. |
| Coverage | Rust request/response/schema dispatch; operation core and section hydration; exact ownership browse and vocabulary; independent evidence ranking and expansion; comparison/classifier composition; pure-stage cache admission, keys, snapshot rebinding and CLI rebuild orchestration. Adjacent native and MCP adapters, classifier preparation, publication validation and serving leases were inspected. |
| Exclusions | No broad tests, formatting, live embedding calls, service changes, evaluation/heldout inspection, accuracy assessment or comparative scoring. New analyzer families and general behavioral completeness are outside PR5. Fine-grained reuse and speed improvements are not claimed. |
| Qualification boundary | This reviewer executed no tests. Added test sources were inspected; their existence is **Implemented**, not **Tested**. Full gates, both-profile real MCP journeys, complete rebuild-change controls, reconstruction and current-only operator cutover remain the implementation owner's work. |
| Disposition owner | [Forward plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition). This dated review is evidence, not a second mutable status register. |

The reviewed target serves four journeys: browse the supported surface and vocabulary; find an
original independently of an API; inspect a complete invocation contract and then optional
enrichment; compare named APIs under the same requirements. Rebuild commands support these journeys
without introducing a second canonical store. PR6 comparative value remains unassessed.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Coherent responsibility and consumer contract | Expected reason for change |
|---|---|---|
| `cpg-schema::wire` | Nominal identities, finite requests/results, defaults, bounds and generated schemas. `dispatch` drives decoding and schema lookup; `tool_budget` supplies the shared response policy. | Add a supported journey or change an explicit representation contract. Python does not redeclare semantic request fields. |
| `lctx-postgres::hydration`, `packet` | Shared member/singleton invocation resolution, mandatory catalog closure and independently paged optional sections. | Invocation changes belong to the shared loader; a new optional section changes its typed section contract and hydration route. |
| `lctx-postgres::journeys` | Browse canonical ownership and counts; compose one prepared classifier result with ordered named candidates. | A browse presentation or comparison workflow changes orchestration, not the classifier or source ownership semantics. |
| `cpg-schema::retrieval`, Python `UnitLexical`, `lctx-postgres::evidence_search` | Domain unit/channel/family fusion; lexical scoring; pinned eligibility, exact-vector ranks and bounded response assembly. | A channel or discovery policy changes ranks and cursors, never requirement truth or scenario intent. |
| `cpg-core::catalog`, `evidence`, `catalog_domains` | Pure normalization and contextual association over declared inputs; canonical derivation and validators share these owners. | Captured membership, coverage, source coordinates, policy or evidence changes alter the appropriate pure output. |
| `cpg-core::stage_cache`, `rebuild`, `attempt` | Disposable stage artifacts; rebuild decisions; Delta materialization and validated publication. | Cache mechanisms can change without becoming canonical authority. Source/compiler identity remains explicit. |
| `lctx` rebuild command | Select captured input, optional enrichment and retrieval effects; report the actual stage receipt. | CLI options change orchestration. Publishing an artifact does not import or select it. |

Dependencies flow from schema meaning into pure transformations and effect adapters, then native
and MCP transport. No new generic workflow engine, provider framework, recipe store or incremental
database is needed. Comparison now reuses the same invocation loader as the packet (F03).

| Fact/relation | Provider/revision and fidelity | Coverage/unknown and identity | Consumer |
|---|---|---|---|
| Public member, module and class ownership | Canonical catalog and selection-domain derivation from attributed pinned facts | Distinct public member IDs retain aliases. Missing module ownership is counted; unrecorded scopes are unavailable rather than inferred from dotted names. | Browse outlines and exact scoped facet counts. |
| Signatures, parameters, constructors and types | Canonical source/provider observations with source fact IDs | Complete declared variants/defaults survive packet admission. Effective unresolved surfaces remain explicit. | Packet core; comparison shares its invocation subset. |
| Behaviors, field observations and facets | Existing selected behavioral producer, under its existing model | Not-requested and unavailable sections differ from recorded emptiness. Existing verdicts and discharge evidence are retained. | Optional sections and relationship links. |
| Scenario/deployment original and association | Canonical contextual evidence with member or release subject | Intent, checks, context and source alignment are preserved; association is not successful execution. | Independent evidence search and `get_evidence`. |
| Unit, fragment and rank | Immutable derived unit/fragment identities; BM25/exact cosine/RRF are heuristics | Winning fragments are checked against the pinned unit/family; empty lexical retrieval is not capability absence. | Discovery, never classifier evidence by itself. |
| Reused stage result | Derived from complete declared scans and code/configuration identity | Cache bytes are snapshot-neutral, checked and semantically admitted; publication names a fresh snapshot and full compiler provenance. | Compiler/rebuild only. |

## 3. Contracts, constraints and testing boundaries

| Contract | Invariant and enforcement | Effects/failure and isolated boundary |
|---|---|---|
| `GetOperationRequest` packet/section views | One tagged route; cursors bind generation, member, section, representation and policy. Required invocation closure is admitted before optional rows. | Indivisible required output refuses; optional records have whole-record continuation. Pure schema controls and real PG hydration address different boundaries. |
| Browse scope/view | Rust enum scope, canonical ownership domains, distinct member sets and generated Requirement vocabulary. Page size defaults to 20 and is bounded at 100. | Unrecorded scope and unselected behavioral facet producer have explicit state/reason; cursor changes reject. No dotted-name ownership inference. |
| Independent evidence search | Rust family/intent/subject inputs; exact stored subject membership and winner tuple validation. | Embedding happens before SQL acquisition; only embedding unavailability selects the disclosed lexical route. SQL/corruption failures remain errors. |
| Comparison | One to five caller-ordered spellings, one prepared classifier, all four outcomes and witnesses retained. | Missing and ambiguous names remain explicit. One total budget refuses excess; no synthesized winner or witness trimming. |
| Stage artifact admission | File hashes, format/schema, declared table inventory and canonical-batch equality against the existing pure owner. | Invalid disposable entries recompute. Clean mode reads neither stage cache. Final validation still reads the materialized publication. |
| Canonical publication | Shared source admission, schema-owned snapshot rebinding, replacement full validation and one snapshots publication boundary. | No automatic ready/selection transition; retrieval rebuild retains the source snapshot. New facts require extraction, not reinterpretation of absent old fields. |

Final MCP admission measures the actual SDK `CallToolResult`, including concise text and structured
content. Domain admission reserves envelope space. The 32/256 KiB policy is owned in Rust and
passed to the transport; the transport does not invent a competing semantic cap.

## 4. Composition and execution

| Question/stage | Universe, method and semantic class | Dependencies/effects and evidence | Bounds/determinism |
|---|---|---|---|
| What is in a module/class? | Canonical public-member domain, exact module/class owner, distinct exposure counts. Facet counts use sets per value/verdict. | SQL reads only; schema vocabulary derives from Requirement and codebooks. | Stable ordered groups/members and request-bound cursors; response excess is explicit. |
| How is an operation invoked? | Shared bindings → declarations/constructors → signatures → ordered parameters; packet adds config/type/provenance closure. | Pinned PG reads. No nested constructor behavior is hydrated into core. | Hydration budget, indivisible core refusal, optional section paging. |
| Which original is relevant? | Actual retrieval units, independent of API eligibility. Best fragment per unit/channel; RRF60 within families, equal family ranks, unit-ID ties. | Python lexical computation; exact PG vector scores; Rust tuple validation/fusion; original expansion through existing evidence identities. | Family/text dedup; unit/rank population limits; whole-hit byte paging; no lease during embedding or CPU fusion. |
| Which named API meets requirements? | Existing pure classifier over its declared typed domains; exact/conservative meaning unchanged. | One preparation, caller-order resolution, compact invocation loader. No second comparison classifier. | At most five candidates, shared witness rules and one response budget. |
| Can captured facts be reused? | Snapshot-neutral canonical Arrow inputs, actual scan dependencies, roots/public paths and semantic source identity. Association adds its own code/input dependencies. | Current-only disposable cache; full compiler digest remains publication provenance. Optional behavioral reuse is conservative at the validated compilation boundary. | Semantic cache mismatch becomes miss; clean oracle available; all evidence/provenance columns participate in equality. |
| Can retrieval change independently? | Existing catalog snapshot → current renderer/embedding realization → immutable artifact receipt. | Existing bundle publisher, no implicit PG import/selection. | Explicit embedding spec and artifact identity; old runtime deletion waits for validated replacement and quiescence. |

Cache admission intentionally rederives expected pure output, and final canonical validation
reconstructs it again. This is a correctness-first cache contract, not evidence of reduced
normalization/association CPU time. Behavioral/source hashing is deliberately broader than the
minimal dependency set; conservative extra misses are accepted until a measured workload warrants
a finer boundary. No performance result is inferred from a `Reused` receipt.

## 5. Change and failure scenarios

| Scenario | Observed ownership route and propagation | Settling evidence or remaining qualification |
|---|---|---|
| A tiny release supplies one distinct installation document and no API association | Independent UnitLexical policy admits matching terms, even when every unit shares the same text; eligibility reads actual release/unit subjects. | F01 corrected; the new one-text/member/release/unassociated control is source-inspected. Execution remains outside this review. |
| A canonical context variant changes its serialized meaning | `wire::requirements` is part of semantic identity; the association key combines semantic and association-specific source identity. | F02 corrected by narrowing response-only exclusions. No key stability/performance claim for unrelated shared-type edits. |
| A singleton or large class is compared with a method | Packet and comparison share singleton and invocation loading; comparison does not load and discard recursive type/configuration closure. | F03 corrected. A PG singleton-parity and large-unrelated-type control can exercise this seam without a new framework. |
| Optional behavioral records grow beyond the expanded packet budget | Mandatory core is checked first; separately ordered whole-record sections retain continuation. | Known large CLI/auth packets and final serialized MCP boundaries remain required PR5 runtime qualification. |
| A module contains unresolved bindings, or the catalog profile lacks behavioral facets | Browse uses canonical ownership and explicit state; missing ownership is retained as unknown count. Section absence does not assert no behavior. | Both-profile browse/facet controls remain part of the journey qualification. |
| A correctly encoded cache has the wrong default or association | Existing pure derivation plus canonical batch equality rejects the entry before consumers use it; already-computed expected output repairs it. | F04 corrected; valid-Arrow/wrong-value and malformed-byte controls exist. Test outcomes are not claimed here. |
| Membership, a previously absent lookup, span coordinates or attribution changes | Declared complete scans are canonicalized and hashed; cache output equality includes evidence/provenance. A new fact family is an explicit schema/extraction change. | Full clean/reused change matrix remains PR5 qualification; the current roots/clean/corruption test is not that complete matrix. |
| A query is cancelled during embedding, SQL or fusion | Embedding has no SQL lease; existing QueryLease drains unfinished server work, and CPU admission retains its permit until work ends. | Existing mechanisms are source-inspected; new journey cancellation controls are not executed here. |
| A rebuild fails before artifact import or selection | Delta publication and artifact creation remain separate from PG ready/selection. CLI prints a receipt; it does not mutate the selected operator generation. | Interrupted publication/import and current-only quiesced cutover remain implementation-owner acceptance work. |

## 6. Correctness and fidelity gates

Verdicts concern the inspected implementation contract, not a claim that its full acceptance suite
has run. Runtime qualification remains explicit in §10.

| Gate | Verdict | Evidence and scope |
|---|---|---|
| G1 Authority | pass — source inspected | Rust owns wire semantics; canonical facts and pure derivations own catalog meaning; caches/PG/MCP are derived. |
| G2 Semantic fidelity | pass — source inspected after corrections | Complete mandatory invocation closure, explicit optional state, caller-order comparison and unchanged typed classifier outcomes. |
| G3 Validity | pass — source inspected | Native request/schema admission, pinned tuple checks, semantic cache admission and final materialized validation have enforcement boundaries. |
| G4 Hidden behavior | pass — source inspected | Read-only journeys do not execute evidence or select state; embedding and persistence effects remain explicit owners. |
| G5 Consistency and recovery | pass for implementation route | Generation/request cursors, whole-record pages, explicit refusal, one canonical publication and distinct ready/selection transitions. Current operator cutover is not certified. |
| G6 Transformation and reuse | pass — source inspected after F02/F04 | Complete declared scans, corrected code identity, canonical cache comparison, full evidence/provenance rebinding and clean mode. Full mutation-matrix qualification is pending. |
| G7 Truthful capability claims | pass at the stated boundary | Scores remain heuristic; no winner/accuracy/speed claim. Fallback distinguishes no embedder, no vectors and embedding failure. This review does not claim Tested completion. |
| G8 Library leverage | pass — bounded assessment | Existing Serde/Schemars, SQLx/PG, Arrow/Delta, BM25/NumPy and SDK serialization provide generic mechanisms. Domain paging/fusion and finite stages have concrete consumers. |
| CI-G1 Fidelity | pass — source inspected | Canonical ownership, distinct exposure counts, intent/check preservation, explicit unrequested/unavailable states and existing model-relative verdicts. |
| CI-G2 Evidence closure | pass — source inspected | Winner tuple validation and pinned original-expansion route preserve unit/fragment/original identities. Comparison retains classifier witnesses. |
| CI-G3 Evaluation integrity | n.a. to evaluation execution | No comparative evaluation or tuning occurred; no sealed/evaluation input was inspected or introduced. This does not admit PR0/PR6 comparison. |

## 7. Findings and applicability

These findings identify defects observed during this review and their inspected corrections. The
forward plan owns their current qualification/disposition; source inspection does not mark pending
tests passed.

<a id="F01"></a>
### PR5/F01 — Independent evidence cannot inherit brief-only lexical abstention

**FP-01/03, DP-08, A1/A3, G6.** `Lexical.discriminating` requires `0 < df < size`.
`UnitLexical` indexes distinct family/text pairs, so one distinct text made every query abstain.
A release-only installation document was undiscoverable in lexical-only mode even for an exact
matching phrase. The implementation now selects ordinary matching-term scoring only for
`unit_winners`; legacy brief/member policy retains its previous explicit default. See
[`retrieval.py`](../../../python/lctx_mcp/src/lctx_mcp/retrieval.py), `Lexical.scores` and
`UnitLexical.unit_winners`. The independent three-unit/single-text/duplicate control is in
[`test_retrieval.py`](../../../python/lctx_mcp/tests/test_retrieval.py).
**Correction source-inspected, 2026-09-29; test outcome not claimed.**

<a id="F02"></a>
### PR5/F02 — Canonical association types are not response-only wire code

**FP-04/05, DP-09, A2, G6.** Excluding the entire wire source tree omitted result-affecting
`SelectionContext`, `SelectionDomain` and `WitnessEvidence` semantics used in canonical
`DomainDetail`. A serialized context change could change `catalog_selection_domains` and its
domain ID without changing the association key. Semantic cache equality prevented bad publication,
but did not make that key complete. [`build.rs`](../../../crates/cpg-core/build.rs) now excludes
only the named response-only wire modules; canonical shared wire types remain in semantic identity.
Association identity combines that digest and its evidence/domain transformation sources.
**Correction source-inspected, 2026-09-29; finer hash separation remains deliberately conservative.**

<a id="F03"></a>
### PR5/F03 — Comparison must share invocation resolution without full packet hydration

**FP-01/03/06, DP-08/10/17, A1/A3, G2.** Comparison originally called full `catalog_record`,
discarded its recursively loaded types/configuration, and passed no singleton class while the
packet resolved one. A singleton could therefore have invocation signatures in one journey but
not the other; unrelated type closure could exhaust a comparison's hydration budget. Shared
`singleton_class` and `catalog_invocation` now live in
[`hydration.rs`](../../../crates/lctx-postgres/src/hydration.rs). Both
[`packet.rs`](../../../crates/lctx-postgres/src/packet.rs) and
[`journeys.rs`](../../../crates/lctx-postgres/src/journeys.rs) consume them; only full catalog
hydration adds configuration/type closure. **Correction source-inspected, 2026-09-29.**

<a id="F04"></a>
### PR5/F04 — Validly encoded wrong cache output must be a miss

**FP-04/05, DP-03/09/19, A2, G3/G6.** Hash/schema/row decoding alone admitted a semantically
wrong cache entry; final publication validation refused repeatedly instead of rebuilding that
disposable stage. `stage_cache.rs` (`f6562d3^:crates/cpg-core/src/stage_cache.rs`, recover through Git) now compares
snapshot-neutral canonical table batches against the existing pure derivation before admitting
cached results. Canonical comparison avoids scan-order-dependent false misses. A mismatch uses
the computed expected result and replaces the cache; final Delta validation remains independent
of cache admission. `tests/rebuild.rs` (`f6562d3^:crates/cpg-core/tests/rebuild.rs`, recover through Git) includes both
malformed-byte and valid-Arrow/wrong-value controls. **Correction source-inspected, 2026-09-29;
test outcome not claimed.**

FP-01–FP-06 are **satisfied for the selected bounded scenarios** after these corrections.
DP-01–05/07–09/11/15/18–24 and CI-01–04/08–11/13 have identified enforcement/ownership paths;
DP-06/10/14/16/17 are satisfied with the conservative cache-cost limitation in §4. No new graph
algorithm, recursion semantics or analytic model is introduced, so requalification of unrelated
CI-05–07 kernels is outside this review. Evaluation execution under CI-12 remains excluded.

## 8. Library fit and total complexity

| Capability | Existing mechanism and fit | Decision and burden |
|---|---|---|
| Public schemas and transport | Existing Serde/Schemars dispatch plus FastMCP's Tool/ToolResult and SDK result serialization | Keep thin adapters; no duplicate Pydantic semantic models or custom JSON-schema interpreter. |
| Scoped lookup and counts | SQLx checked queries, canonical domain rows and sets keyed by public member identity | Keep exact catalog ownership; no graph traversal or path-name heuristic is needed. |
| Evidence ranking | Existing BM25/NumPy lexical implementation, PG exact vectors and schema-owned finite fusion | Correct the consumer-specific lexical policy; no second retrieval service or generative relevance judge. |
| Paging and resource control | Typed finite sections, existing SQL leases/CPU admission, canonical serialization size | Domain continuation belongs here; a generic pagination framework would obscure the few distinct completeness rules. |
| Rebuilds | Existing Arrow contracts, pure transformations, Delta publication and immutable retrieval artifacts | A finite stage cache is sufficient. Salsa persistence remains conditional on a demonstrated finer-reuse workload. |

This review adds no new external-library API claim or dependency pin. It assesses the inspected
uses of already adopted mechanisms; it does not certify their complete upstream capability sets.

## 9. Alternatives and tradeoffs

| Alternative | Change/local reasoning and semantic consequences | Decision/revisit |
|---|---|---|
| Hydrate everything then trim | Optional behavior can prevent access to a small invocation contract; trimming required variants is unsound. | Rejected; current core/sections separate admission and optional work. |
| Additional operation-section tool | Adds another public route without adding a distinct semantic capability. | Same typed `get_operation` route is sufficient. Revisit only for a real transport limitation. |
| Full catalog packet per comparison candidate | Loads unrelated closure and duplicates resolution knowledge. | Replaced by shared invocation loading; witnesses remain on classifier results. |
| Cache hash checks only | Fast admission, but validly encoded wrong output repeatedly fails publication. | Replaced by semantic admission. No normalization speed benefit is claimed. |
| Whole-pipeline retry after cache failure | Requires careful abandoned-attempt handling and can repeat effects after invalid cached input reached consumers. | Stage-local admission is simpler for this design phase. |
| Generic incremental engine | Adds lifecycle, dependency tracking and persistence semantics beyond current consumers. | Deferred; coarse reuse plus clean equality is the baseline. |

## 10. Verification and uncertainty

| Claim | Evidence label/date | Inspection or command boundary | Outcome here |
|---|---|---|---|
| F01–F04 correction routes exist | **Implemented, 2026-09-29** | Source paths cited in §7, reinspected after owner corrections | Source-inspected; no test result inferred. |
| Core/section/schema/native/MCP composition exists | **Implemented, 2026-09-29** | Wire dispatch, packet/hydration, native storage and SchemaTool | Final transport and large real-library controls remain qualification. |
| Pure unit fusion has independent arithmetic and duplicate controls | **Implemented, 2026-09-29** | `cpg-schema/src/retrieval.rs` unit tests | `cargo test --release -p cpg-schema` **not_run by this reviewer**. |
| Clean/reused canonical equality controls exist | **Implemented, 2026-09-29** | `cpg-core/tests/rebuild.rs`, every canonical column after envelope neutralization | `cargo test --release -p cpg-core --test rebuild` **not_run by this reviewer**. Membership/absence/coordinates/provider/policy matrix remains broader than this test. |
| Real PG/MCP journey controls exist | **Implemented, 2026-09-29** | `python/lctx_mcp/tests/test_journeys.py`, affected existing tests | `uv run pytest python/lctx_mcp/tests/test_journeys.py` **not_run by this reviewer**. A fake embedder does not qualify the real embedding route. |
| Full code and docs gate | No new evidence from this review | `just fmt`; `just test-all`; `just docs-check` | **not_run by this reviewer**; implementation-owner final gate pending at review cutoff. |
| Both current profiles, restore and current-only cutover | Accepted implementation route | PR5.7–PR5.8 in the forward plan | **not_run by this reviewer**; no new ready/selected state is asserted. |
| Accuracy, comparative quality, performance | Outside scope | No assessment/benchmark command run | **not_run**. PR6 and conditional engine-adoption triggers retain these obligations. |

No percentages, release certificate or measured speed claim follow from this review. The known-answer
cases that matter are sparse/unassociated evidence, aliases and unknown ownership, contradictory and
unresolved comparison, indivisible/oversized optional records, semantically corrupt caches, and
evidence/provenance-preserving rebuild changes.

## 11. Authority changes and dispositions

| Required action | Owner and route | Source findings / boundary |
|---|---|---|
| Carry inspected fixes and their focused qualification into the current finding disposition | Implementation owner, forward-plan §6.2 | PR5/F01–F04; no additional ADR is required for these repairs inside ADR-0081. |
| Complete the authorized integrated qualification and publication work | Existing PR5.7–PR5.8 owners | Full code gate, docs, both-profile MCP journeys, clean/rebuilt mutation controls, import/reconstruction and quiesced current-only cutover. |
| Preserve the evidence boundary in governing prose/handoff | Existing product owner and handoff route | Accepted design, Implemented code and Tested runtime remain distinct; PR6 comparison is still open. |
| Revisit cache validation cost or conservative invalidation only for a real workload | Catalog/rebuild owner; existing §14.11 conditional adoption route | No additional process gate, cache authority or performance project is introduced now. |

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and limits |
|---|---|---|
| A1 Localize change | **satisfied, bounded Implemented** | Shared invocation loading removes comparison's private packet knowledge; evidence policy is explicit per consumer; response-only edits and association implementation have identified owners. |
| A2 Encode meaning structurally | **satisfied, bounded Implemented** | Tagged requests/states, canonical ownership, exact subjects, complete key inputs, canonical cache equality and explicit publication/selection boundaries. |
| A3 Extend through composition | **satisfied, bounded Implemented** | Browse/vocabulary, evidence/original expansion, existing classification and compact invocation facts compose without another inference owner or framework. |

**Bounded decision: Accept scoped at Implemented / source-inspected strength.** The assembled
routes satisfy the selected ownership and fidelity scenarios after F01–F04 correction. This does
not close PR5 testing, deployment, setup/deployment journey qualification, or the full rebuild
change matrix. **Enclosing product:** architecture accepted for these bounded routes; integrated
qualification and comparative value remain unresolved in the forward plan. The next work belongs
to the existing implementation/qualification owners, not a new architecture mechanism.
