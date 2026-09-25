# Synthesis and serving

<!-- owner-intro -->

## §10 Synthesis and briefs

> Decision: ADR-0005, ADR-0019

**Proposed.** Source: IP L1695–L1731, L1886–L1966, L2580–L2637. Changed by ADR-0005: there is no
LLM interpreter. The increment-1 kinds, the kind policy, status derivation, the Outcome order and
the grounding rules below are **Implemented** and **Tested** (slice 1.5 and its review fixes,
2026-09-23; `cpg-core::synth`):
- `briefs_are_synthesized_from_findings_and_verbatim_evidence` snapshots each brief of
  `analysis_shapes`. It checks every spanned evidence text byte-for-byte against its source, some
  past a non-ASCII byte.
- `templates_say_what_the_findings_show` checks the call-site floor, depth-2 boundaries and the
  override-open hop.
- `an_outcome_from_the_docs_is_the_sentence_that_mentions_the_seed` covers Outcome leg 2 on a
  corpus, with passage evidence byte-checked.
- `synthesis_ids_ignore_the_runs_they_cite`, `synthesis_ids_follow_their_identity_columns` and
  the determinism test cover identity.
- Each rule rejects an injected violation in `the_analysis_rules_reject_their_violations`.
  The status rule, a floor and a ceiling, has three cases.


### §10.1 Findings

**A finding is a typed record,** carrying:
- `finding_kind`;
- its subject and related nodes;
- ordered witness steps (call site, callee, modality and phase; the `edge_id` as lineage) and
  cited facts (ADR-0019);
- conditions: source-linked text plus predicate node references;
- boundaries;
- method and parameters.

**It is never a sentence.** Text is produced only in §10.2.


### §10.2 Assertions

**Each assertion is atomic,** and carries:
- `assertion_kind`;
- the applicable case;
- supporting finding ids and evidence ids;
- conditions and limitations;
- `evidence_status`.

**Text** comes from a deterministic template per `assertion_kind`, filled from finding fields, or
from verbatim sentences selected from docstrings or docs.

**Evidence statuses:**

| Status | Meaning |
|---|---|
| `structurally_observed` | read directly from extracted facts |
| `documented` | stated in an official docstring, doc or example |
| `statistically_derived` | community, centrality, kNN; carries method, parameters and a stability score |
| `fixture_checked` | supported by an executed fixture for the stated inputs only |
| `unresolved` | a slot the evidence could not fill |

**Rule:** `statistically_derived` output may set titles, grouping, seeds, ordering, Related
entries and doc links. **It may never state a control, a limit or a behavioral claim.**

**Enforcement.**
- **Kind policy.** `cpg-schema` declares, for each `assertion_kind`, its brief section and its
  permitted statuses.
- **Status derivation** (`findings::derive_status`; slice 1.5 review F1). An assertion's status is
  a function of its supports alone, never chosen:
  - `unresolved` if it has no `support` row, or if any supporting finding is unresolved;
  - otherwise `statistically_derived` if any supporting finding, **or any finding that defined
    its scope**, is `statistically_derived`;
  - otherwise the strongest status among its supports, by `STATUS_STRENGTH`
    (structurally observed < documented < fixture checked).
  - A finding supports with its own status. An evidence row supports with its kind's
    (`EVIDENCE_STATUS`): a fact is structural; a docstring span, a passage or an example is
    documented; a fixture run is fixture checked.
- **Validator.** `semantic:assertion-policy` checks the kind policy.
  `semantic:assertion-status-derived` recomputes the derivation in SQL and requires equality: a
  floor and a ceiling.

**Increment-1 assertion kinds**

