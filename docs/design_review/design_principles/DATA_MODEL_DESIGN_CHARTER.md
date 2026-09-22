# Data Model–Based Design Charter

**Version 1.0 · September 13, 2026**  
**Purpose:** A technology-neutral design and review framework for systems that make explicit data models the source of structure, behavior, constraints, and execution intent.

> **Make meaning authoritative and explicit; derive implementation mechanisms from that meaning; preserve the contract across every representation and lifecycle stage.**

## 0. How to use this charter

Read the governing objective and the scope of the proposed change first. Consult the relevant principle groups, then use the review rubric and companion review template. The stable IDs are intended for architecture decisions, agent instructions, issue descriptions, code reviews, and regression checks.

This is a normative design proposal, not a claim that any storage format, query engine, programming language, or framework supplies these guarantees automatically. The principles apply at different scales: a small library can satisfy them without building a compiler platform or deploying multiple services.

**MUST** means an applicable required contract. A design must not claim alignment while violating it. When an indispensable requirement cannot yet be met, narrow the supported scope or mark the design incomplete against that requirement. **SHOULD** is a strong default; deviations require an explicit rationale and a safer or more proportionate alternative. Examples of machinery are conditional options, not mandatory architecture.

A principle can be not applicable only for a stated reason tied to scope. An unimplemented requirement is not the same as an irrelevant principle. Distinguish design-time claims from implemented and tested guarantees.

### The governing objective

Optimize for **semantic explicitness, enforceable correctness, consistent authority, reusable derivations, local changes, and explainable execution**, subject to the system’s actual performance, operational, and maintenance requirements.

Minimize the number of independently maintained semantic decisions. Do not maximize the number of tables, eliminate every handwritten function, or minimize line count at the expense of meaning. A specialized algorithm behind a complete contract is aligned; a declarative-looking wrapper around hidden business rules is not.

### What “data model–based” means

| Concept | Required interpretation |
|---|---|
| Semantic model | The explicit concepts, types, relationships, constraints, operations, and policies that define what the system means. |
| Schema | A machine-readable contract for a representation, supplemented by semantic and relational invariants where a physical schema alone is insufficient. |
| Canonical authority | The designated source for a fact within a declared scope and revision; not necessarily one universal file, table, service, or physical layout. |
| Declaration | A statement of structure, requirement, relationship, or permitted behavior, independent of incidental construction sequence. |
| Binding/specification | A selection or assignment that applies a reusable definition to a particular context or case. |
| Derived representation | A view, plan, graph, index, layout, or artifact produced from declared inputs through a versioned transformation. |
| Execution state | Mutable or immutable state belonging to an identified attempt, with explicit ownership and publication rules. |
| Evidence | Observations, tests, derivations, measurements, or verified contracts supporting a design claim. |

The aim is one coherent **logical substrate** and authority model, with as many physical representations as justified. Observations and external effects are not magically derivable from declarations: they must enter as explicit inputs or recorded outcomes.

### Five design questions to answer before choosing technology

1. What meanings and distinctions must survive every representation?
2. Which facts are authoritative, which are bindings, and which are derived or observed?
3. Which invariants and effects govern valid changes and executions?
4. Which transformations create execution representations, and under what assumptions?
5. Which ordinary changes should become declarations rather than coordinated procedural edits?

## Principle index

| Topic | Principle IDs |
|---|---|
| 1. Semantic authority and the modeling boundary | DM-01–DM-05 |
| 2. Semantic types, schemas, and invariants | DM-06–DM-10 |
| 3. Identity, versions, and consistency | DM-11–DM-15 |
| 4. Declarative composition and reusable structure | DM-16–DM-20 |
| 5. Compilation, derivation, and semantic preservation | DM-21–DM-25 |
| 6. Planning, execution, mutable state, and effects | DM-26–DM-30 |
| 7. Dependencies, incrementality, and concurrency | DM-31–DM-35 |
| 8. Execution representations and performance | DM-36–DM-40 |
| 9. Boundaries, providers, and extensibility | DM-41–DM-45 |
| 10. Provenance, reproducibility, and explainability | DM-46–DM-50 |
| 11. Evolution, generation, and verification | DM-51–DM-55 |
| 12. Architectural leverage and disciplined improvement | DM-56–DM-60 |

# Principles

## 1. Semantic authority and the modeling boundary

### DM-01 — Make meaning—not storage shape—the primary model

**MUST.** Define the concepts, distinctions, relationships, obligations, and permitted operations that give the system meaning before selecting their physical representation. A field list is not a complete model: the interpretation of those fields and the rules governing their combinations belong to the contract. Preserve distinctions that affect correctness, even when their values share the same primitive representation.

**Assessment test:** Can a reviewer determine what each important fact means, which operations are valid, and what constitutes an invalid state without reverse-engineering implementation code?

**Warning sign:** A richly structured table whose important interpretation exists only in comments, naming conventions, or application branches.

### DM-02 — Assign one authority to each semantic fact and revision

**MUST.** Identify one authoritative definition for every semantic fact within a declared scope and revision. Other representations must be derived, reconciled through an explicit authority protocol, or declared independent observations. Do not let caches, adapters, runtime objects, and configuration files become independently editable definitions of the same fact.

**Assessment test:** For any disputed value or rule, can the system identify the authoritative source, its revision, and the permitted update path?

**Warning sign:** Two individually reasonable components maintain different versions of the same business rule or structural definition.

### DM-03 — Unify logical contracts without mandating one physical structure

**MUST.** A unified substrate means coherent semantics, identity, versioning, and interchange across the system. It does not require one universal table, one object graph, one process, or one storage format. Permit specialized physical representations when their relationship to the canonical model is explicit and they cannot silently become competing authorities.

**Assessment test:** Can each representation state what it owns, what it derives, and how it relates to canonical identities and versions?

**Warning sign:** Either a universal key-value structure erases meaning, or specialized subsystems invent incompatible definitions.

### DM-04 — Declare the semantic boundary and expose opaque behavior

