# Design review: expanded analyzer facts and their analytical cascade

**Design / target, 2026-10-03.** Core principles/template **3.2**, code-intelligence profile
**1.3**, [repository binding](../design_principles/binding/library-context.md).
Accountable reviewer: coordinator, with independent pinned-provider research. Baseline:
`main` at `700476d9` plus preserved serving-qualification edits. Diagnosis is **Interface-checked**
from source; implementation directions and anticipated output improvements are **Proposed**.

## Integrated assessment

**Decision: Revise the extraction and first analytical layer, retaining the semantic-model
architecture.** The current typed model, immutable PostgreSQL generations, native preparation,
bounded analyses and programmatic synthesis are useful foundations. They do not need replacement.
The valuable changes are richer provider payloads, exact attachment, explicit interpretation
operations and consumer corrections. More facts alone would leave several important gaps intact.

The [earlier opportunity review](design_review_code-facts-opportunity_2026-10-03.md) correctly
identifies effective signatures, class characterization, imports/exports, typed exceptions and
located types as productive work. This review expands that scope to **independent latest Ruff**,
captures and timing, per-binding narrowing, implicit protocol origins, generic characterization,
and the actual consumption of already-computed predicates. It also corrects several premises:

- Latest Ruff is a production fact provider independent of Pyrefly's embedded Ruff. Its populated
  semantic model requires its private traversal driver; a public constructor is not an extractor.
- Pyrefly's Pysa call graph supplies `WithEnter`, without a `WithExit` origin. Exit inference is a
  different surface. A `Never` call-result type alone does not certify divergence.
- ty's lazy closure snapshots assume nonlocal boundness. Its narrowing don't-care edge is an OR
  formula contribution, not the reachability diagram's AMBIGUOUS value.
- Local theory computes typing-qualified predicate truth, but the inspected consumers do not use
  that result to restrict a condition or decide a Summary. The earlier review's possible unsafe
  served refutation is **not established**. There is a concrete missing semantic-consumption step.
- Existing parameter facts lose their identity correspondence before FCA and the type layer.
  Fixing this can improve current outputs without adding an analyzer capability.

The most useful boundary is **provider observation → model-owned interpretation → consumer-specific
admission**. A signature answer, a grouping attribute and a behavioral premise legitimately ask
different questions. They should share attribution and assumption propagation, without one global
policy that forces all three to admit the same facts. Typing assumptions are an orthogonal part
of claim basis, not a substitute for source fidelity, verdict, approximation or coverage.

The target below is deliberately square at the fact/first-operation boundary. Later analytics
and serving changes have named outputs, preservation constraints and settling questions. They
are not represented as completed algorithm design or measured improvements.

## 1. Scope, method and finite coverage

The subject includes the earlier review, extraction, normalized relations, local theory,
completion/dispatch, structural projections, FCA/RCA and type/mention layers, catalog/selection,
Summary synthesis and generation-bound serving. Owners are the [architecture map](../../design/README.md),
DESIGN §B1/§B6/§B8/§B9/§13 and focused sections §3–§4, §9–§11, §14–§15.
This is a target review of their code and contracts, not a release-qualification receipt.

| Provider role | Assumed target, independently source-inspected | Current implementation pin, unchanged |
|---|---|---|
| Pyrefly type/resolution provider | 1.4.0-dev.3, `80cec3f57364bc11d4a39a419f6894a8eabcaa00`; embedded Ruff 0.0.14 | Pyrefly 1.3.1, Ruff crates 0.0.11 |
| Standalone Ruff syntax/lexical provider | Ruff 0.16.10, crates 0.0.16, `3265ed1f944c98bb4c04d632fbefb1257cdb583d` | No independent populated semantic provider |
| ty core/index provider | 0.0.16 at the same Ruff commit; salsa family 0.28.5 | ty 0.0.14; salsa family 0.28.2 |

These are the latest target versions supplied by the live `python-analyzers` skill, not a claim
that a floating upstream main is equivalent. [Pinned supply contracts](../evidence/2026-10-03_code-facts-expanded-target/supply-contracts.md)
link exact primary-source revisions and record Context7 orientation separately from pin evidence.

The complete indexed universe is **26 areas, 137 concepts, 340 provider-fact records**. All receive
a disposition in the [provider inventory](../evidence/2026-10-03_code-facts-expanded-target/coverage-inventory.md)
and [consumer ledger](../evidence/2026-10-03_code-facts-expanded-target/consumer-ledger.md).
The supply shortlist has 41 candidate payloads, many partly consumed already; this review maps
those to twelve contract groups rather than inventing 41 new relations. The ledger additionally
classifies diagnostics, environment, identity, docs, framework and unsupported-runtime areas.
The 299 previously labelled unused facts are not the review universe, and a skill usage label
does not establish actual use in this tree.

This is conclusive about the named source contracts and selected consumer defects. It is **not**
an exhaustive proof of every semantic guarantee of all three analyzers, nor proof that a blank
survey cell means upstream absence. All 488 source-file references exist; key consequential
contracts were read at the cited grain. The finite catalogue and broader limitations remain
explicit so subsequent work can refine a subpayload without implying that the whole review failed.

The coordinator reviewed current consumers and reconciled the earlier conclusions. A separate
`library-research` worker audited pinned supply. A fresh principal `design-reviewer` and additional
mapper could not start because the agent thread limit was reached; the coordinator performed
their responsibilities directly. No claim of a second independent architectural verdict is made.
No production, pins or skills were changed. No analyzer runtime probe, build, pilot, integrated
gate, gold/heldout evaluation or serving activation ran. Static source suffices for the diagnosed
identity/consumption errors and explicit provider limitations; implementation seams remain untested.

## 2. Responsibilities and consequential domain distinctions

The dependency direction remains extraction/effect adapters → `lctx-model` contracts and pure
operations → PostgreSQL publication/serving adapters. Graph kernels receive declared projections;
synthesis receives canonical evidence and claims. None becomes a second semantic store.

| Owner | Decision it owns | Consumer contract / reason to change |
|---|---|---|
| Acquisition and provider adapters: `cpg-extract`, `cpg-flow` | Exact input bytes, explicit environment, configured traversal, provider-local coordinates and owned payloads | Change on provider API/algorithm revisions; never decide catalog eligibility or behavioral truth here |
| `lctx-model::domain::source`, attribution, syntax, types, symbols, flow | Persistent identity, source correspondence, fact roles, fidelity and per-role coverage | Distinguish source syntax from provider declarations, synthetic objects and unresolved attachment |
| `domain::normalized` | Alias/module resolution, callable variants/slots, descriptor adjustments, public paths, fields | Define once how observations characterize an entity; consumers stop repeating attachment or spelling rules |
| `local_theory`, `local_semantics`, `execution` | Typed-domain interpretation, atom decisions, exact completion, dispatch basis, finite behavior | Admit assumptions for a particular operation; do not relabel inferred types as observed runtime values |
| Projection/structural/analytics owners | Universe, typed relation selection, weights, incidence and heuristic method/settings | Richer input does not silently change a graph definition or multiply votes from agreeing providers |
| Catalog/selection | Source/effective/synthesized roles, exact type predicates, fields, unresolved evidence and eligibility | Explicitly distinguish declared type, effective inferred type, structural facet and behavioral claim |
| Synthesis/retrieval/packet owners | Programmatic text and ranking from canonical records, same-generation citation closure | Preserve basis and uncertainty through output; do not interpret provider payloads in a text template |

