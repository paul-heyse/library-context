**Yes, your intuition is broadly right: derive graph representations from lower-level facts, then use those graphs to produce higher-order relationships that other analyses can consume. But I would make the architecture more general than “one graph, followed by another graph with additional edges.”**

The architecture I recommend is:

> **A typed semantic data model, exposed through multiple immutable graph projections, with reusable analysis operators that produce new typed relations and retain their supporting evidence.**

The important architectural improvement is not simply “use graphs more.” It is **making relationships, transformations, and their composition rules reusable**, so that each new analysis does not rebuild its own interpretation of the underlying facts.

I revisited the graph and analysis infrastructure at the same reviewed commit, `5626941`. You already have an excellent starting point: the invocation graph is immutable, graph-local indices map back to canonical IDs, parallel arcs are preserved, and metadata is kept separately from graph topology. The projection specification also declares its node universe, accepted evidence, and unresolved-target policy. The limitation is that this general projection registry currently returns only the invocation projection, while other analyses construct their own specialized representations.  

**I would extend and generalize those existing patterns, not replace them.**

# 1. The right mental model: a compiler for semantic relationships

Think of the pipeline as compiling evidence into increasingly useful representations:

**Provider facts → normalized semantic relations → analysis-specific graphs → derived semantic relations → API-level analytical products.**

There are two important qualifications.

First, the process is **not exclusively graph-to-graph**. Some stages are joins, some are graph algorithms, some are fixed-point computations, and some are aggregations. An analysis might consume three graph projections and two ordinary relations, then produce a relation that becomes an edge layer in a subsequent graph.

Second, **a graph is a view of the semantic model, not the semantic model itself**.

For example, the same function can participate in an invocation graph, a type-dependency graph, a parameter-flow graph, and an API-composition graph. Those representations should share its canonical identity without requiring four independently maintained function objects.

This resembles a compiler’s separation between intermediate representations, analysis passes, and analysis management. MLIR is a useful architectural reference for declaring analysis dependencies and deciding when an analysis remains valid after another transformation. I am recommending that separation of responsibilities, not adopting MLIR as a dependency. :chatgpt-content-reference{index="2"}

## Three components should remain separate

| Component | What it owns | What it should not own |
|---|---|---|
| **Semantic data model** | Meaning and identity of entities, relationships, observations, assumptions, and derived assertions | A particular graph library’s indices or storage format |
| **Graph projection** | A selected node universe, selected relationships, topology, and mappings to semantic records | New language semantics hidden inside its construction |
| **Analysis operator** | A declared transformation from inputs to outputs, including composition rules and evidence | Its own private reinterpretation of shared relationship meanings |

This separation is what makes new facts easy to integrate.

An additional provider can contribute to an existing relation such as a resolved call binding. Every analysis consuming that relation benefits without learning about the provider. A genuinely new semantic capability, such as object-sensitive field identities, can add a new relation and transfer rule without forcing a redesign of graph storage.

**New evidence should usually extend a shared vocabulary; new analyses should usually compose that vocabulary.**

# 2. Your “parallel graph” idea is one of four useful constructions

I would support four different ways of building graphs from other representations. They solve different problems.

## 2.1 Edge overlays: additional relationships over existing entities

This is closest to your suggestion.

One graph contains direct relationships. Another exposes derived relationships over some of the same entities:

| Base relationship | Derived overlay |
|---|---|
| Direct invocations | Bounded transitive delegation |
| Definitions and uses | Parameter-to-sink influence |
| Class membership and overrides | Effective member relationships |
| Direct usage handoffs | Multi-step composition relationships |

The derived overlay need not duplicate node metadata. It can share an entity dictionary and retain only its own edge records and adjacency.

However, a derived edge needs a more precise meaning than “connected through the lower graph.” For example:

> “Parameter `p` can contribute unchanged to normal return `r`, under condition `c`, using these admitted transfer rules.”

That is a useful semantic relationship. “There is some path from `p` to `r`” is usually too weak.

## 2.2 Summary or quotient graphs: change the unit of analysis

Sometimes the higher-level graph should have **different nodes**, not merely more edges.

Examples include collapsing a mutually recursive group into a component, representing a function body by its input/output boundary, or grouping related APIs into a subsystem.

Your code already performs one version of this: it builds a caller–callee graph, computes strongly connected components, and establishes a deterministic callee-first schedule. 

I would make the mapping between levels a reusable artifact:

**Higher-level node ↔ constituent lower-level entities**

**Higher-level edge ↔ supporting lower-level relationships or summary**

There is an important distinction here: an SCC aggregation and a semantic function summary are not equivalent. SCC membership is a topological property. A function summary must describe what information crosses its boundary.

Likewise, a community-level edge can be useful for navigation but cannot automatically serve as a witness to a specific value-flow path through that community.

## 2.3 Incidence graphs: represent relationships that involve more than two things

Many of your most important relationships are not naturally binary.

