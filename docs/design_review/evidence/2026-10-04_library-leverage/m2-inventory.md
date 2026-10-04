# M2 inventory: lctx-model domain (generic mechanisms vs domain meaning)

Supporting evidence for the [library-leverage review](../../reviews/design_review_library-leverage_2026-10-04.md). Repository-fact inventory returned by the M2 code-mapper lane (read-only, baseline `948b2a88`); published verbatim by the coordinator. No verdicts.

The inventory covers the assigned `lctx-model` subtree, and the macro survey covers the whole workspace. The most repeated mechanism is stage-port declaration. 66 exported relation-list macros are the per-stage inventories, and 408 local macros expand them or restate type lists by hand. The 11-relation analysis-publication set is spelled out by hand in at least 16 places, even though `analysis_family!` already generates `publication_relations()`.

Baseline: `main` @ `948b2a88`. HEAD equals the baseline and no commits touch `crates/`, so every cited location matches the baseline. Read-only: nothing was built or run.

## 1. Candidate inventory

### M2-01 Typed relation container, foreign-key lookup and charged secondary indexes
- **Location:** `normalized/rows.rs:8-60` (`Rows<R>`: an Id-keyed `ChargedMap` with conflict-on-same-key insert, `decode`, `same`). `charged.rs:81-215` (`ChargedMap`/`ChargedSet`/`ChargedVec` over BTreeMap/BTreeSet/Vec, with byte charging). Hand-built group-by indexes, for example `selection/preparation.rs:17-60` (`ChargedMap<Frame, Id>` and `ChargedMap<Id, Vec<Id>>` with uniqueness and FK checks) and `normalized/callable_normalization.rs:101-234`.
- **Capability:** an in-memory typed relation with primary key, conflict detection, FK dereference-or-error and secondary (multi-)indexes. In other words, a small in-memory relational store.
- **Scale:** `fn need<R: Record>(rows, id) -> Result<&R, _>` is copied 52 times across the workspace (49 in lctx-model, for example `catalog/build.rs:33`, `retrieval/build.rs:12`, `synthesis/seeds.rs:82`; 3 in lctx-postgres). `Rows` has no such method. `ChargedMap<` appears 459 times in 64 model files; `.update(` index-building appears about 121 times.
- **Invariants a replacement must keep:** every retained byte charged (`StateCharge`, `NODE_ALLOWANCE=32`); same-key different payload is a `ModelError::Conflict`; ordering is by nominal `Id` bytes, so it is deterministic and independent of input order; no library-local index may escape (Ids only); model purity.
- **Libraries already used inside it:** std collections only.
- **Stated reason for bespoke:** ADR-0085 (pure model, no DataFusion). ADR-0073 Decision: "Keep suitable bulk joins in DataFusion and simple finite closures as indexed worklists."
- **Tests:** indirect, through every stage replay test; no dedicated `rows`/`charged` unit test was found in `src`.
- **What a library would have to provide:** a pure in-memory multi-index map with a byte-accounting hook (a fallible allocation or reservation callback), deterministic ordered iteration, typed keys, and error-on-conflicting-duplicate.

### M2-02 Nested-loop joins and semijoins written as scans
- **Location, representative:** `synthesis/documentary_templates.rs:46-65`. `native()` filters `d.native` and calls `d.native_qualifications.iter().any(..)` per row, an O(n·m) semijoin. `retrieval/build.rs:110-160` filters `exposures`, `callables` and `invocations` per member. `catalog/access_routes.rs:569+` filters `d.source.core.bindings` per DFS hop.
- **Counts of `.iter().filter(`:** normalized 51, selection 27, catalog 33, synthesis 40, structural 28. Counts of `.iter().any(`: normalized 26, catalog 20, synthesis 22, structural 18.
- **Capability:** equi-joins, semijoins and anti-joins over `Rows`.
- **Invariants:** the outcome must be independent of input order (several sites then `sort_by_key(id)`); ambiguity must be detected (`rows.next().is_some()` → error, `documentary_templates.rs:104-122`).
- **Bounds:** work in these scans is not charged. Only retained state is charged (`charged.rs` header: "Cardinality alone never refuses; semantic work bounds remain with their owners").
- **Stated reason:** same as M2-01 (ADR-0085 purity; ADR-0073 indexed worklists). No per-site reason found.
- **What a library would have to provide:** pure indexed join or semijoin over typed rows, deterministic output order, duplicate/ambiguity reporting, and optional work counting.

