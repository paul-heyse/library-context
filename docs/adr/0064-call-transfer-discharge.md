---
id: ADR-0064
title: Grade call-transfer claims after summaries from claim-keyed discharge evidence
status: accepted
date: 2026-09-27
supersedes: []
superseded-by: null
design: [§3.9, §9.9]
evidence: Proposed
revisit: A served call-transfer claim cannot name its discharge evidence without re-deriving flow membership, or a non-return sink needs a proof object this relation cannot key.
---

## Context

[§3.9](../design/sections/behavior-model.md) makes every value that reaches a sink only through a
call `unknown` (`call_transfer`); `semantic:call-transfer-never-established` enforces it.
[§9.9](../design/sections/behavioral-analysis.md) accepts the target that a matching summary proof
discharges such a claim (ADR-0057/0058), but nothing implements it: behaviors are published
before summaries and never read them. So no L3 proof reaches a served verdict, and the pilot's
`call_transfer` count cannot move. The operator-approved remaining sequence schedules discharge
as P1 (plan §3.0). The [target review](../design_review/reviews/design_review_stage3-discharge-target_2026-09-27.md)
F01–F07 found five choices the accepted target leaves open:

- how a served claim cites its proof;
- how merged claims combine per-contribution grades;
- whether a proof discharges raw provider approximation;
- how decorated callees are refused;
- which rule replaces the unconditional one.

It also found a live defect: summaries compose through decorated callees (F02, confirmed with
`lctx compile-fixture` on `transferpkg.through_decorated`).

## Options

1. **Keep the unconditional rule.** It is consistent, but L3 never reaches a served answer and the
   Stage 3 exit cannot use it. Rejected.
2. **Per-origin decisions, joined later by each consumer.** This is the smallest schema. But the
   claims step, the SQL rule, native serving and Python would each re-derive which contributions a
   claim merged, partly from display text. Rejected (review F01).
3. **Claim-keyed discharge evidence produced with the claims, graded by one pure function
   (chosen).** Membership comes from the owner of the merge, and every consumer reads one relation.
4. **Split claims per contribution** (proved rows established, open rows unknown). More precise
   for mixed rows, but it churns claim identity and adds served rows. Deferred; revisit if a graded
   item is partial solely because of a mixed row.
5. **Discharge at serve time or in SQL.** A second semantic interpreter (T7) or a split
   classifier. Rejected.

## Decision

- **Order.** Summaries precede flow-claim grading. `attempt` publishes the relations summaries
  need, then finite summaries, then behavior claims, facets and documents. No summary input reads
  behaviors.
- **Decisions are pure.** `lctx-analytics::summaries::discharge` decides, per candidate origin,
  `proved` (an established/conditional summary with that `source_origin_id`) or `open` (with the
  summary refusal reason). It also grades a claim from its members' decisions and raw flags.
  Core supplies membership and applies the grade; it owns no verdict policy for this rule.
- **Evidence is claim-keyed.** `behavior_discharges` rows are keyed by snapshot, `behavior_id`
  and `origin_id`. Each carries a typed `proof_kind` (only `caller_return_summary` now), the
  cited `summary_id` when proved, and the reason when open. Membership comes from `flow_model`'s
  merge of contributions into value flows, accumulated across `claim()` merges.
- **All-members join.** A call-transfer claim becomes established/conditional only when every
  member origin is proved. It is graded after the merge, independent of insertion order, with a
  deterministic reason precedence otherwise. `transfer` stays `call`; the claim's identity is
  unchanged.
- **Approximation.** When every member is proved, those proofs discharge the members' raw value
  approximation. Summaries exist for raw-approximated origins only under an origin-specific
  certificate. Raw flags are unchanged. Condition approximation, capture, decoration and
  unreachability still apply afterwards.
- **Decorated callees.** The summary producer refuses a local target in `flow_model`'s decorated
  set (the one predicate, with the builtin descriptor exemption) as `outside_provider_model`. A
  decorated function has no base summary. This also protects path-local native inspection.
- **Rules.** `semantic:call-transfer-never-established` is replaced by: an established or
  conditional call-transfer claim has at least one discharge row, every row is proved, and the
  sibling closure holds. Validation rebuilds decisions from the finite outcome it already
  reconstructs.
- **Serving.** The generation carries `behavior_discharges`. Native admission checks that each
  cited summary exists, matches the origin, is established or conditional, and that a proved row
  cites one. Python renders the citation beside the claim.
- **Deferred.** Refutation and its coverage evidence are appended in P4 as ordered evidence,
  with `semantic:refuted-needs-complete-region` generalized. Argument and store sinks need their
  own proof kind (plan §7).

## Consequences

Verdicts of call-transfer claims now depend on L3; a summary defect becomes a served defect. That
is why the decorated refusal and the CPython challenge
(`tests/scripts/test_semantic_soundness.py`) accompany the change. At the 2026-09-27
checkpoint no pilot summary crosses a call (review O1), so P1's acceptance is fixture-level: a
discharged `local_wrapper`, an open sibling, a mixed row, a decorated callee, order invariance,
and a doctored generation refused natively. The pilot count moves only with P4/P5 producers.
Decision accepted; implementation and its change review are open in the
[plan](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution) (P1).