Consider argument binding. Its meaning depends on the call site, selected target, signature variant, actual argument, formal parameter, and binding context.

Reducing that immediately to:

`argument → parameter`

loses the information needed to prevent invalid combinations.

Instead, retain a typed relationship record such as:

```text
CallArgumentBinding
    call_site
    target_alternative
    signature_variant
    actual_argument
    formal_parameter
    condition
    supporting_evidence
```

An analysis can expose that record as an edge carrying a relationship ID. Another can expose it as a relationship-node connected to its participants.

The latter is an **incidence representation**: a way to express multi-part relationships using ordinary graph infrastructure.

I would use this selectively for call bindings, multi-input transformations, conditional transfers, and derivations. You do not need to convert every fact into a graph node.

## 2.4 Product or state graphs: combine program location with analysis state

More sophisticated analyses often need to distinguish visits to the same program entity under different circumstances.

The working node becomes something like:

```text
(program_point, tracked_source, call_context, abstract_state)
```

For example, the same function could be analyzed with one parameter originating from a timeout option and another originating from a retry policy. Those cannot be merged merely because execution reaches the same function.

This is where graph analysis becomes a reusable semantic computation rather than ordinary breadth-first search.

Such expanded graphs can remain implicit: store discovered states and transitions in a worklist rather than materializing the entire Cartesian product.

**The four constructions should coexist. Do not force summaries, multi-part assertions, and analysis states into one “add more edges” mechanism.**

# 3. The most important modeling decision: entities, occurrences, and boundary ports

A function-level graph is valuable for architecture and discovery, but it is too coarse to serve as the universal substrate for behavioral analysis.

I would distinguish three levels explicitly.

## 3.1 Entities

These are relatively stable semantic objects: functions, classes, parameters, declared fields, types, modules, and public access paths.

These support API discovery, ownership, inheritance, and structural dependency analysis.

## 3.2 Occurrences and events

These are particular source occurrences: a call site, a binding event, a use, a return statement, a predicate evaluation, or an attribute access.

These support flow-sensitive reasoning.

For example:

```python
x = first()
consume(x)
x = second()
consume(x)
```

A graph with one node named `x` loses the distinction between the two values and their uses. Your existing flow provider already preserves uses, definitions, guarded reaching relationships, and value sources. The architectural task is to expose those identities consistently across consumers. 

## 3.3 Boundary ports

These are the interfaces through which a summarized computation interacts with its surroundings.

For a function, useful ports include parameters, normal return, yielded values, exception outputs, and admitted receiver/field or effect channels.

The central representation should therefore be closer to:

**Function input port → function output port**

than:

**Function → function**

A summary might state:

```text
TransferSummary
    owner_function
    input_port
    output_port
    transfer_kind
    condition_over_inputs
    exit_channel
    precision_profile
    support_reference
    unresolved_obligations
```

This is illustrative vocabulary, not a request to duplicate your existing summary tables. Your finite-summary infrastructure already preserves parameter origins, conditions, call bindings, return-site evidence, refusals, and coverage. I would consolidate that into a consistent boundary-oriented interface. 

**Boundary ports are the key to composing analyses across functions without repeatedly traversing their complete implementations.**

They also provide an extension point: a future provider can improve a function’s summary without changing every downstream consumer.

# 4. A concrete example of how the layers compose

Consider:

```python
def select_timeout(timeout, fallback):
    if timeout is None:
        return fallback
    return timeout

def fetch(url, timeout=None, fallback=30):
    selected = select_timeout(timeout, fallback)
    return transport(url, timeout=selected)
```

Assume the call to `select_timeout` is resolved under the selected analysis model.

## The source-level view

The source view contains the definitions, uses, predicate evaluation, return sites, call arguments, and call bindings.

It retains two separate paths through `select_timeout`.

## The function-summary view

The summary exposes:

| Input | Output | Transfer | Condition |
|---|---|---|---|
| `timeout` | Normal return | Identity | `timeout is not None` |
| `fallback` | Normal return | Identity | `timeout is None` |

The predicate also records a **control influence** from `timeout`: it determines which source is selected. That is different from the value of `timeout` being copied into the result.

## The caller-composition view

The composition operator substitutes the caller’s actual arguments into the callee’s formal ports and conditions.

It can then derive, subject to the required execution premises:

- `fetch.timeout` supplies the `transport` timeout argument on the non-`None` branch.
- `fetch.fallback` supplies that argument on the `None` branch.
- `fetch.timeout` controls which source supplies the argument.

Those results become ordinary typed derived relations.

## The API-level view

A higher-level consumer can answer:

> “How do the timeout options interact?”

It need not understand the source AST or redo the interprocedural reasoning. It reads the already-derived influence and selection relationships and follows their evidence references.

But the system **must not infer that either timeout value contributes to `fetch`’s returned value merely because `transport(...)` is returned**. Without an admitted summary for `transport`, the argument-to-result transfer remains open.

That last distinction is why the graph needs typed transfer semantics. Ordinary connectivity would make it easy to overstate the result.

