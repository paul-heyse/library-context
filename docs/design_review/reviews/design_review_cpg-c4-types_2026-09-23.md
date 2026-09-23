# Design review: CPG slice C4, the `types` family (compact)

**Date:** 2026-09-23 · **Depth:** compact · **Mode:** code plus DESIGN.md, at commit `3f0f385`
(read with `git show 3f0f385:<path>` and `git diff 3f0f385~1 3f0f385`), built and tested at
`469d955`. That commit carries the C3 review's fixes, and it leaves the C4 surface unchanged:
`types.rs` is identical, and `git diff 3f0f385 469d955` touches no C4 table, rule, derivation or DESIGN row. The slice adds a fact family,
an extractor surface, a derivation and five edge kinds, so ADR-0001 owes a `compact` review.
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`3f0f385`
**Prior reviews:** `design_review_adr-0014-cpg-graph-catalog_2026-09-22.md`,
`design_review_cpg-c2-syntax_2026-09-23.md` and `design_review_cpg-c3-lexical_2026-09-23.md`,
with their Dispositions. Only the C4 surface is in scope for findings. Findings here are F1–F7,
and observations are O1–O6.

## 1. Decision and scope

**Target.**
- `cpg-extract`:
  - `types.rs` (new, 1,005 lines, read in full);
  - the C4 diffs of `lib.rs`, `walk.rs` (`argument_values`), `pysa_map.rs` (`class_pair`),
    `facts.rs` and `config.rs`;
  - the C3-fix uses of the fork accessors in `lib.rs` (`get_wildcard`), read for the fork
    envelope only.
- `cpg-schema`:
  - `tables.rs` (`type_terms`, `type_term_args`, `type_observations`, `record_fields`);
  - `derived.rs` (`type_class_targets`, compared with `ancestry_targets` and `call_targets`);
  - `graph.rs` (the node kinds `type` and `field`, the five edge kinds, node columns and rules);
  - `codebook.rs` (the four new codebooks);
  - `id.rs` (`field`).
- Tests: `cpg-core/tests/syntax.rs` `types_keep_structure_binders_and_record_fields` and its
  snapshot, `fixtures/python/type_shapes/`, and the udf recipe snapshot.
- DESIGN: §B8, §3.2 (the `types` row), §3.4.1, §3.5, §3.5.1, §3.8 (the C4 row, rules and
  Measured block), §4.2.6 and §13. Also `docs/pins.md` (the Pyrefly row), ADR-0012's envelope and
  `scripts/check_pyrefly_fork.py`.
- Pyrefly's pinned source (`~/.cargo/git/checkouts/pyrefly-*/6a93da3/`):
  - `Type`, `Quantified`, `QuantifiedIdentity` and `Lit`;
  - `binding_to_type_return_type` and `return_type_from_annotation` (`alt/solve.rs`);
  - `KeyAnnotation`;
  - `get_class_field_from_current_class_only`;
  - `Transaction::get_wildcard`, `get_module` and `lookup_export`.

**Observable outcome claimed.** Pyrefly's native types become graph facts:
- `type_terms`: one row per distinct term, under a Merkle id;
- `type_term_args`: each child at its role;
- `type_observations`: "each parameter's type, each `def`'s return type (`Key::ReturnType`), each
  call's result, each argument's value and each `raise`'s exception", with `declared` meaning
  "Pyrefly's reading of the annotation as written, never a computed type relabelled"
  (`tables.rs`; DESIGN §3.5.1 L633-637);
- `record_fields`: "the fields a class declares itself".

Also claimed:
- type variables keep "Pyrefly's own identity … and its binder", so two unrelated `T`s are two
  terms;
- `type_class_targets` maps each class-bearing term to a node "or a reason".

Named consumers: Pass C type compatibility, FCA parameter, return and raised types, controls from
record fields, and a later shared-parameter-type community layer.

**Supported scope and non-goals.** Contextual roles (`expected`, `narrowed`, …) are deferred
(§3.5). Located and narrowed types are deferred (§13). Solver-internal variants are `other`
(`display_only`).

### Method and coverage

**Checks run in this session** (2026-09-23):

| Check | Command | Outcome | Observation |
|---|---|---|---|
| Rust tests at `469d955` | `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass --offline` in a detached worktree of `469d955`, with its own `CARGO_TARGET_DIR` | passed | 74/74; 1 slow: `every_rule_kind_rejects_its_violation`, 210.5 s |
| Lints | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --quiet --offline -- -D warnings` | passed | Same worktree, after the probe file was removed |
| Python and repo checks | `uv run pytest -q`, `uv run pyrefly check --summary=none`, `uv run ruff check --quiet`, `uv run ruff format --check`, `ast-grep scan`, `ast-grep test --skip-snapshot-tests`, `uv run python scripts/adr.py lint`, a fixture `ast.parse` | passed | pytest 21, rule tests 4/4, 14 ADRs, 33 fixture files. These are `just check`'s steps run one by one, not the recipe |
| `lint-agents` | `uv run python scripts/check_agents.py` | blocked in the worktree; passed in the main tree | A worktree lacks the gitignored installed skills. The main tree's agent files and scripts are unchanged from the commit |
| `just gold` | `uv run python scripts/check_gold.py` | passed (main tree) | The main tree has an uncommitted `[tool.lctx.source]` edit to `libraries/fastmcp/pyproject.toml` (C5 work) that leaves every version unchanged |
| `just deps` | `uv run python scripts/check_family.py Cargo.lock`; `cargo deny --log-level error check bans sources licenses`; `uv run python scripts/check_pyrefly_fork.py` | passed | The fork check reads `6a93da34 locked, named by the driver and pins, = tag + patch; every environment read classified` |
| Fork branches | `git ls-remote https://github.com/paul-heyse/pyrefly.git refs/heads/lctx/1.3.1-r2 refs/heads/lctx/1.3.1` | passed | `6a93da34` and `b9f28575`, as pins.md states |
| Patch size | `git diff --numstat 3e3177d0 6a93da34` in cargo's checkout | ran | +30/−12 in 7 files |
| Pilot | `target/release/lctx query --store build/store --snapshot <hex> …` on `076151d4…` (`469d955`) and `da2ffee4…` (the `3f0f385` pilot) | ran | Not rebuilt. `just pilot`: not_run. Both snapshots give the same C4 counts |
| Probes | a scratch test placed only in the worktree (deleted before the lints above), plus a scratch fixture under the session scratchpad | ran | P1–P8 below. Nothing was added to the repository |

