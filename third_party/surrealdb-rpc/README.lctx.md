# Exact SurrealDB RPC stream shape patch

Vendored from the crates.io `surrealdb-rpc` 3.3.0 source. The package version and all
nominal dependency types remain 3.3.0. `EmptyValue` preserves an absent gRPC SINGLE
payload as NONE. The legacy row stream suppresses it; the raw SDK batch stream exposes
it. The framing consumer retains the existing terminal `single` wire field. No wire
protocol or server version change is introduced.