# 5. Reusable analysis operators are more important than additional graph classes

I would build a small operator framework with several computational families, rather than one universal “graph analyzer.”

## 5.1 Topology operators

These include reachability, SCCs, condensation, dominators, path extraction, and graph aggregation.

Their input is a graph whose meaning has already been established. They should not decide Python semantics themselves.

For example, petgraph provides a dominance implementation over a supplied control-flow graph. It does not establish whether your graph correctly represents Python exceptions or finalization. :chatgpt-content-reference{index="6"}

Your existing SCC wrapper is a good pattern: petgraph owns the algorithm, while your wrapper owns canonical identities and deterministic ordering. 

## 5.2 Transfer and fixed-point operators

These propagate an analysis state through relationships.

For an edge-transfer analysis, the core operation is:

\[
X_v \leftarrow X_v \sqcup T_e(X_u)
\]

Here, \(X_u\) is the current abstract state, \(T_e\) interprets the relationship, and \(\sqcup\) combines information reaching the destination.

The reusable framework owns scheduling, change detection, budgets, and result collection. The analysis defines the state domain, transfer functions, and merge semantics.

Examples of state domains include source-parameter sets, conditional identity flows, possible resource states, and finite configuration alternatives.

MLIR’s dataflow framework illustrates this separation between analysis-specific lattice/transfer logic and a reusable propagation driver. :chatgpt-content-reference{index="8"}

The architectural benefit is substantial: **a new analysis supplies different semantics to the same scheduling machinery instead of implementing another bespoke traversal.**

Your current worklist already separates semantic keys from witness IDs and retains nondominated depth/cost representatives. Preserve that sophistication when generalizing; replacing it with “keep the shortest path” would lose valid analytical alternatives. 

## 5.3 Interprocedural composition operators

These combine call bindings with callee summaries.

They must preserve call-site context. Otherwise, a naïve graph can enter a shared callee through one call and leave through another call’s return connection.

For example:

```python
a = identity(first_value)
b = identity(second_value)
```

A context-insensitive traversal must not conclude that `first_value` supplies `b`.

I would make matched call/return composition and formal-to-actual substitution reusable primitives. They are not details each analysis should implement independently.

IFDS is a useful reference for the subset of analyses expressible with finite dataflow facts and distributive transfer functions. Its graph-reachability formulation is powerful, but those restrictions matter; it is not a universal solution for arbitrary Python semantics. :chatgpt-content-reference{index="10"}

## 5.4 Pattern and rule operators

Some outputs are best expressed as joins or rules:

> A public parameter binds to a callee formal; that formal participates in a predicate; that predicate qualifies a raise site.

Those premises can establish a structural finding without requiring a new general graph algorithm.

I would support such rules over the same typed relations, with their derivations recorded. DataFusion remains appropriate for nonrecursive relational work. For genuinely recursive relational rules, Ascent is a candidate backend because it supports fixed points over user-defined lattices. It should be optional and justified by the rules involved, not required for all analyses. :chatgpt-content-reference{index="11"}

## 5.5 Aggregation and discovery operators

These turn lower-level relationships into API-level summaries, ranked associations, communities, or concept attributes.

They should consume the same normalized relationships but publish a different kind of result. A community membership or frequent usage pattern must not silently become a behavioral guarantee.

Your code already distinguishes optional communities, FCA/RCA, PageRank, and neighbor analyses. I would connect those to the shared model without collapsing their statistical or structural meaning into semantic proof. 

# 6. Make the rules of composition explicit

This is the part that most determines whether the architecture will remain extensible.

A generic graph library understands nodes, edges, and weights. Your system needs to understand **what can legitimately be composed**.

## 6.1 Conditions need their own algebra

Suppose one transfer applies under \(c_1\) and the next under \(c_2\). Sequential composition requires their conjunction.

Alternative routes can contribute a disjunction, but only when they refer to compatible semantic states and retain the association between each route and its evidence.

For a simple reachability interpretation:

\[
C(s,t)=\bigvee_{p:s\leadsto t}\ \bigwedge_{e\in p} C(e)
\]

I would treat that as a condition-composition pattern, not as a declaration of runtime feasibility.

Your existing condition checks explicitly distinguish compatibility under declared atoms from selecting a reaching definition or proving that a modeled call completes. That distinction must survive the generalized architecture. 

In particular, conditions need evaluation identity and call substitution. Two equal-looking predicates evaluated before and after a state-changing call cannot automatically be treated as one stable Boolean variable.

## 6.2 Transfer kinds must compose differently

Identity, computed-value dependence, control influence, mutation, and callback involvement should not be interchangeable edge labels.

For example:

**Identity followed by identity** may preserve identity.

**Identity followed by an admitted computation** may preserve dependence but not identity.

**Value supplied to an unknown call** does not establish dependence of the call’s result.

**Value used in a predicate** establishes a different relationship from value copied into a result.

