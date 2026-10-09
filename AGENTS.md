# library-context — agent instructions

library-context targets a **version-pinned API and evidence catalog for feature discovery and
correct use** (ADR-0071; DESIGN §1 and §14). Agents should be able to:
- find built-in APIs;
- inspect invocation, configuration, examples and deployment evidence;
- see precise support and uncertainty.

Existing behavioral analyses are enrichment; general semantic completion no longer gates the first product. The detailed target is `docs/design/sections/api-and-evidence-product.md`. New capabilities there remain Proposed until implemented. FastMCP operation/brief contracts remain Rust-owned.

**Current work, its owners and its verification boundary live in [`STATUS.md`](STATUS.md)** and the plans it links. This file holds durable rules and routes; it does not track status.

The pieces:
- **Extraction.**
  - The accepted code-facts target links independent latest Ruff/ty and native Pyrefly in-process (ADR-0117/0118).
  - Latest Ruff owns canonical syntax. Pyrefly's embedded Ruff stays inside its adapter, and ty's runtime view has explicit source/role correspondence.
  - Current pins are in `docs/pins.md`. The Pyrefly CLI is only a parity-test oracle.
  - Catalog compilation is the default (ADR-0131); `--profile behavioral` explicitly requests the flow provider and retained behavioral enrichment.
- **Facts.**
  - Independent native providers stream bounded typed batches into attempt-owned persisted native contributions. Exact immutable completed views feed normalization and analyses.
  - Arrow/DataFusion is bounded compute/transfer, not completed IPC authority.
  - Necessary semantic admission is distinct from diagnostic producer replay.
  - Explicit detached imports establish the same owner properties without providers. Completed descriptors retain producer/profile/model, exact dependency views and vocabulary identity.
  - Ordinary publication seals the admitted compiler database directly; explicitly requested export/import retains independent admission (ADR-0138).
  - The model owns identities, roles, qualifications, coverage and graph meaning. Internal typed record views do not dictate the published physical schema.
- **Behavior.** Conditions are bounded BDDs over evaluation atoms (biodivine-lib-bdd), with pinned models and finite summaries composed over petgraph SCCs. Five verdicts, never a null.
- **Analytics.** petgraph, leiden-rs and our own FCA/RCA.
- **Synthesis.** Assertions are produced **programmatically**; no generative model runs in the pipeline or the query path.

The pilot library is FastMCP 4.0.5. Every analyzed library, the pilot included, is a pinned uv project under `libraries/<name>/`, acquired and compiled by `lctx` (ADR-0117). The project's own environment is never an analysis input.

This is a personal project with one operator. Process is deliberately light (ADR-0137). Keep it that way: before adding a hook, gate, register or new document type, check that it has a real consumer.

## Start of session

Use instructions already in context, and read this file if it is absent. Root agents follow the task-relevant routes below. Delegated workers start from their brief, shared role and relevant authorities instead of repeating the root's general orientation. Follow discovered dependencies as needed; all permission, preservation and testing rules still apply.

1. Read `STATUS.md`: where we are, what's next, and the current verification boundary.
2. For the current work, read the plan STATUS names and its relevant model/storage owner, then DESIGN §15.
3. For product context, read the forward plan (`docs/plans/behavioral-model-forward-plan_2026-09-24.md`):
   - §1: the current state and the qualification boundary;
   - §3.0: the product PR0–PR6 queue;
   - §6: findings.

   The retained Stage 3–5 sequence is a research backlog, activated only by a product task or an exposed-claim defect.
4. For design questions, follow **owner → decision → open work**:
   - Start at the architecture map `docs/design/README.md`, then read the owning section in `docs/design/DESIGN.md` (scope, §B1–§B14) or `docs/design/sections/`, plus adjacent consumers. Labels there distinguish implemented, accepted target and proposed.
   - The section's `> Decision:` line names the ADR that holds the reason and the rejected alternatives. `docs/adr/README.md` lists accepted records and open proposals.
   - Known defects and deferred choices affecting the section are forward-plan §6 items, linked from the owner.
   - Executable declarations (`lctx-model::domain`) own column-level detail.
