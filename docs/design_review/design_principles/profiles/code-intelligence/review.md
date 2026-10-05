# Code-intelligence review additions

**Version 1.4 · 2026-10-05** · What the [code-intelligence profile](principles.md) adds to
each slot of the [core review template](../../core/design-review-template.md). The additions sit
within the core slots; no slot is added or removed. Each applies only where the subject touches
the behaviour concerned.

| Core slot | Profile addition |
|---|---|
| 1 Scope and outcome | Name the fact families, analyses and served answers in scope, and the workloads considered (principles: *Functional target*). |
| 2 Responsibilities and authority | Add the **fact and fidelity table** after the component/dependency map. |
| 3 Contracts and testing boundaries | Owned semantics of extraction, resolution, derivation and projection; coverage and absence: where extraction can stop short, and how that is recorded (CI-04). |
| 4 Composition and execution | The **analysis record columns** (below); canonical graph or view semantics, physical access/placement, reuse and work versus output limits (CI-05/07/08). |
| 5 Change scenarios | The **code-intelligence journeys** (below), selected by relevance. |
| 6 Gates | Rows CI-G1, CI-G2, CI-G3. |
| 8 Library fit | Compare composed analyzer, query/graph/program-analysis and retrieval capabilities, including optimizer visibility, movement, preparation, degree sensitivity and shared kernels. |
| 9 Alternatives | Optional: how established code-intelligence tools handle the same question, read for behaviour only. |
| 10 Verification | Where relevant, the known-answer shapes that would settle a doubtful claim (below). |
| 12 Decision | Settle A4 for the declared code-intelligence workload independently of CI-G1–CI-G3; bounded output or honest refusal alone does not establish fit. |

For each domain journey, retain the core scenario columns, including kind of change, owner,
contract change, affected consumers, repeated decisions, required context and test setup. A
new semantic category can require a contract migration; an additional instance or contextual
binding should reuse its definition. Assess the meanings and governing operations behind the
fact and fidelity table; common output columns alone do not establish a common meaning.
Assess model adequacy and authoritative behavior under A2, separately from CI-G1–CI-G3.
Assess A4 qualitatively against intended extraction volumes, semantic-kind growth, evidence
journeys and skew where relevant. No numerical SLA, cost model or additional cost proof is implied. Investigate only as far as needed for the scoped
judgment; journeys do not require complete traces or a benchmark campaign.

## Fact and fidelity table (slot 2)

| Fact family or relation | Provider and revision | Fidelity (extracted / resolved / derived / inferred / heuristic) | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|

## Analysis record columns (slot 4)

Add to each projection, analysis or synthesis stage:

| Question answered | Projection (universe, selector, relations, direction, multiplicity, weights, scope) | Method and settings (seed, parameters, convergence) | Exact / conservative / heuristic, and the model | Budgets and partial-result behaviour | Output and evidence linkage |
|---|---|---|---|---|---|

## Code-intelligence journeys (slot 5)

| Journey | What to assess |
|---|---|
| **Add a fact family** | Declarations needed; provenance and fidelity; coverage rows; consumers; places meaning is re-expressed |
| **Add or upgrade an analyzer** | What its facts mean compared with the previous provider; disagreement handling; recorded run context |
| **Add an analytic** | Its projection, settings and model; its consumer; whether its output can reach a served claim as fact |
| **New release of the analysed code** | Identity across releases; what changes and what earlier answers still mean |
| **A module full of unresolved references** | How unknowns are recorded and served; that nothing reads them as absent |
| **Trace a served claim** | From the answer through findings/facts to spans within one pinned realization across calls, returned references, resources and continuations (CI-13) |
| **An evaluation run** | That references stay out of inputs and parameters; criteria fixed before the run |
| **Grow content or semantic kinds** | Same bounded request over more unrelated content; additional kinds without automatic per-kind operational expansion; complete global analysis where required |
| **High-degree node or concurrent compile/serve** | Examined edges/bytes, intermediate state, queues and contention; complete/partial/refused remain distinct |
| **Publish or replace a physical realization** | Drain writers, seal content/definitions before final reconciliation, publish coherently, preserve consumer pins and declared identity mappings |

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
| **Bounded answer, amplified work** | A small evidence request scans/rehashes an unchanged whole artifact or expands a high-degree frontier without appropriate work bounds | FP-07, CI-08 · A4 |
| **Pin ends before the journey** | A returned reference or continuation can resolve under changed content or answer-affecting definitions | CI-13 · G5, G6 |
