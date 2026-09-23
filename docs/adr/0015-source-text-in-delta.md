---
id: ADR-0015
title: Every analyzed module's text and role are stored in the snapshot
status: accepted
date: 2026-09-23
supersedes: []
superseded-by: null
design: [§3.2, §3.5, §4.0, §8]
evidence: Tested
revisit: A snapshot's stored module text outgrows its other tables (the pilot's is 3.0 MB compressed, against 269 MB for the store); a corpus needs a role the four codes cannot say; or a non-UTF-8 module that a consumer must quote appears on a pilot.
---

## Context

The C6 deep review (F2) found two reads with no source in the snapshot. Both belong to consumers
the usage run names (DESIGN §3.2 `docs`: Pass C examples and tests; §10.3–§10.5 usage patterns):
- **Text.** `source_files` held only `content_digest` and `byte_len`. Only doc blocks, passages
  and docstrings had text, so an example or test snippet had to come from the fetched tree. That
  is an input the snapshot pins only by digest, and it breaks the promise that derived output is
  rebuildable from Delta (§4.3, §B6, §6.4).
- **Role.** Whether a module is an example, a test or a doc block was only a path convention,
  because one `usage` glob list selected examples and tests alike. §10.2's `documented` status
  ("doc or example") and §10.5's trimming of test-only detail need the distinction.

The review left the choice to the operator. On 2026-09-23 the operator chose to store the text
in Delta.

## Options

1. **A content-verified re-read.** Stage F reads `build/sources/<name>/<commit>/<path>` and fails
   if its digest differs from `source_files.content_digest`. No storage cost. It loses because a
   published snapshot could no longer be served or rebuilt without the fetched tree beside it,
   and §6.4 would need amending.
2. **Store the text of the usage modules only.** It covers the named consumers. It loses to
   option 3 on uniformity: the library's own modules are 3.2 MB more, and with them every span in
   a snapshot resolves to text from Delta alone, with no role-dependent null.
3. **Store every analyzed module's text** in `source_files.text`, with its role in
   `source_files.role` (chosen).

## Decision

- **`source_files.text`** holds each analyzed module's text: the bytes every span of the snapshot
  indexes. It is null exactly when those bytes are not UTF-8 (`utf8 = false`, whose families are
  already `unavailable`).
- **`source_files.role`** (codebook `source_role`, appended) says what the module is to its
  library:
  - `release`: a module of the analyzed release;
  - `example`: selected by `[tool.lctx.source] examples`;
  - `test`: selected by `tests`;
  - `doc_block`: a selected document's Python block, materialized as a module.
- **`[tool.lctx.source]`**: `examples` and `tests` replace the single `usage` list, so each
  module's role is the key that selected it. A file both keys select is refused.
  `_lctx_blocks/` is cleared before any glob selects.
- **The corpus release id** hashes each usage file's role with its path and content.
- **Two rules** (§8), each with an injected case:
  - `semantic:source-text`: the text is present exactly when the bytes are UTF-8, and its byte
    length is `byte_len`. The content digest is BLAKE3, which SQL does not compute.
  - `semantic:source-role-by-run`: a module has the role `release` exactly when its run declares
    `exports`, which means the library or a source tree, not a corpus.

## Consequences

- **A snippet is read from Delta alone.** `a_corpus_documents_its_library` deletes the fetched
  tree and the environment after publishing, then slices each example and test call from its
  module's stored text by its span.
- **Cost, measured on the pilot** (2026-09-23, snapshot `9a0ec4de…`): 9.1 MB of text across 1,787
  modules:
  - by role: release 3.2 MB, example 0.35 MB, test 5.2 MB, doc block 0.39 MB;
  - 3.0 MB compressed in Delta per snapshot;
  - the compile's time (45.0 s) and peak (7.1 GB) are unchanged within their spread.
- **Migration** (DM-51): `source_files` gains two columns, `EXTRACTOR_OUTPUT_VERSION` goes to 16,
  and a pre-migration store fails with `SchemaDrift`.
- **A library definition must say which globs are examples and which are tests.**
  `libraries/fastmcp/pyproject.toml` and `libraries/README.md` are updated.
- **Harder:** every snapshot stores the text again. Snapshots are append-only and
  snapshot-qualified, so this is by design; the revisit trigger watches the size.
