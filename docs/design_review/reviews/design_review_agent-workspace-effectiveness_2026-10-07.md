# Design review: agent workspace effectiveness

**Date:** 2026-10-07 · **Tier / purpose:** design / target · **Decision:** Revise (main direction retained)

**Subject:** [agent workspace effectiveness plan](../../plans/agent-workspace-effectiveness-plan_2026-10-07.md) (uncommitted, read as of 2026-10-07), its [evidence folder](../evidence/2026-10-07_agent-workspace-effectiveness/README.md) and the gitignored raw material under `build/agent-effectiveness/` (W1 report, W3/W4 captures, W6 drill outputs, targets and timeline).

**Disposition owner:** the plan's §8/§9 once the operator adopts it. This review creates no other ledger.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | The plan's D1–D7, RC01–RC05 and P1–P7, judged against the harness they would change. Baseline: HEAD `1be0dd96714aba85aeb0a01af48758a8e314f9b9`. Another agent's dirty `crates/cpg-core/*` and `semantic-model.md` were neither read for judgment nor touched. |
| Standard | Core principles and template 3.3; Heuristics for Efficient Architecture 1.0; [library-context binding](../design_principles/binding/library-context.md). The code-intelligence profile 1.5 does **not apply**: nothing here extracts, relates or serves code facts. |
| Tier · purpose | Design · target. This is a new harness boundary that changes ownership of environment sync, fixture lifetime and parallel-checkout policy. |
| Reviewer · date | Independent `design-reviewer` subagent, 2026-10-07 |
| Maturity and outcome | Proposed; no harness change has been implemented. This decision should let the operator confirm D1–D7, so that P1–P7 can be implemented without rediscovering contract gaps. |
| Supported scope | The development harness on one workstation, used by Claude Code 2.1.293 and Codex 0.161 (weighted equally), by subagents, and by the operator in a plain shell. It covers environment and native-extension sync (uv 0.12.22, maturin), the fixture command boundary (`scripts/surrealdb_fixture.py`, `scripts/native_controls.py`), verify transparency (`scripts/verify.py`, `justfile`), parallel checkouts, freshness, instructions and runtime configuration. **Workload premise:** a handful of concurrent agent sessions with subagents. Each may run release nextest families of 5–90 minutes, Docker SurrealDB fixtures and a 175 MB in-tree PyO3 extension, against one shared Cargo build directory, tool-call timeouts of 2–10 minutes. **Excluded:** product architecture, performance, and test-family coverage (routed to the testing owner). |
| Expected changes | S1–S9 and the plan's variation axes. Emphasized: S4/S6, a fixture run interrupted by a tool timeout (SIGKILL of the process group); S7, two agents with live native workers while one syncs; S7, a worktree whose agent session was started in main; S2/S5, a filtered mcp run reusing a retained fixture; operator adoption (Q1) registering the lctx MCP server through `uv run`. |
| Baseline | Implemented: syncing verify readiness under an exclusive `.venv/.verification.lock` held for the whole run; an env-keyed in-tree editable extension; a run-scoped fixture context manager without signal handling; families that hard-code packages; instructions that prescribe the build-environment eval. |
| Method and coverage | Static reading of the plan, evidence README, `metrics.json`, the W1 report and W6 timeline; `verify.py`, `native_controls.py`, `surrealdb_fixture.py`, `build_environment.py`, `justfile`, `.cargo/config.toml`, workspace `Cargo.toml` profiles, both pyprojects, `.claude/settings.json`, `.codex/config.toml`, `.mcp.json`, and the fixture consumers (`crates/cpg-core/tests/fixtures/native.rs`, `crates/lctx-surrealdb/src/{cache,config}.rs`, `crates/lctx-serving/tests/native_journey.rs`, `crates/lctx/src/main.rs::uv`). Read-only inspection of the installed `lctx_semantics` dist-info. Three trivial uv parse probes in the session scratchpad (§10). Upstream moby source read at `master`. **Not examined:** maturin 1.15's editable write path; Claude Code's handling of `disableClaudeAiConnectors`; Codex's project-layer handling of `shell_environment_policy` and `project_doc_max_bytes`; the cargo source for fine-grain locking. No uv sync, just recipe, cargo build, test or Docker command was run. |

## 2. Responsibilities, dependencies and semantic ownership

Ten components share the harness, and three cross-cutting facts govern how the proposals compose.

