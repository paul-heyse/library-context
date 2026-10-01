# Phase 4 assembled design review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Subject | Assembled Phase 4 implementation and X0 retirement at `72ba041389b976bba5b19074f589e340311bd9c1` |
| Standard | Core 3.2, code-intelligence profile 1.3, library-context binding |
| Tier · purpose | Design · Target |
| Reviewer · date | Independent design reviewer · 2026-10-01 |
| Decision | **Accept scoped** |
| Evidence strength | **Implemented**, independently inspected; named **Tested** focused receipts, dated 2026-10-01. No **Measured** claim |
| Supported scope | Cumulative typed Analysis and Catalog compilation in both profiles; nominal analysis ownership; vocabulary epochs; finite behavioral proofs and explicit residuals; catalog, selection, synthesis and retrieval preparation; optional analytic/retrieval embedding consumption |
| Exclusions | Phase 5 serving and PR6; full-library upper-frontier pilots; live embedding; product comparison, retrieval quality, performance, hydration and total-RSS measurements |
| Qualification limit | `just test-all` and `just hygiene` are **not_run** at this review baseline |

The architecture is adequate for the implemented finite contract envelope. Supported behavior includes explicit `Unknown`, partial, unavailable and not-requested outcomes; acceptance does not convert these into semantic completion.

The review follows the [detailed Phase 4 plan](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md), particularly §§1–2, 12 and 13, the parent finding disposition, DESIGN §15, ADR-0105/0106/0108 and the accepted ADR-0111 supplement. Scoped prior reviews were leads and historical evidence, not enclosing acceptance.

I inspected decisive declarations, executing producers, shared validators and adjacent consumers directly. Investigation concentrated on disputed ownership, graph lifetime, closure, symbolic fidelity, original-source synthesis, embedding reuse and retirement boundaries. Static investigation was sufficient; no additional probe or standing whole-pipeline trace was necessary.

The final tree retains only the unrelated dirty `.claude/agents/implementer.md` and `docs/library-utilization.jsonl`. Neither was changed or used as authoritative evidence by this reviewer.

## 2. Responsibilities, dependencies and semantic ownership

All source conclusions below are **Implemented**, independently inspected on 2026-10-01.

| Component | Responsibility and contract | Dependency and change boundary |
|---|---|---|
| `lctx-model::domain` | Owns semantic identities, operations, nominal records, expected coverage, condition/proof algebra, selection and replay contracts | Changes for domain meaning; storage and orchestration consume its definitions |
| `cpg-extract`, `cpg-flow` | Capture actual pinned native evidence and its qualifications; admit normalized source relationships | Change for provider mapping or newly supported native phenomena |
| `lctx-postgres::generations` | Owns attempts, immutable stage receipts, vocabulary-prefix visibility, validation and publication | Changes for persistence/lifecycle mechanisms |
| `cpg-core::PreparedCompilation` | Prepares configuration and schedule, coordinates actual producers, retains graph owners and publishes cumulative frontiers | Changes for execution orchestration; does not define a parallel semantic model |
| Model analytics kernels and `lctx-analytics` routes | Compute over declared admitted projections and method settings | Change for an algorithm or its explicit contract |
| Synthesis/retrieval producers | Consume model-owned assertions, originals, coverage and embedding winners | Change for rendering or prepared retrieval composition, without promoting evidence |
| Retained Phase 5 services | Cache, operation, serving and transport contracts awaiting typed serving integration | Outside active Phase 4 semantic authority |

The model contains consequential distinctions and governing operations, rather than merely domain-named output tables:

| Phenomenon | Authority and consequential distinction | Realization and consumers |
|---|---|---|
| Analysis invocation and capability coverage | Nominal producing-owner families; expected scope derived from captured inputs | `analysis/family.rs`, `expected.rs`, `frontier.rs`; scheduled producers and publication |
| Entry value and condition transfer | Actual native read/formal origin, complete scope, exact-one reaching admission; identity is distinct from heap state | `conditions/entry.rs`, `kernel.rs`, `substitution.rs`; Local, Model and Summary |
| Finite Summary result | Proof occurrence, rank/depth/cost and earlier premises; finite proof distinct from residual and complete inventory | `execution/summary_proof.rs`, `summary_worklist.rs`, `summary_production.rs` |
| Source field association | Exact admitted store/reader relationship; source association distinct from runtime value | `normalized/symbolic_fields.rs`, `catalog/evidence/symbolic.rs`, `summary_symbolic.rs` |
| Catalog and selection | Source/effective contracts, total default states, contextual domains, quantifiers and four outcome groups | `catalog/build.rs`, `selection/vocabulary.rs`, `evaluate.rs`, `algebra.rs` |
| Authored prose and code | Original bytes/coordinates, interpretation status and source qualification | `synthesis/documentary.rs`, assertion builders and retrieval preparation |
| Embedding consumption | One complete spec, exact input/codec and immutable winning bytes; independently owned consumer uses | `embedding/consumption.rs`, realization session, cache and nominal consumer validators |

The active-source search covered `cpg-core/src`, `lctx-analytics/src`, `cpg-extract/src` and `cpg-flow/src` for `cpg_schema` and legacy model imports, followed by module-export and manifest inspection. The remaining core match is the uncompiled Phase 5 recovery `bundle.rs`.

This does **not** establish that legacy code is absent from the compiled dependency graph. Core still depends transitively on `cpg-schema` through named PostgreSQL Phase 5 services; `lctx-model` retains public `decl`, `id` and `legacy` machinery for that consumer. Their presence does not supply active Phase 4 semantics. No additional feature gate is necessary to establish the inspected ownership boundary.

## 3. Contracts, constraints and testing boundaries

| Contract | Enforcement and lifecycle | Verification boundary |
|---|---|---|
| Vocabulary epoch | `generations/vocabulary.rs::close` locks the attempt and ordered deltas, verifies sealed content, rejects changed prior payloads and unavailable references, validates the candidate prefix and closes atomically | Real PostgreSQL controls; shared model validators |
| Validation input | `(relation, prefix)` remains explicit through stage input, publication, group close and final validation | Earlier Facts-bound normalization cannot silently observe later vocabulary |
| Entry admission | Private derived witness requires actual owner/read/formal identity, complete native coverage and exact admitted reaching inventory | Native adversarial controls and scheduled publication |
| Summary publication | Earlier premise references, proof equations, derivation acyclicity, complete member inventory and coverage are replayed | Intact/permuted publication and independently corrupted twins |
| Graph borrowing | Collection owns hydrated snapshots and reservations; borrower requires matching completed sources and budget; generative brands prevent graph-index interchange | Source contracts and retained compile-fail controls; full doc-test execution remains Q0 work |
| Embedding realization | Declared effect and stored selection admit service access; full spec/input identity and codec govern cache lookup and winning bytes | Pure replay plus focused cold/warm/cleared effect/cache controls |
| Synthesis | Private programmatic assertion construction and replay require supported status, exact originals and same-generation premises | Actual documentary and negative S0 publication controls |

Invalid construction is prevented where practical; persisted and provider-derived states are revalidated at their effect boundary. Repeated producer/publication checks use shared definitions rather than separate semantic interpreters.

Absent, unresolved, unrequested, complete-empty and partial outcomes remain distinguishable. Resource exhaustion and unsupported transfers produce named limits or boundaries; they do not silently truncate a complete result.

## 4. Composition and execution

`PreparedCompilation` in `crates/cpg-core/src/compilation.rs` owns one fresh cumulative attempt, ordered publication groups and the appropriate Analysis or Catalog checkpoint. It does not select the resulting generation. Mandatory catalog construction is independent of brief seeds, optional analytics and vectors.

`PreparedGraphs` in `analysis_graphs.rs` hydrates selected stored snapshots once within collection-owned preparation and retains their reservations. Later stages borrow matching sources. `projection/snapshot.rs` preserves the native graph representation behind a branded view; canonical normalized records remain the semantic authority.

