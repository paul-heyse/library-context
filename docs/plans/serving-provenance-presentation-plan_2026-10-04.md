# Serving provenance and presentation alignment

**Proposed, 2026-10-04.** Supporting design for the [target-alignment coordinator](target-implementation-alignment-plan_2026-10-04.md). It actions TA-F02/F03, LL-F05 and selected LL-F09/10 opportunities while preserving separately stopped Phase 5 activation. The coordinator owns disposition and combined fixture acceptance.

## 1. Foundations and product boundary

The current Rust response/schema owner, immutable process generation, bounded packet hydration, retained CPU/query grants, canonical assertions and thin FastMCP transport remain suitable. No generation mixing, false API nonexistence or retrieval-quality regression is established by the review. The concrete gaps are implicit requested input admission and discarded generation metadata in copied capability resources.

The [API/evidence target](../design/sections/api-and-evidence-product.md) owns intended discovery/correct-use outcomes and §14.12 comparative qualification. A result is evidence only under its actual source/model/coverage; retrieval is nomination. Authoring and fixture qualification do not authorize a live pilot or product-confirmation run.

## 2. S1 — requested library domain admission

### Domain operation and response policy

Add one pure model-owned resolution operation/prepared index for a requested library Name within the pinned generation. Its declared inputs are captured input/package/release identity, `InputDistribution` FirstParty membership, admitted input/provider coverage and `CorpusLibrary` association. Do not resolve from observed members or all dependency packages: acquired dependencies are not automatically first-party analyzed libraries.

A Name denotes the finite union of matching first-party `(input, release)` captures and admitted corpus captures in this generation. There is no implicit latest-version choice. Preserve multiple captures and release IDs; exact operation lookup can remain Ambiguous. Corpus-only evidence is admitted where captured for that library even when there are no public members. Build the index with existing bounded/charged primitives and stable nominal IDs; no persistent library registry or second selection authority.

An unknown/unadmitted Name produces a dedicated typed semantic admission refusal **before** member filtering or ranking. Do not reuse store `Error::Absent`, whose meaning is missing generation, or turn unknown domain into a successful Available/Missing result. Propagate the dedicated outcome through the PG effect wrapper, native failure mapping and Rust-owned public failure/schema policy; Python only transports it.

For a known admitted domain, successful empty results remain legitimate. Responses include admitted-domain/release metadata and relevant coverage so partial/unavailable/not-requested collection remains distinct from complete enumeration of the finite canonical store. `CompleteDomain` describes exhaustive enumeration of the specified admitted canonical catalog universe. Keep provider/analysis coverage separate: finite store enumeration may be complete while extraction coverage is partial. Neither member count nor CompleteDomain proves universal Python API absence.

An omitted library filter continues to search the admitted generation union under existing rules. Multiple admitted inputs with the same name do not silently select one. Partial admission retains known positive candidates and their releases while explaining unresolved coverage; it does not certify the missing remainder as absent.

### Consumer migration and schema effects

Reuse the resolved domain in catalog belongs/candidates, browse, compare, GetOperation/packet lookup and SearchOperations/SearchEvidence/SearchCapabilities membership, including captured corpus-only units. Resolve once per request/prepared view and pass the typed result; adapters do not recreate matching rules.

Extend `PreparedDependency::CatalogIdentity`/affected mapping inputs beyond current distribution/ownership/verification to include the resolver's input/release/corpus/coverage dependencies. Hydrate through the declared PG reader with charge/lease lifetime retained. Public request syntax can remain Name-based; response metadata and the dedicated failure are a deliberate wire/schema migration. Preserve existing candidate ambiguity, selection, evidence closure and rank eligibility-before-score.

Document admission/outcome meaning at the API/evidence owner and record any meaningful changed public policy via ADR before publication. Review schema snapshots, actual schema-backed Tool exposure and native adapters. No canonical admission registry is persisted; if existing captures cannot certify a required case, return explicit unavailable support rather than invent admission.

