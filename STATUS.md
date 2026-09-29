# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Restart checkpoint: semantic model phases 0–2

- **Accepted target:** ADR-0085 typed definitions, ADR-0086 immutable PostgreSQL generations,
  ADR-0087 no compatibility, ADR-0088 canonical chunks. **No phase exit is qualified.**
- Stopped at the user's restart boundary after completing local guard and transaction corrections.
  [Cutover plan §4.2](docs/plans/semantic-model-cutover-plan_2026-09-29.md#42-execution-status)
  owns detailed implementation/verification limits; §8 owns findings. Phase 0 remains reopened.
- **Implemented foundation:** nominal records/sums/assertions, generated Arrow/PG lowerings,
  captured inputs/chunks, occurrence BDDs, qualified calls, transfers, derivations and attachments.
- **Stage/store:** one execution-bound sink; exact successful output receipts, including empty output.
  Generation transactions acknowledge rollback on refusal; active COPY explicitly aborts first.
  Failed finalization remains unconfirmed. Explicit reader release acknowledges unlock/close.
- **Representative domains:** lexical, document, raw flow, structural type and deployment records
  preserve attributed alternatives, transitive source ownership and opaque/display-only fidelity.
  These are contract fixtures, not complete migrated producers. Bounded reviews accepted corrections.
- **Guards:** typed invocation origins preserve distinct/nested caller sites; shared construction and
  stored checks refuse unsupported formal/receiver rebasing. Source checks follow the full lineage.
- **Ownership/allocation:** shared assertion/coverage scope policy; borrowed identity hashing and
  streamed codecs remove known intermediates. Complete allocation/RSS accounting remains open.
- **Authorized scope:** model/store/facts; downstream runtime suspends at P1 until P3–5.
  The old pipeline remains active. No compatibility adapter or old-ID bridge was introduced.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`; receipts are bounded.
Earlier domain/capture/producer/runtime receipts remain in plan §4.2.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain_types --test domain_assertions` | passed: four type and three assertion controls |
| `cargo test --release -p lctx-postgres --test domain_types` | passed: real PG18 four-case type test |
| `cargo test --release -p lctx-model --test domain_deployment` | passed: three controls |
| `cargo test --release -p lctx-postgres --test domain_deployment` | passed: three-case contract and two-reader release controls |
| `cargo test --release -p lctx-model --test domain_guard_rebase` | passed after F02: five controls |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase` | passed after F02: real PG18 eight-case test |
| `cargo test --release -p lctx-postgres --test domain_guard_rebase --test domain_transfer --test generation_stages --test generations` | passed after rollback/COPY correction: five tests incl. active COPY refusal and 65 MiB; before final guard F02, separately rerun above |
| `cargo check -p lctx-postgres`; `cargo check -p cpg-extract -p cpg-core` | passed; final guard F02 subsequently compiled by its tests |
| `just docs-check` | passed: 197 canonical pages, zero link errors |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

## Resume here

1. **P0 / lctx-model:** complete whole-call binding/root composition and stability-evidenced formal
   substitution; coordinated resources (including total Ruff traversal work), exact coverage and
   production admission; then assembled P0 review. Local opaque rebasing is now implemented.
2. **P1 / store/runtime/CLI:** installation verification/reset, service classification, provider
   lease/stream integration, hard PostgreSQL cutover and Delta deletion. Qualify cancellation and
   transport/finalization failures; current focused lifecycle controls do not cover these.
3. **P2 / producers:** migrate every raw relation/field in plan §4.1.2, integrate acquisition/facts
   assembly and frontier admission, delete old authority, and rebuild owned state from pinned inputs.

Follow plan §4.1.1 dependency order. Run formatting and integrated gates only after all authorized
functional scope is implemented; bounded review acceptance does not qualify the enclosing architecture.
