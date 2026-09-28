# MRL geometry observation and embedding boundary inspection

**Accepted input, 2026-09-27.** The operator has reviewed the official benchmarks and selected
1024 dimensions as the standard for retrieval and analytics. The review does not require
additional dimension-fidelity, task-quality or analytics-quality validation. The earlier
offline observations below are informational; they are neither an objection to that choice
nor an adoption gate. No further dimension probes are scheduled.

**Measured, 2026-09-27.** This bounded offline probe compares the existing live 4,096-dimensional
vectors with their first 1,024 components, renormalized with a Float64 L2 norm and division and
rounded to Float32. It does not run a model, use PostgreSQL, inspect gold or held-out questions,
or change any embedding specification. It is not a task-quality evaluation, an actual E0
output comparison, or a numerical oracle for vLLM's pooling implementation.

```bash
OPENBLAS_NUM_THREADS=4 uv run python \
  docs/design_review/evidence/2026-09-27_postgresql-expansion/embedding/mrl_probe.py \
  build/postgresql-live-offline/62c0ea68246bfea5 \
  --output docs/design_review/evidence/2026-09-27_postgresql-expansion/embedding/raw/observations.json
```

Outcome: **passed**. Source generation content digest:
`c90b558725e8894ff8f2551847ec5e3dfcad1ae061fb5c6d17835ae31f43562c`.
The source is the prior PostgreSQL qualification's live Qwen3-Embedding-8B generation.
[Raw observations](raw/observations.json) include source file digests and NumPy version.

| Corpus | Distinct vectors | Document pseudoqueries | Same top-1 neighbour | Retained top-3 neighbours | Retained top-10 neighbours | Pairs crossing cosine 0.5 |
|---|---:|---:|---:|---:|---:|---:|
| Operation views | 2,443 | 256 | 246 / 256 | 722 / 768 | 2,364 / 2,560 | 17,865 / 625,152 |
| Brief documents | 20 | 20 | 17 / 20 | 55 / 60 | 191 / 200 | 22 / 380 |

Vectors are deduplicated and ordered by input hash; operation pseudoqueries are evenly spaced
in that ordering. Self matches are excluded; exact score ties break by input hash. The probe
compares document vectors to document vectors rather than task-query embeddings. These are
neighbourhood-stability observations, not relevance judgements or evidence of a quality loss.
The operation prefix norms range from 0.46898 to 0.53059 before renormalization. Vector payload
shrinks fourfold; model weights and Transformer inference work are unchanged by output slicing.

## Interface evidence

**Interface-checked, 2026-09-27.** Context7 resolved `/qwenlm/qwen3-embedding` and supplied MRL
and vLLM examples. Its generated autodoc's blanket statement that vLLM output is unnormalized
does not describe this pinned launch: exact pinned source and the existing live receipts take
precedence for that claim.

- The [official model card](https://huggingface.co/Qwen/Qwen3-Embedding-8B) declares MRL and
  dimensions 32–4096. The operator's benchmark assessment is the accepted basis for the
  1024-dimensional standard; this review concerns its implementation and architectural effects.
- The [pinned model configuration](https://huggingface.co/Qwen/Qwen3-Embedding-8B/raw/1d8ad4ca9b3dd8059ad90a75d4983776a23d44af/config.json)
  has hidden size 4096 and no Matryoshka declaration consumed by vLLM.
- [vLLM 0.30.0 pooler source](https://github.com/vllm-project/vllm/blob/v0.30.0/vllm/model_executor/layers/pooler/seqwise/heads.py)
  slices the embedding prefix before its normalization activation. The installed pinned
  `pooling_params.py:163` rejects requested dimensions when `ModelConfig.is_matryoshka` is false;
  `config/model.py:1942` derives that property from `is_matryoshka` or `matryoshka_dimensions`
  in Hugging Face config. A spec-owned launch override and request dimensions are required for
  server-side MRL; live conformance remains **not_run** for a 1024-dimensional launch.
- [pgvector 0.8.6](https://github.com/pgvector/pgvector/blob/v0.8.6/README.md) supports PostgreSQL18,
  exact search, and HNSW/IVFFlat indexes on `vector` through 2000 dimensions. Storage supports
  more dimensions than these indexes. Filtered ANN can under-return; iterative scans stop at
  resource limits and do not establish exact recall. PostgreSQL extension installation and a
  digest-pinned disposable image remain implementation work, not outcomes of this probe.
- [pgvector-rust 0.4.2 manifest](https://github.com/pgvector/pgvector-rust/blob/v0.4.2/Cargo.toml)
  supports SQLx versions `>=0.8,<0.10`; SQLx0.9 can retain transaction ownership when adding
  the vector type adapter. No dependency or server extension was installed in this review.

## Interpretation boundary

**Proposed implementation of the accepted standard.** Use one 1024-dimensional production
spec for new retrieval and analytics outputs. Implement prefix truncation followed by L2
normalization consistently, encode the transformation in spec/cache identity, and regenerate
dependent vectors, findings and serving artifacts under that spec. Check request/response
contracts, dimensions, normalization, exact receipt replay and Rust/Python conformance as
implementation correctness. Do not add a dimension-quality acceptance gate or a temporary
search-only embedding standard. Existing immutable generations retain their original spec
and remain readable.

The code consumers to revisit are `cpg_core::embed::Spec`, both HTTP request builders,
`scripts/embed_serve.py`, fake/shared conformance corpora, dynamic serving schemas and their
known answers, cache/receipt validation, `lctx_analytics::neighbours` (exact top-k, centroids,
cosine floor0.5), `communities` (kNN layer), and the independently frozen retrieval policy.
Any pgvector ANN path belongs to ranked discovery. It must not replace exhaustive
`find_operations`, deterministic hydration, or the compiler's complete-under-model kNN
calculation without a separate changed contract.
