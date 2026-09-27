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
