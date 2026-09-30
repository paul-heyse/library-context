<a id="section-15"></a>

# §15 Semantic relation model

This page owns the **target** representation of the analyzed codebase:
- one declared relation model, with a single owner for every semantic question;
- PostgreSQL as the single relational store;
- DataFusion as the in-process compute engine.

**What this page decides.** It states the vocabulary (entities, occurrences, places, calls, bindings,
transfers, conditions, obligations and derivations), the declaration contract every relation
satisfies, the named semantic policies, the transfer algebra, and the store and serving contract.

**Status.** **Accepted target** (2026-09-29). Phase 0's contracts are **Implemented /
focused-Tested**:
- typed model, stages and memory generation;
- resource accounting;
- declarations, call-site facts and the owner rule;
- stability witnesses and whole-call composition;
- the facts frontier and admission;
- the stage-bound capture and syntax subset.

The assembled P0 exit review accepted them, scoped, on 2026-09-29. It excludes the P4 composition
engine and the store-side frontier enforcement (P1.7). Exact contracts, commands and exclusions are
in cutover plan §4.2; finding dispositions are in §8. The generation store and complete facts producers are **Implemented / Tested**,
2026-09-30, after the complete gate, both full profiles, repeated behavioral content and refusal
controls. The [assembled P0–P2 exit review](../../design_review/reviews/design_review_p0-p2-exit_2026-09-30.md)
is **Accept scoped** for facts, with explicit resource allowances and downstream exclusions.
Commands and measured envelope are in the plan. Phase 3 runtime foundations (R1–R3) are **Implemented / focused-Tested**, 2026-09-30, with [scoped independent acceptance](../../design_review/reviews/design_review_phase3-foundation_2026-09-30.md). Normalized producers and Phase 3 exit qualification remain in progress; Phases 4–5 remain **Proposed**.

**Current state.** The Delta canonical store and its compile orchestration were removed in cutover
phase 1 (P1.3/P1.4). The legacy `cpg-schema` contracts and the analysis, catalog and serving code
described by §3–§14 remain dormant until phases 3–5 rebuild them.
`lctx compile --through facts` publishes either facts profile and never selects it.
`lctx-model` owns all facts contracts; extraction and native flow have no `cpg-schema` dependency.

**Execution owner.** The [semantic model cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md)
owns the execution order, qualification and deletion obligations.
The [Phase 3 detailed plan](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md)
develops normalized relations and their runtime prerequisites. Its ADR-0101/0102 choices are accepted targets; they do not extend the facts-only implementation receipt.

**Where the requirement came from.** The
[semantic data model target review](../../design_review/reviews/design_review_semantic-data-model_2026-09-29.md)
supplies it, F01–F13 especially:
- the same semantic decision is made independently at 2–8 places each;
- places and transfers have no shared representation;
- conditions exist in two representations;
- the serving schema is a second, hand-written authority.

The review's evidence folder holds the probes.

> Decision: ADR-0085, ADR-0086, ADR-0087, ADR-0092, ADR-0094, ADR-0095, ADR-0096, ADR-0097, ADR-0098, ADR-0099, ADR-0100

<a id="section-15-1"></a>

## §15.1 Layers, owners and mechanisms

Every relation belongs to exactly one layer and has exactly one producing stage.

| Layer | Holds | Producer |
|---|---|---|
| **L0 observations** | Source artifacts and occurrences; attributed provider assertions, provider conditions and coverage: syntax, declarations, Pysa call facts, ty flow facts, types, documents, package metadata and invocation context | `cpg-extract`, `cpg-flow` |
| **L1 normalized relations** | Normalized entities and occurrence ownership, places, call sites/targets/resolutions and call-policy views, effective callables, signatures, call bindings | Normalization stages (DataFusion, one Rust binder) |
| **L2 derived semantic relations** | Transfers, control influences, derived conditions over the shared atom vocabulary, obligations, analysis coverage, the derivation index, analysis invocations and findings | Analysis stages (`lctx-analytics` operators, DataFusion) |
| **L3 product relations** | Catalog contracts, requirements, associations, scenarios, deployment evidence, selection domains, retrieval units | Catalog and retrieval stages |
| **L4 serving** | Generated views, grants and indexes over one pinned generation; derived artifact caches | Generated from declarations |

**Rules for the layers**
- L2 and L3 relations are canonical, published relations. Later stages consume them exactly like L1.
- Projections, operators and caches are **execution mechanisms**, not layers.
- Three graphs stay distinct:
  - program projections (§15.10);
  - the derivation index (§15.9);
  - the stage table (§15.11).

