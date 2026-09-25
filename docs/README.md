# Documentation

Start with the task, then read the owning contract and adjacent consumer. Expand scope when a
contradiction or dependency requires it; the whole corpus is not the default reading context.

| Task | Start here |
|---|---|
| Understand the system | [Architecture map](design/README.md) → relevant section |
| Resume work | [STATUS](../STATUS.md) → [Stage 3 plan](plans/behavioral-model-forward-plan_2026-09-24.md) |
| Change conditions, models or summaries | [Behavior model §3.9](design/sections/behavior-model.md) → [analysis §9.9](design/sections/behavioral-analysis.md) → affected serving contract |
| Change schemas or identity | [Binding decisions and identity in DESIGN](design/DESIGN.md) → [schema owner](../crates/cpg-schema/src/lib.rs) → governing ADR |
| Review a design or implementation | [Review skill](../.claude/skills/design-review/SKILL.md) → [repository binding](design_review/design_principles/binding/library-context.md) |
| Research a library capability | [Pins](pins.md) → [local capability skills](../.claude/skills/README.md) → existing research route |
| Understand an earlier decision/result | [ADR index](adr/README.md) → cited dated review/evidence |
| Maintain this site | [Publishing operations](publishing.md) → [documentation plan](plans/architectural-documentation-and-search_2026-09-25.md) |

## Authority and search

The architectural collection owns accepted contracts and labeled targets. ADRs own rationale;
reviews own dated assessments; plans own scheduled findings' current disposition; STATUS links
the current checkpoint. Executable declarations own detailed implementation contracts.

The site derives navigation and search. **Current** is the default: architecture, entry pages,
active standard/process, accepted decisions and selected work. **Reference** contains proposals,
pins and supporting guidance. **History** contains retired decisions and unselected dated work.
**Everything** searches all three. A search result's scope is a reading aid, not a truth verdict.
Evidence labels still apply. Local capability indexes are not bundled into the website.

## Historical recovery

Retired plans, reviews, evidence, research input and ADRs are not part of the working set
(ADR-0042); current owners carry every surviving obligation, so recovery is never a reading
prerequisite. To recover a deleted path:

- Find the commit that deleted it: `git log --diff-filter=D --name-only --format=%h -- '<path or glob>'`
  (for an ADR id, `'docs/adr/NNNN-*'`).
- Show its last content: `git show <commit>^:<path>`; list a deleted directory with
  `git ls-tree -r --name-only <commit>^ -- <dir>`.
- Raw evidence is in Git LFS: `git lfs fetch origin <commit>^ --include=<path>`, then
  `git show <commit>^:<path> | git lfs smudge`.
