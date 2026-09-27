# Design review: Stage 3 channel contracts

## 1. Scope, outcome and coverage

**2026-09-26 · design/target · document and contract review · reviewer: independent design-reviewer agent.**
Applies core **3.0**, code-intelligence **1.1**, and the repository binding through
[standard.toml](../design_principles/standard.toml).

The subject is [ADR-0058](../../adr/0058-stage3-channel-contracts.md), its working-tree
governed sections (§B5/§B6, §9.9, §11.3), and the adopted
[execution queue](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution).
This review enables initial S1 contract implementation; it does not accept production behavior.
The current value-only contracts, phase-specific model activation, existing transfer/RCA consumers,
and FORMAT 9 query/evidence requirements constrain the target.

**Proposed target: Accept scoped**, with F01 corrected by owner/decision inspection on 2026-09-26.
**Current implementation:** incomplete. Committed compiler output 81 and FORMAT 8 remain
the baseline; the four pre-existing modified definition-header files contain unverified draft
work and were neither changed nor tested here. S1–S8 production acceptance remains outstanding.
Stage 5 deferred execution, general alias analysis, a generic interpreter/proof framework, and
new Stage 4 behavioral FCA/registry features are excluded. Their exclusion does not excuse
misclassification of current synchronous facts or current RCA output.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and consumer contract | Change boundary |
|---|---|---|
| `cpg-schema` | Typed channel subjects/origins, source references, phase applicability, proof obligations, identities and coverage invariants | New semantic category deliberately changes the contract; an ordinary model instance does not |
| Providers and model catalog | Attributed observations versus pinned authored assertions; neither proves execution merely by naming a target | Provider revision or model/pin revision |
| Pure analytics | Ordered evaluation, binding, completion and bounded summary composition over explicit inputs | Python semantic rules and recursive transfer rules |
| Core and publication validation | Acquire relations, order stages, reconstruct results and publish one snapshot | Transport and lifecycle; no private semantic classifier |
| Immutable native executor / Python adapter | Admit shared structural proofs and perform bounded semantic selection / validate and render protocol messages | Query semantics / protocol presentation |
| Current RCA and Stage F | Typed relation/modality/endpoint meaning survives analysis; labels render it | Analytic attributes versus rendering, with §9.6 as owner |

Dependency direction remains orchestration → capability owners → schema contracts. Sharing proof
admission does not authorize schema → core or schema → analytics dependencies: schema checks
structural obligations; pure semantic reconstruction stays with its existing transformation owner.

| Fact/fidelity family | Authority and identity | Coverage and consumers |
|---|---|---|
| Source observations | Pinned provider facts, source spans and snapshot; extracted/resolved meaning retained | Missing, approximate and open dispatch observations remain visible to composition |
| Model application | Catalog revision/pin plus cited source candidate; synthetic assertion, not runtime observation | Independent channel and phase applicability; summary consumers preserve modality |
| Channel result and coverage | Derived semantic subject/origin plus condition/channel/phase; proof identity separate from semantic progress | Complete empty result requires independent coverage proof; no inference from missing positive rows |
| Served claim / RCA attribute | Same-snapshot evidence and typed meaning; presentation is derived | Native/Stage F must not discard phase, modality, support or unresolved alternatives |

## 3. Contracts, constraints and testing boundaries

The target correctly separates normal evaluation, exact value, call binding, completion,
potential effect, semantic completeness and proof omission. S1 must encode these distinctions
in checked construction and shared publication/native admission, not only in documentation.

Typed subjects refer to actual source relations. Parameterless effects and effects preceding a
raise cannot borrow a return or parameter identity. A complete empty channel names the finite
domain it closed and its coverage proof; an empty table cannot be that proof. Subject, condition,
channel, invocation phase and snapshot must agree across all cited obligations. The full domain
includes unresolved alternatives. Positive witnesses and witness counts are not completeness.

The existing ordered proof model remains the default: new typed obligations may cite several
pieces of evidence without introducing a generic multi-parent proof engine. Any genuinely new
proof topology follows §9.9's explicit decision route. Shared admission must check meaning and
references; sharing a step-kind whitelist is insufficient. Independent oracle expectations must
not be generated from these production checks.

Pure contract tests require only explicit rows and diagrams. Real source → publication → native
tests check translation and lifecycle separately. Old FORMAT 8 stores are rebuilt under ADR-0048;
the target promises no historical reader compatibility.

## 4. Composition and execution

| Stage/question | Projection, method and model | Budgets, effects and evidence |
|---|---|---|
| Evaluation/completion: what executes and how does it exit? | Ordered source operands, predecessor suites and entered frames; conservative synchronous Python model | Bounded depth/work/proof size; exact outcome or named refusal; no store or runtime execution |
| Summary composition: what crosses this call? | Declared callable universe and attributed directed call sites; retain parallel sites/origins; SCC fixed point over typed channel states | Separate semantic progress, shorter representatives and retained proofs; deterministic caps produce refusals, never negatives |
| Coverage/discharge: which claim is closed? | Subject/condition/channel/phase domain, model/callee alternatives and relevant handlers/exits | Matching coverage certificate discharges only its claim; shared reconstruction precedes snapshot publication |
| Native semantic selection: which operations match together? | Whole declared public universe; exact formal bindings and compatible witness conditions/scopes | Matched/excluded/source-open/unexamined partition, generation/query-bound cursor, bounded closure and explicit work; path inspection stays path-local |
| RCA: which current structural attributes co-occur? | Existing object universe and typed relation/modality/endpoint attributes, retaining evidence; fixed current FCA/RCA analysis | Labels never become keys; candidate arcs stay candidates; no behavioral membership inference |

The S2b/S3a bootstrap removes the context-model dependency cycle. Model declarations provide only
the minimum supported synchronous protocol facts; completion composes ordered entry and reverse
exit. S4 then composes channels, S5 discharges coverage and repairs adjacent projections, and S6
serves the same contracts. Independent challenges accompany this work; integrated gates follow it.

## 5. Change and failure scenarios

| Scenario | Owner and propagation | Settling evidence required during implementation |
|---|---|---|
| Add parameterless logging before a later raise | New channel subject; reached-prefix evidence in analytics; mechanical schema/publication/native projections | Positive effect with exceptional completion; no fabricated return witness |
| Prove an operation has no effects in a covered phase | Coverage owner closes the declared domain; query consumes that certificate | Complete empty versus unanalyzed empty; open dispatch or another phase prevents exclusion |
| Add another pinned model using existing semantics | Catalog and focused controls; existing bindings, proof checks and serving reuse contracts | One positive/withholding source pair; no new seed/native semantic interpreter |
| Add synchronous nested context exit | Completion plus supported model declarations; resource identity and pending outcome survive composition | Later entry failure, reverse unwind, suppression and replacement exception |
| Add a semantic query conjunction or renderer | Native chooses jointly compatible witnesses; Python/Stage F renders canonical results | Individually satisfiable but incompatible witnesses do not match; missing closure stays unknown |
| Relabel an RCA target with a candidate call | Typed attribute identity and evidence remain; renderer changes only text | Membership invariant under relabeling; candidate and definite controls retain distinct meaning |

These are **Proposed** change routes, not measured edit locality or executed results.

## 6. Correctness and fidelity gates

Verdicts below concern the **documented target**, not the unfinished production implementation.

| Gate | Verdict | Evidence and boundary |
|---|---|---|
| G1 Authority | pass | Schema/proof owners are coherent; corrected §9.6 names the accepted RCA target and ADR-0058, whose governed association includes §9.6 |
| G2 Semantic fidelity | pass | Typed subjects, phases, empty coverage and independent modalities are required; fake value identifiers excluded |
| G3 Validity | pass | Shared construction, reconstruction and publication/native checks have named owners; actual enforcement remains S1–S6 work |
| G4 Hidden behavior | pass | Pure analytics, explicit core effects, immutable serving; oracles run only isolated generated programs |
| G5 Consistency/recovery | pass | ADR-0048 current-store rebuild, one snapshot/generation and verified independent embedding-cache reuse; no new recovery guarantee |
| G6 Transformation/reuse | pass | Semantic versus witness identity, preserved origin/condition alternatives, bounded fixed point, typed RCA projection and deterministic checks specified |
| G7 Truthful claims | pass | Proposed implementation is explicit; focused history, draft work and integrated `not_run` remain separate |
| G8 Library leverage | pass | Existing library boundaries retained; actual-contract engine comparison precedes any replacement; no generic framework added |
| CI-G1 Fidelity | pass | Unknown is not absent; invocation phase, modality and claim-specific coverage constrain the target; current W10 remains unfixed production work |
| CI-G2 Evidence closure | pass | Same-snapshot support and shared proof admission required; caps cannot silently erase evidence |
| CI-G3 Evaluation integrity | pass | Frozen questions, isolated oracle lane and heldout exclusion remain explicit; evaluation requests do not feed compiler facts |

## 7. Findings and applicability

<a id="F01"></a>**F01 — Associate the newly decided RCA target with its owning architectural section.**

- **Initial evidence:** ADR-0058 decided typed current RCA attributes, but its `design` list omitted
  §9.6. The first inspected §9.6 gave the implementation/known-gap route and W10 link without the
  newly accepted decision; the new target appeared under §B5/§9.9 instead. A developer changing an
  RCA attribute through owner → decision could therefore miss the decision at the actual owner.
- **Principles and consequence:** FP-01/FP-04, DP-01/DP-24; A2/G1. This is a documentation ownership
  gap, not a rejection of the selected mechanism or evidence of an additional production defect.
- **Correction inspected:** ADR-0058 now includes §9.6 in its governed association. The analytics
  owner adds the accepted typed-attribute/renderer target, its Proposed implementation label and
  `Decision: ADR-0020, ADR-0058`, while retaining the current implementation gap and W10 link.
- **Closure evidence:** **Interface-checked, 2026-09-26**, by reading ADR-0058's front matter and
  `analytics.md` §9.6 after correction. This documentation ownership finding is corrected; no
  product test is warranted and no production RCA correction is implied. Current execution disposition belongs with
  [W10](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition) and S0/S5.

FP-02/03/05/06 and their relevant DP-02/03/08/12/15/18/20/21/23 obligations are satisfied at the
**Proposed target** level by §§2–5. FP-01/04 are satisfied after the inspected F01 correction.
No source implementation conformance is inferred from these verdicts.

## 8. Library fit and total complexity

Keep schema-owned Arrow contracts, DataFusion construction/validation, the existing bounded
condition kernel and petgraph schedule. They already occupy coherent representation, relation,
Boolean and graph boundaries; this review proposes no API or version change. Specialized Python
admission remains a pure repository responsibility, not generic graph reachability.

ADR-0053's worklist is the default until S4 compares Ascent/datafrog on the actual multi-channel
state, proof and refusal contract. The prior reachability comparison does not qualify that target.
No performance claim or fresh library API qualification is made here. Requiring a generic proof
engine or another crate now would add machinery without an established consumer advantage.

## 9. Alternatives and tradeoffs

Extending value-shaped rows is the smallest edit but invents identities and prevents honest empty
coverage. Repeating source/native classifiers makes an ordinary model require multiple semantic
edits. The chosen typed contracts are the smallest viable design that accommodates the agreed
channels; they incur deliberate schema migration and focused reconstruction work. A universal
interpreter/provider/proof framework adds configuration and lifecycle without providing Python
evidence. Engine replacement remains a separate actual-contract comparison, with one production
engine and deletion of the superseded implementation if selected.

## 10. Verification and uncertainty

**Interface-checked, 2026-09-26:** ADR-0058, governed-section and plan diffs, adjacent §9.6 owner,
existing §9.9 proof/coverage contract, §11.3 semantic terms, ADR-0048/0053 and validation-lane owner
were read. `git status --short` and the scoped documentation diff established the dirty boundary.

`just test-all`, `just pilot`, focused production tests, oracle probes, clean-wheel query and live
embedding checks: **not_run**, because this review judges documents/contracts before implementation.
`just docs-check`: **not_run** by this reviewer; the author owns publication verification.

Meaningful S1 controls are wrong source-kind/subject/phase references, complete empty versus
missing analysis, reordered/missing proof obligations, and mixed-snapshot support. S2–S7 add real
source/native round trips and independent semantic challenges. These must establish implementation;
this document does not pre-credit those outcomes.

## 11. Authority changes and dispositions

ADR-0058 extends ADR-0057 without replacing ADR-0053's current scheduler decision or ADR-0048's
migration policy. F01's owner/decision association is corrected; no new register or plan is
needed. The forward plan alone owns current execution disposition. Existing source-review IDs remain intact.
S1/S6 delete migrated seed-specific/native interpretation; S5 deletes unconditional strongest-
transfer pruning and label-based RCA interpretation only after their real consumers migrate.
Independent controls survive those deletions.

## 12. Architectural judgment and decision

| Judgment | Verdict | Basis |
|---|---|---|
| A1 Localize change | satisfied, Proposed target | Coherent schema, pure semantic, publication and serving owners; ordinary model additions reuse their contracts |
| A2 Encode meaning structurally | satisfied, Proposed target | Subjects, phases, coverage and proofs have explicit authorities; F01's inspected correction restores the RCA owner/decision route |
| A3 Extend through composition | satisfied, Proposed target | Evaluation/completion/model channels compose without inventing a generic framework; engine choice remains evidence-dependent |

**Bounded decision: Accept scoped.** The initial channel-contract target and adopted ownership
are suitable for S1 after the inspected F01 correction. No additional production blocker was
established by this document review. This accepts the specified architecture at Proposed strength,
not implemented contract enforcement, native conformance or assembled Stage 3 behavior.

**Enclosing Stage 3 architecture and release:** unresolved implementation and qualification.
Acceptance is scoped to the initial target decision;
typed contract enforcement, multi-channel composition, FORMAT 9, current transfer/RCA repairs and
all integrated evidence remain the forward plan's unfinished obligations.

## 13. Bounded implementation follow-up: initial S1 and definition headers

**2026-09-26 · change/conformance · source inspection.** This follow-up reviews only the dirty
typed-subject migration, local statement coverage, shared callee-reference admission and inherited
fresh-definition header implementation. It does not replace the target decision in §12 or certify
S1, S2 or Stage 3. Production files were read, not changed by this reviewer.

**Implemented, inspected:** `SummarySubject` distinguishes `ValueOrigin` and `ExecutionSite`;
the migrated coverage key includes the subject kind and identity. Execution sites need no formal
or return identity. Core's `completion_outcomes` source selection excludes asynchronous
declarations and functions with directly owned yield/yield-from nodes; analytics creates site
coverage conditional on entry. Empty escaping-exception rows therefore do not prove that a site,
callback or enclosing callable executes. No callback/resource channel is certified by this slice.
Publication reconstructs both completion rows/steps and coverage rather than trusting the stored
`complete` flag (`validate.rs::validate_summary_flows` and completion reconstruction).

