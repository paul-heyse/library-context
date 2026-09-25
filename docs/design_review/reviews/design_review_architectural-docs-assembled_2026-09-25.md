# Assembled review: architectural documentation and search

## 1. Scope, outcome and coverage

| Field | Value |
|---|---|
| Subject | ADR-0041 and the assembled documentation changes over `90cda89`, inspected in the shared dirty tree on 2026-09-25; DOC-01 correction inspected before this review closed |
| Standard | Core 3.0, code-intelligence profile 1.1 and repository binding, loaded through `standard.toml` |
| Tier · purpose | Design · target; independent final assembled review |
| Reviewer | Independent `docs_assembled_review` agent |
| Outcome sought | Bounded source reading, durable architectural citations, useful static search and isolated publication during frequent design changes |
| Examined | Section resolver, ADR governance, source moves and routes, publisher discovery/transport/publication, Pagefind integration, commands, CI and process instructions |
| Evidence strength | Implemented from source inspection; Tested for the focused regressions below; artifact observations cover the existing local build only |
| Excluded | Product code conformance, historical probe truth, Stage 3 completion, public deployment, concurrent publishers and process-crash recovery |

The [execution plan](../../plans/architectural-documentation-and-search_2026-09-25.md) owns terminal
build/browser/clean-checkout receipts and finding disposition. This review ran no publisher build
while the implementation agent exercised the real browser and deployment prefix. It does not
convert that agent's unfinished acceptance work into a receipt. Existing reasoning reviews and
their product findings remain outside this review's execution scope.

## 2. Responsibilities, dependencies and semantic ownership

| Owner | Responsibility and consumer contract | Change boundary |
|---|---|---|
| Architectural Markdown collection | Enduring contracts and labeled targets; stable § identities | Meaning changes through the relevant ADR; file moves preserve identity and old entry fragments |
| `scripts/design_sections.py` | Resolve one substantive owner per ID and logical governing ancestry | ADR lint and generated section directory consume the same identity map |
| ADR records, standard manifest and plans | Decision lifecycle, active standard paths and execution disposition respectively | Publication reads these owners; it does not maintain another progress or truth register |
| `docs/site.toml` | Collection selection, exceptional titles, current-work choices, URLs and binary pins | A normal page addition is discovered; an exceptional publication choice changes this declaration |
| `scripts/docs.py` | Compose discovery, staging, rendering, transport annotation, indexing, checks and replacement | Repository-specific interpretation stays in ordinary functions; mdBook/Pagefind/lychee supply generic capabilities |
| `scripts/repo_paths.py` | Recognize declared optional local capability references | Agent checks and publication agree about clean checkouts without those ignored corpora |
| Theme, commands and CI | Present search and run the same isolated pipeline | UI changes stay in the theme; automation invokes the local commands and uploads their result |

Dependency direction is source declarations → section/page interpretation → staging → library
outputs → checked artifact. Source files never import publisher state. Product schemas, compiler
packages, stores and native extensions do not enter this path. The architecture map names useful
owners without turning navigation into an exhaustive implementation inventory.

**Profile applicability:** no code fact family, graph analysis, behavioral synthesis or product
served answer is introduced. The fact/fidelity table and analysis-record additions therefore do
not apply. Search scope is explicitly a reading aid; it does not relabel historical prose as a
verified implementation claim.

## 3. Contracts, constraints and testing boundaries

| Contract | Enforced boundary and failure | Evidence |
|---|---|---|
| One substantive owner for a section ID | Resolver rejects duplicates, excludes fenced examples and relocation stubs; ADR lint rejects unresolved governed references | Resolver/ADR tests, including moved owner and inherited decision cases |
| Governing decision stays meaningful after a move | Logical ancestry spans files; `own_lines` prevents a sibling/child decision from satisfying its parent | Inspected resolver and positive/negative fixtures |
| One canonical page per selected source | Discovery rejects duplicates, missing configured pages, missing titles and omitted active-standard documents | Publisher tests |
| Source transport remains truthful | Published targets remain local; omitted tracked paths use the revision; ignored or uncommitted local references are labeled; missing intended targets fail | Link tests and `rewrite_links` inspection |
| Only complete successful candidates replace output | Tool versions checked before work; fresh staging; mdBook, Pagefind and offline fragment checks precede replacement | Failure fixtures preserve the prior site for each external stage |
| Documentation work is isolated | `uv --no-project`, existing Python/pytest pins, offline ordinary commands and explicit test files | Command construction test; focused suite executed without product dependencies |

Single-process preview is the declared lifecycle. The publisher does not claim atomic reader
generation pinning or crash recovery; `docs/publishing.md` tells the operator to run one publisher
at a time. Those constraints are proportionate to the accepted local-artifact use.

## 4. Composition and execution

