# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared `main`._

## Current work: reconstruct semantic model phases 0–2

- **Accepted target:** ADR-0085 typed domain definitions, ADR-0086 immutable PostgreSQL generation
  schemas, ADR-0087 clean reconstruction without compatibility. No phase exit is qualified.
- The [cutover plan §4.2](docs/plans/semantic-model-cutover-plan_2026-09-29.md#42-execution-status)
  owns the implementation checkpoint and §8 owns findings. Earlier Phase 0 completion is reopened.
- **Implemented foundation:** bounded domain/codebook/sum derives, nominal IDs and keys, generated
  Arrow codecs, subtype references, model validation and typed scheduling. Source/provider/support/
  coverage subset round-trips through ordinary PostgreSQL generation schemas. Content manifests,
  acquisition/verification/ownership, source bounds and invocation family membership have sealed
  model-owned invariants. Indexed occurrence attachment preserves ambiguity.
- **Store foundation:** COPY, write-draining seal, stored validation, atomic publication, pool-bound
  reader leases, selection/retirement and failed-attempt cleanup. Not wired into compiler/CLI.
- **Still open:** remaining domain families and implementation of the 57-family raw-field inventory, semantic policies,
  stage execution and session isolation, coordinated memory budgets, CLI/runtime cutover, producer migration
  and deletions. Old runtime/framework remain present; no compatibility adapter was introduced.
- **Authorized scope:** model/store/facts. Analysis/catalog/MCP suspend at runtime cutover and return
  only in phases 3–5. Product work remains paused. Protected evidence and evaluation isolation remain.

## Focused verification (2026-09-29)

Build commands use `python3 scripts/build_environment.py --`.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain` | passed: 14 domain controls |
| `cargo test --release -p lctx-model --doc` | passed: five negative/positive pairs; one legacy ignore |
| `cargo test --release -p lctx-postgres --test generations` | passed: real PG18 vertical slice and lifecycle refusals |
| `just docs-check`; `uv run python scripts/adr.py lint` | passed: docs/decision checks |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

The bounded input reviewer accepted manifest/span/ownership corrections; coordinated memory
qualification remains open in plan §8. Attribution and indexing were outside that review.
This bounded review and focused tests do not qualify the reconstructed architecture or product.

## Next

Finish P0.2–P0.5 contracts and inventory, obtain the assembled P0.6 review, then cut over runtime/store
and migrate all raw producers. Follow the plan's compile/focused-test timing; integrated gates remain
at completion of all functional scope.
