# Native graph retrieval, selection and evidence serving

**Proposed, 2026-10-05.** Supporting plan for S1/S2/S3 in the
[replacement coordinator](graph-native-pivot-plan_2026-10-05.md). It consumes one admitted
graph and its [sealed SurrealDB realization](graph-native-surrealdb-realization-plan_2026-10-05.md).
The coordinator owns current state/disposition. Native querying is a selected part of the target,
not a capability awaiting a benchmark, accounting proof or adoption verdict.

## 1. Foundation assessment and operation ownership

Current `GenerationReader::prepare_selection` loads 95 classification inputs, then output and
admission inputs. `Prepared::select` enumerates the whole public-exposure domain; the catalog
service applies library filtering afterwards. Store-backed packet/evidence/native services have
many chained key reads; explanations can hash entire derivation relations for a small answer.
These source-inspected routes, 2026-10-05, are replacement targets, not performance measurements.

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
All execution outcomes in this document are planned, not newly Tested or Measured.
