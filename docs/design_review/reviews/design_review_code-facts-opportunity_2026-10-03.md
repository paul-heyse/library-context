# Design review: code-facts opportunity (pyrefly, ruff and ty facts for the analysis pipeline)

**Design / target review, 2026-10-03.** Standard: core 3.2 principles and template, code-intelligence
profile 1.3, [library-context binding](../design_principles/binding/library-context.md). Baseline
`main` at `6885dbf6`, clean tree. Read-only static review; no build, test, pilot or probe was run.

## Integrated assessment

library-context turns a pinned Python library into provider-attributed facts. Model-owned
normalization, behavioral analysis, analytics and a catalog/selection/retrieval product consume
those facts. The question here is which further pyrefly, ruff and ty facts should enter that
pipeline, and how each downstream layer should use them. Two benefits count: less bespoke analysis,
and wider, deeper or better-assured characterization. Two premises are fixed: the pyrefly 1.4.0-dev.3
/ ruff 0.0.14 migration happens (so pyrefly and `cpg-flow`'s ty 0.0.14 share one parser line), and
feasibility is out of scope.

**The most valuable additions have consumers waiting for them.** Accepted targets already describe
the questions, but nothing extracts the facts that would answer them. The plainest case is §14.4,
which asks the catalog to preserve "source, effective, synthesized, documented and observed"
signature roles "with types". The only signature fact is Pysa's *undecorated* list
(`crates/cpg-extract/src/type_records.rs:1042-1044` reads only `f.undecorated`). So
`callable_normalization.rs:536-612` leaves every callable effective-Unknown when it carries more
than one decorator, carries one decorator other than a bare builtin descriptor, or has a
synthesized origin. Selection's `MemberKind`, `InvocationForm` and `ParameterType` then stay
unresolved for that member. Class metadata is similar. Pyrefly computes finality, enum members,
protocol and abstract status, metaclass, `dataclass_transform`, record-model class parameters and
PEP 702 deprecation, but only four booleans reach `ClassTraitObservation` (`symbols.rs:163-175`).
Dataclass options are therefore re-parsed from decorator syntax (`symbolic_fields.rs`), and
dispatch has no closure basis at all. Import aliases resolve by name against a relation that
excludes the analyzed modules themselves (F05).

**The central design constraint is admissibility, not supply.** Provider facts arrive in distinct
claim classes:
- exact syntactic;
- resolved identity;
- type-level inferred;
- evaluated verdict;
- diagnostic;
- heuristic.

Each class supports different downstream uses. Today no model owner decides what a type-level fact
may establish:
- Local theory already uses Pyrefly type terms as premises "under the typing model"
  (`local_theory.rs:41-51`, `:984-989`, `:1369-1372`), but `EvidenceStatus` (`analysis/policy.rs:10-16`)
  has no way to say so.
- Selection admits only declared, exact, unconditional types (`selection/evaluate.rs:568-597`).
- Analytics admits inferred `Raised` types (`analytics/attributes.rs:495-511`).
- §3.9 states that a type observation is never an execution witness.

Behavioral uses of the recommended facts would multiply these independent decisions: closing
dispatch through `@final`, typed receivers, `NoReturn` pruning and literal guard truth. **F01**, one
owned admission policy with a visible typing-model qualification, is therefore the prerequisite for
those uses. Catalog uses need no such prerequisite, because their questions are themselves
type-level ("declared parameter type", "effective typed signature").

**Several premises in the prepared evidence do not hold:**
- *Finality does not close dispatch without assumptions.* `typing.final` is not enforced at
  runtime, so finality closes an override set only under the typing model. Enum classes that have
  members are runtime-closed.
- *The 730 ty-AMBIGUOUS rows are mostly not a provider gap.* A `try` body is AMBIGUOUS because any
  earlier statement may raise. The owned completion operation can certify that; provider dead-code
  verdicts can only refute.
- *ty's narrowing constraints are unconsumed.* The `python-analyzers` skill says otherwise and is
  wrong. For the questions in scope they add little beyond the predicates already lowered.
- *Exceptions need few new facts.* The core of exception identity, `raise C(...)` and `except (A, B)`,
  is derivable from resolution facts already extracted, through the existing complete-MRO
  `CheckedExactClass` operation (`execution/model_context.rs:25-56`).

**Assurance (S4).** ty's type inference (`ty_python_semantic`) does not belong in the pipeline as a
premise provider. ty sees a different environment (the release under a virtual `/flow` root plus
vendored typeshed, `cpg-flow/src/lib.rs:324-345`). It returns `Unknown` for unannotated returns.
It disagrees with Pyrefly on spec-undefined points. An in-pipeline disagreement would therefore
be uninterpretable. ty is valuable as an offline differential oracle over Pyrefly's type facts.
§B1 needs clarification, not reversal: the flow exception should name ty's index-level facts, and
inference should stay out of the pipeline (F10).

**Defects found along the way:**
- Source `try` statements admit only a bare first handler (`execution/completion.rs:351-385`).
  Typed handler matching existed before the cutover (`efa01f3d`, 2026-09-26), is listed as a
  retained obligation in the Phase 4 plan (line 429) and is claimed by §9.9 (F04).
- Imports between analyzed modules cannot resolve (F05).
- Four §4.2 claims are stale, and two "no consumer" deferrals are contradicted by named consumers
  (F06, F08).
- `FacetMembership` is on the serving wire schema, but its evaluator always returns `Ok(None)`.
  §14.7 requires unsupported predicates to be refused before effects (F11).

**Decision: Revise.** G7 and G8 fail; A2 is violated in scope; CI-G1 is unresolved for existing
typing-model refutations. The integration target below is the recommended direction (**Proposed**),
in two priority bands:
- **Catalog-facing facts (T1–T3, T5)** and the defect corrections can proceed now.
- **Behavioral uses of type-level facts** wait for F01's decision.

---

## 0. The integration target

The targets are ordered by consequence. Prerequisites are named separately, by the capability they
need. Every row is **Proposed**. Provider surfaces are **Interface-checked** through the supply
catalogue (item presence at pyrefly 1.4.0-dev.3 / ty 0.0.14, 2026-10-03); this reviewer did not
re-inspect provider source. Claim class follows §3 below. Attachment codes are the supply catalogue's:
PR = Pyrefly's parse of record, TY = ty's second parse.

| # | Facts | Provider and attachment | Claim class | First consumers whose output changes | Prerequisite |
|---|---|---|---|---|---|
| T1 | Effective (decorated) callable type and synthesized-member signatures; decorator callees | Pyrefly decorated function type, Pysa `decorator_callees`, `ClassMetadata` synthesized `__init__`; PR-range | type-level (signature); resolved (callees) | N3 effective callable; catalog `CatalogCallable`/`CatalogInvocation`; selection `MemberKind`, `InvocationForm`, `ParameterType`; retrieval API units | none for catalog roles |
| T2 | Typed signature slots and return types per variant (declared and inferred kept apart) | Pyrefly native `Type` per parameter and return of undecorated, decorated and synthesized variants; PR-range | type-level | selection `ParameterType`; FCA `ParameterType`/`Returns`/Type layer; retrieval rendering; catalog option types | none |
| T3 | Class metadata: final, enum members and values, protocol and runtime-checkable, explicit abstract and unimplemented abstract members, metaclass, record-model class parameters (dataclass, `dataclass_transform`, pydantic config), slots, NewType, deprecation (class, function, overload) | Pyrefly `ClassMetadata` and its abstract-members/deprecation answers; PR-range | resolved (metaclass, decorators, enum members) or type-level (final, abstract) per field | catalog constructors (abstract, synthetic), selection option domains (enum) and a deprecation facet; `symbolic_fields` (retires option parsing); dispatch and local theory closure | none for catalog; F01 for typing-model closure |
| T4 | Exact exception class identity at raise sites and handlers | **Existing** resolution facts (Pysa init/new targets, lexical builtin/import resolution, symbols, complete MRO) through `CheckedExactClass`; existing `TypeRole::Raised` as an upper bound; Pyrefly except-handler types and `unreachable-except-clause` as evidence | resolved (exact core); type-level (upper bound); evaluated verdict (evidence) | BaseCompletion/Enriched completion; Summary exception paths; S0 `ConditionalRaise`; selection `Raises` facet; FCA `Raises` | none for the exact core; F01 for upper-bound matching |
| T5 | Per-alias import resolution; provider facts for computed `__all__` | Pyrefly `import_handle` per alias (roots included); Pyrefly `Exports` `unresolvable_dunder_all_range`, partial and invalid entries; PR-range | resolved | N2 `imports` (retires name matching); ImportReference projection; catalog alias and public paths; Exports coverage boundary | none (corrects F05) |
| T6 | Located types at receivers, attribute bases, assignment values and returns; attribute → declaration resolution; class field inventory including method-defined attributes | Pyrefly `get_type_trace` at more roles, class members/`get_class_fields`, cross-references; PR-range | type-level (types, attribute targets); resolved (field inventory) | `local_fields` FieldLocationCandidate; `read_dynamic` ClassInspection; selection `ReceiverLocation`; catalog field-access candidates; `read_fields` screening; dispatch `MissingReceiverClass` | F01 for any premise use; candidates need none |
| T7 | Decisions for ty predicates ty leaves open: `IsNonTerminalCall`, `ContextManagerSuppresses` | ty predicates (TY-id at call/`with` ranges) joined to Pyrefly `CallResult` = `Never` and `__exit__` return type; PR-range | evaluated verdict, decided under the typing model | `cpg-flow` lowering (keeps a typed atom); Local and completion (prune under the typing model) | F01 |
| T8 | Literal and enum-literal types at expressions and `Final` names | Pyrefly literal values; PR-range | type-level | completion `if` truth (as a separate basis); selection `ConfigurationLiteral` (enum fields); local theory | F01 for behavioral; none for selection |
| T9 | Evidence only: call-check diagnostics on materialized examples; pytest fixture binding; per-call overload choice | Pyrefly error kinds, fixture resolution, `get_chosen_overload_trace`; PR-range | diagnostic, resolved, type-level | C1 scenario check-status dimensions and fixture context; scenario invocation witnesses; binder test oracle | none |

### T1 Effective and synthesized callable signatures

- **Semantic contract.** Pyrefly states the type the decorated `def` binds: its parameter list,
  ParamSpec/Concatenate effect and return type. For classes, it states the synthesized `__init__`
  that its record model computes. N3 stays the single owner of effective-callable assessment
  (§15.5). It records the provider's effective type as a new **effective-typed** signature role,
  with its own support, separate from the source variants. The role answers a type-level question:
  "what does a type checker accept at this name".
- **Consumers.**
  - *Normalization:* N3 emits effective-typed variants. The binder may bind against them for
    catalog invocation validity only. The private composition-admission token is unchanged.
  - *Behavior:* not a premise. §14.4 says `wraps` / `__wrapped__` is a link, not behavioral
    equivalence. The effective-typed variant never admits the undecorated body.
  - *Catalog, selection and retrieval:* `CatalogInvocation` gains the effective-typed role.
    `MemberKind` and `InvocationForm` take descriptor kind from the provider's resolved decorated
    type when syntax recognition is Unknown. That includes `contextmanager` / `asynccontextmanager`
    results, which feed `InvocationForm::ContextManager` (no producer today), and awaitable or
    iterable results for the forms §14.4 lists. Retrieval renders the typed effective signature,
    labeled as such.
  - *Analytics:* FCA `Decorator` attributes key on resolved decorator-callee identity, not
    expression spelling (`attributes.rs:525-547`).
- **Simplifies or retires.** Nothing in behavior. In the catalog, it retires "effective Unknown"
  for type-preserving decorators. Bare-builtin descriptor recognition (`descriptor()`) remains the
  behavioral route and agreement check.
- **Guarantees preserved.**
  - Source and effective identities stay distinct.
  - A decorator whose return type is dynamic (`Any`, `Unknown`) yields effective Unknown with
    reason, never `(*args, **kwargs)`.
  - Display-only or opaque closures stay `DisplayOnly` (ADR-0098/0100).
  - Overload alternatives stay separate variants; no union signature is invented.
  - A missing decorated type is not an undecorated callable (CI-04).
- **Authority change.** An ADR plus amendments to §15.5 (effective callables), §14.4 and the
  §4.2.3 "not carried" table (F02).

### T2 Typed signature slots

- **Semantic contract.** Each `SignatureParameter` slot and return port of each variant (source,
  effective-typed, synthesized, stub) carries a type observation. Its subject is the slot, not only
  the source declaration. The observation keeps the existing `declared` flag and gains an explicit
  inferred basis.
- **Consumers.**
  - Selection `ParameterType` evaluates declared types as today and inferred types under a
    separately named basis. A requirement says which bases it accepts.
  - FCA `ParameterType`, `Returns` and the Type layer gain inferred attributes as a distinct
    attribute kind. CI-09 already lets FCA group on them; they never merge with declared
    attributes, because labels match (§9.6).
  - Retrieval API units render types.
  - Behavior is unchanged: types remain theory inputs under F01's policy.
- **Simplifies.** Removes the special path that only source parameters carry types
  (`evaluate.rs:577-585`).
- **Guarantees preserved.** Structural type equality and nominal identity never imply runtime
  assignability (§14.7). Declared and inferred stay distinguishable to every consumer.
- **Authority change.** §15.4 Types amendment. It is a schema migration; no ADR is needed beyond T1's.

### T3 Class metadata

- **Semantic contract.** It is one attributed class-metadata observation per class. Each field
  carries its own basis:
  - *resolved:* metaclass, decorator-derived kinds, enum member names and values from the class
    body, NewType, deprecation with its message;
  - *declared:* `@final`, explicit `ABC`/`abstractmethod`;
  - *type-level computed:* unimplemented abstract members, `dataclass_transform`-driven record
    parameters.

  `ClassTraitObservation`'s four booleans become derived views of it, or stay as they are, so no
  duplicate authority exists.
- **Consumers.**
  - *Normalization:* `symbolic_fields` reads class-level record parameters (`init`, `kw_only`,
    `frozen`, `order`, `eq`, `slots`, `match_args`, `dataclass_transform` field specifiers) from the
    provider instead of parsing decorator keyword literals. It keeps the stdlib-identity guard
    (`standard_target`) for behavioral initialization admission, because a `dataclass_transform`
    library's typed `__init__` need not equal its runtime `__init__`.
  - *Behavior:* dispatch gains two closure bases (F03):
    - **runtime-closed**, for an enum class with members and for builtins that cannot be
      subclassed at runtime, closes exactly;
    - **declared final** (`@final`) closes only *under the typing model* (F01).

    Local theory's class domain can close on the same bases instead of returning
    `OpenClassUniverse`.
  - *Catalog and selection:*
    - Constructors of classes with unimplemented abstract members are marked non-instantiable.
    - A synthetic constructor takes its parameters from the provider's synthesized `__init__`
      (with T1).
    - `ConfigurationLiteral` decides enum-typed fields from the member set (CA3).
    - A deprecation facet and requirement become available. For "correct use", this is the single
      most valuable product fact here.
  - *Analytics:* class metadata can define attributes for FCA.
- **Simplifies or retires.** Retires `decorator_options` literal parsing (`symbolic_fields.rs`, B6)
  for class options. It shortens the open floor in `dispatch.rs:600-604` for closed bases.
- **Guarantees preserved.**
  - An open subclass universe stays open unless one of the two bases applies.
  - Typing-model closure is never reported as runtime closure.
  - A false flag from complete metadata is "not declared final", never "subclasses exist".
  - Field flags already provider-stated (`RecordFieldObservation`) are not restated.
- **Authority change.** §15.4 and §3.5.1 amendments. The dispatch closure bases need F01's ADR.
  The forward plan §7 closed-hierarchy row is amended (F03, F07).

### T4 Exact exception class identity

- **Semantic contract.** The exception domain becomes "exact class by pinned symbol identity with
  complete MRO" instead of the one-member `ExactRuntimeException` codebook (`execution.rs:30-42`).
  Each raise is admitted by kind:

  | Raise | Admission | Basis |
  |---|---|---|
  | `raise C(...)` | `C` resolves (Pysa init/new target, or lexical builtin/import resolution to a class symbol) and construction completes normally | resolved; normal construction is a modelable premise for builtin exception classes |
  | `raise C` | a resolved class object | resolved |
  | `raise e` | admitted only through value flow (entry or local origin), otherwise Unknown; Pyrefly's `Raised` type `T` is an upper bound under the typing model (F01) | type-level bound |

  For handlers, each `except` type expression resolves to an ordered class tuple through the same
  resolution facts. `CheckedExactClass::matches` (`model_context.rs:46-56`) already implements
  exact subclass matching.
- **Consumers.**
  - *Completion:* source `try` admits ordered typed handlers. This restores the retained
    obligation (F04). Nonmatch needs complete class knowledge of the *raised* class (§9.9).
  - *Summary:* exception paths over admitted classes.
  - *Synthesis:* S0 `ConditionalRaise` names a class instead of rendering statement text
    (`synthesis/assertions.rs:745-749`).
  - *Selection:* the `Raises` facet gets a producer.
  - *Analytics:* FCA `Raises` may keep the inferred type as a grouping attribute.
  - *Evidence only:* Pyrefly `unreachable-except-clause` corroborates or withholds a handler path
    and is never a negative premise.
- **Simplifies.** No provider fact is needed for the exact core. It removes name lookup in typeshed
  `builtins` as the only class route (`context_execution.rs:463-470`); that route is principled but
  narrow.
- **Guarantees preserved.**
  - A raise inside `try`/`with` still needs handler, protocol and finalizer evidence before escape.
  - Opaque construction, exception groups and named-handler cleanup stay refusals.
  - **Propagation stays bespoke:** no provider states may-raise sets (`exceptions.propagation` is
    blank).
- **Authority change.** None beyond the forward plan §6 row (F04). §9.9 already states this target.

### T5 Import resolution and computed `__all__`

- **Semantic contract.** Each `ImportAliasObservation` carries the provider module Pyrefly resolved
  for it, analyzed roots included. Unresolved is an explicit provider module. Pyrefly's
  `unresolvable_dunder_all_range`, partially known `__all__` and invalid entries are attributed
  Exports facts beside the syntax detector.
- **Consumers.**
  - N2 `imports` (`relation_normalization.rs:263-320`) uses the per-alias resolution instead of
    absolute-name matching against `DependencyModuleObservation`, which excludes roots
    (`pyrefly_stage.rs:1254`).
  - The ImportReference projection and catalog alias/public paths gain intra-release arcs.
  - Exports coverage cites the provider's own blind-spot report.
- **Retires.** `absolute_module` name matching (B8) as the resolution route. The syntax `__all__`
  detector remains an independent completeness control.
- **Guarantees preserved.** Ambiguous and unresolved stay explicit; module identity stays
  provider-qualified (`acquired` vs `bundled` vs `namespace`).
- **Authority change.** None (implementation inside §15.4's contract). Correct §4.2.3 (F05, F06).

### T6 Located types and attribute resolution

- **Semantic contract.** Type observations are added at receiver, attribute-base, assignment-value,
  return-value and `with`-item roles. Each role has a named consumer, so B13's consumer-driven role
  policy stays. The class field inventory comes from Pyrefly class fields, including attributes
  defined in methods. Attribute → declaration targets come through the receiver type. All are
  type-level, except the field inventory, which is resolved.
- **Consumers.** Every consumer here is a **candidate** consumer:
  - `local_fields` takes receiver classes from any receiver-role type, not only `TestOperand`
    (`local_fields.rs:346-349`).
  - `read_dynamic` adds a typed `ClassInspection` candidate when its exact traces
    (`ReceiverDeclaration`, `UniqueGlobalInitializer`) fail.
  - Catalog field-access candidates separate fields from methods and inherited members (forward plan
    §7 "inherited methods counted as field reads").
  - `read_fields` screens only the classes whose field inventory contains the name.
  - Selection `ReceiverLocation` becomes decidable as a candidate witness.

  A located type becomes a premise only under F01's typing-model qualification, and a field
  identity is exact only when the receiver class is closed (T3).
- **Simplifies.** Retires spelling-only matching as the sole candidate basis (G3). The exact traces
  stay, because they are a stronger claim class (value flow).
- **Guarantees preserved.** A field location never establishes allocation, alias or mutation
  stability (§B5). An unresolved receiver still screens every class.
- **Authority change.** An ADR removing the §13 "located/contextual types" deferral, which states
  that no consumer needs them, plus a §15.4 amendment (F08).

### T7 Deciding ty's open predicates

- **Semantic contract.** `cpg-flow` currently lowers `IsNonTerminalCall` to `Condition::always()`
  (`predicate.rs:482`) and `ContextManagerSuppresses` to an opaque atom (`:487-490`). Each should
  remain a typed atom keyed by its call or `with` occurrence. Local or completion decides it from
  Pyrefly facts:
  - a call whose result type is `Never` is terminal *under the typing model*;
  - an `__exit__` whose declared return is `None` or `Literal[False]` does not suppress, under the
    same model.

  Other cases keep today's conservative value.
- **Consumers.** Local value observations and completion regions after terminal calls and
  non-suppressing `with` exits. This addresses the `with`-exit subset of the 730-row bucket
  (forward plan §7, dated 2026-09-27).
- **Stays bespoke.** The `try`-body AMBIGUOUS case, which is the bulk of the bucket. The certificate
  that earlier statements complete normally is completion's job: it is our runtime model, and no
  provider states it.
- **Guarantees preserved.** Dead-code verdicts never certify reachability. A typing-model pruning
  is visible in the claim's qualification (F01).
- **Authority change.** Forward plan §7 reachability row decomposed (F09). The §3.9 "Lowering from
  ty's diagrams" text gains the decided-atom rule.

### T8 Literal and enum-literal types

- **Semantic contract.** "Declared or inferred literal type" is a basis distinct from evaluation,
  as the behavior model requires.
- **Consumers.**
  - Selection's `ConfigurationLiteral` widens to enum literals and `Final` constants (no
    prerequisite).
  - Completion `if` truth and the closed evaluator may consume the basis only under F01, as
    `...UnderTypingModel`, never as an evaluated value.
- **Authority change.** Covered by F01's ADR.

### T9 Evidence-only facts

- **Call-check diagnostics** on materialized example code feed a "static type check" status
  dimension of C1 scenarios. §14.5 lists independent status dimensions for parse, binding,
  environment and execution checks. A diagnostic never makes an example executed or correct, and
  its configured error preset is part of the recorded run context (CI-10).
- **Pytest fixture binding** gives C1 its "import/fixture context" for test-derived scenarios.
- **Per-call overload choice** gives a type-level witness of which typed variant a documented call
  matches. Runtime always executes the implementation, so it is never a behavioral binding.
- **Authority change.** None; plan rows when a product task selects them (§14.11).

### Not recommended, and concepts that stay bespoke

| Concept | Why it stays as it is | Owner |
|---|---|---|
| Lexical binding history (B1) | Catalog profile runs no ty. Pyrefly's binding graph drops pruned branches. Ruff's semantic model has no public driver. ty's index is a second parse keyed by node identity. Parse-of-record occurrence identity must own binding events (§15.4) | `cpg-extract::lexical` (retain; ty's index may serve as a behavioral-profile test oracle) |
| Argument binding (B3) | One binder is the authority (§15.5). Pyrefly states no data map. ty's public map needs ty inference (S4). A provider map is a test oracle only | `lctx-model::calls::bind` |
| Occurrence owner (B11) | It is the model's only definition of "caller" | `occurrence_owner` |
| Value evaluation (G8) | Value semantics are model-owned; literal types are a separate basis (T8) | closed evaluator |
| Points-to, heap, aliasing | Blank in all three providers; excluded by §B5 and §13 | Summary obligations |
| Effects, callbacks, resources | `dataflow.effect-kinds` is blank; authored models own them (§9.9) | `domain::models` |
| Exception propagation and may-raise | Blank; completion and Summary own escape | completion, Summary |
| CFG, dominators, loops | No provider CFG (ruff's is a stub); ty diagrams plus completion suffice; loop unrolling stays deferred | completion, `cpg-flow` |
| Taint, threads, async lifecycle, runtime evidence | Blank or out of scope; runtime evidence belongs to offline oracles | — |
| Native extension behavior | Stubs and models only | models |
| ty narrowing constraints | Redundant with lowered test predicates for the in-scope questions. G11's tuple classinfo comes from the existing `Argument`-role type of the classinfo expression | local theory (extend) |
| Ruff lint, metrics, style and fix facts | No consumer in the served model (binding question 17) | — |

---

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | Which additional pyrefly/ruff/ty code facts library-context should integrate, and how normalization, behavioral analysis, analytics and catalog/selection/synthesis/retrieval would use them. Code and documents at `6885dbf6`, clean tree |
| Standard | Core 3.2 (principles and template), code-intelligence profile 1.3, library-context binding |
| Tier · purpose | Design · target. Blocking authorities (§B1, §B5, forward plan §2, §13 deferrals) are evaluated as revision candidates (slot 11) |
| Reviewer · date | `design-reviewer` subagent (fresh context), 2026-10-03 |
| Maturity and outcome | Cutover Phases 0–4 implemented; Phase 5 serving implemented, qualification stopped. The decision should yield a prioritized integration target that a feasibility follow-up can cost |
| Supported scope | Fact families Syntax, Lexical, Exports, Signatures, Calls, Types, Flow. Consumers: N1–N5, Local/execution/Model/Summary, dispatch, local theory, Structural/Analytic (FCA, projections), C0–C2 catalog/selection, S0 synthesis, retrieval units. Workloads: extraction, analysis, answering, change (profile functional target) |
| Exclusions | Integration cost, dependency mechanics and pin logistics, except where a semantic property constrains correct use. Docs/Deployment families. Serving transport. Evaluation |
| Expected changes | S1 domain extension (effective signatures, exception identity); S2 mechanism substitution (B1–B14); S3 analyzer upgrade (pyrefly 1.4.0-dev.3 on ruff 0.0.14); S4 second type provider |
| Baseline | Pyrefly fork `a07b7bae` (1.3.1) on ruff 0.0.11; ty 0.0.14 in `cpg-flow`. The migration is assumed |
| Method and coverage | Read the owners and adjacent consumers (DESIGN §1.1, §2, §13; sections §3, §3.9, §4.2–§4.3, §9, §9.9, §14.4–§14.8, §14.11–§14.12, §15.1, §15.4–§15.5, §15.8, §15.13; forward plan §2, §6, §6.2, §7; ADR-0046 extracts). Verified the decisive source myself (cited inline). Used three prepared maps as leads ([supply map, demand map, supply catalogue](../evidence/2026-10-03_code-facts-opportunity/README.md)), and the `python-analyzers` skill's class-metadata page and opportunity register. **Not examined:** the roughly 270 row-integrity `MissingEvidence` sites; Python serving code; the provider source itself (presence is the catalogue's verification); behavior at ty/ruff 0.0.14, which the catalogue marks unverified |

