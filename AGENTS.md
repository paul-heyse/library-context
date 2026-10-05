# library-context — agent instructions

library-context targets a **version-pinned API and evidence catalog for feature discovery and
correct use** (ADR-0071; DESIGN §1 and §14). Agents should find built-in APIs, inspect invocation,
configuration, examples and deployment evidence, and see precise support and uncertainty. Existing
behavioral analyses are enrichment; general semantic completion no longer gates the first product.
The detailed target is `docs/design/sections/api-and-evidence-product.md`; new capabilities there
remain Proposed until implemented. FastMCP operation/brief contracts remain Rust-owned; native serving is pending.

**Current work: the graph-native hard pivot** (ADR-0128, accepted 2026-10-05).
The [coordinator](docs/plans/graph-native-pivot-plan_2026-10-05.md) owns dependencies,
acceptance and finding disposition. The first execution scope is the complete
[model/compiler stage](docs/plans/graph-native-model-compiler-plan_2026-10-05.md), including
compiler-side projections and selected analysis effects. Rust owns the semantic graph;
`cpg-core` is the sole store-free compiler. SurrealDB native persistence, querying and serving
are selected target responsibilities scheduled after this compiler scope.

The PostgreSQL backend and its clients are retired. No legacy readers, old-ID bridges, dual
stores or historical runtime retention are required. Compiler integration/admission is in progress;
compile checks alone do not establish a completed frontier or admitted artifact.
`lctx compile --artifact-only --output DIR` is the artifact route under implementation.
Ordinary compile requires the future native publisher and reports unavailable before acquisition.
MCP startup is unavailable until native serving is implemented. Product features pause during
this cut; STATUS and the coordinator own its current evidence and remaining work.

The pieces:
- **Extraction:** the accepted code-facts target links independent latest Ruff/ty and native
  Pyrefly in-process (ADR-0117/0118). Latest Ruff owns canonical syntax; Pyrefly's embedded Ruff
  stays inside its adapter, and ty's runtime view has explicit source/role correspondence.
  Migration is in progress: current pins/acceptance are in docs/pins and the code-facts coordinator. The
  Pyrefly CLI is only a parity-test oracle. Catalog compilation is the default (ADR-0078);
  `--profile behavioral` explicitly requests the flow provider and retained behavioral enrichment.
- **Facts:** independent native providers stream bounded typed batches into an attempt-owned,
  spillable Arrow/DataFusion workspace. Completed immutable streams feed normalization and analyses;
  graph admission and export are separate from native publication (ADR-0128). The model owns
  identities, roles, qualifications, coverage and graph meaning. Internal typed record views do
  not dictate the published physical schema.
- **Behavior:** conditions are bounded BDDs over evaluation atoms (biodivine-lib-bdd), with
  pinned models and finite summaries composed over petgraph SCCs; five verdicts, never a null.
- **Analytics:** petgraph, leiden-rs and our own FCA/RCA.
- **Synthesis:** assertions are produced **programmatically**; no generative model runs in the
  pipeline or the query path.

The pilot library is FastMCP 4.0.5. Every analyzed library, the pilot included, is a pinned uv
project under `libraries/<name>/`, acquired and compiled by `lctx` (ADR-0117); the project's own
environment is never an analysis input.

This is a personal project with one operator. Process is deliberately light (ADR-0079). Keep
it that way: before adding a hook, gate, register or new document type, check that it has a
real consumer.

## Start of session

Use instructions already in context; read this file if absent. Root agents follow the task-relevant
routes below. Delegated workers start from their brief, shared role and relevant authorities instead
of repeating the root's general orientation. Follow discovered dependencies as needed; all permission,
preservation and testing rules still apply.

1. Read `STATUS.md`: where we are and what's next.
2. For the pivot, read the [coordinator](docs/plans/graph-native-pivot-plan_2026-10-05.md)
   and the task-relevant supporting plan, then DESIGN §15.
3. For product context, read the forward plan
   (`docs/plans/behavioral-model-forward-plan_2026-09-24.md`): §1 current state and qualification
   boundary, §3.0 product PR0–PR6 queue and §6 findings. The retained Stage 3–5 sequence is a
   research backlog, activated only by a product task or an exposed-claim defect. It coordinates product sequencing and owns
   finding disposition. The graph-native coordinator owns the storage replacement and compiler acceptance.