Four distinctions govern the additions:

1. **A declaration is not an occurrence container.** A parameter-with-default syntax node, its
   formal node, a native signature parameter and a normalized slot have related but different IDs.
   One correspondence operation must attach them; equal spelling or range does not equate them.
2. **A type is not a value or event.** Declared/inferred/expected/narrowed types have separate roles.
   A capture name is not its call-time value; a resolved method is not proof that dispatch executed.
3. **Precision is not provenance.** A structurally observed guard can have a conclusion conditional
   on a typing premise. Both facts must survive, along with model, verdict and approximation.
4. **An observation set is not a complete universe.** Exports, overloads, members, dispatch targets
   and flow candidates need explicit completeness at the question's scope. None of the providers
   supplies unrestricted Python heap, effects, native behavior or execution evidence.

## 3. Shared contracts before new interpretation

All contracts here are **Proposed** extensions of the existing model, not a replacement framework.
Names below identify conceptual products/operations; implementation planning can reuse existing
records or ordinary functions where they provide the same meaning.

**Source correspondence.** Consume immutable input/artifact digest, provider/revision/context,
byte range, node role/kind and enclosing origin. Produce a unique typed attachment, an ambiguous
candidate set, synthetic origin, unsupported-kind result or unmatched result. Retain evidence
for the decision. Do not fall back to an arbitrary same-name node. Parameter parent↔formal is an
explicit structural correspondence, not a fuzzy join. Synthetic signatures attach to a callable
and native variant/slot identity even when they have no source formal.

Source correspondence also records the analyzer input view and range units. Matching artifact
digests alone cannot equate different decoded text or a transformed runtime view. Current
`cpg-flow/src/lib.rs:20–22,289–357` renames TYPE_CHECKING tokens to a same-length sentinel before
indexing and retains original spans. Preserve that explicit projection and its resolved runtime
override separately from the typing view. A branch classifier must not overwrite it by spelling;
shadowed/reassigned TYPE_CHECKING and string annotations are meaningful controls.

**Coverage.** Extend existing provider/obligation coverage by payload role and scope: requestedness,
successful enumeration, unsupported semantics, provider failure, attachment gaps and operational
truncation remain distinguishable. E.g. source signatures may be complete while decorated
signatures are unavailable. Empty exports after a complete static enumeration differs from
computed `__all__` with unresolved entries. Agreement from two providers is not a completeness proof.

**Claim basis and assumptions.** Keep current evidence status and five verdicts. Introduce typed
premises such as declared-type conformance, no runtime override beyond a typed dispatch set,
specific provider environment and modeled context-exit behavior only where required. An owned
qualification-composition operation unions compatible assumptions with source witnesses and
preserves incompatible contexts as an explicit refusal/unknown. It must be used by atom decision,
transfer, Summary composition, catalog behavioral facets and S0 synthesis. Do not add only an
`UnderTypingModel` evidence-status variant: it would flatten orthogonal source status and premise
sets. Ordinary catalog type characterization needs no behavioral assumption gate.

**Consumer admission.** A model-owned operation for each semantic question consumes candidate facts,
their qualifications and coverage and returns established/conditional/unknown/refused evidence.
Declared-parameter matching and behavioral guard pruning may intentionally choose different
roles. Centralize shared identity, basis and propagation; retain distinct domain policies rather
than a universal registry or blanket provider trust switch. A heuristic screening result cannot
become an exact permission to prune behavior.

**Effects and lifecycle.** Provider handles and borrowed AST/SemanticModel values stay in their
transaction/traversal/salsa lifetime. Emit owned bounded rows. No provider-local ID becomes a
persistent ID or survives as an unqualified string. Configuration, typeshed, Python/platform,
source kind, search paths, typing modules/builtins and relevant rule settings enter provider
context/dependency identity. Publication and selection remain separate; readers retain one lease.

## 4. Selected facts and first analytical operations

Each card specifies the fact addition, current gap, intended operation, downstream consequence and
a revealing legitimate case. Sources are the current paths/ranges and pinned supply contracts.
No card asserts that every field needs a separate new relation or a separate stage.

### C1. Independent latest Ruff syntax and lexical interpretation

**Supply:** `ruff.bindings`, `scopes`, `references`, `qualified-names`, `semantic-context`,
`static-branch-classification`, standard tables and typing classifiers. Ruff has lexical/import
knowledge, not inferred expression types. Public `SemanticModel::new` creates no visited program.
Private `Checker`/`check_ast` performs body, deferred and export visits and does not return that model.
Final global-state flags cannot recover per-node runtime/typing context.

**Operation:** a small pinned upstream/fork extraction seam executes the existing driver and
emits owned scope/binding/reference/export projections plus contextual node answers while the
correct traversal context is active. The adapter records settings and returns explicit failure;
it does not copy Ruff's Python traversal. Model normalization uses qualified identities and
shadow chains for standard/decorator recognition and runtime-versus-annotation characterization.
Ruff's import-qualified name is not proof of filesystem resolution or arbitrary instance-member identity.

**Preferred syntax owner:** latest Ruff supplies canonical syntax occurrences. Pyrefly and ty
attach independent observations to that snapshot. This avoids limiting syntax coverage to
Pyrefly's embedded parser; a Pyrefly-unsupported latest syntax remains a type-provider boundary,
not silently unsupported Ruff syntax. Move the syntax producer out of dependence on Pyrefly's
borrowed AST and use stable model-owned kinds. Keep the two Ruff crate lines named by provider;
never cast their ASTs. Whether ty and standalone Ruff can share their same-version parse is a
local optimization requiring actual lifetime/API evidence, not an assumed property.

**Alternative:** keep Pyrefly's canonical syntax and attach standalone Ruff. It is a smaller
initial wiring change, but any latest-only syntax needs a named unsupported boundary or owner
extension. A narrow compile/fixture spike should settle transition burden; it must not restore
the artificial embedded-version ceiling. No cross-provider framework is justified.

**Case/output:** `from typing import overload as ov` versus a local `ov` should give distinct
recognized decorator roles. A rebound `staticmethod` must not establish builtin descriptor
semantics. Preserve each provider assertion when they disagree; canonical syntax is one authority,
not a vote that discards disagreement. Replace spelling classifiers only where the seam supplies
the required identity/context. Ruff lint JSON alone cannot substitute for these facts.

