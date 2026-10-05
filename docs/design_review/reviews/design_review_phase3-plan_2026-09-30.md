# Phase 3 normalized-relations plan: design review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Subject | [Phase 3 detailed plan](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md), proposed ADR-0101/0102 and amended DESIGN §15, against source baseline `e4ab3ea` and the current documentation worktree |
| Standard | Core/template 3.2, code-intelligence profile 1.3, library-context binding; loaded through `standard.toml` |
| Tier · purpose | design · target; broad reconsideration of P0–P2 was permitted |
| Reviewer · date | Fresh independent `design-reviewer` agent, 2026-09-30 |
| Maturity | Proposed implementation architecture; source and selected pinned interfaces independently inspected |
| Outcome sought | An executable design for cumulative normalized generations and reusable normalized semantic contracts, with explicit P4/P5 handoffs |
| Supported scope | Entity correspondence, source/effective callable distinction, total resolutions, complete call events, policy memberships, stored bindings, projection inputs and their runtime/store prerequisites |
| Exclusions | P3 production implementation; P4 transfer/summary/catalog algorithms; P5 serving; product or performance qualification |
| Method | Read the governing owners, actual model/binder/runtime/store boundaries, the provider-fork cancellation boundary, library contracts and the author's bounded test receipt. No product gate or new probe was run by this reviewer. |

The reviewed workloads include reusable feature-discovery evidence, identifying invocation
alternatives, exposing unsupported decorators without losing source contracts, and preparing
safe inputs for later behavioral and catalog consumers. P3 supplies evidence and explicit
uncertainty for these workloads; it does not claim completed answers or restored serving.

The review found two gaps in binding admission. Both were corrected in the proposal and
reinspected on 2026-09-30: F01 establishes effective/body authority and total variant applicability;
F02 establishes normalized target/signature applicability without relabeling raw evidence.
The final decision is **Accept scoped**, at Proposed strength. Concurrent AGENTS, agent-definition
and settings edits were not changed.

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility and hidden decisions | Contract/direction | Expected reason for change |
|---|---|---|---|
| `lctx-model::domain` | Meaning of identities, correspondence, availability, effective assessment, policies, applicability and binding | Pure typed operations; generated representation contracts; mechanisms depend on this owner | A newly supported semantic distinction or policy |
| `cpg-core::normalize` | Compose declared operations and DataFusion joins over completed inputs | Does not invent correspondences, descriptor semantics or policy predicates | New derivation using existing contracts |
| `domain::stages` and attempt runtime | Dependency/transport declarations, capabilities, memory and admitted execution | One schedule; store-backed input does not imply retained-batch input | Execution mechanism or resource policy |
| `lctx-postgres` | Freeze/receipt/read/publication protocol and role authorization | Lowers validated model frontiers; private staging reads remain separate from published readers | Store or lifecycle mechanism |
| Provider fork | Bounded transport and terminal close/drain behavior | Does not own semantic generation authorization | Driver/protocol repair |
| `lctx-analytics` | Projection adaptation and private dense indices | Typed universe/arcs/evidence in; no store or new semantic classifier | Graph consumer or representation |
| P4/P5 consumers | Analyses and serving over admitted normalized contracts | Do not reinterpret policy memberships or manufacture composition admission | Their own algorithm, model or wire contract |

The proposed dependency direction is coherent. Ordinary modules and functions are sufficient;
neither a new framework nor a universal semantic language is justified. Store representations,
policy views and graph layouts derive from semantic owners.

### Fact and fidelity table