The source and native paths now call `summary_contract::admit_callee_proof`; the native adjacency
loop has been removed. The shared function checks contiguous control obligations, cited callee
conditions, admitted verdicts and decreasing proof depth. It is still **callee-reference structural
admission**, not completion of all S1 proof semantics: source reconstruction continues to own
argument binding/evaluation, and FORMAT 9 support remains open.

Definition-header preparation admits only a directly nested synchronous undecorated definition
with a fresh unique lexical binding and checked source/declaration/default relationships. Defaults
are evaluated in definition order; the function body is not executed. Rebinding, unsupported
headers and failing default evaluation withhold completion. This establishes header completion,
not omitted-default availability or stability at a later call. No new invocation-default positive
was found in this slice.

### Findings

<a id="F02"></a>**F02 — Shared structural admission initially erased the local proof-limit reason.**

The first inspected `finite.rs` caller mapped every `admit_callee_proof` error to `MissingEvidence`.
The shared check refuses a proof exceeding `MAX_SUMMARY_PROOF_STEPS` before `push_finite_path`,
so a long local-call application lost its required `SummaryProofLimit` reason. This affected
DP-02/08/21 and G2/G6; the architectural owner was correct but its failure contract was flattened.

**Correction inspected, 2026-09-26:** `ProofAdmissionError` now carries the typed boundary reason,
the source caller retains `error.reason`, and the native adapter renders its message. The new
`oversized_local_application_retains_the_named_proof_limit` control exercises the local application
and checks both boundary and coverage reasons. This corrects the demonstrated cause by inspection;
the author's post-correction focused receipt is attributed below. The forward plan S1/W5 owns execution status.

<a id="F03"></a>**F03 — Empty escaping-exception coverage needs a schema-owned domain distinction.**

The first inspected `completion::coverage` computed only exceptions escaping an entered statement,
but published that result under the unqualified `SummaryChannel::Exception`. The schema-owned
contract did not state this narrower domain, while modeled exception actions include `Raise`, `Catch`,
`Convert` and `Suppress`. A `try` statement that raises and catches `TypeError` has no escaping
exception, yet has exception activity: its complete zero-witness row cannot close a raised/caught
exception claim. The initially inspected site producer was locally conservative, but consumers
could not recover the distinction from its published type without private producer knowledge.

This affected FP-02/04/05, DP-02/03/08 and CI-04/06. **Correction inspected, 2026-09-26:**
schema-owned `CoverageDomain` and `coverage_domain(subject_kind, channel)` explicitly distinguish
`EscapingException` from value transfer and completion. The table documents that domain and checks
the allowed subject/channel combinations. Consumers must match this typed domain before using
completeness. The new handled-raise control keeps one exception witness for the nested raise and
zero escaping witnesses for the whole handled statement. The correction preserves conditional-on-
entry meaning and does not create generic raised/caught, operation-wide or other-phase coverage.
The identified domain ambiguity is corrected by inspection; the author's post-correction focused
receipt is attributed below. Current execution disposition belongs in forward-plan S1/S5; no new plan is required.

### Bounded judgment and verification

| Judgment/gate | Bounded implementation verdict | Evidence and remaining boundary |
|---|---|---|
| A1 Localize change | satisfied | Typed subject owner, pure producer and mechanical native consumer; no new framework |
| A2 Encode meaning structurally | satisfied for the inspected migration | F03 corrected: schema owns the checked subject/channel domain and explicit escaping-exception meaning |
| A3 Extend through composition | satisfied for the inspected migration | Source/native callee rules share one implementation; general proof/channel composition is excluded |
| G1/G3/G4/G5 | pass by inspection for this slice | Schema and reconstruction owners retained; no new hidden effects or publication boundary |
| G2/G6; CI-G1 | pass by inspection for this slice | F02 retains typed refusal and F03 exposes the claim-specific domain; no broader completeness inferred |
| G7/G8; CI-G2/CI-G3 | no new defect found in inspected scope | No new measured claim, library selection, evidence projection or oracle-input path; enclosing verification remains open |

**Author-recorded focused receipts, 2026-09-26:** read from [STATUS](../../../STATUS.md) after
the F02/F03 corrections; this reviewer did not execute these commands. Cargo work and the editable
rebuild explicitly used `CARGO_TARGET_DIR=/home/paul/library-context/target`, correcting an inherited
shell value pointing to another checkout. The receipts establish only the named cases.

| Command | Attributed outcome and limit |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p lctx-analytics -p cpg-schema -p cpg-core -p lctx-semantics --lib --test contracts --test codebooks --test compile --test bundle -E 'test(completion::tests) \| test(summaries::finite::tests) \| test(summary_contract::) \| binary(contracts) \| binary(codebooks) \| test(composed_argument_reads_keep_ordered_source_evidence) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response) \| package(lctx-semantics)' --status-level fail --final-status-level fail` | `passed`: 52 focused Rust/schema/source-Delta cases after F02/F03, including reviewed schema migrations and header/coverage reconstruction. Its Python subprocess loaded an older editable native binary; this run does not establish current-native conformance. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: rebuilt the editable native package from current sources. This is not clean-wheel qualification. |
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core --test bundle -E 'test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` | `passed`: one fresh source → Delta → generation → rebuilt-native fixture case. Fake embedding qualifies the fixture only. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -k missing_proof_steps` | `passed`: one current-native test including missing/orphan proof controls and mismatched callee-condition rejection; uses an existing fixture generation, not an integrated pilot. |

**Bounded implementation decision: Accept scoped at source-inspection strength.** F02 and F03
are corrected by inspection. No additional false-completeness or definition-header blocker was
established. General channel admission, default invocation stability, callback execution, model
phases and FORMAT 9 remain excluded unfinished work, not accepted implementation. The target
acceptance in §12 stays separate. The attributed receipts above add focused testing evidence;
this reviewer ran no tests. Full gates, pilot, clean-wheel, live embedding and general multi-channel/
native qualification remain `not_run` here. Neither the earlier editable-binary result nor the
subsequent focused rebuilt-native pass certifies assembled Stage 3.

## 14. Bounded implementation follow-up: fresh source-default binding

**2026-09-26 · change/conformance · dirty slice after `bde50d7` · source inspection.** Reviewed
`lctx-analytics::call_binding`, its core acquisition, the local-summary consumer, the removed SQL
binding-count classifier, new codebook entries, and the source/native and independent runtime
controls. No production files were edited and no tests were executed by this reviewer.

**Implemented, inspected:** the pure source binder calls schema-owned `bind_arguments` and
checks the explicit argument/formal mappings. An omitted source default requires a synchronous,
undecorated nested definition directly followed by its sole direct return call, a fresh binding,
the sole resolving callee reference, and the independently reconstructed normal header outcome.
Module functions, escaped/aliased defaults, intervening actions, recursion, unsupported headers,
and arguments containing nested calls or named assignments remain withheld. Header steps and
the exact default syntax supply availability/value evidence; no default expression is evaluated
again at invocation and no argument row/ordinal is fabricated.

The finite producer separately requires every actual argument's ordered normal-evaluation proof
and the return's predecessor/exit proof. Current evaluation admits primitive operations and
ordinary name reads but withholds arbitrary truthiness, attribute protocols and overloaded
operators; combined with the binder's stricter call/assignment exclusion, this supports the narrow
no-intervening-effect window. Definition-time execution already occurs in the predecessor proof;
later `DefaultAvailabilityEvidence` references that established creation, rather than describing
another execution. Exact Boolean default values may specialize the linked callee controls;
opaque or non-Boolean default values are not converted to Boolean facts.

The binding proof is composed before shared callee admission and the existing cumulative proof
limit. `DefaultUnavailable` and `DefaultStabilityUnknown` remain explicit refusals. The old SQL
`formal_counts`/`bound_counts` and `binding_complete` decision are removed: SQL still acquires
attributed candidates, while source-binding policy has one pure owner. Core reconstruction invokes
that production binder, so publication checks the same source obligations; native loading remains
the previously scoped structural admission, not a second default interpreter.

**Change scenario:** another positional or keyword-only default in the same supported window
changes source/model data consumed by the shared binder and a focused control. It does not require
another SQL count policy or native classification branch. A broader lifetime or implicit protocol
requires a deliberate stability contract change. In particular, if the expression evaluator gains
normal outcomes that may run attribute/operator/protocol hooks, the current call/assignment
exclusion must not silently become a no-effect certificate: keep those forms withheld here or
consume a shared effect/stability certificate before admitting them. This is the boundary of this
acceptance, not a newly established defect in the current supported expressions.

| Judgment/gate | Verdict for this narrow slice | Evidence and qualification |
|---|---|---|
| A1 | satisfied | Explicit pure inputs and output; source binding decisions leave the SQL adapter |
| A2 | satisfied | Explicit arguments, omitted defaults, exact values and availability/stability proofs remain distinct |
| A3 | satisfied | Existing binder, header/evaluation proofs, condition specialization and shared admission compose without a new interpreter |
| G1–G6; CI-G1 | pass by inspection | One binding owner; ordered actual evaluation; bounded proof admission; no default re-evaluation, phase promotion or coverage closure |
| G7/G8; CI-G2/CI-G3 | no new defect found in inspected scope | No new library/version, measured benefit, independent oracle input, or serving-format claim |

No blocking finding was established. Inspected controls cover enabled/disabled Boolean defaults,
keyword-only and unused defaults, a skipped versus evaluated unsupported default expression,
removed positional/keyword defaults, escape, an intervening statement, and a nested-call argument.
The pure finite test checks that specializing a default adds no `ArgumentEvaluation` step.
The generated CPython program checks fresh default selection, definition-time evaluation and
mutation behavior independently; agreement alone is not compiler/runtime equivalence.

**Bounded decision: Accept scoped at source-inspection strength.** Actual source → Delta → native
fixture execution is in progress with the author; no new test receipt is credited here yet.
This accepts only the fresh immediate-call window and existing explicit-argument behavior.
General default lifetimes, default-derived value transfer, synchronous context exit, callback/
resource fate, full S2 and assembled Stage 3 remain unfinished. Earlier sections' target and
bounded-slice decisions keep their original scope.

## 15. Bounded follow-up: private callable proof support and FORMAT 9 prerequisite

**2026-09-27 · change/conformance · dirty slice after `bde50d7` · source inspection.** Reviewed
the removal of the public-path restriction from `cpg-core::entry_links::test_uses`, the serving
`callable_parameters` projection, and the corresponding schema, Python and native reader changes.
This is the necessary callable-support subset of S6 brought forward for S2's fresh default calls;
it is not acceptance of the complete FORMAT 9 semantic-query target. No production edits or tests
were performed by this reviewer.

**Implemented, inspected:** the entry-link producer uses the same single reaching-parameter,
complete flow coverage, exact owner, predecessor-test and intervening-effect checks for private and
nested functions as for public functions. Removing the `public_paths` join changes the source
analysis universe, not those proof obligations. The effect-model digest includes the relation SQL,
so the broadened derivation changes the digest without relying only on its revision label.
Public visibility is owned by the query surface, not by this source identity proof.

`cpg-schema::bundle` declares `callable_parameters(function_node_id, formal_node_id, name)`;
core derives it from declared functions/async functions and their raw parameter syntax in the
same snapshot. The former `operation_parameters` was itself a direct operation-ID/raw-parameter
join; this replacement does not remove an additional class-to-constructor formal mapping.
The projection supplies private callee evidence required by shared proof admission. Merely
including an async declaration's parameter is structural metadata: it supplies no Call-phase
execution, completion, callback activation or channel-coverage claim.

Native loading builds the complete `(function, formal)` membership set independently of the
public query lookup. It rejects duplicate formal pairs, duplicate callable parameter names and
one formal assigned multiple owners. Every loaded value link must name a pair from that set;
the existing leaf/atom/place/condition and effect-digest checks still apply. Only functions in
`operations` populate the name lookup, and requests still resolve through public paths belonging
to those operations. Private metadata therefore supports a public caller's proof without making
the callee directly queryable. Schema-owned IPC validation, the existing byte/row bounds and the
Python adapter boundary remain in place.

**Ownership and deletion:** this is an implementation prerequisite within ADR-0058's accepted
FORMAT 9 target and ADR-0048's fresh-generation policy, not a new architecture authority. The
writer and Python manifest reader now require FORMAT 9, and the old serving filename/field reader
is removed rather than retained as a FORMAT 8 fallback. `operations` and `public_paths` retain
their public-surface responsibilities. A private function becoming public changes those surface
rows; it does not require a second source proof policy or a new native proof interpretation.

The owning documentation now describes the source link as a **callable** entry-formal proof,
renames the storage/publication inventory entry, and marks FORMAT 9's callable-support subset as
implemented while keeping the rest of S6 unfinished. These owner/STATUS/plan updates were
re-inspected on 2026-09-27 after commit `3c59c0c`; the documentation condition is closed.
The initial target review's FORMAT 8 statements
describe its dated baseline, not the current slice. Full formal-reference admission for summary
and boundary rows, complete typed channel/proof support, bounded cited spans and operation-wide
conjunction/effect/role queries remain broader S6 obligations; this subset establishes none of
their closure.

| Judgment/gate | Verdict for this subset | Evidence and remaining boundary |
|---|---|---|
| A1 | satisfied | Source proof universe, serving metadata and public query selection have separate owners |
| A2 | satisfied | Callable formal identity is explicit; public query membership is checked separately |
| A3 | satisfied | Private callee proofs reuse existing source and shared native admission, without a visibility-specific interpreter |
| G1–G6; CI-G1/CI-G2 | pass by source inspection | Attributed same-snapshot projection, unchanged proof checks, explicit public lookup and bounded loading; full S6 closure excluded |
| G7 | pass by inspection after documentation correction | Current owners and checkpoint distinguish the implemented FORMAT 9 prerequisite from the unfinished query contract |
| G8; CI-G3 | no new defect found in inspected scope | No new library, independent evaluation input or measured claim |

**Bounded decision: Accept scoped at source-inspection strength; documentation closure confirmed
2026-09-27.** No production correctness blocker was found.
Focused acceptance should cover a real nested default call through fresh source → Delta → native,
refusal of a direct private-path query, rejection after removing the private formal needed by a
link, duplicate callable name/owner rejection, and strict refusal of the retired format/schema.
**Author-recorded receipts, 2026-09-27:** read from STATUS after commit `3c59c0c`; this reviewer
did not execute them. Cargo commands use `CARGO_TARGET_DIR=/home/paul/library-context/target`.

| Command | Attributed outcome and limit |
|---|---|
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p lctx-analytics -p cpg-schema -p cpg-core --lib --test contracts --test codebooks --test bundle -E 'test(completion::tests) \| test(summaries::finite::tests) \| test(summary_contract::) \| binary(contracts) \| binary(codebooks) \| test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail` | `passed`: 52 focused cases; reviewed append-only reason/proof codes, defaults and callable-support migration. |
| `uv sync --frozen --reinstall-package lctx-semantics` | `passed`: editable-native rebuild for callable support, not clean-wheel qualification. |
| `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-core --test bundle -E 'test(finite_depth_and_unsupported_refusals_reach_the_native_response) \| test(serving_schema_digests_are_the_shared_known_answers) \| test(a_generation_rebuilds_to_the_same_bytes)' --status-level fail --final-status-level fail` | `passed`: 3 final source/generation cases including missing private formal rejection, default evidence without invocation-time evaluation, private query refusal, schema digests and deterministic rebuilding; fixture fake embedding only. |
| `uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py -k private_callable_formals`; `uv run --no-sync pytest python/lctx_mcp/tests/test_digest.py -k expected_schemas` | `passed`: 1 each; duplicate callable name/owner rejection, private query boundary and Python/Rust schema agreement. |
| `uv run python docs/design_review/evidence/2026-09-26_default-availability/probe.py` | `passed`: independent CPython 3.14.7 default mutation/freshness/override/definition-once/unused-raise/short-circuit controls; not exhaustive compiler equivalence. |

