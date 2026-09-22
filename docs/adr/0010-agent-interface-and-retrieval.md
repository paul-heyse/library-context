---
id: ADR-0010
title: FastMCP interface with in-process hybrid retrieval and one Qwen3 embedding spec
status: accepted
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§B13, §B14, §11]
evidence: Tested
revisit: The corpus exceeds a few thousand briefs, ANN or managed FTS is needed (adopt LanceDB), the Rust and Python embedding clients disagree beyond tolerance on the conformance vectors, or the embedding model or revision changes (a new spec hash, so every cached vector is re-derived).
---

## Context

The research input (IP L2145–L2230, L2640–L2965) proposes a Rust LanceDB worker. It notes that
LanceDB 0.39 is on Arrow 58 / DataFusion 54, a different dependency set from our Arrow 59.3 /
DataFusion 55.1, so batches would have to cross over IPC. The operator chose FastMCP for the
agent interface.

Verified 2026-09-22:
- **FastMCP 4.0.3 skill vs installed 4.0.5:**
  - lifespan state, Pydantic output schemas, `ToolAnnotations`, `ToolError`, stdio, the
    in-memory `Client`;
  - both protocol eras.
- **LanceDB v0.39.0 Python** (read from the tagged source): hybrid search, FTS, `RRFReranker(K=60)`,
  `bypass_vector_index`, cosine. Python's RRF uses 1-based ranks; Rust's uses 0-based.
- **vLLM 0.30.0** (installed source):
  - `--runner pooling` L2-normalizes the output;
  - `dimensions` is rejected without Matryoshka overrides;
  - the query format is `Instruct: …\nQuery:…`, with no space. The research input's template is
    wrong.

**Model change (operator decision, 2026-09-22).** The model is Qwen3-Embedding-8B instead of 4B:
the same family, instruction format and 32K context, at 4,096 dimensions. Context7
(`/qwenlm/qwen3-embedding`) gives the specifications.

Spike results (2026-09-22, branch `spike/pyrefly-inproc`, `analysis/SPIKE_RESULTS.md`, E1–E3;
all passed):
- **E1.** vLLM 0.30.0 serves revision `1d8ad4ca` on the RTX 5090: ~15.5 GiB of weights and ~27 GB
  in total at 0.80 utilization. Every vector has norm 1 ± 1e-7 (pooling `LAST` plus normalize,
  from the model's sentence-transformers config). That contradicts Context7's "not normalized by
  default" for this model and version.
- **E2.** The Rust (`reqwest`) and Python (`httpx`) clients build byte-identical request texts.
  **vLLM is not bitwise deterministic across requests**: identical inputs differed by up to
  3.8e-3 in a component (cosine ≥ 0.99988). The clients agree to cosine ≥ 0.99991.
- **E3.** `fastmcp.Client` round trips work in both eras (`2026-07-28` and `2025-11-25`), with
  object outputs and annotations. Generations with a mismatched schema or spec fail at connect.
  pyarrow was 25.0.1.

## Options

1. **A Rust LanceDB worker with its own dependency set** (the research input). Rejected: a third
   dependency train plus an IPC protocol, for a few dozen rows.
2. **LanceDB Python inside the FastMCP server.** Viable, and it keeps Arrow 58 sealed inside the
   wheel. Deferred.
   - Both options need pyarrow to read the bundle, so pyarrow isn't the difference.
   - LanceDB's cost is the `lancedb` dependency and lancedb's own `pyarrow<25` test constraint.
   - It also brings an upstream-untested Python 3.14 path and FTS index management.
   - All of that comes at a corpus size where exact search is expected to be cheap (a hypothesis,
     not measured).
3. **The simpler alternative, chosen:**
   - in-process exact cosine, BM25 and RRF over the bundle's cached vectors and lexical text;
   - exact-symbol promotion;
   - LanceDB behind a size trigger.

   This is the research input's own reference oracle (IP L3055), used as the implementation.

## Decision

- **Server.** A FastMCP server in `python/lctx_mcp` implementing DESIGN §11.3. It reads only the
  pinned serving generation, with a pinned `pyarrow` as its Arrow reader.
- **Startup checks.** The lifespan rejects a generation whose per-file schema digests (from
  `cpg-schema`'s canonical schema form) or `embedding_spec` hash don't match what the server
  expects.
- **Search output.** Hits carry outcome status, relevance, mode and coverage. An unknown library
  raises `ToolError`.
- **Retrieval.** DESIGN §11.2, including a degraded lexical-only mode and recorded exact-symbol
  promotion.
- **Embeddings.**
  - Qwen3-Embedding-8B at revision `1d8ad4ca…`, served by a **separate vLLM 0.30.0 service**.
    vLLM is never a dependency of `lctx_mcp`; the uncommitted `vllm>=0.30.0` project dependency
    moves to a `uvx`-run service recipe in increment 1.
  - The spec also hashes the vLLM version and the served dtype (bfloat16).
  - One hashed spec governs every vector. Compile-time vectors come from Rust (`reqwest`,
    already locked), query-time vectors from Python (`httpx`).
  - **Conformance oracle.** Over shared inputs, both clients must build byte-identical request
    texts, apply the same rejections, and produce vectors that agree to cosine ≥ 0.9995. An
    exact vector match is not a valid oracle, because vLLM is not bitwise deterministic (E2).
  - Vectors are cached by `spec_hash + input_hash` in the canonical `embedding_cache` Delta
    table and copied into each bundle.
  - A deterministic fake embedder keeps `just check` GPU-free.

## Consequences

- **No new dependency set.** No third Rust dependency train and no retrieval worker process.
  Retrieval is testable with `fastmcp.Client` against a fixture bundle.
- **Two client implementations** (Rust and Python) are the price of avoiding a language crossing
  in the compile pipeline. The conformance vectors are the oracle that keeps them honest.
- **The spikes before acceptance passed** (E1–E3 above). The 8B model fits the 5090 with room
  for KV cache. It still competes with other GPU work, which is one more reason vLLM runs as its
  own service.