### M2-03 Stage-input dependency closure, implemented separately in several places
- **Shared owner:** `dependency_closure.rs:16-175` (`DependencyClosure::build`). Called from `catalog/evidence/build.rs:1071`, `selection/build.rs:855`, `synthesis/production.rs:151` and `analytics/build.rs:788`.
- **Separate implementations of the same walk** (relation → `fields().target()` plus invariant inputs, fixpoint over a `pending` stack, own-output refusal, facts/vocabulary filter):
  - `retrieval/build.rs:943-990`
  - `structural/build.rs:600-640`
  - `local_semantics.rs:890-930` (this one panics instead of returning an error)
  - `embedding/text.rs:~190-210`
  - `analysis/preparation.rs:~160-180`
  - `analysis/frontier.rs:433` and `:643`
- **Related:** relation lookup by name is a linear scan, `model.relations().iter().find(|r| r.name()==name)`, at about 46 sites. `ValidatedModel` (`model.rs:111-470`) has no by-name accessor.
- **Capability:** transitive closure over the declared reference graph.
- **Invariants:** a predecessor that requires its own output is refused; vocabulary epochs (`at_epoch`) and the facts/vocabulary omission policy apply; the result is sorted and deduped by name.
- **Libraries available:** petgraph is pinned in lctx-model but not used here.
- **Stated reason:** the `dependency_closure.rs` header says "Exact validation universes and their sufficient stage grants are different products". No reason found for the separate copies.
- **Tests:** `tests/dependency_closure.rs` (3 tests) covers the owner only.
- **What a library or derive would have to provide:** one model-owned reference graph (for example a petgraph `DiGraphMap` built once in `ValidatedModel`) with a reachability query parameterised by the stop/omit policy.

### M2-04 Stage port declarations: data structs, name dispatch, validation inputs, stage inputs/outputs and writer declarations
- **Summary:** generated by macro_rules rather than by a derive. The detailed counts are in §2.
- **Representative:** `normalized/binding_normalization.rs:18-48`. Local `inputs!`/`outputs!` expand into `BindingData`/`BindingOutput`: `Rows` fields, `new`, `visit(name, batch)` name dispatch, `validation_inputs()`, `stage_inputs()` with the vocabulary-epoch rule, and `matches`. The authoritative list is `normalized/binding_inventory.rs:3-174`.
- **Invariants:** codebooks and relation names are append-only; every stage output is declared by its writer, which is enforced only at runtime (`stages.rs:2129-2152` refuses a declaration outside the stage; `stages.rs:2234` refuses "undeclared outputs" on finish); input epochs are applied; validation key columns are always `&["id"]`.
- **Stated reason:** ADR-0085 Decision lists "inventories … and typed operation ports" as generated forms. DESIGN §15.2: "A bounded derive generates the typed key, identity recipe, physical Arrow codec and metadata." The derive in `lctx-model-macros/src/lib.rs:211-504` does not generate ports or inventories; the macro_rules lists fill that role.
- **Tests:** `tests/domain_stages.rs:668` (`the_schedule_refuses_double_or_missing_writers_foreign_relations_and_cycles`) and the runtime equality check.

### M2-05 Canonical hashing and encoding: one structural framing plus three other byte sources
- **Primary:** `identity.rs:163-185` `KeySink` (tagged, length-framed parts over blake3) and the `Key` trait, `identity.rs:228-275`. There are 132 `KeySink::new(` sites in lctx-model.
- **Other encodings feeding identities:**
  - **postcard:** `serving/cursor.rs:29,58`, `serving/identity.rs:50` (`policy_identity`), `serving/dispatch.rs:36` (`RequestIdentity`).
  - **serde_json bytes:** `selection/algebra.rs:514-536` (`selection_digest`); `serving/dispatch.rs:177-202` (schema JSON bytes hashed into the wire identity); `embedding/spec.rs:101-108` (SHA-256 of canonical JSON, citing DESIGN §11.1).
  - **Debug formatting (`format!("{x:?}")`) hashed:**
    - `serving/mappings.rs:119,127,132,146,149,169`, including Arrow `Field` Debug and `Sum` Debug.
    - `model.rs:311`: `digest.part(b"sum", format!("{sum:?}"))` inside the model digest.
  - **Bespoke framing kept on purpose:** `embedding/value.rs:33-46` `value_digest`. The comment says: "Preserve its complete byte recipe so current immutable cache winners retain their meaning".