These focused receipts supplement §§14–15 only. Full gates, pilot, clean-wheel and assembled
S2/S6/Stage 3 acceptance remain `not_run` in this review.

## 16. Bounded contract follow-up: dynamic validation schemas

**2026-09-27 · change/conformance · dirty slice after `3c59c0c` · source inspection.** Reviewed
`cpg-schema::models`, the model/site Arrow rows and shared validation rules, typed context-class
discovery in extraction, the argument-binding projection, and core's catalog/site reconstruction.
This is the initial S1 schema contract required before activating validation models. No new
validation model is authored by this slice; no production edits or tests were performed here.

**Implemented, inspected:** catalog format 3 replaces the validation effect's string with the
closed `ValidationSchema` variants `StaticClass`, `RuntimeValue` and `Unresolved`. Static classes
must resolve uniquely to a pinned context class node and fact through the same resolver used by
exception models. `Rule::context_classes` exposes the typed acquisition requirements; extraction
does not interpret a rendered effect label. `Validate.argument` is absent. A runtime source is an
`InputPath` with its own `ModelPathRole::Schema`, separate from the affected subject's Input role.
The existing pinned-signature binder maps that role to a source argument without a second model
grammar or schema-specific binding policy.

`ModeledEffectSites` preserves the schema variant and either the static class evidence or the
runtime argument node/fact pair. Missing, ambiguous, unpacked and unsupported source mappings
retain a schema reason independently of the subject status/reason. Only an exactly bound
Parameter path may presently identify a runtime source; field/global paths and implicit receiver
values remain open rather than substituting the root argument for the selected value. A source
expression witness identifies **where** a runtime schema comes from. It neither resolves its
contents nor proves evaluation, validation success, invocation completion or a reached effect.
The row remains an attributed candidate with the model/target modalities and open-target state.

The shared shape helper requires the matching variant fields, makes validation and schema-kind
presence agree, and keeps every schema field absent on other effect kinds. Runtime paths are
restricted to the three InputPath kinds; ReturnValue and Raise are excluded. Site rules require
either the complete expression pair or an explicit reason, with no class/source invention for
Unresolved. `NOT COALESCE(..., false)` rejects nullable malformed shapes. New reference rules
join schema class and source witnesses to their catalogs, while publication still reconstructs
the exact authored catalog and site rows. The Schema role is append-only and the existing resource
model/site rules now constrain resource roles to Input/Output, avoiding an accidental widening
of that adjacent contract. These two invariant-strengthening observations were corrected during
the bounded inspection; no producer false claim was established.

**Scope and change scenario:** adding a supported validation rule changes the authored model,
its pinned evidence and focused controls; its schema source and subject can use the same formal
or different formals without adding a native interpreter. Static acquisition is currently bounded
by available, described pinned context classes. Extraction still splits an authored full class
name at its last dot and does not independently load arbitrary schema modules: an otherwise
undescribed module or nested class can therefore be missing and compilation fails closed. Do not
claim generic static-schema activation from this scaffold. The first model needing broader
acquisition must establish the provider module/qualified-class boundary and cited class evidence.
Likewise, supporting field/global/receiver values later needs actual selected-value evidence;
an opaque path ID or its display spelling does not supply it.

| Judgment/gate | Verdict for this contract subset | Evidence and remaining boundary |
|---|---|---|
| A1 | satisfied | Typed grammar/class discovery and resolver own model meaning; existing binder and reconstruction are reused |
| A2 | satisfied after inspected shape corrections | Static identity, runtime source, unresolved schema and effect subject remain distinct; resource role retains its smaller domain |
| A3 | satisfied for admitted Parameter sources | Model data composes with shared argument binding; broader source acquisition/value proofs are explicitly excluded |
| G1–G6; CI-G1/CI-G2 | pass by source inspection for this subset | Closed variants, source references and equality reconstruction; no activation, success or negative-coverage claim |
| G7 | scoped | Owners/checkpoint must describe the supported contract and fail-closed acquisition limits, separately from validation-model activation |
| G8; CI-G3 | no new defect found in inspected scope | No dependency change, runtime execution, evaluation-data input or performance claim |

**Bounded decision: Accept scoped at source-inspection strength.** Focused acceptance shapes are:
parse old-string/invalid-variant rejection; missing/ambiguous static class;
same versus distinct subject/schema formals; runtime exact/missing/unpacked/overload-disagree
binding; field/global/implicit receiver boundaries; forged class/source/variant reconstruction;
and nonvalidation/resource regressions. Reviewed schema/codebook snapshots and a real
source/Delta application control must precede a tested contract claim. Any schema-specific
negative later requires schema-qualified coverage; a bound source or complete generic effect
catalog alone cannot supply it. Model activation, multi-channel propagation, native schema
selection, full S1/S3/S6 and integrated Stage 3 remain unfinished. Full gates and pilot are
`not_run` in this review.

**Author-recorded focused receipt, 2026-09-27:** STATUS and
`/tmp/lctx-stage3-schema-kind-final.log` record compiler 85's command below as `passed` (25 cases,
100 skipped). This reviewer inspected the receipt, not a new execution. Cargo used
`CARGO_TARGET_DIR=/home/paul/library-context/target`.

`INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release -p cpg-schema -p cpg-core --lib --test contracts --test codebooks --test compile --test bundle -E 'test(models::tests) | binary(contracts) | binary(codebooks) | test(validation_schema_candidates_keep_attribution_separate_from_subjects) | test(pinned_identity_models_require_and_publish_their_real_formals) | test(model_target_requires_its_cited_pinned_definition) | test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail`

The author reports catalog rejection, candidate schema attribution/missing binding, forged
evidence-pair controls, reviewed schema/codebook/rule migrations and the extraction-only regression.
The validator now reconstructs entry links only when the snapshot has the required published
analysis-condition catalog; the analyzed private/default bundle regression passed in the same
focused run. The design owner and checkpoint distinguish the **Tested** candidate-attribution
contract from **Proposed** production-model activation. This adds no active validation-model,
full S1 or integrated Stage 3 acceptance.

## 17. Bounded implementation follow-up: transfer alternatives and adjacent consumers

**2026-09-27 · change/conformance · dirty compiler-86 slice · source inspection.** Reviewed
`flow_model::Model::sink`, value-flow/predecessor contracts, `behavior::v2_flows` and field
composition, Pass B's consumers, dynamic receiver/negative-premise admission, and FORMAT 9/Python
Fate projection. This is current S5 transfer repair, not all-channel summary or current RCA
acceptance. No production edits or tests were performed by this reviewer.

**Implemented, inspected:** the global minimum-transfer pruning is deleted. Conditions merge
within one origin/transfer kind, while Identity, Derived and Call alternatives survive together.
The value-flow key now includes sink and `through_call`. The author-discovered predecessor-key
collision is addressed by retaining successor/predecessor source-origin IDs through candidate,
compatibility and modeled assignment-return contracts, exact joins and equality reconstruction.
The completion-entry projection deliberately enumerates return-site conditions and remains a
different domain from a chosen origin's transfer proof. No DISTINCT-based alternative collapse
is introduced to repair these keys.

Typed FlowTransfer and condition-scope IDs reach the behavior contract, serving projection and
Python Fate. Known identity forwards from Pass B are typed as Identity. A raw Call transfer
remains Unknown under a shared publication rule. Captured contributions remain ScopeBoundary;
their lambda/deferred bodies do not become active Stage 5 behavior. An unknown dynamic receiver
withholds field/singleton negatives while leaving unrelated unused-parameter premises independent.
These are appropriate ownership changes, subject to the consumer findings below.

<a id="F04"></a>**F04 — Preserve the store's own condition before composing a field read.**
The original draft selected an Established store before its separate condition tuple was
finalized, then replaced that condition with the reading method's condition. A guarded identity
store could consequently support an unconditional composed read. **Correction inspected:**
`behavior::run` now requires the stored identity's actual condition to be unconditional and
nonapproximate, and its callable undecorated; conditional direct stores remain their own claims.
This closes the bounded source-inspection finding without asserting a cross-invocation condition
substitution capability. Acceptance control: guarded store versus unconditional store followed
by the same field reader. Owner/disposition: behavior composition, forward-plan S5.

<a id="F05"></a>**F05 — Receiver exclusivity needs a complete source proof, including raw fidelity.**
The source lattice intentionally omits untracked values; one retained own-receiver entry is not
a complete alternative set. **Correction inspected:** the bounded receiver walk requires one
unconditional non-loop reaching definition/value at each step and rejects missing alternatives,
cycles and raw approximation. `flow_reaching.approximated` and `flow_values.approximated` are
acquired independently of the hydrated condition ID. Approximation propagates through Source,
merge/fixed-point equality, value flows and source contributions; direct behavior, Pass B and
summary seed/source obligations consume it. It is not attached globally to a shared BDD ID.

The adjacent global-receiver fallback originally selected one singleton class by overwriting a
map entry and taking a reference's first root. It now requires all root evidence to resolve to
one meaning, an unambiguous bounded
re-export chain, one module definition and an exact unconditional construction region. Competing
conditional/rebound globals therefore cannot narrow an unknown receiver to the selected class.
This is the existing module-construction model, not proof of arbitrary runtime global stability.
Controls are present for straight-line self aliases, competing/untracked alternatives, loop
replacement, stable/conditional/rebound globals and provider approximation injected into real
extracted tables without changing condition IDs. The combined flag injection does not isolate
each provider flag independently. Owner/disposition: flow model and negative-premise admission,
forward-plan S5; corrected by inspection, with attributed focused evidence below.

The qualified-import regression exposed an adjacent provider-kind distinction: ty's
`ImportFromSubmodule` (code 21) marks an implicit package-submodule binding, not an ordinary
global that shadows that submodule. The module-global shadow set now excludes that marker while
retaining real shadowing bindings. The existing `behavior_shapes` control again resolves both
`bpkg.config.settings.debug` and the directly imported `settings.host` to their singleton keys.

<a id="F06"></a>**F06 — Decorator uncertainty applies to empty operations and intermediate bodies.**
Downgrading only rows whose public operation is decorated missed an empty decorated operation,
a decorated callee along a Pass B path, and a decorated field-reader body. **Correction
inspected:** every decorated public callable now receives its operation boundary independently
of behavior rows; row admission checks the operation, condition scope and every recorded hop's
caller/callee. This is withholding until an identity-preserving decorator model exists, not
decorator execution. Focused controls must exercise all three routes, including the empty
callable. Owner/disposition: behavior admission, forward-plan S5.

<a id="F07"></a>**F07 — Pass B must retain the alternatives it cannot follow.**
The initial `v2_flows` adapter removed attributed ParameterReads after projecting
derived/call/captured contributions to Other, which Pass B ignored. Merely retaining the old
read was insufficient: the `(argument, formal)` followed set suppressed it when a sibling
identity witness existed. **Correction inspected:** source-bearing Other flows now materialize
explicit open-alternative reads, and sibling identity following cannot suppress them. The
append-only `UnfollowedReason::OpenTransfer` states that unchanged transfer lacks proof; it does
not relabel captured or bounded identity as Computed. Synthesis retains this open wording.

The adapter follows only uncaptured, nonapproximate identity with an admitted, nonempty exact
diagram. Missing/refused/approximate conditions retain an open alternative; direct Stage 2
fallback also downgrades approximation. Pass B's projection deliberately groups Other kinds
while the full typed alternatives remain in value flows and direct behavior claims. Controls
are present for the mixed argument and an outer caller, reordered raw inputs and approximation
injection. Owner/disposition: behavior projection and its Pass B consumer, forward-plan S5;
corrected by inspection, with attributed focused evidence below. This does not close
all-channel compositional summary obligations.

The final identity recipe preserves the architecture's claim/grade separation: flow-claim IDs
add transfer kind and condition-scope identity to the existing claim identity. Verdict and
boundary reason grade that claim and remain outside identity, including after normal-path,
decorator and unreachable admission. The interim admission-cause hash and final rehash are
removed. Nonflow hop ownership retains its existing identity route.
The focused source→Delta→bundle→Python fixture checks the expanded Fate transfer/scope fields;
this is fixture qualification, not a complete native or clean-wheel receipt.

**Provider obligations, inspected:** extractor output 32 separates `through_call` from provider
approximation. A source/model proof may discharge a call transfer; its existence does not erase
an approximated value or reaching fact. The finite producer now refuses approximate value
paths, preserving their origin-specific unknown boundaries.

**Attributed focused receipt, 2026-09-27:** the author ran the following command, using this
checkout's established target directory because the inherited shell targeted another workspace.
The inspected log is `/tmp/lctx-stage3-transfer-final-acceptance.log`:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-core -p cpg-schema -p lctx-analytics --lib \
  --test contracts --test codebooks --test compile --test behavior --test bundle \
  -E 'test(reach_fixed_point_tests) | test(summaries::) | test(pass_b::tests) | binary(contracts) | binary(codebooks) | test(transfer_alternatives_keep_conditions_verdicts_and_receiver_boundaries) | test(a_negative_claim_is_refuted_only_where_its_premise_holds) | test(serving_schema_digests_are_the_shared_known_answers) | test(finite_depth_and_unsupported_refusals_reach_the_native_response) | binary(behavior)' \
  --status-level fail --final-status-level fail
