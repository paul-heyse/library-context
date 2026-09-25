# Architectural documentation and search execution

**Accepted execution scope, 2026-09-25; implementation in progress.** Implements the
[proposal](../lightweight_architectural_documentation_and_search_proposal.md) under ADR-0041.
This plan owns documentation implementation dispositions. Stage 3 and the concurrent reasoning
reviews retain their existing owners; this work closes none of their product findings.

## Sequence and responsibility

| Packet | Owner and deliverable | Acceptance | State |
|---|---|---|---|
| P00 | ADR, DESIGN authority and this plan | Independent target review; preserve ADR-0040 | Complete |
| P01 | Shared section resolver; move §3.9 and §9.9 | Unique owners, fences/stubs, cross-file decisions, legacy fragments, immutable ADR tests | Complete |
| P02 | Root/docs/architecture entry pages | Task routes, ownership and target/implementation distinction | Complete |
| P03 | Stdlib publisher, site/book declarations | Deterministic discovery, titles, one output/source, bounded assets and truthful source links | Pending |
| P04 | Pagefind UI and transactional artifact | Current default, all scopes, heading links, root/prefix, offline fragments, failure preservation | Pending |
| P05 | Isolated commands/bootstrap and docs-only CI artifact | Single pins, no product dependencies, real build, SHA-pinned actions, no deployment | Pending |
| P06 | Instructions, skills, final review and handoff | Collection-aware bounded reading; focused checks; independent assembled review | Pending |

Dependencies: P00 → P01 → P02/P03 → P04 → P05 → P06. Source entry points need no website.
Implement small coherent commits; stage explicit paths and preserve concurrent edits.

## Contracts and deletion obligations

- DESIGN plus `docs/design/sections/` owns architecture. IDs survive moves; relocation headings
  retain historical fragments and point to the sole substantive owner. Delete duplicated prose.
- ADR lifecycle, the declared standard and current plans remain authorities. Derive navigation
  and search classification; never add a parallel implementation inventory or proof register.
- `docs/site.toml` owns publication choices and documentation pins; `docs/book.toml` owns renderer
  settings. Stage repository-relative paths under ignored `build/docs/`; publish `build/docs/site/`.
- Include evidence READMEs, not recursive raw artifacts or ignored capability corpora. Published
  targets become local HTML; tracked omitted targets link to the build revision; optional ignored
  capability references are explicitly local. Reject invalid intended targets.
- Discover current working sources, identify revision and dirty previews honestly, and replace a
  successful site only after rendering, indexing and offline link checks. Deleted pages disappear.
- Current search includes entry pages, architecture, active standard/process, accepted ADRs and
  explicitly selected work. Reference includes proposals/pins/templates; History includes retired
  material and unselected reviews/evidence. Everything clears the filter. No duplicate index pages.
- Local commands and a docs-only CI artifact are authorized. Public Pages deployment is excluded.
- Bootstrap alone installs tools/dependencies. Ordinary commands run offline without syncing the
  project. Full product tests, pilot, stores, native extensions and historical probe replay are outside scope.

## Verification

Focused tests cover resolver semantics, discovery/add/delete, title overrides and fenced H1s,
link rebasing and fragments, asset exclusion, missing/wrong tools, pipeline failures preserving
last output, and clean-checkout operation. Use real mdBook/Pagefind/lychee for integrated acceptance;
mocks may test failure branches only. Inspect browser search at `/` and `/library-context/`, keyboard
opening, scope reset/clearing and heading results. Record one search asset size observation, with
no permanent budget gate. Use the existing Ruff/pyrefly pins for changed Python only.

Commands: `just bootstrap-docs`, `just docs-test`, `just docs-check`, `just docs-serve`.
Remote CI requires a pushed workflow and a run; local validation cannot certify it. No push is
requested. Final review uses the existing design standard and leaves future benefits Proposed.

## Findings and outcomes

Target review: [Accept at Proposed strength](../design_review/reviews/design_review_architectural-docs-target_2026-09-25.md).
The section resolver now prevents sibling decision prose from satisfying governance; existing ADR
`design:` declarations were made explicit in their owners. A source comparison confirmed both
extractions preserve all normative prose and labels, with only heading level, anchors and link rebasing.

| Finding | Owner / current disposition | Closure |
|---|---|---|
| DOC-01 (assembled review) | Publisher staging; corrected | Frontmatter is stripped before locating owned heading IDs; regression proves anchors precede §1 and §2 correctly. |
| Browser result titles | Publisher metadata; corrected | Explicit attribute-backed title metadata; regression plus real browser result titles. |

Focused acceptance is in progress; final receipts follow after the assembled review and clean-checkout run.

## Donor basis

Read-only inspection of pse-arrow's uncommitted publisher/theme on 2026-09-25, based on
`1601b757`. Reuse its small mdBook/Pagefind composition, with repository-specific section, source
link, asset and isolation tests. Its implementation is not a qualified release or our evidence.
