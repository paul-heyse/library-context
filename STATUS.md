# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

## Current work: reconstruct semantic model phases 0–2

- **Accepted target:** ADR-0085 typed definitions, ADR-0086 immutable PostgreSQL generations,
  ADR-0087 no compatibility, ADR-0088 canonical chunks. **No phase exit is qualified.**
- [Cutover plan §4.2](docs/plans/semantic-model-cutover-plan_2026-09-29.md#42-execution-status)
  owns implementation/verification boundaries; §8 owns findings. Phase 0 remains reopened.
- **Existing foundation:** nominal records/sums/assertions, generated Arrow/PG lowerings, captured
  input/chunks, occurrence BDDs, qualified native calls, transfers, derivations and indexed attachments.
- **Stage/store correction:** schedule identities include profile/effect/code/config; an execution
  binds one sink. Sealing requires generation-specific successful writes, including explicit empty
  outputs. A no-op callback or foreign receipt cannot certify unwritten output.
- **Representative lexical/document/flow contracts:** typed relationships and generated supports
  preserve attributed alternatives. Shared checks enforce structural ancestry, document spans and
  raw flow source geometry; optional subjects and transitive Place roots retain provenance.
  These are contract fixtures, not complete migrated producers. Bounded reviews accepted corrections.
- **Consolidated ownership:** one policy serves assertion support and stored coverage; matching
  provider/context cannot authorize a foreign input. Explicit corpus membership remains required.
- **Codec allocation:** borrowed key hashing and streamed borrowed Arrow encoding remove intermediate
  key/payload copies; builders retain known row preallocation. Full accounting/RSS remains open.
- **Types:** structural terms, recursive restrictions and native-owner/fidelity checks pass model and PG18 controls.
- **Deployment:** typed reported values/collections and source checks pass model3/PG18 three-case controls; explicit lease release preserves two-reader exclusion.
- **Still open:** whole-call/root composition and opaque
  rebasing; coordinated resources (including total syntax traversal work), complete coverage/admission
  and assembled P0 review; P1 provider/store/CLI cutover and Delta deletion; full P2 producer migration.
- **Authorized scope:** model/store/facts. Downstream runtime suspends at P1 until P3–5.
  The old pipeline remains active. No compatibility adapter or old-ID bridge was introduced.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`; all receipts are bounded.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain` | passed: 18 after codec preallocation correction |
| `cargo test --release -p lctx-model --test domain_lexical` | passed: four, including two review corrections |
| `cargo test --release -p lctx-model --test domain_documents` | passed: three after enclosing-heading correction |
| `cargo test --release -p lctx-model --test domain_coverage` | passed: input/corpus ownership matrix |
| `cargo test --release -p lctx-model --test domain_flow` | passed: three after cross-file structure correction |
| `cargo test --release -p lctx-model --test domain_types --test domain_assertions`; `cargo test --release -p lctx-postgres --test domain_types` | passed: four + three model controls and real PG18 four-case type test |
| `cargo test --release -p lctx-model --doc` | passed after codec change: seven positive/seven negative; one old ignored |
| `cargo test --release -p lctx-postgres --test domain_lexical` | passed: real PG18 good/foreign-value cases |
| `cargo test --release -p lctx-postgres --test domain_coverage --test domain_documents` | passed: real PG18 good/foreign scope/span cases |
| `cargo test --release -p lctx-postgres --test domain_flow` | passed after correction: real PG18 good/foreign-place cases |
| `cargo test --release -p lctx-postgres --test generation_stages --test generations` | passed after sink correction; generations also passed after codec change, including 65 MiB |
| `cargo check -p cpg-extract -p cpg-core` | passed after codec/composite-subject changes, before final flow source invariant |
| `just docs-check` | passed: 193 canonical pages and zero link errors after MDX source-reference correction |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

Earlier producer/capture/call/transfer/proof/runtime receipts and limits remain in plan §4.2.
Review acceptance is bounded; complete raw-field parity and enclosing architecture remain open.

## Next

Finish P0 whole-call composition/rebasing, resource/coverage admission
and assembled review. Then execute the plan's P1 hard cutover and P2 migration/deletion sequence.
Integrated gates wait until all authorized functional scope is implemented.
