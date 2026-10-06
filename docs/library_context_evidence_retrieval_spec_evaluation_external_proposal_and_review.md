# Library-context: evidence-preserving retrieval with specification-driven evaluation

**Design status:** Proposed update, October 6, 2026.  
**Reviewed revision:** `ad68af7002cdabe86e15ca9fdf863d8b773135b5` (`main` when inspected).  
**Scope:** Focused static review of the current compiler/retrieval, native publication/search, serving, and evaluation owners. No repository tests, live inference, operator-store changes, or benchmark campaign were executed for this review.  
**Supersedes:** The PostgreSQL placement details in the earlier contextual-retrieval design, and the agent-first evaluation emphasis in the subsequent SurrealDB retrieval proposal. It preserves contextual windows, exact attribution, one primary encoder, native hybrid search, typed graph expansion, and connected evidence packets.

## 1. Decision

Implement **evidence-preserving retrieval**, with an independent **specification-driven evaluation loop**. Embeddings nominate useful places in the corpus; typed relationships connect applicable evidence; selection and presentation preserve the distinctions needed to answer the request. Evaluation measures whether those distinctions and sufficient evidence survive, rather than asking a model or human to rate the response.

The primary engineering objective is **valid evidence sufficiency at a stated budget**, subject to attribution, scope, qualification, and outcome correctness. Nearest-neighbor recall, document relevance, neural scores, and downstream agent success remain separate measurements. None is interchangeable with evidence sufficiency.

This is one product architecture with replaceable numerical scorers. It does not require model fine-tuning, generated relevance judgments, a new canonical store, or a universal program verifier. A pretrained encoder or reranker may have been trained with supervision upstream; the proposed evaluation and optimization process does not depend on new human/LLM judgments.

Three boundaries must remain separate:

1. **Production semantic admission:** existing necessary checks on typed records, ownership, qualifications, references, and admitted content.
2. **Production retrieval:** indexed candidate discovery, qualified graph expansion, scoring, and bounded packet delivery.
3. **Offline evaluation:** independent expected outcomes, reference computations, metamorphic tests, countermodels, ablations, and failure reduction.

Do not move offline exhaustive search, expected answers, fixture markers, or benchmark witness sets into ordinary requests or publication.

## 2. Current codebase: retained improvements and remaining changes

### 2.1 Retain the new ownership and execution structure

The architecture remains a store-free compiler with SurrealDB realization and native Rust serving. In the reviewed source, `cpg-core/src/retrieval.rs::produce` prepares small frame metadata and processes actual evidence roots or briefs individually. It renders, realizes vectors, checks, and publishes a grain before continuing. Its `realize` path calls `service.prepare(...)` before consuming individual results. The stage declares the complete output families once, including empty families. Preserve those execution decisions. [R1, R2]

`lctx-publisher/src/search.rs` now externally orders/deduplicates shared search documents and vector values, decodes vectors at their shared grain, streams occurrence construction, and uses the same lowering for private construction and read-only cold reconciliation. This is a meaningful change from the earlier publisher. Preserve bounded batches, terminal stream checks, and exact immutable winners. [R3]

The latest status describes remediation implementation as integrated, with final targeted acceptance still in progress. Some detached transport/frontier cases and final native acceptance remain active or not run. A source-level improvement is not a current whole-suite qualification receipt. This design does not restart the user-stopped compiler campaign or assume operator adoption has occurred. [R1]

### 2.2 Remaining representation and retrieval gaps

| Current owner | Observed state | Required change |
|---|---|---|
| `domain/retrieval/mod.rs` | Renderer v2; 4,096-byte default fragments; member/release subjects; no dedicated option/invocation/source-region origins | Extend unit/view identities and introduce contextual windows and applicable bindings |
| `domain/retrieval/build.rs` | Member source view still reads the artifact of `m.access` | Follow actual defining declarations; separate exposure and implementation |
| `lctx-publisher/src/search.rs` | Streaming nested loops still combine each fragment with each parent member and anchor | Lower exact window bindings and provenance sets; streaming alone does not correct attribution |
| `lctx-serving/src/search.rs` | Raw document/vector cap 100 before target grouping; HNSW `<|100,200|>`; vector shape 1,024 | Distinct-target/context-aware recall tiers; parameterized representation |
| `lctx-surrealdb/src/materialization.rs` | Native vector schema/index fixed at 1,024 dimensions | Separate full embedding values from index projections |
| `lctx-serving/tests/native_search.rs` | Actual scoped query test with constructed vectors and added nonmatching background documents to keep lexical scores positive | Preserve this seam test; add zero-score discovery, crowding, real-publication attribution, and live-encoder quality tests |
| `validation-and-evaluation.md` | Agent comparison remains primary first-product criterion; separate partly manual behavioral assessment and legacy brief gold | Make programmatic information contracts the primary development loop; preserve gold/heldout and reserve agent comparison for external usefulness validation |

These are static observations, not measured frequency or performance claims. [R3–R9]

The native lexical test is correctly scoped to eligible positive-score search. Its background documents should not be removed just to overload its purpose. Add a separate product test for ubiquitous terms: a literal match can have BM25 score zero. The new route must separate matching from weighting and exact identifier lookup from BM25. [R8, E2]

## 3. Architecture and data flow

```text
PRODUCTION CONTENT
Pinned sources / captured documents / examples
    -> attributed facts and qualified analyses
    -> evidence units + exact parts + contextual windows
    -> window-to-target bindings and required interpretation context
    -> full embedding values + derived ANN projections
    -> immutable SurrealDB search realization

PRODUCTION QUERY
Request scope + query + explicit selection / evidence needs
    -> exact identifiers + lexical matches/BM25 + dense candidates
    -> target-and-context aggregation / bounded widening
    -> typed graph expansion and missing-role completion
    -> full-vector rescoring and family-normalized fusion
    -> optional connected-packet neural reranking
    -> context-closed evidence bundle selection
    -> actual structured/text response + exact expansion references

OFFLINE EVALUATION
Independent fixture specifications / scoped reference facts / natural pairs
    -> evaluation task + expected outcome / witness alternatives
    -> invoke the unchanged production query interface
    -> inspect returned bytes, structured fields, references and stage observations
    -> sufficiency, attribution, numerical losses, counterexamples, cost curves
    -> minimized regressions and structural changes
```

The evaluator must never expose its expected target IDs or missing-witness inventory to the running query. Controlled diagnostic injections are allowed only as separately labeled experiments.

## 4. Production semantic contracts

Names in this section are proposed logical contracts, not existing Rust types or an instruction to create one physical table per concept. Prefer extending the existing retrieval and serving vocabulary; use compact nested fields where independent graph traversal adds no value.

### 4.1 RetrievalUnit

A unit is an independently addressable coherent subject in an admitted input/context. Extend `Origin` to distinguish:

- Public-member overview.
- Actual callable/invocation/signature-variant contract.
- Contextual option: source parameter, effective parameter, or configuration field.
- Defining source declaration and bounded source region.
- Authored document section and release-scoped deployment material.
- Scenario and focused scenario region.
- Qualified analytical or relationship view with its actual result/assertion owner.

A public overview is not a synthetic union of its variants. A defining-source unit can be internal and unassociated with a public member. A conceptual document can exist without any API association. A relation-focused view must identify its endpoints, supported composition, applicability and qualification, not an arbitrary walk.

Do not fabricate cross-release stable identity. Preserve canonical identities where the current model establishes them, and use explicit correspondence for comparisons across changed fixtures or releases.

### 4.2 ContentPart and interpretation context

A part holds either an original-source reference, a deterministic projection of canonical records, or a fixed renderer literal. It has an ordered content role and a primary-versus-context role.

Examples of interpretation dependencies are a parameter's owning variant, a source block's enclosing condition, a scenario's expected-failure intent, a default's factory status, or an analysis conclusion's premise set.

Store these relationships compactly. Reuse existing support/qualification and source owners; do not duplicate the entire proof graph in retrieval. Source bytes remain with the original artifact store.

### 4.3 SearchWindow

A window is the actual search payload. Its logical fields include:

```text
unit / view identity
render and partition specification
payload content identity
ordered primary segments
ordered context segments
rendered-range -> original-span or canonical-projection mapping
required interpretation dependencies
applicable window bindings
tokenizer identity / exact encoder-input token count
disposition: full view | contextual excerpt | explicitly partial lexical-only
```

The text need not be one contiguous original slice. Its mapping must explain the assembled text honestly. A synthetic header does not shift original source coordinates. A rendered effective signature points to its actual native records rather than a fabricated source span.

One payload can be shared across contextual occurrences only when its complete rendered text and encoder inputs match. Shared payload identity must not merge release, provenance, applicability, or permission scope.

### 4.4 WindowBinding

Replace broad parent membership as the source of ranking occurrences. A binding states why a given window's **primary material** can nominate a particular target.

Retain target, relevant invocation/configuration context, upstream association basis, exact evidence locator or explicit whole-view applicability, and association status. Original provenance is a set or ordered mapping, not another Cartesian-product dimension.

An import included only as setup does not become a primary API match. A window discussing API A does not nominate API B elsewhere on the page. A callable helper does not automatically acquire the public wrapper's contract.

Exact associations generate normal operation votes. Candidate/ambiguous associations remain available as labeled evidence-navigation suggestions; they cannot masquerade as established support. Evaluate their discovery recall separately rather than treating ambiguous candidates as definitively irrelevant.

### 4.5 EvidenceDemand versus EvaluationTask

Production may accept optional typed evidence needs alongside existing query and selection fields:

```text
goal: discovery | declaration | implementation | comparison | investigation
focus: public target(s), option(s), or evidence scope, when supplied
selection: existing typed requirements
desired roles: contract, configuration, use, deployment, explanation, source
budget: response size and permitted bounded expansion
```

The calling agent can supply these. For text-only requests, use a broad route default; do not silently translate natural-language negation into a strict predicate.

An evaluation task additionally contains expected answers, sufficient witness alternatives, known forbidden inferences, and oracle completeness. Those fields are **offline-only**. The product must not depend on their availability.

### 4.6 PacketEvidenceMap and delivery state

Extend packet construction so each delivered field/span points to its exact premises and qualifications. The map is task-neutral. Do not persist a benchmark-specific assertion that a packet is sufficient.

