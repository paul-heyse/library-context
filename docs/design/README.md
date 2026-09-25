# Architecture map

**Interface-checked, 2026-09-25:** these source entrypoints locate current responsibilities.
[DESIGN](DESIGN.md) and [focused sections](sections/behavior-model.md) own architectural meaning;
their evidence labels distinguish accepted targets from implemented behavior. [STATUS](../../STATUS.md)
and the [active plan](../plans/behavioral-model-forward-plan_2026-09-24.md) own completion claims.

| Responsibility | Contract and source entry | Adjacent consumer / change boundary |
|---|---|---|
| Authoritative facts, schemas, codes and rules | [cpg-schema](../../crates/cpg-schema/src/lib.rs), DESIGN §3/§B2 | Extraction and derivation produce these contracts; codebooks stay append-only |
| Pinned acquisition and extraction | [cpg-extract](../../crates/cpg-extract/src/lib.rs), DESIGN §4 | Acquires library context and emits attributed facts; external analysis environments are explicit |
| Python flow semantics | [cpg-flow](../../crates/cpg-flow/src/lib.rs), [§3.9](sections/behavior-model.md) | Reads ty's index; byte ranges join the second parse to facts |
| Relational derivation, validation and publication | [cpg-core](../../crates/cpg-core/src/lib.rs), DESIGN §5/§6 | Shared validators govern publication; Delta snapshots form the serving boundary |
| Finite graph/summary analysis | [lctx-analytics](../../crates/lctx-analytics/src/lib.rs), [§9.9](sections/behavioral-analysis.md) | Composes attributed conditions and facts; explicit unknown boundaries constrain claims |
| CLI and serving | [lctx](../../crates/lctx/src/main.rs), DESIGN §11 | Orchestrates compilation and snapshot queries; native/Python serving consumes published contracts |

Dependency direction runs from orchestration through capability owners to schema and semantic
contracts. Effects (acquisition, persistence, serving) are explicit boundaries around transformations.
This describes intended ownership; known coupling and incomplete summary capabilities remain in
the active plan and source reviews. It is not a certificate of current architectural conformance.

## Reading and extending

- Conditions, places, coverage and verdicts: [§3.9](sections/behavior-model.md).
- Model application, transfer summaries and composition: [§9.9](sections/behavioral-analysis.md).
- Scope, binding decisions, acquisition, publication, analytics and serving: [DESIGN](DESIGN.md).
- Reasons and alternatives: [ADR index](../adr/README.md).

For a change, identify the semantic owner, its consumer contract and the expected extension axis.
Read that owner plus affected consumers; reuse or compose existing capabilities where their
semantics fit. Detailed fields and invariants belong in executable declarations. Update architecture
when meaning or boundaries change, and use the existing review cadence. Internal refactors need
no additional proof packet. Section IDs never renumber; moves leave short historical pointers.