| Analysis question | Projection and semantics | Settings, bounds and linkage |
|---|---|---|
| Invocation recursion | Declared native invocation universe; directed relationships retain identity and unresolved/open remainder | Petgraph SCC decomposition; canonical callee-first scheduling; separate finite worklist and proof policy |
| Structural reachability and usage | Declared vertices/arcs, including isolates, parallel paths and exact source-definition relationships | Selected output does not redefine the universe; output links to admitted projection inputs |
| Weighted ranking | Explicit vertex universe, directed pairs and weights | Versioned parameters, bounded work/allocation, iterations, residual and stop status; heuristic result stays qualified |
| Communities | Explicit graph construction and weight interpretation | Seeded sequential resolution/seed runs, canonical labels and iteration/quality records |
| FCA/RCA | Declared object/attribute/incidence context; concept semantics remain exact over that context | Enumeration/support/basis bounds and explicit availability; source context and settings retained |
| Vector neighbourhoods | Selected vector layer and exact consumed winners | Spec/codec/width and finite-value validation; layer-only mode does not imply ordinary public neighbours |

Petgraph SCC discovery is not treated as a completeness proof. Summary proof identity is not conflated with semantic worklist identity: bounded nondominated representatives preserve meaningful equal-cost evidence, and capped siblings retain residual uncertainty.

The derivation graph uses relation-qualified row references and generated nominal premise declarations. Its acyclicity check prevents a proof family from certifying itself through an aggregate or future premise.

Reservations cover retained graph preparation, kernels, validation and results. This supports the implemented allowance contract; it is not a measured total-RSS guarantee.

## 5. Change and failure scenarios

These are interface/source assessments, not measured change-cost experiments.

| Scenario | Ownership and expected propagation | Assessment |
|---|---|---|
| Add an authored model using existing channels | One model declaration and actual new domain behavior; existing native requirements, configuration digests, applicability and validators consume it | Satisfied. `domain/models.rs` and `models/requirements.rs` govern preparation; no separate core/Python model classifier is needed |
| Add a selection predicate | Predicate owner declares dependencies, witnesses and closure semantics; interpreter and algebra realize it; consumers receive typed outcomes | Satisfied. Query/render consumers need no independent classification rule |
| Add an analysis over existing facts | Declare inputs, nominal invocation/outcomes, parameters and kernel; derive store/receipt contracts | Satisfied. A new semantic analysis may legitimately add its own owner; extractor or serving-schema edits are not inherently required |
| Replace graph/numeric implementation | Preserve universe, direction, multiplicity, weights, precision, diagnostics, limits and source linkage | Satisfied. Existing native view and admitted kernel inputs give a credible substitution boundary; independent numerical/structural controls remain necessary |
| Add source association or rendering | Association owner proves typed basis against originals; renderer consumes qualification | Satisfied. A renderer cannot make runtime knowledge from source association |
| Recollect changed inputs/model | Fresh capture and derivation, coherent attempt publication, old readers retained on their leases | Satisfied. No incremental repair or legacy-format fallback is required |
| False-only native read with competing states | Preserve the entire eligible native reaching set; Entry independently refuses multiple/unbound origins | Satisfied under ADR-0111; no preferred formal is selected |
| Interrupted or uncertain publication | Atomic close/abort and explicit unconfirmed outcome; no speculative output becomes committed authority | Satisfied by inspected store lifecycle and existing focused controls |

A genuinely new semantic channel or heap-state model would require new domain operations and evidence contracts. That is legitimate domain extension, not an unresolved promise that the existing instance-extension route supports arbitrary semantics.

## 6. Correctness and fidelity gates

These verdicts judge the supported architectural/source contracts. They do not certify the pending complete qualification gate.