```

Outcome **failed: 57 cases run, 56 passed, 1 failed, 151 skipped** (12.330 seconds test time).
The passing transfer fixture covers source→Delta→bundle→Python typed Fate output, positive/open
forwarding siblings, reversed raw input, guarded field stores, captured/decorated callables,
receiver/global narrowing controls and combined provider-approximation injection. The injection
also verifies that no finite summary is admitted. The existing `behavior_shapes` checks, schema
and codebook contracts, and serving schema digests passed in this run. This reviewer inspected
source and the receipt but did not execute it; no mocked-provider result is credited as source
behavior evidence.

The sole failed test, `finite_depth_and_unsupported_refusals_reach_the_native_response`, still
expects a positive native path for `handler_entry_identity`. Its diagnostic separates
`raw_value=false`, `propagated=true` and `exit_region=true`: the return has exact source
entry/exit evidence, including typed handler-class evidence, but its reaching value remains
approximated. The current `outside_provider_model` result is the conservative boundary, not a
missing flag to clear. Dropping only exit-region approximation would not discharge the value
obligation. The next S1/S2 work is an occurrence-specific source parameter-identity certificate
with cited binding/reference/syntax evidence and matching-origin discharge, retaining raw flags
and all uncertified alternatives. The [forward plan](../../plans/behavioral-model-forward-plan_2026-09-24.md)
and [STATUS](../../../STATUS.md) own this incomplete qualification boundary.

| Judgment/gate | Current bounded verdict | Basis |
|---|---|---|
| A1 | satisfied for the chosen repair | Typed meaning stays with schema; source composition, admission and serving retain their owners |
| A2 | satisfied for the bounded repair | Typed transfer/scope and exact source-origin keys preserve meaning; row-local fidelity survives receiver and transfer admission |
| A3 | satisfied for the bounded repair | Explicit open alternatives survive adapter/consumer composition independently of sibling positive identity paths |
| G1/G3/G4/G5 | no new blocker established outside listed findings | Shared reconstruction, explicit projections and fresh-generation migration retained |
| G2/G6; CI-G1 | no remaining soundness blocker established in this slice | Conservative receiver proof and explicit refusal propagation address F05/F07; the handler-value qualification gap remains explicitly unknown |
| G7/G8; CI-G2/CI-G3 | scoped | Attributed fixture receipts only, no performance claim or new library/oracle-input path; complete native and integrated serving qualification remain open |

**Bounded decision: Accept the conservative transfer checkpoint at source-inspection strength
after F04–F07 corrections, with the attributed partial focused result above.** Flow identity
preserves claim/grade separation. The native handler positive control is still failed; there is
no clean 57-case compiler-86 receipt. Initial target acceptance and earlier S1/S2/FORMAT 9 slices
remain separate. Full gates, pilot, complete S5/RCA repair and integrated Stage 3 acceptance
remain `not_run` in this review.

## 18. Bounded implementation follow-up: source parameter identity

**2026-09-27 · change/conformance · dirty compiler-87 slice · source inspection.** Reviewed
`cpg-schema::parameter_identity`, `lctx-analytics::parameter_identity`, direct finite admission
and origin coverage, core publication/reconstruction, and FORMAT 9/native certificate loading.
This addresses §17's handler-value gap only. No production edits or tests were performed by
this reviewer; executed evidence below is attributed to the author.

**Implemented, inspected:** the schema owns the certificate identity and shared tuple admission;
the pure analytics producer owns lexical proof; core acquires and reconstructs the rows; native
loading checks the served contract. The proof requires an exact bare return-name occurrence,
one same-scope parameter binding and a unique lexical name, an undecorated synchronous owner,
and an unapproximated raw value expression. Deletion, nonlocal mutation and generator hazards
withhold certificates, including hazards in nested declaration chains. This is a local binding
proof, not general alias analysis or a relaxation of provider fidelity.

Only direct seeds can cite a matching certificate. Entry/exit admission remains mandatory;
provider reaching and exit-region flags remain unchanged. The summary's nonapproximate result
comes from independent source evidence. Coverage discharges the approximation reason only for
the certified snapshot/function/parameter/source/condition/origin tuple, and only certificates
cited by admitted summary steps are published. Foreign origins and uncertified siblings remain
open. Core validation recomputes the certificates and summary products from source. The retained
tuple constructor cannot supply certificates and therefore refuses a certificate-dependent proof.

<a id="F08"></a>**F08 — Native certificate closure must include unreferenced rows.**
The initial loader validated a `SourceParameterIdentity` step when present, but did not require
every loaded certificate to be cited. Removing that proof step and making the remaining
ordinals dense could leave an orphan certificate while the summary loaded positively.
**Correction inspected:** each cited ID must resolve and pass shared tuple admission before it
enters the cited set; equality with the loaded certificate set rejects orphans. More than one
identity step in one summary is rejected. Missing/foreign certificates and removed/duplicated
steps with dense renumbering are separate IPC controls, so ordinal-gap validation cannot mask
the missing closure check. The certificate table is registered, and publication reconstruction
also rejects a removed certificate table's contents. Owner: native FORMAT 9 evidence admission;
current work disposition remains forward-plan S1/S2/S6. The correction reuses the source-owned
lexical proof rather than duplicating that analysis in native loading.

| Judgment/gate | Bounded verdict | Basis |
|---|---|---|
| A1 | satisfied | Lexical proof, semantic contract, persistence and loading have separate owners; publication shares reconstruction |
| A2 | satisfied | The independent certificate names its exact occurrence and does not erase raw fidelity or replace completion evidence |
| A3 | satisfied for this slice | The source-to-serving chain retains matching tuple admission and certificate closure, including missing proof rows |
| G1/G3/G4/G5 | no additional blocker established | Existing identity, source reconstruction and bounded finite proof mechanisms are reused |
| G2/G6; CI-G1 | no remaining blocker established in this slice | Present references and reverse certificate closure are checked; uncertified origins retain their fidelity boundary |
| G7/G8; CI-G2/CI-G3 | scoped | No new library, evaluation-input path or performance claim; focused receipts do not certify integrated serving |

**Bounded decision: Accept at source-inspection strength after F08 correction.** Focused evidence
is recorded separately below. No full S1/S2/S6 completion or integrated Stage 3 acceptance is
credited.

**Attributed focused receipt, 2026-09-27:** after the author's reviewed schema/codebook/rule
snapshot acceptance and editable native rebuild (`uv sync --frozen --reinstall-package
lctx-semantics`, reported `passed`), the inspected final log
`/tmp/lctx-stage3-parameter-identity-final.log` records:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-schema -p cpg-core -p lctx-analytics --lib \
  --test contracts --test codebooks --test bundle --test compile \
  -E 'test(summaries::) | binary(contracts) | binary(codebooks) | test(finite_depth_and_unsupported_refusals_reach_the_native_response) | test(transfer_alternatives_keep_conditions_verdicts_and_receiver_boundaries) | test(serving_schema_digests_are_the_shared_known_answers)' \
  --status-level fail --final-status-level fail
```

Outcome **failed: 43 cases run, 42 passed, 1 failed, 156 skipped** (11.253 seconds test time).
The native fixture progressed past the positive handler identity and four certificate-closure
mutations, then failed its `summary_proof_boundary` positive expectation: 63 finalizer passes,
raw identity and the new required certificate total 65 steps, above the unchanged 64-step cap.
The author resized the boundary fixture to 62 passes (64 total) and its over-cap companion to
63 (65 total), preserving the production cap and admission rule. The inspected targeted log
`/tmp/lctx-stage3-parameter-identity-cap-control.log` records the following rerun:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-core --test bundle \
  -E 'test(finite_depth_and_unsupported_refusals_reach_the_native_response)' \
  --status-level fail --final-status-level fail
```

Outcome **passed: 1 case, 6 skipped** (10.048 seconds test time). This completes the fixture's
source→Delta→native handler identity, missing/foreign/removed/duplicate certificate controls,
64/65-step boundary controls and publication rejection of missing source certificates. The
author also records `uv run --no-sync pytest python/lctx_mcp/tests/test_digest.py -k
expected_schemas` as **passed**; the new known-answer digest is the certificate schema addition.
The initial 43-case command retains its failed outcome; the targeted rerun resolves its sole
failure without a cap increase. F08 is corrected at source-inspection strength with this focused
fixture evidence. Full S1/S2/S6, all-channel summaries, RCA repair, clean-wheel qualification,
full gates/pilot and integrated Stage 3 acceptance remain open.

## 19. Bounded implementation follow-up: typed current FCA/RCA and handoff support

**2026-09-27 · change/conformance · source inspection with attributed focused receipts.**
This review covers the compiler-88/template-20 repair of existing §9.3/§9.6 consumers under
ADR-0058: typed attributes, attributed object incidences, Pass C, synthesis, declared facets,
publication validation and the FORMAT 9 support projection. It does not introduce behavioral
FCA features, registry membership or Stage 5 execution. The
[forward plan §6](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns W10's current disposition; this section supplies bounded review evidence.

**Implemented, source-inspected:** [the schema contract](../../../crates/cpg-schema/src/concept_attributes.rs)
owns typed identity, rendering and retained-support selectors. Analytics maps sorted attribute
IDs into the existing FCA kernel; core reconstructs observations; Python checks projected
closure. The FCA algorithm and limits remain unchanged. This is domain-contract repair using
existing relational/FCA capabilities, with no new generic framework.

<a id="F09"></a>**F09 — Shared attribute meaning must not depend on presentation or its object.**
An initial consumer-formal key split shared TakesFrom attributes by object; unchecked display
could alter facets, and implications lacked supporter objects. The correction keeps formals
in incidence evidence, reconstructs display from source, and records premise-extent supporters.
Attribute/finding IDs exclude display; incidence IDs retain object, fact, sites, edges and
formal. Stage F, Pass C and facets consume typed variants; English-prefix classifiers are
deleted. Focused controls distinguish modality/phase and structural type identity, preserve
label invariance, and reject forged source display.

<a id="F10"></a>**F10 — Retained handoff pairs need their own evidence contract.**
Sibling usage calls cannot be delegation-chain `Witnesses`. Pass C now uses a typed
`HandoffAttribute`, complete ordered site pairs and one formal. Its independent support does
not enter an RCA-disabled FCA context. Shared requirements/selectors constrain incidence to
retained pairs/formals or FCA extent objects. Publication and serving reject orphan members
and missing pair support; serving validates evidence shape by attribute kind, including
combined edge/fact deletion at either endpoint. Both facts retain provenance with truthful
`fact_only` resolution. Candidate endpoints render as candidate handoff pairings.

Source reconstruction validates retained attributed support, not lattice/context completeness.
Support projections retain the 100,000-row refusal boundary; selection does not become an
exhaustive behavioral claim.

| Judgment/gate | Bounded verdict | Evidence and limit |
|---|---|---|
| A1 — Localize change | satisfied at source-inspection strength | Schema owns meaning/selectors; analytics owns context/grouping; core owns acquisition/publication; Python owns protocol checks |
| A2 — Encode meaning structurally | satisfied after F09/F10 corrections | Typed keys, incidence-local occurrences, source-checked display and complete retained pair shape replace implicit label/path meaning |
| A3 — Extend through composition | satisfied for current consumers | The same catalog feeds FCA, independent Pass C support, facets and rendering; ordinary observations do not add semantic classifiers |
| G1/G2/G3/G6; CI-G1/CI-G2 | no remaining blocker established in this slice | Shared source reconstruction and selectors, typed endpoint fidelity and explicit malformed-evidence refusals; full source-span/native semantic-query closure remains separate S6 work |
| G4/G5/G7/G8; CI-G3 | scoped, no new blocker established | Pure analytics and pinned projections retained; bounded support, attributed receipts, no new evaluation-input path or generic engine; no performance or whole-product claim |

**Attributed focused receipt, 2026-09-27:** the author ran the following command. This reviewer
inspected its source coverage and `/tmp/lctx-stage3-typed-attributes-final.log`, and ran no tests.
The target-directory override reuses this checkout's established cache, as explained in STATUS.

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
LCTX_PY_FIXTURE=/home/paul/library-context/build/py-fixture \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p lctx-analytics -p cpg-schema -p cpg-core --lib \
  --test contracts --test codebooks --test analysis --test bundle --test syntax \
  -E 'test(concepts::) | binary(contracts) | binary(codebooks) | test(concepts_come_from_each_seeds_structural_scope) | test(fca_scopes_are_nodes_and_state_only_their_own_apis) | test(variants_add_relational_attributes_and_layers) | test(typed_attributes_keep_source_evidence_through_serving) | test(typed_handoff_pair_support_survives_serving_without_becoming_a_call_chain) | test(handoffs_and_usage_patterns_come_from_official_code) | test(pass_c_and_usage_patterns_are_identical_across_location_and_module_order) | test(serving_schema_digests_are_the_shared_known_answers) | test(writes_the_python_fixture_generation)' \
  --status-level fail --final-status-level fail
```

Outcome **passed: 29 cases, 181 skipped** (9.786 seconds), covering FCA controls, typed identity,
reversed input, real source→Delta→bundle→Python output and the F09/F10 refusal cases.
Intermediate commands remain **failed**: SQL errors, including unavailable `mod()`, were
corrected; schema/codebook/rendering snapshot differences were read and accepted before this
final no-update run.

