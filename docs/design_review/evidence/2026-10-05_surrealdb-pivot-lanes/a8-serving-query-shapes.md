# A8: serving workload query shapes (static evidence for the SurrealDB pivot review)

Lane A8. Baseline: working tree at HEAD `f66eb15a` (2026-10-05), read-only. No compiles, tests or DB access.
Labels: **[obs]** observed in source or docs; **[int]** interpretation. No verdict.

Sources read in full or in the relevant span:
- Python: `python/lctx_mcp/src/lctx_mcp/{server,wire,generation}.py`, `python/lctx_semantics/src/lib.rs`, `python/lctx_storage/src/serving.rs:770-1146`.
- Contracts: `crates/lctx-model/src/domain/serving/{dispatch,requests,resources,cursor,mappings}.rs`, `selection/classification.rs`.
- Services: `crates/lctx-postgres/src/generations/{mod.rs:779-1143, runtime.rs:251-304, service.rs, selection.rs, catalog_service.rs, packet_service.rs, packet_reads.rs, operation_sections.rs, evidence_service.rs, capability_service.rs, flow_inventory_service.rs, source_characterization_service.rs (partial), source_usage_service.rs (scan only), access_routes_service.rs, retrieval_service.rs, vectors.rs:300-370, native_service.rs (scan), ddl.rs:355-410, serving_shape.rs}`.
- Probe: `docs/design_review/evidence/2026-10-05_surrealdb-native-realization/{README.md, journeys.py, pgref.py}`.
- Design: `sections/semantic-model.md` §15.12 (lines 926-994) and `sections/api-and-evidence-product.md` (lines 390-410).

---

## 0. Headline facts

1. **Serving has two data tiers, not one [obs].** Only the second tier is per-request one-hop chaining.
   - **Tier P (prepared at startup).** About 95 relations go into `ClassificationData` (`selection/classification.rs:33-128`), plus selection Output/Admission rows, about 28 retrieval relations and the native inventory. Each is read **whole** (`selection_read` / `visit_verified`, no `WHERE`; `selection.rs:46-118, 162-272`) and receipt-checked. The rows are held in memory as `Rows<…>` indexes. Even the largest relations are included: `source::Occurrence`, `SyntaxPlacement`, `OccurrenceOwnership` and `EntityRef` each hold about 1M rows in A1's live counts.
   - **Tier R (per request).** `GenerationLease::read_for/read_ids/visit_for` (`mod.rs:881-1006`). Each call issues one `SELECT <all declared cols> FROM g.rel WHERE field = ANY($1) ORDER BY …` (`mod.rs:1050-1088`), autocommit, with no LIMIT.
   - **Consequence [obs].** Four of the ten tools never touch tier R: `find_operations`, `compare_operations`, `browse_library` and `inspect_value_paths`. Neither does core identity resolution for `get_operation`. These are pure Rust over tier P (`catalog_service.rs:701-819, 952+`; `native_service.rs:185-207`).
2. **Tier R has 243 production hop call sites** (plus one wrapper definition at `mod.rs:940`) in 10 files. The counts match Phase A's 244. About 170 of them pass a single-element key slice `&[x]`, i.e. N+1 style lookups inside Rust loops (heuristic regex; per-file numbers in §4).
3. **Each `e.query(…)` block acquires a fresh lease** (`runtime.rs:251-293`):
   - pool connection;
   - lease protocol transaction with installation lock, digest compatibility, `pg_advisory_xact_lock_shared` and registration reads (`lease.rs:137-180`);
   - `set_config('statement_timeout')` (`runtime.rs:296-303`);
   - release.

   `ResourceLimits::query_connections = 2` (`serving/resources.rs`). Nested service calls inside loops multiply lease acquisitions:
   - `original_range` per span or anchor;
   - `capability()` per brief or per search hit.