4. For design questions, follow **owner → decision → open work**:
   - start at the architecture map `docs/design/README.md` and read the owning section in
     `docs/design/DESIGN.md` (scope, §B1–§B14) or `docs/design/sections/` plus adjacent consumers;
     labels there distinguish implemented, accepted target and proposed;
   - the section's `> Decision:` line names the ADR that holds the reason and rejected
     alternatives (`docs/adr/README.md` lists accepted records and open proposals);
   - known defects and deferred choices affecting the section are forward-plan §6 items, linked
     from the owner. Executable declarations (`lctx-model::domain`) own column-level detail.
5. Retired plans, reviews, evidence and ADRs are not reading context; recover one from Git only
   when a current owner is insufficient (`docs/README.md`, Historical recovery).

## Repository navigation

Use the [design principles](docs/design_review/design_principles/core/design-principles.md)
together with the [Heuristics for Efficient Architecture](docs/design_review/design_principles/core/efficient-architecture-heuristics.md)
when making consequential architectural and implementation choices. Consider relevant execution
patterns before committing to physical organization, interfaces, preparation, assurance and
lifecycles; address material mismatches while the design remains easy to change. Apply them
qualitatively, without an exhaustive checklist, cost model or additional proof machinery.

The [documentation task routes and repository map](docs/README.md) identify owners and paths.
Fixtures under `fixtures/python/` are input data, never executed or linted. Evaluation heldout data
remain sealed until increment 5. Add ast-grep rules only from design-review findings. Optional review evidence uses one dated topic folder under
`docs/design_review/evidence/`; raw outputs and binaries use Git LFS, never venvs or `target/`.

## Commands

