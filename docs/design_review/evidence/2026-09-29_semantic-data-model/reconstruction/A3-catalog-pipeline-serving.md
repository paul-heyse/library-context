# A3 — Catalog pipeline, registries, identities and serving (Phase-1 reconstruction)

**Baseline.** Code at commit `35afc09` (PR5 journeys and coarse rebuilds, ADR-0081). HEAD later moved to `fedd4a0`; `git diff --stat 35afc09 fedd4a0 -- crates python` is empty, so all code line numbers hold. The PR5 review cited here (`docs/design_review/reviews/design_review_pr5-agent-journeys-rebuild_2026-09-29.md`) was committed in `fedd4a0`; its F04/§4/§8/§9 text is quoted as read on 2026-09-29. No repository file was edited; no build, test, pytest, probe or query was run for this document (2026-09-29).

**Labels.** Every claim is **Implemented** (`[I]`: the code path was read) or **Interface-checked** (`[IC]`: only a signature, DDL, contract, test name or third-party behaviour was read). Nothing here is labelled Tested. Citations are `path::symbol:line` at `35afc09`. Sections 1–2 use the path prefixes defined in §1.0; the other sections use repository-relative paths (a bare `attempt.rs`, `stage_cache.rs`, `rebuild.rs`, `catalog.rs`, `evidence.rs`, `bundle.rs`, `derive.rs`, `delta.rs`, `snapshot.rs`, `validate.rs`, `analyze.rs` in §0, §4, §5 and §7.1 means `crates/cpg-core/src/…`; bare `config.rs` means `crates/cpg-extract/src/config.rs`).

**Method.** §4, §5, §7.1 and E16/E17 were reconstructed directly; §1–§2 (CI tables), §3 (registry probe P7) and §6/§7.2/E12–E15 (serving) were reconstructed by three read-only sub-investigations and spot-checked here against source (G1 default-state collapse `crates/cpg-schema/src/selection/catalog.rs:897-899`; `find_operations` masking `python/lctx_mcp/src/lctx_mcp/server.py:585-605` with `NativeWorkers.run:129-151` not mapping `ValueError`; type-closure BFS `crates/lctx-postgres/src/hydration.rs:237-267`; hand `ALTER TABLE … literal_json` at `crates/lctx-postgres/migrations/202609280012_selection_retrieval.sql:111`; no `summary_flows` arm in `serving_projection::unique_key`; per-request `PreparedCatalog::new` at `crates/lctx-postgres/src/selection.rs:326`).

## 0. Decision-relevant results

1. **The PR5 stage cache verifies, it does not skip.** `stage_cache::contracts` always runs the full pure derivation and admits a cache entry only when its canonical batches equal the fresh result (`crates/cpg-core/src/stage_cache.rs:292-301,330-342`); publication validation then derives the contracts a second time (`crates/cpg-core/src/catalog.rs::validate:1410-1420`). The only compute reuse is the coarse whole-publication copy in `crates/cpg-core/src/rebuild.rs::copy_publication:295-303`, admitted by `semantic_digest` + `catalog_compilation.input_digest` + embedding spec (`rebuild.rs:163-178`). The coarse digest over actual fact bytes (`rebuild.rs::input_digest:96-132`) is recorded in the receipt and never compared. This matches the PR5 review's explicit "correctness-first cache contract" and its decision "A finite stage cache is sufficient" [I].
2. **Stage/artifact dependencies are implicit outside the catalog stage.** Only normalization/association declare their input tables (one macro binds load and key: `catalog.rs::catalog_inputs!:506-546`, `evidence.rs::evidence_inputs!:51-73`). `relations!` `deps` are hand strings that nothing checks against SQL. Output membership is implicit: analysis tables a writer forgot are silently published empty (`attempt.rs:1646-1651`), and unwritten analysis tables read as typed empty relations (`attempt.rs:737-740`). `catalog_members.brief_status` is written back from Stage F (`attempt.rs:1568-1592`). `Step.dependencies` are receipt-only [I].
3. **Identity has two compiler digests and two generation keys, neither exactly aligned with canonical output.** `semantic_digest` (producer id, reuse) excludes `crates/cpg-core/src/evidence.rs` and `catalog_domains.rs`, the producers of canonical `catalog_{artifacts,spans,scenarios,deployments,associations}` and `catalog_selection_domains` (`crates/cpg-core/build.rs:103-125`); `compiler_digest` (snapshot rows, `content_digest`) includes serving-only code. `content_digest` hashes run ids, whose extractor part is a hand-bumped `EXTRACTOR_OUTPUT_VERSION`, not fact bytes. Both the bundle key and the PG `generation_digest` include the random `snapshot_id`, so identical content never shares a generation. PG `generation_digest` does cover every served row (row-hash receipts) and artifact, so it is a sound key for per-generation serving caches [I].
4. **Registries: `table!` already owns the whole Delta side; the serving side is a second hand authority.** 201 `table!` declarations = 57 raw + 22 derived + 121 analysis in three hand-ordered macros, plus `snapshots`. 18 catalog serving files derive from `table!` (`crates/cpg-schema/src/catalog.rs::serving_files:128-161`); 49 serving files re-declare schemas and SQL by hand, `retrieval_units` has no `table!` at all, PG DDL is generated once and then frozen and hand-edited with no drift test, and there are ~15 hand relation-name inventories and two already-diverged referential registries (`rules::REFERENCES` vs `serving_projection::foreign_keys`) [I].
5. **Serving rebuilds per request and hydrates sequentially.** No hydrated-object or prepared-selection cache exists. `PreparedCatalog` is rebuilt from full-generation loads on every `find_operations`/`search_operations`/`compare_operations` request and page. Sequential round trips: `get_capability` 10; `catalog_record` 9+2L; `get_operation` packet 15+2L to 20+2L; `compare_operations` up to ~58; `search_evidence` 2–3 + k..2k (N+1). The native executor (29 IPC files) exists only in behavioral generations, is loaded whole at startup and serves only `inspect_value_paths`. `nodes`/`edges` are not served. Missing/corrupt generation or artifacts refuse at startup; request-time failures are typed refusals, with four masking/reporting gaps [I].
6. **CI fidelity gaps that affect served claims:** G1 unknown/unavailable default state becomes a definite `false` and can yield `Contradicted/ClosedAbsence`; G2 association basis is lost in composed deployments, selection deployment observations, domain contexts and `retrieval_subjects`; G5 BM25 parameters sit outside the retrieval profile digest; G8 a joint-work budget hit becomes `NotEstablished` with `evaluation_complete` hard-coded `true`; G18 catalog compilation has no cardinality budget; G19 heuristic revisions are absent from served identity [I].
7. **Claims:** E12 confirmed/refined; E13 confirmed/refined (behavioral-only, dual-stored, eager); E14 confirmed (operator report path, not MCP); E15 confirmed/refined (per-relation batches, awaited sequentially); E16 refined (separation by crate boundary, not by acquisition phase); E17 confirmed for serving, refined for compile.

## 1. CI fact/fidelity table (catalog, evidence/association, scenario/deployment, retrieval)

### 1.0 Path legend (sections 1–2)

**Path legend.** Each citation has the form `path::symbol:line`, with these prefixes:

| Prefix | Directory |
|---|---|
| `cs/` | `crates/cpg-schema/src/` |
| `cc/` | `crates/cpg-core/src/` |
| `ce/` | `crates/cpg-extract/src/` |
| `lp/` | `crates/lctx-postgres/src/` |
| `lq/` | `crates/lctx-postgres/queries/` |
| `lm/` | `crates/lctx-postgres/migrations/` |
| `la/` | `crates/lctx-analytics/src/` |
| `py/` | `python/lctx_mcp/src/lctx_mcp/` |

`D::` stands for `docs/design/sections/api-and-evidence-product.md`.

### 1a. Upstream inputs consumed by the catalog families

These are summary rows only; this review does not cover the input families themselves.

| relation | provider/derivation | fidelity | coverage & unknowns | identity | consumers |
|---|---|---|---|---|---|
| `public_paths` | DataFusion SQL `cs/public.rs::public_paths_sql:~100-131`, computed once per attempt from roots in `cc/attempt.rs::finish:729-734` [I] | Exact over the export and MRO relations. `preferred` is a rank tie-break by `own DESC`, dot depth and path `cs/public.rs::public_paths_sql:120-126` [I] | Only paths under the roots `cs/public.rs::public_paths_sql:123-124` [I] | Key `access_path` `cs/serving_projection.rs::unique_key:705` [IC] | Member creation `cc/catalog.rs::derive_indexed:714-770` [I]; packet `own_paths` `lp/packet.rs::operation_packet:102-150` [I] |
| `catalog_export_candidates` (query relation) | `LEFT JOIN ranked` over public_names and declarations, `cs/public.rs::export_candidates:177-181` [I] | Unresolved observations keep `declaration_node_id = NULL`. The LEFT JOIN keeps them as unknowns [I] | A NULL candidate makes `provider_resolved = false`, which makes the signature contexts incomplete (`cc/catalog_domains.rs::derive:36-46,130-140`) [I] | none (query row) | `cc/catalog.rs::derive_indexed:813-847,853-877` [I] |
| `signatures`, `parameters` (derived) | SQL `cs/derived.rs::Signatures::sql:323-409` and `Parameters::sql:431-460` [I] | `reason` is a BoundaryReason (`missing_evidence`, `unreachable_in_context`, `provider_disagreement`). `form` is NULL when Pysa does not describe the signature `cs/derived.rs::Signatures::sql:397-399` [I] | Missing provider data is an explicit reason, not an absence [I] | Signature key `signature_node_id`; parameters keyed by (signature, ordinal) [IC] | `cc/catalog.rs::derive_indexed:1004-1086` [I] |
| `record_fields`, `record_field_syntax` | `ce/types.rs:~1093`, `ce/walk.rs:~681` [IC] | Provider record model (`record_kind`, `has_default`, `required`, `init`) [IC] | More than one syntax row for a field is treated as uncertainty (`cc/surface.rs::derive:788-796`) [I] | `node_id` | `cc/surface.rs::derive:777-875` [I] |
| `captured_artifacts` | `ce/lib.rs::capture:86-111`. Call sites: invalid UTF-8 `1019-1038`, METADATA/entry points `1039-1081`, documents `1083-1116`, configuration assets `1125-1145` [I] | `alignment` is `exact` for distribution metadata, `mapped_with_evidence` for corpus documents and configs, `unknown` for invalid UTF-8 [I]. A document captured outside a Corpus origin gets alignment `mapped_with_evidence` with provenance "source mapping unavailable" (`ce/lib.rs:1098-1106`). This branch is latent because documents only come from corpus runs (`ce/lib.rs:332`) [I] | Metadata interpretation failures stay `interpretation=failed` `ce/lib.rs:1047-1057` [I] | `cs/evidence.rs::artifact_id:247-253` [I] | `cc/evidence.rs::PreparedEvidence::derive:407-420,1012-1168` [I] |
| `documents`, `passages`, `code_blocks`, `mentions` | `ce/docs.rs:583,619,688` [IC] | Mention `class` is `exact` or `lexical`. Modality is Definite only for an exact mention with one target per span `ce/docs.rs:674-677` [I] | Mentions outside any passage are dropped `ce/docs.rs:671-673` [I] | fact rows | `cc/evidence.rs::derive:458-564,883-921` [I] |
| `edges` (CallTarget/SiteTarget), `pysa_calls`, `arguments`, `facts` | graph and extractor [IC] | Per edge: `modality` and `unresolved_reason` from the evidence fact [I] | A missing evidence fact skips that support edge silently (`cc/evidence.rs::derive:773-775`) [I] | `edge_id` | `cc/evidence.rs::PreparedEvidence::new:177-185`, `derive:631-882` [I] |
| `coverage`, `boundaries` | extractor coverage rows [IC] | `CoverageStatus`, `BoundaryReason` codebooks [IC] | Drive `source_complete` and `signature_coverage` (`cc/catalog_domains.rs::derive:106-129`) and scenario parse status (`cc/evidence.rs::parse_status:241-259`) [I] | – | same [I] |

### 1b. Catalog contract family (`family = Findings`; mandatory in both profiles, `cs/catalog.rs::mandatory_table:309-322` [I])

| relation | provider/derivation | fidelity | coverage & unknowns | identity | consumers |
|---|---|---|---|---|---|
| `catalog_compilation` `cs/catalog.rs:77-82` | `cc/attempt.rs::finish:773-785` [I] | Selection and provenance record only | Exactly one row, checked at bundle time `cc/bundle.rs:901-913` [I]. Capabilities come from the profile `cs/catalog.rs::Capabilities::for_profile:39-46` [I] | `input_digest` = profile + roots + analysis variant digest `cc/catalog.rs::CompileInputs::digest:41-52` [I]. No catalog-derivation or heuristic revision is included | bundle manifest, `cc/rebuild.rs:149-150,355-359`, `cc/catalog.rs::validate:1410-1420` [I] |
| `catalog_members` `cs/catalog.rs:83-90` | `cc/catalog.rs::derive_indexed` from three sources: public_paths `714-732`, public_names `784-799`, class-member enumeration `905-917` [I] | `resolution` is either `source_known_effective_unresolved` or `unresolved`; the vocabulary has no "resolved" value `cs/catalog.rs::vocabulary:275-277` [I]. `kind` is free text drawn from DeclarationKind, SymbolKind or `"unknown"` (`cc/catalog.rs:728,792-795,913`) [I]. The DDL has no CHECK on it `lm/202609280009_catalog.sql:18` [IC]. The row carries no `source_fact_id`; attribution comes only through bindings | `operation_node_id` is NULL for name-only and class-member observations [I]. `brief_status` defaults to `not_requested` [I] | `IdHasher("public-member").id(export).str(path)` `cc/catalog.rs::member_id:417-422`. Here `export` is `public_paths.export_node_id` or `IdHasher("export").id(release).str(access_path)` `:780-783,872-875` [I] | Domains `cc/catalog_domains.rs:28`; evidence `cc/evidence.rs:384-406`; retrieval `cs/retrieval/catalog.rs::derive:61-183`; PG `lctx_serving.catalog_members` `lm/…0009:11-24`; tools find/search/compare/get_operation/browse `lp/journeys.rs:25`, `lp/packet.rs::resolve_members:255-282`, `lq/packet_resolve.sql` [I] |
| `catalog_bindings` `cs/catalog.rs:91-97` | `cc/catalog.rs::derive_indexed:737-769,800-847,928-941` [I] | `role` vocabulary: selected_source, overload, shadowed_source, stub_source, source_alternative, provider_public_observation (`cs/catalog.rs:278-285`, DDL `lm/…0009:39`). Alternatives stay as rows, not a union [I] | `declaration_node_id` is NULL for `provider_public_observation`. Candidates without a declaration are skipped `:817-822` [I] | `IdHasher("public-binding").id(member).id(fact).str(role)` or `("public-observation").id(member).id(name_fact)` `:758-762,803-806,931-935`. Role is part of identity [I] | Domains `:30-51`; selection `cs/selection/catalog.rs::PreparedCatalog:285`; retrieval anchors `cs/retrieval/catalog.rs:135-143`; hydration `lp/hydration.rs::catalog_invocation:100-108` [I] |
| `catalog_signatures` `cs/catalog.rs:98-106` | Source rows `cc/catalog.rs:1004-1029`; provider constructors `1087-1143` [I] | `role` is source, overload or provider_constructor. `form` falls back to `s.form.map_or("source_list",…)` `:1024`: the catalog vocabulary adds `source_list` to the SignatureForm codebook (`cs/catalog.rs:287` vs `cs/codebook.rs:335-339`) [I]. `reason` keeps the provider BoundaryReason `:1026` [I] | `return_annotation` is set only when exactly one declared return observation exists `:1292-1307`; otherwise NULL, meaning unknown [I] | Source: `signature_node_id`. Provider: `IdHasher("provider-signature").id(synthetic).i64(index)` `:1116-1119` [I] | Domains signature contexts `cc/catalog_domains.rs:146-176`; facets `cc/catalog.rs:206-251,313-337`; selection; retrieval; hydration via `fetch_contract`, which nulls `docstring` `lp/repository.rs::fetch_projection:291-307` [I] |
| `catalog_parameters` `cs/catalog.rs:107-117` | Source `cc/catalog.rs:1030-1085`; provider `1144-1166` [I] | Source and provider observations kept in separate columns. `default_state` is absent, literal_none, literal, source_expression, optional_expression_unavailable or unknown (`cc/catalog.rs::default_value:425-453`, DDL `lm/…0009:93`) [I]. `reason` = provider_disagreement [I] | Provider-only parameters get `default_value(None, required)`, which yields `optional_expression_unavailable` or `unknown` `:1146` [I]. See Gap G1 for how selection treats these | Key (signature_id, ordinal) [IC] | Selection signature predicates `cs/selection/catalog.rs::evaluate:840-908`; retrieval text `cs/retrieval/catalog.rs:87-99`; field links `cc/surface.rs:885-1000`; packet [I] |
| `catalog_constructors` `cs/catalog.rs:324-329` | `cc/catalog.rs:944-994,1170-1191` (first constructor along the MRO; a rebound `__init__` stops the search `:981-983`) [I] | `own` and `ancestry_fact_id` attribute inherited contracts [I] | A class with no owner gets no row. The facet status then reports `NotAnalyzed "constructor contract unavailable"` `:305-312` [I] | Key (class, signature) [IC] | Domains `:150-153`; retrieval `:75-77`; hydration `:118-126` [I] |
| `catalog_evidence` `cs/catalog.rs:118-125` | Declaration bytes `cc/catalog.rs:1309-1330`; specificity citations `1331-1399` [I] | `role` is declares, surface, configuration or reader (`cs/catalog.rs:300`, `lm/…0010`). **Every field-link citation uses role "reader"**, including exact_storage statements `:1361-1371` (Gap G3) [I] | Rows are silently skipped when source text is NULL (non-UTF-8) `:1312,1377-1379` or when the syntax node is not found `:1334,1346-1349,1362` [I] | `IdHasher("catalog-evidence").id(decl_fact)` `:1320`; `("catalog-specificity-evidence").id(subject).id(fact).str(role)` `:1385-1389` [I] | Retrieval `Anchor::Catalog` `cs/retrieval/catalog.rs:107-115,144-152`; `source_fact` span associations `cc/evidence.rs:975-1011`; `lq/unit_original_span.sql` (LIMIT 2, picks `spans[0]`) `lp/evidence.rs::get_retrieval_unit:427-443` [I] |
| `catalog_types`, `catalog_type_args`, `catalog_type_observations` `cs/catalog.rs:166-183` | Worklist type closure `cc/catalog.rs:1193-1291` [I] | TypeTermKind includes `truncated` (`lm/…0009:135`) [IC]. Conflicting observations cause a hard error `:1197-1206,1277-1288` [I] | `declared` flag on observations [I] | term = `type_terms.node_id` | Selection `type_match` and `literal_domain` `cs/selection/catalog.rs:1217-1269`; hydration closure `lp/hydration.rs::catalog_record:226-267` [I] |
| `catalog_surfaces` `cs/catalog.rs:331-342` | `cc/surface.rs::derive:651-776`, `aspects:251-367` [I] | Separate typed aspects: binding_mode, accessor_role, protocol, registration, admission (body_preserved or withheld). `reason` is free text with no DDL CHECK `lm/…0010:30` [IC]. `registration` is FastMCP-only and gated on `pilot` (Gap G4) [I] | A decorator in a file without text is dropped, but `ordinal` still advances `:659-662` [I]. Unresolved targets get reason "unresolved or unsupported decorator" `:352-358` [I] | Key (declaration, ordinal); ordinal is the source position [I] | InvocationForm and member_kind `cs/selection/catalog.rs:482-516,750-810`; domain `preserved` `cc/catalog_domains.rs:130-140`; field-link exactness `cc/surface.rs:931-956`; packet [I] |
| `catalog_configurations` `cs/catalog.rs:343-354` | `cc/surface.rs::derive:777-875` [I] | `default_state` has 7 values including factory_expression (`cs/catalog.rs:258-266`). Conflicts are named in `reason` (`:796,841,845`) [I] | `syntax_fact_id` is NULL when a field has more than one syntax row [I]. A field without syntax has no configuration evidence row (`cc/catalog.rs:1345-1359`) [I] | Key (field_id = record_fields.node_id, source_fact_id) [I] | Domains `cc/catalog_domains.rs:177-222`; selection configuration predicates `cs/selection/catalog.rs:911-1054`; retrieval text `cs/retrieval/catalog.rs:117-134` [I] |
| `catalog_field_links` `cs/catalog.rs:355-363` | `cc/surface.rs::derive:876-1087`, `direct_store:432-631` [I] | `kind` is declared_parameter, exact_storage or exact_reader. A name-equality link is conservative and labelled `declared_parameter` with reason "runtime storage unproved" `:972-998`. `exact_reader` carries "intervening mutation is not excluded" `:1082-1084` [I] | exact_storage requires a dataclass, a simple decorator and fields, no hooks, and a plain base `:957-971` [I] | `IdHasher("catalog-field-link").id(field_fact).id(sig).i64(ord).str(kind)`; reader: `("catalog-field-reader").id(link).id(node_fact)` `:979-984,1075-1078` [I] | Selection ConfigurationRelationship `cs/selection/catalog.rs:973-1019`; domains; FK to `catalog_parameters` `cs/serving_projection.rs::foreign_keys:~773-777` [I] |
| `catalog_selection_domains` `cs/selection/catalog.rs:35-41` | `cc/catalog_domains.rs::derive:15-333` [I] | Per domain: `corpus_complete` and `analyzer_complete`; per context: `complete` + evidence. A complete context without evidence is rejected `cs/selection/catalog.rs::validate_detail:1382-1384` [I]. Relationships, Scenarios, SourceArtifacts and ReleaseDeclarations are never closed, and ConfigurationFields is never analyzer-complete `cc/catalog_domains.rs:282-311` [I] | `module` and `class_owner` are NULL when unknown [I]. `module` may be chosen by lexicographic `min` (Gap G10) `:57-83` [I] | `IdHasher("selection-domain-v1").id(member).str(detail_json)` `:324-327`; checked by `canonical_domain:62-85` [I] | PG `lq/selection_domains.sql` (projects only the requested domain kinds, keeps `domain_id`); classifier; browse `lq/browse_ownership.sql` [I] |

### 1c. Evidence, scenario and deployment family (`cs/evidence.rs`, ADR-0076)

| relation | provider/derivation | fidelity | coverage & unknowns | identity | consumers |
|---|---|---|---|---|---|
| `catalog_artifacts` `cs/evidence.rs:214-221` | `cc/evidence.rs::derive:407-457`. Captured artifacts are copied; source files are recaptured, except the DocBlock role [I] | `alignment` is `exact` for the Release role, `mapped_with_evidence` otherwise `:439-444` [I]; DDL CHECK `lm/…0011:20` [IC]. `provenance` is free text | Retention keeps only artifacts referenced by a span or task `:1169-1203` [I] | `artifact_id` (release, path, digest). Bytes, digest and identity are re-verified `cs/evidence.rs::validate:273-282` [I] | `get_evidence` `lp/evidence.rs::read:173-218`; `lq/artifact_alignment.sql` → SourceAlignment; retrieval text [I] |
| `catalog_spans` `cs/evidence.rs:222-226` | `cc/evidence.rs::span:85-112`, called from passages, fences, enclosing context, imports, modules, arguments, catalog source and acquired artifacts [I] | `extraction` is a free-text label with no DDL CHECK `lm/…0011:39` [IC]. When a span is registered twice, the lexicographically smaller label is kept `:95-99` (Gap G15) [I] | Bounds and UTF-8 boundaries are validated `cs/evidence.rs::validate:283-300` [I] | `IdHasher("evidence-span").id(artifact).i64(start).i64(end)` `cs/evidence.rs::span_id:254-260` [I] | Scenarios, deployments, associations, retrieval, get_evidence [I] |
| `catalog_scenarios` `cs/evidence.rs:227-232` | Doc fences `cc/evidence.rs:497-524`; enclosing call-site context `649-770`; option bindings `839-881`; post-processing `1177-1200` [I] | `ScenarioDetail` holds context, intent, four independent check axes, requirements and extraction. `checks.execution` must be `not_run` `cs/evidence.rs::validate:345-348` [I]. Parse status: `failed` on a syntax boundary, `passed` on complete Syntax coverage, `blocked` otherwise `cc/evidence.rs::parse_status:241-259` [I] | Option bindings and requirements are capped at 128. Counts go to `omitted_*` and `context=truncated` `:1192-1198` [I]. The context is always `ContextDependent` because the requirements list is never empty `:677,738-742` [I] | `IdHasher("contextual-scenario-v1").id(primary_span)` `:503-505,673-675`, checked `cs/evidence.rs::validate:316-322` [I] | Selection Scenario contexts; retrieval Scenario units; `lq/evidence_search_scenario.sql`; `lq/evidence_eligible.sql` intent filter; get_evidence [I] |
| `catalog_deployments` `cs/evidence.rs:233-238` | Snippet fences `cc/evidence.rs:525-538`; captured metadata `1012-1060` [I] | `DeploymentDetail.interpretation` is passed, failed or not_run. Snippets are `not_run` with a diagnostic `:529`. A task receipt must byte-equal the original artifact and match the source and environment digests `cs/evidence.rs::validate:367-392` [I] | Configurations or assets without observations get a synthetic `original_configuration`, `not_run` `:1033-1037` [I] | `("deployment-metadata").id(span).i64(ord)` or `("deployment-snippet").id(span)`; the derivation is checked `cs/evidence.rs::validate:354-366` [I] | Selection DeploymentDeclaration; retrieval DocumentationDeployment units; get_evidence [I] |
| `catalog_associations` `cs/evidence.rs:239-245` | `cc/evidence.rs::associate:117-151`. Sources: snippets `539-562`, call sites `772-837`, mentions `883-921`, catalog source `975-1011`, config refs `1060-1098`, tasks `1099-1132`, release `1133-1166` [I] | `role`, `basis` and `intent` come from closed vocabularies (`cs/catalog.rs:217-248`, DDL `lm/…0011:92-96`) [I]. The validator rejects a relabel from candidate to resolved and from release to member `cs/evidence.rs::validate:435-437,481-485` [I]. **Composed bases drop the fidelity of the member link** (Gap G2) [I] | Each association has exactly one of member or release `:416-418` [I]. `support` holds at most one provider edge per identity `:487-496` [I] | `IdHasher("evidence-association")(member?, release?, evidence, kind, role, basis, site?, intent)`, then `("evidence-association-supported").id(id).id(edge)` `cs/evidence.rs::validate:438-499` [I] | Domain Scenario and Source contexts `cc/catalog_domains.rs:223-256`; selection Relationship and Deployment `cs/selection/catalog.rs:657-700,1143-1214`; retrieval subjects `cs/retrieval/catalog.rs:200-218`; evidence page `lp/evidence.rs::association_page:287-343`; demonstrations `lq/packet_demonstrations.sql` [I] |

### 1d. Operation facet tables read by selection and browse (catalog profile)

| relation | provider/derivation | fidelity | coverage & unknowns | identity | consumers |
|---|---|---|---|---|---|
| `operations`, `operation_facets`, `operation_facet_status` | `cc/catalog.rs::derive_population:118-385`; catalog-profile call `cc/attempt.rs:1349-1360` [I] | Facet verdict is the minimum over its sources (`put`) `:153-157` [I]. Status is a per-facet verdict and reason `:293-381` [I] | Behaviour-dependent facets are forced to `NotAnalyzed "behavioral analysis not requested"` `:363-373` [I]. In the catalog profile, `behavior_status` is NotAnalyzed with reason NotRequested `:130-145` [I] | Key (node, facet) | FacetMembership `cs/selection/catalog.rs:602-656`; browse FacetValues `lp/journeys.rs:132-206`; packet `incomplete_facets` `lp/packet.rs:111-133`; Facets section [I] |

### 1e. Retrieval family (`cs/retrieval.rs`, `cs/retrieval/catalog.rs`, ADR-0077)

