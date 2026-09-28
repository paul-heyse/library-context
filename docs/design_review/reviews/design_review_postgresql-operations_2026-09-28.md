# PostgreSQL recovery and physical admission review

Date: 2026-09-28. Tier **design**, purpose **target**, assembled PG16–PG17 scope and adjacent
PG12–PG15 consumers. Core standard 3.0, code-intelligence profile 1.1 and repository binding apply.
[ADR-0070](../../adr/0070-postgresql-recovery-and-index-admission.md) governs the accepted changes.
[Forward-plan §6.1](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
owns current finding status. [Evidence](../evidence/2026-09-28_postgresql-operations/README.md)
separates focused tests, measurement and deployment acceptance.

## 1. Scope and claim

**Implemented, Tested and Measured for the local exact-serving scope.** The review reconstructs policy/admission, database/artifact
recovery, diagnostics and serving lifetime boundaries. A fresh independent subagent inspected the
source and performed read-only PostgreSQL probes; the implementation owner checked the affected
consumers and corrections. Live ANN quality remains a measured admission decision, not a source
review assertion. Stage 3 semantic completeness, retention pruning and remote HA are excluded.

## 2. Responsibilities

Canonical facts/vectors and projection identity remain cpg-schema/cpg-core/Delta-owned. Rust
lctx-postgres owns SQLx effects, immutable policy validation, query execution and admission checks.
PostgreSQL migration functions own finite privileged transitions. Python keeps lexical/fusion and
MCP orchestration; backup orchestration invokes PostgreSQL tools and existing Rust manifest/artifact
validators. The JSON recovery receipt is checked against restored rows, never a second authority.

## 3. Contracts and dependencies

Exact policy bytes remain stable. Mixed policy declares count/selectivity/class routing independently
of a physical realization. Realization binds cluster/database, engine/server/extension and actual
index topology. Selection, pin and ANN execution consume admission. Ready-generation identity is
unaffected by new indexes, attempts, selection or artifact relocation. Recovery distinguishes durable
history from canonical projection reconstruction and explicitly selects exact after logical restore.

## 4. Expected changes

| Change | Owner and propagation | Challenge |
|---|---|---|
| Restore to another cluster/artifact root | Backup orchestrator → stored manifest closure → Rust relocation/reconcile → selection | Full row equality before mutation; every ready generation loads and captured selection survives |
| Reindex or replace an index | PG realization → admission lookup → selected/new/existing reader | Stale ANN refuses; exact stays available; new confirmation is required |
| Larger eligible operation sets | Existing mixed policy/qualifier → declared count classes | Serving-role measured routing/plan/recall/latency; no global automatic ANN switch |
| Missing artifact or incompatible schema | Diagnostic entry point and pin validator | Report availability separately; observation does not become an optional startup prerequisite |

## 5. Alternatives and library fit

Reuse SQLx pools/transactions, PostgreSQL exported snapshots/catalog/advisory locks, pg_dump/restore
and pgvector indexes. A separate Python driver, ORM, telemetry service or backup catalog would add
owners without a selected consumer. Exact-only is the simplest valid deployment when mixed ANN
cannot meet its benefit threshold. Reusing logical qualification after restore is simpler but does
not describe the rebuilt index, so physical admission is necessary.

## 6. Architectural judgments

**A1 satisfied:** semantic generation, policy, physical state, publication and selection are distinct.
**A2 satisfied:** recovery consumes shared contracts and verifies derived receipts against restored
truth; qualification uses production role/planner behavior. **A3 satisfied:** finite functions and
existing processes compose without a new generic provider/recovery framework. FP-01–FP-06 hold for
the examined changes. DP-18 motivates lightweight startup logging instead of optional metric queries.

## 7. Gates and domain fidelity

G1–G6 hold at source/focused-test strength: explicit claims, authority, types, absence/refusal,
recovery/lifecycle and failure boundaries. G7 is supported by actual serving-role workload/ANN measurements for exact deployment and explicit
ANN rejection;
G8 holds through reuse of PostgreSQL, SQLx and pgvector capabilities; workload/cost evidence belongs to G7, with exact-only support where ANN fails its gate. CI-G1–CI-G3
remain unchanged: PostgreSQL carries attributed facts and complete evidence; ranked discovery never
becomes proof and bounded semantic analysis remains unknown. No dimension-quality revalidation.

## 8. Findings

These findings describe defects found during implementation review; status is only in forward §6.1.

| ID | Finding / consequence | Owner and correction | Closure evidence |
|---|---|---|---|
| <a id="F01"></a> F01 | Recovery selected the last retained generation instead of the captured pointer (G3/G5) | Recovery loop uses receipt selections, changing only profile to explicit exact | Two-generation non-last-selection restore; probe default pins |
| <a id="F02"></a> F02 | Partial receipt inventory could report incomplete recovery as success (A2/G5) | Compare ready manifests, artifact closure and selections with restored rows before mutation | Omitted-generation negative control and populated restore |
| <a id="F03"></a> F03 | Timezone changed equal-row fingerprints (G3/G5) | Canonical session serialization settings in both readers | Non-UTC source → UTC disposable restore |
| <a id="F04"></a> F04 | Qualification omitted routing/admission cost and briefly used importer RLS (G7) | Serving-role measurement, same mixed rank work, atomic writer realization recheck | Read-only RLS plan probe; calibration receipts; confirmation is not_run because no class passed calibration |
| <a id="F05"></a> F05 | Diagnostic permissions/identity drift could misreport state or block startup (G3/G5) | Finite ready-only counts, shared policy checks, descriptor-only startup log | Least-privilege diagnostics and actual MCP lifespan |
| <a id="F06"></a> F06 | Format2 ignored foreign eligible IDs (G3) | Check membership against pinned consumer before vector population | Bounded validator and real pack controls |
| <a id="F07"></a> F07 | RTO omitted verification and printed premature success (G7) | Timer starts before verification; deadline checked before success | Full restore-through-serving duration |

<a id="F08"></a>
**F08 — independent numerical comparison (G3/G7).** The first live report required full
float64 ordinal equality despite the selected PostgreSQL cosine policy. At signature_doc ranks
382/383 one unequal-score pair crosses; neither calculation has a tie. The corrected comparison
revision 2 validates complete entity/view inventories, each calculation's score/ID ordering,
contiguous PostgreSQL ranks and per-entity score tolerance. It explicitly reports displaced ranks,
bounded examples, maximum error and vector/fused top-10 equality. Production ranking/profile
identity is unchanged. The original failed ordinal assertion and frozen input are preserved;
fuzzy ties are not introduced. Regression controls cover crossings, malformed ordering, true ties,
nonfinite scores and excessive error. Current disposition lives in forward §6.1.

## 9. Cost and complexity

The added machinery has concrete consumers: admission prevents stale-index use; artifact receipts
make dumps sufficient to restart native serving; diagnostics distinguish unavailable from unpublished.
The measured hydration fixture is repeated transport volume, not a larger-corpus scalability claim.
Native/lexical state remains resident and is measured; no claim of automatic fourfold speedup.

## 10. Verification boundary

The evidence page records actual commands and dates. Lifecycle control may seed an admission as
synthetic fixture measurements solely to test the finite SQL transition and invalidation; it cannot establish ANN recall/latency. The
initial larger restore failed in a subprocess; the repeated full drill passed and TCP readiness was
made explicit. The separate assembled receipts now include the full code gate, fresh fake/live pilots,
operator migration008, exact selection, two-server rollover and a 15.31-second populated restore
through both retained generations. Later comparator and membership changes have narrow regression
receipts; full-gate counts remain attributed to the earlier gate.

## 11. Authority and disposition

ADR-0070 replaces 0069 while preserving its context/publication/query contracts. Storage/serving
owners and the detailed PG plan carry the accepted change. The forward plan keeps Stage 3 sequencing
and stable source finding IDs. Superseded rationale is recoverable through Git.

## 12. Decision

**Accept scoped, Implemented, Tested and Measured for local exact serving.** No remaining source-
level blocker was found in the corrected boundaries. The separately recorded operator cutover and
post-cutover populated recovery passed. Conditional mixed admission is implemented; the actual
calibration selected no ANN candidate because natural plans and latency benefit failed. Confirmation
is **not_run: no admissible candidate**, rather than a missing exact-deployment prerequisite.
Remote topology and automated pruning retain explicit future consumer triggers; this review does
not qualify larger-corpus scalability or Stage 3 semantic completeness.
