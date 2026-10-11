# Inquiry F2: Verification, evidence and environment tooling

**Provenance.** This is the verbatim final report of supporting inquiry F2, from one read-only
`design-reviewer` agent, for the [codebase efficiency-alignment review](../../reviews/design_review_efficiency-alignment-codebase_2026-10-11.md).
It was produced on 2026-10-11 at baseline `88f33bc0`, against the [inquiry briefs](inquiry-briefs.md).
Only this header was added.

How to read it:
- Candidate labels such as `F2-01` are local to this report. They are not the review's finding
  IDs.
- Its judgments are bounded and supporting. The principal review owns the combined decision.
- See the [evidence index](README.md) for coordinator checks and reconciliation.

---

# F2: verification, evidence and environment tooling (supporting assessment)

**Inquiry:** F2, a bounded supporting assessment for the codebase efficiency-alignment review (DESIGN/TARGET tier). The local candidate labels `F2-nn` are not finding IDs.

**Baseline:** `main` at `88f33bc0`, 2026-10-11. I read HEAD content for the four files with another session's uncommitted edits; none of those files is in this inquiry's scope.

**Standard:** core 3.3, heuristics 1.0, code-intelligence profile 1.5, and the library-context binding (Q17).

## 1. Scope examined and coverage limits

**Read in full or at the decisive symbols:**
- `scripts/verify.py`: `Step`, `Boundary`, `narrow`, `restricted_arguments`, `step_argv`, `observe_requirements`, `environment_owner`, `run_boundary`, `_run_boundary`, `_run_steps`, `execute`, `launch`, `real_fixture`, `legacy`, `rerun_selection`, `wrap_in_run`, `main`.
- `scripts/verify_evidence/capture.py` (`CaptureOwner`, `Invocation.command_finished`, `optional_owner`).
- `scripts/verify_evidence/context.py` (`SourceObserver`).
- `scripts/verify_evidence/nextest.py` (`prepare`, `finalize`).
- `scripts/verify_evidence/rerun.py::nextest_case`.
- `scripts/verify_evidence/cli.py` (`compare`, `retention`, and the invocation listing).
- `scripts/surrealdb_service.py`: `_installed_binary`, `verify_binary`, `_owned_server_generation`, `_descriptor`, `Installation.load`, `check_daemon`, `check`, `borrow`, `sql`.
- `scripts/surrealdb_server.py::validate_descriptor`, `_validate_descriptor`, `execute`.
- `scripts/surrealdb_fixture.py`: `substrate_readiness`, `_server_identity`, `Server.open`, `Attachment.create`, `Attachment.release`, `run_attached`, `fixture`, `recover`.
- `scripts/storage_lifecycle.py`: `admission`, `Storage.__init__`, `metadata`, `records`, `validate`, `save`, `publish`, `retain`, `complete`, `trim_metadata`.
- `scripts/storage_owners.py::enroll`, `complete`.
- `scripts/runs.py`: `Owner.run`, `_run_owned`, `_own`, `cancel`, `cmd_logs`.
- `scripts/harness.py::tail_bytes`.
- `scripts/build_environment.py` (whole file).
- `scripts/workspace_env.py::observe`, `sync`, `uv_environment`, `sync_command`, `resources_for`.
- `scripts/storage_acquisition.py::prune_finished_leases`.
- `scripts/test_scope_coverage.py`: work surface only.
- The justfile header and the verify recipes.
- `.config/nextest.toml`, `.cargo/config.toml`.
- ADR-0134, ADR-0142 and ADR-0144 (decision sections), ADR-0147 (whole).
- The relevant passages of `docs/surrealdb.md` and README.md.

**Records inspected read-only:**
- `build/runs/*/summary.json`, boundary logs, `invocation.json`, `nextest-events.jsonl` and `junit.xml` for:
  - run `20261011T071240.860Z-5e33c9`, the only run with ADR-0147 capture;
  - the only complete qualify run, `20261008T225907.834Z-dd7ee2`;
  - five focused `store:rust` runs from 2026-10-10.
- `~/.local/state/library-context/surrealdb/installation.json`, binary block only, no credentials read.
- The attachment records (status counts only).
- The counts in `~/.local/state/library-context-storage`.