| Component | Responsibility and hidden decisions | Consumer contract | Expected reason for change |
|---|---|---|---|
| `build_environment.py` | Normalizes inherited Cargo target paths; computes the content/membership key `LCTX_NATIVE_SEMANTICS_INPUTS` | `just` shell, `--shell` eval, `normalized_env()` | Key inputs; launcher policy |
| `python/lctx_semantics` + uv | Editable maturin install. The 175 MB `_native*.so` is written **into the source tree** (`python/lctx_semantics/python/lctx_semantics/`), reached through a `.pth` holding an absolute checkout path. uv decides rebuilds from `uv_cache.json` (the installed record holds `{"LCTX_NATIVE_SEMANTICS_INPUTS": null}`, observed 2026-10-07) | `import lctx_semantics` | Sync policy, artifact identity |
| `verify.py` | Families, prerequisites, readiness (currently `uv sync`), outcome mapping, the environment lock | `just verify-*`, `qualify` | D1, D4 |
| `native_controls.py` | Owns the compiler runtime config (cache database, selection file, viewer credentials); boundary package selection; the mcp journey, `restart()` and retained serving config | verify families | D2, D4 |
| `surrealdb_fixture.py` | Container lifecycle: name, port, memory cap, credentials, readiness/version check, removal by container ID | `fixture()`, CLI, `build_measurements.py` | D2 |
| `native_journey.rs` | Test that, as a side effect, writes `serving.json` with the endpoint and port baked in (`:704-724`) and creates the selection with `create_new(true)` | mcp pytest | D2/D4 reuse |
| `lctx::uv` (Rust) | Strips every `UV_*` and `VIRTUAL_ENV` before acquiring analyzed libraries | acquisition | **Preserve:** D1's runtime env cannot leak into acquisition |
| `.cargo/config.toml` | Shared `build-dir`, sccache wrapper, `-Zfine-grain-locking` | every cargo invocation, every checkout | D3 |
| Runtime configs | Claude `settings.json` (env, plugins); Codex `config.toml` (shell policy, MCP, doc budget); `.mcp.json` | sessions, subagents, MCP children | D1, D5–D7 |
| Instructions | AGENTS.md, roles, skills README | every session | D5, RC01–RC05 |

The three cross-cutting facts:

- **Environment synchronization.** After D1 its authority is `just sync`. The runtime env makes incidental `uv run` non-syncing, but only inside agent shells.
- **Fixture lifetime.** After D2 it has three kinds: run-owned, kept, and orphaned (owner died). Only the first is defined in the plan.
- **Verification outcome and coverage.** Its authority is verify's `summary.json`. It must still distinguish *established in this run* from *reused*.

Dependency direction is sound throughout. Recipes are thin over scripts; scripts import modules, never recipes; product code (`lctx::uv`) is insulated from the harness env. The plan's choice of one fixture owner, with `native_controls` delegating to it, is correct. The remaining question is whether the operations those owners expose carry the right meaning for lifetime, override and coverage.

## 3. Contracts and testing boundaries

| Contract | Consumer expectation | Gap in the plan as written |
|---|---|---|
| `uv run` in agent shells | Never rebuilds incidentally; `UV_NO_SYNC=0` restores syncing | Holds in Claude for every child process, but in Codex only for shell commands. Plain shells, the IDE and repository-defined launchers are not covered (F03). |
| `just sync` | The one keyed synchronizer | Nothing coordinates it with live importers of the in-tree `.so` (F02). |
| Readiness | Observes; reports `blocked: run just sync` | The observation is unscoped: it blocks families that never import the extension. `--frozen` drops the current `--locked` check (F06). |
| `just fixture -- cmd` | Every native variable supplied; cleanup on every exit | Kept/orphan lifetimes, sweep safety, SIGKILL recovery, attach in non-persistent shells, and port stability across `restart()` are undefined (F01). |
| `verify --print` / run directories | Truthful selection, cost and outcome | "What the filter reaches" requires building; reused fixtures and skipped journeys must not read as established (F05). |
| `just worktree` | Parallel builds and tests; own venv and extension | Parallel artifact builds are unproven; cold workspace builds; origin under an inherited main env; no route back to main (F04). |

## 4. Composition and execution

The plan composes its remedies in three places where the composition, not the individual remedy, decides correctness.

1. **D1 × D4: what the environment lock protects.** Today `environment_owner` exists because `prepare()` mutates the shared venv (`verify.py:215-218`). D1 removes that mutation, but D4 keeps the lock and only announces it. The lock then serializes two read-only verify runs, while the one remaining mutator, `just sync`, is not described as taking it. That is the H23 failure (exclusion detached from the effect it protects); see F02.
2. **D2 × D4: retained serving fixture × daemon-allocated port × `restart()`.** The mcp boundary restarts the container (`native_controls.py:110`) after a Rust test has written the endpoint into `serving.json`. A port allocated by the daemon (`-p 127.0.0.1::8000`) is not guaranteed to survive `docker restart` (see F01 and §10). Making the fixture reusable across runs (P4) extends the window in which that baked-in endpoint must stay valid.
3. **D3 × shared build directory × incremental workspace profile.** Workspace members build with `incremental = true` (`Cargo.toml:139-162`), and sccache does not cache incremental compilations. Unit hashes for path packages also differ per checkout path. A new worktree therefore compiles every workspace crate cold, whichever build directory it uses. The shared build directory only warms registry and git dependencies (F04).

**Execution fit (A4).** No change here amplifies product work. The material execution questions are resource lifetimes:

- containers outliving their launcher;
- an exclusive lock held across 15–90 minute families for an effect that no longer occurs;
- per-worktree intermediates accumulating in a build directory that policy forbids cleaning;
- run directories accumulating under `build/verify/`.

These are qualitative assessments; no quantitative claim is made.

## 5. Change and failure scenarios

