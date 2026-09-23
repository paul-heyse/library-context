# Design review: CPG slice C5, the source corpus (compact)

**Date:** 2026-09-23 · **Depth:** compact · **Mode:** code plus DESIGN.md.
- **What was read.** C5b at `bcfb2e2`, together with all of C5a (`3e223f9`), through
  `git diff 3e223f9~1 3e223f9` and `git diff e27cdb4 bcfb2e2`.
- **Where it ran.** Built, tested and probed in a detached worktree of `bcfb2e2`, with its own
  `CARGO_TARGET_DIR`.
- **Out of scope.** `e27cdb4`, the C4 review's fixes, except where C5 builds on it: the merge of
  `type` nodes and the `type_binders` module-name scoping.
- **Why a review is owed.** The slice adds a fact family, a second extractor run per attempt, two
  derivations and five edge kinds, so ADR-0001 owes a `compact` review.

**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`3e223f9` and `bcfb2e2`.
**Prior reviews:** `design_review_cpg-c2-syntax_2026-09-23.md`, `…-c3-lexical_…` and
`…-c4-types_…`, with their Dispositions. Findings here are F1–F8; observations are O1–O6.

## 1. Decision and scope

**Target.**
- `lctx`: `git`, `fetch_source`, the compile flow, the attempt id print and `query --unpublished`
  (`main.rs`); `crates/lctx/tests/acquire.rs` (the stub `git`).
- `cpg-extract`:
  - `docs.rs`, new, read in full: the parse, passages, code blocks, links, `Vocabulary`, the
    recognizer and `python_blocks`;
  - `library.rs`: `Source`, `source()`, `glob_match`, `select`, `corpus()`;
  - `config.rs`: `ReleaseOrigin::Corpus`, `Release::corpus`, the corpus search path, `context()`
    and `run_id`;
  - `lib.rs`: `run`, `merge`, `run_release`, `CORPUS_FAMILIES`, the documents loop and
    `release_rows`;
  - the `lexical.rs` augmented-assignment change;
  - `pysa_map.rs`'s `ModuleRefs`, read for F2.
- `cpg-schema`:
  - `tables.rs`: the five `docs` tables;
  - `derived.rs`: `mention_targets`, `usage_targets`, the release-scoped `exports` and
    `import_targets`, `external()` DISTINCT, and the first-fact picks;
  - `graph.rs`: the three node kinds, the five edge kinds, `first_fact`, the merged `type` nodes,
    `unique:release-paths` and `unique:type_terms`;
  - `rules.rs`: the per-document coverage and the "modules or documents" reference;
  - `codebook.rs` (the appends) and `id.rs` (the three recipes).
- `cpg-core`: `snapshot::latest`; `tests/syntax.rs` `a_corpus_documents_its_library` and its
  snapshot; `fixtures/python/docs_shapes/`.
- DESIGN at `bcfb2e2`:
  - §3.2 (the `docs`, `lexical` and `types` rows), §3.4 and §3.4.1;
  - §3.8: the C5 edge row, the rules and the C5a/C5b Measured blocks;
  - §4.0 (source and CLI) and the revision rows.
- Also: ADR-0013's two 2026-09-23 amendments, `docs/pins.md` (markdown `=1.0.0`),
  `libraries/fastmcp/pyproject.toml`, and ADDENDUM §5 as committed.
- markdown-rs 1.0.0's `Point` and `ParseOptions::mdx` in the cargo registry source.

**Observable outcome claimed.**
- **The corpus run.** A library's upstream tree at its pinned commit, fetched hermetically, is
  compiled as a second run of the attempt.
- **The `docs` family.** Documents, passages that partition each document, code blocks at any
  depth, and links. Mentions come in two classes that are never merged: `exact` (inline code or a
  dotted prose token that is a public path or a public class's member) and `lexical` (candidates).
- **The usage run.** Examples, tests and every Python code block are materialized as modules and
  compiled with every code family but `exports`. The search path is "the tree, then
  site-packages … so a module both runs import is one file" (§3.2).
- **Linking.** `usage_targets` links each dependency definition that is one of the release's
  installed files to the release's own node.
- **Two runs, one node.** "What both runs assert about one thing (a dependency module or
  definition, a type term and its structure) is one node and one edge, from its first fact"
  (§3.4, L411-413).
- **Named consumers.** Pass C examples and tests; §10.5 usage patterns and §9.4 co-use; exact
  doc links to APIs and extractive brief text.

**Supported scope and non-goals.**
- Embedding-based linking (§9.7) is out of scope.
- `exports` is not run on the corpus.
- Each Python block is its own module.
- The pilot corpus is FastMCP 4.0.5's tree at `004bf15a`.

### Method and coverage

**Checks run in this session** (2026-09-23):