Discovery derives scope from ADR lifecycle and the active standard, with explicit current-work
selection. Staging retains repository-relative paths, derives SUMMARY and the section directory,
and supplies renderer configuration. The renderer owns Markdown/HTML behavior. HTML annotation
owns transport policy and Pagefind metadata; the indexer owns search. Lychee checks local links
and fragments before the successful candidate replaces `build/docs/site/`.

These are inspectable stages with narrow data flow, not a plugin framework. Navigation, print/404
pages and the landing alias do not become duplicate search documents. The preview server serves
the finished artifact without implicitly rebuilding or reindexing it.

## 5. Change and failure scenarios

| Scenario | Owning change and affected consumers | Observed locality / settling evidence |
|---|---|---|
| Add or delete a review/evidence README | Add/remove source; collection declaration already selects it | Discovery fixture adds/deletes a page deterministically; raw evidence stays excluded |
| Move a governed architectural section | Move prose, retain ID/decisions and historical pointer | One shared resolver serves ADR lint and directory; historical ADR metadata remains unchanged |
| Add scalar frontmatter to an architectural owner | Source title metadata should not move its section targets | DOC-01 exposed stale source offsets; corrected staging uses the resolver-owned ID at its body heading |
| Retire an ADR or select a current plan | Change its existing lifecycle owner, or publication's reading selection | Scope follows that source; no duplicate status table is introduced |
| Read a source link in a clean or dirty checkout | Publisher distinguishes local HTML, revision-pinned source and unavailable local references | Nested/source/local-reference tests exercise those distinctions; dirty artifact banner states the revision limit |
| Change search presentation or binary version | Theme or declared tool pin; rerun focused docs checks | Generic indexing remains Pagefind-owned; a wrong/missing binary is rejected with bootstrap guidance |
| Renderer/indexer/link check fails, or page is deleted | Discard candidate; preserve last success, or replace with complete new tree | Failure tests for all three stages and stale-page deletion test |

The section-move comparison against `HEAD:docs/design/DESIGN.md` found preserved normative text
for §3.9 and §9.9 after accounting for heading level, stable anchors and three necessary evidence
link rebases. This observation concerns relocation fidelity, not the truth of the moved claims.

## 6. Correctness and fidelity gates

| Gate | Verdict | Evidence / scope |
|---|---|---|
| G1 Authority | pass | One section resolver; existing lifecycle/standard/disposition owners; generated outputs stay under ignored build storage |
| G2 Semantic fidelity | pass after DOC-01 correction | Moved text preserved; stable anchor target regression; scope is a reading classification rather than an evidence label |
| G3 Validity | pass for inspected contracts | Duplicate/missing sections, invalid selections/targets and binary mismatches reject; focused regressions exercise meaningful boundary failures |
| G4 Hidden behavior | pass | Installation is explicit bootstrap; normal docs commands isolate Python and do not run the compiler, stores or historical probes |
| G5 Consistency and recovery | pass for declared single-publisher lifecycle | Fresh candidate and checks precede replacement; failed stages preserve the last artifact; concurrency/crash guarantees are explicitly excluded |
| G6 Transformation and reuse | pass after DOC-01 correction | Section IDs, prose and source paths survive the inspected transformations; stale-page deletion and title/alias fixtures cover derived output behavior |
| G7 Truthful capability claims | pass at stated strength | Implementation, test receipts, dirty source revision and remaining remote qualification are distinguished; no product acceptance inferred |
| G8 Library leverage | pass | mdBook renders, Pagefind indexes/presents search, lychee checks, stdlib parses HTML/TOML and serves files; own code binds repository semantics |
| CI-G1, CI-G2, CI-G3 | n.a. | No product facts, behavioral answers, evidence snapshot synthesis or evaluation inputs are changed |

Browser interaction and final clean-checkout qualification remain separately recorded execution
checks, not additional architectural findings invented from missing review-agent runs.

## 7. Findings and applicability

<a id="DOC-01"></a>

### DOC-01 — Frontmatter shifted stable section destinations

**Dated finding, 2026-09-25.** Initial `stage` inserted anchors at resolver source-line offsets
into a body from which metadata had already been removed. A temporary architectural document
with scalar frontmatter and §1/§2 placed `section-1` before §2 and `section-2` after its body.
The generated directory could therefore navigate to the wrong section while fragment existence
checks passed. This violated FP-02/FP-05, DP-08/DP-24 and G2/G6 for a documented authoring form.

