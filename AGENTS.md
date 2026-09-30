# library-context — agent instructions

library-context targets a **version-pinned API and evidence catalog for feature discovery and
correct use** (ADR-0071; DESIGN §1 and §14). Agents should find built-in APIs, inspect invocation,
configuration, examples and deployment evidence, and see precise support and uncertainty. Existing
behavioral analyses are enrichment; general semantic completion no longer gates the first product.
The detailed target is `docs/design/sections/api-and-evidence-product.md`; new capabilities there
remain Proposed until implemented. Existing FastMCP operation/brief tools remain the current interface.

**Current work: the semantic model cutover** (ADR-0085/0083/0084, accepted 2026-09-29). The target is
one declared relation model with a single owner per semantic question (`lctx-model`), PostgreSQL as
the single relational store, and DataFusion as in-process compute (DESIGN §15). The
[cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md) runs it as hard layers:
0. core;
1. store;
2. facts;
3. normalized relations;
4. analysis and catalog;
5. serving.

Legacy code is removed at the ownership boundary; no compatibility adapters, legacy IDs or dual stores.
Phases 0–2 reconstruct model/store/facts; downstream capabilities remain unavailable until phases 3–5. Product work (PR6, new features)
pauses until phase 5. The pieces below describe the implemented pipeline until each layer cuts over.

The pieces:
- **Extraction:** Pyrefly (a pinned, minimally patched fork) and Ruff 0.0.11 crates, both linked
  in-process over one parse (ADR-0046). The one exception is the flow facts: `cpg-flow` reads ty's
  semantic index over a second parse, joined by byte range (ADR-0046, ADR-0045). The
  Pyrefly CLI is only a parity-test oracle. Catalog compilation is the default (ADR-0078);
  `--profile behavioral` explicitly requests the flow provider and retained behavioral enrichment.
- **Facts:** the typed model (`lctx-model::domain`) is the contract and PostgreSQL generations
  store it (ADR-0086); `lctx-postgres` owns the generation store and the retained
  cache/operation services. The Delta store was removed in cutover phase 1 (P1.3/P1.4). Facts publication is implemented through `lctx compile --through facts`; qualification
  receipts and remaining work are in the cutover plan.
- **Behavior:** conditions are bounded BDDs over evaluation atoms (biodivine-lib-bdd), with
  pinned models and finite summaries composed over petgraph SCCs; five verdicts, never a null.
- **Analytics:** petgraph, leiden-rs and our own FCA/RCA.
- **Synthesis:** assertions are produced **programmatically**; no generative model runs in the
  pipeline or the query path.

The pilot library is FastMCP 4.0.5. Every analyzed library, the pilot included, is a pinned uv
project under `libraries/<name>/`, acquired and compiled by `lctx` (ADR-0046); the project's own
environment is never an analysis input.

This is a personal project with one operator. Process is deliberately light (ADR-0079). Keep
it that way: before adding a hook, gate, register or new document type, check that it has a
real consumer.

## Start of session

1. Read `STATUS.md`: where we are and what's next.
2. For the cutover, read the [cutover plan](docs/plans/semantic-model-cutover-plan_2026-09-29.md)
   (phases, work packages, qualification, §8 finding disposition) and DESIGN §15.
3. For product context, read the forward plan
   (`docs/plans/behavioral-model-forward-plan_2026-09-24.md`): §1 current state and qualification
   boundary, §3.0 product PR0–PR6 queue and §6 findings. The retained Stage 3–5 sequence is a
   research backlog, activated only by a product task or an exposed-claim defect. It coordinates product sequencing and owns
   finding disposition. For PostgreSQL work, §3.4 owns integration, §6.1 finding status and §7 conditional adoption.
4. For design questions, follow **owner → decision → open work**:
   - start at the architecture map `docs/design/README.md` and read the owning section in
     `docs/design/DESIGN.md` (scope, §B1–§B14) or `docs/design/sections/` plus adjacent consumers;
     labels there distinguish implemented, accepted target and proposed;
   - the section's `> Decision:` line names the ADR that holds the reason and rejected
     alternatives (`docs/adr/README.md` lists accepted records and open proposals);
   - known defects and deferred choices affecting the section are forward-plan §6 items, linked
     from the owner. Executable declarations (`cpg-schema`) own column-level detail.
