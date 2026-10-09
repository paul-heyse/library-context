# Agent workspace effectiveness

**Status (2026-10-08): implemented, with design-phase acceptance.** P1–P7 are implemented on `main` (§13). OD1 was approved (ADR-0134), and OD2 chose the native fixture binary. Following the operator's design-phase scope, acceptance is each feature's revealing case plus a first-principles value check, not a revalidation of the existing test scope; full functional testing follows the planned design changes. A fresh [implementation review](../design_review/reviews/design_review_agent-workspace-implementation_2026-10-08.md) concluded Revise (scoped), and its findings are fixed (§8).

Both reviews concluded **Revise**: keep the main direction and correct the mechanisms.
- The [first review](../design_review/reviews/design_review_agent-workspace-effectiveness_2026-10-07.md) corrected contracts and lifetimes.
- The [capability review](../design_review/reviews/design_review_agent-workspace-effectiveness-capabilities_2026-10-07.md) re-examined that result. It corrected how capabilities are scoped and composed, and replaced policy where a capability belongs.

Both sets of findings are integrated below (§8).

**Follow-up route, 2026-10-09:** the [Codex-focused plan](agent-effectiveness-followup-plan_2026-10-09.md)
owns new F01–F05 and transferred AE-24. Its operator-confirmed RC01 scope replaces this plan's
historical equal-runtime/repository-only choices for that work. This plan retains its dated
implementation receipts and all other dispositions; scheduling a follow-up does not close them.

**Execution order:**
- P1, D5 and D7 proceed directly.
- D6/P7 use the selective-suppression contract.
- P2–P6 follow the shared execution contracts in §5.8.
- D1's readiness switch and RC09 wait for OD1.

## 1. Context

The operator asked how the environment, command surface, tooling and agent configuration could make highly capable agents more *effective* here. Claude Code and Codex are weighted equally.

"Effective" means:
- less up-front thinking about how to phrase a command;
- fewer complications caused by the environment;
- related actions reachable through one parameterised command;
- legible feedback;
- no tooling that boxes in an agent that already knows what it wants.

Reducing context, increasing wrapper use or enforcing a preferred procedure does not by itself establish effectiveness. Commands should expose useful operations, explain their effects and accept ordinary tool arguments.

Machine-specific configuration is welcome: this is a single-operator project on one workstation. The method adapts pse-arrow's Plan 29 assessment (2026-10-07) and the corrections its review made.

**Operator choices (2026-10-07):**
- Proposals are **repo-level only**. User- and machine-level observations are notes (§10).
- The **library catalog is retired** and is not assessed as a capability. Its leftover wiring is treated as a hazard (AE-19).
- The transcript miner lives in the [evidence folder](../design_review/evidence/2026-10-07_agent-workspace-effectiveness/README.md).

**Ownership.** This plan owns the harness:
- environment preparation;
- fixtures as a command boundary;
- command selection and run handles;
- parallel checkouts;
- freshness and scoped maintenance;
- instructions and runtime configuration.

It does not own:
- test-family *coverage*: the owner of the [testing architecture plan](testing-architecture-pivot-plan_2026-10-04.md);
- product performance: the [persisted graph execution plan](persisted-graph-execution-plan_2026-10-07.md).

The concurrent product work, its tree and STATUS ownership were not touched.

**Method.** All evidence is local. Raw artifacts are in gitignored `build/agent-effectiveness/`. Aggregates and the miner are in the evidence folder. No transcript text entered the repository.

| Workstream | What it examined |
|---|---|
| W1 transcript mining | 88,235 shell commands in 113,321 events, from 259 Claude transcripts and 277 Codex threads (2026-09-22 to the 10-07T22:42Z cutoff), stratified by runtime × main/subagent × role × configuration period. **Current-period sample:** P4 (10-05) has 2 Claude main sessions (111 shell commands) and 10 Claude subagents. P5 (10-06 onward) has **no Claude activity**; for Codex it has **one** main session plus 43 subagents. Current-period counts therefore largely describe one Codex coordinator's workflow, and runtime is confounded with role. W1 is a catalogue of mechanisms: counts carry their strata and denominators, and no general rates are claimed. Cited command classes were spot-checked (evidence README). Historical counts diagnose friction; they do not select a remedy. |
| W2 command surface | `just --dump` (41 recipes); `verify.py`/`native_controls.py` families × boundaries; `cargo metadata` test targets; `pytest --collect-only` (337 tests) |
| W3 instruction and configuration load | Context loaded by a fresh session in each runtime; instruction bytes; duplication and staleness |
| W4 capability research | uv 0.12.22, maturin 1.15, nextest 0.9.146, Docker 29.8.2, Codex 0.161 and Claude Code 2.1.293 (Context7, tagged source, `--help`). pse-arrow's results at the same versions were reused. |
| W5 probes | Read-only `uv sync --dry-run/--check` with and without the native key; Python 3.12 parse check of `scripts/`; fresh-session context measurement; Docker daemon port across `restart`; `build_environment.py --shell` timing (0.02 s) |
| W6 cold-start drills | Six read-only tasks, each run once by `claude -p --permission-mode plan` and once by `codex exec -s read-only`, scored by the plan author against targets written beforehand (§11). Qualitative evidence, not a baseline. |
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
- test parallelism is never lowered by the harness (operator, 2026-10-08): nextest keeps its default of one thread per logical CPU (32 here), and pytest keeps its own;
- performance is not a design gate;
- every new default is overridable;
- each changed capability is exercised once, in its revealing case.

No workflow engine, daemon, capability registry or universal workspace model is justified. Ordinary functions, small records and existing recipes supply these boundaries.

## 2. Scenarios

| ID | Scenario |
|---|---|
| S1 | Edit → compile → focused test in one crate |
| S2 | Select the tests a change affects, across families and the Rust/PyO3/MCP boundaries |
| S3 | Confirm that enumerated generated outputs are fresh |
| S4 | Store, serving or compiler control that needs a disposable SurrealDB and the native extension |
| S5 | Scope-end leaves or `qualify` |
| S6 | Diagnose a failure as environment-shaped or code-shaped |
| S7 | Several agents (Claude, Codex, subagents) building and testing at once |
| S8 | Library API lookup and correct use |
| S9 | Session start or resume |

**Consequential variations:**
- system vs venv `python3`;
- `just`-wrapped vs bare `uv`;
- an inherited absolute `UV_PROJECT_ENVIRONMENT`;
- an edited native-extension input;
- a second concurrent run;
- a worker copy or worktree;
- a launcher that is interrupted, times out or receives SIGKILL;
- a fixture restart;
- enabling an optional runtime capability;
- a dirty tree shared with another agent.

## 3. Overall reading

In the current period, few commands fail outright with environment errors: 23 in P4–P5. Agents have learned the traps. They now pay for friction by **building their own workarounds**:
- hand-assembled environments;
- private fixture drivers;
- invented build locks;
- private build directories with sccache disabled;
- ad hoc checkout copies;
- sleep-polling of long runs.

Much of the current-period evidence for the copies, locks and polling comes from one Codex coordinator's workflow.

The drills show the same thing. Both runtimes reached correct answers to all six tasks. They spent their turns discovering what the wrappers do (sync, Docker, locks, hard-coded packages) and assembling multi-command procedures that the repository does not offer.

The changes with the highest value supply those operations directly, as composable capabilities over existing tools:
- scoped explicit preparation, with managed environment ownership;
- one fixture boundary under bare commands;
- one resolved command plan, with usable run handles;
- optional, correctly prepared checkouts;
- bounded freshness and scoped maintenance;
- current instructions without competing workflow injections.

## 4. Findings

Evidence labels follow design principles §D. "Agent-h" is summed command wall time: it includes the work itself, and subagents overlap. W1 counts are historical workload evidence, not measurements of any remedy. Every finding was re-checked at HEAD `1be0dd96`.

### Environment and the native extension (S1, S4, S6)

**AE-01 The native extension's uv cache key differs between `just` and bare invocations, so the 175 MB extension is rebuilt back and forth.** *Interface-checked* (uv 0.12.22 `cache_info.rs`); *Tested* 2026-10-07 with read-only `uv sync --project … --frozen --dry-run` and `--check --inexact`, key unset and then set.
- **The key.** `python/lctx_semantics/pyproject.toml` declares `{ env = "LCTX_NATIVE_SEMANTICS_INPUTS" }`. uv records the value (null when unset) and rebuilds on any difference.
- **Who sets it.** The justfile shell (`scripts/build_environment.py`) exports the key for every recipe. Bare `uv run` and the MCP launchers do not.
- **Current state.** The installed build records `null`. With the key set, `--check` reports "environment is outdated" and plans a lctx-semantics reinstall.
- **The ping-pong.** The next syncing recipe (`fmt`, `types`, `adr-lint`, `deps`, `gold`, `doctor`) rebuilds the extension, and the next bare syncing `uv run` rebuilds it back.
- **File keys compare ctime**, so source edits in lctx-model, lctx-surrealdb or lctx-serving also trigger rebuilds.
- **History.** W1 counted 813 syncing `uv run` commands before P4 and 0 in P4–P5: agents learned `--no-sync`.
- **Related.**
  - The declared build requirement `maturin>=1.15.0` is unpinned and absent from `uv.lock`, against the exact-pin rule.
  - A stale `lctx-storage` editable install survives inexact syncs.