Distinguish:

- Material exposed in the current response.
- A reference that is expandable through an existing route.
- Material omitted by budget.
- Material not requested or unavailable.
- Analysis that cannot establish the requested conclusion.
- Contradictory observations or complete-domain negative evidence.

A returned ID is not the same as delivering its source text. An opaque condition hash is not a readable condition unless its definition is present or the task contract explicitly permits a follow-up.

The evaluator checks the final serialized response and its map together. Metadata saying “condition retained” cannot rescue a response whose actual displayed condition was lost.

## 5. What to embed

### API and options

Always expose the public universe independent of brief selection. Render readable names, exact roles, ordered signatures, defaults, invocation forms, qualified types, and associated authored descriptions. Unknown descriptions are omitted, not guessed.

Use independently meaningful option views for configuration fields, documented options, declared choice domains, or relationship-bearing controls. Ordinary small parameters can remain in their contract windows. Do not generate a vector for every primitive fact merely because it exists.

### Source

Build defining-source roots independently of the public catalog, then connect them through actual exposure/entity/declaration relationships. Re-exports keep access provenance separate. Use declaration/statement boundaries and retain enclosing conditions. Keep module-level registration/global material separately searchable without assigning it to every sibling.

Tests/examples primarily belong to scenarios. Avoid generating another source-family vote merely because a scenario was parsed as Python.

### Documents and deployment

Use authored section hierarchy, title/breadcrumb context, table headings, warning subjects and configuration scope. These must enter the payload if the encoder is expected to use them. Preserve standalone conceptual discovery.

Deployment units distinguish declared metadata, documented setup, and actually checked environments. Missing external runtime checks are not a successful deployment claim.

### Scenarios

Retain coherent usage, exact associations, setup references, receiver state qualifications, and positive/negative/skip/unknown intent. A focused excerpt remains linked to its parent. Do not label a disconnected subset runnable or repeat every fixture body in every window.

### Analytical and relationship views

Render a selected finite set of existing conclusions: option-to-field/reader, registration-to-invocation, producer-to-consumer, exposure-to-implementation, qualified control outcomes, and documentary warnings. Keep complete applicability and support references, without copying every premise into dense text.

Use deterministic templates. Do not infer a relation from vector similarity, community membership, or a common type. Do not feed downstream enriched retrieval views back into the analytical embeddings that helped generate those views.

## 6. Window and encoder policy

### 6.1 Partitioning

Initial preferred window size: approximately 1,024 total encoder-input tokens. Keep 2,048 as the normal hard limit. An explicitly admitted exceptional policy up to 4,096 can accommodate indivisible contexts, but it is a new configuration—not silent use beyond the current service contract.

Partition by semantic structure, not fixed bytes. Required context counts against the limit. Never trim an unknown status, enclosing predicate, or signature role merely to fit. Oversized unresolved units remain addressable with explicit lexical-only/partial search availability and complete original expansion.

Use locally acquired, immutable tokenizer assets with matching normalization, pretokenization, postprocessing, and special tokens. Count the complete rendered input, not a sum of independently tokenized parts. Qualify parity with the inference endpoint and disable silent endpoint truncation. The Rust `tokenizers` crate provides the tokenizer pipeline; it should be a narrow adapter, not a second semantic parser. [E3]

The retrieval root stays the semantic construction grain, but inference can batch a bounded number of finalized windows across grains. Preserve backpressure and output ownership; do not materialize the whole corpus to gain batch size.

### 6.2 Full values and index projections

Retain Qwen3-Embedding-8B as the initial encoder while changing output storage to full normalized 4,096-dimensional values. Qwen documents native 4,096 output, Matryoshka support, and query-side instructions. This is a supported representation choice, not a claim of present model superiority. [R10, E1]

Derive a normalized 1,024-prefix value for the initial ANN index:

```text
full = normalize(encoder(text))
ann  = normalize(full[0:1024])
```

Validate nonzero finite norms, dimensions, projection policy, and matching query/document transforms. Rescore the candidate union at full resolution. Full rescoring cannot recover candidates missed by every first-stage route.

Separate `EncoderSpec`, `DocumentRenderSpec`, `QuerySpec`, `VectorProjectionSpec`, and search/ranking policy identities. Preserve complete request provenance, while reusing document vectors when their actual encoder inputs are unchanged. A query-instruction change does not intrinsically require document re-inference.

The current 1,024-only values cannot be expanded back to full values. Build a new admitted specification and values. Do not mutate a selected snapshot or reuse old receipt bytes under the new dimension.

### 6.3 Numerical references

Keep an offline exact full-vector scorer and exact projected-vector scorer over the same scoped values. Separate dimensional reduction from ANN approximation and grouping/truncation losses. Numeric exactness is not semantic relevance.

Use tolerance/tie-aware checks for numeric backends. Preserve the exact admitted winning bytes rather than requiring repeated inference to be bitwise identical across devices.

## 7. Retrieval and packet selection

### 7.1 Candidate routes

Execute three complementary routes: exact identifiers, lexical match/BM25, and dense retrieval. Original identifier spellings and exact option/configuration keys need their own typed route. Identifier components can aid lexical recall without destroying exact identity.

Do not conflate BM25 zero with “no match.” SurrealDB explicitly documents this case. Keep positive-score fusion policy where useful and a bounded match/exact fallback; do not give every zero-scoring document an arbitrary relevance bonus. [R7, R8, E2]

Use finite recall tiers instead of one raw top-100 cap. A starting diagnostic sequence is 128/256/512/1,024 candidate documents or values. The served default is an engineering setting to compare, not a promised optimum. Widen based on distinct applicable targets and unsatisfied requested evidence roles, under a common request budget.

Aggregate by target and relevant context before final result limits. Do not combine different signature/configuration contexts merely because they share a member. Keep candidate enumeration, ranking, and full-domain find distinct. Early stopping in approximate discovery never establishes absence or exact global top-K.

### 7.2 Native physical layout

Retain shared full embedding values. Use a separate ANN projection with the smallest useful typed scalar scope fields, such as encoder/projection, evidence family, and capture bucket. Do not duplicate vectors per source anchor or per member when the same value and scope suffice.

Whether one ANN entry per value or per value/family/capture bucket is preferable depends on access shape. Declare the physical policy and test it; keep semantic bindings separate so this choice does not change evidence ownership.

SurrealDB supports predicate-aware ANN and index-derived prefiltering, but correlated graph predicates are not automatically a bitmap allow-list. Inspect the actual engine/realization plan. Use explicit exact scoring of small eligible sets when that is the simpler operation. [E4]

Parameterize declared dimensions and thresholds from the installed search specification. Never accept an arbitrary request dimension against an incompatible existing index.

### 7.3 Typed graph expansion

Use named semantic routes: public exposure to definition, option to initializer/read/consumer, registered callback to invocation, API to scenario/setup, producer to consumer, and deployment control to associated setup.

Each returned relation retains applicability and support. Physical record/participant hops are not semantic hop counts. Batch complete useful expansions and reuse existing compact projections for selected kernels. Do not hydrate the entire graph for a small search, add a universal path engine, or enumerate all evidence paths.

Expansion can introduce a new candidate or complete evidence for an existing one. Preserve which happened. Candidate relationships can guide navigation without upgrading proof status.

### 7.4 Neural scoring is subordinate to evidence validity

Retain family-normalized fusion as the baseline. A connected-packet cross-encoder is an implementation option in the quality path, not an evaluator. Qwen3-Reranker-4B remains a concrete compatible starting candidate; its final use/size should be decided by programmatic sufficiency/cost comparisons. [E5]

A packet scorer receives applicable contracts, focused evidence, relations and qualifications—not incompatible snippets concatenated to increase apparent coverage. It cannot override requested selection groups, resolve contradictions, establish absence, or supply missing witnesses.

If neural scoring fails, apply a consistent declared fallback across the affected pool. Do not compare raw logits to BM25 or cosine values as if they were calibrated. No model rating is used as evaluation truth.

### 7.5 Pack coherent bundles, not individual snippets

First form bounded evidence bundles with their required interpretation context. Then select bundles using request-role coverage, nonredundancy, relevance, and actual rendered cost. Protect mandatory identity/qualification/signature sections.

A pair of fragments may be useful only jointly. Therefore, a greedy algorithm over individual snippets can fail even when each item's separate marginal coverage is zero. Select at the bundle level and explicitly retain AND dependencies. This is a heuristic packing algorithm, not a claim of globally optimal set cover or a submodularity guarantee.

At runtime, completeness is only relative to explicit modeled requirements and known evidence roles. Never claim general natural-language answer completeness solely because a packet template is full. Offline task oracles perform the independent sufficiency judgment.

## 8. Evaluation design

### 8.1 Separate inputs at the process boundary

The evaluation runner stores two objects:

```text
request.json         # exactly what the production tool receives
oracle.json          # private to the independent evaluator
```

The oracle contains task variables, scoped expected outcomes, alternative sufficient evidence patterns, applicability constraints, forbidden inferences, and its completeness/basis. The retrieval service never reads it.

A fixture-level example can refer to symbolic roles such as `client_constructor`, `per_call_override`, and `selection_condition`. An independent mapping resolves those roles to original source anchors and public identities. If production extraction misses the role, record an extraction failure; do not discard it from the denominator.

Do not give exact target IDs to a purported natural-language discovery test unless the task itself names those targets. Report structured-query, text-only discovery, and composed evidence-delivery lanes separately.

### 8.2 Oracle bases

**Constructive fixtures.** A small semantic fixture specification emits source/docs/examples and expected declarations/relationships/outcomes through separate implementations. Use bounded supported constructs and intentional unknown boundaries. Maintain independently written seed cases or runtime observations so shared generator mistakes can be challenged.

**Reference-over-admitted-facts.** Simple scans/joins establish expected results relative to the admitted corpus. This measures retrieval/serving fidelity, not extraction truth. Do not invoke production classification/search to obtain its own oracle.

**Natural cross-view pairs.** Explicit documentation-to-API and scenario-to-use relationships establish known positives. They do not establish exhaustive negatives. Do not calculate ordinary precision by marking every unlinked result irrelevant. Exclude trivial copied-text retrieval from the semantic-transfer stratum.

