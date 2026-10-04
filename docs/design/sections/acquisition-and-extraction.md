# Acquisition and extraction

**Implemented / Tested, 2026-09-30, at the qualified facts frontier.** [§15](semantic-model.md) owns the current typed
facts contracts (ADR-0085/0086/0089/0092). Acquisition captures the locked installed closure and
selected pinned corpus before extraction. `lctx-model::domain` owns source, occurrence, attribution,
call, type, flow, document and deployment meaning; permanent lowerings publish PostgreSQL facts
generations. `catalog` omits the ty provider and reports Flow NotRequested; `behavioral` runs it.
Both profiles run documents and deployment. Publication never selects a generation.

Current producers are `crates/cpg-extract/src/acquisition.rs`, `typed_syntax.rs`, `lexical.rs`,
`symbol_records.rs`, `call_records.rs`, `type_records.rs`, `ty_flow.rs`, `document_parser.rs` and
`deployment.rs`; `pyrefly_stage.rs` currently composes native Pyrefly facts per input, and `cpg-flow`
provides transient native flow data over ty's separate parse. `cpg-core/src/facts.rs` owns stage
composition, admission and publication; `crates/lctx/src/compile.rs` drives the cumulative frontier CLI.
Focused receipts and qualification status are in the [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md).

**Current downstream design (Implemented, 2026-10-02).** Normalization, analysis, catalog and
serving follow the single-model PostgreSQL pipeline in §15. Their dated scoped receipts do not
establish current real-library qualification or activation; both remain stopped. Acquisition
requirements describe input verification, while typed row declarations own emission.

> Decision: ADR-0078

## §4 Pipeline


### §4.0 Library acquisition and the run contract

**Implemented** in `libraries/`, `cpg_extract::library` and `crates/lctx`, and **Tested**
(`crates/cpg-extract/tests/library.rs`, `crates/lctx/tests/acquire.rs`, `crates/lctx/src/propose.rs`,
2026-09-22), except where a line states a known gap.

**A library is data.** Every analyzed library, the pilot included, is a committed uv project
`libraries/<name>/`. This is the one production path for any Python library, and none of them
needs to be a dependency of this project.
- `pyproject.toml`: a virtual project (`[tool.uv] package = false`) with exactly one pinned
  requirement and an exact `requires-python`.
- `[tool.lctx] release`: the first-party distributions whose code is compiled. Everything else
  installed is dependency context, analyzed only as far as imports reach.
- `[tool.lctx.source]`: the upstream repository, tag and the full 40-hex `commit` the tag names
  (a tag can move), for docs, examples and tests. Stage A checks the tag names the locked version.
  `pyproject.toml` is read through typed serde structs: `[tool.lctx]` and `[tool.lctx.source]`
  refuse unknown keys (a misspelled `[tool.lctx.sourse]` fails, naming it) and mistyped values.
- **Corpus selection** (ADR-0018). `documents`, `documents_exclude`, `examples`,
  `examples_exclude`, `tests` and `tests_exclude` select the corpus by glob from the tree's root,
  in globset's syntax (`*` and `?` within one name, `**` across directories, `[…]`, `{a,b}`). A
  module's role is the key that selected it, and a file both `examples` and `tests` select is
  refused (ADR-0015). Each include glob must select a file. The tree is walked once (walkdir),
  dot-directories skipped, **no link followed**. Before selection, the corpus import-root check
  below refuses every directory or analyzer-readable symlink, even under an excluded path:
  selected code can import an unselected helper. Selection also refuses a selected non-Python
  document link; an excluded non-Python document link is harmless. **Tested**:
  `symlinks_in_corpus_imports_are_refused_even_when_unselected` checks selected links,
  excluded import directories, an excluded loop and an excluded non-Python document link.
  A source tree (`Release::from_tree`) refuses any link.
- **The fetched tree.** `lctx acquire` fetches the tree hermetically (every `GIT_*` variable
  removed, no system or global git configuration, no prompts; `git init`, a shallow fetch of the
  one commit, checkout, then `rev-parse HEAD` checked every time; no ambient git attributes,
  `core.autocrlf` off) into `build/sources/<name>/<commit>`; **Tested** by a stub `git`.
