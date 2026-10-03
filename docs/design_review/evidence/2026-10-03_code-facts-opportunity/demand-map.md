# Demand map: where downstream layers are limited by code facts

Baseline: `main` at `6885dbf6` (clean). Read-only static inspection on 2026-10-03; no builds,
tests or generations. Paths are relative to the repository root; `D/` means
`crates/lctx-model/src/domain/`. The cpg-core files named in the brief (`semantic_*.rs`,
`local_semantics.rs`, `catalog_*.rs`, `analytic*.rs`, `synthesis*.rs`, `retrieval*.rs`) are stage
adapters. They load relations, run the model-owned operation and publish. For example,
`crates/cpg-core/src/catalog_core.rs:1` says "C0 execution consumes completed native/normalized
authorities". The semantics, and therefore every refusal below, live in `lctx-model::domain`.

**Method.** I grepped every `ObligationKind` emission and every per-layer reason codebook
(`DispatchReason`, `CallableReason`, `AuthorityReason`, `ReceiverReason`, `TheoryReason`,
`FieldLocationReason`, `ProjectionGapReason`, `PublicPathBoundaryReason`, selection `Reason`)
under `D/`, then read the source behind the clusters that involve provider facts. Most of the
roughly 270 `MissingEvidence` sites check row integrity: a cited premise row is absent, as in
`D/conditions/entry.rs:167`. They are not a missing provider fact, and I excluded them. I verified
the skill's boundary map (`.claude/skills/python-analyzers/content/project/boundaries.md`,
written at `761687bd`) against current code, and note corrections where it differs.

**Pilot counts.** No stored generations exist locally. Both counts come from forward plan §7, and
`git log -S` dates their origin:

- 2,018 `override_dispatch` rows: added 2026-09-27 in commit `50969ba0`, which records an
  "informal Stage 3 diagnostic of generation cdcf4b4e519e8b79".
- 730 `missing_evidence` rows: added 2026-09-27 in commit `8a7c9367` (P0.7).

Both predate the cutover model. In the current model the 730-row cause has a different code; see
G5.

Labels: **Observed** means read in source at this baseline. **Interpretation** is my reading of
the consequence.

---

## A. Behavioral analysis (Local → BaseEvaluation → BaseCompletion → SourceCall → Enriched → Model → Summary)

### A.0 Fact relations consumed (Observed)

**Flow (ty, through cpg-flow), `D/flow.rs`:**

- `FlowUse`, `FlowDefinition` and `FlowUseObservation`.
- `FlowDefinitionObservation`.
- `FlowReachingObservation`, with `ReachingDefinition::{Bound, Unbound, Nested}` and
  `loop_carried`.
- `FlowValueObservation`, with sink kind and `through_call`.
- `FlowRegionObservation`: reachability is carried in `qualification.condition`.
- `FlowTestObservation` and `FlowTestLeafObservation`: test leaves to `EvaluationAtom`.
- `FlowAttributeLoadObservation`: native name only.
- `FlowValuePathObservation` with `FlowCallStep`.

**Types (pyrefly), `D/types.rs`:**

- `TypeObservation` with `TypeRole ∈ {Parameter, Return, CallResult, Argument, Raised,
  TestOperand}` (`D/types.rs:679`).
- `TypeTerm`.
- `FunctionBodyObservation`.
- `RecordFieldObservation`.

**Signatures and symbols (Pysa/pyrefly), `D/symbols.rs`, `D/calls.rs`:**

- `FunctionTraitObservation`: overload, static, class, property, stub, origin,
  `defining_class`, `overrides` (`D/symbols.rs:119`).
- `ClassTraitObservation`: synthesized, dataclass, named_tuple, typed_dict (`D/symbols.rs:166`).
- `ClassAncestryObservation`: Bases/Mro plus `Linearization`.
- `Signature`, `SignatureParameter` and `ParameterShape`: shape only, no types
  (`D/calls.rs:253-302`).
- `SignatureEnumerationObservation`.

**Calls (Pysa), `D/calls.rs:581-668`:**

- `CallTarget`, with `receiver_class`, `passing` and `class_method`/`static_method`.
- `CallDestination::{Resolved, Overrides, Unresolved}`.
- `PysaUnresolvedReason`.

**Lexical and syntax (ruff):** occurrences, placements, `LexicalResolution`/`LexicalTarget`
(Builtin/Binding), `BindingObservation`, `DeclarationDecorator`, `ImportAliasObservation`,
`SyntaxDetail` literals and operators.

### A.1 Fact-shaped gaps

