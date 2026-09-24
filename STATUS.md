# Status

_Updated 2026-09-23 by the handoff skill (increment 2 close-out)._

## Where we are

- **Increments 1 and 2 are done** (DESIGN §1.2). Increment 2 added:
  - Pass B and parameter docs (2.1), Pass C and usage patterns (2.2);
  - Leiden communities with seed consensus, where ADR-0011 was accepted (2.3);
  - PageRank (2.4), FCA in a structural scope (2.5), and selection plus Related (2.6).
  - Its compact review (`design_review_inc2-compact_2026-09-23.md`, Revise small) is
    dispositioned at `5ccd38f`: direct usage now ranks selection, delegation PageRank is the
    `+pagerank` variant, FCA scopes are nodes, concepts are shared signatures (D37–D40).
- **Increment 3 is in progress.** 3.1 (embeddings in analytics) is done. So is the 3.2/3.3
  groundwork: `lctx compile --analytics <variant>` and the gold scorer `scripts/score_gold.py`.
  Next is 3.2: RCA and the extra community layers, behind variants.
- The plan is `~/.claude/plans/we-have-not-yet-pure-swing.md`. The operator's instruction is to
  run straight through, logging every judgment call in
  `docs/design_review/reviews/deviations_remaining-scope_2026-09-23.md` (D1–D40) for review at
  the end.
- **Unpushed:** every commit since `origin/main`. Push only when asked.

## Last verified (2026-09-23)

| Command | Outcome |
|---|---|
| `just test-all` (at `5ccd38f`) | passed: nextest 194/194, pytest 52, rule tests, adr lint 19, deps, gold (with the analytics and selection freezes) |
| `just pilot` (at `5ccd38f`) | passed: snapshot `a8591982`, generation `4a789baa`, 5 briefs, smoke 5/5; 35.1 s, peak about 3.9 GiB |
| `just pilot-live` (3.1, snapshot `89d3d4d0`) | passed: 957 doc links, 29/29 communities labelled; the vLLM service was stopped afterwards |
| §1.5 ranking check (live vectors, 3.1) | failed: `FastMCP.tool` is first for 0 of 2 `fm.register` aliases (ranks 4 and 2). The 3.3 evaluation judges it |
| First gold scoring (3.3 groundwork, generation `91828e4c`, lexical only) | mean best Jaccard 0.022, 4 of 22 families touched; hit@1 5/8, hit@5 6/8; span recall 2/157 |

## Known gaps

- **`build/`** is local. The disk is shared and at about 90%; `target/debug/incremental` and
  stale `target/debug/deps` can be regenerated.
- **Brief content:** the Applicable-case slot is absent on every brief by decision (U2; no
  input-or-mode source in v1). Usage patterns fill only where official code shows a handoff.
- **Pilot selection is idle:** the brief budget equals the five seeds, so nothing is selected
  until 3.4 raises the budget (an ADR-0004 amendment).

## Open decisions

- **The LLM-trigger ADR** (5.4) will stay `proposed` for the operator.
- **Deferred rows**, with triggers:
  - ADR-0011 review F6 and the property-getter row: rerun the SQL when 3.4 raises the budget;
  - FCA time on large scopes;
  - the deep review's O2–O6 and O9;
  - older C- and H1 rows.
- **The review's O2**: implications are the ablation candidate in 3.3.
- **`just adr revisit`**: nothing has fired. ADR-0017's "a relational diff" trigger is due at 3.3's
  `lctx diff`; check it there.

## Next

Slice 3.2 adds two variants, both off by default:
- RCA: ∃-scaled `calls X` and `hands off to X` attributes in the same FCA;
- the extra community layers: type, mention (C5 O1) and kNN.

Then 3.3: `lctx diff`, variant compiles and gold scoring (a)–(c), live vectors when the GPU is
free, and the keep rule.
