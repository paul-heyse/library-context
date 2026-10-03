# Supply map: Python code facts library-context derives today

Baseline: `main` at `6885dbf6`, clean tree, read 2026-10-03. Read-only: source reading, `rg`; nothing built or run.
Linked versions today: pyrefly fork `a07b7bae` (1.3.1 + patch) with ruff crates `=0.0.11`
(`Cargo.toml:55-64`), and ty `=0.0.14` (`ty_python_core`, `ty_module_resolver`, `ty_vendored`, `ruff_db`) with ruff
`=0.0.14` aliases (`Cargo.toml:69-75`). The review assumes the shift to pyrefly 1.4.0-dev.3 on ruff 0.0.14. Where a claim
here rests on the shared skill's index (which covers pyrefly 1.4.0-dev.3 **and ty 0.0.16**, not ty 0.0.14), it is marked
"skill index".

Paths are relative to the repository root. `E` = `crates/cpg-extract/src`, `F` = `crates/cpg-flow/src`,
`M` = `crates/lctx-model/src/domain`, `N` = `M/normalized`.

---

## 0. Pipeline shape (what produces what)

| Stage | Code | Provider handles | Families covered | Profile |
|---|---|---|---|---|
| `pyrefly` | `E/pyrefly_stage.rs` (`session` at :464) | one `State` + one `Transaction`, `Require::Everything`, Pysa reporter with `write_files: false` (:550-563) | Syntax, Lexical, Exports, Signatures, Calls, Types (`FAMILIES`, :55) | Catalog, Behavioral |
| `ty_flow` | `E/ty_flow.rs` + `F/lib.rs`, `F/predicate.rs` | `semantic_index(db, pf)` per file over a virtual `/flow` root (`F/lib.rs:324-418`) | Flow | Behavioral only (`E/ty_flow.rs` declaration, `profiles: vec![Profile::Behavioral]`) |
| documents | `E/document_parser.rs` | markdown-rs + own mention recognizer | Docs (markdown, **not** Python docstrings) | — |
| acquisition/assembly | `E/acquisition.rs`, `E/assembly.rs` | — | Artifacts | — |
| deployment | `E/deployment.rs` | own parsers of distribution bytes | Deployment | — |

Order inside one pyrefly session, per analyzed root module (`E/pyrefly_stage.rs:620-990`): admit/UTF-8 check → typed
syntax walk → syntax records → lexical recognizer → Pysa definitions (symbols) → Pysa call graphs (calls) → types →
parameter docs → per-family coverage. After all modules: public names, then dependency context (import resolution,
supporting definitions in non-root modules, model-requirement checks), `:995-1288`.

The flow stage reads the pyrefly stage's stored Lexical/Syntax rows as inputs (`DeclarationObservation`,
`BindingEvent`, `BindingObservation`, `LexicalTarget`, `LexicalResolution`, `ReferenceObservation`,
`ImportAliasObservation`, `SyntaxObservation`) and attaches ty facts to them by byte range and lexical scope.

---

## 1. Fact families: producer, provider source, relations, coverage

### 1.1 Syntax

- **Producer:** `E/typed_syntax.rs` `emit` (:131) — one `SourceOrderVisitor` over `Transaction::get_ast(handle)`, text
  from `get_module_info(handle).lined_buffer()` (`E/pyrefly_stage.rs:654-660`). Then `E/syntax_records.rs` `records`
  (:214), a second source-order walk over the same AST.
- **Provider source:** ruff 0.0.11 AST as retained by pyrefly (parse of record). Bespoke: node placement (parent,
  `SyntaxField`, ordinal — `typed_syntax.rs` `fields` :446), operator/literal details (`details` :718, `literal_of` :689),
  `NodeKind`→`SyntaxKind` codebook (:765).
- **Relations:** `Occurrence`, `SyntaxObservation` (identifier leaves), `SyntaxPlacement`, `SyntaxDetailObservation`
  (operators, literal values); `DeclarationObservation` (kind, name occurrence, parent, `overload` flag, docstring
  statement occurrence), `DeclarationDecorator` (decorator occurrence + ordinal), `ClassFieldSyntaxObservation`
  (class-body `x: T = v` / `x = v` with a single Name target only, `syntax_records.rs:569-596`), `CallSyntax` +
  `CallArgument` (callee occurrence, `in_annotation`, ordered actuals with kind/keyword, :614-650). Support:
  `Origin::SourceObservation`, `ExtractionMode::NativeTraversal`, `Fidelity::NativeStructural`.
  Note the family split in `write_records` (`E/pyrefly_stage.rs:1733-1818`): `ImportAliasObservation` and
  `DunderAllObservation` are supported under **Exports**, `ParameterSyntaxObservation` under **Signatures**, the rest
  under Syntax.
- **Coverage/boundaries:** Artifact-scoped `ProviderCoverage`. `ResourceRefused` (size/node/depth/callback limits,
  `typed_syntax.rs` `admit` :121, `SyntaxError::coverage` :102), `UndecodableSource` (own UTF-8 check before pyrefly,
  `pyrefly_stage.rs:525-529`). `SyntaxError` → `Partial` when any pyrefly error bucket holds `ErrorKind::ParseError`
  (`:720-731`, reads `get_errors(...).collect_errors()` but only for that kind). Lexical coverage is a copy of Syntax
  coverage (`:903-909`).