5. Retired plans, reviews, evidence and ADRs are not reading context. Recover one from Git only when a current owner is insufficient (`docs/README.md`, Historical recovery).

## Repository navigation

Use the [design principles](docs/design_review/design_principles/core/design-principles.md) together with the [Heuristics for Efficient Architecture](docs/design_review/design_principles/core/efficient-architecture-heuristics.md) when making consequential architectural and implementation choices. Consider relevant execution patterns before committing to physical organization, interfaces, preparation, assurance and lifecycles; address material mismatches while the design remains easy to change. Apply them qualitatively, without an exhaustive checklist, cost model or additional proof machinery.

The [documentation task routes and repository map](docs/README.md) identify owners and paths.

**Semantic navigation.** For callers, definitions and implementations:
- **Claude:** the `LSP` tool (the user-level rust-analyzer plugin) answers `documentSymbol` immediately.
- **Codex:** the user-level rust-analyzer MCP.
- `findReferences`, `workspaceSymbol` and call hierarchy need a warm, fully indexed workspace. On 2026-10-08 they returned nothing early in a session.
- Fall back to `rg` and `ast-grep`. lctx-model's roughly 300 files mix `mod.rs` and `foo.rs` layouts, so search for a path rather than guessing it.

**Repository conventions:**
- Fixtures under `fixtures/python/` are input data, never executed or linted.
- Evaluation heldout data remain sealed until increment 5.
- Add ast-grep rules only from design-review findings.
- Optional review evidence goes in one dated topic folder under `docs/design_review/evidence/`. Raw outputs and binaries use Git LFS; never commit venvs or `target/`.

## Commands

`just --list` shows every recipe, and `just verify --help` lists verification boundaries. Recipes are shortcuts: bare `cargo`, `cargo nextest`, `uv run --no-sync` and `pytest` remain first-class, and `just env --explain` shows exactly what they inherit.