### C2. Source, effective, synthesized and overload signatures

**Supply:** Pyrefly native `function-signatures`, `decorators`, `overloads`, function flags,
generic callable types. `type_records.rs:1042–1128` uses the undecorated callable; source slot
types/return observations exist. `normalized/callable_normalization.rs:536–612` recognizes only
a narrow builtin descriptor route. Native full types, not Pysa's coarsened report type, are needed.

**Operation:** normalize a role-labelled callable variant with origin, overload-family identity,
descriptor adjustment, slot list, return type, generic binders and availability. Attach slot
types directly to normalized/native signature parameters even without a source declaration;
preserve source formal correspondence as optional evidence. Effective type produced by an
arbitrary decorator is still a typing characterization, not proof of its runtime implementation.
Opaque transformed callables stay unknown; do not assign the undecorated signature to them.

**Consumers:** catalog displays source/effective/synthesized separately; selection gains explicit
effective-type and invocation queries rather than silently changing `ParameterType`'s existing
declared-type meaning (`selection/evaluate.rs:568–597`). Call binding and transfer can later use
a variant only with the appropriate dispatch/model premise. Overload alternatives are not multiple
runtime implementations; selection resolution, ambiguous applicability and implementation body
must remain distinct. Overload evidence must not be flattened to one convenient alternative.

**Case:** a ParamSpec-preserving wrapper, a signature-changing decorator, bound `classmethod`,
overloaded implementation, and dataclass-generated `__init__` challenge source/effective role
separation. Expected output names which signature is available and which source/default is known.
Delete duplicate source-signature fallbacks once this owned operation replaces them.

### C3. Generic binders, substitutions and role-aware typed slots

**Supply:** Pyrefly `generics`, type-parameter bounds, constraints, defaults and variance metadata;
the current model already represents generic/type-variable/ParamSpec terms. This is extraction
and correspondence work, not evidence that generic types require a new type system.

**Operation:** retain binder identity and scope, declared versus inferred variance stage, parameter
default and normalized specialization substitution. Resolve signature/member terms through that
substitution with bounded recursion and explicit alias/unknown boundaries. Display text is never
the substitution key. Declared `T`, inferred `T`, `Self`, `ParamSpec`, constrained and defaulted
parameters remain different meanings; do not assign a generic origin's signature to a specialized
member without the substitution witness.

**Consumers/case:** catalog and selection can answer a specialized effective return/parameter query;
type layers can group by declared structure under a named policy. `Box[T].get -> T` specialized
to `Box[int]` must retain both generic declaration and specialized result. A recursive alias or
unresolved ParamSpec yields a partial type answer, not a class-name match guessed from rendering.

### C4. Class/member metadata and typed record options

**Supply:** Pyrefly class metadata, abstract members, class fields/members, deprecation, MRO,
metaclass, explicit abstract/protocol/runtime-checkable/final/enum/slots/record-transform details.
`symbols.rs:166–175` and `symbol_records.rs:462–468` persist only a narrow trait subset.
`normalized/symbolic_fields.rs` reconstructs record options from syntax.

**Operation:** normalize class characterization with origin, explicit/effective distinction,
inherited/synthesized/source member roles and record-option evidence. Use provider-decided options
where they mean the requested typing/constructor question; retain source text/defaults as source
facts and explicit disagreement. Provider metadata does not establish default-value identity,
factory purity, complete runtime attributes or decorator effects. A single owned record-option
interpretation should replace equivalent ad hoc reconstruction, not erase genuinely separate
source-versus-effective views.

**Dispatch:** a separate closed-target assessment states its basis and universe: exact runtime
class witness, complete modeled MRO, typing-final premise, enum construction constraint, or open
world. `typing.final` is not runtime enforcement. Enumerating enum members is not a complete
heap/attribute model. A protocol's callable signature is not its implementation. This assessment
feeds call/field resolution before behavior, not a trait boolean consumed as closure everywhere.

**Output/case:** expose abstract/deprecated/protocol/record status, generated constructor role and
field evidence. An inherited field overridden in a subclass and a custom metaclass keep correct
origin and uncertainty. Deprecation text/version evidence may inform a brief; it must not invent
an alternative API. These additions also support metadata facets after unsupported predicates
have an honest admission route.

### C5. Per-alias module resolution and partial exports

**Supply:** Pyrefly import binding/handle resolution, export maps, `__all__` specified/partially
unresolvable/invalid status, re-export and wildcard evidence. Existing analyzed root modules are
excluded from dependency observations (`pyrefly_stage.rs:1254`); normalization indexes only those
dependencies (`relation_normalization.rs:263–320`). This confirms earlier F05.

**Operation:** resolve each import alias to a provider module identity using the recorded importing
context; include analyzed roots, dependencies, namespace/bundled modules and unresolved findings.
Preserve multiple candidates and exact alias/re-export origin. Supplying a path to Pyrefly's
`import_handle` bypasses lookup and must not be cited as independent resolution. Normalize public
path candidates with export-enumeration completeness and uncertainty; partial computed exports
cannot certify absence of a public path.

**Consumers/case:** public catalog aliases, demonstration association, module dependency projection
and feature discovery can improve immediately. `from . import a, b`, namespace packages and
`__all__ = known + dynamically_computed` must retain per-alias results and partial export status.
Two aliases of one entity remain distinct public paths with shared entity identity; they are not
two declarations or two graph votes. Never replace resolution with string equality over module names.

### C6. Located types, field identity and expected-type context

**Supply:** Pyrefly expression/located/expected types, native member/attribute targets and class
fields. Current observations cover call results, arguments and test operands; the prior F08 names
actual consumers beyond those roles. The prior blanket no-consumer deferral is too broad.

**Operation:** extract only role-selected located types and member-resolution observations needed
by receiver/field/call consumers, with source expression, context and declared/inferred/expected
role. Normalize exact target, typed candidate set, descriptor access and unresolved result.
Expected type is an expectation imposed by surrounding syntax; it is not the expression's inferred
type or runtime value. Keep field lookup, property invocation and storage-field identity separate.

**Consumers/case:** catalog field-access witnesses, receiver-location selection, local field
analysis and dynamic-read resolution gain candidates. A property and stored attribute with the
same spelling must not produce identical access/effect evidence. An `Any` receiver gives a
candidate/unknown answer, not an exact matching field. Behavioral reads require transfer/state
evidence after identity; the extra type does not close effects or aliasing. Select locations by
consumer role to avoid persisting all expression types with no benefit.

### C7. Captures, global/nonlocal identity and timing-qualified flow

