# Skills

Two kinds live here. Both are visible to Codex through the `.agents/skills` symlink.

## Process skills (tracked in this repo)

Workflows for building this repository. A skill is added when a workflow has actually repeated.

| Skill | Use |
|---|---|
| [`adr/`](adr/SKILL.md) | Record or supersede a decision; amend the owning architectural section in the same commit |
| [`plan-design-review/`](plan-design-review/SKILL.md) | Plan an approach to the user's review goal, using the existing design-review skills and standard |
| [`design-review/`](design-review/SKILL.md) | Review explicit domain models, ownership, contracts, change scenarios and qualitative workload execution fit against the layered design standard (`docs/design_review/design_principles/standard.toml`); cadence in the library-context binding |
| [`design-review-code-intelligence/`](design-review-code-intelligence/SKILL.md) | The code-intelligence profile, loaded with `design-review` |
| [`plan-creation/`](plan-creation/SKILL.md) | Plan the authoring approach from a design review or existing plan, including focused assessment of relevant dependencies |
| [`create-plan/`](create-plan/SKILL.md) | Write the design and execution plan, including foundation improvements and an adaptable document structure |
| [`plan-execution/`](plan-execution/SKILL.md) | Develop the approach to executing an existing plan: prerequisites, work ownership, integration and evidence |
| [`execute-plan/`](execute-plan/SKILL.md) | Carry authorized plan scope through implementation, review, functional verification and handoff |
| [`handoff/`](handoff/SKILL.md) | Rewrite STATUS.md from the actual tree at session end |
| [`pin-check/`](pin-check/SKILL.md) | Deliberately pin, cap or hold a dependency, or verify a version claim |

In Codex, invoke `/plan $plan-design-review <review goal>` to enter Plan mode and plan a review.
The skill also works in an ordinary conversation; it does not itself switch modes.

For a plan document, use `/plan $plan-creation <reference and goal>` to develop the authoring
approach. When ready to write the document, leave Plan mode and invoke `$create-plan <reference and goal>`.
Either skill can be used directly when the intended task is already clear. The
[suggested outline](create-plan/references/plan-structure.md) provides a flexible starting point
for the document.

For execution, use `/plan $plan-execution <plan and scope>` to develop the approach. Leave Plan
mode and invoke `$execute-plan <plan and scope>` to implement it; invoke execution directly when
the approach is already clear. Existing plans remain the durable owners of packages and receipts.

All three pairs use the [shared agent roles](../../.agents/roles/README.md). Skills own workflows;
roles supply reusable capabilities. The coordinator selects useful delegation and owns design,
integration and acceptance. Planning skills do not themselves switch runtime modes, and no hook
is needed.

## Library capability skills (shared live copies)

`.config/library-skills.toml` selects the shared library skills for both runtimes.
Run `just skills-sync` after changing it; `just skills-check` checks the links. Each selected
folder links to `~/.local/share/library-skills/skills/<name>` (or `LIBRARY_SKILLS_ROOT`).
Edit the shared copy to improve it for every selecting repo. Process skills above stay local.
Each library skill retains its pinned references, lookup scripts, evidence and build inputs.
Silence in an index is not evidence of absence.

Check a skill's pinned profile against the locked version (`uv.lock`/`Cargo.lock`) before transferring a claim: `python-analyzers` indexes
pyrefly 1.4.0-dev.3 (with its embedded ruff 0.0.14), ruff 0.16.10 (crates 0.0.16) and ty crates 0.0.16, the planned shift
targets. We link pyrefly 1.3.1 plus a visibility patch with ruff crates **0.0.11**, and ty **0.0.14** in `cpg-flow`, until
the shifts; the skill's `show migration` lists what changes on our surface.
It is the one shared skill with a labeled library-context layer (`show project`).

