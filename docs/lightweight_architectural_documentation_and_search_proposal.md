# Proposal: architectural documentation and search for library-context

**Adoption update, 2026-09-25:** adopted through [ADR-0041](adr/0041-architectural-documentation.md).
The [execution plan](plans/architectural-documentation-and-search_2026-09-25.md) owns implementation
and qualification. The original proposal and baseline below are retained as dated context.

**Status: Proposed · 2026-09-25.** This is a target-system proposal, not an adopted decision or
implementation receipt. It builds on [ADR-0040](adr/0040-architecture-change-scenarios.md) and the
supplied `docs/lightweight_architectural_documentation_and_search_other_repo.md`. All proposed
benefits and acceptance checks below remain Proposed unless explicitly labeled otherwise.

## 1. Recommendation

Adopt **focused, authoritative Markdown with generated mdBook navigation and Pagefind search**.
Make source documents useful before building the website. Integrate the publishing layer with
the responsibility boundaries already established by ADR-0040:

- Architecture documents own accepted contracts and labeled targets.
- Executable schemas and declarations own detailed implementation contracts.
- ADRs own decision rationale and supersession.
- Reviews own dated assessments; plans own scheduled findings' current disposition.
- STATUS owns the session checkpoint and links to those owners.
- The website derives navigation, presentation and search from these sources.

The central improvement is **bounded reading context with one owner for each kind of information**.
The publishing tools make that structure easier to navigate. Documentation upkeep should follow
changes to enduring meaning, boundaries and workflows; an internal implementation change should
not require an architectural evidence packet.

Use a small repository-owned publishing adapter. Reuse suitable mechanisms from pse-arrow after
its implementation settles, with a recorded donor revision and local tests. Keep library-context's
configuration and lifecycle rules local. A shared package can wait for demonstrated cross-repo
maintenance work.

## 2. What is established today

**Interface-checked, 2026-09-25:** source inspection at library-context `92eca2e`, including the
subsequent uncommitted review/handoff material, establishes the following baseline.

| Existing surface | Observation | Design implication |
|---|---|---|
| [DESIGN](design/DESIGN.md) | 4,375 lines and approximately 50,700 whitespace-delimited words; it declares itself the single authoritative file | Retain its identifiers and authority while moving coherent responsibilities into focused documents |
| Repository entry points | No root README, documentation home, book configuration, SUMMARY, or `.github` directory at inspection | Establish source reading paths and a local publisher; this is a new documentation surface here |
| Design-review process | Core 3.0, CI profile 1.1, A1–A3, proportional cadence and single disposition ownership are already implemented | Route into this process; avoid another review framework |
| [ADR tooling](../scripts/adr.py) | `design_ids()` and `design_decisions()` read one DESIGN file; the ADR index is generated from records | Extend the existing resolver for movable sections and preserve its current checks |
| [Agent checks](../scripts/check_agents.py) | Existing checks validate skill discovery, instruction links and command names | Retain their bounded purpose |
| Documentation corpus | 183 tracked Markdown documents under `docs/`, including substantial review/evidence history | Separate current reading paths from dated evidence without relocating the history |
| [Active plan](plans/behavioral-model-forward-plan_2026-09-24.md) and [STATUS](../STATUS.md) | Already own execution sequencing, dispositions and checkpoint context; newer reviews still own unscheduled findings | Navigation should point to these owners without copying their status tables |

The supplied pse-arrow plan recommends the same publishing stack, collection discovery, search
scopes, incremental splitting and limited automation. Its working tree at inspection, based on
`1601b757`, already contains `scripts/docs.py`, `docs/site.toml`, Pagefind integration and proposed
ADR-0095. The inspected CI workflow still invokes mdBook directly. This is an implementation in
progress, not a verified reusable release; no pse-arrow build was run for this proposal.

### Adaptations specific to this repository

