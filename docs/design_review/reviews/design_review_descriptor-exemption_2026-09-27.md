# Design review: builtin descriptor exemption from decorator withholding

**2026-09-27 · change/conformance · compact.** Core standard 3.0, code-intelligence profile 1.1
and the library-context binding. Reviewer: fresh `design-reviewer` subagent (not the author).
Subject: commit `8a7c936` (clean tree), "exempt builtin binding-preserving descriptors from
decorator withholding (compiler 103)". It narrows
[channel-contracts F06](design_review_stage3-channel-contracts_2026-09-26.md#F06), which withholds
every behavior of a decorated operation, condition scope or hop "until an identity-preserving
decorator model exists". Design owner: [behavior model, "Value flows and call transfers"](../../design/sections/behavior-model.md#value-flows-and-call-transfers).
Plan row: forward plan §3 P0.7.

## 1. Scope, ownership and method

| Concept | Authority after the change | Consumers |
|---|---|---|
| "A decorator may replace the binding the body describes" | `flow_model::decorated_functions` (`crates/cpg-core/src/flow_model.rs:318`) → `FlowModelRows.decorated` | Operation boundary (`behavior.rs:379`), identity-store composition (`behavior.rs:1111`), row admission for operation, condition scope and every hop (`behavior.rs:1291`) |
| The same question, for no-read premises | `not_behavior` (`flow_model.rs:1628`, any `Decorator` row) — **unchanged** | Parameter premises (`flow_model.rs:2643`) → `is_read` rows (`behavior.rs:1072`) |
| Lexical evidence | C3 resolver (`cpg-extract/src/lexical.rs`) via the `roots` relation. `builtin_name` is set only when no scope binds the name (`tables.rs:784`) | `decorated_functions` |
| Provider evidence | Pyrefly's special-decorator flags (`pyrefly/lib/alt/function.rs:1024–1090`, pinned `a07b7ba`) via `cpg_schema::flows::descriptor_functions_sql` (`flows.rs:470`) | `decorated_functions` |

**Examined:** the diff; the relations `bodies`, `roots` and `descriptors`, plus `not_behavior`;
receivers (`receivers_sql`, field accesses, dynamic-access narrowing); Pass B arcs and the
argument→formal mapping (`flows.rs:80`, `:151`); Pyrefly `has_implicit_receiver`
(`report/pysa/call_graph.rs:1278`); the lexical `resolve_ref`/`outside`; the fixture, test and
design text. **Read-only measurements:** these ran against the checkpoint store
`build/store-stage3-checkpoint-2026-09-27`, snapshot `8b4fb9ab6dcaa3698c27018593553129`,
compiled by compiler 102. Compiler 103 does not change the syntax, lexical, Pysa, provider-map
or premise inputs, so the measurements transfer. **Not examined:** Python serving and brief
rendering beyond `synth.rs` `Form::Property`.

## 2. Semantic soundness per claim kind (question 1)

| Claim path | What happens for an exempt descriptor | Verdict |
|---|---|---|
| Operation parameters | `receivers_sql` drops the first positional parameter unless Pysa says static. So `cls` and a getter's `self` are receivers, and a staticmethod has none. `seed_parameters` excludes receivers | Sound |
| Pass B hops into a descriptor | The formal offset comes from Pysa's per-site `implicit_receiver`. It is `False` for a staticmethod, and `TrueWith{Class,Object}Receiver` (offset 1) for a classmethod called on a class or an instance | Sound |
| Property getter | Arcs admit only `call`/`init` phases, so a getter is never a Pass B callee. It is an operation only when it owns the public path. With a setter, the setter node owns it: pilot `FastMCP.instructions` and `…SessionManager.event_store` are setter nodes and stay withheld. `Form::Property` renders the getter as a read | Sound |
| Identity-store composition (`:1111`) | A classmethod's `cls.f = v` is `receiver_self` and composes with relatives' reads. The class attribute is visible to instance reads unless shadowed, which is the same may-model as `self.f = v` | Sound within the stated model |
| Field/singleton negatives, `getattr(cls, …)` narrowing | These are not gated by `decorated`. Narrowing to the class blocks negatives (conservative) | Unchanged |
| `is_read` refutation | Still withheld by `not_behavior` | **F01** |
| Definitions outside a class body | Neither conjunct requires a class | **F02** |

## 3. Agreement rule (question 2)

- **Independence holds.** One side is our scope-rule resolver over Ruff syntax. The other is
  Pyrefly's type-based decorator recognition. Across the pilot, all 175 sole bare-name builtin
  decorators agree (57 `classmethod`, 74 `property`, 44 `staticmethod`), with 0 disagreements
  (Measured, §6). The conjunction is load-bearing. Pyrefly copies the property's metadata onto
  a `@x.deleter` (`function.rs:1085`), so a deleter after a getter-only property reports
  `is_property_getter`. Only the lexical `ExprAttribute` test rejects it (Interface-checked).
- **Shadowing and conditional bindings.** A class-local binding before the decorator gives an
  `Assignment` candidate and is withheld (tested). A class-local binding after it gives both
  candidates (the `LOAD_NAME` rule) and is withheld. A module-level (conditional) rebinding
  gives a module candidate and is withheld (untested). An unknown star import gives an `Import`
  candidate and is withheld. `from builtins import classmethod` is withheld too, which is
  conservative. Each of these fails `all(Implicit ∧ same builtin)`.
- **`.pyi`/`.py`.** Each file has its own `module_node_id`, and `provider_node_map` is per file.
  The pilot has one Pysa row per node (13,950/13,950). `all()` over the rows, with empty meaning
  withheld, stays conservative if duplicates ever appear.
- **Overloads and `abstractmethod`.** An `@overload` signature carries its own decorator and is
  withheld. An implementation with a sole `@classmethod` is exempt, which is correct because its
  body runs. `@classmethod @abstractmethod` is withheld by the stack rule, and Pyrefly's
  `is_abstract_method` also routes the premise to `abstract_body`. A concrete default body that
  is reached through `super()` stays unknown. This is acceptable; override dispatch dominates
  there anyway.

## 4. Extension scenario (question 3): add `typing.override`

`override` preserves identity and is usually stacked (`@override @classmethod`,
`@property @override`). The route is:

1. Owner: `decorated_functions`.
2. Accept an import resolution. `roots` already carries `imported_module`/`imported_name`, and
   re-export chains exist through `resolve_dotted`.
3. Provider agreement: Pyrefly sets `is_override`, but `pysa_functions` does not persist it.
   Either append the column (a schema migration) or read `function_implementations`, which
   already holds Pyrefly's resolved flags (W14/F10).
4. Replace the `let [row]` sole-decorator guard with per-decorator classification. The stack
   rule becomes: every decorator is identity-preserving or a builtin descriptor, with at most
   one descriptor.
5. The three behavior consumers read the set and need no change.

The change localizes to one pure function and one provider column, except that `not_behavior`
needs a parallel edit and has no control (F01). `abc.abstractmethod` is already owned by
Pyrefly's `is_abstract_method` (`abstract_body`). `functools.wraps` marks the wrapper that an
outer decorator substitutes, so it belongs to the general decorator model (F06), not to this
exemption.

## 5. Findings

| ID | Finding and consequence | Principles · gate | Evidence | Correction and owner | Closure evidence |
|---|---|---|---|---|---|
| <a id="F01"></a>**F01** · Medium | **Two classifiers of one decorator decision now disagree.** `decorated_functions` exempts builtin descriptors, but `not_behavior` still treats any `Decorator` row as "a decorator may replace the callable binding". For `@staticmethod def f(value, unused)`, the operation is Established. Yet `is_read(unused)` is `unknown`/`outside_provider_model` with that stale reason, never `refuted_under_model`, which contradicts the DESIGN sentence "is not treated as decorated". Today this only withholds: 2 pilot premise rows, both on the non-operation `InheritSecurity.__get_pydantic_core_schema__`. But every extension (§4) must edit both sites, and only one has controls | FP-04, DP-01 · A2 · G1 | `flow_model.rs:1628–1633` vs `:318`; `:2643`; DESIGN behavior-model "Negative premises" lists no decorator condition | Compute `decorated_functions` once, before `not_behavior`. Make `not_behavior` test `decorated.contains(&f)` and drop its syntax scan. Owner: `cpg-core::flow_model` (a few lines) | A `transferpkg` control: `@staticmethod`/`@classmethod` with an unread non-receiver parameter gives `is_read` `refuted_under_model`, and the `stacked` variant stays `outside_provider_model` |
| <a id="F02"></a>**F02** · Low | **Class context is not required.** The rule rests on "these descriptors run the function's own body". That holds only for a class attribute accessed through the descriptor protocol. A module- or function-level sole `@classmethod`/`@property` binds a non-callable object (`TypeError` on call), yet it is exempt and its body's claims are served as the operation's. Pyrefly sets the flags outside a class: pilot `json_schema_type._create_dataclass._apply_defaults` has `is_classmethod` with `defining_class` NULL (currently stacked, so withheld). There is no pilot instance of the unsafe shape | DP-08 · G2, CI-G1 (latent) | `flows.rs:470` has no class condition; `decorated_functions` ignores `FunctionRow.parent_kind` | Require the declaration's parent to be a class: `parent_kind == Class` from the already-fetched `function_rows`, or `f.defining_class IS NOT NULL` in `descriptor_functions_sql`. `staticmethod` can share the guard, which is conservative. Owner: `cpg-core::flow_model` / `cpg-schema::flows` | A module-level `@classmethod` fixture function stays `outside_provider_model` |
| <a id="F03"></a>**F03** · Low | **Controls exercise only the lexical conjunct, and the label over-covers.** The negatives are `stacked` (count ≠ 1) and a class-local `Shadowed` (non-`Implicit`). Nothing exercises a Pysa-side rejection, a module-scope shadow or conditional rebinding, a property with a setter/deleter, or an unread parameter (F01). A regression that dropped the Pysa conjunct would pass every test. DESIGN labels the whole paragraph "focused Tested" although its disagreement, setter/deleter and conditional-binding sentences are only Implemented | DP-23, DP-22 · G7 (label) | `compile.rs:223–292`; fixture `transferpkg/__init__.py:148–172` | `decorated_functions` is pure over three row slices. Add a store-free unit test with hand-built rows covering: missing, false and mixed Pysa rows; a builtin-name/flag mismatch (`property` name, `is_classmethod` flag); a non-`Implicit` root; mixed resolutions. Add `transferpkg` cases: module-level `classmethod = replace`, a getter + `@x.setter` + `@x.deleter` property, and the F01 control. Then relabel the untested sentences Implemented. Owner: `cpg-core` tests; behavior-model section | The unit test and fixture cases pass; the DESIGN label is scoped to them |

**Observations (no action).** The admission consumers read one set, so A1 holds for them.
Replacing F06's any-syntax test with provider-agreed resolution is the right direction and
needs no ADR. It narrows an accepted withholding within the same boundary reason. Compiler
output 103, the flow-model digest (which includes the new relation's SQL) and the moved guard
give correct reuse invalidation.

## 6. Gates, verification and library fit

| Gate | Verdict | Evidence |
|---|---|---|
| G1 Authority | **fail** | F01: two independently edited classifiers disagree about one fact |
| G2 Semantic fidelity | **fail (latent)** | F02. Within class bodies, the §2 claim paths are sound |
| G3 Validity | pass | No new state reaches an operation that assumes validity. Empty provider evidence means withheld |
| G4 Hidden behavior | pass | Pure SQL and a pure function. The digest covers the new relation |
| G5 Consistency | n/a | No publication or lifecycle change |
| G6 Transformation and reuse | pass | Compiler output version 103; guard moved for the new query |
| G7 Truthful claims | pass (label overreach under F03) | An implementation route exists for every sentence; see F03 for the Tested scope |
| G8 Library leverage | pass | Reuses Pyrefly's resolved decorator flags and our resolver. No bespoke decorator semantics. Pysa-only would admit deleters; lexical-only would lose type-aware resolution |
| CI-G1 / CI-G2 / CI-G3 | fail-latent (F02) / n/a / pass | F01 keeps unknown distinct from absent. The gold is untouched: the fixture is analysis input, and the change corrects a model rather than tuning to Q01 |

| Check | Outcome | Command / receipt |
|---|---|---|
| Lexical/Pysa agreement over sole bare-name decorators | passed (query ran): 175 agree, 0 disagree, all in class bodies | `target/release/lctx query --store build/store-stage3-checkpoint-2026-09-27 --snapshot 8b4fb9ab6dcaa3698c27018593553129 "<the §3 reproduction over syntax_nodes field=22 kind=55 ⋈ references ⋈ reference_resolutions ⋈ provider_node_map ⋈ pysa_functions>"` |
| Pysa rows per mapped node; classless descriptor flags | passed (query ran): 13,950/13,950; one `is_classmethod` with NULL `defining_class` | same store, `descriptor_functions_sql` body with counts |
| Premise rows withheld by `not_behavior` on descriptors | passed (query ran): 2 non-receiver rows, 0 on operations | same store, `negative_premises ⋈ parameter_syntax ⋈ pysa_functions WHERE boundary_reason = 10` |
| Author's focused checks | historical receipt, 2026-09-27 (commit message): transferpkg positives and stacked/shadowed withholding passed; cpg-core and cpg-schema suites passed after guard updates; pytest, fmt, clippy, rules-scan and fixtures-check passed | not re-run by this reviewer |
| Focused test re-run | not_run | A run cannot settle the missing cases (F03) |
| `just pilot` at compiler 103 | not_run | Interim slice (AGENTS.md timing) |

## 7. Judgment and decision

| Judgment | Verdict | Scope |
|---|---|---|
| A1 Localize change | satisfied | One pure owner over three declared relations. Consumers read the set. Testable with hand-built rows |
| A2 Encode meaning structurally | **violated** | F01: the decision has a second, divergent authority |
| A3 Extend through composition | satisfied for the builtin set | §4: `override` composes existing provider evidence plus one column and a local stack rule, once F01 is fixed |

**Bounded change decision: Revise (small).** The admission semantics are accepted as sound for
the three builtin descriptors in class bodies, including receiver binding, the hop formal
offset, getter phase and store composition. Revision means correcting F01 (one classifier) and
F02 (class guard), with F03's controls as closure evidence. The exemption itself needs no
rollback.

**Enclosing architecture: not certified.** General decorator interpretation remains withheld
under F06. Stage 3 qualification and the compiler-103 pilot delta are unresolved.

| Finding | Disposition |
|---|---|
| F01, F02, F03 | **Open, unscheduled.** Owned here until forward plan §6 (or a P0.7 follow-up row) takes them; they should close before the P1 S5a discharge work extends decorator-adjacent admission |

## 8. Author follow-up (2026-09-27)

F01–F03 are corrected in the follow-up commit to `8a7c936`:

- **F01:** `flow_model::run` computes `decorated_functions` once. `not_behavior` and
  `FlowModelRows.decorated` both read that set. The control `transferpkg.Ignoring.ignores`
  asserts that no row carries the decorator boundary.
- **F02:** `descriptor_functions_sql` requires `defining_class IS NOT NULL`, so a module-level
  `@classmethod` (`transferpkg.loose`) stays withheld.
- **F03:** the store-free `decorator_exemption_tests` cover each refusal independently:
  lexical-only, provider disagreement, class-local shadow, mixed resolutions, stacked
  decorators, attribute decorator and disagreeing provider rows. `transferpkg.Settable.level`
  (getter/setter/deleter) stays withheld. The DESIGN label now names the covered cases.

Receipt: `INSTA_UPDATE=no cargo nextest run --release -p cpg-core -p cpg-schema` **passed**
(236 tests, including the moved all-techniques guard). `just pilot` at compiler 103 remains
`not_run` (interim slice). These closures carry no forward-plan §6 row.