| relation | provider/derivation | fidelity | coverage & unknowns | identity | consumers |
|---|---|---|---|---|---|
| `retrieval_units` `cs/retrieval.rs::files:270-274` | `cs/retrieval/catalog.rs::derive:44-293` via `cc/retrieval.rs::derive:8-10,prepare:290-297` [I] | A rendering, not evidence (`cs/retrieval.rs:321`). `family` is one of 4 (`VIEWS:29-50`). Unknown values are rendered as the literal strings "unknown" or "<unresolved>" `cs/retrieval/catalog.rs:87-99,122-130` [I] | Hard refusal above 200k units or 128 MiB of text `:289-291` [I]. Anchor closure, subject closure and rendering equality are re-validated `cs/retrieval.rs::validate_projection:386-450` [I] | `IdHasher("retrieval-unit-v1").str(family).id(source_key)` `:132-139`. Source keys: member_id, `("retrieval-member-source")(member, evidence)`, scenario_id, deployment_id, span_id [I] | `lq/unit_detail.sql` → get_evidence(RetrievalUnit) `lp/evidence.rs:355-484`; `lq/evidence_eligible.sql`; `lq/evidence_search_unit.sql`; packet `evidence` pointer `lp/packet.rs:186` [I] |
| `retrieval_subjects` | Same derive, `subjects:200-218` [I] | **No basis column.** A candidate or ambiguous association becomes a plain subject (Gap G2) [I]. A span with no association falls back to its release as subject `:211-216` [I] | Either member or release, never both (`cs/retrieval.rs::validate_projection:456-464`) [I] | (unit, member?, release?) | Lexical and vector member votes `py/retrieval.py::UnitLexical.__init__:143-159`, `lq/unit_ranks.sql`; evidence subject filter `lq/evidence_eligible.sql` [I] |
| `retrieval_fragments` | `cs/retrieval.rs::fragments:141-175` (window 4096 at `cc/retrieval.rs::materialize:78`), status at `:155-169` [I] | `embedding_status` is embedded, not_requested, token_refused, token_unavailable or embedding_failed. Rust validates it `cs/retrieval.rs:493-499`; the DDL has no CHECK `lm/…0012:65` [IC] | Every unit is fragmented; embedding is admitted per fragment [I] | `IdHasher("retrieval-fragment-v1")(unit, RENDER_REVISION, ordinal, content_digest)` `:157-162` [I] | Lexical corpus, vector ranks, get_retrieval_unit [I] |
| `retrieval_vectors` | `cc/retrieval.rs::materialize:170-264` [I] | `input_hash` = sha256 of the spec-document text, re-verified `cs/retrieval.rs:536-554` [I] | The embedded set equals the vector set (closure) `:515-517` [I] | `fragment_id` | `lq/unit_ranks.sql`, `lq/evidence_unit_ranks.sql` [I] |
| `retrieval_receipt` | `cc/retrieval.rs::materialize:265-277` [I] | `input_digest`, `view_revision`, `render_revision`, `spec_hash`. **No `FUSION_REVISION` or lexical parameters** `cs/retrieval.rs::RetrievalReceipt:99-105` [I] | One row, bound to the snapshot `cs/retrieval.rs::validate_snapshot:569-589` [I] | `IdHasher("retrieval-inputs-v1")(json units)` [I] | Import validation `cs/serving_projection.rs:912-914` [I] |
| `retrieval_profiles` (PG policy) | `lp/profiles.rs::Policy::exact:22-30` [I] | Digest over `{format, route, metric, ties, rrf_k}` `:37-43` [I] | Only the exact route is admitted `:31-36` [I] | sha256 of the canonical policy | `RetrievalMetadata.profile`, ranked cursor `lp/selection.rs:539` [I] |


## 2. CI analysis-record table (catalog derivations and serving journeys)

| # / analysis | question answered | inputs | method | exact / conservative / heuristic (governance) | budgets / partial | output & evidence linkage | consumers (tools) |
|---|---|---|---|---|---|---|---|
| A1 Contract assembly `cc/catalog.rs::derive_indexed:629-1406` | Which public exposures exist, and what bindings, signatures, parameters and constructors does each have | CatalogFacts `:474-504`, public_paths | Rust joins over prebuilt indexes (`CatalogIndex:557-593`); MRO chains `:661-688` | **Exact** for attributed rows, with alternatives kept `:737-769`. The `source_list` fallback label is a catalog vocabulary extension `:1024`. No named revision; the stage cache keys on a code digest (`cc/stage_cache.rs::key:14-54`) [I] | **No internal work caps.** Several O(n·m) scans: `:985-987,1334,1362`; `cc/surface.rs:903-956`. Errors only on invariant violations [I] | Rows carry `source_fact_id`. Declaration bytes go to `catalog_evidence` `:1309-1330`. Reconstruction is compared with stored rows at publication `:1410-1498` [I] | All catalog tools |
| A2 Surface normalization `cc/surface.rs::derive:651-776`, `aspects:251-367` | What each decorator establishes: binding, accessor, protocol, registration, body admission | syntax_nodes (Decorator), name roots, descriptors, bindings | ruff re-parse plus lexical-root resolution `resolved:94-135`; admission requires one bare builtin plus provider agreement `admitted_descriptor:65-90` | Exact target resolution (conflicts leave it unresolved `:129-131`). **Heuristic**: a hard-coded name table `:252-359`. FastMCP registration is gated on `releases.distributions == "fastmcp==4.0.5"` `:641-644`, with `registration_target:213-242`. It is unnamed, unversioned and has no parameter digest (Gap G4) [I] | None. A missing file text drops the decorator silently `:660-662` [I] | Each row cites its decorator `source_fact_id`; `surface` evidence rows at `cc/catalog.rs:1333-1344` [I] | get_operation, find/search_operations (InvocationForm, MemberKind) |
| A3 Configuration normalization `cc/surface.rs::derive:777-875` | Declared record fields and their default or factory expression, never evaluated | record_fields, record_field_syntax | `literal()` decodes scalars without evaluation `:16-47`. Only these known callees are recognized for field defaults: dataclasses.field, attrs.field, attr.ib, pydantic.Field `:808-817` | Exact spelling. Conflicts are named in `reason` `:796,841,845`. The callee list is unversioned [I] | None | `configuration` evidence rows when exactly one syntax row exists `cc/catalog.rs:1345-1359` [I] | get_operation, Configuration* predicates, ApiOptions text |
| A4 Field links `cc/surface.rs:876-1087`, `direct_store:432-631` | Which constructor formal initialises or reads a field | constructors, parameters, signatures, syntax_nodes, roots, descriptors, ancestry | Straight-line constructor proof: rejects calls, branches, augmented assignment, yield, descriptors and hooks `:460-499,903-956` | **exact_storage** only when all 11 conditions of `:957-971` hold; otherwise **declared_parameter** (conservative, with reason). **exact_reader** is a self-attribute read in a body-preserved method `:1003-1087` [I] | None | `source_fact_id` points to the store statement or read node. Evidence is cited with **role "reader" even for storage** (Gap G3) [I] | ConfigurationRelationship, get_operation |
| A5 Type closure `cc/catalog.rs:1193-1291` | Which structural type terms are reachable from the selected subjects | type_terms, type_term_args, type_observations | Indexed worklist | Exact; an identity conflict is a hard error [I] | No cap on closure size [I] | `source_fact_id` on each row [I] | ParameterType, ConfigurationLiteral, packet |
| A6 Legacy facets and status `cc/catalog.rs::derive_population:118-385` | Facet membership and per-facet completeness | operation_sources, contracts, concept attributes | Verdict is the minimum over sources | Established or Unknown or NotAnalyzed, with a reason. Parameter status is Established only when every signature form is `list` or `source_list` `:313-337` [I] | – | No evidence column (the verdict string only) [I] | FacetMembership, browse FacetValues, packet `incomplete_facets` |
| A7 Artifacts and spans `cc/evidence.rs:407-481` | Which original bytes back each piece of evidence | captured_artifacts, source_files, documents, passages | copy plus content address | Exact bytes; digest re-verified [I]. The alignment rule is a two-way role switch `:439-444` [I] | Artifact retention `:1201` | – | get_evidence |
| A8 Scenarios and intent `cc/evidence.rs::derive:483-524,649-770`, `intent:294-380` | What enclosing context an API use sits in, and what that use intends | blocks, syntax_nodes, bindings, edges, coverage, boundaries | Walks ancestors. Intent comes from a `with pytest.raises`/`assertRaises` context `expected_manager:260-292` or a `pytest.mark.skip/skipif/xfail` decorator `:337-342`; the SourceRole sets the default | **Heuristic but conservative**: hard-coded manager and decorator names, unversioned. Resolution must be exact (`resolved()`, or a Definite unresolved-free edge `:275-290`) [I] | Options and requirements are truncated at 128 with explicit `omitted_*` and `truncated` `:1192-1198` [I] | ScenarioDetail lists original spans and requirements, each with an evidence span `:677,710,731` [I] | search_evidence, get_evidence, ScenarioIntent/Check |
| A9 Call-site associations `cc/evidence.rs:631-837` | Which member a usage site invokes | edges, facts, pysa_calls, members_by_target (+ context definitions `:589-628`) | Edge → member join. Basis is `resolved_target` only for a Definite fact with an unresolved-free call; otherwise `candidate_targets` `:809-815` | **Exact or conservative**. Candidates are never relabelled as resolved (validated at `cs/evidence.rs:481-485`) [I] | – | One support edge per association, carrying edge, fact, modality, phase and coordinates `:783-803` [I] | Evidence section, demonstrations (`resolved_target` only, `lq/packet_demonstrations.sql`), Relationship predicate |
| A10 Mention associations `cc/evidence.rs:883-921` | Which member a documentation passage documents | mentions, passages, members | Lexical: by access path or qualified name | Basis is `exact_textual_reference` iff class is Exact and exactly one member matches, else `ambiguous_textual_mention`. **The extractor's per-span Candidate modality is ignored** (Gap G12, vs `ce/docs.rs:674-677`) [I] | – | Span evidence id [I] | search_evidence, Relationship |
| A11 Composed deployment associations `cc/evidence.rs:1060-1166` | Which member a configuration, task receipt or release declaration concerns | deployments, scenarios, existing scenario associations | Transitive: config `referenced_path` → artifact → scenarios → members. Task: `target_artifact` → scenarios → members | **Mislabel.** The basis `explicit_config_reference` or `task_observation` names the first hop only. Members reached through `candidate_targets` scenario associations are carried over without a basis filter `:1077-1084,1111-1118` (Gap G2) [I]. The release association is exact by distribution name `:1133-1166` [I] | – | Deployment evidence id and `site_id` for tasks; the member hop is not cited [I] | Evidence section, DeploymentDeclaration |
| A12 Selection domains `cc/catalog_domains.rs::derive:15-333` | For each member and domain, the applicable contexts and whether absence can be concluded | contracts, contextual evidence, coverage | Rust | Conservative. A complete signature context requires Exports and Signatures coverage, a preserved body, a resolved provider and a list form `:161-164`. Relationships and other evidence domains never close `:288-311` [I]. `module` is chosen by a lexicographic `min` tie-break `:62,83` (Gap G10) [I] | – | Every complete context cites Fact witnesses `:165-172`; closure is cited as a Domain witness [I] | find/search/compare_operations, browse |
| A13 Classifier `cs/selection/catalog.rs::PreparedCatalog::classify:332-398`, `evaluate:597-1216`; `cs/selection.rs::classify:38-156`, `joint:203-306`, `aggregate:307-333` | Whether each member supports, contradicts, leaves unresolved or conflicts with each requirement, and jointly | Inputs `:88-107` from PG `lp/selection.rs::prepare_selection:201-344` | Per-context observations reduced by (context, basis). Quantifiers are AnyApplicable or AllApplicable. Joint uses BDD conjunction `:231-288` | Named `POLICY_REVISION=1` `cs/selection.rs:9`, included in `selection_digest:335-353` and the cursor `lp/selection.rs:86-96` [I]. Exact for the typed predicates; **unknown collapses into false in ParameterDefaultState** (G1); **deployment "suggests" is relabelled source_declaration** (G2b); MemberKind vocabulary mismatch (G11) [I] | 16 requirements; 200k rows per input and 2M total; 64 MiB detail `:181-214`; 2M work `:349-373`; 64 MiB evidence `:388-393`; `MAX_OBSERVATIONS`/`MAX_CONTEXT_WORK` `cs/selection.rs:10-11`. Hitting a cap is a hard `resource_refused`, **except joint(), which returns `NotEstablished` without a reason** `:255-257`. `evaluation_complete` is hard-coded `true` `:147` (G8) [I] | `RequirementResult` with Positive, Negative or Conflict witnesses (Fact, Catalog, FieldLink, Association, Original, Domain, Facet) and coverage [I]. Fact witnesses cannot be opened through tools, and projection validation skips them `:1511` (G14) [I] | find_operations, search_operations, compare_operations |
| A14 PG preparation and paging `lp/selection.rs:18-158,201-344` | Grouped pages and the eligible set | `lq/selection_domains.sql`, dependency tables `cs/wire/requirements.rs::dependencies:366-429`, `lq/artifact_alignment.sql` | Only the requested domain kinds are loaded. The classifier runs on CPU | "promoted" means exact `access_path == query.trim()` `:328-332`, an explicit primary ordering [I] | 64 MiB retained input `charge_input:186-199`; page limit 1..100 `:71` [I] | `SelectionResults` with group totals and `contradicted_count` `:140-154` [I] | find_operations `py/server.py:585-605` |
| A15 Retrieval rendering `cs/retrieval/catalog.rs::derive:44-293` | Which addressable text units exist for API, source, scenario and documentation/deployment | 12 catalog and evidence relations `:12-25` | Deterministic templates | **Heuristic rendering**, versioned by `VIEW_REVISION` and `RENDER_REVISION` `cs/retrieval.rs:9-10` [I]. Shadowed and alternative signatures are rendered with no binding-role label `:63-116` (G16) [I] | 200k units / 128 MiB, hard refusal `:289-291` [I] | Unit anchors: Declaration facts, Catalog evidence, Original refs. Per-line claims in ApiOptions text have no individual anchor; provider-only lines anchor only to a Declaration fact [I] | search_operations, search_evidence, get_evidence |
| A16 Fragments and embedding admission `cc/retrieval.rs::materialize:68-287` | Which fragments get vectors | units, embedder | fixed 4096-byte window | Exact status per fragment; the embedding-service failure is explicit `:183-189` [I] | 256 MiB session `:154` | Receipt `:265-277`; immutable replay `save:436-479` [I] | vector ranks |
| A17 Lexical channel `py/retrieval.py::Lexical:34-60`, `UnitLexical:140-222` | Keyword relevance | retrieval_fragments, retrieval_subjects | BM25 (bm25s, lucene, k1=1.5, b=0.75) over regex tokens. Identical (family, text) pairs are deduplicated before corpus statistics `:148-159` | **Heuristic.** Parameters are Python literals **outside the profile digest** (G5) [I] | 128 MiB ledger `FUSION_BYTES:63,185-186,198-199` [I] | (unit, fragment) winners [I] | search_operations, search_evidence |
| A18 Member fusion `cs/retrieval.rs::channel_winners:177-208`, `fuse:210-258`; `lp/selection.rs::finish_selection_search:425-543`; `py/retrieval.py::fuse_families:247-282` | Rank of eligible members across families | lexical winners, `lq/unit_ranks.sql` (exact cosine) | RRF with K=60 within each family, then an equal-weight family RRF. The Rust oracle re-checks the Python fusion `:471-496` | **Heuristic, governed.** `rrf_k`, ties and metric are in the profile digest `lp/profiles.rs:22-30`; `routing_reason "addressable_family_rrf_v1"` [I]. **The fallback reason is dropped from the response** `:540` (G9) [I] | 200k rows; 32 MiB IPC `lp/retrieval.rs:19`; 8 winners per member `:450` [I] | `RankedMember.winners`, a ranking witness kept separate from requirement evidence [I] | search_operations `py/server.py:291-328` |
| A19 Evidence search `lp/evidence_search.rs::search_evidence:17-244`; `cs/retrieval.rs::unit_channel_winners:745-775`, `fuse_units:783-812` | Which original documentation, scenario or deployment units match a query | lexical unit winners, `lq/evidence_eligible.sql`, `lq/evidence_unit_ranks.sql` | Best fragment per (unit, channel), then family RRF | Governed: `FUSION_REVISION` is included in the cursor target `:145`; `routing_reason "independent_unit_family_rrf_v1"` `:174`; fallback carries `channel_state` `:167-171` [I] | 200k population, 1..100 page, 32/256 KiB responses. An indivisible hit gets an explicit refusal `:224-240` [I] | `EvidenceHit`: header (≤16 anchors or subjects, `metadata_omitted`, `cs/wire/evidence.rs:103-116`), winners and scenario summary (counts only) [I]. **Subjects carry no association basis** (G2) [I] | search_evidence `py/server.py:673-729` |
| A20 Browse `lp/journeys.rs::browse_library:12-241` | Module, class or member outline; facet values; vocabulary | catalog_members, `lq/browse_ownership.sql`, operation_facets | In-memory grouping | Ownership taken from `DomainDetail` [I]. **Module scope drops unknown-module members without saying so** `:54-61,77` (G6). FacetValues omits `operation_facet_status` coverage `:132-206` [I] | Hydration byte budget; 200k; page ≤100 [I] | Counts only, no evidence citations [I] | browse_library `py/server.py:626-649` |
| A21 Compare `lp/journeys.rs::compare_operations:242-323` | Side-by-side selection over up to five named candidates | prepared selection, `resolve_members` | Reuses the classifier; no new inference | Exact name resolution. Ambiguous and NotFound cases are explicit `:262-312` [I] | 1..5 candidates `cs/wire/journeys.rs:264-273`; packet budget [I] | `CandidateSelection` witnesses plus signatures [I] | compare_operations |
| A22 Operation packet and sections `lp/packet.rs::operation_packet:34-252`, `section_page:284-493`; `lp/evidence.rs::association_page:287-343` | The invocation contract and bounded enrichment for one member | catalog record `lp/hydration.rs::catalog_record:185-273`, associations, facets | Hydration by key | Contract is exact. `effective_surface` is the constant "unresolved" `lp/hydration.rs:271` [I]. The evidence page includes **every** release-scoped association `lp/evidence.rs:294,300` (G13). An empty section is labelled `EmptyUnderCoverage` `lp/packet.rs:462-475` (G7) [I] | 32 or 256 KiB. Demonstrations capped at 2, relationships at 5, with `omitted_budget` `:207-249` [I] | Catalog rows keep their `source_fact_id`; demonstrations are typed refs; the evidence pointer is a RetrievalUnit [I] | get_operation `py/server.py:487-506` |
| A23 Original evidence `lp/evidence.rs::read:108-257`, `get_retrieval_unit:355-484` | The original bytes for a reference | spans, artifacts, scenarios, deployments | Byte-paginated substring | Exact bytes. `metadata_omitted` when the detail exceeds half the budget `:149-151` [I] | 32 or 256 KiB; cursor bound to generation, target and representation `:47-74` [I] | `OriginalContent` with digest, alignment and provenance. Declaration anchors are skipped `:445` [I] | get_evidence |
| (out of scope) `cc/entry_links.rs`, `cc/usage.rs`, `la/selection.rs` | Behavioural and brief-only analyses | – | – | For contrast, governed heuristics: `EFFECT_RULE_REVISION` `cc/entry_links.rs:25` [IC]; `Params::digest` `la/selection.rs:40-59` [IC]; usage caps `MAX_CANDIDATES`/`MAX_STATEMENTS` `cc/usage.rs:30-32` [IC] | – | – | briefs, behavioural profile only (`cc/synth.rs:903`, `cc/attempt.rs:926`) [IC] |

### 2.1 Stated fidelity labels in the design doc compared with the code

1. **Section labels.**
   - §14.4 has a PR2 "Implemented" status with a qualification label (`D::§14.4:149`).
   - §14.5 and §14.6 have PR3 "Implemented" (`D::§14.5:245`, `D::§14.6:300`).
   - §14.7 has PR4 "Implemented and bounded …" (`D::§14.7:337`).
   - §14.9 has PR5 "Implemented" (`D::§14.9:481`).
   - **§14.8 (retrieval units and ranking) has no status label** (`D::§14.8:436-473`). Under the section rule "Unless explicitly labeled otherwise, all new behavior in this section is Proposed" (`D::§14:27`), it therefore reads as Proposed. The code implements it: `cs/retrieval.rs:1-812`, `cc/retrieval.rs:68-287`, `lp/evidence_search.rs:17-244` [I].
2. **"None, no default, unknown default and a factory expression are different"** (`D::§14.4:178-179`). Storage keeps them distinct (`cc/catalog.rs::default_value:425-453`) [I]. The `ParameterDefaultState` predicate then collapses `unknown` and `optional_expression_unavailable` into a definite false (`cs/selection/catalog.rs::evaluate:897-899`) [I]. See G1.
3. **"Exact symbol resolution, explicit cross-reference, ambiguous textual mention and similarity remain distinct"** (`D::§14.5:222-223`). There is no explicit-cross-reference basis. `doc_links` (`cs/tables.rs:1394`) is not an evidence input (`cc/evidence.rs::EvidenceInputs:21-35`, `evidence_inputs!:59-73`) [I]. The basis vocabulary has only `explicit_config_reference` (`cs/catalog.rs:230-241`) [I].
4. **"a singleton candidate does not become a resolved invocation"** (`D::§14.5:250-251`). This holds for call sites (`cc/evidence.rs:809-815`; validator `cs/evidence.rs:481-485`) [I]. It does not hold for composed bases (`cc/evidence.rs:1077-1097,1111-1131`) or for selection deployment observations (`cs/selection/catalog.rs:663-692`) [I]. See G2.
5. **Signature roles "source declaration, effective public surface, provider-synthesized constructor, documented form, or runtime observation"** (`D::§14.4:173-174`). The code has only source, overload and provider_constructor (`cs/catalog.rs:286`). `effective_surface` is the constant `"unresolved"` (`lp/hydration.rs::catalog_record:271`) [I]. This is consistent with the "Proposed unless labelled" rule, but §14.4 carries an Implemented label.
6. **Invocation forms** "…awaitable result, generator iteration…" (`D::§14.4:184-188`). `InvocationForm` has no awaitable, generator or unbound-method variants (`cs/wire/requirements.rs:75-83`) [IC].
7. **Decorator normalization "resolved identity/candidates … argument expression/literals"** (`D::§14.4:190-193`). `catalog_surfaces` stores one `resolved_target` (no candidate set) and the raw `expression` (no argument literals) (`cs/catalog.rs:331-342`, `cc/surface.rs:735-749`) [I]. "Pilot registration forms" (`D::§14.4:194-195`) is implemented as a hard-coded distribution-pin gate (`cc/surface.rs:641-644`) [I].
8. **"Include class, property and config-object units"** (`D::§14.8:452-453`). There is no separate configuration-object unit; fields are rendered as lines inside the owning member's ApiOptions unit (`cs/retrieval/catalog.rs:117-134`) [I].
9. **"Lexical-only availability is labeled"** (`D::§14.8:459`). search_evidence reports `fallback = channel_state` (`lp/evidence_search.rs:167-171`) [I]. search_operations reports `actual_route: lexical-only` but no fallback reason (`lp/selection.rs:540`), even though Python computed one (`py/server.py:306-314`) [I].
10. **"The result envelope … includes … absent/not-requested capabilities, budget refusal"** (`D::§14.7:426-428`). `SelectionResults` has no capability or budget fields (`cs/wire/requirements.rs:684-696`). `SelectionCoverage.evaluation_complete` is always `true` (`cs/selection.rs::classify:147`) [I].
11. **"Distribution ownership and first-party release membership are separate association bases"** (`D::§14.6:304-305`). The vocabulary has `owning_distribution` (`cs/catalog.rs:237`), but no code path produces it; only `release_distribution` is emitted (`cc/evidence.rs:1133-1166`) [I]. The vocabulary entry is unused.
12. **"Release-level package metadata … explicit release subjects alongside member subjects"** (`D::§14.5:248-249`). This matches the code, but the member evidence page is not scoped to the member's own release: `(member_id=$2 OR release_id IS NOT NULL)` (`lp/evidence.rs::association_page:294,300`) [I].

### 2.2 Absent-lookup handling (unknown vs absent)

Unknown is preserved at these points:

- `catalog_export_candidates` uses a LEFT JOIN that keeps NULL declarations. Downstream, `provider_resolved` becomes false and the contexts become incomplete (`cs/public.rs:177-181`, `cc/catalog_domains.rs:36-46`) [I].
- In `derived::Signatures` and `Parameters`, missing provider rows produce BoundaryReason values, not NULLs meaning "no" (`cs/derived.rs::Signatures::sql:376-400`, `::Parameters::sql:450-457`) [I].
- In the classifier, the following all return `None`:
  - a missing parameter in an incomplete context (`cs/selection/catalog.rs:906-908`);
  - a missing type term (`type_match:1218`);
  - a non-literal default (`primitive_match:1272`);
  - a deployment non-match (`:685`);
  - no release scope (`:1082-1092`).

  Relationship and Deployment evidence only ever adds positives (`:1143-1214`) [I].
- In the PG requests:
  - `selection_domains.sql` loads only the requested domains, and an unloaded domain is a hard error (`cs/selection/catalog.rs::classify:341-346`) [I].
  - A missing operand is a hard error (`validate_operands:474-478`) [I].
  - `resolve_members` distinguishes ambiguous, not found and resolved (`lp/packet.rs:49-58`, `lp/journeys.rs:262-312`) [I].
  - `behavior_status` defaults to `not_analyzed` when there is no operations row (`lp/packet.rs:184`) [I].

Unknown is collapsed into absent or false at these points (details under Gaps):

- `ParameterDefaultState`: `Some(p.default_state == word(state))` (`cs/selection/catalog.rs:897-899`) [I]. G1.
- `MemberKind` for member kinds outside the `MemberKind` enum (`cs/selection/catalog.rs:747-749`) [I]. G11.
- Browse Module scope (`lp/journeys.rs:56,77`) [I]. G6.
- The empty evidence section is labelled `empty_under_coverage` (`lp/packet.rs:462-475`) [I]. G7.
- `joint()` on budget exhaustion (`cs/selection.rs:255-257`) [I]. G8.
- `Hydration::fetch` uses `WHERE key = ANY($2)`: a missing key silently returns fewer rows (`lp/repository.rs::fetch_projection:290,313`) [I]. This is benign where keys come from the same generation. `catalog_record` returns empty arrays with no completeness marker (`lp/hydration.rs:185-273`); the packet's `incomplete_facets` covers this only for members that have an operation node (`lp/packet.rs:111-133`) [I].
- In the catalog builder, missing text or syntax nodes cause evidence rows to be skipped with no flag (`cc/catalog.rs:1312,1334,1377-1379`), and decorators are dropped (`cc/surface.rs:660-662`) [I]. For non-UTF-8 modules this is latent, because such modules have no parse or declarations (`cs/tables.rs::SourceFiles:170-172`) [IC].

### 2.3 Notable CI gaps

**G1: an unknown default state becomes a definite negative, which can produce a false ClosedAbsence.**

- Provider-constructor parameters get `default_value(None, p.required)`. That yields `optional_expression_unavailable` (a default exists but its text is unknown) or `unknown` (`crates/cpg-core/src/catalog.rs::derive_indexed:1146`, `::default_value:429-438`) [I].
- `ParameterDefaultState` evaluates `Some(p.default_state == word(state))` (`crates/cpg-schema/src/selection/catalog.rs::PreparedCatalog::evaluate:897-899`) [I].
- Provider-constructor signatures can be `complete` contexts. Their form is `list`, and the check is `crates/cpg-core/src/catalog_domains.rs::derive:161-164` together with `crates/cpg-core/src/catalog.rs:1138` [I].
- Result: for a dataclass `Config(timeout: int = 30)` whose `__init__` is generated by the provider, the requirement `parameter_default_state(timeout, literal)` over a closed domain gives `negatives == claims.len()`, which classifies as **Contradicted/ClosedAbsence** (`crates/cpg-schema/src/selection.rs::classify:125-129`) [I].
- Under `all_applicable`, any such negative is a Counterexample (`:108-112`) [I].
- The fix is to return `None` when the stored state is `unknown`, or when it is `optional_expression_unavailable` and a literal-type state is requested.