The author's `uv run --no-sync pytest python/lctx_mcp/tests/test_server.py
python/lctx_mcp/tests/test_digest.py` is **passed: 15 cases** (1.17 seconds), recorded in
`/tmp/lctx-stage3-typed-attributes-python.log`. The separate `test_digest.py -k expected_schemas`
check is reported **passed**. `uv run --no-sync pyrefly check
python/lctx_mcp/src/lctx_mcp/generation.py python/lctx_mcp/src/lctx_mcp/server.py` is **passed:
0 errors, 1 warning**, confirmed by `/tmp/lctx-stage3-typed-attributes-pyrefly.log`.

**Bounded decision: Accept the current typed FCA/RCA and Pass C repair at source-inspection
strength, supported by the attributed focused receipts above.** F09/F10 are corrected within
this slice. This accepts neither all S5 coverage/discharge nor enclosing Stage 3. Synchronous
context completion, remaining model activation and channel composition, full S6 semantics and
evidence, independent S7 challenges and S8 qualification remain open. Full `just test-all`,
fresh `just pilot`, clean-wheel and all-techniques/cost qualification were **not_run** here.

## 20. Bounded target follow-up: synchronous class protocols

**2026-09-27 · design/target · document and source-interface inspection.** The subject is
[ADR-0059](../../adr/0059-synchronous-context-protocols.md) and its amended §B5/§9.9/§11.3 owners.
**Proposed target: Accept scoped. Implementation remains Proposed.** This additive decision
settles class-protocol applicability under ADR-0057/0058; it does not replace observed-call
models or broaden Stage 3 into arbitrary protocol interpretation or Stage 5 execution.

The target separates pinned class identity and constructor signatures from authored runtime
transitions and provider method observations. This addresses the actual interface mismatch:
the provider omits exits and reports inherited stub entry for `suppress`; neither fact should
be relabelled as an observed runtime method. Class assertions remain synthetic model evidence.
The existing pure completion owner evaluates ordered items and unwinds entered managers in
reverse; the catalog selects typed transitions, core reconstructs source admission, and native
consumers share structural proof admission. A second runtime interpreter or model-name-specific
classifier is unnecessary. Future instances using supported transitions add declarations;
new transitions deliberately extend the contract.

| Judgment/gate | Target verdict | Basis and qualification boundary |
|---|---|---|
| A1 — Localize change | satisfied | Existing catalog, completion, acquisition/publication and native owners retain their responsibilities |
| A2 — Encode meaning structurally | satisfied | Class, constructor roles, provider observations, manager occurrence, entry result and pending outcome remain distinct; phase and evidence are explicit |
| A3 — Extend through composition | satisfied | Pinned declarations compose with argument evaluation, existing frame completion and shared admission; no generic protocol engine |
| G1/G2/G3/G6; CI-G1/CI-G2 | target route satisfactory; implementation open | Authored actions do not invent call facts; source reconstruction and native missing/foreign/swapped-proof refusal are required |
| G4/G5/G7/G8; CI-G3 | scoped | Pure semantics, explicit unknowns and existing bounded mechanisms; no performance, production behavior or evaluation claim established |

The initial implementation must preserve the ADR's important boundaries: successful entry
registers the current exit before `as` assignment; failed entry does not. Later evaluation,
construction or assignment failure unwinds earlier successful entries. Suppression changes
only Raise; exit failure replaces the pending outcome before outer cleanup. Manager identity
never becomes the entered value, and returning an `as` binding needs a separate transfer
certificate. Exact builtin suppression and handler matching may share pinned hierarchy facts
without sharing their semantic meaning. Unknown bodies, groups, custom matching, aliases and
deferred execution remain open. Normal completion closes no unrelated effect/resource channel.

**Attributed interface receipt:** the author's corrected
`uv run --no-sync python docs/design_review/evidence/2026-09-27_sync-contexts/source_probe.py`
completed without unavailable queries in the inspected `/tmp/lctx-stage3-context-source.log`.
The [probe](../evidence/2026-09-27_sync-contexts/README.md) exercises real extraction of generated
source; it does not execute that source or prove admission. It records separate constructor
phases, exact ranges/receiver identities, bindings, signatures and WithItems. Notably, bare
`TypeError` argument references also have provider `new`/`init` observations at identifier
sites; source admission must not treat those as executed constructor calls. WithItem and call
ranges differ when an `as` target exists, and entry's implicit-dunder flag is false in these
observations. Matching must use explicit source roles, not those shortcuts.

The author reports `just adr index` and `just adr lint` **passed**, with 31 records; these are
documentation receipts only. Implementation acceptance requires independent pinned CPython
challenges and source→Delta→native controls for partial entry, assignment failure, reverse
cleanup, suppression/nonmatch, preserved abrupt outcomes, exit replacement, nested active
exception restoration and malformed certificates. No tests or production edits were performed
by this reviewer. S2b/S3a implementation, all S5/S6 obligations and integrated Stage 3 acceptance
remain open; §19's earlier bounded implementation acceptance is unchanged.

## 21. Bounded implementation follow-up: synchronous lifecycle and return obligations

**2026-09-27 · change/conformance · source inspection with attributed focused receipts.**
This reviews the compiler-89/catalog-4/extractor-33 implementation of ADR-0059's initial
`nullcontext`/`suppress` lifecycle subset. §20 remains target acceptance; this section accepts
only the implementation boundary described here. The forward plan §3.0 S2b/S3a owns remaining
work and §6 owns scheduled finding disposition.

Schema owns typed protocol/site identity and shared proof admission; the catalog owns pinned
runtime assertions; analytics owns source binding and completion; core acquires, publishes and
reconstructs; native loading checks immutable commitments. Provider constructor phases remain
observations, distinct from authored entry/exit meaning. A new declaration using the supported
transitions changes the catalog and its controls; a new transition deliberately changes the
typed contract and completion owner. Neither case requires a serving-side Python interpreter
or a library-name classifier.

<a id="F11"></a>**F11 — Balanced lifecycle steps alone do not prove this return's obligations.**
`completion_proof.rs` now commits the function, return site, entry/exit conditions, counts and
ordered kind/evidence/condition digests, including empty obligations. Source-derived certificates
are mandatory on the production IPC path. Explicit implication permits stronger entry scope;
finalizer scope is retained. Admission checks the return anchor, exact prefix and cleanup slice,
and refuses context evidence outside those obligations. Source finite admission applies the
same checks before coverage and removes callers of refused witnesses; publication reconstructs
certificates. Native loading also requires exact cited certificate closure. This corrects the
earlier native-only lifecycle check and missing match/condition/whole-group evidence gap.
The final correction preserves typed condition-budget reasons through shared
`ProofAdmissionError` and finite refusal instead of relabelling them `MissingEvidence`.

<a id="F12"></a>**F12 — Unsupported constructor binding must not become exact TypeError.**
The source binder distinguishes bound, demonstrated mismatch and unknown. Incomplete or
unsupported signatures, omitted defaults without authored meaning and bounded binding refuse
admission. An explicit guard keeps the shared binder's greater-than-128-formal boundary unknown;
only a proved argument-shape mismatch produces the modeled construction TypeError. Complete
overload sets and exact allocation/initialization observations remain separate premises.

The inspected completion code evaluates arguments in source order, registers a successful entry
before assignment, unwinds entered managers in reverse, suppresses only Raise, preserves other
abrupt outcomes and propagates replacement exceptions to outer exits. It restores the previous
active exception. These models assert normal entry; broader failing-entry or opaque argument
evaluation remains unknown. The manager is never substituted for the entered value.

| Judgment/gate | Bounded verdict | Evidence and limit |
|---|---|---|
| A1 — Localize change | satisfied at source-inspection strength | Existing schema, pure semantics, acquisition/publication and native owners retain distinct responsibilities |
| A2 — Encode meaning structurally | satisfied after F11/F12 corrections | Class/constructor/site roles, ordered return obligations, condition scope and unknown-versus-mismatch remain explicit |
| A3 — Extend through composition | satisfied for the two supported transitions | Catalog declarations compose with existing evaluation, completion and shared admission; new transitions require a deliberate extension |
| G1/G2/G3/G6; CI-G1/CI-G2 | no remaining blocker established in this slice | Source reconstruction and native closure reject missing, foreign, reordered and omitted obligations without fabricating provider calls |
| G4/G5/G7/G8; CI-G3 | scoped, no new blocker established | Pure bounded evaluation and independent generated runtime controls; no performance, live embedding or whole-product qualification |

**Attributed focused receipt:** this reviewer inspected the source and
`/tmp/lctx-stage3-context-final2.log`; the author ran:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-schema -p lctx-analytics -p cpg-core --lib \
  --test contracts --test codebooks --test compile --test bundle \
  -E 'test(completion_proof::) | test(summary_contract::) | test(context_protocol::) | test(summaries::finite::) | test(completion::) | binary(contracts) | binary(codebooks) | test(context_protocols_bind_class_and_constructor_roles_independently) | test(a_generation_rebuilds_to_the_same_bytes)' \
  --status-level fail --final-status-level fail
```

Outcome **passed: 57 cases, 147 skipped** (10.544 seconds). Coverage includes ordered certificate
digests, explicit condition specialization, foreign scope, typed implication/binding limits,
real extraction→Delta→FORMAT 9/native paths, missing/duplicate certificate and lifecycle support,
deleted match evidence, omitted groups, same-function site swaps, and source certificate forgery.
The source/native case separately runs 14 finite generated CPython 3.14.7 programs, including
monitored partial construction, assignment-before-cleanup and reverse exits; it does not execute
the fixture or use compiler outputs to choose expected runtime outcomes. Fake embeddings serve
fixture storage only. Expected schema snapshots were read and accepted before the no-update run;
the earlier snapshot-failing command remains **failed**. The author subsequently removed an
unused import warning; that edit changes no semantics.

The author then rebuilt the current native module with
`CARGO_TARGET_DIR=/home/paul/library-context/target uv sync --frozen --reinstall-package lctx-semantics`
(**passed**, `/tmp/lctx-stage3-context-native-sync3.log`) and reran the source/Delta/native case:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-core --test compile \
  -E 'test(context_protocols_bind_class_and_constructor_roles_independently)' \
  --status-level fail --final-status-level fail
```

Outcome **passed: 1 case, 30 skipped** (5.905 seconds), confirmed by this reviewer's inspection
of `/tmp/lctx-stage3-context-current-native.log`. This establishes the focused current-native
replay after the rebuild; the earlier 57-case receipt alone is not that rebuild receipt.

**Bounded decision: Accept the lifecycle/completion implementation at source-inspection
strength, supported by the focused receipt.** F11/F12 are corrected within this slice. Returning
an `as` value still lacks entry-result identity evidence; general failing-entry models, broader
protocols, callback/resource fates, named-handler cleanup, remaining channels/coverage, complete
S6 evidence and integrated Stage 3 remain open. Normal completion proves no unrelated channel
complete. Full `just test-all`, fresh `just pilot`, clean-wheel and all-techniques/cost
qualification were **not_run**. This reviewer ran no tests and changed no production files.

## 22. Bounded implementation follow-up: context entry-value identity

**2026-09-27 · change/conformance · source inspection with attributed focused receipts.**
Compiler 90 adds the narrow entry-result proof left open in §21: an immutable current parameter
is the explicit modeled entry argument, and a bare return reads the unique `WithTarget` inside
that exact active body. General aliases and returns after a completed context remain outside
this slice. ADR-0059's owner/authority split is unchanged.

Schema [`context_value.rs`](../../../crates/cpg-schema/src/context_value.rs) owns identity and
shared admission; analytics' corresponding producer proves both lexical reference/resolution/
binding chains and return ancestry. Core reconstructs the source certificate at publication;
native loading checks its hash, scope, exact argument expression and lifecycle references.
The proof requires entry, successful target assignment, value witness, matching exit and return
anchor in that order, with the anchor's original condition. Existing return commitments still
prove predecessor and cleanup obligations independently. The entered value never denotes the
manager merely because it is a context object.

<a id="F13"></a>**F13 — Completion cannot substitute for a value basis.** Removing both the new
certificate and its witness must not leave an admitted base summary. Shared `admit_base_value`
now requires exactly one raw or context value basis, its original condition and a later return
anchor; the raw basis must cite that summary's source flow. Source and production IPC consumers
both call it. Missing, foreign, duplicate and uncited context certificates are refused. An
internal `BaseReturnBasis` distinguishes raw identity from source/model identity; synthesized
coordinates copy actual origin fidelity, with missing, ambiguous or budgeted origins withheld.
Raw provider nonidentity/call-crossing flags are preserved, and the new certificate does not
discharge complete-origin coverage. Full native verification of a raw fact's semantic meaning
remains S6 work; this structural check is not that broader evidence closure.

| Judgment/gate | Bounded verdict | Basis and qualification boundary |
|---|---|---|
| A1 — Localize change | satisfied at source-inspection strength | Source proof, typed admission, mechanical acquisition and immutable loading retain their existing owners |
| A2 — Encode meaning structurally | satisfied after F13 correction | Manager, entered value, raw transfer, independent source/model basis and completion obligations remain distinct |
| A3 — Extend through composition | satisfied for this case | The new value certificate composes with existing site/protocol and return commitments; it does not introduce general alias interpretation |
| G1/G2/G3/G6; CI-G1/CI-G2 | no remaining blocker established in this slice | Source reconstruction, active-site ordering, shared base admission and cited-set closure; complete raw-fact/native support remains open |
| G4/G5/G7/G8; CI-G3 | scoped, no new blocker established | Bounded inputs and proofs, preserved raw fidelity, independent opaque-object identity challenge; no integrated or performance claim |

**Attributed focused receipt:** the author ran the command below. This reviewer inspected the
source and `/tmp/lctx-stage3-context-value-final.log`, but ran no tests.

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 LCTX_WRITE_KNOWN_ANSWERS=1 \
LCTX_PY_FIXTURE=/home/paul/library-context/build/py-fixture \
cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib \
  --test contracts --test codebooks --test compile --test bundle \
  -E 'binary(contracts) | binary(codebooks) | test(context_value::) | test(summary_contract::) | test(summaries::finite::) | test(context_protocols_bind_class_and_constructor_roles_independently) | test(serving_schema_digests_are_the_shared_known_answers) | test(writes_the_python_fixture_generation) | test(a_generation_rebuilds_to_the_same_bytes)' \
  --status-level fail --final-status-level fail
```

Outcome **passed: 45 cases, 160 skipped** (9.789 seconds). This command enabled known-answer
regeneration; schema snapshots used `INSTA_UPDATE=no`. The initial selection remains **failed:
38 passed, 3 expected schema/codebook migration failures**, subsequently read and accepted.
The controls admit positional/keyword arguments and the correct formal among two inputs; they
withhold rebinding, deletion, nested nonlocal mutation, omitted/default entry values, suppressor
entry, completed sibling contexts, opaque predecessors and overridden returns. Structural
controls challenge assignment/value/exit order and return scope. Publication rejects forged
target-binding evidence; native IPC rejects certificate/witness omission and malformed support.