- **Invariants:**
  - Encoding must be stable across dependency upgrades.
  - Debug output of `arrow_schema::Field` is not a declared stable contract.
  - JSON map ordering depends on serde_json `preserve_order`, which is enabled through `crates/lctx-workspace-hack/Cargo.toml:98` with `feature-unification = "workspace"` (`.cargo/config.toml:13`). It is consistent inside workspace builds; not verified for out-of-workspace consumers.
  - `Id` keeps a 16-byte truncation of the 32-byte blake3 digest.
- **Libraries already used:** blake3, sha2, postcard, serde_json.
- **Tests:**
  - `identity.rs:444` checks prepared-hash framing.
  - `tests/serving_contracts.rs:169,206,310` covers request identity, cursor invalidation and policy/wire identity.
  - `tests/snapshots/ids__known_answer_vectors_pin_the_encoding.snap` and `ids__the_kinds_are_an_append_only_codebook.snap` have no owning test file (last commit `8ae5faff`, identity v2).
  - No KeySink v3 known-answer vector was found (search limited to ≥32-character hex literals in `tests/` and `src/`).
- **What a library would have to provide:** a canonical, versioned, schema-stable binary encoding usable for hashing (sorted-map rule, tagged sums, explicit framing), plus a stable canonical encoding of Arrow schemas or fields.

### M2-06 Hex encoding and decoding (three bespoke copies)
- **Location:** `identity.rs:28-30` (`Id::hex`), `identity.rs:148-150` (`ContentHash::hex`), `serving/cursor.rs:62-91` (`Cursor::encode`/`decode`: JSON → hex text, hand-written parse with even-length and hex-digit checks).
- **Libraries already used:** blake3 is pinned and already in use; whether `blake3::Hash` has a hex API was not checked. `base64 =0.22.1` is pinned but only used in `crates/lctx/tests`.
- **Invariants:**
  - The cursor must refuse a changed binding (generation, request, policy, wire, channels, group, section, member, ordering).
  - The token is bounded by `CursorToken = Text<1,8192>`.
  - The cursor token format is an external contract.
- **Tests:** `tests/serving_contracts.rs:206` (`continuation_rejects_each_invalidated_boundary`).
- **Stated reason:** none found.

### M2-07 Hand-written JSON Schema for codebooks and domain sums
- **Location:** `serving/schema.rs:6-28` (`Id`/`ArmId`/`ContentHash` schemas); `:29-110` `code_schema!` over a hand list of about 76 `DomainCode` types; `:117-152` `enum_schema!`, 5 invocations that restate every variant, field and field type.
  - `enum_schema!` carries a compile-time exhaustiveness guard: `const _: fn(&T) = |v| match v {…}`.
  - Domain enums are not `#[derive(JsonSchema)]`; `DomainCode` (`lctx-model-macros/src/lib.rs:506-585`) emits `codes()` but no schema.
- **Invariants:** a schema must cover every canonical variant (guarded); integer codes come from `FieldValue::codes()`; codebooks are append-only; draft 2020-12 closed objects.
- **Stated reason:** module header: "JSON schemas for existing nominal and finite selection values; no independent predicate meaning". ADR-0073: "matching flat wire components derive from existing declarations".
- **Libraries already used:** schemars 1.2.2, plus jsonschema as a dev dependency.
- **Tests:** `tests/serving_contracts.rs:56,78` (`all_ten_routes_decode_and_emit_distinct_schemas`, `schema_null_tag_type_and_unknown_field_controls`).
- **What a derive would have to provide:** a JsonSchema emitted by `DomainCode`/`DomainSum`/`Domain` from the same declaration, or schemars derive on the domain types with explicit codebook integer representation. The exposure set (which types are on the wire) would still need a declaration.

### M2-08 Wire DTO bundles and closed-request checking
- **Location:**
  - `serving/packets.rs:6-9` `packet!` (68 DTOs).
  - `serving/responses.rs:6-9` `response!` (10).
  - `serving/requests.rs:75-79` `request!` (5).
  - `serving/dispatch.rs:8-60` `routes!` (`Tool` enum with `ALL:[Self;10]`, `name`, `from_name`, `description`, schema dispatch, `Request`/`Response` enums, `to_json`, `json_len`).
  - `serving/dispatch.rs:208-226` `check_closed` (recursive raw-JSON vs re-serialized comparison that rejects unknown fields at any depth).