4. **The one true recursive expansion is `explain`** (`evidence_service.rs:511-665`). It is a BFS over model-declared derivation premises: depth ≤ 8, nodes ≤ 256, edges ≤ 1024 (`resources.rs` defaults).
   - **Per depth:** for every derivation relation whose conclusion target appears in the frontier, it runs a **full unfiltered scan** of that relation (`visit_named` with no filter). It re-hashes all rows and compares the hash to the stored receipt digest (`:557-627`).
   - **Existence check:** a `count(*) … WHERE id = ANY` per relation group (`:478-509`).
   - About 137 `rule = "…"` declarations exist in `lctx-model`.
5. **The SurrealDB probe's three journeys (J1–J3) cover the shallowest tier-R shapes.** They are 3–4 set-based hops with no sum-type dispatch, no per-row semantic filter, no N+1 loop and no recursion (`journeys.py:1-15`). The deep shapes were not expressed in SurrealQL by the probe:
   - `claim_basis`;
   - `resolve_original`;
   - `explain`;
   - source characterization and usage;
   - flow inventory;
   - access-route search;
   - the nested capability and original-range calls.

---

## 1. Request path (every tool)

`SchemaTool.run` (`wire.py:151-232`) runs these steps:
1. `service.admit()` (grant);
2. `encode_request`;
3. `request_info`;
4. `admit_request_id`;
5. optional query embedding (`generation.py:27-45`);
6. `service.dispatch` (`serving.rs:774-893`);
7. `tool_result`, then `encode_envelope`, then final byte admission.

Search tools go through `RetrievalService::request → numerical callback (Python BM25S) → rank → finish`; the others go through `ServingService::dispatch` (`service.rs:52-80`).

**Inventory [obs].** There are 10 tools (`dispatch.rs` `routes!`) and one resource template, `lctx://capability/{capability}` (`dispatch.rs` `resources()`), served by `capability_resource` (`serving.rs:1022+`).

**Shared contracts [obs]:**
- `PageRequest { size (default 20, max 100), cursor, expanded }` (`requests.rs`).
- Responses are 32 KiB by default and 256 KiB expanded; final MCP envelope bytes are admitted, never truncated (`resources.rs`, `dispatch.rs` `admit_envelope`).
- 30 s request deadline, 1 s admission wait, 64 MiB request bytes.
- The cursor is `{binding, offset}`. The binding covers generation, request identity, policy, wire identity, channels, group, section, member and ordering (`cursor.rs`). A mismatch is a `Continuation` error.
- Every page is an **offset into a fully recomputed complete domain** (`catalog_service.rs:645-700`, `operation_sections.rs:144-199`).
- `SectionPage { availability: Available | Partial{reason} | Unavailable{reason} | NotRequested, items, continuation, omitted, truncated }`.
- Refusal classes: `ResourceRefused`, `Contract` (incomplete closure → `incompatible`), `Absent`, `Codec`/`Invalid` (`serving.rs:31-36`).

---

## 2. Operation table

Key:
- **Hops** = tier-R chain depth.
- **DD** = data-dependent: the next key set comes from the previous rows.
- **Expressibility** is assessed as an **(S)urrealQL / (P)ostgreSQL set SQL** pair for the *read pattern* only. Rust semantics that must stay are listed separately in §3.