I would centralize these composition rules in typed transfer modules. Avoid a global rule that “the weakest edge decides everything”; that can lose distinctions between entirely different channels.

## 6.3 Precision belongs to the result contract

A relationship should retain the context in which it was derived: relevant call-context policy, field abstraction, condition domain, external-model set, and treatment of unknown behavior.

This should not become an arbitrary floating-point confidence score.

Also distinguish:

> “This call site has one resolved target.”

from:

> “This target executes on every path.”

Those are different claims with different proof requirements.

## 6.4 Absence requires more than traversal failure

A bounded traversal that finds no path does not establish “never influences.”

Negative conclusions need an explicit coverage obligation: which sources, targets, dynamic accesses, external transfers, and execution channels were considered, and whether that domain is closed under the stated model.

Your existing boundary and coverage machinery is valuable precisely because it already represents many of these limitations. I would reuse it as a shared analysis service rather than introduce a second uncertainty system. 

# 7. Maintain a separate derivation graph

There are actually **three distinct kinds of graphs** in this architecture.

| Graph | Nodes represent | Edges represent |
|---|---|---|
| **Program/semantic graph** | Entities, occurrences, ports, or analysis states | Calls, bindings, transfers, dependencies |
| **Derivation graph** | Facts, derived assertions, rule applications, obligations | Which premises jointly or alternatively support a result |
| **Pipeline dependency graph** | Projection and analysis artifacts | Which computations must precede or invalidate others |

Do not merge them conceptually.

A function calling another function is not the same relationship as an analysis depending on another analysis.

## Why ordinary provenance lists are insufficient

Suppose a conclusion can be established either by:

**A and B**, or by **C and D**.

A flat support set `{A, B, C, D}` loses the distinction between joint premises and alternatives.

I would represent separate derivation records. Each derivation names its rule and required premises; a conclusion can have multiple derivations.

Provenance circuits provide a useful model: conjunction-like nodes combine joint dependencies, while alternative nodes retain different derivations. ProvSQL’s documentation explicitly distinguishes this from flattened lineage, which loses the breakdown into individual witnesses. I would borrow the representation principle, not add ProvSQL merely for this purpose. :chatgpt-content-reference{index="15"}

This enables several useful operations over the same evidence structure: select a compact explanation, identify which missing premise blocks a conclusion, invalidate affected outputs, and compare independent supporting routes.

## Separate semantic results from presentation witnesses

Do not let the number of retained example paths determine whether the underlying result exists.

I would keep three separate concerns:

**Semantic state:** what the analysis established.

**Derivation support:** why that result is justified and which obligations remain.

**Presentation witnesses:** a bounded selection suitable for an agent packet.

Recursive analyses also need care: storing every unfolded proof can grow indefinitely. Use finite derivation records and explicit fixed-point/SCC support, rather than allowing a result to justify itself through an unexplained cycle.

Your current distinction between semantic progress and witness representatives is already aligned with this approach. 

# 8. What the graph objects should look like in Rust

I would keep the physical representation close to your existing adapter.

**The graph object should mostly contain topology, indices, and references to typed records—not a large network of mutable domain objects.**

## 8.1 Shared identity, projection-local indexing

Retain canonical IDs for semantic entities and relationships. Each projection builds its own compact local indexing.

Never assume `NodeIndex(17)` means the same entity in two different projections. The canonical-ID mapping is the bridge.

Share immutable dictionaries and metadata where useful, but do not require every projection to carry every node in the corpus.

## 8.2 Lightweight topology, typed side data

Your existing `Graph<(), u32, Directed, u32>` design is appropriate: the edge weight points to an arc row, while arc metadata lives separately. 

I would generalize that to projection-specific typed rows:

- Invocation arcs retain target resolution and call-site information.
- Value-flow arcs retain transfer semantics and condition references.
- Summary arcs retain port mappings and summary IDs.
- Derivation arcs retain premise roles.

The shared infrastructure should handle indexing, adjacency, validation, and canonicalization. It should not force every projection into one enormous edge struct with many irrelevant optional fields.

## 8.3 Preserve semantic parallelism

Two arcs can have the same endpoints while differing in call site, condition, signature alternative, or evidence. They must remain distinguishable.

For algorithms that need only simple topology, deliberately collapse those arcs and retain a mapping back to the original relationship records.

This matters when selecting a storage backend: petgraph’s `Csr` does not support parallel edges. Therefore, it is suitable only for a deliberately simplified topology or a representation with an explicit parallel-arc mapping—not as a drop-in replacement for the semantic multigraph. :chatgpt-content-reference{index="18"}

## 8.4 Reuse adjacency; do not copy complete graphs for every question

Use masks or filtered views for restricted universes, and reuse incoming/outgoing adjacency where repeated analyses need it.

A concrete optimization opportunity in the current adapter is that `out_arcs` collects and sorts outgoing arcs on each call. A generalized immutable graph service could precompute canonical adjacency once when repeated traversals justify the memory cost. 

