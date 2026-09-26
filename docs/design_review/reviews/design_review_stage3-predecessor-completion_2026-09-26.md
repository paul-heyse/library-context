# Stage 3 predecessor completion

## 1. Scope, outcome and coverage

**Interface-checked, 2026-09-26; change/conformance review.** This reviews the dirty S2
predecessor migration after `d464d9b`, separately from the accepted restricted
[expression/frame checkpoint](design_review_stage3-expression-completion_2026-09-26.md).
Standard: core 3.0, code-intelligence 1.1 and the repository binding. Authority:
[ADR-0057](../../adr/0057-compositional-stage3-semantics.md),
[behavioral analysis §9.9](../../design/sections/behavioral-analysis.md) and the
[active plan §3.0](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution).
This is not the deeper alignment review excluded by the operator.

The scope is `completion::Kernel::entry`/`truth`, the new return-entry tables, their core
acquisition/reconstruction and finite-summary consumption. Source inspection also followed
`cpg-flow::predicate::Translator::test`, `flow_tests`, `exit_sites`, and native proof admission.
Follow-up inspection includes condition-keyed entry certificates and path-specific first local
initialization. It does not extend acceptance to general rebinding or dynamic execution.
Tests, formatting and integrated gates were **not_run** by this reviewer. The author reports
24 focused pure tests and two source tests passed for this migration; native/model tests were
pending at initial inspection. These are attributed reports, not independently executed receipts.

The prior checkpoint's separate source/native receipt was reported passed by the author:
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p cpg-core --test compile --test bundle -E 'test(composed_argument_reads_keep_ordered_source_evidence) | test(nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries) | test(pinned_identity_models_require_and_publish_their_real_formals) | test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail`
(four tests, 20.852 seconds, 2026-09-26). That receipt does not qualify this subsequent migration.

### Ownership, facts and the change scenario

| Owner | Contract and fidelity | Adjacent consumer |
|---|---|---|
| Provider/schema | Attributed source syntax, per-site test conditions and return regions | Completion does not infer execution from a call relation |
| Pure completion | Ordered function-entry-to-return prefix under a return/candidate condition; explicit refusal | Statuses and dense steps keyed by return and condition |
| Core | Acquire source inputs, persist and reconstruct the same transformation | Publication validation compares entries and steps |
| Finite summaries | Require one scoped entry row, condition implication, dense steps and shared proof cap | Native paths retain the cited steps; full FORMAT 9 admission remains S6 |

The change scenario is adding a non-call predecessor that may raise, or a branch whose body
is skipped. The deleted call-only scanner could not establish those semantics. The new owner
walks enclosing frames from outermost to innermost, executes preceding siblings in source
order, and composes the existing statement/expression contracts. This localizes the semantic
change in completion; finite summaries no longer own a competing predecessor classifier.
Unsupported loop, handler, finally and context-manager entry remains a refusal rather than
being silently skipped.

### Path assumptions and opaque truthiness

**Interface-checked:** the inspected use of a same-site `flow_tests` predicate is valid for
the declared **path-local conditional** model. A normal opaque read does not independently
prove successful `__bool__`/`__len__` execution. The return condition instead assumes the
outcome of that particular evaluated predicate; `truth` requires implication of it or its
negation, cites the test fact, and retains that condition on the entry proof. Provider atoms
are evaluation-site identities, not stable properties shared between reads
([behavior model §3.9](../../design/sections/behavior-model.md)). A normal call's nonterminal
predicate is separately translated by the provider and is not a source test returned by
`Translator::test`.

This must remain a conditional entry proof. It cannot establish unconditional completion,
primitive-value identity, or stability across another truthiness evaluation. The implementation
resets the assumption for standalone statement certificates and pending-return finalizers;
the existing call-specific value-link barrier is not removed. An unconditional exit after
an opaque test has no implied truth outcome and remains unknown. Unsupported comparison or
dunder expressions still fail expression admission. Future consumers must preserve this
distinction; stronger conclusions require independent completion/stability evidence.

## 6. Gates

| Gate | Inspection verdict | Evidence or limitation |
|---|---|---|
| G1 Authority | pass | One predecessor semantic owner; old call-only scanner/SQL deleted |
| G2 Semantic fidelity | pass, scoped | Conditional and total completion are separated; corrected F01 retains withheld-root causes |
| G3 Validity | pass, scoped | Shared reconstruction and scoped/dense finite admission enforce the declared rows; pending execution receipts remain separate |
| G4 Hidden behavior | pass, scoped | Same-site outcomes are assumptions, not hidden total-completion assertions |
| G5 Consistency/recovery | pass, scoped | Unsupported frames refuse; F02 correction bounds expansion by actual requested pairs and existing per-certificate work/proof limits |
| G6 Transformation/reuse | pass, scoped | Operation-time and pre-existing root refusals now survive; candidate-condition admission is checked |
| G7 Claims | pass, scoped | No complete Stage 3 or integrated acceptance is claimed |
| G8 Library fit | pass, scoped | Relational acquisition, existing bounded BDD operations and Python-specific traversal retain their owners |
| CI-G1 | pass, scoped | Unknown and its supplied condition-root cause remain typed after F01 correction |
| CI-G2 | pass at canonical boundary | Source/test facts are retained; full native structural closure remains S6 |
| CI-G3 | pass, scoped | No reference/gold inputs added to the examined path |

## 7. Findings

<a id="F01"></a>
### F01 — Existing condition-root refusals lose their cause at completion acquisition