| Operation | Input / output contract | Read pattern | Rust semantics between hops | S / P expressibility of the reads |
|---|---|---|---|---|
| `find_operations` | library, `Selection` (mode Strict/Discovery), page. Output: supported / unresolved / conflicting pages, each with its own cursor; extent `CompleteDomain{total}` | **None per request.** Tier P only (`catalog_service.rs:701-769`) | All of it: `candidates()` runs the model Selection classifier (outcomes Supported / Unresolved / Conflicting), plus group split and paging | n/a. Moving the classifier into either QL would create a second authority |
| `compare_operations` | library, 1–5 selectors, selection | None; tier P (`:771-819`) | Candidate classification and `resolves()` path matching; ambiguity flag | n/a |
| `browse_library` | library, scope (Library/Module/Class), view, selection, page | None; tier P (`:952+`). An unknown module or class returns `Absent` | Ownership and membership logic (`ownership`, `belongs`) | n/a |
| `inspect_value_paths` | member, analysis context, exact inputs, assumptions | None; `PreparedNative` (`native_service.rs:185-207`) | Native Entry replay, BDD conditions, refusals | n/a (pure model) |
| `search_operations` | optional library, query, selection, page. Output: results page, `Ranked{returned}`, channels, ranking | Tier P corpus. Lexical channel: Python BM25S callback over a Rust-tokenized query. Optional vector channel: **one SQL** over `lctx_cache.serving_vectors`, exact cosine for *all eligible uses*, with a bit-identity check against the in-memory copy (`vectors.rs:347-366`). No canonical hops | Eligibility filter, channel binding identity, score expansion, `RankingPolicy::rank` fusion (`retrieval_service.rs:543-645`), paging | S: kNN exists, but BM25 tokenization and fusion would be a second ranking authority, so awkward. P: natural for the vector leg (as today) |
| `search_evidence` | as above plus families | Ranking as above, then **per page hit, per anchor:** `original_range` = its own lease plus the chain in the `get_evidence` row (`retrieval_service.rs:755-779`) | As above; anchor and subject association from tier P | Chain: see `get_evidence`. Nested per-hit leases are an orchestration artifact |
| `search_capabilities` | optional library, query, page | Ranking, then **per page hit, a full `get_capability`** (`:813-836`) | As above | See `get_capability` |
| `get_capability` and resource `lctx://capability/{id}` | brief id. Output: title, rendered body, assertions (each with claim basis, supports and proofs, optional terminal question), originals | One query block (`capability_service.rs:35-58`) with this chain: Brief → BriefDocument parts (ordered, ordinals checked) → BriefAssertion (ordered) → ProgrammaticAssertion → ProgrammaticAssertionSupport → AssertionSource → BriefSource → DocumentaryConclusion → ProseSlice → ProseSource → AssertionQualification (`:402-520`), about 11 set hops, all DD. Then **per assertion** `claim_basis` (below) and **per TerminalSummary support** `terminal_question` (about 15 single-id hops, `:127-398`). Then **per original** an `original_range` lease (`:61-64`) | Ordinal-contiguity checks; sum-type dispatch on `AssertionSource` (6 arms); TerminalSummary admissibility (template id, kind, status); proof-reference assembly; claim-basis verification | Brief chain: S natural (`<~(t FIELD f)` and link fetch); P natural (CTE chain or LATERAL). Per-assertion sum dispatch: S awkward (IF chains per arm); P awkward (LEFT JOIN per arm or CASE). terminal_question: see `claim_basis` |
| `get_operation` (core) | library, selector (member id or public path), optional comparison and reference parameter, sections[], page. Output: `OperationResolution::Missing / Ambiguous{candidates} / Unique{packet}` | Resolution is tier P. `operation_cores` builds the core from tier P, then one query block (`packet_service.rs:208-218`): TypePresentation by term (set), AssertionQualification (set), then **per qualification `claim_basis`** (N+1), then TypeSupport, SignatureTypeSupport and NativeSignatureSupport by assertion (sets) and CatalogOptionEvidence (set). Then `fit_operation_response` | Typing-origin dispatch (SourceDeclared / NativeObserved); context-equality checks; "every option's evidence row exists" (`:220-230`); proof-chain assembly; indivisible required signatures; byte fitting | Set reads: S and P natural. claim_basis: below |
| `claim_basis` (shared sub-shape, `packet_reads.rs:510-682`) | qualification → `ClaimBasisPacket` | AssumptionSet → members (count checked) → Assumption definitions. **Per assumption** (loop, single-id): sum dispatch TypeConformance / NoExtraOverrides. That gives Observation → Support → `assumption_support` (Qualification → ProviderRun → AnalysisContext → Provider → ProviderSurface → Evidence → sum dispatch Invocation / Occurrence / SourceSpan → Occurrence → SourceArtifact), plus for NoExtraOverrides AssumptionUniverse → UniverseSupport → ModelCatalog → AuthoredModel. **Depth 6–14**, all DD, polymorphic | Recomputes `AssumptionSet::new(members)` and checks it equals the stored set; qualification must be context-equal, empty-assumption, `always()` condition, Definite, Exact; `AssumptionUniverseSupport::new(u, catalog, model)` parses the authored model and checks it; byte-range validity | S: expressible as nested link fetches, but the per-arm dispatch and two-level polymorphic Evidence make it awkward. P: awkward (polymorphic FKs need a LEFT JOIN per arm) but feasible as one statement. The semantic re-derivations (set identity, universe parse) cannot move into either |
| section `scenarios` | page ≤ 2 positives | Tier P filter, sort and page; then 1 set hop (ScenarioSpan by scenario). **Per association:** per span an `original_range` lease, plus a 3-hop chain DiagnosticUseTarget → Link → Assessment in its own lease, truncated to 64 with `Partial` *after reading all* (`operation_sections.rs:354-528`) | Span-role filter and ordinal sort; empty spans → Contract | S and P natural for each chain. The per-association lease is orchestration |
| section `deployment` | | 1 hop (ReleaseDeployment by release), CPU page, then **per item** `original_range` (`:530-643`) | | Natural |
| section `relationships` | ≤ 5 links | Set chain OccurrenceOwnership-keyed NormalizedCallEvent → CallPolicyAssessment → CallPolicyAdmission → NormalizedCallAlternative (4 DD hops, `:645-700`), then CPU over tier P (callables, source field links, classes) | Relationship classification from admissions plus prepared catalog | Reads natural in both |
| section `conflicts`, `callable_comparison`, `contextual_typing`, `incoming_references` | | None; tier P only (`:201-352`, `:909-1093`) | `contract_comparison::compare`, `types::contextual::explain`, `incoming_references::incoming` (model) | n/a |
| section `briefs` | ≤ 100 | 3 DD set hops: member → CatalogMemberInvocation → SelectedSeed → Brief (`:1101`), then page, then **per brief a full `get_capability`** (`:1110-1121`) | | Chain natural in both (this is probe J1) |
| section `behavior` | ≤ 100 facets | 4 hops: member → invocations → SummaryFacet, plus input → AnalysisInvocation → AnalysisOutcome (count must match, else Contract) (`:1151-1156`). Page; then page-scoped set hydration (ClaimConclusion …) plus `capture_packets::captures` (10 hops including parent walk `origin_site_parent`, `capture_packets.rs:766`) | Availability derivation: NotRequested / Unavailable("summary analysis invocation absent") / Partial; facet without qualification counted as `unavailable` (unknown ≠ absent); conclusion (qualification, verdict) must equal the facet's | First part natural (this is probe J3); captures: see `get_evidence` |
| section `access_routes` | ≤ 20 per page | Tier P computes the module scope and alias and event selectors; then one query block with **15 set hops** (`access_routes_service.rs:95-128`): ClassEntity, ImportAliasObservation → ImportModuleAssessment → Candidate → ModuleResolutionObservation → Support; RuffBinding → Support; PublicName → Support; ExportOrigin; ProviderRun / Surface / Evidence / Provider. Then **model path search** `catalog::access_routes::explain(max_depth = 16, max_routes = 128)`, with visited sets and `partial` flags (`lctx-model/.../access_routes.rs:305-563`) | The re-export route search with correspondence checks is model logic. `partial` marks a finite-search cut | Reads natural (set chain). The route search is **recursive path expansion** over a hydrated subgraph: S has `{1..16}` recursion but no visited-set or partial semantics; P has a recursive CTE with array cycle check. Both are awkward and would duplicate the model's correspondence rules |
| `get_evidence` | `OriginalReference` (6 variants), page (byte offset cursor; 4 KiB / 32 KiB body). Output: `OriginalRange`, body page, flow_inventory (≤ 16/64), source_characterization (≤ 16/64), derivation explanation; byte-fit loop pops items, setting `Partial` (`evidence_service.rs:224-449`) | (a) `resolve_original`: a **sum-type loop**. It walks Catalog → OriginalSource arm, Anchor → AnchorSource arm, Prose → ProseSlice → ProseSource arm, reaching Artifact / Occurrence / Span (up to 4 iterations, single-id). It then resolves release through ArtifactOwnership or CorpusLibrary → InputDistribution (FirstParty unique) or DistributionVerification, and context through ProviderRun (unique) (`:39-198`). (b) `visit_for` ArtifactChunk by artifact ordered by ordinal, **streaming all chunks** through `ArtifactVerifier` while slicing the page (`:290-312`). (c) `flow_inventory`: reads **all occurrences of the artifact**; **byte-range filter in Rust**; FlowUse → FlowUseInventoryObservation; **per inventory** a qualification lookup (N+1) with a context filter; sort; then per selected (≤ max) 6 single-id hops plus model `explain()` replay (`flow_inventory_service.rs:18-140`). (d) `source_characterization`: all characterizations of the artifact; per row qualification + source chain + context and range filter; per kept row a premise sum dispatch → `source_usage` (40 single-id hops, `source_usage_service.rs`) or `diagnostic_correlation`, which walks an ordered `DiagnosticUsePath` and verifies `placement.parent` continuity step by step (`source_characterization_service.rs:39-243, 394-626`). (e) `explain` (§0.4) | Relative-offset arithmetic; unique-release and unique-context rules (`unique()` → Contract); whole-artifact digest verification; context-equality filters; model flow `explain` / `entry_outcomes`; path-continuity proof; omitted counts computed *after* full resolution, so they are exact; derivation receipt digest equality | (a) S: awkward (polymorphic loop; FOR/IF in a block); P: awkward (recursive CTE over a union of 6 polymorphic tables, or several statements). (b) Natural in both. (c, d) Natural set reads; the Rust range and context filters are plain predicates in either QL, but the model replay and usage semantics are not. (e) Recursive: S `{1..8}` over ~137 heterogeneous derivation tables, awkward; the receipt-digest re-hash is **infeasible** in either QL without reading whole relations. P: recursive CTE over a UNION ALL of derivation tables is feasible but awkward; the digest check stays in Rust |

