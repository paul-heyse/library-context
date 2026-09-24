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