| Adopt from pse-arrow | Adapt here |
|---|---|
| Markdown, mdBook, Pagefind and generated collection navigation | Add the source entry pages first; there is no existing site or Pages contract to preserve |
| Publication declarations without implementation claims | Read active standard paths from `standard.toml`; preserve ADR-0040's status owners |
| Incremental architecture extraction | Start with Stage 3's frequently read §3.9 and §9.9, after the section resolver is ready |
| Stable section identity and legacy pointers | Extend both ADR reference resolution and governing-decision checks, including inherited decisions |
| Search scopes | Keep this repo's current DESIGN discoverable as Current during migration; label its mixed evidence strengths |
| Independent documentation build | Exclude compiler runs, Delta stores, embeddings, native extensions, skill corpora and product test fixtures |
| Mechanical publication checks | Add a focused documentation check; preserve the existing integrated product acceptance cadence |

The classification difference matters: pse-arrow intentionally treats its mixed legacy blueprint
as Reference. Our DESIGN is still the declared current authority. A large file alone is not a
reason to hide its remaining current contracts from the default search.

## 3. Source information architecture

### Entry points and owners

Proposed new paths below do not exist yet.

| Surface | Responsibility | Update trigger |
|---|---|---|
| `README.md` | Explain the project briefly; route to documentation and the current checkpoint | Product identity or entry path changes |
| `docs/README.md` | Route by task: understand, change, review, resume, research, investigate history | Reading paths change |
| `docs/design/README.md` | Small architecture map: responsibilities, allowed dependency direction, contract entrypoints and section links | A meaningful owner, boundary or extension path changes |
| `docs/design/DESIGN.md` plus `docs/design/sections/` | Authoritative architectural collection, retaining existing § identities | Architectural decisions and contract changes |
| Existing standard, binding and process skills | Principles, cadence, review behavior, ADR and handoff procedures | Deliberate process changes |
| Existing plans, reviews and STATUS | Execution and evidence under ADR-0040 | Their existing update triggers |
| Existing pins, library projects and reference material | Dependency and capability context | Their existing owners' triggers |

The architecture map should name coherent capabilities and a few useful entrypoints, with links
to the owning sections. For example: extraction in `cpg-extract`/`cpg-flow`, contracts in
`cpg-schema`, relational orchestration/publication in `cpg-core`, finite analytics in
`lctx-analytics`, and native/Python serving. Source pointers are navigation aids; they are not
exhaustive symbol inventories or declarations that every planned boundary is implemented.

Within a focused architecture page, explain the responsibility, consumer contract, invariants,
dependency/effect boundary, intended extension path, important alternatives and relevant ADRs.
Use only the material the subject needs. Link detailed schemas and rules to their executable
owners. Preserve evidence labels for implemented behavior and targets. Execution progress stays
with the plan.

### Task-based reading paths

| Task | Bounded starting path |
|---|---|
| Understand the architecture | Documentation home → architecture map → relevant owner and adjacent consumer |
| Resume Stage 3 | STATUS → active plan queue/disposition → relevant architectural section |
| Change a summary or condition rule | §3.9/§9.9 owners → cited contracts and source → affected serving boundary |
| Change a schema or identity | §B2 and the relevant contract/identity section → executable schema → applicable migration decision |
| Review a proposal | Relevant architecture → core foundations and declared profile → binding's review route |
| Research a capability | Pins and relevant local capability skill, then the existing research route |
| Understand a decision or old result | ADR index or linked source review → original dated evidence |

These paths guide investigation; an agent still expands scope when a dependency or contradiction
requires it. The default should not load all of DESIGN, every review, or the entire skill corpus.
Direct file access and `rg` remain the agent interface, available without a built site.

## 4. Modular authority and citation compatibility

Retain DESIGN as the durable entry and home for scope, binding decisions and revision history.
Expand architectural authority to DESIGN plus a narrowly declared `docs/design/sections/`
collection. Titles, numbers and decision references travel with their substantive content.

