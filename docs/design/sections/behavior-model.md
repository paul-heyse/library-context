<a id="section-3-9"></a>

# §3.9 Behavior model: places, conditions and verdicts

**Proposed** (ADR-0022, 2026-09-24; revised by the Stage 2 review the same day). **Implemented**
and **Tested** for the provider, the lowering and the encoding (`cpg-flow`, `flow_shapes`,
`cpg_schema::condition`). Source: the behavioral-model review, §3 and §8.2.

**Places.** What a definition or use names:
- a local name;
- `self.f` or `self.f.g` (at most two fields below `self`);
- a module global, settings singletons included (`fastmcp.settings.<field>`);
- a ContextVar object (from Stage 5, with the framework models that read it; the ADR review's F14);
- an attribute of a function object (`fn.__fastmcp__`).

A place is known in two layers (ADR-0022 §Places):
- **spelled**, as written in its scope: a condition's text. Every source test carries its
  evaluation site in its atom identity. Reaching definitions remain separate facts; even the
  same definition set does not prove a value stayed stable across an impure call. Same-line
  bindings remain distinct;
- **resolved**, the join key across scopes: `Parameter[…]`, `Local[…]`, `Field[C.f]` or
  `Global[module.name]`, then at most two segments (§9.9).

Anything deeper, or reached through a computed name, is a boundary: `dynamic_access` for the
getattr family and its kin, `unresolved_target` otherwise. It is never a guess.

**The flow provider** (ADR-0022, D-2).
- **What ty supplies.** `ty_python_core` 0.0.14, in the crate `cpg-flow`, builds the use-def map:
  reaching definitions per use, each with its reachability as a decision diagram over predicates.
  It also gives every statement's reachability, relative to its scope's entry.
- **What crosses the boundary:** byte ranges, place text and our condition data (ADR-0012
  amendment).
- **Normalizations:**
  - loop-header definitions become the body bindings they stand for (`loop_carried`);
  - an import alias's target becomes the bound name;
  - an augmented assignment's target is a use and a definition.
- **Parity, both ways.** Every flow use joins a `references` row and every reference a flow use;
  likewise for definitions and `bindings`. ty's reaching definitions stay within our candidates.
  The residue is names inside annotations, which `references` does not model as reads. Counting
  it per run is **Proposed**, not built (the Stage 2 end review's R10).
- **ty's exception model is part of the stated model.** Ambient exceptions (`KeyboardInterrupt`)
  are outside it.
- **Panics abort the extraction.**

**The runtime view.**
- `TYPE_CHECKING` is false. ty decides it as true at index time, so before ty parses a module every
  `TYPE_CHECKING` **name token** is renamed to a sentinel of the same length. Strings and comments
  keep their text. ty then keeps both branches. Our evaluator decides only a name resolved by our
  lexical facts to `typing.TYPE_CHECKING` or `typing_extensions.TYPE_CHECKING`, aliases included,
  or an attribute on an imported `typing` module. A parameter with that spelling remains ordinary.
- `sys.version_info` comparisons follow full tuple ordering, only when the root resolves to the
  stdlib `sys` module. The context knows its first three numeric fields; a literal tuple of up to
  three fields can be compared because the runtime tuple has two more fields. Longer literals
  remain undecided until release level and serial are modeled.
- `sys.platform` and `os.name` follow its platform (§4.0), only under resolved stdlib roots.
- C3's static-branch marks (§4.2.4) remain the checker view that the CPG layers keep.

**Conditions** are data in a closed language, in disjunctive normal form (ADR-0022 §Conditions).
- **Literals.** A literal is an atom with a polarity. The atoms (codebook `condition_atom`) are
  `is_none(p)`, `is_value(p,v)`, `equals(p,v)`, `member_of(p,{v…})`, `truthy(p)`, `isinstance(p,C)` and
  `opaque("text")`.
- **Literal values:** `None`, `True` and `False`; decimal integers; JSON strings. A test on any
  other literal is opaque.
- **Operators stay distinct:**
  - a single-value set remains `member_of`;
  - `== None` is `equals(p,None)`, distinct from `is_none`;
  - `is True` is `is_value(p,True)`;
  - the literal is the second operand.
- **Encoding.**
  - A translated literal is `[!]kind(place[,literal])#module:source`, where the source is a byte
    site or a synthetic predicate identity. The module key includes path and content. Text labels
    are not identities; tests never share by spelling. A definition-set encoding is reserved for
    a later rule that proves value stability across sites, but none is emitted now.
  - Conjunctions are sorted bytewise, deduplicated and joined by ` & `.
  - Disjunctions are sorted, deduplicated, absorbed and joined by ` | `.
  - `true` and `false` are the empty conjunction and the empty disjunction.
  - The id is `H("condition", encoding)` (DP-04). Equality is syntactic, and that is declared.
- **The lowering from ty's diagrams**, one procedure:
  1. Follow only `if_true` and `if_false`.
  2. Read ty's **ambiguous** terminal as `true`, because verdicts state **may**-behavior.
  3. Map the predicates, evaluating the runtime view first.
  4. Drop contradictions; self-subsuming resolution (`a & !y | y` is `a | y`, and
     `a & r & !y | r & y` is `a & r | r & y`), to a fixpoint; absorb.
- **Opaque text** is the test's source with comments removed and whitespace collapsed.
- **Budget:** at most 16 conjunctions of 8 literals. A larger condition is not stated: its record
  is `unknown` (`budget_reached`).
- **Skipped false branches** are counted in each module's flow coverage detail by ty false,
  resolved runtime-view decisions and stable-atom contradictions. Value-source branch skips are
  counted separately. The generated-program `sys.monitoring` oracle checks every observed line
  and local reaching definition against the flow model. Its developer CLI receives explicit
  resolved-name spans for runtime-view cases; the separate extractor integration test proves
  lexical resolution supplies those spans. Neither check alone is an end-to-end import resolver
  differential.
- **Predicates we do not read as tests:**
  - calls are assumed to return (`NoReturn` is Stage 3's models);
  - ty's non-empty-iterable, context-manager-suppression and finally-path predicates are opaque.
- **Compatibility** of two conditions waits for its first question, Stage 3's Q9 (the Stage 2
  review's F11). Stage 2 has no Python twin (the ADR review's F12).

**Proposed Stage 3 condition kernel (ADR-0024).** The authoritative condition becomes a bounded
decision diagram over per-evaluation atoms, persisted losslessly as nodes in the `flow` family.
Variable order is the sorted atom identity. DNF becomes a capped display with a truncation marker,
not the id input. `given`, `implies` and `compatible` run on the diagram with explicit node and
invocation and pair-work budgets; a hit is `unknown`, not false. A typed exclusion needs
`flow_test_leaves`, emitted per provider predicate and leaf before `flow_tests`' span-only
deduplication. It retains distinct `match` arm atoms at a shared subject span. `flow_test_types`
cites that leaf row: the Rust producer queries Pyrefly's expression trace at the exact operand-use
span, joins it to the flow use by source identity, and records the leaf evaluation atom identity,
`flow_test_leaves.fact_id`/`flow_uses.use_id`, type-term id,
closed proof origin and cited facts. An absent or ambiguous trace proves nothing. The shared
validator checks leaf support in its own root, span, role, type term, origin and facts.
Runtime-exact origins are modeled literals or an exact `type(x) is builtin` guard.
Same-evaluation identity proves stability only; broad Pyrefly annotations alone do not prove
exactness. Different sites stay independent until an
effect-stability witness connects them. Raw Boolean ids do not depend on the theory revision;
theory-conditioned decisions cite a witness and revision. Condition rows name roots in the
lossless node relation. Publication and native load validate terminals, child closure,
acyclicity, atom order, reduction and recomputed Merkle ids. The result records whether ty's
ambiguous terminal or a declared runtime assumption was admitted as `approximated`.

**Implemented and Tested (2026-09-24, direct link only).** `flow_test_value_links` is a separate
analysis relation, not a promotion of `flow_test_types` to exact-runtime proof. Its first origin
requires one stated, non-approximated, non-loop-carried parameter reaching fact, an exact
attributed test operand and no intervening source call, binding, potentially effectful syntax or
prior predicate evaluation. The sole expression-statement exception is the exact recorded
literal docstring span, which has no call-time effect. The effect-rule digest and source fact ids
are stored on each link;
publication recomputes the entire relation from its pinned raw views. A missing link remains
unknown. Exact builtin value/type origins, modeled transfers and typed exclusion are still
Proposed.

**Proposed first typed-theory scope (2026-09-24).** An exact-origin relation must cite either a
query-supplied literal or a resolved one-argument builtin `type(x) is <builtin>` guard, its inner
operand use, branch condition and path-specific same-value witness. A parameter default is not
an exact entry value for calls that may pass an argument. Initially, theory constraints cover
`is_none` against a proved non-`None` singleton and unequal string `==` tests only where the
runtime value is proved exact builtin `str`. Numeric/Boolean cross-equality, subclass equality,
custom `__eq__` and all unproved relationships stay unknown. The existing bounded BDD kernel
conjoins these constraints; a query-supplied exact literal may directly assign source atoms with
checked same-value links rather than create a synthetic query atom. A satisfiable remainder stays
unknown. The raw condition id remains independent of the theory revision.
The trusted builtin `type` call is exempt from the effect barrier only with resolved builtin
identity and a revisioned model. No generic SMT engine is included without a registered need.

**Implemented and Tested (2026-09-24, source attribution only).** The flow translator emits a
`type_is` atom for one-argument `type(x) is str|int|bool` only when lexical resolution identifies
both names as builtins. It attributes the test operand to the inner `x` use; a shadowed `type` or
class name stays opaque.

**Implemented and Tested (2026-09-24, exact origin at the guard only).** The value-link producer
admits its own resolved one-argument builtin `type(x)` call as a revisioned, cited exception to
the call barrier; any other preceding call or effect still withholds the link. The separate
`flow_test_exact_origins` relation cites that link and the guard leaf, and asserts exact builtin
class only under the atom's true assignment. Publication recomputes both relations. This is
not by itself a proof that a later use still observes the same value.

The exact-class rule assumes the standard CPython builtin namespace has not been mutated by
the embedding application or dynamic code. Lexical resolution does not establish that runtime
environmental fact. Until a generation declares and enforces that assumption, a served negative
answer depending on this origin must remain `unknown` (Deferred from the Stage 3.0 review).

**Implemented and Tested (2026-09-24, narrow later-use proof).** A third value-link origin
connects a later test operand to the same entry formal only when the exact guard's predicate
contains that atom alone, the later reaching fact's path condition implies its true assignment,
and no other source call, binding, effect-bearing syntax or prior predicate intervenes. The row
cites the exact origin and reaching path condition. The nested positive fixture publishes one
such link; an unknown call inside the guard withholds it. This remains a positive identity
proof, not a served compatibility verdict. The exact-input primitive evaluator may refute a
condition only when its checked link assignments make the bounded BDD false; a satisfiable
remainder is unknown. Source-source exclusions and the served API remain Proposed.

> Decision: ADR-0022, ADR-0024

**Verdicts** (codebook `verdict`, append-only). Every behavioral answer carries exactly one, never
a null, and a positive answer states **may**-behavior:

| Verdict | Meaning |
|---|---|
| `established` | Derived under the stated model with no boundary in the region the claim reads |
| `conditional` | Established under a stated condition |
| `refuted_under_model` | The claim's premise holds (`negative_premises`, below). A rule rejects it anywhere else |
| `unknown` | A boundary intervenes; its `boundary_reason` is named, `budget_reached` included |
| `not_analyzed` | Out of scope or not requested |

`established` and `conditional` admit may-behavior under the model; they do not prove a concrete
execution exists. AMBIGUOUS is admitted, calls are assumed to return, primitive operators,
f-strings and containers are computed directly. **Implemented and Tested in focused cases
(2026-09-25; ADR-0027):** an explicit raise under a `try` or `with` body cannot establish
escape until L2 proves the frame's action. An unframed raise retains its escape witness;
`raise_sites.escapes = false` means unknown, not caught. A negative needs complete may-analysis
and its premise.

Discovery results (FCA, communities, vectors) carry no verdict: they are `statistically_derived`
nominations.

**An opaque condition is still stated.** Its record is `conditional`, and the opaque text is shown.

**Premises per place kind** (`negative_premises`; ADR-0022 §Verdicts).

| Place kind | The negative claim's premise |
|---|---|
| Parameter or local | No reference resolves to the binding, closures included |
| Field `f` of `C` | No attribute load named `f` on any receiver anywhere in the release; no dynamic access reaches `C` |
| Module global | No read of the resolved place anywhere in the release; no dynamic access reaches the module |
| Forward chain | The operation's `behavior_status` is `established` |

- Every premise also needs flow IR for the modules it reads.
- External readers, such as serializers, are outside the model: "never read" means never read by
  release code.
- The rule `semantic:refuted-needs-complete-region` joins each `refuted_under_model` row to its
  premise.

**Boundary reasons the behavior model adds:**
- `dynamic_access`: the getattr family (a non-literal name), `vars()`, `__dict__`,
  `importlib.import_module` and `__import__` (module objects only), `exec` and `eval` (every
  place).
  - It reaches class `C`'s fields when the receiver's reaching definitions, through local copies,
    include `self` in `C` or a relative, or a module global bound to an instance of `C`. A value
    Pyrefly types as `C` or a relative is **Proposed**, not built (the Stage 2 end review's R10).
    An `x.__dict__` load is resolved the same way.
  - `Settings.get_setting`'s `settings = self; getattr(settings, name)` is the first kind, so "this
    setting is never read" is `unknown` (review §5, journey b).
  - A module `__getattr__` reaches no bound place.
  - An untyped receiver is outside the model, and every negative answer names that assumption.
- `override_dispatch`: a path through a `candidate` arc (`self.m(...)` that a subclass may
  override) is `unknown`, as the delegation over it is.
- `runtime_unreachable`: an operation whose seed declaration the runtime cannot reach.

**The read phase** of a read comes from its site's scope:
- a module or class body is `import`;
- `__init__` or `__post_init__` is `construction`, and a `snapshot` when the value is stored to a
  field;
- anything else is `per_call`.

A read reached from module scope through calls is Stage 3's.

**Three vocabularies** (binding §3).
- `modality` is a call-graph input: a behavior across a candidate or potential arc is at best
  `unknown`.
- `evidence_status` is a brief assertion's. When a brief renders a behavior:
  - `established` and `conditional` become `structurally_observed`;
  - `unknown` becomes `unresolved`;
  - refuted and not-analyzed records are not rendered.

**Identity (F9).**
- The flow provider is part of the extractor's `producers.revision`, and so of `producer_id`.
- A change to `cpg-flow`'s output bumps `EXTRACTOR_OUTPUT_VERSION`.
- A registry concept's id is `H("concept", key)`. Its labels are data, digested into the registry
  digest.

**Composed layers (F11).**
- The CPG layers keep the checker view.
- A behavior takes the runtime view at its site:
  - an unreachable site yields no behavior;
  - a site that Pyrefly prunes but the runtime reaches yields `unknown`
    (`outside_provider_model`).
- The exports seed stays the checker view. An operation whose declaration the runtime cannot reach
  is `unknown` (`runtime_unreachable`).

**Implemented and Tested in focused cases (ADR-0028, 2026-09-25):** each `flow_values` use
inside calls has raw, ordered outer-to-inner `flow_value_calls` steps. A step cites its parent
value fact, byte span and callee/argument operand role; publication checks the path and rejects
a skipped ordinal. A nested keyword path joins the same module's Ruff `call_syntax` and
`arguments` facts by exact spans in a focused fixture. **Implemented and Tested in focused cases
(2026-09-25):** `flow_value_call_links` persists the unique source join or an explicit
missing/ambiguous status for every step. Focused cases cover matched nested keyword and callee
roles, removed source calls, doubled source calls and a doctored status rejected by the shared
validator. **Proposed:** L3 must withhold a positive transfer if a step is unmatched, a callee is
relabelled as an argument, or the argument value is derived from its use. Existing
`through_call` remains an unknown boundary until each modeled hop is proved.

**Implemented and Tested in focused cases (ADR-0032, 2026-09-25):**
`analysis_conditions` and `analysis_condition_nodes` persist the structural
roots and reachable BDD nodes recomposed by the flow analysis. Provider
`conditions` remain source facts; analysis roots have their own authority and
are reconstructed and hydrated by the shared publication validator. Display
DNF is not parsed to answer a condition question. Approximation remains on
path rows because Boolean identity does not encode it. A boundary root
continues to mean `unknown`. L3 must check this catalog before composing any
derived condition.

> Decision: ADR-0022, ADR-0027, ADR-0028, ADR-0032

---
