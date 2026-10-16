# SurrealDB 3.3.0 operation diagnostics and MCP access

**Question.** What does the pinned SurrealDB server offer for diagnosing operations (efficiency and
correctness, during execution and after), and what can its HTTP `/mcp` surface do for an agent
consumer? Input to a design review (assignment A, 2026-10-15); evidence, not a decision.

**Binary.** The project's owned server generation `405f6c02…`
(`~/.local/state/library-context/surrealdb.tools/surreal/405f6c02…/surreal`, sha256
`710229df33346fdf9733ef86d46e31874f0cacc2691529327e19e6534477b8ed`), reporting
`3.3.0+lctx.timeout-stack.1`: upstream `238bfeb11f…` (v3.3.0) plus the transaction-timeout
TreeStack repair, default+`cjk` features, Community edition (`target_info{service_edition="community"}`).
Source read is the exact build tree recorded by that generation (`…/server-build/3efaf277…/source`;
`surrealdb/core/src/dbs/executor.rs` matches the patched sha256 `281d6cac…`). Paths below are
relative to that tree's `surrealdb/` directory.

**Method.** One private memory-engine server on loopback `127.0.0.1:18461` (root credentials
generated into a scratch file outside the repository, passed by environment, deleted after), with
`--log=warn --query-timeout 2s --transaction-timeout 10s --slow-log-threshold 150ms
--slow-log-param-deny secret`, `SURREAL_SLOW_QUERY_METRIC_THRESHOLD_MS=150` and OTLP export
(`SURREAL_TELEMETRY_PROVIDER=otlp`, per-signal `OTEL_EXPORTER_OTLP_{TRACES,METRICS,LOGS}_ENDPOINT`
to ports 18462–18464) into `otlp_sink.py`, a stdlib HTTP/2 OTLP receiver written for this probe
(no collector was installed). No RocksDB, no shared/operator/stable/campaign target. Both processes
ran under the managed run owner and were cancelled at the end:

| Run | Command | Outcome |
|---|---|---|
| `20261015T185242.347Z-da9475` | `just run --background --label opdiag-surreal-probe -- uv run --no-project python otlp_sink.py OUT 18462 18463 18464` | cancelled after probes (exit 143) |
| `20261015T185246.417Z-151af1` | `just run --background --label opdiag-surreal-probe -- start_server.sh BINARY 18461 PASSFILE 18461 2s` | cancelled after probes (exit 0) |

Drivers: `curl` (HTTP `/sql`, `/metrics`; gRPC via `--http2-prior-knowledge` with frames from
`grpc_frames.py`), `mcp_probe.py` (MCP over HTTP). Curated outputs: `observed/server_log.txt`
(server stderr), `observed/metrics_families.txt`, `observed/otlp_summary.txt`.

## Results (executed 2026-10-15 on the binary above unless marked source)

