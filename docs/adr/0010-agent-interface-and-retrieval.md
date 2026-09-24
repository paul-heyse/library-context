---
id: ADR-0010
title: FastMCP interface with in-process hybrid retrieval and one Qwen3 embedding spec
status: superseded
date: 2026-09-22
supersedes: []
superseded-by: ADR-0025
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

## Amendments

- 2026-09-23: remaining-scope plan, Phase 0.
  - **BM25 is `bm25s` 0.3.11** (numpy backend, `get_scores`) over our own tokenization, with ties
    broken by `brief_id`, replacing the spike's hand-written scorer. The operator's uncommitted
    `scipy` line (added for this) is dropped with the Python restructure, by their decision.
  - **Layout.** The root `pyproject.toml` becomes a uv workspace of dev tools with members
    `python/lctx_mcp` (the server) and `eval` (gold, scoring, agent evaluation). vLLM is served
    from **a locked uv project** `services/vllm/` (`just embed-serve`), not `uvx`: `uvx` would
    leave torch and CUDA unlocked, outside the spec.
  - **The stdio start** is `mcp.run(transport="stdio", show_banner=False)` with
    `FASTMCP_CHECK_FOR_UPDATES=off`. FastMCP 4.0.5's banner makes an HTTP GET to PyPI and writes a
    cache file at every start (`utilities/version_check.py`), a network call the design never
    allowed. Nothing may print at import or in the lifespan, and one `StdioTransport` subprocess
    test guards it.
  - **Errors.** One domain exception from the shared hydration code, mapped to `ToolError` in
    tools and `ResourceError` in the resource. A `ToolError` raised inside a resource becomes an
    internal error. Embedder failures are caught inside `search_capabilities`, which answers
    lexical-only. Otherwise FastMCP turns a timeout into "please retry" even under masking.
  - **Not adopted:** `ResponseCachingMiddleware` (it caches every call for an hour and would keep
    serving a degraded result), `ResponseLimitingMiddleware` (it drops structured output),
    `fastmcp install`/`fastmcp.json` (unpinned).
- 2026-09-23, slice 1.8 (built):
  - The root is a uv workspace with one member, `python/lctx_mcp`. The `eval` member is added with
    its first code, the 3.3 scoring (deviation log D13). The root drops `vllm`, `scipy` and
    `fastmcp`: no script uses them.
  - `bm25s` 0.3.11 depends on numpy only.
  - The manifest carries `library` and `requirement`, so `search_capabilities` checks the library
    against the generation itself.
  - The server takes its query spec from package data. That data equals the committed specs, as
    a test checks. The fake twin is held to `specs/embedding/fake_vectors.json`.
  - `just py-fixture` builds the generation the tests serve from `analysis_shapes`.
- 2026-09-23, increment-1 deep review F2 (deviation log D19), pre-registered before any further
  gold-informed retrieval measurement:
  - **Fusion: a leg votes only where it discriminates.** The lexical leg scores only query words
    that occur in some briefs but not all, and abstains when there are none. A word every brief
    contains cannot tell them apart, and a ranking by it alone is BM25's length normalization.
    The vector leg always votes.
    - The rationale is independent of the gold, and a constructed generation tests it
      (`test_a_word_every_brief_contains_does_not_vote`).
    - One lexical-only run followed the policy, to test the check's exit code. It ranked the seed
      3rd and 1st, and nothing changed because of it.
  - **The retrieval evaluation (3.3, §12(b)):**
    - Each task alias of every gold family whose operations include a published brief's public
      path is searched with `limit = 5`.
    - Hit@1 and hit@5 are reported per family and overall, over the full brief set, with live
      vectors (`blocked` without the service). Lexical-only results are reported apart and
      labelled.
    - No retrieval parameter, fusion rule or brief-document template changes on the strength of
      these scores, except by an amendment giving a rationale independent of the gold.
  - **§1.5 for 15–25 briefs:** the primary seed's brief ranks first for every alias of its gold
    family. `just ranking-check <generation> vllm` exits 0 then, and 1 on any miss.
  - **D14 recorded:** the §11.1 brief document leaves out `analysis_boundary` text (slice 1.9,
    measured with live vectors on one query).