#### G1. Dispatch closure for `self.m()` / `Overrides` destinations

- **Consumer and claim.** The consumers are `D/normalized/dispatch.rs::assess` (lines 437-656)
  and `D/composition.rs:687` (`CallComposition::Obligation(OverrideDispatch)`). In the
  projection, a dispatch arc becomes a gap (`D/projection/normalization.rs:514-521`). Every
  assessment is written `open: true`, and its floor reason is
  `DispatchReason::OpenRuntimeSubclasses` (`dispatch.rs:604`). An open override therefore
  blocks negative closure, finite Summary composition through the call, and any refutation over
  that call. Forward §7 records 2,018 pilot rows (2026-09-27, `50969ba0`).
- **Missing fact.** The provider already supplies the candidate set. Members are built from
  `FunctionTraitObservation.overrides` chains plus complete MROs, so that part is not
  re-derived. What no relation states is a closure basis:
  1. Finality: `@final` or `typing.final` on a class or method. `ClassTraitObservation` and
     `FunctionTraitObservation` have no final flag (`D/symbols.rs:119-175`). `TypingForm::Final`
     (`D/types.rs:77`) covers only `Final[...]` annotations.
  2. A complete, attributed enumeration of in-corpus subclasses per receiver class: a reversed
     override and subclass index with its own coverage scope.

  Secondary reasons that also need facts: `MissingReceiverClass` (the Pysa target has no
  `receiver_class`) and `IncompleteAncestry` (MRO `Prefix`/`Cyclic`).
- **Re-derivation.** None. The open floor is a modelling choice; the design says "complete
  captured ancestry is never a closed runtime subclass universe" (`dispatch.rs:1-2`). Finality is
  the only provider fact that changes the verdict without a closed-world assumption.

#### G2. Receiver type at arbitrary expressions

- **Consumers and claims.**
  1. `D/local_fields.rs:346-349`. `FieldLocationCandidate` admits a receiver type only from a
     `TypeObservation` with `role == TypeRole::TestOperand` at the receiver placement. An
     attribute whose receiver is not also a condition operand gets no class candidate:
     `FieldLocationReason::MissingReceiverClass`, `D/local_fields.rs:18-26`.
  2. `D/execution/read_dynamic.rs::trace/constructed_class` (lines 520-707) infers a dynamic
     access's receiver class by walking ty reaching definitions to the method's first parameter
     (`ReceiverDeclaration`) or to a unique module-global `x = Cls(...)`
     (`UniqueGlobalInitializer`). Anything else becomes `ClassInspection::Unknown` with
     `ObligationKind::DynamicAccess` (`read_dynamic.rs:732-750`).
  3. Selection `Witness::ReceiverLocation` is never decided (`D/selection/evaluate.rs:1039-1043`).
- **Missing fact.** The provider's inferred type of the receiver or attribute-base expression,
  attached by exact range: a `TypeRole` for attribute bases or receivers, or a general
  expression-type observation. A separate declared-versus-inferred flag is also needed.
  `TypeRole` has no such role (`D/types.rs:679-686`).
- **Re-derivation.** Yes. `read_dynamic.rs` rebuilds receiver class from syntax and flow. This is
  forward §7 "Pyrefly-typed receivers for dynamic access" (trigger: a refutation on a field read
  through a typed non-`self` receiver).

#### G3. Attribute → declaration resolution (field identity)

- **Consumers and claims.**
  1. `D/local_fields.rs:396-415`: candidate fields match
     `f.class == class_entity && f.name == load.name` on the one receiver class. There is no MRO
     walk, so inherited fields are missed (Interpretation).
  2. `D/execution/read_fields.rs:1-4`: "A same-name load screens every class; unresolved dynamic
     receivers screen all classes". Name-keyed screening therefore withholds negatives broadly.
  3. `D/catalog/evidence/fields.rs:68-95`: catalog field-access candidates match attribute
     identifier spelling against `field.name` inside the class's methods.
  4. `D/normalized/symbolic_fields.rs:1064,1196,1370,1434`: stores and readers are matched by
     spelling.

  This limits `HeapFieldStateUnavailable` and `FieldReadAssessment` negatives, catalog
  `FieldRelationship::{ExactStorage, ExactReader}` and selection `ConfigurationRelationship`.
- **Missing fact.** For each attribute load or store occurrence: the declaration it resolves to
  through attribute lookup (class field, inherited field, method, property, descriptor,
  `__getattr__` fallback), with its defining class and kind. This separates a field from a method
  of the same name; forward §7 lists "inherited methods counted as field reads".
