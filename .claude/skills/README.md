# Skills

Two kinds live here. Both are visible to Codex through the `.agents/skills` symlink.

## Process skills (tracked in this repo)

Workflows for building this repository. A skill is added when a workflow has actually repeated.

| Skill | Use |
|---|---|
| [`adr/`](adr/SKILL.md) | Record or supersede a decision; amend DESIGN.md in the same commit |
| [`design-review/`](design-review/SKILL.md) | Review a design or code against the layered design standard (`docs/design_review/design_principles/standard.toml`); cadence in the library-context binding |
| [`design-review-code-intelligence/`](design-review-code-intelligence/SKILL.md) | The code-intelligence profile, loaded with `design-review` |
| [`handoff/`](handoff/SKILL.md) | Rewrite STATUS.md from the actual tree at session end |
| [`pin-check/`](pin-check/SKILL.md) | Change or re-verify any dependency or tool pin |

## Library capability skills (gitignored — versioned outside this repo)

Pinned, offline indexes of the libraries this project builds on, queried with ripgrep, ast-grep
and `Read`. They exist because an agent writing against a fast-moving library otherwise writes
the API it remembers. Several are tens of GB with their evidence, so this repo ignores them;
each carries its own `build/` to reproduce it and a `reference.md` saying what it does **not**
claim — silence in an index is not evidence of absence.

Check a skill's pinned profile against DESIGN §7 before transferring a claim: `deltalake` is
pinned to exactly this repo's delta-rs git profile; `pyrefly-ruff` indexes ruff 0.16.7 (crates
0.0.13) / pyrefly 1.3.1. We link pyrefly 1.3.1 plus a visibility patch, and ruff crates **0.0.11**;
the known 0.0.11 vs 0.0.13 deltas are listed in `docs/pins.md`.

| Repository | Indexes | Built from |
|---|---|---|
| [`datafusion/`](datafusion/SKILL.md) | DataFusion and the Arrow family | hosted rustdoc JSON |
| [`deltalake/`](deltalake/SKILL.md) | delta-rs and the kernel | rustdoc JSON, part locally built |
| [`fastmcp/`](fastmcp/SKILL.md) | `fastmcp`, `mcp`, `mcp-types`, plus 774 files of upstream docs, examples and tests | Griffe + the ty language server + pinned GitHub tarballs |
| [`pyrefly-ruff/`](pyrefly-ruff/SKILL.md) | 42 ruff and pyrefly crates, every catalog the two tools emit about themselves, a question router and 16 reviewed briefs, and source-derived maps of all 970 rules and 144 error kinds | rustdoc JSON + the tools' own JSON oracles + ast-grep facts from the pinned source, and 21 **executed** probes |
| [`ast-grep-ripgrep/`](ast-grep-ripgrep/SKILL.md) | every flag, node kind, rule field and regex construct of both tools, plus the 23 library crates underneath | the installed binaries' own help, ast-grep's shipped schemas, PCRE2 10.48's manual, rustdoc JSON — and 49 **executed** probes |
| [`rust-code-model/`](rust-code-model/SKILL.md) | the seven layers of Rust program knowledge -- rustdoc JSON, ra_ap_syntax, ra_ap_hir, the project-loading layer, MIR, dataflow and cargo metadata -- across 11 crates | docs.rs rustdoc JSON, pinned rustc and rust-analyzer source, and 33 **executed** probes |
| [`datafusion-tracing/`](datafusion-tracing/SKILL.md) | `datafusion-tracing` and `instrumented-object-store`, plus the `tracing` and OpenTelemetry wiring they cannot be used without -- 11 crates | docs.rs rustdoc JSON **and** a local `--document-private-items` capture, upstream's own trace snapshots, 16 releases of registry history, and executed probes |
| [`typer-rich/`](typer-rich/SKILL.md) | `typer`, `rich`, and the Click that typer 0.26 vendored, as three subjects; plus 855 files of upstream docs, runnable tutorials, examples and tests | Griffe + ty + pyrefly + pinned GitHub tarballs, and 19 **executed** probes that capture rendered output |
| [`rust-graphs/`](rust-graphs/SKILL.md) | seven pinned graph libraries with petgraph as the hub (petgraph 0.8.3, rustworkx-core 0.18.1, leiden-rs 0.8.1, graphops 0.5.1, graphina, rust-igraph, raphtory): a library ladder, capability coverage and interop matrices, reviewed decision briefs and exact-release API contracts | pinned source per library, a compile-proved algorithm-by-container matrix, and cross-library behaviour and compile probes (B001–B015, C001–C005) |
| [`ty-flow/`](ty-flow/SKILL.md) | the embeddable ty flow line: 15 crates (ty_* 0.0.14, ruff_db and ruff 0.0.14, salsa =0.28.2); a router, 14 briefs, source-derived maps of the index builder, salsa queries and env reads, and a ruff 0.0.11/0.0.13/0.0.14 name table | docs.rs rustdoc JSON plus a private-items capture, ast-grep facts from the pinned `.crate` sources, and 17 **executed** probes (the ty CLI oracle is blocked: no release matches the crates' commit) |
| [`rust-reasoning/`](rust-reasoning/SKILL.md) | symbolic reasoning over conditions and relations: biodivine-lib-bdd 0.6.3, OxiDD 0.12.0, z3 0.21.1 / z3-sys 0.13.1 on **Z3 5.1.0** (all 807 C functions and 1,040 z3 items in 28 capability groups, each marked safe or z3-sys-only), ascent 0.8.1, datafrog 2.0.1, fcars 0.2.2; a library ladder, 43 briefs, executed interop routes, and Z3's own parameter, tactic, probe and simplifier catalogs | docs.rs plus local all-features captures of 12 crates, catalogs read from Z3 5.1.0 through its C API, and 64 behaviour and 9 compile **executed** probes |
| [`python-oracles/`](python-oracles/SKILL.md) | independent oracles for a static analyzer's claims: pyre-check/Pysa 0.10.0, crosshair-tool 0.0.110, Hypothesis 6.168.1, CPython 3.14.7 `sys.monitoring`, bytecode 0.19.0; a fact-family router and 14 briefs | Griffe + ty + pyrefly API models, pinned upstream tarballs, parser-walked CLI tables, and 25 **executed** probes |
| [`pyomo-and-solvers/`](pyomo-and-solvers/SKILL.md) | Pyomo, and the eight open-source solvers it can reach -- Ipopt with sIpopt, SCIP, Bonmin, Couenne, Cbc, Clp, GLPK, HiGHS | **built from source into one prefix**, against one compiler, one BLAS and one ASL; each binary's own option registry; and 17 **executed** probes |
