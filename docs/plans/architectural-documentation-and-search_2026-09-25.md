# Architectural documentation and search execution

**Implemented and Tested locally, 2026-09-25.** Implements the
[proposal](../lightweight_architectural_documentation_and_search_proposal.md) under ADR-0041.
This plan owns documentation implementation dispositions. Stage 3 and the concurrent reasoning
reviews retain their existing owners; this work closes none of their product findings.

## Sequence and responsibility

| Packet | Owner and deliverable | Acceptance | State |
|---|---|---|---|
| P00 | ADR, DESIGN authority and this plan | Independent target review; preserve ADR-0040 | Complete |
| P01 | Shared section resolver; move §3.9 and §9.9 | Unique owners, fences/stubs, cross-file decisions, legacy fragments, immutable ADR tests | Complete |
| P02 | Root/docs/architecture entry pages | Task routes, ownership and target/implementation distinction | Complete |
| P03 | Stdlib publisher, site/book declarations | Deterministic discovery, titles, one output/source, bounded assets and truthful source links | Complete |
| P04 | Pagefind UI and transactional artifact | Current default, all scopes, heading links, root/prefix, offline fragments, failure preservation | Complete |
| P05 | Isolated commands/bootstrap and docs-only CI artifact | Single pins, no product dependencies, real build, SHA-pinned actions, no deployment | Complete; remote run not_run |
| P06 | Instructions, skills, final review and handoff | Collection-aware bounded reading; focused checks; independent assembled review | Complete |

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
| [DOC-01](../design_review/reviews/design_review_architectural-docs-assembled_2026-09-25.md#DOC-01) | Publisher staging; corrected | Frontmatter is stripped before locating owned heading IDs; regression proves anchors precede §1 and §2 correctly. |
| Browser result titles | Publisher metadata; corrected | Explicit attribute-backed title metadata; regression plus real browser result titles. |

Assembled review: [Accept after DOC-01 correction](../design_review/reviews/design_review_architectural-docs-assembled_2026-09-25.md).
A1–A3 are satisfied for the named documentation scenarios. Improved long-term reading and change
cost remains Proposed. Implementation commits: `b92857e` (authority/resolver/moves) and `821173f`
(publisher, commands, source routes and process integration).

### Qualification, 2026-09-25

| Command or observation | Outcome and scope |
|---|---|
| `just bootstrap-docs` | **passed**; declared binaries already installed at the expected versions, isolated pytest 9.1.1 prepared. No new compiler install. |
| `just docs-test` | **passed**; 31 focused ADR/resolver/publisher tests, including cross-file governance, frontmatter anchors, failure preservation and source transport. |
| `just docs-check` | **passed**; ADR lint (41 records), agent checks, real mdBook render, Pagefind index and lychee offline fragments. |
| `uv run --no-project --offline --no-python-downloads python scripts/docs.py build --base-url /library-context/` | **passed** after giving the offline checker a matching prefix mount. The earlier 404 base-link failure preserved the prior successful artifact. |
| Browser at root and `/library-context/` | **passed** in Chrome: keyboard opening, Current default, source titles, heading-result navigation, and prefixed assets/results. Root query `verdicts`: Current 20, Reference 6, History 106, Everything 132; clearing/restoring scopes and reopening Current worked. Counts are dated observations, not gates. |
| Disposable clean checkout of `821173f`, `GIT_LFS_SKIP_SMUDGE=1`, then `just docs-test` and `just docs-check` | **passed**; 31 tests, 204 canonical pages, 0 local link errors. No `.venv`, `target`, `build/store` or ignored capability indexes before/after. Tool/interpreter/test caches were prepared; no product dependencies installed. |
| Focused Ruff check + format check and pyrefly check | **passed** on `scripts/{adr,design_sections,docs,repo_paths,check_agents}.py` and `tests/scripts/test_{adr,design_sections,docs}.py`, using `uv run --no-project --offline --no-python-downloads`; existing tool pins unchanged. |
| `actionlint .github/workflows/docs.yml` | **passed**. Official action commit SHAs and relevant inputs inspected; same local commands, contents-read permission, hidden process pages included in artifact. |
| `git diff --check` | **passed** for this scope. |
| One-time Pagefind asset observation (stdlib file sizes after final local build) | **Measured**: 211 canonical pages; 269 generated search files totaling 3,670,672 bytes on disk. This includes optional bundled UI assets; it is not initial browser transfer size or a permanent warning threshold. |
| Remote workflow / artifact upload | **not_run**: workflow has not been pushed; local qualification is not a remote receipt. No Pages deployment configured. |
| `just test-all`, `just pilot`, historical probe reruns | **not_run**: product qualification and unrelated historical evidence are outside documentation scope. |

The source comparison used the pre-migration DESIGN at `90cda89`: both moved sections retained
normative text and labels after normalizing heading level, supplemental anchors and relative links.
All duplicate normative prose was removed from the old locations. Stable source anchors and legacy
heading fragments remain usable. Generated navigation, staging and index assets are ignored.

The bootstrap and build consume the same tool declarations; pytest and Python retain their
existing owners. Primary integration references: [mdBook](https://rust-lang.github.io/mdBook/format/configuration/renderers.html),
[Pagefind metadata](https://pagefind.app/docs/metadata/), [lychee](https://lychee.cli.rs/guides/cli/),
[uv scripts](https://docs.astral.sh/uv/guides/scripts/),
[artifact upload](https://github.com/actions/upload-artifact), [setup-uv](https://github.com/astral-sh/setup-uv).


## Donor basis

Read-only inspection of pse-arrow's uncommitted publisher/theme on 2026-09-25, based on
`1601b757`. Reuse its small mdBook/Pagefind composition, with repository-specific section, source
link, asset and isolation tests. Its implementation is not a qualified release or our evidence.
