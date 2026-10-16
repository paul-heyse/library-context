# Exact SurrealDB RPC stream shape and admission patches

Vendored from the crates.io `surrealdb-rpc` 3.3.0 source. The package version and all
nominal dependency types remain 3.3.0. `EmptyValue` preserves an absent gRPC SINGLE
payload as NONE. The legacy row stream suppresses it; the raw SDK batch stream exposes
it. The framing consumer retains the existing terminal `single` wire field.

The catalog-efficiency adapter additionally retains library-neutral ingress reservations
through generated decode, conversion, queued delivery and response ownership. Configured
unary replies and raw query results use the same owner. The byte-framing extension adds
`max_batch_bytes` at QueryRequest tag6, guarded by the peer capability
`QUERY_BATCH_BYTES_V1`; it requires the matching protocol and server patches. Package
versions remain unchanged. The engine-api, types and Tonic patches carry the corresponding
reservation and codec boundaries; their own provenance records identify the exact changes.

Physical EOF is established only by checked transport completion. A local decoder or
admission refusal may end delivery while physical completion remains unconfirmed. The
native adapter preserves that uncertainty and retains its original session. Escaped bare
SDK values require a destination application owner before their source response is dropped.
Acceptance and remaining qualification belong to the active catalog-efficiency plan.
