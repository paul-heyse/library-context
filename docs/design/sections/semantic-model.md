<a id="section-15"></a>

# §15 Semantic relation model

This page owns the **target** representation of the analyzed codebase:
- one declared relation model, with a single owner for every semantic question;
- PostgreSQL as the single relational store;
- DataFusion as the in-process compute engine.

**What this page decides.** It states the vocabulary (entities, occurrences, places, calls, bindings,
transfers, conditions, obligations and derivations), the declaration contract every relation
satisfies, the named semantic policies, the transfer algebra, and the store and serving contract.

**Status.** All of it is the **accepted target, with implementation Proposed** (2026-09-29).

**Current state.** Until each layer cuts over, the implemented pipeline is the legacy one described
by §3–§14:
- `cpg-schema` contracts;
- the Delta canonical store;
- bundle import into PostgreSQL serving.

**Execution owner.** The [semantic model cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md)
owns the order, adapters, parity and deletion obligations.

**Where the requirement came from.** The
[semantic data model target review](../../design_review/reviews/design_review_semantic-data-model_2026-09-29.md)
supplies it, F01–F13 especially:
- the same semantic decision is made independently at 2–8 places each;
- places and transfers have no shared representation;
- conditions exist in two representations;
- the serving schema is a second, hand-written authority.

The review's evidence folder holds the probes.

> Decision: ADR-0082, ADR-0083, ADR-0084

<a id="section-15-1"></a>

## §15.1 Layers, owners and mechanisms

Every relation belongs to exactly one layer and has exactly one producing stage.

| Layer | Holds | Producer |
|---|---|---|
| **L0 observations** | Attributed provider assertions: syntax, declarations, Pysa call facts, ty flow facts, types, documents, package metadata, runs/contexts/producers/facts | `cpg-extract`, `cpg-flow` |
| **L1 normalized relations** | Entities, occurrences, places, call sites/targets/resolutions and call-policy views, effective callables, signatures, call bindings | Normalization stages (DataFusion, one Rust binder) |
| **L2 derived semantic relations** | Transfers, control influences, conditions and atoms, obligations, coverage, the derivation index, analysis invocations and findings | Analysis stages (`lctx-analytics` operators, DataFusion) |
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
  policies, the derivation-source registry, the stage table, and generated DDL/view text.
- **`lctx-postgres`** owns all PostgreSQL effects: the generation lifecycle, COPY, constraints, roles,
  and provider registration.
- **`cpg-core`** orchestrates stages and runs DataFusion derivations and semantic validators.
- **`lctx-analytics`** keeps pure Arrow-in/Arrow-out operators.
- **`cpg-schema`** is retired by the cutover. Its surviving wire, selection and retrieval contracts move
  to `lctx-model`.

> Decision: ADR-0082, ADR-0083

<a id="section-15-2"></a>

## §15.2 Relation declarations: the single authority

**What a declaration states.** Each relation is declared exactly once. The declaration states:
- name, layer, family and producing stage;
- typed columns: nominal IDs, codebook columns, nullability;
- the key, whose first column is always `generation_id`;
- the identity recipe (§15.3);
- **participant roles** for relationship relations, as typed references to entity/occurrence/place
  kinds;
- a **coverage scope** and **polarity**. The scope is the domain within which an absent row is
  meaningful; the polarity is may/must, and over- or under-approximation;
- a **fidelity class**: extracted, resolved, derived or heuristic;
- **serving exposure**: grant, lookup indexes, and wire exposure.

**What is generated mechanically from it**
- the Rust row type, Arrow schema and decoder;
- PostgreSQL DDL: the table partitioned by generation, primary/unique keys, generation-qualified
  foreign keys from roles, codebook foreign keys, CHECKs, NOT NULL, and declared indexes;
- identity-recompute validators;
- graph-catalog entries and derivation-source entries;
- serving views, grants and inventories;
- wire DTOs, where the row is the DTO;
- every relation-name list.

**No second copy.** No inventory, foreign-key list, codebook mapping or serving schema is written by
hand a second time. One generated `RelationId` enumeration is the registry.

**Codebooks** stay append-only `Int16`. They are published as codebook tables and referenced by foreign
keys, so the database enforces membership.

**What stays hand-written:** derivations (DataFusion SQL or Rust operators) and semantic validators
that constraints cannot express.

**The generation catalog.** `nodes`/`edges` remain a materialized, role-generated navigation catalog.
It is the reference universe for validation and ad hoc exploration. It is never the substrate for
semantic re-interpretation (§15.5). `edge_kinds` has a reader or is retired.

> Decision: ADR-0082, ADR-0083

<a id="section-15-3"></a>

## §15.3 Identity

**The recipe.** Every identifier is `BLAKE3("lctx-id/v2" ‖ kind ‖ fields…)`, truncated to 16 bytes,
where `kind` is a value of one declared `IdKind` enumeration. An ad hoc tag string does not compile.

**What goes into identity.** Identity is the **semantic key**. Provenance never enters it: path, depth,
witness, run, rendered text or provider-internal indices.

**Recipes for each kind of identity**
- **Parallel relationships** stay distinct because their identity includes the occurrence or
  discriminator that separates them. `(source, target)` is never an identity.
- **Occurrence:** `(module entity, start, end, syntax kind)`.
- **Entity:** `(kind, owner entity, name, declaring occurrence)`.
- **Entity symbol key:** a release-independent descriptor (distribution, qualified path, member
  descriptor) for cross-release and dependency joins.