That is an implementation optimization, not the reason for the architectural change.

## 8.5 Persist results, not just serialized graph objects

Canonical relationships, derived results, projection manifests, and their evidence should remain durable and queryable. The in-memory graph is rebuildable execution infrastructure.

I would not introduce a graph database for this work. Nor would I precompute every transitive closure: reusable function summaries, selected indexes, and bounded slices are a more targeted starting point.

# 9. Generalize projection and operator contracts

Your existing `ProjectionSpec` already captures much of the right intent. I would extend it to cover multiple input relation families, explicit output row types, and semantic policies beyond invocation-specific fields. Its current digest includes declared fields and queries, which is a good foundation for reproducibility. 

For every projection, I would require a declared node universe, relationship inputs, accepted evidence/profile, parallel-edge policy, unresolved-boundary representation, identity mapping, and schema.

For every operator, I would require:

| Contract element | Purpose |
|---|---|
| **Input artifacts and assumptions** | Prevent hidden dependencies and incompatible compositions |
| **Semantic version and output schema** | Make changes in meaning explicit |
| **State/transfer/merge rules, where applicable** | Define what the computation actually infers |
| **Completion and budget policy** | Distinguish completed analysis from truncation |
| **Derivation and coverage outputs** | Make results explainable and limitations queryable |
| **Determinism policy** | Make results reproducible despite input ordering or parallel execution |

These should extend your existing schema and relation authorities, not create a competing registry.

The pipeline scheduler can then build the required dependency graph. Recursive computation stays inside explicitly declared fixed-point groups; the outer artifact pipeline remains understandable.

For incremental reuse, start with coarse artifact or function/SCC-level invalidation. Include not only present inputs but also relevant domain membership, absent lookups, policies, and model versions. A newly added possible target can invalidate a formerly unique resolution even when none of its old supporting records changed.

**Incremental recomputation is not simply “rerun analyses whose cited positive facts changed.”**

# 10. The first implementation I would actually commission

I would not begin by building every graph layer or a universal analysis language.

I would implement one architectural slice that proves the abstractions across two consumers.

## A. Generalize the existing graph/projection infrastructure

Extract shared handling for canonical identities, immutable topology, typed arc storage, manifests, validation, and deterministic traversal.

Keep invocation-specific semantics in its adapter. Do not make all analyses inherit its node and edge vocabulary.

## B. Add guarded value-flow and boundary-summary projections

Build these directly from the flow and summary relations already present.

The first version should preserve existing semantics and uncertainty, rather than use the refactor as an opportunity to claim broader flow completeness.

## C. Generalize shared semantic computation services

Reuse the existing condition kernel, call-binding logic, SCC schedule, and bounded worklist machinery.

Centralize formal-to-actual substitution, transfer composition, semantic equality, witness management, and boundary propagation.

## D. Deliver two consumers using the same infrastructure

My preferred pair is:

**Parameter/configuration influence:** where an input is copied, transformed, tested, passed, or left unresolved.

**Backward explanatory slicing:** the connected evidence needed to explain a selected return, call argument, option relationship, or restriction.

These exercise different traversal directions and presentation needs while sharing most of the underlying semantic relationships.

The architectural acceptance test is that the second consumer should not need its own interpretation of reaching definitions, call bindings, or condition composition.

## E. Test the abstraction boundaries, not just output examples

The critical controls should include two calls to the same callee, parallel transfers with different guards, unresolved external calls, recursive summaries, incompatible signature alternatives, and shuffled input order.

Also test that adding another witness does not change semantic equality, that presentation limits do not erase an established result, and that adding an unresolved alternative can appropriately weaken a conclusion.

Only after that slice works would I extend the architecture to fuller control dependence, richer field/state analysis, lifecycle state machines, or broader concept-based discovery.

---

## Bottom line

Your proposed parallel-graph model is a useful part of the answer, but the stronger architecture is:

> **One shared semantic model; multiple purpose-specific graph representations; reusable operators with explicit composition rules; and derived results that remain typed, attributable, and consumable by subsequent analyses.**

The most valuable shift is from **“each analysis builds a graph and walks it”** to **“each analysis composes established semantic relationships through a common set of graph and inference operators.”**

That is what will make additional facts genuinely easier to exploit: their meaning is normalized once, their relationships become available through shared projections, and their consequences propagate through reusable analyses without every downstream feature being rewritten.


**The change to commission is a shared semantic projection/operator layer—not a graph rewrite.** Most of the necessary libraries and several of the hardest semantic components are already present. The work is to make them reusable across analyses and to give those reusable representations a coherent persistence and hydration path.

:chatgpt-content-reference{index="25"}[Download the programming-agent handoff](sandbox:/mnt/data/library_context_graph_architecture_handoff.md)

The handoff incorporates the earlier graph review at `5626941`, with PostgreSQL and dependency paths rechecked at `7152f2135a111d29fd8896c0694fd9ed645513dd`. These are architectural recommendations, not implemented or benchmarked changes. 