**MUST.** Specify what the model describes and what remains an external input, specialized implementation, or intentionally opaque component. Decision-relevant behavior outside the declarative language must enter through a registered contract with explicit dependencies, effects, assumptions, and limitations. Do not claim complete model coverage while hiding material rules inside callbacks.

**Assessment test:** Can a reviewer enumerate the important behavior that is not inspectable in the model and explain the contract governing each exception?

**Warning sign:** An architecture claims to be fully declarative, but arbitrary scripts determine outputs behind an opaque operation name.

### DM-05 — Separate intent from mechanisms and incidental technology

**SHOULD.** Represent the desired structure, relationships, constraints, and outcomes independently of a particular class hierarchy, query engine, deployment topology, or execution backend. Select mechanisms through explicit policy and lowering decisions. Technology-specific facts are legitimate at the appropriate stage; they should not leak upward and redefine portable intent.

**Assessment test:** Could another conforming backend consume the same intent without changing its meaning, even if that backend has different capability limits?

**Warning sign:** Public model definitions mirror one framework’s internal objects or require callers to reproduce its construction sequence.


## 2. Semantic types, schemas, and invariants

### DM-06 — Type semantic distinctions, not only machine representations

**MUST.** Use types or explicit semantic references for distinctions that change interpretation: role, category, coordinate system, reference convention, ordering, basis, scope, and shape. Define the operations and conversions these distinctions allow. When a distinction cannot be statically enforced, validate it before the affected operation executes.

**Assessment test:** Could two values with the same primitive type be interchanged incorrectly without the system rejecting or explicitly converting them?

**Warning sign:** Strings, floats, or integer IDs are treated as interchangeable because their physical encodings match.

### DM-07 — Make validity rules explicit and enforce them at identified boundaries

**MUST.** Declare local, relational, cross-field, conditional, and lifecycle invariants. Identify the layer that enforces each invariant and the point at which it must hold. Structural validation alone is insufficient when semantic compatibility or cross-record consistency affects correctness. A schema registry must not be credited with enforcement that no implementation performs.

**Assessment test:** For each critical invariant, can the reviewer point to an enforcement mechanism, failure behavior, and negative test?

**Warning sign:** An invariant is documented or annotated but ordinary write or execution paths can bypass it.

### DM-08 — Represent absence, unknowns, uncertainty, invalidity, and failure distinctly

**MUST.** Do not overload nulls, empty strings, zeros, NaNs, or missing records with incompatible meanings. Model distinctions such as not supplied, not applicable, not yet computed, deliberately unknown, uncertain, invalid, and failed when they affect behavior. Separate an unknown to be determined from the absence of an initial value or observation.

**Assessment test:** Can callers distinguish all materially different states without inferring meaning from a sentinel or undocumented convention?

**Warning sign:** A missing value is interpreted as a default in one subsystem, an unknown in another, and a failure elsewhere.

### DM-09 — Model relationships and valid domains explicitly

**MUST.** Represent ownership, membership, connectivity, ordering, dependencies, and correspondence with explicit roles, cardinalities, and valid indexing domains. Do not assume that all combinations of otherwise valid values are themselves valid. Prefer normalized authoritative facts; use denormalized execution views when beneficial and derive them through explicit rules.

**Assessment test:** Are valid combinations and relationship meanings declared, or inferred from coincidental row positions, matching labels, or unrestricted Cartesian products?

**Warning sign:** Parallel arrays, position-based joins, or string parsing encode relationships that should have their own contracts.

### DM-10 — Keep important structure typed and queryable

**SHOULD.** Use dedicated fields, relations, and tagged alternatives for stable, decision-relevant structure. Reserve maps, free text, and opaque blobs for genuinely open-ended content or well-defined boundaries. Promote a previously opaque field when validation, joins, transformations, or execution decisions begin to depend on its internal meaning.

**Assessment test:** Which important queries or checks require parsing arbitrary text or untyped payloads, and why has that structure not been modeled?

**Warning sign:** An apparently schema-first system is an entity-attribute-value store or JSON envelope with application-specific interpretation everywhere.


## 3. Identity, versions, and consistency

### DM-11 — Use stable semantic identity independent of physical location

**MUST.** Give persistent entities identities whose meaning does not depend on names, file paths, row order, memory addresses, storage partitions, or backend-local indices. Treat those properties as attributes or implementation details. Define identity behavior under rename, move, duplication, split, merge, and regeneration where those operations exist.

**Assessment test:** Do identities and references survive harmless reordering, repartitioning, renaming, and serialization round trips?

**Warning sign:** A row index, dictionary encoding, mutable path, or process-local pointer is used as a durable entity identifier.

### DM-12 — Distinguish entity, revision, artifact, and execution identity

**MUST.** An enduring entity, a particular revision, an encoded artifact, and an execution attempt answer different questions. Represent them separately and define their relationships. Content equality does not necessarily imply entity identity, and a repeated run does not necessarily create a new model revision. Make identity scope explicit.

**Assessment test:** Can the system separately answer “which thing,” “which definition,” “which stored representation,” and “which attempt”?

**Warning sign:** One UUID or hash is expected to identify both a logical entity and every evolving state and execution associated with it.

### DM-13 — Separate definitions, specifications, policies, observations, and results

**MUST.** Distinguish what exists, how it is bound or configured, which policy selects behavior, what has been observed, and what an execution produced. Version these concepts according to their own lifecycles while recording compatibility and dependency links. Do not let a computed result silently overwrite the declaration or input from which it was obtained.

**Assessment test:** Can a new case or execution reuse the same definition, and can a result be traced without mutating that definition?

**Warning sign:** Configuration, runtime state, user intent, measurements, and cached output occupy one mutable object with unclear update semantics.

### DM-14 — Publish semantically consistent revisions through explicit commit boundaries

**MUST.** Readers must know which coherent revision they are observing. When a change spans several authoritative structures, validate and publish it as a consistent unit at the level required by the application. Use snapshots, transactions, manifests, or equivalent protocols; do not imply that individually atomic writes provide multi-structure consistency.