The author rebuilt the editable native module using §21's `uv sync --frozen
--reinstall-package lctx-semantics` command with the same checkout target (**passed**,
`/tmp/lctx-stage3-context-value-sync2.log`), then reran §21's targeted source/context command:
**passed: 1 case, 30 skipped** (6.796 seconds), inspected in
`/tmp/lctx-stage3-context-value-native-current.log`. That replay includes the independent
generated CPython challenge with a fresh opaque object, so identity is distinguished from
primitive equality/interning. It retains the real source→Delta→FORMAT 9/native checks; fake
embeddings serve fixture storage only.

**Bounded decision: Accept this active-site entry-value implementation at source-inspection
strength, supported by the focused receipts.** F13 is corrected within this slice. Broader
entry/alias domains, general failing entry, resource/callback fates, named cleanup, all-channel
composition and coverage, complete S6 support and integrated Stage 3 remain open. Full
`just test-all`, fresh `just pilot`, clean-wheel and all-techniques/cost qualification were
**not_run**. Only this review artifact was changed by the reviewer.

## 23. Bounded implementation follow-up: reached call inputs

**2026-09-27 · change/conformance · source inspection with attributed focused receipts.**
Compiler 92 adds source/Delta certificates for reaching a sole pinned bare `from`-import call
under entry to its synchronous, undecorated, nongenerator owner. The call must be the whole
expression, return value or assignment value. This is the initial invocation boundary in
[§9.9](../../design/sections/behavioral-analysis.md); action timing, callee outcomes and
resource/callback identity remain separate obligations under ADR-0057/0058/0059.

[`evaluation.rs`](../../../crates/lctx-analytics/src/evaluation.rs) owns the common prepared
call and source-ordered callee/argument evaluation. Normal-expression evaluation additionally
requires the pinned `normal_return` promise and retains `PrecedingCallNormal`; invocation
ends in `ModelInvocation` without that promise. Completion's existing entry walker proves
the independent preceding-statement group. Schema
[`call_execution.rs`](../../../crates/cpg-schema/src/call_execution.rs) owns both ordered
commitments and structural admission; core acquires, persists and reconstructs the exact
rows/steps at publication. A reached call before a later raise therefore remains represented,
while its return, effects and callback invocation are not invented. These are conditional
source/model claims, not runtime observations or complete coverage of all calls.

`SignatureParameter::from_context` now provides one normalization for ordinary calls and
context initializers: absent requiredness means an empty variadic slot only for a `List`
signature's `VarPositional`/`VarKeyword`. A fixed formal with absent requiredness still refuses.
The shared binder does not acquire general variadic collection support. Non-total calls with
omitted ordinary defaults retain `DefaultUnavailable`.

<a id="F14"></a>**F14 — Oversized refusal rows must remain admissible.** The initial schema
checked the positive 128-argument cap before its refusal branch; completion retained the actual
oversized count when clearing the proof, producing a refusal that admission still rejected.
The schema now applies the cap only to positive entries. Analytics records append-only
`InvocationArgumentLimit` (code 34), and refused entries preserve the actual source count with
empty proof groups. Shared and real source/Delta controls admit the 130-argument refusal and
reject its conversion to a positive entry. **Corrected within this slice.** The forward plan's
[§3.0 checkpoint](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns current scheduled disposition.

| Judgment/gate | Bounded verdict | Basis and limit |
|---|---|---|
| A1 — Localize change | satisfied at source-inspection strength | Binder/evaluator, entry semantics, schema admission and persistence retain distinct existing owners; signature normalization is shared |
| A2 — Encode meaning structurally | satisfied after F14 correction | Invocation, normal return, ordered obligations and typed refusal remain distinct; raw provider facts are unchanged |
| A3 — Extend through composition | satisfied for this boundary | Whole-call reach composes the existing expression and statement kernels; future actions must add their own trigger/outcome/identity premises |
| G1/G2/G3/G5; CI-G1 | pass for this slice at source-inspection strength | Source reconstruction and exact commitments reject incomplete support; positive caps do not invalidate refusal records |
| G4/G8; CI-G3 | pass for this slice at source-inspection strength | Pure semantic owners are reused; generated runtime challenges remain evaluation inputs only; no generic replacement framework is introduced |
| G6/G7 | pass for the changed invocation boundary; enclosing summary conformance unresolved | Normal-expression completion still requires its separate promise; the encountered modeled-identity regression below remains open |
| CI-G2 | not applicable to new served claims yet | These tables have no native export/action consumer in this slice; full tuple and evidence closure must precede serving |

**Attributed focused receipt:** the author ran the following command; this reviewer inspected
the source and `/tmp/lctx-stage3-call-entry-tests5.log` and ran no tests.

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-schema -p cpg-core -p lctx-analytics --lib \
  --test contracts --test codebooks --test compile \
  -E 'binary(contracts) | binary(codebooks) | test(call_execution::) | test(evaluation::) | test(completion::) | test(context_protocol::) | test(call_execution_proves_reached_inputs_without_inventing_callee_completion)' \
  --status-level fail --final-status-level fail
```

Outcome **passed: 34 cases, 166 skipped** (4.949 seconds). Controls cover source argument and
statement order, fallible call entry without normal return, omitted defaults, skipped/deferred/
nested cases, proof deletion/substitution/reordering, and Delta publication rejection of missing
steps. Expected schema/codebook migrations were read and accepted by the author before this
no-update run. The source test invokes the independent 13-program CPython 3.14.7 monitoring
challenge; `uv run --no-sync python docs/design_review/evidence/2026-09-27_call-entry/runtime_oracle.py`
also **passed**, recorded in `/tmp/lctx-stage3-call-entry-runtime.json` and the
[retained evidence](../evidence/2026-09-27_call-entry/README.md). Generated workers execute no
analyzer fixture and distinguish a reached fallible call from its failed return and callback
registration from callback invocation. Fake embeddings serve fixture storage only.

**Encountered adjacent regression remains open.** The author rebuilt the editable native
module with `CARGO_TARGET_DIR=/home/paul/library-context/target uv sync --frozen
--reinstall-package lctx-semantics` (**passed**, `/tmp/lctx-stage3-call-entry-sync.log`) and ran:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-core --test compile --test bundle \
  -E 'test(pinned_identity_models_require_and_publish_their_real_formals) | test(finite_depth_and_unsupported_refusals_reach_the_native_response)' \
  --status-level fail --final-status-level fail
```

Outcome **failed: 1 passed, 1 failed, 37 skipped** (11.402 seconds), inspected in
`/tmp/lctx-stage3-call-entry-native-regression.log`. The existing finite-depth/native boundary
case passed; `pinned_identity_models_require_and_publish_their_real_formals` failed because
`framed_modeled_identity` has no positive modeled finalizer path (`OutsideProviderModel`,
code 10). The 34-case selection omits that test and does not supersede this failure. Inspection
confirms that `summaries::finite::push_finite_path`'s blanket approximation refusal already
exists in the preceding commit, but no clean baseline run establishes when the regression
began. The forward plan's §3.0 retains the S5 repair: an occurrence-specific independent
source/model identity proof, with raw approximation flags and the positive expectation
preserved. Normal argument evaluation alone cannot discharge value identity.

**Bounded decision: Accept the source/Delta call-input certificate implementation at
source-inspection strength, supported by the focused receipt.** F14 is corrected. Dynamic
guards, nested expression reach, active contexts and calls after completed context prefixes
remain withheld; the current condition is unconditional under owner entry. Native export and
full evidence closure, action/outcome timing, resource/callback fates, all-channel summaries and
coverage, the named modeled-identity regression and integrated Stage 3 remain open. The native
rebuild does not certify a new call-execution consumer. Full `just test-all`, fresh `just pilot`,
clean-wheel and all-techniques/cost qualification were **not_run**. Only this review artifact
was changed by the reviewer.

## 24. Bounded implementation follow-up: modeled return identity

**2026-09-27 · change/conformance · source inspection with attributed focused receipts.**
Compiler 93 repairs §23's encountered modeled-finalizer positive regression for a direct,
single identity-model call returned from the current function. A separate lexical source
certificate discharges that occurrence's approximation obligation; it does not reinterpret
normal argument evaluation as value identity or modify raw provider flags.

[`lexical_identity.rs`](../../../crates/lctx-analytics/src/lexical_identity.rs) extracts the
existing immutable parameter-read checker for reuse by direct-return and modeled-return
producers. Unique same-scope binding, exact source name/range, undecorated synchronous owner,
and deletion/nonlocal/generator hazards remain enforced. The existing modeled seed adapter
owns the raw origin's one-call/argument association. Analytics' new
[`modeled_identity.rs`](../../../crates/lctx-analytics/src/modeled_identity.rs) independently
proves that exact selected argument is a bare current-parameter read and the call is the whole
return expression, then commits the existing `modeled_return_proof` rather than classifying
models again. Schema's corresponding contract binds the function, parameter, origin, source
flow, condition, return, call/model/rule and lexical evidence to that ordered group.

Finite admission replaces the group's raw marker with `SourceModeledIdentity` only for a
matching certified approximate occurrence. Mandatory entry/exit completion remains independent;
publication reconstructs the certificate, and the FORMAT 9/native reader shares its hash,
scope, group and cited-set checks. Missing, foreign, duplicate, reordered or uncited support
refuses. General approximate modeled chains and assignments remain withheld; complete-origin
coverage is unchanged. Full semantic verification of cited raw facts remains S6 work.

<a id="F15"></a>**F15 — Required value evidence cannot depend on which evidence survives.**
The first admission helper inferred a modeled obligation from any `ModelRule` in the whole
proof. This both rejected a valid context-entry-value return with a modeled predecessor and
allowed omission of the complete modeled group. Shared admission now follows `path_depth`:
depth zero retains the independent base-value owner and prohibits modeled certificates;
callee-based paths retain their separate callee proof; a positive-depth path without a callee
requires model rules and exactly one raw/modeled value basis. Removing the entire modeled
group and certificate therefore cannot leave completion alone as a value proof. The real
context-value/predecessor control and native complete-group deletion control passed.
**Corrected within this slice.** The [forward plan §3.0](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns current scheduled disposition and the remaining S5/S6 obligations.

| Judgment/gate | Bounded verdict | Basis and limit |
|---|---|---|
| A1 — Localize change | satisfied at source-inspection strength | One lexical-read checker serves two occurrence-specific producers; source acquisition/publication and immutable native loading retain their existing roles |
| A2 — Encode meaning structurally | satisfied after F15 correction | Lexical identity, model transfer, raw fidelity and return completion have separate premises; value obligations survive evidence omission |
| A3 — Extend through composition | satisfied for direct single-call returns | Existing model/binding and completion proofs compose with the new source certificate; no general alias or second model interpreter is introduced |
| G1/G2/G3/G5/G6; CI-G1 | pass for this slice at source-inspection strength | Shared admission and source reconstruction preserve occurrence scope, bounded proofs and typed fidelity; adjacent context/default controls pass |
| G4/G7/G8; CI-G3 | pass for this slice at source-inspection strength | Pure owners are reused, runtime challenges are independent and finite, and broader capability claims remain explicitly open |
| CI-G2 | pass for certificate closure in this slice; full raw-fact semantics unresolved | Native checks exact certificate/group references and omission; it does not independently reconstruct Python lexical or model semantics |

**Attributed focused receipts:** the author ran the commands below; this reviewer inspected
their logs, source, fixture correction and retained runtime evidence, and ran no tests. The
native module was first rebuilt with `CARGO_TARGET_DIR=/home/paul/library-context/target uv sync
--frozen --reinstall-package lctx-semantics` (**passed**, `/tmp/lctx-stage3-modeled-identity-sync.log`).

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-schema -p cpg-core -p lctx-analytics --lib \
  --test contracts --test codebooks --test compile --test bundle \
  -E 'binary(contracts) | binary(codebooks) | test(summaries::finite::) | test(pinned_identity_models_require_and_publish_their_real_formals) | test(context_protocols_bind_class_and_constructor_roles_independently) | test(finite_depth_and_unsupported_refusals_reach_the_native_response)' \
  --status-level fail --final-status-level fail
```

Outcome **failed: 39 passed, 1 failed, 167 skipped** (26.035 seconds),
`/tmp/lctx-stage3-modeled-identity-tests2.log`. The original modeled-finalizer positive test,
source certificate-removal/condition-root controls, context/native and finite/schema cases
passed. The remaining bundle test exposed accidental import edits in the expanded fixture,
which broke an existing function's indentation and altered a guarded import. Those edits were
restored; the final fixture diff contains only the intended top-level import and new functions.
The author then reran the affected case against the rebuilt native module:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target \
INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest run --release \
  -p cpg-core --test bundle \
  -E 'test(finite_depth_and_unsupported_refusals_reach_the_native_response)' \
  --status-level fail --final-status-level fail
