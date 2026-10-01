# Status

_Updated 2026-10-01 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Restart checkpoint

The operator requested a stop for restart. All active subagent changes are integrated on main
through `18c3809`, including explicitly partial work. No implementation build remains running.
The [Phase 4 plan §13.3](docs/plans/semantic-model-phase4-detailed-plan_2026-09-30.md#133-restart-checkpoint-and-remaining-work)
owns the current resume order, package limits and unfinished controls; §13.2 retains earlier receipts.
Selected logs now live in the [durable restart bundle](docs/design_review/evidence/2026-10-01_phase4-restart/README.md).
Preserve unrelated dirty formatter/tooling work and existing worktrees/caches. Do not merge copied
worker prerequisites: all owned checkpoints have already landed.

## Phase 4 — incomplete

- **Accepted execution target:** ADR-0105/0106/0108; D0–Q0 remains the authorized implementation
  scope. This checkpoint stops execution at the operator's request, without claiming phase exit.
- **Implemented / focused-Tested:** vocabulary epochs, nominal owners and source-bound validation;
  normalized receiver/dispatch and catalog metadata; graph preparation; entry/Local, execution,
  authored models and finite Summary prerequisites; C0/C1/C2, structural and optional analytics;
  embedding spec/realization/consumption; deterministic synthesis and retrieval prerequisites.
  These are bounded receipts, not complete assembled acceptance.
- **Implemented:** `37cb4d9` wires `compile --through analysis|catalog` through one capture/attempt
  and cumulative checkpoints; compilation never selects. Final assessments/reporting and actual
  CLI fixture controls are present. Upper-frontier runtime qualification remains **not_run**.
- **Partial:** `18c3809` adds the single normalized symbolic field source prerequisite and typed
  negative synthesis kind. Record admission deliberately refuses MissingEvidence; Local/Summary/C1
  consumers are unfinished. The MDX template draft is unregistered and not compiled or qualified.
- The [foundation review](docs/design_review/reviews/design_review_phase4-foundation_2026-09-30.md)
  is **Accept scoped** at `0a60954`. The [behavioral review](docs/design_review/reviews/design_review_phase4-behavior_2026-10-01.md)
  is **Revise** at `bc6428a`; its P4B3-F01 source-association restoration and corrective reinspection
  remain open. The [parent finding register](docs/plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
  owns disposition; the detailed plan owns the correction sequence.

## Last verified — 2026-10-01

Cargo commands use `python3 scripts/build_environment.py --`. The durable bundle links raw logs
and scope; earlier passing results do not certify subsequent model changes.

| Command / scope | Outcome |
|---|---|
| `cargo check -p lctx --tests`, final merged `18c3809` | **passed**, 59.72s; unregistered MDX draft excluded |
| `cargo test --release -p lctx-model --test analysis_schedule --test upper_frontier_closures` | **passed**, 2 static controls before final metadata addition; no provider claim |
| `cargo test --release -p cpg-extract --test final_coverage` | **passed**, 1 actual native control |
| Worker release `domain::synthesis` controls | **passed**, 47 before final negative-kind addition; current runtime rerun not_run |
| Worker `cargo check -p lctx-model --release`; native `symbolic_fields` diagnostic | **passed**, partial source owner and 1 native shape capture; association qualification not_run |
| Native `read_channels`; actual PG `base_execution field_read_screen` | **passed**, 3 native + 2 PG |
| Actual PG `catalog_evidence`, `catalog_selection`, `model_publication`, `structural` | **passed**, 7 controls; no complete F0 claim |
| `cargo test --release -p cpg-core --test summary_publication -- --nocapture` and companion targets | **failed** enclosing attempts: original PG contract failures corrected; later Catalog passed, Behavioral interrupted; newest attempts failed missing build output or stopped before runtime |
| Upper CLI release fixture controls | **not_run** at runtime: first build failed missing path, isolated retry stopped for restart |
| `just test-all` / Phase 4 exit | **not_run**: functional scope, X0 retirement and assembled review incomplete |
| Live embedding, pilots, full-library comparisons and performance | **not_run**, excluded scope remains unchanged |

The operator reported disk cleanup during vanished-build-path failures. No semantic regression is
inferred from those build failures. The corrected Summary fixture uses depth two, retains all
permutation/corruption controls and leaves production defaults unchanged; Behavioral PG is unqualified.
No hygiene/hook commands were run or inspected for this checkpoint.

## Resume order

1. Complete the normalized source association gates; integrate Local, uncertain Summary, C1 and S0.
   Keep Unknown/proof=None separate from finite proof authority; qualify and reinspect P4B3-F01.
2. Finish native MDX Warning/ParamField templates and exact control-warning wording; qualify typed
   behavioral refutations. Preserve original evidence and documentary applicability boundaries.
3. Rerun corrected actual Summary PG publication/replay/corruption controls.
4. Qualify both upper frontiers/profiles and S0/E0 with seedless/nonzero briefs and fake cache replay;
   rerun normalized metadata/schema controls changed by the last checkpoint.
5. Map retained expectations, then perform X0 legacy/dependency retirement, preserving named P5 work.
6. Obtain assembled review, run Q0 `just test-all` after all functional work, and update handoff.

## Prior phases and standing limits

Phases 0–2 are **Implemented / Tested** within the [facts receipt](docs/design_review/evidence/2026-09-30_facts-qualification/README.md).
Phase 3 is **Implemented / focused-Tested** with scoped reviews and the historical 2026-09-30
composite automated gate in [its plan §11](docs/plans/semantic-model-phase3-detailed-plan_2026-09-30.md#11-current-package-status-and-finding-routes)
and [qualification bundle](docs/design_review/evidence/2026-09-30_phase3-qualification/README.md).
That historical gate does not qualify current Phase 4 changes. Pilot-scale costs, graph hydration
measurements, streaming joins and total RSS limits remain unqualified.

`lctx-model::domain` owns semantics; PostgreSQL owns canonical storage; DataFusion supplies compute.
Facts/normalized compilation publishes either profile without selection. Catalog Flow is NotRequested;
behavioral uncertainty remains explicit. Phase 5 serving/MCP and PR6 remain unavailable.
Legacy P4/P5 sources, retained services and protected backups remain preserved pending their named
consumers. No compatibility reader, old-ID bridge or dual store was introduced.

ADR-0109's shared roles and task-specific delegation remain implemented. Continue ordinary work on
main, preserving concurrent edits. The automatic end-of-turn hook owns formatting and hygiene;
do not run or troubleshoot it manually. Reactivate excluded measurements only on user instruction.
