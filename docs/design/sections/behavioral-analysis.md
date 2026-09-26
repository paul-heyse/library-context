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
- **Total normal completion** is a separate, optional assertion (`normal_return`, default false =
  unknown): the target itself returns normally once its arguments are evaluated, under the pinned
  model. It is accepted only for a function target with complete exception coverage and no
  exception rule, carried on the validated `model_targets` row, and never inferred from a transfer
  rule (which describes the value *if* the callee returns) or from the absence of modeled
  exceptions (which does not exclude divergence). Asserting it needs pinned source or
  documentation evidence.

**The current catalog** (`external.toml`; pinned CPython 3.14.7 unless noted):

| Target | Asserted | Left open |
|---|---|---|
| `typing.cast`, `typing.assert_type` | Identity transfer; `normal_return` (the pinned bodies directly `return val`) | — |
| `builtins.print` | Potential `io.write`, no subject stream | Which stream |
| `json.dumps`, `json.dump` | Potential `obj` → return transform (`dumps`); potential JSON serialization of `obj`; potential `io.write` on the bound `fp` (`dump`) | Custom encoder and `default` callbacks; completion |
| `json.loads`, `gzip.compress`, `gzip.decompress` | Potential input → return transforms; `compress(gzip)` on `data` for `gzip.compress` | Malformed input, resource exhaustion, JSON hooks; completion; channel coverage |
| `logging.Logger.warning` | Potential `log` on the `msg` formal | Configuration can suppress emission; handlers run arbitrary code |
| `builtins.open` | Candidate resource `acquire` on normal return; potential `OSError` | Release; completion |
| `atexit.register` | Identity transfer; `registered` callback action on normal exit | Invocation |
| `pydantic==2.13.5` `TypeAdapter.validate_python(object)` | Potential input → result transform | Effects, raises, completion (below) |

- **Pydantic dynamic schema.** A `TypeAdapter`'s schema is selected at runtime by the instance, so
  the named-schema `validate(schema)` effect cannot name it without a false exact-schema claim.
  Do not populate it with the adapter class name; a typed dynamic-schema case, with a source
  witness where one can be proved, precedes any validation-effect or negative-coverage claim. The
  focused fixture leaves this dependency model dormant; binding against the full pinned dependency
  context is `not_run` until the integrated pilot.
- **Target families** (Proposed; plan §3.3): pure identity/value (`str`, `dict`, `list`, `tuple`,
  `functools.partial`/`wraps`), I/O (`io`, `pathlib`, `zlib`), async/timeouts/context (asyncio,
  anyio, `contextvars`, `contextlib`), validation/settings (pydantic `BaseModel`/`Field`,
  pydantic-settings), HTTP/servers (httpx, starlette, uvicorn). A family without an in-scope
  consumer stays a candidate. Framework models (registries, middleware, lifespan, ContextVar
  places, function-object metadata) are Stage 5.
