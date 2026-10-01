# Synthesis and serving

**Target, 2026-09-29.** [§15](semantic-model.md) (ADR-0085/0083/0084) is the accepted target, and the [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md) delivers it layer by layer. Phase 5 replaces the serving generation with generated views over the canonical PostgreSQL generation (§15.12). Until that phase exits, this page describes the implemented legacy pipeline.

This owner covers how typed findings become cited assertions and briefs (Stage F, §10), how
texts become vectors under one hashed embedding spec, and how one pinned serving generation is
exposed to coding agents through the FastMCP server (§11). Its inputs are the analytics findings
([§9](analytics.md#section-9)), behavioral relations ([§9.9](behavioral-analysis.md#section-9-9)),
extracted evidence spans and the published generation
([§6.4](storage-and-publication.md#section-6-4)); its outputs are the `assertions`/`briefs`
analysis tables, the consumed-vector receipts and the tool responses agents read. Dependencies run
one way: synthesis reads findings and evidence, never the reverse; the server reads only an
immutable generation and never Delta, DataFusion or the compiler. Authoritative declarations
live in `crates/cpg-schema/src/findings.rs` (kinds, statuses, `ASSERTION_POLICY`) and
`bundle.rs` (the generation format); Stage F is `crates/cpg-core/src/synth.rs` and `usage.rs`;
embedding is `crates/cpg-core/src/embed.rs`, `crates/lctx-embed` and
`specs/embedding/`; serving is `python/lctx_mcp` (tests under `python/lctx_mcp/tests/`) with the
native executor in `python/lctx_semantics`. Rationale: ADR-0106 (programmatic synthesis),
ADR-0078 (interface, retrieval, embeddings) and proposed ADR-0025 (native executor). See the
[architecture map](../README.md).

## §10 Synthesis and briefs

**Accepted Phase 4 target, not implemented, 2026-09-30 (ADR-0106).** The
[detailed plan §8](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#8-catalog-selection-synthesis-and-retrieval)
puts shared findings/assertion policy, mandatory catalog, pure requirement selection and optional
grounded briefs under lctx-model. It retains original evidence and independent closure per value.
Phase 5 owns request loading, generated wire schemas, leases, ranking and served journeys.
Conditional operator review replaces the unconditional per-brief review target. Until the
forward plan selects its operator workflow, grounded briefs remain explicitly unreviewed.

> Decision: ADR-0106

**Label.** Programmatic synthesis is accepted (ADR-0106): there is no generative model in the
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

**PR1 Implemented and Tested, 2026-09-28 (ADR-0078):** [§14](api-and-evidence-product.md)
adds mandatory API catalog construction and optional valid briefs. Existing synthesis validators
still govern every emitted brief; absence of an admissible Outcome omits the brief with an explicit
reason while preserving the valid catalog. Original declaration evidence has independent catalog
roots. Broader contextual evidence remains PR3 work.

Briefs are one rendering of the analysis. Behavioral claims served through the operation tools
(§11.3) come from the behavior relations directly, not from briefs.

> Decision: ADR-0106, ADR-0086, ADR-0049, ADR-0071, ADR-0078


### §10.1 Findings

**A finding is a typed record** (`cpg_schema::findings`; ADR-0086), carrying:
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
- **Support closure (FORMAT 9, [ADR-0049](../../adr/0049-served-support-closure.md)).** The
  generation carries only cited findings, with invocation/model, ordered witness source spans and
  member fact identities. Startup refuses absent support edges or witness spans. Structured and
  Markdown renderings use the same hydrated support object. A member without a generic source
  span is marked `fact_only` or `unavailable`, not silently treated as a resolved citation.
  **Implemented (2026-09-27):** typed attributes and their source incidences accompany cited
  FCA/RCA and handoff findings. FCA evidence is restricted to the finding's supporter objects;
  handoff evidence to its retained pairs/formal, with both endpoint fact/model references.
  Publication and startup reject missing or foreign support, malformed pair members and incomplete
  incidence shapes. A handoff pair is not a delegation-chain witness. Labels are rendered from
  typed attributes by the schema-owned renderer (template version 20). Synchronous
  context-protocol evidence is **Implemented and focused Tested (2026-09-27)** under ADR-0059.
  Source-reached actions retain class assertions, constructor roles and lifecycle sites. Ordered
  return-completion commitments bind entry, exit and exception evidence. Rust shares structural
  admission; Python transports the validated projection. Full S6 semantics remain open.

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

**Conditional operator review** (ADR-0106; accepted target, **workflow not implemented**,
2026-09-30). Automated grounding permits explicitly unreviewed briefs. It does not establish that
an extracted sentence is true of this entry point or imply manual approval. Phase 4 adds no
mandatory review queue. The forward plan's [F1 trigger](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-adoption)
selects an operator-facing review consumer before implementing exact-subject-revision events,
provenance/idempotency, optimistic conflicts, frozen canonical inputs and backup/restore/replay.
Never patch a published brief or infer approval from missing events. Existing dormant legacy
`briefs.review_state` values remain `unreviewed`; the column's existence is no workflow receipt.

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

> Decision: ADR-0106, ADR-0086, ADR-0049

---

## §11 Serving and agent interface

**Label.** The interface, retrieval and embedding contracts are accepted (ADR-0078). Lines cite
their own evidence: vLLM and model behaviour was **Tested** against the pinned service on
2026-09-22; the server and clients are **Implemented** and **Tested** as stated per section;
the native semantic executor is **Proposed** (ADR-0025) with a partial implementation (§11.3).
Pins are in `docs/pins.md`.

> Decision: ADR-0046, ADR-0078, ADR-0080


### §11.1 Embedding spec and vectors

**Accepted Phase 4 target, not implemented (ADR-0106):** one spec/value/admission/cache contract supports two
one-shot consumption relations: early analytic text/vector use and later retrieval-unit use.
Exact consumed bytes are canonical generation records for replay without a live service/cache.
Early analytics never depend on later briefs. P5 pgvector is a derived serving representation.
See [plan §8.6](../../plans/semantic-model-phase4-detailed-plan_2026-09-30.md#86-retrieval-inputs-and-embedding-realization).

> Decision: ADR-0106

**Model (Implemented, 2026-09-28; ADR-0080).** The published gpu-stack
Qwen3-Embedding-8B-NVFP4-r2 checkpoint (`docs/pins.md`), served by a **separate
vLLM service** from the locked `services/vllm` project (`just embed-serve`: `vllm serve …
--runner pooling --max-model-len 8192`). vLLM is never a dependency of the compiler's Python
tools or of `lctx_mcp`; running it as its own service also keeps GPU use explicit.
- **Output (Implemented and Tested, 2026-09-27):** 1,024 dimensions, `Float32`, cosine; source width 4,096.
- **Reduction and normalization:** the format-2 spec declares MRL prefix selection before L2
  normalization. Both clients send `dimensions=1024`. The launcher derives
  `is_matryoshka=true`, `matryoshka_dimensions=[1024]`, pooling `LAST`, activation and dimensions
  from the same spec. The operator selected this standard; no new dimension-quality gate applies.
- **Deployment:** the SM120 B3/r2 wheel and role E settings use 0.85 GPU utilization,
  16,384 batched tokens and disabled prefix caching. The operator controls service lifetime.
  Local accuracy and performance comparisons are outside this adoption scope.

**Spec.** One committed canonical JSON, `specs/embedding/qwen3-embedding-8b.json`, whose SHA-256
is `spec_hash` (`cpg_schema::embedding_spec::Spec`, re-exported by `cpg_core::embed`; `the_committed_spec_is_its_canonical_form`). It covers
the model and revision, tokenizer revision, full vLLM wheel version and activation dtype (bfloat16), pooling,
query instruction and template, document template, dimensions, output dtype, normalization and
the document token cap (2,048), format, source width, reduction and launch admission. **One spec governs every vector** in a generation; mixing spec
hashes is rejected (`semantic:one-embedding-spec`). The NVFP4 spec binds both revision fields
to the SHA-256 of the published checkpoint manifest, covering weights, tokenizer, configuration
and provenance. The controlled launcher checks the manifest identity and all file checksums;
local artifact revisions are not Hugging Face revision flags. NVFP4 vectors never reuse BF16 keys.
- **Query template (query only):** `Instruct: {task_description}\nQuery:{query}`. There is no
  space after `Query:`. Documents take no prefix, so one spec identifies one vector space for
  briefs and addressable retrieval fragments alike.
- **One query instruction.** Both search tools use: “Given a coding task, retrieve relevant Python
  library APIs, capability briefs, configuration options, source code, usage examples, and
  documentation.” A change invalidates the full spec and all affected vectors, including briefs.
- **Documents.** The brief document is the deterministic projection in §10.3. Addressable
  retrieval fragments come from the schema-owned API/options, source, scenario and documentation/
  deployment families ([§14.8](api-and-evidence-product.md#section-14-8)). Unit ownership and
  original anchors are distinct from rendered text; identical text under one spec can reuse a
  vector while every member retains its own evidence occurrence.
- **Rejected responses:** wrong count, wrong index mapping, wrong length, non-finite values,
  norm ≠ 1 ± ε, model mismatch.

**Token admission.** Every text embedded under the spec is admitted by the
spec's tokenizer at or below the document token cap before its vector enters the cache. Byte
limits (the 4-bytes-per-token proxy that chunks brief documents, and the 4,096-byte windows that
prepare retrieval fragments and E0 text) only prepare texts; a byte window is **not** a guarantee of
staying under 2,048 tokens. **Implemented, source-inspected 2026-09-27:** brief documents use
`embed_documents`; retrieval fragments and E0 use the shared embedding session. Admission
counts missing texts through the tokenizer and rejects over-cap requests before insertion.
Focused controls include `every_cache_fill_entry_admits_with_the_tokenizer` and
`completed_batches_survive_a_later_failure`; current qualification is owned by
([plan W9](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**Clients.**
- Compile-time vectors come from Rust (`crates/lctx-embed`: `reqwest` + `tokio`, without TLS,
  since the service is local; 10 s connect and 300 s request timeouts).
- Query-time vectors come from Python (`httpx2`, pydantic's continuation of `httpx`, which
  FastMCP already depends on).
- **Conformance** (**Tested**; ADR-0078). Over the shared conformance inputs, both clients build
  byte-identical request bodies (`specs/embedding/request_bodies.json`), apply the same rejections
  (`every_rejection_fires`) and judge responses alike, Rust by its serde types and Python by
  pydantic strict models, held to one corpus of 25 bodies (`specs/embedding/responses.json`;
  `responses_are_judged_as_the_shared_corpus_says` in both suites). Against the live service they
  agreed to cosine ≥ 0.9995 on the prior BF16 deployment; that is not an NVFP4 accuracy receipt.
  NVFP4 adoption checks only the live output contract, by operator instruction.
  vLLM is not bitwise deterministic across requests (identical inputs
  differed by up to 3.8e-3 in a component, Measured 2026-09-22), so an exact vector match is
  never the oracle. A stub service exercises the real HTTP path; an unreachable service is
  `blocked`, never faked.
- **The fake embedder** (`FakeEmbedder`, its own spec) draws a unit vector by splitmix64 from the
  request text's SHA-256, so Python reproduces it with its standard library
  (`specs/embedding/fake_vectors.json`). Tests and `just check` use it; it qualifies cache and
  retrieval mechanics only, never vector meaning.
- **Deployment identity.** The spec hash identifies a declaration, not the running deployment.
  The operator-controlled `just embed-serve` launch derives model, model revision, tokenizer
  revision and dtype from that hashed spec; local checkpoint files are checksum-verified
  before startup (ADR-0080). Both clients still
  accept an arbitrary endpoint by model name plus vector shape; their spec-hash equality with a
  generation does not attest to that endpoint's deployed revision. The supported deployment
  identity claim is limited to the operator-controlled launch. An arbitrary endpoint is an
  operator-supplied source of vectors whose deployment identity is **unverified**; do not claim
  spec-hash equality proves it. Broader guarantees need endpoint attestation and a new contract
  ([plan W16](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**Cache and receipts (Implemented, 2026-09-27).** PostgreSQL reuses one immutable winner
per `spec_hash + input_hash`. Bounded SQLx inserts use `ON CONFLICT DO NOTHING`, followed by a
separate READ COMMITTED statement. Runtime permissions forbid value replacement/deletion.
The attempt retains exact returned values before any consumer uses them; later consumers reuse
those bytes even if the service becomes unavailable or its disposable cache is restored.
`used_embeddings` and `embedding_uses` in Delta capture all operation, E0/kNN and brief inputs.
Content identity includes exact value digests, and bundle replay reads the published snapshot
alone. See [§6.5](storage-and-publication.md#section-6-5) and ADR-0078/0067.

`lctx compile --embedder vllm|fake` requires configured PostgreSQL. The deterministic fixture
API has an explicit uncached route; database errors never select it. `--embedder none` and
immutable readers remain usable without a database. Live numerical/conformance qualification
remains separate from deterministic integration under [W9/W16](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition).

### §11.2 Retrieval

**In-process, exact, over the pinned generation** (ADR-0078; `lctx_mcp.retrieval`;
**Implemented**, with focused **Tested** controls). PostgreSQL preserves exact full ranks.
The current runtime admits only the exact profile (ADR-0078).

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

`search_operations` uses addressable retrieval units and family fusion under
[§14.8](api-and-evidence-product.md#section-14-8): best member/family/channel ranks, RRF60 within
families and equal family-rank RRF60 across families. Returned winners identify original evidence.
Typed selection determines eligibility before either retrieval channel and exact-symbol promotion.

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

**Implemented PG route (2026-09-28; ADR-0078/0077/0078).** pgvector stores standard full-float
1024 vectors. Exact requests rank all eligible members before fusion, with a metadata-only
membership query admitting each winning unit. ANN admission, qualification and old-format readers
have been removed. Any future approximate route needs its own measured need and qualification.

> Decision: ADR-0077, ADR-0078

### §11.3 FastMCP contract

**Product contracts (ADR-0071/0077):** [§14.7–§14.10](api-and-evidence-product.md#section-14-7)
owns implemented typed selection and addressable retrieval; PR5 bounded implementation packets
and independent evidence/browsing remain Proposed.
**PR1 Implemented and Tested, 2026-09-28 (ADR-0078):** ordered
catalog contracts, optional semantic capability loading and versioned migration. Contracts below
also retain existing behavioral queries and semantic research.

**PR2 wire contracts Implemented (ADR-0073), 2026-09-28:**
[§14.7/§14.9](api-and-evidence-product.md#section-14-7) own the Rust requests and complete nested
current packets. All seven existing tools use generated Schemars input/output schemas through a thin
FastMCP `Tool`; the capability resource uses the same request/response owner. Rust validates before
effects and after final tagging/default insertion. Native work detaches from the GIL inside the
bounded worker/deadline envelope; cancellation keeps its admitted slot until completion. Generic
Python packet views serve presentation only; migrated semantic Pydantic declarations and Python
value-path cursor decisions are removed. `get_operation` enforces 32 KiB by default and 256 KiB with
`expanded=true`, refusing rather than truncating signature contracts. PR4 classifier semantics are Implemented;
PR5 tools remain Proposed; [forward-plan §6.2](../../plans/behavioral-model-forward-plan_2026-09-24.md#62-product-target-findings-and-recommendation-disposition)
owns qualification and finding disposition.

> Decision: ADR-0071, ADR-0078, ADR-0073

- **Package.** `python/lctx_mcp`. It depends on `fastmcp` 4.0.x, `pyarrow` (the generation
  reader), `numpy`, `bm25s` and `httpx2`, never on vLLM, Delta or the compiler; versions are in
  `docs/pins.md` and the uv lock. The native executor is the separate workspace member
  `python/lctx_semantics` (ADR-0025).
- **Startup checks (Implemented, 2026-09-28).** The lifespan opens `lctx_storage` with read-only
  credentials and pins one ready full generation/profile. Rust checks schema/extension/history,
  canonical manifest identity and digest-checked artifacts. Python checks the query embedding
  spec and initializes lexical consumers, plus native analysis only when advertised. Context includes library, requirement and
  complete coverage. Missing database/artifact or incompatible format fails explicitly; there is
  no file fallback. SQLx and native work own separate bounded lifetimes.

- **Catalog packets (Implemented and Tested, 2026-09-28).** `get_operation` accepts stable public
  member IDs and returns explicit choices for ambiguous declaration IDs. Its `catalog` section
  carries ordered source/provider signatures, constructor associations, binding roles and original
  source/type evidence. Default/expanded response limits are 32/256 KiB with explicit refusal,
  never signature truncation. Brief and native tools return typed `not_requested` when unselected;
  empty results or missing fates do not establish absence.

- **State.** The generation is exposed through `ctx.lifespan_context`. One generation per
  process.

**Tools** (Rust-owned Schemars/Serde inputs and object outputs, never bare lists, and
`ToolAnnotations(read_only_hint=True, idempotent_hint=True, open_world_hint=False)`):

| Tool | Parameters | Output | State |
|---|---|---|---|
| `search_capabilities` | library, query (1–4000 chars), limit (1–10) | `SearchResult`: snapshot, generation key, mode (hybrid or lexical-only), coverage summary, hits (brief id, title, outcome, outcome `evidence_status`, relevance score, rank source, promoted flag) | Implemented, Tested |
| `get_capability` | snapshot_id, capability_id | `Capability`: all §10.3 sections, evidence and statuses | Implemented, Tested |
| `get_operation` | snapshot_id, operation (a public path or id, or a module-global singleton's name, which resolves to its class) | `Operation`: paths, signature, docstring summary, facets and which are incomplete, each parameter's fates (forwards, derives, stores, returns, raises, tests, is-read claims) with verdicts, conditions and lines, delegations, handoffs, settings read with their phase, a class's constructor record, a singleton's fields (their reads and never-read claims), boundaries, the brief id if any | Implemented, Tested |
| `find_operations` | library, typed `selection` (default discovery), limit (1–100, default 20), cursor | `SelectionResults`: independently paged supported, unresolved and conflicting groups; contradiction count, contextual witnesses and joint applicability | Implemented; focused Tested |
| `search_operations` | library, query, typed `selection`, limit (1–100, default 20), cursor | Ranked selection groups with addressable winning units, exact route and channel/policy metadata | Implemented; focused Tested |
| `get_evidence` | snapshot_id, tagged evidence reference, limit, cursor | Original evidence or retrieval-unit header with bounded subjects/anchors and paged original bytes | Implemented; focused Tested |
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

**Executor (Implemented, 2026-09-28).** Rust uses generation-qualified SQLx queries for exact
selection and complete hydration over materialized rows. Python owns transport, lexical scoring
and compact rank fusion. Paths are precomputed as summaries and witnesses; responses have row/byte
budgets, a `truncated` flag and request-bound cursors. Python never re-implements predicate or
condition semantics. Pure Rust/PyO3 handles admitted bounded native queries.
**Proposed** (ADR-0025): a pinned in-process Rust/PyO3 extension executes bounded semantic
queries (condition compatibility and implication, effect and role filters, witness traversal)
over the same immutable generation, returning cited row/node ids, budgets, unknown boundaries and
truncation; startup checks its ABI and kernel format against the manifest. Direct lookup and
retrieval stay on the materialized route.

**Served claim fidelity.** The contract is that every served claim keeps its verdict and its
support closure in every form:
- `get_operation` now returns typed `{value, verdict}` facet entries with separate completeness;
  focused mixed-verdict tests passed ([plan W2](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- Brief hydration resolves only direct evidence ids, so finding-backed claims cannot be followed
  to their model and source, and the Markdown resource omits per-assertion support
  (§10.2; [plan W3](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).
- The native loader now admits proof-step kinds from the schema codebook, including the finalizer
  step, but still decodes positional string tuples. A single typed decoder and a real
  finalizer-bearing generation check remain
  ([plan W1](../../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)).

**Selection semantics (Implemented, 2026-09-28; ADR-0077).** `selection` is a conjunction of at
most sixteen typed requirements. The shared Rust classifier owns coverage, existential/universal
quantification, comparable-context conflict and whole-conjunction context intersection. Missing
facts remain unresolved unless a scoped domain establishes absence. A positive witness does not
establish complete coverage. Supported, unresolved and conflicting groups retain actual contextual
witnesses; contradictions are excluded and counted. Strict mode returns supported members only.
[§14.7](api-and-evidence-product.md#section-14-7) owns the complete contract.

Behavioral facets retain their five verdicts and per-operation completeness from
`operation_facet_status`. Class parameter facets use public constructor contracts; direct raises,
usage shapes and open calls do not establish complete absence. Selection consumes these facts
without converting unknown into false or changing the source verdict.

**Cursor and snapshot.** Each opaque group cursor binds the generation, canonical request,
retrieval policy and channel availability. Groups continue independently. Every candidate has a
public member ID, an optional behavioral operation ID and separate aliases. Raw member IDs can
be passed to `get_operation`; winning unit IDs can be passed to `get_evidence`. Metadata omission,
original-evidence pagination and classification coverage remain separate.

**Proposed Stage 3 semantic terms** (ADR-0025): `{effect}`, `{role}` and
`{compatible_with: <typed condition>}`. The condition grammar is closed and canonicalized by
Rust. Compatibility and implication use the bounded condition kernel and stable-place theory
(ADR-0085, [§3.9](behavior-model.md#section-3-9)); an undecided condition leaves its operation in
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

**Accepted consolidation target (Proposed implementation, 2026-09-26).** The expanded contract
uses the current bundle contract through a complete store/generation rebuild (ADR-0048/0078). The callable-support
subset is **Implemented (2026-09-27)**: private/nested formals validate callee controls without
entering public lookup. Flow fates also carry typed transfer and the scope that owns their condition;
Python preserves these fields instead of inferring transfer from presentation text. The remaining
expanded semantics are still proposed implementation. `source_parameter_identities` carries the
bounded lexical return-read certificate and its source citations; the Rust loader checks its
content identity, exact summary tuple and reverse proof-step closure. Source reconstruction owns
the lexical proof. This does not establish general native semantic reconstruction.
Typed effect/role terms and conjunctions of exact entry-formal primitive bindings are interpreted
and canonicalized by Rust. Python remains a protocol/rendering adapter. Shared typed proof
admission replaces shape-specific native rules, with full bounded source support. Coverage,
truncation and omitted evidence remain separate. The existing path-inspection API remains
path-local. Only the current format is accepted.

The channel contract additionally separates complete empty results from unanalyzed emptiness.
Conjoined semantic terms require compatible conditions and subject/phase scopes. Structural proof
admission and evidence closure are shared with compile-time semantics; labels are not query logic.

**Implemented and focused Tested (2026-09-27; ADR-0059), partial:** context support retains the
fresh manager occurrence and authored class assertions for implicit exits. Mandatory return
certificates bind ordered entry/exit evidence and condition scope, including empty obligations.
Shared Rust admission rejects missing, foreign, swapped or omitted evidence; Python does not
infer lifecycle facts. A separate context-entry value certificate binds both lexical reads and
the actual source origin; shared admission requires the exact active site, constructor argument,
assignment, later cleanup and return scope. A mandatory local value basis rejects omission even
when completion obligations are intact. Full raw-fact semantic support and remaining S6 queries
stay open; these structural checks do not independently prove a forged raw identity.

**Normal-exit model postconditions (Accepted target; implementation Proposed).** A future semantic response may expose a model
implication under an explicitly undischarged Normal outcome of its exact call occurrence. It must
keep that obligation distinct from path compatibility, proved actions and Definite/Potential
modality. A symbolic returned-resource endpoint is not a concrete resource or release pair.
The native executor retains the shared candidate/invocation contract and source support; Python
adapts it. Until this consumer is implemented, the relation is not served as a successful action.

> Decision: ADR-0057, ADR-0058, ADR-0059, ADR-0062

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

> Decision: ADR-0078, ADR-0025, ADR-0046, ADR-0049, ADR-0071


**Accepted frame-completion target, implementation in progress (ADR-0063).** Every modeled normal-call obligation,
including a returned root call, must cite its exact frame-release certificate and complete
argument domain. The native consumer uses shared admission; model identity or body-return
syntax alone is insufficient. This remains part of S6 until source, publication and native
mutation controls pass. Typed Normal postconditions remain independent of actual completion.

> Decision: ADR-0063

<a id="section-11-4"></a>

### §11.4 PostgreSQL serving and conditional workflows

**Implemented and bounded Tested, 2026-09-28; live embedding waived for PR4.** Current-format PostgreSQL
serving uses exact ranking and explicit generation pinning. PR4 replaces obsolete runtime state
after validation; no historical reader or ANN activation path remains.
The [PostgreSQL workstream](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream) owns execution;
[§6.5](storage-and-publication.md#section-6-5) owns effects and physical storage.

`cpg-schema` declares manifests, relation shapes/keys and complete support validation once.
`lctx-postgres` owns SQLx operations and codecs; `lctx_storage` provides explicit coarse awaitables
on one process Tokio runtime and lifespan-owned pools. The serving native extension excludes DataFusion,
Delta and compiler code. `lctx_semantics` remains a pure bounded IPC/kernel boundary. Offline reference loading exports schemas from Rust, retains an independent Python digest oracle
and shares projection validation. Online Python retains only the pinned descriptor, native image
and lexical state; Rust selects and hydrates complete relational answers. Dense Python vector
matrices and production relational dictionaries/selectors have been removed after same-input
reference parity. Native work releases the GIL, uses two bounded worker slots, and keeps its slot
through cancellation until actual completion; each MCP request has a 30-second total deadline.

PG12 imports invisible generations through bounded COPY staging, freezes writes, verifies every
relation and required artifact, then publishes readiness atomically. Each server pins a ready
generation/profile once; every selection, cursor and hydration binds those identities. Missing
support/artifacts and database loss are explicit failures. No hot switching, silent file fallback
or reader-unsafe ready-generation deletion is introduced. Exact semantic selection preserves
unknowns, coverage and the five verdicts. SQL selects materialized facts; native Rust interprets
bounded semantics. Ranked retrieval never defines the exhaustive operation universe.

PostgreSQL owns exact scores/ranks; Python retains lexical scoring and family fusion, and native assembly validates the shared ranking contract. Old fixed-view ANN code is removed; future adoption requires current-content qualification. PG15 owns admitted DataFusion predicates and coherent read views. Independently
pooled mutable reads are not one snapshot; the initial coherent report materializes under one
bounded read-only repeatable-read transaction.

Conditional later work remains: attributed operator events frozen to exact subject/revision;
Psycopg 3/SQLAlchemy only for a distinct Python-owned domain; pgrx only for a measured SQL-side
kernel consumer; ADBC/protocol/notification/FTS/topology features only at the plan's named triggers.
None adds a second migration owner or changes model/evaluation meaning.

> Decision: ADR-0078

**PR4 Implemented and bounded Tested under the live-embedding waiver (2026-09-28; ADR-0077/0078).** Bundle16/projection6/wire3
bind the mandatory catalog, explicit optional capabilities, contextual selection and addressable retrieval.
Publication/import validate stored rows and complete artifact closure before readiness. Exact is the sole
current vector route; old ANN admission/routing and historical-runtime recovery code are removed.
Current-format reconstruction, typed selection, native CPU admission, winning-unit references and full
code/live qualification remain separately reported in the forward plan and PR4 evidence.
