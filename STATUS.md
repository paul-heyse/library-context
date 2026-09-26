# Status

_Updated 2026-09-25 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- **Increment 4, Stage 3 is functionally incomplete.** The
  [forward plan](docs/plans/behavioral-model-forward-plan_2026-09-24.md) is the sole execution
  plan and §6 owns W1–W16 disposition. The pre-Stage-3 remediation has several focused-tested
  slices since `acbcee5`, but no assembled qualification.
- **Landed slices:** W2 facet value verdicts (`1dfde60`); W9 canonical cache fill (`6c92ddc`);
  W8 corpus helper identity (`b0d93a8`); W14/F14 suppression visited state (`df84905`);
  W11 literal theory, cube factoring, bounded exact-input restriction/link minimization and
  ten-atom truth table (`96081b7`, `dd3bdcf`, `0db9eb9`, `7d02b42`); W16 spec-derived
  controlled vLLM launch (`2d48196`); W4 resolved builtin access and W14/F11 ty run context
  (`5044fee`); W1 proof-kind admission and W6 native retained-node admission (`8781246`).
- **Schema migration:** W14/F10 (`4921a27`) persists Pyrefly resolved function status in
  `function_implementations`; extractor output 31, compiler output 75. Contract/codebook snapshots
  were inspected and accepted. W15's [ADR-0048](docs/adr/0048-schema-rebuild-policy.md)
  (`de97cb9`) chooses rebuilding the current store from pinned inputs; old binaries and
  historical reads are not a product requirement. The fresh rebuild has not run.

## Last verified (2026-09-25)

| Command | Outcome |
|---|---|
| `cargo test -p cpg-schema --lib primitive_theory --quiet` | `passed`: 6 focused tests, including irrelevant-link removal. |
| `cargo test -p cpg-schema --lib condition_kernel --quiet` | `passed`: 13 focused tests, including the ten-atom truth table. |
| `cargo test -p cpg-schema --test codebooks --test contracts --quiet`; `cargo test -p cpg-extract --test variants --quiet`; `cargo test -p cpg-core --test function_implementation --quiet` | `passed` for the F10 migration (`4921a27`). |
| `just adr lint`; `git diff --check` | `passed` after ADR-0048 and the focused code edits. |
| `just fmt`; `just test-all`; fresh `just pilot`; all-techniques guard; Q01/Q03/Q05/Q09; structured evaluation; clean-wheel query | `not_run`: explicitly deferred until all planned functional scope is implemented. |

The earlier 2026-09-25 `just test-all` attempt stopped after two stale Python tool-list tests;
their focused correction passed, but no complete rerun was observed. The current targeted green
tests are not an integrated pass. The all-techniques digest will need repinning after the new
`function_implementations` table is included in a fresh compile.

## Known failures, blocks and decisions

- The plan's §6 identifies unfinished W1 typed native decoder/real finalizer generation, W3
  evidence closure, W4 publication trace, W5 summary ownership/refusals, W6 acceptance controls,
  W7 cyclic reach fixed point and typed budget boundary, and W11 oracle/served Q09 controls.
  W12–W13 remain open; W10 and W14/F15 retain their stated deferral triggers. W16's focused
  correction supports the operator-controlled launch only; arbitrary endpoint identity is not
  established and is deferred until a consumer requires it.
- W7's current DFS can lose a loop-carried transfer according to the recorded query-order
  reproduction. A work cap must surface a typed boundary before incomplete source sets can feed
  positive summaries; no production W7 edit has landed yet.
- A fresh store rebuild, integrated tests and pilot are required before claiming Stage 3 exit.
  Moved-aside stores under `build/store-pre-*` and `build/store-stage3-*` remain operator-owned
  cleanup; ADR-0048 does not require retaining their matching binaries.

## Next

Finish W1 in `lctx_semantics`: one typed generation decoder and real finalizer-bearing native
query, with schema-drift and unknown-kind controls. Then trace W4's corrected premise through
publication/serving and implement W3/W5 before W7's SCC solver and budget publication. Use only
targeted checks during implementation; run `just fmt`, integrated tests and the pilot after the
full planned functional scope lands.