**Crate owners**
- **`lctx-model` (new)** owns the pure contracts: relation declarations and registry, identity kinds,
  vocabulary types, named policies, transfer algebra, the condition kernel, the obligation and verdict
  policies, the derivation-source registry, the stage table, and validated model metadata. PostgreSQL lowering belongs to `lctx-postgres`.
- **`lctx-postgres`** owns all PostgreSQL effects: the generation lifecycle, COPY, constraints, roles,
  and reader leases. `cpg-core` owns DataFusion provider registration so the Python storage wheel
  does not link the compute engine.
- **`cpg-core`** orchestrates stages and runs DataFusion derivations and semantic validators.
- **`lctx-analytics`** keeps pure Arrow-in/Arrow-out operators.
- **`cpg-schema`** is retired by the cutover. Its surviving wire, selection and retrieval contracts move
  to `lctx-model`.

**Phase 3 generation closure (Accepted target; Proposed implementation evidence, 2026-09-30).** Facts contains L0; Normalized contains
L0+L1 in one self-contained generation. A model-owned frontier descriptor declares relation
closure, coverage, checkpoints and admission. Fresh normalized compilation runs the combined
schedule without intermediate publication or selection. P4/P5 extend the declared closure;
external linked generations are not a P3 input mechanism.

> Decision: ADR-0085, ADR-0086, ADR-0101

<a id="section-15-2"></a>

## §15.2 Relation declarations: the single authority

Ordinary Rust domain structs and tagged enums in `lctx-model` are the semantic authority.
They define attributes, typed participant roles, key participation, provenance, fidelity and coverage.
A bounded derive generates the typed key, identity recipe, physical Arrow codec and metadata. The
root manifest supplies membership once. Storage and execution require a privately constructed
`ValidatedModel`. There is no independently maintained row DSL, schema or foreign-key inventory.

`Id<T>` identifies its target nominally. A reference may also be provenance; these are independent
field properties. Reference collections become first-class relationships. Tagged sums distinguish
inactive arms from present optional values and reject malformed payloads. Subtype references enforce
subtype membership. Same-key conflicting payload is an error unless an explicit model merge owns it.

`lctx-postgres` lowers the validated model into ordinary tables in each generation schema, with
qualified keys, references, checks and indexes. Explicit Arrow schemas drive `serde_arrow`; no sample
inference defines a contract. Codebooks remain append-only Int16 values. Semantic validators and
computations stay handwritten where they express actual behavior, consuming the same typed contracts.

> Decision: ADR-0085, ADR-0086

<a id="section-15-3"></a>

## §15.3 Identity

One generated typed semantic key supplies equality, hashing and BLAKE3 identity. The
encoding is structural and length tagged, with a model namespace and nominal type discriminator.
Entity identity, qualified proposition identity and support/run identity are distinct. A condition,
modality or approximation qualifies an assertion; additional supports do not strengthen it.

Input revisions identify canonical manifests of analyzer-visible bytes, independently of acquisition
labels or absolute checkout locations. Package/version releases, installed verification and acquired
input revisions have separate identities. Installed, source-tree and corpus acquisition origins are
explicit alternatives; multiple distributions and corpus attribution remain relationships. Original
artifact bytes survive independently of interpretation as canonical 1 MiB `ArtifactChunk` records.
Acquisition captures the complete analyzer-readable input (site-packages sources and stubs, `.pth`
and `py.typed` files, distribution `METADATA` and entry points, and selected corpus files). Every
captured byte a `RECORD` lists is verified against the frozen copy; a `RECORD` itself is digested,
not captured, because its console-script lines carry the environment's location. Derived
artifacts, such as Markdown Python blocks and task receipts, are written only into a reserved
`_lctx/` namespace of the frozen capture with typed provenance; the source tree is never written.
Every captured artifact has one ownership class: owned by verified distributions, unowned (a loose
file, an unowned stub, a tree or corpus file) or derived (`DerivedArtifact`: document, derivation,
ordinal and fence). Only derived artifacts live under `_lctx/`, each derived from an original
document of its own input. The model refuses a second class; facts admission refuses an artifact
with none. An `EnvironmentFingerprint` states an installed input's environment in the terms a
deployment receipt reports (Implemented, focused Tested 2026-09-29, plan A1).
Empty artifacts have no chunks; the last chunk alone may be shorter. A streaming model-owned
validator reconstructs the full byte length and digest before publication (ADR-0088). The model owns cross-relation reconciliation
of the stored artifacts with the input manifest.

Source artifacts anchor occurrences by half-open byte span, syntax kind and structural discriminator.
Spans must fit their source bytes; valid empty spans remain representable. Typed roles and structural
paths distinguish same-span events. Module and
source identities are acyclic; `.py` and `.pyi` remain distinct. Provider variable indices never
identify atoms. An atom names its actual evaluation occurrence and predicate. Parallel relationships
include the discriminator that separates them. Transfer aggregation keys exclude the accumulating
condition; the owned merge joins conditions and retains all support.