**Interface-checked, initial inspection.** `cpg-core::summaries::completions` discarded the boundary map
returned by `load_conditions`. When `Kernel::truth` cannot find the exit assumption diagram it
returns `unsupported_control_flow`; when the test diagram is missing it returns
`missing_evidence`. A root withheld for an existing atom/node/work limit therefore loses its
specific reason in `return_entry_statuses`, even though a new limit reached inside `implies`
or `and` correctly passes through `condition_limit`.

This violates the accepted typed-refusal contract (DP-02/DP-08/DP-21, CI-04). **Owner and
correction:** carry the existing condition boundary map through `completion::Inputs`; consult
it before applying generic missing/unsupported defaults for unavailable assumption and
predicate roots. No second refusal classifier is needed. **Closure:** pure controls for a
withheld assumption and predicate root preserve their supplied reason, alongside the existing
operation-time BDD controls. **Disposition:** active plan S2/S7 and §6 W5 own current status.

**Correction re-inspected, Implemented:** `Inputs::boundaries` now reaches `Kernel::truth`,
which consults it for both absent roots. The pure operation-cap test now includes withheld
assumption/predicate controls. Their pending rerun is not claimed passed here.

<a id="F02"></a>
### F02 — Condition-specific entry initially expanded every function condition at every return

**Interface-checked, follow-up inspection.** The first condition-keyed adapter obtained every
contribution condition for a function. Completion executed every return under every distinct
function condition, resetting its work budget per pair: R returns and C conditions eagerly
produced R × C certificates, including unrelated sinks. The per-certificate budget did not
bound this unnecessary product (DP-12/DP-20, CI-08).

**Correction re-inspected, Implemented:** core now requests actual
`(snapshot, return-site fact, candidate condition)` pairs. Contributions join the returned
flow sink by span and owner; predecessor conditions join only that successor flow. Completion
indexes and deduplicates these requests by return, adds the return's own condition, and
evaluates only those pairs. Output therefore grows with distinct requested pairs and returns,
each with the existing depth/work/proof limits, rather than an independent return/condition
product. Finite admission selects the exact candidate certificate; a sole-row fallback still
requires condition implication. **Closure evidence still pending:** source/native mixed-origin
regression and a focused unrelated-return request control. **Disposition:** active plan S2/S7
and §6 W5/W12 own current status.

**Path initialization extension, Interface-checked:** multiple syntactic assignment sites are
eligible during a full entry walk only when all bindings of the name are assignments and the
sites are outside loops. The selected execution prefix tracks initialized names and refuses a
second write; parameters and other binding kinds are excluded. Standalone and finalizer
certificates retain unique-initialization admission. This supports mutually exclusive first
initializations without admitting arbitrary overwrite or destructor execution.

## 8. Library fit and change cost

**Interface-checked:** the bounded BDD implication/conjunction calls reuse the schema kernel;
Python entry order and selected suites belong in the domain transformation. The smaller
correction is retaining the kernel's existing boundary output. No library migration, generic
interpreter or broader architecture change is proposed by this review.

FP-01/FP-03/FP-04/FP-06 are satisfied for this bounded owner migration: the next supported
predecessor uses statement completion rather than another finite-summary shape. FP-02/FP-05
are satisfied for the corrected scoped refusal and request contracts; test receipts remain
separate from this source-level judgment.

## 12. Judgment

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied, scoped | Predecessor semantics moved to the existing completion owner |
| A2 Encode meaning structurally | satisfied, scoped | Entry conditions, supplied root refusals and actual candidate requests are explicit |
| A3 Extend through composition | satisfied, scoped | Non-call predecessors reuse expression/statement semantics and shared proof admission |

**Bounded decision: Accept scoped, by inspection after F01/F02 corrections.** The path-assumption
design is accepted only for the stated conditional model. Targeted source/native/request
reruns remain pending, not presumed passed. Broader handler/frame entry and call-specific
stability are not accepted by this slice. The enclosing Stage 3 architecture and integrated
qualification are not assessed here; the earlier checkpoint's acceptance is unchanged.

## Implementation receipt (2026-09-26)

**Tested, bounded scope.** The author ran the corrected acquisition and native path checks:
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo nextest run --release -p lctx-analytics --lib -p cpg-core --test compile --test bundle -E 'test(completion::tests) | test(evaluation::tests) | test(summaries::finite) | test(composed_argument_reads_keep_ordered_source_evidence) | test(nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries) | test(pinned_identity_models_require_and_publish_their_real_formals) | test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail --final-status-level fail`
— **passed**, 36 cases. `INSTA_UPDATE=no cargo nextest run --release -p cpg-schema --test contracts --test codebooks --status-level fail --final-status-level fail`
— **passed**, 13 cases after snapshot diff inspection and acceptance. Generation with
`INSTA_FORCE_PASS=1` was only snapshot preparation, not the acceptance receipt.

The extension for one comparison independently evaluates ordered operands and cites the full
same-site predicate outcome; it leaves the general expression row unknown and creates no
primitive/stability link. Tuple operands retain ordered normal element evidence. A focused test
checks conditional-versus-standalone behavior, missing operands and an unrelated return request;
existing source/native string membership and integer equality cases pass. Existing guarded
recursive paths now cite their preceding predicate read, so the native test scopes explicit
call-argument assertions to the call segment rather than counting every read in the whole proof.

Formatting, integrated gates and full Stage 3 acceptance remain **not_run**. This receipt closes
neither the remaining S2 contract scope nor the broader S3–S8 plan.
