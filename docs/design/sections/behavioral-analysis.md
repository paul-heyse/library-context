<a id="section-9-9"></a>

# §9.9 Summaries, models and the capability registry

This page owns how behavior propagates beyond one expression: the **models catalog** (L4) that
gives meaning to stdlib, dependency and framework callables, **model application** at source
calls, **L2 fates** (exits, handlers, finalizers), **transfer summaries** (L3) that resolve
`call_transfer`, the **native serving boundary** for their proofs, and the target **capability
registry** (L5). It consumes [§3.9](behavior-model.md#section-3-9)'s places, conditions, verdicts,
value flows and condition catalogs, plus Pysa call targets and pinned context definitions
([§3.2](facts-and-identity.md#section-3-2), [§4.0](acquisition-and-extraction.md#section-4-0)).
Its outputs are model, application, fate and summary relations in one snapshot, consumed by
serving ([§11](synthesis-and-serving.md#section-11)) and later by registry membership.
Dependencies point from orchestration (`cpg-core/src/attempt.rs`) to relation acquisition
(`cpg-core/src/summaries.rs`) and the pure finite summary producer
(`lctx-analytics/src/summaries/finite.rs`, over typed inputs and outcomes), and to `cpg-schema`, which declares
the catalog (`models.rs`, `crates/cpg-schema/models/external.toml`), the relation schemas and
DataFusion derivations (`behavior.rs`), rules and codebooks. The shared validator is
`cpg-core/src/validate.rs`; focused fixtures are under `fixtures/python/` (`model_shapes`,
`model_handler_shapes`, `handler_shapes`, `return_completion_shapes`, `behavior_shapes`,
`pysa_tito_shapes`, `summary_caps`), exercised by `crates/cpg-core/tests/`. See the
[architecture map](../README.md).

**Evidence.** Models, their source application, the L2 relations and the finite summary producers
below are **Implemented and Tested in focused cases** (2026-09-24/25; focused release Nextest,
each with a positive and a withholding fixture and shared publication equality). Recursive
value paths through an SCC-local worklist and one exact literal-controlled recursive path are
**Tested in focused cases** (2026-09-26); general conditional recursive composition,
effect/exception/role summaries, `call_transfer` discharge,
operation-wide serving and the registry remain **Proposed** targets. Integrated Stage 3 acceptance and the pilot are
`not_run`; the plan owns the queue and the current disposition of the findings cited here
([plan §3, §6](../../plans/behavioral-model-forward-plan_2026-09-24.md#3-stage-3-execution-queue)).
The rationale is ADR-0045 and ADR-0050.

**The organizing rule.** Meaning comes from models, propagation from summaries: a call alone never
propagates an effect or capability. Every step from a source call to a summary is a separate,
cited relation, and each one states what it does *not* prove. Absence from a positive relation is
unknown unless an independent coverage relation establishes a specific negative claim.

## Action triggers

**Accepted target; source/Delta subset Implemented and focused Tested (2026-09-27).** Every authored effect,
callback and resource action names Invocation, Normal, Exceptional or Finally. Invocation proves
reached and bound callee entry; the exit variants require an independently proved outcome of that
exact callee. Finally accepts either callee exit, never an enclosing finalizer or pending caller
outcome. Potential modality remains potential even when its trigger is established. Missing
trigger or required subject evidence stays an explicit unresolved candidate; an authored subjectless
effect requires no invented subject. Input arguments can be
available at entry; a returned resource requires Normal and separate returned-value identity.
Positive actions do not establish complete channel coverage. Schema owns shared admission,
analytics composes it, core reconstructs publication, and native consumes the same contract.

`action.rs` owns candidate-backed assessments and shared structural admission; `actions.rs`
composes existing call-input and normal-expression evidence. Catalog format 7 requires effect
triggers and appends Invocation to the existing selector. Partial I/O/logging retain Potential
at Invocation; completed serialization/compression require Normal. Every existing candidate has
an assessment, including a named refusal. Candidate references retain channel, descriptor and
modality; no effect-shaped value flow is invented. Subject bindings match the same call's actual
arguments. Normal/Finally require matching whole-call normal evidence and the existing 64-step
bound; exact exceptional-call outcomes and returned-resource identities remain unsupported.
Publication reconstructs the producer and its upstream source relations. The independent
[partial-I/O challenge](../../design_review/evidence/2026-09-27_action-triggers/README.md) distinguishes
reached calls with and without writes. Native action export/selection and full semantic support
remain S6 work; structural admission does not replace raw-fact source validation.

> Decision: ADR-0060

## Pinned call default availability

**Implemented and focused Tested (2026-09-27; compiler95/catalog6).** An exact pinned callable may author
`call_defaults_available` independently of its normal-return assertion. The promise concerns
already-created defaults for every omitted optional fixed formal in each admitted signature;
it supplies no default value, control truth, mutable-value stability or omitted-argument subject.
Known requiredness and cross-signature agreement remain mandatory. The common invocation owner
cites the availability model and omitted-formal context facts without evaluating default
expressions at the call. Source publication reconstructs the same evidence. The normal-return
shortcut is removed. Independently bound omitted-formal count/digest survive refused or removed
proofs; shared admission requires the exact bounded root evidence group, rejecting missing,
duplicate, foreign, reordered and oversized groups. Nested groups require their own independent
binding commitments before flattened invocation admission.

The individually qualified targets are `json.dump`, `json.dumps`, `json.loads` and `gzip.compress`.
Their ordinary omitted-default calls reach invocation while normal completion remains unproved.
Removing only the model availability premise withholds all four source calls and invalidates
publication. The [independent pinned-runtime qualification](../../design_review/evidence/2026-09-27_pinned-defaults/README.md)
checks runtime default creation and seven generated entry/outcome controls. Pure tests cover
signature agreement, requiredness and work bounds. Broader local-default/binding domains and
native action support remain open; no omitted subject is manufactured.

> Decision: ADR-0061

## Normal-exit postconditions

**Implemented and focused Tested (2026-09-27; compiler96, source/Delta subset).** A separate relation preserves the exact pinned Normal rule as an implication:
under a proved reached/bound invocation, if that exact callee returns normally, the authored
effect/callback/resource action holds with its declared modality. The Normal obligation remains
undischarged; no outcome feasibility, expression completion, action occurrence, transfer discharge
or coverage is inferred. Existing activated assessments retain ADR-0060's proved-trigger rule.
Both consumers compose the same schema-owned candidate/input/invocation admission.

A Normal acquisition's return-value path is a symbolic result inside that implication. It is
never a runtime resource identity, alias or acquire/release pairing. Input subjects retain their
bound identities. Only Normal rules are initially represented; Exceptional and Finally require
separate outcome domains. Composition and native serving must preserve the exact occurrence,
phase and pending outcome obligation. Unsupported consumers cannot treat it as an active claim.
Source/local-callee completion can discharge an obligation only with a closed normal-completion
domain under that occurrence's admitted inputs and condition. See the active plan's S3/S4/S6 queue.

`modeled_action_postconditions` retains a mandatory Normal obligation independently of the
candidate's descriptor and modality. Removing or replacing it, swapping occurrences or deleting
invocation/binding evidence invalidates shared admission or source reconstruction. The common
invocation owner serves both relations; core publishes them from one pure composition result.
Seven source cases qualify serialization/compression/registration implications while their
activated assessments keep unproved-outcome refusals. Symbolic acquisition has pure contract
controls only: explicit `open` retains `summary_proof_limit` under its overload evidence, and
ordinary `open` remains unsupported. No cap is raised or resource identity inferred.
The [six-program independent challenge](../../design_review/evidence/2026-09-27_normal-postconditions/README.md)
distinguishes normal completion from partial failure and registration from later invocation.
Native export/selection, source/local outcome discharge and concrete resource fates remain open.

> Decision: ADR-0062

## Models catalog

A model is committed, typed data about a callable we do not analyze from source.
- **What a model states:** transfers (identity or transform, definite or potential), effects,
  callback actions, resource actions and exceptions of a pinned stdlib, dependency or framework
  callable, over the resolved access-path grammar below. Effects come from an append-only
  codebook: `io.read`, `io.write`, `net`, `log`, `timeout`, `thread_dispatch`, `compress(format)`,
  `serialize(format)`, `validate(schema)`, `register(container)`, `invoke(callable)`.
- **Typed, not a string grammar.** Serde tagged enums with unknown-field rejection parse the
  committed TOML; typed path components determine stable path ids and one renderer writes the
  display path. Consumers never parse a rendered path (`model_formal_paths` publishes typed
  formal roles).
- **Identity and provenance.** Model ids are append-only. Source bytes, target pin and revision
  enter model identity; the catalog digest joins `compiler_digest` and the extractor's producer
  identity. Every derived row has origin `synthetic_model` and an authored rule id.
- **Authorship.** The operator, from pinned library source and official docs, **never from
  `.claude/skills/`** (the gold is evaluation-only).
- **Binding.** A model binds to a pinned `context_definitions` row only when the module origin and
  exact Python or distribution pin match; `model_targets` cites the module and definition facts.
  An unreferenced target stays dormant. A referenced target with an incomplete signature, an
  unresolved formal, or an unresolved or ambiguous exception class fails before any Delta write.
- **Exception classes are pinned identities.** The extractor retains the class definitions named
  by committed exception rules in the context modules it already describes, and the compiler binds
  every authored source and conversion class to a unique pinned class row; it never matches by
  name. A class in an unvisited context module is a visible compile failure, to be fixed by
  describing that module, not by a text match.
- **Channel coverage.** Each target declares transfer, effect, callback, resource and exception
  coverage independently as complete, partial or unspecified. Only complete coverage can support a
  negative summary conclusion.
- **Phase applicability (Implemented, 2026-09-26).** Catalog format 7 requires an authored phase
  per model. The same target may have distinct models and channel coverage for different phases;
  target/phase pairs are unique and phase participates in model identity. `model_targets.phase`
  is reconstructed from the catalog. A provider observation activates a model only on exact phase
  equality; it cannot widen applicability. The existing declarations explicitly select `call`.
- **Body and frame completion (Implemented and focused Tested, 2026-09-27; ADR-0063).** The optional typed
  `normal_body = { kind = "direct_return_parameter", name = "val" }` replaces unqualified
  `normal_return`. It asserts only the exact pinned eager body domain: direct return of that
  parameter, no other value roots and no user code or invalidation of caller retainers.
  Full callee completion additionally requires ordered completed arguments, all-signature
  binding and a complete frame-release certificate. Closed values and caller-retained values
  are distinct; retained values qualify only over this effect-free invocation window. Available
  defaults remain insufficient for release safety. Complete catalog channel coverage also remains conditional on the frame-release domain;
  cleanup effects cannot become negatives from body coverage alone. Invocation and Normal postconditions survive
  a refused frame outcome. Source bodies must reuse the completion kernel with separate release
  evidence; definition-header completion and a body-return event prove no caller continuation.
  Compiler97 persists ordered bound-argument and invocation commitments; source publication
  reconstructs them and native serving checks shared structural admission. The bounded fresh
  source-call domain below composes body completion; general source-body completion and full
  native raw-signature/read closure remain open.
  The [independent controls](../../design_review/evidence/2026-09-27_frame-exit/README.md)
  observe finalization before continuation; they are not compiler false-positive reproductions.

**Source-body foundation (Implemented and focused Tested under ADR-0063, 2026-09-27).** The existing completion
kernel evaluates a zero-parameter synchronous, undecorated body's suite under entry, separately
from its definition header and independently of value-flow seeds. Fallthrough Normal, explicit
Return, exact primitive-raise TypeError and unknown remain distinct. Closed expression values
and first local initializations support a separately committed local release domain. Reads of
locals require the exact executed initialization before acquiring closed-value provenance;
unproved reads, defaults, closure/cell obligations and dynamic definitions remain withheld.
The callable object itself still needs an external retainer. A body/local-release certificate
therefore cannot establish caller continuation alone: its consumer must prove the exact function
object remains held, initially through an immediate fresh nested definition's live binding and
an effect-free body. Source publication reconstructs both domains; call composition and serving
must preserve this independent retainer obligation.
Compiler99 persists `source_body_completions`, `source_body_steps` and ordered
`source_body_release_inputs`. Terminal anchors and release-root segments are shared structural
obligations; publication independently reconstructs their source meaning. Leading docstrings
remain definition metadata; a docstring-only body has zero runtime statements and no invented
proof. Indexed preparation preserves the existing suite, work and 64-step proof bounds. The
fresh-call consumer also requires a sole exact zero-argument source target, successful fresh
header creation, the immediately following bare call and the live lexical function binding.
`source_call_normals` and its header steps commit those premises separately from the body.
Lexical reference identity and syntax name identity are joined explicitly, never interchanged.

**Fresh source-call consumption (Implemented and focused Tested, 2026-09-27).** Preparation has
an explicit acyclic order: pinned expressions and base bodies, fresh-call admission, then enriched
caller expressions/completion. Base bodies cannot depend on source-call certificates. Publication
reconstructs both outputs once from source; persisted results are not their own premises.
Normality under expression entry does not establish reached invocation. Closed ignored results
can be discarded, while unknown local-read release, defaults, captures, aliases and intervening
statements remain withheld. Nested pinned calls retain their independent callee-owned frame
proofs. Consumers charge expanded header/body evidence against the original 64-step bound,
including invocation prefixes and Normal/Finally actions. FORMAT 9 exports referenced bodies and
call certificates with shared admission and row/byte limits; missing groups and foreign-caller
substitution refuse. Full raw-signature/read reconstruction and other S6 obligations remain open.

> Decision: ADR-0063

**The current catalog** (`external.toml`; pinned CPython 3.14.7 unless noted):

| Target | Asserted | Left open |
|---|---|---|
| `typing.cast`, `typing.assert_type` | Identity transfer; typed direct-return body; frame release separately required | — |
| `builtins.print` | Potential `io.write`, no subject stream | Which stream |
| `json.dumps`, `json.dump` | Potential `obj` → return transform (`dumps`); potential JSON serialization of `obj`; potential `io.write` on the bound `fp` (`dump`) | Custom encoder and `default` callbacks; completion |
| `json.loads`, `gzip.compress`, `gzip.decompress` | Potential input → return transforms; `compress(gzip)` on `data` for `gzip.compress` | Malformed input, resource exhaustion, JSON hooks; completion; channel coverage |
| `logging.Logger.warning` | Potential `log` on the `msg` formal | Configuration can suppress emission; handlers run arbitrary code |
| `builtins.open` | Candidate resource `acquire` on normal return; potential `OSError` | Release; completion |
| `atexit.register` | Identity transfer; `registered` callback action on normal exit | Invocation |
| `pydantic==2.13.5` `TypeAdapter.validate_python(object)` | Potential input → result transform | Effects, raises, completion (below) |

- **Validation schema contract (Implemented; focused Tested, 2026-09-27).** Catalog format 7
  separates the effect subject from `static_class`, `runtime_value` and `unresolved` schema
  attribution. Static classes require a unique pinned class node/fact pair from the available
  context; runtime sources have their own typed path and schema-role binding. A missing or
  ambiguous binding remains a candidate with a reason. Resource endpoints still accept only
  input/output roles. Candidate attribution establishes neither schema contents nor evaluation,
  validation success or closed coverage. The extractor only acquires classes in available,
  described context modules; unsupported class acquisition fails closed.
- **Pydantic activation remains Proposed.** A `TypeAdapter` selects its schema at runtime;
  its adapter class is not the validation schema. No validation effect is active in the current
  catalog. The focused fixture leaves this dependency model dormant; binding against the full
  pinned dependency context is `not_run` until the integrated pilot.
- **Target families** (Proposed; plan §3.3): pure identity/value (`str`, `dict`, `list`, `tuple`,
  `functools.partial`/`wraps`), I/O (`io`, `pathlib`, `zlib`), async/timeouts/context (asyncio,
  anyio, `contextvars`, `contextlib`), validation/settings (pydantic `BaseModel`/`Field`,
  pydantic-settings), HTTP/servers (httpx, starlette, uvicorn). A family without an in-scope
  consumer stays a candidate. Framework models (registries, middleware, lifespan, ContextVar
  places, function-object metadata) are Stage 5.
- **Independent checks** of pure models (CrossHair `diffbehavior`; only exhausted paths support
  equivalence; the `int` specializations of `cast` and `assert_type` are **Tested**, the generic
  models are not) belong to the validation lane ([§8.1](validation-and-evaluation.md#section-8-1)).

## Synchronous context protocols

**Implemented and focused Tested (2026-09-27; ADR-0059), partial.** Catalog format 4 separates
pinned class protocols from observed call models. The initial CPython 3.14.7 models are
`nullcontext` and `suppress`: class, allocator and initializer facts bind independently. The
source binder admits fresh direct constructions through unique stable imports and exact explicit
new/init observations. Complete initializer signatures bind arguments; a demonstrated mismatch
and an unsupported or bounded binding remain distinct. Missing provider exit calls are never
invented; inherited stub entry observations remain attributed.

Pure completion evaluates arguments in source order, registers each successful entry before `as`
assignment, and unwinds entered contexts in reverse. Suppression applies only to Raise. Supported
constructor and unpacking failures trigger partial cleanup; exit failures replace pending
outcomes. Return, break and continue survive normal exits. Exact builtin matching retains class,
resolution and complete MRO evidence when needed. Arbitrary matching, exception groups, opaque
argument evaluation, custom managers and deferred execution remain unknown. These initial models
assert normal entry; a general failing-entry model is not implemented. Manager identity and
entry-result identity remain separate. A bounded source/model certificate now proves a bare
return of a unique `as` name inside this exact `With` body when the explicit entry argument is
an immutable current parameter. Both lexical reference/resolution/binding chains and the actual
raw contribution remain cited. Raw nonidentity/call-crossing flags are unchanged; a distinct
`ContextEntryValueIdentity` supplies the modeled basis. Missing/default entry values, sibling or
completed contexts, mutation and overridden returns do not acquire that identity proof.

`return_completion_certificates` commits each normal return's ordered entry and frame-exit
obligations, even when empty. Conditions specialize explicitly; finalizer conditions retain their
scope. Publication reconstructs commitments from completion, and source/native consumers share
structural admission plus lifecycle order checks. A local base return requires exactly one value
basis independently of completion. A context basis requires successful assignment before the value
witness, its matching exit after it, and the same return-condition anchor. Missing, swapped, reordered or omitted evidence
cannot become a value summary; bounded condition decisions keep their specific reason. This is
compiler output 90/extractor output 33, with fresh stores under ADR-0048. The [independent
controls](../../design_review/evidence/2026-09-27_sync-contexts/README.md) compare actual CPython
lifecycle outcomes with the focused source/Delta/native paths; they do not qualify all Stage 3.

> Decision: ADR-0059

## Model application at source calls

Each relation below is **candidate-local**: it says a model applies at one cited call candidate,
never that the call ran, completed, raised, released a resource or invoked a callback. Target and
model modalities, candidate-set completeness and the unresolved dispatch remainder are retained on
every row; the shared validator reconstructs each relation and rejects forged status.

- **`model_applications`** joins a source `call_targets` fact to an exactly pinned `model_targets`
  row, with the Pysa target's modality, origin and phase, the model identity/revision,
  `resolutions`' candidate-set completeness, unresolved remainder and target count, and the
  target's typed body contract and exact frame-release certificate. Neither completeness nor the assertion alone means the call
  returns. Higher-order argument targets and annotation-only calls cannot pose as direct
  invocation; a shadowed builtin or a mismatched authored invocation phase has no application.
- **`model_argument_bindings`** binds a model formal to the call's explicit argument only if every
  pinned Pysa signature selects the same argument ordinal (positional or exact keyword name). For a
  direct Ruff attribute callee whose Pysa target says the receiver is an implicit **object**
  (`true_with_object_receiver`), positional ordinals shift past it: Pysa owns receiver kind, Ruff
  the callee shape, the signature the ordinals. Class receivers, unpacking, unsupported callees,
  absent arguments and overload disagreement stay `unknown` with a boundary. Classmethod and
  descriptor receivers need their own proof.
- **`modeled_transfer_sites`, `modeled_effect_sites`, `modeled_callback_sites`,
  `modeled_resource_sites`, `modeled_exception_sites`** apply one authored action to one call
  candidate. Parameter endpoints come only through `model_argument_bindings`; a `ReturnValue`
  endpoint identifies the call expression (a call-expression id is not a runtime resource
  identity). A subjectless effect is explicitly `unqualified`; field and global paths, failed
  bindings and open dispatch are explicit boundaries. Exception sites carry the pinned class node
  and fact.
- **`modeled_argument_evaluations`** accounts for every explicit argument of an exact one-call
  model candidate in source order: the selected operand cites its raw value fact and a separate
  normal-read witness; a direct literal has `literal_normal`, while the pure bounded
  `lctx_analytics::evaluation` owner reconstructs nested expressions from placed Ruff
  syntax. `expression_evaluations` separates normal completion from an exact Boolean
  value and retains an explicit refusal and work count. Supported unary, numeric `+`/`-`,
  multi-operand Boolean and conditional forms evaluate only the operands Python selects.
  Integer arithmetic is representable only within a checked i64 domain; arbitrary integers are
  normal literals but cannot silently enter floating-point arithmetic. Unsupported calls, reads,
  operators and selected raising operands supply no evaluation witness. Depth 64 and
  work 1024 have append-only `expression_depth_limit` / `expression_work_limit` boundaries.
  **Implemented, 2026-09-26:** `expression_evaluation_steps` records every selected operand,
  read, call resolution, agreeing signature binding and pinned normal-return/model witness in
  evaluation order. Normal name reads compose without inventing exact values or normal arbitrary
  truthiness. Nested sole closed total calls require an earlier unconditional module-level import
  and agreement across all pinned signatures. Optional omissions refer to definition-time defaults;
  they do not invent a default value. `composed_expression_normal` and `pinned_call_normal` extend
  the append-only evaluation codebook. The shared validator reconstructs decisions, values,
  ordered proofs and work. The shared SQL argument adapter joins
  these rows instead of independently recognizing fixed expression shapes. An exact
  unshadowed builtin name also has a local normal-evaluation witness. A direct parameter
  name is `parameter_name_normal` only when one non-approximate, non-loop-carried ty reaching
  definition matches that same lexical parameter binding and has a stored condition root. In a
  `try` body, ty may mark this reach approximate; a direct reference resolved to the current
  function's parameter is instead `lexical_parameter_normal` when that function has no explicit
  `del` or exception-handler frame. It cites the resolution fact and does not generalize to
  arbitrary local reads. The
  same exact check admits a direct local assignment read as `assignment_name_normal`; its
  definition must match the lexical assignment binding. Both cite the reaching fact, not the
  name spelling. A one-call source operand keeps its raw fact in `evidence_id` and the normal
  read in `source_normal_evidence_id`; without both it is unknown. Unpacking, possibly unbound/deleted names,
  unproved dispatch and other unsupported expressions remain unknown. The expression classifier is
  shared with the predecessor-completion relation;
  neither treats a callee's typed body contract as proof that its arguments complete.

## L2 fates: exits, handlers and finalizers

L2 establishes what happens at a frame. Lexical containment never substitutes for an execution
witness.

- **Structural exits.** `exit_sites` records explicit `return` and `raise` statements and direct
  `finally`-body actions from Ruff syntax and ty regions, with owning function, path condition and
  approximation, for every compile. It proves no escape, catch or completion.
- **Raise escape** is withheld inside any `try` or `with` body (§3.9); resolved L2 fates may later
  admit narrower escape witnesses with cited class, frame and exit evidence.
- **Handler sources.** `handler_clauses` and `handler_actions` cite each `except` clause, its type
  expression and direct body statements with their regions. `handler_types` gives one status per
  clause: `pinned_builtin` only when the lexical reference resolves uniquely to a builtin with one
  matching pinned class; bare `except` has its own status; shadowed, compound or unbound types stay
  `unknown`. The `try` entry condition is not a handler-match condition.
- **Class relationships** use Pyrefly's pinned MRO (`context_class_mro`: ordered ancestors, or an
  explicit empty/cyclic marker) instead of a custom exception hierarchy. A handler is a
  `pinned_ancestor` candidate only when one MRO row names its pinned class. A missing, unbound or
  cyclic relationship is `class_relation_unknown`, and MRO non-membership is **not** a nonmatch,
  because a model class can denote a family of subclasses. A negative match needs a separate exact
  raised-class contract. The MRO is a static model, not an observation of runtime `__bases__`.
- **Handler candidates.** `modeled_exception_handler_candidates` connects a modeled raise in a
  `try` body to each clause of each enclosing frame through a bounded recursive syntax-ancestor
  walk (128 edges, stopping at the innermost function). `modeled_exception_handler_walks` records
  one coverage row per modeled raise, with explicit reasons for missing syntax or the cap; absence
  from the candidates is interpretable only when the walk is complete. `frame_possible` excludes a
  later clause after a proven earlier match; `frame_first_match_if_raised` needs a positive match
  and no possible earlier one. Both are conditional on the raise reaching the frame.
- **Bounded handler return.** `modeled_exception_return_none_paths` composes a modeled potential
  raise with the first provable matching clause of a **direct function-body** `try` and that
  handler's sole direct `return None` (`handler_return_none_sites`, whose ty region is
  approximate). A complete ancestry walk must show no inner `try`/`with` and the frame no
  `finally`. It is a candidate path conditional on the raise, not a catch or normal-return verdict;
  nested frames, uncertain precedence, computed actions and finalizers stay unknown.
- **Return frames and finalizers.** `return_exit_statuses` walks each return's same-function
  ancestry to a declared cap and cites the nearest controlling `with` or pending `finally`.
  **Implemented, 2026-09-26:** pure `lctx_analytics::completion` reconstructs
  `statement_completions`, `statement_completion_steps` and `return_exit_steps`. Normal finalizers
  preserve a pending outcome; an abrupt or unknown finalizer withholds that original return.
  Supported actions include pass, normal expressions, unique local initialization outside loops,
  exact selected branches and nested try/finally. An opaque exception operand cannot establish
  a raise: exception classes can execute user constructors. Exact invalid primitive exception
  operands establish exact `TypeError`. Ordered bare or pinned typed handlers can catch that
  exception; bare re-raise propagates a known active exception through nested handlers/finalizers.
  Handler matching cites pinned class identity or MRO ancestry. Nonmatch additionally needs the
  native Pyrefly `linearization_complete()` flag retained in `context_class_mro`: the Pysa report
  alone drops that flag and omits self/object. Extraction retains the modeled runtime class and
  acquires public metadata at the same pin; there is no fork or dependency change. Handler facts
  are prepared before completion. Named-handler cleanup, exception groups (`except*` is marked
  in syntax), opaque constructors, arbitrary rebinding/finalization, loops and unmodeled context exits
  remain unknown. Pinned synchronous protocols have the separate bounded owner above. Handler/else/finalizer entry now reconstructs the pending outcome through
  the same try-body owner. A shared `CompletionOutcome` enum prevents invalid combinations;
  `statement_completions.exception` retains an exact raised kind independently of potential effects.
  Full operand/call/binding evidence is retained in inner-to-outer frame order. Completion depth
  128 and work 4096 have separate typed refusals. The shared summary-proof cap is 64 local steps;
  the producer emits `summary_proof_limit` before native admission could reject a generation.
  Pending-frame completion is separate from the path-specific entry certificates below.
  [Independent CPython/Hypothesis controls](../../design_review/evidence/2026-09-26_expression-completion/README.md)
  challenge normal/overriding/exceptional finalizers and implicit execution (**Tested**, 2026-09-26).
  [Typed-handler controls](../../design_review/evidence/2026-09-26_typed-completion/README.md)
  add 100 independent CPython/Hypothesis observations. The
  [bounded handler review](../../design_review/reviews/design_review_stage3-typed-handler-completion_2026-09-26.md)
  records the public-API correction and limits; eleven focused pure/source/Delta/native tests pass.
  **Implemented and focused Tested (2026-09-27):** entering a named handler refuses with
  `handler_name_cleanup`, retaining the unproved name binding and implicit deletion boundary
  through statement/return completion and native summary inspection. A skipped handler does
  not introduce that boundary. This is a specific refusal, not a cleanup proof.
- **Target** (Proposed; plan order 2): nested `try`/`finally` and `with` frame order, normal and
  exceptional completion, suppression and handler propagation; callbacks stored, invoked,
  forwarded or registered, and resource acquire/release, each only with an execution and exit
  witness. Generators and coroutines stay at the Stage 5 deferred-execution boundary.

## Transfer summaries

**Accepted consolidation target (Proposed implementation, 2026-09-26).** Ordered argument
bindings and evaluation/completion certificates replace source-shape-specific admission.
Normal evaluation, exact primitive value, transfer, effect and channel completeness are separate
concepts. The pure producer consumes them with handler/frame inputs; core owns acquisition and
publication order. Simultaneous condition substitution uses checked stable-value links and
bounded Boolean composition with cumulative work/retention limits. Semantic fixed-point keys
are separate from bounded proof alternatives, which preserve origins and parallel call sites.
Shared proof admission serves publication and native decoding. Existing narrow implementations
below remain the current behavior until each migration is tested; the active plan §3.0 owns
the dependency order and deletions.

> Decision: ADR-0057

**The contract** (accepted target; finite parts implemented as stated below).
- **Tables:** `summary_flows` (callable, input path, output path, kind `value`/`transform`/
  `constant`, condition, verdict), `summary_effects` (callable, effect, role bindings, condition),
  `summary_boundaries` (callable, reason, site). Summaries reference lossless condition roots,
  never display DNF.
- **Paths:** `Parameter[name]`, `Parameter[self].Field[f]`, `ReturnValue`,
  `Argument[formal]@Call[target]`, `Global[<module>.<name>]`, `Raise[T]`: CodeQL's models-as-data
  shape without its file format, and §3.9's resolved place key in written form.
- **Composition:** the call graph's SCCs, callees first, each iterated to a fixed point over a
  finite domain (path depth, condition nodes, pair work, iterations). Exhaustion writes a specific
  `summary_boundaries` cause and makes dependent verdicts `unknown`; override-open calls join their
  candidates and stay open ([§3.6](facts-and-identity.md#section-3-6)); exceptions convert through
  handlers; value/transform/effect/exception/role paths compose by BDD conjunction with declared
  modality.
- **Discharge:** a callee's summary resolves a `call_transfer` claim to `established` or
  `conditional` only with the matching proof; `refuted_under_model` needs a complete summary with
  every candidate call, handler and modeled channel closed.

**Positive paths** (implemented producers). A finite `summary_flows` path is admitted only when:
- **Direct base:** a synchronous function body returns its own parameter by raw identity, with no
  crossed call or generator yield, citing the value fact, return syntax/region and recomposed
  condition. A compatible preceding call can be crossed only when one closed, definite pinned
  target asserts normal return, the callee has one earlier module-level `from` import whose ty
  region is unconditionally reached, every argument is a direct literal, a `+`/`-` numeric
  literal with one direct numeric operand, an exact unshadowed builtin name, or a uniquely reaching
  parameter/assignment name with ordered evidence, and the ty call region is non-approximate. A
  signed literal cites the outer unary syntax fact; a unary expression over a binary operation
  remains unresolved. The proof cites the import binding and region, callee resolution,
  each argument, call site, Pysa target and
  model id before the raw return step.
  This `preceding_call_normal` step is a safety witness if the call runs, not an assertion that it
  runs. An opaque or possibly raising sibling, conditional/local import, unresolved target,
  unpacking, approximate region or missing condition withholds the direct summary; a
  BDD-proved disjoint call needs no completion witness. The bounded kernel gives `established`,
  `conditional` or a named `unknown`;
  a false condition yields nothing. **Tested in focused pure and Delta/native cases
  (2026-09-26).** A recursive call on the non-terminating branch does not erase a cited direct
  return on the terminating branch; an unconditional self-call before return still withholds the
  direct flow. Nested calls as arguments of an earlier call and non-import callees are not yet admitted.
- **Independent source parameter identity (Implemented, 2026-09-27):** an approximated reaching
  contribution can use an occurrence-specific `source_parameter_identities` certificate for a
  bare return read. The pure producer resolves the exact syntax span through reference,
  resolution, scope and the sole immutable parameter binding; deletion, nonlocal mutation,
  capture, decoration and yield withhold it. Source publication reconstructs the certificate.
  Raw provider flags remain unchanged. Only the matching function/formal/source/origin/condition/
  return tuple can discharge this value approximation, and entry and exit completion remain
  mandatory. A sibling origin or crossed call stays open. The summary cites append-only proof
  step 26; FORMAT 9 carries the certificate, and native admission rejects foreign, missing,
  duplicated or uncited certificates. This certifies value identity, not general frame completion.
- **Modeled call:** an exact whole-expression call of `typing.cast` or `typing.assert_type` whose
  sole source target is closed, both modalities definite, the target has its typed body and exact frame-release proof, the
  callee is one resolved simple name, and every explicit argument has ordered normal-evaluation
  evidence. The call span must equal the whole value sink span (an outer operator or fallback
  cannot borrow the call). The candidate condition must be satisfiable and imply the direct return
  region. An earlier same-function call before that return also needs the ordered
  normal-completion witness used by direct parameter returns; an opaque compatible earlier call
  withholds the modeled positive.
  **Implemented and focused Tested, 2026-09-27:** a direct single-model return in an approximate
  provider region can instead cite `source_modeled_identities`. The shared immutable lexical
  read owner proves the actual selected argument denotes its parameter; the existing source
  adapter binds that raw occurrence to the exact call/model/rule and return. The certificate
  commits the existing ordered model-call proof, replacing its raw value basis while preserving
  provider flags. Completion remains independent, and coverage remains open. Schema/native
  admission follows declared path depth; it rejects even deletion of the entire modeled group
  and certificate while admitting a context-value return with an unrelated modeled predecessor.
  Chains and assignment-mediated approximated paths need their own complete source proof.
  [Independent controls](../../design_review/evidence/2026-09-27_modeled-identity/README.md)
  challenge object identity through pass finalizers, rebinding, deletion and override.
- **Nested modeled identity chain:** an ordered raw `flow_value_calls` path through two or more
  source arguments may compose when every step uniquely binds to a Ruff call and explicit
  argument, a sole closed definite pinned identity target asserts normal return, and each sibling
  argument has a normal-evaluation witness. `modeled_chain_arguments` gives the pure producer one
  row per step and argument; it checks dense steps, exact spans, source and return identity, and
  inserts the inner call proof at its outer source-argument position. The limit is eight call
  steps and 128 argument rows per source contribution; exceeding it records a boundary. A missing
  or unresolved step never supplies a normal result. The innermost source must be a direct formal
  read with either that exact ty reaching witness or the narrow lexical parameter witness; its
  proof cites both the raw flow fact and the read evidence. Focused real-provider and native checks admit two- and three-call `typing.cast`
  chains and withhold a raising inner sibling (2026-09-26). A deleted formal has no
  parameter-origin contribution and supplies no positive path.
  This covers exact total identity chains, not arbitrary nested expressions or transforms.
- **Assignment then return:** the same model-call proof on a whole assignment value, then a
  returned use with exactly one reaching definition whose bounded compatibility is proved and
  whose condition implies the reaching, successor and return-region conditions. Every earlier
  compatible call before the returned use, including the assignment's source call, needs an
  independent ordered normal-completion witness.
- **Local wrapper:** a synchronous caller inherits a value summary of its sole definite local
  target when ty's one-call value path, lexical callee resolution, closed target set, required
  formal coverage and direct return exit agree. `summary_contract::LocalCallArgument` represents
  every explicit argument independently of source position; the producer requires dense source
  order, unique mappings and a separate normal witness for every argument, including the tracked
  source. The raw flow is cited as `return_source`, followed by its normal-read evaluation.
  Missing required arguments, unpacking and missing witnesses remain unknown. The current cap
  is 128 explicit arguments and depth eight over the callee-first SCC schedule.
  Each effective conditional callee atom needs a checked direct entry-value link. Exact Boolean
  values or directly read caller formals fixed by a linked caller condition supply simultaneous
  BDD replacements; the restricted callee must become true. `summary_contract` admits the entire
  ordered control-link group in both producer and native reader. The caller's condition remains
  authoritative; residual symbolic conditions are not admitted. The pure contract handles
  multiple controls, but extraction withholds a later test link after arbitrary earlier
  truthiness: source multi-control calls remain unknown until call-specific stability is proved.
  Defaults used as control values and general cross-scope conjunction remain unimplemented.
  Compatible earlier calls still require independent normal-completion witnesses.

Supporting candidate relations: `modeled_exact_value_transfers` (a raw return or definition value
joined to one exact model step), `value_flow_predecessor_candidates` (a successor use joined to a
cited reaching definition and earlier raw fact) and `value_flow_predecessor_compatibility`
(tri-state: false refutes this candidate under the declared atoms, true admits a may-path, and
loop-carried, missing or capped roots are unknown, a cap staying `budget_reached`). None of them
chooses a reaching definition or proves completion. `preceding_normal_call_arguments` is the
query-only, all-arguments witness for the narrow direct or modeled-return case; it produces no row if one
argument or the callee's earlier module import is unproved, and the producer checks that import's
condition is `true` before admitting. Otherwise it records `unsupported_control_flow` rather
than borrowing a target's normal-return claim.

**Preparation and transport (Implemented, 2026-09-26).** Core prepares handler clauses, types,
actions, walks and return-None candidates before summary composition. Twenty SQL-only
behavior derivations retain Arrow batches through strict declared-schema conversion and
canonical sort into Delta. Typed rows remain at pure semantic transformation boundaries, and
publication still reconstructs semantics; the transport change is not a new validation policy
or a measured memory claim.

**Coverage.** The finite producer takes raw parameter-origin return contributions as typed
`summary_boundary_candidates` and returns admitted flows, ordered steps, typed refusals and
`summary_boundaries` together, without acquiring a session. `cpg-core` acquires and publishes;
the shared validator reconstructs the same outcome once. A crossed call without a narrower
cause stays `call_transfer`, an unsupported control path stays `unsupported_control_flow`, and
a depth or BDD cap carries its own append-only boundary reason. A stored bounded return
condition is classified before predecessor checks, and a bounded predecessor conjunction keeps
its kernel refusal instead of falling through to a generic control reason. Each unaggregated
`value_flow_contributions` row has a stable `origin_id` derived from its semantic key and checked
against the `lctx_id` UDF. A proof closes only its matching origin and condition; a sibling
contribution or a specifically refused path stays open with its own reason. Missing evidence
alone does not replace the call-transfer fallback. These unknowns are coverage, not refutations.

**Proof identity.** `summary_flows` keys on a canonical `summary_id` hashing callable, formal,
source origin, input/output paths, transfer kind, condition, return site/region and the ordered typed proof
`(step kind, evidence id, step condition)`. `summary_flow_steps` stores the steps (codebook
`summary_flow_step_kind`, append-only: raw identity, callee resolution, argument evaluation, call
target, model rule, return exit, call site, reaching definition, return source, callee summary,
finalizer pass). Equal endpoints with different evidence or order get different ids, so parallel
paths stay distinct and explain themselves through source facts; a callee summary is cited by id.
A new step kind needs its checked source relation, deterministic encoding, publisher, shared
reconstruction and native admission before it creates positives. If a rule ever needs a
multi-parent proof, that is a new decision, not parents smuggled into text.

**Recursive value engine** (Tested in focused pure and source cases, 2026-09-26). The first value relation
uses an SCC-local, one-million-pair bounded worklist over the petgraph component schedule.
Each admitted source/callee pair cites the exact callee summary and its own origin, condition,
ordered predecessor and call proof. New semantic states and strictly shorter representatives feed
only dependent local edges; a fresh proof ID alone does not restart recursion. Semantic identity
retains source origin, return site/region, condition, paths, kind, verdict and approximation.
Alternate local witnesses survive; omitted expansion propagates along cited callee dependencies.
A shorter representative replaces only its semantic alternative's prior refusal; other blocked
alternatives remain explicit. Depth eight and pair-work exhaustion still leave typed
origin-specific unknown boundaries;
the latter uses append-only `summary_pair_work_limit` rather than the general budget reason. A
base-free cycle remains unknown. Modeled bases in a recursive member are admitted only after
their independent source, argument, predecessor and exit checks. The exact literal-controlled
transfer cites the literal syntax and direct `flow_test_value_links` row (`callee_condition_link`,
append-only code 14). The same route consumes separately derived exact Boolean values from
bounded nested closed expressions, preserving their outer syntax witness before the callee
links. Normal numeric/string values are never relabelled Boolean. Default expression depth/work
refusals propagate through local-call origin boundaries; they never establish a false guard.
Direct formal forwarding additionally cites the caller's fixed test link
(`caller_condition_link`, append-only code 15) before the callee link; native admission checks the
direct source link and the caller's fixed guard, and the shared validator reconstructs the full
producer. Real terminating literal and guarded symbolic mutual-recursion fixtures publish
conditional depth-one paths; their opposing guards and an unaltered conditional self-call remain
unknown. General argument substitution and bounded cross-scope BDD conjunction remain targets. Ascent and datafrog
agreed on a narrower finite-base reachability probe, not on these product semantics; recompare
if several channels share recursive rules
([ADR-0053](../../adr/0053-bounded-scc-summary-worklist.md)).

**Predecessors (Implemented and focused Tested, 2026-09-26).** Pure completion owns
`return_entry_statuses` and `return_entry_steps`, keyed by return site and candidate condition.
Core requests only conditions from that return's source contributions and its assignment
predecessors. The walker evaluates outer-to-inner enclosing suites and preceding statements in
source order; non-call expressions, first local initialization and selected branches must
complete normally. Function docstrings are definition-time metadata. Loops and unsupported
context entry and named-handler cleanup remain explicit unknowns. A second executed assignment to the same local
is refused because releasing the old value can execute user finalization. Mutually exclusive
syntactic assignments are admissible only on a fully walked path with no prior initialization.

A same-site predicate outcome can select a conditional path after independently normal operand
evaluation. A single comparison supports this conditional use; chains remain unknown. Tuple
operands preserve element order and refuse unpacking/unsupported elements. These assumptions
neither make an arbitrary comparison/truthiness expression unconditionally normal nor establish
primitive identity or cross-site stability. Missing predicates and existing or newly reached BDD
limits retain their typed causes. Standalone statements and pending finalizers have no such
assumptions. Finite summaries consume scoped dense entry proofs with condition implication and
the shared proof cap; core reconstructs exactly the same rows for publication. The old call-only
predecessor queries and classifier are deleted. See the
[bounded review](../../design_review/reviews/design_review_stage3-predecessor-completion_2026-09-26.md).

**Origin coverage (Implemented and focused Tested, 2026-09-26).**
`summary_origin_coverage` separates completeness, refusal, retained witness count and omitted
witnesses per typed subject, condition, channel and phase. Value/Call origins retain actual
parameter and flow evidence. Execution-site completion coverage uses its syntax identity without
a fabricated formal. `summary_contract::coverage_domain` and the checked subject/channel pairs
define site Exception as **escaping exceptions under entry to the statement**: a handled raise
can leave an empty escaping domain while its nested raise site has an exception witness. This
never closes generic raise/catch/convert/suppress activity, nor proves operation entry. Async and
generator scopes are excluded from Call-phase site coverage. Complete zero-witness certificates
are reconstructed from actual completion outcomes, independently of witness omission.
A positive finite call path does not close its callee's alternatives: every call crossing
remains open until S4/S5 compose coverage. Raw or retained-summary approximation also leaves it
open. Complete direct-origin coverage is neither operation-wide completeness nor a negative
claim about another channel. Core publishes and reconstructs the pure producer's full rows.
Extraction-only runs without the behavioral condition catalog emit no such coverage: the absent
entry premise is unexamined, rather than an invented condition reference.
See the [bounded review](../../design_review/reviews/design_review_stage3-origin-coverage_2026-09-26.md).

**Default binding (Implemented and focused Tested, 2026-09-27).** The shared source binder
separates explicit source-ordered arguments from omitted definition-time defaults. Syntax alone
cannot prove availability: even unused omitted formals need a certificate. The initial domain is
an undecorated, uniquely bound, synchronous nested definition immediately followed by its sole
direct call in a return. The header completes normally; no alias/escape, intervening action,
assignment expression or nested argument call is admitted. Defaults are evaluated at definition,
never at invocation; availability, exact Boolean value and stability have distinct cited proof
kinds. Missing defaults and unproved stability keep separate typed boundaries. Pinned total models
own their signature-default promise separately. The pure source binder replaces the SQL
argument-count classifier, and private/nested entry-formal links share the same reaching/no-effect
admission as public ones. Broadening expression protocols requires rechecking the stability rule.
The [channel review](../../design_review/reviews/design_review_stage3-channel-contracts_2026-09-26.md)
bounds acceptance; broader default domains and all-channel coverage remain open.

**Shared callee proof admission (Implemented, 2026-09-26).** Schema-owned structural admission
checks contiguous scoped controls, the cited condition, decreasing callee depth and the common
proof cap. Source composition and native loading use it; native's independent adjacency policy
is deleted. Typed admission failures preserve `summary_proof_limit` rather than replacing it with
missing evidence. This is the current value proof subset; other channel obligations remain S1/S4.

**Schedule and recursion.** `summary_components` records every release function's SCC, sorted
members, canonical component id and a callee-first schedule over attributed local call targets;
petgraph 0.8.3's iterative `kosaraju_scc` computes components and a sorted condensation
worklist makes ties independent of row order. A 30,000-edge chain and the existing
mutual/self-recursion order control passed in focused tests (2026-09-26). A self-call marks a
singleton recursive. Candidate/open dispatch contributes topology only.

> Decision: ADR-0052

**Known gaps** ([plan W5, W7, W12, W13](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- W5 (ARC-02/03) has focused production and native evidence for explicit finite inputs and
  specific depth/control/atom-limit refusals. Actual predecessor conjunctions also preserve
  work- and node-limit reasons through the pure producer and boundary decision in focused tests
  (2026-09-26). ADR-0054 carries stable contribution IDs through summary and native boundary
  contracts. A real `mixed_origin` return now retains one admitted and one `call_transfer`
  sibling on the same raw fact through Delta and native in one focused generation. Work/node-limit
  publication controls remain open.
- W7: the flow model's cyclic reach uses the bounded source fixed point (§3.9); its production-cap
  native trace and fresh pilot cost remain open. A real extracted-flow row shuffle now preserves
  canonical published contribution, value-flow and premise bytes in a focused fixture.
- W12: ADR-0053's SCC-local bounded value worklist passes pure finite-base, base-free,
  self/mutual cycle, parallel-origin, shuffle and cap controls. Real exact literal and guarded
  direct-formal controls pass source, shared-publication and native proof checks; opposing
  literal/formal guards withhold the recursive path. Broader bounded cross-invocation
  substitution, other channels, native cap trace and pilot cost remain open. Ascent/datafrog
  remain candidates when several recursive
  relation families share rules. A wall-clock timeout is not a deterministic budget.
- W13: iterative SCC routine and schedule ownership are decided by ADR-0052 and tested in a
  deep chain. The separate type-term recursive set closures now terminate on cyclic controls;
  pilot cost and the integrated digest remain open. Keyed condensation ordering stays
  repository-owned.

## Native serving boundary

A FORMAT 9 generation carries the structural condition catalogs, summaries, proof steps and
boundaries. The native executor (`python/lctx_semantics`, one immutable PyO3 executor per
process) validates internal `callable_parameters` for private/nested control evidence while
query lookup stays restricted to public operations. This early S6 subset is **Implemented and
focused Tested (2026-09-27)**; expanded channel/query support remains open. It checks the schema-owned Arrow IPC projection and codebook before admission; Python
validates requests and shapes results. `inspect_value_paths` pages one formal's finite paths
and open boundaries. Each positive path exposes its cited summary, source-flow fact and
source-origin IDs; an open boundary exposes its separate source fact and origin. Each exact-input
result is **path-local inspection, never an operation-wide verdict**. **Implemented; Tested in
focused cases (2026-09-26)** through a real same-fact sibling-origin Delta/native generation.
The former duplicate native proof-kind whitelist and positional Python production decoder are
closed under [plan W1](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).
**Implemented and focused Tested (2026-09-27):** fresh source-call support carries its base body,
ordered release inputs and header certificate. Native admission validates the same separate
callee-frame and caller-retention obligations, including foreign-caller substitution and missing
support groups. This structural closure does not establish the still-open full raw-source closure.
Serve-time semantic selection remains a proposed ADR-0025 target
([§11.3](synthesis-and-serving.md#section-11-3)).
- **Target** (Proposed; plan order 9): typed operation-wide compatibility, effect and role
  filters. A request names a real public operation and formal and a typed primitive predicate; the
  operation/formal and exact primitive origin are resolved before any BDD evaluation; missing
  proof is `unknown` even if raw atoms happen to be compatible; row, node, pair-work and depth
  bounds are checked before allocation and return explicit `unknown`/`truncated` with work
  accounting.

## The capability registry

**Proposed** (Stage 4). It lives in `cpg-schema` as TOML (`deny_unknown_fields`) compiled to Arrow.
- **A concept** has an append-only id, a `prefLabel`, `altLabels` (each with its source),
  `broader`/`related`, a scope note and facets, and a **definition**: a conjunctive query with
  shared variables over the `behavior` family and the summaries, written as a Rust enum AST,
  compiled to DataFusion SQL and digested. Definitions over conditions use the kernel's
  compatibility and implication.
- **`concept_members`** is materialized: concept, operation, role bindings, condition, verdict,
  witness. Every member cites a definition digest and a witness.
- **Integrity:** `broader` is acyclic (a recursive CTE); `related` is disjoint from the `broader`
  closure.
- **Discovery nominates, definitions decide.** FCA over behavioral attributes suggests facets and
  vectors rank per view; communities have no tool consumer
  ([§9.8](analytics.md#section-9-8)). None of them writes `concept_members`.
- **`lookup_concepts`** returns every concept with its labels and scope notes while the catalog is
  small (20–40 authored); ranked lookup waits until it outgrows one page. **`explain`** returns the
  stored witness chain.

> Decision: ADR-0045, ADR-0024, ADR-0028, ADR-0050, ADR-0053, ADR-0054, ADR-0055, ADR-0056

---

## Channel composition contracts

**Proposed implementation, 2026-09-26.** Stage 3 uses typed channel subjects and origins instead
of forcing all channels through parameter-return identifiers. Coverage concerns a subject,
condition, channel and phase, with separate semantic completeness, refusal and witness omission.
A complete empty channel cites its coverage proof; a positive path alone never closes alternatives.
Shared structural proof admission owns evaluation, binding, completion, model and callee links.
Model rules/coverage declare phase applicability; dynamic validation schema identity is distinct
from a named static schema. Effects before a raise use reached-prefix evidence. Analytics remains
pure; core acquires and publishes, and native serving admits the same proofs.

**Implemented and focused Tested, 2026-09-27:** `call_executions` and its ordered steps prove
reached eager call inputs separately from normal expression completion. Schema owns shared
structural admission; the pure evaluation/binding and completion owners reconstruct invocation
and statement-prefix groups; core persists and validates exact source equality. A sole pinned
bare imported target, fixed binding, ordered explicit arguments and available defaults are
required. The invocation ends at its own model/target/site tuple, not a return or modeled
action. Refused oversized calls retain their actual argument count; positive proofs keep the
128-argument and 64-step bounds. Nonconstant guards, context frontiers and broader expression
placement remain unknown. Native export and action consumers remain S3/S6 work. An isolated
13-program CPython CALL-event challenge distinguishes reached calls from callee return
([evidence](../../design_review/evidence/2026-09-27_call-entry/README.md)).

The [active execution queue](../../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns implementation. Transfer projections preserve kind/condition alternatives and the current
RCA projection retains typed modality and evidence. No Stage 5 lifecycle execution is pulled forward.

> Decision: ADR-0058