- **The corpus release.** Its id hashes its label (`repository@commit`), the library's
  `release_id` (its vocabulary and the files it reaches) and every selected file's path, content
  and role. It runs in the library's environment under its own context (the tree ahead of
  site-packages on its search path, each entry relative to its root).
- `.python-version`: the exact interpreter.
- `uv.lock`: the acquisition lock. Every distribution of the closure with the sha256 of each of
  its artifacts, reviewed and committed.

**Acquisition** is `lctx acquire <name>`: `uv sync --project libraries/<name> --frozen
--no-install-project --no-config --python <.python-version> --link-mode copy` into
`build/envs/<name>`.
- `--frozen` never re-resolves, and uv verifies every artifact hash.
- `--no-config` ignores user and system uv configuration, `--python` enforces the pin, and
  `--link-mode copy` keeps the environment's files from sharing inodes with the uv cache and other
  environments.
- Every other `UV_*` variable and `VIRTUAL_ENV` is removed from uv's environment.
- **Tested** by a stub `uv` that records its arguments and environment.
- `lctx acquire <name> --reinstall` rebuilds every package: the remedy when Stage A finds a changed
  file.
- The environment is gitignored and rebuildable; the lock is the record.
- `lctx library init` locks without `--no-config`: a library's own `[tool.uv]` applies there, and
  the lock is reviewed.

**Stage A** (`cpg_extract::acquisition::inventory`) reads the definition, the lock, `pyvenv.cfg` and
every `*.dist-info`, with no network and no interpreter. It works under one equivalence: **the
analyzer-readable bytes**, the files Pyrefly's module finder reads (`.py`, `.pyi`, `py.typed`;
never `.pth`).
- Every installed distribution must be the version the lock names, and the interpreter
  `.python-version`'s; otherwise the attempt stops.
- **Every** distribution's analyzer-readable `RECORD` entries must match their sha256, in the
  release and in its dependencies (the remedy: `lctx acquire --reinstall`).
- A release distribution the lock records without artifact hashes (a git or local source) is
  refused.
- The release's modules are exactly its distributions' `.py`/`.pyi` `RECORD` entries. Nothing
  walks a directory, and the root is site-packages.
- `release_id` hashes the release distributions' names, versions and verified content (§3.4.1). A
  re-listed artifact or a dependency-only upgrade leaves it unchanged.
- A source tree (a fixture or a local checkout) is the other input, `Release::from_tree`, whose
  `release_id` hashes its label: one label on two trees gives one id.

**Adding and upgrading** (`libraries/README.md`):
- `lctx library init <name> --requirement REQ` writes the definition, locks it and acquires it. It
  proposes `release` as the requested distribution plus every installed distribution that shares
  a repository root (`host/owner/repo` under a source label; sponsor and funding links never
  count). **Tested** by unit tests over METADATA; observed on FastMCP (the three first-party
  distributions) and `attrs`, 2026-09-22.
