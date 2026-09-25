# Acquisition and extraction

<!-- owner-intro -->

## §4 Pipeline


### §4.0 Library acquisition and the run contract

**Implemented** in `libraries/`, `cpg_extract::library` and `crates/lctx`, and **Tested**
(`crates/cpg-extract/tests/library.rs`, `crates/lctx/tests/acquire.rs`, `crates/lctx/src/propose.rs`,
`just pilot`, 2026-09-22), after a standard review corrected the first identity recipes and the
acquisition flags. Source: IP L1146–L1170 (Stage A), L536 (run supporting tables).

**A library is data.** Every analyzed library, the pilot included, is a committed uv project
`libraries/<name>/`. This is the one production path for any Python library, and none of them
needs to be a dependency of this project.
- `pyproject.toml`: a virtual project (`[tool.uv] package = false`) with exactly one pinned
  requirement and an exact `requires-python`.
- `[tool.lctx] release`: the first-party distributions whose code is compiled. Everything else
  installed is dependency context, analyzed only as far as imports reach.
- `[tool.lctx.source]`: the upstream repository, tag and the full 40-hex `commit` the tag names
  (a tag can move), for docs, examples and tests. Stage A checks the tag names the locked
  version. `documents`, `documents_exclude`, `examples` and `tests` select the corpus by glob
  from the tree's root, in globset's syntax (`*` and `?` within one name, `**` across directories,
  `[…]`, `{a,b}`; H1 C2); a module's role is the key that selected it, and a file both `examples`
  and `tests` select is refused (ADR-0015). The tree is walked once (walkdir), dot-directories
  skipped, **no link followed**: a symlink a glob selects is refused, naming it, and so is a
  directory link that could hold a selection (it and a glob's literal prefix lie one under the
  other) unless an exclude covers it. An exclude covers a link only when it names the link, or
  matches any name under it (tested with two unrelated probe names); `documents`, `examples` and
  `tests` each have an `_exclude` list (ADR-0018; H1 review F5; **Tested**:
  `symlinks_in_a_tree_are_refused_unless_excluded`, with a partial exclude that does not cover and
  a loop under an excluded path that is harmless). A source tree (`Release::from_tree`) refuses
  any link. `lctx acquire` fetches the tree
  hermetically (every `GIT_*` variable removed, no system or global git configuration, no
  prompts; `git init`, a shallow fetch of the one commit, checkout, then `rev-parse HEAD` checked
  every time; no ambient git attributes, `core.autocrlf` off) into
  `build/sources/<name>/<commit>`; **Tested** by a stub `git`. `pyproject.toml` is read through typed serde structs:
  `[tool.lctx]` and `[tool.lctx.source]` refuse unknown keys (a misspelled `[tool.lctx.sourse]`
  fails, naming it; H1 C3) and mistyped values, and each include glob must select a file. The corpus release's id hashes its label
  (`repository@commit`), the library's `release_id` (its vocabulary and the files it reaches) and
  every selected file's path, content and role. It runs in the library's environment under its own
  context (the tree ahead of site-packages on its search path, each entry relative to its root).
- `.python-version`: the exact interpreter.
- `uv.lock`: the acquisition lock. Every distribution of the closure with the sha256 of each of
  its artifacts, reviewed and committed.

**Acquisition** is `lctx acquire <name>`: `uv sync --project libraries/<name> --frozen
--no-install-project --no-config --python <.python-version> --link-mode copy` into
`build/envs/<name>`.
- `--frozen` never re-resolves, and uv verifies every artifact hash.
- `--no-config` ignores user and system uv configuration, `--python` enforces the pin, and
  `--link-mode copy` keeps the environment's files from sharing inodes with the uv cache and other
  environments (each file's link count is 1 on the pilot).
- Every other `UV_*` variable and `VIRTUAL_ENV` is removed from uv's environment.
- **Tested** by a stub `uv` that records its arguments and environment.
- `lctx acquire <name> --reinstall` rebuilds every package: the remedy when Stage A finds a changed
  file.
- The environment is gitignored and rebuildable; the lock is the record.
- `lctx library init` locks without `--no-config`: a library's own `[tool.uv]` applies there, and
  the lock is reviewed.

**Stage A** (`cpg_extract::library::acquired`) reads the definition, the lock, `pyvenv.cfg` and
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
  count). **Tested** by unit tests over METADATA. Observed, not a repo test (2026-09-22): for
  FastMCP it proposed exactly the three distributions, and `attrs` 25.3.0 went from nothing to a
  published snapshot in 0.8 s.
