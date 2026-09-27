# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and checkpoint

- **Increment 4 / Stage 3 is functionally incomplete and resumed.** The operator approved an
  exit-driven remaining sequence P0–P7. The owners are
  [forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
  ("Remaining sequence"), the done bar, §3.3 (added builtins) and §7 (new deferrals). §6 still
  owns finding disposition.
- **P0 in progress.**
  - Landed in `2c6a59c`, before any Stage 3 packet was read: the qualified unknown→partial rule
    (`eval/behavior/README.md`), the evaluation-only requests
    (`eval/behavior/fastmcp-4.0.5.requests.toml`), `structured_eval.py --requests`, input digests,
    source-range marks and a guard test.
  - The baseline diagnostic is recorded in
    [plan §1.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#12-stage-3-baseline-diagnostic-2026-09-27-not-qualification).
    It is an informal reading, not the exit.
  - P0.6 (S6.0 native row-struct cleanup) has not started.
- **Diagnostic findings that bear on the next step** (plan §1.2):
  - Decorator over-withholding: builtin `@classmethod`, `@staticmethod` and `@property` count as
    binding-replacing, giving 2,436 `outside_provider_model` rows.
  - 612 shared rows moved to `missing_evidence`.
  - A field-insensitive `ToolMeta` smear in `FunctionTool.from_function`.
  - Behaviors never consume summaries (`call_transfer` is set unconditionally).
- **Prior checkpoint (unchanged):**
  - Compiler output 102, extractor 33, template 20, catalog 7, FORMAT 9.
  - Scheduler foundation (compiler101) and pilot binding repair (compiler102).
  - Exact receipts are in [plan §1.1](docs/plans/behavioral-model-forward-plan_2026-09-24.md#11-operator-requested-checkpoint-2026-09-27).

## Last verified (2026-09-27)

| Command | Outcome and scope |
|---|---|
| `uv run --no-sync pytest tests/scripts/test_structured_eval_guard.py -q` | **passed:** 3 tests (Stage 3 set 23+/5−, exit-rule hash, requests evaluation-only, source-range mapping) |
| `ruff check`, `ruff format --check`, `pyrefly check` on the changed script and test | **passed** |
| `scripts/structured_eval.py build/generations/cdcf4b4e519e8b79 … --stage 3 --requests …` | **passed:** `build/structured/stage3-baseline-diagnostic.md` written; diagnostic only |
| Read-only verdict diff of generations `7219df40ce349931` → `cdcf4b4e519e8b79` (pyarrow over `behaviors.arrow`) | **passed:** 13,091 shared keys; 914 established/conditional → `outside_provider_model`; 612 → `missing_evidence` |
| `just docs-check` | **passed:** 135 canonical pages, offline links |
| `just test-all`, `just pilot` | **not_run:** no product code changed; end-of-scope acceptance per AGENTS.md. Last full receipts are in plan §1.1 |

## Next

- **Operator decision pending:** whether to add a precision-repair slice for the decorator and
  `missing_evidence` withholding before P1. Without one, P1 proceeds as planned.
- Then P0.6: native typed row structs and one native file list. Focused native and Python checks.
- Then P1: a design/target review (design-reviewer) of discharge ordering, the `summary_effects`
  representation and the generic component driver; an ADR; then S5a discharge in
  `cpg-core::behavior`/`attempt`, `lctx-analytics::summaries`, `validate.rs` and the generation.
- Environment: `CARGO_TARGET_DIR=/home/paul/library-context/target`, `RUST_MIN_STACK=16777216`.
