# L2 — analyzer crates: linked-but-unused surface against bespoke extraction

Lane L2 of the 2026-10-04 library-leverage review. **Static research only; no probe was run.**
The baseline is `main` at `948b2a88`, with concurrent work landing, so line numbers may drift and
symbols are the anchors. Verified 2026-10-04 against the Cargo git checkouts of the pinned forks:

- Ruff/ty fork `paul-heyse/ruff@f7bdff69`, which is upstream tag `0.16.10` (`3265ed1f`) plus one
  aggregate patch;
- Pyrefly fork `paul-heyse/pyrefly@63cda076`, which is upstream `80cec3f5` (1.4.0-dev.3) plus one
  patch.

Evidence kinds are marked as follows. **src** means read in the pinned source. **repo** means read
in the repository's code. **skill** means taken from the `python-analyzers` brief or probe named.
**upstream** means GitHub release or commit metadata.

## 0. Facts that change the framing

1. **The Ruff `SemanticModel` is already driven in production through a fork seam.**
   - **src:** `ruff_linter/src/semantic_facts.rs` (389 lines, new in the fork patch) is a public
     `semantic_facts`/`observe_parsed` entry.
   - **src:** the seam is backed by a crate-private `check_ast_with_observation`
     (`checkers/ast/mod.rs:3541`). It streams owned `Fact::{Input, Node, Branch, Scope,
     Definition, Binding, Reference, Unresolved, Export}` rows after Ruff's deferred passes, with
     row and work budgets and an explicit `Incomplete`.
   - **repo:** `ruff_context.rs` (`CanonicalSyntax`, `observe_parsed`) and `ruff_lexical.rs`
     (`NativeRows::lower`) consume it. They lower the rows to `RuffBindingObservation`,
     `RuffDefinitionObservation`, `RuffContextObservation`, `LexicalScopeObservation` and a subset
     of `LexicalResolution`, attributed to the Ruff provider. These rows sit **beside**
     `lexical.rs` and do not replace it.
2. **The `python-analyzers` skill's library-context layer is stale on these points.**
   - `show semantic` says "nothing is fed from `SemanticModel`" and "no public driver".
   - `show migration` describes the repo as on Pyrefly 1.3.1 / ty 0.0.14.
   - The `show unused:*` stamps list several members as unused that the repo now calls:
     `get_chosen_overload_trace`/`get_all_overload_trace`, `get_annotation_type_at`, and most
     `ClassMetadata` predicates.
   - Its "upstream tag plus visibility-only patch" invariant no longer describes the Pyrefly fork.
     That patch is +348 lines and adds solver observations (`alt/observations.rs`,
     `observe_context_exit`, `observe_terminal_call`, overload-origin retention in `answers.rs` and
     `overload.rs`).
   - These are skill-maintenance findings. Per the brief, I did not edit the skill.
