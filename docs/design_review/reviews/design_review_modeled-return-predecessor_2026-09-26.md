# Design review: modeled-return predecessor completion

**2026-09-26 · change/conformance · compact.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: the
finite modeled-return producer under [plan orders 1 and 4](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion).

## 1–5. Owner, scenario and alternatives

The source-ordered predecessor facts are owned by Ruff syntax, ty's call
regions and the pinned model application. `cpg-schema::behavior` now carries
the return's start byte into the typed direct-modeled and assignment-modeled
query-only seeds. The pure producer applies the existing predecessor-completion
check before admitting either modeled result and prefixes the resulting cited
steps to its canonical proof. The shared validator reconstructs that same
producer output.

Previously `opaque(); return cast(object, value)` could get a positive modeled
summary because the direct-return producer alone checked earlier calls. It
now retains `unsupported_control_flow` and publishes no positive summary.
`cast(object, 1); return cast(object, value)` cites the earlier call's
independent argument evaluations, import, target and normal-return model
before the returned model call. The return expression's own call is excluded
by its start byte and is checked by the separate modeled-call proof.
For `result = cast(object, value); cast(object, 1); return result`, the
assignment source call and later call both have ordered normal-completion
steps. Replacing the later call with `opaque()` blocks the assignment path.

Reusing the existing checked relation and pure producer avoids a second
evaluator and keeps bounded condition refusals as unknown. Local-wrapper
predecessors, path-specific boundary identity, nested
arguments and recursive composition remain open. No stored table schema or
append-only codebook changes; the derivation changes the compiler digest.

## 6–8. Judgment and disposition

DP-01/02/03/08/11/13/15/16/21/23 and CI-01/02/04/06/08/11 are satisfied
for the tested modeled-return case: a later model assertion does not certify
an earlier source call, and ordered proof identity retains both. G1–G8 and
CI-G1/CI-G2 pass within the tested boundary; CI-G3 is unchanged. No §B
decision changes and no ADR is needed.

**Tested 2026-09-26:**
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed real extraction, Delta, native positive/unknown controls and shared
step validation;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile pinned_identity_models_require_and_publish_their_real_formals --quiet`
passed the existing modeled-identity fixture;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet`
passed ten schema contracts. `just fmt`, `just test-all`, fresh
`just pilot` and structured evaluation remain `not_run` until functional
completion.
