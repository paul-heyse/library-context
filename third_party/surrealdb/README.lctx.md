# SurrealDB gRPC physical completion backport

This is the published `surrealdb 3.3.0` SDK, selected through the root
`[patch.crates-io]`. Its crate archive SHA256 is
`f04b42991e6a01e2d01f1ea6183714cf364d3c40fa2197bd8993ec7834e0b4c4`;
its upstream source revision is `238bfeb11f5725bebed370167656748df8067595`.
The standalone Cargo.lock and Cargo cache markers are omitted. Upstream license
texts and the remaining published source are retained.

The physical terminal change is in `src/engine/remote/grpc.rs`. On a successful
application `QueryResponse::End`, export `DataTrailer`, or subscription `End`,
the SDK now retains and polls the tonic stream through its checked transport
EOF. A late gRPC status or protobuf error remains an error. Additional payload
or a duplicate application terminal is refused. Previously these paths dropped
the physical stream at the application terminal, before receiving the gRPC
status and HTTP/2 completion. Intentional cancellation when a consumer's result
channel closes retains its original behavior.

The private drain helper has revealing controls for delayed EOF, late transport
status, late protobuf errors, and payload/duplicate terminal after application
End. These controls exercise the helper with finite streams; actual transport
qualification also requires the repository's native SDK and large selection
controls. Source inspection alone does not establish that this gap caused an
observed HTTP/2 cancellation. Verification results belong in the active native
execution plan, not in this provenance file.

Remove this patch when a qualified published SDK provides checked physical
completion for these paths. Do not edit the shared Cargo registry cache.

The HS7 logical-export adapter additionally exposes borrowed `Transaction::cancel_ref` and
`invalidate_session`. They use the original router, session and transaction ID so failed
rollback acknowledgement cannot discard the only exact-session finalization handle. Export
retains the transaction through checked query EOF, explicit cancellation and invalidation.
`Surreal::clone` creates another session and is not a substitute. These methods do not make
static source feasibility a qualified composed snapshot-export result.

The external transaction adapter also exposes `Begin::retain_on_error`, retaining the exact consumed exporter session for explicit invalidation when the begin RPC fails. No dependency revision or ordinary clone semantics changed. Runtime composition is qualified by the consumer snapshot/cancel controls, separately from source review.

The finite-query `Query::stream_batches` adapter owns and polls the engine future together
with its raw bounded delivery channel. EOF waits for both, including late execution/physical
transport errors. `BatchItem` preserves Rows, Value, EmptyValue and Finished; the exact
`surrealdb-rpc` 3.3.0 patch carries an absent SINGLE payload as EmptyValue. Empty batched
payload remains Rows, and the gRPC End result count is checked. The existing `stream_items`
mapping still suppresses empty rows and EmptyValue. Streaming registers no LIVE subscriptions;
finite generated read consumers drain this owner and retain the original borrowed session.

The catalog-efficiency admission adapter adds library-neutral `IngressAdmission` owners,
per-query `QueryStreamOptions::max_batch_bytes`, retained admitted batch/failure values,
and configured bootstrap/control reply admission. `QUERY_BATCH_BYTES_V1` is required before
issuing a byte-framed query; an older server is refused rather than silently receiving an
unbounded fallback. The root patch set also pins the protocol, RPC, engine-api, types and
Tonic source seams at their existing exact versions; each new vendor owner has its own
archive and changed-file provenance. Server framing is an opt-in owned generation recipe.

Configured raw query responses keep an `AdmittedQueryResults` owner through engine and
SDK indexing. Bare extracted SDK values require an application owner before the response
is dropped. The native adapter transfers backing under its actual ResourceBudget and
materializes shared Bytes before source reservations release. Native owned row delivery
retains queue and current-row reservations through separate acknowledgments. Compiled
Regex needs an explicit qualified compiler hook; the catalog profile refuses it.

The admission claim covers the selected owned framing, decompression, generated Prost,
conversion, queue and retained-value buffers, including their coexistence. It does not
claim a bound on process RSS, shared H2/TLS/network/runtime scheduler allocation, arbitrary
external bare SDK outputs or legacy-zstd peers. Configured gzip and modern zstd use admitted
workspaces. Current acceptance belongs to the catalog-efficiency coordinator; old physical-
terminal controls do not qualify these new seams.

The acknowledged-session adapter exposes gRPC `close_session` and a separate non-retaining
`connection_completion` join for the SDK session-lifecycle task. Close is ordered with existing
clone/drop events; interrupted or repeated waits observe one retained result. Sibling sessions
remain usable, and closed handles cannot create fresh authenticated sessions. Failed close retains
its typed cause and allocation grants. Connection joining follows actual client-owner drop and
retains its own backing through the final token/error borrower; it does not qualify shared H2/TLS
or runtime workspace or remote executor drainage. Native consumers check query/pin/source finality
separately before close. Other engines explicitly refuse this qualified API while retaining their
original public session-event interface. Transaction::close_session borrows its exact client.
No server wire change or dependency version move is involved.
