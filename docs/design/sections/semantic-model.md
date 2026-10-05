<a id="section-15"></a>

# §15 Semantic relation model

This page owns the **target** representation of the analyzed codebase:
- one declared relation model, with a single owner for every semantic question;
- PostgreSQL as the single relational store;
- DataFusion as the in-process compute engine.

**What this page decides.** It states the vocabulary (entities, occurrences, places, calls, bindings,
transfers, conditions, obligations and derivations), the declaration contract every relation
satisfies, the named semantic policies, the transfer algebra, and the store and serving contract.

**Status, 2026-10-01.** Accepted single-model/store target under ADR-0085/0086/0087.
Facts storage and producers are **Implemented / Tested** within the
[2026-09-30 facts receipt](../../design_review/evidence/2026-09-30_facts-qualification/README.md).
Normalized relations and runtime foundations are **Implemented / focused-Tested** with the
[Phase 3 qualification](../../design_review/evidence/2026-09-30_phase3-qualification/README.md);
its automated gate excludes stopped library comparisons and measurements.

Phase 4 analysis/catalog, immutable vocabulary groups, typed Local/execution/models/Summary,
structural/optional analytics, catalog/evidence/selection, synthesis and retrieval are
**Implemented / Tested within the accepted Phase 4 scope**, 2026-10-01. Mapped ownership retirement and Q0 acceptance are complete; the
[current qualification receipt](../../design_review/evidence/2026-10-01_phase4-qualification/README.md)
records the complete functional gate and composite hygiene. Bounded antecedent receipts
are in the [Phase 4 plan](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md).
The [foundation review](../../design_review/reviews/design_review_phase4-foundation_2026-09-30.md)
is **Accept scoped**. The [P4B3-F01 corrective reinspection](../../design_review/reviews/design_review_phase4-symbolic-reinspection_2026-10-01.md)
and [assembled Design/Target review](../../design_review/reviews/design_review_phase4-assembled_2026-10-01.md)
are **Accept scoped**, 2026-10-01; Q0 passed within the recorded finite envelope.
Phase 5 current serving is **Implemented / scoped qualification; real-library qualification stopped**, 2026-10-02. No live embedding, real-library upper-frontier
pilot, comparative product, hydration-cost or total-RSS measurement is claimed.

**Current state.** PostgreSQL is the sole canonical relational store; DataFusion supplies in-process
compute. `lctx compile --through facts|normalized|analysis|catalog` is implemented for either profile
and never selects the generation. Upper-frontier runtime controls passed within the recorded scope, 2026-10-01. Catalog Flow
is `NotRequested`; behavioral uncertainty remains explicit. `lctx-model` owns all semantic contracts;
extraction/native flow have no `cpg-schema` dependency. Legacy Phase 4 engine retirement is complete within the exact mapped scope after independent
replacement controls; Q0 passed within the recorded scope, 2026-10-01. Phase 5 has retired `cpg-schema`, the dormant `cpg-core::bundle` and their obsolete serving/IPC consumers; current qualification is pending.

**Execution owner.** The [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md)
owns layer sequencing, cross-phase finding disposition and deletion obligations. The Phase 5 and incremental alignment plans
own their current qualification and remaining work; STATUS is the restart/handoff entrypoint. Canonical
catalog construction is independent of optional analysis success and brief selection. Exact
source/effective defaults, signature variants, predicate-specific selection closure and original
evidence remain typed. Symbolic source associations retain Unknown/no-proof reader outcomes;
known field location never establishes allocation, alias, mutation or temporal value identity.

**Implemented incremental foundations; integrated acceptance pending, 2026-10-04:** the [target-alignment series](../../plans/target-implementation-alignment-plan_2026-10-04.md)
integrates faithful native Bound/unattached candidate formulas, finite publication/closure expansion,
one physical-layout owner and explicit identity encodings. Candidate attachment failure must not
become Unbound; retained formulas do not authorize Entry proof. Exact validation epochs and sufficient
grants remain distinct. Existing declared schema/identity changes and their qualification remain
pending; plan publication does not qualify the model. The coordinator owns the transferred findings.

**Accepted canonical validation target; implementation in progress, 2026-10-04 (ADR-0126).**
Validation definitions have stable semantic IDs/revisions, complete ordered relation/prefix inputs
and one constructor each. Records and stage uses reference definitions rather than reconstructing
them. Definition/ref/input identity participates in model identity. Unresolved references, conflicting
definitions and missing required premises refuse model construction; finite models do not silently
run a smaller replay. Source-call headers, invocations and arguments reference the same replay
obligation. Store execution/acknowledgement remains an effect of this model, never an independent
semantic registry. [Assurance coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md)
owns implementation and acceptance.