**Assessment test:** What can a reader observe during failure or concurrent publication, and can it accidentally combine incompatible revisions?

**Warning sign:** Related tables are updated separately and readers have no way to detect or avoid a mixed-version model.

### DM-15 — Define canonicalization and equivalence before using content identity

**MUST.** Specify which differences are semantically meaningful and which are merely representational. Canonicalization must address ordering, encoding, defaults, metadata, numeric edge cases, and schema versions as applicable. Distinguish byte equality, structural equality, declared semantic equivalence, and approximate numerical agreement. Do not claim general semantic equivalence from hashing.

**Assessment test:** Would harmless encoding changes preserve the intended identity while a meaningful change to interpretation reliably invalidate it?

**Warning sign:** Hashes vary with batch layout, or silently ignore a change in reference convention that alters meaning.


## 4. Declarative composition and reusable structure

### DM-16 — Represent material structure and policy as declarations

**SHOULD.** Express reusable structures, constraints, selection rules, and important workflows as typed declarations where this improves validation, composition, or explanation. Keep algorithmic implementation inside well-defined interpreters and kernels. Model the semantic choices an author needs to control, not every loop, allocation, or implementation branch.

**Assessment test:** Can an ordinary domain change be expressed as a validated model change instead of coordinated edits to procedural construction code?

**Warning sign:** Either scripts conceal all meaningful decisions, or every implementation detail becomes an elaborate configuration language.

### DM-17 — Use templates and bindings instead of copied construction logic

**SHOULD.** Represent repeated structures once with declared parameters, required capabilities, feature conditions, and instantiation rules. Separate a template’s meaning from particular instances and their bindings. Generated instances must retain their origin and bound arguments. Prefer composition over duplicating nearly identical constructors or inheritance branches.

**Assessment test:** Does a new instance require new semantic information, or duplicated code whose differences could have been typed bindings?

**Warning sign:** A family of components repeats construction and validation logic with small variations encoded in custom subclasses.

### DM-18 — Preserve high-level structure until expansion is required

**SHOULD.** Keep collections, indexed operations, patterns, constraints, and other compact structures explicit until a consumer requires their expansion. Make expansion rules, domains, and generated identities inspectable. Lazy materialization is acceptable when requests and dependency closure are explicit; it must not depend on accidental inspection order.

**Assessment test:** Can a large instance retain a compact definition, and can the compiler explain when and why that definition was expanded?

**Warning sign:** The model is immediately flattened into millions of low-level objects, or hidden lazy construction depends on attribute access.

### DM-19 — Select behavior through declared capabilities and explicit bindings

**MUST.** Describe what an operation requires and what a provider guarantees. Resolve provider selection through validated capabilities and explicit policy, then persist the selected identity and version. Avoid dispatch based on accidental names, import order, reflection side effects, or undocumented type hierarchies. Account for ambiguity and absence.

**Assessment test:** Can the system explain why a provider was selected, prove it satisfies the required contract, and reproduce that selection later?

**Warning sign:** Installing a package or changing import order silently selects a different implementation for an existing model.

### DM-20 — Make inspection and validation semantically non-mutating

**MUST.** Reading, reporting, inspecting, and validating must not silently change the authoritative model or select new behavior. Explicit demand resolution and materialization may produce declared derived artifacts. Internal caches may mutate only under a documented contract that does not change the logical definition or externally observable interpretation.

**Assessment test:** Does inspecting a field, generating a report, or asking whether a capability exists leave the model’s meaning unchanged?

**Warning sign:** A getter constructs new rules, a diagnostic changes specifications, or a report chooses a provider.


## 5. Compilation, derivation, and semantic preservation

### DM-21 — Use explicit intermediate representations and progressive lowering

**SHOULD.** Separate portable intent, resolved structure, instantiated details, and execution-specific plans when those stages have different responsibilities. Give each representation a schema, invariants, identity scope, and source mapping. Do not force one structure to be simultaneously an authoring format, validation model, execution layout, and result store.

**Assessment test:** Can the reviewer identify what information is introduced, resolved, retained, or discarded at each stage?

**Warning sign:** A backend-specific object graph becomes the only surviving representation of the user’s original intent.

### DM-22 — Give every meaningful transformation a contract

**MUST.** Declare each transformation’s inputs, outputs, versions, preconditions, postconditions, dependencies, effects, and failure modes. A transformation with several output schemas should publish an explicit artifact bundle. Apply the same contract discipline whether its implementation uses queries, generated code, a specialized algorithm, or a foreign library.

**Assessment test:** Can a pass be invoked, validated, cached, tested, and explained without knowing its internal implementation?

**Warning sign:** Pipeline functions mutate shared structures and communicate through undocumented ordering or ambient state.

### DM-23 — Make derived representations traceable and non-competing

**MUST.** Every derived structure must identify its source revisions and derivation procedure. Rebuilding should be possible from the complete declared inputs when the derivation contract promises it. Editable projections require an explicit inverse or command translation with conflict checks; otherwise they are read-only. Do not confuse derived results with independently acquired observations.

**Assessment test:** Can a cache or backend object be discarded and recreated, or can the system explain the external evidence needed to reproduce it?

**Warning sign:** A materialized view acquires manual corrections that cannot be expressed in or reconciled with the authoritative model.

### DM-24 — Preserve semantics across rewrites and lowerings

**MUST.** Define the equivalence promised by each rewrite, optimization, conversion, or approximation. Attach assumptions and validity conditions, enforce them, and invalidate dependent artifacts if they cease to hold. Where a transformation deliberately changes behavior, model the change as a selected policy rather than calling it a semantics-preserving optimization.

**Assessment test:** What observable behavior is preserved, under which assumptions, and which tests, derivations, or proofs support that claim?

**Warning sign:** An optimizer assumes nonzero values, stable ordering, commutativity, or identical precision without recording the condition.

### DM-25 — Unify operation contracts while allowing specialized implementations

**SHOULD.** Define reusable operation contracts that support validation, planning, execution, and any required auxiliary operations. Generate adapters where possible, but allow specialized kernels and algorithms inside the contract. A unified substrate must not force unsuitable operations into generic queries or limit the system to whatever one engine already implements.

