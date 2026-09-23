# Design review: ADR-0013, libraries as pinned uv projects and Stage A (standard)

**Date:** 2026-09-22 · **Depth:** standard · **Mode:** document plus code, as committed in
`b1cc9a4`. ADR-0013 supersedes ADR-0007 and changes the run contract (§3.4.1, §4.0), so a
`standard` review is owed (ADR-0001).
**Reviewer:** `design-reviewer` subagent (fresh context) · **Author:** the session that wrote
`b1cc9a4`
**Prior reviews that bear on this:**
- `design_review_adr-0012-pyrefly-ruff-in-process_2026-09-22.md`, F9: absolute locations entered
  the context digest;
- `design_review_inc1-slice1-schema-extract-delta_2026-09-22.md`, F2: two environments gave the
  same identities and different facts.

Both were fixed in slice 1. This change partly regresses both (F1 below).

## 1. Decision and scope

**Proposal.** ADR-0013 (`proposed`, `evidence: Tested`) does five things:
- **Libraries.** Every analyzed library becomes a committed uv project, `libraries/<name>/`: a
  virtual `pyproject.toml` with one pin and `[tool.lctx] release`, a `.python-version` and a
  `uv.lock`.
- **Acquisition** is `uv sync --frozen`, run by the new `lctx` CLI.
- **Stage A** (`cpg_extract::library`) reads the definition, the lock, `pyvenv.cfg` and every
  dist-info. It refuses version drift and any release file that differs from its `RECORD`, and
  takes the release's modules from the `RECORD`s.
- **Identity.**
  - `release_id` hashes the lock's artifact hashes for the release distributions.
  - `context_id` gains an environment digest (dist-info `RECORD` digests plus unowned top-level
    entries) and a lock digest.
  - Two provenance tables are added: `releases` and `distributions`.
- **The pilot** moves to FastMCP 4.0.5, and `scripts/check_gold.py` keeps the gold reference on
  the same version.

**Target.**
- ADR-0013, and ADR-0007 for the carry-forward check.
- ADR-0008's registry rule.
- The DESIGN amendments: §1.2, §1.4, §3.2, §3.4.1, §4.0, §4.1, §11 and §12.
- Code: `crates/cpg-extract/src/library.rs`, `config.rs` (environment digest, context),
  `lib.rs` (`release_rows`), `crates/lctx/src/main.rs`, `crates/cpg-extract/src/bin/lctx-extract.rs`,
  and `crates/cpg-schema/src/tables.rs` (`releases`, `distributions`, `contexts`).
- Tests: `crates/cpg-extract/tests/library.rs`.
- The rest of the change: `scripts/check_gold.py` and its tests, `libraries/fastmcp/*`,
  `libraries/README.md`, the justfile and `docs/pins.md`.

**Status of the claims.**
- DESIGN §4.0 is labelled **Implemented** and **Tested**, with a **Measured** paragraph.
- §1.4 is labelled **Tested** by `just pilot`.
- ADR-0013 says `evidence: Tested`.

**Observable outcome.** There is one production path for any Python library: a committed
definition, a frozen sync, a verified Stage A, and a published snapshot. It adds identities that
follow the pinned artifacts, and it keeps gold and analysis on one FastMCP.

**Baseline.** ADR-0007 planned an acquisition manifest plus `ACQUISITION.json`, which was never
built. The slice-2 code worked like this:
- a hand-made `release_root`;
- `release_id = H(--release-label)`;
- a full-content walk of site-packages (`site_packages_digest`), which was
  location-independent because it never looked outside site-packages.

**Supported scope.**
- PyPI-hosted libraries acquired through uv.
- The release's code comes from its installed wheels.
- Docs, examples and tests are deferred to increment 3.

**Constraints and uncertainty.**
- One operator on Linux ext4.
- `uv` 0.12.18 is taken from `PATH`, and its cache (`~/.cache/uv`) is shared with the project's own
  `.venv`.
- No Context7 tool was available in this session. uv's behaviour comes from `uv help sync` for
  0.12.18 and from observed runs.

### Method and coverage

**Read in full:**
- ADR-0013 and ADR-0007;
- ADR-0008's producer rule;
- the DESIGN diff and the current text of §3.2, §3.4.1, §4.0 and §4.1;
- `library.rs`, `config.rs`, `lctx/src/main.rs` and `lctx-extract.rs`;
- `release_rows` and the Stage A wiring in `cpg-extract/src/lib.rs`;
- the `Contexts`, `Releases`, `Distributions` and `SourceFiles` contracts;
- `cpg-core/src/attempt.rs`: `compile`, `content_digest` and `compiler_digest`;
- `tests/library.rs`, and the names of the identity tests;
- `check_gold.py` and the names of its tests;
- the committed `libraries/fastmcp` files, `libraries/README.md`, and the justfile and pins.md
  diffs.

**Checks run in this session:**

| Check | Command | Outcome |
|---|---|---|
| Default loop | `just check` | **passed**: nextest 54/54, pytest 21, rule tests 4, `lint-agents` ok, `adr lint` ok (13 records) |
| Stage A tests | `INSTA_UPDATE=no cargo nextest run -p cpg-extract --test library` | **passed**, 5/5 |
| Fixtures | `just fixtures-check` | **passed**, 20 files parse |
| Dependency policy | `just deps` | **failed**, at `scripts/check_gold.py` only (the skill installs 4.0.3; `libraries/fastmcp` requires 4.0.5). `check_family.py`, `cargo deny` and `check_pyrefly_fork.py` passed. The skill refresh is out of scope, as briefed |
| `just test-all` | not run as one command | Its parts are the three rows above, so it would fail at the gold check |
| Pilot, twice | `/usr/bin/time -v just pilot` | **passed** both times. 275 modules, 103 distributions, published (validation runs before publication, `attempt.rs` L193–L196). Same `release_id` `082ce22e…` and `content_digest` `d949f713…` both times. `lctx` reported an 8.0 s total; wall time 8.20 s and 8.24 s; peak RSS 1,639,188 kB and 1,677,732 kB |
| Revisit triggers | `just adr revisit` | ran. ADR-0002 is **failed** (`$ just deps`), solely because of the gold check (F9). ADR-0013's triggers are manual |

**Reviewer probes.** All ran against `target/release/lctx` and `lctx-extract` at `b1cc9a4`. Every
write went under the session scratchpad, and `git status` stayed clean throughout. Environment
copies were made with `cp -r`, so they have fresh inodes and no shared inode was ever written.