| # | Probe | Observed | Control |
|---|---|---|---|
| E1 | Slow log, `RETURN [sleep(300ms), $secret, $ok]` | One `WARN surrealdb_core::kvs::slowlog` line: `Slow query detected - time: … - query: <normalized single-line SurrealQL> - params: [ $ok='visible-param' ]`; `$secret` omitted (deny list). No ns/db/user/session/connection/trace id in the line. | Fast `RETURN [$ok]`: no line. |
| E2 | Slow log on timeout, `RETURN sleep(3s)` with `--query-timeout 2s` | Logged at 2.0009 s when the timeout fired; client gets `TimedOut`; metric `outcome="error",error_class="internal"`. | — |
| E3 | Slow log inside `BEGIN … COMMIT`, scans, updates | Each slow statement logged individually (`CREATE`, `SELECT … WHERE sleep()`, `UPDATE`, `LET`). Emitted only on completion. | — |
| E4 | Correlation by bound variable: `/sql?attempt=0f9e…` and statement referencing `$attempt`; SQL comments added | Slow line shows `params: [ $attempt='0f9e8d7c6b5a' ]`; comments are absent (statement is re-rendered from the AST). | Parameters not referenced by the statement are never printed (E1). |
| E5 | `/metrics` anonymous vs root Basic | Anonymous: 5 families (build info, uptime, memory, CPU %, `target_info`). Root: 30 families incl. `surrealdb_statement_total/_duration_seconds/_rows_total`, `surrealdb_query_*`, `surrealdb_transaction_*` (kv ops, keys/bytes read/written, conflicts), `surrealdb_rpc_*`, `surrealdb_http_*`, `surrealdb_network_*`, `surrealdb_slow_query_total`, `surrealdb_mcp_tool_*`. Statement labels: `statement_type, outcome, error_class, namespace, database, user`. | Database OWNER and VIEWER Basic users see only the anonymous 5 (root-only boundary). |
| E6 | `surrealdb_slow_query_total` | Counted per statement type ≥ threshold; also counted a `begin` statement whose block took > 150 ms. | — |
| E7 | In-flight visibility | `surrealdb_http_active_requests{http_route="/sql"}` = 1 while a 1.5 s `/sql` statement ran. | Same 1.5 s statement over gRPC `Query` (streamed): gauge = 0 for the gRPC route during execution (curl wall 1.51 s). |
| E8 | OTLP traces | Spans: HTTP `request` (`POST /sql`, `POST /surrealdb.protocol.rpc.v1.SurrealDBService/{*method}`, `POST /mcp`, …) with HTTP semconv attributes incl. `url.query`, `user_agent.original`, `client.address`, `http.request.id`; child `executor` spans (code location, thread, busy/idle ns only: no statement text, kind, ns/db or session in Community). Slow-log lines arrive as span **events** on the statement's `executor` span (full statement text and allowed params). | — |
| E9 | W3C `traceparent` | HTTP `/sql` with `traceparent: 00-a1a1…-b2b2…-01`: server `POST /sql` span has trace `a1a1…`, parent `b2b2…`, and the executor spans are its descendants. gRPC `Query` with `traceparent: 00-c3c3…-d4d4…-01` as HTTP/2 metadata: same parenting. | Requests without the header start fresh root traces. |
| E10 | OTLP metrics/logs | 30 metric names with the same attribute keys as `/metrics`. Logs endpoint received 0 records (Community has no audit/slow-query log pipeline; source `server/src/observe/provider.rs`). | — |
| E11 | gRPC per-statement stats | Each statement's terminal `QueryBatchFrame` carries `QueryStats{records_returned, bytes_returned=-1, records_scanned=-1, bytes_scanned=-1, execution_duration}` (e.g. 322 604 968 ns for the slow SELECT); `QueryBegin` carries a server `query_id` UUID and statement count; `QueryEnd` the whole-query duration. | — |
| E12 | `INFO FOR ROOT STRUCTURE` | `system{available_parallelism, cpu_usage, load_average, memory_usage, physical_cores, memory_allocated, memory_requested}`, plus `config{QUERY_TIMEOUT}`, nodes, namespaces, users (hashes redacted). No running statements/transactions. | — |
| E13 | `EXPLAIN ANALYZE [FORMAT JSON]` on SELECT (default `best-effort` planner) | Operator tree with per-operator `metrics{elapsed_ns, output_batches, output_rows, records_fetched}` and `total_rows`; text format `{rows, batches, elapsed}`. Legacy `… EXPLAIN` suffix also returns the new-planner tree. | `EXPLAIN ANALYZE … UPDATE`: error "EXPLAIN is only supported with the new execution model"; the UPDATE did not run (count unchanged). |
| E14 | Observer during a long write transaction | `BEGIN; CREATE prog:1; LET $w = sleep(1500ms); CREATE prog:2; COMMIT` in flight; a concurrent `SELECT * FROM prog` at 0.5 s returned in 0.2 ms with only committed state (table did not yet exist). | After commit: both rows. (Memory engine only.) |
| E15 | MCP root, protocol 2025-11-25 session | 14 tools; `readOnlyHint=true` only for `info`, `list`, `select`. `query` structured content: `{results:[{index,status,value|error,kind,time_ms,truncated}], has_errors}`; text block labels each statement. Resources: `surrealdb://instructions`, `info` (advertises protocol `2026-07-28`, version `3.3.0`), `version`; templates `surrealdb://schema/ns/{ns}/db/{db}[/table/{t}]` returned definitions/fields/indexes/events/lives. `list` needs `kind`. | — |
| E16 | MCP truncation | 300 002-byte result replaced by `{$truncated:true, limit_bytes:262144, original_bytes, message}` with `truncated:true`. | Small results `truncated:false`. |
| E17 | MCP stateless 2026-07-28 | `tools/call` without `initialize`, Basic auth + `surreal-ns/db` per request, `Mcp-Method`/`Mcp-Name` headers and `_meta` `protocolVersion`/`clientInfo`/`clientCapabilities`: succeeded. Missing `Mcp-Method` → 400 `-32020`; missing `clientCapabilities` → 400 `-32602`. | Legacy version without `initialize` → 422; wrong password → 401. |
| E18 | MCP as database VIEWER (Basic needs `surreal-auth-ns/db`) | `SELECT` works; `CREATE`/`UPDATE`/`DELETE` via `query` and the `create` tool return `status ok`, value `[]`, nothing written; `DEFINE TABLE` and `INFO FOR ROOT` → `NotAllowed`. | Data identical before/after. |
| E19 | MCP Host guard | `Host: evil.example.com` → 403 `Host header is not allowed`. | Loopback host accepted. |
| E20 | MCP logging at `--log=warn` | Tool failures and truncation appear as `WARN surrealdb::mcp`; the per-call audit (`INFO surrealdb::mcp::audit`) does not appear. | — |

