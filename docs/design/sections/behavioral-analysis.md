<a id="section-9-9"></a>

# §9.9 Summaries, models and the capability registry

**Proposed** (ADR-0022, 2026-09-24; plan Stages 3–4).

**Models** are committed data (`crates/cpg-schema/models/{external,frameworks}.toml`):
- **What a model states:** the effects and roles of stdlib, dependency and framework callables, in
  the summary access-path grammar below.
- **Effects** come from an append-only codebook: `io.read`, `io.write`, `net`, `log`, `timeout`,
  `thread_dispatch`, `compress(format)`, `serialize(format)`, `validate(schema)`,
  `register(container)`, `invoke(callable)`.
- **Framework models** cover pydantic `BaseModel` and `Field` (a generated `__init__` and a
  validation effect), pydantic-settings (environment binding), ContextVar `get`/`set`,
  `functools.partial` and `wraps`, anyio and asyncio, `contextlib` and logging.
- **Provenance:** origin `synthetic_model`; each file's digest joins `compiler_digest`.
- **Authorship:** the operator, from library source and docs. **Never from `.claude/skills/`.**
- **Normal completion** (ADR-0033): an optional authored assertion means that, after its
  arguments have been evaluated, the pinned function target returns normally. It defaults
  false (unknown), is permitted only with complete exception coverage and no exception rule,
  and is carried on the validated `model_targets` row. It is not inferred from a transfer rule
  or from the absence of modeled exceptions. The first two assertions are for pinned
  `typing.cast` and `typing.assert_type`, whose CPython 3.14.7 bodies directly return `val`.

**Implemented and Tested in focused cases (2026-09-25, Stage 3.1 model assertion boundary):**
Serde tagged enums with unknown-field rejection parse the committed `external.toml` catalog;
one renderer writes display paths, while typed path components determine stable path ids.
Source bytes, target pin and revision enter model identity, and catalog bytes join the compiler
digest. An analyzed compile binds applicable targets to pinned `context_definitions` only when
the module origin and exact Python or distribution pin match. `model_targets` cites the module
and definition facts. An unreferenced target stays dormant; an active target with incomplete
signatures or an unresolved formal fails before Delta writes. One compilation pass produces
typed `model_transfers`, `model_effects`, `model_callbacks`, `model_resources` and
`model_exceptions` assertions, all with `synthetic_model` provenance and an authored rule id.
Transfer, effect and callback assertions distinguish definite from potential; callback and
resource actions identify their exit. Each target declares transfer, effect, callback, resource
and exception coverage independently as complete, partial or unspecified. Only complete
coverage can later support a negative summary conclusion. The shared publication validator
reconstructs every model row from the committed catalog and pinned context, rejecting missing,
extra or altered rows. `typing.cast`, `typing.assert_type`, `builtins.print`, `json.dumps`,
`json.dump`, `builtins.open` and `atexit.register` are the first authored targets. These rows
remain model assertions: summary application, broader pure-model oracles and served behavioral
claims remain Proposed. Focused
release Nextest and Clippy passed; the integrated gate and fresh pilot are reserved for the
assembled Stage 3 end.

**Implemented and Tested in focused cases (2026-09-25, Stage 3.1 decoding/compression data):**
the pinned `json.loads`, `gzip.compress` and `gzip.decompress` targets add potential
input-to-return transforms. Only `gzip.compress` asserts a potential `compress(gzip)` effect
on its exact `data` formal. None asserts total normal completion or complete channel coverage:
malformed inputs, resource exhaustion, and JSON's caller-supplied hooks leave those claims
open. Pinned CPython 3.14.7 signatures and the Python standard-library JSON/gzip documentation
support the authored paths; the focused analyzed fixture checks target binding, formal identity,
source application, a shadowed-module withholding case and shared publication equality. These
rows are candidates, not a positive transform summary or a CrossHair equivalence claim.

**Implemented and Tested in focused cases (2026-09-25, Stage 3.1 logging data):** the pinned
`logging.Logger.warning(msg)` target asserts a potential `log` effect on its exact message
formal. Logger configuration can suppress emission, and handlers can execute arbitrary code,
so the other model channels and normal completion remain open. The source fixture has a
bound object-receiver call, an untyped logger call and an unpacked message; only the first
binds its `msg` subject. The method-binding rule is below (ADR-0035).

