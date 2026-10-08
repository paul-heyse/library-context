# SurrealDB operator runbook

**Accepted target / implementation in progress, 2026-10-07:** [Persisted graph execution](plans/persisted-graph-execution-plan_2026-10-07.md) coordinates the new compiler and shared-consumer pivot. The same managed persistent server remains the selected host. PG5/PG7 change compiler/import/restore boundaries and completed-state format; ADR-0133 owns this execution boundary; current acceptance is in STATUS and the persisted plan. Operator runtime actions and real-library pilots remain held during this implementation.

**Implemented / focused Tested route, 2026-10-07; operator selection/adoption held for catalog compilation design revision.** The native server hosts fresh,
immutable graph snapshots and a separate mutable embedding cache. Compilation writes exact
completed native state and ordinary publication seals that same database. Explicit external import
consumes a verified complete export without replaying providers. The
[persisted execution plan](plans/persisted-graph-execution-plan_2026-10-07.md#91-current-execution-checkpoint-2026-10-07)
owns current functional receipts; earlier receipts remain with the graph/evaluation coordinators. Disposable databases own contract validation.
Earlier combined execution authorized local operator preparation; its recorded persisted server
and embedding cache are preserved. No operator-state change is authorized by the current fixture checks. Fresh library compilation stopped without a verdict, and selected-snapshot adoption is held for the [catalog compilation speed review](design_review/reviews/design_review_catalog-compilation-speed_2026-10-07.md). The persistent server and cache remain available; this review did not change service configuration.

## Server and configuration

Use the reviewed SurrealDB 3.3 server, with persistent RocksDB storage and authentication. The
owned control fixture runs the native 3.3.0 release binary, byte-identical to `/surreal` in image
`surrealdb/surrealdb@sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20`, and
checks its sha256 and `surreal version` before every start ([pins](pins.md#native-runtime-control);
`LCTX_SURREAL_BIN` selects the file). It binds an explicit loopback port that a restart keeps; gRPC
and HTTP use that same port. Configure a 20-second query timeout
and 10-second transaction timeout. Choose block-cache and write-buffer sizes for the server
allocation using the actual 3.3 variables `SURREAL_ROCKSDB_BLOCK_CACHE_SIZE`,
`SURREAL_ROCKSDB_WRITE_BUFFER_SIZE` and `SURREAL_ROCKSDB_MAX_WRITE_BUFFER_NUMBER`.
Leave allocation room for requests, other engine state and background compaction. The server
memory threshold is a guard rather than an RSS cap; retain synchronous durability and background
maintenance. Host-derived defaults can exceed an intended allocation. The owned fixture explicitly sets a 64 MiB block cache, 32 MiB write buffers with at most two buffers, a tracked-memory threshold of half its cap and a user-systemd `MemoryMax` cap (`LCTX_FIXTURE_MEMORY`, default 16 GiB, with an 8 GiB tracked-memory threshold; idle RSS is about 200 MB, so caps below about 250 MB fail at startup). An OOM kill is reported as an infrastructure failure. These are fixture choices, not universal capacity recommendations. Default durable `Every` synchronization is preserved. See `scripts/surrealdb_fixture.py` for the launch
options, persistent restart and readiness checks. It never inspects the operator store.

Set `SURREAL_GRPC_MAX_MESSAGE_SIZE=128MiB` for this native row contract. SurrealDB3.3 defaults
to4MiB and advertises that ceiling to the Rust SDK; the SDK takes it by default. Native writes
target128 rows with the existing8MiB byte target, and an admitted single larger row travels alone
up to64MiB. Its RPC framing also needs room. This transport ceiling permits indivisible rows;
it does not increase transaction timeouts or remove native row/byte guards. Configure it before
starting the managed server; current implementation controls change only their owned fixture.
The operator deployment remains held. The pinned implementation couples the server's advertised,
decoding and encoding ceilings in [server configuration](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/server/src/cnf/mod.rs)
and the [SDK](https://github.com/surrealdb/surrealdb/blob/v3.3.0/surrealdb/src/opt/grpc.rs)
reconciles them; setting only a client limit cannot raise the server's request ceiling.

**Inspected, 2026-10-07:** the installed system CLI is visible as `surreal` in a fresh terminal
and reports `3.3.0`; `surreal start --help` accepts an explicit durable store path and uses best-effort
planning by default. The pinned 3.3.0 source also enables a cross-transaction definition cache
(`SURREAL_DATASTORE_CACHE_SIZE`, default 1,000 entries) and a process-shared HNSW vector cache
(`SURREAL_HNSW_CACHE_SIZE`, default 256 MiB). Retain these caches; size them together with
RocksDB and request memory for the deployment. The HNSW graph is loaded separately and is not
bounded by the vector-cache setting. The [index documentation](https://surrealdb.com/docs/reference/query-language/statements/define/indexes)
describes that distinction. Persistent storage preserves data and indexes across restart;
in-memory caches warm again. Neither establishes a query-speed claim. Our readers share their
pinned SDK client, and ranked continuations reuse their retained candidate pool rather than
rerunning discovery. Caching complements bounded indexed queries; it does not replace removal
of repeated correlated scans. Current eligibility follows indexed record adjacency before channel
quotas; lexical winner scores are prepared once in a keyed dictionary. No general result cache or
duplicate canonical store is required.

Request hydration follows keyed record adjacency. Original-evidence incoming ownership uses
the packet's declared source families; its wider outgoing proof dependencies do not grant
incoming ownership to unrelated analytical values. Recognized native engine timeouts return
the typed resource-refusal envelope. A normal response can refuse when its indivisible core,
coverage and delivery map exceed the budget; use the existing expanded request mode for larger
packets. The limits remain protective guards rather than a guarantee that every packet fits.

Keep `build/native/runtime.json` outside Git and mode 0600. Its closed fields are:

```json
{
  "endpoint": "grpc://127.0.0.1:8000",
  "username": "installer",
  "password": "<root installer secret>",
  "viewer_username": "snapshot_reader",
  "viewer_password": "<different database reader secret>",
  "namespace": "lctx",
  "cache_database": "cache",
  "selection": "build/native/selected.json"
}
```

The root installer creates private databases and their database-scoped VIEWER user. Product
serving receives only the emitted `selected.serving.json`: endpoint, database VIEWER credentials
and the absolute path of the single atomic selection file. New sessions read that handle once;
running sessions retain their pin. Never pass runtime configuration to MCP, log secrets, or share a
mutable SDK database-selection context between readers. The current SDK runtime accepts reviewed
3.3 engines; another server family requires checking its protocol and physical contracts first.

## Compile, publish and select

```sh
lctx store --runtime-config build/native/runtime.json init
lctx store --runtime-config build/native/runtime.json check
lctx compile fastmcp --through catalog --runtime-config build/native/runtime.json
# Export complete graph and compiler state from the same native compiler:
lctx compile fastmcp --through catalog --artifact-only --output build/admitted --runtime-config build/native/runtime.json
lctx publish-artifact build/admitted --runtime-config build/native/runtime.json
```

Ordinary compilation and publication return an unselected handle. Save that JSON as
`build/native/handle.json`; selection is a separate action:

```sh
lctx snapshot --runtime-config build/native/runtime.json select build/native/handle.json
lctx snapshot --runtime-config build/native/runtime.json show
lctx snapshot --runtime-config build/native/runtime.json query 'SELECT VALUE id FROM entity LIMIT 5'
just sync native
uv run --no-sync python -m lctx_mcp --serving-config build/native/selected.serving.json
```

Compilation creates a private STRICT database before producing any output. Immutable completed
contributions/views and bindings exclude pending rows; original chunks have one physical byte owner.
Ordinary publication admits and reconciles that same database, builds native roles/search indexes,
closes operation admission, drains retained work and invalidates its writer before returning a
read-only handle. Explicit import independently validates graph plus completed state. Failure
keeps the attempt unselected and removes it; uncertain cleanup reports its owned database name. A transport dump or copied publication
marker does not establish publication trust.

Every running MCP process pins its complete handle. Selection affects new processes. Restart
retains persistent content; an answer-affecting definition, index policy or Rust operation change
requires a fresh realization. The linked service checks its sealed operation implementation.
Build the native Python extension from the current checkout before starting clients. During
development, Cargo can build the cdylib and the editable package can load it directly; wheel
packaging is unnecessary. A previously installed extension may represent an earlier Rust
operation definition. Drain native workers before replacing the local extension.

`lctx snapshot list` inspects complete publications in the configured namespace. An explicit
`lctx snapshot audit build/native/handle.json` reconciles canonical content and checks current
database/table definition inventory against the sealed realization, and compares every derived
query-visible search/scope/vector/witness row with the shared canonical lowering without writes. This is a cold operator
action, not per-request corpus accounting. Users/access definitions and live subscriptions are
outside that fingerprint; SurrealDB 3.3 metadata does not expose the database's STRICT mode.
Audit therefore does not certify credentials, live subscriptions or database mode. Publication
itself explicitly creates STRICT databases and a separate read-only viewer.

## Embeddings and native search

Use the selected full4096 Qwen specification and the existing custom inference service. The
encoder identity includes the actual checkpoint/tokenizer/runtime/output semantics; document and
query recipes are independently identified. Complete document inputs are admitted with the acquired
local tokenizer at2048 tokens. Query admission uses its separate8192-token recipe. Compilation
opens cache/inference only when requested; exact normalized full winners and their declared
normalized1024 prefix projections enter admitted content and replay without the mutable cache.
Search and E1 analytics share that explicit1024 representation; bounded search rescoring uses the
full winners. MCP's optional
`--embedding-url` names the existing service. Omission disables vector search; service failure
reports degradation while native lexical search remains available. No service is started or
upgraded implicitly. BM25 and shared-vector HNSW candidates are restricted to eligible contextual
occurrences before distinct target/context channel caps. Exact name/path/option matches have an
independent prioritized lane, including literal matches with zero BM25. Session-retained ranked
continuations reuse the initial candidate pool and query vector under the original snapshot,
request and policy. Analytical exact-neighbor contracts remain separate.

## Export, restore and retirement

`lctx snapshot export` writes a named input/context projection, with semantic IDs, isolates,
parallel arcs, lineage, coverage and gaps. It is an analytical export, not a database backup:

```sh
lctx snapshot export --projection <name> --input <32-hex-id> --context <32-hex-id> --output graph.json
```

```sh
lctx snapshot backup --handle build/native/handle.json --output snapshot.surql
lctx snapshot restore snapshot.surql
# After readers stop and this handle is no longer selected:
lctx snapshot retire build/native/handle.json --readers-stopped
```

Portable backup uses the SDK's terminally checked gRPC file export on the same local authority,
restricted to canonical entities/assertions, native role arcs, originals and the manifest.
Derived search tables and executable definitions are reconstructed rather than transported;
this also avoids SurrealDB 3.3's export/import mismatch for nullable fixed-array search fields.
Restore imports through the checked HTTP import response into a fresh private staging database,
reconciles canonical content and performs pure semantic re-admission without providers, reconstructs
current definitions and derived indexes in a fresh final database, then issues a newly reconciled
handle. It does not select that handle or inherit imported users and executable authority.
Never treat imported historical handles as current trust. Before retiring a snapshot, stop and
drain every known CLI/MCP reader and remove any selection pointing at it. Do not retain rollback
copies or old-format readers without a current consumer. Other projects' PostgreSQL stores and
the host PostgreSQL installation are outside this pivot.

## Focused controls

`just fixture -- COMMAND` owns one authenticated persistent disposable server, supplies
`LCTX_SURREAL_TEST_CONFIG` and `LCTX_COMPILER_RUNTIME_CONFIG`, and removes only its own server and
state (`--keep`, `--attach`, `--list` and `--stop` manage kept servers).
Fixture-backed verification boundaries (`providers:extract`, `compiler:producer`,
`compiler:cli`, `store:rust`, `serving:rust`, `serving:mcp`) each receive their own attachment,
with both variables, through `just verify --select BOUNDARY --nextest-args "-E '<filter>'"` (or
the `just verify-<family> --command BOUNDARY -- …` shortcuts); `just verify --print …` shows the
resolved commands without building. `--attach ID` uses a kept fixture, and `serving:mcp` can
reuse retained content (`--attach ID --serving NAME`, produced by `--retain-serving NAME`); the
skipped journey is recorded as `not_run` with the content identity. These controls use owned
persistent fixtures; they do not inspect the operator store. The previously stopped broad compiler
suite is not restarted. Current native/MCP/programmatic acceptance follows the
[persisted execution plan](plans/persisted-graph-execution-plan_2026-10-07.md).
Real FastMCP Catalog/Behavioral compilation, Q1 selection and operator adoption remain held.

Logical text is declared by the model field type independently of Arrow storage. Native query
projections preserve exact `Utf8Text`, including enum fields, and exclude opaque byte/vector
payloads. The 2026-10-06 field-contract migration changes model schema identity; rebuild artifacts
and realizations from current inputs rather than retaining an old-format reader.