> Decision: ADR-0126

One model-owned embedding specification/value/text/admission/consumption contract serves analytics
and retrieval. PostgreSQL retains immutable cache winners; attempts publish exact consumed values
and consumer receipts. Replay is service-free. Fake effects qualify only that seam and cache behavior.

**Where the requirement came from.** The
[semantic data model target review](../../design_review/reviews/design_review_semantic-data-model_2026-09-29.md)
supplies it, F01–F13 especially:
- the same semantic decision is made independently at 2–8 places each;
- places and transfers have no shared representation;
- conditions exist in two representations;
- the serving schema is a second, hand-written authority.

The review's evidence folder holds the probes.

> Decision: ADR-0085, ADR-0086, ADR-0087, ADR-0092, ADR-0094, ADR-0095, ADR-0096, ADR-0097, ADR-0098, ADR-0099, ADR-0100, ADR-0121

<a id="section-15-1"></a>

## §15.1 Layers, owners and mechanisms

Every relation belongs to exactly one layer and one semantic production owner. Ordinary relations
have exactly one producing stage. **Implemented / focused-Tested, 2026-10-01 (ADR-0105):** the finite shared vocabulary has one assembly owner and
one writer per declared immutable epoch. Ordinary relations retain one producing stage;
§15.11 specifies the publication/read boundary.

**Implemented / focused-Tested, 2026-10-01 (ADR-0108):** shared analysis contracts have finite nominal
instances at immutable producing owners. Preflight-known definitions remain early one-shot rows;
late invocation, support, coverage and proof records belong to their actual publication boundary.
Mechanical declarations share semantic operations. Their reference sums and invariant inputs name
only native evidence, completed predecessors and their own finite proof occurrences.

| Layer | Holds | Producer |
|---|---|---|
| **L0 observations** | Source artifacts and occurrences; attributed provider assertions, provider conditions and coverage: syntax, declarations, Pysa call facts, ty flow facts, types, documents, package metadata and invocation context | `cpg-extract`, `cpg-flow` |
| **L1 normalized relations** | Normalized entities and occurrence ownership, places, call sites/targets/resolutions and call-policy views, effective callables, signatures, call bindings | Pure model normalization/one binder; model-owned relational membership views |
| **L2 derived semantic relations** | Transfers, control influences, derived conditions over the shared atom vocabulary, obligations, analysis coverage, the derivation index, analysis invocations and findings | Pure model and `lctx-analytics` kernels; selected model-owned relational compute |
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
- **`lctx-model`** owns the pure contracts: relation declarations and registry, identity kinds,
  vocabulary types, named policies, transfer algebra, the condition kernel, the obligation and verdict
  policies, the derivation-source registry, the stage table, and validated model metadata. PostgreSQL lowering belongs to `lctx-postgres`.
- **`lctx-postgres`** owns all PostgreSQL effects: the generation lifecycle, COPY, constraints, roles,
  and reader leases. `cpg-core` owns DataFusion provider registration so the Python storage wheel
  does not link the compute engine.
- **`cpg-core`** orchestrates stages, adapts charged inputs/outputs and registers DataFusion providers/logical views. Pure model operations own current semantic derivations and validators; suitable relational lowerings remain model-owned alternatives.
- **`lctx-analytics`** keeps pure native concept, ranking, neighbour and SCC kernels; model-owned contracts govern their inputs and results.
- **Python** is a thin validated transport and numerical library adapter. `lctx-model::domain::serving` owns wire mappings, cursors and ranking; `native_requests` owns finite exact-input semantics. `lctx-postgres::generations` owns their generation-bound effects.

**Phase 3 generation closure (Implemented / focused-Tested, 2026-09-30; automated gate passed; pilot measurements unqualified).** Facts contains L0; Normalized contains
L0+L1 in one self-contained generation. A model-owned frontier descriptor declares relation
closure, coverage, checkpoints and admission. Fresh normalized compilation runs the combined
schedule without intermediate publication or selection. P4/P5 extend the declared closure;
external linked generations are not a P3 input mechanism.

> Decision: ADR-0085, ADR-0086, ADR-0105, ADR-0108

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

**Accepted target, adoption in progress, 2026-10-03.** Every assertion qualification requires an
explicit canonical assumption set, including the empty set. Supported lower observations and
pinned model definitions supply conditional premises without content-key cycles. Shared conjunction
unions compatible sets; alternative union retains distinct governing bases. Transfer, Summary and
served claims preserve and resolve the same basis. Typing-based refinement does not erase the
conservative runtime alternative; question policy owns admission. Q0 acceptance remains pending.

