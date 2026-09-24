# Entry-value to guard stability probe

**Question (2026-09-24).** Which existing flow facts can support a link from an
operation's entry formal to a later source guard, and where must the link stay
unknown? This is a focused design probe for proposed ADR-0025
`flow_test_value_links`, not a product proof producer.

Run from the repository root using the already-built developer CLI:

```bash
target/release/lctx flow docs/design_review/evidence/2026-09-24_entry-value-bridge/probe.py
```

The command **passed** and returned `uses`, `definitions`, `reaching` and
`regions` JSON. The compact observations below use zero-based `use_ix` and
`def_ix` from that output; byte spans are relative to [probe.py](probe.py).

| Case | Guard use and reaching evidence | Interpretation |
|---|---|---|
| `direct(mode)` | `mode` at `25..29`, use 0, reaches parameter definition 4 under `true` | Candidate direct entry-to-test link, subject to the exact leaf/operand join and query value model. |
| `rebound(mode)` | `mode` at `116..120`, use 1, reaches assignment definition 6 under `true`; the formal is definition 5 | No entry-value identity after the explicit rebind. |
| `after_call(mode, callback)` | The second guard reads `mode` at `247..251`, use 4, and still reaches formal definition 7 under the first guard's condition | A static reaching edge alone cannot certify stability through the intervening `callback()` or exact equality semantics. The first direct-path implementation withholds this link. |
| `changed_closure(mode)` | The second guard reads `mode` at `439..443`, use 7; the rows include formal definition 9 and an unknown definition under the first guard's condition | The `nonlocal` write in `change()` prevents a unique entry-value proof. |

The region for `after_call`'s inner return contains distinct site atoms for
`mode == "sse"` (`202..215`) and `mode == "http"` (`247..261`), rather than
collapsing the two evaluations. `changed_closure` likewise preserves distinct
sites (`396..409`, `439..453`). These are Stage 2 may-conditions, not evidence
that both equality tests can execute for an exact string value.

**Proposed link rule.** A positive `direct_parameter_reach_no_effect` row needs
one exact `flow_test_leaves` → `flow_uses` tested operand, one same-snapshot
entry formal and reaching witness, and a checked no-rebind/no-unmodeled-effect
path between entry and the guard. An explicit rebind, unknown reaching source,
unmodeled call or write, ambiguous alias, or unresolved equality model yields
no positive link. A later `modeled_identity_transfer` can cross a call only
with a cited summary and effect-model revision. The query returns `unknown`
where the link or its semantic comparison is unproved.

This probe establishes the conservative cases the producer must distinguish.
It does not test the proposed relation, validator, native query or FastMCP
result.
