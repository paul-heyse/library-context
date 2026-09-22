# Design review: increment 1, slice 1 (cpg-schema, cpg-extract, cpg-core) (compact)

**Date:** 2026-09-22 · **Depth:** compact · **Mode:** code plus DESIGN.md. This is the end of a
slice that adds fact families and an extractor, so ADR-0001 owes a `compact` review.
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`1f0737f..0bf6c2d`
**Prior review:** `design_review_adr-0012-pyrefly-ruff-in-process_2026-09-22.md` (standard). This
slice implements the oracles in its §12 disposition. Its findings are cited here as review-F1 …
review-F11; this review's own findings are F1 … F9.

## 1. Decision and scope

**Target.** The four commits `1f0737f`, `d1f6827`, `97b72ac` and `0bf6c2d`:
- `crates/cpg-schema`: 23 codebooks, `IdHasher`, 17 tables declared through `table!`, and
  their insta snapshots;
- `crates/cpg-extract`: the driver, the walker, the Pysa mapper, public names, fact ids, the CLI,
  4 test files and 5 fixtures;
- `crates/cpg-core`: Delta create, verify, append and read, plus the SQL helper;
- `rules/` and `rule-tests/`;
- `scripts/check_pyrefly_fork.py`;
- the DESIGN amendments in `0bf6c2d`.

**Governing sections:** DESIGN §3.2–§3.5, §4.0–§4.3, §6, §8; ADR-0012; ADR-0008 (`proposed`).

**Observable outcome claimed.** For one Python package, one in-process run yields canonically
sorted raw tables for `exports`, `signatures`, `calls`, `coverage` and `provenance`. Those tables
round-trip exactly through Delta, CHECKs are enforced, and drift is refused at open. DESIGN §4.2
and §4.3 are relabelled from Proposed to **Implemented/Tested**.

**Supported scope.** Stage B only (raw extraction), plus the Delta write and read helpers.
Derivation, the §8 cross-table validators, `snapshots` and publication are declared Proposed
until slice 2 (DESIGN §4.3 header), and this review does not count them against the slice.

### Method and coverage

**Read in full:**
- every `src` file in the three crates, every test file and every snapshot;
- the fixtures, rules, rule tests, `check_pyrefly_fork.py` and its test, and the justfile;
- ADR-0012, ADR-0008, the prior review, STATUS.md;
- DESIGN §1, §3.1–§3.7, §4.0–§4.3, §6 and §8.

**Read at the grain cited:** the pinned Pyrefly checkout `b9f2857`, `report/pysa/`:
- `class.rs` L112–116 (`ClassRef` carries `class_id`);
- `call_graph.rs` L109–138 (`OriginKind`), L317–333 (`PysaCallTarget`), L560–565, L600–611
  (`CallCallees`), L725–740 (`strip_unresolved_if_called`), L745–758
  (`AttributeAccessCallees`, including `is_attribute`), L820–826 and L914–957.

**Checks run in this session:**

| Check | Command | Outcome | Observation |
|---|---|---|---|
| Default loop | `just check` | passed | fmt, clippy `-D warnings`, 31 nextest, 14 pytest, pyrefly, `ast-grep scan` clean, 3 rule tests, lint-agents, adr lint (12) |
| `test-all` additions | `just fixtures-check`; `just deps` (run separately after `just check`; together, the `test-all` recipe) | passed | 10 fixture files parse; family ok; cargo-deny ok; `pyrefly-fork: ok (b9f28575 = tag + patch; every environment read classified)` |
| Slice crates | `INSTA_UPDATE=no cargo nextest run -p cpg-extract -p cpg-core -p cpg-schema` | passed (31/31) | the harness tests ran the real `uv` dev-group pyrefly 1.3.1 CLI, not a mock |
| Probe 1: context sensitivity, `.py`+`.pyi`, nested classes | `target/debug/lctx-extract` on a scratch package, run twice against two venvs; read back with `uv run --no-project --with pyarrow` (25.0.1) | ran | F2, F3 |
| Probe 2: `is_attribute`, `if_called` remainder | the same binary on a second package, plus `.venv/bin/pyrefly check --report-pysa` (CLI 1.3.1) on it | ran | F1; Pysa's JSON has `"is_attribute": true` |
| Probe 3: BOM offsets against raw bytes | the binary on `fixtures/python/unicode_bom`; slicing the raw `__init__.py` bytes | passed | offsets include the BOM, as §3.4 states (O6) |
| Probe 4: what the import-cycle fixture emits | the binary on `fixtures/python/import_cycle` | ran | F6 |

The working tree has the operator's uncommitted `pyproject.toml`/`uv.lock` changes (fastmcp,
vllm), and `uv run` synced them. They touch no Rust crate, and the harness uses
`uv run --no-sync`.

**Not inspected or not attacked (asserted only):**
- the patch content, beyond the fork check;
- the `lsp-types` git source;
- memory and runtime cost (S7 is unchanged, and nothing was re-measured);
- behaviour on FastMCP itself, since no test runs the extractor on the pilot;
- how often `.py`/`.pyi` pairs and same-named nested classes occur in FastMCP 4.0.3 (not
  counted);
- whether the import-cycle fixture differs at `NumThreads(N)` (reasoned in F6, not executed;
  that would need a code change);
- `family_smoke.rs` (pre-existing).

## 2. Authority and lifecycle (compressed)

- **Column contracts: one authority.** A `table!` declaration (`cpg-schema/src/table.rs`
  L130–178) generates the row struct, the Arrow schema, the key, the CHECKs, the builder and
  `hash_fields`.
  - The walker and mapper construct `…Row` structs and never name a column type.
  - `cpg-core` creates Delta from `T::schema()` (`delta.rs` L47) and casts reads back to it
    (L154–169).
  - Without the macro, a column added to a table would need matching hand edits in two
    builders and the Delta path (DM-52).
