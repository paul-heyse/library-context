# Representative lexical definitions — bounded review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Tier · purpose | Change · conformance; compressed template slots 1, 6, 7, 8 and 12 |
| Reviewer · date | Independent Codex reviewer, 2026-09-29 |
| Subject | Uncommitted lexical and optional-subject changes over `be685f3d5027629265c1cf92a9f48e02f0b588e3`, including the F01/F02 corrections inspected in this session |
| Standard | [Core 3.0 and template](../design_principles/standard.toml), code-intelligence profile 1.1 and repository binding; design-review and companion code-intelligence skill |
| Authority | [ADR-0085](../../adr/0085-typed-semantic-domain.md), [ADR-0086](../../adr/0086-immutable-postgresql-generations.md), [DESIGN §15](../../design/sections/semantic-model.md), [cutover plan P0-B](../../plans/semantic-model-cutover-plan_2026-09-29.md#411-detailed-remaining-execution-order) |
| Bounded decision | **Accept scoped** after source reinspection of F01/F02 corrections, with author-reported post-correction focused model and real PG18 passes |
| Evidence strength | **Interface-checked / Implemented, 2026-09-29:** independent inspection. **Tested — author-reported, 2026-09-29:** post-correction focused model and real PG18 tests passed. No reviewer test execution or independent terminal-log verification is claimed. |
| Exclusions | Full raw-field mapping and producer migration in P2; complete lexical extraction or Python scoping qualification; normalization/consensus; full production admission; assembled phase acceptance. Concurrent document definitions, now wired according to the author, are excluded. |

Inspected changes: [`lexical.rs`](../../../crates/lctx-model/src/domain/lexical.rs), lexical
subject handling in [`assertion.rs`](../../../crates/lctx-model/src/domain/assertion.rs), lexical
membership in [`mod.rs`](../../../crates/lctx-model/src/domain/mod.rs), and optional-subject
generation in [`lctx-model-macros/src/lib.rs`](../../../crates/lctx-model-macros/src/lib.rs).
Controls comprise the shared
[`lexical fixture`](../../../crates/lctx-model/tests/fixtures/lexical.rs),
[`model tests`](../../../crates/lctx-model/tests/domain_lexical.rs),
[`PostgreSQL tests` (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/tests/domain_lexical.rs), and
[`source fixture`](../../../fixtures/python/semantic_lexical/example.py).

Adjacent inspection covered source occurrences and qualification/support validation, plus the old
[`lexical producer`](../../../crates/cpg-extract/src/lexical.rs),
[`lexical tables`](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/cpg-schema/src/tables.rs) and
[`codebooks`](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/cpg-schema/src/codebook.rs). The old implementation supplies
field-shape evidence, not authority over the accepted target. This review does not cover every
dirty file in the shared tree.

### Responsibilities, fidelity and change scenarios

**Implemented / Interface-checked, 2026-09-29.** The lexical module owns source-anchored scope,
binding-event and resolution-target vocabulary, qualified lexical observations and lexical
structure invariants. The shared assertion owner resolves nominal subjects to sources and checks
qualification, invocation, provider surface and evidence ownership. The derive expands declared
subject fields mechanically, including `Option<T>`; it does not decide lexical meaning.
PostgreSQL consumes the same generated contracts and shared invariants.

| Fact family / relation | Attribution, identity and fidelity boundary | Consumer / coverage |
|---|---|---|
| Lexical scopes and binding events | Nominal identities reference source occurrences and structural discriminators; attributed scope/binding observations are separate records | Shared lexical and support validators; no normalized entity or provider-local index is substituted |
| Scope, binding and reference observations | Qualification is keyed; generated support records retain run, surface, evidence, origin, mode and fidelity | Typed/Arrow/PG contract fixtures; no production producer coverage is established |
| Lexical resolutions | Binding, builtin and unresolved targets remain distinct; binding targets resolve to source events; captured builtins/unresolved targets refuse | Source/support validation; no runtime-call or complete-resolution claim |

The fixture authors synthetic records over source bytes. Its provenance and coverage values
exercise contract shapes; they are not evidence that a real lexical provider established those
claims. The real PostgreSQL test exercises stored-content validation using the shared model
validator, not an independent semantic oracle.

| Scenario | Owner and observed propagation | Evidence / boundary |
|---|---|---|
| Add an independent interpretation of the same binding position | Another qualified assertion and its provider support can coexist without changing normalization policy | F01 correction; `domain_lexical.rs:52–65` constructs a second provider/run/surface and distinct supported observation |
| Populate an optional value subject | The derive calls `SubjectValue::append_subjects`; `Some` participates in the existing source checks, `None` contributes no subject | `assertion.rs:76–109,242–249,285–335`; macro expansion at `lib.rs:54–68`; foreign-value controls |
| Encounter distinct occurrences sharing byte spans | Identity retains structural paths; a reference parent must have a strict prefix path, so equal spans alone establish no ancestry | F02 correction; `lexical.rs:189–196`; equal-span negative and positive controls |
| Change lexical interpretation without changing captured source | Interpretation remains in qualified observations/supports; source identities remain nominal anchors | Binding kind/value/ordinal are keyed observation data; no new producer classifier in PostgreSQL |

Source-level checks also retain lexical-scope kind/opening-occurrence agreement, module-root
parent rules, strict scope ancestry, same-source relationships, nonnegative binding ordinals and
paired static-branch/polarity values. Strict ancestry decreases path length, preventing cycles
without introducing another graph mechanism. Reference validation establishes structural ancestry;
it does not prove nearest placed-parent selection or Python scope semantics.

### Field-shape fit and explicit P2 boundary

**Interface-checked, 2026-09-29.** The representative definitions expose scope kinds, binding
kinds, static-branch qualification, reference parent/field placement and explicit resolution
alternatives corresponding to the old lexical tables (`tables.rs:679–790`). Full mapping remains
unresolved by design. In particular, the old producer keeps a bound-name range separately from
its declaration/parameter site (`cpg-extract/src/lexical.rs:345–357,739–747,955–978`); the new
`BindingEvent` currently has `site` and `name`. P2's existing field-disposition work must preserve
both meanings when migrating those cases. This is a retained mapping boundary, not a demand to
implement deferred producers or complete every field in P0-B.

### Verification and uncertainty

All observations below are dated **2026-09-29**. The author reported post-correction focused
lexical model and real PG18 passes, and earlier adjacent-suite results. Commands identify the relevant suites; original invocation
logs were not supplied to or independently checked by this reviewer.

| Suite command | Evidence and attribution |
|---|---|
| `cargo test --release -p lctx-model --test domain_lexical` | Post-correction **passed — author-reported**, four tests including the independent-provider and reference-ancestry controls; **not_run — reviewer**. |
| `cargo test --release -p lctx-model --test domain --test domain_assertions --test domain_stages` | Earlier **passed — author-reported**, 24 tests. Not treated as post-correction evidence; **not_run — reviewer**. |
| `cargo test --release -p lctx-postgres --test domain_lexical` | Post-correction **passed — author-reported**, one real PG18 test with local-positive/foreign-optional-value-negative cases; **not_run — reviewer**. |
| `just fmt`, `just test-all`, `just pilot`, `just docs-check` | **not_run — reviewer**; no formatting, integrated qualification or documentation publication gate was run. |

Post-correction passes above come from the author's explicit report, not the earlier receipts.
The new F01 control checks both lexical structure and support attribution. The F02 control
exercises the lexical structure invariant directly; it does not run each mutated topology
through sealed PG validation. The existing PG test covers the common stored validator route
and optional-subject source refusal. No real lexical producer or phase qualification is claimed.

## 6. Correctness and fidelity gates

Verdicts apply to inspected contracts after correction, at source-inspection strength. A pass
here is a design judgment, not a reviewer-executed test outcome.

| Gate | Verdict | Evidence / scope reason |
|---|---|---|
| G1 Authority | pass | Typed definitions author lexical meaning; generated schemas/codecs and shared validators consume those definitions. |
| G2 Semantic fidelity | pass after F01 correction | Distinct attributed interpretations survive; optional absence, unresolved targets and concrete bindings remain distinguishable. Full old-field mapping is excluded. |
| G3 Validity | pass after F02 correction | Reference ancestry uses source, span and strict structural prefix; present optional subjects cannot bypass source validation. |
| G4 Hidden behavior | pass | Added validation and derive logic is explicit and store-free; no acquisition or ambient analyzer configuration is added. |
| G5 Consistency and recovery | n.a. to changed lifecycle | No lifecycle protocol is changed. Existing PG sealed validation is an adjacent consumer, not newly qualified by this review. |
| G6 Transformation and reuse | pass for optional-subject expansion | Present subject values retain their nominal interpretation through generated enumeration and source checks; no cache/reuse mechanism is introduced. |
| G7 Truthful capability claims | pass within stated exclusions | Representative contracts and synthetic fixtures are separated from producer coverage and phase completion; post-correction passes are explicitly author-reported. |
| G8 Library leverage | pass for added scope | Existing derives, Arrow/PG lowerings and library collections serve the new definitions; source ancestry is a bounded domain predicate. |
| CI-G1 Fidelity | pass after F01 correction | Qualification and support attribution remain separate; raw disagreement is retained rather than rejected as premature consensus. |
| CI-G2 Evidence closure | n.a. to serving | Support source ownership is inspected, but no served claim/evidence-navigation consumer is changed or certified. |
| CI-G3 Evaluation integrity | n.a. | No evaluation-reference input, tuning or gold-consumption path is changed. |

## 7. Findings and applicability

Current execution disposition belongs to
[cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition).
This artifact preserves stable source findings and dated correction evidence. The reviewer
has not edited the plan and does not create a competing status register here.

<a id="F01"></a>
### F01 — Ordinal uniqueness rejected independently attributed observations

**Original priority: High.** Stable source:
`design_review_semantic-lexical_2026-09-29.md#F01`.

**Original finding, Interface-checked, 2026-09-29.** The first reviewed `LexicalCheck` stored
`(qualification, scope, ordinal)` in a uniqueness set (`lexical.rs:134,189` in that working-tree
version). Qualification excludes provider/run. Two distinct supported observations at one
position, differing in kind or value, therefore failed validation despite distinct semantic
keys. This forced agreement at the raw-observation layer, contrary to ADR-0085's separation
of attributed observations from normalization. The old duplicate-kind control reinforced that
overconstraint.

**Principles / judgment:** FP-02/03/04, DP-02/07/08 and CI-01/02; A2/A3 violated, G2/CI-G1 failed.
The correction owner is `lctx-model::domain::lexical`, with shared support validation preserving
provider attribution.

**Correction evidence, Implemented / Interface-checked, 2026-09-29.** The cross-attribution
ordinal set and rejection are removed. `lexical.rs:184–188` checks source relationships without
imposing ordinal uniqueness; local nonnegative validation remains at lines 73–77.
`independent_binding_interpretations_survive_at_the_same_ordinal`
(`domain_lexical.rs:52–65`) constructs two providers with their own runs/surfaces/supports,
sharing qualification/scope/ordinal but differing in binding kind, and requires both lexical
and support validation to succeed.

**Closure assessment:** the reported structural cause is removed and the correction is accepted
by independent source reinspection. The author reports the post-correction four-test lexical
suite passed, including this control (**Tested — author-reported, 2026-09-29**). This supplies
bounded closure evidence; current disposition belongs in plan §8. No reviewer execution is claimed.

<a id="F02"></a>
### F02 — Reference-parent validation ignored structural identity

**Original priority: Medium.** Stable source:
`design_review_semantic-lexical_2026-09-29.md#F02`.

**Original finding, Interface-checked, 2026-09-29.** The first reference-placement check
(`lexical.rs:191–195` in that working-tree version) required only source equality and byte
containment. `parent = read` and an unrelated occurrence with the same span both passed.
That admitted false ancestry despite occurrence structural paths being available. The old
reference contract describes a placed ancestor (`cpg-schema/src/tables.rs:745–763`).

**Principles / judgment:** FP-05, DP-03/04/07; A2 violated, G3 failed. The owning correction is
the lexical structure validator; it does not require implementing nearest-parent selection or
the P2 producer.

**Correction evidence, Implemented / Interface-checked, 2026-09-29.**
`lexical.rs:189–196` retains source/span checks and additionally requires the parent's path to
be a strictly shorter prefix of the read's path. Self-parenting and unrelated same-span paths
refuse; equal spans remain legal when structural ancestry holds.
`reference_parent_requires_strict_structural_ancestry_even_for_equal_spans`
(`domain_lexical.rs:69–84`) supplies those two negative cases and the equal-span ancestor positive.

**Closure assessment:** the reported gap is corrected and accepted by independent source
reinspection. The author reports the post-correction four-test lexical suite passed, including
this control (**Tested — author-reported, 2026-09-29**). This supplies bounded closure evidence;
current disposition remains plan §8's responsibility. No broader source-tree reconstruction claim follows.

**Applicability after correction.** FP-01/04/06 are satisfied for coherent lexical ownership,
mechanically derived forms and store-free semantic controls. FP-02/03/05 are satisfied for the
named disagreement, optional-subject and structural-ancestry scenarios. DP-03/04/07/08/22 and
CI-01/02 retain those bounded verdicts; completeness of P2 field migration, producer fidelity
and downstream consumers remains outside this decision. No new actionable finding was identified
in the correction reinspection.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-29.** No new external API or dependency is introduced. Ordinary
typed records/sums, the existing bounded derives, generated Arrow/PG representations and the
shared support validator provide the mechanism. `Option<T>` delegates to the same subject
contract rather than introducing a separate optional-reference interpreter.

Removing the ordinal uniqueness set is the simplest correction consistent with raw attributed
observations. An invocation-scoped ordering contract could be added only when its owner and
consumer require it; this slice does not invent one. Strict prefix ancestry uses existing
structural data and ordinary slice comparisons. A generic graph framework would add machinery
without improving this local invariant. No broader architectural alternative is required by this
bounded conformance review.

## 12. Architectural judgment and decision

| Judgment | Verdict | Bounded evidence |
|---|---|---|
| A1 Localize change | satisfied | Lexical meaning, shared source validation and mechanical derive expansion have identifiable owners; local checks need no extraction or database service. |
| A2 Encode meaning structurally | satisfied after F01/F02 correction | Attribution boundaries remain distinct; present optional subjects retain source ownership; reference ancestry respects structural identity. |
| A3 Extend through composition | satisfied after F01 correction | Another provider can contribute a distinct observation through the same assertion/support contracts without a global ordinal-consensus rule. |

**Bounded change decision: Accept scoped.** Independent source inspection accepts both
corrections; the author reports the post-correction four-test lexical model suite and real PG18
good/foreign cases passed. Those receipts support the bounded lexical checkpoint and are not
reviewer-executed tests. No further in-slice implementation change is requested by this reinspection.

**Enclosing architecture: not assessed as complete.** Full lexical field mapping, real producer
integration and qualification remain P2 work; no full P0/P1/P2 or production-admission claim is
made. Document definitions and their wiring are excluded. The existing cutover plan owns follow-up
and current finding disposition. Revisit field-shape/fidelity acceptance when P2 migrates the
lexical producer, and revisit these contracts if attribution or source-identity semantics change.