- **Restated fields:** many packet fields restate record fields by hand (for example `NativeSourceSupportPacket`, `packets.rs:562`). These are built field by field in `lctx-postgres/src/generations/source_usage_service.rs:322` and `source_characterization_service.rs:340`.
- **Invariants:** closed requests; no silent truncation; wire byte accounting (`json_len`); absent is distinct from null (`values.rs` `Optional`/`Nullable`).
- **Stated reason:** ADR-0073 says "nested envelopes have their own typed composition". The `PacketMapping` doc (`mappings.rs`): "this is not a projection DSL".
- **Tests:** `tests/serving_contracts.rs` (25 tests, lines 56-891).

### M2-09 Ranking: tokenizer, document-frequency statistics and reciprocal-rank fusion
- **Location:** `serving/ranking.rs`:
  - `:97-103` `tokenize`: lowercase, split on non-ASCII-alphanumeric.
  - `:573-657` corpus preparation: dedup by `KeySink` document id, document frequency per family.
  - `:663-697` discriminating query words.
  - `:336-499` `rank`: per-(family, channel) winners, contiguous ranks, RRF k=60 within families, then equal-weight RRF across families, exact-name promotion, deterministic tie-breaks with `total_cmp` then target.
  - `:504` `reciprocal`.
- **Numerical BM25:** computed in Python, `python/lctx_mcp/src/lctx_mcp/retrieval.py:38-70` (bm25s 0.3.11 on numpy, fed by Rust tokens).
- **Invariants:**
  - The Rust tokenizer defines the tokens bm25s indexes.
  - Lexical scores must be finite and ≥0; duplicate conflicting scores are refused.
  - The admitted closure must equal the covered occurrences.
  - Allocation is charged (`PREPARATION_BYTES_PER_ROW=768`, `FUSION_BYTES_PER_ROW=2048`).
  - The policy is pinned: `RankingPolicy::validate` refuses anything other than the default.
  - Ranks aid navigation and never count as evidence.
- **Stated reason:**
  - ADR-0114:52: "Retain bm25s as numerical scoring … Rust owns ranking decisions and witnesses once."
  - ADR-0077:50-53.
  - Phase 5 plan :274-276.
  - Forward plan §7 F13 (a change of lexical policy requires a named capability).
- **Tests:** `tests/ranking_policy.rs:40` (`member_projection_shares_independent_evidence_frequencies_and_exact_occurrences`).

### M2-10 Interval containment index
- **Location:** `attachment.rs:37-130` (sorted entries plus an implicit augmented `max_end` tree; `build_index` and recursive `search` for the innermost containing span) and `:137-260`. Separately, `occurrence_owner.rs:61-80` (`owner_of`) finds the owner through BTreeMap structural-path prefixes.
- **Capability:** an interval-containment and innermost-enclosing query.
- **Invariants:**
  - Ambiguity is explicit (`Attachment::Ambiguous`), never a convenient pick.
  - `BudgetExceeded` is reported when `visited_nodes` (1,000,000) or `alternatives` (256) is exhausted.
  - Index buffers are charged.
  - Grouping is by (source, kind, role).
- **Libraries already used:** none.
- **Stated reason:** header: "One indexed source/span/kind/role attachment rule."
- **Tests:** `crates/cpg-extract/tests/attachment_oracle.rs` (a scalar-scan oracle); `src/domain/attachment.rs` has 0 unit tests.

### M2-11 Bounded enumeration of all simple paths (access routes)
- **Location:** `catalog/access_routes.rs:268-300` (per-route visited set and charge) and `:532-770` (depth-first `pending` stack; cycle → `RouteStop::Cycle`; `max_depth` → `RouteStop::Frontier`; `max_routes` → `omitted_frontiers` count).
- **Graph construction:** edges are produced on the fly by scanning `d.source.core.bindings` per hop.
- **Invariants:** route cap and omitted-frontier accounting; cycles reported as typed stops rather than skipped; charged per hop; deterministic order.
- **Libraries available:** petgraph 0.8.3 is pinned in lctx-model; its `all_simple_paths` was not compared.
- **Tests:** `catalog/access_routes.rs` has 6 unit tests, including `:1284` `route_cap_counts_every_discarded_pending_frontier`.
- **Stated reason:** none found beyond the module header.