3. **Pins are at the newest upstream releases (upstream, checked 2026-10-04).**
   - Ruff `0.16.10` was published 2026-10-01 and is the latest. No commits touch
     `crates/ty_python_core` or `crates/ruff_python_semantic` after the tag.
   - Pyrefly `1.4.0-dev.3` was published 2026-10-02 and is the latest. Post-tag `main` holds NumPy
     shape work and one default-severity change (`potential-bad-keyword-argument` → ignore,
     #5083). Hermetic configuration makes the severity change irrelevant.
   - The ty CLI's latest release, `0.0.84` (2026-09-24), is *older* than the ty crates at the Ruff
     tag.

## 1. Capability table

| Crate · item (pinned ref) | Contract / limits | Repo consumer | Fit: what it would replace or enable, and what it loses |
|---|---|---|---|
| `ruff_linter::semantic_facts::{observe_parsed, Fact}` (fork; `semantic_facts.rs`) | Owned rows after deferred passes. Refuses invalid or unsupported syntax and notebooks as `Incomplete`. Platform is recorded, not folded. Builtins come from Ruff's own version list plus `custom_builtins`. `Fact::Branch` classifies only TYPE_CHECKING. | **Used.** `ruff_context.rs`, `ruff_lexical.rs`, `pyrefly_stage.rs` | **In use:** attributed second lexical provider. **Unused rows:** `Fact::Export` is emitted, but `NativeRows::capture` drops it (`ruff_lexical.rs` keeps only Scope/Definition/Binding/Reference/Unresolved); `Fact::Node` is used only for context in `ruff_context.rs`. **Losses if it became primary:** see §2 Q1. |
| `ruff_python_semantic::SemanticModel::resolve_load` (`model.rs:490`) | Returns one `BindingId` per read: the scope's *current* binding at traversal time. Function bodies are deferred, so they see the final module binding. Class scopes are skipped except the immediate parent, `Type` and `DunderClassCell`. PEP 563 forward references prefer the global. | indirectly, through the seam | Cannot supply `lexical.rs`'s candidate sets (all bindings with ordinals) or the module/class-body `LOAD_NAME` dual candidates. The shadow chain (`shadowed_binding`, `shadowed_bindings`) does preserve per-scope binding history, so the *event inventory* is recoverable. |
| `ScopeKind::Generator { kind, is_async }` (`scope.rs:218`) | No node or range | `ruff_lexical.rs` counts these scopes as `unlocated` | Comprehension and `Type` scopes cannot attach by exact span without a further patch field. |
| `SemanticModel::extract_dunder_all_names` + `DunderAllFlags` (`model/all.rs`) | Per-name string ranges. Handles list/tuple, `+` chains, `list()`/`tuple()` resolved as builtins, walrus. A comprehension, `__all__`, or `x.__all__` term yields no names **and no flag**. `.extend`/`.append`/`.remove` are not handled here (no hit in `checkers/ast/mod.rs`). | none (its output reaches us as the dropped `Fact::Export`) | Could add exact per-name ranges to `syntax_records.rs::dunder_all` observations. **It cannot replace that detector:** it loses the `literal` versus computed distinction (an unknown would read as absent, CI-04) and the `.extend`/`.append` cases. The earlier "keep independent detector" verdict holds. |
| `ruff_linter` `docstrings::{Docstring, SectionContexts}` and pydoclint `parse_parameters_google/numpy` (`docstrings/`, `rules/pydoclint/rules/check_docstring.rs:628,681`) | `mod docstrings` and `check_docstring` are **crate-private**. Google and NumPy only; no Sphinx. Ranges cover parameter **names**, not descriptions. | none | Would add NumPy section coverage, which Pyrefly lacks. It does not locate descriptions, which is what `docstrings.rs::locate_description` does. Adoption needs a fork visibility patch plus our own description extent. Low value. |
| `pyrefly_python::docstring::parse_parameter_documentation` (`docstring.rs:598`) | Sphinx and Google → `HashMap<name, text>`, with no ranges and no NumPy | **Used.** `docstrings.rs`, `symbol_records.rs::parameter_docs` | The earlier deferral is now resolved: it is consumed. `docstrings.rs` only recovers byte spans for Pyrefly's text, so it is necessary glue, not duplication. |
| `ruff_python_ast::str::{raw_contents, leading_quote, trailing_quote}`, `AnyStringFlags::opener_len/closer_len` (`str.rs:268–324`, `nodes.rs:1014`) | Exact prefix and quote arithmetic | none | Would replace `docstrings.rs::literal_body`, a hand-rolled prefix/quote scan. Small and lossless. |
| `ruff_python_trivia::{leading_indentation, textwrap::dedent}` | Whitespace utilities | none | Could replace `leading_spaces`/`skip_blank` in `docstrings.rs`. Cosmetic. |
| `pyrefly_python::sys_info::SysInfo::{evaluate_bool, depends_on_sys_info, pruned_if_branches}` (`sys_info.rs:831–1123`) | Decides version/platform/`os.name`/TYPE_CHECKING by **spelling** (`sys`, `os` names). Handles 4–5-field `version_info` tuples with a release level, and boolean combinations. | **Used.** `native_branches.rs::clause_marks`, `lexical.rs` | `cpg-flow/src/predicate.rs::Translator::runtime` re-implements the same decision for the ty view, but **by resolved name**: it is alias-aware and does not decide `version_info` literals longer than 3 fields. **The two disagree by design.** Joining Pyrefly's per-range decision would lose predicate.rs's alias-aware resolution, so it is a cross-check opportunity, not a replacement (§2 Q4). |
| `pyrefly::export::exports::Exports` (patched public), `commands::coverage::collect::{trace_export_origin, is_public_*}` | Partial and explicit `__all__`, unresolvable range, wildcard set | **Used.** `public_records.rs` | Skill opportunity `computed-dunder-all-boundary` is **done** (`public_records.rs` uses `unresolvable_dunder_all_range`). |
| `pyrefly_python::qname::QName::{name_relative_to_module, module_qualified_name}`, `NestingContext::owner_path` | Dotted path through classes **and** functions, with no `<locals>` segment | none (`symbol_records.rs::qualified_names` rebuilds the same dotted path from Pysa parent keys) | It is equivalent only when a `QName` is at hand. Pysa definitions carry ids, not `QName`s, so the bespoke join is reasonable. Neither form is a Python `__qualname__`: both omit `<locals>`. |
| `Transaction::{find_local_references, find_global_references_from_definition}` (`state/lsp.rs:4162, 5379`) | LSP-grade, flow-sensitive, one target per use; transaction-scoped | none (`ty_ide::find_references` serves only as a dev oracle in `tests/python_reference_oracle.rs`) | Enables a cross-reference family, the skill's `glean-cross-references` opportunity. Per DESIGN, it complements `reference_resolutions` and never replaces it. |
| `Answers::get_expected_type_trace`; `ClassMetadata::{enum_metadata, total_ordering_metadata, django_model_metadata, base_class_objects, extends_abc, has_base_any}` | Typed answers and class facts (src list via `show unused`, cross-checked by grep) | none | Would enrich class and type facts (enum members and values, ordering synthesis, `Any` bases as an explicit unknown). It displaces nothing bespoke. |
| `report::pysa::{type_of_expression, global_variable, is_test_module, call_graph::resolve_decorator_callees}` (`report/pysa.rs`: all `pub mod` in the fork) | Collector outputs keyed by `PysaLocation` | none | Mostly duplicates `Answers::get_type_at` (used). `global_variable` would give typed module globals. Converting through `PysaLocation` costs the `symbol_records.rs::Locator` inverse. |
| `ty_python_core::UseDefMap::{bindings_at_use, reachable_bindings, end_of_scope_bindings, all_reachable_symbols}`, `SemanticIndex::{enclosing_snapshot, visible_ancestor_scopes, symbol_resolves_to_global_scope}` (`use_def.rs:998–1352`, `lib.rs:485–837`) | `bindings_at_use` gives **multiple** reaching bindings per use, with reachability and narrowing constraints. `reachable_bindings(place)` gives all scope-reachable bindings (`AssumeBound`). Per scope (TY13). TYPE_CHECKING is decided true (TY03). `version_info` is not decided (TY04). Builtins and class-body→global fallback live in `ty_python_semantic`, not here. | **Used.** `cpg-flow/src/lib.rs` (behavioural profile only) | The one engine that keeps multiple candidates. It could supply the flow-sensitive candidate subset and the scope chain. **To replace `lexical.rs` it lacks** builtins, LOAD_NAME dual candidates, Pyrefly's star-import sets for dependencies not loaded into `FlowDb`, and the catalog-profile availability that Flow is NotRequested in. §2 Q1. |
| `ty_python_core::narrowing_constraints::NarrowingConstraints::precision_lost` (fork, +65 lines) | Flag set at the builder's existing resource fallbacks | **Used.** `cpg-flow/src/narrowing.rs` | Present; not a gap. |
| `ty_ide`, `ty_project`, `ty_python_semantic` (in `Cargo.lock`, dev-only) | Inference-backed navigation. `ty_python_semantic` substance is mostly `pub(crate)` (skill) | dev oracle only | Production use would make ty a second type authority, which plan §5 rejects. Keep as oracle. |
| `ruff_python_index::Indexer`, `Parsed::comment_ranges`, `ruff_python_codegen::Stylist` | Token-derived indices | **Used** in `diagnostic_records.rs` only | No hand-rolled comment or continuation scanner found in `cpg-extract/src` (grep: `'#'`, `.lines()`, `char_indices`). Nothing to displace. |
| `ruff_python_ast::visitor::{source_order, Visitor, statement_visitor}` | Standard walkers | **Used** throughout | No hand-rolled walker found. |
| `ruff_source_file::LineIndex` (embedded 0.0.14) | Position conversion | **Used.** `symbol_records.rs::Locator` | Required to invert `PysaLocation`; correct family. |
| salsa 0.28.5 (`compact_str, macros, salsa_unstable, inventory, ordermap`; no default features) | One `FlowDb` per `cpg_flow::index` call, in-memory `TestSystem` | `cpg-flow/src/db.rs` | Incremental reuse and parallel features are not used and not needed for one-shot compilation. No displacement. |

## 2. Per-question findings

### Q1. Lexical recognizer (`lexical.rs`)

These are the prior (2026-09-23) rejection reasons, reassessed at the current pins as the
coordinator asked.

- **"`ruff_python_semantic` is 0.0.11."** → **No longer holds.** The independent latest family is
  0.0.16 / `ruff_linter` 0.16.10 at `f7bdff69`.
- **"Only the crate-private `Checker` fills the model, so a port would exceed `lexical.rs`."** →
  **Partly holds, and the consequence no longer does.**
  - `Checker` is still `pub(crate) struct Checker` (`checkers/ast/mod.rs:195`), and `check_ast` is
    still `pub(crate)` (src).
  - A narrow fork patch already exposes it: +115 lines in `checkers/ast/mod.rs` plus a 389-line
    `semantic_facts.rs`, with a no-observer lint-parity test.
  - So no port is needed. The real cost is fork maintenance, which is already being paid.
- **"It keeps one binding per name, so candidate sets stay ours."** → **Holds for Ruff.**
  - `resolve_load` returns one binding per read, and order depends on traversal (src).
  - Deferred function bodies resolve to the scope's last binding.
  - There is no `LOAD_NAME` dual candidate for a module or class body read before a later local
    binding.
  - `ruff_lexical.rs` already admits Ruff resolutions only for uncaptured same-scope loads without
    the GLOBAL/NONLOCAL flags (repo, comment at the `LexicalResolution` push). That restriction is
    the repository's own evidence of this limit.
- **ty's index does keep multiple candidates.**
  - `bindings_at_use` is multi-valued and flow-sensitive under constraints.
  - `reachable_bindings` approximates the scope-wide inventory.
  - Its limits:
    - it is per scope;
    - class-body→global fallback and builtins live in `ty_python_semantic`, which is not linked as
      a production dependency;
    - star imports resolve only against modules present in `FlowDb`;
    - TYPE_CHECKING needs the sentinel rename;
    - it runs today only under `--profile behavioral`.
- **"Pyrefly `Bindings` prune static branches."** → **Holds.**
  - At `63cda076`, `binding/function.rs:1053` still iterates `pruned_if_branches` (src).
  - The rebinding rule in `acquisition-and-extraction.md` §4.2.4 depends on events inside pruned
    branches.

What each alternative would preserve or lose against `lexical.rs`:

| `lexical.rs` distinction | Ruff seam | ty index |
|---|---|---|
| Scopes with exact owner span | Module/Class/Function/Lambda: yes. Generator/Type: **no range** (unlocated). | Yes, all kinds, including Comprehension/TypeParams. |
| Binding events in all branches, with ordinals | Yes: all branches are visited, and the shadow chain is kept. | Yes, as definitions. |
| Static-branch marks | No. These come from `native_branches.rs`, so they could be joined by range. | Constraint predicates; `version_info` is undecided. |
| global/nonlocal binding in the declared scope | Partially: flags only, and the repo refuses them as authority. | Yes (`symbol_is_global_in_scope`). |
| Class-scope skipping | Yes | Yes (`visible_ancestor_scopes`) |
| Comprehension first iterable, walrus | Yes | Yes |
| `LOAD_NAME` dual candidates | **No** | **Not in the index** |
| Flow-insensitive candidate set | **No** (one binding) | Derivable; flow-sensitive by default |
| Builtins and implicit globals = Pyrefly's | **No** (Ruff list; `custom_builtins` can only add) | **No** (typeshed via semantic) |
| Star-import name sets | Only an "unresolved via wildcard" flag | Only for loaded modules |
| Identity: one canonical parse, exact span | Same latest parse (`observe_parsed`) | Second parse, same bytes, sentinel view |

**Assessment.** At these pins neither library displaces `lexical.rs` without losing explicit
distinctions: candidate sets, `LOAD_NAME`, Pyrefly-sourced outside names, and catalog-profile
availability. The design's current position, "native Ruff observations supplement the
source-history recognizer" (`acquisition-and-extraction.md` ~l.412), matches the evidence.

Smaller, lossless leverage:

- (a) Lower `Fact::Export`. Today it is captured by the sink and dropped.
- (b) Add a scope-range field for `Generator`/`Type` to the existing Ruff patch, so comprehension
  scopes attach.
- (c) Pass Pyrefly's builtin names through `ContextSettings::custom_builtins`. This narrows
  spurious Ruff `Unresolved` disagreements, though it cannot remove Ruff-only builtins.

`ruff_lexical.rs` itself depends on the recognizer's `BindingEvent`s for attachment
(`source.events`). Retiring `lexical.rs` would therefore also mean minting events from Ruff
bindings.

### Q2. Docstrings, public API, names

- **Docstrings.**
  - Pyrefly supplies the text (Sphinx/Google). Ruff's richer section model is crate-private,
    covers Google/NumPy only, and gives name ranges, not description extents.
  - `docstrings.rs` is span-recovery glue for Pyrefly text, so keep it.
  - Use `ruff_python_ast::str` / `AnyStringFlags` instead of `literal_body`.
  - NumPy parameter docs are uncovered by every provider wired today. That is a coverage gap, not
    a leverage gap.
  - Escapes in non-raw docstrings make the raw-source match fail closed (`None`). This is honest.
- **`__all__`.** As in the table. Pyrefly `Exports` owns semantics (used). The bespoke syntactic
  detector keeps `literal`/computed. Ruff's extractor would add only per-name ranges.
- **`ruff_python_stdlib`.** It is now implicitly present through the seam's builtin decisions. The
  "second authority" risk from the 2026-09-23 rejection has become concrete for Ruff `Unresolved`
  rows: they come from Ruff's list, not Pyrefly's, and are emitted as `LexicalResolution` under
  Ruff's qualification. This is attributed (CI-01), but disagreement with Pyrefly-based rows should
  be surfaced, not deduplicated away.
- **Module names.** `syntax_records.rs::absolute_module` already calls Pyrefly's `ModuleName`.
  `ty_module_resolver` would be a second resolver; do not use it.

### Q3. Pyrefly session and Pysa collectors

- The repo already uses the following:
  - `get_answers`, `get_type_at` and `get_type_trace`;
  - both overload traces and `get_annotation_type_at`;
  - most `ClassMetadata` predicates (dataclass, pydantic, named tuple, typed dict, protocol and
    runtime-checkable, enum flag, final, abstract, new-type, slots, attrs, metaclass, deprecation,
    dataclass transform);
  - `find_definition`, Pysa definitions, call graphs, MRO, the captured-variable and
    override-graph collectors;
  - the patched exit/terminal observations.
- No remaining Pyrefly surface found would *displace* bespoke lowering in `type_records.rs` or
  `call_records.rs`. The earlier review's reason for keeping the term builder still applies:
  Pyrefly's visitor loses child roles and ordinals.
- Enrichment-only candidates:
  - `find_local_references`/`find_global_references_from_definition` (cross-references);
  - `enum_metadata` (member values);
  - `total_ordering_metadata`, `has_base_any` (explicit unknown base), `get_expected_type_trace`;
  - Pysa `global_variable`.
- The skill's `overloaded-type-terms` and `computed-dunder-all-boundary` opportunities appear done
  in code. Re-stamping is a skill task.

### Q4. ty and salsa

- `predicate.rs` and `narrowing.rs` **lower** ty's TDDs and narrowing graphs into `Condition`s. They
  do not re-implement them.
- The one re-implementation is `Translator::runtime` (sys/os/TYPE_CHECKING folding), duplicated by
  Pyrefly `SysInfo`. ty's own folding lives in `ty_python_semantic` inference, which is not a
  production dependency.
- Recommended: a range-joined cross-check against `native_branches::clause_marks`. It would flag
  alias or spelling disagreements and the 4–5-field `version_info` literals that predicate.rs
  leaves undecided, and it would not change authority.
- The `ty_ide` references remain the right dev oracle. Production would pull in inference, which
  plan §5 rejects.
- salsa: nothing to adopt.

### Q5. Canonical syntax utilities

- Ruff visitors, `Indexer` and `Stylist` are used where needed. No hand-rolled comment,
  continuation, line-index or codegen equivalents were found in `cpg-extract/src` or
  `cpg-flow/src`.
- The only small displacement is the docstring prefix/quote/indent arithmetic.
- `cpg_flow::rename` (the TYPE_CHECKING sentinel) has no ty configuration alternative (skill
  TY03/TY18).

### Q6. Upgrade deltas

- There are no newer upstream releases for any of the three.
- The deltas are fork-maintenance cost, not missing features:
  - **Ruff patch.** It touches `checkers/ast/mod.rs`, an algorithm-adjacent hot file that upstream
    edits often, plus `settings/mod.rs` and `narrowing_constraints.rs`.
  - **Pyrefly patch.** It now touches the solver: `alt/solve.rs` +135 lines, `answers.rs`,
    `overload.rs`. That is a larger rebase surface than the "visibility-only" invariant the skill
    and `show project` still state.
- Per upstream, `ty` 0.0.84 trails the crates. A ty CLI oracle must be built from the Ruff tag, as
  the skill notes.

## 3. Uncertainties and absences

- **Search coverage.**
  - Every `.rs` file under `crates/cpg-extract/src` and `crates/cpg-flow/src` was searched for
    `semantic_facts`, `Fact::`, Pyrefly member names (list in §2 Q3), visitor, line-index and
    comment idioms, `qualname`/`ModuleName` and docstring parsers.
  - Pinned sources: `ruff_linter/src/{lib.rs, semantic_facts.rs, checkers/ast/mod.rs,
    docstrings/, rules/pydoclint}`, `ruff_python_semantic/src/{model.rs, model/all.rs, scope.rs,
    analyze/visibility.rs}`, `ty_python_core/src/{lib.rs, use_def.rs, scope.rs}`, and Pyrefly
    `{sys_info.rs, docstring.rs, qname.rs, nesting_context.rs, report/pysa.rs, state/state.rs,
    state/lsp.rs, binding/{function,stmt}.rs}` plus the fork diff.
  - An empty grep does not prove absence of an equivalent written under another name.
- **Ruff `__all__` mutation handling** was checked only in `checkers/ast/mod.rs` and
  `model/all.rs`. Another site may handle `.extend`, so verify before relying on it.
- **Not probed:**
  - whether ty `reachable_bindings` matches `lexical.rs`'s candidate set on the rebinding fixtures;
  - how many Ruff-versus-Pyrefly builtin `Unresolved` disagreements real libraries produce;
  - whether `FlowDb` receives dependency modules for star imports.

  Each is a consequential precondition for any adoption decision. A probe would belong in this
  evidence folder.
- No upstream issue or PR offering a public `SemanticModel` driver was searched for.