| Family | Provider/revision and fidelity | Coverage/unknowns | Identity and consumer |
|---|---|---|---|
| Raw declarations/signatures | Existing pinned producer, attributed L0 assertions; native signatures are undecorated | Missing declaration and unavailable signature stay explicit | `ProviderSymbol`, `Signature`, occurrence links; normalized entity/surface owners |
| Entities/correspondence | P3 model revision; derived from explicit declaration evidence | Ambiguous and unresolved outcomes preserve candidate premises | Declaration occurrence and kind, or qualified external/synthetic identity; all later consumers |
| Effective callables | P3 model operation; bounded supported descriptor semantics | Arbitrary transformations remain unknown while source variants survive | Source callable/context/decorator chain; binding/catalog consumers |
| Calls/policies | Existing attributed targets plus normalized P3 assessment | Open remainders and phase/channel alternatives precede policy selection | Event occurrence/origin/context, relationship identity; named views and P4 |
| Bindings | One model binder; derived shape result with signature authority | Refusal and undecidable variant applicability survive | Event/alternative/variant and complete premise set; private composition admission |
| Projection inputs | Model-declared role projection; exact over that declared input | Scoped availability plus unresolved side records; no runtime-completeness claim | Canonical entity/arc IDs with evidence; petgraph indices remain private |

Source inspection confirms that [declaration links](../../../crates/lctx-model/src/domain/declarations.rs)
already carry exact occurrences, while [provider symbols](../../../crates/lctx-model/src/domain/calls.rs)
are qualified by provider and context. The plan correctly avoids converting names into equivalence.

## 3. Contracts, constraints and testing boundaries

| Contract | Preconditions and invariant owner | Effects/failure | Isolated verification |
|---|---|---|---|
| Symbol resolution | Complete applicable correspondence evidence, compatible entity kinds; model-owned total result | Pure; missing/conflicting evidence is an outcome | Frozen typed observations, independent expected entities and candidates |
| Effective assessment | Ordered decorators, authoritative lexical outcomes and agreeing native traits | Pure; source-known/effective-unknown is valid | Shadowed builtin, arbitrary wrapper and exact descriptor twins |
| Complete event/policy | All declared resolution members before selection; normalized phase-specific destinations | Pure assessments; incomplete input cannot mint complete admission | Cross-provider equivalence/disagreement, constructor sibling and open remainder cases |
| Binding/admission | Complete actuals, raw formal list, private `ApplicableSignature`, body authority and total variant assessment | Pure; unknown variant is not a proven mismatch | Source signature binds while arbitrary wrapper remains inadmissible; cross-provider applicability and variant omission/tamper controls |
| Completed relation | Sole stage writer; all outputs frozen with receipts before handle creation | PostgreSQL transaction; uncertain/failed completion is abort-only | Real PG late-write/freeze races and wrong-source controls |
| Stage query | Declared source-bound inputs and availability; complete physical scan demand admitted | Bounded read-only execution; refusal before scans if demand unknown/excessive | Multiple/repeated scans, insufficient capacity, retained table after close |
| Projection | Explicit universe, relation meaning, scope, multiplicity and evidence | Pure bounded construction; capacity refusal is explicit | Isolates, parallel arcs, cross-file cycles and selector/universe twins |

The strengthened F01 contract is material: a successful source-signature bind cannot prove an
effective body is called, and one successful variant plus an undetermined variant cannot prove
unique applicability. Preconditions belong in private model construction, not P4 conventions.

## 4. Composition and execution

| Stage/capability | Question and semantic input/output | Mechanism and composition | Projection/model, bounds and evidence |
|---|---|---|---|
| R1–R3 | Which completed data may this stage read? | Model frontier/schedule → atomic store completion → private source handle → admitted query | Exact relation/source identity; per-scope availability; shared memory/connection budgets |
| N1/N2 | What does this observation denote, and who owns it? | Existing owner operation and DataFusion joins produce total attributed outcomes | Exact only where correspondence establishes it; retain unmatched/conflicting sources; direct relations only |
| N3 | Which source and effective contracts are established? | Consume N2 lexical outcomes and complete native variants through one model operation | Supported descriptor model, source/effective distinction and explicit unknowns |
| N4/N5 | Which policy permits this relationship, and can a complete variant bind? | Whole-event assessment → policy membership → applicability/shape binding → private admission | Five distinct meanings; no Potential-to-invocation promotion; raw provenance and complete premises retained |
| N6 | What topology is available for a named question? | Role declarations → validated relational projection sources/assessment → bounded immutable directed petgraph on request | Separate projection meanings, parallel arcs/self-loops/isolates, canonical construction; no graph construction during compilation or persistent indices |
| N7 | Is this generation a valid normalized result? | Final coverage and shared cumulative validation → frontier admission → publication | L0+L1 closure; no intermediate publication, no automatic selection; later frontiers refuse |