**G2: fidelity is dropped when associations are composed or projected.**

- **(a)** `explicit_config_reference` and `task_observation` associations are attached to every member of any scenario association, including `candidate_targets` (`crates/cpg-core/src/evidence.rs::PreparedEvidence::derive:1077-1097,1111-1131`) [I].
- **(b)** The selection DeploymentDeclaration Launch/Configuration path accepts every member association with `evidence_kind == Deployment`, including `suggests`/`same_document_passage`. It emits `EvidenceBasis::SourceDeclaration` with no Association witness (`crates/cpg-schema/src/selection/catalog.rs::evaluate:663-692`) [I].
- **(c)** Scenario and Source domain contexts are built from associations of any basis (`crates/cpg-core/src/catalog_domains.rs::derive:223-256`, `crates/cpg-schema/src/selection/catalog.rs::associated_sources:1391-1438`). So ScenarioIntent, ScenarioCheck and SourceAlignment can be Supported through an ambiguous mention. The witnesses cite only `Original` and `Domain` (`evaluate:1055-1075,1117-1132`) [I].
- **(d)** `retrieval_subjects` has no basis column (`crates/cpg-schema/src/retrieval/catalog.rs::derive:200-218`, `crates/cpg-schema/src/retrieval.rs::files:275-283`). Member votes in search_operations and the subject filter in search_evidence (`crates/lctx-postgres/queries/evidence_eligible.sql`) therefore treat candidate or ambiguous links as equal to resolved ones [I].
- By contrast, `packet_demonstrations.sql` correctly restricts to `basis='resolved_target'` [I].

**G3: storage evidence is relabelled as "reader".**

- An exact_storage link's `source_fact_id` is the store statement (`crates/cpg-core/src/surface.rs::derive:991-995`, `::direct_store:628-630`) [I].
- The citation loop labels every field-link citation `"reader"` (`crates/cpg-core/src/catalog.rs::derive_indexed:1361-1371`) [I].
- The label then propagates into `catalog_associations.role` through `source_fact` span associations (`crates/cpg-core/src/evidence.rs:998-1008`) [I].
- The vocabulary has no `storage` role (`crates/cpg-schema/src/catalog.rs::vocabulary:300`) [I].

**G4: the pilot-library heuristic in compiler normalization is ungoverned.**

- FastMCP registration recognition runs only if some release distribution equals the string `"fastmcp==4.0.5"` (`crates/cpg-core/src/surface.rs::derive:641-644`), with hard-coded targets at `::registration_target:236-241` and `::aspects:313-337` [I].
- It has no named model or revision, is outside the committed models catalog (`crates/cpg-schema/models/External.toml` [IC]), and has no parameter digest. `catalog_compilation.input_digest` does not cover it (`crates/cpg-core/src/catalog.rs::CompileInputs::digest:41-52`) [I].
- It is versioned only implicitly, through the stage-cache code digest (`crates/cpg-core/src/stage_cache.rs:312-315`) [I].
- A patch-version bump silently removes all `registration` aspects.
- For contrast, the task-receipt policy is named and versioned, `"fastmcp-stdio-v1"`, and bound to the runner hash (`crates/cpg-extract/src/observations.rs::validate:154-189`) [I].

**G5: lexical ranking parameters are outside the profile digest.**

- BM25 `k1=1.5, b=0.75, method="lucene"`, the tokenizer regex and the discriminating-word abstention are Python literals (`python/lctx_mcp/src/lctx_mcp/retrieval.py:25-60`) [I].
- `Policy` digests only `{format, route, metric, ties, rrf_k}` (`crates/lctx-postgres/src/profiles.rs::Policy:14-43`) [I].
- `RetrievalReceipt` omits `FUSION_REVISION` (`crates/cpg-schema/src/retrieval.rs:99-105`) [I].
- A lexical policy change therefore leaves the profile, cursors and receipts unchanged.

**G6: browse ownership misreports unknowns.**

- In Module scope, members with `module == None` are silently excluded (`crates/lctx-postgres/src/journeys.rs::browse_library:56`) [I].
- `unknown_ownership` is counted only inside the admitted scope (`:77`), so it is always 0 for Module scope [I].
- An empty result is reported as "scope is not recorded" (`:67-76`), even when members with unknown ownership exist [I].

**G7: `empty_under_coverage` is asserted with no association coverage.**

- The evidence section reports this state whenever `total == 0` (`crates/lctx-postgres/src/packet.rs::section_page:462-475`) [I].
- Association coverage is never established: the Relationships, Scenarios and SourceArtifacts domains are hard-coded open (`crates/cpg-core/src/catalog_domains.rs::derive:288-305`) [I].
- The state name is not documented anywhere else (only `crates/cpg-schema/src/wire/journeys.rs:30`) [I].

**G8: a budget hit or incomplete evaluation is indistinguishable from a semantic outcome.**

- `joint()` returns `NotEstablished` when `work > MAX_CONTEXT_WORK` (`crates/cpg-schema/src/selection.rs::joint:255-257`), with no reason field [I].
- `SelectionCoverage.evaluation_complete` is hard-coded `true` (`::classify:147`) [I].
- `SelectionReason::ResourceRefused` (`crates/cpg-schema/src/wire/requirements.rs:645`) is never emitted by classify [I].

**G9: search_operations drops the degradation reason.**

- Python computes `lexical-only`, `lexical-only:embedding-unavailable` and similar reasons (`python/lctx_mcp/src/lctx_mcp/server.py::search_members:306-314`) [I].
- Rust puts that value only into the cursor hash. The response JSON has no `fallback` (`crates/lctx-postgres/src/selection.rs::finish_selection_search:539-540`) [I].
- The unranked path also does not distinguish "no embedder" from "no vectors". search_evidence does (`python/lctx_mcp/src/lctx_mcp/server.py:692-694`) [I].

**G10: `DomainDetail.module` is an arbitrary winner with mixed semantics.**

- The value is `min(access_module)` for exposures that have public names. Otherwise it is `min(module_name)` over the files of the non-shadowed declarations, which is the defining module (`crates/cpg-core/src/catalog_domains.rs::derive:57-83`) [I].
- The PublicModule predicate treats it as exact, "attributed by the exposure source" (`crates/cpg-schema/src/wire/requirements.rs:206-211`, `crates/cpg-schema/src/selection/catalog.rs::evaluate:741-743`) [I].
- Browse grouping uses it as well [I].
- This contradicts "Alternatives remain sets rather than arbitrary winners" (`crates/cpg-core/src/evidence.rs:156`) [I].

**G11: member kind is a free-text vocabulary compared against a closed enum.**

- `catalog_members.kind` takes DeclarationKind or SymbolKind texts (e.g. `attribute`, `constant`, `type_alias`) or `unknown` (`crates/cpg-core/src/catalog.rs:728,792-795,913`) [I].
- The DDL has no CHECK on it (`crates/lctx-postgres/migrations/202609280009_catalog.sql:18`) [IC].
- `MemberKind` has only 7 values (`crates/cpg-schema/src/wire/requirements.rs:66-74`) [IC].
- A `constant` member therefore gives `Some(false)` for `Variable` (`crates/cpg-schema/src/selection/catalog.rs::evaluate:747-749`), and PublicExposures can close (`crates/cpg-core/src/catalog_domains.rs:270-275`) [I].
- `MemberKind{Unknown}` can never be Supported, because an unknown kind maps to `None` (`::member_kind:483-485`) [I].

**G12: mention exactness ignores the extractor's modality.**

- The extractor marks a span with several exact targets as `Candidate` (`crates/cpg-extract/src/docs.rs:674-677`) [I].
- The association derivation re-derives exactness from `class` and a per-row member count (`crates/cpg-core/src/evidence.rs::derive:903-916`) [I]. It can label each such target `exact_textual_reference` when the rows' access paths are distinct and their qualified names do not collide.

**G13: the evidence page is unscoped across releases.**

- Every member's evidence page and counts include all release-scoped associations (`crates/lctx-postgres/src/evidence.rs::association_page:294,300`) [I].
- This includes metadata associations for other captured releases, for example dependency contexts [IC for multi-release].

**G14: served witnesses are not fully closed for the agent.**

- `WitnessEvidence::Fact` is accepted without a check during projection validation. The comment says it relies on the pre-publication reconstruction (`crates/cpg-schema/src/selection/catalog.rs::validate_projection:1511`) [I].
- No served tool opens a Fact id. `Anchor::Declaration` is skipped by `get_retrieval_unit` (`crates/lctx-postgres/src/evidence.rs:445`) [I].
- Provider-only ApiOptions lines therefore have no openable original bytes [I].

**G15: some labels are untyped or chosen by tie-break.**

- `catalog_spans.extraction` keeps the lexicographic minimum when a span has several roles (`crates/cpg-core/src/evidence.rs::span:95-99`) [I].
- These columns are free text with no DDL CHECK:
  - `extraction` (`…/202609280011_evidence.sql:39`) [IC];
  - `catalog_surfaces.reason` (`…/202609280010_specificity.sql:30`) [IC];
  - `retrieval_fragments.embedding_status`, which Rust validates (`…/202609280012_selection_retrieval.sql:65`) [IC].
- Two vocabulary entries are never produced: `owning_distribution` (`crates/cpg-schema/src/catalog.rs:237`) and `ContextStatus::Refused` (`crates/cpg-schema/src/evidence.rs:19`) [I].
- Association intent lacks `mixed`, while scenario intent has it (`crates/cpg-schema/src/catalog.rs:242-248` vs `crates/cpg-schema/src/evidence.rs:23-30`). It never occurs at the association level [I].

**G16: API rendering conflates binding alternatives.**

- ApiOptions text includes the signatures of `shadowed_source`, `stub_source` and `source_alternative` bindings. Each is labelled only with its signature role (`crates/cpg-schema/src/retrieval/catalog.rs::derive:63-116`) [I].
- Selection and domains exclude `shadowed_source` (`crates/cpg-core/src/catalog_domains.rs:33`, `crates/cpg-schema/src/selection/catalog.rs:917-922`) [I].

**G17: `Relationship{target: Member}` is degenerate.** It iterates only the candidate's own associations and tests `a.member_id == target`, so it can be Supported only when the candidate is the target itself (`crates/cpg-schema/src/selection/catalog.rs::evaluate:1150-1155,1182-1184`) [I].

**G18: catalog compilation has no cardinality bound.** Contract, surface and evidence derivation use repeated linear scans inside loops, for example:

- `crates/cpg-core/src/catalog.rs:985-987,1334,1362`
- `crates/cpg-core/src/surface.rs:903-956,1003-1087`
- `crates/cpg-core/src/evidence.rs:1067-1084` [I]

There is no work budget or `resource_refused` path. Only the retrieval step (`crates/cpg-schema/src/retrieval/catalog.rs:289-291`) and serving are bounded [I].

**G19: heuristic and derivation revisions are not in served identity.**

- The following have no named revision or parameter digest in any served row or in `catalog_compilation`:
  - surface aspect table, field-default callees, scenario intent names, snippet languages, alignment rule;
  - `crates/cpg-core/src/surface.rs:252-359,808-817`;
  - `crates/cpg-core/src/evidence.rs:260-292,337-342,525-528,439-444` [I].
- Compare the governed examples:
  - `POLICY_REVISION` (`crates/cpg-schema/src/selection.rs:9`) [I];
  - `VIEW_REVISION`, `RENDER_REVISION` and `FUSION_REVISION` (`crates/cpg-schema/src/retrieval.rs:9-11`) [I];
  - `Params::digest` (`crates/lctx-analytics/src/selection.rs:57-59`) [IC].


## 3. Registry aspect table (paper probe P7)

"Hand" means a site where a person writes the aspect's content directly: a literal field list, a
string name, SQL text or a literal value list. "Derived" means code that computes the aspect from
another declaration. Hand sites come in two kinds:

- **compile-coupled**: a Rust struct literal or a `From` impl, where renaming a column breaks the build;
- **string-coupled**: SQL text, a relation or column name as a string, or a JSON key, where drift shows
  up only at runtime or not at all.

Citations use `path::symbol:line`, with paths relative to the repository root.

---

### 3.0 Registries, counts and where the five sampled tables live

#### 3.0.1 What `table!` generates [I]

`crates/cpg-schema/src/table.rs::table!:148-210` takes one declaration (name, family, key idents,
CHECK strings, field list) and generates:

- the row struct and `hash_fields` (`:158-168`);
- `Table::schema()`, built field by field through `ArrowColumn::field`, which adds the
  `lctx.codebook` metadata for codebook types (`column.rs::ArrowColumn for C: Codebook:277-320`);
- `key()` as `stringify!` of the key idents (`:184-186`);
- `checks()` (`:187-189`) and `to_batch` (`:190-195`);
- the `QueryRow` decoder (`:199-208`).

Two gaps matter below:

- **Key idents are not checked against the field list at compile time**: `stringify!` only
  (`table.rs:184-186`) [I].
- CHECK strings are free SQL text (`:155`) [I].

Everything else in the store derives from these methods:

| Consumer | Site |
|---|---|
| Delta create (schema plus CHECKs) | `crates/cpg-core/src/delta.rs::create:61-82` |
| Delta verify on open | `delta.rs::verify:110-146` |
| Canonical sort | `table.rs::canonical_sort:47-69` |
| Contract text | `table.rs::contract:117-129` |
| Schema digest | `table.rs::schema_digest:110-114` |

#### 3.0.2 The three hand-ordered table registries [I]

| Registry | Site | Entries |
|---|---|---|
| `for_each_table!` (raw, "declaration order") | `crates/cpg-schema/src/tables.rs::for_each_table:1536-1599` | **57** (includes `evidence::CapturedArtifacts`) |
| `for_each_derived_table!` ("dependency order") | `crates/cpg-schema/src/derived.rs::for_each_derived_table:1166-1195` | **22** |
| `for_each_analysis_table!` ("declaration order", ADR-0019) | `crates/cpg-schema/src/findings.rs::for_each_analysis_table:479-606` | **121** |
| Total in registries | | **200** |
| `table!` invocations in `cpg-schema/src` | 20 files (57 in `tables.rs`, 64 in `behavior.rs`, …) | **201** |

- The one `table!` outside the registries is `snapshots` (`tables.rs::Snapshots:1512`). `tables.rs::contracts:1602-1612` appends it by hand after the three macros [I].
- I extracted the struct names of every `table!` and of every registry entry and compared them: the two sets are **equal** apart from `Snapshots` [I]. No test enforces this equality. A `table!` that is missing from every registry gets no contract snapshot, no Delta write and no rules, and nothing reports the omission.

These consumers derive from the registries (each is `[I]`):

| Consumer | Site |
|---|---|
| Write every raw table | `crates/cpg-core/src/attempt.rs::write_raw:305-335` |
| Derive in registry order | `attempt.rs:718-724` (`derive_all`) |
| Register empty analysis tables | `attempt.rs:736-740` (`initialize`) |
| Write empty unselected analysis tables | `attempt.rs:1646-1651` (`persist_unselected`) |
| Schema digest by name | `attempt.rs::schema_digest_of:356-369` |
| Rule shapes | `crates/cpg-schema/src/rules.rs::shapes:2415-2429` |
| `catalog-unselected` rules | `rules.rs::rules:3816-3822` |
| Contract list | `tables.rs::contracts:1602-1612` |
| Derivation list | `derived.rs::derivations:1198-1203` |
| Rebuild copy | `crates/cpg-core/src/rebuild.rs:301-303` |
| Stage-cache bind | `crates/cpg-core/src/stage_cache.rs:37` |
| Reference test | `crates/cpg-schema/tests/contracts.rs::references_name_real_columns:36-58` |

**Where each sampled table lives.**

| Table | Declaration | Registry | Producer | In Delta? |
|---|---|---|---|---|
| `edges` | `crates/cpg-schema/src/graph.rs::Edges:1280-1301` | derived (`derived.rs:1190`, after `Nodes` at `:1189`) | `Derived::sql` (`graph.rs::<Edges as Derived>::sql:1303-1327`) | yes |
| `summary_flows` | `crates/cpg-schema/src/behavior.rs::SummaryFlows:1459-1490` | analysis (`findings.rs:542`) | Rust (`crates/lctx-analytics/src/summaries/finite.rs:1043-`, `:1413-`) | yes |
| `catalog_parameters` | `crates/cpg-schema/src/catalog.rs::CatalogParameters:107-117` | analysis (`findings.rs:489`) | Rust (`crates/cpg-core/src/catalog.rs::derive_contracts:1059-1085`, `:1147-1166`) | yes |
| `behaviors` | `crates/cpg-schema/src/behavior.rs::Behaviors:1119-1178` | analysis (`findings.rs:587`) | Rust (`crates/cpg-core/src/behavior.rs`, e.g. `:575`, `:1345`) | yes |
| `retrieval_units` | **no `table!`**: `crates/cpg-schema/src/retrieval.rs::files:259-307` (hand-written `ServingFile`) | **none** | Rust (`crates/cpg-core/src/retrieval.rs::materialize:68-288`) | **no**: serving artifact plus IPC replay receipt (`crates/cpg-core/src/retrieval.rs::save:436-478`) |

All five rows are [I].

#### 3.0.3 Codebooks and vocabularies

- **Int16 codebooks: 102** `codebook!` invocations, all in `crates/cpg-schema/src/codebook.rs`. No
  other `impl Codebook` exists (grep found only the macro at `:41`) [I].
- **`codebook::registry()` is a hand list of 102** `CodebookEntry::of::<T>()` calls
  (`codebook.rs::registry:1771-1876`). I compared its names with the declared names and they are
  **equal** [I].
  - Tests: `crates/cpg-schema/tests/codebooks.rs::registry_snapshot:7` and
    `codes_are_dense_from_zero_and_names_unique:19` check snapshot, uniqueness and density. Neither
    checks that every declared codebook appears in the registry [IC].
  - If a codebook were missing, the generated `codebook:` rule would be built with an empty `NOT IN ()`
    list (`rules.rs:3876-3895`, `unwrap_or_default()`), which fails only when validation runs [I].
- **Catalog text vocabularies**: `crates/cpg-schema/src/catalog.rs::vocabulary:209-306` has **25 match
  arms covering 27 (relation, field) pairs across 14 catalog relations** [I].
  - 20 arms are hand-written string lists.
  - 5 arms (7 pairs) derive their values from codebook texts: `ParameterKind` (3 pairs), `RecordKind`,
    `TypeTermKind`, `TypeRole` and `TypeArgRole`.
  - The wire layer re-keys **22** of these pairs by string in `vocabulary!` adapters
    (`crates/cpg-schema/src/wire/vocabulary.rs:3-173`) [I].
- **A third vocabulary mechanism**: `retrieval::VIEWS` / `Family::name()`
  (`crates/cpg-schema/src/retrieval.rs::VIEWS:29-58`) governs `retrieval_units.family`. It is neither a
  codebook nor a `catalog::vocabulary` entry [I].

#### 3.0.4 Serving files, native files and wire contracts [I]

- `crates/cpg-schema/src/bundle.rs::files:48-826` produces **72** serving files:
  - **49** hand-written `file(...)` schemas in `bundle.rs:54-821`;
  - **18** derived from `table!` by `catalog.rs::serving_files:128-164`, which drops `snapshot_id` and
    sets `key = &T::key()[1..]`;
  - **5** hand-written in `retrieval.rs::files:259-307`.
- `crates/cpg-schema/src/serving_projection.rs::NATIVE_FILES:12-42`: **29** hand-written names.
- `crates/cpg-schema/src/wire/dispatch.rs::contracts!:14-86`: **71** wire contracts.

#### 3.0.5 PostgreSQL DDL: generated once, then frozen and hand-maintained

**Migrations** [I]:

- 12 files in `crates/lctx-postgres/migrations/`.
- **91** static `CREATE TABLE` statements: 85 in `lctx_serving`, 4 in `lctx_ops`, 2 in `lctx_cache`.
- 4 of them are dropped later (`202609280012_selection_retrieval.sql:208-209,226-227`), leaving
  **87 live static tables**:
  - **72 projection relations**, whose names are set-equal to `bundle::files`;
  - 9 `lctx_serving` service tables;
  - 6 ops/cache tables.
- Per-generation partitions are created dynamically, for example by `EXECUTE format('CREATE TABLE … PARTITION OF …')` at `202609280012_selection_retrieval.sql:164`.

**The generator** [I]:

- `crates/lctx-postgres/src/projection.rs::relation_ddl:51-116` builds DDL from five inputs:
  `bundle::files(1024)`, `catalog::vocabulary`, `serving_projection::codebook_name` (both become
  `CHECK … IN (…)`), `serving_projection::unique_key` (becomes `UNIQUE`) and
  `serving_projection::foreign_keys` (becomes `ADD FOREIGN KEY`).
- It is run by `crates/lctx-postgres/examples/projection_ddl.rs` and pasted into migrations. The
  migrations carry the marker "Generated by lctx-postgres/examples/projection_ddl.rs"
  (`202609270004_projection.sql:1`, `…0009_catalog.sql:10`, `…0010_specificity.sql:15`,
  `…0011_evidence.sql:9`, `…0012_selection_retrieval.sql:9`).
- `table!` CHECKs are **not** projected into PostgreSQL (`projection.rs:55-99` emits none).

**Evidence that the migrations are frozen snapshots, not regenerated** [I]:

- `202609280012_selection_retrieval.sql:111` contains a hand-written
  `ALTER TABLE lctx_serving.catalog_types ADD COLUMN literal_json text`. The generated
  `catalog_types` in `202609280009_catalog.sql:130-143` lacks the column, but the current `CatalogTypes`
  Arrow schema has it.
- `202609270004_projection.sql:1066,1081` created `operation_text` and `operation_vectors` from an
  older bundle declaration; `0012:208-209` drops them.
- Codebook value lists are frozen into `CHECK IN` clauses. For example, the 38 `boundary_reason` values
  appear at `202609270004_projection.sql:381,488`. They match the current `BoundaryReason`
  (`codebook.rs:165`, 38 values) only because nothing has been appended since.

**Other hand copies inside migrations** [I]:

- `lctx_serving.cleanup_generation`
  (`202609280012_selection_retrieval.sql:173-188`) hard-codes an array of all **72** relation names
  (`:179`). That array is currently set-equal to `bundle::files`.
- `analyze_generation` (`:189-195`) hard-codes 30 names (`:192`).
- `generation_formats` (`:8`) hard-codes `(6,16)`, a copy of `serving_projection.rs::FORMAT:44` and
  `BUNDLE_FORMAT:45`.
- `check_indexes` (`crates/lctx-postgres/src/import.rs::check_indexes:689-755`) hard-codes the expected
  index definitions. The indexes themselves are hand DDL, e.g. `202609280005_publication.sql:161-169`.

**Is PostgreSQL DDL generated from the Arrow schemas?** Transitively, yes, but only at the moment a
migration is authored. **No test or check compares `relation_ddl()` output with the migrations** [I].

- grep for `relation_ddl` or `projection_ddl` finds only the generator, the example and the migration
  comments.
- `just sqlx-check` runs `scripts/postgres_check.py:104-117`: it migrates a fresh database, then runs
  `cargo sqlx prepare --check`. That checks the hand-written `query_file!` SQL against the migrated DDL,
  **not** against the Arrow schemas [IC] (`justfile:204-205`).
- Runtime drift detectors on import [I]:
  - `import.rs::load_batch:585-655` inserts by column name (`:643`).
  - `import.rs::validate_stored:657-687` reads back with `SELECT *`, decodes by name with
    `projection::decode_rows:119-131` and compares with `serving_projection::validate_batch:499-509`.
  - Together these catch a missing, renamed or re-typed column, or a new codebook value, when a
    generation is imported. That happens in the integration tests
    `crates/lctx-postgres/tests/serving.rs::copy_codec_pgvector_and_empty_declared_schemas:186` and
    `production_import_is_atomic_repeatable_and_selection_is_explicit:450` [IC]. The fixture
    `empty_source_version:357-369` loops over `bundle::files(0)`.

---

### 3.1 `edges` (derived registry; not served)

| # | Aspect | Hand-written sites | Derived from | Drift cross-check |
|---|---|---|---|---|
| 1 | Arrow schema | **Hand:** field list `graph.rs::Edges:1289-1300` (10 fields; the single authority) [I].<br>**String-coupled:** output column aliases in `graph.rs::<Edges as Derived>::sql:1318-1325` and `graph.rs::row:79-96`, matched to fields by name in `crates/cpg-core/src/delta.rs::to_schema:211-227` via `crates/cpg-core/src/derive.rs::derive:23-43` [I]. | `table!`: `EdgesRow`, `schema()`, `to_batch`, `QueryRow` (`table.rs:158-208`); Delta schema (`delta.rs::create:63`); contract text and schema digest (`table.rs:80-129`, `attempt.rs::schema_digest_of:356-369`); consumer `crates/cpg-core/src/evidence.rs::evidence_inputs!:65` uses `EdgesRow` [I]. | `crates/cpg-schema/tests/contracts.rs::contracts_snapshot:10` and `::derivations_snapshot:17` [IC]; runtime `delta.rs::verify:110-146` (SchemaDrift) and by-name cast in `to_schema` [I]. |
| 2 | Identity recipe | **Hand:** SQL `lctx_id('edge', e.edge_kind, e.src_node_id, e.dst_node_id, e.ordinal, e.discriminator)` at `graph.rs:1319-1320`, with the kind **literal** `'edge'`. `crates/cpg-schema/src/id.rs::kind::EDGE:89` is declared but never referenced (grep) [I]. The discriminator is chosen per kind in the hand registry `graph.rs::edge_sources:248-1096` [I]. | `lctx_id` UDF = `IdHasher` (`crates/cpg-core/src/udf.rs::lctx_id:36`, `:162-190`) [I]. No Rust recipe exists for edges. | No `id:edges` recompute rule: the `id:` list at `graph.rs::rules:1690-1748` covers other tables [I]. `udf.rs::tests::sql_ids_equal_the_rust_recipe:217` uses kind `"edge"` only as a synthetic known answer; it does not pin the edges field order [IC]. `derivations_snapshot` pins the SQL text [IC]. `key:edges` uniqueness is enforced (`rules.rs:3790-3799`) [I]. |
| 3 | Sort key | **Hand:** `key = [snapshot_id, edge_id]` `graph.rs:1287` [I]. | `Table::key()` feeds `canonical_sort` (`derive.rs:42`), the `key:edges` rule (`rules.rs:3793`) and `delta::read_at` ordering (`delta.rs:245`) [I]. | `contracts_snapshot` [IC]. `keys_and_checks_name_real_columns` (`contracts.rs:60-109`) uses a **hand list of 26 tables that omits `Edges`** [I]. |
| 4 | Validation | **Hand:** CHECK `ordinal_nonnegative` (`graph.rs:1288`) [I].<br>**Hand graph registry** `graph.rs::edge_sources:248-1096`: endpoint kinds, parallel flag, evidence table and lineage per kind [I].<br>**Hand `REFERENCES` into edges:** `rules.rs::REFERENCES:254-266` (5 entries) [I].<br>**Hand semantic SQL reading edges:** `rules.rs:3585-3597` (`semantic:refuted-not-overridden`), `rules.rs:3710` [I]. | Delta CHECK (`delta.rs::create:75-81`, verified at `delta.rs::verify:134-145`) [I].<br>Rules generated from the schema: `key:edges`, `codebook:edges.{edge_kind,src_kind,dst_kind}` (`rules.rs:3873-3895`) [I].<br>Rules generated from the graph registry: `endpoint:`, `evidence:`, `one-per-evidence:`, `no-parallel:` and `lineage:` per kind (`graph.rs::rules:1614-1676`), plus `support:edges` (`:1826`) [I]. | `crates/cpg-core/tests/graph.rs::each_graph_rule_rejects_a_doctored_catalog:441` [IC]; `contracts.rs::references_name_real_columns:36` (column existence) and `::rules_snapshot:27` [IC]. |
| 5 | Codebook / vocabulary | None hand-written for the column types (`EdgeKind` `codebook.rs:462`, `NodeKind` `:426`) [I]. `codebook::registry()` is itself a hand list (`codebook.rs:1771-1876`) [I]. | `lctx.codebook` metadata (`column.rs:277-320`); `codebook:` rules; `CAST(kind.code())` in the derivation (`graph.rs:1309`); `edge_kinds` publishes kind texts (`graph.rs::<EdgeKinds as Derived>::sql:1388-1410`) [I]. | `codebooks.rs::registry_snapshot:7` [IC]. No test that every `EdgeKind` has an `EdgeSource`: grep finds no `EdgeKind::all()` [I]. |
| 6 | Delta write | **Hand:** position in `for_each_derived_table!` (`derived.rs:1190`). It must follow `Nodes` because the SQL joins `nodes` (`graph.rs:1324-1325`) [I]. | `attempt.rs:718-724`, then `write_derived:336-352`, then `write::<T>:289-302` (schema equality, then `open_or_create`); rebuild copy `rebuild.rs:301-303` [I]. | Runtime only: a misordering fails at plan time because `nodes` is not yet registered [I]. |
| 7 | Bundle / serving | **None:** `edges` is not a served file. The consumer SQL for `support_attribute_incidences` joins it (`crates/cpg-core/src/bundle.rs:313-314`) [I]. | n/a | n/a |
| 8 | PG DDL / migration | none | n/a | n/a |
| 9 | PG import mapping | none | n/a | n/a |
| 10 | IPC / native | none. The evidence stage-cache input is keyed by `T::NAME` (`crates/cpg-core/src/evidence.rs:51-72`) [I]. | derived (`evidence.rs:53`) | n/a |
| 11 | Wire DTO | none | n/a | n/a |
| 12 | Hydration SQL | none | n/a | n/a |
| 13 | Python model | none | n/a | n/a |
| + | Relation deps naming `edges` | **Hand `deps = [...]` string lists:**<br>- `crates/cpg-schema/src/behavior.rs:3393,3405,3411,3418,3450,3470`<br>- `crates/cpg-core/src/flow_model.rs:465,587`<br>- `crates/cpg-core/src/usage.rs:63`<br>- `crates/cpg-core/src/validate.rs:212`<br>[I] | The `relations!` macro (`crates/cpg-schema/src/query.rs::relations:105-123`) just copies them [I]. | **None.** `Relation.deps` is read only by `crates/cpg-core/src/catalog.rs::catalog_inputs!:508-513` and `evidence.rs:53` (stage-cache keys) and by a unit test (`query.rs:207`). Nothing compares deps with the SQL text [I]. |

