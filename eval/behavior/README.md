# Behavioural evaluation set (pre-registered)

`fastmcp-4.0.5.toml` holds 20 questions a coding agent might ask about FastMCP 4.0.5,
each with atomic target items. Q01–Q12 probe behaviour (controls, configuration,
settings, exceptions, lifecycle, dispatch, modes, inert parameters, execution,
callbacks); Q13–Q20 probe Stage-1 controls and documented handoffs.

**Provenance.** Every target was written from the library's own source
(`source_root`) and official docs (`docs_root`) before any behavioural output of the
compiler existed. No compiler output, store, generation or `.claude/skills/` gold
reference was consulted. Each positive item cites exact `path:line` ranges (relative
to `source_root`; `../` names a sibling package) and, where the docs state it, a doc page.

**Rubric.** Each item is judged per answer as `present`, `partial`, `absent`,
`incorrect` or `misleading`. Items with `polarity = "negative"` are tempting wrong
claims: they must NOT appear in an answer. Stating one scores `incorrect` (or
`misleading` if hedged); omitting it is the correct outcome. Outcomes are reported per
item, never as a percentage.

**Append-only.** Once committed, the file is append-only. New questions get new ids
(Q21, ...). Existing items are never edited or deleted; to correct one, add a new item
with `supersedes = "<old item id>"` and leave the old item in place.

**Served `unknown` answers (recorded 2026-09-27, before any Stage 3 generation's packet was
read).** An item whose only served answer is a claim with verdict `unknown` is `partial` when
that claim names the operation, parameter, callee or path the item describes and its boundary
reason does not contradict the item. An `unknown` that fails either condition does not count
toward `present` or `partial`. The negative-item rule is unchanged: a hedged or `unknown`
statement of a negative item is still `misleading`. The Stage 1 and 2 assessments applied an
unqualified form of this convention; it applies as written from Stage 3 on.

**Evaluation-only requests.** `fastmcp-4.0.5.requests.toml` lists extra served calls the
packet renders for a question: callees an agent would follow, and semantic queries. Requests
are never targets and are never scored; they carry no `claim` or `polarity`, and they cannot
alter a question, item or exit rule. The file is append-only in the same way as the question
set. A current-model packet runner is activated with the retained evaluation scope; the obsolete
bundle-based runner was retired during the semantic model cutover.