A `just test-all` started in the main tree was stopped and is not counted: the author's
uncommitted C5 edits were landing in that tree while it ran.

**Probes on the pilot** (snapshot `076151d4…`; `da2ffee4…` agrees on every count below):
- **P1 coverage of subjects.**
  - Observed: all 6,201 `def` parameters, all 2,449 `def`s (return), and all 734 `raise`
    statements with an exception.
  - Not observed: 98 of 13,969 calls outside annotations, and 146 of 19,154 of their arguments.
    Neither has a `types` boundary: there are 0 in the snapshot.
  - Coverage is `complete_under_stated_model` for all 275 modules.
  - A text classification of the 98 calls (approximate; it slips in non-ASCII files): about 44
    `TypeVar(...)` declarations, about 26 calls in branches Pyrefly decides unreachable for the
    context (`sys.platform == "win32"`, `os.name == "nt"`), and about 26 calls inside lambda
    bodies (`lambda r: str(r.uri)`, `lambda context: self.list_tools(...)`) (F4).
- **P2 async returns.** 792 `async def`s with a return annotation have a `declared` return of
  `Coroutine[Unknown, Unknown, X]`. That is 792 of the 2,284 declared returns (F2).
- **P3 type variables.** 102 `type_var` terms, 90 with a binder (the §3.8 counts). With the 2
  value forms, 104 terms carry a variable, over 95 distinct variables.
  - 7 of the 12 without a binder are anchored in **release** modules. Examples: `F` in
    `fastmcp.apps.app:6381-6382`, `fastmcp.server.server:78410-78411`.
  - Each of those 7 is a second term for a variable that also has a term with a binder (F3).
  - Only 5 are dependency-anchored (`cyclopts`, `functools`, 3 × `pydantic`).
- **P4 literals.** 17 enum-member literal terms (`Literal[SpanKind.CLIENT]`, `Literal[MCPType.TOOL]`,
  …). None has a class or a `type_class` edge (F5).
- **P5 record fields.** 773 rows. By source reading, 162 of the 164 pydantic rows with
  `has_default = false` or an alias are right:
  - bare annotations;
  - `Field(alias=…)` or `Field(description=…)` with no default;
  - `Annotated[str, Field(min_length=1)]`.

  The other 2 are `PrefabAppConfig.resource_uri` and `.csp` (`fastmcp/apps/config.py`).
  `PrefabAppConfig` inherits both from `AppConfig`, where they default to `None`. Its rows read
  `has_default = false`, with the span of `self.resource_uri = …` (L144) and `self.csp = …`
  (L167) inside a method (F6).

  25 of FastMCP's 43 `TypeVar`s have a `bound=`, a `default=` or constraints (grep of the
  release; F5).

**Scratch probes** (a 3-module fixture, compiled and read through the published tables):
- **P6a** `async def fetch(n: int) -> str` gives a declared return of
  `Coroutine[Unknown, Unknown, str]` (F2).
- **P6b** `pr.a.ident`'s `T` (`pr.a:198-199#0/legacy`) is two terms:
  - `ce4e5cfb…`, with binder `pr.a.ident`, in `ident`'s own observations;
  - `02033aff…`, with no binder, inside `[T](x: T) -> T`, the argument value of `take(a.ident)`
    in `pr.b` (F3).
- **P6c** `B = TypeVar("B", bound=int)`: the term `B` has no children, and `int` appears nowhere
  in its structure (F5).
- **P6d** `Literal[Color.RED]` over `pr.a.Color` and over the unrelated `pr.b.Color` is **one**
  term (`6f2110ca…`), with no class and no `type_class` edge (F5).
- **P6e** `@dataclass class Sub(Base)`, whose `__post_init__` assigns `self.x = 5`, gets a row
  `Sub.x`, ordinal 0, `has_default = false`, spanning the `self.x` target. `Base.x = 0` has
  `has_default = true` (F6).
- **P6f** Four calls and three arguments have no observation and no boundary:
  - `TypeVar("T")` and `TypeVar("B", bound=int)`, and their name arguments;
  - `str(v)` inside `sorted(xs, key=lambda v: str(v))`, although its argument `v` is observed;
  - `os.startfile("x")` under `sys.platform == "win32"`, and its argument.

  Coverage for `types` is `complete_under_stated_model` for all 3 modules (F4). The committed
  `type_shapes` fixture has the same gap: its `TypeVar("T")` and `ParamSpec("P")` have no
  `call_result` rows in the snapshot.
- **P6g** `meta(x: Annotated[int, "unit:ms"])`: the declared parameter type is `int`, with the
  metadata gone (O3).
- **P7 (the catch-all).** In `type_shapes`, rewrite every `type_terms.class_module` (`@…` to
  `@nowhere.py`, any other to `<name>_nowhere`), then compile. The snapshot **publishes**:
  - 20 of 20 `type_class_targets` rows read `missing_evidence`;
  - 0 `type_class` edges;
  - every rule passes (F1).
- **P8** An injected `record_fields.node_id` is rejected by `id:record_fields` alone.

**Read and not attacked:**
- publication and recovery (unchanged);
- determinism beyond the passing identity tests;
- performance beyond the author's pilot log;
- whether `Transaction::get_solutions` can be `None` for a release module. This is **asserted**:
  `Require::Everything` solves release modules (O5).
- Pydantic `alias` semantics (`validation_alias` versus `alias`), beyond reading 32 aliased rows;
- attrs (0 on the pilot).

## 2. Authority and lifecycle (compressed)

- **One producer.** The four raw tables are surface `pyrefly-types`, `native_traversal`,
  `analyzer_assertion`. The rows' authority is Pyrefly's answers, read in memory from the
  constructed transaction. `type_class_targets` is a Stage-D join, and the catalogs are generated
  from the registry (as in C1–C3).
