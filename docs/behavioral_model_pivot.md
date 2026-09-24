**Yes. I would evolve `library-context` into an evidence-carrying semantic query engine, rather than a brief generator with a more sophisticated retrieval layer.**

The central architectural change is:

> **Code facts → conditional behavioral relations → executable capability concepts → structured queries and semantic discovery.**

Embeddings, community detection, and RCA belong in that architecture, but they should operate on and help organize the structured knowledge. They should not determine whether a capability exists.

Your intuition about needing an ontology is directionally right. My main adjustment is that **the ontology should be an executable, compositional model of capabilities, grounded in program analysis**, rather than a large hierarchy of descriptive labels attached to code.

I reviewed the design and key extraction, analysis, and embedding files at commit `0e98d85cfbe4ee7c8280cd52bfc2ebe24a0fa4dc`. This is a source-and-design review; I did not execute the repository’s test suite. 

## 1. What your repository already has, and where the current architecture constrains you

You have substantially more than a basic code-indexing pipeline. Several existing foundations are exactly what I would retain.

| Area | What I found | Implication for the redesign |
|---|---|---|
| Extraction and types | Pyrefly and Ruff run in-process over one parse. Type extraction preserves structural terms and their children, rather than reducing types to display strings. | Retain the extraction architecture and structural type model. These are valuable inputs to executable semantic queries. |
| Provenance and projections | Facts retain origin, modality, fidelity, and evidence. Graph projections preserve call-site identity, candidate targets, unresolved boundaries, and separate canonical IDs from graph indices. | Build the intelligence layer on this machinery rather than introducing another authoritative graph store. |
| Behavioral analysis | Passes A–C already recognize delegation, restricted parameter forwarding and guards, and direct handoffs in official usage code. | Generalize these into reusable behavioral relations. They are useful initial analyses, but not yet a general behavioral model. |
| Embeddings | The analytics API view is currently `path(parameters)` plus the docstring. The serving product remains searchable briefs. | The change is not simply “use code embeddings instead.” You need new source- and fact-level retrieval units, independently addressable by queries. |
| FCA/RCA and serving | FCA primarily organizes signature-related attributes. RCA adds one-step attributes such as `calls X`. Agent serving excludes query-time traversal and exposes precomputed briefs. | Both the analysis representation and the agent-facing query contract need to expand. |

Those findings are visible in the extraction implementation, projection specification, analysis passes, and embedding code.     

Two distinctions matter here.

**First, your existing briefs are already programmatic, not LLM-generated.** The problem is therefore not primarily nondeterministic summarization. It is that a small, selected set of textual outputs constrains the information agents can access and combine. Your design explicitly excludes a domain-capability ontology beyond briefs, general Python CFG/dataflow/alias analysis, and query-time graph traversal. Those are deliberate scope decisions that your new objective now supersedes.  

**Second, the raw store is already queryable through `lctx query`.** What is missing is not the ability to run SQL over facts, but an agent-facing semantic query layer that turns those facts into composable answers about behavior. 

The recent ablation also deserves reinterpretation. Communities, FCA, RCA, and related techniques were evaluated largely by their effects on published briefs and brief-retrieval metrics. Several are now off by default. That does **not** establish that they would be useful for your new objective, but neither does it establish that they are unhelpful for structural discovery, faceted queries, or relational explanations. Those are different consumers and require different evaluations. 

**My recommendation is to retain briefs as an optional presentation format, but remove them as the gateway to the library’s knowledge.**

## 2. The architecture I would build

I would organize the system as follows:

```text
Pinned library source, dependencies, configuration, official usage
                              │
                              ▼
                    Existing typed CPG facts
                  Arrow / DataFusion / Delta
                              │
                              ▼
                   Semantic analysis overlays
          Value flow · guards · effects · protocols · contracts
                              │
                              ▼
                   Conditional behavior model
           Inputs · outputs · controls · requirements · evidence
                              │
                              ▼
                   Executable capability model
             Typed predicates and compositional concepts
                              │
                ┌─────────────┼──────────────┐
                ▼             ▼              ▼
           FCA / RCA     Graph analysis   Code/fact embeddings
           concepts      and communities  and lexical indexes
                └─────────────┼──────────────┘
                              ▼
                    Typed query executor
          Exact matches · discovery · paths · facets · explanations
                              │
                              ▼
                  Evidence-backed agent responses
                     Optional rendered briefs
```

