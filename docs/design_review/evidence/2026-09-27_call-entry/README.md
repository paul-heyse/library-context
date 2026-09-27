# Reached call inputs

**Tested, 2026-09-27; bounded S2b/S3a evidence.**
`runtime_oracle.py` executes 13 independently authored generated programs in isolated CPython
3.14.7 workers, with a two-second CPU bound, 512 MiB address-space bound and five-second wall
bound. It never executes compiler fixtures. `sys.monitoring.CALL` records calls from the
selected source to the real `gzip.compress`, `gzip.decompress` and `atexit.register`; the event
precedes invocation and proves neither return nor a modeled action. Registered callbacks are
removed before worker shutdown and are never invoked. Source, harness and gzip digests are in
`raw/runtime_receipt.json` (Git LFS).

Run: `uv run --no-sync python docs/design_review/evidence/2026-09-27_call-entry/runtime_oracle.py`.
Outcome: **passed**, 13 programs. Runtime-observed but unsupported nested calls and omitted
defaults remain explicit compiler unknowns; these are not negative execution claims.

The compiler test `call_execution_proves_reached_inputs_without_inventing_callee_completion`
compiles `fixtures/python/invocation_shapes` without executing it, compares its published reached
calls with this independent worker output, validates the shared proof contract and removes
published steps to challenge source-equality validation. Positive sites include calls before a
raise and calls that themselves raise; completion of the enclosing function is not a premise.
The 130-argument control retains its actual count and `invocation_argument_limit` refusal.

The focused schema/analytics/core selection passed 34 cases; the command is recorded in the
[current review](../../reviews/design_review_stage3-channel-contracts_2026-09-26.md).
Ordinary invocation evidence is not yet part of the native generation. Active/completed context
frontiers, nonconstant guards, nontrivial callee expressions and invocation-action/return semantics
are outside this receipt. The [forward plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md)
owns remaining scope and the separate encountered modeled-finalizer regression.