**Supply:** Pyrefly capture origin/name/kind and mutable-capture information; ty scopes, places,
live bindings/use-def and eager/lazy closure snapshots. `pyrefly_stage.rs:1412–1440,2012–2028`
computes captures for Pysa exporters, but symbol/call records do not persist a capture-state
contract. `local_semantics.rs:225,324` refuses captured/global state. ty's lazy snapshot explicitly
assumes nonlocal boundness; it is not a runtime guarantee.

**Operation:** normalize capture identity and timing first: declaring scope/place, enclosing
callable, eager/lazy evaluation context, mutable/global/nonlocal role, and candidate definitions
with source/condition/boundness qualification. Persist no imagined concrete capture value.
Lower a value only for a modeled, demonstrably stable closed case whose definition and call-time
conditions satisfy the existing value contract; retain unknown for mutation/late binding otherwise.
Captures can still characterize a callable or explain a refusal without value lowering.

**Case/output:** closures created in a loop, a nonlocal reassignment after definition and a capture
called before assignment must not be certified from names or an AssumeBound snapshot. A stable
literal capture is a bounded candidate for greater finite-summary coverage, not justification
for a general heap analysis. S0 can explain "depends on captured state not modeled here" with a
specific origin instead of an unlocated generic boundary.

### C8. Reachability predicates, per-binding narrowing and actual truth consumption

**Supply:** ty core predicates/reachability, additional per-binding narrowing constraints,
per-place targets and stored predicate aliases; Pyrefly type domains and literal/enum-literal
terms. Current `cpg-flow/predicate.rs:595–639` translates reachability diagrams. It does not call
the narrowing evaluator. The earlier claim that narrowing adds little is not established for
these additional payloads. These are different operations even when they share predicates.

**Operation A — structural lowering:** preserve typed predicate kind, source evaluation identity,
polarity, place and formula role. Lower a reachability AMBIGUOUS node as a documented conservative
approximation. Lower narrowing as `uncertain OR (p AND true) OR (!p AND false)`; do not treat its
don't-care edge as an unknown Boolean verdict. An upstream precision fallback to ALWAYS_TRUE
does not establish tautology. If the provider does not expose per-result fallback provenance,
retain a conservative model-level limit or add a narrow observer; do not claim exactness from the
terminal value. Keep binding-versus-declaration distinctions after ty's import representation change.

**Operation B — typed atom decision:** consume exact leaf/support/operand correspondence, domain
interpretation, inhabitedness, provider premise and question-specific policy. Produce a true,
false, mixed/undecided or refused decision with basis and witness. `local_theory.rs:1369–1385`
already computes `PredicateResult`; `local_semantics.rs:1194–1276` emits a derivation using the
original qualification without reading that result. `summary_production.rs:193` declares the
witness input without a truth consumer. Add the missing operation and controlled restriction/
transfer use only after assumption propagation exists. Empty/Never domains are not runtime
observations of a branch's outcome.

**Consumers/case:** literal guard and exact-class decisions can reduce unknown finite behavior
under explicit assumptions; use-def narrowing can support operand association without creating
another type authority. Challenge `x: Literal[0]` called with a nonconforming runtime value,
shadowed builtins, TypeGuard/TypeIs aliasing and an empty domain. A type-level result can be served
as such, or as conditional behavior with its premise; it cannot silently remove the runtime path.

### C9. Protocol calls, terminal calls and context exits

**Supply:** existing Pyrefly call-target/receiver/higher-order payloads and 23 Pysa origin kinds,
including operators, subscriptions, iteration and `WithEnter`. These are partly consumed today;
audit missing subpayloads and phase linkage, not the fiction that implicit calls are wholly new.
Pysa has no exit-call origin. Pyrefly's solver has distinct exit inference and non-complementary
definitely-suppresses / definitely-does-not-suppress predicates.

**Operation:** normalize protocol action with exact origin, dispatch candidate, phase and coverage.
For context exit use explicit typed member inference/lookup and ordered with/async-with semantics;
unknown suppression remains unknown, not false. For terminal calls consume the provider's qualified
divergence decision or reconstruct its necessary declaredness/receiver/dispatch checks under a
named model. `CallResult=Never` can arise from an already-Never receiver and is insufficient.
Implicit-target facts alone do not establish effect-free completion.

**Consumers/case:** explain which iteration/operator/enter method participates; improve completion
only with the appropriate model certificate. A suppressing manager, unknown exit type, overridden
Never-returning method and async manager are negative twins. Source expression call phase and
deferred coroutine/generator execution must not collapse. Terminal/suppression candidates and a
try body's exact completion are distinct prerequisites; neither closes every AMBIGUOUS region.

### C10. Exact exception identity and ordered source handlers

**Supply:** mostly current symbol resolution/MRO/source syntax; additional raised type observations
are upper-bound typing characterization, not complete per-call may-raise sets. Earlier F04 is
confirmed by `execution/completion.rs:345–385`: it admits a first bare handler and refuses typed
handler children. Current exact runtime exception vocabulary is narrow.

**Operation:** use the existing `CheckedExactClass` mechanism to certify modeled exception identity,
then match handlers in Python order, including tuples and inheritance. Preserve the active exception,
named-handler disposal, else/finally behavior and reraising. Do not execute arbitrary exception
constructors or treat typed `Raised` observations as exhaustive runtime exceptions. Exception
groups/`except*` require a separate supported contract; they must not inherit ordinary-handler matching.

**Consumers/case:** exact finite completion and error facets can improve without waiting for all
new facts. A subclass matched by an earlier broad handler, a non-exception class, a tuple handler
and a finally replacement expose identity/order failures. Retain explicit refusal for dynamic
raise construction and unsupported groups. No analyzer supplies universal exception propagation.

### C11. Existing facts reaching structural/type/FCA/RCA analytics

**Current defect:** `syntax_records.rs:312–382` creates `ParameterWithDefault` and formal `Parameter`
occurrences. `symbol_records.rs:67–91,625–633` and `type_records.rs:1080–1095` attach declarations/
types to the formal. `analytics/attributes.rs:27–63,228–236,445–480` compares them to the enclosing
syntax parameter. The mismatch affects receiver exclusion, ordinary Parameter/ParameterType
incidences and parameter contribution to the type layer. It applies to ordinary parameters even
without a literal default: the syntax kind differs. Varargs use self-correspondence.

**Operation:** consume the shared normalized parameter/formal/slot correspondence from C2/C3;
repair the current source correspondence first rather than waiting for enriched signatures.
Policies explicitly select source/effective, declared/inferred and receiver-included/excluded
roles. Do not independently reconstruct formal mapping in FCA, type-layer and selection code.

