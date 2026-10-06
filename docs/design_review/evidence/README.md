# Evidence

Probes, spikes and investigations that informed a decision, kept so they are not recreated and so
a later reader can see what a decision rested on. Evidence, never authority: the architectural
collection and the ADRs decide (binding §4).

**Layout.** One folder per investigation, `YYYY-MM-DD_<topic>/`, with:
- `README.md`: the question, how it was run (commands, versions, machine), the result, and which
  review, ADR or plan item consumes it;
- the probe's sources (small text, tracked normally);
- `raw/` for raw tool output (Git LFS, with PDFs, archives, Arrow and Parquet; `.gitattributes`).

Never commit virtualenvs, `target/` directories, stores or caches (`.gitignore` here); record how
to rebuild them instead. Keep a probe runnable where that is cheap; otherwise keep its source and
its recorded output.

## Index

A folder stays while a current decision, open finding, test or build consumes it; otherwise it is
removed and recovered from Git (ADR-0042, [historical recovery](../../README.md#historical-recovery)).

| Folder | Question | Current consumer |
|---|---|---|
| [`2026-10-06_graph-native-pivot-audit`](2026-10-06_graph-native-pivot-audit/README.md) | Does cold audit detect changed search data, and can failed selection already publish a serving pin? | [Graph-native implementation audit](../reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md), F04/F05 |
| [`2026-09-24_bdd-pilot-survey`](2026-09-24_bdd-pilot-survey/README.md) | Can the published pilot's conditions convert to BDDs, and at what size? | ADR-0082 (kernel allowance, §3.9, §15.7) |
| [`2026-09-24_test-leaf-proof-joins`](2026-09-24_test-leaf-proof-joins/README.md) | Do test leaves and entry values join to proof rows by source identity? | ADR-0082 (§3.9 test leaves) |
| [`2026-09-24_entry-value-bridge`](2026-09-24_entry-value-bridge/README.md) | When is an entry formal stable up to a guard use? | ADR-0025 (proposed native executor) |
| [`2026-09-24_rust-build-performance`](2026-09-24_rust-build-performance/README.md) | Which development build configuration reuses cached work? | ADR-0079; retained benchmark |
| [`2026-09-24_z3-5-migration`](2026-09-24_z3-5-migration/README.md) | Can the vLLM service's tilelang build against Z3 5? | `services/vllm` (its tilelang wheel is referenced from here) |
| [`2026-09-25_bdd-decision-apis`](2026-09-25_bdd-decision-apis/README.md) | Which biodivine decision APIs decide what the kernel refuses? | Forward plan W6/W11 (reasoning review F01–F03) |
| [`2026-09-25_library-fit-followup-kernel`](2026-09-25_library-fit-followup-kernel/README.md) | Actual-kernel decision refusal, support history, catalog hydration | Forward plan W6 |
| [`2026-09-25_library-fit-followup-flow`](2026-09-25_library-fit-followup-flow/README.md) | Cycle-head reach and qualified/aliased builtin access | Forward plan W4, W7 |
| [`2026-09-25_library-fit-followup-serving`](2026-09-25_library-fit-followup-serving/README.md) | Facet verdicts and claim citations in served projections | Forward plan W2, W3 |
| [`2026-09-28_postgresql-query`](2026-09-28_postgresql-query/README.md) | Do immutable PG publication, complete queries, gated retrieval and admitted federation preserve their contracts? | ADR-0069, PostgreSQL PG12–PG15 and forward-plan §6.1 |
| [`2026-09-28_cargo-cache`](2026-09-28_cargo-cache/README.md) | Do normalized targets, shared intermediates and workspace feature unification preserve build boundaries? | ADR-0079 |
| [`2026-09-29_semantic-data-model`](2026-09-29_semantic-data-model/README.md) | Can one shared semantic model replace per-consumer re-derivation, and which library capabilities fit it at the pins? | [Semantic data model target review](../reviews/design_review_semantic-data-model_2026-09-29.md) |
| [`2026-09-29_library-utilization-semantic-stage`](2026-09-29_library-utilization-semantic-stage/README.md) | Can workspace references be resolved to library items without a build, and does `rust-analyzer scip` or a targeted `ra_ap_*` tool fit? | `docs/library-utilization.md` (stage S2) |
| [`2026-09-29_p0e-subset-envelope`](2026-09-29_p0e-subset-envelope/README.md) | What do the stage-bound capture and syntax stages cost on fastmcp, and does budget exhaustion stay clean? | Plan §4.1.1 E1; input-validation F02 residual (E1 envelope) |
| [`2026-10-03_code-facts-opportunity`](2026-10-03_code-facts-opportunity/README.md) | Which additional pyrefly/ruff/ty code facts should the pipeline implement, and how would downstream layers use them? | [Code-facts opportunity review](../reviews/design_review_code-facts-opportunity_2026-10-03.md) |
| [`2026-10-03_code-facts-expanded-target`](2026-10-03_code-facts-expanded-target/README.md) | Which latest independent Ruff, Pyrefly and ty payloads and first interpretations improve analysis and target answers, with precise limits? | [Expanded analyzer target review](../reviews/design_review_code-facts-expanded-target_2026-10-03.md) |
| [`2026-10-03_code-facts-analytical-architecture`](2026-10-03_code-facts-analytical-architecture/README.md) | How should the expanded facts, intermediate analyses and target outputs compose within the current architecture? | [Analytical architecture review](../reviews/design_review_code-facts-analytical-architecture_2026-10-03.md) |
| [`2026-10-04_library-leverage`](2026-10-04_library-leverage/README.md) | Which bespoke generic mechanisms could more of a pinned library, or a new library, displace? Which library-enabled enhancements have a consumer? | [Library-leverage review](../reviews/design_review_library-leverage_2026-10-04.md) |
| [`2026-10-05_surrealdb-pivot-lanes`](2026-10-05_surrealdb-pivot-lanes/README.md) | What do the store, compute, graph and serving responsibilities consist of, and can SurrealDB 3.3 or Neo4j GDS deliver the outcomes natively? | [SurrealDB graph-store review](../reviews/design_review_surrealdb-graph-store_2026-10-05.md) |
| [`2026-10-05_surrealdb-native-realization`](2026-10-05_surrealdb-native-realization/README.md) | Can a SurrealDB-native snapshot realization meet the generation intent, and do served journeys keep their evidence closure? | [SurrealDB graph-store review](../reviews/design_review_surrealdb-graph-store_2026-10-05.md) |
| [`2026-10-05_snapshot-diff`](2026-10-05_snapshot-diff/README.md) | Do structurally shared snapshots and identity/semantic diffs work across PostgreSQL, SurrealDB, Dolt and TerminusDB, and which validation is incremental? | [SurrealDB graph-store review](../reviews/design_review_surrealdb-graph-store_2026-10-05.md), snapshot-model supporting document |
| [`2026-10-05_graph-analytics-parity`](2026-10-05_graph-analytics-parity/README.md) | Do Neo4j GDS and SurrealDB traversal meet our analytic contracts (determinism, exactness, truncation, edge identity)? | [SurrealDB graph-store review](../reviews/design_review_surrealdb-graph-store_2026-10-05.md) |
