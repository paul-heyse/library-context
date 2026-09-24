# Code-intelligence review additions

**Version 1.0 · 2026-09-24** · What the [code-intelligence profile](principles.md) adds to
each slot of the [core review template](../../core/design-review-template.md). The additions sit
within the core slots; no slot is added or removed. Each applies only where the subject touches
the behaviour concerned.

| Core slot | Profile addition |
|---|---|
| 1 Scope | Name the fact families, analyses and served answers in scope, and the workloads considered (principles: *Functional target*). |
| 2 Authority map | A **fact and fidelity table** (below). |
| 3 Contracts | Coverage and absence: where extraction can stop short, and how that is recorded (CI-04). |
| 4 Derivation and execution | The **analysis record columns** (below) for every projection, analysis or synthesis stage. |
| 5 Journeys | The **code-intelligence journeys** (below), selected by relevance. |
| 6 Gates | Rows CI-G1, CI-G2, CI-G3. |
| 8 Library ledger | Consider analyzers, graph algorithms, program-analysis frameworks and retrieval components wherever own code performs them. |
| 9 Alternatives | Optional: how established code-intelligence tools handle the same question, read for behaviour only. |
| 10 Verification | Where relevant, the known-answer shapes that would settle a doubtful claim (below). |

## Fact and fidelity table (slot 2)

| Fact family or relation | Provider and revision | Fidelity (extracted / resolved / derived / inferred / heuristic) | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|

## Analysis record columns (slot 4)

Add to each projection, analysis or synthesis stage:

| Question answered | Projection (universe, selector, relations, direction, multiplicity, weights, scope) | Method and settings (seed, parameters, convergence) | Exact / conservative / heuristic, and the model | Budgets and partial-result behaviour | Output and evidence linkage |
|---|---|---|---|---|---|

## Code-intelligence journeys (slot 5)

| Journey | What to trace |
|---|---|
| **Add a fact family** | Declarations needed; provenance and fidelity; coverage rows; consumers; places meaning is re-expressed |
| **Add or upgrade an analyzer** | What its facts mean compared with the previous provider; disagreement handling; recorded run context |
| **Add an analytic** | Its projection, settings and model; its consumer; whether its output can reach a served claim as fact |
| **New release of the analysed code** | Identity across releases; what changes and what earlier answers still mean |
| **A module full of unresolved references** | How unknowns are recorded and served; that nothing reads them as absent |
| **Trace a served claim** | From the answer back through synthesis, findings and facts to spans, in one snapshot |
| **An evaluation run** | That references stay out of inputs and parameters; criteria fixed before the run |

## Known-answer shapes (slot 10)

Where a claim about graph or analysis behaviour is in doubt, these small shapes settle it cheaply:
isolates, parallel relationships, self-loops, reconvergent paths, cycles that cross file or
module boundaries, unresolved targets, mixed configurations, malformed weights, and shuffled
input order for determinism. Compare partitions by membership, not labels; compare numbers under
stated tolerances.

## Calibration: adequate finding shapes

| Shape | What makes it evidence | Principles · gate |
|---|---|---|
| **Relation relabelled** | The site where one analyzer's relation is consumed as another (inference as dataflow, possible target as call), and the served output that changes | CI-02 · CI-G1 |
| **Unknown served as absent** | The query or template that turns an empty result over incomplete coverage into "none" | CI-04 · CI-G1 |
| **Parallel relationships collapsed** | The projection or table that keys on endpoints, and the evidence or multiplicity lost | CI-03, CI-05 · G6 |
| **Output filter shrinking the universe** | The selector applied before a traversal or global metric, and the answer that differs | CI-05 · G6 |
| **Heuristic reaching a claim** | The path from a community, rank or similarity output to a served control, limit or behaviour | CI-09 · CI-G1 |
| **Claim beyond its model** | A behavioural answer stated without its model, or a refuted-under-model verdict served as absence | CI-06 · CI-G1 |
| **Cross-snapshot citation** | A served claim whose evidence ids can resolve in a different snapshot | CI-11 · CI-G2 |
| **Ambient analyzer input** | Configuration or environment the analyzer discovers for itself, outside the recorded context | CI-10 · G4 |
| **Gold leakage** | An evaluation reference path, or a parameter tuned on it, reachable from the compiler | CI-12 · CI-G3 |
| **Reachability as dataflow** | A program-analysis question answered by plain graph reachability without transfer semantics | CI-07, CI-06 · G8, CI-G1 |