**Implemented and Tested for authored parsing (2026-09-25, Stage 3.1 Pydantic candidate):**
the pinned dependency target `pydantic==2.13.5` `TypeAdapter.validate_python(object)`
declares a potential input-to-result transform. The adapter's schema is instance-selected;
the current `validate(schema)` effect cannot name it without a false exact-schema claim.
Arbitrary user validators can add effects or raise, so the other channels and normal completion
stay open. The empty-site fixture correctly leaves this dependency model
dormant. **Interface-checked (2026-09-25):** the pinned local source has the `object` formal,
and a read-only earlier FastMCP snapshot contains its matching dependency module and method
definition. Current-compiler binding to the full pinned dependency context is **not_run**
until the integrated pilot; these observations do not certify a source application or summary.

**Implemented and Tested in focused cases (2026-09-25, Stage 3 source/model bridge):**
`model_applications` joins each source `call_targets` fact to an exactly pinned `model_targets`
row, retaining the Pysa target's modality/origin and phase, the model's identity and revision,
and `resolutions`' candidate-set completeness, unresolved remainder and target count. The row
also carries the selected pinned target's authored normal-return assertion; neither the
candidate-set-complete flag nor the assertion alone means this source call must return.
Higher-order argument
targets and annotation-only calls cannot masquerade as direct invocation. The shared validator
reconstructs this join, and a shadowed builtin call has no application. This is evidence that a
model applies at a call site, not evidence that a callback ran, a resource was released or a
transfer completed. Those L2/L3 fates remain Proposed; the integrated gate is `not_run`.

**Implemented and Tested in focused cases (2026-09-25, typed formal-path bridge):** the
catalog compiler also publishes `model_formal_paths`, keyed by rule, typed path identity and
input/output role. A row names a pinned formal only when the catalog AST path refers to one;
return values and globals produce no false formal. All authored formals were already checked
against every pinned Pysa signature, and the shared validator reconstructs these rows from
committed model bytes. Source-argument binding remains Proposed; callers must not parse the
rendered path string to fill that gap.

**Implemented and Tested in focused cases (2026-09-25, model argument boundary):**
`model_argument_bindings` joins each applied model formal to the source call's explicit
argument only if every pinned Pysa signature selects the same argument ordinal. It supports
positional and named keyword binding. For a direct Ruff attribute callee whose Pysa target
cites an implicit **object** receiver, the binder shifts positional signature ordinals past
that receiver. An unpacked argument, class receiver, unsupported callee, absent argument
or overload disagreement yields `unknown` with a boundary, never an inferred value. The row
cites the chosen argument and its source fact when bound, and shared publication validation
reconstructs every row. Binding is still local to one candidate model target. Dispatch
closure, value stability, effects and summary composition remain Proposed; no served verdict
follows from this relation alone.

> Decision: ADR-0035

**Implemented and Tested in focused cases (2026-09-25, modeled callback source):**
`modeled_callback_sites` applies an authored callback action to a cited source call target
without erasing the model exit, either modality, or the target-set boundary. It carries the
bound source callback argument when every signature agreed, otherwise an explicit unknown
binding reason. The first positive shape is `atexit.register`'s `registered` action on normal
exit; starred and positional-only keyword calls retain unknown binding. The row is a
candidate-local model action, not evidence of callback invocation or a whole-operation fate.
Publication reconstructs it; other callback and resource sources and L3 composition remain
Proposed.

**Implemented and Tested in focused cases (2026-09-25, modeled resource source):**
`model_resources` now carries the path kind compiled from its tagged AST, independent of
the rendered access-path string. `modeled_resource_sites` applies an authored resource action
to a pinned source call target. A `ReturnValue` path identifies the call expression as the
source of the returned resource; a parameter path can identify an argument only through the
exact `model_argument_bindings` relation. Field and global paths, failed argument bindings
and open dispatch remain explicit boundaries. The row retains the action, exit and both
modalities, so `builtins.open` yields a candidate `acquire` on normal return but a shadowed
`open` does not. A call-expression id is not a runtime resource identity, and this relation
does not prove the call completed or that any release happened. The shared validator
reconstructs these rows and rejects a doctored source status. L2 lifecycle pairing and L3
summary composition remain Proposed; the integrated gate is `not_run`.