### M2-12 Text windowing and sentence/literal extraction
- **Location:**
  - `embedding/text.rs:459-520` `windows`: line-preferring byte windows with a UTF-8 boundary backoff.
  - `synthesis/documentary.rs:360-398` `first_sentence`: heuristic paragraph stops (blank line, `:`, `>>>`, underline) and then `unicode_segmentation::split_sentence_bound_indices`.
  - `synthesis/documentary.rs:329-358` `literal_interior`: hand parse of Python string prefix and quote kind.
  - `embedding/spec.rs:111-123`: template substitution via `.replace("{text}")`.
- **Invariants:**
  - Byte-exact offsets into the original text.
  - Charged index.
  - Windows never stand in for tokenizer admission (comment at `text.rs:457`).
  - Prefixes other than r/u are refused.
- **Libraries already used:** unicode-segmentation 1.13.3. The Ruff parser lives in extraction, outside the pure model.
- **Tests:** `synthesis/documentary.rs` has 12 unit tests; `embedding/text.rs` has 0 (see `tests/analytic_text.rs`).

### M2-13 Text rendering (string building)
- **Location:**
  - `retrieval/build.rs:106-270` and `:415-510`: 22 `format!`, 13 `push_str`, 14 `{:?}` Debug renderings of domain enums into retrieval/embedding corpus text.
  - `synthesis/briefs.rs:262-313` (brief markdown).
  - `synthesis/summary.rs:483-513`.
  - `synthesis/assertions.rs:761-825`.
  - `serving/packets.rs:545-555` (markdown with fenced JSON).
- **Invariants:**
  - Rendered text is content-hashed (`ContentHash::of`), so it feeds identities and embeddings; Debug output of domain enums therefore becomes part of the corpus contract.
  - Renders are bounded ("finite rendering bound", `retrieval/build.rs:475`) and charged before allocation (`:337-366`).
- **Stated reason:** forward plan §7: "Stage F brief-builder restructuring | A rendering consumer needs it; briefs remain one rendering". No templating reason found.
- **Tests:** `tests/domain_retrieval.rs`; `retrieval/build.rs:1012+`.

### M2-14 Allocation-size accounting (`HeapSize`)
- **Location:** `record.rs:94-260` (trait plus impls; `inline_only!`, `tuple_heap!`, `primitive_list!` macros). The `Domain`/`DomainCode` derives emit `HeapSize` (`lctx-model-macros/src/lib.rs:433-437`, `:575`).
- **Scale:** 58 manual impls in the workspace (22 empty), 37 `fn heap_bytes`.
- **Invariants:** admission, not measurement (header at `record.rs:92`); capacity-based; saturating.
- **Libraries in the lockfile:** `get-size2 0.11.0` with `derive` is already transitively locked (Cargo.lock:3106, from the ty/ruff family; listed in workspace-hack line 57). It is not a direct dependency of lctx-model.
- **Stated reason:** none found.

### M2-15 Checked scalars and closed-shape validation
- **Location:**
  - `serving/values.rs:6-37` `Text<MIN,MAX>` (byte bounds plus a NUL check plus schema), used as `Text<1,512>`, `<1,8192>`, `<0,16384>`, `<0,262144>`, `<0,8192>`.
  - `values.rs:38-99` `Optional`/`Nullable` (absent vs null): 51 `Nullable<` and 12 `Optional<` uses.
  - `models.rs:181-226` exact Python pin and distribution/version string validation by hand; `models.rs:909` `dotted_name`.
  - 90 `#[model(validate = …)]` row validators (for example `embedding/text.rs:39-118`): 12 status-implies-`is_none` shapes and 22 start/end range checks.
- **Invariants:** a shared checked-ingress/schema parity; the absent/null distinction (`Nullable` deserializes through `serde_json::Value`, `values.rs:91-98`).
- **Observation:** ADR-0073 states "a 500-Unicode-scalar limit for bounded selector/filter text, with a separate encoded-byte budget; migration must explicitly reconcile the current native byte limit". The code bounds bytes (`Name = Text<1,512>`).
- **Pinned elsewhere:** `pep508_rs =0.9.2` (with `pep440_rs`) is pinned and used in `cpg-extract/src/deployment_parser.rs:45,117`, not in lctx-model.
- **Tests:** `tests/serving_contracts.rs:351` (`resource_refusals_and_exact_scalar_boundaries`); `tests/unicode_values.rs`.

