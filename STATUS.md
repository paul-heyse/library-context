# Status

_Updated 2026-09-27 under the [handoff skill](.claude/skills/handoff/SKILL.md); work is on `main`._

## Increment and checkpoint

- **Increment 4 / Stage 3 is resumed and functionally incomplete.**
  [Forward plan §3.0](docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
  ("Remaining sequence", P0–P7, exit-driven done bar) owns execution; §6 owns finding disposition.
- **P0 done:**
  - `2c6a59c`: qualified unknown→partial rubric rule, evaluation-only requests and packet
    digests, committed before any Stage 3 packet was read.
  - `50969ba`: baseline diagnostic,
    [plan §1.2](docs/plans/behavioral-model-forward-plan_2026-09-24.md#12-stage-3-baseline-diagnostic-2026-09-27-not-qualification).
  - `ddcc149`: the native extension owns its file list and row caps.
  - `8a7c936` + `a8f6b49`: P0.7 builtin descriptor exemption from decorator withholding
    (compiler 103; [review](docs/design_review/reviews/design_review_descriptor-exemption_2026-09-27.md)
    F01–F03 corrected).
  - The `missing_evidence` shift is ty's AMBIGUOUS reachability, withheld by design; its
    repair is a §7 row.
- **P2 partly done:** `8c3ca5c` adds `lctx compile-fixture`, the served-claim CPython challenge
  (`tests/scripts/test_semantic_soundness.py`) and `just oracles`. The original W5/W7/W12
  default-cap traces (P2.3) are open.
- **P1 implemented, change review running:**
  - `9263af8`: ADR-0064, after the [target review](docs/design_review/reviews/design_review_stage3-discharge-target_2026-09-27.md).
  - `220a0c8`: summaries refuse decorated functions (compiler 104; a live F02 defect).
  - `d8fa5f5`: claim-keyed `behavior_discharges`; return claims graded after summaries
    (compiler 105, FORMAT 10); native citation admission.
  - Pilot `call_transfer` cannot move yet: no pilot summary crosses a call (target review O1).
- **Versions:** compiler output **105**, extractor **33**, synthesis template **20**, catalog
  **7**, FORMAT **10**. Schema migrations since the checkpoint: the `behavior_discharges`
  table, two codebooks, three replacement rules and the serving known answers. A fresh store is
  needed under ADR-0048.

## Last verified (2026-09-27)

`CARGO_TARGET_DIR=/home/paul/library-context/target`, `RUST_MIN_STACK=16777216`.

| Command | Outcome and scope |
|---|---|
| `INSTA_UPDATE=no cargo nextest run --release --workspace` | **passed:** 434 tests after the P1.2 manifest-list update |
| `cargo fmt --all --check`; `cargo clippy --release --workspace --all-targets -- -D warnings` | **passed** |
| `just py-fixture`; `uv sync --frozen --reinstall-package lctx-semantics`; `uv run pytest` | **passed:** includes native discharge admission and the extended CPython harness |
| `just docs-check` (PostgreSQL assessment) | **passed:** 140 pages; 36 records; product checks above were not rerun for this assessment |
| `just test-all`, `just pilot` | **not_run:** end-of-scope acceptance (P7), apart from one diagnostic pilot after P5 |

## Known issues and decisions

- A documentation-only brief for an undocumented, analysis-free seed fails
  `semantic:documentation-only-has-outcome` (fail-closed). Seen only with generated packages;
  unscheduled.
- Target review F08–F11 bind P3/P4 (plan §3.0 rows).
- [PostgreSQL assessment](docs/design_review/reviews/design_review_postgresql-storage_2026-09-27.md): Proposed cache/operational scope alongside Delta;
  adoption findings F01–F04 remain in that review. No database or product change; PostgreSQL integration **not_run**.

## Next

- Settle the P1 change review (`design_review_stage3-discharge-change_2026-09-27.md`), then P2.3
  default-cap traces and P3: `Logger.*` models; CallableFormal/CallableEntry coverage with the
  F08 escape premise; `summary_effects`.