**AE-02 Verify readiness itself synchronizes the shared environment.** *Interface-checked.*
- `verify.py::prepare` runs `uv sync --locked --inexact --only-group dev` for every family except model and analytics.
- For serving it runs a full `uv sync --locked --inexact`, which can rebuild the extension.
- It does so under `.venv/.verification.lock`, held for the whole run, with a silent wait.
- AGENTS.md:251 says never to synchronize while native guards or workers are live. The families can violate that for other agents' workers.
- In both runtimes, every D1–D4 drill answer flagged the sync as a surprise. Codex routed around `verify-store` because of it.

**AE-03 Nothing reports whether the installed native extension matches its sources.** *Measured* (W1); *Interface-checked*.
- 10 failures (8 in P4–P5) imported a symbol that the installed extension lacked.
- uv does not recompile before import, and the module exposes no build identity.
- A read-only observation exists, but no recipe uses it: `uv sync --locked --check --inexact` with the key, or the build-time key in the installed `uv_cache.json`.
- Freshness on disk also does not identify the extension already loaded by a persistent process.

**AE-04 The interpreter depends on how the agent was launched.** *Measured*; *Tested* 2026-10-07.
- `/usr/bin/python3` is 3.12.3.
- Five scripts fail to parse under 3.12 because they use PEP 758 `except A, B:` syntax: `surrealdb_fixture.py`, `check_family.py`, `build_measurements.py`, `programmatic_eval.py` and `library_catalog_hints.py`.
- Three places invoke `surrealdb_fixture.py` through bare `python3`: its own docstring, `docs/surrealdb.md:179` and `build_measurements.py:484`.
- Five P5 Codex failures hit this in worker copies.
- The justfile shell (`build_environment.py`) runs under the system Python and must keep parsing there.

**AE-05 Agents assemble the environment by hand because the instructions prescribe it.** *Measured* (W1).
- `eval "$(python3 scripts/build_environment.py --shell)"` appears in 1,089 commands (402 in P4–P5). The `build_environment.py --` wrapper appears in 2,442.
- AGENTS.md:141 prescribes the eval. Five of the twelve drill answers began with it.
- For Cargo, its effect is to normalize an inherited foreign `CARGO_TARGET_DIR`/`CARGO_BUILD_TARGET_DIR`. No agent runtime in this assessment had one. For uv, its effect is the AE-01 key.

### Fixtures as a command boundary (S1, S4, S6)

**AE-06 Bare `cargo nextest` cannot run fixture-backed tests, and the one fixture wrapper supplies only half the environment.** *Interface-checked*; *Measured*.
- About 30 `expect()` sites in 19 files need `LCTX_SURREAL_TEST_CONFIG` or `LCTX_COMPILER_RUNTIME_CONFIG`.
- `surrealdb_fixture.py -- CMD` supplies only the first. The second exists only inside `native_controls.py:54-66`.
- Agents rebuilt the environment by hand:
  - 273 commands set `LCTX_*` inline. The most common was `LCTX_SURREAL_TEST_CONFIG` pointed at a long-lived `/tmp` runtime, 82 times in P4–P5.
  - Private `/tmp` driver scripts re-implement the compiler-runtime fixture: 93 runs and 14.7 agent-h in P4–P5.
- Tests also choose some of their own state names. `native_journey.rs:186-189` hard-codes namespace `gn_serving_journeys` and cache database `cache`.
- nextest setup scripts are experimental in 0.9.146.

**AE-07 The fixture's lifecycle leaks, races and caps silently.** *Interface-checked*; *Measured* (journal); *Tested* 2026-10-07 (Docker port across `restart`).
- **Leaks.** SIGTERM or SIGKILL of the launcher skips `finally`, so the detached container and the scratch directory survive. Containers carry no label.
- **Port race.** The port is chosen by binding port 0, closing it, then passing it to `--publish`.
- **Restart moves the port.** A daemon-allocated port changed on `docker restart` (33192 → 33193). The mcp path restarts the server after a Rust test has recorded its port.
- **Silent OOM.** The 1 GB cap is silent. On 2026-10-07 at 16:13 the kernel OOM-killed a uid-1000 `surreal` container at 1.04 GB RSS; an agent sees only a connection failure.
- **Outside agent cgroups.** Containers run in `system.slice`.
- **A native `surreal` 3.3.0 binary is also installed.**

### Selection, cost, runs and feedback (S1, S2, S5)

**AE-08 Families are coarse and slow, so agents bypass them.** *Measured* (W1, n < 30 per family); *Interface-checked*.

Observed p50 wall times:

| Family / boundary | p50 wall time |
|---|---|
| serving | 1,065 s |
| compiler/producer | 1,222 s |
| mcp | 1,649–5,625 s |
| filtered 6-test store run | 317–835 s |

- **Usage in P4–P5.** 21 `just verify-*` calls and 8 direct `native_controls` calls, against 114 bare cargo test runs and 38 run with inline `LCTX_*`.
- **Causes at HEAD:**
  - each run syncs and takes the lock;
  - mcp always builds lctx-eval and runs all of `native_journey` before it applies filters to pytest;
  - the tooling docs boundary runs whole-workspace release doc-tests;
  - there is no nextest thread default suited to concurrent agents.

**AE-09 Selection is fragmented, and agents cannot see what a family will run.** *Interface-checked*; drills D1, D2, D4.
- **Fragmented definitions.** Family names, boundaries and requirements live in `verify.py`; package defaults and MCP setup live in `native_controls.py`.
- **One family per invocation.** The parser accepts one family and one optional boundary, so in D2 each runtime assembled four invocations with separate filter destinations and outcomes.
- **Hard-coded packages.** The store family hard-codes `-p lctx-surrealdb -p lctx-publisher`. Three of six drill answers were unsure whether `--test X` is valid when only one of those packages has X.
- **No preview.** Nothing prints the resolved commands, environment names or prerequisites.

**AE-10 Long runs have no handle, so agents poll and scatter logs.** *Measured* (W1; P4–P5 counts mostly from the P5 Codex coordinator and its subagents).
- **No handle.** `verify.py` runs children synchronously, inherits stdout, and writes no logs or structured result.
- **Polling.**
  - Codex `clock.sleep`: 1,659 calls, 17.3 agent-h (806 calls and 8.0 h in P4–P5).
  - 3,300 `ps`/`pgrep` checks and 1,345 `wait_agent` calls.
- **Ad hoc logs.** 4,170 build/test runs were redirected to ad hoc files (693 in P4–P5), and 1,797 of them were followed by a log read.
- **Timeouts.** Claude's 2–10 min Bash ceiling sits below every family's p50 wall time.

**AE-11 Freshness of generated outputs has no read-only answer.** *Interface-checked*; drill D3.
- Both runtimes assembled about 12 commands each. Codex declared the task blocked.
- The current checks are uneven:
  - `deps` mixes policy with Hakari drift;
  - `docs-check` rebuilds the site;
  - snapshots are checked only by running tests;
  - formatting has no check recipe;
  - `skills-check` sits outside `qualify`;
  - `check_gold.py` prints `not_run` and exits 0 when its skill is missing.
- The `crates/lctx-model/tests/snapshots/ids__*.snap` files are orphaned.

**AE-12 Families leave tests unrun and run tests that cannot pass.** *Interface-checked*; routed (§9).
- **Never run by any family:**
  - 53 of 61 cpg-extract integration targets;
  - cpg-flow `call_paths`;
  - lctx-embed `client`;
  - lctx-eval, lctx-semantics and lctx-model-macros (doc-tests only);
  - 13 MCP pytests.
- **Collected but unable to pass, in the tooling family:**
  - 71 tests for the retired catalog;
  - `test_programmatic_native*.py`, which fail without the serving configuration.

**AE-21 Whole-tree maintenance forces a choice between touching others' work and leaving one's own maintenance incomplete.** *Interface-checked*; observed 2026-10-07.
- `just turn-end` runs whole-tree formatting, `ruff --fix`, Hakari regeneration and the ADR index.
- DESIGN §1 (ADR-0126) prescribes it at the end of every turn that changes files.
- With another agent's dirty production files in the tree, both this assessment and the capability review left it `not_run`. No path-scoped alternative exists.

### Parallel work in one checkout (S7)

**AE-13 One Codex coordinator's workflow invented build serialization and isolation.** *Measured* (W1, mostly the P5 Codex coordinator and its subagents).
- **Hand-made locks.** 164 build/test commands wrapped in hand-made `flock` on a shared `/tmp` lock: 17.8 agent-h, p50 140 s, p90 884 s.
- **Cargo lock waits.** "Blocking waiting for file lock" appeared in 28 of 325 Codex-subagent build/test commands in P4–P5.
- **Private build directories.** 747 commands (154 in P4–P5) pointed Cargo at private build or target directories, most with sccache disabled.
- **Copies of the checkout.**
  - 4,688 P4–P5 commands ran in copies under `~/.cache/lctx-*` or `/tmp/lctx-remediation-*`.
  - Agents ran 24 `git worktree add`, 19 removes and 65 `git apply`.
  - The copies isolated workers from concurrent edits to the main checkout, which is the case worktrees exist for.