- 2026-09-24, the holistic assessment's A1 and the ADR-0020 review's F1 and F9 (plan Phase 2,
  step 1). **Pre-registered before any rescore and before the code that implements it.** The
  review found the direction this moves: under node identity, `+type-layer` improves §12(b) (16
  against 14 hits@5 on the old default's generation) and communities lose on all three metrics
  (Measured by the reviewer, 2026-09-23). The rationale below does not depend on the gold: a
  public name is a spelling of a declaration, and two spellings of one declaration are one
  object (`FastMCP.http_app is TransportMixin.http_app`).
  - **Identity is the declaration node.** A gold operation resolves by exact equality of its
    path with a served `public_paths` row (the one public-path relation: own and inherited
    spellings, exported classes included). There is no fuzzy or suffix fallback. A family's
    **node set** is its resolved operations' nodes.
  - **Classes.** An operation that resolves to a class matches only a brief whose seed is that
    class. Its methods and constructor are other nodes.
  - **Unresolved operations** are listed apart in two groups: those under the release's public
    root that resolve to nothing, and those outside it (another distribution's names). Both stay
    in the family's Jaccard union as strings, so a family is never scored as smaller than the
    gold states.
  - **Metrics.**
    - (a) is node Jaccard: per family, the best over briefs of
      `|F ∩ {seed}| / |F ∪ {seed}|`, where `F` is the node set plus the unresolved strings. A
      brief contributes its seed node, not its spellings.
    - (b) searches **every** task alias (44 in `fastmcp-4.0.5`) with `limit = 5`. A hit is a
      returned brief whose seed node is in the family's node set. A family with no such brief
      is searched and counts as misses.
    - (c) is unchanged: a gold span is recalled when a served evidence span of that file
      overlaps it.
    - **Units (F9):** all three are counted over all gold units (22 families, 44 aliases, 157
      spans), never over touched families, and a decision is reported only if it holds under that
      one convention.
  - **Tokenization.** A brief's lexical text holds each distinct name token once, however many
    spellings or splits produce it. BM25 term frequency then reflects the document, not how many
    aliases a name has. Rust and Python share known answers in `specs/serving/tokens.json`.
  - **Promotion.** Exact-symbol promotion matches any public spelling of a brief's seed
    (`symbol_map` from `public_paths`), so `fastmcp.FastMCP.http_app` promotes the
    `TransportMixin.http_app` brief.
  - **One matcher.** `scripts/score_gold.py` and `scripts/ranking_check.py` share one matching
    function. Every score JSON records `matcher_version` 2 (string equality of access paths was
    1), and each alias's mode. An alias that degrades to lexical-only under `--embedder vllm`
    makes the run `blocked`.
  - Nothing else about retrieval changes with this amendment. Fusion, the brief-document
    template and every parameter stay as registered in the entry above.
- 2026-09-24, the R2 review's F1 (`design_review_holistic-phase2-r2_2026-09-24.md`).
  **Pre-registered before any score.** The amendment above registered "each distinct name token
  once" and promotion by any spelling, but it did not fix **which** spellings a brief's lexical
  text names. `FORMAT` 2's first build used every spelling, own and inherited. That was
  unregistered, and it contradicted "Nothing else about retrieval changes".
  - **The names part of a brief's lexical text is its seed's own public spellings only**, the
    paths whose export declares the seed (`public_paths.own`), each distinct token once. Its
    inherited spellings still **promote** the brief (`symbol_map`), because an exact query names
    one node.
  - **The rationale does not depend on the gold.** The own set depends only on the node, never on
    which spelling named the seed (unlike `FORMAT` 1's container set). It matches the brief's own
    "Public access" text. It also keeps the number of subclasses that inherit a name out of
    document frequency. Under every spelling, a word shared by many subclasses (`proxy`) reaches
    all 20 pilot briefs and stops discriminating, and `Provider.disable`'s name tokens grow from 6
    to 62 (the review's measurement, disclosed here).
  - **Exposure, disclosed:** the author ran the scorers once on fake vectors over the
    every-spelling text (deviation log D52). That run's lexical leg was real BM25, so its (b)
    partly reflected the rejected set. The choice above rests on the rationale, not on that run.
  - Wording: "lexical text holds each distinct token once" means the **name** tokens. A brief's
    document text keeps its natural term frequency.
- 2026-09-24, ADR-0021 (the behavioral-model plan; decisions D-4 and D-6):
  - **Tools.** The server grows from two tools to the plan's five behavioral tools, alongside
    `search_capabilities` and `get_capability`:

    | Tool | Returns | Lands |
    |---|---|---|
    | `get_operation` | The whole record for one public operation | Stage 1 |
    | `find_operations` | Exhaustive matches over materialized rows, with `complete` and the operations whose answer is unknown; never uses vectors | Stage 1 |
    | `search_operations` | Ranked discovery, labelled as such | Stage 1 |
    | `lookup_concepts` | Candidate concepts with scope notes | Stage 4 |
    | `explain` | The derivation behind one claim | Stage 4 |

    All take typed pydantic input, return objects, and carry read-only annotations.
  - **Executor.** pyarrow compute over **materialized** rows. No SQL string is built at serve
    time, and nothing recurses at serve time: paths are precomputed as summaries and witnesses.
    Results have row caps, a `truncated` flag and cursors. The server still reads files only.
    Predicate semantics are never re-implemented in Python; the condition evaluator's Python twin
    is held to a shared known-answer corpus (ADR-0022).
  - **Bundle `FORMAT` 3** adds `operations` and `behaviors` (Stage 1), then conditions, concepts,
    members and vocabulary (later stages). It keeps every `FORMAT` 2 file. Briefs remain for the
    curated subset.
  - **Embedding views.** A spec is one model, one vector space. A **view** (signature and
    docstring, source body, and later others) is a template id and version inside the input's
    identity and a column of `vectors`. So `semantic:one-embedding-spec` holds per model, and two
    views of one operation have distinct keys.
- 2026-09-24 (operator): **the query embedder's HTTP client is `httpx2` 2.13.1**, pydantic's
  maintained continuation of `httpx`, with the same API under `import httpx2`. FastMCP 4.0.5
  already depends on it, so the server's lock drops `httpx` 0.28.1. The request bytes are
  unchanged (`content=`, never `json=`), and the conformance tests pass as before.
- 2026-09-24, the ADR set's standard review (F8, F10, F12):
  - **`find_operations`.**
    - `where` is a conjunction of facet equalities, `kind` and `path_prefix`, with no negation.
    - `complete` is true for declared facets. For a behavioral facet it is true only when no
      operation in the queried universe has an `unknown` row of that kind; those operations are
      returned, capped at 50.
    - The cursor binds the generation key, the request's hash and an offset.
  - **`explain`** returns the stored derivation only.
  - **The spec.** One spec covers briefs and operation views, and `search_operations` uses its one
    query instruction until a trigger justifies a per-tool one.
  - **No Python condition evaluator.** This corrects the amendment above. No tool takes a
    condition.
- 2026-09-24, the ADR set's compact re-review (R1, R4); this corrects the lines above.
  - **`complete`** is true only for facets read from the declaration and its annotations. `raises`
    and the behavioral facets are never complete in Stage 1. `unknown` lists the operations known
    to hide possible matches (status `unknown`: a depth cut or open call sites; or an `unknown` row
    of the facet's kind).
  - **A view is a column**, not part of the cache key. Identical texts share one vector.
- 2026-09-24, increment 3's deep review (F3, F4) and Stage 2.6 (ADR-0022):
  - **Bundle `FORMAT` 4** serves `operation_facet_status`; `complete` and `unknown` are read from it,
    not decided in Python (this replaces the "`complete` is true only for declared facets" line
    above). Only `established` and `conditional` facet rows match. The facet names are held to the
    codebook by `specs/serving/facets.json`.
  - **Bundle `FORMAT` 5** adds each behavior's `condition`, `callee_text`, `phase` and
    `premise_key`, and three files: `singletons`, `ambient_reads` (every read of a singleton's
    field, at its resolved key, with its phase and condition) and `place_claims` (field and setting
    claims with their premises). `get_operation` resolves a singleton's global name to its class and
    shows its fields.