| Scenario (kind) | Owner | Proposed route | Assessment |
|---|---|---|---|
| A tool timeout kills `just fixture -- cargo nextest …` by SIGKILL to the process group (failure) | fixture module | SIGTERM handler, plus "a cleanup query lists or removes leaked runs by label" | The handler never runs. Recovery depends entirely on the sweep, and the sweep has no liveness rule, so it could remove another agent's live or kept fixture. **Violated** (F01). |
| A filtered mcp run reuses a retained fixture (composition) | native_controls / verify | "mcp reuse … so filters reach the tests they name" | The journey and the restart-persistence property are not established in this run. Nothing says how `summary.json` records that. **Unresolved** (F05). |
| Agent A runs `just sync` while agent B's `verify-serving` has PyO3 workers live (concurrency) | `just sync` | Not addressed; D1 claims it "makes AGENTS.md:251 true" | The `.so` is rebuilt in place in the source tree, and nothing prevents or reports the collision. **Violated** as claimed (F02). |
| Q1 adoption registers `lctx_mcp` via `uv run` in `.mcp.json` and `.codex/config.toml` (binding) | runtime configs | UV_NO_SYNC through runtime env | Claude's MCP child inherits no-sync; Codex's MCP child does not (`shell_environment_policy` reaches shell commands only), so it would sync unkeyed and rebuild. **Violated** for repository launchers (F03). |
| An agent started in main works in `~/library-context-wt/x` (variation) | worktree recipe | `--python` builds its own venv | `uv run` honours the worktree project. But a bare `python3`/`pytest` resolved through an inherited `VIRTUAL_ENV`/`PATH` imports main's in-tree `.so` through its absolute `.pth`. The acceptance case checks only the `uv run` route. **Unresolved** (F04). |
| Two worktrees run release `cargo nextest run` at once (concurrency) | Cargo | "Worktrees, not locks" | AE-13 itself states fine-grain locking helps check-type commands only, and P5's acceptance proves only overlapping checks. **Violated** as claimed (F04). |
| Store family while the extension is stale (policy) | readiness | Global `uv sync --check` | A family that never imports the extension is blocked. **Violated** (F06). |
| Operator runs `just qualify` from a plain shell (policy) | verify | Readiness no longer prepares | The intentional behaviour change is not stated, and the AGENTS testing rule "families prepare their actual prerequisites" changes with it (review#RC03). |

The domain extension (a new fixture kind or family) has a credible single-owner route once F01 and F05 hold. A mechanism substitution (a native `surreal` 3.3.0 child process instead of Docker) is credible and simpler for run ownership; see §9.

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence | Required action |
|---|---|---|---|
| G1 Authority | unresolved | One fixture owner and one synchronizer are named, but kept/orphan authority and lock meaning are not defined | F01, F02 |
| G2 Semantic fidelity | unresolved | The meaning of `passed` for a run that reused a fixture is undefined | F05 |
| G3 Validity | unresolved | `--print` validity of filters is promised without saying how it is established | F05 |
| G4 Hidden behavior | pass for D1's direction; unresolved for P2's optional import gate | Removing incidental sync removes a hidden effect. A non-bypassable import failure would add hidden authority. | F06 |
| G5 Consistency and recovery | **fail** | The label sweep has no owner-liveness rule; SIGKILL recovery is undefined; `restart()` can invalidate retained configs | F01 |
| G6 Transformation and reuse | unresolved | The validity premise for reusing a retained serving fixture is unstated | F05 |
| G7 Truthful capability claims | **fail** | "Parallel build/test" via a shared build directory; "ends the ping-pong" and "makes AGENTS.md:251 true" exceed the mechanisms | F02, F03, F04 |
| G8 Library leverage | pass (Proposed) | uv, Docker labels and `docker port`, nextest, git worktree and runtime configs supply the capabilities; nothing bespoke where a tool exists | — |
| Profile gates | n.a. | Code-intelligence profile not applicable | — |

## 7. Findings

The findings are ranked by consequence. Each states the concrete plan edit it implies; §11 holds the rule impacts.

### <a id="f01"></a>F01 — The fixture boundary has run cleanup but no run ownership for kept, orphaned or restarted fixtures

**Principles:** FP-04, FP-06; DP-19, DP-20 · G5, G1 · pse-arrow F03 lineage.

**Evidence.** `surrealdb_fixture.py:178-248` ties the lifetime to a context manager. Removal is by container ID and is safe within one process. D2/P3 adds `--keep`, `--stop ID`, a label, and "a cleanup query [that] lists or removes leaked runs by label". Three problems follow:

- **Leaked versus live.** The cleanup query cannot tell a leaked run from another agent's live run or from a kept fixture.
- **Kills the handler never sees.** Tool timeouts commonly kill the whole process group with SIGKILL, so the handler is skipped and the sweep becomes the *primary* recovery path, not a backup.
- **Exports that don't persist.** `--keep` "prints exports", but neither Claude nor Codex keeps shell state between commands.

Kept fixtures also carry state across commands. `compiler_cache` is a fixed cache database (`native_controls.py:60`, `lctx-surrealdb/src/config.rs:18`), the selection file persists, and `native_journey.rs:706-711` uses `create_new(true)`. A second command therefore exercises warm-cache or conflicting paths, not the cold paths the first exercised.

**Port and restart.** P3 replaces bind-0 with `-p 127.0.0.1::8000`. The mcp boundary calls `owned.restart()` *after* `native_journey` has baked the endpoint into `serving.json`. Ephemeral host ports are reported to change across `docker restart` (forum reports only; unverified at 29.8.2). If they do, P3 breaks the mcp boundary.

**`--rm`.** An `--rm` flag would not help here. It removes only containers that have exited, so an orphaned running container stays. It also discards `State.OOMKilled`, which is the one signal that would turn AE-07's "silent memory cap" into a diagnosable infrastructure failure. On moby master, `docker restart` suppresses auto-removal (`HasBeenManuallyRestarted` gates `autoRemove` in `daemon/monitor.go`). That is Interface-checked at master only, and is moot if `--rm` is not adopted.

**Consequence.** Concurrent agents can delete each other's fixtures; leaked containers persist after timeouts; and a correct-looking port change breaks the mcp control.

**Plan edits (P3/D2).**

- **Three lifetimes.**
  - *Run-owned* is the default. Labels: `lctx.fixture.run`, `owner_pid`, owner start time, `checkout` and `scratch`.
  - *Kept* is explicit, with a `kept=1` label. It is removed only by `--stop ID` or an explicit `--stop --kept`.
  - *Orphaned* means a run-owned fixture whose owner is dead.
- **Sweep.** Every `just fixture` start, plus `--sweep`, removes only orphaned run-owned fixtures and their scratch directories. Kept fixtures are listed, never removed.
- **Signals.** On SIGTERM, SIGINT or SIGHUP: terminate the child process group, wait for it, inspect the container, then remove it.
- **Kept fixtures.** Provide `--attach ID -- cmd`, or write an env file whose path is printed, instead of printed exports. Give each attached command a fresh cache database and selection path, or state the carried state.
- **Port.** Keep a launcher-owned, stable host port with retry on a "port is already allocated" collision. Otherwise, re-resolve `docker port` after every `restart()` and rewrite every derived config, which is a cross-owner edit of the test-written `serving.json`. The stable port is the simpler choice.
- **No `--rm`.** On a failed command, report `blocked: fixture OOM-killed at cap X` from `docker inspect`.
- **Memory override.** `LCTX_FIXTURE_MEMORY` scales `SURREAL_MEMORY_THRESHOLD` to half the selected allocation and reports both values.
- **Serving fixture.** Keep it a mcp-boundary concern rather than a general `just fixture` option. A failure to produce it reports `blocked`, not a failure of the user's command.

**Acceptance.**

- SIGKILL the process group of a running fixture command; the next `just fixture` start removes the orphan and leaves a concurrently running and a kept fixture untouched.
- The mcp boundary passes across `restart()` with the chosen port policy.
- An OOM at a small cap reads as infrastructure.
- The coordinator runs one Docker probe: publish `127.0.0.1::8000`, `docker restart`, compare `docker port` on 29.8.2.

### <a id="f02"></a>F02 — Removing sync from readiness changes what the environment lock protects; `just sync` is uncoordinated with live importers

**Principles:** FP-03, FP-07; DP-19, DP-20 · G7 · Heuristic H23 · pse-arrow F02/F05 lineage.

**Evidence.** The lock comment (`verify.py:215-216`) says ownership is held so that "another family launcher [cannot prepare] the shared environment while these fixtures run". After D1 no family prepares anything. D4 keeps the lock, announces its holder, and makes "a second concurrent tools run prints who holds the lock" an acceptance case. So the plan preserves serialization between two read-only runs. `just sync`, the only remaining mutator, is not said to take the lock.

The extension is rebuilt *in the checkout's source tree*, and live PyO3 workers map that file. How maturin 1.15 writes it, by replacing the inode or by overwriting in place, was not verified. D1's claim that it "makes AGENTS.md:251 true" therefore exceeds the mechanism. It makes collisions deliberate rather than incidental, but they are not prevented or reported.

**Consequence.**

- Verify runs serialize for no protected effect, up to 90 minutes behind each other.
- A deliberate sync can still replace the extension under live workers.
- An agent reading the announcement learns about the wrong contention.

**Plan edits (P2/P4).**

- `.verification.lock` becomes shared for verify runs and for `just fixture` runs that import the extension, and exclusive for `just sync`.
- `just sync` lists all current holders (pid, family, start time; a set, not "the holder"), then waits, or refuses with `--no-wait`.
- Bare `uv run --no-sync pytest` does not register. State that AGENTS.md:251 remains a rule for the syncing agent, now with observable holders.
- If the operator wants verify runs serialized for load reasons, make that a separately named, overridable policy, not a side effect of environment ownership.
- Replace P4's acceptance case with two: `just sync` during a live `verify-serving` reports the holder and waits; two verify runs proceed concurrently.
- Settle maturin's write semantics by reading its editable install path, or have `just sync` refuse while holders exist.

### <a id="f03"></a>F03 — `UV_NO_SYNC` through runtime configuration has asymmetric reach and lifetime; repository launchers must not depend on it

**Principles:** FP-02, FP-06; DP-18, DP-24 · G7 · pse-arrow F01 lineage.

**Evidence (uv 0.12.22).**

- `uv run`, `uv add` and `uv remove` all take `--no-sync` from `UV_NO_SYNC`. `uv sync` does not (`--help`, 2026-10-07).
- `UV_NO_SYNC=0` and `=false` parse as false, and `UV_NO_SYNC=1` raises no conflict with `--frozen` or `--locked` (scratch probes, §10).
- `UV_NO_SYNC=1` with `--no-project` prints "`--no-sync` has no effect when used alongside `--no-project`" on every run. That covers `adr`, `adr-index`, `lint-agents` and all `docs*` recipes.

**Reach of the runtime env.**

- Claude's settings `env` reaches every Claude child: Bash, hooks and stdio MCP.
- Codex's `shell_environment_policy.set` reaches shell commands, not MCP servers.
- Neither reaches the operator's plain shell, the IDE, or `just` recipes run there.
- Nested launchers in scripts inherit whatever the caller had. For example, `embed_serve.py:60` runs `uv run --project services/vllm --frozen`, which would silently skip syncing the vLLM project under an agent and sync it under the operator.

**What D1 delivers.** It ends the AE-01 ping-pong for agent ad hoc commands only. P2's first acceptance case, "from a fresh shell: `just fmt` then a bare `uv run …` causes no rebuild", fails in a plain shell, because nothing sets `UV_NO_SYNC` there.

**Plan edits (D1/P2/RC01).**

- Every repository-defined launcher carries explicit `--no-sync`, or performs an explicit, owned sync. That covers recipes, scripts' nested `uv run` (including `embed_serve`'s other project), and any future MCP registration such as the Q1 `lctx_mcp`. The runtime env is only the default for ad hoc agent commands.
- State the reach per runtime, and state that the operator's unkeyed syncs remain possible but are now reported by readiness.
- State override precedence:
  - inline `UV_NO_SYNC=0` for a command;
  - `settings.local.json` for a Claude session;
  - `-c shell_environment_policy.set.UV_NO_SYNC=0` for Codex.