- **Two values on these rows are ours, not Pyrefly's:**
  - **The binder.** `binder()` (`types.rs:222-238`) finds the `def`/`class` header of **the
    observing module** that contains the anchor, and the id hashes it (`types.rs:668`). The row
    carries Pyrefly's provenance (F3). Pyrefly states an owner itself for function type
    parameters (`Quantified.owner`, "e.g. `mod.func`", `quantified.rs:172-173`); the extractor
    does not read it.
  - **`type_class_targets.reason = missing_evidence`.** It is written whenever our join misses
    (`derived.rs:733-736`), so it presents our failure as the provider's statement (F1).
- **Identity.**
  - `type` is a Merkle id over kind, **display**, detail, class pair, variable, **binder**, arity
    and children (`types.rs:658-681`). There is no SQL form. It is pinned only by the snapshot
    tests.
  - `field = H(field, class, name)` (`id.rs`), recomputed by `id:record_fields`, which P8
    exercises.
- **The fork.**
  - `6a93da34` is tag 1.3.1 plus a +30/−12 patch in 7 files: visibility, the `write_files`
    switch, `pysa_reporter()`, `ClassField::dataclass_flags_of` made `pub`, and
    `Transaction::get_wildcard`.
  - No upstream hunk changes behaviour.
  - `check_pyrefly_fork.py` verifies identity (the parent is the tag, the `patch-id` equals the
    committed patch, every environment read is classified). It does not verify the envelope (O6).

## 3–4. Contracts and derivation (merged)

