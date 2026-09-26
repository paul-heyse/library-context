# Design review: first recursive summary engine

**2026-09-26 · design/target · standard.** Core standard 3.0,
code-intelligence profile 1.1 and library-context binding. Subject:
[ADR-0053](../../adr/0053-bounded-scc-summary-worklist.md), DESIGN §B4,
the [finite summary owner](../../design/sections/behavioral-analysis.md#section-9-9),
and [plan W12/order 6](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).
This reviews the engine decision, not a completed recursive producer.

## 1–5. Ownership, contract and change scenarios

Petgraph owns SCC discovery; `lctx-analytics::summaries` owns the semantic
value transfer, bounded BDD condition, ordered proof and typed refusal.
`cpg-core` acquires/publishes and its shared validator reconstructs the same
outcome; the native reader serves the persisted proof and unknown. Backend
relation tuples or SCC indices never become persistent semantic identities.

| Trigger | Required owner response | Evidence now |
|---|---|---|
| A recursive SCC has a cited finite base | Compose a finite value path with condition and ordered call/source proof | Ascent, datafrog and SCC worklist agree only on the base-origin reachability relation; full product proof is open |
| A self/mutual cycle has no finite base | Retain `call_transfer` unknown, never infer a negative from empty reachability | Same-state probe's no-base control passes |
| Pair work or BDD nodes exceed a cap | Publish a per-origin typed unknown, no partial positive borrowed from an unfinished path | Isolated SCC-worklist cap control passes; product Delta/native control remains open |
| Several effect/exception/role relations begin to share recursive rules | Recompare Ascent's rule/lattice facilities and datafrog's joins against the same proof/refusal contract | Revisit trigger in ADR-0053; no general rejection of either library |

The three engines agree on the finite reachability set under reversed inputs.
Ascent's macro supplies declarative recursive rules; datafrog supplies
semi-naive joins keyed by the first tuple element. Neither removes the
repository's BDD, provenance, budget and unknown-state responsibilities for
this **one** value relation. Reusing the accepted petgraph schedule gives the
smallest integration surface now, while a later multi-relation contract could
reverse that economy. The comparison did not measure speed or model actual
Python condition substitution; adopting the engine without those product
controls would overstate the evidence.

## 6–8. Judgments, gates and disposition

A1–A3 and FP-01–FP-06 are **satisfied for the ownership decision**: one
semantic producer remains authoritative, the change scenarios identify
which future variation can overturn the choice, and the pinned alternatives
were exercised on the same narrow state. Applicable DP-01/02/03/07/08/11/
13/15/16/21/23/24 and CI-01/02/03/04/05/06/08/11 are satisfied at this
decision boundary. G1/G2/G6/G8 and CI-G1 pass for the stated target and
probe; G3/G5/G7 and CI-G2 await the actual producer, shared validator and
served generation. CI-G3 is unchanged because the gold is not an input.
W12/order 6 retain the full semantic state, per-origin cap and cost actions;
the open work is recorded in the forward plan, not closed by this ADR.

**Tested 2026-09-26:**
`CARGO_TARGET_DIR=/home/paul/library-context/build/evidence-target cargo run --locked --manifest-path docs/design_review/evidence/2026-09-26_summary-engine-comparison/Cargo.toml --quiet`
passed set equality, reversed/no-base controls and a bounded unknown.
Pinned `ascent 0.8.1`, `datafrog 2.0.1` and `petgraph 0.8.3` are in the
isolated lock. Full product recursive fixtures, fresh pilot, `just fmt` and
`just test-all` are `not_run`.

**Decision:** accept ADR-0053 for the first recursive value channel only.
Keep the recursive-member refusal until the new producer passes its actual
condition, source-proof and publication controls.