- **Absolute environment paths.** 23 commands set an absolute `UV_PROJECT_ENVIRONMENT` pointing at the main `.venv`. uv uses an absolute value as-is (uv 0.12.22 docs), so a copy or worktree can prepare and import main's environment.
- **At HEAD:**
  - AGENTS.md:338 restricts worktrees to concurrent production edits;
  - no recipe prepares another checkout;
  - fine-grain locking is established for check-type commands only;
  - every checkout shares one build directory (ADR-0136);
  - workspace crates build incrementally, so sccache does not cache them.

### Instructions and runtime configuration (S8, S9, all)

**AE-14 Always-loaded context is large, and part of it is competing or stale.**
*Measured* 2026-10-07, in fresh `-p` sessions with `--max-turns 1`:

| Runtime | First-request context |
|---|---|
| Claude, full | 47.2k tokens (36.9k cache creation + 10.4k cache read) |
| Claude with `--setting-sources project,local --strict-mcp-config` | 36.5k tokens |
| Codex | 32.8k input tokens |

- **Superpowers.** Its SessionStart injection appeared in 16 of 35 Claude main sessions. It mandates its own brainstorming and planning workflow, competing with the repository's process skills.
- **Connector instructions.** Several claude.ai connectors (Notion, Claude Docs, Dropbox) inject server instruction blocks that direct workflow. Examples observed in this session: "recommend Notion for work…" and "make the doc FIRST".
- **Unauthenticated connectors.** They distracted the D6 Claude drill.
- **AGENTS.md** is 28,762 B, and its 6.5 KB preamble is a drifting status narrative. Codex counts only project docs against `project_doc_max_bytes` (32,768), which leaves 4 KB of headroom (*Interface-checked*).
- **Codex history.** 659 compactions and 3,361 truncated-output warnings.
- **Labels.** The project-layer configuration keys in P7 stay *Interface-checked* until P7's fresh-session acceptance. Equal capability across runtimes does not require equal context size.

**AE-15 The instructions contradict themselves and route to stale owners.** *Interface-checked.*
- **`just qualify`.** AGENTS.md prescribes it (:118, :254) and forbids it (:242).
- **Stale routing.** AGENTS.md routes to the earlier coordinator's §9 (:239).
- **Testing rule.** It says families "prepare their actual prerequisites".
- **Retired PostgreSQL.** `test-agent.md:8` mentions PostgreSQL.
- **Skills README.** It cites stale analyzer versions, lists an unselected skill and has no rows for three selected skills.
- **docs/README.** Its library route starts at the retired catalog.
- **Duplicated content.** The process-skill list appears in four places, and the principles paragraph in seven files.
- **Dead recipes.** Agents invoked 26 recipe names that no longer exist.

**AE-16 Project memory is partly stale (Claude only, user-level).** *Interface-checked.*
- About 7 of 27 entries describe retired mechanisms.
- The library-catalog entry was corrected on 2026-10-07.
- Memory is outside the repository (see §10).

**AE-17 Agents read paths that don't exist.** *Measured* (W1).
- There were 1,017 probe misses in P4–P5, out of 17,840 shell commands.
- 486 were guessed files in directories that do exist: lctx-model 291, cpg-core 89.
- 72 confused `mod.rs` with `foo.rs`, and 55 pointed at removed crates or docs.
- Semantic navigation is configured at user level: Claude's `LSP` tool through the rust-analyzer plugin, and Codex's rust-analyzer MCP. No transcript shows either one used.

**AE-18 Codex's skills scan reports a truncated inventory at every start.** *Interface-checked*; *Tested*.
- The symlinked library skills hold about 1.26M files, which exceeds the scan caps.
- The scan is breadth-first, so all 27 skills still load.
- Low impact.

**AE-19 The retired library catalog is still wired in, and its wiring syncs.** *Interface-checked.*
- `.codex/config.toml` enables a `library-catalog` MCP server for every Codex thread and subagent via `uv run --frozen`, which still syncs, without the AE-01 key.
- `.mcp.json` declares the same server.
- The recipe, scripts, `tools/lu-resolve`, docs and 71 tests remain.

**AE-20 Stale artifacts.** *Interface-checked.*
- Two committed 0-byte `rustc-ice-2026-10-01*.txt` files.
- A committed root file named `1`.
- The `justfile:144` "just pilot" comment.
- The `Cargo.toml:112,138` delta comments.
- Five Postgres allow rules in `.claude/settings.json`.
- An orphaned `python/lctx_storage/`.
- `docs/pins.md` records nextest 0.9.144; 0.9.146 is installed.
- `build/` holds 2.8 GB of logs and captures. This is an observation, not a deletion target: it is gitignored scratch, and useful captures are kept.

## 5. Decisions for confirmation

Two of these change clauses of the accepted ADR-0126:
- D1/RC07: readiness observes rather than prepares (ADR-0126 :59–60).
- RC09: scoped maintenance (ADR-0126 :69–74; DESIGN §1.2 :97–99).

Both are operator decisions under OD1 (§5.9). No other decision alters a §B decision or an accepted ADR. The review findings that shaped each contract are recorded in §8.

### D1 — Explicit preparation scoped by project and requirement; managed environment ownership; readiness observes

**Preparation routes.** Each route prepares only the environment and capability requested:

| Route | What it does |
|---|---|
| `just sync tools` | `uv sync --locked --inexact --only-group dev`; does not build the extension |
| `just sync native` | Full `--locked --inexact` sync, with the native key set; establishes matching extension inputs once and is reused while unchanged |
| `just sync vllm` | `uv sync --project services/vllm --locked`; the independently locked service keeps its own route |
| `just ready` | Composes skill links, then `sync native` once, then the doctor check, with no repeated synchronization |

Every blocked message names the route that repairs it.

**No implicit sync.**
- Every launcher the repository defines carries an explicit `--no-sync` wherever a preparation route exists: recipes, scripts, nested launches and any future MCP registration.
- `UV_NO_SYNC=1` is a default for agents' ad hoc `uv run`:
  - Claude: project `env`, which reaches every child, including MCP servers.
  - Codex: `shell_environment_policy.set`, which reaches shell commands only.
  - IDE terminals and plain shells keep today's behaviour.
  - Override with `UV_NO_SYNC=0` or `false`.
- The native fingerprint is computed only by `sync native` and by native readiness, not by the justfile shell for every recipe.

**Managed ownership.**
- **Shared.** Managed commands hold shared ownership while they use the shared Python environment: verify boundaries whose requirements include tools or native-python, and fixture commands that import it.
- **No borrowing.** Pure Rust commands do not borrow the environment, even under a fixture.
- **Exclusive.** Managed synchronization takes exclusive ownership. While shared holders are live, it reports them (pid, command, start time) and waits.
- **One acquisition.** Ownership is acquired once per command tree, and nested managed operations reuse it.
- **Lock identity.** Ownership covers two resources:
  - the effective environment;
  - the checkout's extension directory, because maturin builds the extension into the source tree and `.venv` holds only a `.pth`.

  The locks are machine-wide and keyed by absolute path, so they work before the environment exists (§5.8).
- **Outside the guarantee.** Unmanaged commands (bare importers, an explicit `uv sync`) remain available. An intentional override names the guarantee it bypasses.

**Why wait rather than refuse.** P2's step 0 is settled. maturin 1.15.0 unlinks the old editable extension (ignoring unlink errors), then copies the new one. Live mapped importers keep the old inode, but a new import during the copy is not atomic. So `just sync` waits for managed holders; it does not refuse.

**Readiness observes, per prerequisite.**
- A boundary checks only what it imports: `uv sync --locked --check --inexact` with the key, for native-python boundaries.
- A failed check reports `blocked: run just sync native` (or `tools`).
- `qualify` does not self-prepare.
- An optional `build_identity()` (an embedded inputs hash) would diagnose the extension already loaded by a persistent process. It is **deferred**: it needs a Rust change to the extension, so a later product change carries it.

**Environment launcher.** The existing `build_environment.py -- <cmd>` remains the thin optional launcher for any command. It gains `--explain`, which reports the effective target and build directories, interpreter, venv and uv settings. Bare tools stay available, and the documentation describes honestly what they inherit.

**Dependency.** The declared build requirement becomes `maturin==1.15.0` (an exact pin; ordinary dependency maintenance).

### D2 — One fixture boundary with defined lifetimes and owned state

`just fixture … -- <cmd>` runs any command with the native fixture variables supplied: `LCTX_SURREAL_TEST_CONFIG`, `LCTX_COMPILER_RUNTIME_CONFIG` and, optionally, retained serving content. It is the single owner of fixture and runtime setup, and families delegate to it.