---

## 3. Semantic decisions between hops that would become a second authority if moved [obs]

1. **Selection classification.** `selection::Outcome` Supported / Unresolved / Conflicting, Strict vs Discovery and joint status are tier-P model algebra (`catalog_service.rs:359-640`; DESIGN §15.12 "one ranking owner"; api-product line 298-315).
2. **Ranking.** `RankingPolicy::rank` fusion and channel identity (`retrieval_service.rs:543-645`). BM25S runs in Python over a Rust-supplied corpus.
3. **Sum-type and polymorphic dispatch, each validating arm invariants:**
   - `Assumption`, `assertion::Evidence`, `OriginalSource`, `AnchorSource`, `ProseSource`;
   - `AssertionSource` (6 arms; TerminalSummary admissibility);
   - `NativeAssertionPremise`;
   - `SignatureTypingOrigin`.
4. **Reconstruction equality checks:**
   - `AssumptionSet::new(members) == stored set` (`packet_reads.rs:524-530`);
   - `AssumptionUniverseSupport::new(u, catalog, model)` parses an authored model (`:550-551`);
   - qualification shape (context, empty assumptions, `always()` condition, Definite, Exact; `:584-591`).
5. **Model replay inside hydration:**
   - flow `explain` / `entry_outcomes` (`flow_inventory_service.rs:98-140`);
   - `access_routes::explain` path search;
   - `contract_comparison`, `contextual::explain`, `incoming_references`.
