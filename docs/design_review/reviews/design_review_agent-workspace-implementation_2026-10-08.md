# Design review: agent workspace implementation

**Date:** 2026-10-08 · **Tier / purpose:** change / conformance · **Decision:** Revise (scoped; structure and environment guard accepted)

**Subject:** the integrated development harness, range `ec278be4..95297fe5` on `main` (HEAD `95297fe5`), harness files only. Baseline update: `d9375ba4` removed verify's `NEXTEST_TEST_THREADS` halving. Per the operator, the harness must never lower test parallelism, and any cap or throttle is a defect. A search of `scripts/*.py`, `justfile` and `.config/nextest.toml` on 2026-10-08 found no remaining cap.

**Disposition owner:** the [workspace plan](../plans/agent-workspace-effectiveness-plan_2026-10-07.md) §9. This review creates no other ledger.

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | `scripts/{harness,workspace_env,build_environment,surrealdb_fixture,runs,verify,worktree,freshness,maintenance}.py`; `justfile`; `.claude/settings.json`; `.codex/config.toml`; `python/lctx_semantics/pyproject.toml`; the nine matching `tests/scripts/test_*.py`. The four dirty product files (`crates/cpg-core/src/{artifact_manifest,facts,workspace}.rs`, `docs/design/sections/semantic-model.md`) were neither read for judgment nor touched. |
| Standard | Core principles and template 3.3; the [library-context binding](../design_principles/binding/library-context.md). The code-intelligence profile does **not apply**: nothing here extracts or serves code facts. |
| Contract | Plan §5 D1–D4, §5.8 execution contracts, §5.9 (OD1 approved, OD2 native binary); [ADR-0134](../../adr/0134-scoped-preparation-and-maintenance.md); the findings of the [first review](design_review_agent-workspace-effectiveness_2026-10-07.md) and the [capability review](design_review_agent-workspace-effectiveness-capabilities_2026-10-07.md), F01–F08 each. |
| Supported scope | One workstation (systemd 255, user manager), Claude Code and Codex sessions with subagents, a handful of concurrent verify runs, run-owned and kept native SurrealDB 3.3.0 fixtures, optional worktrees. |
| Change scenarios | (a) A worktree session that inherited main's absolute `UV_PROJECT_ENVIRONMENT` runs recipes, `sync` and `ready`. (b) `verify-serving --command mcp` on a run-owned fixture, including its restart. (c) A tool timeout SIGKILLs a foreground `just verify`. (d) `worktree-remove` on a worktree with a kept fixture. |
| Method and coverage | Static reading of every listed script, the justfile and the configs, the deleted `native_controls.py` and the old `verify.py` at `ec278be4`, and the fixture consumers (`crates/lctx-publisher/tests/publication.rs:494`, `crates/lctx-serving/tests/native_journey.rs:178–188, 704`, `crates/cpg-extract/tests/harness.rs:74`, `crates/lctx-model/src/domain/value/presentation.rs:236`, `crates/lctx/src/main.rs:256`). Probes and tests are listed in §3. **Not examined:** how Claude Code's TaskStop and Codex's timeout signal a process tree (this bounds F05); `freshness.py` beyond its command list; maturin's write path. No wide build, family run, `qualify` or `just sync` was run. |

## 2. Integrated assessment

The harness delivers the planned shape with ordinary functions and small records. It provides:
- scoped preparation routes with observing readiness;
- flock ownership;
- one boundary table that drives `--print`, `--list`, execution, `summary.json` and `--rerun`;
- generic run handles;
- a native-binary fixture with run-owned and kept lifetimes;
- optional worktrees;
- read-only freshness checks;
- scoped maintenance.

The prior reviews' structural findings are not reintroduced. In particular:
- readiness never synchronizes;
- `--print` builds nothing;
- outcomes and terminations are separate vocabularies;
- run-owned servers die with their launcher;
- ports are explicit and kept across restart;
- liveness crosses pid namespaces through the owner lock.