**Assessment test:** Can one operation definition support multiple execution paths without separately reimplementing its meaning in each?

**Warning sign:** Either every operation is squeezed into a universal interpreter, or each backend independently defines its semantics.


## 6. Planning, execution, mutable state, and effects

### DM-26 — Separate preparation from repeated execution

**SHOULD.** Resolve structure, validate compatibility, determine dependencies, and prepare execution layouts before repeatedly evaluating a workload when those facts are stable. Bind changing values through explicit interfaces. Record which assumptions justify reuse so that preparation is repeated only when required, rather than rediscovered inside every inner-loop operation.

**Assessment test:** Which work depends on structure and which depends on current inputs, and is that distinction reflected in the execution lifecycle?

**Warning sign:** Each evaluation rebuilds the model, repeats joins to rediscover dependencies, or re-crosses language boundaries per primitive operation.

### DM-27 — Represent important workflows as inspectable plans or state machines

**SHOULD.** Model material sequencing, dependencies, temporary specifications, transitions, retries, cancellation points, and completion criteria explicitly. Use an appropriate structure: a plan, DAG, state machine, or protocol—not one universal graph shape. Generate the routine orchestration from that structure while leaving specialized step implementations behind typed contracts.

**Assessment test:** Can the system explain the next valid action, why it is enabled, and how interruption or resumption changes the state?

**Warning sign:** A long script contains the only definition of workflow order, recovery rules, and temporary modifications.

### DM-28 — Declare effects, ambient inputs, and nondeterminism

**MUST.** Mark operations that read external state, use time or randomness, perform I/O, mutate authoritative state, or depend on environment-specific behavior. Give optimizers and schedulers truthful purity and ordering contracts. Capture or bind external inputs when reproducibility is required. Never hide an effectful operation inside an interface treated as pure.

**Assessment test:** Could an optimizer safely duplicate, reorder, cache, or omit this operation under its declared contract?

**Warning sign:** An apparently deterministic query function launches a job, reads the current clock, changes a file, or performs a write.

### DM-29 — Isolate mutable workspaces and commit their outcomes explicitly

**MUST.** Allow efficient mutable runtime state within a clearly owned execution context. Define how it differs from authoritative definitions and which outputs become committed facts. Temporary overrides should be scoped overlays or equivalent controlled state, not hidden edits requiring fragile manual reversal. Do not require a full immutable snapshot for every inner-loop update.

**Assessment test:** What state may change during execution, who owns it, and what remains visible after success, cancellation, or failure?

**Warning sign:** A failed attempt leaves the reusable model partly fixed, partially deactivated, or otherwise altered for the next caller.

### DM-30 — Make partial failure and recovery explicit

**MUST.** Distinguish planned, running, completed, partial, stale, cancelled, and failed outcomes as needed. Define retry and idempotency semantics for effectful actions, and distinguish an execution attempt from its logical operation. Validate output completeness before publication. Do not imply rollback or exactly-once behavior when the underlying protocol cannot guarantee it.

**Assessment test:** After a crash or retry, can the system identify what happened, which outputs are valid, and which effects may already have occurred?

**Warning sign:** A partially written artifact looks complete, or an automatic retry duplicates an external action.


## 7. Dependencies, incrementality, and concurrency

### DM-31 — Expose every dependency that can affect meaning or output

**MUST.** Track structural inputs, parameters, policies, provider versions, interpretation rules, assumptions, and relevant environmental facts. Include dependencies on absence or selection decisions when they matter. Dependencies may be represented conservatively when exact analysis is impractical; hidden dependencies must not be ignored simply to obtain more reuse.

**Assessment test:** Could any undeclared change alter this artifact or provider selection without changing its recorded inputs?

**Warning sign:** A cache key includes data files but omits a policy, default provider, locale, or assumption that changes interpretation.

### DM-32 — Key reuse to semantic dependencies rather than convenience

**MUST.** Cache artifacts only when their complete declared inputs and applicable equivalence conditions match. Separate a cache lookup mechanism from its correctness argument. Test reuse against clean recomputation after relevant changes. Caches must remain disposable unless explicitly promoted to authoritative state with a separate persistence contract.

**Assessment test:** For each reused artifact, why is reuse valid, and what evidence shows invalidation catches meaningful changes?

**Warning sign:** A filename, timestamp, object identity, or broad model ID is used as a sufficient cache key without dependency analysis.

### DM-33 — Invalidate at the smallest trustworthy semantic granularity

**SHOULD.** Distinguish changes to structure, bindings, policies, assumptions, numerical values, and execution settings. Recompute only affected artifacts when dependency evidence supports doing so. Prefer conservative invalidation to unsound reuse; measure invalidation precision before introducing complicated mechanisms intended to reduce rebuilding.

**Assessment test:** Does a small case change trigger a needless full rebuild, and does a meaningful structural or assumption change invalidate all affected products?

**Warning sign:** Everything recompiles for every value change, or structural artifacts survive changes to assumptions used during their construction.

### DM-34 — Keep different relationship structures semantically distinct

**MUST.** Do not conflate ownership, physical connectivity, dependency, dataflow, control flow, provenance, equivalence, or conflict relationships. Represent their roles and direction explicitly. Preserve legitimate cycles where the domain permits them; use derived acyclic scheduling structures only where their interpretation justifies it.

**Assessment test:** What does each edge mean, and would reversing, removing, or traversing it have the same meaning across all edge types?

**Warning sign:** One generic graph is used for ownership, scheduling, and dependency analysis despite incompatible edge semantics.

### DM-35 — Make concurrency respect dependencies, ownership, and declared ordering

**MUST.** Parallelize independent work or use explicit synchronization and reduction contracts for shared work. Specify ownership, thread safety, cancellation, resource limits, and ordering sensitivity. Coordinate nested execution pools where necessary. A columnar or graph representation does not by itself establish that operations can execute independently.