- **Independent checks** of pure models (CrossHair `diffbehavior`; only exhausted paths support
  equivalence; the `int` specializations of `cast` and `assert_type` are **Tested**, the generic
  models are not) belong to the validation lane ([§8.1](validation-and-evaluation.md#section-8-1)).

## Model application at source calls

Each relation below is **candidate-local**: it says a model applies at one cited call candidate,
never that the call ran, completed, raised, released a resource or invoked a callback. Target and
model modalities, candidate-set completeness and the unresolved dispatch remainder are retained on
every row; the shared validator reconstructs each relation and rejects forged status.

- **`model_applications`** joins a source `call_targets` fact to an exactly pinned `model_targets`
  row, with the Pysa target's modality, origin and phase, the model identity/revision,
  `resolutions`' candidate-set completeness, unresolved remainder and target count, and the
  target's `normal_return` assertion. Neither completeness nor the assertion alone means the call
  returns. Higher-order argument targets and annotation-only calls cannot pose as direct
  invocation; a shadowed builtin has no application.
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
  normal-read witness; a direct literal has `literal_normal`, while bounded unary
  `+`/`-` on a numeric literal, `not` on a Boolean literal, `+`/`-` on two direct numeric
  literals and a decisive two-operand `False and ...` or `True or ...` have
  `closed_expression_normal` (ADR-0056). The latter skips the right operand entirely; its
  opposite `True and ...` or `False or ...` requires a separate right-operand proof. These
  witnesses cite the whole Ruff expression, not a computed value. Division, other evaluated
  operands, chained Boolean operations and other operators remain unresolved. An exact
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
  read in `source_normal_evidence_id`; without both it is unknown. Unpacking, possibly unbound/deleted names, nested calls outside the exact
  modeled-chain route and other unproved expressions retain `outside_provider_model`. The closed-expression/builtin/local-name classifier is
  shared with the predecessor-completion relation;
  neither treats a callee's `normal_return` assertion as proof that its arguments complete.

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
  ancestry to a declared cap and cites the nearest controlling `with` or pending `finally`. A
  return is admitted through pending frames only when **every** pending frame is a `try` whose
  entire nonempty direct `finalbody` consists of literal `pass` statements; the pass facts are reconstructed in
  inner-to-outer execution order and each admitted summary cites all of them as ordered
  `finalizer_pass` proof steps (the single-pass status fields are populated only for one
  controlling frame with one pass). This is a
  local normal-exit proof, not proof that the return expression or an earlier call completes.
  A nontrivial finalizer, `with` and capped ancestry keep their control boundary: expanding them
  needs an ordered exit witness per frame, not a wider allowlist. A targeted CPython 3.14.7
  `sys.monitoring` check agrees for a pending return through `finally: pass` and an overriding
  `finally` return (**Tested**, 2026-09-25).
- **Target** (Proposed; plan order 2): nested `try`/`finally` and `with` frame order, normal and
  exceptional completion, suppression and handler propagation; callbacks stored, invoked,
  forwarded or registered, and resource acquire/release, each only with an execution and exit
  witness. Generators and coroutines stay at the Stage 5 deferred-execution boundary.

## Transfer summaries

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
- **Modeled call:** an exact whole-expression call of `typing.cast` or `typing.assert_type` whose
  sole source target is closed, both modalities definite, the target asserts `normal_return`, the
  callee is one resolved simple name, and every explicit argument has ordered normal-evaluation
  evidence. The call span must equal the whole value sink span (an outer operator or fallback
  cannot borrow the call). The candidate condition must be satisfiable and imply the direct return
  region. An earlier same-function call before that return also needs the ordered
  normal-completion witness used by direct parameter returns; an opaque compatible earlier call
  withholds the modeled positive.
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
- **Local wrapper:** a synchronous caller inherits an unconditional value summary of
  its sole definite local target when ty's one-call value path, Ruff's exact single explicit
  positional-or-keyword argument syntax, lexical callee resolution, Pass B's single formal mapping, a closed target set and a
  direct return exit agree, along a callee-first SCC schedule capped at depth 8. Within a
  recursive component a deterministic worklist reuses each newly cited value path. One narrow
  two-argument case can also specialize a **conditional** callee path: the first positional
  argument is the tracked parameter; the second maps definitely to a distinct tested formal.
  It is either an exact boolean literal or a directly read caller formal whose lexical
  parameter binding agrees with Pass B, with a cited ty reaching definition and direct
  entry-value test link. In the latter case, the caller's
  existing BDD path must imply that formal's tested atom or its negation. Bounded restriction
  of the callee's linked truthy atom by the proved value must make its condition true; false
  or residual conditions stay unknown. The caller's own condition remains authoritative.
  General argument expressions and cross-scope BDD conjunction are unproved. Compatible earlier calls in
  the caller must independently complete normally; the returned wrapper call itself is proved
  by its cited callee summary.

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
ordered predecessor and call proof. Newly derived paths feed only dependent local edges;
depth eight and pair-work exhaustion leave typed origin-specific unknown boundaries;
the latter uses append-only `summary_pair_work_limit` rather than the general budget reason. A
base-free cycle remains unknown. Modeled bases in a recursive member are admitted only after
their independent source, argument, predecessor and exit checks. The exact literal-controlled
transfer cites the literal syntax and direct `flow_test_value_links` row (`callee_condition_link`,
append-only code 14). Direct formal forwarding additionally cites the caller's fixed test link
(`caller_condition_link`, append-only code 15) before the callee link; native admission checks the
direct source link and the caller's fixed guard, and the shared validator reconstructs the full
producer. Real terminating literal and guarded symbolic mutual-recursion fixtures publish
conditional depth-one paths; their opposing guards and an unaltered conditional self-call remain
unknown. General argument substitution and bounded cross-scope BDD conjunction remain targets. Ascent and datafrog
agreed on a narrower finite-base reachability probe, not on these product semantics; recompare
if several channels share recursive rules
([ADR-0053](../../adr/0053-bounded-scc-summary-worklist.md)).

**Predecessors.** A true return region does not prove that an earlier call returned. The direct
producer ignores an earlier same-function call when its narrowest non-approximate ty statement
region and the return-value condition have a bounded conjunction of **false**. A compatible call
requires the ordered pinned normal-completion proof above; a bounded or missing condition
withholds the direct path with its specific reason. The pure summary producer sorts the
source-cited predecessor calls before its stop-at-return scan, so an input-row shuffle cannot
hide an earlier unproved call; it also orders finalizer-pass candidates before proof hashing.
This path-incompatibility screen is interim;
the target (plan order 1)
distinguishes, for each argument and preceding statement on a proposed path, an evaluated direct
value, a cited normal outcome, a possible raise and an unresolved expression, preserving
evaluation order (a modeled target's normal return starts after its arguments). Recursive SCC
members can supply a modeled base only after the same proof checks as acyclic members; missing,
approximate or capped evidence stays an explicit unknown. No generic `return x` positive after
an unproved call.

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

A FORMAT 8 generation carries the structural condition catalogs, summaries, proof steps and
boundaries. The native executor (`python/lctx_semantics`, one immutable PyO3 executor per
process) checks the schema-owned Arrow IPC projection and codebook before admission; Python
validates requests and shapes results. `inspect_value_paths` pages one formal's finite paths
and open boundaries. Each positive path exposes its cited summary, source-flow fact and
source-origin IDs; an open boundary exposes its separate source fact and origin. Each exact-input
result is **path-local inspection, never an operation-wide verdict**. **Implemented; Tested in
focused cases (2026-09-26)** through a real same-fact sibling-origin Delta/native generation.
The former duplicate native proof-kind whitelist and positional Python production decoder are
closed under [plan W1](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).
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