**Three things with different lifetimes:**
- **Server substrate.**
  - Run-owned by default.
  - `--keep` retains it and returns an ID.
  - `--stop ID` removes it.
  - `--list` inventories fixtures: ID, kind, owner, attachments, port, memory and age. Inspection never sweeps.
- **Attachment state.** `--attach ID -- <cmd>` gives each attached command its own scratch directory and configuration, and identified mutable databases or namespaces. Commands that choose their own names remain responsible for that state (`native_journey.rs` does). Concurrent attachment does not promise universal isolation.
- **Retained serving content.**
  - It records its producing inputs, configuration, immutable content identity and producing run.
  - New attachments preserve it.
  - Reuse checks that the expected realization is still available; a source fingerprint alone is not enough.
  - Reuse records the obligations it skipped.

**Orphans.**
- An orphan is a run whose owner process identity (pid plus start time and boot ID, robust to pid reuse and reboot) is gone, and whose owned command tree has no survivors.
- A SIGKILLed launcher can leave its command alive. The sweep preserves live consumers, or explicitly terminates its owned surviving tree before removing the server.
- Automatic sweeping at fixture start is advertised and overridable (`--no-sweep`).

**Signals.** On SIGTERM or SIGINT the fixture kills the child's process group. It then inspects the container: an `OOMKilled` state is reported as an infrastructure failure. Then it removes the container.

**Port.** The launcher owns a stable explicit port and retries on collision. A daemon-allocated port moves on restart (tested). No `--rm`.

**Memory.** `LCTX_FIXTURE_MEMORY` overrides the 16 GiB default and scales `SURREAL_MEMORY_THRESHOLD` (8 GiB by default). The effective value is printed.

**Substrate choice is P3's step 0.** The choice is between Docker (the exact image digest; lifecycle as above) and a native `surreal` 3.3.0 child process pinned by binary sha256. The native child dies with its process group, keeps its port and is accounted in the agent's cgroup, so run-owned orphans and port moves disappear by construction. Decide on pinned provenance, lifecycle and total integration burden. The current image choice is not a prohibition.

**Invocation.** Docs and docstrings use `uv run --no-sync python` (AE-04). Scripts keep their 3.14 syntax.

### D3 — Optional, correctly prepared checkouts; no build lock

**When to use a worktree.** Concurrent commands may share a stable checkout when their effects do not conflict. Worktrees support independent revisions or conflicting mutable state, for example another agent's concurrent edits, which is what W1's copies were isolating.

**Preparation.** `just ready` works in any checkout, including runtime-created worktrees:
- It selects that checkout's environment explicitly. It sets or overrides `UV_PROJECT_ENVIRONMENT` to the checkout's own `.venv`, and reports any inherited absolute value it replaced. Checking `VIRTUAL_ENV` alone is not enough.
- It coordinates the environment it selected.
- It prints the interpreter, the extension's import origin and the effective build directory.

**Optional helpers.**
- `just worktree <name> [--ref R] [--carry] [--build-dir shared|own]` creates a worktree.
- `just worktree-remove <name>` removes it.
- `--carry` is an explicit copy of the selected tracked, staged, unstaged, untracked and binary content. It reports how staging was treated and any conflicts, preserves the source checkout, and excludes environment and credential material.
- Removal reports dirty or unintegrated work instead of discarding it.

**Build directories.**
- ADR-0136's shared build directory stays the default. `--build-dir own` selects a per-checkout directory, and the effective path is always shown.
- No build-concurrency probe is run. Every crate depends on lctx-model, so builds are never disjoint. A cold per-checkout build is a performance question, and performance is out of scope.
- sccache caches dependency units but not incremental workspace crates, so neither choice promises warm workspace or link reuse.

**Getting work back.** Work returns to main by branch merge or cherry-pick, or by an explicit patch (RC06).

**No repository build lock** (§7.1).

### D4 — One resolved command plan; usable run handles; truthful results

**One resolved command plan.**
- `just verify --select family[:boundary] …` (repeatable) resolves the invocation into a single plan: selected boundaries, packages/targets from each family's actual package set, prerequisites, effective defaults and tool-scoped arguments (`--nextest-args …`, `--pytest-args …`). Each filter reaches its owner.
- Families remain named shortcuts (`just verify-<family>`). Arbitrary underlying arguments stay available.
- Help and boundary descriptions derive from the executable definitions. There is no second catalogue.

**Inspection.** Two modes consume the plan:
- **`--print` is static.** It shows commands, environment *names*, prerequisites and readiness observations. It builds nothing, creates no fixtures and does not fingerprint native sources.
- **`--list` builds.** It uses each tool's own discovery: `cargo nextest list` with machine-readable output, existing-build reuse with its freshness responsibility stated, and `pytest --collect-only`. Invalid targets name the actual package set.

**Run handles.** These are generic over any command; verify is one consumer.
- Foreground execution is preserved.
- Each runtime's own background, wait and cancellation features come first (Claude background tasks with Monitor/TaskStop; Codex background exec). The repository adds only what crosses commands and sessions: `just run [--background] -- <cmd>` writes a run record, and `just runs [list | status | logs | cancel] <id>` reads it.
- The record holds phase and current command, waiting reason, process identity, the log paths and the terminal result.
- Cancellation affects the owned command tree and its run-owned fixtures.
- A killed launcher leaves a discoverable `interrupted` run, never a permanent "running".
- `--rerun <id>` reuses the original selection for the failed or unexecuted boundaries without silently replaying successful work.

**Results.**
- Boundary outcomes are `passed`, `failed`, `blocked` or `not_run`. Interruption is a separate termination reason, and child exit/signal information is preserved.
- An unexplained nonzero exit is not automatically an infrastructure failure. That classification needs evidence, such as `OOMKilled` or a failed readiness observation.
- `summary.json` is written per run in a unique directory. It may link nextest JUnit written to a unique per-run destination. Boundary outcomes come from the runner, never from missing JUnit entries.
- Output is concise by default; `--live` streams.
- Pruning selects completed runs and preserves active or explicitly retained runs.

**Fixture reuse.** Retained serving reuse follows D2's content identity. A skipped journey is recorded as `not_run` with the reused content's identity. `qualify` refuses reuse.

**Locks.** Lock waits are announced. Test parallelism is **not** constrained (operator, 2026-10-08). A halving default was briefly added during execution and has been removed.

### D5 — AGENTS.md carries durable rules; STATUS carries status

- The volatile preamble narrative moves to STATUS and plan owners, coordinated with STATUS's current writer.
- The AE-15 contradictions and stale routes are fixed.
- `project_doc_max_bytes = 65536` in project `.codex/config.toml` gives headroom.
- Task routes, consequential constraints and deliberate overlap for isolated workers stay. Compactness is a target, not proof of effectiveness.

### D6 — Remove identified competing workflow injections; preserve capability and discoverability

**What is removed:** only specifically identified competing workflow injections, each named, with its version-qualified restoration route:
- **The superpowers plugin, id `superpowers@synced`.** Its SessionStart injection mandates a competing workflow. It is disabled through project `enabledPlugins` false.
  - *Tested* 2026-10-07 with `claude -p --settings '{"enabledPlugins":{"superpowers@synced":false}}'`: the transcript had 3 superpowers mentions by default and 0 with the plugin disabled. First-request context went from 47.2k to 46.9k tokens, so the cost was the competing mandate rather than size.
  - Restoration: set `enabledPlugins` true in `.claude/settings.local.json`, which takes precedence over project settings (Claude Code 2.1.293).
- **The claude.ai connectors whose server instructions direct workflow:** Notion, Claude Docs and Dropbox. They are disabled through project `deniedMcpServers` entries naming each server. Restoration: remove the entry from project settings. Denylists merge across scopes, so there is **no per-session undo**.

**What is not used:** `disableClaudeAiConnectors`. It is true if any settings source sets it true, so a higher-priority setting cannot undo it, and it would also remove useful connectors.

**What is kept:**
- Other plugins and connectors.
- Deferred tool discovery and useful skill descriptions. `skillListingMaxDescChars` is only a cap on oversized descriptions; it is not a means of suppression.
- Unavailable or unauthenticated capabilities are explained rather than hidden.

### D7 — Retire the catalog's agent wiring and code

- Remove the Codex MCP server, `.mcp.json`, the `settings.local.json` entry and the recipe.
- The tooling family stops collecting the catalog's tests.
- The catalog's code, tests and docs are deleted, because the functionality is retired. That covers `scripts/library_{catalog_db,catalog_hints,catalog_mcp,names,scan,semantic,unitgraph,utilization}.py` and their eight tests, `tools/lu-resolve`, `docs/library-utilization.{md,jsonl}` and the evidence folder `2026-09-29_library-utilization-semantic-stage`.
- `scripts/library_skills.py` is **not** catalog code: it serves `skills-sync`/`skills-check`, so it stays.
- fastmcp also stays, because other code uses it.

### 5.8 Execution contracts

