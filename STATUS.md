# Status

_Updated 2026-09-22 by the handoff skill._

## Where we are

- **Increment 1** (DESIGN §1.2): slices 1–3 are done.
  - Slice 1: extraction (Pyrefly fork + ruff in-process), raw contracts, Delta persistence.
  - Slice 2: Stage C/D derivations, generated validators, `snapshots` publication, reader
    (ADR-0008 accepted).
  - Slice 3 (`b1cc9a4`, `f179df1`, `771b7e8`): **Stage A and the pivot to FastMCP 4.0.5**
    (ADR-0013 accepted, superseding ADR-0007).
    - Every analyzed library is a committed uv project `libraries/<name>/`, acquired with
      `uv sync --frozen --no-config --python <pin> --link-mode copy`.
    - Stage A verifies analyzer-readable bytes against `RECORD`s and derives `release_id` from
      release content.
    - `lctx library init|acquire|compile` is the production path (`libraries/README.md`).
- **The fastmcp skill (gold) is at 4.0.5,** re-pinned through its own pipeline; the 4.0.3
  original is archived at `~/skill-work/fastmcp-4.0.3-original`. `just gold` checks they agree.
- **DESIGN.md has no line budget** (operator; ADR-0004 amendment).
- **Pushed** to `origin/main` (2026-09-22), including the operator's
  `docs/design_review/design_principles/rust_code_intelligence_data_graph_guidelines.md`.

## Last verified (2026-09-22, at `771b7e8`)

| Command | Outcome |
|---|---|
| `just test-all` | passed: nextest 63/63, pytest 21/21, ast-grep rules 4/4, adr lint (13), lint-agents, fixtures, family ok, cargo-deny ok, pyrefly-fork ok, `just gold` ok |
| `just pilot` (FastMCP 4.0.5, release build) | passed: 275 modules, 103 distributions, every rule, about 8 s, 1.6 GB RSS; two environment paths give one `content_digest` |
| fastmcp skill at 4.0.5: `verify.py`, `qualify.py`, 37 runtime cases | passed (the refresh agent's report, and `verify.py` re-run independently) |

## Known gaps

- **`build/`** (environments, stores) is local and rebuildable. A contract change makes an old
  store fail with `SchemaDrift`; delete it or use a new `--store`.
- **`pyproject.toml`** (project environment: fastmcp, vllm) is serving and dev only, and never an
  analysis input. Its restructuring per ADR-0010 is still due.
- **An asserted library claim not yet located:** petgraph's Bfs and Dfs visit siblings in
  opposite orders (§5). The ADR-0011 spike settles it.

## Open decisions

- **ADR-0011** (analytics) stays `proposed` until its increment-2 spike.
- **ADR-0007's revisit trigger** is resolved: its identity triggers now live in ADR-0013's
  `revisit:`, and the environment and lock are in `context_id`.
- **Deferred review rows** (each with its reopen trigger in its review):
  - ADR-0013: module → distribution link, lock forks, `python_platform` off Linux, typed Stage A
    errors, `lock_digest` granularity, an end-to-end `library init` test with a real uv;
  - slice 2: the ambiguous committed append, generated references, unbound-`def` calls,
    `caller_key`;
  - slice 1: hooks behind a dev feature, boundary provenance, the locator clamp, the harness
    `canon`.

## Next

**CPG first** (operator, 2026-09-22; ADR-0014 accepted 2026-09-23): slices C1–C6 before Pass A.
- **Done:**
  - C1: the node and edge catalogs, typed endpoints, retention, stage metrics; standard review
    fixed.
  - C2: the `syntax` family; its compact review fixed (every expression placed).
  - C3: the `lexical` family (scopes, bindings, references, full Python name resolution).
  - Pilot 2026-09-23: 234,328 nodes, 335,442 edges, every rule, no unresolved name; 17.5 s at
    3.91 GB.
- **C3 compact review:** due.
- **C4:** `types` (type terms, observations, record fields; probes P3/P3b answered by the Pyrefly
  API survey).
- **C5:** the source corpus (`docs`, examples, tests).
- **C6:** pilot measurement (the 3.9 GB peak decides the streaming trigger) and a deep review.

Then increment 1, slice 4: the analytics config, the invocation projection and Pass A.