- **Provenance: one authority.** Raw rows carry only `fact_id`. Run, origin, extraction mode,
  modality, fidelity and model live once, in `facts` (`facts.rs` L55–72).
- **Codebooks: one authority.** Rust enums, so no out-of-book code can be built; the registry
  snapshot pins them.
  - DESIGN §3.5 still repeats the values of the core codebooks. They agree today (compared by
    hand), but nothing checks that.
- **The analyzer revision behind `producer_id` has three machine-readable copies** (F4):
  - `Cargo.toml` L30–34 (`rev`), the one that is built;
  - `config.rs` L21 (`PYREFLY_REVISION`), the one that is hashed;
  - `check_pyrefly_fork.py` L24 (`REV`), the one that is checked.

  Nothing links them.
- **Identity behaviour.** Summarized under question 3 below.

## 3–4. Contracts and derivation

These merge into §6 and §7 at this depth. The absence lattice holds and is Tested
(`coverage.rs`):
- `unavailable` + `undecodable_source` for every family of a non-UTF-8 module;
- `partial` + `syntax_error` for every family of a recovered module;
- `partial` for `exports` with an `outside_provider_model` boundary per non-literal `__all__`;
- `partial` for `calls` with a `missing_evidence` boundary per unmatched non-annotation call;
- `complete_under_stated_model` otherwise.

`failed` and `not_requested` are never emitted. That is correct, because a panic aborts
extraction and the producer declares exactly three families.

## 5. Journey: the dependency environment changes

The same release, `rp`, with `usedep.py` calling `dep.helper(1)`, was extracted against two
analysis venvs:
- `venv1/site/dep/__init__.py` defines `def helper(x: int) -> int`;
- `venv2`'s `helper` is a class.

**Result:**
- Both runs have `run_id ea29d38d…` and `context_id a7efd694…`, and `site_package_path` is
  `["$venv/site"]` in both.
- `pysa_calls` at bytes 46–59 differs. venv1 has one `call` row targeting `dep F:0 helper`.
  venv2 has `new` → `builtins F:3 __new__` and `init` → `dep F:0 __init__`.

**What this means:**
- The run identity claims two different analyses are the same run.
- Rows whose payload happens to be equal get equal `fact_id`s across the two environments,
  even when a name inside them (an annotation `dep.X`) denotes different code.
- §3.4.1 builds `content_digest` from the sorted `run_id`s, so its "compares reruns" would call
  this an unchanged rerun.

This is F2. It is the mirror image of review-F9: making paths root-relative removed the one
field that used to differ between venvs.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **fail** (narrow) | Columns, provenance and codebooks each have one authority, and every representation is derived from it (§2). The exception: the analyzer revision that feeds `producer_id` is a hand-kept string (`config.rs` L21–24), independent of the pin that is built (`Cargo.toml` L30–34). `just deps` checks a third copy (`check_pyrefly_fork.py` L24, L96–97) against whatever cargo checkout matches `REV[:7]`, not against `Cargo.lock` | F4 |
| **G2** Semantic fidelity | **fail** | Demonstrated: a union-receiver property getter is `definite` although Pysa says `is_attribute: true`, and an uncalled identifier gets a `definite` unresolved remainder (F1). Class references collapse `A.Config` and `B.Config` into one label, and a `.py`/`.pyi` pair gives duplicate `(module_name, function_key)` rows (F3). The absence lattice itself holds (§3–4) | F1, F3 |
| **G3** Validity | **pass** (slice scope) | These reject: `RecordBatch::try_new` against the declared schema; enum-typed codebooks; CHECK on `DeltaTable::write` (`append_enforces_the_immutable_checks`); verify at open (`open_refuses_a_table_whose_checks_drift_or_are_missing`); plan-time refusal of DML, DDL and statements (`the_sql_helper_rejects_writes_at_plan_time`). §8 cross-table validators are Proposed (slice 2). The one soft conversion is the Pysa locator clamp (O5) | — |
| **G4** Hidden behaviour | **pass** | Tested: env refusal (`ambient_pyrefly_knobs_are_refused`) and absolute paths (`relative_paths_are_refused`). The `ConfigFile` is constructed and `ConfigFinder::new_constant` is used. No crate source names `.claude` or the skills (grep). `read_at`/`open_verified` create a directory (O1), which is inert for readers | — |
| **G5** Consistency and recovery | **pass** (slice scope) | A panic becomes `ExtractError::Panicked` and no output (`a_panic_aborts_the_whole_extraction`). A table left without CHECKs by a crash is refused at open (Tested). Publication is Proposed (slice 2). The reader's version pin and `snapshot_id` filter exist (`delta.rs` L177–204) but no test can see them fail (F7) | F7 (regression control) |
| **G6** Transformation and reuse | **fail** | Demonstrated: `context_id` and `run_id` ignore the dependency environment (F2). Unresolved: the one-thread contract's oracle can't discriminate (F6), and the syntax-node-id recipe is unpinned and rests on a `Debug` string (F9) | F2, F6, F9 |
| **G7** Truthful capability claims | **fail** (narrow) | Lines relabelled Implemented/Tested describe mechanisms the code does not use: `OriginKind` "fails the build"; target identity via `function_def_range`/class ranges/site-relative paths; the §4.2.2 built-ins list and "field role"; and more (F5) | F5 |

Each failure is narrow and closes with a few lines plus a fixture test. None touches the slice's
architecture: the `table!` contract, one parse, in-process collectors, the facts/raw split, or
Delta through `DeltaTable::write` with verify-at-open.

## 7. Findings

Ordered by severity: wrong facts and identities first, then authority and claims, then oracle
quality.