**Generated executable cases.** Reuse the repository's generated-program observation boundary. Execute only isolated generated programs in these lanes. Do not start importing analyzed packages or running existing `fixtures/python/` contrary to the retained policy. A runtime observation is scoped, not universal. [R9]

Oracle status is explicit: valid/scoped, unsupported, timeout, inconsistent, or incomplete. A broken oracle does not become a failed retrieval or a free pass. Report unresolved evaluation cases as a separate stratum.

### 8.3 Independent witness language

Use a finite AND/OR witness expression with typed bindings and local context constraints:

```text
Exists context C:
  All(
    declaration_matches(target, C),
    option_owner_and_default_matches(option, C),
    Any(
      qualified_relationship_and_premises(C),
      original_source_condition_and_environment(C)
    ),
    applicable_original_usage_with_required_context(C)
  )
```

The evaluator matches semantic roles, source locations, structured values, and admitted alternatives—not one producer-chosen proof ID. Independence is lost if the expected witness formula is reconstructed from the candidate's own declared supports without an external reference basis.

For positive-answer tasks:

`Sufficient(t, P) = some admissible sufficient witness set is delivered by P, with compatible scope/context and no disallowed claimed inference.`

Represent alternatives as a DAG/circuit and evaluate lazily. Do not enumerate all minimal witness sets. Sharing a vocabulary is reasonable; sharing the production decision algorithm under test is not independent verification.

For contradicted, unresolved, and conflicting tasks, check the required epistemic outcome and its basis separately. “Unresolved” can be correct for one task and a failure to provide available information for another. A system that refuses every answer must not score well simply because it never lies.

### 8.4 Delivered versus expandable sufficiency

`E_delivered(P)` includes only information actually exposed in the final response with interpretation context. `E_expandable(P, budget)` additionally includes what an allowed scripted traversal can retrieve through actual expansion tools within the stated total budget.

Report both. A source ID alone can count as a navigation success but not immediate source delivery. Follow-on errors, foreign handles, omitted qualifications, or expired continuations must be exercised, not assumed away.

A script's shortest successful expansion path measures interface possibility. It does not prove that an agent will choose that path. Later usability studies address that separate question.

### 8.5 Bounded countermodels

For selected semantic fixture families, define a finite hypothesis class and the observable information in the packet. Require a satisfiable observable model first. Then search for two admissible models consistent with those observations but giving different task answers.

For example, compare:

```python
value = default if override is None else override
```

and:

```python
value = override or default
```

Both can share the same signatures. With `default=10, override=0`, their answers differ. A packet that retains only signatures cannot distinguish them. A source predicate or a suitable qualified result can.

An opaque source hash or canonical ID is not a semantic observation that permits the oracle to infer which behavior was hidden. The countermodel uses only delivered content under the task's allowed interpretation rules.

Two consistent models demonstrate insufficiency. An exhaustive absence of countermodels establishes determinacy only in that finite class. Solver `unknown` or timeout is inconclusive. Conflicting documentary statements are provenance-qualified observations, not axioms combined through classical explosion.

Begin with enumeration for small finite cases. Z3 is an optional offline adapter when constraints or minimum-cost witness selection warrant it. Do not introduce solver calls into production. [E6]

### 8.6 Information cost and baselines

For small tasks, compute the least-cost sufficient packet over a declared finite evidence inventory and renderer. Include mandatory context, response wrappers, and shared-content deduplication. Token counts refer to the final serialized response using a declared reference tokenizer; bytes remain a model-independent secondary measure.

If exact optimization uses additive token estimates, label it approximate unless verified against the actual rendering. For large cases, report a feasible reference, lower bound, or both—not a fictional global optimum.

Keep four distinct references:

- A source/fixture information oracle.
- An admitted-corpus information oracle.
- Exact numerical retrieval at full and projected dimensions.
- Best feasible packet construction within an observed candidate pool.

This separates source coverage, extraction, retrieval, and packing failures.

## 9. Measurements and experiment design

### 9.1 Primary measurements

Report task-family and oracle-basis strata, not one weighted score:

- Correct outcome and valid attribution/qualification rate.
- Immediate evidence sufficiency at response budget B.
- Expandable evidence sufficiency at total budget and tool-call limit.
- Sufficient-witness-set presence in each candidate stage.
- Missing-information obligations and unsupported claims, separately.
- Delivered size, redundancy, native-query cost, inference work and latency.
- Explicit unscorable/oracle-limited tasks and genuinely unanswerable tasks.

Known-positive natural pairs produce recovery measurements, not exhaustive relevance precision. Large unknown corpora must not acquire fabricated negative labels.

### 9.2 Numerical ablations

Compare:

1. Full 4,096 exact scoring against 1,024/2,048 exact scoring: representation reduction.
2. Projected exact scoring against projected HNSW: index approximation.
3. Same numerical candidate stream under different raw caps and target grouping: crowding.
4. Same candidates with and without graph expansion: connection/completion.
5. Same connected packets with baseline and neural scoring: reranking.
6. Same ranked evidence with alternative bundle selectors: presentation loss.

Use identical eligibility and metric definitions. Account for duplicate vectors, ties and floating-point tolerance. Exact cosine rankings are not semantic gold.

### 9.3 Metamorphic laws versus robustness experiments

Hard laws include source-attribution preservation, qualification survival, correct scope changes, no extra vote for exact duplicate payload occurrences under the declared dedup policy, and correct withdrawal of support when a necessary premise is removed.

Do not infer contradiction just because support disappeared. Do not require exact score/rank invariance under corpus additions, identifier renaming, arbitrary paraphrases, or ANN rebuilds. Those are robustness measurements unless equivalence is independently established.

Useful transformations include re-exports, sibling distractors, shifted source coordinates, split/merged document sections, overload option moves, fixture-only imports, deleted supports, new conflicts, and foreign release insertions. Record preconditions for each transformation.

### 9.4 Stage loss and interventions

An opt-in evaluation observer records compact identities at representation, channel retrieval, aggregation, expansion, rescoring, reranking and delivery boundaries. Score against actual output, not the observer alone. Trace truncation is explicitly incomplete.

The first stage where a witness set disappears is a diagnostic, not automatically the causal source. Some stages intentionally introduce new evidence. Use controlled interventions: inject an omitted known candidate, substitute exact retrieval, retain all valid candidates through a scorer, or use a reference bundle selector.

Each intervention is labeled and does not count as ordinary product success. Reduce the fixture, query and evidence while retaining the failure. Property-based shrinking and delta debugging can supply small reproducible cases; minimality is relative to permitted reductions. [E7, E8]

### 9.5 Evaluate the evaluator

Inject domain faults that preserve basic schema validity: swap sibling source anchors, drop a condition but keep its ID, merge overloads, remove required setup, mislabel negative examples, or change release attribution. The semantic checker should reject them.

Distinguish rejected invalid mutants, equivalent mutants, surviving semantic mutants, and oracle-inconclusive cases. A syntax error is not evidence that a semantic information test is effective.

### 9.6 Splits and optimization

Separate development and confirmation by fixture composition, query grammar, library/release and near-duplicate source families. Hold out structural combinations, not just random seeds. Macro-average across declared strata; thousands of variations of one template are correlated, not independent evidence of broad quality.

Freeze oracle definitions and comparison populations before a configuration comparison. Pick Pareto-efficient settings subject to hard correctness constraints. A model/ranker switch must earn its useful-information and cost trade-off; no supervised judge is needed.

Keep current gold and heldout material unchanged and unexposed to tuning. Do not convert undocumented or unlabeled alternatives to negatives to make optimization easier.

## 10. Serving observability without production accounting machinery

Always retain the minimal provenance and availability needed for truthful answers. Enable detailed stage observations only in evaluation/debug runs, using a bounded sink. Do not persist all candidate matrices, every graph edge traversal, or full proof closures for routine queries.

Use one request identity tying snapshot realization, query text/vector, selection, rendering, expansion, ranking, and fallback policy together. A cursor must not silently switch numerical channels or rerun into a different ordering. Either return the ranked packet in one response or bind continuations to a bounded search-session result/query realization.

An evaluator may use local JSONL or an existing analytical substrate for reports. It does not need a second mutable production graph or new online telemetry database. The type and parameter boundaries exist to localize losses, not to require a proof packet for every physical step.

## 11. Source-level integration map

| Owner | Proposed responsibility |
|---|---|
| `lctx-model/domain/catalog/evidence/` | Independent source and association roots; exact applicability premises |
| `lctx-model/domain/retrieval/` | Extended origins/subjects, contextual parts/windows/bindings, interpretation dependencies, renderer/partition specs |
| `cpg-core/src/retrieval_preparation.rs` and `retrieval.rs` | Owner-scoped input selection, per-root/window construction, bounded batches, once-per-stage output declarations |
| Existing embedding owners / `lctx-embed` | Full output spec, local tokenization parity, request identities and immutable value reuse |
| `lctx-surrealdb/src/materialization.rs` | Parameterized search layout, full-value/ANN projections, typed filter keys and exact identifier/match route |
| `lctx-publisher/src/search.rs` | Stream exact bindings, not parent cross-products; preserve externally ordered terminal reconciliation |
| `lctx-serving/src/search.rs` and `operations.rs` | Explicit candidate policy, context-preserving grouping, bounded widening, typed expansion, scorer interface |
| Packet/evidence/serving schema owners | Actual delivered evidence maps, interpretation closure, optional evidence demand and exact expansion selectors |
| Proposed offline `lctx-eval` package or existing evaluation-tool equivalent | Task specs, independent references, witness matching, stage observers, numerical baselines, countermodels, reducers |
| `docs/design/sections/validation-and-evaluation.md`, product §14.12, synthesis/serving owner | New primary development evaluation protocol and clear relation to later Context7/agent studies |

For every new canonical record, update all applicable vocabulary, graph lowering, neutral detached import, artifact transport, publication and restore inventories. The latest remediation found missing companion transport records; do not repeat that defect with evaluation-friendly retrieval records. Add a focused new-record roundtrip/omission control instead of replaying every existing compiler stage. [R1]

The evaluation package may share the public types but should not import production classification as its expected-answer engine. No release code should read evaluation fixtures or oracle data.

## 12. Recommended dependency-ordered delivery

### Slice A: contracts and attribution