| When | Run |
|---|---|
| During a design/implementation phase | Compile checks (`cargo check`/`cargo build` on touched crates) and focused affected controls through `just verify --select FAMILY[:BOUNDARY] [--nextest-args "…"] [--pytest-args "…"]`, which is repeatable, with each argument applying to the selection before it. `just verify --print …` shows the resolved commands without building. The per-family shortcuts remain (`just verify-model`, `just verify-store --command rust -- ARGS`). No assembled gate after a slice or commit. |
| Fixture-backed tests with a bare tool | `just fixture -- cargo nextest run --release -p <crate> …`. It supplies `LCTX_SURREAL_TEST_CONFIG` and `LCTX_COMPILER_RUNTIME_CONFIG` on a disposable native SurrealDB. `--keep`, `--attach ID`, `--list` and `--stop ID` manage longer-lived servers. Exit 75 means blocked (binary, systemd, OOM or readiness), not a test failure. |
| Long commands | Use the runtime's own background facility, or `just run [--background] [--label L] -- <cmd>`, then `just runs list\|status\|logs\|cancel`. Every `just verify` run writes `summary.json` in its run directory. `just verify --rerun RUN` repeats only what did not pass. |
| Attribute slow Rust compilation | `just compile-profile doctor`, then `just compile-profile record --focus cpg-core -- cargo nextest run …` (or `--focus workspace`/a package). Use `status RUN`, `report RUN` and `view RUN --kind sampled\|compiler\|hotspot`. This reuses `just runs` ownership and preserves ordinary compiler/test parallelism. Wrap fixture-backed foreground recordings with `just fixture -- just compile-profile record …`; a background recording must own a longer-lived fixture. `just compile-profile-tools check\|sync` owns the exact analysis readers outside the product workspace. |
| At functional scope completion | Affected controls and applicable non-functional leaves once. `just qualify` (assembled: every family, representative journeys, compile-fail/doc contracts, keep-going Clippy and leaves) runs only when the plan STATUS names calls for it, or for a shared model/receipt/trust/transport change (ADR-0126). |
| Environment preparation | `just sync tools` (dev tools; never builds the native extension), `just sync native` (the full locked sync and native extension; a no-op while current) or `just sync vllm`. `just ready` runs after a dependency, toolchain or skill-selection change, or an environment-shaped failure, in any checkout: it selects that checkout's own `.venv`, then links skills, syncs native once and reports. A blocked verification names its repair route (ADR-0134). |
| Generated outputs | `just fresh [output…]` reports `clean`/`stale`/`heuristic`/`not_run` for Hakari, the ADR index, skill links, formatting, gold and insta, read-only, with each output's regenerate command |
| End of a turn that changed files | The root agent runs `just turn-end`: ADR index, `build-features`, formatting. When the tree holds another agent's uncommitted work, use `just turn-end --paths P…` or `--staged`. It formats only those paths and reports any skipped step for a later whole-tree run (ADR-0134). Subagents don't run it. |
| Parallel work | Concurrent commands may share one stable checkout when their effects do not conflict. For an independent revision or conflicting mutable state, such as another agent's uncommitted edits, use `just worktree NAME [--ref R] [--carry PATH…] [--build-dir shared\|own]`, and `just worktree-remove NAME`, which refuses dirty or unintegrated work without `--force`. |
| The real library, end to end | `lctx compile fastmcp --artifact-only --output DIR --through facts\|normalized\|analysis\|catalog --profile catalog\|behavioral`. Ordinary compilation admits and publishes an unselected native handle. Run real-library qualification only when authorized. |
| Native publication and serving | `lctx publish-artifact`, `lctx snapshot show/select/query/export/backup/restore/retire`, `lctx store init/check` and `lctx tool`. These use explicit runtime configuration, immutable viewer handles and reader quiescence; see `docs/surrealdb.md`. Disposable checks imply no operator action. |
| Add or upgrade a library | `lctx library init <name> --requirement '<req>'`; upgrade with `uv lock --project libraries/<name> --upgrade-package <dist>` (`libraries/README.md`) |
| Dependency policy | `just deps`, the dependency-policy leaf. It checks: one version each of Arrow/DataFusion/object_store/pyrefly/blake3 and the scoped Ruff/ty source families; every declared Cargo dependency exact (`=x.y.z` or a git `rev`); cargo-deny bans and sources; the Pyrefly/Ruff fork checks; `cargo shear`; Hakari. |
| Decisions | `just adr new <slug> --title "…"`, `just adr supersede ADR-NNNN <slug>`, `just adr revisit`. `just turn-end` regenerates the index; agents run `just adr-lint` for affected decision metadata at scope end. |
| Documentation changes | `just docs-test` for publisher/resolver changes; `just docs-check` for affected publication. First run: `just bootstrap-docs`; preview: `just docs-serve`. No product gate solely for docs. |
| Tools present? | `just doctor`, or `just ready` |

The Rust toolchain is pinned to `nightly-2026-09-29` in `rust-toolchain.toml` (ADR-0137). Do not pass floating `+nightly` or `+stable`. Python is 3.14.7 via `uv`; run Python tools as `uv run --no-sync …`. The system `python3` is 3.12, which cannot parse most scripts here. The type checker is **pyrefly**, not pyright or mypy.

**Cargo setup.**
- Cargo uses available CPU parallelism, sccache, Clang/mold and workspace feature unification.
- Intermediates live in `{cargo-cache-home}/build/library-context`, shared across this repository's checkouts, with fine-grain locking; final artifacts stay in local `target/`.
- Never routinely run `cargo clean` (it also removes shared intermediates).
- Preserve benchmark captures and results.
- Workspace dev/release builds use O2 incremental; imported dependencies O3 non-incremental, including path dependencies.
- Qualify local tests separately; until BC3 passes, verification defaults remain release. After qualification, ordinary tests use the test profile and production acceptance uses release (ADR-0137). Compiler frontend/backend workers use available logical CPUs.
- Run tests with their normal available parallelism and threading. Resolve concurrency failures
  in production or test code; do not impose job, worker or thread caps to make checks pass.