The runtime plan addresses the inspected baseline defects rather than assuming new transport is
equivalent to existing handoffs. [Stage retention](../../../crates/lctx-model/src/domain/stages.rs)
currently follows every declared reader; [registration](https://github.com/paul-heyse/library-context/blob/e4ab3eae7d42fad95564dae317496bc03d67f14a/crates/cpg-core/src/model_runtime.rs)
accepts an arbitrary provider after schema/permit checks and returns an unrestricted DataFrame.
The proposal changes both boundaries explicitly.

Generation shared leases conflict with exclusive transitions under the actual
[lock contract](https://github.com/paul-heyse/library-context/blob/e4ab3eae7d42fad95564dae317496bc03d67f14a/crates/lctx-postgres/src/generations/locks.rs). Closing stage inputs before
freeze is therefore necessary. The revised plan also keeps reservations through cancellation/drain,
closes clone-shared pools terminally, and admits all remote scans before execution rather than
letting partially started joins compete for insufficient connections.

## 5. Change and failure scenarios

| Scenario/kind | Owning change and affected consumers | Locality/composition assessment | Settling evidence |
|---|---|---|---|
| Add a normalized family — domain extension | Model record, operation, invariants and schedule; generated store/codec inventories follow | No facts-specific store dispatch arm should be needed; genuinely new semantics remain explicit | R1 test frontier and N1–N7 declarations |
| Add a policy — policy instance | One model evaluator/category plus independent expected answers | Membership views and consumers reuse the result; no duplicated Rust/SQL predicate | N4 policy/view equivalence and independent truth table |
| Upgrade a provider — mechanism/binding | Producer absorbs private representation changes; semantic change alters qualified observations | Same declaration correspondence can be reused; agreement cannot erase disagreement or strengthen modality | Source/stub, repeated-support and conflicting-candidate controls |
| Bind an equivalent other-provider signature — composition | Private `ApplicableSignature` bridges established same-entity/input/context evidence into the sole shape algorithm | F02 correction removes raw-equality coupling without editing attributed source rows | Exact declaration equivalence positive; foreign entity/context or conflicting evidence negative |
| Arbitrary decorator, source bind succeeds — semantic boundary | Effective-assessment and composition admission owner | F01 correction blocks body execution claims while preserving useful source inspection | Wrapper-negative and known-effective positive controls |
| One successful plus one unsupported variant — partial evidence | Total binding-set assessment | Unknown does not become inapplicability; only proven mismatches may be excluded | NativeUnavailable/ParamSpec/unpacking twins |
| Replace completed-store read with an admitted handoff — mechanism substitution | Input adapter preserving source, availability, lifecycle and reservation contracts | Domain normalization does not learn table-provider internals | Same typed inputs/results plus source-capability refusal cases |
| Cancellation before `query_raw` returns — failure | Fork guard and stage pool lifecycle | Guard starts before await; no late use after close or uncharged drain | Existing baseline lacks this guarantee; R3 real-store control is required |
| Empty call result with missing family — incomplete input | Scoped normalization coverage | Complete computation remains distinct from complete evidence; absence cannot be inferred | Explicit no-events/partial/unavailable profile controls |
| Add P4 graph analytic — planned consumer | New analytic selects a declared projection and model | No new graph authority; selector cannot remove intermediate universe nodes | N6 independent known-answer shapes; algorithm qualification remains P4 |

## 6. Correctness and fidelity gates

Verdicts concern the **specified proposal**, not implemented P3 behavior.

| Gate | Verdict | Evidence or required action |
|---|---|---|
| G1 Authority | pass | Model-owned operations; stored memberships and generated views; no parallel semantic store |
| G2 Semantic fidelity | pass | F01/F02 corrections preserve source/effective distinctions, unknown variants and original raw evidence at normalized applicability |
| G3 Validity | pass | Private complete-event, applicability, binding and composition construction; exact receipts/source capabilities and shared validators |
| G4 Hidden behavior | pass | Effects stay at acquisition/store/query boundaries; no fixture execution/reparse or ambient spill |
| G5 Consistency and recovery | pass | Cumulative one-attempt publication, atomic freeze, terminal failures, drain before transition, explicit budgets |
| G6 Transformation and reuse | pass | F02 names the explicit normalized applicability transformation; raw premises survive; projection/coverage and reuse contracts are specified |
| G7 Truthful capability claims | pass | Proposed labels, scoped baseline tests, explicit unknown effective behavior, no measured benefit claimed |
| G8 Library leverage | pass | DataFusion relational work, existing typed codec/SQLx/COPY, petgraph topology; no unnecessary generic engine |
| CI-G1 Fidelity | pass | F01/F02 corrected: incomplete evidence, source-only binding and Potential association cannot become unsupported invocation/composition claims |
| CI-G2 Evidence closure | n.a. for served claims | Serving is excluded; cumulative references/evidence validation are specified inputs to P5, not qualification of a server |
| CI-G3 Evaluation integrity | pass | Gold/skills/sealed evaluation excluded from compilation and tuning; independent controls named before implementation |

## 7. Findings and applicability

<a id="F01"></a>

**F01 — composition admission omitted effective/body authority and total variant applicability.**
The first reviewed §4.3 permitted a token from Summary membership, owner, target and successful
raw binding, despite §3.2 allowing arbitrary wrappers to retain bindable source signatures with
unknown effective behavior. It also did not distinguish proven-incompatible from unassessed
variants. A consumer could instantiate a source body's summary or assert unique applicability
without those premises. This violates FP-04/FP-05, DP-02/DP-03/DP-08 and CI-02/CI-04 (A2,
G2/G3/G6, CI-G1).

The model's admission boundary is the correction owner. The revised §4.3 adds `BindingAuthority`,
exact effective-target/context/descriptor/body premises and total `BindingSetAssessment`, with
one Bound and all remaining variants ProvenIncompatible required for uniqueness. DESIGN §15.5
and ADR-0102 carry that meaning. Static reinspection establishes a **Proposed design correction**;
the wrapper and unknown-variant negative controls remain implementation obligations in N5/Q.
Current disposition belongs in [plan §11](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes).

<a id="F02"></a>

**F02 — normalized signature applicability was not connected to the existing raw binder contract.**
The first reviewed §4.3 said to reuse `calls::bind` on raw variants, but that function requires
`CallDestination::Resolved.symbol == Signature.symbol` in addition to context/qualification checks
([calls.rs](../../../crates/lctx-model/src/domain/calls.rs)). P3 intentionally reasons about
multiple provider symbols denoting one established normalized entity. Leaving this seam implicit
forces the implementer to choose between losing usable equivalent signatures and rewriting a raw
target to satisfy the binder, which would misattribute evidence. This is FP-02/FP-04,
DP-02/DP-08/DP-15 and CI-01/CI-02 (A2/A3, G2/G3/G6, CI-G1).

The revised §4.3 chooses a private, evidence-bearing `ApplicableSignature` operation and feeds its
validated same-entity/input/context target/signature relationship into the single shape algorithm.
Original raw IDs, supports and parameter declarations survive. Unproved applicability is
Undetermined; the old raw-equality API migrates and is removed without a compatibility adapter.
The same-provider-only alternative would forgo the established correspondence and was not selected.
DESIGN §15.5 and ADR-0102 agree. Static reinspection establishes a **Proposed design correction**;
N5/Q retain independent cross-provider positive and foreign-entity/context negative controls.
Current disposition belongs in
[plan §11](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes).

FP-01–FP-06 are satisfied at Proposed strength by the ownership, corrected contracts and scenarios
above. Related DP-01/04/05/07,
DP-09–14 and DP-16–24 are supported by the identified contracts, alternatives and qualification
obligations. CI-03/05/07/08/10/12 are satisfied within P3's proposed scope. CI-06/09/11/13 are
assessed only at their input/handoff boundary; P4 behavioral/heuristic results and P5 serving are
not accepted by this review. N3's dependency was corrected to N2 so exact builtin classification
consumes the authoritative lexical outcomes instead of inventing another resolution route.
The final draft also gives native-coordinate `TypeBinderAssessment` total outcomes and preserves
the P2 `FlowCallStep` call/operand proof in qualified path-to-event links. These avoid restoring
legacy source/stub preferences or confusing call linkage with operand-type or transfer evidence.

## 8. Library fit and total complexity

| Capability | Candidates and pinned fit | Judgment |
|---|---|---|
| Relational construction | DataFusion 55.1.0/Arrow 59.3.0 versus hand maps or moving semantics into PostgreSQL SQL | Existing query engine is appropriate for joins and grouping; domain algorithms remain ordinary Rust. Streaming avoids terminal retention but does not bound blocking state. Explicit shared reservations and no-spill refusal are correctly separate. |
| Persistence/transfer | SQLx 0.9, SeaQuery 1.0.2, pgpq 0.12 and existing typed codec versus ORM/new driver | Reuse existing transactions/DDL/COPY boundary. Provider-internal Rust-Postgres stays isolated. New lifecycle guarantees belong to the fork and store and require focused acceptance. |
| Topology | petgraph 0.8.3 Graph, StableGraph, GraphMap and CSR | Immutable directed Graph preserves parallel relationship IDs and isolates with existing trait coverage. Private indices remove any reason to choose StableGraph solely for semantic identity. |
| Policy rendering | Typed evaluator plus stored admissions, duplicate SQL/Rust predicates, generic predicate language | Stored finite admissions are the smallest design that keeps one meaning and queryable views. |
| Reuse/storage | All-memory handoffs, completed PostgreSQL reads, external generation links, IPC spool | Completed reads introduce permissions/receipts but reuse the store and preserve independent retirement; alternative reuse is deferred to a named measured need. |
| Reasoning engine | Existing bounded operations versus Ascent/datafrog/additional incremental framework | No P3 fixed-point consumer warrants new generic machinery. P4 must select from its actual analysis semantics. |

The resolved dependency inventory and selected capability sources are in the
[investigation receipt](../evidence/2026-09-30_phase3-design/README.md). This reviewer independently
read the pinned DataFusion consumption/resource brief and petgraph container/trait guidance,
as well as actual runtime/store/fork interfaces. Hosted current-version documentation is discovery,
not proof of the pinned API. No library reputation or probe count establishes a performance result.

## 9. Alternatives and tradeoffs

| Alternative | Authority/change locality | Operational and testing burden | Decision/revisit |
|---|---|---|---|
| Extend current all-memory handoffs | Smallest API change, same semantic owner | Facts remain retained alongside join/index state | Declined for cumulative normalization; preserve small P2 handoffs |
| Linked published facts generation | Reuses existing facts | Adds multi-generation leases, retention/reference closure and access protocol | Declined absent a named incremental consumer and measured benefit |
| Fresh cumulative generation with frozen reads | Same model and single-generation publication/retirement | More PostgreSQL reads and stage lifecycle machinery; requires source/cancellation/race controls | Selected, Proposed; no speedup claim |
| Generic policy DSL | Can generate multiple renderings | Adds syntax/algebra/interpreter and migration surface for five policies | Declined; revisit only real extensible-policy requirement |
| Mechanical legacy SQL port | Reuses old query text | Restores legacy identity and last-winner classifiers; makes unknowns difficult to preserve | Declined; retain independent expected answers, migrate ownership and delete old paths |

Broad P0–P2 redesign was considered rather than prohibited. The present model/codec/store choices
serve the target; the concrete foundation changes are stage/read/frontier/resource boundaries.
Preserving them avoids an unrelated backend requalification without hiding the identified defects.

## 10. Verification and uncertainty

| Claim | Evidence label/date | Command or inspection | Outcome/boundary |
|---|---|---|---|
| Baseline reader/runtime behavior | Tested, author-run receipt inspected 2026-09-30 | `python3 scripts/build_environment.py -- cargo test --release -p cpg-core --test model_runtime --test generation_read` | passed: 11 real PG reader tests and 2 runtime tests in retained log; not run by this reviewer, not P3 qualification |
| Selected resolved dependencies | Interface-checked, 2026-09-30 | `uv run python docs/design_review/evidence/2026-09-30_phase3-design/inspect_dependencies.py` receipt and JSON | passed in author receipt; resolved features, not runtime performance |
| Source-bound stage API and completed-stage protocol | Proposed; current interfaces inspected 2026-09-30 | Model stages, runtime registration, store lifecycle/DDL/locks and fork guard placement | Implementation not_run; existing code does not already supply the proposal |
| Semantic admission | Proposed, 2026-09-30 | Source binder/surface/composition inspected; F01/F02 corrections reinspected in plan, DESIGN and ADR | Design gaps corrected; no semantic runtime tests added or claimed by this review |
| Normalized profiles, resource envelope and phase exit | Proposed | Plan §10 commands and preregistered cases | not_run: implementation is absent |
| Product gate | Outside documentation-review scope | `just test-all` | not_run by this reviewer |

Static source/type review is sufficient to judge these proposed contracts. No additional probe
was needed to identify the two admission gaps. Real-store races, early cancellation, multi-scan
capacity, content equivalence and independent semantic twins remain required implementation
evidence; their absence is not relabeled a passed test. Known-answer graph shapes include isolates,
parallel arcs, self-loops, cross-file cycles, unresolved endpoints and shuffled insertion order.

## 11. Authority changes and dispositions

| Change | Owner/decision route | Disposition and remaining obligation |
|---|---|---|
| Cumulative normalized frontier and completed reads | Proposed ADR-0101 + DESIGN §15.1/§15.11 | Plan R1–R3/N7; accept decision before changing production contracts; existing reader/store findings remain under parent §8 |
| Normalized identity, effective contracts, policies and binding | Proposed ADR-0102 + DESIGN §15.4/§15.5/§15.10 | Plan N1–N6; F01/F02 have one current disposition owner in plan §11 |
| P3/P4/P5 recovery/deletion | Parent cutover plan plus detailed plan §9 | Partition legacy compile expectations by assertion/consumer; do not delete P4/P5 obligations or create legacy-ID bridges |
| Qualification and handoff | Detailed plan Q, current architecture owners | Real-store and both-profile normalized runs; assembled review at exit; no implementation finding closes merely because this design is accepted |

## 12. Architectural judgment and decision

| Judgment | Verdict | Evidence and required action |
|---|---|---|
| A1 Localize change | satisfied | Semantic owners, typed mechanisms, generated representations and bounded pure tests localize the chosen extensions |
| A2 Encode domain meaning explicitly | satisfied | F01/F02 corrections establish governing effective/body, variant-set and normalized applicability operations; derived forms retain their premises |
| A3 Extend through composition | satisfied | Stage/frontier/projection composition and a single binder behind validated normalized applicability cover the selected changes without competing classifiers |

**Bounded decision: Accept scoped**, for the final Proposed P3 architecture and implementation plan.
F01/F02's design corrections are adequate; their implementation and controls remain outstanding.
The explicitly excluded arbitrary effective-callable transformations reopen when typed producer
evidence supports them. No production behavior, performance or release is qualified by this review.

**Enclosing architecture:** accepted as a Proposed target for the named P3 scenarios, not as an
implemented normalized system. P0–P2 retains its facts-only qualification; P4 analysis/catalog and
P5 serving remain outside this bounded acceptance. D0 decision reconciliation/ADR acceptance
precedes R1–R3. Their implementation review must establish source authorization, freeze/drain and
resource behavior; N4/N5 must establish the corrected semantic contracts before whole-phase Q.