- **Re-derivation.** Yes, by spelling match. The design keeps the results as candidates, not
  identity (`D/catalog/evidence/runtime.rs:1`).

#### G4. Raised exception class and handler class sets (exception completion)

- **Consumer and claim.** `D/execution/completion.rs:240-400`.
  - The exception domain is `ExactRuntimeException` with one member, `TypeError`
    (`D/execution.rs:33-43`).
  - `raise <expr>` is admitted only when `<expr>` is a primitive literal, giving a TypeError
    (lines 283-311). Every other raise is `UnsupportedControlFlow`.
  - `try` admits only the first handler, and only a bare `except:` (lines 357-385). A typed
    handler is `UnsupportedControlFlow`, per the comment "Typed matching and named-handler
    disposal need their own earlier certificates".
  - In the modeled context protocol (`D/execution/context_execution.rs:463-470`,
    `D/execution/model_construction.rs:570-586`), the raised class is found by
    **symbol name lookup** (`s.name == "TypeError"` in the typeshed `builtins` bundle). The
    handler class is resolved from a lexical reference name to a builtins symbol. Only bare
    `ExprName` builtin handlers are supported.

  Limits: escape and normal-completion claims for any function with a non-literal raise or a
  typed handler; `with`-suppression; Summary exception paths.
- **Missing facts.**
  1. Per `raise` site: the resolved exception class (instance or class object), as a
     `ProviderSymbol` with complete MRO.
  2. Per `except` clause: the resolved class tuple of its type expression.
  3. Per call site: a may-raise set. No provider supplies one; the skill lists
     `exceptions.propagation` as blank.

  `TypeRole::Raised` exists, but its only consumer is analytics (`D/analytics/attributes.rs:495`).
  Completion, structural controls and synthesis never read it.
- **Re-derivation.** Yes. Class identity comes from name lookup in builtins, and raise
  classification from syntax kind.
- **Plan drift.** The forward plan §3.0.1 historical text (`behavioral-model-forward-plan`,
  around lines 663-666) says "Ordered pinned typed handlers and active re-raise now compose that
  exact TypeError". In the current tree, a grep for `F::Handler` and `ExceptHandlerExceptHandler`
  under `D/` finds handler handling only in `completion.rs`, and only for bare handlers. Either
  typed handlers were not carried through the cutover, or they live outside `D/`; I found no
  evidence of the latter.
- Cross-call composition is forward §7 "Cross-call exception composition". No producer emits a
  source-level exception channel: a grep finds `AnalysisChannel::Exception` (`D/analysis/config.rs:159`)
  only in its declaration, and exceptions appear only in authored models
  (`D/execution/model_rules.rs:423`).

#### G5. Reachability and static truth

- **Consumers and claims.**
  1. cpg-flow lowers ty's `AMBIGUOUS` reachability to `Condition::always().with_approximation()`
     (`crates/cpg-flow/src/predicate.rs:616-620`). Local then refuses any non-Exact value
     observation with `ObligationKind::Approximation` (`D/local_semantics.rs:189-190`). This is
     the 730-row bucket in forward §7: try bodies, with exits, loops over unknown iterables,
     including Q03.d and Q05.b. At recording (2026-09-27) the code was `missing_evidence`; at
     this baseline it is `Approximation`.
  2. Completion `if` and `elif` require an exact closed-expression truth value, otherwise
     `UnsupportedControlFlow` (`completion.rs:313-345`).
  3. Retained plan item 2 (forward §3.0.1, around line 597) says "ty's `end_of_scope_reachability`
     may add one append-only implicit-exit kind". It is not implemented.
- **Missing facts.** A per-region reachability certificate that decides an ambiguous region:
  dead after a NoReturn call, a non-suppressing context exit or an impossible handler. The
  provider's static truthiness per test (ALWAYS_TRUE/ALWAYS_FALSE) as an attributed fact
  separate from evaluation. Implicit end-of-scope reachability.
- **Re-derivation.** The closed-expression evaluator decides truth itself (`D/execution/evaluation.rs:560-790`).
  Static provider truth is not consumed except through ty's diagram constants.

#### G6. Effective (decorated or synthesized) callable identity and signature