**What bare tools inherit.**
- Bare `cargo` reads `.cargo/config.toml` directly and needs nothing else, unless a foreign `CARGO_TARGET_DIR` is inherited. `just env -- <cmd>` normalizes that, and `LCTX_CARGO_TARGET_DIR` is the explicit override.
- Agent runtimes set `UV_NO_SYNC=1` for ad hoc `uv run`:
  - Claude, through the project `env`;
  - Codex, through `shell_environment_policy`;
  - IDE shells don't, so pass `--no-sync` there.
- Repository launchers always pass `--no-sync` or `--no-project`.
- An inherited absolute `UV_PROJECT_ENVIRONMENT` outside the checkout is ignored, unless `LCTX_ALLOW_FOREIGN_ENV=1`.
- `just turn-end` refreshes the CLI's Hakari crate, and `just deps` checks it. Lower libraries and Python bindings stay outside its dependency closure.
- Isolated benchmark trials own both artifact directories and never clean the shared build directory.

## Operator state

This work does not inspect, reset or activate operator databases or client registrations. Native SurrealDB persistence, publication and serving follow the graph-native coordinator; operator adoption remains held. Compiler, provider and CLI controls use owned disposable fixtures through `just fixture` or `just verify`. Pure model and finite kernel controls need no database.

## Writing code against libraries

### Dependencies

- **Pinned.** Every declared dependency is pinned exactly: `==x.y.z` (`[tool.uv] add-bounds = "exact"` makes `uv add` write it) or `=x.y.z` (`cargo add name@=x.y.z`). The committed lockfiles hold everything beneath, so environments change only when someone changes them on purpose (ADR-0132).
- **Change versions deliberately, on judgment.** Add or bump a dependency when the work calls for it, with no ADR or approval:
  - move that dependency (or its family);
  - check the lock diff moved only what you meant;
  - run the tests it affects;
  - name the move in the commit.

  No wholesale re-resolve (`uv lock --upgrade`, bare `cargo update`) unless the operator asks.
- **Holds go in `docs/pins.md`.** A version that must not be bumped casually gets a row with its reason and when to revisit it (the `pin-check` skill). Examples: a parity oracle, golden output, a known breakage, a library skill's profile. The analyzer forks, families, toolchain and the analysed `libraries/*` keep their own rules.

### Library skills

Select library skills in `.config/library-skills.toml`, then run `just ready`; `just skills-check` inspects them. The gitignored `.claude/skills/<name>` links expose one live copy per skill, from `~/.local/share/library-skills/skills/`, to both Codex and Claude Code. Improvements there reach every selecting repo; process skills remain local. Set `LIBRARY_SKILLS_ROOT` if the shared store is elsewhere. A new worktree gets its links from `just ready`.

The library capability skills under `.claude/skills/` are pinned, offline indexes, and helpful references for finding and understanding library functionality in depth:
- `datafusion` (DataFusion, Arrow, object_store)
- `rust-graphs`: petgraph plus rustworkx-core, leiden-rs, graphops and others. It covers which library to use, how to reach it from a petgraph graph, and each one's silent failures.
- `python-analyzers`:
  - pinned to pyrefly 1.4.0-dev.3 (with its embedded ruff 0.0.14), ruff 0.16.10 / crates 0.0.16, the ty 0.0.16 crates and salsa 0.28.5;
  - covers ty in full beside pyrefly and ruff;
  - maps every code fact the three tools output (`show facts`).

  Check `docs/pins.md` and the acquisition/extraction owner before transferring a claim. The shared skill has no project overlay.