---

### 3.2 `summary_flows` (analysis registry; served as a native file)

| # | Aspect | Hand-written sites | Derived from | Drift cross-check |
|---|---|---|---|---|
| 1 | Arrow schema | **Hand #1:** `behavior.rs::SummaryFlows:1471-1489` (16 fields).<br>**Hand #2:** serving file `crates/cpg-schema/src/bundle.rs::files:378-398`, a full mirror of 15 columns with `snapshot_id` dropped and codebooks as Utf8.<br>**Hand #3:** serving SQL column list `crates/cpg-core/src/bundle.rs::query:414-423`.<br>**Hand #4:** native by-name decode `python/lctx_semantics/src/ipc_input.rs::decode:300-322` into the positional `FlowInput` tuple (`python/lctx_semantics/src/lib.rs::FlowInput:23-33`, 9 of 15 columns).<br>**Hand #5:** wire `ValuePath` (`crates/cpg-schema/src/wire/responses.rs::ValuePath:541-554`), a subset that renames `verdict` to `source_verdict`.<br>All [I]. | `table!` gives Delta and contract. Hand #2 generates the PG DDL (item 8). Producer struct literals are compile-coupled (`lctx-analytics/src/summaries/finite.rs:1055-`, `:1426-`) [I]. | No test compares Hand #2 with Hand #1 [I]. Runtime [I]:<br>- `crates/cpg-core/src/bundle.rs::normalized:611-639` ("the query lacks …");<br>- `serving_projection.rs::validate_batch:499-509` at bundle build (`crates/cpg-core/src/bundle.rs:961`) and PG read-back;<br>- native "schema drift" check (`ipc_input.rs:209-211`);<br>- the `definition_digest` guard (`serving_projection.rs::definition_digest:68-121`, checked at `python/lctx_semantics/src/lib.rs:144` and `crates/cpg-core/src/bundle.rs:1144`).<br>**A new Delta column is silently not served.** `contracts_snapshot` covers the Delta side only [IC]. |
| 2 | Identity recipe | **Hand:** Rust-only `crates/cpg-schema/src/id.rs::recipe::summary_flow:278-297` (`kind::SUMMARY_FLOW:113`). Two hand call sites fill `SummaryFlowIdentity` (`lctx-analytics/src/summaries/finite.rs:1043-1054`, `:1413-1424`) [I]. No SQL form [I]. | n/a | `crates/cpg-core/src/validate.rs::validate_summary_flows:660-725` recomputes the rows, ids included (`summary-flow-source-equality`) [I]. `crates/cpg-schema/tests/ids.rs::summary_identity_retains_the_ordered_evidence_path:26` [IC]. |
| 3 | Sort key | **Hand #1:** `key = [snapshot_id, summary_id]` (`behavior.rs:1466`).<br>**Hand #2:** `ServingFile.key = &["summary_id"]` (`bundle.rs:397`). It is **never consumed** for hand-written files: `ServingFile.key` is read only at `crates/cpg-core/src/bundle.rs:96-111` (catalog files) and `crates/cpg-core/src/retrieval.rs:278-285`.<br>**Hand #3:** `ORDER BY summary_id` (`crates/cpg-core/src/bundle.rs:418`).<br>**`unique_key` has no `summary_flows` arm** (`serving_projection.rs::unique_key:676-713`), so there is no PG `UNIQUE` (`202609270004_projection.sql:478-497`) and no serving duplicate check.<br>All [I]. | `to_sorted_batch` (`attempt.rs::write_analysis:427-436`); `key:summary_flows` rule [I]. | Delta only (`key:` rule) [I]. |
| 4 | Validation | **Hand:**<br>- CHECKs `boundary_iff_unknown` (with `Verdict` codes **hard-coded** as `verdict = 3`, `IN (0, 1)`; compare `codebook.rs::Verdict:1304-1316`) and `depth_nonnegative` (`behavior.rs:1467-1470`);<br>- 7 `REFERENCES` from the table (`rules.rs:922-948`) and 2 into it (`:950-953`, `:978`);<br>- semantic `discharge-cites-summary` (`rules.rs:3488-3505`);<br>- the relation `stored_summary_flows`, whose `deps = ["summary_flows"]` restates `T::DEPS` (`validate.rs:280-281`);<br>- the source-equality validator (`validate.rs:660-725`).<br>All [I]. | Generated: `key:`, `codebook:summary_flows.{kind,verdict,boundary_reason}`, and `catalog-unselected:summary_flows` via `catalog.rs::mandatory_table:309` (`rules.rs:3816-3822`) [I]. Serving text codes are checked by `validate_batch` through `codebook_name` [I]. | `references_name_real_columns`, `rules_snapshot` [IC]. |
| 5 | Codebook / vocabulary | **Hand:**<br>- `text_of::<C>` choices per column in the serving SQL (`crates/cpg-core/src/bundle.rs:420-422`);<br>- `codebook_name` arms `("summary_flows","kind")` (`serving_projection.rs:455`), `(…"summary_flows","boundary_reason")` (`:460`) and wildcard `(_, "verdict")` (`:474`);<br>- PG `CHECK IN` lists frozen in `202609270004_projection.sql:485-488`.<br>All [I]. | Column types `SummaryFlowKind` (`codebook.rs:1431`), `Verdict`, `BoundaryReason` (`:165`) set the metadata and rules; `text_of` renders from `C::all()` (`crates/cpg-core/src/bundle.rs::text_of:77-84`) [I]. | No test compares `codebook_name` with the `lctx.codebook` metadata [I]. The native decode reads `verdict` and `boundary_reason` as plain text (`ipc_input.rs:313-314`) [I]. |
| 6 | Delta write | **Hand:** `write_analysis::<SummaryFlows>` (`attempt.rs:1296`) [I]. | Backstop `persist_unselected` over the registry (`attempt.rs:1646-1651`); rebuild copy (`rebuild.rs:303`) [I]. | Runtime `write::<T>` schema-equality check (`attempt.rs:294-296`) [I]. |
| 7 | Bundle / serving registration | **Hand:**<br>- the file in `bundle.rs:378-398`;<br>- the `NATIVE_FILES` entry (`serving_projection.rs:20`);<br>- the `codebook_name` arms;<br>- the 72-name `cleanup_generation` array (`202609280012_selection_retrieval.sql:179`) and the 30-name `analyze_generation` array (`:192`);<br>- the 72-name pin in the test `crates/cpg-core/tests/bundle.rs::a_generation_rebuilds_to_the_same_bytes:1891` (list at `:1966-2041`).<br>All [I]. | `artifacts_for` (`serving_projection.rs:51-66`) and `relation_requested` (`:177-191`) derive from `NATIVE_FILES` [I]. | `a_generation_rebuilds_to_the_same_bytes` pins the name set [IC]. |
| 8 | PG DDL / migration | Frozen generated text `202609270004_projection.sql:478-497` (no `UNIQUE`); hand arrays (see 7) [I]. | Generated from Hand #2 by `projection.rs::relation_ddl:51-116` at authoring time [I]. | None that compares with the generator. Runtime import and read-back (§0.5) [I]. |
| 9 | PG import mapping | none | `import.rs::load_order:196-216`, `load_batch:585-655`, `validate_stored:657-687`, all driven by `bundle::files` [I]. | Runtime (§0.5). |
| 10 | IPC / native file | **Hand:**<br>- `NATIVE_FILES` entry;<br>- field names in `ipc_input.rs:300-322`;<br>- the `FlowInput` tuple (`lib.rs:23-33`);<br>- `SURFACE_FILES` and `CONDITION_FILES` partitions (`ipc_input.rs:21-28`) plus a duplicate surface-name `matches!` (`:229`).<br>All [I]. | `ipc_input::NAMES = NATIVE_FILES` (`ipc_input.rs:18`); schemas from `bundle::files(0)` (`:193-198`); Python `NATIVE_IPC_FILES` from `native_files()` (`python/lctx_semantics/src/lib.rs::native_files:200-219`, `python/lctx_mcp/src/lctx_mcp/generation.py:16-19`) [I]. | Runtime schema equality (`ipc_input.rs:209-211`). A `NATIVE_FILES` name missing from the bundle files would panic at `schemas[name]` (`ipc_input.rs:206`) [I]. |
| 11 | Wire DTO | **Hand:** `ValuePath` (`responses.rs:541-554`) and `ProofStep` (`:519-523`); contract entry `"ValuePath"` (`dispatch.rs:61`) [I]. | n/a (built from native executor output) | `crates/cpg-schema/tests/wire.rs::generated_wire_schemas_snapshot:29`, wire shape only [IC]. |
| 12 | Hydration SQL | none. The PG copy is imported and validated but not queried: no `queries/*.sql` or `hydration.rs` read of it (grep) [I]. | n/a | n/a |
| 13 | Python model | none (Python only transports native results) [I]. | n/a | n/a |

---

### 3.3 `catalog_parameters` (analysis registry; the best-derived sample)

| # | Aspect | Hand-written sites | Derived from | Drift cross-check |
|---|---|---|---|---|
| 1 | Arrow schema | **Hand #1:** `catalog.rs::CatalogParameters:111-117` (18 fields, with serde and schemars attributes).<br>**Hand #2:** wire `crates/cpg-schema/src/wire/responses.rs::CatalogParameter:170-193`, 17 fields that omit `snapshot_id` and `documentation`, with `Option`s lacking `required_nullable`. This duplicates the row even though `CatalogParametersRow` already derives `Serialize`, `Deserialize` and `JsonSchema` (`catalog.rs:109`) and sibling DTOs are type aliases of rows (`responses.rs:167-168,214-221`).<br>Producer struct literals are compile-coupled (`crates/cpg-core/src/catalog.rs:1059-1085`, `:1147-1166`).<br>All [I]. | **Derived** [I]:<br>- serving file `catalog.rs::serving_files:128-164` (entry `:149`; `snapshot_id` dropped at `:131-139`);<br>- serving SQL columns and `ORDER BY` (`crates/cpg-core/src/bundle.rs::query:96-112`);<br>- PG DDL (generated, frozen at `202609280009_catalog.sql:78-102`);<br>- typed PG decode through the row's serde (`crates/lctx-postgres/src/selection.rs::typed:160-185`, target `cpg-schema/src/selection/catalog.rs::Inputs:95`);<br>- `serving_projection.rs::projected_rows:1030-1051` (used at `retrieval/catalog.rs:305`). | `contracts_snapshot` and `generated_wire_schemas_snapshot` are separate snapshots [IC]. No DTO-vs-row test [I]. Runtime: hydrated packets are re-decoded (`crates/lctx-postgres/src/hydration.rs::packet:11-19`) [I]. |
| 2 | Identity recipe | **Hand:**<br>- the composite key `(signature_id, ordinal)`;<br>- `signature_id` comes either from the derived SQL `s.node_id AS signature_node_id` (`derived.rs:396`) or from `IdHasher::new("provider-signature")` (`crates/cpg-core/src/catalog.rs:1116-1119`), whose kind literal is outside `id::kind`;<br>- `ordinal` comes from the `Parameters` derivation (`derived.rs:421-451`).<br>All [I]. | n/a | `catalog-source-equality:catalog_parameters` (`crates/cpg-core/src/catalog.rs::validate:1410`, `check!` at `:1422-1442`, `:1471`) recomputes [I]. The test `crates/cpg-core/tests/catalog.rs::catalog_profile_publishes_original_contracts_without_native_analysis:8` forges `default_text` and expects that rule (`:415-439`) [IC]. |
| 3 | Sort key | **Hand #1:** `key = [snapshot_id, signature_id, ordinal]` (`catalog.rs:110`).<br>**Hand #2:** `unique_key` `"signature_id,ordinal"` (`serving_projection.rs:693`), which becomes PG `UNIQUE` (`202609280009_catalog.sql:101`).<br>**Hand #3:** hydration sorts by `ordinal` (`crates/lctx-postgres/src/hydration.rs:174`).<br>All [I]. | Serving key `&T::key()[1..]` (`catalog.rs:139`), which feeds the serving `ORDER BY` (`crates/cpg-core/src/bundle.rs:106-110`); `to_sorted_batch`; the `key:` rule [I]. | `serving_projection.rs::validate_relations:903-910` checks `unique_key` uniqueness [I]. Nothing checks that `unique_key` equals `T::key()[1..]` [I]. |
| 4 | Validation | **Hand:**<br>- CHECK `ordinal >= 0` (`catalog.rs:110`), which is **not** carried into PG;<br>- `REFERENCES` ×3 (`rules.rs:113-127`);<br>- serving `foreign_keys` `catalog_parameters.signature_id → catalog_signatures` (`serving_projection.rs:797-802`), a second hand copy of the first `REFERENCES` entry;<br>- the composite serving FK `catalog_field_links(signature_id,ordinal) → catalog_parameters` (`:773-778`, PG `202609280010_specificity.sql:97`), which has **no store-side equivalent**: `REFERENCES` has only `catalog_field_links.signature_id → catalog_signatures` (`rules.rs:88-92`).<br>All [I]. | Delta CHECK; generated rules `key:` and `catalog-code:catalog_parameters.{kind,provider_kind,default_state}` (`rules.rs:3800-3815`); `validate_batch` vocabulary check (`serving_projection.rs:517-525`) [I]. | `references_name_real_columns` [IC]; the source-equality validator above [I]. |
| 5 | Codebook / vocabulary | **Hand:**<br>- the `default_state` list (`catalog.rs:288-295`), which overlaps a separate hand list for `catalog_configurations` (`catalog.rs:258-266`);<br>- the arm keys `("catalog_parameters","kind"\|"provider_kind")` (`:296-299`);<br>- the producer's literal strings (`crates/cpg-core/src/catalog.rs::default_value:425-453`, `:1052-1054`);<br>- the wire adapters keyed by string, `vocabulary!(DefaultState/ParameterKind/ProviderParameterKind, "catalog_parameters", …)` (`wire/vocabulary.rs:63-83`).<br>All [I]. | The `kind` and `provider_kind` values come from `ParameterKind::all()` texts (`catalog.rs:299`, `codebook.rs:324`). The same vocabulary drives the `catalog-code` rule, the PG `CHECK` (frozen at `202609280009_catalog.sql:84,89,92`), `validate_batch`, `definition_digest` (`serving_projection.rs:97-104`) and the serde decoders on the row (`catalog.rs:113-117`) [I]. | `catalog-code:` rules check producer strings at publication [I]. A missing vocabulary key panics when the schema is generated (`wire/vocabulary.rs:8`), which `generated_wire_schemas_snapshot` exercises [IC]. The PG `CHECK` does not track later list changes [I]. |
| 6 | Delta write | **Hand, string- or macro-listed** [I]:<br>- `write_analysis::<CatalogParameters>` (`attempt.rs:1460-1466`);<br>- `Contracts.parameters` (`crates/cpg-core/src/catalog.rs::Contracts:395`);<br>- `normalization_tables!` (`crates/cpg-core/src/stage_cache.rs:128-146`, entry `:134`);<br>- `contract!` (`rebuild.rs:255-281`, entry `:267`);<br>- `check!` (`crates/cpg-core/src/catalog.rs:1471`). | Backstop `persist_unselected`; rebuild `copy!` over the registries [I]. | The stage-cache decode counts relations (`stage_cache.rs:230-234`) [I]. |
| 7 | Bundle / serving registration | **Hand:**<br>- the entry list in `catalog.rs::serving_files:144-163` (only the list is hand-written; schema and key are derived);<br>- `unique_key`;<br>- `foreign_keys`;<br>- the migration arrays (`0012:179,192`);<br>- the bundle test pin;<br>- `Requirement::dependencies` string lists naming `"catalog_parameters"` (`crates/cpg-schema/src/wire/requirements.rs::dependencies:366-414`, at `:377,388,397,407`) and the consumer's name match (`crates/lctx-postgres/src/selection.rs:276`).<br>All [I]. | Schema, key and `ORDER BY` derive from `table!` [I]. | The dependency lists are checked by `pr4::check`, which omits undeclared inputs and compares answers (`crates/cpg-core/tests/catalog/pr4.rs:95-130`, via `catalog_profile_publishes_original_contracts_without_native_analysis`) [IC]. |
| 8 | PG DDL / migration | Frozen generated text `202609280009_catalog.sql:78-102`, FK `:210`, FK from `catalog_field_links` at `202609280010_specificity.sql:97` [I]. | Generated at authoring time [I]. | None that compares with the generator [I]. |
| 9 | PG import mapping | **Hand:**<br>- invocation projection nulls `"documentation"` by name (`crates/lctx-postgres/src/repository.rs::fetch_projection:293-296`);<br>- the lookup key `"signature_id"` (`hydration.rs:148-156`), validated at runtime against the bundle schema (`repository.rs:286-288`).<br>All [I]. | Generic import from `bundle::files` [I]. | Runtime (§0.5) [I]. |
| 10 | IPC / native file | Hand list in stage-cache IPC (`normalization_tables!`) [I]. | Bundle `catalog_parameters.arrow` is derived; not native [I]. | Stage-cache count check [I]. |
| 11 | Wire DTO | **Hand:** `CatalogParameter` (`responses.rs:170-193`), nested in `CatalogSignature.parameters` (`:195-213`); contract entry `"CatalogParameter"` (`dispatch.rs:36`) [I]. | n/a | `generated_wire_schemas_snapshot` [IC]. |
| 12 | Hydration SQL | No `queries/*.sql`. The generic `SELECT` column list is built from the serving file (`repository.rs::fetch_projection:273-362`) [I]. | Derived [I]. | Runtime `decode_rows` [I]. |
| 13 | Python model | none [I] | n/a | n/a |

---

### 3.4 `behaviors` (analysis registry; served, then hydrated into `Fate`)

| # | Aspect | Hand-written sites | Derived from | Drift cross-check |
|---|---|---|---|---|
| 1 | Arrow schema | **Hand #1:** `behavior.rs::Behaviors:1127-1177` (30 fields).<br>**Hand #2:** serving file `crates/cpg-schema/src/bundle.rs::files:297-327`. It has 23 columns: it drops 7 (`parameter_node_id`, `target_node_id`, `site_node_id`, `site_module_node_id`, `site_start_byte`, `site_end_byte`, `invocation_id`), adds the joined `callee` and `path`, and renames `site_line` to `line`.<br>**Hand #3:** serving SQL `crates/cpg-core/src/bundle.rs::query:373-393`.<br>**Hand #4:** hydration pick list `crates/lctx-postgres/src/hydration.rs::fate:568-613` (16 names plus 3 renames: `condition_scope_node_id`→`condition_scope_id`, `parameter_name`→`parameter`, `target_name`→`target`).<br>**Hand #5:** wire `Fate` (`responses.rs::Fate:90-138`).<br>All [I]. Test-only read: `python/lctx_mcp/tests/test_operations.py:64` [I]. | Delta and contract from `table!`. The PG DDL is generated from Hand #2 and frozen (`202609270004_projection.sql:366-392`) [I]. | No static test [I]. Runtime [I]:<br>- `serde_json::from_value::<Fate>` fails as "fate contract" (`crates/lctx-postgres/src/packet.rs:366-367`);<br>- `validate_batch` at build and import;<br>- `normalized` catches missing SQL columns. |
| 2 | Identity recipe | **Hand:** `behavior.rs::behavior_id:1800-1820` (kind literal `"behavior"`) and `flow_behavior_id:1823-1838` (`"flow-behavior"`). Both are called by hand at `crates/cpg-core/src/behavior.rs:575,615,681,764,849` [I]. | n/a | No publication validator recomputes `behavior_id`: `validate_costed` (`validate.rs:156-202`) has no behaviors source-equality step [I]. Only `crates/cpg-core/tests/compile.rs::transfer_alternatives_keep_conditions_verdicts_and_receiver_boundaries:170` does (`:559-570`) [IC]. Duplicates are refused by the producer (`crates/cpg-core/src/behavior.rs:1345-1355`) and by `key:behaviors` [I]. |
| 3 | Sort key | **Hand #1:** `key = [snapshot_id, behavior_id]` (`behavior.rs:1124`).<br>**Hand #2:** `ServingFile.key` `&["behavior_id"]` (`bundle.rs:326`, never consumed).<br>**Hand #3:** `ORDER BY b.behavior_id` (`crates/cpg-core/src/bundle.rs:386`).<br>**Hand #4:** `unique_key` `"behavior_id"` (`serving_projection.rs:699`), which becomes PG `UNIQUE` at `202609270004_projection.sql:391`.<br>**Hand #5:** `ORDER BY behavior_id` in `crates/lctx-postgres/queries/packet_behaviors.sql`.<br>**Hand #6:** the `behavior_owner` index, created in `202609280005_publication.sql:163` and expected by `import.rs::check_indexes:711-715`.<br>All [I]. | `to_sorted_batch`; producer sort (`crates/cpg-core/src/behavior.rs:1345`) [I]. | `validate_relations` checks uniqueness against `unique_key` [I]; `check_indexes` checks the index definition at runtime [I]. |
| 4 | Validation | **Hand:**<br>- CHECK `depth_nonnegative` (`behavior.rs:1125`);<br>- `REFERENCES` for `condition_scope_node_id` (`rules.rs:184-188`);<br>- semantic rules at `rules.rs:3460,3471,3488,3506,3524,3553,3564,3585`;<br>- serving FK `behaviors.operation_node_id → operations` (`serving_projection.rs:861`), which becomes PG `202609270004_projection.sql:1116`.<br>All [I]. | Generated: `key:`, `codebook:behaviors.{kind,transfer,verdict,boundary_reason,phase}`, `catalog-unselected:behaviors` [I]. | `references_name_real_columns`, `rules_snapshot` [IC]. `crates/cpg-core/tests/analysis.rs:1957` targets `semantic:call-transfer-discharged` [IC]. |
| 5 | Codebook / vocabulary | **Hand:**<br>- `text_of` choices (`crates/cpg-core/src/bundle.rs:387-391`);<br>- `codebook_name` arms (`serving_projection.rs:445-447,460,474`). These are **order-sensitive**: `("behaviors"\|"ambient_reads","phase")` must precede the wildcard `(_, "phase"\|"producer_phase") => "invocation_phase"` (`:478`);<br>- PG `CHECK` lists frozen at `202609270004_projection.sql:371-372,380-381,384`;<br>- `BehaviorKind` **texts hard-coded** in `crates/lctx-postgres/queries/packet_behaviors.sql` and `packet_behavior_count.sql` (`'delegates','supplies_literal','hands_off_to','takes_from'`);<br>- wire `FateTransfer` (`responses.rs:11-18`), a hand mirror of the `FlowTransfer` codebook (`codebook.rs:1403-1407`);<br>- `Fate.kind`, `verdict` and `phase` typed as `String`.<br>All [I]. | Column types `BehaviorKind` (`codebook.rs:1269`), `FlowTransfer`, `Verdict`, `BoundaryReason`, `ReadPhase` (`:1568`) set the metadata and rules [I]. | `sqlx-check` validates the SQL columns but not the literal kind texts [IC]. A new `FlowTransfer` value would fail `Fate` decoding at runtime [I]. |
| 6 | Delta write | **Hand:** `write_analysis::<Behaviors>` (`attempt.rs:1338`) [I]. | Backstop over the registry [I]. | Runtime schema check [I]. |
| 7 | Bundle / serving registration | **Hand:**<br>- the file (`bundle.rs:297-327`);<br>- `relation_requested` name match (`serving_projection.rs:184`);<br>- `unique_key`;<br>- `foreign_keys`;<br>- `codebook_name`;<br>- the migration arrays;<br>- the bundle test pin.<br>All [I]. | n/a | Test pin `a_generation_rebuilds_to_the_same_bytes` [IC]. |
| 8 | PG DDL / migration | Frozen `202609270004_projection.sql:366-392` and FK `:1116`; hand index `202609280005_publication.sql:163` [I]. | Generated at authoring time [I]. | Runtime `check_indexes` (index); import and read-back (columns) [I]. |
| 9 | PG import mapping | `check_indexes` entry (`import.rs:711-715`) [I]. | Generic [I]. | Runtime [I]. |
| 10 | IPC / native file | Not native. Bundle `behaviors.arrow` only [I]. | Bundle writer [I]. | n/a |
| 11 | Wire DTO | **Hand:** `Fate` and `Discharge` (`responses.rs:78-138`); contract entry `"Fate"` (`dispatch.rs:30`) [I]. | n/a | `generated_wire_schemas_snapshot` [IC]. |
| 12 | Hydration SQL | **Hand:** `crates/lctx-postgres/queries/packet_behaviors.sql` and `packet_behavior_count.sql` (via `packet.rs:335-351`); generic fetch by `"behavior_id"` (`packet.rs:353-355`); the `fate()` mapping [I]. | Generic column list from the serving file [I]. | `just sqlx-check` (`scripts/postgres_check.py:112-117`) for `query_file!` SQL against the migrated DDL [IC]. |
| 13 | Python model | none (test-only read) [I] | n/a | n/a |

---

### 3.5 `retrieval_units` (no `table!`, no registry, no Delta; serving-only artifact)