| ID | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | The Pysa mapper labels two conditional invocations `definite`. It drops `AttributeAccessCallees.is_attribute`, and it emits every `if_called` remainder as `definite`. | DM-42, DM-24, DM-08 · G2 | **(a)** `pysa_map.rs` L549–582 maps `if_called`, `property_getters` and `property_setters`, but never reads `is_attribute` (`call_graph.rs` L754–757: "at least one … execution flow where this is a regular attribute access"). DESIGN §4.2.3's "not carried" row does not list it either. `push_list` (L415–421) makes a lone getter `definite`. **Probe 2:** `def union_property(x: Union[A, B]): return x.size`, with a property only on `A`, gives a getter row with `modality = definite`; Pysa's JSON has `"is_attribute": true`. **(b)** L474–483 emits the remainder with `modality: Modality::Definite` even when `potential` is true. **Probe 2:** in `h = a1 if flag else other; return h`, `h` gets a row with callee `identifier`, phase `call`, `UnexpectedPyreflyTarget` and modality **definite**, though `h` is never called. `overrides_are_never_definite_and_if_called_is_potential` (`variants.rs` L63–66) asserts that such rows are `potential`. It passes only because the fixture never produces one. | (a) Pass A (§9.1) reads `x.size` as always invoking `A.size`; on `B` instances nothing is called (ADDENDUM Q5, "a call edge as always reached"). (b) A reference that is never called carries a definite unresolved call, which a brief can report as a limit at a site with no call. | (a) Carry `is_attribute` (a nullable column), and make a getter or setter with `is_attribute = true` `candidate`. Add a row to the §4.2.3 table. (b) Give the remainder of a potential list `potential`. Add both probe functions to `fixtures/python/pysa_variants`. | **Test.** With the fixture extended, the existing invariant test fails on today's mapper. Add an assertion that a getter with `is_attribute` is never `definite`, and re-review the snapshot. |
| **F2** | `context_id`, and so `run_id`, ignores the dependency environment. Two venvs with different site-packages yield the same identities and different facts. | DM-31, DM-32, DM-48 · G6 | `config.rs` L132–161 hashes the Python version, platform, root-relative search and site paths, and the relativized config JSON. There is no lock digest (DESIGN §3.4.1 L429, §4.0 L554) and no "content digests of those roots" (§4.0 L568). `Contexts` (`tables.rs` L56–74) has no column for either. **Probe 1** (§5): identical `run_id` and `context_id`; `pysa_calls` at 46–59 differs. `identities_do_not_depend_on_install_location` tests invariance only; nothing tests sensitivity. The review-F9 correction ("… together with content digests of those roots") is half applied. | A dependency upgrade in the analysis venv reads as the same run. `content_digest` (§3.4.1) and §9.8 ablation joins treat different inputs as a rerun. Equal `fact_id`s can denote different code (an annotation naming `dep.X`). | Add a digest of the site-package roots' content (sorted relative path and content digest per file) to `context_id` and to `contexts`. Replace it with, or add, the acquisition lock digest when Stage A lands. State in §4.0 that until Stage A the CLI's `release_id` is a label hash, so the same label on a different tree gives the same ids. | **Test.** The probe as a test: one release, two venvs whose `dep` differs; assert that `context_id` and `run_id` differ. Keep the two-location test, so the pair checks invariance *and* sensitivity. |
| **F3** | Pysa-side identity uses labels where DESIGN promises keys. Class references drop the provider's class id, and every Pysa table keys a module by name only. | DM-11, DM-42, DM-02 · G2, G6 | **(a)** `class_str` (`pysa_map.rs` L80–82) renders `module.name` and drops `ClassRef.class_id` (`class.rs` L112–116, present in Pysa's JSON). It feeds `receiver_class`, `defining_class`, `class_ancestry.ancestor` and `annotation_classes`. **Probe 1:** `A.Config.m` and `B.Config.m` both get `defining_class = rp.nested.Config`. Both call sites have `receiver_class = rp.nested.Config`. Parameters annotated `A.Config` and `B.Config` both have `annotation_classes = ['rp.nested.Config']`, while the display string keeps `rp.nested.A.Config`. So the "structure" is weaker than the string, contrary to `report_projection` (§3.5). **(b)** `pysa_functions`, `parameter_semantics`, `class_ancestry` and `pysa_calls` carry `module_name` and no path or `module_node_id`. DESIGN §4.2.3 L700–702 says (module name, site-relative path). **Probe 1:** a release with `rp/dual.py` and `rp/dual.pyi` has two `pysa_functions` rows keyed `("rp.dual","F:0")`, even with the same name offset (4). A `pysa_calls` row `caller F:1 → rp.dual F:0` can't say which file. | (a) Pass A and B conflate methods on same-named nested classes (the `class Config` pattern), and ancestry can't be joined back to `class_key` unambiguously. (b) The Stage-C bridge (§4.1 C) can't map `(module, function_key)` to one declaration for any library that ships inline stubs, such as attrs or numpy. Its uniqueness check would refuse the release, an unsupported input with no declared limit. | Carry `module_node_id` (or the release-relative path) on the four Pysa tables. Key classes as functions are keyed: `(module_node_id, class_id)`. Either change is a schema migration now, with no data to migrate. Alternatively, declare `.py`/`.pyi` pairs unsupported through a coverage row until Stage C. | **Test.** A fixture with `dual.py` + `dual.pyi` and two nested `Config` classes. Assert that `(module_node_id, function_key)` is unique and that the two `Config` references differ. |
| **F4** | The analyzer revision and adapter build hashed into `producer_id` are hand-kept constants, linked to nothing. | DM-02, DM-31 · G1 | `PYREFLY_REVISION` and `RUFF_LINE` (`config.rs` L21–22) are strings. The built pin is `Cargo.toml` L30–34 (`Cargo.lock` L4318). `check_pyrefly_fork.py` checks its own `REV` (L24) against a checkout found by `REV[:7]` (L96–97), and never reads `Cargo.lock` or `config.rs` (grep). The build digest is `CARGO_PKG_VERSION/EXTRACTOR_OUTPUT_VERSION` (L175). The workspace version is fixed at 0.1.0, so only a manual bump moves it. | A pin bump that edits `Cargo.toml` but not `config.rs` builds a new analyzer under the old `producer_id`, so different analyzers share a `run_id`. With the old checkout still in `~/.cargo`, `just deps` passes. A mapper fix (F1) without a manual bump also keeps `producer_id`. | Have the fork check read the rev from `Cargo.lock`, and assert that it equals `REV` and appears, with the patch sha256, in `PYREFLY_REVISION`. Or generate the constant in a `build.rs`. For the output version, either tie it to the variant snapshot, or accept it as manual and say so in §4.0. | **`just deps` recipe** (the extended fork check), plus a pytest case with a mismatched lock. |
| **F5** | DESIGN lines promoted to Implemented/Tested in `0bf6c2d` describe mechanisms the code does not have. This is review-F10's shape again. | DM-59, DM-43 · G7 | **Claims with no mechanism behind them:** **(1)** §3.5 L484–486 says a new `OriginKind` "fails the build", but `pysa_map.rs` L501 and L507 call `o.kind.to_string()` through upstream's `Display`, and none of our matches cover it. **(2)** §4.2.3 L700–702 names `function_def_range`, `ClassRef.class.range()` and site-relative dependency keys; none is used (F3). **(3)** §4.2.2 L654–657 lists `helpers::is_docstring_stmt`, `Docstring::range_from_stmts`, `Parameters::iter_source_order`, `AnyParameterRef` and `Ast::if_branches` as used; `walk.rs` uses none of them (its own `docstring()` at L109–117, its own parameter list at L248–278). L659's "(parent, field role, child ordinal)" is `Debug` kind plus ordinal (L445). **Divergences from the code:** **(4)** §3.4.1 L428 puts the span in syntax `node_id`s, which the code rightly omits; L421 leaves `kind_tag` unprefixed, while `id.rs` L68 length-prefixes it. **(5)** §4.2.1 L635 says the stack setting changes `producer_id`; it changes `run_id` (`config.rs` L174, L199). **Stale:** **(6)** §4.2.5 L757 still says "the nextest test is slice-1 work". **Overclaim:** **(7)** the §4.2 header's "Tested … the variant table" rests on a snapshot with no `ArtificialAttributeAccess` row (`site=2` occurs 0 times) and no `is_attribute` (F1). | A later session relies on §3.5 at a Pyrefly bump and trusts the build to flag a new `OriginKind`, and Pass A, dispatching on the kind string, silently misses it. An implementer following §4.2.3 builds Stage C on spans the tables don't carry. | Choose per line. Either make `OriginKind` a codebook through our own exhaustive match (with `ChainedAssign` and `Nested` structured), or rewrite the sentence. Reconcile (2)–(6) to the code; the code is the better choice for (4). Relabel (7), or add an artificial-attribute-access case to the fixture. | **Prose** for the labels (no mechanical oracle; AGENTS.md). **Test** for the fixture row, and for `OriginKind` if it becomes a codebook. |
| **F6** | The review-F8 oracle can't fail for the reason it exists. | DM-40, DM-54 · G6 (unresolved) | `import_cycle_output_is_identical_across_processes` (`identity.rs` L64–87) runs the CLI three times in the same order. The driver sorts handles (`lib.rs` L249), so no shuffle is possible, though §4.2.1 L638–639 says "shuffled order". The fixture's modules import each other, but its values form no binding cycle: `a.VALUE → b.OTHER` (a literal), and `b.DERIVED → a.PAIR → a.VALUE`. **Probe 4:** the only inferred column emitted is `receiver_class = builtins.int` on `b.OTHER + 1`, which comes from a literal. | A regression to `NumThreads(N)`, or a Pyrefly change that makes `Inline` order-sensitive, stays green. The test does catch HashMap-order nondeterminism across processes, which is its real value. | Use a fixture with a real SCC whose inferred type reaches an emitted column (for example, mutually recursive unannotated functions whose result is a method receiver). Add a test-only reversed-handle-order knob next to `fault_at_module`. Once, show that the fixture differs at `NumThreads(4)`. | **Test.** |
| **F7** | The reader's two G5 guards are unexercised. Deleting either one keeps every test green. | DM-14, DM-53 · G5 (regression control) | `every_table_round_trips_through_delta_exactly` writes each table once with one `snapshot_id` and reads at the version just written. So `.with_version(v)` (`delta.rs` L178) and `WHERE snapshot_id = …` (L199–204) are indistinguishable from "latest, unfiltered". §4.3 labels Read **Tested**. | An edit that reuses a loaded handle or drops the filter regresses to latest or cross-snapshot reads unnoticed (ADDENDUM Q8). | Append snapshot A, then B. Read A at A's version and at B's version, and assert that only A's rows come back. Until then, label Read **Implemented** for these two properties. | **Test.** |
| **F8** | Fact identity excludes provenance, and a conflict resolves silently first-wins. | DM-15, DM-02 · G2 (latent) | `FactSink::fact` (`facts.rs` L55–72) hashes the run, the table and the payload. Modality, origin, fidelity and model are not in the payload of `pysa_calls` rows, and `or_insert_with` keeps the first provenance for a repeated id. No colliding case was found in the fixtures or the probes. | If two mapping paths ever emit the same payload with different modality (for example, the same target in a definite list and a candidate list at one site and phase), one modality silently wins. | Return an error when a repeated `fact_id` arrives with different provenance, or hash the provenance. | **Test:** a unit test on the sink. |
| **F9** | Syntax `node_id`s depend on `format!("{:?}", node.kind())` and on ruff's visitor traversal set, and nothing pins the id recipes. | DM-15, DM-51 · G6 (unresolved) | `walk.rs` L445. The known-answer vector (`ids.rs`) pins `IdHasher`, not the extractor's recipes: syntax (L155–161), declaration (L185–190), parameter (L281–287), fact (`facts.rs` L56–59). §3.4.1 says `node_id` is "stable across snapshots and runs". Review-F9 rejected `Debug` in identity for sys info on the same grounds. | A ruff bump that renames a `NodeKind` variant, or visits a new node kind, changes every call and import `node_id` of an unchanged release. Ablation diffs across the bump then misreport. A recipe edit shows up nowhere. | Map `NodeKind` to a declared stable tag, or state in §3.4.1 that syntax ids are producer-scoped. Snapshot a few extractor ids on a fixture. | **Test:** an insta snapshot of one declaration, parameter, call and fact id on `unicode_bom`. |