| Invariant | Enforcement | Evidence |
|---|---|---|
| Every Pyrefly `Type` variant maps to a kind | an exhaustive `match` under `#![deny(clippy::wildcard_enum_match_arm)]` (`types.rs:14`, `396-650`) | **Implemented**. Clippy passed. All 54 variants (`pyrefly_types/src/types.rs:883-1035`) appear in the match |
| A term is finite | classes as pairs, recursive aliases as references, a depth cap of 32 | **Tested** (`type_shapes` `Tree`; 0 `truncated`); pilot 0 truncated |
| One structure is one term | Merkle id | **Violated** for type variables observed outside their module (F3), and for same-named enum literals (F5) |
| Children are distinguishable | the `type_arg` edge id takes the ordinal and the role discriminator (`graph.rs:882-899`) | Pilot: 6,013 arguments, `key:edges` passes |
| Declared = the annotation as written | `declared` from the header's `returns` flag (`types.rs:850`); the type from `Key::ReturnType` (`types.rs:846-849`) | **Violated** for async `def`s (F2) |
| Each in-scope subject is observed or explained | none: `if let Some(t) = get_type_trace(..)` with no `else` (`types.rs:867`, `875`, `891`) | **Violated**: P1 and P6f (F4) |
| Every class-bearing term has a node or a provider's reason | `typed:type_class_targets` (`graph.rs:1650`) and the `type_class` lineage (`graph.rs:931-932`) | **Cannot fail**: `ELSE {missing}` (`derived.rs:736`); P7 publishes (F1) |
| A record row is a field the class declares | `get_class_field_from_current_class_only` (`types.rs:951`) | **Violated** for fields a subclass assigns in a method (F6) |
| Field ids equal their SQL form | `id:record_fields` | **Tested** by P8 only (no repo case) |
| C4 lineages | generated; `has_type`, `type_arg`, `has_field` and `field_type` are the C2 `simple` edit guards (C3 O2's decision) | as decided |

**Absence.** An observation has four states: a declared type, a computed type, `Any` (`explicit`,
`implicit` or `error` in `detail`, which is good), and no row. "No row" covers each of these,
indistinguishably, under a coverage row that says complete (F4):
- a call Pyrefly deems unreachable in the context;
- a `TypeVar` declaration Pyrefly binds without a trace;
- a call inside a lambda body;
- a bare `raise` (O4).

## 5. Journey: do the named consumers get what they read?

- **Pass C type compatibility (`Optional[X]` against `X`): served for the plain case.** Argument →
  `has_type` → term → `type_arg` members → `type_class` → class node. `Optional[int]` is a union
  of `int` and `None` (the snapshot's `ts.maybe.x`). Gaps:
  - **Async producers.** The call result is `Coroutine[…, X]`, which is right for the call. But
    `await` has no observation, and the producer's own `declared` return is the coroutine
    (F2). A checker comparing the producer's declared return with the consumer's parameter
    fails on every async API unless it knows to unwrap.
  - **Generic consumers** (`def tool(...) -> Callable[[F], F]`, `F` bound to `Callable`).
    - The bound is not in the graph (F5). Guidelines L128 require a handoff candidate to satisfy
      the generic constraints.
    - When the producer is observed in another module, its `F` is a different term from the
      consumer's `F` (F3), so term-id equality misreads identical types as different.
  - **Enum-literal parameters.** `Literal[a.Color.RED]` and `Literal[b.Color.RED]` compare equal
    by id (F5).
  - **Producers inside lambdas** have no type at all (F4).
- **FCA parameter, return and raised types: served, with one distortion.** Each return type is a
  term id. Sync `-> Table` and async `-> Table` become different attributes
  (`Table` / `Coroutine[Unknown, Unknown, Table]`), so an implication such as "every reader
  returns `Table`" splits by `async` (F2). Raised types keep `raise X` (`class_object`) apart
  from `raise X()` (`class_instance`), both with a `type_class` edge. That is structure, not a
  defect. Re-raises are absent (O4).
- **Controls from record fields: served for the flags, not for constraints.**
  - Default, `init`, alias and `kw_only` are right on the fixture and on 771 of 773 pilot rows.
  - The 2 wrong rows say an inherited, defaulted field is required (F6).
  - Pydantic constraints (`gt`, `min_length`, `pattern`) are in neither the terms nor the rows
    (O3).
- **Shared-parameter-type communities: served.** The Merkle dedup makes identical parameter types
  one node. F3's split applies only to types that contain a type variable.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **fail** (narrow) | `type_class_targets.reason` presents our join's miss as `missing_evidence`, a provider statement (F1). This is the same authority confusion C3's F2 fixed for imports. The binder, our containment heuristic, is stored under Pyrefly's provenance (F3). Otherwise: one producer per table, the catalogs generated, and derivation classes consistent with the C1 precedents (`type_class` is analyzer-through-join, like `base_class`) | F1, F3 |
| **G2** Semantic fidelity | **fail** | Each finding is **Tested** by a probe: `declared` relabels a computed type for 792 async returns (F2); one variable is two terms (F3); unobserved subjects are indistinguishable from each other and from complete (F4); enum classes and type-variable bounds are dropped under `native_structural` (F5); inherited fields read as declared, with the wrong default (F6) | F2–F6 |
| **G3** Validity | **fail** (narrow) | `typed:type_class_targets` and the `type_class` lineage cannot fail: P7 publishes with every class reference corrupted. No C4 rule has an injected-violation case in `compile.rs`. `id:record_fields` works (P8) | F1 |
| **G4** Hidden behaviour | **pass** | The pass reads the transaction's answers, solutions and class metadata in memory, and the walker's rows. No new ambient read; the config is unchanged. The fork's `get_wildcard` demands a module's exports inside the same constructed transaction. That is lazy computation, not an ambient input (O6) | — |
| **G5** Consistency and recovery | **pass** | Publication unchanged. The five C4 tables go through the same attempt, and P8's injected violation returned `Invalid` from `compile`. Whether anything was published afterwards was not re-read; `compile.rs` already asserts that for its cases. Not attacked beyond that | — |
| **G6** Transformation and reuse | **pass** | `EXTRACTOR_OUTPUT_VERSION` 9 → 10, `PYREFLY_REV` and the patch sha256 move `producer_id` and so `run_id`. Term ids are deterministic for the same inputs (passing identity tests). Their producer scope is undocumented (F7d) but causes no invalid reuse, since `run_id` changes with the producer | — |
| **G7** Truthful capability claims | **fail** | Several claims are contradicted: "declared … never a computed type relabelled" (F2); "each call's result, each argument's value" under complete coverage (F4); "the 12 without are anchored in dependency modules" (§3.8 L821; 7 are release-anchored, F3); `native_structural` on terms that keep the enum class as display text (F5); "record fields a class declares itself" (F6). Other lines are stale (F7) | F2–F7 |

## 7. Findings

### 7.1 Findings

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | `type_class_targets` reintroduces the catch-all reason, the fourth instance of the shape ADR-0014's review F1, C2's F2 and C3's F2 removed. Our join's miss is written as `missing_evidence`, so the typed rule and the lineage cannot fail | DM-07, DM-08, DM-02, DM-60 · G1, G3; guidelines §2 (unknown targets explicit) | `derived.rs:733-736`: `WHEN m.class_key IS NOT NULL THEN m.reason ELSE {missing} END`. The sibling `ancestry_targets`, with the same three-way join, ends `… THEN t.reason END` with no `ELSE` (`derived.rs:694`). The rule it breaks: ADR-0014 L117-118 ("No derivation has a catch-all reason"), DESIGN §3.2 L434-436, §8 L1438. `typed:type_class_targets` is `class_node_id IS NULL AND reason IS NULL` (`graph.rs:1650-1654`). The lineage counts any row with a reason as explained (`graph.rs:931-932`). **P7:** with every `class_module` corrupted, the snapshot publishes (20/20 `missing_evidence`, 0 edges). The table's doc comment (`derived.rs:712-714`) repeats `call_targets`' stale comment (`derived.rs:625-626`), which still describes the catch-all ADR-0014 F1 removed from its SQL (`function_target_reason`, `derived.rs:604-610`) | A regression in `class_pair`, in `ModuleRefs.referenced` or in `context.rs`'s definition filter, or a Pyrefly bump that changes `ClassId` numbering between the types pass and the Pysa definitions, publishes every class-bearing term as "the provider has no evidence". The graph then loses every `type_class` edge while every rule passes. Pass C and FCA read class identity through exactly those edges | Drop the `ELSE`. A miss then keeps a null reason and fails `typed:type_class_targets`, as in `ancestry_targets`. Correct both doc comments. Two lines | **test:** P7 as a case in `every_rule_kind_rejects_its_violation` (`type_shapes` or the existing base fixture), expecting `typed:type_class_targets`. Add P8 as the `id:record_fields` case |
| **F2** | For an `async def`, the `declared` return is Pyrefly's **computed** return, the coroutine wrapping the annotation, with two implicit `Any`s the author never wrote | DM-06, DM-24, DM-59 · G2, G7 | `types.rs:846-851` reads `Key::ReturnType` and sets `declared` from the header's `returns` flag. Pyrefly documents `Key::ReturnType` as "the actual type of the return for a function" (`binding/binding.rs:930-931`). `return_type_from_annotation` wraps an async non-generator in `coroutine(any_implicit, any_implicit, annotated_ty)` (`alt/solve.rs:5889-5905`). The claims contradicted are §3.5.1 L633-637 ("its type is Pyrefly's reading of that annotation") and the `type_observations` doc ("never a computed type relabelled"). **P2:** 792 of 2,284 declared returns on the pilot. **P6a:** `-> str` gives `Coroutine[Unknown, Unknown, str]`. Pyrefly keys the annotation itself: `KeyAnnotation::ReturnAnnotation(ShortIdentifier)` (`binding/binding.rs:1661-1662`) | FCA's return-type attribute splits sync and async APIs that return the same type. A handoff checker comparing a declared producer return with a consumer parameter rejects every async producer. The `Unknown`s read as implicit `Any`, the marker for "unannotated", on fully annotated APIs. The fixture has no `async def`, so nothing caught it | When the `def` has a return annotation, observe `KeyAnnotation::ReturnAnnotation`'s type (the annotation as written; **Interface-checked**, key read, access path not built) with `declared = true`. Otherwise observe `Key::ReturnType` with `declared = false`. The call site's `call_result` keeps the coroutine, which is right for the call | **test:** an `async def f(n: int) -> str` in `type_shapes`, whose snapshot row reads `declared \| str` |
| **F3** | A type variable's term depends on the module that observes it. The binder is resolved only against the observing module's headers, and the Merkle id hashes it. So one Pyrefly variable (`QuantifiedIdentity`, globally unique) becomes two terms when observed outside its own module, and so does every term containing it. The binder is our own containment rule, stored under Pyrefly's provenance | DM-11, DM-15, DM-13, DM-59 · G1, G2, G7; guidelines §2 (extracted and heuristic distinguishable) | `types.rs:230`: `if !anchored \|\| identity.module.as_str() != self.m.module_name { return None; }`. `types.rs:233-237` searches `self.headers`, which holds only this module's headers. `types.rs:668` puts `.opt_id(t.binder)` in the id, although `variable` (`types.rs:243-250`) already carries the full identity. **P3:** 7 release-anchored variables have a binderless twin on the pilot (both snapshots). **P6b:** `pr.a:198-199#0/legacy` gives terms `ce4e5cfb…` (binder `pr.a.ident`) and `02033aff…` (none). §3.8 L821's "the 12 without are anchored in dependency modules" was already false at `da2ffee4…` (7 of 12 in `fastmcp.*`). `type_shapes` never observes a generic from another module | Every call site outside the defining module of `FastMCP.tool(...)`, `resource(...)` or `prompt(...)` (the `Callable[[F], F]` decorators) gets a structurally different term from the declaration's own. Joins by term id between a declaration and its uses then miss, and a shared-type community splits one type in two. A consumer reading `binder_node_id IS NULL` as "anchored outside the release" is wrong 7 times in 12 | Take the variable out of the observing module's hands. Either (a) resolve binders over all release modules' headers in a pre-pass, or (b) derive the binder in Stage D (the innermost declaration whose span holds the anchor), or read Pyrefly's own `Quantified.owner` for function parameters. In both cases drop `binder` from the id, since `variable` already identifies the variable. (b) also labels the binder a joined fact. See §8 | **test:** a second `type_shapes` module that passes `ts.ident` as a value. Assert that `SELECT variable, kind FROM type_terms WHERE variable IS NOT NULL GROUP BY 1, 2 HAVING count(*) > 1` is empty, and that the binder is `ts.ident` |
| **F4** | Subjects the family claims are silently missing. A call or argument with no Pyrefly trace gets no observation and no boundary, and coverage says `complete_under_stated_model` | DM-08, DM-43, DM-30 · G2, G7; guidelines §2 ("record their omission and its effect on completeness"), §7 | `types.rs:867`, `875`, `891`: `if let Some(t) = ctx.answers.get_type_trace(..)` with no `else`. The claim contradicted is §3.2 L449 ("each call's result, each argument's value", excluding only calls in annotations). **P1:** 98 calls and 146 arguments on the pilot: `TypeVar` declarations, platform branches Pyrefly skips, and lambda bodies. 0 `types` boundaries; 275 of 275 modules complete. **P6f:** 4 calls and 3 arguments in a 50-line fixture. `type_shapes`' own `TypeVar("T")`/`ParamSpec("P")` are unobserved and unasserted | Pass C cannot tell "Pyrefly skipped this code for the context" from "Pyrefly has no type here". Calls inside `lambda`s passed as callbacks (about 26 on the pilot, e.g. `lambda context: self.list_tools(...)`) are real, reachable handoffs with no type. A brief built on "no observation" states nothing wrong, but the coverage row promises there was nothing to state | Emit a `types` boundary per unobserved in-scope subject, with a provider-backed reason: `missing_evidence` ("Pyrefly records no type at this span" is its own answer here). Where the range sits in a branch Pyrefly's bindings mark unreachable, use `unreachable_in_context`. Mark the module's coverage `partial`. Or declare `TypeVar` declarations outside the model and say so in §3.2. About 20 lines | **test:** `type_shapes` asserts one `types` boundary per unobserved call (`TypeVar("T")`, `ParamSpec("P")`), plus a lambda-body call and a `sys.platform == "win32"` call. A generated rule "every call outside annotations has an observation or a `types` boundary" would make it permanent |
| **F5** | Structure Pyrefly provides is reduced to display text or dropped, while the rows say `native_structural`. (a) An enum literal's class (`LitEnum.class: ClassType`) survives only in the display string, so same-named enums in different modules share one term and get no `type_class` edge. (b) A type variable's bound, constraints and default (`Quantified.restriction`, `.default`) are not recorded | DM-42, DM-10, DM-06 · G2; guidelines §6 L128 (handoff candidates satisfy generic constraints; "matching type names" cannot establish interoperability); §3.5 "a row's fidelity is that of its weakest semantic field" | (a) `types.rs:399`: `Type::Literal(_) => Term::new(K::Literal).detail(ty.to_string())`, with no class. `Lit::Enum(Box<LitEnum>)` holds `class: ClassType` (`pyrefly_types/src/literal.rs:53-59`, `102-104`). The fidelity match marks `Literal` `NativeStructural` (`types.rs:698`). **P4:** 17 enum-literal terms, 0 class edges. **P6d:** one term for `pr.a.Color.RED` and `pr.b.Color.RED`. (b) `types.rs:255-262` keeps name, kind and identity. `Quantified` has `pub default` and `pub restriction` (`quantified.rs:160-173`). **P6c:** `B` (bound `int`) has no children. **P5:** 25 of 43 FastMCP `TypeVar`s carry a bound, constraints or a default | (a) A parameter typed `Literal[a.Color.RED]` is "the same type" as one typed `Literal[b.Color.RED]`. The enum class, which a brief's allowed-values control needs, is unreachable from the graph. (b) Pass C cannot check `ClientTransportT: ClientTransport` or `F: Callable[..., Any]`. It must accept any argument for a type-variable parameter, which is exactly the "type compatibility alone" over-approximation §9.3 already distrusts | (a) For `Lit::Enum`, set the class pair from `class.class_object()` and the member as `detail`; the `type_class` edge follows. (b) Emit the bound, each constraint and the default as children under appended `type_arg_role` codes (`bound`, `constraint`, `default`). Codebooks are append-only, so this needs no ADR. About 25 lines | **test:** `type_shapes` gains `TypeVar("B", bound=int)` and two same-named enums in two modules, asserting two terms, two class edges and the `bound` child |
| **F6** | `record_fields` treats an inherited field that a subclass assigns in a method as a field the subclass declares. The flags then describe the assignment, not the record | DM-06, DM-24 · G2, G7 | `types.rs:951` takes any field `get_class_field_from_current_class_only` returns. That includes Pyrefly's `ClassFieldDefinition::DefinedInMethod` (`binding/binding.rs:3246`), because `class_fields.contains(name)` covers method-assigned attributes (`report/pysa/class.rs:290-297`). `types.rs:960-964` then reads `dataclass_flags_of`, which gives no default for a method assignment. **P5:** `PrefabAppConfig.resource_uri` and `.csp` (`has_default = false`; their base `AppConfig` defaults both to `None`). **P6e:** `Sub.x` reads `has_default = false` against the base's `x: int = 0` | A control derived from record fields says `PrefabAppConfig(resource_uri=…)` is required, and `has_field` gives the class a field it does not declare. Pydantic validators and `__post_init__`s that normalize inherited fields are common, so the error rate grows with the library | Skip fields whose class-field definition is `DefinedInMethod`, or whose `field_decl_range` is not a direct statement of the class body. They stay the base's rows. A few lines | **test:** P6e's `Base`/`Sub` pair in `type_shapes`, where the snapshot has no `Sub.x` row |
| **F7** | DESIGN and code claims are out of step with C4 | DM-59, DM-55 · G7 | (a) §3.8 L821 "the 12 without are anchored in dependency modules" (F3). (b) §3.5.1 L633-637 and the `type_observations` doc (F2). (c) §3.2 L449 "each call's result, each argument's value" under complete coverage (F4). (d) §3.4.1 L520 says type-term ids are "stable across snapshots and runs for the same inputs". It says nothing of their producer scope: they hash Pyrefly's display text, Pysa `ClassId` integers and anchor byte offsets, so a Pyrefly bump or an earlier edit in the module renames them, just as the same row notes for syntax and external-symbol ids. (e) §B8 L255-258 and pins.md call `Transaction::get_wildcard` a "borrow-only accessor". It calls `get_module` (which may create the module's entry) and `lookup_export` (which demands `Step::Exports`) (`state/state.rs:1753-1765`, `1879-1882`), the same kind of query as the already-exposed `get_exports_data`. (f) `derived.rs:625-626` (F1) | A reader takes the binder column's nulls as dependency anchors, and takes `declared` returns as annotations. A future pin move judges a new accessor against a "borrow-only" rule that the last accessor did not meet | Correct the lines. Define §4.2.6's allowed class as "visibility, and accessors composing existing upstream queries". Relabel (c) until F4 lands | prose (no mechanical oracle for spine wording), plus F1–F4's tests |

**Observations** (moving none of the three severity classes today; each names where it would
land):

| # | Observation | Evidence | Oracle |
|---|---|---|---|
| O1 | The extractor-channel reasons for our own lookup misses are chosen by us: "a def Pyrefly types has no declaration" and "parameter … has no parameter node" read `missing_evidence`, and any class-span miss reads `no_source_declaration` ("a synthesized class"), whatever the cause. By C2's F2 rule, a span with no node is our failure, never a reason. The module's coverage goes `partial`, so the case is visible. Pilot 0 | `types.rs:824-832`, `839-843`, `935-946` | **test:** a functional `NamedTuple("P", [...])` record confirms `no_source_declaration` is Pyrefly's; route any other miss to an error |
| O2 | Nominal identity is dropped in other kinds. A `def`'s term keeps its name, not its `FuncId` (module, class), so same-named, same-signature functions share a term. `SuperInstance` (125 on the pilot) and `KwCall` (1; `type_shapes`' `dataclass_transform()`) are `other`/`display_only` though they carry a class. `Intersect`'s fallback is dropped (3 terms) | `types.rs:406-411`, `441-447`, `646-647` | none until a consumer reads callable identity from types (Pass A and C read call edges) |
| O3 | "Controls from record fields" gets the dataclass-style flags, but not pydantic constraints (`Field(gt=0)`, `Annotated[int, Field(min_length=1)]`). Pyrefly strips `Annotated` when it resolves an annotation (P6g), and `Type::Annotated`'s metadata is dropped where it survives (1 term) | `types.rs:569-573`; P5, P6g | prose in §3.2 (constraints reach Pass B through the `Field(...)` call's `argument_value`s), until Pass B's control recognizer lands |
| O4 | A bare `raise` (66 on the pilot) has no `raised` observation, so FCA's raised types miss re-raises. This is undeclared | `types.rs:886-894` | prose in §3.2 |
| O5 | When `get_solutions` is `None`, the pass returns with no record fields and no boundary. Unreachable for release modules under `Require::Everything` (asserted) | `types.rs:897-899` | a `types` boundary (`native_unavailable`) in that branch |
| O6 | The fork change is within ADR-0012's envelope in size (42 of ~60 lines) and changes no upstream behaviour. But `check_pyrefly_fork.py` checks identity, not the envelope. ADR-0012's decision (immutable) still names `b9f28575` on `lctx/1.3.1`; §B8 and pins.md carry the move, which is enough | patch numstat; `check_pyrefly_fork.py` | `just deps`: count the patch's changed lines and fail above 60, if the patch grows again (Deferred) |

