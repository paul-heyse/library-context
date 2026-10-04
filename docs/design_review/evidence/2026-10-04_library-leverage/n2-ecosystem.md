# N2: code-intelligence and Python-ecosystem candidates

Lane N2 of the [library-leverage review](README.md). Baseline `main` at `948b2a88`, using the
coordinator's [B0 baseline](b0-baseline.md). This lane makes no adoption decision: by binding
rule K1, product need and implementation fit are judged separately, and the availability of a
library is not a need. Licences are not judged (K2).

**Method and evidence level.** Registry metadata came from the crates.io API, the PyPI JSON API,
the npm registry and the GitHub API (`gh api repos/...`), all queried 2026-10-04. Library
contracts came from Context7 (`/mkdocstrings/griffe`), GitHub READMEs and source at the cited
tags, and the shared library skills (`python-analyzers`, `python-oracles`, `fastmcp`). Repository
facts come from static reading of the files cited at `948b2a88`. Corpus counts were measured with
`grep`/`find` over the acquired FastMCP 4.0.5 environment (`build/envs/fastmcp`) and its fetched
tree (`build/sources/fastmcp/004bf15a…`). No probe, build or test was run, so every outcome is
`not_run`. Every candidate below is **Proposed** at most.

Invariants used for fit:
- **hermetic**: pinned inputs and no ambient discovery (§4.0, CI hermetic inputs);
- **§B11**: no generative model;
- **unknown**: unknown is not absent (CI-04), and there are no relabelled relations (CI-02);
- **attribution**: every fact has a provider and basis (§B6).

---

## 1. Code-intelligence interchange and name binding

### 1.1 SCIP (protocol, Rust `scip` crate, scip-python)