| # | Aspect | Hand-written sites | Derived from | Drift cross-check |
|---|---|---|---|---|
| 1 | Arrow schema | **Hand (sole authority):** `retrieval.rs::files:270-274` (`unit_id` FSB16, `family` Utf8, `detail` Utf8).<br>**Payload authority:** `retrieval.rs::Unit:78-87`, serialized as JSON into `detail`. Columns are built positionally against that schema (`crates/cpg-core/src/retrieval.rs::materialize:101-122`).<br>**Hand JSON keys in SQL:** `'text'` in `crates/lctx-postgres/queries/evidence_search_unit.sql`, and `detail::jsonb->>'source_key'` in `queries/evidence_eligible.sql`.<br>All [I]. | PG DDL generated from `files()`, frozen at `202609280012_selection_retrieval.sql:26-40` [I]. `RetrievalUnitHeader` is compile-coupled through `From<&Unit>` (`wire/evidence.rs:94-116`) [I]. | Runtime: `RecordBatch::try_new` type and arity; replay schema equality (`crates/cpg-core/src/retrieval.rs::restore:534-543`); `validate_batch` [I]. Nothing checks the JSON keys used in SQL (`sqlx-check` sees only columns) [I]. |
| 2 | Identity recipe | **Hand:** `retrieval.rs::unit_id:132-139` (`"retrieval-unit-v1"`); fragment recipe `:141-175` [I]. | n/a | `retrieval.rs::validate_projection:389-399` (identity/family) and `:445-450` (whole-rendering equality with `catalog::derive`) run at bundle build (`crates/cpg-core/src/bundle.rs:993`) and PG read-back (`import.rs:686`) [I]. The test `crates/cpg-core/tests/catalog/pr4.rs:335` (forged units) and `:458` (corrupt replay) belongs to `catalog_profile_publishes_original_contracts_without_native_analysis` [IC]. |
| 3 | Sort key | **Hand #1:** `files` key `&["unit_id"]` (`retrieval.rs:273`).<br>**Hand #2:** `unique_key` `"unit_id"` (`serving_projection.rs:683`), which becomes PG `UNIQUE` (`0012:33`).<br>All [I]. | `canonical_sort` by `file.key` (`crates/cpg-core/src/retrieval.rs:278-285`) [I]. | Duplicate check in `validate_projection` (`retrieval.rs:440`) and in `validate_relations` [I]. |
| 4 | Validation | **Hand:**<br>- the validator `retrieval.rs::validate_projection:322-567` and `validate_snapshot:569-589`;<br>- serving FKs `retrieval_fragments` and `retrieval_subjects` → `retrieval_units` (`serving_projection.rs:723-734`), which become PG `0012:106-107`.<br>All [I]. | Invoked from `serving_projection.rs::validate_relations:914` [I]. | As in row 2 [IC]. |
| 5 | Codebook / vocabulary | **Hand:** `family` values come from `retrieval::VIEWS` names (`retrieval.rs:29-58`). The serde `rename_all = "snake_case"` on `Family` (`:16`) is a parallel spelling that must agree with those names. The PG DDL has **no `CHECK` on `family`** (`0012:30`), because `relation_ddl` consults only `catalog::vocabulary` and `codebook_name` (`projection.rs:73-92`) [I]. | `Family::name()` [I]. | Only `validate_projection` (`retrieval.rs:395`) [I]. |
| 6 | Delta write | **None.** Written instead as an IPC replay receipt (`crates/cpg-core/src/retrieval.rs::save:436-478`); `restore:480-554` carries a **hand allow-list of 7 names** (`:519-527`) [I]. | n/a | Replay digest and schema checks (`:532-543`) [I]. |
| 7 | Bundle / serving registration | **Hand:**<br>- `retrieval::files` (appended at `bundle.rs:824`);<br>- the `artifacts_for` name chain (`serving_projection.rs:57-63`);<br>- `unique_key`;<br>- `foreign_keys`;<br>- the restore allow-list;<br>- the migration arrays;<br>- the bundle test pin.<br>All [I]. | n/a | Test pin [IC]. `Manifest` artifact inventory equality (`serving_projection.rs:393-401`) [I]. |
| 8 | PG DDL / migration | Frozen `202609280012_selection_retrieval.sql:26-40`, FKs `:106-107`, indexes `:140-141` [I]. | Generated at authoring time [I]. | None that compares with the generator [I]. |
| 9 | PG import mapping | none | Generic [I]. | Runtime [I]. |
| 10 | IPC / native file | Generation file `retrieval_units.arrow` plus the replay directory; not native. Python reads `retrieval_fragments` and `retrieval_subjects` but not units (`python/lctx_mcp/src/lctx_mcp/generation.py:108-109`) [I]. | n/a | n/a |
| 11 | Wire DTO | `Unit` (`retrieval.rs:78-87`) is the payload DTO. `RetrievalUnitHeader` is derived from it (`wire/evidence.rs:103-116`) [I]. | Compile-coupled [I]. | `generated_wire_schemas_snapshot` [IC]. |
| 12 | Hydration SQL | **Hand:** `crates/lctx-postgres/queries/unit_detail.sql`, `evidence_search_unit.sql` and `evidence_eligible.sql`, each decoded to `Unit` by serde (`crates/lctx-postgres/src/evidence.rs:397-407`, `evidence_search.rs:185-196`) [I]. | `families` parameters come from `Family::name()` (`evidence_search.rs:50-54`) [I]. | `sqlx-check` covers columns, not JSON keys [IC]. Serde decode fails at runtime as "unit contract" [I]. |
| 13 | Python model | none [I] | n/a | n/a |

---

### 3.6 Conclusion

#### 3.6.1 Aspects already derived from `table!` (one authority)

These are [I] unless marked.

- **Delta**: schema, CHECK constraints, append-only verification, canonical sort, read ordering and
  per-snapshot schema digest (`delta.rs:61-146`, `table.rs:47-129`, `attempt.rs:289-302,356-369`).
- **Rust rows**: the row struct and its Arrow codec (`table.rs:158-208`). Codebook column typing
  follows through `lctx.codebook` metadata (`column.rs:277-320`).
- **Generated validation**: `key:`, `codebook:`, `finite:`, `fact:`, `catalog-code:` and
  `catalog-unselected:` rules (`rules.rs:3787-3930`).
- **Registry-driven loops** (§0.2): write, derive, initialize, backstop, rebuild copy and stage-cache
  bind.
- **The 18 catalog serving files**, including `catalog_parameters`, and their downstream chain:
  - serving schema and key (`catalog.rs:128-161`);
  - serving SQL (`crates/cpg-core/src/bundle.rs:96-112`);
  - PG DDL at authoring time;
  - generic PG import and read-back;
  - typed PG decode through the row's own serde (`selection.rs:160-185`).
- **Graph rules** (`graph.rs:1614-1676`): generated from the hand `edge_sources` registry, which is one
  authority for edge kinds.

#### 3.6.2 Aspects declared twice or more by hand (true second authorities)

Ranked by breadth, then by the absence of a cross-check.

1. **Serving Arrow schema vs Delta `table!` schema.**
   - The 49 hand-written serving files include full mirrors (`summary_flows`, `bundle.rs:378-398`) and
     renaming projections (`behaviors`, `bundle.rs:297-327`). Each has a separate hand SQL text
     (`crates/cpg-core/src/bundle.rs::query:113-495`).
   - No static test compares them. Detection is runtime-only, by column name.
   - Adding a Delta column is silently not served [I].
   - `catalog::serving_files` shows that the derived path exists: 18 files already use it [I].
2. **PG DDL vs serving Arrow schema.**
   - Generated by `relation_ddl`, but frozen into migrations and then edited by hand (`0012:111`).
   - Codebook and vocabulary value lists are frozen as `CHECK IN` clauses, so every value appended to an
     append-only codebook on a served column needs a hand migration.
   - The PL/pgSQL relation arrays (`0012:179`, `:192`) are further hand inventories.
   - **No test compares `relation_ddl()` with the migrations.** The detectors are `sqlx-check`
     (query SQL vs DDL) [IC] and runtime import and read-back [I].
3. **Wire DTOs vs row columns.**
   - `wire::CatalogParameter` duplicates `CatalogParametersRow`, although the row is already
     serde/JsonSchema-capable and sibling catalog DTOs alias their rows.
   - `Fate` duplicates the `behaviors` serving file with renames, and `fate()` is a third hand list
     between the two.
   - `ValuePath` is a hand subset of `summary_flows`.
   - `FateTransfer` and `wire::DischargeDecision` mirror codebooks.
   - The only cross-check is the wire-shape snapshot [IC]; the rest is runtime serde [I].
4. **"Column X uses codebook C" is stated in three places** for every served codebook column:
   - the `table!` field type;
   - the `text_of::<C>` call in the serving SQL;
   - `serving_projection::codebook_name`, which is string-keyed and order-sensitive through its
     wildcards.

   On top of that, codebook codes and texts appear as literals in a `table!` CHECK (`summary_flows`
   `verdict = 3`) and in hand SQL (`packet_behaviors.sql`). No cross-check was found [I].
5. **Keys and uniqueness.**
   - For hand-written serving files, the key is restated in `ServingFile.key` (never consumed), in the
     SQL `ORDER BY` and in `unique_key`.
   - `summary_flows` has no `unique_key` entry, so PG enforces no uniqueness although the Delta key is
     total.
   - `unique_key` equals `T::key()[1..]` for catalog tables, but by hand [I].
6. **Referential integrity.**
   - `rules::REFERENCES` (store) and `serving_projection::foreign_keys` (serving and PG) are independent
     hand lists.
   - They already diverge: the composite `catalog_field_links → catalog_parameters` exists only on the
     serving side [I].
7. **Relation-name inventories.** The same name set is written by hand in:
   - `NATIVE_FILES` (29);
   - `artifacts_for`;
   - the retrieval `restore` allow-list;
   - `relation_requested`;
   - `check_indexes`;
   - `cleanup_generation` (72) and `analyze_generation` (30);
   - the bundle test pin (72);
   - `catalog::serving_files`;
   - `stage_cache::normalization_tables!`;
   - the `rebuild.rs` `contract!` list;
   - the `Contracts` struct;
   - the catalog `check!` list;
   - `Requirement::dependencies`;
   - the `selection.rs` `load!` match;
   - the `keys_and_checks_name_real_columns` test list (26, which omits all five sampled tables).

   The only cross-checks are the bundle test pin and the `pr4::check` omission test [IC], plus runtime
   inventory equality in `Manifest::validate` [I].
8. **Relation `deps` vs SQL text** (`relations!`, e.g. 9 lists naming `edges`, and
   `stored_summary_flows` restating `T::DEPS`). They are hand-written and unchecked; they feed only
   stage-cache keys [I].
9. **The three table registries and `codebook::registry()`.** Each is a single hand list with
   downstream derivation. They are set-equal to their declarations at `35afc09` by my comparison, but
   no test enforces completeness [I].
10. **Identity recipes.** The kind strings are literals outside `id::kind` (`"behavior"`,
    `"flow-behavior"`, `"provider-signature"`, `"retrieval-unit-v1"`), and `kind::EDGE` is declared
    but unused [I].
    - `edges` has a SQL-only recipe, pinned only by the SQL-text snapshot [IC].
    - `behaviors` has a Rust-only recipe with no publication-time recompute [I].
11. **Vocabularies.** Five sources state value lists:
    - hand `default_state` lists (`catalog_parameters` and `catalog_configurations`);
    - literal producer strings in `default_value`;
    - wire `vocabulary!` adapters keyed by (relation, field) strings;
    - the separate `retrieval::VIEWS` vocabulary, which has no PG `CHECK`;
    - JSON keys written into SQL that mirror `Unit` serde fields.

    Of these, only the producer strings are cross-checked at publication, by the `catalog-code:`
    rules [I].

#### 3.6.3 Per-sample verdict

| Table | Aspects with at least two hand authorities |
|---|---|
| `catalog_parameters` | Least duplicated. Only the wire DTO, `unique_key`, the FK and vocabulary lists, and the hand name lists repeat it. |
| `edges` | Only its identity-kind literal, relation deps and a test list. It is not served. |
| `summary_flows` | Schema ×5, key ×3 (uniqueness registry missing), codebook mapping ×3, name inventories. |
| `behaviors` | Schema ×5, key ×6, codebook mapping ×3 plus literal SQL texts, wire. |
| `retrieval_units` | It has no `table!` at all, so every aspect is a hand authority: schema, key, vocabulary and name lists. |

All rows are [I].


## 4. Pipeline stage table (compile attempt and PR5 coarse rebuild)

All rows **Implemented** (code path read at 35afc09) unless labelled otherwise.

### 4.1 Entry: `compile_catalog_mode` and raw writes

| # | Step | Inputs read | Outputs written | Effects | Pure? | Reuse boundary |
|---|---|---|---|---|---|---|
| 0 | Admission `cpg-core/src/attempt.rs::compile_catalog_mode:568-583` | `CompileInputs` (roots, profile), optional `Analysis` | none | refuses profile/roots/enrichment disagreement | yes | none |
| 1 | Model binding `attempt.rs::bind_models:609-648` (behavioral only) | raw `context_definitions`, `contexts`, `context_modules`, `context_parameters` batches + committed model catalog (`models/external.toml`) | in-memory `BoundModels` | none | yes | none (recomputed every attempt) |
| 2 | Compiler run/producer rows `attempt.rs::with_compiler_run:373-424` | raw `runs` (exactly one run declaring `exports`) | appends compiler `runs`/`producers` rows; `producer_id` from `semantic_digest()` (`analyze.rs::compiler_rows:179-211`), `run_id` over release, context, producer, `inputs.digest()` | none | yes | none |
| 3 | Raw writes `attempt.rs::write_raw:305-336` via `write:289-302` | every `for_each_table!` batch exactly once; schema equality, foreign-snapshot refusal | one Delta commit per raw table (`delta.rs::append:166-193`, `lctx.snapshot_id` commit metadata) | Delta appends (invisible until `snapshots` row) | effectful | none |

### 4.2 `attempt.rs::finish:696-1709`

| # | Step (lines) | Inputs read | Outputs written | Effects | Pure? | Reuse boundary |
|---|---|---|---|---|---|---|
| F1 | Session `:711` (`snapshot.rs::session:174-189`, `register:156-172`) | versions of raw tables, filtered to `snapshot_id`, commit-scoped providers | DataFusion session | none | yes | none |
| F2 | Derivations `:719-724` (`for_each_derived_table!`, `derive.rs::derive:23-43`, `write_derived:338-353`) | raw tables via one SQL per table (`Derived::sql`) | 22 derived tables in hand-kept "dependency order" (`cpg-schema/src/derived.rs::for_each_derived_table:1168-1195`) incl. `nodes`, `edges`, `graph_gaps`, `edge_kinds` | Delta appends, each re-registered | SQL pure | none; always recomputed. Rebuild reuse path **copies** them (`rebuild.rs::copy_publication:295-303`) |
| F3 | Public paths `:729-734` | `cpg_schema::public::public_paths()` with `$roots` | in-memory `public` (written at F6) | none | yes | parameter of the normalization key |
| F4 | Planning relations `:737-740` | — | registers a typed **empty** batch for every `for_each_analysis_table!` table | none | yes | a read of an analysis table before its writer returns the empty relation, not an error |
| F5 | **Catalog stage cache** `:741-772` → `stage_cache.rs::contracts:250-371` | `catalog::load_facts` (25 tables + 4 relations, `catalog.rs::catalog_inputs!:506-546`), `evidence::load` (13 tables, `evidence.rs::evidence_inputs!:51-73`) | in-memory `Contracts` + `Vec<Step>` | disposable file writes under `<store>/rebuild-cache/{normalization,association}/` (`stage_cache.rs::write:98-127`); failure only warns (`save:235-248`) | derivation pure (`catalog.rs::derive_contracts:621-628`, "no database, embedding, acquisition or ambient provider reads") | **Stage `ContractNormalization`, `ContextualAssociation`**; see §4.3 |
| F5' | Error path `:761-771` | the session | — | runs relational rules to turn a typed decoding failure into named violations | yes | — |
| F6 | Canonical scalar tables `:773-857` | `inputs`, `public`, `BoundModels` | `catalog_compilation` (profile, roots, `input_digest = inputs.digest(analysis)`), `public_paths`, `model_context_protocols`, `model_targets`, `model_transfers/effects/callbacks/resources/exceptions/formal_paths` | Delta appends | yes | none |
| F7 | Model SQL analyses `:858-913` (`write_analysis_query:440-458`) | session incl. F6 | `model_applications`, `model_argument_bindings`, `modeled_{callback,resource,transfer,effect,exception}_sites` | Delta | SQL pure | none |
| F8 | Flow model / entry links `:917-928` (behavioral only) | session | in-memory `FlowModelRows`, entry links | none | yes | none |
| F9 | **Behavioral block** `:931-1348` (behavioral profile) | session; `catalog` contracts (`configurations`, `field_links`, `behavior.rs:1127-1143`); finite-summary decisions | ~60 tables in fixed call order (conditions, value flows, predecessor candidates/compatibility, handler*, source call/body/completion, certificates, summary components/flows/steps/boundaries/coverage, guards, parameter reads, handoffs, delegations, operations, facets, behaviors, behavior steps, discharges) | Delta; `behavior::run` may call the embedder (`:1314-1325`, `&mut embeddings`) | mostly pure; embeddings effectful | **Stage `BehavioralEnrichment`** only at whole-compilation granularity (rebuild copies all of it) |
| F9' | Catalog-profile branch `:1349-1386` (`catalog.rs::populate:66-79`) | `operation_sources`, `catalog_qualified_names`, concept attributes | `operations`, `operation_facets`, `operation_facet_status` | Delta | yes | none |
| F10 | Contextual + normalization outputs `:1387-1499` | `Contracts` | 5 contextual tables (`catalog_artifacts/spans/scenarios/deployments/associations`), `catalog_selection_domains`, 11 catalog tables (`surfaces, configurations, field_links, constructors, bindings, signatures, parameters, evidence, types, type_args, type_observations`) | Delta | yes | outputs of F5 |
| F11 | Stage E `:1504-1518` (`analyze.rs::run:555-`) behavioral only | `projection::invocation()` via `analyze.rs::project:547-552`, plus ~18 further SQL acquisitions interleaved with passes (`passages_sql`, `api_texts_sql`, `shared_types_sql`, `co_mention_sql`, `co_use_sql`, `flows::*_sql`, `concepts::attributes_sql`) | in-memory `AnalysisRows` | embedder calls (E0, `analyze.rs:597-680`) | passes pure (lctx-analytics has no DataFusion/Delta dependency: `crates/lctx-analytics/Cargo.toml [dependencies]`) | none |
| F12 | Stage E writes `:1519-1548` | `AnalysisRows` + behavior invocations | `analysis_invocations`, `concept_attributes`, `concept_incidences`, `findings`, `finding_members`, `witnesses` | Delta | yes | none |
| F13 | Stage F `:1551-1566` (`synth::run`) | findings, public | `SynthRows`; brief documents embedded | embedder calls | mostly pure | none |
| F14 | Embedding receipt `:1567` | embedding session | `Receipt` (spec, values, uses, digest) | none | yes | — |
| F15 | **Brief-status write-back** `:1568-1592` | `made.skipped`, `made.briefs`, profile | mutates `catalog.members[*].brief_status/brief_reason`, then writes `catalog_members` | Delta | yes | cross-stage: a normalization output finalized from Stage F; rebuild reuse re-applies it from the source (`stage_cache.rs::finish_members:373-395`); validator neutralizes it (`catalog.rs::validate:1476-1481`) |
| F16 | Embedding + synthesis tables `:1593-1645` | receipt, `SynthRows` | `used_embeddings`, `embedding_uses`, `evidence`, `assertions`, `assertion_support`, `briefs`, `brief_assertions`, `brief_members`, `brief_documents`, `embedding_specs`, `assertion_policy` | Delta | yes | — |
| F17 | **Persist unselected** `:1646-1651` | `written.versions` | every analysis table not yet written is written **empty** | Delta | yes | membership is implicit: which producer owns which table is not declared |
| F18 | Validation `:1658-1682` (`validate.rs::validate_costed:156-`) | whole session cached in memory (`validate.rs::cached_session:71-110`) | violations/costs | none | yes | catalog validator **re-derives** all contracts (`catalog.rs::validate:1410-1497` via `catalog.rs::contracts:457-470`) and compares canonical batches |
| F19 | Content digest + snapshot rows `:1684-1698` | run ids, embedding receipt | `SnapshotsRow` per table (version, schema digest, full `compiler_digest()`, row count) | none | yes | — |
| F20 | Publish `:1699`, `attempt.rs::publish:1714-1728` | `snapshots` | one `snapshots` append; ambiguous commit resolved by re-reading | Delta commit = visibility | effectful | the only visibility boundary |

After `finish`, the CLI bundles (`crates/lctx/src/main.rs:705-711`, `cpg-core/src/bundle.rs::build_with_retrieval:827-1101`) and records the generation in PG; import/selection is a separate explicit act (see §6).

### 4.3 PR5 `Stage` enum, stage-cache and rebuild keys

`cpg-core/src/rebuild.rs::Stage:22-29` = `Facts, ContractNormalization, ContextualAssociation, BehavioralEnrichment, CatalogFinalization, Retrieval`; `Outcome:32-36` = `Reused, Recomputed, NotRequested`; `Step:38-43` carries hand-written `dependencies` (e.g. `stage_cache.rs:351,362`; `rebuild.rs:186,217,227`; `rebuild.rs:395`). The receipt is "explanatory, never consulted to admit reuse or readiness" (`crates/lctx/src/rebuild.rs:154`). Distinct from `cpg_schema::metrics::Stage` (timing marks). **Implemented.**

| Key | Composition (exact) | What is compared for admission | Where |
|---|---|---|---|
| `normalization` | `IdHasher("pure-stage-v1")` ∘ `sha256`(for each dep in sorted `BTreeSet`: name ‖ len ‖ Arrow IPC **stream** bytes of the table read from the session, `to_declared`, canonical-sorted by `T::key()`, `snapshot_id` rebound to `Id::ZERO`) ∘ `parameters` = `json(roots) + ":" + sha256(IPC of rebound public_paths)` ∘ `digest_field(semantic_digest())` | deps = `catalog::input_dependencies()` = 25 tables + deps of 4 relations + hand-added `facts, runs, producers, contexts, coverage, boundaries` (`catalog.rs:506-513`) | `stage_cache.rs::key:14-54`, `contracts:260-284` |
| `association` | same hasher over `evidence::input_dependencies()` (13 tables incl. derived `edges`), `parameters = normalization.hex()`, code = `IdHasher("association-transform-v1").digest_field(semantic_digest()).str(LCTX_ASSOCIATION_SOURCE_DIGEST)` | — | `stage_cache.rs:312-323` |
| Cache entry admission | `CURRENT.json` ≤ 64 KiB, `format == 1`, `key` equal, ≤ 32 files, lowercase/underscore names, ≤ 512 MiB total, per-file sha256, exactly one IPC batch, typed decode of exactly the declared table set (`count == batches.len()`), **then canonical batch equality against the freshly derived expected output** (`output(c) == expected_output`) | a miss or mismatch uses the computed expected result and overwrites the entry | `stage_cache.rs::read:63-97`, `decoded:204-234`, `contracts:285-345` |
| `coarse-catalog-input-v1` | `sha256`(for each raw table in `for_each_table!` order: name ‖ IPC stream of the rebound batch, compiler producer/run rows removed) ∘ `inputs.digest(analysis)` ∘ `semantic_digest()` ∘ optional embedder spec hash | **not compared**; recorded as the key of `Facts`, `BehavioralEnrichment` and `CatalogFinalization` steps | `rebuild.rs::input_digest:96-132`, `catalog:162,184-234` |
| Coarse reuse decision | `!clean && producer_matches && original.input_digest == inputs.digest(analysis) && spec_matches` where `producer_matches` = some `lctx-compiler` producer has `build_digest == semantic_digest()`; `spec_matches` = one stored spec equal to requested, or none | on reuse: stage cache runs (recompute+compare), `finish_members`, `copy_publication` copies every other raw/derived/analysis table from the source, then **full** `validate_costed` and a fresh `snapshots` publication with fresh random snapshot id | `rebuild.rs::catalog:163-206`, `copy_publication:244-350` |
| `retrieval-realization-v1` | `IdHasher(...).str(generation.key)` computed after bundling | none; outcome always `Recomputed` | `rebuild.rs::retrieval:386-399`; `crates/lctx/src/rebuild.rs:143-151` |

**PR5 review F04 rationale** (`docs/design_review/reviews/design_review_pr5-agent-journeys-rebuild_2026-09-29.md` §4 and F04; committed in `fedd4a0`): "Hash/schema/row decoding alone admitted a semantically wrong cache entry; final publication validation refused repeatedly instead of rebuilding that disposable stage. `stage_cache.rs` now compares snapshot-neutral canonical table batches against the existing pure derivation before admitting cached results … A mismatch uses the computed expected result and replaces the cache; final Delta validation remains independent of cache admission." and §4: "Cache admission intentionally rederives expected pure output, and final canonical validation reconstructs it again. This is a correctness-first cache contract, not evidence of reduced normalization/association CPU time … No performance result is inferred from a `Reused` receipt." §8 decision: "**A finite stage cache is sufficient.** Salsa persistence remains conditional on a demonstrated finer-reuse workload." §9 rejects a "Generic incremental engine … Deferred; coarse reuse plus clean equality is the baseline."

Consequence (Implemented, derived from the code above): a `Reused` normalization/association step still performs the full pure derivation (`stage_cache.rs:292,330-334`), and publication validation performs it a second time (`catalog.rs::validate:1420` → `catalog.rs::contracts:457-470`). Per compile the catalog contracts are therefore derived twice, plus two whole-input IPC hashing passes for the keys, plus encoding the expected result to compare it; on `Reused` the admitted cached value (byte-equal to the expected one) replaces it (`stage_cache.rs:297-301,339-342`). The cache saves no derivation work; `Reused` means "verified equal", not "skipped". The only real compute reuse in PR5 is the coarse **copy** of every raw, derived, behavioral, Stage E and Stage F table in `rebuild.rs::copy_publication:295-303`, admitted by `semantic_digest` + `input_digest` + spec equality, then fully revalidated. (Lead notes, not re-verified here: the PR5 pilot receipt shows the catalog stage at 58.6 s "Recomputed" and validation at 177.2 s of a 473.9 s compile.)

### 4.4 What a declared artifact DAG would need that is implicit today

| Need | Current state | Evidence |
|---|---|---|
| Stage → input tables | Declared for F5 only, by one macro that binds loading and key deps (`catalog_inputs!`, `evidence_inputs!`). Six extra names are hand-appended without being loaded (conservative). Behavioral block, Stage E and Stage F read through ad hoc `relations!`/SQL with no declared stage input set. | `catalog.rs:506-546`, `evidence.rs:51-73`, `analyze.rs` ~18 `collect(` calls |
| Relation → tables (`relations!` `deps`) | Hand-declared; **no runtime or test enforcement** that `deps` covers the SQL's table references. `sql::fetch` ignores `deps`. Paper probe of `flow_model_roots`, `catalog_all_name_roots`, `flow_model_descriptors`, `catalog_export_candidates`, `catalog_member_observations`: deps match SQL today. | `cpg-schema/src/query.rs::relations!:105-124`, `cpg-core/src/sql.rs::fetch:119-140`, `flow_model.rs:558-568`, `public.rs:177-191` |
| Absent-lookup dependencies | Handled by hashing **complete** declared scans ("Keys bind complete scans, not lookup hits"; "Membership and missing observations are dependencies too"). Correct but coarse: any row change anywhere in 25+ tables invalidates. | `stage_cache.rs:1`, `catalog.rs:472` |
| Stage → output tables (membership) | Implicit in `finish` call order; analysis tables not written are silently persisted empty (F17). A catalog-profile rule refuses non-empty unselected tables, but no rule refuses a behavioral table that a writer forgot. | `attempt.rs:1646-1651`; `crates/cpg-schema/src/rules.rs:3816-3822` |
| Ordering | Derived tables: hand-kept "dependency order"; analysis tables: planning relation is an empty typed batch until written (F4), so a mis-ordered read yields empties, not an error. | `crates/cpg-schema/src/derived.rs:1168-1197`; `attempt.rs:737-740,468-470` |
| Code identity per stage | Only two: `semantic_digest` (whole semantic source set) and association source digest. Exclusion list in `build.rs` is hand-maintained (F02 found it wrong once). | `cpg-core/build.rs:103-125` |
| Cross-stage write-back | `catalog_members.brief_status` depends on Stage F. | `attempt.rs:1568-1584` |
| Effects | Embedding (HTTP embedder + PG `Store` cache), disposable stage files, Delta appends are interleaved in one function. | `attempt.rs:706-710`, `stage_cache.rs:98-127` |
| Step dependencies | Hand-written in each `Step`, receipt-only, never used for scheduling or admission. | `rebuild.rs:184-234`, `crates/lctx/src/rebuild.rs:154` |

