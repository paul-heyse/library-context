# SurrealDB realization and immutable publication

**Implemented / focused Tested, 2026-10-06; operator adoption not_run.** Supporting plan for P1/P2/I1 in the
[replacement coordinator](graph-native-pivot-plan_2026-10-05.md). The coordinator owns state,
shared decisions and disposition. This plan consumes the [admitted graph](graph-native-model-compiler-plan_2026-10-05.md)
and the [native serving operations](graph-native-serving-plan_2026-10-05.md); it owns their
physical realization, not their meaning.

## 1. Selected deployment and library boundary

Use one managed local **SurrealDB 3.3 persistent RocksDB server**, with the stable remote Rust
SDK, strict per-snapshot databases and a small mutable control/cache database. Compiler, MCP and
operator tools share the server. Keep its failure/memory separate from compiler and native Python
workers. The server owns maintenance; avoid direct `surrealdb-core` coupling and embedded primary
storage. Memory-engine controls can serve local codec/function cases, but persistent/authenticated
controls exercise the selected real server.

The comprehensive skill assessment covered all currently available SurrealDB briefs/catalog
families, including core and frontends, and decisive source at 3.3.0. Its source anchor is
`238bfeb11f5725bebed370167656748df8067595`; existing image reference is
`sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20`.
Those identify inspected contracts, not a new running installation or measured winner.

Add the remote SDK through ordinary dependency resolution. If the resolved release differs from
the inspected 3.3.0 family, inspect the changed selected interfaces before transferring claims;
do not silently couple distinct types/protocol families. Pin the server image for the exercised
deployment/parity contract and document that reason/revisit condition. An exact SDK/type-family
pin is justified only when actually required by family agreement. Avoid unrelated dependency pins.
Use `protocol-grpc` for large incremental scans and ordinary operations, and HTTP for documented
portable export/import; omit embedded `kv-*` features from production clients. Native gRPC also
has export/import support at 3.3; it may be used through the same transport after its focused
finality controls, without retaining a second abstraction.

Reuse existing structured logging. Configure explicit query/call deadlines, cancellation,
RocksDB cache and tracked-memory headroom, bounded client/loader queues and process limits suited
to the machine. Native buffer/spill controls are useful implementation settings. Do not introduce
a resource-budget service, core counter bridge or per-query accounting protocol. Health/version
readiness, schema installation, index readiness and snapshot publication are different states.

## 2. Physical graph, codecs and originals — P1

### 2.1 Compact families designed with native operations

Start with entity records; attributed assertion/value records; source artifact/chunk/evidence
records; runs/derivations/coverage; retrieval occurrences/shared vectors; and classic relation
families for binary assertions, participants, support and ordered premises. These are functional
families, not a table per semantic kind or a universal unindexed edge bag. Split a hot family
only for a real access pattern. The model remains free to remove incidental old records.

Use generated SCHEMAFULL common envelopes, discriminated typed payloads and explicit key/role
fields. Generate physical types/kind codes and structural constraints from the model; admission
owns kind-specific semantics. Select useful indexes for exact public paths, input/release,
assertion kind/context, ordered premises/chunks, retrieval text/spec and the chosen native queries.
Create fresh 3.3 search indexes so shared document-ID/bitmap capabilities are available.

The implementation retains canonical payloads and derived atomic scope fields, with one sparse
`scope_keys.*` index per canonical entity/assertion family. Only present values contribute typed
`semantic_type|field|value` keys; indexed `CONTAINSANY` requests select their exact type/field
before decoding. This avoids a separate index for every optional field across the entire graph.
Search documents, contextual occurrences and shared vectors retain their dedicated hot indexes.

Classic identity-bearing relation records are inserted through `INSERT RELATION` / the SDK's
relation insertion path. Ordinary INSERT of an edge-shaped object is not the same adjacency
realization. Preserve distinct assertions between identical endpoints; no blanket unique endpoint
pair. ENFORCED checks endpoint existence/table families, while admission checks semantic roles,
scope and membership. Load every addressable assertion endpoint before its dependent relation;
reify n-ary structures through role-labelled participants. Keep isolates explicitly.

Use small INLINE kind/role/context fields when they avoid record fetches, and vertex INLINE
edge/reference caches where low-degree immutable access benefits. These are different mechanisms:
an inline-property filter can select adjacency keys rather than the vertex cache. Choose them by
the actual composed query, not an assumption that every feature's gains multiply. Native fixed
traversals and their limits are the ordinary serving path. Directional compound indexes are
available for continuation-heavy pages where they simplify the operation.

### 2.2 Value boundary