**Implemented and Tested in focused cases (2026-09-25, modeled transfer source):**
`model_transfers` now carries input and output path kinds compiled from the tagged catalog
AST. `modeled_transfer_sites` applies one authored transfer to one cited source call candidate.
For a parameter input it takes the exact source argument only from
`model_argument_bindings`; for a `ReturnValue` output it identifies the call expression and
call fact. Unsupported paths or argument shapes keep the endpoint `unknown` with a reason.
The row retains the transfer kind, target and model modalities, candidate-set completeness,
and unresolved remainder. `typing.cast`, `typing.assert_type` and `atexit.register` produce
candidate identity
transfers when their inputs bind; invalid keyword or unpacked arguments do not acquire a
false source value. Shared publication validation reconstructs the relation and rejects a
forged endpoint status. This is not yet a summary flow or a whole-operation verdict:
the call's condition, dispatch, normal return and enclosing callable still need L3
composition. The integrated gate remains `not_run`.

**Implemented and Tested in focused cases (2026-09-25, modeled effect source):**
`model_effects` now carries an optional subject path kind from its tagged AST.
`modeled_effect_sites` joins an authored effect to one cited source call candidate while
retaining the effect kind and argument, target and model modalities, and open candidate-set
state. A subjectless effect is explicitly `unqualified`, with no invented stream or value;
a parameter subject is identified only through exact pinned-signature argument binding;
other or unbound subjects remain `unknown` with a reason. The first fixture maps
`builtins.print` to a potential I/O write at the `display` call, without claiming which
stream receives it. Shared publication validation reconstructs the row and rejects a
forged subject status. Source occurrence is not proof of a completed effect or a
whole-operation fate; L3 composition and integrated Stage 3 testing remain open.

**Implemented and Tested in focused cases (2026-09-25, JSON model family):** pinned
`json.dumps` contributes a potential `Parameter[obj]` → `ReturnValue` transform and a
potential JSON serialization action on `obj`; pinned `json.dump` contributes potential
serialization of `obj` and a potential I/O write on the exact bound `fp` argument. A
custom encoder or `default` callable leaves other effects and callback behavior open.
The focused source fixture verified the typed formal bindings and candidate-local rows;
these actions do not establish successful serialization, stream ownership, or a
completed I/O effect. Integrated tests remain `not_run`.

**Implemented and Tested in focused cases (ADR-0029, 2026-09-25, modeled exception source):**
`model_exceptions` carries the uniquely pinned context class node and fact for each authored
source and conversion class. `modeled_exception_sites` attaches an action such as the potential
`builtins.OSError` of `builtins.open` to its cited source call candidate, retaining model and
target modality and unresolved dispatch. An unreferenced model has no applied row. The shared
publication validator reconstructs catalog and source rows and rejects forged class identity.
These are candidate model actions, not observed exceptions, handler catches or exits. The
integrated Stage 3 gate remains `not_run`.

**Tested, narrow oracle (2026-09-24):** an isolated CrossHair 0.0.110 `diffbehavior` probe on
CPython 3.14.7 exhausted the paths for the pure `int` specialization of `typing.cast` versus
identity; a deliberately wrong control produced `value=0`. This does not certify the generic
model or its use in a summary. [Evidence](../../design_review/evidence/2026-09-24_typing_cast_model_oracle/README.md).
**Tested, narrow oracle (2026-09-25):** an isolated CrossHair 0.0.110 probe exhausted
paths for the pure `int` specialization of `typing.assert_type` versus identity; a wrong
control returned `value=0`. The generic model and summary use remain unproved.
[Evidence](../../design_review/evidence/2026-09-25_typing_assert_type_model_oracle/README.md).

**Implemented and Tested (2026-09-24, Stage 3 L2 structural exits only):** every compile
derives `exit_sites` from Ruff `syntax_nodes` and ty `flow_regions`. Explicit `return` and
`raise` statements and direct actions in a `try` statement's `finally` body carry the owning
function, source and region fact ids, path condition, and approximation flag. A `return` or
`raise` directly in `finally` has both site kinds. The relation is independent of a Stage E
analysis configuration; publication reconstructs it from the pinned raw views. It does not
prove that a raised exception escapes, a handler catches it, or a `finally` action completes.
Those fates, callbacks, resources and composed summaries remain Proposed.

**Implemented and Tested in focused cases (2026-09-25, bounded return frames):**
`return_exit_statuses` walks each attributed return's same-function syntax ancestry to a
declared depth cap and cites the nearest controlling `with` or pending `finally` frame. A
missing frame with no cap is only a local normal-return candidate, not proof that expression
evaluation succeeds. Direct, modeled and acyclic local-call value summary seeds may now admit
nested returns under ordinary branches, but require a status without an unresolved frame/cap
boundary.
Returns under `with` or an unproved pending `finally` remain `summary_boundaries` until L2 proves normal
completion. The shared validator reconstructs the status rows; a focused analyzed fixture
admits an `if` return while withholding both controlling frames. This is compiler output
version 65 and a schema migration. Nested handler propagation, suppression, callback/resource
fates and the integrated Stage 3 gate remain open.

