<a id="section-3-9"></a>

# §3.9 Behavior model: places, conditions and verdicts

**Implemented; code-facts enhancements in progress, 2026-10-03.** The typed owners in
`lctx-model::domain` implement this abstraction. [§15.4](semantic-model.md#section-15-4)
owns entities and places, [§15.6](semantic-model.md#section-15-6) transfers,
[§15.7](semantic-model.md#section-15-7) conditions and
[§15.8](semantic-model.md#section-15-8) obligations and verdicts. The
[code-facts coordinator](../../plans/code-facts-expansion-plan_2026-10-03.md) owns current
enhancement qualification and finding disposition; earlier Phase 4 receipts retain their scope.

`cpg-flow` supplies attributed ty observations. Normalized attachment, Local, execution and
Summary derive their conclusions from those observations through model-owned operations and
shared stored validators. `cpg-core` acquires completed inputs, runs these operations and
publishes into the PostgreSQL generation store. Arrow and DataFusion are transport and compute;
there is no separate schema, behavioral database or serving-side reconstruction authority.
Serving is implemented within its recorded fixture boundaries; real-library qualification and
activation remain separate ([§11](synthesis-and-serving.md#section-11)).

## Places

A place has a typed root and structural access path, not a parsed display name. Roots distinguish
a formal variable, its entry value, a receiver, return/yield/raise ports, a source occurrence,
a local slot, a class field, a module global and the runtime class of an actual expression.
`Formal` may be rebound by the body; `Entry` denotes what the caller delivered. This distinction
is required before a callee value becomes a call-summary port.

Fields and supported literal subscripts extend paths through typed segments. A path's written
form is a label. Provider origins, normalized entity correspondence and occurrence ownership
establish identity; equal spellings do not. Unsupported dynamic access, captured cells and
unproved heap state retain their own uncertainty. A field location does not prove allocation,
alias identity or stability against mutation. ContextVar and arbitrary lifecycle state remain
outside the restored finite behavioral envelope.

## The flow provider

`cpg-flow` links ty's pinned semantic index over its separate declared runtime-view parse. Its boundary contains
source ranges and model-owned flow observations, never ty, salsa or a second Ruff family's
internal types. It describes reaching definitions, uses, regions, predicate leaves and value
sources. Independent Ruff owns canonical occurrence identity; an indexed exact attachment joins ty observations
to those occurrences. Ambiguous or unmatched attachment records its candidates and Partial
coverage rather than choosing the nearest source event.

Provider facts carry input, context, run, origin, fidelity and scope. Loop-carried reaching,
unbound alternatives, annotations and provider-model omissions remain explicit. Provider
reachability is a stated abstraction: exceptional paths follow ty's exception model, not
arbitrary ambient exceptions. A provider panic fails extraction. Version/platform and captured
modules come from the pinned run context; the operator's environment is not an input.

**Implemented / focused-Tested, 2026-10-01:** direct bare-name identity candidates retain their
actual native use and checked-false selection condition. A false-only reaching set is retained
only when nonempty, wholly exact false and free of loop headers; every Defined/Unbound state
survives. Live or approximate sets keep their existing pruning. Entry still requires its complete
supported inventory and one exact unchanged formal origin; missing, multiple or unbound origins
remain unresolved. These candidates support a negative about one qualified finite alternative,
not a conclusion from silence or a whole-function transfer-absence claim. Retained candidates
are not discarded skip counts, and mapping source is included in provider build identity.

> Decision: ADR-0111

## The runtime view

The runtime-view extraction treats resolved `typing.TYPE_CHECKING` as false while preserving
source geometry. The same-length token substitution prevents ty from deciding by spelling;
strings and comments retain their original bytes. A parameter or unrelated attribute with the
same spelling remains an ordinary source evaluation. Sentinel collisions refuse extraction.
Supported `sys.version_info`, `sys.platform` and `os.name` decisions require the resolved stdlib
origin and the captured Python/platform context. Unsupported tests remain observations.

Checker-facing static branches and runtime-flow branches are separate views. Neither a type
observation nor a display string is an execution witness. Raw exception, import and protocol
observations acquire behavioral meaning only through the later typed owners.

## Conditions

### Evaluation atoms and identity

An evaluation atom identifies a specific predicate occurrence, its typed predicate/place and
source/context. Literals and polarity remain structural: equality, identity, membership,
truthiness, None checks, supported type/class tests and opaque evaluations are different
questions. Exact literal values are model-owned values; rendered source is not a value parser.

Same-spelling tests at different sites are distinct. A call may change a global, a closure or
object state; matching reaching sets alone do not establish cross-site equality.
`conditions::entry` and `conditions::stability` own the admitted identity and stability
witnesses. Guard identity is required before both truth arms, while a derived control proposition
retains the leaf's truth condition. Use, Value and Guard execution-domain proofs cannot substitute
for one another.

### Lowering from ty's diagrams

The provider lowers its decisions to model-owned evaluation atoms and conditions. Runtime-view
decisions precede structural Boolean lowering. An undecided provider reachability branch is a
may-path with explicit approximation. Opaque tests retain their source occurrence; provider
synthetic predicates do not become exact source evaluations by label.

Local retains the precise observation/support pairs and relevant statement regions. False source
branches can be absent from the raw provider inventory; that absence is not a negative behavioral
claim. A false condition produced by admitted later composition is a different, retained finite
question. A raise inside a `try` or `with` does not establish escape until completion/protocol
proofs account for the frame.

### Representation and the condition kernel

`domain::conditions` owns the bounded reduced ordered BDD kernel. Conditions persist as
content-addressed roots and ordered Merkle nodes over atom identities. Library variable indices
remain private. One hydrated validator checks closure, terminals, order, reduction, acyclicity
and recomputed identities across publication and replay.

The kernel owns conjunction, disjunction, negation, implication, compatibility, checked factoring,
bounded substitution and existential elimination. Operations charge the selected support, node
and work budgets. A limit produces its named obligation and Unknown; it never produces false.
The bounded DNF is presentation only and is never parsed to answer a question. Factoring accepts
a quotient only after the exact factor/quotient equality check.

### Typed primitive theory, value links and exact origins

The theory asks whether a condition can hold for an exact primitive value at an admitted entry
formal. Identity precedes value reasoning. `EntryValueWitness` requires the normalized owner and
formal, exact access/place, parameter definition, complete provider/context-qualified reaching
set and complete relevant Flow coverage. Assignment, unbound, loop-carried, mixed-owner or
nonlocal reaching prevents that proof.

The finite scalar whitelist and conservative nominal class/MRO reasoning consume structural
TypeTerms and attributed operand links. Display-only, open or truncated type domains cannot
establish a negative. Binding identity is not mutable-object stability. Guard substitution cites
the stored call binding, actual value, source evaluation and stability witness; it also retains
actual-place control influence. Missing, ambiguous or unsupported origin/substitution is an
obligation, not a guessed value.

## Value flows and call transfers

**Implemented / focused-Tested, 2026-10-03:** qualifications require a canonical assumption
basis, including an explicit empty set. Supported lower typing observations can introduce a
type-conformance premise for bounded predicate decisions. Empty, mixed, uninhabited or
nonconforming domains do not prune. A typing-conditioned Local restriction changes its actual
Summary guard while the conservative runtime alternative remains separately qualified.
Conjunction combines premises; alternatives retain distinct bases. S0 text discloses nonempty
bases and serving resolves their actual definitions rather than defaulting missing premises to
empty. The coordinator owns current S0/packet qualification; the scoped producer receipt does
not establish an unrestricted runtime result.

> Decision: ADR-0120

A transfer relates input and output places under one qualification. Identity means the same value;
Derived means a value computed from it. Identity composed with Identity remains Identity; any
composition involving Derived is Derived. `ControlInfluence` and `Selection` are separate
relationships and never compose as value transfers.

Local publishes direct assessments and their exact native premises. A value crossing an open
call remains an obligation until binding, evaluation, completion and an applicable finite callee
proof admit composition. Summary matches caller and callee by call site, restates only supported
formal atoms and retains invocation-distinct guards. Unknown dispatch or defaults do not become
normal return. Source signatures and effective signatures stay separate authorities.

**Implemented / focused-Tested, 2026-10-03:** a direct synchronous nested sole-return callable
can carry an outer source formal or one immutable primitive literal through a checked caller
frame. The model requires exact declaration/Identifier-child correspondence, native capture
support, complete unchanged source inventory and nonescape. It records the formal or literal
origin without inventing a nested formal or caller-entry read. Actual completed source/body
execution feeds the existing finite Summary transfer; mutation, global/nonlocal, loop, delayed,
unbound and escaped cases retain their refusal. Native enclosing snapshots, including lazy
candidate and unavailable states, characterize timing and supply no value authority. Arbitrary
captured-cell or heap analysis remains outside this lane. Scoped native/PG/replay receipts and
the remaining capture packet journey are owned by the code-facts coordinator.

**Implemented / focused-Tested, 2026-10-03:** bounded builtin exception values retain ordered
handler, reraising and finalizer behavior. Summary exception outcomes require actual completed
bodies and an exact empty-basis qualification. A normal outcome is evidence about that admitted
body under entry; a missing body or dynamic raise cannot establish absence. Behavioral Raises
selection and its body-entry packet remain distinct from native raised-type characterization.

Supported source constructor/field/reader associations retain declaration, store and read
premises. Their symbolic constructor→field→reader route is not a temporal runtime value proof:
uncertain reader alternatives remain Unknown with proof=None and their actual reason. Source
association depth is independent of the finite call-path proof limit.

## Verdicts

`domain::obligation` owns the one priority, discharge and verdict policy:

| Verdict | Meaning |
|---|---|
| Established | The exact qualified question has admitted positive support |
| Conditional | Its admitted positive support depends on the retained condition |
| RefutedUnderModel | An exact, definite false question has complete relevant coverage and a checked negative proof |
| Unknown | Evidence, applicability, alternatives, approximation or a budget leaves the question open |
| NotAnalyzed | The relevant analysis was not requested |

A positive finite witness is distinct from a claim over every applicable alternative. It cannot
close an open sibling. Candidate/Potential alternatives cannot establish, refute or discharge.
Catalog profile does not request Flow and retains NotRequested/NotAnalyzed outcomes. A behavioral
profile's Partial coverage is local uncertainty, not a blanket loss of its admitted positives.
Selection's Supported/Contradicted/Unresolved/Conflicting states are a separate contract.

## Negative premises

A negative needs a declared closed domain and exact membership evidence. Empty reads, flow tables,
exception tables or summaries are insufficient. Summary Refutation proofs retain the false BDD,
original qualification, exact invocation and complete native coverage members. Partial or erased
coverage, missing proof and unsupported applicability cannot acquire negative finding authority.
S0 preserves RefutedUnderModel as `BehavioralRefutation`, in Limits, at the structural evidence
floor. Documentary warnings cannot establish behavioral absence.

## Boundary reasons the behavior model adds

Reasons are append-only typed obligations, owned with their policy. Missing entry identity,
unsupported substitution/control flow, captured state, defaults, heap state, unresolved targets,
incomplete coverage and specific depth/proof/work/node limits remain distinguishable. A bounded
refusal names its subject/channel/phase and retains observed premises. Neither a count nor a
rendered reason is a replacement semantic relation.

## Read phase

A global or field read retains its source occurrence and execution phase. Module initialization,
call entry, normal/exceptional completion and finalization are different contexts. A later setting
mutation does not rewrite an earlier captured read; a declaration is not evidence of when its
runtime value was observed.

## Three vocabularies

Provider observations, normalized semantic identities and later qualified analyses are separate
layers. A provider label is not an entity correspondence; a normalized candidate is not a proved
runtime target; an analysis conclusion is not a raw fact. Nominal support families retain those
boundaries in the shared derivation graph.

## Identity

`lctx-model` content identities include their declared semantic keys, qualifications and source
premises. Alternatives with the same transfer aggregation key may merge by the owned OR operation
while retaining all witnesses. Worklist semantic states also retain their canonical condition;
proof ancestry, arrival order and cost are not semantic identity. Codebooks remain append-only.

## Composed layers

Local → BaseEvaluation → BaseCompletion → SourceCall → EnrichedExecution → Model → Summary is
an acyclic dependency order. Each later owner consumes completed nominal parents and cannot
supply an earlier proof to itself. `cpg-core::compilation` orchestrates those declarations;
`lctx-postgres` seals, validates and publishes their immutable generations. Later Structural,
Analytic, catalog and S0 consumers preserve the earlier qualification and uncertainty. Phase 5
will read the same generation without introducing a second behavioral authority.

> Decision: ADR-0045, ADR-0051, ADR-0085, ADR-0106, ADR-0108, ADR-0028, ADR-0064, ADR-0074