There should be **one authoritative body of facts and derived assertions**, with multiple rebuildable analytical and retrieval projections.

The most important new object is not a “better document.” It is something like a **conditional behavior record**:

```text
Operation behavior
    subject operation
    input and output roles
    transformations and value relationships
    configuration predicate
    effects and lifecycle transitions
    required capabilities of supplied objects
    assumptions and unresolved boundaries
    derivation and source evidence
```

An operation can have several such records, corresponding to different modes, input types, configurations, or execution phases.

That is what enables questions such as:

> Which public operations accept a caller-provided destination, expose a compression choice, and pass the resulting payload to that destination under the same configuration?

This question should be answerable by joining behavioral relations. It should not require discovering whether a brief happens to mention all three properties.

## 3. The highest-priority addition: semantic program analysis

### A CPG is the substrate, not the complete behavioral interpretation

Your design correctly states that Pyrefly’s inference graph must not be relabeled as runtime dataflow. That distinction becomes central in the redesigned system. 

A resolved call, a type observation, an AST branch, and an actual runtime effect describe different things. CodeQL makes a similar distinction between source structure, value flow, and global analyses, with explicit modeling needed where implementation bodies or precise dispatch information are unavailable. :chatgpt-content-reference{index="11"}

I would add semantic analyses in three stages.

### 3.1 Local behavioral relations

Start with facts whose interpretation can be established within a function or a tightly bounded region:

- Parameter-to-expression and expression-to-return flow.
- Argument-to-formal mapping, including the distinctions between forwarding, transformation, and replacement by a constant.
- Reads and writes through modeled object access paths.
- Branch predicates and the operations they control.
- Normal versus exceptional exits.
- Callback storage, invocation, and forwarding.
- Object construction and resource acquisition/release.

Your existing Pass B is a good starting point, but its current restricted recognizers should become producers of reusable relations rather than principally producers of sentences. The repository already records cases it declines to follow, which is an important behavior to preserve. 

The corresponding implementation work is a CFG overlay, definition-use relations, bounded alias/access-path analysis, and explicit treatment of exceptions and execution phases.

### 3.2 Interprocedural transfer models

Next, compute how a callable transforms its inputs into outputs and effects, including what it delegates to other callables.

Static-analysis literature often calls these **function summaries**. Here, “summary” means a formal transfer relation such as:

```text
input.parameter_0 → returned_object.field_x
input.parameter_1 → downstream_call.parameter_y
configuration.flag == true → effect_z
```

It does **not** mean an English synopsis or an agent brief.

CodeQL’s library models are a useful concrete precedent: they represent input/output access paths, distinguish value-preserving from taint-preserving flow, and support explicit models for library boundaries. Its Python data-extension format is currently marked beta, so I would borrow the modeling approach rather than couple your architecture to that format. :chatgpt-content-reference{index="13"}

For your Rust implementation, I would compute these relations over call-graph strongly connected components, using monotone worklists and an explicitly bounded abstraction. Recursive functions then have defined fixed-point behavior rather than depending on an arbitrary traversal depth.

**Do not propagate capabilities merely because one function calls another.**

A wrapper might hard-code a callee’s configuration, discard its output, catch its exception, or expose only one branch. Capability propagation needs argument substitution, control conditions, return flow, and effect handling.

For example:

> “This wrapper calls a configurable serializer” does not imply “this wrapper exposes configurable serialization.”

The analysis must establish which controls remain available to the caller.

### 3.3 Protocol and lifecycle analysis

Many useful library capabilities concern sequences and relationships, not individual methods:

> Construct a resource, configure it, enter a context, consume results, and close it.

I would represent these as state transitions and usage constraints, with evidence distinguishing an enforced protocol from an observed usage pattern.

There is relevant research on mining API usage scenarios as partial orders from source, and on combining mined multi-object protocols with static checking. Those approaches are particularly relevant to extending your existing direct-handoff analysis beyond adjacent producer/consumer patterns. They do not justify treating every observed sequence as a mandatory protocol. :chatgpt-content-reference{index="14"}

### Native and external implementations need explicit treatment

Your current scope excludes native-extension bodies. That boundary remains important regardless of how deep the Python extraction becomes. 

I would use a combination of automatically derived source-level transfer models, small versioned contracts for important external primitives, and explicit unknown results where neither is available. Later, another language frontend could supply deeper facts for selected native dependencies.