**Implemented and Tested in focused cases (2026-09-25, ordered pass finalizers):**
`return_exit_statuses` admits a bounded chain of pending `try/finally` frames only when
each entire direct `finalbody` is one literal `pass`. The one-frame status cites its pass
node and fact; the multi-frame status leaves those singular fields null. A separate source
query reconstructs every pass in inner-to-outer execution order, and each admitted finite
summary cites all of them as ordered `finalizer_pass` steps. The return expression still
needs its own normal-evaluation and condition proof. A nontrivial suite, `with`, and capped
ancestry retain their control boundary. Direct and modeled analyzed fixtures prove the
single and nested pass paths, and a mixed effectful outer finalizer stays unknown; the
shared validator rejects missing pass evidence. The nullable one-pass status fields were
introduced at compiler output version 66, and the ordered derivation changes output version
70. Other handler, callback and resource fates and the integrated Stage 3 gate remain open.

**Tested (2026-09-25, targeted CPython 3.14.7 oracle):** an isolated `sys.monitoring` worker
observed a pending local value return through `finally: pass` and an overriding `finally`
return control. `PY_RETURN` locates the first completion at the finalizer line, so the worker
attributes the returned identity to the latest executed load at an AST `return Name` expression
span rather than requiring the event line to equal the load line. This independently checks
value-flow admission and observed exit regions for these two shapes; it does not prove the
compiler's L3 summary closure or the remaining finally actions.

**Implemented and Tested in focused finite-proof cases (2026-09-25):** direct identity and
modeled/assignment/local-call summary producers use the same safe return status and cite
each source pass from the ordered ancestry query; modeled proofs place the pass sequence
immediately before `return_exit`. The canonical summary ID hashes these steps, and the
shared validator rejects their removal. Uncontrolled frames still have no positive summary.
The append-only step kind was introduced at output version 69. Full per-step source spans in
FORMAT 7 and effectful-frame exit composition remain open.

> Decision: ADR-0037

**Implemented and Tested (2026-09-24, Stage 3 L2 handler source boundary):**
`handler_clauses` cites a `try`, each authored `except` clause, its optional type expression,
and the region reaching the `try`. `handler_actions` cites direct statements in that clause's
body and their own ty regions. These rows are derived for every compile and reconstructed by
the publication validator. **Implemented and Tested in focused cases (2026-09-25):**
`handler_types` keeps one status per clause. A direct `Name` gets `pinned_builtin` only if its
lexical reference resolves uniquely to a builtin and one matching class is present in the pinned
context; bare `except` has its own status, while shadowed, compound or unbound types stay
`unknown` with a boundary reason. The row cites the lexical and context facts and is
reconstructed at publication.
This identifies an authored handler class, not a catch. The `try` entry condition is not a
handler-match condition, and a body action may fail or branch. Full exception matching, conversion
and completion remain Proposed. The integrated repository and pilot gates remain `not_run` for
Stage 3.

**Implemented and Tested in focused cases (2026-09-25, local handler return):**
`handler_return_none_sites` records a handler whose sole direct body statement is
`return None`, citing that statement, its exact `None` literal and ty region.
The region's `approximated` flag is retained: the focused `except` body is
approximate under ty. A computed return or an earlier body statement produces
no row. Shared publication validation reconstructs the relation and rejects
missing rows. This is a local, pre-`finally` witness conditional on entering
the handler, not a proof that the modeled exception selects it or that the
operation completes normally. `COMPILER_OUTPUT_VERSION` is 45; integrated
Stage 3 testing remains `not_run`.

**Implemented and Tested in focused cases (ADR-0031, 2026-09-25):**
`modeled_exception_return_none_paths` composes a modeled potential raise with
the first provable matching clause of a direct function-body `try` and that
handler's sole direct `return None`. A complete syntax-ancestor walk must show
no inner `try` or `with`; the frame must have no `finally`. Each path retains
the cited raise, class relationship, handler return and ty region, together
with target/model modalities, open dispatch and approximation. This is a
candidate-local path conditional on the modeled raise, not a completed
operation-level catch or normal-return verdict. Nested frames, uncertain
clause precedence, computed handler actions and finalizers remain unknown.
Shared publication validation reconstructs the rows. `COMPILER_OUTPUT_VERSION`
is 46; integrated Stage 3 tests remain `not_run`.

