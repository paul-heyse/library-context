# library-context — agent instructions

Stage 1 is a **capability compiler**. It turns a Python library's code facts plus its official
docs, examples and tests into evidence-backed capability briefs. Coding agents reach those briefs
through a FastMCP server with two tools, `search_capabilities` and `get_capability`
(`docs/design/DESIGN.md` §1).

The pieces:
- **Extraction:** Pyrefly (a pinned, minimally patched fork) and Ruff 0.0.11 crates, both linked
  in-process over one parse (ADR-0012). The one exception is the flow facts: `cpg-flow` reads ty's
  semantic index over a second parse, joined by byte range (ADR-0012 amendment, ADR-0022). The
  Pyrefly CLI is only a parity-test oracle.
- **Facts:** Arrow schemas are the contract, DataFusion constructs and validates the facts, and
  Delta stores them.
- **Analytics:** petgraph, leiden-rs and our own FCA/RCA.
- **Briefs:** assertions are synthesized **programmatically**. There is no LLM in v1.

The pilot library is FastMCP 4.0.5. Every analyzed library, the pilot included, is a pinned uv
project under `libraries/<name>/`, acquired and compiled by `lctx` (ADR-0013); the project's own
environment is never an analysis input.

This is a personal project with one operator. Process is deliberately light (ADR-0026). Keep
it that way: before adding a hook, gate, register or new document type, check that it has a
real consumer.

## Start of session

1. Read `STATUS.md`: where we are and what's next.
2. For design questions, read `docs/design/DESIGN.md` (the current truth) and
   `docs/adr/README.md` (why, and what was superseded). `docs/initial_plan/Initial_plan.md` is
   the research input; don't edit it.

## Where things are