- Upgrading is: edit the pin, `uv lock --project libraries/<name> --upgrade-package <dist>`,
  review the lock diff, `lctx compile <name>`. For FastMCP the skill moves with it (§1.4).

**Context.** The context records:
- the Python version (from `pyvenv.cfg`) and platform;
- the **ordered** search paths;
- the **environment digest**: the installed distributions' dist-info names (which carry their
  versions) and every analyzer-readable file's site-relative path and content digest. `RECORD`
  lines outside site-packages (console scripts carry the environment's absolute path) and files
  Pyrefly never reads stay out, so a moved environment keeps its identity and every
  analyzer-visible change moves it. It costs well under a tenth of a second on the pilot;
- the **lock digest**;
- the digests of every configuration file an analyzer receives.

**Explicit inputs only.** Extractors receive their environment and configuration as arguments.
- **Pyrefly reads only the environment's `site-packages`.** It never runs the interpreter.
- **Pyrefly's configuration is a constructed value, not a discovered file** (§4.2.1):
  - explicit search path and site-package path;
  - explicit Python version and platform;
  - heuristics, walk-up fallback and the interpreter query all disabled.
- **Recorded.** `contexts` stores the digest of the configured `ConfigFile` (keys sorted:
  DataFusion turns on serde_json's `preserve_order`, and no digest may depend on the build
  graph), the search and site-package paths **relative to their roots** (release, environment),
  the sys info from its fields (not `Debug`), the environment digest and the lock digest. Where a
  checkout, tempdir or environment sits never changes an identity; what the dependencies contain
  always does (**Tested**: two fixture locations, two acquired environment paths with
  location-dependent `RECORD`s, two environments, an unowned stub inside a package, a loose file,
  a lock-only change; on the pilot, two environment paths give one `content_digest`; and at
  pilot scale for both runs, a freshly synced environment in another directory plus a copied
  source tree give byte-identical `contexts`, `nodes` and `edges`, a C6 review observation
  (R2, 2026-09-23), not a repo test). A changed
  analyzer-readable byte a `RECORD` owns is refused.
- `releases` and `distributions` record the library, requirement, lock digest, release
  distributions, installer and every installed distribution (§3.2).

So nothing ambient can change an answer without changing `context_id` (G4). That covers `PATH`,
`VIRTUAL_ENV`, `PYTHONPATH`, `CONDA_PREFIX`, the working directory and an upward
`pyproject.toml`. **Tested** (spike S2, 2026-09-22): no process was spawned, and output was
byte-identical with each of them perturbed.

**Run.** A run is one producer applied to one context for a declared set of families under one
analysis configuration. `runs` records that. `producers` records the tool, the revision (for the
extractor: the fork revision and patch digest, which `just deps` checks against `Cargo.lock` and
the patch file, and the ruff line) and the adapter build digest (an output version bumped by hand
when mapping output changes, which the variant and id snapshots show).

**Measured** (`just pilot`, release build, warm environment and uv cache so acquisition is a
no-op, this Linux host, 2026-09-22): FastMCP 4.0.5, 275 modules, 103 distributions; acquire +
extract 7.1 s, the whole compile 7.9 s wall time, 1.61 GB peak RSS. Two runs, and two environment
paths, give the same `release_id` and `content_digest`.
- **After C1 (ADR-0014)** the compile takes 13.6 s at 2.53 GB. `lctx compile` now prints each
  stage (§4.3): acquire 0.01 s, Stage A 0.04 s, the release check 2.1 s, per-module extraction 4.5 s
  (the Pysa collectors 4.4 s, the Ruff walk 0.08 s), the dependency check 3.7 s, dependency
  definitions 0.7 s, the Delta stages 1.6 s.
- A published snapshot is inspected with `lctx query --store DIR --snapshot HEX "SQL"`: read-only,
  every table at its recorded version, filtered to the snapshot (§6.2). `lctx compile` prints
  the attempt's id first, and `--unpublished` reads an attempt validation rejected at the commits
  carrying its own `lctx.snapshot_id`, found in each table's kept log whatever was written after
  it; a table it did not write is left out, so a query naming it fails (`attempt_versions`;
  ADR-0017, H1 review F2; `a_rejected_attempt_is_inspected_at_its_own_commits`). For inspecting a
  failure, never for a reader.

**Implemented and Tested in focused cases (ADR-0029, 2026-09-25):** the extractor producer build
digest includes the committed model catalog digest, because context extraction now retains model-
named exception classes. A catalog edit therefore cannot reuse the prior producer/run identity;
`EXTRACTOR_OUTPUT_VERSION` is 29 for this migration.

> Decision: ADR-0013 (superseding ADR-0007), ADR-0012, ADR-0015, ADR-0017, ADR-0018, ADR-0029


### §4.1 Stages

> Decision: ADR-0013

| Stage | Owner | Output |
|---|---|---|
| A. Source and analysis universe | uv (`lctx acquire`) + `cpg_extract::library` (§4.0) | the verified release files and context; `releases`, `distributions`, `source_files` |
| B. Typed provider facts | Pyrefly and Ruff in-process (§4.2) + Arrow builders (§4.3) | raw family batches, written to Delta |
| C. Provider-local identity | A DataFusion name-span join (`cpg_schema::derived`) | `provider_node_map`, its keys checked unique and injective before publication (after use by D, which is safe because nothing publishes on failure) |
| D. Semantic relationships | DataFusion over the written raw tables, written back through Delta (§4.3) | derived family tables and views |
| E. Projections and analytics | DataFusion (projection SQL on the attempt's session) → `lctx-analytics` (petgraph / leiden-rs / FCA; Arrow in, Arrow out) → the attempt's write path (§5, §9) | `findings` family, provenance in-row (ADR-0019), with lineage |
| F. Synthesis | `cpg-core::synth`: evidence, kind policy and status propagation in DataFusion; Rust templates + extractive selection (§10); brief documents embedded through the cache | assertions, briefs |
| G. Publication | Rust (§6) | `snapshots` row; serving bundle |

Unmapped rows stay in the derived table with a null node and, where one applies, a reason
column. No inner join drops them.

> Decision: ADR-0012, ADR-0019


### §4.2 Extraction

> Decision: ADR-0012

**Labels.** A line that cites a spike result (S1–S7, `spike/pyrefly-inproc`, FastMCP 4.0.3,
2026-09-22) is **Tested** or **Measured**. The rest is **Implemented** in `cpg-extract` and
**Tested** by `crates/cpg-extract/tests` (slice 1, 2026-09-22), where each test names the claim
it checks: the variant table (every site kind, `is_attribute`, potential remainders), module and
class keys, `__all__` forms, `_invalid/` modules, BOM/CRLF offsets against the stored bytes, two
install locations, two dependency environments, module order and cross-process determinism, the
id recipes, the panic abort, the ambient refusal and harness equivalence with the CLI. The
`catch_unwind` and per-module-thread ban is an ast-grep rule. Pyrefly is linked from the pinned fork (§B8). A run is one call of the driver over one context, and everything below happens in
one process.

| Provider surface (`model_id` suffix) | Mode | Raw tables (v1) |
|---|---|---|
| Ruff `=0.0.11` walk over Pyrefly's parse (`ruff-ast`) | `native_traversal` | `declarations` (with docstrings), `export_syntax`, `parameter_syntax`, `call_syntax`, `arguments`; `syntax_nodes` (C2) |
| Pyrefly's Pysa collectors, in memory (`pyrefly-pysa`) | `native_traversal` | `parameter_semantics`, `pysa_calls`, class ancestry; `type_observations` (increment 3) |
| Pyrefly's public-name helpers (`pyrefly-public`) | `native_traversal` | `public_names`. **This defines "public"** |
| Our local-binding recognizer over the Ruff AST | `recognizer` | `lexical` (C3) |


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
   is part of the producer config (the spike used 512 MiB). The check and the lazy solves all run
   on that thread; it is the producer's config digest, so changing it changes `run_id`. One
   thread is a precaution (upstream's cycle placeholders are per thread) with no difference
   shown. FastMCP is identical at `Inline`, `NumThreads(1)` and `NumThreads(4)` (S3). The
   import-cycle fixture (a return-type cycle and a global cycle whose solved types reach
   `pysa_calls`) is identical at `NumThreads(8)`, 6 runs in each module order (probe,
   2026-09-22); Pyrefly 1.3.1 iterates cycles to a fixpoint. **Tested:** sorted and reversed
   module order and separate processes give identical output.
4. **Handles.** One per project module, from `cfg.handle_from_module_path`, sorted by module name.
5. **Run.** Install a `PysaReporter` with `write_files: false` and `ModuleIds::new(&handles)`, then
   call `transaction.run(&handles, Require::Everything, None)`. Keep the reporter installed during
   extraction (borrow it with `pysa_reporter()`), because dependency modules solve lazily.
6. **Extract** per module, in sorted order (§4.2.2–§4.2.3). Then emit coverage (§3.7).

**Measured** (spike S7; FastMCP 4.0.3, 257 modules, release build): 5.4 s cold at `NumThreads(1)`
(2.5 s check, 2.8 s extraction), peak RSS about 918 MB; 4.1 s and about 765 MB with `Inline`.


### §4.2.2 Syntax: one walk over Pyrefly's parse

- **Order (C2).** Per module the Pysa collectors run, then the walk. Placement depends on the
  source alone: every statement, clause and expression outside annotations is placed
  (`syntax_nodes`, §3.2; C2 review), and Pysa's sites are matched to those nodes in Stage C.
- **One walk.** A single `SourceOrderVisitor` walks `Transaction::get_ast(handle)`, the unmodified
  ruff parse Pyrefly analyzed, which is kept at `Require::Everything`. The text is
  `get_module_info(handle)`'s contents. There is no second parse.
- **Built-ins used:** `SourceOrderVisitor` with `walk_annotation`, `Arguments::iter_source_order`,
  `ArgOrKeyword` and `StringLiteralValue::to_str`.
- **Ours:**
  - the parameter list (the five lists in declaration order, which is source order) and the
    docstring check (a first-statement string literal), a few lines each;
  - the structural occurrence path (each ancestor's ruff `NodeKind` name and child ordinal).
    Ruff's `node_index` is always unset, so it can't be used;
  - the qualified-name stack;
  - the `@overload` decorator match.
- **Recovered and unreadable files.** A module whose acquired bytes fail our own UTF-8 check is
  `unavailable` for every family; Pyrefly would load it as an empty module, which must not read as
  "no API". Parse errors are read from Pyrefly's per-module errors (the `parse-error` kind). A
  recovered tree still yields facts, but **every** family of that module is `partial`, with a
  `boundaries` row, so recovery artefacts never read as complete.


### §4.2.3 Semantics: Pyrefly's own collectors

- **Collectors.** Per module: `PysaResolver::new`, `ModuleAnswersContext::create`,
  `collect_captured_variables_for_module`, `create_reversed_override_graph_for_module`, then
  `export_module_definitions` and `export_module_call_graphs`, the functions behind
  `--report-pysa`. The in-memory structs equal the CLI's JSON (S4: 257/257 modules; definitions
  equal as sets).
- **Locations → bytes.** Every `PysaLocation` is converted with its module's `LineIndex` (§3.4).
- **Join key.** `pysa_calls` joins `call_syntax` on the **full call-expression range**. This
  matched 13,104 of 13,292 calls (S5). Every unmatched call is inside an annotation, which Pysa's
  call model does not cover. Each becomes a `boundaries` row with `outside_provider_model`.
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
  | `Target::FormatString`, `Return` shims, `global_targets`, `captured_variables`, `return_type` | **not carried** in v1: synthetic or no consumer | — | — | — |
  | `Define` | **not carried**: it links a nested `def` to the function it creates, which `declarations` already records | — | — | — |

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
  (`native_structural`) is reachable through `Answers` when a consumer needs it (§13).


### §4.2.4 Binding rule (conservative)

- **The rule.** Any binding of a parameter's name that appears lexically before a guard or
  forwarding site in the same scope marks that site `ambiguous_binding` (a boundary).
- **As implemented** (slice 2.1; review F4). Stricter than "before": a parameter whose name has
  any second binding event in its scope (after the site, or in a statically pruned branch) is
  never followed, and a guard on it is no guard. The trace is an analysis finding, not a
  `boundaries` row (extraction's table): a read of the name at a call into the subsystem is an
  `unfollowed_argument` with reason `rebound`, stated in the brief's Limits (§9.2).
- **The motivating case** is pyarrow's `write_dataset`. Its `schema` guard only applies to
  caller-supplied scanners, because earlier branches rebind `schema`.
- **Pyrefly's flow-sensitive `Bindings` are not used for this rule.** Its binding pass drops
  branches it decides statically (`TYPE_CHECKING`, `sys.version_info`), so a rebinding there is
  invisible.

**Deferred:** Ruff's full semantic model, which nothing public drives (§13).


### §4.2.5 Failure, determinism and the parity oracle

- **Panics abort the attempt.** Any panic in code that touches Pyrefly (`run`, the collectors, the
  public-name helpers, lazy solves during extraction) aborts it, and nothing is published (§6.1).
  There is no `catch_unwind`: Pyrefly treats its state as unsupported after any panic (a poisoned
  lock, unpublished cycle answers), so continuing with the next module is unsafe.
  - Load and parse errors are not panics; they still become coverage rows (§4.2.2). Per-module
    isolation would need ADR-0012's Option 4, a separate process.
- **Determinism oracle.** Reruns, reversed module order, separate processes and perturbed ambient
  variables give byte-identical sorted tables (S2, S3, fixture tests).
- **Harness-equivalence oracle.** A test runs the pinned Pyrefly CLI (the `uv` dev group, same
  revision) with an equivalent generated `pyrefly.toml` and asserts, per project module, that the
  in-process Pysa structs equal its `--report-pysa-format json` output as sets (`module_id`
  removed), and that the public set explains its `--public-only` report (an exact match or a
  public parent prefix).

  It shares the collectors with the CLI, so it checks our driver (configuration, reporter
  lifecycle, lazy solving), not the correctness of Pysa. It is **Tested** (S4, and a nextest test
  on two fixtures, 2026-09-22). The CLI is never a production input.


### §4.2.6 Upgrading Pyrefly

1. Rebase the fork commit onto the new tag and regenerate `third_party/pyrefly-<ver>.patch`.
   - A **logic change** is anything beyond visibility changes, accessors that borrow or compose
     existing upstream queries, and fields whose default reproduces upstream behaviour.
   - A patch that needs a logic change, or grows past about 60 changed lines, needs an ADR
     (ADR-0012's revisit trigger).
2. `just deps` checks that `Cargo.lock`, the driver's `PYREFLY_REV`/`PYREFLY_PATCH_SHA256` and
   pins.md name one revision, that it is the tag plus the patch, and that every `env::var` read
   in the pinned source is classified against the refused list (§4.2.1).
3. Move the ruff pin to the line the new Pyrefly compiles against (`pin-check`).
4. Fix compile errors in the mappers, and append codebook values where exhaustive matches demand
   them. Record how many lines the Pyrefly-facing module changed, because port cost is also a
   revisit trigger.
5. Run the harness-equivalence and determinism oracles, and record the pin with its date in
   `docs/pins.md`.

> Decision: ADR-0012


### §4.3 Fact construction and persistence

**Implemented** in `cpg-schema` (build, canonicalize, ids, derivation SQL, rules) and `cpg-core`
(create, open, write, derive, validate, publish, read), and **Tested** there (slices 1–2,
2026-09-22): every table round-trips exactly through Delta; open refuses drifted or missing
CHECKs; the helper refuses writes; the `INSERT INTO` bypass is asserted; the derived tables are
snapshot-tested on three fixtures; each rule kind rejects an injected violation and nothing
publishes. This section says which built-in owns each step; §6 and §8 hold the protocol and the
rules.

| Stage | Built-in | Ours |
|---|---|---|
| Build | Typed builders against the table's `cpg-schema` `SchemaRef` (`FixedSizeBinaryBuilder`, `Int16Builder`, `BooleanBuilder::append_option`, `GenericListBuilder`, `StructBuilder`). `RecordBatch::try_new` with default options is the local type check: exact types, nested names, nullability, metadata | One builder per table. The arrow-json serde path is tests-only: it expects hex for `FixedSizeBinary` |
| Canonicalize | `lexsort_to_indices` + `take_record_batch` on the table's declared **total** key, over the in-memory batch (Arrow's sort is unstable). The Delta writer may store the rows in another order (it fans partitions into one writer), so readers sort (`read_at`, every rendered query) and never rely on storage order (H1 review O1) | Key declarations |
| Ids | The `blake3` crate (`=1.8.6`, shared with Pyrefly) inside one `IdHasher` (§3.4.1). Not `RowConverter` bytes: the encoding may change between releases. Not SQL `digest`: it can't write length prefixes | `IdHasher` |
| Create | `DeltaTable::create().with_columns(..).with_configuration_property(TableProperty::AppendOnly, Some("true"))` plus, from C1, `EnableExpiredLogCleanup = "false"` and `LogRetentionDuration = "interval 36500 days"` (§6.1), then `add_constraint()` with the table's **immutable** per-row CHECKs: span order and non-negative offsets. delta-rs counts a NULL result as a violation (**Tested**), so a CHECK on a nullable column reads `c IS NULL OR …`. Codebook membership is not a CHECK, because codebooks grow (§8). `CreateBuilder` rejects `delta.constraints.*` keys (Interface-checked: observed in S6, not asserted) | CHECK declarations |
| Open | When an attempt opens a table, compare its `delta.constraints.*`, `delta.appendOnly` and (C1) the two retention properties, as exact strings, with the generated set, in delta-rs's normalized form, and abort on a mismatch. A table left without its constraints (a crash between create and `add_constraint`) is refused | The verify helper |
| Write raw | `DeltaTable::write(batches)` (`WriteBuilder`), with `CommitProperties::with_metadata` carrying `lctx.snapshot_id`. That metadata is audit only; `snapshots` stays the authority. **Tested:** CHECK is enforced, `appendOnly` rejects deletes, and the metadata reads back through `history()` | — |
| Derive | A session over the attempt's tables at their written versions, each filtered to the snapshot (§6.2). The derivation SQL from `cpg-schema` computes derived ids with the `lctx_id` UDF (§3.4.1, C1) and runs through the one helper, `ctx.sql_with_options` with DDL, DML and statements disallowed; no other code calls `ctx.sql`. The result is collected, cast strictly to the declared schema, sorted canonically and written like a raw table, so it passes the same local type check. `with_input_plan` streaming (Tested in S6) is not needed at pilot scale: the review probe on FastMCP 4.0.5 (2026-09-22, Measured) had 33,012 `pysa_calls`, 15,772 `call_targets` and 93,101 `facts` rows. Derived tables can be rebuilt from Delta (DM-23) | SQL per derived table |
| Validate | DataFusion queries generated from the contracts (§8), over the session read once into memory, 8 rules at a time (H1 P2). No float aggregate exists yet; when one does, its query fixes its own reduction order | The generator, semantic rules, and a finite-float loop (there is no built-in `isfinite`) |
| Publish | `snapshots.write([rows])` in one commit (§6.1). **Tested:** a rejected append is classified unpublished by re-reading | Classification after an ambiguous error |
| Read | `DeltaTableBuilder::from_url(..)?.with_version(v).load()`, assert `version()`, `update_datafusion_session`, `table_provider()`. Ids come back through the two-step cast (§3.3). **Tested:** with two snapshots in one table, dropping the version pin or the snapshot filter changes the result. Reading a missing table creates nothing | One helper |

Operations are methods on `DeltaTable`. `DeltaOps` does not exist at this pin.

**Never** (the write, SQL and Parquet-scan items are ast-grep rules):
- DataFusion `INSERT INTO` or `DataFrame::write_table` into a Delta table. The `DeltaDataSink`
  path skips CHECK constraints and invariants; a test asserts the bypass at the pinned revision,
  so an upstream fix gets noticed.
- delta-rs's low-level `RecordBatchWriter` or `JsonWriter` on fact tables (no constraint
  handling; Interface-checked).
- `SaveMode::Ignore`; deletion vectors (they switch off Parquet pushdown); column mapping; raw
  Parquet scans; vacuum or optimize.

**Snapshot-scoped reads (H1 P3; closes the known limit).** Delta log statistics skip the Binary
`snapshot_id` (`writer/stats.rs:212-229` at `58f07cd`), so a filter alone skipped no file and
cost one footer read per file of every snapshot. Instead each pinned read opens **only the files
its version's commit added**: `LogStore::read_commit_entry(v)` + `logstore::get_actions` → the
`Add` actions → `TableProviderBuilder::with_adds` (`snapshot::commit_provider`). A snapshot's rows
of a table are exactly one commit's (one commit per table per attempt, §6.1), the JSON commits are
kept (log cleanup off, verified at open), and a selected file that is gone fails the scan rather
than returning fewer rows (**Tested**: `a_pinned_read_opens_only_its_commits_files`). The
`snapshot_id` filter stays as the row predicate.

**Metrics** (C1, **Implemented**; DP-22; `cpg_schema::metrics`).
- `lctx compile` reports, per stage, wall time and the process's peak RSS so far (`VmHWM`, which
  the kernel updates lazily, so it only grows approximately; it includes allocator retention,
  §4.3 Measured): acquire, Stage A, the Pyrefly check, per-module extraction (with its Ruff walk and
  Pysa collectors), public names, the dependency check and definitions, raw write per table,
  derive per table, validate and publish (review O4).
- They are returned with the published attempt and never stored in Delta: they are not content.

**Measured, the whole CPG (C6, 2026-09-23;** `just pilot` after the C5 and C6 review fixes, with
per-rule validation costs; FastMCP 4.0.5 and its corpus; fresh store, snapshot `ddee0669…`; a
32-thread, 188 GB host; the default glibc allocator**):**
- 905,648 nodes and 1,449,162 edges; every one of the 493 rules passing; **44.6 s** in all (the
  C6 review reproduced 44.2 s and 44.5 s). (At the C5b build, before its review: 907,845 nodes,
  1,452,970 edges, 503 rules, 50.0 s, snapshot `063b8eb3…`.)
- **Extraction, 31.3 s.** The library run: the Pyrefly check 2.1 s, per-module extraction 5.1 s
  (4.4 s of it the Pysa collectors), and the dependency check 3.8 s. The corpus run: its check
  3.4 s, per-module extraction 7.8 s, its dependency check 3.7 s, and the documents 0.2 s.
- **Raw writes, 0.7 s. Derivation, 2.4 s** (`edges` 1.1 s, `nodes` 0.5 s).
- **Validation, 10.0 s:** 493 queries, the slowest 0.15 s (`unique:type_terms`), so the cost is
  their number, each re-scanning its Delta views. `lctx compile` reports the three slowest rules and
  the one that raised the peak most.
- **Peak RSS, and what it measures** (C6 review F3). With the default allocator the peak is
  7.1 GB here, and 6.7–8.0 GB across seven runs with identical inputs. It climbs from 3.8 GB after
  the raw writes to 5.1 GB after derivation and 7.1 GB after validation. **About 40–45% of it is
  glibc arena retention, not working set:** under `MALLOC_ARENA_MAX=2` the same compile peaks at
  4.2 GB (3.7 GB after extraction, +0.5 GB in derivation, nothing in validation), but takes
  61.2 s (derivation 12.2 s, validation 16.9 s). `VmHWM` is updated lazily, so "only grows" holds
  approximately. The raw batches are now released once written (`attempt::compile_owned`): the
  effect is inside the default allocator's spread, and 0.1 GB under the arena limit (4.3 → 4.2 GB).
- **Decision:** streaming derive stays deferred, since its trigger is not met: derivation is 5% of
  the wall time, and its working set about 0.5 GB.

**Measured, after H1 (2026-09-23;** `just pilot` on a fresh store at `fff5aa5`: jemalloc
(ADR-0016), validation over cached tables 8 at a time, per-commit reads, zstd, fork `a07b7bae`;
FastMCP 4.0.5 and its corpus; snapshot `15fecdab…`, content `10e56541…`; the same host**):**

| Stage | Before H1 (baseline, `1a4c4406…`) | After H1 |
|---|---|---|
| Total | 45.0 s | **29.9 s** |
| Extraction | 31.4 s | 25.7 s (library: check 1.8 s, per-module 4.2 s, dependencies 3.1 s; corpus: 3.1 s, 6.7 s, 3.0 s, documents 0.2 s) |
| Raw writes | 0.73 s | 0.95 s (zstd) |
| Derivation | 2.56 s | 2.24 s (`edges` 1.02 s, `nodes` 0.50 s) |
| Validation | 10.16 s | **0.90 s** (the slowest rule's compute 0.28 s, `key:edges`; the largest hash build 160 MiB) |
| Peak RSS (`VmHWM`, MiB as `lctx` prints) | 7,587 MiB (6,678–8,044 MiB across seven runs) | **3,646 MiB** (3.6 GiB), flat from extraction on |
| Store (`du -h`) | 272 MiB | 235 MiB |

- 905,648 nodes and 1,449,162 edges, all 496 rules passing, before and after.
- **What stayed the same.** Through H1b, every runtime-only commit left the content digest
  (`f01077be…`) and the table fingerprints (the sorted ids of `facts`, `nodes`, `edges`,
  `type_terms`, `syntax_nodes`, `bindings`, `mentions`, hashed) identical to the baseline.
- **What moved.** The fork revision bump (D6) moves producer, run and fact ids. It also moves the
  1,085 nodes of Pyrefly's bundled stubs (their identity is the Pyrefly revision, §3.4.1) and the
  91,278 edges that touch them. Every other node and edge is byte-identical to the pre-D6 build.
- **The test suite** runs in 83 s instead of 225 s.
- **After the H1 review's fixes** (2026-09-23, fresh stores): content `19c3e8e2…`. F1's extractor
  version bump moved producer, run and fact ids; the node, edge, type-term, syntax and binding
  fingerprints are identical to the table above. 29.9–33.4 s across three runs, peak
  3,640–3,649 MiB, 235 MiB. A compile's stderr carries no warning (the known Binary-statistics
  lines are quieted, and tables are created without a failed load; H1 review F6).

**Deferred, with triggers.**
- `datafusion-tracing` (compatible with 55.1 per its skill): until per-operator spans are needed.
- **Streaming derive:** `WriteBuilder::with_input_plan(LogicalPlan)` streams per partition and
  still enforces CHECKs (read in the pinned source, `write/execution.rs:405-431`, 2026-09-22).
  - It would need its own schema and foreign-snapshot checks, and row counts from write metrics.
  - Reopen when derivation dominates the per-stage time, or its working set dominates the peak.
    Under jemalloc (ADR-0016) the reported peak tracks the working set (flat and repeatable,
    within 7 MiB across runs; jemalloc still holds freed pages for its decay period); the earlier
    `MALLOC_ARENA_MAX=2` reading applied only to glibc. At C6 it did neither (above).
- **Validation over cached tables and concurrent rules: taken** (H1 P2, operator 2026-09-23).
  `validate` reads every registered table once through its pinned, snapshot-filtered Delta view
  into a `MemTable` (`cached_session`), then runs the rules 8 at a time on spawned tasks, putting
  violations back in `rules()` order; every session plans on `TARGET_PARTITIONS = 8`. Still one
  query per rule, the same SQL, the one shared validator (§B3). Per-rule cost is read from each
  rule's own physical plan (wall time, summed `elapsed_compute`, largest `build_mem_used`; H1 P5),
  since a process-wide peak delta means nothing under concurrency. **Measured** on the pilot:
  validation 10.2 s → 0.89 s, the peak unchanged (3,646 MiB under jemalloc).
- **Peak memory:** taken by ADR-0016 (jemalloc: the peak tracks the working set, 3,646 MiB on the
  pilot, flat from extraction on) and the raw batches released once written. Reopen when the
  peak nears the host's memory.
- **File skipping on `snapshot_id`: taken** (H1 P3, above): no schema, partition or store change.

> Decision: ADR-0012, ADR-0014, ADR-0016

---