6. **Closure checks (CI-11) that return Contract rather than empty:**
   - row counts equal requested keys;
   - every option evidence row exists;
   - outcome count equals invocation count;
   - non-empty supports, spans and parts;
   - unique release and context;
   - ordinal contiguity.
7. **Availability semantics (unknown ≠ absent).** `Partial` / `Unavailable` / `NotRequested` derivation (behavior section `:1176-1196`, scenarios, flow inventory, source characterization), plus exact `omitted` counts.
8. **Budget charging.** About 523 lines (§4) of `reserve` / `retain` / `try_resize`. `read_for` itself charges keys and decoded rows (`mod.rs:966-994`); `visit_physical_inner` admits raw bytes and `MAX_ROW_BYTES` per row (`:1092-1129`).
9. **Integrity re-checks:**
   - `ArtifactVerifier` over all chunks (whole-artifact digest);
   - `explain`'s per-relation receipt re-hash;
   - the vector bit-identity check;
   - `PacketBinding::permits` declared read scopes per packet kind (`packet_reads.rs:7-51`, `mappings.rs:230-330`; 16 `PacketKind`s).

---

## 4. Why one-hop exists (from `read_for` and its callers) [obs] / [int]

**What the primitive enforces itself [obs]:**
1. **Typed declaration check before any SQL.** The relation must be declared and inside the generation's frontier (`relations` set). The field must be a non-list `Scalar::Id` whose `target()` is `T`. The order fields must be declared (`mod.rs:888-915, 948-965`). Identifiers never come from a request (doc comment `:937-938`).
2. **Budget charging and per-row byte admission** (above).
3. **Typed decode** (`Batch::<R>::read`), using the same codec as compile.
4. **Generation pinning by lease.** The lease's own session holds the advisory lock. Reads run autocommit; consistency comes from published-generation immutability, not a snapshot transaction. Only `selection_read` uses `REPEATABLE READ READ ONLY` (`selection.rs:224-226`).
5. **Declared read scope per packet** through the `PacketLease` wrapper.

