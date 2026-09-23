# Status

_Updated 2026-09-23 by the handoff skill._

## Where we are

- **Increment 1 is done** (DESIGN §1.2). The CPG feeds:
  - the invocation projection and Pass A (1.4);
  - Stage F synthesis (1.5);
  - embeddings with a global cache (1.6);
  - byte-identical serving generations (1.7);
  - the `python/lctx_mcp` FastMCP server with hybrid retrieval (1.8);
  - an end-to-end pilot with a stdio smoke (1.9).
  - Its deep review (`design_review_inc1-deep_2026-09-23.md`, Revise small) is dispositioned.
- **Increment 2 is in progress:**
  - 2.0 pre-registered the seed `fastmcp.FastMCP.mount` by a mechanical audit;
  - 2.1 added Pass B (`cpg_schema::flows`, `lctx_analytics::pass_b`) and `parameter_docs`.
  - Next is 2.2, Pass C and usage patterns.
- The plan is `~/.claude/plans/we-have-not-yet-pure-swing.md`. The operator's instruction is to
  run straight through, logging every judgment call in
  `docs/design_review/reviews/deviations_remaining-scope_2026-09-23.md` (D1–D23 so far) for
  review at the end.
- **Unpushed:** every commit since `origin/main`. Push only when asked.

## Last verified (2026-09-23)

| Command | Outcome |
|---|---|
| `just test-all` (at `083507f`) | passed: nextest 154/154, pytest 51/51 (lctx_mcp over the `just py-fixture` generation, raw-pipe stdio), rule tests, adr lint 19, deps, gold (with the analytics-config freeze) |
| `just pilot` | passed: snapshot `7799813f`, 5 briefs, generation `0c19ec176c80eb41`, stdio smoke passed; 31.4 s, peak about 3.65 GiB |
| `just pilot-live`, `just embed-conformance` (GPU, 1.9) | passed (snapshot `3d3aab40`; worst cosine 0.9999334). The service was stopped afterwards |
| §1.5 ranking check (live vectors, 1.9) | failed: the seed is first for 1 of 2 `fm.register` aliases. The fusion policy is now pre-registered (ADR-0010, D19); it is re-run at 3.3 |

## Known gaps

- **`build/`** is local. A schema migration makes an old store refuse, so move it aside;
  `build/store-slice1.6` keeps the 1.6 live vectors.
- **Brief content:** Applicable case and Usage pattern are absent in every brief (the manifest's
  `absent_slots`). They are filled by 2.2 and 2.5.

## Open decisions

- **ADR-0011** (analytics) stays `proposed` until the 2.3 Leiden spike.
- **Deferred rows** carry their triggers in their reviews: the deep review's O2–O6 and O9, the 1.5
  review's O7, and older C2–C6 and H1 rows. 2.1 fired C2 O2, C2 O4, C4 O3 and C6 O2 (noted in the
  2.1 commit), and the deep review's O7 (fixed).
- **`just adr revisit`**: none fired. ADR-0002's check passed.

## Next

Slice 2.2: Pass C, which finds `x = producer(); consumer(x)` handoffs and direct nesting in the
usage run's straight-line regions (C2/C3). Usage patterns are built from official examples and
tests. Then 2.3, communities and the ADR-0011 spike.