| Check | Command | Outcome | Observation |
|---|---|---|---|
| Rust tests at `bcfb2e2` | `INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass --offline`, in a detached worktree with its own `CARGO_TARGET_DIR` | passed | 78/78, including `a_corpus_documents_its_library`. Slowest: `every_rule_kind_rejects_its_violation`, 246.5 s |
| Lints | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --quiet --offline -- -D warnings` | passed | Same worktree, before any probe file existed |
| Python and repo checks | `uv run pytest -q`, `uv run pyrefly check --summary=none`, `uv run ruff check --quiet`, `uv run ruff format --check`, `ast-grep scan`, `ast-grep test --skip-snapshot-tests`, `uv run python scripts/adr.py lint`, the fixtures `ast.parse` | passed | pytest 21; 50 files formatted; rule tests 4/4; 14 ADRs; 37 fixture files. These are `just check`'s steps run one by one, not the recipe |
| Dependency policy | `uv run python scripts/check_family.py Cargo.lock`; `cargo deny --log-level error check bans sources licenses`; `uv run python scripts/check_pyrefly_fork.py` | passed | `6a93da34 locked, named by the driver and pins, = tag + patch` |
| `lint-agents` | `uv run python scripts/check_agents.py` | blocked in the worktree; passed in the main tree | The worktree lacks the gitignored installed skills |
| `just gold` | `uv run python scripts/check_gold.py` | not_run in the worktree (skill not installed); passed in the main tree | The main tree carries the author's uncommitted C6 edits (`attempt.rs`, `validate.rs`, DESIGN.md, ADDENDUM.md). None of them touches agent files, `scripts/` or `libraries/` |
| Pilot | `target/release/lctx query --store build/store --snapshot 063b8eb3a754ae994dd93978ccbb8127 "…"` | ran (read only) | See the note below. `just pilot`: not_run |
| Probes | a scratch test file in the worktree (deleted, then `git status` clean); a real-git probe in the session scratchpad (deleted) | ran | P-A to P-G below. Nothing was added to the repository |

**The pilot snapshot.**
- The brief's snapshot, `82e00ed6…`, is no longer in `build/store`: neither published nor present
  as unpublished rows.
- The store was rebuilt at 04:21–04:22, after the commit at 04:19:45, by a release binary built at
  04:21:10. It publishes one snapshot, `063b8eb3…`.
- That snapshot has exactly the commit message's 907,845 nodes and 1,452,970 edges.
- **Asserted:** I treat it as the `bcfb2e2` pilot on that count match.

**Pilot queries** (snapshot `063b8eb3…`):
- **Q1 counts: reproduced.**
  - Documents and passages: 148 documents (144 complete; the 4 `snippets/*.mdx` JavaScript
    components are `unavailable` with markdown-rs's "Unexpected end of file in expression");
    1,608 passages.
  - Code blocks: 1,357, of which 960 are Python, with 960 distinct `module_path`s.
  - Links: 5,766, of which 5,031 are in `changelog.mdx`.
  - Mentions: 3,363 in all. Exact: 56 in inline code and 21 in prose. Lexical: 1,021 in inline
    code and 2,265 in prose.
  - Usage modules: 1,512, all complete except 35 that are `types`-partial (125 examples, 427
    tests, 960 blocks).
  - Links to the release: all 1,464 `usage_targets` rows linked; 18,529 `call_target` edges end
    at a linked symbol.
- **Q2 the corpus context.** Its `search_path` is
  `[$release, /home/paul/library-context/build/envs/fastmcp/lib/python3.14/site-packages]`. The
  library's is `[$release]` (F1).
- **Q3 release classes as terms.** `FastMCP` is two terms (F2):
  - `8d57ce0c…`, class `(@fastmcp/server/server.py, 2)`, with 170 observations, all in the
    library run;
  - `8b523ee1…`, class `(fastmcp.server.server, 2)`, with 5,116 observations, all in the corpus
    run.

  352 library-run terms of release classes have a corpus-run twin with the same class module
  name, key and kind.
- **Q4 FastMCP's tree root.** It holds `fastmcp_slim/`, `fastmcp_tasks/` and `fastmcp_remote/`
  project directories. In the corpus context, `fastmcp_tasks.*` still resolves to site-packages
  (`site_packages`, `fastmcp-tasks`). So the pilot is **not** affected by F3.
- **Q5 prose lexical mentions.** 1,370 are "FastMCP" and 568 are "OAuth" (`fastmcp.client.OAuth`):
  1,938 of the 2,265 (O1).
- **Q6 unresolved names in materialized blocks.**
  - 753 of 5,974 references are unresolved (`unresolved_target`), in 387 of 960 block modules.
  - The top names are `mcp` 175, `client` 100 and `FastMCP` 55.
  - Examples and tests have 0 unresolved of 78,637 references (O2).
- **Q7 same-run duplicates.** No `context_definitions` or `type_terms` id appears twice within
  one run (O3).
- **Q8 node share.** 644,682 of 907,845 nodes belong to corpus modules, 342,002 of them syntax
  nodes (O6).

**Scratch probes.** Each is the `a_corpus_documents_its_library` setup (the `docs_shapes`
release installed in site-packages with a `RECORD` owner), plus extra corpus files, compiled with
`cpg_core::attempt::compile` and read through the tables:
- **P-A (flat layout).** The tree also holds `pkg/__init__.py` and `pkg/core.py` (the release's
  own files at the repository root) and no glob selects them. The snapshot **publishes**, every
  rule passing:
  - `pkg` and `pkg.core` resolve to the tree (origin `search_path`, no distribution);
  - there are 0 `usage_targets` rows and 0 `usage_link` edges (F3).
- **P-B (block names).** `docs/a-b.mdx` and `docs/a_b.mdx`, each with one Python block, both get
  `module_path = _lctx_blocks/d_docs_a_b_mdx/block_0.py`.
  - The one file on disk holds the second document's code.
  - Validation rejects the attempt with `key:coverage (5 rows)`, because the path was pushed onto
    the usage list twice (F7).
  - `snapshot::latest` plus `session` (the `--unpublished` path) read the rejected rows.
- **P-C (empty selection).** `documents = ["documentation/**/*.mdx"]` **publishes**. The corpus
  run declares `docs` (`[calls, docs, lexical, signatures, syntax, types]`), with 0 documents and
  0 `docs` coverage rows (F5).
- **P-D (two locations).** The same scenario was compiled under two temp directories.
  - Unchanged: the library run `1c4ce81e…` and the corpus release `931c9f90…`.
  - Different: the corpus context (`03ac9ca4…` against `43eaca5d…`) and the corpus run
    (`60c4e64d…` against `9edeaea7…`).
  - The two `search_path`s read `[$release, /tmp/.tmpEpBMRm/venv/site-packages]` and
    `[$release, /tmp/.tmpsxBVOT/venv/site-packages]` (F1).
- **P-E (recognizer and terms).**
  - `` `httpx.Server.tool` `` and `` `somelib.Server.run()` `` are `exact` mentions, modality
    `definite`, of `pkg.core.Server.tool` and `pkg.core.Server.run` (F6).
  - `Server` is two terms: `ffc04ad9…` (`@pkg/core.py`, 2 `has_type` edges) and `0f03597f…`
    (`pkg.core`, 3) (F2).
- **P-G (git).** Real git 2.43.0, run with exactly `git()`'s environment
  (`GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_TERMINAL_PROMPT=0`, no other
  `GIT_*`), runs `init`, a shallow fetch of one commit and a checkout.
  - The file was committed as `line1\nline2\n`, and `$HOME/.config/git/attributes` holds
    `* text eol=crlf`.
  - The checkout writes `line1\r\nline2\r\n`, while `rev-parse HEAD` still equals the pinned
    commit (F4).

**Read and not attacked** (asserted):
- markdown-rs parse behaviour beyond the fixture and the 4 pilot failures;
- `fetch_source` against a real remote (the stub test and the existing pilot tree only);
- Pyrefly's P4 key agreement beyond the pilot's 1,464 links (the typed rule passing on them);
- the lexical change beyond its two new snapshot rows;
- publication and recovery, which are unchanged;
- performance: the author's 46.3 s and 6.7–7.7 GB were read, not rerun.