- Run P2's first acceptance case in a fresh Claude session and a fresh Codex session, not a "fresh shell".
- Accept or suppress the `--no-project` warning deliberately.
- Preserve `lctx::uv`'s stripping of `UV_*`.

### <a id="f04"></a>F04 — Worktree concurrency and isolation are claimed beyond what the mechanism and acceptance establish

**Principles:** FP-07; DP-15, DP-20, DP-22 · G7 · pse-arrow F02/F07 lineage.

**Evidence.**

- **Locking.** D3 offers "parallel build/test work … worktrees, not locks" over a shared `build-dir` (`.cargo/config.toml:6`). But AE-13 says fine-grain locking "helps check-type commands only", and P5's acceptance proves only that two checks overlap. The pinned cargo 1.101.0-nightly (3d7cf6e93, 2026-09-25) describes `-Zfine-grain-locking` only as "instead of locking the entire build cache"; the tracking issue limits parallelism to non-artifact commands.
- **Cold builds.** Workspace crates are incremental, so sccache cannot serve them, and their unit hashes differ by checkout path. Every new worktree therefore builds cpg-core, lctx-model and the rest cold. §7.1 rejects per-agent build directories because they "lose sccache", but the observed workaround *disabled* sccache. A per-worktree build directory with sccache enabled would keep warm registry dependencies and avoid any shared-directory serialization.
- **Accumulation.** Removed worktrees leave their intermediates in a build directory that policy forbids cleaning.
- **Isolation.** The extension's `.pth` holds an absolute checkout path, and agents launched in main inherit main's `VIRTUAL_ENV` (Claude) or `PATH` (Codex), per AE-04. Bare `python3` or `pytest` in a worktree can therefore import main's `.so`.
- **Missing preparation and integration.** The skill links in `.claude/skills/` are gitignored, so a new worktree has none until `just ready` runs. The plan has no route for work in a worktree to return to `main`, which AGENTS.md Git requires, and no explicit route to transfer dirty work (65 `git apply` in W1).

