# Heuristics for Efficient Architecture

**Version 1.0 · Adopted guidance · 2026-10-05.** Repository- and domain-agnostic companion to the [design principles](design-principles.md).

## Purpose and standing

Use these heuristics to recognize avoidable work and machinery while preserving the core's correctness, fidelity, authority, consistency and declared assurance obligations. They guide judgment; they do not introduce MUSTs, gates, required artifacts, a scoring system or an obligation to assess every heuristic for every change. Mechanisms mentioned below are alternatives to consider, not components to implement by default.

For a supported outcome and its guarantees, first look for a realization that removes work rather than performing the same work faster. Retain additional work or machinery when it purchases a concrete capability, protection or operational benefit whose value warrants the overall burden. Judge material differences over representative operations and lifecycles, not against an imaginary globally optimal design.

Use the design principles together with these heuristics when making consequential architectural and implementation choices. Consider relevant execution patterns before committing to physical organization, interfaces, preparation, assurance and lifecycle mechanisms. Address material mismatches while the design remains easy to change: operational structure can become expensive to replace even when ownership, typing and local correctness are strong.

These assessments use plain-language reasoning about material architectural consequences. They do not require numerical estimates, exhaustive inventories, cost models, runtime accounting or additional proof machinery. Such mechanisms need a separate concrete functional or operational requirement. Quantitative performance claims still require measurement.

## A. Contracts and physical realization

| # | Heuristic | How to assess it |
|---|---|---|
| H1 | **Preserve promised behavior and assurance, not incidental mechanisms.** | Separate required meaning, completeness, effects, consistency and failure protection from the protocol currently implementing them. Ask whether a simpler mechanism preserves those obligations. A matching happy-path result is insufficient, but an existing stage, intermediate state or compatibility path is not automatically a requirement. |
| H2 | **Choose physical units for execution needs, not for the number of semantic distinctions.** | Look for each declared kind, operation or module automatically creating its own object, call, transaction, task or lifecycle. Retain distinctions in contracts while grouping, splitting or fusing execution where useful. Assess which obligations apply to the complete operation and which must hold at each internal step. Internal physical operations need not individually reproduce the full domain representation or re-establish guarantees secured by their enclosing operation. Preserve invariants required for safe execution and establish promised properties before consumers rely on them. Conversely, do not fuse work whose isolation, failure handling or independent scheduling has a real purpose. |
| H3 | **Keep authority over meaning separate from execution placement.** | Let the semantic owner define and qualify an operation while a suitable engine or kernel executes it. Compare moving computation with moving data, including conversion and preparation. Do not force execution beside the owner, or inside the database, merely because that is where the definition or data resides. |
| H4 | **Delegate complete capabilities; implement only the remaining contract gap.** | After choosing a library or engine, identify which obligations it actually discharges and which remain. Scrutinize wrappers that recreate its planning, caching, transactions, retries or validation. Retain additional protection for a concrete gap or failure exposure, not because delegation itself feels insufficient. |

## B. Necessary work and optimization

