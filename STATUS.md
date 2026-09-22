# Status

_Updated 2026-09-22 by the handoff skill._

## Where we are

- **The design is detailed for the capability-compiler target** (DESIGN.md, rewritten
  2026-09-22):
  - pilot: FastMCP 4.0.3, server-components surface;
  - synthesis is programmatic, with no LLM in v1;
  - the agent interface is FastMCP.
- **Increment 1** ("one complete path", DESIGN §1.2) has not started.
- **Code** is still the bootstrap: `crates/cpg-schema` (placeholder) and the pinned-family smoke
  test (ADR-0002, `Tested`).

## Last verified (2026-09-22)

| Command | Outcome |
|---|---|
| `just check` | passed: 2 Rust tests, 10 Python tests, adr lint (11 records), lint-agents |
| `just deps` | passed (run by the design reviewer) |
| `just rules-scan` / `just fixtures-check` | not_run (no rules or fixtures yet) |

## Decisions (see `docs/adr/README.md`)

- **Accepted:**
  - 0001 process;
  - 0002 dependency family;
  - 0004 stage-1 scope;
  - 0005 programmatic synthesis;
  - 0007 run contract and IDs.
- **Proposed until their spikes pass:**
  - 0006 analyzers and topology (supersedes 0003);
  - 0008 fact-slice schema;
  - 0009 publication;
  - 0010 agent interface and retrieval;
  - 0011 analytics algorithms.
- **Latest review:** `docs/design_review/reviews/design_review_capability-compiler-design_2026-09-22.md`
  (standard). F1–F12, O1–O3 and O5 are folded into DESIGN and the ADRs. O6 is below; O7 was
  handled by raising the budget to ~1,200 lines.

## Known gaps and uncommitted operator changes

- **`pyproject.toml` / `uv.lock`** (uncommitted, operator) add `fastmcp>=4.0.5` and
  `vllm>=0.30.0`. Increment 1 restructures them per ADR-0010:
  - vLLM moves to a separate service;
  - pin `fastmcp<4.1` and `pyarrow`;
  - add the `python/lctx_mcp` package;
  - fix the "No product Python" description (review O6).
- **`.claude/skills/README.md`** (uncommitted, external) updates the pyrefly-ruff row after that
  skill was rebuilt.
- **Context7** needs an MCP reconnect to pick up the new API key.
- **Two library claims in DESIGN are asserted, not located in a skill:**
  - Delta writes FixedSizeBinary as BINARY (§3.3);
  - petgraph's Bfs and Dfs visit siblings in opposite orders (§5).

  The ADR-0009 and ADR-0011 spikes settle them.

## Next: increment 1 spikes, then the slice

1. **ADR-0006:**
   - Pysa column units on non-ASCII source;
   - Pysa size and decode time on the fastmcp skill's cached captures (project files only);
   - sorted-output determinism;
   - a separate FastMCP 4.0.3 analysis venv;
   - `pyrefly dump-config` invariance.
2. **ADR-0009:** a local Delta probe covering
   - Binary stats and BinaryView reads;
   - an injected validation failure;
   - an error after the `snapshots` commit;
   - a byte-identical generation rebuild.
3. **ADR-0010:**
   - vLLM serving Qwen3-Embedding-4B on the RTX 5090;
   - Rust and Python clients agree on the conformance vectors;
   - a `Client(mcp)` round trip in both protocol eras, plus mismatch fixtures.
4. **Then slice 1:**
   - write the analytics config (the subsystem and the `fastmcp.FastMCP.tool` seed);
   - add `cpg-schema` families with insta snapshots and codebook tests;
   - build Pass A → brief → bundle → both tools;
   - run a `deep` review at the end of increment 1.
