# Design principles

**Version 2.0 · 2026-09-24** · Core layer: repository- and domain-agnostic.
Supersedes the Data Model–Based Design Charter 1.0; its IDs map to this document in §I.

> **Author meaning once, typed and explicit. Take generic mechanisms from qualified
> libraries. Derive everything else. Back every claim with evidence.**

## 0. How to use this document

**Layers.** This is the core of a layered standard:

| Layer | Holds | May reference |
|---|---|---|
| Core (this document and the core review template) | General design principles `DP-nn`, gates `G1`–`G8`, evidence vocabulary, review contract | Nothing outside the core |
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

## 1. Governing objective

Design toward the best available design for the system's functional target. Local rules,
plan scope and conventions serve that target: where one blocks a clearly better design, record
the conflict as a required change with its route instead of treating it as a constraint.

When qualities conflict, resolve them in this order:

1. **Correctness and semantic fidelity** — meaning is explicit, enforced and preserved.
2. **Diagnosability** — failures and decisions can be explained from structured evidence.
3. **Extensibility** — an ordinary change is local: one declaration, genuinely new code, tests.
4. **Performance** — measured, end to end.
5. **Implementation convenience.**

Minimize **independent semantic decisions** and **bespoke generic code**, not source lines.
A specialized algorithm behind a complete contract is aligned; a declarative-looking wrapper
around hidden rules is not.

Before choosing technology, answer: which meanings must survive every representation; which
facts are authoritative, bound, observed or derived; which invariants and effects govern
valid change; which transformations create execution forms and under what assumptions; and
which generic capabilities an existing library already provides.

## 2. Principle index

| Pillar | ID | Principle | Level | Gate |
|---|---|---|---|---|
| A. Meaning and authority | DP-01 | One authority per fact | MUST | G1 |
| | DP-02 | Semantic distinctions are types | MUST | G2 |
| | DP-03 | Invariants have an enforcement point | MUST | G3 |
| | DP-04 | Identity is semantic and layered | MUST | G1 |
| | DP-05 | Definitions, bindings, policies, observations and results stay separate | MUST | G1, G5 |
| | DP-06 | Author structure once | SHOULD | — |
| | DP-07 | Relationships keep their kind, direction and multiplicity | MUST | G2, G6 |
| B. Derivation and computation | DP-08 | Transformations carry contracts and preserve meaning | MUST | G6 |
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

# Principles

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
not primitives that are interchangeable because their encodings match. Parse authored and
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
combinations and domains — not every combination of individually valid values is valid. Each
critical invariant has a boundary where it is enforced and an observable rejection.
Documentation, annotations and schema metadata are not enforcement.

**Audit.** For each critical invariant: where is it enforced, and what does a violation
produce? Can any write or execution path bypass it?

### DP-04 — Identity is semantic and layered

**MUST · G1.** Distinguish the enduring entity, its revision, a specialization (a definition
bound to structural choices), an instance, an execution attempt and a stored artifact. None is
derived from names, paths, row order, memory addresses or library-local indices; keep domain
identities separate from library handles and map between them explicitly. Before using content
hashes as identity, define canonicalization (ordering, encoding, defaults, numeric edge cases,
versions) and the equivalence level claimed: byte, structural, semantic or approximate.

**Audit.** Do identities survive reordering, renaming and reserialization? Is one identifier
expected to answer "which thing", "which definition" and "which attempt"? Is any result
sequence zipped against an unrelated iteration order?

### DP-05 — Definitions, bindings, policies, observations and results stay separate

**MUST · G1, G5.** What exists, how it is bound for a case, which policy selects behaviour, what
was observed and what an execution produced are separate concepts, versioned on their own
lifecycles with explicit links. Execution never mutates the definition it executes; temporary
overrides are scoped overlays, not edits that need manual reversal.

**Audit.** Can the same definition serve a new case unchanged? Can a failed execution leave the
definition altered? Does a result ever overwrite the input it came from?

### DP-06 — Author structure once

**SHOULD.** Represent repeated structure once, as a template with declared parameters,
capability requirements and instantiation rules; instances are typed bindings that retain their
origin. Keep compact and indexed structure until a consumer needs expansion, and make expansion
inspectable. Declare the choices an author controls, not every loop or allocation.

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

### DP-08 — Transformations carry contracts and preserve meaning

