# Pysa TITO rule control, 2026-09-25

**Question.** Can the pinned Pysa 0.10.0 and explicit Pyrefly 1.3.1 produce an independent
source-to-sink challenge with a real rule, verified models, and TITO ports whose control
comes out the other way? This is an oracle readiness probe, not the FastMCP differential.

**Inputs.** `probe.py` has direct identity, a one-call wrapper, and a constant-return
counter-control. `models/taint.config` joins one `Probe` source to one `Probe` sink with
rule 9001. `models/probe.pysa` marks the source and sink; it has no invalid `-> None`
annotation. A local `pyproject.toml` keeps Pyrefly from inheriting the repository's
different project include list. The source is byte-identical to
`fixtures/python/pysa_tito_shapes/probe/__init__.py`, which the compiler analyzes. Neither
the analyzed library nor `fixtures/python/` is executed.

**Reproduce.** Run the first three commands from this folder and the final Cargo command
from the repository root:

```bash
uvx --from pyre-check==0.10.0 pyre --noninteractive analyze \
  --pyrefly-binary /home/paul/.local/bin/pyrefly \
  --verify-taint-config-only
uvx --from pyre-check==0.10.0 pyre --noninteractive analyze \
  --pyrefly-binary /home/paul/.local/bin/pyrefly \
  --save-results-to raw/out > raw/analyze.log 2>&1
uv run --no-project --python 3.14.7 python check.py
RUST_MIN_STACK=33554432 cargo test -p cpg-core --test compile \
  pysa_tito_control_uses_the_same_source_as_finite_summary_fixture --locked
```

**Observed.** All four commands passed on 2026-09-25. `taint-metadata.json` reports zero
model verification errors. Pysa issued rule 9001 at the identity and wrapper sinks, but not
at the constant control. Its `probe.identity` and `probe.wrapper` models each have the exact
`formal(value, position=0)` → `LocalReturn` TITO port with no obscure-callee feature. The
compiler independently publishes finite `Parameter[value]` → `ReturnValue` paths for
`identity` and `wrapper`, none for `constant`, and validates the publication. The source
equality checks prevent drift between the two runs. The raw log confirms that Pyrefly used
this folder's configuration. Raw outputs are tracked through Git LFS.

**Boundary.** This is a tiny matched-source positive/control differential, not a claim of
general analyzer equivalence. Pysa silence is not proof of no flow. The Stage 3.4 pilot
differential still needs the pinned FastMCP source and compiler summary paths, with
`obscure` and unresolved paths classified separately. No generated oracle result is a
compiler input.