**Observations** (no gate impact; each has a one-line fix):
- **O1.** `table_url` (`delta.rs` L38–42) runs `create_dir_all`, so `read_at` and `open_verified`
  create a directory on a missing table. This is inert, because `open_or_create` keys on
  `_delta_log`. On a read-only store, though, a missing table fails as EROFS rather than "not a
  table". Create the directory only in `create`. `delta_session()` (L214) has no caller.
- **O2. Rule gaps.**
  - `sql-through-helper` doesn't flag `sql_with_options` with permissive options outside
    `sql.rs`.
  - `no-catch-unwind-in-extractor` doesn't catch per-module scoped-thread isolation, which
    catches panics the same way.
  - No rule bans raw Parquet scans (AGENTS.md testing rules; DESIGN §4.3 "Never").
- **O3.** `fault_at_module` is a public, production-reachable panic injector (`config.rs`
  L46–48). A dev-only cargo feature would scope it. The injected panic fires in our loop
  (`lib.rs` L331–333), before that module's Pyrefly calls, so the test proves the thread
  boundary, not unwinding through Pyrefly frames.
- **O4. Diagnostic detail.**
  - Parse-error boundaries keep the range only as display text in `detail`, with
    `start_byte = None` (`lib.rs` L393–401), though the error carries a range (DM-47).
  - A `partial` coverage row has `reason = None` when the cause is `missing_evidence` or a
    non-literal `__all__` (L463–475); the reason is only on the boundary rows.
  - Boundary facts are attributed to `…/source` with `native_traversal`, though they compare
    two surfaces.