- **Consumers and claims.** `D/normalized/callable_normalization.rs:536-612`. Effective identity
  is `Unknown` for:
  - more than one decorator (`UnsupportedDecorator`);
  - one decorator other than an exact bare builtin `staticmethod`, `classmethod` or `property`
    (`ShadowedOrUnresolved`, from `descriptor()` at lines 249-321, matched by builtin name);
  - a provider origin other than `DefStatement` (`UnsupportedNativeOrigin`, line 583), which
    covers every synthesized `__init__` and callable field.

  The `validate_assessment` invariant (`D/normalized/callables.rs:84-95`) makes `identity=Known`
  require `descriptor` and `signatures` Known. Downstream:
  - Binding authority becomes `AuthorityReason::{EffectiveUnknown, DescriptorUnknown}`
    (`D/normalized/signature_applicability.rs:21-33`).
  - SourceCall requires a source function, else `NoSourceDeclaration`
    (`D/execution/source_call.rs:250-252`).
  - Receiver assessment returns `ReceiverReason::DescriptorUnknown`.
  - Selection `MemberKind` and `InvocationForm` return `None` when `descriptor_kind` is None
    (`D/selection/evaluate.rs:631-707`).
  - Catalog `CatalogCallable` carries the Unknown assessment.
- **Missing fact.** The provider's effective callable type after decorator application: the
  decorated signature, ParamSpec and Concatenate propagation, `functools.wraps` preservation,
  and descriptor kind. Also the synthesized member's typed signature: dataclass, attrs and
  pydantic `__init__`, with alias, kw_only and default per parameter. Today `Signature` is
  Pysa's *undecorated* signature list (`D/calls/signature_enumeration.rs:1`).
- **Re-derivation.** Yes. Descriptor kind is recognized by bare builtin spelling. Aspect kinds
  (`D/normalized/callable_aspects.rs:784-835`) match resolved decorator symbol name plus module
  allowlist, and are `MetadataOnly`.

#### G7. Argument → parameter binding beyond plain positional and keyword calls

- **Consumer and claim.** The sole binder is `D/calls.rs::bind/assign_arguments` (lines 1446-1600),
  through `D/normalized/binding_normalization.rs:294-336`. It refuses:
  - `*args` or `**kwargs` at the call site (`UnsupportedUnpacking`, `calls.rs:1556-1562`);
  - non-`List` signature forms, meaning ParamSpec or `...` (`OutsideProviderModel`, line 1550);
  - a non-Direct call channel (`CallTransfer`, line 1462);
  - more than 128 actuals.

  An Undetermined outcome blocks SourceCall, model application, Summary composition and
  `ConstructorCandidateLink`. Selection must not contradict keyword acceptance under `**kwargs`
  (api-and-evidence §14.7).
- **Missing facts.** A provider per-call, per-variant argument→parameter mapping, including
  unpacked operands and `Unpack[TypedDict]` kwargs. Provider call-checking diagnostics as
  evidence of proven incompatibility: bad argument count, unexpected keyword, missing argument.
- **Re-derivation.** Yes. The binder reimplements Python's binding over Pysa shapes; this is
  intended as the semantic authority. A provider mapping would be corroborating evidence.
  Forward §7 lists "`**kwargs` binding TypeErrors" and "an unmapped argument at a multi-callee
  site".

#### G8. Exact values of non-literal expressions

- **Consumer and claim.** The closed evaluator (`D/execution/evaluation.rs:569-790`) handles
  literals, names (through an entry witness or a builtin read), unary not and ±, binary + and −,
  tuples, bool ops and if-expressions. `ExprAttribute` always yields `HeapFieldStateUnavailable`
  (line 597). Everything else is `UnsupportedControlFlow` (line 199). This limits guard truth,
  default values and argument values, so module constants, `Enum.MEMBER` and `Final` attributes
  stay open.
- **Missing fact.** The provider's Literal or enum-literal type at an expression, kept as a
  distinct "declared/inferred literal type" basis and never as an evaluation. The design forbids
  treating a type as an execution witness (behavior-model "The runtime view").
- **Re-derivation.** Yes, by design. Value semantics are owned here.

#### G9. Record-model class metadata and constructor targets

- **Consumers and claims.** `D/normalized/symbolic_fields.rs:640-800` recognizes `@dataclass`
  through import-alias spelling (`"dataclasses"`, `"dataclass"`). It then parses decorator
  keyword literals itself: `init`, `kw_only`, `repr`, `eq`, `order`, `unsafe_hash`, `frozen`,
  `slots`, `weakref_slot`. `field(default_factory=…)` is recognized by syntax (line 923). Context
  binding needs exactly one Pysa `Init`-phase target, else `UnresolvedTarget`
  (`D/execution/context_binding.rs:305-317`). This limits catalog constructor contracts
  (`ConstructorOrigin::Synthetic`, `D/catalog/build.rs:435`), symbolic constructor→field→reader
  links and record initialization (forward §7).
