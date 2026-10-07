# Agent workspace effectiveness

**Status (2026-10-07): Proposed. The assessment and its independent review are complete, and the decisions await operator confirmation. No harness change has been implemented.**

The [independent design review](../design_review/reviews/design_review_agent-workspace-effectiveness_2026-10-07.md) concluded **Revise**: keep the main direction, but correct the contracts. Its F01–F08 are integrated below (§8). P1, D5–D7 and P7 can proceed once confirmed. P2–P5 carry the corrected contracts, plus two settling checks that run before implementation.

## 1. Context

The operator asked how the environment, command surface, tooling and agent configuration could make highly capable agents more *effective* here. Claude Code and Codex are weighted equally. "Effective" means:
- less up-front thinking about how to phrase a command;
- fewer complications caused by the environment;
- related actions reachable through one parameterised command;
- legible feedback;
- no tooling that boxes in an agent that already knows what it wants.

Machine-specific configuration is welcome, since this is a single-operator project on one workstation. The method adapts pse-arrow's Plan 29 assessment (2026-10-07) and the corrections from its independent review.

**Operator choices (2026-10-07):**
- Proposals are **repo-level only**. User- and machine-level observations are notes (§10).
- The **library catalog is retired** and is not assessed as a capability. Its leftover wiring is treated as a hazard (AE-19).
- The transcript miner lives in the [evidence folder](../design_review/evidence/2026-10-07_agent-workspace-effectiveness/README.md).

**Ownership.** This plan owns the harness:
- the environment and native-extension sync;
- fixtures as a command boundary;
- verify transparency;
- parallel checkouts;
- the freshness check;
- instructions and runtime configuration.

It does not own:
- test-family *coverage*, which belongs to the owner of the [testing architecture plan](testing-architecture-pivot-plan_2026-10-04.md);
- product performance, which belongs to the [persisted graph execution plan](persisted-graph-execution-plan_2026-10-07.md).

The other agent's persisted-pivot work, its tree and its STATUS ownership were not touched.

**Method.** All evidence is local. Raw artifacts are in gitignored `build/agent-effectiveness/`; aggregates and the miner are in the evidence folder. No transcript text entered the repository.

| Workstream | What it examined |
|---|---|
| W1 transcript mining | 88,235 shell commands in 113,321 events, from 259 Claude transcripts and 277 Codex threads (2026-09-22 to the 10-07T22:42Z cutoff), stratified by runtime × main/subagent × role × configuration period. **Current-period sample:** P4 (10-05) has 2 Claude main sessions (111 shell commands) and 10 Claude subagents. P5 (10-06 onward) has **no Claude activity**, and Codex is **one** main session plus 43 subagents. Current-period counts therefore largely describe one Codex coordinator's workflow, and runtime is confounded with role. W1 is a catalogue of mechanisms: it reports counts with their strata and denominators, not general rates. Command classes cited below were spot-checked (evidence README). |
| W2 command surface | `just --dump` (41 recipes); `verify.py`/`native_controls.py` families × boundaries; `cargo metadata` test targets; `pytest --collect-only` (337 tests) |
| W3 instruction and configuration load | The context a fresh session loads in each runtime; instruction bytes; duplication and staleness |
| W4 capability research | uv 0.12.22, maturin 1.15, nextest 0.9.146, Docker 29.8.2, Codex 0.161 and Claude Code 2.1.293 (Context7, tagged source, `--help`). pse-arrow's results at the same versions were reused. |
| W5 probes | Read-only `uv sync --dry-run/--check` with and without the native key; Python 3.12 parse check of `scripts/`; fresh-session context measurement; Docker daemon port across `restart` |
| W6 cold-start drills | Six read-only tasks, each run once by `claude -p --permission-mode plan` and once by `codex exec -s read-only`. Scored by the plan author against targets written beforehand (§11). This is qualitative evidence, not a baseline. |
| W7 isolation | Kernel/oomd journal since 09-22; cgroup placement; swap |

**Rubric.** Each proposal is judged against twelve properties:
1. The bare tool path is first-class; recipes and families are shortcuts.
2. Wrappers are thin and pass arguments through.
3. Selection is composable and reports its true coverage and cost.
4. Default output is concise; full logs go to unique paths; structured results are available on request.
5. Infrastructure failures can be told apart from code failures.
6. Behaviour is the same under any inherited environment.
7. Commands are safe to run concurrently, and contention is reported honestly.
8. Guards and caps can be overridden in-session, and effective limits are visible.
9. Tools are self-describing.
10. Tools offer capabilities, not mandated procedures.
11. Always-loaded text pays its way.
12. Both runtimes get the same capability, or the gap is stated.

**Standing preferences:**
- light tooling;
- no new alignment lints;
- judgment over evidence ceremony;
- low parallel test load;
- performance is not a design gate;
- every new default is overridable;
- each changed capability is exercised once, in its revealing case.

## 2. Scenarios

| ID | Scenario |
|---|---|
| S1 | Edit → compile → focused test in one crate |
| S2 | Select the tests a change affects, across families and the Rust/PyO3/MCP boundaries |
| S3 | Confirm that no generated output is stale |
| S4 | Store, serving or compiler control needing a disposable SurrealDB and the native extension |
| S5 | Scope-end leaves or `qualify` |
| S6 | Diagnose a failure as environment-shaped or code-shaped |
| S7 | Several agents (Claude, Codex, subagents) building and testing at once |
| S8 | Library API lookup and correct use |
| S9 | Session start or resume |

**Consequential variations:**
- system vs venv `python3`;
- `just`-wrapped vs bare `uv`;
- an edited native-extension input;
- a second concurrent run;
- a worker copy or worktree;
- an interrupted, timed-out or SIGKILLed launcher;
- a fixture restart;
- enabling an optional runtime capability.

## 3. Overall reading

In the current period, few commands fail outright with environment errors: 23 in P4–P5. The agents have learned the traps. The friction is now paid as **workaround construction**:
- hand-assembled environments;
- private fixture drivers;
- invented build locks;
- private build directories with sccache disabled;
- ad hoc copies of the checkout;
- sleep-polling of long runs.