Generation IDs identify attempts. Model, physical schema, producer and content digests have separate
meanings. Content equality excludes runtime timestamps and measurement data.

> Decision: ADR-0085, ADR-0086, ADR-0088, ADR-0089

<a id="section-15-4"></a>

## §15.4 Entities, occurrences and places

**Entities** are relatively stable semantic objects: modules, classes, functions, parameters, fields,
types, external symbols and synthetic callables.

**Occurrences** are source events:
- call sites, arguments, bindings and uses;
- returns, yields and raises;
- predicate evaluations;
- attribute accesses and subscripts;
- decorator applications, imports and annotations.

**Who owns occurrence identity.** The Pyrefly/Ruff parse owns occurrence identity. ty observations
attach by source artifact, span, syntax kind and structural role through one indexed join, checked
against a scalar oracle for names, attributes and subscripts. Only an exact match attaches; any
other outcome records a subject boundary with its candidates and Partial coverage. Ambiguity stays
unresolved.

**Typed syntax** (Implemented, focused Tested 2026-09-29, plan A3). Provider-qualified records carry:
- an occurrence's placement in the parse (`SyntaxPlacement`: parent, field and ordinal);
- the detail its bytes do not state (`SyntaxDetail`: an operator kind or a parsed literal);
- the syntax of declarations and decorators, import aliases, `__all__`, parameters (a literal default
  as its value) and class fields.

Texts are the source bytes at their occurrences. One geometry invariant keeps every part inside its
declaration, statement or class, with the kinds the parse gives. A `SubjectBoundary` sits under its
provider's partial, unavailable or failed coverage of its scope and family, with its subject inside
that scope. An `AttachmentOutcome` is disclosed by its matching reason (`AttachmentAmbiguous`,
`AttachmentUnmatched`, `BudgetReached`) and keeps exactly its candidates: one innermost container,
two or more when ambiguous, none when unmatched.

**Provider modules.** Symbols and type variables name a provider module:
- the acquired module over captured bytes;
- a stub in one of the provider's bundles (typeshed, typeshed third-party, or the provider's own
  third-party stubs);
- a namespace package, which has no bytes, in one analysis context;
- an unresolved spelling in one analysis context.

The same spelling in different origins is never one module.

**Symbols** (Implemented, focused Tested 2026-09-29, plan A6). A provider states:
- that it defines a symbol, nested in a class or function of the same module or at top level;
- a function's and a class's native traits;
- a class's bases and its MRO, whose linearization is complete, a recovery prefix, or cyclic with
  no ancestors;
- a parameter annotation's display;
- public names with a traced or untraced origin;
- docstring parameter documentation;
- how it resolved a module it references.

Every symbol a record names belongs to the provider and context of its subject. A record is
*about* its subjects, which lie in its scope. It *refers to* its referents: a re-export's origin, or an
inherited field's declaration. A referent belongs to the asserting provider and to bytes the invocation
captured, wherever those lie. An annotation display
is display-only and establishes no structure; types are type observations. Qualified names,
signature counts and a module's distribution are derived.

**Types** (Implemented, focused Tested 2026-09-29, plan A8).

A type term is Pyrefly's structure. Codes keep the existing type kinds, and forms the old kinds told
apart only by display text have their own codes. Those forms are:
- anonymous `TypedDict`s;
- recursive alias references;
- enum and `LiteralString` literals;
- `TypeForm`;
- the forms of a type variable, such as `P.args`.

Callables, parameter lists and anonymous `TypedDict`s hold ordered, named and kinded slots. Other
children are content-addressed sequences whose roles each form checks. Structural identity carries
no rendering; an alias's display name is a presentation. A closure that reaches an opaque or
truncated leaf, at any depth, is supported only as display-only.

The types family also states:
- test operands;
- what a provider resolved about a `def`'s body;
- a record class's fields in field order, each with exactly the flags its record model defines.

**Owner rule.** One owner rule assigns each occurrence its enclosing entity: the innermost declaration
whose body holds it, otherwise the module. It is the only definition of "caller".

**Places**
- A **place** is a root plus a bounded access path.
- **Roots:** formal parameter, receiver, return, yield, raise, class field, module global, local
  variable (its scope-opening occurrence and name), expression occurrence, or parameter entry value.
  Every definition and use of one local variable shares its place.
- **Formal versus entry value.** The formal root is the parameter *variable*: flow facts and guards
  name it, and the body may rebind it. The **entry** root is the value the parameter held when the
  callable was entered, which is the caller's value. A declared callable's receiver is written as
  the entry value of its first parameter. The receiver root is reserved for callables without a
  declared first parameter (modeled and external summaries). Only entry roots are ports of a
  declared callable (§15.6).
