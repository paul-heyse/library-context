# Design principle audit

**Audit not_run — scaffold only, 2026-10-20.** No codebase audit has been performed for this
collection. There is no audited source baseline or finding set yet; this state establishes no
alignment claim. The first audit is separate work.

The [audit skill](../../../.claude/skills/design-principle-audit/SKILL.md) maintains one living
account of current design principle misalignment. The
[repository binding](../design_principles/binding/library-context.md#design-principle-audit)
owns its place before a design-review engagement. Ordinary implementation reads its plan and
architectural owners; this collection is not default production-editing context.

## Reading and organizing the account

The first audit will replace the scaffold state with its date, audited source commit, any included
uncommitted changes, applied standard versions, and meaningful coverage and evidence limits.
Distinguish the source baseline from the commit publishing the account. Subsequent reviews cite
the identified audit revision and stable finding anchors.

Start with these reading categories, adding pages when findings make them useful. They are
organizational aids, not mutually exclusive classifications or additional design criteria.

| Topic | Suggested page | Main concerns |
|---|---|---|
| Semantic model and authority | semantic-model-and-authority.md | Model adequacy, authoritative meanings, independent consumer interpretation |
| Structure and invariants | structure-and-invariants.md | Distinctions, relationships, state, lifecycle, configuration and enforcement |
| Boundaries and composition | boundaries-and-composition.md | Responsibilities, contracts, dependencies, orchestration and library delegation |
| Local reasoning and assurance | local-reasoning-and-assurance.md | Required context, effects, isolation, diagnostics and independent verification |
| Execution fit | execution-fit.md | Necessary and repeated work, crossings, expansion, reuse, resource and recovery scope |

Give each finding one primary home and all relevant FP/DP/profile references. Its component
footprint can be narrow or broad. Document boundaries may evolve as the findings reveal useful
clusters; a principle or component does not need its own file. The
[writing guidance](../../../.claude/skills/design-principle-audit/references/audit-writing.md)
explains useful finding content and illustrative shapes.

## Ownership and history

This collection owns observations at its stated baseline. Reviews own proposed corrections and
alternatives; implementation plans own scheduled disposition and closure evidence; architectural
owners and ADRs own accepted design. Link to the relevant owner without reproducing progress.
Existing source-review finding IDs and evidence retain their original meanings and dates.

Refresh in place before a new review engagement, using prior observations and intervening changes
to focus current source inspection. Reconsider unchanged code when relevant dependencies or
expectations have changed. Preserve finding identity and material lineage; remove resolved
material when its correction is established. Git preserves earlier accounts and the removal
commit identifies closure evidence. No separate audit archive or execution register is needed.