**Not read:**
- `compile_profile*.py` beyond retention observations;
- `worktree.py`, `freshness.py`, `maintenance.py`;
- `storage_archive.py`, `storage.py` (CLI);
- `pytest_adapter.py`, `projection.py` beyond the fact that it is lazy;
- service install, upgrade, backup and handoff paths in `surrealdb_service.py`. These are D-adjacent; I noted only their size.
- `docs/workspace-storage.md`, apart from the policy statements quoted in ADRs.

I ran no builds, tests, `just` recipes, services or SurrealDB queries. My only commands were `cargo nextest --version` and `cargo nextest run --help`, both in `/tmp`.

## 2. Complete operations examined

### Where the minutes go (decisive calibration)

**Focused S10 loop.** Five focused `store:rust` runs from 2026-10-10 (historical receipts, pre-capture) each ran 1 to 2 tests (one ran 4). Their timings:

| Measure | Range |
|---|---|
| Boundary total | 35.7–380.8 s |
| Cargo `Finished … in` | 33.7 s – 5 m 54 s |
| Nextest suite | 1.0–24.2 s |
| Outside the step | 0.6–1.1 s |

- Runs: `20261010T054722…`, `…080622…`, `…072635…`, `…073403…`, `…062356…`.
- In the focused loop, release-profile compilation of the boundary's test targets dominates. The harness's fixed cost is around 1 s.

**Assembled runs.**
- The complete qualify of 2026-10-08 took 6,119 s, all boundaries running sequentially.
- `compiler:producer` took 1,972 s, of which cargo was 2 m 33 s. Its suite took 1,816 s with 168 of 326 tests timed out.
- `serving:mcp` took 2,007 s.
- In the capture-bearing run of 2026-10-11, `model:rust` rebuilt 12 units: the `lctx_model` lib plus test crates. This accounts for most of its 125.9 s outside the suite.

**Correction to P4.** In run `20261011T071240`, 32 of the 76 `providers:extract` JUnit cases are `failure type="test abort" message="process aborted with signal 15 (SIGTERM)"`. The run's `termination` is `interrupted`, and `providers:extract` is `not_run: interrupted`. So the uniform ~280.8 s durations are the moment of cancellation, not completed per-process preparation. The data therefore does not support P4's "repeated per-process `OnceLock` preparation" hypothesis. It does show that at least 32 tests had each been in flight for at least 280 s.

### Fixture-backed boundary

1. `just` → `uv` → `python build_environment.py` → exec bash → `uv` → `python verify.py` → execv `runs.py` → Popen `verify.py` #2.
2. `_run_boundary`:
   - `runtime.owner` takes no Python ownership for `native-store`.
   - `observe_requirements` → `surrealdb_fixture.substrate_readiness`, which takes no borrow.
   - `real_fixture` → `Server.open` → `Attachment.create` → `service.borrow` (shared admission gate plus shared `borrowers.lock`) → `Installation.check`.
   - `_run_steps` → `CaptureOwner.start` (source context) → nextest → `command_finished` (store export, context, binary digests) → `Attachment.release`.
3. `runs.Owner.run` finalization: `storage_owners.complete` twice, then `Storage().records()`.

**Live state:** small JSON records only. Durable writes are listed in operation-map §4.5.

**Crossings per boundary:**
- about 6 `surreal version` children;
- 4 HTTP `/sql` calls;
- 2 `/ready` and 2 `/version` calls;
- `systemctl` 2 plus 1 per step;
- `git ls-files` and `git rev-parse` twice per step;
- 6 nextest subprocesses: `--version`, then the `store` operations `list`, `info`, `export`, `info` on the zip, and `export-chrome-trace`;
- 1 `config_helper.py` child.

The installed service uses an **owned generation binary**: `installation.json` `binary.generation` is present and `path` is under `surrealdb.tools/surreal/405f6c…/surreal`. This settles operation-map §5 item 5: the six-pass path applies.

### Storage lifecycle on the hot path