- **Reaching.** A use is reached by a definition of its place, by an unbound path, or by a binding
  made from a nested scope (`nonlocal`/`global` writes in a closure, lazy snapshots). The nested
  reach names no definition of the use's scope and is never a parameter reach. Writes through frame
  objects (`f_locals`, PEP 667) are dynamic access, outside the stated model.
- A formal of a source callable is its parameter entity. A formal of an external or modeled callable is
  `(callable, position or name)`, resolved once by the binder (§15.5).
- **Access path:** at most two segments, each an attribute name, a literal item key or any item, plus
  an explicit **unknown suffix** flag.
- **Ports** are the places on a callable's boundary: entry values, the receiver, return, yield and
  raise.
- No other place encoding exists. Node-hex keys, name paths and rendered strings are presentations.

**Phase 3 identity realization (Accepted target; Proposed implementation evidence, 2026-09-30).** Declaration occurrences establish
callable/class identities and cross-provider correspondence. Source and stub declarations stay
distinct, with evidenced correspondence relationships. Synthetic and external identities remain
provider-qualified until a supported correspondence exists. Total resolution outcomes retain
conflicting and missing alternatives. Public exposure is a relationship, not another identity.
Existing P2 place identities remain authoritative; normalized entity links add correspondence
without recoding places. The shared occurrence-owner rule supplies execution ownership.

> Decision: ADR-0085, ADR-0089, ADR-0102

<a id="section-15-5"></a>

## §15.5 Calls, policies and bindings

**The call relations**
- A **call site** is an occurrence.
- A **call event** is a site plus its **call origin**. The origin is the empty sequence for the
  explicit call, or the ordered desugaring steps of an implicit one: `for` calls `__iter__` and then
  `__next__` at its iterable, which are two events. Events at one site are distinct calls, never
  alternatives of one call (Implemented, focused Tested 2026-09-29, plan A7).
  - Steps run from the outermost context to the implicit operation.
  - A format string's calls have their own steps.
  - The origin is the one authority on whether an event is implicit; a support's origin states
    only who asserts it.
- **Call targets** are provider-attributed alternatives of one event. Each carries:
  - its destination: one symbol; a dispatch set, never one callee; or unresolved, with the model's
    reason and the provider's native one;
  - modality, fidelity and phase;
  - the receiver class the provider resolved it through, and the native receiver evidence its
    classified receiver comes from.
- **Resolutions** state status, completeness under the model, remainder and reason, per event.
- A **provider call site** keeps the provider's native classification of an event: its identifier and
  callee record kinds, whether an attribute access reads a plain attribute, and the callable whose graph
  reports it. That caller is the provider's attribution; the owner rule stays the model's caller.
- Implicit invocations, such as decorator applications, are call targets with a disclosed origin and
  implicit flag. They are never silently merged with explicit calls.
- An override dispatch set is the named method or any override of it in a class extending the
  target's receiver class, which the set requires. It makes its event dispatched, never unique.
  - Until P4 expands it from complete MROs, the invocation view admits its named member.
  - Dataflow and summaries do not bind through it.
  - Composing it yields an `OverrideDispatch` obligation.
- A receiver follows Pysa's implicit-receiver rule. A method called on an object, or a class method
  called on a class, receives that expression. A class method called on an object receives the
  object's class, which no actual denotes, so it is Unknown. A stored receiver is the one its stored
  native evidence classifies.

**Call policies.** "Which calls count" is decided only by **named call policies**. Each is a declared
admission policy, exposed through one generated view:

| Policy | Admits | Consumers |
|---|---|---|
| `invocation` | Analyzer assertions; definite or candidate; call and property phases; definition arcs kept separate | invocation projection, delegation |
| `dataflow` | Direct function invocation targets; call/init phases | flow composition, handoffs |
| `summary` | One normalized, complete target alternative between callables | summary instantiation |
| `usage` | Definite or candidate, with declared origins | usage ranking |
| `association` | All origins, with origin and implicit flag disclosed | catalog evidence |

A consumer selects its policy view. It never re-filters targets or the catalog.

**Effective callables.** The effective callable of each declaration is owned by the single decorator
and wrapper normalization. It carries the effective signature variants.

**Call bindings.** One pure binder in `lctx-model` produces **call bindings**, keyed by
call site × target alternative × signature variant × actual → formal place. Each binding has a kind
(positional, keyword, default, varargs, kwargs, receiver, implicit) and a status (bound, ambiguous,
unmapped, refused). Signature alternatives stay separate rows. Independent alternatives never jointly
establish one valid invocation.