```

Outcome **passed: 1 case, 6 skipped** (12.377 seconds),
`/tmp/lctx-stage3-modeled-identity-tests4.log`. This replay includes old fresh-default positives,
the context-value/model-predecessor case, six native certificate/group mutations, and seven
independent generated CPython 3.14.7 opaque-object identity challenges. Rebinding, deletion,
overriding finalizers and invoked nonlocal mutation remain negative controls. The standalone
`uv run --no-sync python docs/design_review/evidence/2026-09-27_modeled-identity/runtime_oracle.py`
also **passed**; [retained evidence](../evidence/2026-09-27_modeled-identity/README.md) records
the source/harness/pinned-library digests. No analyzer fixture was executed. After fixture
generation, `uv run --no-sync pytest python/lctx_mcp/tests/test_digest.py python/lctx_mcp/tests/test_native_semantics.py -q`
**passed: 16 cases** (`/tmp/lctx-stage3-modeled-identity-python3.log`). The three earlier expected
schema/codebook migrations were read and accepted before the no-update selection; earlier
failed selections remain failed receipts, not an aggregate all-green run.

**Bounded decision: Accept this occurrence-specific modeled identity implementation at
source-inspection strength, supported by the focused receipts.** F15 is corrected, and the
named direct/nested-finalizer regression from §23 is repaired without weakening its positive
expectation. Approximate modeled chains/assignments, broader source/model obligations,
call-action timing, resource/callback fates, all-channel summaries/coverage, complete S6 support
and integrated Stage 3 remain open. Full `just test-all`, fresh `just pilot`, clean-wheel and
all-techniques/cost qualification were **not_run**. Only this review artifact was changed by
the reviewer.

## 25. Target review: authored action triggers

**2026-09-27 · design/target · Proposed architecture, documentation/source-interface inspection.**
This reviews proposed [ADR-0060](../../adr/0060-action-triggers.md) and
[§9.9 Action triggers](../../design/sections/behavioral-analysis.md), under core 3.0, CI 1.1
and the repository binding. It accepts a bounded elaboration of ADR-0058, not an implementation
or assembled Stage 3 result. The existing call-input certificate (§23), normal-expression
evaluator and candidate effect/callback/resource relations are the inspected baseline.

The target has coherent ownership: authored models select trigger, descriptor and modality;
schema owns their typed meaning and shared admission; analytics combines exact candidate,
source binding, reached invocation and any required callee outcome; core acquires/publishes and
reconstructs that result; native consumers use the same admission. Candidate assessments retain
their original channel/descriptor references. They do not rename candidate observations into
execution evidence or create parameter/resource identities to fit a value-shaped table.

The decisive contract is **Invocation versus outcome**. Invocation is reached, bound entry to
the exact callee. Normal and Exceptional require that callee's independently proved outcome;
Finally accepts either proved callee exit, never an enclosing finalizer or pending caller
outcome. A proved invocation plus a Potential rule remains a model-qualified potential action.
It establishes neither occurrence of the action nor normal completion. Potential exception
rules cannot supply exceptional-exit evidence. Coverage remains an independent obligation.

**Subject distinction retained in this acceptance:** an authored subjectless effect is valid
(the existing unscoped I/O declaration is such a case); an authored required subject whose
source binding is unresolved is unknown. The declaration/candidate contract must distinguish
those states. A returned-resource action additionally requires Normal and independent
returned-value identity. Its call-expression id cannot stand in for a resource. Stored,
Registered, Forwarded and Invoked callbacks retain their different meanings and triggers.

The two simpler alternatives in the ADR are unsuitable: Normal-only activation loses partial
effects before failure; Invocation-only activation promotes normal-exit promises before their
premises hold. Extending the existing selector avoids a second policy/interpreter. Catalog 5's
effect selector is mandatory; existing codes 0–2 remain stable and Invocation appends at 3.
Remove any implicit effect-timing default as declarations migrate. New ordinary model rules
then add declarations and focused controls; genuinely new timing semantics revisit this ADR.
This is domain-specific composition over existing kernels, not a missing generic library.

| Judgment/gate | Target verdict | Basis and bounded consequence |
|---|---|---|
| A1 — Localize change | satisfied | Trigger selection and admission have one schema owner; acquisition, evaluation and serving do not independently classify model names or effects |
| A2 — Encode meaning structurally | satisfied | Trigger, callee outcome, modality, subject availability and resource identity remain distinct; unavailable evidence has an explicit unresolved result |
| A3 — Extend through composition | satisfied | Existing call-input and whole-expression normal proofs compose with candidate/rule bindings; no outcome assumptions or general precondition interpreter are added |
| G1/G2/G3/G5/G6; CI-G1 | pass for the target contract | Exact candidate/model/call/site/phase/condition and ordered evidence must agree; no positive action or invocation closes coverage |
| G4/G7/G8; CI-G3 | pass for the bounded target | Pure owners and explicit acquisition persist; proposed capability is labelled; independent partial-I/O challenges are required and never compiler inputs |
| CI-G2 | pass as a target obligation; implementation not assessed | The same candidate, trigger, subject and ordered evidence closure must reach immutable consumers before action serving is accepted |

The initial execution boundary is intentionally smaller than the selector vocabulary. Normal
uses the corresponding whole-expression normal proof for the same exact call. There is no
initial exceptional-outcome witness; unavailable Exceptional/Finally premises remain unresolved.
Returned-resource positives remain withheld until their identity premise exists. Current call
entry limitations, Stage 5 deferred execution, all-channel propagation and full S6 serving
support remain outside this initial implementation acceptance, which has not occurred.

Focused implementation acceptance must challenge a reached partial effect before failure,
the same action with failed argument evaluation, a reached call with an unproved Normal trigger,
subjectless versus unresolved required subjects, wrong call/model/phase/condition and omitted
or reordered evidence. It must retain potential modality, distinguish registration from
invocation, and reject a returned-resource claim without identity. Existing proof caps and
explicit refusals remain applicable; a positive result is not complete coverage. Independent
runtime challenges establish their named finite examples only. No new gate or review authority
is introduced; the [active plan §3.0](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
continues to own sequencing and closure.

**Decision: Accept the ADR-0060 target within these boundaries.** The author can mark the ADR
accepted and its owner “Accepted target; implementation Proposed.” This review establishes no
production action activation, native action consumer, resource identity, all-channel completeness
or Stage 3 closure. Product tests, full `just test-all` and fresh `just pilot` were **not_run**
for this documentation target review. Only this existing review artifact was changed.

## 26. Bounded implementation follow-up: authored action triggers

**2026-09-27 · change/conformance · Implemented; focused Tested as attributed below.**
This reviews compiler 94 / catalog 5 against ADR-0060 and §9.9, under core 3.0, CI 1.1
and the repository binding. The scope is pure admission and source/Delta publication of
candidate-backed effect/callback/resource assessments. FORMAT 9 is unchanged; native action
export, selection and semantic evidence closure are not accepted by this review.

The inspected owners are `cpg-schema/src/action.rs` (candidate identity, typed subject/schema
requirements and shared admission), `lctx-analytics/src/actions.rs` (pure composition), the
existing call-entry/normal-expression producers, and core `summaries::action_assessments`,
publication and `validate_actions`. Core acquires rows and reconstructs the producer; it adds
no model-name or timing classifier. Every existing candidate gets a positive or explicitly
unresolved assessment. This domain is candidate-relative: a provider-omitted, statically skipped
call does not acquire a synthetic candidate, and empty assessments do not imply coverage.

Admission retains the original channel, descriptor, authored modality and exact
model/rule/call/target/function/phase/condition association. Required subjects and runtime schema
sources require this call's unique bound raw argument; a genuinely subjectless effect needs no
invented source. Malformed validation-schema shapes and authored Candidate modality cannot
activate. Invocation consumes reached callee/argument evidence without claiming normal return.
Normal and Finally additionally require the same call's normal-expression proof, extending the
matching invocation in order and retaining the combined 64-step cap. Exceptional has no supported
outcome witness. Returned-resource candidates retain `resource_identity_unavailable`; a call id
does not become a resource identity. Shared admission establishes structural commitments;
upstream source reconstruction remains mandatory to establish the cited facts' meanings.

Catalog 5 makes effect timing mandatory and appends Invocation without renumbering. Potential
partial I/O/logging and completed serialization/compression keep different triggers; no target
was newly declared total-normal. Consequently, the production source fixture activates potential
I/O at reached calls, including before a later raise, while serialization/compression and
registration retain missing-outcome refusals. Positive Normal/Finally controls are synthetic
contract cases, not production model activation. Non-total calls with omitted defaults remain
conservatively withheld. No positive assessment changes independent channel coverage.

The relevant extension scenario is another rule using an existing trigger: its model declaration
and focused qualification feed the same candidate/admission/producer contracts. A new outcome
proof belongs to its existing semantic owner before actions consume it. No second evaluator,
binder, generic rule engine or new library dependency is needed. Adjacent RCA/Pass C inspection
found no effect-candidate consumer: current attributes cover declarations, direct raises, calls
and official-usage handoffs. Their call modality/InvocationPhase remains intact; adding effect
triggers there now would introduce behavioral FCA scope rather than repair this migration.

| Judgment/gate | Bounded verdict | Evidence and limit |
|---|---|---|
| A1 — Localize change | satisfied | Schema owns admission; analytics composes typed inputs; core acquisition/publication has no independent timing policy |
| A2 — Encode meaning structurally | satisfied | Candidate descriptors/modality, invocation, callee outcome, subject availability and resource identity remain separate; source reconstruction checks persisted results |
| A3 — Extend through composition | satisfied | Existing ordered call and normal-expression proofs compose without a second binder/evaluator; ordinary rule additions reuse that boundary |
| G1/G2/G3/G5/G6; CI-G1 | pass for the inspected source/Delta slice | Scope/binding/ordering mutations, refusal states and the 64-to-65-step boundary are challenged; Potential is never promoted to observed occurrence |
| G4/G7/G8; CI-G3 | pass for this slice | Pure production composition, bounded independent runtime challenges and explicit qualification limits; the adapter implements repository-specific contracts rather than a missing generic capability |
| CI-G2 | pass for inspected source publication; native action closure unresolved | Publication reconstructs candidates and their source proofs; action serving and full raw-fact semantic support remain S6 |

**Attributed receipts, inspected 2026-09-27; the reviewer did not run product tests.** The author
ran the following final selection with `INSTA_UPDATE=no` after reading and accepting the schema,
rule and append-only codebook migrations:

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target INSTA_UPDATE=no RUST_MIN_STACK=16777216 \
cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib \
  --test contracts --test codebooks --test compile \
  -E 'binary(contracts) | binary(codebooks) | test(models::) | test(actions::) | test(action_triggers_preserve_partial_io_and_withhold_unproved_outcomes) | test(call_execution_proves_reached_inputs_without_inventing_callee_completion) | test(validation_schema_candidates_keep_attribution_separate_from_subjects)' \
  --status-level fail --final-status-level fail
```

**passed:** 30 tests, 177 skipped, 5.694s (`/tmp/lctx-stage3-actions-tests6.log`). This includes
the real source/Delta action case and publication mutations removing assessments or invocation
steps; prior call-input/default boundaries; pure Normal/Finally success, unavailable outcomes,
foreign scope/binding, missing/reordered proof, shuffled-input invariance, malformed candidates
and cap controls. Earlier attempts encountered expected migrations and corrected fixture/test
controls, including a cap-test enum typo; this final receipt is not an aggregate of those runs.

The author's `uv run --no-sync python docs/design_review/evidence/2026-09-27_action-triggers/runtime_oracle.py`
and corresponding `uv run --no-sync ruff check` on that file **passed**. The retained
[nine-program receipt](../evidence/2026-09-27_action-triggers/README.md) independently distinguishes
partial writes before encoding failure, reached invocation with no writes, and calls prevented
by preceding failure or argument evaluation. It establishes those finite observations only;
runtime invocation does not establish arbitrary effect occurrence or complete coverage.

**Decision: Accept this bounded source/Delta implementation; no remaining in-scope blocker
identified.** All native action consumers, full raw-fact semantics, broader call/context/default
domains, positive resource/callback fates, all-channel propagation and complete coverage remain
open under the active plan. Stage 5 is unchanged. Full `just test-all`, fresh `just pilot` and
assembled Stage 3 acceptance were **not_run** for this slice. Only this review artifact was
changed by the reviewer.

## 27. Target review: pinned call default availability

**2026-09-27 · design/target · Proposed architecture; documentation/source-interface inspection.**
This reviews [ADR-0061](../../adr/0061-pinned-call-defaults.md) and the proposed §9.9
“Pinned call default availability” owner under core 3.0, CI 1.1 and the repository binding.
The inspected baseline is the common signature binder, `evaluation::prepare_calls`/`invoke`,
call-execution admission and the action consumer accepted in §26. This is target acceptance,
not implementation or qualification of new authored assertions.

Separating availability from `normal_return` corrects the ownership of a call-entry premise.
`call_defaults_available=true` is a universal assertion about the exact pinned callable's
omitted optional fixed formals across the admitted signature domain. Absence/false is unasserted,
not evidence that runtime defaults are absent. Qualification concerns already-created runtime
defaults; stub optionality alone is insufficient. The assertion supplies no value, truth,
mutable-value stability, resource identity, callback fate or normal outcome. Omitted subjects
remain unresolved in model argument bindings; no synthetic argument or call-time default
evaluation is introduced. The stated pinned-implementation assumptions exclude arbitrary
mutation of callable metadata; they must remain attached to the model claim.

The Boolean is sufficient for this universal promise. Named-default declarations would add
unneeded machinery for a target whose complete runtime/default interface has been qualified.
If a target's admitted overloads cannot be reconciled with that runtime interface, leave the
promise unasserted and revisit the recorded contract; do not use the Boolean to waive mapping
ambiguity. Requiredness, argument shape and all-signature agreement remain independent checks.
An empty variadic collection is not an omitted fixed default. Initial JSON/compression targets
must be individually qualified against their pinned implementation before authoring true;
this review does not infer a blanket standard-library guarantee from the existing runtime case.

**Mandatory proof detail for implementation acceptance:** reconstruct the complete omitted-fixed-
formal obligation domain from the admitted signatures and raw arguments, independently of which
witnesses remain. Require the availability model and exactly the corresponding omitted-formal
facts, retaining signature-specific identity when names repeat. Removing the entire availability
group, not only one witness, must fail admission. This follows the contract's “each omitted
formal” requirement and avoids repeating the evidence-dependent obligation defect in §24.
Availability evidence joins the same ordered invocation and condition scope; it counts toward
existing work/proof caps. Shared admission must not infer its own obligations from proof presence.

The change scenario is a fallible pinned call with omitted optional arguments. The catalog owns
its availability assertion; the existing binder owns the obligation set; common invocation
composition cites it; normal-expression and action consumers reuse the result. Core reconstructs
publication and immutable consumers retain the same contract as they migrate. Remove the current
`target.normal_return || bound.defaults.is_empty()` shortcut and the binder comment that assigns
availability to a total model. Keep the local fresh-definition/default-value certificates separate.
This is a new model premise composed through existing owners, not a second binder/interpreter or
a generic capability requiring another library.

| Judgment/gate | Target verdict | Basis and limit |
|---|---|---|
| A1 — Localize change | satisfied | One authored premise and the common invocation owner serve expression and action consumers; no target-name switch is added |
| A2 — Encode meaning structurally | satisfied | Availability, requiredness, value/identity and normal outcome remain distinct; exact model and signature-formal evidence own the obligation |
| A3 — Extend through composition | satisfied | Existing binding, ordered invocation, completion and action contracts compose without a new evaluator |
| G1/G2/G3/G5/G6; CI-G1 | pass for the target contract | Exact qualification and complete omitted-formal obligations are mandatory; unsupported mappings and bounded refusals remain explicit |
| G4/G7/G8; CI-G3 | pass for this target | No call-time default evaluation or hidden input; implementation is labelled Proposed and independent challenges remain evaluation evidence |
| CI-G2 | pass as a target obligation; implementation unassessed | Model/formal evidence and scope must survive publication and immutable-consumer admission, including whole-group omission controls |

Focused implementation acceptance must pair omitted-default invocation with unasserted
availability, missing required arguments, raising explicit arguments, omitted action subjects
and fallible outcomes. Challenge wrong target/phase/signature/formal, duplicate/missing/whole-group
evidence and the unchanged proof/work limits. A successful availability proof must permit reached
Invocation while still refusing unproved Normal actions. Existing fully explicit calls and local
default certificates remain adjacent regression controls; independent generated runtime programs
qualify only their named cases.

**Decision: Accept the ADR-0061 target within these boundaries.** The author may mark the ADR
accepted and the owner “Accepted target; implementation Proposed.” No new production default
assertion, native action support, broader local-default domain or Stage 3 completion is accepted.
Product tests, full `just test-all` and fresh `just pilot` were **not_run** for this documentation
review. Only this existing review artifact was changed by the reviewer.

## 28. Bounded implementation follow-up: pinned call defaults

**2026-09-27 · change/conformance · Implemented; focused Tested as attributed below.**
This reviews compiler 95 / catalog 6 against ADR-0061 and §27, under core 3.0, CI 1.1 and
the repository binding. Acceptance covers common invocation evaluation, shared structural
admission and source/Delta publication. Installed native remains compiler 93; no native
default/action acceptance or full S6 semantic closure is claimed.

The catalog's `call_defaults_available` premise reaches model targets, exact applications and
pinned call inputs separately from `normal_return`. `evaluation::prepare_calls` derives omitted
fixed formals from every bound signature before proof construction, requiring known requiredness
and matching omitted names while retaining each signature's fact identities. Empty variadics
are excluded. Invocation cites one availability model and those formal facts; it evaluates no
default expression and invents no argument value. The total-normal shortcut and its stale binder
documentation are removed. Failed binding leaves the obligation domain unknown; an availability
or work refusal retains a successfully bound domain without retaining positive proof.

`CallExecutions` commits that domain's count/digest independently of the retained invocation
proof. Shared admission requires the exact root group, and action admission also checks the
same application's authored promise when defaults were omitted. Core reconstructs source
binding and model application before accepting persisted results. Counts/digests prove structural
agreement, not raw binding semantics by themselves; immutable consumers must retain the latter
obligation during S6 migration. Nested default groups in a flattened invocation remain refused
until they have independently owned commitments.

**F16 — Bounded, exact default-group admission (corrected in this slice).** The first shared
admission checked only the leading group/immediate successor and could accept stray markers
later among argument evidence. The correction then constructed an expected marker range before
bounding a persisted count, allowing a huge count to request an unbounded allocation. Owner:
`cpg-schema/src/call_execution.rs`. The final implementation bounds the count by the already
bounded invocation before constructing ranges, then compares all invocation marker positions
with the sole committed root group. Focused controls reject whole-group deletion after resealing
the retained proof, individual deletion, reorder, duplicate/foreign/extra markers, markers in a
zero-default invocation and `i64::MAX`. The final source inspection and receipt below establish
this bounded correction; they do not establish future native raw-fact closure.

