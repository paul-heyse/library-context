# Shared worker contract

Use applicable instructions already supplied in context; if AGENTS.md is absent, load it before
acting. Load the common and assigned role contracts once, plus relevant workflow or library skills.
The root's general startup tour is not repeated by workers: use the brief and relevant owners,
following additional dependencies when evidence requires it. All permission, preservation and test
rules still apply; named files are not a restriction on necessary read-only investigation. The coordinator's
brief bounds the assignment and permitted effects; a role contract may grant standing write
scopes, which a brief can narrow. Resolve paths from the repository root; Codex
discovers skills through `.agents/skills`, and their tracked source is `.claude/skills`.

Preserve concurrent work. Do not commit, push or delegate further unless assigned. Report a missing
prerequisite, conflicting contract or material discovery beyond scope to the coordinator; continue
independent in-scope work where useful. Make ordinary local decisions within the brief yourself.

Return the result with precise source locations or artifact pointers, the baseline examined,
material uncertainties and anything unfinished. For consequential negative claims, state the search
scope, versions or paths covered and limitations; an empty search does not establish absence. Distinguish observed facts from interpretation.
For edits, identify changed ownership or behavior and deletions. For checks, give commands and
`passed` / `failed` / `blocked` / `not_run`, retaining raw evidence where useful. Keep the response
compact enough for integration without hiding consequential details.

Follow AGENTS.md's test timing and pinned tool routes. Formatting and generators belong to the
end-of-turn hook. Non-functional checks (`just hygiene`) run once at scope end, by the integrator
unless the assignment includes them.
