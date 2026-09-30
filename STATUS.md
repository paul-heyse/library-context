# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main; phase 0 accepted (scoped); phase 1 implemented (P1.13 rehearsed); phase 2 through A3._

## Design standard (2026-09-29)

- **Implemented policy:** [ADR-0091](docs/adr/0091-semantic-model-first-design.md) adopts core/template
  3.1 and profile guidance 1.2. Domain modeling is mandatory; FP-04/A2 require an adequate model
  governing behavior. Reviewer, process skills, AGENTS and DESIGN §2 are aligned. Library
  exploration and capability skills are unchanged; this does not qualify product code.
- `just docs-check`: **failed** on five existing review links to deleted cutover files
  (`attempt.rs`, `db.rs`, `snapshot.rs`, catalog `pr4.rs`, `rebuild.rs`). Its ADR and agent checks
  **passed**. HEAD inspection confirmed all five broken links predate this policy change.
- Generic skill validation (`uv run --no-project --offline --with pyyaml python
  /home/paul/.codex/skills/.system/skill-creator/scripts/quick_validate.py`): with
  `.claude/skills/design-review-code-intelligence`, **passed**; with `.claude/skills/design-review`,
  **failed** on unchanged `model-baseline`/`user-invocable` metadata. Repository agent validation accepts it.
- Product checks: `just test-all`, facts pilots **not_run** for this documentation scope.
  Concurrent P2 work, including A0 in `de89800`, was not validated by this session; the checkpoint
  and receipts below remain attributed to the earlier cutover work. Next policy work: apply core
  3.1 at the next scheduled review; publication link cleanup remains open.

## Restart checkpoint: semantic model cutover

- **Accepted target:** ADR-0085–0091. [Cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md)
  §4.1.1 owns the order, §4.2 dated receipts, §8 finding dispositions.
- **Phase 0** is complete; its exit review is Accept scoped. It excludes the P4 composition engine.
- **Phase 1 (implemented, focused-Tested 2026-09-29; P1.1–P1.12):**
  - service baseline and owner; generated install, `store check`, phased `store reset`;
  - attempt-owned lifecycle; generation catalog; bounded provider fork `09cc8a8`;
  - generation-bound provider sessions; CLI `model`/`store`/`generation`/`query`; ops tooling.
- **Reviews:**
  - the [store-lifecycle review](docs/design_review/reviews/design_review_p1-store-lifecycle_2026-09-29.md):
    re-inspection is **Accept scoped** (F01–F07 closed; F08/F09 deferred);
  - the [provider-session review](docs/design_review/reviews/design_review_p1-provider-sessions_2026-09-29.md):
    Revise, corrected in `7c1594d`; re-inspection is **Accept scoped** (F01–F03, F05 and F06's
    relation set closed; F04/F07 open with triggers);
  - core C01 and C02 are closed.
- **Phase 2 (in progress; focused-Tested 2026-09-29):**
  - **A0** `de89800`: provider framework (`ProviderStage`/`Declared`, `StageContext`, `run_stage`
    over a window-2 channel, `compile_facts`) and the `Attacher`. 41 MB of output peaked at
    3.27 MB. P0 exit F04 is closed;
  - **A1** `0213048`: acquisition classes (owned, unowned, derived), `DerivedArtifact`,
    `EnvironmentFingerprint`. Facts admission requires a class for every artifact;
  - **A2** `a7c71a1`: pure `acquisition::inventory`, verified closure capture with `_lctx/`
    derivations, and the `acquire` provider. The legacy source-tree writes, `Release::corpus`
    and the release/captured rows are deleted; a RECORD is digested, not captured (ADR-0089
    amendment);
  - **A3** `c87c672`: typed syntax records, `SubjectBoundary` and attachment outcomes
    (ObligationKind 52/53).
- **Parallel operator work:** ADR-0091 (model-governed behavior), skill selections (fixedbitset,
  serde-arrow, pyo3), committed separately at the operator's request.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`; per-slice receipts: plan §4.2.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model` (all suites) | passed at `c87c672` |
| `-p cpg-extract` (all 15 suites: acquisition, capture, bundle, typed_*, legacy) | passed at `a7c71a1` |
| `-p cpg-core --test facts_driver --test generation_read` | passed at `a7c71a1` / `7c1594d` |
| `-p lctx-postgres`: lifecycle, installation, generations, generation_stages, generation_catalog, domain_input, domain_syntax | passed at `c87c672` |
| `INSTA_UPDATE=no -p lctx --test model_describe --test store_cli` | passed at `c87c672` (snapshot migrated at A1 and A3) |
| `cargo check --workspace --all-targets`; `adr.py lint` | passed at `c87c672` / `a7c71a1` |
| `just fmt`, `just test-all`, facts pilots | not_run: P0–P2 functional scope incomplete |

## Known open items

- **P1.13 real transition is blocked.** It needs the PostgreSQL superuser, and this session's sudo
  requires a password. The operator's commands are in the
  [evidence](docs/design_review/evidence/2026-09-29_operator-transition/README.md).
- **Routed findings (plan §8):**
  - P0 exit F03 → P4, F05 → B1, F08 → A4;
  - store-lifecycle F08 → P3, F09 → Dc;
  - provider-session F04 → before P3/P4 stage sessions, F07 → before the first family-scoped reader;
  - input-validation F02 → P1.9/P1.10 remainder and Q;
  - F11 → P2.
- **`just docs-check` fails on five stale review links** (the operator's note above); fix at C3x/Q.
- **Deadlocking editor flychecks.** Two concurrent rust-analyzer flychecks
  (`cargo check --workspace --message-format=json`) can deadlock on build-unit locks. It happened
  again on 2026-09-29: builds hung with no `rustc` running. Stopping the two flycheck `cargo`
  processes released them.

## Resume here

1. **A4, the `pyrefly` stage, phase 1** (consult `pyrefly-ruff`). A provider over `ProviderStage`
   emits complete typed syntax (occurrences, placements, details, declarations, imports,
   parameters, class fields, `CallSyntax`), parse-error and undecodable-source coverage, and the
   `__all__` boundary. It replaces `walk.rs`/`syntax.rs` row code, closes P0 exit F08, and runs
   `syntax_shapes`, `unicode_bom` and `dunder_all` known answers.
2. Then A5–A16 → B1–B3 → Dc (`lctx compile --through facts`) → C1x–C3x, then Q qualification and
   the assembled P0–P2 review (core 3.1, ADR-0091).
3. After the operator's P1.13 run, record its receipts and delete the transition tool.
