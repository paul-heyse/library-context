# Inquiry L2: Analysis and compute libraries

**Provenance.** This is the verbatim final report of supporting inquiry L2, from one read-only
`library-research` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `L2-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# L2 — Analysis and compute libraries: supporting assessment

Baseline: `main` at `88f33bc0`, read 2026-10-11. This is read-only static evidence. Nothing was built, run or queried. Files written: **none** (one scratch copy of the HEAD plan was made under the session scratchpad, outside the repository).

## 1. Scope examined and coverage limits

**Read in full or in the relevant parts:**
- `crates/cpg-extract/src/pyrefly_stage.rs`: `session`, `configured`, `run`, `types`, the dependency-context loop, `analysis_pool`.
- `ty_flow.rs`: `run`, `Index::new`, `qualify`.
- `ruff_context.rs`, `diagnostic_records.rs::ruff`, `contracts.rs`.
- `cpg-flow/src/{db.rs, lib.rs::{index, module, rename, program_settings}}`.
- `lctx-model/src/domain/conditions/{graph.rs, kernel.rs}`.
- `analytics/{concepts.rs, ranking.rs, communities.rs}` (headers and kernels).
- `derivation.rs::acyclic`, `graph.rs::{admit_graph_derivations, admit_derivations}`, `scope_program.rs::CompactScope`, `projection/snapshot.rs::MaterializedGraph`, `execution/summary_schedule.rs`.
- `cpg-core/src/facts.rs` (profile filtering), `analysis_graphs.rs`.

**Library side, all at the pinned revisions:**
- pyrefly fork `63cda07`: `pyrefly/lib/state/state.rs` (`State`, `Transaction::run`/`run_internal`/`run_step`, the commit paths) and `crates/pyrefly_util/src/thread_pool.rs`.
- ruff/ty fork `9080ee7`: `ruff_linter/src/{linter.rs::check_path, semantic_facts.rs, checkers/ast/mod.rs::check_ast}`, `ruff_db/src/parsed.rs`, `ty_python_core/src/lib.rs::semantic_index`, `ty_project/src/lib.rs` (parallel check), `ruff_python_parser/src/{lib.rs, lexer.rs}`.
- salsa 0.28.5: `Cargo.toml` features and the persistence sources.
- petgraph 0.8.3: `algo/page_rank.rs`.

**Skills:**
- `python-analyzers`: SKILL.md and the briefs `in-process-session`, `retained-parse`, `parallelism`, `single-file-db`, `star-imports`.
- `salsa`: the `persistence` and `durability` briefs.
- `rust-graphs`: centrality and connectivity.
- `rust-reasoning`: `fca-concepts` and `bdd-bounded-apply`.

**Versions:** checked against `Cargo.toml` `[workspace.dependencies]`, `cargo tree --offline` and `docs/pins.md`.

**Not examined:**
- Behavioral-only kernels (`summary_worklist`, `summary_*`), except their library calls.
- The Leiden configuration beyond the adapter.
- `typed_syntax.rs` and `lexical.rs` internals.
- `type_records.rs` beyond the record-field path.
- Ascent/datafrog fit for behavioral closures.

Context7 was not needed: every decisive question was settled at the pinned source.

**Static counts used below** (dated 2026-10-11; `find`/`rg --no-ignore` over `build/envs/fastmcp/lib/python3.14/site-packages` and `build/sources/fastmcp/004bf15a…`):

| Location | Count |
|---|---|
| `fastmcp` release | 257 `.py` files; 166 with a `class`; 64 mentioning `TYPE_CHECKING` |
| `fastmcp_tasks` | 18 files |
| Whole environment (dependency context) | 7,888 `.py` files, 56.9 MB |
| Corpus | 853 `.py` files (427 under `tests/`, 125 under `examples/`) |

**Correction to the operation map §1.3.** The catalog schedule does **not** run TyFlow. `ty_flow::contract` declares `profiles: vec![Profile::Behavioral]`, and `facts::compile_facts` filters providers with `.filter(|p| p.declaration(profile).profiles.contains(&profile))`. Under the catalog profile, five providers run. ty flow and the condition kernel's flow lowering belong to S04, not S01.

## 2. Complete operations examined

**Pyrefly provider, per captured input** (catalog and behavioral; for the pilot that is the installed release and the corpus):
1. `pyrefly_stage::configured`: a constant `ConfigFile`. The corpus search path is the corpus root, plus the whole library environment as site-packages.
2. `State::new(ConfigFinder::new_constant, ThreadCount::Inline)` creates a **fresh State per input**. `transaction.run(roots, Require::Everything, Some(analysis_pool()))` then runs on a process-static rayon pool: `ThreadCount::AllThreads` = min(available CPUs, 64) = 32 workers with 10 MiB stacks (`ThreadPool::new`). Dependencies load at `Require::Exports`, plus lazy solves.
3. The pool joins. A **serial per-root loop on the provider thread** (512 MiB stack) then does the following for each root module:
   - `get_ast`/`get_module_info` (pyrefly's retained 0.0.14 parse);
   - `CanonicalSyntax::parse` (latest Ruff 0.0.16 parse);
   - `NativeBranches::observe` (walks the pyrefly AST);
   - `typed_syntax::emit` plus the lexical recognizer (walks the canonical AST);
   - `canonical.diagnostics` → `diagnostic_records::ruff`, which runs `ruff_linter::linter::check_path` **twice** (`Noqa::Enabled`, then `Noqa::Disabled`);
   - `canonical.context_rows` → `semantic_facts::observe_parsed` → `check_ast_with_observation`, a **third** Ruff Checker traversal with an empty rule table;
   - Pysa `definitions`/`calls`;
   - `types()`, which can parse further canonical copies (L2-02);
   - emission through `StageContext` (bridge crossing, M2).
4. The dependency-context loop: waves of `transaction.run(deps, Require::Everything, None)`. With `None`, the run falls back to the State's own executor, which is **Inline**.

Live state: the whole State, at `Everything` for every root and the upgraded dependency modules (AST, answers, trace), stays alive through the serial loop.

**ty flow, per input** (behavioral only):
1. `TyFlow::run` → `Index::new`, which materializes 11 Pyrefly-produced relations into `BTreeMap`s.
2. `cpg_flow::index` builds a fresh `FlowDb` (salsa `Storage::new(None)`, `TestSystem`).
3. Per module, serially on the provider thread:
   - `rename` (a full `parse_unchecked_source`, used only for its tokens);
   - `write_file`;
   - `parsed_module` (ty's own parse of the renamed text);
   - `semantic_index`;
   - the `Walk` translation into `CondGraph`.
4. Every module's `ModuleFlow` is collected before writing. For each fact, the Writer lowers through `Diagram::from_graph` (biodivine).

**Graph and condition kernels on the catalog path:**
- `PreparedGraphs` for Structural: petgraph `Graph<…, u32>`, `Dfs`/`EdgeFiltered`/`Reversed` (`MaterializedGraph::reachable`).
- Admission cycle checks: petgraph `toposort` over `DiGraphMap` (`admit_graph_derivations`, `derivation::acyclic`).
- Analytics (Leiden, ranking, FCA) is `NotRequested` under catalog.

**Thread and runtime inventory for compile:**
- the pyrefly rayon pool (32 workers, process-static, idle after the analysis run);
- the tokio main runtime (multi-thread);
- the native bridge thread (current-thread runtime);
- provider threads, one at a time;
- no direct `rayon` use in lctx crates (`rg update_rayon|par_iter|rayon::` over `crates/`: no hits);
- salsa is resolved without its `rayon` feature (`cargo tree -i salsa -e features`: `compact_str`, `inventory`, `macros`, `ordermap`, `salsa_unstable`);
- `tokenizers` (cpg-core) and `blake3`'s `rayon` feature (workspace-hack) can create rayon's global pool. That pool is separate from pyrefly's, but they run in other phases.

## 3. Candidate findings

### L2-01: Latest Ruff traverses each root module three times where one traversal exists in the fork

- **Observed cause.**
  - `diagnostic_records::ruff` builds `Stylist`/`Indexer`/directives, then calls `ruff_linter::linter::check_path` twice: `check(Noqa::Enabled)` and `check(Noqa::Disabled)`. It reconciles them by a multiset over `(id, code, span, headline)` keys. Each call runs a full `check_ast` Checker traversal for the three selected rules (F821, F401, F841, all AST/deferred-scope rules).
  - `ruff_context::CanonicalSyntax::observe` → `semantic_facts::observe_parsed` → `run` rebuilds `Stylist`/`Indexer`/`LinterSettings` and calls `check_ast_with_observation` a third time, with `RuleTable::empty()`.
  - In the fork, `check_ast` is literally `check_ast_with_observation(…, None)` (`ruff_linter/src/checkers/ast/mod.rs::check_ast`).
  - In `linter.rs::check_path`, `check_noqa` already computes the `ignored` indices and only `swap_remove`s them when noqa is enabled.
- **Consequence.** S01, S02 and S03, catalog and behavioral: about 275 release roots plus the corpus roots per compile, re-run on every warm compile (§2). Three semantic-model builds and three binding/deferred-scope passes over the same `Parsed`, where one would do. Mechanisms: H28 (remove avoidable work), H2 (a physical unit per semantic distinction), H12. The multiset key reconciliation is also weaker than native indices: two same-message diagnostics on one span are matched by count, not identity.
- **Simpler conforming alternative (fork-owned).** Extend the fork's entry point so that one `check_ast_with_observation` call runs the three rules with the observation sink. `check_path` (or a sibling) then returns the unsuppressed diagnostics together with `check_noqa`'s `ignored` index set. Emitted = unsuppressed minus ignored. This is the same mechanism the fork already patches (`third_party/ruff-0.16.10.patch` carries `semantic_facts`). No new library is needed.
- **Preservation constraints.**
  - Observation rows must be rule-independent. Enabling rules must not change the `Fact` stream; the observation run currently uses `Noqa::Disabled`, an empty `NoqaMapping` and `RuleTable::empty()`.
  - The `RuffNoqaSuppressed` vs `Emitted` channel distinction must be kept.
  - `provider().build_digest` and the `semantic_revision` for the Ruff supplier change.
- **Uncertainty.** Whether any Checker path consults enabled rules in a way that alters the visited or deferred structure (for example, rule-gated deferred visits). Magnitude relative to pyrefly analysis is unmeasured.
- **Evidence label.** Implemented (current behavior: source-observed). Remedy: Proposed.
- **Closure evidence.**
  - A parity control over the existing Ruff fixture corpus: observation rows and both diagnostic channels byte-identical between the three-pass and one-pass forms, plus a two-identical-diagnostics noqa case.
  - Phase timing of the pyrefly provider before and after (measurement).

### L2-02: Type extraction re-parses and re-emits typed syntax for the current module and ancestor modules, once per root

- **Observed cause.**
  - `pyrefly_stage::types` creates `let mut cached = BTreeMap::new()` **inside** each per-root call.
  - Its `field_occurrence` closure, for every field it is asked about, scans `roots` linearly. It computes `root.join(&a.path)`, which allocates a `PathBuf` per comparison.
  - On a cache miss, it runs `CanonicalSyntax::parse` and then `typed_syntax::emit` over the found module, only to build `Spans`.
  - In `type_records.rs`, the record-field path builds `owners = [(class, ctx), …ancestors]`. It calls `field_occurrence(defining, r)` for **every** owner, **including the class's own module**; there is no `declaring.module() == &ctx.module_info` shortcut there. The parameter path at the second call site does have that shortcut and uses the already-built `spans`.
  - So every root module with class fields is canonically parsed and syntax-emitted at least twice. Each ancestor module in another root is re-parsed once per inheriting root.
- **Consequence.** S01 and S03: 166 of 257 `fastmcp` release modules contain classes; pydantic-model hierarchies inherit across modules. S02 scales this with root count × inheritance fan-in. Mechanisms: H10 (amplification), H12 (preparation repeated although its premise is unchanged), H28.
- **Simpler alternative.**
  - For the own-module case: reuse the `spans` already in scope, as the parameter path does.
  - For ancestors: a provider-session (per-input) cache of `Spans` keyed by artifact ID, charged to the existing budget. Optionally bound it (evict after the last referencing root), since the main loop visits roots in path order.
  - Replace the linear `roots` scan with a `path → artifact` map built once.
- **Preservation constraints.**
  - `Spans` must be identical. They come from the same captured text (`info.lined_buffer().contents()`), the same parser options (artifact plus Python version) and the same `SyntaxLimits`. `ContextSettings` affects only observation and diagnostics, not `typed_syntax::emit`.
  - Refused-module behavior (`None`) is kept.
  - Memory charges remain.
- **Uncertainty.** Fan-in distribution. The live-state cost of a session cache (seam to H).
- **Evidence label.** Implemented (source-observed). Remedy: Proposed.
- **Closure evidence.**
  - A typed-records equality control on a fixture with same-module and cross-module record fields.
  - A count of canonical parses per provider run (one per root) as a debug counter in a focused control.

### L2-03: Dependency-context analysis waves bypass the shared analysis pool

- **Observed cause.**
  - The 2026-10-10 change (commit `78c00134`; comment on `pyrefly_stage::analysis_pool`: "Analysis invocations share the library's available-CPU worker pool") moved only the root run to `Some(analysis_pool())`.
  - The dependency-context loop still calls `transaction.run(&dependencies, Require::Everything, None)`. In pyrefly `Transaction::run_step`, `custom_thread_pool.unwrap_or(&self.data.state.threads)` falls back to the State's `ThreadCount::Inline`, which `ThreadPool::new` turns into "no pool; run on the calling thread".
  - Each wave (modules referenced by facts, public traces, requirements, then MRO ancestors) is therefore solved at `Everything` single-threaded on the provider thread. The `loop` repeats until no new modules appear.
- **Consequence.** S02 explicitly names "heavy dependency context". The pilot already has 7,888 dependency `.py` files in the environment. Waves that upgrade pydantic, mcp or starlette modules from `Exports` to `Everything` run serially while 32 pool workers idle. Mechanisms: H22, H21, H9.
- **Simpler alternative.** Pass `Some(analysis_pool())` to the dependency `transaction.run`. The pool already joins before extraction inspects answers, which is the comment's own rationale.
- **Preservation constraints.**
  - Deterministic outputs: the root run already relies on pyrefly being thread-count-neutral, so no new premise is introduced.
  - Stack-depth headroom: pool threads have 10 MiB (`PYREFLY_STACK_SIZE` default) versus 512 MiB on the provider thread. Dependency modules may nest more deeply than roots, though pyrefly's own over-nesting guard (`MAX_EXPRESSION_DEPTH = 1000`, per the `retained-parse` brief) bounds expression depth.
- **Uncertainty.** Whether Inline was deliberately kept here for stack depth. No comment says so, and the commit touched only the root run. Wave sizes are unmeasured.
- **Evidence label.** Implemented (source-observed). Remedy: Proposed.
- **Closure evidence.**
  - The existing dependency-context extraction controls pass unchanged with the pool.
  - Facts digests are identical across `NumThreads(1)` and `AllThreads` on one fixture.
  - Phase timing.

### L2-04: Post-analysis extraction is strictly serial although the library permits shared read-only access

- **Observed cause.**
  - After the joined run, `session` iterates `roots` serially on the provider thread. It threads `&mut Natives`, `&mut StageContext` and per-module accumulators (`accesses`, `imports`, `root_modules`).
  - The library side permits concurrency. Pyrefly itself shares `&Transaction` across pool workers (`run_step`: `pool.spawn_many(|| … self.work() …)` requires `Fn() + Sync`, so `Transaction: Sync` is compile-established upstream), and `Subscriber: Sync`.
  - The persisted-graph plan §9.1 row "Provider parallel analysis and checked completion" (2026-10-10) records "subsequent extraction remains ordered/inline".
- **Consequence.** S01, S02 and S03: the per-root loop (two parses, about five AST walks, three Ruff Checker passes, Pysa contexts, type records) runs single-threaded for the release and corpus roots. A plausible but unmeasured share of the provider phase. The same plan records 27 of 69 `providers:extract` controls timing out at 300 s (2026-10-10, before later provider corrections; cause not attributed). Mechanism: H22.
- **Simpler alternative.**
  - Compute per-root record sets in parallel on the existing `analysis_pool`, with module-local interning, using `ThreadPool::install` plus a scoped parallel map.
  - Then merge `Natives` interning and emit in path order on the provider thread. Lazy `ad_hoc_solve` calls are designed for concurrent readers.
  - This is distinct from L2-03: it needs merge semantics for `Natives` and ordered emission, not a one-argument change.
- **Preservation constraints.**
  - Deterministic emission order and IDs; charged budgets (the `ResourceBudget` must be thread-safe, or per-task sub-reservations are needed); `Partial` coverage semantics.
  - Peak live state rises: parallel record sets are held before emission (H17).
- **Uncertainty.** The share of provider time in the serial loop versus `transaction.run`. Whether pyrefly's lazy solves contend on internal locks.
- **Evidence label.** Interface-checked (`Sync` from upstream usage). Remedy: Proposed.
- **Closure evidence.**
  - Existing phase observations ("provider execution/completion") split into analysis-run time versus extraction time (measurement), before any implementation.
  - Then identical facts digests between the serial and parallel forms.

### L2-05: Flow conditions are tree-copied and lowered node by node through fresh BDD variable sets (S04)

- **Observed cause.**
  - `CondGraph::join` clones both operands' node vectors on every `and`/`or`. Shared subconditions are duplicated, so a DAG becomes a tree, capped at `MAX_GRAPH_NODES = 4096`, beyond which the result becomes `Refused` (approximate).
  - `cpg_flow` builds conditions with chained `and`/`or` (`predicate.rs`, `lib.rs` region and boolean-operator handling, `narrowing.rs`). It memoizes per diagram node, but joins still copy.
  - `ty_flow::Writer::qualify` then calls `Diagram::from_graph` per fact, with no memo across facts. That lowers every live `And`/`Or` node via `Diagram::and/or`, and each of those:
    - builds a union support `BTreeSet`;
    - formats a `String` per atom (`kernel::var_name`) into a **new** `BddVariableSet`;
    - `transfer_from`s both operands;
    - then applies the operation.
  - `Diagram::from_expr` shows the efficient form already exists: one support, one variable set, folds within it.
- **Consequence.** S04 only, per use/definition/reaching fact. Mechanisms: H9/H10 (the representation inflates shared structure; per-operation re-preparation), H12. Also a precision exposure: tree inflation can hit the 4096 cap and turn an exact condition into `Unknown`. That is CI-04-safe, because refusal is explicit, but it is avoidable loss.
- **Simpler alternative.**
  - Lower each `CondGraph` with one support and one `BddVariableSet`, as `from_expr` does, keeping the existing pair-work and node limits.
  - Memoize lowering per distinct graph within one Writer, keyed by structural graph equality (which `CondGraph` already derives), not by digest.
  - Optionally hash-cons `CondGraph` nodes inside `join` to keep sharing.
  - A library alternative, an OxiDD shared manager, is rejected by the `rust-reasoning` brief for this use: whole-manager failure rather than per-operation refusal (`bdd-bounded-apply`, OxiDD row).
- **Preservation constraints.**
  - Canonical `Diagram` identity: variable order = sorted atom IDs, which a single sorted support preserves.
  - The existing `MAX_PAIR_WORK`/`MAX_NODES` refusals and their distinct obligation kinds.
  - `approximation` propagation.
- **Uncertainty.** The frequency of non-trivial conditions and of 4096-cap refusals in real flow facts. `flow.skips` counters exist but do not count `Refused`.
- **Evidence label.** Implemented (source-observed). Remedy: Proposed.
- **Closure evidence.**
  - Identical `Condition` IDs between the old and new lowering over the flow fixtures.
  - A count of `KernelBoundary` refusals before and after.

## 4. Opportunities and uncertainties that are not findings

**Decisive question 1: library-owned incrementality for S03.**

*pyrefly.*
- `State`/`Transaction` incrementality is resident-only:
  - committed epochs;
  - `invalidate_disk`, `set_memory`, `invalidate_config`;
  - export-change propagation with early cutoff at exports (`run_internal`'s epoch loop: "recompute modules who depend on modules that changed").
- There is no serialization of `State` at `63cda07`: no `Serialize` in `pyrefly/lib/state/`, and no persistence module (searched `state/*.rs` for serde, save, persist and disk-cache terms).
- Cross-process reuse is therefore **unavailable**. Cross-run reuse would need a long-lived compiler process holding one State per (library, config) and applying `invalidate_disk` for changed captured files. That is a process-model change, so a candidate rule impact (§7). Even then, pyrefly reports which modules were re-solved, not which facts changed. Fact-level reuse would still need an lctx owner of "module answers unchanged".

*ty / salsa.*
- `FlowDb` is created, used for one revision and dropped per input. Durability, LRU (`parsed_module` has `lru=200`) and backdating are therefore inert, and `semantic_index` is `no_eq` (no backdating) in any case.
- salsa 0.28.5 has a `persistence` feature (`Cargo.toml [features]`), but:
  - ty's queries are not `persist`-annotated (`rg persist` over `ty_python_core`, `ruff_db`, `ty_module_resolver`: no attribute hits);
  - the salsa brief records that every input a persisted query reads must itself be persisted, and that a persisted function returning tracked or interned structs requires `persist` on them (E0277).
  - Persisting `SemanticIndex` is therefore a deep fork change. **Not viable.**

*Ruff.* Parse, lint and observation are pure per file: inputs are bytes, `Settings` (path, package root, version, builtins, typing modules) and the rule set.

*Conforming route.*
- Module-granular reuse lies in lctx, not in the libraries:
  - Ruff outputs keyed by (captured content, Ruff supplier digest, settings);
  - ty flow keyed by (content, version, platform, plus the export sets of star-import targets, because `semantic_index` creates `StarImport` definitions only for resolved targets, per the `star-imports` brief);
  - pyrefly only by transitive-closure equality, or residency.
- This is a facts-product granularity question owned by inquiries A and C. The library facts above are its premises.
- The §14.11 Salsa trigger ("demonstrated fine-grained derivation cost beyond coarse reuse") is not met by library capability alone.

**Decisive question 2: parses and indexes per root module under catalog.**

| # | Work | Status |
|---|---|---|
| 1 | Pyrefly parse at 0.0.14, with bindings, answers and solutions | Necessary for typing |
| 2 | Latest Ruff canonical parse at 0.0.16 | Necessary under §B1/ADR-0117: separate nominal crate lines, canonical syntax owner |
| 3–5 | Three Ruff Checker semantic-model traversals | One is necessary (L2-01) |
| 6 | Bespoke lexical recognizer walk | Retained deliberately: `analyzer-fidelity-alignment-plan` defers replacement until Ruff or ty offers multi-candidate and LOAD_NAME semantics. Not examined further. |
| 7 | `NativeBranches` walk over the pyrefly AST | Part of the per-root loop |
| 8 | `typed_syntax::emit` walk | Part of the per-root loop |
| 9+ | Extra canonical parses in `types()` | Avoidable (L2-02) |

Under behavioral, add:
- a `rename` parse: full parse used only for tokens; `ruff_python_parser::lexer::lex` is public at 0.0.16. This is a minor opportunity, kept out of the findings because parser error recovery can re-lex, so a lexer-only stream could differ on invalid modules;
- ty's own parse of the renamed text, which is necessary because ty's index references its own `ParsedModule` and the text differs;
- one `semantic_index`.

**Sharing pyrefly analysis between the release and corpus inputs.**
- The corpus State re-loads the release modules (and the dependency closure and stdlib via `compute_stdlib`) in a second State.
- A single State with a path-sensitive `ConfigFinder` could share them, but only if every release module resolves imports identically under both configurations. The corpus root is first on the corpus search path and can shadow names.
- This is a precondition-bearing opportunity, not a finding.

**ty flow parallelism (S04).**
- `cpg_flow::index` is serial per module.
- ty's own checker parallelizes across files with cloned database handles (`ty_project/src/lib.rs`, `into_par_iter().for_each_with_project_db`), and `FlowDb` derives `Clone`.
- ty's checker also `clear`s parsed ASTs after use to bound memory; `cpg_flow` retains every `ModuleFlow` and the database until the input finishes (seam to H).
- Unmeasured; S04 only.

**FCA kernel.** `concepts.rs::implication_closure` is the naive fixpoint over the basis. LinClosure is an established linear-time alternative. Not on the catalog path, and bounded by `max_examined`. An opportunity only.

## 5. Strengths, preservation constraints and areas checked adequate

**Pyrefly in-process use matches the library's intended pattern** (`in-process-session` brief):
- a constant `ConfigFinder`;
- dependencies at `Require::Exports` with roots at `Everything`;
- the retained AST used for native joins;
- the Pysa reporter with `write_files: false`.

The one process-static pool avoids per-input pool construction.

**Provider-role separation (§B1) is physically honest.** Latest Ruff and Pyrefly's embedded Ruff are separate nominal crates (`ruff_python_parser = "=0.0.14"` versus the `_latest`/`_ty` aliases on one fork revision). Byte-checked capture correspondence is enforced in `CanonicalSyntax::parse` and `ty_flow` (`ContentHash::of(&bytes) != artifact.content`).

**Thread coordination (decisive question 3) is adequate.** There is no nested pool contention:
- pyrefly's pool runs while tokio is idle for this provider;
- emission happens after the join;
- lctx code has no rayon use;
- salsa runs without rayon;
- ty never sizes a pool (the `parallelism` brief: `max_parallelism` is unused by `ty_python_core`).

The coordination issues are underuse (L2-03, L2-04), not oversubscription.

**G8 graph kernels (decisive question 4): library-owned where it fits; bespoke kernels are justified.**
- SCC uses `petgraph::algo::kosaraju_scc` (`summary_schedule::invocation_sccs`, `projection/normalization.rs`). The skill notes that `tarjan_scc` has fewer bounds; this is trivial.
- Cycle checks use `petgraph::algo::toposort` over `DiGraphMap`. It is iterative, which the code comments rely on; `is_cyclic_directed` is recursive.
- Traversal uses petgraph `Dfs` with `EdgeFiltered` and `Reversed`.
- Communities use `leiden_rs` from a dense index, sequential, with library ARI and NMI metrics.
- `CompactScope::select` is a 15-line multi-seed DFS over `FixedBitSet`. It is equivalent to petgraph `Dfs::empty` + `move_to`; adequate.
- **Weighted ranking: bespoke, and the library alternative is worse.** petgraph 0.8.3 `page_rank` is unweighted and computes, per iteration, for every (v, w) pair, `graph.edges(w).any(|e| e.target()==v)`: O(V²·deg) per iteration. It has no tolerance or work bound. The bespoke `ranking::rank` is O(E) per iteration with checked weights, canonical order and a work limit. **G8 refuted.**
- **FCA: bespoke NextClosure over `fixedbitset`, justified.** fcars gives concepts only, with no implications, support threshold or order (`fca-concepts` brief). fcars and odis are dev-only oracles (`cargo tree`), which keeps independent assurance (calibration 7).
- **Conditions: biodivine with `binary_op_with_limit` plus a pair-product preflight**, exactly the `bdd-bounded-apply` brief's recommended pattern. A typed refusal maps to obligations.

**Containers suit S02 scale:**
- `Graph<_, _, Directed, u32>` with an explicit `check_capacity`;
- dense indices;
- `FixedBitSet` sizes charged with a SIMD tail allowance (`concepts::bit_bytes`).

One noted duplication: `MaterializedGraph` keeps `outgoing`/`incoming: Vec<Vec<ArcId>>` beside petgraph's own adjacency, presumably for sorted arc order. It is minor, and a seam to H.

**Preservation constraints to carry into any remedy:**
- deterministic emission and IDs;
- charged budgets;
- explicit refusal (never a silent cap);
- capture byte verification;
- separate supplier digests per provider role;
- the independent dev-only oracles.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| Persisted-graph plan §9.1, "Provider parallel analysis and checked completion" (2026-10-10, Implemented/source-accepted) | **Residual** | The root run uses `analysis_pool()`; dependency waves remain Inline (L2-03); extraction is serial (L2-04) |
| Persisted-graph plan "PG3 … no provider algorithm or blanket parallelism change" | **Confirmed still present** (as a disposition); see L2-03 and L2-04 | `pyrefly_stage::session` |
| Operation map §1.3: 6 providers in the catalog schedule | **Refuted** for TyFlow | `ty_flow::contract().profiles == [Behavioral]`; filter in `facts::compile_facts` |
| Operation map §2: facts providers always re-run on warm recompile | **Confirmed still present**; no library-owned cross-process remedy exists at the pins | pyrefly State has no persistence; ty queries are not `persist`; §4 |
| §14.11 conditional Salsa for compile reuse | **Not triggered** by library capability; salsa persistence is unusable for ty's index without deep fork work | salsa `persistence` brief; `ty_python_core::semantic_index` |
| `docs/pins.md` FCA row: "production kernel retained; oracle has no equivalent allocation/work contract" | **Confirmed** | `concepts.rs`; `fca-concepts` brief |
| `docs/pins.md` leiden-rs: sequential, no rayon, dense index | **Confirmed** | `Cargo.toml`, `communities.rs` |
| Analyzer-fidelity plan: lexical recognizer replacement deferred | **Not examined** beyond confirming three parallel scope analyses exist (recognizer, Ruff SemanticModel, ty index under behavioral) | `lexical.rs` header; `acquisition-and-extraction.md` provider table |
| `python-analyzers` SKILL: "a second parse is a second identity" (re-parsing beside pyrefly) | **Moved/accepted by design**: the second parse is the §B1 canonical owner, not an accident; the extra parses are L2-02 | `ruff_context.rs`; ADR-0117 |

## 7. Candidate rule impacts

- **Process model, for S03 via pyrefly residency.**
  - Rule: `lctx compile` is a one-shot CLI (AGENTS.md Commands; DESIGN §1 compile path).
  - Proposed change: allow an optional long-lived compiler worker that keeps a pyrefly `State` (and a ty `FlowDb`) per (library, config). Patch upgrades could then apply `invalidate_disk` and rely on pyrefly's export-cutoff propagation.
  - If the rule is kept: S03 proportionality must come from lctx-owned per-module fact reuse (§4), with pyrefly re-run in full. That is the recommended default.
- **No other rule impact.** L2-01 is a change within the existing Ruff fork. ADR-0117/0118 already govern fork changes ("fork changes get a new pinned revision"), so L2-01 needs a fork revision bump, not a rule change.

## 8. Proposed light probes

1. **Where provider time goes** (decides the priority of L2-01, L2-03 and L2-04).
   - Command: `rg --no-ignore -n 'provider_execution|providers' /home/paul/library-context/build/runs/*/summary.json | head -50`. Then read the referenced phase or event logs for a completed real-provider compile control.
   - Discriminating outcome: a provider "execution" interval much longer than the joined `transaction.run` share points at serial extraction (L2-04 high priority). If no run separates the two intervals, the probe is inconclusive, and the coordinator should request a timed run under authorization.
2. **Inheritance fan-in for L2-02.**
   - Command: `rg --no-ignore -o '^class \w+\(([\w.]+)' -r '$1' /home/paul/library-context/build/envs/fastmcp/lib/python3.14/site-packages/fastmcp -g '*.py' | sort | uniq -c | sort -rn | head -20`.
   - Discriminating outcome: many classes subclassing bases defined in a few other release modules means many repeated cross-root re-parses (L2-02 material). Mostly local or `BaseModel` bases means mainly the own-module duplicate remains.
3. **Pyrefly thread-count neutrality** (a premise for L2-03, needs no build).
   - Command: `rg -n 'TEST_THREAD_COUNT|ThreadCount::' ~/.cargo/git/checkouts/pyrefly-b213d093d4e7816a/63cda07/pyrefly/lib/test -g '*.rs' | head`.
   - Discriminating outcome: upstream tests run multi-threaded (`TEST_THREAD_COUNT = NumThreads(3)`) against the same expectations as single-threaded runs, which supports output-neutrality. Any test pinning `Inline` for determinism would counsel caution.