| Kind | Brief section | From | Permitted statuses |
|---|---|---|---|
| `outcome` | Outcome | docstring summary or explicit doc mention | documented, unresolved |
| `public_access` | Public access | Pass A `public_alias`, `exports` | structurally_observed |
| `coordinates` | Public access, "already coordinates" | Pass A `direct_delegation`, `bounded_delegation_path` | structurally_observed |
| `parameter` | Important controls | `parameters` (name, kind, default, required) and, from 2.1, `parameter_docs` (the description, its span cited); since A3, else a top-level `<ParamField>`'s lead (§10.3), citing its anchoring mention as `scope` | structurally_observed, documented (with parameter-doc evidence, or a `<ParamField>` lead under `semantic:documented-parameter-cites-its-doc`; deviation log D8) |
| `control` (2.1) | Important controls | Pass B `forwarding`, per seed parameter, with the path qualifiers | structurally_observed (documented from 3.4, with its rule; slice 2.1 review F7) |
| `transformed_control` (2.1) | Important controls | Pass B `transformed_argument` | structurally_observed |
| `restriction` (2.1) | Limits and prerequisites | Pass B `conditional_raise`, with the test's and raise's syntax facts and the path qualifiers | structurally_observed (documented only with precondition documentation, from 3.4 with its rule; review F7) |
| `unfollowed_control` (2.1 review) | Limits and prerequisites | Pass B `unfollowed_argument`, per seed parameter | structurally_observed |
| `usage_pattern` (2.2) | Usage pattern | `cpg_core::usage`: a verbatim statement subset of official usage code, its `example` spans, and each handoff it shows | documented, fixture_checked |
| `handoff` (2.2) | Usage pattern | Pass C `handoff`: the other callable, the formal and the occurrence count | structurally_observed |
| `analysis_boundary` | Limits and prerequisites | `boundaries` on the seed's neighbourhood | structurally_observed |
| `related` | Related | community co-membership, ordered by direct usage (PageRank in its variant) | statistically_derived |
| `applicable_case` | Applicable input or mode | reserved: no v1 source (the increment-2 review's U2) | structurally_observed |
| `implication` (2.5) | Important controls | an FCA `implication` of the seed's own scope whose premise it meets | structurally_observed |
| `doc_link` (3.1) | Related | kNN `doc_link` findings | statistically_derived |
| `shared_signature` (increment-2 review) | Related | the FCA concept of the seed's own scope with the most shared pairs | structurally_observed |
| `documented_warning` (3.4; A3) | Limits and prerequisites | a `<Warning>` component of a passage that mentions the seed exactly, its inner bytes cited, and its anchoring mention cited as `scope` (§10.3; `semantic:documented-warning-anchored`) | documented |


### §10.3 Brief structure and the Outcome order

**One brief = one outcome, anchored to one public operation**, optionally with one direct handoff.

| Section | Filled from |
|---|---|
| Outcome | the order below |
| Public access | Pass A `public_alias` / exports, plus "already coordinates" from Pass A delegation findings (with the note that these are implementation details the caller does not need to rebuild) |
| Applicable input or mode | an input or mode source: none in v1, so the slot is absent (the increment-2 review's U2; FCA concepts are shared signatures under Related) |
| Important controls | Pass B forwarding + parameter docs |
| Usage pattern | Pass C handoff or official example, with setup preserved |
| Limits and prerequisites | Pass B restrictions, documented warnings, boundaries |
| Evidence | all cited findings and evidence ids |
| Related | other public APIs of the seed's community, at most five, the most called in official usage first (`statistically_derived`; slice 2.6, the increment-2 review's U1); the seed's shared signature (`structurally_observed`); doc links (`statistically_derived`) |

**Outcome order.** Take the first source that applies:
1. the entry point's docstring summary: the first sentence of the docstring's first paragraph;
2. the lead sentence of a doc paragraph that **explicitly** mentions the entry point, when the
   mention lies inside that sentence;
3. otherwise `unresolved`.

Sentences are found with UAX #29 on a view in which each line break is a space, so a hard-wrapped
sentence is one. The evidence is the source bytes; the assertion reads the sentence with its
whitespace collapsed. Change-log documents (`synth::CHANGELOG_STEMS`) never give an Outcome
(slice 1.5 review F2, O1; deviation log D7).

The nearest doc passage by embedding is never an Outcome. It is published as a doc link
(`statistically_derived`), which keeps the gap metric honest.

**The count of `unresolved` slots, by section, is the §B11 gap metric.** Read from a published
snapshot with `lctx query`:
`SELECT p.brief_section, count(*) FROM assertions a JOIN assertion_policy p ON
p.assertion_kind = a.assertion_kind AND p.evidence_status = a.evidence_status WHERE
a.evidence_status = 4 GROUP BY p.brief_section ORDER BY 1`.

**How increment 1 fills the sections** (slice 1.5, `cpg-core::synth`):
- **Outcome:** the seed's docstring summary.
  - It is located in the literal's own source bytes. Pyrefly's `Docstring::clean` renders
    Markdown and loses spans.
  - Its paragraph ends at a blank line, a section header or the closing quote.
  - Else the first exact mention, by document path, passage ordinal and position, whose
    paragraph's lead sentence holds it. The mention is of the seed's declaration, or of an export
    whose target is the seed: a class's export does not name its method.
  - Else `unresolved`. The evidence is the byte span.
- **Public access:** the call form the seed's declaration gives:
  - a function is called, and a class constructed;
  - a method, class method, static method or property is named with its class and how to use it.
  - Then its configured access path, and every other public access path naming it (Pass A's
    `public_alias`).
- **Already coordinates:** one assertion per delegation finding, total over its fields:
  - at depth 1: a direct call with its number of call sites ("or more" when the witness cap
    omitted some), a definition, a property read or set, or an override-open call;
  - deeper: the intermediate callables, with each hop that is a definition, a property access or
    an override-open call named at the step where it occurs.
- **Important controls:** one `parameter` assertion per parameter of the seed's own signature (the
  receiver aside): kind, default, requiredness and annotation.
  - Its evidence is the syntax fact and span, plus Pysa's semantics fact for requiredness.
  - Where Pysa has no semantics row (an overloaded seed), it says "requiredness not observed".
  - Its description, when the documentation gives one (`documented`): the docstring's
    (`parameter_docs`, citing its span). Otherwise (A3; Implemented and Tested, 2026-09-24) the
    lead paragraph of a **top-level** `<ParamField>` whose literal `body` is the parameter's
    name, in a passage that mentions the seed exactly (changelogs are never read), citing that
    paragraph's bytes as Passage evidence and the anchoring mention as `scope` (R1 F1). *Top-level*
    means **no `<ParamField>` ancestor**: a `<Card>` or `<Expandable>` around the field does not
    count (R1 F3; the schema's depth 0 is another thing). If several such fields disagree after
    normalization, none is used. A `<ParamField>` nested in another describes a field of its
    parent's value, never a parameter. Fields named by `path`, `query` or `header` are not read
    (R1-3).
- **Limits:** one `analysis_boundary` assertion per stop reason (dependencies, synthesized
  callables, release code outside the subsystem, unresolved sites). What the seed calls itself
  is kept apart from what the callables it reaches call. Plus the depth bound or a budget
  truncation, citing Pass A's `traversal_stop` finding.
- **Documented warnings** (Limits; slice 3.4, rebuilt on components by A3, Implemented and
  Tested 2026-09-24): one `documented_warning` assertion, `documented`, per `<Warning>` component
  of a passage that mentions the seed exactly (changelogs are never read). Its text is the
  warning's inner bytes, verbatim, with where it is ("in `docs/x.mdx` § Heading (which mentions
  `seed`)") and its literal `title` when it has one, citing those bytes as Passage evidence.
  Inside a `<ParamField>`, the warning is about the **nearest** such field's parameter and says so
  ("about the parameter `p`"); if `p` is not a parameter of the seed other than its receiver, the
  warning is skipped. A `<Warning>` in fenced or inline code is not a component, so it is never
  read. The anchoring mention is cited as `scope`, and `semantic:documented-warning-anchored`
  checks the quote, the anchor and the scoping (R1 F1). Warnings enter the brief document as the
  capability's own limits.
