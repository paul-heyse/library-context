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

## Next

1. Run the baseline `compact` design review of DESIGN.md (`design-reviewer` subagent).
2. Increment 1, slice 1:
   - choose the analyzer revisions (ADR);
   - define the first `cpg-schema` table family (`nodes`/`facts`/`edges`/`spans`) with schema
     snapshots and codebooks;
   - write the first fixture.