**Wrong-environment mutation holds on every syncing path.** Every syncing path drops a foreign absolute `UV_PROJECT_ENVIRONMENT`:
- the justfile shell (`build_environment.py:82–119`);
- `sync` and `observe` (`workspace_env.py:364–377`);
- `ready`'s selection (`workspace_env.py:458–478`);
- worktree preparation (`worktree.py:279–296`).

Shebang recipes (`ready`, `fmt`, `_bundle`, `doctor`, `doctor-check`) bypass the justfile shell, but none of them syncs: each uses `--no-sync` or `--no-project`. So the incident of 2026-10-07 cannot recur through a recipe. The commit's claim that the guard covers "every recipe" is true only because of that.

The other mutation paths are also clean:
- `--carry` reads the source with `GIT_OPTIONAL_LOCKS=0` and refuses environment and credential paths;
- `runs prune` selects only terminal, unretained records;
- the sweep is limited to the checkout's own records;
- scoped `fmt` touches only named files.

The material defects sit in **lifecycle edges and argument routing**:
1. **Run-owned systemd scopes never stop** (F01). As a result, a run-owned restart always fails, so `serving:mcp` can never pass. OOM evidence is lost, and each run-owned fixture leaks a transient unit.
2. **Family shortcuts misroute every option outside a five-item list** (F02), and `--cli` no longer exercises the CLI (F03). Both report coverage that did not happen.
3. **Several smaller edges:**
   - a server ending during a step is classified `blocked` without evidence (F04);
   - a SIGKILLed foreground launcher leaves its work running (F05);
   - `worktree-remove` can orphan a kept fixture (F06);
   - requirements are broader than what the boundaries import (F07).

None of these needs a new mechanism; each fix is local.

## 3. Verification run for this review (2026-10-08)

