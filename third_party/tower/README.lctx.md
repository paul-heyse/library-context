# Buffered request tracing origin

Base: the published Tower 0.5.3 archive. `provenance.json` records the archive,
upstream revision and original/patched hashes. Archive SHA-256:
`ebe5ef63511595f1344e2d5cfa636d973292adc0eec1f0ad45fae9f0851ab1d4`.
The only modified upstream files are `src/buffer/message.rs`,
`src/buffer/service.rs` and `src/buffer/worker.rs`. No package version or dependency
requirement changed.

The private buffer message now retains a span together with its actual originating
`tracing::Dispatch`. The field wrapper releases that span inside the dispatch,
including canceled, queued and unprocessed messages. `Message` itself does not
implement `Drop`, preserving independent request/sender field moves.

The worker installs each message's dispatcher before entering its span and polling
readiness or calling the service. Both guards exist only inside the synchronous
`Future::poll`; none crosses a suspended poll. Different requests on one worker may
have different originating dispatchers. Service futures delivered to callers keep
their existing public behavior and are not newly wrapped by this patch.

This prevents tracing-subscriber 0.3.23 from routing a parent span's final release
to an unrelated ambient registry. Patching tracing-core's `try_close` is avoided:
its dispatcher access holds thread-local borrowing state, and changing defaults
inside that generic callback is not the same ownership boundary.

DB-free regression controls live in `crates/lctx-tracing/src/tests.rs`. They cover
successful delivery, queued/pending cancellation, queued/pending worker shutdown,
receiver loss after reservation, response-send refusal and readiness failure, plus
two originating subscribers sharing a foreign-polled worker. They assert exact
origin closures, originating readiness/call context and untouched foreign spans.