**New attributes:** resolved decorator identity, characterized class/record traits, deprecation,
typed fields, signature roles and capture dependence can have named discovery consumers. Define
exact attribute meaning and evidence before adding each. FCA incidence is entity×attribute,
with multiple supporting source rows; it is not a count of agreeing analyzers. The existing
`analytics/concepts.rs:75–103` bitset context correctly treats repeated incidences as one Boolean
membership. Preserve that property. Type-layer pair construction currently deduplicates membership
by class/entity (`attributes.rs:268–273`); retain provenance while making any new weight policy explicit.

**Case/output:** a source `f(x: int=0, *, y: str="")` should contribute both parameter names/types;
a bound receiver is excluded under that policy; aliases do not multiply votes. More accurate
concepts improve grouping/navigation and exact scoped implications. They do not prove API safety,
runtime dependency, absence of missing attributes, or universal implications beyond that frame.

### C12. Diagnostics, framework and usage evidence with bounded consumers

**Supply:** provider diagnostics and rule vocabulary, Pyrefly pytest fixture understanding,
deprecation, call-site/argument/overload evidence, and existing docs/test source facts. Ty inferred
types/IDE signatures are credible competing/oracle surfaces, not automatically a second pipeline
type authority. Diagnostics differ by suppressions, settings, stubs and environment.

**Operation:** preserve selected diagnostics as attributed corroboration attached to an original
example/test or operator explanation, including suppression/configuration scope if absence is
discussed. Fixture injection can explain a test parameter's role and its association; it does not
certify the fixture ran, prove a feature works or remove all unresolved parameters. Pick emitted
call applicability evidence when an API actually exposes it; do not invent overload-choice facts
from signature enumeration or display text. Expected types can corroborate usage intent, not observed execution.

**Consumers/case:** evidence search can distinguish a source example, a test expecting failure,
a statically diagnosed invalid invocation and actual execution evidence. A suppressed diagnostic
or dynamically named fixture keeps uncertainty. Formatting/fix/refactor/LSP transport surfaces
have no new served-fact consumer here. Configured diagnostics can be useful without becoming a
new lint subsystem or blocking every catalog compilation. An offline ty differential oracle must
harmonize environments before interpreting disagreement.

## 5. Cascade through analysis and target outputs

This table specifies what the additions would change. All new behavior is **Proposed**; current
method owners and their typed settings/outcomes remain in place. Each graph/frame names generation,
context, universe, relation kinds, direction, multiplicity, selector, weights and coverage.

| Layer / question | Inputs and first change | Method/model, bounds and partial behavior | Output / target benefit and limit |
|---|---|---|---|
| Normalized entity/callable/public-path characterization | C1–C6 exact attachment, variants, substitutions, resolved aliases and metadata | Relational/domain operations; bounded candidate sets and type traversal; role-level unknowns | Richer catalog-ready identity and callable/field facts with witnesses; no behavior claim yet |
| Local theory, entry reads and field/capture analysis | C6–C8 located roles, definitions, typed atom decisions, stable capture cases | Existing finite value/place model and BDDs; assumption-aware decisions; bounded recursion/refusal | Fewer unexplained unknowns, typed/capture boundaries located; no general heap or alias guarantee |
| Completion, dispatch and finite summaries | C4/C8–C10 closed-target basis, terminal/exit certificate, exact ordered handlers | Existing transfer semantics and SCC summaries; preserve call phases, conditions, five verdicts, resource stop reasons | Conditional/established/refuted-under-model summaries cite premises; unknown suppression/dispatch does not become absence |
| Structural projections | C5 imports/public paths, C9 protocol targets, corrected identity | Typed import/call/containment/type projections remain distinct; complete vertex universe separate from output selection; parallel arc IDs retained | Better dependency/usage navigation; possible/implicit/deferred calls labelled by relation/phase, not relabelled as executed calls |
| Communities and PageRank | Only explicitly revised declared projections; metadata can characterize returned members | Existing governed native kernels, weights/seeds/convergence diagnostics and budget outcomes | Rank/group discovery within a named projection; do not make new facts automatic edges or let eligibility filters shrink the compute universe |
| Type/mention similarity layers | C3/C6/C11 role-selected type/member/decorator identity, repaired parameter mapping | Existing sparse bounded construction; normalized entity memberships with declared weights and missing-role coverage | More meaningful optional similarity/grouping; no dense all-pairs requirement or behavioral premise |
| FCA/RCA | C11 corrected source incidences; selected signature/class/decorator/capture attributes | Boolean incidence and named RCA relations; bounded concept enumeration, support and operational partial result | Exact implications over the stated frame, candidate facets and explainable groups; missing evidence remains unknown outside that incidence model |
| Catalog and declaration selection | C2–C6/C12 roles, types, fields, metadata, aliases and original evidence | Model-owned predicate evaluation/admission; exact versus conditional selection explicit; unsupported predicate refusal before effects | Agent can select effective typed callables/record fields and see source defaults/aliases; current declared ParameterType meaning preserved |
| S0 assertions and briefs | Canonical catalog/behavior/analytic evidence plus assumption basis | Programmatic synthesis; existing claim/proof/evidence identity; no interpretation of analyzer internals in templates | "Typed signature", "inferred effective signature", "conditional behavior" and "grouped by" have different wording/evidence |
| Lexical/vector retrieval and packets | Rebuilt canonical texts/facets with richer evidence | Existing retrieval spec, pinned generation and embedding spec; rank only admissible candidates, do not prove truth by similarity | Better discoverability is plausible; retrieval quality and embedding benefits require later ablation, not claimed by this review |

RCA needs a separately declared relation universe when adding imported/member/protocol relationships;
an inferred type reference is not automatically a runtime dependency. An analytic facet requires
a named product consumer and membership support. Fix prior F11's unsupported predicate contract
before exposing new membership facets. Public API eligibility remains independent of rank or community.

Three target answer shapes make the cascade concrete:

- **"How do I invoke this decorated or generated API?"** Return source and effective/synthesized
  signatures separately, slot types with role, generic specialization, invocation form and exact
  original evidence. A generated source default may remain Unknown even with a complete typed signature.
- **"Which public path and option should I use?"** Resolve re-export aliases to one entity, expose
  record/field/deprecation evidence and matching typed options; show partial computed exports or
  missing effective types as uncertainty rather than excluding the API as absent.
- **"Does it raise, return or suppress under this condition?"** Return a finite-model verdict,
  condition, completion/dispatch witnesses and any typing or closed-world premise. Community,
  type similarity and a lint-free example cannot establish this answer.

Changed facts/roles require a model/schema digest migration and rebuilding downstream generations,
catalog texts and dependent embeddings through the existing pipeline. No old-format readers,
parallel store or compatibility semantic authority should be introduced. Exact scheduling and
qualification belong to implementation planning; stopping old readers and preserving evidence
remain prerequisites to activation, not work authorized by this review.

## 6. Reassessment of the source review and new findings

Original IDs retain their source meaning. This is a dated reassessment, not duplicated mutable
execution status. New findings below are unscheduled and **Deferred to implementation planning**,
with the explicit triggers in §11; acceptance is not claimed by that disposition.

