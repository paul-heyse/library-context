# Pysa (pyre-check 0.10.0) on FastMCP 4.0.5, 2026-09-24

**Question.** Does Pysa run offline on the pinned FastMCP, with Pyrefly types, and what do its
taint-in-taint-out (TITO) models look like?

**Facts verified.** Pysa lives at github.com/facebook/Pysa and ships in `pyre-check` (0.10.0,
2026-08-06, manylinux wheel), which depends on `pyrefly`. Pyrefly is the default type checker
(`--use-pyre1` opts out). Pysa picked up `~/.local/bin/pyrefly` (1.3.1) from `PATH`, so an oracle
must pass `--pyrefly-binary`.

**Run.** In a Python 3.12 venv: `uv pip install pyre-check==0.10.0`. The project dir holds `fastmcp/`
and `fastmcp_tasks/` copied from `build/envs/fastmcp/lib/python3.14/site-packages`, the file
`pyre_configuration.json` here saved as `.pyre_configuration` (its `search_path` is that
site-packages), and `models/taint.config`. Then
`pyre --noninteractive analyze --no-verify --save-results-to ../fm-out --infer-self-tito`.

**Measured.** 8.5 s wall, 702 MiB peak RSS; 2,051 models, 6,054 TITO ports, 4,412 with `obscure`
features. The same run on `fixtures/python/behavior_shapes` took 0.9 s (28 models).
`raw/` holds both `taint-output.json` files and the FastMCP run's log.

**Finding.** Neither `_parse_call_tool_result` carries `client_name` into its result, which
independently confirms the Stage 2 evaluation's fix #1.

**Decision.** ty stays the flow provider and Pysa stays the cross-check (operator, deviation B21).