The extension scenario is a separately qualified fallible target with omitted defaults. It adds
an authored availability assertion and qualification evidence; the existing binder/evaluator,
call proof, action consumer and publication reconstruction compose it. The four initial targets
are individually qualified `json.dump`, `json.dumps`, `json.loads` and `gzip.compress`. Source
controls establish invocation while keeping their normal outcomes unproved; only the existing
potential I/O rule activates. Availability adds no default truth/value, callback behavior,
resource identity or complete coverage. Model argument bindings still withhold omitted subjects.

| Judgment/gate | Bounded verdict | Basis and limit |
|---|---|---|
| A1 — Localize change | satisfied | Catalog owns the promise, the common binder owns obligations, and invocation/action consumers reuse the same contract |
| A2 — Encode meaning structurally | satisfied | Unknown, empty and nonempty obligation domains differ; availability is independent of normal outcome and value identity |
| A3 — Extend through composition | satisfied | Existing signature binding, evaluation and proof commitments supply the extension; no second binder or model-name classifier |
| G1/G2/G3/G5/G6; CI-G1 | pass for this source/Delta slice | F16 corrected; all-signature/requiredness, retained-obligation, mutation and work-bound controls preserve explicit refusal |
| G4/G7/G8; CI-G3 | pass for this slice | No call-time default evaluation, generic interpreter or oracle input; qualification and implementation limits remain explicit |
| CI-G2 | pass for inspected source publication; native closure unresolved | Source reconstruction remains authoritative; no shared structural hash is presented as complete raw-fact semantics |

**Attributed receipt, inspected 2026-09-27; the reviewer did not run product tests:**

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target INSTA_UPDATE=no RUST_MIN_STACK=16777216 \
cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib \
  --test contracts --test codebooks --test compile \
  -E 'binary(contracts) | binary(codebooks) | test(models::) | test(actions::) | test(call_execution::) | test(pinned_defaults_require_an_independent_promise_and_all_signature_agreement) | test(action_triggers_preserve_partial_io_and_withhold_unproved_outcomes) | test(call_execution_proves_reached_inputs_without_inventing_callee_completion) | test(validation_schema_candidates_keep_attribution_separate_from_subjects)' \
  --status-level fail --final-status-level fail
```

**passed:** 33 tests, 176 skipped, 6.797s (`/tmp/lctx-stage3-pinned-defaults-tests4.log`), after
the author read and accepted the model/application/call schemas and codebook/rule migrations.
The source negative control removes only application default availability for the four otherwise
bound JSON/gzip calls: each becomes `default_unavailable`, retains its obligations and has no
positive invocation proof; publication detects the mismatch. `print()` remains an unsupported
binding control and is not misreported as an availability-specific success. Earlier attempts
encountered that control mismatch and expected migrations; the final receipt supersedes them.

The author's `uv run --no-sync python docs/design_review/evidence/2026-09-27_pinned-defaults/qualify.py`
and corresponding `uv run --no-sync ruff check` on that file **passed**. The retained
[qualification and seven generated programs](../evidence/2026-09-27_pinned-defaults/README.md)
compare pinned runtime/source optional formals and observe actual callee-body `PY_START`, including
fallible bodies and non-entry on missing required/raising explicit arguments. A CALL event alone
does not establish body entry. Analyzer fixtures are not executed; these finite observations do
not qualify arbitrary inputs or normal completion.

**Decision: Accept this bounded implementation; F16 is corrected with no remaining in-scope
blocker identified.** Native call/action support, full raw-fact semantics, nested default groups,
broader local/default-value domains, positive resource/callback fates, all-channel composition
and assembled Stage 3 remain open. Full `just test-all`, fresh `just pilot` and integrated
qualification were **not_run** for this slice. Only this review artifact was changed by the reviewer.

## 29. Target review: normal-exit action postconditions

**2026-09-27 · design/target · Proposed architecture; documentation/source-interface inspection.**
This reviews [ADR-0062](../../adr/0062-normal-action-postconditions.md), §9.9's proposed
normal-exit postcondition owner and §11.3's serving boundary under core 3.0, CI 1.1 and the
repository binding. Existing activated assessments, candidate subject/binding checks, ordered
invocation admission and the compiler95 default contract are the inspected baseline. This is
acceptance of a distinct implication contract, not positive execution or implementation acceptance.

The target makes a useful distinction without weakening ADR-0060: a reached/bound exact
invocation can support the model-relative implication “if this callee returns normally, this
authored Normal rule holds,” while its activated assessment still lacks a normal-outcome proof.
The pending obligation is separate from Definite/Potential modality, path compatibility and
outcome feasibility. It must not set expression normality, discharge a `call_transfer` boundary,
establish a possible successful execution, or close any outcome/channel domain. Exceptional is
not inferred as the complement of Normal; divergence and unresolved outcomes remain open.

Schema remains the semantic owner. Analytics composes exact candidate/application/input and
invocation evidence; core acquires and reconstructs publication; the native consumer must retain
the same meaning and evidence before Python adapts it. Factor the common candidate, model,
binding and invocation admission once for assessments and postconditions. A postcondition must
not be implemented by clearing an assessment's outcome refusal or by duplicating its admission
logic. The original activated relation and its proved-trigger requirement remain unchanged.

**Mandatory implementation detail:** an undischarged Normal obligation must be structurally
required by the postcondition relation and bound to the exact call occurrence, target, invocation,
phase and condition. Its requirement cannot depend on a retained witness or optional field
being present. Shared admission and source reconstruction must reject whole-obligation deletion,
retagging and cross-occurrence substitution. A caller's return or finalizer cannot satisfy that
callee obligation. A future discharge requires a closed normal-completion domain under the
admitted entry inputs/condition, not merely one normal-return witness.

The symbolic-result exception is narrow: an authored Normal acquisition at Output/ReturnValue
may describe the returned endpoint inside the implication without claiming an existing resource.
Its source call coordinates locate the occurrence; they do not prove resource identity or aliasing.
Admission must check that exact shape rather than blanket-ignore `resource_identity_unavailable`
or other candidate refusals. Input resources, callbacks, effect subjects and runtime schemas keep
their exact argument bindings; subjectless effects remain explicitly subjectless. Open targets,
wrong phases, missing required subjects, refused invocations and exceeded bounds still withhold
the postcondition. Concrete acquire/release pairing and negative release claims remain excluded.

The change scenario is another ordinary Normal model rule. It adds a declaration/qualification
and uses the existing shared candidate/invocation contracts, with the same explicit pending
outcome. A genuinely new exit domain or concrete alias/fate proof invokes the ADR's revisit
trigger rather than growing per-API exceptions. Retaining only generic refusals loses this
qualified meaning; activating on invocation or inventing a success BDD premise invents evidence.
The chosen separate relation avoids both. This is domain-specific contract composition, with no
new generic interpreter, provider framework or library dependency justified by this scope.

| Judgment/gate | Target verdict | Basis and limit |
|---|---|---|
| A1 — Localize change | satisfied | Shared schema admission separates common invocation support from activation versus implication; core and serving do not reclassify library names |
| A2 — Encode meaning structurally | satisfied | Mandatory exact-occurrence Normal obligation, authored modality and symbolic result endpoint retain different meanings |
| A3 — Extend through composition | satisfied | Existing candidates, source bindings and invocation proofs feed the new relation; future completion can discharge its explicit obligation |
| G1/G2/G3/G5/G6; CI-G1 | pass for the target contract | No inference of execution, feasibility, complement outcome, identity or completeness; malformed/missing obligations and unsupported inputs must be rejected |
| G4/G7/G8; CI-G3 | pass for this target | Proposed capability and boundaries are explicit; no runtime execution or oracle-derived production fact is introduced |
| CI-G2 | pass as a target obligation; implementation unassessed | S6 must expose outcome qualification separately or omit/report unsupported, with shared structural and raw-fact evidence closure |

The §11.3 amendment closes the important adjacent-consumer question: a postcondition cannot
enter ordinary positive action or compatibility selection by losing its pending obligation.
Rendering it as merely Potential is also insufficient. Interprocedural composition must retain
occurrence, phase and obligation; current FCA/RCA scope is unchanged. The active plan explicitly
retains source/local completion, concrete resource fates, transform/value composition and S6;
this new relation completes none of those obligations.

Focused acceptance must preserve an unactivated Normal assessment beside its valid postcondition,
challenge a call that enters then raises, and reject failed argument evaluation, missing subjects,
open/foreign targets, wrong phase/condition, removed/swapped obligations and original proof caps.
Include symbolic-return versus actual-resource identity and input-binding controls. Any future
serving test must retain the pending obligation in both selection and output. Independent runtime
cases challenge named examples and cannot prove the implication's antecedent universally.

**Decision: Accept the ADR-0062 target within these boundaries.** The author may mark the ADR
accepted and both owners “Accepted target; implementation Proposed.” No postcondition producer,
activated action, native consumer, resource identity or Stage 3 completion is accepted here.
Product tests, full `just test-all` and fresh `just pilot` were **not_run** for this documentation
review. Only this existing review artifact was changed by the reviewer.

## 30. Bounded implementation follow-up: normal-exit postconditions

**2026-09-27 · change/conformance · Implemented; focused Tested as attributed below.**
This reviews compiler 96's source/Delta implementation of ADR-0062 and §29 under core 3.0,
CI 1.1 and the repository binding. Catalog 6 and FORMAT 9 remain unchanged. Installed native
is compiler 93; this review accepts no native postcondition export, selection or rendering.

`cpg-schema::action` owns the new `modeled_action_postconditions` contract, identity and shared
admission. Normal is a mandatory non-null obligation, checked against both the row and authored
candidate before a refusal or positive implication is admitted. A usable implication must cite
the exact execution/condition and pass `admit_invocation`. This common function now owns
candidate/application/target, input/schema binding, raw argument domain, pinned defaults and
ordered invocation checks for both assessments and postconditions. Activated assessments retain
their independent proved-trigger requirement; no expression, action, transfer or coverage state
is promoted by the new relation.

`lctx-analytics::actions::assess` returns the two relations from one acquisition/indexing path,
including one positive or named-refusal postcondition per Normal candidate. Core publishes both
and independently compares each with the reconstructed pure result. Descriptor and authored
modality remain attached to the original candidate. The pending Normal obligation survives even
if an assessment separately proves normal completion; this slice implements no obligation
discharge. A valid implication claims neither feasibility nor occurrence of its antecedent.

The symbolic resource exception has a separate subject interpretation: only Normal Acquire at
Output/ReturnValue with CallResult status and this call's exact node/fact coordinates qualifies.
It waives only the absent resource-identity premise for that symbolic endpoint. Open targets,
foreign coordinates, wrong role/action/status, source refusal and malformed authored modality
still fail. Actual-value assessments retain the resource-identity refusal. Input subjects use
the unchanged exact argument binding. No resource identity, alias, acquisition occurrence,
release pairing or release absence is produced.

The qualification boundary matters here: **seven real source serialization/compression/
registration implications pass**. Their corresponding Normal actions remain unactivated without
callee completion. The fully explicit source `open` call exceeds the original 64-step proof cap
and retains `summary_proof_limit` (code 30); the ordinary open call is also withheld. The positive
symbolic-resource case is **contract-only**, not successful source activation or concrete resource
qualification. The first test's contrary source expectation was corrected without changing the
proof cap or weakening evidence requirements.

| Judgment/gate | Bounded verdict | Basis and limit |
|---|---|---|
| A1 — Localize change | satisfied | Shared invocation admission owns common premises; analytics and core compose/publish both contracts without another semantic classifier |
| A2 — Encode meaning structurally | satisfied | Mandatory Normal obligation, candidate modality and symbolic result interpretation remain distinct from activated outcomes and resource identity |
| A3 — Extend through composition | satisfied | Existing candidate/binding/invocation contracts support ordinary Normal rules; no second catalog, evaluator or generic framework |
| G1/G2/G3/G5/G6; CI-G1 | pass for the source/Delta slice | Independent reconstruction and obligation/scope/binding mutations protect implication meaning; source and pure cap controls retain refusal |
| G4/G7/G8; CI-G3 | pass for this slice | Pure composition, explicit supported limits and independent generated runtime cases; no execution oracle feeds production facts |
| CI-G2 | pass for inspected publication; native serving unresolved | Candidate and invocation support remain source-reconstructed; unsupported native consumers receive no successful-action projection |

The extension scenario remains a new ordinary Normal rule: its declaration and qualification
reuse shared admission and one producer, while a new exit domain requires the ADR's explicit
revisit. Concrete completion/resource/transform composition and outcome-qualified serving are
still separate consumers; this relation does not satisfy those remaining Stage 3 obligations.
No additional in-scope architectural or correctness finding was identified.

**Attributed receipt, inspected 2026-09-27; the reviewer did not run product tests:**

```sh
CARGO_TARGET_DIR=/home/paul/library-context/target INSTA_UPDATE=no RUST_MIN_STACK=16777216 \
cargo nextest run --release -p cpg-schema -p cpg-core -p lctx-analytics --lib \
  --test contracts --test codebooks --test compile \
  -E 'binary(contracts) | binary(codebooks) | test(models::) | test(actions::) | test(call_execution::) | test(pinned_defaults_require_an_independent_promise_and_all_signature_agreement) | test(action_triggers_preserve_partial_io_and_withhold_unproved_outcomes) | test(call_execution_proves_reached_inputs_without_inventing_callee_completion) | test(validation_schema_candidates_keep_attribution_separate_from_subjects)' \
  --status-level fail --final-status-level fail
```

**passed:** 36 tests, 176 skipped, 8.194s (`/tmp/lctx-stage3-postconditions-tests2.log`). The author
read and accepted the new table/rule migrations. Pure controls cover mandatory obligation,
foreign/missing execution, condition/scope, open target, shared bound-subject requirements,
narrow symbolic shape and the 64-to-65-step boundary. Source/Delta controls replace Normal
obligations with Invocation and remove postconditions or invocation evidence; publication rejects
the changed meaning/missing support. The earlier selection had the expected migrations and the
incorrect open-positive expectation; the final receipt supersedes that attempt.

The author's `uv run --no-sync python docs/design_review/evidence/2026-09-27_normal-postconditions/runtime_oracle.py`
and corresponding `uv run --no-sync ruff check` on that file **passed**. The retained
[six-program evidence](../evidence/2026-09-27_normal-postconditions/README.md) distinguishes
registration from later invocation, complete serialization from partial writes before failure,
and successful open from failure returning no resource. The exit queue is explicitly exercised
only by the isolated oracle after observing registration; it adds no production Stage 5 semantics.
These finite observations do not establish arbitrary outcome feasibility or exhaustive coverage.

**Decision: Accept the bounded source/Delta implementation.** Normal outcomes remain undischarged;
source/local-callee completion, concrete resource/callback/transform fates, all-channel summaries
and coverage, native semantic support and assembled Stage 3 remain open. Full `just test-all`,
fresh `just pilot` and integrated qualification were **not_run** for this slice. Only this review
artifact was changed by the reviewer.