- `Storage.publish` (Interface-checked) runs under `admission([path])` and `metadata()`, both exclusive flocks.
- Its lookups are bounded: the `paths/` index, plus one `_path_row` per ancestor.
- It makes 2–3 `durable_json` writes. The critical section is: validate, up to 3 file+directory fsyncs, and a handful of small reads.
- Ordinary publication scans no history. The scans are in finalization, cancel, recover, sweep and trim (see F2-03).

## 3. Candidate findings

### F2-01: A fixture-backed boundary proves the installed server identity up to six times, before and after taking the borrow that protects it

**Observed cause** (Interface-checked):
- With an owned generation, every call to `surrealdb_service._installed_binary` → `_owned_server_generation` → `surrealdb_server.validate_descriptor` does three things:
  - takes an admission flock;
  - hashes the full 368.7 MB executable (`storage_service.digest`);
  - spawns `surreal version` under `spawn_group`, polled every 50 ms with group-cleanup confirmation.
- `Installation.load` → `_descriptor` performs the same validation.
- Per boundary, the call sites are:
  - `substrate_readiness`: `Installation.load` (1), `check` → `check_daemon` (2), `_server_identity` (3);
  - `Server.open` → `Installation.load` (4);
  - `Attachment.create` → `borrow` → `check` → `check_daemon` (5), then `_server_identity` (6).
- `_server_identity` re-proves immediately what `check_daemon` proved moments before.
- `Installation.check` runs twice per boundary, each time querying `native_installation:current` in **both** `main` and `validation`. The borrower attaches only to `validation` (`Attachment.create` config `"database": "validation"`).
- `just fixture --` (`fixture()`) performs 3 of these passes; `verify` performs 6.

**Premise owner.**
- The binary is mode 0o500 under an owned tools root, and its descriptor is 0o400.
- A replacement happens only through `handoff_server` or maintenance, which close admission exclusively, while a borrower holds shared `borrowers.lock` until `release`.
- `/proc/<pid>/exe` resolution plus the `/version` HTTP check bind the running daemon to the path.
- So after `borrow` succeeds, the premise cannot change through any managed route for the rest of the boundary.
- The pre-borrow readiness check (`substrate_readiness`) proves a property *before* the ownership that protects it. ADR-0142's own rule binds readiness to effective ownership, and the borrow-time check already satisfies it: `Attachment.create` raises `FixtureBlocked` with a repair, and `verify._fixture_blocked` already maps that to `blocked`.

**Consequence.** Under S10 this is a fixed per-boundary cost (H18–H20, H12):
- 6 × 0.16 s of hashing (P1, Measured for one pass);
- plus 6 version children of unmeasured cost;
- plus 4 SQL round trips.

P3 bounds all fixed fixture overhead at a median of 1.1–2.8 s. That is small next to multi-minute steps, but material in a warm `just fixture --` loop where the step itself runs for seconds. Under concurrent agents, every boundary also re-reads the 368 MB file.

**Simpler conforming alternative.**
- Prove identity once, inside `borrow`, and carry the validated `Installation` into `_server_identity` and `Attachment.create` rather than re-deriving it.
- Make `substrate_readiness` in execution mode either the borrow itself or a descriptor-only observation. Keep a full check for `--print` and `just service check`.
- Query only the borrowed database's marker.
- For the executable, keep one full digest per service-start epoch, or per verify invocation, keyed by the daemon's `/proc/<pid>/exe` stat identity `(dev, ino, size, mtime_ns, ctime_ns)`. The capture code already accepts this premise within an invocation (`SourceObserver`, `CaptureOwner.binary_cache`).
- The `surreal version` child adds nothing beyond the digest, which pins bytes whose version was validated when the generation was built, together with `/version` from the running daemon.

