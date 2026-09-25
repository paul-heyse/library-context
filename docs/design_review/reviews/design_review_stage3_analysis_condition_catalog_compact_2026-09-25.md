# Stage 3 analysis condition catalog — compact design review

## 1. Scope, purpose and coverage

| | |
|---|---|
| Subject | `analysis_conditions`, `analysis_condition_nodes`, ADR-0032 |
| Standard | Core 2.0, code-intelligence 1.0, library-context binding |
| Tier · purpose | Change · conformance |
| Reviewer · date | Codex · 2026-09-25 |
| Decision | Accept the structural flow-analysis catalog |

The flow model already constructs bounded BDD objects when it combines reaching and sink paths. This change writes their content-addressed roots and deduplicated node closure as analysis relations, distinct from provider condition facts. Publication reconstructs the rows from raw inputs and calls the shared `hydrate_catalog` validator; direct transfer and predecessor rows reference this catalog. No displayed DNF is parsed. Focused release Nextest passed 3/3 (175 skipped) and targeted release Clippy passed on 2026-09-25. Integrated pilot, generation load and serving remain `not_run` by operator direction.

## 6. Gates

| Gate | Verdict | Evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | Pass | Producer serializes the same `BoundedCondition` objects that flow analysis used, with a separate analysis identity. | — |
| G2 Semantic fidelity | Pass, scoped | Boundary roots carry named unknowns; approximation stays row-local. | Preserve both in summaries. |
| G3 Validity | Pass, scoped | Shared row reconstruction and `hydrate_catalog` check root closure, node identity/order and budget. | Verify pilot-scale limits at integrated end. |
| G4 Hidden behaviour | Pass | Existing Rust BDD objects are serialized; no analyzed Python or DNF reparse. | — |
| G5 Recovery | Pass, scoped | Version 49 and full publication validation precede snapshot admission. | Run end-to-end generation load later. |
| G6 Transformation | Pass | Provider and recomposed condition catalogs remain separate; derived references target the latter. | — |
| G7 Claims | Pass, scoped | The catalog enables, but does not itself decide, condition compatibility or transfer verdicts. | Implement L3 consumption and explicit unknowns. |
| G8 Library leverage | Pass | Reuses biodivine BDD serialization and the shared hydration validator; Arrow contracts and DataFusion references enforce persisted shape. | — |
| CI-G1 Fidelity | Pass | Condition identity derives from structural root, not display text. | — |
| CI-G2 Evidence closure | N.a. | No served claim consumes these rows yet. | Check FORMAT 7 root closure. |
| CI-G3 Evaluation integrity | Pass | Fixture input is independent of gold capability families. | — |

## 7. Findings and principle verdicts

| ID | Finding | Principles · gate | Evidence and consequence | Correction |
|---|---|---|---|---|
| F01, pilot gate | The shared catalog has finite root/node admission limits and its release-scale size is unmeasured. | DP-11, CI-08 · G3 | Focused fixtures do not establish FastMCP volume or hydration cost. | Report counts, work and limit hits on the end pilot; shard or cap with explicit unknowns if needed. |
| F02, deferred to L3 | Summary-created conditions are not in this flow-analysis catalog. | DP-08, CI-04 · G7 | The producer serializes `FlowModelRows.condition_models` only. | Persist summary roots through a validated extension before serving them. |

DP-01/02/03/04/05/07/08/11/13/14/15/18/19/21/22/23/24 and CI-01/02/03/04/06/07/08/10/12 are satisfied within this scope. Other principles impose no new local obligation. No SHOULD exception is requested.

## 8. Library-leverage ledger

| Capability | Bespoke code | Built-in feature | Fit and limit | Recommendation |
|---|---|---|---|---|
| BDD persistence | Row mapping and deduplication | `Diagram::root_and_nodes`, content ids | Lossless root/node authority already exists in the kernel. | Reuse, with no DNF interpretation. |
| Admission | None beyond shared reconstruction | `hydrate_catalog` | Same structural checks as provider/native admission; limited to 100,000 roots/nodes per catalog. | Measure at the integrated pilot. |

## 12. Decision

**Accept the separate structural analysis catalog.** Focused release Nextest passed 3/3 and targeted Clippy passed on 2026-09-25. This closes the condition-persistence prerequisite for source candidates only. Integrated `just test-all`, `just pilot`, structured evaluation and formatting remain `not_run` until the complete Stage 3 scope is implemented.