> Decision: ADR-0031

**Implemented and Tested in focused cases (2026-09-25, candidate handler frame):**
`modeled_exception_handler_candidates` uses a bounded DataFusion recursive syntax-ancestor
walk, stopping at an innermost-function boundary, to connect a modeled potential raise in a
`try` body to each authored clause of that frame. It retains nested frames, clause ordinals,
call/model/class facts and the handler type fact. The class relation is typed as same pinned
class, bare handler, unresolved relationship between different pinned classes, or unresolved
handler type. `modeled_exception_handler_walks` records one coverage row per modeled raise;
missing source syntax and a 128-edge ancestry cap produce explicit reasons. Absence from the
candidate relation is interpretable only when its walk is complete. Publication reconstructs
and validates both relations. **Implemented and Tested in focused cases (2026-09-25):**
`frame_possible` excludes a later clause after a proven earlier match within the same frame;
`frame_first_match_if_raised` requires a positive match and no prior possible match. An
unknown earlier class relationship leaves later clauses possible but not proven first.
These booleans are conditional on the modeled raise reaching this frame. They do not decide
whether the call raises, an inner frame propagates, or a handler completes. Those L2 fate
decisions and integrated testing remain open.

**Implemented and Tested in focused cases (ADR-0030, 2026-09-25):**
`context_class_mro` retains Pyrefly's resolved ancestor identities or an empty/cyclic marker
for each pinned context class. When a modeled raised class's MRO contains the pinned handler
class, `modeled_exception_handler_candidates` records `pinned_ancestor` and the source MRO
fact. A missing, unbound or cyclic relationship stays `class_relation_unknown`; absence from
the MRO is not a negative match because a model class can denote possible subclasses. This is
still a candidate catch relation. The integrated Stage 3 tests
remain `not_run`.

**Implemented and Tested in focused cases (2026-09-25; ADR-0027):** until those L2 fates are
proved, an explicit raise inside a `try` or `with` body has no definite escape witness. The
flow producer no longer parses handler/raised names from source text or assumes an opaque
context manager cannot suppress. This is conservative withholding, not a caught-exception
claim; an explicit unframed raise still establishes escape.

**Transfer summaries** are a Stage E kernel (`lctx_analytics::summaries`).
- **Condition semantics (ADR-0024, Proposed):** summary composition and Stage 4 definitions
  call the shared bounded diagram kernel. `summary_flows` and `summary_effects` reference
  lossless condition roots; a rendered DNF is only display. Type-derived scalar exclusions
  require a stable-value witness, and node-limit hits become explicit boundaries.
- **Output tables:**
  - `summary_flows`: callable, input path, output path, kind (`value`, `transform`, `constant`),
    condition, verdict;
  - `summary_effects`: callable, effect, role bindings, condition;
  - `summary_boundaries`: callable, reason, site.