- **Versions.** Protocol and Rust crate `scip` 0.10.0, released 2026-09-03, Apache-2.0, MSRV
  1.81. The repository has moved from `sourcegraph/scip` to
  [scip-code/scip](https://github.com/scip-code/scip), last pushed 2026-10-02. The crate's only
  non-optional dependency is `protobuf =3.7.2` (rust-protobuf, not prost).
  [scip-python](https://github.com/sourcegraph/scip-python) is at 0.6.6 on npm, released
  2025-09-05. It is a pyright fork whose `pyright-internal` reports version **1.1.301**, an early
  2023 base.
- **What it provides.** Per `scip.proto` at v0.10.0:
  - `Index` → `Document` → `Occurrence`, carrying a `symbol_roles` bitset (`Definition`,
    `Import`, `ReadAccess`, `Generated`, `Test`, `ForwardDefinition`, …);
  - `SymbolInformation` with `display_name`, `signature_documentation` and
    `Relationship{is_reference, is_implementation, is_type_definition, …}`;
  - `enclosing_range`, `external_symbols`, and an explicit `PositionEncoding` for line/character
    ranges;
  - a standard symbol string, `<scheme> <manager> <package-name> <version> <descriptors>`. This is
    a package- and version-qualified moniker such as `scip-python python fastmcp 4.0.5
    fastmcp/FastMCP#add_tool().`
- **Contract limits.**
  - SCIP has no provenance, modality, coverage or "unknown". A missing occurrence is
    indistinguishable from an unresolved one, and relationships are bare booleans.
  - Ranges are line/character, not byte offsets, so an exporter needs a line index (we already
    have one).
  - In this lane's reading, the symbol grammar has no notion of overload variants or of
    source-versus-effective signatures.
- **Consumer.**
  - *Export:* **no current consumer.** §14.11 makes "a concrete external-schema/client
    consumer" the trigger. That trigger would fire if an agent or IDE client (Sourcegraph, an
    editor, a code-search tool) were named as a consumer of our identities.
  - *Moniker design idea:* the package+version+descriptor moniker is a possible **external
    alias** for §14.4 public identity, for joining our catalog to other tools' indexes. It would
    never replace our content-addressed IDs.
  - *Oracle (scip-python):* in principle this is the only *independent* resolver in this survey,
    because pyright shares no code with Ruff, Pyrefly or ty. It could cross-check
    `reference_resolutions` and public access paths. **Fit is poor today:** the base is pyright
    1.1.301 (2023), its support for 3.12+ syntax is unverified, and the pilot runs on CPython
    3.14. It needs Node.js and drives a pip/importlib environment probe (its 0.6.6 fix "locating
    packages using importlib") that conflicts with hermetic inputs unless it is sandboxed.
- **Fit.**
  - Export is fine as an explicitly *lossy, labelled projection* of supported facts, never
    re-imported (CI-02).
  - Unknown and candidate rows cannot be expressed, so they must be omitted with a disclosed
    coverage note.
  - An export would need its own generation-bound artifact under §B12.
- **Burden.** The Rust crate is small. `protobuf` 3.7.2 is a new family that does not overlap
  Arrow or DataFusion. The exporter would map identities to symbols, which is the real work. As
  an oracle, scip-python means Node plus an isolated environment.
- **Duplicates an owner?** Not as an export. As a resolver it would be a parallel analyzer if
  used in production, which §14.11 forbids. It is acceptable only as an oracle.

### 1.2 LSIF

[microsoft/lsif-node](https://github.com/microsoft/lsif-node) was last pushed 2026-09-29. The
Rust crate `lsif` is 0.0.1 (2018). Sourcegraph moved from LSIF to SCIP. LSIF is a graph dump
(vertices and edges) with weaker identity than SCIP. **No consumer.** It is dominated by SCIP for
any export trigger. Reject.

### 1.3 GitHub stack-graphs / tree-sitter-graph

[github/stack-graphs](https://github.com/github/stack-graphs) is **archived**
(`archived: true` from the GitHub API, 2026-10-04; last crate releases 2024-12-13: `stack-graphs`
0.14.1, `tree-sitter-stack-graphs-python` 0.3.0).
[tree-sitter-graph](https://github.com/tree-sitter/tree-sitter-graph) 0.12.0 had its last push on
2024-12-11. Both would add a tree-sitter parse beside Ruff's, which is a second syntax authority
(§B1). **Reject:** the project is unmaintained and duplicates an owner. The design idea, name
binding as path-finding with *partial paths* precomputed per file, is already covered in spirit by
our per-module facts plus normalized binding.

### 1.4 Glean schemas and Pyrefly's Glean collector

- **Upstream.** [facebookincubator/Glean](https://github.com/facebookincubator/Glean) was pushed
  2026-10-04. Its Python schemas are `python.angle`, `python.xrefs.angle`,
  `python.branches.angle`, `codemarkup.python.angle` and `search.python.angle` in
  `glean/schema/source`.
- **In-family route.** Pyrefly's `report::glean::glean(&Transaction, &Handle)` is reachable
  in-process: `report` is already public through the fork patch. It emits `python.*.4` predicates
  (`XRefsViaNameByTarget.4`, `CalleeToCaller.4`, `DeclarationLocation.4`) with `src.ByteSpan`
  byte spans. These are type-resolved, so attribute references follow typed receivers. Sources:
  `python-analyzers` opportunity `glean-cross-references` and brief `xref`.
- **Limits.**
  - Predicate versions (`.4`) are part of the contract.
  - CLI fact order is not stable across runs, so rows would need sorting.
  - `convert::glean` unwraps `get_ast`, so it needs `Require::Everything`.
  - It offers no unknown or remainder representation, unlike Pysa's `Unresolved`.
- **Consumer.** None at present. DESIGN §13 defers it. The nearest consumers are §14.5
  "Related API … co-use" and §14.9's "captured incoming name references". The latter is
  implemented today, with ty navigation as a development-only oracle (§14, ADR-0121).
  - **Trigger:** a served "where is this member used" answer that is graded partial because the
    lexical resolver and Pysa miss attribute references through typed non-`self` receivers. This
    is the same shape as the forward-plan §7 "Pyrefly-typed receivers" row.
- **Fit.** Good: it is the same Pyrefly session and the spans join exactly, as an attributed
  provider surface. It must complement, never replace, `reference_resolutions` (§13).
- **Burden.** A decoder (`pyrefly_glean_schema` is not on crates.io; it is internal to the fork
  tree), a new fact family, and a harness-oracle pin like the one for Pysa.
- **Duplicates?** Partly, with Pysa call edges and lexical resolution. It must be modelled as an
  independent observation, not a merged edge set.
- **B0 note.** B0 groups "Pyrefly `parse_parameter_documentation`" with Glean as an open trigger.
  The parameter-documentation half has **already been adopted**. `crates/cpg-extract/src/docstrings.rs`
  plus `symbol_records.rs::parameter_docs` consume Pyrefly's parser, and the `python-analyzers`
  brief `docstrings` records its status as `used`. Only the Glean half is still open.

### 1.5 Kythe

[kythe/kythe](https://github.com/kythe/kythe) was pushed 2026-09-18; its latest release is
v0.0.76 (2026-07-16). No maintained Python indexer exists in this lane's search:
[kamahen/pykythe](https://github.com/kamahen/pykythe) was last pushed 2024-09-23, and
`google/pytype`, which some Kythe Python work used, is archived. **No consumer, no usable Python
route.** The design idea is noted in §7.

**CI-profile summary for §1.** Every interchange format above is *lossier* than our attributed
facts: none of them carries modality, coverage or unknown. They can be export targets or oracle
inputs, never authorities. An export must say which supported subset it contains. An oracle
disagreement is a finding, not a correction.

---

## 2. Python API-surface extraction (oracle or enrichment)

### 2.1 Griffe

- **Versions.** `griffe`, `griffelib` and `griffecli` 2.3.0, released 2026-09-04 (ISC).
  [mkdocstrings/griffe](https://github.com/mkdocstrings/griffe) was pushed 2026-10-03.
  Context7 `/mkdocstrings/griffe` was consulted.
- **What it provides.**
  - Static extraction through Python `ast` (plus an optional dynamic "inspection" mode): modules,
    classes, functions, attributes and aliases, with `lineno`/`endlineno` and signatures.
  - Labels, `Module.exports` (`__all__`), and docstring parsers for Google, NumPy and Sphinx.
    These produce typed sections (`DocstringSectionParameters`, `…Raises`, `…Returns`,
    `…Examples`, `…Warns`, `…Yields`, `…Deprecated`, …) with line numbers. Per the Griffe docs,
    the Sphinx parser only partly supports Examples, Warns and Yields.
- **Limits, measured by the shared `python-oracles` and `fastmcp` skill builders**
  (`python-oracles/build/extract.py` docstring, `build/semantic.py`):
  - `Module.exports` is `__all__` only. It misses PEP 484 redundant aliases (`from .x import Y as
    Y`).
  - It records `TYPE_CHECKING`-only imports as importable aliases.
  - It includes `if __name__ == "__main__"` members.
  - It cannot see namespace packages that lack `__init__.py`.
  - Annotations resolve only one hop.
  - It has nothing to say where an annotation is missing.
  - It gives line numbers, not byte spans.
- **Consumer.**
  - **Oracle role (§8.1):** an independent cross-check of `public_records.rs` (the "public"
    definition from Pyrefly's public-name helpers) and of signature parameter names, kinds and
    default-expression text. Griffe's `ast` walk shares no code with Ruff, Pyrefly or ty.
  - Its known divergences (listed above) are exactly the cases our model already handles
    explicitly: `TYPE_CHECKING` runtime view, partial `__all__`, redundant aliases. A
    disagreement report would therefore be informative rather than noise.
  - **No production consumer:** using Griffe as a production surface would be a parallel
    Python semantic engine (§14.11).
- **Circularity caution (CI-12).** The `fastmcp` skill, our **gold reference**, is itself built
  from Griffe (`fastmcp/build/extract.py`, `model.py`). Griffe-oracle agreement and gold scores
  are therefore *not* independent of each other. An evaluation must not count Griffe agreement
  as corroboration of a gold-scored item.
- **Fit.**
  - Hermetic when run with `uv run --with griffe==2.3.0 --no-project` over the captured
    site-packages, with static mode only (dynamic mode imports code).
  - It is a dev-only oracle, no generative model is involved, and it is attributed as an oracle.
- **Burden.**
  - Python only, which lives in the test lane beside Pysa and CrossHair.
  - It has no Rust family conflict.
  - Maintenance risk is low: the project is active and the version is single-sourced in the
    skill builders.

### 2.2 Docstring section parsers

- **Pinned in-family option: `ruff_linter::docstrings`, through the linked latest-Ruff fork**
  (`f7bdff69`).
  - `docstrings::sections::SectionContexts::from_docstring` gives Google and NumPy sections.
    `SectionKind` has 36 kinds, including `Args`, `Returns`, `Raises`, `Yields`, `Example(s)`,
    `Note(s)`, `Warns` and `SeeAlso`. Each section carries a `TextRange` for its name, summary
    and following lines.
  - `rules/pydoclint/rules/check_docstring.rs` parses Google and NumPy parameter entries
    (`ParameterEntry{name, range}`) and `Raises` names (`parse_raises_google/numpy` →
    `QualifiedName`).
  - **All of this is `pub(crate)`.** `lib.rs` declares `mod docstrings;` privately. The fork
    already exposes `pub mod semantic_facts`, so reaching these sections means adding a
    visibility hunk or an observer to the existing aggregate patch (§B8, §4.2.6 step 2).
- **Pyrefly `parse_parameter_documentation`.** Already used. It covers Sphinx and Google only,
  returns text without spans, and keeps only the first paragraph.
- **Python options (oracle only):**
  - `docstring_parser` 0.18.0 (2026-04-14, MIT; Google, NumPy, reST, Epydoc; no positions);
  - `numpydoc` 1.11.0 (2026-09-15; NumPy only, plus a validator);
  - Sphinx napoleon (`sphinx` 9.1.0, 2025-12-31), which converts to reST and is not a parser API.
- **Corpus measurement** (FastMCP 4.0.5 site-packages, `grep`, 2026-10-04):
  - Google sections: 452 `Args:`, 265 `Returns:`, 100 `Example:` plus 16 `Examples:`, 70
    `Raises:`, 9 `Yields:`;
  - 11 NumPy-style underlines, 0 Sphinx `:param`;
  - 11 doctest `>>>` lines;
  - **300 Markdown fence lines inside docstrings.** Generated pages also show reST `Usage::`
    literal blocks.
- **Consumer.**
  1. **Parameter descriptions after the first paragraph, and NumPy style.** These are currently
     a `Partial`/boundary (the `docstrings` brief in `python-analyzers`). There are few cases on
     the pilot. The trigger is a second library written in NumPy style.
  2. **`Raises` / `Returns` documentation as documented evidence (§14.5, §14.4 "documented"
     role).** No relation carries these today: a grep of `cpg-extract/src` and
     `lctx-model/src/domain` found none. `Facet::Raises` and `Predicate::BehavioralRaises` are
     behavioral, not documentary. The candidate consumer is a §14.7 requirement or packet section
     that cites the documented exception, which must be kept distinct from behavioral raises.
  3. **Docstring examples as scenarios (§14.5; §14.9 "at most two positive original
     scenarios").** About 116 example sections with fenced Python are not harvested: scenarios
     come from MDX, `examples/` and `tests/`. Ruff section ranges plus the **already pinned**
     markdown-rs could locate the fenced blocks by byte offset. This would follow the
     `source_code` materialization rule already used for MDX fences (§10.5).
  - Whether items 2 and 3 have a *need* is a product judgment that waits on the frozen PR0
    tasks.
- **Fit.**
  - Ruff ranges are byte offsets on the parse of record, and the Ruff family is the canonical
    syntax owner, so this would not add a second parser.
  - Section recognition is heuristic (pydocstyle rules), so it must carry a recognizer basis and
    an unknown result when the section style is ambiguous.
  - The Ruff and Pyrefly recognizers may disagree on parameter descriptions. Both must be kept as
    independent observations, not merged.
- **Burden.**
  - Patch hunk maintenance on every Ruff upgrade.
  - A new documentary relation and codes in `lctx-model`.
  - No new dependency.
- **Duplicates?** `docstrings.rs` (209 lines) would still own span location for Pyrefly's text.
  A Ruff route could replace its locate-again logic only if Pyrefly's parse were dropped. That is
  a decision between two in-family recognizers, not a new library.

### 2.3 libcst, jedi, typeshed

- **libcst** 1.9.0 (2026-07-29). Lossless CST plus a metadata framework (scope, qualified names,
  positions). It would be a second parser and scope analyzer beside Ruff, ty and Pyrefly.
  **Reject** under §14.11 (no parallel analyzer). It has no oracle need that Griffe or Pysa does
  not cover better.
- **jedi** 0.20.0 (2026-05-01). Completion and inference engine: a semantic Python engine,
  excluded by §14.11. **Reject.**
- **typeshed** ([python/typeshed](https://github.com/python/typeshed), pushed 2026-10-04).
  Pyrefly and ty bundle the stdlib stubs. For a third-party library without inline types,
  `types-<lib>` stubs would enter as an **ordinary locked dependency of the
  `libraries/<name>/` uv project**. Pyrefly reads `.pyi` already, so no new library is needed.
  `typeshed_client` 2.13.0 has no consumer. **Trigger:** a second analyzed library without
  `py.typed`. The decision then is whether stub-sourced signatures get their own §14.4 role,
  rather than being presented as "source".
- **Design reference, runtime surface (§14.11 "optional runtime surface inspection").** mypy's
  `stubtest` imports a module and compares runtime `inspect.signature` against static stubs.
  This is the shape of the isolated wrapper-effective-signature observation that §14.1's
  "unmodeled wrapper" journey leaves unresolved. Stdlib `inspect` in an isolated worker would be
  enough. There is **no consumer until** a frozen task is blocked by an unresolved wrapper
  signature (§14.2 says runtime inspection is conditional).

---

## 3. Packaging and deployment metadata

The bespoke code at `948b2a88`:

- `library.rs::normalize`, a PEP 503 name normalizer (hand-rolled; `pep508_rs::PackageName`
  normalizes too);
- `lock()`, a serde subset of `uv.lock` with unknown keys tolerated;
- `pyvenv()`, `version_triple()`, and `record_entries()` (RECORD through `csv`);
- `deployment_parser.rs`:
  - `metadata()` (mailparse headers, singleton and duplicate retention, pep508_rs and pep440
    validation);
  - `entry_points()` (rust-ini, hand-rolled object-reference and extras validation, duplicate
    retention);
  - `configuration()` (selected JSON pointers).

| Candidate | Version (date) | What it would replace | Fit and burden | Verdict |
|---|---|---|---|---|
| [python-pkginfo](https://github.com/PyO3/python-pkginfo-rs) | 0.6.8 (2026-02-24), MIT | `metadata()` | Parses METADATA into a `Metadata` struct and reads wheels and sdists (zip/tar/flate2). Depends on **mailparse ^0.16**, a second mailparse beside our pinned 0.17.0. It applies RFC 2047 decoding, which `deployment_parser.rs` deliberately does not ("no MIME word decoding"). Its struct model does not keep the duplicate-singleton and per-field failure rows that §14.6 requires ("Retain duplicate fields and interpretation failures") | **Reject.** Weaker contract than the bespoke code, plus a family duplicate |
| uv crates (`uv-pep508`, `uv-pep440`, `uv-pypi-types`, `uv-distribution-filename`, `uv-install-wheel`, `uv-lock`, `uv-resolver`) | 0.0.90 (2026-10-03), the component of uv 0.12.23. Local uv is 0.12.22 | `lock()`, entry-point validation, PEP 508/440 | The README of each crate states it "is an internal component of uv. The Rust API … is unstable and will have frequent breaking changes." `uv-pypi-types` pulls petgraph ^0.8, rkyv, toml_edit, jiff and more, and releases in lockstep with uv. The bespoke `uv.lock` read is about 30 lines over three fields | **Reject for now.** Trigger: `uv.lock` schema drift (lock `version`/`revision` change) breaks `lock()`, or we need marker evaluation against an environment. Pin it to the exact uv that writes the locks |
| `pep508_rs` / `pep440_rs` (current pins) | 0.9.2 (2025-01-02) / 0.7.3 (2024-12-04); repos pushed 2026-02 | already used | Maintenance has moved largely to uv's forks. No crate release for about 21 months | **Watch.** Trigger: a current-metadata construct (newer marker variables, Metadata-Version > 2.5) fails to parse. The replacement is then `uv-pep508` at an exact pin, with the unstable-API caveat |
| rattler (`rattler_conda_types` 0.55.0) | 2026-09-29 | — | Conda ecosystem. Our libraries are uv/PyPI | No consumer |

The entry-point object-reference check in `entry_points()` is a hand-written version of the
packaging spec's regex. A library would not improve it: no maintained standalone crate exposes
that validation, as far as this search found. The bespoke code is small and spec-shaped.

**Conclusion for §3.** The bespoke parsing is **thin glue over the already pinned
pep508_rs/mailparse/rust-ini/csv/toml**. It exists because §14.6 needs observation-preserving
semantics (duplicates, failures, original bytes) that the candidate libraries normalize away. No
displacement is recommended.

---

## 4. Documentation and examples

- **Markdown/MDX.**
  - FastMCP's corpus is `docs/**/*.mdx`: 594 `.mdx` files in the tree, of which `[tool.lctx.source]`
    selects a subset.
  - markdown-rs `markdown` =1.0.0 (2025-04-23) is the only Rust parser in this survey that
    parses **MDX** (JSX and ESM) with byte offsets. Pins record that it handled 144 of 148 guide
    pages.
  - [comrak](https://github.com/kivikakk/comrak) 0.55.0 (2026-09-06, active) and
    [pulldown-cmark](https://github.com/raphlinus/pulldown-cmark) 0.13.4 (2026-05-20) are
    CommonMark/GFM only. Applied to MDX, they would mis-read JSX components as HTML or text.
  - **No displacement.** Trigger for comrak: a library whose docs are plain GFM *and* a
    markdown-rs defect. Note that markdown-rs has had no release since 2025-04, a mild staleness
    risk.
- **Doc-example extraction.** See §2.2 item 3: docstring fenced examples are the gap.
  - The Ruff formatter (`ruff_python_formatter::string::docstring`) has a `CodeExampleKind`
    recognizer for doctest, reST literal and Markdown fenced blocks.
  - `ruff_python_formatter` is **not in `Cargo.lock`**, and that module is `pub(crate)`. This is
    a design reference only.
  - Python's stdlib `doctest.DocTestParser` is the reference for the 11 `>>>` lines. That number
    is too small to justify a consumer.
- **Notebooks.** The FastMCP tree has 0 `.ipynb` files. Ruff's `ruff_notebook` is already in the
  lock through the fork, and [runtimed `nbformat`](https://github.com/runtimed/runtimed) 3.0.0
  (2026-04-26) exists. **No consumer.** Trigger: a selected library whose examples are notebooks.
  Then `ruff_notebook`, being in-family, is preferred.

---

## 5. Retrieval and lexical ranking

**Current owner** (static reading).
- `lctx-model::domain::serving::ranking` fixes `RankingPolicy`: bm25s 0.3.11, NumPy backend,
  Lucene method, k1 1.5, b 0.75, and the tokenizer `LowercaseAsciiAlphanumeric`.
- `tokenize()` lowercases, then splits on every non-`[a-z0-9]` character. As a result,
  `addTool` → `addtool` and `StreamableHttpTransport` → `streamablehttptransport`. There is **no
  camel-case split and no stemming**.
- `python/lctx_mcp/src/lctx_mcp/retrieval.py` (68 lines) is a numerical callback. Rust supplies
  tokens and Python returns `bm25s` scores.
- ADR-0114 records "Retain bm25s as numerical scoring", which is a retention, not a rejection of
  a Rust scorer.

| Candidate | Version (date) | Consumer and fit | Verdict |
|---|---|---|---|
| **bm25s** (current) | pinned 0.3.11. **0.3.12 was released 2026-10-02** | `RankingPolicy::validate` and the adapter both check the version string exactly | **Pin finding.** An upgrade needs a policy revision. No need is shown |
| rust-stemmers | 1.2.0 (2019-11-17), Snowball | §14.8 lexical channel. ADR-0114 gives Rust ownership of tokenization, so **forward-plan §5's "PyStemmer, Stage 4 adopt" no longer matches the owner**: a Python stemmer cannot be applied to Rust-produced tokens without moving tokenization back to Python. If stemming is wanted, the in-owner route is a Rust Snowball stemmer. Risk: English stemming mangles identifier tokens (`settings`→`set`), so it would apply to prose fields or as a secondary token. The algorithm is frozen, so the old release date is a low risk | **Conditional.** Trigger: a frozen development task that misses because of inflection. Gated by §14.12 ("freeze a 24-task development set before changing retrieval") |
| PyStemmer | 3.1.0 (2026-05-22) | See the row above. Mismatched with current ownership | **Record as stale plan item** for the coordinator |
| Identifier-aware tokenization (no library) | — | The `[a-z0-9]+` tokenizer does not split camel case, so the natural-language query "streamable http transport" has no lexical overlap with the class name. That hurts the §14.1 discovery journeys. A camel/acronym splitter is about 20 lines. The `heck` crate converts case; it does not split for search. This is a **design observation, not a library candidate**, and it changes policy identity | Raise with the coordinator as a retrieval-policy question under the PR0 gate |
| Pure-Rust BM25 (`bm25` crate 2.3.2, 2025-09-07) | §14.8 | It would remove the Rust→Python numerical callback. But the crate **hashes tokens into u32/u64 dimensions** (collisions) and scores against a fitted `avgdl`. That conflicts with the existing exact-Lucene control (`test_bm25_scores_are_the_lucene_formula`). If the callback were to go, writing the Lucene formula directly in `lctx-model` is simpler and exact | **Reject crate.** Retaining bm25s is reasonable |
| [tantivy](https://github.com/quickwit-oss/tantivy) | 0.26.2 (2026-09-08), active | Full engine: BM25, tokenizer pipeline with stemmer, phrase queries, fuzzy (Levenshtein automata) for typo-tolerant names. **Contract limit:** BM25 uses 1-byte quantized fieldnorms, so its scores are *not* the exact Lucene formula on true lengths. It would be a second index store, which §B12 allows as a rebuildable projection. It brings a heavy dependency closure (rayon, time, uuid, typetag, …) | **No current consumer.** Trigger: F13 (a named fuzzy or phrase need, or measured lexical failure) |
| PostgreSQL built-ins: `pg_trgm`, FTS | PG18 contrib | `pg_trgm` adds no dependency and suits "did you mean" on mistyped member names (exact-symbol promotion, §14.8). FTS `ts_rank` is not BM25 | **Conditional (F13).** §14.2 already requires a measured gap |
| PG BM25 extensions | [pg_textsearch](https://github.com/timescale/pg_textsearch) v1.5.1 (2026-10-02, PostgreSQL licence); [ParadeDB pg_search](https://github.com/paradedb/paradedb) v0.26.0 (2026-10-03, tantivy inside); VectorChord-bm25 0.3.0 (2025-12-15) | Lexical scoring inside the canonical generation as a derived index. That would remove the Python callback and keep one store. Burden: an extension in the PG18 image and bootstrap (today the image is pgvector's), plus the scoring-exactness controls | **Conditional (F13).** pg_textsearch has the lowest licence and ops friction of the three. Record as an F13 option |
| HF `tokenizers` crate | stable 0.23.2; 1.0.0-rc.2 (2026-09-21) | §11.1 token admission. Today `lctx-embed` uses vLLM's own `POST /tokenize` ("no Rust tokenizer is needed"). A local tokenizer.json would allow admission without the service, but it would be a **second tokenization authority** that can drift from the served model (special tokens, instruction prefix) | **No consumer.** Trigger: admission must run with the service unavailable, or a measured `/tokenize` cost |

---

## 6. Unenabled shared skills

`.config/library-skills.toml` enables 16 skills. The 16 below are in the store but not enabled.
Each verdict comes from the skill's SKILL.md description, checked against repository consumers.

| Skill | Pinned scope (from SKILL.md) | Consumer here | Verdict |
|---|---|---|---|
| **pydantic** | pydantic 2.13.5, pydantic-core 2.46.5 | **Yes.** `python/lctx_mcp/pyproject.toml` pins `pydantic==2.13.5` "used directly", imported by `wire.py` and `embedder.py`. The versions match exactly | **Enable (config-only finding).** Useful for byte-stable JSON and strict mode in the presentation models |
| **vllm** | vLLM 0.30.0 server HTTP surface (`/tokenize`, embeddings fields, `vllm serve` flags) | **Yes.** `crates/lctx-embed` (the compile-time embedding client over vLLM, `/tokenize`) and the `--embedder vllm` path in `lctx_mcp`. **Version transfer caveat:** `docs/pins.md` pins a `0.30.1rc1.dev286+…` custom build, so the skill's 0.30.0 claims must be checked before transfer | **Enable, with a pin-check note** |
| stats-metrics | scipy.stats, statsmodels, scikit-learn, pytrec_eval | Latent. §14.12 says "report uncertainty" for paired A/B/C results, and retrieval diagnostics could use trec_eval measures. No current code; PR0 is blocked | **Trigger:** PR0 resumes and the protocol names an uncertainty statistic or retrieval metric |
| hf-tokenizers | tokenizers 0.22.2 Python bindings | None (see §5) | No consumer |
| arrow-polars-duckdb | pyarrow 25.0.1, polars, duckdb, Python DataFusion | **None found.** `pyarrow==25.0.1` is declared in `python/lctx_mcp/pyproject.toml`, but no Python or Rust file imports or exchanges pyarrow objects (grep over `python/`, `tests/`, `crates/`; incidental observation, may be a stale dependency) | No consumer. Flag the stale-looking dependency to the coordinator |
| python-postgres | psycopg, SQLAlchemy, Alembic, … | F6 trigger not fired: `lctx_mcp` reaches PostgreSQL only through the native Rust module | No consumer (F6) |
| lancedb | LanceDB 0.39.0 | §13 deferred: writes are not byte-reproducible | No consumer (F13) |
| deltalake | delta-rs | Delta store removed (P1.3/P1.4) | **Reject** |
| markdown-latex | markdown-it-py, pylatexenc | Markdown is parsed in Rust (markdown-rs). No LaTeX corpus | No consumer |
| mineru-docvortex | PDF extraction (MinerU, DocVortex) | No PDF corpus | Trigger: a library whose primary docs are PDF |
| gliner2-spacy | GLiNER2 (an encoder, non-generative), spaCy | Forward plan §7 "spaCy in compile" trigger (regex directive tagging misses conditions) has not fired. Under §B11 statistical output may only nominate | No consumer |
| cognee | LLM knowledge-graph pipeline (litellm/instructor) | Conflicts with §B11 and with §1.3 (no ontology store) | **Reject** |
| neo4j | Graph DB | §B10 excludes a graph database | **Reject** |
| jena | RDF/SPARQL/SHACL | §1.3 non-goal (no RDF store or reasoner) | **Reject** |
| native-solver-libraries | Ipopt, HiGHS, SCIP, … | §1.3 and §B10 (no general solver) | **Reject** |
| symbolica-faer-oximo | Symbolic and numeric maths | None | **Reject** |

---

## 7. Comparable systems: one design idea each

| System | Design idea relevant to §14 | Library we could adopt? |
|---|---|---|
| **Context7** ([upstash/context7](https://github.com/upstash/context7), pushed 2026-10-03) | Its parsing, crawling and API backend are private, per the README; only the MCP server is open. Per the Upstash blog ([new-context7](https://upstash.com/blog/new-context7)), it moved to **server-side reranking** to cut tokens returned. The idea for §14.9: hand back the *smallest answering unit* with a hard budget. We already have 32 KiB default packets and winning-unit witnesses. Context7's "neural reranker" is excluded for us by §B10. Its maintainer-authored per-library configuration parallels our `[tool.lctx.source]` corpus selection | No (closed service, and it is the comparator in §14.12) |
| **Sourcegraph** | It labels navigation as **precise** (from a SCIP index) or **search-based** (heuristic) in the UI. This matches §14.5's basis distinction (exact resolution / explicit cross-reference / ambiguous mention / similarity). The idea is to surface the basis label per relationship in the packet, not only in the witness | SCIP crate, only for an export trigger (§1.1) |
| **CodeQL** | (a) **API graphs**: a library's public surface modelled as access paths (`moduleImport("fastmcp").getMember("FastMCP").getReturn().getMember("tool")`). This is directly the §14.4 "public member = exposed owner/name/access path" notion, extended to *returned* objects. (b) **Models-as-data**: summary and source models as data rows with access-path strings (`Argument[0].Member[x]`), comparable to §B5's pinned authored models | No: CodeQL would be a parallel analyzer (§14.11). Idea only |
| **pdoc / mkdocstrings** | mkdocstrings **autorefs and the `objects.inv` inventory**: explicit doc-prose → API-object cross-references resolved by identifier. This is §14.5's "explicit cross-reference" basis. On the pilot, measured 2026-10-04, guide MDX pages contain **0** links into the generated `docs/python-sdk/` reference, so there is no consumer on FastMCP. Trigger: a library with a Sphinx or mkdocs site (`sphobjinv` reads inventories) | Not now |
| **Kythe** | **Anchors as first-class byte-offset nodes** with `ref`/`defines/binding` edges, and the VName identity (corpus, root, path, language, signature). This is close to our original-anchor and evidence model. It confirms byte offsets, not line/column, as the canonical coordinate | No (no Python indexer) |
| **Glean** | A **language-neutral `codemarkup` layer** derived over per-language schemas, and versioned predicates (`.4`). The idea: keep provider-specific facts versioned and derive one product-facing view, which is what normalized relations do. Its xref predicates are reachable through the Pyrefly collector (§1.4) | The Pyrefly in-process collector, only at its trigger |

---

## 8. Ranked shortlist (strongest consumer × fit)

1. **Enable the `pydantic` and `vllm` skills** (config only). There are existing direct
   consumers (`lctx_mcp` wire and embedder; `lctx-embed`). The pydantic version matches exactly.
   vLLM needs a version-transfer check against the custom 0.30.1rc1 build. No code changes.
2. **Griffe 2.3.0 as a dev-only public-surface and signature oracle (§8.1)** against
   `public_records.rs` and signature facts. It is independent of all three linked analyzers, and
   its known divergences are pre-measured in the `python-oracles` skill. **Caveat:** it shares an
   extractor with the `fastmcp` gold skill (CI-12).
3. **`ruff_linter::docstrings` sections through a visibility hunk in the existing Ruff fork
   patch.** It gives byte-ranged Google and NumPy sections: Raises, Returns, Examples, and
   multi-paragraph parameters. Candidate consumers are documentary §14.4/§14.5 evidence and
   docstring-example scenarios (about 116 example sections and 300 fence lines on the pilot), the
   latter combined with the already-pinned markdown-rs. No new dependency. Need is unproven until
   PR0 tasks exist.
4. **Retrieval tokenization policy (design, not library):**
   - the missing camel-case split, gated by the §14.12 development freeze;
   - the forward-plan PyStemmer item should become "a Rust Snowball stemmer (rust-stemmers) if a
     frozen task needs it";
   - bm25s 0.3.12 is available.
5. **Pyrefly Glean collector** at its §13 trigger (typed-receiver attribute references). It is
   in-process, joins by exact span, and is already reachable.
6. **F13 options, to be recorded as conditional:** `pg_trgm` for name suggestions; pg_textsearch
   (BM25 in PG18) if the lexical channel should leave Python; tantivy only for a fuzzy or phrase
   need.
7. **SCIP export / moniker**, only on a named external client. Treat scip-python as a stale
   oracle (pyright 1.1.301 base).

Rejected outright:
- stack-graphs and tree-sitter-graph (archived or unmaintained; second parser);
- LSIF (dominated by SCIP);
- libcst and jedi (parallel engines, §14.11);
- python-pkginfo (weaker contract and a mailparse duplicate);
- uv crates for now (an unstable internal API);
- the `bm25` crate (hashed tokens, inexact);
- the cognee, neo4j, jena, deltalake, native-solver and symbolica skills.

## 9. Uncertainties and absences

- **Search coverage.**
  - crates.io: 33 crate names queried by exact name.
  - PyPI: 15 packages.
  - npm: scip-python.
  - GitHub API: repository status and releases for 22 repositories.
  - Context7: `/mkdocstrings/griffe`.
  - Web search: Context7's architecture.
  - I did not browse lib.rs categories exhaustively. An empty result is not proof of absence:
    there may be Rust docstring parsers or entry-point validators this name-based search did not
    find.
- **Not verified by execution:**
  - Griffe 2.3.0 behaviour on CPython 3.14 syntax over the FastMCP environment;
  - scip-python on 3.14;
  - tantivy score divergence from exact Lucene (the 1-byte fieldnorm claim is from tantivy's
    documented design, not a probe);
  - whether a visibility hunk for `ruff_linter::docstrings` stays small across Ruff upgrades;
  - whether `pyarrow` in `lctx_mcp` is a needed runtime dependency for another reason (for
    example a FastMCP extra).
- **Context7's ingestion and ranking internals are private.** Statements about it come from its
  README and the Upstash blog, not from source.
- **Registry dates** are `created_at` of the newest version, or the GitHub release
  `published_at`. A repository `pushed_at` date can reflect branch activity rather than a
  release (for example, scip-python was pushed 2026-10-04 while its newest release is from
  2025-09-05).
- **The Kythe Python absence** rests on pykythe's staleness and pytype's archival. A private
  Google indexer is out of reach in any case.
- **The corpus measurements** are `grep` heuristics over docstring text. Section counts include
  any line matching the header pattern.

**Files written:** this file only. Scratch: `scip.proto` copy under the session scratchpad
`n2/` (not in the repository).
