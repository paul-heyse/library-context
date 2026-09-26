# Typed completion controls — 2026-09-26

**Tested, bounded observation.** `probe.py` generates its own small programs and observes their
returns/unwinds with CPython 3.14.7 `sys.monitoring`, driven by Hypothesis 6.168.1. It does not
execute repository fixtures, consume compiler output or write analysis facts. The
[active plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md) S2/S7 owns the remaining
compiler/oracle comparison and integrated acceptance.

Command (**passed**, 2026-09-26):

```sh
HYPOTHESIS_STORAGE_DIRECTORY=/tmp/lctx-typed-completion-hypothesis timeout 30s uv run --no-sync python docs/design_review/evidence/2026-09-26_typed-completion/probe.py
```

The bounded 100-case run observed 18 exact catches, 13 superclass catches, 14 later matching
handlers, 14 re-raised then caught exceptions and 19 re-raises from an active handler's nested
finally. Ten nonmatches unwound with TypeError; twelve bare re-raises after the handler finished
unwound with RuntimeError. Normal cases preserved the pending return's object identity.
Each generated function emitted exactly the expected return or unwind event. Raw JSON and
expected syntax-warning output are in `raw/` through the repository's LFS rules.

This finite observation is not a proof for arbitrary exception instances, exception groups,
user constructors, custom metaclasses, named-handler cleanup, or dynamic class mutation. It is
not a completed generated-compiler comparison. The bounded implementation review separately
records the pinned Pyrefly source finding: report MROs omit self/object and lose the native
linearization-completeness flag; the compiler now acquires that public metadata directly.