| Path | What |
|---|---|
| `docs/design/DESIGN.md` | Current design. §2 holds the binding decisions §B1–§B14 |
| `docs/adr/` | Decision records, a generated index, and `TEMPLATE.md` |
| `docs/design_review/design_principles/` | The layered design standard, declared in `standard.toml`: six foundations (FP-01–06), architectural judgments A1–A3, supporting rules DP-01–24 and gates G1–G8, the CI profile, and the repository binding (ADR-0040) |
| `docs/design_review/reviews/` | Review outputs: evidence, never authority |
| `docs/design_review/evidence/` | Probes, spikes and investigations behind decisions, one `YYYY-MM-DD_<topic>/` folder each with a README; raw outputs and binaries through Git LFS; never venvs or `target/`. Put probes here, not in the session scratchpad |
| `docs/pins.md` | Every pin, with dated verification |
| `crates/` | The single Rust workspace. `cpg-schema` holds the authoritative Arrow contracts, derivations, rules and the graph registry (`graph.rs`: the `nodes`/`edges` catalogs, ADR-0014); `cpg-extract` (Stage A in `library.rs`, extraction, the dependency context in `context.rs`), `cpg-core` (Delta, the `lctx_id` UDF, derive, validate, publish) and `lctx` (the CLI). Further crates are added as increments need them (ADR-0012) |
| `docs/initial_plan/` | Research input (don't edit it) and `DISPOSITION.md`, which maps each input section to where it landed |
| `libraries/` | One committed uv project per analyzed library (`pyproject.toml` with `[tool.lctx] release`, `.python-version`, `uv.lock`); `libraries/README.md` has the add/upgrade procedure (ADR-0013). Environments go to `build/envs/` (gitignored) |
| `fixtures/python/` | Tiny Python packages to analyze. Input data: never executed or linted |
| `third_party/` | `pyrefly-<ver>.patch`: the one commit our Pyrefly fork adds to the upstream tag (ADR-0012, `docs/pins.md`) |
| `scripts/` | `adr.py`, `check_family.py`, `check_agents.py`, and the format hook |
| `rules/`, `rule-tests/` | ast-grep rules. They grow only from design-review findings |

## Commands

| When | Run |
|---|---|
| During a design/implementation phase | Use focused compile, probe, and behavior checks only where they resolve a material question. Do not run the fully integrated gate after each slice or commit. |
| At the end of the integrated scope | `just test-all`: fmt-check, clippy `-D warnings`, release-profile nextest, pytest + pyrefly, rules, ADR/agent lint, fixture parsing, `just deps` and `just gold` |
| The real library, end to end | `just pilot`: `lctx compile fastmcp` (release build) into `build/store`, printing rows and per-stage time and peak RSS. Run at the integrated end; report `not_run` for interim slices. |
| Inspect a published snapshot | `target/release/lctx query --store build/store --snapshot <hex> "SQL"` (read-only; every table by name at its recorded version) |
| Add or upgrade a library | `lctx library init <name> --requirement '<req>'`; upgrade with `uv lock --project libraries/<name> --upgrade-package <dist>` (`libraries/README.md`) |
| Format (mutating) | `just fmt` |
| Dependency policy | `just deps`: one version each of Arrow/DataFusion/object_store/delta-rs/ruff/pyrefly/blake3, cargo-deny, and the Pyrefly fork check (tag + patch, classified env reads) |
| Decisions | `just adr new <slug> --title "…"`, `just adr supersede ADR-NNNN <slug>`, `just adr index`, `just adr lint`, `just adr revisit` |
| Tools present? | `just doctor` |

The Rust toolchain is pinned to 1.98.1 in `rust-toolchain.toml`. The machine default is
nightly, so don't pass `+nightly` or `+stable` to cargo in this workspace. Python is 3.14.7 via
`uv`; run Python tools as `uv run …`. The type checker is **pyrefly**, not pyright or mypy.

The repository's `.cargo/config.toml` sets 16 Cargo jobs, `sccache` as the compiler wrapper,
and incremental compilation off so the compiler cache can store workspace crates. Stable rustc
uses one frontend thread by default; do not add nightly `-Zthreads` flags to the development
loop. Keep Clang and mold as the linker route. Avoid changing `CARGO_TARGET_DIR`, rustflags,
or worktrees during ordinary development because those changes disrupt build reuse. For an
isolated diagnosis, override the config through Cargo's environment variables and report it.

## Writing code against the pinned libraries

The library capability skills under `.claude/skills/` are pinned, offline indexes. Use them
**before** writing against an API, rather than relying on memory:
- `datafusion` (DataFusion, Arrow, object_store)
- `deltalake` (this repo's exact delta-rs git profile)
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
- `fastmcp` (FastMCP; must match `libraries/fastmcp`, checked by `scripts/check_gold.py`). This is
  also the **gold reference for evaluation**: its capability
  families are never a compiler input (DESIGN §1.4).

For a library no skill covers (e.g. vLLM, the Qwen embedding models, LanceDB, pyarrow),
**Context7 is the first stop**, then the `library-research` skill. The Context7 MCP server needs
a reconnect after its API key changes. Check a skill's pinned version
against `docs/pins.md` before transferring a claim. An empty search result is not evidence that a
capability is absent.

## Testing rules

- Full `just test-all` and `just pilot` runs are end-of-scope acceptance, not the design loop.
  Run focused checks during design, then run both once the integrated Stage 3 scope is ready;
  repeat only for a failure or a subsequent material change. Reuse cached release-profile Rust
  code for tests. Test data may be fresh, existing, or empty according to the test's purpose.
- **Schema contracts** are insta snapshots. `just check` runs with `INSTA_UPDATE=no`. To accept
  a change, read the `.snap.new` diff first, then run `cargo insta accept`. Never run
  `cargo insta review`, which is interactive. A schema snapshot change is a schema migration,
  so say so in the commit.
- **Codebooks are append-only.** Never renumber or reorder existing codes.
- **Validators are shared.** DataFusion invariant validators are library code, used by both
  tests and publication. Don't write test-only copies.
- **Fixtures** go under `fixtures/python/<case>/`. Intentional syntax-error cases go under
  an `_invalid/` subdirectory there.
- **Delta tests** go through Delta (the table provider or a scan), never a raw Parquet directory
  scan.

## Reporting

- Report outcomes as `passed`, `failed`, `blocked` (name the missing prerequisite) or `not_run`,
  and give the command that produced each. A mocked provider is never `passed`. Never turn
  outcomes into a percentage.
- Design claims carry a principles §D label (`Proposed` … `Tested` … `Measured`). An unlabelled
  claim is a defect; `Proposed` is not.
- Date every "verified" claim.

## Decisions and reviews

- **Write an ADR** (the `adr` skill) when a change alters a §B decision, chooses between real
  alternatives, or would surprise a future session. Amend DESIGN.md in the same commit. To pivot,
  supersede the old ADR; accepted ADRs are immutable.
- **Design reviews** use `design-review` and its declared profiles, usually through the
  `design-reviewer` subagent. The binding's **Reviews in this repository** section owns cadence
  and tier/purpose selection (ADR-0040). Begin with responsibilities, contracts and expected
  changes; assess A1–A3 separately from correctness/fidelity gates. A bounded slice does not
  certify the enclosing architecture. Review depth follows impact and uncertainty.
- **Findings** retain stable source-review IDs. The active plan owns current execution disposition
  for scheduled work; unscheduled deferrals stay in the source review with a trigger (binding §4).
  Record the responsible component and closure evidence; link from follow-up reviews and STATUS.
  A decision being accepted does not establish implementation or verified closure.

## Git

- Work on `main` in the current working tree for ordinary edits, reviews and spikes. Use a
  separate worktree only when truly parallel agents must edit production code concurrently.
- Commit to `main` in small commits. Each message names the slice and any ADR, and states the
  test outcome.
- Never force-push or `reset --hard`.
- `.claude/skills/*` is gitignored except the process skills: `adr`, `design-review`,
  `design-review-code-intelligence`, `handoff`, `pin-check`.
- At the end of a session that changed what's true, run the `handoff` skill.