| Gate | Verdict | Independent evidence and scope |
|---|---|---|
| G1 Authority | **pass** | Nominal model owners, normalized policy/admission, immutable vocabularies and post-X0 active imports; retained Phase 5 compilation is a named separate consumer |
| G2 Semantic fidelity | **pass** | Typed defaults, qualification, identity layers, graph multiplicity and source/runtime knowledge remain distinct |
| G3 Validity | **pass** | Shared candidate-prefix, entry, proof, derivation, selection, source and embedding replay; private admitted constructors |
| G4 Hidden behavior | **pass** | Effects belong to declared stage/session contracts; pure selection/replay introduces no service fallback; pinned configuration enters preparation |
| G5 Consistency and recovery | **pass** | Attempt-owned publication, immutable epochs, finite residuals, resource admission and explicit outcomes |
| G6 Transformation and reuse | **pass** | Source-bound graph borrowing, simultaneous bounded substitution, exact originals and complete embedding-cache identities |
| G7 Truthful capability claims | **pass** | Supported/unknown/partial/not-requested boundaries are explicit; focused receipts and exclusions are separately labelled |
| G8 Library leverage | **pass** | Existing libraries own generic BDD, SCC, partitioning and persistence mechanisms; retained bespoke kernels have concrete semantic gaps in available alternatives |
| CI-G1 Fidelity | **pass** | Source association remains runtime `Unknown`; heuristics remain governed; native qualification is not relabelled |
| CI-G2 Evidence closure | **pass, scoped** | Compiled assertions, briefs and retrieval preparation close over exact generation evidence and coverage. Phase 5 served transport is not assessed |
| CI-G3 Evaluation integrity | **pass, scoped** | Inspected compilation inputs do not include gold/heldout answers; independent mutation/oracle controls challenge contracts. Product-quality evaluation is excluded |

## 7. Findings and applicability

**No new actionable P4Q-Fnn finding was identified.** Architectural acceptance does not depend on assuming future gate success.

| Principles | Verdict | Reason |
|---|---|---|
| FP-01, FP-02, FP-06; DP-05/08/10/17/18/23/24 | **satisfied** | Coherent model/provider/store/orchestration boundaries support bounded local reasoning and the examined changes |
| FP-03; DP-06/13/14/16 | **satisfied** | Shared primitives compose through declared inputs; library mechanisms are reused where they fit |
| FP-04; DP-01–05/08/09 | **satisfied** | Domain operations govern behavior, including admission, coverage, proof, selection and consumption; active legacy semantic authority is retired |
| FP-05; DP-02/03/07/11/12/19/20 | **satisfied** | Identity, constraints, finite recursion, publication and allowances are explicit |
| DP-15/21/22; CI-01–10 | **satisfied in scope** | Pinned boundaries, qualified results, reproducible linkage and explicit capability limits |
| CI-11/12 | **satisfied in scope** | Prepared claim closure and non-circular controls; no serving/product-quality inference |
| CI-13 | **not assessed for Phase 5 serving** | Serving is unavailable and excluded. Prepared Phase 4 projections and preservation obligations are assessed above |