- **Missing fact.** Class-level record-model parameters: dataclass `init`, `frozen`, `slots`,
  `kw_only`, `order`, `eq`, `dataclass_transform` field specifiers, and pydantic config. Also the
  synthesized `__init__` parameter list.
  - `ClassTraitObservation` carries only `dataclass/named_tuple/typed_dict/synthesized` booleans.
  - `RecordFieldObservation` carries per-field flags (`D/types.rs:1494-1515`) but no class-level
    parameters.
- **Re-derivation.** Yes. Dataclass options are parsed from decorator syntax.

#### G10. Captured variables and closures

- **Consumer and claim.** `D/local_semantics.rs:220-226`: a sink whose owner differs from the
  use owner gets `CapturedStateUnavailable`. `D/execution/definition.rs:165` does the same.
- **Missing fact.** A per-function set of captured and free variables, mapped to the defining
  scope's cell. The skill's `pysa-variants` opportunity says Pysa's `captured_variables` are not
  carried.
- **Re-derivation.** None.

#### G11. `isinstance` and `type()` predicate meaning

- **Consumer and claim.** `D/local_theory.rs:400-686`. Theory re-derives the predicate from call
  syntax: an exact positional actual count plus a lexical `Builtin` resolution named
  `"isinstance"` or `"type"`. `type()` admits only `str`, `int` and `bool`
  (lines 586-589). A tuple classinfo is unsupported (exactly one `ClassObject` argument type is
  required). The class domain closes only as `FiniteUnderTypingModel` versus
  `OpenClasses`/`OpenClassUniverse`/`IncompleteMro` (`local_theory.rs:23-45`).
- **Missing facts.** The provider's narrowing operation per test leaf, with its resolved class
  set (including tuples and unions). Class finality, to close a nominal domain; this shares the
  gap with G1.
- **Re-derivation.** Partly. Class identity comes from the `Argument`-role `ClassObject` type,
  which is a typed fact. The predicate shape is syntactic.

#### G12. Protocol member lookup (modeled context protocols)

- **Consumer.** `D/execution/model_context.rs:293-314` finds `__enter__` and `__exit__` by
  appending `".__enter__"` to the model target's callable name and matching symbols. `optional()`
  maps `MissingEvidence` to `None`.
- **Missing fact.** Class attribute resolution through the MRO for dunder protocol members.
- **Uncertainty.** I did not trace whether `entry`/`exit == None` changes a verdict or only drops
  premises. An inherited `__enter__` would not be found by name. Flagged for the reviewer, not
  asserted as a defect.

#### G13. Unresolved and annotation call targets; body availability

- `UnresolvedTarget` is the default when Pysa reports no callees (`D/calls.rs:2064`). Its native
  reasons are `PysaUnresolvedReason` (`D/calls.rs:581-596`). Missing fact: the callee
  expression's type by range as a fallback route. Annotation calls stay `OutsideProviderModel`.
- `AbstractBody` comes from `CallableReason::BodyExcluded` (`D/execution/read_channels.rs:759`),
  and `NoSourceDeclaration` from `composition.rs:126,509` and `source_call.rs:250`. These need no
  new fact: the boundary is true. Forward §7 "Native-extension bodies" is the only trigger, and
  stub-only module origin is already modeled.

#### G14. Captured provider qualifiers that drop rows

`ObligationKind::Approximation` (`local_semantics.rs:189`, `source_call.rs:113`) and
`IncompatibleContexts` (44 sites) apply when a provider row's modality is not Definite or its
approximation is not Exact. Interpretation: when a provider fact arrives Candidate or Over (for
example ty AMBIGUOUS, or a Potential bare decorator call at `symbolic_fields.rs:722`), downstream
drops it rather than weakening it. Richer qualifiers alone would not change these verdicts; the
underlying fact would have to become exact.

---

## B. Analytics

### B.0 Relations consumed (Observed)

**Projections** (`D/projection.rs:17-69`; `D/projection/normalization.rs:470-560`):

| Projection | Source relation |
|---|---|
| CallableInvocation | `NormalizedCallAlternative` |
| DefinitionContainment | `OccurrenceOwnership` |
| ImportReference | `ImportModuleCandidate` |
| PublicExposure | `PublicExposureCandidate` |

Reference arcs come from `ReferenceEntityCandidate`.

**FCA attributes** (`D/analytics/records.rs:265-294`; derivation in
`D/analytics/attributes.rs:440-548`):