| Earlier finding | Follow-up assessment / obligation |
|---|---|
| F01 admission/typing premise | Retain the assumption-propagation need before behavioral activation. Replace a universal admission policy with owned question-specific policies plus shared basis composition. Unsafe current served predicate refutation remains unestablished; new F02 identifies actual dormant truth |
| F02 effective signatures | Confirmed in current source; C2/C3 include synthesized slots and overload/generic roles, not only more signatures |
| F03 dispatch closure | Retain; C4 requires a basis-labelled target universe, not runtime finality from `typing.final` |
| F04 typed source handlers | Confirmed; C10 can substantially use current facts and existing exact-class operation |
| F05 intra-release imports | Confirmed; C5 replaces dependency-name-only matching with per-alias module identity |
| F06 stale extraction claims | Retain source-current documentation correction. This target does not promote stale extraction labels to tested facts; architectural edits accompany actual decision/adoption |
| F07 record options | Retain for equivalent interpretation; preserve distinct source/effective semantics and factory/default limits, C4 |
| F08 located-type deferrals | Retain; C6 names selected consumers and expected-versus-inferred distinction |
| F09 erased predicates / AMBIGUOUS | Retain the split between provider predicates and completion certificates. Extend to typed predicate identity and distinct narrowing algebra, C8/C9; T7's bare Never test is unsafe |
| F10 index-only ty / §B1 clarification | Refine: independently latest Ruff requires reversal of the one-parse/embedded-version constraint. Retain ty core as selected flow provider; inference oracle first is a preference, not a categorical impossibility after environment harmonization |
| F11 unsupported facet predicate | Confirmed `selection/evaluate.rs:1049` always returns `Ok(None)`; provide honest pre-effect admission/refusal before adding analytic facets |

### <a id="F01"></a>F01 — Independent Ruff has no owned populated extraction route

**Diagnosis, Interface-checked:** §B1 and extraction wiring depend on Pyrefly's AST/parse; the target
requires independent latest Ruff. Its public semantic model is unpopulated without its private
Checker traversal. A final-state accessor also loses contextual flags. This is an architectural
barrier to the selected provider, not a reason to reject it. G7/G8 and FP-01/02/06 are unresolved
until the boundary is chosen and qualified.

**Correction, Proposed:** C1's narrow upstream/fork traversal seam and owned projection, model-owned
source correspondence, explicit independent contexts and one canonical syntax owner. Prefer latest
Ruff for syntax; a compile/fixture comparison can settle the smaller alternative's limits. Delete
equivalent spelling recognizers only after their semantic replacement is established. Fork patches
must be scoped and parity-checked; do not copy the Checker or export borrowed internals to analytics.

**Closure:** an actual populated driver compiles at the target pin; alias/shadowing, deferred
annotation, TYPE_CHECKING and latest-syntax fixtures produce correctly attached/contextual facts;
configuration perturbation is recorded or refused. Update §B1/§B8/§B9 and dependency checks through
an ADR. This review establishes no compiled seam.

### <a id="F02"></a>F02 — Computed predicate truth has no governing consumer

**Diagnosis, Interface-checked:** C8's producer records `PredicateResult`, while `emit_theory`
only emits support over the original qualification. The summary input declaration is not a
truth-use operation. There is no identified current restriction/transfer consumer of that field.
Computed false/true typing-domain results therefore do not supply the promised analytical precision.
FP-04/05 and A2 are violated for that integration target: a domain result exists without governing
the operation it is intended to inform. This is not evidence of an unsafe current runtime refutation.

**Correction, Proposed:** add a typed atom-decision operation with inhabitedness and premise basis;
activate condition/transfer restriction only through that result and assumption-aware composition.
Display-only theory evidence and behavioral pruning remain separate legitimate consumers. Do not
"fix" this by switching on a result without propagating premises or by deleting conservative paths.

**Closure:** known-answer literal/exact-class cases change the controlled analytical result and
served conditional basis; nonconforming runtime values, empty domains, unresolved attachment and
mixed domains remain correctly scoped/refused. Trace the truth result into its first semantic use
and the resulting Summary/S0 claim; code existence or persistence alone does not close it.

### <a id="F03"></a>F03 — Parameter identity mismatch drops existing analytic inputs

**Diagnosis, Interface-checked:** C11's exact producer/consumer identity chain proves ordinary
parameter declarations/types are compared against the wrong node. Missing parameter incidences
and type-layer contribution are not provider incompleteness. Receiver exclusion uses the same
mismatch. FP-02/04/05, DP-04/08, A2 and G2/G6 fail at this boundary.

**Correction, Proposed:** resolve through the shared source-formal/native-parameter/normalized-slot
correspondence, then apply the declared analytic role and receiver policy. Repair current source
identity now; effective variants can extend the same operation later. Do not change type-observation
identity to the parent merely to satisfy this consumer: selection correctly consumes formal identity.

**Closure:** independent known-answer source parameters with/without defaults, positional-only,
keyword-only, varargs and bound receiver yield expected parameter/type incidences. Type-layer
controls must share a release-defined class: the existing layer deliberately excludes builtin/
dependency classes, so an `int` example alone cannot establish a missing similarity edge.
Preserve that universe unless a separate policy decision changes it. Retain multiple evidence
rows without duplicate Boolean membership or weights. This
does not require a real-library campaign to establish the local correction.

These new findings interact without requiring one redesign. F01 enables richer source observations;
F03 corrects current attachment regardless of the upgrade; F02 needs the shared assumption contract
before behavior changes. Existing imports/typed-handler/facet defects can proceed independently.
Consequence priority and prerequisite order are deliberately different.

## 7. Change scenarios, library fit and alternatives

| Scenario / kind | Expected owning changes and consumer propagation | Revealing failure / settling evidence |
|---|---|---|
| Upgrade standalone Ruff independently / mechanism | Provider seam absorbs private API drift; source contract remains owned; pin/config changes invalidate dependent facts | No borrowed AST leak, no embedded-version ceiling; private traversal fixture and owned-output compile check |
| Add synthesized generic constructor / domain concept + binding | C2/C3/C4 declare origin, variant/slot types and substitution; catalog/selection consume roles | A source-only parameter join must not erase generated slots; negative opaque decorator case |
| Use typed finality to reduce targets / policy composition | Dispatch owns basis; behavioral admission requires typing/no-extra-override premises | Runtime monkeypatch/subclass counterexample must not become unconditional refutation |
| Add a resolved-decorator FCA attribute / analytic extension | Normalization supplies identity; analytics declares attribute/frame; synthesis consumes membership support | Alias/local same-spelling decorator and duplicate providers; no second spelling classifier |
| Import unresolved modules / failure | Resolution and export coverage retain candidates/unknowns; catalog serves uncertainty | Empty list cannot prove no public API, absent call or unused feature |
| New analyzed release / instance | Acquisition/content and provider settings identify new facts; rebuild normalized/analytic/catalog/serving projections | Same names across snapshots do not authorize old citations or cached semantic facts |
| Trace a conditional claim / composition | Leaf/type/decision/transfer/Summary/S0 share assumption basis and generation | One renderer must not drop the premise or derive runtime truth from rank/type similarity |

