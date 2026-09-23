# Status

_Updated 2026-09-22 by the handoff skill._

## Where we are

- **Increment 1, slice 1 is done** (DESIGN §1.2): `cpg-schema` (17 table contracts, codebooks,
  `IdHasher`), `cpg-extract` (Pyrefly 1.3.1 fork `b9f28575` + ruff `=0.0.11` in-process, one
  parse, the `lctx-extract` CLI) and `cpg-core` (Delta create/verify/append/read, read-only SQL).
  Commits `1f0737f`, `d1f6827`, `97b72ac`, `0bf6c2d`, and `3129102` for the review fixes.
- **Slice 1's compact review** (`docs/design_review/reviews/design_review_inc1-slice1-…`) said
  Revise. Its disposition section records F1–F9 fixed, a new N1 fixed, and O3–O5 and part of O7
  deferred. The fixes changed five table contracts (a schema migration, no stored data).
- **Spike evidence** stays on branch `spike/pyrefly-inproc` (`5fb2eed`), worktree
  `../lc-spike-pyrefly`: S0–S7 (ADR-0012), P1–P4 (ADR-0009), E1–E3 (ADR-0010).
- **Pushed** to `origin/main` (2026-09-22).

## Last verified (2026-09-22, at `3129102`)

| Command | Outcome |
|---|---|
| `just test-all` | passed: nextest 37/37, pytest 18/18, ast-grep rules scan and rule tests 4/4, adr lint, lint-agents, fixtures parse, family ok, cargo-deny ok, pyrefly-fork ok (one revision in `Cargo.lock`, the driver and pins; tag + patch; env reads classified) |
| harness equivalence vs the pinned CLI (`pysa_variants`, `unicode_bom`) | passed (inside nextest; needs `uv` with the dev group) |
| probe: import-cycle fixture at `NumThreads(8)`, 6 runs × 2 orders | identical to `Inline`; thread sensitivity not shown (review F6) |

## Known blocks and uncommitted changes

- **`pyproject.toml` / `uv.lock`** (committed at the operator's request) add `fastmcp>=4.0.5`
  and `vllm>=0.30.0`. Increment 1 restructures them per ADR-0010 (vLLM as a separate service,
  pin `fastmcp<4.1` and `pyarrow`, the `lctx_mcp` package).
- **`.claude/skills/README.md`** has one uncommitted external hunk: the pyrefly-ruff row.
- **DESIGN.md is 1,458 lines** against "about 1,450"; the review fixes were net zero.
- **An asserted library claim not yet located:** petgraph's Bfs and Dfs visit siblings in
  opposite orders (§5). The ADR-0011 spike settles it.

## Open decisions

- **ADR-0007's revisit trigger fired in slice-1 code.** An analyzer answer changed without
  `context_id` changing (review F2: site-packages content was not hashed). The same inputs gave
  different `fact_id`s depending on the build graph (N1: serde_json `preserve_order`). Both are
  fixed as implementation defects against ADR-0007's own decision, which already listed the lock
  digest, and are now Tested. The operator decides whether the ADR needs an amendment.
- **Still `proposed`:**
  - ADR-0008, until review §7 Q5 (a)–(d) hold: coverage-completeness and fact-reference
    validators; a Stage-C uniqueness query (the keys now exist); the node/edge mapping; one
    producer per provenance table.
  - ADR-0011, until its increment-2 spike.
- **Deferred review rows:**
  - slice 1: O3 (hooks behind a dev feature), O4 (boundary provenance), O5 (locator clamp) and
    O7 (harness `canon`);
  - ADR-0012 review: sidecar isolation, dropping `State` before Stage C, end-to-end cost.

  Each reopens on its trigger.

## Next

Increment 1, slice 2 (DESIGN §4.1 C–D, §6, §8):
- derivations (`exports`, `signatures`/`parameters`, `call_sites`/`resolutions`/`call_targets`);
- the generated validators, including ADR-0008's (a) and (b);
- `snapshots` publication and the pinned reader.