**Why one table per call [int, supported by observation]:**
- The primitive is per `Record` type so that each read is typed, scope-checked and charged against one declared relation and one nominal field. A JOIN would need a typed multi-relation result shape, plus a scope and charge model across relations.
- Nothing in the code or §15.12 forbids joins. §15.12 says serving shapes are "generated views over canonical relations, with grants and lookup indexes", but the generated views are identity projections (`ddl.rs:383-390`), not joins. Hops read canonical tables directly (`qualified(g, R::NAME)`).
- The design text names "bounded hydration" and "attempted-read controls" (`semantic-model.md` §15.12 line 990-992), not a one-hop rule.
- So the pattern stems from **typed decoding + scope + charging per relation, plus simplicity**, not from an absence of join capability.
- The **N+1 single-id loops** are a further, separate choice: per-row semantic dispatch inside a loop, rather than collecting keys per arm. `behavior_section` shows the batched alternative is already used where convenient. Its comment reads "Each relation is hydrated once for the exact selected page, not once per facet" (`operation_sections.rs:1213`).

**Indexing [obs].** `serving_*` lookup indexes exist only for `mappings::inventory()` relations' key fields, on `(generation_id, key)` (`ddl.rs:391-406`). `read_for` emits `WHERE key = ANY($1)` without `generation_id`. There is no check that a hop's field is indexed (see uncertainties).

**Line counts.** Heuristic statement classifier over the src portion of the 14 serving files; script at `scratchpad/classify.py`, statements split at `;{}`:

| Category | Lines | Share |
|---|---|---|
| hop call statements (read/visit/sqlx) | 853 | 9.0% |
| lease/query plumbing | 100 | 1.1% |
| budget charging | 523 | 5.5% |
| closure/contract checks | 691 | 7.3% |
| semantic assembly (DTOs, dispatch, paging, model calls, imports) | 7,331 | 77.2% |
| **total** | **9,498** | |

Hop statements are most concentrated in the deep-chain files:

| File | Hop share |
|---|---|
| source_usage | 25% |
| source_characterization | 26% |
| capability | 25% |
| flow_inventory | 19% |
| catalog_service, retrieval, native | about 0% |

Single-key (`&[x]`) hop calls per file:

| File | Single-key / total |
|---|---|
| source_usage | 40/40 |
| source_characterization | 37/37 |
| capability | 31/43 |
| flow_inventory | 19/28 |
| packet_reads | 18/25 |
| evidence | 14/16 |
| capture | 8/10 |
| operation_sections | 5/25 |
| packet_service | 0/6 |
| access_routes | 0/15 |

