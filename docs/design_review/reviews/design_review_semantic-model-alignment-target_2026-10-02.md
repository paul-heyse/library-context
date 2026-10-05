# Semantic-model alignment target

**Design / Target review · 2026-10-02**

**Decision: Accept scoped, at Proposed maturity.** The four plans define a coherent bounded correction of the inspected semantic-model boundaries. They retain the current architecture, give semantic preparation and policy identifiable owners, and preserve the distinctions that could otherwise be lost during consolidation. All six further opportunities have a source-substantiated functional or maintenance contribution; none depends on demonstrating a speed benefit.

This accepts the proposed target and its implementation obligations. It does not certify the current implementation, close the original review’s findings, authorize production remediation, or complete Phase 5 qualification.

## 1. Scope, baseline and evidence

| Field | Scope |
|---|---|
| Targets | [Coordinator](../../plans/semantic-model-incremental-alignment-plan_2026-10-02.md), [native preparation](../../plans/semantic-model-native-preparation-plan_2026-10-02.md), [execution foundations](../../plans/semantic-model-execution-foundations-plan_2026-10-02.md), [serving discovery](../../plans/semantic-model-serving-discovery-plan_2026-10-02.md) |
| Baseline | `main` at `42551010c579c2f7c9ccd1f7449d36911917ee99`, with the initially preserved 117 dirty tracked paths; concurrent documentation publication was visible during review |
| Standard | [Core/template 3.2, code-intelligence profile 1.3 and repository binding](../design_principles/standard.toml) |
| Reviewer | Independent delegated design reviewer; read-only assignment |
| Method | Static inspection of proposed contracts, architectural owners, decisive source and adjacent consumers |
| Maturity | Target remedies remain **Proposed**; inspected existing interfaces provide evidence of fit, not executed remediation |
| Exclusions | Builds, tests, probes, measurements, PostgreSQL actions, real-library compilation, activation and implementation changes |

The functional scope includes finite native request semantics, analytic settings and identity, upper-stage composition, exact dependency closure, narrow synthesis reuse, packet hydration dependencies, meaningful discovery and admitted failure metadata. Extraction/provider semantics and broader behavioral-model extensions remain unchanged.

The [original incremental review](design_review_semantic-model-incremental-alignment_2026-10-02.md) remains the dated **Revise / Implemented** assessment of the existing source. This review assesses its proposed remedies independently.

## 2. Responsibilities and semantic ownership

The proposed ownership direction is appropriate:

| Owner | Governing responsibility | Consumers |
|---|---|---|
| `lctx-model` | Native preparation/formal resolution, fixed analytic policy, exact dependency requirements, packet declarations and public failure meaning | Stage adapters, generation services and generated discovery |
| `cpg-core` | Finite executable stage bindings, completed-input adaptation, CPU placement and operational sampling | Scheduler and model operations |
| `lctx-postgres` | Original guard, canonical receipts/reads, actual-row admission, leases, budgets and request lifecycle | Prepared semantic owners and generation-bound hydration |
| Python MCP | Generated registration and pinned transport serialization | Agent discovery and tool/resource clients |
| Extraction build | Declared embedded runtime scripts and producer capture | Acquisition/compilation identity |

These responsibilities follow [semantic-model §15](../../design/sections/semantic-model.md), [analytics §9](../../design/sections/analytics.md) and [synthesis/serving §11](../../design/sections/synthesis-and-serving.md). No new crate, store, general executor, registry or projection language is needed.

### Fact and fidelity boundary

| Family or result | Provider/revision and fidelity | Coverage, identity and consumer obligations |
|---|---|---|
| Native premises | Existing captured provider/model inputs; finite derived semantics | Preserve stored premise membership, original path/condition identity and explicit unexamined outcomes |
| Exact consumed relations | Canonical generation receipts and acknowledged epochs | Earlier and later prefixes remain different source universes despite sharing one sufficient grant |
| Analytic outcomes | Model definition and complete fixed policy; heuristic or exact-under-context interpretation | Preserve run observations, seeds, limits, diagnostics and restrictions on behavioral claims |
| Synthesis parents | Completed nominal analytic relations | Share only the same acknowledged source universe; replay supplies its own declared inputs |
| Serving packets | Model DTOs over generation-bound canonical inputs | Typed dependency agreement does not establish evidence admission |
| Public failures | Existing coarse failure classification | Safe category/message only; no raw driver, configuration or exception content |

## 3. Native contracts and local verification