# 1. The gap to close

The project already has immutable petgraph projections with canonical-ID mappings, guarded flow facts, bounded summaries, condition reasoning, evidence, and deterministic worklists. **The gap is that these capabilities are not consistently exposed as reusable semantic inputs and operators across analyses.** The general projection registry currently registers the invocation projection; richer flow and summary processing uses specialized representations.   

The recommended target is:

**Canonical facts → shared semantic relations → typed graph projections → reusable analysis operators → durable derived relations → API/evidence products.**

An operator may consume graphs and ordinary relations, and its output may become another projection’s input. New provider facts should usually enter through existing semantic relations, rather than require downstream analyses to understand a new provider.

Preserve the existing ownership boundaries:

| Component | Recommended responsibility |
|---|---|
| `cpg-schema` | Semantic contracts, identities, relation schemas, projection definitions |
| `cpg-core` | Loading, orchestration, relational derivation, publication |
| `lctx-analytics` | Pure graph and semantic computation |
| `lctx-postgres` | Import, serving representations, hydration |

The current Stage E implementation already separates projection acquisition from analytics. Build on that separation. 

# 2. Recommended target architecture

## A. A shared semantic vocabulary, not another universal property table

Make **entities, source occurrences, boundary ports, and relationships** explicit and reusable. Distinguish a function from its call sites, a variable binding from its uses, and a function’s input/output ports from the function itself.

Represent call bindings, guarded transfers, summaries, and derivations as typed records with canonical identities. Preserve call-site identity, target/signature alternatives, transfer kind, conditions, evidence, and unresolved obligations.

**Reuse existing tables wherever they already express the relationship.** Do not add duplicate “graph facts” merely to satisfy a common interface.

Multi-part relationships, such as actual-argument-to-formal binding under a particular target and signature, may be exposed as relationship-nodes in an incidence graph or as typed edge records. Their meaning must not be reduced to unqualified endpoint pairs.

## B. Multiple immutable projections over that vocabulary

Initially support **invocation, guarded value flow, function-boundary summaries, and derivation/support** projections. Later control-flow, type-dependency, community, or other projections should use the same infrastructure without becoming prerequisites for the first release.

Generalize shared identity mapping, canonical adjacency, validation, projection manifests, and typed side data. Keep domain-specific payloads in projection-specific types. Preserve parallel arcs: identical endpoints may represent different call sites, guards, or alternatives.

Use edge overlays when entities stay the same, summary graphs when implementation detail is abstracted away, and membership mappings when components become higher-level nodes. Share canonical identities across projections; share dense indices only when they deliberately share the same versioned node dictionary.

**Do not force every relationship through the existing global `nodes`/`edges` catalog.** A first-class typed projection over canonical relations is sufficient. The existing lightweight graph-plus-side-data design is the right starting point. 

## C. Reusable operators with explicit semantic contracts

Separate topology algorithms from semantic interpretation. Reuse common infrastructure for SCC scheduling, guarded propagation, formal-to-actual substitution, matched call/return composition, evidence assembly, and boundary propagation.

Each operator should declare its inputs, semantic assumptions, output contract, version, completion status, and budget policy. Domain-specific operators still own their transfer functions and merge semantics.

Avoid both extremes: another bespoke walker for every feature, or one giant generic interface that hides important differences between analyses.

The existing finite-summary and worklist machinery should supply reusable components, **not be replaced with unrestricted reachability**. Preserve semantic equality independently of witness identity and retain meaningful depth/cost alternatives.  

Centralize the invariants that graph connectivity cannot express: identity transfer differs from derived dependence or control influence; a resolved target does not prove execution; entering through one call must not return through another; compatible Boolean atoms do not by themselves prove runtime feasibility.

## D. First-class summaries and derivations

Treat admitted function summaries as reusable **input-port-to-output-port relationships**, with conditions, exit channels, context, and supporting evidence. Compose these at call sites instead of repeatedly traversing complete callee implementations.

Keep the program graph, derivation graph, and pipeline dependency graph distinct. A derivation must preserve **joint premises versus alternative proofs**: `(A and B) or (C and D)` must not become an undifferentiated set of four supporting facts.

Separate semantic results, derivation support, and bounded presentation witnesses. A smaller response budget must not erase an established semantic result. Negative conclusions additionally require coverage/domain-closure evidence, not merely the absence of a discovered path.

Extend the existing summary, proof, condition, and coverage contracts rather than introducing a parallel evidence authority.  

## E. Artifact-level dependency management before fine-grained incrementality

Give projections and analysis outputs content/dependency manifests. Reuse keys must cover actual inputs and semantics: analyzed environment, source/domain membership, model set, projection policy, operator version, and relevant assumptions. Dependencies on absent lookups and unresolved alternatives matter too.

Start with whole-projection or function/SCC-level recomputation. Keep a path to finer invalidation, but do not make a new incremental engine a prerequisite.