- **Tested** (R1 F2, 2026-09-24; `docs_shapes`, `a_documented_warning_is_a_limit`,
  `documentation_statements_are_anchored_to_their_seed`): a warning under a field that is no
  parameter is skipped; the nearest field decides (a warning under `backoff`, itself inside the
  parameter `retries`, is skipped); a field nested under `ParamField > Expandable` describes
  nothing; a field inside a `<Card>` describes its parameter; a warning in a passage that mentions
  no seed is not stated; each rule rejects a doctored anchor, quote, scope or nesting.
- **The components are Mintlify's** (`cpg_schema::mdx`, read by Stage F and the rules; R1 F5).
  A library documented another way has none, so a zero there means no such source.
- **Measured on the pilot** (2026-09-24, snapshot `8c76bc75…`, 20 briefs): the corpus has 1,250
  components (450 nested, depth at most 3; 1,605 attributes: 1,504 literal, 70 expression, 31
  bare), including 93 `<Warning>`s and 173 `<ParamField>`s. **None reaches a brief:** no documented
  warning is stated and no parameter is described from a `<ParamField>` (60 of 97 are described
  by their docstrings). Only one passage holding a warning has any exact mention. The passages
  that document the seeds' arguments (`docs/servers/tools.mdx` § Decorator Arguments, and the like
  for `resource` and `prompt`) name the operation as `@mcp.tool`, a receiver-variable spelling
  that is no exact mention. A code-block call anchor is **a probe, not a measurement** (R1 F4): the
  author's count was 13 warnings in 7 briefs, the reviewer's reconstruction 14 in 9, with no
  committed query; both reach no `<ParamField>` (that block's `mcp` is unbound), and the anchor is
  noisy (`TransportMixin.run` anchors 44 passages through `mcp.run()`). **The exact anchor stays**
  (R1 §10; deviation log D45). One widening candidate, a receiver-resolved mention, is registered
  as a content variant in the ADR-0020 re-registration and decided by the structured evaluation;
  the heading/title and code-block anchors are rejected, each with a reopening trigger.
- **The brief document** (§11.1): outcome, applicable case (when one exists: none in v1),
  public APIs, control names, usage and limits. Over 2,048 tokens (a declared proxy of 4 bytes per
  token) it is split into chunks of whole parts, each under the header (the outcome and any
  applicable case), never truncated; a part too long for any chunk fails the compile (slice 2.5,
  `chunked`). Retrieval scores a brief by its best chunk.
- A template or extractive-rule change bumps `synth::TEMPLATE_VERSION`; the analysis output is
  pinned to the versions in a test ledger (ADR-0019 review O4).


### §10.4 Grounding checks

These are mechanical and run before publication.
- Every named public symbol and parameter exists in this snapshot.
- Every cited finding, witness path and evidence id exists in this snapshot.
- Every snippet parses, and every name it loads is bound in it, imported by it or a builtin (§10.5).
  Whether each imported API exists is the release's to say: a pattern is verbatim official code
  (slice 2.2 review F1(d), narrowed).
- No assertion's status exceeds its evidence (see the §10.2 rule).
- Warnings and unresolved conditions are never dropped for length. An over-long brief's document
  is chunked at whole parts instead (§10.3); the brief itself is never split (the increment-2
  review's F8).

**Manual review.** From increment 3, each brief gets one manual review pass before
publication, which checks that its extracted sentences are true of **this** entry point. The
review is recorded as `briefs.review_state` with extraction mode `manual_review`. Mechanical
checks cannot establish that (IP L1965, L2634).

Repository text is treated as untrusted data. It is never an instruction to the compiler.


### §10.5 Usage patterns

- **Contents.** One principal operation, or one direct handoff, plus the setup it needs
  (initialization, schema, resources, configuration).
- **Trimming.** Test-only details are removed only when that removes no precondition.
- **Publication.** A pattern is published only with an official example, a relevant test, or an
  executed fixture.
- **Fixtures** run offline, with no network or credentials. A pass supports only the tested case.

**Implemented** and **Tested** in slice 2.2, revised by its compact review (2026-09-23;
`cpg_core::usage`; deviation log D24, D30):
- **Selection.** Candidates are the seed's call and decorator sites in official examples, then doc
  blocks, then tests; within a role a handoff's sites are tried first. The pattern that shows a
  whole handoff occurrence (producer and consumer site) wins within its role, then the smallest.
- **The pattern.** It is the statement holding the site, in a module or function body only (never
  under a `with`, `if`, loop or `try` header it would drop, such as `with pytest.raises`), plus,
  transitively, the same-block statements before it that bind a name it reads, and the imports
  binding one. A candidate that reads a name bound anywhere else, or bound nowhere (neither a
  binding nor a builtin: a continuation doc block's, C5 O2), is not self-contained and is refused.
- **Publication.** Each statement is dedented by its own indentation. The code must parse (Ruff),
  and every name it loads, annotations included, must be bound in it, imported by it or a builtin
  of the analyzed Python (`ruff_python_stdlib`); otherwise the candidate is refused (review F1).
  `lctx_mcp.smoke` parses every served pattern again.
- **Assertion.** The pattern is a `usage_pattern` assertion (`documented`) citing each
  statement's `example` span verbatim, and each handoff it shows whole (review F3;
  `semantic:usage-pattern-shows-its-handoff`). Its code enters the brief document as §11.1's
  usage description.

> Decision: ADR-0005, ADR-0019

---

## §11 Serving and agent interface

> Decision: ADR-0013

**Tested** where a line cites spike E1–E3 (`spike/pyrefly-inproc`, 2026-09-22). Otherwise
**Interface-checked** (fastmcp skill and installed FastMCP; vLLM 0.30.0 source; model card).
Source: IP L2037–L2071, L2640–L2965.


### §11.1 Embedding spec and vectors

**Model.** Qwen3-Embedding-8B at a pinned revision (`docs/pins.md`), served by a **separate vLLM
0.30.0 service** (`vllm serve … --runner pooling --max-model-len 8192`).
- **Output:** 4,096 dimensions, `Float32`, cosine.
- **Normalization (Tested, E1).** vLLM L2-normalizes: every norm was 1 ± 1e-7. The pooling
  (`LAST`, with activation) comes from the model's sentence-transformers config.
- **Never send `dimensions`.** We use the full 4,096, and vLLM rejects the parameter without a
  Matryoshka override.
- **Measured (E1):** ~15.5 GiB of weights, ~27 GB of the 5090 at 0.80 utilization, 85 s to start.

**Spec.** The spec is hashed into `spec_hash`. It covers the model and revision, tokenizer
revision, vLLM version and served dtype (bfloat16), pooling, instruction template, document
template, dimensions, output dtype and normalization.
- **Query template (query only):** `Instruct: {task_description}\nQuery:{query}`. There is no
  space after `Query:`. Documents take no prefix.
- **Document text** is the deterministic brief projection (IP L2666–L2689): outcome, applicable
  case (when one exists), public APIs, controls, usage description and limits, capped at 2,048
  tokens. From ADR-0021 it is also each public callable's **views** (`operation_documents`):
  signature and docstring, and source body. Views are cut into windows of at most 4,096 bytes at
  line ends, like E0's windows (§9.7), so they stay under the cap without a token count.
  - **One spec for both** (the ADR review's F10). Documents take no prefix, so a spec hash
    identifies one vector space for briefs and views alike.
  - **One query instruction for now.** `search_operations` embeds its query with the spec's one
    instruction, which names capability briefs. A per-tool instruction would change `spec_hash`,
    and so every cached key. It waits for its trigger: the structured evaluation attributes
    operation-search misses to the wording. An over-long document is chunked at whole parts under its header, never truncated
  (§10.3; the increment-2 review's F8).
  - The limits are the capability's own: Limits-section kinds other than `analysis_boundary`.
  - What the analysis did not follow (dependency and synthetic boundaries, unresolved sites, the
    depth bound) stays in the served brief, out of retrieval. Slice 1.9 measured this with live
    vectors (deviation log D14).
- **Rejected responses:** wrong count, wrong index mapping, wrong length, non-finite values,
  norm ≠ 1 ± ε, model mismatch.

**Clients.**
- Compile-time vectors come from Rust (`reqwest` + `tokio`).
- Query-time vectors come from Python (`httpx2`, pydantic's continuation of `httpx`; 2026-09-24).
- **Conformance (Tested, E2).** Over the fixed conformance inputs, both clients build
  byte-identical request texts, apply the same rejections, and return vectors that agree to cosine
  ≥ 0.9995. vLLM is not bitwise deterministic across requests (identical inputs differed by up to
  3.8e-3 in a component), so an exact vector match is never expected.

**Implemented** and **Tested** in slice 1.6 (2026-09-23):
- **The spec** is committed canonical JSON, `specs/embedding/qwen3-embedding-8b.json`, whose
  SHA-256 is the spec hash (`cpg_core::embed::Spec`; `the_committed_spec_is_its_canonical_form`).
- **The Rust client** (`lctx-embed`) builds its request bodies with `serde_json` and is held to
  `specs/embedding/request_bodies.json` for the shared conformance inputs, which the Python
  client is held to as well; every rejection fires (`every_rejection_fires`). Both clients judge
  a response alike, Rust by its serde types and Python by pydantic strict models (the holistic
  assessment's A7: a JSON boolean is never an index or a component), held to one corpus of 25
  bodies, `specs/embedding/responses.json` (`responses_are_judged_as_the_shared_corpus_says` in
  both suites); a stub service
  exercises the real HTTP path; an unreachable service is `blocked`, never fake. It uses the
  `reqwest` 0.12.28 already in the lock, without TLS (the service is local).
- **Token counts** come from the service's `/tokenize`; an over-cap document fails the compile.
  Since the holistic assessment's D3 (2026-09-24), only keys the cache lacks are counted (a cached
  key passed the cap when it was embedded), so a fully cached `--embedder vllm` compile needs no
  service; batches completed before a failing one are merged, so a rerun embeds only what is
  missing; the client has a 10 s connect and a 300 s request timeout; and the request bodies are
  typed structs, so their key order does not depend on `serde_json`'s features (the bytes are
  unchanged, held to `request_bodies.json`). Tested: `only_documents_the_cache_lacks_are_counted_and_embedded`,
  `completed_batches_survive_a_later_failure`.
- **The fake embedder** (`FakeEmbedder`, its own spec) draws a unit vector by splitmix64 from the
  request text's SHA-256, so Python reproduces it with its standard library.
- **The cache** is written by an insert-only MERGE (`delta::merge_global`), probed at the pinned
  delta-rs: only Add actions on an append-only table, only missing keys inserted, CHECKs enforced,
  and four racing merges of one key leave one row
  (`an_insert_only_merge_adds_only_missing_keys_to_an_append_only_table`,
  `a_merge_enforces_the_immutable_checks`, `concurrent_merges_of_one_key_leave_one_row`).
- **`lctx compile --embedder vllm|fake|none`**; the service is `just embed-serve`, from the
  locked `services/vllm` project.

**Cache.** Vectors are keyed by `spec_hash + input_hash` in the canonical `embedding_cache` Delta
table (§3.2), because vLLM numerics vary between requests (E2). Snapshots record the cache version they
read, and bundles copy the vectors they need from it.
- A **deterministic fake embedder**, with its own spec hash, is used by tests and `just check`.
- Mixing spec hashes within one generation is rejected.


### §11.2 Retrieval

**In-process, over the pinned generation.**

1. **Lexical.** BM25 over `lexical_text`, scored by `bm25s` 0.3.11 (numpy backend,
   `get_scores`) over our own tokenization (ADR-0010 amendment). That text is the brief's text
   plus the distinct name tokens of its seed's own public spellings, each once
   (`write_dataset` → `write dataset`; `FORMAT` 2, ADR-0010's amendments of 2026-09-24),
   computed in Rust when the bundle is built.
2. **Vector.** Exact cosine over the generation's vectors, using the instruction-prefixed query
   vector.
3. **Fusion.** Reciprocal-rank fusion with K = 60, 1-based ranks, and ties broken by `brief_id`.
4. **Exact symbols.** Matches in `symbol_map` (every public spelling of a brief's seed, own and
   inherited, in `FORMAT` 2) are merged by brief id and **recorded as promoted**.
5. **Return.** At most `limit` results (default 5), with relevance and coverage information.
   **A score is never presented as proof of task fit.**
6. **Degraded mode.** Lexical + exact-symbol only when the embedding service is down, reported in
   the result.

**Hydration.** `(snapshot_id, brief_id)` → the full brief, all conditions, limits, evidence and
usage patterns, by deterministic lookup. It never depends on a second search.

**Implemented** and **Tested** in slice 1.8 (2026-09-23; `lctx_mcp.retrieval`):
- the tokenizer is lower-cased runs of letters and digits;
- bm25s (`method="lucene"`, k1 1.5, b 0.75) equals a hand computation of the Lucene formula
  (`test_bm25_scores_are_the_lucene_formula`); unknown words count for nothing;
- a brief's vector score is its best chunk's cosine;
- RRF ties go to the lower brief id (`test_fusion_ranks_ties_by_id_and_promotes_exact_symbols`);
- a promoted brief ranks first;
- a down embedder gives `lexical-only` with its reason (`test_a_down_embedder_degrades_to_lexical_and_says_so`).
- Hybrid ranking with **real** vectors is 1.9's check. The fixture's fake vectors carry no
  meaning.

**LanceDB.** LanceDB 0.39.0 is **deferred behind §13's trigger**: more than ~10⁵ vectors at
4,096 dimensions, or filtered ANN together with managed FTS. When it lands, it goes in an isolated
workspace (the ADR-0002 amendment).
- Its hybrid, FTS and RRF call chain is Interface-checked.
- Its wheel isolates its own Arrow 58 / DataFusion 54, so it would not affect §B9.


### §11.3 FastMCP contract

- **Package.** `python/lctx_mcp`. It depends on `fastmcp` 4.0.x, `pyarrow` 25.0.1 (the bundle
  reader), `numpy` and `httpx2` (the continuation of `httpx`, which FastMCP 4.0.5 already uses;
  2026-09-24), never on vLLM.
- **Startup checks.** The lifespan rejects a generation whose per-file schema digests differ from
  the canonical schemas it expects, or whose `embedding_spec` hash differs from its query
  client's spec.

**Tools**

| Tool | Parameters | Output |
|---|---|---|
| `search_capabilities` | library, query (1–4000 chars), limit (1–10) | Pydantic `SearchResult` object: snapshot, generation key, mode (hybrid or lexical-only), coverage summary, hits (brief id, title, outcome, outcome `evidence_status`, relevance score, rank source, promoted flag). An unknown library raises `ToolError` |
| `get_capability` | snapshot_id, capability_id | Pydantic `Capability` object: all §10.3 sections, evidence and statuses |

- **Annotations.** Both tools carry `ToolAnnotations(read_only_hint=True, idempotent_hint=True,
  open_world_hint=False)`.
- **Output shape.** Always object outputs, never bare lists.
- **Resource.** A `capability://{snapshot_id}/{capability_id}` template shares the hydration code.
  Resource reads return MIME text, not structured content.
- **State.** A lifespan loads the active generation once and exposes it through
  `ctx.lifespan_context`. One generation per process.
- **Errors.** `ToolError` for unknown ids or a snapshot mismatch; `mask_error_details=True`.
- **Transport.** stdio, started with `mcp.run(transport="stdio", show_banner=False)` and
  `FASTMCP_CHECK_FOR_UPDATES=off`: FastMCP 4.0.5's banner otherwise makes an HTTP GET to PyPI at
  every start. Nothing may write to stdout, at import or in the lifespan either; a
  `StdioTransport` subprocess test checks it (ADR-0010 amendment).
- **Tests.** `fastmcp.Client(mcp)` in both the auto and legacy protocol modes, asserting
  `.structured_content`, plus generations with a mismatched schema or spec, which must fail at
  connect.
  **Tested** (E3): `auto` negotiated `2026-07-28` and `legacy` `2025-11-25`; both mismatch
  fixtures failed at connect.
- **Implemented** and **Tested** in slice 1.8 (2026-09-23; `python/lctx_mcp`, 19 tests over the
  `just py-fixture` generation):
  - The round trip in both eras (`test_the_tools_round_trip_in_both_protocol_eras`).
  - Promotion, and degraded mode.
  - A tool error for an unknown library, snapshot or id, and for out-of-bounds arguments.
  - The resource as Markdown. An unknown id is invalid params, −32602, because `CapabilityError`
    is FastMCP's `ValidationError` (the holistic assessment's A7; it was `ResourceError`, which
    reached the wire as an internal error, −32603).
  - A schema digest, spec or key mismatch fails at load and at connect
    (`test_a_mismatched_generation_fails_at_connect`).
  - Byte-identical request bodies (`test_request_bodies_are_byte_identical_to_rusts`), and the
    fake twin (`test_the_fake_twin_reproduces_rusts_vectors`).
  - The serving digests' known answers.
  - A `StdioTransport` subprocess that writes nothing but the protocol
    (`test_the_server_speaks_only_the_protocol_on_stdout`).
  - Started as `python -m lctx_mcp --generation DIR --embedder vllm|fake|none`.

**The behavioral tools** (ADR-0010 amendment, 2026-09-24; by plan stage; §1.1). The three Stage 1
tools are **Implemented** and **Tested** (2026-09-24, `python/lctx_mcp/tests/test_operations.py`,
9 tests over the fixture generation, both protocol eras in `test_server.py`); the Stage 4 tools
are **Proposed**.

| Tool | Parameters | Output |
|---|---|---|
| `get_operation` (Stage 1; conditions from Stage 2) | snapshot_id, operation (a public path, or a module-global singleton's name, which resolves to its class) | `Operation`: paths, signature, docstring summary, facets and which are incomplete, each parameter's fates (forwards, derives, stores, returns, raises, tests, is-read claims) with verdicts, conditions and lines, delegations, handoffs, settings read with their phase, a class's constructor record, a singleton's fields (their reads and never-read claims), boundaries, the brief id if any. An unknown path is invalid params |
| `find_operations` (Stage 1; semantic filters Stage 3) | library, `where` (typed facet, effect, role and condition-compatibility filters; concepts from Stage 4), limit (1–50), cursor | `OperationSet`: matches, `complete`, the operations that could still match (rows `unknown` or `not_analyzed`, per `operation_facet_status` and query boundaries), `truncated`, next cursor. Never vectors |
| `search_operations` (Stage 1) | library, query, filters, limit | `OperationHits`: ranked, per-view RRF, labelled `ranked_discovery` |
| `lookup_concepts` (Stage 4) | text, limit | Candidate concepts with labels and scope notes |
| `explain` (Stage 4) | snapshot_id, claim id | The rule id, premises and source spans |

All return objects with read-only annotations, and follow the error contract above. Direct
lookup/retrieval still uses materialized rows. **Proposed Stage 3:** a pinned in-process Rust
extension executes bounded semantic queries over the same immutable generation (§B13,
ADR-0025). It returns cited row/node ids, budgets, unknown boundaries and truncation; Python
does not reinterpret condition truth. The extension's ABI and generation format are checked at
startup.

**Semantics** (the ADR review's F8):

**`where`** is a conjunction of terms. Current Stage 1 terms are:
- `{facet, value}`: exact equality on an `operation_facets` row;
- `{kind}`: function, method or class;
- `{path_prefix}`.

**Proposed Stage 3 terms (ADR-0025):** `{effect}`, `{role}`, and
`{compatible_with: <typed condition>}`. The condition grammar is closed and canonicalized by
Rust. Compatibility and implication use ADR-0024's node budget and stable-place theory; an
undecided condition leaves its operation in `unknown`. Rows and witnesses come only from the
pinned generation. A condition term is anchored to an operation's entry formal. The Rust
flow/summary producer writes `flow_test_value_links`: operation/formal id, leaf evaluation
atom identity, `flow_test_leaves.fact_id`/`flow_uses.use_id` and operand span,
resolved place/path, proof origin, cited flow/summary facts and effect-model digest. It first
certifies direct paths without calls or writes; a later modeled transfer must be
identity-preserving, not merely pure. The shared validator rejects ambiguous binding,
alias uncertainty, unmodeled effects, mixed snapshots and digest drift. Missing links are
`unknown`, not source-atom matches by spelling. Compatibility means may-model non-refutation,
never proof of feasible execution. Candidates partition into matched, proven-excluded,
source-open and unexamined. Unvisited/undecided operations make `complete = false`; an early
budget stop returns an unexamined count and a resumable generation/query-bound cursor. Row,
depth and pair-work budgets have the same unknown/truncated behavior.

**Implemented and Tested (2026-09-25, producer and internal native boundary only).** The direct
`flow_test_value_links` origin and its publication validator exist. FORMAT 7 carries the checked
value-link and test-leaf projections, with the exact effect-rule digest in its manifest. The
native index admits the links only when their leaf atom, source module, condition, place, public
operation/formal and digest agree. It invokes the shared primitive-theory kernel to assess one
cited summary path for an exact query input. A checked assignment that leaves the BDD
satisfiable is `compatible_under_model`; a missing applicable link is `unknown`. An
unconditionally true path is compatible without a link. These are path-local may-model
outcomes, never concrete executions or operation-wide verdicts.

**Implemented and Tested (2026-09-25, partial FORMAT 7).** Structural analysis conditions and
finite value-summary proofs are generation-pinned and load through one native index. The internal
value-path lookup resolves an actual public operation and formal and reports only cited positive
paths plus open boundaries. A bounded, cursor-paged `inspect_value_paths` MCP tool now exposes
path-local exact-input assessment and cited link operand spans; the cursor binds generation and
query. It does not aggregate an operation-wide compatibility verdict or offer the planned
cross-operation compatibility/effect/role filters. Proof steps still carry evidence ids rather
than source text and full step spans.

**Implemented and Tested (2026-09-25, targeted).** The path-local exact-input kernel now returns
work evidence even on a budget refusal: checked link rows, completed assignments, the sum of
BDD input-node-product preflights and the peak result-node count. The native response carries
these counters per path, and the MCP page sums the first three and takes the peak over displayed
paths. The pair figure is a conservative preflight bound, not a count of internal BDD visits;
`examined_rows` separately counts paged summary/boundary rows. Assignment-cap refusal remains
`unknown` with `budget_reached`. Page totals cover this page only, never unexamined cursor rows.

There is **no negation**: a NOT would read absent facts as false, and Stage 1 makes no negative
claims. An unknown facet is invalid params. An unknown **value** is invalid params only where
every operation's rows for that facet are complete, and the error names close values; elsewhere
absence is not known, and the answer is empty with `complete = false`.

**Completeness is served data** (increment 3's deep review, F3 and F4), not a class Python decides:
- `operation_facet_status` holds, per operation and facet, `established` when its rows are
  complete, or the verdict and why they are not:
  - a class's `parameter` and `parameter_type` are its public constructor's (its own or inherited
    `__init__` path), and without one they are `not_analyzed`;
  - `returns` does not apply to a class;
  - `raises` is never complete (only a typed `raise` directly in the body is a row);
  - `forwards_to` and `delegates_to` take the operation's `behavior_status`;
  - `hands_off_to` and `takes_from` are never complete (two usage shapes only).
- `operation_facets` rows carry a verdict. Only `established` and `conditional` rows **match**. An
  `unknown` row (a path through an override-open call) leaves its operation open.
- **An operation hides a possible match** when, for every term, it matches or is open (its rows
  are incomplete, or its row is `unknown`), and it is not a match. `complete` is true exactly when
  no operation in the universe hides one.
- `unknown` lists the hiding operations, capped at 50, with a count and a flag. It is evidence, not
  a bound.

**`complete` is never "not truncated".** Truncation has its own flag.

**Cursor and snapshot.** The cursor is opaque and binds the generation key, the request's canonical
hash and an offset. A cursor from another generation or request is invalid params. Every result
names its `snapshot_id`.

**`explain`** currently returns the stored derivation. The proposed Rust executor may traverse
bounded stored witness links at request time, retaining their row ids and reporting a boundary
if the depth budget is reached (§1.3, ADR-0025).

> Decision: ADR-0025 (superseding ADR-0010), ADR-0013

---
