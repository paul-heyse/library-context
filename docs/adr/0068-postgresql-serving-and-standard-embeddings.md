---
id: ADR-0068
title: Serve immutable PostgreSQL projections with Rust services and standard 1024 embeddings
status: accepted
date: 2026-09-27
supersedes: [ADR-0065, ADR-0066]
superseded-by: null
design: [§B13, §B14, §6.4, §6.5, §11, §11.1, §11.2, §11.4]
evidence: Proposed
revisit: Qualified query or lifecycle evidence favors another owner; projection fidelity fails; or existing native and resource limits prevent a named consumer.
---

## Context

The operator accepted the expanded PostgreSQL target and the PG8–PG11 execution plan on
2026-09-27. [The expanded review](../design_review/reviews/design_review_postgresql-expanded-architecture_2026-09-27.md)
identifies whole-generation readiness, query ownership and Arrow conversion boundaries. PG0–PG7
are deployed and qualified; the expanded target is not thereby tested. The operator selected 1024
for all new retrieval and analytics embeddings; no additional dimension-quality gate applies.

## Options

1. **Retain file serving and add a vector-only database.** Simple initially, but retains Python's
   whole-generation relational maps and creates independent readiness/query owners.
2. **Rust SQLx services and immutable typed serving projections (chosen).** Reuse one transaction
   and migration family; compose pgpq, pgvector and a separate bounded read-provider adapter.
   A small effect crate lets the compiler and Python bridge share database code without linking
   DataFusion or Delta into the serving wheel.
3. **Python Psycopg/SQLAlchemy or an ORM for serving.** Adds a second query/schema owner for no
   Python-owned transactional domain. These remain conditional future capabilities.
4. **Replace canonical Delta or use PostgreSQL to interpret behavior.** Rejected: changes the
   semantic authority and replay contract without an earned requirement.

## Decision

**Accepted target, 2026-09-27.** `cpg-schema` owns pure domain contracts and projection validation.
`lctx-postgres` owns SQLx effects, configuration, cache/operations, migrations, COPY and serving
repositories. `cpg-core` retains Delta publication and projection construction. A separate
`lctx_storage` extension owns asynchronous PostgreSQL effects; `lctx_semantics` stays pure and
bounded. One process Tokio runtime serves lifespan-owned pools, never a runtime per request.

Delta remains canonical, including exact `used_embeddings` receipts (ADR-0067). Cache winners
are insert-only, read back in a subsequent READ COMMITTED statement, then frozen per attempt.
Bundle/projection construction never consults the mutable cache. Operational events are durable
history; discovery rows are reconciled indexes and never establish semantic publication.

A projection identifies a published snapshot, definition/schema/spec and complete relation and
native/lexical artifact manifests. Content identity excludes locations, attempts, batching and
index readiness. A ready projection is immutable and retained. Publication follows loading →
validating → ready, with a write barrier before validation and no cross-store transaction.
Read-only roles see ready generations only. Importer writes use bounded COPY staging followed
by validated inserts; PostgreSQL row security does not support COPY FROM into protected tables.
All meaningful foreign keys include generation identity. Missing evidence is an error, never
an abbreviated answer. Ready profiles are separate immutable records, so adding HNSW does not
change a pinned exact reader. Native IPC remains a digest-checked, bounded immutable artifact.

FastMCP pins one generation/profile for its lifespan. It never reads Delta or runs compiler code.
Rust owns relational selection and full evidence hydration; Python owns transport/Pydantic,
lexical scoring and the registered fusion policy. The existing native semantic boundary is
preserved without accepting proposed ADR-0025. No Python condition interpreter is introduced.
PostgreSQL loss is explicit unavailability, never an automatic file/generation fallback.
Portable file generations remain export/recovery/reference artifacts; the current file-backed
runtime stays until PG13 qualification, then its relational selectors/maps are removed.

Use SQLx 0.9, pgvector Rust 0.4.2/extension 0.8.6, pgpq 0.12, PyO3 async runtimes 0.29 and the
qualified DataFusion55/Arrow59 PostgreSQL provider plus federation0.5.7. The owned provider fork
has an immutable revision and separate bounded read pool; it is not an application write owner.
ADBC and SQL-wire services remain deferred. PostgreSQL catalog types never define Arrow meaning;
strict declared codecs reconstruct domain metadata and reject incompatible values.

Use separate migration, application, importer and serving credentials and explicit administrator
extension installation. Runtime roles cannot alter application schemas or ready content.
Trust, timeouts and combined connection budgets are enforced for both client families. Protect
backups/configs and preserve the PG7 database/binary/store together: its old schema check rejects
an upgraded database. No ready-generation deletion is implemented without reader protection.

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
  of `lctx_mcp`. 1,024 dimensions, `Float32`, cosine; both clients send `dimensions=1024`. The versioned spec records MRL prefix selection before L2 normalization and the vLLM admission overrides.
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
- Vectors are reused by `spec_hash + input_hash` through SQLx-owned PostgreSQL insert-only
  admission. One attempt-owned session freezes every committed value before exposing it to
  operation, E0 or brief consumers; repeated keys reuse the retained value. Each snapshot writes
  its complete `used_embeddings` receipt before validation/publication. Bundle construction reads
  that snapshot alone. Fake-provider unit fixtures may explicitly use an uncached session; the
  production cache has no automatic fallback. This record owns deployment and operational effects; ADR-0067 owns canonical publication.

## Consequences

PostgreSQL supplies transactional services and indexed serving while facts, behavioral semantics
and exact replay stay with their existing owners. The costs are an extension, physical mappings,
role/pool lifecycle and projection recovery. Source-level and focused evidence do not qualify the
whole target. The [PostgreSQL plan](../plans/postgresql-integration-plan_2026-09-27.md) owns PG8–PG17;
[forward-plan §6.1](../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings) owns
finding disposition. PG8–PG11 stop before production import, query cutover, ANN and federation.
No registry semantics, Stage 3 reasoning parameters or frozen evaluation targets change here.
