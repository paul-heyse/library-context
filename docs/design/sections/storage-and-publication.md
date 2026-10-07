# Storage and publication

**Accepted target / implementation in progress, 2026-10-07 (ADR-0133).**
The [persisted graph execution plan](../../plans/persisted-graph-execution-plan_2026-10-07.md)
owns this hard compiler and shared-consumer pivot. One private persisted compiler database
retains exact completed views and dependency bindings through direct sealing. Independent
external import and stored reconciliation remain separate exposures. Prior graph-native receipts
retain their date and scope; acceptance of this target does not establish its implementation.

## §5 Projections

**Implemented compiler and published native projections, 2026-10-05.** Named projection contracts specify their
completed source, semantic roles, contexts, vertex universe, direction, multiplicity, weights,
coverage, provenance and declared simplifications. Vertices are independent of edges; isolates and
parallel attributed assertions survive. Dense indices are private and map back to semantic IDs.
Compatible compiler consumers share compact topology; rich evidence is hydrated separately.

The accepted persisted target constructs these compiler views from exact completed native memberships, without a
publication handle. Indexed native selection and projected Arrow streams feed model-owned kernels. Summary, Structural and optional analytical consumers wait for their actual
semantic/vector predecessors. Published export uses scoped native queries and terminally successful
responses under the same projection contract. External destinations remain consumer-triggered.

> Decision: ADR-0133, ADR-0103, ADR-0044, ADR-0085

## §6 Persistence and publication

**Accepted target, 2026-10-07; operator activation not_run.** Compilation owns a managed
STRICT native database from its first persisted output. `lctx-model` owns semantics;
`lctx-surrealdb` owns mechanical storage/access, `cpg-core` owns orchestration/admission, and
`lctx-publisher` owns visibility. The publisher seals that same admitted database. Selection
remains an explicit operator action. The [execution plan](../../plans/persisted-graph-execution-plan_2026-10-07.md)
owns integration and focused functional acceptance.

> Decision: ADR-0133

### §6.1 Admitted content and native publication

The compiler returns canonical graph streams, original chunks, coverage/outcomes, selected
method definitions and consumed vector values. It checks local shapes during construction and
reference closure, required domains and meaningful cross-element invariants over completed inputs.
Immutable completed memberships define inputs; incomplete output remains private.

Ordinary publication consumes the compiler-owned admitted native authority directly; explicit
external artifacts undergo independent transport and semantic validation. `lctx-surrealdb` mechanically lowers
canonical typed payloads into compact entity/assertion families with enforced native role edges.
Typed atomic keys for active scope fields share one array-element index; absent fields add no
scope entries. This avoids both ID-byte index flattening and per-field index amplification. Originals use bounded
binary chunks. The publisher performs checked bounded bulk writes, verifies stored content and query-visible
mappings once after content/definition writers drain, readies indexes/functions and seals the
executable realization. No transaction spans extraction, CPU analyses or embeddings. A short control
transition publishes the complete realization; selection remains explicit and separate.

Sealing fingerprints the effective database and table definitions returned by native metadata,
including fields, indexes, events, functions and analyzers. An explicit cold audit reuses content
reconciliation and that fingerprint. It excludes credentials and live subscriptions; SurrealDB
3.3 metadata cannot certify database STRICT mode. Creation enforces STRICT directly. Portable
backup transports canonical graph/original families, exact completed contribution memberships,
current and frozen bindings, retained dependencies and nongraph backing values. Restore preserves
their versioned state identity and rebuilds current derived search/executable definitions.

> Decision: ADR-0133, ADR-0088, ADR-0126

### §6.2 Readers

Compiler readers bind explicit immutable native completed views, including exact contribution
membership and frozen semantic boundaries. Pending values cannot enter selections or absence
answers. Compact projected streams and scoped typed indexes avoid repeated rich reconstruction.
Native reads preserve explicit nominal-key demand, point-read deterministic memberships under
verified completed owners, and deduplicate physical IDs before payload hydration. Recognized atomic
field selections use bounded indexed scope windows. Compact native scope constants are prepared
before index selection; inline value mapping/casts remain outside its predicate. General filters retain compact native selection. Explicit empty demands and unfiltered zero-row views retain authority, exact-view and
shape validation while avoiding a row query; arbitrary SQL retains normal execution semantics.
Compiler-private sessions have no published snapshot handle. Cancellation drains owned reads,
writes and provider workers before the attempt is removed; ambiguity fails that owned attempt.

Published readers pin one content/physical realization under read-only credentials and supervisor
lifetime ownership. Functions, analyzers, index specs, engine identity and adopted module bytes
cannot change beneath a pinned handle. Streamed results remain provisional until terminal success.
A partial diagnostic subgraph reports its boundary and cannot claim global compiler completeness.

> Decision: ADR-0133, ADR-0126

### §6.3 Schema evolution

Typed declarations and explicit canonical codecs own contracts; Arrow IPC/container bytes and
engine coercions do not define semantic identity. Snapshot changes are explicit migrations;
codebooks remain append-only. Rebuild fresh from pinned inputs without old-format readers, legacy
IDs, dual writes or rollback/runtime archives. Quiesce actual readers before replacing their state.

> Decision: ADR-0133, ADR-0087, ADR-0048

### §6.4 Serving realizations

**Implemented S2/S3, 2026-10-05; scoped functional evidence remains distinct from operator activation.** Serving uses the pinned native realization,
indexed restrictions and coarse connected queries/functions. Rust owns meaning, even where
SurrealQL is the only executor; Python remains a thin adapter. Complex kernels consume appropriate
batched inputs. Search eligibility precedes channel limits; exact analytical neighbors remain a
separate contract from approximate discovery. Resources and continuations pin complete realization.

> Decision: ADR-0133, ADR-0071, ADR-0131, ADR-0049

<a id="section-6-5"></a>

### §6.5 Services and capability adoption

**Implemented managed runtime and disposable fixture; operator deployment not_run, 2026-10-05.** A managed local SurrealDB 3.3 RocksDB server serves
strict snapshot databases and a small mutable control/cache database. Cache keys include complete
embedding spec/text; exact consumed winners enter graph content. Compiler operation can use a
supplied embedder without a persistent cache. Unrequested effects open neither service nor cache.

Native graph/documents/functions/search/bulk operations are selected. Query planning tools are
available for concrete uncertainties, not mandatory per-operation accounting or adoption proofs.
Custom APIs, buckets, reactive enrichment, connectors and external exporters require a named
consumer. Historical runs/operation records are retired rather than reconstructed as another service.
The [operator runbook](../../surrealdb.md) documents configuration, publication and explicit selection.
Operator reconstruction, live vectors and activation belong to separately authorized Q1 work.

> Decision: ADR-0133, ADR-0073, ADR-0131