| # | What | Result |
|---|---|---|
| P1 | The same lock acquired twice at different paths (`build/envs/fastmcp` and a scratch dir); both extracted | `environment_digest`, `context_id` and `run_id` differ; **0 of 98,656 `fact_id`s are shared**, while `release_id` is equal. 21 of 103 `RECORD`s differ, only in their `../../../bin/<script>` lines: each script's shebang holds the environment's absolute interpreter path |
| P2 | A copy of the pilot environment, plus an empty `mcp_types/__init__.pyi` (unowned, inside the `RECORD`-owned `mcp_types/`) | `contexts.arrow` and `runs.arrow` are **byte-identical** (same `context_id` and `run_id`). `pysa_calls` goes from 35,057 to 34,594 rows; `parameter_semantics` from 10,314 to 10,022; `facts` from 98,656 to 97,817 (1,550 gone, 711 new) |
| P3 | Cost of SHA-256 verifying every analyzer-readable file (`.py`, `.pyi`, `py.typed`) in the pilot environment against its `RECORD`, and listing unowned ones (Python, warm page cache) | 8,045 files, 57.2 MB, **0.076 s**. RECORD parsing takes 0.003 s. It flags exactly `_virtualenv.py` and, on the P2 copy, the injected stub |
| P4 | `stat` and `find -inum` on pilot files | `fastmcp/server/server.py` (inode 31351196) and `mcp/__init__.py` (inode 32148612) are each **one inode** shared by `build/envs/fastmcp`, the project's `.venv`, `build/envs-demo` and `~/.cache/uv/archive-v0/…`. On ext4, uv's default `clone` link mode falls back to hard links |
| P5 | `lctx acquire` with `XDG_CONFIG_HOME` pointing at a `uv.toml` containing `compile-bytecode = true`, `offline = true` | **7,888 `.pyc`** against 0 in the control, so user-level uv config steers acquisition. The same file in a parent directory of the library is **not** picked up, because the library's `pyproject.toml` has `[tool.uv]`. `uv sync … --no-config` suppresses it (0 `.pyc`), **but also ignores `.python-version`**: `version_info = 3.14` from the minor-version link, which Stage A refuses, fail-closed. `--no-config --python 3.14.7`: 0 `.pyc`, `version_info = 3.14.7`, and Stage A plus extraction pass. `--no-config` also drops the project's own `[[tool.uv.index]]` at lock time |
| P6 | A git-sourced release (`demo @ git+file://…`), locked by uv 0.12.18, synced and extracted at two commits with different code | The lock entry is `source = { git = "…#<commit>" }`, with **no `wheels` or `sdist`**. Stage A accepts both commits. **Same `release_id` `64410352…` and same module `node_id`**; file `content_digest` differs, and so do `context_id` and `run_id` |
| P7 | Release `demo==1.0` depending on `other`. A second `demo-1.0` wheel (different bytes) appears on the index, and `other` 1.1 is published | Local find-links, no `--refresh`: README step 2, `uv lock --upgrade-package other`, grows demo 1.0's `wheels` from 1 to 2 (plain `uv lock` does not). Find-links artifacts are recorded **without hashes**, so this is the P6 fallback again. Hashed PEP 503 index on localhost, with `--refresh`: `release_id` goes from `04c91e13…` to `9cd8b521…` and the module `node_id` from `39dcb204…` to `d72a0e7c…`, while demo's version and **file bytes are identical** (`d19eaacd…`) |
| P8 | `lctx library init jsonschema --requirement jsonschema==4.26.0` (scratch `--libraries`/`--envs`), plus a faithful re-implementation of `source_urls` over the 103 pilot distributions | Proposed `release = ["jsonschema", "jsonschema-specifications", "referencing", "rpds-py"]`, joined through `https://github.com/sponsors/julian`. The same mechanism groups pydantic, pydantic-core, pydantic-settings and watchfiles (`sponsors/samuelcolvin`), and starlette with uvicorn (`sponsors/kludex`). griffelib, jiter and py-key-value-aio have no GitHub or GitLab URL |
| P9 | A release file edited in a private copy, then `uv sync --frozen` (what `lctx acquire` runs), then Stage A | uv reports "Checked 103 packages in 0.40ms" and repairs nothing. Stage A still refuses with "…does not match its RECORD sha256; **run `lctx acquire`**" |
| P10 | `uv sync --python 3.14.5` for `libraries/fastmcp`, whose `.python-version` is 3.14.7 | Stage A **accepts** it; the context records 3.14.5 |