| Attribute | Source |
|---|---|
| `Parameter` | ruff `ParameterSyntaxObservation` |
| `ParameterType` | `TypeObservation(Parameter, declared=true)` |
| `Returns` | `TypeObservation(Return, declared=true)` |
| `Raises` | `TypeObservation(Raised)` at raise sites owned by the entity, single class only |
| `Decorator { expression: Utf8Text }` | decorator expression spelling |
| `Calls` / `Handoff` (RCA) | normalized call alternatives and structural handoffs |

**Community layers** (`D/analytics/records.rs:81-113`): Invocation arcs, CoUse (usage evidence),
Type (declared parameter types that share a source class; `attributes.rs:215-262`), Mention
(document mentions) and Nearest (embeddings).

### B.1 Gaps

- **AN1, invocation topology holes.** Arcs exist only for selected, resolved alternatives.
  Unresolved, ambiguous, open-dispatch and uncertain events become `ProjectionGapReason`
  records, not arcs (`D/projection/normalization.rs:487-552`; codebook `D/projection.rs:202-214`).
  `DispatchMember` candidates are never arcs. Centrality, communities, delegation traversal and
  RCA `Calls` attributes see only resolved edges. Facts that would change this: G1 (closure or
  finality) and G2/G13 (typed receivers and callee types for unresolved sites). A typed
  candidate-arc layer would also need its own modality.
- **AN2, declared types only.** `ParameterType`, `Returns` and the Type layer require
  `declared=true`. Unannotated parameters and inferred returns contribute nothing. `CallResult`
  types are produced but have no analytic or behavioral consumer (grep of `TypeRole::` uses,
  `D/` plus `crates/cpg-core` and `crates/lctx-analytics`).
- **AN3, Raises is direct-raise only.** It records the class at a `raise` statement inside the
  entity, not exceptions that escape through calls, and only single-class terms. G4's may-raise
  facts would turn it into an escape attribute.
- **AN4, Decorator attribute is spelling.** `Attribute::Decorator { expression }` uses rendered
  expression text (`attributes.rs:525-547`; the comment reads "Exact decorator symbol spellings").
  Aliased imports of one decorator split into separate attributes; same-spelled different
  decorators merge (Interpretation). Missing fact: resolved decorator target identity. It exists
  as `AspectSource::Decorator.target/resolution` in normalized aspects, but FCA does not use it.
- **AN5, no class-relationship projection.** `ProjectionName` has no inheritance, override or
  type-reference graph, though `ClassAncestryObservation` and `FunctionTraitObservation.overrides`
  exist. Interpretation: grouping and centrality ignore type structure, except through the
  optional Type layer.

---

## C. Catalog, selection, synthesis and retrieval

### C.0 Contracts produced from code facts (Observed)

- **Members, exposures, candidates and paths** (`D/catalog/mod.rs:19-77`, `:225-262`): from
  normalized public exposures, lexical bindings and `ClassAncestryObservation` for inherited
  paths.
- **CatalogCallable, CatalogInvocation and CatalogCallableAspect:** from
  `EffectiveCallableAssessment`, `SignatureVariant` (Pysa undecorated signatures plus descriptor
  `SignatureAdjustment`) and normalized aspects.
- **CatalogConstructor** (Own/Inherited/Synthetic; Init/New): from traits and MRO.
- **CatalogOption, CatalogDefault, CatalogOptionEvidence:** from parameter syntax, native slots,
  `RecordFieldObservation` and `FieldDefaultAssessment`.
- **C1 evidence** (`D/catalog/evidence/*`): scenarios, intent (resolved Pysa destinations plus a
  pytest/unittest name allowlist, `intent.rs:7-80`), field-access candidates, field-location and
  constructor-candidate links (always `HeapFieldStateUnavailable`, `runtime.rs:106,201`) and
  `SourceFieldLink` (symbolic associations).

### C.1 Gaps

- **CA1, effective signatures and invocation forms.** These are Unknown under any non-builtin
  decorator or for synthesized members (G6). Selection `MemberKind` and `InvocationForm` are
  then unresolved (`evaluate.rs:657-705`). `InvocationForm::{ContextManager,
  AsyncContextManager}` (`D/selection/vocabulary.rs:120-128`) has no producer. The
  `contextmanager` aspect (`callable_aspects.rs:802-806`) is not consulted, so these forms are
  never Supported. A grep for both variants finds only the vocabulary. The vocabulary also has no
  awaiting or iteration form, though §14.4 asks for them. Missing fact: G6's decorated
  callable type, plus context-manager, awaitable or iterable result type.