Start with two useful extractions:

| New owner | Existing material | Reason |
|---|---|---|
| `sections/behavior-model.md` | §3.9 | Gives condition, place, coverage and verdict work a focused semantic entry |
| `sections/behavioral-analysis.md` | §9.9 | Gives Stage 3 models, summaries and composition a bounded architectural entry |

Keep the other sections in place initially. Later extraction follows substantive changes to
extraction, publication, analytics, synthesis or serving. If §9.9 itself becomes difficult to
reason about, assign stable child IDs to coherent responsibilities and split them when needed.
There is no mandatory page count, line budget or one-page-per-crate rule.

Extend the existing ADR tooling with **one section resolver** that returns identifier, owning
source path and section span. Both ADR checks and the generated section directory use it:

1. Scan authoritative numbered headings only. Ignore fenced examples, historical documents,
   navigation pages and relocation pointers when identifying owners.
2. Each section has one owner. Duplicate IDs and unresolved governed references produce useful
   mechanical errors. Keep §B1–§B14 and all existing numeric IDs unchanged.
3. Preserve the current checks for decision references, supersession and accepted-record
   immutability. At extraction, make a governing decision explicit in the new page where it
   previously depended on an enclosing section; do not silently lose that relationship.
4. Leave the old heading/fragment and a short link at the vacated location. Mark it as a
   relocation pointer so it is not another owner. Preserve subordinate fragments as needed.
   Remove the old normative prose once its new owner exists.
5. Keep historical ADR `design:` fields and review citations unchanged. New citations can use
   `DESIGN §9.9` plus a direct link to the owner. New stable fragment IDs can supplement existing
   heading-derived fragments; they should work in source Markdown as well as rendered HTML.
6. Derive the site's section directory from this resolver. Do not maintain a second section
   registry, source checksum catalog or document-to-test map.

Perform section moves as structural changes first. Preserve existing labels, reservations and
dated outcomes; reconcile a substantive contradiction through the appropriate decision afterward.
This reduces the chance of a document move silently changing the architecture.

## 5. Publishing and search architecture

```text
Authored Markdown + existing generated ADR index
              │
              ├── direct reading / rg / source links
              │
       collection discovery and staging
              │
       generated SUMMARY → mdBook HTML
              │
       body/scope annotations → Pagefind
              │
       offline links/fragments → complete static artifact
```

### Small configuration and adapter boundary

Propose `docs/site.toml` for publication choices: collection roots, explicit source inclusions,
exclusions, ordering, selected current-work entrypoints, deployment URL settings and documentation
tool pins. `docs/book.toml` holds renderer settings. Each value has one owner; local installation
and any future CI read the same tool declarations. `docs/pins.md` can link that declaration rather
than duplicate its versions.

The adapter has ordinary functions for discovery, staging, rendering, search annotation/indexing
and publication. It imports no product packages. Use the standard-library Python route already
used by ADR tooling, with a docs recipe that bypasses project dependency synchronization. Obtain
the existing pinned interpreter through the repository's uv conventions; installation is distinct
from an offline build.

Stage only selected sources under ignored `build/docs/`, preserving repository-relative paths.
This accommodates links from docs to STATUS, AGENTS and selected tracked process skills without
copying the repository. Publish to `build/docs/site/`; there is no existing `docs/book/` consumer
here to preserve. Generate SUMMARY in staging, never as another authored navigation list.

Use existing scalar titles where present, otherwise the first real H1 outside fenced code.
Require no metadata backfill. Each page has one publication path. Derive active standard membership
from `design_review/design_principles/standard.toml`. Select current plan/review entrypoints in
site configuration; this selection expresses reading priority and contains no copied progress.

For links to source files outside the published selection, resolve the target in the repository
and render an appropriate repository-browser link. Edit links point to original sources, not
staging. Identify the build revision; a dirty local preview cannot claim that remote source links
show its uncommitted contents. These are transport rules, not symbol-conformance checks.