**Assessment test:** Can concurrent execution change correctness, visible ordering, effect counts, or reproducibility beyond the stated contract?

**Warning sign:** Parallelism is added based on data layout while hidden shared state, dependent stages, or competing thread pools are ignored.


## 8. Execution representations and performance

### DM-36 — Choose physical layouts for demonstrated access patterns

**SHOULD.** Use columnar batches, dense arrays, sparse structures, adjacency layouts, trees, or other representations according to actual operations. Keep the mapping to canonical identities and semantics explicit. Treat physical layouts as compiled or materialized views unless they truly own independent facts. Avoid both object-per-cell overhead by default and a universal table mandate.

**Assessment test:** Which measured access pattern motivates the layout, and how can it be replaced without rewriting the semantic model?

**Warning sign:** An inappropriate layout is defended as architectural purity, or a fast layout becomes the only place meaning is recorded.

### DM-37 — Cross expensive boundaries in coarse, typed units

**SHOULD.** Transfer validated bundles, batches, plans, or artifacts across process, language, storage, and accelerator boundaries when that suits the workload. Declare schema, ownership, lifetime, ordering, encoding, and cancellation behavior. Accept necessary copies for correctness or performance; zero-copy is an optimization with preconditions, not a design objective that overrides semantics.

**Assessment test:** Is boundary overhead proportional to useful work, and are representation, lifetime, and transfer costs accounted for?

**Warning sign:** Cross-language calls occur per primitive operation, or a zero-copy claim ignores reordering, mutable workspaces, or process boundaries.

### DM-38 — Match the execution mechanism to the operation’s semantics

**SHOULD.** Use relational engines for relational work, specialized algorithms for specialized work, and generated evaluation programs for repeated computation where appropriate. Integrate them through the same typed operation and artifact contracts. Shared representation should enable cooperation, not require every algorithm to be expressed in one execution language.

**Assessment test:** Is an operation placed in an engine because its semantics and workload fit, or merely because that engine is already present?

**Warning sign:** Inner-loop computation repeatedly performs relational reconstruction, or relational assembly is rewritten as avoidable per-record scripts.

### DM-39 — Evaluate performance end to end and distinguish evidence from expectation

**MUST.** Measure construction, validation, compilation, transfer, execution, storage, recovery, and inspection costs where material. Include memory use, cold and warm behavior, representative scale, and contention. Attribute gains to measured mechanisms. Do not infer whole-system speedups from language choice, a datatype, vectorization, or an isolated kernel benchmark.

**Assessment test:** Which bottleneck improves, what new costs appear, and what representative evidence supports the claimed benefit?

**Warning sign:** A proposal promises major speedups while ignoring model construction, copies, invalidation, external execution, or memory growth.

### DM-40 — Declare precision, approximation, ordering, and determinism requirements

**MUST.** State the required equivalence and accuracy class for calculations and outputs. Specify permitted approximation, precision changes, reduction order, stochastic behavior, and tolerance policies. Record when backend or hardware choices can affect outcomes. Require bitwise reproducibility only when needed and supported; otherwise define an honest weaker contract.

**Assessment test:** Can two conforming executions differ, by how much or in what ways, and how will conformance be assessed?

**Warning sign:** Mathematical equivalence, structural equality, approximate agreement, and identical bytes are treated as the same guarantee.


## 9. Boundaries, providers, and extensibility

### DM-41 — Keep adapters mechanical and domain conversions explicit

**MUST.** Adapters may translate representations, bind identifiers, and implement declared conversions. They must not become independent owners of domain rules, hidden defaults, provider selection, or policy. When a conversion changes interpretation, represent it as a named, validated transformation rather than burying it in serialization or glue code.

**Assessment test:** Does crossing a boundary merely realize a known contract, or introduce a new interpretation that exists nowhere else?

**Warning sign:** A Python bridge, serializer, API handler, or storage adapter quietly adds rules that the canonical model does not contain.

### DM-42 — Make interchange loss-aware and reject silent semantic degradation

**MUST.** Preserve or explicitly translate required type information, metadata, identities, ordering, null semantics, and provenance across boundaries. Define how receivers handle unknown versions and unsupported constructs. A projection or lossy export may be legitimate, but it must declare what is omitted and must not masquerade as a complete authoritative representation.

**Assessment test:** Can an exported and re-imported artifact preserve its promised semantics, and does the system detect information the target cannot represent?

**Warning sign:** Metadata disappears during a query, cast, export, or bridge operation and downstream code continues as if meaning were preserved.

### DM-43 — Negotiate capabilities and expose unsupported behavior

**MUST.** Require execution providers to declare supported operations, auxiliary functionality, limitations, and versions. Validate the requested plan against that contract before execution. A fallback must preserve the required semantics or be explicitly selected as a changed policy. Never interpret successful registration as proof of end-to-end executability.

**Assessment test:** Which required capability is supplied by which provider, and what happens when an operation, shape, or guarantee is unsupported?

**Warning sign:** A feature is accepted at construction but fails deep in execution, or silently runs with different semantics on a fallback backend.

### DM-44 — Make extensions complete, versioned, and conformance-testable

**MUST.** An extension contract must cover its signature, semantics, validity, dependencies, effects, failures, execution bindings, and required auxiliary operations. Auxiliary operations may include validation, inversion, dependency analysis, differentiation, or explanation, depending on the system. Generate wrappers from the contract where possible; do not pretend a schema supplies missing implementations.

**Assessment test:** Can an extension be independently tested for every capability it claims, and do generated wrappers share one underlying semantic definition?

**Warning sign:** Adding an operation requires disconnected implementations in several backends, each with incomplete or inconsistent behavior.

### DM-45 — Treat trust and authority as explicit execution constraints

**MUST.** Validate external inputs and agent-proposed changes through the same semantic gates as other writes. Distinguish data from executable authority, and declare which operations may read, execute, or modify external resources. Match isolation, permissions, and resource limits to actual risks. Schema conformance alone does not authorize an effect or establish input trust.

**Assessment test:** Can untrusted content, a plugin, or an agent bypass model validation or trigger effects beyond the explicitly granted authority?