**Phase 3 policy and binding realization (Accepted target; Proposed implementation evidence, 2026-09-30).** Assemble complete normalized
events and their alternatives before applying policy; preserve context, origin, channel, phase,
open remainders and qualifications. One model evaluator stores total policy assessments and
admissions. Generated views select these memberships by policy code; they do not reimplement
the predicates. Association retains Potential evidence without granting invocation or summary
admission. Keep source signatures separate from effective-callable assessments; current producer
evidence leaves arbitrary decorator transformations explicitly unknown. Recognizing descriptor
metadata does not itself admit a body.

The sole binder consumes complete raw variants and actuals. Store successful and refused attempts,
their exact premises and member-set digest. Shared validation replays the binder; only a validated
complete event, ownership and binding can create the private composition-admission token for P4.
Source-inspection binding is distinct from established effective-invocation authority. The token
also requires the exact effective target/context/descriptor and compatible variant to admit the
body; a source signature that binds beneath an unknown wrapper grants no body-composition right.
Unique binding requires one Bound variant and all other declared variants ProvenIncompatible;
an Undetermined variant, including native-unavailable or unsupported arguments, prevents uniqueness.
Normalized signature applicability replaces raw provider-symbol equality at the binder boundary:
both attributed owners must resolve to the same callable in the same input/context, retaining
their evidence and effective-signature authority. One argument algorithm remains behind that check.

> Decision: ADR-0085, ADR-0102

<a id="section-15-6"></a>

## §15.6 Transfers and their algebra

**The relation.** A **transfer** states that a value at an in-place reaches an out-place. It records:
- the owner callable;
- a kind;
- a condition;
- a context: none, or the call site of an instantiated or composed transfer;
- a provenance class;
- a verdict;
- a supporting derivation.

**Provenance classes** are `flow_local`, `derived_summary`, `composed`, `authored_model`,
`provider_summary` (Pysa taint-in-taint-out) and `catalog_field_link`. Authored models are data: they
are keyed by symbol key, and a provider's summary may disagree with ours, visibly.

**Kinds and composition**

| Kind | Meaning |
|---|---|
| `identity` | The same value arrives unchanged |
| `derived` | A value computed from the input arrives |


Sequential composition is an explicit table in one module, never an order over codebook codes:
- identity · identity = identity;
- any composition involving `derived` = `derived`;
- **ControlInfluence** is a separate relationship between a place and a predicate evaluation.
  It does not compose sequentially with value transfers. An influence on an atom guarding a
  transfer yields a first-class **Selection** relationship.

**Open calls and alternatives**
- **A value crossing an unresolved or unsummarized call** is not a transfer kind. It is an obligation
  (§15.8), so nothing is inferred through an unknown callee.
- **Alternatives** stay separate rows. Rows with an equal semantic key merge by OR-ing conditions and
  keeping every derivation.

**Composition across a call**
- An in-caller transfer to an argument, the call binding and a callee transfer compose, **matched by
  call site**, into a `composed` transfer.
- Its condition is the caller condition ∧ the callee condition. In the callee condition, formal atoms
  are substituted only with an established binding and stability witness. Opaque local guards remain
  conditional; unsupported substitutions yield an obligation. All actual/formal bindings contribute
  separate outputs. Access paths compose only through identity transfers.
- **Callee transfers are port summaries.** Composition maps only entry roots through the binding. Their producer justifies each such root by the same entry-value evidence a stability
  witness states for a guard (parameter-only reaching under complete flow coverage) at the access it
  summarizes. The stored check of that justification lands with the first producer of callee
  transfers (P4).
  - A transfer that reads the formal *variable* composes to an `EntryValueUnknown` obligation.
  - Writing a whole parameter slot composes to nothing: rebinding the variable, assigning an entry
    root, or storing one element of a collected `*args`/`**kwargs` aggregate.
  - A write below an entry value (`t.x = v`, `t[k] = v`, `kw['k'].x = v`) mutates the caller's
    object. Below the variable it is `EntryValueUnknown`.
- **Totality.** Roots resolve through one map over the bound signature variant: each parameter has
  exactly one declaration link and at least one binding.
  - The receiver is the first parameter's entry value, whether bound as receiver (`obj.m(v)`) or
    positionally (`C.m(obj, v)`). A receiver root is refused.
  - A missing link is `NoSourceDeclaration`.
  - A root of another callable, such as a closure read of an enclosing parameter, is not a port:
    nothing the caller delivers enters through it. A callee *output* rooted there is refused until
    P4 decides closure writes.
  - A value delivered into an unresolved target's call, as an argument or as its receiver, is that
    target's obligation.
