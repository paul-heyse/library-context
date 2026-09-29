# Architecture map

**Interface-checked, 2026-09-25:** these source entrypoints locate current responsibilities.
[DESIGN](DESIGN.md) holds scope, the binding decisions §B1–§B14 and durable deferrals; the
focused owners below hold each responsibility's contracts, accepted targets and known limits.
Evidence labels distinguish accepted targets from implemented behavior. [STATUS](../../STATUS.md)
and the [forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md) own completion claims
and the disposition of known defects.

| Responsibility | Owner | Source entry / adjacent consumer |
|---|---|---|
| API/evidence product target and Context7 differentiation | [Product target (§14)](sections/api-and-evidence-product.md) | Catalog and ordered contracts under ADR-0078; shared Rust wire schemas, isolated derivations and bounded surface/configuration links Implemented under ADR-0073/0074, original contextual evidence under ADR-0076; comparative acceptance open |
| Scope, binding decisions, dependency family, deferrals | [DESIGN §1, §2, §7, §13](DESIGN.md) | Every owner below; ADRs govern its §B sections |
| Fact authority, identity, codebooks, coverage, graph catalog | [Facts and identity (§3–§3.8)](sections/facts-and-identity.md) | [cpg-schema](../../crates/cpg-schema/src/lib.rs); extraction produces these contracts, codebooks stay append-only |
| Catalog wire contracts and library adoption | [Query §14.7](sections/api-and-evidence-product.md#section-14-7), [library policy §14.11](sections/api-and-evidence-product.md#section-14-11) | PR2 Implemented under ADR-0073: shared current requests/packets, nominal IDs, Schemars and offline conformance; PR4 classifier and conditional engines retain named consumers |
| Pinned acquisition, extraction and fact construction | [Acquisition and extraction (§4)](sections/acquisition-and-extraction.md) | [cpg-extract](../../crates/cpg-extract/src/lib.rs), [lctx](../../crates/lctx/src/main.rs); external analysis environments are explicit inputs |
| Places, conditions, verdicts and the condition kernel | [Behavior model (§3.9)](sections/behavior-model.md) | [cpg-flow](../../crates/cpg-flow/src/lib.rs), `cpg-schema` condition kernel; ty's index joins Pyrefly's parse by byte range |
| Projections, Delta publication, readers, serving generations | [Storage and publication (§5–§6)](sections/storage-and-publication.md) | [cpg-core](../../crates/cpg-core/src/lib.rs); shared validators gate publication, snapshots bound serving |
| PostgreSQL services and later library capabilities | [Storage §6.5](sections/storage-and-publication.md#section-6-5), [serving §11.4](sections/synthesis-and-serving.md#section-11-4) | [PostgreSQL workstream](../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream); SQLx effect boundary, exact-vector receipts and conditional consumers; local exact deployment qualified; new catalog work follows the product plan |
| Analytic passes, communities, centrality, FCA/RCA, keep rule | [Analytics (§9–§9.8)](sections/analytics.md) | [lctx-analytics](../../crates/lctx-analytics/src/lib.rs): Arrow in, Arrow out, no store |
| Models, L2 fates, summaries, proof identity, registry | [Behavioral analysis (§9.9)](sections/behavioral-analysis.md) | `lctx-analytics::summaries`, `cpg-core`; explicit unknown boundaries constrain claims |
| Findings, assertions, briefs, embeddings, retrieval and tools | [Synthesis and serving (§10–§11)](sections/synthesis-and-serving.md) | `cpg-core` Stage F, `cpg-schema` projection contracts, `lctx-postgres`/`lctx_storage` effects, [lctx_mcp](../../python/lctx_mcp/src/lctx_mcp/server.py), pure `lctx_semantics` |
| Publication rules, independent oracles, evaluation | [Validation and evaluation (§8, §12)](sections/validation-and-evaluation.md) | `cpg_schema::rules`, `eval/behavior/`, `tests/scripts/test_flow_soundness.py` |

Dependency direction runs from orchestration (`lctx`) through capability owners to the schema and
semantic contracts in `cpg-schema`. Effects (acquisition, persistence, serving) are explicit
boundaries around transformations. This table describes intended ownership; known coupling and
incomplete capabilities are items in the forward plan's §6. It is not a certificate of current
architectural conformance.

## Reading and extending

For a change, identify the semantic owner, its consumer contract and the expected extension axis.
Read that owner plus affected consumers, then the ADR that governs the section
([index](../adr/README.md)); reuse or compose existing capabilities where their semantics fit.
Detailed fields and invariants belong in executable declarations. Update the owner when meaning or
boundaries change, and follow the existing review cadence. Internal refactors need no additional
proof packet. Section IDs never renumber; a moved live section keeps a relocation pointer, and
retired material is recovered from Git ([historical recovery](../README.md#historical-recovery)).