**Warning sign:** A valid payload can inject arbitrary code, invoke hidden effects, or alter authoritative state through an ungoverned escape hatch.


## 10. Provenance, reproducibility, and explainability

### DM-46 — Preserve source-to-result lineage through transformations

**MUST.** Connect authored declarations, imported observations, selected policies, transformations, generated structures, executions, and outputs. Record many-to-many derivation where needed rather than forcing one parent per output. Distinguish provenance from ownership and ordinary dependency edges. Preserve enough origin information to explain generated artifacts and diagnose failures.

**Assessment test:** Can a problematic output or generated rule be traced to the specific inputs, selections, and transformations responsible for it?

**Warning sign:** Generated objects have identifiers but no link to the authored declaration or policy that caused them to exist.

### DM-47 — Represent diagnostics as structured evidence

**MUST.** Use stable diagnostic codes, severity, affected identities, stage, observed values, violated rules, causal context, and remediation hints where appropriate. Distinguish model invalidity, unsupported capability, numerical or algorithmic failure, infrastructure failure, and inconclusive checks. Human-readable messages should be projections of structured evidence, not the only source of meaning.

**Assessment test:** Can a tool classify and act on a failure without parsing prose or guessing whether the model or execution infrastructure is at fault?

**Warning sign:** Everything becomes a generic exception or log string, including unsupported features and actual correctness failures.

### DM-48 — Define and support the required reproducibility contract

**MUST.** Record the versions, input snapshots, specifications, policies, execution settings, assumptions, provider artifacts, and environmental conditions needed for the promised replay class. Capture external observations when they cannot be reproduced. Distinguish recomputation from replay of recorded evidence, and disclose nondeterministic or unavailable dependencies.

**Assessment test:** What exactly can be reproduced later, with which inputs and guarantees, and what is explicitly outside that promise?

**Warning sign:** A result is labeled reproducible because input values were saved, while provider versions, policies, or external state were omitted.

### DM-49 — Make changes understandable at the level of meaning

**SHOULD.** Provide semantic diffs that identify changed declarations, constraints, bindings, policies, provider selections, assumptions, and affected derived artifacts. Separate these from harmless physical reorderings or serialization changes. Use dependency information to explain the expected impact without claiming certainty about untested numerical or external outcomes.

**Assessment test:** Can a reviewer explain what changed and why it matters without comparing raw bytes or reverse-engineering generated code?

**Warning sign:** A small meaningful change produces an unreadable generated diff, or a major semantic change looks like an innocuous metadata edit.

### DM-50 — Observe the model lifecycle, not only low-level operations

**SHOULD.** Expose inspectable plans, validity status, dependency selections, cache decisions, stage costs, artifact sizes, and outcomes linked to semantic identities. Provide accessible projections for both humans and agents. Instrumentation must respect non-mutating inspection and should be budgeted so that explanation does not accidentally dominate execution.

**Assessment test:** Can a user see why the system constructed, selected, reused, rejected, or executed something and where its cost arose?

**Warning sign:** There are abundant infrastructure logs but no explanation of the model-level decision that produced the behavior.


## 11. Evolution, generation, and verification

### DM-51 — Evolve schemas and semantics through explicit migrations

**MUST.** Version structural and semantic contracts, classify compatibility, and supply validated migration paths where required. Preserve origin and migration provenance. Generated tooling can handle mechanical changes, but meaning-changing migrations need explicit decisions and tests. Unknown or incompatible versions must not be silently reinterpreted using current defaults.

**Assessment test:** Can an older artifact be interpreted under its original contract or migrated with documented effects and verified invariants?

**Warning sign:** A renamed field, changed default, or altered semantic convention silently changes the meaning of stored models.

### DM-52 — Generate repeated mechanical artifacts from shared contracts

**SHOULD.** Generate types, column accessors, builders, validators, serialization bindings, schemas, documentation, and conformance scaffolding when they express the same contract. Keep handwritten logic for genuinely semantic or algorithmic work. Define ownership and regeneration rules so that generated files do not become independently edited authorities.

**Assessment test:** When a contract changes, which artifacts update automatically and which require a genuinely new semantic decision?

**Warning sign:** A field or operation is separately declared in application types, transport schemas, validators, documentation, and adapters.

### DM-53 — Verify invariants and equivalence across representations

**MUST.** Test authoritative invariants, serialization round trips, backend conformance, transformation conditions, and declared equivalence. Use property-based, metamorphic, differential, or reference tests where appropriate. Independent execution paths are useful evidence, but agreement alone is not proof if both share the same faulty transformation.

**Assessment test:** What would detect semantic drift in an adapter, rewrite, physical layout, generated implementation, or alternate backend?

**Warning sign:** Tests check a few expected outputs while identity, metadata, invalid inputs, and representation changes remain untested.

### DM-54 — Test adversarial lifecycle and boundary conditions

**MUST.** Exercise malformed inputs, invalid relationships, empty domains, unknown tags, missing capabilities, precision boundaries, interrupted publication, stale caches, concurrent changes, cancellation, partial results, and retry behavior as applicable. Include negative tests showing that prohibited states or actions are rejected at their declared boundaries.

**Assessment test:** Which failures occur between happy-path stages, and are their effects on state, authority, and recovery actually tested?

**Warning sign:** The design is validated only through successful small examples and assumes construction, publication, and execution always complete.

### DM-55 — Make contracts and extension paths discoverable to humans and agents

**SHOULD.** Expose stable names, schema documentation, operation contracts, capability registries, examples, diagnostics, and source mappings in a navigable form. Document the sanctioned change path: declaration, binding, transformation, kernel, adapter, or runtime policy. Prefer a compact map with focused references over duplicated narratives that drift from implementation.

**Assessment test:** Can a competent agent find the authoritative concept and correct extension point without inferring conventions from unrelated examples?

**Warning sign:** Correct changes require tribal knowledge of hidden hooks, special filenames, reflection behavior, or undocumented construction order.


## 12. Architectural leverage and disciplined improvement

