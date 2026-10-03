# Code-facts opportunity: evidence maps (2026-10-03)

**Question.** Which additional pyrefly, ruff and ty code facts should library-context implement, and
how would its downstream layers use them? The operator assumes the migration to pyrefly 1.4.0-dev.3
with ruff 0.0.14 happens; feasibility is a later question.

**Consumer.** [design_review_code-facts-opportunity_2026-10-03](../../reviews/design_review_code-facts-opportunity_2026-10-03.md)
uses these maps as leads; it verifies its decisive evidence itself.

**How they were made.** Read-only static inspection of `main` at `6885dbf6` (clean tree) on
2026-10-03. Nothing was built, tested or probed, and the local store held no generations.

| File | Role | Author |
|---|---|---|
| [`supply-map.md`](supply-map.md) | What the pipeline extracts per family, from which provider or bespoke recognizer (B1–B14), how normalization consumes it, and linked provider getters that go uncalled | code-mapper |
| [`demand-map.md`](demand-map.md) | Fact-shaped gaps (G1–G13) in behavioral analysis, analytics and the catalog, with the verdicts and claims they limit, and the forward-plan §6/§7 items each would change | code-mapper |
| [`supply-catalogue.md`](supply-catalogue.md) | The 299 provider facts not consumed today: claim class, attachment basis, presence at pyrefly 1.4.0-dev.3 / ty 0.0.14, and access route | library-research, from the shared `python-analyzers` skill and pinned sources |

**Limits.**
- "Not consumed" in the catalogue follows the skill's library-context overlay. That overlay was
  wrong about ty's narrowing constraints, which are unconsumed (review O1).
- Provider behavior at ty and ruff 0.0.14 is unverified; item presence was checked.
- Pilot counts quoted (2,018 dispatch rows, 730 AMBIGUOUS rows) date from 2026-09-27, before the cutover.