> Decision: ADR-0120

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

**Accepted alignment contract, 2026-10-04; assembled qualification pending.** Declaration and
mapping/preparation compatibility use explicit framed scalar/role/reference/code/sum encodings,
with versioned namespaces. Fields retain declaration order and sum arms numeric-code order.
Incidental Arrow presentation metadata does not define a semantic contract. The conservative
artifact fingerprint still includes raw Rust sources, manifests and lockfile; formatting and
revision changes can invalidate it. Existing ordered whitespace-free embedding-specification JSON
remains its byte identity contract. Native candidate kind/formulas survive attachment failure;
no Bound/unattached evidence becomes Unbound or an Entry witness.

> Decision: ADR-0124

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

**Who owns occurrence identity (Implemented, 2026-10-04 source inspection).** Independent latest
Ruff over original captured bytes owns canonical occurrence identity. Pyrefly retains its embedded
Ruff parse for native answers; those nominal AST families never cross the adapter boundary. ty
observations come from the declared transformed runtime view and attach to original occurrences by
source artifact, mapped span, syntax kind and structural role through one indexed join, checked
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

**Normalized entities** (Implemented, focused Tested 2026-09-30, Phase 3 N1).
The model owns nominal source, synthetic and external identities, total symbol resolutions,
correspondence evidence, occurrence ownership, parameter/field links and public exposures.
Source/stub anchors stay distinct; agreement on a source declaration can join providers, while
external spelling cannot. Missing source attachment is unresolved.
Only Definite, Exact, unconditional declaration/native-origin premises establish correspondence;
tentative candidates and their support remain inspectable without resolved identity authority.
Pyrefly's declaration map distinguishes synthesized methods from callable-valued source fields;
its `is_def_statement` flag alone cannot establish that distinction. The completed-stage driver
and shared exact-output invariant enforce the same contract. N1 is a tested internal stage;
normalized publication is implemented; pilot-scale qualification is bounded by detailed plan §11/Q.

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

**Phase 3 identity realization (Implemented, focused Tested 2026-09-30; N1/N2).** Declaration occurrences establish
callable/class identities and cross-provider correspondence. Source and stub declarations stay
distinct. Cross-provider correspondence requires supported source anchors. Synthetic and external identities remain
provider-qualified until a supported correspondence exists. Total resolution outcomes retain
conflicting and missing alternatives. Public exposure is a relationship, not another identity.
Existing P2 place identities remain authoritative; normalized entity links add correspondence
without recoding places. The shared occurrence-owner rule supplies execution ownership.

N2 preserves total lexical, import-module, ancestry and mention outcomes, named structural type
links, native type-binder outcomes, place links and exact test-leaf/type links. Original observations,
all candidate members and ordered ancestry evidence remain referenced. Binder coordinates apply
only to their explicit acquired provider module; synthetic and external coordinates do not attach
by offset. Test operand matches require exact occurrence, context and role, with coverage premises
and typed missing outcomes. Catalog has no flow-leaf read; its checkpoint establishes NotRequested.
Shared exact-output validators reject missing outcomes/candidates and altered operand/type links.
These stages and normalized publication have focused controls; full phase exit remains Q.

> Decision: ADR-0085, ADR-0089, ADR-0103

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

**Phase 3 callable realization (Implemented, focused Tested 2026-09-30; N3).**
`normalized::callable_normalization` owns one operation for materialization and shared exact-output
validation. It emits total callable/context assessments and raw-signature variants, ordered raw
slots and parameter-entity premises. Effective identity, signature availability, descriptor metadata
and body admission have separate Known/Unknown/Conflicting outcomes. Complete source syntax is
required to establish an empty decorator chain. A single bare builtin descriptor needs the N2
lexical result and agreeing native traits; arbitrary and stacked decorators retain both orders and
source signatures with effective uncertainty. Exact descriptor metadata alone cannot establish
body admission: it also needs a compatible structural signature and exact, non-excluded native
body evidence. Candidate/approximate evidence cannot grant this authority. Receiver adjustments
are presentation metadata; raw formals remain intact for N5's sole argument algorithm. Default
slots identify definition-time slots without asserting runtime values. The charged pure operation
is invoked through the common completed-stage reader and resource owner. Catalog serving now
hydrates canonical typed relations; the inventoried dormant later-layer consumers were retired
in Phase 5. Serving activation and real-library qualification remain stopped.

