# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Product:** Stage 3 of increment 4 is in progress and **functionally incomplete**; production
  checkpoint `acbcee5` (no `crates/` or `python/` change since, apart from one doc comment). The
  [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) is the sole execution
  plan: §1 current state and qualification boundary, §3 queue, §6 findings W1–W16 (the single
  disposition owner for ARC-01–03 and the reasoning reviews' RF/F01–F16, RFU/F01–F08).
- **Documentation migration** ([plan](docs/plans/legacy-documentation-migration_2026-09-25.md),
  ADR-0042): M0–M4 landed. The forward plan internalizes earlier plans; ADR-0042 (accepted after
  a target review) makes Git the home of retired material; DESIGN keeps scope and §B decisions,
  and eight focused owners hold current contracts; ADR-0043–0047 consolidate rationale in force;
  retired plans, reviews, research input, standard files, unused evidence and 28 superseded ADRs
  left the tree ([historical recovery](docs/README.md#historical-recovery)). M5–M6 remain.

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| `just docs-test`; `just adr lint`; `just lint-agents`; `just docs-check` | `passed` for the migration commits (38 focused tests; lint 19 records after retirement). |
| Focused Ruff/pyrefly on changed documentation scripts | `passed`. |
| `just test-all` (2026-09-25 attempt, before the migration) | `failed` then corrected: 313/313 release Rust and 110/112 Python passed; the two stale tool-list expectations were fixed (`test_server.py` 12/12 `passed`). The rerun was stopped at operator direction; a complete pass is **not observed**. |
| Fresh `just pilot`; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query; increment-end review | `not_run`; Stage 3 functionality and exit are incomplete. |
| Reasoning-review probes (kernel, reach, access, serving) | `passed` as diagnostics on 2026-09-25: they reproduce defects, not conformance ([follow-up review §10](docs/design_review/reviews/design_review_library-fit-reasoning-followup_2026-09-25.md#10-verification-and-uncertainty)). |

## Known failures, blocks and decisions

- Open product defects and their closure checks are forward-plan §6 items; none is fixed. The
  most consequential: native proof-kind admission (W1), unsupported no-read claims for
  qualified/aliased builtin access (W4), lost facet verdicts and claim citations in serving (W2,
  W3), summary policy coupled to acquisition with discarded refusal causes (W5), `Model::reach`
  not a fixed point (W7), and BDD decision/support/aggregate budget defects (W6).
- ADR-0020, ADR-0024, ADR-0025 and ADR-0028 remain proposed (open choices); acceptance of
  ADR-0043–0047 restates decisions already in force and certifies no implementation.
- The complete gate, fresh pilot and every Stage 3 exit measurement are outstanding; the
  operator's expectation that the interrupted gate passes is an assumption.
- Moved-aside stores under `build/store-pre-*` and `build/store-stage3-*` await the operator.

## Next

Documentation: finish M5 (publication) and M6 (focused acceptance, assembled review, handoff),
then retire the migration plan. Product: start with W1 (ARC-01), deriving native proof-kind
admission from the schema contract and verifying a real finalizer-bearing generation; then W4
and W5 before any new SCC or limit policy (forward plan §3.1).