These shared contracts bind P2–P6 and were settled before any work was delegated.

1. **Harness launcher.**
   - Harness scripts (`verify`, `runs`, `workspace_env`, the fixture) use only the standard library. They run as `uv run --no-project --offline --no-python-downloads python scripts/<x>.py`.
   - `--no-project` uses an active or parent-directory venv when one exists (*Tested* 2026-10-07: it resolved to `.venv/bin/python` 3.14.7). Scripts therefore derive the target environment from the checkout root and `UV_PROJECT_ENVIRONMENT` alone, never from `sys.prefix` or `VIRTUAL_ENV`.
   - Worktrees are created outside the checkout.
   - The justfile wrapper drops `UV_NO_SYNC`, because combining it with `--no-project` warns (*Tested* 2026-10-07).
2. **`build_environment.py`** is the justfile shell and stays importable by the system Python 3.12. It never imports the new helpers or `surrealdb_fixture`.
3. **`scripts/harness.py`** (coordinator-owned) provides:
   - `ProcessIdentity` and `alive()`. The identity is pid, start ticks, boot id and pid-namespace inode; a foreign namespace is never swept.
   - Process-group spawn, and group signalling with one grace period. A process survives if it is still in the recorded group.
   - **Liveness across pid namespaces** (amended 2026-10-07, from the run-handles work). Codex runs each sandboxed tool call in its own pid namespace, so `ProcessIdentity` alone cannot see a live owner from a sibling call. An owner therefore also holds an `flock` on its record, and liveness is judged as "identity alive **or** lock held". `runs.py` implements this. Fixture records follow the same rule.
   - The outcome vocabulary `passed | failed | blocked | not_run`, and the termination vocabulary `completed | interrupted | cancelled`.
   - Atomic JSON writes.
4. **Ownership.**
   - Locks live in `$XDG_RUNTIME_DIR/library-context/locks/`, falling back to `~/.cache/library-context/locks/`.
   - There are two resources: the environment, keyed by the absolute path of the selected environment, and the checkout's extension directory. Managed native operations take both, in that order.
   - Locks are `fcntl.flock` on a non-inheritable file descriptor, held by the managing process for the child's lifetime.
   - A child reuses its parent's ownership through `LCTX_ENV_OWNERSHIP`, but only while the recorded owner is alive.
   - Holder records feed the "waiting for …" reports.
   - Pure-Rust work never takes a lock.
5. **Records.**
   - Only `runs.py` writes `build/runs/<UTC>-<rand>/record.json`, and only verify writes `summary.json` into `$LCTX_RUN_DIR`.
   - A child reuses `LCTX_RUN_DIR` only while its owner is alive.
   - Fixture records are kept in `build/fixtures/<id>/`.
   - Fixture servers are labelled with their checkout and owner, and a sweep touches only matching labels.
   - Classifying a failure as infrastructure requires evidence: OOM, a failed readiness observation, or launch error 127.
6. **Recipe surface.**
   - **New:** `sync <tools|native|vllm>`, `env [--explain] [-- cmd]`, `fixture …`, `verify …`, `run`, `runs`, `fresh`, `fmt [paths]`, `turn-end [--paths …]`, `worktree`, `worktree-remove`.
   - **Kept:** `ready`, `qualify`, `turn-end`, `deps`, the `verify-<family> [--command B] [-- args]` shortcuts, and every recipe name `lint-agents` checks.
   - The `verify-*` shortcuts keep working at every commit, because the paused product work's planned controls consume them.

### 5.9 Operator decisions

| ID | Decision | Status |
|---|---|---|
| **OD1** | Readiness observes, and preparation runs through the scoped `just sync` routes (D1/RC07). Scoped maintenance is allowed when whole-tree `turn-end` would rewrite unrelated dirty work (RC09). Both change ADR-0126, so they wait for the operator. **Recommended route:** a narrow **ADR-0134** (`design: [§1.2]`) created with `just adr new`, plus a dated pointer under ADR-0126's `## Amendments` naming the two superseded clauses. ADR-0126's other decisions (trusted acknowledged inputs, verification families, assembled qualification) stay in force. Full supersession of 0126, which would re-point §6.1, §6.2, §8 and §15, waits until the paused product files are committed: `adr-lint` would otherwise force an edit to the dirty `semantic-model.md`. | **Approved 2026-10-07.** [ADR-0134](../adr/0134-scoped-preparation-and-maintenance.md) is accepted, and ADR-0126 carries the dated pointer. |
| OD2 | P3's server substrate: Docker pinned by image digest, or a native `surreal` 3.3.0 child process pinned by sha256 | **Decided 2026-10-07: native binary.** In P3's spike, `/surreal` in the pinned image was byte-identical to the official v3.3.0 release binary (sha256 `58ad479c…b7bc5`). A run-owned server dies with its launcher (`PR_SET_PDEATHSIG`); the port is stable across restart; OOM is reported directly (exit status and `Result=oom-kill`); kept servers are `lctx-fixture-<id>` user units. The cost is a sha256 check before each start and a dependency on user systemd. |

**Proposed ADR-0134 text** (created on approval):
- **Context:** AE-01/02/05/21 and capability-review F03, F04 and F08.
- **Decision:**
  1. Verification readiness *observes* each boundary's prerequisites and reports `blocked` with the named repair route. It never synchronizes.
  2. Preparation is explicit and scoped by project and requirement: `just sync tools|native|vllm`, composed by `just ready`. Managed synchronization takes exclusive environment ownership.
  3. The root agent's end-of-turn maintenance may be scoped to the paths a turn changed (`just turn-end --paths …`) when whole-tree maintenance would rewrite another agent's uncommitted work. Whole-tree `turn-end` stays the default. When scoping skips something, the turn's report says which step was not performed.
- **Consequences:** ADR-0126's other decisions are unchanged. DESIGN §1.2 :97–99, AGENTS.md :249–251 and AGENTS.md :261–264 are amended in the same commit.

## 6. Rule changes

These take effect only when confirmed.

| ID | Current rule / location | Change | Packet |
|---|---|---|---|
| RC01 | AGENTS.md:141 prescribes `eval build_environment` before bare Cargo/uv | State what bare tools inherit. Bare `cargo` uses `.cargo/config.toml` directly, and needs normalization only when a target-directory variable is inherited; `build_environment.py --explain` shows that, and `-- <cmd>` normalizes. Repository launchers carry `--no-sync`. Ad hoc `uv run` is non-syncing by runtime default, and each runtime's reach is stated. | P2, P7 |
| RC02 | AGENTS.md Commands/Testing: families are *the* route for contract controls | Families are named shortcuts over one resolved command plan. `just fixture -- <bare command>` is first-class. | P3, P4, P7 |
| RC03 | AGENTS.md:338 limits worktrees to concurrent production edits | Concurrent commands may share a stable checkout when their effects do not conflict. Worktrees serve independent revisions or conflicting mutable state. `just ready` prepares any checkout. Fully merged worktrees are still cleaned up. | P5, P7 |
| RC04 | AGENTS.md:251 "never synchronize while native guards/workers are live" | Keep it, and name the mechanism. Managed commands hold shared ownership; managed `just sync` takes exclusive ownership and reports holders. Unmanaged commands sit outside the guarantee. | P2, P7 |
| RC05 | AGENTS.md preamble holds current status | Status lives in STATUS and plans; AGENTS.md holds durable routes and rules | P7 |
| RC06 | AGENTS.md Git: work and commit on `main` in the current tree | Add the integration route for worktree work: branch merge, cherry-pick or explicit patch | P5, P7 |
| RC07 | AGENTS.md Testing: families "prepare their actual prerequisites"; `verify.py` docstring | Families observe their prerequisites and report `blocked` with a route. Preparation is scoped (`just sync tools\|native\|vllm`, `just ready`). `qualify` does not self-prepare. | P2, P7 |
| RC08 | Runtime settings for plugins/connectors (D6) | Selective, named suppression of identified competing injections, each with its restoration route. No blanket connector switch. | P7 |
| RC09 | DESIGN §1 / ADR-0126: the root runs whole-tree `just turn-end` at the end of a turn that changed files | Allow scoped maintenance (formatting/fixes for selected paths; only the generators whose inputs changed) when whole-tree maintenance would rewrite unrelated dirty work. Whole-tree operation stays available, and an unperformed step is documented. **Requires an ADR on adoption** (`just adr new`, amending DESIGN §1). | P6, P7 |

## 7. Packets

**Sequencing follows dependencies, not a fixed order:**
- **P1** is independent.
- **P4** can proceed without P2 for its command plan, `--print`, run handles and logs. Its readiness reporting follows P2.
- **P3**'s pure fixture lifecycle work does not depend on Python synchronization. Its managed ownership for native boundaries follows P2. Retained serving reuse depends on P3's content identity.
- **P5** depends on P2, because environment selection lives in `ready`.
- **P6**'s freshness check is independent. Its scoped maintenance needs RC09's ADR.
- **P7**'s wording follows the behaviour it describes.