### DM-56 — Optimize for fewer independent semantic decisions—not fewer lines

**SHOULD.** Judge an abstraction by how much duplicated meaning, coordinated editing, and exception handling it removes. Boilerplate generated from one contract is less harmful than compact code that hides several conflicting rules. Measure extension locality, semantic duplication, and failure surface; do not use line count or the number of tables as the primary design score.

**Assessment test:** Does an ordinary extension require one new semantic declaration plus focused implementation and tests, or coordinated policy edits across the system?

**Warning sign:** A shorter implementation is praised despite moving important meaning into implicit conventions or scattered generic callbacks.

### DM-57 — Prefer a small coherent core with explicit extension mechanisms

**SHOULD.** Keep the core semantic vocabulary and lifecycle understandable. Add new concepts when they capture recurring distinctions or remove real exceptions. Support new algorithms through registered contracts rather than uncontrolled escape hatches. Avoid both a closed language that cannot express legitimate needs and a universal engine whose every case requires custom scripts.

**Assessment test:** Does the extension use existing concepts coherently, justify a new concept, or merely route around the model?

**Warning sign:** A growing collection of flags and arbitrary callbacks turns a nominally generic system into many hidden special cases.

### DM-58 — Scale architectural machinery to demonstrated needs

**SHOULD.** Implement the smallest coherent design that preserves the important semantic boundaries and required guarantees. Introduce elaborate registries, distributed storage, incremental compilers, code generation, or orchestration only when their leverage is demonstrated or a concrete requirement demands them. Preserve useful extension seams without building hypothetical platforms.

**Assessment test:** Which present requirement or tested extension justifies each layer, service, engine, and abstraction?

**Warning sign:** The system needs an entire platform before one representative end-to-end use case can work.

### DM-59 — Make design claims falsifiable and label uncertainty

**MUST.** For proposed benefits, state the mechanism, assumptions, baseline, validation method, and current evidence. Distinguish an implemented capability, a checked library interface, a tested prototype, a measured benefit, and a hypothesis. Present tradeoffs and uncertainty directly. Do not convert architectural preference into an unsupported correctness or performance claim.

**Assessment test:** What observation could show that this proposal fails to improve its stated objective, and has that observation been sought?

**Warning sign:** A design is called unified, declarative, scalable, or faster without operational definitions or supporting evidence.

### DM-60 — Turn the principles into change-level review and regression controls

**MUST.** For each material design change, identify applicable principles, record evidence and deviations, and add tests or automated checks for guarantees that can regress. Give exceptions explicit scope, rationale, consequence, compensating controls, owner, and revisit conditions. A high aggregate score cannot compensate for a violated mandatory correctness boundary.

**Assessment test:** Can a future change reintroduce the defect this design removed, and what review or automated control would catch it?

**Warning sign:** The charter is praised once, then implementation and later extensions are reviewed without reference to its contracts.


# Review and improvement framework

## A. Acceptance gates

These gates apply to the supported behavior and correctness requirements actually in scope. They do not require implementing hypothetical features. A conceptual proposal can record an unresolved gate, but must not be presented as verified or ready for an affected use case. No weighted score compensates for a failed gate.

| Gate | Reject or revise the design when… | Main principles |
|---|---|---|
| G1 — Authority | Two independently mutable definitions can disagree about the same authoritative fact without an explicit reconciliation protocol. | DM-02, DM-11–15, DM-23 |
| G2 — Semantic fidelity | Required meaning is ambiguous, silently dropped, reinterpreted, or represented by an indistinguishable sentinel. | DM-01, DM-06–10, DM-24, DM-40, DM-42 |
| G3 — Validity | An invalid state can reach an operation that assumes it is valid, without a defined rejection or safe handling path. | DM-07, DM-22, DM-43–45, DM-53–54 |
| G4 — Hidden behavior | Inspection, optimization, provider selection, or an apparently pure operation can introduce undeclared effects or alter the model’s meaning. | DM-04, DM-19–20, DM-28 |
| G5 — Consistency and recovery | Readers can mistake an inconsistent revision or partial failed output for a valid committed result, or retries can produce ungoverned effects. | DM-14, DM-29–30, DM-35 |
| G6 — Transformation and reuse | A rewrite, cache hit, projection, or alternate backend changes required behavior without a valid contract or selected approximation policy. | DM-24, DM-31–33, DM-40–43 |
| G7 — Truthful capability claims | A required capability is claimed without an actual implementation route, or an unsupported operation silently falls back to different semantics. | DM-43–44, DM-59 |

## B. Assessment dimensions

Use this rubric to compare alternatives or track improvement, not to manufacture precision. Score only dimensions relevant to the change and state any reweighting. Show an **evidence-supported current score** separately from a **proposed target**; a narrative promise is not an implemented guarantee. For small changes, a qualitative assessment is preferable to a numerical total.

| Dimension | Suggested weight | What to inspect |
|---|---:|---|
| Semantic completeness and validity | 20 | Important distinctions are typed, relationships are explicit, invariants are enforced, transformations preserve the declared meaning. |
| Authority, identity, and lifecycle consistency | 15 | One authority per fact; stable IDs; clear revisions, specifications, executions, commit boundaries, and derived artifacts. |
| Declarative leverage and extension locality | 15 | Reusable templates, explicit capabilities, generated mechanical artifacts, few independent semantic decisions. |
| Derivation, dependency, and reuse correctness | 10 | Typed stages, complete dependencies, explainable invalidation, separation of preparation from repeated execution. |
| Execution, boundaries, and failure behavior | 10 | Effects, mutable workspaces, concurrency, adapters, unsupported features, and partial failures have explicit contracts. |
| Verification and evolution | 10 | Negative and conformance tests, schema migrations, equivalence checks, and regressions are covered. |
| Provenance and inspectability | 10 | Source-to-result evidence, structured diagnostics, semantic diffs, and realistic reproducibility guarantees. |
| Measured cost and proportionality | 10 | Appropriate layouts and mechanisms, end-to-end measurements, no unsupported benefits or unjustified platform complexity. |

