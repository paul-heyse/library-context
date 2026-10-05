# SurrealDB 3.3 capability and integration assessment

**Proposed target, assessed 2026-10-05. Decision: Accept scoped for capability selection and
the operation boundaries below; implementation, performance and production qualification remain open.**

Use SurrealDB as an application query engine as well as a graph store. The highest-value combination
is typed graph/document storage, indexed eligibility, lexical/vector retrieval, database functions,
and bounded evidence hydration. This can remove substantially more orchestration than replacing
PostgreSQL tables with relation records while retaining today's lookup chains.

The Rust model still owns meaning. Its generated schemas, query lowerings and executable kernels
may run in several places. **Rust ownership does not require host-only execution.** Conversely,
moving work into a database does not make a built-in ranking or permission convention the product's
semantic authority. Choose placement per operation, including its failure and resource behavior.

This follows the [graph-native target review](design_review_graph-native-target_2026-10-05.md).
SurrealDB is the selected premise. Existing data are disposable; this is not a compatibility migration.
The review expands the target, particularly its execution placement, rather than reopening the
PostgreSQL-versus-graph decision. Nothing here claims the replacement has been implemented.

## 1. Scope, method and prioritized recommendation — slots 1, 8, 12

| Field | Assessment boundary |
|---|---|
| Baseline | `main` at `7f65ab7b`, clean at investigation start; only review/navigation/handoff documents change in this work |
| Standard | Core 3.2, code-intelligence profile 1.3 and the [repository binding](../design_principles/binding/library-context.md); design tier, target purpose |
| Intended product | [Version-pinned API and evidence catalog](../../design/sections/api-and-evidence-product.md): discovery, selection, invocation/configuration, original evidence and precise uncertainty |
| Required outcomes | Validated pinnable snapshots; attributed identity-bearing assertions; evidence closure; typed fidelity; deterministic declared analytics; bounded explicitly partial serving; the pinned embedding spec |
| Assessment team | Root integration; comprehensive skill research and focused retrieval research; independent extension/application-operation research; independent design review |
| Evidence | Static source, types and documentation plus previously executed skill/repository receipts. No new database, compiler, embedding or benchmark run |
| Non-goals | Product implementation, store reset, dependency changes, old-format readers, simultaneous canonical stores, a new universal query compiler or workflow framework |