| # | Heuristic | How to assess it |
|---|---|---|
| H5 | **Make ordinary work follow the smallest semantically sufficient input scope.** | Ask which records, fields, partitions and dependencies can affect the answer, and whether the access path can reach them without repeatedly processing unrelated state. Narrow scope only when justified: a requested output subset may still require intermediate elements, global context or evidence of absence. |
| H6 | **Match result detail and computation to useful demand.** | Question richer results or additional analyses when producing them adds material work without a consumer or reuse benefit. A richer native or shared result may be preferable when narrowing it would introduce more machinery than it saves. Let demand and valid termination conditions reach upstream work. Stop when the required answer and its completeness obligations are established, not simply when enough output has accumulated. |
| H7 | **Reduce data before expensive work or expansion when semantics permit.** | Look for opportunities to apply selective predicates, projection, aggregation or deduplication before decoding, transfer, joins, traversal or other expansion. Compare the reduction's own cost with what it avoids. Reordering must preserve effects, error behavior, multiplicity, ordering and any context required by later computation. |
| H8 | **Expose useful units of intent and preserve optimization across boundaries.** | Prefer interfaces that can express sets, filters, requested fields and composed operations over caller-driven item-by-item coordination. Keep useful intent visible until the responsible mechanism can plan it. A bulk call that merely hides scalar calls, or a pipeline that materializes between every stage, may retain the original cost. |
| H9 | **Choose algorithms and access paths for workload shape, not just data type.** | Examine lookup versus scan behavior, repeated searches, sorting, density, selectivity, skew and update frequency. A graph representation does not choose a traversal algorithm; a table does not guarantee an efficient query. Prefer a suitable established algorithm or access path before compensating for a poor one with more concurrency. |
| H10 | **Recognize material amplification between input and answer.** | Consider how scans, repeated passes, fan-out, path counts, intermediate width, retained versions and simultaneous requests interact beyond the returned rows. Look for combinations of individually reasonable bounds that still multiply into unreasonable work. Include highly connected or skewed inputs and empty-result cases where proving absence requires extensive examination. |
| H11 | **Establish normal-workload feasibility before relying on protective limits.** | Identify a credible physical route for the input sizes, shapes and operations claimed as supported. Cancellation, budgets and honest refusal do not alone establish feasibility. Streaming, partitioning or spilling may provide a credible execution route when they suit the operation and available resources. Distinguish a deliberate product-scope limit from a workaround that predictably prevents ordinary supported use. |

## C. Preparation, reuse and change

| # | Heuristic | How to assess it |
|---|---|---|
| H12 | **Separate stable preparation from changing values, and amortize it over its validity.** | Find parsing, resolution, validation, planning, indexing and layout work repeated although its dependencies remain unchanged. Prepare at the appropriate lifetime and bind changing inputs separately. Distinguish reusable preparation from reusable results, and avoid paying eager preparation for capabilities that are not actually used. |
| H13 | **Propagate relevant changes, not merely the fact that something was touched.** | Track complete dependencies at a useful granularity, including membership changes, deletions and absent lookups. Where justified, update affected portions and stop downstream propagation when a recomputed value is substitutable under the consumer's contract. Separate incidental provenance from result-affecting dependencies; uncertain dependencies still require conservative invalidation. |
| H14 | **Make reuse earn its lookup, retention and maintenance costs.** | Include key construction, hashing, validation, synchronization, storage, eviction and rebuilding in the comparison with recomputation. Check whether proving an input unchanged repeats most of the work being avoided. Coalesce equivalent in-flight preparation where appropriate, and avoid keeping overlapping caches or artifacts without distinct scopes and benefits. |

## D. Representations, movement and live state

| # | Heuristic | How to assess it |
|---|---|---|
| H15 | **Use operation-shaped physical representations under one semantic authority.** | Consider compact, indexed, columnar, adjacency-based or otherwise specialized forms when consumers repeatedly reshape the canonical representation. Preserve required identity and meaning through explicit mappings and dependencies. One authority does not require one physical copy; multiple representations do not justify independent editable meanings. |
| H16 | **Materialize, copy, transfer or persist only for a useful physical purpose.** | Follow major representations through the operation and question repeated conversion, persistence and reload. Keep compact or factorized forms and hydrate details when needed. A copy may be worthwhile to improve locality, release a large backing buffer or isolate ownership; zero-copy and never-materialize are not objectives in themselves. |
| H17 | **Manage the combined live working set, not only individual collections.** | Consider which substantial allocations and representations remain live together: input buffers, hydrated objects, intermediates, queues, caches, workspaces and outgoing results. Release state after its last required use; stream, partition or spill where suitable. A bounded batch does not bound memory if many batches or backing allocations remain live elsewhere in the pipeline. |

## E. Assurance without repeated reconstruction