The goal should be **fully programmatic inference under explicit semantic models**, not an assumption that all implementation semantics are recoverable from Python wrappers.

## 4. Build a detailed but compositional, executable ontology

I would separate three layers.

**The code ontology** is largely what you already have: callables, parameters, types, bindings, references, call sites, and source artifacts.

**The behavioral ontology** describes what code does: consumes, produces, transforms, forwards, validates, invokes, mutates, acquires, releases, and conditionally raises.

**The capability ontology** composes those behaviors into concepts relevant to developers: configurable serialization, callback-based extension, schema validation, backend selection, incremental consumption, and so forth.

The distinction is that a capability concept should have **machine-executable membership criteria**, not merely a label and an embedding.

### Model facets rather than enumerating every combination

A useful capability model would include the following dimensions:

| Facet | Example distinctions |
|---|---|
| Operation | Parse, serialize, register, transform, validate, query |
| Data roles | Consumed value, produced value, schema, format, destination |
| Configuration | Exposed control, fixed default, forwarded control, mode-dependent control |
| Execution | Immediate call, coroutine execution, iteration, callback invocation |
| Effects and lifecycle | Mutation, I/O, allocation, acquisition, release, exception propagation |
| Extension and composition | Callback, protocol implementation, subclass hook, backend injection, direct handoff |

This gives you detailed descriptions through composition, rather than requiring a separately authored class for every combination.

For example, “write JSON with gzip to a caller-provided sink” should resolve to a conjunction involving a writing operation, a format choice, a compression stage, and the origin of the sink.

But the conjunction must preserve relationships:

**It must be the compressed JSON payload that reaches that sink, under one compatible configuration.**

Finding a JSON-related call somewhere, a compression-related call elsewhere, and a sink parameter somewhere else is insufficient.

### Make configuration part of capability identity

I would avoid a flat relation such as:

```text
api HAS_CAPABILITY streaming
```

as the principal model.

Prefer a relation shaped like:

```text
capability_occurrence(
    operation,
    concept,
    configuration_condition,
    behavior_context,
    evidence
)
```

with additional typed relations for input, output, control, and resource roles.

This prevents a subtle but serious error: combining properties that only occur in mutually exclusive modes.

An API might support streaming in one mode and a particular transformation in another. Independent Boolean tags could incorrectly make it appear to support both together.

### Domain meanings still need anchors

There is an unavoidable distinction between discovering a structural pattern and assigning a domain meaning to it.

The system can discover that several operations transform input and send the result to a supplied object. Calling that pattern “serialization” requires a definition grounded in known operations, formats, contracts, or other semantic evidence.

I would therefore maintain a modest, versioned catalog of **semantic primitives and executable definitions**, not handwritten capability descriptions for every function. Their consequences would be propagated and combined automatically.

For vocabulary management, SKOS provides useful distinctions among preferred labels, alternatives, broader concepts, and related concepts. Those vocabulary relationships should remain separate from the executable rules that establish membership. :chatgpt-content-reference{index="16"}

There is no need to migrate the canonical store to RDF merely to gain these ideas. The definitions can live in typed Rust structures and Arrow relations, with query compilation into your existing execution stack.

## 5. How FCA, RCA, Graph-FCA, and communities should fit

These methods answer different questions. I would not make them interchangeable stages in one clustering pipeline.

### FCA: organize shared, explicit properties

FCA is useful for deriving concepts from an object–attribute context. In your system, the objects might be public operations, and the attributes should increasingly be **behavioral predicates**, not just parameter names and type labels.

Your current implementation already has a context and implication machinery, but its documented attributes are predominantly signature-related. That naturally produces shared-signature concepts rather than a rich capability model.  

I would replace string-only feature identity with typed attributes carrying their predicate, arguments, applicability conditions, evidence, and modality.

Two safeguards are essential.

An FCA implication is a regularity in the chosen context, not automatically a universal law about software behavior. Also, an absent extracted attribute must not silently become a claim that the runtime property is false.

Finally, **frequent concepts must not define the searchable universe**. A unique operation can be the most valuable capability in a library even when it falls below a concept-mining support threshold.

### RCA: discover concepts through relationships to other concepts

The repository’s current RCA step adds attributes such as `calls X`, where `X` is a particular target named by its preferred path. That is a useful relational-feature step, but it does not yet perform multi-context, concept-target iteration. 

