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
- **The pilot graph**: FastMCP 4.0.5 plus its corpus at `004bf15a`. 905,648 nodes,
  1,449,162 edges and 493 rules (18 of them declared edit guards, DESIGN §8).
- **Unpushed:** 13 commits on `main` since `origin/main` (`494557c`). Push only when asked.

## Last verified (2026-09-23, at `8d1cac1`)

| Command | Outcome |
|---|---|
| `just test-all` | passed: nextest 83/83, pytest 21/21, rule tests 4/4, adr lint (14), lint-agents, fixtures, family, cargo-deny, pyrefly-fork, gold |
| `just pilot` (fresh store) | passed: snapshot `ddee0669…`, content `66cddc06…`, every rule, 44.6 s at 7.1 GB peak (default glibc) |
| `MALLOC_ARENA_MAX=2 target/release/lctx compile fastmcp --store build/review-store` | passed: the same content digest, 61.2 s at 4.2 GB (scratch store deleted) |

## Known gaps

- **`build/`** is local and rebuildable. A contract change makes an old store fail with
  `SchemaDrift`; delete it or pass a new `--store`. `build/store.pre-c4fix` (621 MB) is an
  obsolete pre-C4-fix store and can be deleted.
- **Peak memory** is 6.7–8.0 GB with the default allocator, about 40–45% of it arena retention
  (DESIGN §4.3). The memory triggers read the peak under `MALLOC_ARENA_MAX=2`.
- **`pyproject.toml`** (the project environment) still owes its ADR-0010 restructuring.
- **petgraph's** Bfs/Dfs sibling order (§5) is still unlocated; the ADR-0011 spike settles it.

## Open decisions

- **Operator:** the usage modules' text and usage role (DESIGN §13; C6 review F2). Either store
  the text or re-read it with a content check, and record the role as a codebook. It must be
  decided before §10.5.
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