| # | Heuristic | How to assess it |
|---|---|---|
| H18 | **Enforce where relevant uncertainty enters or changes, before the property is relied on.** | Identify the failure class each check covers and whether inputs, interpretation, authority, environment or failure exposure have changed. A read-only step may still cross a trust boundary; an internal layer crossing may add no uncertainty. Scope enforcement to what can invalidate the guarantee, not merely to what mutates data. |
| H19 | **Carry established properties through operations that preserve their premises.** | Avoid making controlled immutable data repeatedly lose its validated status as it moves between components. Retain the validity scope through types, ownership, snapshots or other sufficient mechanisms. Re-establish affected properties when premises change; a label or old check is not protection against uncontrolled mutation or stale context. |
| H20 | **Match assurance work to the guarantee and failure exposure.** | Distinguish checks that establish different properties from checks that repeat unchanged conclusions. Prefer straightforward existing checks and established validity where they provide the required protection. Preserve independent protection: a digest does not establish semantic correctness and repeating the producer is not an independent oracle. More elaborate evidence mechanisms, such as certificates, are warranted only when they resolve a concrete assurance or operational problem. |

## F. Execution, coordination and recovery

| # | Heuristic | How to assess it |
|---|---|---|
| H21 | **Coordinate shared capacity across the operation, not independently inside each layer.** | Examine nested pools, queued work, connections, device memory and competing workload classes together. Let downstream capacity constrain upstream admission. Work that no longer has a consumer should be cancelled or drained appropriately, unless another consumer or committed obligation requires completion. More asynchronous work does not establish more useful throughput. |
| H22 | **Order only the work that has a real dependency.** | Look for broad barriers, global ordering, sequential coordination and shared ownership that force independent work to wait. Narrow synchronization or pipeline stages when contracts permit. Preserve required ordering, deterministic reductions and effect visibility; reducing elapsed time is not permission to increase total work without a worthwhile benefit. |
| H23 | **Match exclusion and atomic visibility to the effects they protect.** | Keep locks, transactions, leases and scarce exclusive resources out of unrelated computation where possible. Separate private preparation from publication when the contract allows. A short final commit remains correct only if the published work's premises still hold—for example through a pinned snapshot or a conflict check covering concurrent change. |
| H24 | **Bound failure amplification and repeat only the necessary unit of work.** | Give retries an appropriate owner, preserve idempotency and consider retry multiplication across layers. Compare replaying a failed step with restarting an entire workflow. Checkpoints are useful when recovery benefits exceed their write and lifecycle costs; disposable computation does not automatically need durable resumability. |
| H25 | **Scale operational machinery to actual participants and recovery obligations.** | Assess coordination, durable history, compatibility, leases and recovery against real writers, readers, failure domains and supported lifecycles. Use the simplest sufficient protocol rather than importing a more demanding deployment model. A local deployment can still have concurrent writers or crashes; simplicity is not an assumption that failures cannot occur. |

## G. Whole-system economics and architectural judgment

| # | Heuristic | How to assess it |
|---|---|---|
| H26 | **Keep auxiliary work proportionate to the capability it supports.** | Inspect the cost of planning, introspection, hashing, provenance, logging, diagnostics, cleanup and health checks as well as useful computation. Prefer compact references, aggregation and selective detail where they preserve required assurance. Rich observability need not mean copying every evidence payload or emitting an event for every primitive operation. |
| H27 | **Judge the full lifecycle and expanded machinery, not the concise declaration.** | Follow a routine use or extension through construction, reads, writes, index maintenance, retained versions, cleanup, recovery, build/test setup and operator work. Consider the operational and maintenance burden introduced by generated objects and library integration. A faster read may buy an unjustified write burden; less handwritten code may still create a more expensive system. |
| H28 | **Remove avoidable work first; compare complete alternatives before tuning what remains.** | Usually consider removing unnecessary work before optimizing the work that remains. Compare complete alternatives qualitatively, considering work, latency, memory, movement, coordination and maintenance where material. A clearly useful local improvement does not require redesigning or exhaustively comparing the surrounding system. Static evidence can establish unnecessary amplification; quantitative speed or capacity claims require representative end-to-end measurements. |

