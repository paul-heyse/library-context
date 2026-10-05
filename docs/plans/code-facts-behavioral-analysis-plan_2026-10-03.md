# Code facts: qualified first decisions and finite behavior

**Proposed, 2026-10-03.** The [coordinator](code-facts-expansion-plan_2026-10-03.md) owns current
disposition and acceptance. This plan owns C7–C10 and behavioral interpretation of C4/C6.
It consumes provider observations from [migration](python-analyzer-migration-plan_2026-10-03.md)
and model-owned identities from [normalization](code-facts-normalization-plan_2026-10-03.md).
Source diagnoses are Interface-checked; no behavioral improvement is Tested by plan authoring.
Enduring owners are behavior-model §3.9, behavioral-analysis §9.9 and model execution/transfer.

## 1. Preserve finite analysis and repair the missing first operation

Reuse current bounded BDDs, evaluation atoms, local theory/value contracts, CheckedExactClass,
completion, transfer, SCC summaries and programmatic assertion synthesis. Do not replace them
with a general interpreter, heap model or second type checker. Current local theory computes
PredicateResult, but inspected derivation/summary consumers do not use that truth result to
restrict conditions. Implementing that use without a premise contract would create new unsound
behavior; leaving it merely stored would leave expanded-review F02 open.

The target admits exact modeled source/runtime values unconditionally where current contracts
allow, and typing-derived refinements under explicit assumptions. Unknown/Refused cases retain
source witnesses. Catalog type characterization remains useful without proving runtime behavior.
Async flags/protocol candidates describe invocation; general scheduling/resource lifetime and
async completion remain unsupported beyond existing finite models.

## 2. B0: one claim basis, complete propagation

Extend AssertionQualification with a required canonical assumption-set reference, separate from
context, scope, condition, modality, approximation and fidelity/status. Empty set is explicit
and canonical. Assumptions are typed, attributed premises such as conformance to a specific type
observation or no-extra-overrides relative to a pinned modeled universe. Terms refer to lower
observations, unconditional identities or model definitions, **not the assessment whose own
qualification contains that assumption**; avoid content-key cycles.

Assumption members have deterministic sorted-set identity. Context/environment compatibility,
membership existence and premise support are model invariants. Empty vocabulary is supplied by
the existing core/preflight owner; late sets follow existing shared qualification/vocabulary
epochs at their producing boundary. Do not require preflight to predict every late analysis
premise or invent a global registry. Update stage input/grant/dependency closure so every reader
can resolve the set in its selected generation.

Composition is part of the shared model operation, not each renderer's policy:

- Conjunction validates compatible context/scope and unions the required premise sets while
  combining conditions under existing approximation rules.
- AlternativeUnion keeps separately qualified alternatives. Group/union conditions only when
  their premise sets and other governing qualifications agree. An unconditional path and a
  typing-conditional path never flatten into one unconditional or globally conditional result.
- Refused/incompatible premises remain explicit unknown/refusal, not empty-basis fallback.
  Reusing a truth witness across context, leaf evaluation or modeled universe is prohibited.

Migrate all existing qualifications, derivations, proof/witness identities, transfer, summary
composition, selection behavioral results, synthesis, retrieval candidate payloads and packets
to the required basis. P4 participates immediately in generated-wire migration, then supplies
the final cascade controls. Existing ordinary records use explicit empty sets. Old stored/wire
formats are hard-migrated; a missing premise field cannot mean unconditional truth.

A model-owned question policy decides whether a premise permits runtime refinement, a typed
answer, or only characterization. Do not introduce a universal provider trust switch. An answer
under typing conformance is conditional even when its type term is exact. Existing conservative
runtime summaries remain available; conditional derivations reuse the same analysis operations
under another qualification, not another analysis authority.

Acceptance includes mixed conditional/unconditional alternatives, conjunction deduplication,
cross-context rejection, changed model-universe identity and PG/wire premise resolution. Independent
expected sets expose dropped/spread assumptions. Amend owning sections and record the changed
behavioral meaning in an ADR before activating B1/B3 typing-based decisions.

## 3. B1: structural predicates and actual atom decisions

M3 supplies typed predicate nodes, evaluated source/operand identity, polarity, place and formula
role. Structural lowering preserves source evaluations and suppression/nonterminal distinctions.
Reachability AMBIGUOUS remains a documented over-approximation. Narrowing lowers
`uncertain OR (p AND true) OR (!p AND false)` and carries its precision-loss flag; it is not the
reachability three-way diagram or an unknown truth verdict. Limited ALWAYS_TRUE cannot justify
exact tautology. Binding/use-def candidates aid attachment, not a second type authority.

