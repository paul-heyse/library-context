# Status

_Updated 2026-09-29 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared `main`._

## Current work: reconstruct semantic model phases 0–2

- **Accepted target:** ADR-0085 typed definitions, ADR-0086 immutable PostgreSQL generations,
  ADR-0087 no compatibility, ADR-0088 canonical artifact chunks. No phase exit is qualified.
- [Cutover plan §4.2](docs/plans/semantic-model-cutover-plan_2026-09-29.md#42-execution-status)
  owns implementation/verification boundaries; §8 owns findings. Earlier Phase 0 remains reopened.
- **Implemented foundation:** ordinary typed records/sums/Assertion derives, nominal references,
  generated Arrow/PG lowerings, qualified assertions/support, canonical chunks/content proofs,
  acquired-input ownership, occurrence-keyed BDDs, typed values and indexed attachments.
- **Capture/extraction:** selected inputs freeze into an owned tree with copy/chunk/metadata
  reservations and change detection. The pinned native-AST subset emits typed occurrences and
  identifier assertions; real PG conformance uses that capture. Coverage remains Partial/Unavailable.
- **Call contract:** native provider-qualified symbols, complete signatures and target sets, explicit
  unknown receivers, whole-variant binding and named policies. Binding kinds/aggregate coordinates
  survive; wrong-provider support and forged shape maps refuse. P2/P3 integration remains open.
- **Execution/store:** stage capabilities, cancellation/failure refusal, fresh DataFusion catalogs,
  shared reservation interface, sealed validation/receipts, write drain and leased readback.
  Conformance subsets cannot select facts; cleanup distinguishes absence from orphaned state.
- **Still open:** lexical/type/flow/document/deployment representative domains; transfer/control/
  selection/derivation policies; full resource/coverage admission and assembled P0-E review;
  P1 provider/store/CLI cutover and Delta removal; P2 full raw-producer migration and reconstruction.
- **Authorized scope:** model/store/facts. Downstream runtime suspends at P1 until P3–5.
  The old pipeline remains active. No compatibility adapter or old-ID bridge was introduced.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`. These are bounded receipts.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain --test domain_calls` | passed: 17 domain + seven call controls; call reviewer independently reran seven |
| `cargo test --release -p lctx-postgres --test domain_calls` | passed: real PG18 contract fixtures/readback and wrong-provider refusal |
| `cargo test --release -p cpg-extract --test capture --test typed_conformance` | passed: three capture + one native Pyrefly/PG18 control |
| `cargo test --release -p cpg-extract --lib capture::tests` | passed: changed-during-capture refusal/cleanup |
| `just docs-check` | passed: 184 canonical pages, zero link errors |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

Checkpoint `1e203df` also recorded 29 foundational model controls, paired doctests, two real PG18
lifecycle controls and two runtime controls; exact commands and limits remain in plan §4.2.
Capture reservations do not establish total process RSS. Native-call PG tests use contract fixtures;
the native syntax test uses the actual pinned parser. Neither qualifies a complete facts producer.

Independent bounded reviewers accepted native syntax, capture and corrected call contracts.
Call-review F01–F03 are closed at that scope. Assembled P0 and full resource qualification remain open.

## Next

Continue P0 representative families, transfer/control/derivation policies and coordinated resource/
coverage admission; then assembled P0 review. Follow the existing P1/P2 execution and deletion order.
Integrated gates wait until all authorized functional scope is implemented.