**Rules for every packet:**
- Acceptance exercises each changed capability once, in its revealing case, and deletes what it replaces in the same change.
- No packet adds a lint.
- Every new default is overridable, and its effective value is visible.
- The acceptance examples are planned demonstrations, not executed tests.

| Packet | Responsibility / dependencies | Acceptance (revealing case) | Replaces / deletes | pse-arrow precedent |
|---|---|---|---|---|
| P1 Precise stale cleanup and catalog retirement | <ul><li>Delete exactly: the two `rustc-ice-*` files, root `1`, the `justfile:144` comment, the `Cargo.toml` delta comments, the orphaned `ids__*.snap`, `python/lctx_storage/` and the Postgres allow rules.</li><li>Fix `test-agent.md`, the skills README, the `docs/README` library route and the `docs/pins.md` nextest entry.</li><li>D7: catalog wiring, plus its scripts, tests and docs if confirmed.</li><li>Logs in `build/` are not touched.</li></ul> | <ul><li>`just lint-agents` passes.</li><li>A fresh Codex session starts no catalog server.</li><li>The tooling family collects no catalog tests.</li><li>Each deletion is listed.</li></ul> | The listed artifacts and the catalog wiring | P1 (adopt) |
| P2 Scoped preparation and managed ownership | D1, RC01/RC04/RC07.<ul><li>`just sync tools\|native\|vllm` and the `ready` composition.</li><li>Explicit `--no-sync` launchers.</li><li>`UV_NO_SYNC=1` in both project runtime configs, dropped for recipes.</li><li>Native fingerprint only in `sync native` and readiness.</li><li>Requirement-scoped shared/exclusive ownership keyed to the effective environment, with nested reuse and holder reporting.</li><li>Per-prerequisite `--locked --check` readiness.</li><li>`build_environment.py --explain`.</li><li>`maturin==1.15.0`.</li><li>Optional `build_identity()`.</li><li>A docstring note that `build_environment.py` must parse under the system Python.</li></ul>Step 0 is settled (maturin unlink, then copy). | <ul><li>`just sync tools` completes without building the extension; an unchanged `sync native` is a no-op.</li><li>A missing vLLM environment names `just sync vllm`.</li><li>In fresh Claude and Codex sessions, `just fmt`, then a bare `uv run python -c 'import lctx_semantics'`, causes no rebuild.</li><li>A pure-Rust fixture run holds no environment lock; a managed native run does.</li><li>Two managed readers overlap; `just sync` reports and waits for them; nested fixture execution does not deadlock.</li><li>After a touched lctx-surrealdb source, serving reports `blocked: run just sync native` while store is not blocked.</li><li>`--explain` shows effective paths without fingerprinting.</li></ul> | Incidental sync in recipes, `prepare()` and MCP launchers; per-recipe fingerprinting | P2 boundary (adapt) |
| P3 Fixture boundary | D2.<ul><li>Step 0: choose Docker or the sha256-pinned native child, on provenance, lifecycle and integration burden.</li><li>Then: the `just fixture [--keep \| --attach ID \| --stop ID \| --list \| --sweep \| --no-sweep] -- <cmd>` owner, covering the substrate, attachment and serving-content distinction, owner identity (pid + start time + boot ID), the surviving-tree-aware sweep, process-group signals with OOM inspection, a stable port and the memory override.</li><li>`native_controls.py` delegates to it.</li><li>The bare-`python3` callers are fixed.</li></ul> | <ul><li>Bare `just fixture -- cargo nextest run -p cpg-core --test compiler_artifacts -E 'test(/^graph_artifact::/)'` passes without hand-set `LCTX_*`.</li><li>Two attachments have distinct mutable state while sharing retained immutable content; stopping one preserves the other.</li><li>A SIGKILLed launcher whose child survives is not swept as idle.</li><li>Missing or altered retained content is reported despite a matching source fingerprint.</li><li>The mcp path survives `restart()`.</li><li>A forced small cap reports OOM as an infrastructure failure.</li><li>`--list` never sweeps.</li></ul> | Hand-built `LCTX_*`, private driver scripts, the bind-0 picker | — (pse-arrow review F03 run ownership applied) |
| P4 Command plan, run handles and results | D4, RC02.<ul><li>One resolved plan with repeatable `--select` and tool-scoped arguments.</li><li>Static `--print` and building `--list`.</li><li>Generic `just run`/`just runs`.</li><li>The interrupted state, `--rerun`, `summary.json` and unique JUnit destinations.</li><li>Pruning that preserves active runs.</li><li>Retained-content reuse recorded as `not_run`.</li></ul> | <ul><li>`just verify --select compiler:producer --select serving:mcp --nextest-args "-E 'test(admit)'" --pytest-args "-k native_session" --print` shows each filter reaching its owner, without building.</li><li>A failed boundary preserves the other's outcome.</li><li>A background run is observed from a later tool call, cancelled, and its terminal result retrieved while another run survives.</li><li>A killed launcher leaves an `interrupted` run.</li><li>`--rerun` repeats only the failed boundaries.</li><li>Adding a boundary changes only its definition.</li></ul> | Inline streaming; the silent lock; one-family invocations | P7 logs/feedback (adapt; adds composition and handles) |
| P5 Optional checkouts | D3, RC03/RC06. Step 0: a concurrent-build probe in a temporary worktree at a quiet time, comparing two release builds of disjoint crates on the shared and on per-checkout build directories, to inform the default. Then:<ul><li>environment selection in `ready`;</li><li>optional `just worktree`/`worktree-remove` with `--carry` and `--build-dir`;</li><li>`.worktreeinclude` if gitignored inputs are needed.</li></ul> | <ul><li>Disjoint checks share one stable checkout.</li><li>A worktree prepared from a main-started session that carries main's absolute `UV_PROJECT_ENVIRONMENT` gets its own environment: import origin and lock follow the selected checkout.</li><li>`--carry` preserves mixed content and the source checkout.</li><li>Removal reports unintegrated work.</li><li>Work returns by cherry-pick.</li></ul> | Ad hoc `~/.cache`/`/tmp` copies; main-pointed `UV_PROJECT_ENVIRONMENT`; private build dirs without sccache | P3 worktree (adopt, narrowed) |
| P6 Freshness and scoped maintenance | AE-11/AE-21, RC09.<ul><li>`just fresh [selection]`, read-only, over enumerated outputs: Hakari, the ADR index, skill links, formatting, gold and the snapshot hint.</li><li>Each output reports `clean`, `stale`, `heuristic` or `not_run` (e.g. gold without its skill), with its regenerate command; nothing is invoked silently.</li><li>Outputs that need a test run (insta) are named.</li><li>Scoped maintenance: `just fmt [paths…]` and a path- or input-scoped turn-end, after RC09's ADR. Whole-tree operation stays available.</li></ul> | <ul><li>`fresh` distinguishes clean, stale, heuristic and not_run.</li><li>A touched Hakari input is named with its regenerate command.</li><li>Scoped maintenance leaves unrelated dirty files byte-identical.</li></ul> | Hand-assembled lists; the choice between touching others' work and skipping maintenance | P6 codegen-check (adapt) |
| P7 Instructions, runtime configuration and navigation | D5, D6, RC01–RC09 wording.<ul><li>Move the status narrative; fix AE-15.</li><li>One owner each for the process-skill list and the principles paragraph, with deliberate overlap kept where isolated workers need it.</li><li>Project `.claude/settings.json`: `env.UV_NO_SYNC`; `enabledPlugins` false for superpowers; `deniedMcpServers` for the named connectors (`skillListingMaxDescChars` is deferred: no evidence of oversized entries).</li><li>Project `.codex/config.toml`: `project_doc_max_bytes` and `shell_environment_policy.set.UV_NO_SYNC`.</li><li>A short navigation route in existing docs: definition, references, hover, document symbols and, where exposed, workspace symbols, through Claude's `LSP` tool and Codex's rust-analyzer MCP, with workspace binding stated; fallbacks `rg` and `ast-grep`.</li></ul> | <ul><li>A fresh session discovers and uses a relevant capability whose full description was not initially loaded.</li><li>Superpowers' injection and the named connector instructions are absent.</li><li>Each documented restoration route works at the installed version.</li><li>A failed connector does not hide working tools.</li><li>Navigation locates a moved module through the tools or the fallback.</li><li>`just lint-agents` passes.</li><li>The project-layer keys move from Interface-checked to Tested.</li><li>Token counts are recorded but are not acceptance.</li></ul> | Duplicated paragraphs; the status narrative in AGENTS.md | P8 (adopt, made selective) |

### 7.1 Not proposed