## Interpretation safeguards

These are conditional defaults, not maxims such as “always batch,” “always cache,” “always push down,” “always use native execution,” or “always avoid copying.” Batching can worsen latency; caching and indexing add maintenance; fusion can increase live state or weaken failure isolation; a retained view can be more economical than repeatedly reconstructing it. The relevant question is which complete conforming realization fits the supported workload.

“Semantically sufficient scope” is not synonymous with the requested output subset. An exact global analysis, a negative answer or a relationship whose path crosses outside the output region can require broader input. Likewise, an unchanged identifier or a matching digest is not independent evidence that semantics, interpretation or required premises are unchanged.

A material structural cost can be challenged without first building a benchmark. A proposed improvement must still distinguish work demonstrably removed from latency, throughput or capacity benefits that remain unmeasured. The purpose is to select a better architecture, not to require a performance experiment for every change.

## Relationship to the design principles

The [design principles](design-principles.md) and the architectural judgments named by the repository binding govern acceptance. This companion makes their execution-fit guidance tangible through recognizable patterns. It creates no separate acceptance gates or exception process. Existing principles, assessment lenses and identifiers remain in place; overlap is intentional.

Use the companion during design and planning before material physical choices become fixed, during reviews within their existing scope, and for consequential choices left open during implementation. Do not restart settled reviews or repeat a whole-list assessment during ordinary work. Record concrete concerns in the existing design, plan or review discussion.

Use only the relevant heuristics to settle a scoped design question. A useful finding names the avoidable work, the reason it arises, a simpler conforming alternative and any material uncertainty. It does not require a new registry, exhaustive trace, permanent checker or per-change checklist.

## Relationship to the original 20 drafts

| Original draft | Current coverage |
|---|---|
| 1. Scope contract enforcement | H2, H18 |
| 2. Physical versus semantic granularity | H2 |
| 3. Meaning ownership versus execution placement | H3 |
| 4. Guarantees versus mechanisms | H1 |
| 5. Validation where uncertainty changes | H18 |
| 6. Checking versus independent assurance | H20 |
| 7. Preserve established properties | H19 |
| 8. Operation-appropriate representations | H15 |
| 9. Useful interface units | H8 |
| 10. Relevant work scope | H5 |
| 11. Work versus result size | H10 |
| 12. Preparation lifetime | H12 |
| 13. Coordination/resource lifetime | H23 |
| 14. Materialization and transfer purpose | H16 |
| 15. Expansion and bounds | H10 |
| 16. Feasibility versus protective limits | H11 |
| 17. Dependency-aligned invalidation | H13 |
| 18. Delegation without defensive duplication | H4 |
| 19. Proportionate operational machinery | H25 |
| 20. Fully expanded design burden | H27 |

H6, H7, H9, H14, H17, H21, H22, H24, H26 and H28 make additional mechanisms explicit; several are already supported at a higher level in the governing principles. “Additional” here means additional to the brainstormed heuristic list, not necessarily new principles.

## Source relationships

The draft was synthesized from core design principles version 3.3, including its governing objective, foundations, computation and evidence rules, and execution-fit assessment. The [local principles](design-principles.md) remain authoritative; the repository binding maps this companion to its applicable foundation and architectural judgment. This guidance does not claim that every sentence appears verbatim in the core.

Relevant external primary-source examples named by the source proposal are PostgreSQL's “Using EXPLAIN” documentation (examined versus returned rows and physical plans), Apache DataFusion's “Query Optimizer” documentation (equivalence-preserving plan transformations), the Rust Compiler Development Guide's “Incremental compilation in detail” (unchanged-result propagation cutoff), and Google's SRE chapters “Handling Overload,” “Addressing Cascading Failures,” and “Monitoring Distributed Systems” (resource coupling, retry amplification and observability cost). These examples support mechanisms, not a claim of measured improvement for this repository.
