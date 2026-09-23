# Design review: H1, the library-leverage hardening slice (standard)

**Date:** 2026-09-23
**Reviewer:** `design-reviewer` subagent (fresh context), running the `design-review` skill. The
author of H1 is not the reviewer.
**Target:** commit range `2473e6a..8f8693c` on `main` (20 commits). This is the H1 slice from
`design_review_library-leverage_2026-09-23.md`: items C1–C9, P1–P7, O1–O5, D1–D6, its §5 stale
claims and the operator's Decisions. The target was reviewed against:
- DESIGN §3.2, §3.3, §3.4.1, §4.0, §4.3, §5, §6.2, §7, §8, §9.4–§9.6 and §13;
- ADR-0016 (new) and ADR-0011 (amended in place while still `proposed`; it changes §B4);
- the amendments to ADR-0009, ADR-0012, ADR-0013 and ADR-0002;
- the charter, `ADDENDUM.md` and the graph guidelines.

**Depth:** standard. ADR-0011 changes §B4 and ADR-0016 is new, so ADR-0001 owes a standard review.
The same review also covers the compact-review cadence for C1 (a change to extractor output) and
for C2, C3 and C5 (Stage A).

**Prior reviews:** the leverage review is the source, and it records probes, not the as-built
code. The C3 and C5 compact reviews bear on C1 and on `--unpublished`, and the C6 deep review
bears on the memory claims. Their Deferred rows are not re-raised here.

---

## 1. Decision and scope

**Proposal.** H1 hardens the codebase before slice 4 by leaning on libraries it already links:
- **Correctness (C1–C9):**
  - static branches come from Pyrefly's own `SysInfo::evaluate_bool`;
  - corpus selection uses globset and walkdir;
  - library definitions are typed serde structs;
  - `RECORD` is read as CSV;
  - the CLI uses clap;
  - `git init` is hermetic;
  - Pyrefly's own predicates replace local ones;
  - the hash and sort code gets known-answer tests.
- **Runtime (P1–P7):**
  - jemalloc is the allocator (ADR-0016);
  - validation runs over cached tables with concurrent rules and per-rule plan metrics;
  - each pinned read opens only its commit's files;
  - the canonical sort skips a constant leading key;
  - data files are written with zstd;
  - the `lctx_id` UDF is checked at plan time.
