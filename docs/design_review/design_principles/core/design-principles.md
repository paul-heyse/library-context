# Design principles

**Version 3.3 · 2026-10-05** · Core layer: repository- and domain-agnostic.
Supersedes core 3.2; §I preserves its lineage and the Data Model–Based Design Charter 1.0 ID map.

> **Localize change. Encode domain meaning explicitly. Extend through composition.**

The seven foundations below organize architectural judgment. DP-01–DP-24 retain their IDs as
supporting rules; §I records the version lineage. Domain profiles supply additional constraints.

## 0. How to use this document

Use these principles together with the [Heuristics for Efficient Architecture](efficient-architecture-heuristics.md)
when making consequential architectural and implementation choices. Consider the relevant
patterns before physical organization and lifecycle mechanisms become expensive to change.
The companion supports qualitative judgment; it adds no exhaustive checklist or proof obligation.

**Layers.** This is the core of a layered standard:

| Layer | Holds | May reference |
|---|---|---|
| Core (this document, the core review template and its heuristic companion) | Foundations `FP-01`–`FP-07`, architectural judgments `A1`–`A4`, supporting rules `DP-01`–`DP-24`, gates `G1`–`G8`, evidence vocabulary, review contract | Nothing outside the core |
| Domain profile | Principles and gates for a class of system (IDs with the profile's own prefix), plus review additions | The core |
| Repository binding | No principles. Maps profile roles to one repository's authorities, commands, routes and local policies | Core, profiles, repository |

A repository states which profiles and binding apply. §B gives the layering rules.

**Levels.** **MUST** is a required contract: a design must not claim alignment while violating
it. When a MUST cannot yet be met, narrow the supported scope or record the design as
unresolved against it. **SHOULD** is a strong default; a deviation needs a stated reason and an
exception record (§H) when material. A principle is *not applicable* only for a reason tied to
scope — unimplemented is not the same as irrelevant.

**How alignment is established.** Principles describe properties of a design, not procedures.
They are applied with judgment to the scope at hand — not by enumerating every principle for
every change, and not by a mechanical alignment checker. The engineers and agents applying them
establish facts by whatever means is most efficient and reliable for them: reading
documentation, source and types, reasoning from experience. Probes, tests and written records
are tools to use where real uncertainty or risk remains, never obligations or proof of
diligence. The audit questions ask what is true of the design, not which artifact shows it.

**Assessment scope.** Apply domain-model assessment during bounded design and review work at
the declared review cadence. The criterion concerns model adequacy and authority over behavior;
it does not require a separate modeling exercise for every implementation task. Reviewers choose
the least investigation sufficient to settle the scoped question. Following a flow is an optional
technique for resolving a concrete uncertainty, not a required sequence or exhaustive survey.

## 1. Governing objective

Design toward the best available design for the system's functional target. Local rules,
plan scope and conventions serve that target: where one blocks a clearly better design, record
the conflict as a required change with its route instead of treating it as a constraint.

Seek extensibility, maintainability, modularity, testability and evolvability through the seven
foundations. Correctness, fidelity and truthful claims constrain every supported design.
Within those constraints, compare realistic changes, isolation of tests, cognitive scope,
diagnosability and cost. Record the tradeoff and the scenario that makes it acceptable;
working output alone does not establish architectural fitness.

During early design, favor inexpensive learning, reversible choices and coherent boundaries.
Contracts can change deliberately; stable semantics do not require freezing a prototype API.
Compatibility, recovery and operational guarantees are scoped to real consumers and claimed
behavior. A planned capability can justify a seam before implementation; speculative variation
does not justify a framework. Proportionality applies to libraries and bespoke mechanisms alike.

Minimize **independent semantic decisions** and **bespoke generic code**, not source lines.
A specialized algorithm behind a complete contract is aligned; a declarative-looking wrapper
around hidden rules is not.

**Semantic-model-first domain design — MUST.** The implementation must embody an explicit,
coherent model of the phenomena and behavior the system is intended to represent. Distinctions
that affect validity, interpretation, composition or observable behavior must have deliberate
representations and authoritative definitions within a declared semantic scope.

Domain-relevant operations must realize that model through identifiable contracts: what they
consume, when they apply, what they produce or change, and the invariants, outcomes and effects
that govern them. Consumers and implementation mechanisms must use these definitions rather
than independently reconstruct their meaning.

The model may be expressed through types, relations, schemas, constraints and ordinary domain
functions. Its executable realization must govern construction, validation and behavior;
documentation or standardized output structures alone are insufficient. Model only the
distinctions needed for supported responsibilities and credible changes. No universal ontology,
object-oriented design, custom language, registry or conversion of every algorithm into data
is required.

This requirement applies throughout the implementation wherever domain meaning is established
or interpreted, including adapters, orchestration, persistence and presentation. Generic
mechanisms may remain generic; any domain decisions they make remain subject to the same
requirement. A MUST gap cannot be waived for supported behavior (§0, §H).

Establish functional intent and semantic obligations, then co-design boundaries and candidate
library realizations. Do not freeze a backend-neutral interface before examining the capabilities
it must preserve. Meaning remains authoritative; implementation capabilities inform its realization.

Design the semantic contract and its physical realization together. Preserve correctness, fidelity
and the declared assurance scope while choosing mechanisms that minimize unnecessary work, data
movement, coordination and lifecycle machinery for the supported workload. Domain authority does
not determine physical layout, execution placement or enforcement frequency. Compare the composed
system, including preparation, normal execution, failure and change. Prefer a simpler conforming
realization that removes substantial work or machinery; state a concrete benefit when retaining
the more expensive choice.

## 2. Seven foundational principles

These foundations are the primary architectural assessment. Evaluate them through the relevant
change scenarios (§E), with satisfied, violated or unresolved judgments. A concrete architectural
defect can require revision even when outputs are correct and a supporting rule is a SHOULD.

### FP-01 — Separation of concerns

Group decisions that change together; encapsulate decisions that change independently. Each
component owns a coherent responsibility and hides its implementation choices. Domain meaning,
representation, infrastructure, orchestration and configuration have identifiable owners and
dependency directions. Boundaries may be modules; another crate, process or service needs a reason.
A semantic or module boundary need not create a process call, store operation or materialization.

**Audit.** Which expected change belongs here? Why would it propagate to another owner? Is the
propagation required by a changed contract, or caused by knowledge of private mechanics?

### FP-02 — Stable contracts, replaceable implementations

Consumers depend on explicit meaning and behavior: inputs, outputs, preconditions, invariants,
effects, failure modes and compatibility. A substitute preserves the domain meaning as well as
the interface shape of that contract. Hide volatile provider details at the owning boundary;
use shared library types intentionally when they are the chosen contract. Introduce a trait,
protocol or adapter for a credible variation or testing need. Replacement need not mean
supporting several backends at once. An incompatible replacement
exposes the contract it cannot honor rather than silently redefining the domain; replacement
locality does not promise that every substitute is inexpensive.

**Audit.** What would an analyzer upgrade or mechanism replacement change in its consumers?
Which conformance obligations distinguish a valid substitute? Which compatibility is actually needed?

### FP-03 — Composition over entanglement

Build workflows by composing independently understandable capabilities with explicit contracts.
Keep policy with its semantic owner and effects at controlled boundaries. Orchestration orders
and connects domain operations through their owned semantics; new workflows should reuse
primitives without duplicating or independently interpreting their rules.
Lowerings may group, split or fuse operations while preserving their owned contracts and effects.
Use functions, pipelines or library operators where sufficient. A universal workflow engine or
plugin registry is justified only by real variation.

**Audit.** Can the next supported workflow be assembled from existing capabilities? Which new
semantics truly need implementation? Does it require another branch inside an unrelated core?

### FP-04 — Explicit domain model and scoped semantic authority

Represent the domain through the smallest sufficient set of meaningful concepts, relationships,
contextual bindings, constraints and operations. Align implementation vocabulary and contracts
with those meanings. Author each semantic decision once within its scope and revision, and
make behavior consume or realize that authority.

Authority can be distributed among coherent domain modules. Different contexts may use
different models when their scope and translations are explicit. Physical layouts, typed
values, tables and graphs may differ while preserving the meaning required by their contracts.
Declarations judge these representations; they do not prescribe a table, stage, privilege or
execution hop per declared kind. Semantic identity, physical identity mapping and lifecycle
mechanics are separately owned choices. Generate or inspect mechanical representations where practical. Independent tests
challenge the model and its realization rather than deriving every expectation from production
definitions.

**Audit.** Where are the phenomena and operations defined? Does that model govern behavior,
or merely describe outputs? Which owner changes a meaning, and which consumers would otherwise
reinterpret it? Can a projection be rebuilt? Would a shared mistake make the oracle agree?

### FP-05 — Explicit structure and constraints

Represent dependencies, lifecycle, ownership, state transitions, configuration, effects, failure
states, lineage and extension points explicitly where behavior depends on them. Prefer validated
construction and type/schema constraints to scattered conventions. Relational or dynamic
invariants may need shared runtime enforcement; select the boundary that can enforce them honestly.

**Audit.** What invalid state is prevented, where, and what remains a runtime obligation? Can a
consumer discover the dependencies and lifecycle without following hidden registrations or globals?

### FP-06 — Local reasoning

A developer or agent can understand, modify and test a component from its contract and bounded
dependencies. Inputs, outputs, ownership and effects are explicit; mutable state is scoped and
determinism is declared. Diagnostic observations explain execution without changing it. Tests of
local behavior should not require unrelated acquisition, storage or serving infrastructure.

**Audit.** What must be understood or bootstrapped for this change? Can the transformation be
exercised through its contract? Is uncertainty visible rather than hidden in ambient state?

### FP-07 — Execution fits the workload

The [Heuristics for Efficient Architecture](efficient-architecture-heuristics.md) give recognizable
patterns for applying this foundation before material execution choices become fixed.

Choose representations, algorithms, access paths, execution placement and lifecycle granularity
for the operations and input sizes the system supports. Work should follow necessary data and
dependencies, with explicit treatment of skew, intermediate size, concurrency and failure.
Avoid incidental per-item crossings, repeated whole-input processing, unnecessary materialization
and coordination. Preserve optimization opportunities in composed libraries. Reuse stable work
and established immutable properties; every extra enforcement or recovery mechanism addresses
a concrete failure or operational need. Semantic ownership and physical decomposition remain
separately changeable. A bounded refusal is honest, but does not by itself make the architecture
fit a workload it claims to support.

Assess execution fit qualitatively from relevant operations and growth/failure scenarios.
Explain material tradeoffs in plain language. This design/review consideration does not require
numerical estimates, cost models, estimators, runtime cost accounting, execution-planning
machinery, instrumentation, formal cost proofs or additional proof artifacts. Introduce such
mechanisms only for a separate concrete functional or operational requirement. Quantitative
performance/capacity claims still require measurements; semantic correctness obligations remain.

**Assessment.** What work, movement and coordination does a complete operation require? How do they
grow with relevant input dimensions and concurrency? Which work and machinery serve the result or
guarantee, and which arise only from the chosen boundaries? What simpler conforming realization
was considered?

### Foundations and supporting rules

| Foundation | Supporting rules (non-exclusive) | Architectural judgments |
|---|---|---|
| FP-01 Separation of concerns | DP-05, DP-17, DP-18, DP-19 | A1, A3 |
| FP-02 Stable contracts | DP-02, DP-08, DP-15, DP-24 | A1, A3 |
| FP-03 Composition | DP-06, DP-08, DP-13, DP-14, DP-16, DP-17 | A3 |
| FP-04 Domain model and semantic authority | DP-01, DP-02, DP-03, DP-04, DP-05, DP-06, DP-08, DP-09 | A2 |
| FP-05 Explicit structure | DP-02, DP-03, DP-07, DP-11, DP-12, DP-19, DP-20, DP-24 | A2 |
| FP-06 Local reasoning | DP-08, DP-10, DP-17, DP-18, DP-21, DP-22, DP-23 | A1, A3 |
| FP-07 Execution fits the workload | DP-03, DP-08–DP-10, DP-13–DP-16, DP-19–DP-23 | A4 |

## 3. Supporting rule index

| Pillar | ID | Principle | Level | Gate |
|---|---|---|---|---|
| A. Meaning and authority | DP-01 | One authority per fact | MUST | G1 |
| | DP-02 | Semantic distinctions are types | MUST | G2 |
| | DP-03 | Invariants have an enforcement point | MUST | G3 |
| | DP-04 | Identity is semantic and layered | MUST | G1 |
| | DP-05 | Definitions, bindings, policies, observations and results stay separate | MUST | G1, G5 |
| | DP-06 | Author structure once | SHOULD | — |
| | DP-07 | Relationships keep their kind, direction and multiplicity | MUST | G2, G6 |
| B. Derivation and computation | DP-08 | Domain operations and transformations carry contracts | MUST | G6 |
| | DP-09 | One reuse mechanism, keyed on complete dependencies | MUST | G6 |
| | DP-10 | Prepare once, execute many; cross boundaries coarsely | SHOULD | — |
| | DP-11 | Precision, approximation and determinism are contracts | MUST | G6 |
| | DP-12 | Recursion, cycles and iteration have explicit semantics | MUST | G5, G6 |
| C. Libraries and code economy | DP-13 | Library first | MUST | G8 |
| | DP-14 | Built-ins and thin adapters | SHOULD | G8 |
| | DP-15 | Qualify every library boundary | MUST | G7 |
| | DP-16 | Minimize bespoke machinery; maximize extension locality | SHOULD | G8 |
| | DP-17 | Module boundaries follow ownership | SHOULD | — |
| D. Execution and failure | DP-18 | Effects are declared; pure code stays pure | MUST | G4 |
| | DP-19 | Owned workspaces, coherent publication, explicit outcomes | MUST | G5 |
| | DP-20 | Concurrency and resources are bounded and coordinated | MUST | G5 |
| | DP-21 | Diagnostics, lineage and reproducibility are structured | MUST | G3, G7 |
| E. Evidence and evolution | DP-22 | Claims are labelled and falsifiable | MUST | G7 |
| | DP-23 | Verification matches the risk | MUST | G3, G6 |
| | DP-24 | Contracts evolve explicitly and are discoverable | MUST | G2 |

# Supporting rules

## A. Meaning and authority

### DP-01 — One authority per fact

**MUST · G1.** Every semantic fact has one authoritative declaration within a declared scope
and revision. Caches, memo tables, compiled artifacts, execution layouts, language-bridge
objects, generated code and documentation are *derived*: they identify their source revision
and derivation, can be rebuilt from declared inputs (or state the external evidence needed),
and are never edited as sources. Specialized physical representations are welcome when their
mapping to the authority is explicit. An editable projection needs an explicit inverse with
conflict checks; otherwise it is read-only.

**Audit.** For each fact the scope introduces or touches, can you name its one authority and
its update path? Is there a second writable definition — a constant, a configuration file, an
adapter default, a fixture, a document — that could disagree? Can each derived artifact be
discarded and rebuilt?

**Warning sign.** Two individually reasonable components each maintain a version of the same rule.

### DP-02 — Semantic distinctions are types

**MUST · G2.** Define meaning before storage. Distinctions that change interpretation — role,
category, unit or convention, basis, ordering, scope, shape — are types or checked references,
not primitives that are interchangeable because their encodings match. The representations
must capture the distinctions required by the domain's phenomena and operations; domain names
or matching field shapes alone do not establish an adequate model. Parse authored and
external input into those types at the boundary; make illegal states unrepresentable where
practical. Represent *not supplied*, *not applicable*, *not yet computed*, *unknown*,
*uncertain*, *invalid* and *failed* as distinct states when behaviour depends on the
difference — never by null, zero, NaN, an empty string or a missing record. Keep
decision-relevant structure in typed fields; promote an opaque payload once decisions depend
on its interior.

**Audit.** Could two values with the same primitive type be swapped without rejection or
explicit conversion? Does any consumer infer meaning from a sentinel? Does any decision parse
free text or an untyped payload?

**Warning sign.** A missing value means a default in one subsystem, unknown in another and failure in a third.

### DP-03 — Invariants have an enforcement point

**MUST · G3.** Declare local, relational, conditional and lifecycle invariants, including valid
combinations and domains — not every combination of individually valid values is valid. Assign
each invariant to the concept, relationship or operation whose meaning requires it. Each
critical invariant has a boundary where it is enforced and an observable rejection. Multiple
enforcement points may be appropriate; independently maintained definitions of one invariant are not.
Documentation, annotations and schema metadata are not enforcement. Select enforcement points
by the failure classes and trust transitions they cover. Reuse established immutable validity
where its premises remain true; additional enforcement should contribute distinct protection
or cheaper recovery.

**Audit.** For each critical invariant: where is it enforced, and what does a violation
produce? Can any write or execution path bypass it?

### DP-04 — Identity is semantic and layered

**MUST · G1.** Distinguish the enduring entity, its revision, a specialization (a definition
bound to structural choices), an instance, an execution attempt and a stored artifact. None is
derived from names, paths, row order, memory addresses or library-local indices; keep domain
identities separate from library handles and map between them explicitly. Before using content
hashes as identity, define canonicalization (ordering, encoding, defaults, numeric edge cases,
versions) and the equivalence level claimed: byte, structural, semantic or approximate.
Distinguish semantic identity, preparation/result dependencies, physical realization and full
build provenance. One digest need not serve all four purposes; source text changes affect each
only according to its declared equivalence and dependency contract.

**Audit.** Do identities survive reordering, renaming and reserialization? Is one identifier
expected to answer "which thing", "which definition" and "which attempt"? Is any result
sequence zipped against an unrelated iteration order?

### DP-05 — Definitions, bindings, policies, observations and results stay separate

**MUST · G1, G5.** What exists, how it is bound for a case, which policy selects behaviour, what
was observed and what an execution produced are separate concepts, versioned on their own
lifecycles with explicit links. A contextual role or assignment belongs to its binding rather
than becoming an intrinsic property of an entity. Genuinely authored assignments remain facts;
do not invent derivations for them. Execution never mutates the definition it executes; temporary
overrides are scoped overlays, not edits that need manual reversal.

**Audit.** Can the same definition serve a new case unchanged? Can a failed execution leave the
definition altered? Does a result ever overwrite the input it came from?

### DP-06 — Author structure once

**SHOULD.** Represent repeated structure once, as a template with declared parameters,
capability requirements and instantiation rules; instances are typed bindings that retain their
origin. Express recurring variation within a coherent semantic family through reusable
definitions, composition and contextual bindings before adding another procedural path.
Branching over a genuine domain alternative is valid; independent reinterpretation across
consumers is not. Keep compact and indexed structure until a consumer needs expansion, and make
expansion inspectable. Declare the choices an author controls, not every loop or allocation.

**Audit.** Does a new instance need new meaning, or copied construction code? Is structure
flattened earlier than any consumer requires?

### DP-07 — Relationships keep their kind, direction and multiplicity

**MUST · G2, G6.** Ownership, connectivity, dependency, dataflow, control flow, provenance and
equivalence are different relationships, each with explicit roles, direction and cardinality.
A graph or other projection states its node and edge kinds, direction, multiplicity, self-loop
and isolate policy, weight meaning and scope, and it retains every intermediate element the
requested semantics need — an output filter is not an input filter. Heuristic analyses
(communities, rankings, similarity, predicted links) are analytical results, never structural
facts, proof of independence or valid execution boundaries.

**Audit.** What does each edge mean? Would symmetrizing, collapsing parallel edges or dropping
isolates change the answer? Could a cycle cross the boundary of the analysed region? Is a
heuristic result consumed where a structural fact is required?

## B. Derivation and computation

### DP-08 — Domain operations and transformations carry contracts

**MUST · G6.** Every domain operation and meaningful pass, lowering, rewrite, conversion or
approximation has an explicit, owned contract: inputs, applicability, outputs or state changes,
invariants, effects and failure or uncertainty outcomes. Operations realize the domain model
and its composition laws where applicable. A transformation states the equivalence it promises,
the distinctions it preserves, permitted omissions, introduced assumptions and uses for which
its output remains valid. A projection need not retain facts outside its promised semantics.
Obligations a rewrite could erase — domain restrictions, ordering, precision — are captured
before rewriting and enforced after it. A deliberate change of behaviour is a selected policy,
not an "optimization". Use distinct intermediate representations where stages have different
responsibilities, so no backend-specific structure becomes the only record of intent. Behaviour
implemented by specialized code has an explicit, owned operation contract with declared
dependencies and limits, never a hidden callback. An ordinary domain function may be sufficient;
registration or a first-class operation specification is needed only when a consumer must
discover, inspect, compose, plan or persist operations in that form. For material operations,
state the relevant input universe, supported workload and completeness semantics. Distinguish
work limits from output limits, and resource exhaustion from a complete answer to a bounded
question. Qualitative assessment of execution fit is sufficient; no numerical cost model, formal
cost proof or new operation registry is required by this principle.

**Audit.** Can the operation or transformation be invoked, reused and tested without knowing
its internals? Which domain rules govern it? For a transformation, which equivalence does it
promise, under which assumptions, and what enforces them?

### DP-09 — One reuse mechanism, keyed on complete dependencies

**MUST · G6.** Reuse — memoization, caching, incremental recomputation, retained artifacts — is
owned by one mechanism per scope and keyed on every result-affecting input: structure,
parameters, policies, provider and algorithm versions, configuration, seeds, warm starts, and
membership, including additions, removals and failed lookups. Nothing reused silently observes
mutable files, "latest" references, clocks or registries. Separate structural from value inputs
so a value change does not rebuild structure; distinguish reuse of a prepared artifact from
reuse of an executed result. Equality used for reuse must make results substitutable.
Invalidate at the finest granularity the evidence supports; conservative beats unsound. Assess
material over-invalidation qualitatively, explaining the tradeoff and a refinement route where warranted.
Complete build provenance need not become every preparation or result key.

**Audit.** Could an undeclared change alter a reused result? Does a second mechanism track the
same dependencies? Would incremental output equal a clean recomputation after additions,
deletions and structural edits?

### DP-10 — Prepare once, execute many; cross boundaries coarsely

**SHOULD.** Resolution, validation, dependency analysis, compilation and layout preparation run
once per stable input and are reused across instances, cases and iterations; changing values
bind through explicit interfaces. Choose physical layouts for actual access patterns, mapped
explicitly to domain identities. Cross language, process, storage and device boundaries in
validated batches or artifacts, not per primitive operation; zero-copy is an optimization with
preconditions, not an objective. Declaration and stage boundaries do not prescribe execution
granularity. Choose indexes, shared scans, compact reusable views, late hydration, streaming or
spill according to actual operations and working sets. Avoid unnecessary passes, conversions
and intermediate materialization; scalar operations remain appropriate where their cost fits.

**Audit.** Which work depends on structure and which on values, and does the lifecycle reflect
that? Is any boundary crossed per element inside an inner loop?

### DP-11 — Precision, approximation and determinism are contracts

**MUST · G6.** State the accuracy class and equivalence required of computations and outputs:
tolerances, permitted approximation, precision changes, reduction order, stochastic behaviour,
and whether conforming runs may differ bitwise, within a tolerance or statistically. Record when
hardware, backend or thread count can change results. A seed alone does not establish
determinism.

**Audit.** Can two conforming runs differ, how, and how is conformance judged? Is an
approximation or tolerance hidden inside a contract described as exact?

### DP-12 — Recursion, cycles and iteration have explicit semantics

**MUST · G5, G6.** Detecting a cycle is not resolving it: decompose cyclic structure and give each
cyclic block explicit resolution semantics — fixed point, iterative method or simultaneous
solve. Every iterative or recursive process declares termination, convergence criteria and
behaviour on resource exhaustion; non-termination and truncation are reported as such, never
presented as a complete result. Avoid all-paths, all-pairs or full-closure materialization
unless that output is required.

**Audit.** On non-convergence or budget exhaustion, is the outcome distinguishable from success?
Does a limit truncate silently?

## C. Libraries and code economy

### DP-13 — Library first

**MUST · G8.** Generic capability — mathematics, numerics, optimization, graph algorithms,
relational processing, storage, incremental computation, serialization, parsing, concurrency,
error reporting, command-line handling — comes from established libraries. The system's own code
owns domain meaning and the composition of libraries. Bespoke generic code is a deliberate
choice made after considering established libraries, with its reason stated briefly where the
next reader will look (§F). Place each operation in the mechanism whose semantics fit it, not the
one that happens to be present. Consider the full library surface for the agreed capability,
including planned consumers. Availability does not establish a product requirement. Include
coupling, configuration, lifecycle, semantic surface and replacement cost in the choice. Evaluate
the composed capability: physical access paths, optimizer visibility, data movement, preparation,
resource behavior and recovery. Individually suitable libraries can compose into an unsuitable
execution plan. Shared kernels or engine-native execution may preserve more capability than a
lowest-common-denominator wrapper.

**Audit.** Does the scope contain generic code that an adopted or established library provides?
Is each bespoke generic component a deliberate choice with a stated reason? Is each operation
placed because its semantics fit?

**Warning sign.** A hand-written solver loop, graph traversal, cache, parser, retry framework or
derivative routine.

### DP-14 — Built-ins and thin adapters

**SHOULD · G8.** Within the standard library and adopted libraries, use the built-in function,
kernel, algorithm or idiom rather than a hand-rolled equivalent; do not re-implement what the
resolved version provides, and do not wrap a library only to rebuild its features. Adapters
translate representations and bind identifiers; domain rules, defaults, provider selection and
policy do not live in adapters. A conversion that changes interpretation is a named
transformation under DP-08. Preserve useful bulk, lazy, indexed and fused interfaces where the
workload benefits. Prefer a built-in when its semantics and total integration fit; neither a
single query nor native execution establishes an efficient plan by itself.

**Audit.** Does any loop replicate a built-in? Does an adapter contain a default or rule found
nowhere else?

### DP-15 — Qualify every library boundary

**MUST · G7.** Qualify behaviour at the resolved (locked) version and the enabled features. Establish that the specific routine's semantics,
restrictions and outputs fit the intended use — from documentation, source, types or
experience, with a probe or test only where genuine doubt remains. A library's name or
reputation alone is not enough, and an entry point's name may not match its algorithm. Keep one resolved
version of each type-sharing dependency family, and never bridge versions by layout
assumptions. Providers declare supported operations, auxiliary capabilities and limitations;
requested work is validated against them before execution, and unsupported work is rejected.
A fallback preserves semantics or is an explicitly selected policy. Selection is by declared
capability and explicit policy, persisted and reproducible — never by import order or naming.
Interchange declares what it loses (metadata, nulls, ordering, identity) and how it treats
unknown versions.

**Audit.** What establishes that the selected routine does what the design claims? Can unsupported
work fail late or silently fall back? Can installing a package change a selection?

### DP-16 — Minimize bespoke machinery; maximize extension locality

**SHOULD · G8.** Judge a design by the independent semantic decisions and bespoke code it
removes. An ordinary extension should be one authoritative declaration, any genuinely new
implementation, and focused tests where warranted (§E). Generated code is a derived artifact
(DP-01): regenerate it from its authority; never edit it or treat it as an authority.
Proportionality applies to every mechanism, including adopted libraries, registries, generators
and wrappers. Qualify their total integration burden against a current or agreed planned need.
Include generated and library-induced runtime objects, operations, lifecycle state and operator
or developer work. One declaration expanding into many tables, checks or jobs is still machinery;
count the work removed as well as the handwritten code removed.
Once a replacement lands and its callers have moved, delete the replaced
code, tests and fixtures in the same change; keep no shim or parallel path "as evidence".

**Audit.** Where must a typical extension be expressed, and is any meaning re-expressed in
several places? Does a bespoke layer lack a current requirement? Does a replaced path survive?

### DP-17 — Module boundaries follow ownership

**SHOULD.** Modules and packages are deep: small, stable interfaces over substantial behaviour,
each owning one concern, with an acyclic dependency graph. Dependencies point from mechanism to
meaning: the domain core does not depend on a particular backend, solver, store or language
bridge unless that dependency is an explicit architectural contract. Mechanisms are selected
by explicit policy and lowering. Shared library types may form an intentional boundary contract;
provider-private handles and mutable implementation details do not leak into unrelated owners.
Add an abstraction when it hides volatility, preserves meaning or permits meaningful isolated
testing. Splitting files or introducing forwarding wrappers does not by itself improve ownership.

**Audit.** Could another conforming backend consume the same intent? Does a public model type
mirror a library's internal objects? Must a caller know a module's internals to use it?

## D. Execution and failure

### DP-18 — Effects are declared; pure code stays pure

**MUST · G4.** Mark operations that perform I/O, read time or randomness, mutate authoritative
state or depend on the environment. Memoized, cached or "pure" computations observe none of
these except through declared inputs; externally required effects happen at an explicit
execution or publication boundary, because memoized bodies may be skipped or repeated.
Inspection, validation and reporting never change meaning or select behaviour. External input,
plugins and agent-proposed changes pass the same validation as any other write; data never
acquires executable authority implicitly.

**Audit.** Could an optimizer safely duplicate, reorder, cache or skip this operation? Can a
getter or diagnostic change state? Can any input bypass validation?

### DP-19 — Owned workspaces, coherent publication, explicit outcomes

**MUST · G5.** Mutable runtime state belongs to one identified attempt. Outcomes become visible
through one commit boundary — transaction, manifest or snapshot — so readers never combine
incompatible revisions; individually atomic writes are not multi-structure consistency.
Planned, running, completed, partial, stale, cancelled and failed are distinguishable; output
completeness is validated before publication; stale asynchronous results are rejected. Retries
define idempotency, and no rollback or exactly-once claim exceeds what the protocol provides.
Material workflows are explicit plans or state machines, not only statement order in a script.
Separate preparation, durable progress, sealing and visibility where the contract permits.
Match transaction/lock lifetime and retry unit to the actual effect; coherent publication need
not mean one long transaction. Before final persisted reconciliation, drain writers and freeze
the content and answer-affecting definitions being certified; then publish that same realization.
Recovery scope should bound repeated work without inventing resumability for disposable work.

**Audit.** After a crash, cancellation or retry, which outputs are valid and which effects
already happened? Can a partial output look complete?

### DP-20 — Concurrency and resources are bounded and coordinated

**MUST · G5.** Parallelize independent work, or use explicit synchronization and reduction
contracts. Coordinate nested thread pools rather than maximizing each independently. Budget
intermediates, conversions, workspaces, caches and retained artifacts together; exceeding a
budget fails explicitly. Parallel execution changes results only within the declared
determinism class (DP-11). Distinguish work, resident-state, queue, transport and output budgets.
Control admission, backpressure, cancellation/drain and contention between workload classes where
material. A small output, shallow traversal or bounded worker pool does not bound examined work,
fan-out or queued bytes. Explicit refusal establishes truthfulness, not fitness for claimed use.

**Audit.** Can concurrency change results, visible ordering or effect counts beyond the
contract? What happens at the memory limit?

### DP-21 — Diagnostics, lineage and reproducibility are structured

**MUST · G3, G7.** Diagnostics carry a stable code, severity, stage, affected domain identities,
observed values, the violated rule and a remediation hint where useful. They distinguish an
invalid model, an unsupported capability, a numerical or algorithmic failure, an infrastructure
failure and an inconclusive check; human-readable messages are projections of that structure.
Every output traces to its authored sources, selected policies and providers, transformations
and attempt, many-to-many where needed. Declare the reproducibility class — recomputation or
replay of recorded evidence — and record what it needs. Make changes explainable at the level of
meaning, and make lifecycle decisions (selection, reuse, rejection) observable without
disproportionate instrumentation. Explain complete operations and assess significant sources of
work qualitatively; cost telemetry needs a separate functional or operational requirement.
Compact provenance references and selective hydration can preserve attribution without repeatedly
moving the full evidence payload; observability cardinality should not dominate useful work.

**Audit.** Can a tool classify a failure without parsing prose? Can a bad output be traced to the
inputs responsible? What exactly can be reproduced, and what is outside that promise?

## E. Evidence and evolution

### DP-22 — Claims are labelled and falsifiable

**MUST · G7.** Every design claim carries an evidence label (§D). A proposed benefit states its
mechanism, baseline, validation method and current evidence. Performance is measured end to end
— construction, preparation, transfer, execution, publication, memory, cold and warm — at
representative scale; an isolated kernel benchmark or a language choice does not establish a
system speed-up. An unmeasured benefit is a hypothesis. A demonstrated avoidable scan, crossing
or amplification is architectural evidence even without a benchmark. Lack of a speed claim does
not exempt a supported operation from A4.

**Audit.** What observation would show the claim false, and was it sought? Do *Tested* and
*Measured* name the test or benchmark and its conditions?

### DP-23 — Verification matches the risk

**MUST · G3, G6.** Direct verification at what could actually be wrong, in proportion to its risk
and to how much doubt remains after careful reasoning. The useful kinds include negative cases
for invariants, round trips across representations, provider conformance, incremental against
clean recomputation, property-based, metamorphic or differential checks of declared
equivalences, and adversarial lifecycle cases (malformed input, empty domains, missing
capabilities, precision edges, interruption, stale caches, concurrent change, retry). None is
mandatory for every change. Agreement between two implementations is evidence, not proof, when
they share a transformation. Tests protect behaviour where regression risk warrants them; design
alignment itself is established by judgment and review (§0).

Distinguish admission/semantic verification, persistence reconciliation, certificate checking and
independent audit/testing. Assign each check the failure it can detect and reuse valid immutable
premises. A digest checks agreed content, not semantic correctness; a certificate needs a sound
checker and its required premises. Repeating the producer is not an independent semantic oracle
by itself, though rechecking can cover a new trust boundary, changed state or transient failure.
Preserve distinct protection when removing redundant recomputation.

**Audit.** For the risks that matter here, what would detect semantic drift in an adapter, a
rewrite or an alternate backend? Are the failures most likely to occur between happy-path
stages covered?

### DP-24 — Contracts evolve explicitly and are discoverable

**MUST · G2.** Version structural and semantic contracts, classify compatibility, and migrate with
recorded provenance; unknown or incompatible versions are never reinterpreted under current
defaults. Meaning-changing migrations need explicit decisions. Document the sanctioned
extension path — declaration, binding, transformation, kernel, adapter or policy — in a compact,
navigable map that references its authorities instead of restating them.

**Audit.** Can an older artifact be read under its original contract, or migrated with
documented effect? Can a newcomer or agent find the right extension point without tribal
knowledge?

# Reference sections

## §A Acceptance gates

Gates apply to the behaviour a design claims to support. Each is settled on its own evidence as
*pass*, *fail*, *unresolved* or *not applicable* (with a scope reason). **Unresolved is not a
pass, and no strength elsewhere offsets a failed gate.** Profiles add gates; they never remove
or relax these.

| Gate | Fails when… | Principles |
|---|---|---|
| G1 — Authority | Two independently mutable definitions can disagree about one fact without a reconciliation protocol. | DP-01, DP-04, DP-05 |
| G2 — Semantic fidelity | Required meaning is ambiguous, silently dropped or reinterpreted, or carried by an indistinguishable sentinel. | DP-02, DP-07, DP-24 |
| G3 — Validity | An invalid state can reach an operation that assumes validity, without a defined rejection. | DP-03, DP-15, DP-21, DP-23 |
| G4 — Hidden behaviour | Inspection, optimization, selection or an apparently pure operation can introduce undeclared effects or change meaning. | DP-18 |
| G5 — Consistency and recovery | A reader can mistake an inconsistent or partial output for a committed result, a retry produces ungoverned effects, or a limit truncates silently. | DP-05, DP-12, DP-19, DP-20 |
| G6 — Transformation and reuse | A rewrite, cache hit, projection or alternate backend changes required behaviour without a valid contract or selected approximation policy. | DP-07, DP-08, DP-09, DP-11, DP-23 |
| G7 — Truthful capability claims | A capability is claimed without an implementation route or evidence, or unsupported work silently falls back. | DP-15, DP-21, DP-22 |
| G8 — Library leverage | Generic capability is implemented bespoke where an adopted or established library clearly provides it, without a stated reason. | DP-13, DP-14, DP-16 |

A G8 failure calls for revision, not rejection: it is a cost and extensibility defect, not a
correctness defect, unless the bespoke code also fails another gate.

## §B Layering rules

1. A profile or binding may **add** principles and gates, **refine** a core principle with
   concrete requirements and audit questions, or **promote** a SHOULD to a MUST. It may not
   weaken, waive or redefine a core principle.
2. Every profile principle names the core principles it refines, or says it is new.
3. Profile IDs carry the profile's prefix (for example `PS-nn`, gates `PS-Gn`). A binding
   defines no principle IDs; it cites the repository's own authorities.
4. The core never refers to a profile or repository; a profile never refers to a repository.
5. A repository policy that conflicts with a core or profile principle is recorded in the
   binding as a known conflict with its resolution route. Until it is resolved, a review states
   which authority it followed and whether the conflict affects its decision.

## §C Placement: model, library or bespoke code

| Question | Default placement |
|---|---|
| Does it distinguish valid from invalid meaning or change an observable rule? | An authoritative domain definition or operation contract, including an ordinary domain function |
| Is it a reusable structural pattern or selection policy? | A typed template, binding or policy |
| Is it a generic capability (math, numerics, graphs, relational work, storage, parsing, caching, concurrency)? | A library, behind a thin adapter owned by one module |
| Is it a specialized domain algorithm implementing an existing contract? | Ordinary code behind that contract, after considering libraries (§F) |
| Is it a representation translation with no new domain decision? | A mechanical adapter |
| Is it an optimized layout or queryable projection? | A derived artifact with explicit dependencies and identity mapping |
| Is it an effectful action or an important execution sequence? | A typed action or workflow at an execution boundary |
| Is it a repeated mechanical expression of an existing contract? | Derived from that contract: a runtime or library mechanism, or generated code |
| Is it flexibility without a current or agreed planned requirement? | Defer it; assess library adoption and bespoke mechanisms by the same total complexity test |

## §D Evidence vocabulary

| Label | Meaning |
|---|---|
| Proposed | Described architecture or intended behaviour; not implemented. |
| Interface-checked | A required interface or integration mechanism has been inspected; end-to-end behaviour is not established. |
| Implemented | Code exists for the stated path; its testing scope must still be described. |
| Tested | Named tests exercise the stated behaviour over specified cases. |
| Measured | A named benchmark or operational observation supports a quantitative claim under recorded conditions. |
| Formally established | A specific property follows from an identified formal argument or verified method with explicit assumptions; never used for ordinary testing. |

Labels describe different claims, not one ladder: a measured implementation can still be
incorrect, and an interface-checked design can still need substantial engineering.

## §E Change locality and execution fit

Choose realistic changes from the product's next capabilities or known variation axes: a new
model, provider revision, analytic, workflow, rendering, invariant or isolated test. Assess which
owner and contracts would change, which consumers would be affected, and whether the result can
be verified locally. Investigate propagation only to the depth needed for that judgment.

Distinguish a new instance, contextual binding, composition, policy, domain concept or execution
mechanism. New instances and bindings generally reuse definitions; compositions reuse operation
contracts; genuinely new phenomena may require new concepts, invariants or algorithms. A
mechanism replacement preserves the domain contract or makes its incompatibility explicit.
Judge whether edits follow these semantic owners, not whether an extension avoids new code.

Record material propagation, independent semantic edits, hidden knowledge and test setup.
A scenario can be assessed from contracts and relevant source without implementing it or
constructing a complete flow trace.
Quantitative claims need measurements; reasoned scope analysis is useful evidence without them.
The target for an ordinary extension:

> One authoritative semantic addition, any genuinely new specialized implementation, and
> focused tests where warranted; adapters, validation, documentation and execution bindings follow
> from existing contracts and library mechanisms.

A change touching several files is not a failure. A change that re-expresses the same meaning
independently in several places is. A genuinely new core concept may legitimately require
versioned changes to the core and its backends.

Also assess relevant operation, growth, skew, concurrency and failure scenarios against a brief
workload premise drawn from functional intent: representative operations, meaningful input
dimensions, deployment constraints and resource envelope where material. Do not silently reduce
the product to convenient fixtures. Place this premise in the existing scope/scenario discussion;
it is not a new artifact or standing review obligation.

The following lenses guide judgment where material; they are not twelve mandatory checks:

Consult the [Heuristics for Efficient Architecture](efficient-architecture-heuristics.md) alongside these
lenses. Existing principles and acceptance judgments govern; the companion supplies concrete
assessment patterns without replacing this guidance.

| Lens | Architectural choice to assess |
|---|---|
| Semantic/physical independence | Lower declarations into replaceable layouts, execution plans, identity mappings and lifecycle mechanisms. |
| Necessary work | Match access paths and algorithmic work to required input and dependencies; retain a complete universe when semantics demand it. |
| Composed capability | Preserve useful optimizer, bulk and native capabilities across library boundaries. |
| Data/compute locality | Place work near suitable representations; qualitatively assess crossings, copies and serialization. |
| Working set | Bound live state, intermediate cardinality and expansion; stream, compact or spill where appropriate. |
| Amortization | Reuse preparation and immutable results under complete, appropriately scoped dependencies. |
| Assurance economy | Place checks at failure/trust boundaries; reuse established validity without losing independent protection. |
| Lifecycle scope | Separate private computation, durable progress, sealing and publication; match retry and transaction scope to effects. |
| Admission and contention | Bound queues and fan-out, coordinate capacity, and define cancellation and drain. |
| Reusable physical views | Derive indexed or compact representations for real consumers under explicit mappings; avoid parallel authorities. |
| Work and answer semantics | Distinguish examined-work limits, returned-output bounds, completeness and selected approximation. |
| Total machinery | Qualitatively assess generated/runtime objects and operator/developer obligations alongside source complexity. |

Usually remove unnecessary work first, then improve algorithms/access paths, placement and reuse,
before tuning concurrency or kernels. A clearly dominant local optimization need not follow this order.

### Architectural acceptance judgments

Use these in the review's existing decision section. They are judgments made by the reviewer,
not additional commands or mechanically scored gates. Settle each as **satisfied**, **violated**,
**unresolved**, or **not applicable** with a scope reason. Relevant violations or unresolved
judgments prevent architectural acceptance; a scoped acceptance explicitly excludes the
unsupported scenario and carries a disposition/revisit trigger. It never certifies the wider system.

| Judgment | What must be established for the selected scenarios |
|---|---|
| A1 Localize change | Coherent owners and narrow contracts constrain propagation; local behavior can be understood and tested with bounded dependencies. |
| A2 Encode domain meaning explicitly | Supported phenomena and consequential distinctions have a coherent, scoped model. Domain behavior realizes its owned definitions and operation contracts; constraints, lifecycle and dependencies are explicit, and derived representations preserve their promised semantics. Meaning is not independently reconstructed by consumers or recoverable only from incidental implementation details. |
| A3 Extend through composition | Existing capabilities can be recombined or substituted through their contracts; new semantic code has an owner, and new machinery has a credible need. |
| A4 Fit execution to the supported workload | For selected operation and growth/failure scenarios, qualitatively assess whether a credible physical route suits the workload, considering access paths, necessary versus repeated work, important crossings/intermediates, resource/transaction lifetimes and recovery scope. The composed architecture introduces no unjustified amplification or predictable failure of the supported workload. Compare a materially simpler or more native realization where credible. Static evidence can establish a structural violation; latency, throughput and capacity claims require measurements. |

Preserve each correctness/fidelity gate independently. Working behavior cannot compensate for an
architectural violation, and architectural elegance cannot compensate for a failed semantic gate.
A2 requires both model adequacy and semantic authority: one owner for an inadequate model does
not satisfy it. An in-scope violation requires revision even when current outputs are correct.
A4 cannot be satisfied by merely noting overhead or safe refusal: explain qualitatively why the
retained benefit warrants the tradeoff against a conforming alternative. A material unresolved
physical premise prevents acceptance of that use; it does not require measuring every implementation constant. Proposed architectural
acceptance remains distinct from runtime/release qualification.

## §F Library consideration

What to think through before choosing bespoke generic code (DP-13). It is not a required
artifact. When bespoke code is chosen, a brief note of the reason — in the change description,
a review or a code comment — helps the next reader; use whichever points are useful.

| Point | Content |
|---|---|
| Capability | What is needed, stated as behaviour and contract, not as an implementation |
| Owner and scenario | The boundary that owns it, its consumer or agreed planned use, and the change it supports |
| Candidates | Libraries and built-ins that plausibly provide it |
| Fit and gaps | What each provides and what it lacks for this contract |
| Composed execution fit | Access paths, optimizer visibility, preparation/reuse, crossings, working set, resource and failure behavior across the complete operation |
| Integration burden | Coupling, exposed concepts, configuration, lifecycle, generated/runtime objects, operator/developer work, test setup, compatibility and replacement cost |
| Decision | Adopt, adapt, or build — and for build, the bounded scope of the bespoke code |
| Revisit | What would reopen the decision, such as a candidate gaining the capability |

## §G Common false positives

A design is not aligned because it uses the vocabulary. Check the hidden defect behind each
attractive claim.

| Attractive claim | Hidden defect to check |
|---|---|
| "It is modular." | Modules have separate files but share private representations, policy decisions or lifecycle assumptions. |
| "It is extensible." | The next agreed extension edits independent classifiers or workflow branches across unrelated owners. |
| "The implementation is replaceable." | The interface exposes provider-private mechanics, or the proposed abstraction has no credible variation or test need. |
| "It is easy to test." | Local semantic behavior requires unrelated acquisition, storage or serving setup; assertions only repeat generated production expectations. |
| "It composes." | Orchestration reimplements invariants and defaults instead of connecting capabilities through their contracts. |
| "There is a single source of truth." | Several independently editable definitions share one storage format. |
| "We have a domain model." | Only output records are modeled; operations establish their meaning independently. |
| "The rule has one owner." | Consumers reinterpret it, or its single definition omits a consequential domain distinction. |
| "Everything is generic." | Meaningful distinctions survive only in strings and conventions inside generic records. |
| "Everything is declarative." | Callbacks or scripts independently define domain rules outside their declared contracts. |
| "The schema enforces it." | The invariant is metadata that no path rejects on. |
| "It uses library X." | A wrapper re-implements, bypasses or restricts the library's core capability. |
| "It is incremental." | A hidden read, a mutable handle or untracked membership makes reuse stale. |
| "It is generated from one source." | Generated output is edited, stale, or also restated by hand elsewhere. |
| "The algorithm succeeded." | Its status or convergence was not checked against the declared contract. |
| "Every backend is supported." | Lowerings or auxiliary capabilities exist for only some accepted operations. |
| "It is zero-copy" or "it is faster." | Ownership, conversion, construction and end-to-end costs were never measured. |
| "The outputs match." | Both paths share the same transformation or ignore the same metadata. |
| "It safely rejects overload." | The mechanism predictably cannot serve its claimed workload; safe refusal does not establish A4. |
| "The answer is bounded." | A row, depth or output-byte limit leaves examined work, fan-out, intermediates or queues unbounded. |
| "It is one native query." | The engine still performs unnecessary scans, expansion or materialization. |
| "The declarations generate everything." | Mechanical per-kind objects and operations amplify cost without a workload reason. |
| "We validate it repeatedly." | Repeated work contributes no new failure coverage; shared producer logic is mistaken for independent assurance. |

## §H Exception record

A SHOULD-level deviation records: principle IDs; scope; reason; alternatives considered;
consequence; compensating controls; evidence; accountable owner; revisit trigger. A MUST-level
gap is never an exception: it narrows the supported scope or is recorded as unresolved. A short,
concrete record is enough for a small deviation.

## §I Lineage: Data Model–Based Design Charter 1.0

**Version 3.3 lineage.** Adds FP-07 and A4 for execution fit alongside independent semantic gates.
Strengthens existing rules on physical lowering, composed libraries, reuse, assurance and lifecycle
scope. Review uses workload and growth/failure scenarios without new artifacts, permanent checks
or mandatory probes. Earlier IDs and historical review versions keep their meanings.

**Version 3.2 lineage.** Domain-model adequacy and authority remain design criteria under §1,
FP-04 and A2. Assessment belongs to bounded design and review work; mandatory tracing sequences
are removed. Reviewers choose investigation depth according to the scoped question. All IDs
retain their meanings; historical reviews retain their recorded versions.

**Version 3.1 lineage.** FP-01–FP-06, A1–A3, DP-01–DP-24 and G1–G8 retain their IDs.
§1 makes semantic-model-first domain design a MUST; FP-04 and A2 explicitly assess model
adequacy and authority over behavior. Operation contracts, invariant ownership, contextual
bindings and change classification strengthen existing rules without prescribing a modeling
mechanism. Historical reviews retain their recorded versions and are not certified under 3.1.

**Version 3.0 lineage.** Version 2.0's DP-01–DP-24 and G1–G8 identifiers remain stable.
The six FP foundations and A1–A3 architectural judgments now organize assessment. DP-13,
DP-16 and DP-17 clarify proportionality and intentional library contracts; existing citations
refer to the version recorded by their review. Historic reviews are not retroactively certified
under version 3.0. The foundation mapping in §2 relates every supporting rule to the new structure.

Every charter ID maps to a principle here, so earlier citations remain interpretable. Charter
gates G1–G7 keep their names and meaning; G8 is new. The charter's weighted assessment
dimensions are retired in favour of gate verdicts and findings.

| Charter IDs | Now |
|---|---|
| DM-01, DM-06, DM-08, DM-10 | DP-02 |
| DM-02, DM-03, DM-23 | DP-01 |
| DM-04, DM-21, DM-22, DM-24, DM-25 | DP-08 |
| DM-05 | DP-17 |
| DM-07, DM-09 | DP-03 (relationship roles: DP-07) |
| DM-11, DM-12, DM-15 | DP-04 |
| DM-13 | DP-05 |
| DM-14, DM-27, DM-29, DM-30 | DP-19 |
| DM-16, DM-17, DM-18 | DP-06 |
| DM-19, DM-42, DM-43, DM-44 | DP-15 |
| DM-20, DM-28, DM-45 | DP-18 |
| DM-26, DM-36, DM-37 | DP-10 |
| DM-31, DM-32, DM-33 | DP-09 |
| DM-34 | DP-07 |
| DM-35 | DP-20 |
| DM-38 | DP-13 |
| DM-39, DM-59 | DP-22 |
| DM-40 | DP-11 |
| DM-41 | DP-14 |
| DM-46, DM-47, DM-48, DM-49, DM-50 | DP-21 |
| DM-51, DM-55 | DP-24 |
| DM-52, DM-56, DM-57, DM-58 | DP-16 (version 3 applies proportionality to total integration burden) |
| DM-53, DM-54, DM-60 | DP-23 |
| Charter §D, §E, §F, §G, §H | §D, §E, §C, §G, §H |

## Closing standard

A good design makes valid changes easy and invalid states hard to introduce. Its explicit domain
model governs behavior and keeps meaning inspectable. It takes generic mechanisms from libraries
and keeps execution mechanisms replaceable. Its extensions add meaning rather than machinery.
Its claims match its evidence, and its remaining uncertainty is visible.
