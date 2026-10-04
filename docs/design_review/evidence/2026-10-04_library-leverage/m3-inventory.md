# M3 inventory: extraction (`cpg-extract`) and flow (`cpg-flow`)

Supporting evidence for the [library-leverage review](../../reviews/design_review_library-leverage_2026-10-04.md). Repository-fact inventory returned by the M3 code-mapper lane (read-only, baseline `948b2a88`); published verbatim by the coordinator. No verdicts.

**Baseline:** `948b2a88`. `git log 948b2a88..HEAD -- crates/cpg-extract crates/cpg-flow third_party` is empty, so no file cited here changed after the baseline. I read the code only: nothing was built, tested or edited.

**Historical reasons.** Where a bespoke choice had a reason in the retired review, I cite it as **R0923** (`git show b4ee5cc5:docs/design_review/reviews/design_review_library-leverage_2026-09-23.md`, §6 "Keep as is" and §8 "Rejected"). That review predates two things we now have: the independent latest-Ruff link (`ruff_linter`, including the fork's `semantic_facts`) and ty's semantic index in `cpg-flow`.

## 1. Candidate inventory

### M3-01 — The lexical recognizer: Python scoping, binding and name resolution
- **Location:**
  - `crates/cpg-extract/src/lexical.rs`: module doc 1–26; state 120–204; `enter` 304–636; `bind`/`nonlocal_target`/`class_cell` 695–831; `finish` (ordinals, `__class__` cells) 832–912; `resolve_ref` with the dual `LOAD_NAME` candidates 935–975; `outside` (star sets, then builtins, then unresolved) 977–1018; `resolve`/`free` (skipping class scopes) 1020–1075.
  - Driver: `lexical_records.rs` 1–188, a Ruff `SourceOrderVisitor` that maps each node to an `Id<Occurrence>` by span and kind.
  - Inputs: `pyrefly_stage.rs` 2390–2437 (`outside_names`: `ImplicitGlobal::implicit_globals`, `builtins` exports filtered by `is_public_name`) and 2439–2492 (`star_imports`: `get_wildcard`).
- **Generic contract:** Python scopes (module, class, function, lambda, comprehension); binding events with shadow history; `global`/`nonlocal`; skipping class scopes for free names; evaluation scope of defaults, decorators and the first comprehension iterable; walrus binding; `LOAD_NAME` reads in module and class bodies; star imports; builtins; the `__class__` cell; per-scope ordinals; static-branch marks on bindings.
- **Consumers:**
  - `pyrefly_stage::write_lexical` (2337) produces the `scopes`, `bindings`, `references` and `reference_resolutions` rows.
  - `ruff_lexical::lower` attaches Ruff bindings to the recognizer's events (`source.events`, `ruff_lexical.rs` ~199–230).
  - `ty_flow.rs` `Index::root`/`runtime` (338–560) uses the lexical resolutions to choose place owners and runtime-special names.
  - `cpg-flow/src/lib.rs` 527–533 and 760–766 skip PEP 695 type-parameter scopes because "Not modelled by our lexical recognizer".
  - The §4.2.4 binding rule (ordinals).
- **Invariants carried:**
  - Identity-neutral: generic over `I`.
  - Unknown is explicit: `Target::Unresolved(ObligationKind::UnresolvedTarget)`, and an unknown star import becomes a candidate.
  - A multi-candidate resolution is qualified as a candidate.
  - Deterministic: ordinals by `(scope, start, index)`.
  - Every set of names from outside the module's text comes from Pyrefly (doc lines 18–21).
- **Linked library items used inside:**
  - `ruff_python_ast_latest::{AnyNodeRef, Expr, Stmt}` and `ruff_text_size_latest`.
  - `NativeBranches` (Pyrefly `SysInfo`).
  - Test only: Pyrefly `Ast`, `SysInfo`, `PythonPlatform`, `PythonVersion`.
  - No `ruff_python_semantic` item is used.
- **Stated reasons for bespoke:**
  - `acquisition-and-extraction.md` §4.2.4 (~398–415): "Pyrefly's flow-sensitive `Bindings` are not used … drops branches it decides statically".
  - Same section, 2026-10-03 target: "native populated Ruff contextual observations supplement this source-history recognizer … a bespoke port of Ruff's semantic model remains deferred".
  - `docs/plans/python-analyzer-migration-plan_2026-10-03.md` ~110: "Replace equivalent spelling recognizers only after alias, rebound-name, deferred-annotation and static-branch fixtures establish coverage".
  - The provider table lists it as `lctx-lexical`, mode `recognizer`.
  - R0923 §8 rejected two alternatives:
    - A Ruff `SemanticModel` port: the model is filled only by the crate-private `Checker`, and it keeps one binding per name.
    - Pyrefly `Bindings`, `find_definition` or `Definitions`: they prune branches; the probe found 1 of 3 binding events.
- **What has changed since R0923:**
  - The fork now exposes `ruff_linter::semantic_facts::observe_parsed` (`third_party/ruff-0.16.10.patch` 232–620). It emits the Checker's scopes, bindings (with `shadowed`, `outer_shadowed` and `definition_scope`), references, unresolved names and exports.
  - That output is consumed by `ruff_lexical.rs`, so the "crate-private Checker" obstacle no longer applies.
  - The patch says Ruff "does not fold platform branches" (patch ~263). Ruff's model still keeps one binding per name.
  - ty's `semantic_index`/`UseDefMap` is now linked in `cpg-flow`. It computes scopes, places, definitions and use-def per scope, and it is flow-sensitive and reachability-aware.
- **Repetition elsewhere:** scope, binding and resolution are computed three times per module: by this recognizer, by Ruff `semantic_facts` (M3-02) and by ty's semantic index (`cpg-flow/src/lib.rs` 527–546, 755–855). Closure capture appears a fourth way (M3-16).
- **Tests pinning it:**
  - `lexical.rs` 1102 `static_marks_agree_with_pyrefly_pruning`.
  - `tests/typed_lexical.rs` 189 `static_polarity_is_pyrefly_recursive_pruning` and 230 `each_scoping_rule_resolves_as_python_defines_it`.
  - Also `retired_read_expectations.rs`, `normalized_bindings.rs`, `local_semantics.rs` and `read_channels.rs`.
  - `tests/python_reference_oracle.rs` uses `ty_project`/`ty_ide` as a dev-only reference oracle.
- **What a library would have to provide (observation):**
  - Every binding event per name, including bindings in statically decided or pruned branches, each marked with its branch decision.
  - Python's `LOAD_NAME` multi-candidate semantics in module and class bodies.
  - Builtin, implicit-global and star sets injected from Pyrefly rather than from the library's own lists (R0923 §8 rejected `ruff_python_stdlib` lists as a second authority).
  - Ranges exact enough to attach by span and kind.
  - Explicit unknown and candidate outcomes.

### M3-02 — Ruff `semantic_facts` lowered as a supplement; part of its output dropped
- **Location:**
  - `ruff_context.rs`: 8, 92–102 (`Settings`), 127–129 (`observe`), 152–335 (`Sink` collector, decoding `SemanticModelFlags`).
  - `ruff_lexical.rs`: 15–40 (`capture` keeps Scope, Definition, Binding, Reference and Unresolved only); 70–96 (`binding_kind` maps the native `&'static str` kind names); 98–138 (`binding_site`); 151–496 (`lower`).
- **Generic contract:** turn Ruff's `Checker` semantic output into scope, definition, binding, context and resolution observations.
- **Consumers:**
  - `pyrefly_stage.rs` 991 (`context_rows`) and 1052 (`lower_lexical`).
  - `write_native_lexical` (3034).
  - `syntax_records::declaration` reads `resolved_name` from these rows for `@overload` (427–433).
- **Invariants carried:** native ids stay local; attachment only by span, kind and parent; unlocated rows are counted; Incomplete is explicit; a byte budget is charged.
- **Linked library items used inside:** `ruff_linter::semantic_facts::*`, `ruff_python_semantic::{BindingFlags, SemanticModelFlags}`.
- **Facts observed:**
  - `Fact::Export` (patch 376, 540–543), `Fact::Branch { type_checking }` (patch ~368) and `Fact::Input` are emitted but captured nowhere in `cpg-extract`. The grep for `Fact::Export|Fact::Branch` finds no consumer.
  - Native resolutions are admitted only for same-scope loads without the GLOBAL or NONLOCAL flag (`ruff_lexical.rs` ~405–425). Wildcard-unresolved names become candidates (~440–470).
  - Ruff's binding kinds arrive as strings, because the fork's `Binding.kind` is `&'static str`.
- **Stated reason:** migration plan ~90–113 (narrow entry point; supplement, not replacement).
- **Repetition elsewhere:** M3-01, M3-08 (exports), M3-03 (TYPE_CHECKING branch).
- **Tests pinning it:** `tests/ruff_context.rs` 36/100/134; `tests/native_lexical.rs` 45/153; `tests/typed_ruff_context.rs` 19.
- **What a library would have to provide:** the fork already emits it; this is a consumption gap, not a missing library capability.

### M3-03 — Static branch classification (pyrefly-side)
- **Location:** `native_branches.rs` 13–98 (`observe`/`marks`, joining the Pyrefly-AST `StmtIf` to the latest-Ruff `StmtIf` by `(start, end)` and the test ranges); 102–135 `clause_marks`; 138–165 `static_kind`.
- **Generic contract:** decide each `if`/`elif`/`else` clause statically (TYPE_CHECKING, `sys.version_info`, `sys.platform`/`os.name`, constant) and classify which kind of test decided it.
- **Consumers:** `lexical.rs` (binding `branch` marks); `StaticBranch` records.
- **Invariants carried:** the decision is exactly Pyrefly's: `SysInfo::evaluate_bool` per clause, as `pruned_if_branches` applies it. A clause that is not decided is `None`.
- **Linked library items used inside:** `pyrefly_python::{ast::Ast::if_branches, sys_info::SysInfo::{evaluate_bool, is_type_checking_constant_name}}` and `ruff_python_ast::helpers::any_over_expr` (0.0.14).
- **Bespoke part:** `static_kind` classifies by spelling: a `Name` or `Attribute` named `sys`/`version_info`/`platform`, `os`/`name`, or a TYPE_CHECKING constant name. It does not use resolved bindings.
- **Stated reason:** module doc ("H1 C1"); §4.2.4.
- **Repetition elsewhere:**
  - Ruff `Fact::Branch.type_checking`, unconsumed (M3-02).
  - `ruff_python_semantic::analyze::typing::{is_type_checking_block, is_sys_version_block}` (public at the pin, `analyze/typing.rs` 438/456).
  - ty reachability (`cpg-flow`).
  - `cpg-flow/src/predicate.rs::runtime` (M3-04).
- **Tests pinning it:** `lexical.rs` 1102; `typed_lexical.rs` 189.
- **What a library would have to provide:** the kind of decided test, resolved through bindings, together with Pyrefly's exact decision.

### M3-04 — Runtime evaluation of the predicates in `cpg-flow`
- **Location:** `crates/cpg-flow/src/predicate.rs`: 359–434 `runtime` (compares `sys.version_info` tuples with five-field semantics; `sys.platform ==`/`!=` and `.startswith`; maps `os.name` to `nt`/`posix`); 436–467 `runtime_decides`, `root_is` and `resolved_attr`; 697–735 the `literal`/`literal_set`/`int_tuple` helpers.
- **Generic contract:** evaluate version and platform tests against an analysis context.
- **Consumers:** `Translator::test` (127–167); reachability diagrams (601–695).
- **Invariants carried:**
  - Roots must be lexically resolved spans (`RuntimeBindings`), not spellings.
  - Literal tuples longer than three elements stay undecided.
  - Non-modelled forms give `None`.
- **Linked library items used inside:** `ruff_python_ast_ty` nodes only.
- **Stated reason:** lib.rs doc 19–21 ("runtime override"): ty decides `TYPE_CHECKING` as true while indexing, so the runtime view re-decides it.
- **Repetition elsewhere:**
  - Pyrefly `SysInfo::evaluate_bool` (M3-03).
  - ty's own static-truthiness of `sys.version_info`/`sys.platform` under `ProgramSettings`, which `program_settings` (lib.rs 481) already fills from the same context.
- **Tests pinning it:** `cpg-flow/tests/flow_shapes.rs`; `cpg-extract/tests/typed_flow.rs` 202.
- **What a library would have to provide:** a per-expression static verdict under an explicit runtime view (TYPE_CHECKING false) with resolved roots.

### M3-05 — Runtime-special name resolution (`typing`, `sys`, `os`, the builtins `type` and `isinstance`)
- **Location:**
  - `ty_flow.rs` 438–478 `import_special`: matches `resolved_module` strings such as `"typing" | "typing_extensions"`, `"sys"`, `"os"` against the imported identifier text.
  - `ty_flow.rs` 480–559 `runtime`, including the hard-coded builtin class list `int`, `str`, `bool`, `float`, `bytes`, `object`, `list`, `dict`, `tuple`, `set` (512–526).
  - `cpg-flow/src/predicate.rs` 233–268 `isinstance`: checks `f.id.as_str() != "isinstance"` by spelling, and takes the class as whitespace-stripped source text. By contrast `type_is_builtin` (202–231) requires the lexically resolved span.
- **Generic contract:** decide whether a name load denotes `typing.TYPE_CHECKING`, the `typing`/`sys`/`os` module or a specific builtin.
- **Consumers:** `cpg_flow::RuntimeBindings` (lib.rs 84–96); `predicate.rs` M3-04 and `compare`.
- **Invariants carried:** exact spans; every candidate resolution must agree, otherwise no special.
- **Linked library items used inside:** none directly. It is built from model rows (imports, identifiers, lexical resolutions).
- **Stated reason:** none found beyond lib.rs 19–21.
- **Repetition elsewhere:**
  - Ruff `RuffContextObservation.qualified_name` (from the `semantic_facts` `Node`/`Reference` rows) already supplies resolved qualified names. It is used for `typing.overload` in `syntax_records.rs` 427–433, but not here.
  - Ruff `SemanticModelFlags::TYPE_CHECKING_BLOCK` is decoded (`ruff_context.rs` 233).
  - R0923 §6 kept "the textual `is_overload`". Current code resolves overload through Ruff's qualified name, so that circumstance has changed for overload. `isinstance` is still textual.
- **Tests pinning it:** `local_semantics.rs` 857 `native_builtin_type_predicates_use_supported_operands_and_refuse_shadowed_functions`; `execution_channels.rs` 119.
- **What a library would have to provide:** a resolved qualified name per name-load span (Ruff already emits this), plus a builtin identity drawn from Pyrefly's builtins.

### M3-06 — The `TYPE_CHECKING` same-length sentinel rewrite
- **Location:** `cpg-flow/src/lib.rs` 66–68 (`WORD`, `SENTINEL`); 366–397 `rename` (a lexer pass over `TokenKind::Identifier` that rewrites bytes and refuses modules already using the sentinel); 400–447 `index` (records the original and view digests); 856–864 `unsentinel`; `predicate.rs` 112–121 and 360–369.
- **Generic contract:** make ty index the runtime view in which `TYPE_CHECKING` is false.
- **Consumers:** all ty flow facts.
- **Invariants carried:** byte ranges are preserved; the view and original digests are retained (acquisition-and-extraction.md "Implemented flow provider").
- **Linked library items used inside:** the `ruff_python_parser_ty::parse_unchecked_source` tokens.
- **Stated reason:** ADR-0022 (cited in lib.rs 19–21); ADR-0117 line 125 ("same-length TYPE_CHECKING token view is explicit").
- **Repetition elsewhere:** sentinel handling is spread across three places.
- **What changed:** the fork already patches `ty_python_core` (`narrowing_constraints.rs`, patch 652–791 adds `precision_lost`), but has no option for the TYPE_CHECKING decision.
- **Tests pinning it:** `cpg-flow/tests/flow_shapes.rs`; `typed_flow.rs`.
- **What a library would have to provide:** a ty setting or patch that treats `TYPE_CHECKING` as runtime-false (or undecided) in reachability.

### M3-07 — Hand lexer to strip comments from Python source text
- **Location:** `cpg-flow/src/predicate.rs` 737–779 `strip_comments`, a quote and triple-quote state machine. Callers are at 100, 530, 551 and 594 (opaque atom text).
- **Generic contract:** remove `#` comments from an expression's source while respecting string literals.
- **Consumers:** the text of `Atom::opaque`, which feeds the condition atoms.
- **Invariants carried:** deterministic text, so that repeated tests produce equal atom text.
- **Linked library items:** none inside. `cpg-flow` already holds the parsed token stream (`ParsedModuleRef`, `TokenKind`, lib.rs 43).
- **Stated reason:** none found.
- **Bespoke-specific edge cases observed:** string prefixes, and f-strings with nested quotes in 3.12+ syntax, are not modelled specially.
- **Tests:** none found by grep for `strip_comments` in tests.
- **What a library would have to provide:** comment ranges per range (Ruff token `Comment` kinds, `ruff_python_trivia::CommentRanges`, or `ruff_python_index::Indexer`, which is linked in `cpg-extract` only for diagnostics).

### M3-08 — Syntactic `__all__` interpretation
- **Location:** `syntax_records.rs` 355–396 (`is_dunder_all`, `StringSubset`, `string_sequence`, including `+` and starred); 534–571 (`dunder_all`); 662–692 (`__all__.extend`/`.append` mutation at module level); rows `DunderAllObservation` and `computed_all`.
- **Generic contract:** extract `__all__` names from assignment, augmented and annotated assignment, `extend`/`append`, flagging computed or partial forms.
- **Consumers:** the `export_syntax` rows; the subject boundary for a computed `__all__` (`pyrefly_stage.rs` doc 21).
- **Invariants carried:** never guesses names; literal versus partial is explicit.
- **Linked library items used inside:** Ruff AST only.
- **Repetition elsewhere:**
  - `public_records.rs` 92–140 uses Pyrefly `Definitions::new(...).dunder_all.entries` (`DunderAllEntry`), `Exports::get_partially_known_dunder_all`, `explicit_dunder_all_names` and `unresolvable_dunder_all_range`.
  - Ruff `SemanticModel::extract_dunder_all_names` and `DunderAllFlags` (`ruff_python_semantic/src/model/all.rs` 10–75, public at the pin).
  - Ruff `Fact::Export`, emitted by the fork and unconsumed.
- **Stated reason:** R0923 §6 kept "the independent `__all__` detector (§4.2.3)". It was then the second, independent assertion beside Pyrefly. Since then Ruff's export facts became available through the linked fork.
- **Tests pinning it:** `tests/native_exports.rs` 67/167/228/269/352; `normalized_entities.rs`.
- **What a library would have to provide:** exact per-name ranges and flags for invalid or computed formats (Ruff provides `DunderAllFlags`; the fork emits per-name ranges).

### M3-09 — Locating a docstring parameter description
- **Location:** `docstrings.rs`: 8–14 `docstring` (the first statement as a string literal); 27–39 `literal_body` (prefix and quote stripping); 41–74 `leading_spaces`/`skip_blank`/`entry_header` (Sphinx `:param` and Google `name (type):` headers); 75–131 `locate_description`. Driver: `symbol_records.rs` ~684–757.
- **Generic contract:** the byte span of a parameter's description inside a docstring literal.
- **Consumers:** `ParameterDocObservation`, with `Evidence::SourceSpan`.
- **Invariants carried:**
  - The span must reproduce Pyrefly's parsed text exactly (`Exact`) or as a prefix (`Extended`).
  - Ambiguous or missing gives `None`, which is a boundary.
- **Linked library items used inside:** `pyrefly_python::docstring::parse_parameter_documentation`, which returns text only.
- **Stated reason:**
  - Module doc ("slice 2.1 review F2, F3").
  - R0923 §7 deferred Pyrefly's parser itself ("Duplicates nothing we have").
  - Pyrefly gives no ranges.
- **Library surface observed:**
  - Ruff's docstring section and Google/NumPy parsing is `pub(crate)` in `ruff_linter::docstrings` (`mod.rs` 9–13 at the pin), so it is not reachable without a patch.
  - `ruff_python_ast::str::leading_quote` is public (`str.rs` 303).
- **Coverage observation:** there is no NumPy-style header recognition.
- **Tests pinning it:** `docstrings.rs` 133–209 (four unit tests); typed symbol tests.
- **What a library would have to provide:** a ranged parameter-description parse matching Pyrefly's segmentation.

### M3-10 — Syntax field placement by range containment
- **Location:** `typed_syntax.rs` 199–225 (`Frame::place`: the first field whose range contains the child, else `Child`, plus a per-field ordinal); 441–649 `fields(node)`, about 200 lines of per-node field ranges.
- **Generic contract:** label each child with its parent's field name and ordinal.
- **Consumers:** `SyntaxEvent.placement`, which is stored identity; `Spans.parent`/`children`; `ruff_lexical::binding_site`; and `SyntaxField` checks.
- **Invariants carried:** deterministic; exhaustive `NodeKind` mapping at 772 (`deny(clippy::wildcard_enum_match_arm)`).
- **Linked library items used inside:** `ruff_python_ast_latest` node accessors.
- **Stated reason:**
  - Module doc 1–8 (plan A4).
  - R0923 §6 kept "the exhaustive `NodeKind` match" and "the structural-path syntax ids".
  - R0923 §6 also kept the `types.rs` term builder because "Pyrefly's visitor loses child roles, ordinals and parameter names". That reason maps to `type_records.rs` `Builder` (300–1377), not to this code.
- **Search limits:** I found no field-label API in Ruff's `source_order` visitor, which visits children without field names. I did not search exhaustively for generated field metadata in `ruff_python_ast/src/generated.rs`.
- **Tests pinning it:** `typed_syntax_shapes.rs`; insta snapshots.
- **What a library would have to provide:** a child traversal that yields `(field, index, node)`.

### M3-11 — Import-target resolution composed by hand over Pyrefly
- **Location:**
  - `pyrefly_stage.rs` 1724–1784 `resolve_import`: for `from base import member`, it tries `get_type_at_preserving_declaration`, which yields `Type::Module` and its parts joined with `.`; then the `base` exports via `contains_key(member)`; then `base.member` as a module.
  - `pyrefly_stage.rs` 789–794: `is_package` is derived from the path suffix `__init__.py`/`__init__.pyi`.
  - `syntax_records.rs` 300–314 `absolute_module` calls `ModuleName::new_maybe_relative`, a library call. R0923 §6 kept it for that reason.
  - `syntax_records.rs` 615–637 builds the `spelling` with `format!` and `"."`.
- **Generic contract:** resolve an import alias to a module (submodule versus attribute).
- **Consumers:** `ImportAliasObservation`; `Natives` resolutions.
- **Invariants carried:** unresolved is explicit (`natives.unresolved`); resolution reads captured inputs only.
- **Linked library items used inside:** `Transaction::{import_handle, get_exports, get_type_at_preserving_declaration}`, `ModuleName`.
- **Repetition elsewhere:**
  - `public_records.rs` 96 uses `handle.path().is_init()` for the same package test.
  - ty `ImportFromSubmodule` definitions are distinguished in `cpg-flow/src/lib.rs` 774–776.
- **Stated reason:** none found for the fallback order.
- **Tests:** `native_exports.rs` 269 `exact_alias_lookup_retains_namespace_bundle_unresolved_and_native_parse_loss`.
- **What a library would have to provide:** Pyrefly's own binding resolution for an import alias (module versus member) at the alias range.

### M3-12 — Reconstructing qualified names from Pysa parent chains
- **Location:** `symbol_records.rs` 204–211 `parent_key` (string keys `F:{n}`); 212–228 `top_level`; 230–270 `qualified_names` (walks parents, detects cycles, joins with `.`); 271–276 `qualified`. Caller: `pyrefly_stage.rs` 2006.
- **Generic contract:** the dotted lexical path of a class or function definition.
- **Consumers:** the dependency keep set in `definitions` (`pyrefly_stage.rs` 1893–2043).
- **Invariants carried:** a cycle or missing parent gives no name.
- **Linked library items used inside:** Pysa `ScopeParent`, `ClassId::to_int`, `FunctionId::serialize_to_string`.
- **Repetition elsewhere:** `pyrefly_python::qname::QName` is used in `type_records.rs` 29. Ruff `RuffDefinitionObservation` carries the parent chain (`ruff_lexical.rs` 263–291).
- **Stated reason:** none found.
- **Search limits:** I did not check whether Pysa exports a qualified name directly.
- **What a library would have to provide:** a qualified name per Pysa definition.

### M3-13 — Packaging and environment reading
- **Location:**
  - `library.rs` 29–45 `normalize` (PEP 503); 47–50 `analyzer_readable`; 113–170 `uv.lock` partial serde (sha256 prefix strip at 162); 172–180 `pyvenv` (`key = value` split); 182–188 `version_triple` (split on `.`); 195–217 `record_entries` (csv); 219–242 `installed` (dist-info directory name parsed with `rsplit_once('-')`); 244–267 `check_source`.
  - `acquisition.rs` 43–45 `metadata_file`; 144–315 `library_inventory`: site path `lib/pythonX.Y/site-packages` (177–180, POSIX layout), RECORD-hash cross-checks, `walkdir` closure, and `platform: std::env::consts::OS` (310); 556–580 streaming sha256 compared as base64 URL-safe without padding.
- **Generic contract:** installed-distribution discovery, RECORD verification, lock reading and name/version normalization.
- **Consumers:** `InputInventory` and capture; `deployment.rs` identity.
- **Invariants carried:**
  - Every frozen RECORD-listed byte must match.
  - Location-independent digests.
  - Refusal (not omission) on mismatch.
- **Linked library items used inside:** `toml`, `csv`, `sha2`, `base64`, `walkdir`, `globset`, and `pep508_rs` via `deployment_parser::requirement`.
- **Repetition elsewhere:** `pep508_rs::PackageName::from_str` (which normalizes) is already used in `deployment_parser.rs` 125, beside the hand `normalize`.
- **Stated reasons (R0923 §6):** kept "PEP 503 normalization", "`sha2`/`base64`, the encoding `RECORD` dictates" and "the environment-scrubbed `uv` call (uv has no library API)".
- **Coverage observations:** only the `sha256=` RECORD algorithm is accepted (`library.rs` 211); the dist-info directory name is parsed without reading METADATA `Name`.
- **Tests pinning it:** `library.rs` 273/281/478–515/532/547; `tests/acquisition.rs`, `tests/capture.rs`.
- **What a library would have to provide:** dist-info and RECORD reading (with hash verification) plus normalized names and versions from METADATA, and the site-packages location from the venv.

### M3-14 — Deployment metadata and entry-point interpretation
- **Location:** `deployment_parser.rs`: 47–77 (METADATA header folding over `mailparse::parse_headers`); 129–135 (the metadata-version allowlist `1.0`–`2.5`); 148–212 `entry_points` (`rust-ini`, then hand validation of the object reference: `split(':')`, segments are identifiers, and extras in brackets); 216–286 `configuration` (JSON pointer list; hand normalization of `..`/`.` path segments at 253–270).
- **Generic contract:** core-metadata fields, entry-point syntax and launch-config fields.
- **Consumers:** `deployment.rs` 305+ stage; `DeploymentObservation`.
- **Invariants carried:** original text kept; failures are local with a diagnostic; there is no environment expansion.
- **Linked library items used inside:** `mailparse`, `pep508_rs` (`Requirement`, `VersionSpecifiers`, `Version`, `PackageName`, `ExtraName`), `rust-ini`, `serde_json`.
- **Stated reason:** module doc ("Pure interpretation … No environment expansion").
- **Tests:** `deployment_parser.rs` 292–336; `tests/typed_deployment.rs`.
- **What a library would have to provide:** a typed core-metadata parser and an entry-point object-reference parser.

### M3-15 — Markdown mention recognizer helpers
- **Location:** `document_parser.rs` 102–107 `title` (reads the frontmatter `title:` line, not YAML); 126–137 `is_dotted` (ASCII identifier test); 139–144 `distinctive` (heuristic); 158–215 `recognize`; 217–247 `tokens` (hand dotted-identifier tokenizer). Markdown is parsed twice, at 424–445 (`python_blocks`, capture) and at 846–850 (document stage).
- **Generic contract:** frontmatter field, Python identifier test, prose tokenization and vocabulary matching.
- **Consumers:** the `docs` family (`lctx-docs` recognizer, provider table).
- **Invariants carried:** spans are byte offsets from mdast `position`.
- **Linked library items used inside:** `markdown::{to_mdast, ParseOptions::mdx, mdast::*}`.
- **Stated reason:** acquisition-and-extraction.md provider table ("our mention recognizer (`lctx-docs`)"). The mention policy itself is domain.
- **Tests:** `tests/typed_documents.rs`.
- **What a library would have to provide:** YAML frontmatter parsing; a Unicode Python identifier predicate (e.g. `ruff_python_stdlib::identifiers::is_identifier`, pub at the pin, `identifiers.rs` 7); multi-pattern matching.

### M3-16 — Closure capture determined by several providers
- **Location:** `lexical.rs` 1045–1075 (`free` gives `captured`) and 811–831 (the `__class__` cell); `capture_records.rs` 46–114 (Pysa `ModuleCapturedVariables`, Global/Local); `cpg-flow/src/lib.rs` 1023–1136 `capture_snapshot` (ty `EnclosingSnapshotResult`).
- **Observation:** each answers a different question (lexical capture, Pysa capture projection, ty capture timing). The repetition is recorded as fact only.
- **Tests:** `typed_captures.rs`; `cpg-flow/tests/capture_timing.rs`.

### M3-17 — Places rendered twice in `cpg-flow`
- **Location:** `lib.rs` 453–479 `native_place` (structured root, attribute and item segments, depth 256); `predicate.rs` 105–125 `place`/`plain`, which uses `PlaceExpr::try_from_expr(e).to_string()`, swaps in the source text for the sentinel, and applies the policy "at most two dots, no `[`".
- **Linked library items used:** `ty_python_core::place::PlaceExpr`.
- **Stated reason:** `place` doc cites DESIGN §3.9 (the policy limit).
- **Observation:** these are two renderings of one ty concept.

## 2. Linked but thinly used items (actual `use` paths)

| Crate (manifest) | Items imported |
|---|---|
| `ruff_python_semantic` (cpg-extract) | `BindingFlags` (`ruff_lexical.rs` 8) and `SemanticModelFlags` (`ruff_context.rs` 178). These are flag decoders only; `SemanticModel` and `analyze::*` are not referenced directly. |
| `ruff_linter` | `semantic_facts::{observe_parsed, Settings, Sink, Fact, Binding, NodeOrigin, StopReason, Incomplete, TraversalStats}`; `linter::check_path`, `Locator`, `directives`, `package::PackageRoot`, `registry::Rule`, `settings::{LinterSettings, TargetVersion, flags::Noqa, rule_table::RuleTable}`, `source_kind::SourceKind`, `suppression::Suppressions` (`diagnostic_records.rs` 11–18). |
| `ruff_db` (cpg-extract) | `diagnostic::{Diagnostic, Severity, Span}` |
| `ruff_python_codegen_latest` / `ruff_python_index_latest` | `Stylist` and `Indexer`, in `diagnostic_records.rs` only |
| `ruff_python_parser_latest` | `parse_unchecked`, `ParseOptions`, `Parsed` |
| `ruff_python_ast_latest` | `visitor::source_order::{SourceOrderVisitor, TraversalSignal, walk_*}`, `statement_visitor`, `visitor::{Visitor, walk_stmt}`, node types, `NodeKind`, `ExprContext`, `PySourceType`, `PythonVersion`, `ExprRef` |
| `ruff_text_size_latest` | `Ranged`, `TextRange`, `TextSize` |
| `ruff_python_ast` 0.0.14 (Pyrefly's) | `name::Name`, `helpers::any_over_expr`, visitors, `Stmt`/`StmtIf`/`Expr`/`ModModule`/`PySourceType`, `AnyNodeRef::Parameter` |
| `ruff_source_file` 0.0.14 | `LineIndex`, `OneIndexed`, `PositionEncoding`, `SourceLocation` (`symbol_records.rs` 26, Pysa location inverse) |
| `ruff_text_size` 0.0.14 | `TextRange`, `TextSize`, `Ranged` |
| `pyrefly` | `state::{State, Require, Transaction, lsp::{DefinitionMetadata, FindPreference}}`. Transaction methods: `run`, `get_ast`, `get_module_info`, `get_exports`, `get_exports_data`, `get_wildcard`, `import_handle`, `get_type_at_preserving_declaration`, `get_answers`, `get_solutions`, `get_errors`, `set_pysa_reporter`, `resolve_pysa_solutions`, `observe_context_exit`, `observe_terminal_call`. Also `report::pysa::{call_graph::*, class::*, function::*, context::*, captured_variable::*, override_graph::*, module::*, location::PysaLocation, scope::ScopeParent, export_module_*}`; `export::{definitions::{Definitions, DunderAllEntry}, exports::{ExportLocation, Exports}}`; `commands::coverage::collect::{EXCLUDED_MODULE_DUNDERS, is_public_module, is_public_name, trace_export_origin}`; `alt::{observations, answers::{Solutions, NativeOverloadSelection}, types::class_metadata::DataclassKind}`; `binding::binding::{IsAsync, ClassFieldDefinition, BindingClass, Key*}` |
| `pyrefly_python` | `module_name::ModuleName` (with `from_str` and `new_maybe_relative`), `module_path::{ModulePath, ModulePathDetails}`, `sys_info::{SysInfo, PythonPlatform, PythonVersion}` (`evaluate_bool`, `pruned_if_branches`, `is_type_checking_constant_name`), `ast::Ast::{if_branches, locate_node, parse}`, `docstring::parse_parameter_documentation`, `symbol_kind::SymbolKind`, `qname::QName` |
| `pyrefly_types` | `types::*`, `callable::*`, `class::{Class, ClassKind}`, `literal::Lit`, `quantified::*`, `tuple::Tuple`, `type_alias::TypeAliasData`, `type_var::*`, `typed_dict::TypedDict`, `special_form::SpecialForm`, `globals::ImplicitGlobal`, `function::{FunctionKind, BodyKind}` |
| `pyrefly_config` / `pyrefly_build` / `pyrefly_util` | `config::{ConfigFile, ConfigSource}`, `finder::ConfigFinder`, `error_kind::{ErrorKind, Severity}` / `handle::Handle` / `arc_id::ArcId`, `thread_pool::ThreadCount` |
| `ty_python_core` (cpg-flow) | `semantic_index`, `UseDefMap`, `FileScopeId`, `ProgramFile`, `EnclosingSnapshotResult`, `ast_ids::HasScopedUseId`, `definition::{Definition, DefinitionKind, DefinitionState}`, `place::{PlaceExpr, PlaceExprRef}`, `platform::PythonPlatform`, `predicate::{Predicate, PredicateNode, PatternPredicate, PatternPredicateKind}`, `program::{Program, ProgramSettings, FallibleStrategy}`, `reachability_constraints::ScopedReachabilityConstraintId`, `scope::{NodeWithScopeKind, NodeWithScopeRef}`, `NarrowingEvaluator`, `narrowing_constraints::{NarrowingConstraints, ScopedNarrowingConstraint, InteriorNode}` |
| `ty_module_resolver` | `SearchPathSettings`; tests only: `ModuleName`, `resolve_module_confident` |
| `ty_vendored`, `ruff_db` (cpg-flow) | `Db`, `files::{Files, system_path_to_file}`, `system::{DbWithTestSystem, DbWithWritableSystem, System, TestSystem, SystemPathBuf}`, `vendored::VendoredFileSystem`, `parsed::{parsed_module, ParsedModuleRef}` |
| `ruff_python_ast_ty` / `parser_ty` / `text_size_ty` | `token::TokenKind`, visitors, node types / `parse_unchecked_source` / `Ranged`, `TextRange` |
| dev-only | `ty_ide`, `ty_project`, `ruff_ranged_value` (`tests/python_reference_oracle.rs`); `ruff_index_latest::Idx` (`narrowing.rs` tests) |

## 3. Domain or adapter, not a candidate
- `type_records.rs`: lowers Pyrefly `Type` into `TypeTerm`/signatures while the transaction is live. R0923 §6 kept the term builder (Pyrefly's visitor loses roles, ordinals and parameter names); CinderX was rejected in §8.
- `call_records.rs`: Pysa `CallCallees`/`Unresolved` mapped to codebooks.
- `protocol_records.rs`: Ruff visitor queries plus the fork's `observe_context_exit`/`observe_terminal_call`.
- `diagnostic_records.rs`: `ruff_linter::check_path` and Pyrefly errors lowered as they are.
- `parameter_definition_records.rs`: Pyrefly LSP definition answers.
- `capture_records.rs`: Pysa capture map joins.
- `public_records.rs`: uses Pyrefly `Definitions`/`Exports`/`trace_export_origin` directly. This is the library; it is listed here only as the counterpart to M3-08.
- `natives.rs`: interns provider modules and symbols from `ModulePathDetails`.
- `assembly.rs` and `syntax_records::Spans` (61–263): the exact span and kind attachment index; ADR-0089 domain invariant.
- `syntax_records` declarations, parameters, class fields and call syntax: AST lowering.
- `typed_syntax` `details`/`literal_of`/`syntax_kind`: AST lowering.
- `cpg-flow` `narrowing.rs` (lowers ty `NarrowingConstraints`), `native.rs` (atom codebook), `db.rs` (salsa database) and `lib.rs` `Walk`/`sources`/`Visitor` (the value-source identity/derived policy over ty definitions). These are adapter plus domain policy (ADR-0022).
- `ty_flow.rs` `Writer` (739–1773): typed attachment and qualification.
- `bundle.rs`: the provider framework (thread per stage, tokio mpsc window 2, panic abort, build digest). Domain orchestration.
- `capture.rs`: a checked frozen copy with stat stamps and digests (ADR-0089).
- `native_context.rs`, `runtime_scripts.rs`, `logging.rs`: configuration and wiring.

## 4. Search coverage and limits
- **Covered:** every `.rs` under `crates/cpg-extract/src` (not `src/bin`) and `crates/cpg-flow/src`, plus both manifests, the workspace pins, `third_party/ruff-0.16.10.patch` and the list of files the Pyrefly patch touches.
- **Read in full:** `lexical.rs` (header, resolution and finish sections), `ruff_lexical.rs`, `ruff_context.rs`, `native_branches.rs`, `docstrings.rs`, `public_records.rs`, `natives.rs`, `library.rs` (to line 270), `deployment_parser.rs` (to line 290), and the cited `cpg-flow` predicate and lib sections.
- **Skimmed by item list and targeted reads:** `pyrefly_stage.rs`, `type_records.rs`, `call_records.rs`, `symbol_records.rs`, `typed_syntax.rs`, `syntax_records.rs`, `document_parser.rs`, `acquisition.rs`, `ty_flow.rs`, `capture.rs`, `bundle.rs`, and `lexical.rs` `enter` (304–636, read only in part).
- **Use-path tables:** built by grepping `crate::…` paths and `use` blocks. Fully qualified calls split across lines may be missing; for example `transaction.import_handle` and `get_wildcard` were seen in the code but not counted by the grep.
- **Library visibility checks:** done against the Ruff checkout at `~/.cargo/git/checkouts/ruff-8efe7e87ebf7d7ab/f7bdff6` (fork rev `f7bdff69`). I did not check Pyrefly or Pysa source for qualified-name or ranged-docstring APIs, did not consult the `python-analyzers` skill (that belongs to L2), and did not run probes. Absence claims (no consumer of `Fact::Export`/`Fact::Branch`, no tests for `strip_comments`) come from grep over `crates/cpg-extract` and `crates/cpg-flow` only.
