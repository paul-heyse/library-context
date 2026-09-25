# Synthesis and serving

This owner covers how typed findings become cited assertions and briefs (Stage F, §10), how
texts become vectors under one hashed embedding spec, and how one pinned serving generation is
exposed to coding agents through the FastMCP server (§11). Its inputs are the analytics findings
([§9](analytics.md#section-9)), behavioral relations ([§9.9](behavioral-analysis.md#section-9-9)),
extracted evidence spans and the published generation
([§6.4](storage-and-publication.md#section-6-4)); its outputs are the `assertions`/`briefs`
analysis tables, the `embedding_cache` rows and the tool responses agents read. Dependencies run
one way: synthesis reads findings and evidence, never the reverse; the server reads only an
immutable generation and never Delta, DataFusion or the compiler. Authoritative declarations
live in `crates/cpg-schema/src/findings.rs` (kinds, statuses, `ASSERTION_POLICY`) and
`bundle.rs` (the generation format); Stage F is `crates/cpg-core/src/synth.rs` and `usage.rs`;
embedding is `crates/cpg-core/src/embed.rs`, `crates/lctx-embed` and
`specs/embedding/`; serving is `python/lctx_mcp` (tests under `python/lctx_mcp/tests/`) with the
native executor in `python/lctx_semantics`. Rationale: ADR-0005 (programmatic synthesis),
ADR-0043 (interface, retrieval, embeddings) and proposed ADR-0025 (native executor). See the
[architecture map](../README.md).

## §10 Synthesis and briefs

**Label.** Programmatic synthesis is accepted (ADR-0005): there is no generative model in the
pipeline or the query path. The assertion kinds, the kind policy, status derivation, the Outcome
order and the grounding rules below are **Implemented** and **Tested** (2026-09-23/24;
`cpg-core::synth`):
- `briefs_are_synthesized_from_findings_and_verbatim_evidence` snapshots each brief of
  `analysis_shapes` and checks every spanned evidence text byte-for-byte against its source, some
  past a non-ASCII byte;
- `templates_say_what_the_findings_show` checks the call-site floor, depth-2 boundaries and the
  override-open hop;
- `an_outcome_from_the_docs_is_the_sentence_that_mentions_the_seed` covers Outcome leg 2 on a
  corpus, with passage evidence byte-checked;
- `synthesis_ids_ignore_the_runs_they_cite`, `synthesis_ids_follow_their_identity_columns` and
  the determinism test cover identity;
- each rule rejects an injected violation in `the_analysis_rules_reject_their_violations`; the
  status rule, a floor and a ceiling, has three cases.

Briefs are one rendering of the analysis. Behavioral claims served through the operation tools
(§11.3) come from the behavior relations directly, not from briefs.

> Decision: ADR-0005, ADR-0047


### §10.1 Findings

**A finding is a typed record** (`cpg_schema::findings`; ADR-0047), carrying:
- `finding_kind`;
- its subject and related nodes;
- ordered witness steps (call site, callee, modality and phase; the `edge_id` as lineage) and
  cited facts;
- conditions: source-linked text plus predicate node references;
- boundaries;
- the invocation that produced it (method and parameters).

**It is never a sentence.** Text is produced only in §10.2.


### §10.2 Assertions

**Each assertion is atomic,** and carries:
- `assertion_kind`;
- the applicable case;
- supporting finding ids and evidence ids (`assertion_support`, role `support` or `scope`);
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
- **Kind policy.** `cpg_schema::findings::ASSERTION_POLICY` declares, for each `assertion_kind`,
  its brief section and its permitted statuses; it is authoritative over the table below.
- **Status derivation** (`findings::derive_status`). An assertion's status is a function of its
  supports alone, never chosen:
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
- **Support closure.** Every assertion is meant to be traceable from the served form to its
  finding, witnesses, invocation/model and source spans. In canonical Delta that chain exists.
  **Known gap:** an assertion supported only by a finding (e.g. `public_access`, `coordinates`)
  keeps the finding id in the generation but not the finding's witness/invocation closure, so the
  server cannot follow it to source, and the Markdown resource drops even the finding id
  (§11.3; [plan W3](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**Assertion kinds**

| Kind | Brief section | From | Permitted statuses |
|---|---|---|---|
| `outcome` | Outcome | docstring summary or explicit doc mention | documented, unresolved |
| `public_access` | Public access | Pass A `public_alias`, `exports` | structurally_observed |
| `coordinates` | Public access, "already coordinates" | Pass A `direct_delegation`, `bounded_delegation_path` | structurally_observed |
| `parameter` | Important controls | `parameters` (name, kind, default, required) and `parameter_docs` (the description, its span cited), else a top-level `<ParamField>`'s lead (§10.3), citing its anchoring mention as `scope` | structurally_observed, documented (with parameter-doc evidence, or a `<ParamField>` lead under `semantic:documented-parameter-cites-its-doc`) |
| `control` | Important controls | Pass B `forwarding`, per seed parameter, with the path qualifiers | structurally_observed (no recognizer documents a forwarded parameter) |
| `transformed_control` | Important controls | Pass B `transformed_argument` | structurally_observed |
| `restriction` | Limits and prerequisites | Pass B `conditional_raise`, with the test's and raise's syntax facts and the path qualifiers | structurally_observed (a public precondition would need documented support no recognizer gives) |
| `unfollowed_control` | Limits and prerequisites | Pass B `unfollowed_argument`, per seed parameter | structurally_observed |
| `usage_pattern` | Usage pattern | `cpg_core::usage`: a verbatim statement subset of official usage code, its `example` spans, and each handoff it shows | documented, fixture_checked |
| `handoff` | Usage pattern | Pass C `handoff`: the other callable, the formal and the occurrence count | structurally_observed |
| `analysis_boundary` | Limits and prerequisites | `boundaries` on the seed's neighbourhood | structurally_observed |
| `related` | Related | community co-membership (`+communities`), ordered by direct usage (PageRank in its variant) | statistically_derived |
| `applicable_case` | Applicable input or mode | reserved: no source yet | structurally_observed |
| `implication` | Important controls | an FCA `implication` of the seed's own scope whose premise it meets (`+fca`) | structurally_observed |
| `doc_link` | Related | kNN `doc_link` findings (`+knn`) | statistically_derived |
| `shared_signature` | Related | the FCA concept of the seed's own scope with the most shared pairs (`+fca`) | structurally_observed |
| `documented_warning` | Limits and prerequisites | a `<Warning>` component of a passage that mentions the seed exactly, its inner bytes cited, and its anchoring mention cited as `scope` (§10.3; `semantic:documented-warning-anchored`) | documented |


### §10.3 Brief structure and the Outcome order

**One brief = one outcome, anchored to one public operation**, optionally with one direct handoff.

| Section | Filled from |
|---|---|
| Outcome | the order below |
| Public access | Pass A `public_alias` / exports, plus "already coordinates" from Pass A delegation findings (with the note that these are implementation details the caller does not need to rebuild) |
| Applicable input or mode | an input or mode source: none yet, so the slot is absent (FCA concepts are shared signatures under Related) |
| Important controls | Pass B forwarding + parameter docs |
| Usage pattern | Pass C handoff or official example, with setup preserved |
| Limits and prerequisites | Pass B restrictions, documented warnings, boundaries |
| Evidence | all cited findings and evidence ids |
| Related | with the variants on (§9.8): other public APIs of the seed's community, at most five, the most called in official usage first (`statistically_derived`); the seed's shared signature (`structurally_observed`); doc links (`statistically_derived`). The default analytics publish no Related line |

**Outcome order.** Take the first source that applies:
1. the entry point's docstring summary: the first sentence of the docstring's first paragraph;
2. the lead sentence of a doc paragraph that **explicitly** mentions the entry point, when the
   mention lies inside that sentence;
3. otherwise `unresolved`.

Sentences are found with UAX #29 on a view in which each line break is a space, so a hard-wrapped
sentence is one. The evidence is the source bytes; the assertion reads the sentence with its
whitespace collapsed. Change-log documents (`synth::CHANGELOG_STEMS`) never give an Outcome.

The nearest doc passage by embedding is never an Outcome. It is published only as a doc link
(`statistically_derived`); otherwise statistical text would fill exactly the slots the gap metric
counts.

**The count of `unresolved` slots, by section, is the [§B11](../DESIGN.md#section-b11) gap
metric.** Read from a published snapshot with `lctx query`:
`SELECT p.brief_section, count(*) FROM assertions a JOIN assertion_policy p ON
p.assertion_kind = a.assertion_kind AND p.evidence_status = a.evidence_status WHERE
a.evidence_status = 4 GROUP BY p.brief_section ORDER BY 1`. A brief with no assertion in a slot
section (`findings::SLOT_SECTIONS`) has an **absent slot**, which the served summary counts beside
explicit `unresolved` assertions.

**How the sections are filled** (`cpg-core::synth`; **Implemented** and **Tested**):
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
    (`parameter_docs`, citing its span). Otherwise the lead paragraph of a **top-level**
    `<ParamField>` whose literal `body` is the parameter's name, in a passage that mentions the
    seed exactly (changelogs are never read), citing that paragraph's bytes as Passage evidence
    and the anchoring mention as `scope`. *Top-level* means **no `<ParamField>` ancestor**: a
    `<Card>` or `<Expandable>` around the field does not count. If several such fields disagree
    after normalization, none is used. A `<ParamField>` nested in another describes a field of its
    parent's value, never a parameter. Fields named by `path`, `query` or `header` are not read.
- **Limits:** one `analysis_boundary` assertion per stop reason (dependencies, synthesized
  callables, release code outside the subsystem, unresolved sites). What the seed calls itself
  is kept apart from what the callables it reaches call. Plus the depth bound or a budget
  truncation, citing Pass A's `traversal_stop` finding.
- **Documented warnings** (Limits): one `documented_warning` assertion, `documented`, per
  `<Warning>` component of a passage that mentions the seed exactly (changelogs are never read).
  Its text is the warning's inner bytes, verbatim, with where it is ("in `docs/x.mdx` § Heading
  (which mentions `seed`)") and its literal `title` when it has one, citing those bytes as
  Passage evidence. Inside a `<ParamField>`, the warning is about the **nearest** such field's
  parameter and says so ("about the parameter `p`"); if `p` is not a parameter of the seed other
  than its receiver, the warning is skipped. A `<Warning>` in fenced or inline code is not a
  component, so it is never read. The anchoring mention is cited as `scope`, and
  `semantic:documented-warning-anchored` checks the quote, the anchor and the scoping. Warnings
  enter the brief document as the capability's own limits.
- **Tested** (2026-09-24; `docs_shapes`, `a_documented_warning_is_a_limit`,
  `documentation_statements_are_anchored_to_their_seed`): a warning under a field that is no
  parameter is skipped; the nearest field decides (a warning under `backoff`, itself inside the
  parameter `retries`, is skipped); a field nested under `ParamField > Expandable` describes
  nothing; a field inside a `<Card>` describes its parameter; a warning in a passage that mentions
  no seed is not stated; each rule rejects a doctored anchor, quote, scope or nesting.
- **The components are Mintlify's** (`cpg_schema::mdx`, read by Stage F and the rules). A library
  documented another way has none, so a zero there means no such source.
- **Known limit of the exact anchor** (Measured on the pilot, 2026-09-24): of 93 `<Warning>`s and
  173 `<ParamField>`s, none reaches a brief. The passages that document the seeds' arguments name
  the operation by a receiver-variable spelling (`@mcp.tool`), which is no exact mention. The
  exact anchor stays: heading/title and code-block anchors were rejected as noisy (one
  `mcp.run()` anchor attaches 44 passages). A receiver-resolved mention is the one widening
  candidate; it is not scheduled.
- **The brief document** ([§11.1](#section-11-1)): outcome, applicable case (when one exists),
  public APIs, control names, usage and limits. Above the spec's 2,048-token cap, estimated with
  a declared proxy of 4 bytes per token (`synth::DOCUMENT_BYTE_CAP`), it is split into chunks of
  whole parts, each under the header (the outcome and any applicable case), never truncated; a
  part too long for any chunk fails the compile. The proxy only prepares chunks; token admission
  is §11.1's. Retrieval scores a brief by its best chunk.
- A template or extractive-rule change bumps `synth::TEMPLATE_VERSION`; the analysis output is
  pinned to the versions in a test ledger.


### §10.4 Grounding checks

These are mechanical and run before publication (**Implemented** and **Tested** as the shared
analysis rules).
- Every named public symbol and parameter exists in this snapshot.
- Every cited finding, witness path and evidence id exists in this snapshot.
- Every snippet parses, and every name it loads is bound in it, imported by it or a builtin
  (§10.5). Whether each imported API exists is the release's to say: a pattern is verbatim
  official code.
- No assertion's status exceeds its evidence (the §10.2 rule).
- Warnings and unresolved conditions are never dropped for length. An over-long brief's document
  is chunked at whole parts instead (§10.3); the brief itself is never split.

**Manual review** (ADR-0005; accepted target, **not implemented**). Each brief is to get one
manual review pass before publication, checking that its extracted sentences are true of
**this** entry point, which mechanical checks cannot establish. The result is recorded as
`briefs.review_state` (outside `brief_id`). The column exists, but every brief is currently
published `unreviewed` and there is no review workflow.

Repository text is treated as untrusted data. It is never an instruction to the compiler.


### §10.5 Usage patterns

- **Contents.** One principal operation, or one direct handoff, plus the setup it needs
  (initialization, schema, resources, configuration).
- **Trimming.** Test-only details are removed only when that removes no precondition.
- **Publication.** A pattern is published only with an official example, a relevant test, or an
  executed fixture.
- **Fixtures** run offline, with no network or credentials. A pass supports only the tested case.

**Implementation** (`cpg_core::usage`; **Implemented** and **Tested**, 2026-09-23):
- **Selection.** Candidates are the seed's call and decorator sites in official examples, then doc
  blocks, then tests; within a role a handoff's sites are tried first. The pattern that shows a
  whole handoff occurrence (producer and consumer site) wins within its role, then the smallest.
- **The pattern.** It is the statement holding the site, in a module or function body only (never
  under a `with`, `if`, loop or `try` header it would drop, such as `with pytest.raises`), plus,
  transitively, the same-block statements before it that bind a name it reads, and the imports
  binding one. A candidate that reads a name bound anywhere else, or bound nowhere (neither a
  binding nor a builtin, as in a continuation doc block), is not self-contained and is refused.
- **Publication.** Each statement is dedented by its own indentation. The code must parse (Ruff),
  and every name it loads, annotations included, must be bound in it, imported by it or a builtin
  of the analyzed Python (`ruff_python_stdlib`); otherwise the candidate is refused.
  `lctx_mcp.smoke` parses every served pattern again.
- **Assertion.** The pattern is a `usage_pattern` assertion (`documented`) citing each
  statement's `example` span verbatim, and each handoff it shows whole
  (`semantic:usage-pattern-shows-its-handoff`). Its code enters the brief document as §11.1's
  usage description.
- Executed fixtures (`fixture_checked`) are permitted by the policy but not produced yet.

> Decision: ADR-0005, ADR-0047

---

## §11 Serving and agent interface

**Label.** The interface, retrieval and embedding contracts are accepted (ADR-0043). Lines cite
their own evidence: vLLM and model behaviour was **Tested** against the pinned service on
2026-09-22; the server and clients are **Implemented** and **Tested** as stated per section;
the native semantic executor is **Proposed** (ADR-0025) with a partial implementation (§11.3).
Pins are in `docs/pins.md`.

> Decision: ADR-0046, ADR-0043


### §11.1 Embedding spec and vectors

**Model.** Qwen3-Embedding-8B at a pinned revision (`docs/pins.md`), served by a **separate
vLLM service** from the locked `services/vllm` project (`just embed-serve`: `vllm serve …
--runner pooling --max-model-len 8192`). vLLM is never a dependency of the compiler's Python
tools or of `lctx_mcp`; running it as its own service also keeps GPU use explicit.
- **Output:** 4,096 dimensions, `Float32`, cosine.
- **Normalization** (**Tested**, 2026-09-22). vLLM L2-normalizes: every norm was 1 ± 1e-7. The
  pooling (`LAST`, with activation) comes from the model's sentence-transformers config.
- **Never send `dimensions`.** We use the full 4,096, and vLLM rejects the parameter without a
  Matryoshka override.
- **Footprint** (Measured, 2026-09-22): ~15.5 GiB of weights, ~27 GB of the RTX 5090 at 0.80
  utilization, 85 s to start. This is why live GPU legs run only with the service started
  deliberately and stopped afterwards.

**Spec.** One committed canonical JSON, `specs/embedding/qwen3-embedding-8b.json`, whose SHA-256
is `spec_hash` (`cpg_core::embed::Spec`; `the_committed_spec_is_its_canonical_form`). It covers
the model and revision, tokenizer revision, vLLM version and served dtype (bfloat16), pooling,
query instruction and template, document template, dimensions, output dtype, normalization and
the document token cap (2,048). **One spec governs every vector** in a generation; mixing spec
hashes is rejected (`semantic:one-embedding-spec`).
- **Query template (query only):** `Instruct: {task_description}\nQuery:{query}`. There is no
  space after `Query:`. Documents take no prefix, so one spec identifies one vector space for
  briefs and operation views alike.
- **One query instruction.** `search_capabilities` and `search_operations` embed queries with the
  spec's one instruction, which names capability briefs. A per-tool instruction would change
  `spec_hash`, and so every cached key; it waits for its trigger, the structured evaluation
  attributing operation-search misses to the wording.
- **Documents.** The brief document is the deterministic projection in §10.3: outcome,
  applicable case (when one exists), public APIs, controls, usage description and limits. The
  limits are the capability's own (Limits-section kinds other than `analysis_boundary`); what
  the analysis did not follow stays in the served brief, out of retrieval. Each public
  callable's **views** (`operation_documents`: signature and docstring, and source body; ADR-0021)
  are documents too. A view is a **column** (`operation_documents.embedding_view`,
  `operation_vectors.embedding_view`), not part of the cache key, so two views with the same text
  share one vector.
- **Rejected responses:** wrong count, wrong index mapping, wrong length, non-finite values,
  norm ≠ 1 ± ε, model mismatch.

**Token admission.** The intended contract: every text embedded under the spec is admitted by the
spec's tokenizer at or below the document token cap before its vector enters the cache. Byte
limits (the 4-bytes-per-token proxy that chunks brief documents, and the 4,096-byte windows that
cut views and E0 texts at line ends) only prepare texts; a byte window is **not** a guarantee of
staying under 2,048 tokens. **Current state:** brief documents go through `embed_documents`, which
counts tokens with the service's `/tokenize` for keys the cache lacks and fails the compile on an
over-cap document (**Tested**: `only_documents_the_cache_lacks_are_counted_and_embedded`,
`completed_batches_survive_a_later_failure`). Operation views and E0 go through `embed_texts`,
which has no tokenizer admission, so a view window above the cap is not detected, and a cached
key admitted by one path is trusted by the other
([plan W9](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**Clients.**
- Compile-time vectors come from Rust (`crates/lctx-embed`: `reqwest` + `tokio`, without TLS,
  since the service is local; 10 s connect and 300 s request timeouts).
- Query-time vectors come from Python (`httpx2`, pydantic's continuation of `httpx`, which
  FastMCP already depends on).
- **Conformance** (**Tested**; ADR-0043). Over the shared conformance inputs, both clients build
  byte-identical request bodies (`specs/embedding/request_bodies.json`), apply the same rejections
  (`every_rejection_fires`) and judge responses alike, Rust by its serde types and Python by
  pydantic strict models, held to one corpus of 25 bodies (`specs/embedding/responses.json`;
  `responses_are_judged_as_the_shared_corpus_says` in both suites). Against the live service they
  agree to cosine ≥ 0.9995. vLLM is not bitwise deterministic across requests (identical inputs
  differed by up to 3.8e-3 in a component, Measured 2026-09-22), so an exact vector match is
  never the oracle. A stub service exercises the real HTTP path; an unreachable service is
  `blocked`, never faked.
- **The fake embedder** (`FakeEmbedder`, its own spec) draws a unit vector by splitmix64 from the
  request text's SHA-256, so Python reproduces it with its standard library
  (`specs/embedding/fake_vectors.json`). Tests and `just check` use it; it qualifies cache and
  retrieval mechanics only, never vector meaning.
- **Deployment identity.** The spec hash identifies a declaration, not the running deployment.
  The launch recipe repeats the revision and dtype instead of deriving them from the spec, and
  both clients accept an endpoint by model name plus vector shape, so spec-hash equality between
  generation and server does not establish which deployment produced a vector. The claim is
  limited to the operator-controlled `just embed-serve` launch until launch arguments are derived
  from the spec and an endpoint contract is declared
  ([plan W16](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**Cache.** Vectors are keyed by `spec_hash + input_hash` in the canonical `embedding_cache`
Delta table ([§3.2](facts-and-identity.md#section-3-2)), because live numerics vary between
requests: the committed vector, not a recomputation, is the run input. Snapshots record the cache
version they read, and generations copy the vectors they need from it.
- **Writes** are an insert-only MERGE (`delta::merge_global`) on an append-only table: only Add
  actions, only missing keys inserted, CHECKs enforced, and concurrent merges of one key leave
  one row (**Tested** at the pinned delta-rs:
  `an_insert_only_merge_adds_only_missing_keys_to_an_append_only_table`,
  `a_merge_enforces_the_immutable_checks`, `concurrent_merges_of_one_key_leave_one_row`).
- Only keys the cache lacks are embedded, so a fully cached `--embedder vllm` compile needs no
  service; batches completed before a failing one are merged, so a rerun embeds only what is
  missing.
- **Known gap.** A cache fill should return exactly the committed winner for each key. `embed_texts`
  returns the vectors it computed locally without rereading, so under a concurrent fill E0/kNN
  can consume a vector that differs from the committed one the snapshot and generation use; and
  the two fill entry points apply different admission (above). The intended fix is one cache-fill
  operation owning admission, batching, insert-only merge and versioned readback
  ([plan W9](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- `lctx compile --embedder vllm|fake|none` selects the embedder.


### §11.2 Retrieval

**In-process, exact, over the pinned generation** (ADR-0043; `lctx_mcp.retrieval`;
**Implemented** and **Tested**). At the current corpus size exact search needs no index service;
LanceDB waits for its trigger (below).

1. **Lexical.** BM25 over `lexical_text`, scored by `bm25s` (`method="lucene"`, k1 1.5, b 0.75,
   numpy backend, `get_scores`) over our own tokenization: lower-cased runs of letters and digits.
   A brief's lexical text is its document text plus the distinct **name** tokens of its seed's
   **own** public spellings (`public_paths.own`), each once (`write_dataset` → `write dataset`),
   computed in Rust when the generation is built; Rust and Python share known answers in
   `specs/serving/tokens.json`. Inherited spellings are left out, so the number of subclasses that
   inherit a name does not inflate document frequency. The document text keeps its natural term
   frequency.
2. **Discriminating words only.** The lexical leg scores only query words that occur in some
   briefs but not all, and **abstains** when there are none: a word every brief contains cannot
   tell them apart. The vector leg always votes.
3. **Vector.** Exact cosine over the generation's vectors, using the instruction-prefixed query
   vector; a brief's vector score is its best chunk's cosine.
4. **Fusion.** Reciprocal-rank fusion with K = 60, 1-based ranks, ties broken by the lower
   `brief_id`.
5. **Exact symbols.** A query naming any public spelling of a brief's seed, own or inherited
   (`symbol_map` from `public_paths`), promotes that brief; promotion is merged by brief id,
   **recorded as promoted**, and ranks first.
6. **Return.** At most `limit` results (default 5), with relevance and coverage information.
   **A score is never presented as proof of task fit.**
7. **Degraded mode.** Lexical + exact-symbol only when there is no query vector (no embedder, no
   vectors, or the service down), reported as `lexical-only` with its reason. Embedder failures
   are caught inside the search tool, never surfaced as a retry.

`search_operations` applies the same fusion per operation view, labelled `ranked_discovery`.

**Registered parameters.** Tokenization, fusion, the brief-document template and every parameter
above are registered. None changes on the strength of gold scores; a change needs an ADR with a
rationale independent of the gold ([§12](validation-and-evaluation.md#section-12) owns the
evaluation itself).

**Hydration.** `(snapshot_id, brief_id)` → the full brief, all conditions, limits, evidence and
usage patterns, by deterministic lookup. It never depends on a second search.

**Tests** (`python/lctx_mcp/tests`): bm25s equals a hand computation of the Lucene formula
(`test_bm25_scores_are_the_lucene_formula`), unknown words count for nothing; a word every brief
contains does not vote (`test_a_word_every_brief_contains_does_not_vote`); RRF ties go to the
lower brief id and a promoted brief ranks first
(`test_fusion_ranks_ties_by_id_and_promotes_exact_symbols`); a down embedder gives `lexical-only`
with its reason (`test_a_down_embedder_degrades_to_lexical_and_says_so`). The fixture's fake
vectors carry no meaning, so ranking quality is an evaluation question, not a unit test.

**LanceDB** is deferred behind its trigger ([§13](../DESIGN.md#section-13)): more than ~10⁵
vectors at 4,096 dimensions, or filtered ANN together with managed FTS. Its hybrid, FTS and RRF
call chain is Interface-checked; it would sit in an isolated workspace with its own lock, and its
wheel isolates its own Arrow/DataFusion line, so it would not affect
[§B9](../DESIGN.md#section-b9).


### §11.3 FastMCP contract

- **Package.** `python/lctx_mcp`. It depends on `fastmcp` 4.0.x, `pyarrow` (the generation
  reader), `numpy`, `bm25s` and `httpx2`, never on vLLM, Delta or the compiler; versions are in
  `docs/pins.md` and the uv lock. The native executor is the separate workspace member
  `python/lctx_semantics` (ADR-0025).
- **Startup checks.** The lifespan loads the generation once and rejects one whose manifest
  format, condition-kernel format, per-file schema digests (from `cpg-schema`'s canonical schema
  form) or `embedding_spec` hash differ from what the server and its query client expect
  (`test_a_mismatched_generation_fails_at_connect`). The manifest names the library and
  requirement, so tools check a requested library against the generation itself.
- **State.** The generation is exposed through `ctx.lifespan_context`. One generation per
  process.

**Tools** (all typed pydantic inputs, object outputs, never bare lists, and
`ToolAnnotations(read_only_hint=True, idempotent_hint=True, open_world_hint=False)`):

| Tool | Parameters | Output | State |
|---|---|---|---|
| `search_capabilities` | library, query (1–4000 chars), limit (1–10) | `SearchResult`: snapshot, generation key, mode (hybrid or lexical-only), coverage summary, hits (brief id, title, outcome, outcome `evidence_status`, relevance score, rank source, promoted flag) | Implemented, Tested |
| `get_capability` | snapshot_id, capability_id | `Capability`: all §10.3 sections, evidence and statuses | Implemented, Tested |
| `get_operation` | snapshot_id, operation (a public path or id, or a module-global singleton's name, which resolves to its class) | `Operation`: paths, signature, docstring summary, facets and which are incomplete, each parameter's fates (forwards, derives, stores, returns, raises, tests, is-read claims) with verdicts, conditions and lines, delegations, handoffs, settings read with their phase, a class's constructor record, a singleton's fields (their reads and never-read claims), boundaries, the brief id if any | Implemented, Tested |
| `find_operations` | library, `where`, limit (1–50), cursor | `OperationSet`: matches, `complete`, the operations that could still match, `truncated`, next cursor. Never vectors | Implemented (facet terms), Tested; semantic terms Proposed |
| `search_operations` | library, query, optional `where`, limit (1–10) | `OperationHits`: ranked, per-view RRF, labelled `ranked_discovery` | Implemented, Tested |
| `inspect_value_paths` | snapshot_id, operation, formal, exact primitive input, limit (1–50), cursor | A page of cited value-summary paths with path-local assessments, link operand spans and work counters | Implemented, Tested (native, path-local) |
| `lookup_concepts` | text, limit | Candidate concepts with labels and scope notes | Proposed (Stage 4) |
| `explain` | snapshot_id, claim id | The rule id, premises and source spans | Proposed (Stage 4) |

- **Resource.** A `capability://{snapshot_id}/{capability_id}` template shares the hydration
  code and returns Markdown text, not structured content.
- **Errors.** One domain exception, `CapabilityError`, is FastMCP's `ValidationError`: a tool
  returns it as an error result, and the resource answers it as invalid params (−32602), never as
  an internal error. It covers an unknown library, snapshot, id or path, a snapshot mismatch and
  a foreign cursor. Everything else is masked (`mask_error_details=True`).
- **Transport.** stdio, started as `python -m lctx_mcp --generation DIR --embedder
  vllm|fake|none`, which runs `mcp.run(transport="stdio", show_banner=False)` with
  `FASTMCP_CHECK_FOR_UPDATES=off`: FastMCP's banner otherwise makes an HTTP GET to PyPI at every
  start, a network call the design does not allow. Nothing may write to stdout, at import or in
  the lifespan either (`test_the_server_speaks_only_the_protocol_on_stdout`, a `StdioTransport`
  subprocess).
- **Not adopted:** FastMCP's response-caching middleware (it would keep serving a degraded
  result), response-limiting middleware (it drops structured output), and `fastmcp install` /
  `fastmcp.json` (unpinned).
- **Tests** (`python/lctx_mcp/tests`, over the `just py-fixture` generation built from
  `analysis_shapes`): `fastmcp.Client(mcp)` round trips in both protocol eras, asserting
  `.structured_content` (`test_the_tools_round_trip_in_both_protocol_eras`; the eras negotiated
  `2026-07-28` and `2025-11-25`, Tested 2026-09-22); promotion and degraded mode; tool errors for
  an unknown library, snapshot or id and for out-of-bounds arguments; the resource as Markdown
  with an unknown id as invalid params; schema digest, spec or key mismatches failing at load and
  at connect; byte-identical request bodies (`test_request_bodies_are_byte_identical_to_rusts`)
  and the fake twin (`test_the_fake_twin_reproduces_rusts_vectors`); the serving digests' known
  answers; the behavioral tools in `test_operations.py` and `test_server.py`.

**Executor.** The accepted current route (ADR-0043) is **lookup over materialized rows**: pyarrow
compute and indexed dictionaries over the generation, with no SQL built and nothing recursing at
serve time; paths are precomputed as summaries and witnesses; results have row caps, a
`truncated` flag and cursors. Python never re-implements predicate or condition semantics.
**Proposed** (ADR-0025): a pinned in-process Rust/PyO3 extension executes bounded semantic
queries (condition compatibility and implication, effect and role filters, witness traversal)
over the same immutable generation, returning cited row/node ids, budgets, unknown boundaries and
truncation; startup checks its ABI and kernel format against the manifest. Direct lookup and
retrieval stay on the materialized route.

**Served claim fidelity — known gaps.** The contract is that every served claim keeps its
verdict and its support closure in every form:
- `get_operation` drops each facet value's verdict: facets are returned as plain value lists, so
  an `unknown` facet value reads like an established one, although `find_operations` filters the
  same rows correctly. The intended response is typed `{value, verdict}` entries with separate
  completeness ([plan W2](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- Brief hydration resolves only direct evidence ids, so finding-backed claims cannot be followed
  to their model and source, and the Markdown resource omits per-assertion support
  (§10.2; [plan W3](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- The native loader keeps its own whitelist of proof-step kinds and decodes positional string
  tuples, so a proof kind the compiler emits (the finalizer step) is rejected and every new kind
  or column needs matching edits on both sides. The intended contract derives native admission
  from the schema codebook with one typed decoder
  ([plan W1](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**`find_operations` semantics.** **`where`** is a conjunction of terms, with **no negation**
(a NOT would read absent facts as false). Implemented terms:
- `{facet, value}`: exact equality on an `operation_facets` row;
- `{kind}`: function, method or class;
- `{path_prefix}`.

An unknown facet is invalid params. An unknown **value** is invalid params only where every
operation's rows for that facet are complete, and the error names close values; elsewhere
absence is not known, and the answer is empty with `complete = false`.

**Completeness is served data**, not a class Python decides (**Implemented** and **Tested**):
- `operation_facet_status` holds, per operation and facet, `established` when its rows are
  complete, or the verdict and why they are not:
  - a class's `parameter` and `parameter_type` are its public constructor's (its own or inherited
    `__init__` path), and without one they are `not_analyzed`;
  - `returns` does not apply to a class;
  - `raises` is never complete (only a typed `raise` directly in the body is a row);
  - `forwards_to` and `delegates_to` take the operation's `behavior_status`;
  - `hands_off_to` and `takes_from` are never complete (two usage shapes only).
  Facet names are held to the codebook by `specs/serving/facets.json`.
- `operation_facets` rows carry a verdict. Only `established` and `conditional` rows **match**. An
  `unknown` row (a path through an override-open call) leaves its operation open.
- **An operation hides a possible match** when, for every term, it matches or is open (its rows
  are incomplete, or its row is `unknown`), and it is not a match. `complete` is true exactly when
  no operation in the universe hides one.
- `unknown` lists the hiding operations, capped at 50, with a count and a flag. It is evidence, not
  a bound. **`complete` is never "not truncated"**: truncation has its own flag.

**Cursor and snapshot.** The cursor is opaque and binds the generation key, the request's
canonical hash and an offset. A cursor from another generation or request is invalid params.
Every result names its `snapshot_id`.

**Proposed Stage 3 semantic terms** (ADR-0025): `{effect}`, `{role}` and
`{compatible_with: <typed condition>}`. The condition grammar is closed and canonicalized by
Rust. Compatibility and implication use the bounded condition kernel and stable-place theory
(ADR-0024, [§3.9](behavior-model.md#section-3-9)); an undecided condition leaves its operation in
`unknown`. A condition term is anchored to an operation's entry formal. The Rust flow/summary
producer writes `flow_test_value_links`: operation/formal id, leaf evaluation atom identity,
`flow_test_leaves.fact_id`/`flow_uses.use_id` and operand span, resolved place/path, proof origin,
cited flow/summary facts and effect-model digest. It certifies direct paths without calls or
writes first; a later modeled transfer must be identity-preserving, not merely pure. The shared
validator rejects ambiguous binding, alias uncertainty, unmodeled effects, mixed snapshots and
digest drift. Missing links are `unknown`, never source-atom matches by spelling. Compatibility
means may-model non-refutation, never proof of feasible execution. Candidates partition into
matched, proven-excluded, source-open and unexamined; unvisited or undecided operations make
`complete = false`; an early budget stop returns an unexamined count and a resumable
generation/query-bound cursor. Row, depth and pair-work budgets have the same unknown/truncated
behavior.

**Implemented so far toward that target** (**Implemented** and **Tested**, 2026-09-25, focused
and internal):
- The direct `flow_test_value_links` origin and its publication validator exist. FORMAT 7
  carries structural analysis conditions, finite value-summary proofs, the checked value-link
  and test-leaf projections and the exact effect-rule digest; they load through one immutable
  native index.
- The index admits a link only when its leaf atom, source module, condition, place, public
  operation/formal and digest agree, and invokes the shared primitive-theory kernel to assess one
  cited summary path for an exact query input. A checked assignment that leaves the BDD
  satisfiable is `compatible_under_model`; a missing applicable link is `unknown`; an
  unconditionally true path is compatible without a link. These are path-local may-model
  outcomes, never concrete executions or operation-wide verdicts.
- `inspect_value_paths` exposes this per public formal, cursor-paged (the cursor binds generation
  and query), with cited link operand spans. It does not aggregate an operation-wide verdict or
  offer the cross-operation compatibility/effect/role filters, and proof steps carry evidence ids
  rather than full step spans.
- Work evidence is returned even on a budget refusal: checked link rows, completed assignments,
  the sum of BDD input-node-product preflights (a conservative bound, not a count of internal BDD
  visits) and the peak result-node count, per path; the page sums the first three and takes the
  peak over displayed paths, and `examined_rows` separately counts paged summary/boundary rows.
  Assignment-cap refusal remains `unknown` with `budget_reached`. Page totals cover this page
  only, never unexamined cursor rows.
- A clean-wheel installation running a generation-pinned native query is outstanding (plan
  Stage 3 order 9).

**`explain`** (Stage 4) returns the stored derivation: rule id, premises and spans, each derived
row storing its rule id and proof height. Under ADR-0025 the native executor may traverse bounded
stored witness links at request time, retaining their row ids and reporting a boundary if the
depth budget is reached.

> Decision: ADR-0043, ADR-0025, ADR-0046