- **Strength.** The composed modality is no stronger than the caller, the callee or the target
  alternative, and Definite only for a Summary-admitted target with one binding variant. The
  approximation joins all three.
- **Stored equation.** A stored `CallCompositionStep` is re-derived at validation: the composed
  condition must equal caller ∧ callee with each callee atom replaced by its unique restatement at
  the site. Where signature variants restate one guard differently, the composed condition names
  the restatement. The input must be the caller's, extended only through identity. The output must
  be the site, one of its actuals or its receiver, or a field or global.
- **Open (P4).** Instantiation inside a strongly connected component mints a fresh atom per pass and
  cannot reach a fixpoint. A widening decision precedes the SCC engine design.

> Decision: ADR-0085

<a id="section-15-7"></a>

## §15.7 Conditions

**The canonical form** is a reduced ordered BDD over atoms (§15.3), persisted as conditions, Merkle-id
nodes and atoms. A library variable index is never an identity. Same-spelling tests at different
occurrences stay distinct atoms. Cross-site equality needs an effect-stability witness.

**The bounded kernel owns the operations:**
- `and`, `or`, `not`;
- `given`, verified by a factor and quotient check, otherwise `not_factored`;
- implication (incompatibility of `a ∧ ¬b`) and compatibility;
- bounded substitution and existential elimination.

**Kernel limits**
- Every operation is preflighted against support/node budgets and node-capped. A budget hit is a
  distinct `budget_reached` obligation, never `false`.
- The typed primitive theory admits only its closed whitelist of exact origins.

**Display.** A **rendering** is presentation only: a bounded DNF read from capped satisfying paths,
with a truncation marker. Served conditions carry `condition_id` together with their rendering. No
parallel DNF computation exists.

> Decision: ADR-0085

<a id="section-15-8"></a>

## §15.8 Obligations, coverage and verdicts

**Obligations.** An **obligation** records what an established conclusion still lacks:
- its subject, as a relation and row;
- its kind, from one codebook that unions the former boundary and refusal reasons;
- the domain that would discharge it;
- its status: open, discharged or refused;
- the discharging derivation.

**One owner for each policy**
- **Obligation priority:** one owner orders obligations. Producers, validators and the native executor
  share that ordering and the discharge-validity check. There are no restatements.
- **Budgets:** named budgets charge deterministically. Exhaustion is `budget_reached(kind)`,
  distinguishable from dominance and from evidence.
- **Coverage:** coverage is stated per family and scope. A negative claim requires complete coverage
  inside the relation's declared closure. Every fact family is a coverage family. Each assertion
  relation, and its support, declares the family whose coverage states its completeness.
- **Verdicts:** one verdict function maps condition, open obligations, coverage, approximation and
  the claim's modality to the five verdicts (established, conditional, refuted under model,
  unknown, not analysed).
  - A candidate or potential alternative is never established, conditional or refuted. It is
    unknown (`NonDefiniteAlternative`) until discharge over every alternative of its site.
  - A refutation under partial coverage is unknown (`IncompleteCoverage`).
  - Priority names the most specific cause first.

> Decision: ADR-0085

<a id="section-15-9"></a>

## §15.9 Derivations, findings and witnesses

**Derivation sources.** Each proof or step relation declares itself as a **derivation source**,
stating:
- its conclusion relation and column;
- its rule;
- its premise columns and roles.

**The generated index.** From these, PostgreSQL views `derivations` and `derivation_premises` are
generated. It supports explanation lookup ("why", "why unresolved") and reverse dependency. The rules:
- Alternatives are separate derivations of one conclusion.
- Premise graphs are acyclic.
- The typed step relations and their sealed digests remain the premise payloads.

**Findings.** Analysis findings come from one emitter. Invocations record their input invocations.

**Witnesses** are selected at serve time from derivations, under response budgets. A smaller budget
never removes an established conclusion.

> Decision: ADR-0085

<a id="section-15-10"></a>

## §15.10 Projections and analytics

A projection is generated from role declarations and a call policy. It states:
- its universe, separate from its selector;
- arc sources, direction and parallel-arc policy;
- its unresolved representation;
- typed (not free-text) policies.

**The runtime.** One projection runtime builds a shared dense index and canonical adjacency in both
directions. It uses arc IDs as `petgraph::Graph` weights and composes petgraph filtered and reversed
views.

**Who uses it**
- It serves **topology analyses only**: delegation traversal, ranking, communities and the SCC
  schedule. Relational questions stay SQL.
- The SCC schedule uses `kosaraju_scc` with the canonical callee-first order.
- Heuristic analytics remain governed and never reach a served claim as fact.