- **Paths:** `Parameter[name]`, `Parameter[self].Field[f]`, `ReturnValue`,
  `Argument[formal]@Call[target]`, `Global[<module>.<name>]`, `Raise[T]`. The shape is CodeQL's
  models-as-data, without its file format. These are the **resolved** place key's written form
  (§3.9's two layers; ADR-0022 §Places), never a third grammar.
- **Order and fixpoint:** the call graph's SCCs in `tarjan_scc` order (callees first), each SCC
  iterated to a fixpoint over a finite domain: path depth ≤ k and condition size ≤ c.
  - **Widening** yields `unknown` (`budget_reached`), and the invocation records its budgets.
  - **Override-open calls** join their candidates and stay marked open (§3.6).
  - **Exceptions** convert through `handlers`.
  - **A call alone never propagates an effect.**
- **Oracle:** Pysa's inferred TITO models on the pinned library, run offline. It is differential,
  not truth.

**Tested (2026-09-25, Stage 3.4 targeted differential):** a separate Pysa 0.10.0 probe uses
an actual source-to-sink rule, verified source/sink models and an explicit Pyrefly 1.3.1
binary. It reports the expected source-to-sink issues and exact TITO ports for direct
identity and a one-call wrapper, while a constant-return control has no issue. The local
Pyrefly configuration prevents accidental reuse of this repository's unrelated project
include list. No obscure-callee feature appears on the two positive ports. A byte-identical
compiler fixture yields finite parameter-to-return summaries for the same identity and
wrapper, and none for the constant control; shared publication validation passes. This is
a targeted matched-source differential. Comparison with pinned FastMCP `summary_flows` and
classification of its disagreements remain Proposed.
[Probe](../../design_review/evidence/2026-09-25_pysa-tito-rule/README.md).

**Proposed call-result join (ADR-0028, 2026-09-25):** summaries read a validated,
ordered `flow_values` call path through `value_flow_contributions`, then join each local step to one exact pinned call target and
modeled argument/result pair. The path carries operand role and direct-value span; a
`through_call` flag or shared text alone cannot discharge `call_transfer`. Missing,
ambiguous, computed or budget-cut steps write a boundary. This bridge precedes SCC
composition and negative claims.

**Implemented and Tested in focused cases (ADR-0028, 2026-09-25, direct
transfer candidate):** `modeled_exact_value_transfers` joins a raw return or definition-value
fact and its unmerged parameter contribution to exactly one ordered call step,
its uniquely bound Ruff argument, and a pinned model whose argument input and
call-result output cite those same nodes. The call span must equal the full
value sink span, excluding an enclosing computation or fallback.
The upstream transfer must be
identity and the source parameter must belong to the sink's callable.
The row retains sink kind/span, condition, raw-fact approximation, target/model
modality and open dispatch. An assigned intermediate's exact model-call value
step is retained; nested calls and outer computations have no exact step. The
predecessor chain still needs compatibility and completion proof. Shared
publication validation reconstructs the relation and rejects dropped rows.
This is a candidate source-to-value
path, not proof that the call completes or a `summary_flows` verdict.
The relation began as return-only in output version 47 and includes definition
values in output version 52; integrated Stage 3 testing is `not_run`.
**Implemented and Tested in focused cases (2026-09-25):** the exact
call/sink-span equality closes the one-call outer-expression gap; output
version 50. An outer Boolean fallback remains outside this direct bridge.

**Implemented and Tested in focused cases (2026-09-25, argument-evaluation boundary):**
`modeled_argument_evaluations` records every explicit argument of each exact one-call model
candidate in source ordinal order. The selected source operand cites its raw candidate value
fact and exact argument role. A direct Ruff string, bytes, number, Boolean, `None` or ellipsis
literal sibling cites its syntax fact and has a local normal-evaluation witness. An exact
unshadowed builtin-name sibling cites lexical resolution as its normal-evaluation witness.
An unpacked, dynamic, shadowed or otherwise unproved sibling has an explicit
`outside_provider_model` boundary and no evaluation witness. These rows do not yet prove the callee expression, source call completion or
enclosing exit. The shared validator reconstructs them, including the candidate-specific source
role; output version 60 and integrated Stage 3 testing remains `not_run`.

**Implemented and Tested in focused cases (ADR-0028, 2026-09-25, predecessor
candidate):** `value_flow_predecessor_candidates` joins an inherited-call
contribution's use to a cited provider reaching definition, its value span,
and a raw predecessor value fact with the same parameter origin and sink
callable. Each edge retains the reaching condition, the separately recomposed
predecessor/successor conditions, loop-carried and approximation flags, and
whether the predecessor's call crossing is local or inherited. Assignment
followed by return has a candidate edge; a nested call in one expression does
not acquire an invented predecessor. The shared validator reconstructs the
relation and rejects missing rows. This edge does not establish condition
compatibility, uniqueness, transfer or completion. Recomposed analysis
condition ids may lack rows in provider `conditions`; L3 must persist their
structural BDD roots before composing them. `COMPILER_OUTPUT_VERSION` was 48
for this candidate relation; integrated Stage 3 testing is `not_run`.

**Implemented and Tested in focused cases (ADR-0032, 2026-09-25):**
the flow analysis's recomposed condition ids now have a separate,
content-addressed root/node catalog (`analysis_conditions`,
`analysis_condition_nodes`). The producer serializes the existing BDD objects;
publication reconstructs every row and uses the same `hydrate_catalog`
structural checks as the provider and native loader. `modeled_exact_value_transfers`
and `value_flow_predecessor_candidates` reference this catalog for recomposed
conditions, while a raw reaching condition still references provider
`conditions`. This closes the cited persistence prerequisite, not condition
compatibility or summary completion. `COMPILER_OUTPUT_VERSION` is 49;
integrated Stage 3 testing remains `not_run`.

**Implemented and Tested in focused cases (2026-09-25, bounded predecessor
compatibility):** `lctx_analytics::summaries` hydrates the provider and
flow-analysis BDD catalogs through the shared structural validator, then
conjoins the predecessor, reaching and successor roots for each cited
`value_flow_predecessor_candidates` edge. It writes a tri-state
`value_flow_predecessor_compatibility` row: false refutes this source-path
candidate under the declared atoms, true admits a may-compatible path, and
loop-carried, missing or capped roots carry an explicit unknown boundary.
An input catalog boundary that names a cap remains `budget_reached`; it is
not relabelled as missing evidence.
The compatibility check does not choose a reaching definition or prove the
modeled call's transfer or completion. Publication reconstructs every row.
`COMPILER_OUTPUT_VERSION` is 51; integrated Stage 3 testing is `not_run`.

**Implemented and Tested in focused cases (2026-09-25, two-step model path):**
`modeled_assignment_return_paths` joins an exact whole-assignment modeled
value step to a later raw identity return via one cited reaching definition.
It keeps every separate candidate and its bounded BDD compatibility result,
three condition ids, approximations, model provenance and open target status.
A computed outer return cannot take this route. A true compatibility result
admits only a may-path; call completion, handler/finally action and complete
candidate selection still need L3. Publication reconstructs the relation.
`COMPILER_OUTPUT_VERSION` is 53; integrated Stage 3 testing remains `not_run`.

**Implemented and Tested in focused cases (2026-09-25, finite summary base):**
`summary_flows` begins with a narrow synchronous direct-body identity return
of a local parameter, with no crossed call or generator yield. Each row cites
its raw value fact, return syntax/region facts and recomposed structural BDD
condition. The shared bounded kernel gives `established`, `conditional`, or a
named `unknown`; a false condition yields no positive flow. Async returns,
generators, unproved nested frames and modeled-call candidates are withheld until L2/L3
proves their execution and completion semantics. Shared publication validation
reconstructs the rows. The `summary_flow_kind` codebook was appended and
`COMPILER_OUTPUT_VERSION` is 54. Effect summaries, SCC composition and
integrated Stage 3 testing remain open.

**Implemented and Tested in focused cases (2026-09-25, summary coverage):**
`summary_boundaries` records each same-callable parameter-origin raw return
path outside the finite direct summary producer. A crossed call retains
`call_transfer`; other unproved control/execution shapes retain
`unsupported_control_flow`. A positive contribution does not hide a separate
unproved contribution to the same fact. Each row cites the raw value fact and
condition id, and shared publication validation reconstructs it. This is
unknown coverage rather than a negative verdict. `COMPILER_OUTPUT_VERSION` is
55; full L3 closure and integrated Stage 3 testing remain open.

**Implemented and Tested in a targeted sibling-origin case (2026-09-25):** the finite producer
now suppresses a boundary for a proved raw fact/condition only when that key has exactly one
same-callable parameter-origin contribution. A second contribution sharing the raw fact and
condition keeps an aggregate boundary even if one summary is positive; the current boundary
key cannot identify which sibling was proved. This conservative rule makes the earlier coverage
claim true without treating a positive may-path as exhaustive. The shared validator uses the
same derivation. Compiler output version 67; full path-specific origin closure remains open.

**Implemented and Tested in focused cases (ADR-0034, 2026-09-25, summary proof identity):**
`summary_flows` now keys on a canonical path id computed from callable, formal, input/output
paths, transfer kind, condition, return site/region and ordered typed evidence. `summary_flow_steps` stores the first
`raw_identity` witness citing its source `flow_values` fact and BDD condition. A pair of paths
with equal endpoints but different evidence or order gets different ids. The publication
validator reconstructs both tables; an omitted step is rejected. This changes the identity
contract under compiler output version 58, without promoting a modeled call to a completed
flow. The integrated gate remains `not_run`.

**Implemented and Tested in focused cases (2026-09-25, first modeled return):**
An exact whole-expression `typing.cast` or `typing.assert_type` identity call can now seed a
finite `summary_flows` value path when its sole source target is closed, both target and model
modalities are definite, the pinned target asserts normal return, its callee is one resolved
simple name, and every explicit argument has ordered local normal-evaluation evidence. The
candidate BDD must be satisfiable and imply the direct synchronous return's region BDD;
generator functions and unresolved control frames remain excluded. Typed steps cite callee
resolution, argument evaluations, call syntax and target, the model rule, and return exit.
The canonical summary id includes this ordered proof, which the shared validator rebuilds.
`summary_boundaries` remains for return facts without an admitted path. This narrow positive
producer does not establish assignment predecessors, broader call compositions, exception or
effect fates. Compiler output version 61; integrated Stage 3 testing remains `not_run`.

**Implemented and Tested in focused cases (2026-09-25, assignment return):**
The same completed model-call proof now admits a two-hop assignment-to-return identity path
only when the returned use has one reaching-definition row, the provider predecessor is
compatible, and its BDD condition implies the reaching, successor-value and direct return
region conditions. Ordered steps cite the source call, model, unique definition edge and
returned value; a different predecessor changes the summary id. Unproved siblings retain a
named boundary. This is not recursive composition or general assignment transfer. Compiler
output version 62; integrated Stage 3 testing remains `not_run`.

**Implemented and Tested in focused cases (2026-09-25, source-call SCC topology):**
`summary_components` records every release function's SCC, sorted members, canonical
component id and deterministic callee-first schedule over attributed local call targets.
Petgraph 0.8.3 computes the SCCs; a sorted condensation worklist makes ties independent of
provider row order. A self-call marks a singleton recursive. Candidate/open dispatch contributes
topology only, not a completed transfer or negative coverage claim. The shared validator
reconstructs the rows from source calls. Compiler output version 63; composition, discharge
and integrated Stage 3 testing remain open.

**Implemented and Tested in focused cases (2026-09-25, first local composition):**
An acyclic synchronous wrapper can inherit an unconditional value summary of its sole
definite local target when ty's one-call value path, Ruff's exact one-positional-argument
syntax, lexical callee resolution, Pass B's single formal mapping, closed source target set
and direct return exit all agree. The caller condition must imply its return region; the
callee condition must be true, avoiding unproved cross-scope atom substitution. The proof
steps cite the callee summary id, so parallel callee paths stay distinct. A callee-first SCC
schedule propagates these finite paths to later acyclic callers with depth capped at eight.
Recursive and conditional callee paths remain unknown, and a depth refusal currently retains
the generic `call_transfer` boundary pending a specific budget row. Compiler output version
64; full SCC composition, effect summaries and integrated Stage 3 testing remain open.

**Implemented and Tested in focused predecessor-control cases (2026-09-25):** version 71
withheld all positive finite paths for recursive SCC members after an unconditional self-call
exposed an unproved predecessor completion. Version 72 added an earlier same-function call
screen, admitting a base return before recursion. Version 73 (ADR-0039) refines it with ty
statement regions: DataFusion selects each call's narrowest enclosing region, then the bounded
BDD kernel ignores an earlier call only if that region and the return-value condition have
a proved false conjunction. Missing, approximate or over-budget conditions retain an
explicit `summary_boundaries` unknown. An `else` return disjoint from an earlier `if` call
now has a finite may-path; an unconditional recursive or nonrecursive prior call remains
unknown. This is still not a normal-completion proof for a compatible call or non-call
predecessor. The modeled and assignment producers withhold recursive members; the local-call
producer already did so. Full predecessor execution and SCC worklist remain open, as does
integrated acceptance.

> Decision: ADR-0039

**The capability registry** lives in `cpg-schema`, as TOML compiled to Arrow.
- **A concept** has:
  - an append-only id, a `prefLabel`, `altLabels` (each with its source), `broader`/`related`, a
    scope note and facets;
  - a **definition**: a conjunctive query with shared variables over the `behavior` family and the
    summaries, written as a Rust enum AST, compiled to DataFusion SQL and digested.
- **`concept_members`** is materialized: concept, operation, role bindings, condition, verdict,
  witness.
- **Rules:**
  - every member cites a definition digest and a witness;
  - SKOS integrity: `broader` acyclic (a recursive CTE); `related`
    disjoint from the `broader` closure.
- **Discovery nominates, definitions decide.** FCA over behavioral attributes suggests facets;
  vectors per view rank. Communities have no tool consumer (§9.8). None of them writes
  `concept_members`.
- **`lookup_concepts`** returns every concept with its labels and scope notes while the catalog is
  small: 20–40 authored. Ranked lookup waits until the catalog outgrows one page (the ADR review's
  F14).

> Decision: ADR-0022, ADR-0024, ADR-0027, ADR-0028, ADR-0029, ADR-0030, ADR-0032, ADR-0033, ADR-0034

---