Much of the current-period evidence for the copies, locks and polling comes from one Codex coordinator's workflow.

The drills show the same pattern. Both runtimes reached correct answers to all six tasks. They spent their turns working out what the wrappers do (sync, Docker, locks, hard-coded packages) and assembling multi-command procedures the repository does not offer.

The highest-value changes therefore make those workarounds unnecessary:
- one coordinated synchronizer for the native extension;
- one fixture boundary under bare commands, with owned lifetimes;
- a transparent verify;
- a supported route for parallel checkouts;
- lighter, current instructions.

## 4. Findings

Evidence labels follow design principles §D. "Agent-h" is summed command wall time; it includes the work itself, and subagent time overlaps. W1 counts are historical workload evidence, not measurements of any remedy. Every finding was re-checked against HEAD `1be0dd96`.

### Environment and the native extension (S1, S4, S6)

**AE-01 The native extension's uv cache key differs between `just` and bare invocations, so the 175 MB extension is rebuilt back and forth.**
*Interface-checked* (uv 0.12.22 `cache_info.rs`). *Tested* 2026-10-07 with a read-only `uv sync --project … --frozen --dry-run` and `--check --inexact`, run with the key unset and then set.
- `python/lctx_semantics/pyproject.toml` declares `{ env = "LCTX_NATIVE_SEMANTICS_INPUTS" }`. uv records the value (null when unset) and rebuilds on any difference.
- The justfile shell (`scripts/build_environment.py`) always exports the key. Bare `uv run` and the MCP launchers do not.
- The installed build records `null`. With the key set, `--check` reports "environment is outdated" and plans a lctx-semantics reinstall.
- So the next syncing recipe (`fmt`, `types`, `adr-lint`, `deps`, `gold`, `doctor`) rebuilds the extension, and the next bare syncing `uv run` rebuilds it back.
- File keys compare ctime, so any edit to lctx-model, lctx-surrealdb or lctx-serving sources also makes the next syncing `uv run` rebuild.
- W1: 813 syncing `uv run` commands before P4, 0 in P4–P5 (agents learned `--no-sync`).
- Related:
  - `maturin>=1.15.0` is an unpinned build requirement absent from `uv.lock`, against the exact-pin rule.
  - A stale `lctx-storage` editable install, no longer a workspace member, survives inexact syncs. The untracked `python/lctx_storage/` holds only `__pycache__`.

**AE-02 Verify readiness itself synchronizes the shared environment.** *Interface-checked.*
- `verify.py::prepare` runs `uv sync --locked --inexact --only-group dev` for every family except model and analytics. For serving it runs a full `uv sync --locked --inexact`, which can rebuild the extension.
- It does so under `.venv/.verification.lock`, held for the whole run with a silent wait.
- AGENTS.md:251 says never to synchronize while native guards/workers are live. The families can violate that for other agents' workers.
- All D1–D4 drill answers, in both runtimes, flagged the sync as a surprise. Codex routed around `verify-store` for that reason.

**AE-03 Nothing reports whether the installed native extension matches its sources.** *Measured* (W1); *Interface-checked*.
- 10 failures (8 in P4–P5) imported a symbol the installed extension lacked.
- uv does not recompile before import, and the module exposes no build identity.
- A cheap read-only observation already exists and no recipe uses it: `uv sync --locked --check --inexact` with the key set, or the build-time key in the installed `uv_cache.json`.

**AE-04 The interpreter depends on how the agent was launched.** *Measured*; *Tested* 2026-10-07 (3.12 parse check of every `scripts/*.py`).
- `/usr/bin/python3` is 3.12.3.
- Five scripts fail to parse under 3.12 because they use PEP 758 `except A, B:`: `surrealdb_fixture.py`, `check_family.py`, `build_measurements.py`, `programmatic_eval.py` and `library_catalog_hints.py`.
- Three places invoke `surrealdb_fixture.py` through bare `python3`: its docstring, `docs/surrealdb.md:179` and `build_measurements.py:484`.
- Five P5 Codex failures hit this in worker copies.
- This session's Claude and Codex both resolved `python3` to the venv, through `VIRTUAL_ENV` and `PATH` respectively, so the trap appears in other launch contexts.
- The justfile shell, `build_environment.py`, runs under the system Python. It currently parses under 3.12 and must keep doing so.

**AE-05 Agents assemble the environment by hand because the instructions prescribe it.** *Measured* (W1).
- `eval "$(python3 scripts/build_environment.py --shell)"` appears in 1,089 commands (402 in P4–P5); the `build_environment.py --` wrapper appears in 2,442.
- AGENTS.md:141 prescribes the eval before bare Cargo or uv. Five of the twelve drill answers began with it.
- For Cargo, its only effect is to normalize an inherited foreign `CARGO_TARGET_DIR`; `.cargo/config.toml` already supplies the toolchain, cache, linker, jobs and build directory.
- For uv, its effect is the AE-01 key.

### Fixtures as a command boundary (S1, S4, S6)

**AE-06 Bare `cargo nextest` cannot run fixture-backed tests, and the one fixture wrapper covers half the environment.** *Interface-checked*; *Measured*.
- About 30 `expect()` sites in 19 files need `LCTX_SURREAL_TEST_CONFIG` or `LCTX_COMPILER_RUNTIME_CONFIG`.
- `surrealdb_fixture.py -- CMD` supplies only the first. The second exists only inside `native_controls.py:54-66`.
- Agents reconstructed it by hand:
  - 273 commands set `LCTX_*` inline. The most frequent was `LCTX_SURREAL_TEST_CONFIG`, pointed at a long-lived `/tmp` runtime (82 in P4–P5).
  - Private `/tmp` driver scripts re-implement the compiler-runtime fixture: 93 runs and 14.7 agent-h in P4–P5.
- nextest setup scripts, which could inject the environment, are experimental in 0.9.146.