**Not inspected, or not settled:**
- whether Pyrefly reads `.pth` files;
- uv's cache integrity model beyond the behaviour observed;
- the `attrs` 0.8 s claim (not re-run);
- the platform string on macOS: `std::env::consts::OS` is `"macos"`, while Pyrefly expects
  `"darwin"` (not verified; out of this operator's scope);
- lock forks: `lock()` keys packages by name, so the last duplicate wins. That was read in the
  code, not exercised. The version check makes it fail closed.

**Guarantees not attacked, so asserted only:**
- a race between Stage A's verification and extraction's re-read of the same files (same process,
  milliseconds apart);
- two concurrent `lctx compile` runs on one environment;
- uv's own hash verification on first download (trusted);
- the parts of the ADR-0009 publication protocol this change does not touch.

## 2. Authority and lifecycle (reconstructed)

| Concept or fact | Identity | Authority / owner | Revision boundary | Permitted update path | Derived representations |
|---|---|---|---|---|---|
| Library definition (pin, `release`, `source`) | `libraries/<name>/` | operator, as a reviewed commit | git revision | edit, relock, commit | `releases.library`, `.requirement` |
| Acquisition lock | `uv.lock` bytes | uv writes it, the operator reviews it | git revision | `uv lock [--upgrade-package]` | `release_id` (artifact hashes), `contexts.lock_digest`, `releases.lock_digest`, `distributions.artifact_sha256` |
| Interpreter pin | `.python-version` | operator | git revision | edit | **none**: Stage A never reads it; it reads `pyvenv.cfg` (P10). *Invented cell: no reconciliation* |
| Acquired environment | `build/envs/<name>` | uv; mutable and rebuildable | **none**: known only through `environment_digest` | `lctx acquire` | `contexts.environment_digest`, `distributions` |
| Bytes of the environment | inodes | **shared** with the project `.venv` and the uv cache (P4). *Invented cell: DESIGN says the project environment "is never an input"* | — | anything that writes in place through any link | — |
| Release modules | `RECORD` `.py`/`.pyi` entries of the `release` distributions | Stage A, after verifying | the attempt | — | `source_files` (with a per-file `content_digest`) |
| `release_id` | BLAKE3 of names, versions and **locked** artifact hashes | derived | — | — | every `node_id`. *Equivalence never stated (F2)* |
| `context_id` | BLAKE3 of Python version, platform, relative paths, config digest, environment digest, lock digest | derived | — | — | `run_id`, and so every `fact_id` |
| `releases` / `distributions` rows | `(snapshot, release)` / `(snapshot, release, name)` | **stated three ways** (F4) | the snapshot | the extractor run writes them today | — |
| Gold reference | the skill's `build/manifests` | the skill's own maintenance | the skill's refresh | `MAINTENANCE.md` | reconciled by `check_gold.py` (pytest, 3 cases) |
| Docs source | `[tool.lctx.source]` repository + tag | operator | **none pinned**: a tag is mutable (F6) | edit | none yet |

**Deliberately opaque.** uv's resolution and installation are trusted behind the lock's hash
verification. What the heuristic treats as "the same source" (METADATA URLs) is opaque too.

**Identity behaviour.**
- Renaming a library directory changes only `releases.library`. `release_id` hashes the constant
  `"library"`, `library.rs` L240.
- Moving the environment changes every `fact_id` (P1).
- A dependency-only upgrade can change every `node_id` (P7).
- A new git commit at the same version changes no `node_id` (P6).

## 3. Contracts and invariants

| Invariant | Representation | Enforcement boundary | Failure behaviour | Verification |
|---|---|---|---|---|
| Each installed distribution is at the locked version | dist-info name vs `uv.lock` | `library.rs` L222–L236 | refuses before any write | **Tested** (`an_environment_not_synced_to_the_lock_is_refused`). One direction only: a locked dependency missing from the environment is not refused (its absence does change `environment_digest`) |
| Release files match their `RECORD` | sha256 per entry | `library.rs` L253–L266 (skips `..` entries, L254) | refuses | **Tested** plus P9. The remedy text is wrong (F3) |
| Release modules are exactly the `RECORD` `.py`/`.pyi` entries | `Release.files` | `library.rs` L267–L269 | — | **Tested** (`…compiles_its_release_distributions_only`) |
| Every `release` entry is installed | — | `library.rs` L244–L246 | refuses | Implemented, not tested |
| The release's locked entry carries artifact hashes | — | **none** | silently falls back to name + version (P6) | none (F2) |
| `environment_digest` covers what the analyzer reads | — | **none** | unowned files inside owned directories are invisible (P2) | a test encodes the blind spot (`library.rs` L175–L181) (F1) |
| Identity is independent of location | relative paths in `contexts` | `config.rs` `relative`/`relativize` | **violated** through `RECORD` bytes (P1) | the two-location test uses `from_tree` fixtures without scripts (F1) |
| The interpreter is the pinned one | `.python-version` | **none** | the observed version is recorded (P10) | none (F3) |
| Gold and analysis name one FastMCP | install line and release versions | `check_gold.py`, in `just deps` | `just deps` fails | **Tested** (pytest) and seen failing correctly today |
| Acquisition is hermetic | `UV_*` and `VIRTUAL_ENV` removed (`main.rs` L67–L72) | partial | user `uv.toml` honoured (P5) | none (F3) |

**Absence.** `lock_digest` is `Option` and hashed through `opt_digest`, so a source tree and a
library are distinguishable (**Tested**, `ids.rs` `optional_absent_and_empty_differ`).
`distributions.artifact_sha256 = []` is an unmarked sentinel for "the lock recorded no hash"
(P6, P7), and `release_id` treats it as a valid artifact set.

**Equivalence.** Neither of the two new digests states its equivalence (DM-15):
- `release_id` behaves as "same names, versions and locked artifact list".
- `environment_digest` behaves as "same dist-info names, same `RECORD` bytes, same unowned
  top-level bytes". The `RECORD` bytes include location-dependent script hashes.

## 4. Derivation and execution

| Stage | Inputs | Output contract | Assumptions | Effects | Provenance / invalidation |
|---|---|---|---|---|---|
| Acquire (`lctx acquire`) | definition, lock; **ambient:** `uv` on `PATH`, user or system `uv.toml`, the uv cache, managed interpreters | `build/envs/<name>` | the lock carries hashes; the cache is intact | writes the environment; network on first run; hard links into the cache (P4) | `pyvenv.cfg` records `uv = 0.12.18`, which Stage A does not read |
| Stage A | definition, lock, `pyvenv.cfg`, dist-infos, release bytes | `ExtractInput`: files, `release_id`, `AcquiredLibrary` | installed bytes of dependencies equal their `RECORD`s | reads only | `releases`, `distributions` |
| Extract | `ExtractInput` | raw batches, `contexts`, `runs` | as before | reads | `context_id`: F1 |
| Derive, validate, publish | unchanged (ADR-0009) | `snapshots` row | — | Delta writes | `content_digest` = sorted `run_id`s + `compiler_digest` (`attempt.rs` L90–L99) |

**Coherent publication** is unchanged. Stage A refuses before any write, and validation runs
before the `snapshots` append (`attempt.rs` L193–L196).

## 5. Representative journeys

**Ordinary extension: add a library.**
- Adding one is data, not code, and the `release` list is human-reviewed.
- P8 shows the proposal is unsafe to accept without that review. For `jsonschema` it proposes
  three dependencies from other repositories as first-party. The operator has to catch it, and
  nothing records that the list was reviewed (F5).

**Meaningful change: upgrade a dependency by the README procedure.**
- P7: `uv lock --upgrade-package other` also rewrites the unchanged release's artifact list.
  Against a hashed index, `release_id` and every `node_id` move, although the release bytes are
  identical.
- The snapshot diff DESIGN §3.4.1 promises ("an identical rerun re-emits the same `node_id`",
  "diffing … is a join") then shows every node as removed and re-added (F2).
- The converse is P6: a new commit at the same version keeps every `node_id` while the code
  changes.

**Boundary: the same analysis from another checkout.** AGENTS.md asks for `git worktree` for
spikes. A worktree acquires into `<worktree>/build/envs/fastmcp`, so console-script shebangs
change, then 21 `RECORD`s, then `environment_digest`, `context_id` and every `run_id` and
`fact_id` (P1). Two worktrees analysing identical inputs share no `fact_id`, and their
`content_digest`s differ. DESIGN §4.0 L646–L649 claims the opposite, labelled **Tested** (F1).

**Interruption or failure: an edited file.**
- A coding agent debugging the MCP server writes in place into `.venv/…/mcp/…`, the serving
  environment. P4 means that write lands in the analysis environment and the uv cache too.
- **For a dependency file,** the next compile publishes changed facts under an unchanged
  `context_id`, which is the P2 mechanism.
- **For a release file,** Stage A refuses. That is correct. But its instruction, "run
  `lctx acquire`", cannot repair it: uv only checks dist-info metadata (P9), and with hard links the
  cache copy is corrupted too.
- A failed `lctx library init` leaves `pyproject.toml` and `.python-version` without a lock.
  `acquire` then refuses for want of a `uv.lock`, so the half-written directory cannot publish.

## 6. Acceptance gates

| Gate | Result | Evidence or scope rationale | Required action |
|---|---|---|---|
| **G1** Authority | **pass** | One definition per library. Stage A reconciles the environment with the lock for version drift, and `check_gold.py` reconciles gold with analysis (Tested, and failing correctly today). The two `lock_digest` columns come from one in-memory value, so they are not a second authority. The unreconciled interpreter pin is an intent-versus-observation gap: the observation is recorded honestly, so it is handled under F3. Who writes `releases`/`distributions` is stated three ways, but there is one writer today (F4, latent) | F4 before a second producer |
| **G2** Semantic fidelity | **pass** | The new tables distinguish tree and library origins (`Option` columns, `opt_digest`), release and dependency (`in_release`). The empty-artifact sentinel is judged under G6 and G7, where its consequence lands | — |
| **G3** Validity | **pass** | Every Stage A refusal fires before any write: version drift, `RECORD` mismatch, missing release distribution, a malformed `pyvenv.cfg` (P5's `3.14` case fails closed). A missing locked dependency is not refused, but extraction does not assume it and the context records its absence | — |
| **G4** Hidden behaviour | **fail** (narrow: acquisition) | DESIGN §4.0 L603–L604: "so nothing ambient steers it". P5: a user-level `uv.toml` steers `lctx acquire`, and no `UV_*` variable is involved. P4: the analysis environment shares inodes with the project `.venv`, which ADR-0013 calls "never an input". P10: the committed interpreter pin is not enforced. Stage A and the extractor themselves still pass: constructed config, refused knobs, as in the ADR-0012 review. ADDENDUM Q15 passes: `check_gold.py` reads the skill only in `just deps`, and nothing in the compiler does | F3 |
| **G5** Consistency and recovery | **pass** | Nothing publishes on a Stage A failure, and publication is unchanged. The recovery advice is wrong (P9), which is a DM-30 defect handled in F3, but no reader can mistake a partial output for a valid one | F3 (remedy text) |
| **G6** Transformation and reuse | **fail** | Two derived identities fail their contract. **`context_id`** changes with the environment's location (P1: 0 of 98,656 `fact_id`s shared) and stays fixed when analyzer-visible bytes change (P2). **`release_id`** stays fixed across commits (P6) and changes with no release byte changed (P7). DESIGN §3.4.1 promises `node_id` "stable across snapshots and runs" and snapshot diffs "as a join" | F1, F2 |
| **G7** Truthful capability claims | **fail** | Hash-less release sources silently fall back to name+version identity (P6, P7); the ADR presents acquisition as "the one way any Python library is pinned". Falsified claims labelled Tested or stated as fact: "where a checkout or tempdir sits never changes an identity (**Tested**…)" (L646–L649, P1); "uv verifies every artifact hash" (L603, false for git, directory and local find-links sources); "nothing ambient steers it" (P5); proposal "**Tested**" with no test (L623) | F1, F2, F3, F8 |

Each failure is local. The corrections are confined to `library.rs`, `config.rs` and
`lctx/src/main.rs`, plus focused tests and sentence edits. None touches the structural decision:
committed uv projects, a frozen sync, `RECORD`-based release selection, the `lctx` CLI and the gold
guard.

### The eight questions in the brief

| # | Question | Answer | Where |
|---|---|---|---|
| 1 | Is not re-reading `RECORD`-owned dependency bytes sound under G6/DM-31? | **No.** The proxy "installed bytes = `RECORD`" is unsound three ways. **(a)** In-place writes through the hard links shared with the project `.venv` falsify it, and uv never re-checks (P4, P9). **(b)** Its ownership test works at top-level granularity, so unowned files inside owned directories are invisible, and they do change answers (P2). **(c)** The `RECORD` file is not a canonical form: its `..` script hashes carry the environment path (P1). The cost argument ("so a large environment stays cheap") is an unmeasured hypothesis (DM-39): verifying every analyzer-readable byte takes 0.076 s, about 1% of the 8.0 s compile (P3) | F1 |
| 2 | Is `release_id` from the lock's artifact hashes (every wheel plus the sdist) the right identity? | **Not as specified.** It identifies the lock entry, not the analyzed release. It is **blind** where the lock records no hash (git, directory, local find-links: P6, P7), and **volatile** where the lock re-lists artifacts that were never installed (P7). The release's `RECORD` entries, which Stage A already verifies, are the identity of what is analyzed | F2 |
| 3 | Can anything ambient still steer `lctx`'s uv invocation? | **Yes.** Removing `UV_*` works for environment variables, but user and system `uv.toml` are honoured (P5), and a parent-directory `uv.toml` is not (P5). `.python-version` is found from the project root, but it is not reconciled (P10), and `--no-config` ignores it (P5). The uv cache hard-links into both the project `.venv` and the analysis environment (P4). `uv` comes from `PATH`, and its version is in `pyvenv.cfg` but unrecorded | F3 |
| 4 | What are the release-proposal heuristic's failure modes? | **False positives** through shared funding or sponsor URLs, which are real in the pilot's own closure (P8). **False negatives** when a first-party distribution has no GitHub or GitLab URL (3 of 103 here) or a stale one after a repository move (FastMCP moved from `jlowin` to `PrefectHQ`). **Untested.** It is a proposal for review, so no gate | F5 |
| 5 | Does dropping `ACQUISITION.json` and the tag tarball lose anything increment 3 needs? | **Yes, two things.** A committed integrity pin for the docs input (`[tool.lctx.source]` has a mutable tag and no commit or sha256), and ADR-0007's slot for it in `content_digest`. The path map is still promised (§4.0 L595–L596). Nothing checks that the tag names the locked version | F6 |
| 6 | Who produces the provenance tables (ADR-0008's registry rule)? | **Undecided.** §3.2 L365–L366 lists four registries and says `source_files` "is the extractor's until Stage A lands". §4.1 L676 gives `releases`, `distributions` and `source_files` to Stage A. ADR-0013 L92 calls them "registries the extractor run writes". The code has the extractor write them, with no `run_id` and no `context_id` | F4 |
| 7 | Do the §4.0 labels match the tests? | **Partly.** The Measured figures reproduce (8.0 s, 1.64–1.68 GB), but their conditions leave out that the environment was warm. The **Tested** labels on two locations, on the release proposal and on `crates/lctx` have no test behind the stated behaviour | F8 (and F1, F3) |
| 8 | Is ADR-0007 carried forward completely? | **The substance, yes; the controls, no.** Encoding, `lctx-compiler`, overloads and "no config digest" survive in DESIGN §3.4.1. ADR-0007's revisit trigger ("an analyzer answer changes without `context_id` changing"), which P2 fires, and its per-input `run_id` oracle are dropped. `just adr revisit` no longer surfaces any identity trigger | F7 |

## 7. Findings

Ordered by severity: correctness and authority first, then extension and proportionality.

| ID | Finding | Principle IDs | Concrete evidence or gap | Consequence | Proposed correction | Verification |
|---|---|---|---|---|---|---|
| **F1** | `environment_digest` digests `RECORD` *files* as a proxy for the bytes the analyzer reads. The proxy is not canonical, so the location enters identity (a regression of ADR-0012 review F9), and it is incomplete, so analyzer-visible changes leave identity unchanged (a regression of slice-1 F2). | DM-15, DM-31, DM-32, DM-48 · G6, G7 | **The code.** `config.rs` L242–L253: `records.push((name(p), content_digest(&record)))` hashes the whole `RECORD`, including the `../../../bin/*` lines that Stage A itself skips (`library.rs` L254). Ownership is by top-level name (`config.rs` L245–L250: `path.split('/').next()`), so anything under an owned directory is never read. **The test encodes the blind spot:** `tests/library.rs` L175–L181 asserts that a changed dependency byte leaves `context_id` unchanged. **The claim.** DESIGN L646–L649: "Where a checkout or tempdir sits never changes an identity … (**Tested**: two locations …)". The two-location test uses `Release::from_tree` fixtures without console scripts. **P1:** same lock at two paths gives 0 of 98,656 `fact_id`s in common. **P2:** one unowned `.pyi` changes 2,261 facts with identical `contexts` and `runs` bytes. **P4:** dependency files share inodes with the project `.venv`. **P3:** closing the gap costs 0.076 s. | **(a)** Two worktrees, or any `--envs` choice, analyse identical inputs and get disjoint `fact_id`s and different `content_digest`s. §3.4.1's "compares reruns", and the §9.8 ablation joins across machines, report total change where there is none. **(b)** A stray or leftover `.pyi`, or an in-place edit in the project `.venv` that reaches the analysis environment through a hard link, changes Pysa call targets and signatures. The snapshot then claims the same `context_id`, `run_id` and `content_digest` as a clean one. This is exactly ADR-0007's revisit trigger, now dropped (F7). | Define the environment's equivalence as **the analyzer-readable bytes of site-packages**. The digest becomes the sorted `(site-relative path, sha256)` of every `.py`, `.pyi` and `py.typed` file (plus unowned top-level entries, as today). Owned files are verified against their `RECORD`, as Stage A already does for the release (refuse on mismatch); unowned files are hashed directly; `..` entries are dropped; the dist-info name and version list is kept. That also removes the declared "What we give up" line. Reword DESIGN L631–L634 and L646–L651. | **Tests** in `tests/library.rs`: **(i)** make `install()` write a location-dependent `../../../bin` hash and assert that two environment paths give one `context_id` (fails today); **(ii)** an unowned `.pyi` inside an owned directory changes `context_id` (fails today); **(iii)** replace L175–L181 with "a changed owned dependency byte is refused". |
| **F2** | `release_id` identifies the lock *entry*, not the analyzed release. It degrades silently to name + version when the lock records no hash, and it changes when the lock re-lists artifacts that were never installed. | DM-15, DM-11, DM-32, DM-43 · G6, G7 | **The code.** `library.rs` L126–L136 collects `wheels[].hash` and `sdist.hash`; an entry without hashes yields `[]`. L247–L251 hashes that list, and nothing refuses an empty one. **The claims.** DESIGN L598–L599: "Every distribution of the closure with the sha256 of each of its artifacts". L603 and ADR L30: "uv verifies every artifact hash". **P6:** a git source (lock `source = { git = "…#<commit>" }`, no artifacts) gives the same `release_id` and `node_id`s at two commits with different code. **P7:** local find-links are also recorded without hashes. Against a hashed index, README step 2 for a *dependency* re-lists `demo 1.0`'s wheels and moves `release_id` and the module `node_id`, while the file bytes stay identical. The test `the_release_id_follows_the_release_distributions_locked_artifacts` protects the lock coupling rather than the identity. | **(a)** `lctx library init foo --requirement "foo @ git+…"` is accepted: the requirement parser splits on `@`. Every later commit publishes under the same `node_id`s, so cross-snapshot joins treat changed code as unchanged. That quietly enters the situation ADR-0013's own revisit trigger was meant to catch ("needs a non-PyPI build"). A local find-links source, recorded without hashes (P7), pointing into `.claude/skills/` would pass the same way (ADDENDUM Q15; a `path` source was not probed). **(b)** For compiled libraries, which get wheels uploaded after release (new CPython or platform tags), a routine dependency upgrade renames every node of an unchanged release, so the §9.8 and §12 diffs across it are meaningless. | **Author decision.** Either **(i, recommended)** make `release_id = H(name, version, sorted (path, sha256) of the release's verified analyzer-readable RECORD entries)`, which is exactly what is analyzed. Stage A already holds these; the lock's artifact hashes stay as provenance in `distributions.artifact_sha256`. Or **(ii)** keep the lock identity but hash only the installed artifact. With either option, **refuse a release distribution whose lock entry has no artifact hash**, so "uv verified it" is true, and state the equivalence in §3.4.1. | **Tests** in `tests/library.rs`: a release locked with `source = { git = … }` and no artifacts is refused. Under (i), an extra artifact hash in the release's lock entry leaves `release_id` unchanged, and a changed release file with a matching `RECORD` changes it. |
| **F3** | `lctx acquire` is not hermetic as claimed. User and system uv config steer it; the committed interpreter pin is not enforced; the environment shares inodes with the project `.venv` and the uv cache; and the installer version is unrecorded. Stage A's recovery instruction cannot recover. | DM-28, DM-31, DM-48, DM-30, DM-47 · G4, G7 | **The code.** `main.rs` L64–L82 removes `UV_*` and `VIRTUAL_ENV` only; there is no `--no-config`, `--python` or `--link-mode`. **The claims.** DESIGN L603–L604: "so nothing ambient steers it". ADR Context: "the project environment is never an input". `.python-version`: "the exact interpreter". **P5:** 7,888 `.pyc` from a user `uv.toml`; `--no-config` alone ignores `.python-version`, and Stage A then refuses `3.14`; `--no-config --python 3.14.7` works. **P10:** a 3.14.5 environment is accepted against a 3.14.7 pin. **P4:** one inode shared with `.venv` and the cache. **P9:** "run `lctx acquire`" repairs nothing. `pyvenv.cfg` carries `uv = 0.12.18`, which is not recorded. | A user-level setting (`no-binary-package`, `link-mode = "symlink"`, `python-preference`, build settings) changes what is installed on one machine and not another. It mostly reaches identity through `RECORD`s, but a reproduction on a second machine silently differs, against DM-48. The P4 channel makes F1(b) reachable by ordinary edits to the serving environment. An operator who hits a `RECORD` mismatch follows the printed advice and stays stuck. | `acquire` passes `--no-config --python <.python-version>` (P5 verified both) and `--link-mode copy`: about 127 MB per library on this non-reflink filesystem, and it decouples the analysis environment from `.venv` and the cache. Do **not** use `--no-config` for `init`'s `uv lock`, because it drops library-level `[tool.uv]` settings (P5); the lock is reviewed anyway. Stage A checks `pyvenv.cfg`'s `version_info` equals `.python-version`, and `releases` records the `uv` version from `pyvenv.cfg`. Change the mismatch remedy to `lctx acquire --reinstall` (uv `--reinstall-package <release dists>`). Reword L603–L604. | **None exists** (`crates/lctx` has no tests). Test: a stub `uv` script first on `PATH` records argv and environment; assert `--frozen --no-config --python 3.14.7 --link-mode copy` and no `UV_*`. A Stage A test: a `pyvenv.cfg` with `3.14.5` against `.python-version` 3.14.7 is refused. |
| **F4** | Who writes `releases`, `distributions` and (now) `source_files` is stated three ways. `distributions` is keyed on the release but describes the context's environment. | DM-02, DM-13, DM-46 · G1 (latent) | DESIGN §3.2 L365–L366: "`runs`, `contexts`, `producers` and `facts` are registries each producer appends its own rows to; `source_files` is the extractor's until Stage A lands". That was not updated, although Stage A has landed. §4.1 L676 gives all three to Stage A. ADR-0013 L92: "registries the extractor run writes". Code: `lib.rs` L531/L618, the extractor. Contracts: `releases` key `[snapshot_id, release_id]` (`tables.rs` L102) and `distributions` key `[snapshot_id, release_id, name]` (L121), with no `run_id` and no `context_id`. | **Increment 3.** A docs producer following ADR-0008's registry reading appends its own `releases` row for the same `(snapshot_id, release_id)`. The generated uniqueness rule fails and the attempt aborts; or, under the "Stage A's" reading, nothing tells it not to. **Two environments.** Two contexts for one release in a snapshot (a second Python version, a second extractor) collide on `(snapshot, release, name)` with different versions. A reader cannot tell which run saw which `mcp`. | One sentence in DESIGN §3.2, and in ADR-0013 while it is proposed: `releases`, `distributions` and `source_files` are written once per attempt by the extractor run, which carries Stage A's output; later producers reference `release_id` and never append. Add `context_id` to `distributions`, with a reference rule to `contexts`: the environment belongs to the context (a schema migration). | The generated rules (a new `ref:distributions.context_id->contexts`, and the existing uniqueness rule) plus the contract snapshot. When increment 3 lands: a two-producer attempt test. |
| **F5** | The release-proposal heuristic treats any shared GitHub URL as a shared source, including funding and sponsor links. It proposes dependencies from other repositories as first-party. | DM-54, DM-59 · — | `main.rs` L145–L162 keeps every `Project-URL`/`Home-page` value containing `github.com/` or `gitlab.com/`, with no label or path filter. L239–L243 takes any intersection. **P8:** a real run of `lctx library init jsonschema` proposes `jsonschema-specifications`, `referencing` and `rpds-py`. Over the pilot closure, `pydantic-settings` would pull in `pydantic` and `pydantic-core`. DESIGN L623 labels the heuristic **Tested**, but no test exists. | An operator accepts the printed proposal. Three other projects' code is then compiled as jsonschema's: `release_id`, briefs and the analyzed boundary all change, and dependency APIs are presented as the library's own. The only guard is a line of human review. | Compare only repository roots (`host/owner/repo`, the first two path segments). Exclude `github.com/sponsors/*` and `…/orgs/*`, and prefer the `Source`/`Repository`/`Homepage` labels. When the requested distribution has no repository URL, propose it alone and say so. | **Test:** a unit test over METADATA snippets: the FastMCP trio grouped, the jsonschema/referencing sponsor case not grouped, and a distribution with no URL. Move `source_urls` into a testable module. |
| **F6** | The docs, examples and tests source for increment 3 has no committed integrity pin, and no identity slot. ADR-0007 had both. | DM-31, DM-48 · G6 (unresolved, increment 3) | `libraries/fastmcp/pyproject.toml`: `[tool.lctx.source]` has `repository` and `tag = "v4.0.5"`, nothing more. ADR-0013 L55–L57 says it "is fetched when the docs family lands". DESIGN L595–L596 says "fetched with their sha256 and a path map", but the sha256 is computed at fetch time, not pinned. ADR-0007 committed it in a manifest and put the manifest digest in `content_digest`. §3.4.1's `content_digest` row now has no docs input. `check_gold.py` does not compare the tag with the locked version. | A tag moved upstream, or a regenerated GitHub archive, gives increment 3 different docs than the definition implies. Two compiles of the same definition can then analyse different docs, with nothing in identity to show it. A `tag` left behind after a pin bump pairs 4.0.6 code with 4.0.5 docs. | **Now, one line:** add `commit = "<40-hex>"` to `[tool.lctx.source]`, and check that `tag` names the locked version of the release. **At increment 3:** state that the docs run's context digest includes the commit and the fetched tree's digest, and define the path map there. | **None exists.** Test: Stage A, or `check_gold.py`'s pytest, refuses a `[tool.lctx.source]` with no 40-hex `commit` or whose `tag` does not contain the release version. |
| **F7** | The supersession carries ADR-0007's substance but drops its controls: the identity revisit triggers and the per-input `run_id` oracle. Two references are stale. | DM-59, DM-60 · — | ADR-0007 L10: "…same inputs producing different node_id or fact_id, or an analyzer answer changes without context_id changing". ADR-0013 L10 has neither. `just adr revisit` lists no identity trigger (ADR-0007 is filtered out as superseded). **P2 fires the dropped trigger.** ADR-0007's oracle, "changing any single context or producer input changes run_id": the new `lock_digest` input has no behavioural test (only the schema snapshots mention it). **Stale:** `attempt.rs` L87–L89 still says "The acquisition-manifest … inputs join it"; ADR-0013 `design:` (L8) omits §11, which the DESIGN changelog lists. | The next "answer changed, identity didn't" observation has no recorded trigger to reopen the run contract. F1 would have been caught by it. A regression that drops `lock_digest` from `context_id` passes every test. | ADR-0013 is still `proposed`, so amend it in place. Add ADR-0007's two triggers, reworded to the stated equivalences from F1 and F2. List the carried oracles. Fix the stale comment and the `design:` field. | `just adr lint` (exists). A new test: a lock-only change (same versions and hashes, one extra dependency entry) changes `context_id`. |
| **F8** | Several §4.0 and ADR labels are stronger than their evidence. | DM-59, DM-39 · G7 | L584–L585 labels `crates/lctx` **Implemented** and **Tested**, but `crates/lctx` has no tests; `just pilot` exercises `compile` on one library. L623 says "**Tested**: for FastMCP it proposes exactly the three distributions", which was a manual run. L649 claims two locations are **Tested** (P1: false for acquired environments, F1). L666–L668 **Measured**: reproduced here (8.0 s, 1.64–1.68 GB), but the conditions leave out that acquisition was a no-op ("Checked 103 packages in 0.32ms"), the warm cache and the host. ADR L106–L109: `attrs` in 0.8 s, not re-run here, asserted. | A later session skips tests for `lctx` and for the heuristic because DESIGN says they exist. The timing is read as including acquisition. | Relabel the heuristic and the CLI hermeticity **Implemented** (observed 2026-09-22) until F3's and F5's tests exist. Add "warm environment and uv cache, acquisition a no-op; host" to the Measured line. | **No mechanical oracle**, prose (AGENTS.md labels). F3's and F5's tests turn these into **Tested**. |
| **F9** | Putting the gold check in `just deps` makes ADR-0002's automated revisit trigger (`$ just deps`) fire on a gold mismatch. | DM-47 · — | justfile L47 (`check_gold.py` in `deps`); ADR-0002 `revisit: $ just deps`. `just adr revisit` today prints "ADR-0002: failed: $ just deps", caused only by the skill lag. | During every skill refresh ADR-0002's trigger is red for reasons unrelated to the Rust dependency family. A real family break then looks the same, and the signal learns to be ignored (ADR-0001's "costs more than it catches"). | A separate `just gold` recipe, run by `test-all`; or point ADR-0002's trigger at `check_family.py` and `cargo deny`. | `just adr revisit` (exists). |

**Applicability.**
- **Applied:**
  - group 3 (identity: F1, F2);
  - group 6 (effects, recovery: F3);
  - group 7 (dependencies and reuse: F1, F2, F6);
  - group 9 (provider boundary and capability: F2, F3);
  - group 10 (lineage, reproducibility: F4, F6);
  - group 11 (verification: F7);
  - group 12 (falsifiable claims, proportionality: F5, F8, F9, and §8).
- **Partly applied:**
  - group 1 (authority of the definition and the provenance writers: G1, F4);
  - group 2 (validity boundaries: G3, which passes).
- **Did not bear on this scope, and why:**
  - group 4: a library as data is the declarative part, and it is satisfied; there is no
    capability selection here;
  - group 5: Stage A is a reader, with no lowering;
  - group 8's layout and boundary principles (DM-36–38): no physical layout changed. DM-39 was
    applied to the timing claims.

**Maturity scores:** omitted. The gates carry the decision.

## 8. Alternatives and architectural leverage

| Alternative | Semantic duplication and extension locality | Correctness and operational risks | Cost | Performance evidence | Why selected or rejected |
|---|---|---|---|---|---|
| **Baseline** (slice 2 + ADR-0007 plan) | A hand-made release root and a label as identity; a manifest format to invent | Label identity; full-content site-packages walk (location-independent; covered every byte) | a bespoke acquisition tool | not measured | Rightly superseded. Resolution, hashing and installation belong to uv |
| **Proposed** (ADR-0013 as committed) | Three independent equivalence decisions: lock artifact list (`release_id`), `RECORD` file bytes (environment), top-level ownership (loose files) | F1: location enters identity, analyzer-visible changes are invisible. F2: a blind and volatile `release_id`. F3: ambient uv config and shared inodes | small: about 600 lines | 8.0 s pilot, reproduced | Structure sound; identity recipes not |
| **Simpler viable alternative** (the same acquisition; one canonical form) | **One** equivalence, "the analyzer-readable bytes, `RECORD`-verified", used twice: the release's entries give `release_id`, the rest of site-packages gives the environment digest. The lock's artifact hashes and the `RECORD` digest stay as provenance columns only | Closes F1 and F2; refuses hash-less release entries; location-independent by construction (no `..` entries); detects in-place edits; a `--link-mode copy --no-config --python` acquisition closes F3 | less code than today. It deletes the ownership computation and the lock-artifact hashing from identity, and reuses Stage A's existing verification loop | P3: +0.076 s on the pilot | **Recommended.** It removes decisions rather than adding machinery |

**On the rejected Option 1** ("introspect the project's own `.venv`"): P4 shows the chosen design
shares the project `.venv`'s bytes anyway, through the uv cache's hard links. The isolation argument
for Option 3 holds only once acquisition copies instead of linking.

**Abstractions justified by current needs.**
- `lctx` has a real consumer: `just pilot`, and every future library.
- `releases` has two: the `runs.release_id` reference and provenance.
- `distributions` has no named reader yet. It is 103 rows, cheap, and serves DM-48
  reproducibility (which dependency versions a snapshot saw). Keep it, keyed to the context (F4).
- `check_gold.py` guards the §12 metric against comparing different code.

**What stays ordinary code.** The `RECORD` parser, the lock reader and the proposal heuristic are
specialized, small and well placed. None needs a declaration layer.

## 9. Verification and measurement plan

| Claim or risk | Evidence label (now) | Test / analysis | Conditions and expected result | Current result or gap |
|---|---|---|---|---|
| Stage A refuses version drift and a `RECORD` mismatch | **Tested** | `tests/library.rs` (5) | synthetic environment | passed (this session) |
| Location-independent identity for acquired libraries | claimed **Tested**; **falsified** (P1) | F1 test (i) | two environment paths with location-dependent script hashes give one `context_id` | fails today |
| Analyzer-visible dependency change changes `context_id` | **Proposed** (declared exclusion) | F1 tests (ii), (iii) | an unowned `.pyi` and an owned byte | fail today |
| `release_id` identifies the analyzed release | **Implemented**; blind and volatile (P6, P7) | F2 tests | a git lock is refused; an extra artifact does not move the id | fail today |
| Acquisition hermetic, pin enforced | claimed; **falsified** (P5, P10) | F3 stub-`uv` test; `pyvenv.cfg` test | argv and environment asserted; 3.14.5 refused | missing |
| Proposal heuristic | claimed **Tested**; manual only | F5 unit test | sponsor URLs not grouped | missing |
| Gold and analysis agree | **Tested** | `tests/scripts/test_check_gold.py` (3) | — | passed; `just deps` correctly red until the skill refresh |
| End-to-end pilot | **Measured** | `/usr/bin/time -v just pilot` ×2 | release build, warm environment and cache, acquisition a no-op, this host | 8.0 s, 1.64–1.68 GB, equal `release_id` and `content_digest` (at one path only, P1) |
| Carried identity triggers | — | `just adr revisit` | lists the identity triggers | missing (F7) |

**Cost accounting.**
- Closing F1 and F2 adds about 0.08 s per compile on the pilot (P3), under 1% of the total.
- `--link-mode copy` costs about 127 MB of disk per library environment on this non-reflink
  filesystem.
- Nothing else is material.

## 10. Exceptions and unresolved decisions

- **No SHOULD-level exception is recorded.**
- **The declared owned-bytes exclusion (ADR-0013 "What we give up") is not an exception
  candidate.** DM-31 is a MUST, and this is not a conservative representation of the dependency:
  it drops it. Measured at about 1%, it is also no longer a cost trade-off. Close it (F1). If the
  author keeps it:
  - narrow DESIGN L655, "So nothing ambient can change an answer without changing `context_id`",
    to exclude in-place edits of owned dependency bytes;
  - record the requirement as unresolved;
  - restore ADR-0007's revisit trigger, so the next occurrence reopens it.
- **Unresolved for increment 3:** where the docs source's digest enters identity (F6).

## 11. Decision

**Decision: Revise. ADR-0013 stays `proposed`.**

**Reason.** The structural decision is sound and well evidenced: libraries as committed uv
projects, `uv sync --frozen`, release modules from verified `RECORD`s, one `lctx` path and a gold
guard. The pilot reproduces: 275 modules, all rules passing, 8.0 s, a stable `release_id` and
`content_digest` at one path. `just check` passes.

Two derived identities break their contracts on in-scope behaviour, so G6 fails:
- **`context_id`** depends on where the environment sits (0 of 98,656 `fact_id`s survive a move).
  It is also blind to analyzer-visible files inside owned packages (P2). Both are regressions of
  problems fixed in slice 1.
- **`release_id`** is blind to git and hash-less sources, and it moves on a dependency-only
  upgrade.

The acquisition's hermeticity claim is also falsified (G4), and several labels outrun their tests
(G7). Every correction is local: `library.rs`, `config.rs`, `lctx/src/main.rs`, focused tests,
and sentence edits. Because ADR-0013 is `proposed`, it can be amended in place. Meanwhile ADR-0007
is already `superseded`, so the run contract rests on a proposed record until this closes; accept
ADR-0013 in the same commit as the F1–F3 fixes.

**The author has to decide:**
1. **`release_id`'s equivalence:** verified release content (recommended), or the lock identity
   restricted to the installed artifact. Either way, refuse hash-less release entries (F2).
2. **The environment digest:** verify and hash the analyzer-readable bytes (recommended, 0.076 s),
   or keep the exclusion with the narrowed claim and the restored trigger (F1, §10).
3. **Provenance writers,** and whether `distributions` moves to `context_id` (F4, a schema
   migration).
4. **The docs source pin:** the commit now, the identity slot at increment 3 (F6).

| Priority | Change | Principle IDs | Acceptance evidence | Regression protection |
|---|---|---|---|---|
| 1 | Canonical environment digest over analyzer-readable, `RECORD`-verified bytes; drop `..` entries (F1) | DM-15, DM-31, DM-32 | DESIGN §4.0 L631–L651 reworded; the P1 and P2 probes pass | tests F1 (i)–(iii) |
| 1 | `release_id` from verified release content; refuse hash-less release entries; state the equivalence (F2) | DM-15, DM-11, DM-43 | §3.4.1 row; P6 refused; P7 id stable | tests F2 |
| 1 | `--no-config --python <pin> --link-mode copy`; enforce `.python-version`; record the `uv` version; fix the remedy text (F3) | DM-28, DM-48, DM-30 | §4.0 L600–L605 reworded; P5 and P10 behave | stub-`uv` test; `pyvenv.cfg` test |
| 2 | One-sentence writer rule; `distributions.context_id` (F4) | DM-02, DM-13, DM-46 | §3.2 L365–L366, ADR-0013 L92 | generated reference rule, contract snapshot |
| 2 | Repository-root URL comparison, excluding sponsor links (F5) | DM-54, DM-59 | proposal for jsonschema is `["jsonschema"]` | unit test over METADATA fixtures |
| 2 | Restore ADR-0007's identity triggers and oracles in ADR-0013; lock-only `context_id` test; stale references (F7) | DM-59, DM-60 | `just adr revisit` shows them | `just adr lint`; new test |
| 3 | `commit` in `[tool.lctx.source]`; tag versus version check (F6) | DM-31, DM-48 | `libraries/fastmcp/pyproject.toml` | pytest or Stage A test |
| 3 | Relabel; add the Measured conditions (F8) | DM-59, DM-39 | DESIGN labels | prose |
| 3 | `just gold` separate from `just deps` (F9) | DM-47 | `just adr revisit` shows ADR-0002 passing during a skill lag | `just adr revisit` |

**Continuity with prior reviews.**
- **ADR-0012 review F9** (absolute locations in the context digest): resolved in slice 1,
  **regressed** here through the `RECORD` `..` entries (F1).
- **Slice-1 compact F2** (different site-packages, same identities): resolved in slice 1 by the
  full-content walk, **partly regressed** by the `RECORD` proxy (F1, P2).

### Deferred

| Item | Why deferred | Reopen when |
|---|---|---|
| Module → distribution link in `source_files` | Recoverable from module names today (`fastmcp-slim` owns `fastmcp/`, `fastmcp-tasks` owns `fastmcp_tasks/`), and no brief states an install condition | a brief needs to state a required extra (e.g. `fastmcp[tasks]`), or two release distributions share a top-level package |
| Lock forks (`resolution-markers`, the same name at two versions) | `lock()` keys by name, so the last entry wins; the version check fails closed, and the pilot lock has none | a library whose lock carries `resolution-markers` |
| `python_platform = std::env::consts::OS` (`"macos"` versus Pyrefly's `"darwin"`) | single operator on Linux; not verified | Stage A runs on a non-Linux host |
| Stage A diagnostics as one `ExtractError::Library(String)` | one operator reads them; F3 fixes the one misleading remedy | a caller needs to act on the failure class (auto-reinstall, CI) |
| `lock_digest` in `context_id` invalidates on lock changes irrelevant to the analyzer | conservative, allowed by DM-31; revisit with F1's canonical digest | spurious context changes obstruct §9.8 ablation joins |

## Disposition (author, 2026-09-22)

The author took the recommended option on all four open decisions:
1. `release_id` is the verified release content, and a hash-less release is refused.
2. Every distribution's analyzer-readable bytes are verified and hashed.
3. The extractor run writes `releases`, `distributions` and `source_files` once per attempt, and
   `distributions` is keyed by `context_id`.
4. The docs commit is pinned now.

ADR-0013 was amended in place and **accepted** in the same commit as these fixes, with evidence
Tested and ADR-0007's identity triggers carried into its `revisit:`.

**Checks** (2026-09-22):
- `just test-all`: passed. It covered nextest 63/63, pytest 21/21, ast-grep rule tests 4/4, adr lint (13), lint-agents, fixtures, family, cargo-deny, the fork check and `just gold` (ok).
- `just pilot`: passed (release `f454411b…`, every rule, 8.0 s).
- `just adr revisit`: ADR-0002 passes, and ADR-0013 lists the identity trigger.

**Schema migration** (no stored data beyond the rebuildable local store):
- `releases` adds `distributions` and `installer`;
- `distributions` is keyed `(snapshot_id, context_id, name)`, with `in_release` dropped;
- a reference rule `distributions.context_id -> contexts`;
- output version 5.

| ID | Outcome | Change | Oracle |
|---|---|---|---|
| F1 | fixed | **Environment digest:** the dist-info names plus every analyzer-readable file's path and content. Analyzer-readable means `.py`, `.pyi` and `py.typed`; Pyrefly's module finder reads `py.typed` but never `.pth`. `RECORD` `..` lines never enter. **Stage A** verifies every distribution's analyzer-readable `RECORD` entries and refuses a mismatch. The ownership computation is deleted. | `tests/library.rs`: two paths with location-dependent `RECORD`s give one `context_id`; an unowned `.pyi` inside a package moves it; an owned byte change is refused. **Real library:** two environment paths give the same `release_id` and `content_digest` (probe P1 inverted) |
| F2 | fixed (option i) | **`release_id`:** hashes the release distributions' names, versions and sorted (path, sha256) of their verified analyzer-readable entries. **Refusal:** a release the lock records without artifact hashes is refused. | `tests/library.rs`: a re-listed artifact leaves the id; changed content moves it; a git-sourced release is refused |
| F3 | fixed | **`acquire`:** passes `--no-config --python <.python-version> --link-mode copy`, and `--reinstall` is added as the remedy, now named in every Stage A error. **Stage A** refuses an interpreter drift. `releases.installer` records the installer from `pyvenv.cfg`. `init` locks without `--no-config`, as advised. | `crates/lctx/tests/acquire.rs` (stub `uv`: argv and environment); `tests/library.rs` (3.14.5 refused). **Pilot:** re-acquired with `--reinstall`, and every file's link count is 1 |
| F4 | fixed | **Writer rule** in DESIGN §3.2 and ADR-0013: the extractor run writes `releases`, `distributions` and `source_files` once per attempt, carrying Stage A's output. **`distributions`** is re-keyed by `context_id`. | generated `ref:distributions.context_id->contexts` and `key:distributions`; contract snapshots |
| F5 | fixed | `crates/lctx/src/propose.rs` compares repository roots (`host/owner/repo`) under source labels, excluding sponsors, orgs and users. A distribution with no repository is proposed alone, and the output says so. | unit tests: the FastMCP trio is grouped; a shared sponsor link does not group jsonschema and referencing; a distribution with no URL stands alone |
| F6 | fixed | `[tool.lctx.source] commit = "004bf15a2ba99f077160993c00a404e1a9da83ea"`: the `v4.0.5` tag, resolved through the GitHub API. Stage A requires a 40-hex commit and a tag naming the locked version. The identity slot (the docs run's context digest) is stated for increment 3. | `tests/library.rs`: a wrong tag and a missing commit are refused |
| F7 | fixed | ADR-0013's `revisit:` carries ADR-0007's identity triggers; the carried oracles are listed; `design:` gains §11; the stale `attempt.rs` comment is fixed. | `just adr revisit` shows the trigger; the lock-only-change test (the per-input `run_id` oracle) |
| F8 | fixed | DESIGN §4.0: the heuristic is Tested by unit tests, and the FastMCP and attrs runs are labelled "observed, not a repo test". The CLI's hermeticity is Tested by the stub `uv`. The Measured line states its conditions: warm environment and cache, acquisition a no-op, this host. | prose; the new tests |
| F9 | fixed | `just gold` is its own recipe, run by `just test-all`; `just deps` no longer runs it. | `just adr revisit`: ADR-0002 passes |

**Also, found while fixing:** a store created under an older contract failed with an opaque Delta
cast error. `delta::verify` now compares the stored schema with the declared one and refuses it as
`CoreError::SchemaDrift`, naming it a migration (`open_refuses_a_table_whose_schema_drifted`).

| Deferred | Why | Reopen when |
|---|---|---|
| The review's Deferred rows: module → distribution link, lock forks, `python_platform` spelling off Linux, typed Stage A errors, `lock_digest` granularity | as the review states | their stated triggers |
| A repo test that runs `lctx library init` and `compile` end to end with a real uv | It needs the network and a registry; the pieces are tested separately | a CI host with a package mirror |