| Check | Outcome |
|---|---|
| `UV_NO_SYNC=1 uv run --no-sync pytest tests/scripts/test_{harness,workspace_env,build_environment,surrealdb_fixture,runs,verify,worktree,freshness,maintenance}.py -q` | **failed**: `test_cap_below_startup_rss_is_reported_as_oom_during_readiness` (reproduced 3/3: "blocked: readiness: server ended (signal 9)", not OOM). With it deselected, the remaining 98 **passed**. |
| `/usr/bin/python3 scripts/build_environment.py --explain` (3.12.3) | passed; `ast.parse` under 3.12 of every `scripts/*.py` fails only for 3.14-only scripts that are never run by the system Python |
| `just lint-agents` | passed |
| `just fixture -- true` | passed (exit 0), but left `lctx-fixture-<id>.scope` *active/running* with 0 processes (F01) |
| Run-owned `Server.create("run").restart()` under `LCTX_FIXTURES_ROOT` in the scratchpad | failed: `Unit lctx-fixture-<id>.scope was already loaded` (F01) |
| `just fixture --keep`, `--restart ID`, `--stop ID` | passed; no unit or record left |
| `just verify-store --cargo-config profile.release.debug=0 --print`; `just verify-serving --command mcp --attach abc --serving pilot --print`; `just verify-store --cli --print` | executed (static). They show F02 and F03. |
| `just verify --select compiler:producer --select serving:mcp --nextest-args "-E 'test(admit)'" --pytest-args "-k native_session" --print` (plan P4's acceptance text) | exit 2: arguments bind to the *preceding* selection (F02 note) |
| `just verify-compiler --command producer -- --test graph_artifact --print` | executed by mistake: `--print` after `--` passes through, as specified. nextest rejected the argument at parse time; run `20261008T040652.098Z-d47027` is left in `build/runs/`. |

Cleanup: these tests and probes leaked six empty fixture scopes (three OOM test runs and three probes). All six were stopped with `systemctl --user stop`. No `lctx-fixture-*` unit and no `build/fixtures/` record remains.

## 4. Findings

| ID | Severity | Finding |
|---|---|---|
| [F01](#f01) | High | Run-owned scopes are never stopped: restart always fails, OOM evidence is lost, units leak |
| [F02](#f02) | High | `verify-<family>` misroutes options to the primary tool |
| [F03](#f03) | Medium | `--cli` builds the CLI but never exercises it |
| [F04](#f04) | Medium | A server end during a step is `blocked` without evidence |
| [F05](#f05) | Medium | A SIGKILLed foreground launcher leaves verify, nextest and the fixture running |
| [F06](#f06) | Medium | `worktree-remove` deletes live fixture and run state and orphans kept units |
| [F07](#f07) | Medium-low | Python requirements do not follow what boundaries import |
| [F08](#f08) | Low | The ownership token never reaches verify's step children |
| [F09](#f09) | Low | Exclusive `sync` can starve behind overlapping shared holders |
| [F10](#f10) | Low | A record from another boot is treated as unknowable rather than dead |
| [F11](#f11) | Low | Fixture-setup errors and failed retention leave the run in a bad state |

### <a id="f01"></a>F01 — Run-owned systemd scopes are never stopped (High)

**Evidence.**
- A run-owned server is `systemd-run --user --scope --unit=lctx-fixture-<id>.scope -- surreal …` (`surrealdb_fixture.py:595–618`).
- On teardown, `stop_process` terminates the process, then `settled_result` waits up to 5 s for the unit to leave the active state, then calls `reset-failed` (`:723–747`). Nothing stops the scope.
- On this host's systemd 255, the emptied scope stays `active/running` with `Result=success`.
- After an OOM kill, the cgroup's `memory.events` shows `oom_kill 1`, but the unit's `Result` stays `success`. Read 2026-10-08 on three leaked scopes.

**Failure scenarios.**
1. **Restart.** `restart()` calls `stop_process()` and then `start()` with the same unit name (`:748–763`). systemd refuses ("already loaded"), the server exits 1, and `ready()` raises `FixtureBlocked("readiness")`. Every `serving:mcp` run on a run-owned fixture therefore ends `blocked` right after `native_journey` passes, and the Python wire and session suite never runs. P3's acceptance "the mcp path survives `restart()`" fails.
2. **OOM.** `exited()` (`:672–689`) relies on `Result=oom-kill`, which never appears. A 150M cap is reported as "server ended (signal 9)". The live test fails 3/3, and P3's commit claim "a 150M cap is blocked as OOM" does not reproduce. The "raise `LCTX_FIXTURE_MEMORY`" repair is never shown.
3. **Leak.** Every run-owned fixture leaves one empty transient unit and costs about 5 s in `settled_result`. `--list` shows these units as `unrecorded`, and the sweep never touches them.

**Fix.**
- After the process exits, read `oom_kill` from `memory.events` in the scope's `ControlGroup` (`systemctl show -p ControlGroup`) as the OOM evidence.
- Then call `systemctl --user stop` on the scope in `stop_process`, `destroy` and the sweep. With the unit stopped, restart reuses the name.
- Add two live tests:
  - after `just fixture -- true`, no `lctx-fixture-*` unit remains;
  - a run-owned `restart()` succeeds on the same port.
- Keep the existing OOM test.

**Closes when** those three live tests pass.

### <a id="f02"></a>F02 — `verify-<family>` misroutes options (High)

**Evidence.** `legacy()` passes only `--print`, `--list`, `--live`, `--json` and `--cli` to the plan parser. Every other item before `--` becomes a tool argument (`verify.py:1187–1232`, the set at `:1213`). Observed:
- `just verify-store --cargo-config profile.release.debug=0 --print` plans `cargo nextest run … --cargo-config profile.release.debug=0`;
- `just verify-serving --command mcp --attach abc --serving pilot --print` plans a run-owned fixture, keeps the journey, and appends `--attach abc --serving pilot` to pytest.

`--rerun`, `--retain-serving`, `--select` and `--nextest-args` are also misrouted.

**Failure scenario.** An agent asks a shortcut to reuse retained serving content. It instead gets a fresh run-owned journey and then a pytest usage error, recorded as `failed` against the product. A `--cargo-config` override silently becomes a nextest error. The P4.2 commit says "--cli and --cargo-config are carried over"; that holds for `just verify` but not for the shortcuts that the paused product plan consumes.

**Fix.**
- Parse everything before `--` with the main parser (`parse_known_args`). Pass through only what follows `--`, which is the documented `verify-<family> [--command B] [-- ARGS]` contract.
- An unknown option before `--` becomes a usage error.

**Related contract mismatch.** Plan P4's acceptance example puts both tool arguments after the second `--select`. The implementation correctly binds them to `serving:mcp`, so the example fails with exit 2 (§3). Correct the example in the plan, not the code.

**Closes when** each of the probes above shows its option reaching the plan parser.

### <a id="f03"></a>F03 — `--cli` builds the CLI but never exercises it (Medium)

**Evidence.**
- `store:rust`'s `build-cli` step is enabled by `--cli`, and its nextest step has `env=()` (`verify.py:190–202`).
- The deleted `native_controls.py` set `LCTX_REMEDIATION_CLI_BIN` after the build.
- `publication.rs:494` falls back to `config.select(&handle)` when that variable is absent.

**Failure scenario.** `just verify-store --cli` passes after compiling `lctx` without ever running its selection CLI. Its help says "build and exercise the native CLI".

**Fix.** Give the nextest step `("LCTX_REMEDIATION_CLI_BIN", "{release}/lctx")` when `--cli` is set: either a `when`-gated env entry or an option-dependent expansion. Add a fake-runtime test asserting the variable.

### <a id="f04"></a>F04 — A server end during a step is `blocked` without evidence (Medium)

**Evidence.** After every step, `_server_end` maps any server exit to "fixture readiness: … ended", and `_run_steps` returns `blocked` (`verify.py:713–722, 847–850`).

**Contract.** Plan §5.8.5 admits OOM, a failed readiness observation or launch error 127 as the only infrastructure evidence.

**Failure scenario.** A store or serving change that crashes or shuts down SurrealDB, which is a product defect, is reported as `blocked` and reads as environment-shaped. Until F01 is fixed, real OOMs land in this same bucket.

**Fix.** Return `blocked` only for `oom-kill` (with F01's evidence). Otherwise return `failed`, keeping the server's end in the step record.

### <a id="f05"></a>F05 — A SIGKILLed foreground launcher leaves the work running (Medium)

**Evidence.**
- `Owner.run` starts the command with `spawn_group`, that is, a new session, and with no death tie (`runs.py:331`).
- Every `just verify` re-executes itself under a foreground run (`verify.py:1299–1306, 1343`).
- The fixture already ties its children with `PR_SET_PDEATHSIG` (`surrealdb_fixture.py:334–366`); the run handle does not.

**Failure scenario.** A tool-call timeout or a terminal SIGKILLs the launcher's process group. verify, nextest (all logical CPUs by default) and its run-owned server sit in another session and keep running to completion. `just runs` shows the run as `interrupted` with survivors. D4 puts each runtime's own cancellation first, and SIGKILL of a group no longer reaches the work.

The magnitude depends on how each runtime kills a process tree, which was not examined. A tree walk by parent pid would still reach the work; a group kill would not.

**Fix.** Spawn the foreground child with `PR_SET_PDEATHSIG(SIGTERM)` and keep the new session for group signalling. Move `spawn_tied` into `harness.py` so both owners share it. Background runs are unchanged, since their launcher is detached by design.

### <a id="f06"></a>F06 — `worktree-remove` deletes live fixture and run state (Medium)

**Evidence.** `inspect` reports dirty files, unintegrated commits and managed environment holders only (`worktree.py:309–328`). `git worktree remove` (`:358`) deletes ignored files, so `build/fixtures` and `build/runs` go even without `--force`.

**Failure scenario.** A worktree holds a kept fixture (`lctx-fixture-<id>.service`, data under `<wt>/build/fixtures/<id>/data`) or a running background run. Removal deletes the data directory, `server.env` and the records. The service keeps running with no record. It is visible only as `unrecorded` from another checkout's `--list`, and no sweep ever stops it. A background run loses its record and log while its command continues.

**Fix.** Have `inspect` also report:
- live kept or run-owned fixtures from `<wt>/build/fixtures/*/record.json`;
- units whose description names `checkout=<wt>`;
- runs whose derived state is `running`.

Refuse on any of these. With `--force`, stop kept fixtures before removing the tree.

### <a id="f07"></a>F07 — Python requirements do not follow what boundaries import (Medium-low)

**Evidence.**
- `compiler:producer`, `compiler:cli`, `store:rust` and `serving:rust` declare `tools` (`STORE`/`SERVING`, `verify.py:106–107`). They inherited it from the old family table.
- A search of their crates found no use of the shared environment. `lctx` runs `uv` only for library projects, and with `UV_*` cleared (`main.rs:256–270`).
- `providers:extract` does use it (`cpg-extract/tests/harness.rs:74`, `uv run --no-sync pyrefly`).
- `model:rust` runs `uv run --no-sync python -I` (`presentation.rs:236`) yet declares nothing.

**Contract.** D1 says "a boundary checks only what it imports", and "pure Rust commands do not borrow the environment."

**Failure scenario.**
- After an unrelated dev-group drift, the store and compiler boundaries report `blocked: run just sync tools`.
- Those boundaries hold shared ownership through long release builds, which delays `just sync`.
- `model:rust` imports the environment unobserved and unowned.

**Fix.**
- Keep `tools` for `providers:extract`, `oracles`, `tooling:python` and the leaves.
- Give `model:rust` the interpreter requirement (`tools`).
- Drop it elsewhere.

### <a id="f08"></a>F08 — The ownership token never reaches step children (Low)

**Evidence.**
- Step environments start from `runtime.base_env` (`verify.py:806`). It is computed before `ownership()` sets `LCTX_ENV_OWNERSHIP` in `os.environ`, and `Ownership.environment` is never called in verify.
- `--print` nevertheless lists the variable (`verify.py:998`).

**Consequence.** Nested managed operations re-acquire instead of reusing, against §5.8.4's "one acquisition". A nested exclusive request would wait on its own ancestor instead of raising `OwnershipConflict`.

**Fix.** Make `runtime.owner` yield the `Ownership` and apply `owned.environment(env)` to every step.

### <a id="f09"></a>F09 — Exclusive `sync` can starve (Low)

**Evidence.** `_acquire` polls `flock(LOCK_NB)` (`workspace_env.py:218–243`). flock gives no writer preference.

**Failure scenario.** While several agents run overlapping verify boundaries, a new shared holder always gets in, so `just sync native` waits without bound. It does report the holders.

**Fix (proportionate).** Use a gate lock: an exclusive waiter holds a gate file `LOCK_EX`, and shared acquirers take the gate `LOCK_SH` briefly before the resource. Alternatively, document the behaviour.

### <a id="f10"></a>F10 — A record from another boot is treated as unknowable (Low)

**Evidence.** `ProcessIdentity.foreign()` returns true for another boot id as well as another pid namespace (`harness.py:58–60`). The sweep never touches foreign records (`surrealdb_fixture.py:1239, 1255`). `holders()` keeps foreign records (`workspace_env.py:159–161`).

**Consequence.**
- Run-owned fixture directories and kept-fixture attachments recorded before a reboot are never swept, and their RocksDB data accumulate under `build/fixtures`.
- Holder records accumulate in the `~/.cache` fallback.

Another boot id proves the process is dead; only another pid namespace is genuinely unknowable.

**Fix.** Treat another boot as dead, and keep `foreign()` for another namespace in the same boot.

### <a id="f11"></a>F11 — Setup errors and failed retention leave a bad state (Low)

**Evidence.**
- `run_boundary` re-raises any exception without `kind`/`detail` (`verify.py:778–784`). A `URLError` or `RuntimeError` from `Attachment.create`'s namespace definition (`surrealdb_fixture.py:893–897`) or from `serving_available` therefore aborts the whole run with a traceback. The remaining boundaries are missing from `summary.json`, and `termination` stays null.
- When `--retain-serving` fails in verify, the `serving/<name>` directory created by `retain_serving` (`:968–976`, called from `verify.py:820`) is left behind. The command-line path removes it (`surrealdb_fixture.py` `main`). A later `--retain-serving NAME` is then refused as "already retained".

**Fix.**
- Wrap setup errors as `FixtureBlocked("readiness", …)` at the fixture boundary.
- Move retention cleanup into an `Attachment` method that both callers use.

## 5. Proportionality

The size is concentrated in three scripts. Most of it serves a planned capability, but the parts below have no consumer, or re-implement a facility the kernel or Cargo already provides. They can go without losing anything §5 plans.

| Cut | Where | Why it can go |
|---|---|---|
| Holder-record subsystem (about 90 lines) | `workspace_env.py:122–166` and `:537–551`, plus the record writes in `_acquire`/`_release` | The kernel already lists flock holders in `/proc/locks`, matched by the lock file's inode, and `/proc/<pid>/cmdline` gives the command. The "waiting for…" report would no longer depend on self-written records, their cleanup, or F10's stale files. |
| Identity-plus-lock liveness | `harness.py`, `runs.owner_alive`, `surrealdb_fixture.owner_alive` | Every owner already holds a close-on-exec flock. The lock alone answers liveness across namespaces and reboots. Keep `ProcessIdentity` only to guard signalling. This also removes F10. |
| Cargo target re-discovery (about 125 lines) | `verify.py:333–459` | It re-implements Cargo's auto-discovery and misses `autotests = false`, custom `path` and `required-features`. `cargo metadata --no-deps --offline` is static and builds nothing. Or drop the narrowing and let Cargo's own error name the package. |
| Unconsumed fixture API (about 70 lines) | `Attachment.export`, `import_dump`, `run`, `query`, `ready` (`surrealdb_fixture.py:923–966`); `_record_in_run` (`:1125–1143`), whose `$LCTX_RUN_DIR/fixtures/*.json` nothing reads | No consumer in `scripts/`, `tests/` or `python/` (search of 2026-10-08). |
| Unused CLI verbs | `workspace_env.py hold`, `observe`, `identity` | No recipe or documentation uses them. Keep `holders`. Keep `hold` only if it is documented as the way for a bare command to join managed ownership. |
| Three readiness shapes | `workspace_env.Readiness`, `surrealdb_fixture.Readiness`, `verify.Observation` | They are one record adapted twice; a single dataclass in `harness.py` suffices. |
| A second list of fixture variables | `verify.ENVIRONMENT_NAMES` (`verify.py:986–989`) beside `Attachment.environment` | It duplicates the fixture's variable set and contradicts "no second catalogue". Export one constant from the fixture. |
| Unused input comparison | `source_inputs()` runs a whole-tree `git diff HEAD --binary` twice per reuse; `current_inputs` is recorded but never compared | Record it once, or compare it. |

**Kept as justified:**
- process-group ownership;
- the run record and its derived state;
- the run-owned and kept fixture lifetimes;
- flock ownership;
- the single boundary table;
- the retain marker that prune honours.

## 6. Judgment

- **A1 (ownership).** Satisfied: one owner per resource, with records written only by their owner.
- **A2 (contracts).** Violated by F02, F03 and F04: the selection and outcome contracts misreport coverage.
- **A3 (change locality).** Satisfied: adding a boundary changes only its definition (P4.2 test).
- **A4 (lifecycle).** Violated by F01 and F06, and unresolved for F05.

**Decision: Revise, scoped.** The environment-ownership guard, the plan structure and the run vocabulary are accepted as implemented at *Tested* strength for the cases in §3. F01–F03 should be fixed before `serving:mcp`, the `verify-<family>` shortcuts or `--cli` are relied on by the paused product work. F04–F11 and the §5 cuts can follow in ordinary harness maintenance.
