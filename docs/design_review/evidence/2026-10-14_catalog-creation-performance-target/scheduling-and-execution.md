# Scheduling and execution evidence

Supporting facts for the [principal review](../../reviews/design_review_catalog-creation-performance-target_2026-10-14.md), inspected 2026-10-14 at `b0306d62d2f0d4175877f5ee147471eb99bb542d`. The coordinator published this source map from a read-only code-mapper inquiry. It supplies no separate overall verdict or finding ledger. Source observations are **Implemented**; alternatives are **Proposed**. No builds, tests, probes or measurements ran.

## Actual sequence and dependencies

`crates/lctx/src/compile.rs:149–160` compiles, admits the artifact, then exports or seals native publication. Export is a distinct route. Catalog completion therefore includes admission and sealing; producer completion is insufficient.

`crates/lctx-model/src/domain/stages.rs:767–875` constructs a stage DAG from declared inputs, completed-input invariant dependencies, contributor-to-assembler edges and epoch edges. It rejects cycles and uses names as deterministic topological ties. This is a dependency description, not a concurrent executor. Publication group codes do not themselves define order (`stages.rs:281`). However epoch reads include every group member (`828–838`), and later groups depend on every preceding publication group (`848–866`). `analysis/owner.rs:313–326` fixes the upper group sequence. Thus the effective DAG includes a publication-prefix chain beyond individual data inputs.

`crates/cpg-core/src/facts.rs:40–94` executes providers serially, completing and draining each. `compilation.rs:717–726` waits for all facts and availability; `740–805` runs normalization serially; `806–814` admits normalized semantics; `839–1150` drives upper stages serially. Existing preparation of normalized authority, bindings and graphs is retained across its real consumers and released at their last consumer (`808`, `818`, `1128`). Reuse does not currently imply overlap.

Upper publication groups are Facts, Local, BaseEvaluation, BaseCompletion, SourceCall, EnrichedExecution, Model, Summary, Structural, AnalyticEmbedding, Analytic, Selection, Synthesis and Retrieval. C0 Core and C1 Evidence are upper owners without their own publication boundary (`analysis/owner.rs:126`), yet the driver still orders their execution. Completed input selection binds exact dependency identity (`compilation.rs:495–519`); Analysis rejects Catalog-only inputs (`464`). Frozen completed inputs become available only when their whole group is complete (`582`). These are consequential invariants for any revised execution route.

Catalog's Local through Summary operations are compact uniform-unrequested owners (`compilation/uniform.rs:6`), consuming required metadata to exact EOF and emitting explicit `UniformNotRequested` plus empty typed outputs. They are not full Behavioral computations. Their completion still participates in the prefix checkpoints. Removing the unnecessary physical chain must retain genuine not-requested and coverage meaning.

## Existing concurrency and bounded ownership

| Location | Existing mechanism | Limit of the observation |
|---|---|---|
| `cpg-core/src/catalog_core.rs:214–283` | Charged 32-root windows; load, optional request lookup, blocking model build, optional retention and emission; window data released | Windows execute in sequence. This is useful bounded ownership, not proof that the chosen window count is optimal. |
| `cpg-core/src/catalog_evidence.rs:413–422` | 32-root evidence windows, awaited in sequence | No assertion that C1 rich frame construction is missing or globally unbounded. |
| `cpg-core/src/analytic.rs:220–242` | Requested analytics consume scoped structural roots | Optional techniques are not mandatory Catalog latency. |
| `lctx-compiler-store/src/stage_runtime.rs:7` | Borrowed CPU work uses `block_in_place` on a multithreaded runtime, inline otherwise | Maintains borrowed ownership; does not create independent stage tasks. |
| `lctx-compiler-store/src/native_bridge.rs:46–58` | Dedicated current-thread runtime, capacity-two submission channel, concurrent `JoinSet` tasks | The bridge is not wholly serialized; asynchronous read/write work can coexist. CPU work on that runtime still warrants separate scrutiny. |
| `lctx-native/src/native_calls.rs:44,112,164` | Eight admitted native calls, owned task registry and drainage | Existing admission is not stage parallelism or a reason to impose another cap. |
| `lctx-compiler-store/src/compiler/ingress.rs:37,67`; `workspace.rs:2087,2572,2623` | Capacity-one per-contribution ingress, one pending acknowledgement, row/byte limits and final drainage | Necessary backpressure and acceptance ownership do not require every independent contribution to run serially. |

## Questions this evidence supports

The next design should distinguish semantic prerequisites, invariant-closure prerequisites, contribution assembly and final publication barriers from physical ordering inherited from the prefix. An ordinary ready-work composition over exact completed immutable views is a simpler initial alternative than a generic scheduler framework. Independent providers and windows may overlap where their actual dependencies and resource ownership permit it; nesting another unrestricted pool around Pyrefly's own shared worker pool is not automatically beneficial.

A revealing change scenario adds an independent fact provider or a Catalog owner that consumes only normalized facts. Its execution should not require unrelated behavioral owner completion unless an actual invariant explains that dependency. A failure scenario must still cancel siblings, join owned workers, settle accepted ingress, freeze deterministic completed views and prevent a consumer from observing unfinished contribution assembly. Native same-attempt write conflicts must be considered separately from CPU readiness.

This map identifies source-level serialization and existing overlap; it does not establish which operation dominated the prior 900-second timeout, or a measured speedup from the proposed schedule. Source reads and searches passed; runtime validation and assembled acceptance are **not_run**.
