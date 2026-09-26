# Design review: remaining Stage 3 composition

**2026-09-26 · Design / target · Decision: accept the proposed implementation direction.**
Standard: core 3.0, code-intelligence profile 1.1, repository binding. This is a proposed
architecture, not implementation acceptance or Stage 3 qualification.

## 1. Scope and evidence

Read-only inspection at `34c8784` covered the active plan, current design owners, the two
library-fit reviews, `behavior.rs`, `summaries/finite.rs`, core acquisition/publication and
validation, and native/MCP consumers. Pinned reasoning, ty-flow, DataFusion and Python-oracle
contracts informed placement. No tests or benchmarks were run for this review.

## 2. Responsibilities and fidelity

Schema owns semantic contracts; providers own observations; analytics owns pure composition;
core owns storage/orchestration; native owns bounded queries. `LocalCallSummaryFlowSeed` has
one optional control represented by correlated nullable fields. `simple_argument_evidence_sql`
is shared, but the source-specific producer paths and native link adjacency retain shape
knowledge. Source facts, candidate models, execution witnesses and channel coverage must stay
distinct (CI-01/02/04). A value-flow origin remains the unit of discharge (CI-03).

## 3. Contracts and isolation

The existing `FiniteSummaryInputs`/outcome boundary is retained. Add normalized bindings,
evaluation/completion certificates and channel coverage there. Local tests need typed inputs,
not acquisition or Delta. Publication reconstructs claims through shared validators; independent
oracles are still necessary because shared reconstruction can share a mistake.

## 4. Composition and boundedness

Order observations and handler/frame inputs before recursive summaries. Bind formals separately
from source evaluation order. Simultaneously substitute supported predicates, compose conditions,
then produce channel outcomes. Semantic fixed-point progress and witness identity are separate;
proof alternatives preserve parallel source sites and have explicit retention bounds.

## 5. Change scenarios

| Trigger | Owner and contract | Consumers and focused verification |
|---|---|---|
| Second or reordered control argument | Ordered binding/evaluation rows | Pure producer and native certificate admission; swapped-formal/capture controls |
| New existing-category pinned model | Typed catalog declaration | Existing application, evaluation and summary contracts; positive/withholding pair |
| Effectful finalizer | Frame completion transformation | Value and exception summaries consume the same outcome; overriding-return/raise controls |
| New rendering | Canonical result and support projection | No predicate reimplementation; structured/Markdown closure equality |
| Provider revision | Attributed extraction boundary | Contract parity; no provider-private handles downstream |

## 6. Gates

G1–G6 and CI-G1/2 have proposed enforcement routes through typed contracts, shared admission,
origin/channel coverage, explicit budgets and snapshot validation; assembled implementation
remains **unresolved**. G7 claims are explicitly Proposed/Interface-checked. G8 placement uses
existing relational, graph, BDD and serialization libraries. CI-G3 preserves frozen evaluation
inputs and independent generated-program oracles. No unresolved gate is certified as passed.

## 7. Findings and disposition

| ID | Finding (Interface-checked) | Proposed correction and disposition |
|---|---|---|
| <a id="F01"></a>F01 | One-control shape spreads knowledge across SQL, producer and native reader; FP-01/03/05, A1/A3 | Normalized bindings and shared evidence admission; active plan S1/S4 |
| <a id="F02"></a>F02 | Proof-bearing IDs drive recursive pending pairs; adding alternatives can expand work without new semantic knowledge; DP-04/12/20 | Separate semantic keys and bounded witnesses; active plan S4 |
| <a id="F03"></a>F03 | Summary publication currently precedes handler facts needed by future exception composition; FP-03/05 | Dependency-ordered preparation and explicit completion inputs; active plan S2/S5 |
| <a id="F04"></a>F04 | Path-local serving cannot establish operation-wide exclusion; CI-04/06/11 | Explicit channel coverage and operation partitions; active plan S5/S6 |

## 8. Library fit

DataFusion owns relational construction and Arrow transport; petgraph supplies SCC topology.
biodivine 0.6.3 supplies bounded apply/decision/restriction. Its `substitute` has no result/work
bound and no simultaneous capture-safe API; compose its bounded primitives over the original
DAG with cumulative admission. Ascent's timeout checks SCC iterations, not deterministic join
work. datafrog remains a comparison candidate; both leave Python semantics and refusal ownership
to the application. No performance advantage or replacement qualification is claimed.

## 9. Alternatives

Continuing narrow allowlists costs less initially but repeats upcoming semantic decisions.
Shared functions/types in existing crates hide those decisions without a framework. A wholesale
engine replacement leaves evaluation, coverage and proof gaps intact; defer it unless the
same-state multi-channel comparison establishes a concrete advantage.

## 10. Verification

Targeted pure tests, source controls, tamper validation, bounded native cases and independent
CPython/CrossHair/Pysa challenges accompany functional work. Formatting, full tests, fresh pilot,
clean wheel, live replay, structured evaluation and measurement wait until functionality lands.

## 11. Authority route

ADR-0057 and the behavioral-analysis/serving owners define the accepted target. The existing
forward plan §3.0 and §6 own current execution/disposition; this review stays dated evidence.

## 12. Architectural judgment

A1/A2/A3 are **satisfied for the proposed direction** through the scenario routes above;
implementation and assembled acceptance remain unresolved. Acceptance does not close W1–W16,
certify a negative claim, or establish integrated test outcomes.