**Separate semantic context from retrieval/presentation settings.** The same graph should serve different queries and ranking settings when its admitted semantics are unchanged. Query-specific assumptions belong in result-cache keys, not automatically in the base-graph key. Existing generation identity may already cover many dependencies; avoid duplicating or inconsistently hashing them.

# 3. PostgreSQL and hydration

## Build on the existing two-part serving design

PostgreSQL already supports generation-pinned selection and set-based hydration. The serving contract also retains Arrow artifacts for conditions, summaries, identities, call bindings, and related native inputs. Therefore, the change is to add graph-ready hydration and reusable artifacts to these paths—not invent persistence from scratch.  

Keep canonical facts and published derived results under the existing Delta/publication contract; PostgreSQL remains a rebuildable serving representation. Extend import, validation, manifests, and reconstruction together when exposing new relations. 

## Representations to make durable and addressable

These describe logical responsibilities, **not mandatory new table names**. Extend the existing inventory where it already covers them.

| Representation | Recommended content and purpose |
|---|---|
| **Projection and analysis manifests** | Generation/context, projection/operator definition, dependency digests, selected node universe, completeness/boundaries, schema/encoding versions, counts, and artifact receipts. Separate semantic identity from physical location and encoding. |
| **Typed semantic relationships and ports** | Selected occurrences, port ownership, guarded transfers, call bindings, source/target IDs, relationship IDs, conditions, and support references. Support access by owner and both endpoint directions. |
| **Function summaries and component membership** | Input/output transfers, exit channels, applicability context, conditions, coverage, and supporting derivations. Preserve function-to-SCC membership and component dependencies where useful for scheduling and scoped loading. |
| **Conditions** | Structured atoms with evaluation identities, condition roots/nodes, and the interpretation/version needed to reconstruct them. Serialized BDD caches must include or reference their variable dictionary and ordering; opaque BDD bytes alone are insufficient. |
| **Derivations and obligations** | Conclusion-to-derivation and derivation-to-premise records, premise roles, alternative support, and open coverage/model obligations. Support both explanation lookup and reverse dependency lookup. |

Persist reusable semantic results and summaries—not every transient worklist state, possible path, or all-pairs transitive closure. Keep large source bodies and rendered evidence off the hot graph-hydration path.

## Use two hydration modes

**Selective relational hydration:** fetch a bounded function, SCC, or requested subgraph through set-based queries into typed Rust inputs. Do not hydrate a semantic graph by repeatedly calling public API packet assembly or issuing one database query per edge.

The current catalog hydration already batches keys and separately expands nested type relations. Add a bulk analytical-input path beside it where needed; do not broadly rewrite the existing packet path without evidence. 

**Bulk artifact hydration:** extend the content-addressed Arrow IPC artifact path with node dictionaries, typed relationship columns, summary/support tables, and optionally precomputed canonical adjacency arrays. PostgreSQL can retain searchable relations plus artifact manifests/locations; large graph payloads do not all need to be database blobs.

Include portable export/reconstruction coverage rather than assuming another process or machine can access a local path. Arrow IPC provides standardized transport, but does not make PostgreSQL decoding or petgraph construction zero-copy. Rebuild topology from validated columns unless measured load costs justify a separately versioned binary adjacency cache. :chatgpt-content-reference{index="14"}

For endpoint lookup, start with generation-qualified indexes matching actual access patterns, such as `(generation, projection, source_id)` and `(generation, projection, target_id)` where those columns apply. Keep relationship identity separate from endpoint pairs so parallel relationships survive. Add owner and premise-reference indexes for summary loading and explanation expansion. Validate actual plans and workload costs rather than indexing every field. :chatgpt-content-reference{index="15"}

## Cache hydrated immutable objects, not database leases

Share validated projections and condition catalogs through `Arc` within a bounded serving-process cache. Fetch inputs, release database capacity, then run CPU analysis under the existing admission/cancellation controls. Existing readers should remain pinned to their generation. 

Account separately for stored bytes, decoded graph memory, concurrent builds, and objects retained by in-flight readers. Eviction does not imply those readers released their memory.

Missing required support must produce an explicit incomplete/refused result. An unavailable or corrupt representation must not silently become an empty graph.

# 4. Libraries to leverage

## Existing dependencies are sufficient for the initial pivot

The checked workspace already pins the following core dependencies. No family-wide version upgrade is required merely to implement this architecture. 

| Library | Recommended role |
|---|---|
| **`petgraph = 0.8.3`** | Default immutable topology, SCCs, traversal, and other appropriate topology algorithms. Retain lightweight weights identifying typed relationship rows. |
| **`biodivine-lib-bdd = 0.6.3`** | Condition combination and restriction through the existing bounded condition kernel, not direct ad hoc use by every operator. |
| **`fixedbitset = 0.5.7`** | Dense projection masks, visited/source sets, and finite set-valued analysis states where appropriate. |
| **Arrow `59.3.0`, DataFusion `55.1.0`, existing Delta integration** | Typed relation interchange, relational derivation, IPC artifacts, and canonical publication. Keep relational joins relational; use graphs for topology and propagation. |
| **`sqlx = 0.9.0` and existing bulk import** | Generation-qualified reads, import, and hydration. Reuse pooling, admission, schema, and reconstruction controls. |
| **Serde, Schemars, existing ID/hash infrastructure** | Shared typed contracts, boundary schemas, reproducible manifests, and explicit semantic/physical identities. |

