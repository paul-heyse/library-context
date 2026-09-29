# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main; phase 0 accepted (scoped), phase 1 started._

## Restart checkpoint: semantic model cutover

- **Accepted target:** ADR-0085 typed definitions, ADR-0086 immutable PostgreSQL generations,
  ADR-0087 no compatibility, ADR-0088 canonical chunks, ADR-0089 stage contributions and input
  closure. [Cutover plan §4.1.1](docs/plans/semantic-model-cutover-plan_2026-09-29.md) owns the order;
  §4.2 owns dated receipts; §8 owns finding dispositions.
- **Phase 0 is complete and its exit review is Accept scoped** (2026-09-29,
  [P0 exit review](docs/design_review/reviews/design_review_p0-exit_2026-09-29.md), F01 re-inspected
  at `7595557`). It excludes the P4 composition engine (P0 exit F03, composition F04/F06/F07) and
  store-side frontier enforcement (P0 exit F02, P1.7). No phase is release-qualified.
- **Phase 0 contracts (Implemented, focused-Tested):**
  - resources: reserved batches, charged validators, the Ruff traversal bound (R1–R3);
  - stages and `MemoryGeneration` (D0); declarations, call-site facts and the owner rule (C1–C3);
  - witnessed whole-call composition with `Entry` ports (C4–C5r); old semantics deleted (C6);
  - the facts frontier and admission (D1);
  - modality-aware verdicts (X0 F01);
  - the capture and syntax subset (E1), measured in
    [evidence](docs/design_review/evidence/2026-09-29_p0e-subset-envelope/README.md).
- **P1.1 (committed with this handoff):** `lctx` keeps `library`, `acquire`,
  `deployment-identity`, `flow` and `runs`, and `compile` exits 3. Removed: the Delta, serving and
  parity commands, the `lctx-analytics`/`lctx-embed` dependencies and the pilot recipes.
  `lctx_mcp` and its smoke check exit 3.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`; per-slice receipts: plan §4.2.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model` (all suites, incl. admission, composition, verdicts) | passed at `7595557` |
| `-p lctx-postgres --test domain_composition --test domain_stability --test generations --test generation_stages` | passed (real PG18), at the slices that last changed them |
| `-p cpg-extract --test typed_conformance --test typed_limits --test typed_owner` | passed at `c4c7cba` (incl. PG18 conformance generation) |
| `cargo test --release -p lctx` (P1.1 working tree) | passed: 5 unit, 5 `acquire` incl. compile exits 3 and never runs `uv`/`git` |
| `lctx_mcp`, `lctx_mcp.smoke`, `lctx flow`, `lctx compile`; `just build-features` (P1.1) | exit 3, exit 3, JSON, exit 3; no Hakari change |
| `cargo check --workspace --all-targets` | passed on the P1.1 tree |
| `just fmt`, `just test-all`, `just docs-check`, facts pilots | not_run: functional scope incomplete (P0–P2) |

## Known open items

- P0 exit F02 (frontier-scoped schemas; typed `Frontier` read refusal) and F07 (typed store
  failures) are written into P1.7's controls. C01 and C02 are routed to P1.7/P1.10.
- Input-validation F02 is narrowed to A0, P1.9/P1.10 and Q. Other routed findings (resource review, composition review, P0 exit F03–F08) are in plan §8.
- Two concurrent rust-analyzer flychecks deadlock on build-unit locks (hung twice on 2026-09-29 and
  terminated). If builds hang with no `rustc` running, look for them.

## Resume here

1. P1.2–P1.13 in plan order.
   - P1.2 removes the partition kernel and parity adapters, P1.3 the Delta runtime, and P1.4 the
     delta-rs family (ADR-0090 supersedes ADR-0002).
   - Then the service baseline, generated install/check/reset, the attempt-owned lifecycle (with P0
     exit F02/F07), the catalog, the provider fork and sessions, the CLI, ops, and the operator
     database transition.
   - The operator authorized the fork push, the database transition and `build/` copy deletion
     without further confirmation (2026-09-29).
2. P2 (A0 … C3x), then Q qualification. The bounded reviews named in §4.1.1 continue.