Typed SDK values/`SurrealValue` or the appropriate wrapper encode/decode model-owned values.
Canonical string record keys avoid numeric-composite equivalences and ambiguity. A string that
looks like a record ID is still a string. Validate unsigned bounds before encoding; distinguish
NONE, NULL and tagged domain uncertainty. Exact byte contracts, signed zero and consumed float32
vectors remain canonical bytes with a digest; the engine's numeric array is their search lowering.
Canonical payload/content hashing is not delegated to arbitrary JSON or lossy numeric roundtrips.

The codec maps both the typed payload and all query-visible endpoints, fields and denormalizations.
Reconciliation checks the whole served representation: intact opaque payloads cannot hide wrong
adjacency, eligibility fields or witness links. Schema enforcement is useful structural protection,
not a second authored semantic model. Avoid independently maintaining semantic Rust and DB checks.

### 2.3 Original evidence

Lower the existing `SourceArtifact` / `ArtifactChunk` byte contract into artifact/chunk records.
Keep canonical 1 MiB chunks, ordinal ordering, complete length/digest and original source context;
transport batch size is independent. Use the existing streaming `ArtifactVerifier` rather than a
new DB-specific definition. Store bytes for binary/non-UTF8 artifacts without text coercion.
Serving reads the needed chunks/ranges through one coarse operation and preserves original anchors.

Native buckets or external objects are optional later realizations when actual artifact access
justifies them. Do not retain PostgreSQL for source bytes. If objects/modules are later external,
publish immutable digest-addressed bytes before graph pointers and account for their restore
separately; graph export alone does not copy them.

## 3. Checked loading, sealing and publication — P2

The initial state machine is private construction → admitted load → ready/drained reconciliation
→ published realization → optional selection → retired. Failure before visibility leaves an
unreachable target that can be dropped and rebuilt. No transaction spans compilation, embedding,
the whole load or final CPU validation.

1. Create a fresh strict database with its physical definitions and a single supervised loader.
   The admitted artifact and declared operation/index specs identify everything to install.
2. Load byte-bounded record batches, then dependent relation batches, using bound values and
   bounded concurrency. Use ordinary checked INSERT, not INSERT IGNORE. Inspect every statement
   through `Response::check` and typed results; permission-filtered writes can appear successful.
3. Build remaining required indexes, await their actual ready state, and finish all answer-affecting
   functions/analyzers/permissions/modules. Definition acceptance is not index construction success.
4. Drain/close every content and definition writer before final reconciliation. The only loader
   is supervised; terminate its mutable contexts and revoke surviving loader access. Install
   read-only serving access. Neither private naming, a READONLY field nor revocation alone is a
   native frozen-database guarantee.
5. Stream canonical stored content and compare complete identities/payloads/counts/digests with
   the artifact. Verify physical endpoint/role mapping, graph adjacency and query-visible derived
   fields through the owning mapping. Compare installed definitions with the realization manifest.
   Shared index mechanisms need focused query-path controls, not a second database implementation.
6. Write the completed immutable handle in one short control-record transaction. Serialize this
   single-operator publication operation. Selection is a separate small update; publishing does
   not change an existing reader's pin.

Use a deterministic chunk identity and content when interpreting uncertain write outcomes. On a
lost acknowledgement, reconcile that bounded chunk; identical complete content may finish it,
conflicting/partial content fails or restarts the private target. Do not blindly retry arbitrary
non-idempotent writes or accept a count as an exact-content acknowledgement. For disposable
failed builds, rebuilding is simpler than a generic recovery journal.

The stored reconciliation is one purposeful bulk boundary, not every-stage readback or routine
request validation. SDK incremental rows remain provisional until terminal statement success;
late stream failure never produces a completed realization/export. Streaming receiver segments
need not buffer the whole database to recover atomicity.

### 3.1 Executable realization and trust

Bind semantic content/admission to codec/layout revision, installed operation bodies/revisions,
analyzer/index specifications, exact embedding spec, engine version and any module bytes/settings.
Definitions and content are fixed before visibility. Index maintenance preserving the declared
contract is different from changing a search/operation realization. Build a new database/handle
for answer-affecting changes, reusing admitted content rather than rerunning providers.

Separate trusted operator/installer, private publisher/loader and database-scoped VIEWER access.
The serving process gets only its snapshot database handle and read-only operation capability;
no mutable control/cache credentials or loader clients. A VIEWER system identity can bypass
record permissions intentionally because the database contains only admitted snapshot data.
It cannot be used as a filter for mixed untrusted tenants. Approved function bodies have no writes
or remote enrichment. Actual mutations may be silently denied, so affected-content checks rather
than outer success establish write outcomes.

