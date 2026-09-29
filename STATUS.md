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
- **Transfer/proof contract:** stable keys and qualified alternatives, separate control/selection,
  identity-only path extension, nominal derivation declarations and generated PG views. Shared
  validation rejects cross-scope call sites/guard operands and cycles through proof-step identities.
- **Resource progress:** attachment buffers and retained ambiguous results own shared reservations.
- **Still open:** lexical/type/flow/document/deployment representative domains; whole-call/root
  composition and opaque rebasing; full resource/coverage admission and assembled P0-E review;
  P1 provider/store/CLI cutover and Delta removal; P2 full raw-producer migration and reconstruction.
- **Authorized scope:** model/store/facts. Downstream runtime suspends at P1 until P3–5.
  The old pipeline remains active. No compatibility adapter or old-ID bridge was introduced.

## Focused verification (2026-09-29)

Cargo commands use `python3 scripts/build_environment.py --`. These are bounded receipts.

| Command | Outcome |
|---|---|
| `cargo test --release -p lctx-model --test domain_transfer --test domain_assertions --test domain_paths` | passed: reviewed five transfer/three assertion/two path; author follow-up six transfer/three assertion |
| `cargo test --release -p lctx-model --test domain --test domain_resources` | passed: 17 domain + three resource; independently rerun |
| `cargo test --release -p lctx-model --test domain_derivation` | passed: three controls; independently rerun after source-identity correction |
| `cargo test --release -p lctx-postgres --test domain_derivation` | passed: real PG18 four-case proof/grant test; independently rerun |
| `cargo test --release -p lctx-postgres --test domain_transfer` | passed: real PG18 transfer/control/selection/readback, call-site/guard-operand scope refusal and views |
| `cargo test --release -p lctx-model --doc` | passed: seven positive/negative pairs; one old ignored example |
| `cargo check -p cpg-extract -p cpg-core` | passed after generated invariant API change |
| `cargo test --release -p lctx-postgres --test generations` | passed: two lifecycle/chunk controls after generated proof views |
| `just build-features` | passed: union refreshed; model macro crate excluded; petgraph pin unchanged |
| `just docs-check` | passed: 187 canonical pages, zero link errors |
| `just fmt`, `just test-all`, facts pilots | not_run: functional scope incomplete |

Earlier capture/native-syntax/call/runtime/lifecycle receipts and their limits remain in plan §4.2.
Independent bounded reviews accepted transfer/path, attachment accounting and corrected derivations.
Transfer F01 and nominal-derivation F01 are closed at that scope. No enclosing phase is qualified.

## Next

Continue P0 representative families, whole-call composition/rebasing and coordinated resource/
coverage admission; then assembled P0 review. Follow the existing P1/P2 execution and deletion order.
Integrated gates wait until all authorized functional scope is implemented.