- **Hygiene (O1–O5):** a tracing subscriber; fs-err and anyhow; `cargo shear`.
- **Design (D1–D6):**
  - ADR-0011 is amended (own PageRank, normalized Leiden input, own FCA with an oracle,
    condensation from SCCs, §5's adapter recipe);
  - a fork accessor reads the declared return annotation.

**Status.**
- Implemented for C1–C9, P1–P7, O1–O5 and D6.
- Proposed (document-stage) for D1–D5.

**Affected revisions.**
- Code at `8f8693c`; fork `a07b7bae` (branch `lctx/1.3.1-r3`).
- Published pilot snapshot `15fecdabc74a2dfa04527228524b5a5a`, content digest `10e56541…`.

**Observable outcome.**
- Static-branch marks that match the analyzer.
- A fail-closed corpus selection.
- A pilot compile of 29.9 s at about 3.6 GB. It was 45 s at about 7 GB.
- A test suite of about 84 s (it was 225 s).
- Graph-algorithm choices corrected before slice 4 builds on them.

**Baseline.** `main` at `2473e6a`, as characterized by the leverage review.

**Supported scope and non-goals.**
- No new fact family; one codebook append (`static_branch` 3 and 4).
- No schema change.
- D1–D5 build nothing now.

**Constraints and uncertainty.**
- One pilot library.
- The claims about the pre-H1 build and about the pre-D6 comparison rest on the author's scratch
  fingerprints. This review could not rebuild those revisions and did not.

### Method and coverage

**Read in full, at the grain cited:**
- `crates/cpg-core/src/`: `snapshot.rs`, `validate.rs`, `delta.rs` and `attempt.rs`;
- `udf.rs` L1–190;
- `cpg-schema/src/table.rs` L25–75;
- `cpg-extract/src/lexical.rs`: the C1 code (L133–224, L500–516, L740–790) and its test
  (L1150–1209);
- `library.rs` L90–200 (typed definitions, `uv.lock`) and L480–720 (walk, `glob_set`,
  `literal_prefix`, `pick`, `corpus`);
- `types.rs` L825–862 (D6);
- `lctx/src/main.rs` L1–140 and L400–500;
- `lctx-extract.rs` L1–80;
- ADR-0016;
- the ADR-0011 diff, and the ADR-0009, ADR-0012, ADR-0013 and ADR-0002 amendments;
- every DESIGN hunk in the range;
- the `docs/pins.md` diff and `third_party/pyrefly-1.3.1.patch`.

**Read in the pinned upstream sources:**
- Pyrefly `a07b7ba`:
  - `sys_info.rs` (`evaluate`, `evaluate_bool`, `is_type_checking_guard`,
    `pruned_if_branches`);
  - `binding/stmt.rs` L1242–1343 (the binding pass's `if`);
  - `alt/answers.rs` L1518–1660 (`get_idx`, the new `get_annotation`);
  - `is_public_name`, `Docstring::range_from_stmts` and `ModulePath::is_init`.
- delta-rs `58f07cd`:
  - `writer/stats.rs` L208–232;
  - `operations/write/mod.rs` L347–360;
  - `write/execution.rs` L540–800 (the writer's fan-in).
- petgraph 0.8.3: the lines cited in §5.

**Run in this session (2026-09-23, this host):**

| Check | Command | Outcome |
|---|---|---|
| Whole suite | `just test-all` | **passed**. nextest 104/104 in 84.4 s; pytest 21/21; rule tests 4/4; `adr lint` ok (16); fixtures 39; `just deps` ok, including `pyrefly-fork: ok (a07b7bae …)` and `cargo shear`; gold ok |
| Flakiness under concurrency | `cargo nextest run -p cpg-core`, 5 consecutive runs | **passed** each time (41/41; 82–90 s) |
| Mutation re-run owed by C8 | `cargo mutants --in-place --file crates/cpg-schema/src/{hash,table}.rs -p cpg-schema`, in a scratch clone at `8f8693c`, deleted after | **failed** (exit 2): 31 mutants; 23 caught, 4 missed, 4 unviable. All 13 `hash_into → ()` mutants and the `num_rows() < 2` mutants are caught. The 4 survivors are P4's skip loop, and all are equivalent (§9) |
| Pilot rerun | `target/release/lctx compile fastmcp` into a scratch copy of `build/store` | **passed**. Attempt `57fe73c0…`, content `10e5654157643cb6…`, equal to the published `10e56541…`; 29.88 s wall; `VmHWM` 3639 MiB; validation 0.95 s |
| Rerun content | a pyarrow probe over each commit's files: every table's row multiset, `snapshot_id` dropped, both snapshots | **passed**: all 57 data tables equal; only `snapshots` differs (its versions) |
| Nested static branches | debug `lctx-extract` over a 20-line scratch module | shows F1 |
| `--unpublished` over two snapshots | `lctx query` on the scratch store, with and without `--unpublished` | shows F2 |
| globset coverage probe | a scratch crate on `globset =0.4.20` | shows F5 |
| Fork branch | `git ls-remote https://github.com/paul-heyse/pyrefly.git 'refs/heads/lctx/*'` | **passed**: `r3` → `a07b7bae`; `r2` → `6a93da34` and `lctx/1.3.1` → `b9f28575`, both unchanged |
| D6 bundled nodes | SQL on `build/store` at `15fecdab…` | **passed**: 103 bundled modules + 982 symbols = 1,085 nodes; 91,278 edges touch them |

The scratch store, the clone and the probe crate were deleted.

**Not inspected, or not reproduced:**
- The claim that every non-bundled node and edge is byte-identical to the pre-D6 build. It was
  checked in the author's scratch worktree; only the counts were reproduced here.
- The equality of the pre-H1 and H1b fingerprints (it needs pre-H1 builds).
- The clap parse tests in detail; the fs-err/anyhow conversions beyond a grep; `propose.rs`.
- The `environment_digest` walk. This review relied on the fixture `context_id` being unchanged
  in the identity snapshot.
- P5's metric arithmetic.

**Guarantees not attacked (asserted):**
- jemalloc's interplay with Pyrefly's threads.
- Old readers reading zstd files.
- D6's fallback to the computed return when an annotation key is unsolved. It is safe only
  because solving `ReturnType` solves its annotation key first; that is argued, not tested.

### 1.1 What H1 did, item by item

| Item | Commit | Verdict |
|---|---|---|
| step 0 | `b4ee5cc` | Decisions recorded |
| C1 | `296000b` | Done per clause and matches `pruned_if_branches`. **F1**: nested marks are wrong |
| C2 | `24ed13d` | Done; fail-closed in the main. **F5**: the "covered" probe and no exclude key for `examples`/`tests` |
| C3 | `591f1f8` | Done |
| C4 | `5586df7` | Done (`compile_honours_reinstall`) |
| C5 | `a2a5cc1` | Done |
| C6, C7, C9 | `e7fe21e` | Done. `is_public_name` differs from the old test only for `_x__`-style names |
| C8 | `9beb798` | Done. The mutation target is met: the survivors come from P4 (after C8) and are equivalent |
| P1 | `4fd8fa2` | Done (ADR-0016). **F7**: units |
| P4 | `d5e0434` | Done. **O1**: the stored order is the writer's |
| P2, P5 | `92ecb32` | Done. The rule order is real; the cache is memoization over the registered views |
| P3 | `2056a2d` | Done for published reads. **F2**, **F3** |
| P6 | `c3d6cab` | Done (`data_files_are_zstd`) |
| P7 | `6691eb0` | Done |
| O1 | `2f1ca94` | Done. **F6**: noise |
| O3, O4 | `6a86716` | Done |
| O2, O5 | `c700f40` | Done. **F8**: `futures` is unpinned |
| D6 | `bda9e3c` | Done, within ADR-0012 (54 changed lines by diffstat; the ADR says 51) |
| D1–D5 and the §5 stale claims | `fff5aa5` | Recorded. The four stale claims are corrected (the citations verified). **F9**: unresolved items for the consuming slices |
| step 20 | `8f8693c` | Measured. **F7** |

---

## 2. Authority and lifecycle map

| Concept or fact | Authority / owner | Revision boundary | Permitted update path | Derived representations |
|---|---|---|---|---|
| Whether Pyrefly analyzes a branch | Pyrefly `SysInfo::evaluate_bool`, applied as `pruned_if_branches` does, and recursively (the binding pass never walks a pruned body) | fork revision | a pin bump | `bindings.static_branch`/`static_polarity`. **Per clause only; not recursive (F1)** |
| Package, public name, docstring | Pyrefly (`is_init`, `is_public_name`, `range_from_stmts`) | fork revision | pin bump | `source_files.is_package`, `public_names`, declaration docstrings |
| Declared return type | Pyrefly `KeyAnnotation::ReturnAnnotation` (fork accessor) | fork revision (moves `producer_id`) | fork patch under ADR-0012 | `type_observations` (`declared`) |
| Corpus selection | `[tool.lctx.source]` globs (typed, unknown keys refused) over one no-follow walk | the library definition's commit | edit the definition | the corpus release id; `source_files` roles |
| A snapshot's rows of a table | the `snapshots` row names the version; **the commit at that version holds the rows** (P3) | Delta commit | one write per table per attempt (the write path's structure) | registered views; the validation cache; `read_at` |
| Validation input | the attempt session's registered views | the attempt | none; the cache is rebuilt per call | a `MemTable` per table (`cached_session`) |
| Engine and allocator identity | `Cargo.lock` (`ENGINES`, which feeds `compiler_digest`) | lock | `just deps` | the content digest (inputs only) |

**Deliberately opaque behavior.**
- The Delta writer's row order inside a file (O1).
- jemalloc's retention (F7).

**Identity behavior.**
- D6 moves `producer_id`, and with it run and fact ids. It also moves the bundled-stub nodes,
  whose identity is the fork revision (O2).
- H1b moves nothing.
- A rerun under this review reproduced the content digest, and all 57 tables have equal row
  multisets.

---

## 3. Semantic contracts and invariants

| Contract or invariant | Representation | Enforcement boundary | Failure behavior | Verification evidence |
|---|---|---|---|---|
| `static_polarity` is "Pyrefly analyzes this branch" (DESIGN §3.2 L462; `tables.rs:652-656`) | `clause_marks` (`lexical.rs:164`), then the innermost enclosing mark (`lexical.rs:766-772`) | `static_marks_agree_with_pyrefly_pruning`, **per `if` statement only** | Nested decided clauses inside a pruned clause read `true` (**F1**) | **Tested** per clause; **violated** for nesting (probe) |
| The rows of a snapshot's table are exactly one commit's | `write_raw`/`write_derived` write each table once (`attempt.rs`) | **None at read**: `commit_adds` (`snapshot.rs:69`) takes the commit's adds without checking whose they are | A read at a commit of another attempt returns 0 rows, silently (**F2**) | **Tested** positively (`a_pinned_read_opens_only_its_commits_files`); no negative case |
| A selected link, or a directory link that could hold a selection, is refused unless excluded | `pick` (`library.rs:596-640`) | Stage A | An exclude that only matches `<link>/_` counts as "covered", and the link is skipped silently (**F5**) | **Tested** (`symlinks_in_a_tree_are_refused_unless_excluded`) except that case |
| Validators only read; one query per rule; violations in `rules()` order | `validate_costed` (`validate.rs`), the index sort | library code, shared by tests and publication | n/a | **Tested** (`violations_come_back_in_rule_order`, `every_table_is_read_by_some_rule`); 6 green concurrent runs |
| The canonical sort's order is unchanged by P4 | `table.rs:44-69`, constancy checked | before each write | n/a | **Tested** (`a_constant_leading_key_column_is_skipped_but_never_assumed`); the mutants are equivalent (§9) |
| The `lctx_id` kind is a non-null literal; the result is non-null | `return_field_from_args` (`udf.rs`) | plan time | a plan error | **Tested** (the known answers are unchanged) |
| Fork = tag + one commit; every env read classified | `check_pyrefly_fork.py` | `just deps` | the recipe fails | **passed** this session |
| `[tool.lctx]`/`[tool.lctx.source]` refuse unknown or mistyped keys | serde `deny_unknown_fields` | Stage A | refused, naming the key | **Tested** |

**Absence and uncertainty.** Two of the defects collapse distinct states:
- F1 collapses "Pyrefly pruned this binding" into "Pyrefly analyzed it".
- F2 collapses "this attempt wrote no rows" into "this attempt is not the latest writer".

**Equivalence requirements.**
- Runtime-only commits must be content-equal. The author's id fingerprints and this review's
  rerun multiset probe support that.
- The stored row order is not a contract (O1).

---

## 4. Derivation and execution design

- **Extraction.** C1 threads the module's `SysInfo` into the recognizer. The kind comes from the
  expression tree (`static_kind`), and the decision from Pyrefly. C9 and D6 replace local
  predicates with Pyrefly's.
- **Stage A.**
  - One walkdir pass per tree, with no link followed.
  - globset per key, with hit counts for "each include selects something".
  - Typed definitions.
  - A CSV `RECORD`.
- **Write.** Each table is written once per attempt with zstd, through `DeltaTable::write`. The
  writer runs the input through a partitioned plan and fans the streams into one file writer
  (`write/execution.rs:545-560`, L796-800). So the row order in a file is not the canonical sort
  order, and it differs between reruns (O1).
- **Read.** The version comes from `snapshots`. Then `load_at` → the commit's `Add`s →
  `with_adds` → the `snapshot_id` filter (P3).
- **Validate.** Every registered view is collected into a `MemTable` session, and the rules run
  8 at a time with 8 partitions each. The results are re-sorted by rule index, and cost comes
  from the plan metrics.
- **Coherent publication.** Unchanged: one `snapshots` append after validation.

---

## 5. Representative journeys

- **Interruption or failure (F2).**
  1. Attempt A fails validation, and `lctx compile` prints A's id.
  2. The operator fixes something and compiles again (attempt B, published).
  3. `lctx query --unpublished --snapshot A "SELECT count(*) FROM bindings"` returns 0 rows.

  `snapshot::latest` gives B's versions (`main.rs:424`), and `register` opens only B's commit.
  Reproduced: a store holding two snapshots answered `101359 | 905648 | 1449162` (bindings,
  nodes, edges) for the earlier snapshot through `query`, and `0 | 0 | 0` through
  `query --unpublished`. Before P3 the latest version held every file, and the filter found A's
  rows. The C5 review recorded that as working ("`--unpublished` reads a rejected attempt").
- **Meaningful change (D6, O2).** D6 was a borrow-only accessor with no logic change and no
  typeshed change. It still renamed 1,085 bundled-stub nodes and 91,278 edges, about 6% of the
  edges (verified by the counts), because a bundled module's owner version is the fork revision
  (§3.4.1).
- **Boundary (F5).** Take a library whose `docs/` holds a directory link and whose
  `documents_exclude` has `docs/**/_*` (a common idiom for partials). The probe gives:
  - `covered = true` for `docs/linked`;
  - the include glob would select `docs/linked/page.mdx`.

  So the link's pages drop out of the corpus silently. That is exactly what the rule exists to
  prevent.
- **Ordinary extension (O5).** Suppose a later Pyrefly pin decides a new test, such as another
  `sys` attribute. `static_kind` labels anything that is not type-checking, version or platform
  as `constant`, and the codebook defines `constant` as "a literal". The label would be wrong,
  with no test failing.

---

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| G1 Authority | **pass** (implemented scope) | Static decisions, package, public, docstring and the declared return now have Pyrefly as their one authority. The validation cache derives from the registered views, not a second copy. Rules and contracts are unchanged. **Forward:** DESIGN §6.2 step 3 contradicts §3.2/§6.4 for `embedding_cache` (F3), which is not built yet | F3 before `embedding_cache` lands |
| G2 Semantic fidelity | **fail** | F1: `static_polarity = true` for bindings Pyrefly never analyzes. The probe found 4 of 6 nested bindings wrong; the pilot has 0 today. F2: `--unpublished` returns an empty snapshot for a real attempt | Fix F1 and F2 |
| G3 Validity | **fail** (narrow) | C2's fail-closed rule lets a directory link through when an exclude matches only `<link>/_` (F5, probe). Otherwise C2, C3, C5 and P7 reject at their boundaries, and P3 fails on a missing file | Fix F5 |
| G4 Hidden behavior | **pass** | `git init --template=` removes the last ambient git input. `LCTX_LOG` and `_RJEM_MALLOC_CONF` affect only stderr and memory, and are declared in code, pins and ADR-0016. The validation cache is memoization over the same views, and cannot change what a rule sees | — |
| G5 Consistency and recovery | **pass** | A published read resolves through `snapshots`, and the recorded version is the snapshot's own commit. Tested by the reader tests and by this review's two-snapshot store. Unpublished rows stay invisible. `--unpublished` is an inspection path (F2, under G2 and G7) | — |
| G6 Transformation and reuse | **pass** | P4 keeps the order (tests; the mutants are equivalent). P7 keeps the ids (known answers). P2 and P3 keep the content: all 57 tables have equal multisets across a post-H1 rerun, and the author's H1b id fingerprints agree. D6's identity movement is declared and its counts verified | O2 deferred |
| G7 Truthful capability claims | **fail** (narrow) | DESIGN §4.0 L1066 says `--unpublished` "reads an attempt validation rejected". Since P3 it does so only when that attempt is the latest writer of every table; otherwise it falls back to an empty result (F2). DESIGN §3.2's "whether Pyrefly analyzes … that branch" is labelled Tested, but the test covers per-`if` decisions only (F1) | Fix F2 and F1, or narrow the claims |

---

## 7. Principle findings

Severity order: correctness and authority first, then extension difficulty, then cost and
hygiene.

| # | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | **A decided clause nested in a clause Pyrefly prunes is marked "Pyrefly analyzes this branch".** | DM-42, DM-08, DM-59 · G2, G7 | Each `if` pushes its own clause marks (`lexical.rs:507-516`), and a binding takes the innermost enclosing mark (`lexical.rs:766-772`, `.rev().find(…)`). Pyrefly's binding pass never walks a body it decides false (`binding/stmt.rs:1305-1315`, `abandon_branch(); continue`), so nothing inside it is analyzed. **Probe:** `a` under `if not TYPE_CHECKING: if sys.version_info >= (3, 10):` reads `(version_info, true)`. `d` and `e` in the `else` of `if TYPE_CHECKING:` read `(platform, true)` and `(constant, true)`. `f` under `if sys.version_info < (3, 9): if TYPE_CHECKING:` reads `(type_checking, true)`. Only `b` and `g` are right. The differential test compares each `if` in isolation, and `branches.py` has no nesting. A rough AST scan of the pilot's release, examples, tests and blocks found 0 such nestings | §3.2 names Pass B (§4.2.4) and Pass C (bindings and values) as consumers. A consumer that uses `static_polarity = false` to explain why Pyrefly has no answer for a binding would call these bindings analyzed. It could not tell "pruned" from "analyzed, no answer", and would emit a missing-evidence reason where "another branch" is true. The first library with nested version or TYPE_CHECKING guards is affected | Mark effectively: a binding under any pruned clause is pruned, with the kind of the outermost pruning clause. Simplest: don't push the marks of an `if` that lies inside a pruned range, so the innermost lookup finds the pruning clause. About 5 lines. Declare it as an output change (`EXTRACTOR_OUTPUT_VERSION`) | **test**: extend `static_marks_agree_with_pyrefly_pruning`. Walk each module recursively with `pruned_if_branches` (as Pyrefly's `definitions.rs:933` and `function.rs:1029` do), and assert that every binding's effective polarity equals "inside the analyzed set". Add the probe's nested cases to `lex/branches.py` |
| **F2** | **P3's per-commit read never checks that the commit it opens belongs to the snapshot it filters for.** `--unpublished` for any attempt that is not the latest writer of every table silently returns an empty snapshot. Published reads rest on an unchecked invariant | DM-07, DM-08, DM-14 · G2, G7 | `commit_adds` (`snapshot.rs:69-86`) keeps the `Add` actions and ignores the same commit's `commitInfo`, although `append` writes `lctx.snapshot_id` into every data commit (`delta.rs:163-171`). `--unpublished` takes `snapshot::latest` (`main.rs:424`). **Reproduced** on a store with two snapshots: `0 \| 0 \| 0` against `101359 \| 905648 \| 1449162` rows. `reads_pin_the_version_and_filter_the_snapshot` now asserts the empty read as intended behavior. DESIGN §4.0 L1066 still promises the inspection | (a) An operator who retried a failed compile inspects the failure and sees nothing: an empty result reads as "no rows" (ADDENDUM Q4). (b) A future writer that commits one table twice within an attempt would publish a snapshot whose readers silently see only the last commit, and nothing compares a read with `snapshots.row_count`. Examples: the deferred streaming derive, or a retried raw write | In `commit_adds`, read the commit's `commitInfo` and refuse a commit whose `lctx.snapshot_id` is not the snapshot being registered. `snapshots` stays the authority for *which* version; the metadata only confirms it. For `--unpublished`, locate each table's commit carrying the attempt's `lctx.snapshot_id` (the JSON log is kept), or refuse and name the table. Amend §4.0 to match. About 20–30 lines | **test** (`cpg-core/tests/compile.rs`): attempt A rejected, attempt B published; the `--unpublished` session for A returns A's rows or errors, never 0. A `register` at a version whose commit names another snapshot errors |
| **F3** | **DESIGN §6.2 step 3 makes "read only that commit's files" the rule for every table. §3.2 and §6.4 read `embedding_cache`, which is global and accumulates across attempts, "at its recorded version".** | DM-02, DM-14 · G1 (forward) | §6.2 L1518 has no scope; §3.2 L458 says "Global and append-only; not snapshot-qualified"; §6.4 L1539 says "including `embedding_cache`". Before H1, step 3 was also unscoped ("Filter `snapshot_id`"), but that would have failed loudly: there is no such column. The per-commit rule would instead succeed on the last commit's vectors only | Increment 1's bundle build, following §6.2 through `register`, would see only the vectors the latest cache commit added. Any vector cached by an earlier attempt would be missing: recomputed, or absent from the bundle | Scope step 3 to snapshot-qualified tables. State that `embedding_cache` is read at its recorded version over all active files, with no snapshot filter. Record this in the ADR of F4 | **none today** (not built). When `embedding_cache` lands: a **test** that a vector cached by attempt 1 is visible to attempt 2's bundle |
| **F4** | **Two "amendments" record new decisions, which the ADR skill (step 5) reserves for a superseding ADR. ADR-0009's frontmatter trigger now reads as fired.** | DM-59, DM-60, DM-55 | ADR-0009's amendment changes the reader contract. The version becomes a file selector, not "an audit coordinate and a lower bound". It adds the invariant F2 relies on, and closes the trigger. Yet it says "The publication protocol is unchanged". ADR-0013's amendment records the operator's choice between alternatives: globset's full syntax, and refusing links rather than skipping them. `just adr revisit` still prints ADR-0009's trigger, "Binary columns lack Delta statistics and snapshot filtering becomes a full scan", whose first clause is true today. `ADDENDUM.md` §1 still maps §B4 as "petgraph traversal and `page_rank`" | A later session running `just adr revisit` sees a live trigger and may reopen a closed question. Reading ADR-0009's Decision, it learns the old reader contract. The embedding-cache exception (F3) has no decision record to live in | Supersede ADR-0009 with a short ADR: the per-commit read, its invariant and check (F2), the `embedding_cache` scope (F3), and a new trigger. For ADR-0013, the operator decides whether the C2 selection semantics need their own record. Update the `ADDENDUM.md` §B4 row | **none mechanical**: `adr lint` accepts appended amendments by design. Prose (the ADR skill); `just adr revisit` shows the stale trigger |
| **F5** | **C2's "an exclude covers the link" test under-approximates, so some directory links are skipped silently. `examples` and `tests` have no exclude key, so a link under their prefix cannot be excluded at all.** | DM-07, DM-42, DM-54 · G3 | `library.rs:606`: `covered = exc.is_match(rel) \|\| exc.is_match(format!("{rel}/_"))`. **Probe** (globset 0.4.20, `literal_separator`): with `documents_exclude = ["docs/**/_*"]` (or `**/_*`), `docs/linked` is covered, while `docs/**/*.mdx` would select `docs/linked/page.mdx`. `pick(…, "examples", …, &[])` and `pick(…, "tests", …, &[])` at L668-669 | (a) A library with a symlinked docs directory and an underscore-partials exclude loses those pages from the corpus without an error, against operator decision 2 ("refused, naming it"). (b) A `tests/fixtures/pkg -> ../../src/pkg` link fails the compile, and the only remedy is rewriting the `tests` globs | Count a link as covered only when an exclude matches the link path itself, or matches both a file and a nested file under two unrelated probe names. Either give `examples`/`tests` an exclude key or document the narrowing remedy in `libraries/README.md` | **test**: in `symlinks_in_a_tree_are_refused_unless_excluded`, a `docs/**/_*` exclude with a directory link under `docs/` is still refused |
| **F6** | **O1's default `warn` filter prints 256 known delta-rs warnings per pilot compile, burying the Pyrefly warnings O1 was meant to surface.** | DM-47, DM-50 | `main.rs:482` (`EnvFilter::new("warn")`). This session's compile printed 256 `deltalake_core::writer::stats: Skipping column … because it's a binary field` lines (`writer/stats.rs:220-226`), one per Binary column per write, and nothing else at `warn`. The limit behind them is already documented and designed around (§4.3, P3) | A real Pyrefly or DataFusion warning goes unread in a stderr the operator learns to skip | Default to `warn,deltalake_core::writer::stats=error`, with a comment citing §4.3 | **test**: factor `default_filter()` and assert its directives. Or a stub-store compile test asserting no `writer::stats` line |
| **F7** | **Several measured claims carry the wrong unit or a scope wider than the evidence.** | DM-39, DM-59 | (a) `lctx` prints MiB (`main.rs:324-329`), and this session's peak is 3639 MiB = 3.82 GB = 3.55 GiB. §4.3 L1357, the revision row, ADR-0016 option 4 ("3.64 GB") and `main.rs:22` write MiB/1000 as "GB". The store is 235 MiB (`du -h`), written "235 MB". (b) §4.3 L1375 says "the reported peak **is** the working set". jemalloc also retains freed pages for its decay period; the measurement shows the peak is flat and repeatable, not that it equals live bytes. (c) §4.3 L1380 says "every session plans on `TARGET_PARTITIONS = 8`", but `delta::read_at` builds `create_session()` (`delta.rs:227`). (d) ADR-0012's amendment and the pins row say the patch is 51 changed lines; its diffstat is 42+/12− = 54 (still under 60). (e) C8's commit defers the mutation result to "the H1 disposition". It is in §9 here | Each is small, but these are the numbers ADR-0016's trigger and §4.3's reopen triggers compare against. A 5% unit error and "is the working set" make "exceeds extraction's working set by more than 1 GB" harder to judge | Use the unit `lctx` prints (or convert). Say "tracks the working set". Route `read_at` through `snapshot::empty_session()` or narrow the sentence. Correct 51 → 54. Copy §9's mutation result into the disposition | **prose**; no mechanical oracle for units in documents |
| **F8** | **`futures` became a used direct dependency (P2) with neither an exact pin nor a pins row.** | DM-31 | `Cargo.toml:69` is `futures = "0.3"`, and `tokio = "1"` at L68 predates H1. The H1 plan's rule: every new direct dependency gets `=x.y.z` and a `docs/pins.md` row. `cargo shear` checks use, not exactness | A `cargo update` can move `futures` within 0.3 without a reviewed diff. The O2 probe showed that an unused range pin resolves silently | Pin `futures` (and `tokio`) exactly, and add their rows | **`just` recipe**: a few lines in `check_family.py` (or beside it) that fail on a `[workspace.dependencies]` entry without `=` or without a pins row |
| **F9** | **Document-stage decisions left open for the consuming slices** (Unresolved, not violated). | DM-40, DM-46, DM-59 | §9.5 L1775 runs PageRank "over the usage projection", which §5 does not declare. It names no damping, tolerance, iteration budget, dangling target, or where they are recorded (the analytics-config digest). §9.4's (min,max) aggregation says nothing about keeping the pair → arcs mapping (guidelines §4 and §10; ADDENDUM Q10). §5 L1427's arc order `ORDER BY src, dst, call_site_id` is not shown to be total, though `EdgeIndex(k) == row k` depends on it | Two implementers of §9.5 would build different rankings. A community could not cite the arcs behind it. With duplicate `(src, dst, call_site_id)` the edge indices would differ between runs | Each consuming slice decides and records it: the PageRank parameters and input projection; the pair lineage; the projection's total arc key (for example, ending in `resolution_id`) | **test**, already specified: §9.5's four tests, §9.4's shuffle test, and §5's "shuffled arc rows give an identical adjacency order". Proposed until built |

**Observations** (no finding; §11 Deferred where a trigger applies):
- **O1. The stored row order is the Delta writer's, not the canonical sort's.**
  - Every data file has out-of-order boundaries at 8,192-row chunk edges, and their number
    differs between reruns: `edges` 80 vs 82, `scopes` 1 vs 2 (its first difference is at row
    3,248 = 19,632 − 2 × 8,192).
  - delta-rs fans a partitioned plan into one writer (`write/execution.rs:545-560`).
  - Readers always sort (`read_at`), so no result changes. But §4.3's "Canonicalize" and
    "Derive … sorted canonically and written like a raw table" read as if storage keeps the order.
  - Say that the canonical sort orders the in-memory batch, which is what `read_at` and the
    round-trip tests compare.
- **O2. Keying bundled typeshed modules on the fork revision is coarser than their content.**
  D6's visibility-only patch renamed 1,085 nodes and 91,278 edges.
- **O3. The four C8/P4 mutation survivors are equivalent.**
  - `key.len() > 1` → `==`, `<`, `>=`, and `constant → Ok(false)`.
  - Each either never skips (the same order, slower), or would strip the last key column only
    when every key column is constant, which a total key forbids for two or more rows.
  - The author's C8-time run (24 mutants, 20 caught, 4 unviable) predates P4.
- **O4. The content digest is an input identity**, so its equality alone cannot show "no output
  change":
  - it covers run ids and `compiler_digest`, not rows;
  - §4.3 correctly pairs it with the id fingerprints of 7 tables;
  - this review's multiset comparison of all 57 tables is the stronger oracle, at about 30 lines.
- **O5. `static_kind` defaults to `constant` for any decided test that names no known kind.**
  That is correct at this pin: `evaluate` decides only literals and the three kinds. A pin
  bump could make it wrong silently (§5).
- **O6. Diagnostics under concurrency.**
  - After the first rule error, `validate_costed` returns whichever error finishes first, and
    the other spawned tasks run on until the runtime drops.
  - `Violation.sample` shows whichever batch arrives first.
  - Both are diagnostics only.

### 7.1 Applicability and verdicts

| Group | Bore on H1? | Verdicts |
|---|---|---|
| 1 Authority | yes (C1, C9, D6; F3) | DM-02 **satisfied** for the implemented scope; **unresolved** for `embedding_cache` (F3) |
| 2 Types and invariants | yes (codebook append, typed definitions, UDF) | DM-07 **violated** narrowly (F2, F5); DM-08 **violated** (F1, F2); DM-09 satisfied |
| 3 Identity and consistency | yes (P3, D6) | DM-14 **satisfied** for published reads (F2(b) is an unguarded premise); DM-11/DM-12 satisfied, with D6's movement declared (O2) |
| 4 Composition | only DM-20 (the validation cache) | DM-20 **satisfied**: the cache is built from the registered views and cannot repair or alter them. DM-16–19 not touched |
| 5 Derivation | yes (P4, P7, D6) | DM-22/DM-24 **satisfied** (order and ids preserved; tests; mutants) |
| 6 Effects | yes (C6, O1, P1) | DM-28 **satisfied** (git template removed; `LCTX_LOG` and `_RJEM_MALLOC_CONF` declared). DM-30: F2(a) sits here too |
| 7 Dependencies and concurrency | yes (P2, O2, new crates) | DM-31 **violated** (minor, F8); DM-35 **satisfied** (independent read-only rules, order restored) |
| 8 Performance | yes (P1–P6) | DM-39 **satisfied** with the unit caveat (F7); DM-40 **satisfied** for P2 and P4, **unresolved** for §9.4, §9.5 and §5 (F9) |
| 9 Boundaries | yes (C2, C3, C5, the fork) | DM-42 **violated** (F1, F5); DM-43/DM-44 **satisfied** (plan-time UDF refusal, unknown keys, the fork check) |
| 10 Provenance | yes (static marks, reruns, O1) | DM-46 **unresolved** for §9.4 (F9); DM-47 **violated** (minor, F6); DM-48 **satisfied** (rerun content equal) |
| 11 Verification | yes (C8, the differential test, the meta-test) | DM-51 **satisfied** (append-only codes 3 and 4; `EXTRACTOR_OUTPUT_VERSION` 17); DM-53 **satisfied**, with F1's gap; DM-54 partly (F5); DM-55: F4 |
| 12 Proportionality | yes | DM-56–58 **satisfied**. H1 removed more bespoke code than it added, and added no platform machinery; `cargo shear` has a consumer (O2's probe). DM-59/DM-60 **violated** (minor: F4, F7) |

The two semantic rules from ADR-0015 are held to injected cases (`graph.rs:602-612`) by
`every_rule_is_exercised_or_declared_an_edit_guard`. The count is 496 rules: the rules snapshot
lists 496 names. Every test name cited in DESIGN, the ADRs and `ADDENDUM.md` exists as a
`fn`.

---

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| Baseline (pre-H1) | Hand text matching for static tests, a hand glob matcher and walk, hand TOML and CSV parsing: second authorities beside Pyrefly and the libraries | The leverage review's C1–C9 defects (5 of 10 static tests wrong, links followed, silent misspellings) | — | 45 s, 6.7–8.0 GB, validation 10 s | Replaced |
| H1 as built | One authority per decision (Pyrefly, globset, serde, csv, clap). Validation, reads and the allocator are library features | F1, F2 and F5 are regressions or gaps in the new code; the rest verified | About 2,660+ / 830− lines, including tests; no new crate versions | 29.9 s, about 3.6 GB, validation 0.9 s (reproduced) | Accepted, with F1, F2 and F5 fixed |
| **Simpler viable alternative (reads):** keep filter-only reads, and cache plus concurrency for validation only | P3's invariant (one commit per table per attempt) would not exist, and neither would F2/F3's surface | No `--unpublished` regression, no `embedding_cache` conflict | About 25 fewer lines | The leverage probe: 15.7 ms vs 4.7 ms per read at 200 snapshots. The pilot stores hold one snapshot, so P3 bought nothing measurable in the pilot | Viable until a store holds many snapshots. P3 is still the better long-run choice **if** the commit-ownership check (F2) makes its invariant explicit, at about 10 lines |
| **Simpler viable alternative (C1):** compute the analyzed set by walking `pruned_if_branches` recursively, as Pyrefly's `definitions.rs` does, and take the kind from `static_kind` | Reuses Pyrefly's traversal instead of re-deriving it per clause | Fixes F1 by construction | About the same size | — | Recommended as F1's implementation |

**Abstractions justified by current needs.**
- `cached_session` has a measured consumer: validation, 10.2 → 0.9 s.
- The per-commit provider has a latent one: many-snapshot stores.
- ADR-0011's rejections of `page_rank`, the petgraph adapter and `condensation` remove misuse
  before any code exists.
- `TARGET_PARTITIONS` is one constant with its measurement.

**What remains ordinary code.** The weighted PageRank (about 40 lines), NextClosure and the §5
adapter are specialized algorithms behind declared contracts (charter §F). That is right.

---

## 9. Verification and measurement plan

| Claim or risk | Evidence label | Test / analysis / benchmark | Conditions and expected result | Current result or remaining gap |
|---|---|---|---|---|
| C1 per-clause decisions equal Pyrefly's | **Tested** | `static_marks_agree_with_pyrefly_pruning` | the `lexical_shapes` fixture, 3.14, Linux | passed; **gap: nesting (F1)** |
| C1 effective polarity equals Pyrefly's analyzed set | Proposed | the recursive differential (F1) | nested fixture cases | not written |
| Runtime-only commits change no output | **Tested** (ids of 7 tables, author); **Tested (probe)** for post-H1 rerun determinism across all 57 tables | author: `fingerprint.sh`; this review: the multiset probe | the same inputs and code | equal; pre-H1 vs H1b not reproduced here |
| P3: published reads see exactly their snapshot | **Tested** | `a_pinned_read_opens_only_its_commits_files`, the reader tests, a two-snapshot scratch store | 2 snapshots in one store | passed |
| P3 invariant checked at read | Proposed | the F2 test | a commit that names another snapshot | missing |
| P2: order, only reading, every table read | **Tested** | `violations_come_back_in_rule_order`, `every_table_is_read_by_some_rule`; 6 concurrent green runs | multi-thread runtime | passed |
| P4 keeps the order; C8 id encoding | **Tested** | `tests/hash.rs`; cargo-mutants on `hash.rs`/`table.rs` | scratch clone at `8f8693c` | 23 caught, 4 equivalent survivors (O3), 4 unviable |
| C2 fail-closed on links | **Tested** except for F5 | `symlinks_in_a_tree_are_refused_unless_excluded`; the globset probe | a `docs/**/_*` exclude | gap (F5) |
| D6 moves only producer, run and fact ids and the bundled stubs | **Tested** (counts: 1,085 / 91,278) | SQL on `build/store` | — | byte-identity of the rest: author only |
| Pilot time and peak (§4.3) | **Measured** | `lctx compile fastmcp`, fresh or scratch store | this host, jemalloc | 29.88 s, 3639 MiB, validation 0.95 s; units per F7 |
| Fork = tag + patch; branch pushed | **Tested** | `just deps`; `git ls-remote` | 2026-09-23 | passed |

**Cost accounting.**
- Validation now holds every table in memory. The peak is unchanged, because extraction dominates
  (3639 MiB).
- zstd costs about +0.2 s of raw writes, for −13.6% of store size.

---

## 10. Exceptions and unresolved decisions

- No SHOULD-level exception is claimed.
- F9 is the unresolved set. Each item is owned by the slice that builds its consumer (§9.4,
  §9.5, slice 4's §5 adapter), and none gates H1.

---

## 11. Decision and implementation changes

**Decision: Revise, narrowly.**
- F1, F2 and F5 each fail a gate (G2, G7 and G3) on behavior H1 claims. Each is a small change
  with a named test. Until they land, the matching claims should be narrowed:
  - §3.2: "the innermost clause's own decision";
  - §4.0: "`--unpublished` reads the latest writer's commit";
  - §4.0: the exclude-coverage wording.
- Everything else in H1 is **accepted**:
  - C3–C9, P1, P2, P4–P7, O1–O5 and D6 as built;
  - ADR-0016;
  - ADR-0011 as amended, as a proposed record whose items land with their consumers;
  - P3's published path, with F2's check added.

**Reason.** H1 does what the leverage review asked. It removes second authorities, and its
runtime changes preserve content: a rerun reproduced the content digest, and all 57 tables have
equal row multisets. Its measured gains reproduce. The three defects are new-code edge cases in
C1, P3 and C2. None touches published pilot data: the pilot has no nested static branches, no
links in selected trees, and one snapshot per store. Each can mislead the first consumer or the
first new library.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 (correctness) | F1: effective static marks (recursive pruning) | DM-42, DM-08 | the recursive differential test on nested fixtures | the same test; `EXTRACTOR_OUTPUT_VERSION` bump |
| 1 | F2: `commit_adds` checks `lctx.snapshot_id`; `--unpublished` finds the attempt's commits or refuses | DM-07, DM-08, DM-14 | a two-attempt test | the same test |
| 1 | F5: a strict coverage test for directory links; the remedy for `examples`/`tests` | DM-07, DM-42 | the extended symlink test | the same test |
| 2 (authority and records) | F3: scope §6.2 step 3; F4: supersede ADR-0009 (reader contract, invariant, `embedding_cache`, new trigger); update the ADDENDUM §B4 row | DM-02, DM-59, DM-60 | `just adr lint`; `just adr revisit` no longer lists the closed trigger | — |
| 3 (hygiene) | F6 default filter; F7 units and scope; F8 exact pins | DM-47, DM-39, DM-31 | a compile's stderr has no `writer::stats` lines; `just deps` | the `default_filter` test; the `check_family.py` extension |
| — | F9: decided by the consuming slices | DM-40, DM-46 | their specified tests | — |

### Deferred

| Item | Why not now | Trigger |
|---|---|---|
| O1: stored row order | No reader depends on it; readers sort | Byte-identical data files or key-ordered row-group pruning become a requirement; or streaming derive lands (it writes through the same fan-in) |
| O2: bundled-stub identity by typeshed content digest, not fork revision | A schema-free id recipe change that moves every bundled id once; the only consumer of cross-fork stability would be diffs | The first cross-snapshot relational diff across a fork bump (guidelines §11), or the next fork patch |
| O4: the full-content multiset fingerprint as a repo oracle (`lctx` subcommand or test helper) | The author's id fingerprint plus this review's probe suffice for H1 | The next runtime-only change to derive, write or read (streaming derive, a writer-property change) |
| O5: refuse or relabel a non-literal "constant" static test | Correct at this pin | The next Pyrefly pin bump (ADR-0012's harness run) |
| O6: deterministic error choice and samples in concurrent validation | Diagnostics only | A test or tool compares violation text |

**Final check.**
- The design's claims match the evidence, except F1, F2 and F7, which are named.
- The scope matches the implemented guarantees once §3.2 and §4.0 are narrowed or fixed.
- Slice 4 has a validated path: the §5 adapter recipe is specified and cited to source, F9 names
  what it must still decide, and the reader contract becomes explicit once F2 and F4 land.
