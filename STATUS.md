# Status

_Updated 2026-09-24 by the handoff skill (the behavioral-model plan, Stage 2.1 and Stage 1's revise)._

## Where we are

- **The pivot (ADR-0021, accepted):** the behavioral model of the whole public surface is the
  product; briefs are one rendering. The plan is
  `docs/plans/behavioral-model-pivot-plan_2026-09-24.md`, run straight through under the operator's
  instruction, with every judgment call logged in
  `docs/design_review/reviews/deviations_behavioral-model_2026-09-24.md` (B1–B14).
- **Stage 1 is done and revised.** Increment 3's deep review (Revise) is dispositioned at `f6f00b7`:
  one verdict per arc, the status over the scan's region, served facet completeness, persisted
  steps, `FORMAT` 4.
- **Stage 2.1 is done** (`b00a69d`): the ty flow provider (`cpg-flow`), the condition language
  (`cpg_schema::condition`) and the `flow_shapes` known answers. ADR-0022 was revised in place after
  its Stage 2 standard review (Revise) and stays **proposed** until Stage 2's oracles pass.
- **Next is Stage 2.3:** wire `cpg-flow` into extraction as the `flow` family, with the two-way parity
  rules; then 2.6's L2 relations (value flows, field and settings reads, `negative_premises`).
- **Unpushed:** every commit since `origin/main`. Push only when asked.

## Last verified (2026-09-24)

| Command | Outcome |
|---|---|
| `just test-all` (at `f6f00b7`) | passed: nextest 250/250, pytest 79, rule tests 7/7, lint-agents, adr lint (22), fixtures-check, deps (with the declared ty family), gold |
| `just pilot` (at `f6f00b7`) | passed: snapshot `eb444ac5`, generation `0af33db8`, 36.8 s, smoke 20/20; 570 operations established, 601 unknown, 363 classes not analyzed |
| `cargo nextest run -p cpg-flow` | passed: 21/21 `flow_shapes` known answers, facts pinned by a snapshot |
| The Stage 2.1 spike (FastMCP 4.0.5, 275 modules) | measured: 213 ms, 47 MiB peak; parity and reaching-definition results in deviation B11 |
| `just pilot-live` | not_run since increment 3.1: the GPU was not checked this session |

## Known gaps

- **Disk:** the volume is at about 99% (12 GB free after `target/debug/incremental` was removed,
  deviation B14). Two moved-aside stores are kept for the operator: `build/store-pre-stage1-2026-09-24`
  (1.5 GB) and `build/store-pre-revise-2026-09-24`.
- **Stage 1's evaluation** (`structured_eval_stage1_2026-09-24.md`, "met in part") awaits the
  operator's review (B10). Stages 2, 3 and 5 now have pre-registered pass/fail exit rules.
- **The source-body view** stays under a trigger: Q23 and Q24 at Stage 2's evaluation (B13).

## Open decisions

- **ADR-0022** (proposed): accepted when Stage 2's implementation meets its oracles (the review's §12).
- **ADR-0020** (proposed) and the LLM-trigger ADR stay `proposed` for the operator.
- **`just adr revisit`:** nothing has fired. ADR-0021's trigger (an exit criterion failing twice)
  can now fire, because exit rules exist.
- **Deferred rows:** the deep review's O2, O5–O7 and the Stage 2 review's O2, each with its trigger.

## Next

Stage 2.3: run `cpg_flow::index` inside `run_release` over the release modules' text, write the
`flow` family (definitions, uses, reaching, value sources, regions, conditions and atoms) as facts
with coverage rows, add the parity and reaching-within-candidates rules with injected cases, and
put `cpg_flow::PROVIDER` into the producer revision (bumping `EXTRACTOR_OUTPUT_VERSION`).