### Independent acceptance

Use actual generation fixtures for unknown library, admitted-empty, admitted-nonempty, partial/not-requested provider coverage, dependency-only name, multiple first-party releases, corpus-only evidence and omitted filter. Check failure versus success/coverage at pure model, actual PG/native and MCP wire layers, including cursor policy invalidation and original-generation pinning. Compare/search/browse/packet paths must agree on admitted identities. A positive candidate in partial coverage remains discoverable; no fabricated CompleteDomain or latest choice.

## 3. S2 — generation-bearing capability resources

Move resource rendering onto `GetCapabilityResponse`, the generation-bearing Rust owner. Render the unchanged authored `CapabilityPacket.rendered` bytes first, then compact snapshot metadata containing the canonical GenerationKey and capability ID, then existing assertion metadata. Metadata remains outside the authored body so copied resources identify their snapshot without changing canonical assertion evidence or authored content hashes.

Keep `lctx://capability/{capability}` explicitly process-relative to the server's lifetime pin. Do not introduce generation-qualified URI routing, process repinning or another selection service. The returned metadata makes the snapshot visible; a client needing another generation starts the existing separately pinned process/selection path.

Native serving renders from the full response rather than discarding its generation. Budget the complete resource representation, including snapshot/assertion additions. Python's final SDK-envelope byte admission remains required; pre-serialization Rust limits do not replace it. Preserve assertion/proof identity, failure-once fallback and resource cancellation semantics.

Bump the resource-presentation identity in dispatch: renderer source is not automatically part of the present wire identity. This is a presentation/wire contract migration, not a canonical relation/corpus rebuild for S2 alone. Update model owner, native method, schema/dispatch identity and thin Python consumer together; remove the generation-dropping call path.

Acceptance: tool/resource GenerationKey parity, independently checked unchanged authored prefix/content hash, exact assertion/proof metadata and the complete final resource byte envelope. Process selection changes elsewhere must not change the existing process resource generation. Foreign/stale capability and pin-loss failure controls remain in scope. A tiny stub is not actual MCP qualification.

## 4. S3 — owned value presentation

Create ordinary model-owned rendering functions with explicit modes: human display, captured source spelling when available, and executable Python expression only when representable. Share them across retrieval/options and synthesis; do not treat the original source as an evaluated default. Preserve Absent, Unknown, Unavailable, unevaluated Expression and Factory states.

Render semantic literal variants deliberately instead of whole-record Debug or consumer-specific switches. Strings and bytes preserve escaping and byte meaning; integers remain exact; finite floats preserve signed zero and precise value. Nonfinite floats and NaN payloads retain exact bits/source qualification and receive explicit display/unavailable-expression treatment. Never pretend every value has a finite executable Python literal. Do not evaluate a factory or import a library to obtain presentation.

Codebook labels come from F1's finite code owner where used; display vocabulary must not become a new codebook authority. Keep canonical literal values and evidence untouched. Only consumers whose modes actually change migrate; source snippets remain original ranges.

Bump `RENDER_VERSION`/affected corpus and synthesis identity, rebuild units/briefs and invalidate matching derived vector artifacts. Coordinate F4 where identity contracts overlap. Keep original evidence and mandatory packet replay; no stale text/vector reuse. Live embeddings remain stopped, so fixture/cache invalidation controls establish this migration without claiming live quality.

Acceptance: independent strings/bytes/escaping, large integers, signed zero, infinities/distinct NaN bits, absent/unknown/unavailable/factory defaults and unavailable source/expression controls. Retrieval and synthesis must preserve distinctions while using appropriate modes. Source text/hash stays unchanged. Quality benefit remains Proposed until IS1/IS3 comparison; absence of measured improvement does not permit a false value rendering.

## 5. S4 — serving mechanics and maintenance

