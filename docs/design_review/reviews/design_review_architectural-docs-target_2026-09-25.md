# Architectural documentation target review

## 1. Scope, outcome and coverage

**Design · target · compact effort · 2026-09-25.** Independent reviewer: Codex
`docs_target_review`. Standard: repository core 3.0, code-intelligence profile 1.1 and the
[binding](../design_principles/binding/library-context.md), as declared by
[`standard.toml`](../design_principles/standard.toml).

**Decision: Accept at Proposed strength for the documentation architecture.** Reviewed
[ADR-0041](../../adr/0041-architectural-documentation.md), the
[proposal](../../lightweight_architectural_documentation_and_search_proposal.md), the
[execution plan](../../plans/architectural-documentation-and-search_2026-09-25.md) and its
adjacent consumers: DESIGN's authority statement and §2, existing ADR section checks,
binding disposition rules, pin-check and STATUS. The target is working-tree work based on
`90cda89`; the publisher and section extraction were not implemented at review start.

This review settles the proposed ownership, contracts and composition. It does not qualify a
site, measure maintenance savings, certify Stage 3, or close the concurrent product reviews.
Product fact extraction, behavioral claims, graph analysis, evaluation and MCP serving are
outside scope; static documentation search does not produce those answers.

## 2. Responsibilities and authority

| Owner | Contract and dependency direction | Reason for change |
|---|---|---|
| DESIGN plus narrowly scoped section collection | Architectural meaning and stable § identities; source reading works without a publisher | Changed architectural contract or boundary |
| Existing ADR records, standard and plans | Rationale/lifecycle, active review standard, and current scheduled disposition respectively | Decision, standard revision or execution transition |
| Shared section resolver | Owners/spans/ancestry to ADR validation and generated directory | Supported source syntax or section movement |
| Publication declaration | Selected sources, ordering, current reading priorities, URLs and docs pins | Publication policy or tool change |
| Publisher stages | Derive navigation and HTML/search from selected source; no product imports | Rendering/link transport integration |
| Established tools | Markdown rendering, static search/UI and offline link checking | Tool compatibility or a demonstrated limitation |

The directory is derived, and classification consumes existing ADR and standard lifecycle.
An explicit current-work list owns reading priority only. The architecture map is a small
navigation surface, not an inventory of implementation or a second progress table.

## 3. Contracts and testing boundaries

| Contract | Required invariant and failure | Verification boundary |
|---|---|---|
| Section resolution | One substantive owner per stable ID; fenced examples and relocation pointers are not owners; unresolved governed references reject | Temporary Markdown/ADR fixtures without product setup |
| Section extraction | Preserve substantive text/labels/decision references and historical fragments; remove the old normative copy | Source diff plus resolver and legacy-link cases |
| Link transport | Published target → local HTML; selected asset → staged asset; omitted committed target → revision URL; ignored optional skill → visible local reference; invalid target → error | Representative links, actual rendered fragment checker |
| Classification | Current is relevance, never evidence strength; Everything removes scope restriction | Derived metadata cases plus initial browser acceptance |
| Publication | Fresh candidate becomes current only after render/index/link checks; failure retains successful output; deletion removes obsolete output | Failure branches and a real integrated build |
| Isolation | Installation belongs to bootstrap; ordinary docs commands require no native product, store or server | Clean-checkout/isolated command run |

The section resolver must preserve logical enclosing relationships after extraction. Governing
decisions cannot come from an unrelated sibling or from a relocation pointer. ADR metadata and
accepted-record immutability continue to be checked by the existing ADR owner.

## 4. Composition and execution

The declared sequence is discovery → staging/derived SUMMARY → mdBook → body/scope annotation →
Pagefind → offline link checks → artifact replacement. Each stage has a concrete consumer and
ordinary function boundary. Source directories are inputs, build directories are disposable
outputs, and no incremental cache or semantic proof service is introduced. Evidence READMEs
enter the collection without recursively copying raw probe trees or replaying them.

## 5. Change and failure scenarios

| Scenario | Owner and affected consumers | Required context and settling check |
|---|---|---|
| Add an ordinary ADR or review | Source document; existing ADR index where applicable; collection discovers navigation | Existing lifecycle convention and publication selection; add/delete fixture |
| Move §9.9 | Section source plus legacy pointer; resolver keeps ADR references and directory valid | Relevant section, inherited governing decisions, relative links; cross-file and fragment cases |
| Supersede an ADR or standard | Existing record/manifest lifecycle; derived search classification changes | No independent status declaration in publisher; lifecycle fixture |
| Select next active plan | Publication current-work selection changes reading priority | No copying plan rows or changing historical conclusions |
| Replace renderer or change hosting prefix | Rendering/URL integration changes; architectural meaning and source IDs remain | HTML contract and root/prefix browser cases; no product acquisition |
| Candidate build fails or source disappears | Build attempt and publication boundary | Previous artifact preservation and successful rebuild without stale page |