### M2-16 Enum name, iteration and lookup boilerplate
- **Location:** `const ALL:[Self;N]` in 11 places (`stages.rs:23,47,311`, `admission.rs:34,695`, `projection.rs:62`, `serving/mappings.rs:235`, `normalized/events.rs:39`, `serving/failure.rs:13`, `serving/dispatch.rs:13`); 5 `fn name(self)->&'static str` matches; 5 `from_name`/`from_code`. 221 `DomainCode` derives already provide `codes()` labels.
- **Invariants:** codebooks are append-only with explicit i16 codes (`DomainCode` refuses inferred discriminants, `lib.rs:520-545`).

## 2. Declaration-repetition survey (workspace)

- **Totals:** 514 `macro_rules!` definitions. By crate: lctx-model 116 files; cpg-core 20 files; cpg-extract 9; lctx-postgres 7.
- **Most common names:** rows 34, output 29, declare 28, data 23, inputs 21, outputs 20, write 16, read 15.
- **Classification** (script over macro bodies plus call sites; heuristic, ±a few):

| Pattern | Count | What it restates | Representative |
|---|---|---|---|
| Exported relation list (the authoritative per-stage inventory, X-macro style) | 66 | stage inputs/outputs as `field: Type` pairs | `normalized/binding_inventory.rs:3,161`; `catalog/inventory.rs:59`; `analysis/expected.rs:14` |
| Local macro passed to an exported list (re-expansion mechanics) | 268 | per site: data struct (65), runtime load (37), write (36), name-dispatch decode (25), writer `declare` (20), `Relation` list (19), `RelationUse` list (16), `ValidationInput` list (12), other (38) | `binding_normalization.rs:18-48`; `cpg-core/src/catalog_core.rs:60-81` |
| Local macro applied to a hand-written type list | 140 | writer declares (23), name-dispatch (40), stage `RelationUse` (11), write (19), other (47) | `cpg-core/src/catalog_core.rs:108-122`; `cpg-extract/src/pyrefly_stage.rs:116,321` |
| Other (generic derive-like impls, counters, comparisons) | 40 | `HeapSize`/`Key`/`FieldValue` impls, `count`/`compare`/`measure` | `record.rs:157-246`; `native_requests/preparation.rs:177-183,366,596`; `retrieval/build.rs:343-365` |

- **Expansion fan-out of the exported lists:** `expected_domain_inputs!` 16 sites in 13 files; `entry_value_inputs!` 12; `normalized_binding_outputs!` 11; `normalized_binding_inputs!` 10; `summary_vocabulary!` 9. All 66 are expanded at least once; 63 are expanded in at least 2 places.

**Same list spelled in several places for one contract:**

1. **Analysis publication set.** The 11 relations are Invocation, InvocationSource, AnalysisInput, ProjectionInput, SourceReceipt, AnalysisOutcome, AnalysisCoverage, CoverageSource, AnalysisCoveragePremise, CoverageRequirement and CoverageRequiredSource.
   - The authority is generated per owner by `analysis_family!` (`analysis/family.rs:3,132`, `publication_relations()`; 17 owners) and used by the registry (`analysis/mod.rs:71-102`).
   - The list is restated inline in order 16 times:
     - stage outputs in lctx-model: `catalog/build.rs:818`, `catalog/evidence/build.rs:1049`, `selection/build.rs:833`, `synthesis/production.rs:107`, `retrieval/build.rs:931`, `structural/build.rs:550`, `analytics/build.rs:731`, `embedding/analytic.rs:427`;
     - writer `declare!` in cpg-core: `catalog_core.rs:115`, `catalog_evidence.rs:114`, `catalog_selection.rs:118`, `retrieval.rs:250`, `structural.rs:249`, `synthesis.rs:233`, `analytic.rs:175`, `analytic_embedding.rs:222`.
   - With further variants in `semantic_execution.rs` (×4), `semantic_models.rs`, `semantic_summaries.rs` and `local_semantics.rs`, `CoverageRequiredSource` appears in stage/writer lists in 20 files.