## 2. Responsibilities, dependencies and semantic ownership

| Component | Responsibility and hidden decisions | Consumer contract | Dependencies / direction | Expected reason for change |
|---|---|---|---|---|
| `cpg-extract::pyrefly_stage` + record mappers | One Pyrefly session; maps provider answers to attributed records; selects type roles (B13); bespoke lexical, docstring and `__all__` recognizers | Typed `lctx-model::domain` relations with support and coverage | Pyrefly fork, ruff → `lctx-model` | New fact family members (T1–T6, T9), provider upgrades |
| `cpg-flow` | ty semantic index over the second parse; lowers predicates and reachability to model conditions | Ranges, place text and model conditions only (ADR-0046) | ty_python_core → `lctx-model` | T7 atoms; S3/S4 |
| `lctx-model::domain` facts | Declares relations, supports, coverage, codebooks | Single authority (§B2) | none | Every target adds declarations |
| N1–N5 normalization | Entities, ownership, references, imports, effective callables, receivers, events, policies, binding | Total assessments with typed reasons | facts → normalized | T1 (N3), T5 (N2), T6 (N4 receivers) |
| Execution (Local → Summary), dispatch, local theory | Runtime abstraction, completion, dispatch sets, typed primitive theory | Five verdicts, obligations | normalized → analysis | T3 closure, T4, T6, T7, T8 under F01 |
| Structural / Analytic | Controls, delegation, projections; FCA/RCA, communities, ranking | Exact under a projection, or heuristic | analysis → analytics | T1 decorator identity, T2 inferred attributes, T4 raises |
| Catalog C0–C2, S0, retrieval | Members, callables, constructors, options, evidence, selection, assertions, units | Supported, contradicted, unresolved or conflicting outcomes; witnesses | everything above | T1–T3, T5, T8, T9 |

