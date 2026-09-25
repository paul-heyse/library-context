---
id: ADR-0020
title: The §9.8 keep rule applied: the default analytics
status: proposed
date: 2026-09-23
supersedes: []
superseded-by: null
design: [§B4, §9.4, §9.6, §9.7, §9.8, §12]
evidence: Measured
revisit: The increment-5 held-out evaluation, which is §9.8's unbiased check, attributes an agent failure to a removed technique's missing content (a Related operation, a shared signature or implication, a doc link); or a second library's ablation, under the same frozen rule, reverses a keep decision.
---

## Context

§9.8's **keep rule** (DESIGN; ADR-0005; pre-registered before any ablation score) keeps a
technique only if it **changes published output and improves the development metric §12(b),
hit@5 of `search_capabilities` on the gold task aliases, without lowering §12(a) or §12(c)**.
Otherwise the technique is removed by ADR. The gold used this way is a development set; the
increment-5 held-out evaluation is the unbiased check.

**The ablation ran on 2026-09-23.** Every variant was compiled
at the pilot's budget of 20 briefs with live vectors: Qwen3-Embedding-8B
from `just embed-serve`, and E0 and documents through the cache. Each was scored by
`scripts/score_gold.py --embedder vllm` and diffed against the default by `lctx diff`. Scoring is
deterministic: two rescorings of the default were identical. Hits are counted over all 44 gold
aliases, so the variants share one denominator; a family no brief touches cannot be hit.

| Variant | hit@5 | hit@1 | (a) mean Jaccard | families touched | (c) spans | briefs +/−/~ vs default |
|---|---|---|---|---|---|---|
| default (communities, FCA, kNN) | 14 | 7 | 0.0444 | 8 | 4 | — |
| `-communities` | 13 | 9 | 0.0430 | 7 | 5 | 12/12/7 |
| `-fca` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/14 |
| `-knn` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/20 |
| `+pagerank` | 13 | 8 | 0.0442 | 8 | 4 | 10/10/7 |
| `+rca` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/10 |
| `+type-layer` | 14 | 8 | 0.0509 | 9 | 6 | 3/3/17 |
| `+mention-layer` | 14 | 7 | 0.0444 | 8 | 4 | 0/0/18 |
| `+knn-layer` | 13 | 7 | 0.0444 | 8 | 4 | 1/1/19 |
| `-communities,-fca,-knn` | 13 | 9 | 0.0430 | 7 | 5 | — |

Per technique:
- **Communities** (with against without): hit@5 is 14 against 13, but (c) is 4 against 5. They
  improve (b) and lower (c): **not kept**. Without the community cap, selection takes more of the
  server surface (`add_tool`, `Tool.from_function`, `add_provider`), which touches one gold family
  fewer. It also recalls one more gold span, and hit@1 rises from 7 to 9.
- **FCA** (shared signatures, implications) and **kNN** (doc links, community labels): they change
  14 and 20 briefs, but hit@5 does not move. **Not kept.** By construction, neither enters the
  retrieval document or selection (their text is kept out of the retrieval document), so (b) cannot
  see them.
- **Variants off by default:** none improves hit@5. PageRank and the kNN layer lower it, and RCA
  and the mention layer leave it unchanged. The type layer leaves it unchanged (hit@1 8) and raises
  (a) and (c). **None is adopted.**

The §1.5 ranking check (`FastMCP.tool` first for both `fm.register` aliases) fails on both the
default (1 of 2) and the kept set (0 of 2). The keep rule does not read it.

Every difference in the table is one or two units, over 44 aliases and 157 spans. The rule has no
margin, and none is added after the fact: that would tune the rule on the scores.

## Options

1. **Apply the rule as registered (chosen).** Remove communities, FCA and kNN from the default,
   and adopt no variant.
2. **Add a noise margin now** (for example, a change of at most one unit counts as none). It would
   keep FCA and kNN out either way, but would keep communities. It loses: the margin would be
   chosen after seeing that communities fall on a one-unit trade, which is tuning the rule on the
   development set.
3. **Defer removals to the increment-5 agent evaluation**, which can see content that retrieval
   cannot (a Related operation, an implication, a doc link). It loses as the default: the rule
   was registered to settle keep decisions at increment 3, and the held-out evaluation is the
   check of this decision, not its replacement. It is the revisit trigger instead.
4. **Delete the code.** It loses: variants cost nothing when off, `lctx compile --analytics`
   reinstates any technique for the held-out check or a second library, and deleting reviewed,
   frozen kernels would make reversal a rewrite.

## Decision

- **The default analytics** are Passes A–C, direct usage (§9.5), seed selection and Stage F.
  `Techniques::default()` turns off `communities`, `fca` and `knn`, as well as `pagerank`, `rca`
  and the three layers, which were already off.
- **Removed means off by default.** The code, its tests and its frozen parameters stay, reachable
  as variants (`+communities`, `+fca`, `+knn`, …). A variant's label joins its config digest
  (§9.8), so a snapshot always says which techniques made it. Fixture tests exercise the kernels
  through an explicit `+communities,+fca,+knn` variant.
- **What the default no longer publishes:** Related lines and community labels, shared signatures
  and implications, doc links. Selection has no community cap, because there are no communities.
  E0 runs only when a variant reads it.
- **This decision is revisited** when the increment-5 held-out evaluation attributes a failure to
  a removed technique's missing content, or when a second library's ablation under the same frozen
  rule reverses it.

## Consequences

- Briefs are smaller, and the default compile skips Leiden (40 runs), FCA and E0. With a live
  embedder, Stage E no longer embeds passages and API texts.
- Seed selection has no diversity device. Its order is direct usage alone, which on the pilot
  concentrates the 15 added seeds on the server surface. A later technique that restores
  diversity must pass the same rule.
- ADR-0044's algorithm decisions stand as the variants' methods. This record decides only what
  runs by default. The earlier algorithm record's trigger ("a technique's ablation changes no
  published output after increment 3") did not fire: every technique changed published output. The keep rule's metric
  clause decided.

## Amendments

- 2026-09-24, ADR-0021 (the behavioral model):
  - **Brief retrieval no longer decides.** The keep rule judged techniques by brief retrieval
    (§12(b)). Under ADR-0021 a technique is judged by its consumer in the served model, through
    the pre-registered behavioral question sets.
  - **This record stands as the brief default's decision.**
  - **The planned deletion exit is paused.** A technique failing the rule would have been deleted.
    Communities, FCA and kNN instead stay as variants until a stage gives them a tool consumer
    (ADR-0021's criterion; the forward plan defers this decision to Stage 4).
  - The record stays `proposed` until those consumers are evaluated.
- 2026-09-24: the criterion that replaces this rule, and its
  exit, are ADR-0021's. A variant is judged at the stage that introduces its tool consumer.