- Upgrading is: edit the pin, `uv lock --project libraries/<name> --upgrade-package <dist>`,
  review the lock diff, `lctx compile <name>`. For FastMCP the skill moves with it
  ([§1.4](../DESIGN.md#section-1-4)).

**Context.** The context records:
- the Python version (from `pyvenv.cfg`) and platform;
- the **ordered** search paths;
- the **environment digest**: the installed distributions' dist-info names (which carry their
  versions) and every analyzer-readable file's site-relative path and content digest. `RECORD`
  lines outside site-packages (console scripts carry the environment's absolute path) and files
  Pyrefly never reads stay out, so a moved environment keeps its identity and every
  analyzer-visible change in site-packages, owned or unowned, moves it;
- the **lock digest**;
- the digests of every configuration file an analyzer receives.

**Explicit inputs only.** Extractors receive their environment and configuration as arguments.
- **Pyrefly reads only the environment's `site-packages`** (and, for the corpus run, the fetched
  tree). It never runs the interpreter.
- **Pyrefly's configuration is a constructed value, not a discovered file** (§4.2.1): explicit
  search path and site-package path; explicit Python version and platform; heuristics, walk-up
  fallback and the interpreter query all disabled.
- **Recorded.** `contexts` stores the digest of the configured `ConfigFile` (keys sorted:
  DataFusion turns on serde_json's `preserve_order`, and no digest may depend on the build graph),
  the search and site-package paths **relative to their roots** (release, environment), the sys
  info from its fields (not `Debug`), the environment digest and the lock digest. Where a
  checkout, tempdir or environment sits never changes an identity; what the dependencies contain
  always does. **Tested**: two fixture locations, two acquired environment paths with
  location-dependent `RECORD`s, two environments, an unowned stub inside a package, a loose file,
  a lock-only change; at pilot scale (2026-09-23, observed, not a repo test), a freshly synced
  environment in another directory plus a copied source tree give byte-identical `contexts`,
  `nodes` and `edges`. A changed analyzer-readable byte a `RECORD` owns is refused.
- `releases` and `distributions` record the library, requirement, lock digest, release
  distributions, installer and every installed distribution (§3.2).

For the library run nothing ambient can change an answer without changing `context_id` (G4). That
covers `PATH`, `VIRTUAL_ENV`, `PYTHONPATH`, `CONDA_PREFIX`, the working directory and an upward
`pyproject.toml`. **Tested** (2026-09-22): no process was spawned, and output was byte-identical
with each of them perturbed.

**Corpus input identity** ([plan W8](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition),
RFU/F07; **Tested** in focused helper edit/add, relocation and clean-recomputation fixtures,
2026-09-26). The corpus
run puts the fetched tree ahead of site-packages, so Pyrefly can import an unselected helper.
The corpus `release_id` now hashes content and membership of every analyzer-readable file under
that root, whether selected or not, and refuses analyzer-readable symlinks. Ordinary
site-packages content and membership were already hashed. Editing an unselected imported helper
between extractions now yields the same fact tables as extracting the edited tree in a fresh
location. Adding an unselected analyzer-readable helper after a warm extraction also matches a
clean extraction of the augmented tree. Both changes move corpus release identity while
relocation preserves it. The compiler102 fresh pilot rebuilt and published successfully on 2026-09-27 (plan §1.1);
the helper-mutation controls, rather than that ordinary pilot alone, establish this input-identity boundary.

**Run.** A run is one producer applied to one context for a declared set of families under one
analysis configuration. `runs` records that. `producers` records the tool, the revision (for the
extractor: the fork revision and patch digest, which `just deps` checks against `Cargo.lock` and
the patch file, the ruff line, and the flow provider with its runtime-view version) and the
adapter build digest: captured production source membership and bytes, manifests, pinned
toolchain, specifications and third-party sources. The shared runtime-script declaration supplies
both embedded runner bytes and their fingerprint paths; unrelated administration scripts are not
producer inputs (ADR-0115; **Implemented**, 2026-10-02). The declaration and fingerprint helper
are captured too. This deliberately retains conservative semantic source roots. The committed
model catalog digest also participates because context extraction retains model-named exception
classes (§3.2). A catalog or runtime-script edit therefore cannot reuse a prior producer or run
identity.

**Inspecting a generation (Implemented, 2026-10-02).** `lctx generation show <id>` reports
its lifecycle and receipts; `lctx query --generation <id> "SQL"` reads an admitted published
generation under the original read-only lease. `lctx compile` reports its unselected generation.
Failed/interrupted staging is inspected through generation metadata and compilation receipts,
not a published-reader bypass. [Storage §6](storage-and-publication.md) owns admission and reset.

> Decision: ADR-0117, ADR-0086, ADR-0015, ADR-0018, ADR-0045, ADR-0115


### §4.1 Stages

**Implemented, 2026-10-02.** The hard cumulative pipeline is executed by
`cpg-core::compilation`; dated qualification remains with the cutover/phase plans.

| Stage | Owner | Output |
|---|---|---|
| Acquisition | uv and `cpg-extract::library` | Captured pinned release, corpus and attributed input context |
| Facts | Pyrefly/Ruff in-process and explicitly requested `cpg-flow` | Typed `lctx-model` facts and provider coverage, stored in one PostgreSQL generation |
| Normalized | `cpg-core::normalize` adapting model operations | Entities, ownership, places, effective signatures and checked binding |
| Analysis | Finite upper-stage bindings and model-owned operations | Local/Model/Summary, structural and optional analytic results; canonical catalog/evidence/selection |
| Catalog | Model-owned synthesis and retrieval with compute/effect adapters | Attributed assertions, briefs and addressable evidence units |
| Publication | `lctx-postgres` generation lifecycle | Shared validation, immutable receipts and one final unselected published generation |
| Serving | Model contracts, original generation guard and Python MCP adapter | Canonical hydrated packets and bounded admitted wire envelopes |

Unknown targets and incomplete provider coverage remain explicit. No identity mapping drops
unresolved rows; no compatibility store is retained. Facts/Normalized/Analysis checkpoints
remain explicit and compilation never selects a generation. [§15](semantic-model.md) owns the
layer/declaration contract; [§11](synthesis-and-serving.md) owns serving.

> Decision: ADR-0117, ADR-0086, ADR-0073


### §4.2 Extraction

**Implemented, 2026-10-03.** The extraction stage declares finite family/provider
grants. Canonical Ruff syntax and Pyrefly typing retain their own provider/run coverage
within the joint captured-input session; another provider’s matching family is not admitted.
[§15.11](semantic-model.md#section-15-11) owns stage admission.

> Decision: ADR-0117, ADR-0119

**Accepted target, 2026-10-03:** latest independent Ruff owns canonical syntax/contextual
semantics, Pyrefly supplies native typing/callable/member/public evidence, and ty supplies requested
flow/index/timing observations. Narrow observational forks preserve upstream behavior. M0–M3
in the [code-facts coordinator](../../plans/code-facts-expansion-plan_2026-10-03.md) own production
adoption and acceptance. The pinned families and canonical parser are now implemented; the coordinator distinguishes
scoped producer/consumer receipts from remaining enrichment and assembled acceptance. Earlier
pre-migration receipts retain their original boundary.

**Implemented** in `cpg-extract` and `cpg-flow`, and **Tested** by `crates/cpg-extract/tests` and
`crates/cpg-flow/tests` (2026-09-22 onward), where each test names the claim it checks: the
variant table (every site kind, `is_attribute`, potential remainders), module and class keys,
`__all__` forms, `_invalid/` modules, BOM/CRLF offsets against the stored bytes, two install
locations, two dependency environments, module order and cross-process determinism, the id
recipes, the panic abort, the ambient refusal and harness equivalence with the CLI. The
`catch_unwind` and per-module-thread ban is an ast-grep rule. Pyrefly is linked from the pinned
fork (§B8; revision in `docs/pins.md`). A run is one call of the driver over one context, and
everything below happens in one process.

| Provider surface (`model_id` suffix) | Mode | Raw tables |
|---|---|---|
| The acquired bytes (`source`) | `native_traversal` | `source_files` rows and their text (ADR-0015) |
| Latest independent Ruff canonical walk (`ruff-ast`) | `native_traversal` | `declarations` (with docstrings), `export_syntax`, `parameter_syntax`, `call_syntax`, `arguments`, `syntax_nodes` |
| Pyrefly's Pysa collectors, in memory (`pyrefly-pysa`) | `native_traversal` | `pysa_functions`, `parameter_semantics`, `pysa_calls`, `class_ancestry`, `pysa_classes`, `context_definitions` |
| Pyrefly's public-name helpers (`pyrefly-public`) | `native_traversal` | `public_names`. **This defines "public"** |
| Pyrefly's native types (`pyrefly-types`) | `native_traversal` | `type_terms`, `type_term_args`, `type_observations`, `record_fields` |
| Pyrefly's docstring parser (`pyrefly-docstring`) | `native_traversal` | `parameter_docs` |
| Our local-binding recognizer over the Ruff AST (`lctx-lexical`) | `recognizer` | `scopes`, `bindings`, `references`, `reference_resolutions` |
| Surface comparison (`compare`) | `relational_derivation` | `boundaries` rows that compare two surfaces |
| markdown-rs over the corpus documents (`markdown-rs`) and our mention recognizer (`lctx-docs`) | `native_traversal`, `recognizer` | the `docs` family (§3.2) |
| ty's semantic index through `cpg-flow` (`ty-flow`; `python-analyzers` skill) | `native_traversal` | the `flow` family (§3.2) |

**Implemented flow provider** (ADR-0117, ADR-0045), 2026-10-03. `cpg-flow` links ty/Ruff
0.0.16 from the independent Ruff fork and salsa 0.28.5 (ADR-0118). It parses captured text
with every `TYPE_CHECKING` name token renamed to a same-length sentinel. Native source-view
observations retain both original and view digests/lengths; matching byte ranges do not assert
identical source bytes. Latest Ruff's original-byte parse owns canonical syntax. Flow joins
through model-owned range/kind/context correspondence; provider-local IDs remain transient.
Python version and platform in ty's `ProgramSettings` now come from the analysis context.
Reachability and narrowing use distinct formulas; precision loss remains explicit Partial
coverage. Scoped narrowing/source-view/protocol/capture-timing controls are **Tested**,
2026-10-03, in coordinator §7. Capture timing characterizes native index snapshots; the
bounded stable-capture interpretation has its own caller-frame/value certificate and scoped
native/PG/replay receipts. Capture packet acceptance remains pending in that coordinator.

**Implemented selected source characterization**, 2026-10-03. The independent Ruff parse
also supplies configured F821/F401/F841 observations, including retained `noqa` suppression.
Pyrefly supplies native diagnostic category/severity/header/details and selected parameter
definition answers. Its duplicate fixture lookup follows the actual last-retained definition;
that is a native answer, not a runtime fixture-registration claim. Missing or foreign locations
remain unavailable. Catalog original-evidence packets keep the observations inside the same
source grant; a diagnostic-free source or fixture lookup never certifies execution success.
The implemented usage cascade joins exact native call sites to normalized events and original
source. It preserves arguments, receivers, binding roles, API association or unresolved targets,
and separate chosen/candidate traces. Higher-order targets remain potential callback
characterization, without invocation/effect authority. Source grants do not expand to reveal
foreign or outside locations. Scoped native and actual disposable-PG diagnostic/usage
original-evidence controls are **Tested**, 2026-10-03, as a composite receipt in the code-facts
coordinator §7; current-tree assembled qualification remains open. The typed
model owns payloads and attachments.



### §4.2.1 Driver

1. **Refuse ambient knobs.** The driver exits before any analysis if `PYREFLY_STACK_SIZE`,
   `PYREFLY_FIXPOINT_DETAILS` or any `PYSA_DUMP*` is set; clearing them would need `unsafe`
   (edition 2024), which the workspace forbids. Every path passed in is absolute, because Pyrefly
   reads the working directory to absolutize relative paths.
2. **Configuration.** Build a `ConfigFile` with:
   - `search_path_from_args`;
   - `disable_search_path_heuristics: true`, `disable_project_excludes_heuristics: true` and
     `enable_fallback_search_path: false`;
   - `python_environment.{python_version, python_platform, site_package_path: Some(..)}`;
   - `interpreters.skip_interpreter_query = true`;
   - no build system.

   `configure()` must return no errors. `ConfigFinder::new_constant` rules out discovery and
   walk-up.
3. **State.** `State::new(finder, ThreadCount::Inline)` on a driver-owned thread whose stack size
   is part of the producer config, so changing it changes `run_id`. The check and the lazy solves
   all run on that thread. One thread is a precaution (upstream's cycle placeholders are per
   thread), not a demonstrated need: FastMCP and an import-cycle fixture (a return-type cycle and
   a global cycle whose solved types reach `pysa_calls`) gave identical output at `Inline` and
   several thread counts in each module order (2026-09-22), and `Inline` was also the cheaper
   setting. **Tested:** sorted and reversed module order and separate processes give identical
   output.
4. **Handles.** One per project module, from `cfg.handle_from_module_path`, sorted by module name.
5. **Run.** Install a `PysaReporter` with `write_files: false` and `ModuleIds::new(&handles)`, then
   call `transaction.run(&handles, Require::Everything, None)`. Keep the reporter installed during
   extraction (borrow it with `pysa_reporter()`), because dependency modules solve lazily.
6. **Extract** per module, in sorted order (§4.2.2–§4.2.3). Then emit coverage (§3.7).


### §4.2.2 Canonical syntax and provider correspondence

**Accepted target, 2026-10-03:** parse the captured snapshot with latest Ruff once, then reuse
that parsed module for the canonical SourceOrderVisitor and populated Checker observer. The
observer emits owned node/context/branch and final binding/reference/export rows after deferred
passes. Work/row/cancellation/sink limits become explicit Partial coverage; initial parsing is
an indivisible upstream call. String-annotation nodes retain their distinct origin.

Pyrefly's retained embedded AST supplies native adapter queries. Owned correspondence checks
snapshot/view, range, model node kind, role, context and uniqueness. Containers, formal declarations
and native/synthetic slots have distinct identities. Varargs may be their own formal; missing
or ambiguous attachment never uses a parent/name guess. Source traversal assigns structural
paths independently of provider-local node indexes. Source defaults and typed defaults remain
distinct observations. The ty runtime view is separately labelled even when byte ranges match.

**Implemented, 2026-10-03:** `CanonicalSyntax` now parses with latest Ruff and reuses the
borrowed parsed module for structural extraction and contextual observation. Unreadable bytes
remain unavailable and recovered syntax remains Partial; no recovered empty module certifies
absence. Owned contextual bindings/definitions and normalized reference/declaration characterization
are implemented with scoped M2 receipts. Missing or unlocated native answers remain explicit,
including absent flags on FinalUnresolved; they do not substitute for exact source binding.
Independent parser-unavailability remains explicit. Cancellation is bounded by upstream pass
draining, not a hard interruption. Producer→first-operation→output fixture controls qualify the
new source contract; a fork compile alone does not.

> Decision: ADR-0117

### §4.2.3 Semantics: Pyrefly's own collectors

- **Collectors.** Per module: `PysaResolver::new`, `ModuleAnswersContext::create`,
  `collect_captured_variables_for_module`, `create_reversed_override_graph_for_module`, then
  `export_module_definitions` and `export_module_call_graphs`, the functions behind
  `--report-pysa`. The in-memory structs equal the CLI's JSON (**Tested**, 2026-09-22: 257 of 257
  modules; definitions equal as sets).
- **Locations → bytes.** Every `PysaLocation` is converted with its module's `LineIndex` (§3.4).
- **Join key.** `pysa_calls` joins `call_syntax` on the **full call-expression range**. Every
  call it leaves unmatched in the 2026-09-22 measurement was inside an annotation, which Pysa's
  call model does not cover.
- **Mapping** (§3.6). Every Pysa variant has one row. The mapper is exhaustive, so a variant
  missing here fails the build:

  | Pysa variant | Row | Phase | Modality | Origin |
  |---|---|---|---|---|
  | `call_targets` with `Target::Function` | call target | `call` | `definite` if it is the only target and nothing is unresolved, else `candidate` | analyzer_assertion |
  | `Target::Overrides(f)` (any list) | candidate target to `f`; the resolution has `candidate_set_complete_under_model = false` | the list's phase | `candidate` | analyzer_assertion |
  | `init_targets`, `new_targets` | call targets | `init`, `new` | as `call_targets` | analyzer_assertion |
  | `higher_order_parameters[i]` | target attached to argument `i` | `call` | `potential` | analyzer_assertion |
  | `if_called` (identifier or attribute) | target on a `Reference`, and its unresolved remainder | `call` | `potential` | analyzer_assertion |
  | `property_getters`, `property_setters` | call targets | `property_get`, `property_set` | as `call_targets`, but at most `candidate` when `is_attribute` | analyzer_assertion |
  | `AttributeAccessCallees.is_attribute` (some flow reads a plain attribute) | a column on the attribute access's rows | — | — | — |
  | `ArtificialCall`, `ArtificialAttributeAccess`, format-string callees | as the callee kind above, keeping the `OriginKind` | as above | as above | synthetic_model |
  | `Unresolved::True(reason)` | an `unresolved` row with the reason: `has_unresolved_remainder` on the resolution | `call` | `definite`, or `potential` under `if_called` and higher-order lists | as the site |
  | receiver fields (`implicit_receiver`, `receiver_class`, `implicit_dunder_call`, class and static method flags) | columns on the target row | — | — | — |
  | `Target::FormatString`, `Return` shims, `global_targets`, `captured_variables`, `return_type` | **not carried**: synthetic or no consumer | — | — | — |
  | `Define` | **not carried**: it links a nested `def` to the function it creates, which `declarations` already records | — | — | — |

  Pysa numbers higher-order arguments with the same `iter_source_order().enumerate()` as
  `arguments.ordinal` (Interface-checked against the pinned fork, 2026-09-23), so keyword and
  starred arguments before them do not shift the index.
- **Unmatched calls.** The walker marks calls inside annotations (`visit_annotation`). An
  unmatched call there is a `boundaries` row with `outside_provider_model`. Any other unmatched
  call is a `missing_evidence` boundary, and that module's `calls` coverage is `partial`.
- **Module and class keys.** Every Pysa row carries its file's `module_node_id` (a `.py` and its
  `.pyi` share a module name). A reference resolves through Pysa's `ModuleId` (never stored) to a
  *module ref*: `@<release-relative path>` for a release file, else the module name (one file per
  name in a context). Classes are `<module ref>:<Name>#<ClassId>`, functions
  `<module ref>::<key>`; Stage C takes spans from the `pysa_functions` and `class_ancestry` name
  spans. **Tested** (`pysa_keys`: a `.py`/`.pyi` pair, two nested `Config` classes).
- **Set-valued lists** (`captured_variables`, a union's `class_names`) come out of hash sets in
  varying order. Every record set is sorted by its declared key.
- **Public names.** Per public module (`is_public_module`): `explicit_dunder_all_names` if
  present, else local definitions plus explicit re-exports, each traced by `trace_export_origin`
  to a row (access path, origin, `via_dunder_all`). The flattened set must equal
  `compute_public_fqns` (behind `coverage report --public-only`) or the run fails. Pyrefly stays
  the only definition of "public".
  - **A completeness detector replaces Ruff's corroboration.** Pyrefly reads a non-literal
    `__all__` (`sub.__all__ + [...]`, a call) as absent or in part, a blind spot every check
    sharing its code shares. So `exports` is `partial`, with an `outside_provider_model`
    boundary, when `unresolvable_dunder_all_range()` is set or the Ruff walk finds an `__all__`
    that is not a literal list or tuple of strings.
- **Fidelity.** Pysa-model facts are `report_projection`, in memory or not: Pysa's types are a
  display string, scalar properties and class names, and `parameter_semantics` keeps all three
  (a row keeping only the string would be `display_only`). Native `pyrefly_types::Type`
  (`native_structural`) is read by the `types` surface.
- **Resolved function status.** Pyrefly's abstract-method flag and body kind are persisted in
  `function_implementations` and drive downstream negative-premise admission. Focused aliased
  abstract and Protocol fixtures passed; this is a reviewed schema migration
  ([plan W14](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition),
  RF/F10).


### §4.2.4 Binding rule (conservative)

- **The rule.** Any binding of a parameter's name that appears lexically before a guard or
  forwarding site in the same scope marks that site `ambiguous_binding` (a boundary).
- **As implemented** (**Tested**), it is stricter than "before": a parameter whose name has any
  second binding event in its scope (after the site, or in a statically pruned branch) is never
  followed, and a guard on it is no guard. The trace is an analysis finding, not a `boundaries`
  row (extraction's table): a read of the name at a call into the subsystem is an
  `unfollowed_argument` with reason `rebound`, stated in the brief's Limits (§9.2).
- **The motivating case** is pyarrow's `write_dataset`. Its `schema` guard only applies to
  caller-supplied scanners, because earlier branches rebind `schema`.
- **Pyrefly's flow-sensitive `Bindings` are not used for this rule.** Its binding pass drops
  branches it decides statically (`TYPE_CHECKING`, `sys.version_info`), so a rebinding there is
  invisible.

**Accepted target, 2026-10-03:** native populated Ruff contextual observations supplement this
source-history recognizer; lexical resolution/screening does not itself establish runtime execution.
M2 owns adoption; a bespoke port of Ruff's semantic model remains deferred.


### §4.2.5 Failure, determinism and the parity oracle

- **Panics abort the attempt.** Any panic in code that touches Pyrefly (`run`, the collectors, the
  public-name helpers, lazy solves during extraction) or ty aborts it, and nothing is published
  (§6.1). There is no `catch_unwind`: Pyrefly treats its state as unsupported after any panic (a
  poisoned lock, unpublished cycle answers), so continuing with the next module is unsafe.
  - Load and parse errors are not panics; they still become coverage rows (§4.2.2). Per-module
    isolation would need ADR-0117's rejected sidecar-process alternative.
- **Determinism oracle.** Reruns, reversed module order, separate processes and perturbed ambient
  variables give byte-identical sorted tables (**Tested**, 2026-09-22).
- **Harness-equivalence oracle.** A test runs the pinned Pyrefly CLI (the `uv` dev group, same
  revision) with an equivalent generated `pyrefly.toml` and asserts, per project module, that the
  in-process Pysa structs equal its `--report-pysa-format json` output as sets (`module_id`
  removed), and that the public set explains its `--public-only` report (an exact match or a
  public parent prefix).

  It shares the collectors with the CLI, so it checks our driver (configuration, reporter
  lifecycle, lazy solving), not the correctness of Pysa. It is **Tested**
  (`crates/cpg-extract/tests/harness.rs` on two fixtures, 2026-09-22). The CLI is never a
  production input.


### §4.2.6 Upgrading the analyzer families

1. Read exact upstream/tag manifests and the pinned python-analyzers source contracts. Independently
   choose latest Ruff/ty and Pyrefly's embedded adapter line; one never imposes a ceiling on the other.
2. Rebase one aggregate fork patch per provider, preserving ordinary upstream semantic behavior.
   Inspect changed native queries/callback phase and record patch size, immutable revision/digest,
   environment reads and paired parity. New inference/lint algorithms require another decision.
3. Move exact manifests/lock and CLI pins using pin-check. Check nominal Cargo source as well as
   version, scoped family reachability and feature isolation; remove unused older family consumers.
4. Migrate owned adapters and append model codes as needed. Source/role attachment failures remain
   explicit, never casts, string parsing or silent old-pin fallback.
5. Run focused native/current adapter parity and deterministic fixtures, then the scope-end full
   functional and hygiene gates. Record dated pins/limits in docs/pins and the coordinator.

> Decision: ADR-0117, ADR-0118


**Implemented, 2026-10-03:** native ClassTrait provenance comes from the exact pinned
Bindings class index and solved ClassMetadata. It is added only when the full native payload
uniquely matches the existing assertion; report support remains independently attributed.
Missing, ambiguous or disagreeing correspondence remains Partial with a boundary. Six native
class forms and payload disagreement refusal are scoped **Tested** in the code-facts
coordinator §7; final assembled qualification remains open.

### §4.3 Fact construction and persistence

**Implemented, 2026-10-02.** `lctx-model::domain` owns typed records, schema/codebooks,
canonical IDs and shared invariants. Provider adapters emit those records through bounded Arrow
batches. `lctx-postgres` lowers declarations into COPY, constraints, receipts and generation
publication; `cpg-core` registers completed sources for DataFusion compute. Canonical readers
verify the actual content and declared publication prefix. Shared validation runs before
publication; a failed attempt publishes no reader authority. [§6](storage-and-publication.md)
and [§15.11](semantic-model.md#section-15-11) own the lifecycle and exact consumption contract.

**Extending a fact family (Implemented route, 2026-10-02).** Add the typed attributed relation
and append-only codes to its model owner, declare support and provider coverage separately,
then wire the provider and the frontier's relation closure. Normalization consumes its declared
facts and emits the corresponding nominal results; shared invariants name their complete ordered
inputs. Add an independent observed/unknown or missing-evidence twin that challenges the new
meaning. Do not copy schema fields, infer absence from unavailable coverage, or amend gold to
agree with extraction. Fields remain executable declarations rather than a second documentation
registry.

**Operational observations (Implemented, 2026-10-02).** Stage measurement reports elapsed time,
sampled RSS and admitted reservation peaks outside semantic content. A separately spawned sampler
continues across borrowed CPU regions on the multithread runtime; current-thread/no-runtime CPU
placement is explicitly inline. Executing opaque kernels retain access and charges until they
drain. Earlier retired-pipeline measurements do not establish current throughput or total RSS;
preserved benchmark captures keep their original boundaries.

> Decision: ADR-0086, ADR-0016, ADR-0116