**AE-07 The fixture's lifecycle leaks, races and caps silently.** *Interface-checked*; *Measured* (journal); *Tested* 2026-10-07 (Docker port across `restart`).
- **Leaks.** SIGTERM (sent by timeouts) or SIGKILL of the launcher skips `finally`, so the detached container and scratch directory survive. Containers carry no label to find them by.
- **Port race.** The port comes from bind-0, then close, then `--publish`.
- **Restart moves the port.** A daemon-allocated port (`-p 127.0.0.1::8000`) changed on `docker restart` (33192 → 33193, probe). `native_controls`' mcp path restarts the server after a Rust test has written the port into `serving.json`, so the daemon port is unsafe there.
- **Silent memory cap.** The 1 GB cap is silent. On 2026-10-07 at 16:13 the kernel OOM-killed a uid-1000 `surreal` container at 1.04 GB RSS, and an agent sees only a connection failure (S6).
- **Outside agent cgroups.** Containers run in `system.slice`.
- **A native `surreal` 3.3.0 binary is also installed.**

### Selection, cost and feedback (S1, S2, S5)

**AE-08 Families are coarse and slow, so agents bypass them.** *Measured* (W1, n < 30 per family); *Interface-checked*.
- Observed p50 wall times:

  | Family / boundary | p50 |
  |---|---|
  | serving | 1,065 s |
  | compiler/producer | 1,222 s |
  | mcp | 1,649–5,625 s |
  | filtered 6-test store run | 317–835 s |

- P4–P5 had 21 `just verify-*` calls and 8 direct `native_controls` calls, against 114 bare and 38 inline-`LCTX_*` cargo test runs.
- Causes at HEAD:
  - every run syncs and takes the lock (AE-02);
  - mcp always builds lctx-eval and runs the whole `native_journey`, and only then applies filters to pytest (`native_controls.py:96-105`);
  - the tooling docs boundary runs the release doc-tests for the whole workspace;
  - no nextest thread default suits concurrent agents, so agents hand-set `NEXTEST_TEST_THREADS`.

**AE-09 Agents cannot see what a family will run.** *Interface-checked*; drills D1, D2, D4.
- The store family hard-codes `-p lctx-surrealdb -p lctx-publisher`, and explicit Cargo targets replace the defaults (`native_controls.py:21-25`).
- Multi-boundary families reject filters without `--command`.
- Three of six drill answers were unsure whether `--test X` is valid when X exists in only one hard-coded package, and fell back to `-E 'binary(...)'`.
- Nothing prints the resolved commands, the environment names or the prerequisites.

**AE-10 Long runs give no handle, so agents poll and scatter logs.** *Measured* (W1; P4–P5 counts mostly describe the P5 Codex coordinator and its subagents).
- `verify.py` streams child output and writes no logs and no structured result.
- Codex `clock.sleep`: 1,659 calls and 17.3 agent-h (806 calls and 8.0 h in P4–P5). Also 3,300 `ps`/`pgrep` checks and 1,345 `wait_agent` calls.
- 4,170 build/test runs were redirected to ad hoc files (693 in P4–P5; 1,065 Codex redirects to `/tmp`). 1,797 of them were followed by a log read.
- Claude's 2–10 min Bash ceiling sits below every family's p50 (AE-08).

**AE-11 "Is anything generated stale?" has no answer short of regenerating.** *Interface-checked*; drill D3.
- Both runtimes assembled about 12 commands, and Codex declared the task blocked.
- `deps` mixes dependency policy with Hakari drift.
- `docs-check` rebuilds the site.
- Snapshots are checked only by running their tests.
- Formatting has no check recipe.
- `skills-check` sits outside `qualify`.
- `crates/lctx-model/tests/snapshots/ids__*.snap` are orphaned (their test was deleted in `91751d1c`).

**AE-12 Families leave tests unrun and run tests that cannot pass.** *Interface-checked*; routed (§9).
- Tests no family runs:
  - 53 of 61 cpg-extract integration targets;
  - cpg-flow `call_paths`;
  - lctx-embed `client`;
  - lctx-eval, lctx-semantics and lctx-model-macros, except their doc-tests;
  - 13 MCP pytests (`test_embedder`, `test_embed_serve`, `test_transport_envelope`).
- The tooling family collects 71 tests of the retired library catalog, and `test_programmatic_native*.py`, which assert rather than skip when serving configuration is missing.
- For agents, this means a family name implies coverage it lacks (rubric 3).

### Parallel work in one checkout (S7)

**AE-13 One Codex coordinator's workflow invented build serialization and isolation.** *Measured* (W1, mostly the P5 Codex coordinator and its subagents).
- 164 build/test commands were wrapped in hand-made `flock` on a shared `/tmp` lock: 17.8 agent-h, p50 140 s, p90 884 s.
- "Blocking waiting for file lock" appeared in 28 of 325 Codex-subagent build/test commands in P4–P5.
- 747 commands (154 in P4–P5) pointed Cargo at private build/target directories, and most also disabled sccache, which forced cold builds.
- 4,688 P4–P5 commands ran in copies under `~/.cache/lctx-*` or `/tmp/lctx-remediation-*`, all from Codex subagents. There were also 24 `git worktree add`, 19 removes and 65 `git apply`.
- 23 commands pointed a copy's `UV_PROJECT_ENVIRONMENT` at the main `.venv`. A worker may therefore import the main checkout's extension (inferred).
- At HEAD:
  - AGENTS.md:338 allows worktrees only to "edit production code concurrently".
  - No recipe prepares a parallel checkout.
  - Fine-grain locking (on since 09-28) is established for check-type commands only.
  - Every checkout shares one build directory (ADR-0079).
  - Workspace crates build incrementally and sccache does not cache them, so a new worktree builds them once.

### Instructions and runtime configuration (S8, S9, all)

**AE-14 Always-loaded context is large, and part of it is unrelated or stale.**
*Measured* 2026-10-07, fresh `-p` sessions with `--max-turns 1`:

| Runtime | First-request context |
|---|---|
| Claude, full | 47.2k tokens (36.9k cache creation + 10.4k cache read) |
| Claude with `--setting-sources project,local --strict-mcp-config` | 36.5k |
| Codex | 32.8k input tokens |

- Historical Claude main sessions started at about 55k tokens (P0–P3, n = 22).
- Superpowers' SessionStart injection appeared in 16 of 35 Claude main sessions. It mandates its own brainstorming and planning workflow, which competes with the repository's process skills.
- The D6 Claude drill was distracted by nine unauthenticated connectors.
- AGENTS.md is 28,762 B. Its 6.5 KB preamble is a status narrative that drifts.
- Codex counts only project docs against `project_doc_max_bytes` (32,768), leaving 4 KB of headroom (*Interface-checked*, Codex 0.161 `agents_md.rs`).
- Codex history: 659 compactions and 3,361 truncated-output warnings (1,474 in P4–P5).
- The project-layer configuration keys proposed in P7 are *Interface-checked* until P7's fresh-session acceptance:
  - Claude `enabledPlugins`/`disableClaudeAiConnectors`/`skillListingMaxDescChars`/`env`;
  - Codex `project_doc_max_bytes`/`shell_environment_policy`/`mcp_servers.*.enabled`.

**AE-15 The instructions contradict themselves and route to stale owners.** *Interface-checked.*
- AGENTS.md both prescribes `just qualify` for this pivot (:118, :254) and forbids it (:242).
- It routes the testing boundary and operator state to the earlier coordinator's §9 (:239).
- Its testing rule says families "prepare their actual prerequisites".
- `.agents/roles/test-agent.md:8` still says "real PostgreSQL tests".
- The skills README cites pyrefly 1.3.1, ruff 0.0.11 and ty 0.0.14. It lists the unselected sqlx-postgres and has no rows for neo4j-surrealdb, pydantic or salsa.
- `docs/README.md`'s library route starts at the retired library-utilization catalog.
- The process-skill list is kept in four places, and the principles paragraph is duplicated in seven roles/skills.
- Agents invoked 26 recipe names that no longer exist.

**AE-16 Project memory is partly stale (Claude only).** *Interface-checked.*
- About 7 of 27 entries describe retired mechanisms:
  - the Postgres superuser;
  - after-turn hooks;
  - `just hygiene`;
  - analyzer-shift status;
  - library-skill selection.
- The library-catalog entry was corrected on 2026-10-07.

**AE-17 Agents read paths that don't exist.** *Measured* (W1).
- There were 1,017 probe misses in P4–P5, out of 17,840 shell commands in that period.
- 486 of them were guessed files in directories that exist, mostly lctx-model (291) and cpg-core (89).
- 72 confused `mod.rs` with `foo.rs`; 55 pointed at removed crates or retired docs.
- lctx-model has about 300 source files and a mixed module layout.
- Semantic navigation (rust-analyzer LSP for Claude, MCP for Codex) is configured at user level, but no transcript shows it used.

**AE-18 Codex's skills scan reports a truncated inventory at every start.** *Interface-checked* (Codex 0.161 `ext/skills`); *Tested*.
- The symlinked library skills hold about 1.26M files, which exceeds the scan caps (depth 6, 2,000 dirs, 20,000 entries).
- The scan is breadth-first, so all 27 skills still load (tested).
- The cost is one error line and a latent stale nested `SKILL.md`. Low impact.

**AE-19 The retired library catalog is still wired in, and its wiring syncs.** *Interface-checked.*
- `.codex/config.toml` enables a `library-catalog` MCP server for every Codex thread and subagent via `uv run --frozen`. `--frozen` still syncs, and does so without the AE-01 key.
- `.mcp.json` declares the same server; Claude disables it locally.
- Also left over: the recipe, `scripts/library_*.py`, `tools/lu-resolve`, `docs/library-utilization.*` and 71 tooling tests.

**AE-20 Stale artifacts.** *Interface-checked.*
- Two committed 0-byte `rustc-ice-2026-10-01*.txt` files.
- A committed root file named `1`.
- The `justfile:144` "just pilot" comment.
- The `Cargo.toml:112,138` delta comments.
- Five Postgres allow rules in `.claude/settings.json`.
- 2.8 GB of ad hoc logs in `build/`.
- `docs/pins.md` records nextest 0.9.144 against the installed 0.9.146.

## 5. Decisions for confirmation

None of these alters a §B decision or an accepted ADR's decision. The wording changes are tooling and instruction changes (§6). Review findings shaped each contract (§8).

### D1 — Repository launchers never sync implicitly; one coordinated synchronizer; readiness observes