**Preservation.**
- Identity must still be established before any administrative query (`check_daemon`'s docstring).
- Maintenance exclusion and the refusal of a running binary that differs from the installation must remain.
- Safeguard 3: a stat tuple is not semantic equivalence. Here, though, the property is "the same file object whose bytes were digested". An in-place rewrite changes ctime, which a user cannot forge. Deliberate same-user tampering within one boundary is the residual that this trades away. If that matters, keep one digest per boundary rather than six.

**Uncertainty:** the cost of a `surreal version` start for a 368 MB binary is unmeasured (probe PR-1).

**Evidence label:** Interface-checked for the structure; Measured (P1) for the per-pass hash.

**Closure evidence:**
- a fixture-backed boundary performs at most one executable digest and no version child after borrow;
- a replaced or modified executable is still refused, shown by a control that mutates or swaps a test generation.

### F2-02: Evidence capture re-hashes every built test executable after every step, at gigabyte scale, for a digest with no comparing consumer

**Observed cause** (Interface-checked): `verify_evidence/capture.py::Invocation.command_finished` reads `nextest-events.jsonl` or `cargo.jsonl` and SHA-256s every `compiler-artifact` with an `executable`. Its stat-keyed cache, `CaptureOwner.binary_cache`, lives only for one `CaptureOwner`, which is one verify invocation.

In run `20261011T071240`:
- `model:rust` listed 87 executables, 6.75 GB at current sizes.
- `providers:extract` listed 18, 4.24 GB.
- `analytics:rust` listed 5, 0.01 GB.
- Most were `fresh: true`. For `model:rust`, 255 artifacts were fresh and 12 rebuilt.

**Consumers.** `cli.compare` diffs source contexts, argv, settings, fixture and config, not `binaries`. Binary digests are only echoed by the invocation listing (`cli.py` key list). ADR-0147, the README and the evidence plan say explicitly that the digest establishes neither source closure nor executed bytes.

**Consequence** (S10, H26/H14):

| Boundary | Total | Step | Outside the step |
|---|---|---|---|
| `model:rust` | 133.225 s | 128.56 s | 4.67 s |
| `analytics:rust` | — | — | 0.33 s |
| `providers:extract` | — | — | 5.13 s |

- P3's pre-capture median outside the step for `model:rust` was 0.0 s.
- `analytics:rust` comes from the same run and has the same store-export and context work, but only 0.01 GB of binaries.
- **Inference** from these historical receipts: about 4.3 s for `model:rust` scales with executable bytes.
- Large focused selections (`--select model:rust --nextest-args …`) pay this after every run, even when nothing was rebuilt.
- The cost grows with the number and size of test binaries, which F1 owns.

**Simpler conforming alternative.**
- Record cargo's artifact identity (path, `fresh`, package and target, stat identity) at completion.
- Compute the SHA-256 lazily, only when an export or citation (`export`, `--consumer citation:…`) actually needs bytes, or when cargo reports `fresh: false`.
- Alternatively, keep a host-level stat-keyed digest cache across invocations.
- Either way, the observation keeps its current honest label of "artifact bytes at completion, if computed".

**Preservation.**
- `observation: "changed"` must still be detected, which the stat-before/after check already does without a digest.
- An exported bundle that claims artifact bytes must digest at export.
- Counterexample check: a rebuilt binary at the same path gets a new inode or mtime, so a stat-keyed cache cannot carry a stale digest across a rebuild.

**Uncertainty:** cold-cache cost. Attribution by difference is not a measurement.

**Evidence label:** Interface-checked; historical receipt (2026-10-11 run, mixed load).

**Closure:** a warm repeated focused selection with unchanged binaries performs no full executable digest, and export still produces verified digests.

**Candidate rule impact:** RC-1 (§7).

### F2-03: Ordinary owner-to-object lookups scan and validate the whole host storage registry, despite existing bounded indexes

**Observed cause:** these paths all iterate `Storage().records()`, which globs every `objects/*.json` and calls `validate` on each row:
- `runs.Owner.run` finalization, on every verify run that has `verify-evidence.json`;
- `runs.cancel`;
- `surrealdb_fixture.recover`;
- `verify_evidence/cli.py::retention` for bundles.

Each scan finds objects whose paths or IDs are already known:
- `CaptureOwner` already holds both component IDs in `manifest["storage"]` (`raw`, `receipt`);
- `Storage._path_row` resolves a path in O(1) through `paths/<sha256>.json`.

**Consequence:**
- **Cost** (H25, S08): 700 objects parse in about 6 ms (P2, Measured), linear in retained descriptors including tombstones. That is negligible today.
- **Failure coupling.** `records()` raises on the *first* invalid descriptor anywhere on the host: one written by another checkout, an older schema, or a partially written file. `Owner.run` catches the error, prints "verification completion unresolved; content kept", and leaves that run's evidence without completion obligations. A single bad unrelated descriptor would therefore keep every subsequent run's raw evidence protected from expiry. That is retention growth caused by state the run never touched.
- I observed 54 historical "completion unresolved (Blocked)" messages (background launches, 2026-10-10 to 2026-10-11 00:57), none since. They had a different cause, but they show the swallowed-failure path is exercised.

**Alternative:**
- Complete by the IDs in the manifest (`Storage.get`, already a bounded lookup).
- Use `_path_row` for the cancel and recover paths.
- Reserve whole-registry scans for `sweep`, `plan`, `trim_metadata` and inventory.

**Preservation:** keep "complete only fixed, already-enrolled components after confirmed cleanup" and the refusal to adopt legacy scopes. Both hold with ID-based lookup.

**Uncertainty:** none material. Priority is low.

**Label:** Interface-checked; P2 Measured.

**Closure:** with one corrupt foreign descriptor present, a verify run still completes its own evidence components.

### F2-04: Exact test identity narrows execution but never the build; a focused or rerun selection still builds every test binary of the boundary's packages

**Observed cause:**
- `verify.narrow` narrows packages and targets only when the user passes cargo target options (`--test`, `--lib`, …).
- Filter-only selections (`-E …`) keep `-p PKG` with default targets.
- `verify_evidence/rerun.py::nextest_case` knows the exact `binary_id` (for example `lctx-surrealdb::native`), but emits only `-E binary_id(…) & test(…)` while preserving the original `-p` list. It never adds `--test native` or `--lib`, so the printed exact rerun still builds every target of `lctx-surrealdb` and `lctx-publisher`.
- For `store:rust` that is 11 integration binaries plus the library harnesses; for `model:rust` it is 87 binaries.

**Consequence** (S10; H5/H6, A4): in the focused loop, cargo time (34 s to 5 m 54 s) dwarfs the suite (1–24 s), as the 2026-10-10 receipts show. When a change touches only one test target, building and release-linking the boundary's other test binaries is avoidable. When the library itself changes, the library rebuild dominates either way. The narrowed share is unmeasured.

**Alternative:** project an exact `binary_id` into cargo target selection: `package::name` → `-p package --test name`, and `package` → `--lib`.
- Apply it in `rerun.nextest_case`.
- Optionally apply it in `restricted_arguments` when every user expression is binary-qualified.
- The existing `target_filters` handling already keeps `required_filter` consistent when targets narrow.
- This is library-owned: nextest builds exactly the cargo targets it is given.

**Preservation:**
- A name-only filter (`-E test(x)`) cannot be narrowed without listing, which builds everything. Leave it as is; that is the counterexample.
- Keep the boundary's `required_filter` intersection, so a narrowed selection never admits tests outside the boundary.
- Coverage claims stay with boundary runs; a narrowed run is diagnostic.

**Uncertainty:** the saved compile and link share per case is unmeasured. The test-target structure and the release default are F1's.

**Label:** Interface-checked; historical receipts for the focused-loop split.

**Closure:** `rerun-plan … --print` emits `--test <binary>` (or `--lib`), and a run of that argv shows only the named test target in `cargo.jsonl` or `nextest-events` as compiled.

### F2-05: Rerun repeats whole boundaries while the captured native recording supports failed-only reruns

**Observed cause:**
- `verify.rerun_selection` re-selects each non-passed boundary in full.
- The capture already exports `nextest-recording.zip` per invocation (`nextest.finalize`).
- Nextest 0.9.146's `cargo nextest run -R/--rerun <RUN_ID_OR_RECORDING>` "rerun[s] tests that failed or didn't complete in a previous recorded run" (installed `--help`, read 2026-10-11).
- pytest's cache provider (`--lf`) is the analogue for pytest boundaries.

**Consequence** (S10, S07; H24): after an interrupted `test-all`, such as the run above, `--rerun` repeats `providers:extract` in full plus everything not started. A single failing case repeats `store:rust`, whose step median is 273 s (P3).

**Alternative:** a distinct `--rerun-incomplete` diagnostic operation that passes the recorded zip to nextest `-R`, and `--lf` to pytest, with the fixture re-attached.

**Preservation:** keep `--rerun` as the boundary-level acceptance operation, because rerunning only the failed tests cannot show that a fix did not break passing ones. Keep the outcomes of the original run.

**Uncertainty:** I could not settle whether `-R` needs `NEXTEST_EXPERIMENTAL_RECORD=1` at 0.9.146, because the source is not in the registry. Priority is low, since `rerun-plan` already gives exact single cases.

**Label:** Interface-checked from CLI help.

**Closure:** after a recorded run with N failed or incomplete tests, the diagnostic rerun executes exactly those N, plus new tests, without changing the original run's outcomes.

## 4. Opportunities and uncertainties that are not findings

**`test-all` and `qualify` ordering (H22).** `verify.execute` runs boundaries sequentially. Each nextest step already uses nextest's default parallelism, and the fixture boundaries share one service, so running boundaries concurrently would mostly add contention. Not a finding. A unified up-front build of all test targets (`cargo nextest archive`, or one cargo invocation) might overlap cross-crate compilation; that belongs to F1. A related seam for F1: in the 2026-10-11 run, `analytics:rust` built second, non-fresh copies of `petgraph`, `fixedbitset` and `aws_lc_sys` with different feature sets from `model:rust`'s artifacts, despite `feature-unification = "workspace"`.

**Fixture concurrency on the shared service (S10).**
- Logical attempts share one `validation` database, in which every native effect writes `native_installation:current` (operation map M1).
- Recent logs show SurrealDB 10 s transaction timeouts, and nextest 300 s timeouts in qualify (168/326) and in focused runs.
- Single-test runs also timed out, so I cannot attribute the timeouts to concurrency. The hot-row mechanism belongs to D and H.
- The harness has no retirement of validation attempts (`Attachment.release` only writes a record), and whether the store retires them is D's question. The `surrealdb/data` directory is 3.0 GB. Validation-database growth under S08 × S10 is unobserved by the harness.
- Any remedy that admits fewer concurrent fixture tests conflicts with RC-2 (§7).

**Eagerly derived and duplicated evidence (H26/H6), small.**
- Each nextest step eagerly runs `export-chrome-trace` (1.2 MB for `model:rust`). No script consumes it.
- Each step keeps the private `nextest-state` store (680 KB) beside the exported zip (666 KB) of the same recording.
- About 3–4 MB per step, so not material.

**Source-context hashing.** About 2,295 allowed files (128 MB) are hashed once per verify invocation, with stat reuse within it. That is proportionate to `compare`'s consumer. A cross-invocation stat cache is optional.

**Process chain.** Each `just verify` starts 2 `uv` and 3 Python processes, and 1 per recipe line through `build_environment.py`. This is likely well under a second (probe PR-3). Adequate.

**Admission lock files.** `state/admission/*.lock` (1,244) are never removed. Lookups are by hash, so only inode growth results. Deleting flock files safely is racy, so keeping them is defensible.

**`trim_metadata` growth.** It contains O(n²) scans (`any(child.get("parent")…for child in rows)` and the owner-path scan) and per-row substring scans over reference documents. It runs only on sweep or trim. Revisit at about 10⁴ objects.

**Retained diagnostics.** `build/runs` is 9.1 GB, dominated by compile-profile raw output and reports plus one 1.2 GB `output.log`. Run `20261008T181813` holds two identical 586 MB `chrome_profiler.json` copies, from an old `assessment/` layout that current code no longer writes. They are retained under the `retained-compiler-reports` obligation, which has no `until`. This is an explicit hold, not a defect.

**Tooling size (H27).** About 26.5k lines in total:
- `surrealdb_service.py` (2,932 lines) is mostly install, upgrade, backup and handoff, all D-adjacent.
- `compile_profile*` (about 3.5k lines) is opt-in.
- Duplicated helpers exist (six file-digest helpers, three durable-JSON writers), but they are tiny. Not material.

## 5. Strengths, preservation constraints and adequate areas

- **Readiness observes and never synchronizes**, and it delegates staleness to uv's own `--locked --check` and `cache-keys` (H4). Native fingerprints are computed only for the `native` route.
- **Storage publication is bounded.** It uses a path index and scope pointers with no history scan. Exclusive critical sections are short, limited to validation plus at most 3 fsync'd writes (safeguard 9 counted). Hierarchical shared ancestor admission is a legitimate protocol for whole-root retirement.
- **Cleanup authority is independent of exit status.** `SpawnGuard` and cleanup records in `runs` and the fixture are a preservation constraint.
- **Log tails are bounded.** `harness.tail_bytes` seeks from the end, used by `runs.cmd_logs` and `verify._report_result`.
- **Evidence projection is lazy** (`projection.prepare` on read), and native originals stay authoritative (ADR-0147).
- **Honest labels.** Both "source_to_binary: unknown" and "executed_bytes: unknown" are kept. Any remedy for F2-02 must preserve them.
- **The fixture separates logical attempts from service lifetime.** Release never stops the daemon or tears down storage.
- **Boundaries are explicitly blocked, failed or not_run**, and one boundary's outcome never decides another's.

## 6. Positions on earlier leads

| Lead | Position | Evidence |
|---|---|---|
| build-storage F01: whole-run retention conflates receipts and raw evidence | residual | `CaptureOwner.__init__` enrolls `run-log` (30 d) and `run-receipt` (90 d) separately; `Owner.run` enrolls the run receipt; profile `profile-raw` and `profile-report` are distinct categories. Reports are still retained without `until` (§4). |
| build-storage F02: isolated compiler roots lack a task lifetime | resolved (Implemented, not exercised) | `build_environment.main`'s managed-scope path (scopes, nonce, `retain`/`release` around `spawn_group`); `storage_owners.observe` kind `task` |
| build-storage F03: profiling safeguard misses its dominant writer | not examined (retention observed only) | §4 |
| storage-lifecycle-plan F01: descendant protection versus enclosing removal | resolved in mechanism (Implemented) | `storage_lifecycle.admission` takes shared flocks on every lexical ancestor with ordered acquisition. Interleaving controls not run by me. |
| storage-lifecycle-plan F02: portable replay omits external dependencies | not examined | |
| agent-effectiveness-followup F02: live display in the supervision path | resolved | `verify.launch` and `runs._run_owned` use a best-effort `FileDisplay` over the owned log |
| agent-effectiveness-followup F03: readiness detached from ownership | resolved for Python environments; **residual (redundant) for native-store** | `_run_boundary` takes ownership, then observes; native readiness runs before `borrow` and is then re-proved under it (F2-01) |
| agent-effectiveness-followup F04: small log reads scale with the whole log | resolved | `harness.tail_bytes` |
| agent-effectiveness-followup F01, F05 | not examined | |
| agent-test-evidence F02: context cannot support exact comparison | resolved; it is the source of F2-02's cost | `SourceObserver`, `capture.configuration`, `cli.compare` |
| agent-test-evidence F03: boundary reruns too coarse | residual | `rerun.nextest_case` prints the exact case but keeps a package-wide build (F2-04); `--rerun` remains boundary-only, and nextest's native `-R` is unused (F2-05) |
| agent-test-evidence F01, F04 | resolved as far as I examined | lazy projection with pinned coverage; bounded reads |
| agent-test-evidence-integrated F01–F05 | F02 and F05 confirmed implemented (`configuration`; `copy.deepcopy(fixture)` in `CaptureOwner.start`); others not examined | |
| agent-workspace-effectiveness F01: fixture lacks run ownership | resolved | attachments enrolled under storage; recovery via `surrealdb_fixture.recover` |
| agent-workspace-effectiveness F02: sync uncoordinated with importers | resolved | `workspace_env.ownership` exclusive for sync, shared for consumers |
| agent-workspace-effectiveness F03–F08 | not examined | |
| agent-effectiveness-integrated F01–F02 | not examined | |
| storage-lifecycle-management-plan §9 rows | not individually examined | |
| P4 "uniform ~280 s from per-process preparation" (coordinator probe hypothesis) | refuted as stated | JUnit shows 32 × "process aborted with signal 15", and the run was `interrupted` |

## 7. Candidate rule impacts

- **RC-1** (F2-02).
  - *Rule:* the agent-test-evidence plan says to "Capture available Cargo/native artifact identities…"; README.md (Tests) says "Source and binary digests describe observations…". Both imply eager binary digests.
  - *Proposed change:* artifact identity means cargo identity plus stat at completion, with the digest computed on export or citation, or for non-fresh artifacts.
  - *If the rule is kept:* use a host-level stat-keyed digest cache, so that only rebuilt binaries are hashed.
- **RC-2** (conditional; §4 seam, not an F2 recommendation).
  - *Rule:* AGENTS.md "do not impose job, worker or thread caps", operator memory "never constrain test parallelism", and `docs/surrealdb.md` "normal available parallelism" for fixture controls.
  - *When it applies:* only if D or H establish that shared-service capacity, rather than a defect, causes fixture timeouts. In that case an H21 remedy, such as a nextest test-group or a service-side admission for fixture-backed tests, would need this rule changed.
  - *If the rule is kept:* the alternative is physical isolation per attachment, such as a per-attempt database or scoped control rows, which belongs to D.
- No rule impact for F2-01, F2-03, F2-04 or F2-05.

## 8. Proposed light probes

- **PR-1** (F2-01: how much do the version children cost?)
  - *Command:* `for i in 1 2 3 4 5; do /usr/bin/time -f %e /home/paul/.local/state/library-context/surrealdb.tools/surreal/405f6c02ddfe12c004b04deab6bd91fd1458ed1c3ba247c05baa4a5396d5e206/surreal version; done`. This executes the version subcommand only; no service starts.
  - *Discriminates:* at ≤0.05 s each, the six children are negligible and F2-01 rests on the hashing (about 1 s). At ≥0.3 s each, version children dominate the fixed fixture overhead.
- **PR-2** (F2-02: what does the binary-digest cost?)
  - *Command:* `python3 -I` over the `binaries[*].path` list in `build/runs/20261011T071240.860Z-5e33c9/verify-evidence/d6a69f3371d04fe1920af33204a5fade/invocation.json`, timing `hashlib.file_digest` on each existing path, 3 passes, read-only.
  - *Discriminates:* about 3 s warm confirms the attribution by difference (about 4.3 s). Under 1 s means the outside-step time lies elsewhere, such as the nextest store export.
- **PR-3** (process-chain adequacy).
  - *Command:* `for i in 1 2 3 4 5; do /usr/bin/time -f %e env -u UV_NO_SYNC uv run --no-project --offline --no-python-downloads --python 3.14.7 python scripts/build_environment.py -- true; done` from the repository root. No sync, because of `--no-project`.
  - *Discriminates:* under 0.2 s per recipe line confirms "adequate"; over 0.5 s would make multi-line recipes and the nested `uv run` worth a candidate.
- **PR-4** (F2-05 applicability).
  - *Command:* the `cargo-nextest` 0.9.146 source or changelog, for whether `--rerun` requires `NEXTEST_EXPERIMENTAL_RECORD` and accepts an exported zip from another state directory. This is a static source or documentation read.
  - *Discriminates:* if it requires an unsupported store layout, F2-05 reduces to pytest `--lf` plus the existing `rerun-plan`.

**Relevant paths:**
- `/home/paul/library-context/scripts/verify.py`
- `/home/paul/library-context/scripts/verify_evidence/capture.py`
- `/home/paul/library-context/scripts/verify_evidence/rerun.py`
- `/home/paul/library-context/scripts/verify_evidence/nextest.py`
- `/home/paul/library-context/scripts/surrealdb_service.py`
- `/home/paul/library-context/scripts/surrealdb_server.py`
- `/home/paul/library-context/scripts/surrealdb_fixture.py`
- `/home/paul/library-context/scripts/storage_lifecycle.py`
- `/home/paul/library-context/scripts/runs.py`
- `/home/paul/library-context/build/runs/20261011T071240.860Z-5e33c9/`
- `/home/paul/library-context/build/runs/20261008T225907.834Z-dd7ee2/`