## 2. Authority and lifecycle (compressed)

- **Producers.**
  - `documents`: surface `source`, `input_context`, `raw`.
  - `passages`, `code_blocks`, `doc_links`: `markdown-rs`, `native_traversal`,
    `source_observation`.
  - `mentions`: `lctx-docs`, `recognizer`, `derived_analysis`; `definite` for a single exact
    target, `candidate` otherwise.
  - `mention_targets`, `usage_targets`: Stage D.
  - The catalogs: generated from the registry.
- **Our rules on markdown-rs rows.** Two values are ours, both deterministic functions of the
  tree and the document path: the passage boundaries (a root-level heading to the next) and
  `code_blocks.module_path`. They are structural, like C2's placement. No provider's inference is
  relabelled.
- **Two runs per attempt** (ADR-0013 amended twice). The library run and the corpus run cover
  distinct releases (`key:releases` rejects a repeat) in distinct contexts. `merge` concatenates
  them and writes the shared producer once (`lib.rs:287-321`).
- **Identity.**
  - **Corpus release id.** `H(corpus, label, n, (path, digest)…)` (`config.rs:104-126`). It hashes
    content and is location-independent (P-D).
  - **Corpus context id.** It hashes the search-path strings (`config.rs:337-353`), and one of
    them is absolute (F1).
  - **Corpus run id.** `H(release, context, producer, families)` (`config.rs:400-409`). That
    excludes the library release, whose public names are the recognizer's vocabulary (F1).
  - **Document, passage and code block.** `H(document, release, path)` and
    `H(passage|code_block, document, ordinal)`, recomputed by the `id:` rules.
- **The fetched tree.**
  - It is fetched once into `<commit>.partial` and then renamed. `rev-parse` runs on every
    compile.
  - The compile clears and rewrites `_lctx_blocks/` inside it. The pilot tree's
    `git status --porcelain` shows only `?? _lctx_blocks/` (O4).

## 3–4. Contracts and derivation (merged)

| Invariant | Enforcement | Evidence |
|---|---|---|
| The selection is the declared globs (`**` spans directories, `*` one name) | `glob_match`, `select` | **Tested**: the `globs_span_directories_only_with_double_star` unit test; the fixture's `docs/old/**` exclusion |
| A declared family is not complete over nothing | the reference "a run's release has modules **or** documents" (`rules.rs:46-51`) | **Violated** per family: P-C (F5) |
| Offsets are bytes | slicing `text[start..end]`, which would panic on a character offset past non-ASCII text | **Tested**: the fixture's non-ASCII line precedes two later passages; author's P5. markdown-rs's own doc says "character" (`unist.rs:15`), so the slicing is the evidence |
| A document's passages partition its body | preamble plus root-level headings (`docs.rs:448-498`) | **Tested**: spans 27-95, 95-275, 275-489 and 489-511 in the snapshot |
| A materialized block is its code block's module | `block_module_path` plus the `block_module` lineage | **Tested** (960/960 on the pilot). The name map is not injective, and the failure is closed only by accident (P-B, F7) |
| `exact` means the text names the API | `recognize` (`docs.rs:191-243`) | **Violated** for a dotted prefix it never reads (P-E, F6) |
| Every mention and every release-owned dependency definition has a target | `typed:mention_targets`, `typed:usage_targets` | Pilot passes; no injected case (F7) |
| A module both runs import is one file | the search path: tree, then site-packages (`config.rs:219-226`) | Holds on the pilot (Q4). **Violated** silently for a flat layout (P-A, F3) |
| What both runs assert is one node | `first_fact`, the merged kinds, `unique:type_terms` | Holds for dependency terms. The library's own classes are two terms (Q3, P-E, F2). `first_fact` never checks that the duplicate comes from another run (O3) |
| Identity never depends on location | `relativize` for the config JSON; `relative(…, release root)` for search paths | **Violated** for the corpus context (Q2, P-D, F1) |
| The fetch is hermetic | `GIT_*` removed, no system or global config (`main.rs:136-151`); the stub test | **Violated**: attributes files (P-G, F4) |