**Call bindings.** One pure binder in `lctx-model` produces **call bindings**, keyed by
call site × target alternative × signature variant × actual → formal place. Each binding has a kind
(positional, keyword, default, varargs, kwargs, receiver, implicit) and a status (bound, ambiguous,
unmapped, refused). Signature alternatives stay separate rows. Independent alternatives never jointly
establish one valid invocation.

**Phase 3 event and policy realization (Implemented, focused Tested 2026-09-30; N4).** Assemble complete normalized
events and their alternatives before applying policy; preserve context, origin, channel, phase,
open remainders and qualifications. One model evaluator stores total policy assessments and
admissions. Generated views select these memberships by policy code; they do not reimplement
the predicates. Association retains Potential evidence without granting invocation or summary
admission. Keep source signatures separate from effective-callable assessments; current producer
evidence leaves arbitrary decorator transformations explicitly unknown. Recognizing descriptor
metadata does not itself admit a body. Raw provider/site and resolution supports remain explicit;
missing declared provider resolution evidence keeps an event open. `CompleteEvent` is privately
constructed from the full normalized event and can be recovered only by shared validation, never
from a policy-filtered query. The former public raw-symbol Summary route is removed. Flow-path
links retain each qualified path and ordered argument/callee step, matching only the explicit event
in the same context and recording missing correspondence. They establish no transfer. PostgreSQL
views and DataFusion logical views use the same model-owned membership query. The latter require
completed sources including the event invariant owner; inlining preserves physical-scan admission.

**Stored binding realization (Implemented, focused Tested 2026-09-30; N5).**
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
`normalized::binding_normalization` stores every alternative/variant/syntax attempt and its raw
argument digest, receiver, authority, outcome and formal-slot members. Effective variant-set
classification includes every refusal and requires complete scoped signature coverage. Different
syntax reports remain alternatives. Implicit events do not reuse explicit syntax; annotation calls
do not establish runtime invocation authority. The public verifier replays N1–N4 and the sole
binder before issuing `ValidatedBoundCall` and `CompositionAdmission`; arbitrary stored lower
assessments or member rows cannot mint these tokens. P4 still owns the composition engine cutover.

**Implemented / Interface-checked, 2026-10-04:** source-body composition may use an exact,
complete published Source-role signature enumeration for the selected callable when unrelated
artifact signatures are unavailable. Shared replay checks every selected member, digest and support
and the unique binding rules; a private closure token retains that evidence. Independently Known
effective identity, compatible descriptor and admitted Known body, canonical source definition,
complete original call-target event and Summary admission remain required. Missing/foreign or
unavailable selected members, ambiguity, unknown wrappers and bodies refuse the token. Global
`BindingSetAssessment` and `EffectiveInvocationAdmission` completeness do not change. This applies
ADR-0122's scoped-versus-family distinction to declared signature closure without substituting Flow
origin evidence. The analytical-enrichment coordinator owns implementation and qualification.

> Decision: ADR-0085, ADR-0103, ADR-0123

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

**Implemented, 2026-10-01.** Entry-value witnesses name a typed native execution-domain source.
Use reads, Value transfers and Guard identity each replay their own exact observation/support pair;
Value and Guard additionally retain the same-owner statement region. Guard identity must hold
before both truth arms, while the derived control proposition retains the leaf truth condition.
A narrower Use proof cannot establish guard stability, and a Value/Guard proof cannot stand in for
a bare-name evaluation. Binding identity never establishes mutable object-state stability.
The direct Local producer publishes attributed transfer/control assessments and explicit unsupported
boundaries; catalog compilation publishes NotRequested without requesting Flow. Its broader
behavioral domain retains explicit Partial refusals; later execution and Summary owners do not retroactively strengthen Local coverage.
**Implemented / focused-Tested, 2026-10-01:** native checked-false direct identity candidates retain
their actual use and complete exact-false reaching set, including Unbound alternatives, with no
loop-header expansion. Live/approximate sets keep their existing pruning. Entry still refuses
missing, multiple or unbound origins. Exact finite negatives keep the same qualified source,
proof and complete-coverage requirements through Summary/S0; silence is not a negative.

> Decision: ADR-0122
The canonical Local definition includes finite scalar type theory, conservative nominal class
reasoning and exact field-location witnesses. An open runtime class domain cannot prove a negative;
a field location does not establish allocation, alias or mutation stability. **Focused-Tested,
2026-10-01:** BaseEvaluation publishes a complete recomputed expression/refusal inventory from
completed Local/native frames, reuses a charged syntax index, and checks the actual profile before
acknowledgment. Catalog produces NotRequested; behavioral refusals remain explicit. Completion,
source-call certification and enriched execution are separate later owners. BaseCompletion now
publishes a separately replayed statement/refusal inventory from earlier BaseEvaluation outcomes,
including finalizer replacement order and profile-bound NotRequested results (**focused-Tested,
2026-10-01**, 4 actual PG + 9 native + 3 outcome controls). Exact attribute evaluation retains the
explicit heap-state prerequisite. The detailed plan owns these receipts; scoped assembled acceptance passed, 2026-10-01.