The comprehensive skill assessment examined all 16 capability briefs, all catalog families,
routes/coverage, the 428-function discovery catalog and selected underlying source/docs. Discovery
coverage is broader than individually qualified function semantics. Targeted current official
documentation and Context7 filled ecosystem/operational gaps. The pinned runtime/source is 3.3.0;
the skill's documentation capture is from main, so newer prose does not override that runtime.
Sources and coverage limits are recorded in [§9](#9-evidence-and-source-map--slot-10).

| Priority | Recommendation | Why it changes the target |
|---|---|---|
| 1 | **Use now:** graph/document records, strict schema envelopes, typed Rust SDK, parameterized SurrealQL and database functions | Persist the admitted artifact and execute complete useful retrieval operations close to it |
| 2 | **Use now:** full-text/BM25, exact symbol matching, vector search and occurrence-aware eligibility; **design around** bitmap fusion and selected INLINE storage | One engine can narrow, retrieve and hydrate; indexes participate in the representation design |
| 3 | **Use now:** checked bulk writes, a short publication transition, index readiness, EXPLAIN and basic telemetry | Comparable assurance with one persistence reconciliation and visible operational cost |
| 4 | **Use now:** canonical projection export and petgraph consumption; **design around** typed streaming | Reuse expensive extraction/admission for analytics, debugging and additional destinations |
| 5 | **Investigate for a named kernel:** Surrealism Rust/WASM; **use conditionally:** native RRF and custom APIs | Potentially remove another process/transport boundary, but specific semantics determine suitability |
| 6 | **Defer until consumed:** buckets, reactive enrichment, generated GraphQL, external connector pipelines and distributed deployment | Useful extension paths; none is a prerequisite to the first coherent graph product |

“Use now” means include in the proposed replacement design, not install or enable every feature in
this review. “Design around” means preserve the relevant representation/operation opportunity and
select its physical configuration during implementation. Experimental features can be valuable;
their uncertainty should attach to their optional use, not hold the whole pivot hostage.

## 2. Target responsibilities and composed operations — slots 2–4

### 2.1 Meaning, placement and the admitted artifact

**Proposed.** Keep five coherent responsibilities; these need not become five crates.

| Owner | Owns | Derived implementations and consumers |
|---|---|---|
| Rust semantic model | Entity/assertion kinds; typed properties/endpoint roles; identity recipes; qualifications; coverage; selection, contribution and evidence rules | Admission operations, schemas/codecs, supported query predicates, functions or shared kernels |
| Compiler | Pinned acquisition, extraction/normalization, declared analyses, admission and certificates | Immutable graph stream/workspace; no requirement to persist and reread every stage |
| SurrealDB realization | Compact physical families, schema/index/function installation, typed encoding, checked loading and reconciliation | Database-local retrieval operations and rebuildable indexes |
| Application operation owner | Public request interpretation, response envelope, budgets, completeness and permitted effects | Host Rust, SurrealQL or qualified shared Rust/WASM realization; thin MCP presentation |
| Projection consumer | Source snapshot, topology/universe, direction, parallel-edge policy, weights, method/settings and result mapping | petgraph first; other analytics/export destinations when consumed |

Mechanical lowering is appropriate for schema envelopes, field types, kind codes and query
parameters. It is not a promise to transpile arbitrary Rust into SurrealQL. Keep complicated
semantic classification in its existing authoritative kernel unless a specific database lowering
or shared module is simpler and demonstrably preserves the operation. Authoring a SurrealQL body
beside its Rust-owned contract is acceptable; separately reimplementing the same meaning in Python,
Rust and database triggers is not. A model-owned operation can deliberately have SurrealQL as its
sole implementation, with Rust owning its declarations and callers. Generation or shared WASM is
especially useful when that same meaning must execute in multiple places; neither is mandatory
ceremony for every native operation.

| Fact/fidelity family | Authority and identity | Coverage and interpretation | Served/derived use |
|---|---|---|---|
| Source entities and occurrences | Pinned artifact, source coordinates, provider/run/revision; semantic entity ID distinct from occurrence | Original bytes and analyzer coverage remain addressable | Invocation/configuration, source navigation |
| Provider assertions and normalized conclusions | Assertion ID, kind, endpoint roles, provider/context and premise identity | Extracted, resolved, observed and derived remain distinct; disagreement retained | Requirement classification and evidence |
| Derivations and certificates | Method/revision, ordered premises, subject, settings and witnesses | Checked under the declared finite model; incomplete/unrequested remains explicit | Claims and explanations |
| Retrieval units and embedding uses | Unit/text identity, contextual occurrence, family, vector/spec identity | Relevance is heuristic; a ranking witness is not proof of a requirement | Search and winning contextual evidence |
| Analytic projections/results | Snapshot/projection/method/settings/seed identity; local graph indices are temporary | Exact, conservative or heuristic as declared; no invented convergence | Ranking/grouping or qualified behavioral output |

Store ordinary binary assertions as classic, identity-bearing relation records where suitable.
Represent n-ary assertions as records with role-labelled participants. Keep nested invocation,
configuration and typed value payloads as documents where traversing each field has no value.
Use a small number of physical families chosen for access patterns, not one table per Rust
declaration. Common typed envelopes and admitted payloads can coexist; an untyped property bag
must not become the model.

### 2.2 Snapshot isolation without the old lifecycle

**Proposed default:** one strict database per immutable snapshot within the project namespace,
with the same compact physical families, and a small control database listing published snapshots.
This makes schema, functions, analyzer configuration and corpus-level search statistics snapshot-local.
It also avoids requiring every traversal to remember a generation predicate. Database selection is
resolved once to a dedicated request/process handle, never changed on a shared client during work.

Logical entity IDs remain independent of physical database names. Snapshot identity covers admitted
content and semantic context; the realization manifest additionally identifies codecs, functions,
analyzers, index specifications, engine version and any module artifacts. A snapshot is more than
immutable rows: changing a stored function or tokenizer must not silently change a pinned answer.
Index rebuilding can be a new recorded realization over the same semantic content.

The publisher loads a private target, finishes required index construction, then quiesces and drains
all content/definition writers before final reconciliation. It removes the application's ability to
mutate sealed records, functions, analyzers and module definitions; reader credentials and approved
operation bodies expose no mutation path. Administrative recovery remains explicitly trusted. A
private database name or READONLY field alone is not sealing. The publisher then makes the completed
handle visible in a short control-record transition. This is deliberately not a cross-database transaction.
A crash before visibility leaves an unselected target; a crash after visibility leaves a completed
immutable target. Serialize this single-operator publication path; use a conditional control update
or lock if concurrent publishers are later introduced. Retention must not remove a database while
its readers are active. No per-table leases or distributed epochs follow from this requirement.

A changed function, analyzer policy, module or rebuilt search realization gets a new served handle,
or affected readers are explicitly drained before replacement. Do not mutate a realization underneath
already pinned readers. Engine maintenance preserving the admitted realization's contract is distinct
from changing its definition.

A shared database with snapshot-qualified records remains a viable alternative if the number of
retained databases becomes material. It adds cross-snapshot filtering and search-statistics concerns;
structural sharing is not required for the first target. Native `VERSION` queries are useful for
selected operational histories, but do not replace semantic snapshot identity or freeze functions,
external artifacts and embedding settings.

### 2.3 One retrieval operation, several execution mechanisms

**Proposed operation:** consume a pinned snapshot, normalized requirements, query text/vector and
embedding spec, ranking policy and work/output budgets. Produce member results with classifications,
winning unit/occurrence/channel witnesses, evidence references and explicit completeness metadata.

1. Resolve eligibility over the requested context domains. Preserve supported, unresolved,
   conflicting and contradicted meanings. A shared vector is eligible only through an eligible
   occurrence; the returned witness must come from that eligible subset.
2. Execute exact-symbol, lexical and vector candidate channels with eligibility inside each leg.
   Supported indexed predicates can use bitmap fusion; complex graph predicates may remain residual
   work. Do not retrieve a global top-k and then discard ineligible records.
3. Canonically order candidates and collapse contributions by the declared member/family key.
   Fuse those contributions and retain a separate witness inventory. Apply the final member limit
   only after this semantic operation.
4. Hydrate winning evidence through bounded fixed paths. Requirement-establishing evidence remains
   separate from the passage that made the member rank highly.
5. Return a typed envelope with availability, coverage, ranking mode, truncation/stop reasons and
   continuation where meaningful. A timeout or absent vector index is never an empty complete answer.

Requirement witnesses retain variant, binding and configuration context. Two requirements supported
by different overloads are not thereby a jointly supported invocation. One authoritative classifier
owns independent support versus same-context joint applicability wherever that classifier executes.

Use a database function for the supported complete operation or its coarse retrieval/hydration
segments. Ordinary arrays, grouping, links, subqueries and functions can remove transport stitching.
If a complex classifier remains host Rust, retrieve its bounded inputs in batches and keep the
candidate-completeness contract explicit. Do not reproduce its meaning in an approximate WHERE clause.

Top-k unit retrieval is not automatically top-k member retrieval: one member with many examples can
consume the candidate budget. A fixed overfetch multiplier cannot establish exact member ranking.
The default discovery contract should state **ranking over bounded candidates**; exhaustive find
uses separate ordered eligibility enumeration. An exact member-top-k claim requires a stopping
criterion or full eligible computation that actually establishes it.

### 2.4 Compute, evidence and export contracts

| Operation | Input universe and method | Output and limits | Preferred initial placement |
|---|---|---|---|
| Compile/admit | Pinned provider outputs and typed model; keys/references checked as records are constructed | Admitted graph, explicit unknown inventory and certificates; spillable/bounded workspace | Rust compiler |
| Stored reconciliation | Canonically decoded nodes/relations and manifest; physical adjacency realization checked | One persisted-content identity/closure result; never just row counts | Rust streaming reader plus bounded engine queries |
| Fixed evidence packet | Named roots and permitted relation roles in one snapshot | Original claims/premises/source links under row/edge/byte budgets | SurrealQL function, then thin envelope validation |
| Variable graph exploration | Declared topology/direction; visited node and assertion IDs; frontier | Compact witnesses and explicit incomplete frontier; no all-walk materialization | Bounded operation; host Rust initially where engine work accounting is insufficient |
| SCC/fixpoint/FCA/ranking | Declared projection, weights, method/settings/seed and convergence | Canonical results linked back to premises; heuristic status retained | Existing Rust libraries; qualify selected database-local kernels when valuable |
| Export | Snapshot plus projection definition; nodes independently of edges | Typed records/stream with source IDs, isolates, multiplicity and declared losses | Rust SDK/query stream and a destination-specific adapter |

The native graph is the persisted source for these consumers. Building a dense petgraph topology
is still a projection, but it no longer requires reconstructing meaning from hundreds of storage
relations. Export is a semantic capability, not a promise that every destination accepts SurrealDB
records unchanged.

Mandatory packet structure is indivisible: an incomplete signature or missing required qualification/
evidence status is packet unavailability or a declared refusal, never an apparently complete answer.
Optional explanation/source expansion may be partial independently. Native placement is adopted only
where the operation's work bounds can be realized; opaque unrestricted walks are outside that scope.

## 3. Native capabilities worth using — detailed inventory, slots 3, 4, 8

All adoption recommendations in this section are **Proposed, 2026-10-05**. Mechanism descriptions
are **Interface-checked** unless explicitly attributed to a named existing Tested receipt.

### 3.1 Graph, documents, schema and value fidelity

| Capability | Recommendation and concrete value | Qualification/obligation |
|---|---|---|
| Classic `TYPE RELATION`, `RELATE`, `INSERT RELATION` | **Use now.** Assertion IDs, edge properties and graph adjacency replace hand-built relational navigation; parallel assertions remain distinct | Use the relation insertion path, not ordinary INSERT of edge-shaped objects. Endpoint-pair uniqueness is inappropriate for distinct assertions |
| Typed IN/OUT and `ENFORCED` | **Use now** as structural defense for closed physical families | Checks table/existence contracts, not all Rust kind/role/context/snapshot semantics; admission still owns those |
| Record links and reverse `REFERENCE` relationships | **Use now** for ordinary references and inverse lookup where they match ownership | A record link alone can dangle and read NONE; inverse references do not establish evidential support |
| Nested objects/arrays, typed record IDs, composite keys | **Use now.** Preserve invocation/configuration structures without proliferating graph nodes | Encode identities canonically; strings resembling record IDs are not record values. Avoid numeric ID ambiguity and oversized unsigned integers |
| `SCHEMAFULL`, strict database, typed fields, ASSERT/UNIQUE | **Use now** for generated common envelopes and physical keys | 3.3 SCHEMAFULL rejects unknown fields; STRICT prevents implicit tables. Coercions mean engine acceptance is not typed semantic admission |
| DEFAULT, VALUE, READONLY, COMPUTED | **Use selectively.** Defaults and local projections reduce loader/read boilerplate | Source-derived values must have one owner. READONLY fields do not make a database immutable; COMPUTED work happens on reads and must remain bounded/pure |
| LIGHTWEIGHT edges | **Defer for canonical assertions.** Consider only rebuildable connectivity where pair identity and no assertion payload are intentional | They cannot substitute for identity-bearing assertions, their provenance or assertion-as-endpoint relationships |
| INLINE relation fields | **Design around** small, frequently filtered kind/role/context values | Accelerates rejection during adjacency scans; choose physical inline payload deliberately, not every property |
| `INLINE EDGES` / `INLINE REFERENCES` | **Design around** low-degree batched expansion | Distinct from inline edge fields; caches rewrite on mutation and high-degree cases fall back. Immutable serving is a promising fit, not a measured speedup |
| NONE, NULL and scalar/bytes codecs | **Use typed SDK values now**, explicit tagged uncertainty and availability states | Neither NONE nor empty arrays encode all product unknown/refused/not-requested states. Canonical identity/digest bytes cannot be delegated to arbitrary JSON serialization |

Sources: skill graph/schema/value briefs and SB006/011/020/022/023/028/029/046/047;
[relation tables](https://surrealdb.com/docs/reference/query-language/statements/define/table),
[field definitions](https://surrealdb.com/docs/reference/query-language/statements/define/field),
[record IDs](https://surrealdb.com/docs/reference/query-language/language-primitives/data-types/record-ids).

### 3.2 Query language, optimizer and derived forms

| Capability | Recommendation and concrete value | Qualification/obligation |
|---|---|---|
| Parameterized SurrealQL, record paths, arrows and FETCH | **Use now.** Retrieve complete evidence segments in one engine operation | Bound fanout and selected fields; unrestricted hydration can recreate the same memory problem with fewer network calls |
| Correlated subqueries, SELECT over arrays/parameters, GROUP and ORDER | **Use now.** Replace host joins/stitching when the access plan fits | There is no general ANSI JOIN operator; correlated nested scans are not automatically efficient. Keep genuinely columnar compile work in Rust/DataFusion where suitable |
| Typed database functions and `run` | **Use now.** Named retrieval/hydration/projection operations shared by transports | Signature types are not full domain contracts; functions, permissions and operation revision belong in the snapshot realization |
| Bitmap index fusion | **Design around now.** Combine supported scalar, full-text and graph restrictions before record fetch | New/rebuilt 3.3 indexes share doc-ID mappings. Supported single-hop graph forms do not imply indexed arbitrary traversals |
| Filtered KNN and its exact/graph tiers | **Use now** with eligible predicates in the search statement | Scalar/FTS restrictions may prefilter; residual predicates still cost work. `WITH`/`VERSION` can disable this path; record the actual EXPLAIN plan |
| `IndexCountScan`, covered/index-aware selection | **Use when applicable.** Availability/count endpoints need not hydrate every record | A total must name its domain and completeness. Explain supported shapes instead of assuming every COUNT is cheap |
| `EXPLAIN ANALYZE FORMAT JSON` | **Use now** for operation development and regression diagnosis | Inspect operators, candidates, rows/batches and prefilter tier. Plan rendering is diagnostic and version-sensitive, not the public product schema |
| Materialized `AS SELECT` views | **Defer broad use; use narrowly** for measured repeated local aggregates | Updates follow the FROM table; linked changes and import paths can leave views stale. Explicit rebuild before immutable publication can make selected views safe |
| Collection/object/string/type helpers and closures | **Use now** for mechanical transformation, deduplication and projection | Determine actual ordering/equality/missing-value semantics. A database-wide scripting language is unnecessary |
| Recursive collect/path/shortest-path idioms | **Use bounded fixed paths; restrict variable exploration** to operations with frontier accounting | Depth truncation is not a completeness certificate. `+path` enumerates walks and vertex paths do not retain assertion identity automatically |

Sources: skill querying/GQL briefs; [query optimization](https://surrealdb.com/docs/learn/querying/concepts-and-guides/query-optimisation),
[EXPLAIN](https://surrealdb.com/docs/reference/query-language/clauses/explain),
[filtered similarity search](https://surrealdb.com/docs/learn/data-models/vector-search/similarity-search),
and the existing [real-graph traversal evidence](../evidence/2026-10-05_graph-analytics-parity/README.md).

### 3.3 Search, ranking and embedding integration

| Capability | Recommendation and concrete value | Qualification/obligation |
|---|---|---|
| Exact symbol/key lookup | **Use now**, independent of prose relevance | A qualified name match is not evidence that requested behavior is supported |
| Full-text analyzers and BM25 | **Use now.** Separate identifier tokenization from prose; consider camel/class/punct and lowercase for names | Inspect `search::analyze` on actual dotted/underscore/Unicode symbols. Language stemming is a policy choice, not a universal improvement |
| `search::score`, highlight and offsets | **Use now** for relevance witnesses and useful UI/MCP excerpts | Original source byte anchors remain authoritative; transformed text offsets do not replace them |
| Exact vector distance / `<\|k,COSINE\|>` | **Use now** for small eligible sets, controls and declared exact computations | A scan can be expensive; memory/time budgets and eligibility remain necessary |
| HNSW | **Use for explicitly approximate ranked discovery** when the eligible corpus warrants it | Missing index can return empty results. Admit index readiness and spec identity; ANN omissions cannot establish absence |
| DISKANN | **Design around as an alternative** when vector-index residency becomes material | Existing SB098 exercises primitives, not this corpus's recall/latency. It has different graph/vector cache behavior from HNSW |
| Native `search::rrf` | **Use conditionally** after semantic contribution collapse and with separate witnesses | Counts duplicate IDs even within one list; cutoff ties are not canonical; merged fields do not preserve a channel inventory. See F01 |
| Native `search::linear` | **Defer as default**, retain as a quality-policy alternative | Pass explicit scores. Score precedence/normalization and rank fallback are substantive policy; the source fallback counter spans lists |
| Shared vectors linked to contextual occurrences | **Use now** to retain deduplication without losing provenance | Request eligibility must reach the vector through eligible occurrences before top-k; benchmarkable query shape may determine whether a derived mapping is worthwhile |
| CJK tokenization/dictionaries | **Defer until multilingual evidence is supported** | Skill SB108 covers the mechanism; dictionary availability/build features and analyzer revision enter the realization |
| External embedding providers / native HTTP | **Retain the existing admitted embedder now**; avoid query-triggered remote enrichment | Network calls add effects/retries and can extend transactions. Generic integrations do not preserve the pinned model/spec automatically |
| SurrealML uploaded models | **Defer** until a compatible model and consumed inference task exist | Native inference availability does not establish Qwen transformer/tokenizer/pooling parity |

Preserve the [embedding specification](../../../specs/embedding/qwen3-embedding-8b.json):
Qwen3-Embedding-8B and tokenizer revision, query instruction, document rendering, token admission,
4096-dimensional source/MRL reduction to 1024, float32 output and L2 normalization. The source
model service and stored/search vector are different boundaries. Store the canonical consumed vector
and its specification identity; HNSW/DISKANN indexes are rebuildable realizations over it.

Native fusion can still save code. Pre-collapse each input list to its intended contribution key,
request every ID in the bounded candidate union from RRF, then canonical-sort and limit; retain
witness tuples separately. Where family normalization needs different semantics, a small generated
fold or the existing Rust kernel is clearer. Sorting an already truncated tied result cannot recover
candidates that the native cutoff discarded.

Sources: skill search/vector briefs, SB035/037–039/098/108;
[analyzers](https://surrealdb.com/docs/reference/query-language/statements/define/analyzer),
[fusion implementation at 3.3.0](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/fnc/search.rs),
and [vector index definitions](https://surrealdb.com/docs/reference/query-language/statements/define/indexes).

### 3.4 Publication, transactions and operational assurance

| Capability | Recommendation and concrete value | Qualification/obligation |
|---|---|---|
| Bounded multi-record INSERT / INSERT RELATION | **Use now** for admitted chunks; tune by bytes as well as records | Finish node/endpoint loading before dependent enforced edges; check every statement and intended identity/count, not only outer SDK success |
| `Response::check`, typed statement results | **Use now** in one store adapter | `query().await` can succeed while a statement fails. Permission-filtered writes and INSERT IGNORE can lose intended rows without an error |
| Explicit transactions / client transactions | **Use now** for each bounded unit and publication metadata | Do not hold a transaction open through CPU validation or network embedding. Independent statements outside a transaction can partially succeed |
| Conditional writes, `FOR UPDATE`, conflict handling | **Use at the few shared mutable records** | Native backends permit write skew. Locking targets record IDs; retries belong to a bounded idempotent operation, not arbitrary error-string retry |
| `DEFINE INDEX ... CONCURRENTLY`, `INFO FOR INDEX` | **Use when ingestion benefits**, then await required readiness | Definition acceptance is not completed construction. Failed UNIQUE construction creates no uniqueness protection; ANN absence can look empty |
| Users/access methods, table/field/function permissions and capabilities | **Use a simple publisher/serving/operator separation** | Record permissions do not constrain root/system identities in the same way. Embedded auth must be explicitly enabled; endpoints have additional behavior (§3.5) |
| Export/import and selective export configuration | **Use now** for diagnostics/recovery and portable snapshot copies | Logical export is not a destination-neutral graph schema or proof of external-file backup; use the documented HTTP export/import path |
| Health/version/startup readiness | **Use now** in the operator service | Listening is not necessarily fully initialized, and server health is not graph publication/index readiness |
| Query/call timeouts and cancellation | **Use now** beside explicit operation budgets | A timeout limits elapsed execution, not examined edges or allocated memory; reconnect waits also need caller deadlines |
| Resource and cache controls | **Use now** for bounded server operation, with actual footprint observation | 3.3 memory threshold includes allocator-reserved memory and RocksDB cache; leave headroom. A configured threshold is not measured per-operation peak memory |
| JSON logs, slow-query helpers, `/metrics`, OTLP | **Use now**, starting with existing logging/telemetry infrastructure | Community provides basic signals; durable audit/slow-query pipelines and distributed metrics have Enterprise distinctions |
| Index maintenance and background tasks | **Use managed server lifecycle initially** | Direct-core embedding that disables maintenance affects async events and other services; hidden background requirements must be owned |

Assurance remains concentrated at three boundaries: typed graph admission; one stored-content and
physical-realization reconciliation; public operation qualification. Certificates for SCC order,
witnesses and fixpoints can avoid universal re-derivation when their verifier establishes the
required property. Row counts alone cannot detect changed values, missing-and-duplicated identities
or broken adjacency. Independent small semantic controls remain valuable; per-stage store rereads,
per-field anti-joins and blanket replay are not inherited requirements.

Sources: skill bulk/error/transaction/security briefs and SB013/030–033/048/053/091–104;
[Rust export](https://surrealdb.com/docs/reference/rust/methods/export),
[operational signals](https://surrealdb.com/docs/manage/observability),
[server configuration](https://surrealdb.com/docs/reference/cli/surrealdb-cli/environment-variables).

### 3.5 Application endpoints and computation extensions

| Capability | Recommendation and concrete value | Qualification/obligation |
|---|---|---|
| Typed `DEFINE FUNCTION`, permissions and RPC `run` | **Use now** as the main native operation surface | Ordinary function execution retains caller record-permission behavior; invocation from a privileged API inherits that context |
| `DEFINE API`, middleware, `$request`, `api::invoke` | **Use conditionally** when an HTTP/UI consumer exists | API handlers run with record-permission checking disabled after endpoint authorization. Generated bounded bodies must enforce their input/snapshot scope |
| `api::req::max_body`, `api::timeout` | **Use with custom endpoints** | Body/time limits are useful transport protection, not graph work or response-size accounting |
| HTTP response/error construction | **Use deliberately** | A returned HTTP error status is still successful query execution and can commit preceding writes; throw/fail the transaction to roll back |
| Surrealism Rust/WASM exports and `.surli` modules | **Investigate for selected pure kernels; do not block the initial product** | 3.3.0 links experimental Surrealism 0.6.0, WASI P2 / ABI 2. Reuse the same authoritative Rust logic if its dependency graph is compatible; do not assume all of lctx-model compiles unchanged to WASI |
| Module transactional `sql()` / `run()` host calls | **Promising for bounded projection preparation or reduction** | Calling once per edge simply relocates N+1 work. Batch data access and bound the module's work |
| Module capabilities, memory/time/state controls | **Required for any adopted module** | Instances/statics survive calls; module KV is runtime-local in-memory state, not durable transaction state. Configure memory/time ceilings and require strict timeout mode; deterministic results must not depend on incidental prior state |
| Silo versioned distribution / local artifact loading | **Prefer owned content-addressed artifacts initially** | 3.3 uses `FROM`, exact Silo coordinates and mandatory `UNSIGNED`; do not mistake a version string for signature verification |
| Embedded JavaScript and eval functions | **No default use** where SurrealQL or shared Rust suffices | An additional language/runtime is justified only by a concrete operation; eval requires separate capabilities |

There is no requirement to move all analytics into SurrealQL. Pure bounded kernels are the first
WASM candidates; native analyzers, Python bridges, thread-heavy libraries and large graph jobs can
stay host-side. A named module should remove a real data-transfer or worker burden, or serve an
agreed new operation more simply, while preserving its result contract. Its initial compatibility/
resource qualification is local to that kernel. Bounded condition operations, vector/spec validation
and deterministic assertion rendering are plausible candidates. Prefer sharing their portable source
implementations over porting the complete Arrow/analytics/analyzer dependency closure. The runtime's
`strict_timeout=false` path cannot interrupt computation by deadline; memory/time limits are not all
configured by default. These are configuration obligations for a selected kernel, not grounds for
rejecting extensions generally.

The native API permission and commit behavior is visible in
[API invocation](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/api/invocation.rs)
and [datastore handling](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/kvs/ds.rs).
See also [functions](https://surrealdb.com/docs/reference/query-language/statements/define/function),
[APIs](https://surrealdb.com/docs/reference/query-language/statements/define/api),
[modules](https://surrealdb.com/docs/reference/query-language/statements/define/module) and
[Surrealism extensions](https://surrealdb.com/docs/learn/extensions). Runtime details:
[host calls](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/surrealism/host.rs),
[module runtime](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealism/runtime/src/runtime.rs),
[timeout engines](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealism/runtime/src/epoch.rs).

### 3.6 Reactive features and original files

| Capability | Recommendation and concrete value | Qualification/obligation |
|---|---|---|
| Synchronous events | **Use narrowly** for small deterministic dependent records or operational state | They run in the writer's transaction with record permissions disabled and can add hidden write amplification; domain semantics and authorization must remain owned |
| ASYNC events, RETRY/MAXDEPTH and enqueue | **Defer mandatory graph construction**; useful for optional cache warming/notifications | Execution-time state is distinct from original mutation state. Required outputs must finish before publication, and effects must tolerate retry |
| LIVE SELECT | **Use when progress/UI notifications are needed** | Not a durable publication receipt; reconnect does not restore subscriptions automatically |
| CHANGEFEED / SHOW CHANGES | **Defer until a real incremental consumer exists** | Configure retention and cursor/gap recovery. No changefeed can produce an empty result without proving no change occurred |
| `DEFINE BUCKET`, file values and local/cloud storage | **Promising optional source-artifact integration** | Experimental; bucket metadata/permissions and object data have separate recovery concerns. Keep graph pointers content-addressed |
| `file::head/get/list/copy/put_if_not_exists` | **Use if buckets are adopted** to reduce artifact-service glue | No-overwrite object writes help immutable publication, but external object effects are not assumed to roll back with graph transactions |
| Bucket READONLY / permissions | **Use for published originals where supported** | A read-only setting is an access boundary, not content identity or complete backup verification |

The simplest initial artifact choice is to lower the current canonical `ArtifactChunk` byte contract
into SurrealDB bytes/chunk records alongside its metadata and references. The current originals live
in PostgreSQL generations, not a separate artifact service; retaining that backend is not part of
this target. Native buckets or a content-addressed external object store are alternative realizations
when artifact size/access makes them useful. Keep export/restore of module files and source blobs
explicit; exporting bucket definitions does not establish that object bytes were copied.

The source makes these limits concrete: async retries have no backoff and exhausted entries are
removed after logging, so events are not automatically a durable external-job ledger. Use explicit
job/attempt outcomes if they trigger embedding or extraction. File functions call their object-store
controller immediately, and cloud rename uses copy/delete. When adopting buckets, write immutable
bytes and verify their digests first, finalize graph records second, and publish the pointer last;
orphan objects can be reconciled separately. Keep credentials in server configuration rather than
credential-bearing bucket URLs exposed through schema inspection.

Sources: skill real-time/core briefs and SB043/100/101/104;
[events](https://surrealdb.com/docs/reference/query-language/statements/define/event),
[buckets](https://surrealdb.com/docs/reference/query-language/statements/define/bucket),
[file functions](https://surrealdb.com/docs/reference/query-language/functions/database-functions/file),
[file implementation](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/fnc/file.rs)
and [event implementation](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/core/src/doc/event.rs).

### 3.7 SDKs, frontends and engines

| Capability | Recommendation and concrete value | Qualification/obligation |
|---|---|---|
| Stable Rust SDK, `SurrealValue` and typed record values | **Use now** as primary application integration | SDK feature selection affects compile cost; some features pull in core. Serde-only application structs need the documented wrapper/derive path |
| Remote server | **Use as the initial primary deployment** | Compiler, MCP and operator tools can share one owner of persistent storage; isolates database memory/failure from the compiler |
| WebSocket / HTTP / gRPC | **Use supported operations through the SDK; exploit gRPC streaming where useful** | Keep HTTP for documented export/import; use WS where LIVE is required. Protocol parity in small probes is not blanket equivalence of session/stream features |
| Typed incremental result streaming | **Design around** export/large scans using SDK `.stream_items()` where supported, or RPC `query_stream`/`query_cancel` | Rows can precede terminal statement failure. A receiving artifact stays provisional until success, or supports retraction; avoid buffering an entire database merely to recover atomicity |
| Sessions, attach/detach, reconnect/cancel | **Use to simplify transport ownership** | Separate pinned database handles; bound reconnect waits and reinstall LIVE subscriptions explicitly if used |
| Embedded mem | **Use for pure/local store controls** | Persistent-engine and authenticated-serving controls still exercise their real path; mem is not a durability oracle |
| Embedded SurrealKV/RocksDB | **Retain as viable packaging alternatives** | One owning process per directory; direct-core lifecycle/feature burden must buy something. No performance winner is established here |
| Direct `surrealdb-core` and custom engine interfaces | **Defer** unless stable SDK/server omits a required operation | Patch-unstable APIs add maintenance; do not acquire them merely to reach functionality already exposed by the server |
| Native MCP | **Use for controlled operator exploration; assess a bounded domain facade separately** | Generic database tools are not the API/evidence product. Confirm actual listed tools/permissions; stdio logs need the documented configuration |
| Generated GraphQL | **Defer to an actual UI/external read consumer** | Useful schema/filter/connection scaffolding, not automatic domain evidence envelopes. Mutations and ID conversion require explicit scope |
| PostgreSQL wire | **Optional tooling access** | Executes SurrealQL/GQL; it is not ANSI SQL, relational schema export, PostgreSQL extension compatibility or pgvector |
| ISO GQL | **Optional operator syntax** | Skill catalog exercises 24/38 constructs, not complete ISO GQL/Cypher; no benefit in porting native operations merely for syntax |
| TiKV, SurrealDS/Cloud, clustered deployment | **Defer to scale/availability needs** | Single-store TiKV receipts do not establish distributed behavior; engine availability, edition and operational model require separate selection |
| Browser IndexedDB/WASM and other-language SDKs | **Optional future offline/UI consumers** | Browser storage is not the primary artifact; Python/JS SDK API contracts were outside the Rust skill's qualification |

Sources: skill connecting/features/core/frontends and protocol catalogs; SB003/050–056/087/100–114,
SX001–006; [SDK documentation](https://surrealdb.com/docs/reference/rust),
[server deployment](https://surrealdb.com/docs/running/docker) and
[Studio](https://surrealdb.com/docs/explore/studio).

### 3.8 Remaining built-in families

The full discovery catalog is accounted for without requiring use of every function. `api`, `array`,
`set`, `object`, `record`, `schema`, `search`, `vector`, `type`, `value`, `string`, `bytes`, `encoding`,
`parse`, `math`, `count` and `not` are ordinary composition candidates. `time`, `duration`, `session`,
`meta`, `crypto` and `sequence` serve operational metadata or authentication where needed; generated
sequences and built-in hashes do not replace declared semantic identity. `file`, `http` and `eval`
have explicit effect/capability boundaries. `geo` has no present product consumer; `rand` belongs
only to declared sampling/fixtures, and `sleep` chiefly to controls. Time bucketing can support run
history without changing the evidence domain into a time-series application. Native ML and scripting
are separately assessed above. The catalog establishes discoverability, not full argument-contract
qualification for every entry.

### 3.9 What the 3.3 expansion changes in this design

**Interface-checked version distinctions, 2026-10-05.** Do not attribute every mature capability
above to the latest release. The important new or changed contracts include:

| 3.3 distinction | Consequence for the proposed target |
|---|---|
| Shared index doc IDs, bitmap fusion and indexed KNN prefiltering | Build fresh indexes on 3.3; co-design predicates with retrieval layout |
| Vertex INLINE edge/reference caches | Make low-degree evidence expansion an explicit layout candidate |
| Dependency-ordered field evaluation and final ASSERT pass | Derived field definitions can express dependencies; avoid loader-side ordering conventions |
| COMPUTED bodies cannot write; write-permission clauses can have effects | Keep read projections pure and permission logic intentionally small |
| Vector degeneracies now error instead of returning NaN; Jaccard is set-based | Validate vectors and qualify numerical semantics rather than inheriting older examples |
| GQL is enabled without its former experimental switch; eval remains separately gated | Explicitly configure exposed surfaces; optional syntax need not become a product endpoint |
| Module FROM/UNSIGNED syntax, Silo paths and changed Rust/core APIs | Use matching artifacts/SDKs; avoid older extension examples and unnecessary direct-core coupling |
| Memory accounting and request-timeout semantics changed | Configure against 3.3 behavior rather than carrying old operational values forward |

The [upgrade guide](https://surrealdb.com/docs/build/migrating/from-old-surrealdb-versions/32-to-33)
is a useful version map even though this pivot needs no old-data migration. The broader typed
transport/streaming and optimizer surfaces are assessed against their actual 3.3 interfaces above.

## 4. Ecosystem integrations: actual leverage and adoption limits — slot 8

**Interface-checked documentation and Proposed fit judgments, 2026-10-05.** An official integration
page can describe a maintained connector, a sample adapter or a walkthrough. These are different
adoption costs. No package in this section was installed or exercised during the review.

| Integration | Actual available surface and useful consumer | Recommendation |
|---|---|---|
| SurrealDB Studio and CLI | Official query/schema/record exploration; `surreal sql`, export/import and operational commands | **Use now** for diagnosis instead of building an operator UI; keep administrative and product identities distinct. [Studio](https://surrealdb.com/docs/explore/studio) |
| Beekeeper Studio | Native SurrealDB connection; its documentation labels support alpha | **Optional**, familiar browsing/querying only after checking needed features. [Guide](https://surrealdb.com/docs/build/integrations/database-clients/beekeeper-studio) |
| CocoIndex | Declarative table/relation/vector targets, source connectors and incremental reconciliation; its target manager can upsert/delete and manage schema | **Promising acquisition/staging integration**, not a concurrent writer of published graphs. Use only if it displaces real source synchronization work; graph admission and pinning remain ours. [Connector](https://cocoindex.io/docs/connectors/surrealdb/) |
| Kreuzberg / `kreuzberg-surrealdb` | Document extraction/OCR plus optional chunking, embeddings, generated schema and hybrid search | **Promising when adding PDF/Office/image evidence**. Prefer extraction outputs through admission; its automatic schema/embedding choices do not own canonical records. [Guide](https://surrealdb.com/docs/build/integrations/ai-frameworks/kreuzberg) |
| LangChain / `langchain-surrealdb` | `SurrealDBVectorStore`, metadata filtering and retriever integration; creates document tables/indexes and embeddings | **External-consumer option**, not the internal compiler/serving authority. Provide a domain-operation adapter when a consumer needs it. [Guide](https://surrealdb.com/docs/build/integrations/ai-frameworks/langchain) |
| LlamaIndex | Current guide implements a small custom SurrealVectorStore rather than documenting a shipped adapter | **Defer to a consumer**; budget adapter work and evidence-envelope mapping explicitly. [Guide](https://surrealdb.com/docs/build/integrations/ai-frameworks/llamaindex) |
| Agno, CAMEL, CrewAI, Google Agent, Hermes, Mastra, Pydantic AI, SmolAgents | Documented agent/memory/vector/tool integrations with differing packaging | **Consumer ecosystem**, no reason to add an agent framework inside deterministic compilation. Expose the existing bounded MCP/API operations; qualify a chosen adapter when requested. [Family index](https://surrealdb.com/docs/build/integrations/ai-frameworks/overview) |
| Dagster | Official guide supplies a custom Python resource using the SDK | **Defer orchestration replacement**; useful if multi-library scheduling becomes a real workload. It is not a zero-cost native scheduler adoption. [Guide](https://surrealdb.com/docs/build/integrations/ai-frameworks/dagster) |
| FastEmbed, Mistral, OpenAI and framework embedder adapters | Documented ways to produce vectors outside the store | **Do not replace the pinned Qwen specification incidentally**. Useful for a deliberate separately qualified model change or another consumer. [Provider index](https://surrealdb.com/docs/build/integrations/embeddings-providers/overview) |
| Airbyte | SurrealDB destination connector in the vendor organization | **Defer**; candidate for supplementary external evidence landing, not typed compiler publication. Exact 3.3 compatibility remains unqualified here. [Repository](https://github.com/surrealdb/airbyte-connector) |
| Fivetran | Documented destination configuration/managed transfer | **Defer** until managed data ingestion is needed; destination availability is not semantic mapping or runtime qualification. [Guide](https://surrealdb.com/docs/build/integrations/data-management/fivetran) |
| n8n | Official community node for self-hosted n8n; guide reports testing on SurrealDB 2.x | **Defer**; optional operator automation requires a 3.3 compatibility check. Do not insert a second workflow engine into publication. [Guide](https://surrealdb.com/docs/build/integrations/data-management/n8n) |
| Qyrus | Documented data generation/comparison/validation integration | **No current adoption**; independent domain fixtures/certificates are a better fit than generic data-quality orchestration. [Data integration index](https://surrealdb.com/docs/build/integrations/data-management/overview) |
| Better Auth | Adapter targets Better Auth 1.6.x / SurrealDB 3.1+; guide describes schema regeneration and limited compensating rollback | **Defer to a multi-user application**. Product publication must not inherit adapter compensation semantics; native service authentication suffices initially. [Limits](https://surrealdb.com/docs/build/integrations/authentication/better-auth/transactions-and-limitations) |
| Agent rules/editor tooling | Documentation aids for writing SurrealQL and operating tools | **Use as developer assistance**, never compiler/evaluation inputs or evidence of product behavior. [Integration index](https://surrealdb.com/docs/build/integrations) |
| Community schema/migration tooling | Possible convenience around schema evolution; outside the skill's API qualification | **No migration framework now**: generate a fresh snapshot realization and discard old design-phase data. Revisit for durable external schemas with a real upgrade consumer |
| Docker/managed cloud/Kubernetes | Official server packaging and deployment choices | **One managed local server now**; distribution/HA later when its operational value outweighs its state and monitoring burden |

This is a technically broad adoption policy, not a license filter. Most peripheral integrations
are deferred because the project already owns deterministic acquisition/analysis and has one
operator. Their strongest future role is adding sources or consumers around the admitted graph.

## 5. Second-order benefits, alternatives and total machinery — slots 8–9

**Proposed.** A reusable admitted graph gives these concrete benefits:

- A petgraph adapter streams nodes independently of edges, preserves isolates/parallel assertion
  IDs, and constructs dense indices once for a declared topology. SCC, centrality and other consumers
  reuse that projection; new algorithms need not rerun analyzers or decode hundreds of tables.
- A Neo4j export can use native relationships for binary assertions and reified records for
  n-ary or assertion-referencing structures. Nested values, null/missing distinctions, parallel
  relationships and heuristic outputs have an explicit destination mapping. GDS becomes an optional
  specialized consumer, not another canonical store.
- A PostgreSQL or Arrow/DataFusion export can expose selected typed facts for joins/reporting.
  It need not reconstruct the legacy 878-table layout. PostgreSQL wire access is unrelated to this
  semantic relational lowering.
- Bounded subgraph extraction provides reproducible debugging inputs with original IDs and premises.
  Include coverage boundaries and dependencies; a truncated neighborhood is not automatically a
  standalone valid compiler input.
- Database functions centralize coarse operations for MCP, CLI and a future UI. Generated GraphQL
  or custom APIs can reuse them instead of each transport reconstructing evidence paths.
- Snapshot-local indexes/functions permit query-plan and search-policy experiments over admitted
  content without repeating extraction. Changing approximation or ranking is an explicit realization
  or operation-policy change, not silently changed semantic evidence.
- Inline adjacency and bitmap filtering can reduce the need for custom lookup caches, and native
  diagnostics can explain when the engine falls back. These are plausible operational benefits,
  not a measured production speedup.

| Alternative | What it buys | Cost and decision |
|---|---|---|
| SurrealDB only as graph persistence; all host operations retained | Small initial adapter change | Leaves N+1 transport, record stitching and possible duplicate search infrastructure; insufficient ambition for the requested target |
| Database owns every analysis and workflow | Maximum apparent consolidation | Rewrites mature Rust semantics, imports module/event/resource complexity and risks one opaque transaction; reject as a universal rule |
| Coarse native operations plus Rust semantic kernels | Narrowing/hydration near data, explicit semantics and local tests | **Recommended initial composition**; no generic backend abstraction or new workflow framework required |
| Same composition plus selected Rust/WASM kernels | Fewer bulk transfers/process boundaries for a proven kernel | Promising optional extension; adopt per kernel after source/build/resource suitability, not as a prerequisite |
| Embedded primary instead of a server | Single-process packaging, no network hop | Credible alternative for a packaged app; shared compiler/MCP/operator use and resource isolation favor a server initially |

Store-independent benefits remain compile/admit-first, semantic identities, one persistence
reconciliation, certificate verification and destination-neutral projection contracts. SurrealDB
adds the convenient combination of native adjacency, rich values, integrated search, procedures,
typed transports and operational tooling. These reinforce the hard pivot; they are not reasons to
add every available feature.

## 6. Change, failure and assurance scenarios — slots 5, 10

These are **static Proposed walkthroughs**, not executed tests. They identify the acceptance
questions for later implementation without requiring a new pre-design benchmark campaign.

| Scenario / kind | Ownership and expected propagation | Revealing failure / settling evidence |
|---|---|---|
| Add a binary assertion kind / domain extension | Rust criteria and genuinely new producer/consumer semantics change; existing physical family and loader can remain | Parallel assertions and source-specific qualifications survive; no per-kind lifecycle/grant/stage machinery |
| Add a n-ary configuration interaction / domain concept | Explicit assertion and participant roles, not an invented binary edge | Scope/role information survives serving and export; changed meaning legitimately touches consumers |
| Replace host packet hydration with a function / mechanism | One operation contract and derived realization change; MCP keeps its envelope | Same original evidence and unknown states, fewer round trips, no hidden cross-snapshot or permission behavior |
| Use a Rust/WASM reduction / mechanism | Shared kernel, artifact/environment identity and resource boundary | Shuffled input and repeated calls preserve declared result; no retained module state or per-edge host calls |
| Search duplicated examples with selective requirements / composition | Occurrence eligibility precedes channel limits and family/member collapse | Duplicate examples cannot outvote distinct members; tied cutoffs and winning witnesses are reproducible |
| Individually supported requirements on incompatible overloads / semantic composition | One selection classifier retains variant/binding/configuration witnesses and joint status | Independent positive matches cannot become a jointly supported invocation |
| Empty result under missing index/permission/coverage / failure | Adapter and operation classify cause before presentation | Empty complete, unavailable, refused and unresolved remain distinguishable |
| Cycle/high fanout/deep source chain / bound | Explicit frontier, visited assertion IDs, work/byte limits and stop reason | Engine depth limit/timeout does not become absence; no all-walk materialization |
| Batch interruption or lost commit acknowledgement / lifecycle | Private target, deterministic IDs/chunk identity, reconcile uncertain writes | Never blindly repeat non-idempotent writes; incomplete target cannot become selected |
| Index failure or function/module drift / lifecycle | Realization manifest and publication readiness | Valid rows alone do not authorize serving an incomplete or altered realization |
| Projection to Neo4j/PostgreSQL / mechanism substitution | Named mapping, original IDs and declared losses; no back-write | Isolates, parallel assertion identity, n-ary roles and unknowns remain recoverable or explicitly refused |
| Restore/export source artifacts / recovery | Database state plus external objects/definition artifacts | A bucket definition without its bytes cannot pass publication restoration |
| New library/analyzer/spec / instance or semantic revision | Pinned capture and affected model/representation revision; old data may be rebuilt | No mixing query/document vectors, source contexts or evidence revisions; references remain out of inputs |

Qualification should challenge the operation boundaries using small independent known answers,
then exercise representative native store/MCP journeys and a real corpus under the authorized
implementation scope. Avoid reproducing every legacy relation-level control in a new backend.
Keep controls for actual semantic distinctions, codec boundaries, publication and partial results.

## 7. Findings, independent judgments and bounded decision — slots 6–7, 12

The following findings constrain proposed adoption, not claims that SurrealDB has already been
integrated. Their corrections are incorporated in this review's recommended scope.

<a id="F01"></a>

### F01 — Native fusion alone does not implement member/family ranking

**Interface-checked diagnosis; Proposed correction.** In 3.3.0 `fnc/search.rs`, RRF accumulates
every occurrence of an ID, compares heap entries by score only, iterates a HashMap and merges fields.
Duplicate units can contribute extra votes; tied candidates can disappear before a later sort;
merged metadata cannot establish a winning channel witness. This affects DP-02/08/11/15,
FP-04, A2, G2/G6 and CI-09/11.

The retrieval owner must define the contribution key, collapse, canonical ties and witness inventory
once. Use native fusion only with the adaptations in §3.3, or retain the small authoritative fold.
Closure: duplicate-occurrence and tied-cutoff independent cases produce the declared member results
and witnesses through the real operation. Current execution disposition belongs to the
[replacement coordinator](../../plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities).
Raw native top-k fusion is excluded from supported scope; the correction remains Proposed.

<a id="F02"></a>

### F02 — Custom API authorization and transactional failure differ from ordinary functions

**Interface-checked diagnosis; Proposed correction.** After checking API permissions, 3.3 API
invocation disables record-permission checks; returning an HTTP error status does not itself fail
the datastore operation. A handler can therefore bypass an assumed table filter or commit writes
while returning an apparent error. DP-03/18/19, FP-05, G3/G4/G5 apply.

The operation owner must generate bounded, authorized handlers and use actual execution errors for
rollback. Prefer ordinary function invocation until an HTTP consumer exists. Closure: allowed/denied
snapshot identities and write-then-fail cases through the actual endpoint. The
[replacement coordinator](../../plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities)
owns conditional disposition, triggered by DEFINE API adoption. Relying on table permissions or
response status as its security/rollback contract is excluded.

<a id="F03"></a>

### F03 — Pinning graph rows alone leaves executable serving behavior mutable

**Proposed design correction.** Moving selection/hydration into stored functions means code,
analyzers and module artifacts join the snapshot's serving dependencies. Existing content-only
publication descriptions are insufficient if those can change independently. DP-04/09/19/24,
FP-02/04/05, A2, G1/G5/G6 and CI-13 apply.

The realization/publisher owner must bind operation definitions, index/analyzer specs and optional
module bytes to the served handle. Closure: an attempted mismatched realization is refused or exposed
as a deliberate new realization, never silently substituted. Current execution disposition belongs
to the [replacement coordinator](../../plans/graph-native-pivot-plan_2026-10-05.md#7-sole-finding-disposition-and-optional-capabilities).
§2.2 supplies the Proposed remedy; planning does not establish implementation or closure.

| Architectural judgment | Proposed target verdict | Reason |
|---|---|---|
| A1 Localize change | Satisfied in the recommended scope | New assertion semantics stay with their owner; physical indexes/transports and projection destinations have distinct lowerings |
| A2 Encode domain meaning | Satisfied in the recommended scope | Entities/assertions/occurrences, coverage, operation policy and evidence closure govern admission and execution; native defaults cannot redefine them |
| A3 Extend through composition | Satisfied in the recommended scope | Functions, graph/search operators, shared kernels and projections compose without another semantic authority |

FP-01/02/03/04/05/06 are **satisfied for these Proposed operation boundaries**. DP-13/14/16 are
satisfied by the capability-first selection and explicit deletion opportunities; DP-20/23 retain
implementation obligations rather than asserting unmeasured resource ceilings. Optional modules,
file-backed source publication and new frontends are not accepted implemented behavior.

| Gate | Target-review verdict | Evidence and boundary |
|---|---|---|
| G1 Authority | Pass at Proposed scope | One Rust semantic owner, generated/owned realizations and pinned function/artifact identity |
| G2 Fidelity | Pass at Proposed scope | Typed assertion/role/occurrence/coverage distinctions; raw fusion and empty-result inference excluded |
| G3 Validity | Pass at Proposed scope | Admission, checked writes, reconciliation and index readiness have explicit enforcement owners |
| G4 Hidden behavior | Pass at Proposed scope | API/event/HTTP/module effects explicit; pure operations do not inherit ambient state |
| G5 Consistency/recovery | Pass at Proposed protocol scope | Private immutable target before short visibility transition; incomplete streams and failed writes remain unpublished |
| G6 Transformation/reuse | Pass at Proposed scope | Projection/fusion/spec semantics declared; no inferred lossless export or all-purpose WASM parity |
| G7 Truthful capabilities | Pass for this assessment | Version/backend/edition and evidence boundaries explicit; no product throughput or quality claim |
| G8 Library leverage | Pass for selection | Native capabilities evaluated against operations and total burden, including conditional and deferred uses |
| CI-G1 Fidelity | Pass at Proposed scope | Heuristics do not establish requirements; incomplete/unknown states survive retrieval |
| CI-G2 Evidence closure | Pass at Proposed scope | Claims and original evidence resolve within pinned content and realization |
| CI-G3 Evaluation integrity | Pass for this review | Skills/integration examples inform engineering only; no new compiler/evaluation input or parameter tuning |

These passes judge the specified design, not runtime correctness. Persistence throughput, filtered
retrieval quality/plan cost, work-budget enforcement and restore behavior in the new implementation
remain **unresolved/not_run**. The enclosing replacement cannot be called implemented, Tested or
production-qualified on this evidence. Optional WASM/file/API paths require their own named checks
only if selected; their deferral does not block the baseline design.

## 8. Authority changes and next decisions — slot 11

This review is evidence, not a new architectural authority. No accepted ADR or production contract
is changed by publishing it. The next architectural decision should adopt the graph target and
these operation boundaries together, followed by one direct replacement plan.

| Required change | Owner / route | Existing obligations and closure |
|---|---|---|
| Replace PostgreSQL layout/lifecycle and host-only serving assumptions | ADR + DESIGN §B2/B3/B7/B10/B12/B13 and focused storage/serving owners as applicable | Preserve required outcomes, remove obsolete mechanisms; do not rewrite accepted ADR bodies |
| Define admitted graph and realization manifest | Rust model + publisher plan | Incorporate GN01/GN02 and this review F03; close through actual boundary evidence |
| Select initial coarse native operations/search representation | Serving/retrieval plan | Incorporate F01/F02 and original query-bound obligations; retain declared bounded candidate semantics |
| Retire previous coupled store code | Direct cutover plan | Carry surviving [earlier F01–F14](design_review_surrealdb-graph-store_2026-10-05.md#11-authority-changes-and-dispositions-slot-11) and [GN01–GN03](design_review_graph-native-target_2026-10-05.md) into one execution ledger |
| Optional Rust/WASM, buckets, external consumers | Named consumer and owning operation | Deferred until reduced machinery or an agreed new operation justifies qualification; no mandatory subsystem projects |

The first consequential implementation choice is the admitted graph plus snapshot realization
boundary, followed immediately by one representative database retrieval/evidence operation. Choose
index/layout details with that operation, rather than installing a generic graph and postponing
its access patterns. SurrealDB-specific benefits are sufficient to proceed now; broad new probes
would not change that recommendation. The eligible-occurrence KNN plan is the most likely narrow
question to change a physical layout choice, not the store verdict.

## 9. Evidence and source map — slot 10

**Assessed 2026-10-05.** No new runtime probe was needed to make the recommendations. Existing
receipts retain their original scope; their counts do not establish this product's scale or quality.

| Evidence | What was inspected / qualification boundary |
|---|---|
| `.claude/skills/neo4j-surrealdb/SKILL.md`, `surrealdb/GUIDE.md`, `reference.md` | Comprehensive skill entrypoints, source precedence and limits; SDK/types/derive/core/MCP 3.3.0 |
| `surrealdb/content/capabilities/`, routes/topics/coverage and catalogs | All 16 briefs and catalog families; individual substantive contracts followed where recommended use depends on them |
| Pinned runtime | `surrealdb/surrealdb` v3.3.0, commit `238bfeb11f5725bebed370167656748df8067595`; image digest `sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20` |
| Pinned docs | Commit `82b7ac1935fcbb400e80cf357dd68f47182bad26` from main on 2026-10-05; source/probes take precedence over newer prose |
| Existing skill receipts | 90 behavior and 6 compile probes, typed values/DDL/functions/frontends and upstream language tests across recorded backends; not rerun here |
| Focused source | Search fusion, custom function/API execution, module runtime/state/capabilities, transaction handling; exact relevant source links are beside recommendations |
| Context7/current official docs | Resolved `/surrealdb/docs.surrealdb.com`; focused observability/export queries plus official ecosystem pages and connector references |
| Product owners | API/evidence §14; synthesis/serving; semantic model/storage/analytics routes; exact embedding spec; preceding graph-native and store reviews |
| Comparison/benchmark | Comparison matrix and benchmark protocol inspected; only a smoke receipt was present in the inspected result directory. No completed scale/latency comparison is asserted |

Some brief/comparison text still carries earlier GQL counts and earlier change-data coverage.
This review uses the expanded catalog/probe scope instead. A catalog call rejected for wrong
arguments proves discovery, not correct-use semantics. Multi-store TiKV, distributed managed
operations, other-language SDK parity, full WASI dependency compatibility and end-to-end external
connector behavior were not qualified.

3.3-specific planning uses the
[3.2→3.3 guide](https://surrealdb.com/docs/build/migrating/from-old-surrealdb-versions/32-to-33):
shared index doc IDs and bitmap/prefilter plans, streaming/protocol surfaces, module syntax and
runtime API changes matter. New targets are built on 3.3; no old-index or old-datastore migration
is needed for this cutover. Feature adoption is tied to the exact release/source, not an assumed
announcement date.

The scoped decision is **Accept scoped**: adopt the native operation/query/search architecture
and the capability priorities above. Keep optional integrations optional, preserve the small
semantic controls that establish meaning, and qualify the actual replacement when it exists.

**Independent review, 2026-10-05:** a fresh design reviewer inspected the combined draft, current
product/model owners and decisive fusion/API source, and returned **Accept scoped**. Its material
corrections are incorporated: writer draining and executable sealing; same-context joint requirement
witnesses; mandatory packet integrity under budgets; and original-byte lowering into SurrealDB.
The reviewer independently judged A1–A3 satisfied and the gates passing at the Proposed scope,
without claiming implementation or runtime qualification. No remaining design-blocking disagreement
was reported.

Documentation verification for this review: `just docs-check` **passed**, 2026-10-05, with 308
canonical pages and zero link errors. Product checks, `just qualify` and real-library compilation
were **not_run** because this work changes review documentation only.
