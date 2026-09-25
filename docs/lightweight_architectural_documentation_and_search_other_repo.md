 # Execution plan: lightweight architectural documentation and search

  ## 1. Target and completion criteria

  Retain Markdown as the authoritative source and mdBook as the renderer. Replace its built-in search with Pagefind, generate publication
  navigation from document collections, and begin the agreed incremental blueprint split.

  The resulting system should make relevant context easy to find while keeping routine code changes inexpensive:

  - Agents read focused Markdown documents and inspect the relevant implementation.
  - Current guidance, current work, reference material, and historical evidence have distinct reading paths.
  - Adding a document to an existing collection requires no second navigation entry.
  - Architecture sections retain stable identities when they move.
  - Automated checks establish publication integrity, links, and document identity.
  - Architectural quality remains a reasoned review judgment supported by appropriate product tests.

  No design-review pilot, production refactor, semantic code inventory, or historical requalification is part of this plan.

  Completion means the complete documentation site and search build successfully, applicable mechanical checks pass against a zero-error
  baseline, existing citations remain usable, and the workflow instructions consistently describe the new system.

  ## 2. Target structure and interfaces

  ### Content and authority

   Surface                       Responsibility
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   Documentation home            Explain the repository and route readers by task
  ────────────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   Architecture entry page       Identify authoritative sections, current contract entrypoints, and relevant decisions
  ────────────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   Focused architecture pages    Own particular design responsibilities and contracts
  ────────────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   Current-work entry page       Link the plans and packets that own ongoing work
  ────────────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   Reference collections         Expose capability maps, schemas, development guidance, and supporting material
  ────────────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   Historical collections        Preserve reviews, superseded decisions, and previous execution records

  Source documents retain their existing paths wherever possible. Classification changes navigation and search; it does not require
  physically moving historical material.

  Use one small publishing declaration, docs/site.toml, for collection roots, exclusions, collection ordering, selected current-work
  entrypoints, and documentation-tool versions. It must contain no implementation claims, symbol inventories, test mappings, or duplicated
  progress fields.

  Select current work explicitly: initially Plans 15, 16, 17, and this plan, with their associated packets. Existing in-progress metadata
  is insufficient because some historical plans still carry that status. Selection means “useful current reading,” not “all work remains
  incomplete.”

  ### Publishing pipeline

  Authored Markdown and existing generated reference
                      ↓
          Collection discovery and staging
                      ↓
         Generated SUMMARY → mdBook HTML
                      ↓
         Content annotations → Pagefind
                      ↓
               Static Pages artifact

  Implement the adapter with Python’s standard library. It must run without importing pse, synchronizing the project environment, compiling
  product crates, or starting native solvers.

  Use ignored staging under build/docs/. Preserve docs/book/ as the final artifact directory. A successful build replaces the previous
  artifact completely, preventing deleted pages or obsolete search assets from surviving.

  Use mdBook 0.5.4 and Pagefind 1.5.2 as the initial tool versions. Pagefind provides a standalone binary and bundled browser UI, so this
  design needs no Node application or hosted search service. Pagefind installation

  ### Search behavior

  Provide four scopes: Current, Reference, History, and Everything. Default to Current whenever a new search interface opens.

  - Current: entry pages, active standards and guidance, extracted authoritative pages, accepted ADRs, and explicitly selected current
    work.

  - Reference: capability maps, generated schemas, supporting documents, and other proposals.
  - History: historical reviews, previous execution plans, superseded/rejected/deprecated ADRs, and superseded standards.
  - Everything: removes the scope filter.

  The remaining blueprint mixes current contracts with retained older material. During incremental migration, classify that large document
  as Reference and link prominently from the Current architecture entry page to §§0.5–0.6 and other applicable sections. This avoids
  presenting the entire historical body as uniformly current.

  Classification describes a reading purpose; it does not certify implementation or resolve contradictions automatically. Explicit
  exceptions belong in the publishing declaration, without rewriting historical metadata.

  ### Command changes

   Command                Result
  ━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   just docs              Build the complete static site and Pagefind index
  ─────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   just docs-test         Exercise the publishing adapter and citation behavior with focused fixtures
  ─────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   just docs-serve        Build and serve the completed site locally
  ─────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   just bootstrap-docs    Install the declared documentation binaries without product setup
  ─────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   just adr-index         Continue generating the source-readable ADR index; stop editing book navigation
  ─────────────────────  ───────────────────────────────────────────────────────────────────────────────────────
   just adr-lint          Preserve existing checks and resolve section citations across authoritative documents

  For the initial implementation, local serving uses Pagefind’s preview server. Run just docs after edits to refresh its files; do not
  introduce a custom watcher or live-reload service.

  No production Rust or Python API changes are required.

  ## 3. Dependency-ordered execution packets

  ### P00 — Record the decision and establish ownership

  Responsibility: documentation governance and execution tracking.

  - Allocate the next implementation-plan number through the existing recipe.
  - Create a proposed successor ADR covering modular architectural authority and generated mdBook publication with Pagefind.
  - Identify ADR-0033 and ADR-0036 as the decisions being replaced. Preserve their historical bodies.
  - Record intended supersession in the proposed decision; apply symmetric status metadata through the existing decision-PR procedure when
    formally adopted.

  - Review the proposed change against the six foundations, emphasizing bounded reading context, one owner per fact, and publishing
    maintenance cost.

  - Keep packet progress and findings in this plan. Link Plan 17’s outstanding search warning to this work.

  Acceptance: the decision explains responsibilities, compatibility, automation limits, and the migration sequence. Its evidence remains
  Proposed until implementation supports a stronger claim.

  ### P01 — Build the publishing adapter and generated navigation

  Responsibility: document discovery, staging, and rendering.

  - Introduce the publishing declaration and a small adapter with separate discovery, staging, rendering, and indexing functions.
  - Discover Markdown within explicitly approved collection roots. Exclude output directories, implementation fixtures, and optional
    rustdoc HTML from chapter discovery.

  - Preserve linked evidence assets and other legitimate supporting files without executing them.
  - Use existing front-matter titles where available, then the first H1. Fail with a useful diagnostic when neither exists. Do not impose a
    repository-wide metadata backfill.

  - Use stable filename ordering within collections, preserving numeric ADR and plan ordering.
  - Generate SUMMARY.md only inside staging. A page has one chapter location; additional reading paths link to it.
  - Preserve the disabled mdBook index preprocessor and existing README.html behavior.
  - Preserve edit links to original source paths rather than staging paths.
  - Retain the generated, committed ADR README for GitHub readers. Remove scripts/adr.py ownership of the SUMMARY block.
  - Remove the hand-maintained source SUMMARY once the generated replacement passes its tests.

  Acceptance scenarios:

  1. Adding a Markdown page inside a declared collection publishes it without another navigation edit.
  2. Removing a page removes its output on the next build.
  3. Building changes no tracked source file.
  4. Missing titles, duplicate publication paths, and invalid configured entrypoints fail clearly.
  5. Linked supporting assets survive staging.

  ### P02 — Replace search and integrate the browser interface

  Responsibility: search indexing and presentation.

  - Disable mdBook’s built-in search and remove its configuration and obsolete generated assets.
  - Annotate canonical chapter content with Pagefind body, scope, title, and document-kind metadata.
  - Exclude navigation, print pages, error pages, duplicate landing aliases, and optional rustdoc output from the documentation search
    index.

  - Use Pagefind’s bundled Component UI for input, results, and keyboard behavior. Add only the small integration needed for the scope
    selector and mdBook placement.

  - Set the initial scope before the first query; Everything clears that filter through the documented component API. Pagefind component
    integration

  - Load search assets from the built site, supporting both local preview and the /pse-arrow/ Pages prefix.
  - Show scope and existing document status where useful, without inventing a second status field.
  - Make Pagefind failure fail the build. Do not fall back silently to the old search.
  - Keep ordinary document navigation usable when JavaScript is unavailable.

  Acceptance: real searches return the correct document classes and working section links; keyboard operation and light/dark presentation
  work; the old monolithic search index is absent.

  ### P03 — Establish focused reading paths

  Responsibility: concise human and agent context.

  - Rewrite the documentation and architecture entry pages around reader tasks.
  - Remove stale claims such as “revision 5 is current”; link the actual revision owner.
  - Provide direct paths for understanding the current architecture, changing a contract, reviewing a design, resuming current work,
    researching a library, and investigating historical decisions.

  - Simplify the plans entry page so current progress comes from linked owners. Remove manually synchronized live-status summaries.
  - Preserve historical outcomes in their original plans and reviews.
  - Explain that source Markdown is the agent interface; rendering and search are conveniences, not prerequisites for reasoning.

  Acceptance: each common task has a clear starting point and a bounded next reading step, without requiring a whole-blueprint or whole-
  history read.

  ### P04 — Add movable section ownership and perform the initial split

  Responsibility: authoritative document boundaries and citation compatibility.

  Establish docs/authoritative_design/sections/ as the directory for extracted authoritative sections. Initially create:

  - reading-guide.md, owning §§0.1, 0.3, and 0.4.
  - design-change-workflow.md, owning §24.4.

  Keep subsystem contracts, §§0.5–0.6, and the remainder of the blueprint in place.

  Implementation requirements:

  - Extend the existing section resolver to scan numbered headings in the blueprint and the extracted-section directory.
  - Require each section identifier to have exactly one authoritative owner. Exclude historical proposals and migration stubs from
    ownership discovery.

  - Preserve section numbers and the existing blueprint §… citation vocabulary.
  - At each vacated location, retain its previous HTML anchor and a short link to the new owner. Do not retain a second copy of its
    normative prose.

  - Preserve moved subordinate anchors as well as the main section anchor.
  - Update live navigation and instructions to link directly to the new owners. Existing historical citations remain unchanged.
  - Produce the rendered section directory from the same resolver; do not introduce a separately maintained section registry.
  - Keep revision history in one place and add the required amendment row and decision references.
  - Make relocation failures mechanical: duplicate identifiers, missing cited sections, or broken legacy links fail checks.

  Acceptance: existing ADR section references resolve unchanged, legacy browser links reach a useful pointer, and each extracted section’s
  substantive text has one owner.

  ### P05 — Align skills, rules, and review tracking

  Responsibility: consistent contributor and agent behavior.

  Update shared instructions, documentation/decision rules, ADR and design-review skills, canonical role instructions, and relevant
  templates to establish:

  - Authoritative architecture is a document collection with stable section identities.
  - Reviews begin with the relevant reading path, contract, and change scenario.
  - Source pointers identify useful modules or entrypoints; they are not exhaustive symbol lists.
  - Ordinary implementation changes need documentation updates only when an enduring contract, explanation, or workflow changes.
  - Structural automation checks publishing and identity. It does not prove architectural conformance.
  - Evidence comes from relevant implementation inspection and existing tests or measurements, proportionate to the claim.
  - Plans own current dispositions; reviews retain historical observations.
  - New source seals, proof manifests, mandatory finding-to-test matrices, and routine whole-system requalification are not prerequisites
    for documentation changes.

  Preserve scientific and runtime verification obligations already owned by product plans. Regenerate Codex roles from their canonical
  definitions rather than editing generated files.

  Acceptance: no active instruction still requires a single authoritative file, manual SUMMARY maintenance, or documentation-specific
  semantic proof bookkeeping.

  ### P06 — Connect installation, CI, and final qualification

  Responsibility: one reliable build path and complete removal of the replaced path.

  - Read tool versions from the publishing declaration in local installation and CI.
  - Remove mdBook from the unrelated unversioned bootstrap tool list; teach doctor to read the documentation-tool declaration.
  - Keep the existing prebuilt installation mechanisms; do not write a binary downloader.
  - Route both local and CI builds through the adapter.
  - Preserve the required docs / build job name, Pages artifact directory, and existing main-only deployment behavior.
  - Retain offline fragment checking with lychee and existing ADR/register checks.
  - Keep same-commit rustdoc inclusion optional. Its absence must not trigger native compilation or fail documentation publication.
  - Update validation path references for the removed SUMMARY and actual book configuration location, without expanding source-receipt
    machinery.

  - Correct command documentation: just docs proves rendering and indexing; the CI link step proves internal-link resolution.
  - Remove obsolete search configuration, navigation writers, stale command examples, and replaced tests.

  Acceptance: local and CI publishing use the same implementation, a failed render/index/link check prevents publication, and no alternate
  legacy publishing path remains.

  ## 4. Verification and closure

  Use targeted adapter tests while implementing. Run the complete applicable documentation/tooling checks once at plan close.

   Area                       Required verification
  ━━━━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   Discovery and staging      New/deleted documents, title fallback, duplicate paths, supporting assets, unchanged tracked sources
  ─────────────────────────  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────
   Authority and citations    Unique section ownership, unresolved citations, moved sections, legacy anchors, historical-document
                              exclusion
  ─────────────────────────  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────
   Search                     Default Current scope, Reference/History isolation, Everything, working result anchors, duplicate-page
                              exclusion
  ─────────────────────────  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────
   Deployment paths           Local root and /pse-arrow/ prefix, nested pages, correct edit links
  ─────────────────────────  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────
   Failure behavior           Missing tools, failed mdBook/Pagefind processes, failed rebuild preserving the last successful site
  ─────────────────────────  ──────────────────────────────────────────────────────────────────────────────────────────────────────────────
   Optional reference         Successful build both with and without a supplied rustdoc artifact

  Final checks include:

  - just docs-test, just setup-test, just adr-lint, and just lint-agents.
  - The complete just docs build and CI-equivalent offline link check.
  - Applicable Python, shell, workflow, spelling, licence, and whitespace checks for changed tooling.
  - One browser verification of representative architectural, current-plan, library-reference, and historical queries.

  During browser verification, inspect transferred search assets and record the observed behavior once. Confirm that queries use Pagefind’s
  divided assets instead of loading the old roughly 26 MB search-index file. Do not create a permanent ranking benchmark, screenshot suite,
  or source-proof archive.

  Record commands, conditions, and failure counts in the plan’s final Verification and Outcome. Close Plan 17’s warning by linking this
  result; preserve its original historical check result. Process effectiveness remains unpiloted, as requested.

  ## 5. Boundaries and future migration

  - Preserve concurrent work in the shared checkout; this plan owns documentation, publishing tooling, and directly related workflow
    integration.

  - The reported stale native environment is outside this scope. Documentation must build independently of it.
  - Formal ADR adoption and remote publication follow existing repository procedures; local implementation does not imply either occurred.
  - Do not bulk split the remaining blueprint. Extract a subsystem section when substantive work next changes that subsystem, preserving
    its identifiers and legacy links through the established mechanism.

  - Add another automated rule only when it catches a concrete recurring mechanical defect cheaply. Architectural judgments remain review
    arguments grounded in the relevant code and contracts.