- **CA2, parameter types.** `Signature`/`ParameterShape` carry no types. `ParameterType` evaluates
  only `TypeObservation(Parameter, declared)` at a *source* parameter declaration
  (`evaluate.rs:568-597`). Synthesized, stub-only, native or decorated-effective parameters have
  no type. Missing fact: a typed parameter list per signature variant, covering effective and
  synthesized forms as well as source.
- **CA3, option domains.** `ConfigurationLiteral` decides only `Literal` or union-of-`Literal`
  terms on declared record fields (`evaluate.rs:436-504`). An enum-typed field returns None,
  because it has no member enumeration. Type aliases and `Annotated` constraints are unresolved,
  and there is no parameter-literal-domain predicate. Missing facts: enum class member sets (name
  and value), alias expansion, and `Annotated` metadata (for example pydantic `Field`
  constraints) as typed terms.
- **CA4, overloads.** Variants are Pysa's undecorated signatures. Variants that fail
  `Signature::new` become an `OutsideProviderModel` coverage boundary (skill boundary map,
  verified as current in `crates/cpg-extract/src/pyrefly_stage.rs` per the skill). Overloaded
  type terms are display-only (cutover types/F01). Missing fact: typed overload alternatives for
  every variant.
- **CA5, declared facets with no evaluator.** `Predicate::FacetMembership` always returns
  `Ok(None)` (`evaluate.rs:1049`). The facets are `Returns`, `Raises`, `Decorator`, `Async`,
  `DelegatesTo`, `ForwardsTo`, `HandsOffTo`, `TakesFrom` and `ReadsSetting`
  (`vocabulary.rs:67-81`). `Raises` has no catalog-side producer at all. It would need G4 facts,
  and `Returns` would need declared or inferred return types.
- **CA6, field links.** Catalog field-access candidates match attribute spelling (G3).
  `FieldLocationLink.state` and `ConstructorCandidateLink.state` are fixed
  `HeapFieldStateUnavailable`. §14.4 says reads stay Unknown or proof-free without
  instance/temporal alias closure. No provider supplies points-to (skill
  `dataflow.points-to`: blank), so that part is out of reach. Attribute→declaration resolution
  (G3) would still sharpen candidate identity.
- **CA7, public identity.** `PublicPathBoundaryReason::OpenAncestry` (`D/catalog/paths.rs:99,120,156`)
  depends on MRO completeness. A computed `__all__` is `OutsideProviderModel` (extraction, per the
  skill). The `VariableOrigin` codebook entry (`D/obligation.rs:36`) has no emitter in `D/`.
- **CA8, synthesis of raises.** `structural::controls::ConditionalRaise`
  (`D/structural/controls.rs:130-145`) has no exception class. S0 renders the raise statement's
  *text* (`D/synthesis/assertions.rs:745-749`, `occurrence_text`). Missing fact: G4's raise-site
  class.
- **CA9, retrieval text.** API units render the parameter name, kind, required flag and default
  slot, with no types or return type (`D/retrieval/build.rs:147-205`). Interpretation: typed
  signature rendering depends on CA2 and AN2 facts. Retrieval itself does no re-derivation.

---

## D. Plans and findings that name missing facts