- **Place:** `(root, access path)` (§15.4).
- **Atom:** `(evaluation occurrence, predicate kind, operand places and literals)`.
- **Transfer:** `(owner, in place, out place, kind, condition, context, provenance class)`.

**Generations.** A generation is one compile attempt. `generation_id` is random per attempt. Its
content digest covers relation digests, run identities and consumed-vector receipts. Producer identity
covers every canonical producer's code.

> Decision: ADR-0082, ADR-0083

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
attach to occurrences by `(module, span)` through one join function, with parity rules for name,
attribute and subscript places alike.

**Owner rule.** One owner rule assigns each occurrence its enclosing entity: the innermost declaration
whose body holds it, otherwise the module. It is the only definition of "caller".

**Places**
- A **place** is a root plus a bounded access path.
- **Roots:** formal parameter, receiver, return, yield, raise, class field, module global, or local
  binding occurrence.
- A formal of a source callable is its parameter entity. A formal of an external or modeled callable is
  `(callable, position or name)`, resolved once by the binder (§15.5).
- **Access path:** at most two segments, each an attribute name, a literal item key or any item, plus
  an explicit **unknown suffix** flag.
- **Ports** are the places on a callable's boundary.
- No other place encoding exists. Node-hex keys, name paths and rendered strings are presentations.

> Decision: ADR-0082

<a id="section-15-5"></a>

## §15.5 Calls, policies and bindings

**The call relations**
- A **call site** is an occurrence.
- **Call targets** are provider-attributed alternatives. Each carries its target entity, or its
  unresolved state, plus modality, origin, fidelity and phase.
- **Resolutions** state status, completeness under the model, remainder and reason.
- Implicit invocations, such as decorator applications, are call targets with a disclosed origin and
  implicit flag. They are never silently merged with explicit calls.

**Call policies.** "Which calls count" is decided only by **named call policies**. Each is a declared
admission predicate, compiled once to a view:

| Policy | Admits | Consumers |
|---|---|---|
| `invocation` | Analyzer assertions; definite or candidate; call and property phases; definition arcs kept separate | invocation projection, delegation |
| `dataflow` | Function targets; call/init phases | flow composition, handoffs |
| `summary` | Definite, single, complete target between callables | summary instantiation |
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

> Decision: ADR-0082

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
| `control` | The input decides which value arrives: a predicate atom whose evaluation reads the in-place |

Sequential composition is an explicit table in one module, never an order over codebook codes:
- identity · identity = identity;
- any composition involving `derived` = `derived`;
- `control` does not compose sequentially with value transfers. A control influence on an atom that
  guards a transfer yields a **selection** relation.

**Open calls and alternatives**
- **A value crossing an unresolved or unsummarized call** is not a transfer kind. It is an obligation
  (§15.8), so nothing is inferred through an unknown callee.
- **Alternatives** stay separate rows. Rows with an equal semantic key merge by OR-ing conditions and
  keeping every derivation.

**Composition across a call**
- An in-caller transfer to an argument, the call binding and a callee transfer compose, **matched by
  call site**, into a `composed` transfer.
- Its condition is the caller condition ∧ the callee condition. In the callee condition, formal atoms
  are substituted by actual atoms, and callee-local atoms are eliminated existentially under bounded
  work.

> Decision: ADR-0082

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

> Decision: ADR-0082

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
  inside the relation's declared closure.
- **Verdicts:** one verdict function maps condition, open obligations and approximation to the five
  verdicts (established, conditional, refuted under model, unknown, not analysed).

> Decision: ADR-0082

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

> Decision: ADR-0082

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

> Decision: ADR-0082

<a id="section-15-11"></a>

## §15.11 Store, compute and stages

**The store.** **PostgreSQL 18 is the single relational store.** Every canonical relation is
list-partitioned by `generation_id`, with generated constraints and codebook tables.

**The generation lifecycle** is staging → validated → published → selected → retired.
- An attempt writes into staging partitions under a writer role, by binary COPY from Arrow.
- Publication is one transaction, and only after both database constraints and DataFusion semantic
  validators pass.
- Published partitions are read-only by privilege.
- Retirement drops whole partitions.
- A failed attempt publishes nothing, and a retry is a new generation.

**Compute.** DataFusion computes derivations and semantic validators over in-memory Arrow batches
within an attempt. It reads published relations through the owned PostgreSQL table-provider fork,
with pushdown. `lctx query` runs DataFusion SQL over a pinned generation.

**Artifacts.** Arrow IPC artifacts are only derived, content-addressed caches for bulk consumers, such
as native executor inputs, with their manifests in PostgreSQL. They are never canonical.

**The stage table.** A declared **stage table** names each stage's input relations, output relations,
effect class and code identity. From it:
- the scheduler is derived;
- every output has exactly one writer;
- a read before its writer runs is an error.

Reuse keeps recompute-and-compare admission. Skipping on key stays behind ADR-0081's trigger.

**Performance** of publication and provider reads is measured at phase exits and tuned later. It is
not a decision gate.

> Decision: ADR-0083

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

> Decision: ADR-0083

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

**Temporary machinery.** Legacy adapters, legacy-ID side relations and the parity harness are
temporary migration machinery owned by the plan. They are not part of this model. Every adapter and
declared legacy quirk is deleted by the end of the final phase.

**Research engines.** Engines whose ablation leaves served output and retained controls unchanged are
retired rather than migrated, with operator confirmation.

> Decision: ADR-0084