**Fact and fidelity table** (current, plus the proposed rows marked ▲):

| Fact family or relation | Provider and revision | Fidelity | Coverage and unknowns | Identity | Consumers |
|---|---|---|---|---|---|
| Syntax occurrences, placements, details, declarations, decorators, call syntax/arguments | Ruff AST retained by Pyrefly (parse of record) | extracted (`NativeStructural`) | Artifact-scoped; `ResourceRefused`, `UndecodableSource`, `SyntaxError` → Partial | Occurrence ids of the parse of record | everything |
| Lexical scopes, bindings, references, resolutions | Bespoke recognizer over the Ruff AST, with Pyrefly builtins, wildcard and `SysInfo` | derived (`Recognizer`, `NormalizedStructural`) | Unresolved is a typed target; Candidate modality for multiple bindings or unknown star imports | Occurrence ids | N2, catalog paths/aliases, `read_dynamic`, aspects, flow attachment |
| Exports: public names, `__all__`, import aliases | Pyrefly public-name helpers; syntax `__all__` detector | analyzer assertion / extracted | Partial with `OutsideProviderModel` for computed `__all__` | Provider-qualified modules | catalog public identity, N2 imports |
| Signatures: symbols, traits (4+7 booleans), MRO with completeness, undecorated variants, parameter docs | Pysa collectors + Pyrefly class MRO | report projection / native structural | `NativeUnavailable` variants Partial; unmatched attachment boundaries | Provider symbols | N1, N3, N5, dispatch, catalog |
| Calls: sites, targets, destinations, receivers, resolutions | Pysa call graphs | analyzer assertion | Unresolved with native reason; any boundary → module Partial | Provider call sites | N4, N5, projections |
| Types: terms, six roles, body kind, record fields | Pyrefly native `Type` | native structural; opaque → DisplayOnly | `MissingEvidence` per missed trace; budget | Content-addressed terms | local theory, selection, FCA |
| Flow: uses, definitions, reaching, regions, tests, value paths | ty 0.0.14 index (second parse) | native traversal | Exact-only attachment; `ScopeBoundary`; AMBIGUOUS → approximation | Attached to parse-of-record occurrences | Local, execution, theory |
| ▲ Effective-typed signatures, decorator callees (T1) | Pyrefly decorated type, Pysa | type-level / resolved | "decorated type unavailable" boundary; dynamic → Unknown | Variant of the declaration's callable | N3, catalog, selection, retrieval |
| ▲ Typed slots/returns (T2) | Pyrefly | type-level, declared vs inferred | per-slot `MissingEvidence` | Slot subject | selection, FCA, retrieval |
| ▲ Class metadata (T3) | Pyrefly `ClassMetadata` | per-field basis (resolved / declared / computed) | Class-level; absent metadata ≠ "no flags" | Class symbol | `symbolic_fields`, dispatch, theory, catalog |
| ▲ Import alias resolution, `__all__` blind spots (T5) | Pyrefly | resolved | Unresolved provider module explicit | Alias occurrence | N2, projections, catalog |
| ▲ Located types, attribute targets, field inventory (T6) | Pyrefly | type-level / resolved | per-occurrence `MissingEvidence` | Occurrence | candidates only (F01 for premises) |
| ▲ Terminal and suppression decisions (T7) | Pyrefly types joined to ty predicates | evaluated verdict, typing model | undecided stays conservative | Call/`with` occurrence | Local, completion |