**MUST · G6.** Every meaningful pass, lowering, rewrite, conversion or approximation declares
inputs, outputs, preconditions, effects, failure modes and the equivalence it promises.
Obligations a rewrite could erase — domain restrictions, ordering, precision — are captured
before rewriting and enforced after it. A deliberate change of behaviour is a selected policy,
not an "optimization". Use distinct intermediate representations where stages have different
responsibilities, so no backend-specific structure becomes the only record of intent. Behaviour
outside the declarative model enters through a registered contract with declared dependencies
and limits, never through a hidden callback.

**Audit.** Can the transformation be invoked, reused and tested without knowing its internals?
Which equivalence does it promise, under which assumptions, and what enforces them?

### DP-09 — One reuse mechanism, keyed on complete dependencies

**MUST · G6.** Reuse — memoization, caching, incremental recomputation, retained artifacts — is
owned by one mechanism per scope and keyed on every result-affecting input: structure,
parameters, policies, provider and algorithm versions, configuration, seeds, warm starts, and
membership, including additions, removals and failed lookups. Nothing reused silently observes
mutable files, "latest" references, clocks or registries. Separate structural from value inputs
so a value change does not rebuild structure; distinguish reuse of a prepared artifact from
reuse of an executed result. Equality used for reuse must make results substitutable.
Invalidate at the finest granularity the evidence supports; conservative beats unsound.

**Audit.** Could an undeclared change alter a reused result? Does a second mechanism track the
same dependencies? Would incremental output equal a clean recomputation after additions,
deletions and structural edits?

### DP-10 — Prepare once, execute many; cross boundaries coarsely

**SHOULD.** Resolution, validation, dependency analysis, compilation and layout preparation run
once per stable input and are reused across instances, cases and iterations; changing values
bind through explicit interfaces. Choose physical layouts for actual access patterns, mapped
explicitly to domain identities. Cross language, process, storage and device boundaries in
validated batches or artifacts, not per primitive operation; zero-copy is an optimization with
preconditions, not an objective.

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
one that happens to be present. A library's full capability surface is eligible: the absence of
a current consumer is not a reason to write bespoke code instead of using a library mechanism.

**Audit.** Does the scope contain generic code that an adopted or established library provides?
Is each bespoke generic component a deliberate choice with a stated reason? Is each operation
placed because its semantics fit?

**Warning sign.** A hand-written solver loop, graph traversal, cache, parser, retry framework or
derivative routine.

### DP-14 — Built-ins and thin adapters

**SHOULD · G8.** Within the standard library and adopted libraries, use the built-in function,
kernel, algorithm or idiom rather than a hand-rolled equivalent; do not re-implement what the
pinned version provides, and do not wrap a library only to rebuild its features. Adapters
translate representations and bind identifiers; domain rules, defaults, provider selection and
policy do not live in adapters. A conversion that changes interpretation is a named
transformation under DP-08.

**Audit.** Does any loop replicate a built-in? Does an adapter contain a default or rule found
nowhere else?

### DP-15 — Qualify every library boundary

**MUST · G7.** Pin versions and features. Establish that the specific routine's semantics,
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
implementation, and focused tests where warranted (§E). Prefer runtime and library mechanisms to generated
static projections; generate code only where no runtime mechanism serves, and never treat
generated output as an authority. Proportionality applies to bespoke machinery — registries,
compilers, generators, services and frameworks we would write ourselves — not to adopting a
library capability. Once a replacement lands and its callers have moved, delete the replaced
code, tests and fixtures in the same change; keep no shim or parallel path "as evidence".

**Audit.** Where must a typical extension be expressed, and is any meaning re-expressed in
several places? Does a bespoke layer lack a current requirement? Does a replaced path survive?

### DP-17 — Module boundaries follow ownership

**SHOULD.** Modules and packages are deep: small, stable interfaces over substantial behaviour,
each owning one concern, with an acyclic dependency graph. Dependencies point from mechanism to
meaning: the domain core does not depend on a particular backend, solver, store or language
bridge; mechanisms are selected by explicit policy and lowering. Library types are used freely
inside the module that owns the integration, but they do not define the public semantic model
or leak into unrelated modules.

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

**Audit.** After a crash, cancellation or retry, which outputs are valid and which effects
already happened? Can a partial output look complete?

### DP-20 — Concurrency and resources are bounded and coordinated

**MUST · G5.** Parallelize independent work, or use explicit synchronization and reduction
contracts. Coordinate nested thread pools rather than maximizing each independently. Budget
intermediates, conversions, workspaces, caches and retained artifacts together; exceeding a
budget fails explicitly. Parallel execution changes results only within the declared
determinism class (DP-11).

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
meaning, and make lifecycle decisions (selection, reuse, rejection, cost) observable without
letting instrumentation dominate cost.