The richer RCA approach would maintain contexts for several object categories, such as operations, parameters, types, resources, and behavior records. Relations connect those categories. Attributes can then describe relationships to **classes of targets**, rather than individual named targets.

For example:

```text
Current form:
    calls particular_function_X

Richer relational description:
    calls some operation classified as a schema validator

Further description:
    accepts a parameter whose type supports a modeled output protocol
```

RCA literature explicitly works with multiple contexts and relational scaling, and *On-demand Relational Concept Analysis* explores computing the relevant concepts without requiring exhaustive construction up front. :chatgpt-content-reference{index="20"}

For your use case, I would begin with bounded, demand-driven relational descriptions. Preserve the distinction between a possible call target and a behavior established under the analysis model.

The fixed-point choice also needs to be explicit. Euzenat’s 2025 treatment shows why circular relational dependencies require careful semantics rather than an assumption that there is one self-evident induced ontology. :chatgpt-content-reference{index="21"}

### Graph-FCA: particularly relevant to executable query patterns

**Graph-FCA is an additional method I would investigate seriously.**

It generalizes the context to graph relations, and its concept descriptions can be projected graph patterns corresponding to conjunctive queries. This allows a concept to retain relationships among multiple objects and shared variables. :chatgpt-content-reference{index="22"}

That matters for code questions such as:

> Find operations that acquire a resource and pass that same resource to a consumer.

Independent attributes like “acquires a resource” and “calls a consumer” discard the same-object condition. A graph pattern can preserve it.

I would treat Graph-FCA initially as an experimental **query-pattern discovery mechanism** over semantic projections, not as a mandatory replacement for your storage or query engine. Its published implementation is a research tool, not a drop-in Rust component. :chatgpt-content-reference{index="23"}

### Community detection: architecture and navigation, not capability truth

Your existing community implementation already pays attention to projection semantics, normalization, seeds, and stability. I would reuse that discipline. 

I would give communities new consumers: subsystem navigation, related-operation discovery, analysis prioritization, and scoped exploration.

Keep invocation, type-sharing, official co-use, and embedding-similarity layers distinguishable. A community formed mainly by shared infrastructure is not necessarily a capability family.

**Community membership should neither establish a behavioral property nor limit which operations an exact query can find.**

## 6. Embed actual code and facts through multiple views

I agree with changing the embedding inputs. I would not, however, replace a brief with one enormous serialized CPG neighborhood.

Instead, create several independently addressable retrieval views.

| View | Input | Purpose |
|---|---|---|
| Source implementation | Actual function bodies and structurally bounded source spans | Match implementation intent and mechanisms |
| Signature and type structure | Canonical signatures, structural types, parameter roles | Find interface-compatible operations |
| Semantic facts | Canonical, role-labeled behavioral relations and conditions | Find specific controls, effects, transformations, and relationships |
| Evidence slices | Relevant source slices associated with an output, guard, effect, or callback | Retrieve the mechanism supporting a particular property |

Documentation and official examples should remain independent evidence channels. Avoid making them the only bridge between a query and the implementation.

### A fact embedding is not automatically a graph embedding

A text model can consume a canonical representation such as:

```text
operation: package.write_records
control: compression
condition: compression == "gzip"
flow: encoded_payload -> compression_input
flow: compressed_payload -> caller_supplied_sink.write.argument_0
```

That is a useful baseline, and it is not a prose brief. But serializing facts does not guarantee that the embedding preserves graph direction, variable identity, negation, or path conditions. Those must remain available to the exact query executor.

GraphCodeBERT provides a relevant research precedent because it explicitly incorporates dataflow and trains code–structure alignment. It is evidence for the value of structure-aware representations, not evidence that an arbitrary CPG serialization will work equivalently. :chatgpt-content-reference{index="25"}

I would retain your existing Qwen3-Embedding-8B deployment as the first baseline while changing retrieval units and evaluation. Its official model card includes code retrieval among its supported tasks. I would not assume that changing models is the highest-leverage intervention before testing actual code and fact views. :chatgpt-content-reference{index="26"}

### Keep views and evidence separate

Each vector should retain its entity ID, source spans or fact IDs, view specification, input digest, model specification, and snapshot.

Do not collapse code, signature, facts, and documentation into an unexplained average vector. Separate views let you measure which representation helps which query family.

