# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared `main`._

## Current work: reconstruct semantic model phases 0–2

- **Accepted target:** ADR-0085 typed definitions, ADR-0086 immutable PostgreSQL generations,
  ADR-0087 no compatibility, ADR-0088 canonical artifact chunks. No phase exit is qualified.
- [Cutover plan §4.2](docs/plans/semantic-model-cutover-plan_2026-09-29.md#42-execution-status)
  owns the checkpoint; §8 owns findings. Earlier Phase 0 completion remains reopened.
- **Implemented foundation:** typed records/sums/Assertion derives, nominal keys/references,
  required support companions, generated Arrow/PG lowerings; chunked complete-content proofs;
  acquisition/corpus/distribution/ownership checks; typed values and occurrence-keyed BDDs;
  qualified syntax assertions with separate typed support and source/context validation.
- **Execution foundation:** stage capabilities, cancellation/failure refusal, fresh DataFusion
  catalogs and a shared compute/external reservation interface. Full buffer accounting is open.
  Verdicts preserve scope/approximation unknowns; display truncation does not alter truth.
- **Store foundation:** sealed validation/receipts, write drain, leased readback; conformance
  subsets cannot select a facts frontier. Cleanup removes schema/registry/receipts/events;
  complete absence and orphan repair are distinct. Compiler/CLI are not yet connected.
- **Still open:** remaining representative families, call/binding/transfer/control/derivation
  policies, ownership, full resource accounting and coverage admission, real extraction subset
  and assembled P0-E review; P1 store/provider/CLI cutover and Delta removal; all P2 producers.
- **Authorized scope:** model/store/facts. Downstream runtime suspends at P1 cutover until P3–5.
  The old pipeline remains active; no compatibility adapter or old-ID bridge was introduced.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`; exact target commands are in plan §4.2.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain --test domain_conditions --test domain_assertions --test domain_stages --test domain_resources --test domain_verdicts` | passed: 29 controls |
| `cargo test --release -p lctx-model --doc` | passed: six negative/positive pairs; one legacy ignore |
| `cargo test --release -p lctx-postgres --test generations` | passed: two real PG18 tests |
| `cargo test --release -p cpg-core --test model_runtime` | passed: two catalog/capability/shared-pool controls |
| `just docs-check`; `just adr lint` | passed: documentation publication and 44 ADR records |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

Independent artifact and assertion reviewers accepted their bounded corrections. The assertion
review's F01–F03 are closed within that scope; resource qualification and assembled architecture
remain open. Stage/session/lifecycle changes are not independently reviewed yet.

## Next

Complete P0-B representative families and P0-C policies, integrate reservations/coverage and the
pinned producer subset, then obtain the assembled P0-E review. Follow the existing plan through
P1/P2 and its deletion obligations. Integrated gates remain at the end of all functional scope.
