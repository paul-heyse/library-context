# Design review: ordered pass-only finalizer suites

**2026-09-26 · change/conformance · compact.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: the
source-cited return-frame witness under [plan order 2](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion).

## 1–5. Owner, scenario and alternatives

Ruff syntax owns the ordered direct `finalbody` statements. The
`return_exit_statuses` derivation admits a pending return only if every
controlling frame is safe; `return_exit_pass_steps` reconstructs every pass in
inner-to-outer frame order and source order within a frame. The finite producer
and shared validator use those steps to identify the resulting proof. The
single-pass status columns remain populated only when exactly one frame has
exactly one pass, so they cannot misrepresent a multi-step proof.

The positive case is `try: return value; finally: pass; pass`. A finalizer
with `pass; marker = 1` remains unresolved. A first-pass-only heuristic would
have falsely admitted the latter; duplicating whole exit analysis in the
producer would create two authorities. The changed query uses the existing
syntax and proof schemas without an append-only codebook change. It does not
extend to effectful finalizers, context managers, exceptional suppression or
handler fate.

## 6–8. Judgment and disposition

DP-01/02/03/08/11/13/15/16/21/23 and CI-01/02/04/06/08/11 are satisfied for
the bounded pass-only case: each admitted action has a source fact and order;
an effectful sibling blocks admission. G1–G8 and CI-G1/CI-G2 are satisfied
within this tested slice; CI-G3 is unchanged. The full exit/fate target remains
open in the forward plan. No §B decision changes and no ADR is needed.

**Tested 2026-09-26:**
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries --quiet`
passed the real extractor, Delta publication, ordered proof and shared validator;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finalizer_proof_round_trips_through_the_native_generation_reader --quiet`
passed the two-step FORMAT 8 native round trip.
`just fmt`, `just test-all`, fresh `just pilot` and structured evaluation are
`not_run` until functional completion.
