# Status

_Updated 2026-09-22 by the handoff skill._

## Where we are

- **Increment 1** ("one complete path", DESIGN §1.2) has not started. The code is still the
  bootstrap: `crates/cpg-schema` (a placeholder) plus the pinned-family smoke test.
- **The extraction front end was pivoted** (ADR-0012, accepted; supersedes ADR-0006).
  - Pyrefly 1.3.1, with a visibility patch (`third_party/pyrefly-1.3.1.patch`), and ruff crates
    `=0.0.11` are linked in-process over one parse.
  - The fork is published: `github.com/paul-heyse/pyrefly`, branch `lctx/1.3.1`, at
    `b9f28575`.
  - DESIGN §4.2.1–§4.2.6 and §4.3 detail the driver, the mappers and the Arrow → DataFusion →
    Delta built-ins.
- **Spike evidence** is on branch `spike/pyrefly-inproc` (`5fb2eed`), worktree
  `../lc-spike-pyrefly`, summarized in `analysis/SPIKE_RESULTS.md`:
  - ADR-0012: S0–S7;
  - ADR-0009: P1–P4;
  - ADR-0010: E1–E3.
- **The embedding model is Qwen3-Embedding-8B** (operator decision, 4,096 dims), revision
  `1d8ad4ca`. It is in the local HF cache (15 GB) and served by vLLM 0.30.0 on the 5090.

## Last verified (2026-09-22, after ADR-0012 acceptance)

| Command | Outcome |
|---|---|
| `just test-all` | passed: 2 Rust tests, 10 Python tests, adr lint (12 records), lint-agents, family ok, cargo-deny ok |
| `just rules-scan` / `just fixtures-check` | not_run (no rules or fixtures yet) |
| spike `cargo test -p spike-pyrefly --test s6_delta` | passed (3/3; re-run by the reviewer) |
| spike build by `git`/`rev` on the fork + `check_family.py` + `cargo deny` + FastMCP run h | passed; output byte-identical to the local-clone runs |
| spike `cargo test --test p_publication` (ADR-0009 P1–P4) | passed (4/4) |
| spike `pytest analysis/mcp` (ADR-0010 E3), `py_client.py` + `embed_client` + `compare.py` (E1, E2) | passed (8/8; clients agree to cosine ≥ 0.9999) |

## Known blocks and uncommitted changes

- **`pyproject.toml` / `uv.lock`** (uncommitted, operator) add `fastmcp>=4.0.5` and
  `vllm>=0.30.0`. Increment 1 restructures them per ADR-0010 (vLLM as a separate service, pin
  `fastmcp<4.1` and `pyarrow`, the `lctx_mcp` package).
- **`.claude/skills/README.md`** has one uncommitted external hunk: the pyrefly-ruff row.
- **An asserted library claim not yet located:** petgraph's Bfs and Dfs visit siblings in
  opposite orders (§5). The ADR-0011 spike settles it.

## Open decisions

- **ADR-0012 is accepted.** Its review findings F1–F11 and O1 were applied in `1345e5a`, and the
  operator confirmed the four author decisions. Its oracles are slice-1 work (ADR-0012
  Consequences). Its revisit trigger is manual; a `just` recipe for fork identity and the env
  list is one of those oracles.
- **ADR-0009 and ADR-0010 are accepted** on their spike evidence.
  - ADR-0010 now names Qwen3-Embedding-8B.
  - ADR-0010's conformance oracle is identical request texts plus cosine ≥ 0.9995, because
    vLLM is not bitwise deterministic.
- **Still `proposed`:**
  - ADR-0008 (fact-slice schema), settled by slice 1's `cpg-schema` snapshots;
  - ADR-0011 (analytics), settled by its increment-2 spike.
- **Review deferred rows:** sidecar isolation (Option 4), dropping `State` before Stage C, and
  end-to-end cost. Each reopens on the trigger in the review's §11.

## Next

Increment 1, slice 1, from the spike branch:
- the workspace dependencies, and the `deny.toml` and `check_family.py` changes;
- `cpg-schema` families with insta snapshots;
- the ADR-0012 oracles.