Your existing embedding-spec and input-hash cache is a good foundation for this. Extend its identity to include the view-construction specification. 

Most importantly:

> **Embeddings may nominate or rank candidates. They must not allow a candidate to bypass an unsatisfied structural requirement.**

## 7. Make the agent interface a semantic query API

I would expose a compact query algebra rather than either raw graph storage details or an ever-growing collection of bespoke tools.

The core operations should cover entity selection, typed predicates, relationship joins, bounded paths, configuration conditions, aggregates and facets, and explanations.

An illustrative request could look like this:

```json
{
  "snapshot_id": "...",
  "find": "public_operations",
  "where": {
    "capability": "serialization.write",
    "configuration": {
      "format": "json",
      "compression": "gzip"
    },
    "destination_origin": "caller_parameter"
  },
  "require": {
    "compatible_behavior_context": true,
    "evidence_policy": "established_under_model"
  },
  "return": [
    "public_access_paths",
    "required_arguments",
    "applicability_conditions",
    "source_evidence",
    "derivation",
    "unresolved_boundaries"
  ]
}
```

This is a proposed interface, not an existing repository API. Its important feature is that the ontology terms compile into explicit predicates and joins.

### Separate language interpretation from query execution

The calling programming agent can translate a natural-language task into this representation using a discoverable predicate and concept catalog.

The server can provide lexical and embedding-based concept lookup without requiring a generative model in its execution path. Ambiguity should remain visible: “streaming” might mean returning an iterator, incrementally reading input, incrementally emitting output, or bounded-memory processing. Those should not silently become one predicate.

Once the interpretation is selected, the query should be mechanically executable.

### Support both exhaustive and ranked queries

An exact structured query should search the full eligible entity universe. It must not begin with a vector top-k shortlist and then claim completeness.

Semantic discovery can retrieve and validate a ranked candidate set, but its response should say that it is ranked discovery rather than exhaustive enumeration.

This separation gives you both rich natural-language access and reliable structured querying.

### Return explanations with explicit claim strength

Every result should distinguish:

**What was established:** an extracted fact, a rule-derived relation, an observed test behavior, or a statistical association.

**Under what conditions:** configuration, dependency models, dispatch assumptions, execution phase, and analysis boundaries.

**How complete the answer is:** whether unresolved calls, unsupported expressions, or bounded analysis affect the requested property.

A proof of a database derivation is not automatically a proof of unrestricted runtime behavior. Soufflé’s provenance machinery is a useful implementation reference for explaining derived tuples through rule applications and supporting facts. :chatgpt-content-reference{index="28"}

The closest research match to your overall objective is *Semantic Code Browsing*. It searches for code using properties inferred by abstract interpretation and distinguishes checked, false, and unresolved conditions. Its prototype targets a different language ecosystem, but the conceptual separation between semantic specifications, analysis results, and query checking is highly relevant. :chatgpt-content-reference{index="29"}

## 8. How I would implement this without replacing your stack

I would preserve the existing division of responsibilities and extend it deliberately.

**Arrow, DataFusion, and Delta** remain the contracts, relational execution layer, and authoritative published store. Use DataFusion for normalization, large joins, predicate materialization, and validation.

**Petgraph-based projections** remain the topology layer for call-graph components, paths, slicing support, and structural analyses. Continue preserving relation kinds and evidence rather than flattening everything into one graph.

**Semantic fixed-point evaluation** is the genuinely new computation layer. For small, specialized analyses, your existing Rust worklist style may be the clearest implementation. For repeated relational inference patterns, Ascent is worth a focused evaluation: it embeds Datalog-style rules and fixed-point computation in Rust. It is an option for reducing repeated inference-engine code, not a reason to move ordinary joins out of DataFusion. :chatgpt-content-reference{index="30"}

**The ontology registry** should declare predicate signatures, argument roles, execution dependencies, evidence policies, and definitions. Initially, this can be ordinary typed Rust configuration rather than a new general-purpose language.

**The query executor** should read immutable published generations. You do not need to give serving unrestricted access to the compiler or replace immutable bundles with a mutable database service. A richer generation can contain queryable Arrow relations, adjacency projections, indexes, and evidence references.

The existing files-only brief server would become a thin interface to that executor. This changes what a generation contains and what serving may compute, while preserving your snapshot and publication discipline.  

### The implementation order I recommend