| When | Run |
|---|---|
| During a design/implementation phase | Compile checks (`cargo check`/`cargo build` on touched crates) and focused affected contract controls through `just verify-model`, `verify-compiler`, `verify-analytics`, `verify-providers`, `verify-store`, `verify-serving`, `verify-oracles` or `verify-tooling`. Select the family and filters explicitly; no assembled gate after a slice or commit. |
| At functional scope completion | Run affected family controls and applicable non-functional leaf checks once. Use `just qualify` for this assurance pivot, a changed shared model/receipt/trust/transport contract, or uncertainty that focused controls cannot resolve: all required families, representative actual native-store/native/MCP journeys, compile-fail/doc contracts, full keep-going Clippy and applicable leaves for one tree. Fix failures and rerun affected controls or the failed `just <id>`; minor unrelated docs/library changes do not automatically trigger qualification. |
| The real library, end to end | `lctx compile fastmcp --artifact-only --output DIR --through facts|normalized|analysis|catalog --profile catalog|behavioral`; artifact path remains under integration. Run real-library qualification only when authorized. Compiler evidence is scoped in the graph-native coordinator; native serving, live vectors and real-library operator adoption remain not_run. |
| Native publication and serving | Pending the realization/serving stages; no default operator actions are implied by compiler checks. |
| Add or upgrade a library | `lctx library init <name> --requirement '<req>'`; upgrade with `uv lock --project libraries/<name> --upgrade-package <dist>` (`libraries/README.md`) |
| Dependency policy | `just deps`, the dependency-policy leaf check: one version each of Arrow/DataFusion/object_store/pyrefly/blake3 and scoped Ruff/ty source families, every exact Cargo pin or git rev has a `docs/pins.md` row, cargo-deny bans and sources, and the Pyrefly fork check (tag + patch, classified env reads) |
| Move dependencies to the latest | `just upgrade` (root `uv.lock` and `Cargo.lock`) at your discretion, then the affected tests; `libraries/*` and `services/vllm` move only deliberately (above, and their pins rows) |
| Decisions | `just adr new <slug> --title "…"`, `just adr supersede ADR-NNNN <slug>`, `just adr revisit`; `just turn-end` regenerates the index; agents run `just adr-lint` for affected decision metadata at scope end |
| Documentation changes | `just docs-test` for publisher/resolver changes; `just docs-check` for affected publication. First run: `just bootstrap-docs`; preview: `just docs-serve`. No product gate solely for docs. |
| End of a turn that changed files | The root agent runs `just turn-end` (ADR index, `build-features`, formatting with ruff's safe auto-fixes). Subagents don't. Clippy, pyrefly, lint, rules, ADR and agent lint, fixtures, gold, `docs-check` and `deps` findings are yours through applicable leaves at scope end. Native disposable store checks return with the realization stage; compiler qualification implies no operator action |
| After a dependency, toolchain or skill-selection change, or an environment-shaped failure | `just ready` (skills sync and tool check) |
| Tools present? | `just doctor`, or `just ready` |

The Rust toolchain is pinned to `nightly-2026-09-29` in `rust-toolchain.toml` (ADR-0079).
Do not pass floating `+nightly` or `+stable`. Python is 3.14.7 via `uv`; run Python tools as
`uv run …`. The type checker is **pyrefly**, not pyright or mypy.

Cargo uses 16 jobs, sccache, Clang/mold and workspace feature unification. Intermediates live in
`{cargo-cache-home}/build/library-context` across this repository's checkouts, with fine-grain
locking and its implied new layout; final artifacts stay in local `target/`. Never routinely run
`cargo clean` (it also removes shared intermediates). Preserve benchmark captures and results.
Workspace dev/release builds use O2 incremental; imported dependencies O3 non-incremental,
including path dependencies. Keep release tests and the default single frontend thread.

`just` and SQLx normalize inherited default/foreign target exports. For bare Cargo/uv or an IDE
shell use `eval "$(python3 scripts/build_environment.py --shell)"` before building. Prefer Cargo
config `build.target-dir`/`build.build-dir` to exported target paths; `LCTX_CARGO_TARGET_DIR` is an
explicit override for an external target. Keep paths and rustflags stable during ordinary edits.
`just turn-end` runs `just build-features`, refreshing the CLI's
Hakari crate; `just deps` checks it. Lower libraries and Python bindings stay outside its dependency closure. Isolated benchmark
trials own both artifact directories and never clean the shared build directory.
The build-environment wrapper also derives each native Python adapter's content/membership cache
key from its declared uv inputs. Test readiness synchronizes those adapters before Rust fixtures
invoke Python with `--no-sync`; a successful cached build alone does not prove model agreement.

## Operator state

This compiler stage does not inspect, reset or activate operator databases or client registrations.
Native SurrealDB persistence, publication and serving follow the graph-native coordinator; operator
adoption is a separately authorized package. Store-free compiler controls require no database.

## Writing code against libraries

### Dependencies

- **Add freely.** Add any library the work warrants without asking: no ADR and no
  `docs/pins.md` row. Use the tool's default specifier — `uv add` (a `>=` floor), `cargo add`
  (a caret requirement).
- **Latest by default.** The resolver picks versions; the committed lockfiles record them and
  the `--locked` gates keep runs reproducible. Move to the latest at your discretion with
  `just upgrade`, then run the tests the move affects.
- **Pin only for a reason.** An exact version, upper cap, git rev or hold-back needs an overt
  reason specific to that dependency — a named breakage, a type-sharing family that must
  resolve to one version, a fork or vendored source, golden or byte-stable output, a parity
  oracle, wheel availability — recorded in `docs/pins.md` with when to revisit it (the
  `pin-check` skill; `just deps` checks that every exact Cargo pin has a row). Reproducibility,
  "already in the lock" and a version entering a key or digest are not reasons (ADR-0125).
  The analyzer forks, families, toolchain and the analysed `libraries/*` keep their pins.

### Library skills


Select library skills in `.config/library-skills.toml`; then run `just ready`
(`just skills-check` inspects). The gitignored `.claude/skills/<name>` links expose one live
copy per skill from `~/.local/share/library-skills/skills/` to both Codex and Claude Code.
Improvements there reach every selecting repo; process skills remain local. Set
`LIBRARY_SKILLS_ROOT` if the shared store is elsewhere. A new worktree gets its links from its first `just ready`.

The library capability skills under `.claude/skills/` are pinned, offline indexes. They are helpful reference for identifying and understanding library functionality in depth:
- `datafusion` (DataFusion, Arrow, object_store)
- `rust-graphs` (petgraph plus rustworkx-core, leiden-rs, graphops and others: which library, how
  to reach it from a petgraph graph, and each one's silent failures)
- `python-analyzers` (pyrefly 1.4.0-dev.3 with its embedded ruff 0.0.14; ruff 0.16.10 / crates
  0.0.16; ty crates 0.0.16 plus its unpublished IDE, project and server crates and a CLI built
  from the same tag; salsa 0.28.5). The workspace now links these families; the code-facts
  coordinator records scoped migration receipts and remaining consumer qualification. Check
  `docs/pins.md` and the acquisition/extraction owner before transferring a claim. It covers ty in full beside pyrefly and
  ruff (semantic index, use-def, reachability and narrowing, module resolution, the salsa pin
  trap, and types, classes, calls, navigation, diagnostics, project and server) and maps every
  code fact the three output (`show facts`: per-concept providers, duplications, blanks). It
  supplies independently pinned capabilities, coordinate contracts and a generic facts taxonomy.
  Repository usage, boundary/finding disposition and migration status belong to the acquisition/
  extraction owner, docs/pins and active coordinator. The shared skill has no project overlay.
- `rust-reasoning` (biodivine-lib-bdd, OxiDD, z3 0.21.1 on Z3 5.1.0, ascent, datafrog, fcars: the
  condition kernel, full z3 capability coverage incl. what needs raw z3-sys, Datalog fixpoints and
  the FCA oracle)
- `python-oracles` (Pysa, CrossHair, Hypothesis with `sys.monitoring`, bytecode: independent
  cross-checks of the facts)
- `rust-code-model` (rustdoc JSON, ra_ap_* syntax/HIR/project model, MIR and dataflow, cargo
  metadata: which layer answers which question about Rust code)
- `ast-grep-ripgrep` (structural and regex search/rewrite: flags, node kinds, rule fields, PCRE2)
- `datafusion-tracing` (DataFusion planning, execution and object-store spans, metrics, exporters)
- `salsa` (0.28.4 index; the workspace pins 0.28.5, so check before transferring a claim: macro
  options, durability, backdating, cycles, cancellation)
- `pydantic` (2.13.5 / pydantic-core 2.46.5): strict wire models, JSON/schema and byte contracts.
- `pyo3` (pyo3 0.29.2 and pyo3-async-runtimes 0.29.0: attach/detach, `Bound`/`Py`, macro options,
  exceptions, the async bridge, the obsolete-API catalog)
- `serde-arrow` (serde_arrow 0.15.1 with the `arrow-59` profile, marrow: the Arrow major must equal
  the Arrow crates we pin; explicit-schema traps)
- `fixedbitset` (0.5.7, petgraph's own dependency: length rules, tail bits, the `serde` feature)
- `typer-rich` (Typer 0.27.2 with its vendored Click, Rich 15.0.0: CLI options, help/error output,
  console styling, tables and progress)
- `neo4j-surrealdb` (Neo4j Python driver 6.3.1 on Neo4j 2026.09 Community with Cypher 25;
  SurrealDB 3.3 Rust SDK/core, SurrealQL and GQL: choosing between them for graph modelling and
  writing correct code against either)
- `pyomo-and-solvers` (Pyomo 6.10.1, HiGHS, PySCIPOpt, cyipopt and native solvers; idaes-pse 2.13.0
  characterized statically)
- `fastmcp` (FastMCP; must match `libraries/fastmcp`, checked by `scripts/check_gold.py`). This is
  also the **gold reference for evaluation**: its capability
  families are never a compiler input (DESIGN §1.4).

The shared vLLM skill is a discovery lead at 0.30.0, while the isolated service uses custom
0.30.1rc1.dev286+g3d5f4d4cd.sm120.r2. Do not transfer its pinned contracts or enable it as an exact
match without checking the installed source for the selected API. No service upgrade is implied.

For a library no selected skill covers (e.g. the Qwen embedding models or LanceDB),
**Context7 is the first stop**. The Context7 MCP server needs
a reconnect after its API key changes. Check a skill's version
against the locked version (`uv.lock`/`Cargo.lock`) before transferring a claim. An empty search result is not evidence that a
capability is absent.

## Testing rules

- **During implementation, compile checks and focused contract controls.** Select the affected
  verification families and ordinary filters explicitly; validate each new piece with meaningful
  tests or probes. Pure model/analytics checks prepare no Python adapters or PostgreSQL. Provider,
  store, serving and oracle families prepare their actual prerequisites once before execution.
  Tests use `--no-sync`; never synchronize the environment while native guards/workers are live.
- **At functional scope completion, affected controls and applicable non-functional leaves.**
  This assurance pivot, changes to shared model/receipt/trust/transport contracts, or unresolved
  cross-boundary uncertainty require `just qualify` on the assembled tree (ADR-0126). It collects
  required families and representative journeys, compile-fail/doc contracts, full keep-going
  Clippy and applicable leaves without skipping independent failures. Failed readiness blocks its
  dependents and reports the prerequisite; empty required selections fail. Repair failures and
  rerun affected boundaries/leaves. Record chosen scope and limits; minor unrelated documentation
  or library changes do not automatically require assembled product qualification. Real-library
  pilots/activation still require authorization.
- **No formatting mid-work; applicable non-functional leaves once, at scope end** (ADR-0126).
  Run `just turn-end` as the last step of a turn that changed files, not mid-work. Agents run
  applicable leaves at scope end and fix findings; no non-functional checks during functional
  implementation.
- Reuse cached release-profile Rust code for tests. Test data may be fresh, existing, or empty
  according to the test's purpose. Store controls use owned disposable PostgreSQL 18; qualification
  never implicitly inspects or resets the default operator store.
- **Schema contracts** are insta snapshots. Verification runs with `INSTA_UPDATE=no`. To accept
  a change, read the `.snap.new` diff first, then run `cargo insta accept`. Never run
  `cargo insta review`, which is interactive. A schema snapshot change is a schema migration,
  so say so in the commit.
- **Codebooks are append-only.** Never renumber or reorder existing codes.
- **Validators are shared.** DataFusion invariant validators are library code, used by both
  tests and publication. Don't write test-only copies.
- **Fixtures** go under `fixtures/python/<case>/`. Intentional syntax-error cases go under
  an `_invalid/` subdirectory there.
- **Store tests** exercise the actual native persistent realization when that implementation exists.
  Compiler artifact controls require no database; they do not establish persistence or serving.

## Reporting

- Report outcomes as `passed`, `failed`, `blocked` (name the missing prerequisite) or `not_run`,
  and give the command that produced each. A mocked provider is never `passed`. Never turn
  outcomes into a percentage.
- Design claims carry a principles §D label (`Proposed` … `Tested` … `Measured`). An unlabelled
  claim is a defect; `Proposed` is not.
- Date every "verified" claim.

## Decisions and reviews

- **Write an ADR** (the `adr` skill) when a change alters a §B decision, chooses between real
  alternatives, or would surprise a future session. Amend the owning architectural section in the same commit. To pivot,
  supersede the old ADR; accepted ADRs are immutable.
- **Keep a current working set** (ADR-0042). Update the owner whose meaning changed instead of
  appending history. When a plan, review, evidence folder or ADR loses its last current consumer,
  carry its surviving obligations forward and delete it; Git holds it
  ([historical recovery](docs/README.md#historical-recovery)).
- **Design reviews** use `design-review` and its declared profiles, usually through the
  `design-reviewer` subagent. The binding's **Reviews in this repository** section owns cadence
  and tier/purpose selection (ADR-0040/0093). Apply the principles and review skill within
  those bounded review periods. Review depth follows impact and uncertainty.
  Execution fit is a qualitative assessment of material tradeoffs. It does not require numerical
  cost models, runtime accounting or additional proof machinery; those need a separate concrete
  functional or operational requirement. Quantitative performance claims still need measurement.
- **Review evidence:** static review of documentation, source and types can be sufficient,
  including for library features and fit. The reviewing agent judges whether complexity,
  criticality or unresolved uncertainty warrants creating and running probes. No review tier
  or kind requires probes or an evidence folder merely because it is a review. Existing
  implementation acceptance checks and the evidence needed for Tested/Measured claims still apply.
- **Findings** retain stable source-review IDs. The active plan's findings table owns current
  disposition for scheduled work (forward plan §6); unscheduled deferrals stay in the source review
  with a trigger (binding §4).
  Record the responsible component and closure evidence; link from follow-up reviews and STATUS.
  A decision being accepted does not establish implementation or verified closure.

## Design-phase cutover

After validating a change, fully pivot to the current design (ADR-0078). Rebuild project stores
and projections from pinned inputs; remove obsolete generations, runtime copies, rollback assets
and compatibility-only paths. Do not add old-format readers or retain historical runtime records
without a named current consumer. Quiesce project readers before replacing their state.

## Agent coordination

Use subagents when independent coverage, context isolation, distinct capabilities or independent
judgment justify handoff and integration cost. Keep small or tightly coupled tasks with the root
when appropriate. Follow the
[shared roles and coordination contract](.agents/roles/README.md) (ADR-0109), including its explicit
model/effort routing. The root agent owns design, integration and acceptance; choose the roles
and decomposition that serve the task. `library-research` may write new dated folders under
`docs/design_review/evidence/` (adding their Index row) and the shared library skill its brief
assigns; [its contract](.agents/roles/library-research.md) bounds those writes (ADR-0113).

## Git

- Work on `main` in the current working tree for ordinary edits, reviews and spikes. Use a
  separate worktree only when truly parallel agents must edit production code concurrently.
  Clean up any remaining worktrees that are fully merged.
- Commit to `main` in small commits. Each message names the slice and any ADR, and states the
  test outcome.
- Never force-push or `reset --hard`.
- `.claude/skills/*` is gitignored except the process skills: `adr`, `design-review`,
  `design-review-code-intelligence`, `plan-design-review`, `create-plan`, `plan-creation`,
  `plan-execution`, `execute-plan`, `handoff`, `pin-check`.
- At the end of a session that changed what's true, run the `handoff` skill.