- **O5.** The locator maps line or column 0 to 1 (`pysa_map.rs` L53–54), and `LineIndex::offset`
  clamps past the end, silently. S5 found no such case on FastMCP.
- **O6.** The BOM claim (§3.4) holds: probe 3 slices `lcfix/__init__.py`'s raw bytes exactly at
  every `export_syntax` span. But the repo tests compare Ruff against Pysa ranges, which a
  uniform shift would not break. One assertion that slices acquired bytes would pin it.
- **O7. Assertions that add nothing.**
  - In the F3 oracle, the block `if !missing.is_empty() { assert_eq!(exports("dunder"),
    PARTIAL) }` (`coverage.rs` L81–87) repeats L60–64 and can't fail on its own. The non-literal
    modules' public sets are not asserted.
  - The harness `canon` (`harness.rs` L27–37) sorts *every* JSON array, including MRO and
    parameter order.

**Applicability.**
- **Bore on this scope:**
  - group 1 (F4);
  - group 2 (F1, F3, the absence lattice);
  - group 3 (F2, F3, F8, F9);
  - group 6: effects and panics, now Tested;
  - group 7 (F2);
  - group 9: adapters and capabilities (F1, F3, F5);
  - group 10 (F2, F4, O4);
  - group 11: snapshots and oracle quality (F6, F7, F9);
  - group 12 (F5).
- **Bore little:**
  - group 4: `table!` is the only declaration, handled under DM-52;
  - group 5: there is no derivation yet; only DM-24 applies, in F1;
  - group 8: this slice makes no performance claim, and S7 is unchanged.

**Verdicts:**
- **Satisfied:**
  - DM-52 (`table!` generates every mechanical artifact);
  - DM-51 (contract and registry snapshots under `INSTA_UPDATE=no`);
  - DM-07 at the local and storage boundaries (Tested);
  - DM-28 (Tested);
  - DM-30 for panics (Tested);
  - DM-08 for undecodable, recovered and non-literal-`__all__` modules (Tested);
  - DM-41 (the locator is mechanical; BOM confirmed);
  - DM-15 for `IdHasher` (length prefixes, presence bytes, known-answer vector, proptests).
- **Violated:** DM-24 and DM-42 (F1, F3); DM-31 and DM-32 (F2); DM-11 (F3); DM-02 (F4);
  DM-59 (F5).
- **Unresolved:** DM-40 (F6); DM-14 regression control (F7); DM-15 and DM-51 for the id
  recipes (F9).

### The five questions

**1. Does the implementation match §4.2/§4.3, and do the labels hold?**
- **§4.3 largely holds:**
  - Build, sort and ids: **Tested** (contract tests, known-answer vector).
  - Create, verify, write, the SQL helper and the `INSERT INTO` bypass assertion: **Tested**.
  - Read: **Tested** for values; **Implemented** only for the version pin and the snapshot
    filter (F7).
- **§4.2 is mixed:**
  - **Tested:** the driver, env refusal, absolute paths, the panic abort (at the thread
    boundary; O3), the `__all__` detector, `_invalid/` coverage, and harness equivalence on two
    fixtures, including the `--public-only` "explained" check.
  - **Implemented, but its oracle doesn't discriminate:** the one-thread contract (F6).
  - **Tested with gaps:** the variant table (F1; site kind 2 absent).
  - **Not as written:** target identity and the built-ins list (F3, F5).
  - **Incomplete against §4.0:** context recording (F2).
  - **False as stated:** "exhaustive matches catch a new `OriginKind`" (F5).

**2. Are the review-F1–F11 oracles present and meaningful?**

