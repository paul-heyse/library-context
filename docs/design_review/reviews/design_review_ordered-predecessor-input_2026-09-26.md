# Design review: ordered predecessor input and proof

**2026-09-26 · change/conformance · compact.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject: finite
source-to-return summary input order under [plan order 1](../../plans/behavioral-model-forward-plan_2026-09-24.md#32-functional-completion).

## 1–5. Owner, scenario and alternatives

The pure finite producer owns predecessor traversal and canonical proof order.
It previously trusted the acquisition layer to sort `preceding_calls`, then
stopped scanning on the first call after the return. A shuffled later call
could therefore hide an earlier unproved call and create a positive flow.
The producer now sorts by source byte and fact id before that scan; it also
sorts finalizer-pass inputs by declared ordinal and fact id before proof
hashing. The acquisition layer may still sort its rows, but safety and
canonical identity no longer depend on that behavior.

The real positive case has two pinned, independently evaluated calls before
`return value`; the ordered proof retains both. A second case makes the first
call complete normally and gives the second a raising expression, which must
leave `unsupported_control_flow`. The pure adversarial case supplies a later
call before an earlier unproved one. Sorting in the producer is smaller than
replacing the relation or assuming a SQL row order. The broader
path-specific predecessor relation and origin-identity migration remain open.

## 6–8. Judgment and disposition

DP-01/02/03/08/11/13/15/16/21/23 and CI-01/02/04/06/08/11 are satisfied
for this bounded case: the producer deterministically processes the same
source facts, every admitted call has ordered evaluation and completion
steps, and the unresolved sibling cannot borrow the earlier proof. G1–G8
and CI-G1/CI-G2 pass for the tested cases; CI-G3 is unchanged. No §B
decision changes and no ADR is needed.

**Tested 2026-09-26:**
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p lctx-analytics --lib shuffled_ --quiet`
passed three focused order controls;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test bundle finite_depth_and_unsupported_refusals_reach_the_native_response --quiet`
passed real extraction, Delta publication, two-call proof and native positive/
unknown queries. `just fmt`, `just test-all`, fresh `just pilot` and structured
evaluation are `not_run` until functional completion.
