---
id: ADR-0022
title: The behavior model: a stated runtime flow abstraction, closed conditions, five verdicts, and models as data
status: proposed
date: 2026-09-24
supersedes: []
superseded-by: null
design: [§B5, §B10, §3.2, §3.9, §9, §9.9]
evidence: Proposed
revisit: The structured evaluation finds a question class the closed condition language cannot express; or a ty upgrade breaks range parity or decides a test at index time that differs from runtime.
---

## Context

ADR-0021 makes the behavioral model the product. The deep review of the pivot found four gaps its
semantics must close:
- **F3:** no verdict lattice, so negative answers have no completeness premise.
- **F5:** every ready-made flow provider carries a type checker's view that would be relabelled
  as runtime flow. Pyrefly drops statically decided branches (§B1); `ty_python_core` 0.0.14
  decides `TYPE_CHECKING` as true (review probe).
- **F6:** the pivot's "compatible behavior context" is, in general, satisfiability, and §B10
  excludes a constraint solver.
- **F4:** the channels that carry FastMCP's behavior (`self` fields, the settings singleton,
  ContextVars, function-object attributes, framework registries) are absent from the pivot's
  relation list.

## Options

1. **The simpler alternative: extend Pass B's recognizers** (more predicate shapes, more value
   classes), with no IR. Rejected. On the pilot, 15,364 of 32,075 mapped arguments are computed
   expressions and only 79 of 516 raising `if`s are accepted guards. Recognizers over syntax
   cannot follow values through assignments, fields or exceptions.
2. **Adopt a provider's IR as the model** (Pyrefly's binding graph, or ty's use-def map).
   Rejected as the *model*, and allowed as a *provider*. Both are built for type inference;
   relabelling them breaks §B5.
3. **Our own stated runtime abstraction, whichever provider builds it, with closed conditions, five
   verdicts and models as data.** Chosen.

## Decision

- **The flow IR is our stated runtime abstraction (§B5).** It is reaching definitions over
  **places** with the conditions under which they reach, statement reachability, and value
  sources. `ty_python_core` builds its raw material (§The flow provider). Pyrefly's binding graph
  is a parity oracle only.
- **Conditions are data in a closed language** (§Conditions). There is no Python twin (the first
  standard review's F12), and no serve-time condition filter without its own ADR. Compatibility of
  two conditions waits for its first question, Stage 3's Q9 (the Stage 2 review's F11).
- **Five verdicts, never a null.** Codebook `verdict`, append-only:

  | Verdict | When |
  |---|---|
  | `established` | No boundary in the region the claim reads |
  | `conditional` | Established under a stated condition |
  | `refuted_under_model` | **Only** where the claim's premise (§Verdicts) holds; a rule rejects any other |
  | `unknown` | A `boundary_reason` is named, `budget_reached` included |
  | `not_analyzed` | Out of scope or not requested |

  **Verdicts state may-behavior**: a positive record says the behavior can happen on some
  execution the model admits, under its condition. Discovery results (FCA, communities, vectors)
  carry no verdict. They are `statistically_derived` nominations and never write membership.
- **Meaning comes from models, propagation from summaries.**
  - Effects and roles of stdlib, dependency and framework callables are committed data
    (`crates/cpg-schema/models/*.toml`, origin `synthetic_model`, append-only ids), digested into
    `compiler_digest`.
  - Transfer summaries carry them bottom-up over the call graph's SCCs, with argument
    substitution, conditions and exception handling. A call alone never propagates a capability.
- **Materialized at compile time.** Every predicate, summary and concept membership is computed in
  the compile, in DataFusion or in `lctx-analytics` kernels, before publication. The server never
  re-implements a semantic decision.
- **Decided at Stage 2.1.** The first standard review's F5, F6, F9 and F11 are decided here, after
  the provider spike (deviation log B11). They were then revised by the Stage 2 review
  (`design_review_behavior-semantics-stage2_2026-09-24.md`, F1–F11).

### The flow provider (D-2)