**Phase 3 projection realization (Accepted target; Proposed implementation evidence, 2026-09-30).** Typed endpoint roles in the relation
model supply projection structure. Preserve isolates, parallel arcs, canonical ordering, evidence
membership and unresolved side records. Dense petgraph indices stay private to a bounded immutable
adapter; no separate graph schema or materialized path closure becomes semantic authority.

> Decision: ADR-0085, ADR-0102

<a id="section-15-11"></a>

## §15.11 Store, compute and stages

**The store.** PostgreSQL 18 is the single relational store. Each generation owns an ordinary
schema, without parent partitions. A stable control schema owns lifecycle state, manifests and selection.
Generation-qualified local references support cycles; tables/keys are created before foreign keys.

**Lifecycle:** staging → sealed → validated → published; failed and retired are terminal. Selection
is a separate pointer. Seal waits for active writes and revokes writer access. Required validators
inspect stored sealed contents; receipts bind the exact contents and complete validator set.
Publication changes state and reader grants atomically. Readers verify digests and hold leases;
retirement requires an unselected generation and exclusive access, then drops its schema atomically.
Every generation belongs to the attempt that registers it: only that attempt advances it, its
lifecycle connection holds the attempt lock until the attempt ends, and a refusal at any step records
the generation failed (terminal, abort only) with a typed class. A failed attempt publishes nothing
and retry creates a new generation; a generation whose attempt vanished is listed as interrupted.
A facts generation lowers only the facts relations, and readers verify its digests and live columns
before any scan. `store check` compares the live catalog with a rolled-back shadow lowering; `store
reset` removes one generation per transaction and is resumable.

**Compute.** DataFusion computes derivations and semantic validators over in-memory Arrow batches
within an attempt. It reads published relations through the owned PostgreSQL table-provider fork,
with pushdown. `lctx query` runs DataFusion SQL over a pinned generation.

**Artifacts.** Arrow IPC artifacts are only derived, content-addressed caches for bulk consumers, such
as native executor inputs, with their manifests in PostgreSQL. They are never canonical.

**The stage table.** A typed **stage table** is the sole writer authority and names each stage's input relations, output relations,
contributions, effect class and code identity. From it:
- the scheduler is derived;
- every output has exactly one writer;
- a read before its writer runs is an error;
- a stage contributing shared vocabulary hands budget-reserved rows to that relation's writer,
  which runs after every contributor and emits each identity once.

A stage that reports coverage names its provider; the provider is part of the schedule digest.

**The facts frontier** (`domain::admission`). A facts generation publishes only facts relations,
which reference only facts relations. One table states, for each family:
- its grain: once per input, or once per input artifact of a class (Python source or document);
- the profiles that request it: Flow in the behavioral profile only;
- whether facts are admissible when it is entirely unavailable: Artifacts and Syntax are required.

*Preflight* refuses a schedule before any store effect when:
- it reads or writes above the frontier;
- it attempts a family its profile does not request;
- it leaves a requested family uncovered;
- a family's assertions are written by a stage that does not report that family's coverage, or an
  unrequested family's assertions are written at all;
- it has no coverage writer.

*Admission* reads the sealed inputs, artifacts and coverage and requires exactly the expected rows:
- one row per scope, requested family and covering provider;
- one `NotRequested` row per scope of an unrequested family;
- no missing, extra or duplicate row, and no `Failed` status.

Each stage's reported outcome must agree with the coverage its provider stated. Each family's
availability is recorded as Complete, Partial, Unavailable, NotRequested, or no scope. The
admission record binds the contract, model, schedule, coverage and content digests, and only
admission constructs it.

Reuse keeps recompute-and-compare admission. Skipping on key stays behind ADR-0081's trigger.

**Performance** of publication and provider reads is measured at phase exits and tuned later. It is
not a decision gate.

**Compile outcomes (Implemented, ADR-0094).** The generation registry, failure record and live
attempt lock own new compile outcomes. `cpg-core::facts` never writes the retained `lctx_ops`
attempt/events; `runs` reads those historical records. `generation list|show` reports current
compilation. Required producer failure removes its generation and every registry record;
preflight refusal has no generation. Publication never selects. No durable history is promised
for a removed generation or an acquisition failure before registration.

