# Documentation

Start with the task, then read the owning contract and adjacent consumer. Expand scope when a
contradiction or dependency requires it; the whole corpus is not the default reading context.

| Task | Start here |
|---|---|
| Understand the system | [Architecture map](design/README.md) → relevant owner |
| Resume work | [STATUS](../STATUS.md) → [forward plan](plans/behavioral-model-forward-plan_2026-09-24.md) (queue, open findings, qualification boundary) |
| Change conditions, models or summaries | [Behavior model §3.9](design/sections/behavior-model.md) → [behavioral analysis §9.9](design/sections/behavioral-analysis.md) → affected serving contract → plan §6 items |
| Change schemas or identity | [Facts and identity](design/sections/facts-and-identity.md) → [schema owner](../crates/cpg-schema/src/lib.rs) → governing ADR |
| Change serving, retrieval or embeddings | [Synthesis and serving](design/sections/synthesis-and-serving.md) → [storage and publication §6.4](design/sections/storage-and-publication.md) |
| Review a design or implementation | [Review skill](../.claude/skills/design-review/SKILL.md) → [repository binding](design_review/design_principles/binding/library-context.md) |
| Research a library capability | [Pins](pins.md) → [local capability skills](../.claude/skills/README.md) → Context7, then the `library-research` route |
| Understand why a decision was made | Owner section's `> Decision:` line → [ADR index](adr/README.md) |
| Maintain this site | [Publishing operations](publishing.md) |

## Authority and search

The architectural collection owns accepted contracts and labeled targets. Current ADRs own
rationale and open choices; the active plan owns execution and the current disposition of
scheduled findings; retained reviews hold dated evidence for open findings; STATUS links the
current checkpoint. Executable declarations own detailed implementation contracts.

The site derives navigation and search from the retained working set. **Current** is the default:
architecture, entry pages, the active standard and process, accepted decisions and selected work.
**Reference** contains proposals, supporting reviews and evidence, pins and guidance.
**Everything** searches both. A search result's scope is a reading aid, not a truth verdict;
evidence labels still apply. Local capability indexes are not bundled into the website.

## Historical recovery

Retired plans, reviews, evidence, research input and ADRs are not part of the working set
(ADR-0042); current owners carry every surviving obligation, so recovery is never a reading
prerequisite. To recover a deleted path:

- Find the commit that deleted it: `git log --diff-filter=D --name-only --format=%h -- '<path or glob>'`
  (for an ADR id, `'docs/adr/NNNN-*'`).
- Show its last content: `git show <commit>^:<path>`; list a deleted directory with
  `git ls-tree -r --name-only <commit>^ -- <dir>`.
- The documentation migration removed its retired set in `df1ebd8`; its parent
  `f54b09d090e66abcee1fcc90d2005524defe4582` holds all of it (`git show f54b09d:<path>`).
- Raw evidence is in Git LFS: `git lfs fetch origin <commit>^ --include=<path>`, then
  `git show <commit>^:<path> | git lfs smudge`.
