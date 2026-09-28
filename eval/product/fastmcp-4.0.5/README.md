# PR0 product evaluation inputs

Evidence state: **Proposed**, 2026-09-28. This directory is evaluation-only; it is never a compiler
input. The forward plan §3.0/§6.2 remains sequencing and finding authority. `protocol.json` and
`development.json` were hashed before this functional work. `confirmation.sealed` is an independently
curated candidate, **not a qualified confirmation population**: see `confirmation-manifest.json`.
The implementation author has not read its plaintext or key. Gold and existing heldout inputs are
unchanged. Do not promote the candidate until an independent reviewer establishes task distinctness
and release/material parity; if it must change, record a new population and exposure, not an overwrite.

## Contract and consumer inventory

| Requirement / journey | Existing facts and preserved consumers | PR0–PR1 delivery; following package |
|---|---|---|
| R01 ordinary API inspection without deep analysis | public_names, declarations, signatures, parameters, docs; find/search/get_operation | Mandatory catalog, profile selection and finite capabilities |
| R02 complete API record | ParameterSyntax/Semantics/Docs, PysaFunctions/Classes, provider maps, TypeTerms/Args/Observations | Ordered observations, constructor associations, raw source evidence and explicit ambiguous exposures; effective surface normalization PR2 |
| R03 options, defaults and exact field relationships | parameter defaults, field/type facts, ambient reads/place claims | Preserve source expressions and requiredness; cross-field precision PR2 |
| R04 registration, decorators and inherited use | decorators, public paths/MRO, attributed provider identities, invocation phase | Preserve source/provider alternatives and defining ownership; normalized invocation models PR2 |
| R05 original scenarios | corpus source files, usage sites, enclosing scopes, mentions, examples/tests | Preserve corpus acquisition; catalog-wide scenario associations and original intent PR3 |
| R06 evidence closure and uncertainty | facts, source files, spans, condition/summary boundaries | Catalog subjects root original evidence independently of assertions; broader scenario closure PR3 |
| R07 customized discovery | operation facets/status, canonical graph projections, parameter/return/type predicates, SQL queries | Existing predicates and ranked search retained; task-facing bounded vocabulary/coverage PR4 |
| R08 useful bounded product packet and fair comparison | PG immutable projections, MCP structured results, response budgets | Typed API packet + concise text, explicit 32/256 KiB refusal; browse/explanations PR5, comparison PR6 |
| Deployment extension | pinned uv projects, source metadata, selected dependencies, docs/code/test bytes | Keep installation evidence separate from all-extras analysis context; requirements/extras/entry-point associations PR3 |

Core acceptance examples include FastMCP construction, tool/resource/prompt registration, client calls,
transports and configuration, middleware/auth choices and deployment. These are test subjects, never
a restricted public-root universe or authored capability taxonomy.

Existing behavioral `get_operation` fates, `inspect_value_paths`, condition kernels, SCC summaries,
exact query predicates, model catalogs, vector/lexical discovery, brief synthesis, federation/reporting,
ADBC/pgwire and recovery remain supported or explicitly retained with their prior obligations.
Additive specified capabilities with structured data remain in the forward plan's preservation inventory.
Catalog-first scheduling does not retire their schemas or contracts. No new general semantic research
is a PR0–PR1 prerequisite.

## Reproduction

- `uv run python scripts/product_eval.py check-freeze` verifies hashes and population shape.
- `uv run python scripts/product_evidence.py <pinned-git-checkout> --commit <40-hex> --inventory <new-path>`
  serves B's original Git blobs on localhost:8091. It ignores working-tree additions, including
  `_lctx_blocks`, and returns hashed source lines. C uses this same server plus the structured service.
- `run-development --task D01 --condition A --condition-config eval/product/fastmcp-4.0.5/condition-A.json --out <new-directory>`
  admits only reviewed, hash-bound parity receipts. Conditions currently point to a **blocked** receipt;
  endpoint lists must be frozen with the eventual matched evidence inventory before a trial.
- The runner sends the frozen model/reasoning and explicit MCP function schemas to the Responses API;
  no local filesystem/shell/browser/memory tools or user configuration reach the trial model. It records
  effective tool schemas, inputs, every response and call, usage, time, failures and terminal receipts.
  Tool count/output/wall limits stop further work; exceeding reported token limits invalidates the run.
  No missing usage is converted to zero. No model substitution, hidden retry or confirmation execution.
- `OPENAI_API_KEY` must be configured in the execution environment. Never place a key in this directory
  or receipts. API/model availability and effective-tool execution still need live qualification.

The four-stratum A/B development smoke is blocked on exact FastMCP 4.0.5 material parity and model
access; no trial score or comparative win is claimed. Context7's resolved version list is an availability
observation, not evidence that an API is absent. Review findings retain IDs EVAL/F01–F04 in the forward plan.

The runner follows the [OpenAI function-calling interface](https://developers.openai.com/api/docs/guides/function-calling)
and the locally pinned FastMCP 4.0.5 client contract. The protocol thresholds are unchanged.