Suggested maturity scale:

| Rating | Evidence level |
|---:|---|
| 0 | Missing, contradictory, or outside the claimed supported behavior without disclosure. |
| 1 | Stated in prose or convention; no reliable implementation or enforcement. |
| 2 | Explicitly represented and partly implemented, with significant manual coordination or verification gaps. |
| 3 | Implemented, enforced at defined boundaries, and supported by representative tests or other suitable evidence. |
| 4 | Systematically reusable across the relevant scope, conformance-tested across applicable paths, and protected against regression. |

Do not equate rating 4 with formal proof or unlimited generality. A small design can reach rating 4 within a narrow, well-tested scope.

## C. A repeatable design review

**Frame the change.** Describe the observable outcome, the existing baseline, scope, non-goals, required guarantees, and uncertainty. Separate semantic changes from mechanical refactoring and performance changes.

**Map authority and lifecycles.** Identify definitions, bindings, policies, observations, execution state, results, and derived views. State their identities, version boundaries, owners, and permitted update paths.

**Specify contracts before interfaces.** Describe semantic types, invariants, valid domains, important operations, effects, capabilities, and failure modes. Identify what remains deliberately opaque and why.

**Trace representative journeys.** Walk an ordinary extension, a meaningful change, a representation boundary, and an interrupted execution. Track each piece of meaning and state through the lifecycle. Select the journeys relevant to the change rather than completing irrelevant paperwork.

**Compare with a simpler viable alternative.** Explain the proposed abstraction’s leverage, costs, and failure surface. Count independent semantic decisions and coordinated edits, not merely source files. A declaration plus a genuinely new algorithm and tests is a legitimate extension.

**Check gates and record evidence.** Cite specific artifacts, schemas, tests, code paths, measurements, or documented gaps. Differentiate verified behavior from expected benefits and assumptions.

**Choose changes and regression controls.** Prioritize correctness and authority defects, then semantic duplication and extension difficulty, then measured performance costs. Record accepted tradeoffs and make recurring regressions mechanically detectable where practical.

## D. Evidence vocabulary

| Label | Meaning |
|---|---|
| Proposed | Described architecture or intended behavior; not implemented. |
| Interface-checked | A required interface or integration mechanism has been inspected; end-to-end behavior is not yet established. |
| Implemented | Code exists for the stated path; testing scope must still be described. |
| Tested | Named tests exercise the stated behavior over specified cases. |
| Measured | A stated benchmark or operational observation supports a quantitative claim under recorded conditions. |
| Formally established | A specific property follows from an identified formal argument or verified method with explicit assumptions; do not use this label for ordinary testing. |

Evidence labels describe different claims, not a single universal ladder. A measured implementation can still be incorrect; an interface-checked design can still need substantial engineering.

## E. The extension-locality test

Ask where a typical new rule, entity type, provider, or policy must be expressed. The desired pattern is:

> One authoritative semantic addition, any genuinely new specialized implementation, and focused conformance tests; routine adapters, validation scaffolding, documentation, and execution bindings derive from existing contracts where feasible.

A change spanning several files is not automatically a failure. A change requiring the same meaning to be independently re-expressed in several subsystems is the problem. Introducing a genuinely new semantic concept may legitimately require changes to the core language and its backends, with explicit versioning and conformance work.

## F. What belongs in the model and what belongs in code?

| Question | Default placement |
|---|---|
| Does this distinguish valid from invalid meaning or change an observable rule? | Canonical semantic declaration or a registered operation contract. |
| Is this a reusable structural pattern or important selection policy? | Typed template, binding, or policy. |
| Is this a specialized algorithm implementing an existing contract? | Ordinary well-tested code behind that contract. |
| Is this representation translation with no new domain decision? | Mechanical adapter or generated binding. |
| Is this an optimized layout or queryable projection? | Versioned derived artifact, with explicit dependencies and identity mapping. |
| Is this an effectful action or important execution sequence? | Typed action or workflow contract, with an implementation appropriate to the task. |
| Is this repeated mechanical expression of an existing schema? | Generated artifact. |
| Is this speculative flexibility with no demonstrated consumer? | Defer it; retain only an inexpensive, justified extension seam. |

## G. Common false positives

A system is not meaningfully data model–based merely because it uses tables, a graph, a schema language, immutable objects, a compiled language, or code generation. Inspect the meaning and authority, not the branding.

| Attractive claim | Hidden defect to check |
|---|---|
| “Everything is declarative.” | Arbitrary scripts or callbacks still decide important behavior. |
| “There is one substrate.” | Multiple independently editable definitions are stored in the same format. |
| “The schema enforces correctness.” | Critical semantic or relational invariants are only metadata. |
| “All state is immutable.” | Runtime services hide mutable state, or the design needlessly snapshots inner-loop work. |
| “It is zero-copy.” | Ownership, process boundaries, casting, reordering, or mutable workspaces invalidate the claim. |
| “Every backend is supported.” | Lowerings or auxiliary capabilities exist only for a subset of accepted operations. |
| “All behavior is generic.” | Every real use case adds more flags, special cases, or untyped escape hatches. |
| “It is faster because it is data-oriented.” | End-to-end transfer, compilation, construction, and recovery costs were never measured. |
| “The outputs match.” | Both implementations share the same incorrect transformation or ignore semantic metadata. |

## H. Scoped exception record

A SHOULD-level deviation should record: principle IDs; scope; reason; alternatives considered; consequences; compensating controls; evidence; accountable owner or role; and a revisit trigger. A MUST-level gap must be identified as unresolved or handled by narrowing supported scope—not hidden in an exception that still claims full compliance.

Do not add review machinery that costs more than the risk and decision warrant. A short, concrete decision record is sufficient for a small deviation.

## Closing standard

A successful design makes valid changes easier, invalid states harder to introduce, meaning easier to inspect, and execution mechanisms easier to replace. Its structure explains its behavior. Its extensions add meaning rather than duplicate machinery. Its claims are supported by evidence, and its remaining uncertainty is visible.
