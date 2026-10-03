# Status

_Updated 2026-10-03 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Incremental alignment is complete within scoped Tested acceptance.** The authorized
[four-plan alignment series](docs/plans/semantic-model-incremental-alignment-plan_2026-10-02.md)
owns current execution; coordinator §7 is the sole mutable F01–F08/O1–O6 disposition. All N/E/D
and T0/T1/T2 packages are integrated, with current complete functional and composite hygiene receipts.

**Phase 5 qualification is stopped at the operator’s code-facts review pivot, 2026-10-03.**
The operator selected both profiles with communities, PageRank, FCA/RCA and type/mention layers;
kNN and embeddings are off. Live embedding/vector qualification is deliberately deferred pending
analytical-output review; client registrations remain unchanged. [Phase 5 §10](docs/plans/semantic-model-phase5-detailed-plan_2026-10-01.md#10-finding-routes-limits-and-current-state)
retains prior receipts/limits; [parent cutover §8](docs/plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition)
owns older findings. PR6 remains paused until serving is qualified. R4 simultaneous multi-variant
Summary qualification is not_run; retrieval quality, speed and total RSS remain unmeasured.

The model remains the single semantic owner, PostgreSQL the canonical relation store and
DataFusion in-process compute. Pure native preparation/formal resolution now live in the model;
storage retains receipt/membership/guard checks and resource admission. Exact dependency
requirements, finite upper-stage bindings, one fixed analytic policy and typed packet bindings
replace duplicate authorities. Catalog Flow stays NotRequested, default stability remains
Unknown, and finite-association/five-verdict limits survive. Python owns transport effects.

Analyzer-skill checkpoint, 2026-10-03: the shared `pyrefly-ruff` and `ty-flow` skills merged into
`python-analyzers`. It indexes the planned shift targets (pyrefly 1.4.0-dev.3 on ruff 0.0.14; ruff
0.16.10 / crates 0.0.16; ty 0.0.16, salsa 0.28.5) plus a code-facts model, and its `show migration`
now also covers `cpg-flow`'s ty move. The five-plan migration is now in progress; current package receipts are in the coordinator. The skill's
`build/verify.py --repo` (11 checks), 75/75 probes, `just docs-check` and `just lint-agents`
passed on 2026-10-03. Those receipts are separate from alignment acceptance.

## Current implementation and acceptance

Execution began at `ed2d68d7`, preserving unrelated library-utilization edits and concurrent
operator guidance `628b018d`. T1/ADR-0115 is `d84a2287`; T0 gate composition is `761687bd`;
N/E/D, architectural owners, ADR-0116 and independent review are `e3e7c78d`. Adapter freshness
is `2a03adc8`; final transport/deadline control corrections are `50f58ffa`. The final documentation commit records receipt/handoff
publication. Concurrent role/catalog edits remain outside this work's commits.

The [independent implementation review](docs/design_review/reviews/design_review_semantic-model-alignment-implementation_2026-10-02.md),
including Appendix C dated 2026-10-03, accepts the bounded design at **Implemented** strength.
Actual runtime acceptance and failures are recorded separately in coordinator §8.

| Last verified command / boundary, 2026-10-03 | Outcome |
|---|---|
| Normalized release model/core/real-store focused Nextest | **passed**, 109, zero skipped; invariant/epoch, visible Catalog, checkpoint and CPU drain controls |
| Python wire/schema and build-environment controls | **passed**, nine each |
| Locked uncached native refresh, then `just native-adapter-ready` | **passed**; current CLI/installed storage match all 240 model-source captures |
| First `NEXTEST_TEST_THREADS=8 just test-all` | **failed**, 942 passed, two failed, two optional measurement controls skipped; control defects repaired |
| Final `NEXTEST_TEST_THREADS=8 just test-all` | **passed**, exit0; 944 Rust, 230 Python/oracle and 32 doctests (20 compile-fail); two optional measurements skipped |
| `just hygiene` plus named repair checks | **composite passed**; all constituents including full workspace/all-target `just clippy` passed |
| `just docs-check` | **passed** after final disposition/handoff publication; 269 canonical pages, zero offline link errors |
| Confirmed `lctx store reset`, then `just store-check` | **passed**, current empty model, zero generations/findings |
| Current lexical-only Q0 reconstruction/activation | **interrupted** at operator pivot, exit130; unselected staging preserved; behavioral/journeys/gates/activation not_run |
| Live embedding/vector qualification, R4 and measurements | **not_run**, deliberately deferred/separate qualification boundaries |

Persistent command logs/exit receipts are under
`/home/paul/.cache/lctx-alignment-execution/receipts-2026-10-03/`.
The model digest is `12242a5558134418fc3b54fdb526628d1715720e60e6199a8a6079f6066582da`.
The reviewed model-describe snapshot is a schema migration for digest/S0 input ordering;
relation fields and existing codes are unchanged. Declared uv inputs and per-member
content/membership fingerprints prevent stale native adapters; readiness precedes Nextest.
The first full gate's defects were resource routing metadata and an admission-relative timer
assertion; repairs alter test controls, preserving production deadline/transport contracts.

## Retirement, limits and next action

No readers were connected before retiring the old unselected staging generation
`de1e0c2b09a319134fcef595a9bcbe89`. No serving vectors/artifacts existed. The preserved
`retirement-controls.json` contains all 14 control tables and 204 stage receipts; SHA-256
`49dc3c2e4863df869151194544edfe3caf8a457f2e1b4fa3c5ae639ad00d0fa3`.
Reset installed the current model and named store-check passed. Reconstruction from pinned
inputs remains an activation prerequisite; no old runtime authority or compatibility reader remains.

The [expanded analyzer target review](docs/design_review/reviews/design_review_code-facts-expanded-target_2026-10-03.md)
is complete at **Interface-checked** strength, 2026-10-03: finite coverage is 340 provider facts /
137 concepts, with twelve fact/first-operation contracts and downstream output implications.
Inventory consistency and `just docs-check` **passed** (273 pages, zero offline link errors);
provider seams/remedies remain Proposed; runtime probes and product gates were not_run.
Decision **Revise** retains the model/store architecture. The
[five-plan code-facts series](docs/plans/code-facts-expansion-plan_2026-10-03.md) now schedules the
full C1–C12 cascade and is the sole current disposition owner for source-qualified original/new
findings. Execution is in progress under the [execute-plan skill](.claude/skills/execute-plan/SKILL.md).
N0/P0 source-formal/native-slot correspondence and P2a pre-admission facet refusal are committed
(`107e1933`, `e58575a3`): 32 model controls and two real-PG analytic journeys passed on 2026-10-03.
The operator-directed independent Ruff policy is accepted by ADR-0117/0118; both narrow forks
are published at exact immutable revisions. Native seam/parity controls passed in isolation.
The working tree resolves Pyrefly 1.4.0-dev.3, Ruff 0.16.10/crates 0.0.16 and ty 0.0.16/salsa
0.28.5, retaining embedded Ruff 0.0.14. Release extraction/flow all-target compile checks and
19 source-family/fork policy controls passed; current-tree native runtime tests are running.
N4a per-alias/root module resolution passed 43 focused controls in its executor worktree,
including real PostgreSQL; root integration is pending. B0 required canonical assumption sets
and composition are being implemented in an isolated executor worktree. Canonical latest-Ruff
syntax adoption and remaining characterization/behavior/product packages are open.
Coordinator §7 owns dated package receipts; §5 owns all source finding disposition.
Full test-all/hygiene and assembled Q0 are not_run until all functional packages are integrated.
Real-library reconstruction/activation remain a separately authorized follow-up. The prior
278-page authoring docs check and finite inventory verifier receipts establish traceability,
not semantics. No current full-series acceptance, retrieval-quality or performance claim is made.
Prior qualification work remains uncommitted: actual stdio runner and source-default packet repair.
Focused controls passed: real-PG both-profile runner (one), transcript (two), source-default packet
(one); full gates/hygiene and real-library journeys are not_run for these changes. Phase 5 §10 owns
receipts and remaining integration. The catalog compile stopped with SIGINT/exit130; staging
`d3a3fa026a1237a1c4e175b9b1019eeb` has 204 receipts, no readers/writer, and remains unselected.
`/home/paul/.cache/lctx-phase5-qualification/2026-10-03/operator-pivot-interruption.json`
preserves the stop state. No alignment implementation remains open; PR6 remains separate.
Actual transport failure seams exercise the service/original grant and complete response bytes;
they do not qualify physical backend faults or universal early errors without an admitted request.
Borrowed CPU kernels drain; hard cancellation and performance improvements are not claimed.

Prior Phase 5 tests retain their dated incomplete boundary: workspace 927 and Python 228 passed
before the operator stopped the gate; separate PG was stopped and doctests not_run. Current
alignment acceptance does not silently extend that receipt to real-library serving. Foundation
and Phase 4 receipts remain at their owners. Gold, heldout data and benchmark captures are untouched.

Cargo retains pinned release settings, stable shared intermediates, jobs16 and default frontend1.
Nextest8 limits test processes. Independent builds may overlap under Cargo's artifact locking.
The automatic end-of-turn hook owns formatting and generated indexes; no cache cleanup is needed.
