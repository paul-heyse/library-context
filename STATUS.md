# Status

_Updated 2026-09-23 by the handoff skill._

## Where we are

- **Increment 1** (DESIGN §1.2): slices 1–3 are done (extraction, derive/validate/publish,
  Stage A and the pivot to FastMCP 4.0.5; ADR-0013).
- **The CPG plan is complete** (operator, 2026-09-22: CPG before Pass A; ADR-0014; ADR-0004
  re-sequencing). Slices C1–C6 are built, and each review's findings are fixed:
  - C1: node/edge catalogs, typed endpoints, retention, stage metrics (standard review);
  - C2 `syntax`; C3 `lexical`; C4 `types` (fork `6a93da34`, branch `lctx/1.3.1-r2`);
  - C5: the source corpus (docs family, and a usage run that names the release's files by
    `@path`);
  - C6: the whole-CPG measurement and the **deep review** (`design_review_cpg-c6-whole-cpg_2026-09-23.md`,
    Accept with claims narrowed; fixed in `8d1cac1`).
  - ADR-0015 (the operator's decision on C6 F2): every analyzed module's text and role are
    stored in `source_files`; `[tool.lctx.source]` takes `examples` and `tests`.
- **The pilot graph**: FastMCP 4.0.5 plus its corpus at `004bf15a`. 905,648 nodes,
  1,449,162 edges and 496 rules (18 of them declared edit guards, DESIGN §8).
- **Pushed** to `origin/main` on 2026-09-23 (operator).

## Last verified (2026-09-23, at the ADR-0015 commit)

| Command | Outcome |
|---|---|
| `just test-all` | passed: nextest 84/84, pytest 21/21, rule tests 4/4, adr lint (15), lint-agents, fixtures, family, cargo-deny, pyrefly-fork, gold |
| `just pilot` (fresh store) | passed: snapshot `9a0ec4de…`, every rule, 45.0 s at 7.1 GB peak (default glibc); 9.1 MB of module text, 3.0 MB in Delta |
| `MALLOC_ARENA_MAX=2 target/release/lctx compile fastmcp --store build/review-store` (at `8d1cac1`) | passed: 61.2 s at 4.2 GB (scratch store deleted) |

## Known gaps

- **`build/`** is local and rebuildable. A contract change makes an old store fail with
  `SchemaDrift`; delete it or pass a new `--store`. `build/store.pre-c4fix` (621 MB) is an
  obsolete pre-C4-fix store and can be deleted.
- **Peak memory** is 6.7–8.0 GB with the default allocator, about 40–45% of it arena retention
  (DESIGN §4.3). The memory triggers read the peak under `MALLOC_ARENA_MAX=2`.
- **`pyproject.toml`** (the project environment) still owes its ADR-0010 restructuring.
- **petgraph's** Bfs/Dfs sibling order (§5) is still unlocated; the ADR-0011 spike settles it.

## Open decisions

- **ADR-0011** (analytics) stays `proposed` until its increment-2 spike.
- **Deferred review rows**, each with its trigger in its review. None has fired unhandled (the
  C6 review re-checked them):
  - ADR-0014 standard review O5–O7;
  - C2 O2, O3, O5; C3 O1, O3–O7; C4 O1 (first trigger only), O2–O4, O6;
  - C5 O1–O5; C6 O1–O5;
  - older: the ADR-0013, slice-2 and slice-1 rows.
- `just adr revisit` (2026-09-23): ADR-0002's `just deps` passed; the other triggers are manual,
  and none has fired (ADR-0012's patch is 40 changed lines of visibility and accessors).

## Next

Increment 1, slice 4: the analytics config, the invocation projection (§5
`GraphProjectionSpec` over the published catalogs) and Pass A.