5. Retired plans, reviews, evidence and ADRs are not reading context; recover one from Git only
   when a current owner is insufficient (`docs/README.md`, Historical recovery).

## Where things are

| Path | What |
|---|---|
| `docs/design/DESIGN.md`, `docs/design/sections/` | Architectural collection; stable § IDs. DESIGN §2 holds §B1–§B14 |
| `docs/README.md`, `docs/publishing.md` | Task routes and isolated documentation commands; site navigation/search is derived |
| `docs/plans/` | The semantic model cutover plan (current execution) and the product/research forward plan; finished or superseded plans are removed once their obligations move |
| `docs/adr/` | Current decision records (accepted and open proposals), a generated index, and `TEMPLATE.md` |
| `docs/design_review/design_principles/` | The layered design standard, declared in `standard.toml`: six foundations (FP-01–06), independent architectural judgments A1–A3, supporting rules DP-01–24 and gates G1–G8, the CI profile, and the repository binding (ADR-0040/0093) |
| `docs/design_review/reviews/` | Review outputs: evidence, never authority; kept while a finding they supply is open |
| `docs/design_review/evidence/` | Optional probes, spikes and investigations behind decisions. When created, use one `YYYY-MM-DD_<topic>/` folder with a README; raw outputs and binaries through Git LFS; never venvs or `target/`. This location convention does not require a review to create or run probes |
| `docs/pins.md` | Every pin, with dated verification |
| `docs/library-utilization.md`, `.jsonl` | Potentially valuable context on library capabilities and integration patterns already used in the codebase; optional focused lookups can inform design alternatives |
| `crates/` | The single Rust workspace. `cpg-schema` holds the authoritative Arrow contracts, derivations, rules and the graph registry (`graph.rs`: the `nodes`/`edges` catalogs, ADR-0086; the cutover replaces this crate with `lctx-model`); `cpg-extract` (Stage A in `library.rs`, extraction, the dependency context in `context.rs`), `cpg-core` (the stage runtime helpers, generation-bound provider sessions in `generation_read.rs`, the session and legacy `lctx_id` UDF, and the dormant analysis, catalog and serving code), `cpg-flow` (ty flow facts), `lctx-analytics` (passes, FCA/RCA, communities, summaries; Arrow in/out, no store), `lctx-embed` (compile-time embedding client), `lctx-postgres` (the generation store in `generations/`, the service baseline, role pools, the `testing` feature's disposable database and harness, and the dormant serving code) and `lctx` (the CLI). Further crates are added as increments need them (ADR-0046) |
| `python/` | `lctx_mcp` (the FastMCP server over one pinned generation) , `lctx_semantics` (the pure PyO3 native executor) and `lctx_storage` (explicit asynchronous PostgreSQL service lifetime) |
| `eval/` | `behavior/` pre-registered question sets, `gold/` evaluation-only gold extract and freeze, `heldout/` sealed until increment 5 |
| `libraries/` | One committed uv project per analyzed library (`pyproject.toml` with `[tool.lctx] release`, `.python-version`, `uv.lock`); `libraries/README.md` has the add/upgrade procedure (ADR-0046). Environments go to `build/envs/` (gitignored) |
| `fixtures/python/` | Tiny Python packages to analyze. Input data: never executed or linted |
| `third_party/` | `pyrefly-<ver>.patch`: the one commit our Pyrefly fork adds to the upstream tag (ADR-0046, `docs/pins.md`) |
| `scripts/` | `adr.py`, `design_sections.py` and `docs.py` (documentation), `check_family.py`, `check_agents.py`, and gold/eval scripts |
| `rules/`, `rule-tests/` | ast-grep rules. They grow only from design-review findings |

## Commands

| When | Run |
|---|---|
| During a design/implementation phase | Compile checks (`cargo check`/`cargo build` on the touched crates) and targeted tests or probes for the scope just implemented. No integrated gate after a slice or commit. |
| After all functional scope in the plan is implemented | `just test-all`: release-profile nextest, pytest (including the oracles), real PostgreSQL and compile-fail doc tests. Qualification also cites a clean end-of-turn report for the same tree (`.git/after-turn/report.json`) |
| The real library, end to end | `lctx compile fastmcp --through facts --profile catalog|behavioral`; reports a facts generation without selecting it. Analysis and serving remain unavailable. |
| The store and its generations | `lctx store install\|check\|reset`, `lctx generation list\|show\|select\|retire\|abort`, `lctx query --generation <id> "SQL"` (read-only); runbook: `docs/postgresql.md` |
| Add or upgrade a library | `lctx library init <name> --requirement '<req>'`; upgrade with `uv lock --project libraries/<name> --upgrade-package <dist>` (`libraries/README.md`) |
| Dependency policy | `just deps`, an end-of-turn check: one version each of Arrow/DataFusion/object_store/ruff/pyrefly/blake3, cargo-deny, and the Pyrefly fork check (tag + patch, classified env reads) |
| Decisions | `just adr new <slug> --title "…"`, `just adr supersede ADR-NNNN <slug>`, `just adr revisit`; the end-of-turn hook regenerates the index and runs the ADR lint |
| Documentation changes | `just docs-test` for publisher/resolver changes; `just docs-check` (publication) runs at the end of each turn. First run: `just bootstrap-docs`; preview: `just docs-serve`. No product gate solely for docs. |
| After every turn (automatic) | The end-of-turn hook (`scripts/after_turn.py`, ADR-0104, wired in `.claude/settings.json` and `.codex/hooks.json`) runs once the main agent stops: `just skills-sync`, `just adr index`, `just build-features` after dependency changes and `just fmt`; then, in the background, missing PostgreSQL images and tools and every `just hygiene` check (clippy, ruff with auto-fixes, pyrefly, rules, ADR and agent lint, fixtures, gold, `docs-check`, `deps`, `store-check`). A Sonnet or GPT fixer repairs what it can, the operator sees what is left, and `just library-catalog` runs last; the next prompt waits for the checks. Never run, check or troubleshoot any of this yourself |
| Tools present? | `just doctor`; the end-of-turn checks report a missing tool |

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
The end-of-turn hook runs `just build-features` after dependency changes, refreshing the CLI's
Hakari crate; `just deps` checks it. Lower libraries and Python bindings stay outside its dependency closure. Isolated benchmark
trials own both artifact directories and never clean the shared build directory.

## PostgreSQL superuser access for agents

The operator has granted agents full superuser access to the local PostgreSQL 18 cluster
(port 5432), to use however they see fit. Everything in it is regenerable from pinned inputs, and
no other data lives there. This covers all databases (`lctx`, `pse` and the `pse_test_*` leftovers)
and any administration: create, alter, drop, inspect, repair.
- Connect as `lctx_superuser`; `~/.pgpass` supplies the password:
  `psql -h 127.0.0.1 -U lctx_superuser -d lctx` (use `-d postgres` for cluster-level work).
- The same credential as a URL is in `~/.config/library-context/postgres-superuser.json`
  (`{"url": ...}`, mode 0600), for authorized local administration.
- This is the agents' own access. The four service roles (`lctx_migrator`, `lctx_app`,
  `lctx_importer`, `lctx_serving`) stay non-superuser, and `lctx store check` still enforces that.
  Product code and runtime configuration never use the superuser.
