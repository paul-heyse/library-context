# Status

_Updated 2026-10-09 under the [handoff skill](.claude/skills/handoff/SKILL.md); shared main._

**Current scope: graph compilation and hashing design review; earlier implementation paused.**
The [principal review](docs/design_review/reviews/design_review_graph-compilation-kernels-and-hashing_2026-10-09.md)
concludes **Revise scoped relation-operation composition**, at static/source-inspected strength.
It recommends **Proposed** model-owned scope operations and shared prepared selection, with
graph/native/relational lowerings and qualified hashing or incremental reuse where their contracts fit.
The review owns its new unscheduled findings and unapplied rule impacts; no remediation or rule
change is implemented. Product tests, probes and benchmarks are **not_run** for this review.
**passed, 2026-10-09:** `just docs-check` — 350 canonical pages, zero link errors.

**Previous implementation checkpoint: populated correction implemented; targeted acceptance recorded.**
The [populated companion](docs/plans/populated-journey-execution-amplification-plan_2026-10-09.md)
defines PJ0–PJ3, materially triggered PJ4 and targeted PJ5. The integrated correction is committed locally on main as
`2a62e817`; no push is requested. The
[persisted coordinator §8/§9.1](docs/plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07)
owns finding disposition, exact current commands, failed/repaired receipts and remaining acceptance.

**Implemented / source-inspected, 2026-10-09:** ADR-0138 consolidates the native execution,
supplier, completion and publication contracts. Fixed native envelopes and supplied scopes replace
vocabulary-wide body/scope programs. Declaration-selected synchronous adapters validate bounded
windows before effects; cold reconstruction compares complete actual bodies, envelopes and originals.
The reviewed fixed-envelope snapshot is a **physical schema migration**. Completed-state format2,
artifact format3, semantic digest domains and append-only codebooks are unchanged.

Final admission fences the full Workspace/native completion-binding-visibility handoff, drains
content mutations and checks actual descriptors before attestation and semantic admission.
Complete alias-aware pointer runs are prepared once; independent charged cursors retain scratch
through worker terminality and the last consumer. Ordinary publication seals that admitted native
owner. Detached artifacts and SQL staging restore remain independently admitted. Publisher's
private workflow accepts only store/manifest-bound admission owners and retains its own effect
session. Compiler admin clients and public seal/marker routes are removed. Live finalization
parents cover post-closure reads; acknowledged committed identity survives cancellation/abandonment.

Coarse phases use captured dispatch/span attribution across tasks, provider/fixture threads and
borrowed CPU regions. Result owners emit terminals; drop leaves interruption incomplete.
A composed core binary suppressed native macro callsites despite capturing ordinary scoped INFO;
explicit fixed-metadata emission repairs that observed symptom and preserves filtering. Its
underlying library/compiler cause remains unproven. Facts, invariant and frontier admission are
separately attributable. No per-row telemetry, worker caps or measured speed claim is introduced.

The [independent assembled review](docs/design_review/reviews/design_review_assembled-populated-execution-correction_2026-10-09.md)
and bounded diagnostics reassessment conclude **Accept scoped at static / Implemented strength**.
Current focused and populated acceptance is recorded below; enclosing qualification remains open.

**passed, 2026-10-09:**

- Final formatted-source native refresh, locked release CLI/evaluator build, 15 pure controls
  and four source-fingerprint controls (`20261009T070023.218Z-35efa2`); `just ready`.
- Current affected crate/test compile checks; final full keep-going workspace/all-target Clippy
  (`20261009T055818.261Z-c08c03`) after applicable repairs.
- Twelve pure codec/schema/adapter/reconstruction controls and reviewed snapshot acceptance.
- Fifty-two controls in the 54-control native selection; its two failed backing expectations
  were corrected without weakening checks, and their exact rerun passed
  (`20261009T054808.562Z-cbcc63`), including independent database absence.
- Actual core admission failure phase control (`20261009T055352.008Z-869ea3`); four current
  native auth-refusal/filter/concurrent-attribution/abort controls (`20261009T055723.301Z-c3525e`).
- Single Catalog Facts compile/admit/export, 138.296s (`20261009T054911.783Z-475010`);
  intact/tampered known-export transport, 247.378s (`20261009T062646.015Z-b2b5ea`).
- Two populated native journeys (1741.50s bodies), publication/tools/backup/fresh restore and
  35 Python/MCP/native-evaluator controls (27.378s), `20261009T060142.629Z-c9c159`.
- CLI Catalog Facts/Normalized and Behavioral Facts; foreign captured cold admission, within
  the otherwise failed composite `20261009T052757.430Z-f56701`.
- Applicable lint-agents/ADR/fixtures/gold/rules/Ruff/types/dependency leaves and corrected
  rules-scan. Final dependency replay passed (`20261009T062616.617Z-e74670`) after recording
  the external shared fixture dependency cargo-shear cannot discover. Hakari is regenerated;
  no dependency version moved.
- `just docs-check`: 349 canonical pages, zero link errors; ADR/Hakari freshness clean.

**Remaining failed boundaries:** initial core selections had the repaired phase-control failure
and 19 combined 300s timeouts. Compiled-export publication also timed out at 300s. A split-test
selector missed qualified names; its correction preserves the original 900s orchestration allowance.
The seven-case frontier run finished: Behavioral Facts passed in 488.727s; six Normalized,
Analysis or Catalog cases timed out at their unchanged 900s deadline
(`20261009T055800.496Z-d3005d`). Retained phases place these observed long intervals in
normalization, before final admission: Catalog callable aspects took 342.592s, while Behavioral
upper cases had begun entity normalization. No universal freeze/read wait cycle is supported;
the specific cause of the time between small CPU regions remains unproven. No failed composite
is relabelled as a clean pass.

**Next:** select the review's target and create its implementation plan, resolving operation ownership,
shared demand and reuse contracts before choosing lowerings or cache libraries. The existing
operator-deferred normalization/semantic-validation followup and enclosing
qualification retain their existing owners; scoped maintenance and final preparation passed. The one populated
`serving:mcp` run passed after a guard-free native refresh; its runtime evidence is distinct from
build/wait time and does not establish a measured speedup. It precedes mechanical scoped
formatting; final preparation and 19 pure/source-identity controls cover the formatted source. The coordinator retains the source
boundaries and remaining large detached-publication/upper-frontier timeouts. Normalization and
semantic-validation repeated-access avenues are recorded in the companion for the operator-deferred
followup; static review identified no bounded PJ2/PJ3 correctness regression or freeze/read cycle.

**Enclosing boundaries remain open:** PC6/CU6/BC3/BC5, the operator-deferred timeout followup
and assembled release `qualify`. Active defaults remain release until BC3 qualifies its candidate.
Earlier successful populated acceptance (31m23s native bodies) describes the preceding source,
not this physical schema. The coroutine/compilation companions retain their compiler evidence
and source limits. Current focused passes do not establish whole-plan or performance qualification.

**Preservation:** retain PC3's dirty isolated planner-control copy, shared intermediates and
profiling/benchmark captures. No operator store/configuration, client registration, Qwen service,
protected gold/heldout, real FastMCP/Q1 pilot, wheel, benchmark campaign, activation or push is requested.
Earlier product/evidence/graph-native obligations remain with the plans linked by the coordinator.
Workspace commands and end-of-turn maintenance follow [AGENTS.md](AGENTS.md#commands).