| Idea | Reason |
|---|---|
| A repository build lock or `just serial` wrapper | fd-inheriting locks have a deadlock history (pse-arrow). Cargo serializes its own units, and checkouts (P5) cover conflicting state. |
| Worktrees as the ordinary route for all parallel work | It adds preparation and compilation when commands share a stable revision without conflicting effects (capability review F06). |
| `disableClaudeAiConnectors` | True if any settings source sets it, so it cannot be undone by a higher-priority false. It removes useful connectors. D6 is selective instead. |
| nextest setup scripts to inject the fixture | Experimental in 0.9.146. P3's thin wrapper works with any command. |
| Rewriting scripts to parse under Python 3.12 | It fights ruff's 3.14 target. Routing through `uv run --no-sync` is simpler. `build_environment.py` is the one exception and is documented as such. |
| An agent systemd slice or per-command scopes | No library-context agent process was OOM-killed in the journal; the only local kill was the fixture's own cap (P3). `pse.slice` is shared and user-level (§10). |
| A replacement skill scanner or a shallow `.agents/skills` copy | All 27 skills load; the cost is one log line. |
| A non-bypassable import gate | It would be new authority. `build_identity()` stays an overridable observation. |
| A new job scheduler or always-running service | Runtime background facilities plus a small run record cover the need (D4). |
| `--message-format libtest-json` | Experimental. `summary.json`, optionally linking unique per-run JUnit, suffices. |
| Normalizing the `mod.rs` layout | It would need a lint to keep it normalized. |
| A universal "affected tests" predictor | Both drill runtimes found the 20–21 affected tests in about 2 minutes. D4's composable selection and `--list` supply the operations, and coverage completeness stays with the testing owner. |

## 8. Review integration

### First review

The [first review](../design_review/reviews/design_review_agent-workspace-effectiveness_2026-10-07.md) concluded Revise. G5 (consistency and recovery) and G7 (truthful capability claims) failed.

| Finding | Disposition | Where |
|---|---|---|
| F01 Fixture run ownership | Accepted. Lifetimes, sweep, signals, stable port, no `--rm`. Port behaviour settled by probe (2026-10-07). Refined further by capability review F05. | D2, P3 |
| F02 Lock meaning; uncoordinated `just sync` | Accepted. Shared and exclusive ownership. Scope refined by capability review F04; maturin question settled. | D1, P2 |
| F03 Asymmetric `UV_NO_SYNC` reach | Accepted. Explicit launchers; reach stated. Preparation scope refined by capability review F03. | D1, P2 |
| F04 Worktree claims | Accepted. Claim narrowed; P5 step 0. Policy replaced by capability review F06. | D3, P5 |
| F05 Verify coverage truthfulness | Accepted. Preview/discovery split; skipped journey recorded as `not_run`. Composition and lifecycle added by capability review F01/F02/F05. | D4, P4 |
| F06 Scoped readiness; import gate | Accepted. Per-prerequisite observation; overridable identity. | D1, P2 |
| F07 Overstated evidence | Accepted. Attribution, denominators and labels. | §1, §4 |
| F08 Minor items | Accepted. Interpreter note, `build_measurements.py`, heuristic label, actual package set. Maintenance scope added by capability review F08. | P2, P3, P4, P6 |

### Capability review

The [capability review](../design_review/reviews/design_review_agent-workspace-effectiveness-capabilities_2026-10-07.md) reassessed the plan and the first review's integration and concluded Revise. G5 and G7 failed. Its findings are integrated as below; its rule impacts RC01–RC06 map to this plan's RC01, RC03, RC04/RC07, RC08, §10 (memory) and RC09.

| Finding | Disposition | Where |
|---|---|---|
| F01 Parameterized composition | Accepted. One resolved command plan, repeatable `--select`, tool-scoped arguments, help derived from definitions. | D4, P4 |
| F02 Usable long-run handle | Accepted. Runtime facilities first; a generic run record with status, logs, cancel, interrupted state and rerun. | D4, P4 |
| F03 Scoped preparation; overstated bare reliability | Accepted. Tools, native and vLLM routes; `ready` without a double sync; fingerprint only where needed; honest bare-tool inheritance; exact maturin pin. | D1, P2, RC01 |
| F04 Managed ownership | Accepted. Requirement-scoped holders, effective-environment identity, nested reuse, unmanaged commands stated. maturin's unlink-then-copy settles P2 step 0 as wait, not refuse. | D1, P2, RC04 |
| F05 Substrate, attachment state and serving content | Accepted. The three are distinguished; no universal-isolation promise; content identity for reuse; inventory; robust orphan identity with surviving-tree handling. In addition, the native child option was promoted to P3 step 0. | D2, P3 |
| F06 Worktrees as policy | Accepted. Optional for independent revisions or conflicting state; explicit environment selection (absolute `UV_PROJECT_ENVIRONMENT`); carry and removal contracts; no promise of warm reuse. | D3, P5, RC03 |
| F07 Blanket suppression and restoration | Accepted. The connector restoration claim was wrong. The blanket switch is dropped, and the named competing injections are suppressed with accurate restoration routes. Connector instruction blocks that direct workflow count as identified injections under the review's own rule. | D6, P7, RC08 |
| F08 Freshness, maintenance, navigation, cleanup | Accepted. Freshness outcome classes; scoped maintenance (AE-21, RC09, ADR); actionable navigation; precise cleanup without log deletion; memory moved to notes. | P1, P6, P7, §10 |

### Implementation review

The [implementation review](../design_review/reviews/design_review_agent-workspace-implementation_2026-10-08.md) of `ec278be4..95297fe5` concluded Revise (scoped). Every finding is fixed:

| Finding | Fix | Commit |
|---|---|---|
| F01 Run-owned fixture scopes never stopped (restart failed, OOM undetected, unit leak) | Scopes stop when the server ends. OOM is read from the cgroup's `memory.events` before removal. Restart keeps the unit and port. | `b862a8ce` |
| F02 `verify-<family>` shortcuts misrouted verify options | Verify options come before `--`; only the remainder passes through | `894ef1f0` |
| F03 `--cli` never set `LCTX_REMEDIATION_CLI_BIN` | Option-gated step environment | `894ef1f0` |
| F04 Any server exit reported as `blocked` | Only OOM, readiness, launch 127 or `FixtureBlocked` count as `blocked`; anything else is `failed` | `894ef1f0` |
| F05 A SIGKILLed foreground launcher left its tree running | Parent-death SIGTERM; the leader terminates its group | `894ef1f0`, `b7f8ec30` |
| F06 `worktree-remove` ignored live fixtures and runs | Refuses and names them; `--force` stops them through their routes | `5ba65185` |
| F07 Declared Python requirements didn't match imports | `model:rust` and `providers:extract` need tools; compiler, store and `serving:rust` don't | `894ef1f0` |
| F08 Step children lacked the ownership token | Passed to every step | `894ef1f0` |
| F09 Exclusive sync could starve | Pending-writer gate | `5ba65185` |
| F10 Previous-boot records never swept | `ProcessIdentity.previous_boot()` means dead | `2264ccda`, `b862a8ce` |
| F11 Setup exceptions aborted runs; failed retention blocked reuse | `FixtureBlocked` classification; abandoned retention is cleaned up | `b862a8ce` |
| Proportionality cuts | Fixture methods and run-dir records with no reader; hand-written target discovery replaced by `cargo metadata`; duplicate readiness types; holder records replaced by `/proc/locks`; the duplicated parent-death helper; unused CLI commands | `b862a8ce`, `894ef1f0`, `5ba65185`, `b7f8ec30` |

## 9. Finding dispositions