| Prior | Oracle promised | Present | Meaningful? |
|---|---|---|---|
| review-F1 | `catch_unwind` rule; fault-hook test | yes (rule plus 3 rule tests; `a_panic_aborts_the_whole_extraction`) | Yes, at the driver boundary. The panic is injected outside Pyrefly frames, and "no `snapshots` row" can't be checked before publication exists (O2, O3) |
| review-F2 | variant fixture + insta | yes (`pysa_variant_table_snapshot`, 41 rows + 1 boundary) | Partly. No `ArtificialAttributeAccess`. `is_attribute` is unmapped. The `if_called` remainder branch is unexercised and contradicts the invariant test (F1) |
| review-F3 | `__all__` fixture, hand-written sets | yes (`dunder_all_forms_hand_written_expectations`) | Yes for the four statuses, the boundary count and `lit`'s set. One conditional block is vacuous (O7) |
| review-F4 | open-time verify; codebook-append | yes (`open_refuses…`, `codebook_growth_needs_no_constraint_change`) | Yes. The dropped constraint, the crash before `add_constraint` and a future code 99 are all exercised |
| review-F5 | `_invalid/` fixture | yes (`undecodable_and_broken_modules_are_never_silent`) | Yes |
| review-F6 | fidelity test | **no** | Moot as designed: `parameter_semantics` always keeps string, classes and scalars, so `display_only` never applies. The fidelity claim fails elsewhere instead, through the class id dropped in F3 |
| review-F7 | write/SQL rules; `SQLOptions` test; asserting bypass test | yes (2 rules, 3 tests) | Yes, with small rule gaps (O2) |
| review-F8 | cycle fixture, shuffled order and processes | processes only | **No** for its purpose (F6) |
| review-F9 | two-location test | yes (`identities_do_not_depend_on_install_location`) | Yes for invariance. The root-digest half of the correction is missing (F2) |
| review-F10 | prose | — | Recurred (F5) |
| review-F11 | fork and env `just` recipe | yes (`check_pyrefly_fork.py` in `just deps`: passed) | Yes for the checkout it finds, which is not tied to `Cargo.lock` or the producer string (F4) |
| O1 | `Decision` line on §3.3; §6 header | yes | — |

**3. Identity: collision or instability risk?**
- **Node ids from occurrence paths:** no collision found.
  - Syntax ids hash `(release_id, path, [Kind#ordinal …])`. The child ordinal is unique per
    parent, and length prefixes separate the encodings.
  - Declaration ids use `(path, qualified name, occurrence)`, which distinguishes
    redefinitions and overload stubs.
  - The instability risk is F9 (`Debug` names, visitor set, no recipe pin).
- **Parameter ids** (`SYNTAX`, path, `"Parameter"`, function id, ordinal): no collision. They
  share the `syntax` tag with call ids, but the first field after the path is an 8-byte list
  count for syntax ids and the 9-byte string `"Parameter"` for parameter ids, so the two can't
  alias. They are stable while the declaration occurrence is.
- **Fact ids:** `snapshot_id` and `fact_id` are zeroed before hashing (`facts.rs` L84–85,
  verified), and the run and table name are included. Equal ids mean byte-identical rows, so
  `dedup_by_fact` loses only multiplicity. The risk is F8 (provenance outside the id).
- **Module-name keys:** they collide in two demonstrated ways:
  - within a release, a `.py`/`.pyi` pair (F3b);
  - across environments, where the same `dep.helper` names different code under one
    `run_id` (F2).

  Within one context Pyrefly resolves each imported module name to one file, so dependency
  targets are unambiguous per run. Adding the site-relative path, as §4.2.3 promises, fixes
  F3b but not F2.

**4. Are the facts/raw split, coverage/boundaries and the codebook additions sound under G1
and G2?**
- **The split: sound under G1.** Provenance has one home, and every raw row maps to exactly one
  `facts` row by construction (`fact_row!`). The latent weakness is F8.
- **Coverage/boundaries: sound under G2 for what they cover** (the §3–4 lattice, Tested). They
  do not see the F1 and F3 losses, because those are mapping losses, not stops.
- **`syntax_error` and `undecodable_source`** are appended as codes 11 and 12. No numbering
  existed before, the registry snapshot now pins them, and they are distinct from
  `native_unavailable`. Sound.
- **The twelve extraction codebooks** mirror Pyrefly enums through exhaustive matches.
  - `pysa_target_kind` folds "no target" in as `unresolved`, which is acceptable for a raw
    table.
  - `invocation_phase.decorator` is unused: decorator applications arrive as `ArtificialCall`
    with phase `call`, as the §4.2.3 table says.
  - The gap is `OriginKind`, kept as a `Display` string (F5).

**5. What should gate ADR-0008's acceptance?** It is `proposed`, and STATUS says slice 1's
snapshots settle it. They settle its column and codebook half. I would accept it when four more
hold, or narrow it so they are explicitly deferred:
- **(a)** The §8 coverage-completeness query (a row for every declared family × module) and a
  raw-`fact_id` → `facts` reference query exist as shared validators and run on the slice-1
  fixtures. Today "absence is never implicit" is enforced only by the producer's own
  construction.
- **(b)** The provider-local keys Stage C will join on are unique per snapshot (F3), checked by
  the Stage-C uniqueness query.
- **(c)** The family → node/edge mapping that ADR-0008's Decision places in `cpg-schema`
  exists, or the ADR says it waits for the first endpoint validator. Slice 1 declares none.
- **(d)** Each provenance table names its producer. The extractor now writes `source_files`,
  `runs`, `contexts` and `producers`, while §4.1 assigns the provenance family to Stage A, so
  "one producer per table" needs a decision before Stage A lands.

The registry and contract snapshots are the right regression oracles for the rest.

## 8. Alternatives (compressed)

| Alternative | Assessment |
|---|---|
| Current: a `table!` macro, `ArrowColumn` and `HashField` traits, one declaration per table | Justified. It removes the second-authority risk between the schema, the builders and the fact hash (DM-52). Seventeen tables already use it, and extending it is one declaration plus a snapshot |
| Simpler: hand-written builders per table, as the research input sketches | Rejected. Each table would re-declare its columns (REVIEW_REFERENCE §5, first row) |
| Current: immutable CHECKs + `verify` re-deriving delta-rs's normalization (`delta.rs` L69–85) | Keeps defence in depth. The cost is coupling to delta-rs's parse-simplify-render pipeline, and a delta-rs bump that changes it fails *loudly* (every open refused). Acceptable |
| Simpler: no CHECKs, only the §8 validators (review-F4 option ii) | Viable. It would remove `expected_constraints` and the verify step. Not recommended while §8 is still Proposed, because the CHECKs are the only storage-boundary validation in slice 1 |

