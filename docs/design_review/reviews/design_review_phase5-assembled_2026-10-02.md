# Phase 5 assembled generation-bound serving — design review

Command excerpts identify verification scope; original invocations remain in the cited logs and Git history.

**Decision: Revise, at Implemented maturity, 2026-10-02.**

The assembled implementation establishes a coherent serving boundary over canonical generations. One Rust runtime owns the original generation guard, request capacity, query connections, deadlines and reservations. Classification, ranking and finite exact-input assessment have model owners; Python supplies transport and numerical BM25 scoring. The disposable vector artifact remains subordinate to canonical embedding values.

Two material gaps prevent acceptance of the complete supported scope. Transport serialization allocates and executes outside the declared native execution boundary before admission. Capability hydration loads authoritative assertion evidence status but removes it from served packets and resource text. These are bounded integration and representation defects; neither requires restoring legacy authorities or redesigning the generation store.

Architectural acceptance and Phase 5 qualification remain separate. Full Q0 gates, both-profile pinned FastMCP journeys and operator activation are **not_run** at this review boundary.

## 1. Scope and evidence boundary

| Field | Value |
|---|---|
| Standard | Core/template 3.2; code-intelligence profile 1.3; library-context binding |
| Tier · purpose | Design · target |
| Reviewer · date | Independent delegated design reviewer, 2026-10-02 |
| Baseline | Main `0bc8ea11171d6fd96827a8e250964d3de5b3b4ce`, plus current uncommitted Phase 5 implementation |
| Preservation boundary | Initially dirty work preserved under snapshot `bd22f66c71023cf7ca642eea8e82689941e5ac44`; apparent formatting is not treated as new semantic scope |
| Supported scope | Ten retained tools, capability resource, canonical Catalog admission, optional brief/native enrichment, original evidence, exact-profile retrieval and lifecycle/resource contracts |
| Method | Read-only inspection of accepted owners, production paths and adjacent consumers; no edits, Cargo execution or probes |
| Exclusions | Retrospective Phase 0–4 audit, foundation recertification, PR6/new tools, ANN, general heap identity, broader R4 admission, product comparisons, retrieval quality, total RSS and performance |