| Finding | Scenario | Disposition | Owner | Evidence / revisit trigger |
|---|---|---|---|---|
| AE-01, AE-02, AE-03, AE-05 | S1, S4, S6 | **implemented** (P2, P7) | — | *Tested* 2026-10-07: no rebuild from recipes or a bare `uv run`; a touched source reports `blocked: run just sync native`. The redundant `build_environment.py` cache key was removed. |
| AE-04 | S4, S6 | **implemented** | — | Harness scripts launch with `uv run --no-project`; `build_environment.py` is pinned to py312 for ruff and parses under 3.12 (*Tested*) |
| AE-06, AE-07 | S1, S4, S6 | **implemented** (P3, native binary) | — | *Tested* 2026-10-08: `just fixture -- cargo nextest … --test cache` 7/7 with no hand-set `LCTX_*`; SIGTERM/SIGKILL leave no server; OOM detected 3/3; restart keeps the port |
| AE-08, AE-09, AE-10 | S1, S2, S5 | **implemented** (P4) | — | *Tested* with fakes and static `--print`; one real background run with `summary.json`. Family wall time is unmeasured (performance is out of scope). |
| AE-11, AE-21 | S3, all | **implemented** (P6, ADR-0134) | — | *Tested*: `just fresh` writes nothing; scoped turn-end left the paused product files byte-identical |
| AE-12 | S2, S5 | routed | Testing architecture owner; catalog tests go with P1 | W2 family/target matrix. Revisit when that owner next revises families. |
| AE-13 | S7 | **implemented** (P5) | — | *Tested*: environment and locks follow the worktree under an inherited absolute environment; a foreign `UV_PROJECT_ENVIRONMENT` is dropped by every recipe |
| AE-14, AE-15 | S8, S9 | **implemented** (P7, D6) | — | *Tested* 2026-10-07: fresh sessions show no superpowers injection and no denied connectors; context went from 47.2k to 42.5k (recorded, not a criterion); Codex sees `UV_NO_SYNC=1`; AGENTS.md went from 28.7 KB to 25.3 KB |
| AE-16 | S9 | note | Operator (§10) | Memory is user-level |
| AE-17 | S1 | **implemented** (note) | — | *Tested* 2026-10-08: the LSP tool answers `documentSymbol`, while references and workspace symbols need a warm index. Revisit if wrong-path reads remain a visible share of exploration. |
| AE-18 | S8 | not proposed | — | Revisit if a skill goes missing in Codex |
| AE-19, AE-20 | all | **implemented** (P1) | — | *Tested*: a fresh Codex session starts no catalog server; no catalog tests are collected; `lint-agents` passes |
| First review F01–F08 | — | accepted | §8 | That review, §11–12 |
| Capability review F01–F08, RC01–RC06 | — | accepted | §8, §6 | That review, §3, §7 |
| Implementation review F01–F11 | — | **fixed** | §8 | That review |
| AE-22 Hakari stale at HEAD | S3 | **fixed** (`71f3a4ae`) | — | Found by `just fresh` |
| AE-23 103 committed Rust files are unformatted (mostly product work) | S3 | routed | The product owner's whole-tree `turn-end` on resumption | `just fresh rust-fmt` |
| AE-24 `just fresh` does not cover documentation publication, and the Hakari check can rewrite a stale `Cargo.lock` | S3 | transferred | [Follow-up plan §8](agent-effectiveness-followup-plan_2026-10-09.md#8-finding-and-recommendation-disposition-owner) | Original evidence: Drill D3-after; current disposition/closure remains at the linked owner |
| AE-25 The tooling family collects `test_programmatic_native*.py`, which need serving config | S2 | routed (AE-12) | Testing architecture owner | A's P2 run |

## 10. User- and machine-level notes (not packets)

- **Context7 token.** `~/.codex/config.toml` stores the Context7 bearer token in plain text.
- **Codex user instructions.** `~/.codex/AGENTS.md` duplicates the global rules and lacks the python-analyzers exception.
- **Codex memories.** The 16.8 KB of memories reference a nonexistent `cpg-schema` crate.
- **Stale Codex entries.** Some trust and hooks entries are stale (e.g. `/tmp/library-context-role-trial-*`).
- **Context7 guidance.** The global rule says to always use Context7; AGENTS.md says skills come first. In D5, both runtimes reconciled the two.
- **Claude project memory (AE-16).** About 7 of 27 entries describe retired mechanisms. Update or delete them on an explicit request; they are not part of repository completion.
- **Agent slice.** pse-arrow's `pse.slice` and its `CPUWeight` placement could become a machine-wide agent slice. That would be a cross-repo decision.
- **Swap** is effectively full (6.7–8 of 8 GiB).

## 11. Drill appendix (W6)

The drills were read-only, run once per runtime, and scored against targets written before the first drill (`targets.md` 19:07:23; first drill 19:07:43). With n = 1 there is no cross-runtime score, and the drills are not a baseline.

| Drill | Claude (turns, s, $) | Codex (s) | Outcome and friction |
|---|---|---|---|
| D1: focused tests for `codec::batch_bodies` (S1/S4) | 11, 60, 0.70 | 62 | Both correct, including the fixture-free unit loop. Both flagged `verify-store`'s sync and Docker side effects. Claude was unsure whether `--test` works across two packages. Both prefixed the build-environment eval or wrapper. |
| D2: tests reaching `cpg_core::artifact::admit` (S2) | 21, 112, 0.97 | 130 | Both found 20–21 tests across four boundaries in four separate invocations. Both hit "filters reach pytest only" and the `--command` rule. |
| D3: is anything generated stale (S3) | 14, 79, 0.70 | 69 | Both assembled about 12 commands by hand; Codex declared the task blocked. Both flagged that `docs-check` regenerates, that recipe `uv run`s sync, and that snapshots need tests. |
| D4: classify a captured failure (S6) | 8, 39, 0.53 | 45 | Both correct (fixture, then Python 3.12). Codex avoided `verify-store` because of its sync. |
| D5: SurrealDB per-statement errors (S8) | 9, 124, 0.55 | 47 | Both correct and version-checked against Cargo.lock. Claude used the selected skill and the registry source. It noted that the skill's top advice under-signals `take_errors`, and its working directory moved into the skill store. Codex used the skill, then Context7 (no matching docs), then the registry source. |
| D6: current work and the F03 owner (S9) | 8, 33, 0.55 | 36 | Both correct. Claude was distracted by unauthenticated connectors and noticed that STATUS lagged two commits. |

### After-change drills (2026-10-08, HEAD `72966523`)

D1 and D3 were rerun with targets written beforehand (`build/agent-effectiveness/w6-after/`). Each runtime ran once, so this is qualitative.

| Drill | Claude (turns, s, $) | Codex (s) | Compared with the baseline |
|---|---|---|---|
| D1 focused tests | 14, 61, 0.64 | 60 | Neither runtime ran the `eval build_environment` step or was surprised by an implicit sync. Both ran the unit loop with bare `cargo`, and the integration tests with `just fixture -- cargo nextest …`. Claude also offered `just verify --select store:rust`. |
| D3 generated outputs | 7, 45, 0.46 | 63 | Claude needed a single `just fresh` instead of about 12 hand-assembled commands. Codex assembled parts by hand, because the drill preamble forbids `uv run`. Both named AE-24 and the insta limit. |

## 12. Execution checkpoint (2026-10-08)

**Commits on `main`.**

| Area | Commits |
|---|---|
| Contracts and harness | `ec278be4` |
| P1 | `6ae79a2e`, `69dad0e1` |
| ADR-0134 | `c06651a2` |
| P2 + P4.1 | `f212bd42` |
| Decisions recorded | `442b8dbe` |
| P6 | `e962475a`, `0e4df1a2` |
| Runtime configuration | `f6813b45` |
| Hakari | `71f3a4ae` |
| P5 | `d59f193b`, `c4afd533` |
| P3 | `0f81b9c1` |
| P4.2 | `95297fe5` |
| P7 | `72966523` |
| Parallelism | `d9375ba4` |
| Implementation-review fixes | `2264ccda`, `b862a8ce`, `894ef1f0`, `5ba65185`, `b7f8ec30`, `c546799d` |

**Incidents and corrections during execution.**
- **Main's `.venv` repointed.** A worktree probe that carried main's absolute `UV_PROJECT_ENVIRONMENT` re-pointed main's `.venv` at worktree code for about 12 minutes. A rebuild repaired it. Every recipe and `ready` now drops a foreign absolute environment unless `LCTX_ALLOW_FOREIGN_ENV=1`.
- **Available test parallelism.** Nextest uses its normal available CPU parallelism. Resolve concurrency failures in production or test code.
- **Testing scope enforced mid-execution.** The operator restated it: limited functional testing, no validation of existing scope. A `lctx-publisher` value-check build was stopped mid-compile without a result, and the long mcp journey was dropped in favour of fakes plus static `--print`.
- **A completed run was pruned.** `just runs prune --keep 0` removed one completed run that another session had made. Pruning never touches active, interrupted or retained runs.

**Deviations from the plan.**
- The P5 build-concurrency probe and OD3 were dropped.
- `build_identity()` and `skillListingMaxDescChars` are deferred.
- The fixture runs the native binary (OD2) rather than Docker.

**Size.** The harness is larger than the rest of the scripts: `verify.py` about 1.4k lines, `surrealdb_fixture.py` about 1.5k, `runs.py` about 0.8k. The review's cuts were applied, and remaining size is a revisit item if maintenance cost shows.

**Not run (design-phase scope):**
- `qualify`;
- the full mcp journey;
- wide consumer suites;
- fixture-backed `verify` selections beyond the `cache` test;
- whole-tree `just turn-end`, because the paused product files are dirty; the scoped form was used instead.

## 13. Verification of this plan

Non-functional leaves, run once at scope end (2026-10-08, after the review fixes):

| Leaf | Command | Result |
|---|---|---|
| Agent instructions | `just lint-agents` | passed |
| ADR records | `just adr-lint` | passed |
| Types | `just types` | passed |
| Dependency policy (including Hakari) | `just deps` | passed |
| Ruff on the harness scripts and their tests | `uv run --no-sync ruff check …` | passed |
| Documentation | `just docs-check` | passed, 338 pages, after fixing three links this work had broken: the retired `sqlx-postgres` skill link, the deleted `native_controls.py` link and a review's relative plan link |

Functional evidence is limited to each packet's premise and value checks (§9, §12), under the design-phase scope. Harness unit tests (`uv run --no-sync pytest tests/scripts/test_{harness,workspace_env,build_environment,surrealdb_fixture,runs,verify,worktree,freshness,maintenance}.py`) passed at the final commits.

Not run:
- `just turn-end` in whole-tree form, because the paused product files are dirty. The scoped `just turn-end --paths …` was used on this work's paths.
- `just qualify`, by design-phase scope.
- Ruff on the miner, because the project ruff configuration excludes `docs/`.
