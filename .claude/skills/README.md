# Skills

Two kinds live here. Both are visible to Codex through the `.agents/skills` symlink.

## Process skills (tracked in this repo)

Workflows for building this repository. A skill is added when a workflow has actually repeated.

| Skill | Use |
|---|---|
| [`adr/`](adr/SKILL.md) | Record or supersede a decision; amend DESIGN.md in the same commit |
| [`design-review/`](design-review/SKILL.md) | Review a design or code against the charter; cadence in ADR-0001 |
| [`handoff/`](handoff/SKILL.md) | Rewrite STATUS.md from the actual tree at session end |
| [`pin-check/`](pin-check/SKILL.md) | Change or re-verify any dependency or tool pin |

## Library capability skills (gitignored — versioned outside this repo)

Pinned, offline indexes of the libraries this project builds on, queried with ripgrep, ast-grep
and `Read`. They exist because an agent writing against a fast-moving library otherwise writes
the API it remembers. Several are tens of GB with their evidence, so this repo ignores them;
each carries its own `build/` to reproduce it and a `reference.md` saying what it does **not**
claim — silence in an index is not evidence of absence.

Check a skill's pinned profile against DESIGN §7 before transferring a claim: `deltalake` is
pinned to exactly this repo's delta-rs git profile; `pyrefly-ruff` indexes ruff 0.16.7 /
pyrefly 1.3.1, which may differ from the analyzer revisions increment 1 selects.

| Repository | Indexes | Built from |
|---|---|---|
| [`datafusion/`](datafusion/SKILL.md) | DataFusion and the Arrow family | hosted rustdoc JSON |
| [`deltalake/`](deltalake/SKILL.md) | delta-rs and the kernel | rustdoc JSON, part locally built |
| [`fastmcp/`](fastmcp/SKILL.md) | `fastmcp`, `mcp`, `mcp-types`, plus 774 files of upstream docs, examples and tests | Griffe + the ty language server + pinned GitHub tarballs |
| [`pyrefly-ruff/`](pyrefly-ruff/SKILL.md) | 42 ruff and pyrefly crates, and every catalog the two tools emit about themselves | rustdoc JSON + the tools' own JSON oracles |
| [`ast-grep-ripgrep/`](ast-grep-ripgrep/SKILL.md) | every flag, node kind, rule field and regex construct of both tools, plus the 23 library crates underneath | the installed binaries' own help, ast-grep's shipped schemas, PCRE2 10.48's manual, rustdoc JSON — and 49 **executed** probes |
| [`rust-code-model/`](rust-code-model/SKILL.md) | the seven layers of Rust program knowledge -- rustdoc JSON, ra_ap_syntax, ra_ap_hir, the project-loading layer, MIR, dataflow and cargo metadata -- across 11 crates | docs.rs rustdoc JSON, pinned rustc and rust-analyzer source, and 33 **executed** probes |
| [`datafusion-tracing/`](datafusion-tracing/SKILL.md) | `datafusion-tracing` and `instrumented-object-store`, plus the `tracing` and OpenTelemetry wiring they cannot be used without -- 11 crates | docs.rs rustdoc JSON **and** a local `--document-private-items` capture, upstream's own trace snapshots, 16 releases of registry history, and executed probes |
| [`typer-rich/`](typer-rich/SKILL.md) | `typer`, `rich`, and the Click that typer 0.26 vendored, as three subjects; plus 855 files of upstream docs, runnable tutorials, examples and tests | Griffe + ty + pyrefly + pinned GitHub tarballs, and 19 **executed** probes that capture rendered output |
| [`petgraph/`](petgraph/SKILL.md) | petgraph's seven graph containers, its 47-trait lattice and 46 algorithms, plus all 27,183 lines of upstream source and its 23 integration tests | docs.rs rustdoc JSON **and** a local `--all-features` capture, boundary contracts for three mandatory deps, and 13 **executed** probes — 7 of which re-decide the whole capability matrix with rustc |
| [`pyomo-and-solvers/`](pyomo-and-solvers/SKILL.md) | Pyomo, and the eight open-source solvers it can reach -- Ipopt with sIpopt, SCIP, Bonmin, Couenne, Cbc, Clp, GLPK, HiGHS | **built from source into one prefix**, against one compiler, one BLAS and one ASL; each binary's own option registry; and 17 **executed** probes |
