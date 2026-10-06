# Architecture map

**Interface-checked, 2026-10-02:** these source entrypoints locate current responsibilities.
[DESIGN](DESIGN.md) holds scope, the binding decisions §B1–§B14 and durable deferrals; the
focused owners below hold each responsibility's contracts, accepted targets and known limits.
Evidence labels distinguish accepted targets from implemented behavior. [STATUS](../../STATUS.md)
locates current completion claims. The
[graph-native replacement coordinator](../plans/graph-native-pivot-plan_2026-10-05.md)
owns the accepted hard-pivot series grounded in the graph-native target and SurrealDB capability
reviews. Native querying is selected by ADR-0128; execution-fit principles guide efficient
design rather than introducing accounting proofs or adoption gates. The compiler stage is user-accepted complete; native publication, projections and serving are implemented.
The coordinator's §9/R-Q0 records current remediation acceptance and the separate operator activation boundary.
Earlier PostgreSQL evidence is baseline history, not replacement acceptance.
The [target-alignment coordinator](../plans/target-implementation-alignment-plan_2026-10-04.md)
retains the implemented baseline's dated correction/qualification receipt and original source IDs; the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md) retains product/research dispositions.

| Responsibility | Owner | Source entry / adjacent consumer |
|---|---|---|
| **Implemented:** semantic graph, store-free compiler and native SurrealDB realization | [Semantic graph model (§15)](sections/semantic-model.md) | `lctx-model` graph contracts and `cpg-core` store-free compilation; ADR-0128; the [coordinator](../plans/graph-native-pivot-plan_2026-10-05.md) owns scoped functional evidence; operator activation remains pending |
| API/evidence product target and Context7 differentiation | [Product target (§14)](sections/api-and-evidence-product.md) | Model-owned catalog, source/effective options, original evidence and declaration selection; native tool serving is implemented; product comparison and real-library qualification remain open |
| Scope, binding decisions, dependency family, deferrals | [DESIGN §1, §2, §7, §13](DESIGN.md) | Every owner below; ADRs govern its §B sections |
| Fact authority, identity, codebooks, coverage, graph catalog | [Facts and identity (§3–§3.8)](sections/facts-and-identity.md) | [lctx-model](../../crates/lctx-model/src/lib.rs); native extraction produces these contracts, codebooks stay append-only |
| Catalog wire contracts and library adoption | [Query §14.7](sections/api-and-evidence-product.md#section-14-7), [library policy §14.11](sections/api-and-evidence-product.md#section-14-11) | Typed catalog and selection are model-owned; generated wire/native/Python conformance controls challenge the retained pure wire contracts; native serving is implemented |
| Pinned acquisition, extraction and fact construction | [Acquisition and extraction (§4)](sections/acquisition-and-extraction.md) | [cpg-extract](../../crates/cpg-extract/src/lib.rs), [lctx](../../crates/lctx/src/main.rs); external analysis environments are explicit inputs |
| Places, conditions, verdicts and the condition kernel | [Behavior model (§3.9)](sections/behavior-model.md) | [cpg-flow](../../crates/cpg-flow/src/lib.rs), `lctx-model::domain::conditions` kernel; ty's declared runtime view joins canonical latest-Ruff occurrences by mapped span and structural role |
| Projections, admitted artifacts and native publication | [Storage and publication (§5–§6)](sections/storage-and-publication.md) | `cpg-core` owns store-free compilation/admitted artifacts; `lctx-publisher` and `lctx-surrealdb` implement native publication |
| Native services and later library capabilities | [Storage §6.5](sections/storage-and-publication.md#section-6-5), [serving §11.4](sections/synthesis-and-serving.md#section-11-4) | [native realization plan](../plans/graph-native-surrealdb-realization-plan_2026-10-05.md); exact-vector consumption is compiler-owned, `lctx-surrealdb` hosts cache and native publication |
| Structural analysis, communities, centrality and FCA/RCA | [Analytics (§9–§9.8)](sections/analytics.md) | [lctx-analytics](../../crates/lctx-analytics/src/lib.rs): pure native kernels; `lctx-model` owns settings, contracts and nominal outcomes, no store |
| Models, L2 fates, summaries, proof identity, registry | [Behavioral analysis (§9.9)](sections/behavioral-analysis.md) | `lctx-model::domain::execution` and `transfer`; `cpg-core::semantic_execution` and `semantic_summaries` adapt completed inputs; explicit unknown boundaries constrain claims |
| Findings, assertions, briefs, embeddings, retrieval and tools | [Synthesis and serving (§10–§11)](sections/synthesis-and-serving.md) | Model-owned synthesis/retrieval/embedding with `cpg-core` adapters; native serving is implemented; [lctx_mcp](../../python/lctx_mcp/src/lctx_mcp/wire.py) serves the fixed native snapshot through the PyO3 bridge |
| Publication rules, independent oracles, evaluation | [Validation and evaluation (§8, §12)](sections/validation-and-evaluation.md) | `lctx-model` shared invariants and admitted graph content, `eval/behavior/`, `tests/scripts/test_flow_soundness.py` |

Dependency direction runs from orchestration (`lctx`) through capability owners to the schema and
semantic contracts in `lctx-model`. Effects (acquisition, persistence, serving) are explicit
boundaries around transformations. This table describes intended ownership; known coupling and
incomplete capabilities follow graph-native coordinator §7/§9 or forward plan §6, as their source disposition specifies. It is not a certificate of current
architectural conformance.

## Reading and extending

Use the [design principles](../design_review/design_principles/core/design-principles.md) with
the [Heuristics for Efficient Architecture](../design_review/design_principles/core/efficient-architecture-heuristics.md)
when choosing the physical realization. Consider relevant patterns before the mechanism becomes
costly to replace; this does not require reopening settled reviews for ordinary implementation.

For a change, identify the semantic owner, its consumer contract and the expected extension axis.
Read that owner plus affected consumers, then the ADR that governs the section
([index](../adr/README.md)); reuse or compose existing capabilities where their semantics fit.
Detailed fields and invariants belong in executable declarations. Update the owner when meaning or
boundaries change, and follow the existing review cadence. Internal refactors need no additional
proof packet. Section IDs never renumber; a moved live section keeps a relocation pointer, and
retired material is recovered from Git ([historical recovery](../README.md#historical-recovery)).

Existing extension routes (Implemented guidance, 2026-10-02) stay with their owners: new fact
families follow [§4.3](sections/acquisition-and-extraction.md#section-4-3); finite models follow
[§9.9](sections/behavioral-analysis.md#section-9-9); selection predicates and packet routes follow
[§15.12](sections/semantic-model.md#section-15-12). The
[graph-native coordinator §9](../plans/graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution)
owns current bounded improvements and acceptance. The incremental-alignment plan retains its dated
baseline receipts; product investigations and separately authorized Q1 work stay with the forward plan.