Implement contextual windows, exact bindings, independent defining-source roots and final packet mappings. Preserve owner-scoped streaming. Deliver independent fixtures for siblings, re-exports, overloads, candidate-only associations, setup-only imports and missing qualification in the same slice.

### Slice B: numerical and native retrieval

Add full-vector storage/projections, exact numerical references, code-aware match/exact routing and candidate tiers. Deliver ubiquitous-term, scoped eligible-set, crowding, projection parity and real publication/restore checks with the feature.

### Slice C: connected information delivery

Add named relation views, typed expansion, evidence needs and context-closed bundle selection. Deliver configuration, callback, handoff, deployment and contradiction/unknown information contracts with them.

### Slice D: independent evaluation and scorer comparisons

Run the fixed-corpus sufficiency ladder, finite countermodels and minimal-packet references on selected task families. Integrate connected-packet reranking through the scorer boundary and compare it programmatically. Keep a no-reranker reference path and a clear disabled/degraded state.

### Slice E: broadening and external validation

Expand real-library cross-view tasks, unseen structural combinations and model/index/scorer challengers. Use later agent tasks to challenge task coverage and usability, not as the routine quality judge. Translate each informative failure into a missing distinction, independent oracle or diagnostic case.

Do not require every countermodel family or optional scorer to be implemented before any useful measurement. Ship feature-specific tests and the central sufficiency evaluator early. The end state is ambitious; the dependency order prevents the evaluation system from becoming the next large blocking architecture.

## 13. Acceptance semantics

Hard semantic failures are never traded for a higher aggregate relevance score. Unavailable inference, resource refusal, incomplete oracle coverage, actual unknown behavior and unsupported rendered claims are distinct outcomes.

Before comparative runs, declare task populations, evidence budgets, required strata and non-regression limits. This design intentionally does not invent numerical task-success thresholds without a baseline. Report paired results and uncertainty, especially for ANN builds and related fixture families.

The immediate target is an architecture that can answer: **what useful information was available, what reached each stage, what was delivered, and what minimal distinction was missing when a task failed?** That is a much stronger foundation for improving embeddings and translating later real-world feedback than an opaque relevance score.

## References

### Repository sources at the reviewed revision