- **`ty_python_core` 0.0.14** builds the IR's raw material in one crate, `cpg-flow`:
  - the use-def map: reaching definitions per use, each binding's reachability a decision diagram
    over predicates;
  - every statement's reachability.

  Its pins are ruff 0.0.14 and salsa exactly 0.28.2, a declared extra family (ADR-0002
  amendment, `check_family.py`).
- **What crosses the boundary:** byte ranges, place text and our condition data. No ruff 0.0.14
  or ty type leaves `cpg-flow` (ADR-0012 amendment).
- **Normalized before use:**
  - ty's synthetic loop-header definitions become the loop-body bindings they stand for, marked
    `loop_carried`;
  - an import alias's target becomes the bound name, which is where our bindings put it;
  - an augmented assignment's target is both a use and a definition.
- **The runtime override** (the Stage 2 review's F7). ty's builder decides `TYPE_CHECKING` as true
  at index time.
  - Before ty parses a module, every **name token** `TYPE_CHECKING` is renamed to a sentinel of the
    same length, `TYPE_CHECKIN_`. The tokens come from ruff 0.0.14's lexer over the module, so
    strings and comments keep their text; a name in an f-string replacement field is renamed.
  - A module that already uses the sentinel as a name is refused, and its flow coverage is
    `failed`.
  - The runtime view matches the sentinel as a name and as an attribute of a dotted name.
  - Place text, literal values from our text, and opaque text are cut from the module **as
    written**, by range.
  - `import TYPE_CHECKING as TC` leaves `TC` a `truthy` atom, as declared.
  - ty decides no other test at index time that differs from runtime. Its literal folding covers
    `True`, `False`, `None`, integers, `...`, lambdas, generators and `not`.
- **ty's exception model** is part of the stated model. Exceptions come only from operations ty's
  model says can raise. A `try` suite whose statements cannot raise, in its model, has unreachable
  handlers. Ambient exceptions (`KeyboardInterrupt`, `MemoryError`) are outside it.
- **Regions are relative to their scope's entry.** A statement inside a function is reached under
  its region's condition **and** the `def` statement's region in the enclosing scope.
- **Panics abort the extraction**, as for every analyzer (ADR-0012 §Panics). The rule
  `no-catch-unwind-in-extractor` covers `cpg-flow`.
- **Parity, both ways** (the Stage 2 review's F5). The rules:
  - every flow use joins a `references` row by module and range, and every reference has a flow
    use;
  - every flow definition of a name joins a `bindings` row, and every binding has a flow
    definition;
  - `semantic:flow-reaching-within-candidates`: ty's reaching definitions of a use lie within our
    candidate bindings for it (the spike's exit test, kept).

  The declared residue (Stage 2.3, measured on the pilot and the fixtures):
  - names inside annotations and PEP 695 type-alias values, which ty indexes and `references`
    does not model as reads (§3.2); they are flow uses marked `annotation`;
  - bindings ty never defines as such: `global` and `nonlocal` declarations, `del` (an unbinding
    in both models: a `del` target is no flow use either), implicit names, and a star import
    (ours is the `*`, ty's are the names it brings in);
  - a class's or an alias's type parameters, which our lexical recognizer does not model, so the
    flow IR records no definition for them.
- **The spike's measurements** on FastMCP 4.0.5's 275 release modules (2026-09-24, *Measured*):
  - 213 ms and 47 MiB peak;
  - every one of our 39,986 references is a ty use, once augmented-assignment targets are read as
    uses;
  - ty's reaching definitions fall within our candidate bindings for 27,782 uses (2,319 strictly
    narrower);
  - 251 fall outside, all from the two focus conventions normalized above;
  - 11,917 uses have no local definition (builtins or enclosing scopes).

  `cpg-flow`'s tests pin the fixture's conditions for `try`, `with`, `for`, `match`, `elif` and
  the static branches (`flow_shapes`).

### Conditions (F5; the Stage 2 review's F2 and F10)

- **Normal form: DNF.** A condition is a disjunction of conjunctions of **literals**. A literal is
  an atom with a polarity: `is not None` is `!is_none`.
- **Atom kinds** (codebook `condition_atom`, append-only):
  - `is_none(p)`;
  - `equals(p, v)`;
  - `member_of(p, {v…})`;
  - `truthy(p)`;
  - `isinstance(p, C)`, with `C` as written;
  - `opaque("text")`, for any other test.
- **Places in conditions** are written as spelled in the record's scope: a root name, then at most
  two attribute segments, with no subscript. The record carries its scope. The **join key** for
  places across scopes is the resolved place (§Places).
- **Literals:**
  - `None`, `True` and `False` as written;
  - integers in decimal, a negative one included;
  - strings as JSON strings, serialized by serde_json (non-ASCII characters kept as UTF-8).

  A test on any other literal is opaque.
- **Canonical forms:**
  - a single-value `member_of` is `equals`;
  - `== None` is `is_none`;
  - `is True` and `is False` are `equals` with `True` and `False`;
  - the literal is always the second operand;
  - `in` takes a tuple, list or set of literals on the right, and anything else is opaque.
- **Encoding.**
  - A literal is `[!]kind(place[,value])`.
  - A `member_of` set is sorted and deduplicated.
  - A conjunction is its literals sorted bytewise by encoding and deduplicated, joined by ` & `.
  - A disjunction is its conjunctions sorted and deduplicated, joined by ` | `.
  - `true` is the empty conjunction; `false` is the empty disjunction.
  - The id is `H("condition", encoding)` (DM-15). Equality is syntactic, and that is declared.
- **Opaque text** is the test's source by its range (parentheses outside the range excluded), with
  comments removed and runs of whitespace collapsed to one space.
- **The lowering from ty's diagrams**, one procedure:
  1. Enumerate the diagram's paths to the true terminal, following only `if_true` and `if_false`,
     because every atom is two-valued at runtime.
  2. ty's **ambiguous** terminal (reachability it leaves undecided: a `try` body, a loop over an
     unknown iterable, a `with` exit) reads as `true`, because verdicts state may-behavior.
  3. Map each predicate to a condition, evaluating the runtime view first, and expanding `and`,
     `or`, `not` and conditional expressions in tests.
  4. Simplify:
     - drop a conjunction holding a literal and its negation;
     - `x | !x` is `true`;
     - self-subsuming resolution, to a fixpoint: drop `!y` from a conjunction when another
       conjunction holds `y` and otherwise only literals the first holds too
       (`a & r & !y | r & y` is `a & r | r & y`; `x | !x & r` is `x | r` is the case with nothing
       else). Stage 2's evaluation added it (deviation B15): one normal path met in two
       differently nested forms must normalize to one encoding before `Condition::given` can
       factor it out;
     - absorb (drop a conjunction that contains another).
- **Budget:** at most 16 conjunctions of at most 8 literals. A larger condition is not stated, and
  its record is `unknown` with `budget_reached`. ty's own saturation (512K diagram nodes per scope)
  is not observable through its API. The pilot's largest scope is reported per run.
- **Fates are stated on the normal path** (Stage 2.6). A statement past a guard that raises
  (`if x is None: raise …`) is reached under the guard's negation, which says only that no error
  was raised. Each fate's condition has every such factor removed where it is one
  (`Condition::given`: all conjunctions share it and the remainders agree); the guard itself is
  its own `raises_when` fate.
- **The runtime view** evaluates, before normalization:
  - `TYPE_CHECKING` (the sentinel, as a name or an attribute of a dotted name) as false;
  - `sys.version_info` comparisons against the context's Python version;
  - `sys.platform` (`==`, `!=`, `startswith`) and `os.name` against its platform.
- **ty predicates we do not read as tests:**
  - a call's `IsNonTerminalCall` is true: calls are assumed to return, and `NoReturn` callables
    are Stage 3's models;
  - `IsNonEmptyIterable` (a `for` over `range(...)`), `ContextManagerSuppresses` and
    `FinallyNormalPathImpossible` become opaque atoms with fixed text;
  - or-pattern alternatives, subject-element patterns and star-import placeholders are opaque
    with the fixed text `<undecided by the flow provider>`.

### Places (the Stage 2 review's F4)

- **Two layers.**
  - The **spelled** place is a condition's text: syntactic, and scope-relative.
  - The **resolved** place is the join key for `value_flows`, field reads, premises and summaries.
    Its root is resolved through our reference resolutions or the flow IR's reaching definitions:
    - to a parameter or local binding (`Parameter[name]`, `Local[name]` of a declaration);
    - to a class's field (`Field[C.f]`, where `C` is the class whose method binds `self`);
    - to a module global (`Global[module.name]`).

    At most two segments follow the root.
- **A spelled place names a value, so a rebound place is versioned** (Stage 2's evaluation,
  deviation B15). A place bound more than once in its scope can hold two values at two tests on one
  path (`if x is None: x = d`, then `if x is None:`); as one atom, the two tests would contradict
  and a feasible path would be dropped. A test's value is versioned by the line of the latest
  binding other than a parameter reaching it (none: the value the scope received; ty's
  loop-header bindings do not count as a second binding). Where one scope's tests of a place read
  more than one version, each versioned test is spelled `place@line`; otherwise all of them are
  spelled plainly, so versions appear only where they disambiguate. Two tests with the same
  spelling on one path read one value, except around a loop's back edge, where ty's diagrams do
  not unroll either. A versioned place still names its root for matching a parameter (the value
  may be the parameter's).
- **§9.9's `Parameter[…]`, `Field[…]` and `Global[…]` are this key's written form.** A singleton
  read as `settings.X` in one module and as `fastmcp.settings.X` in another is one place,
  `Global[fastmcp.settings].X`.

### Verdicts (F6; the Stage 2 review's F1, F3, F8, F9)

- **An opaque condition is still stated.** A record under a condition with an opaque atom is
  `conditional`, and the atom's source text is shown.
- **A budget cut is `unknown`**, with `budget_reached`. `not_analyzed` means out of scope or not
  requested, and nothing else.
- **Premises per place kind.** A negative claim ("never read", "never forwarded") is
  `refuted_under_model` only where its premise holds, in one relation `negative_premises`: the
  place key, the premise kind, whether it holds, and the reason if not.

  | Place kind | The premise holds when |
  |---|---|
  | A parameter or local | No reference resolves to the binding, closures included; the module has flow IR |
  | A field `f` of `C` | Across the release, no attribute load named `f` on **any** receiver; every release module has flow IR; no dynamic access reaches `C` |
  | A module global | No read of the resolved place anywhere in the release; every release module has flow IR; no dynamic access reaches the module |
  | A forward chain | The operation's `behavior_status` is `established` (the scan met no boundary in its region) |

  - The field premise is name-based on purpose: it covers mixins and bases without types. Its
    precision cost is that a common field name blocks refutation.
  - **External readers are outside the model.** "Never read" means never read by release code.
    Framework serializers, such as pydantic's, may read fields the release never names.
  - The rule `semantic:refuted-needs-complete-region` joins every `refuted_under_model` row to its
    premise.
  - A constructor parameter's claims attach to the class's `__init__` operation.
- **Dynamic access** gets `boundary_reason` `dynamic_access`.
  - **The getattr family** (`getattr`, `setattr`, `hasattr`, `delattr` with a non-literal name,
    `vars()`, `__dict__`) reaches class `C`'s fields when its receiver's reaching definitions,
    through local copies, include any of these:
    - `self` in a method of `C`, a subclass, a base or a mixin;
    - a module global bound to an instance of `C`;
    - a value Pyrefly types as `C`, a subclass, a base or a mixin.

    `settings = self; getattr(settings, name)` is the first kind.
  - **Any other receiver** is outside the model, and every negative answer names that assumption.
  - `importlib.import_module` and `__import__` with a non-literal name reach module objects only.
  - A module-level `__getattr__` supplies names the module does not bind, so it reaches no bound
    place.
  - `exec` and `eval` reach every place.
- **A value inside a call is a transfer, not a flow.** A use inside a call within a value (the
  callee, its receiver or an argument) reaches the value only if the callee's result carries it,
  which is a summary's question (Stage 3). The flow IR marks such a use `through_call`, a path is
  as weak as its weakest step (identity, derived, through a call), and a `derives`, `stores`,
  `returns` or `raises_when` claim reached only through a call is `unknown` with `boundary_reason`
  `call_transfer` (appended), keeping the condition it would hold under. The forward into the
  callee's formal stays its own claim. An operator, an f-string or a container is computed from
  its operands directly. Stage 2's evaluation found this (deviation B15): `Client(name=…)` was
  served as established in a helper's result that only logs it.
- **Override dispatch** gets `boundary_reason` `override_dispatch`: a call through a `candidate`
  arc (`self.m(...)`, which a subclass may override). A behavior whose path crosses such an arc is
  `unknown`, as the delegation over it is (increment 3's deep review, F1).
- **The read phase** of a read is decided by the reading site's scope:
  - a module or class body is `import`;
  - `__init__` or `__post_init__` is `construction`, and a `snapshot` when the value is stored to
    a field;
  - anything else is `per_call`.

  A read reached from module scope through calls is Stage 3's (summaries).
- **Verdicts, modality and evidence status are three vocabularies** (ADDENDUM §3):
  - `modality` (definite, candidate, potential) describes a call-graph input fact. A behavior that
    crosses a candidate or potential arc is at best `unknown`.
  - `evidence_status` is a brief assertion's. When a brief renders a behavior (F7, Stage 2.6):
    - `established` and `conditional` become `structurally_observed`;
    - `unknown` becomes `unresolved`;
    - `refuted_under_model` and `not_analyzed` are not rendered as assertions.

### Identity (F9)

- **Provider identity.** The flow provider runs inside the extractor, so its identity is part of
  the extractor's `producers.revision`: `ty_python_core 0.0.14 (ruff 0.0.14, salsa 0.28.2)`.
  - A change to `cpg-flow`'s output bumps `EXTRACTOR_OUTPUT_VERSION`, which is the one counter
    (the Stage 2 review's O5). `flow_shapes`' facts are pinned by a snapshot, so an unbumped change
    shows.
  - The context is unchanged: `contexts` already carries the Python version and platform that the
    runtime view reads.
- **Registry labels are data, never identity.** A concept's id is `H("concept", key)`, where the key
  is its stable key. Labels and definitions are digested into the registry digest and so into
  `compiler_digest`. A membership's id is `H(concept, operation, basis)`.

### Composed layers (F11; the Stage 2 review's F9)

- **The CPG layers keep Pyrefly's checker view** (§B1), as labelled.
- **A behavior takes the runtime view at its site.**
  - A site that the runtime view finds unreachable yields no behavior, for example inside
    `if TYPE_CHECKING:`.
  - A site in code Pyrefly prunes but the runtime reaches (the `else` of `if TYPE_CHECKING:`) has
    no call facts. A behavior that needs them is `unknown` with `outside_provider_model`.
- **An operation whose seed declaration the runtime cannot reach** is `unknown`, with the reason
  `runtime_unreachable` (appended). That holds whether or not a runtime definition of the name
  exists. `unreachable_in_context` keeps its meaning: the checker never binds it.

## Consequences

- **Negative answers become possible, but narrower:** "never read" is `unknown` wherever a dynamic
  access (`getattr` by string, `importlib`) could reach the value.
- **More to maintain:** a flow family, a verdict codebook, an effect codebook, models files, and
  two fixtures (`flow_shapes`, `behavior_shapes`).
- **D-2 chose `ty`:** a second ruff line (0.0.14) enters the lockfile under a declared family
  exception (the ADR-0002 amendment), confined to `cpg-flow`, with salsa pinned exactly at 0.28.2.
  A ty upgrade is a pin change with a parity rerun; the TYPE_CHECKING rename is ours to keep.
- **Revisit:** when the structured evaluation needs a condition the closed language cannot
  express, or when a ty upgrade breaks two-way parity or decides a test at index time.
