# Plan: PostgreSQL deployment and integration

**Revised 2026-09-27. PG0–PG7 are implemented, deployed and accepted; PG8–PG17 are planned.**
This revision incorporates the [expanded architecture review](../design_review/reviews/design_review_postgresql-expanded-architecture_2026-09-27.md).
It selects 1024-dimensional embeddings, PostgreSQL relational serving and pgvector, Rust-owned
serving queries, pgpq ingestion and qualified DataFusion federation as the next integrated scope.
The old serving/COPY/provider consumer triggers are met by this selected work; they are not
additional prerequisites. Implementation and expanded deployment have **not started**.

This document owns PostgreSQL packages, contracts, deployment, acceptance and conditional library
adoption. The [forward plan §3.4](behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream)
owns coordination with Stage 3–5; its [§6.1](behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
is the **single current disposition owner** for PGS/F01–F04, PGK/F01–F02 and PGE/F01–F05.
The reviews retain their original finding meanings and dated evidence; this plan does not close them.

Sources: [storage review, PGS](../design_review/reviews/design_review_postgresql-storage_2026-09-27.md),
[stack review, PGK](../design_review/reviews/design_review_postgresql-stack_2026-09-27.md),
[expanded review, PGE](../design_review/reviews/design_review_postgresql-expanded-architecture_2026-09-27.md).
[ADR-0065](../adr/0065-postgresql-services-and-vector-receipts.md),
[ADR-0066](../adr/0066-pinned-serving-and-postgresql-vector-receipts.md) and
[ADR-0067](../adr/0067-canonical-snapshots-with-consumed-vector-receipts.md) govern the implemented
baseline. The expanded target below is **Proposed** until PG8 records the successor decisions and
owning design changes. The operator's **1024 dimension choice is settled**, with no further
4096-versus-1024 fidelity or analytics-quality gate.

## 1. Outcome, scope and baseline

Make PostgreSQL the transactional and online serving store while retaining Arrow/DataFusion and
Delta for canonical analysis, validation and reproducible publication. Build immutable relational
serving projections from published snapshots, retrieve candidates in pgvector and hydrate complete
evidence through one Rust repository. Python keeps FastMCP transport and the existing lexical/fusion
policy; the native semantic executor remains pure. PostgreSQL is not the canonical fact store.

| Scope | Planned result | Boundary |
|---|---|---|
| Embeddings | One production 1024-dimensional spec for E0/kNN, operation views, brief documents and query vectors | Same pinned model; explicit MRL prefix/normalization/launch contract, new spec identity and rebuilt outputs. No permanent 4096 analytics/1024 search split |
| Cache and operational history | Preserve PG0–PG7's exact winners, receipts, attempt/events and canonical discovery | Cache remains exact bytes; similarity never selects cache hits. Operational state does not redefine publication |
| Relational serving | Immutable generation-qualified operations, paths, facets/status, parameters, behaviors, brief/assertion/support relations and artifact references | Exact matching, unknown coverage and complete evidence closure; no mutable patching of published semantics |
| Retrieval | pgvector `vector(1024)`, exact reference route and explicit HNSW discovery profile | Approximate candidates never limit exhaustive `find_operations` or compiler kNN. BM25/RRF/name promotion keep their declared owner |
| Rust/Python boundary | Coarse asynchronous Rust query/hydration operations, one serving pool lifecycle | Python is not a second SQL query catalog; native semantic evaluation does not perform database I/O |
| Bulk and analytical access | pgpq→SQLx COPY imports; qualified PG provider/federation over declared read views | Domain schema normalization, restricted predicates and a coherent read view; no arbitrary remote query surface |
| Operations | Explicit extension/migrations, import/readiness/selection/reconciliation, backup/rebuild/rollback and bounded resources | Existing PG18 cluster; no automatic migration on reads, extra cluster, replica, queue or network endpoint |
| Later product work | Stage 4 concept projections and explicitly frozen review-event revisions | TOML/definition AST and semantic kernels retain authority; full authoring workflow and other transports remain §7 consumers |

**Implemented baseline, source-inspected 2026-09-27 at `219070e`:** SQLx 0.9, local PG18.6,
migrations 202609270001/2, one shared cache writer, `used_embeddings`/`embedding_uses`, exact
snapshot-local receipts, operational CLI and reconciled discovery. Compiler 106/extractor 33,
catalog 7/template 20 and bundle FORMAT 10 are current. Serving still reads a complete file generation
and materializes Python maps/dense vectors; the live spec still requests full 4096 output.
[Initial qualification](../design_review/evidence/2026-09-27_postgresql/README.md) is historical
acceptance of that scope, not evidence for PG8–PG17.

**Reviewed target evidence, 2026-09-27:** the supplied bridge revisions resolve with the repository's
DataFusion 55.1/Arrow 59.3 family, SQLx and exact delta-rs revision. Selected provider/federation/Delta
compilation and real PG18.6 COPY/representative provider reads passed in an isolated workspace.
Native ADBC, expanded MCP serving, pgvector runtime and full domain/pushdown conformance are not
qualified by those probes. See the [expansion evidence](../design_review/evidence/2026-09-27_postgresql-expansion/README.md).

**Migration baseline:** preserve the current binary, lockfiles/configuration, canonical stores,
file generations and evaluation artifacts before PG9/PG10 change production identities. Keep the
older PG0–PG7 rollback assets while they have a consumer. Use fresh, separately named stores and
results for 1024 and projection-format migrations. Existing 4096 artifacts are never relabelled or
rewritten; use a compatible retained reader for rollback when a new reader rejects their format.
This preserves evidence without creating an indefinite historical-schema support promise.

## 2. Stack and feature policy

**Proposed selection, based on the dated review; actual dependency edits belong to PG10.** Keep
exact DataFusion 55.1 and Arrow 59.3, the existing delta-rs pin and one object_store family. The
supplied lower minor requirements are compatible ranges, not a reason to downgrade root pins.
The [pins page](../pins.md#postgresql-services-adr-0065-verified-2026-09-27) continues to describe
what is installed; PG10 updates it only after resolution/build verification.

| Layer | Selected role / package | Constraint and qualification |
|---|---|---|
| SQLx 0.9.0 | Existing app driver, pool, migrations, static/runtime SQL and binary COPY; `default-features=false`, `postgres`, `runtime-tokio`, `macros`, `migrate`, `tls-rustls-ring-native-roots` | One app transaction/migration owner. Keep offline `.sqlx` workflow, fresh-schema checking and no ambient build DB |
| Tokio 1.53.1 and Rustls | Existing runtime/TLS route | Bounded pools/tasks and deadlines; no runtime per request; explicit remote trust/hostname verification |
| sqlx-cli and Testcontainers | Existing pinned CLI and testcontainers-modules 0.15.0 `postgres`/core reexport | Retain installed PG18 patch/digest checks; add separately pinned extension-bearing test image and extension version assertion |
| pgvector Rust 0.4.2 + extension 0.8.6 | SQLx adapter (`sqlx` feature) and PG `vector(1024)` exact/HNSW search | Client, server extension and test image have separate pins. Retain float32; no unselected quantization or IVFFlat path |
| pgpq 0.12.0 | Arrow 59 batches→PG binary COPY, transported by existing SQLx connection | No second application client for COPY. Vector import uses validated `real[]` staging→`vector(1024)` or the SQLx vector adapter |
| datafusion-federation 0.5.7 | Qualified remote plan composition with the PG provider | Not a shared-transaction mechanism; qualify optimizer rewrites as well as table scans |
| datafusion-table-providers-postgres | Maintained immutable-revision fork based on `CaptainEureka/datafusion-table-providers@33095588fcdd17301a5d1c340dcd66cd60e41ec8` | Record upstream base, local patch, final revision and features. Its internal Rust-Postgres pool is a separate bounded read-adapter pool, not a second application write API |
| pyo3-async-runtimes 0.29.0 | Async Rust serving operations→Python awaitables with existing PyO3 0.29.2 | One configured process runtime, lifespan-owned SQLx pool. Qualify cancellation, Python 3.14/native build and connection reuse |
| Existing bm25s/native executor | Keep lexical scoring, RRF/name-promotion owner and pure native semantic evaluation | No PostgreSQL `ts_rank` substitution, duplicate fusion owner or SQL-side behavioral interpreter |

Optional compatible paths stay explicit rather than becoming default dependencies:

| Candidate | Capability direction and pin from the review | Adoption owner |
|---|---|---|
| ADBC core/driver-manager/FFI 0.24.0; r2d2_adbc 0.3.0; ADBC table provider at the same migration revision | PG native driver→Arrow/DataFusion; C ABI removes Rust Arrow-major coupling, not semantic conversion. Anchor broad ADBC dependencies to one resolved 0.24 family | F11; pin native PG driver artifact separately and qualify synchronous/FFI lifecycle |
| ADBC 0.25-dev `c942d481c6e083040c68676e3dd454dad89503e9` | Compatible Arrow>=58,<61 route | F11 only for an actual feature absent from sufficient 0.24 |
| datafusion-postgres family `eda0da032ed8d6003b5041fce67c1e5b2f101876` | PostgreSQL wire **server over DataFusion**, with arrow-pg, datafusion-pg-catalog and datafusion-pg-functions | F12; not the PostgreSQL storage reader |
| adbc-driver-datafusion 0.27.0 source `3d24e0f3ad8bf914b9d2a48d0151fc313b8ae28c` | Embedded DataFusion **exposed through ADBC** | F12; not the PostgreSQL ADBC driver |
| Psycopg 3 + psycopg_pool; optional SQLAlchemy | Direct Python-owned relational workflow; select one pool and retain Rust migration history | F6; main serving uses the Rust repository |
| SeaQuery 1.0.2 + sea-query-sqlx 0.9.1 | Substantive typed dynamic query composition | F4; simple bound filters stay SQLx-owned |
| tokio-postgres 0.7.x / Cornucopia; pgrx | Alternate application query/driver architecture; separate server-side extension respectively | PGK/F01 on an actual driver switch; F8 for pgrx. The provider's internal driver does not select Cornucopia |
| connector_arrow 0.12.2 | Arrow 58 family | Excluded from this family; use selected compatible paths |

Only enable features with a selected consumer. Check root resolution/builds, not only probe
manifests. Semantic IDs stay fixed-length domain bytes, codebooks checked numeric values, and
spec JSON uses its canonical encoder. SQLx macros check SQL/types, not semantic invariants.
Secrets and live schema discovery never enter build scripts. Provider configuration adapts the
protected config to explicit fields/libpq keyword syntax; do not pass SQLx URLs blindly or expose
provider options to semantic consumers. Coordinate both pools, TLS and sanitized diagnostics.

## 3. Ownership and persistence contracts

**Implemented owners and Proposed extensions:**

| Owner | Responsibility | Consumer and deletion boundary |
|---|---|---|
| `cpg-schema` | Canonical facts, identity, coverage, codebooks, receipts, declared serving schemas and validation rules | PG DDL/projections/codecs derive from this meaning; catalog inference is not a second semantic authority |
| `cpg-core::postgres`, migrations and `lctx` | Existing cache/operations; extend config, roles, migrations and operator commands | One migration history; no generic StorageBackend framework or duplicate app driver |
| Embedding spec, Rust/Python clients, launcher | One 1024 transformation/request/launch contract and exact receipt handling | Remove new-generation 4096/no-dimensions assumptions; retain immutable historical identity |
| Rust serving projection writer | Read a published Delta snapshot at recorded versions, normalize/export/import and validate PG relations/artifacts | Explicit rebuildable projection; no distributed commit or live cache dependency during replay |
| Rust serving repository | Exact universe selection, batched hydration, complete vector ranks and bounded ANN candidates | One query owner for CLI/MCP; remove production Python full-table maps and dictionary selectors after parity |
| `lctx_mcp` and async binding | Transport/types, lifespan and current lexical/fusion policy | Coarse requests/results, not arbitrary SQL or one call per witness; no direct Python DB pool |
| `lctx_semantics` / pure kernel | Bounded evaluation of validated immutable native input | No SQLx, network or pool in the semantic kernel. A separate effect-owning module/crate holds serving I/O |
| DataFusion PG adapter | Restricted read-only views, domain normalization, qualified pushdown and coherent read scope | Compiler computations stay DataFusion/Arrow; no speculative general query federation service |
| Stage 4 registry/review owners | Authored definitions/events and explicit frozen inputs; derived concept serving projections | TOML/AST/condition semantics remain canonical. New evidence family uses declared projection/closure, not bespoke tool SQL |

### 3.1 Application schemas

- **Implemented `lctx_cache`:** canonical specs and insert-only exact embedding values keyed by
  `(spec_hash,input_hash)`, versioned Float32 byte codec and value digests. Existing SELECT/INSERT
  privileges and collision/winner checks remain. New 1024 values receive a new spec key; do not
  cast 4096 cache rows in place or confuse derived pgvector values with canonical cache bytes.
- **Implemented `lctx_ops`:** attempts/events and reconciled canonical snapshot/file-generation
  discovery. Missing completion remains unfinished with unknown terminal outcome unless explicit
  evidence establishes interruption. Operational state stays outside semantic digests.
- **Planned `lctx_serving`:** immutable projection manifests plus generation-qualified normalized
  serving relations and vectors. Separate loading/validation/ready state and operator selection
  pointer from immutable content. Runtime readers cannot mutate content or read partial imports.
- **Conditional authored workflow:** F1 adds revision-bound append-only events and explicit
  conflict/idempotency rules. A selected frozen revision enters compiler inputs; published facts
  never consult a mutable latest review state.

SQL migrations own operational physical detail; semantic declarations own Arrow/domain meaning.
Avoid JSONB for relationships the query or validator must understand, and never hash its database
rendering instead of the canonical encoder. Backups distinguish reconstructible cache/projections,
canonical replay inputs/artifacts and non-reconstructible operational/operator history.

### 3.2 Exact-vector receipt and cache protocol

1. An attempt consults its retained values first, then reads missing keys from PostgreSQL in
   bounded batches. Validate retrieved values and spec metadata before use.
2. Admit uncached request text through the same tokenizer/spec rules. Compute embeddings
   outside any database transaction/leased connection. Insert only validated values with
   `ON CONFLICT DO NOTHING`, committing successful batches even if later embedding work fails.
3. In a **subsequent statement at READ COMMITTED**, read every requested key and return committed
   winners. Check completeness and preserve request ordering. Do not assume an insert-or-select
   CTE's statement snapshot can see a concurrent winner. Reconcile ambiguous commits by key on
   a new connection before bounded retry. SQLSTATE classification belongs in this owner.
4. Retain the exact committed bytes actually consumed by every embedding caller. The same key
   within an attempt has one value. If cache restoration/recreation yields a conflicting value,
   reuse the already-frozen attempt value or fail explicitly; never silently mix values.
5. Persist the union as snapshot-qualified `used_embeddings` exactly once, before shared
   validation and the `snapshots` append. Include E0/analytics-only keys as well as serving
   documents. A pending attempt's retained data may spill to its owned workspace under a
   memory budget; finalization deduplicates/sorts without re-reading the live cache.
6. Hash the actual values through a versioned canonical Float32 byte representation and a
   sorted receipt recipe owned by `cpg-schema`. Include the receipt digest in content identity;
   spec/key-only identity is insufficient when served vectors can differ. Store the canonical
   embedding spec with the snapshot. Declare compiler/schema and any required bundle-format
   changes; no backend OID, cache version or server identity becomes a semantic ID.
7. Shared validators prove unique keys, legal spec/shape/value/digest, same-snapshot identity,
   and coverage of every required consumer key. Bundle construction reads only this relation.
   No-embedding snapshots have the declared empty relation/receipt behavior; fake and live
   specs remain distinct. A full cache hit continues to require no live embedder.

[PostgreSQL 18 transaction visibility](https://www.postgresql.org/docs/18/transaction-iso.html)
supports the separate read in step 3. Initial cache maintenance offers no online eviction or
replacement. Later retention must preserve in-flight attempts; snapshot replay is independent
because values have already moved into immutable canonical evidence.

### 3.3 Standard 1024 embedding migration

The operator has selected 1024 for all new production embeddings. PG9 implements prefix selection
followed by L2 normalization in the pinned service, records transformation/admission in the hashed
spec, and makes both clients send the matching dimensions. Keep query/document templates, token
admission, float32 output and model revision unless an independently scoped change requires otherwise.
The pinned vLLM route requires a spec-derived Matryoshka override; changing a JSON width alone fails.

Regenerate E0/kNN/centroid/community inputs, operation views, brief vectors and query clients under
the new identity. Existing analytic thresholds, seeds and model semantics remain fixed; no tuning
against gold or heldout data. Retain exact cache winners and snapshot-local receipts. A new launch
checks request/response dimensions, unit norms, conformance and replay as implementation correctness;
it is not another task-quality or 4096-versus-1024 comparison. Fake specs remain distinct and their
known answers change only where their declared contract changes.

### 3.4 Immutable serving publication and recovery

1. Resolve a published canonical snapshot through its exact Delta versions and snapshot predicate.
   Freeze a projection input manifest; never use a raw Parquet scan or the live cache.
2. Compute a **serving projection identity** from canonical snapshot/content identity, projection
   format/definition digest, compiler/catalog/kernel/spec identities and the deterministic relation
   and native/lexical artifact manifests. Store full digests; any short display key has collision
   checks. A new projection format over the same snapshot is a new projection, not a new snapshot.
3. Import into an invisible loading generation. Keys, joins and foreign keys include its generation;
   preserve source IDs, modalities, verdicts, unknown reasons, conditions and parallel supporters.
   Bound COPY batches and transaction duration. Retried batches compare identity/count/digest;
   conflicting content fails rather than updating an existing immutable generation.
4. Store native/lexical data as immutable, digest-checked artifacts referenced by the manifest,
   not a second database blob store. Import/shared validators verify exact schemas, coverage,
   vector spec/width/norm and whole support closure. Build ordinary indexes on unpublished data;
   validate counts/digests/index readiness and artifact availability before visibility.
5. A final short transaction marks the whole projection ready and, only when explicitly selected,
   updates the operator's current-generation pointer. Delta publication has already happened;
   PG import failure never unpublishes it. An ambiguous PG commit is reconciled by identity/readiness.
6. Each server resolves the selection once and pins a ready generation and supported retrieval
   profile for its lifespan. Every query/cursor/hydration includes the relevant identities.
   Retrieval profiles are separately versioned, generation-qualified records: a later HNSW profile
   cannot become selectable until its indexes and controls are ready, and never mutates canonical
   content or an existing exact profile. New selection affects new servers; missing required artifacts,
   rows or incompatible schemas fail explicitly, with no mid-request switch or silent file fallback.
7. Retain all ready projections and artifacts initially. Cleanup may remove only unpublished,
   owned attempts after proving they are not in use. Before enabling ready-generation deletion,
   implement reader protection covering DB rows and artifacts; a stale heartbeat alone is not proof.
8. Restore durable events from protected backups; reconstruct serving/discovery from canonical
   snapshots and validated artifacts. Check ready references after restore before serving. Rollback
   selects a compatible prior binary/config and generation; never reverse-migrate canonical facts.

Importer, migration, cache/ops application and read-only serving roles have separate grants. Schema
startup checks and status commands verify extension, migration and projection compatibility without
performing DDL. PostgreSQL readiness complements, rather than replaces, canonical Delta publication.

### 3.5 Query ownership, fidelity and resource contracts

- **Exact operations:** preserve the complete kind/path universe and per-term match/no/open logic,
  including unknown totals/truncation, five verdicts, alias resolution and deterministic pagination.
  Absence plus incomplete coverage is unknown; ANN results never define this universe.
- **Hydration:** fetch selected entity sets and all required supporters/witnesses/spans in bounded
  batches. Missing support is corruption or explicit refusal, not a shorter answer. Native input
  comes from the same generation and retains existing kernel/row/byte limits.
- **Exact retrieval:** aggregate best chunks per entity and rank every eligible entity per view
  before current RRF/name promotion. With Python-owned fusion, O(entities×views) ranks still cross
  the boundary; dense O(chunks×dimensions) Python vector matrices can be removed. The repository
  bounds query time/transfer; fusion preflights retained rank state. Budget exhaustion refuses or
  uses an explicitly requested approximate profile, never silent top-k truncation of an exact leg.
- **Approximate retrieval:** HNSW profile identity declares metric, candidate depth per view,
  generation/filter isolation, deduplication, widening/exact-fallback or underfill policy, numeric
  tolerance/ties and result metadata. Adding an index cannot silently route an exact request to ANN.
  Keep full-float vectors and current BM25 discriminating-term policy; pgvector is the selected
  scalable path instead of the old LanceDB trigger.
- **Effects:** one configured process Tokio runtime; lifespan-owned SQLx pool, coarse awaitables,
  explicit acquire/query/lock/cancellation limits and no connection held across embedding calls.
  Cancellation must stop or safely drain server work and leave reusable connections; Python future
  cancellation alone is not proof. Keep the semantic executor pure.
- **PG→Arrow:** explicit codecs validate fixed-size IDs/digests, widths, codebooks, nullable/list
  shapes, floats and original domain metadata. The provider's `source_type` metadata is useful but
  not the canonical schema. Empty results carry the declared schema; unsupported conversion fails.
- **Federation:** declare the admitted expression/cast/collation subset for both scan and optimizer
  rewrites. Unparsing SQL is not proof of `Exact` semantics. Preserve residuals for inexact filters
  and forbid premature limit pushdown; unsupported rewrites stay local or fail explicitly.
- **Read views:** generation predicates plus immutable retained rows cover separate provider
  connections. For coherent mutable operational reports, materialize all needed relations under one
  bounded read-only repeatable-read transaction before DataFusion joins. Export/import snapshots is
  a later alternative if genuinely required; independent pooled transactions do not share a view.

## 4. Dependency-ordered implementation

PG0–PG7 are complete historical packages (§8), not an instruction to repeat deployment. The active
implementation queue is **PG8–PG17, all planned/not_run**. Contract/source inventory changes precede
production edits; focused checks settle each boundary and integrated gates run at PG17.

| Package | Owner and deliverable | Depends on |
|---|---|---|
| PG8 | Architecture/schema owners: successor decisions, contract inventory, baseline and acceptance criteria | Expanded review and preserved PG0–PG7 state |
| PG9 | Embedding owners: standard 1024 spec, clients, launcher and rebuilt receipts/outputs | PG8 |
| PG10 | Dependency/deployment owners: selected libraries, maintained fork, extension, roles and offline tooling | PG8; can prepare independently of PG9 |
| PG11 | Schema/storage/native owners: serving manifest, projected relations, codecs and shared validation | PG8; PG9 defines the production vector contract |
| PG12 | Importer/CLI: COPY load, indexing, atomic readiness and recovery | PG9–PG11 |
| PG13 | Rust repository/native/Python: exact selection, evidence hydration and async serving | PG12 |
| PG14 | Retrieval owner: exact rank route, pgvector HNSW profile and bounded fusion | PG13 and PG9; pgvector from PG10 |
| PG15 | DataFusion adapter: qualified read views/federation and coherent operational report | PG11/PG12 and PG10; integrates alongside PG13/PG14 |
| PG16 | Operator/CLI: coordinated serving cutover, restore/rebuild, diagnostics and deletion | PG13–PG15 |
| PG17 | Integrated product/operations acceptance, assembled review and handoff | PG9–PG16 |

### PG0 — Settle authority, schemas and success criteria

**Complete, historical.** ADR-0065/0066/0067, initial budgets and preserved baseline established
SQLx cache/operations and exact receipt ownership. PG8 handles the new serving/embedding decisions;
PG0 is not reopened merely because scope expands.

### PG1 — Qualify the Rust boundary and disposable database workflow

**Complete, historical.** SQLx/Tokio/Rustls, real PG18 role/type/recovery tests, explicit migrations,
offline `.sqlx`, stale-metadata controls and dependency checks. Keep compiler-affecting external SQL
in the source inventory; operational-only query revisions remain separate. Extend these mechanisms
in PG10/PG17 rather than introduce a second migration/build system.

### PG2 — Deploy and operate the local PostgreSQL service

**Complete, historical.** Dedicated PG18.6 database/roles, protected config, bounded resources,
readiness and populated backup/restore. HBA remains unchanged; runtime is not superuser. PG10 adds
extension/import/read roles through this deployment route, leaving the separate PostgreSQL 16 cluster
and unrelated databases alone.

### PG3 — Make snapshots own all consumed vectors

**Complete, historical.** Shared `used_embeddings`/`embedding_uses`, exact codec/digest/coverage,
all E0/operation/brief consumers and cache-independent bundle replay. PG9 reuses this protocol with
a new 1024 spec; no global cache-version read path returns.

### PG4 — Integrate PostgreSQL cache admission and recovery

**Complete, historical.** One bounded writer, immutable committed-winner readback, ambiguity/retry,
attempt-owned values and race/failure/restore controls. Cache-backed compiles fail explicitly on DB
failure; extraction, no-embedding compilation and canonical query/diff/bundle remain independently usable.

### PG5 — Import, cut over and remove the old mutable cache path

**Complete, historical.** Versioned legacy import with tokenizer re-admission, conflict receipts,
old mutable/global reader deletion and prior-binary/store rollback. No new 4096→1024 cache conversion
is implied: new production values use a new spec; existing receipts retain their exact bytes.

### PG6 — Add operational consumers without another semantic authority

**Complete, historical.** Attempt/event and snapshot/generation CLI, canonical reconciliation and
unknown recovered history. Events do not establish Delta publication; optional journaling failure
does not invalidate an already-published snapshot. PG15/PG16 extend these actual operator consumers.

### PG7 — Integrated qualification, deployment acceptance and handoff

**Complete, historical.** Full code gate, fake/live/concurrent pilots, controlled client conformance,
PG-offline byte replay, restore/rollback and measured initial costs. §8 retains exact evidence and
scope. None of these results qualifies new 1024 clients, database serving or ANN.

### PG8 — Settle expanded contracts, decisions and the migration baseline

**Owner:** storage/serving/schema owners. **Dependencies:** expanded review. **Deliver:**

- Supersede ADR-0065/0066 using the ADR skill and update §B13/§B14, storage §6.5 and serving §11
  with self-contained service/serving/spec/ranking decisions. Carry unrelated FastMCP, evidence,
  immutable generation and native-executor clauses forward; do not implicitly accept ADR-0025.
  Retain ADR-0067 canonical Delta/receipt authority; change it only if that authority actually changes.
- Inventory exact tool requests/results, aliases, facets/status, native files, full support closure,
  every embedding consumer and existing Python generation maps. Assign domain declarations, SQL
  projection/codec, query owner and deletion destination once per surface; include planned Stage 4
  concept membership/explanation and frozen review inputs without inventing their semantics now.
- Declare spec, canonical snapshot, serving projection, native/lexical artifact and retrieval-profile
  identities; supported current/rollback formats; typed errors for unavailable, incomplete, corrupt,
  incompatible and resource-refused work. Operational location/timing stays outside content identity.
- Preserve baseline binaries/locks/config/stores and frozen evaluation artifacts. Inventory hashed
  compiler SQL/source changes and required reviewed schema/output/FORMAT migrations. Register exact
  answer controls, ANN/index criteria, latency/RSS/import/WAL budgets and deployment recovery goals
  before measurements.1024 quality is not an acceptance criterion to rediscover.

**Exit:** owners/ADRs agree, input/output and deletion inventory is concrete, selected workload sizes
and limits have named criteria. Finding disposition stays in forward-plan §6.1. **Deletion:** retire
superseded text only after all surviving clauses and references move; retain stable source IDs.

#### PG8–PG11 execution contracts (2026-09-27)

**Accepted; implementation in progress.** ADR-0068 supersedes 0065/0066. The functional slice
ends at projection contracts, codecs, role deployment and the async service foundation. PG12
owns production import/promotion; PG13 exact query cutover; PG14 indexes/ranking; PG15 admitted
federation. The file MCP remains a reference consumer throughout this slice. Per operator
instruction, only targeted functional checks run during implementation. Formatting, integrated
checks and documentation checks wait until this functional scope is complete.

| Surface / consumer | Contract and owner | Migration / deletion destination |
|---|---|---|
| All brief tools/resource: briefs, assertions, members, symbols, evidence and complete analytic support | `cpg-schema::bundle` and `serving_support`; `lctx-postgres::projection` physical codec | Delete Python schema inventory and closure algorithm now; PG13 deletes relational hydration/maps after exact tool parity |
| `get_operation` and `find_operations`: paths/aliases, operation rows, parameters, facets/status, behaviors, globals/place claims | Existing Arrow meaning plus generation-local keys; five verdicts and unknown partitions retained | PG13 repository owns selection and full hydration; current file selectors remain until then |
| `search_operations` and `search_capabilities`: text, chunks, vector views, input/spec identity | One standard spec; current lexical policy; exact ranks and explicit approximate profiles | PG14 removes Python dense matrices, retaining Python lexical scoring/fusion; index installation alone never changes exact routing |
| Native conditions, source/model identities, summaries/proofs and value paths | `serving_projection::NATIVE_FILES`, existing `lctx_semantics` IPC decoder/kernel | Native file inventory moves to schema owner; bounded pure IPC remains. No SQL in semantic executor |
| Compiler E0, operation views, brief chunks, query text | `embedding_spec::Spec` format 2; both clients send 1024 dimensions; prefix then L2 | Compiler output 107 and bundle FORMAT 11; fresh outputs, old receipts never rewritten |
| Stage 4 membership/explanation; selected operator review inputs | Existing attributed facts and source identity; future query contracts remain Stage 4 owners | No placeholder semantics/tables. Freeze selected review revision before attributed compiler ingestion |
| Python service lifetime | `lctx_storage` coarse awaitables, one two-worker Tokio runtime, explicit open/check/close | No effects at import and no compiler/DataFusion dependency in the serving wheel; PG13 adds request methods |
| PostgreSQL read provider | Maintained immutable fork, isolated `cpg_core::postgres_read` configuration/pool adapter | Provider qualification and production federation activation remain PG15 |

Content identities are separate: canonical snapshot/content/compiler; full 256-bit projection
manifest digest; digest/size/format of every native/lexical artifact; separately qualified retrieval
profile. The projection includes schema, catalog/kernel, entry-effect and embedding identities,
all logical relation receipts and artifact receipts. Relation receipts ignore row order and COPY
chunk boundaries but preserve duplicates, nulls and float bits. Locations, attempts, timing,
selection and index readiness do not enter content identity. Typed failures are `unavailable`,
`incomplete`, `corrupt`, `incompatible`, and `resource_refused`; refusal never means an empty answer.

**Registered budgets, before the new live workload measurements.** Use the real pinned FastMCP
pilot plus 10× repeated query/codec workload, bounded by existing native caps; these are resource
controls, not a new embedding-fidelity experiment. Keep individual native IPC files at 64 MiB,
summary/support relations at 100,000 rows, public surface at 200,000 rows. Projection relations
are at most 200,000 rows/128 MiB; full file validation refuses above 512 MiB. COPY encoded staging
is at most 16 MiB with 128 MiB aggregate import buffering. The importer has at most two SQLx
connections; serving defaults to six total, with at most two reserved for the provider, never
six plus two. Acquire/lock/statement defaults are 5/5/30 seconds, import transactions at most
30 seconds, and unfinished leases are closed. Target bounded codec/validation RSS below 512 MiB
for the pilot and growth workload; report actual peak separately from retained file hydration.
Live request conformance remains cosine ≥0.9995 between clients. Cold/warm receipts and offline
rebuild require exact content equality. ANN recall/filter/underfill controls and whole-import
WAL/latency acceptance belong to PG12/14/17, before those measurements; PG8–PG11 cannot certify them.

**Rollback boundary.** `build/postgresql-pg8-baseline-6403b60/` preserves binary, locks, protected
config and a checksummed PG7 dump. Its reader supports FORMAT 10/spec format 1. The new reader
supports FORMAT 11/spec format 2 and refuses incompatible generations. A rollback restores a
separate PG7 database/config: the old binary's exact migration check rejects an expanded database.
A successful dump is insufficient; perform the disposable restore and compare all PG7 table
fingerprints. Canonical stores/evaluation artifacts remain preserved and unmodified.

### PG9 — Implement the standard 1024 spec and rebuild dependent outputs

**Owner:** `Spec`, `lctx-embed`, Python embedder, launcher and analytics orchestration. **Depends:** PG8.

- Record MRL prefix→L2 normalization and admission override in canonical spec identity; keep
  Rust/Python canonical encoding and shared request bodies equal. Derive vLLM launch admission and
  `dimensions=1024` requests from that spec; update copies, help and known answers together.
- Enforce model/count/index/width/finiteness/norm/token controls at both clients. New cache keys
  preserve exact receipt semantics and cold/warm committed winners. Reject mixed old/new specs.
- Rebuild analytics and serving inputs from pinned source in a fresh store; preserve existing
  thresholds/seeds, BDD/model semantics and evaluation targets. New geometric outputs are expected
  under the new spec; do not relabel old outputs or require 4096 equality. Keep fake/live identity
  distinct; preserve historical 4096 artifacts for their existing/rollback reader.

**Exit:** focused request/spec/response and mismatch tests, controlled 1024 live Rust/Python
conformance, finite unit vectors and exact cold/warm receipt replay. Schedule the live leg with
operator-owned GPU resources; stop only the service this work starts. Full product gates wait for
PG17. **Deletion:** full 4096 default/no-dimensions instructions and obsolete known answers; no dual
permanent embedding standard. **Rollback:** preserved binary/spec/store, never rewritten receipts.

### PG10 — Integrate the selected stack and extend the existing deployment

**Owner:** dependency, PostgreSQL service and test tooling. **Depends:** PG8; coordinate PG9/PG11 contracts.

- Use pin-check to integrate only §2's selected packages/features into root locks, keeping one
  Arrow/DataFusion/object_store family. Establish an owned fork from the tested migration with
  explicit patch provenance/final immutable revision; no moving branch dependency. Carry schema,
  pushdown or pool fixes there rather than forking client behavior across call sites.
- Keep SQLx migration ownership and COPY transport. Isolate the provider's internal driver/pool and
  configuration conversion; coordinate both pools, trust, statement/lock/acquisition limits and
  diagnostics. Configure one PyO3 async process runtime; the repository pool remains lifespan-owned.
- Install/enable the separately pinned pgvector extension in the intended PG18 database through
  explicit administrator/migration steps. Add importer and read-only serving privileges; deny
  runtime DDL and mutation of ready projections. Extend protected config without printing credentials.
- Pin an extension-bearing PG18 test image by patch/digest, assert server and extension versions,
  and reuse disposable role/readiness fixtures. Preserve ordinary offline builds, fresh-schema
  `.sqlx` checks and dependency policy; cover new application targets/features. No Python ORM,
  native ADBC artifact or SQL/ADBC listening service is installed for unused alternatives.

**Exit:** resolved and compiled product graph; real PG18/extension/type/grant checks, migrations and
upgrade from the PG7 schema, stale metadata rejection, bogus-DSN offline build, TLS/config redaction
and combined pool-budget controls. Missing admin access is a named deployment prerequisite; continue
code/disposable qualification independently. **Deletion:** probe-only or competing default clients.

### PG11 — Declare serving generations, physical mappings and complete validation

**Owner:** `cpg-schema`, projection/native/storage owners. **Depends:** PG8 and PG9's spec contract.

- Implement §3.4's deterministic projection manifest and typed relations for every current tool:
  operations/paths/facets/status/parameters/behaviors; briefs/assertions; support/findings/witnesses/
  evidence; embedding views/chunks; native/lexical artifact references. Keep full source relation
  identity, direction, multiplicity, verdict, coverage and model/condition identity.
- Declare SQL physical mappings plus checked Arrow reconstruction; IDs/digests carry width checks,
  codebooks remain append-only, empty batches keep schemas and metadata is reconstructed from its
  owner. Do not invent a second universal schema DSL or derive meaning from PG catalog types.
- Reuse shared semantic/evidence validators and add projection-local uniqueness, generation FK,
  support-closure, vector and readiness checks. Validation consumes the same contracts as import
  and serving. Native input remains its existing pure, bounded IPC contract.
- Separate generation handle/native/lexical state from full-file Python relational hydration.
  Version format changes explicitly; old generations retain their identities and are rejected by
  incompatible readers rather than coerced. Add only extension points required by Stage 4's declared
  registry/membership/witness consumers, not empty tables for hypothetical semantics.

**Exit:** independent known-answer projections, null/empty/wrong-width/code/spec/metadata controls,
parallel/cross-generation/missing-support rejection and native round trips. Read `.snap.new` before
acceptance; report schema migration. **Deletion:** duplicate shape/meaning declarations and permissive
casts on this boundary; do not remove independent malformed-input controls.

### PG12 — Build COPY import, index construction and atomic serving readiness

**Owner:** Rust projection writer and `lctx` serving-generation commands. **Depends:** PG9–PG11.

- Implement the complete §3.4 state machine over published canonical inputs. Stream bounded pgpq
  COPY batches through a SQLx transaction; validate `real[]` staging before `vector(1024)` conversion.
  Use declared codecs for unsupported domain types, never a general handwritten binary protocol.
- Record idempotent import/batch identities and content digests, reconcile ambiguous commits and
  refuse conflicting retry content. Keep artifact ownership explicit. Build B-tree/exact-query
  indexes while unpublished; HNSW-ready profiles integrate in PG14 before their profile is selectable.
- Validate all relations, native/lexical artifacts and selected index profile before the final ready
  transaction. Expose explicit import/status/select/reconcile routes; discovering a Delta snapshot
  does not automatically activate a PG serving generation. Bound transactions, COPY buffers and WAL.
- Keep ready rows immutable and all ready generations retained. Clean only abandoned unpublished
  imports after proving ownership/non-use; preserve diagnostic and durable history records.

**Exit:** real PG imports, interrupted load/index/validation, resumable retry/conflict, missing artifact,
ambiguous ready commit, no partial read visibility, idempotent rebuild and two-generation selection
controls. **Deletion:** no direct vector-only publication path or automatic migrate/import on reads.

### PG13 — Integrate exact Rust serving and the asynchronous MCP boundary

**Owner:** Rust serving repository, Python application and native-input owner. **Depends:** PG12.

- Provide typed resolve/get, exact facet page and batched full-evidence hydration operations. Bind
  every query/cursor to the pinned ready generation and request. Preserve match/no/open and full
  unknown totals, all five verdicts, public aliases and deterministic ordering; enforce response budgets.
- Use a narrow effect-owning module/crate to keep SQLx out of the pure native executor. Return
  coarse typed results through PyO3 awaitables; retain one authoritative selection/hydration query
  owner for CLI/MCP. Python keeps transport/Pydantic models and current lexical/fusion policy.
- Load/check the generation's native image and lexical state once; fetch relational answer data
  selectively. Support counts/errors and all evidence relationships remain complete. No arbitrary
  SQL, one-call-per-witness N+1 path, live Delta query or second Python condition interpreter.
- Integrate FastMCP lifespan with read-only pool open/close and process runtime. Qualify cancellation,
  concurrency, deadlines, saturation, startup failure and connection reuse. PostgreSQL loss yields
  an explicit availability error; no silent backend/generation switch. Portable bundles remain
  offline export/recovery/reference artifacts rather than a second maintained online query engine.

**Exit:** current exact tools match independent fixture/reference answers on the same canonical 1024
input, including unknown coverage, aliases, conditions, parallel witnesses and cursor errors; clean
Python 3.14 wheel/native integration and lifecycle controls. **Deletion:** superseded production
Python relational dictionaries/selectors and duplicated schema/hydration logic after cutover; retain
semantic oracles and golden inputs, not obsolete implementation-mirroring tests.

### PG14 — Add exact pgvector ranks and explicit HNSW discovery

**Owner:** retrieval repository and existing Python lexical/fusion owner. **Depends:** PG13/PG9/PG10.

- Implement separate exhaustive-rank and bounded-candidate operations under §3.5. Keep exact
  symbol promotion, BM25 tokenization/discriminating-term abstention and RRF K=60. Stable profile
  identity records any selected numerical ranking semantics; exact enumeration does not promise
  bit-identical PostgreSQL/NumPy arithmetic without evidence.
- Use a distinct exact route with complete eligible entity/per-view ranks and declared numeric/tie
  controls. Stream rank pairs in bounded batches; preflight total O(entities×views) fusion state and
  refuse limits explicitly. Remove Python dense vector matrices after this route is qualified.
- Add generation-scoped `vector(1024)` HNSW indexes and explicit approximate profile selection.
  Publish each profile's readiness separately from immutable generation content; a later profile
  addition leaves already-pinned exact readers and cursors unchanged.
  Validate prepared-query plans/partition pruning, filters, per-view depth, best-chunk deduplication,
  iterative widening and bounded underfill/exact fallback. Indexed candidates cannot replace exact
  facet enumeration, full support hydration or compile-time kNN.
- Predeclare ANN recall/latency and filter/candidate-budget criteria against an independent 1024
  exact reference. This qualifies the ANN index, not the settled dimension choice. Surface the
  approximate profile and limit/degradation outcome in structured results; unavailable query
  embeddings may retain the declared lexical-only mode, while DB failures remain explicit.

**Exit:** exact/ranked semantics and result labels, multiple views, duplicate chunks, selective
filters, ties, generation isolation and resource exhaustion; actual index-use plans and agreed ANN
criteria. No implicit ANN switch when an index is added. **Deletion:** old 4096/LanceDB adoption route,
dense Python cosine scans and unversioned truncated-rank substitutes. F13 owns any later FTS change.

### PG15 — Qualify DataFusion PostgreSQL views and operational federation

**Owner:** DataFusion PG adapter and operational CLI. **Depends:** PG10–PG12; coexists with PG13/PG14.

- Provide read-only registered views over immutable serving/operational identities with explicit
  domain Arrow codecs. Use the maintained PostgreSQL provider plus federation; do not expose raw
  provider catalog inference as a canonical schema or promise unrestricted SQL semantics.
- Declare and execute a finite supported expression/cast/collation set for table-scan and federation
  pushdown. Compare pushed/local controls for nulls, ordering, limits, floats, timestamps, casts and
  empty results. Return Unsupported or retain an inexact residual where equivalence is unqualified;
  constrain federation rewrites too, not only `supports_filters_pushdown`.
- Deliver a concrete operator report joining PG attempts/events/projection readiness with an explicitly
  pinned Delta snapshot (for example run identity, publication, row counts and serving readiness).
  Immutable inputs carry their identity; coherent mutable relations are fetched together under one
  read-only repeatable-read transaction and then materialized for DataFusion. Bound fetched rows/bytes
  and transaction lifetime. Do not assume provider pool sessions share MVCC snapshots.
- Adapt config once; measure row-protocol conversion/transfer separately from query time. ADBC may
  later replace this transport through F11 after native-driver qualification; no redundant default
  pool/driver stack is installed merely because both candidates compile.

**Exit:** actual product-provider queries with exact schemas/metadata and supported predicate/limit
controls, cancellation/reuse, concurrent mutable-update coherence, explicit unsupported cases and
representative report output. **Deletion:** probe-only shortcuts, silently dropped columns and
blanket Exact declarations for unsupported semantics in the owned integration. General-purpose
federation beyond the admitted views/expressions remains unsupported, not an unfinished implicit API.

### PG16 — Complete operational cutover, recovery and obsolete-path removal

**Owner:** `lctx`, deployment and serving owners. **Depends:** PG13–PG15.

- Reconcile canonical discovery, PG projection readiness and selection through explicit commands.
  Extend doctor/status and tracing with projection/spec/index version, counts, artifact failures,
  import stage, latency and both pool pressures; never log secrets, source text or vector payloads.
- Cut over the production server to one selected PG repository; preserve prior binary/config/file
  generation for a controlled rollback window. Verify two servers can pin different ready
  generations and a pointer change cannot affect an existing server or cursor.
- Perform populated backup/restore into an isolated PG18+extension database; reconstruct projections
  from canonical snapshots/artifacts with the cache and embedder unavailable. Verify readiness after
  restore, mismatch/corruption rejection and durable event recovery. PostgreSQL loss does not destroy
  canonical replay, but online PG serving remains explicitly unavailable until restored/rebuilt.
- Keep ready-generation/artifact deletion disabled. Document disk/WAL growth and manual capacity
  management; F10 requires actual reader protection before pruning and still owns replicas/PITR/proxy
  topology. Set measured connection/memory/index/import budgets alongside compilation and vLLM.
- Complete PG8's consumer search and delete superseded production maps, dense-vector paths, stale
  no-dimensions/default 4096/LanceDB instructions and duplicate query adapters. Preserve portable
  export, pure native evaluation, exact receipts and independent answer fixtures.

**Exit:** operator commands, least-privilege startup, missing DB/artifact/extension cases, restart,
rollover, restore/rebuild and prior-compatible rollback. **Deletion:** only owned obsolete paths;
unrelated stores/services and operator-retained historical artifacts remain untouched.

### PG17 — Qualify the integrated expanded scope and hand off

**Owner:** storage/serving/application owners. **Depends:** PG9–PG16.

- Run the focused DB/native/query controls accumulated above, then the existing full `just test-all`
  once assembled; incorporate selected extension/provider features into real PG and SQLx checks.
  Reuse the stable release target. Missing Docker/image/admin/live-service prerequisites are named
  blocks; no skipped or fake-provider success substitutes for required runtime evidence.
- Run `just pilot` against a fresh expanded-scope store, then a controlled live 1024 compile/client
  conformance leg and exact/ANN MCP smoke over its imported PG generation. Compare exact tool
  answers/hydration to independent same-input fixtures/reference, not 4096 geometry. Test warm cache
  and cache/embedder-offline bundle/projection rebuild. Retain the Stage 3 semantic packet separately.
- Measure declared entity/chunk/fan-out/filter/generation workloads: import time/WAL/index storage,
  startup/RSS (including remaining native/lexical/rank state), exact/ANN latency, concurrent serving/
  compile contention and pool saturation. Report capability gains and limits; no inferred 4× speedup
  from fourfold payload reduction. A latency miss changes tuning/ANN support claims, not 1024 adoption.
- Execute recovery/cancellation/rollover controls, clean-wheel installation, existing dependency/
  metadata checks, `just docs-check` and assembled design/target review. Retain passing evidence
  after localized fixes; repeat only affected checks unless the changed boundary warrants broader work.
- Update implementation/test labels, runbook, actual pins and forward-plan findings only from
  produced evidence. Run handoff; distinguish reviewed design, supported backend/query subset and
  end-to-end qualification. Do not close Stage 3 P7 or arbitrary endpoint attestation through this gate.

**Exit:** selected 1024/cache/import/exact/ANN/federation/operator consumers run on the deployed PG18
route, supported semantics and failure outcomes are explicit, stale production paths are removed,
and PGE closure evidence is linked. Future §7 alternatives are not required to accept this scope.

## 5. Verification inventory and boundaries

| Check family | Independent challenge | Package |
|---|---|---|
| Existing invariants | Exact winner race, receipts, token admission, PG-offline canonical replay and operational history | Retain PG0–PG7 controls in PG17; do not reopen historical acceptance |
| 1024 implementation | Spec/request/launch consistency, wrong count/index/width/model/norm, mixed spec and cold/warm receipt replay | PG9; no dimension-quality comparison |
| Dependency/deployment | Single family, owned immutable fork, PG18/extension versions, roles, migrations, offline/stale SQLx metadata and sanitized two-pool config | PG10 |
| Domain conversion | Byte widths, codebooks, null/empty/list shape, float values, canonical metadata and empty schema; unsupported values fail | PG11/PG15 |
| Projection publication | Corrupt/missing support, cross-generation FK, interrupted COPY/index, retry conflict, ambiguous ready commit and artifact loss | PG11/PG12 |
| Exact tools/native | Known answers for coverage/unknowns, five verdicts, aliases, parallel witnesses, evidence closure, cursors and native limits | PG13 |
| Retrieval | Independent 1024 exact reference, full ranks versus ANN candidates, per-view/chunk/filter/tie controls, index plans and bounded underfill | PG14 |
| Federation | Supported pushdown on/off equivalence, residual-before-limit, schema fidelity, concurrent mutable views and concrete operator report | PG15 |
| Lifecycle/recovery | Cancellation/pool exhaustion/reuse, restart, pinned servers across rollover, restore/rebuild, explicit availability and compatible rollback | PG13/PG16 |
| Integrated scope/cost | Full code gate, fresh pilot, controlled live conformance, clean wheel, exact/ANN smoke and declared whole-workload costs | PG17 |

Commands report `passed`, `failed`, `blocked` with prerequisite or `not_run`. Labels distinguish
Proposed/Interface-checked/Implemented/Tested/Measured and give dates. Real PG semantics use real
PG18 with the selected extension; model-only/native logic retains bounded tests without databases.
The [existing evidence route](../design_review/evidence/README.md) holds material probes/receipts;
no new register or full-gate-per-slice workflow is introduced.

## 6. Design, planning and documentation integration

| Owner | Required update and timing |
|---|---|
| This plan | PG8–PG17 detail and §7 conditional capabilities; §8 distinguishes historical acceptance, review probes and unimplemented expansion |
| Forward plan | §1/§3.4 current boundary and sequencing, Stage 4/5 projection consumers, §6.1 all 11 PostgreSQL finding dispositions, deferred/risk/identity conventions |
| ADR/design owners, PG8 | Supersede 0065/0066; update §B13/§B14, storage §6.4/§6.5 and serving §11.1–§11.4. Carry unchanged clauses; retain 0067 canonical authority |
| Schema/bundle/native owners, PG9/PG11 | One spec/serving schema/manifest/codec authority and explicit version migration; reviewed snapshots and known answers |
| Pins/Cargo/uv/tooling, PG10 | Actual selected revisions/features, extension image, maintained fork and existing offline/dependency checks; no documentation-only claim of installation |
| CLI/README/runbook, PG12–PG16 | Concrete import/select/reconcile/serve configuration, readiness, resource limits, availability, backups/rebuild/rollback and retained artifact policy |
| Design map/AGENTS, PG8–PG16 | Correct runtime/dependency routes and command surfaces as they change; pure native and two-plan ownership remain clear |
| Reviews and STATUS | Reviews keep dated evidence/source IDs and link §6.1 for status. STATUS links current work without duplicating a finding register |

This planning revision does not edit accepted ADR rationale or present new targets as implemented.
PG8 settles binding changes before dependent production work; it does not reopen the settled
operator selection of 1024. Remove finished/superseded documents only after their surviving obligations
and evidence consumers move under the existing current-working-set policy.

## 7. Later capabilities and adoption triggers

Stable F identifiers are retained for traceability. **F2/F3(vector)/F5(selected COPY/provider) are
promoted into PG8–PG17**, not deferred behind old size/performance triggers. F10's conservative
retention/recovery baseline is also in scope. The remaining rows describe genuine later consumers;
compatible library availability alone does not install a second transport, query owner or service.

| Package | Current route / trigger | Authority, lifecycle and acceptance |
|---|---|---|
| F1 Operator/manual-review workflow | Selected operator-facing review consumer under §10.4; existing operations plus projection contracts | Append exact-subject-revision events, provenance/idempotency and optimistic conflicts. Freeze a selected revision into canonical compiler inputs; never patch a published brief or infer approval from absence. Backup/restore and frozen-input replay |
| F2 PostgreSQL serving projection | **Promoted:** PG8/PG11–PG13/PG16–PG17 | Rust-owned generation-pinned exact selection and full hydration; no renewed startup/RSS prerequisite |
| F3 Database vector retrieval | **Promoted:** PG9/PG10/PG14/PG17 | 1024 standard, exact and explicit HNSW profiles; future LanceDB selection replaced through PG8. Additional text-search policy is F13 |
| F4 Dynamic SQL | Substantive typed variable joins/expressions beyond clear SQLx queries | SeaQuery+binder with allowlisted identifiers and bound values; null/array/adversarial SQL controls. Stage 4 AST remains Rust→DataFusion |
| F5 Bulk/Arrow/federation | **Promoted:** pgpq+SQLx in PG12; maintained PG provider+federation in PG15 | One family, checked domain codecs and admitted read views. Native ADBC alternative is F11; this does not select the Cornucopia stack |
| F6 Direct Python workflow/SQLAlchemy | Python gains its own relational workflow, not just an MCP transport caller | Psycopg 3 default; one async pool owner; SQLAlchemy only for substantive Core/ORM ownership. Python 3.14 lifecycle/type controls, same Rust migration history |
| F7 Notifications/durable jobs | Actual worker or polling-cost consumer | Durable rows/events are truth; LISTEN/NOTIFY is a reconnectable hint. Bound transactions, claiming/retry/idempotency and crash recovery before job scheduling |
| F8 pgrx extension | Measured candidate-transfer cost cannot be handled by bounded fetch into the native executor | Separate PG18 extension crate/toolchain/runtime rules; pure shared kernel, explicit model/generation identity and refusal/evidence fidelity; no Tokio analysis engine in a PG backend |
| F9 Canonical-store replacement | Agreed canonical PG-transaction requirement or measured whole-store bottleneck | Separate §B3/§B7 decision; complete writes/validators/IDs/publish/read/rebuild comparison and removed machinery. PGS/F03 stays deferred |
| F10 Retention/advanced operations | PG16 delivers retain-all, restore/rebuild and capacity diagnostics; actual growth/pressure/recovery need selects further work | Before pruning: protect live readers and referenced artifacts, plus in-flight cache users. Replicas/PITR/proxies need separate durability/read/session contracts; no automatic topology expansion |
| F11 Native ADBC bulk reads | Representative PG15 row-conversion/transfer cost or a named columnar client favors it | Pin core/manager/FFI 0.24, r2d2_adbc 0.3 and compatible provider plus native PG driver artifact; qualify mappings (including numeric/opaque types), C ABI, blocking executor/pool health, transactions/cancellation and supported expressions. Replace the selected transport for that consumer rather than duplicate defaults |
| F12 DataFusion client protocols | Named psql/BI or embedded ADBC client | Select datafusion-postgres family for PG-wire serving, or adbc-driver-datafusion for ADBC access. Pin supplied revisions; own authentication/exposed SQL/read-only limits/cancellation/session generation and operational port/artifact. Neither is a PG storage provider |
| F13 PostgreSQL lexical/FTS or alternate vector engine | Named lexical/discovery capability or measured limitation of the qualified target | PostgreSQL FTS/trigram/extension scores require a new policy versus bm25s; LanceDB requires a new case versus PG serving, not the obsolete 4096 threshold. Preserve abstention/name promotion, evidence separation and predeclared retrieval criteria |

PostgreSQL arrays, JSONB, constraints, indexes, CTEs, windows, partitioning and transactions are
available through the existing driver when a named schema/query needs them. Do not add a wrapper
framework, ORM or another client merely to expose those capabilities. Authored event data is backed
up independently of reconstructible serving projections; notifications never replace that history.

## 8. Finding traceability, finish and current verification

This table maps source findings to work; **status and closure live only in
[forward-plan §6.1](behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)**.

| Source | Detailed work / acceptance route |
|---|---|
| PGS/F01 exact consumed vectors | Completed PG3–PG7; preserve controls through PG9/PG17 |
| PGS/F02 pinned serving semantics | PG8/PG11–PG14/PG16–PG17; exact answers, complete evidence, rank policy and publication/recovery |
| PGS/F03 canonical replacement case | F9; expanded target still retains Delta |
| PGS/F04 federation/Arrow boundary | PG10–PG12/PG15/PG17; selected product graph and semantic/read-view qualification |
| PGK/F01 external-stack composition | Conditional reconsideration of the Cornucopia/application-driver alternative, not automatically triggered by the provider's internal Rust-Postgres dependency |
| PGK/F02 PG server defaults | Completed PG1/PG2/PG7; extension-bearing image/version controls extend in PG10/PG17 |
| PGE/F01 dimension/request/launch identity | PG8/PG9/PG17; 1024 conformance and exact receipts, no dimension-quality gate |
| PGE/F02 serving ownership/full hydration | PG8/PG11/PG13/PG16/PG17; one repository, coarse async boundary and obsolete-path deletion |
| PGE/F03 whole-generation readiness | PG11/PG12/PG16/PG17; manifest/closure, pinned readers and restore/rebuild |
| PGE/F04 schema/pushdown/read views | PG10/PG11/PG15/PG17; admitted domain/expressions and coherent actual consumer |
| PGE/F05 exact versus indexed ranking | PG8/PG14/PG17; full ranks, explicit ANN/fusion/underfill and unchanged exact semantic queries |

**Historical PG0–PG7 acceptance, 2026-09-27:** `just test-all` passed 430 ordinary Rust tests,
8 real PostgreSQL tests, 154 Python tests, strict lints, dependency/gold policy and fresh-schema
SQLx metadata. PG18.6 local roles/config and migrations 202609270001/2 are deployed. Fresh fake/live
and concurrent pilots, controlled Rust/Python conformance, cold/warm equality, all 52 bundle files
byte-equal with PostgreSQL unavailable, populated backup/restore, rollback and CLI reconciliation
passed. All 2,463 compared old/new fake vectors were byte-equal. The scoped publication at that
checkpoint passed 146 pages; full docs-check failed on the unchanged external input's missing H1.
Commands, identities, costs and limits remain in the [initial evidence](../design_review/evidence/2026-09-27_postgresql/README.md).

The [initial assembled review](../design_review/reviews/design_review_postgresql-change_2026-09-27.md)
was **Accept scoped** for source ownership/composition, with runtime acceptance recorded separately.
Those receipts remain valid for their historical scope and do not establish a whole-compiler speedup.
The [runbook](../postgresql.md) describes the deployed baseline until the planned changes land.

**Expansion design evidence, 2026-09-27:** isolated all-candidate resolution with one DF55.1/
Arrow 59.3/ADBC 0.24 family, selected providers/federation/exact delta-rs/SQLx/pgpq compilation and
real PG18.6 COPY/null-filter/limit/empty-result/join probes passed. The
[expanded review](../design_review/reviews/design_review_postgresql-expanded-architecture_2026-09-27.md)
recommends revising the target; unrestricted federation semantics remain unresolved. Optional
protocol packages were resolved, not executed; native ADBC and full serving/ANN were not run.
[Probe evidence](../design_review/evidence/2026-09-27_postgresql-expansion/README.md) is not PG17 acceptance.

**Current planning checkpoint:** PG8–PG17 implementation, extension deployment, live 1024 serving,
new exact/ANN tools and product acceptance are **not_run**. This revision updates planning and
finding ownership only. Remote topology and arbitrary embedding-endpoint attestation remain
unqualified; Stage 3 semantic completion and its independent exit remain in the forward plan.