**Checked and clean:**
- The `Type` match is exhaustive, with no wildcard arm and `deny` on the lint. All 54 upstream
  variants appear in the match (read).
- `type_arg` ids carry the ordinal and the role discriminator, so `tuple[int, int]` and
  `Callable[[T], T]` give distinct edges.
- The recursive alias is one finite term (snapshot).
- Two unrelated `T`s in one module are two terms (the test's assertion).
- `argument_values` records the value expression (`*xs` → `xs`, `k=v` → `v`) (`walk.rs:500-538`).
- `class_pair` registers dependency classes in `ModuleRefs.referenced` before `context.rs` runs, so
  all 2,416 pilot class-bearing terms map to a node (the absence of F1's catch-all firing, not
  its falsification).
- The record flags on the fixture are right: `default_factory`, `kw_only`, `init=False`,
  `NotRequired` and a `NamedTuple` default.
- Every parameter, `def` and exception-bearing `raise` on the pilot is observed.
- Determinism: sorted batches; `emitted` only dedupes.
- The fork: the remote branches, the patch sha256 and `patch-id`, and the environment
  classification.

### 7.2 Applicability and verdicts

- **Bore on this scope:**
  - Group 1 (authority: F1, F3's provenance);
  - Group 2 (semantic types and absence: F2, F4, F5, F6);
  - Group 3 (identity: F3);
  - Group 9 (the provider boundary with Pyrefly: F4, F5, O6);
  - Groups 10–12 (lineage, regression controls, claims: F1, F7).
- **Did not bear:**
  - Group 4: the registry entries follow C1's shape, with no new declaration surface.
  - Group 6: no new effects. G4 passes.
  - Group 7: the dependency change (the fork revision) moves `producer_id`, which was checked
    clean.
  - Group 8: the pass takes 0.13 s (**Measured**, author's log for `da2ffee4…` and `076151d4…`;
    read, not rerun). No claim here rests on performance.

| Verdict | Principles |
|---|---|
| Satisfied | DM-09 (endpoint kinds and node columns generated), DM-40 (determinism, **Tested** by the identity tests), DM-51 (the commit is labelled a schema migration; codebooks appended), DM-52 (rules and catalogs generated) |
| Violated | DM-07 and DM-02 (F1); DM-08 (F1, F4); DM-11 and DM-15 (F3); DM-06 and DM-24 (F2, F6); DM-42 and DM-10 (F5); DM-43 (F4); DM-59 (F2–F7); DM-60 (F1: no C4 rule has an injected case) |
| Unresolved | DM-13: whether the binder is an analyzer fact or our joined fact is not decided (F3 and §10) |

**Guidelines MUSTs (ADDENDUM §5), for C4:**
- **§2 edge identity, typed endpoints, evidence and snapshot scope: met.** `type_arg` is parallel
  with role and ordinal discriminators; `has_type` gives one per subject by construction.
- **§2 isolates: met** (the `type_terms` and `record_fields` existence sources).
- **§2 distinguishable derivations: partly.** The edge classes are consistent, but the binder is
  our containment rule on a `pyrefly-types` row (F3).
- **§2 unknowns explicit: partly** (F1 invents a reason; F4 records no omission).
- **§6 L128, handoff candidates satisfy generic constraints: not supplied by the CPG** (F5b).
- **§7 partial ≠ complete: partly** (F4's coverage rows).
- **§12 known-answer shapes: partly.** `type_shapes` has no `async def`, enum literal, bounded
  variable, cross-module generic use, method-assigned inherited field, lambda-body call, or
  pydantic or attrs record (its environment is empty). It does contain two unobserved calls, but
  asserts nothing about them.

## 8. Alternatives (compressed)

| Alternative | Duplication and extension locality | Risks | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| Current: Merkle id over display, detail, class, variable **and our binder**; the binder resolved per observing module; observations by trace with no `else`; `type_class_targets` with a catch-all | The binder rule is ours and sits inside the provider's identity. The display text and the structure both enter the id | F1–F6 | 1,005 + ~300 lines | **Measured** (author's log, 2026-09-23): types 0.13 s; compile 18.4 s at 3.81 GB | Selected by the author |
| Current with F1–F6 corrected in place | Same structure | O-items remain | about +60 lines and 6 fixture shapes (**Proposed**) | same order (**Proposed**) | Minimum for Accept |
| **Simpler viable: identity from Pyrefly alone; our joins as derivations** | The term id hashes only Pyrefly's structure and identities: kind, detail, class pair (with the enum class for `Lit::Enum`), `QuantifiedIdentity`, and children with their roles. It drops the display (already implied by detail and children) and the binder. The binder becomes a Stage-D derivation: the innermost `declarations` row whose span holds the anchor, or Pyrefly's `Quantified.owner`. That makes it a `joined` fact, provenance-labelled as ours. This removes the `Headers` visitor and `binder()` (about −40 lines) and F3's cause. Ids stop depending on Pyrefly's `Display`, so they become stable across a Pyrefly bump that only changes rendering (F7d) | A structural equal with a different display (none found by reading the kinds: `detail` carries every leaf's text) would have to share an id. The existing fact dedup would surface it as a collision rather than a silent merge. The anchor needs columns (module, start, end) for the SQL join | about −40 lines of ours, +1 derivation of ~20 SQL lines (**Proposed**) | negligible (**Proposed**, unmeasured) | Recommended. F3 and F5a converge on it, and it keeps our own inference out of the provider's identity, as §B6 and guidelines §2 ask |

**What stays ordinary code:** the exhaustive `Type` walk and the record-flag extraction. No
declarative type-mapping layer is warranted. Each fix is a local branch or a column.

## 9. Top verification gaps

| Claim or risk | Label now | Check | Expected result | Gap |
|---|---|---|---|---|
| A class reference we cannot map fails validation | **Implemented**, unfalsifiable (P7) | the P7 mutation case | fails `typed:type_class_targets` | F1 |
| `declared` = the annotation as written | **Implemented**; contradicted for async (P2, P6a) | an `async def` in `type_shapes` | `declared \| str` | F2 |
| One variable, one term | **Tested** within one module only | the cross-module case | one term per `(variable, kind)` | F3 |
| Every in-scope subject is observed or explained | not claimed as a rule; coverage says complete (P1) | boundary rows, plus the rule | 98 + 146 boundaries on the pilot | F4 |
| Terms keep enum classes and variable bounds | **Implemented** as display only / not at all | fixture shapes | class edge; `bound` child | F5 |
| Record rows are the class's own fields | **Tested** on body-declared fields only | the `Base`/`Sub` pair | no `Sub.x` | F6 |

## 10. Exceptions and unresolved decisions

No SHOULD-level exception is requested. Decisions the author has to make:
- **F2:** read `KeyAnnotation::ReturnAnnotation` for declared returns (recommended), or keep
  `Key::ReturnType` and mark async returns `declared = false`. Either way, §3.5.1 has to state
  which.
- **F3 and §8:** the binder as an extractor column resolved over all release modules, or a Stage-D
  derivation (recommended). Whether the binder and the display stay in the term id is a
  migration of every term id (DM-51), so it is best decided now, before C5 adds usage runs that
  observe the same types.
- **F4:** a boundary per unobserved subject, or a declared exclusion (for `TypeVar` declarations)
  plus boundaries for the rest.
- **F5b:** which `Quantified` parts become children (`bound`, `constraint`, `default`), as
  appended `type_arg_role` codes.

## 11. Decision

**Decision: Revise** (small surface).
**Reason:** The family is well shaped:
- an exhaustive map of Pyrefly's `Type`;
- finite Merkle terms whose children keep their roles;
- observations on the walker's own nodes;
- record flags that are right on the fixture and on 771 of 773 pilot rows;
- a fork change that stays inside ADR-0012's envelope.

The pilot's parameters, returns and raises are all observed. But C4 reintroduces the catch-all
reason ADR-0014 forbids, for the fourth time (F1). It publishes a computed type as `declared` on
35% of the pilot's declared returns (F2). It lets one type variable become two terms depending on
where it is observed (F3). It records no omission for 244 unobserved calls and arguments under a
"complete" coverage row (F4). Each fix is local, and each gets costlier once C5 compiles
examples and tests through the same pass. That holds most of all for F3, whose fix changes term
ids.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: drop the `ELSE` in `type_class_targets`; fix both doc comments | DM-07, DM-08, DM-02 | P7 rejected | test: the mutation case, plus P8 for `id:record_fields` |
| 2 | F2: declared returns from `KeyAnnotation::ReturnAnnotation` | DM-06, DM-24 | pilot: 0 declared returns of `Coroutine[Unknown, Unknown, …]` | test: an `async def` fixture row |
| 3 | F3: the binder resolved independently of the observing module and kept out of the id (§8) | DM-11, DM-15, DM-13 | pilot: 0 variables with two terms; 12 binderless all dependency-anchored, or fewer | test: the cross-module case, and the `GROUP BY variable` assertion |
| 4 | F4: boundaries for unobserved subjects; `partial` coverage | DM-08, DM-43 | pilot: 98 + 146 boundaries | test: fixture boundaries, plus the generated rule |
| 5 | F5: the enum class pair; bound, constraint and default children | DM-42, DM-10 | P6c, P6d | test: fixture shapes |
| 6 | F6: skip method-assigned fields | DM-06, DM-24 | pilot: no `PrefabAppConfig.resource_uri`/`.csp` rows | test: the `Base`/`Sub` pair |
| 7 | F7: correct the DESIGN lines and the accessor wording | DM-59 | — | prose |

### Deferred

| Item | Why not now | Trigger that reopens it |
|---|---|---|
| O1: self-chosen boundary reasons for our lookup misses | Pilot 0; visible as `partial` | A library with functional `NamedTuple`/`TypedDict` records, or any `types` boundary on a pilot |
| O2: function identity, `super()` and `KwCall` classes, the `Intersect` fallback | No consumer reads them from types | Pass C or FCA reads a callable's or a super-instance's type |
| O3: pydantic constraints | Pass B's control recognizer has not landed | That recognizer's slice |
| O4: re-raises | FCA has not landed | FCA's raised-type attribute (increment 2) |
| O5: `get_solutions` returning `None` | Asserted unreachable | Any `types` coverage row that is not complete on a pilot |
| O6: a mechanical envelope check | 42 of ~60 lines; one more accessor fits | The next patch change |

## Disposition (author, 2026-09-23)

The author took the simpler alternative and fixed F1–F7 in one commit after C5a (`3e223f9`),
which had not touched the C4 surface.

| # | Outcome | What changed | Oracle |
|---|---|---|---|
| Simpler alternative | taken | A term's id hashes Pyrefly's structure and identities alone (kind, detail, class pair, children with roles); the display is a label, hashed only for `other`/`truncated`. A variable's id is its `QuantifiedIdentity`, whatever module observes it. The binder is a Stage-D derivation, `type_binders`, from Pyrefly's anchor (new `anchor_*` columns): the innermost release declaration, type-alias or assignment statement holding it; `scope_boundary` for an anchor outside the release; null with no reason is our failure. The `Headers` visitor and the extractor's binder are gone. A same-id term with a different display is emitted twice, so `key:nodes` rejects rather than merges it | `type_shapes` snapshot; one term per variable form asserted |
| F1 | fixed | `type_class_targets` has no `ELSE`; both stale doc comments corrected | test: `the_types_rules_reject_their_violations` corrupts every class key and fails `typed:type_class_targets` |
| F2 | fixed | an annotated `async def` that is not a generator: Pyrefly's `Coroutine[Any(implicit), Any(implicit), X]` wrapping (`return_type_from_annotation`) is inverted exactly, so the declared return is `X`. The annotation key itself stays `pub(crate)`, so reading it would need a fork change | test: `async def fetch(n: int) -> str` reads `declared \| str`. Pilot: 0 declared coroutines |
| F3 | fixed | by the simpler alternative | test: `ts.uses` observes `ident` (`[T](x: T) -> T`) with the same `T` term; the split query is empty. Pilot: 0 split |
| F4 | fixed | a call, argument or raised exception Pyrefly records no type for is a `types` boundary (`missing_evidence`, subject the call site) and the module's coverage `partial`; no solutions for a module is a `native_unavailable` boundary (O5) | test: the `TypeVar`/`ParamSpec` declarations, a lambda-body call and a `sys.platform == "win32"` call are boundaries in the snapshot. Pilot: 244 in 65 modules |
| F5 | fixed | an enum literal keeps its class (member as detail, `type_class` edge); a variable's bound, constraints and default are its children (`type_arg_role` `bound`, `constraint`, `default`, appended) | test: two `Color.RED` terms with two class edges; `B`'s `bound` child `int`. Pilot: 17 enum literals with a class |
| F6 | fixed | a field whose class-field definition is `DefinedInMethod` stays its base's row | test: `SubRec` has no `x` row. Pilot: 771 fields (the 2 rows gone) |
| F7 | fixed | DESIGN §3.2, §3.4.1 (producer-scoped term ids), §3.5.1, §3.8 (Measured, rules), §B8 and §4.2.6 (accessors "that borrow or compose existing upstream queries"), pins.md | prose; the tests above |
| O1–O4, O6 | deferred | as the review's table | — |
| O5 | fixed | with F4 | — |

**Checks** (2026-09-23): `just test-all` passed (nextest 78/78, including the new
`the_types_rules_reject_their_violations`; pytest 21/21; rule tests 4/4; `lint-agents`; `adr
lint`; fixtures 36; family; cargo-deny; `pyrefly-fork`; gold). `just pilot` passed on a fresh
store (the migration changes `type_terms`): snapshot `69bc2a1e…`, every rule, 20.1 s.
