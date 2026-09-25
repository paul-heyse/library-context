---
id: ADR-0041
title: Publish a focused architectural collection with derived navigation and search
status: accepted
date: 2026-09-25
supersedes: []
superseded-by: null
design: [§2]
evidence: Proposed
revisit: A meaningful design change requires duplicate architectural statements or publication metadata, section moves break durable citations, or documentation builds require product qualification.
---

## Context

The operator approved the architectural documentation proposal and execution plan on 2026-09-25.
DESIGN is large enough that ordinary changes require excessive reading context. ADR-0040 already
owns architectural review principles and finding ownership; publication must serve that process.
This personal project is early in design and needs low maintenance under frequent change.

## Options

1. Keep one DESIGN file and direct search: minimal tooling, but growing context and weak navigation.
2. Focused Markdown with generated mdBook navigation and Pagefind (chosen): source remains usable
   directly, with static search and no hosted service or Node application.
3. A semantic documentation/proof platform: more automation, but another maintained model without
   a current consumer. Do not adopt source seals, symbol inventories or code-to-document proof maps.

## Decision

- Architectural authority is DESIGN plus `docs/design/sections/`. Preserve section IDs and old
  fragments with relocation pointers. One resolver serves ADR validation and the generated directory.
- Keep ADR-0040 in force. Architecture owns contracts/targets, executable declarations own details,
  ADRs rationale, reviews dated findings, plans current scheduled disposition and STATUS the checkpoint.
- Publish declared source collections with a small stdlib adapter. `docs/site.toml` owns selection,
  ordering, current work, URL settings and tool pins; `docs/book.toml` owns rendering choices.
- Generate staging/navigation/search only under ignored `build/docs/`. Render with mdBook, search
  with Pagefind's bundled Component UI and check offline links/fragments with lychee. Replace the
  last successful artifact only after all stages succeed. Preserve source edit paths and revision.
- Search defaults to Current; Reference, History and Everything remain accessible. Derive lifecycle
  from existing records and standard manifest. Do not duplicate progress or architecture authority.
- Include evidence READMEs and bounded selected assets. Omitted tracked sources link to the build
  revision; optional ignored capability skill references are visibly local, not fake public links.
- Provide isolated local commands and a docs-only CI artifact, without public Pages deployment.
  Bootstrap installs tools; routine builds do not install the product or invoke product qualification.
- Documentation-tool pins use focused docs tests/build checks. Product dependency/toolchain pins
  retain the existing pin-check gates. Documentation edits follow changes of meaning and boundaries;
  internal implementation changes require no new architectural evidence packet.

## Consequences

The accepted policy supplements ADR-0040. Implementation and improved reading/change cost remain
Proposed until exercised; acceptance of this record does not certify the publisher. The execution
plan owns implementation and review dispositions. Existing product findings and qualification
remain open. Site search and mechanical checks cannot judge architectural quality or historical
truth. Further section splits follow real reading needs, with no fixed page budget.