**Audit.** Can a tool classify a failure without parsing prose? Can a bad output be traced to the
inputs responsible? What exactly can be reproduced, and what is outside that promise?

## E. Evidence and evolution

### DP-22 — Claims are labelled and falsifiable

**MUST · G7.** Every design claim carries an evidence label (§D). A proposed benefit states its
mechanism, baseline, validation method and current evidence. Performance is measured end to end
— construction, preparation, transfer, execution, publication, memory, cold and warm — at
representative scale; an isolated kernel benchmark or a language choice does not establish a
system speed-up. An unmeasured benefit is a hypothesis.

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
| Does it distinguish valid from invalid meaning or change an observable rule? | An authoritative declaration or a registered operation contract |
| Is it a reusable structural pattern or selection policy? | A typed template, binding or policy |
| Is it a generic capability (math, numerics, graphs, relational work, storage, parsing, caching, concurrency)? | A library, behind a thin adapter owned by one module |
| Is it a specialized domain algorithm implementing an existing contract? | Ordinary code behind that contract, after considering libraries (§F) |
| Is it a representation translation with no new domain decision? | A mechanical adapter |
| Is it an optimized layout or queryable projection? | A derived artifact with explicit dependencies and identity mapping |
| Is it an effectful action or an important execution sequence? | A typed action or workflow at an execution boundary |
| Is it a repeated mechanical expression of an existing contract? | A runtime or library mechanism; generate code only where none serves |
| Is it speculative bespoke flexibility with no current requirement? | Defer it. Adopting a library capability is not speculative flexibility |

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

## §E The extension-locality test

Ask where a typical new entity, rule, provider, model or policy must be expressed. The target:

> One authoritative semantic addition, any genuinely new specialized implementation, and
> focused tests where warranted; adapters, validation, documentation and execution bindings follow
> from existing contracts and library mechanisms.

A change touching several files is not a failure. A change that re-expresses the same meaning
independently in several places is. A genuinely new core concept may legitimately require
versioned changes to the core and its backends.

## §F Library consideration

What to think through before choosing bespoke generic code (DP-13). It is not a required
artifact. When bespoke code is chosen, a brief note of the reason — in the change description,
a review or a code comment — helps the next reader; use whichever points are useful.

| Point | Content |
|---|---|
| Capability | What is needed, stated as behaviour and contract, not as an implementation |
| Candidates | Libraries and built-ins that plausibly provide it |
| Fit and gaps | What each provides and what it lacks for this contract |
| Decision | Adopt, adapt, or build — and for build, the bounded scope of the bespoke code |
| Revisit | What would reopen the decision, such as a candidate gaining the capability |

## §G Common false positives

A design is not aligned because it uses the vocabulary. Check the hidden defect behind each
attractive claim.

| Attractive claim | Hidden defect to check |
|---|---|
| "There is a single source of truth." | Several independently editable definitions share one storage format. |
| "Everything is declarative." | Callbacks or scripts still decide important behaviour. |
| "The schema enforces it." | The invariant is metadata that no path rejects on. |
| "It uses library X." | A wrapper re-implements, bypasses or restricts the library's core capability. |
| "It is incremental." | A hidden read, a mutable handle or untracked membership makes reuse stale. |
| "It is generated from one source." | Generated output is edited, stale, or preferred where a runtime mechanism exists. |
| "The algorithm succeeded." | Its status or convergence was not checked against the declared contract. |
| "Every backend is supported." | Lowerings or auxiliary capabilities exist for only some accepted operations. |
| "It is zero-copy" or "it is faster." | Ownership, conversion, construction and end-to-end costs were never measured. |
| "The outputs match." | Both paths share the same transformation or ignore the same metadata. |

## §H Exception record

A SHOULD-level deviation records: principle IDs; scope; reason; alternatives considered;
consequence; compensating controls; evidence; accountable owner; revisit trigger. A MUST-level
gap is never an exception: it narrows the supported scope or is recorded as unresolved. A short,
concrete record is enough for a small deviation.

## §I Lineage: Data Model–Based Design Charter 1.0

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
| DM-52, DM-56, DM-57, DM-58 | DP-16 (reframed: proportionality limits bespoke machinery, not library use) |
| DM-53, DM-54, DM-60 | DP-23 |
| Charter §D, §E, §F, §G, §H | §D, §E, §C, §G, §H |

## Closing standard

A good design makes valid changes easy and invalid states hard to introduce. It keeps meaning
inspectable, takes generic mechanisms from libraries, and keeps execution mechanisms
replaceable. Its extensions add meaning rather than machinery. Its claims match its evidence,
and its remaining uncertainty is visible.