- Never print the password or commit either file.

## Writing code against the pinned libraries

Select library skills in `.config/library-skills.toml`; the end-of-turn hook runs `just skills-sync`
(`just skills-check` inspects). The gitignored `.claude/skills/<name>` links expose one live
copy per skill from `~/.local/share/library-skills/skills/` to both Codex and Claude Code.
Improvements there reach every selecting repo; process skills remain local. Set
`LIBRARY_SKILLS_ROOT` if the shared store is elsewhere. A new worktree gets its links when its first turn ends.

The library capability skills under `.claude/skills/` are pinned, offline indexes. They are helpful reference for identifying and understanding library functionality in depth:
- `datafusion` (DataFusion, Arrow, object_store)
- `rust-graphs` (petgraph plus rustworkx-core, leiden-rs, graphops and others: which library, how
  to reach it from a petgraph graph, and each one's silent failures)
- `pyrefly-ruff`. It indexes ruff crates 0.0.13, but we link 0.0.11 (Pyrefly's line). The deltas are
  listed in `docs/pins.md`
- `ty-flow` (the ty 0.0.14 / ruff_db / salsa =0.28.2 line that `cpg-flow` links: use-def maps,
  reachability and narrowing, module resolution, the salsa pin trap)
- `rust-reasoning` (biodivine-lib-bdd, OxiDD, z3 0.21.1 on Z3 5.1.0, ascent, datafrog, fcars: the
  condition kernel, full z3 capability coverage incl. what needs raw z3-sys, Datalog fixpoints and
  the FCA oracle)
- `python-oracles` (Pysa, CrossHair, Hypothesis with `sys.monitoring`, bytecode: independent
  cross-checks of the facts)
- `rust-code-model`
- `ast-grep-ripgrep`
- `datafusion-tracing`
- `sqlx-postgres` (sqlx 0.9 with PostgreSQL, plus sea-query, pgpq, pgvector, testcontainers-modules
  and the DataFusion table-provider fork: `AssertSqlSafe`, `query!`/`.sqlx`, pools, COPY
  finish/abort, migrations, disposable PG18; executed probes against PostgreSQL 18)
- `pyo3` (pyo3 0.29.2 and pyo3-async-runtimes 0.29.0: attach/detach, `Bound`/`Py`, macro options,
  exceptions, the async bridge, the obsolete-API catalog)
- `serde-arrow` (serde_arrow 0.15.1 with the `arrow-59` profile, marrow: the Arrow major must equal
  the Arrow crates we pin; explicit-schema traps)
- `fixedbitset` (0.5.7, petgraph's own dependency: length rules, tail bits, the `serde` feature)
- `fastmcp` (FastMCP; must match `libraries/fastmcp`, checked by `scripts/check_gold.py`). This is
  also the **gold reference for evaluation**: its capability
  families are never a compiler input (DESIGN §1.4).

For a library no skill covers (e.g. vLLM, the Qwen embedding models, LanceDB, pyarrow),
**Context7 is the first stop**. The Context7 MCP server needs
a reconnect after its API key changes. Check a skill's pinned version
against `docs/pins.md` before transferring a claim. An empty search result is not evidence that a
capability is absent.

## Testing rules

- **During implementation, only compile checks and targeted tests.** Validate each new piece of
  scope with a compile check and the focused tests or probes that exercise it. Integrated tests
  (`just test`, `just check`, `just test-all`, facts pilots) wait until all functional scope in
  the plan is implemented; repeat them only for a failure or a subsequent material change.
- **No formatting, linting or other non-functional checks.** The end-of-turn hook runs them all after
  each turn and a fixer agent repairs what it can (ADR-0104). Don't run `just fmt`, `cargo fmt`,
  `cargo clippy`, `ruff`, `pyrefly check`, `just deps` or any other `just hygiene` check, and don't
  troubleshoot their failures: they rewrite code other than yours, which you then have to reassess.
- Reuse cached release-profile Rust code for tests. Test data may be fresh, existing, or empty
  according to the test's purpose.
- **Schema contracts** are insta snapshots. `just check` runs with `INSTA_UPDATE=no`. To accept
  a change, read the `.snap.new` diff first, then run `cargo insta accept`. Never run
  `cargo insta review`, which is interactive. A schema snapshot change is a schema migration,
  so say so in the commit.
- **Codebooks are append-only.** Never renumber or reorder existing codes.
- **Validators are shared.** DataFusion invariant validators are library code, used by both
  tests and publication. Don't write test-only copies.
- **Fixtures** go under `fixtures/python/<case>/`. Intentional syntax-error cases go under
  an `_invalid/` subdirectory there.
- **Store tests** go through the generation store against real disposable PostgreSQL 18, never
  around it.

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

## Git

- Work on `main` in the current working tree for ordinary edits, reviews and spikes. Use a
  separate worktree only when truly parallel agents must edit production code concurrently.
- Commit to `main` in small commits. Each message names the slice and any ADR, and states the
  test outcome.
- Never force-push or `reset --hard`.
- `.claude/skills/*` is gitignored except the process skills: `adr`, `design-review`,
  `design-review-code-intelligence`, `plan-design-review`, `create-plan`, `plan-creation`,
  `handoff`, `pin-check`.
- At the end of a session that changed what's true, run the `handoff` skill.