**First: remove the brief bottleneck.** Make all in-scope public operations and their existing facts available through structured queries. Expose current delegation, forwarding, restriction, and handoff relations directly. Add source-code retrieval views. Neither a seed budget nor a concept-support threshold should determine whether an operation is discoverable.

**Second: introduce the behavioral intermediate representation.** Convert the existing passes into reusable conditional relations, then extend local value flow, guard representation, exception handling, and parameter-effect tracking. Define a small set of external semantic contracts for the pilot’s important boundaries.

**Third: add executable capability definitions.** Choose several query families with clear expected answers and implement their concepts as predicate compositions. Include difficult cases where a parameter exists but is ineffective, a capability is conditional, or two features cannot be used together.

**Fourth: expand relational discovery.** Apply FCA to behavioral attributes, implement bounded or on-demand RCA over multiple contexts, and expose concepts as queryable descriptors. Reintroduce community projections where they help exploration. Test Graph-FCA on a limited semantic projection.

**Fifth: deepen analyses according to measured gaps.** Add more precise aliasing, lifecycle models, callback protocols, or structure-aware learned representations only where existing queries demonstrate the need.

This sequence delivers useful intelligence before requiring a comprehensive ontology or a complete Python abstract interpreter.

## 9. Evaluate intelligence, not whether an algorithm changes a brief

The new evaluation should be organized around questions the system must answer correctly.

I would include structural lookup, control propagation, conditional behavior, producer/consumer relationships, lifecycle requirements, semantic discovery, and honest negative or unknown answers.

Particularly valuable test cases include:

- An option that is accepted but never used.
- A callee capability that a wrapper disables.
- Two requested features that exist only in incompatible modes.
- A resource that is acquired and a different resource that is released.
- A rare public API with no popular usage or community membership.
- An unresolved native or dynamic boundary that prevents a negative claim.

These cases test whether the system has retained the information needed for reasoning, rather than merely retrieved plausible code.

For embeddings and graph analytics, compare marginal improvements on the relevant task: candidate recall, predicate correctness, useful facet discovery, explanation completeness, and query latency. Keep an independent held-out set of queries and code cases.

For repeatability, record the compiler, rules, ontology, projection, embedding-view specifications, seeds, and budgets. Distinguish exact replay from cached vectors from numerical reproducibility when recomputing model outputs.

**Determinism should be an execution property; correctness and coverage should be separately measured properties.**

## The research I would prioritize

The most useful sources are spread across program analysis, concept analysis, and code representation learning rather than one established end-to-end framework.

| Resource | What I would take from it |
|---|---|
| **Semantic Code Browsing** | The overall model of searching by inferred semantic properties and returning checked, refuted, or unresolved conditions. :chatgpt-content-reference{index="33"} |
| **CodeQL’s Python dataflow, API graphs, and library models** | Executable relationships over code, stable API references through aliases and local flow, and explicit contracts across library boundaries. :chatgpt-content-reference{index="34"} |
| **Joern’s CPG architecture** | Semantic overlays and queryable code relationships. I would borrow these ideas, not replace your frontend. :chatgpt-content-reference{index="35"} |
| **On-demand RCA and the 2025 fixed-point treatment** | Multi-context relational concepts, controlled exploration, and explicit semantics for circular dependencies. :chatgpt-content-reference{index="36"} |
| **Graph-FCA** | Concepts that are executable graph patterns, especially where shared-object and multi-role constraints matter. :chatgpt-content-reference{index="37"} |
| **API protocol and usage-pattern mining** | Moving beyond isolated functions to supported sequences and relationships among objects. :chatgpt-content-reference{index="38"} |
| **GraphCodeBERT** | Evidence that explicitly incorporating program structure can improve learned code representations, while keeping learned similarity separate from exact semantics. :chatgpt-content-reference{index="39"} |

### Bottom line

Your extraction pipeline gives you a strong basis for this pivot. I would not start by designing a giant ontology or training a graph embedding model.

I would first build **a conditional behavioral model and an evidence-carrying query executor**. Then define capabilities as executable compositions over that model, use FCA/RCA to organize and discover descriptions, use communities to support navigation, and use source- and fact-level embeddings to connect task language to the right entities.

That changes the product from:

> “Find a prewritten interpretation that sounds relevant.”

to:

> **“Find the operations that satisfy these requirements, show the configuration and relationships that make them applicable, and expose exactly what evidence and uncertainty support the answer.”**