**Absence.**
- **Explicit:** an unparsable document (`unavailable`, with markdown-rs's message); an unresolved
  name in a block (`unresolved_target`); an untyped subject (`types` boundary).
- **Not recorded:** a selection that matches nothing (F5); a release shadowed by the tree (F3).

## 5. Journey: do the named consumers get what they read?

- **Pass C, examples and tests: calls served, types not joinable across runs.**
  - Usage calls reach the release: 18,529 `call_target` edges end at a linked external symbol, and
    `usage_link` covers all 1,464.
  - A usage argument's type and a library parameter's declared type are different terms for the
    same class (F2). The top-level class can be reconciled through `type_class` → external symbol
    → `usage_link`. A composite term (`list[Tool]`, `FastMCP | None`,
    `Callable[[Context], …]`) cannot.
- **§10.5 usage patterns and §9.4 co-use: served for examples and tests, thin for doc blocks.**
  - 387 of 960 block modules read a name an earlier block bound (`mcp` 175 times), so a
    continuation block's `@mcp.tool` has no receiver type and no target (O2).
  - That is explicit (`unresolved_target`), not silent.
- **Exact doc links to APIs: correct on the pilot, unsafe as a class.**
  - All 77 exact mentions were read, and each names the element it says. No member form carries a
    prefix.
  - A prefixed form (`httpx.Server.tool`) becomes a `definite` exact link to this library (F6).
  - The generated API reference (`python-sdk/`) is excluded, so no hyperlink resolves to an API
    element. Hyperlinks are recorded with no consumer yet (O5).
- **Extractive brief text: served.**
  - Passages carry raw text, level and heading path.
  - Prose `lexical` candidates are 86% the product and protocol names (O1).
- **A future library.** Two ways the corpus goes empty with every rule passing: a flat layout (F3)
  and a selection that no longer matches (F5).

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **pass** | One producer per table. The recognizer's rows are labelled `lctx-docs` / `recognizer` / `derived_analysis`, the joins are Stage-D derivations, and the catalogs are generated. The passage and block-path rules on markdown-rs rows are structural. Two runs cannot write one release (`key:releases`) | — |
| **G2** Semantic fidelity | **fail** | The library's own types are two terms depending on the observing run (F2; Q3, P-E). A flat-layout tree silently replaces the release in the usage run (F3; P-A). A prefixed member form is a `definite` `exact` mention of this library (F6; P-E) | F2, F3, F6 |
| **G3** Validity | **fail** (narrow) | The "modules or documents" reference exists so that "`coverage:complete` cannot pass vacuously" (`rules.rs:46`), but it passes vacuously per family (F5; P-C). No C5 rule has an injected-violation case. `unique:type_terms`, now the only guard against a term-id collision, is among them (F7) | F5, F7 |
| **G4** Hidden behaviour | **fail** (narrow) | The checkout reads the operator's and the system's git attributes files, which change the corpus bytes under an unchanged commit (F4; P-G). No other ambient read was found: the search path, site-packages and the vocabulary are explicit inputs | F4 |
| **G5** Consistency and recovery | **pass** | Publication is unchanged. Both runs go through one attempt and one validation, and a failed corpus extraction fails the whole extract. `--unpublished` only reads, and P-B used its path on a rejected attempt. The `.partial` rename makes the fetch atomic. The `_lctx_blocks` rewrite is a cache refresh (O4) | — |
| **G6** Transformation and reuse | **fail** | The corpus run's identity is not a function of its inputs. It includes an absolute site-packages path, so one input set gives two run ids and two `content_digest`s by location (P-D). It excludes the library release whose names the recognizer reads (F1). `EXTRACTOR_OUTPUT_VERSION` 11 → 12 (C5a) and 13 → 14 (C5b; 13 is the C4 fix) moves `producer_id` correctly | F1 |
| **G7** Truthful capability claims | **fail** | Contradicted claims: "never changes an identity" (§4.0 L982, F1); "one node … a type term" (§3.4 L411-413, F2); "a module both runs import is one file" (§3.2, F3); "nothing ambient steers what is fetched" (`main.rs:137`, F4); "cannot pass vacuously" (`rules.rs:46`, F5); `exact` as "a public class's member" (F6). Stale lines (F8) | F1–F6, F8 |

## 7. Findings

### 7.1 Findings

| # | Finding | Principle IDs | Evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | The corpus run's identity is not a function of its inputs. (a) It **includes** where the environment sits: the corpus search path's second entry, site-packages, lies outside the tree, and `context()` relativizes search paths against the release root only. (b) It **excludes** the library release, whose public names and declarations are the recognizer's vocabulary | DM-11, DM-31, DM-48 · G6, G7 | (a) `config.rs:223-226` puts `input.site_packages` on the corpus search path. `config.rs:337-340` reads `.map(\|p\| relative(p, &input.release.root, "release"))`, and `relative` returns an outside path unchanged (`config.rs:251`). `config.rs:353` hashes it (`.strs(search_path…)`), then `run_id` (`lib.rs:346-351`) and `content_digest` (`attempt.rs:229`). Q2 shows the pilot row; P-D shows two locations giving two corpus contexts and runs over one corpus release. The claim contradicted is DESIGN §4.0 L979-985: "paths **relative to their roots** … Where a checkout, tempdir or environment sits never changes an identity … two environment paths give one `content_digest`". (b) `lib.rs:268-282` passes the library run's `Vocabulary` into the corpus run, but `run_id` hashes `(release, context, producer, families)` (`config.rs:400-409`). The `[tool.lctx] release` list enters neither the environment nor the lock digest | (a) The same pin compiled from a second checkout or `--envs` directory gets a new corpus context, run id and fact ids for the whole corpus side (71% of nodes), and a new `content_digest`, so "compares reruns" (§3.4.1) reports a change that is not one. (b) Adding a distribution to `release` changes the corpus's `mentions` under an unchanged corpus run id: two different outputs claim to be one run | (a) Relativize each search-path entry against the environment root too, as `relativize` already does for the JSON (a few lines). (b) Hash the library's `release_id` into the corpus release or run id (two lines). Extend the §4.0 location test to a corpus | **test:** P-D as a case (the `docs_shapes` corpus under two temp dirs gives one corpus context and run id), plus a changed release list giving a different corpus run id |
| **F2** | The library's own types are different terms depending on which run observes them. A class pair's module reference is run-relative (`@path` for a file of **this run's** release, the module name otherwise), and the term id hashes it. So each release class, and every term containing one, exists once per run | DM-15, DM-11 · G2, G7; guidelines §12 (mixed configurations) | `pysa_map.rs:108-117`: `match self.release_files.get(&id) { Some(path) => format!("@{path}"), None => … name }`. The library run writes `(@fastmcp/server/server.py, 2)`; the corpus run, where that file is a dependency, writes `(fastmcp.server.server, 2)`. **Q3:** 170 against 5,116 observations; 352 library class terms have a corpus twin. **P-E:** `Server` is two terms. Claims contradicted: §3.4 L411-413 ("… a type term and its structure) is one node"); `graph.rs:1294` ("One structure is one term, whichever runs observe it"); the ADR-0013 second amendment ("What the two runs both assert (a dependency module or definition, a type term) is one node") | Pass C compares a usage argument's term with the library parameter's declared term, and the ids differ for identical types. The top-level class can be reconciled through `type_class` → external symbol → `usage_link`; composite terms (`list[Tool]`, unions, callables) have no bridge. Shared-type communities split the library's types by run. The `docs_shapes` fixture never asserts a type across runs | In the corpus run, give `ModuleRefs.release_files` each release-owned installed file (from `environment_library().owners`, release distributions only) under its site-relative path, which is the library run's `source_files.path`. `unique:release-paths` already makes that path name one file. The terms then coincide and merge, and `unique:type_terms` guards their displays. This is §8's simpler alternative | **test:** in `a_corpus_documents_its_library`, one `class_instance` term for `Server` (`SELECT count(DISTINCT node_id) FROM type_terms WHERE display = 'Server' AND kind = 0` is 1), with a `type_class` edge to `pkg.core.Server` |
| **F3** | When the package sits at the repository root (a flat layout), the usage run reaches the tree's copy instead of the release, and nothing says so. The search path puts the tree first, and `usage_targets` only links site-packages files owned by a release distribution | DM-42, DM-43, DM-08 · G2, G7 | `config.rs:219-226`: "the tree first, then the environment's site-packages … So a module both runs import is the same file". `derived.rs:888-892`: `owned` needs `m.origin = {site} AND m.distribution IS NOT NULL`. **P-A:** `pkg` resolves to the tree (`search_path`, no distribution); 0 `usage_targets`, 0 `usage_link`; the snapshot publishes. `typed:usage_targets` passes because it has no rows. FastMCP is unaffected (Q4). The claim contradicted is §3.2 `docs`: "so a module both runs import is one file" | For such a library, every usage call ends at unowned search-path modules. Pass C, §10.5 and §9.4 read no usage of the release, while the corpus run reports complete coverage and every rule passes. ADR-0013 makes "every analyzed library" data, so the next library can hit this | Fail closed first. A rule: every corpus-context module whose name is a module of the library release is a `site_packages` file owned by a release distribution; about 10 SQL lines. Then decide the search-path order for the release's top-level packages (§10) | **test:** P-A as a `docs_shapes` variant, expecting the new rule |
| **F4** | The source checkout reads ambient git attributes, so the corpus bytes depend on the operator's and the system's git configuration although the commit is pinned | DM-28, DM-31 · G4, G7 | `main.rs:136-151` removes `GIT_*` and sets `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL=/dev/null`. It sets neither `core.attributesFile` (whose default is `$XDG_CONFIG_HOME/git/attributes` or `~/.config/git/attributes`) nor `GIT_ATTR_NOSYSTEM`. `rev-parse HEAD` (`main.rs:207`) proves the commit, not the working tree. **P-G:** with exactly this environment, `* text eol=crlf` in a home attributes file checks out `\r\n`. The claims contradicted are `main.rs:137` ("so nothing ambient steers what is fetched") and §4.0 ("fetches the tree hermetically"). The stub test (`acquire.rs:138`) cannot see attributes | Two machines compile different corpora from one pin: every passage and code block gains `\r`, byte offsets shift, and the corpus release id differs, while the label still reads `repository@commit`. The release id hashes content, so nothing is wrongly reused | Pass `-c core.attributesFile=/dev/null` (and `-c core.autocrlf=false`), and set `GIT_ATTR_NOSYSTEM=1`. Optionally check `git status --porcelain` after checkout, with `_lctx_blocks` excluded through `.git/info/exclude`. A few lines | **test:** the stub `git` test asserts `GIT_ATTR_NOSYSTEM=1` and the `-c` arguments |
| **F5** | A corpus selection that matches nothing publishes a `docs` family that is complete over zero documents. The same holds for the code families when only documents are selected. The rule meant to prevent vacuity works per release, not per declared family | DM-07, DM-08, DM-43 · G3 | `library.rs:468-477`: a missing, misspelled or non-array key becomes `[]` (`.and_then(\|v\| v.as_array())… .unwrap_or_default()`). `select` returns an empty set silently. `rules.rs:46-51`: "A run's release has modules or documents, so `coverage:complete` cannot pass vacuously". The `docs` arm of `coverage:complete` expects one row per document (`rules.rs:279-284`). **P-C:** it publishes, the corpus run declares `docs`, and there are 0 documents and 0 `docs` coverage rows | An upstream move of `docs/` at the next upgrade, or `documents = "docs/**/*.mdx"` written as a string, publishes a corpus with no documentation and every rule passing. A brief built on it states that the library has no docs | At Stage A: each declared include glob matches at least one file, and `[tool.lctx.source]` keys are arrays of strings, with unknown keys refused. In the rules: a run declaring `docs` has a document, and a run declaring a code family has a module. About 15 lines | **test:** P-C, expecting the Stage-A error; a mutation case (every `documents` row dropped) whose violations include the per-family rule |
| **F6** | The `exact` member recognition reads only the last two segments of a dotted form, so another library's same-named class member becomes a `definite` exact mention of this library's API | DM-06, DM-42 · G2 | `docs.rs:216-225`: `let tail = segs[segs.len() - 2..].join(".")`, looked up in `members`, with every earlier segment ignored. `docs.rs:576`: a single match is `Modality::Definite`. **P-E:** `` `httpx.Server.tool` `` gives `exact`, `definite`, `pkg.core.Server.tool`. Pilot: none of the 77 exact mentions has a prefix (read). The claims contradicted are §3.2 ("`exact` for … a public class's member") and the `mention_class` doc | A docs page that compares APIs (`httpx.Client.get`, or `mcp.server.fastmcp.FastMCP.tool` in a migration guide) links definitely to this library's `Client.get` or `FastMCP.tool`. "Exact doc links to APIs" is the consumer that trusts `exact` | With a prefix, require `prefix.Class` to be an access or origin path of that class (in `paths`); otherwise record nothing, or a lexical candidate. About 5 lines | **test:** a `docs_shapes` line with `` `other.Server.tool` ``, and no exact row for it in the snapshot |
| **F7** | No C5 rule has an injected-violation case, and one C5 guarantee holds only by accident | DM-60, DM-54, DM-11 · G3 | The C3 and C4 reviews made injected cases the practice (DESIGN §3.8 L785 and L792-793; `every_rule_kind_rejects_its_violation`, `the_types_rules_reject_their_violations`). `crates/cpg-core/tests/` has no case for `id:documents`, `id:passages`, `id:code_blocks`, `typed:mention_targets`, `typed:usage_targets`, `unique:release-paths`, `unique:type_terms`, the `docs` arm of `coverage:complete`, or the `block_module`, `usage_link` and first-fact lineages. `unique:type_terms` is now the **only** guard against a term-id collision: C5b merges `type` nodes, so `key:nodes` no longer sees one (`graph.rs:1286-1297`). Separately, `block_module_path` (`docs.rs:357-363`) maps each non-alphanumeric character to `_`. **P-B:** two documents share one module file, and the attempt fails only because `corpus()` pushed the path twice (`library.rs:584`) | A regression in these rules publishes silently. Example: a Pyrefly bump that changes one term's display, which now merges instead of failing. A document-name collision surfaces as `key:coverage (5 rows)` and names neither document | Add the cases to `every_rule_kind_rejects_its_violation` on `docs_shapes`, including a doctored `type_terms.display`. Make block module names injective (a short digest of the document path in the directory name), or refuse a duplicate path in `corpus()` with both documents named | **test:** the cases; P-B's pair either compiles or fails naming both documents |
| **F8** | DESIGN and code lines are stale after C5b | DM-59, DM-55 · G7 | (a) §4.0 L911-912: "and it runs in the library's context". C5b gave the corpus its own context (§3.4; ADR-0013's second amendment). (b) `config.rs:54-55`: "(so the context is the library's)". (c) §3.2 `types` row: "so `key:nodes` rejects the snapshot"; since C5b that check is `unique:type_terms` (`types.rs`'s doc was updated, DESIGN was not). (d) §3.4.1 L544-547: the list of merged kinds omits `type`. (e) §3.2 `lexical` row: "`references` (every name load …)", but an augmented assignment's target, a store, is now also a reference. (f) The claims F1–F6 contradict | A reader expects one context per attempt and `key:nodes` as the collision guard, and cannot find the augmented-assignment references in the contract | Correct the lines; relabel (f) until F1–F6 land | prose (no mechanical oracle for spine wording), plus F1–F7's tests |

**Observations** (none moves the three severity classes today; each names where it would land):

| # | Observation | Evidence | Oracle |
|---|---|---|---|
| O1 | Prose `lexical` candidates are dominated by the product name and a protocol name. "FastMCP" gives 1,370 and "OAuth" gives 568 (`fastmcp.client.OAuth`) of the 2,265. `distinctive` admits any word with two capitals and a lower-case letter. They are labelled `candidate`, so nothing is mislabelled. But §9.4 co-mention would link every OAuth passage to the `OAuth` class | Q5; `docs.rs:172-177` | when §9.4 lands: a known-answer co-mention test, or excluding the library's own name and non-code prose from co-mention |
| O2 | Each Python block is compiled alone, so a block that continues an earlier one has unresolved names: 753 of 5,974 references in 387 of 960 block modules (`mcp` 175, `client` 100, `FastMCP` 55), against 0 in examples and tests. The gap is explicit (`unresolved_target`). A per-document module that concatenates the blocks in order would resolve most of them, at the cost of one syntax error making the whole document's module partial | Q6 | when §10.5 reads doc blocks: a `docs_shapes` document with two blocks, the second using the first's `mcp` |
| O3 | `first_fact` counts any second row of an id as "the other run's fact", including a second row from the same run. Before C5b, such a same-run row failed `key:edges`. Today it is guarded by Pysa's per-module key uniqueness (`tables.rs` `context_definitions` doc) and the `id:` rules, and the pilot has 0 same-run duplicates. Only `type_terms` has a cross-run agreement rule | Q7; `graph.rs:1237-1243` | a `unique:context_definitions` rule (one row per symbol per run), if a provider change ever makes keys non-unique |
| O4 | The fetched tree doubles as a scratch area. The compile rewrites `_lctx_blocks/` inside it, so two concurrent compiles of one library race. `select` follows directory symlinks, so a loop would recurse without bound (none in FastMCP's selected directories). `lctx acquire` fetches without Stage A's `commit` check, which runs only in `compile` and after the fetch. A non-hex value then fails at `rev-parse`, which fails closed | `library.rs:568-585`, `library.rs` `select`, `main.rs:415-419` and `main.rs:224-232` | materialize blocks under `build/`, keyed by the corpus release, if the tree is ever shared or verified with `git status` (F4) |
| O5 | `doc_links` records markdown links only: MDX components' `href`s are not links (17 in the selected pages by a single-line grep). 5,031 of 5,766 links come from `changelog.mdx`. No consumer reads links yet, and none resolves to an API element, since `python-sdk/` is excluded | Q1 | when a consumer reads links |
| O6 | Cost. C5b makes corpus modules 71% of the graph (644,682 of 907,845 nodes; 342,002 syntax nodes) and doubles peak RSS (3.91 → 7.7 GB, in validation). C6's measurement owns the streaming decision (§4.3) | Q8; author's Measured block | C6 |

**Checked and clean:**
- **Ordinals.** `python_blocks` and `document` enumerate one `collect` walk, so each materialized
  path is its row's `module_path` (960/960 distinct on the pilot; the `block_module` lineage).
- **Two mention classes.** `exact` and `lexical` stay apart, and a span with several exact targets
  is `candidate`. Without this, a bare `Context` would be a definite link to one of several
  classes.
- **Release-scoped joins.** The module-name joins in `exports`, `import_targets` and the stub
  `declared_in` edge are scoped by release. Without that, a corpus module that shares a library
  module's name would join the wrong release.
- **`external()` is DISTINCT.** A dependency definition that both runs describe does not double
  its targets.
- **Every link is typed.** All 1,464 dependency definitions that are the release's installed
  files map to a release node. So if a Pyrefly bump changed Pysa keys for a file seen as a
  dependency, the compile would fail rather than silently drop usage links.
- **Codebooks.** Appended only: `fact_family` 10, `scope_kind` 3, node kinds 16–18, edge kinds
  31–35, `mention_class`, `mention_source`.
- **Hermeticity.** The corpus release id is location-independent (P-D), and `GIT_*` removal is
  asserted by the stub test.
- **Coverage.** An unparsable document is `unavailable` with markdown-rs's message.
- **Lexical.** An augmented assignment's read is a reference in the reading scope (the two new
  snapshot rows), consistent with C3's flow-insensitive model.
- **`--unpublished`.** It reads a rejected attempt without touching published reads (P-B).

### 7.2 Applicability and verdicts

- **Bore on this scope:**
  - Group 3, identity: F1, F2, F7;
  - Group 2, semantic types and absence: F3, F5, F6;
  - Group 6, effects and ambient inputs: F4;
  - Group 7, dependencies and reuse: F1;
  - Groups 11–12, regression controls and claims: F7, F8.
- **Did not bear:**
  - Group 1, authority: G1 passes; one producer per table.
  - Group 4, declarations and registry: the entries follow C1's shape.
  - Group 5, derivation contracts: `usage_targets` and `mention_targets` follow the C3/C4
    no-catch-all rule, since their typed rules reject a null with no reason.
  - Group 8, performance: the only claim is the author's Measured block, and O6 defers it to C6.

| Verdict | Principles |
|---|---|
| Satisfied | DM-02 (one producer per table), DM-09 (endpoint kinds and node columns generated for the new kinds), DM-13 (the recognizer's output labelled as ours; joins are derivations), DM-40 (sorted batches; the corpus release id location-independent), DM-51 (codebooks appended; commits labelled as schema migrations), DM-52 (rules and catalogs generated) |
| Violated | DM-11, DM-31, DM-48 (F1); DM-15 (F2); DM-42, DM-43 (F3, F6); DM-28 (F4); DM-07, DM-08 (F5); DM-06 (F6); DM-54, DM-60 (F7); DM-59 (F8) |
| Unresolved | DM-58: whether the Stage-D bridge (`usage_targets`, first-fact dedup, a second set of external nodes for the release) or naming release files by `@path` at extraction is the smaller design (§8, §10) |

**Guidelines MUSTs (ADDENDUM §5, as committed), for C5:**
- **§2 edge identity, typed endpoints, evidence and snapshot scope: met.** `mentions` is parallel,
  with `start_byte` as its discriminator; `block_module` and `usage_link` are one per evidence
  row.
- **§2 isolates: met.** Documents, passages and code blocks have their own existence sources.
- **§2 distinguishable derivations and explicit unknowns: partly.** Exact and lexical are kept
  apart, but F6 mislabels. The omissions in F3 and F5 are not recorded.
- **§7 partial ≠ complete: partly** (F5).
- **§11: met.** Publication is unchanged.
- **§12 known-answer shapes: partly.** `docs_shapes` covers selection and exclusion, frontmatter,
  a code block inside a component, links, both mention classes, an unparsable page, a usage test
  and a block. It lacks:
  - a flat layout;
  - a prefixed member;
  - an empty selection;
  - colliding document names;
  - a continuation block;
  - a type observed by both runs (the "mixed configurations" shape).

## 8. Alternatives (compressed)

| Alternative | Duplication and extension locality | Risks | Cost | Performance evidence | Verdict |
|---|---|---|---|---|---|
| Current: two runs; the release reached as a dependency; a Stage-D bridge for definitions (`usage_targets`, `usage_link`); first-fact dedup; merged node kinds | Each release definition the corpus touches exists twice, as a declaration and as an external symbol (1,464 on the pilot). Types get no bridge (F2) | F1–F7 | C5a +2,066/−86 lines; C5b +627/−127 | **Measured** (author's log, 2026-09-23; not rerun): 46.3 s, corpus run 17.8 s, peak 6.7–7.7 GB in validation | Selected by the author |
| Current with F1–F7 corrected in place | Same structure; the types bridge has to be added as another derivation | O-items remain | about +60 lines and 6 fixture shapes (**Proposed**) | same order (**Proposed**) | Minimum for Accept |
| **Simpler viable: the corpus names the release's files as the release** | In the corpus run, `ModuleRefs.release_files`, the one place a module reference is written (`pysa_map.rs:108-117`), also maps each release-owned installed file to `@<site-relative path>`. That is exactly the library run's `source_files.path`, and `unique:release-paths` already makes it name one file. Pysa call targets, class pairs and ancestors from usage code then resolve through the existing `@path` joins in `call_targets`, `type_class_targets` and `ancestry_targets` to the release's own nodes; P4 already shows that the keys agree. This removes `usage_targets`, the use of `usage_link` (its code stays reserved) and the duplicate external symbols and modules for the release. It fixes F2 by construction, and it makes F3 detectable: a release module name in the corpus context without an `@path` | `import_targets` needs "own release, then the library release" for a corpus import. Raw corpus facts reference library nodes, as `usage_link` already does through Stage D | about +15 extractor lines, −45 derived SQL lines (**Proposed**; the chokepoint read, not built) | fewer nodes and edges (**Proposed**, unmeasured) | Recommended. It removes machinery rather than adding a types bridge, and it is cheaper to take before C6 measures the graph |

**What stays ordinary code:** the markdown walk, the recognizer and the glob matcher. None needs a
declarative layer. Each fix is a local branch, a column or a rule.

## 9. Top verification gaps

| Claim or risk | Label now | Check | Expected result | Gap |
|---|---|---|---|---|
| Identity never depends on location | **Implemented**; contradicted for the corpus (P-D) | the two-location case | one corpus context and run id | F1 |
| One type, one term, whichever run observes it | **Implemented** for dependency types; contradicted for release types (Q3, P-E) | a fixture term count | one `Server` term with a `type_class` edge to the release | F2 |
| The usage run reaches the release | **Measured** on FastMCP (1,464/1,464); silently empty for a flat layout (P-A) | the P-A variant | the new rule fails | F3 |
| The fetch is hermetic | **Tested** with a stub that cannot see attributes; contradicted by real git (P-G) | the stub asserts the attribute settings | `GIT_ATTR_NOSYSTEM=1` and the `-c` arguments are present | F4 |
| A declared family is never complete over nothing | **Implemented** per release; contradicted per family (P-C) | P-C and the mutation case | a Stage-A error; the per-family rule fails | F5 |
| `exact` means the text names the API | **Tested** on unprefixed forms only | a prefixed form | no exact row | F6 |
| Each C5 rule rejects its violation | not claimed; no case | the mutation cases | each named rule fails | F7 |

## 10. Exceptions and unresolved decisions

No SHOULD-level exception is requested. Decisions the author has to make:
- **F2 and §8.** Bridge types in Stage D, which extends the current design, or name the release's
  installed files `@path` in the corpus run (recommended). The second removes `usage_targets`
  and changes the corpus's call, type and ancestor facts, which is a migration (DM-51). It is
  cheapest before C6.
- **F3.** Detect and fail only, or also put the release's top-level packages ahead of the tree on
  the corpus search path. The config comment says site-packages goes on the search path at all so
  that its stubs outrank Pyrefly's bundled ones; any reordering must keep that.
- **F1(b).** Whether the vocabulary dependency enters the corpus **release** id ("the corpus of
  this library release") or the corpus **run** id.
- **F5.** Refuse at Stage A (recommended), add the per-family rule, or both.

## 11. Decision

**Decision: Revise** (small surface).

**Reason.** Most of the slice is well built:
- the fetch is pinned and atomic;
- the corpus release is content-addressed;
- documents, passages and blocks come from one markdown-rs walk, with byte offsets;
- exact and lexical mentions are never merged;
- release-scoped joins and `unique:release-paths` protect the new two-run attempt;
- every usage link is typed, and all 1,464 hold on the pilot.

But:
- The corpus run's identity depends on where the environment sits, and misses the vocabulary it
  reads (F1).
- The library's own types split by observing run, against the slice's own "one node" claim (F2).
  That matters most to Pass C, the first consumer.
- The usage run can silently lose the release for a flat-layout library (F3), and a selection
  that matches nothing publishes as complete (F5).
- The fetch is not hermetic against git attributes (F4).
- `exact` can name a foreign API (F6).
- No new rule is exercised by an injected violation (F7).

Each fix is local. F2's is best taken as the §8 alternative, before C6 measures a graph that it
shrinks.

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | F1: relativize the corpus search path; hash the library release into the corpus identity | DM-11, DM-31, DM-48 | pilot: the corpus `contexts.search_path` has no absolute entry | test: the two-location case and the release-list case |
| 2 | F2: release files named `@path` in the corpus run (§8), or a types bridge | DM-15, DM-11 | pilot: one `FastMCP` class term; 0 release classes with a twin | test: the fixture term count |
| 3 | F3: the shadowing rule, then a decision on search-path order | DM-42, DM-43 | pilot unchanged (Q4) | test: the P-A variant |
| 4 | F4: attributes disabled for git | DM-28 | — | test: the stub asserts the settings |
| 5 | F5: Stage-A glob and type checks; the per-family rule | DM-07, DM-08 | pilot unchanged | test: P-C and the mutation case |
| 6 | F6: the prefix checked before `members` | DM-06, DM-42 | pilot: 77 exact mentions unchanged | test: `other.Server.tool` in `docs_shapes` |
| 7 | F7: injected cases for every C5 rule; injective block paths | DM-60, DM-54 | — | test: the mutation cases and P-B's pair |
| 8 | F8: correct the lines | DM-59 | — | prose |

### Deferred

| Item | Why not now | Trigger that reopens it |
|---|---|---|
| O1: prose lexical precision | Candidates by design; no co-mention consumer yet | §9.4 co-mention lands |
| O2: continuation blocks compiled alone | Explicit as `unresolved_target`; a design choice | §10.5 reads doc blocks, or a Pass C example needs a block's receiver |
| O3: `first_fact` does not check that duplicates come from distinct runs | Guarded by Pysa's per-module key uniqueness; pilot 0 | A provider change, or a same-run duplicate on any pilot |
| O4: the tree as a scratch area, symlinks, validation order | Single operator; fail-closed or content-hashed | Shared or concurrent compiles, or F4's `git status` check |
| O5: MDX `href`s and link resolution | No consumer reads links | A consumer of `doc_links` |
| O6: corpus share of the graph and peak memory | C6's measurement owns it | C6 (§4.3) |

## Disposition (author, 2026-09-23)

The author took §8's simpler alternative for F2, and fixed F1–F8 in one commit after C6's first
measurement (`e313148`), which had not touched the C5 surface. The decisions §10 asked for:
- **F2:** the corpus run names each installed file of the release's distributions by the library
  run's `@path` (not a Stage-D bridge). `usage_targets` and the `usage_link` edge retire; code 35
  stays in the codebook, marked retired.
- **F3:** detect and fail only. The search-path order is unchanged, so site-packages stubs still
  outrank Pyrefly's bundled ones; a flat layout is a compile error naming the module, raised by
  the extractor where Pyrefly resolves the import (not a rule, since no published row could show
  it).
- **F1(b):** the library's `release_id` enters the corpus **release** id ("the corpus of this
  library release").
- **F5:** both: Stage A refuses, and a rule checks every declared family.

| # | Outcome | What changed | Oracle |
|---|---|---|---|
| F1 | fixed | `context()` makes each search-path entry relative to the release root or the environment root (`$release`, `$venv`); `Release::corpus` hashes the library's `release_id` | test: `a_corpus_run_does_not_depend_on_its_location` (two temp dirs, one context and run id; another library release, another corpus release). Pilot: the corpus search path is `$release \| $venv/lib/python3.14/site-packages` |
| F2 | fixed | by the alternative: `ModuleIds` and `ModuleRefs` in the corpus run include the release's installed `.py`/`.pyi` files under their site-relative paths; the corpus imports exclude them | test: `a_corpus_documents_its_library` asserts one `Server` class term; its snapshot shows the usage calls on `pkg.core` nodes. Pilot: `FastMCP` one term; 0 terms naming a release module by its dotted name; 18,564 usage call edges and 3,605 usage imports on release nodes |
| F3 | fixed | for each installed release file, Pyrefly's own import of its module name in the corpus must land on the installed handle; otherwise "the corpus tree shadows the release: …" | test: `a_tree_that_shadows_the_release_fails` (P-A's flat layout). Pilot unchanged |
| F4 | fixed | `git()` sets `GIT_ATTR_NOSYSTEM=1`; the checkout passes `-c core.attributesFile=/dev/null -c core.autocrlf=false` | test: the stub `git` in `crates/lctx/tests/acquire.rs` asserts both |
| F5 | fixed | Stage A refuses unknown `[tool.lctx.source]` keys and non-list or non-string values, and an include glob that selects nothing; `coverage:family-has-scope`: a run declaring `docs` has a document, one declaring a code family has a module | tests: `a_glob_that_selects_nothing_is_refused`; the `coverage:family-has-scope` case below |
| F6 | fixed | a dotted form's member is recognized only when its prefix is the class's access or origin path (or the form is `Class.member`) | test: `` `other.Server.tool` `` in `docs_shapes`'s quickstart gives no exact row (snapshot). Pilot: 77 exact mentions, unchanged |
| F7 | fixed | `the_corpus_rules_reject_their_violations`: `id:documents`, `id:passages`, `id:code_blocks`, `typed:mention_targets`, `unique:type_terms` (a doctored display), `unique:release-paths`, `coverage:complete` (docs arm), `coverage:family-has-scope` and `lineage:block_module`, each failing and publishing nothing. Block module directories carry an 8-hex digest of the document path, so P-B's pair is two modules | the test; the corpus snapshot's block paths |
| F8 | fixed | DESIGN §3.2 (`lexical` references include augmented-assignment targets; `types` names `unique:type_terms`; `docs` describes the `@path` naming), §3.4.1 (type terms among the merged kinds), §3.8 (the C5 edge row, the rules list, tests), §4.0 (the corpus's own context; hermetic attributes; key and glob checks), the Measured blocks; `config.rs`'s comment | prose; the tests above |
| O1–O5 | deferred | as the review's table | — |
| O6 | closed by C6 | the measurement (DESIGN §4.3): the corpus doubles the graph, but streaming derive's trigger is not met | — |

**Checks** (2026-09-23): `just test-all` passed (nextest 82/82; pytest 21/21; rule tests 4/4;
`lint-agents`; `adr lint`; fixtures; family; cargo-deny; `pyrefly-fork`; gold). `just pilot`
passed on a fresh store (a declared migration: `import_targets` rewritten, `usage_targets` gone,
`EXTRACTOR_OUTPUT_VERSION` 15): snapshot `ab6d98a3…`, 905,648 nodes, 1,449,162 edges, all 492
rules, 44.6 s at 7.4 GB peak.
