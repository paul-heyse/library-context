# Status

_Updated 2026-10-02 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Phase 5 implementation is in progress; serving activation is not qualified.** Full execution
of the [Phase 5 plan](docs/plans/semantic-model-phase5-detailed-plan_2026-10-01.md) is authorized.
Its §10 owns package receipts and current assembled-review dispositions. The
[parent cutover §8](docs/plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
owns cross-phase findings. PR6 remains paused until the serving cutover is qualified.

The implemented target uses `lctx-model::domain` as semantic owner, PostgreSQL as canonical
storage, and DataFusion as compute. [ADR-0114](docs/adr/0114-generation-serving-contracts.md)
accepts the closed serving contracts. Current serving code uses an original generation guard,
shared preparation, canonical typed packet/evidence/native hydration and model-owned ranking.
Python supplies numerical effects and validated FastMCP transport. Compilation never selects.
Catalog Flow remains NotRequested; Behavioral uncertainty and all five verdicts stay explicit.
Finite association establishes no allocation, alias, mutation or temporal heap identity.

## Current implementation and receipts

Committed Phase 5 slices include D0 (`a55870c8`), L0 (`5223cf73`), bounded condition allocation
(`441b9c8f`), retained service/evaluation retirement (`17bfe0e4`), canonical selectors
(`0bc8ea11`), and additive model serving/native contracts (`4acf8551`). Canonical store services, bridges, CLI, original controls and ownership-boundary retirement
are committed as `91751d1c`; bounded normalization and qualification repairs are committed
as `3591b68c`. Complete Q0 qualification remains pending.

X0 removed 375 unchanged inventoried source files and 21 obsolete query metadata entries,
including the retired schema, bundle, native IPC and old serving authorities. Three obsolete
wire assets were subsequently inventoried and removed. There is no compatibility reader.
Independent expectations now belong to current model, store, CLI and actual transport controls.
Protected gold, heldout inputs, preregistration and benchmark captures remain preserved.

The [assembled review](docs/design_review/reviews/design_review_phase5-assembled_2026-10-02.md)
was Revise at its original inspected boundary. Independent Appendices A and B accept its
codec and per-assertion fidelity corrections; both findings are Closed / scoped Tested in plan
§10. This is bounded review evidence, not the full Q0 gate. Authored BriefDocument bytes stay
canonical; resources append attributed assertion status/kind/qualification/text/support metadata.

| Current Phase 5 command / boundary, 2026-10-02 | Outcome |
|---|---|
| Normalized `cargo nextest run --release -p lctx-model --test serving_contracts --test ranking_policy` | **composite passed**, 32 / zero skipped; missing test dev dependency repaired |
| Original `cargo nextest run --release -p lctx --test serving_native`, before latest ordering fix | **composite passed**, 1 / zero skipped, 177.59s; both profiles each passed ten actual transport controls |
| Original `cargo nextest run --release -p lctx --test serving_vectors` | **composite passed**, 1 / zero skipped, 120.119s; controlled numerical HTTP seam |
| Focused compile-facts/packets/retrieval/evidence/soundness command | **failed**, nine passed, one failed, one timed out; original receipt retained |
| Corrected `cargo nextest run --release -p lctx --test serving_evidence` | **passed**, 1 / zero skipped, 92.778s; actual BaseEvaluation parent, original bytes and damaged-view controls |
| CPython served-path oracle with scoped 900s harness | **failed**, 833.640s; native preparation exposed receipt/validator row-order conflict |
| Normalized `cargo check --release -p lctx-postgres`, ordering repair | **passed**, 39.63s |
| CPython 3.14 `cargo build --release -p lctx-storage`, ordering repair | **passed**, 19.18s; rebuilt ABI installed |
| Current ordering-discriminator native fixture | **passed**, 1 / zero skipped, 200.266s; both profiles and all original transport/tamper controls |
| Original generated CPython oracle after ordering repair | **passed**, 1 / zero skipped, 795.987s; unchanged original assertions |
| First Phase 5 `NEXTEST_TEST_THREADS=8 just test-all` | **failed at compile**, duplicate service-test import; no workspace test result |
| Corrected `NEXTEST_TEST_THREADS=8 just test-all` | **failed**, Rust 916 passed / eight failed / two skipped; downstream Python, separate PG and doctests not_run |
| Current model/native/wire repair controls | **passed**, 51 selected; signed-cursor and final cache envelope covered |
| CLI exact attempt-ID parser | **passed**, original one selected control |
| Phase 5 `just hygiene` | **composite passed**, all current source/policy checks including workspace/all-target Clippy; live store reset and final zero-finding CLI check passed |
| Both-profile fresh pinned FastMCP Catalog journeys and activation | **in progress / not qualified** |
| Live query embedding | **blocked**, configured endpoint absent; controlled seams establish no live quality |

The focused nine passes include all packet controls, direct mixed-status canonical comparison,
actual eligibility and ranked-winner original bytes, original guard loss/cancellation and the
cumulative 30-second deadline. The evidence fixture originally asked for Local AnalysisInput
parents although Local declares none; the corrected actual BaseEvaluation witness retains all
canonical receipt, damaged-view and captured-byte assertions. Plan §10 owns commands and logs.

The CPython oracle retains its original nine grouped programs / 52 functions and independent
observations. Its longer retry compiled and published the generation, then failed preparation:
receipt content requires increasing IDs while a shared validator requires call/ordinal groups.
The correction hashes in ID order, then streams declared grouping order when different, inside
one read-only repeatable-read transaction on the original leased connection. Both passes remain
bounded. No product budget, deadline, guard or oracle assertion was weakened. The actual native
fixture now has two eight-argument calls demonstrating the order difference before preparation.

The earlier pinned FastMCP 4.0.5 Catalog compile was quiesced with a scoped SIGINT after source
and model changed. It stopped after 2h50 elapsed / 2h36 CPU in callable normalization. Its 204 original receipt records were
captured before the scoped reset retired that unselected staging generation. This
is an interrupted obsolete-source preliminary journey, not a stage failure or final-tree Q0.
A charged, byte-preserving inventory cache and span candidate index now have scoped controls
and static integration acceptance; original native symbolic/summary controls passed six, and
exact PostgreSQL service/installation controls passed 17. No speedup is claimed.

## Prior qualification and standing limits

Foundation F1–F3 are **Accept scoped / Tested** within the
[companion plan §5.4](docs/plans/semantic-model-foundation-enhancements_2026-10-01.md#5-coordination-verification-and-current-state)
source boundary and [assembled review](docs/design_review/reviews/design_review_semantic-foundations-assembled_2026-10-02.md).
Its final `NEXTEST_TEST_THREADS=8 just test-all` passed: workspace 1,000 passed / 12 skipped,
Python 238 passed / 56 skipped, separate PG 376 passed / two skipped, doctests 34 passed /
two ignored; hygiene was composite passed. Subsequent hook formatting and Phase 5 edits need
fresh qualification. The stopped external probe is no longer a known container prerequisite.

Phase 4 remains Implemented / Tested within its
[2026-10-01 receipt](docs/design_review/evidence/2026-10-01_phase4-qualification/README.md).
Phases 0–2 retain their [facts receipt](docs/design_review/evidence/2026-09-30_facts-qualification/README.md),
and Phase 3 [its scoped receipt](docs/design_review/evidence/2026-09-30_phase3-qualification/README.md).
R4's simultaneous two-variant Summary qualification remains **not_run / open**; revisit before
broadening current complete uniquely Bound admission. Retrieval quality, comparisons, preparation
speed, hydration and total RSS remain unmeasured and are not invented Phase 5 completion gates.

Cargo keeps pinned release profiles, stable shared intermediates, 16 jobs and the default single
frontend thread. Nextest8 caps test processes; it does not request eight threads per process.
A verified cross-worktree Cargo artifact lock cycle is avoided by serializing builds, with no
cache cleanup. Earlier compiler SIGTERM remains unexplained, not a proven threading failure.

Next: the current model is `dac961ba`; both CPython ABIs are rebuilt and installed.
The digest-only model snapshot is reviewed/accepted; its original control passed. Run the
complete functional gate, then final-source both-profile FastMCP Catalog journeys, explicit
vector preparation and reviewed activation.
The old staging generation was inventoried, its receipt metadata captured, and retired by the
resumable reset. Local PostgreSQL now uses `max_locks_per_transaction=512` after the operator
restart; reset passed and the final CLI reports zero generations / zero store findings.
The earlier eight functional gate defects have focused repair receipts; full Q0 remains pending.
Main preserves all initial 118 dirty paths against snapshot `bd22f66c`. The automatic hook
completed ADR indexing/formatting; the authorized feature-union refresh is complete. Formatting
and generated-file refreshes remain hook-owned; no manual formatter was run.
