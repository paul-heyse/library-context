# Stage 3 completion entry and origin coverage

## 1. Scope and outcome

**Interface-checked, 2026-09-26; change/conformance; Accept scoped after correction.**
Core 3.0, code-intelligence 1.1 and the repository binding govern this bounded review under
ADR-0057. It covers compiler output 80's shared completion outcome, exact exception column,
handler/else/finalizer entry and initial Value/Call origin coverage. It does not certify assembled
Stage 3. The reviewer inspected code and ran no tests. Author receipts are stated separately.

## 2. Responsibilities and fidelity

Schema owns the outcome enum, exception/channel codebooks and persisted contracts. Pure completion
owns Python statement ordering and active exceptions. Pure finite summaries own origin coverage.
Core acquires, publishes and reconstructs those rows. The exact exception column describes a
raised outcome under statement entry; it is not a potential modeled exception or proof of entry.

## 3. Change scenario

Adding handler-body entry now reuses the same handler selection and try-body evaluation as
whole-statement completion. Adding a coverage consumer reads an explicit origin/channel row,
without interpreting absence of a summary as a negative. Other channels require their own inputs.

## 4. Analysis contract

Completion consumes attributed syntax, reads, conditions and pinned class/MRO evidence, and emits
ordered outcomes/refusals. Coverage consumes source candidates, retained summaries and refusals,
and emits deterministic origin-scoped rows. Publication compares reconstructed full rows, including
identity associations and the new exception field. Neither stage executes the analyzed library.

## 5. Consumer journeys

A normal try body can enter else but cannot enter a handler. An exact raise selects its handler;
unknown predecessors and named-handler cleanup refuse. A finalizer reconstructs its pending
outcome. A native handler-return fixture follows the resulting proof through Delta and generation
loading. Coverage currently remains a publication contract; FORMAT 9 consumption is still S6.

## 6. Gates

G1–G7 and CI-G1: no new violation identified within the corrected bounded scope. G8: no duplicate
generic library mechanism introduced. CI-G3: no evaluation-reference dependency introduced.
CI-G2 remains bounded by FORMAT 8's existing incomplete native reconstruction of completion
support; this slice does not close S6's structural evidence-closure obligation.

## 7. Findings

<a id="F01"></a>
**F01 — A finite call witness is not exhaustive coverage.** Initial coverage could close a
caller while a cited callee had unfinished alternatives. Correction: call-crossing origins remain
open with `call_transfer` until callee/model coverage is composed; the positive witness survives.

<a id="F02"></a>
**F02 — Retained-summary approximation must survive.** Raw-flow exactness alone misses exit-site
approximation. Correction: retained approximated summaries contribute `outside_provider_model`.

<a id="F03"></a>
**F03 — Omitted evidence is independent of semantic coverage.** A proposed schema check forced
every witness omission to open coverage. It was removed. Current failed proof construction still
leaves coverage open; future omission of redundant evidence can remain independent.

The [forward plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns current disposition. Pure regressions cover F01/F02, duplicate ordering and sibling origins;
F03 is reflected in the reviewed contract snapshot.

## 8. Architecture judgment

A1–A3 are satisfied for this slice: representation has one schema owner, orchestration does not
reclassify semantics, and entry/whole-statement evaluation compose through shared functions.
The completion enum also removes the initially unrestricted outcome constructor.

## 9. Alternatives

A second entry interpreter would repeat handler and finalizer decisions. Inferring completeness
from any positive summary would lose open alternatives. The chosen shared owners and conservative
coverage require neither a new framework nor a scheduler/library migration.

## 10. Author verification

**Tested, 2026-09-26:** 27 cases passed with `INSTA_UPDATE=no RUST_MIN_STACK=16777216 cargo nextest
run --release -p cpg-schema --test contracts --test codebooks -p lctx-analytics --lib -p cpg-core
--test compile --test bundle -E 'binary(contracts) | binary(codebooks) | test(completion::tests) |
test(positive_origin_does_not_close) | test(composed_argument_reads_keep_ordered_source_evidence) |
test(finite_depth_and_unsupported_refusals_reach_the_native_response)' --status-level fail
--final-status-level fail`. Includes source/Delta tampering, nested re-raise, native handler-return
positive and nonmatch/cleanup controls. Fixture embedding is synthetic and qualifies no live service.
Four migration snapshots were reviewed before `cargo insta accept`.

## 11. Remaining integration

Defaults, contexts, complete-zero-witness negative coverage, callee/model coverage composition,
multi-channel fixed points, operation-wide exhaustiveness and FORMAT 9 remain open. Formatting,
integrated tests, pilot, clean wheel and live checks were not run.

## 12. Disposition

Accept only the restricted implementation and corrected findings above. The active plan S1–S8
continues to own the remaining functional scope and integrated acceptance.
