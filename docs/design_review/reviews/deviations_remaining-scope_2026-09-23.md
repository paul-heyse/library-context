# Deviations and judgment calls: the remaining-scope plan

The operator approved the remaining-scope plan (`~/.claude/plans/we-have-not-yet-pure-swing.md`,
2026-09-23) with one instruction: "run straight through all the way through the end without
stopping. If there is a necessary deviation, I trust your judgment call in order to get to
completion, just make sure to note them so that at the end you can share them for my review".

This log is that note. It is append-only, one entry per judgment call or deviation, each with
what was planned, what was done, why, and how to reverse it. It is evidence for the operator's
review, not authority: the decisions it records live in ADRs and DESIGN.

| # | Date | Slice | Planned (design or plan) | Done | Why | Reverse by |
|---|---|---|---|---|---|---|
| D1 | 2026-09-23 | Phase 0 | §12 / plan 5.1: the operator writes or approves the held-out evaluation tasks | A fresh subagent restricted to FastMCP's fetched docs (no briefs, gold, analytics config or design) writes about 20 tasks, fixtures with reference solutions, and expected calls into `eval/heldout/`, sealed by `MANIFEST.sha256` and committed before any brief exists | Straight-through execution; sealing before any brief exists keeps the set held out from what it evaluates | Replace `eval/heldout/` with operator-written tasks before increment 5 runs |
| D2 | 2026-09-23 | Phase 0 | DESIGN §1.2: hybrid retrieval (BM25, RRF, promotion, degraded mode) in increment 4 | Moved into increment 1 (slice 1.8), ADR-0004 amendment; increment 4 is the generation lifecycle and serving reliability | The §9.8 keep rule (increment 3) must judge techniques on the retrieval stack that is served; the spike already had it | Revert the ADR-0004 amendment and move the retrieval code behind a mode flag until increment 4 |
