# Status

_Updated 2026-09-22 by the handoff skill._

## Where we are

- The **development environment is bootstrapped** (ADR-0001). Increment 1 (fact substrate,
  DESIGN §1.2) has not started.
- **Pinned family verified:**
  - DataFusion 55.1.0 / Arrow 59.3.0 / object_store 0.13.2
  - delta-rs git `58f07cd6` + kernel `8ba063f8`
  - The `cpg-schema::family_smoke` tests write two Delta commits and query them through
    DataFusion (ADR-0002, `Tested`).

## Last verified (2026-09-22)

| Command | Outcome |
|---|---|
| `just test-all` | passed: 2 Rust tests, 9 Python tests, adr lint, lint-agents, deps |
| `just rules-scan` | not_run (no rules yet) |
| `just fixtures-check` | not_run (no fixtures yet) |

## Known failures and blocks

None.

Two notes:
- cargo-deny 0.20.2 misses duplicate versions that enter only through dev-dependencies, so
  `scripts/check_family.py` owns the family check (ADR-0002).
- `just <several recipes>` swallows later recipe names as `test` arguments. Run
  `just check`/`test-all` rather than chaining recipe names after `test`.

## Open decisions

- **ADR-0003** (separate adapter workspaces) is `proposed` until the first adapter exists.
- **Ruff/Pyrefly revisions** for the adapters. The candidates are in `docs/pins.md` under
  "Analyzers". This is the first decision of increment 1.
- **The 66-schema reference package** cited by Initial_plan (Arrow schemas, registries,
  reference Rust) is not in the repo. Add it under `docs/initial_plan/` if it's available.

## Baseline design review (2026-09-22)

`docs/design_review/reviews/design_review_design-spine-baseline_2026-09-22.md`, compact.

- **Decision:** Not Accept as a decidable specification for increment 1. G3 and G7 pass; G1, G2,
  G4, G5 and G6 are unresolved.
- **Fixed in DESIGN.md:** F2 (rules dropped in condensation, restored), F6 (validators placed),
  O1, O2.
- **Open, all belonging to slice 1:**
  - **F1:** declare only the tables and codebooks slice 1 emits, including the
    `extraction_mode`/`modality`/`model` and resolution `status`/`domain` domains, plus the
    edge-kind/endpoint registry.
  - **F3:** the publication protocol needs an ADR. The suggested answer is an append-only
    `snapshots` Delta table whose commit is the act of publication, with readers filtering by
    `snapshot_id`.
  - **F4:** the run contract and the inputs each ID kind is derived from.
  - **F5:** `coverage` and `resolution_issues` tables, and a definition of "fact family".

## Next

Increment 1, slice 1:
1. Choose the analyzer revisions (ADR).
2. Settle F1/F3/F4/F5 while defining the first `cpg-schema` family (`nodes`/`facts`/`edges`/
   `spans`/`coverage`) with schema snapshots and codebooks.
3. Write the first fixture.
