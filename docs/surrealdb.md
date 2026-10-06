# SurrealDB operator runbook

**Implemented / focused Tested route, 2026-10-06; operator activation not_run.** The native server hosts fresh,
immutable graph snapshots and a separate mutable embedding cache. Publication consumes a verified
compiler export without replaying providers. The [pivot coordinator](plans/graph-native-pivot-plan_2026-10-05.md#8-current-checkpoint)
owns actual functional receipts and their limits. These commands are explicit operator actions;
this execution used disposable databases only.

## Server and configuration

Use the reviewed SurrealDB 3.3 server, with persistent RocksDB storage and authentication. The
owned control fixture selects image
`surrealdb/surrealdb@sha256:681c6c22c287421b5c7d99e0fde79b6e0d32c36c1ddeaab2762a1661cb04cd20`.
Bind its port to loopback; gRPC and HTTP use that same port. Configure a 20-second query timeout
and 10-second transaction timeout. Choose block-cache and write-buffer sizes for the server
allocation using the actual 3.3 variables `SURREAL_ROCKSDB_BLOCK_CACHE_SIZE`,
`SURREAL_ROCKSDB_WRITE_BUFFER_SIZE` and `SURREAL_ROCKSDB_MAX_WRITE_BUFFER_NUMBER`.
Leave allocation room for requests, other engine state and background compaction. The server
memory threshold is a guard rather than an RSS cap; retain synchronous durability and background
maintenance. Host-derived defaults can exceed an intended container allocation. The owned fixture explicitly sets a 64 MiB block cache, 32 MiB write buffers with at most two buffers, a 512 MiB tracked-memory threshold and a 1 GiB container limit. These are fixture choices, not universal capacity recommendations. Default durable `Every` synchronization is preserved. See `scripts/surrealdb_fixture.py` for the launch
options, persistent restart and readiness checks. It never inspects the operator store.

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
# Alternatively, prepare and independently verify an export before publication:
lctx compile fastmcp --through catalog --artifact-only --output build/admitted
lctx publish-artifact build/admitted --runtime-config build/native/runtime.json
```

Ordinary compilation and publication return an unselected handle. Save that JSON as
`build/native/handle.json`; selection is a separate action:

```sh
lctx snapshot --runtime-config build/native/runtime.json select build/native/handle.json
lctx snapshot --runtime-config build/native/runtime.json show
lctx snapshot --runtime-config build/native/runtime.json query 'SELECT VALUE id FROM entity LIMIT 5'
python3 scripts/build_environment.py -- uv sync --locked
uv run --no-sync python -m lctx_mcp --serving-config build/native/selected.serving.json
```

Publication installs strict typed families, enforced native roles and indexes/functions, loads
bounded batches, reconciles canonical content, adjacency, original chunks and derived search
materialization, then invalidates its writer before returning a checked read-only handle. Failure
keeps the attempted database unreachable and removes it. A transport dump or copied publication
marker does not establish publication trust.

Every running MCP process pins its complete handle. Selection affects new processes. Restart
retains persistent content; an answer-affecting definition, index policy or Rust operation change
requires a fresh realization. The linked service checks its sealed operation implementation.
Build the native Python package from the current checkout before starting clients; a previously
installed wheel may represent an earlier Rust operation definition.

`lctx snapshot list` inspects complete publications in the configured namespace. An explicit
`lctx snapshot audit build/native/handle.json` reconciles canonical content and checks current
database/table definition inventory against the sealed realization, and compares every derived
query-visible search/scope/vector/witness row with the shared canonical lowering without writes. This is a cold operator
action, not per-request corpus accounting. Users/access definitions and live subscriptions are
outside that fingerprint; SurrealDB 3.3 metadata does not expose the database's STRICT mode.
Audit therefore does not certify credentials, live subscriptions or database mode. Publication
itself explicitly creates STRICT databases and a separate read-only viewer.

## Embeddings and native search

Keep the existing exact Qwen specification and inference service. Compilation opens cache/inference
only when requested; consumed winner bytes enter admitted content. MCP's optional
`--embedding-url` names the existing service. Omission disables vector search; service failure
reports degradation while native lexical search remains available. No service is started or
upgraded implicitly. BM25 and shared-vector HNSW candidates are restricted to eligible contextual
occurrences before their finite channel caps; analytical exact-neighbor contracts remain separate.

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

`python3 scripts/surrealdb_fixture.py -- COMMAND` owns one authenticated persistent disposable
server, supplies `LCTX_SURREAL_TEST_CONFIG`, and removes only its own container and scratch files.
Use the current `verify-store` / `verify-serving` routes and explicit filters. Compiler-stage tests
stopped by the user are not prerequisites to restart. Real FastMCP compilation, live Qwen inference
and operator selection remain separately authorized Q1 work.

Logical text is declared by the model field type independently of Arrow storage. Native query
projections preserve exact `Utf8Text`, including enum fields, and exclude opaque byte/vector
payloads. The 2026-10-06 field-contract migration changes model schema identity; rebuild artifacts
and realizations from current inputs rather than retaining an old-format reader.
