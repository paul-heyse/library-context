# Default availability control

**Tested, 2026-09-27:** `uv run python docs/design_review/evidence/2026-09-26_default-availability/probe.py`
passed eight mutation cases, twelve fresh/default override cases, a definition-once event
control and two unused-default raise/skip controls in independent CPython 3.14.7 snippets; [raw output](raw/results.json).

An unused positional or keyword-only default remains part of runtime argument binding. Removing
`__defaults__` or `__kwdefaults__` makes an omitted argument raise `TypeError`; supplying that
argument explicitly preserves identity. A syntactic source default therefore cannot alone certify
availability at a later call. The probe executes its own snippets, never repository fixtures.

This is a runtime counter-control for S2, not an exhaustive equivalence proof or a generated
compiler/oracle comparison. Source/Delta/native withholding and explicit-argument controls live
in `finite_depth_and_unsupported_refusals_reach_the_native_response`; the
[forward plan](../../../../docs/plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution)
owns the compiler qualification and remaining Stage 3 work.