Authorities are the [Phase 5 detailed plan](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md), [DESIGN §15.12](../../design/sections/semantic-model.md#section-15-12), [§11](../../design/sections/synthesis-and-serving.md#section-11) and [ADR-0114](../../adr/0114-generation-serving-contracts.md). The earlier Proposed target review supplies context, not the verdict.

Decisive inspected-file SHA-256 values:

| File | Digest |
|---|---|
| `python/lctx_mcp/src/lctx_mcp/wire.py` | `0b57dd9a9e621ceef0cdf1ed6e2fe9e24451c4016c69773ba8cffc5eeb510833` |
| `crates/lctx-model/src/domain/serving/packets.rs` | `03b3951d9f910654f381283971582cd46fd174ecce8433f2c1b99b9a2978fa2d` |
| `crates/lctx-postgres/src/generations/capability_service.rs` | `3487e84194156dc37400dd77c0794d43e2197f68d1e368ba1a6da17dd83c7c35` |
| `crates/lctx-postgres/src/generations/runtime.rs` | `71b489ab1d82eb99ea1073f5477e4015e236609fda481f3b5ed35331b9394a0d` |
| `python/lctx_storage/src/serving.rs` | `0b34e8e1897b21b367884ac2d95742a9922e261aec0d9855c16b80bfd635a7a3` |

## 2. Responsibilities and domain fidelity

| Owner | Governing responsibility | Consumer boundary |
|---|---|---|
| `lctx-model::domain::serving` | Finite requests, schemas, mappings, availability, ranking and continuation identities | Store, bridge and transport consume declared contracts |
| `domain::selection` | Eligibility, contextual witnesses and conjunction | Complete-domain discovery and ranked nomination reuse one classifier |
| `domain::native_requests` | Exact scalar restriction over admitted paths and proof frames | Returns bounded path/model claims without executing Python |
| `lctx-postgres::generations` | Admission, leases, hydration, physical realization and cleanup | Effects operate against the original generation |
| Rust/PyO3 service bridge | Shared execution grants and charged handoffs | Python callbacks participate in native execution |
| Python/FastMCP | Registration, lifecycle, presentation and numerical scoring | No independent eligibility, fusion or cursor interpretation |

The model captures consequential distinctions: unavailable/partial/not-requested, source/effective signatures, requirement versus ranking witnesses, stored conditions versus request-derived restrictions, and canonical values versus disposable physical vectors.

| Fact family | Fidelity and scope | Identity and consumers |
|---|---|---|
| Catalog and selection | Derived finite catalog; contextual outcomes and completeness remain explicit | Nominal member/context/witness identities; discovery and packets |
| Retrieval | Heuristic relevance over admitted canonical occurrences | Generation, unit, fragment, context, anchor, policy and channel |
| Original evidence | Captured bytes with range, digest, release and context | Canonical artifact/chunk closure |
| Behavioral/native paths | Qualified finite results; exact-input refutation is path-local | Original condition/proof plus distinct restricted-result identity |
| Brief assertions | Authoritative status exists in canonical assertions but is discarded during serving | **F02** affects all capability consumers |

Provider reconstruction is outside this review. Its existing qualification is consumed, not recertified.

## 3. Contracts and lifecycle

`GenerationService::admit` resolves selection once, creates the shared and preparation scopes, retains one guard and prepares classification ([runtime.rs:47 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/runtime.rs#L47)). The guard records terminal loss and never reacquires a replacement session ([guard.rs:40 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/guard.rs#L40)).

A request grant owns CPU capacity, its cumulative deadline and retained reservations. Blocking workers retain the grant after caller cancellation; query tasks retain separate query capacity through lease release ([runtime.rs:125 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/runtime.rs#L125), [runtime.rs:142 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/runtime.rs#L142)). Shutdown stops admission, drains active work and then closes the original guard.

These are meaningful **Implemented** mechanisms. They do not establish instantaneous network-loss detection or measured RSS bounds. F01 identifies transport work that has not yet been incorporated into this ownership contract.

Packet set lookups check nominal relation fields before SQL construction ([mod.rs:872](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/lctx-postgres/src/generations/mod.rs#L872)). Required signatures remain indivisible; optional sections can be removed with explicit continuation and omission metadata ([operation_sections.rs:144 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/operation_sections.rs#L144)).

## 4. Composition and execution

| Stage / question | Universe, method and model | Bounds and linkage |
|---|---|---|
| Which operations meet requirements? | Complete admitted classification domain; existing model classifier | Contextual conjunction and original requirement witnesses |
| Which eligible results are relevant? | Canonical occurrence universe; BM25/exact cosine and model-owned family RRF | Eligibility precedes ranking; ranked extent and winning occurrences |
| What does the operation declare? | Selected nominal metadata and presentation records | Complete core; independently paged optional sections |
| What are the original bytes? | Canonical artifact/chunk stream | Digest verification, exact byte pages and contextual provenance |
| What supports a conclusion? | Model-declared relation-qualified derivations | Receipt-checked streams; bounded depth, nodes and edges |
| What does an exact scalar refute? | Admitted Entry/path frame and checked atom links | Five verdicts, assumptions, work limits and unexamined state |

Ranking policy and numerical contracts are centralized in [ranking.rs:43](../../../crates/lctx-model/src/domain/serving/ranking.rs#L43). Python returns document scores; Rust validates membership and performs fusion. The numerical adapter contains no second classifier or production fusion implementation (retrieval.py:23 (retired source in the recorded review baseline)).

Native assessment checks the resolved owner/formal/path frame and distinguishes refuted paths from compatibility under a may-model ([evaluate.rs:37](../../../crates/lctx-model/src/domain/native_requests/evaluate.rs#L37), [evaluate.rs:162](../../../crates/lctx-model/src/domain/native_requests/evaluate.rs#L162)).

## 5. Change and failure scenarios

| Scenario and change kind | Observed ownership and propagation | Judgment |
|---|---|---|
| Add a supported predicate — domain extension | Model predicate/classifier and finite encoding; loader follows declared inventories | Localized semantic addition; no Python evaluator required |
| Add an optional packet field — representation | Owned DTO/mapping and hydration; schemas derive from declarations | Credible bounded route; F02 shows a missing consequential field |
| Replace numerical BM25 — mechanism substitution | Numerical adapter/settings and admitted score contract | Classification, fusion and generation lifecycle remain independent |
| Change ranking/display policy — policy | Prepared consumer and cursor identities | Separate from canonical source identity |
| Compile a new library release — instance | Fresh canonical generation; original server remains pinned | Earlier answers retain their original generation meaning |
| Cancel SQL/native work — lifecycle | Runtime tasks retain capacity and guard until cleanup/completion | Implemented boundary; actual qualification remains required |
| Oversized transport input — resource failure | Python serialization precedes native byte/resource admission | **F01** |
| Trace a served capability claim — fidelity | Canonical status is loaded but not represented to the consumer | **F02** |

No full analyzer-upgrade walkthrough is required: this scope changes consumers, not provider semantics.

## 6. Independent gates

These are source-based judgments at **Implemented** maturity, not test receipts.

| Gate | Verdict | Evidence / required action |
|---|---|---|
| G1 Authority | pass within scope | Current model owners; derived views/artifact; no inspected legacy semantic fallback |
| G2 Semantic fidelity | **fail** | F02 removes assertion evidence status |
| G3 Validity | pass within inspected contracts | Structural decoding, nominal hydration checks, content receipts and numerical validation |
| G4 Hidden behavior | pass | Startup reads; preparation is explicit; query path executes no analyzed library |
| G5 Consistency and recovery | **fail** | F01 bypasses conversion/execution admission |
| G6 Transformation and reuse | pass within scope | Channel/request-bound cursors, shared codec and canonical artifact checks |
| G7 Truthful capability claims | pass within stated maturity | Implementation, qualification and activation remain distinct |
| G8 Library leverage | pass | Existing libraries supply storage, numerics, schemas and transport mechanisms |
| CI-G1 Fidelity | pass in inspected selection/native/ranking paths | Unknown and may-model results remain distinct from absence or execution |
| CI-G2 Evidence closure | **fail** | F02 hides a claim’s authoritative evidence status |
| CI-G3 Evaluation integrity | pass within inspected boundary | No new reference/gold input or tuning route is introduced |

## 7. Findings and foundations

<a id="F01"></a>

### F01 — Transport codecs run before allocation and execution admission

**Diagnosis: Implemented defect, 2026-10-02.**  
**Principles:** DP-20, FP-05; G5.

`SchemaTool.run` materializes the complete request with `json.dumps(arguments)` before `request_info` reaches native byte checks ([wire.py:57](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L57)). Final envelope construction likewise runs directly in Python; `response_encodings` eagerly produces both serialized byte strings before native admission ([wire.py:15](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L15), [wire.py:71](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L71)). The resource path uses the same pattern.

An oversized request therefore allocates its encoded copy before refusal. Codec CPU work also occurs outside `RequestExecution::cpu`, so its work is not governed by that operation’s deadline/completion boundary. Holding a grant limits admitted requests but does not charge these conversions or make their synchronous execution obey the cumulative deadline. This finding concerns owned allocations and work, not total process RSS.

**Correction: Proposed.** Incorporate generic request encoding and pinned SDK envelope serialization into the original execution grant. Bound/count borrowed JSON values before materialization, reserve codec scratch, and run the numerical/serialization callback under native CPU execution. Keep semantic request decoding in the model; do not introduce Python schema or applicability interpretation.

A legitimate Unicode request must be accepted according to actual UTF-8 bytes, without a new unrelated character/container cap. Final serializers must preserve pinned SDK metadata, actual request IDs and structured content.

**Closure evidence:** inspection of the integrated codec path plus actual transport controls for oversized/Unicode requests, cancellation, cumulative deadline and both final writer encodings. The Phase 5 plan §10 should own current disposition.

<a id="F02"></a>

### F02 — Capability serving discards per-assertion evidence status

**Diagnosis: Implemented fidelity defect, 2026-10-02.**  
**Principles:** FP-04, DP-02/21/22, CI-11; A2, G2, CI-G2.

`ProgrammaticAssertion` owns evidence status, kind, qualification and text ([assertions.rs:96](../../../crates/lctx-model/src/domain/synthesis/assertions.rs#L96)). Capability hydration loads these values, then returns only their IDs ([capability_service.rs:32 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/capability_service.rs#L32)). `CapabilityPacket` has no per-assertion status ([packets.rs:49](../../../crates/lctx-model/src/domain/serving/packets.rs#L49)). Canonical brief rendering prints assertion text without status ([briefs.rs:265](../../../crates/lctx-model/src/domain/synthesis/briefs.rs#L265)).

Consequently, `get_capability`, capability search, operation brief sections and the capability resource serve claims without exposing each claim’s evidence status. `unreviewed` and `documentation_only` express different distinctions and cannot substitute for it.

The diagnosis is status loss, not an assertion that nominal support targets are absent: generated same-generation foreign keys already enforce those references ([ddl.rs:329](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/lctx-postgres/src/generations/ddl.rs#L329)).

**Correction: Proposed.** Add a model-owned assertion representation carrying the canonical ID and evidence status, with the attribution/support needed by its consumer. Hydrate it once and use that same closure for structured packets and resource presentation. Preserve canonical document bytes/digests; presentation metadata can accompany them. Do not infer status from prose or promote it from ranking.

**Closure evidence:** actual mixed-status capability packet/resource controls that compare served status with canonical assertions and retain final byte refusal. Update the wire/presentation invalidation boundary. The Phase 5 plan §10 should own current disposition.

| Foundation | Verdict |
|---|---|
| FP-01 Separation | satisfied within scope |
| FP-02 Stable contracts | satisfied within scope |
| FP-03 Composition | satisfied in the inspected semantic composition |
| FP-04 Domain model and authority | **violated by F02’s served representation** |
| FP-05 Explicit structure and constraints | **violated by F01’s resource boundary** |
| FP-06 Local reasoning | satisfied; corrections have bounded owners and verification routes |

Other applicable supporting rules and CI constraints are satisfied by the inspected mechanisms at Implemented maturity. Upstream extraction/analytics and broader evaluation are not recertified.

## 8. Library fit

Pins were checked against repository declarations and relevant offline capability skills: SQLx 0.9.0, SeaQuery 1.0.2, pgvector crate 0.4.2/extension 0.8.6, PyO3 0.29.2, async runtimes 0.29.0, FastMCP 4.0.5 and BM25S 0.3.11.

Their roles are appropriate. SQLx/SeaQuery handle effects and lowering; BM25S and pgvector supply numerical scoring; Serde/Schemars supply wire mechanisms; PyO3 and FastMCP supply bridge/transport mechanisms. Application-specific eligibility, proof meaning and fusion remain domain code. Library adoption does not remove application responsibilities for grant retention, complete packet fidelity or final byte accounting.

## 9. Alternatives and tradeoffs

Direct request-local reconstruction would reduce preparation machinery but repeat stable classification and native work. Restoring bundle/import/projection authorities would violate the accepted cutover. A universal projection DSL would add machinery without a present consumer.

The selected prepared consumers and narrow vector artifact are justified. Explicit preparation uses canonical values, atomic publication and exact readback; startup validates rather than repairs ([vectors.rs:85 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/vectors.rs#L85), [vectors.rs:114 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/vectors.rs#L114)). F01/F02 can be corrected within this architecture.

## 10. Verification and uncertainty

| Check / evidence | Outcome |
|---|---|
| `git rev-parse HEAD`; source/authority reads and decisive-file hashes | **passed**, establishes the inspected baseline |
| Reviewer Cargo tests or probes | **not_run**, read-only assignment |
| Targeted implementation receipts | Attributed to Phase 5 plan §10; not independently rerun or promoted |
| `just test-all` and `just hygiene` for complete Phase 5 tree | **not_run** at this boundary |
| Both-profile pinned FastMCP native/MCP journeys and activation | **not_run** at this boundary |
| Retrieval quality, latency, hydration, total RSS and performance | **not_run**; no measured claim |

No new probe is necessary to establish either diagnosis: the executing expressions and discarded fields are decisive. Corrections need focused verification, followed by the existing Q0 qualification.

## 11. Disposition and authority routes

F01 belongs to T0/runtime integration; F02 belongs to M0/E0/T0 capability representation. The Phase 5 plan §10 should record their current disposition and closure evidence. Existing cross-phase findings retain the parent cutover §8 owner.

Both proposed corrections preserve ADR-0114’s selected boundaries. No new store, readiness registry, compatibility reader or product tool is required. If the correction changes enduring representation/resource meaning, update its existing architectural owner through the repository decision route.

## 12. Decision

| Architectural judgment | Verdict |
|---|---|
| A1 Localize change | satisfied within the selected scenarios |
| A2 Encode domain meaning explicitly | **violated by F02** |
| A3 Extend through composition | satisfied within the inspected semantic architecture |

**Revise the assembled Phase 5 implementation for F01 and F02.** Preserve the existing generation guard, classifier, ranking owner, current-only native semantics and derived vector boundary.

A bounded follow-up can reassess the corrected transport and capability paths. Closure would permit a new scoped architectural decision; it would not substitute for full Q0 qualification, activation or evidence for excluded product/performance claims.


## Appendix A. Bounded F01/F02 follow-up — 2026-10-02

**Bounded decision: Accept the F01/F02 corrections at Implemented maturity.** The inspected source resolves both architectural defects. Actual transport receipts establish the narrower Tested boundary described below. The direct mixed-status canonical comparison remains **not_run** at this assessment boundary; that missing receipt does not establish a remaining source defect.

This follow-up reassesses only F01 and F02. It preserves the principal review’s scope, standard, exclusions and unaffected judgments. It does not repeat the assembled review, recertify Phase 0–4 or establish Phase 5 qualification.

### Baseline and method

The inspected baseline remains MAIN `0bc8ea11171d6fd96827a8e250964d3de5b3b4ce`, with the corrected uncommitted implementation. The original preservation snapshot remains `bd22f66c71023cf7ca642eea8e82689941e5ac44`.

The reviewer independently read the corrected production paths, relevant controls and cited receipts. No edits, Cargo execution or probes were performed by the reviewer. Decisive production-file hashes were independently compared with `/home/paul/.cache/lctx-phase5-n0`, the copy used for the native transport receipt; they match.

| Corrected file | SHA-256 |
|---|---|
| `python/lctx_storage/src/serving.rs` | `f4d4a2ec33fccdccb360cc4f5bf806d656a8bd7dec01239eb235af5c62878242` |
| `python/lctx_mcp/src/lctx_mcp/wire.py` | `83adf6e8a0a2feac4eed3f26d8c3077e992e23fe194c64ba00c8555d9ae853a5` |
| `crates/lctx-model/src/domain/serving/packets.rs` | `60a4b7d5fc165dfc76b2cf4a7c7ae3caff1ece7a6c87ae976bbf7307b0a361a0` |
| `crates/lctx-postgres/src/generations/capability_service.rs` | `eff11e04716600374dbf41394bbb2ec043178224f245002e3e36db4105727e98` |
| `crates/lctx-model/src/domain/serving/schema.rs` | `a9e61b925b7fdef28f8a7e6fb84e569722625fc063b20fa43a0f93d5aeb893e6` |
| `crates/lctx-model/src/domain/serving/dispatch.rs` | `626e2388048adaad6056944b0d363bf1b179c1c3898410e7137a04eba4695374` |

The runtime source remains unchanged from the principal assessment: SHA-256 `71b489ab1d82eb99ea1073f5477e4015e236609fda481f3b5ed35331b9394a0d`.

### F01 — Codec admission correction

**Source correction accepted: Implemented, 2026-10-02.**

`Service.encode_request` now performs generic borrowed-value preflight inside the original `RequestExecution::cpu` operation. The traversal charges its scratch before retaining children, derives a JSON byte lower bound, rejects active-container cycles and counts repeated acyclic containers separately ([serving.rs:52 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/python/lctx_storage/src/serving.rs#L52)). Exact builtin checks avoid user-defined conversion or length behavior. This machinery interprets JSON shape and size; semantic request decoding remains model-owned.

Before invoking the trusted encoder, the bridge reserves codec scratch from the existing 256 KiB wire bound. It then checks the encoder’s actual UTF-8 output and charges the handoff ([serving.rs:145 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/python/lctx_storage/src/serving.rs#L145)). The string-length preflight is a lower bound, not a replacement byte limit: valid Unicode is not rejected merely for requiring multiple UTF-8 bytes.

Final envelope construction likewise runs under the original CPU operation, with scratch reserved before the callback and both actual writer encodings checked before return ([serving.rs:160 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/python/lctx_storage/src/serving.rs#L160)). Tool and resource paths capture request-context values before entering the worker, then construct their protocol results within that callback ([wire.py:78](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L78), [wire.py:128](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L128)). Negotiated field shaping uses the pinned SDK’s public `serialize_server_result`; server identity and actual request IDs participate in the admitted encodings ([wire.py:17](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L17)).

Capability-resource construction receives its own prior charge: an allocation-free packet count supplies the 32× construction reservation, followed by the actual 256 KiB check and retained handoff ([serving.rs:295 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/python/lctx_storage/src/serving.rs#L295)).

These calls reuse the original cumulative deadline, guard and reservations. The unchanged runtime retains blocking workers after cancellation or timeout until actual completion ([runtime.rs:125 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/runtime.rs#L125)). The correction therefore closes F01’s identified owned-allocation and execution-boundary gap. It makes no total-RSS claim.

### F02 — Assertion fidelity correction

**Source correction accepted: Implemented, 2026-10-02.**

The model now declares `AssertionPacket` with the canonical assertion ID, kind, section, evidence status, qualification and text. Support packets retain their nominal support/source IDs and typed proof references ([packets.rs:49](../../../crates/lctx-model/src/domain/serving/packets.rs#L49)).

Hydration copies those values directly from `ProgrammaticAssertion`. Each support exposes references to the actual support row, source row and the target identified by the canonical source variant ([capability_service.rs:57 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/capability_service.rs#L57)). It does not derive evidence status from prose, ranking or the brief-level flags. Canonical foreign-key guarantees remain the same-generation target-existence boundary.

The existing capability service supplies this representation to direct capability reads, search results and operation brief sections ([retrieval_service.rs:176 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/retrieval_service.rs#L176), [operation_sections.rs:66 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/operation_sections.rs#L66)). Resource presentation consumes that same hydrated packet.

`resource_text` preserves the canonical rendered body as its prefix and appends attributed assertion evidence ([packets.rs:113](../../../crates/lctx-model/src/domain/serving/packets.rs#L113)). Canonical `BriefDocument` bytes and their checked digest remain unchanged ([capability_service.rs:23 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/capability_service.rs#L23)). Numeric schema enums derive from existing `FieldValue::codes()` ([schema.rs:20](../../../crates/lctx-model/src/domain/serving/schema.rs#L20)); presentation evolution participates in wire identity ([dispatch.rs:127](../../../crates/lctx-model/src/domain/serving/dispatch.rs#L127)). No parallel status authority or codebook migration is introduced.

### Verification boundary

The reviewer independently inspected these receipts:

| Evidence | Outcome and limit |
|---|---|
| `/tmp/lctx-phase5-native-resource-final-abi_2026-10-02.log` | **passed**: final native release build, 10.94 seconds; compilation evidence |
| `/tmp/lctx-phase5-native-final-transport_2026-10-02.log` | **passed**: `serving_native::canonical_native_restriction_both_profiles_and_request_binding`, 177.59 seconds; Catalog and Behavioral each passed ten transport controls |

The native fixture invokes `uv run --no-sync pytest -q python/lctx_mcp/tests/current_transport.py` against each still-live compiled generation ([serving_native.rs:120 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx/tests/serving_native.rs#L120)). Its controls include generic preflight refusal and valid Unicode, callback cancellation retention, final-byte refusal, exact stdio/modern HTTP writers, and assertion metadata in capability packets/resources.

Those receipts establish scoped Tested behavior for the corrected codecs and transported assertion presentation. They do not establish every possible assertion-status combination. The direct canonical mixed-status comparison ([serving_packets.rs:69 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx/tests/serving_packets.rs#L69)) and new pure numeric-code/resource control ([serving_contracts.rs:272](../../../crates/lctx-model/tests/serving_contracts.rs#L272)) remain **not_run** here.

Full Q0 gates and both-profile pinned FastMCP qualification remain pending. No measured quality, RSS or performance conclusion follows.

### Affected judgments and disposition

| Judgment | Follow-up verdict |
|---|---|
| A1 — Localize change | satisfied within these corrections |
| A2 — Encode domain meaning explicitly | satisfied; F02’s authoritative values survive serving |
| A3 — Compose through contracts | satisfied; existing model/runtime/SDK boundaries are retained |
| G2 — Semantic fidelity | pass at Implemented maturity for F02 |
| G5 — Consistency and recovery | pass within F01’s corrected codec boundary |
| CI-G2 — Evidence closure | pass at Implemented maturity for F02 |
| FP-04 / FP-05 | satisfied within the corrected representation/resource boundaries |

No new material F01/F02 defect was found. Other principal-review judgments remain unchanged rather than independently reassessed.

The original **Revise** assessment remains a valid dated diagnosis of its baseline. This follow-up accepts its two corrections and removes their identified architectural blockers. Current execution disposition remains owned by [Phase 5 plan §10](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md). M0/E0/T0 own the remaining direct mixed-status receipt; the coordinator owns Q0 qualification and activation.


## Appendix B. F02 direct verification — 2026-10-02

**F02 is closed within the scoped Tested boundary.** The reviewer independently inspected the completed receipts and exact controls. This supersedes Appendix A’s **not_run** status for the direct mixed-status comparison and pure assertion-schema control. The bounded acceptance decision remains unchanged.

MAIN remains `0bc8ea11171d6fd96827a8e250964d3de5b3b4ce` plus uncommitted implementation. The decisive production-file hashes match Appendix A. The direct test file, `crates/lctx/tests/serving_packets.rs`, has SHA-256 `dfeb3e169f9aa77694cbb1060ce0b5692b445016090cb6d87f0bbb2dc7cddee1`. The reviewer performed no edits, builds or probes.

The actual Catalog-generation control `serving_packets::large_optional_brief_keeps_the_complete_core_and_resumes_with_expanded_budget` **passed**, taking 73.223 seconds, in `/tmp/lctx-phase5-serving-focused-original-artifact_2026-10-02.log`.

Its assertions are nonvacuous: the capability must contain both `Documented` and `StructurallyObserved` claims. Each served claim’s kind, section, status, qualification and text is compared directly with its canonical `ProgrammaticAssertion` row. Supports must be present with three proof references. The operation brief packet must equal the standalone capability packet; resource presentation must preserve the canonical rendered prefix and expose each assertion’s identity and status label ([serving_packets.rs:68 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx/tests/serving_packets.rs#L68)).

The pure model command also **passed** all 32 tests across two binaries:

```sh
INSTA_UPDATE=no python3 scripts/build_environment.py -- cargo nextest run --release -p lctx-model --test serving_contracts --test ranking_policy
```

Receipt: `/tmp/lctx-phase5-model-final-corrected_2026-10-02.log`. This includes the assertion control that accepts canonical numeric codes, rejects unknown status, preserves authored bytes and renders both evidence-status distinctions ([serving_contracts.rs:272](../../../crates/lctx-model/tests/serving_contracts.rs#L272)). The original missing-development-dependency compile failure remains recorded separately.

Together with Appendix A’s source assessment and transported packet/resource controls, these results establish F02’s required direct canonical comparison. A2, G2 and CI-G2 now have scoped Tested support for this correction. F01’s assessment is unchanged.

The enclosing focused run **failed overall**: nine tests passed, one failed and one timed out. This appendix does not reassess those other results or promote that run to a passing gate. Full Q0 qualification and pinned FastMCP pilots remain pending. Current finding disposition remains owned by [Phase 5 plan §10](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md).

## Appendix C. Final Q0 assembled target assessment — 2026-10-02

**Decision: Accept scoped, at Implemented maturity with the bounded Tested evidence below.** The current assembled Phase 5 design retains one semantic owner and one generation-bound effect/runtime owner. F01 and F02 remain corrected. No new material source defect was found in the inspected assembly. This decision does **not** establish Q0 completion, passing final-source FastMCP journeys or operator activation.

### Baseline and review boundary

This is the scheduled final assembled Design/Target assessment under the principal review's core 3.2, code-intelligence 1.3 and repository binding. The baseline is MAIN `8e10d256cf1f742dad18673f2257a7b894715821` plus the preserved initial dirty source tree. Apparent formatting is not treated as new semantic scope. The current model identity is `dac961ba186c11b01144120a749e0be4d3c7aec8fe7bd908a052850955fbf025`, recorded in the accepted model-description snapshot and current plan.

The scope remains the ten retained tools, capability resource, canonical Catalog admission, classification, retrieval, optional packets/native enrichment, original evidence and service resource/lifecycle boundaries. The assessment includes the normalization optimization only where its identity and preparation behavior affect this assembly. It is not a retrospective Phase 0–4 review or foundation recertification. PR6, ANN, broader native admission, live embedding qualification, product-quality comparisons, total RSS and performance measurement remain outside this decision.

The reviewer inspected current owners, production paths, integration changes and the cited receipts. No builds, tests, probes, formatters or generators were launched. This appendix is the only reviewer edit; the root retains acceptance, plan and disposition ownership.

Decisive current SHA-256 values:

| File | SHA-256 |
|---|---|
| `python/lctx_storage/src/serving.rs` | `3e1341b8656fa87b3606e4c970d775ab4e3728d7c2e216b0e9823aaeecf7d2f3` |
| `python/lctx_mcp/src/lctx_mcp/wire.py` | `587fd2f917c7c851ebbd4c5ca469a4aa61dbd175b216ae1be09c2a8679c51c19` |
| `crates/lctx-model/src/domain/serving/packets.rs` | `d07c8fc39b0dcfbdb16d9e7ee4d0cedd1c8a86dfdbeba9f4ba06e4dc74b61f58` |
| `crates/lctx-postgres/src/generations/capability_service.rs` | `a2cef54dee86a81ae9047cd0eb01dc5caa61ce028ff1600f02113aac8dc5717d` |
| `crates/lctx-postgres/src/generations/runtime.rs` | `f7281e629370529f4055f5cfbde111ed39d408fbef36d63ae037c8713bb94cdb` |
| `crates/lctx-postgres/src/generations/selection.rs` | `12aba9bc42d4fbbe80063e99a8430b76358d1728e1428a2d69c17cdabae249ed` |
| `crates/lctx-postgres/src/generations/native_service.rs` | `f58b79c4a45b96c0fdf38db7a94d35aeb8429ded285c5a56cc07a588a8c2bd58` |
| `crates/lctx-model/src/domain/identity.rs` | `005eb77f424f5a202662caea5100f94eda180b325f40d65bbee6503c01d59728` |

### Assembled ownership, fidelity and composition

The model continues to own request/schema contracts, classifications, ranking, cursors and finite native assessment. `ServingService` composes catalog, retrieval and native preparation over one admitted runtime, and shuts it down if preparation fails ([service.rs:20 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/service.rs#L20)). Views remain generated from declared mappings rather than a copied semantic store ([serving_shape.rs:19 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/serving_shape.rs#L19)). The disposable vector artifact remains a separate derived effect with explicit preparation; it is not a source of semantic authority.

Python still receives model-produced lexical tokens/document identities and supplies pinned BM25S numerical scores (retrieval.py:24 (retired source in the recorded review baseline)). Rust retains eligibility, grouping, fusion and continuation decisions. Capability search hydrates the same capability service used by direct reads and optional brief packets ([retrieval_service.rs:811 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/retrieval_service.rs#L811)). Unknown, unavailable and NotRequested remain explicit distinctions rather than inferred negative behavior.

F01's correction survives integration: borrowed generic preflight and trusted request/envelope callbacks execute under the original charged CPU grant ([serving.rs:429 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/python/lctx_storage/src/serving.rs#L429), [serving.rs:484 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/python/lctx_storage/src/serving.rs#L484)). Resource construction reserves before materialization and checks actual output bytes ([serving.rs:974 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/python/lctx_storage/src/serving.rs#L974)). The Python bridge captures the actual request context before worker execution and uses public SDK result serialization ([wire.py:96](../../../python/lctx_mcp/src/lctx_mcp/wire.py#L96)).

F02's correction also survives: canonical status, kind, section, qualification and text are copied into model-owned assertion packets with actual support/source/target references ([capability_service.rs:218 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/capability_service.rs#L218)). Resource presentation preserves authored bytes and appends that packet's evidence metadata ([packets.rs:240](../../../crates/lctx-model/src/domain/serving/packets.rs#L240)); its presentation revision participates in wire identity ([dispatch.rs:181](../../../crates/lctx-model/src/domain/serving/dispatch.rs#L181)). Appendix B's direct mixed-status evidence remains scoped evidence, not an assertion that the current whole gate passed.

The original guard and shared execution own cancellation, deadlines and memory. Defaults remain 256 MiB shared, 128 MiB preparation and 64 MiB per request, with two CPU grants, two query connections and a cumulative 30-second deadline ([resources.rs:25](../../../crates/lctx-model/src/domain/serving/resources.rs#L25)). Blocking work and query cleanup retain their execution ownership after caller cancellation ([runtime.rs:228 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/runtime.rs#L228), [runtime.rs:250 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/runtime.rs#L250)).

The ordering repair checks content receipts in ID order and streams declared validator grouping order inside one read-only repeatable-read transaction on the original connection ([selection.rs:206](https://github.com/paul-heyse/library-context/blob/0bc8ea11171d6fd96827a8e250964d3de5b3b4ce/crates/lctx-postgres/src/generations/selection.rs#L206)). Native preparation uses those reads for actual model-owned invariant checks, including their vocabulary epochs ([native_service.rs:179 (pre-pivot source)](https://github.com/paul-heyse/library-context/blob/6f1a7e98ebad029e7e38876cede4d7726a33eb4d/crates/lctx-postgres/src/generations/native_service.rs#L179)). It does not replace proof authority with receipt presence.

The normalization caches remain invocation-local execution mechanisms. Shared scalar framing preserves the old digest stream; `ClassInventory` preserves macro/ID ordering and owns its allocation reservation ([identity.rs:185](../../../crates/lctx-model/src/domain/identity.rs#L185), [symbolic_fields.rs:398](../../../crates/lctx-model/src/domain/normalized/symbolic_fields.rs#L398)). `TargetSpanIndex` retains duplicates, original ID order, missing targets and first-target-before-expression checking ([callable_aspects.rs:257](../../../crates/lctx-model/src/domain/normalized/callable_aspects.rs#L257)). No persisted authority, semantic codebook change or performance claim follows.

### Judgments and evidence strength

| Judgment | Current assessment |
|---|---|
| A1 Localize change | satisfied: route semantics, storage effects and codec policy have bounded owners |
| A2 Encode domain meaning explicitly | satisfied: canonical distinctions and assertion evidence survive serving |
| A3 Compose through contracts | satisfied: model operations, generation effects and numerical callbacks compose without a parallel semantic interpreter |
| G1 Authority | pass within scope: model declarations and original generation remain authoritative |
| G2 Semantic fidelity | pass at Implemented maturity; F02 has the scoped direct Tested evidence in Appendix B |
| G3 Validity | pass for inspected mechanisms: closed decoding, canonical hydration, receipts and numerical validation; final qualification pending |
| G4 Hidden behavior | pass: explicit preparation, read-only startup and no analyzed-library execution on the query path |
| G5 Consistency and recovery | pass for inspected ownership: admitted codecs, original guard, snapshot reads and cancellation-safe cleanup |
| G6 Transformation and reuse | pass: shared framing/codecs, canonical artifact identity and bound continuations |
| G7 Truthful capability claims | pass within the owners' stated Implemented/scoped-Tested boundary; Q0 and activation remain pending |
| G8 Library leverage | pass: SQLx/PostgreSQL, BLAKE3, schema libraries, BM25S/NumPy and FastMCP/SDK retain their mechanical responsibilities |
| CI-G1 Fidelity | pass within selection/native/ranking scope; no promotion of may-model results to execution facts |
| CI-G2 Evidence closure | pass within inspected representations and the recorded F02 controls |
| CI-G3 Evaluation integrity | pass within scope; no quality result or new reference/gold authority is introduced |

FP-01–FP-06 are satisfied within this assembly. The cache/index additions and snapshot repair retain existing owners and contracts; no alternative architecture, new dependency or ADR is warranted by these internal corrections. Retaining the old copied schema or moving semantic interpretation into Python would reintroduce duplicate authority without resolving the present target.

Independently inspected receipts include the native ordering control (1 passed, 200.266s; `/tmp/lctx-phase5-native-ordering_2026-10-02.log`), original generated CPython oracle (1 passed, 795.987s; `/tmp/lctx-phase5-soundness-ordering_2026-10-02.log`) and corrected canonical evidence control (1 passed, 92.778s; `/tmp/lctx-phase5-evidence-parent-corrected_2026-10-02.log`). Their commands and retained failures are recorded in [plan §10](../../plans/semantic-model-phase5-detailed-plan_2026-10-01.md). These precede the final automatic source-formatting/model transition and are not relabelled as a clean final-tree gate.

The current `lctx-model` repair run passed 51 selected controls, including the final index allocation envelope; 90 tests were outside its filter (`/tmp/lctx-phase5-final-model-repair-controls_2026-10-02.log`). Hygiene is a composite receipt: `/tmp/lctx-phase5-hygiene-final_2026-10-02.log` passed source/policy checks including all-target Clippy but failed its obsolete-store inspection; after the authorized reset, named `just store-check` passed with zero generations/zero findings (`/tmp/lctx-phase5-hygiene-store-final_2026-10-02.log`). It is not an initially clean hygiene run.

At this assessment boundary, `/tmp/lctx-phase5-test-all-final_2026-10-02.log` was still running and established no complete final functional result. Final-source both-profile pinned FastMCP journeys likewise had no passing receipt. Their completion is required for Q0/product acceptance and activation, including evidence that the actual library can be admitted under the declared preparation limits. A failure there must be diagnosed on its own facts; this source judgment does not pre-accept it.

### Disposition and enclosing status

No new material finding is opened. F01/F02 retain their stable identities and the correction/closure evidence in Appendices A/B. The accepted authority remains ADR-0114 and the current semantic-model/synthesis-serving owners; legacy wording in operator documentation is a documentation reconciliation obligation, not evidence that the removed runtime architecture survives. Root owns those existing-document updates.

The assembled architecture is **Accept scoped** within the finite, generation-bound serving contracts at the stated evidence strength. The enclosing Phase 5 qualification and operator activation remain **pending**. Root owns the final gate/journey assessment and existing plan/STATUS updates; this appendix creates no competing acceptance or disposition ledger.