**Plan edits (D3/P5/RC03).**

- Either run one probe: two concurrent release `cargo build -p <small crate>` in two worktrees on the pinned toolchain, watching for "Blocking waiting for file lock on build directory". Or restate D3 as "parallel edits and checks; artifact builds may serialize", and compare it with a per-worktree `build-dir` with sccache enabled.
- State the cold-build cost. Prefer stable worktree names (slots) so units are reused, and name the intermediates-retention consequence.
- Split *create* from *prepare*. `just prepare` (sync plus skills-sync) should run in any checkout, including Claude- and Codex-native worktrees.
- Acceptance checks import origin from a session started in main, through bare `python`/`pytest` as well as `uv run`.
- Add an explicit dirty-work transfer option and the route back to main (review#RC02).

### <a id="f05"></a>F05 — Verify transparency can misreport coverage through `--print` and fixture reuse

**Principles:** FP-05; DP-19, DP-21, DP-22 · G2, G3, G6 · pse-arrow F04 lineage.

**Evidence.**

- **`--print`.** It "runs nothing" yet "shows what the filter reaches". Reach under nextest filters needs built test binaries (`cargo nextest list`). Statically, only the commands and the validity of target selectors (via `cargo metadata`) are knowable.
- **mcp reuse.** Reuse skips the journey and its `restart()`. That journey is the only thing in the boundary that establishes the published fixture and its persistence across a restart. A run that reuses an older fixture could report `passed` for tests run against content produced from different serving or eval inputs.
- **Retention.** Run directories have no retention rule, while P1 deletes 2.8 GB of the same kind of accumulation.

**Plan edits (D4/P4).**

- Split `--print` (static: commands, env *names*, prerequisites, metadata validity) from an explicit `--list` that is labelled as building.
- `summary.json` records a per-boundary outcome, including `not_run` for a skipped journey or restart. A reused fixture records its run ID, its producing inputs fingerprint and its age.
- Reuse requires the fingerprint to match the current serving and eval inputs (an observation, not a cache).
- `qualify` refuses reuse.
- Add a retention rule, such as keeping the last N runs plus every failed run.
- Revise P4's revealing case to "a filtered mcp run with a retained fixture skips the journey **and reports it as not_run with the fixture identity**".

### <a id="f06"></a>F06 — Readiness observation must be scoped to the prerequisite, and the extension gate must stay an observation

**Principles:** FP-04; DP-09, DP-18 · G4, G6 · pse-arrow F05 lineage · H13, H18.

**Evidence.**

- **Unscoped observation.** D1 specifies one observation, `uv sync --frozen --check --inexact` with the key. The current prerequisites distinguish `tools` (dev group) from `native-python` (`verify.py:91-101, 127-141`). Store, compiler and providers do not import `lctx_semantics`, yet a global check would block them on extension staleness.
- **Weaker lock check.** `--frozen` drops the `--locked` lock-consistency check that `prepare()` performs today.
- **ctime keys.** The file `cache-keys` use ctime, so a touch or branch switch with unchanged content reports stale and triggers a 175 MB rebuild. That is safe but conservative.
- **Import gate.** P2's optional embedded fingerprint "fail[s] import", and §7.1 prizes that it "cannot be bypassed". That contradicts rubric 8 (overridable guards). It would also hash several hundred source files at every import, and it would break imports outside a source checkout.

**Plan edits (P2).**

- Scope observations per requirement:
  - `tools`: `uv sync --locked --check --inexact --only-group dev`;
  - `native-python`: the full check with the key.
- Keep `--locked`.
- State that `qualify` no longer self-prepares, so the operator runs `just ready` or `just sync` first (review#RC03).
- Make the fingerprint an exposed `build_identity()` that readiness compares. If import ever refuses, an environment override such as `LCTX_ALLOW_STALE_NATIVE=1` must exist and be printed.
- Optionally, drop the ctime file keys in favour of the content key now that syncing is explicit, accepting that unkeyed operator syncs then always rebuild.

### <a id="f07"></a>F07 — Several evidence labels and rates overstate what the sample supports

**Principles:** DP-22 · G7.

**Sample.** `metrics.json` corpus for the current period:

- **Claude:** 0 sessions in P5; 2 main sessions (111 shell commands) and 10 subagent sessions in P4.
- **Codex:** **1** main session plus 43 subagent sessions in P5.

**Rates that describe one workflow.** The 4,688 copy-located commands (2,665 `cache-copy` plus 1,993 `tmp-copy`, all Codex subagents) and the 164 hand-made `flock` wrappers ("almost all in P5") largely describe one Codex coordinator's workflow, not "agents". The following are rates, which the plan's own "not a rate study" caveat disclaims:

- "8.6% of Codex subagent build/test commands";
- "about a quarter";
- "5.7% probe misses";
- AE-17's "> 5%" revisit trigger.

**Wheel rebuilds.** "44 visible wheel rebuilds" counts the `native-wheel-build-in-output` class. The only recorded spot check of wheel-build classification found 0 of 8 caused by the command, and the corrected class has no spot check of its own.

**Drills.** They are n = 1 per runtime and scored by the plan's author against targets written beforehand (`targets.md` 19:07:23, before the first drill at 19:07:43). That is sound qualitative evidence, but not a baseline for measuring improvement.

**Plan edits.**

- Attribute AE-10 and AE-13 counts to "the P5 Codex coordinator and its subagents".
- Present percentages as counts with their denominators and strata.
- Replace AE-17's trigger with a qualitative one.
- Drop the "44 rebuilds" figure, or spot-check its class.
- Label Claude/Codex configuration keys (`disableClaudeAiConnectors`, the project-layer `project_doc_max_bytes` and `shell_environment_policy`) Interface-checked until P7's fresh-session acceptance.
- Record the AE-01 `--check` probe's command and date for its Tested label.

### <a id="f08"></a>F08 — Smaller composition and lifecycle items

**Principles:** DP-16, DP-24.

- **`justfile` interpreter.** Every recipe's shell is the system `python3 scripts/build_environment.py` (`justfile:8`), which is 3.12. P3 moves docs and docstrings to `uv run --no-sync python` but leaves this trap. State in that script's docstring that it must parse under the system Python. That is a comment, not a lint.
- **Other `python3` callers.** `build_measurements.py:484` also calls `python3 scripts/surrealdb_fixture.py`; include it in AE-04's fix.
- **Snapshot detection.** P6's static orphaned-snapshot detection is heuristic. Either label it so, or rely on insta's unreferenced-snapshot handling during an actual test run.
- **Package selection.** P4's "invalid filters are reported, not guessed" belongs with F05's metadata-validity print. Both should derive from the family's actual package set rather than a second hard-coded list.

## 8. Library fit and total complexity

| Capability | Candidates | Fit at resolved version | Choice |
|---|---|---|---|
| Suppress incidental sync | `UV_NO_SYNC` env; explicit `--no-sync` per launcher | Both are supported by uv 0.12.22; the env var is boolish | Both: explicit flags for repository launchers, env for ad hoc commands (F03) |
| Lock-consistency check | `uv sync --check` with `--locked` or `--frozen` | Both supported; `--frozen` skips lock drift | `--locked` (F06) |
| Fixture identity and cleanup | Docker labels plus `docker ps --filter label=`; `--rm`; native child process | Labels fit; `--rm` loses OOM state and doesn't cover orphans | Labels with owner liveness (F01) |
| Ephemeral port | Daemon `::8000`; bind-0 with retry | The daemon port is unverified across restart | Stable launcher port with retry, unless probed (F01) |
| Parallel builds | Shared `build-dir` with fine-grain locking; per-worktree `build-dir` with sccache | The first is proven only for checks | Probe or restate (F04) |
| Run results | Runner-written `summary.json`; nextest JUnit | JUnit is stable in nextest 0.9.146 and carries per-test outcomes | `summary.json` per boundary, optionally linking nextest's JUnit for per-test detail rather than reinventing it |

Total machinery stays light. Nothing proposed is a registry, daemon or lint, and the corrections add only labels, a shared/exclusive lock mode and two print modes.

## 9. Alternatives and tradeoffs

| Alternative | Change propagation | Authority and composition | Machinery and risk | Decision |
|---|---|---|---|---|
| Current baseline | Every agent reinvents fixtures, locks and copies | Readiness mutates the shared venv; one lock serializes everything | High hidden cost (W1), latent rebuild collisions | Replace |
| Plan as written | Mostly local to scripts and configs | Sound single owners, with the lifetime, lock and coverage gaps above | Light, but G5/G7 failures | Revise per F01–F06 |
| **Native `surreal` 3.3.0 child process** for run-owned fixtures (installed at `~/.local/bin/surreal`, version-checked by the existing `ready()`) | Same fixture module; `native_controls` unchanged | The child dies with the launcher's process group. It is placed in the agent's cgroup, so memory is accounted where agents can see it, and a restart reuses the launcher's port. | Removes the orphan, port-restart and system.slice concerns for run-owned fixtures. Docker remains the exact-digest parity image (`surrealdb_fixture.py:30-35`). | Worth a §7.1 row. Adopt it only if the parity contract can name the binary's provenance as exactly as the image digest. Revisit when F01's probe shows port instability. |
| Simplest viable | `UV_NO_SYNC` plus explicit `--no-sync` launchers; readiness observes; fixture labels plus sweep; worktree prepare recipe; `--print`/`summary.json` | As above | Smallest set that removes the observed workarounds | This is the plan corrected by F01–F06, not a different design |

## 10. Verification and uncertainty

| Claim | Label · date | Evidence | Gap |
|---|---|---|---|
| `UV_NO_SYNC` is boolish; no conflict with `--frozen`/`--locked`; `--no-project` warning | Tested 2026-10-07 | In the session scratchpad (no project): `UV_NO_SYNC={1,0,false} uv run --offline --no-project {--frozen,--locked} python -c 'print("ok")'` — all exited 0. Only `=1` printed "`--no-sync` has no effect when used alongside `--no-project`". | Behaviour inside the project was not exercised |
| `uv run/add/remove` read `UV_NO_SYNC`; `uv sync` has `--check` | Interface-checked 2026-10-07 | uv 0.12.22 `--help` | — |
| Extension in source tree; `.pth` absolute; installed key `null` | Interface-checked 2026-10-07 | `.venv/.../lctx_semantics-0.1.0.dist-info/{uv_cache.json,direct_url.json}`, `lctx_semantics.pth`, `ls python/lctx_semantics/python/lctx_semantics/` | — |
| `docker restart` preserves `--rm` containers | Interface-checked, moby `master` only | `daemon/monitor.go` `HasBeenManuallyRestarted` gate | Not checked at the 29.8.2 tag |
| Daemon-allocated host port changes across restart | Unresolved | Docker forum reports | **Settling probe** (coordinator, Docker allowed): publish `127.0.0.1::8000`, restart, compare `docker port`. This decides F01's port policy. |
| Shared build directory serializes artifact builds | Unresolved | Plan AE-13 text; cargo `-Z help`; tracking issue #4282 | **Settling probe** on the pinned toolchain decides D3's wording or a per-worktree `build-dir` |
| maturin 1.15 editable `.so` replacement | Not examined | — | Read maturin's editable write path. It decides whether `just sync` must refuse with live holders (F02). |
| Claude/Codex config keys honoured at project layer | Proposed / Interface-checked by the plan | W4 captures | P7's fresh-session acceptance settles them |

The diagnoses in F01–F06 rest on source and interface evidence and stand without the probes. The probes choose between remedies: port policy, build-directory sharing and the sync refusal rule.

## 11. Authority changes and dispositions

These are review impacts in addition to the plan's RC01–RC05. They take effect only on operator confirmation in the plan.

| ID | Rule and location | Proposed change | Findings | If kept |
|---|---|---|---|---|
| <a id="rc01"></a>review#RC01 | AGENTS.md:251 "never synchronize the environment while native guards/workers are live" | Keep the rule. `just sync` takes exclusive environment ownership and reports live holders; verify and fixture runs hold it shared. The rule text then names that mechanism. | F02 | Collisions remain possible and invisible |
| <a id="rc02"></a>review#RC02 | AGENTS.md Git: "Work on `main` in the current working tree … Commit to `main`" | Add the integration route for worktree work (a branch plus merge or cherry-pick to main, or explicit patch transfer). Plan RC03 changes only :338. | F04 | Worktree work has no sanctioned way back |
| <a id="rc03"></a>review#RC03 | AGENTS.md Testing: "Provider, store, serving and oracle families prepare their actual prerequisites once before execution"; `verify.py` docstring ("readiness is run-owned preparation") | Families observe their prerequisites and report `blocked`. Preparation is `just sync`/`just ready`, and `qualify` no longer self-prepares. | F06 | D1 contradicts the testing rule |

Finding dispositions belong in the plan's §9 once adopted. Recommended dispositions:

| Finding | Recommended disposition |
|---|---|
| F01 | Revise P3 before implementation |
| F02 | Revise P2/P4 together |
| F03 | Revise D1/P2 and RC01 wording |
| F04 | Revise D3/P5; settle by probe or restated claim |
| F05 | Revise P4 |
| F06 | Revise P2 |
| F07 | Edit §4/§9 text |
| F08 | Fold into P2/P3/P4/P6 |

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence | Action |
|---|---|---|---|
| A1 Localize change | satisfied | Each remedy lands in one owner: the fixture module, verify, runtime configs or recipes. `lctx::uv` insulates product acquisition. | Preserve |
| A2 Encode domain meaning explicitly | violated | Fixture lifetimes, the meaning of lock ownership and the established-versus-reused outcome are consequential distinctions that the proposal leaves implicit | F01, F02, F05 |
| A3 Extend through composition | unresolved | The remedies compose unsafely at D1×D4, D2×restart and D3×shared build directory | F02, F01, F04 |
| A4 Fit execution to the workload | unresolved | Resource lifetimes outlive their purpose: orphaned containers, a whole-run lock for an absent effect, accumulating intermediates and run directories. Parallel artifact builds are unproven. | F01, F02, F04, F05 |

**Bounded decision: Revise.** The plan's diagnosis is well grounded, and its direction is right. One synchronizer with readiness as observation, one fixture boundary under bare commands, transparent verify, a supported parallel route and lighter instructions all remove workarounds actually observed. P1, D5–D7 and P7 can proceed as written, subject to F07's label edits. P2–P5 need the contract corrections above before implementation.

**Enclosing architecture:** the harness is not assessed beyond this boundary. Test-family coverage (AE-12) remains with its owner.

| Priority | Change and component | Findings | Closure evidence |
|---|---|---|---|
| 1 | Fixture lifetimes, owner-liveness sweep, signal handling, port policy (`surrealdb_fixture.py`) | F01 | SIGKILL/orphan/kept/concurrent case; mcp across `restart()`; OOM reads as infrastructure |
| 2 | Shared/exclusive environment ownership; `just sync` reports holders (`verify.py`, `just sync`) | F02 | Sync during live serving waits and reports; two verify runs overlap |
| 3 | Explicit `--no-sync` launchers; per-runtime reach; scoped readiness | F03, F06 | Fresh Claude and Codex sessions show no rebuild; store family not blocked by a stale extension |
| 4 | Truthful `--print`/`--list`/`summary.json`; validated reuse | F05 | Reused-fixture run reports `not_run` with identity |
| 5 | Worktree claim, cost and prepare split; integration rule | F04 | Probe or restated claim; origin check from a main-started session |
| 6 | Labels and minor items | F07, F08 | Text edits |

On prerequisites: P2's lock mode (F02) must precede P4's announcement, and P3's port policy (F01) must precede P4's mcp reuse. Those follow the plan's own ordering.

**Next decision:** the operator confirms D1–D4 with F01's three fixture lifetimes and F02's shared/exclusive lock as part of their contracts. The coordinator then arranges the two settling probes (Docker port across restart; concurrent artifact builds on a shared build directory) before P3 and P5 are implemented.