### 1.2 Lexical

- **Producer:** `E/lexical.rs` (`Lexical::enter` :374, `finish` :902), driven by `E/lexical_records.rs` `facts`/`records`.
- **Provider source:** **bespoke recognizer** over the ruff AST (scopes, binding events, reads, resolution under
  Python's rules). Provider inputs only for the "outside" name sets:
  - implicit globals: `pyrefly_types::globals::ImplicitGlobal::implicit_globals(false)` (`pyrefly_stage.rs:1873-1920`);
  - builtins: `Transaction::get_exports` of the resolved `builtins` module, filtered by the patched
    `coverage::collect::is_public_name` and export `symbol_kind` (same function);
  - star imports: `Transaction::get_wildcard(&Handle)` (patch change 8) per `from m import *`, `star_imports` (:1922);
  - static branches: `SysInfo::evaluate_bool` per `if`/`elif` clause and `Ast::if_branches`, reproducing
    `SysInfo::pruned_if_branches` (`lexical.rs` `clause_marks` :213); the branch kind is classified bespoke from the
    test expression (`static_kind` :248, uses `SysInfo::is_type_checking_constant_name` and ruff `any_over_expr`).
- **Relations:** `LexicalScope`, `LexicalScopeObservation`, `BindingEvent`, `BindingObservation` (kind, ordinal, value
  occurrence, `static_branch`, `static_polarity` — `M/lexical.rs:128-145`), `ReferenceObservation`, `LexicalTarget`
  (binding / builtin with `variable` flag / unresolved), `LexicalResolution`. A read with several candidate bindings,
  or one an unknown star import may bind, is under a `Modality::Candidate` qualification (`pyrefly_stage.rs:777-783`).
  Support: `Origin::DerivedAnalysis`, `ExtractionMode::Recognizer`, `Fidelity::NormalizedStructural`
  (`write_lexical` :1820). Surface name is still `"retained Ruff parse"` for every family (:497-509).
- **Coverage/boundaries:** no Lexical-specific boundary rows; unresolved names are `LexicalTarget::Unresolved
  (UnresolvedTarget)` rows (`lexical.rs` :44-54). Names inside annotations are deliberately not lexical (module doc
  :20-21); they belong to Types.

### 1.3 Exports

- **Producer:** `E/public_records.rs` `public_names` (:67); `__all__` syntax in `syntax_records.rs` `dunder_all`
  (:407) and `statement` (:444); import aliases in `statement`.
- **Provider source:** pyrefly through the patch: `get_exports_data(handle).explicit_dunder_all_names()`,
  `is_explicit_reexport`, `get_exports`, `coverage::collect::{is_public_module, is_public_name,
  EXCLUDED_MODULE_DUNDERS, trace_export_origin, compute_public_fqns}`. The flattened set must equal
  `compute_public_fqns` or the run fails (:126-141). `__all__` forms are bespoke syntax recognition: literal list/tuple
  of strings, `+=`, annotated assign, `.extend([...])`, `.append("x")`; anything else is "computed".
- **Relations:** `PublicNameObservation` (access module, name, `via_dunder_all`, origin), `ExportOrigin`
  (`Traced{module,name,kind}` | `Untraced`), `DunderAllObservation` (`literal`, names `LiteralSet`),
  `ImportAliasObservation` (statement, alias, level, `resolved_module` computed bespoke by `absolute_module`
  :198 via `ModuleName::new_maybe_relative`). Public-name support: `AnalyzerAssertion`/`NativeTraversal`/
  `NativeStructural`, evidence = Pysa invocation.
- **Coverage/boundaries:** Exports `Partial` + `OutsideProviderModel` when the syntax walk finds a computed `__all__`
  (`pyrefly_stage.rs:861-873`, `:914-921`); one `SubjectBoundary` per computed statement.
  **Discrepancy:** `acquisition-and-extraction.md:347-349` says the detector also fires on
  `unresolvable_dunder_all_range()`; `rg unresolvable_dunder_all|dunder_all_range` over `crates/` finds no call. Only the
  syntax test is implemented.

### 1.4 Signatures (symbols, traits, MRO, signatures, parameter docs, dependency context)

- **Producer:** `E/pyrefly_stage.rs` `definitions` (:1398) → `E/symbol_records.rs` `records` (:291);
  `symbol_records::parameter_docs` (:673); `write_symbols` (:1579), `write_docs` (:1538).
- **Provider source:** Pysa collectors in memory: `PysaResolver::new`, `ModuleAnswersContext::create`,
  `collect_captured_variables_for_module`, `create_reversed_override_graph_for_module`, `export_module_definitions`
  (:1411-1428). Supplemented directly from pyrefly because the report drops information:
  `class::get_all_classes` + `get_class_mro(...).linearization_complete()` (MRO completeness, :1438-1446);
  `class::get_class_field_declaration` to tell a callable class field from a synthesized method
  (`FunctionOrigin`, :1449-1484). Locations converted with `ruff_source_file::LineIndex` (`Locator`, `symbol_records.rs:36-60`).
  Docstrings: `pyrefly_python::docstring::Docstring::range_from_stmts` and `parse_parameter_documentation`; the
  description's byte span inside the literal is located bespoke (`E/docstrings.rs` `locate_description`).
- **Relations:** `ProviderModule`, `ProviderSymbol`, `SymbolObservation`, `SymbolSequence[Member]` (MRO lists),
  `FunctionTraitObservation` (overload, staticmethod, classmethod, property getter/setter, stub, defining class,
  origin — `symbol_records.rs:540-560`), `ClassTraitObservation` (synthesized, dataclass, named_tuple, typed_dict only —
  :460-469), `ClassAncestryObservation` (+ `Linearization`), `Signature`/`SignatureParameter`/`ParameterShape` per
  **undecorated** signature variant (`function.undecorated_signatures`, :575), `SignatureEnumerationObservation`,
  `ParameterAnnotationObservation` (Pysa's display string), `SymbolDeclaration`/`ParameterDeclaration` (link to the exact
  name/formal occurrence), `ParameterDocObservation` (`Recognizer`/`NormalizedStructural`, evidence = source span),
  `DependencyModuleObservation` (non-root modules only, `:1250-1272`).
  Pysa-derived rows are `AnalyzerAssertion`/`NativeTraversal` with `ReportProjection` fidelity per the design table.
- **Coverage/boundaries** (`:922-950`, `:1262-1287`):
  - `OutsideProviderModel` when a native signature variant cannot be built (`SignatureForm::NativeUnavailable`, :587)
    or when a model-context requirement fails (pin, location, missing definition, incomplete MRO — `:1208-1240`);
  - `AttachmentUnmatched` for a symbol that does not attach at an exact name span, and for traits without a
    defining class (:544);
  - `ProviderDisagreement` for a parameter description not located in the docstring bytes (`write_docs`).
  - Input-scoped Signatures coverage for the dependency context (`:1284-1287`).
- **Not carried** (design table §4.2.3 and code): Pysa `captured_variables`, `global_targets`, `return_type`,
  `Define`, `FormatString` targets. `rg decorator_callees|global_variables` over `E/` and `F/`: no hit, so Pysa's
  decorator callees and global-variable definitions are not read either.

### 1.5 Calls

- **Producer:** `E/pyrefly_stage.rs` `calls` (:2000) → `E/call_records.rs` `records` (:389); `write_calls` (:2089).
- **Provider source:** `export_module_call_graphs` (Pysa) + `Transaction::resolve_pysa_solutions(&found)
  .function_base_definitions` to name targets (:2049-2067). Mapping is exhaustive over Pysa's enums
  (`#![deny(clippy::wildcard_enum_match_arm)]`), with `UnresolvedReason` mirrored into `PysaUnresolvedReason` (:121).
  Attachment of each Pysa site to an occurrence by exact range + expected kind (`Spans::event`, :473); call syntax
  comes from §1.1.
- **Relations:** `ProviderCallable`, `ProviderCallSite`, `CallOrigin`/`CallOriginStep` (artificial/implicit-call
  origin chains, :77), `CallDestination` (Resolved / Overrides / SyntheticFormatting / Unresolved), `CallChannel`,
  `Receiver` (implicit receiver, receiver class, dunder flags), `CallTarget` (phase: call/init/new/property get/set,
  higher-order argument, if-called), `CallResolution`/`CallResolutionMember` (set with remainder).
- **Coverage/boundaries:** `UnresolvedTarget` as an unresolved destination; `AttachmentAmbiguous` /
  `AttachmentUnmatched` for unattachable Pysa events (:474-491); per Ruff call without a Pysa record
  `OutsideProviderModel` (inside an annotation) or `MissingEvidence` (:608-622). **Any** boundary or unattached
  event makes module Calls coverage `Partial` with reason `MissingEvidence` (`pyrefly_stage.rs:814`, `:951-958`),
  so the coverage reason does not distinguish annotation calls from real gaps; the subject rows do.

### 1.6 Types

- **Producer:** `E/pyrefly_stage.rs` `types` (:2208) → `E/type_records.rs` `records` (:1001); `write_types` (:2321).
- **Provider source:**
  - decorated/undecorated defs: `report::pysa::function::get_all_decorated_functions(ctx)` (:1042) — only
    `f.undecorated` is read (identifier, `metadata.flags`, `params`);
  - declared return: patched `Answers::get_annotation(&Bindings, &KeyAnnotation::ReturnAnnotation)` (:1108, patch
    change 1, obsolete at 1.4.0-dev.3 per the skill's migration page); inferred return:
    `Bindings::key_to_idx(&Key::ReturnType)` + `Answers::get_type_at(idx)` (:1117-1119);
  - expression types: `Answers::get_type_trace(range)` (`trace`, :1331), only for selected roles;
  - records: `Solutions::get(&KeyClassMetadata)` → `typed_dict_metadata` / `named_tuple_metadata` /
    `dataclass_metadata` / `is_pydantic_model` (:1172-1210); fields through `get_class_mro`,
    `get_class_field_declaration`, `get_class_field_from_current_class_only`, `bindings.metadata().get_class().fields
    .field_decl_range`, patched `dataclass_flags_of` (:1270), `as_named_tuple_requiredness`,
    `as_typed_dict_field_info`.
  - Type lowering: bespoke `Builder::build` (:516-962) over `pyrefly_types::types::Type`.
- **Relations:** `TypeTerm` (content-addressed), `TypeSequence[Member]`, `CallableParameterList`/`CallableParameter`,
  `TypedDictFieldList`/`TypedDictField`, `TypeVariable`, `TypeVariableRestriction`, `TypeObservation` with
  `TypeRole ∈ {Parameter, Return, CallResult, Argument, Raised, TestOperand}` (`M/types.rs:679-686`) and a `declared`
  flag, `TypePresentation`, `FunctionBodyObservation` (body kind, abstract, in-protocol, in-TYPE_CHECKING, overload),
  `RecordFieldObservation` (record kind dataclass/attrs/pydantic/NamedTuple/TypedDict, ordinal, term, declared,
  declaration occurrence, default, init, alias, kw_only, required, read_only).
- **Which occurrences get a type** (:1130-1166): call results and call arguments (non-annotation calls), the `exc` of
  `raise`, and Name/Attribute/Subscript under a `Test` field. Not: assignment targets/values, attribute receivers,
  `return` values, comprehension targets, module/class variables, `with` items, `for` targets (absence from the code
  path read above; no other `trace` call sites in `E/`).
- **Coverage/boundaries:** `MissingEvidence` (trace absent; native parameter without syntax parameter; return type
  unavailable), `AttachmentUnmatched` (typed def without exact declaration name, :1056), `OutsideProviderModel`
  (opaque/display-only closure, :980-986; callable/restriction shapes, :339, :439), `BudgetReached` (:488),
  `NativeUnavailable` (record field without retained native field, record solutions unavailable). Module Types
  coverage collapses to `MissingEvidence` whenever any boundary exists (`pyrefly_stage.rs:840`, `:967-974`).
  Opaque `Type` variants today: `CallableResidual`, `TypeLevelDslCall`, `ShapedArray`, `IntTuple`, `NNModule`,
  `DataFrame`, `Series`, `Int`, `Var`, `Sentinel`, `SuperInstance`, `KwCall`, `Materialization` (:949-961). At
  1.4.0-dev.3 the skill reports `CallableResidual` removed and `Overloaded`, `NamedInts` added.

### 1.7 Flow (Behavioral profile only)

- **Producer:** `F/lib.rs` `index` (:324) / `module` (:407) and `F/predicate.rs`; adapter `E/ty_flow.rs`
  (`Writer::write` :1035).
- **Provider source:** ty 0.0.14 `ty_python_core`: `semantic_index(db, pf)` (:418); per scope
  `SemanticIndex::{scope_ids, scope, use_def_map, place_table, parent_scope_id, try_expression_scope_id,
  try_node_scope}`; `UseDefMap::{definitions_with_usage (:454), bindings_at_use (:825), loop_header, definition,
  predicates (:526), range_reachability (:926), reachability_constraints}`; `PlaceExpr::try_from_expr`;
  `ScopedUseId` via `HasScopedUseId`. Second parse from the text pyrefly read, with every `TYPE_CHECKING` token renamed
  to a same-length sentinel (`rename` :293). `ProgramSettings` now carry major.minor version and platform from the
  run context (`program_settings` :392-405) — the "Known gap: ProgramSettings are empty" line in
  `acquisition-and-extraction.md` §4.2 is stale.
  Bespoke on top: predicate→condition translation (`Translator`, TDD paths to `Condition`, runtime view of
  `TYPE_CHECKING`, typing/sys/os/`type`/`__class__` specials supplied from **our lexical resolutions**,
  `E/ty_flow.rs` `runtime` :459), place shapes (`native_place` :363), value-source classification
  (identity/derived/through-call), loop-header expansion, read/write node roles.
- **Relations:** `FlowUse`/`FlowUseObservation`, `FlowDefinition`/`FlowDefinitionObservation`, `ReachingDefinition`/
  `FlowReachingObservation` (per use, each reaching definition with its reachability condition; Unbound/Deleted kept),
  `FlowValueObservation`, `FlowValuePathObservation`, `FlowCallStep`, `FlowRegionObservation`, `FlowTestObservation`,
  `FlowTestLeafObservation`, `FlowAttributeLoadObservation`, `Place`, `PlaceRoot` (Formal / Local …).
- **Coverage/boundaries:** `ResourceRefused`, `UndecodableSource` (own checks), `Failed`+`OutsideProviderModel` when ty
  cannot index the file, `SyntaxError` from ty's parse error count, `ScopeBoundary` when a ty scope/use/definition
  cannot attach to a lexical scope or occurrence, `OutsideProviderModel` for place shapes the model lacks,
  `NativeUnavailable` for unattached reaching definitions (`E/ty_flow.rs:654-674`, :1071-1166). Coverage diagnostic
  reports runtime-view and ty-false reach skips.
- **Not read** (`rg narrowing` over `F/` and `E/ty_flow.rs`: no hit): the per-binding `narrowing_constraint` that
  `bindings_at_use` yields; predicates enter only as tests and reachability atoms. The skill's `facts.md` lists
  `ty.narrowing-constraints` as **used**; that is not supported by the current tree, unless the skill means the shared
  predicate arena.

### 1.8 Docs, Artifacts, Deployment (for completeness)

`FactFamily::Docs` is markdown documents (`E/document_parser.rs`: markdown-rs offsets, mention recognizer against a
vocabulary built from public names and declarations, :1-30). Python docstrings appear only as
`DeclarationObservation.docstring` (an occurrence pointer) and `ParameterDocObservation` (Signatures family).

---

## 2. Bespoke computation that restates something a provider could state

| # | Bespoke computation | Where | What it computes | Stated reason no provider is used | Distinctions it preserves |
|---|---|---|---|---|---|
| B1 | Lexical recognizer | `E/lexical.rs`, `E/lexical_records.rs` | scopes (module/class/function/lambda/comprehension, with evaluation-parent: defaults, decorators, bases, first comprehension iterable in the enclosing scope), binding events with ordinals and kinds, `global`/`nonlocal` retargeting, walrus scope, `__class__` cell, `LOAD_NAME` dual candidates, flow-insensitive candidate sets | DESIGN §B1 and `acquisition-and-extraction.md` §4.2.4 (:361-377): pyrefly's `Bindings` drop statically decided branches (`TYPE_CHECKING`, `sys.version_info`), so a rebinding there is invisible; ruff's semantic model has no public driver (`ruff_python_semantic` not in `Cargo.lock`). §4.2.4's own rule (`ambiguous_binding` on any second binding event) is the named consumer | bindings in pruned branches, kept with `static_branch`/`static_polarity`; every candidate (flow-insensitive) rather than a flow choice; exact occurrence ids of the parse of record; unknown star import → Candidate modality; unresolved as an explicit target. ty's semantic index (already linked) also keeps pruned-branch definitions under reachability constraints, but its scope model is consumed only for Flow, and Flow attaches back **to these lexical scopes** (`E/ty_flow.rs` `self.index.scope(...)`) |
| B2 | Static-branch marking | `lexical.rs` `clause_marks` :213, `static_kind` :248 | per `if`/`elif`/`else` clause: decided or not, kept or pruned, and kind (TypeChecking/VersionInfo/Platform/Constant/Combined) | reproduces pyrefly's `SysInfo::pruned_if_branches` so the marks equal pyrefly's view (test `static_marks_agree_with_pyrefly_pruning`, :1172); the kind classification is ours | which clause pyrefly prunes vs analyzes; kind from the expression tree, never text. Only `if` statements (no conditional expressions, `while`, `match`) |
| B3 | Argument binding | `M/calls.rs` `bind` :1446; `N/binding_normalization.rs` `attempt` :260, `application` :171 | per normalized call alternative × undecorated signature variant: argument→slot bindings, `Bound` / `ProvenIncompatible` / `Undetermined` with reasons | no in-repo statement in code; the skill names ty's argument mapping as the alternative. That mapping is in `ty_python_semantic`, **not linked** (absent from `Cargo.lock`), so not "reachable" today. Pyrefly answers per-call overload choice (`get_chosen_overload_trace`, skill index) but not a slot mapping | refuses binding for implicit/protocol events (`ImplicitEvent`), unproved applicability (receiver/dispatch proofs), unsupported shapes; the outcome is stored and replayable; missing premise is never "Python rejects" |
| B4 | Call normalization (events, receivers, dispatch) | `N/event_normalization.rs`, `N/receiver.rs`, `N/dispatch.rs` | `ReceiverAssessment::ClassOf` — the receiver actual occurrence of a call from call syntax + placement + Pysa receiver fields + effective descriptor; dispatch members from captured override graph + MRO | expression-relative receiver, "never an instance identity" (`receiver.rs:1`); dispatch is "conservative captured override members", complete ancestry never a closed subclass universe (`dispatch.rs:1-2`) | `ReceiverReason` codebook (missing/ambiguous syntax/placement, support disagreement, incomplete coverage); open subclass universes (`OverrideDispatch`) |
| B5 | Decorator / effective-callable handling | `N/callable_normalization.rs` `descriptor` :251, `trait_descriptor` :331, `normalize` :536-610 | the effective callable descriptor: Known only when 0 decorators, or exactly 1 bare decorator lexically resolved to builtin `staticmethod`/`classmethod`/`property` and agreeing with Pysa traits; `>1` decorator → `Unknown(UnsupportedDecorator)` (:567-571); unresolved/shadowed → `ShadowedOrUnresolved` | pyrefly is not asked for the decorated type: `get_all_decorated_functions` is read for `f.undecorated` only, Pysa `decorator_callees` and `get_exported_decorated_function` unused | decorator chain digest and order; shadowing of a builtin decorator name; conflict between syntax and Pysa traits; qualified uncertainty. Also `@overload` is matched by trailing decorator name in syntax (`syntax_records.rs:304-306`) beside Pysa's `is_overload` |
| B6 | Dataclass handling | `N/symbolic_fields.rs` `decorator_options` :765, `standard_target` :703; `E/type_records.rs:1172-1310` | from syntax: a single `@dataclass` / `@dataclasses.dataclass(...)` whose callee resolves (Pysa target) to typeshed `dataclasses.dataclass`, keyword options read as literal bools (`init`, `kw_only`, harmless `repr`/`eq`/`match_args`, refuses `order`/`frozen`/`slots`/... when true); fields from pyrefly metadata | "deliberately small plain initializer model" (`symbolic_fields.rs:1-5`); pyrefly's `DataclassMetadata` (class-level options) is read only to enumerate instance fields; class-level options are not emitted as facts | exact standard decorator identity through import and typeshed; refuses option shapes outside the storage model; per-field `dataclass_flags_of` (default/init/alias/kw_only) is provider-stated |
| B7 | Computed `__all__` | `E/syntax_records.rs` `dunder_all`/`statement` :407-519 | literal names, or "computed" → Exports Partial | design (§4.2.3) calls it a completeness detector for pyrefly's blind spot | per-statement subject boundary; literal-set identity. Pyrefly's `unresolvable_dunder_all_range`, `get_partially_known_dunder_all`, `invalid_dunder_all_entries` (skill index, `Exports`) unused |
| B8 | Import module naming | `syntax_records.rs` `absolute_module` :198; `N/relation_normalization.rs` `imports` :263 | each alias's absolute module name from level + package; normalization matches it **by name** against `DependencyModuleObservation` | pyrefly resolution is used only for the name list (`import_handle`, `pyrefly_stage.rs:1047-1064`), not per alias | relative-import climb past the top → `None`. See unresolved edge U3 |
| B9 | Type term lowering | `E/type_records.rs` `Builder::build` :516 | pyrefly `Type` → content-addressed `TypeTerm` graph; opaque variants with `DisplayOnly` support | model needs content-addressed, provider-neutral terms (DESIGN §3.5.1) | opaque vs structural (`opaque` flag → `OutsideProviderModel`), depth budget (`BudgetReached`), declared vs inferred, `SelfType`, `PartialTypedDict`, untyped alias |
| B10 | Docstring location | `E/docstrings.rs` `locate_description`; `symbol_records.rs` `parameter_docs` :673 | byte span of each parameter description inside the literal (Sphinx and Google headers; "Extended" when pyrefly's Google parser splits an entry) | pyrefly parses text but gives no offsets | exact source span evidence; unlocated → `ProviderDisagreement`. Only `def` docstrings, only parameters; no returns/raises/attributes, no class or module docstring text (module docstring range getters unused) |
| B11 | Occurrence ownership ("caller") | `M/occurrence_owner.rs` (owner rule, DESIGN §15.4), used by `N/entity_normalization.rs` `normalize` :108-150 | innermost declaration whose body holds an occurrence; decorators/defaults/annotations/bases belong to the enclosing owner | "the only definition of caller" | evaluation owner of headers; comprehensions are not declarations. ty's `try_expression_scope_id`/`node_scope` and Pysa `ScopeParent` state related scoping, not this owner rule |
| B12 | Dynamic-access recognition | `M/execution/read_dynamic.rs` `produce` :853, name table :806-814, `__dict__` :1021 | sites of `getattr/hasattr/setattr/delattr/vars/exec/eval/__import__/importlib.import_module`, `__dict__` access; class inspection of the receiver | analysis layer; native target from Pysa resolution, falling back to the lexical reference name or the spelling `importlib.import_module` | mixed or unresolved targets stay `mixed`; `ClassInspection::Unknown`; no pyrefly receiver type used (skill opportunity "typed dynamic receivers", Proposed) |
| B13 | Type-role selection | `type_records.rs:1130-1166` | which occurrences get an expression type (call result, argument, raised, test operand) | consumer-driven (test operands feed `TestOperandTypeLink`) | `MissingEvidence` per occurrence when the trace misses |
| B14 | Flow condition translation | `F/predicate.rs` | ty predicates/TDDs → `Condition` atoms (IsNone, Equals, opaque UNDECIDED/FINALLY/…) with evaluation sites | ADR-0022 conditions; ty's AMBIGUOUS edges are preserved as unknown | runtime-view `TYPE_CHECKING`; synthetic predicates without source coordinates kept opaque |

---

## 3. Normalization consumers

Normalization compute lives in `N/*` (pure functions over typed rows); `crates/cpg-core/src/normalize/mod.rs` only loads
inputs and writes outputs per stage (`entities` :31, `relations` :77, `callables` :131, `receivers` :173, `events` :217,
`bindings` :261, `projections` :306, `coverage` :365). Input inventories are executable macros.

| Stage | Inventory | Consumes (extracted, by family) | Produces |
|---|---|---|---|
| N1 entities | `N/inventory.rs` | Syntax `Occurrence`, `ClassFieldSyntaxObservation`; Signatures `ProviderSymbol`, `ProviderModule`, `SymbolDeclaration[Support]`, `FunctionTraitObservation`, `ClassTraitObservation`, `Signature`, `SignatureParameter`, `ParameterDeclaration`; Types `RecordFieldObservation`, `TypeTerm`; Exports `PublicNameObservation`, `ExportOrigin`, `PublicNameSupport`; Lexical `BindingEvent`; Flow `Place` | `CallableEntity`, `ClassEntity`, `ParameterEntity`, `FieldEntity`, `EntityRef`, `SymbolEntityResolution/Candidate/Premise/Evidence`, `OccurrenceOwnership`, parameter/field links, `PublicExposure[Candidate]` |
| N2 relations | `N/relation_inventory.rs` | Signatures `SymbolObservation`, `ClassAncestryObservation`, `SymbolSequenceMember`; Lexical `ReferenceObservation`, `LexicalResolution`, `LexicalTarget`, `BindingObservation`; Exports `ImportAliasObservation`, `DependencyModuleObservation`; Types `TypeVariable`, `TypeObservation`; Syntax `DeclarationObservation`; Docs mentions; Flow `PlaceRoot`, `FlowTestLeafObservation`; coverage | reference→entity targets/assessments (`references` :197), import→module (`imports` :263), ancestry→entities (:326), mention→entities (:384), type→entity links (`types` :541), type binders (:597), place→entity (:737), test-operand type links + coverage (`test_operands` :793, joins ty test leaves to pyrefly `TestOperand` observations) |
| N3 callables | `N/callable_inventory.rs` | Syntax `DeclarationObservation`, `DeclarationDecorator`, `SyntaxPlacement`; Signatures traits, signatures, shapes; Types `FunctionBodyObservation`; Lexical references/targets/resolutions; N1/N2 outputs | `EffectiveCallableAssessment`, `EffectiveDecoratorMember`, premises/evidence, `SignatureVariant`, `SignatureSlot[Entity]` |
| N3b receivers | `N/receiver_inventory.rs` | the N3 inputs + Calls `CallTarget[Support]`, `CallDestination`, `Receiver`, `CallSyntax[Support]`, `SyntaxPlacementSupport` | `ReceiverAssessment`, evidence, premises |
| N4 events | `N/event_inventory.rs` | the above + `ProviderCallable`, `ProviderCallSite[Support]`, `CallChannel`, `CallResolution[Support|Member]`, `ClassAncestryObservation[Support]`, `SymbolSequence[Member]`, `FunctionTraitSupport`, Flow `FlowValuePathObservation`, `FlowCallStep` | `NormalizedCallEvent`, sources, resolutions, alternatives, `DispatchAssessment/Member/Premise/Evidence`, policy assessments/admissions, `FlowCallEventLink` |
| N5 bindings | `N/binding_inventory.rs` | everything above + `CallArgument`, `SignatureEnumerationObservation/Member/Support`, `SignatureSupport` | `CallBindingAttempt`, `CallBinding`, `BindingSetAssessment`, sources, projections |
| symbolic fields / aspects | `N/symbolic_fields.rs`, `N/callable_aspects.rs` | decorators, call syntax/arguments, literals, import aliases, Pysa targets, record fields | constructor→field source associations (analysis consumers) |

**Where normalization re-derives meaning from syntax because a typed provider fact is absent (observed):**
1. Effective descriptor from decorator syntax + lexical resolution (B5), because no decorated-callable type or
   decorator-application fact is extracted; any decorator other than a lone builtin descriptor is Unknown.
2. Dataclass options from decorator call syntax + literal keyword arguments (B6), although pyrefly's class metadata
   holds them.
3. Receiver actual from call syntax and placement (B4); Pysa states receiver class and implicit-receiver flags but not
   the receiver occurrence.
4. Argument→parameter binding from `CallArgument` × undecorated `SignatureParameter` (B3); Pysa's undecorated
   signature is the only signature fact, so decorated wrappers (`functools.wraps`, framework decorators) bind against the
   undecorated function or not at all.
5. Import alias → module by absolute **name** (B8) rather than by a per-alias provider resolution.
6. Occurrence ownership from placement (B11).
7. Test-operand types: ty supplies the test leaves, pyrefly supplies the types by range; normalization joins them
   (`test_operands`), because no single provider states "type of this predicate operand at this test".

---

## 4. Provider facts reachable but unused (notable, not exhaustive)

"Reachable" = on a crate already linked and a handle the producer already holds. Member names for pyrefly come from the
skill's unused index at **1.4.0-dev.3** (`.claude/skills/python-analyzers/content/project/unused/*.json`, snapshot
`e3e7c78d`); spot-checked absent in the tree with `rg` over `E/` and `F/` (no hit for `decorator_callees`,
`global_variables`, `get_chosen_overload`, `overload_trace`, `get_expected_type`, `enum_metadata`, `is_protocol`,
`is_final`, `metaclass`, `is_explicitly_abstract`, `unused_variables`, `unused_imports`, `find_local_references`,
`goto_definition`, `imported_modules`, `reachable_member`, `end_of_scope`, `declarations_at`,
`unresolvable_dunder_all`, `module_docstring`, `narrowing`). Existence at 1.3.1 not individually verified.

**pyrefly `Transaction`** (holds 10/103 used): `get_type_at`, `get_type`, `get_computed_type_at_range`,
`get_type_at_preserving_declaration`, `get_expected_type_at`, `get_result_type_at`, `get_class_fields`,
`find_definition*`, `goto_definition/declaration/type_definition`, `find_local_references`,
`find_global_references_from_definition`, `get_module_docstring_range`, `docstring_ranges`, `get_all_errors`,
`get_dependency_graph`, `get_transitive_rdeps`, `inferred_types`, `infer_parameter_annotations`, `symbols`,
`semantic_tokens`, `search_exports*`.

**pyrefly `Answers`/`Solutions`/`Bindings`:** `get_chosen_overload_trace`, `get_all_overload_trace` (per-call
overload choice), `get_expected_type_trace`, `get_annotation_type_at` (the upstream replacement for patch change 1),
`try_get_getter_for_range`; `Bindings::{definition_at_position, available_definitions, enclosing_class,
function_has_return_annotation, unused_imports, unused_parameters, unused_variables, module_deletes,
subsequently_initialized}`.

**pyrefly `ClassMetadata`** (4/43 used): `enum_metadata`/`is_enum`, `is_protocol`/`is_runtime_checkable_protocol`/
`protocol_metadata`, `is_final`, `metaclass`/`custom_metaclass`/`is_metaclass`, `is_explicitly_abstract`/`extends_abc`,
`dataclass_transform_metadata`, `slots_info`/`has_explicit_slots`, `total_ordering_metadata`, `deprecation`,
`is_new_type`, `has_base_any`, `pydantic_model_kind`, `django_*`, `keywords`. Class traits today stop at four booleans.

**pyrefly `Exports`:** `unresolvable_dunder_all_range`, `get_partially_known_dunder_all`, `invalid_dunder_all_entries`,
`dunder_all_name_at`, `docstring_range`, `is_implicit_reexport`, `is_submodule_imported_implicitly`.

**pyrefly `report::pysa`:** `export_module_type_of_expressions`, `global_variable::collect_global_variables_for_module`,
`call_graph::resolve_decorator_callees`, `function::get_exported_decorated_function`, `class::get_class_fields`,
`class::get_super_class_member_defining_class`, `types::{is_callable_like, is_bound_method_like, has_superclass}`; plus
fields of structures already built and dropped: `captured_variables`, `global_targets`, `return_type`,
`decorator_callees`, class `fields`.

**ty `SemanticIndex`** (7/39 used): `is_in_type_checking_block`, `imported_modules`, `class_definition_of_method`,
`child_scopes`/`ancestor_scopes`/`visible_ancestor_scopes`, `symbol_is_global_in_scope`,
`symbol_resolves_to_global_scope`, `definitions`/`try_definitions`, `semantic_syntax_errors`,
`has_future_annotations`, `enclosing_snapshot`, `try_unpack`, `is_boolean_test_root`.

**ty `UseDefMap`** (7/31 used): per-binding `narrowing_constraint` (on the `bindings_at_use` iterator already walked),
`narrowing_evaluator`, `applicable_constraints`, `declarations_at_binding`, `bindings_at_definition`,
`end_of_scope_bindings/declarations/reachability`, `all_end_of_scope_symbol_*`, `reachable_member_bindings/
declarations` (attribute members such as `self.x = ...`), `if_chain_start_for_use`, `multi_bindings_at_use`. Imported
`Final` candidate queries are 0.0.16 additions (skill).

**Not reachable without a new crate** (verified absent from `Cargo.lock`): ty type inference, argument binding, call
targets and class fields (`ty_python_semantic`); ruff's semantic model (`ruff_python_semantic`); ruff lint rules
(`ruff_linter`).

---

## 5. Unresolved edges

- U1. The skill claims `ty.narrowing-constraints` are used; the tree reads predicates and reachability only. Confirm the
  intended meaning before treating narrowing as supplied.
- U2. Design §4.2.3 says Exports partiality also uses `unresolvable_dunder_all_range()`; no call exists. Either the
  design or the code is behind.
- U3. `DependencyModuleObservation` is emitted for non-root modules only (`E/pyrefly_stage.rs:1250-1256`), and
  `N/relation_normalization.rs` `imports` matches aliases to those rows by name. An import of another **analyzed** module
  may therefore assess `Unresolved/MissingCorrespondence` unless another path supplies the row. Not traced to a test.
- U4. Calls and Types module coverage collapse every boundary to `MissingEvidence`, including annotation calls stated as
  `OutsideProviderModel`; consumers that read coverage reasons rather than subject boundaries see the coarse reason.
- U5. ty receives major.minor only (micro dropped) while pyrefly gets the full triple; no effect known.
- U6. Version transfer: the skill indexes ty 0.0.16, the brief assumes ty 0.0.14 on ruff 0.0.14. ty-side member lists
  above may include 0.0.16 additions (the skill names the imported-`Final` queries, `try_expression_use_id`,
  `ProgramSettings::virtual_environment`).
- U7. `acquisition-and-extraction.md` §4.2's provider table still uses retired table names (`pysa_functions`,
  `syntax_nodes`, ...); the typed relations above are the current contract.
- U8. Downstream consumers of `static_branch`, `TypeObservation` roles, record fields and flow relations beyond
  normalization (analysis/execution/catalog) were not mapped here (sibling mapper's scope).
