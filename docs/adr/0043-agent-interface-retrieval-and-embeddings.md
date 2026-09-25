---
id: ADR-0043
title: FastMCP interface over one pinned generation, in-process hybrid retrieval and one hashed embedding spec
status: accepted
date: 2026-09-25
supersedes: [ADR-0010]
superseded-by: null
design: [§B13, §B14, §11]
evidence: Proposed
revisit: The corpus exceeds ~10⁵ vectors at 4,096 dimensions or needs filtered ANN with managed FTS (adopt LanceDB); the Rust and Python embedding clients disagree beyond tolerance on the conformance inputs; the embedding model, tokenizer or serving revision changes, or another endpoint must be accepted (a new spec hash, and plan W16's deployment-identity work); or ADR-0025 is accepted, replacing the lookup-only executor clause below.
---

## Context

Coding agents reach the compiled model through an interface that must be local, deterministic
over one published generation, and honest about what it does not know. The operator chose FastMCP
for that interface. Retrieval needs both exact names and task-language queries; the vectors come
from a large local embedding model whose numerics are not bitwise reproducible across requests.

This record replaces ADR-0010, restating the clauses of it and its amendments that are still in
force as one current decision. The gold-scoring and evaluation clauses ADR-0010 also carried
belong to evaluation ([§12](../design/sections/validation-and-evaluation.md#section-12)), not
here. ADR-0025 (proposed) would replace only the executor clause below.

Checked facts behind the decision (2026-09-22, against the pinned releases): FastMCP 4.0.5
provides lifespan state, pydantic output schemas, `ToolAnnotations`, the in-memory `Client` and
both protocol eras; vLLM 0.30.0 with `--runner pooling` L2-normalizes Qwen3-Embedding-8B's output
(every norm 1 ± 1e-7, Tested), rejects `dimensions` without a Matryoshka override, and expects the
query format `Instruct: …\nQuery:…` with no space; identical requests differed by up to 3.8e-3 in
a component (Measured), so exact vector equality cannot be an oracle. FastMCP 4.0.5's start banner
makes an HTTP GET to PyPI unless disabled. LanceDB 0.39 sits on a different Arrow/DataFusion line
from ours.

## Options

1. **A Rust LanceDB worker with its own dependency set.** Rejected: a third dependency train and
   an IPC protocol for a corpus exact search handles.
2. **LanceDB Python inside the FastMCP server.** Viable, and it keeps LanceDB's Arrow line sealed
   inside its wheel, but it adds a dependency with its own pyarrow constraint, an untested
   Python 3.14 path and FTS index management at a size where exact search is cheap. Deferred
   behind a size trigger.
3. **In-process exact hybrid retrieval over the generation (chosen):** BM25, exact cosine over
   cached vectors, reciprocal-rank fusion and exact-symbol promotion in the server itself.
4. **Embedding inside the compiler or the server process.** Rejected: vLLM and its CUDA stack
   would enter both locks, and the GPU job would compete with everything else. A separate
   service with a hashed spec keeps both clients small.

## Decision

**Server and generation boundary.**
- A FastMCP server, `python/lctx_mcp`, serves one pinned, immutable serving generation per
  process, read with a pinned pyarrow. It never reads Delta, runs DataFusion or compiler code, or
  touches the network except the configured local embedding endpoint.
- **Startup checks.** The lifespan loads the generation once and rejects it if its manifest
  format, condition-kernel format, per-file schema digests (from `cpg-schema`'s canonical schema
  form) or `embedding_spec` hash differ from what the server and its query client expect. The
  manifest names the library and requirement; a request for another library is an error. A
  generation carries every file its tools read (briefs, operations and behaviors,
  `operation_facet_status`, conditions and the singleton/ambient-read/place-claim files); the
  current format number and file list are [§6.4](../design/sections/storage-and-publication.md#section-6-4)'s.
- **Executor.** Tools answer by lookup over **materialized** rows (pyarrow compute and indexed
  dictionaries): no SQL string is built and nothing recurses at serve time; paths are
  precomputed as summaries and witnesses; results carry row caps, a `truncated` flag and
  cursors. Python never re-implements predicate or condition semantics and has no condition
  evaluator. ADR-0025 proposes a native executor for bounded semantic queries; until it is
  accepted this clause is current.

**Tools.** All take typed pydantic input, return objects (never bare lists) and carry
`ToolAnnotations(read_only_hint=True, idempotent_hint=True, open_world_hint=False)`.
- `search_capabilities` (hits with outcome status, relevance, rank source, promotion, mode and
  coverage) and `get_capability` (the whole brief by id), plus a
  `capability://{snapshot_id}/{capability_id}` resource sharing the hydration code. Hydration
  is a deterministic lookup, never a second search.
- The behavioral tools: `get_operation` (the whole record of one public operation),
  `find_operations` (exhaustive over materialized rows; never vectors), `search_operations`
  (ranked discovery, labelled as such) and, in Stage 4, `lookup_concepts` and `explain` (the
  stored derivation only).
- **`find_operations`.** `where` is a conjunction of facet equalities, `kind` and `path_prefix`,
  with no negation. Only `established` and `conditional` facet rows match. `complete` and the
  `unknown` list are read from the served `operation_facet_status`, never decided in Python: an
  operation that matches or is open on every term but is not a match hides a possible match,
  `complete` is true only when none does, and `unknown` lists them (capped at 50, with a count).
  `complete` is never "not truncated". The cursor binds the generation key, the request's hash
  and an offset. Facet names are held to the codebook by `specs/serving/facets.json`.
- **Errors.** One domain exception from the shared code, FastMCP's `ValidationError`: an error
  result in tools and invalid params (−32602) in the resource, never an internal error; all
  other exceptions are masked (`mask_error_details=True`). Embedder failures are caught inside
  search, which answers lexical-only and says why.
- **Transport.** stdio, started with `mcp.run(transport="stdio", show_banner=False)` and
  `FASTMCP_CHECK_FOR_UPDATES=off`. Nothing writes to stdout at import or in the lifespan; a
  `StdioTransport` subprocess test guards it. Not adopted: response-caching middleware (it would
  keep serving a degraded result), response-limiting middleware (it drops structured output),
  and `fastmcp install`/`fastmcp.json` (unpinned).

**Retrieval** (DESIGN §11.2).
- Lexical BM25 by `bm25s` (Lucene method) over our own tokenization (lower-cased runs of letters
  and digits). A brief's lexical text is its document text plus each distinct **name** token of
  its seed's **own** public spellings once; Rust and Python share known answers in
  `specs/serving/tokens.json`.
- The lexical leg scores only query words that occur in some briefs but not all, and abstains
  when there are none; the vector leg (exact cosine with the instruction-prefixed query vector,
  a brief scored by its best chunk) always votes.
- Reciprocal-rank fusion with K = 60 and 1-based ranks; ties by the lower brief id.
- Exact-symbol promotion by **any** public spelling of a brief's seed (`symbol_map` from
  `public_paths`), recorded as promoted.
- Degraded lexical-plus-symbol mode when no query vector is available, reported as such.
- These rules and parameters are registered. None changes on the strength of gold scores; a
  change needs an ADR whose rationale is independent of the gold.

**Embeddings** (DESIGN §B14, §11.1).
- Qwen3-Embedding-8B at a pinned revision (`docs/pins.md`), served by a **separate vLLM
  service** from a locked uv project (`services/vllm`, `just embed-serve`), never a dependency
  of `lctx_mcp`. Full 4,096 dimensions, `Float32`, cosine; `dimensions` is never sent.
- **One hashed spec governs every vector**: model and revision, tokenizer revision, vLLM version,
  served dtype, pooling, query instruction and template, document template, dimensions, output
  dtype, normalization and the document token cap. It is committed canonical JSON whose SHA-256
  is `spec_hash`. Documents take no prefix, so briefs and operation views share one vector
  space, and one query instruction serves every search tool until the structured evaluation
  attributes misses to its wording. Every embedded document must be within the spec's token
  cap as the spec's tokenizer counts it; byte limits only prepare texts.
- A **view** (signature and docstring, source body, later others) is a column of the documents
  and vectors, not part of the cache key; identical texts share one vector.
- Compile-time vectors come from Rust (`reqwest`), query-time vectors from Python (`httpx2`).
  **Conformance oracle:** over shared inputs both clients build byte-identical request bodies,
  apply the same rejections and judge responses alike; against the live service their vectors
  agree to cosine ≥ 0.9995.
- Vectors are cached by `spec_hash + input_hash` in the canonical `embedding_cache` Delta table,
  written insert-only; snapshots record the cache version they read and generations copy the
  committed vectors. A deterministic fake embedder with its own spec keeps tests GPU-free.

## Consequences

- No third dependency train and no retrieval worker; retrieval is testable with
  `fastmcp.Client` against a fixture generation. LanceDB waits behind its trigger (DESIGN §13)
  and, if adopted, lives in an isolated workspace.
- Two embedding clients are the price of keeping the compiler free of a language crossing; the
  shared conformance corpora keep them honest.
- Evidence: the server, retrieval, spec, conformance and cache-merge clauses are **Tested**
  (DESIGN §11 names the tests); the Stage 4 tools are **Proposed**, which sets this record's
  floor.
- Known gaps against this decision, owned by the forward plan: cache fill has two admission
  paths and one returns uncommitted local vectors
  ([W9](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)); spec-hash
  equality does not identify the running deployment
  ([W16](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)); lookup
  drops facet-value verdicts and finding-backed claims lose served support closure
  ([W2, W3](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
  Accepting this record does not close them.