Exclude build outputs, environments, raw probe output, stores, binaries, code fixtures and ignored
library-skill corpora from recursive discovery. Include evidence READMEs as History; link necessary
raw evidence to its repository/LFS source or explicitly selected asset. Do not recursively copy
all assets under the evidence tree. Ordinary illustrations can travel with their pages.

Build a fresh candidate artifact and replace the previous site only after rendering, indexing and
mechanical checks succeed. A failed build preserves the last successful site. Deleting a source
page removes its old output on the next successful build.

### Tool choice

mdBook provides Markdown chapter rendering driven by SUMMARY, and its built-in search can be
disabled. Pagefind indexes generated HTML and supplies static search assets and a bundled
Component UI. It can be installed as a standalone binary. Those boundaries let the repository
compose publishing capabilities without a Node application or hosted search service.
[mdBook configuration](https://rust-lang.github.io/mdBook/format/configuration/renderers.html),
[Pagefind overview](https://pagefind.app/docs/),
[Component UI](https://pagefind.app/docs/search-ui/),
[installation](https://pagefind.app/docs/installation/).

Treat the sibling's mdBook 0.5.4 / Pagefind 1.5.2 selection as the initial candidate tool pair.
Verify and pin the chosen releases during implementation under the pin-check workflow. Current
documentation inspection establishes the integration route, not compatibility of an unbuilt
library-context site. Use Pagefind's library-owned input/results/keyboard behavior; keep local
JavaScript limited to placement, scope selection and URL configuration.

### Search contract

| Scope | Contents |
|---|---|
| **Current** — default | Entry pages; current architectural collection; active core/profile/binding; selected contributor/process guidance; accepted ADRs; selected active work and relevant pending reviews |
| **Reference** | Pins and supporting research; proposals, including this one; proposed ADRs unless explicitly selected for current work |
| **History** | Superseded/rejected ADRs and standards; previous plans/handoffs; dated reviews/evidence not selected for current work; initial research |
| **Everything** | All indexed documentation scopes; not every file in the repository |

Scopes describe reading relevance. Current does not mean implemented, verified or accepted in
every sentence. A review selected for current work remains a dated review; a proposed ADR remains
proposed. Show document kind and existing lifecycle/evidence text where available, and preserve
links to current disposition owners. Avoid a new mandatory status field on every Markdown file.

Initialize Current before the first query; Everything clears the scope restriction. Make the
scope visible and offer an obvious way to broaden a search with no results. Index canonical main
content, excluding navigation, print duplicates, error pages and landing aliases. Results should
open relevant headings where supported, with working local and deployment-prefix URLs.
Pagefind supports content selection and page filters through HTML annotations.
[Indexing](https://pagefind.app/docs/indexing/), [filters](https://pagefind.app/docs/filtering/).

Do not infer currentness from timestamps, a filename containing “latest,” or all findings being
open. Select the current plan explicitly and route pending review reading from STATUS/that plan.
When work moves, update the publication selection without editing historical reviews' conclusions.

## 6. Relationship to architecture reviews and change tracking

Preserve ADR-0040's process. A review starts at the relevant architecture owner and traces a
credible change through adjacent contracts. The site helps find that context; it cannot award
an A1–A3 verdict, certify implementation, or close a finding.

| Change | Documentation/process work expected |
|---|---|
| Internal implementation change within an accepted contract | Relevant code/tests and existing handoff obligations; architecture changes only if its explanation became inaccurate |
| New instance using an existing extension path | The new declaration and warranted checks; update architectural prose only if the contract or supported claim changes |
| New ownership boundary, semantic contract or material alternative | Relevant architecture page + ADR + review at the binding's cadence |
| Finding is scheduled or resolved | Update its single disposition owner and evidence links; preserve the original review |
| New review or ADR document | The document, with the existing ADR-index update where applicable; collection navigation is derived |
| Documentation wording or navigation fix | Focused documentation check; no product qualification run solely because prose changed |

Retain the current plan's ARC-01–03 rows. The newer reasoning reviews' unscheduled findings remain
with their existing source owners until explicitly transferred. This proposal introduces no
parallel finding register or automatic migration of those dispositions.

The ADR, review, handoff and reviewer-agent instructions need small routing amendments for the
authoritative collection and source reading paths. Keep cadence in the binding and versions in
the standard manifest. Update AGENTS' design-start instruction to read the architecture entry and
relevant owners instead of requiring the entire monolithic file.

## 7. Automation budget

Automate properties that are cheap, objective and directly consumed by publication:

- Discovery, existing entrypoints, title fallback and duplicate publication paths.
- Unique architectural section ownership and existing ADR-reference rules.
- Internal links/fragments and source-link targets in the selected publication.
- Successful rendering/indexing, complete artifact replacement and correct scoped search wiring.

Use an established link checker such as lychee for rendered local links/fragments, with its
offline mode and a fixture for the deployment prefix. Pin it with the documentation tools.
Exact flags follow the selected release; avoid copying a sibling invocation across version
changes without checking it. [Official CLI reference](https://lychee.cli.rs/guides/cli/).

Architecture conformance remains a reasoned review supported by relevant source and product
evidence. Source hashes, symbol manifests, proof receipts for documentation, mandatory
finding-to-test matrices, automatic coverage scores and recurring whole-system requalification
are outside this design. Existing schema validators, snapshots, semantic controls and integrated
product acceptance retain their own responsibilities.

Use small publisher fixtures for actual behavior: new/deleted pages, scoped classifications,
relocation links and failed builds. Keep browser checks focused on the initial integration and
material UI changes. A one-time observation of search loading is enough initially; add a numeric
budget only if a recurring measured problem needs one.

Do not use library-context's own compiler, embeddings, Delta store or MCP service to publish or
search its architectural docs. That would couple the development guide to the product being
redesigned and create an unnecessary second research/search application.

## 8. Alternatives and foundation assessment

| Alternative | Assessment |
|---|---|
| Focused Markdown and source search alone | Smallest useful first increment and permanent fallback. It does not provide the requested browser discovery experience across the corpus. |
| mdBook with built-in search | One fewer binary, but still needs content/navigation work and offers less separation for the chosen scoped-search design. We have no measured local search defect to claim. |
| **mdBook + Pagefind + bounded adapter** | Recommended: readable sources, explicit information ownership, replaceable publishing stages and relevant reuse from the sibling repository. Costs are tool pins and a small adapter/UI integration. |
| Another static generator | Viable if a concrete mdBook limitation appears. Current requirements can be met without changing Markdown conventions or adopting another theme/plugin stack. |
| Hosted or product-powered semantic search | Extra lifecycle, ingestion and operational dependencies with no demonstrated requirement for this documentation task. Revisit only after useful lexical search is shown insufficient. |

**Proposed architectural assessment:**

| Foundation | How the target supports it |
|---|---|
| FP-01 Separation | Content, decisions, execution state, discovery, rendering and search have distinct owners |
| FP-02 Stable contracts | Markdown paths, section IDs and rendering/indexing interfaces permit implementation replacement |
| FP-03 Composition | Discover → stage → render → index → publish; established tools supply generic mechanisms |
| FP-04 Authority | Source prose/declarations and existing lifecycle fields are authored once; navigation and indexes derive |
| FP-05 Explicit structure | Collection selection, exclusions, section ownership and artifact boundaries are inspectable |
| FP-06 Local reasoning | A task starts with a relevant contract and a bounded source neighborhood |

A1–A3 have credible proposed routes: an ordinary addition changes one source; a section move
preserves one owner; renderer replacement leaves architectural meaning intact. Actual search
quality, maintenance cost and review effectiveness remain unmeasured.

## 9. Dependency-ordered adoption

These are proposed work packages, not new live status rows. If adopted, one implementation plan
owns their progress and findings.

| Order / owner | Scope | Acceptance and deletion obligations |
|---|---|---|
| 1 — Documentation/process | Record the collection/publishing decision through the existing ADR workflow, preserving ADR-0040's foundations and tracking policy. Add the root/docs entry points and architecture reading map. | Source-only reading paths answer understand/change/review/resume tasks. No repeated status tables; accepted authority change is explicit. |
| 2 — Existing ADR tooling | Introduce the shared section resolver; extend both `design_ids` and `design_decisions`; perform the two initial section extractions. Update live source pointers and generated ADR-index wording. | Unique owners, inherited-decision handling and legacy fragments work; accepted ADR bodies and old section IDs survive. Delete relocated normative duplicates. |
| 3 — Publishing | Add collection configuration, staging, generated SUMMARY and mdBook. Reuse qualified sibling mechanisms where suitable. | Add/delete document behavior, stable source/edit URLs, legitimate assets and source-tree non-mutation; builds need no product environment. No authored SUMMARY. |
| 4 — Search/checks | Add Pagefind, scopes, bundled UI and offline link/fragment checking. | Current/Reference/History/Everything queries, nested pages, source links and both root/prefix hosting work. Print/navigation duplicates excluded; failed candidates preserve the last successful site. |
| 5 — Workflow integration | Align AGENTS, process skills and binding references. Add local commands and, when desired, a docs-only CI artifact job using the same path. | One command route, one set of tool pins, focused fixtures and one complete real publication check. Product gates remain independently scheduled. |

Proposed commands: `just docs` builds and validates a complete render/search artifact, including
offline links before replacement; `just docs-check` uses that same path plus the existing ADR
and agent-instruction checks; `just docs-test` exercises the small adapter/resolver
fixtures; `just docs-serve` builds and previews; `just bootstrap-docs` installs declared tools.
They are **not present today**. Preserve the existing `just adr …` and `just lint-agents` spellings.
Avoid hiding documentation installation or product compilation inside a build/serve command.

There is no CI deployment contract here to inherit from pse-arrow. Begin with local output and
an optional CI artifact. Public Pages hosting is a separate choice; the local system should be
complete without it. If CI is added, scope it to documentation, included entry sources and
publishing tooling rather than every production edit.

## 10. Acceptance boundary and maintenance

Implementation is complete when the source reading paths work, the declared documentation corpus
builds and searches correctly, citations survive the initial moves, and all applicable mechanical
checks pass on that corpus. Check representative questions: “where do I add a summary rule?”,
“what owns a finding's current status?”, “why was this decision superseded?”, and “where is the
pinned library reference?”. Record outcomes once in the implementation plan.

Handle discovered broken links through small corrections or explicit source/history destinations;
do not mask the entire historical collection with a blanket ignore. Rendering old evidence does
not require rerunning it or promoting it to current qualification. If historical content needs
reconciliation, keep that a distinct architectural task with a real consumer.

After adoption, ongoing work is limited to source edits when meaning changes, current-work
selection when focus changes, and occasional pinned tool updates. Revisit tooling only for a
documented limitation or recurring maintenance burden. Broader architecture extraction follows
subsystem work rather than a repository-wide cleanup campaign.

## 11. Evidence and proposal validation

**Interface-checked, 2026-09-25:** the baseline above comes from local Markdown, manifests and
tooling source; the publishing route was checked against Context7 and official mdBook, Pagefind
and lychee documentation linked above. The local pse-arrow files describe work in progress.
No sibling or local publishing build, browser qualification, product tests or pilot was run.

Proposal acceptance should decide the information ownership, initial extraction, publishing
boundary and automation budget. Tool/build claims become Tested only after the real adapter and
site exist. Adoption also needs the relevant focused design/target review under the existing
binding; this proposal is not that independent review.