**Symbolic source associations (Implemented / Tested within the recorded scope,
2026-10-01).** The normalized callable-metadata stage owns source class/store/reader
associations for both profiles. `normalized::symbolic_fields` admits the bounded standard-record
subset through exact decorator/options, defaults, generated signatures and source membership.
Local adds exact Entry/Flow store premises. Summary retains separate uncertain reader alternatives:
Unknown, proof=None, with the actual refusal. C1 consumes the same normalized association directly.
The symbolic depth-two marker counts the source route constructor→field→reader; it is independent
of the finite call-path proof limit and never denotes a proved runtime transfer.
Constructor and reader owners and reader conditions stay explicit even when their Artifact scopes
coincide. This does not establish temporal heap identity. The [restart checkpoint](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#133-restart-checkpoint-and-remaining-work)
owns integration, native/store controls and corrective reinspection of P4B3-F01.

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
- **Implemented / Tested within the finite contract, 2026-10-01 (ADR-0106).** Keep exact invocation-distinct guards within the existing
  depth-8/proof-step-64 evidence envelope. Semantic equality excludes proof ancestry and cost;
  stable finite residual keys expose uncovered recursion without erasing guards and claiming
  Exact. Separate evidence-occurrence DAGs from aggregate summary publication. An open residual
  prevents universal discharge; an exact finite witness retains its own qualified meaning.
  [Plan §6.4–§6.5](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#6-behavioral-evidence-and-finite-composition)
  owns the implemented finite engine and its independent native/store controls. Production defaults remain depth eight and proof steps sixty-four; bounded fixtures do not qualify every recursive program.

**Implemented / focused-Tested, 2026-10-04 (ADR-0123).** Source composition witnesses retain
the exact selected signature enumeration and its support when scoped declared closure is used;
global closure leaves both references absent. Shared replay reconstructs this pair, and generic
explanation follows the enumeration's member/signature premises. Actual PostgreSQL explanation,
JSON round-trip and coupled-witness corruption controls passed within the analytical-enrichment
coordinator's receipt. Global signature-family and effective-invocation completeness remain unchanged.

> Decision: ADR-0085, ADR-0106, ADR-0123

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

**Implemented, 2026-10-03.** Conditional conclusions preserve the canonical premise
set through composition and rendering (§15.3); missing or incompatible premise evidence
refuses the operation rather than supplying an empty unconditional basis.

> Decision: ADR-0085, ADR-0120

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

**Implemented / focused-Tested, 2026-10-01 (ADR-0106).** Derived support cites nominal AnalysisInvocation and typed premises,
separately from native provider support, under one qualification operation. **Implemented:** finite composition
witnesses cite earlier witnesses and source evidence; completed summary aggregates cite witnesses
after SCC closure and never serve as their own proof premises. Coverage membership is retained
independently of witness deduplication.

**Implemented / focused-Tested, 2026-10-01 (ADR-0108).** Invocation/support/coverage and
proof families are nominal per producing owner, with one qualification and coverage policy.
The generated index validates their combined DAG. Later discharge references earlier obligations
without extending completed membership. S0 alone writes the shared finding family; earlier stages
publish qualified conclusions for its emitter. A final read-only union does not become another
writable authority or grant access to unfinished owners. Bound selectors are declared once in
`analysis::expected` and each producing family accepts only its implemented finite methods.
Unbound nonempty invocations refuse activation. Local consumes normalized dispatch evidence
directly; no unused Dispatch analysis parent or fabricated completion is required. Invocation and
coverage predecessors, admissible derived-proof sources and inherited obligation sources have
independent declared routes. Production relation membership contains only actual writers; specialized
execution/analytic proofs do not require empty generic obligation/derivation families. Shared
foundation declarations remain usable by their scoped conformance controls. Declared analysis producers and S0 now consume these owners; their scoped assembled qualification passed, 2026-10-01.

**Witnesses** are selected at serve time from derivations, under response budgets. A smaller budget
never removes an established conclusion.

> Decision: ADR-0085, ADR-0106, ADR-0108

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

**Phase 3 projection realization (Implemented / focused-Tested, 2026-09-30; automated gate passed; pilot measurements unqualified).** Typed endpoint roles in the relation
model supply projection structure. Preserve isolates, parallel arcs, canonical ordering, evidence
membership and unresolved side records. Dense petgraph indices stay private to a bounded immutable
adapter. Every fresh normalized generation builds all named projections and stores their actual
petgraph objects as versioned Serde/Postcard snapshots in bounded PostgreSQL BYTEA chunks. Analysis
hydrates these immutable computational objects. Canonical normalized records retain semantic
authority; snapshot validation checks the canonical projection. Rebuild unconditionally for each
new collection lifecycle, with no graph reuse hash or incremental cache. The wrapper binds input,
context, projection/format version and petgraph version; graph indices never become domain IDs.
No separate graph schema or materialized path closure becomes semantic authority.

**Implemented / focused-Tested adapter, 2026-09-30 (ADR-0106).** Algorithms can borrow generatively
branded native visit views after hydration; canonical SCC ordering uses this interface without
reconstructing edges from SQL. **Implemented / focused-Tested, 2026-10-01:** collection preparation
retains hydrated graphs and their reservations; each later borrower needs matching completed-source
permits and the same budget. The cumulative compilation driver wires those lifetimes to the current Structural and Analytic consumers; actual upper-frontier controls passed within the recorded scope, 2026-10-01.
Dispatch expansion precedes the graph build on every fresh collection.
**Implemented / focused-Tested, 2026-10-01:** projection version 3 gives source callable
definitions their own role and arc identity from normalized declaration ownership. Ordinary
containment retains occurrence endpoints; definition edges never become runtime calls. Bounded
delegation borrows both projections, preserving canonical witness ties, open dispatch and limits.
**Implemented / focused-Tested, 2026-10-01:** typed early analytics configuration retains authored
roots, ordered configured seeds, bounds and the complete resolved technique set. The pure parser
rejects contradictory flags and missing technique dependencies before effects. Final seed selection
belongs to S0 after optional analytics; the mandatory C0 catalog owns public path expansion.
Structural Pass A and direct usage are **Implemented / focused-Tested (2026-10-01)** through real
PostgreSQL in both profiles. C0 owns public access candidates; selected settings and release/module
membership define the structural scope. Prepared graphs are borrowed by production; publication
replay recomputes exact path, boundary, usage and invocation inventories from completed inputs.
Path steps retain their own conditions and phases: structural reachability does not establish
execution or normal completion. S0 selects final brief seeds independently of the mandatory catalog. Optional topology/concept analyses publish explicit selected or NotRequested outcomes.
**Implemented / Tested within the recorded controls, 2026-10-01:** numeric conversions have explicit universe/weight/lineage contracts and share
the attempt budget. Optional analytics retain their existing disabled defaults.

> Decision: ADR-0085, ADR-0103, ADR-0106

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

**Compute — Implemented / source-inspected, 2026-10-04; current full gates pending.** Most
current derivations and semantic validators invoke pure model kernels over admitted charged typed
Rows. Model-owned validated-membership SQL is installed in PostgreSQL; DataFusion logical-view
registration has bounded inspected test consumers. DataFusion reads published relations through
the owned PostgreSQL table-provider fork with admitted pushdown; `lctx query` runs SQL over a
pinned generation. Model-owned in-memory Arrow/DataFusion plans remain suitable for selected
bulk operations when order, multiplicity, nulls, decoding, caps and refusals are qualified.

**Scoped CI-07 SHOULD rationale, source assessment 2026-10-04.** For existing finite nominal
semantic operations, pure charged kernels preserve typed refusals, deterministic work bounds and
store-free testing with less integration burden than a query-engine rewrite. Alternative:
model-owned in-memory relational lowering, eligible where it simplifies an actual operation.
Consequence: bespoke kernels remain, with shared model validators and independent finite/native
controls as compensating checks. Accountable owner: the model operation/compute owner. Reopen on
a named bulk-operation gap or measured attempt-budget/maintenance cost. This rationale waives no
MUST and is neither a blanket DataFusion exclusion nor an exemption for generic duplicates.
The [foundation plan](../../plans/model-contract-execution-alignment-plan_2026-10-04.md)
coordinates the remaining contract consolidations and bounded mechanism comparisons.

**Artifacts.** Arrow IPC artifacts are only derived, content-addressed caches for bulk consumers, such
as native executor inputs, with their manifests in PostgreSQL. They are never canonical.

**The stage table (Implemented / focused-Tested, 2026-10-01).** A typed **stage table** is the sole writer authority and names each stage's input relations, output relations,
contributions, effect class and code identity. From it:
- the scheduler is derived;
- every ordinary output has exactly one writer; vocabulary has one assembly owner and one writer
  per declared immutable epoch;
- a read before its writer runs is an error;
- publication groups close declared private vocabulary deltas and ordinary results atomically,
  with no read authority before group acknowledgement (§15.11).

**Implemented, qualification in progress, 2026-10-03.** Scheduled extraction coverage names the
actual provider per fact family through `FamilyCoverage`; one stage can coordinate canonical
Ruff syntax and native Pyrefly typing without merging their support identities. Admission derives
exact scope/family/provider expectations and reconciles the joint stage outcome. Relation writers
remain singular; no provider discovery registry is introduced.

> Decision: ADR-0119


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

**Completed-stage reads (Implemented / focused-Tested, 2026-09-30; automated gate passed; pilot measurements unqualified).** A cumulative attempt freezes each completed
stage's outputs atomically with receipts and importer read grants, after draining readers/writers.
An immutable validated facts checkpoint precedes normalization. Private source-bound input
capabilities carry completed content and scoped availability; published-reader authorization stays
separate. Read leases last through drain, and query execution admits the full physical plan's scan
capacity before any scan starts. One attempt budget covers acquisition through publication.
The [detailed protocol](../../plans/semantic-model-phase3-detailed-plan_2026-09-30.md#6-cumulative-frontiers-and-private-stage-reads)
owns the implementation sequence and controls; none of this is included in P0–P2 qualification.

**P4 vocabulary publication (Implemented / focused-Tested, 2026-09-30; ADR-0105/0108).** Analysis creates vocabulary already closed at the
facts checkpoint. Private typed deltas plus one atomic publication group close a declared new
prefix together with ordinary results referencing it. Computed-stage receipts grant no reads;
group closure drains writers, revokes delta grants, merges and validates stored contents, then
issues completed receipts atomically. Existing rows never change. Lower readers see immutable
security-barrier views with literal prefix bounds, never a growing canonical table. Old receipts
verify their original prefix. Failed/unconfirmed closes poison the attempt; final seal requires
every group closed. [Plan §3](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#3-prerequisite-immutable-shared-vocabulary)
owns the finite whitelist, schedule and migration. Named publication boundaries map to contiguous,
schedule-bound physical prefix ordinals; their append-only domain codes do not determine execution
order. Ordinary outputs inherit the maximum prefix of their declared acknowledged inputs, including
completed handoffs. Completion validates vocabulary references against that bound and any narrower
explicit input prefix before granting reads. An unrelated later close cannot widen this authority.
A runtime permit can narrow an acknowledged vocabulary grant to an earlier closed prefix from
the same attempt and schedule. It cannot widen that grant or supply a receipt of its own; the
provider reads the selected immutable view. This also lets one consumer use Facts-bound native
semantics and a later derived vocabulary without conflating their authority.
The [R0/R1 foundation review](../../design_review/reviews/design_review_phase4-foundation_2026-09-30.md)
is **Accept scoped, 2026-10-01**. Scoped Phase 4 acceptance passed, 2026-10-01.

**Implemented foundation, 2026-10-01 (ADR-0108).** The finite nominal owner graph follows
actual stored-read dependencies, including execution sub-stages and catalog core → evidence →
selection → synthesis → retrieval. New vocabulary closes occur only where those consumers need
new predecessor vocabulary; ordinary results can complete against the existing prefix. No global
proof reference sum may pull future owners into an earlier stage's invariant dependencies. Final
frontier coverage has a separate one-shot writer checking the independently declared expected
owners/outcomes (**Implemented / focused-Tested, 2026-10-01**); actual cumulative publication
controls passed within the recorded scope, 2026-10-01. Catalog construction remains independent of optional analysis success.

> Decision: ADR-0105, ADR-0108

**Normalized availability (Implemented / Tested within the recorded scope, 2026-10-01).** A final writer
records capability/scope/context outcomes independently of successful computation. Model-owned
capabilities declare anchor and dependency families. Shared input/context/family premise sets
retain original scoped coverage once; outcomes and their producing-stage receipts refer to these
sets. Local unavailable and unrequested anchors remain explicit even with zero observations.
Complete requires complete dependency coverage across the input, preserving cross-source target
uncertainty. Frontier admission checks exact scoped membership and acknowledged output receipts.
The same scoped operation is available over the private facts checkpoint before final assembly.

> Decision: ADR-0086, ADR-0089, ADR-0094, ADR-0105

**Execution alignment (Implemented, 2026-10-02; ADR-0116).** Model dependency closure retains
exact relation/publication-prefix/stream-order requirements and lowers a separate sufficient
grant using scheduler ordinals. Direct facts and explicit invariant epochs survive inference;
unfinished ordinary outputs refuse. C1/C2, Analytic and S0 consume this operation. The compiler's
finite upper-stage binding supplies declaration, phase, publication membership, graph needs and
runner; preflight checks routes and cross-phase dependencies before effects. Actual adapters read
acknowledged sources in separate sessions, coalescing only identical source universes.

> Decision: ADR-0116

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
- Explanations follow model-declared canonical derivation sources with bounded node/edge/depth budgets.
  Source receipts are verified; damaged browsing views cannot hide canonical proof premises.

**Accepted target, 2026-10-02:** [ADR-0114](../../adr/0114-generation-serving-contracts.md)
and the [Phase 5 detailed plan](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md)
develop a serving-role process guard, generation-prepared classification and one ranking owner.
Semantic shapes remain generated views. A narrow disposable vector-only PostgreSQL artifact is
accepted for the existing exact profile, derived through the canonical byte codec and explicitly
prepared outside read-only startup; it is not a copied semantic serving schema. Its validation,
atomic preparation and generation cleanup are required. No new semantic frontier or readiness
registry is added. These contracts are implemented in the current tree; Phase 5 composite
qualification and operator activation remain pending.

**Implemented foundations, 2026-10-02:** the
[foundation enhancement scope](../../plans/semantic-model-foundation-enhancements_2026-10-01.md)
provides one model-owned expected-domain dispatch and owned Local/Summary consumed inventories
with epoch-aware typed streaming; their composite focused controls passed on 2026-10-01.
Selection now owns narrow classification data and charged indexes while retaining strict broad
C2 replay. A serving-role canonical reader shares the existing generation lease protocol without
provisioning credentials. Its opaque selection owner verifies consumed content and producer
source/epoch receipts on the original leased connection, retains the lease and memory through
shared ownership, and terminally refuses classification after guard loss. Pure selection controls
and both-profile actual Catalog admission controls passed on 2026-10-02; foundation qualification passed with the composite receipt in companion §5.4. The later hook
formatting belongs to Phase 5’s fresh current-tree qualification. This is the reusable foundation consumer; Phase 5 implements process admission, route mappings,
ranking, native reconstruction and MCP with scoped receipts. Real-library qualification remains stopped.

> Decision: ADR-0086; ADR-0114

**Prepared native owner (Implemented, 2026-10-02; ADR-0116).** `native_requests::PreparationInputs`
names typed hydrated relations; `PreparedNativeSemantics` owns Entry replay, full structural
correspondence, checked atoms, Local/rebase/Summary/path composition and exact refused contexts.
Its independently prepared public formal domain includes valid defaulted formals even when exact
assignment is unsupported. Refusals preserve the constructor's actual cause. Storage owns shared
validator execution, content/epoch receipts, actual proof membership, retained charges and the
original guard; it releases the lease mutex before pure CPU preparation and reconfirms that guard
before publishing state. Pure helper agreement never admits a generation or invented proof row.

**Startup execution (Implemented; focused qualification pending, 2026-10-04).** Selection,
native semantics, retrieval indexes and numerical initialization share the original guard's CPU
capacity. Startup admission has a finite wait; preparation uses the preparation allowance and
does not inherit a request deadline. Tracked blocking workers retain the slot, guard and charges
until actual completion even after caller cancellation. Shutdown fences new startup registration,
drains request and startup workers, then releases the original canonical session. Numerical
buffers retained by the service keep their preparation reservation until shutdown clears them.
The [target coordinator](../../plans/target-implementation-alignment-plan_2026-10-04.md#7-current-disposition--sole-owner-for-transferred-review-obligations)
owns IS2 controls and stopped activation; this implementation label is not lifecycle acceptance.

**Existing extension routes (Implemented guidance, 2026-10-02).** A new finite model extends its
checked construction, declared typed inputs and production/replay path; an independent matching
and refused-context twin challenges it. A new selection predicate extends the finite model
vocabulary, classification/evaluation algebra and generated schema; supported, unresolved and
conflicting results remain separate. A new packet composes its typed output binding and canonical
hydrator with attempted-read controls. Do not add a second Python meaning table or generic extension
registry. [§4.3](acquisition-and-extraction.md#section-4-3) owns the analogous fact-family route.

> Decision: ADR-0116

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
stores are retained. Phases 0–2 restore model/store/facts; phases 3–5 reconstruct normalized,
analysis/catalog and serving layers in order. The Phase 4 driver passed its scoped cumulative publication controls, 2026-10-01; Phase 5 current MCP is implemented, with qualification pending. Availability is recorded per generation; missing capabilities never become empty
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
