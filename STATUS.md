# Status

_Updated 2026-09-24 by the handoff skill (Stage 2.9.1, the forward plan, and ADR-0023)._

## Where we are

- **The plan** is `docs/plans/behavioral-model-forward-plan_2026-09-24.md`, which supersedes the
  pivot plan. Stages 0–2 of the pivot are built. The operator approved decisions D-10 to D-13
  (evaluation atoms, a BDD condition kernel, the validation lane, a typed theory; deviation B23),
  and kept ty as the flow provider with Pysa as the cross-check (B21).
- **Stage 2.9 (hardening the semantic bridge) is in progress.** 2.9.1 is done (`4a79501`): the
  Stage 2 end review's fixes R1–R11, `flow_tests`, `flow_attribute_loads`,
  `raise_sites.escapes`, `abstract_body`, and four new semantic rules. Pilot refutations fell from
  63 to 19.
- **Reviews use the layered design standard** (ADR-0023, accepted, `fc3942d`), declared in
  `docs/design_review/design_principles/standard.toml`: core DP-01–24 and G1–G8, the
  code-intelligence profile CI-01–13 and CI-G1–G3, and `binding/library-context.md`. The skills
  are `design-review` and `design-review-code-intelligence`. Naming a check in every finding is
  optional. No new lints for design alignment.
- **Unpushed:** every commit since `origin/main`. Push only when asked.

## Last verified (2026-09-24)

| Command | Outcome |
|---|---|
| `just test-all` (at `24f381a`) | passed: nextest 262/262, pytest 80, rule tests 7/7, lint-agents, adr lint (23), fixtures 65, deps, shear, gold. This also covers `fc3942d`, which had left it not_run |
| `just adr lint`, `just lint-agents` (at `fc3942d`) | passed |
| `just pilot` (the tree of `4a79501`) | passed: snapshot `8f40e20a`, generation `4c50ccdc`, 41.5 s, smoke 20/20; 0 conditions `false` |
| `just pilot-live` | passed at Stage 2's evaluation, generation `85304572` (before the end review's fixes); not re-run since |
| Stage 2's structured evaluation | passed on attempt 2 (`structured_eval_stage2_2026-09-24.md`); to be re-applied at Stage 2.9.8 |

## Known failures and blocks

- **Translation defects** (the external review's assessment, X1–X3): condition atoms identified by
  their text drop feasible paths. The eight reproduced shapes cover impure calls, loops over
  `range`, mutation, `==` against `is`, version tuples and a `TYPE_CHECKING` parameter. **Tested**
  by probes and confirmed by CPython 3.14.7; the fixes are Stage 2.9.2–2.9.4.
- **Disk:** 189 GB free; the moved-aside stores (`build/store-pre-*`, ~11 GB) await the operator.
- **Untracked:** `docs/full_cpg_pipeline_external_review.md` (the operator's; not committed).

## Open decisions

- **ADR-0022** (proposed): accepted after Stage 2.9's re-review, with D-10 amended in. **ADR-0020**
  stays proposed; the LLM-trigger decision waits for increment 5.
- **`just adr revisit`:** nothing has fired. ADR-0021 needs two consecutive exit failures, and
  Stage 2's attempt 2 passed. ADR-0023's trigger: a principle that needs a repository-specific
  reading, or the shared core changing without a refreshed copy.
- **Deferred rows:** in `design_review_stage2-end_2026-09-24.md` §12 and
  `design_review_external-review-assessment_2026-09-24.md` §10.
- **For the operator's review:** the Stage 1 and Stage 2 evaluations, deviations B1–B23, and the
  moved-aside stores.
- **The first review under the new standard** is Stage 2.9.9's compact re-review. Record any
  friction in its Deferred table and tell the operator (ADR-0023's revisit trigger).

## Next

Stage 2.9.2: evaluation identity for condition atoms (the forward plan §4.1). Operators are kept
(`== None` becomes `equals`, and `is <literal>` gets its own atom, a codebook append). Synthetic
predicates are per site. Versions become definition sets. Sharing is limited to identity and
`isinstance` tests on names, and each of the assessment's P1 shapes gets a `flow_shapes` case.