Administrative access remains trusted recovery, outside the ordinary immutable-reader contract.
An explicit audit streams content/definitions when an anomaly needs diagnosis; routine requests
do not rehash the database. Recovery closes affected readers and replaces the realization.
The implemented seal/audit inventory uses effective native database and table metadata, retaining
fields, indexes, events, functions, analyzers and other declared executable groups. Credentials
and live subscriptions are excluded. SurrealDB 3.3 INFO omits database STRICT mode; audit does not
certify it. The publisher explicitly creates STRICT databases. Portable transport exports only
canonical graph/role/original/manifest tables, rebuilding executable definitions and derived search
in a fresh final database on restore.

### 3.2 Selection, pinning and retirement

A complete handle names semantic snapshot, realization digest and exact database. A client/session
is dedicated to that database; do not mutate database selection on a shared active client.
MCP pins once for its lifetime, including resources and multi-call continuation. A selection
change affects new processes only. For one operator, supervise known readers rather than adding
renewable distributed leases. Retirement stops/drains those readers before dropping the database.
Retain only current consumers; there is no automatic snapshot history or structural-sharing layer.

## 4. Cache, operator integration and deletion — P1/I1

Move the live embedding-cache contract to the mutable control/cache database, initialized fresh.
Key it by complete embedding spec/text identity; validate winner bytes/spec/admission and reconcile
concurrent same-key writes. Batch lookups/writes. Exact consumed winners are copied into each
admitted graph, so serving/replay never depends on a mutable cache. Cache failure does not
manufacture successful vectors. Do not change the embedder/model during the store pivot.

The former `lctx_ops` / `runs` were historical-only interfaces. They and the generation store are
removed; new compiles use the private workspace. Do not recreate an event-history service
in SurrealDB. New attempts use existing structured logs and the minimal private/public target
state needed for publication. LIVE/events are optional notifications, not publication authority.

Replace store configuration/installation/check/reset/query and snapshot-management CLI, Python
native bridge lifetimes, readiness fixtures, backup/export tooling, and explicit test prerequisites.
Provide a SurrealDB operator runbook with the selected server, credentials, checked queries,
publication/index readiness, export/restore and reader quiescence. Preserve explicit/redacted
secrets and failure outcomes; remove PostgreSQL environment fallback and role/schema machinery.
Do not drop other projects' PostgreSQL databases or remove the host PostgreSQL installation.

Delete `lctx-postgres`, its generation/services/migrations, SQLx/pgpq/pgvector-only dependencies,
PG-native Python build closure and PG-specific scripts/specs/tests after their actual consumers
move. Remove alias/compatibility paths in the same boundary change. Dependency/runtime scans
confirm closure; no broad cargo clean or unrelated tooling repair is needed.

## 5. Focused functional controls and completion

| Boundary | Revealing control |
|---|---|
| Typed values/adjacency | Known IDs, parallel assertions, edge-as-endpoint/reification, isolates, tagged unknowns, byte/signed-zero/vector roundtrip and rejected out-of-range integers |
| Bulk load | Statement error inside outer success, denied writes, identical/conflicting chunks, missing/extra/changed body, wrong relation-write path and late stream failure |
| Originals/cache | Reordered/missing binary chunks; exact consumed cache winner/spec; conflicting cached value and spec mismatch |
| Readiness/sealing | Missing/failed required index, altered function/analyzer/derived field, open writer or stale mutable context prevents visibility |
| Persistent lifecycle | Restart with a published handle; interrupted private load remains invisible; publication completed but selection acknowledgement lost; original readers retain their pin |
| Restore/retirement | Fresh export/import reproduces content/definition/source-byte identity and usable query paths; active readers close before deletion |

Use owned disposable persistent 3.3 server databases and actual SDK credentials for those tests.
Mem is useful for local checks only. HTTP logical export/import is the initial portable restore
route; test gRPC streaming if used, including missing final success. Restoring a copy requires a
fresh reconciliation/ready handle; a transport file is not itself published trust.

Run compile checks and affected `verify-store`, `verify-serving` or `verify-tooling` controls
with their new prerequisites. Q0 owns integrated acceptance through the targeted actual journeys
selected by the user, without broad qualification or legacy parity. Fresh operator store reset,
real-library loading, embedding and selection are separately authorized Q1 actions. No runtime or
Measured performance outcome is asserted by this plan; actual functional outcomes are recorded
in coordinator §8.

Source anchors for library-specific decisions: [bulk relations/table contracts](https://surrealdb.com/docs/reference/query-language/statements/define/table),
[typed streaming finality](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/method/query.rs),
[gRPC/export implementation](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/engine/remote/grpc.rs)
and the capability review's detailed versioned evidence map. Current web documentation can be
newer than the inspected runtime; source/tests at the selected family decide applicability.