## 5. Identity table

All rows **Implemented** (recipe code read). "Keys reuse?" = the value is compared to admit reuse or idempotency somewhere.

| Identity | Composition | Stored / carried | Keys reuse? | Notes |
|---|---|---|---|---|
| `snapshot_id` | 16 random bytes (`crates/lctx/src/main.rs::random_id:486-490`); rebuild target is a new random id (`crates/lctx/src/rebuild.rs:124`) | every Delta row; `snapshots`; commit metadata `lctx.snapshot_id` (`delta.rs::append:171-174`) | no (visibility filter only, `snapshot.rs::register:156-172`) | Not content-addressed. Neutralized to `Id::ZERO` in stage/coarse keys (`stage_cache.rs:33`, `rebuild.rs:117`) but **included** in bundle and PG generation identities |
| `release_id` | `kind::RELEASE` over library/corpus file content digests (`cpg-extract/src/library.rs:429`; corpus `cpg-extract/src/config.rs:141-155`) | `releases`, `runs` | indirectly (via run ids) | content-addressed |
| `environment_digest` | `kind::ENVIRONMENT` over site-packages dist-info and analyzer-readable file digests (`config.rs::environment_digest:313-352`) | `contexts` | via `context_id` | content-addressed |
| `context_id` | `kind::CONTEXT` over python version, platform, search/site paths, `config_digest` (canonical pyrefly config JSON), `environment_digest`, optional `lock_digest` (`config.rs::context:368-422`, id at `:397-405`) | `contexts`, `runs` | via run ids | content-addressed |
| extractor `producer_id` | `recipe::producer(TOOL, revision, build_digest)` (`cpg-schema/src/id.rs::recipe::producer:225-231`); `revision` = pyrefly rev + patch sha + ruff line + flow provider; `build_digest` = `IdHasher("extractor-build")` over `CARGO_PKG_VERSION/EXTRACTOR_OUTPUT_VERSION` (=37, "Bumped by hand") + profile (+ committed model catalog if behavioral) (`config.rs::producer:427-455`, `:31`) | `producers`, `runs` | via run ids | **Hand-versioned**, not a source hash of `cpg-extract` |
| extractor `run_id` | `recipe::run(release, context, producer, sorted families, producer config digest)` (`id.rs::recipe::run:235-246`; `config.rs::run_id:458-466`) | `runs`, fact provenance | via `content_digest` | identity proxy for fact content, not a hash of the facts |
| `semantic_digest` | `compiler_digest_for(LCTX_SEMANTIC_SOURCE_DIGEST)` (`attempt.rs:174-176`): `kind::COMPILER` over engines + analytics libraries, `COMPILER_OUTPUT_VERSION` (112, hand-bumped), UDF version, `TEMPLATE_VERSION`, every derivation SQL, projection specs, Pass B/community/concept/neighbour relation digests, pre-registered parameters, public relations SQL, committed model catalog digest, the semantic source digest, all table contracts and all rules (`attempt.rs::compiler_digest_for:182-250`, `compiler_digest_of:139-160`). Semantic source = blake3 over `cpg-core/src`, `lctx-analytics/src`, `cpg-schema/src`, `lctx-postgres/src/cache.rs` **minus** `wire/{journeys,responses,dispatch}.rs`, `cpg-schema/src/retrieval.rs`, `cpg-core/src/{evidence,catalog_domains,stage_cache}.rs`, `bundle*`, `retrieval*`, `rebuild*` (`cpg-core/build.rs:103-125`) | compiler `producers.build_digest` and `revision` (`analyze.rs::compiler_rows:185-209`) | **yes**: stage keys (`stage_cache.rs:282,313`), coarse `producer_matches` (`rebuild.rs:163-166`), coarse input key (`rebuild.rs:127`) | Excludes the producers of the canonical `catalog_{artifacts,spans,scenarios,deployments,associations}` and `catalog_selection_domains` tables |
| association source digest | blake3 of `cpg-core/src/evidence.rs` + `catalog_domains.rs` (`build.rs:126-139`) | none | **yes** (association key, `stage_cache.rs:312-315`) | not part of any producer id |
| `compiler_digest` | `compiler_digest_for(SOURCE_DIGEST)` — same as semantic but over the full source set (`attempt.rs:171,178-180`) | every `snapshots` row (`attempt.rs:1694`, `rebuild.rs:335`); bundle/projection manifest `compiler_digest`; rebuild `Receipt.compiler` | no | Changes on serving-only edits (bundle, retrieval, wire responses, stage cache, rebuild) |
| `schema_digest` (Delta) | `IdHasher("schema")` over canonical schema text (`cpg-schema/src/table.rs::schema_digest:110-114`) | `snapshots.schema_digest` (`attempt.rs:1693`) | no (Delta schema drift is refused structurally by `delta.rs::verify:110-146`) | |
| `inputs.digest(analysis)` | `IdHasher("catalog-compile-inputs")` over profile, roots, optional `variant_config_digest(config.digest(), techniques json)` (`catalog.rs::CompileInputs::digest:41-52`; `analyze.rs::variant_config_digest:144-149`) | `catalog_compilation.input_digest` (`attempt.rs:781`); compiler run `config_digest` → compiler `run_id` (`attempt.rs:585`) | **yes**: coarse reuse equality (`rebuild.rs:177`), coarse input key | appears in three places |
| `content_digest` (snapshot) | `kind::SNAPSHOT_CONTENT` over sorted unique run ids ∘ `compiler_digest()` ∘ optional (spec hash, receipt digest) (`attempt.rs::content_digest_with:276-287`) | `snapshots.content_digest`; manifests `content_digest`/`snapshot_digest`; PG `record_snapshot` | no | Over-broad (full compiler digest); under-covers extraction code changes that skip the hand bump |
| embedding spec hash / receipt digest | `Spec::hash` (`cpg-schema/src/embedding_spec.rs:79`); `embedding-receipt/v1` over sorted (spec, input_hash, value_digest) (`cpg-schema/src/embedding.rs::receipt_digest:54-`) | `embedding_specs`, content digest, manifest `spec_hash` | **yes**: coarse `spec_matches` (`rebuild.rs:167-174`); PG embedding cache by spec + input hash (`lctx-postgres/src/cache.rs`) | |
| `normalization` / `association` stage keys | see §4.3 | `rebuild-cache/<stage>/CURRENT.json` | **yes** | current-only, one entry per stage |
| `coarse-catalog-input-v1` | see §4.3 | receipt only | **no** (recorded, never compared) | the only digest over actual fact bytes, unused for admission |
| retrieval ids | `retrieval-unit-v1`, `retrieval-content`, `retrieval-fragment-v1` (`cpg-schema/src/retrieval.rs:134,154,157`); receipt `input_digest = IdHasher("retrieval-inputs-v1").str(json(units))` + hand `VIEW_REVISION`/`RENDER_REVISION` (`cpg-core/src/retrieval.rs:265-272`) | retrieval artifacts | no | digest of rendered output, not inputs |
| bundle `generation` key | first 16 hex of sha256 of the sorted outer `MANIFEST.json` without `generation` (`bundle.rs::key_of:710-717`) | directory name `out/<key>/` | **yes**: an existing directory with the key must hold identical bytes (`bundle.rs:1085-1098`) | 64-bit truncation; includes `snapshot_id` |
| projection `definition_digest` | serialized relation/link/native-file/artifact definition (`cpg-schema/src/serving_projection.rs::definition_digest:68-`) | projection manifest | checked at verify (`bundle.rs:1143-1147`) | |
| PG `generation_digest` (= `projection_generation`) | full sha256 of the serde JSON of the projection `Manifest` (`serving_projection.rs::Manifest::generation:432-437`): capabilities, formats, library context, **snapshot_id**, snapshot/content digest, compiler digest, projection digest, model catalog digest, kernel format, entry-value effect digest, spec hash, dimensions, per-relation row-hash receipts (`receipt:573-`), artifact sha256/bytes (`Manifest:215-231`) | `lctx_serving.generations`, every projection row, advisory lock (`lctx-postgres/src/import.rs::lock:37-41`) | **yes**: import idempotency/admission (`import.rs::Source::open:60-71`, batch `content_digest` at `:434`), pinning, cursors | covers all served content; random snapshot id prevents cross-compile sharing |
| import `transport` digest | sha256 of file map (`import.rs:85-87`) | import attempts | yes (bound at `:329`) | |
| retrieval `profile_digest` | sha256 of canonical policy (`lctx-postgres/src/profiles.rs::digest:41-43`) | `lctx_serving.retrieval_profiles`, `selections` | yes (selection lookup `repository.rs:79-95`) | |
| `prepared-selection-v1` | selection digest ∘ query (`lctx-postgres/src/selection.rs:333-338`) | response/cursor | cursor binding | recomputed per request |
| `ranked-selection-cursor-v1` | prepared hash ∘ channel state ∘ profile (`selection.rs:539`) | cursor | cursor binding | |
| journey cursor | `{format, generation, policy digest, sha256(request JSON), offset}` base64 (`lctx-postgres/src/journey_cursor.rs::encode:19-29`, `position:30-52`) | client | cursor admission refuses on mismatch | |

**Overlaps and redundancy (Implemented unless marked).**
1. Two compiler identities: `semantic_digest` names the producer; `compiler_digest` goes on every snapshot row and into `content_digest`. Neither is exactly "what produced canonical rows": semantic excludes `evidence.rs`/`catalog_domains.rs` (canonical association/domain producers), full includes serving-only code.
2. Two generation keys over nested manifests: bundle `generation` (truncated, outer manifest) and `projection_generation` (full, inner manifest). Both include the random `snapshot_id`, so identical content compiled twice yields distinct generations; only `content_digest` is content-level, and PG does not key on it.
3. `inputs.digest` is stored three ways (compilation row, compiler run config, coarse key).
4. Facts are identified by run ids (identity proxies with a hand-bumped extractor version); the only fact-byte digest (`coarse-catalog-input-v1`) is receipt-only.
5. Hand-bumped versions coexist with automatic source digests: `COMPILER_OUTPUT_VERSION` (`attempt.rs:132`), `EXTRACTOR_OUTPUT_VERSION` (`config.rs:31`), `TEMPLATE_VERSION`, `VIEW_REVISION`, `RENDER_REVISION`, bundle `FORMAT`, projection `FORMAT`, wire `FORMAT`.

## 6. Serving: per MCP tool

**Labels.** `[I]` = Implemented: I read the code path end to end. `[IC]` = Interface-checked: I read
only the signature, DDL, contract or docstring, or the claim relies on third-party library
behaviour that I did not trace in full. Line numbers are at commit 35afc09. Citations use the form
`path::symbol:line`. For SQL files, the symbol is the file name and the line is `1`.

**How round trips are counted.** One round trip is one SQL statement awaited on a connection. The
code awaits every statement in turn on one `PgConnection`, so there is no pipelining. Pool lease
acquisition is not counted. SQLx may spend one extra Parse/Describe exchange the first time it
prepares a statement on a connection. That is not counted either (`[IC]`, SQLx behaviour). `L` is
the number of nested type levels expanded by the catalog type closure. `k` is the number of hits
on a page.

---

### 6.0 Serving spine (shared by every tool)

1. **Startup pin, once per server lifespan.** The lifespan calls `open_repository`, then
   `repository.pin(library, generation, profile)`, then `pinned.inputs()`, then
   `generation.load(...)`, then `serve(...)`
   (`python/lctx_mcp/src/lctx_mcp/server.py::build_server.lifespan:405-429`) [I].
   - `open_repository` checks the role config (`crates/lctx-postgres/src/serving.rs::RoleConfig::validate:59-90`).
     The session is read-only: `default_transaction_read_only=on` for Serving
     (`serving.rs::RoleConfig::options:119-127`). The pool is opened only if the migration checksums
     equal the compiled `MIGRATOR` and pgvector is 0.8.6 in `lctx_ext`
     (`serving.rs::check_connection:240-258`, `serving.rs::check_extension:260-266`). The role must
     be non-elevated and must have the expected name (`serving.rs::RoleConfig::pool:139-148`) [I].
   - `ServingStore::pin` works as follows [I] (`crates/lctx-postgres/src/repository.rs::ServingStore::pin:69-121`):
     1. It takes `generation_digest` and `profile_digest` from `lctx_serving.selections`, unless an
        explicit digest is given (`:76-94`).
     2. It joins `generations` with `retrieval_profiles` where `state='ready'` (`:95-98`).
     3. It parses the manifest and runs `Manifest::validate` (`:99-101`).
     4. It recomputes the manifest digest and checks it equals the id, and checks the library
        (`:102-104`).
     5. It validates the policy and its digest (`:105-110`).
     6. It calls `verify_locations`, which re-reads and SHA-256-checks every registered artifact
        file (`:111`, `crates/lctx-postgres/src/import.rs::verify_artifact_locations:814-870`).
   - `PinnedRepository::inputs` re-reads every artifact. It checks type and size, enforces a 512 MiB
     total budget, and re-hashes each file through `Manifest::verify_artifact`
     (`python/lctx_storage/src/lib.rs::PinnedRepository::inputs:150-216`) [I].
   - `generation.load` does the following [I] (`python/lctx_mcp/src/lctx_mcp/generation.py::load:66-112`):
     - It checks `bundle_format == BUNDLE_FORMAT` (`:74-75`).
     - It checks that the embedding-spec artifact hash equals the manifest `spec_hash` and the
       client spec (`:76-84`).
     - It builds `SemanticExecutor.from_ipc` only when `capabilities.native_value_paths` is set
       (`:85-94`).
     - It builds the lexical state from `lexical_text`, `retrieval_fragments` and
       `retrieval_subjects` (`:96-111`).
   - `serve` builds the in-process BM25 indexes once: `Lexical` over brief text and `UnitLexical`
     over unit fragments (`server.py::serve:166-174`, `retrieval.py::UnitLexical.__init__:143-159`) [I].
2. **Per request.** `SchemaTool.run` works as follows [I]:
   - It decodes arguments through the Rust wire contract (`wire_decode`). A `ValueError` becomes a
     FastMCP `ValidationError` (`python/lctx_mcp/src/lctx_mcp/wire.py::SchemaTool.run:104-116`).
   - It runs the callback under a 30 s deadline (`:118-121`).
   - It re-encodes the result through `wire_tool_result`: union tagging, contract decode and a
     per-tool byte budget (`:123-138`, `crates/cpg-schema/src/wire/mod.rs::tool_result:390-429`,
     `wire/mod.rs::tool_budget:367-380`).
   - It measures the final MCP `CallToolResult` bytes against `byte_limits` (`wire.py:146-163`).
3. **Error mapping, Rust to Python to MCP** [I]:
   - Rust `Error::Request` becomes Python `ValueError`
     (`python/lctx_storage/src/lib.rs::error:14-17`).
   - Every other `Error` becomes `StorageError` with `.kind`, as follows (`lib.rs::error:18-34`):
     - `Projection` keeps its own kind: unavailable, incomplete, corrupt, incompatible or
       resource_refused.
     - `Config` and `Schema` become incompatible.
     - `Integrity` becomes corrupt.
     - `Database` and `Admission` become unavailable.
   - In the server, `storage()` converts as follows (`server.py::storage:195-203`):
     - `StorageError` becomes `ToolError("storage {kind}: …")`.
     - `ValueError` becomes `CapabilityError`, which is a `ValidationError`, so MCP reports invalid
       params.
     - A timeout becomes `ToolError("storage unavailable: request deadline")`.
   - FastMCP 4.0.5 lets `FastMCPError` subclasses (ToolError, ValidationError) through unchanged. It
     replaces every other exception with the masked `"Error calling tool '<name>'"`, because
     `mask_error_details=True` (`.venv/.../fastmcp/server/server.py:1529-1577`, `server.py:431-437`).
     I read this in the vendored library source.
4. **No hydrated-object cache.** No hydrated rows or packets are cached, and the Rust `Hydration`
   ledger is created per request (`repository.rs::Hydration::new:216-221`). The only per-process
   state is:
   - the pinned manifest and artifact paths (`repository.rs::PinnedGeneration:24-30`);
   - the BM25 and lexical tables;
   - the native `SemanticExecutor`.

   `crates/lctx-postgres/src/cache.rs` is a compile-time embedding value cache
   (`lctx_cache.embedding_values`, `cache.rs::Store::cached:53-96`, `cache.rs::Store::admit:100-149`).
   Its only production consumer is `crates/cpg-core/src/embed.rs:189`, the bundle/embedding build,
   not serving. `selection.rs` opens with the comment "No connection or cache survives preparation"
   (`crates/lctx-postgres/src/selection.rs:1`). A grep for lru, moka, OnceCell, OnceLock and
   functools caching in the serving crates found nothing. [I]

---

### 6.1 Per-tool table

Ten tools are registered through `register(mcp, READ_ONLY)` (`python/lctx_mcp/src/lctx_mcp/wire.py::register:171-190`,
which calls `mcp.add_tool(SchemaTool)`). There is also one resource template. The Rust contracts
are named in `crates/cpg-schema/src/wire/mod.rs::tool_contract:350-365` [I].