2. **Writer declares vs stage outputs** for the same stage: `cpg-extract/src/pyrefly_stage.rs:116-140` (`uses!` outputs) and `:321-345` (`declare!`), identical lists. Likewise `cpg-extract/src/assembly.rs:177-200` (`vocabulary()`) and `:77-95` (`declare!`). `ty_flow.rs`, `document_parser.rs` and `deployment.rs` already share one list macro (`output_types!`, `document_types!`, `outputs!`).
3. **Metadata inputs:** `catalog/build.rs:781-799` (`metadata_inputs`) and `embedding/analytic.rs:405-415` (`add!`) each restate a subset of `expected_domain_inputs!` (`analysis/expected.rs:14-26`) plus AnalysisDefinition, MethodParameters, AnalysisContext and ProviderRun.
4. **Stage assembly:** 67 `Stage { … }` literals. 22 use `code: ContentHash::of(include_bytes!(file))`. There are 69 `dedup_by_key(name)` uses; input lists are concatenated from upstream `stage(profile).inputs` and then sorted and deduped (for example `catalog/build.rs:735-761`, `binding_normalization.rs:1553-1586` with a Catalog-profile `retain` exclusion list).
5. **Stage I/O consistency is checked only at runtime:** `stages.rs:2129-2152` and `:2234`.

