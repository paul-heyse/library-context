# Architecture map

**Interface-checked, 2026-10-02:** these source entrypoints locate current responsibilities.
[DESIGN](DESIGN.md) holds scope, the binding decisions §B1–§B14 and durable deferrals; the
focused owners below hold each responsibility's contracts, accepted targets and known limits.
Evidence labels distinguish accepted targets from implemented behavior. [STATUS](../../STATUS.md)
locates current completion claims. The
[target-alignment coordinator](../plans/target-implementation-alignment-plan_2026-10-04.md)
owns the new review corrections and combined acceptance; the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md) retains product/research dispositions.

| Responsibility | Owner | Source entry / adjacent consumer |
|---|---|---|
| **Accepted target:** the semantic relation model, PostgreSQL single store and layered cutover | [Semantic relation model (§15)](sections/semantic-model.md) | `lctx-model` contracts and `lctx-postgres` generations; `cpg-core` orchestrates cumulative stages. ADR-0085/0083/0084; the [cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) owns execution and qualification; Phase 5 serving is implemented with scoped controls; activation and real-library qualification remain stopped; [detailed plan](../plans/semantic-model-phase5-detailed-plan_2026-10-01.md) |
| API/evidence product target and Context7 differentiation | [Product target (§14)](sections/api-and-evidence-product.md) | Model-owned catalog, source/effective options, original evidence and declaration selection; tool serving is implemented; product comparison and real-library qualification remain open |
| Scope, binding decisions, dependency family, deferrals | [DESIGN §1, §2, §7, §13](DESIGN.md) | Every owner below; ADRs govern its §B sections |
| Fact authority, identity, codebooks, coverage, graph catalog | [Facts and identity (§3–§3.8)](sections/facts-and-identity.md) | [lctx-model](../../crates/lctx-model/src/lib.rs); native extraction produces these contracts, codebooks stay append-only |
| Catalog wire contracts and library adoption | [Query §14.7](sections/api-and-evidence-product.md#section-14-7), [library policy §14.11](sections/api-and-evidence-product.md#section-14-11) | Typed catalog and selection are model-owned; generated wire/native/Python conformance controls challenge current generation serving |
| Pinned acquisition, extraction and fact construction | [Acquisition and extraction (§4)](sections/acquisition-and-extraction.md) | [cpg-extract](../../crates/cpg-extract/src/lib.rs), [lctx](../../crates/lctx/src/main.rs); external analysis environments are explicit inputs |
| Places, conditions, verdicts and the condition kernel | [Behavior model (§3.9)](sections/behavior-model.md) | [cpg-flow](../../crates/cpg-flow/src/lib.rs), `lctx-model::domain::conditions` kernel; ty's declared runtime view joins canonical latest-Ruff occurrences by mapped span and structural role |
| Projections, PostgreSQL publication and generation readers | [Storage and publication (§5–§6)](sections/storage-and-publication.md) | [lctx-postgres](../../crates/lctx-postgres/src/lib.rs) owns persistence/leases; `cpg-core` owns generation-bound DataFusion providers; model invariants gate publication |
| PostgreSQL services and later library capabilities | [Storage §6.5](sections/storage-and-publication.md#section-6-5), [serving §11.4](sections/synthesis-and-serving.md#section-11-4) | [PostgreSQL workstream](../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream); SQLx effect boundary, exact-vector receipts and conditional consumers; service receipts retain their original scope; canonical generation serving is implemented; wider qualification remains stopped |
| Structural analysis, communities, centrality and FCA/RCA | [Analytics (§9–§9.8)](sections/analytics.md) | [lctx-analytics](../../crates/lctx-analytics/src/lib.rs): pure native kernels; `lctx-model` owns settings, contracts and nominal outcomes, no store |
| Models, L2 fates, summaries, proof identity, registry | [Behavioral analysis (§9.9)](sections/behavioral-analysis.md) | `lctx-model::domain::execution` and `transfer`; `cpg-core::semantic_execution` and `semantic_summaries` adapt completed inputs; explicit unknown boundaries constrain claims |
| Findings, assertions, briefs, embeddings, retrieval and tools | [Synthesis and serving (§10–§11)](sections/synthesis-and-serving.md) | Model-owned synthesis/retrieval/embedding with `cpg-core` adapters; generation-bound PostgreSQL services and [lctx_mcp](../../python/lctx_mcp/src/lctx_mcp/server.py) implement serving; current-state reconstruction remains an activation prerequisite |
| Publication rules, independent oracles, evaluation | [Validation and evaluation (§8, §12)](sections/validation-and-evaluation.md) | `lctx-model` shared invariants and actual generation admission, `eval/behavior/`, `tests/scripts/test_flow_soundness.py` |

Dependency direction runs from orchestration (`lctx`) through capability owners to the schema and
semantic contracts in `lctx-model`. Effects (acquisition, persistence, serving) are explicit
boundaries around transformations. This table describes intended ownership; known coupling and
incomplete capabilities follow the target coordinator or forward plan §6, as their source disposition specifies. It is not a certificate of current
architectural conformance.

## Reading and extending

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
[alignment coordinator](../plans/semantic-model-incremental-alignment-plan_2026-10-02.md#7-current-disposition--sole-execution-owner)
owns the current bounded improvements and acceptance, separately from stopped Phase 5 Q0.