Petgraph’s `Graph` preserves parallel edges; its `Csr` does not. Use `Csr` only for an intentionally simplified topology with retained mappings—not as a drop-in semantic multigraph. The BDD library supports serialization and logical operations, while FixedBitSet supplies set operations; the repository’s semantic wrappers should still control their use. :chatgpt-content-reference{index="18"}

Keep existing Leiden/FCA-related machinery as optional discovery consumers. Community membership or similarity must not silently become a certified behavioral relationship. The analytics configuration already separates these optional techniques. 

## Optional additions, with specific adoption triggers

**Moka** is the most directly relevant addition if shared hydrated-object caching needs concurrent initialization and eviction. It supports weighted capacity and coalesced initialization for the same key. Use `Arc` values and retain independent hard memory/admission accounting: capacity enforcement is best-effort, not a strict process-memory ceiling. :chatgpt-content-reference{index="20"}

**`rustworkx-core` 0.18.x** is an algorithm extension when a named analysis needs functionality beyond petgraph. It builds on petgraph; verify exact type/dependency compatibility and feature costs before pinning. It supplies algorithms, not the semantic operator or provenance architecture. :chatgpt-content-reference{index="21"}

**Ascent** is worth considering only when a substantial set of recursive relational rules is clearer as Datalog/lattice computation. Its support for user-defined lattices makes it relevant, but the existing typed Rust worklist should remain the default until there is a concrete consumer. Ascent must not become a second owner of call or condition semantics. :chatgpt-content-reference{index="22"}

**No new graph database, general-purpose hypergraph library, or production-wide incremental engine is required.** Salsa’s presence in the analyzer dependencies does not make it the appropriate owner of application-level graph caching.

# 5. Code review and change map

| Area | Where to review | Proposed change |
|---|---|---|
| **Projection contracts and topology** | `cpg-schema/src/projection.rs`; `lctx-analytics/src/graph.rs` | Generalize invocation-specific infrastructure into typed projection contracts/adapters; retain deterministic identity and multigraph behavior. |
| **Flow and composition** | `cpg-core/src/flow_model.rs`, `summaries.rs`; `lctx-analytics/src/summaries.rs`, `summaries/finite.rs`, `summaries/worklist.rs`, `call_binding.rs`, `source_call.rs` | Expose reusable transfer/summary inputs and factor shared scheduling/composition services without weakening admission rules. |
| **Conditions and evidence** | `cpg-schema/src/condition_kernel.rs`, `condition_kernel/substitution.rs`, `summary_contract.rs`, `behavior.rs`, `evidence.rs` | Reuse existing semantics for guarded composition, support alternatives, coverage, and reconstruction. |
| **Pipeline and consumers** | `cpg-core/src/analyze.rs`, `derive.rs`, `catalog.rs`; `lctx-analytics/src/pass_a.rs`, `pass_b.rs`, `pass_c.rs` | Declare dependencies and migrate selected consumers to shared projections/operators rather than creating another inference path. |
| **Serving and portable artifacts** | `cpg-schema/src/serving_projection.rs`, `bundle.rs`; `lctx-postgres/src/projection.rs`, `import.rs`, `repository.rs`, `hydration.rs`; migrations | Extend generation-scoped storage, typed bulk hydration, manifests, and parity/reconstruction checks together. |

*Paths above are relative to `crates/`.*

Do not assume DataFusion-over-PostgreSQL is already a general graph-loading route: `cpg-core/src/postgres_read.rs` admits a deliberately restricted set of report views and expressions. Keep graph hydration in the explicit serving boundary unless expanding that provider is separately justified and tested. 

# 6. First delivery and acceptance

Deliver the shared projection layer with **guarded value-flow and function-summary views**, then use it for both **parameter/configuration influence** and **backward explanatory slicing**.

The second consumer is the architectural test: it should not reimplement reaching-definition interpretation, call substitution, condition composition, or support assembly.

Require semantic/evidence parity between compile-time inputs, cold PostgreSQL/artifact hydration, and warm cached execution. Exercise separate calls to a shared callee, parallel guarded transfers, incompatible signature alternatives, unresolved external calls, recursion, shuffled input order, and incomplete support. Preserve existing admitted results and refusals unless an explicit semantic change is separately reviewed.

Measure cold/warm hydration, projection-build time, query work, peak memory, and transferred bytes. Demonstrate safe reuse across different retrieval contexts and explicit invalidation when semantic context changes.

**Success is a new analysis consuming reusable semantic products—not merely more facts being copied into petgraph.**