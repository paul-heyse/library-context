---
id: ADR-0018
title: Corpus selection reads the fetched tree without following links, with globset syntax
status: accepted
date: 2026-09-23
supersedes: []
superseded-by: null
design: [§4.0]
evidence: Tested
revisit: A library's tree needs a symlinked directory's contents in its corpus (following links would then need its own containment rule); or globset's syntax admits a pattern whose meaning differs from the operator's reading.
---

## Context

ADR-0013 (with its C5 amendments) introduced `[tool.lctx.source]`: globs from the fetched tree's
root, where `**` spans directories and `*` stays within one name. The library-leverage review
(C2) found that the hand-written matcher and walk behind it had four defects:
- it followed symlinks: a probe read a file outside the tree, and a `..` link looped 40 levels
  deep;
- it read `?`, `[…]` and `{…}` as literals;
- it backtracked exponentially: 14 stars ran for minutes;
- it walked the tree once per glob.

The operator decided (2026-09-23) to accept globset's full syntax and to refuse a selected
symlink, naming it, rather than skip it. The H1 review (F4) asked that these choices be recorded
as a decision rather than an amendment; its F5 tightened the coverage test.

## Options

1. **Keep `*`/`**` only and refuse other metacharacters.** It keeps the old contract, but a
   hand-written matcher remains.
2. **Follow links, confined to the tree.** Contents behind a link would be selected. That needs
   a containment rule, and loop detection, of our own.
3. **Skip links silently.** It drops files the globs would otherwise select, with no error.
4. **globset syntax; no link followed; a link that could hold a selection refused unless an
   exclude covers it (chosen).**

## Decision

- **Syntax.** globset 0.4.20 with `literal_separator`: `*` and `?` within one name, `**` across
  directories, `[…]`, `{a,b}`. `documents`, `examples` and `tests` each have an `_exclude` list.
- **The walk.** walkdir, one pass per tree, dot-directories skipped, **no link followed**.
- **A symlink an include glob matches is refused**, naming it.
- **A directory link is refused when it could hold a selection:** it and an include glob's
  literal prefix (the path before its first metacharacter) lie one under the other. It is exempt
  only when an exclude covers it, meaning the exclude names the link itself, or matches any name
  under it. "Any name" is tested with two unrelated probe names, one nested. An exclude that
  matches only some names under the link (`docs/**/_*`) covers nothing.
- **A source tree** (`Release::from_tree`) refuses any link.
- **Each include glob must select a file** (C5 review F5), counted per glob in the one walk.

## Consequences

- **Tested:**
  - `symlinks_in_a_tree_are_refused_unless_excluded`: a selected file link; a directory link
    under `docs/`; a partial exclude that does not cover; a `tests_exclude` that does; a loop
    under an excluded path; a source-tree link;
  - `braces_classes_and_single_characters_select`;
  - `many_stars_match_in_linear_time`.
  - On the pilot tree the selection is identical: 148 documents, 125 examples, 427 tests, and the
    same corpus release id.
- **A library with a symlinked docs or tests directory** must exclude it explicitly, or the
  compile fails naming the link. That is intended: the operator chooses.
- ADR-0013's C2 amendment line points here.