The built-in choices are specific rather than a blanket "use all analyzer APIs":

- Ruff driver/library tables replace duplicated lexical/scoping and standard-name interpretation
  where they actually fit. Public AST truthiness/side-effect helpers are useful screening inputs;
  `Absent/Possible/Present` or an identity callback is not a general purity/effects proof. A small
  bespoke join in the model remains appropriate. A hand-copied Checker has much higher semantic
  maintenance cost than a narrow extraction patch.
- Pyrefly native type/signature/metadata operations are preferable to another source inference
  engine or display-string parser. Custom variant/admission/dispatch meaning remains project-owned
  because the provider cannot decide the product's behavioral model.
- ty core is the selected symbolic flow source. Per-binding narrowing and closure candidates are
  useful without adopting all ty inference. Ty inference/argument maps are alternatives for an
  independently configured assurance oracle. If harmonized evidence shows unique high-value
  premises, reopen that choice explicitly; disagreement must retain model/environment attribution.
- Retain current relational/native/graph/FCA mechanisms. Correct their inputs and projections;
  no replacement graph stack, Datalog layer or whole-program runtime model is warranted here.

| Alternative | Benefit and cost | Decision / revisit |
|---|---|---|
| Upgrade pins only, keep interpretations | Smaller diff, but leaves dormant truth, wrong joins and source-only signatures | Reject as the selected target; no causal route to the desired answers |
| Add all 340 payloads to storage | Broad apparent coverage, large schemas and duplicate authorities without consumers | Reject; the finite inventory is a selection tool, not a storage requirement |
| Independent latest Ruff + targeted owned facts + first operations | Better provider use and richer answers; adds parse/context correspondence and narrow fork maintenance | Preferred, Proposed; qualify driver seam and meaningful output changes |
| Keep source types only; forbid all typed behavioral premises | Simple honest behavioral limit, forfeits bounded precision improvements | Viable scope fallback, not the preferred expanded target; can stage typed catalog work first |
| One global trust/admission policy for all consumers | Superficially centralized; would reject legitimate inferred catalog/grouping facts or overadmit behavior | Reject; shared basis/invariants plus question-owned policies |
| Add ty inference as equal pipeline authority immediately | Possible independent facts; extra environment, disagreement and access contracts | Not selected yet; use oracle-first and reopen on demonstrated additional consumer value |

No implementation-time performance gain is claimed. Independent Ruff adds traversal/memory and
correspondence work. Contextual callbacks and owned selected payloads avoid retaining whole models;
role-selected located types avoid uncontrolled relation growth. Existing budgets must cover new
enumerations and partial output explicitly. A later scoped ablation should determine discovery
benefit; measurement is not a prerequisite for fixing demonstrated identity/consumption defects.

## 8. Gates, foundations and independent architectural judgments

Verdicts below concern the **current implementation plus expanded target**, at static evidence
strength. They do not re-certify earlier accepted increments or completed runtime receipts.

| Gate | Verdict | Evidence / required action |
|---|---|---|
| G1 Authority | unresolved | Existing model/store ownership is preserved; independent provider reconciliation, syntax owner and premise composition need the C1/§3 decision |
| G2 Semantic fidelity | fail | F03 wrong parameter join; source-only signature/typed-handler gaps; C8/C9 expose unsafe shorthand in proposed interpretation |
| G3 Validity | unresolved | Current typed invariants are a useful enforcement point; new role/assumption/correspondence construction and validation are not implemented |
| G4 Hidden behavior | unresolved | Existing explicit acquisition/context is preserved; Ruff driver/settings and harmonized optional ty inference have no qualified target configuration yet |
| G5 Consistency and recovery | unresolved | Existing immutable generation/publication boundaries inspected; new payload precision-fallback and coverage propagation need verification, not a new store |
| G6 Transformation and reuse | fail | F03 changes required analytic inputs; assumptions and variant-role information must survive every dependent transformation/cache |
| G7 Truthful capability claims | fail | Prior F04/F06/F11 supported claims exceed current routes; populated Ruff seam and typing-truth activation must remain Proposed |
| G8 Library leverage | unresolved | Rich native capabilities have credible consumers, but Ruff seam/metadata fit must be qualified before declaring bespoke paths equivalent and deleting them |
| CI-G1 Fidelity | unresolved | No current unsafe theory-result refutation established; expanded behavioral admission/precision/phase propagation remains unimplemented |
| CI-G2 Evidence closure | unresolved | Existing typed same-generation claim mechanisms preserved; new role/basis and slot/capture evidence must reach packets without losing premise status |
| CI-G3 Evaluation integrity | pass, scoped inspection | No proposed analyzer input or criterion uses gold/heldout; no evaluation executed or parameter tuning performed; preservation constraint remains |

| Foundation / applicable rules | Verdict | Reason |
|---|---|---|
| FP-01 separation; DP-05/17/18/19 | unresolved | Existing pure/effect boundaries fit; independent Ruff seam must keep traversal mechanics and product interpretation separate |
| FP-02 stable contracts; DP-02/08/15/24 | violated | Current formal/syntax mismatch breaks a legitimate consumer; enriched native variants require a stable owned slot contract |
| FP-03 composition; DP-06/13/14/16 | unresolved | Existing typed pipeline composes well; dormant truth and basis propagation lack a governing composition operation |
| FP-04 domain model/authority; DP-01/03/04/09 | violated | F02/F03: existing named results/identities do not govern intended operations; role/assumption/dispatch distinctions need first-class owned meaning |
| FP-05 explicit structure; DP-07/11/12/20 | violated | Parameter relationship is structurally ignored; new formulas/precision and candidate universes must retain bounded explicit semantics |
| FP-06 local reasoning; DP-10/21/22/23 | unresolved | Thin effects and source receipts help; current consumers require hidden parameter identity knowledge, new seam must localize provider lifetime/config |
| CI-01/02/04/06 | unresolved | Shared attribution exists; expanded type/flow meaning and assumption/coverage are not fully composed |
| CI-03/05/07/08/09/13 | satisfied for inspected preservation mechanisms; unresolved extensions | Typed arc IDs, distinct universe/selector, Boolean incidence and pinned generations are strengths; new projections/weights require explicit contracts |
| CI-10/11/12 | unresolved for new provider/output, satisfied evaluation separation | New Ruff context/basis output needs controls; references remain outside compiler inputs |