| Tool | Entry (Python to Rust) | Relations read | Hydration route and sequential round trips | Per-request rebuilds | Native executor use | Missing, stale or corrupt behaviour |
|---|---|---|---|---|---|---|
| `search_capabilities` | `server.py::search_capabilities:441-455` → `server.py::search:207-287` → `PinnedRepository.search_scope` (`lctx_storage/src/lib.rs:427-433`) → `repository.rs::ServingStore::search_scope:146-175`. Optional `vector_ranks` (`lctx_storage lib.rs:435-452` → `crates/lctx-postgres/src/retrieval.rs::ServingStore::vector_ranks:44-91`). `hit_records` (`hydration.rs::ServingStore::hit_records:526-566`) | `briefs`, `symbol_map`, `vectors` (brief chunk vectors, per-generation partition), `briefs` again | 1. `search_scope`: `SELECT brief_id FROM briefs`, then `SELECT … FROM symbol_map` (2 RTs, `repository.rs:152-161`).<br>2. Hybrid only: `vector_ranks` calls `search_scope(generation,"")` again (2 RTs, `retrieval.rs:63`), then an exact `GROUP BY brief_id` cosine scan (1 RT, `retrieval.rs::exact:106-132`).<br>3. `hit_records` → `Hydration::fetch("briefs","brief_id")` (1 RT).<br>Total: **3** (lexical-only) or **6** (hybrid), over 2 or 3 leases [I] | Brief eligibility is fetched twice on the hybrid path (`retrieval.rs:63`). RRF fusion runs in Python (`retrieval.py::fuse_legs:100-137`). BM25 index built at startup, not per request [I] | None (BM25 over the `lexical_text` artifact held in memory) [I] | Briefs capability off: typed `CapabilityUnavailable` (`server.py::unavailable:83-90`, `:451-452`). Library mismatch: `CapabilityError` (`server.py::search:214-215`). No vectors, no embedder or embedder failure: lexical-only with explicit `degraded_reason` (`server.py:236-248`). Spec mismatch: `Error::Request` → `CapabilityError` (`retrieval.rs:51-55`). A missing selected hit is corrupt (`hydration.rs:559`) → `ToolError("storage corrupt…")`. A Python budget or corrupt-rank `ValueError` (`retrieval.py:81-86,94-95,109-110`) is **masked** as a generic tool error [I] |
| `get_capability` | `server.py::get_capability:459-477` → `server.py::hydrate:331-336` → `lctx_storage lib.rs::get_capability:365-381` → `hydration.rs::ServingStore::get_capability:276-525` | `briefs`, `brief_members`, `assertions`, `supports`, `support_findings`, `support_witnesses`, `support_members`, `support_attributes`, `support_attribute_incidences`, `evidence` | 10 × `Hydration::fetch` with `key=ANY($2)` (`hydration.rs:286-395`), one lease, strictly sequential. Several are independent of each other (witnesses, members, incidences and evidence all key on `finding_ids` or support `evidence_id`s) but are still serialised. **10 RTs** [I] | `Hydration` ledger. `bundle::files()` inventory rebuilt on every fetch (`repository.rs:282-285`). `validate_batch` runs on every 200-row decode (`crates/lctx-postgres/src/projection.rs::decode_rows:119-131`) [I] | None [I] | Snapshot not pinned: `CapabilityError` (`server.py:469-470`; Rust `repository.rs::check_snapshot:48-55`). Unknown id: `Request("no capability in this generation")` (`hydration.rs:295-297`) → `CapabilityError`. Briefs capability off: `CapabilityUnavailable`. Dangling cited finding, attribute or evidence: `corrupt` (`hydration.rs:429,465,493`). Oversize: refused (`repository.rs:338-340`, `hydration.rs:400`) [I] |
| resource `capability://{snapshot_id}/{capability_id}` | `server.py::capability:731-747` (not a SchemaTool) | same as `get_capability` | same, **10 RTs** [I] | same, plus Markdown render (`server.py::markdown:339-392`) | None | Snapshot mismatch: `CapabilityError`, which the docstring says becomes -32602 (`server.py:12-14,742-743`) [IC for the JSON-RPC code]. Briefs off: returns `CapabilityUnavailable` JSON as the resource text (`:744-745`). Over 8 MiB: `ToolError` (`:390-391`) [I] |
| `get_operation` (default packet view) | `server.py::get_operation:487-506` → `lctx_storage lib.rs::get_operation:225-255` → `crates/lctx-postgres/src/packet.rs::ServingStore::operation_packet:34-252` | `catalog_members`, `singletons` (behavioral only), `catalog_bindings`, `catalog_constructors`, `catalog_signatures` (×2), `catalog_parameters`, `catalog_surfaces`, `catalog_configurations`, `catalog_field_links`, `catalog_type_observations`, `catalog_types`/`catalog_type_args` (per level), `operations`, `public_paths`, `operation_facet_status`, `catalog_associations`. Behavioral only: `behaviors`, `behavior_discharges` | 1. `packet_resolve.sql` + `catalog_members` fetch (2).<br>2. `packet_singleton.sql` (0 or 1, `hydration.rs::singleton_class:56-83`).<br>3. `catalog_record` = `catalog_invocation` (5: `hydration.rs:100-157`) + surfaces, configurations, field_links, observations (4: `:198-234`) + type closure (**2 per level**: `:240-267`).<br>4. `operations`, `public_paths`, `operation_facet_status` (3: `packet.rs:93-119`).<br>5. `packet_demonstrations.sql` (1: `:190-196`).<br>6. Behavioral only: relationships preview = `packet_behaviors.sql`, `packet_behavior_count.sql`, `behaviors`, `behavior_discharges` (4: `packet.rs:211-249` → `section_page:333-370`).<br>Total **15+2L** (catalog profile) up to **20+2L** (behavioral, singleton) [I] | Same inventory and validation rebuilds as above. The packet is re-serialised for the budget check after every optional record (`packet.rs:189,207,229`) [I] | None [I] | Snapshot mismatch: `Request` → `CapabilityError` (`packet.rs:39`). Not found: `Request("public member not found…")` (`:56-58`). Ambiguous: typed `AmbiguousOperation` result (`:49-55`). More than 100 choices: refused (`:270-272`). Behavioral off: sections `not_requested` with a reason (`:152-180`). No operation node: `unavailable` (`:166-167`). Core over budget: refused with guidance (`evidence.rs::check_operation_packet:261-285`). Demos or relationships over budget: `omitted_budget` state (`packet.rs:207-210,236,245`) [I] |
| `get_operation` (section view) | same entry, `OperationView::Section` → `packet.rs::section_page:284-493` | Evidence: `catalog_associations`, `catalog_scenarios`. Behavior/Relationships: `behaviors`, `behavior_discharges`. Facets: `operation_facets`. Fields: `ambient_reads`, `place_claims` | 2 to 3 (resolve and singleton) plus the section's own queries:<br>- Evidence: 2 (`evidence.rs::association_page:294-301`).<br>- Behavior or Relationships: 4 (`packet.rs:335-359`).<br>- Facets: 2 (`packet.rs:372-386`: `packet_facet_count.sql`, `packet_facets.sql`).<br>- Fields: 4 (`packet_field_keys.sql`, `packet_field_ordinals.sql`, `ambient_reads`, `place_claims`: `packet.rs:401-450`).<br>Total **4 to 7** [I] | Population recomputed on every page, using `LIMIT 20 OFFSET n` (`packet_behaviors.sql`, `packet_facets.sql`). Fields fetches every key, then skips (`packet.rs:401-417`) [I] | None | Journey cursor bound to generation, policy and target (see §b6). Mismatch: `Request` → `CapabilityError`. Offset past the population: `Request` (`packet.rs:459-461`). Empty: `empty_under_coverage` (Evidence) or `unavailable` with a reason, not a silent empty list (`:462-476`). Indivisible record over budget: refused (`:489-491`) [I] |
| `get_evidence` | `server.py::get_evidence:510-536` → `lctx_storage lib.rs::get_evidence:329-364`, which dispatches on `EvidenceTarget`: `crates/lctx-postgres/src/evidence.rs::ServingStore::get_evidence:85-107` or `evidence.rs::ServingStore::get_retrieval_unit:355-483` | Original: `catalog_scenarios` or `catalog_deployments`, `catalog_spans`⋈`catalog_artifacts`, `catalog_artifacts.body` (substring). Unit: `retrieval_units`, `catalog_evidence`⋈`catalog_artifacts`⋈`catalog_spans` (`unit_original_span.sql`), then the original read | Original span: 2 (`evidence.rs::read:173-174,217-218`). Scenario or deployment: 3 (`:140-145` + 2). Retrieval unit: `unit_detail.sql` (1). A fragment page needs no more. An anchor page adds `unit_original_span.sql` (1 per catalog anchor visited) plus 2 to 3. Total **1 to about 5** [I] | Fragments recomputed from the unit JSON (`evidence.rs:408`) [I] | None | Snapshot mismatch: `Request`. Reference not in generation: `Request` (`evidence.rs:146-148,174`). Span closure missing for a catalog anchor: `corrupt` (`:436-439`). Metadata over half the budget: typed `metadata_omitted=true` fallback to the primary span (`:149-151`). Bad cursor: `Request` (`:62-72`, `:385-392`). Page over budget: refused (`:248-255`, `:474-480`) [I] |
| `inspect_value_paths` | `server.py::inspect_value_paths:540-581` → `lctx_storage lib.rs::resolve:217-223` → `repository.rs::resolve_on:177-203`. Then `value_paths.py::inspect:53-83` → `SemanticExecutor.inspect_value_page` (`python/lctx_semantics/src/lib.rs::SemanticExecutor::inspect_value_page:1521-1640`) | `public_paths`, `singletons`, `operations` (resolve UNION). The executor's in-memory tables | **1 RT** (the resolve UNION query `repository.rs:187-188`), then native CPU [I] | None. Executor built once at startup [I] | **Yes. The only tool that uses it.** All 29 `NATIVE_FILES` are loaded at startup (`generation.py:85-94`, `python/lctx_semantics/src/ipc_input.rs::decode:187-239`) [I] | `native_value_paths` off: typed `CapabilityUnavailable` before any query (`server.py:557-562`). Snapshot mismatch: `CapabilityError` (`:555-556`) and a native check (`lib.rs:1536-1540`). Ambiguous or unknown operation: `Request` → `CapabilityError` (`repository.rs:191-198`). Native `ValueError` → `OperationError` → `CapabilityError` (`value_paths.py:82-83`, `server.py:580-581`). Cursor bound to format, generation and query digest (`wire/mod.rs::value_offset:544-564`) [I] |
| `find_operations` | `server.py::find_operations:585-605` → `lctx_storage lib.rs::prepare_selection:383-404` → `selection.rs::ServingStore::prepare_selection:201-344`, then the sync `PreparedSelectionHandle.page` (`lctx_storage lib.rs:496-504` → `selection.rs::PreparedSelection::page:64-158`) | Always a full-generation scan of `catalog_members` and `catalog_selection_domains` (`selection_domains.sql`). Plus each requirement dependency, as a **full-generation** table load: `catalog_bindings`, `catalog_surfaces`, `catalog_signatures`, `catalog_parameters`, `catalog_type_observations`, `catalog_types`, `catalog_type_args`, `catalog_configurations`, `catalog_field_links`, `operation_facets`, `operation_facet_status`, `catalog_associations`, `catalog_scenarios`, `catalog_deployments`, `catalog_spans`, and `catalog_artifacts` alignment only (`artifact_alignment.sql`) | 2 + \|deps\| (+1 when artifacts are needed) RTs on one lease (`selection.rs:219-321`). \|deps\| ≤ 15 (`crates/cpg-schema/src/wire/requirements.rs::Requirement::dependencies:366-428`). Then CPU. An empty selection costs **2 RTs**. Worst case **18** [I] | **`PreparedCatalog::new` and `classify` rebuilt on every request, and on every page** (`selection.rs:325-327`). All members × requirements are re-classified (`crates/cpg-schema/src/selection/catalog.rs::PreparedCatalog::classify:351-395`) [I] | None | A bad or stale cursor raises `Error::Request` inside the **synchronous** `page` call. It becomes a Python `ValueError` in the worker thread (`server.py:603-605`), which is outside `storage()`, so it is **masked** by FastMCP as `"Error calling tool 'find_operations'"`, not a typed refusal. Input budgets: refused (`selection.rs::charge_input:186-199`, `catalog.rs:201-214`). Domain closure broken: `WireError` → corrupt (`catalog.rs:238-247`) [I] |
| `search_operations` | `server.py::search_operations:609-622` → `server.py::search_members:291-328`. Calls: `prepare_selection` (as above), `PreparedSelectionHandle.scope`, Python `UnitLexical.winners`, optional `.vectors` (`selection.rs::ServingStore::selection_vectors:360-424`), `.finish` (`selection.rs::ServingStore::finish_selection_search:425-543`) | As `find_operations`, plus `retrieval_vectors`⋈`retrieval_fragments`⋈`retrieval_subjects` (`unit_ranks.sql`) and `retrieval_subjects`/`retrieval_fragments` (`winner_membership.sql`) | prepare (2+\|deps\|, +1) + `unit_ranks.sql` (0 or 1, exact brute-force cosine over all eligible members' fragments) + `winner_membership.sql` (1). Total **3+\|deps\| to 5+\|deps\|** over 3 leases [I] | PreparedCatalog rebuilt. Lexical winners recomputed in Python. Fusion recomputed in Python (`retrieval.py::fuse_families:247-282`) and re-verified against the Rust oracle (`selection.rs:440-499`). The whole ranking is recomputed for every page [I] | None | Spec mismatch: `Request` (`selection.rs:370-374`). Embedder failure: lexical-only, but the reason lives only in `channel_state`, which feeds the cursor hash. The response's `retrieval.fallback` stays `null` (`selection.rs:540`, `wire/responses.rs::RetrievalMetadata:503-517`). Tampered winners: `corrupt` (`selection.rs:453,462,486,495,528`). Bad cursor: `Request` → `CapabilityError`, because `finish` is async and inside `storage()`. Python-side budget `ValueError` (`retrieval.py:199,227-243,249-250`) is masked [I] |
| `browse_library` | `server.py::browse_library:626-649` → `lctx_storage lib.rs::browse_library:297-310` → `crates/lctx-postgres/src/journeys.rs::ServingStore::browse_library:12-241` | Full `catalog_members`. `catalog_selection_domains` projected to module and class_owner (`browse_ownership.sql`). `operation_facets` (facet view only) | 2 or 3 RTs (`journeys.rs:25-30,151-159`) [I] | The whole outline or facet count is rebuilt on every page. `detail::jsonb` is parsed per member on the server side (`browse_ownership.sql`) [I] | None | Library mismatch: `Request` (`journeys.rs:17-19`). Unknown scope: typed `state=unavailable` with a reason (`:67-76`). Behavioral facet without the producer: `not_requested` (`:137-143`). Member without an ownership domain: `corrupt` (`:50-53`). Whole page over 32/256 KiB: **refused** (no item popping) (`:239`). Cursor bound to generation, policy and target, which includes `limit` (`:20-22`) [I] |
| `compare_operations` | `server.py::compare_operations:653-669` → `lctx_storage lib.rs::compare_operations:311-328` → `journeys.rs::ServingStore::compare_operations:242-323` | As `find_operations` (prepare), plus, per candidate, `catalog_members`, `singletons`, `catalog_bindings`, `catalog_constructors`, `catalog_signatures` ×2, `catalog_parameters` | prepare (2+\|deps\|, +1). Then per candidate (1 to 5, `wire/journeys.rs::ComparisonCandidates:263-284`): `packet_resolve.sql` + `catalog_members` (2), optional `packet_singleton.sql` (0 or 1), and `catalog_invocation` (5). That is 7 or 8 per resolved candidate and 2 per missing or ambiguous one, in a **per-candidate loop** (`journeys.rs:254-313`). Worst case about **58 RTs** [I] | PreparedCatalog rebuilt for the full catalog even though at most 5 members are compared [I] | None | Not found: typed `NotFound` entry. Ambiguous: typed `Ambiguous` (`journeys.rs:262-272,308-311`). A resolved member missing from the classification: `corrupt` (`:279-284`). Over budget: **refused** (`:321`) [I] |
| `search_evidence` | `server.py::search_evidence:673-729`. Python `UnitLexical.unit_winners` (`retrieval.py:161-187`) and optional embedding, then `lctx_storage lib.rs::search_evidence:257-296` → `crates/lctx-postgres/src/evidence_search.rs::ServingStore::search_evidence:17-244` | `retrieval_units`, `catalog_scenarios`, `retrieval_subjects` (`evidence_eligible.sql`), `retrieval_fragments` (`evidence_winner_membership.sql`), `retrieval_vectors`⋈`retrieval_fragments` (`evidence_unit_ranks.sql`), `retrieval_units` (`evidence_search_unit.sql`), `catalog_scenarios` (`evidence_search_scenario.sql`) | Lease 1: eligible (1) + membership (1) + unit ranks (0 or 1) (`evidence_search.rs:67-138`). Lease 2: **per hit** `evidence_search_unit.sql` (1), plus `evidence_search_scenario.sql` (1) for scenario units (`:178-203`). Total **2 to 3 + k to 2k**, with k up to `limit` (≤100) until the byte budget stops it. This is an **N+1** row-by-row loop [I] | Whole ranking recomputed per page (`:142-147`). `evidence_eligible.sql` joins through `encode(scenario_id,'hex') = detail::jsonb->>'source_key'`, which parses JSON per unit and cannot use an index [I] | None (Python lexical tables only) | Library mismatch: `Request`. Families 0 or more than 4, or input budgets: refused (`:28-34`). A winner not in the generation: `corrupt` (`:106-108`). Unit detail over 16 MiB: refused (`:191`, `evidence_search_unit.sql`). Lexical fallback reported as `retrieval.fallback = channel_state` (`:167-171`). A hit over budget is popped and the cursor continues. An indivisible hit is refused (`:224-240`). Cursor bound to query, families, intent, subject, limit, expanded, channel (vector digest) and fusion revision (`:145`) [I] |

---

### 6.2 Notes per subsystem

#### 6.2.b1. Generation pinning, admission and staleness

- **One pin per server process.** `build_server` pins once in the lifespan. The docstring says:
  "pin selection once for the entire server lifespan" (`server.py::build_server:403`) [I]. Every
  later SQL statement carries `generation_digest=$1` from `PinnedGeneration.id`. Examples:
  `repository.rs::Hydration::fetch_projection:312-315`, every `queries/*.sql` file at line 1, and
  `evidence.rs::read:134-136,173,217` [I]. The CLI documents that re-selecting a generation does
  not move running servers: "Existing servers retain their pin"
  (`crates/lctx/src/serving.rs::Command::Select:44`) [I].
- **The database admits only ready generations to the serving role.** Row-level security
  `ready_read … USING (lctx_serving.is_ready(generation_digest))` is declared on all 51 projection
  tables in `202609270004_projection.sql`. It is also on each catalog, evidence and retrieval table
  added later, for example
  `crates/lctx-postgres/migrations/202609280009_catalog.sql::catalog_members:26`. For
  `generations` itself it is `USING(state='ready')` (`202609270003_serving.sql::generations:25-27`).
  `is_ready` is defined at `202609270003_serving.sql:31-34` [I from DDL].
- **Ready is terminal and immutable.** Three mechanisms enforce this [I from DDL]:
  - The `guard_generation` trigger allows only loading→validating/failed and
    validating→ready/failed, and rejects DELETE (`202609270003_serving.sql:50-60`).
  - The `require_loading` trigger on every projection table rejects writes unless the state is
    `loading` (`:38-47`). It is attached on each table (51/51 in `0004`, and all tables in
    `0009`–`0012`).
  - `cleanup_generation` refuses ready or selected generations
    (`202609280012_selection_retrieval.sql:173-178`).
- **Staleness is therefore limited to:**
  1. **A client holding an old snapshot id or cursor.** This is refused with
     `Request("snapshot is not the pinned one; search again")` (`repository.rs::check_snapshot:48-55`).
     Cursors are refused with "cursor belongs to another generation…"
     (`journey_cursor.rs::position:36-50`, `selection.rs::page:86-96`, `evidence.rs::position:62-72`,
     `wire/mod.rs::value_offset:558-563`) [I].
  2. **A newer generation selected while the server runs.** The server keeps serving its pin
     without notice, by design [I].
- **Readiness is not re-asserted per request.** No per-request code checks the state. If a row
  were ever hidden by RLS through out-of-band tampering, the SQL would return empty sets. That
  would surface as `Request("no capability…")`, typed `unavailable` states, or silently empty lists
  (for example, `browse_library` `members:0`). It would not appear as a distinct "generation not
  ready" refusal [I for the code paths; the tampering scenario is hypothetical].
- **Profile.** Only one policy is admitted:
  - `Policy::exact()` with format 3, route exact and RRF k=60 (`crates/lctx-postgres/src/profiles.rs::Policy::exact:22-31`).
  - `validate` rejects anything else (`:32-37`).
  - The database enforces `policy->>'route'='exact' AND policy->>'format'='3'`
    (`202609280012_selection_retrieval.sql:228-229`).
  - `select_generation` requires a ready generation, a matching library and the exact profile
    (`:212-221`).
  - `pin` re-validates the policy and its digest (`repository.rs:105-110`).
  - `selection_vectors` re-checks the route (`selection.rs:375-380`) [I].
- **Contract and format identity at load** [I]:
  - `Manifest::validate` checks all of the following (`crates/cpg-schema/src/serving_projection.rs::Manifest::validate:314-421`):
    - FORMAT 6 and BUNDLE_FORMAT 16;
    - `projection_digest == definition_digest()`, a digest over every relation schema, key,
      codebook, vocabulary, FK link, `NATIVE_FILES`, artifact inventory and `KERNEL_FORMAT`
      (`serving_projection.rs::definition_digest:68-123`);
    - the native catalog digest and kernel format when the native capability is on;
    - the relation schema digests;
    - that the artifact inventory equals `artifacts_for(capabilities)`.
  - The database accepts only `(format, bundle_format) = (6, 16)`
    (`202609280012_selection_retrieval.sql:8`).
  - At serving start, the migration checksums must equal the binary's
    (`serving.rs::check_connection:240-256`).

#### 6.2.b2. Failure taxonomy per tool (summary)

| Condition | Where detected | What the client sees |
|---|---|---|
| No selected or ready generation, invalid manifest, profile mismatch | `repository.rs::pin:79-110` | **The server does not start**: the lifespan raises (`server.py:410`) [I]. FastMCP lifespan failure semantics are [IC] |
| Artifact file missing or hash mismatch | `import.rs::verify_artifact_locations:834-863` (missing → Unavailable, damaged → Corrupt); `lctx_storage lib.rs::inputs:159-201` | Server does not start [I] |
| Bundle format, embedding spec or native IPC schema drift | `generation.py::load:74-84`; `ipc_input.rs::decode:187-239`; kernel format check in `lctx_semantics/src/lib.rs::ConditionGraph::new:256-260` | Server does not start (`GenerationError` or `ValueError`) [I] |
| Migration or pgvector drift | `serving.rs::check_connection:250-257` → `Error::Schema`/`Config` | Server does not start [I] |
| Capability not compiled (catalog profile) | Manifest `capabilities` | Typed `CapabilityUnavailable` (briefs, native value paths) or section `not_requested` (behavior, facets) [I] |
| Snapshot, library or cursor mismatch | `check_snapshot`, the library checks, the cursor checks | `CapabilityError` (MCP invalid params). **Exception:** a `find_operations` cursor error is masked (see the table) [I] |
| Dangling in-generation reference (cited finding, evidence, attribute, ownership domain, span closure, hit) | `hydration.rs:429,465,493,559`; `journeys.rs:53`; `evidence.rs:436-439` | `ToolError("storage corrupt: …")` [I] |
| Budget exceeded (rows 200k, hydration 64 MiB, response 8 MiB, packet 32/256 KiB, CPU slots) | `repository.rs:338-352,406-423`; `evidence.rs::check_operation_packet:261-285`; `serving.rs::run_cpu:290-294`; `server.py::NativeWorkers.run:130-151` | `ToolError("storage resource_refused: …")` or `ToolError("resource_refused: …")`. Paged tools pop items instead. Python-side `ValueError("resource_refused: …")` raised outside `Contract.model_validate_json` is **masked** [I] |
| PostgreSQL down or timeout | `lib.rs::From<sqlx::Error>:57-72` | `ToolError("storage unavailable: PostgreSQL … (SQLSTATE …)")`. After a cancelled lease cannot drain, the **whole pool is closed** (`serving.rs::QueryLease::drop:228-234`), and every later DB tool call fails until the server restarts [I] |

#### 6.2.b3. Hydration (`repository.rs::Hydration`, `hydration.rs`)

- **The fetch primitive is set-based per key list.** It issues
  `SELECT {cols} FROM lctx_serving."{name}" WHERE generation_digest=$1 AND "{key}"=ANY($2) ORDER BY row_ordinal LIMIT 200001`
  (`repository.rs::fetch_projection:290-314`) [I].
  - The relation and key names are checked against `bundle::files` (`:282-288`).
  - Data values are always bound.
  - Rows are streamed and decoded in 200-row chunks through `projection::decode_rows`, which
    re-validates the declared schema, nullability, codebooks and vocabularies on read
    (`projection.rs::decode_rows:119-131` → `serving_projection.rs::validate_batch:499-565`).
  - The byte ledger is 256 + 192/column + 3× raw bytes per row, with a 64 MiB cap (`:327-340`).
  - The row cap is 200 000 (`:350-352`).
  - The "invocation projection" variant nulls `docstring` and `documentation` at SQL read time
    (`:291-311`).
- **`catalog_record` round trips (checking the reported "~10").** The claim holds as a lower
  bound. `catalog_record` = `catalog_invocation` (bindings, constructors, signatures by callable,
  signatures by id, parameters: 5, `hydration.rs:100-157`) + surfaces, configurations,
  field_links, observations (4, `:198-234`) + the type closure (2 per level: `catalog_types` then
  `catalog_type_args`, BFS with a `visited` set, `:237-267`). That is **9 + 2L sequential RTs**
  with no join, LATERAL or recursive CTE. Inside `get_operation` it runs after 2 or 3 resolution
  RTs and is followed by 4 to 8 more (see the table). `get_capability` is exactly **10** sequential
  fetches (`hydration.rs:286-395`). [I]
- **Serialisation of independent reads.** In `get_capability`, `support_witnesses`,
  `support_members`, `support_attribute_incidences` and `evidence` depend only on `finding_ids` or
  support rows that are already available. They are still awaited one after another on a single
  connection (`hydration.rs:338-395`) [I].
- **Index coverage for hydration keys** [IC, DDL read only]:
  - Covered by a unique constraint or index: `catalog_bindings(member_id)` (unique
    `generation_digest, member_id, binding_id`, `202609280009_catalog.sql:43`),
    `catalog_constructors(class_node_id)` (`:198`), `catalog_signatures(callable_node_id)`
    (`:279` index), `catalog_parameters(signature_id)` (`:99`), `catalog_types(term_id)` (`:142`),
    `catalog_type_args(parent_term_id)` (`:163`), `catalog_type_observations(subject_node_id)`
    (`:282`), `catalog_configurations(class_node_id)` and `catalog_field_links(class_node_id)`
    (`202609280010_specificity.sql:160-161`).
  - **Not covered:** `brief_members(brief_id)` (PK only, `202609270004_projection.sql:213-222`),
    `public_paths(node_id)` (unique on `access_path` only, `:244-254`), and
    `operation_facets(node_id)` (the `facet_lookup` index leads with `facet`,
    `202609280005_publication.sql:162`).

#### 6.2.b4. Packets (`packet.rs`, `packet_*.sql`)

- `operation_packet` assembles the mandatory invocation closure and checks the core against the
  budget before it admits any optional record (`packet.rs:181-189`) [I]. It then adds, in order:
  - up to 2 demonstrations (`packet_demonstrations.sql` `LIMIT 2`);
  - a relationships preview of up to 5 records from a 20-record section page (`packet.rs:211-249`).

  After each addition it re-checks the budget and degrades to `omitted_budget`.
- Section pages use `journey_cursor` (§b6) with the target
  `{member, section, expanded}` (`packet.rs:298-299`). Populations are paged with
  `LIMIT 20 OFFSET $n` in SQL (`packet_behaviors.sql`, `packet_facets.sql`) or with an in-memory
  skip over every key (`packet_field_keys.sql` `LIMIT 200001`, `packet.rs:401-417`) [I].
- Section states are explicit, so a missing section is never an empty success
  (`wire/journeys.rs::SectionState`; `packet.rs:152-180,311-320,462-476`) [I]:
  `not_requested`, `unavailable`, `empty_under_coverage`, `omitted_budget`, `available`.

#### 6.2.b5. Selection (`lctx-postgres/src/selection.rs`, `cpg-schema/src/selection.rs`, `selection/catalog.rs`)

- **Classifier inputs.** `Inputs` has 18 typed vectors (`selection/catalog.rs::Inputs:87-107`). The
  loaders fill them as follows [I]:
  - `catalog_members` is always loaded (`selection.rs:219`).
  - Domains come from `selection_domains.sql`. It projects only the requested domain kinds from
    each member's canonical JSON detail and keeps the canonical `domain_id` (`selection.rs:220-264`;
    `selection/catalog.rs::DomainInput:42-50`). The projected JSON cannot be re-verified against
    `domain_id`, which the comment acknowledges (`queries/selection_domains.sql:1-2`).
  - Other relations load only if some requirement needs them (`Requirement::dependencies`,
    `requirements.rs:366-428`).
  - Every loaded relation is a **full-generation load** (`key=None`, `selection.rs::typed:160-185`).
  - `catalog_artifacts` contributes only `artifact_id, release_id, alignment`
    (`artifact_alignment.sql`).
- **Per-request construction (confirmed).** Every `find_operations`, `search_operations` and
  `compare_operations` call does the following [I]:
  1. `prepare_selection` acquires a lease and reloads all of the above.
  2. It releases the lease.
  3. It runs `PreparedCatalog::new(&input)?.classify(&selection)` on a CPU slot
     (`selection.rs:322-327`).

  `PreparedCatalog::new` builds about 18 BTreeMap indexes and re-parses the domain, scenario and
  deployment JSON (`selection/catalog.rs::PreparedCatalog::new:180-331`). `classify` walks every
  member × requirement under a work budget of 2 000 000 and an evidence budget of 64 MiB
  (`:332-398`). The Python `PreparedSelectionHandle` is request-local, and nothing reuses it across
  requests or pages (`lctx_storage lib.rs:477-483`; `server.py:299-303,598-602`).
- **Closure checks** [I]:
  - Member set = domain set (`selection/catalog.rs:238-247`).
  - Every required domain is loaded (`:341-346`).
  - No duplicate binding or signature identities (`:248-264`).
- **Cursor.** `{generation, request, group, ordering=POLICY_REVISION, offset}` (`selection.rs::Cursor:35-43`) [I].
  - For find, `request` = `IdHasher("prepared-selection-v1")(selection_digest, query)`
    (`:335-339`).
  - For ranked search, `request` = `IdHasher("ranked-selection-cursor-v1")(prepared.hash, channel_state, profile)`
    (`:539`).
  - Each group (supported, unresolved, conflicting) has its own cursor. The contradicted group
    cannot be paged (`:91`).
- **Ranked search integrity** [I]:
  - `finish_selection_search` re-derives the per-channel ranks and the family RRF with the Rust
    oracle and requires them to equal the Python fusion (`selection.rs:440-499`).
  - It checks the winning tuples against `retrieval_subjects` and `retrieval_fragments` in SQL
    (`winner_membership.sql`, `selection.rs:515-528`).

#### 6.2.b6. Journeys: browse, compare, evidence search (`journeys.rs`, `evidence_search.rs`, `journey_cursor.rs`, `wire/journeys.rs`)

- **Journey cursor** (`journey_cursor.rs::Cursor:9-15`, `::encode:19-29`, `::position:30-52`) [I]:
  - Contents: `{format=wire::FORMAT(4), generation hex, policy digest, request=sha256(target JSON), offset}`.
  - Encoding: base64 JSON with no MAC. Integrity rests only on equality checks plus population
    bounds checks (`journeys.rs:210-212`, `evidence_search.rs:149-151`, `packet.rs:459-461`).
  - Targets:
    - browse: `{scope, view, expanded, limit}` (`journeys.rs:20`);
    - evidence search: `{query, families, intent, subject, limit, expanded, channel, fusion}`
      (`evidence_search.rs:145`);
    - operation sections: `{member, section, expanded}` (`packet.rs:298`).
  - Evidence-search cursors therefore bind the **query vector digest and spec hash**, through
    `channel_state = "vector:"+sha256(vector)+":"+spec` (`server.py:704-712`). The embedding
    channel must be the same on every page, or the next page is refused.
- **Browse.** Every page reloads the whole `catalog_members` relation and `browse_ownership.sql`,
  rebuilds the outline or facet counts, then skips or takes (`journeys.rs:25-217`). There is no
  pagination in SQL [I].
- **Compare.** The sequence is [I]:
  1. `prepare_selection` over the full catalog.
  2. For each requested name (≤5), sequentially: resolve (2 RTs), singleton (0 or 1), then
     `catalog_invocation` (5 RTs) (`journeys.rs:250-313`).
  3. It returns the signatures and a `GetOperationRequest` to hydrate later. It never returns a
     full packet.
- **Evidence search.** The flow is [I]:
  1. **Python** computes lexical unit winners over the whole in-memory fragment index for the
     requested families, without eligibility (`retrieval.py::UnitLexical.unit_winners:161-187`).
  2. **Rust** filters them by the eligibility query (`evidence_eligible.sql`: family, intent and
     subject filters), validates membership, and adds exact vector ranks.
  3. It releases the lease before fusion (`evidence_search.rs:139-143`).
  4. It then hydrates **each hit separately** (`:178-241`).

#### 6.2.b7. Evidence (`evidence.rs`)

- Metadata admission comes before body reads [I]:
  - The scenario or deployment detail is returned only if
    `octet_length(detail) <= budget/2`. Otherwise it falls back to the primary span with
    `metadata_omitted` (`evidence.rs::read:133-168`).
  - The body is read with `substring(body FROM … FOR count)`, where count is sized for 6× JSON
    expansion (`:211-218`).
  - UTF-8 boundaries are respected. Undecodable bytes become base64 (`:219-233`).
- **Cursors** [I]:
  - Original evidence: `{format, generation, target(EvidenceRef JSON), position, offset, expanded}`
    (`evidence.rs::Cursor:18-27`, `::position:47-74`).
  - Retrieval unit: `UnitCursor{generation, unit, position, inner, expanded}`, which has no
    `format` field (`:345-353`, checked at `:385-392`).
- Association paging (Evidence section) runs one count query and one `ORDER BY … LIMIT OFFSET`
  query over `catalog_associations` where `member_id=$2 OR release_id IS NOT NULL`
  (`evidence.rs::association_page:294-301`). The contextual count uses a sub-select that parses
  `detail::jsonb` [I].

#### 6.2.b8. Retrieval (`retrieval.rs`, `unit_ranks.sql`, `evidence_unit_ranks.sql`, Python `retrieval.py`)

- All vector ranking is **exact brute-force cosine** in PostgreSQL, with no ANN route [I]:
  - briefs: `1-min(vector <=> $2) GROUP BY brief_id` (`retrieval.rs::exact:113`);
  - members: `DISTINCT ON (member_id, family)` over every eligible member's fragments
    (`unit_ranks.sql:1-16`);
  - evidence units: `DISTINCT ON (unit_id)` (`evidence_unit_ranks.sql`).

  The HNSW builders were dropped (`202609280012_selection_retrieval.sql:222-227`). Query vectors
  must be 1024-dimensional, finite and normalised (`retrieval.rs::validate_query:93-105`), and the
  spec hash must equal the manifest's (`retrieval.rs:51-55`, `selection.rs:370-374`,
  `evidence_search.rs:41-48`).
- Lexical ranking is **in Python** over artifacts loaded at startup: BM25 via `bm25s`, one index
  per family (`retrieval.py::Lexical:34-60`, `UnitLexical:140-222`). Rust validates the
  Python-supplied lexical winners against SQL membership, and for ranked member search also
  against the Rust fusion oracle (§b5) [I].
- Brief search calls `search_scope` twice on the hybrid path (§a) [I].

#### 6.2.b9. Native executor (`lctx_semantics`, `NATIVE_FILES`)

- `NATIVE_FILES` lists 29 relations (`serving_projection.rs::NATIVE_FILES:12-42`).
  `artifacts_for` includes them **only** when `capabilities.native_value_paths` is set. Seven
  lexical/retrieval artifacts are always added: `lexical_text`, `embedding_spec`,
  `retrieval_units`, `retrieval_subjects`, `retrieval_fragments`, `retrieval_vectors`,
  `retrieval_receipt` (`serving_projection.rs::artifacts_for:51-67`) [I].
- The artifacts are the **same relation IPC files** (`{relation}.arrow`) that the importer COPYs
  into PostgreSQL. The import source reads `{name}.arrow` for relations
  (`import.rs::Source::read:111-136`) and places the manifest's artifacts, by the same file names,
  in content-addressed storage (`import.rs::Source::artifacts:137-170`). Every native relation is
  therefore **dual-stored**: PG rows (for example `lctx_serving.conditions`,
  `202609270004_projection.sql:401`) and a verified Arrow file [I].
- Load-time checks in `ipc_input::decode` [I] (`python/lctx_semantics/src/ipc_input.rs::decode:187-239`):
  - the exact file set;
  - ≤64 MiB per file;
  - `reader.schema() == bundle::files(0)` schema;
  - exactly one record batch;
  - row caps (surface files vs summary files).

  Then [I]:
  - `SemanticExecutor::from_ipc` checks `admit_discharges`, the kernel format
    (`lib.rs::ConditionGraph::new:256-260`) and effect-model digest consistency on value links
    (`lib.rs:646`) (`lib.rs::SemanticExecutor::from_ipc:1299-1345`).
- **Loading is whole-generation and eager at startup.** There is no per-operation or per-SCC
  selective load. The executor serves only `inspect_value_paths` (`server.py:563-576`) [I].
- `retrieval_units`, `retrieval_vectors` and `retrieval_receipt` are read, hashed and then dropped
  at startup (`server.py:412-416`, where `del inputs`). Only four non-native artifacts are kept
  (`generation.py:96-111`) [I].

#### 6.2.b10. Operations and cache (`operations.rs`, `cache.rs`)