- **Explicit `--no-sync`.** Every launcher the repository defines carries an explicit `--no-sync`: recipes, scripts, nested launches (e.g. `embed_serve.py`'s `uv run --project services/vllm`) and any future MCP registration. That behaviour does not depend on the runtime environment.
- **`UV_NO_SYNC=1` for ad hoc commands.** It is a default for an agent's own ad hoc `uv run`, set in both projects:
  - Claude: project `.claude/settings.json` `env`. It reaches every child process, including MCP servers.
  - Codex: `.codex/config.toml` `shell_environment_policy.set`. It reaches shell commands only.
  - IDE terminals and plain shells get neither, and keep today's behaviour.
  - Override with `UV_NO_SYNC=0` or `false`, which uv honours (tested).
  - The justfile shell wrapper drops `UV_NO_SYNC` for recipes so `--no-project` recipes stop warning.
- **`just sync` is the only synchronizer.** It runs `uv sync --locked --inexact` with the native key always set, and removes stale members such as `lctx-storage` when asked. `just ready` calls it.
- **Shared and exclusive environment ownership.**
  - Verify runs and fixture runs hold `.venv/.verification.lock` *shared*. Two verify runs overlap.
  - `just sync` takes it *exclusively*. While shared holders are live, it reports them (pid, family or command, start time) and waits.
  - Serializing verify runs against each other, if wanted, is a separately named policy rather than an accident of the lock.
- **Readiness observes, scoped per prerequisite.**
  - A family checks only what its boundary imports. The extension's freshness matters to `native-python` boundaries; the store family is not blocked by a stale extension.
  - The check keeps `--locked`: `uv sync --locked --check --inexact` with the key.
  - A failed check reports `blocked: run just sync`.
  - `qualify` no longer prepares the environment.
- **Optional build identity.** The extension may expose `build_identity()`, the embedded inputs hash, for observation. A stale import warns rather than fails unless a strict mode is set. It is overridable, an observation and never new authority.

### D2 — One fixture boundary with defined lifetimes makes bare commands first-class

`just fixture … -- <cmd>` runs any command with every native fixture variable supplied: `LCTX_SURREAL_TEST_CONFIG`, `LCTX_COMPILER_RUNTIME_CONFIG`, and optionally the retained serving fixture. It is the single owner of the fixture and runtime setup, built on `surrealdb_fixture.py` with the runtime setup lifted from `native_controls.py`. Families delegate to it.

- **Three lifetimes:**
  - **Run-owned** (default): created for one command, removed when it ends.
  - **Kept** (`--keep`): survives for later commands. It returns an ID, and `just fixture --attach ID -- <cmd>` gives each attached command its own fresh cache database and selection file. Printed exports do not persist between agent commands in either runtime.
  - **Orphaned**: a run whose owner process has died. Labels carry the run ID, owner pid and kind. A sweep (`just fixture --sweep`, also run at fixture start) removes only orphans whose owner is dead, never another agent's live or kept fixture.
- **Signals.** On SIGTERM/SIGINT, the launcher:
  1. kills the child's process group;
  2. inspects the container (an `OOMKilled` state is reported as an infrastructure failure);
  3. removes the container.

  SIGKILL leaves an orphan for the sweep.
- **Port.** The launcher owns a stable explicit port with retry on collision. A daemon-allocated port changes on restart (tested), so it is not used. No `--rm`: it would not remove a running orphan, and it would erase the OOM state.
- **Memory.** `LCTX_FIXTURE_MEMORY` overrides the 1 GB cap, scaling `SURREAL_MEMORY_THRESHOLD`. The effective value is printed.
- **Invocation.** Docs and docstrings use `uv run --no-sync python` (AE-04), and scripts keep 3.14 syntax.

### D3 — Parallel checkouts get a supported route; no build lock

- **`just worktree <name> [--ref R] [--carry]`** creates a git worktree at `~/library-context-wt/<name>` from a printed ref. Uncommitted work moves only with an explicit `--carry` patch.
- **`just prepare`** works in any checkout. It builds that checkout's own `.venv` and extension through `just sync`, links skills (`just ready`), and prints the interpreter, the extension's import origin and the shared build directory.
- **`just worktree-remove <name>`** removes the worktree.
- **What is claimed:**
  - an isolated source tree, venv and extension;
  - checks that overlap without blocking on disjoint units;
  - shared dependency artifacts through the common build directory and sccache.
- **Not yet claimed:** non-blocking concurrent *artifact* builds. That waits for the settling probe in P5 step 0. If shared-directory builds serialize materially, the alternative is a per-worktree build directory with sccache on.
- **Getting work back to main:** commit on the worktree branch, then merge or cherry-pick to main (RC06).
- **No repository build lock** (see §7.1).

### D4 — Verify becomes transparent and truthful about coverage

- **`--print`** is static. It shows each boundary's resolved commands from the family's actual package set, the environment *names* it sets, its prerequisites and its readiness observations. It builds nothing.
- **`--list`** is labelled as building. It resolves exactly which tests a filter reaches (`cargo nextest list`), and reports invalid target and package combinations.
- **Run directory.** Each run writes `build/verify/<timestamp>-<random>/` containing per-command logs, a live `status` file and `summary.json` (outcome, duration, command and fixture identity). The summary can link nextest JUnit instead of re-deriving per-test detail. Paths are printed. Output is concise by default, and `--live` streams. The exit status is preserved. A retention rule keeps the newest N run directories, and `--prune` removes older ones.
- **Lock waits** are announced under D1's shared/exclusive semantics.
- **mcp fixture reuse.** The mcp boundary may reuse a retained serving fixture only when its inputs fingerprint matches. The skipped `native_journey` is then recorded as `not_run` with the reused fixture's identity, so the run never reports a fresh `passed` for it. `qualify` refuses reuse.
- **Thread count.** An overridable nextest `test-threads` default suits concurrent agents.

### D5 — AGENTS.md carries durable rules; STATUS carries status

- The volatile preamble narrative moves to its STATUS or plan owners, coordinated with STATUS's current writer.
- The AE-15 contradictions and stale routes are fixed.
- `project_doc_max_bytes = 65536` in project `.codex/config.toml` gives headroom.
- Compactness is a target, not proof of effectiveness. Task routes and consequential constraints stay.

### D6 — Unrelated Claude plugins and connectors are off for this project

Project `.claude/settings.json` sets:
- `enabledPlugins` false for superpowers and the unrelated synced plugins;
- `disableClaudeAiConnectors: true`;
- `skillListingMaxDescChars`.

Each can be re-enabled per session or in `settings.local.json`.

### D7 — Remove the retired catalog's agent wiring

- Remove the Codex MCP server, `.mcp.json`, the `settings.local.json` entry and the recipe.
- The tooling family stops collecting the catalog's tests.
- Whether to delete the catalog's scripts, `tools/lu-resolve`, tests and docs is the operator's choice. The default is to delete them, since the functionality is retired.

## 6. Rule changes

These take effect only when confirmed.

| ID | Current rule / location | Change | Packet |
|---|---|---|---|
| RC01 | AGENTS.md:141 prescribes `eval build_environment` before bare Cargo/uv | Bare `cargo` needs nothing. Repository launchers carry `--no-sync`, and ad hoc `uv run` defaults to non-syncing in agent runtimes. The eval remains only for an IDE shell that inherits foreign target paths. State each runtime's reach. | P2, P7 |
| RC02 | AGENTS.md Commands/Testing: families are *the* route for contract controls | Families are named shortcuts. `just fixture -- <bare command>` is first-class. State what each layer supplies. | P3, P7 |
| RC03 | AGENTS.md:338 limits worktrees to concurrent production edits | Worktrees are the ordinary route for parallel builds and tests, within D3's established claims. Fully merged worktrees are still cleaned up. | P5, P7 |
| RC04 | AGENTS.md:251 "never synchronize while native guards/workers are live" | Keep it, and name the mechanism: `just sync` takes exclusive ownership and reports live shared holders (review RC01) | P2, P7 |
| RC05 | AGENTS.md preamble holds current status | Status lives in STATUS and plans; AGENTS.md holds durable routes and rules | P7 |
| RC06 | AGENTS.md Git: work and commit on `main` in the current tree | Add the integration route for worktree work: a branch, then merge or cherry-pick to main, or an explicit patch (review RC02) | P5, P7 |
| RC07 | AGENTS.md Testing "families … prepare their actual prerequisites"; the `verify.py` docstring | Families observe their prerequisites and report `blocked`. Preparation is `just sync`/`just ready`, and `qualify` does not self-prepare (review RC03). | P2, P7 |

## 7. Packets

**Sequencing follows dependencies, not a fixed order:**
- P1 is independent.
- P2 (synchronizer and lock mode) precedes P3 and P4.
- P3's lifetimes and port policy precede P4's mcp reuse.
- P5 depends on P2.
- P7's wording follows the behaviour it describes.

**Rules for every packet:**
- Acceptance exercises each changed capability once, in its revealing case, and deletes what it replaces in the same change.
- No packet adds a lint.
- Every new default is overridable, and its effective value is visible.

| Packet | Responsibility / dependencies | Acceptance (revealing case) | Replaces / deletes | pse-arrow precedent |
|---|---|---|---|---|
| P1 Stale cleanup and catalog wiring | <ul><li>AE-20 artifacts, the orphaned `ids__*.snap`, `python/lctx_storage/`.</li><li>Fix `test-agent.md`, the skills README, the `docs/README` library route and the `docs/pins.md` nextest entry.</li><li>Remove the Postgres allow rules.</li><li>D7 wiring, plus scripts/tests/docs if confirmed.</li></ul> | <ul><li>`just lint-agents` passes.</li><li>A fresh Codex session starts no catalog server.</li><li>The tooling family collects no catalog tests.</li></ul> | Catalog wiring; ICE files; orphans | P1 (adopt) |
| P2 Synchronizer, lock mode and readiness | **Step 0:** read maturin 1.15's editable write path, to know how the in-tree `.so` is replaced and whether `just sync` must refuse rather than wait while shared holders are live. Then:<ul><li>D1, RC01/RC04/RC07: explicit `--no-sync` on every repository launcher; `UV_NO_SYNC=1` in both project runtime configs; the wrapper drops it for recipes.</li><li>`just sync`, holding the lock exclusively and reporting holders; verify/fixture hold it shared.</li><li>Scoped `uv sync --locked --check --inexact` readiness reporting `blocked: run just sync`.</li><li>Pin maturin via `[tool.uv] build-constraint-dependencies`.</li><li>Optional `build_identity()`.</li><li>Note in `build_environment.py`'s docstring that it must parse under the system Python (comment, no lint).</li></ul> | <ul><li>In fresh Claude and Codex sessions, `just fmt` then a bare `uv run python -c 'import lctx_semantics'` causes no rebuild.</li><li>After touching a lctx-surrealdb source, a serving readiness check reports the stale extension as blocked while the store family is not blocked. `just sync` rebuilds once.</li><li>`just sync` during a live verify run reports the holder and waits.</li><li>Two verify runs overlap.</li><li>`UV_NO_SYNC=0 uv run …` syncs.</li></ul> | Incidental sync in recipes, `prepare()` and MCP launchers | P2 boundary (adapt) |
| P3 Fixture boundary | D2:<ul><li>The `just fixture [--keep \| --attach ID \| --stop ID \| --sweep] -- <cmd>` owner.</li><li>Labels carrying run ID, owner pid and kind.</li><li>Owner-liveness sweep.</li><li>Process-group signal handling with OOM inspection.</li><li>A stable launcher port with retry.</li><li>The memory override with a scaled threshold.</li><li>`native_controls.py` delegates.</li><li>Fix the bare-`python3` callers (docstring, `docs/surrealdb.md`, `build_measurements.py:484`).</li></ul> | <ul><li>Bare `just fixture -- cargo nextest run --release -p cpg-core --test graph_artifact` passes without hand-set `LCTX_*`.</li><li>`timeout -s TERM` leaves no container.</li><li>A SIGKILLed run is removed by the next sweep while a concurrent live run and a kept fixture survive.</li><li>The mcp path works across `restart()`.</li><li>A forced small memory cap reports OOM as an infrastructure failure.</li><li>Two concurrent fixtures get distinct ports.</li></ul> | Hand-built `LCTX_*`, private driver scripts, the bind-0 picker | — (applies pse-arrow review F03 run ownership) |
| P4 Transparent verify | D4:<ul><li>Static `--print`.</li><li>Building `--list`.</li><li>Run directories with `summary.json`, `status`, printed paths and retention.</li><li>Concise default with `--live`.</li><li>Lock announcements.</li><li>Fingerprint-validated mcp reuse recorded as `not_run`.</li><li>nextest `test-threads` default.</li></ul> | <ul><li>`just verify-store --print -- --test cache` shows the resolved command without building.</li><li>`--list` shows which package owns `cache`.</li><li>A two-boundary run with one failure prints both log paths and exits non-zero.</li><li>A reused-fixture mcp run reports the journey `not_run` with fixture identity.</li><li>`qualify` refuses reuse.</li></ul> | Inline streaming; the silent lock | P7 logs/feedback (adapt) |
| P5 Parallel checkouts | **Step 0:** settling probe, run in a temporary worktree at a quiet time. Two concurrent release builds of disjoint workspace crates on the shared build directory, compared with a per-worktree build directory with sccache on. It decides D3's build claim. Then:<ul><li>D3 and RC03/RC06: `just worktree`/`worktree-remove` (with `--carry`) and `just prepare`.</li><li>`.worktreeinclude` for Claude worktrees, if gitignored inputs are needed.</li></ul> | <ul><li>`just worktree x` then `just prepare` there yields `import lctx_semantics` resolving to its own sources, checked from a session started in main.</li><li>Checks in two checkouts overlap.</li><li>The step 0 build result is recorded with its conditions.</li><li>Work returns to main by cherry-pick.</li><li>Removal cleans up.</li></ul> | Ad hoc `~/.cache`/`/tmp` copies; `UV_PROJECT_ENVIRONMENT` pointed at main; private build dirs without sccache | P3 worktree (adopt) |
| P6 Freshness check | AE-11. `just fresh`, read-only:<ul><li>Hakari `--diff`/dry-run;</li><li>`adr.py lint`;</li><li>`skills-check`;</li><li>`cargo fmt --check`;</li><li>`ruff format --check`;</li><li>the gold check;</li><li>a static orphaned-snapshot hint, labelled heuristic.</li></ul> It names the outputs only a test run can confirm (insta). | <ul><li>Clean on a clean tree.</li><li>A touched Hakari input or an unformatted file is named with its regenerate command.</li></ul> | Hand-assembled lists | P6 codegen-check (adapt) |
| P7 Instructions and runtime configuration | D5/D6 and RC01–RC07 wording:<ul><li>Move the status narrative out of AGENTS.md.</li><li>Fix AE-15.</li><li>One owner for the process-skill list and the principles paragraph, with short deliberate overlap where an isolated worker needs it.</li><li>Project `.claude/settings.json`: `env.UV_NO_SYNC`, `enabledPlugins` false, `disableClaudeAiConnectors`, `skillListingMaxDescChars`.</li><li>Project `.codex/config.toml`: `project_doc_max_bytes`, `shell_environment_policy.set.UV_NO_SYNC`.</li><li>Update or delete the AE-16 memories.</li><li>A short semantic-navigation note on when the rust-analyzer LSP/MCP is worth using (AE-17).</li></ul> | <ul><li>Fresh `claude -p` and `codex exec` sessions: re-measure first-request context; no superpowers injection or unrelated connectors; D6 is answered correctly.</li><li>A per-session re-enable works.</li><li>`just lint-agents` passes.</li><li>The project-layer keys move from Interface-checked to Tested.</li></ul> | Duplicated paragraphs; status narrative in AGENTS.md | P8 (adopt) |

### 7.1 Not proposed

| Idea | Reason |
|---|---|
| A repository build lock or `just serial` wrapper | fd-inheriting locks have a deadlock history (pse-arrow). Cargo serializes its own units, and worktrees (P5) cover the need. |
| Per-agent `CARGO_TARGET_DIR` or build directories as the default | That loses the shared build directory (ADR-0079). It stays the P5 step 0 alternative if shared-directory artifact builds serialize materially, and then with sccache on. The cold builds W1 saw came from agents disabling sccache. |
| nextest setup scripts to inject the fixture | They are experimental in 0.9.146. P3's thin wrapper works with any command. |
| Rewriting scripts to parse under Python 3.12 | It fights ruff's 3.14 target. Routing through `uv run --no-sync` is simpler. `build_environment.py` is the one exception and is documented. |
| A native `surreal` child process instead of Docker for run-owned fixtures | It would tie the server's life to the launcher, keep the port and account memory in the agent's cgroup (review §9). It is held because the Docker image digest is the exact parity contract, and the binary's provenance is not yet named as exactly. Revisit if P3's sweep proves insufficient or the binary can be pinned by digest. |
| An agent systemd slice or per-command scopes | No library-context agent process was OOM-killed in the journal; the only local kill was the fixture's own cap (P3). `pse.slice` is shared and user-level (§10). |
| A shallow `.agents/skills` copy | All 27 skills load; the cost is one log line. |
| A non-bypassable import gate | It would be new authority. `build_identity()` stays an overridable observation (D1). |
| `--message-format libtest-json` | Experimental. The runner's `summary.json`, optionally linking nextest JUnit, suffices. |
| Normalizing the `mod.rs` layout | It would need a lint to keep. |
| An "affected tests" selector now | Both drill runtimes found the 20–21 affected tests in about 2 minutes. P4's `--print`/`--list` and truthful family coverage (§9) come first. Revisit if S2 cost grows. |

## 8. Independent review integration

The [review](../design_review/reviews/design_review_agent-workspace-effectiveness_2026-10-07.md) concluded Revise. G5 (consistency and recovery) and G7 (truthful capability claims) failed, and the main direction was retained. Every finding is accepted into the contracts above.

| Review finding | Disposition | Where |
|---|---|---|
| F01 Fixture run ownership (kept, orphaned, restarted) | Accepted. Three lifetimes, an owner-liveness sweep, process-group signals with OOM inspection, a stable launcher port, no `--rm`. Its port question was settled by a Docker probe on 2026-10-07: the daemon port changes on restart. | D2, P3, AE-07 |
| F02 Lock meaning once readiness stops syncing; `just sync` uncoordinated | Accepted. Shared lock for runs, exclusive for `just sync` with holder reporting. The maturin replacement question is P2 step 0. | D1, D4, P2, RC04 |
| F03 Asymmetric reach of `UV_NO_SYNC` | Accepted. Explicit `--no-sync` on every repository launcher; runtime env only for ad hoc commands; reach stated; fresh-session acceptance. | D1, P2, RC01 |
| F04 Worktree claims beyond the mechanism | Accepted. The claim is restated; P5 step 0 probe; `just prepare` split; `--carry`; the route back to main. | D3, P5, RC03, RC06 |
| F05 Verify coverage truthfulness | Accepted. Static `--print` vs building `--list`; fingerprint-validated reuse recorded as `not_run`; `qualify` refuses reuse; retention. | D4, P4 |
| F06 Unscoped readiness; the import gate as authority | Accepted. Per-prerequisite checks with `--locked`; an overridable `build_identity()` observation; `qualify` no longer self-prepares. | D1, P2, RC07 |
| F07 Overstated evidence | Accepted. Counts attributed to the P5 Codex coordinator; counts given with denominators; the unchecked "44 rebuilds" dropped; qualitative AE-17 trigger; project-layer keys labelled Interface-checked; probe commands and dates recorded. | §1, §4, §9 |
| F08 Minor items | Accepted. The justfile-shell 3.12 constraint is documented; `build_measurements.py:484` added; the snapshot hint labelled heuristic; filter validity derived from the actual package set. | P2, P3, P4, P6 |

The review's third settling question (concurrent artifact builds on the shared build directory) is P5 step 0.

## 9. Finding dispositions

| Finding | Scenario | Disposition | Owner | Evidence / revisit trigger |
|---|---|---|---|---|
| AE-01, AE-02, AE-03, AE-05 | S1, S4, S6 | scheduled | P2 (wording in P7) | `uv_cache.json`; `--dry-run/--check` probe (2026-10-07); W1 sync counts |
| AE-04 | S4, S6 | scheduled | P2 docstring note; P3 callers; P7 | 3.12 parse check (2026-10-07); W1 failures |
| AE-06, AE-07 | S1, S4, S6 | scheduled | P3 | `expect` sites; journal 10-07 16:13; Docker port probe; W1 `LCTX_*` counts |
| AE-08, AE-09, AE-10 | S1, S2, S5 | scheduled | P4 (and P2 for the sync/lock causes) | W1 family wall times; drills D1/D2/D4 |
| AE-11 | S3 | scheduled | P6 | Drill D3 |
| AE-12 | S2, S5 | routed | Testing architecture owner; the catalog tests go with P1 | W2 family/target matrix. Revisit when that owner next revises families. |
| AE-13 | S7 | scheduled | P5 | W1 flock, private-dir and copy counts (P5 Codex coordinator) |
| AE-14, AE-15, AE-16 | S8, S9 | scheduled | P7 | Fresh-session measurements; instruction audit |
| AE-17 | S1 | deferred | Operator | Revisit if wrong-path reads remain a visible share of agents' exploration after P7's navigation note |
| AE-18 | S8 | not proposed | — | Revisit if a skill goes missing in Codex |
| AE-19, AE-20 | all | scheduled | P1 | Config audit |
| Review F01–F08 | — | accepted | §8 | Review §11–12 |

## 10. User- and machine-level notes (not packets)

- **Plaintext token.** `~/.codex/config.toml` stores the Context7 bearer token in plain text.
- **Global instructions.** `~/.codex/AGENTS.md` duplicates the global rules and lacks the python-analyzers exception.
- **Codex memories** (16.8 KB) reference a nonexistent `cpg-schema` crate.
- **Stale Codex entries.** Trust and hooks entries are stale, for example `/tmp/library-context-role-trial-*`.
- **Context7 guidance conflict.** The global rule says "always use Context7", while AGENTS.md puts skills first. D5 showed both runtimes reconciling the two.
- **Shared slice.** pse-arrow's `pse.slice` and `CPUWeight` placement could become a machine-wide agent slice shared by all repositories. That is a cross-repo decision.
- **Swap** is effectively full (6.7–8 of 8 GiB).

## 11. Drill appendix (W6)

All drills were read-only, run once per runtime and scored against targets written before the first drill (`targets.md` 19:07:23; first drill 19:07:43). With n = 1 there is no cross-runtime score, and the drills are not a baseline.

| Drill | Claude (turns, s, $) | Codex (s) | Outcome and friction |
|---|---|---|---|
| D1 focused tests for `codec::batch_bodies` (S1/S4) | 11, 60, 0.70 | 62 | Both correct, including the fixture-free unit loop. Both flagged `verify-store`'s sync and Docker side effects. Claude was unsure of `--test` across two packages. Both prefixed the build-environment eval or wrapper. |
| D2 tests reaching `cpg_core::artifact::admit` (S2) | 21, 112, 0.97 | 130 | Both found 20–21 tests across compiler/producer, compiler/cli, store and serving/mcp. Both hit "filters reach pytest only" and the `--command` rule. No tool selects affected tests. |
| D3 is anything generated stale (S3) | 14, 79, 0.70 | 69 | Both hand-assembled about 12 commands; Codex declared the task blocked. Both flagged that `docs-check` regenerates, that recipe `uv run`s sync, and that snapshots cannot be checked without tests. |
| D4 classify a captured failure (S6) | 8, 39, 0.53 | 45 | Both correct (fixture and Python 3.12). Codex avoided `verify-store` because of its sync. |
| D5 SurrealDB per-statement errors (S8) | 9, 124, 0.55 | 47 | Both correct and version-checked against Cargo.lock. Claude used the selected skill and the registry source. It noted that the skill's top advice under-signals `take_errors`, and its working directory moved into the skill store. Codex used the skill, Context7 (no matching docs) and the registry source. |
| D6 current work and F03 owner (S9) | 8, 33, 0.55 | 36 | Both correct. Claude was distracted by unauthenticated connectors and noticed STATUS lagging two commits. |

## 12. Verification of this plan

- **`just docs-check`: passed (2026-10-07), 335 canonical pages.** An earlier run failed on a pre-existing stale ADR index, which was regenerated outside this work.
- **`just docs`: passed (2026-10-07).**
- **`just lint-agents`: not_run.** No instruction files changed.
- **`just turn-end`: not_run, deliberately.** It formats the whole tree and syncs, and another agent's native work was live.
- **Ruff: not_run on the miner.** The project ruff configuration excludes `docs/`.