**Semantic ownership.**

| Concept or operation | Authority | Assessment |
|---|---|---|
| Effective callable | N3 (`callable_normalization`) | Single owner, adequate distinctions (Known/Unknown/Conflicting × identity/descriptor/signature/body). **Inadequate input:** the effective signature it must own has no provider fact (F02) |
| Dispatch closure | `normalized::dispatch` | Single owner; every set is open by floor. No closure basis exists in the model (F03) |
| Exception identity and handler matching | `execution` completion + `model_context::CheckedExactClass` | The operation exists; the domain is a one-member codebook, and source `try` uses only bare handlers (F04) |
| Admissibility of type-level facts | **no single owner**: local theory (`UnderTypingModel`), selection (`declared` + Exact), analytics (any), §3.9 prose | Independent decisions about one question (F01) |
| Record-model class options | `symbolic_fields` decorator syntax, while field flags come from Pyrefly | Two mechanisms for one record-model concept (F07) |
| Import module resolution | N2 by absolute name | Re-derived by spelling while the provider resolution is computed and discarded (F05) |

## 3. Contracts, constraints and testing boundaries

**The admission contract the target needs.** A provider fact's downstream use depends on its claim
class, not on its family:

| Claim class | What the provider establishes | Premise for | Candidate / evidence | Never |
|---|---|---|---|---|
| Exact syntactic | What the bytes say (parse of record) | Every owned operation | — | — |
| Resolved identity | Which declaration, module or class a name, alias or callee denotes, in context | Identity questions when Definite and Exact (precedent: Pysa targets, lexical resolution, MRO) | when Candidate or Potential | a runtime value or a target it was not asserted for |
| Type-level inferred | The static type, effective signature or computed class property under typing semantics | (a) catalog/selection questions that are themselves type-level, labeled with that basis; (b) behavioral questions only **under a visible typing-model qualification** (F01) | always usable as a candidate | an execution witness (§B5); closing an open universe without a stated closure basis |
| Evaluated verdict | A provider's conclusion (unreachable, always-truthy, exhaustive, NoReturn) | under the typing model, only for pruning (T7) | corroboration or withholding | certifying reachability; a refutation premise without our own check |
| Diagnostic | A finding from the provider's configured checks | — | scenario check status (T9), test oracle | a behavioral negative |
| Heuristic | Pattern or side-effect guesses | — | ranking, grouping (CI-09) | a control, limit or behavioral claim |

**Coverage and absence for the new facts.** Each new relation declares its coverage family (§15.8).
These states differ:
- "provider did not report a decorated type", which is Unknown;
- "the decorated type equals the undecorated type";
- "no decorator", which is known from complete syntax.

A `final = false` flag from complete metadata means "not declared final". It never asserts that
subclasses exist. A missing import resolution is `Unresolved`, never an absent import. For every
row, §4.3's route requires an observed/unknown twin.

**Routing constraint (profile semantics).** Catalog is the default profile and does not request
Flow (§3.3). Any fact a catalog consumer needs must come from the parse of record (Pyrefly/Ruff).
ty facts can serve only behavioral-profile consumers. This rule decides T6's field inventory
(Pyrefly class fields, not ty `reachable_member_bindings`), T3 and T5.

**Cross-parse semantics after the migration.** With one parser line, a ty byte range and a Pyrefly
`TextRange` coincide for the same bytes. Node identity still differs. TY-id facts (predicates,
definitions, loop headers) attach only through their node's range. The exact-only join with
Partial coverage on mismatch stays mandatory (§15.4). After the migration, a mismatch signals a
modeling difference, such as the `TYPE_CHECKING` runtime view, not parser skew.

## 4. Composition and execution

Analysis record columns for the stages the target changes:

| Stage | Question answered | Projection / inputs | Method and settings | Exact / conservative / heuristic, and model | Budgets and partial results | Output and evidence linkage |
|---|---|---|---|---|---|---|
| N3 effective callable (T1) | What the declaration binds after decoration | Declarations, decorators, traits, undecorated variants; ▲ effective-typed variants | Pure model operation | Exact under syntax for source; type-level for effective-typed | Unknown with reason; no fallback | Assessment cites the provider-type support |
| Dispatch (T3) | Which callables an override event may reach | Override chains, complete MROs; ▲ closure basis | Pure | Open unless runtime-closed (exact) or declared-final (under the typing model) | `IncompleteAncestry`, `MissingReceiverClass` | Members + premises; the closure basis is a premise |
| Completion (T4) | Normal or exceptional outcome of a statement sequence | Syntax, resolutions, `CheckedExactClass` | Charged worklist, 64-statement cap | Exact under the runtime model for resolved classes | `UnsupportedControlFlow` for unresolved raises | Raise and handler premises cite resolution rows |
| Local theory (T3, T8) | Truth of a test over a type domain | TestOperand/Argument types, MRO | Bounded domain ≤4096 | Exact under the **typing model** (stated in code, not in status) | `OpenClassUniverse`, `WorkLimit` | `TheoryWitness` → Role-channel derivation |
| Selection `ParameterType` / `MemberKind` / `FacetMembership` (T1, T2, T4) | Does a member satisfy a requirement | Catalog + facts | Pure predicates | Basis declared per predicate | Unresolved; `FacetMembership` always unresolved today (F11) | Witnesses cite slots and observations |
| FCA attributes (T1, T2, T4) | Shared-attribute grouping | Typed attributes | Bounded NextClosure | Exact under the admitted context; analytic, not behavioral | Bounds | Incidence source cites observations |
| Retrieval units (T1, T2) | Text for API units | Catalog contracts | Deterministic rendering | Presentation only | — | Unit identity includes rendering input |

**Composition assessment.** Every target composes through an existing owner: N3, dispatch,
`CheckedExactClass`, local theory domains, selection predicates or FCA attributes. None needs a new
engine, registry or provider framework (A3). The one composition hazard is cross-cutting: each of
these owners would decide on its own what a type-level premise may establish (F01).

## 5. Change and failure scenarios

| Scenario and kind | Owning component | Contract change | Expected vs observed consumers | Independent edits / hidden knowledge | Evidence |
|---|---|---|---|---|---|
| **S1a** Add effective-typed signatures (domain extension: new variant role) | extraction mapper → N3 | New variant role and support; N3 assessment input | Expected: N3, catalog invocation, selection, retrieval. Observed route: §4.3 (model declaration, provider wiring, normalization consumption) holds | The binder must not grant composition admission from the new role; this is a deliberate rule in N5, not an incidental edit | §4.3; §15.5 N5 token rule; `callable_normalization.rs:536-612` (Interface-checked) |
| **S1b** Exception identity (domain concept change: closed codebook → symbol identity) | `execution` | `ExactRuntimeException` → exact class symbol | Completion, Summary, S0, selection; `CheckedExactClass` reused | Codebook consumers must move to the symbol; append-only codes stay reserved | `execution.rs:30-42`, `model_context.rs:25-56` |
| **S2** Mechanism substitution, per bespoke mechanism (table below) | various | varies | — | — | — |
| **S3** Analyzer upgrade to Pyrefly 1.4.0-dev.3 / ruff 0.0.14 (instance/binding) | `pyrefly_stage`, `type_records` | `Type::Overloaded` new (opaque unless modeled), `CallableResidual` removed, `Enum.value` keeps literal types, `X \| None` with unresolved X → `Unknown \| None`; patch change 1 obsolete; one ruff line for both parses | Types-family consumers see moved facts; Flow attachment loses parser skew | Fact meanings move without a codebook change; the upgrade needs a declared fact-diff | Skill migration page (dated 2026-10-03, not run) |
| **S4** ty inference as a second type provider (new provider) | would be `cpg-flow` or a new stage | Provider-qualified type terms from two providers | Every type consumer must reconcile disagreement | Environment mismatch (virtual `/flow` root, no site-packages: `cpg-flow/src/lib.rs:324-345`), gradual `Unknown` for unannotated returns, and spec-undefined disagreements make in-pipeline disagreement uninterpretable | Rejected as a premise provider; offline oracle recommended (F10) |
| Journey: a module full of unresolved references | extraction + N2 | — | Per-alias resolution (T5) keeps Unresolved explicit; located types (T6) are candidates | — | CI-04 preserved |
| Journey: trace a served claim (T4 raises) | S0 → completion → resolution → span | — | Claim cites resolution rows and the MRO premise in one generation | — | Proposed |

**S2: mechanism substitution verdicts.**

| # | Bespoke mechanism | Provider substitute | Verdict | Reason |
|---|---|---|---|---|
| B1 | Lexical recognizer | ty index; Pyrefly bindings; ruff semantic model | **Retain** | §2 "Not recommended" row; catalog profile has no ty |
| B2 | Static-branch marks | Pyrefly `SysInfo` (already) | Retain | Already provider-backed; a tested agreement exists |
| B3 | Argument binder | ty argument map; Pyrefly hints | **Retain** (single authority) | §15.5; provider maps are oracle-only |
| B4 | Receiver/dispatch normalization | Typed receivers (T6) | Retain; add candidate input | Expression-relative receiver semantics are model-owned |
| B5 | Decorator/effective recognition | Pyrefly decorated type (T1) | **Complement** | The effective-typed role is added; behavioral descriptor recognition stays |
| B6 | Dataclass options from syntax | Pyrefly `ClassMetadata` (T3) | **Replace** for class options; keep the stdlib-identity guard for behavior | Provider-stated; covers `dataclass_transform` |
| B7 | Computed `__all__` detector | Pyrefly `Exports` blind-spot facts (T5) | Complement | The detector stays as an independent control |
| B8 | Import naming by absolute name | Pyrefly per-alias resolution (T5) | **Replace** | Already computed; corrects F05 |
| B9 | Type term lowering | — | Retain; extend for `Overloaded` | Provider-neutral content addressing |
| B10 | Docstring offsets | — | Retain | No provider gives offsets |
| B11 | Occurrence owner | — | Retain | Model definition of "caller" |
| B12 | Dynamic-access receiver traces | Typed receivers (T6) | Complement (candidate) | Exact value traces are a stronger claim class |
| B13 | Type-role selection | — | Retain the policy; widen roles per named consumer (T2, T6) | Consumer-driven extraction is correct |
| B14 | Flow condition translation | Decided atoms (T7) | Extend | Keeps typed atoms instead of `always()` |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence or scope reason | Required action |
|---|---|---|---|
| G1 Authority | pass | No two independently mutable definitions of one fact were found in scope. Dataclass class options (syntax) and field flags (provider) are different facts. T1 keeps N3 the single effective owner; T4 reuses `CheckedExactClass` | Keep the single owners when integrating (T1, T3) |
| G2 Semantic fidelity | unresolved | Typing-model premises in local theory publish without a distinguishable status (`EvidenceStatus` has no such member); whether the assumption reaches served Limits text was not traced (F01) | Trace (P3); F01 |
| G3 Validity | pass | No new path admits invalid state; the targets require twins under §4.3 | — |
| G4 Hidden behavior | pass | Provider configuration is constructed; ty settings come from the run context (`cpg-flow/src/lib.rs:392-404`). For T9, Pyrefly error presets must enter recorded context | Record presets (T9) |
| G5 Consistency and recovery | n.a. | No publication or lifecycle change in scope | — |
| G6 Transformation and reuse | pass | Content-addressed terms and projections unaffected; new roles add relations rather than rewriting them | — |
| G7 Truthful capability claims | **fail** | §9.9 claims typed handler matching that source completion lacks (F04). §4.2 claims empty `ProgramSettings` and an `unresolvable_dunder_all_range()` detector input that do not exist (F06). `FacetMembership` is accepted on the wire and is never decidable, contrary to §14.7 (F11) | F04, F06, F11 |
| G8 Library leverage | **fail** | Generic capabilities that the linked provider already computes are rebuilt bespoke without a current reason (§4.2.3 and §13 give "no consumer", now false): dataclass class options (F07), per-alias import resolution (F05), effective signatures (F02) | F02, F05, F07 |
| CI-G1 Fidelity | unresolved | No relabeled relation or unknown-as-absent was found. The typing assumption behind `...UnderTypingModel` theory results may not be visible where a refutation is served (F01) | P3; F01 |
| CI-G2 Evidence closure | n.a. | No served claim changes in this review; the targets must cite their new supports | — |
| CI-G3 Evaluation integrity | pass | No path from gold or evaluation references into the target facts. The `python-analyzers` skill is reference material and must not become a compiler input | — |

## 7. Findings and applicability

### Finding index

