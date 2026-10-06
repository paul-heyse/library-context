# Graph-native remediation capability investigation

**Interface-checked / Proposed designs, 2026-10-06.** Current consumer is the
[graph-native coordinator §9](../../../plans/graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution)
and its four supporting plans. The [implementation audit](../../reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md)
owns defects and dated evidence; coordinator §7 alone owns current finding disposition.
This investigation designs their correction and useful foundation improvements. It is not another
store-selection review, an implementation receipt or a product/performance qualification.

Baseline: clean `main` at `e52e6312`, production unchanged from audited `b90cb611`.
Root authored plan documents; bounded capability researchers and compiler advice read current
sources independently. These reports were produced in assigned scratch and copied here by the
root because the current proposed designs consume their evidence. No shared capability skill,
production code, dependency, runtime store or operator configuration was changed.

## Method and coverage

SurrealDB investigation began with the neo4j-surrealdb skill: guide/reference, all 18 capability
briefs, all catalogs/routes and the complete topic/feature inventory; relevant examples, contracts,
probes and source received deeper inspection. This covers SDK/core/engines/typed values/DDL,
SurrealQL/query/graph/bulk/planner/index/full-text/vector, transactions/errors/security and
GQL/MCP/GraphQL/Postgres/custom API/modules/files/events/maintenance. Broad inventory inspection
is distinguished from independent revalidation of every example. Pinned skill probe receipts
remain historical, 2026-10-05 evidence. Neo4j was not newly investigated as a store alternative.

Targeted Context7 resolution/query followed skill review for selective query and bulk/concurrency
contracts, and supplied the first uncovered-tooling stop for SurrealKit/RocksDB. Official GitHub
source and web follow-up resolved terminal export, cancellation and conflicting tooling/config
versions. Exact installed FastMCP/MCP source settled F10 transport feasibility. Compiler advice
used the matched DataFusion/Arrow capability skill and current semantic owners. No empty search
is treated as absence and no current documentation example is silently transferred across versions.

## Read the evidence by question

| Evidence | Question and design consequence |
|---|---|
| [SurrealDB operations](surrealdb.md) | gRPC terminal export, genuine provisional query streaming, complete read-only derived reconciliation, typed bulk cache admission and exact committed winners |
| [Tooling and deployment](tooling.md) | GQL/frontends versus product consumers, SurrealKit beta.6/3.3 fit, actual server RocksDB controls, maintenance/deadline cancellation, exact FastMCP structured failure paths |
| [Compiler foundation advice](compiler.md) | Necessary semantic admission versus replay; detached re-admission; dependency-closed spill/stream normalization; prepared normalized/Enriched authority and all affected upper/projection consumers |

## Root synthesis and decisive choices

The root inspected current artifact/support checks, normalization collectors, config selection,
manual executable guard, scoped browse, nested pagination, service/PyO3/MCP failures, publisher
backup/search and native cache/reconciliation. It also read decisive pinned SDK export/streaming,
INSERT IGNORE source and installed FastMCP passthrough. Selected mechanisms are developed at the
actual plan owners rather than making these reports another architectural authority.
Independent focused assessment of the assembled proposed plans found no material blocker to the
compiler-first start. It clarified cache text/token ownership and in-batch winner selection,
backup post-commit durability uncertainty, and selection lock/revalidation scope. Those clarifications
are incorporated in the owners; the assessment executed no product tests and is not a conformance
certificate for the unchanged implementation.

- gRPC file export already requires terminal engine completion and matching byte count. Replace
  HTTP backup; do not create a new export protocol. Optional trailer BLAKE3 is not SDK-verified.
- Bound typed arrays implement actual cache bulk admission. INSERT IGNORE can suppress arbitrary
  row errors; prevalidation and complete winner readback are required. Concurrent behavior remains
  an execution control, not a previously demonstrated defect or passed guarantee.
- Query rows are provisional until successful statement end; preserve that meaning in private
  construction, cold audit and staged projection export. A row queue bound is not a byte bound.
- Production semantic checks and immutable predecessor reuse must be co-designed. Detached
  canonical imports re-establish necessary semantics from retained typed data, without providers
  or producer replay. A self-authored manifest is not an acknowledged admission.
- Complete model/serving executable source membership replaces manual helper knowledge. One
  atomic selected handle replaces separately published CLI/serving pins. Rust-owned safe failure
  payloads use FastMCP's existing exact tool/resource passthrough.
- Keep SurrealQL, Rust semantic kernels and product MCP. GQL and generic DB frontends do not
  fulfill a missing current contract. SurrealKit is real maintained tooling but its mutable rollout
  lifecycle is unnecessary for fresh sealed snapshots; revisit only an actual mutable-cache need.
- Use source-confirmed 3.3 RocksDB server controls and maintenance, preserving durability. No
  underlying RocksDB library integration or universal tuning/accounting service is selected.

All proposed benefits are qualitative. No new probes, compile/product tests, concurrent cache race,
late export fault, actual native cancellation, real-library/live-Qwen/operator action or performance
measurement ran in this authoring scope. The two audit counterexamples remain failed product
guarantees from the earlier audit, not newly repaired results. Planned focused closure scenarios
live in the supporting plans. Documentation checks are recorded separately in STATUS.
