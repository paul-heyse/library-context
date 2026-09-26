# Design review: local-wrapper predecessor completion

**2026-09-26 · change/conformance · compact.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: the
finite local-wrapper producer under [plan orders 1 and 6](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion).

## 1–5. Owner, scenario and alternatives

`local_call_summary_flow_seeds` now carries the exact return statement start
from Ruff syntax. Before composing a cited unconditional callee summary, the
pure producer applies the same source-ordered predecessor check used by direct
and modeled returns. It prefixes any earlier call-completion steps to the
canonical wrapper proof. The returned local call starts inside the return
statement, so it is excluded from predecessor traversal and is supported by
its separate callee-summary step.

`cast(object, 1); return f0(value)` retains the earlier pinned normal-call
witness and the local callee summary. `opaque(); return f0(value)` publishes no
positive wrapper flow and retains `unsupported_control_flow`. Previously the
local-wrapper producer could admit the latter because it checked the callee
summary but skipped the earlier call. The shared producer/validator owns the
check once; a parallel local-only evaluator would split proof authority.

This does not establish conditional callee transfer, recursive composition,
effect or exception propagation. It changes no stored table schema or
append-only codebook; the query derivation changes the compiler digest.

## 6–8. Judgment and disposition

DP-01/02/03/08/11/13/15/16/21/23 and CI-01/02/04/06/08/11 are satisfied
for the tested wrapper path. The earlier source call retains its own
evaluation and normal-completion citations; an opaque call remains unknown.
G1–G8 and CI-G1/CI-G2 pass for this case; CI-G3 is unchanged. No §B decision
changes and no ADR is required.

**Tested 2026-09-26:**
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed the real extraction, Delta, native positive/unknown and shared
validator controls;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet`
passed ten schema contracts.
`just fmt`, `just test-all`, fresh `just pilot` and structured evaluation
remain `not_run` until functional completion.