**Correction inspected and Tested:** the resolver still owns IDs, while staging locates each
owned ID in the actual body headings before inserting its anchor. The new
`test_architecture_frontmatter_preserves_stable_anchor_destinations` checks both destinations.
The focused suite passed after the correction. The [plan's findings and outcomes](../../plans/architectural-documentation-and-search_2026-09-25.md#findings-and-outcomes)
owns current disposition; this review retains the original failure and dated closure evidence.

No additional blocking finding was established. FP-01–FP-06 are satisfied for the named source,
move, search and publication scenarios after the correction. Applicable DP-01–09, DP-11,
DP-13–19 and DP-21–24 are supported by the ownership map and boundaries above. DP-10 is satisfied
by preparing one static index per build. DP-12 adds no recursive computation obligation here;
DP-20 is bounded by the documented single-publisher lifecycle. This does not certify product
architecture against those same rules.

## 8. Library fit and total complexity

| Capability | Choice and boundary | Assessment |
|---|---|---|
| Markdown rendering/navigation | Installed mdBook 0.5.4, generated SUMMARY; built-in search and README renaming disabled | Keeps source authority and renderer responsibilities separate; no custom Markdown renderer |
| Static search | Installed Pagefind 1.5.2; bundled Component UI, scope metadata and small local UI adapter | Avoids a Node service or bespoke search engine; theme owns only placement, scope and URL resolution |
| Publication integrity | Installed lychee 0.24.2 in offline fragment mode | Does not turn network availability or old external claims into routine blockers |
| Repository semantics | Stdlib functions plus shared section/local-reference modules | Required semantics are section governance, collection selection and transport; no provider framework is justified |

Versions above were read with each installed binary's `--version` in this review. Source/API
inspection establishes the integration route; the existing real index was independently inspected
below. This review does not claim a separately executed end-to-end build.

## 9. Alternatives and tradeoffs

The previous single-file design and direct text search had minimal tooling but expanding default
reading scope. Focused Markdown with generated navigation retains direct file access and adds
bounded static publication. A full documentation application, semantic proof graph or generated
code inventory would introduce lifecycle and modeling work without an agreed consumer. Raw
Markdown alone remains the simplest fallback if publication stops earning its maintenance cost.

The accepted composition is appropriate now. Revisit if ordinary section moves repeatedly need
publisher-specific exceptions, source selection becomes another status registry, or the website
requires product qualification. There is no reason to add another verification platform now.

## 10. Verification and uncertainty

All observations below are dated **2026-09-25**.

| Command / inspection | Evidence and outcome |
|---|---|
| `uv run --no-project --offline --no-python-downloads --with pytest==9.1.1 pytest -q tests/scripts/test_adr.py tests/scripts/test_design_sections.py tests/scripts/test_docs.py` | **Tested, passed** after DOC-01 correction; focused suite only |
| Temporary stdlib `docs.stage` probe with frontmatter and two sections | **Tested, passed as a defect reproduction** before correction; wrong destinations observed; retained regression now checks the correction |
| `git show HEAD:docs/design/DESIGN.md` compared to focused section files with heading/anchor normalization | **Interface-checked, passed** for preserved normative text and expected link rebasing |
| Stdlib gzip/JSON inspection of `build/docs/site/pagefind/fragment/*` | **Measured local artifact observation:** 210 fragments, 210 distinct URLs; Current 50, History 149, Reference 11; no root/index/print/404 aliases; §3.9 had its own title and Current scope |
| `mdbook --version`; `pagefind --version`; `lychee --version` | **Interface-checked, passed** against the declared binary pins |
| Real build, browser/prefix and clean-checkout acceptance | **not_run by this reviewer** to avoid concurrent publication; implementation agent records its actual receipts in the plan |
| Remote CI, `just test-all`, `just pilot` | **not_run**; remote execution is distinct, product qualification is outside this docs scope |

The artifact count is an observation of a dirty local build, not an expected-count gate. The
concurrent uncommitted reasoning/evidence sources are intentionally present in that observation.

## 11. Authority changes and dispositions

ADR-0041 correctly supplements ADR-0040. It changes publication and architectural collection
ownership without replacing the review principles, status owners or unrelated accepted decisions.
The active process surfaces consistently route to the collection and relevant owner. Accepted ADR
immutability remains enforced; historical reviews are not rewritten as current certification.

DOC-01 belongs in the documentation plan's existing findings section with its regression receipt.
Final command/browser/clean-checkout receipts and the handoff also belong there and in the normal
checkpoint route. No new register, automated architectural judgment or product finding schedule
is required.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario evidence |
|---|---|---|
| A1 Localize change | satisfied | Source additions, section moves, lifecycle changes, search presentation and isolated tests have bounded owners; product setup is excluded |
| A2 Encode meaning structurally | satisfied after DOC-01 correction | Stable section IDs and existing lifecycle/standard owners drive derived forms; staged anchors bind to the correct body representation |
| A3 Extend through composition | satisfied | Ordinary additions use collection discovery; established renderer/indexer/checker capabilities compose without a bespoke workflow per page |

**Bounded decision: Accept the assembled documentation architecture at the evidence strength
stated here.** No blocking defect remains from this review after DOC-01 correction. Complete and
record the plan's terminal publication checks before reporting task closure.

**Enclosing architecture:** accepted for the named documentation scenarios and declared
single-publisher lifecycle. Product architecture and Stage 3 qualification are not assessed.
Longer-term reductions in reading and maintenance cost remain Proposed, not measured by this
implementation review.