| Item (source) | Limited claim or verdict | Fact that would change it | Map ref |
|---|---|---|---|
| Closed-hierarchy `self` dispatch, 2,018 pilot rows (forward §7; `50969ba0`, 2026-09-27, generation `cdcf4b4e…`) | `OverrideDispatch` open; negatives and Summary composition blocked; projection gap | Class/method finality; attributed in-corpus subclass and override enumeration; complete MROs | G1, AN1 |
| Pyrefly-typed receivers for dynamic access (forward §7; skill `typed-dynamic-receivers`) | `DynamicAccess` Unknown; field-location candidates | Receiver/attribute-base type by range | G2 |
| Unmapped argument at a multi-callee site; lambda read phase; inherited methods counted as field reads (forward §7) | Binding Undetermined; read-phase; field-read negatives | Per-callee argument mapping; Pysa `LambdaArgument` reason; attribute→declaration resolution | G7, G3 |
| `**kwargs` binding TypeErrors (forward §7; Q09.a/e/f async) | `UnsupportedUnpacking`; no proven-incompatible verdict | Provider argument mapping incl. unpacking; call-check diagnostics | G7 |
| Cross-call exception composition (forward §7; Q01.e/f behind dispatch, Q05 async) | No source exception channel; escape claims | Raise-site class, handler class sets, per-call may-raise | G4 |
| Reachability under ty AMBIGUOUS, 730 rows (forward §7; `8a7c9367`, 2026-09-27; now coded `Approximation`) | Local non-return sink assessments refused | Region reachability certificate; static truthiness; end-of-scope reachability | G5 |
| ty as second type provider; bytecode CFG (forward §7) | Trace gaps (`native type trace unavailable`), display-only terms | Inferred type at range from a second provider, coded as a separate provider claim | G2, CA2, AN2 |
| Native-extension bodies (forward §7) | `AbstractBody`, `NoSourceDeclaration` | None beyond current stub origin; the boundary is true | G13 |
| General pydantic/dataclass initialization (forward §7) | Constructor storage; symbolic field links | Class-level record parameters; synthesized `__init__` signature; field specifiers | G9, G6 |
| F17 broader release/default/binding domains (forward §7) | Default availability | Not a provider fact (default evaluation and stability are model-owned) | — |
| W4 builtin access (forward §6, focused implementation passed) | No-read premises | Already ty and lexical; computed names stay dynamic | — |
| W14/F10 function flags and body kind (forward §6) | Negative premises | Consumed (`FunctionBodyObservation`) | — |
| PR3/F02 unittest dispatch overreach (forward §6.2, closed) | Intent instance form | Same as G1 for any future widening | G1 |
| Retained plan item 2 (forward §3.0.1, around line 597) | Implicit exit kind | ty `end_of_scope_reachability` | G5 |
| Retained S1–S6 rows (forward §3.0.1, lines 342-355) | Typed non-value subjects (exception, effect, callback, resource) | Exception facts (G4); effects are not provider facts (skill: `dataflow.effect-kinds` blank) | G4 |
| Alignment plan §7 (`semantic-model-incremental-alignment-plan_2026-10-02.md:242-271`) | All F01–F08 and O1–O6 closed/Tested 2026-10-03 | **None name a missing code fact**; all are process, gate or ownership remedies | — |

---

## E. Relations consumed but weak

1. **`TypeObservation` role coverage.** Only six roles exist. `TestOperand` is the only route to
   receiver types. `CallResult` is produced with no consumer. `Raised` reaches analytics only.
   Many consumers also require `declared=true` (selection `ParameterType`, analytics, the Type
   layer), so inferred types are effectively unused.
2. **`Signature` / `ParameterShape`.** These hold undecorated shape only, with no types, and the
   `ParamSpec`/`Ellipsis`/`NativeUnavailable` forms cannot be bound. Effective and decorated
   signatures are inferred only for builtin descriptors.
3. **`ClassTraitObservation`.** Four booleans. It has no final, abstract, protocol, metaclass,
   enum or record-parameter flags.
4. **`FunctionTraitObservation`.** It has no final flag, and its decorator effect is limited to
   static, class and property. `overrides` is the only dispatch edge, consumed for candidates but
   never for closure.
5. **`FlowRegionObservation` / reachability conditions.** AMBIGUOUS collapses to
   always-with-approximation, which Local rejects wholesale. Synthetic ty predicates without a
   site become `NativeUnavailable` atoms (skill map; `crates/cpg-extract/src/ty_flow.rs`).
6. **`FlowAttributeLoadObservation`.** It carries only the native `name`, with no resolved
   target, so every field consumer joins by spelling.
7. **`CallTarget.receiver_class`.** It is optional. Absent means `MissingReceiverClass` for
   dispatch and binding.
8. **`RecordFieldObservation`.** Per-field flags are good. Class-level parameters are absent,
   which forces syntax re-derivation in `symbolic_fields.rs`.
9. **`DeclarationDecorator`.** It is consumed as spelling (analytics) and as bare builtin names
   (descriptor recognition). Resolved targets are used only by aspects, and only as metadata.
10. **`ExactRuntimeException`.** A one-member exception universe (`TypeError`), resolved by name.

## Unresolved edges and limits

- **G12.** I did not trace whether a missing inherited `__enter__` or `__exit__` changes a
  verdict.
- **G4 plan drift.** I did not check whether typed-handler support survives outside
  `lctx-model::domain`. A grep under `D/` found none.
- **CA4/CA7 extraction-side causes.** I took these from the skill's boundary map without
  re-reading `crates/cpg-extract`; the sibling mapper owns extraction.
- **Excluded clusters.** I did not trace the `MissingEvidence` row-integrity checks, which are
  roughly 270 sites, most in `conditions/entry.rs`, `local_theory.rs` and `symbolic_fields.rs`.
  Some may hide provider-absence cases.
- **Pilot counts.** Both counts predate the cutover model and have not been re-measured. The
  730-row cause is now `Approximation`, not `MissingEvidence`.