Introduce a model-owned AtomDecision operation taking a leaf/evaluation identity, attached
operands, interpreted domains, inhabitedness, coverage, question policy and basis. It returns
true, false, mixed/undecided, uninhabited or refused with support and qualification. Reuse current
PredicateResult computation but establish its exact correspondence and premise first. Empty/Never
domains yield uninhabited/refused runtime decision, never vacuous false/true branch evidence.
Any, gradual, contradictory or incomplete domains follow current bounded-domain uncertainty.

Wire AtomDecision into condition restriction and transfer, then Summary production. The truth
result must change a controlled finite analytical result, not just emit a theory support row.
Typing-based restriction produces its own conditional derivation with conformance premise;
it cannot erase the runtime conservative path. Literal/exact runtime state witnesses may decide
without typing assumptions when their current modeled contract proves them.

Native narrowing may help map a use to its defining operand, but does not itself prove Python
branch execution or preserve effect-free evaluation. Recognize builtins/TypeGuard/TypeIs by
resolved contextual identity, not spelling. Ruff side-effect/truthiness helpers are **screening**
evidence only; an exact bounded evaluator/transfer certificate remains the permission to prune.
Preserve evaluation order and repeated-evaluation atom identity.

Controls: literal true/false, mixed union, empty domain, shadowed builtin, predicate alias,
effectful repeated test, unknown attachment, uncertain-edge OR, precision overflow and
`x: Literal[0]` called with a nonconforming runtime value. The last keeps a conservative runtime
answer while the conditional typing answer names its premise. Verify leaf → decision → restricted
transfer → Summary → P4 packet, using independently specified finite expected outcomes.

## 4. B2: capture identity and a bounded value bridge

Normalize captured/global/nonlocal place origin, declaring/enclosing scopes, mutability,
evaluation timing and definition candidates. Pyrefly capture names and ty eager/lazy snapshots
do not certify call-time value. Lazy AssumeBound is explicit candidate qualification. Captures
first improve precise refusal explanations and P1 callable-dependence attributes even where
concrete value transfer is unavailable.

Pyrefly-origin capture characterization remains available in Catalog without ty Flow. The
additional timing/state proofs and stable-value bridge run only when behavioral analysis is
requested; NotRequested is distinct from unavailable or absent capture evidence.

Initial value support is deliberately bounded: a direct synchronous nested call within its
own outer frame, a nonescaping callee, a local immutable literal or admitted entry value,
one dominating definition, no intervening write, and verified exact call-time conditions.
Lower this as a modeled captured formal/entry transfer using caller read proof and scope identity.
No persistent heap-cell alias is invented. Higher-order escape, global/nonlocal mutation,
loop-created late binding, delayed coroutine/generator execution and unbound captures remain
explicit CapturedStateUnavailable (or a more precise existing boundary) with source origin.

Retire the blanket refusal only for this admitted subset. Stable literal and stable entry value
cases must improve an actual finite summary. Mutated-after-definition, called-before-assignment,
loop captures and returned escaping closures must not. No claim that capture extraction removes
captured-state refusals generally. S0 describes the located unsupported dependence.

## 5. B3: dispatch, protocol phases and completion

One model-owned closed-target assessment consumes resolved call/member candidates, class metadata,
MRO coverage, receiver witness and universe. Its result records exact-runtime, typing-conditional
or open basis and unresolved reasons. Exact runtime class plus complete modeled MRO may establish
a finite target. `typing.final`/NoReturn require conformance/no-extra-override premises; a protocol
signature is not an implementation and enum members are not a closed heap. Preserve higher-order
parameter/receiver candidates and applicability uncertainty; never certify closure from a list
being nonempty. Call binding/transfer use this assessment rather than trait booleans.

Audit all Pysa origin kinds rather than pretending implicit calls are all missing (the migrated pin exposes 24).
Retain operator/subscription/iteration/enter action, evaluation site, receiver, candidate target
and phase. Ruff argument lookup supports source argument identity; normalized signature binding
owns semantics. Candidate protocol arcs can characterize/project behavior without certifying
effect-free completion. Generator/coroutine creation and later execution are separate phases.

There is no Pysa WithExit origin. Consume M3's explicit native exit observations with exception
and normal argument results, applicability/diagnostics and receiver/member basis. Model-owned
suppression admission distinguishes known suppressing, known non-suppressing and unknown. Do not
negate one helper bool to get the other; broad bool/Any/error typing results do not prove always
suppresses. Interpret literal truth only under the necessary type-conformance premise or an
existing exact exit model. Typed observations alone do not close exit effects or exceptions.