The symbolic correction supporting [P4B3-F01](design_review_phase4-behavior_2026-10-01.md#P4B3-F01) was independently reinspected. The normalized owner records exact supported store/reader associations. C1 preserves source association `Known` with runtime value `Unknown`; Summary retains original per-reader Flow qualification, no finite transfer proof and explicit unknown alternatives. Synthesis rejects attempts to launder these into finite behavioral evidence.

This supports scoped closure of that correction. It does not establish general plain-class tracking, heap temporal identity or runtime field values. The parent plan must retain those boundaries when recording disposition for semantic-data-model F06 and related obligations.

## 8. Library fit and total complexity

| Capability | Inspected fit and gap | Judgment |
|---|---|---|
| BDD conditions | Pinned `biodivine-lib-bdd` supplies diagram operations; model-owned atom identity, bounded admission, substitution and qualifications supply domain policy | Appropriate composition; no new generic solver is needed |
| SCC and derivation DAG | Petgraph supplies decomposition/topological operations; canonical scheduling and premise contracts remain domain-owned | Built-in mechanisms reused |
| Weighted ranking | Pinned petgraph PageRank uses its own unweighted degree/count and fixed-iteration contract; it does not supply the required weighted residual/stop record | Small bounded weighted kernel justified |
| Communities | `leiden-rs` graph builder and optimizer are used behind explicit settings and recorded outcomes | Appropriate library boundary |
| FCA/RCA | Bounded context, retained allowances, basis/output identity and replay requirements govern the production kernel; independent `fcars` controls challenge concept sets | Bespoke bounded contract justified; oracle retained |
| Store and validation | PostgreSQL/SQLx and DataFusion perform their persistence/relational roles; typed vocabulary lifecycle is domain-specific | No second store or generic application framework needed |
| Embedding | Existing service/cache mechanisms consume the sole spec and codec; consumer ownership and byte consistency are domain contracts | Thin effect boundary with meaningful shared replay |

The retirement initially removed the `leiden-rs` dependency needed by the independent ranking oracle. The final manifest restores it as a dev-dependency; the oracle is preserved and the final check passes. Removing competing production authority did not require removing this independent control.

No library adoption or bespoke replacement is warranted merely to reduce visible file count. The relevant total complexity includes bounds, lifecycle, qualification and replay.

## 9. Alternatives and tradeoffs

| Alternative | Assessment and revisit condition |
|---|---|
| Restore legacy analysis or add compatibility adapters | Rejected. It recreates independently mutable semantics and old identities without a current Phase 4 consumer |
| Force-remove every compiled legacy dependency now | Unnecessary for the active Phase 4 boundary; it would consume named Phase 5 services/recovery contracts. Retire them at their actual replacement boundary |
| Introduce a generic tagged analysis registry or universal proof engine | More machinery and weaker nominal authority than the finite owner families. Revisit only for a demonstrated extension need |
| Replace weighted/bounded kernels with superficially similar library routines | Not equivalent where weights, residuals, basis semantics, limits or evidence differ. Substitute when the complete contract can be preserved |
| Discard false candidates or choose their preferred formal origin | The first loses exact native negative evidence; the second invents origin authority. ADR-0111’s whole-set retention plus unchanged Entry admission is the least sufficient correction |
| Promote source fields into runtime state | Unsupported by source association alone. Requires a separately owned heap/time/value model and discriminating evidence |
| Preserve all obsolete tests and table snapshots | Retains obsolete mechanics and competing ownership. Preserve independent meaning in typed controls and named Phase 5 recovery sources instead |

## 10. Verification and uncertainty

I read the [qualification receipt and linked logs](../evidence/2026-10-01_phase4-qualification/README.md). Commands below were run by the coordinator/implementers, not this reviewer. Cargo commands use `python3 scripts/build_environment.py --`.

| Claim · date | Command/evidence | Outcome and limit |
|---|---|---|
| Final post-X0 compilation · 2026-10-01 | `cargo check -p cpg-core -p lctx-analytics -p lctx --tests`; `raw/phase4-x0-final-check2.log` | **passed**, 41.30s, at the final production baseline |
| Retirement initial check | Same check; `raw/phase4-x0-final-check1.log` | **failed**, missing Leiden oracle dependency; repaired by retaining the dev-dependency |
| Cumulative CLI/profile/cache controls · 2026-10-01 | Functional5 command in receipt, including `--test compile_facts` | Named CLI target **passed**, 3 controls across both profiles, including seedless upper frontiers, nonzero briefs and cold/warm/cleared fake embedding cache. Enclosing command **failed** the separate negative fixture |
| Summary native and actual PostgreSQL replay · 2026-10-01 | Functional4 command, including `--test symbolic_composition --test summary_publication` | Named native2 and Summary PG2 **passed**. Enclosing failures remain recorded |
| C1 strengthened actual PostgreSQL controls · 2026-10-01 | Functional5/6 commands including `--test catalog_selection` | Named controls2 **passed**, including source-link deletion, runtime-Known forgery and sibling-parameter retargeting |
| Negative S0/native correction · 2026-10-01 | `cargo test --release --no-fail-fast -p cpg-flow -p cpg-core --test flow_shapes --test synthesis_refutation -- --nocapture`; native2 log | Actual PostgreSQL S0 control1 **passed**, 86.83s. Exact Summary refutation, coverage member, kind18 finding, Limits assertion and selected brief; missing-proof/Unknown/erased-membership/Partial refusal twins. Enclosing command **failed** a stale discarded-count assertion |
| Corrected native controls · 2026-10-01 | `cargo test --release -p cpg-flow --test flow_shapes -- --nocapture`; false-origin2 log | **passed**, 33 controls. Actual false formal origin, Defined/Unbound competitors, live-read pruning and computed-false refusal |
| Complete functional gate | `just test-all` | **not_run** |
| Complete non-functional gate | `just hygiene` | **not_run** |
| Excluded measurements/pilots | Live embedding, upper real-library pilots, product comparisons, performance/hydration/total-RSS | **not_run** |

The negative result is a refutation of the exact qualified finite alternative under its model. It is not a theorem that the parameter never reaches any return.

Earlier failed recursive and native-origin fixtures are retained in the receipt. They exposed unsupported guard correlation and missing native origin rather than justifying fabricated evidence. Their passing replacements are a **composite focused receipt**, not an initially clean enclosing run.

Fake embedding controls qualify declared effects, cache identity and byte replay. They are not a mocked semantic provider, live-service qualification or retrieval-quality proof. No measurement is inferred from elapsed test times.

## 11. Authority changes and dispositions

| Matter | Owner and disposition |
|---|---|
| Existing vocabulary, nominal owner and typed frontier decisions | ADR-0105/0106/0108 remain the architectural authorities; inspected implementation conforms within this envelope |
| Checked-false native policy | Accepted ADR-0111 supplements ADR-0045. No new semantic type, guard identity, coverage upgrade or solver is introduced |
| Symbolic correction and cross-phase findings | [Parent cutover §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition) owns current disposition; this review supplies independent scoped evidence |
| X0 deletion and preservation | Phase 4 §11 and committed expectation maps own migration. Dormant README names preserved bundle, transport, byte/page, native/Python, cursor and hydration obligations |
| Qualification/status reconciliation | Coordinator owns detailed plan, parent disposition and STATUS updates against this review and the final receipts |
| Future serving retirement | Phase 5 must re-express supplied typed-generation contracts and remove old consumers at replacement; dormant source is neither active authority nor a passing receipt |

The inspected X0 partition removes active obsolete production owners and obsolete table-form controls while preserving independent Phase 5 expectations. The preserved bundle source remains uncompiled. The recovery map and source do not claim Phase 5 execution.

Revisit this acceptance if supported behavior expands to general runtime field state, complete whole-callable origin/sink absence, computed/call-crossing/loop-header false-source retention, live embedding guarantees or served transport.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | **satisfied** | Model instances, predicates and kernel substitutions have coherent owners; storage and rendering consume narrow typed contracts; testing dependencies are bounded |
| A2 Encode domain meaning explicitly | **satisfied** | Governing operations own admission, expected coverage, proof/residual closure, source association, selection and embedding consumption. Supported unknown states remain explicit |
| A3 Extend through composition | **satisfied** | Existing channels, graph views, nominal stage families and pure operations compose without an additional semantic classifier or compatibility authority |

**Bounded decision: Accept scoped.** No unresolved architectural or fidelity gap was identified in the implemented finite Phase 4 contracts and the examined extension/substitution scenarios.

**Enclosing architecture:** accepted for the named Phase 4 Analysis/Catalog scope at **Implemented** and named focused **Tested** evidence strength. Complete release qualification remains pending. Phase 5 serving, PR6, product quality and measurements are not assessed or accepted here.

The coordinator’s next step is Q0: run `just test-all` and `just hygiene` against the assembled tree, repair failures with accurately bounded composite receipts, then reconcile the current plan, finding dispositions and STATUS.