Use compiled hex facilities for cursor encoding/decoding while preserving exact token format, generation/request/policy/channel/group binding and invalid-input rejection. Independent existing token vectors remain the oracle; do not loosen verification because decoding succeeds.

Consolidate SQLSTATE failure classification through its current owner but leave retry eligibility as a separate explicit policy. `57014` cancellation is not made retryable. Preserve committed-winner readback, timeout/cancel/exhaustion classes and request grant lifetime.

Consolidate PostgreSQL option builders and verify-full checks at the existing effect/config owner. Keep role/TLS/URI/secret handling, ordinary non-superuser runtime access and current environment policy. This is not a new configuration framework or driver.

Delete unused exported Service tools/resources/schema accessors after scoped consumer inspection; delete direct pyarrow only if Python/tooling/transitive requirements establish it is unused. A no-import search alone cannot prove an environment dependency removable. Preserve the wire-schema/final-byte mirror and transport-specific objects that have consumers. Add or adjust focused tests only for material contract/regression risk.

Any dependency/tool/transport upgrade is a separate exact-pin task under pin-check and actual transport qualification. Earlier review release/security statements are historical, not verified-current recommendations.

## 6. Product and operational investigations

| Package | Bounded preparation, decision criterion and trigger |
|---|---|
| IS1 lexical/retrieval | Freeze an approved development task/corpus before comparing camel/acronym split, Rust stemming, PG FTS/trigram/textsearch or tantivy. A named fuzzy/phrase/token gap must justify adoption; compare candidate/evidence retention and abstention under fixed ranks/settings. Preserve Rust-owned tokens and version corpus/policy; no Python stemmer in compile. ANN/alternate vector engines require a separate measured/nameable gap |
| IS2 startup lifecycle | Inspect/qualify selection and native preparation admission, cancellation before/during work, retained artifacts, failed startup and drain of actual blocking work. Request 30s deadlines do not certify startup behavior. Keep spawn_blocking grant retention. TaskTracker is considered only for a second drain/tracked-task consumer and must retain grants until work actually ends. Disposable fixtures can precede activation; actual pilot remains stopped |
| IS3 evaluation readiness | Retain §14.12 Proposed protocol: freeze comparable task population, versions, corpus/render/rank identities, tools/budgets, development versus 24 sealed confirmation boundaries and independent grading. Capture failure categories and stopping rule. Preparation excludes sealed answers and execution; PR6/activation authorization is prerequisite for the comparative run |
| IS4 client/library maintenance | Keep conformance-checked Rust/Python embedding twins unless parity drift or a second change warrants replacement plus ADR. HF tokenizers requires service-independent admission need. Inspect useful pydantic/vLLM skill routing; qualify vLLM custom 0.30.1rc1 versus skill claims before enabling. No broad skill/dependency upgrade or runtime service change |

Investigations record adopt, retain or defer at the coordinator, including evidence and enabled consumers. No consumer means a concrete trigger, not an unconditional replacement. Optional quality/performance/RSS campaigns do not block established fidelity and provenance corrections. Backend availability alone proves neither task usefulness nor a new search policy.

## 7. Integration and acceptance

S1/S2 are independent of A1 and F1/F2 but share serving/model/native/wire editing surfaces. S3 consumes finite code-label/identity slices only where needed, not whole foundation plans. S4 local mechanics and readiness inquiries can proceed alongside them with an explicit shared-file owner.

Each selected change includes model producer, hydration/native/wire consumers, old-path deletion and focused tests. Q0 in the coordinator schedules pending Named migration, matching artifacts, remaining actual PG/MCP journeys and same-tree full functional/hygiene gates. Reader pin, resource retention, original condition/support and final bytes remain acceptance constraints.

The Phase 5 plan retains activation/reconstruction/live-vector scope and its historical receipts. No selected generation, store reset, live embeddings, PR6 evaluation, quality/speed/RSS claim or production remediation occurs in this plan-authoring task. All targets and planned controls remain Proposed/not_run until execution.