- `operations.rs` is the append-only attempt, event, snapshot and generation discovery ledger
  (`lctx_ops.*`) on the generic `Store`, not on `ServingStore` (`operations.rs::Store::start_attempt:44-65`
  … `generations:186-194`). No MCP tool uses it [I].
- `cache.rs` is the embedding value cache (`lctx_cache.embedding_values`) keyed by
  `(spec_hash, input_hash)`. Integrity is re-checked on read: codec, dimensions, norm and value
  digest (`cache.rs::Store::cached:64-78`). It is used by compile/bundle embedding
  (`crates/cpg-core/src/embed.rs:189`) and never on the serving path [I].
- **Hydrated-object cache: absent** (§0.4) [I].

#### 6.2.b11. Report views (`cpg-core/src/postgres_read.rs`, `report.rs`)

- This is the operator `lctx db report` path (`crates/lctx/src/db.rs:116`), not MCP [I].
- **View allowlist.** `admits_plan` admits a `TableScan` only if the federated remote table is
  `lctx_report.generation_relations` or `lctx_report.operation_outline`
  (`postgres_read.rs::admits_plan:152-155`).
  - `ProviderPool::register` refuses mutable views (`:652-656`). `View::immutable` is true only for
    those two (`crates/cpg-schema/src/postgres_report.rs::View::immutable:36-38`).
  - The six mutable views are captured once in a REPEATABLE READ READ ONLY transaction into
    MemTables (`crates/lctx-postgres/src/report.rs::ServingStore::capture_report:23-107`).
  - `report()` registers only `GenerationRelations` (`postgres_read.rs::report:780-785`).
    `OperationOutline` is registered only in tests (`crates/cpg-core/tests/postgres.rs:752`) [I].
- **Expression algebra.** `admits_expr` (`postgres_read.rs::admits_expr:103-134`) accepts only the
  following [I]:
  - Column and Literal of type Boolean, Int64, Utf8, Binary or FixedSizeBinary(16|32)
    (`domain:93-102`);
  - Alias, IsNull, IsNotNull;
  - Not (boolean);
  - identity-only Cast;
  - BinaryExpr And/Or on booleans, Eq/NotEq on compatible types, and Lt/LtEq/Gt/GtEq on Int64
    only.

  Everything else is refused.
- **Plan algebra.** `admits_plan` accepts the following nodes, and nothing else (`:135-190`) [I]:
  - Projection of columns and aliases only, with no binary literals;
  - Filter;
  - SubqueryAlias;
  - Limit with non-negative literals;
  - Sort on columns only;
  - an **inner** Join with no filter, on the declared key of the same table:
    `(generation_digest, relation_name)` or `(generation_digest, node_id)` (`declared_join_keys:264-288`).
- **Enforcement layers** [I]:
  - `AdmitFederation::rewrite` leaves an unadmitted plan un-federated (`:437-439`).
  - `AdmittedTable` pushes down only admitted filters (`:540-555`) and refuses scans that carry
    unadmitted filters (`:564-571`).
  - `PinnedExecutor::execute` refuses physical filters (`:336-340`) and refuses catalog discovery
    (`:343-352`).
  - `SealedScan` enforces 200 000 rows and 128 MiB (`:628-634`).
  - `register` verifies that the provider sees the same ready manifest (`:665-676`) and injects
    `generation_digest = pin` (`:703-711`).

  Unadmitted plan shapes probably still run locally in DataFusion over `AdmittedTable` scans,
  rather than being refused. This is inferred from `Transformed::no` and
  `TableProviderFilterPushDown::Unsupported`; I did not trace datafusion-federation internals
  [IC].

#### 6.2.b12. Import (`import.rs`)

- **Per generation.** The generation key is the SHA-256 of the canonical manifest
  (`serving_projection.rs::Manifest::generation:432-437`, `202609280012_selection_retrieval.sql::prepare_generation:145`).
  Each import runs under an advisory lock per generation (`import.rs::lock:38-42`,
  `import_once:320-331`) [I].
- **Source.** The source is a verified Arrow IPC bundle directory, not Delta directly. The CLI
  first builds it from the published Delta snapshot (`crates/lctx/src/serving.rs::command:89-97`
  → `cpg_core::bundle::bundle`). `Source::open` validates everything before any database lease is
  taken: manifest, envelope, per-file SHA-256, relation receipts and `validate_relations`
  (`import.rs::Source::open:61-110`) [I].
- **Set-based COPY in 1000-row slices.** Relations load in FK order (`load_order:196-215`) [I].
  - Each slice goes through binary COPY into a temporary staging table, then
    `INSERT … SELECT` with `row_ordinal`, then a durable batch receipt, all in one transaction
    (`load_batch:585-656`).
  - An idempotency check (`SELECT` of the prior receipt) runs per slice (`:434-439`).
  - Slices halve on COPY byte refusal (`:417-429`).

  This is batched, not row-by-row, but costs about 7 statements per 1000 rows.
- **Finalise.** The steps are [I]:
  1. freeze (`:459-462`);
  2. ANALYZE (`:464-471`);
  3. `validate_stored`, which re-reads every relation with `LIMIT 1000 OFFSET n` paging and
     recomputes receipts (`:657-687`);
  4. `check_indexes`, which checks exact index definitions and the partitions (`:689-760`);
  5. `verify_locations`;
  6. `mark_ready`, which rechecks the artifact closure, receipt equality and row counts per
     relation (`202609280012_selection_retrieval.sql::mark_ready:113-138`).

  Corrupt or incompatible failures mark the generation `failed`. Others are `interrupted`, and the
  import is resumable (`import.rs:332-360`).

#### 6.2.b13. `nodes` and `edges`

The graph tables are **not served** [I]:

- They are absent from the served relation inventory (`crates/cpg-schema/src/bundle.rs::files:48-824`
  and its `catalog::serving_files` and `retrieval::files` extensions).
- `lctx-postgres` has no `lctx_serving.nodes` or `lctx_serving.edges` DDL. A grep over the
  migrations, `lctx-postgres/src`, `lctx_mcp` and `lctx_storage` finds nothing.
- The only contact is at bundle build time: Delta `edges` is joined to resolve fact provenance
  for `support_attribute_incidences` (`crates/cpg-core/src/bundle.rs:313-314`).

---

## 7. Invariant table (pipeline and serving)

### 7.1 Pipeline invariants (compile, rebuild, bundle)

| Invariant | Enforcement point | Failure behaviour |
|---|---|---|
| Every raw table present exactly once; no unknown names | `cpg-core/src/attempt.rs::write_raw:313-327` | `CoreError::UnknownTable`/`MissingTable`; attempt aborts, nothing visible |
| Batch schema equals declaration; all rows carry this snapshot | `attempt.rs::write:294-299` | `SchemaMismatch` / `ForeignSnapshot` |
| Delta table schema, `appendOnly`, retention and CHECK set equal the `table!` declaration | `cpg-core/src/delta.rs::verify:110-146` via `open_or_create:156-164` | `SchemaDrift`, `NotAppendOnly`, `Retention`, `ConstraintMismatch` |
| CHECK and non-null on write | `delta.rs::append:166-193` (`DeltaTable::write`) | write error, abort |
| Derived rows cast strictly to declared types and canonically sorted | `cpg-core/src/derive.rs::derive:23-43` (`to_declared`, `canonical_sort`) | cast error, abort |
| SQL is read-only | `cpg-core/src/sql.rs::read_only:15-20`, `query:24-26` (+ ast-grep `rules/sql-through-helper.yml`, Interface-checked) | planning error |
| A session reads only the snapshot's own commit files and rows | `cpg-core/src/snapshot.rs::commit_provider:144-151`, `register:156-172` | scan fails rather than silently short |
| Exactly one run declares `exports` | `attempt.rs::with_compiler_run:399-404` | `CoreError::Analysis` |
| Profile, roots and enrichment agree | `attempt.rs::compile_catalog_mode:576-583` | `CoreError::Analysis` |
| Catalog/evidence/domain tables equal a fresh pinned-source reconstruction | `cpg-core/src/catalog.rs::validate:1410-1497` (called from `validate.rs::validate_costed:169-183`) | `catalog-source-equality:*` violation → `CoreError::Invalid`, unpublished |
| Evidence identity/closure (scenario id, association id, span bounds, dangling refs, relabelling) | shared `cpg-schema/src/evidence.rs::validate:263-503`; called by the producer `cpg-core/src/evidence.rs:1209-1216` and by serving `serving_projection.rs:912` | `CoreError::Analysis` / projection corrupt |
| Key uniqueness, vocabularies, codebooks, finiteness, fact provenance both ways, declared references | generated/hand rules `cpg-schema/src/rules.rs::rules:3787-` (keys `:3790-3799`, vocab `:3800-3814`, codebooks, `finite:*`, `fact:*`/`fact-payload:*`), hand `REFERENCES:47-2406` | violations → unpublished |
| Unselected analysis tables empty under the catalog profile; brief state coherent | `crates/cpg-schema/src/rules.rs:3816-3823` | violations |
| Exactly one `catalog_compilation` row | `validate.rs::validate_costed:169-183` | `CoreError::Analysis` |
| A snapshot is published at most once; ambiguous commit classified by re-read | `attempt.rs::publish:1714-1728` | `AlreadyPublished` / `Unpublished` |
| Stage-cache bytes never become authority | `stage_cache.rs::read:63-97`, `contracts:285-345` | silent miss → recompute, overwrite; cache write failure only warns (`save:243-247`) |
| A forgotten analysis writer is not an error | `attempt.rs:1646-1651` | table silently published empty (behavioral profile has no emptiness rule) |
| Rebuild source is published and passes relational + raw-flow validators | `rebuild.rs::catalog:145-148`; `validate.rs::rebuild_source:142-154` | `CoreError::Analysis` / `Invalid` |
| Behavioral rebuild requires behavioral source | `rebuild.rs:156-160` | `CoreError::Analysis` |
| Rebuilt (copied) publication fully validated before publish | `rebuild.rs::copy_publication:304-308` | `CoreError::Invalid` |
| Bundle: one snapshot identity, one release, ≤ 1 embedding spec, documents' spec equals snapshot's, exactly one compilation profile, per-file projection validation, support row/byte limits, cross-relation validation, retrieval snapshot, vectors = embedded documents | `cpg-core/src/bundle.rs::build_with_retrieval:833-1011` | `CoreError::Bundle` |
| Same generation key ⇒ same bytes | `bundle.rs:1085-1098` | `CoreError::Bundle` |
| Generation verify: sha256, rows, schema digest per file, key names directory, projection generation and definition digest | `bundle.rs::verify:1130-` | `CoreError::Bundle` |

### 7.2 Serving invariants

| # | Invariant | Enforcement point | Failure behaviour |
|---|---|---|---|
| 1 | The serving role reads only `ready` generations | RLS `ready_read` on every `lctx_serving` projection table (`202609270004_projection.sql:18` and siblings; `202609280009_catalog.sql:26`; …), `generations` (`202609270003_serving.sql:25-27`), `is_ready` (`:31-34`) | Rows invisible, so empty results. Not a typed refusal [I from DDL] |
| 2 | A ready generation is immutable and terminal | `guard_generation` trigger (`202609270003_serving.sql:50-60`); `require_loading` trigger on every projection table (`:38-47`); `cleanup_generation` refuses ready (`202609280012_selection_retrieval.sql:173-178`); `immutable_profile` (`202609280005_publication.sql:113-117`) | SQL exception (SQLSTATE 55000) for writers. Readers are unaffected [I from DDL] |
| 3 | Serving connections are read-only, least-privilege, the expected role, and on PG 18 | `serving.rs::RoleConfig::options:101-127`; `RoleConfig::pool:139-148` | Pool open fails, so the server does not start [I] |
| 4 | Database schema equals the binary's migrations, and pgvector is 0.8.6 in `lctx_ext` | `serving.rs::check_connection:240-258`, `check_extension:260-266` | `Error::Schema`/`Config` → `StorageError(kind=incompatible)`, so the server does not start [I] |
| 5 | One generation and profile pinned per process. Manifest identity equals the digest. Library matches. Policy is exact | `repository.rs::ServingStore::pin:69-121` | `Request` or `corrupt` → startup failure [I] |
| 6 | Manifest format, projection definition, native identity and artifact inventory are compatible | `serving_projection.rs::Manifest::validate:314-421`; DB CHECK `generation_formats` (`202609280012_selection_retrieval.sql:8`) | `Incompatible`/`Corrupt` → startup failure [I] |
| 7 | Every artifact byte-equals its manifest receipt (SHA-256 and size) at startup | `import.rs::verify_artifact_locations:814-870` (from `pin`); `lctx_storage lib.rs::PinnedRepository::inputs:150-216`; `Manifest::verify_artifact:422-430` | Missing → `Unavailable`, damaged → `Corrupt`, over budget → `ResourceRefused`. Startup failure. **No re-verification after startup** (data is held in memory) [I] |
| 8 | Native IPC matches the declared serving schemas, kernel format and effect digest | `ipc_input.rs::decode:187-239`; `lctx_semantics/src/lib.rs::ConditionGraph::new:256-260`; `lib.rs:646`; `generation.py::load:74-94` | `ValueError`/`GenerationError` → startup failure [I] |
| 9 | The query embedding spec equals the generation's | `generation.py::load:77-84` (startup); `retrieval.rs:51-55`; `selection.rs:370-374`; `evidence_search.rs:41-48` | Startup failure, or a `Request` → `CapabilityError` [I] |
| 10 | Every request is scoped to the pinned snapshot and library | `repository.rs::check_snapshot:48-55`; `server.py:469-470,479-483,555-556,742-743`; `journeys.rs:17-19,247-249`; `evidence_search.rs:25-27`; native `lib.rs:1536-1540` | `CapabilityError` (MCP invalid params) [I] |
| 11 | Cursors are scoped to generation, request and policy | `journey_cursor.rs::position:30-52` (format, generation, policy digest, request sha256); `selection.rs::PreparedSelection::page:86-96` (generation, request hash, `POLICY_REVISION`); `evidence.rs::position:47-74` and `:385-392`; `wire/mod.rs::value_offset:544-564` | `Request` → `CapabilityError`, **except `find_operations`, where it is masked** (`server.py:603-605`). Cursors are unsigned base64 JSON [I] |
| 12 | Relation and key identifiers come only from the declared inventory. All data is bound | `repository.rs::fetch_projection:282-290` | `corrupt("undeclared query relation/key")` [I] |
| 13 | Rows read back re-satisfy the declared schema, codebooks, vocabularies and nullability | `projection.rs::decode_rows:119-131` → `serving_projection.rs::validate_batch:499-565` | `corrupt` → `ToolError("storage corrupt…")` [I] |
| 14 | Hydration, response and page byte and row budgets hold | `repository.rs::fetch_projection:327-352` (64 MiB, 200k rows); `repository.rs::check_response:406-423` (8 MiB); `evidence.rs::check_operation_packet:261-285` (32/256 KiB); `wire/mod.rs::tool_result:419-427`; `wire.py::SchemaTool.run:156-163`; `selection/catalog.rs:201-214,355-373,389-393` | `resource_refused` → `ToolError`. Paged tools pop items and set a cursor. Python-side refusals outside the contract decoder are masked [I] |
| 15 | Every tool request and response passes a Rust-owned wire contract | `wire.py::SchemaTool.run:108-116,130-138`; `wire/dispatch.rs::contracts!:3-86`; `wire/mod.rs::tool_result:390-429` | Request → `ValidationError`. Response → `ValueError` (masked), or `ToolError` if `resource_refused` [I] |
| 16 | Ranked winners belong to the generation, member and family, and the fusion equals the Rust policy | `winner_membership.sql`/`selection.rs:515-528`; `evidence_winner_membership.sql`/`evidence_search.rs:97-108`; `selection.rs:440-499` | `corrupt` → `ToolError` [I] |
| 17 | The selection classifier sees a closed, complete domain input | `selection/catalog.rs::PreparedCatalog::new:238-264`; `::classify:341-346`; `selection.rs:290-291` (undeclared dependency) | `WireError`/`corrupt` → `ToolError` [I] |
| 18 | CPU and native work are admitted and bounded | `serving.rs::ServingStore::run_cpu:286-301` (2 slots, try-acquire); `server.py::NativeWorkers.run:130-151` (2 slots, 1 s admission, 30 s deadline); `server.py::request_deadline:177-188`; `wire.py:118-121` | `resource_refused` → `ToolError` [I] |
| 19 | Cancelled database work does not leak pool capacity | `serving.rs::QueryLease::drop:208-237` | Drains. If draining fails, **the whole serving pool is closed**, and later calls fail as `unavailable` until restart [I] |
| 20 | Capability absence is explicit, never silent | `server.py::unavailable:83-90`; `packet.rs:152-180,311-320`; `journeys.rs:137-143`; `serving_projection.rs::relation_requested:177-191` with `Manifest::validate:315-319` (no rows for unadvertised capabilities) | Typed `CapabilityUnavailable`, or a section state `not_requested` or `unavailable` [I] |
| 21 | An imported generation equals its bundle (receipts, row counts, indexes, partitions) before it is ready | `import.rs::Source::open:61-110`; `validate_stored:657-687`; `check_indexes:689-760`; `mark_ready` (`202609280012_selection_retrieval.sql:113-138`) | Import `failed` (corrupt or incompatible) or `interrupted` (resumable). The generation never becomes ready [I] |
| 22 | Report federation touches only immutable, allowlisted views under the pinned generation | `postgres_read.rs::admits_plan:135-190`, `admits_expr:103-134`, `ProviderPool::register:652-711`, `SealedScan::execute:617-639` | Plan error, no federation, or `ResourcesExhausted` [I]. That unadmitted plans fall back to local execution is [IC] |

## 8. External-review claim ledger (`docs/external-review-fully-semantic-graph-data-model.md`, untracked)

| ID | Claim (review line) | Verdict | Summary |
|---|---|---|---|
| E12 | "PostgreSQL already supports generation-pinned selection and set-based hydration" (`:652`) | **Confirmed, refined** | Pinning is real and enforced (one pin per server lifespan, `generation_digest=$1` everywhere, RLS to ready, ready immutable). "Selection" is only the stored default generation for new servers; requirement classification is recomputed in Rust per request and page from full-generation loads. The hydration primitive is set-based (`key = ANY($2)`), but journeys are not uniformly set-based (N+1 evidence hits, per-candidate compare loop, sequential per-relation fetches). |
| E13 | "the serving contract retains Arrow artifacts for conditions, summaries, identities, call bindings and related native inputs" (`:652`) | **Confirmed, refined** | 29 `NATIVE_FILES` cover exactly those classes, but only when `capabilities.native_value_paths` (behavioral profile); they are the same `{relation}.arrow` files also COPYed into PG (dual-stored), loaded whole and eagerly at startup, used only by `inspect_value_paths`. No selective (per-operation/SCC) load path. |
| E14 | "`cpg-core/src/postgres_read.rs` admits a deliberately restricted set of report views and expressions" (`:731`) | **Confirmed** (scope refined) | Two immutable views allowlisted, a narrow expression and plan algebra, declared join keys, generation injection, sealed scan budget. It is the operator `lctx db report` path, not MCP serving; production registers only `generation_relations`. Unadmitted shapes probably run locally rather than being refused [IC]. |
| E15 | "current catalog hydration batches keys and separately expands nested type relations" (`:674`) | **Confirmed, refined** | Keys are batched per relation; types are expanded by a separate BFS, 2 queries per level with a `visited` set. All statements are awaited one after another on one connection: `catalog_record` = 9 + 2L round trips, no join, recursive CTE or pipelining; each 200-row chunk is re-validated. |
| E16 | "The current Stage E implementation already separates projection acquisition from analytics" (`:594`) | **Refined** | Separation is by crate boundary (`lctx-analytics` cannot touch SQL/Delta), with one declared projection acquisition; but Stage E interleaves ~18 further SQL acquisitions and embedder calls between passes, has no declared input set or stage key, and is reused only by whole-publication copy. |
| E17 | "Existing generation identity may already cover many dependencies" (`:646`) | **Confirmed for serving; refined for compile** | PG `generation_digest` covers every served row and artifact and is immutable, so it can key per-generation serving caches (e.g. classifier preparation). Compile-side `content_digest` is one digest per publication, over-broad (full compiler digest) and proxy-based (run ids with a hand-bumped extractor version); per-stage keys are computed separately. Random `snapshot_id` inside both generation identities prevents cross-compile sharing. |

**E12 evidence** [I].
1. Pinning: `crates/lctx-postgres/src/repository.rs::ServingStore::pin:69-121`, `python/lctx_mcp/src/lctx_mcp/server.py::build_server.lifespan:405-429`; every statement carries `generation_digest=$1` (`repository.rs::fetch_projection:312-315`, every `crates/lctx-postgres/queries/*.sql`); RLS `ready_read … USING (lctx_serving.is_ready(generation_digest))` (`crates/lctx-postgres/migrations/202609270004_projection.sql`, `202609280009_catalog.sql:26`); `guard_generation`/`require_loading` triggers (`202609270003_serving.sql:38-60`); `select_generation` sets the default for **new** servers (`202609280012_selection_retrieval.sql:212-221`; "Existing servers retain their pin", `crates/lctx/src/serving.rs:44`).
2. Classification is not stored or reused: `crates/lctx-postgres/src/selection.rs::ServingStore::prepare_selection:201-344`, `PreparedCatalog::new(&input)?.classify(&selection)` at `:325-327`; `crates/cpg-schema/src/selection/catalog.rs::PreparedCatalog::new:180-331` builds ~18 indexes and re-parses domain/scenario/deployment JSON each time.
3. Set-based primitive: `repository.rs::fetch_projection:290-314`. Non-set-based journeys: `crates/lctx-postgres/src/evidence_search.rs:178-203` (per hit), `crates/lctx-postgres/src/journeys.rs:254-313` (per candidate), `crates/lctx-postgres/src/hydration.rs:100-267` (sequential), `hydration.rs:286-395` (`get_capability`, 10 sequential).
4. No cross-request cache: `selection.rs:1` ("No connection or cache survives preparation"); `crates/lctx-postgres/src/cache.rs` is the compile-time embedding cache only.

**E13 evidence** [I]. `crates/cpg-schema/src/serving_projection.rs::NATIVE_FILES:12-42` = conditions (4: `conditions`, `condition_nodes`, `analysis_conditions`, `analysis_condition_nodes`), surface (3: `operations`, `public_paths`, `callable_parameters`), summaries (4: `summary_flows`, `summary_flow_steps`, `summary_boundaries`, `behavior_discharges`), identities (3: `source_parameter_identities`, `source_context_value_identities`, `source_modeled_identities`), call bindings (3: `source_call_bindings`, `source_call_normals`, `source_call_header_steps`), source bodies (3), frame exits (3), context protocols (3), return certificates (1), test value links (2: `flow_test_leaves`, `flow_test_value_links`). Gated by `serving_projection.rs::artifacts_for:51-67` and `crates/cpg-schema/src/catalog.rs::Capabilities::for_profile:39-46`; dual storage via `crates/lctx-postgres/src/import.rs::Source::read:111-136`, `Source::artifacts:137-170`; eager load `python/lctx_mcp/src/lctx_mcp/generation.py::load:85-94`, `python/lctx_semantics/src/ipc_input.rs::decode:187-239`; sole consumer `server.py::inspect_value_paths:540-581`. Hand-name duplication of this inventory: see §3.6.2 item 7.

**E14 evidence** [I]. `crates/cpg-core/src/postgres_read.rs::admits_expr:103-134`, `admits_plan:135-190` (TableScan only on `lctx_report.generation_relations`/`operation_outline`, `:152-155`), `declared_join_keys:264-288`, `ProviderPool::register:652-711` (immutable views only; manifest equality `:665-676`; generation injection `:703-711`), `SealedScan::execute:617-639` (200 000 rows, 128 MiB); `crates/cpg-schema/src/postgres_report.rs::View::immutable:36-38`; production registration `postgres_read.rs::report:780-785`; caller `crates/lctx/src/db.rs:116`. Local fallback of unadmitted shapes inferred from `Transformed::no`/`TableProviderFilterPushDown::Unsupported` [IC].

**E15 evidence** [I]. `crates/lctx-postgres/src/hydration.rs::catalog_invocation:100-157` (5), `catalog_record:185-273` (+4, then `catalog_types`/`catalog_type_args` per level, `:237-267`); `repository.rs::fetch_projection:290-352` (`= ANY($2)`, 200-row decode through `crates/lctx-postgres/src/projection.rs::decode_rows:119-131` → `serving_projection.rs::validate_batch:499-565`). Some hydration keys lack a supporting index (`brief_members(brief_id)`, `public_paths(node_id)`, `operation_facets(node_id)`) [IC, DDL only].

**E16 evidence** [I]. `crates/cpg-core/src/analyze.rs::project:547-552` acquires the declared `ProjectionSpec` (`crates/cpg-schema/src/projection.rs`) into `lctx_analytics::graph::Projection`; `crates/lctx-analytics/Cargo.toml` `[dependencies]` = cpg-schema, arrow-array, petgraph, fixedbitset, leiden-rs, serde, serde_json, toml, thiserror (no DataFusion, Delta or SQL). `analyze.rs::run:555-` then interleaves further `collect(` SQL acquisitions (`neighbours::passages_sql`, `neighbours::api_texts_sql`, `communities::{shared_types_sql, co_mention_sql, co_use_sql}`, `flows::{argument_flows_sql, guards_sql, handoffs_sql, parameter_reads_sql}`, `concepts::attributes_sql`; 18 `collect(`/`sql::` call sites in the file) and embedder calls (`:597-680`) between passes. No Stage E stage key exists; reuse is `rebuild.rs::copy_publication:295-303` only.

**E17 evidence** [I].
- Serving: `serving_projection.rs::Manifest:215-231` and `Manifest::generation:432-437` (full SHA-256 of the manifest JSON, including per-relation row-hash receipts `receipt:573-` and artifact SHA-256s); import requires `projection_generation` equality (`import.rs::Source::open:60-71`); rows frozen after loading (`crates/lctx-postgres/src/projection.rs:110`, `require_loading`). The serving preparer code (`selection.rs`) is not inside the digest, which is harmless for an in-process cache keyed by `generation_digest` (+ `profile_digest`, `crates/lctx-postgres/src/profiles.rs:41-43`).
- Compile: `crates/cpg-core/src/attempt.rs::content_digest_with:276-287` (run ids + `compiler_digest()` + embedding receipt); extractor identity `crates/cpg-extract/src/config.rs:31` ("Bumped by hand") and `config.rs::producer:427-455`; stage keys computed independently by full-scan hashing (`crates/cpg-core/src/stage_cache.rs::key:14-54`).
- Limitation: `snapshot_id` is random (`crates/lctx/src/main.rs::random_id:486-490`) and is a field of both the outer bundle manifest (key `crates/cpg-core/src/bundle.rs::key_of:710-717`) and the projection manifest (`Manifest.snapshot_id`); cross-compile sharing would need `content_digest` or a snapshot-free projection digest.

## 9. Serving summary (from §6)

1. There is **no hydrated-object or prepared-selection cache**. `PreparedCatalog` is rebuilt from
   full-generation relation loads on every `find_operations`, `search_operations` and
   `compare_operations` request, and on every page (`selection.rs:325-327`) [I].
2. Round trips per request:
   - `get_capability`: exactly 10 sequential.
   - `catalog_record`: 9+2L.
   - `get_operation` packet: 15+2L to 20+2L.
   - `compare_operations`: up to about 58.
   - `search_evidence`: 2–3 + k to 2k, a per-hit N+1 loop.

   Independent fetches are serialised on one connection [I].
3. Failure handling is typed and refusal-oriented, with four exceptions [I]:
   - A `find_operations` cursor or `Request` error is **masked**.
   - Python-side retrieval budget or corruption `ValueError`s are **masked**.
   - `search_operations` does not surface its lexical fallback reason.
   - One undrained cancelled lease closes the whole pool.
4. Native artifacts (29) exist only under the behavioral profile. They are dual-stored in PG and
   as IPC files, and loaded eagerly and whole at startup for `inspect_value_paths` only [I].
5. `nodes` and `edges` are not part of the serving contract [I].