- `rust-reasoning`: biodivine-lib-bdd, OxiDD, z3 0.21.1 on Z3 5.1.0, ascent, datafrog and fcars. It covers the condition kernel, full z3 capability coverage, Datalog fixpoints and the FCA oracle.
- `python-oracles` (Pysa, CrossHair, Hypothesis with `sys.monitoring`, bytecode: independent cross-checks of the facts)
- `rust-code-model` (rustdoc JSON, ra_ap_* syntax/HIR/project model, MIR and dataflow, cargo metadata)
- `ast-grep-ripgrep` (structural and regex search/rewrite)
- `datafusion-tracing` (DataFusion planning, execution and object-store spans)
- `salsa`: indexed at 0.28.4, while the workspace pins 0.28.5, so check before transferring a claim.
- `pydantic` (2.13.5 / pydantic-core 2.46.5)
- `pyo3` (pyo3 0.29.2 and pyo3-async-runtimes 0.29.0)
- `serde-arrow`: serde_arrow 0.15.1, `arrow-59` profile. The Arrow major must equal the Arrow crates we pin.
- `fixedbitset` (0.5.7, petgraph's own dependency)
- `typer-rich` (Typer 0.27.2 with its vendored Click, Rich 15.0.0)
- `neo4j-surrealdb`: the Neo4j Python driver 6.3.1 and SurrealDB 3.3 (Rust SDK/core, SurrealQL, GQL)
- `pyomo-and-solvers` (Pyomo 6.10.1, HiGHS, PySCIPOpt, cyipopt and native solvers)
- `fastmcp`: must match `libraries/fastmcp` (`scripts/check_gold.py` checks this). It is also the **gold reference for evaluation**, so its capability families are never a compiler input (DESIGN §1.4).

The shared vLLM skill is a discovery lead at 0.30.0, while the isolated service uses custom 0.30.1rc1.dev286+g3d5f4d4cd.sm120.r2. Do not transfer its pinned contracts or enable it as an exact match without checking the installed source for the selected API. No service upgrade is implied.

For a library no selected skill covers (e.g. the Qwen embedding models or LanceDB), **Context7 is the first stop**. The Context7 MCP server needs a reconnect after its API key changes. Check a skill's version against the locked version (`uv.lock`/`Cargo.lock`) before transferring a claim. An empty search result is not evidence that a capability is absent.

## Testing rules

**Design-phase scope (operator, 2026-10-07).** While substantial design changes are planned, implementation testing confirms two things:
1. the change is implemented correctly, shown by a minimal revealing case;
2. no first-principles basis undermines its value.

It is not a validation of the existing test scope. Long journeys, whole families and `qualify` wait for the full functional testing the plan in STATUS schedules. Failures caused by codebase friction are findings, not reasons to retreat from a change.

- **During implementation: compile checks and focused contract controls.**
  - Select affected boundaries and filters explicitly, with `just verify --select …` or a bare tool under `just fixture --`.
  - Pure model/analytics checks need no Python environment or database.
  - Readiness *observes* each boundary's prerequisites: a stale or missing environment is `blocked` with its repair (`just sync tools|native`). Verification never synchronizes (ADR-0134).
  - Never synchronize while native guards or workers are live. Managed `just sync` takes exclusive ownership and waits for, and names, live managed holders. An explicit bare `uv sync` bypasses that guarantee.
- **At functional scope completion: affected controls and applicable non-functional leaves.**
  - `just qualify` runs when the plan STATUS names calls for assembled acceptance, or for a changed shared model/receipt/trust/transport contract (ADR-0126). It collects required families and representative journeys, compile-fail/doc contracts, full keep-going Clippy and applicable leaves without skipping independent failures, and never prepares the environment.
  - Repair failures and rerun the affected boundaries (`just verify --rerun RUN`).
  - Record the chosen scope and its limits.
  - Real-library pilots and activation still require authorization.
- **No formatting mid-work; applicable non-functional leaves once, at scope end** (ADR-0126). Run `just turn-end` (or its `--paths`/`--staged` form) as the last step of a turn that changed files, not mid-work.
- Reuse cached Rust code for the selected profile. Before BC3 qualification, verification uses release; after it, ordinary Rust controls use test and required optimized acceptance uses release. Test data may be fresh, existing or empty, according to the test's purpose. Store controls use owned disposable SurrealDB 3.3 fixtures; verification never inspects or resets the operator store.
- **Schema contracts are insta snapshots.** Verification runs with `INSTA_UPDATE=no`. To accept a change, read the `.snap.new` diff first, then run `cargo insta accept`. Never run `cargo insta review`, which is interactive. A schema snapshot change is a schema migration, so say so in the commit.
- **Codebooks are append-only.** Never renumber or reorder existing codes.
- **Validators are shared.** DataFusion invariant validators are library code, used by both tests and publication. Don't write test-only copies.
- **Fixtures** go under `fixtures/python/<case>/`. Intentional syntax-error cases go under an `_invalid/` subdirectory there.
- **Store tests** exercise the actual native persistent realization. Pure finite model/kernel controls remain independent of a database and do not establish persistence or serving.

## Reporting

- Report outcomes as `passed`, `failed`, `blocked` (name the missing prerequisite) or `not_run`, and give the command that produced each. A mocked provider is never `passed`. Never turn outcomes into a percentage.
- Design claims carry a principles §D label (`Proposed` … `Tested` … `Measured`). An unlabelled claim is a defect; `Proposed` is not.
- Date every "verified" claim.

## Decisions and reviews

- **Write an ADR** (the `adr` skill) when a change alters a §B decision, chooses between real alternatives, or would surprise a future session. Amend the owning architectural section in the same commit. To pivot, supersede the old ADR; accepted ADRs are immutable.
- **Keep a current working set** (ADR-0042). Update the owner whose meaning changed instead of appending history. When a plan, review, evidence folder or ADR loses its last current consumer, carry its surviving obligations forward and delete it; Git holds it ([historical recovery](docs/README.md#historical-recovery)).
- **Design reviews** use `design-review` and its declared profiles, usually through the `design-reviewer` subagent. The binding's **Reviews in this repository** section owns cadence and tier/purpose selection (ADR-0040/0093). Review depth follows impact and uncertainty. Execution fit is a qualitative assessment of material tradeoffs: it needs no numerical cost models, runtime accounting or proof machinery, but quantitative performance claims still need measurement.
- **Review evidence.**
  - Static review of documentation, source and types can be sufficient, including for library features and fit.
  - The reviewing agent judges whether complexity, criticality or unresolved uncertainty warrants probes.
  - No review tier or kind requires probes or an evidence folder merely because it is a review.
  - Existing implementation acceptance checks, and the evidence needed for Tested/Measured claims, still apply.
- **Findings.**
  - Findings retain stable source-review IDs.
  - The active plan's findings table owns current disposition for scheduled work (forward plan §6). Unscheduled deferrals stay in the source review with a trigger (binding §4).
  - Record the responsible component and closure evidence; link from follow-up reviews and STATUS.
  - A decision being accepted does not establish implementation or verified closure.

## Design-phase cutover

After validating a change, fully pivot to the current design (ADR-0131):
- rebuild project stores and projections from pinned inputs;
- remove obsolete generations, runtime copies, rollback assets and compatibility-only paths;
- add no old-format readers, and retain no historical runtime records without a named current consumer;
- quiesce project readers before replacing their state.

## Agent coordination

Use subagents when independent coverage, context isolation, distinct capabilities or independent judgment justify the handoff and integration cost. Keep small or tightly coupled tasks with the root when appropriate.

Follow the [shared roles and coordination contract](.agents/roles/README.md) (ADR-0109), including its explicit model/effort routing. The root agent owns design, integration and acceptance, and chooses the roles and decomposition that serve the task.

`library-research` may write new dated folders under `docs/design_review/evidence/` (adding their Index row) and the shared library skill its brief assigns; [its contract](.agents/roles/library-research.md) bounds those writes (ADR-0113).

## Git

- **Where to work.** Work on `main` in the current working tree by default. Use a worktree (`just worktree`) for an independent revision or conflicting mutable state.
- **Getting worktree work back.** It returns to `main` by branch merge, `git cherry-pick wt/NAME` or an explicit patch; `just worktree-remove` treats cherry-picked commits as integrated. Clean up fully merged worktrees.
- **Commits.** Commit to `main` in small commits. Each message names the slice and any ADR, and states the test outcome. When another agent's uncommitted work is in the tree, stage explicit paths only.
- **Never** force-push or `reset --hard`.
- **Skills directory.** `.claude/skills/*` is gitignored except the process skills listed in [the skills README](.claude/skills/README.md).
- **Handoff.** At the end of a session that changed what's true, run the `handoff` skill.
