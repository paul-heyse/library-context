# Native graph retrieval, selection and evidence serving

**Persisted follow-up implementation paused, 2026-10-07:** the [persisted graph execution plan](persisted-graph-execution-plan_2026-10-07.md) owns the current combined execution, catalog-speed F01–F05 and additional same-pattern corrections. PG2/PG6/PG8 consolidate typed selected access, byte ranges and bounded final encoding. Existing indexed search, ranking, evidence meaning and pinned journeys remain. Do not create another native reader/cache or turn packet-local scans into scalar RPCs. Original dated receipts and prior audit findings retain their scope; only that new plan owns these follow-up findings.

The [supporting correction plan](persisted-execution-corrections-plan_2026-10-07.md) supplies the Proposed continuation: PC3/PC4 migrate published scope preparation, member selection and capability eligibility while preserving ranking and realization pins. [Persisted coordinator §8](persisted-graph-execution-plan_2026-10-07.md#8-sole-finding-disposition) owns the correction-causes findings; this document adds no second disposition. Plan authoring does not resume production.

**ER production scopes Implemented / focused Tested, 2026-10-07:** this plan owns the serving contracts below. [Graph coordinator §7](graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities) remains the sole graph-audit disposition owner; its [§8.1 checkpoint](graph-native-pivot-plan_2026-10-05.md#81-current-remediation-acceptance--2026-10-07) and [combined coordinator §6](evidence-retrieval-and-evaluation-plan_2026-10-06.md#6-finding-disposition-investigation-outcomes-and-completion) own current receipts and completion. Earlier dated receipts retain their original source and scope; Q1 remains pending.

**Implemented / focused Tested, 2026-10-06; operator adoption not_run.** Supporting plan for S1/S2/S3 in the
[replacement coordinator](graph-native-pivot-plan_2026-10-05.md). It consumes one admitted
graph and its [sealed SurrealDB realization](graph-native-surrealdb-realization-plan_2026-10-05.md).
The coordinator owns current state/disposition. Native querying is a selected part of the target,
not a capability awaiting a benchmark, accounting proof or adoption verdict.

**Current continuation Implemented / focused Tested, 2026-10-07:** §6 develops executable identity and consumer corrections.
Prior implementation/acceptance labels below describe the initial pivot, not closure of the audit.

The [2026-10-06 implementation audit](../design_review/reviews/design_review_graph-native-pivot-implementation-audit_2026-10-06.md) identifies remaining coordinated-plan gaps;
[coordinator §7](graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities)
owns their current disposition. Earlier stage acceptance and dated receipts below are preserved;
this document's implementation label does not establish closure of the audit findings.

## 1. Foundation assessment and operation ownership

The retired `GenerationReader::prepare_selection` loaded 95 classification inputs, followed by
output and admission inputs; selection enumerated all public exposures before library filtering.
Native service now restricts contextual members at indexed roots, hydrates scoped stored facts,
and uses the retained exact kernels only where required. Packets, original evidence and authored
resources share one immutable snapshot. These execution choices are design judgments, not
Measured performance claims; the coordinator records actual focused functional outcomes.

Keep model-owned request meaning, selection/classification algebra, ranking/contribution rules,
mandatory packet assembly, value rendering and precise uncertainty. Recompose their execution:
native functions/queries perform indexed restriction, supported classification, search and connected
hydration near the data. Complex portable kernels can remain Rust over coarsely retrieved inputs.
Do not retain a whole-store startup copy, private PostgreSQL repositories or Python search engine
as a parallel authority. Native semantic enrichment loads only the needed completed domain, lazily.

One operation consumes a pinned realization, normalized request and explicit scope/settings, and
returns a typed response with classifications, original evidence/witness references and truthful
availability/continuation. Its contract belongs to the Rust semantic owner even when its only
implementation is an owned SurrealQL function. Mechanical schemas/parameters derive from that
contract; arbitrary Rust transpilation and a general query language are unnecessary.

## 2. Selection and native search — S1/S2

### 2.1 Preserve predicate-specific meaning without broad preparation

The current `Requirement` is `(Predicate, Quantifier)`. `predicate.domain()` chooses closure over
PublicExposures, SignatureVariants, ConfigurationFields, Relationships, Scenarios, SourceArtifacts
or ReleaseDeclarations. Closure, applicable contexts, support/negative/conflict/unknown and
admissibility are meaningful distinctions; merely matching a property cannot redefine them.

Begin requests at indexed release/input/public-exposure roots before loading/classifying evidence.
Compile compact typed context/assessment/closure summaries where their semantic owner already
knows the complete basis. Use native links/indexes to obtain the requested predicate-domain facts,
not all classification relations. Share coarse preparation within a request and immutable compact
data across requests when justified. Preserve original support references alongside derived fields.

| Predicate family | Efficient route and semantic limit |
|---|---|
| Public path/module/release | Exact indexed scalar/context lookup; member/source/release contexts remain distinct |
| Member kind/invocation/owner | Derived descriptor/consensus with source/public-owner links; ambiguity is unknown, not a chosen convenient kind |
| Parameter/signature/default/type | Indexed variant/slot/name/role facts plus complete signature assessment. Missing name is false only with complete known membership; complex type matching/specialization stays the authoritative kernel when simpler. |
| Configuration/relationship | Typed scope/field/kind/value and target evidence; retain qualifiers, option owner, role and consensus |
| Scenario/deployment/source | Association basis, original context and actual check status. Blocked/NotRun/unresolved targets cannot establish support or absence. |
| Native structural facets | Exact attributed metadata/decorator/type observations and coverage. Existing Rust operations remain usable; heuristic/native-navigation data is not semantic truth. |

For finite supported predicates, lower the operation to native query/function semantics or compile
their exact reusable assessment facts. If a complex classifier remains Rust, obtain its complete
relevant inputs in a batch and run the same owner once. Avoid both independently authored SQL
semantics and a blanket rule that all classification must run in the host. Indexable necessary
restrictions may narrow work but never erase unresolved/conflicting results that the selected
mode must include.

Joint applicability retains variant/binding/configuration witnesses. Two individually supported
requirements on incompatible overloads cannot become a supported combined invocation. Keep
independent support and same-context joint support explicit. Parameter type unions, specialized
types, missing defaults and unknown adjustment keep their existing fidelity/closure meaning.

Exhaustive `find_operations` enumerates the ordered eligible public universe and classifies the
required domains. Use native set-based grouping/joins or streamed batches where suitable. A full
answer may necessarily examine that universe; do not pretend an output LIMIT makes it smaller.
Continue by pinned key/context rather than repeatedly recomputing offset prefixes. Exact totals
are supplied by completed enumeration/certified domain counts when available; a stopped traversal
reports an incomplete domain and unknown remainder instead of preserving an unsupported total.

### 2.2 Search representation and ranking

Start with shared canonical vector records linked to contextual retrieval occurrences and units.
Index exact symbols, identifier/prose text, complete spec and useful release/context fields.
Use native BM25/full-text and HNSW for declared approximate discovery. The analyzer/ranking policy
is a deliberate new realization, not bitwise parity with the old Python BM25 implementation.
Exact-neighbor analytics remain their separate native Rust contract.

Apply eligibility **before channel limits**. A shared vector qualifies through an eligible
occurrence; its winning witness must come from that subset. Native KNN supports residual predicates
during candidate admission, not only after top-k. Supported scalar/full-text restrictions may use
exact prefilter bitmaps; graph eligibility can remain an in-traversal residual. That is a legitimate
native route, not an automatic reason to enumerate in the host or duplicate every ANN vector.

Obtain eligible context membership through the authoritative selection operation first when its
complex kernel cannot execute natively. Pass membership to native search through an operation-sized
bound input or a private request-scoped realization if genuinely necessary, never a second canonical
store. Prefer compact native eligibility summaries/predicates over repeatedly shipping a broad ID
universe. Remove any temporary request state at completion/cancellation. For ordinary supported
predicates, selection and channel retrieval compose in one coarse function/query.

Retrieve eligible occurrence/member/family/channel witnesses together with candidates. Retain
exact-symbol priority, one best contribution per family/member and equal-weight family-normalized
RRF K60, with canonical ties. Native BM25 and declared approximate vector nomination deliberately
change their channel realization, not these contribution rules. Witness inventory is separate
from the score. Final member limit follows collapse
and fusion. Default discovery promises **ranking over bounded candidates**, not exact global
member-top-k. A fixed overfetch factor cannot prove exact member ranking or exhaustive eligibility.
Native filtered ANN remains explicitly approximate.
Eligibility checks ask for one indexed qualifying occurrence with `LIMIT 1`; they do not
materialize every matching role arc just to establish existence. Request input keys are prepared
once and reused by candidate selection and witness hydration.

Use the retained small authoritative Rust fold initially for contribution/family fusion where
its semantics differ from native RRF. This does not prevent native channel retrieval/hydration.
Native RRF is a valid alternative for the same contract after pre-collapse, including the entire
bounded candidate union, canonical final ordering and separate witnesses. Raw RRF duplicate votes,
score-only cutoff ties and merged fields cannot define member semantics. Conditional native linear
fusion is a policy change, not an incidental implementation swap.

Missing index, incompatible spec, permissions/availability failure, deadline/cancellation and
partial coverage are not empty-complete results. Unknown/contradicted/conflicting classification
does not become false merely because search found no candidate.

### 2.3 Preserve embedding inputs and avoid incidental duplication

Keep the exact [Qwen spec](../../specs/embedding/qwen3-embedding-8b.json): model/tokenizer/service
identity, query instruction/document rendering, 4096→1024 MRL admission, float32 L2-normalized
values and 2048-document-token admission. Store consumed canonical bytes/digest/spec; the numeric
search array is derived. No request-time document embedding/enrichment or remote work in a store
transaction. Query-vector creation is an explicit admitted effect outside the query transaction.

If a real query-shape uncertainty matters, inspect native plans and relevant footprint before
choosing a narrower optimization. Derived scalar scope mappings, targeted occurrence search rows
or DISKANN can improve a demonstrated access/residency problem. Duplicated occurrence HNSW vectors
consume additional graph/vector memory; lack of graph-bitmap prefilter alone does not justify that
cost. Choose one active realization, not permanent parallel retrieval backends. No mandatory ANN
bakeoff or tuning against gold/heldout accompanies this pivot.

## 3. Native query design and resource controls

**User-directed design constraint:** use judgment, first principles and library understanding to
design efficient operations. FP-07/A4 do not impose detailed accounting, proofs, required plan
captures or a test of whether SurrealDB querying should be included. The native query path is
selected. Functional controls below test actual outcomes and failure meanings.

Compose fixed paths using native graph operations, record projections, indexed predicates,
subqueries/arrays and functions. Supported native ordered LIMIT shapes push caps into graph scans,
including inline/folded adjacency. Upstream [ordered-limit tests](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/kvs/tests/graph_ordered_limit_test.rs)
exercise these cases. Use suitable limits/fields rather than unrestricted FETCH/all-walk paths.
Variable explanations retain a visited assertion/node frontier only because cycle/continuation
semantics need it; each expansion can still execute as a batched native operation. Do not move
fixed hydration or all candidate enumeration into Rust solely to count engine steps.

For continuation-heavy high-degree pages, directional compound indexes with leading source/role
equalities and trailing canonical assertion-key range are useful. Indexed order/limits avoid
repeated offset prefixes. They are a selected query technique, not a universal mandated table
layout or requirement to inspect every index entry. Keep actual required whole-domain work with
appropriate bulk/native execution rather than hiding it behind shallow-output claims.

Native deadlines/cancellation, resource/cache configuration, bounded request/concurrency/buffer
parameters and response-byte limits complement these physical choices. Disclose partial execution
and a stop reason. Do not expose a fictitious exact engine-work ceiling or equate timeout with
absence. Native guards use different resource scopes; practical operation policy need not reduce
them to one global step counter.

The investigated [EXPLAIN interface](https://surrealdb.com/docs/reference/query-language/statements/explain)
shows operators/access paths. `EXPLAIN ANALYZE FORMAT JSON` executes the operation and reports
native graph edges scanned, ANN candidate records fetched, output rows/batches/time, cache/property
and bitmap-fallback information. The pinned [metrics definitions](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/exec/metrics.rs)
and [explain implementation](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/exec/operators/explain.rs)
were inspected, 2026-10-05. These are useful optional diagnostics for an uncertain or unexpectedly
costly native shape. They are not mandatory receipts or a stable product wire schema. Inclusive
times and different operator row counts cannot simply be summed into total work. Ordinary SDK
statement statistics expose execution time, not every operator counter. Do not execute ANALYZE
and repeat the query on every request merely for telemetry.

## 4. Packets, evidence and transport — S2/S3

The native operation hydrates mandatory identity/release, import/access provenance, signatures,
invocation variants, options and original references as an indivisible core packet. Preserve
singleton/class distinction and exact defaults/factory/Unknown rendering. Optional source/brief/
relationship/behavioral expansion can continue separately. If required content cannot be returned,
report refusal/unavailability rather than a signature that appears complete but is truncated.

Keep current product request/response limits unless a deliberate owner change is needed: 16
requirements, one to five comparison members, 20 default/100 maximum browse/evidence rows,
32 KiB default/256 KiB expanded final response. Those are product limits, not throughput claims.
Optional omissions and continuations are explicit. A deeper source/premise expansion resolves
only the same realization's original assertion/evidence/chunks. Stop at cycles/depth/resource
limits with an incomplete frontier; bounded witnesses do not establish unrestricted absence.

Preserve `search_operations`, `find_operations`, `get_operation`, `browse_library`, `get_evidence`,
`search_evidence`, `compare_operations`, `search_capabilities`, `get_capability` and
`inspect_value_paths`. Migrate all route implementations, resources and native adapters together.
Capabilities/value paths remain optional with their exact supported semantic inputs, not broad
startup-loaded inventories. Lazy domain preparation can be shared within the pin; its cache key
includes actual graph/operation/spec dependencies, and corruption cannot trigger a different backend.

Generated Rust schemas and bounded decoding remain the wire authority. The FastMCP adapter owns
lifespan/stdio or selected transport, listing/invocation, total deadlines and cancellation. Native
workers/client contexts remain owned until cancellation drains; fresh request state cannot switch
the pinned database. Replace PG generation handles with the complete snapshot/realization token
in structured output, resource metadata, cache keys and opaque cursors. Cursors also bind member,
section, representation, ordering/scope and request-policy meaning; incompatible reuse refuses.

Ordinary `DEFINE FUNCTION` / SDK invocation is the native application surface. Custom `DEFINE API`
and generated GraphQL wait for a real HTTP/UI consumer. If API adoption is triggered, endpoint
authorization and explicit snapshot scope must stand on their own: API execution disables ordinary
record-permission checks, and an HTTP error status alone does not roll back preceding writes.
Use actual execution failure for transactional rollback. Do not add a new frontend during this pivot.

Consider one optional portable Rust/WASM kernel only if it removes real transfer/glue: bounded
condition operations, vector/spec validation or deterministic rendering are candidates. Reuse the
same owned source, batch host calls and check WASI/ABI, state independence, strict timeout and
memory. No mandatory full-model WASM port, native-analyzer port or module framework is needed.
An unselected module remains deferred with its consumer trigger.

**Current selection, 2026-10-05:** retain the existing condition, selection and rendering kernels
in Rust over scoped, batched native inputs. Native functions perform restrictions and adjacency
where they avoid transfer; adding a module ABI for these retained kernels would add another
lowering and deployment surface without an identified transfer reduction. Reconsider one bounded
shared-source module only when a concrete operation needs substantial repeated host transfer.

## 5. Packages and targeted functional acceptance

| Package | Integrated implementation and revealing cases |
|---|---|
| S1 | Operation/context/closure contracts and layout together. Unknown input vs admitted empty; requested predicate-domain closure; same-name release ambiguity; incompatible overloads vs independent requirement support. |
| S2 | Native exact/lexical/vector candidate retrieval, selection composition, fusion and evidence. Exact-name priority over stronger prose; eligible occurrence of a shared vector; duplicated passages; tied member cutoff; conflict/unknown retention; missing index/spec; originals and n-ary roles. |
| S3 | All packets, browse/compare/evidence/capability/value paths, cursors/resources, CLI/native/Python bridge. Mandatory packet overflow/refusal; optional continuation; foreign/stale realization; deadline/cancel/drain; actual MCP listing/calls. |

Use small independently authored graph cases with known outcomes through actual native functions
and the persistent server where effects matter. Keep the existing complex semantic kernel controls;
new native semantic implementations need independent known answers for the distinctions they own.
Do not require plan snapshots/counters for each test, exact old ranking parity after an intentional
BM25 change, or a latency campaign to adopt native querying. Use EXPLAIN selectively to settle an
actual shape issue, and ordinary telemetry when diagnosing runtime behavior.

During implementation run touched-crate compile checks and selected `verify-serving`,
`verify-model` or `verify-oracles` controls. Q0 exercises representative real native/store/MCP
journeys on the assembled replacement. Q1 owns separately authorized real FastMCP/live-vector
adoption; task usefulness, confirmation and comparative superiority remain distinct product work.
The tables describe intended coverage. Actual focused execution outcomes, including their Tested
boundaries and omissions, are recorded in coordinator §8; no Measured claim follows.

## 6. Audit remediation: executable identity and composed consumers

**Implemented / focused Tested, 2026-10-07.** This continuation is grounded in audit F06/F08/F09/F10 and the
[capability investigation](../design_review/evidence/2026-10-06_graph-native-remediation-capabilities/README.md).
The [coordinator §9](graph-native-pivot-plan_2026-10-05.md#9-remediation-and-improvement-execution)
owns sequence and completion; §7 owns finding state. Existing §1–§5 contracts remain, with the
specific corrections below taking precedence over an implementation label. No producer, query
adoption or legacy ranking-parity gate is added.

**Acceptance checkpoint:** The earlier 2026-10-06 five-control serving/identity receipt remains bounded to its source before the native text-schema correction. Current acceptance uses fresh native fixtures and a matching CLI/development extension; the checkpoint owners linked above record the actual native and in-process/stdio/error/cancellation outcomes. Wheel packaging remains waived during development. Findings and Q1 adoption remain separately owned rather than implied by implementation or a focused journey.

<a id="R-I1"></a>

### 6.1 R-I1 — Complete executable dependency capture

Current `lctx-serving::operation_definition` manually lists model helper files and omits
`domain/serving/packets.rs`, although `core.rs` consumes its default-value interpretation.
Replace that knowledge with the existing `lctx-model::domain::implementation_digest()` and an
automatically captured, sorted serving-source membership digest. Reuse the model build-script
pattern: frame relative filenames and raw bytes, include additions/deletions, relevant manifests,
resolved dependencies and build feature configuration. Include generated wire/mapping and native
operation policy identities as today. Do not merely add the one missing filename to another
manual list. Keep query/schema/analyzer/module/engine identities in the realization owner.

This conservative executable scope may re-key after comments or unrelated model edits; that is
acceptable here and does not redefine semantic compatibility, logical IDs or graph content.
Do not normalize source bytes or replace identity with handwritten revision bumps. The linked
service, publisher-installed operation function and local Python extension must agree. Rebuild
fresh realizations and the development extension. Wheel packaging is not required during development; no fallback acceptance of older executable identities.

Affected owners are `lctx-serving` source capture/definition and service startup, model build
capture, publisher sealing/restoration, CLI operation-definition installation and the linked PyO3
session. An operation helper extension is captured by membership automatically; an engine/layout
change remains in native realization identity. Preserve those independent scopes.

Closure: a schema-preserving change to default interpretation in the previously omitted helper
changes the executable guard while semantic contract and wire schema stay unchanged. A newly
built service refuses the old sealed realization; a fresh one works. An independently specified
source-membership control includes an added helper and deletion. Run this as a small isolated
build/control, not the stopped compiler suite or source-format parity campaign.

### 6.2 R-S1 — One scoped member set for every browse view

`operations::browse` currently applies scope to ordinary entries, then independently iterates
all selected eligible candidates for vocabulary. Prepare one set of eligible member/context
candidates narrowed by the owned library/module/class ownership policy. All entry views,
vocabulary classification and member counts consume it. Count unique members as the public
contract requires; do not count supporting contexts as extra members. Retain unknown ownership
and selector-domain outcomes rather than silently treating them as empty successful ownership.

Reuse prepared selection and ownership indexes for the request. Avoid rebuilding the same class
path/exposure joins for each member; batch the necessary relationships, preserving ambiguity.
An invalid/foreign scope is checked against the pinned library, not authorized because its key
has valid syntax. Vocabulary availability and its result page remain independently represented.

Actual consumer scope: every `BrowseView`, library/module/class scope, Rust dispatch, CLI and
MCP `browse_library`. Other selector/ranking policies remain owned by the model. A disjoint
two-module/two-class fixture specifies expected predicates and unique counts independently.
Exercise native whole-library, each module/class, selected subset, unknown ownership and an
empty supported scope; foreign predicates must disappear only from the narrowed answers.

### 6.3 R-S2 — Address the owning scenario in a nested continuation

Choose an explicitly addressed nested continuation: its position carries the scenario
association key and last diagnostic key. Keep the complete snapshot, request, policy, wire,
channel, member, ordering, group and section binding. Decode/check the association as belonging
to the requested member and admitted scenario inventory. When resumed, return that owning
scenario with its remaining diagnostic page; do not independently restart and truncate the parent
scenario list. Ordinary parent continuation still pages parent scenarios by canonical key.

This is a deliberate cursor/wire migration. Add the nominal nested-position alternative at the
model owner; update schema identity, native validation, packet assembly and Python schema
consumption together. Reject incompatible old cursors; no old-format reader. Avoid re-encoding a
whole parent packet into the cursor. The association key supplies the necessary parent address;
rich parent content is hydrated from the same pin. Preserve diagnostic multiplicity, correlation,
originals, availability and omission meanings.

Construct the parent page before hydrating child pages where semantics allow. For a nested
request hydrate its addressed parent, not every sibling's diagnostics. This removes both the
composition error and irrelevant child preparation. Unknown/foreign parents or a cursor from a
changed request/realization fail with the typed incompatible cause.

Closure uses the audit's exact trigger: page size one, two scenarios, multiple diagnostics on the
second. Follow the parent token to scenario two, then its nested token; scenario two and its
remaining diagnostics survive. Also cover a further child page, exhaustion, shuffled input,
wrong member/association and changed pin. Expectations name the authored diagnostic IDs rather
than replaying the pagination implementation.

### 6.4 R-S3 — Typed safe failures through Rust, PyO3 and MCP

The model's `FailureKind`/`PublicFailure` already owns five coarse causes and fixed safe messages.
Keep internal diagnostic detail separate. Introduce a typed native service failure that carries a
recognized public cause plus private diagnostic context, or extend the existing owned error
representation equivalently. Classify at the owner where the cause is known: missing admitted
library, incompatible handle/cursor/contract, corrupt evidence/shape, unavailable transport/read,
and refused resource/deadline. Do not infer these causes later by parsing driver/error strings.
Malformed requests and unknown tools retain the appropriate protocol validation error. Unexpected
internal exceptions become a fixed safe unavailable failure and private structured diagnostics.
No new public retryability promise follows from these coarse causes.

Expose one PyO3 exception carrying the serialized owned `PublicFailure` as a checked attribute,
with a fixed safe exception message. The session's slots/closing/deadline failures use the same
cause boundary. Python transports the attribute; it does not reinterpret database diagnostics or
construct independent semantic messages. Pure wire helpers remain model-derived.

At the exact installed FastMCP 4.0.5 / MCP 2.2.0 route, tool errors return a `CallToolResult` with
`isError` and `_meta.lctx_failure`, wrapped using `ToolResult.from_mcp_result`; that wrapper
preserves the raw result. Resource failures raise the preserved `MCPError` carrying the declared
`INTERNAL_ERROR` (-32603) code, fixed message and `data.lctx_failure`; generic `ResourceError`
loses structured data. Implement the advertised model-owned failure envelope, exposing its fixed
code/message/DTO through the pure wire bridge rather than a Python semantic mapping, and validate
final serialized JSON-RPC bytes, including
request ID and newline. Keep a fixed fallback once if the full error envelope exceeds its cap;
never recursively encode an error or release admission before worker drainage.

Migration includes `WireError`/failure definitions, `service::failure` and source read owners,
PyO3 `session.rs`, `lib.rs` wire exports, MCP executor and `wire.py` tool/resource/middleware
paths, generated schemas and affected controls. Keep root ownership of the shared wire declaration;
Rust and Python edits can be divided only after that concrete contract is settled.

Closure: actual in-process and stdio MCP tool/resource requests for unknown library, incompatible
cursor/pin, unavailable read and corrupted evidence have their correct fixed cause/envelope.
Inject an unexpected diagnostic containing a sentinel secret/path and assert it appears in neither
public text nor metadata. Cover resource/slot/deadline refusal and complete-byte admission with a
large ID; workers remain owned and drain. A transport-shim fault can deterministically supply a
late/unavailable read, but report it as an injected failure case, not live service qualification.

### 6.5 Query improvements and native cancellation boundary

Apply native indexed restrictions and bulk field projections wherever the sufficient input is
known, retaining complex model kernels. `EXPLAIN` identifies a concrete access path; selectively
use `EXPLAIN ANALYZE` on owned read-only cases when an actual plan question remains. Neither is
request accounting or a universal performance acceptance requirement. Keep filtered ANN witness
semantics, shared vectors and existing approximate pre-collapse candidate limits unchanged.

The pinned gRPC server has cooperative cancellation on stream drop and requested/server timeout.
Use the SDK route and existing end-to-end remaining deadline; do not add an embedded core solely
for cancellation. Local synchronous kernels need their existing cooperative checks at meaningful
boundaries; an async timeout cannot forcibly interrupt them. Preserve shielding/drain ownership in
Python. During affected native controls exercise an owned delayed/long-running read, release the
client stream, and confirm cancellation/drain permits subsequent work; do not claim stronger
preemption than the implementation demonstrates. This targets an audit evidence limit without
creating a new always-on observer or restoring a broad test campaign.

## 7. ER3/ER4 — Contextual discovery and actually delivered information

**Implemented / focused Tested, 2026-10-07.** The [combined coordinator](evidence-retrieval-and-evaluation-plan_2026-10-06.md)
owns ER state/new findings. Its §2 replaces the baseline §2.2/§2.3 family/member collapse and single
spec in the implemented ER contracts. Combined execution completed §6's targeted audit acceptance; the coordinator records each finding's closure evidence.

### 7.1 Discovery nomination and coherent expansion — ER3

Consume admitted SearchWindows/WindowBindings and declared value/projection policy; no query-side
attribution classifier guesses which sibling a byte fragment belongs to. Exact identifier/option
paths, code-aware lexical matches (including BM25-zero) and1024 HNSW nominate eligible target/context
alternatives with actual window/channel witnesses. Standalone source/document/scenario/deployment
units have independent paths; absence of a member never erases them.

Group best contributions per family/channel/applicable target/context before final result caps;
within-family RRF K60 then equal-family ranks and canonical ties remain the initial policy. Positive
relevance and literal-match eligibility are separate. Bounded widening128/256/512/1024 reduces raw
crowding while retaining honest approximation/tier exhaustion. Full4096 candidate-union rescoring
cannot recover missed nomination. Record exact route, projection, query-value digest, tier, fallback
and ranking policy. A channel failure retains the declared whole-pool fallback or explicit error,
not mixed incompatible scores per candidate.

Use the realization plan's typed cohort/bitmap, residual-context and small eligible exact routes.
Native model-owned restriction/expansion batches useful frontier inputs, applies qualification/
context before growth and hydrates only retained evidence. Named paths preserve support/conditions,
contradictions, high-degree partial frontiers and original byte access; physical adjacency is not
proof. No per-edge network loop or engine-step accounting is introduced.

Optional neural scoring requires a separate qualified checkpoint/template/realization and bounded
pool. Initial baseline has no reranker. vLLM engine reuse does not make embedding weights a classifier
or token-level model; model residency/acquisition and cancellation/fallback are actual obligations.
Validity is established before and after selection, never traded for a heuristic score.

### 7.2 Demand-aware packets and final observations — ER4

Extend existing model-owned operation inputs with optional production EvidenceDemand: requested
information facets/context and bounded delivery/expansion policy. It never carries evaluator expected
answers, witness lists or task IDs that reveal private truth. Existing routes keep their mandatory
identity/signature/qualification defaults when demand is absent. Ten public routes remain; no evaluator
MCP tool or duplicate semantic Python model is added.

Construct compatible context-closed bundles, including mandatory setup/conditions and available
original anchors. A default expression, declared/effective option and configuration override remain
distinct. Select alternatives using actual renderer/envelope cost, then encode once and verify the
real final JSON-RPC/text/structured envelope. Greedy packing is heuristic. Required indivisible cores
remain protected; optional omitted/partial/refused/expandable content is explicit.

Replace unconditional demo/link counts as selection rules with bounded demand-aware alternatives
inside existing final32KiB default/256KiB expanded envelopes. Defaults may retain concise preferred
counts as presentation heuristics, never discard a demanded mandatory dependency solely to meet them.
Whole responses, not hidden graph inventories or opaque IDs, establish immediate information.

PacketEvidenceMap links actual delivered fields/text spans to source/window/binding/qualification
and interpretation dependencies. Maps include synthetic fields and omission/expansion state without
inventing original spans. Incorrect but schema-valid maps must fail independent controls. Source IDs
without readable defaults/conditions are references, not delivered meaning. Public get_evidence and
section expansion retain pinned originals, statuses and actual availability.

### 7.3 Stable ranked-result continuation and safe effects

Retain each bounded ranked ordering in NativeSession as an immutable result entry containing query
vector digest, eligibility/request/policy, candidate ordering/witness keys, channel/fallback/tier,
scorer and snapshot identity. Initial retention:16 entries and8MiB per session,10-minute expiry;
remove expired then least-recently-used entries. Cursor carries session/result identity/digest and
offset plus existing request/realization bindings. Generated outputs disclose ranked-cursor expiry.
Expiry, eviction, foreign session or restart returns explicit continuation-unavailable, never a
silently recomputed differently ranked page. Original/browse/section references retain their own
existing snapshot-bound contracts. This is bounded transient pagination state, not query history
or a persistent registry/service.

Share one ranking realization across each page/expansion journey. If optional scorer availability
changes after first page, continue the retained order or refuse; do not silently fall back. New
search creates a new realization. Bound actual retained bytes/entries using ordinary memory/lifetime
controls, without measuring engine work. Owned session shutdown drains workers before dropping
result/native state; safe typed errors and actual final-envelope controls remain §6 obligations.

### 7.4 Consumers and focused acceptance

Migrate search_operations/search_evidence/find/get/compare packet consumers, model schemas/decoding,
Rust dispatch, PyO3 NativeSession and thin FastMCP listing/calls/resources together. Coverage/defaults,
classification, mandatory signatures, singleton/public access and independent evidence paths survive.
Canonical new records and native lowerings arrive through ER1–ER3, not a second serving authority.

Actual selected persistent/native and editable-extension MCP controls cover sibling/setup/variant
bindings, zero-score identifiers, high raw-window crowding, scoped nearest vectors, mismatched query/
projection policy, readable condition removal with ID intact, contradiction/unknown delivery,
mandatory setup, alternate witnesses, original expansion, final32/256KiB envelopes and expiry/foreign/
evicted/restarted continuation refusal. Channel/scorer loss never creates a different next page.

The evaluator observes these unchanged public interfaces and exact final bytes, but expected logic
is independently authored. Its delivery maps do not self-certify. No real-library/live-vector/paid
agent campaign is run to establish local contract controls; those claims remain separately activated.