| Repository | Indexes | Built from |
|---|---|---|
| [`datafusion/`](datafusion/SKILL.md) | DataFusion and the Arrow family | hosted rustdoc JSON |
| [`fastmcp/`](fastmcp/SKILL.md) | `fastmcp`, `mcp`, `mcp-types`, plus 774 files of upstream docs, examples and tests | Griffe + the ty language server + pinned GitHub tarballs |
| [`python-analyzers/`](python-analyzers/SKILL.md) | pyrefly, ruff and ty in 57 crates (pyrefly's doc-hidden session API and private-module Pysa collectors; ty's semantic index, type inference and unpublished IDE, project and server crates), every catalog the three tools emit about themselves, an 82-question router and 46 reviewed briefs, source-derived maps of all 971 rules, 148 error kinds and 138 ty lints, a code-facts domain model (340 facts in 137 concepts: who outputs what, where they overlap, what none covers), and a library-context layer (usage and reasoning per brief, 14 opportunities, the pyrefly 1.3.1 → 1.4.0-dev.3 and ty 0.0.14 → 0.0.16 migrations) | rustdoc JSON + the tools' own JSON oracles (ty built from the ruff tag) + ast-grep facts from the pinned source, and 75 **executed** probes |
| [`ast-grep-ripgrep/`](ast-grep-ripgrep/SKILL.md) | every flag, node kind, rule field and regex construct of both tools, plus the 23 library crates underneath | the installed binaries' own help, ast-grep's shipped schemas, PCRE2 10.48's manual, rustdoc JSON — and 49 **executed** probes |
| [`rust-code-model/`](rust-code-model/SKILL.md) | the seven layers of Rust program knowledge -- rustdoc JSON, ra_ap_syntax, ra_ap_hir, the project-loading layer, MIR, dataflow and cargo metadata -- across 11 crates | docs.rs rustdoc JSON, pinned rustc and rust-analyzer source, and 33 **executed** probes |
| [`datafusion-tracing/`](datafusion-tracing/SKILL.md) | `datafusion-tracing` and `instrumented-object-store`, plus the `tracing` and OpenTelemetry wiring they cannot be used without -- 11 crates | docs.rs rustdoc JSON **and** a local `--document-private-items` capture, upstream's own trace snapshots, 16 releases of registry history, and executed probes |
| [`typer-rich/`](typer-rich/SKILL.md) | `typer`, `rich`, and the Click that typer 0.26 vendored, as three subjects; plus 855 files of upstream docs, runnable tutorials, examples and tests | Griffe + ty + pyrefly + pinned GitHub tarballs, and 19 **executed** probes that capture rendered output |
| [`rust-graphs/`](rust-graphs/SKILL.md) | seven pinned graph libraries with petgraph as the hub (petgraph 0.8.3, rustworkx-core 0.18.1, leiden-rs 0.8.1, graphops 0.5.1, graphina, rust-igraph, raphtory): a library ladder, capability coverage and interop matrices, reviewed decision briefs and exact-release API contracts | pinned source per library, a compile-proved algorithm-by-container matrix, and cross-library behaviour and compile probes (B001–B015, C001–C005) |
| [`rust-reasoning/`](rust-reasoning/SKILL.md) | symbolic reasoning over conditions and relations: biodivine-lib-bdd 0.6.3, OxiDD 0.12.0, z3 0.21.1 / z3-sys 0.13.1 on **Z3 5.1.0** (all 807 C functions and 1,040 z3 items in 28 capability groups, each marked safe or z3-sys-only), ascent 0.8.1, datafrog 2.0.1, fcars 0.2.2; a library ladder, 43 briefs, executed interop routes, and Z3's own parameter, tactic, probe and simplifier catalogs | docs.rs plus local all-features captures of 12 crates, catalogs read from Z3 5.1.0 through its C API, and 64 behaviour and 9 compile **executed** probes |
| [`python-oracles/`](python-oracles/SKILL.md) | independent oracles for a static analyzer's claims: pyre-check/Pysa 0.10.0, crosshair-tool 0.0.110, Hypothesis 6.168.1, CPython 3.14.7 `sys.monitoring`, bytecode 0.19.0; a fact-family router and 14 briefs | Griffe + ty + pyrefly API models, pinned upstream tarballs, parser-walked CLI tables, and 25 **executed** probes |
| [`pyomo-and-solvers/`](pyomo-and-solvers/SKILL.md) | Pyomo, and the eight open-source solvers it can reach -- Ipopt with sIpopt, SCIP, Bonmin, Couenne, Cbc, Clp, GLPK, HiGHS | **built from source into one prefix**, against one compiler, one BLAS and one ASL; each binary's own option registry; and 17 **executed** probes |
| [`sqlx-postgres/`](sqlx-postgres/SKILL.md) | sqlx 0.9 with PostgreSQL and the libraries that meet it: sea-query 1.0, pgpq 0.12, pgvector 0.4, testcontainers-modules 0.15, and a DataFusion 55 table-provider fork -- 13 crates; probes against PostgreSQL 18, 31 briefs, seams and a router | docs.rs rustdoc JSON, local profile captures, and probes run against a disposable PostgreSQL 18 |
| [`pyo3/`](pyo3/SKILL.md) | pyo3 0.29.2 (with ffi, macros, build-config) and pyo3-async-runtimes 0.29.0: a macro option matrix, obsolete-API catalog, 20 briefs, 66 probes | docs.rs rustdoc JSON, local captures, and probes against an embedded Python 3.14 |
| [`serde-arrow/`](serde-arrow/SKILL.md) | serde_arrow 0.15.1 and marrow 0.3.1 on the `arrow-59` profile; 37 probes, 8 briefs, an Arrow 59 interop page | docs.rs rustdoc JSON plus a local `arrow-59` capture |
| [`fixedbitset/`](fixedbitset/SKILL.md) | fixedbitset 0.5.7, one crate: set-operation length rules, tail bits, serde, the 0.4 delta; 11 probes | docs.rs rustdoc JSON plus local captures |
