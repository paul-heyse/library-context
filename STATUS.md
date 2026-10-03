# Status

_Updated 2026-10-02 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Phase 5 implementation is sufficient for now by operator instruction, 2026-10-02.** Further
qualification is stopped; serving activation is not qualified.
The [incremental alignment review](docs/design_review/reviews/design_review_semantic-model-incremental-alignment_2026-10-02.md)
is **Revise / Implemented**, 2026-10-02: bounded corrections, no wholesale redesign.
The [four-plan alignment series](docs/plans/semantic-model-incremental-alignment-plan_2026-10-02.md) now owns F01–F08 and six substantiated opportunities, all Scheduled / Proposed.
Next: authorize execution of its dependency-ordered packages; plan authoring performed no production remediation or renewed qualification.
[Phase 5 plan §10](docs/plans/semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state)
owns package receipts and current assembled-review dispositions. The
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

Phase 5 contracts, lifecycle, services, bridges, CLI, controls and retirement are committed
through `91751d1c`; normalization/qualification repairs as `3591b68c`, hygiene/automatic output
as `38efdd4b` / `8e10d256`, scoped review/documentation as `eeabf906`.
Plan §10 retains individual package commits and receipts. Full Q0 is deferred.

X0 removed 375 unchanged inventoried source files and 21 obsolete query metadata entries,
including the retired schema, bundle, native IPC and old serving authorities. Three obsolete
wire assets were subsequently inventoried and removed. There is no compatibility reader.
Independent expectations now belong to current model, store, CLI and actual transport controls.
Protected gold, heldout inputs, preregistration and benchmark captures remain preserved.

The [assembled review](docs/design_review/reviews/design_review_phase5-assembled_2026-10-02.md)
was Revise at its original inspected boundary. Appendices A/B accept its codec and assertion
corrections; both findings are Closed / scoped Tested in plan §10. Final Appendix C is
Accept scoped at Implemented maturity, with full Q0 still pending. Authored BriefDocument bytes stay
canonical; resources append attributed assertion status/kind/qualification/text/support metadata.

| Current Phase 5 command / boundary, 2026-10-02 | Outcome |
|---|---|
| Normalized `cargo nextest run --release -p lctx-model --test serving_contracts --test ranking_policy` | **composite passed**, 32 / zero skipped; missing test dev dependency repaired |
| Original `cargo nextest run --release -p lctx --test serving_native`, before latest ordering fix | **composite passed**, 1 / zero skipped, 177.59s; both profiles each passed ten actual transport controls |
| Original `cargo nextest run --release -p lctx --test serving_vectors` | **composite passed**, 1 / zero skipped, 120.119s; controlled numerical HTTP seam |
| Focused compile-facts/packets/retrieval/evidence/soundness command | **failed**, nine passed, one failed, one timed out; original receipt retained |
| Corrected `cargo nextest run --release -p lctx --test serving_evidence` | **passed**, 1 / zero skipped, 92.778s; actual BaseEvaluation parent, original bytes and damaged-view controls |
| CPython served-path oracle with scoped 900s harness | **failed**, 833.640s; native preparation exposed receipt/validator row-order conflict |
| Current ordering-discriminator native fixture | **passed**, 1 / zero skipped, 200.266s; both profiles and all original transport/tamper controls |
| Original generated CPython oracle after ordering repair | **passed**, 1 / zero skipped, 795.987s; unchanged original assertions |
| First Phase 5 `NEXTEST_TEST_THREADS=8 just test-all` | **failed at compile**, duplicate service-test import; no workspace test result |
| Corrected `NEXTEST_TEST_THREADS=8 just test-all` | **failed**, Rust 916 passed / eight failed / two skipped; downstream Python, separate PG and doctests not_run |
| Final `NEXTEST_TEST_THREADS=8 PYO3_PYTHON=.venv/bin/python just test-all` | **interrupted at operator request**; workspace 927 passed / two skipped; Python 228 passed; separate PG 349 passed / eight SIGINT / two skipped / 27 not_run; doctests not_run |
| Current model/native/wire repair controls | **passed**, 51 selected; signed-cursor and final cache envelope covered |
| CLI exact attempt-ID parser | **passed**, original one selected control |
| Phase 5 `just hygiene` | **composite passed**, all current source/policy checks including workspace/all-target Clippy; live store reset and final zero-finding CLI check passed |
| Fresh pinned FastMCP Catalog compile | **interrupted at operator request**, exit 130; unselected staging generation and receipts retained |
| Behavioral compile, vector preparation, real-library MCP journeys and activation | **not_run**, operator stopped qualification |
| Live query embedding | **blocked**, configured endpoint absent; controlled seams establish no live quality |

The focused nine passes include all packet controls, direct mixed-status canonical comparison,
actual eligibility and ranked-winner original bytes, original guard loss/cancellation and the
cumulative 30-second deadline. The evidence fixture originally asked for Local AnalysisInput
parents although Local declares none; the corrected actual BaseEvaluation witness retains all
canonical receipt, damaged-view and captured-byte assertions. Plan §10 owns commands and logs.

The CPython oracle retains nine grouped programs / 52 functions and independent observations.
Its published retry failed preparation: receipt content requires increasing IDs while a
shared validator requires call/ordinal groups.
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
A verified cross-worktree Cargo artifact lock cycle is avoided by serializing builds; no cache cleanup.
Earlier compiler SIGTERM remains unexplained, not a proven threading failure.

No further qualification is running. Model `dac961ba` and both rebuilt CPython ABIs are current; the accepted digest snapshot control passed.
Final workspace run `768e3cf4` passed 927; separate PG run `1fce640e` stopped by scoped SIGINT,
exit 100 (`/tmp/lctx-phase5-test-all-final_2026-10-02.log`). Its eight signal exits are user-requested
cancellation, not eight newly diagnosed code defects. No complete `just test-all` pass is claimed.
FastMCP staging generation `de1e0c2b09a319134fcef595a9bcbe89` remains unselected, with 204 receipts;
callable-aspect normalization stopped before output. Its compiler/backend/locks are gone.
Interruption receipt: `/tmp/lctx-phase5-q0-final-interruption_2026-10-03.json` (UTC filename).
Resume only on operator request; plan §10 retains the omitted Q0 scope and activation boundary.
The old staging generation was inventoried, receipts captured, and retired by the resumable reset.
Local PostgreSQL uses `max_locks_per_transaction=512` after operator restart; reset and its zero-finding CLI check passed.
The earlier eight functional gate defects passed the final workspace run; full Q0 is deferred.
Plan authoring preserves the 117 initial dirty tracked paths at `42551010`; document-only verification is recorded in the alignment coordinator.
ADR indexing/formatting and the authorized feature-union refresh completed; generators remain hook-owned.
