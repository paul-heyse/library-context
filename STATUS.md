# Status

_Updated 2026-09-24 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and slice

- Active scope: [Stage 3 of increment 4](docs/plans/behavioral-model-forward-plan_2026-09-24.md). Stage 2.9 is complete. Stage 3.0 has bounded BDD conditions, attributed entry-value proofs and focused primitive refutation. No served compatibility verdict exists.
- Committed Stage 3.1 (`3e676dc`–`9a3606e`): typed catalog, pinned context target and Pysa formal bindings, authored transfer rows and shared publication validation. The CrossHair receipt covers the `int` specialization of `typing.cast` only.
- Committed Stage 3.2: `6110afd` adds attributed return/raise/finally source sites; `000542f` adds except clauses and direct handler actions. Their focused tests, schema migrations, validators and compact reviews passed. These rows do not prove an exception is caught or an action completes.
- Dirty, uncommitted Stage 3.1 effect slice: `ModelEffectKind`, `model_effects`, `builtins.print` and its fixture, shared formal checks and compiler output version 26. Its focused Nextest command was interrupted without a result. Schema/codebook/rule snapshots, Clippy, design review and commit are still due. Preserve this work in place across the restart.

## Last verified (2026-09-24)

| Command | Outcome |
|---|---|
| `just test-all` at `d61a497` | passed: the last recorded complete integrated gate, before later Stage 3 commits. |
| `just pilot build/store-stage3-entry-links-v2-2026-09-24` at `d61a497` | passed: snapshot `fc9dc0f3bc6007fc26ceae6619f2996a`, 102 positive links, 20/20 smoke briefs; before later Stage 3 commits. |
| Focused release Nextest selections for `6110afd` and `000542f` | passed: 7/7 each, including positive/withholding/tamper, snapshots, table count and ledger. |
| `cargo clippy --release -p cpg-schema -p cpg-core --all-targets -- -D warnings` at each Stage 3.2 slice | passed. |
| `cargo fmt --all` on the dirty effect slice | passed; formatting only, no behavioral result. |
| Focused `cargo nextest run --release -p cpg-core -p cpg-schema -E 'test(pinned_cast_model_requires_and_publishes_its_real_formal) | test(committed_catalog_has_typed_identity_path_and_digest)' --no-tests=pass` on the dirty effect slice | not_run to completion: turn interruption left no reported result; no Cargo/Nextest/rustc process remained. |
| `just adr index`; `just adr revisit`; `git diff --check` | passed; ADR-0002's automatic `just deps` trigger passed, other reported triggers require manual evidence. |
| `just test-all`; fresh `just pilot` after `d61a497`; Stage 3 Q01/Q03/Q05/Q09; clean wheel/native query | not_run: reserved for the assembled Stage 3 end at operator direction. |

## Known failures, decisions and blocks

- No current product-test failure is established for the dirty effect slice; it is **unverified**. The fixture assumes `builtins.print` resolves to a pinned definition with complete signatures, and the test expects two targets. Check that assumption before accepting snapshots; retain fail-closed formal validation if it does not hold.
- ADR-0020, ADR-0024 and ADR-0025 remain proposed. Reviews defer generic model semantics and pilot binding (M02–M04), exception catch/completion (X02, H01–H02), and final pilot cost/counts (X03, H03).
- Remaining Stage 3: broader models, resolved handler fates, callbacks/resources, finite summaries, Pysa/Hypothesis oracles, FORMAT 7 serving and the pre-registered exit questions. Source observations and authored model rows cannot be promoted to behavioral verdicts without these proofs.

## Next

Finish and verify the dirty `model_effects` slice in place: run its focused positive/tamper case, inspect and accept schema/codebook/rule snapshots, run focused ledger and Clippy, then update DESIGN/review and commit. Keep the integrated tests and pilot at the assembled Stage 3 end.