[int] Hop orchestration that a joined or graph query would replace is about 1–1.5k lines: hop statements plus key-collection glue. That matches A1 and A5's "~1–2k of 11.2k".

---

## 5. Facts bearing on the SurrealDB traps

1. **Silent depth bound (`{..N}` truncates silently).**
   - Bounded traversals carry explicit truncation semantics today:
     - `explain` sets `truncated` when `depth+1 == 8` with a non-empty frontier, or when the node or edge cap is reached, with reason "remaining count unknown" (`evidence_service.rs:631-665`);
     - `access_routes::explain` sets `partial` on depth > 16 or routes > 128.
   - Each needs a "was anything cut" signal that a silent `{..N}` bound does not give. An extra probe query at N+1, or a count, would be required.
   - Most other chains have **fixed** depth (schema-determined), so the depth bound is irrelevant to them.
2. **No JOIN.**
   - Every tier-R hop is either a forward link (`read_ids` of ids taken from a field), which is a record-link fetch in SurrealQL, or a reverse nominal lookup (`read_for(field)`), which is `<~(t FIELD f)`. The probe confirmed that the field must be named (trap 1 in the probe README).
   - The join-like work is **cross-path equality checks** and **predicates on non-key columns**:
     - byte-range containment `o.start >= grant.start …`;
     - `q.context == grant.context`;
     - `role == FirstParty`;
     - count equalities.
   - These are WHERE or closure filters in SurrealQL and plain predicates in SQL. No hop requires a non-key equi-join between two independently reached sets except the cross-check equalities, which today raise Contract.
3. **No edge identity in `+path`.**
   - The model has no list-of-Id fields; every edge is a row (A1 §2). Served proofs cite **edge-row ids**:
     - `ProofReference` to support, link, derivation and path rows;
     - `DerivationStep.source` is the derivation row;
     - premises carry `role` names;
     - `DiagnosticUsePath` rows are cited per step.
   - If edges were RELATE edges, `+path` would lose them. In the probe's record-link representation, a logical step is two hops: reverse ref to the edge row, then forward link. That is not `->e->` traversal, and identity is kept only because the edge is a record.
   - `explain`'s premise targets are **heterogeneous**: each derivation relation names its own premise target relations.
4. **Polymorphism.**
   - About 9 sum-typed dispatch points (§3.3) require type-tagged branching mid-chain.
   - In SurrealQL: `IF` / `match`-like blocks.
   - In SQL: a LEFT JOIN per arm, a CASE, or several statements.

---

## 6. Uncertainties and gaps

- **No execution or measurement.** Hop counts per real request (fan-out of occurrences per artifact, assumptions per qualification, briefs per member) and lease-acquisition cost are not measured. The 30 s deadline and 2-connection limit are contract values, not observations.
- **Index coverage.** Whether every `read_for` field has a `serving_*` index, and whether PG18's planner uses `(generation_id, key)` indexes without a `generation_id` predicate (skip scan or CHECK-constraint exclusion), was not checked. Fields such as `ArtifactChunk.artifact`, `ProviderRun.input` and `Occurrence.source` may be unindexed if their relation is not in `mappings::inventory()`.
- **Partially read code.**
  - `source_usage_service.rs` was scanned, not read line by line; its "40 single-id hops" is from regex.
  - `capture_packets.rs` and `relationship_section` / `conflict_section` were scanned.
  - `retrieval_service::request` (`:333-519`) was not read in full.
- **Prepared-relation counts.** "~95 classification relations, ~28 retrieval" are counted from macro field lists. Selection Output and native inputs were not enumerated.
- **The line classifier is heuristic.** Statement splitting at braces assigns block bodies coarsely, so take the percentages as ±5 points.
- **Absence claim, "no SQL JOIN in request-time serving reads".** This covers `crates/lctx-postgres/src/generations/*` request paths. The `JOIN`s present are in `vectors.rs` catalog inspection and control or receipt queries, not semantic hydration. The search was a grep of serving files plus a reading of `visit_physical_inner`.