**Completed-stage reads (Accepted target; Proposed implementation evidence, 2026-09-30).** A cumulative attempt freezes each completed
stage's outputs atomically with receipts and importer read grants, after draining readers/writers.
An immutable validated facts checkpoint precedes normalization. Private source-bound input
capabilities carry completed content and scoped availability; published-reader authorization stays
separate. Read leases last through drain, and query execution admits the full physical plan's scan
capacity before any scan starts. One attempt budget covers acquisition through publication.
The [detailed protocol](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#6-cumulative-frontiers-and-private-stage-reads)
owns the implementation sequence and controls; none of this is included in P0–P2 qualification.

> Decision: ADR-0086, ADR-0089, ADR-0094, ADR-0101

<a id="section-15-12"></a>

## §15.12 Serving

**One pinned generation.** A server process pins one published generation and reads it directly.
- Serving shapes are generated views over canonical relations, with grants and lookup indexes. They are
  not copied tables.
- Wire DTOs derive from relation rows or declared mappings.
- The native executor reads its inputs from the generation, optionally through a derived artifact
  cache.

**Integrity rules**
- There is no second schema authority and no import pipeline.
- A missing or corrupt required relation or artifact is a refusal, never an empty answer.
- Explanations traverse the derivation index with bounded recursive queries.

> Decision: ADR-0086

<a id="section-15-13"></a>

## §15.13 Migration boundary

**Hard layered cutover.** The cutover is layered and hard. Its phases are:
1. core contracts;
2. store;
3. facts;
4. normalized relations;
5. analysis and catalog;
6. serving.

A layer is complete only when it contains no legacy code.

**Clean reconstruction.** No adapters, legacy-ID side relations, compatibility flags or dual
stores are retained. Phases 0–2 restore model/store/facts; analysis, catalog and MCP stay unavailable
until phases 3–5. Availability is recorded per generation; missing capabilities never become empty
answers. Independent semantic expectations and protected evidence survive implementation deletion.
Deleting an engine does not retire its capability obligation; that needs a separate consumer decision.

> Decision: ADR-0087

### Facts scope and native identity (Accepted target, 2026-09-30)

Artifact uses request analysis; captured dependency files supply supporting lookup context. The
model derives one root universe for producers and admission. Signatures additionally state Input
coverage of the supporting definitions referenced by those roots (imports, re-exports, call targets
and nominal type references), not complete dependency analysis. NotRequested coverage has no
provider or invocation. Native caller identity distinguishes module/class bodies and decorator
applications from ordinary symbols. Named callable types retain nominal provider-qualified
Function/Method references and validate their transitive ownership. Assembly owns shared vocabulary.
New compile outcomes belong to generation control; operation attempts remain historical only.

> Decision: ADR-0092

**Native parameter order (Implemented, 2026-09-30; ADR-0095).** Signature ordinals retain provider
order. Positional-only and positional-or-keyword slots may interleave in native method reports;
their kinds remain distinct for keyword binding. Variadic/keyword groups, names and defaults
retain their validation; no sorting or relabeling is performed.

**Unavailable native signatures (Implemented, 2026-09-30; ADR-0096).** Native slots that fail
bindable list grammar retain their ordered shapes and displayed annotations under
`SignatureForm::NativeUnavailable`. Producers report partial signature coverage and a boundary;
binding refuses the variant with `OutsideProviderModel`. Slots are never deduplicated, renamed or
attached to source parameters through this form. Resource and operational ceilings remain fail-stop.
> Decision: ADR-0096

**Typed dictionary fields (Implemented, 2026-09-30; ADR-0097).** Anonymous TypedDicts reference
`TypedDictFieldList` with ordered `TypedDictField` members. Arbitrary string keys, requiredness and
term references retain mapping meaning; callable parameter-name rules belong to callable slots.
The shared membership validator refuses duplicate keys, gaps and content changes in memory and
PostgreSQL. Nominal TypedDict field observations also retain arbitrary string keys.
> Decision: ADR-0097

**Native slot interpretation (Implemented, 2026-09-30; ADR-0098).** Structural native shapes retain
empty text; bindable signature/type lists refuse empty parameter names. Unavailable native
signatures preserve slots and refuse binding. Unavailable type callables preserve members and
returns with display-only support and Partial coverage; type closure enforces that fidelity.
> Decision: ADR-0098

**Unicode values (Implemented, 2026-09-30; ADR-0099).** Literal strings and arbitrary mapping/native
slot names use `Utf8Text`: valid Unicode owned once, lowered mechanically to Arrow Binary and
PostgreSQL bytea. UTF-8 decoding rejects invalid bytes; NUL is preserved. The semantic text key
encoding and string/byte-literal distinction remain unchanged. Identifier and display columns keep
their declared text contracts. The shared round-trip controls and facts pilots own qualification.

> Decision: ADR-0099

**Residual type ports (Implemented, 2026-09-30; ADR-0100).** Bound-method function, overload signature
and generic parameter/body ports retain explicit Other/Truncated children from the bounded native
builder. Role/order/nonempty checks remain; ordinary wrong arms refuse. Shared transitive support
requires DisplayOnly and matching provider/context throughout the residual closure. These envelopes
do not establish invocation or type-variable meaning for the unresolved child.

> Decision: ADR-0100
