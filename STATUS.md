# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main; phase 0 accepted (scoped); phase 1 implemented through P1.12, P1.13 rehearsed._

## Restart checkpoint: semantic model cutover

- **Accepted target:** ADR-0085 typed definitions, ADR-0086 immutable PostgreSQL generations,
  ADR-0087 no compatibility, ADR-0088 canonical chunks, ADR-0089 stage contributions and input
  closure, ADR-0090 the DataFusion family without delta-rs.
  - [Cutover plan §4.1.1](docs/plans/semantic-model-cutover-plan_2026-09-29.md) owns the order.
  - §4.2 owns dated receipts; §8 owns finding dispositions.
- **Phase 0** is complete; its exit review is Accept scoped. It excludes the P4 composition engine.
- **Phase 1 (implemented, focused-Tested 2026-09-29; P1.1–P1.12 committed, `cd22ca6`…`cb0e5e1`):**
  - service baseline and verified owner; generated install, live-catalog `store check`, phased
    resumable `store reset`;
  - attempt-owned lifecycle with failed/interrupted states, facts admission and frontier-scoped
    schemas; generation catalog;
  - bounded provider fork `09cc8a8`; generation-bound provider sessions over a driver-neutral
    lease (`cpg-core/src/generation_read.rs`);
  - CLI `model describe`, `store`, `generation`, `query`; operator transition, bootstrap and
    runbook (`docs/postgresql.md`).
- **Reviews:**
  - The P1.5–P1.7 [store-lifecycle review](docs/design_review/reviews/design_review_p1-store-lifecycle_2026-09-29.md)
    returned Revise. F01–F07 are corrected in `5beafd8`; dispositions are in plan §8.
  - A re-inspection of those corrections and the bounded P1.10 provider-session review were
    running at this checkpoint. Their outcome is not recorded here.
- **Parallel operator work:** the library-utilization catalog and resolver (`645a8bf`, `e78405a`)
  was committed at the operator's request. This session ran only its tests.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`; per-slice receipts: plan §4.2.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-postgres`: services, installation, lifecycle, generation_catalog, generations, generation_stages, 13 `domain_*`, `--doc` | passed at `5beafd8` (run twice) |
| `-p cpg-core --test generation_read --test model_runtime`; `-p cpg-extract --test typed_conformance --test typed_limits`; `-p lctx-model` | passed at `5beafd8` |
| `INSTA_UPDATE=no -p lctx --test store_cli --test model_describe --test acquire --bin lctx` | passed at `244eda4` |
| `LCTX_POSTGRES_TEST=1 uv run pytest tests/scripts/test_postgres_transition.py` | passed at `c65c0c7` |
| Transition rehearsal from the operator database ([evidence](docs/design_review/evidence/2026-09-29_operator-transition/README.md)) | passed at `c65c0c7` |
| fork `cargo test -p datafusion-table-providers-postgres --no-default-features --lib` | passed at `09cc8a8` |
| `cargo check --workspace --all-targets`; `check_agents.py`; `adr.py lint` | passed at `c65c0c7` |
| `just fmt`, `just test-all`, `just docs-check`, facts pilots | not_run: P0–P2 functional scope incomplete |

## Known open items

- **P1.13 real transition is blocked.** It needs the PostgreSQL superuser, and this session's sudo
  requires a password. The operator's commands are in the
  [evidence](docs/design_review/evidence/2026-09-29_operator-transition/README.md). The operator
  database has empty retained service tables; `lctx` refuses its legacy history until it is moved.
- **Routed findings (plan §8):**
  - P0 exit F03 → P4; F04 → A0; F05 → B1; F08 → A4;
  - store-lifecycle F08 → P3; F09 → Dc;
  - input-validation F02 → A0, P1.9/P1.10 and Q.
- **Deadlocking editor flychecks.** Two concurrent rust-analyzer flychecks can deadlock on
  build-unit locks. If builds hang with no `rustc` running, look for them.

## Resume here

1. Read the background reviews' outcomes:
   - the re-inspection appended to the store-lifecycle review;
   - `docs/design_review/reviews/design_review_p1-provider-sessions_2026-09-29.md`.

   Correct any Revise findings, and record C01/C02 closure in plan §8.
2. P2 in plan order: A0 (provider framework: `ProviderStage`, `StageContext`, `compile_facts`)
   → A1–A16 → B1–B3 → Dc (`lctx compile --through facts`) → C1x–C3x. Then Q qualification and
   the assembled P0–P2 review.
3. After the operator's P1.13 run, record its receipts and delete the transition tool.
