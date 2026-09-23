# Status

_Updated 2026-09-22 by the handoff skill._

## Where we are

- **Increment 1** (DESIGN §1.2): slices 1 and 2 are done.
  - Slice 1: extraction (Pyrefly fork + ruff in-process), raw contracts, Delta persistence.
  - Slice 2 (`3448a50`, review fixes `792b8ea`): Stage C/D derivations (`provider_node_map`,
    `exports`, `signatures`, `parameters`, `resolutions`, `call_targets`), validators generated
    from the contracts plus semantic rules, the `compile` attempt (write → derive → validate →
    one `snapshots` append) and the pinned, snapshot-filtered reader.
- **Both slices had compact reviews** (`docs/design_review/reviews/…slice1…`, `…slice2…`). Every
  finding is fixed or deferred in the review's disposition section.
- **DESIGN.md has no line budget** (operator, 2026-09-22; ADR-0004 amendment): keep the detail.
- **Not pushed:** `792b8ea` is ahead of `origin/main`.

## Last verified (2026-09-22, at `792b8ea`)

| Command | Outcome |
|---|---|
| `just test-all` | passed: nextest 48/48, pytest 18/18, ast-grep rules scan and rule tests 4/4, adr lint (12), lint-agents, fixtures parse, family ok, cargo-deny ok, pyrefly-fork ok |
| harness equivalence vs the pinned Pyrefly CLI | passed (inside nextest; needs `uv` with the dev group) |
| review probe: slice-2 derivations and rules on FastMCP 4.0.5 (Python DataFusion 54 replay) | every rule passed (reviewer, 2026-09-22); not a repo test |

## Known blocks and gaps

- **No real-library run in the repo yet.** Every test uses fixtures; FastMCP 4.0.3 needs Stage A
  (acquisition) first. The reviews' probes used 4.0.5 from the dev install.
- **`pyproject.toml`** carries `fastmcp>=4.0.5` and `vllm>=0.30.0` as dev dependencies;
  increment 1 restructures them per ADR-0010 (vLLM as a separate service, `fastmcp<4.1`,
  `pyarrow`, the `lctx_mcp` package).
- **An asserted library claim not yet located:** petgraph's Bfs and Dfs visit siblings in
  opposite orders (§5). The ADR-0011 spike settles it.

## Open decisions

- **ADR-0007's revisit trigger fired in slice-1 code** (an analyzer answer changed without
  `context_id`; build-graph-dependent `fact_id`s). Both were fixed as implementation defects
  against its own decision. The operator decides whether it needs an amendment.
- **ADR-0008 is accepted** (`792b8ea`, evidence Tested). **ADR-0011** (analytics) stays
  `proposed` until its increment-2 spike.
- **Deferred review rows** (each with its reopen trigger in its review):
  - slice 2: an in-repo test of an ambiguous committed append; generating `REFERENCES` and the
    node/edge mapping from one declaration (first projection); calls inside an unbound `def`
    reading `missing_evidence`; `pysa_calls.caller_key` unchecked;
  - slice 1: hooks behind a dev feature, boundary provenance, the locator clamp, the harness
    `canon`;
  - ADR-0012 review: sidecar isolation, dropping `State` before Stage C, end-to-end cost.

## Next

Increment 1, slice 3: **Stage A for the pilot** (DESIGN §4.0): acquire FastMCP 4.0.3 (wheel
bytes plus the tag tarball, sha256-verified, locked manifest), build its analysis venv from its
own lock, derive `release_id` from the artifacts, and run `extract` + `compile` end to end, so
the first published snapshot is the real pilot. Then the analytics config and Pass A.
