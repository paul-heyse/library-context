# Design review: bounded recursive value worklist

**2026-09-26 · change/conformance · scoped review.** Core 3.0, code-intelligence
profile 1.1 and the library-context binding from `standard.toml` apply. The subject is
the ADR-0053 value producer in `lctx-analytics::summaries::finite`, its typed
`summary_components` and local-call inputs, and the focused source-to-Delta control.
This is an author self-review of the bounded change. [Plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)
owns current completion status. Stage 3, conditional recursion and a real native cap trace
are outside this acceptance.

## Scope, responsibility and change scenario

The expected extension is a second local call inside one recursive SCC. The
`cpg-core` DataFusion acquisition selects a closed, definite one-argument target
and publishes `summary_components`; `lctx-analytics` alone composes the path and
chooses a refusal; the shared `cpg-core` validator reconstructs that producer's
outcome. The consumer sees a `summary_flows` row citing a source origin and a
callee summary, or a typed `summary_boundaries` unknown. No candidate call edge
by itself becomes a transfer proof.

The analytic's projection is one snapshot's attributed local call seeds grouped
by SCC and formal parameter. Component order is callee-first; within a component,
each newly admitted unconditional value path feeds exactly the seeds that name
its formal. Parallel call sites and origins remain separate. The worklist sorts
seed keys and pending `(seed, callee summary)` pairs, then sorts output rows by
persistent identity. It has no random seed. A path is a **may-path under the
source/model abstraction**, not universal normal execution. Depth eight and a
one-million-pair bound create explicit unknowns rather than a false/negative
answer. A queue-size check bounds pending allocation as well as executed work.
The pair cap has its own append-only `summary_pair_work_limit` reason (code 25);
the native executor recognizes that reason, but a producer-to-Delta-to-native
pair-cap trace remains open.

The source fact is Pyrefly/Pysa and Ruff/ty extraction at their pinned revisions;
the local seed is a derived relation; the summary and its ordered proof are
derived analysis, not provider assertions. `source_origin_id`, source fact,
callsite, target, condition and callee summary retain distinct identities. A
missing source row or conditional callee path remains unknown. This bounds
CI-01/02/03/04/06/08 and CI-G1 claims to the inspected value relation.

## Gates, findings and library fit

| Gate | Scoped judgment and evidence |
|---|---|
| G1–G3; CI-G1 | Pass for the pure relation: positive paths retain attributed source and ordered callee proof; base-free and open parallel origins remain unknown; shared publication validation reconstructs the producer. The source fixture confirms a conditional recursive seed without promoting it. |
| G4–G6 | Pass for the inspected boundary: all inputs are explicit, component order and pair/depth caps are deterministic, and the existing typed outcomes feed Delta. A fresh native cap trace is not established. |
| G7; CI-G2 | Pass for the narrowed claim: only an unconditional established callee value path composes. A conditional base cannot be asserted as a recursive transfer. Served claim closure beyond this value path is unassessed. |
| G8 | Pass for this first relation: the existing petgraph schedule, BDD `Diagram` decisions, typed rows and Rust collections supply the needed mechanisms. Ascent/datafrog's pinned same-state probe establishes reachability parity only; adopting either still leaves proof, condition and refusal ownership to this producer. |
| CI-G3 | Unaffected: the gold reference and evaluation results are not worklist inputs. |

**F01 — conditional recursive transfer is outside the implemented relation.** A
real `if value: return value; return f(value)` fixture yields a local seed and
conditional direct base but no recursive positive; the invocation-specific atom
substitution and bounded BDD conjunction required to decide it are absent. This
is a declared scope limit rather than an observed wrong positive. To close
order 6, the analytics owner must express argument-to-formal substitution,
publish any composed condition through the authoritative catalog, and prove a
real terminating and withholding source pair through Delta/native. Current
disposition: [plan W12](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).

FP-01–06 and applicable DP-01/02/03/04/08/11/12/13/16/18/19/21 are satisfied
for the scoped value worklist: acquisition, semantic transfer, validation and
serving have distinct owners; one bounded producer owns proof/refusal; new
same-kind call edges compose through its input relation. The broader mixed
value/effect/exception/role design is unresolved. No new generic engine or
production dependency is justified by the first value relation. Recompare
Ascent and datafrog at ADR-0053's multi-channel trigger.

## Verification and decision

**Tested 2026-09-26:** `cargo test -p lctx-analytics --lib summaries::finite
--quiet` passed 12 pure controls including self/mutual finite-base, base-free,
parallel open origin, input shuffle and low pair cap;
`cargo clippy -p lctx-analytics --lib --quiet -- -D warnings` passed;
`INSTA_UPDATE=no cargo test -p cpg-schema --test codebooks --quiet` passed
three cases after review and acceptance of the append-only snapshot;
`INSTA_UPDATE=no cargo test -p cpg-schema --test contracts --quiet` passed
ten cases after reviewing 37 generated boundary validators, each extended only
with code 25;
`uv run --no-sync pytest python/lctx_mcp/tests/test_native_semantics.py::test_native_open_boundaries_keep_sibling_origin_ids -q`
passed the native reason admission and origin control;
`RUST_MIN_STACK=16777216 INSTA_UPDATE=no cargo test -p cpg-core --test compile
nested_returns_need_an_uncontrolled_exit_before_becoming_value_summaries
--quiet` passed one real-source Delta compile with the conditional self-call
withheld. `just fmt`, `just test-all`, `just pilot`, a recursive positive from
real source, and native cap publication are **not_run** until functional scope
completion.

**A1 satisfied, A2 satisfied, A3 satisfied for the bounded relation. Accept
scoped** at Tested strength. The accepted ADR-0053 owner is implemented for
unconditional recursive value paths; F01 and the other order-6 channels remain
open. The enclosing Stage 3 architecture and integrated product qualification
are unresolved.
