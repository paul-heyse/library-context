# Pysa TITO rule control, 2026-09-25

**Question.** Can the pinned Pysa 0.10.0 and explicit Pyrefly 1.3.1 produce an independent
source-to-sink challenge with a real rule, verified models, and TITO ports whose control
comes out the other way? This is an oracle readiness probe, not the FastMCP differential.

**Inputs.** `probe.py` has direct identity, a one-call wrapper, and a constant-return
counter-control. `models/taint.config` joins one `Probe` source to one `Probe` sink with
rule 9001. `models/probe.pysa` marks the source and sink; it has no invalid `-> None`
annotation. A local `pyproject.toml` keeps Pyrefly from inheriting the repository's
different project include list. The analyzed library and `fixtures/python/` are never run.

**Reproduce.** From this folder:

```bash
uvx --from pyre-check==0.10.0 pyre --noninteractive analyze \
  --pyrefly-binary /home/paul/.local/bin/pyrefly \
  --verify-taint-config-only
uvx --from pyre-check==0.10.0 pyre --noninteractive analyze \
  --pyrefly-binary /home/paul/.local/bin/pyrefly \
  --save-results-to raw/out > raw/analyze.log 2>&1
uv run --no-project --python 3.14.7 python check.py
```

**Observed.** All three commands passed on 2026-09-25. `taint-metadata.json` reports zero
model verification errors. Pysa issued rule 9001 at the identity and wrapper sinks, but not
at the constant control. Its `probe.identity` and `probe.wrapper` models each have the exact
`formal(value, position=0)` → `LocalReturn` TITO port with no obscure-callee feature. The
raw log confirms that Pyrefly used this folder's configuration. Raw outputs are tracked
through Git LFS.

**Boundary.** This proves the oracle configuration can disagree on the tiny control. Pysa
silence is not proof of no flow. The Stage 3.4 differential still needs the same pinned
FastMCP source and the compiler's summary paths, with `obscure` and unresolved paths
classified separately. No generated oracle result is a compiler input.
