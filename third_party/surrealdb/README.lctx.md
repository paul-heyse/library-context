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