These are reasoned routes, not observed extension-cost measurements.

## 6. Correctness and fidelity gates

Gate verdicts below assess **target specification**, not implementation execution.

| Gate | Verdict | Evidence or scope reason |
|---|---|---|
| G1 Authority | pass | ADR-0041/proposal §§3–5 retain existing owners; directory/nav/index derive |
| G2 Semantic fidelity | pass | Stable section identities, labels and compatibility pointers; classification explicitly separates relevance from maturity |
| G3 Validity | pass | Duplicate/unresolved owners and invalid publication targets reject; focused tests and offline fragments are required |
| G4 Hidden behavior | pass | Bootstrap/build effects are separated; product compiler, services and corpus ingestion are excluded |
| G5 Consistency and recovery | pass | Fresh candidate and failed-build preservation contract; local artifact rather than a production serving guarantee |
| G6 Transformation and reuse | pass | Preservation and deletion obligations explicit; fresh rebuild avoids an extra invalidation mechanism |
| G7 Truthful claims | pass | ADR acceptance distinguished from implementation; dirty preview and revision limits explicit; final checks still required |
| G8 Library leverage | pass | Established renderer/search/checker composed; bespoke policy limited to repository selection, sections and transport |
| CI-G1, CI-G2, CI-G3 | n.a. | No extracted facts, synthesized behavioral claims or product evaluation inputs are changed or produced |

## 7. Findings and applicability

No actionable architectural defect found in the proposed scope. FP-01–FP-06 and the supporting
DP-01–05, DP-08–09, DP-13–19, DP-21–24 are **satisfied at Proposed strength** by the contracts and
scenarios above. This does not turn intended enforcement into implemented enforcement.

Implementation attention belongs to the existing plan's acceptance, not a new finding register:
logical ancestry for governing decisions; legitimate legacy fragments; source URLs that do not
claim uncommitted bytes exist at HEAD; active-standard classification from the manifest; and
clean-checkout behavior when optional ignored capability skills are absent. Concurrent publisher
processes, crash-durable replacement and public deployment are not qualified by this review.

## 8. Library fit and total complexity

**Interface-checked, 2026-09-25:** `mdbook --version`, `pagefind --version`, `lychee --version`
returned 0.5.4, 1.5.2 and 0.24.2. `mdbook build --help`, `pagefind --help` and `lychee --help`
expose separate HTML output, static-site indexing with body/exclusion annotations, and offline
fragment checking with an absolute root directory. No build or browser behavior was exercised.

Those mechanisms fit the stage contracts without adopting a Node application or the product's
semantic retrieval stack. Repository section/governance and current-work selection are local
semantics, so a small stdlib adapter is justified. A general plugin framework, crawler, custom
search engine, or full Markdown implementation would require a new demonstrated consumer.

## 9. Alternatives and tradeoffs

Source Markdown alone remains the useful minimum and fallback. It does not meet the requested
browser discovery experience. Built-in mdBook search would reduce tool count, but the proposal's
scoped static-search boundary gives a credible reason for Pagefind; there is no claim of a measured
local search defect. Another generator is a future substitute if a concrete renderer limitation
appears. A proof/catalog platform adds semantic maintenance without a current consumer and is
appropriately excluded. Source-first documentation and focused checks carry the main benefit.

## 10. Verification and uncertainty

Source/contract inspection and the CLI interfaces above are the evidence for this review.
Implementation tests, site build, browser search, prefix behavior, CI artifact creation and search
asset observations are **not_run** by this reviewer. The execution plan already assigns their
acceptance. A remote CI receipt requires an actual remote run. Long-term reduction in context and
maintenance cost remains Proposed even after a working site is delivered.

## 11. Authority changes and dispositions

ADR-0041 and DESIGN §2 are the decision route for the collection/publishing policy; ADR-0040
retains review principles and disposition ownership. P06 must remove active single-file routing
assumptions and scope documentation-tool pin qualification without weakening product pin checks.
The [execution plan](../../plans/architectural-documentation-and-search_2026-09-25.md) owns current
implementation status and the final review. No product finding is transferred or closed here.

## 12. Architectural judgment and decision

| Judgment | Verdict | Scenario basis |
|---|---|---|
| A1 Localize change | satisfied, Proposed | Source additions/moves and renderer changes have bounded owners; docs tests exclude unrelated product setup |
| A2 Encode meaning structurally | satisfied, Proposed | Shared section identities, explicit collection declaration, and derived lifecycle classification retain one owner per concept |
| A3 Extend through composition | satisfied, Proposed | Renderer, search and checker form a replaceable static pipeline; ordinary content additions need no bespoke workflow |

**Bounded decision: Accept the target for implementation.** No blocking architectural revision
is required. The documentation implementation remains unqualified until its focused checks and
assembled review. **Enclosing product architecture: not assessed.** Proceed with the shared
resolver and preservation-focused section moves, under the execution plan's existing owner.