The native plan corrects meaning rather than merely moving code. Existing constructors distinguish missing evidence, unsupported defaults and incompatible contexts ([inventory.rs:79](../../../crates/lctx-model/src/domain/native_requests/inventory.rs#L79), [ingress.rs:76](../../../crates/lctx-model/src/domain/native_requests/ingress.rs#L76)). Current service branches erase those causes, while the unexamined operation supplies the generic replacement ([native_service.rs:276](https://github.com/paul-heyse/library-context/blob/42551010c579c2f7c9ccd1f7449d36911917ee99/crates/lctx-postgres/src/generations/native_service.rs#L276), [evaluate.rs:55](../../../crates/lctx-model/src/domain/native_requests/evaluate.rs#L55)).

Retaining the constructor’s cause alongside its Entry value is sufficient. The neutral Partial label avoids assigning one path-level cause to a heterogeneous section. The plan preserves Unknown, unexamined basis, original proofs and identities, zero scalar assignments and NotRequested precedence.

The public formal domain is correctly independent of native context admission. Current resolution follows catalog callables, effective assessments, invocation variants, slots and formal links ([native_service.rs:826](https://github.com/paul-heyse/library-context/blob/42551010c579c2f7c9ccd1f7449d36911917ee99/crates/lctx-postgres/src/generations/native_service.rs#L826)). Preparing that domain once must retain a legitimate defaulted formal even when its exact scalar assignment is unsupported. The planned required/defaulted twin explicitly challenges this distinction.

Pure preparation owns correspondence, replay and composition. Storage retains canonical validation, actual-row membership and the original guard. Releasing the lease mutex before pure CPU work while retaining charged inputs and the guard is a sound boundary; publishing only after reconfirming that guard preserves admission. Definition capture must include inventory and the newly extracted operations; the existing native source list demonstrates that obligation ([mod.rs:24](../../../crates/lctx-model/src/domain/native_requests/mod.rs#L24)).

## 4. Composition, execution and reuse

### Stage order and exact closure

The proposed closure has two products: exact read/validation requirements and sufficient runtime grants. That distinction is essential. Schedule rejects duplicate relation TypeIds ([stages.rs:630](../../../crates/lctx-model/src/domain/stages.rs#L630)), while `read_at_epoch` narrows an acknowledged vocabulary grant without allowing widening ([stages.rs:1457](../../../crates/lctx-model/src/domain/stages.rs#L1457)).

Using the widest required publication **ordinal**, rather than boundary codes, is correct. Reusing the scheduler’s publication-order construction avoids another chronological authority ([stages.rs:528](../../../crates/lctx-model/src/domain/stages.rs#L528)). The design has no unresolved construction cycle: the finite binding supplies publication groups before owner declarations require closure; final Schedule construction revalidates those groups and binds the order to its digest. Planning order grants no runtime access.

The plan also preserves explicit invariant epochs, stream ordering, direct fact inputs and own-output refusal. Epoch-aware sessions are required because registration remains name-keyed. Existing [ConsumedInputs](../../../crates/cpg-core/src/consumed_rows.rs) supplies the relevant source-resolution pattern; migration must retain independent validator obligations rather than treating decoder coalescing as validation equivalence.

### Graph and CPU lifetimes

A shared graph preparation is permitted only when an actual consuming stage’s grants cover the scheduled union. Every later consumer must retain its own source/budget admission. Existing preparation compares the completed source set and budget before exposing a borrowed graph ([analysis_graphs.rs:180](../../../crates/cpg-core/src/analysis_graphs.rs#L180)). The explicit separate-preparation/refusal rule handles a future incompatible source set.

O1 addresses an observed mechanism: the current sampler resides in the same task as synchronous work ([stage_runtime.rs:73](../../../crates/cpg-core/src/stage_runtime.rs#L73)). Tokio 1.53.1’s inspected `block_in_place` contract accepts borrowed closures, suspends same-task concurrency, rejects current-thread runtime use and cannot cancel executing work. The proposed independent sampler, explicit inline fallback and synchronous retention of graph/access borrows therefore fit together. Cancellation is observed between boundaries; opaque kernels drain before resources can be released.

### Narrow S0 reuse

O2 has a concrete three-relation basis: frames and automatic selection independently decode and retain `AnalyticFrame`, analytic invocation and `TechniqueResult` ([frames.rs:30](../../../crates/lctx-model/src/domain/synthesis/frames.rs#L30), [automatic.rs:47](../../../crates/lctx-model/src/domain/synthesis/automatic.rs#L47)); production visits both ([production.rs:33](../../../crates/lctx-model/src/domain/synthesis/production.rs#L33)).

A narrow shared inventory removes that duplication while preserving each operation’s declaration and context/definition checks. Independent replay construction and refusal to share different epoch universes prevent reuse from becoming an undeclared producer dependency.

### Analysis record obligations

| Question | Projection/method/model | Limits and evidence retained |
|---|---|---|
| Structural/Summary graph questions | Existing named projections, universe, directions, multiplicity and finite semantics | Existing source assessments, witnesses, work/proof limits and explicit uncertainty |
| Ranking, communities, concepts and neighbors | Complete model-owned policy; heuristic/exact-under-context distinctions unchanged | Canonical accumulation, all forty community runs/histories, bounds and outcome records |
| Synthesis | Completed declared nominal parents and deterministic construction | Qualification, support closure and independent replay inputs |
| Serving | Canonical typed hydration under one original generation guard | Actual receipts/membership, request budgets, complete admitted envelopes and disclosed omissions |

## 5. Change and failure scenarios

| Scenario | Proposed route and consequence |
|---|---|
| New native path case — domain extension | Model preparation changes; storage hydrates declared inputs and enforces admission without interpreting the new path |
| Analytic policy edit — policy | One authored value changes persisted settings, complete identity and executable bindings together |
| New upper stage — composition | One finite binding supplies phase, declaration, runner, graph needs and publication membership; missing or cross-phase routes refuse before effects |
| Added invariant or two vocabulary epochs — contract | Shared closure retains exact requirements and lowers sufficient grants; actual loaders preserve distinct receipts/universes |
| Packet or predicate addition — representation/domain extension | Typed packet dependencies and model annotations supply derived discovery; Python acquires no semantic dictionary |
| Numerical mechanism substitution — mechanism | Existing complete numerical-result contract and requalification preserve semantic eligibility, identity and ranking |
| Cancellation, unavailable guard or envelope refusal — failure | Actual work drains; structured failure is emitted only when the original grant admits its complete envelope |

These routes localize meaning without promising that new phenomena require no implementation work. O6 supplies guidance and independent controls for these existing extension routes; it does not introduce a universal extension registry or activate broader model work.

## 6. Correctness and fidelity gates

All verdicts below apply to the **Proposed target**, supported by static contract/source inspection.

| Gate | Verdict | Evidence and boundary |
|---|---|---|
| G1 Authority | pass | Native semantics, analytic policy and packet meaning have model owners; effects remain controlled |
| G2 Semantic fidelity | pass | Exact refusal causes, public formals, numeric encodings and distinct epochs are preserved |
| G3 Validity | pass | Checked construction, shared validators, actual-row admission and meaningful negative controls remain required |
| G4 Hidden behavior | pass | Pure preparation is separated from effects; telemetry stays outside semantic identity; script narrowing retains actual inputs |
| G5 Consistency/recovery | pass, scoped | Original guards, retained work, poisoned attempts and unselected failed rebuilds remain; no universal early-error guarantee is claimed |
| G6 Transformation/reuse | pass | Exact requirements survive grant lowering; graph/S0 reuse requires identical acknowledged sources |
| G7 Truthful capability claims | pass | Remedies are Proposed, performance unmeasured and qualification stopped |
| G8 Library leverage | pass | Existing scheduler, typed readers, Tokio and MCP mechanisms fit; no unnecessary framework is introduced |
| CI-G1 Fidelity | pass | Unknown stays distinct from absence; analytic heuristics do not become behavioral proof |
| CI-G2 Evidence closure | pass | Binding/helper agreement never replaces canonical receipt and actual proof-row checks |
| CI-G3 Evaluation integrity | pass | Independent expectations/oracles remain required; no gold input or parameter tuning is introduced |

These are architectural review verdicts, not passing runtime check receipts.

## 7. Findings and applicability

**No new material target finding was established.** FP-01–FP-06 are satisfied for the selected Proposed scenarios. Applicable authority, identity, operation, reuse, lifecycle and discoverability rules are supported by the contracts above.

The original F01–F08 remain open remedies at the [coordinator’s disposition table](../../plans/semantic-model-incremental-alignment-plan_2026-10-02.md#7-current-disposition--sole-execution-owner). Acceptance of this target provides no implementation closure.

O5’s exclusion is consequential. Existing `RequestExecution` does not blanket-poison a grant after an ordinary operation error; a healthy grant can admit a later failure envelope ([runtime.rs:218](https://github.com/paul-heyse/library-context/blob/42551010c579c2f7c9ccd1f7449d36911917ee99/crates/lctx-postgres/src/generations/runtime.rs#L218), [serving.rs:484](https://github.com/paul-heyse/library-context/blob/42551010c579c2f7c9ccd1f7449d36911917ee99/python/lctx_storage/src/serving.rs#L484)). Guard loss, expired deadline or insufficient remaining budget can prevent that admission. Those cases belong to the stated fallback-once path. A recognized failure kind alone therefore never guarantees structured metadata.

## 8. Library fit and total complexity

The fixed policy retains established numerical mechanisms while removing independently authored bindings. Existing source capture already prevents the identity collision that policy prose alone might suggest ([analytics/build.rs:88](../../../crates/lctx-model/src/domain/analytics/build.rs#L88)); the improvement is coherent policy ownership and propagation.

Pinned FastMCP 4.0.5 and MCP 2.2.0 source inspection supports tool `is_error`/metadata and resource `MCPError` handling. Tool failure metadata belongs outside the success schema; resource failure data belongs in the JSON-RPC error envelope. Existing [wire.py:25](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L25) serializes success envelopes only, so the explicit error-envelope extension and actual transport controls remain necessary.

No current library interface inspection establishes those future runtime controls as passed.

## 9. Alternatives and tradeoffs

Ordinary typed functions, fixed policy values and finite bindings are the simplest viable corrections. A general workflow DSL, preparation framework, owned worker registry or replacement MCP transport would introduce lifecycle and configuration obligations without resolving a demonstrated additional need.

O4 appropriately narrows only scripts capture. Current capture traverses that whole subtree ([producer_fingerprint.rs:44](../../../scripts/producer_fingerprint.rs#L44)); the inspected production embedding is the deployment runner ([deployment.rs:257](../../../crates/cpg-extract/src/deployment.rs#L257)). A shared declaration used by embedding and capture avoids parsing Rust source. Other conservative roots remain intact. The explicit maintenance obligation and implementation-time re-audit are reasonable costs; this review does not establish that arbitrary future scripts are irrelevant.

## 10. Verification and uncertainty

Static inspection is sufficient for this Proposed target decision. No probe is necessary to settle the selected ownership and composition questions.

| Claim/control | Current outcome |
|---|---|
| Proposed boundary coherence | Static assessment completed, 2026-10-02 |
| Native twins, exact-epoch cases, policy mutation and S0 reuse | **not_run**; targeted implementation controls required |
| Heartbeat, fallback and cancellation/drain behavior | **not_run**; future deterministic control required |
| Actual discovery and admitted tool/resource failures over both protocols/transports | **not_run**; real transport controls required |
| `NEXTEST_TEST_THREADS=8 just test-all` and `just hygiene` | **not_run** in this review; combined final-tree implementation obligations |
| Real-library Q0, journeys and activation | **not_run**; operator stop remains |
| Speed, throughput and total RSS benefits | Unmeasured; no target acceptance threshold |

Packet coverage must include empty attempted reads, child bindings and startup prepared dependencies. It must not merely compare declaration lists.

Universal early-error byte bounds remain outside this increment. They require a transport-owned request-limit/termination decision before that guarantee is advertised; an unbounded echoed request ID cannot fit a finite response bound.

## 11. Authority, disposition and reconstruction

The coordinator owns current execution disposition and shared identity/removal obligations. Architectural meaning changes use the ADR/owning-section route; accepted ADRs remain immutable.

Native definition, complete analytic policy, schedule/model/producer capture, packet mappings and WireIdentity must reflect their changed contracts. Operational sampling does not enter semantic digests. Redundant authorities are removed with their migrated consumers.

Reconstruction is required before serving affected state as valid for the new tree. It need not restart the stopped FastMCP qualification campaign during plan authoring. Deferred reconstruction remains an activation prerequisite; stale generations cannot be relabeled as validated. Existing Q0 and earlier finding owners retain their independent boundaries.

## 12. Architectural judgments and decision

| Judgment | Verdict | Reason |
|---|---|---|
| A1 Localize change | satisfied, Proposed | Native/policy/packet changes follow coherent owners; store and transport concerns remain bounded |
| A2 Encode domain meaning explicitly | satisfied, Proposed | Refusal causes, public-formal validity, exact epochs, heuristic settings and lifecycle distinctions are represented and govern the proposed operations |
| A3 Extend through composition | satisfied, Proposed | Finite bindings, shared closure and narrow reuse compose existing capabilities without another framework |

**Bounded decision: Accept scoped, at Proposed maturity.** The target is suitable for subsequent authorized implementation and its stated independent acceptance controls.

**Enclosing architecture:** the existing implementation still needs the bounded revisions diagnosed by the original review. Its release qualification, real-library acceptance and operator activation remain unresolved or stopped at their current owners.

The coordinator’s next obligation is to publish this target assessment without converting it into implementation closure. Subsequent remediation must preserve the distinctions and independent controls above, then receive the planned integrated source assessment and final-tree verification.