**No over-construction found.** `code_range` has only a test consumer, and `delta_session`
has none. Both are trivially small (O1).

## 9. Top verification gaps

| Claim | Label now | Gap | Oracle |
|---|---|---|---|
| Modality of Pysa rows | Tested on the fixture, wrong off it | F1 | extend `pysa_variants` with probe 2's `union_property` and `name_partly_unknown` |
| Run identity covers everything that changes output | Implemented (partial) | F2 | the two-venv test (probe 1's `usedep` + `dep`) |
| Pysa keys are unique; class references are keys | Implemented as labels | F3 | a `dual.py`/`dual.pyi` + nested `Config` fixture (probe 1) |
| Producer identity follows the built pin | Implemented by hand | F4 | extended `just deps` |
| One-thread determinism | Implemented; cross-process Tested | F6 | an SCC fixture + reversed order |
| Pinned-version, snapshot-filtered reads | Implemented | F7 | two-snapshot read test |
| BOM-inclusive offsets | Tested (S5, probe 3) | no repo test slices raw bytes (O6) | one assertion on `unicode_bom` |

**The probe fixtures** lived in the reviewer's scratchpad, so they are described here for reuse:
- `rp/dual.py`: `def f(x): return x`, `def g(): return f(1)`; `rp/dual.pyi`:
  `def f(x: int) -> int: ...`, `def g() -> int: ...`.
- `rp/nested.py`: `class A: class Config: def m(self) -> int`, the same under `B`, and
  `def use(a: A.Config, b: B.Config) -> int: return a.m() + b.m()`.
- `rp/usedep.py`: `import dep; def call(): return dep.helper(1)`. There are two
  `site/dep/__init__.py` variants: `def helper(x: int) -> int` and a class `helper`.
- `rq/m.py`:
  - `A` has an `@property size`, and `B` has `size: int = 0`;
  - `def union_property(x: Union[A, B]): return x.size`;
  - `def name_partly_unknown(flag: bool, other: Any): h = a1 if flag else other; return h`.

## 10. Exceptions

None claimed. F1–F9 are recorded as violations or unresolved decisions, not as exceptions.

## 11. Decision

**Decision: Revise before closing slice 1.** `cpg-schema` and `cpg-core` pass their gates as
implemented; F7 there is a regression test to add. `cpg-extract` and DESIGN need F1–F5.

**Reason.** The architecture is sound and mostly well evidenced:
- one contract declaration per table;
- one parse with in-process collectors;
- provenance in `facts`;
- Delta written only through `DeltaTable::write`, with CHECKs verified at open;
- a harness-equivalence test against the real CLI;
- all review-F1–F11 oracles except one present, and most of them meaningful.

Four gates fail, each narrowly and each on in-scope behaviour:
- **G2:** mapped modality and class/module identity (F1, F3);
- **G6:** the run identity misses the dependency environment (F2);
- **G1:** the analyzer revision has unlinked copies (F4);
- **G7:** labels outrun the code (F5).

Under the calibration, a failed gate on claimed behaviour means Revise. None of these needs a
new spike or a §B change:
- F1 adds one row to the §4.2.3 table, which implements ADR-0012's "every variant has a row"
  rather than changing it. I read this as a DESIGN correction citing this review, not a new
  ADR; the author may judge otherwise.
- F2 and F3 bring the code up to DESIGN §4.0 and §4.2.3.
- F4 is a script change.
- F5 is relabelling.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | Carry `is_attribute`; potential remainder; §4.2.3 row (F1) | DM-42, DM-24 | the extended variant fixture and the invariant test pass | test |
| 1 | Dependency-environment digest in `context_id`/`contexts` (F2) | DM-31, DM-32 | the two-venv test gives different ids; the two-location test stays green | test |
| 1 | `module_node_id` on the Pysa tables; class keys `(module, class_id)` (F3) | DM-11, DM-42 | `.py`/`.pyi` + nested-class fixture; unique keys | test (schema migration: snapshot diff) |
| 2 | Tie `PYREFLY_REVISION` and `REV` to `Cargo.lock` (F4) | DM-02, DM-31 | `just deps` fails on a mismatched lock | `just` recipe + pytest |
| 2 | Reconcile or relabel the §3.5, §4.2.1–§4.2.5 and §3.4.1 lines; the `OriginKind` decision (F5) | DM-59 | DESIGN lines match the code | prose; test if `OriginKind` becomes a codebook |
| 3 | Discriminating cycle fixture + order knob (F6) | DM-40 | a fixture shown to differ at N threads | test |
| 3 | Two-snapshot read test (F7) | DM-14 | a filtered, version-pinned read | test |
| 3 | Sink provenance conflict is an error (F8); id-recipe snapshot and stable kind tags (F9) | DM-15, DM-51 | unit test; insta ids | test |
| — | O1–O7 | — | one line each | as noted |

**ADR-0008:** keep it `proposed` until the §7 Q5 conditions (a)–(d) hold or it is narrowed to
defer them.

### Deferred

| Item | Why deferred | Reopen when |
|---|---|---|
| Running the extractor on FastMCP 4.0.3 in a test | no pilot acquisition (Stage A) yet; S2–S7 cover the spike | Stage A lands (increment 1), or the increment-1 `deep` review |
| Counting `.py`/`.pyi` pairs and nested same-name classes in FastMCP | F3 is fixed by schema, not by frequency | if F3 is deferred instead of fixed |
| Carried from the prior review: sidecar isolation, dropping `State` before Stage C, end-to-end cost | no trigger fired in this slice | their triggers (prior review §11) |

## Disposition (author, 2026-09-22)

Every finding is fixed or deferred below. Verified with `just test-all` on 2026-09-22: passed (nextest
37/37, pytest 18/18, ast-grep rule tests 4/4, family, cargo-deny, fork check). The contract snapshots
changed (a schema migration, with no data to migrate): `module_node_id` on the four Pysa tables and in
their keys, `contexts.site_packages_digest`, and `pysa_calls.is_attribute`.

| ID | Outcome | Change | Oracle |
|---|---|---|---|
| F1 | fixed | `is_attribute` is carried; a getter or setter where it holds is at most `candidate`; the unresolved remainder of an `if_called` list is `potential`. The fixture gains `union_property`, `name_partly_unknown` and a 3-argument `getattr`, which also gives the missing `ArtificialAttributeAccess` site | `modality_follows_the_variant_table`: fails on the old mapper, and counts each case so it can't pass vacuously. Variant snapshot re-reviewed |
| F2 | fixed | `site_packages_digest` (every file's relative path and content digest, `__pycache__` skipped) goes into `contexts` and `context_id`. §4.0 states that the CLI's `release_id` hashes the label until Stage A | `identities_follow_the_dependency_environment`, beside the two-location test |
| F3 | fixed | The four Pysa tables carry `module_node_id` and are keyed by it. References resolve through Pysa's `ModuleId` to a module ref (`@<path>` for a release file, else the name). Classes are `<module ref>:<Name>#<ClassId>` | `pysa_keys_are_unique_per_file_and_class_references_carry_the_class_id` (new `pysa_keys` fixture: `.py`/`.pyi` pair, two nested `Config` classes; the stub is the target from outside, the source from within) |
| F4 | fixed | `Cargo.lock` is the authority. The fork check requires one locked commit, and the driver's `PYREFLY_REV`/`PYREFLY_PATCH_SHA256` and pins.md must name it and the patch sha256. The output version stays manual, as §4.0 now says | 4 pytest cases (including a mismatched lock) and `just deps` |
| F5 | fixed | (1) `OriginKind` becomes `site_detail` text through our own exhaustive match, not a codebook: `ChainedAssign` and `Nested` are structured and have no consumer. (2) §4.2.3 "Module and class keys" rewritten. (3) §4.2.2 built-ins list now matches the code. (4) §3.4.1: `kind_tag` length-prefixed; no span in `node_id`. (5) The stack setting changes `run_id`. (6) §4.2.5 harness test line. (7) The fixture now has site kind 2 and `is_attribute` rows | prose, plus the snapshot for (7) |
| F6 | fixed; thread sensitivity not shown | The cycle fixture has a return-type cycle and a global cycle whose solved types reach `pysa_calls`, plus a `reverse_module_order` test hook. A one-off probe (temporary `NumThreads(8)`, 6 runs × 2 orders, 2026-09-22) gave output identical to `Inline`, so the fixture cannot show thread sensitivity: Pyrefly 1.3.1 iterates cycles to a fixpoint. §4.2.1 now calls one thread a precaution | `module_order_does_not_change_the_cycle_output`; the cross-process test is kept |
| F7 | fixed | — | `reads_pin_the_version_and_filter_the_snapshot` |
| F8 | fixed | A repeated `fact_id` with different provenance is `ExtractError::ProvenanceConflict` | unit test in `facts.rs` |
| F9 | fixed (the review's second option) | §3.4.1 says syntax ids are producer-scoped | `extractor_id_recipes_snapshot`: producer, context and run ids, and one id per recipe on `unicode_bom` |
| N1 (new) | fixed | Found by the F9 snapshot. `config_digest` hashed `serde_json::Value::to_string()`, and DataFusion enables serde_json's `preserve_order` wherever it shares the build graph. So `context_id`, `run_id` and every `fact_id` depended on which crates were built together. The config JSON is now `sort_all_objects()`'d before hashing | the id snapshot passes under both `-p cpg-extract` and the workspace run |
| O1 | fixed | `table_url` touches nothing, `create` makes the directory, and `delta_session` is removed | the F7 test asserts that a missing table stays missing |
| O2 | fixed | `sql_with_options` is flagged outside `sql.rs`; `thread::scope` is flagged in the extractor; new `no-raw-parquet-scan` rule | rule tests |
| O3 | partly | The hooks are grouped in a doc-hidden `TestHooks`; the CLI never sets them | Deferred below |
| O4 | partly | Parse-error boundaries carry their byte span. A `partial` coverage row carries its first cause (syntax error outranks) | Deferred below: boundary provenance |
| O5 | deferred | — | below |
| O6 | fixed | — | the `unicode_bom` test slices every declaration name out of the stored bytes |
| O7 | partly | The vacuous block is replaced by exact public sets for `dunder` and `dunder.dyn` | Deferred below: harness `canon` |

ADR-0008 stays `proposed`. Condition (b) has its keys now (`module_node_id`, class ids) but no Stage-C
uniqueness query yet; (a), (c) and (d) are unchanged.

| Deferred item | Why deferred | Reopen when |
|---|---|---|
| O3: gate the hooks behind a dev-only cargo feature; inject a panic inside Pyrefly frames | A feature needs a self dev-dependency. A panic inside Pyrefly frames needs a patch line, and the abort path is the same thread boundary | the fork patch next changes, or a hook is found reachable from the CLI |
| O4: boundary facts attributed to `source`/`native_traversal` though they compare two surfaces | No consumer reads a boundary's model yet | slice 2's validators, or Pass A reading boundaries |
| O5: the locator clamps line/column 0 and offsets past the end | S5 found no such location on FastMCP (29,279/29,279 exact) | any Pysa location outside its module's text |
| O7: the harness `canon` sorts every JSON array (MRO and parameter order too) | Sorting only set-valued arrays needs a per-field list of Pysa's set-valued fields; the snapshot and the mapper tests pin order-bearing fields independently | a harness difference hides behind sorting, or the next Pyrefly bump |