For synchronous with, compose existing entry/body/ordered exit finite models, preserving active
exception, reverse order for nested managers, exit-raised replacement and false/true suppression.
Each needs an admitted model certificate; an inferred exit result is only a candidate when
effects are open. Async enter/exit payloads can be served/projected with correct awaited phase,
but unsupported async completion stays Unknown. No new async lifetime engine is scheduled.

Terminal calls consume native callee/declaredness/receiver/body-kind evidence plus closed-target
basis. A Never result from a Never receiver is not divergence. An inferred Never method that
can be overridden, or a NotImplementedError placeholder, is not unconditional terminal behavior.
Declared NoReturn characterizes a typing promise; finite transfer may use it conditionally.
Exact pinned modeled terminal functions can retain their existing unconditional certificates.

Controls include final-declared-but-runtime-overridden, inherited MRO gap, higher-order unresolved
target, operator implicit phase, iterator next versus creation, Never receiver, inferred method
override, declared NoReturn, known/unknown exit, manager suppression and exit replacement.
Unknown/effectful and async manager cases remain bounded. No broad AMBIGUOUS-region closure
follows from terminal or exit facts alone.

## 6. B4: exact exception values and Python-ordered handlers

Use existing source identity/MRO and CheckedExactClass first; this can proceed before M2.
Admit a small explicit builtin exception constructor set: TypeError, ValueError, RuntimeError
and Exception, using pinned builtin identities and existing modeled value semantics. Arguments
must have completed in order under the current evaluator; never drop their possible effects or
exceptions. No arbitrary user constructor execution. Native raised types remain upper-bound
typing characterization, not complete may-raise sets.

Match ordinary source handlers in Python order, including tuples/subclasses, using exact modeled
exception identity. Preserve active exception, named-handler disposal, else-only-on-normal body,
finally replacement and bare reraising. Handler lookup/evaluation failures retain their effects
or unsupported boundary. Non-exception classes do not match by name. Unknown raised identity
cannot jump to the first convenient handler. Except-star/groups remain explicitly Unsupported.

Typed handlers replace the current first-bare-handler-only restriction where exact contracts
support them. Feed BaseCompletion/Enriched completion, finite Summary and behavioral Raises
selection/S0; structural Raised type queries stay distinct. Fixtures specify earlier broad
handler versus later subclass, tuple order, unmatched raise, finally return/raise replacement,
reraising, named disposal, dynamic constructors and unsupported groups. This is a local finite
semantics enhancement, not universal exception propagation.

## 7. Integration and evidence

| Package | Prerequisite capability | Completion includes |
|---|---|---|
| B0 | Current qualification/vocabulary epochs | All old qualification consumers migrated, shared composition/invariants, P4 required-wire basis controls |
| B1 | B0, M3 predicate precision, N5 operand roles | Actual conditional truth restriction/transfer/Summary and output trace; empty/mixed/nonconforming controls |
| B2 | B0, M3 capture/timing | Capture characterization/refusal plus stable-case summary bridge and negative timing cases |
| B3 | B0, M3 protocol/exit, N3 metadata, N5 receiver/member | Dispatch/phase/completion consumers with exact versus conditional basis and open-effects controls |
| B4 | Current exact-class/source model; B0 for typed refinements | Exact constructor/handler/completion/Summary and raise consumer, unsupported groups retained |

Model declarations/validators, stage inputs, support/prerequisite grants and shared vocabulary
epochs move with their consumers. Charge new premise/predicate/capture/candidate storage and
iteration to existing budgets; on exhaustion preserve explicit Partial/Unknown, not silent exact
results. No uncontrolled Cartesian product of alternative premise sets. Deterministic bounded
dedup may return an operational limit without discarding a conservative result.

Run targeted release-profile model/core tests and real disposable-PG publication controls for
new references. Known truth/completion expectations come from explicit Python semantics and
existing independent oracle fixtures where applicable; do not compare only two consumers of the
same disputed lowering. New runtime probes are selected for material uncertainty, not required
for every row. Q0 owns actual dual-profile journeys and assembled `just qualify` after all functional scope
(ADR-0126), with full keep-going Clippy and applicable leaves on one tree.

All planned evidence here is **not_run**. Completion requires first semantic use and qualified
output, not merely persisted facts or source review. Expanded coverage is Proposed; total runtime
coverage, speed, memory and retrieval benefit remain unmeasured. Capture/async/dynamic/exception
limits must survive current documentation and served explanations after implementation.
