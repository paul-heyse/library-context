# Status

_Updated 2026-09-22 by the handoff skill._

## Where we are

- **Increment 1** ("one complete path", DESIGN §1.2) has not started. The code is still the
  bootstrap: `crates/cpg-schema` (a placeholder) plus the pinned-family smoke test.
- **The extraction front end was pivoted** (ADR-0012 supersedes ADR-0006).
  - Pyrefly 1.3.1, with a visibility patch (`third_party/pyrefly-1.3.1.patch`, fork rev
    `b9f28575`), and ruff crates `=0.0.11` are linked in-process over one parse.
  - DESIGN §4.2.1–§4.2.6 and §4.3 detail the driver, the mappers and the Arrow → DataFusion →
    Delta built-ins.
- **Spike evidence** is on branch `spike/pyrefly-inproc` (`6b38f0b`), worktree
  `../lc-spike-pyrefly`, summarized in `analysis/SPIKE_RESULTS.md`. S0–S6 passed and S7 was
  measured, on FastMCP 4.0.3: 257 modules, CLI parity 257/257, 4.1 s and ~765 MB at `Inline`.

## Last verified (2026-09-22, at `1345e5a`)

| Command | Outcome |
|---|---|
| `just test-all` | passed: 2 Rust tests, 10 Python tests, adr lint (12 records), lint-agents, family ok, cargo-deny ok |
| `just rules-scan` / `just fixtures-check` | not_run (no rules or fixtures yet) |
| spike `cargo test -p spike-pyrefly --test s6_delta` | passed (3/3; re-run by the reviewer) |

## Known blocks and uncommitted changes

- **The fork `github.com/paul-heyse/pyrefly` is not published.** It is waiting for the operator's
  go-ahead. Slice 1 can't depend on it until `b9f28575` is pushed, which is `vendor/pyrefly` in
  the worktree.
- **`pyproject.toml` / `uv.lock`** (uncommitted, operator) add `fastmcp>=4.0.5` and
  `vllm>=0.30.0`. Increment 1 restructures them per ADR-0010 (vLLM as a separate service, pin
  `fastmcp<4.1` and `pyarrow`, the `lctx_mcp` package).
- **`.claude/skills/README.md`** has one uncommitted external hunk: the pyrefly-ruff row.
- **An asserted library claim not yet located:** petgraph's Bfs and Dfs visit siblings in
  opposite orders (§5). The ADR-0011 spike settles it.

## Open decisions

- **ADR-0012** is `proposed`. Its standard review returned Not Accept as written.
  - F1–F11 and O1 were applied in `1345e5a`, adopting the reviewer's recommended option for each
    of the four author decisions: abort on panic, `Overrides` as open candidates, the `__all__`
    detector, and immutable CHECKs verified at open.
  - Accept it once the operator confirms those four choices and the fork is published.
  - Its oracles are carried into slice 1 (ADR-0012 Consequences).
- **Still `proposed`, pending their spikes:** ADR-0008, ADR-0009, ADR-0010, ADR-0011.
- **ADR-0009's Delta probe:** the CHECK and cast parts ran in spike S6. The Binary-statistics,
  injected-failure, post-commit-error and bundle-rebuild parts remain.
- **Review deferred rows:** sidecar isolation (Option 4), dropping `State` before Stage C, and
  end-to-end cost. Each reopens on the trigger in the review's §11.

## Next

Get the operator's go-ahead to publish the pyrefly fork. Then run the remaining ADR-0009 and
ADR-0010 spikes. Then start slice 1 from the spike branch:
- the workspace dependencies, and the `deny.toml` and `check_family.py` changes;
- `cpg-schema` families with insta snapshots;
- the ADR-0012 oracles.