| Judgment | Verdict | Independent scenario evidence |
|---|---|---|
| A1 Localize change | violated for current parameter consumers; unresolved new seam | FCA/type layer/receiver exclusion independently know the wrong source-node shape. Shared correspondence localizes this; private provider drift belongs in the seam |
| A2 Explicit domain meaning governs behavior | violated | F02 dormant result and F03 ignored identity correspondence; richer output records alone would not repair either |
| A3 Extend through composition | unresolved | Proposed role/basis operations can compose with current pipeline; no implemented atom-decision/assumption cascade or populated Ruff seam yet |

**Bounded decision: Revise.** The expanded target has a credible incremental route but is not
architecturally accepted as implemented or fully specified. The enclosing semantic-model/store
architecture is retained; full enclosing conformance and release qualification are **not assessed**.
No positive source-contract result offsets a failed gate or A2 obligation.

## 9. Verification boundary and settling evidence

All diagnoses and provider contracts cited above were source-inspected on **2026-10-03**.
Static contradiction is sufficient to establish F02/F03. Future controls below protect semantic
changes; they were **not_run**, and are not completion requirements imposed on this review.

| Boundary | Independent expected-result controls required during implementation |
|---|---|
| Provider seam/correspondence | Alias versus local shadow, deferred annotation, runtime/typing views, different parser support, synthetic and ambiguous node attachment; fixture expectations independent of extractor output |
| Signatures/generics/classes | Wrapper versus transformed callable, overload declaration versus body, specialized generic, bound receiver, generated constructor, inherited/overridden field, custom metaclass/deprecated API |
| Theory/flow/completion | Literal positive/negative/mixed/empty domains, nonconforming runtime input, lazy unbound capture, don't-care OR formula, known precision-fallback limit, unknown suppression, overridden Never method, ordered handlers/finally |
| Analytics | Missing/duplicate support, isolates, parallel protocol/import arcs, shuffled order, parameter parent/formal mapping, exact scoped concept implication versus incomplete coverage; partitions compared by membership |
| Catalog/selection/S0 | Exact source/effective type query distinction, partial public paths, unsupported predicate refusal, same-generation citation and preserved conditional basis in rendered packets |
| Retrieval benefit | Later controlled lexical/optional-vector ablation over frozen non-gold criteria; output differences and quality evidence, not inference from successful compilation |

Provider source revisions and inventory integrity: **passed** static pin/reference checks, with
commands and exact scope in the [evidence README](../evidence/2026-10-03_code-facts-expanded-target/README.md).
Publication: `just docs-check` receipt is recorded there after writing. Product builds/tests,
`just test-all`, `just hygiene`, runtime probes and real-library qualification: **not_run** for this
documentation-only review. Prior qualification edits/receipts remain at their existing owner.

Remaining consequential uncertainty is specific: compile/lifetime cost of the populated Ruff
seam and latest-Ruff syntax transition; exact per-result visibility of ty precision fallback;
native effective-slot/default/overload applicability access; and assumption exposure in the wire
contract. These affect implementation design, not the source-proven current defects. Full runtime
heap/effects/native/monkeypatch/async resource analysis is outside this incremental target.

## 10. Priority and dependency reasoning

First consequence priority is honest fidelity and advertised capability: current import/handler/
facet corrections and F03 must not wait on a provider migration merely because richer facts are
attractive. The independent Ruff boundary and C2–C6 unlock the largest product breadth. Typing
truth, terminal pruning and closed dispatch need assumption propagation **before activation**;
catalog type/metadata answers and characterization can proceed independently. Captures can first
explain limits, then expand finite behavior only for modeled stable cases.

The proposed integration dependencies are:

1. Set provider roles/configuration, source correspondence and role coverage. Decide latest Ruff
   syntax ownership and qualify its driver seam; migrate pins and dependency checks together later.
2. Repair current formal joins/imports/handlers/unsupported admission inside their existing owners.
   Extend effective slots/generic/member/export contracts as selected, reusing the correspondence.
3. Add shared claim-basis composition and question-specific atom/dispatch/exit admission. Only then
   activate computed typing results and new behavioral premises.
4. Feed explicitly declared analytics/catalog predicates and S0 outputs. Rebuild dependent
   projections and qualify the changed product before returning to real-library serving activation.

These are semantic prerequisite relationships, not execution work packages or a staffing plan.
Independent source-identity, metadata and provider work can overlap; large integrated execution
and precise schema migration sequencing belong to the next plan-creation task.

## 11. Authority changes and single disposition routes

This review is evidence. User direction authorizes evaluating independent latest Ruff despite the
current rule; it does not silently mutate accepted ADRs, installed pins or current production rules.

| Required decision/correction | Owning route | Source obligations / trigger |
|---|---|---|
| Independent latest Ruff and canonical syntax; explicit per-provider crate lines | New ADR superseding/amending lifecycle of ADR-0046 as appropriate; DESIGN §B1/§B8/§B9/§7, §4, dependency/pin checks and AGENTS routes together | New F01; prior F10 refinement. Deferred to adoption planning; no one-version rule may reimpose the embedded-version ceiling |
| Typed claim basis, consumer admission and propagation | New ADR where behavioral-model meaning changes; §3.9/§9.9/§14/§15 and executable qualifications/operations | Prior F01/F03/F09, new F02; before any typed behavioral pruning/closure activation |
| Callable/generic/field/export roles and located consumers | Owning §4/§14/§15 plus executable declarations; ADR for changed semantic roles and §13 deferral removal | Prior F02/F07/F08; C2–C6, before output predicates promise these roles |
| Existing source identity/import/handler/facet corrections | Implementation within current contracts; update stale owning claims and meaningful controls | New F03; prior F04/F05/F06/F11; deferred to next implementation plan, not accepted as harmless |
| Additional analytics/diagnostic/fixture consumers | Existing analytics/evidence owners with explicit frame/fidelity and named served consumer | C11/C12; adopt only selected subpayloads, revisit competing/oracle surfaces on unique consumer value |

Unscheduled new F01–F03 disposition is **Deferred**, owned here, until the next implementation
plan assigns component/closure evidence. Original findings remain at the
[source review](design_review_code-facts-opportunity_2026-10-03.md#7-findings-and-applicability)
until transferred with their original IDs. The next plan must consolidate related remediation
without losing distinct closure obligations. STATUS links here; it is not a second finding ledger.

## 12. Decision to carry into planning

Proceed with targeted analyzer adoption and first-layer corrections under **Revise**, using the
twelve contracts and complete finite inventory as planning inputs. Preserve the current semantic
model, immutable generation/evidence boundary, finite behavior, governed analytics and programmatic
synthesis. Do not treat provider availability, a stored result or a successful rebuild as evidence
of improved answers. Plan acceptance must show the first semantic use and the changed target output,
with the stated basis and unknowns intact.
