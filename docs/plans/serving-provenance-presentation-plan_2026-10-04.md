# Serving provenance and presentation alignment

**Implemented contracts with historical scoped receipts, 2026-10-04–05; native remediation acceptance pending, 2026-10-06.** The [target-alignment coordinator](target-implementation-alignment-plan_2026-10-04.md) retains TA-F02/F03, LL-F05 and selected LL-F09/10 dispositions. Current native implementation and acceptance belong to [graph-native coordinator §9](graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution). IS1–IS4 transfer to the [forward product queue](behavioral-model-forward-plan_2026-09-24.md#product-operational-investigations); Q1/operator activation remains separately stopped.

## 1. Foundations and product boundary

Rust owns response/schema meaning, immutable process pins, bounded packet hydration, owned CPU/query lifetime and canonical assertions; FastMCP remains a thin transport. Native content and complete executable-realization identity replace PostgreSQL generations/grants. The original review's requested-input and resource-provenance gaps retain their product meaning; its historical receipt does not establish current native acceptance or retrieval quality.

The [API/evidence target](../design/sections/api-and-evidence-product.md) owns intended discovery/correct-use outcomes and §14.12 comparative qualification. A result is evidence only under its actual source/model/coverage; retrieval is nomination. Authoring and fixture qualification do not authorize a live pilot or product-confirmation run.

## 2. S1 — requested library domain admission

### Domain operation and response policy

Add one pure model-owned resolution operation/prepared index for a requested library Name within the pinned generation. Its declared inputs are captured input/package/release identity, `InputDistribution` FirstParty membership, admitted input/provider coverage and `CorpusLibrary` association. Do not resolve from observed members or all dependency packages: acquired dependencies are not automatically first-party analyzed libraries.

A Name denotes the finite union of matching first-party `(input, release)` captures and admitted corpus captures in this generation. There is no implicit latest-version choice. Preserve multiple captures and release IDs; exact operation lookup can remain Ambiguous. Corpus-only evidence is admitted where captured for that library even when there are no public members. Build the index with existing bounded/charged primitives and stable nominal IDs; no persistent library registry or second selection authority.

An unknown/unadmitted Name produces a dedicated typed semantic admission refusal **before** member filtering or ranking. Do not turn unknown domain into a successful Available/Missing result. Preserve its typed cause through `lctx-serving`, NativeSession and the Rust-owned safe `PublicFailure` tool/resource envelope; Python only transports it. Missing or incompatible native realization remains a distinct failure.

For a known admitted domain, successful empty results remain legitimate. Responses include admitted-domain/release metadata and relevant coverage so partial/unavailable/not-requested collection remains distinct from complete enumeration of the finite canonical store. `CompleteDomain` describes exhaustive enumeration of the specified admitted canonical catalog universe. Keep provider/analysis coverage separate: finite store enumeration may be complete while extraction coverage is partial. Neither member count nor CompleteDomain proves universal Python API absence.

An omitted library filter continues to search the admitted generation union under existing rules. Multiple admitted inputs with the same name do not silently select one. Partial admission retains known positive candidates and their releases while explaining unresolved coverage; it does not certify the missing remainder as absent.

### Consumer migration and schema effects

Reuse the resolved domain in catalog belongs/candidates, browse, compare, GetOperation/packet lookup and SearchOperations/SearchEvidence/SearchCapabilities membership, including captured corpus-only units. Resolve once per request/prepared view and pass the typed result; adapters do not recreate matching rules.

The model-owned catalog-domain preparation consumes actual input/release/corpus/ownership/verification/coverage dependencies. Hydrate them through the pinned native reader with reservations and owned reader lifetime retained. Public request syntax remains Name-based; response metadata and the dedicated failure belong to the shared wire/schema owner. Preserve candidate ambiguity, explicit selection, evidence closure and rank eligibility-before-score.

Document admission/outcome meaning at the API/evidence owner and record any meaningful changed public policy via ADR before publication. Review schema snapshots, actual schema-backed Tool exposure and native adapters. No canonical admission registry is persisted; if existing captures cannot certify a required case, return explicit unavailable support rather than invent admission.

### Independent acceptance

Use owned persistent native fixtures for unknown library, admitted-empty, admitted-nonempty, partial/not-requested provider coverage, dependency-only name, multiple first-party releases, corpus-only evidence and omitted filter. Check failure versus success/coverage at pure model, native serving and MCP layers, including cursor policy/realization invalidation and unchanged old-reader pins. Compare/search/browse/packet paths must agree on admitted identities. A positive candidate in partial coverage remains discoverable; no fabricated CompleteDomain or latest choice.

## 3. S2 — generation-bearing capability resources

Resource rendering consumes `GetCapabilityResponse`, the snapshot-bearing Rust owner. Render unchanged authored `CapabilityPacket.rendered` bytes first, then compact metadata containing the complete native pin and capability ID, then existing assertion metadata. Metadata remains outside the authored body so copied resources identify their snapshot without changing canonical assertion evidence or authored content hashes.

Keep `lctx://capability/{capability}` explicitly process-relative to the server's lifetime pin. Do not introduce generation-qualified URI routing, process repinning or another selection service. The returned metadata makes the snapshot visible; a client needing another generation starts the existing separately pinned process/selection path.

Native serving renders from the full response rather than discarding its generation. Budget the complete resource representation, including snapshot/assertion additions. Python's final SDK-envelope byte admission remains required; pre-serialization Rust limits do not replace it. Preserve assertion/proof identity, failure-once fallback and resource cancellation semantics.

Automatic serving source membership captures executable renderer changes in the native realization
guard; shared model implementation capture remains complete. Changed public presentation policy
or schema still belongs to the model wire identity. Update actual model/native/Python consumers
together and rebuild a matching realization; no legacy resource path is retained.

Acceptance: tool/resource complete-pin parity, independently checked unchanged authored prefix/content hash, exact assertion/proof metadata and the complete final resource byte envelope. Selection changes elsewhere must not change an existing process's resource pin. Foreign/stale capability and pin-loss failure controls remain in scope. A tiny stub is not actual MCP qualification.

## 4. S3 — owned value presentation

Create ordinary model-owned rendering functions with explicit modes: human display, captured source spelling when available, and executable Python expression only when representable. Share them across retrieval/options and synthesis; do not treat the original source as an evaluated default. Preserve Absent, Unknown, Unavailable, unevaluated Expression and Factory states.

Render semantic literal variants deliberately instead of whole-record Debug or consumer-specific switches. Strings and bytes preserve escaping and byte meaning; integers remain exact; finite floats preserve signed zero and precise value. Nonfinite floats and NaN payloads retain exact bits/source qualification and receive explicit display/unavailable-expression treatment. Never pretend every value has a finite executable Python literal. Do not evaluate a factory or import a library to obtain presentation.

Codebook labels come from F1's finite code owner where used; display vocabulary must not become a new codebook authority. Keep canonical literal values and evidence untouched. Only consumers whose modes actually change migrate; source snippets remain original ranges.

Bump `RENDER_VERSION`/affected corpus and synthesis identity, rebuild units/briefs and invalidate matching derived vector artifacts. Coordinate F4 where identity contracts overlap. Keep original evidence and mandatory packet replay; no stale text/vector reuse. Live embeddings remain stopped, so fixture/cache invalidation controls establish this migration without claiming live quality.

Acceptance: independent strings/bytes/escaping, large integers, signed zero, infinities/distinct NaN bits, absent/unknown/unavailable/factory defaults and unavailable source/expression controls. Retrieval and synthesis must preserve distinctions while using appropriate modes. Source text/hash stays unchanged. Quality benefit remains Proposed until IS1/IS3 comparison; absence of measured improvement does not permit a false value rendering.

## 5. S4 — serving mechanics and maintenance

Use the model-owned cursor codec with complete pin, request/policy, channel/group and nested-parent
binding. Current-format invalid or foreign tokens refuse; decoding alone never admits membership.
Focused current cursor controls challenge checksums, full ordering keys and existing foreign parents;
historical token formats/vectors do not require an old-format reader.

Native failure classification preserves model-owned typed causes and fixed safe public messages. Retry eligibility remains separate from classification; cancellation does not become an automatic retry. Preserve committed-winner readback, timeout/cancel/exhaustion meaning and request/reader lifetime through actual SDK drain. Partial streamed results remain provisional until terminal success.

Use the existing native RuntimeConfig/ViewerConfig and lifecycle owner. Root installer credentials stay outside serving; database-scoped VIEWER credentials and one atomic selection authority supply new session pins. Preserve endpoint/TLS/secret handling and explicit configuration without PostgreSQL environment fallback. The [SurrealDB runbook](../surrealdb.md) owns executable commands.

Delete unused exported Service tools/resources/schema accessors after scoped consumer inspection; delete direct pyarrow only if Python/tooling/transitive requirements establish it is unused. A no-import search alone cannot prove an environment dependency removable. Preserve the wire-schema/final-byte mirror and transport-specific objects that have consumers. Add or adjust focused tests only for material contract/regression risk.

Any served FastMCP/mcp-types or transport upgrade is a separate task under pin-check (their wire-byte pin) and actual transport qualification; other dependencies float (ADR-0125). Earlier review release/security statements are historical, not verified-current recommendations.

## 6. Product and operational investigations

IS1 lexical/retrieval, IS2 startup lifecycle, IS3 evaluation readiness and IS4 client/library
maintenance now have their complete criteria and triggers at the [forward product queue](behavioral-model-forward-plan_2026-09-24.md#product-operational-investigations).
That existing owner records adopt, retain or defer and enabled consumers. This relocation does not
authorize product confirmation, operator activation or quality/performance/RSS campaigns.

## 7. Integration and acceptance

S1/S2 are independent of A1 and F1/F2 but share serving/model/native/wire editing surfaces. S3 consumes finite code-label/identity slices only where needed, not whole foundation plans. S4 local mechanics and readiness inquiries can proceed alongside them with an explicit shared-file owner.

Each selected change includes model producer, hydration/native/wire consumers, old-path deletion and focused tests. Graph-native §9/R-Q0 schedules matching artifact/wheel, owned persistent native/CLI/PyO3/MCP controls and applicable leaves. The user's targeted-testing instruction excludes broad `just qualify`, stopped compiler suites and historical parity. Reader pin, resource retention, original condition/support and final bytes remain acceptance constraints.

The forward plan and analytical-enrichment owner retain Q1/product obligations; Phase 5 retains its historical stopped reconstruction/activation receipt. S1–S4 and enabled IS2 have historical bounded receipts, while current native remediation acceptance remains at graph-native §9/R-Q0. No operator selection/reset, live embeddings, PR6 evaluation or quality/speed/RSS claim is authorized by this execution.