## Source-only facts (read at the build tree; not executed)

- Slow log: `core/src/kvs/slowlog.rs:138-152` (`warn!`, single line, nullish params skipped,
  permissions not visited); called after execution for top-level `LET` and expression statements
  only (`core/src/dbs/executor.rs:1255,1338,1359`), so `KILL`, `LIVE`, `SHOW`, access statements
  are never slow-logged. Flags/env: `server/src/dbs/mod.rs:77-88`.
- Metrics: env gates `server/src/cnf/mod.rs:427-481` (`SURREAL_METRICS_ENABLED` default true,
  `SURREAL_SLOW_QUERY_METRIC_THRESHOLD_MS` default 1000, `SURREAL_PROCESS_METRICS_REFRESH_INTERVAL`
  default 5 s); root-only full view `server/src/observe/session.rs`, allowlist
  `server/src/observe/public.rs`; recording `server/src/observe/metrics.rs:674-760`.
- OTLP: gRPC (tonic) only, `server/src/telemetry/traces/mod.rs`; endpoint from the
  opentelemetry-otlp 0.31.1 env vars (`OTEL_EXPORTER_OTLP_[TRACES_|METRICS_|LOGS_]ENDPOINT`,
  default `http://localhost:4317`); W3C propagator installed only when OTLP tracing is active
  (`server/src/telemetry/mod.rs:336-346`); default OTel span filter `server/src/telemetry/mod.rs:477-494`
  (override `SURREAL_LOG_OTEL_LEVEL`). WebSocket carries `trace_context` in the RPC envelope
  (`rpc/src/request.rs:29-37,98-130`; `server/src/rpc/websocket.rs:100-123,900-915`).
- The project's SDK (vendored `third_party/surrealdb`, 3.3.0 + backport) sets no gRPC metadata or
  `trace_context`; `RequestContext` has only session/transaction/timeout
  (surrealdb-protocol 0.13.1). Per-statement `time` is mapped from `QueryStats.execution_duration`
  and defaults to zero when absent (`third_party/surrealdb/src/engine/remote/grpc.rs:1060-1065`).
- No running-query/transaction listing or kill exists in this tree (searched `running_quer`,
  `active_quer`, `inflight`, `list_queries`, `ShowQueries`, `processlist` across `surrealdb/`);
  `KILL` targets live-query ids.
- MCP: `mcp/src/cnf.rs` (`SURREAL_MCP_QUERY_TIMEOUT_SECS` 60, `SURREAL_MCP_MAX_RESULT_BYTES`
  262144, `SURREAL_MCP_ALLOWED_HOSTS`, `SURREAL_MCP_ALLOW_ALL_HOSTS`, no read-only switch);
  route gate `server/src/ntw/mcp.rs:55-59` (`RouteTarget::Mcp`); rmcp 3.1.2 `LocalSessionManager`
  keep-alive 300 s, init timeout 60 s, no session count cap found; audit target
  `mcp/src/audit.rs` (no query text, params or rows).

## Limitations

Memory engine only: RocksDB snapshot pinning, lock waits and write-conflict behaviour under a live
observer were not exercised. The OTLP receiver is a probe stub (no compression, no TLS); export
overhead and span volume were not measured. The Rust SDK was not run: SDK behaviour over gRPC is
source-read. Live select and changefeeds were not re-probed (prior skill probes SB041–SB043,
SB051, SB101, SB134, SB135 apply within their embedded/ws scope). Standalone SurrealMCP was not
run: its repository now only points to the built-in MCP; its last tag v0.4.0 depends on the
`surrealdb` 2.3.7 SDK (GitHub `surrealdb/surrealmcp`, read 2026-10-15), and its compatibility with
a 3.3.0 server was not established.

**Consumer.** Assignment A of the 2026-10-15 operation-diagnostics design review (coordinator:
root agent).
