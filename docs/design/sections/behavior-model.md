<a id="section-3-9"></a>

# §3.9 Behavior model: places, conditions and verdicts

This page owns the meaning of behavioral facts: which **places** a behavior is about, the
**runtime view** and flow provider that produce reaching definitions and regions, the
**condition** under which a fact holds, and the **verdict** every behavioral answer carries.
Inputs are ty's semantic index read through `cpg-flow`, Ruff syntax and our lexical resolution
([§4.2](acquisition-and-extraction.md#section-4-2)) and Pyrefly facts
([§3.2](facts-and-identity.md#section-3-2)). Outputs are the `flow` family, condition catalogs,
value links, verdicts, negative premises and boundaries. Consumers are behavior derivation
(`cpg-core`: `flow_model.rs`, `behavior.rs`, `entry_links.rs`), the summary producers in
[§9.9](behavioral-analysis.md#section-9-9), the shared publication validator
(`cpg-core/src/validate.rs`) and the native executor (`python/lctx_semantics`). Dependencies point
down to `cpg-schema`, which declares the contracts: `condition.rs` (atom language),
`condition_kernel.rs` (bounded diagrams), `primitive_theory.rs`, `tables.rs`/`flows.rs`/`behavior.rs`
(Arrow rows and derivations), `rules.rs` and `codebook.rs` (`condition_atom`, `verdict`,
`boundary_reason`). Tests: `crates/cpg-flow/tests/`, `crates/cpg-schema/tests/conditions.rs`,
`crates/cpg-extract/tests/flow_runtime_resolution.rs`, `crates/cpg-core/tests/`, and the runtime
oracle ([§8.1](validation-and-evaluation.md#section-8-1)). See the [architecture map](../README.md).

**Evidence.** Unless a line says otherwise, places, the flow provider, the runtime view, the
condition language, verdicts, premises, boundary reasons, the read phase, identity and composed
layers are **Implemented** and **Tested** (Stage 2 exit, 2026-09-24). The condition kernel, the
typed primitive theory, value links and exact origins are **Implemented and Tested in focused
cases** (2026-09-24/25); the kernel's general allowance remains proposed (ADR-0024). Integrated
Stage 3 acceptance (`just test-all`, a fresh `just pilot`, the Stage 3 questions) is `not_run`
([plan §1](../../plans/behavioral-model-forward-plan_2026-09-24.md#1-current-state-and-qualification-boundary)).
The rationale is ADR-0045.

## Places

A place is what a definition or use names:
- a parameter or local name;
- `self.f` or `self.f.g` (at most two fields below `self`);
- a module global, settings singletons included (`fastmcp.settings.<field>`);
- **Proposed** (Stage 5, with the framework models that read them): a ContextVar object and an
  attribute of a function object (`fn.__fastmcp__`).

A place is known in two layers:
- **Spelled**: as written in its scope, a root name and at most two attribute segments, no
  subscript. It is a condition's text and a label only.
- **Resolved**: the join key for value flows, field reads, premises and summaries. The root
  resolves through our reference resolutions or the flow IR's reaching definitions to
  `Parameter[…]`, `Local[…]`, `Field[C.f]` (`C` is the class whose method binds `self`) or
  `Global[module.name]`, then at most two segments. §9.9's access paths are this key's written
  form, never a third grammar. A singleton read as `settings.X` in one module and as
  `fastmcp.settings.X` in another is one place, `Global[fastmcp.settings].X`. The producers emit
  `Parameter`, `Field` and `Global` keys; `Local[…]` is in the grammar but not yet emitted.
- A module's own global binding shadows a submodule of the same name when a dotted root resolves
  (`fastmcp.settings` is the `Settings()` instance, not `fastmcp/settings.py`), as the runtime
  sees it once the package binds the name after importing the submodule. That import order is
  **assumed, not checked**.

Anything deeper, or reached through a computed name, is a boundary: `dynamic_access` for the
getattr family and its kin, `unresolved_target` otherwise. It is never a guess.

## The flow provider

The flow IR is our stated runtime abstraction ([§B5](../DESIGN.md#section-b5)): reaching
definitions over places with the conditions under which they reach, statement reachability and
value sources. A provider supplies raw material; it is never the model. Pyrefly's binding graph
is a parity oracle only.

- **What ty supplies.** `ty_python_core` 0.0.14, linked only in `cpg-flow`, builds the use-def
  map (reaching definitions per use, each with reachability as a decision diagram over
  predicates) and every statement's reachability. Regions are relative to their scope's entry: a
  statement inside a function is reached under its region's condition **and** the `def`
  statement's region in the enclosing scope.
- **What crosses the boundary:** byte ranges, place text and our condition data. No ruff 0.0.14
  or ty type leaves `cpg-flow` (ADR-0046). ty, ruff 0.0.14 and salsa `=0.28.2` are a declared
  extra dependency family ([pins](../../pins.md)); a ty upgrade is a pin change with a parity
  rerun.
- **Normalizations:** ty's loop-header definitions become the body bindings they stand for
  (`loop_carried`); an import alias's target becomes the bound name; an augmented assignment's
  target is both a use and a definition.
- **Parity, both ways.** Every flow use joins a `references` row and every reference a flow use;
  likewise for definitions and `bindings`; `semantic:flow-reaching-within-candidates` keeps ty's
  reaching definitions within our candidate bindings. The declared residue: names inside
  annotations and PEP 695 type-alias values (flow uses marked `annotation`, which `references`
  does not model as reads); bindings ty never defines as such (`global`/`nonlocal` declarations,
  `del`, implicit names, star imports); class and alias type parameters. Counting the residue,
  and ty's per-scope saturation, per run is **Proposed**. **Measured** at provider selection
  (2026-09-24, FastMCP 4.0.5, 275 modules): every one of 39,986 references is a ty use; 213 ms and
  47 MiB peak.
- **ty's exception model is part of the stated model.** Exceptions come only from operations ty's
  model says can raise; a `try` whose statements cannot raise has unreachable handlers. Ambient
  exceptions (`KeyboardInterrupt`, `MemoryError`) are outside it.
- **Panics abort the extraction**; `no-catch-unwind-in-extractor` covers `cpg-flow`.
- ty runs with the run context's Python version and platform and a virtual root containing the
  supplied release modules. Focused import-resolution controls for Python 3.8/3.14 passed
  ([plan W14](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

## The runtime view

- **`TYPE_CHECKING` is false.** ty decides it true at index time, so before ty parses a module
  every `TYPE_CHECKING` **name token** (from ruff 0.0.14's lexer; strings and comments keep their
  text; a name in an f-string replacement field is renamed) becomes the same-length sentinel
  `TYPE_CHECKIN_`. A module that already uses the sentinel as a name is refused and its flow
  coverage is `failed`. The sentinel only stops ty deciding by spelling: our evaluator decides a
  reference false only when every candidate in our lexical resolution binds it to
  `typing.TYPE_CHECKING` or `typing_extensions.TYPE_CHECKING` (aliases included, so
  `TYPE_CHECKING as TC` is decided) or binds an attribute's root to the imported `typing` module.
  A parameter so named, or an unrelated `config.TYPE_CHECKING`, stays an ordinary atom. Place
  text, literal values and opaque text are cut from the module as written.
- **`sys.version_info`** comparisons follow Python's full tuple ordering, only through a name
  resolved to the stdlib `sys`. The context knows the first three numeric fields and the runtime
  tuple has two more, so a literal of up to three fields compares; a longer literal stays
  undecided until release level and serial are modeled.
- **`sys.platform`** (`==`, `!=`, `startswith`) and **`os.name`** follow the context's platform
  ([§4.0](acquisition-and-extraction.md#section-4-0)), only through names resolved to those stdlib
  modules.
- ty decides no other test at index time that differs from runtime: its literal folding covers
  `True`, `False`, `None`, integers, `...`, lambdas, generators and `not`.
- C3's static-branch marks ([§4.2.4](acquisition-and-extraction.md#section-4-2-4)) remain the
  checker view the CPG layers keep (below, composed layers).

## Conditions

### Evaluation atoms and identity

Conditions are data in a closed language; there is no Python twin.
- **Literals.** A literal is an atom with a polarity (`is not None` is `!is_none`). The atoms
  (codebook `condition_atom`, append-only) are `is_none(p)`, `equals(p,v)`, `member_of(p,{v…})`,
  `truthy(p)`, `isinstance(p,C)` with `C` as written, `opaque("text")`, `is_value(p,v)` for
  `p is v`, and `type_is(p,C)` for a resolved one-argument builtin `type(p) is C`.
- **Literal values:** `None`, `True` and `False`; decimal integers, negatives included; JSON
  strings (serde_json, non-ASCII kept as UTF-8). A test on any other literal is opaque.
- **Operators stay distinct:** a single-value set remains `member_of`; `== None` is
  `equals(p,None)`, distinct from `is_none`; `is True`, `is False` and `is <literal>` are
  `is_value`; the literal is always the second operand; `in` takes a tuple, list or set of
  literals, anything else is opaque.
- **An atom is an evaluation, not a spelling.** Every source test carries its evaluation site. A
  literal encodes as `[!]kind(place[,value])#module:source`, where the module key hashes the
  relative path and source content and `source` is `s<start>-<end>` for a source site or
  `p<predicate id>` for a provider synthetic predicate. Text is a label; same-spelling tests at
  different sites, including bindings on one line, stay distinct. The reason: even an identical
  reaching-definition set does not prove the value stable. A nested call can rebind a
  `nonlocal`, a global can change, an `isinstance` class expression can be rebound, a call can
  mutate an attribute or container, and literal objects can differ by site.
- **Sharing needs a stability proof.** Equality, membership or truthiness atoms at different
  sites may share an identity only with a proof that no intervening write or effect changes the
  value; a Pyrefly immutable-scalar type at each use is necessary for the typed theory but not
  sufficient. No such proof exists yet (**Proposed**), so nothing is shared. Even with one, calls
  and other opaque tests, attribute places, object state such as a container's truthiness, and
  synthetic predicates stay per site. The parser reserves a definition-set encoding (`d<…>`) for
  that rule; the translator emits none.
- **Loops.** Around a back edge a loop-carried definition's conditions are an earlier
  iteration's, spelled like this iteration's, so the flow model keeps only the use's side there:
  a sound over-approximation. A feasible flow is never stated under `false`
  (`semantic:condition-not-false`).
- **Which parameters a test reads** comes from the flow IR: the uses inside the test's span
  (`flow_tests`), followed through reaching definitions. A `tests` fate carries the innermost
  test's literals.

### Lowering from ty's diagrams

One procedure, in `cpg-flow/src/predicate.rs`:
1. Enumerate paths following only `if_true` and `if_false`; every atom is two-valued at runtime.
2. Read ty's **ambiguous** terminal (reachability it leaves undecided: a `try` body, a loop over an
   unknown iterable, a `with` exit) as `true`, because verdicts state **may**-behavior. The row is
   marked `approximated`.
3. Map each predicate, evaluating the runtime view first and expanding `and`, `or`, `not` and
   conditional expressions.
4. Simplify: drop contradictions; `x | !x` is `true`; self-subsuming resolution to a fixpoint
   (`a & r & !y | r & y` is `a & r | r & y`); absorb. One input in any order has one encoding;
   two inputs with one meaning may still render differently.

- **Opaque text** is the test's source by range, comments removed, whitespace collapsed.
- **Predicates not read as tests:** a call's non-terminal predicate is true (calls are assumed to
  return; `NoReturn` is a model's question); non-empty-iterable, context-manager-suppression and
  finally-path predicates become opaque atoms with fixed text; or-pattern alternatives,
  subject-element patterns and star-import placeholders are opaque with the fixed text
  `<undecided by the flow provider>`.
- **Fates are stated on the normal path.** A statement past a raising guard is reached under the
  guard's negation, which says only that no error was raised; each fate's condition has such
  factors removed (`given`), and the guard is its own `raises_when` fate. The normal path is
  function-wide: guards factor out together, then each alone; a guard past the budget factors
  nothing.
- **A guard needs a definite escape.** `raise_sites.escapes` is set only for an explicit raise
  outside any `try` or `with` body of its function; `false` means **unknown, not caught**. A raise
  inside such a body establishes no escape until an L2 fate proves the frame's action
  ([§9.9](behavioral-analysis.md#section-9-9)): a shadowed handler name can denote any class, an
  opaque context manager can suppress, and a `finally` return can replace the exception.
- **Counted skips.** Each module's flow coverage detail counts candidate reaching rows whose
  condition was `false` (ty's root false, a runtime-view decision, a stable-atom contradiction)
  and discarded value-source branches. Diagnostics only; they create no rows. The generated-program
  `sys.monitoring` oracle checks observed lines and local reaching definitions against the flow
  model ([§8.1](validation-and-evaluation.md#section-8-1)).

### Representation and the condition kernel

**Implemented; Tested in focused cases (2026-09-24/25).** A condition's Boolean function is a
reduced ordered decision diagram (biodivine-lib-bdd 0.6.3) over encoded atoms, with variables
ordered by atom identity. It is persisted losslessly: `conditions` rows name a root and
`condition_nodes` hold the content-addressed nonterminals (atom, low, high), with fixed terminal
ids and a structural Merkle id; a library-local variable index is never an identity. `approximated`
is row-local provenance, not part of Boolean identity. The DNF (at most 16 conjunctions of 8
literals, sorted, deduplicated, joined by ` & ` and ` | `, `true`/`false` as the empty forms) is a
bounded display; nothing parses it to answer a question. A condition that cannot be stated within
budget carries a named kernel boundary and is `unknown` (`budget_reached`), never `false`.

The **accepted** parts (ADR-0045):
- Recomposed analysis conditions have their own catalog, `analysis_conditions` and
  `analysis_condition_nodes`, separate from provider `conditions`; each reference names its
  catalog. Publication and native load hydrate every catalog with one validator
  (`hydrate_catalog`): closure, terminals, acyclicity, strictly increasing atom order, distinct
  children, unique reduced nodes and recomputed ids. A condition from a later summary pass needs
  its own validated extension, not an unproved reference into this catalog.
- The bounded kernel decides the predecessor compatibility screen of
  [§9.9](behavioral-analysis.md#section-9-9); a missing, approximate or capped operand withholds.

The **general kernel allowance** is ADR-0024, which is proposed: the kernel owns `and`, `or`,
`not`, `given`, `implies` (incompatibility of `a ∧ ¬b`) and `compatible` (satisfiability of
`a ∧ b`), each preflighted against support (≤128 atoms), input-node product (≤1,000,000 pair
work) and a 50,000-node result cap; catalogs admit ≤100,000 conditions and ≤100,000 nodes. A
preflight refusal and a node-limit hit are distinct causes; both are unknown, and there is no
wall-clock guarantee. `given` nominates a quotient by the Stage 2 rule and accepts it only after
checking `original == factor ∧ quotient`, else `not_factored`. The target adds restriction and
existential projection, so that a node limit is the only point where information is cut.

**Known defects** ([plan W6, W11](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
`compatible`/`implies` build the full result under the node cap, so a decidable pair can answer
`NodeLimit` (Tested); declared support never shrinks, so atom-limit admission depends on
construction history (Tested); catalog caps count shared stored rows while hydration retains
per-root expansions (256 roots over a shared 64-node tail hydrate to 17,152 owned nodes, Tested),
so no aggregate preparation budget exists yet. `given` still nominates from the DNF and returns
the original when that is over budget, which loses factoring exactly for large conditions; cube
restriction is the intended candidate generator, with the equality gate kept (restriction is not
factor division).

### Typed primitive theory, value links and exact origins

The theory answers one kind of question: given an exact primitive value at an operation's entry
formal, can a source condition hold? It is layered so that identity is proved before any value
reasoning, and every step is cited.

- **Observation vs proof.** `flow_test_leaves` records each provider predicate's leaves before
  `flow_tests`' span-only deduplication, so a compound test keeps several leaves and separate
  `match` arms sharing a subject span keep distinct atoms (key: snapshot, module, predicate key,
  atom id; [probe](../../design_review/evidence/2026-09-24_test-leaf-proof-joins/README.md)).
  `flow_test_types` joins a leaf to its exact operand use (a structural operator-to-operand match,
  never proximity) and to Pyrefly's expression trace at that span; an absent or ambiguous trace
  proves nothing. It is an observation, not an exact-runtime proof. **Implemented and Tested in
  focused cases (2026-09-24).**
- **Value links** (`flow_test_value_links`) are a separate positive proof that a test operand
  observes the entry formal's value. Origins (codebook `test_value_link_origin`):
  `direct_parameter_reach_no_effect` needs one stated, non-approximated, non-loop-carried
  parameter reaching fact, an exact attributed operand and no intervening call, binding,
  potentially effectful syntax or prior predicate evaluation (the recorded literal docstring is
  the one exempt statement); `resolved_builtin_type_operand` admits the guard's own resolved
  one-argument builtin `type(x)` call as a revisioned, cited exception to the call barrier;
  `stable_after_exact_type_guard` links a later use only when the guard's predicate is that atom
  alone, the later reaching path condition implies its true assignment and nothing else
  intervenes. Links store the effect-rule digest and cited facts; publication recomputes the
  relation from pinned raw views. Identity is never inferred from parameter spelling, an
  annotation, an enclosing span or a post-call reaching row; a missing link is unknown. A new
  origin needs its own checked rule and counterexample. **Implemented and Tested in focused
  cases (2026-09-24);** the entry-value bridge
  [probe](../../design_review/evidence/2026-09-24_entry-value-bridge/README.md) fixed the first
  cases.
- **Exact origins** (`flow_test_exact_origins`) cite a `type_is` guard leaf and its link and assert
  an exact builtin class only under the atom's true assignment. The translator emits `type_is`
  only for one-argument `type(x) is str|int|bool` with both names lexically resolved to builtins;
  a shadowed `type` or class stays opaque. `isinstance`, a narrowed Pyrefly type, an annotation or
  a protocol is never an exact origin. **Implemented and Tested in focused cases (2026-09-24).**
- **Builtin namespace assumption.** Lexical resolution cannot prove the runtime builtin namespace
  is unmodified. A `type_is` atom is evaluated only when the request declares the standard
  namespace (`standard_builtins`, `BuiltinNamespace::StandardAssumed`); otherwise it stays
  unknown. The generation does not enforce this assumption.
- **Evaluation.** A query-supplied exact input (`None`, `bool`, `int`, `str`) assigns source atoms
  only through checked links (≤32 assignments, bounded work); it is not a synthetic query atom. It
  evaluates `is_none`, `is_value` of `None`/Booleans, string and non-bool integer `equals`,
  all-string `member_of`, `truthy` and, under the
  namespace assumption, `type_is` for `str`/`int`/`bool`. The bounded BDD refutes only when the
  assigned condition becomes `false`; a satisfiable remainder is "compatible under the model",
  a may-model non-refutation, not a feasible execution. `==` is never treated as `is`, distinct
  numeric and Boolean literals are not assumed unequal (`1 == True`), and a parameter default is
  not an exact entry value when a caller may pass an argument. **Implemented and Tested in
  focused cases (2026-09-24).**
- **Current limit** ([plan W11](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)):
  `True in {1}` remains unknown; literal membership and integer equality alone do not make Q09
  operation-wide. Exact-input assessment uses bounded BDD restriction; a bounded deletion pass
  removes irrelevant refutation links when its work budget permits. Python-lowering controls,
  served round trips and the operation-wide Q09 check remain.
- **Proposed.** Source-to-source exclusions between atoms on one proved-stable value
  (`is_none` against a proved non-`None` singleton; unequal string `==` under an exact `str`
  origin and same-value witness); typed exclusion from Pyrefly terms; custom `__eq__`, subclasses
  and unproved relationships stay unknown. No theory solver: z3 is reconsidered only for a
  registered question the bounded lowering cannot express ([§B10](../DESIGN.md#section-b10)).

## Value flows and call transfers

- **A value inside a call is a transfer, not a flow.** A use inside a call within a value (callee,
  receiver or argument) reaches the value only if the callee's result carries it, which is a
  summary's question. The flow IR marks such a use `through_call`; a path is as weak as its
  weakest step (identity, derived, through a call). A `derives`, `stores`, `returns` or
  `raises_when` claim reached only through a call is `unknown` (`call_transfer`), keeping the
  condition it would hold under. Operators, f-strings and containers are computed from their
  operands directly.
- **Nested call provenance** (ADR-0028, proposed). Each `flow_values` use inside calls has raw,
  ordered outer-to-inner `flow_value_calls` steps citing the parent value fact, byte span and
  callee/argument operand role; `flow_value_call_links` persists the unique Ruff `call_syntax`
  and `arguments` join, or an explicit missing/ambiguous status, for every step.
  `value_flow_contributions` keeps one row per raw fact and source origin before `value_flows`
  merges them, separating the local transfer from the upstream one and the sink callable from the
  parameter's owner. **Implemented and Tested in focused cases (2026-09-25).** Proposed: L3
  withholds a positive transfer if a step is unmatched, a callee is relabelled as an argument or
  the argument value is derived from its use; `through_call` alone never upgrades a verdict.
- **Known defect** ([plan W7](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)):
  `Model::reach` (`flow_model.rs`) is not a fixed point at a cycle head. A query-order
  reproduction loses a transfer variant (Tested), and dense loops have no work bound. The
  intended contract is an SCC fixed point with a work cap writing `budget_reached`. The
  demonstrated impact is missing contributions and seed/boundary coverage; a broader
  negative-premise consequence was not established.

## Verdicts

Codebook `verdict`, append-only. Every behavioral answer carries exactly one, never a null:

| Verdict | Meaning |
|---|---|
| `established` | Derived under the stated model with no boundary in the region the claim reads |
| `conditional` | Established under a stated condition |
| `refuted_under_model` | Only where the claim's premise holds (`negative_premises`, below); a rule rejects any other |
| `unknown` | A boundary intervenes; its `boundary_reason` is named, `budget_reached` included |
| `not_analyzed` | Out of scope or not requested, nothing else |

**Approximation direction.** `established` and `conditional` state **may**-behavior: a derivation
admits the behavior under the stated model without crossing a boundary; they do not prove a
concrete execution exists. The model admits ty's ambiguous terminal, assumes calls return and
computes primitive operators, f-strings and containers directly. An explicit raise under a `try`
or `with` body has no definite escape until L2 proves the frame's action; an unframed raise keeps
its escape witness. A negative verdict needs a complete may-analysis and its explicit premise.
The validation lane checks that observed executions are admitted; a finite pass is evidence, not
proof ([§8.1](validation-and-evaluation.md#section-8-1)).

- **An opaque condition is still stated:** its record is `conditional` and the opaque text is shown.
- **A budget cut is `unknown`** with `budget_reached`.
- Discovery results (FCA, communities, vectors) carry no verdict: they are `statistically_derived`
  nominations and never write membership.

## Negative premises

A negative claim ("never read", "never forwarded") is `refuted_under_model` only where its premise
holds, in one relation `negative_premises`: place key, premise kind, whether it holds, and the
reason if not.

| Place kind | The premise holds when |
|---|---|
| Parameter or local | No reference resolves to the binding, closures included; the module has flow IR; the declaration is runtime-reachable; the body is not abstract, a stub (only `pass`, `...` or a docstring) or raise-only (else `abstract_body`); no release class that inherits the method defines it again (else `override_dispatch`) |
| Field `f` of `C` | Across the release, no attribute load named `f` on **any** receiver, a place or not, and no `getattr`/`hasattr` with the literal name (`flow_attribute_loads`); every release module has flow IR; no dynamic access reaches `C` |
| Module global | No read of the resolved place anywhere in the release; every release module has flow IR; no dynamic access reaches the module |
| Forward chain | The operation's `behavior_status` is `established` |

- The field premise is name-based on purpose: it covers mixins and untyped bases, at the cost that
  a common field name blocks refutation.
- **External readers are outside the model.** "Never read" means never read by release code;
  framework serializers such as pydantic's may read fields the release never names.
- **"Never read" is about this body:** an override a user writes is outside the model; one the
  release writes blocks refutation.
- Rules: `semantic:refuted-needs-complete-region` joins every refutation to its premise;
  `semantic:refuted-not-overridden` rejects a refutation on a method a release subclass redefines;
  `semantic:premise-no-attribute-load` rejects a holding field or setting premise whose name some
  attribute load spells. A constructor parameter's claims attach to the class's `__init__`.
- **Current verification boundary** ([plan W4, W14](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
  Focused real-provider tests now cover bare, qualified and aliased builtin `getattr`/`hasattr`,
  computed names and shadowed builtins. Literal names contribute reads; computed names contribute
  a dynamic boundary before no-read premises. Pyrefly's resolved function status, including
  aliased abstract methods and Protocol placeholders, now drives abstract-body withholding.
  One corrected premise still needs a publication-to-serving trace before W4 closes.

## Boundary reasons the behavior model adds

- `dynamic_access`: the getattr family (`getattr`, `setattr`, `hasattr`, `delattr` with a
  non-literal name), `vars()`, `__dict__`, `importlib.import_module` and `__import__` (module
  objects only), `exec` and `eval` (every place).
  - It reaches class `C`'s fields when the receiver's reaching definitions, through local copies,
    include `self` in a method of `C` or a relative (subclass, base, mixin), or a module global
    bound to an instance of `C`. A value Pyrefly types as `C` or a relative is **Proposed**. An
    `x.__dict__` load resolves the same way. `Settings.get_setting`'s
    `settings = self; getattr(settings, name)` is the first kind, so "this setting is never read"
    is `unknown`.
  - A module `__getattr__` reaches no bound place. Any other receiver is outside the model, and
    every negative answer names that assumption.
- `override_dispatch`: a path through a `candidate` arc (`self.m(...)` a subclass may override) is
  `unknown`, as the delegation over it is.
- `call_transfer`: a claim reached only through a call (above).
- `runtime_unreachable`: an operation whose seed declaration the runtime cannot reach.
- `outside_provider_model`: a site the runtime reaches but Pyrefly pruned (below).
- `abstract_body`: a negative premise on an abstract, stub or raise-only body.

## Read phase

The read phase of a read comes from its site's scope: a module or class body is `import`;
`__init__` or `__post_init__` is `construction`, and a `snapshot` when the value is stored to a
field; anything else is `per_call`. A read reached from module scope through calls is a summary's
question; "when it runs" versus "when called" for coroutines and generators is Stage 5.

## Three vocabularies

- `modality` (definite, candidate, potential) describes a call-graph input fact: a behavior across
  a candidate or potential arc is at best `unknown`.
- `verdict` is a behavioral answer's (above).
- `evidence_status` is a brief assertion's. When a brief renders a behavior, `established` and
  `conditional` become `structurally_observed`, `unknown` becomes `unresolved`, and refuted and
  not-analyzed records are not rendered as assertions.

## Identity

- The flow provider runs inside the extractor, so its identity (`ty_python_core 0.0.14 (ruff
  0.0.14, salsa 0.28.2)`) is part of `producers.revision` and so of `producer_id`. A change to
  `cpg-flow`'s output bumps `EXTRACTOR_OUTPUT_VERSION`, the one counter; `flow_shapes` snapshots
  show an unbumped change. The context already carries the Python version and platform the
  runtime view reads.
- A condition's identity is its structural root id (above); labels and display text are never
  identities.
- A registry concept's id is `H("concept", key)`; its labels and definitions are data, digested
  into the registry digest and so into `compiler_digest`; a membership's id is
  `H(concept, operation, basis)`.

## Composed layers

- The CPG layers keep Pyrefly's checker view ([§B1](../DESIGN.md#section-b1)), as labelled.
- A behavior takes the runtime view at its site: a site the runtime view finds unreachable (inside
  `if TYPE_CHECKING:`) yields no behavior; a site Pyrefly prunes but the runtime reaches (its
  `else`) has no call facts, so a behavior that needs them is `unknown`
  (`outside_provider_model`).
- The exports seed stays the checker view. A declaration is runtime-unreachable when its own
  statement's region, or an enclosing declaration's, is `false`; its operation and every claim
  about it are `unknown` (`runtime_unreachable`), whether or not another runtime definition of
  the name exists, and its parameters' premises do not hold
  (`semantic:unreachable-not-established`). `unreachable_in_context` keeps its meaning: the
  checker never binds it.

> Decision: ADR-0045, ADR-0024, ADR-0028

---