| ID | Finding | Principles · judgment / gate | Disposition route |
|---|---|---|---|
| [F01](#F01) | No owned admission policy for type-level provider facts; the typing assumption is not a modeled qualification | FP-04, DP-02, DP-05, CI-06 · A2, G2, CI-G1 | ADR + DESIGN (§3.9, §15.8); forward plan §7 "Per-claim assumption records" activated |
| [F02](#F02) | The §14.4 effective/synthesized signature roles have no fact route; the provider's decorated type is extracted and discarded | FP-04, DP-13, CI-04 · A2, G7, G8 | ADR + DESIGN (§15.5, §14.4, §4.2.3); forward plan §6.2 row |
| [F03](#F03) | Dispatch has no closure basis; the proposed basis (finality) is a typing-model assumption, while enum closure is runtime-exact | CI-06, DP-02 · A2 | Forward plan §7 closed-hierarchy row amended; Deferred under its trigger; needs F01 |
| [F04](#F04) | Exception identity is a one-member codebook; typed source handlers were narrowed at the cutover while §9.9 and the retained Phase 4 obligation claim them | FP-04, DP-22 · A2, G7 | Forward plan §6 row; §9.9 correction |
| [F05](#F05) | Intra-release import aliases cannot resolve: roots are excluded from the only relation N2 matches against | DP-02, CI-04 · G8 | Forward plan §6 row (implementation) |
| [F06](#F06) | Stale extraction claims in §4.2/§4.2.3 | DP-22 · G7 | DESIGN correction; forward plan §6 row |
| [F07](#F07) | Record-model class options are re-derived from decorator syntax although the linked provider states them | FP-04, DP-13 · A2, G8 | Forward plan §6.2 row (with T3); §15.4 amendment |
| [F08](#F08) | The §13 deferral of located types and the §4.2.3 "not carried: no consumer" rows are contradicted by named consumers | DP-16, DP-22 · G8 | ADR + DESIGN (§13, §4.2.3), per §13's own return-by-ADR rule |
| [F09](#F09) | ty predicates are erased to `always()` or opaque atoms; the 730-row trigger conflates a completion-owned certificate with a provider-decidable subset | CI-02, CI-06 · A2 | Forward plan §7 reachability row decomposed; Deferred with trigger |
| [F10](#F10) | §B1's exception and exclusion wording blocks index-level ty facts and does not state the oracle route for ty inference | DP-16 · — | ADR + DESIGN (§B1); new ADR beside ADR-0046 |
| [F11](#F11) | `FacetMembership` is a served predicate with no evaluator; §14.7 requires refusal before effects | DP-22, CI-04 · G7 | Forward plan §6.2 row (selection/serving owner) |

<a id="F01"></a>
### F01: Type-level facts have no single admission owner

**Diagnosis (Implemented state, inspected 2026-10-03).** Three consumers decide independently what
a Pyrefly type observation may establish:
- Local theory builds class and scalar domains from structural terms and returns
  `AlwaysTrueUnderTypingModel` / `AlwaysFalseUnderTypingModel` (`local_theory.rs:41-51`,
  `:1369-1372`). Its witness status derives from Flow fidelity (`:1373-1379`) into `EvidenceStatus`
  members (`StructurallyObserved`, `Documented`, `StatisticallyDerived`, `FixtureChecked`,
  `Unresolved`; `analysis/policy.rs:10-16`). None of those members represents "holds if annotations
  are truthful".
- Selection admits only declared, Exact, unconditional observations (`selection/evaluate.rs:568-597`).
- FCA admits inferred `Raised` types (`attributes.rs:495-511`).

§3.9 says "Neither a type observation nor a display string is an execution witness", yet theory
results feed conditions that can support `RefutedUnderModel`.

**Consequence.** Every behavioral target here (T3 closure, T6 premises, T7, T8) would add another
independent "may a type establish this?" decision. A refutation that rests on an annotation, for
example a guard `x is None` decided false for `x: int`, may be served without its typing assumption
(CI-06). The forward plan already defers "per-claim assumption records". Its trigger ("a served claim
misread because of an assumption the `approximated` flag does not show") describes exactly this
shape, but it has not been checked against theory-derived refutations.

**Correction (Proposed).** Add one model-owned **provider-claim admission policy**, analogous to
§15.5's named call policies. It classifies each provider relation's claim class (§3) and declares,
per question kind, whether the relation is a premise, a premise under a typing model, a candidate or
evidence. "Typing model" becomes an explicit model assumption: a qualification member or an
`EvidenceStatus`/interpretation member. It survives derivation, verdict and S0 Limits, so a
consumer can always tell a runtime-model refutation from a typing-model one. Local theory's
existing results become the first instance; they are not a special case.

**Challenge case.** A guard on a declared `x: bool` parameter evaluated as `x is None`. Under the
typing model it is refuted; under the runtime model it is unknown, because callers can pass `None`.
The policy must keep both readings, and the served claim must say which one it is.

**Closure evidence.** (1) Trace one theory-derived refutation to its S0 rendering and show the
typing assumption at each step (P3). (2) One policy owner consulted by local theory, selection and
the T3/T6/T7 consumers. No consumer may filter on `declared` independently.

<a id="F02"></a>
### F02: The effective signature roles have no fact route

**Diagnosis (Implemented, inspected).** `type_records.rs:1042-1044` iterates
`get_all_decorated_functions` and reads only `f.undecorated`. Pysa `decorator_callees` is never
read; the supply map's grep finds no hit. The §4.2.3 table marks `return_type` and related fields
"not carried: no consumer". N3 therefore cannot know an effective signature except for a lone
builtin descriptor (`callable_normalization.rs:536-612`):
- more than one decorator → `UnsupportedDecorator`;
- one other decorator → `ShadowedOrUnresolved`;
- synthesized origin → `UnsupportedNativeOrigin`.

§14.4 requires effective and synthesized roles with types. Selection `MemberKind` and
`InvocationForm` return `None` without a descriptor kind (`evaluate.rs:631-707`, per the demand map).
`InvocationForm::ContextManager` has no producer: grep finds only `selection/vocabulary.rs:126-127`
and the aspect kind at `callable_aspects.rs:805-810`.

**Consequence.** On a decorator-heavy library, every decorated public callable is effective-Unknown,
so requirements on invocation form and parameters are unresolved. This is a barrier to PR6's
comparative tasks, whose C condition is "structured contracts". The magnitude on FastMCP has not
been measured: no generations exist locally.

**Correction.** T1 and T2. **Remedy maturity:** the direction is settled. How often Pyrefly's
decorated type is structural rather than `Any` or opaque for common decorator shapes is unmeasured
(P2). That changes the size of the benefit, not the diagnosis.

**Closure evidence.** A fixture set with `functools.wraps` + ParamSpec, an untyped decorator, a
`def deco(f: F) -> F` registration decorator, `contextmanager`, and a stacked descriptor. In each
case the source variants stay unchanged, the effective-typed role is present or Unknown with reason,
and selection outcomes change only where the role is Known.

<a id="F03"></a>
### F03: Dispatch closure needs a basis, and finality is a typing-model basis

**Diagnosis.** Every dispatch assessment falls through to `DispatchReason::OpenRuntimeSubclasses`
(`dispatch.rs:600-604`). The module states that complete ancestry is never a closed universe
(`:1-2`). Neither `FunctionTraitObservation` nor `ClassTraitObservation` has a final or enum flag
(`symbols.rs:121-175`). The demand map claims finality "changes the verdict without a closed-world
assumption". It does not: `typing.final` is not enforced at runtime, so a subclass that overrides a
`@final` method is a type error that still runs. Enum classes with members are runtime-closed
(subclassing raises).

**Consequence.** Every closed-hierarchy dispatch stays open. The 2,018 pilot rows date from
2026-09-27 and generation `cdcf4b4e…`, before the cutover, and have not been re-measured. Closing
dispatch on `@final` without F01 would serve a typing-model closure as a runtime one.

**Correction.** T3's two bases: runtime-closed for enums with members and for builtins that cannot
be subclassed; declared-final under the typing model, after F01. Never close on in-corpus subclass
enumeration by default: for a library, user subclasses are the extension point.

**Disposition.** Amend the forward plan §7 "closed-hierarchy `self` dispatch" row with this fact
route and basis distinction. It stays Deferred under its trigger.

<a id="F04"></a>
### F04: Exception identity narrowed; typed source handlers lost across the cutover

**Diagnosis (Implemented, inspected; history from Git).**
- `ExactRuntimeException` has one member, `TypeError` (`execution.rs:30-42`).
- Source completion admits `raise <primitive literal>` (a runtime TypeError) and nothing else
  (`completion.rs:283-311`).
- Source `try` admits only a bare first handler. The comment reads: "Typed matching and named-handler
  disposal need their own earlier certificates" (`:356-385`).
- Typed class matching exists only for modeled context-exit suppression (`model_context.rs:46-56`,
  `context_execution.rs:463-510`).
- The legacy engine had a typed-handler migration (`efa01f3d`, 2026-09-26).
- The forward plan §3.0.1 still states "ordered pinned typed handlers and active re-raise now compose"
  (lines 664-670, historical).
- The Phase 4 plan lists "handler matching" as retained (line 429).
- §9.9 states "Handler matching uses exact pinned class/MRO evidence and ordered precedence"
  (`behavioral-analysis.md:144`).
- Current base completion was introduced in `69cb6679` (2026-10-01) with bare handlers only. No
  current plan records the narrowing (grep over the plans and sections, 2026-10-03).

**Consequence.** Escape and normal-completion claims are unavailable for any function with a typed
handler or a constructor-call raise. §9.9 overstates the implemented capability (G7). §15.13 says
deleting an engine does not retire its capability obligation.

**Correction.** T4's exact core over existing resolution facts and `CheckedExactClass`, and correct
§9.9 until it lands. No new provider fact is required. Pyrefly's `Raised` type adds a typing-model
upper bound after F01.

**Closure evidence.** Ordered typed handlers, a constructor-call raise, a subclass match, a nonmatch
against an incomplete MRO (must stay Unknown) and bare re-raise. Also, retained independent runtime
controls equivalent to the pre-cutover 100 controls, or an explicit recorded narrowing with a
trigger.

<a id="F05"></a>
### F05: Intra-release imports cannot resolve

**Diagnosis (Interface-checked by source reading; not exercised).**
- `pyrefly_stage.rs:1254` emits `DependencyModuleObservation` only for resolutions whose module is
  not a root module.
- N2 `imports` matches each alias's absolute module *name* only against those rows
  (`relation_normalization.rs:263-320`). Zero matches gives `Unresolved` / `MissingCorrespondence`
  (`:95-109`).
- The only test asserts that an import of `missing_package` is unresolved
  (`cpg-extract/tests/normalized_relations.rs:55-64`); no test covers an analyzed-to-analyzed import.
- Pyrefly's `import_handle` already resolves roots (`pyrefly_stage.rs:1043-1058`); the result is
  discarded by the filter.

**Consequence.** The internal import graph, which is most of a library's import graph, would assess
Unresolved. ImportReference projection arcs and catalog alias evidence would be lost. The outcome
is explicit (CI-04 holds), but it is a precision loss on a fact the provider states.

**Correction.** T5's per-alias resolution. **Settling check (P1):** a two-module fixture whose
modules import each other, and a look at `ImportModuleAssessment`.

<a id="F06"></a>
### F06: Stale extraction claims

| Claim | Location | Current state |
|---|---|---|
| "Known gap: ty's `ProgramSettings` are empty" | `acquisition-and-extraction.md:236` | Version and platform are built from the run context (`cpg-flow/src/lib.rs:392-404`); micro version dropped |
| Exports Partial "when `unresolvable_dunder_all_range()` is set" | `:347-349` | No call exists; only the syntax detector runs |
| Provider table rows name retired raw tables (`pysa_functions`, `syntax_nodes` …) | `:213-226`, `:336` | Typed relations are the contract |
| "`captured_variables`, `return_type` … not carried: no consumer" | `:323` | Consumers now exist (Local `CapturedStateUnavailable`; T1/T2) |

Correct the DESIGN prose directly. The last row is resolved together with F08 and F02.

<a id="F07"></a>
### F07: Record-model class options have two mechanisms

**Diagnosis.** Field flags come from Pyrefly's `ClassMetadata` (C4 in `docs/pins.md`, per the skill).
Class-level options are parsed from a single `@dataclass(...)` decorator's literal keywords, and
`order`, `frozen` and `slots` are refused when true (supply map B6; `symbolic_fields.rs`
`decorator_options`). `dataclass_transform` libraries and aliased decorators fall outside this path.

**Consequence.** One record-model concept is owned by two mechanisms, and refusals multiply where
the provider has the answer. Catalog constructor contracts are the affected consumer.

**Correction.** T3. Keep the stdlib-identity guard for behavioral initialization only.

<a id="F08"></a>
### F08: "No consumer" deferrals are contradicted

DESIGN §13 (lines 712-714) defers "located/contextual types. Reachable in-process, but no consumer
needs them". Named consumers now exist:
- `local_fields` receiver classes are limited to `TestOperand` (`local_fields.rs:346-349`);
- `read_dynamic`'s bespoke receiver traces fall back to Unknown;
- selection `Witness::ReceiverLocation` is never decided;
- catalog field-access candidates match by spelling;
- selection `ParameterType` has no types for non-source slots.

§13 itself requires return by ADR. **Correction:** an ADR that removes the deferral, scoped to the
roles in T2 and T6, with the candidate-only rule until F01.

<a id="F09"></a>
### F09: ty predicates are erased, and the 730-row trigger conflates two owners

**Diagnosis.**
- `IsNonTerminalCall` → `Condition::always()` (`predicate.rs:482`).
- `ContextManagerSuppresses` → an opaque atom (`:487-490`).
- AMBIGUOUS → `always().with_approximation()` (`:616-620`), which Local refuses as `Approximation`.

The forward plan §7 trigger treats the whole bucket as "reachability certificates". **Consequence:**
the provider-decidable subset (terminal calls, non-suppressing exits) and the completion-owned
subset (a `try` body whose earlier statements complete normally) wait on one trigger, although they
have different owners and different claim classes.

**Correction.** T7 for the first subset, under F01. Completion certificates for the second subset,
with no provider fact. Split the §7 row accordingly.

<a id="F10"></a>
### F10: §B1 needs clarification, not reversal

**Assessment of the authority (target purpose).** §B1's "never a second type checker" protects one
type authority, and this review finds that protection right (S4, §9). Its exception text, "contributes
only flow facts", is narrower than the useful ty surface in `ty_python_core`. That surface is
index-level facts with no inference: predicates, including the terminal-call and suppression nodes
already walked; reachable member bindings; closure snapshots; star-import definitions; unpack
targets; end-of-scope reachability. It also leaves the useful assurance route, ty inference as an
offline oracle, unstated.

**Proposed replacement.** "ty's semantic index (`ty_python_core`) supplies flow and other
index-level facts over the second parse, joined by exact range under parity rules. ty's type
inference is not a pipeline provider. It may serve as an offline differential oracle over Pyrefly's
type facts, outside compilation inputs, like the Pyrefly CLI parity oracle." This needs a new ADR
beside ADR-0046 (accepted ADRs are immutable) and the §B1 edit. The binding requires a design/target
review for a §B change; this review is that review for the wording.

<a id="F11"></a>
### F11: A served predicate that can never be decided

`Predicate::FacetMembership` evaluates to `Ok(None)` (`selection/evaluate.rs:1049`) and is part of
the serving request schema (`serving/schema.rs:107`). §14.7 says "Unsupported predicates are refused
before effects". Every candidate lands in "unresolved discovery", which presents "not supported" as
"not yet known".

**Correction.** Refuse `FacetMembership` at classification until an evaluator exists per facet.
Then implement `Raises` (T4), `Returns` (T2), `Decorator` (T1) and `Async` (T1) from the target, and
the delegation facets from existing structural results. **Settling:** one serving request with
`FacetMembership` is refused before effects.

### Observations (not findings)

- **O1. Skill layer error.** `python-analyzers` lists `ty.narrowing-constraints` as consumed. No
  narrowing read exists in `cpg-flow` or `ty_flow.rs` (grep, 2026-10-03). Route: the skill's
  maintainer. Under the operator's rule, a library-research worker may correct it.
- **O2. Coverage reason collapse.** Calls and Types module coverage collapse any boundary to
  `MissingEvidence` (supply map U4). The subject rows keep the precise reason, so this is not a
  CI-04 breach. A consumer that reads only coverage reasons would see coarse causes.
- **O3. Boundaries that already meet the scenarios.** §4.3's extension route, the exact-only
  cross-parse join, N3's three-way Known/Unknown/Conflicting assessment, the single binder with
  composition-admission tokens, `CheckedExactClass`, and the declared Catalog/Behavioral profile
  split are preservation constraints for every target.

**Foundation verdicts.**

| Foundation | Verdict | Basis |
|---|---|---|
| FP-01 | satisfied | Extraction, normalization and analysis owners are separate; targets land in their owners |
| FP-02 | satisfied | Provider-private types stay behind mappers; T1–T8 add contract members, not exposed internals |
| FP-03 | satisfied | Every target composes through an existing operation (§4) |
| FP-04 | **violated** | F01 (no owner for type-premise admissibility), F04 (domain encoded as a one-member codebook), F07 (two mechanisms) |
| FP-05 | satisfied, with F01 caveat | Coverage and boundaries are explicit; the typing assumption is not |
| FP-06 | satisfied | Each target is testable through pure model operations with fixtures |

## 8. Library fit and total complexity

| Capability and owner | Consumer | Candidates | Pinned semantic fit and gaps | Coupling / lifecycle burden (semantic only) | Choice |
|---|---|---|---|---|---|
| Effective callable type (N3) | catalog, selection | Pyrefly decorated type; ty `decorated-types` (needs inference); bespoke recognition | Pyrefly is on the parse of record and in both profiles; doc-hidden session API moves between dev releases | Fact-diff at each pin move | Pyrefly (T1) |
| Class metadata (dispatch, catalog) | T3 | Pyrefly `ClassMetadata`; ruff `class-kinds` (needs `Checker`); ty (internal) | Pyrefly states the typing spec's interpretation; runtime closure differs for `@final` | Per-field basis needed | Pyrefly with per-field basis |
| Import resolution (N2) | T5 | Pyrefly `import_handle` (already called); bespoke name match | Exact resolution in the recorded environment | none new | Pyrefly |
| Exception identity (completion) | T4 | Existing resolution facts; Pyrefly raise/handler types; ty raise checks | Resolution is exact; types are bounds | none new for the core | Existing facts + model operation |
| Argument binding (N5) | binder | ty argument map; Pyrefly inlay names | ty's map needs inference; Pyrefly has no data map | Second authority | Retain bespoke |
| Type assurance | all type consumers | ty inference as provider or as oracle | Environment and gradual-typing differences | In-pipeline: reconciliation burden in every consumer | Offline oracle only (F10) |
| Flow predicate decisions | Local | Pyrefly `CallResult` `Never`, `__exit__` type; ty `ty_python_semantic` resolution | Pyrefly is on the parse of record; join by range | needs F01 | Pyrefly (T7) |

## 9. Alternatives and tradeoffs

| Alternative | Change propagation | Semantic authority and composition | Test / substitution | Total machinery and risk | Decision |
|---|---|---|---|---|---|
| Current baseline | Each new question re-derives from syntax (B5, B6, B8) | Owners are clear; inputs are thin | Fixture-testable | Rising bespoke recognizers; catalog unknowns on decorated or synthesized members | Revise |
| **Proposed: T1–T9 through existing owners, plus F01's policy** | One declaration plus provider wiring per fact (§4.3); behavioral uses wait on F01 | Single owners kept; admissibility gains one owner | Pure operations plus twins | Pin-sensitive provider facts need a fact-diff at each upgrade | **Recommended** |
| Library-owned: ty inference as a second type provider (S4) | Every type consumer reconciles two providers | Disagreement is a real CI-01 obligation, but uninterpretable across environment and gradual-typing differences | Dual-provider fixtures | A second type authority in the pipeline | Rejected for the pipeline; adopted as an offline oracle. **Revisit:** a measured class of Pyrefly type defects that the oracle catches and that changes served claims |
| Simplest viable: T1, T2, T3 (catalog fields only), T5, plus the F04/F06/F11 corrections | Catalog-only; no behavioral premise change | No F01 dependency | Fixture-testable | Smallest | Acceptable first slice; leaves G1/G5/G8-demand behavioral gaps open |

The recommendation would change if one of these premises fails:
- **P2:** Pyrefly's decorated types are mostly `Any` or opaque for real decorators. T1's value would
  then shrink to registration-style decorators.
- **Unmeasured volume:** typing-model premises turn out to need per-claim assumption records too
  costly to render. T3, T6, T7 and T8 would then stay candidate-only.

## 10. Verification and uncertainty

| Claim or scenario | Evidence label / date | Method | Conditions and expected result | Current outcome |
|---|---|---|---|---|
| Diagnoses F02, F03, F04, F07, F09, F11 | Implemented, inspected 2026-10-03 | Source reading at cited lines | — | read |
| F05 intra-release import | Interface-checked 2026-10-03 | Source reading | **P1:** two-module fixture → `ImportModuleAssessment` Resolved expected after T5, Unresolved before | not_run |
| T1 benefit size | Proposed | **P2:** Pyrefly decorated-type probe over the five decorator shapes in F02 | Structural vs `Any`/opaque per shape | not_run |
| F01 / CI-G1 | unresolved | **P3:** trace one theory-derived `AlwaysFalseUnderTypingModel` refutation to S0 Limits text | The typing assumption is visible, or it is not | not_run |
| Pilot magnitudes (2,018 dispatch, 730 AMBIGUOUS) | Historical, 2026-09-27, pre-cutover | Plan records | Re-measure after real-library qualification resumes | stopped by operator |
| Provider surfaces at pyrefly 1.4.0-dev.3 / ty 0.0.14 | Interface-checked by the supply catalogue, 2026-10-03 | Item presence and visibility | Behavior at ty/ruff 0.0.14 unverified | — |

The missing premise that could change the *order* of the target is P2. Nothing found here changes
the diagnoses. Unexamined breadth (the roughly 270 integrity sites, the Python serving layer) is not
a defect.

## 11. Authority changes and dispositions

**Current disposition (binding §4), recorded by the coordinator at publication, 2026-10-03:** every
finding below is **Deferred in this review**. The routes are proposed, and none has been transferred
to a plan or ADR yet. Revisit trigger: the operator schedules the integration target (the
feasibility follow-up), or a selected product task needs one of these findings' consumers. On
transfer, the receiving plan row or ADR becomes the single disposition owner, and this section
links to it.

| Required change | Route and owner | Source findings | Disposition location | Closure evidence or revisit trigger |
|---|---|---|---|---|
| Provider-claim admission policy; typing model as an explicit assumption | ADR + DESIGN §3.9, §15.8; `lctx-model` analysis policy | F01, F03, F09 | Forward plan §7 "Per-claim assumption records" (activate) and a new §6 row | P3 trace; one policy owner |
| Effective-typed and synthesized signature roles | ADR + DESIGN §15.5, §14.4, §4.2.3 | F02, F06 | Forward plan §6.2 row (catalog owner) | F02 fixtures |
| Remove the §13 located-types deferral | ADR + DESIGN §13, §15.4 | F08 | Forward plan §6.2 row | Roles with named consumers implemented |
| §B1 clarification (index-level ty facts; inference as offline oracle) | New ADR beside ADR-0046 + DESIGN §B1 | F10 | ADR | ADR accepted; no pipeline dependency on `ty_python_semantic` |
| Restore typed source handlers; correct §9.9 meanwhile | Forward plan §6 row (execution owner) | F04 | Forward plan §6 | F04 controls |
| Per-alias import resolution | Forward plan §6 row (extraction/N2) | F05 | Forward plan §6 | P1 |
| Stale §4.2 claims | DESIGN edit | F06 | Forward plan §6 row (doc) | Edited text matches code |
| Record-model class options from provider | §15.4 amendment; forward plan §6.2 row | F07 | Forward plan §6.2 | Option twins incl. `dataclass_transform` |
| Dispatch closure bases | Forward plan §7 row amendment | F03 | Forward plan §7 (Deferred) | Trigger unchanged; basis distinction recorded |
| Split the reachability trigger | Forward plan §7 row amendment | F09 | Forward plan §7 (Deferred) | Graded item depends on either subset |
| Refuse or implement `FacetMembership` | Forward plan §6.2 row (selection/serving) | F11 | Forward plan §6.2 | Refusal before effects |

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence and scope | Required action |
|---|---|---|---|
| A1 Localize change | unresolved | S1a/S1b, T5 and T2 follow §4.3's route with propagation only to their owners. For T6/T7/T8 premises, each consumer would decide admissibility itself until F01 exists | F01 |
| A2 Encode domain meaning explicitly | **violated** | The typing-model assumption is not a represented distinction (F01). Exception identity is a closed name codebook (F04). Record-model options have two mechanisms (F07) | F01, F04, F07 |
| A3 Extend through composition | satisfied | Every target reuses an existing operation (N3, dispatch, `CheckedExactClass`, theory domains, selection, FCA); no new engine or framework | — |

**Bounded decision: Revise.** G7 and G8 fail, A2 is violated, CI-G1/G2 are unresolved. The
integration target is accepted as the recommended direction at **Proposed** strength, subject to
F01's decision before any behavioral use of type-level facts.

**Enclosing architecture:** the extraction → normalization → analysis/catalog architecture is
accepted for the *add a fact family* scenario through §4.3 (Interface-checked). It needs revision
at the admission policy (F01) and the exception domain (F04). This is not release qualification;
real-library qualification remains stopped.

| Priority | Change and responsible component | Source findings | Closure evidence or revisit trigger |
|---|---|---|---|
| 1 | Truthful claims: §9.9 text or restored typed handlers (execution); stale §4.2 prose; `FacetMembership` refusal (selection) | F04, F06, F11 | F04 controls; refusal test; edited text |
| 2 | Effective-typed and typed signature roles (extraction, N3, catalog, selection, retrieval) | F02, F08 | F02 fixtures, P2 |
| 3 | Class metadata incl. deprecation, enum domains, abstract and record parameters (extraction, `symbolic_fields`, catalog) | F07 | Option and constructor twins |
| 4 | Per-alias import resolution (extraction, N2) | F05 | P1 |
| 5 | Exception identity core (execution) | F04 | F04 controls |
| 6 | Admission policy and typing-model qualification (`lctx-model` analysis policy) | F01 | P3; then T3 closure, T6 premises, T7, T8 |
| 7 | §B1 clarification ADR; ty inference as an offline oracle | F10 | ADR |

**Prerequisite order differs from priority.** F01 is priority 6 by consequence, but it is the
prerequisite for the behavioral half of T3 and for T6 premises, T7 and T8. It is not a prerequisite
for anything in priorities 1–5.

**The next consequential decision** is F01's ADR: whether behavioral claims may rest on a stated
typing model, and how that model is carried to served claims. It is the root agent's decision, and
it settles the behavioral half of the target. The catalog-facing half (priorities 1–5) can be
planned now. The feasibility follow-up the operator described should cost T1, T2, T3 and T5 first.

---

**Summary.**
- *Scope:* a design/target review of additional pyrefly/ruff/ty facts and their downstream use, at
  `6885dbf6` under the assumed migration. Static only.
- *A1–A3:* A1 unresolved, A2 violated, A3 satisfied.
- *Gates:*
  - pass: G1, G3, G4, G6, CI-G3;
  - fail: G7, G8;
  - unresolved: G2, CI-G1;
  - n.a.: G5, CI-G2.
- *Material findings:*
  - F01 (typing-model admission);
  - F02 (effective signatures);
  - F04 (typed handlers lost; §9.9 overstated);
  - F05 (intra-release imports);
  - F11 (undecidable served predicate).
- *Bounded decision:* Revise; target direction Proposed.
- *Enclosing architecture:* accepted for the fact-family extension route; needs revision at
  admission policy and exception domain.
- *Evidence limits:* no generations, probes or pilots. Provider behavior at ty/ruff 0.0.14 is
  unverified. Pilot magnitudes predate the cutover.
- *Intended path:* `docs/design_review/reviews/design_review_code-facts-opportunity_2026-10-03.md`.
