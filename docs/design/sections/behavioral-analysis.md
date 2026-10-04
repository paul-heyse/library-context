<a id="section-9-9"></a>

# §9.9 Summaries, models and the capability registry

**Implemented / Tested within Phase 4 receipts, 2026-10-01; incremental alignment implemented 2026-10-02.** `lctx-model::domain::execution`,
`transfer`, `models` and `obligation` own behavioral operations, evidence and replay. Nominal
analysis families record each producing owner. `cpg-core` loads completed inputs and publishes
their typed outputs through the generation store; `lctx-analytics`' native entry points re-export
the model-owned computational kernels. The [Phase 4 plan](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md)
owns its scoped independent controls and dated qualification. The alignment coordinator owns
current corrections; real-library serving qualification remains stopped. No phase-exit or pilot claim
follows from these implementation descriptions.

The dependency order is Local → BaseEvaluation → BaseCompletion → SourceCall → EnrichedExecution
→ Model → Summary. Each owner has its own input closure, canonical definition, invocation,
qualification, coverage and replay. An invocation names actual completed parents, not a fabricated
success for an unavailable analysis. Shared derivation DAG validation covers typed premises across
owners. [§15.6–§15.9](semantic-model.md#section-15-6) are the governing relation contracts.

> Decision: ADR-0085, ADR-0106, ADR-0108, ADR-0045, ADR-0054

## Action triggers

An action describes an effect, callback, resource or exception at a declared phase. Reaching a
call's ordered eager inputs is distinct from normal return, exceptional completion and finalizer
execution. Model activation requires the exact bound target and applicable phase; syntax containment
alone cannot prove that an action ran.

`execution::model_rules` consumes admitted binding/evaluation and modeled outcomes. Triggers retain
source origin, target, action phase, supporting invocation and condition. A reached invocation
can support an invocation-phase action without a normal-return proof. Normal, exceptional and
finally actions require their corresponding outcome witnesses. Unknown argument completion or
an unproved source/default binding remains an explicit boundary. Stored validators replay these
operations, rather than reclassifying rendered action text.

> Decision: ADR-0060, ADR-0106

## Pinned call default availability

An omitted formal's runtime default must be independently available. A signature default
expression is not evidence that it was evaluated or remains stable, and successful callee return
cannot establish its own input availability. Explicit source-ordered actuals and omitted defaults
are separate binding roles.

The model catalog may declare `call_defaults_available` for an exact pinned target/phase. The
bounded source-default domain instead cites definition/header execution and the supported
availability/stability proof. Every required omitted formal has an exact membership commitment;
missing, foreign, duplicate, reordered, unstable or oversized groups refuse call admission.
Availability, exact value and stability are different propositions. A model's availability promise
does not promise the default's value or normal completion.

> Decision: ADR-0061

## Normal-exit postconditions

Body completion, frame release and caller continuation have independent owners and premises. A
pinned direct-return-parameter body states a narrow body domain; releasing retained temporaries
or frame roots can still invalidate caller state. A normal result cannot certify its own release.

Supported postconditions retain ordered execution, result origin, retention/release groups and
exact identity assumptions. Enriched evaluation/completion consumes earlier source-call evidence;
base execution does not depend on that enrichment. Recursive effect, arbitrary heap stability,
async/generator lifecycle and broad release protocols are outside the restored envelope and remain
uncertain.

> Decision: ADR-0062, ADR-0063

## Models catalog

An authored model is typed committed data about a pinned external callable or context protocol.
`domain::models` parses tagged records with unknown-field rejection; its catalog bytes, revision,
target pin and applicability are identity-bearing. `models::requirements` selects dependency
symbols, protocol members and exception/MRO context before extraction. Missing pinned definitions
remain model applicability uncertainty; compilation never imports the operator's environment.
Skills and evaluation gold are not model/compiler inputs.

A callable model declares:

- target origin and exact Python/distribution applicability;
- call/execution phase and independent transfer/effect/callback/resource/exception coverage;
- typed source/target access paths and Identity or Derived value rules;
- action/effect/resource/exception rules, retaining potential alternatives;
- optional runtime-default availability and a narrowly declared normal body.

Paths use structural roots/segments, not a rendered grammar. Effect kinds retain their declared
meaning, including I/O, network, logging, timeout, dispatch, compression, serialization,
validation, registration and invocation. A symbol or exception class binds through its normalized
pinned identity, not name suffixes. Complete MRO evidence can support conservative class reasoning;
an open class family cannot establish nonmatch.

Channel coverage is independent: a complete value model does not close exceptions or effects.
An unreferenced model remains dormant. Model applications retain exact binding and source
premises, model revision, phase and coverage. Unknown or conflicting applicability stays visible.
The canonical definition includes the selected authored catalog and executable source digest.

> Decision: ADR-0055, ADR-0056, ADR-0063, ADR-0106

**Finite model extension route (Implemented guidance, 2026-10-02).** Extend the existing typed
model case and checked constructors; declare its native/normalized inputs at the owning producer
and replay validator. Wire the application/summary operation through those checked values, keeping
source/default binding, exception and path boundaries explicit. Add an independent positive twin
and a missing/crossed-premise refusal. Model catalogue declaration does not alone establish support
or checked construction; no store or Python adapter assigns model meaning.

## Synchronous context protocols

The supported pinned synchronous protocol domain separates resource allocation, initialization,
entry and exit. The acquired resource, constructor result and `with ... as` value are distinct
identities. Entry may supply None or an admitted argument value. Exit may preserve a pending
outcome or suppress only the declared exact exception-class domain.

`execution::context_execution`, `context_binding` and context transfer operations retain ordered
binding/evaluation, protocol applicability, resource identity and outcome premises. Cleanup is
not inferred from lexical `with` containment. Missing context entry/exit, uncertain exception
matching or unmodeled resource identity refuses the narrower proof. No general context-manager,
async protocol or arbitrary suppression engine is implied.

> Decision: ADR-0059

## Model application at source calls

A source call uses normalized event/target alternatives, bound signature variants and exact
actual/formal/default roles. SourceCall requires independently admitted header, body, binding,
default and release inputs. EnrichedExecution consumes those results in a later stage. Model
applications additionally require pinned target and phase applicability.

Argument evaluation, call entry and normal return have distinct proof records. A receiver remains
an explicit binding role. An incomplete effective signature does not erase a known source
signature. Ambiguous targets, unsupported dispatch and defaults retain specific obligations.
Base evidence cannot acquire a later model/source-call conclusion as its own premise.

## L2 fates: exits, handlers and finalizers

The stable heading names the exit domain; the implementation is the typed completion owner,
not a separate L2 runtime. Structural return/raise observations identify source events and
regions; they establish neither escape nor normal completion.

`execution::completion_records` and the base/enriched completion producers retain explicit
Return, Raise, Break and Continue outcomes, active exception state and ordered pending-outcome
replacement. A normal finalizer preserves the pending outcome; an abrupt finalizer replaces it.
A raise in a `try`/`with` needs the matching handler/protocol and finalizer evidence before it
can establish escape. Bare re-raise uses the actual active exception.

Handler matching uses exact pinned class/MRO evidence and ordered precedence. Nonmatch requires
complete relevant class knowledge; absence from an open MRO is insufficient. Opaque exception
construction, named-handler cleanup, exception groups, unsupported loops and broad user
finalization remain explicit refusals. Completion depth/work and proof limits have distinct causes.
Each supported statement and refusal belongs to a recomputed inventory; missing output cannot be
silently reported as complete.

## Transfer summaries

`execution::summary_replay` is the final finite Summary producer/replay boundary. It consumes
completed native/Local/execution/model evidence and normalized dispatch topology. The borrowed
petgraph invocation projection supplies a canonical callee-first SCC schedule; graph indices
never become semantic IDs. Incomplete topology withholds completeness without erasing finite
positive evidence.

The SCC-local worklist distinguishes owner, origin, ports, channel, phase, transfer kind,
modality, approximation and canonical condition. It excludes proof ancestry, arrival order and
proof cost. Nondominated depth/cost representatives can reopen dependent work; equal-cost
witnesses remain separate evidence. Selected defaults preserve call-path depth 8 and proof-step
limit 64, with versioned work, expression, completion and BDD limits. Deterministic queue and
expansion charges distinguish work exhaustion from dominance or missing evidence.

Composition matches the exact call site and binding. Entry-value and stability proofs justify
caller ports and supported guard substitution. Invocation-distinct evaluations remain distinct
atoms; opaque guards are not erased or conflated to claim Exact convergence. Guarded recursion
may retain different finite conditions through the limit. Subject-specific residuals retain
SummaryDepthLimit and their semantic ports/channel; an over-approximate residual cannot establish,
refute or discharge. A base-free cycle stays open.

Finite witness occurrences cite earlier witnesses or raw evidence, exact binding/substitution,
and checked depth/cost/rank equations. Final aggregate alternatives are emitted after component
closure and cite those witnesses. Witnesses do not cite final aggregates. The shared derivation
DAG validator remains active. Transfer aggregation ORs admitted conditions and retains witness
membership; evidence enumeration and response witness limits do not erase an established result.

`summary_consequences` distinguishes finite value alternatives from complete call-member closure.
Every applicable target/variant/origin remains Proved, Open or Excluded with its own cause. One
successful path does not close another origin or an unknown target. `domain::obligation` owns
priority, admissible discharge and the five behavioral verdicts. Coverage membership is retained
independently of witness deduplication; an empty channel alone proves nothing.

A Refutation proof requires a definite exact false BDD and complete relevant native coverage,
with exact invocation, qualification and coverage-member digest. Unknown, proof=None and Partial
coverage do not acquire finding authority. S0 consumes the nominal Summary derivation and keeps
that negative as BehavioralRefutation rather than an applicable-case positive.

**Implemented / focused-Tested, 2026-10-01:** the native provider retains a direct bare-name
identity value candidate when its declared expression-selection condition is checked false.
The actual native use and sink govern Entry/Local admission. A nonempty, wholly exact-false
native reaching inventory without loop headers retains every Defined/Unbound alternative;
live/approximate inventories keep the prior pruning. Missing or multiple formal origins remain unresolved; computed values and call crossings stay
refused. This supports a negative about that exact qualified finite alternative, not an inference
from an empty table or a closed claim that a parameter can never reach any return. Resolved
`TYPE_CHECKING` supplies the existing runtime-false control. Native mapping source participates
in the provider build digest; no guard evaluations are identified or coverage strengthened.

> Decision: ADR-0111

Symbolic constructor→field→reader associations have a separate uncertainty domain. The normalized
association is available in both profiles; Local adds Entry/Flow premises where admitted. Summary
retains Unknown reader alternatives with proof=None and the actual boundary. Source route depth
does not establish temporal heap identity or consume a finite recursive call-path proof.

> Decision: ADR-0052, ADR-0053, ADR-0057, ADR-0064, ADR-0085, ADR-0106

## Native serving boundary

**Implemented Phase 5 contract, 2026-10-02; wider qualification stopped.** The model native preparation
reads the same pinned PostgreSQL generation and model-owned conditions, proofs and obligations.
It validates declared required inputs, preserves qualifications and follows bounded derivation
witnesses. Private/nested callable evidence may support a public operation's proof without entering
public lookup. There is no legacy bundle-format reader or second semantic interpreter.

Path-local exact-input inspection is distinct from operation-wide completeness. A query cannot
upgrade Unknown, truncate an established conclusion away or prove a negative from unvisited
operations. Response row/node/work/depth budgets report unknown or truncation with generation-bound
continuation. [§11.3](synthesis-and-serving.md#section-11-3) owns the MCP boundary.

## The capability registry

**Proposed product/research capability; not activated by Phase 4.** A concept has an authored
identity, preferred/alternate labels with sources, scope note, broader/related relationships and
a typed conjunctive definition over admitted relations. Broader is acyclic; related is disjoint
from its closure. Materialized membership would cite the definition digest, qualified bindings,
verdict and witness.

Discovery may nominate concepts through FCA or retrieval, but definitions decide membership.
Analytics never writes behavioral facts by label. `lookup_concepts` and `explain` retain their
future interface intent; they do not claim a current served capability. Product PR6 and general
ontology expansion remain paused.

> Decision: ADR-0028, ADR-0050, ADR-0085

## Channel composition contracts

Channel and phase are typed components of each question. Value/Call, execution completion,
effects, exceptions, callbacks and resources do not borrow one another's coverage. A site-level
escaping-exception domain differs from generic raise/catch/convert/suppress activity. Model
coverage states phase applicability, and a dynamic validation schema is not a named static schema.
Actions before a raise need reached-prefix evidence.

Supported finite composition and modeled protocols preserve these distinctions. General recursive
all-channel closure and arbitrary lifecycle/heap semantics remain outside the restored envelope.
The active plan records implementation and qualification; future product needs can activate a
bounded extension without reintroducing legacy tables or a parallel policy owner.

> Decision: ADR-0058, ADR-0106, ADR-0108


## Qualified target and terminal questions

**Implemented, 2026-10-03.** Model distinguishes exact runtime closure, typing-conditional
closure and open dispatch. Source inspection and a Candidate Overrides route do not close
runtime targets. A separate typing question requires definite receiver evidence, a unique
checked native member, complete MRO, final metadata and explicit receiver-conformance and
no-extra-overrides premises with their actual authored model.

Protocol origins retain their exact phase. Native NoReturn and exit types characterize typing;
they do not establish divergence, runtime suppression or completed execution. A terminal
frontier concerns the exact following statement of a direct sequential call **given invocation
entered**. Effects, exceptions and cleanup remain unknown. SummaryTerminalWitness retains
that question, frontier, restriction and qualification as StructurallyObserved, without a
finite ClaimConclusion or body execution proof. Scoped composite controls are **Tested** in
the code-facts coordinator §7; terminal serving is Implemented with qualification pending. The behavioral profile requests Model/Summary; default Catalog retains NotRequested. Final assembled qualification remains open.

DeclaredClassInspection may use genuine report-projected function, symbol and parameter
declarations with exact native question/support identity. This describes a reachable source
class, without runtime class, allocation, state or complete member-universe authority.
Executable reads continue to require native structural support; a recognizer or mismatched
fidelity cannot supply it.

> Decision: ADR-0120
