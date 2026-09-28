# Status

_Updated 2026-09-28 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Product target and checkpoint

- **New accepted direction, implementation Proposed:** [ADR-0071](docs/adr/0071-api-evidence-product.md) replaces general behavioral completion as the first-product route with an [API and evidence catalog](docs/design/sections/api-and-evidence-product.md): exact public contracts, options, original scenarios, deployment evidence, typed selection and bounded agent packets.
- **Primary queue:** [forward-plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution), PR0–PR6. [§6.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition) owns AP/F01–F06. The [target review](docs/design_review/reviews/design_review_api-evidence-product_2026-09-28.md) assesses both external recommendations against `739f9df`; A1–A3 satisfied at **Proposed** strength. No differentiated-product claim is qualified.
- **This change is design only.** Architecture, plan, review and ADR authorities changed; no runtime, dependency, database, vector or stored-generation change. The analytics freeze transfers ownership without changing any digest, parameter or prior exposure disclosure.
- **Stage 3 remains functionally incomplete.** Its P0–P7/S1–S7 sequence and Stage 4–5 are retained research, not the next queue. Existing truth criteria, five verdicts, gold/heldout and open findings remain; unsafe exposed claims must be fixed or withheld. [Forward-plan §6](docs/plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition) owns those obligations.
- **Retained implemented foundation:** PG0–PG17 local exact serving (`a0ea858`, qualification recorded at `739f9df`), canonical Delta publication under ADR-0067, expanded stack under ADR-0068 and recovery/physical admission under ADR-0070. [PG plan](docs/plans/postgresql-integration-plan_2026-09-27.md), [runbook](docs/postgresql.md).
- **Unchanged versions:** compiler107, extractor33, template20, catalog7, FORMAT12/projection2, 1024 dimensions; owned provider pin `paul-heyse/datafusion-table-providers@e6fc4c40`.
- **Last recorded operator state, not re-probed this review:** PG18.6/pgvector0.8.6, migrations001–008; generation `32b4cb5d…`, exact profile `ff645e4a…`, prior ready generation retained. Four SQLx plus two provider connections per process. No ANN candidate qualified; exact remains the supported deployment.
- **Preserve recovery assets:** `build/postgresql-pg17-operator.dump` and receipt/artifacts; schema004/schema007 assets in `build/postgresql-pg16-baseline-9471b93/`; PG7 assets in `build/postgresql-pg8-baseline-6403b60/`. Retain ready generations/artifacts.
- **Development:** cached Cargo target, root editable uv environment and native extensions rebuilt in place. No wheel work is scheduled.

## Verification boundary

| Command / evidence | Outcome and scope |
|---|---|
| Target source/document inspection and two independent read-only reviews (2026-09-28) | **passed for design consistency:** refined overload quantifiers, structural type filters, conflict precedence and comparable evaluation population; no functional qualification |
| `just docs-check`; `just adr index`; `just adr lint` (2026-09-28) | **passed after repair:** 158 canonical pages, offline links/fragments, 37 ADR records and agent checks; retained the old semantic-queue anchor after retargeting the plan |
| `just gold`; Python JSON comparison of freeze metadata against `739f9df` (2026-09-28) | **passed:** only owning ADR and transfer note changed; all frozen digests and other fields identical; no scoring or sealed-data inspection |
| `git diff --cached --check` (2026-09-28) | **failed only on pre-existing trailing whitespace in the supplied pathway review**, preserved verbatim; the same check excluding that input **passed** |
| `just test-all`; `just pilot`; live PG/embedding probes; Context7 task comparison | **not_run:** design-only scope; no implementation or comparative run |

Prior product receipts remain in [PG evidence](docs/design_review/evidence/2026-09-28_postgresql-operations/README.md), dated 2026-09-28: full gate, fake/live pilot, exact deployment/rollover and a 69-table/two-generation restore through native/MCP in 15.31 seconds. ANN admission failed natural-plan/latency benefit; confirmation was not_run because no candidate was admissible. These are retained receipts, not checks repeated for this design.

## Next and known boundaries

1. **PR0, evaluation/schema owners:** inventory core API journeys and missing facts; freeze development tasks and independent confirmation criteria; establish comparable Context7 version/source availability before tuning.
2. **PR1, catalog/publication owners:** deliver mandatory catalog and ordered API contracts through real PG/MCP with explicit optional capabilities. The existing no-analysis documentation seed still fails its Outcome validator; preserve that validator and make catalog readiness independent.
3. Continue PR2–PR5 for bounded surface/configuration, contextual evidence/deployment, typed retrieval and agent usability. Usable-pilot delivery and PR6 measured differentiation are separate milestones; comparative parity leaves the value objective open.

Conditional PostgreSQL capabilities remain in its plan §7; arbitrary endpoint attestation remains W16. Broader Python semantics, general heap/decorator/lifecycle completion, ontology work and ANN tuning require a named product gap. New implementation uses targeted functional checks first; formatting and integrated gates follow the complete authorized functional scope.