**What a derive or reflection mechanism would need to know to generate these:**
- per field: the field name, the `Record` type, transport (stored, completed store, epoch via `is_vocabulary` → `PublicationBoundary`), validation key columns (always `["id"]` here), profile membership (Catalog/Behavioral exclusions) and optional named validators (`validated_by`);
- per stage: input sets as a union of upstream-stage inputs and prior outputs, outputs as the owner's `publication_relations()` plus domain outputs, contributes, effect, and the code/configuration identity recipe;
- for the runtime: an async load per field (cpg-core's `read`/`inputs`/`load` bodies vary: `access.read`, `consumed.next`, epoch sessions) and a writer `declare` for each stage output.

## 3. Trigger evidence (forward plan §7, `behavioral-model-forward-plan_2026-09-24.md:1127-1184`)

- **nutype / garde / bon** (checked scalar, contextual form, builder):
  - `Text<MIN,MAX>`: 6 instantiations, about 15 field types in serving.
  - Hand version/distribution checks: `models.rs:181-226`.
  - 90 row validators, of which 12 are status-vs-optional shapes and 22 range checks.
  - Builder: none observed; `MethodParameters` is built with 10 explicit `None` fields at `catalog/build.rs:765-775`, and the same shape recurs in other `definition()` functions.
- **strum / enum-map / serde_with / derive_more:** 11 `ALL` arrays (one hard-coded `[Self;10]` in a macro); `Tool::name`/`from_name` (`dispatch.rs:14-16`); `Optional`/`Nullable` absent/null wrappers (`values.rs:38-99`), 63 uses.
- **Typed dense collections:** none found in scope. Indexes are `ChargedMap<Id,…>` (459 occurrences); the only dense indices are petgraph `NodeIndex` in `projection/*`.
- **lasso / roaring:** no interner or bitset in scope. `fixedbitset` is a dependency of lctx-model but has no use in the files examined.
- **Moka:** no cache in scope.
- **trybuild:** compile-fail doctests exist (`identity.rs:11-18`).
- **Typify / reflection:** none. JSON Schema is emitted from Rust (M2-07).
- **Salsa:** none in lctx-model (`Cargo.toml` deps).
- **F13 lexical/FTS:** current state in M2-09.
- **Lockfile presence** (forward plan: "transitive lockfile presence are not adoption"): `get-size2` with `derive` (M2-14) and serde_json `preserve_order` (M2-05) are unified through `lctx-workspace-hack/Cargo.toml:57,98`.

## 4. Domain meaning, not a candidate

| Module | Judgment |
|---|---|
| `selection/algebra.rs` | Five-outcome classification, joint applicability over BDD conjunction/disjunction, aggregation order. Generic group-by inside is incidental (`:175-308`). |
| `selection/evaluate.rs`, `facets.rs`, `structural_facets.rs`, `vocabulary.rs`, `classification.rs`, `specialization.rs`, `source_fields.rs`, `frames.rs`, `admission.rs` | Predicate meaning and the unknown-stays-unknown rule. |
| `normalized/*` | Argument binder ("sole argument algorithm", carried forward via ADR-0085), dispatch, receiver, overload association, symbolic fields, coverage. Joins and indexes inside are covered by M2-01/02. |
| `catalog/build.rs`, `paths.rs`, `aliases.rs`, `catalog/evidence/*` | Public-slot, path, evidence and association semantics. |
| `synthesis/*` | Programmatic assertion templates are typed enums (`assertions.rs:43`). Documentary/MDX code interprets already-parsed components (`documentary_templates.rs:46-140`) and parses nothing. Rendering is covered by M2-13. |
| `retrieval/mod.rs`, `source.rs`, `consumption.rs` | Unit, anchor and winner recipe. |
| `serving/mappings.rs` (aside from M2-05 hashing), `failure.rs`, `resources.rs` | Row-view inventory and capabilities. |
| `embedding/configuration.rs`, `consumption.rs`, `analytic.rs` (aside from M2-04) | Admission and consumption semantics. |
| `projection/*`, `projection.rs` | Already library-backed: petgraph `Graph`/`Dfs`/`kosaraju_scc`, postcard snapshot with pinned versions (`snapshot.rs:10-21,240-290`). |
| `native_requests/*` | Pure composition over hydrated rows; macros covered by §2. |
| `admission.rs`, `admission/availability.rs` | Facts frontier and coverage admission (ADR-0087). |
| `structural/*` | S0 observation passes and replay reconciliation. |
| `types.rs`, `types/*`, `calls.rs`, `calls/*`, `symbols.rs`, `syntax.rs`, `source.rs`, `lexical.rs`, `ruff.rs`, `value.rs`, `protocols.rs`, `captures.rs`, `declarations.rs`, `documents.rs`, `diagnostics.rs`, `deployment.rs`, `input.rs`, `class_metadata.rs`, `atom_decision.rs`, `artifact.rs`, `attribution.rs` | Fact declarations and attribution. |
| `ownership.rs`, `occurrence_owner.rs` | Owner rule (DESIGN §15.4). The `owner_of` mechanism touches M2-10. |
| `resources.rs` | A pure port; `cpg-core/src/model_runtime.rs:311-360` adapts it to DataFusion `MemoryPool`/`MemoryReservation`. |
| `models.rs`, `models/*`, `models/external.toml` | Authored behavior catalog. Parsing already uses toml and serde with `deny_unknown_fields`; scalar checks are covered by M2-15. |
| `record.rs` Field/Scalar→Arrow (`:20-92`), `model.rs` `ValidatedModel` | Single-declaration lowering, as ADR-0085 intends. |
| `lctx-model-macros` | The intended bounded derive; serde_arrow codec, keys and `HeapSize`. Its gaps are covered by M2-04/07. |
| `build.rs` | Captures model and macro sources plus Cargo manifests and the lockfile as `semantic-contract.bin` (contract identity policy). |

## 5. Search coverage and limits

- **Coverage:** the macro survey used a Python scan of all `crates/*/src/**/*.rs` for `macro_rules!` bodies, a heuristic classifier and call-site matching; scratch output is at `/tmp/claude-1000/-home-paul-library-context/034e37c7-93d1-4b83-8efa-ce9530cfd68c/scratchpad/mac.json`. Classification counts are approximate (±a few). Tests and `lctx` CLI sources were not included in the macro counts.
- **Inline publication-list count:** a regex requiring the AnalysisOutcome…CoverageRequirement order, so differently ordered lists (`semantic_execution.rs` and similar) are undercounted.
- **Join, scan and lookup counts** are grep counts over the listed subdirectories, not semantic analysis. "No dedicated test" claims are limited to `#[test]` in the file plus a grep of `crates/lctx-model/tests` and `crates/cpg-extract/tests`.
- **Read in full or in relevant part:** serving/*, ranking, cursor, schema, values, dispatch, mappings; attachment; identity; rows/charged; dependency_closure and the parallel closures; catalog/build stage; embedding text/spec/value; documentary and literal/sentence code; macros crate; build.rs.
- **Header-level only:** most normalized/structural/native_requests/types files, `selection/evaluate.rs` (1454 lines) and `synthesis/patterns.rs` beyond its header.
- **Not done:** no library capability was checked; that belongs to the library researchers. No claim about what a library offers is made except crates already present in Cargo.toml or Cargo.lock.
- **Uncertainties:**
  - Whether out-of-workspace consumers compute wire/policy identities with different serde_json features (M2-05) was not verified.
  - Whether KeySink v3 has a known-answer test elsewhere: none found with the search above.
- **Docs consulted:** ADR-0073, ADR-0085 (Decision/Consequences), ADR-0114:45-60, forward plan §7, DESIGN §15.2 and §15 text at lines 900-915.