- **R1.** [STATUS.md](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/STATUS.md).
- **R2.** [Compiler retrieval owner](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/cpg-core/src/retrieval.rs).
- **R3.** [Native search publication and reconciliation](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/lctx-publisher/src/search.rs).
- **R4.** [Retrieval model](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/lctx-model/src/domain/retrieval/mod.rs).
- **R5.** [Retrieval construction](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/lctx-model/src/domain/retrieval/build.rs).
- **R6.** [Native materialization definitions](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/lctx-surrealdb/src/materialization.rs).
- **R7.** [Native candidate queries](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/lctx-serving/src/search.rs) and [operation orchestration](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/lctx-serving/src/operations.rs).
- **R8.** [Native search tests](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/crates/lctx-serving/tests/native_search.rs).
- **R9.** [Validation and evaluation owner](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/docs/design/sections/validation-and-evaluation.md).
- **R10.** [Current Qwen specification](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/specs/embedding/qwen3-embedding-8b.json).
- **R11.** [Architecture map](https://github.com/paul-heyse/library-context/blob/ad68af7002cdabe86e15ca9fdf863d8b773135b5/docs/design/README.md).

### Primary external references checked for this update

- **E1.** [Qwen3-Embedding-8B official model card](https://huggingface.co/Qwen/Qwen3-Embedding-8B).
- **E2.** [SurrealDB full-text scoring and zero-score matching](https://surrealdb.com/docs/learn/data-models/full-text-search/scoring-and-ranking).
- **E3.** [Hugging Face Rust tokenizers](https://docs.rs/tokenizers/latest/tokenizers/).
- **E4.** [SurrealDB similarity search and predicate/index filtering](https://surrealdb.com/docs/learn/data-models/vector-search/similarity-search).
- **E5.** [Qwen3-Reranker-4B official model card](https://huggingface.co/Qwen/Qwen3-Reranker-4B).
- **E6.** [Z3 optimization guide](https://microsoft.github.io/z3guide/docs/optimization/intro/).
- **E7.** [Proptest introduction](https://proptest-rs.github.io/proptest/intro.html).
- **E8.** [The Debugging Book: reducing failure-inducing inputs](https://www.debuggingbook.org/html/DeltaDebugger.html).


# Evidence system recommendation

**Yes. For library-context, I would make specification-driven, programmatic evaluation the primary development loop, with agent studies serving as later validation of practical usefulness.** My earlier recommendations put too much emphasis on agent task completion as the main quality signal.

Your system is particularly well suited to a more formal approach because the information being retrieved already has explicit identities, relationships, contexts, qualifications, and original-source references. The codebase we reviewed separates those semantic contracts from native search and presentation. That gives you considerably more to evaluate than similarity between a query and an unstructured document. 

The central shift should be:

> **Evaluate whether the system preserves and delivers the information required to resolve a specified question—not whether another model likes the answer.**

There is one useful terminology distinction: **you do not need manually labeled answers, but you do need a basis for determining what should be true.** That basis can come from executable specifications, independently generated fixtures, exact computations, and source-grounded relationships. This is automated testing and evaluation, not supervised model training. Those same results could later become programmatic supervision, but training is not a prerequisite.

I would build the framework around the following methods.

## 1. Define query-specific information contracts

The unit of evaluation should not be a reference paragraph or a list of “relevant documents.” It should be an **information contract** describing what the returned material must establish.

For example, these are different contracts:

**Discovery:** Find at least one API whose admitted invocation context declares a specified option.

**Enumeration:** Return every API satisfying a predicate within a specified, complete domain.

**Implementation support:** Return an applicable signature, configuration scope, and sufficient original usage context.

**Behavioral investigation:** Return the source or qualified analysis needed to distinguish between specified behaviors.

**Uncertainty:** Explain that a conclusion is unresolved, with the relevant boundary, rather than treating missing evidence as absence.

A conceptual test specification would contain:

```text
InformationTask
    scope:
        snapshot, library/release, admitted contexts

    question:
        typed goal, requirements, requested evidence basis

    required_information:
        identities, contracts, relationships, source/context obligations

    expected_outcome:
        reference-supported / contradicted / unresolved / conflicting

    oracle:
        construction method, authority, completeness boundary

    query_presentations:
        structured request and independently rendered text variants
```

These are evaluation contracts, not a proposal to create another semantic authority. Reuse your existing identities and predicate meanings. Add only the test-specific question, oracle basis, and required-information specification.

### Why this matters

Consider:

> “What happens when I supply a per-call timeout of zero instead of using the client’s default?”

A response containing both timeout parameters might have excellent API recall and still be insufficient. It needs information distinguishing:

```python
effective = default if override is None else override
```

from:

```python
effective = override or default
```

The names, signatures, and most surrounding code could be identical. The decisive information is the selection rule.

That distinction is programmatically testable in a generated fixture. It does not require an LLM to judge whether a passage “seems helpful.”

**This is the level at which I would formalize information quality: which task-relevant distinctions survive retrieval and presentation?**

## 2. Generate reference cases without manual relevance labels

I would use several complementary oracle sources. Their independence and limitations should remain explicit.

### A. Exact queries over admitted facts

Generate tasks from the typed model and solve them with a simple reference evaluator over the relevant complete domain.

Examples include finding APIs with a declared parameter, identifying configuration fields with a specified declared domain, locating resolved callback registrations, or comparing source and effective invocation forms.

For supported predicates, this produces expected result sets, expected classifications, and supporting identities mechanically.

Use a straightforward scan or small reference implementation rather than invoking the production search or selection path and calling its result “ground truth.”

There is an important qualification:

**This evaluates retrieval relative to the admitted facts. It does not independently establish that extraction produced the right facts.**

An extractor that misses a parameter could otherwise cause both the search system and its generated oracle to agree on the same wrong answer.

### B. Independently generated miniature libraries

Create a small declarative fixture language that can emit:

1. Python source, documentation, examples, and configuration artifacts.
2. Expected declarations, relationships, and scoped outcomes.

The second output comes from the fixture specification—not by rereading the production compiler’s output.

A fixture could express two overloads with different options, a re-export, an unresolved wrapper, a configuration field consumed by one method but not another, or a positive example adjacent to an expected-failure test.

Then vary their combinations, naming, layout, and surrounding distractors.

This is a natural application of property-based testing: generate structured inputs, check their properties, and shrink failures to small reproductions. Proptest provides this generation-and-shrinking model for Rust. :chatgpt-content-reference{index="1"}

The fixture language need not model all Python. It should cover the distinctions your product promises to represent. Generated cases should include both supported constructs and deliberate boundaries where the correct outcome is unresolved.

**Keep some independent source-level fixtures and external observations as well.** A fixture emitter and its expected-result generator can share a mistake; generating both from one specification reduces labeling cost but does not eliminate oracle risk.

### C. Naturally paired evidence from real libraries

Real corpora contain useful associations that do not require new annotations: documentation attached to a declaration, explicit cross-references, resolved calls in examples, and known source-to-public-exposure mappings.

These support cross-view retrieval probes such as:

- Use an authored description to find its associated API.
- Use an option explanation to find the exact option and owner.
- Use an example’s stated purpose to retrieve the relevant scenario and calls.
- Use a qualified relationship description to retrieve its endpoints and original evidence.

There is precedent for obtaining query-like text mechanically from function documentation: CodeSearchNet distinguishes these automatically obtained documentation/function pairs from its separately expert-labeled challenge. That distinction is worth preserving in your evaluation. :chatgpt-content-reference{index="2"}

These pairs are **known associations**, not automatically exhaustive relevance labels. An unlinked API may still be relevant. Therefore, evaluate recovery of known positives without silently treating every other result as wrong.

Also separate easy lexical recovery from semantic transfer. Retrieving the same documentation sentence that was used verbatim as the query tests indexing and availability more than language-to-code understanding. A stronger cross-view probe excludes that copied text from the candidate representation being tested.

### D. Executable behavioral observations

For bounded tasks, generate ordinary programs and check their outputs without an agent.

In the timeout example, independently defined cases for `None`, zero, and a nonzero override distinguish the two rules. Other families can cover argument binding, default handling, callback invocation, context-manager usage, and configuration precedence.

Keep execution isolated and environment-pinned. A concrete passing test establishes its observed behavior, not a universal property.

Together, these oracle sources cover different failure classes:

**Fact-query oracles test retrieval against the model; fixture oracles test the model against independent specifications; natural pairs test real-corpus transfer; executable observations test selected behavior.**

## 3. Make evidence sufficiency the primary quality metric

Ordinary document recall is not enough for your architecture. The returned items may each be relevant but fail to form a usable answer.

### Represent sufficient evidence as alternatives and dependencies

For a task \(t\), let \(\mathcal W_t\) describe the admissible sufficient witness sets.

A task might be answerable through an exact contract plus an explicit documentation statement, or through an applicable source region plus a qualified analytical result. These are alternatives—not a requirement to retrieve every possible supporting document.

For a packet \(P\), let \(E(P)\) be the evidence actually exposed in that packet. For a positive-answer task:

\[
\operatorname{Sufficient}(t,P)
=
\mathbf{1}\left[
\exists W\in\mathcal W_t:
W\subseteq E(P)
\ \land\
\operatorname{Compatible}(W,t)
\right]
\]

Compatibility includes release, invocation variant, receiver/configuration context, evidence basis, and required qualification.

A signature from one overload and an option from another do not form a valid joint witness merely because both belong to the same member.

This AND/OR structure is closely related to database provenance: conjunction records jointly required premises, while disjunction records alternative derivations. Provenance semantics provides a principled foundation for representing how conclusions depend on evidence. :chatgpt-content-reference{index="3"}

For implementation, use a compact witness circuit or dependency graph. Do not enumerate every minimal witness set across the corpus.

### Evaluate epistemic outcomes separately

The positive-support formula is not the whole evaluator.

A **contradicted** result needs the applicable negative basis or complete-domain evidence. An **unresolved** result needs the correct boundary and must not conceal available positive information. A **conflicting** result needs the relevant incompatible observations rather than an arbitrary winning source.

In particular:

**Removing support generally establishes loss of support—not proof of the opposite.**

Your evaluator should check these states explicitly, using the same declared meanings but independently constructed expected cases.

### Count delivered information, not just reachable IDs

A packet does not contain sufficient evidence merely because it includes an ID that could eventually lead to it.

Measure two separate capabilities:

**Immediate sufficiency:** The current response contains the necessary material and qualifications.

**Expansion sufficiency:** The response exposes a valid, bounded route to obtain the missing material through the available tools.

The latter can be checked with a scripted traversal over explicit expansion references. Its shortest successful route measures what is possible through the interface, not how easily an agent will discover that route.

Also check the rendered output itself: a valid support reference does not compensate for dropping “under this assumption,” omitting an enclosing condition, or displaying the source signature as an effective signature.

### Use sufficiency-at-budget, not one composite score

The main quality curve should be:

> **What fraction of answerable tasks receive a valid sufficient packet within a given response or retrieval budget?**

Report that alongside task-requirement coverage, invalid attribution, qualification loss, and unresolved/conflicting outcome accuracy.

Measure both corpus-level sufficiency and sufficiency conditional on the candidate pool. This separates “the information exists but was missed” from “the candidates were adequate but packing discarded the crucial piece.”

Do not let a system obtain a perfect safety score by refusing everything. Report unsupported claims and successful information delivery separately, with answerable and genuinely unresolved cases stratified.

## 4. Measure whether the returned information determines the answer

Witness completeness can be strengthened with a more formal question:

> **Could two different task-relevant situations both be consistent with the returned information, yet require different answers?**

This is related to *view-based query determinacy*: whether information retained in a view suffices to answer a query. Database research studies this explicitly, including graph-query settings. Your practical implementation would be a bounded, task-specific version rather than a universal determinacy engine. :chatgpt-content-reference{index="4"}

### A bounded countermodel test

Define a finite model class \(\mathcal H_t\) for a task family. Search for:

\[
H_1,H_2\in\mathcal H_t
\]

such that both satisfy the information exposed by \(P\), but:

\[
\operatorname{Answer}_t(H_1)
\ne
\operatorname{Answer}_t(H_2)
\]

Finding that pair produces a concrete insufficiency witness.

In the timeout example, a packet containing only the two signatures can be compatible with both the `is None` and truthiness-based selection rules. Those implementations disagree for zero. The evaluator can report:

```text
Information loss:
    override-selection condition absent

Distinguishing input:
    default = 10
    override = 0

Remaining alternatives:
    result = 0
    result = 10
```

That is much more actionable than a low relevance score.

When no countermodel exists, the conclusion applies only to the declared bounded model class. Conflicting documentary observations should remain observations with provenance, not be asserted as contradictory logical axioms that make everything derivable.

### Minimum sufficient information cost

For small cases, also calculate the cheapest admissible sufficient packet:

\[
B_t^*
=
\min_{S:\operatorname{Sufficient}(t,S)}
\operatorname{Cost}(S)
\]

Cost can be rendered tokens, bytes, or a declared combination of response size and expansion work. Required interpretation context must be included.

This lets you distinguish an inherently expensive task from poor evidence selection. It also detects a system that achieves completeness only by returning nearly everything.

Use exhaustive search or a constraint solver for small reference cases; Z3 supports satisfiability and optimization, including lexicographic and Pareto objectives. :chatgpt-content-reference{index="5"}

For large cases, use feasible reference solutions or explicit bounds. Do not call a heuristic result the global minimum. Likewise, summing independently tokenized pieces is only an approximation to the cost of the final rendered packet.

**I would prioritize this bounded counterexample approach over generic “information density” or embedding entropy scores. It tests whether something consequential is missing.**

## 5. Use metamorphic and counterfactual testing extensively

Often you cannot cheaply compute the ideal ranking, but you can specify how the system should respond to a controlled change.

This is the purpose of metamorphic testing. It has also been applied specifically to false vector matching in retrieval-augmented systems, as in MeTMaP. I would adopt the testing principle, not assume its NLP transformations automatically preserve your code-query semantics. :chatgpt-content-reference{index="6"}

For your system, the most useful transformations are domain-specific:

| Transformation | Property to evaluate |
|---|---|
| Duplicate the same evidence occurrence | No additional independent support or family vote. |
| Move a definition behind a supported re-export | Public identity and defining-source attribution remain distinct and correct. |
| Add a similarly named sibling API | No transfer of options, source anchors, or behavioral evidence between siblings. |
| Split a document into different windows | Required provenance and qualifications remain recoverable; measure retrieval sensitivity separately. |
| Move an option between overloads | Joint-support results change appropriately; member-level aggregation cannot hide incompatibility. |
| Delete a necessary premise | Dependent support is withdrawn or downgraded; absence does not automatically become contradiction. |
| Add conflicting evidence in the same applicable context | Conflict is retained rather than resolved by retrieval score. |
| Change the selected release | Results and evidence remain within the selected scope. |

### Distinguish semantic laws from ranking preferences

Not every sensible expectation is a hard invariant.

Reordering source records should not change semantic ownership. Rebuilding an approximate index may change the returned ranking. Adding unrelated documents can change BM25 statistics. Renaming identifiers can affect a pretrained encoder even when a controlled program transformation preserves behavior.

Therefore, classify each transformation as:

**A required semantic invariant**, **an expected outcome change**, or **a robustness experiment**.

Do not fail the system because cosine scores are not perfectly invariant to an identifier rename. Do fail it if the rename causes evidence from an unrelated scope to be presented as authoritative.

The broader methodological precedent is axiomatic information retrieval: express desirable behavior as explicit constraints and use violations to diagnose retrieval models. Your constraints should be tailored to evidence identity, scope, and sufficiency rather than copied wholesale from traditional term-weighting axioms. :chatgpt-content-reference{index="7"}

### Generate query variants without an LLM judge

For a typed task, independently render several query forms: an exact identifier request, an option-focused question, a relationship-focused question, or a controlled descriptive formulation.

Keep the generation grammar separate from the indexed-text renderer. Otherwise, the benchmark can reward matching its own templates.

Only claim semantic equivalence for transformations whose meaning is established by construction. Automatically deleting a word such as “not,” changing “may” to “must,” or substituting an approximate synonym cannot receive an unchanged oracle casually.

Hold out formulation families and combinations, not merely random instances.

## 6. Separate numerical retrieval quality from semantic information quality

This is particularly important for the multi-resolution embedding architecture proposed earlier.

You can formally isolate several sources of loss without any human judgments.

### Dimensional reduction

Compare full-resolution exact search with reduced-resolution exact search over the same eligible payloads.

This measures what is lost through the representation projection.

### Approximate indexing

Compare HNSW with exact search using the **same dimensionality, metric, eligibility, and vector values**.

This measures index approximation rather than encoder quality. Account explicitly for ties and numerical tolerance.

### Candidate truncation and aggregation

Compare candidate retention before and after raw-window caps, target/context grouping, duplicate suppression, and family fusion.

Evaluate recall of distinct task targets and sufficient evidence sets—not only vector IDs. Retrieving many windows for one target can look excellent at the vector level while starving other required targets.

This directly addresses the fixed pre-aggregation candidate limits found in the native search implementation we reviewed. 

### Reranking and packet selection

Given a fixed candidate pool, evaluate whether reranking and packing retain or discard the admissible witness sets.

A reranker can increase conventional document relevance while removing the only passage that establishes configuration scope. The evidence-contract evaluator should catch that.

**Exact vector search is a numerical reference, not a semantic oracle.** An embedding can reproduce its exact nearest neighbors perfectly and still retrieve the wrong information. Numerical fidelity and task sufficiency belong on separate axes.

## 7. Use genuinely reference-free signals—but as diagnostics

There are useful signals available even for queries without a formal task oracle.

These include retrieval-score dispersion, lexical/dense disagreement, sensitivity to controlled perturbations, dominance by a few high-frequency targets, duplicate concentration, and instability under modest budget increases.

The established area is *query performance prediction*. Methods such as clarity, weighted information gain, and normalized query commitment estimate retrieval quality without per-query relevance judgments, using query, corpus, and result statistics. Their effectiveness depends on the retrieval setting and score distributions. :chatgpt-content-reference{index="9"}

For library-context, I would add graph-specific diagnostics:

**Unresolved-evidence frontier:** Which required-looking relationships are still missing after expansion?

**Context fragmentation:** Are highly ranked pieces attached to incompatible invocation or configuration contexts?

**Association concentration:** Does one ambiguous passage nominate many targets?

**Budget instability:** Does a small increase reveal an entirely different candidate family?

These can identify cases for deeper automatic testing and potentially guide retrieval widening.

But they should not be the main optimization target.

A tight embedding cluster can contain the wrong APIs. Two retrievers can agree because they share the same attribution defect. A high score gap can reflect a duplicated passage. Better isotropy or lower hubness does not, by itself, establish better answers.

**Use these signals to decide where to investigate—not as substitutes for evidence sufficiency.**

## 8. Turn failures into structural diagnoses

The evaluator should preserve enough intermediate identities to locate where information was lost:

```text
original evidence
    → admitted facts and associations
    → rendered windows
    → channel candidates
    → expanded evidence
    → reranked candidates
    → delivered packet
```

At each boundary, ask whether at least one admissible sufficient witness set remains available.

This produces useful classifications:

| Observed failure | Likely boundary to investigate |
|---|---|
| Required source exists but the admitted fact/association is absent | Extraction or normalization |
| Fact exists but no searchable window preserves its meaning | Unit construction, rendering, or partitioning |
| Valid window exists but no retrieval channel selects it | Encoder, lexical analysis, filtering, or candidate budget |
| Endpoints are found but their applicable relationship is absent | Graph expansion or relationship modeling |
| Adequate candidates exist but reranking demotes the decisive evidence | Reranker input or scoring |
| Adequate evidence survives ranking but disappears from the response | Packet selection or output budgeting |
| Evidence is present but assigned to the wrong target/context | Binding or attribution |
| Missing evidence becomes a negative claim | Completeness or classification semantics |

These are localization hypotheses, not automatic proof of causation.

Strengthen them with controlled interventions: inject the known relevant candidate, replace ANN with exact scoring, preserve all sufficient candidates through reranking, or supply a reference packet selection. Run the remaining stages unchanged.

This answers questions such as:

> “Would better embeddings fix this failure, or would the packet builder still discard the answer?”

### Shrink failures

After detecting a failure, remove unrelated APIs, documents, relationships, and query clauses while preserving the failing property.

Delta debugging is specifically designed to reduce failure-inducing inputs. Its usual minimality guarantee is relative to the allowed reductions, not necessarily a globally smallest reproduction. :chatgpt-content-reference{index="10"}

For your architecture, a useful result might be:

> Two members, one shared module, two source regions, and one incorrect window binding are sufficient to reproduce the error.

That is a direct architectural finding, not an invitation to tune the embedding model blindly.

### Test the evaluator itself

Deliberately introduce meaningful faults: swap source anchors, remove a condition, accept a foreign release, merge incompatible variants, or treat an unresolved relationship as supported.

The evaluator should detect them.

Use domain mutations for these information defects. A tool such as cargo-mutants can separately identify Rust code mutations that survive the ordinary tests. :chatgpt-content-reference{index="11"}

Report equivalent, invalid, and unclassified mutants separately. Do not inflate a mutation score by counting syntax failures as successful detection of semantic defects.

## 9. Optimize the architecture without training a judge

Once the above exists, you can compare renderers, window policies, embedding dimensions, lexical configurations, candidate budgets, graph routes, rerankers, and packet selectors using entirely programmatic outcomes.

I would avoid a single weighted “quality score.”

Instead, use:

**Hard correctness constraints** for attribution, scope, qualification, and outcome semantics.

**Sufficiency and information-retention curves** for quality at different budgets.

**Measured cost curves** for inference, search, transfer, response size, and execution.

Then select Pareto-efficient configurations: those for which no alternative is better on the relevant quality/cost dimensions without a trade-off.

Freeze the oracle semantics and case-generation specification while optimizing. Otherwise, it is easy to “improve” by weakening the required information.

Also preserve separate results by library, task family, relationship type, source modality, ambiguity state, and naming/structural difficulty. Thousands of generated variations of one easy pattern are not thousands of independent demonstrations of generality.

Hold out libraries or releases where feasible, fixture compositions, document origins, and query-rendering families. Keep near-duplicate documentation and generated siblings within the same split.

**A configuration sweep is still an experiment. It does not require a supervised scoring model, but it can still overfit its development workload.**

## 10. How I would integrate this into your project

I would add a focused offline evaluation package under the existing evaluation tooling, with four responsibilities:

**Task generation:** Construct information contracts and query variants from fixture specifications and qualified corpus relationships.

**Reference evaluation:** Compute scoped expected outcomes and witness requirements using simple independent operations.

**Production execution:** Run the actual pinned retrieval/serving implementation and capture intermediate identities.

**Diagnosis and reduction:** Produce failure classifications, controlled ablations, and minimized reproductions.

These do not need to become four services or four new persistent subsystems.

The evaluator should consume the structured packets and window provenance already proposed. It should not reconstruct semantics from arbitrary generated prose, and it should not use the production reranker as its judge.

The essential independence rule is:

> **Share identities, schemas, and declared meanings where appropriate; do not share the exact decision algorithm under test and then call agreement independent validation.**

Run against immutable snapshots. Use private fixture realizations for corpus mutations. Cache unchanged source preparation and vector values. A packing-policy experiment should not re-extract the library; an ANN experiment should not regenerate embeddings.

Large witness enumeration, bounded countermodel search, runtime executions, and mutation campaigns belong in targeted offline runs—not in every request or every publication.

The first implementation I would prioritize is the combination of **generated information contracts, witness-sufficiency checking, and stage-by-stage loss localization**. Add metamorphic generators alongside it. Those foundations make the more advanced countermodel and minimum-packet analyses useful rather than disconnected research tools.

## 11. How later agent feedback becomes more valuable

With this framework, real-world feedback becomes an input to the task model—not a permanent stream of subjective labels.

Suppose an agent fails because it cannot tell whether a setting is global or per-call.

Instead of recording “answer quality: poor,” the process becomes:

**Identify the missing distinction → express the information obligation → determine its evidence basis → generate related cases → localize the failure → repair the responsible structure.**

That could produce a general regression family spanning constructor defaults, configuration objects, invocation overrides, ambiguous wrappers, and version changes.

Sometimes the agent will need a distinction your current model does not express. That is a **task/model coverage gap**, not necessarily a bad embedding. Preserve it as such rather than force it into an existing metric.

Similarly, a programmatically sufficient packet may still be difficult for an agent to use. That feedback concerns presentation, navigation, or workload realism. It can motivate a new information-contract or usability test without undermining the formal results already established.

There is a boundary to keep honest: these methods establish quality over declared tasks, evidence bases, and model classes. They do not by themselves establish the intended meaning of every unconstrained natural-language request. The role of later real-world testing is to challenge that coverage and interface—not to serve as the only way you can improve retrieval.

**My revised recommendation is therefore to make this an evidence-preservation and sufficiency evaluation system first, an embedding benchmark second, and an agent benchmark third.** It would give you a strong programmatic basis for architectural refinement now, while turning later supervised feedback into precise, reusable structural improvements rather than isolated preferences.

# Library recommendations

**Yes. The strongest opportunity is to use more of the capabilities already in your stack, then add a small number of narrowly scoped tools.** I would not introduce a large retrieval or evaluation framework.

There is also an important correction to my earlier framing: **Z3 is already declared in your isolated inference environment.** `services/vllm/pyproject.toml` includes `z3-solver==5.1.0.0` alongside a Z3-specific TileLang build. That does not establish an evaluation integration, but it means Z3 is not an entirely new dependency family for the project. 

I checked the manifests and relevant implementation paths against **`ad68af7`**, which still resolved as `main`. This was a focused capability audit, not an exhaustive search of every call site or an installation test. “Underleveraged” below means *underleveraged for the retrieval and information-quality objectives we have defined*, not necessarily inadequately used for its present responsibility. 

## 1. The library decisions I would make

| Library or capability | Current position | Recommendation |
|---|---|---|
| **Z3** | Declared in the isolated inference environment | Add an explicit offline evaluation adapter for countermodels, tracked assumptions, consequences, and minimum sufficient evidence. |
| **`biodivine-lib-bdd` + `fixedbitset`** | Already part of the semantic model | Extend their use to bounded evidence-sufficiency expressions, interpretation dependencies, and witness-set operations. |
| **Hypothesis** | Already used in generated CPython checks | Add stateful tests, targeted generation, structured fixture strategies, and failure reduction. |
| **SurrealDB** | Already the native serving/search engine | Use its code-oriented analyzers, analyzer inspection, selective vector paths, and query-plan diagnostics more deliberately. |
| **vLLM + Qwen + PyTorch** | Existing isolated inference stack | Reuse for full-resolution embeddings, proper reranker serving, candidate-set late interaction, and numerical reference calculations. |
| **Rust `tokenizers`** | Not declared in the root/model manifests inspected | Add as a narrow native tokenization adapter. |
| **Proptest + cargo-mutants** | Proptest is discussed in design; not declared in the root/model manifests inspected | Add native property testing where it avoids Python/transport overhead, plus targeted mutation campaigns. |
| **Optuna + `ir_measures`** | Proposed evaluation additions | Use for configuration experiments and standard retrieval diagnostics—not as the semantic evaluator. |
| **ACTS, Kani, OR-Tools CP-SAT** | Conditional additions | Use for constrained interaction coverage, bounded Rust verification, and larger discrete packing problems respectively. |

The sections below explain which specific features are valuable, what they would do, and where their boundaries should be.

## 2. Z3: use considerably more than `check_sat`

**Z3 is the most valuable formal-evaluation integration to add, but not as a general replacement for your existing semantic operations.**

I would expose it through a small offline worker. The native Rust `z3` crate is a reasonable interface for a Rust evaluation owner; its current API exposes incremental scopes, assumptions, tracked assertions, unsatisfiable cores, consequence extraction, and model access. A Python `z3-solver` worker is also viable alongside Hypothesis. Choose one main constraint encoder rather than maintaining parallel Rust and Python interpretations. :chatgpt-content-reference{index="2"}

Do not run evaluation through the vLLM service just because that environment already declares Z3. Give the evaluation worker explicit dependencies and qualify its binding/native-library compatibility separately.

### Incremental solving and assumption literals

Many evaluations share the same fixture semantics and differ only in which evidence was delivered.

Load the fixture’s bounded model once. Give each delivered evidence item an activation literal, and test different packets by changing assumptions. Z3 supports both assertion scopes and assumption-based checks; these avoid reconstructing the entire logical problem for every closely related experiment. They are not a guarantee that every incremental workload will be faster. :chatgpt-content-reference{index="3"}

For your evaluator, this supports:

**Packet ablations:** Does the answer remain determined after removing the option-scope explanation?

**Budget curves:** At what delivered-information budget does the task become resolvable?

**Failure minimization:** Which pieces can be removed while preserving the same insufficiency?

This is a better use of a solver than issuing unrelated, fully reconstructed queries for every window.

### Countermodels: identify the missing distinction

The most important application is the two-model test we discussed:

\[
\operatorname{Consistent}(H_1,P)
\land
\operatorname{Consistent}(H_2,P)
\land
\operatorname{Answer}(H_1)\ne\operatorname{Answer}(H_2)
\]

Here, \(P\) is the information actually delivered, and \(H_1,H_2\) belong to a declared bounded hypothesis class.

For the default-versus-override example, the solver can find that the delivered packet permits both a `None`-specific fallback and a truthiness-based fallback, then produce zero as a distinguishing input.

That turns “insufficient answer” into:

> The packet omits the condition distinguishing an explicit zero override from an absent override.

Z3’s model-generation facilities supply concrete assignments for satisfiable constraints. The correctness of this diagnostic still depends on your encoding of the task and delivered information. :chatgpt-content-reference{index="4"}

### Tracked assertions and unsatisfiable cores

Attach evidence identities to tracked assertions.

For a consistency test, a core can identify evidence constraints that cannot all hold together. For an implication test, a core from “background + delivered evidence + negated conclusion” can identify a subset sufficient to rule out the alternative.

**Do not call an unsatisfiable core the uniquely minimal explanation.** The default core need not be minimal, and even a subset-minimal core need not have minimum cardinality or minimum token cost. Z3 provides core minimization options and more elaborate core/correction-set procedures, but these are separate operations. :chatgpt-content-reference{index="5"}

In this project, cores should explain the *formal model’s* conflict or implication. They do not automatically identify the original software defect or establish which conflicting source is correct.

### Consequence extraction

Z3 can extract consequences—facts forced by the current assumptions. That is useful for asking:

> Which requested distinctions are already determined by this packet, and which remain open?

For example, the packet may determine the declared parameter type and default while leaving invocation scope unresolved.

This can be more economical than repeatedly asking the same family of independent yes/no questions, although its suitability depends on the chosen encoding. :chatgpt-content-reference{index="6"}

### `Optimize`: minimum sufficient evidence and diagnostic examples

Use optimization for small offline reference cases:

\[
\min \sum_i c_i x_i
\]

subject to selecting a sufficient, context-compatible evidence set and retaining all necessary interpretation dependencies.

Z3 supports optimization over logical constraints, weighted soft constraints, and multiple-objective policies. :chatgpt-content-reference{index="7"}

Two particularly useful objectives are:

**Minimum sufficient packet:** Establish a reference for whether the production packer wastes response budget.

**Minimum distinguishing example:** Prefer a counterexample with fewer configuration choices, smaller values, or fewer differing facts.

The token objective needs care. A sum of separately measured piece costs is only a surrogate when tokenization depends on concatenation. Either define independently framed pieces with an explicit cost contract or verify the final serialized packet and label the optimization accordingly.

### Boundaries I would impose

Keep `sat`, `unsat`, `unknown`, timeout, inconsistent oracle, and unsupported encoding distinct. In particular, avoid convenience interfaces that collapse several of those cases into “no model.” The Rust binding documents such collapsed-return helpers alongside explicit solver-status APIs. :chatgpt-content-reference{index="8"}

Use the appropriate theory: unbounded integers for modeled Python integer arithmetic, bit-vectors for genuinely fixed-width operations, and floating-point theory where IEEE behavior matters. A real-number abstraction is not automatically equivalent to Python floats. Z3 exposes these distinct theories. :chatgpt-content-reference{index="9"}

I would **not initially use Fixedpoint/CHC solving to rebuild your whole Python behavioral analyzer**. It is a potential independent reference for a selected recursive relation, not a prerequisite for evaluating retrieval.

## 3. Your BDD library can support the evidence algebra

You already use `biodivine-lib-bdd` substantially. The condition kernel has bounded Boolean operations, restriction, substitution, nominal atom identities, and persisted-node reconstruction. This is not an unused library waiting to be adopted.  

The opportunity is a **new application of its existing capabilities**: representing sufficient evidence alternatives.

For example, a task-specific sufficiency expression might be:

\[
S = K \land \left(D \lor (R \land Q)\right) \land U
\]

where \(K\) means the applicable contract is delivered, \(D\) means an explicit documentary rule is delivered, \(R\) means the relevant implementation region is delivered, \(Q\) means its required qualification/context is delivered, and \(U\) means applicable usage information is delivered.

This gives you compact AND/OR alternatives without enumerating every witness set.

### Features worth using

The library exposes restriction, existential and universal projection, satisfying valuations, exact cardinality, and `necessary_clause`, which identifies literals common to all satisfying assignments. :chatgpt-content-reference{index="12"}

Applied to evaluation:

**Restriction** evaluates sufficiency under a delivered packet.

**Existential projection** asks whether some permitted completion of missing evidence would suffice.

**Universal projection** asks whether a result is robust across all remaining modeled choices.

**Necessary literals** identify evidence-presence conditions required by every successful configuration.

**Valuation generation** produces bounded evidence-removal and ambiguity cases.

Keep these distinctions precise. Exact cardinality counts satisfying assignments in the chosen variable universe—not minimal witness sets, probabilities of real-world success, or independent test scenarios.

### Do not reuse runtime-condition identities for evidence presence

“Evidence item is included” is not the same proposition as “runtime predicate is true.”

Use a separate evaluation-variable domain and a thin adapter to the BDD library. Do not repurpose `EvaluationAtom`, relax the current condition kernel’s limits globally, or make Boolean witness bookkeeping contaminate behavioral semantics.

Start with direct circuit/bitset evaluation for straightforward contracts. Compile to BDDs when alternative combinations, repeated restriction, or quantification justify it. BDD compactness depends on the formula and ordering; it is not universally guaranteed.

**Division of labor:** BDDs handle bounded Boolean combinations efficiently; Z3 handles relationships involving values, types, arithmetic, and countermodels. Neither needs to replace the other.

## 4. Hypothesis is the most immediately underleveraged evaluation tool

Hypothesis is already a declared development dependency and is used in `test_flow_soundness.py` alongside independently executed generated programs and CPython monitoring. That existing lane is a strong starting point.  

I would expand three capabilities.

### Structured generation and shrinking

Generate small **library specifications**, not arbitrary Python strings.

The generated structure can contain declarations, options, overloads, re-exports, documentation associations, and scenarios. Separate emitters produce source artifacts and expected information contracts.

Make shrinking preserve dependencies: when an API is removed, references and expectations must either be removed coherently or retained as an intentional unresolved case. Otherwise, the reducer finds malformed fixtures rather than useful counterexamples.

Keep independent seed fixtures or concrete runtime observations to challenge mistakes shared by the specification and its emitters.

### Stateful testing

Hypothesis’s state-machine facilities generate sequences with reusable values and preconditions. :chatgpt-content-reference{index="15"}

For library-context, useful sequences include:

> Search → select an operation → expand its evidence → request another page → narrow scope → repeat under disabled vector availability.

Check that snapshot identity, context, evidence availability, and continuation semantics remain coherent throughout.

This tests **whether information can actually be obtained through the interface**, rather than merely whether a single function returns the expected record.

Run these against disposable fixture realizations, not mutable operator state.

### Targeted generation

`hypothesis.target()` guides generation using a numerical observation. It is useful for searching for difficult inputs rather than drawing only from a fixed random distribution. :chatgpt-content-reference{index="16"}

Good target signals include:

- The gap between exact and approximate distinct-target recall.
- The number of windows consumed by one distractor API.
- The number of interpretation dependencies lost by packing.
- The token-cost gap between a production packet and a sufficient reference.

Use actual semantic assertions to decide pass/fail. The numerical target is only a way to seek hard cases.

Also keep a frozen, separately sampled evaluation suite. Targeted generation intentionally changes the case distribution, so its failure frequency is not a prevalence estimate.

### Proptest is useful, but not a reason to duplicate Hypothesis

One correction to previous wording: the root and `lctx-model` manifests I inspected do **not** declare Proptest, even though the design documentation mentions it. Hypothesis is the clearly active property-testing dependency in the paths inspected.  

Add Proptest as a dev dependency for high-frequency tests of native Rust structures: window bindings, provenance maps, witness circuits, packet selection, and codecs. It supports compositional per-value generation and shrinking. :chatgpt-content-reference{index="19"}

Use Hypothesis for generated Python and interface journeys; use Proptest for pure Rust kernels. Do not reproduce every test in both.

## 5. Extract more value from the existing retrieval stack

### SurrealDB: code-aware analysis and selective search

The current native analyzer is `TOKENIZERS class FILTERS lowercase`. SurrealDB also provides a `camel` tokenizer and `search::analyze`, making it possible to inspect and test the exact tokenization of code-oriented names.  :chatgpt-content-reference{index="21"}

I would add analyzer-level regression cases for qualified paths, snake_case, camelCase, acronyms, configuration keys, and non-ASCII identifiers. Preserve exact spellings in a separate lookup representation; do not assume aggressive normalization preserves identity.

The other underleveraged feature is **plan-aware selective vector retrieval**. SurrealDB distinguishes predicates checked during ANN traversal from index-derived prefilters and documents exact-distance handling for small allowed sets. Your correlated occurrence predicate should not be assumed to receive every prefilter optimization automatically. :chatgpt-content-reference{index="22"}

Use native query-plan inspection to choose between broad HNSW and exact scoring over a small eligible set. This is execution-policy selection, not a new database or graph framework.

### vLLM: reuse its scoring and token-embedding interfaces

Current vLLM documentation provides scoring support for cross-encoders, specific Qwen3 reranker loading, and late-interaction scoring for supported token-embedding models. :chatgpt-content-reference{index="23"}

Therefore:

**Use vLLM for the connected-packet reranker before adding another inference server.**

**Use its supported token-embedding/scoring path for a late-interaction experiment before building a custom neural execution stack.**

Qualify these against your pinned service build. Current documentation does not prove that every capability is available or identical in that custom build.

Also, loading a generic Qwen causal model and reading arbitrary logits is not the same operation as loading the qualified reranker. Preserve model-specific preprocessing, output interpretation, and scorer identity.

### Qwen: keep full values and derive compact search representations

The existing model supports native 4,096-dimensional embeddings and Matryoshka reduction. Your current product specification returns only 1,024 dimensions.  :chatgpt-content-reference{index="25"}

Use that capability as proposed: infer and retain full values once, derive normalized lower-dimensional projections, and compare exact full, exact reduced, and ANN results separately.

This is a representation improvement using the existing model—not a reason to introduce another encoder immediately.

### PyTorch: numerical references without another vector engine

The isolated service already declares PyTorch. 

Use blocked matrix operations over stored normalized vectors for the offline exhaustive reference. Qwen’s own examples calculate similarities through tensor matrix multiplication. :chatgpt-content-reference{index="27"}

Configure reference precision deliberately, handle ties and tolerances, and use a higher-precision small-case reference where necessary. “Exact” here means exhaustive candidate evaluation, not exact real-number arithmetic.

I would not add FAISS merely to obtain an exact baseline that the existing numerical stack can calculate. Reconsider it only when the reference workload warrants a specialized implementation.

## 6. Reuse the parser, graph, and analytical libraries for better test structure

### Ruff, Markdown, and Unicode handling

Your manifest already includes the Ruff parsing/AST/code-generation family, `markdown`, and `unicode-segmentation`. These are the appropriate foundation for syntax-aware source regions, documentation hierarchy, and source-aware boundaries. 

I would not add Tree-sitter or another Markdown parser solely for retrieval chunking.

Use existing syntax and document structure to form windows, but retain original bytes as evidence. Regenerated code is useful for constructing fixtures; it must not silently replace original-source coordinates.

**Add Rust `tokenizers` for the missing complementary operation.** Syntax boundaries and Unicode sentence boundaries do not establish model-token budgets. Hugging Face’s Rust tokenizer implements normalization, pretokenization, model tokenization, and postprocessing, including special tokens. :chatgpt-content-reference{index="29"}

Load immutable assets locally. Count the complete encoded input. Preserve the distinction between token offsets in rendered text and coordinates in original source. Do not turn truncation on and allow it to silently erase conditions or qualifications.

### Petgraph: structural evaluation, not just traversal

Petgraph already offers graph matching, dominators, reachability, shortest paths, SCCs, and DAG operations. :chatgpt-content-reference{index="30"}

The most useful additional applications are:

**Fixture motif matching:** Verify that generated and admitted graphs preserve a registration/invocation or configuration/consumer structure.

**Structural deduplication:** Detect when a thousand fixtures are mostly renamed copies of one topology.

**Missing-context diagnostics:** On an appropriate control-flow projection, identify enclosing structure that a source excerpt cannot discard safely.

Use matching only on small, role-labelled fixtures or bounded projections. For parallel assertions, preserve their identities through an appropriate incidence representation rather than assuming endpoint adjacency captures everything.

And do not confuse graph connectivity with evidence sufficiency: an AND/OR witness circuit has stronger semantics than an ordinary path.

### FCA and fixed bitsets: evaluation coverage

You already have bounded FCA/RCA machinery and independent finite-context checks using `fcars` and development-only `odis`. The documentation explicitly keeps those oracle roles separate from production semantics. 

Reuse this for **evaluation stratification**.

Construct a test-factor context with attributes such as overload ambiguity, re-exporting, inherited access, missing documentation, partial analysis, expected failure, and configuration scope. Inspect which combinations your generated corpus actually covers.

This helps identify whole missing classes of cases rather than just increasing fixture count.

Do not interpret implications observed in a finite test corpus as universal library behavior. Likewise, do not replace the existing FCA kernel merely because a different package exposes more enumeration modes; its current boundary is deliberate.

### Arrow/DataFusion: batch evaluation joins

Use your existing columnar stack for expected-versus-actual comparisons, missing-witness anti-joins, per-stage attrition, and stratified metrics. DataFusion supports these relational operations and exposes execution plans and metrics. :chatgpt-content-reference{index="32"}

The reference queries should be simple and independently authored. Calling the production selection function through SQL does not make it an independent oracle.

No additional analytical database is required for these operations.

## 7. New tools I would actually add

### Optuna: programmatic architecture experiments

Optuna can search a configuration space using supplied objective values; it does not require human relevance labels or model training. Its samplers include grid, random, TPE, and multi-objective approaches. :chatgpt-content-reference{index="33"}

Use it to vary window budgets, candidate tiers, projection dimensions, graph expansion limits, reranker pool size, and packet-selection settings.

The objective comes from your evaluator: sufficient-information delivery, invalid claims, latency, and token cost.

Keep correctness requirements as explicit constraints. Do not let the optimizer trade wrong-release evidence for a better aggregate score. Freeze task meanings and oracle definitions while experimenting.

Start with a reproducible grid or random sweep to establish behavior. Introduce adaptive search after the evaluator is trusted.

Also, do not assume standard pruning works unchanged for multiple objectives; Optuna’s documentation distinguishes that support. :chatgpt-content-reference{index="34"}

### `ir_measures`: standard diagnostics without reimplementing metrics

This library provides established retrieval measures, per-query results, and diversity measures such as `alpha_nDCG`. :chatgpt-content-reference{index="35"}

Use it for secondary diagnostics: target recall, reciprocal rank, judged coverage, and diversity across evidence roles.

Generate relevance records from complete reference cases where possible. For naturally paired data, do not silently label every unpaired item irrelevant.

**It does not replace your witness-sufficiency evaluator.** Independent document relevance cannot represent “these two pieces are jointly necessary and must share the same invocation context.” Likewise, diversity across evidence roles is not proof of a valid connected answer.

### cargo-mutants: test the tests and evaluator

Run targeted mutation campaigns on binding validation, context compatibility, qualification retention, duplicate contribution handling, and the evaluator itself.

cargo-mutants identifies locations where code changes survive the test suite. :chatgpt-content-reference{index="36"}

For your domain, pair ordinary code mutation with semantic mutations: swap anchors, erase a condition, merge overload contexts, or reclassify an expected-failure example.

Keep these campaigns scoped. Rebuilding the entire analyzer/compiler workspace for every mutation would squander much of their value.

### ACTS: constrained interaction coverage

This is the most useful additional test-generation method beyond ordinary property testing.

A generated-library case has factors such as exposure form, signature structure, configuration scope, evidence completeness, scenario intent, and release alignment. An exhaustive Cartesian product quickly becomes impractical.

NIST’s ACTS tooling generates combinatorial covering arrays and supports the broader constrained interaction-testing approach. :chatgpt-content-reference{index="37"}

Use it to choose a compact set covering, for example, every valid three-way interaction among selected factors. Hypothesis can then generate concrete source variations within each selected combination.

ACTS supplies **coverage-oriented configurations**, not expected answers. Your independent fixture semantics still provide the oracle. Pairwise or three-way coverage also does not establish correctness for all higher-order interactions.

I would adopt the method prospectively and add the external tool when the fixture-factor matrix becomes substantial.

## 8. Conditional additions—not immediate requirements

### OR-Tools CP-SAT

Use CP-SAT when minimum sufficient packet selection becomes a substantial discrete optimization workload: thousands of candidate pieces, prerequisite constraints, mutual exclusions, per-role coverage, and integer budgets.

It supports integer constraint optimization and distinguishes optimal, feasible, infeasible, and unknown results. A feasible result is not an optimality certificate. :chatgpt-content-reference{index="38"}

**Start with Z3 Optimize for small reference cases.** Add CP-SAT when packing scale or performance creates a distinct need; do not maintain two equivalent optimization encoders merely for variety.

### Kani

Kani is useful for verifying the **actual Rust implementation** of small pure kernels, whereas Z3 usually checks a separately encoded semantic model. It can check assertions, panics, and arithmetic properties through proof harnesses, subject to supported features and resource limits. :chatgpt-content-reference{index="39"}

Good targets are bounded index mappings, budget calculations, provenance-range transformations, and small witness/compatibility operations.

It is not a way to prove embedding relevance or the whole distributed inference/search system correct. Its toolchain and language-support requirements also make an isolated verification package preferable to disrupting your pinned main workspace.

### Broader solver, inference, and retrieval frameworks

I would not currently add another graph database, a general RAG evaluation framework, a second inference server, or an ontology/rule engine to solve these particular problems.

Similarly, I would not add Datalog, equality saturation, or a separate SAT solver until a named operation is better expressed by that tool than by your existing typed operations, bitsets, BDDs, or Z3.

That is a scope decision, not a claim that those technologies lack useful capabilities.

## 9. What still needs to be yours

Even with all these libraries, four pieces remain domain-specific:

**The information-task vocabulary:** What distinguishes a usable declaration, implementation example, configuration explanation, or uncertainty answer?

**The exact binding and delivery model:** Which window applies to which target, and which information was actually exposed?

**The independent oracle construction:** Where expected facts and behaviors originate, and what their completeness boundary is.

**The failure-to-architecture mapping:** Whether a failure belongs to extraction, rendering, candidate recall, graph expansion, ranking, or packet selection.

No library can supply those semantics from generic retrieval scores.

The effective division of responsibility is:

> **Hypothesis/Proptest generate cases; ACTS organizes interaction coverage; BDDs evaluate bounded evidence combinations; Z3 finds value-sensitive countermodels and optimal small references; existing graph and columnar tools analyze results; `ir_measures` supplies conventional diagnostics; Optuna searches configurations; mutation testing and Kani challenge the implementation.**

For the immediate design, I would prioritize **the offline Z3 adapter, native tokenizer support, expanded Hypothesis use, and the evidence-sufficiency layer built on existing typed records and Boolean/set machinery**. Add native property tests and mutation testing alongside them, then use Optuna and retrieval metrics once those judgments are reliable.

That gives you substantially stronger formal evaluation without turning the product into a collection of overlapping reasoning engines.