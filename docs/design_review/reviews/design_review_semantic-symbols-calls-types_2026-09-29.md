# Symbols, call events and type completion (A6–A8): bounded review

## 1. Scope, outcome and coverage

| Field | Assessment |
|---|---|
| Subject | Commits `b46ede2` (A6), `804a5c1` (A7) and `38ba3ea` (A8) on `main`. The review diffed `git diff d4e980d..38ba3ea -- crates/lctx-model crates/lctx-model-macros` together with the DESIGN, ADR-0089 and plan hunks. The working tree was dirty. Uncommitted A9 producer files (`cpg-extract/src/symbol_records.rs`, `natives.rs`) and the library-catalog scripts are outside this subject. |
| Standard | [standard.toml](../design_principles/standard.toml): core 3.1 and template, code-intelligence profile 1.2, and the repository binding. The design-review and design-review-code-intelligence skills were applied. |
| Tier · purpose | Change · conformance. This is a bounded slice inside the accepted §15 boundaries. Scenario depth goes beyond a normal change review because these contracts are the authority that A9–A11, P3 and P4 build on. |
| Reviewer · date | Independent design-reviewer subagent (Claude), 2026-09-29 |
| Authority | [DESIGN §15.4/§15.5](../../design/sections/semantic-model.md#section-15-4); [ADR-0089](../../adr/0089-stage-contributions-and-input-closure.md) with its A6 amendment; [cutover plan §4.1.1 A6–A11 and §4.2 receipts](../../plans/semantic-model-cutover-plan_2026-09-29.md#411-detailed-remaining-execution-order); ADR-0085/0086 |
| Supported scope | These families: symbols, provider modules, ancestry, traits, public names, parameter docs, dependency modules, call events and origins, provider call sites, receiver evidence, dispatch sets, the remaining type arms, callable parameter lists, function bodies and record fields. It covers their typed contracts, invariants and PG lowering. It does not cover producers (A9–A11), normalization or equivalence (P3), expansion or composition consumers (P4), or serving. |
| Expected changes | (1) the A9 symbol producer; (2) the A10 call producer through `normalize_site`/`classify_receiver`; (3) the A11 exhaustive type producer; (4) P3 equivalence and SQL views, and P4 use: override expansion, negative hierarchy decisions, record-field constructors; (5) a second provider (ty) adding call targets or types |
| Method and coverage | The reviewer read `symbols.rs`, `calls.rs`, `types.rs`, the assertion subject and support check, `declarations.rs`, the composition hunks, admission grain, the model and PG tests with their fixtures, the legacy `pysa_map.rs`/`public.rs`/`context.rs` and the `completion.rs` MRO consumer. The pinned Pysa report and Pyrefly types were read at `a07b7ba` (`report/pysa/{call_graph,function,class}.rs`, `pyrefly_types/{types,function,type_alias}.rs`, `alt/types/class_metadata.rs`). Serving, P3/P4 code and flow were not examined. |

The fact and fidelity reconstruction follows. All labels are Implemented or Interface-checked as of 2026-09-29.

| Fact family / relation | Provider | Fidelity | Coverage and unknowns | Identity | Consumers (planned) |
|---|---|---|---|---|---|
| `symbol_observations`, traits, `class_ancestry_observations` | Pysa definitions; MRO from Pyrefly `ClassMro` | native structural; `Linearization{Complete, Prefix, Cyclic}` | Signatures family, per Python artifact | provider-qualified `ProviderSymbol` (native key); content-addressed `SymbolSequence` | A9; P4 negative exception matching (`completion.rs` semantics map 1:1) |
| `parameter_annotation_observations` | Pysa `PysaType.string` | **DisplayOnly, enforced** by `Assertion::FIDELITY` | Signatures | signature parameter | presentation; structure comes from type observations |
| `public_name_observations` + `export_origins` | Pyrefly `trace_export_origin` | native; traced or untraced | Exports, per Python artifact | (qualification, access module, name) | catalog/exports (P3/P5) |
| `call_origins`, `call_target_observations`, `call_resolutions` | Pysa call graphs | analyzer assertion; modality per list | Calls, per Python artifact | event = (site, origin); alternatives are content-addressed | policy views, binder, composition |
| `provider_call_sites` | Pysa `ExpressionIdentifier` and `ExpressionCallees` kinds | native classification | Calls | (site, origin, kind, caller) | owner-rule comparison (P3) |
| `type_terms` and parameter lists; `record_field_observations`, `function_body_observations` | Pyrefly `Type` | structural, or display-only for opaque/truncated closures | Types, per Python artifact | content-addressed terms | P4 guards, record constructors |

## 2. Responsibilities and domain trace

`lctx-model::domain::{symbols, calls, types}` own the concepts and their invariants. The shared `SupportCheck` in `assertion.rs` owns attribution, scope and subject location. The `Assertion` derive expands subjects and the new `fidelity =` constant mechanically. `lctx-postgres` lowers the same types. The owner rule remains the only model caller. `ProviderCallSite.caller` is explicitly the provider's attribution.

Traced trace chains (phenomenon → concept/operation → implementation → consumer):

- **Implicit call at an occurrence** → call event `(site, CallOrigin)` → `CallOrigin::new`, `validate_provider_site`, `normalize_site`, `SiteTargets::new` → policy views and binder. See F01.
- **Virtual dispatch** → `CallDestination::Overrides` + `receiver_class` → `TargetCheck`, `site_facts`, `CallPolicy::admits`, `compose_*` obligation → invocation and dataflow projections, P4 expansion. See F02.
- **Where a public name is defined** → `ExportOrigin` subject → `SupportCheck::source`/scope rule → Exports coverage and P3 views. See F03.
- **Receiver passing** → `ReceiverEvidence` → `classify_receiver` → `Receiver` → `bind`. See F04.
- **Hierarchy** → `ClassAncestryObservation` + `Linearization` → `SymbolCheck` → P4 exception/`isinstance` decisions. This chain is sound: only `Complete` supports a negative decision, and cyclic MROs state the empty sequence.
- **Type structure** → `TypeTerm` arms, `TypeIndex` closure and shape checks → A11 and P4. This chain is sound apart from F06.

## 5. Change scenarios

| Scenario (kind) | Owner and route | Observed propagation / hidden knowledge | Evidence |
|---|---|---|---|
| 1. A9 symbols (new instances of existing concepts) | `pyrefly` stage emits the `symbols.rs` relations | Traits, nesting, ancestry, docs and dependency modules fit directly. Two obligations fall on the producer: MRO completeness must come from `get_class_mro(..).linearization_complete()`, because Pysa's `PysaClassMro` drops it (`class.rs:576–584`; legacy `context.rs:327`); and name spans map to def/class statement occurrences for `SymbolDeclaration`. **Public names that trace into another acquired module cannot be scoped per artifact (F03).** | Interface-checked |
| 2. A10 calls (composition through `normalize_site`) | `pyrefly` stage → `ProviderCallSite` + `normalize_site` | Regular, identifier, attribute and property records fit. Artificial nested origins need a stated flattening order: `Nested{head, tail}` displays as `tail>head`, while the test assumes `[head, tail]` (`domain_calls.rs`, chained-assign case). Higher-order and potential remainders must be passed as `Unresolved` `NativeCallee`s on their own channel and modality. The `unresolved` argument always lands on (Direct, Call) at the base modality (`calls.rs:929–931`). Pysa emits `LambdaArgument` only on higher-order parameters (`call_graph.rs:2955–2963`); the legacy code kept it there as Potential (`pysa_map.rs:659–674`). **Format-string records merge with explicit calls (F01); instance-method calls are Overrides-only (F02).** | Interface-checked |
| 3. A11 types (exhaustive mapping) | `pyrefly` stage → `TypeTerm` | Every modeled arm has closure and shape checks. Unmodeled Pyrefly variants (`CallableResidual`, DSL/shape types, `Sentinel`, `SuperInstance`, `Var`) go to `Other`/`Truncated` and are forced to display-only. Test operands have a role. **Inherited record fields declared in another acquired module hit F03.** Callable identity choice: see F06. | Interface-checked |
| 4. P3 equivalence and views; P4 behavior | P3 normalization; P4 engine | Sequences, origin steps and parameter lists are relationship rows, so they can be joined in SQL. Qualified names are derived only in a test helper (`domain_symbols.rs::qualified`) and need one owner at P3/P5. Negative hierarchy decisions are supported. **Override expansion has no stated meaning or owner (F02).** Record-field constructors work within one module and hit F03 across modules. | Proposed routes |
| 5. A second provider (mechanism/domain extension) | Its stage emits the same relations with its own `ProviderSymbol`s | Targets and types from another provider coexist. Until P3 equivalence, `SiteTargets` flags them as a disagreement, and that is declared. Pysa-typed fields sit in shared relations: `CallDestination::Unresolved.native`, `OriginStep` (Pysa's `OriginKind`) and `ReceiverPassing` (Pysa's `ImplicitReceiver`). A new provider would need sum-typed native reasons and a mapping of its implicit calls into Pysa's desugaring vocabulary. This is acceptable at L0, with that trigger recorded (F01, correction 4). | Proposed |

## 6. Correctness and fidelity gates

| Gate | Verdict | Own evidence |
|---|---|---|
| G1 Authority | **fail** | Implicitness has two carriers with no reconciliation: `CallOrigin` versus support `Origin::SyntheticModel` (F01). The stored `CallTarget.receiver` and its stored native evidence can disagree (F04). |
| G2 Semantic fidelity | **fail** | Format-string implicit calls are forced into the explicit event (F01). The `Overrides` meaning is misstated (F02). |
| G3 Validity | **fail** | `Overrides` without `receiver_class` is accepted (F02). Pseudo-callable destinations are accepted (F05). A record field's `declaration` kind is unchecked (F03). |
| G4 Hidden behavior | pass | Pure model code. Validators read only declared inputs. |
| G5 Consistency and recovery | pass for limits; n.a. otherwise | Every limit (`MAX_SYMBOL_NESTING`, `MAX_SEQUENCE_SYMBOLS`, `MAX_ORIGIN_STEPS`, 4096 slots or alternatives) refuses explicitly. No lifecycle changes. |
| G6 Transformation and reuse | **fail** | Policy views over a merged format-string event admit `__str__` as a Dataflow callee of the explicit call (F01). |
| G7 Truthful capability claims | **fail** | DESIGN §15.5 and `calls.rs:245–246` describe `Overrides` as "every override of a method". The A7 plan control describes a Pysa shape that does not occur (F02). |
| G8 Library leverage | pass | Mirrored provider vocabularies stay behind `lctx-model` (no pyrefly dependency, DP-17). The existing derive, sequence digests and charged collections are reused. No bespoke generic machinery was added. |
| CI-G1 Fidelity | **fail** | A relabelled relation (an implicit stringify call becomes the explicit call, F01). The dispatch-set meaning is unstated (F02). |
| CI-G2 / CI-G3 | n.a. | No served claims and no evaluation inputs. |

## 7. Findings

Current disposition belongs to [cutover plan §8](../../plans/semantic-model-cutover-plan_2026-09-29.md#8-findings-disposition) once scheduled. F05–F08 carry Deferred rows in §11 below until then (binding §4). This reviewer did not edit the plan.

<a id="F01"></a>
### F01 — Implicit-call status is not owned by the call event (High)

**Finding.** DESIGN §15.5 says events at one site are distinct calls and that implicit invocations are never silently merged with explicit ones. The model's event concept cannot keep that promise for Pysa's format-string calls:

- **Forced explicit origin.** `validate_provider_site` (`calls.rs:398–400`) requires a non-empty origin only for `ArtificialCall`/`ArtificialAttributeAccess`. `FormatStringArtificial` and `FormatStringStringify` sites must therefore carry `CallOrigin::explicit()`, and `OriginStep` has no step for them (`calls.rs:296–302`).
- **Same occurrence as the explicit call.** Pysa places a stringify at the interpolated expression's exact range (`call_graph.rs:3652–3668`). For `f"{g()}"`, the stringify event and the explicit call `g()` share site and origin. `CallTarget`/`CallResolution` do not key on the Pysa site kind.
- **One merged event.** `SiteTargets` therefore treats them as one event. `site_facts` sees `{g, T.__str__}` in the Call phase and reports a false disagreement: `g()` is no longer unique, so Summary refuses it. Dataflow (`calls.rs:957`) admits `__str__` as an alternative of the event whose output is the value of `g()`.
- **Names become call sites.** For `f"{x}"`, a Name occurrence becomes an explicit call site of `__str__`. `f"{obj.prop}"` mixes the Property and Call groups.
- **Second carrier.** Policies decide "analyzer vs synthetic" from support `Origin` (`calls.rs:949–963`), not from `CallOrigin`, and no invariant relates the two. The legacy code marked every artificial and format-string site `SyntheticModel` (`pysa_map.rs:550–554, 702–725`), which is the binding §6 defect shape. Under the new model, whether Invocation admits a `ForIter` event depends on the producer's choice of support origin.

**Principles · judgment/gate.** FP-04, DP-01/02/04/07, CI-02 · A2 violated; G1, G2, G6, CI-G1 fail.

**Correction (`lctx-model::domain::calls`, DESIGN §15.5).**
1. Append origin steps for format-string artificial and stringify calls (codes 22 and 23), and require a non-empty origin for every artificial site kind.
2. Make `CallOrigin` the one implicitness authority that policies read. Decide once whether Invocation admits implicit events, with disclosure. Either stop using `SyntheticModel` for Pysa artificial calls, or add an invariant equating the two.
3. State the `Nested` flattening order in the `CallOrigin` doc.
4. Record the trigger for a model-owned implicit-operation vocabulary: a second provider contributing implicit events. Until then the event discriminator is Pysa's desugaring path.

**Closure.** A known answer where `f"{g()}"` gives two events and `g()` is unique. A stringify at a Name is not the explicit event. A `ForIter` event gets the same Invocation answer regardless of its support origin, or a mismatch is refused. A chained-assign origin matches Pysa's nesting.

<a id="F02"></a>
### F02 — Override dispatch sets: meaning misstated, receiver class unenforced, interim exclusion unowned (High)

**Finding.**

**(a) What Pysa emits.** Pyrefly's Pysa export emits a single `Target::Overrides(callee)` for every call through an attribute on an instance, `self` or `cls` (`is_direct_call`, `call_graph.rs:1430–1478`). It does so whenever a receiver class resolves and the callee has a defining class (`compute_targets_for_virtual_call`, `:1712–1738`). It never pairs that target with a `Function(callee)` target. The meaning is "callee, or any override in a class extending `receiver_class`" ("Pysa is responsible for filtering").

**(b) What the model says.** DESIGN §15.5 (lines 264–265) and `calls.rs:245–246` say "every override of a method". That omits both the base implementation and the receiver-class restriction. `TargetCheck` (`calls.rs:1005–1013`) accepts `Overrides` with `receiver_class: None`, which is a shape Pysa never produces.

**(c) What the policies do with the real shape.** Under Pysa's actual output, `site_facts` gives `dispatch`, not `unique`, and `complete = false` because there are no targets. Invocation, Dataflow and Summary all admit 0 (`direct`, `calls.rs:951`). The dominant form of method call therefore disappears from the invocation projection and from flow composition. The legacy code kept the base method as a Candidate target (`pysa_map.rs:540–549`). The exclusion lasts "until a later layer expands the set", and no plan row owns that expansion.

**(d) The control hides it.** `an_override_dispatch_set_is_never_a_direct_target` (`domain_sites.rs:148–169`) builds a `Resolved{Base.m}` + `Overrides{Base.m}` pair and asserts Dataflow 1 and Invocation 1. The plan's A7 control text repeats this. The control masks (c).

**Principles · judgment/gate.** FP-02/04, DP-02/03/08/22, CI-02/04 · A2 violated, A3 unresolved; G2, G3, G7, CI-G1 fail. A CI-04 risk follows at P3: "no caller" over dispatched events.

**Correction.**
1. `CallDestination::Overrides` doc and DESIGN §15.5: the set is the callee ∪ its transitive overriders (reverse `FunctionTraitObservation.overrides`) whose defining class is a subclass of `receiver_class`. Subclassing is decided by a `Complete` MRO; a `Prefix`/`Cyclic` MRO leaves the set open.
2. Refuse `Overrides` without `receiver_class`.
3. Name the expansion owner as a P3 or P4 plan row with its trigger. Decide the interim projection explicitly: either admit the callee as a Candidate with dispatch disclosed (legacy parity), or have projections mark dispatched events as incomplete rather than absent.
4. Replace the paired control with Pysa's single-target shape.

**Closure.** The amended contract text, a refusal control, a realistic-shape policy control, and the plan row.

<a id="F03"></a>
### F03 — Referents declared as located subjects push cross-module facts off the coverage grain (Medium)

**Finding.**

**How scope is widened.** `PublicNameObservation` declares `subjects(access, origin)` (`symbols.rs:172`). `RecordFieldObservation` declares `subjects(class, term, declaration)` (`types.rs:544`). `SupportCheck` locates `ExportOrigin → ProviderModule → Acquired` and an occurrence at their sources, and requires every located subject to lie inside the assertion's scope (`assertion.rs:309–317, 377–407`).

**Consequence for public names and record fields.**
- A re-export is the ordinary output of `trace_export_origin`: for example, a package `__init__` re-exporting `pkg.server.server.FastMCP`. Such a public name can only be asserted at Input or Release scope.
- The same applies to an inherited record field declared in another acquired module.
- Exports and Types coverage is stated only per Python artifact (`admission.rs:50, 166–187`), and an Input-scope Exports row is refused (`:301`).
- One module's export list is therefore split across scopes. "One origin per name" is detected only within a qualification. P3 views must know to join coverage by `access` rather than by scope.
- The fixture places everything under one Input scope and one module, so this path is untested.

**Inconsistency.** `ClassAncestryObservation` (`subjects(class)`, ancestors checked by `SymbolCheck.related`) and `CallTarget` (`subjects(site)`, destination checked by `NativeSupportCheck`) treat referents as non-subjects.

**Further gap.** `RecordFieldObservation.declaration` accepts any occurrence kind or placement.

**Principles · judgment/gate.** FP-01/04/05, DP-03/05, CI-04 · A1 unresolved, A2 violated for exports and inherited fields; G3 fail for the unchecked declaration.

**Correction (`assertion.rs` derive/check, `symbols.rs`, `types.rs`).**
1. Subject only `access`, or `class`.
2. Check referent ownership without scope location. Either add a `referents(...)` derive attribute that runs the Symbol/ProviderModule ownership arms, or add dedicated invariants in the style of `NativeSupportCheck`. Bundled, namespace and unresolved origins must belong to the supporting provider and context; the declaration must be acquired by the run's input, be a field-declaring occurrence, and lie in the class or an ancestor.

**Closure.** A two-module fixture in which a re-export and an inherited field validate at the artifact scope of the access module or class. A foreign-bundle origin still refuses.

<a id="F04"></a>
### F04 — Receiver classification discards the class-versus-object distinction A7 added (Medium)

**Finding.**
- A7 introduced `ReceiverPassing{NotPassed, Class, Object}` to carry Pysa's `ImplicitReceiver`. For a classmethod called on an instance (`c.f()`), Pysa emits `TrueWithObjectReceiver` (`call_graph.rs:1287–1293`).
- `classify_receiver` (`calls.rs:697–710`) maps `Object` + `class_method = Some(true)` to `Bound{actual: c}`, so `bind` binds `cls ← c` instead of `type(c)`.
- The defect is latent for most instance calls because they are `Overrides` (F02). It surfaces on `super()` calls and at expansion.
- The stored `CallTarget.receiver` is not re-derived from the stored `passing`/`class_method`/`static_method` by any invariant, so the two can disagree.
- `attribute_access` is dead: both of its branches return Unknown.
- The target-level `static_method` flag is Pysa's call-time flag, which is true for `__new__` (`:1837`). It differs from `FunctionTraitObservation.staticmethod`, and no source is stated as governing.

**Principles · judgment/gate.** FP-04, DP-01/02/08 · A2 violated; G1 fail. This refines the closed C10 policy; it is not a duplicate of it.

**Correction (`calls.rs`).**
- Bind the actual only when its value is the receiver: Object passing with a method, or Class passing with a classmethod.
- For Object passing with a classmethod, return either `Receiver::ClassOf{actual}` or a conservative Unknown. Decide which.
- Add a `TargetCheck` arm that re-derives the receiver from the stored evidence.
- Remove `attribute_access` or give it meaning.
- State which classmethod/staticmethod source governs.

**Closure.** Classifier controls for `c.cm()` and `C.cm()`, and a refusal when the stored receiver and its evidence mismatch.

<a id="F05"></a>
### F05 — Pseudo-callables share `SymbolKind` without an owned predicate (Low)

**Finding.** `ModuleBody`, `ClassBody` and `DecoratorApplication` are appended to the Python symbol-kind enum (`calls.rs:12`). Four sites hand-enumerate kinds: `CallerCheck`, `DeclarationCheck` (`declarations.rs:80–86`), the policy's `callable`, and `SymbolCheck` parents. Nothing refuses a pseudo-callable as a `Resolved`/`Overrides` destination, a signature symbol or an observed symbol. A def or class occurrence carries two declared symbols, which P3 entity building must filter out. The correspondence to the owner rule is stated nowhere: a decorator application attributed to `f`'s pseudo-callable is owned by the enclosing scope. As provider attribution, the design is sound and does not compete with the owner rule.

**Principles · judgment/gate.** DP-02/06, FP-04 · A2 (minor).

**Correction.** Use a typed caller sum `ProviderCallable{Symbol, ModuleBody(module), ClassBody(class), DecoratorApplication(function)}`. Alternatively, add an owned `SymbolKind` predicate, refusals, and one mapping to the owner-rule entity.

**Trigger.** A10 emits callers, or P3 builds entities.

<a id="F06"></a>
### F06 — Type identity embeds unqualified names (Low)

**Finding.**
- `Callable{function: Option<String>}` and `Overload{function: String}` (`types.rs:66–67`) put a def's bare name into structural identity. Pyrefly's `Function` identity carries `FuncDefId{qname, cls, def_index}` (`pyrefly_types/function.rs:42–52, 274–278`).
- As a result, same-named, same-signature defs in two modules collapse into one term, while structurally equal callables with different names split.
- `SpecialForm{form}` holds both typing special forms and TypeVar/ParamSpec/TypeVarTuple declaration values by their written name, so `T` in two modules collapses.

**Sound choices.** Dropping the union alias display name is correct: Pyrefly marks it `IdentityIgnored` (`pyrefly_types/types.rs:781–784`). `VariableForm` is validated per kind.

**Principles · judgment/gate.** DP-04, DP-02.

**Correction.** For each arm, choose nominal identity (reference a `ProviderSymbol`/`TypeVariable`) or structural identity (move the name to `TypePresentation`). Give declaration values their own arm.

**Trigger.** A11 mapping, or the first consumer that reads callable identity from types.

<a id="F07"></a>
### F07 — Retiring annotation class/scalar sets needs a stated closure condition (Low)

**Finding.** For `def` parameters, structure is richer through `ParameterDeclaration` → `TypeObservation{role: Parameter, declared}`. For synthesized record constructors, it comes through `RecordFieldObservation`. Legacy consumers used only `required` (`cpg-core/src/synth.rs:438–455`), so no current consumer is lost. Two gaps remain:
- Function-like class fields (`def_statement: false`, such as `handler: Callable[..]`) keep only display. Pysa gave class sets for them, and they have no type role.
- No owner is named for the derived class-set, exhaustiveness and scalar predicates that P4 guard refutation would use (over `TypeTerm` + a `Complete` MRO).

**Principles · judgment/gate.** CI-04, DP-22.

**Correction.** Add an A11 control: every annotated def parameter with a `ParameterDeclaration` has a declared Parameter type observation. Disclose the annotations of non-def callables as display-only. Name the P4 derivation owner.

**Trigger.** A11, or P4 guard refutation.

<a id="F08"></a>
### F08 — Namespace location is relative to an unidentified root (Low)

**Finding.** `DependencyModuleObservation.location` for a `Namespace` is "its directory within its search root" (`symbols.rs:198–208`). The root is not identified, and a context's `search_path`/`site_package_path` can hold several roots with the same relative directory (PEP 420 portions). The identity itself is sound. `Bundled{provider, bundle, name}` is covered by the provider build digest, and `Namespace{provider, context, name}` by the context.

**Principles · judgment/gate.** DP-02.

**Correction.** Carry the root, either as an ordinal into the context path or as a typed root kind, or state that the location is Pyrefly's single reported portion.

**Trigger.** A9 emits namespaces, or a consumer reads the location.

**Observations (not findings).**
- No stored invariant is quadratic or unbounded:
  - every map is charged;
  - the ancestry check walks each observation's shared sequence, O(Σ|MRO|) with each MRO ≤ 4096;
  - `SymbolCheck` retains full `Occurrence` rows (charged) and copies observed keys uncharged in `finish` (O(n));
  - `normalize_site`'s `Vec::contains` scans are O(k²) per site, bounded at 4096 alternatives and not stored.
- Two things are sound: the tri-state `Linearization`, and the `Assertion::FIDELITY` refusal for display-only annotations.

**Foundation verdicts (bounded).**
- FP-01 is satisfied. Module ownership is coherent.
- FP-02 is violated for `Overrides` (F02).
- FP-03 is unresolved because expansion cannot be composed yet (F02).
- FP-04 is violated (F01–F04).
- FP-05 is violated for unrefused shapes (F02, F03, F05).
- FP-06 is satisfied. Controls run store-free and the PG suites are separate.

## 8. Library fit and total complexity

The pinned Pysa and Pyrefly types are mirrored as model codes and are not linked from `lctx-model`. This keeps the domain core free of the provider (DP-17). The cost is that Pysa's encoding choices become model vocabulary, which is the root of F01. The provider's `ExpressionIdentifier` split was copied where the model concept, implicitness, should have governed. No library offers these contracts. The derive, `serde_arrow` codecs and charged collections are reused appropriately. The `docs/library-utilization.jsonl` catalog was not consulted, because no generic capability is in question.

## 9. Alternatives (brief)

| Question | Current | Alternative | Assessment |
|---|---|---|---|
| Format-string event identity | Explicit origin | Add the Pysa site kind to the event key, or add origin steps | Origin steps keep the event semantic and shared across providers. A site-kind key would leak native encoding into identity. Prefer origin steps (F01). |
| Caller of pseudo-callables | `SymbolKind` extension + `SymbolDeclaration` | Typed `ProviderCallable` sum | The sum is smaller and structurally linked (F05). |
| Cross-module referents | Located subjects | A referent ownership check | Keeps scope aligned with coverage grain and matches ancestry and call-target practice (F03). |

## 10. Verification and uncertainty

| Check | Outcome (2026-09-29) |
|---|---|
| `eval "$(python3 scripts/build_environment.py --shell)" && cargo test --release -p lctx-model --test domain_symbols --test domain_calls --test domain_sites --test domain_types` | **passed**, reviewer-run: 10, 12, 6 and 10 tests. The passes confirm the stated controls. They do not exercise F01–F04, and the F02 control uses a non-Pysa shape. |
| `cargo test --release -p lctx-postgres --test domain_symbols --test domain_calls --test domain_types` | **not_run** by the reviewer (disposable PG18 not started). Author receipts of 2026-09-29 in plan §4.2 are historical. |
| `just fmt`, `just test-all`, facts pilots | **not_run**: the plan's functional scope is incomplete (AGENTS.md timing). |

F01, F02 and F04 rest on source reasoning over the pinned Pysa code (Interface-checked). The closure controls named above would settle them. No probe was run, because the code paths are direct.

## 11. Dispositions

| Required change | Owner | Findings | Disposition |
|---|---|---|---|
| Implicit events: format-string origin steps; `CallOrigin` as the policy authority; flattening order | `lctx-model::domain::calls`, DESIGN §15.5 | F01 | open; schedule in plan §8 before A10 |
| Dispatch-set meaning, `receiver_class` refusal, expansion owner and interim policy, realistic control | `calls.rs`, DESIGN §15.5, plan P3/P4 row | F02 | open; schedule in plan §8 before A10 |
| Referent ownership instead of located subjects; record-field declaration check | `assertion.rs` derive/check, `symbols.rs`, `types.rs` | F03 | open; schedule before A9 public names / A11 record fields |
| Receiver classifier and stored re-derivation | `calls.rs` | F04 | open; schedule before A10 |
| Pseudo-callable representation | `calls.rs`, `declarations.rs` | F05 | Deferred; trigger: A10 callers or P3 entities |
| Callable/SpecialForm identity level | `types.rs` | F06 | Deferred; trigger: A11 mapping or first identity consumer |
| Annotation retirement closure and derivation owner | plan A11, P4 | F07 | Deferred; trigger: A11 or P4 guard refutation |
| Namespace location root | `symbols.rs` | F08 | Deferred; trigger: A9 namespace emission |

## 12. Architectural judgment and decision

| Judgment | Verdict | Evidence |
|---|---|---|
| A1 Localize change | unresolved | Concepts are owned per module and producers are separable. However, cross-module exports and inherited fields force hidden scope knowledge into A9 and P3 (F03). |
| A2 Encode domain meaning explicitly | **violated** | The event concept cannot represent format-string implicit calls, and implicitness has two carriers (F01). The dispatch set's meaning is misstated (F02). The receiver classification drops a stored distinction (F04). |
| A3 Extend through composition | unresolved | P4 override expansion cannot be composed from the current contract (F02). A second provider's implicit events must reproduce Pysa's desugaring vocabulary (F01, correction 4). |

**Bounded change decision: Revise.** Correct F01 and F02 before A10, F03 before A9 public names and A11 record fields, and F04 before A10. F05–F08 are deferred with triggers. The symbol, ancestry, trait, docstring, dependency-module and type-arm contracts are otherwise sound for their scenarios.

**Enclosing architecture: not assessed.** A9–A11 producers, P3 equivalence and views, and P4 expansion do not exist yet. This slice certifies none of them. The next step is a C-slice amending `calls.rs` and §15.5 (F01, F02, F04) and the assertion referent contract (F03). Its owner is the cutover plan's A-series.
